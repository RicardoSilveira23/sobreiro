//! The Win32 calls the flyout is placed with: the pointer, and the monitor under a point with its
//! work area and DPI. All in physical pixels.
#![allow(unsafe_code)]

use sobreiro_core::display::flyout_placement::PixelRect;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO,
    MonitorFromPoint,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// The DPI of a monitor when Windows does not say: 100%.
const DEFAULT_DPI: u32 = 96;

/// A monitor: its whole rectangle, its work area (without the taskbar) and its DPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    pub monitor: PixelRect,
    pub work_area: PixelRect,
    pub dpi: u32,
}

/// The pointer, in physical pixels.
pub fn cursor() -> Option<(i32, i32)> {
    let mut at = POINT::default();
    // SAFETY: `at` is ours to write.
    unsafe { GetCursorPos(&mut at) }.ok()?;
    Some((at.x, at.y))
}

/// The monitor nearest the point, else the primary.
pub fn at_point(x: i32, y: i32) -> Option<Screen> {
    // SAFETY: asks Windows about a point; any point is allowed.
    let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) };
    screen_of(monitor).or_else(primary)
}

/// The primary monitor.
pub fn primary() -> Option<Screen> {
    // SAFETY: asks Windows about a point; any point is allowed.
    let monitor = unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    screen_of(monitor)
}

fn rect(r: RECT) -> PixelRect {
    PixelRect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

fn screen_of(monitor: windows::Win32::Graphics::Gdi::HMONITOR) -> Option<Screen> {
    if monitor.is_invalid() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` is ours to write, with its size filled in as the call asks.
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: `x` and `y` are ours to write.
    let dpi = match unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) } {
        Ok(()) if x > 0 => x,
        _ => DEFAULT_DPI,
    };
    Some(Screen {
        monitor: rect(info.rcMonitor),
        work_area: rect(info.rcWork),
        dpi,
    })
}
