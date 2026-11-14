use super::{Pace, PaceDays, When};
use crate::limits::Limit;
use crate::{Span, Time, Zone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    OnTrack,
    Ends,
    UsedUp,
    NotUsedYet,
    TooEarly,
    Resetting,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LimitStatus {
    pub kind: StatusKind,
    pub text: String,
}

fn status(kind: StatusKind, text: impl Into<String>) -> Option<LimitStatus> {
    Some(LimitStatus { kind, text: text.into() })
}

impl LimitStatus {
    /// Under this far from the reset, a limit that is not yet reset reads "Resetting".
    pub fn resetting_within() -> Span {
        Span::seconds(30)
    }

    /// Below this percentage the limit reads "Not used yet".
    pub const NOT_USED_BELOW: f64 = 2.0;

    /// `LimitStatus.For(...)`: the badge for a limit, or None when it has no reset time or window length.
    pub fn for_limit(limit: &Limit, now: Time, zone: Zone, days: PaceDays) -> Option<LimitStatus> {
        let resets_at = limit.resets_at?;
        let window = limit.window?;
        if window <= Span::zero() {
            return None;
        }
        if !limit.is_reset && resets_at - now < Self::resetting_within() {
            return status(StatusKind::Resetting, "Resetting");
        }
        if limit.is_reset || limit.percent < Self::NOT_USED_BELOW {
            return status(StatusKind::NotUsedYet, "Not used yet");
        }
        if limit.percent >= 100.0 {
            return status(StatusKind::UsedUp, "Used up");
        }

        let Some(pace) = Pace::project_with(limit, days, zone) else {
            return status(StatusKind::TooEarly, "Too early");
        };
        if let Some(at) = pace.runs_out_at {
            return if at <= now {
                status(StatusKind::Ends, "Likely used up")
            } else {
                status(StatusKind::Ends, format!("Ends {}", When::format(at, now, zone)))
            };
        }
        status(StatusKind::OnTrack, "On track")
    }
}
