//! Ported from `tests/Sobreiro.Tests/PillPlacementTests.cs`. The last two tests are new.
use sobreiro_core::display::flyout_placement::PixelRect;
use sobreiro_core::taskbar::pill_placement::{PillHidden, PillPlace, place};
use sobreiro_core::taskbar::taskbar_layout::TaskbarAnchors;

const MONITOR: PixelRect = rect(0, 0, 2560, 1440);
const AT_100: TaskbarAnchors = anchors(2314, 1589, 0, 48);

const fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
    PixelRect {
        left,
        top,
        right,
        bottom,
    }
}

const fn anchors(
    tray_left: i32,
    buttons_right: i32,
    band_top: i32,
    band_bottom: i32,
) -> TaskbarAnchors {
    TaskbarAnchors {
        tray_left,
        buttons_right,
        band_top,
        band_bottom,
    }
}

fn shown(x: i32, y: i32, band_top: i32) -> PillPlace {
    PillPlace {
        x,
        y,
        hidden: None,
        band_top,
    }
}

#[test]
fn places_left_of_the_tray_at_100_percent() {
    assert_eq!(
        place(MONITOR, rect(0, 1392, 2560, 1440), Some(AT_100), 88, 38, 4),
        shown(2222, 1397, 1392)
    );
}

#[test]
fn places_left_of_the_tray_at_125_percent() {
    assert_eq!(
        place(
            MONITOR,
            rect(0, 1380, 2560, 1440),
            Some(anchors(2253, 1693, 0, 60)),
            110,
            47,
            5
        ),
        shown(2138, 1386, 1380)
    );
}

#[test]
fn places_on_a_monitor_not_at_the_origin() {
    assert_eq!(
        place(
            rect(-2560, 0, 0, 1440),
            rect(-2560, 1392, 0, 1440),
            Some(AT_100),
            88,
            38,
            4
        ),
        shown(-338, 1397, 1392)
    );
}

#[test]
fn no_anchors_hides() {
    assert_eq!(
        place(MONITOR, rect(0, 1392, 2560, 1440), None, 88, 38, 4).hidden,
        Some(PillHidden::NoAnchors)
    );
}

#[test]
fn a_taskbar_partly_off_the_monitor_hides() {
    // sliding away, then auto-hidden
    for top in [1404, 1438] {
        assert_eq!(
            place(
                MONITOR,
                rect(0, top, 2560, top + 48),
                Some(AT_100),
                88,
                38,
                4
            )
            .hidden,
            Some(PillHidden::OffScreen),
            "top {top}"
        );
    }
}

#[test]
fn hides_when_the_pill_is_taller_than_the_band() {
    assert_eq!(
        place(MONITOR, rect(0, 1392, 2560, 1440), Some(AT_100), 88, 50, 4).hidden,
        Some(PillHidden::NoRoom)
    );
}

#[test]
fn hides_when_it_would_cover_the_app_buttons() {
    let anchors = TaskbarAnchors {
        buttons_right: 2230,
        ..AT_100
    };
    assert_eq!(
        place(MONITOR, rect(0, 1392, 2560, 1440), Some(anchors), 88, 38, 4).hidden,
        Some(PillHidden::NoRoom)
    );
}

#[test]
fn just_enough_room_shows() {
    // x 2222 == buttons 2218 + gap 4
    let anchors = TaskbarAnchors {
        buttons_right: 2218,
        ..AT_100
    };
    assert!(place(MONITOR, rect(0, 1392, 2560, 1440), Some(anchors), 88, 38, 4).is_shown());
}

#[test]
fn a_taskbar_at_the_top_hides() {
    let anchors = TaskbarAnchors {
        band_top: 0,
        band_bottom: 48,
        ..AT_100
    };
    assert_eq!(
        place(MONITOR, rect(0, 0, 2560, 48), Some(anchors), 88, 38, 4).hidden,
        Some(PillHidden::NotAtBottom)
    );
}

#[test]
fn a_taskbar_on_the_side_hides() {
    assert_eq!(
        place(MONITOR, rect(2512, 0, 2560, 1440), Some(AT_100), 88, 38, 4).hidden,
        Some(PillHidden::NotAtBottom)
    );
}

#[test]
fn a_hidden_pill_has_no_place() {
    let hidden = place(MONITOR, rect(0, 1392, 2560, 1440), Some(AT_100), 88, 50, 4);
    assert_eq!((hidden.x, hidden.y, hidden.band_top), (0, 0, 0));
    assert!(!hidden.is_shown());
}

#[test]
fn a_width_whose_sum_wraps_around_an_i32_is_placed_as_the_csharp_would() {
    // 2314 - 4 - i32::MIN wraps to a large negative x: the pill is left of the app buttons.
    assert_eq!(
        place(
            MONITOR,
            rect(0, 1392, 2560, 1440),
            Some(AT_100),
            i32::MIN,
            38,
            4
        )
        .hidden,
        Some(PillHidden::NoRoom)
    );
}
