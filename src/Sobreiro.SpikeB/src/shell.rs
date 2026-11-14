//! SPIKE (shell candidate B): choosing the toolkit's backend and renderer, and reaching its window handle.
use crate::spike;
use std::cell::Cell;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};

thread_local! {
    /// Set right before a component is made: its window is then born inactive and owned by this window.
    /// The toolkit's hook sees a window's attributes before its title, so it cannot tell windows apart itself.
    static NEXT_OWNER: Cell<Option<isize>> = const { Cell::new(None) };
}

/// The next window the toolkit makes on this thread is born owned by `owner` and is not activated when shown.
pub fn next_window_owned_by(owner: HWND) {
    NEXT_OWNER.set(Some(owner.0 as isize));
}

/// The next window the toolkit makes on this thread is not activated when shown, and has no owner.
pub fn next_window_inactive() {
    NEXT_OWNER.set(Some(0));
}

/// Picks the window backend and the renderer before any window exists. Every window is born without a
/// taskbar button; a window announced with [`next_window_owned_by`] is also born owned and inactive.
pub fn select_backend() -> Result<(), slint::PlatformError> {
    let renderer = if cfg!(feature = "gpu") && spike::gpu() { "femtovg" } else { "software" };
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name(renderer.into())
        .with_winit_window_attributes_hook(|attributes| {
            let attributes = attributes.with_skip_taskbar(true);
            match NEXT_OWNER.take() {
                Some(0) => attributes.with_active(false),
                Some(owner) => attributes.with_active(false).with_owner_window(owner),
                None => attributes,
            }
        })
        .select()
}

/// The Win32 handle of one of the toolkit's windows, once it has been shown.
pub fn hwnd_of(window: &slint::Window) -> HWND {
    let handle = window.window_handle();
    match handle.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Win32(h)) => HWND(h.hwnd.get() as *mut _),
        _ => HWND::default(),
    }
}

const WM_QUERYENDSESSION: u32 = 0x0011;

/// Lets a window hear that the session is ending, or that it is asked to leave: the toolkit does not pass
/// that message on, so the window's messages are read first here. The app then leaves its event loop.
pub fn quit_on_end_session(hwnd: HWND) {
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(end_session), 1, 0);
    }
}

unsafe extern "system" fn end_session(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    if message == WM_QUERYENDSESSION {
        let _ = slint::quit_event_loop();
        return LRESULT(1);
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// True when this process is the first to take the name. The mutex is held until the process ends.
pub fn single_instance(name: &str) -> bool {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        let handle = CreateMutexW(None, true, &windows::core::HSTRING::from(name));
        let first = GetLastError() != ERROR_ALREADY_EXISTS;
        // Kept open on purpose: Windows releases it when the process ends.
        std::mem::forget(handle);
        first
    }
}
