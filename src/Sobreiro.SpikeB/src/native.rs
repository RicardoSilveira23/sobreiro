//! SPIKE (shell candidate B): every Win32 call the spike makes, in one place so they can be counted.
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, MonitorFromWindow,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent, WINEVENTPROC};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Shell::SHQueryUserNotificationState;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

pub const WS_EX_OURS: isize = (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize;
pub const WS_EX_TOOL: isize = WS_EX_TOOLWINDOW.0 as isize;

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

/// A bare window of this process, shown without being activated: a stand-in for the taskbar in a test.
pub fn plain_window() -> HWND {
    unsafe {
        let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW, w!("Static"), None, WS_POPUP, 200, 200, 900, 48, None, None, None, None);
        let hwnd = hwnd.unwrap_or_default();
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        hwnd
    }
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

/// The active window of this thread, if it has one.
pub fn active() -> HWND {
    unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow() }
}

/// Asks Windows to make a window the foreground window. Windows may refuse.
pub fn focus(hwnd: HWND) -> bool {
    unsafe { SetForegroundWindow(hwnd).as_bool() }
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

pub fn find_tray_notify(taskbar: HWND) -> HWND {
    unsafe { FindWindowExW(Some(taskbar), None, w!("TrayNotifyWnd"), None).unwrap_or_default() }
}

pub fn is_window(hwnd: HWND) -> bool {
    !hwnd.is_invalid() && unsafe { IsWindow(Some(hwnd)).as_bool() }
}

/// Tells a window what Windows tells it when it comes onto a monitor of another scale: the new DPI and the
/// rectangle Windows would suggest, here the same corner and the size scaled. For a PC with one display.
pub fn pretend_dpi_changed(hwnd: HWND, dpi: u32) {
    let Some(own) = window_rect(hwnd) else { return };
    let old = self::dpi(hwnd).max(1) as i64;
    let scaled = |length: i32| (length as i64 * dpi as i64 / old) as i32;
    let suggested = RECT { left: own.left, top: own.top, right: own.left + scaled(own.width()), bottom: own.top + scaled(own.height()) };
    let wparam = windows::Win32::Foundation::WPARAM(((dpi << 16) | dpi) as usize);
    let lparam = windows::Win32::Foundation::LPARAM(&suggested as *const RECT as isize);
    unsafe {
        SendMessageW(hwnd, WM_DPICHANGED, Some(wparam), Some(lparam));
    }
}

/// The thread and the process a window belongs to.
pub fn thread_and_process(hwnd: HWND) -> (u32, u32) {
    let mut process = 0u32;
    let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut process)) };
    (thread, process)
}

fn monitor_rects(monitor: HMONITOR) -> Option<(PixelRect, PixelRect)> {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    if monitor.is_invalid() || !unsafe { GetMonitorInfoW(monitor, &mut info).as_bool() } {
        return None;
    }
    Some((info.rcMonitor.into(), info.rcWork.into()))
}

/// The bounds of the monitor a window is on.
pub fn monitor_rect(hwnd: HWND) -> Option<PixelRect> {
    monitor_rects(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }).map(|(monitor, _)| monitor)
}

/// The monitor under a point (else the primary one): its bounds, its work area and its DPI.
pub fn screen_at(x: i32, y: i32) -> Option<(PixelRect, PixelRect, f64)> {
    for fallback in [MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTOPRIMARY] {
        let monitor = unsafe { MonitorFromPoint(POINT { x, y }, fallback) };
        let Some((bounds, work)) = monitor_rects(monitor) else { continue };
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }.is_err() {
            dpi_x = 96;
        }
        return Some((bounds, work, dpi_x as f64));
    }
    None
}

/// What Windows says about full-screen apps and presentations, as a number.
pub fn notification_state() -> Option<i32> {
    unsafe { SHQueryUserNotificationState().ok().map(|state| state.0) }
}

pub fn small_icon_size() -> u32 {
    match unsafe { GetSystemMetrics(SM_CXSMICON) } {
        size if size > 0 => size as u32,
        _ => 16,
    }
}

pub const EVENT_FOREGROUND: u32 = EVENT_SYSTEM_FOREGROUND;
pub const EVENT_LOCATION: u32 = EVENT_OBJECT_LOCATIONCHANGE;

/// Hooks one window event, out of context: the callback runs on this thread, through its message loop.
pub fn hook(event: u32, callback: WINEVENTPROC, process: u32, thread: u32, skip_own_process: bool) -> HWINEVENTHOOK {
    let flags = WINEVENT_OUTOFCONTEXT | if skip_own_process { WINEVENT_SKIPOWNPROCESS } else { 0 };
    unsafe { SetWinEventHook(event, event, None, callback, process, thread, flags) }
}

pub fn unhook(hook: &mut HWINEVENTHOOK) {
    if !hook.is_invalid() {
        unsafe {
            let _ = UnhookWinEvent(*hook);
        }
        *hook = HWINEVENTHOOK::default();
    }
}

/// Puts a window above the topmost windows at a place, without resizing or activating it.
pub fn place_topmost(hwnd: HWND, x: i32, y: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
    }
}
