//! Ported from `tests/Sobreiro.Tests/UsageLevelTests.cs`.
use sobreiro_core::limits::usage_level::{self, UsageLevel};
use sobreiro_core::limits::{Limit, LimitKind};

use crate::time;

fn limit(kind: LimitKind, label: &'static str, percent: f64) -> Limit {
    let now = time("2026-11-09T10:00:00Z");
    Limit::new("Claude Code", kind, label, percent, None, now, false)
}

#[test]
fn for_maps_thresholds() {
    let rows = [
        (0, UsageLevel::Normal),
        (69, UsageLevel::Normal),
        (70, UsageLevel::Warning),
        (89, UsageLevel::Warning),
        (90, UsageLevel::Error),
        (100, UsageLevel::Error),
    ];
    for (percent, expected) in rows {
        assert_eq!(usage_level::for_percent(percent), expected, "{percent}%");
    }
}

#[test]
fn highest_picks_worst() {
    let limits = [
        limit(LimitKind::Session, "Session", 72.0),
        limit(LimitKind::Weekly, "Weekly", 91.0),
    ];
    assert_eq!(usage_level::highest(&limits), UsageLevel::Error);
}

#[test]
fn highest_of_none_is_normal() {
    assert_eq!(usage_level::highest(&[]), UsageLevel::Normal);
}
