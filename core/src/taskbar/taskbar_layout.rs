//! Turns the rectangles UI Automation gives into the anchors the pill is placed by. Sums and
//! differences wrap around at the ends of an `i32`, as the C# of 1.x does.
use std::cmp::{max, min};

use crate::display::flyout_placement::PixelRect;

/// Offsets in physical pixels from the taskbar window's left (`tray_left`, `buttons_right`) and
/// top (`band_top`, `band_bottom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarAnchors {
    pub tray_left: i32,
    pub buttons_right: i32,
    pub band_top: i32,
    pub band_bottom: i32,
}

pub const TRAY_CLASS_PREFIX: &str = "SystemTray.";

fn is_tray(class_name: &str) -> bool {
    class_name.starts_with(TRAY_CLASS_PREFIX)
}

fn has_area(rect: PixelRect) -> bool {
    rect.width() > 0 && rect.height() > 0
}

/// Turns UI Automation rectangles into anchors. `items` are the children of the taskbar's
/// InputSite pane (class name, rectangle, process id), `buttons` the children of
/// Taskbar.TaskbarFrameAutomationPeer. Items of `own_process_id` are ignored. None when there are
/// no tray items.
pub fn anchors<'a>(
    taskbar: PixelRect,
    items: impl IntoIterator<Item = (&'a str, PixelRect, u32)>,
    buttons: impl IntoIterator<Item = PixelRect>,
    own_process_id: u32,
) -> Option<TaskbarAnchors> {
    // Our own windows (the pill, which the taskbar owns, so UI Automation lists it) are never
    // tray items or obstacles.
    let all: Vec<(&str, PixelRect)> = items
        .into_iter()
        .filter(|(_, _, process_id)| *process_id != own_process_id)
        .map(|(class_name, rect, _)| (class_name, rect))
        .collect();
    let tray: Vec<PixelRect> = all
        .iter()
        .filter(|(class_name, rect)| is_tray(class_name) && has_area(*rect))
        .map(|(_, rect)| *rect)
        .collect();
    let tray_left = tray.iter().map(|r| r.left).reduce(min)?;

    let mut buttons_right = 0;
    for b in buttons {
        if has_area(b) {
            buttons_right = max(buttons_right, b.right.wrapping_sub(taskbar.left));
        }
    }

    // Other items (a Widgets button, an input indicator) left of the tray are obstacles too;
    // panes that reach past the tray's left edge (the whole-taskbar InputSite, the tray's own
    // pane) are not.
    for (class_name, rect) in &all {
        if !is_tray(class_name) && has_area(*rect) && rect.right <= tray_left {
            buttons_right = max(buttons_right, rect.right.wrapping_sub(taskbar.left));
        }
    }

    let band_top = tray.iter().map(|r| r.top).fold(i32::MAX, min);
    let band_bottom = tray.iter().map(|r| r.bottom).fold(i32::MIN, max);
    Some(TaskbarAnchors {
        tray_left: tray_left.wrapping_sub(taskbar.left),
        buttons_right,
        band_top: band_top.wrapping_sub(taskbar.top),
        band_bottom: band_bottom.wrapping_sub(taskbar.top),
    })
}
