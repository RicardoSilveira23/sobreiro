//! What counts as a pill. Ported from 1.x's PillSightingTests.

use sobreiro_baseline::{Rect, WindowFacts, is_pill};

const TASKBAR: isize = 0x1234;
const TASKBAR_RECT: Rect = Rect::new(0, 1392, 2560, 1440);
const IN_TASKBAR: Rect = Rect::new(2222, 1397, 2310, 1435);

fn pill(visible: bool, owner: isize, rect: Rect) -> bool {
    is_pill(
        &WindowFacts {
            visible,
            owner,
            rect,
        },
        TASKBAR,
        &TASKBAR_RECT,
    )
}

#[test]
fn a_visible_window_owned_by_the_taskbar_and_inside_it_is_the_pill() {
    assert!(pill(true, TASKBAR, IN_TASKBAR));
}

#[test]
fn a_hidden_window_is_not() {
    assert!(!pill(false, TASKBAR, IN_TASKBAR));
}

#[test]
fn a_window_with_another_owner_is_not() {
    assert!(!pill(true, 0, IN_TASKBAR));
}

// The pill is first shown far off screen, to be measured, and only then moved over the taskbar.
#[test]
fn a_window_parked_off_screen_is_not() {
    assert!(!pill(
        true,
        TASKBAR,
        Rect::new(-32000, -32000, -31912, -31962)
    ));
}

#[test]
fn a_window_that_reaches_above_the_taskbar_is_not() {
    assert!(!pill(true, TASKBAR, Rect::new(2100, 900, 2500, 1400)));
}

#[test]
fn an_empty_window_is_not() {
    assert!(!pill(true, TASKBAR, Rect::new(2222, 1397, 2222, 1397)));
}

#[test]
fn nothing_is_the_pill_without_a_taskbar() {
    let facts = WindowFacts {
        visible: true,
        owner: 0,
        rect: IN_TASKBAR,
    };
    assert!(!is_pill(&facts, 0, &TASKBAR_RECT));
}
