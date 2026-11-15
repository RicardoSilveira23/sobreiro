//! Ported from the `Pick_takes_the_newer_and_live_on_a_tie` test of
//! `tests/Sobreiro.Tests/CopilotStateTests.cs`. The sentence under the figures, which that file
//! mostly tests, is not ported yet: it comes with the area that formats times.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::copilot::copilot_shown::pick;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};

use crate::time;

fn seen() -> DateTime<Utc> {
    time("2026-11-10T09:25:00Z")
}

fn reading(observed_at: DateTime<Utc>, source: CopilotSource) -> CopilotReading {
    CopilotReading {
        budget: 15000.0,
        used: 2644.2,
        resets_at: None,
        observed_at,
        source,
    }
}

/// The two came back as the very same value, not only equal ones: `pick` hands a reading back
/// and does not copy it. `Option<&T>` has no way to compare two readings that look alike, so
/// the test compares where they are in memory.
fn is_same(picked: Option<&CopilotReading>, expected: Option<&CopilotReading>) -> bool {
    match (picked, expected) {
        (Some(picked), Some(expected)) => std::ptr::eq(picked, expected),
        (None, None) => true,
        _ => false,
    }
}

#[test]
fn pick_takes_the_newer_and_live_on_a_tie() {
    let live = reading(seen(), CopilotSource::Live);
    let newer_cache = reading(seen() + TimeDelta::minutes(1), CopilotSource::Cache);
    let tied_cache = reading(seen(), CopilotSource::Cache);
    let older_cache = reading(seen() - TimeDelta::minutes(1), CopilotSource::Cache);

    assert!(is_same(
        pick(Some(&live), Some(&newer_cache)),
        Some(&newer_cache)
    ));
    assert!(is_same(pick(Some(&live), Some(&tied_cache)), Some(&live)));
    assert!(is_same(pick(Some(&live), Some(&older_cache)), Some(&live)));
    assert!(is_same(pick(Some(&live), None), Some(&live)));
    assert!(is_same(pick(None, Some(&tied_cache)), Some(&tied_cache)));
    assert!(pick(None, None).is_none());
}
