//! The Win32 calls the pill's tracker needs: the taskbar and its tray, their rectangles, monitor
//! and scale, the notification state, WinEvent hooks, the broadcasts a hidden window hears, and
//! showing, moving and hiding the pill's window. Window handles are plain numbers, 0 for none.
//!
//! Windows calls the hooks and the hidden window back with no data of ours, on the UI thread, so
//! the handler each is given is kept in a value of that thread.
#![allow(unsafe_code)]

use super::notify_icon::{self, TrayEvent};
use sobreiro_core::display::flyout_placement::PixelRect;
use std::cell::Cell;
use std::ffi::c_void;
use std::marker::PhantomData;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, GetDpiForWindow,
    GetSystemMetricsForDpi, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::Shell::SHQueryUserNotificationState;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, ENDSESSION_LOGOFF, EVENT_OBJECT_LOCATIONCHANGE,
    EVENT_SYSTEM_FOREGROUND, FindWindowExW, FindWindowW, GW_HWNDPREV, GetWindow, GetWindowRect,
    GetWindowThreadProcessId, HWND_TOPMOST, IsWindowVisible, RegisterClassW,
    RegisterWindowMessageW, SM_CXSMICON, SW_HIDE, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, WINDOW_STYLE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_DISPLAYCHANGE, WM_QUERYENDSESSION, WM_SETTINGCHANGE, WM_TIMECHANGE,
    WNDCLASSW, WS_EX_TOOLWINDOW,
};
use windows::core::{PCWSTR, w};

/// A WinEvent the tracker listens to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinEvent {
    /// A window of the hooked thread moved or changed size. `object` is 0 for the window itself.
    Location { window: isize, object: i32 },
    /// Another process's window came to the foreground.
    Foreground,
}

/// A broadcast the hidden window hears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Broadcast {
    /// Explorer made a new taskbar (it restarted).
    TaskbarCreated,
    /// A display was added or removed, or its resolution changed.
    DisplayChanged,
    /// A system setting changed: the theme and the accent are among them.
    SettingChanged,
    /// The system clock was set (the time or the time zone changed).
    TimeChanged,
}

pub type WinEventHandler = fn(WinEvent);
pub type BroadcastHandler = fn(Broadcast);
pub type TrayHandler = fn(TrayEvent);
/// Told that Windows asks to end the session: true for a sign-out, false for a shutdown, a
/// restart or an installer.
pub type SessionEndHandler = fn(bool);

const MESSAGE_CLASS: PCWSTR = w!("SobreiroPillMessages");

thread_local! {
    static WIN_EVENT_HANDLER: Cell<Option<WinEventHandler>> = const { Cell::new(None) };
    static BROADCAST_HANDLER: Cell<Option<BroadcastHandler>> = const { Cell::new(None) };
    static TRAY_HANDLER: Cell<Option<TrayHandler>> = const { Cell::new(None) };
    static SESSION_END_HANDLER: Cell<Option<SessionEndHandler>> = const { Cell::new(None) };
    /// The number of the "TaskbarCreated" message, 0 until it is registered.
    static TASKBAR_CREATED: Cell<u32> = const { Cell::new(0) };
}

/// Sets what is told, on this thread, when Windows asks to end the session.
pub fn on_session_ending(handler: SessionEndHandler) {
    let _ = SESSION_END_HANDLER.try_with(|cell| cell.set(Some(handler)));
}

fn hwnd(handle: isize) -> HWND {
    HWND(handle as *mut c_void)
}

/// Makes this process aware of each monitor's scale, so the rectangles below are in physical
/// pixels. The toolkit asks the same when it starts; whichever comes first holds.
pub fn become_dpi_aware() {
    // SAFETY: sets a process-wide setting from a constant; a refusal (already set) is ignored.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// The taskbar of the main monitor, or 0.
pub fn find_taskbar() -> isize {
    // SAFETY: looks a window up by a constant class name.
    unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) }.map_or(0, |h| h.0 as isize)
}

/// The tray's window inside `taskbar`, or 0.
pub fn find_tray_notify(taskbar: isize) -> isize {
    if taskbar == 0 {
        return 0;
    }
    // SAFETY: looks a child window up by a constant class name; any parent handle is allowed.
    unsafe {
        FindWindowExW(
            Some(hwnd(taskbar)),
            None,
            w!("TrayNotifyWnd"),
            PCWSTR::null(),
        )
    }
    .map_or(0, |h| h.0 as isize)
}

/// A window's rectangle on the screen, in physical pixels.
pub fn window_rect(window: isize) -> Option<PixelRect> {
    if window == 0 {
        return None;
    }
    let mut rect = RECT::default();
    // SAFETY: `rect` is ours to write; any handle is allowed and a bad one fails.
    unsafe { GetWindowRect(hwnd(window), &mut rect) }.ok()?;
    Some(PixelRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

/// The rectangle of the monitor nearest a window, in physical pixels.
pub fn monitor_rect(window: isize) -> Option<PixelRect> {
    if window == 0 {
        return None;
    }
    // SAFETY: asks Windows about a handle; any number is allowed.
    let monitor = unsafe { MonitorFromWindow(hwnd(window), MONITOR_DEFAULTTONEAREST) };
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
    let r = info.rcMonitor;
    Some(PixelRect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    })
}

/// A window's DPI (96 at 100%), or 0 for a handle that is no window.
pub fn dpi_of(window: isize) -> u32 {
    if window == 0 {
        return 0;
    }
    // SAFETY: asks Windows about a handle; any number is allowed.
    unsafe { GetDpiForWindow(hwnd(window)) }
}

/// The small-icon size, in pixels, at the primary taskbar's scale; the system's scale when there
/// is no taskbar.
pub fn small_icon_size() -> u32 {
    let mut dpi = dpi_of(find_taskbar());
    if dpi == 0 {
        // SAFETY: takes no arguments.
        dpi = unsafe { GetDpiForSystem() };
    }
    // SAFETY: reads a system metric for a scale; no pointers.
    let size = unsafe { GetSystemMetricsForDpi(SM_CXSMICON, dpi) };
    u32::try_from(size).unwrap_or(0)
}

/// The shell's notification state (QUNS_*), or None when the question fails.
pub fn notification_state() -> Option<i32> {
    // SAFETY: takes no arguments.
    unsafe { SHQueryUserNotificationState() }
        .ok()
        .map(|state| state.0)
}

/// The process and thread that made a window; 0 and 0 for a handle that is no window.
pub fn process_and_thread(window: isize) -> (u32, u32) {
    let mut process = 0u32;
    // SAFETY: `process` is ours to write; any handle is allowed and a bad one gives 0.
    let thread = unsafe { GetWindowThreadProcessId(hwnd(window), Some(&mut process)) };
    (process, thread)
}

/// A WinEvent hook, out of context, called back on the thread that set it. Unhooked when dropped.
/// Not Send: it is unhooked on the thread that set it.
pub struct WinEventHook(isize, PhantomData<*const ()>);

impl WinEventHook {
    /// Location changes of the windows of one thread of one process.
    pub fn location(process: u32, thread: u32, handler: WinEventHandler) -> Option<WinEventHook> {
        if process == 0 || thread == 0 {
            return None;
        }
        Self::set(
            EVENT_OBJECT_LOCATIONCHANGE,
            process,
            thread,
            WINEVENT_OUTOFCONTEXT,
            handler,
        )
    }

    /// Foreground changes, skipping this process's own windows.
    pub fn foreground(handler: WinEventHandler) -> Option<WinEventHook> {
        Self::set(
            EVENT_SYSTEM_FOREGROUND,
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            handler,
        )
    }

    fn set(
        event: u32,
        process: u32,
        thread: u32,
        flags: u32,
        handler: WinEventHandler,
    ) -> Option<WinEventHook> {
        WIN_EVENT_HANDLER
            .try_with(|cell| cell.set(Some(handler)))
            .ok()?;
        // SAFETY: the callback is a function of this module that lives as long as the process;
        // out of context, so no module handle is needed and it is called on this thread.
        let hook =
            unsafe { SetWinEventHook(event, event, None, Some(win_event), process, thread, flags) };
        (!hook.is_invalid()).then_some(WinEventHook(hook.0 as isize, PhantomData))
    }
}

impl Drop for WinEventHook {
    fn drop(&mut self) {
        // SAFETY: the handle is the one SetWinEventHook gave, unhooked once, here.
        let _ = unsafe { UnhookWinEvent(HWINEVENTHOOK(self.0 as *mut c_void)) };
    }
}

unsafe extern "system" fn win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let Some(handler) = WIN_EVENT_HANDLER.try_with(Cell::get).ok().flatten() else {
        return;
    };
    let event = match event {
        EVENT_OBJECT_LOCATIONCHANGE => WinEvent::Location {
            window: window.0 as isize,
            object,
        },
        EVENT_SYSTEM_FOREGROUND => WinEvent::Foreground,
        _ => return,
    };
    handler(event);
}

/// A hidden top-level window that hears "TaskbarCreated", display changes, setting changes, time
/// changes and the tray icons' messages. Top-level, not message-only: broadcasts skip
/// message-only windows. Destroyed when dropped.
/// Not Send: only the thread that made a window may destroy it.
pub struct MessageWindow(isize, PhantomData<*const ()>);

impl MessageWindow {
    /// Makes the window on this thread. A window made by this thread while `born` has a window
    /// pending would be taken by it, so call this before the first `born::show`, or while none is
    /// pending.
    pub fn create(handler: BroadcastHandler, tray: TrayHandler) -> Option<MessageWindow> {
        // SAFETY: registers a constant name; the same number comes back each time.
        let created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
        TASKBAR_CREATED.try_with(|cell| cell.set(created)).ok()?;
        BROADCAST_HANDLER
            .try_with(|cell| cell.set(Some(handler)))
            .ok()?;
        TRAY_HANDLER.try_with(|cell| cell.set(Some(tray))).ok()?;
        // SAFETY: the module handle of this exe; no name is passed.
        let instance = HINSTANCE(unsafe { GetModuleHandleW(PCWSTR::null()) }.ok()?.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(message_procedure),
            hInstance: instance,
            lpszClassName: MESSAGE_CLASS,
            ..Default::default()
        };
        // SAFETY: `class` names a procedure of this module and a constant class name. A class
        // already registered (a second call) fails here and is used as it is below.
        let _ = unsafe { RegisterClassW(&class) };
        // SAFETY: a hidden window of the class above, with no parent, menu or creation data.
        let window = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                MESSAGE_CLASS,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .ok()?;
        Some(MessageWindow(window.0 as isize, PhantomData))
    }

    /// The window's handle, for what needs a hidden window of ours (the pill's menu).
    pub fn handle(&self) -> isize {
        self.0
    }
}

impl Drop for MessageWindow {
    fn drop(&mut self) {
        // SAFETY: the window was made by this thread, and is destroyed once, here.
        let _ = unsafe { DestroyWindow(hwnd(self.0)) };
    }
}

unsafe extern "system" fn message_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == notify_icon::TRAY_MESSAGE {
        if let Some(handler) = TRAY_HANDLER.try_with(Cell::get).ok().flatten() {
            handler(notify_icon::decode(wparam.0, lparam.0));
        }
        return LRESULT(0);
    }
    if message == WM_QUERYENDSESSION {
        // The handler quits the app; the end itself is let go on: TRUE.
        if let Some(handler) = SESSION_END_HANDLER.try_with(Cell::get).ok().flatten() {
            handler(lparam.0 as u32 & ENDSESSION_LOGOFF != 0);
        }
        return LRESULT(1);
    }
    let created = TASKBAR_CREATED.try_with(Cell::get).unwrap_or(0);
    let broadcast = if message == WM_DISPLAYCHANGE {
        Some(Broadcast::DisplayChanged)
    } else if message == WM_SETTINGCHANGE {
        Some(Broadcast::SettingChanged)
    } else if message == WM_TIMECHANGE {
        Some(Broadcast::TimeChanged)
    } else if created != 0 && message == created {
        Some(Broadcast::TaskbarCreated)
    } else {
        None
    };
    if let Some(broadcast) = broadcast
        && let Some(handler) = BROADCAST_HANDLER.try_with(Cell::get).ok().flatten()
    {
        handler(broadcast);
    }
    // SAFETY: the default procedure takes any message.
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

/// Whether a window is shown.
pub fn is_visible(window: isize) -> bool {
    // SAFETY: asks Windows about a handle; any number is allowed.
    unsafe { IsWindowVisible(hwnd(window)) }.as_bool()
}

/// The window just above `window` in the z-order, or 0 at the top.
pub fn window_above(window: isize) -> isize {
    // SAFETY: asks Windows about a handle; any number is allowed.
    unsafe { GetWindow(hwnd(window), GW_HWNDPREV) }.map_or(0, |h| h.0 as isize)
}

/// Moves a window to (x, y), raises it to the top of the topmost band and shows it, without
/// activating it or moving its owner in the z-order.
pub fn raise_to(window: isize, x: i32, y: i32) {
    // SAFETY: changes a window of this process; any handle is allowed and a bad one fails.
    let _ = unsafe {
        SetWindowPos(
            hwnd(window),
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW,
        )
    };
}

/// Moves a window to (x, y) and shows it, without activating it or changing the z-order.
pub fn move_to(window: isize, x: i32, y: i32) {
    // SAFETY: changes a window of this process; any handle is allowed and a bad one fails.
    let _ = unsafe {
        SetWindowPos(
            hwnd(window),
            None,
            x,
            y,
            0,
            0,
            SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    };
}

/// Moves a window to (x, y) without showing it, activating it or changing the z-order. A window
/// that moves to another monitor is told that monitor's scale, shown or not.
pub fn move_hidden(window: isize, x: i32, y: i32) {
    // SAFETY: changes a window of this process; any handle is allowed and a bad one fails.
    let _ = unsafe {
        SetWindowPos(
            hwnd(window),
            None,
            x,
            y,
            0,
            0,
            SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
}

/// Hides a window.
pub fn hide(window: isize) {
    // SAFETY: changes a window of this process; any handle is allowed and a bad one fails.
    let _ = unsafe { ShowWindow(hwnd(window), SW_HIDE) };
}
