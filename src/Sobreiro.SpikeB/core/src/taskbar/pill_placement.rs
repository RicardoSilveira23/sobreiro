use super::TaskbarAnchors;
use crate::display::PixelRect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillHidden {
    NoAnchors,
    NotAtBottom,
    NoRoom,
    OffScreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PillPlace {
    pub x: i32,
    pub y: i32,
    pub hidden: Option<PillHidden>,
    pub band_top: i32,
}

impl PillPlace {
    pub fn is_shown(&self) -> bool {
        self.hidden.is_none()
    }
}

pub struct PillPlacement;

impl PillPlacement {
    /// Where the pill goes, just left of the tray icons, or why it is hidden.
    pub fn place(monitor: PixelRect, taskbar: PixelRect, anchors: Option<TaskbarAnchors>, width: i32, height: i32, gap: i32) -> PillPlace {
        let Some(a) = anchors else { return hide(PillHidden::NoAnchors) };
        if taskbar.width() <= taskbar.height() || taskbar.top < monitor.top + monitor.height() / 2 {
            return hide(PillHidden::NotAtBottom);
        }

        let x = taskbar.left + a.tray_left - gap - width;
        if x < taskbar.left + a.buttons_right + gap {
            return hide(PillHidden::NoRoom);
        }

        if height > a.band_bottom - a.band_top {
            return hide(PillHidden::NoRoom);
        }

        let y = taskbar.top + a.band_top + (a.band_bottom - a.band_top - height) / 2;
        if x < monitor.left || y < monitor.top || x + width > monitor.right || y + height > monitor.bottom {
            return hide(PillHidden::OffScreen);
        }

        PillPlace { x, y, hidden: None, band_top: taskbar.top + a.band_top }
    }
}

fn hide(why: PillHidden) -> PillPlace {
    PillPlace { x: 0, y: 0, hidden: Some(why), band_top: 0 }
}
