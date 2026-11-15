//! The status of a limit, as a word under its figure.
use chrono::{DateTime, TimeDelta, Utc};

use super::{ClockFormat, PaceDays, Zone, pace, when};
use crate::limits::Limit;

/// What a limit's status badge says, in kind: how the limit stands against its reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    OnTrack,
    Ends,
    UsedUp,
    NotUsedYet,
    TooEarly,
    Resetting,
}

/// The badge of a limit: its kind and the words on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitStatus {
    pub kind: StatusKind,
    pub text: String,
}

/// Under this far from the reset, a limit that is not yet reset reads "Resetting".
pub const RESETTING_WITHIN: TimeDelta = TimeDelta::seconds(30);

/// Below this percentage the limit reads "Not used yet".
pub const NOT_USED_BELOW: f64 = 2.0;

fn status(kind: StatusKind, text: &str) -> Option<LimitStatus> {
    Some(LimitStatus {
        kind,
        text: text.to_string(),
    })
}

/// The badge for a limit, or none when it has no reset time or window length.
pub fn for_limit(
    limit: &Limit,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
    days: PaceDays,
) -> Option<LimitStatus> {
    let resets_at = limit.resets_at?;
    let window = limit.window?;
    if window <= TimeDelta::zero() {
        return None;
    }
    if !limit.is_reset && resets_at.signed_duration_since(now) < RESETTING_WITHIN {
        return status(StatusKind::Resetting, "Resetting");
    }
    if limit.is_reset || limit.percent < NOT_USED_BELOW {
        return status(StatusKind::NotUsedYet, "Not used yet");
    }
    if limit.percent >= 100.0 {
        return status(StatusKind::UsedUp, "Used up");
    }

    let Some(projection) = pace::project(limit, days, zone) else {
        return status(StatusKind::TooEarly, "Too early");
    };
    if let Some(at) = projection.runs_out_at {
        return if at <= now {
            status(StatusKind::Ends, "Likely used up")
        } else {
            status(
                StatusKind::Ends,
                &format!("Ends {}", when::format(at, now, zone, clock)),
            )
        };
    }
    status(StatusKind::OnTrack, "On track")
}
