//! Merges a reading into the stored snapshot, window by window, so an idle session cannot
//! overwrite fresher numbers from another one.
use chrono::{DateTime, Utc};

use crate::usage::reset_period;
use crate::usage::{LimitWindow, RateLimits, SpendLimit, UsageSnapshot};

/// Where a reading came from, which decides how far it is trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingFrom {
    /// Claude Code's status line, through the bridge. Several sessions report, some of them idle
    /// with old numbers, so within a period the higher percentage is kept.
    StatusLine,
    /// Live usage, which comes from Anthropic itself: within a period it replaces the stored
    /// session and weekly windows whatever their percentage.
    LiveUsage,
}

/// The snapshot to store after a reading. Each window is decided on its own, and each keeps the
/// time it was read at. Spend follows its own rule whichever the source.
pub fn merge(
    stored: Option<&UsageSnapshot>,
    incoming: &RateLimits,
    now: DateTime<Utc>,
    from: ReadingFrom,
) -> UsageSnapshot {
    let old = stored.map(|snapshot| &snapshot.limits);
    let old_at = stored.map_or(now, |snapshot| snapshot.observed_at);
    let window_fresher: fn(&LimitWindow, &LimitWindow) -> bool = match from {
        ReadingFrom::StatusLine => higher_or_equal,
        ReadingFrom::LiveUsage => always,
    };

    let limits = RateLimits {
        five_hour: pick(
            old.and_then(|limits| limits.five_hour.as_ref()),
            incoming.five_hour.as_ref(),
            old_at,
            now,
            window_fresher,
        ),
        seven_day: pick(
            old.and_then(|limits| limits.seven_day.as_ref()),
            incoming.seven_day.as_ref(),
            old_at,
            now,
            window_fresher,
        ),
        spend: pick(
            old.and_then(|limits| limits.spend.as_ref()),
            incoming.spend.as_ref(),
            old_at,
            now,
            spend_fresher,
        ),
    };
    UsageSnapshot {
        limits,
        observed_at: now,
    }
}

/// Within a period the used percentage only rises, so of two status line readings the higher is
/// the fresher.
fn higher_or_equal(stored: &LimitWindow, incoming: &LimitWindow) -> bool {
    incoming.used_percentage >= stored.used_percentage
}

fn always(_stored: &LimitWindow, _incoming: &LimitWindow) -> bool {
    true
}

/// Not so for spend: raising the cap lowers the percentage, so a new cap is always taken, and
/// with the same cap the dollars decide.
fn spend_fresher(stored: &SpendLimit, incoming: &SpendLimit) -> bool {
    if incoming.limit_usd != stored.limit_usd {
        return true;
    }
    match (incoming.used_usd, stored.used_usd) {
        (Some(used), Some(before)) => used >= before,
        _ => incoming.used_percentage >= stored.used_percentage,
    }
}

/// What the merge needs from a window and from the spend limit alike.
trait Reading: Clone {
    fn resets_at(&self) -> Option<DateTime<Utc>>;
    fn observed_at(&self) -> Option<DateTime<Utc>>;
    /// A copy that says it was read at `at`.
    fn seen_at(&self, at: DateTime<Utc>) -> Self;
}

impl Reading for LimitWindow {
    fn resets_at(&self) -> Option<DateTime<Utc>> {
        self.resets_at
    }

    fn observed_at(&self) -> Option<DateTime<Utc>> {
        self.observed_at
    }

    fn seen_at(&self, at: DateTime<Utc>) -> Self {
        LimitWindow {
            observed_at: Some(at),
            ..self.clone()
        }
    }
}

impl Reading for SpendLimit {
    fn resets_at(&self) -> Option<DateTime<Utc>> {
        self.resets_at
    }

    fn observed_at(&self) -> Option<DateTime<Utc>> {
        self.observed_at
    }

    fn seen_at(&self, at: DateTime<Utc>) -> Self {
        SpendLimit {
            observed_at: Some(at),
            ..self.clone()
        }
    }
}

/// Chooses between what is stored and what came in, for one window. What it gives back always
/// says when it was read.
fn pick<T: Reading>(
    stored: Option<&T>,
    incoming: Option<&T>,
    stored_at: DateTime<Utc>,
    now: DateTime<Utc>,
    fresher: fn(&T, &T) -> bool,
) -> Option<T> {
    // A stored window from before windows kept their own time was read when its file was written.
    let stored = stored.map(|stored| stored.seen_at(stored.observed_at().unwrap_or(stored_at)));

    let Some(incoming) = incoming else {
        // A window the reading does not carry stays until its own reset passes.
        return stored.filter(|stored| !stored.resets_at().is_some_and(|expiry| expiry <= now));
    };
    let Some(stored) = stored else {
        return Some(incoming.seen_at(now));
    };

    let take_incoming = match (stored.resets_at(), incoming.resets_at()) {
        // Both say when they reset: a later period wins, an earlier one loses, and within the
        // same period the fresher of the two is kept.
        (Some(old_reset), Some(new_reset)) => {
            new_reset - old_reset > reset_period::TOLERANCE
                || (reset_period::same(Some(old_reset), Some(new_reset))
                    && fresher(&stored, incoming))
        }
        // Without both reset times there is nothing to compare: the new reading is taken.
        _ => true,
    };
    if take_incoming {
        Some(incoming.seen_at(now))
    } else {
        Some(stored)
    }
}
