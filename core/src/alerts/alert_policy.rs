//! Toast policy: one alert per window per reset period for the highest threshold crossed.
use chrono::{DateTime, Utc};

use super::alert_state::{AlertMark, AlertState};
use crate::display::flyout_model::FlyoutModel;
use crate::display::limit_status;
use crate::display::{ClockFormat, PaceDays, Zone};
use crate::limits::limit::{Limit, LimitKind};
use crate::usage::reset_period;

/// One toast to show: the limit, the threshold it crossed, and the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub kind: LimitKind,
    pub threshold: i32,
    pub title: String,
    pub body: String,
}

/// The percentages that raise an alert.
pub const THRESHOLDS: [i32; 2] = [80, 95];

/// Returns the alerts to show and the state to keep. The state is `None` when nothing changed,
/// so callers can skip saving.
pub fn check(
    limits: &[Limit],
    state: &AlertState,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
    days: PaceDays,
) -> (Vec<Alert>, Option<AlertState>) {
    let mut alerts = Vec::new();
    let mut marks = state.marks.clone();

    for limit in limits {
        if limit.is_reset {
            continue;
        }
        let percent = limit.display_percent();
        let Some(crossed) = THRESHOLDS.into_iter().filter(|t| percent >= *t).max() else {
            continue;
        };

        if let Some(mark) = state.marks.get(&limit.kind)
            && reset_period::same(mark.resets_at, limit.resets_at)
            && mark.threshold >= crossed
        {
            continue;
        }

        let title = if limit.kind == LimitKind::Credits {
            format!("Copilot {} {percent}% used", limit.label)
        } else {
            format!("{} {percent}% used", limit.label)
        };
        let body = [
            limit_status::for_limit(limit, now, zone, clock, days).map(|status| status.text),
            FlyoutModel::resets_in(limit, now, zone, clock),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" \u{b7} ");

        alerts.push(Alert {
            kind: limit.kind,
            threshold: crossed,
            title,
            body,
        });
        marks.insert(
            limit.kind,
            AlertMark {
                resets_at: limit.resets_at,
                threshold: crossed,
            },
        );
    }

    if alerts.is_empty() {
        return (alerts, None);
    }
    // A stable sort: alerts of the same threshold and kind keep the order the limits came in.
    alerts.sort_by(|a, b| b.threshold.cmp(&a.threshold).then(a.kind.cmp(&b.kind)));
    (alerts, Some(AlertState { marks }))
}
