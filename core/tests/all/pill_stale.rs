//! Ported from `tests/Sobreiro.Tests/PillStaleTests.cs`. The last two tests walk rows that the C#
//! of 1.x wrote out itself.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::display::pill_stale;
use sobreiro_core::limits::{Limit, LimitKind};
use sobreiro_core::polling::ProviderState;

use crate::{span, time};

fn now() -> DateTime<Utc> {
    time("2026-11-13T12:00:00Z")
}

const TWO: i32 = pill_stale::MISSES_NEEDED;

const STOPPED: [ProviderState; 6] = [
    ProviderState::Expired,
    ProviderState::NoSignIn,
    ProviderState::Refused,
    ProviderState::Offline,
    ProviderState::BadAnswer,
    ProviderState::RateLimited,
];

fn label(kind: LimitKind) -> &'static str {
    match kind {
        LimitKind::Session => "Session",
        LimitKind::Weekly => "Weekly",
        LimitKind::Spend => "Spend",
        LimitKind::Credits => "Credits",
    }
}

fn aged(kind: LimitKind, age: TimeDelta) -> Limit {
    Limit::new(
        "Claude Code",
        kind,
        label(kind),
        40.0,
        None,
        now() - age,
        false,
    )
}

fn old() -> Vec<Limit> {
    vec![
        aged(LimitKind::Session, TimeDelta::days(3)),
        aged(LimitKind::Weekly, TimeDelta::days(3)),
    ]
}

fn credits(age: TimeDelta) -> Limit {
    Limit::new(
        "GitHub Copilot",
        LimitKind::Credits,
        "AI credits",
        20.0,
        None,
        now() - age,
        false,
    )
}

#[test]
fn every_stopped_state_marks_claude() {
    for state in STOPPED {
        assert!(pill_stale::claude(state, &old(), now(), TWO), "{state:?}");
    }
}

#[test]
fn off_and_ok_are_never_stale() {
    for state in [ProviderState::Off, ProviderState::Ok] {
        assert!(!pill_stale::claude(state, &old(), now(), TWO), "{state:?}");
    }
}

#[test]
fn one_missed_poll_is_not_stale() {
    for misses in [0, 1] {
        for state in STOPPED {
            assert!(
                !pill_stale::claude(state, &old(), now(), misses),
                "{state:?} {misses}"
            );
            assert!(
                !pill_stale::copilot(state, Some(&credits(TimeDelta::days(3))), now(), misses),
                "{state:?} {misses}"
            );
        }
    }
}

#[test]
fn two_missed_polls_are_needed() {
    assert_eq!(pill_stale::MISSES_NEEDED, 2);
}

#[test]
fn fresh_numbers_are_not_stale() {
    let edge = TimeDelta::minutes(15);
    let both = |age| vec![aged(LimitKind::Session, age), aged(LimitKind::Weekly, age)];
    assert!(!pill_stale::claude(
        ProviderState::Expired,
        &both(edge),
        now(),
        TWO
    ));
    let past = edge + TimeDelta::seconds(1);
    assert!(pill_stale::claude(
        ProviderState::Expired,
        &both(past),
        now(),
        TWO
    ));
}

#[test]
fn the_newest_of_the_two_readings_decides() {
    assert!(!pill_stale::claude(
        ProviderState::Offline,
        &[
            aged(LimitKind::Session, TimeDelta::hours(3)),
            aged(LimitKind::Weekly, TimeDelta::minutes(1))
        ],
        now(),
        TWO
    ));
    assert!(!pill_stale::claude(
        ProviderState::Offline,
        &[aged(LimitKind::Weekly, TimeDelta::minutes(1))],
        now(),
        TWO
    ));
}

#[test]
fn a_spend_reading_does_not_count() {
    let spend = || aged(LimitKind::Spend, TimeDelta::minutes(1));
    assert!(pill_stale::claude(
        ProviderState::Offline,
        &[spend(), aged(LimitKind::Session, TimeDelta::hours(3))],
        now(),
        TWO
    ));
    assert!(!pill_stale::claude(
        ProviderState::Offline,
        &[spend()],
        now(),
        TWO
    ));
}

#[test]
fn nothing_to_keep_is_not_stale() {
    for state in STOPPED {
        assert!(!pill_stale::claude(state, &[], now(), TWO), "{state:?}");
    }
}

#[test]
fn a_reading_from_the_future_is_fresh() {
    assert!(!pill_stale::claude(
        ProviderState::Offline,
        &[aged(LimitKind::Session, TimeDelta::minutes(-10))],
        now(),
        TWO
    ));
}

#[test]
fn every_stopped_state_marks_copilot() {
    for state in [
        ProviderState::NoSignIn,
        ProviderState::Refused,
        ProviderState::Offline,
        ProviderState::BadAnswer,
        ProviderState::RateLimited,
    ] {
        assert!(
            pill_stale::copilot(state, Some(&credits(TimeDelta::hours(25))), now(), TWO),
            "{state:?}"
        );
    }
}

#[test]
fn copilot_ok_is_not_stale() {
    assert!(!pill_stale::copilot(
        ProviderState::Ok,
        Some(&credits(TimeDelta::hours(25))),
        now(),
        TWO
    ));
}

#[test]
fn a_copilot_figure_from_today_is_not_stale() {
    assert!(!pill_stale::copilot(
        ProviderState::NoSignIn,
        Some(&credits(TimeDelta::hours(2))),
        now(),
        TWO
    ));
    assert!(!pill_stale::copilot(
        ProviderState::NoSignIn,
        Some(&credits(TimeDelta::hours(24))),
        now(),
        TWO
    ));
}

#[test]
fn no_reading_is_not_stale() {
    for state in [
        ProviderState::Ok,
        ProviderState::NoSignIn,
        ProviderState::Refused,
        ProviderState::Offline,
        ProviderState::BadAnswer,
        ProviderState::RateLimited,
    ] {
        assert!(!pill_stale::copilot(state, None, now(), TWO), "{state:?}");
    }
}

fn state_of(name: &str) -> ProviderState {
    match name {
        "Off" => ProviderState::Off,
        "Ok" => ProviderState::Ok,
        "Expired" => ProviderState::Expired,
        "NoSignIn" => ProviderState::NoSignIn,
        "Refused" => ProviderState::Refused,
        "RateLimited" => ProviderState::RateLimited,
        "Offline" => ProviderState::Offline,
        "BadAnswer" => ProviderState::BadAnswer,
        _ => panic!("no state named {name}"),
    }
}

fn kind_of(name: &str) -> LimitKind {
    match name {
        "Session" => LimitKind::Session,
        "Weekly" => LimitKind::Weekly,
        "Spend" => LimitKind::Spend,
        "Credits" => LimitKind::Credits,
        _ => panic!("no kind named {name}"),
    }
}

/// A state, the polls missed, the limits as (kind, age in ticks), and what the C# answered for
/// `PillStale.Claude` at noon on 2026-11-13.
type ClaudeRow = (&'static str, i32, &'static [(&'static str, i64)], bool);

const CLAUDE: &[ClaudeRow] = &[
    (
        "Off",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "Expired",
        3,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "NoSignIn",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "Refused",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "RateLimited",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "Offline",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "BadAnswer",
        2,
        &[("Session", 2592000000000), ("Weekly", 2592000000000)],
        true,
    ),
    (
        "Off",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Expired",
        3,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "NoSignIn",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Refused",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "RateLimited",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Offline",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "BadAnswer",
        2,
        &[("Session", 9000000000), ("Weekly", 9000000000)],
        false,
    ),
    (
        "Off",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "Expired",
        3,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "NoSignIn",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "Refused",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "RateLimited",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "Offline",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "BadAnswer",
        2,
        &[("Session", 9000000001), ("Weekly", 9000000001)],
        true,
    ),
    (
        "Off",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Expired",
        3,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "NoSignIn",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Refused",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "RateLimited",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Offline",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "BadAnswer",
        2,
        &[("Session", 108000000000), ("Weekly", 600000000)],
        false,
    ),
    (
        "Off",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "Expired",
        3,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "NoSignIn",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "Refused",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "RateLimited",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "Offline",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "BadAnswer",
        2,
        &[("Weekly", 108000000000), ("Session", 12000000000)],
        true,
    ),
    (
        "Off",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Spend", 600000000), ("Session", 108000000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "Expired",
        3,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "NoSignIn",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "Refused",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "RateLimited",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "Offline",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    (
        "BadAnswer",
        2,
        &[("Spend", 600000000), ("Session", 108000000000)],
        true,
    ),
    ("Off", 2, &[("Spend", 600000000)], false),
    ("Ok", 0, &[("Spend", 600000000)], false),
    ("Ok", 1, &[("Spend", 600000000)], false),
    ("Ok", 2, &[("Spend", 600000000)], false),
    ("Ok", 3, &[("Spend", 600000000)], false),
    ("Expired", 0, &[("Spend", 600000000)], false),
    ("Expired", 1, &[("Spend", 600000000)], false),
    ("Expired", 2, &[("Spend", 600000000)], false),
    ("Expired", 3, &[("Spend", 600000000)], false),
    ("NoSignIn", 2, &[("Spend", 600000000)], false),
    ("Refused", 2, &[("Spend", 600000000)], false),
    ("RateLimited", 2, &[("Spend", 600000000)], false),
    ("Offline", 2, &[("Spend", 600000000)], false),
    ("BadAnswer", 2, &[("Spend", 600000000)], false),
    ("Off", 2, &[("Weekly", 600000000)], false),
    ("Ok", 0, &[("Weekly", 600000000)], false),
    ("Ok", 1, &[("Weekly", 600000000)], false),
    ("Ok", 2, &[("Weekly", 600000000)], false),
    ("Ok", 3, &[("Weekly", 600000000)], false),
    ("Expired", 0, &[("Weekly", 600000000)], false),
    ("Expired", 1, &[("Weekly", 600000000)], false),
    ("Expired", 2, &[("Weekly", 600000000)], false),
    ("Expired", 3, &[("Weekly", 600000000)], false),
    ("NoSignIn", 2, &[("Weekly", 600000000)], false),
    ("Refused", 2, &[("Weekly", 600000000)], false),
    ("RateLimited", 2, &[("Weekly", 600000000)], false),
    ("Offline", 2, &[("Weekly", 600000000)], false),
    ("BadAnswer", 2, &[("Weekly", 600000000)], false),
    ("Off", 2, &[("Session", -6000000000)], false),
    ("Ok", 0, &[("Session", -6000000000)], false),
    ("Ok", 1, &[("Session", -6000000000)], false),
    ("Ok", 2, &[("Session", -6000000000)], false),
    ("Ok", 3, &[("Session", -6000000000)], false),
    ("Expired", 0, &[("Session", -6000000000)], false),
    ("Expired", 1, &[("Session", -6000000000)], false),
    ("Expired", 2, &[("Session", -6000000000)], false),
    ("Expired", 3, &[("Session", -6000000000)], false),
    ("NoSignIn", 2, &[("Session", -6000000000)], false),
    ("Refused", 2, &[("Session", -6000000000)], false),
    ("RateLimited", 2, &[("Session", -6000000000)], false),
    ("Offline", 2, &[("Session", -6000000000)], false),
    ("BadAnswer", 2, &[("Session", -6000000000)], false),
    (
        "Off",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Ok",
        0,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Ok",
        1,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Ok",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Ok",
        3,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Expired",
        0,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Expired",
        1,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        false,
    ),
    (
        "Expired",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "Expired",
        3,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "NoSignIn",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "Refused",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "RateLimited",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "Offline",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    (
        "BadAnswer",
        2,
        &[("Credits", 2592000000000), ("Weekly", 720000000000)],
        true,
    ),
    ("Off", 2, &[], false),
    ("Ok", 0, &[], false),
    ("Ok", 1, &[], false),
    ("Ok", 2, &[], false),
    ("Ok", 3, &[], false),
    ("Expired", 0, &[], false),
    ("Expired", 1, &[], false),
    ("Expired", 2, &[], false),
    ("Expired", 3, &[], false),
    ("NoSignIn", 2, &[], false),
    ("Refused", 2, &[], false),
    ("RateLimited", 2, &[], false),
    ("Offline", 2, &[], false),
    ("BadAnswer", 2, &[], false),
];

#[test]
fn claude_answers_as_the_csharp_did() {
    for (state, misses, limits, expected) in CLAUDE {
        let limits: Vec<Limit> = limits
            .iter()
            .map(|(kind, age)| aged(kind_of(kind), span(*age)))
            .collect();
        assert_eq!(
            pill_stale::claude(state_of(state), &limits, now(), *misses),
            *expected,
            "{state} {misses} {limits:?}"
        );
    }
}

/// A state, the polls missed, the age in ticks of the credits figure (none for no figure), and
/// what the C# answered for `PillStale.Copilot` at noon on 2026-11-13.
const COPILOT: &[(&str, i32, Option<i64>, bool)] = &[
    ("Off", 2, None, false),
    ("Ok", 0, None, false),
    ("Ok", 1, None, false),
    ("Ok", 2, None, false),
    ("Ok", 3, None, false),
    ("Expired", 2, None, false),
    ("NoSignIn", 0, None, false),
    ("NoSignIn", 1, None, false),
    ("NoSignIn", 2, None, false),
    ("NoSignIn", 3, None, false),
    ("Refused", 2, None, false),
    ("RateLimited", 2, None, false),
    ("Offline", 2, None, false),
    ("BadAnswer", 2, None, false),
    ("Off", 2, Some(72000000000), false),
    ("Ok", 0, Some(72000000000), false),
    ("Ok", 1, Some(72000000000), false),
    ("Ok", 2, Some(72000000000), false),
    ("Ok", 3, Some(72000000000), false),
    ("Expired", 2, Some(72000000000), false),
    ("NoSignIn", 0, Some(72000000000), false),
    ("NoSignIn", 1, Some(72000000000), false),
    ("NoSignIn", 2, Some(72000000000), false),
    ("NoSignIn", 3, Some(72000000000), false),
    ("Refused", 2, Some(72000000000), false),
    ("RateLimited", 2, Some(72000000000), false),
    ("Offline", 2, Some(72000000000), false),
    ("BadAnswer", 2, Some(72000000000), false),
    ("Off", 2, Some(864000000000), false),
    ("Ok", 0, Some(864000000000), false),
    ("Ok", 1, Some(864000000000), false),
    ("Ok", 2, Some(864000000000), false),
    ("Ok", 3, Some(864000000000), false),
    ("Expired", 2, Some(864000000000), false),
    ("NoSignIn", 0, Some(864000000000), false),
    ("NoSignIn", 1, Some(864000000000), false),
    ("NoSignIn", 2, Some(864000000000), false),
    ("NoSignIn", 3, Some(864000000000), false),
    ("Refused", 2, Some(864000000000), false),
    ("RateLimited", 2, Some(864000000000), false),
    ("Offline", 2, Some(864000000000), false),
    ("BadAnswer", 2, Some(864000000000), false),
    ("Off", 2, Some(864000000001), false),
    ("Ok", 0, Some(864000000001), false),
    ("Ok", 1, Some(864000000001), false),
    ("Ok", 2, Some(864000000001), false),
    ("Ok", 3, Some(864000000001), false),
    ("Expired", 2, Some(864000000001), true),
    ("NoSignIn", 0, Some(864000000001), false),
    ("NoSignIn", 1, Some(864000000001), false),
    ("NoSignIn", 2, Some(864000000001), true),
    ("NoSignIn", 3, Some(864000000001), true),
    ("Refused", 2, Some(864000000001), true),
    ("RateLimited", 2, Some(864000000001), true),
    ("Offline", 2, Some(864000000001), true),
    ("BadAnswer", 2, Some(864000000001), true),
    ("Off", 2, Some(2592000000000), false),
    ("Ok", 0, Some(2592000000000), false),
    ("Ok", 1, Some(2592000000000), false),
    ("Ok", 2, Some(2592000000000), false),
    ("Ok", 3, Some(2592000000000), false),
    ("Expired", 2, Some(2592000000000), true),
    ("NoSignIn", 0, Some(2592000000000), false),
    ("NoSignIn", 1, Some(2592000000000), false),
    ("NoSignIn", 2, Some(2592000000000), true),
    ("NoSignIn", 3, Some(2592000000000), true),
    ("Refused", 2, Some(2592000000000), true),
    ("RateLimited", 2, Some(2592000000000), true),
    ("Offline", 2, Some(2592000000000), true),
    ("BadAnswer", 2, Some(2592000000000), true),
    ("Off", 2, Some(-36000000000), false),
    ("Ok", 0, Some(-36000000000), false),
    ("Ok", 1, Some(-36000000000), false),
    ("Ok", 2, Some(-36000000000), false),
    ("Ok", 3, Some(-36000000000), false),
    ("Expired", 2, Some(-36000000000), false),
    ("NoSignIn", 0, Some(-36000000000), false),
    ("NoSignIn", 1, Some(-36000000000), false),
    ("NoSignIn", 2, Some(-36000000000), false),
    ("NoSignIn", 3, Some(-36000000000), false),
    ("Refused", 2, Some(-36000000000), false),
    ("RateLimited", 2, Some(-36000000000), false),
    ("Offline", 2, Some(-36000000000), false),
    ("BadAnswer", 2, Some(-36000000000), false),
];

#[test]
fn copilot_answers_as_the_csharp_did() {
    for (state, misses, age, expected) in COPILOT {
        let credits = age.map(|age| credits(span(age)));
        assert_eq!(
            pill_stale::copilot(state_of(state), credits.as_ref(), now(), *misses),
            *expected,
            "{state} {misses} {age:?}"
        );
    }
}

#[test]
fn the_rows_hold_both_answers() {
    for rows in [
        CLAUDE.iter().map(|row| row.3).collect::<Vec<bool>>(),
        COPILOT.iter().map(|row| row.3).collect::<Vec<bool>>(),
    ] {
        assert!(rows.contains(&true));
        assert!(rows.contains(&false));
    }
}
