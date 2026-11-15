//! Ported from `tests/Sobreiro.Tests/TaskbarLayoutTests.cs`. The last two tests are new.
use sobreiro_core::display::flyout_placement::PixelRect;
use sobreiro_core::taskbar::taskbar_layout::{TaskbarAnchors, anchors};

const BAR: PixelRect = rect(0, 1392, 2560, 1440);
const EXPLORER: u32 = 4556;
const OWN: u32 = 444;

const fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
    PixelRect {
        left,
        top,
        right,
        bottom,
    }
}

const fn expected(
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

#[test]
fn anchors_from_the_spike() {
    let items = [
        (
            "SystemTray.NormalButton",
            rect(2314, 1392, 2346, 1440),
            EXPLORER,
        ),
        (
            "SystemTray.OmniButton",
            rect(2474, 1392, 2544, 1440),
            EXPLORER,
        ),
        (
            "Taskbar.TaskbarFrameAutomationPeer",
            rect(0, 1392, 2560, 1440),
            EXPLORER,
        ),
    ];
    let buttons = [rect(972, 1392, 1017, 1440), rect(1545, 1392, 1589, 1440)];
    assert_eq!(
        anchors(BAR, items, buttons, OWN),
        Some(expected(2314, 1589, 0, 48))
    );
}

#[test]
fn anchors_without_tray_items_is_none() {
    assert_eq!(
        anchors(
            BAR,
            [("Taskbar.TaskbarFrameAutomationPeer", BAR, EXPLORER)],
            [],
            OWN
        ),
        None
    );
}

#[test]
fn empty_tray_items_are_skipped() {
    assert_eq!(
        anchors(
            BAR,
            [(
                "SystemTray.NormalButton",
                rect(2314, 1392, 2314, 1440),
                EXPLORER
            )],
            [],
            OWN
        ),
        None
    );
}

#[test]
fn no_buttons_means_the_taskbars_left_edge() {
    let found = anchors(
        BAR,
        [(
            "SystemTray.OmniButton",
            rect(2474, 1392, 2544, 1440),
            EXPLORER,
        )],
        [],
        OWN,
    );
    assert_eq!(found.unwrap().buttons_right, 0);
}

#[test]
fn anchors_are_offsets_from_the_taskbar() {
    assert_eq!(
        anchors(
            rect(2000, 1392, 4560, 1440),
            [(
                "SystemTray.NormalButton",
                rect(2314, 1392, 2346, 1440),
                EXPLORER
            )],
            [rect(2050, 1392, 2089, 1440)],
            OWN
        ),
        Some(expected(314, 89, 0, 48))
    );
}

#[test]
fn an_unknown_item_left_of_the_tray_is_an_obstacle() {
    let found = anchors(
        BAR,
        [
            (
                "SystemTray.NormalButton",
                rect(2314, 1392, 2346, 1440),
                EXPLORER,
            ),
            ("ToggleButton", rect(2200, 1392, 2250, 1440), EXPLORER),
        ],
        [rect(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(found.unwrap().buttons_right, 2250);
}

#[test]
fn a_pane_spanning_the_whole_taskbar_is_not_an_obstacle() {
    let found = anchors(
        BAR,
        [
            (
                "SystemTray.NormalButton",
                rect(2314, 1392, 2346, 1440),
                EXPLORER,
            ),
            ("Windows.UI.Input.InputSite.WindowClass", BAR, EXPLORER),
        ],
        [rect(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(found.unwrap().buttons_right, 1017);
}

#[test]
fn own_windows_are_not_obstacles() {
    let found = anchors(
        BAR,
        [
            (
                "SystemTray.NormalButton",
                rect(2314, 1392, 2346, 1440),
                EXPLORER,
            ),
            ("Window", rect(2200, 1392, 2300, 1440), OWN),
        ],
        [rect(972, 1392, 1017, 1440)],
        OWN,
    );
    assert_eq!(found.unwrap().buttons_right, 1017);
}

#[test]
fn an_own_tray_item_is_not_a_tray_item() {
    assert_eq!(
        anchors(
            BAR,
            [("SystemTray.NormalButton", rect(2314, 1392, 2346, 1440), OWN)],
            [],
            OWN
        ),
        None
    );
}

#[test]
fn offsets_wrap_around_at_the_ends_of_an_i32() {
    // The taskbar's left is as far left as an i32 goes: 2314 - i32::MIN wraps to i32::MIN + 2314.
    let bar = rect(i32::MIN, 1392, 2560, 1440);
    let found = anchors(
        bar,
        [(
            "SystemTray.NormalButton",
            rect(2314, 1392, 2346, 1440),
            EXPLORER,
        )],
        [rect(0, 1392, 100, 1440)],
        OWN,
    );
    let found = found.unwrap();
    assert_eq!(found.tray_left, i32::MIN.wrapping_add(2314));
    // 100 - i32::MIN wraps to a negative offset, which the 0 of "no buttons" outranks.
    assert_eq!(found.buttons_right, 0);
    assert_eq!((found.band_top, found.band_bottom), (0, 48));
}
