//! Ported from `tests/Sobreiro.Tests/ResetPeriodTests.cs`.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::usage::reset_period;

use crate::time;

fn t() -> DateTime<Utc> {
    time("2026-11-09T10:00:00Z")
}

#[test]
fn both_null_is_same() {
    assert!(reset_period::same(None, None));
}

#[test]
fn one_null_is_not_same() {
    assert!(!reset_period::same(None, Some(t())));
    assert!(!reset_period::same(Some(t()), None));
}

#[test]
fn fifty_nine_minutes_apart_is_same() {
    let later = t() + TimeDelta::minutes(59);
    assert!(reset_period::same(Some(t()), Some(later)));
    assert!(reset_period::same(Some(later), Some(t())));
}

#[test]
fn sixty_one_minutes_apart_is_not_same() {
    let later = t() + TimeDelta::minutes(61);
    assert!(!reset_period::same(Some(t()), Some(later)));
}
