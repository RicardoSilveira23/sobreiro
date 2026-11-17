//! Ported from `tests/Sobreiro.Tests/PillShowTests.cs`, which holds `PillShowTests` and
//! `ZOrderTests`. The last two tests are new.
use sobreiro_core::taskbar::pill_show::{PillShowStep, ZOrder, step};

#[test]
fn shown_in_place_above_the_taskbar_does_nothing() {
    assert_eq!(step(true, true, true), PillShowStep::None);
}

#[test]
fn shown_above_the_taskbar_elsewhere_moves() {
    assert_eq!(step(true, true, false), PillShowStep::Move);
}

// After a full-screen window closes, the pill is shown again where it was, but Explorer raised the
// taskbar past it while it was hidden: it is "shown, in place" and covered.
#[test]
fn shown_below_the_taskbar_raises() {
    for in_place in [true, false] {
        assert_eq!(step(true, false, in_place), PillShowStep::Raise);
    }
}

#[test]
fn hidden_is_shown_not_just_raised() {
    for (above_taskbar, in_place) in [(true, true), (true, false), (false, true), (false, false)] {
        let got = step(false, above_taskbar, in_place);
        assert_eq!(got, PillShowStep::Show);
        assert_ne!(got, PillShowStep::Raise);
    }
}

// Topmost first: 1 is the top window, 4 the bottom one.
const STACK: [isize; 4] = [1, 2, 3, 4];

fn above(window: isize) -> isize {
    match STACK.iter().position(|&w| w == window) {
        Some(i) if i > 0 => STACK[i - 1],
        _ => 0,
    }
}

#[test]
fn a_window_higher_in_the_stack_is_above() {
    assert!(ZOrder::is_above(2, 4, above));
}

#[test]
fn the_top_window_is_above() {
    assert!(ZOrder::is_above(1, 3, above));
}

#[test]
fn a_window_lower_in_the_stack_is_not_above() {
    assert!(!ZOrder::is_above(4, 2, above));
}

#[test]
fn a_window_is_not_above_itself() {
    assert!(!ZOrder::is_above(3, 3, above));
}

#[test]
fn a_window_not_in_the_stack_is_not_above() {
    assert!(!ZOrder::is_above(9, 3, above));
}

// The stack can change during the walk; a loop must end.
#[test]
fn a_cycle_ends() {
    assert!(!ZOrder::is_above(9, 1, |w| if w == 1 { 2 } else { 1 }));
}

#[test]
fn the_walk_looks_at_4096_windows_and_no_more() {
    // Each window's next one up is the one numbered after it, and never 0.
    let mut asked = 0;
    assert!(!ZOrder::is_above(-1, 10, |w| {
        asked += 1;
        w + 1
    }));
    assert_eq!(asked, 4096);
    assert!(ZOrder::is_above(10 + 4096, 10, |w| w + 1));
    assert!(!ZOrder::is_above(10 + 4097, 10, |w| w + 1));
}

#[test]
fn the_walk_stops_at_the_top_without_asking_again() {
    let mut asked = 0;
    assert!(!ZOrder::is_above(9, 3, |w| {
        asked += 1;
        above(w)
    }));
    assert_eq!(asked, 3);
}
