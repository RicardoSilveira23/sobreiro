use super::{PaceDays, WorkingTime};
use crate::limits::Limit;
use crate::{Span, Time, Zone};

#[derive(Clone, Debug, PartialEq)]
pub struct PaceProjection {
    pub projected_percent: f64,
    pub runs_out_before_reset: Option<Span>,
    pub runs_out_at: Option<Time>,
}

pub struct Pace;

impl Pace {
    /// Below this share of the window elapsed, a projection is too noisy to show.
    pub const MIN_ELAPSED_SHARE: f64 = 0.05;

    /// `Project(limit)`: every day counts, in UTC.
    pub fn project(limit: &Limit) -> Option<PaceProjection> {
        Self::project_with(limit, PaceDays::EveryDay, Zone::utc())
    }

    /// With `PaceDays::Weekdays`, a window longer than a day that holds some working time is measured in
    /// Monday-to-Friday time in `zone`; any other window keeps calendar time.
    pub fn project_with(limit: &Limit, days: PaceDays, zone: Zone) -> Option<PaceProjection> {
        let resets_at = limit.resets_at?;
        let window = limit.window?;
        if limit.is_reset || window <= Span::zero() {
            return None;
        }
        if limit.percent >= 100.0 || limit.observed_at > resets_at {
            return None;
        }

        let start = resets_at - window;
        let working_window = if days == PaceDays::Weekdays && window > Span::days(1) {
            WorkingTime::between(start, resets_at, zone)
        } else {
            Span::zero()
        };
        let weekdays = working_window > Span::zero();

        let elapsed = if weekdays { WorkingTime::between(start, limit.observed_at, zone) } else { limit.observed_at - start };
        let total = if weekdays { working_window } else { window };
        let share = ratio(elapsed, total);
        if share < Self::MIN_ELAPSED_SHARE || share > 1.0 {
            return None;
        }

        let projected = limit.percent / share;
        let mut runs_out = None;
        let mut runs_out_at = None;
        if projected > 100.0 {
            let needed = scale(elapsed, (100.0 - limit.percent) / limit.percent);
            let run_out_at = if weekdays { WorkingTime::after(limit.observed_at, needed, zone) } else { limit.observed_at + needed };
            runs_out = Some(resets_at - run_out_at);
            runs_out_at = Some(run_out_at);
        }
        Some(PaceProjection { projected_percent: projected, runs_out_before_reset: runs_out, runs_out_at })
    }
}

fn ratio(a: Span, b: Span) -> f64 {
    a.as_seconds_f64() / b.as_seconds_f64()
}

fn scale(s: Span, f: f64) -> Span {
    Span::nanoseconds((s.num_nanoseconds().unwrap_or(0) as f64 * f).round() as i64)
}
