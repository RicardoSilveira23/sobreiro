// Ports of FullScreenTests, PillPlacementTests, PillShowTests (with ZOrderTests), TaskbarLayoutTests,
// TaskbarQueryScheduleTests.
use sobreiro_core::Span;
use sobreiro_core::display::PixelRect;
use sobreiro_core::taskbar::*;

fn r(l: i32, t: i32, rt: i32, b: i32) -> PixelRect {
    PixelRect::new(l, t, rt, b)
}

#[test]
fn busy_d3d_and_presentation_are_full_screen() {
    for (state, full) in [(2, true), (3, true), (4, true), (1, false), (5, false), (7, false)] {
        assert_eq!(FullScreen::is_full_screen(state), full);
    }
}

// ---- PillPlacementTests

const MONITOR: PixelRect = PixelRect::new(0, 0, 2560, 1440);
const AT100: TaskbarAnchors = TaskbarAnchors::new(2314, 1589, 0, 48);
const BAR: PixelRect = PixelRect::new(0, 1392, 2560, 1440);

fn place(x: i32, y: i32, band_top: i32) -> PillPlace {
    PillPlace { x, y, hidden: None, band_top }
}

#[test]
fn places_left_of_the_tray_at_100_percent() {
    assert_eq!(PillPlacement::place(MONITOR, BAR, Some(AT100), 88, 38, 4), place(2222, 1397, 1392));
}
#[test]
fn places_left_of_the_tray_at_125_percent() {
    assert_eq!(
        PillPlacement::place(MONITOR, r(0, 1380, 2560, 1440), Some(TaskbarAnchors::new(2253, 1693, 0, 60)), 110, 47, 5),
        place(2138, 1386, 1380)
    );
}
#[test]
fn places_on_a_monitor_not_at_the_origin() {
    assert_eq!(PillPlacement::place(r(-2560, 0, 0, 1440), r(-2560, 1392, 0, 1440), Some(AT100), 88, 38, 4), place(-338, 1397, 1392));
}
#[test]
fn no_anchors_hides() {
    assert_eq!(PillPlacement::place(MONITOR, BAR, None, 88, 38, 4).hidden, Some(PillHidden::NoAnchors));
}
#[test]
fn a_taskbar_partly_off_the_monitor_hides() {
    for top in [1404, 1438] {
        // sliding away, then auto-hidden
        assert_eq!(PillPlacement::place(MONITOR, r(0, top, 2560, top + 48), Some(AT100), 88, 38, 4).hidden, Some(PillHidden::OffScreen));
    }
}
#[test]
fn hides_when_the_pill_is_taller_than_the_band() {
    assert_eq!(PillPlacement::place(MONITOR, BAR, Some(AT100), 88, 50, 4).hidden, Some(PillHidden::NoRoom));
}
#[test]
fn hides_when_it_would_cover_the_app_buttons() {
    let a = TaskbarAnchors { buttons_right: 2230, ..AT100 };
    assert_eq!(PillPlacement::place(MONITOR, BAR, Some(a), 88, 38, 4).hidden, Some(PillHidden::NoRoom));
}
#[test]
fn just_enough_room_shows() {
    // x 2222 == buttons 2218 + gap 4
    let a = TaskbarAnchors { buttons_right: 2218, ..AT100 };
    assert!(PillPlacement::place(MONITOR, BAR, Some(a), 88, 38, 4).is_shown());
}
#[test]
fn a_taskbar_at_the_top_hides() {
    let a = TaskbarAnchors { band_top: 0, band_bottom: 48, ..AT100 };
    assert_eq!(PillPlacement::place(MONITOR, r(0, 0, 2560, 48), Some(a), 88, 38, 4).hidden, Some(PillHidden::NotAtBottom));
}
#[test]
fn a_taskbar_on_the_side_hides() {
    assert_eq!(PillPlacement::place(MONITOR, r(2512, 0, 2560, 1440), Some(AT100), 88, 38, 4).hidden, Some(PillHidden::NotAtBottom));
}

// ---- PillShowTests

#[test]
fn shown_in_place_above_the_taskbar_does_nothing() {
    assert_eq!(PillShow::step(true, true, true), PillShowStep::None);
}
#[test]
fn shown_above_the_taskbar_elsewhere_moves() {
    assert_eq!(PillShow::step(true, true, false), PillShowStep::Move);
}
// After a full-screen window closes, the pill is shown again where it was, but Explorer raised the taskbar past it
// while it was hidden: it is "shown, in place" and covered.
#[test]
fn shown_below_the_taskbar_raises() {
    for in_place in [true, false] {
        assert_eq!(PillShow::step(true, false, in_place), PillShowStep::Raise);
    }
}
#[test]
fn hidden_raises() {
    for (above, in_place) in [(true, true), (true, false), (false, true), (false, false)] {
        assert_eq!(PillShow::step(false, above, in_place), PillShowStep::Raise);
    }
}

// ---- ZOrderTests: topmost first, 1 is the top window, 4 the bottom one.

fn above(window: isize) -> isize {
    let stack = [1isize, 2, 3, 4];
    match stack.iter().position(|&w| w == window) {
        Some(i) if i > 0 => stack[i - 1],
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

// ---- TaskbarLayoutTests

const EXPLORER: i32 = 4556;
const OWN: i32 = 444;
const FRAME: &str = "Taskbar.TaskbarFrameAutomationPeer";

#[test]
fn anchors_from_the_spike() {
    let items = [
        ("SystemTray.NormalButton", r(2314, 1392, 2346, 1440), EXPLORER),
        ("SystemTray.OmniButton", r(2474, 1392, 2544, 1440), EXPLORER),
        (FRAME, r(0, 1392, 2560, 1440), EXPLORER),
    ];
    let buttons = [r(972, 1392, 1017, 1440), r(1545, 1392, 1589, 1440)];
    assert_eq!(TaskbarLayout::anchors(BAR, items, buttons, OWN), Some(TaskbarAnchors::new(2314, 1589, 0, 48)));
}
#[test]
fn anchors_without_tray_items_is_null() {
    assert_eq!(TaskbarLayout::anchors(BAR, [(FRAME, BAR, EXPLORER)], [], OWN), None);
}
#[test]
fn empty_tray_items_are_skipped() {
    assert_eq!(TaskbarLayout::anchors(BAR, [("SystemTray.NormalButton", r(2314, 1392, 2314, 1440), EXPLORER)], [], OWN), None);
}
#[test]
fn no_buttons_means_the_taskbars_left_edge() {
    let a = TaskbarLayout::anchors(BAR, [("SystemTray.OmniButton", r(2474, 1392, 2544, 1440), EXPLORER)], [], OWN).unwrap();
    assert_eq!(a.buttons_right, 0);
}
#[test]
fn anchors_are_offsets_from_the_taskbar() {
    assert_eq!(
        TaskbarLayout::anchors(r(2000, 1392, 4560, 1440), [("SystemTray.NormalButton", r(2314, 1392, 2346, 1440), EXPLORER)], [r(2050, 1392, 2089, 1440)], OWN),
        Some(TaskbarAnchors::new(314, 89, 0, 48))
    );
}
#[test]
fn an_unknown_item_left_of_the_tray_is_an_obstacle() {
    let a = TaskbarLayout::anchors(
        BAR,
        [("SystemTray.NormalButton", r(2314, 1392, 2346, 1440), EXPLORER), ("ToggleButton", r(2200, 1392, 2250, 1440), EXPLORER)],
        [r(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(a.unwrap().buttons_right, 2250);
}
#[test]
fn a_pane_spanning_the_whole_taskbar_is_not_an_obstacle() {
    let a = TaskbarLayout::anchors(
        BAR,
        [("SystemTray.NormalButton", r(2314, 1392, 2346, 1440), EXPLORER), ("Windows.UI.Input.InputSite.WindowClass", BAR, EXPLORER)],
        [r(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(a.unwrap().buttons_right, 1017);
}
#[test]
fn own_windows_are_not_obstacles() {
    let a = TaskbarLayout::anchors(
        BAR,
        [("SystemTray.NormalButton", r(2314, 1392, 2346, 1440), EXPLORER), ("Window", r(2200, 1392, 2300, 1440), OWN)],
        [r(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(a.unwrap().buttons_right, 1017);
}

// ---- TaskbarQueryScheduleTests

const A: TaskbarAnchors = TaskbarAnchors::new(2200, 900, 1392, 1440);
const B: TaskbarAnchors = TaskbarAnchors::new(2170, 900, 1392, 1440);

fn s(seconds: f64) -> Span {
    Span::milliseconds((seconds * 1000.0).round() as i64)
}
fn fresh() -> TaskbarQuerySchedule {
    TaskbarQuerySchedule::default()
}
fn settled(answer: Option<TaskbarAnchors>, asked_at: f64) -> TaskbarQuerySchedule {
    fresh().asked(s(asked_at)).answered(answer).asked(s(asked_at)).answered(answer)
}

#[test]
fn a_fresh_schedule_is_due() {
    assert!(fresh().due(s(0.0)));
}
#[test]
fn a_question_that_is_out_waits_for_the_slow_check() {
    let asked = fresh().asked(s(0.0));
    assert!(!asked.due(s(2.0)));
    assert!(asked.due(s(15.0)));
}
#[test]
fn one_answer_alone_asks_once_more() {
    assert!(fresh().asked(s(0.0)).answered(Some(A)).due(s(2.0)));
}
#[test]
fn two_answers_that_agree_settle_it() {
    assert!(!settled(Some(A), 0.0).due(s(2.0)));
}
#[test]
fn one_answer_alone_then_waits_for_the_next() {
    assert!(!fresh().asked(s(0.0)).answered(Some(A)).asked(s(2.0)).due(s(4.0)));
}
#[test]
fn missing_answers_at_the_start_keep_asking() {
    let mut sch = fresh();
    for i in 0..29 {
        sch = sch.asked(s(i as f64 * 2.0)).answered(None);
        assert!(sch.due(s(i as f64 * 2.0 + 2.0)));
    }
}
#[test]
fn after_30_questions_without_anchors_it_settles() {
    let mut sch = fresh();
    for i in 0..30 {
        sch = sch.asked(s(i as f64 * 2.0)).answered(None);
    }
    assert!(!sch.due(s(29.0 * 2.0 + 2.0)));
    assert!(sch.due(s(29.0 * 2.0 + 15.0)));
}
#[test]
fn missing_answers_agree_once_anchors_were_seen() {
    assert!(!settled(Some(A), 0.0).asked(s(16.0)).answered(None).asked(s(18.0)).answered(None).due(s(20.0)));
}
#[test]
fn an_answer_that_differs_asks_again() {
    assert!(settled(Some(A), 0.0).asked(s(16.0)).answered(Some(B)).due(s(18.0)));
}
#[test]
fn an_answer_going_missing_differs() {
    assert!(settled(Some(A), 0.0).asked(s(16.0)).answered(None).due(s(18.0)));
}
#[test]
fn answers_that_never_agree_keep_asking() {
    let mut sch = fresh();
    for i in 0..50 {
        sch = sch.asked(s(i as f64 * 2.0)).answered(if i % 2 == 0 { Some(A) } else { Some(B) });
        assert!(sch.due(s(i as f64 * 2.0 + 2.0)));
    }
}
#[test]
fn a_settled_schedule_checks_after_15_seconds() {
    for (later, due) in [(14.9, false), (15.0, true), (16.0, true), (3600.0, true)] {
        assert_eq!(settled(Some(A), 100.0).due(s(100.0 + later)), due, "{later}");
    }
}
#[test]
fn a_foreground_switch_asks_at_the_next_tick() {
    assert!(settled(Some(A), 0.0).foreground_switched().due(s(2.0)));
}
#[test]
fn many_switches_ask_twice() {
    let sch = settled(Some(A), 0.0).foreground_switched().foreground_switched().foreground_switched().asked(s(2.0)).answered(Some(A));
    assert!(sch.due(s(4.0)));
    assert!(!sch.asked(s(4.0)).answered(Some(A)).due(s(6.0)));
}
#[test]
fn a_switch_while_a_question_is_out_asks_again_after_the_answer() {
    let sch = settled(Some(A), 0.0).asked(s(16.0)).foreground_switched().answered(Some(A));
    assert!(sch.due(s(18.0)));
    let sch = sch.asked(s(18.0)).answered(Some(A));
    assert!(sch.due(s(20.0)));
    assert!(!sch.asked(s(20.0)).answered(Some(A)).due(s(22.0)));
}
#[test]
fn a_clock_that_went_back_is_not_due() {
    assert!(!settled(Some(A), 100.0).due(s(50.0)));
}
#[test]
fn a_lost_answer_is_asked_again_after_15_seconds() {
    let asked = settled(Some(A), 0.0).asked(s(16.0));
    assert!(!asked.due(s(18.0)));
    assert!(asked.due(s(31.0)));
}
#[test]
fn the_slow_check_is_15_seconds() {
    assert_eq!(s(15.0), TaskbarQuerySchedule::slow_check());
}
