//! Ported from `tests/Sobreiro.Tests/LimitListTests.cs`.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::connect::ConnectionState;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::limits::{Limit, LimitKind, limit_list};
use sobreiro_core::usage::{RateLimits, UsageSnapshot};

use crate::{spend, time, window, window_seen};

fn now() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
}

/// A snapshot read two minutes ago.
fn snap(
    five_hour: Option<sobreiro_core::usage::LimitWindow>,
    seven_day: Option<sobreiro_core::usage::LimitWindow>,
    spend: Option<sobreiro_core::usage::SpendLimit>,
) -> UsageSnapshot {
    UsageSnapshot {
        limits: RateLimits {
            five_hour,
            seven_day,
            spend,
        },
        observed_at: now() - TimeDelta::minutes(2),
    }
}

#[test]
fn from_null_is_empty() {
    assert!(limit_list::from(None, now()).is_empty());
}

// The 6.28 below is dollars, as in the C# test. The linter takes it for a rounded two-pi.
#[allow(clippy::approx_constant)]
#[test]
fn from_maps_windows_in_order_with_labels() {
    let five = now() + TimeDelta::hours(2);
    let seven = now() + TimeDelta::days(5);
    let spend_resets = now() + TimeDelta::days(7);
    let snapshot = snap(
        Some(window(8.0, Some(five))),
        Some(window(44.0, Some(seven))),
        Some(spend(
            62.8,
            Some(spend_resets),
            Some(6.28),
            Some(10.0),
            Some("monthly"),
        )),
    );

    let list = limit_list::from(Some(&snapshot), now());

    assert_eq!(list.len(), 3);
    assert_eq!(list[0].kind, LimitKind::Session);
    assert_eq!(list[0].label, "Session");
    assert_eq!(list[0].provider, "Claude Code");
    assert_eq!(list[0].percent, 8.0);
    assert_eq!(list[0].resets_at, Some(five));
    assert!(!list[0].is_reset);
    assert_eq!(list[0].observed_at, now() - TimeDelta::minutes(2));

    assert_eq!(list[1].kind, LimitKind::Weekly);
    assert_eq!(list[1].label, "Weekly");
    assert_eq!(list[1].percent, 44.0);
    assert_eq!(list[1].resets_at, Some(seven));

    assert_eq!(list[2].kind, LimitKind::Spend);
    assert_eq!(list[2].label, "Spend");
    assert_eq!(list[2].percent, 62.8);
    assert_eq!(list[2].resets_at, Some(spend_resets));
}

#[test]
fn from_marks_passed_window_as_reset_at_zero() {
    let resets = time("2026-11-09T11:00:00Z");
    let snapshot = snap(Some(window(30.0, Some(resets))), None, None);

    let list = limit_list::from(Some(&snapshot), now());

    assert_eq!(list.len(), 1);
    assert!(list[0].is_reset);
    assert_eq!(list[0].percent, 0.0);
    assert_eq!(list[0].resets_at, Some(resets));
}

#[test]
fn from_skips_absent_windows() {
    let tomorrow = now() + TimeDelta::days(1);
    let snapshot = snap(None, Some(window(44.0, Some(tomorrow))), None);

    let list = limit_list::from(Some(&snapshot), now());

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, LimitKind::Weekly);
}

#[test]
fn display_percent_floors_and_clamps() {
    let rows = [(8.9, 8), (99.6, 99), (100.0, 100), (130.0, 100), (-5.0, 0)];
    for (percent, expected) in rows {
        let limit = Limit::new(
            "Claude Code",
            LimitKind::Session,
            "Session",
            percent,
            None,
            now(),
            false,
        );
        assert_eq!(limit.display_percent(), expected, "{percent}");
    }
}

#[test]
fn from_uses_the_windows_own_observed_at_when_it_has_one() {
    let own = now() - TimeDelta::hours(3);
    let snapshot = snap(
        Some(window_seen(10.0, Some(now() + TimeDelta::hours(2)), own)),
        Some(window(20.0, Some(now() + TimeDelta::days(2)))),
        None,
    );

    let list = limit_list::from(Some(&snapshot), now());

    assert_eq!(list[0].observed_at, own);
    assert_eq!(list[1].observed_at, now() - TimeDelta::minutes(2));
}

#[test]
fn from_fills_windows_and_spend_fields() {
    let resets = time("2026-12-01T00:00:00Z");
    let snapshot = snap(
        Some(window(8.0, Some(now() + TimeDelta::hours(2)))),
        Some(window(44.0, Some(now() + TimeDelta::days(5)))),
        Some(spend(
            63.0,
            Some(resets),
            Some(31.5),
            Some(50.0),
            Some("monthly"),
        )),
    );

    let list = limit_list::from(Some(&snapshot), now());

    assert_eq!(list[0].window, Some(TimeDelta::hours(5)));
    assert_eq!(list[1].window, Some(TimeDelta::days(7)));
    assert_eq!(list[2].window, Some(TimeDelta::days(30)));
    assert_eq!(list[2].used_usd, Some(31.5));
    assert_eq!(list[2].limit_usd, Some(50.0));
    assert_eq!(list[2].period.as_deref(), Some("monthly"));
}

#[test]
fn spend_window_follows_the_period() {
    let rows = [
        (Some("daily"), Some(1)),
        (Some("weekly"), Some(7)),
        (Some("yearly"), None),
        (None, None),
    ];
    for (period, days) in rows {
        let tomorrow = now() + TimeDelta::days(1);
        let snapshot = snap(
            None,
            None,
            Some(spend(10.0, Some(tomorrow), None, None, period)),
        );

        let list = limit_list::from(Some(&snapshot), now());

        assert_eq!(list[0].window, days.map(TimeDelta::days), "{period:?}");
    }
}

fn reading() -> CopilotReading {
    CopilotReading {
        budget: 15000.0,
        used: 2644.2,
        resets_at: Some(time("2026-12-01T00:00:00Z")),
        observed_at: now() - TimeDelta::minutes(1),
        source: CopilotSource::Live,
    }
}

#[test]
fn copilot_reading_becomes_one_credits_limit() {
    let list = limit_list::from_copilot(Some(&reading()), now());

    assert_eq!(list.len(), 1);
    let limit = &list[0];
    assert_eq!(limit.provider, "GitHub Copilot");
    assert_eq!(limit.kind, LimitKind::Credits);
    assert_eq!(limit.label, "AI credits");
    // The C# test compares to three decimal places.
    assert!((limit.percent - 17.628).abs() < 0.0005, "{}", limit.percent);
    assert_eq!(limit.used_credits, Some(2644.2));
    assert_eq!(limit.credit_budget, Some(15000.0));
    assert_eq!(limit.window, Some(TimeDelta::days(30)));
    assert_eq!(limit.resets_at, Some(time("2026-12-01T00:00:00Z")));
    assert!(!limit.is_reset);
    assert_eq!(limit.observed_at, now() - TimeDelta::minutes(1));
}

#[test]
fn no_copilot_reading_gives_no_limits() {
    assert!(limit_list::from_copilot(None, now()).is_empty());
}

#[test]
fn credits_without_reset_time_have_no_window() {
    let without_reset = CopilotReading {
        resets_at: None,
        ..reading()
    };

    let list = limit_list::from_copilot(Some(&without_reset), now());

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].window, None);
    assert!(!list[0].is_reset);
}

#[test]
fn passed_reset_reads_zero_and_reset() {
    let passed = CopilotReading {
        resets_at: Some(time("2026-11-01T00:00:00Z")),
        ..reading()
    };

    let list = limit_list::from_copilot(Some(&passed), now());

    assert_eq!(list.len(), 1);
    assert_eq!(list[0].percent, 0.0);
    assert!(list[0].is_reset);
}

#[test]
fn claude_shows_when_on_pc_or_when_nothing_else_shows() {
    let rows = [
        (true, true, true),
        (true, false, true),
        (false, false, true),
        (false, true, false),
    ];
    for (on_pc, copilot_shown, expected) in rows {
        assert_eq!(
            limit_list::show_claude(on_pc, copilot_shown),
            expected,
            "on this PC {on_pc}, Copilot shown {copilot_shown}"
        );
    }
}

#[test]
fn claude_is_on_pc_by_credentials_readings_or_connection() {
    let rows = [
        (false, 0, ConnectionState::NotConnected, false),
        (true, 0, ConnectionState::NotConnected, true),
        (false, 1, ConnectionState::NotConnected, true),
        (false, 0, ConnectionState::Connected, true),
        (false, 0, ConnectionState::Legacy, true),
    ];
    for (file, count, state, expected) in rows {
        assert_eq!(
            limit_list::claude_on_pc(file, count, state),
            expected,
            "file {file}, {count} limits, {state:?}"
        );
    }
}
