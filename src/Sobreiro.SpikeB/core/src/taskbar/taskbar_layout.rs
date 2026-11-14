use crate::display::PixelRect;

/// Offsets in physical pixels from the taskbar window's left (tray_left, buttons_right) and top (band_top, band_bottom).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskbarAnchors {
    pub tray_left: i32,
    pub buttons_right: i32,
    pub band_top: i32,
    pub band_bottom: i32,
}

impl TaskbarAnchors {
    pub const fn new(tray_left: i32, buttons_right: i32, band_top: i32, band_bottom: i32) -> Self {
        Self { tray_left, buttons_right, band_top, band_bottom }
    }
}

pub struct TaskbarLayout;

impl TaskbarLayout {
    pub const TRAY_CLASS_PREFIX: &'static str = "SystemTray.";

    /// Turns UI Automation rectangles into anchors. `items` are (class name, rectangle, process id) of the children
    /// of the taskbar's InputSite pane, `buttons` the children of Taskbar.TaskbarFrameAutomationPeer. Items of
    /// `own_process_id` are ignored. None when there are no tray items.
    pub fn anchors<S: AsRef<str>>(
        taskbar: PixelRect,
        items: impl IntoIterator<Item = (S, PixelRect, i32)>,
        buttons: impl IntoIterator<Item = PixelRect>,
        own_process_id: i32,
    ) -> Option<TaskbarAnchors> {
        // Our own windows (the pill, which the taskbar owns, so UI Automation lists it) are never tray items or obstacles.
        let all: Vec<(S, PixelRect, i32)> = items.into_iter().filter(|i| i.2 != own_process_id).collect();
        let tray: Vec<PixelRect> = all
            .iter()
            .filter(|i| i.0.as_ref().starts_with(Self::TRAY_CLASS_PREFIX) && i.1.width() > 0 && i.1.height() > 0)
            .map(|i| i.1)
            .collect();
        if tray.is_empty() {
            return None;
        }

        let mut buttons_right = 0;
        for b in buttons {
            if b.width() > 0 && b.height() > 0 {
                buttons_right = buttons_right.max(b.right - taskbar.left);
            }
        }

        // Other items (a Widgets button, an input indicator) left of the tray are obstacles too; panes that reach
        // past the tray's left edge (the whole-taskbar InputSite, the tray's own pane) are not.
        let tray_left = tray.iter().map(|r| r.left).min().unwrap();
        for (class_name, rect, _) in &all {
            if !class_name.as_ref().starts_with(Self::TRAY_CLASS_PREFIX) && rect.width() > 0 && rect.height() > 0 && rect.right <= tray_left {
                buttons_right = buttons_right.max(rect.right - taskbar.left);
            }
        }

        Some(TaskbarAnchors {
            tray_left: tray_left - taskbar.left,
            buttons_right,
            band_top: tray.iter().map(|r| r.top).min().unwrap() - taskbar.top,
            band_bottom: tray.iter().map(|r| r.bottom).max().unwrap() - taskbar.top,
        })
    }
}
