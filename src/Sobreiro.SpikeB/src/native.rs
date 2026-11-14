//! SPIKE (shell candidate B): every Win32 call the spike makes, in one place so they can be counted.
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

pub const WS_EX_OURS: isize = (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PixelRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

impl From<RECT> for PixelRect {
    fn from(r: RECT) -> Self {
        Self { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
    }
}

pub fn find_taskbar() -> HWND {
    unsafe { FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or_default() }
}

pub fn window_rect(hwnd: HWND) -> Option<PixelRect> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut r).ok().map(|_| r.into()) }
}

pub fn style(hwnd: HWND) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) }
}

pub fn ex_style(hwnd: HWND) -> isize {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }
}

pub fn add_ex_style(hwnd: HWND, bits: isize) {
    unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style(hwnd) | bits) };
}

/// Makes `owner` the owner of a top-level window that already exists.
pub fn set_owner(hwnd: HWND, owner: HWND) {
    unsafe { SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, owner.0 as isize) };
}

pub fn owner(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_OWNER).unwrap_or_default() }
}

pub fn previous(hwnd: HWND) -> HWND {
    unsafe { GetWindow(hwnd, GW_HWNDPREV).unwrap_or_default() }
}

/// Whether `hwnd` lies above `other` in the z-order.
pub fn is_above(hwnd: HWND, other: HWND) -> bool {
    let mut w = previous(other);
    let mut steps = 0;
    while !w.is_invalid() && steps < 10_000 {
        if w == hwnd {
            return true;
        }
        w = previous(w);
        steps += 1;
    }
    false
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn is_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

pub fn dpi(hwnd: HWND) -> u32 {
    unsafe { GetDpiForWindow(hwnd) }
}

pub fn cursor() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

/// Moves a window without resizing or activating it. `raise` also puts it above the topmost windows.
pub fn place(hwnd: HWND, x: i32, y: i32, raise: bool) -> bool {
    let (after, flags) = if raise {
        (Some(HWND_TOPMOST), SWP_NOOWNERZORDER | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW)
    } else {
        (None, SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW)
    };
    unsafe { SetWindowPos(hwnd, after, x, y, 0, 0, flags).is_ok() }
}

pub fn hide(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}
