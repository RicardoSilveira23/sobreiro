//! Where the pill goes, just left of the tray icons. Sums and differences wrap around at the
//! ends of an `i32`, as the C# of 1.x does.
use crate::display::flyout_placement::PixelRect;
use crate::taskbar::taskbar_layout::TaskbarAnchors;

/// Why the pill is hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillHidden {
    NoAnchors,
    NotAtBottom,
    NoRoom,
    OffScreen,
}

/// Where the pill goes, or why it is hidden. A hidden pill has `x`, `y` and `band_top` of 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

fn hide(why: PillHidden) -> PillPlace {
    PillPlace {
        x: 0,
        y: 0,
        hidden: Some(why),
        band_top: 0,
    }
}

/// Where the pill goes, just left of the tray icons, or why it is hidden.
pub fn place(
    monitor: PixelRect,
    taskbar: PixelRect,
    anchors: Option<TaskbarAnchors>,
    width: i32,
    height: i32,
    gap: i32,
) -> PillPlace {
    let Some(a) = anchors else {
        return hide(PillHidden::NoAnchors);
    };
    if taskbar.width() <= taskbar.height()
        || taskbar.top < monitor.top.wrapping_add(monitor.height() / 2)
    {
        return hide(PillHidden::NotAtBottom);
    }

    let x = taskbar
        .left
        .wrapping_add(a.tray_left)
        .wrapping_sub(gap)
        .wrapping_sub(width);
    if x < taskbar.left.wrapping_add(a.buttons_right).wrapping_add(gap) {
        return hide(PillHidden::NoRoom);
    }

    let band = a.band_bottom.wrapping_sub(a.band_top);
    if height > band {
        return hide(PillHidden::NoRoom);
    }

    let y = taskbar
        .top
        .wrapping_add(a.band_top)
        .wrapping_add(band.wrapping_sub(height) / 2);
    if x < monitor.left
        || y < monitor.top
        || x.wrapping_add(width) > monitor.right
        || y.wrapping_add(height) > monitor.bottom
    {
        return hide(PillHidden::OffScreen);
    }

    PillPlace {
        x,
        y,
        hidden: None,
        band_top: taskbar.top.wrapping_add(a.band_top),
    }
}
