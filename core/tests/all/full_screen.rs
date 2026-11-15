//! Ported from `tests/Sobreiro.Tests/FullScreenTests.cs`. The test after
//! `busy_d3d_and_presentation_are_full_screen` is new.
use sobreiro_core::taskbar::full_screen::is_full_screen;

#[test]
fn busy_d3d_and_presentation_are_full_screen() {
    for (state, full) in [
        (2, true),
        (3, true),
        (4, true),
        (1, false),
        (5, false),
        (7, false),
    ] {
        assert_eq!(is_full_screen(state), full, "state {state}");
    }
}

#[test]
fn states_at_the_ends_of_an_i32_and_below_zero_are_not_full_screen() {
    for state in [i32::MIN, -3, 0, i32::MAX] {
        assert!(!is_full_screen(state), "state {state}");
    }
}
