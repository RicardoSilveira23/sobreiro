//! The licence window, as 1.1's is: a window titled Licences with one read-only text box that
//! fills it. It is plain Win32 and runs its own message loop on the thread that calls `show`,
//! which is the About box's thread, until it is closed. Its owner is a hidden window made on the
//! same thread, which is what keeps it out of the taskbar. One at a time: asked again, it brings
//! the open one forward.
#![allow(unsafe_code)]

use sobreiro_core::display::flyout_placement::PixelRect;
use std::cell::Cell;
use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    COLOR_WINDOW, CreateFontIndirectW, DeleteObject, HBRUSH, HFONT, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{EM_SCROLLCARET, EM_SETLIMITTEXT, EM_SETSEL};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, SystemParametersInfoForDpi};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_READONLY, GetClientRect, GetMessageW, IDC_ARROW, LoadCursorW,
    MSG, MoveWindow, NONCLIENTMETRICSW, PostQuitMessage, RegisterClassW, SPI_GETNONCLIENTMETRICS,
    SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowPos,
    SetWindowTextW, ShowWindow, TranslateMessage, UnregisterClassW, WM_CLOSE, WM_DESTROY,
    WM_DPICHANGED, WM_KEYDOWN, WM_SETFOCUS, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_CAPTION, WS_CHILD,
    WS_EX_CLIENTEDGE, WS_EX_TOOLWINDOW, WS_POPUP, WS_SYSMENU, WS_VISIBLE, WS_VSCROLL,
};
use windows::core::{PCWSTR, w};

/// The client area at 96 DPI.
const CLIENT_WIDTH: i32 = 560;
const CLIENT_HEIGHT: i32 = 520;

const CLASS: PCWSTR = w!("SobreiroLicences");
const TITLE: PCWSTR = w!("Licences");

/// The open window: 0 when none, -1 while one is being made, else its handle. Claimed first, so
/// two asks never make two windows.
static OPEN: AtomicIsize = AtomicIsize::new(0);
const MAKING: isize = -1;

// What the window proc needs, kept on the one thread that runs the window.
thread_local! {
    static EDIT: Cell<isize> = const { Cell::new(0) };
    static FONT: Cell<isize> = const { Cell::new(0) };
}

/// A size in pixels at `dpi` from its size at 96 DPI, rounded to the nearest pixel.
fn scaled(size: i32, dpi: u32) -> i32 {
    let dpi = i64::from(dpi.max(1));
    ((i64::from(size) * dpi + 48) / 96) as i32
}

/// The client area's width and height at `dpi`.
pub fn client_size(dpi: u32) -> (i32, i32) {
    (scaled(CLIENT_WIDTH, dpi), scaled(CLIENT_HEIGHT, dpi))
}

/// The top left corner of a window of `width` by `height` centred in `work`. A window larger than
/// the work area starts at its top left corner, so the title bar stays reachable.
pub fn centred(work: PixelRect, width: i32, height: i32) -> (i32, i32) {
    let x = work.left + (work.width() - width) / 2;
    let y = work.top + (work.height() - height) / 2;
    (x.max(work.left), y.max(work.top))
}

/// Shows the licence window with `text` and runs its message loop until it is closed. If one is
/// open already, brings it forward and returns at once.
pub fn show(text: &str) {
    if OPEN
        .compare_exchange(0, MAKING, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        let open = OPEN.load(Ordering::SeqCst);
        if open > 0 {
            // SAFETY: a stale handle is refused by Windows; a refused foreground is fine.
            let _ = unsafe { SetForegroundWindow(HWND(open as *mut c_void)) };
        }
        return;
    }
    run(text);
    OPEN.store(0, Ordering::SeqCst);
}

fn wide(text: &str) -> Vec<u16> {
    let mut units: Vec<u16> = text.encode_utf16().take_while(|unit| *unit != 0).collect();
    units.push(0);
    units
}

/// The message box font at `dpi`.
fn make_font(dpi: u32) -> Option<HFONT> {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    // SAFETY: `metrics` is ours to write and its size is filled in as the call asks.
    unsafe {
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS.0,
            metrics.cbSize,
            Some((&raw mut metrics).cast::<c_void>()),
            0,
            dpi,
        )
    }
    .ok()?;
    // SAFETY: the LOGFONT is the one Windows just filled in.
    let font = unsafe { CreateFontIndirectW(&metrics.lfMessageFont) };
    (!font.is_invalid()).then_some(font)
}

/// Gives the edit a new font made for `dpi` and lets the old one go.
fn remake_font(dpi: u32) {
    let old = FONT.get();
    let font = make_font(dpi);
    FONT.set(font.map_or(0, |font| font.0 as isize));
    let edit = EDIT.get();
    if edit != 0 {
        let handle = font.map_or(0, |font| font.0 as isize);
        // SAFETY: the edit is ours, on this thread; a font of 0 gives the default one.
        unsafe {
            SendMessageW(
                HWND(edit as *mut c_void),
                WM_SETFONT,
                Some(WPARAM(handle as usize)),
                Some(LPARAM(1)),
            )
        };
    }
    if old != 0 {
        // SAFETY: the old font is ours, and the edit no longer uses it.
        let _ = unsafe { DeleteObject(HGDIOBJ(old as *mut c_void)) };
    }
}

fn fit_edit(window: HWND) {
    let edit = EDIT.get();
    if edit == 0 {
        return;
    }
    let mut client = RECT::default();
    // SAFETY: `client` is ours to write; the windows are ours, on this thread.
    unsafe {
        if GetClientRect(window, &mut client).is_ok() {
            let _ = MoveWindow(
                HWND(edit as *mut c_void),
                0,
                0,
                client.right,
                client.bottom,
                true,
            );
        }
    }
}

extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SETFOCUS => {
            let edit = EDIT.get();
            if edit != 0 {
                // SAFETY: the edit is our child, on this thread.
                let _ = unsafe { SetFocus(Some(HWND(edit as *mut c_void))) };
            }
            LRESULT(0)
        }
        WM_SIZE => {
            fit_edit(window);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let dpi = (wparam.0 & 0xFFFF) as u32;
            // SAFETY: for this message `lparam` points at the rectangle Windows suggests.
            let suggested = unsafe { *(lparam.0 as *const RECT) };
            // SAFETY: the window is ours, on this thread.
            let _ = unsafe {
                SetWindowPos(
                    window,
                    None,
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                )
            };
            remake_font(dpi);
            LRESULT(0)
        }
        WM_CLOSE => {
            // SAFETY: the window is ours, on this thread.
            let _ = unsafe { DestroyWindow(window) };
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: the default handling of a message of this window.
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

fn run(text: &str) {
    let Some(screen) = super::monitor::cursor()
        .and_then(|(x, y)| super::monitor::at_point(x, y))
        .or_else(super::monitor::primary)
    else {
        return;
    };
    let dpi = screen.dpi;
    // SAFETY: asks for this exe's own module handle.
    let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
        return;
    };
    let instance = HINSTANCE(module.0);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        // SAFETY: loads the system's arrow, which is never freed.
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as usize as *mut c_void),
        lpszClassName: CLASS,
        ..Default::default()
    };
    // SAFETY: the class is complete, and its name and procedure outlive it.
    if unsafe { RegisterClassW(&class) } == 0 {
        return;
    }
    let owner = make_owner(instance);
    let made = owner.and_then(|owner| make_window(instance, owner, screen, dpi, text));
    if let Some(window) = made {
        OPEN.store(window.0 as isize, Ordering::SeqCst);
        let mut message = MSG::default();
        // SAFETY: the loop reads this thread's messages into `message`, which is ours.
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            // The edit takes the keys, so Escape is looked for before the message is dispatched.
            if message.message == WM_KEYDOWN && message.wParam.0 == usize::from(VK_ESCAPE.0) {
                // SAFETY: the window is ours, on this thread.
                let _ = unsafe { DestroyWindow(window) };
                continue;
            }
            // SAFETY: a message the loop just read.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    // Everything made is let go: the font, the owner, the class.
    let font = FONT.replace(0);
    EDIT.set(0);
    if font != 0 {
        // SAFETY: the font is ours, and the edit that used it is gone.
        let _ = unsafe { DeleteObject(HGDIOBJ(font as *mut c_void)) };
    }
    if let Some(owner) = owner {
        // SAFETY: the owner is ours, on this thread.
        let _ = unsafe { DestroyWindow(owner) };
    }
    // SAFETY: no window of the class is left.
    let _ = unsafe { UnregisterClassW(CLASS, Some(instance)) };
}

/// The hidden window that owns the licence window, which keeps it out of the taskbar.
fn make_owner(instance: HINSTANCE) -> Option<HWND> {
    // SAFETY: a hidden popup of the system's own STATIC class; nothing is passed along.
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("STATIC"),
            PCWSTR::null(),
            WS_POPUP,
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
    .ok()
}

fn make_window(
    instance: HINSTANCE,
    owner: HWND,
    screen: super::monitor::Screen,
    dpi: u32,
    text: &str,
) -> Option<HWND> {
    let style = WS_CAPTION | WS_SYSMENU;
    let (client_width, client_height) = client_size(dpi);
    let mut frame = RECT {
        left: 0,
        top: 0,
        right: client_width,
        bottom: client_height,
    };
    // SAFETY: `frame` is ours to change.
    unsafe { AdjustWindowRectExForDpi(&mut frame, style, false, Default::default(), dpi) }.ok()?;
    let (width, height) = (frame.right - frame.left, frame.bottom - frame.top);
    let (x, y) = centred(screen.work_area, width, height);
    // SAFETY: the class is registered, the owner is ours, and the title is a literal.
    let window = unsafe {
        CreateWindowExW(
            Default::default(),
            CLASS,
            TITLE,
            style,
            x,
            y,
            width,
            height,
            Some(owner),
            None,
            Some(instance),
            None,
        )
    }
    .ok()?;
    // SAFETY: the edit is a child of the window just made, with the system's own class.
    let edit = unsafe {
        CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            PCWSTR::null(),
            WS_CHILD
                | WS_VISIBLE
                | WS_VSCROLL
                | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    (ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL) as u32,
                ),
            0,
            0,
            client_width,
            client_height,
            Some(window),
            None,
            Some(instance),
            None,
        )
    };
    let Ok(edit) = edit else {
        // SAFETY: the window is ours, on this thread.
        let _ = unsafe { DestroyWindow(window) };
        return None;
    };
    EDIT.set(edit.0 as isize);
    remake_font(dpi);
    let text = wide(text);
    // SAFETY: the edit is ours; the text is nul-ended and alive for the call. The limit is raised
    // before the text is set, as the notices are long.
    unsafe {
        SendMessageW(
            edit,
            EM_SETLIMITTEXT,
            Some(WPARAM(text.len() + 1)),
            Some(LPARAM(0)),
        );
        let _ = SetWindowTextW(edit, PCWSTR(text.as_ptr()));
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(0)));
        SendMessageW(edit, EM_SCROLLCARET, Some(WPARAM(0)), Some(LPARAM(0)));
        let _ = ShowWindow(window, SW_SHOW);
        let _ = SetForegroundWindow(window);
        let _ = SetFocus(Some(edit));
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(0)));
    }
    Some(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
        PixelRect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn the_client_area_scales_with_the_dpi_and_rounds() {
        assert_eq!(client_size(96), (560, 520));
        assert_eq!(client_size(144), (840, 780));
        assert_eq!(client_size(192), (1120, 1040));
        // 560 * 120 / 96 = 700; 520 * 120 / 96 = 650; 125% gives 700 and 650 too.
        assert_eq!(client_size(120), (700, 650));
        // 520 * 110 / 96 = 595.83, rounded up.
        assert_eq!(client_size(110), (642, 596));
    }

    #[test]
    fn the_window_is_centred_in_the_work_area() {
        assert_eq!(centred(rect(0, 0, 1920, 1040), 600, 600), (660, 220));
        // A second monitor to the left, with a negative origin.
        assert_eq!(centred(rect(-1920, 0, 0, 1040), 600, 600), (-1260, 220));
        // An odd gap rounds down.
        assert_eq!(centred(rect(0, 0, 1001, 1001), 600, 600), (200, 200));
    }

    #[test]
    fn a_window_larger_than_the_work_area_starts_at_its_corner() {
        assert_eq!(centred(rect(100, 50, 700, 450), 900, 900), (100, 50));
    }
}
