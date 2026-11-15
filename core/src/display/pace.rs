//! Where a limit is heading at the pace it has had so far.
use chrono::{DateTime, TimeDelta, Utc};

use super::{PaceDays, Zone, working_time};
use crate::limits::Limit;

/// Where a limit ends up at its pace: the percentage it would reach by the reset, and, when
/// that is over 100, how long before the reset it runs out and the moment it does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaceProjection {
    pub projected_percent: f64,
    pub runs_out_before_reset: Option<TimeDelta>,
    pub runs_out_at: Option<DateTime<Utc>>,
}

/// Below this share of the window elapsed, a projection is too noisy to show.
pub const MIN_ELAPSED_SHARE: f64 = 0.05;

/// The pace of a limit, or none when it has no reset time or window, is reset, is full, was read
/// after its reset, or too little of its window has gone to say.
///
/// With [`PaceDays::Weekdays`], a window longer than a day that holds some working time is
/// measured in Monday-to-Friday time in `zone`; any other window keeps calendar time.
///
/// Spans are counted in whole ticks, ten-millionths of a second, as the C# does, so that a
/// share and a run-out moment come out the same to the last digit.
pub fn project(limit: &Limit, days: PaceDays, zone: &Zone) -> Option<PaceProjection> {
    let resets_at = limit.resets_at?;
    let window = limit.window?;
    if limit.is_reset || window <= TimeDelta::zero() {
        return None;
    }
    if limit.percent >= 100.0 || limit.observed_at > resets_at {
        return None;
    }

    let start = resets_at.checked_sub_signed(window)?;
    let working_window = if days == PaceDays::Weekdays && window > TimeDelta::days(1) {
        working_time::between(start, resets_at, zone)
    } else {
        TimeDelta::zero()
    };
    let weekdays = working_window > TimeDelta::zero();

    let elapsed = if weekdays {
        working_time::between(start, limit.observed_at, zone)
    } else {
        limit.observed_at.signed_duration_since(start)
    };
    let total = if weekdays { working_window } else { window };
    let share = ticks(elapsed) as f64 / ticks(total) as f64;
    if !(MIN_ELAPSED_SHARE..=1.0).contains(&share) {
        return None;
    }

    let projected = limit.percent / share;
    let mut runs_out = None;
    let mut runs_out_at = None;
    if projected > 100.0 {
        let needed = scaled(elapsed, (100.0 - limit.percent) / limit.percent);
        let run_out_at = if weekdays {
            working_time::after(limit.observed_at, needed, zone)
        } else {
            limit
                .observed_at
                .checked_add_signed(needed)
                .unwrap_or(DateTime::<Utc>::MAX_UTC)
        };
        runs_out = Some(resets_at.signed_duration_since(run_out_at));
        runs_out_at = Some(run_out_at);
    }
    Some(PaceProjection {
        projected_percent: projected,
        runs_out_before_reset: runs_out,
        runs_out_at,
    })
}

const TICKS_PER_SECOND: i128 = 10_000_000;

/// A span as a count of ticks.
fn ticks(span: TimeDelta) -> i128 {
    i128::from(span.num_seconds()) * TICKS_PER_SECOND + i128::from(span.subsec_nanos()) / 100
}

/// A span times a factor, as .NET does it: the ticks times the factor, rounded to the nearest
/// tick (to the even one at a tie), back to a span.
fn scaled(span: TimeDelta, factor: f64) -> TimeDelta {
    let product = (ticks(span) as f64 * factor).round_ties_even();
    // `as i128` stops at the ends of its range and turns not-a-number into 0.
    let count = product as i128;
    let seconds = count.div_euclid(TICKS_PER_SECOND);
    let nanos = count.rem_euclid(TICKS_PER_SECOND) * 100;
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| TimeDelta::new(seconds, nanos as u32))
        .unwrap_or(TimeDelta::MAX)
}
