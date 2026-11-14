use super::{LimitWindow, RateLimits, ResetPeriod, SpendLimit, UsageSnapshot};
use crate::Time;

/// Merges a reading into the stored snapshot per window, so an idle session cannot overwrite fresher numbers from
/// another one.
pub struct UsageMerge;

impl UsageMerge {
    /// `authoritative` is for Live usage's readings, which come from Anthropic itself: within a period they
    /// replace the stored session and weekly windows whatever their percentage. Status line readings keep the
    /// higher one. Spend follows its own rule either way.
    pub fn merge(stored: Option<&UsageSnapshot>, incoming: &RateLimits, now: Time, authoritative: bool) -> UsageSnapshot {
        let old = stored.map(|s| &s.limits);
        let old_at = stored.map(|s| s.observed_at).unwrap_or(now);
        let window_fresher = |s: &LimitWindow, i: &LimitWindow| authoritative || i.used_percentage >= s.used_percentage;

        let limits = RateLimits {
            five_hour: pick(
                old.and_then(|o| o.five_hour.as_ref()).map(|w| resolve_window(w, old_at)),
                incoming.five_hour.as_ref(), now, &window_fresher, |w| w.resets_at,
                |w, at| LimitWindow { observed_at: Some(at), ..w.clone() },
            ),
            seven_day: pick(
                old.and_then(|o| o.seven_day.as_ref()).map(|w| resolve_window(w, old_at)),
                incoming.seven_day.as_ref(), now, &window_fresher, |w| w.resets_at,
                |w, at| LimitWindow { observed_at: Some(at), ..w.clone() },
            ),
            spend: pick(
                old.and_then(|o| o.spend.as_ref()).map(|s| SpendLimit { observed_at: Some(s.observed_at.unwrap_or(old_at)), ..s.clone() }),
                incoming.spend.as_ref(), now, &spend_fresher, |s| s.resets_at,
                |s, at| SpendLimit { observed_at: Some(at), ..s.clone() },
            ),
        };
        UsageSnapshot { limits, observed_at: now }
    }
}

fn resolve_window(w: &LimitWindow, snapshot_at: Time) -> LimitWindow {
    LimitWindow { observed_at: Some(w.observed_at.unwrap_or(snapshot_at)), ..w.clone() }
}

// Not so for spend: raising the cap lowers the percentage, so a new cap is always taken, and with the same
// cap the dollars decide.
fn spend_fresher(stored: &SpendLimit, incoming: &SpendLimit) -> bool {
    if incoming.limit_usd != stored.limit_usd {
        return true;
    }
    match (incoming.used_usd, stored.used_usd) {
        (Some(used), Some(before)) => used >= before,
        _ => incoming.used_percentage >= stored.used_percentage,
    }
}

/// Stored must already have its observed_at resolved. The result always carries an observed_at.
fn pick<T: Clone>(
    stored: Option<T>, incoming: Option<&T>, now: Time,
    fresher: &dyn Fn(&T, &T) -> bool, resets_at: impl Fn(&T) -> Option<Time>, stamp: impl Fn(&T, Time) -> T,
) -> Option<T> {
    let Some(incoming) = incoming else {
        // A window the reading does not carry stays until its own reset passes.
        return stored.filter(|s| !resets_at(s).is_some_and(|expiry| expiry <= now));
    };
    let Some(stored) = stored else { return Some(stamp(incoming, now)) };

    let old_reset = resets_at(&stored);
    let new_reset = resets_at(incoming);
    let take_incoming = match (old_reset, new_reset) {
        (Some(o), Some(n)) => {
            n - o > ResetPeriod::tolerance() || (ResetPeriod::same(old_reset, new_reset) && fresher(&stored, incoming))
        }
        _ => true,
    };
    if take_incoming { Some(stamp(incoming, now)) } else { Some(stored) }
}
