//! Ported from `tests/Sobreiro.Tests/RingsModelTests.cs`. The last two tests walk rows that the
//! C# of 1.x wrote out itself.
use chrono::{DateTime, Utc};
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::display::rings::{Ring, RingsModel};
use sobreiro_core::limits::{Limit, LimitKind, UsageLevel, limit_list};

use crate::time;

fn now() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
}

fn l(kind: LimitKind, percent: f64) -> Limit {
    Limit::new(
        "Claude Code",
        kind,
        label(kind),
        percent,
        None,
        now(),
        false,
    )
}

fn label(kind: LimitKind) -> &'static str {
    match kind {
        LimitKind::Session => "Session",
        LimitKind::Weekly => "Weekly",
        LimitKind::Spend => "Spend",
        LimitKind::Credits => "Credits",
    }
}

fn ring(percent: i32, level: UsageLevel) -> Ring {
    Ring {
        percent,
        level,
        has_data: true,
    }
}

fn none_model() -> RingsModel {
    RingsModel {
        outer: Ring::NONE,
        inner: Ring::NONE,
        has_inner: true,
    }
}

#[test]
fn no_limits_gives_two_empty_rings() {
    assert_eq!(RingsModel::from(&[]), none_model());
}

#[test]
fn outer_is_session_and_inner_is_weekly() {
    assert_eq!(
        RingsModel::from(&[l(LimitKind::Weekly, 61.0), l(LimitKind::Session, 37.9)]),
        RingsModel {
            outer: ring(37, UsageLevel::Normal),
            inner: ring(61, UsageLevel::Normal),
            has_inner: true,
        }
    );
}

#[test]
fn each_ring_takes_its_own_level() {
    for (percent, level) in [
        (69, UsageLevel::Normal),
        (70, UsageLevel::Warning),
        (89, UsageLevel::Warning),
        (90, UsageLevel::Error),
    ] {
        let m = RingsModel::from(&[
            l(LimitKind::Session, f64::from(percent)),
            l(LimitKind::Weekly, 10.0),
        ]);
        assert_eq!(m.outer.level, level, "{percent}");
        assert_eq!(m.inner.level, UsageLevel::Normal, "{percent}");
    }
}

#[test]
fn percent_is_clamped_to_100() {
    assert_eq!(
        RingsModel::from(&[l(LimitKind::Session, 130.0)])
            .outer
            .percent,
        100
    );
}

#[test]
fn a_reset_window_is_zero() {
    let reset = Limit::new(
        "Claude Code",
        LimitKind::Session,
        "Session",
        0.0,
        Some(now()),
        now(),
        true,
    );
    assert_eq!(
        RingsModel::from(&[reset]).outer,
        ring(0, UsageLevel::Normal)
    );
}

#[test]
fn spend_is_not_drawn() {
    assert_eq!(RingsModel::from(&[l(LimitKind::Spend, 50.0)]), none_model());
}

#[test]
fn sweep_is_clockwise_degrees() {
    for (percent, degrees) in [(0, 0.0), (25, 90.0), (100, 360.0)] {
        assert_eq!(
            ring(percent, UsageLevel::Normal).sweep_degrees(),
            degrees,
            "{percent}"
        );
    }
}

#[test]
fn no_data_sweeps_nothing() {
    assert_eq!(Ring::NONE.sweep_degrees(), 0.0);
}

#[test]
fn capped_sweep_takes_the_round_caps_off_the_arc() {
    let sweep = ring(95, UsageLevel::Normal).capped_sweep(3.25, 2.5);
    // To two decimals, as the C# test asks.
    assert_eq!((sweep * 100.0).round() as i64, 29793);
}

#[test]
fn capped_sweep_is_the_full_circle_at_100() {
    assert_eq!(ring(100, UsageLevel::Normal).capped_sweep(3.25, 2.5), 360.0);
}

#[test]
fn capped_sweep_is_nothing_at_0_or_without_data() {
    assert_eq!(ring(0, UsageLevel::Normal).capped_sweep(3.25, 2.5), 0.0);
    assert_eq!(Ring::NONE.capped_sweep(3.25, 2.5), 0.0);
}

#[test]
fn capped_sweep_leaves_a_dot_for_tiny_percentages() {
    assert_eq!(ring(3, UsageLevel::Normal).capped_sweep(3.25, 2.5), 0.5);
}

fn credits() -> Limit {
    let reading = CopilotReading {
        budget: 15000.0,
        used: 2644.0,
        resets_at: Some(time("2026-12-01T00:00:00Z")),
        observed_at: now(),
        source: CopilotSource::Live,
    };
    limit_list::from_copilot(Some(&reading), now()).remove(0)
}

#[test]
fn copilot_rings_have_one_ring() {
    let m = RingsModel::copilot(&[credits()]);
    assert!(!m.has_inner);
    assert_eq!(m.outer.percent, 17);
    assert_eq!(m.inner, Ring::NONE);
}

/// One ring as the rows hold it: the percentage, the level's name and whether there is data.
type RingRow = (i32, &'static str, bool);

/// A model as the rows hold it: the outer ring, the inner ring and whether there is an inner.
type ModelRow = (RingRow, RingRow, bool);

/// Limits as (kind, percentage), then what the C# made of them: `RingsModel.From`, then
/// `RingsModel.Copilot`.
type ModelsRow = (&'static [(&'static str, f64)], ModelRow, ModelRow);

const MODELS: &[ModelsRow] = &[
    (
        &[],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", -5.0)],
        ((0, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 0.0)],
        ((0, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 0.5)],
        ((0, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 1.0)],
        ((1, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 3.0)],
        ((3, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 25.0)],
        ((25, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 37.9)],
        ((37, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 61.0)],
        ((61, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 69.99)],
        ((69, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 70.0)],
        ((70, "Warning", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 89.99)],
        ((89, "Warning", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 90.0)],
        ((90, "Error", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 99.9)],
        ((99, "Error", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 100.0)],
        ((100, "Error", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 130.0)],
        ((100, "Error", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", -5.0)],
        ((0, "Normal", false), (0, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 0.0)],
        ((0, "Normal", false), (0, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 0.5)],
        ((0, "Normal", false), (0, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 1.0)],
        ((0, "Normal", false), (1, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 3.0)],
        ((0, "Normal", false), (3, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 25.0)],
        ((0, "Normal", false), (25, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 37.9)],
        ((0, "Normal", false), (37, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 61.0)],
        ((0, "Normal", false), (61, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 69.99)],
        ((0, "Normal", false), (69, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 70.0)],
        ((0, "Normal", false), (70, "Warning", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 89.99)],
        ((0, "Normal", false), (89, "Warning", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 90.0)],
        ((0, "Normal", false), (90, "Error", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 99.9)],
        ((0, "Normal", false), (99, "Error", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 100.0)],
        ((0, "Normal", false), (100, "Error", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 130.0)],
        ((0, "Normal", false), (100, "Error", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", -5.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 0.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 0.5)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 1.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 3.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 25.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 37.9)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 61.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 69.99)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 70.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 89.99)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 90.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 99.9)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 100.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 130.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Credits", -5.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 0.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 0.5)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((0, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 1.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((1, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 3.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((3, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 25.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((25, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 37.9)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((37, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 61.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((61, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 69.99)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((69, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 70.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((70, "Warning", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 89.99)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((89, "Warning", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 90.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((90, "Error", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 99.9)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((99, "Error", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 100.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((100, "Error", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 130.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((100, "Error", true), (0, "Normal", false), false),
    ),
    (
        &[("Weekly", 61.0), ("Session", 37.9)],
        ((37, "Normal", true), (61, "Normal", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 10.0), ("Session", 80.0)],
        ((10, "Normal", true), (0, "Normal", false), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[
            ("Weekly", 5.0),
            ("Session", 99.0),
            ("Spend", 50.0),
            ("Credits", 20.0),
        ],
        ((99, "Error", true), (5, "Normal", true), true),
        ((20, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Credits", 17.0), ("Credits", 90.0)],
        ((0, "Normal", false), (0, "Normal", false), true),
        ((17, "Normal", true), (0, "Normal", false), false),
    ),
    (
        &[("Spend", 50.0), ("Credits", 95.0), ("Weekly", 70.0)],
        ((0, "Normal", false), (70, "Warning", true), true),
        ((95, "Error", true), (0, "Normal", false), false),
    ),
    (
        &[("Session", 69.0), ("Weekly", 90.0)],
        ((69, "Normal", true), (90, "Error", true), true),
        ((0, "Normal", false), (0, "Normal", false), false),
    ),
    (
        &[("Session", 90.0), ("Weekly", 10.0), ("Credits", 71.0)],
        ((90, "Error", true), (10, "Normal", true), true),
        ((71, "Warning", true), (0, "Normal", false), false),
    ),
];

fn kind(name: &str) -> LimitKind {
    match name {
        "Session" => LimitKind::Session,
        "Weekly" => LimitKind::Weekly,
        "Spend" => LimitKind::Spend,
        _ => LimitKind::Credits,
    }
}

fn level(name: &str) -> UsageLevel {
    match name {
        "Normal" => UsageLevel::Normal,
        "Warning" => UsageLevel::Warning,
        _ => UsageLevel::Error,
    }
}

fn expected_ring((percent, name, has_data): RingRow) -> Ring {
    Ring {
        percent,
        level: level(name),
        has_data,
    }
}

fn expected_model((outer, inner, has_inner): ModelRow) -> RingsModel {
    RingsModel {
        outer: expected_ring(outer),
        inner: expected_ring(inner),
        has_inner,
    }
}

#[test]
fn rings_model_answers_as_the_csharp_did() {
    for (inputs, from, copilot) in MODELS {
        let limits: Vec<Limit> = inputs
            .iter()
            .map(|(name, percent)| l(kind(name), *percent))
            .collect();
        assert_eq!(
            RingsModel::from(&limits),
            expected_model(*from),
            "from {inputs:?}"
        );
        assert_eq!(
            RingsModel::copilot(&limits),
            expected_model(*copilot),
            "copilot {inputs:?}"
        );
    }
}

/// A ring's percentage, whether it has data, a radius and a stroke, then what the C# gave: the
/// sweep and the capped sweep, each as the bits of the number.
const SWEEPS: &[(i32, bool, f64, f64, u64, u64)] = &[
    (0, true, 3.25, 2.5, 0x0000000000000000, 0x0000000000000000),
    (0, true, 10.0, 2.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 1.0, 0.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 0.5, 3.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 100.0, 0.1, 0x0000000000000000, 0x0000000000000000),
    (0, true, 3.25, 0.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 0.0, 2.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 0.0, 0.0, 0x0000000000000000, 0x0000000000000000),
    (0, true, 7.0, 5.0, 0x0000000000000000, 0x0000000000000000),
    (1, true, 3.25, 2.5, 0x400ccccccccccccd, 0x3fe0000000000000),
    (1, true, 10.0, 2.0, 0x400ccccccccccccd, 0x3fe0000000000000),
    (1, true, 1.0, 0.0, 0x400ccccccccccccd, 0x400ccccccccccccd),
    (1, true, 0.5, 3.0, 0x400ccccccccccccd, 0x3fe0000000000000),
    (1, true, 100.0, 0.1, 0x400ccccccccccccd, 0x400c57754f7323cf),
    (1, true, 3.25, 0.0, 0x400ccccccccccccd, 0x400ccccccccccccd),
    (1, true, 0.0, 2.0, 0x400ccccccccccccd, 0x3fe0000000000000),
    (1, true, 0.0, 0.0, 0x400ccccccccccccd, 0xfff8000000000000),
    (1, true, 7.0, 5.0, 0x400ccccccccccccd, 0x3fe0000000000000),
    (2, true, 3.25, 2.5, 0x401ccccccccccccd, 0x3fe0000000000000),
    (2, true, 10.0, 2.0, 0x401ccccccccccccd, 0x3fe0000000000000),
    (2, true, 1.0, 0.0, 0x401ccccccccccccd, 0x401ccccccccccccd),
    (2, true, 0.5, 3.0, 0x401ccccccccccccd, 0x3fe0000000000000),
    (2, true, 100.0, 0.1, 0x401ccccccccccccd, 0x401c92210e1ff84e),
    (2, true, 3.25, 0.0, 0x401ccccccccccccd, 0x401ccccccccccccd),
    (2, true, 0.0, 2.0, 0x401ccccccccccccd, 0x3fe0000000000000),
    (2, true, 0.0, 0.0, 0x401ccccccccccccd, 0xfff8000000000000),
    (2, true, 7.0, 5.0, 0x401ccccccccccccd, 0x3fe0000000000000),
    (3, true, 3.25, 2.5, 0x402599999999999a, 0x3fe0000000000000),
    (3, true, 10.0, 2.0, 0x402599999999999a, 0x3fe0000000000000),
    (3, true, 1.0, 0.0, 0x402599999999999a, 0x402599999999999a),
    (3, true, 0.5, 3.0, 0x402599999999999a, 0x3fe0000000000000),
    (3, true, 100.0, 0.1, 0x402599999999999a, 0x40257c43ba432f5b),
    (3, true, 3.25, 0.0, 0x402599999999999a, 0x402599999999999a),
    (3, true, 0.0, 2.0, 0x402599999999999a, 0x3fe0000000000000),
    (3, true, 0.0, 0.0, 0x402599999999999a, 0xfff8000000000000),
    (3, true, 7.0, 5.0, 0x402599999999999a, 0x3fe0000000000000),
    (5, true, 3.25, 2.5, 0x4032000000000000, 0x3fe0000000000000),
    (5, true, 10.0, 2.0, 0x4032000000000000, 0x401a29d308f9fcda),
    (5, true, 1.0, 0.0, 0x4032000000000000, 0x4032000000000000),
    (5, true, 0.5, 3.0, 0x4032000000000000, 0x3fe0000000000000),
    (5, true, 100.0, 0.1, 0x4032000000000000, 0x4031f1551054cae0),
    (5, true, 3.25, 0.0, 0x4032000000000000, 0x4032000000000000),
    (5, true, 0.0, 2.0, 0x4032000000000000, 0x3fe0000000000000),
    (5, true, 0.0, 0.0, 0x4032000000000000, 0xfff8000000000000),
    (5, true, 7.0, 5.0, 0x4032000000000000, 0x3fe0000000000000),
    (25, true, 3.25, 2.5, 0x4056800000000000, 0x4046f691c450cd40),
    (25, true, 10.0, 2.0, 0x4056800000000000, 0x4053a29d308f9fce),
    (25, true, 1.0, 0.0, 0x4056800000000000, 0x4056800000000000),
    (25, true, 0.5, 3.0, 0x4056800000000000, 0x3fe0000000000000),
    (25, true, 100.0, 0.1, 0x4056800000000000, 0x40567c55441532b8),
    (25, true, 3.25, 0.0, 0x4056800000000000, 0x4056800000000000),
    (25, true, 0.0, 2.0, 0x4056800000000000, 0x3fe0000000000000),
    (25, true, 0.0, 0.0, 0x4056800000000000, 0xfff8000000000000),
    (25, true, 7.0, 5.0, 0x4056800000000000, 0x404889875add50e1),
    (50, true, 3.25, 2.5, 0x4066800000000000, 0x4060fda471143350),
    (50, true, 10.0, 2.0, 0x4066800000000000, 0x4065114e9847cfe7),
    (50, true, 1.0, 0.0, 0x4066800000000000, 0x4066800000000000),
    (50, true, 0.5, 3.0, 0x4066800000000000, 0x3fe0000000000000),
    (50, true, 100.0, 0.1, 0x4066800000000000, 0x40667e2aa20a995c),
    (50, true, 3.25, 0.0, 0x4066800000000000, 0x4066800000000000),
    (50, true, 0.0, 2.0, 0x4066800000000000, 0x3fe0000000000000),
    (50, true, 0.0, 0.0, 0x4066800000000000, 0xfff8000000000000),
    (50, true, 7.0, 5.0, 0x4066800000000000, 0x40616261d6b75438),
    (95, true, 3.25, 2.5, 0x4075600000000000, 0x40729ed2388a19a8),
    (95, true, 10.0, 2.0, 0x4075600000000000, 0x4074a8a74c23e7f3),
    (95, true, 1.0, 0.0, 0x4075600000000000, 0x4075600000000000),
    (95, true, 0.5, 3.0, 0x4075600000000000, 0x3fe0000000000000),
    (95, true, 100.0, 0.1, 0x4075600000000000, 0x40755f1551054cae),
    (95, true, 3.25, 0.0, 0x4075600000000000, 0x4075600000000000),
    (95, true, 0.0, 2.0, 0x4075600000000000, 0x3fe0000000000000),
    (95, true, 0.0, 0.0, 0x4075600000000000, 0xfff8000000000000),
    (95, true, 7.0, 5.0, 0x4075600000000000, 0x4072d130eb5baa1c),
    (99, true, 3.25, 2.5, 0x4076466666666666, 0x407385389ef0800e),
    (99, true, 10.0, 2.0, 0x4076466666666666, 0x40758f0db28a4e59),
    (99, true, 1.0, 0.0, 0x4076466666666666, 0x4076466666666666),
    (99, true, 0.5, 3.0, 0x4076466666666666, 0x4029402a53729d80),
    (99, true, 100.0, 0.1, 0x4076466666666666, 0x4076457bb76bb314),
    (99, true, 3.25, 0.0, 0x4076466666666666, 0x4076466666666666),
    (99, true, 0.0, 2.0, 0x4076466666666666, 0x3fe0000000000000),
    (99, true, 0.0, 0.0, 0x4076466666666666, 0xfff8000000000000),
    (99, true, 7.0, 5.0, 0x4076466666666666, 0x4073b79751c21082),
    (100, true, 3.25, 2.5, 0x4076800000000000, 0x4076800000000000),
    (100, true, 10.0, 2.0, 0x4076800000000000, 0x4076800000000000),
    (100, true, 1.0, 0.0, 0x4076800000000000, 0x4076800000000000),
    (100, true, 0.5, 3.0, 0x4076800000000000, 0x4076800000000000),
    (
        100,
        true,
        100.0,
        0.1,
        0x4076800000000000,
        0x4076800000000000,
    ),
    (100, true, 3.25, 0.0, 0x4076800000000000, 0x4076800000000000),
    (100, true, 0.0, 2.0, 0x4076800000000000, 0x4076800000000000),
    (100, true, 0.0, 0.0, 0x4076800000000000, 0x4076800000000000),
    (100, true, 7.0, 5.0, 0x4076800000000000, 0x4076800000000000),
    (120, true, 3.25, 2.5, 0x407b000000000000, 0x4076800000000000),
    (120, true, 10.0, 2.0, 0x407b000000000000, 0x4076800000000000),
    (120, true, 1.0, 0.0, 0x407b000000000000, 0x4076800000000000),
    (120, true, 0.5, 3.0, 0x407b000000000000, 0x4076800000000000),
    (
        120,
        true,
        100.0,
        0.1,
        0x407b000000000000,
        0x4076800000000000,
    ),
    (120, true, 3.25, 0.0, 0x407b000000000000, 0x4076800000000000),
    (120, true, 0.0, 2.0, 0x407b000000000000, 0x4076800000000000),
    (120, true, 0.0, 0.0, 0x407b000000000000, 0x4076800000000000),
    (120, true, 7.0, 5.0, 0x407b000000000000, 0x4076800000000000),
    (0, false, 3.25, 2.5, 0x0000000000000000, 0x0000000000000000),
    (40, false, 3.25, 2.5, 0x0000000000000000, 0x0000000000000000),
    (
        100,
        false,
        3.25,
        2.5,
        0x0000000000000000,
        0x0000000000000000,
    ),
];

#[test]
fn ring_sweeps_answer_as_the_csharp_did() {
    for (percent, has_data, radius, stroke, sweep, capped) in SWEEPS {
        let ring = Ring {
            percent: *percent,
            level: UsageLevel::Normal,
            has_data: *has_data,
        };
        let row = format!("{percent}% data {has_data} radius {radius} stroke {stroke}");
        assert_eq!(ring.sweep_degrees().to_bits(), *sweep, "{row}");
        let got = ring.capped_sweep(*radius, *stroke);
        let expected = f64::from_bits(*capped);
        if expected.is_nan() {
            assert!(got.is_nan(), "{row}: {got}");
        } else {
            assert_eq!(got.to_bits(), *capped, "{row}: {got}");
        }
    }
}
