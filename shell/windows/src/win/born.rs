//! A window that is born owned by another window and is never activated, made without the
//! toolkit's unstable window-attributes hook (docs/shell-comparison.md, "Owned by the taskbar and
//! not activated").
//!
//! The toolkit only notes that a window is to be shown and makes it a turn of the event loop
//! later, so there is no moment to reach it first. A `WH_CBT` hook on the UI thread takes the next
//! top-level window as it is created and puts a window procedure of ours before the toolkit's.
//! That procedure keeps the tool-window and no-activate style bits, gives the owner at the first
//! show (an owner written at creation is lost), adds no-activate to every show, move and raise,
//! and answers a click with "do not activate". The hook refuses the one activation Windows still
//! asks for.
//!
//! One window is *made* at a time, and any number may be alive: call `show` with a toolkit window
//! that already exists, wait until `owned_windows` lists it, and only then show the next. `show`
//! refuses while a window is still pending. The window must exist first, because the toolkit makes
//! a hidden top-level window of its own when it is first used, and the hook would take that one.
//! The hook is set by the first `show` and stays until the last taken window is destroyed and
//! nothing is pending; it has to refuse the activation for
//! as long as any of them lives.
#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CBT_CREATEWNDW, CallNextHookEx, CallWindowProcW, DefWindowProcW, GW_OWNER, GWL_EXSTYLE,
    GWLP_HWNDPARENT, GWLP_WNDPROC, GetWindow, HHOOK, IsWindow, STYLESTRUCT, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowsHookExW, UnhookWindowsHookEx, WH_CBT, WINDOWPOS,
    WM_MOUSEACTIVATE, WM_NCDESTROY, WM_SHOWWINDOW, WM_STYLECHANGING, WM_WINDOWPOSCHANGING, WNDPROC,
    WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

const HCBT_CREATEWND: i32 = 3;
const HCBT_ACTIVATE: i32 = 5;
const MA_NOACTIVATE: isize = 3;

/// A window taken at its birth. Plain numbers, so it can be copied out of the list.
#[derive(Clone, Copy)]
struct Taken {
    window: isize,
    owner: isize,
    /// What SetWindowLongPtrW gave back for the window's procedure. It may be an opaque handle,
    /// not an address, so it is only ever passed to CallWindowProcW.
    old_procedure: isize,
    /// The owner has been given.
    owned: bool,
}

thread_local! {
    /// The owner for the next top-level window, once `arm` has been called and until it is taken.
    static PENDING: Cell<Option<isize>> = const { Cell::new(None) };
    /// The hook's handle, 0 while there is none.
    static HOOK: Cell<isize> = const { Cell::new(0) };
    /// The windows taken, in the order they were taken.
    static TAKEN: RefCell<Vec<Taken>> = const { RefCell::new(Vec::new()) };
}

// Windows calls the hook and the procedure at thread exit too, after the thread's values are
// gone. Every read below uses `try_with`; when it fails the caller lets the call through.

fn pending() -> Option<isize> {
    PENDING.try_with(Cell::get).ok().flatten()
}

fn find(window: isize) -> Option<Taken> {
    TAKEN
        .try_with(|taken| taken.borrow().iter().find(|t| t.window == window).copied())
        .ok()
        .flatten()
}

fn edit(window: isize, change: impl FnOnce(&mut Taken)) {
    let _ = TAKEN.try_with(|taken| {
        if let Some(entry) = taken.borrow_mut().iter_mut().find(|t| t.window == window) {
            change(entry);
        }
    });
}

/// Shows `window`, which must already exist, born owned by `owner` (a window handle as a number)
/// and never activated. Nothing is left pending if the toolkit refuses to show it. The owner has
/// been given when `owned_windows` lists the window; that is at its first show, a turn of the
/// event loop later.
pub fn show(window: &slint::Window, owner: isize) -> Result<(), String> {
    arm(owner)?;
    window.show().map_err(|error| {
        disarm();
        error.to_string()
    })
}

/// Whether `handle` is a window that exists.
pub fn is_window(handle: isize) -> bool {
    // SAFETY: asks Windows about a handle; any number is allowed.
    unsafe { IsWindow(Some(HWND(handle as *mut c_void))) }.as_bool()
}

/// Sets the hook for the UI thread, if it is not set, and asks for the next top-level window this
/// thread creates to be born owned by `owner`.
fn arm(owner: isize) -> Result<(), String> {
    if PENDING
        .try_with(Cell::get)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("a window is already being made".into());
    }
    if HOOK.try_with(Cell::get).map_err(|e| e.to_string())? == 0 {
        // SAFETY: the hook procedure is a function of this module that lives as long as the
        // process. The hook is set for this thread only, but it is called for every window the
        // thread makes, so it lets through whatever is not the pending window.
        let hook =
            unsafe { SetWindowsHookExW(WH_CBT, Some(hook_procedure), None, GetCurrentThreadId()) }
                .map_err(|error| error.to_string())?;
        let _ = HOOK.try_with(|cell| cell.set(hook.0 as isize));
    }
    let _ = PENDING.try_with(|cell| cell.set(Some(owner)));
    Ok(())
}

/// Whether a window shown with `show` has not been created yet, so the next top-level window
/// this thread makes would be taken for it.
pub fn is_pending() -> bool {
    pending().is_some()
}

/// Gives up on the window still pending, if any, so `show` can be called again. For a toolkit
/// window that is dropped before the toolkit made it.
pub fn cancel_pending() {
    disarm();
}

/// Takes back an `arm` that no window answered, and the hook if nothing else needs it.
fn disarm() {
    let _ = PENDING.try_with(|cell| cell.set(None));
    unhook_if_idle();
}

/// The windows whose owner has been given and holds (checked with `GetWindow`), in the order they
/// were taken. A window whose owner Windows refused is not listed; it is tried again at its next
/// show.
pub fn owned_windows() -> Vec<isize> {
    TAKEN
        .try_with(|taken| {
            taken
                .borrow()
                .iter()
                .filter(|t| t.owned)
                .map(|t| t.window)
                .collect()
        })
        .unwrap_or_default()
}

fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: passes on the values Windows gave the hook, unchanged; the hook handle is ignored
    // on current Windows.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn hook_procedure(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match code {
        HCBT_CREATEWND if pending().is_some() => {
            let create = lparam.0 as *const CBT_CREATEWNDW;
            // SAFETY: for HCBT_CREATEWND `lparam` points to a CBT_CREATEWNDW, and its `lpcs` to
            // the window's CREATESTRUCTW, both valid for the length of this call.
            let parent = unsafe { create.as_ref().and_then(|create| create.lpcs.as_ref()) }
                .map(|create| create.hwndParent);
            // A message-only window has a parent; only a top-level one has none.
            if parent.is_some_and(|parent| parent.0.is_null()) {
                subclass(HWND(wparam.0 as *mut c_void));
            }
        }
        HCBT_ACTIVATE if find(wparam.0 as isize).is_some() => {
            // A non-zero answer stops the activation.
            return LRESULT(1);
        }
        _ => {}
    }
    call_next(code, wparam, lparam)
}

/// Puts our procedure before the toolkit's on the window that was just created, and takes the
/// pending owner for it.
fn subclass(window: HWND) {
    let Some(owner) = pending() else {
        return;
    };
    // SAFETY: `window` is the window the hook was just told about, created by this thread, so
    // replacing its procedure is allowed. `procedure` has the signature of a window procedure.
    let old = unsafe { SetWindowLongPtrW(window, GWLP_WNDPROC, procedure as *const () as isize) };
    if old == 0 {
        return;
    }
    let _ = TAKEN.try_with(|taken| {
        taken.borrow_mut().push(Taken {
            window: window.0 as isize,
            owner,
            old_procedure: old,
            owned: false,
        });
    });
    let _ = PENDING.try_with(|cell| cell.set(None));
}

/// The style bits the window keeps whatever the toolkit writes.
fn keep_style(style: u32) -> u32 {
    (style | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) & !WS_EX_APPWINDOW.0
}

/// Gives the owner, until it holds. Windows may refuse one, so it is read back before the window
/// counts as owned.
fn give_owner(window: HWND, taken: Taken) {
    if taken.owned {
        return;
    }
    // SAFETY: `window` is ours and this runs on its thread. The owner is a handle that was given
    // by the caller of `show`; Windows refuses one that is not a window.
    let holds = unsafe {
        SetWindowLongPtrW(window, GWLP_HWNDPARENT, taken.owner);
        GetWindow(window, GW_OWNER)
    }
    .is_ok_and(|owner| owner.0 as isize == taken.owner);
    if holds {
        edit(taken.window, |taken| taken.owned = true);
    }
}

/// Removes a destroyed window from the list, and the hook when no window is left or pending.
fn forget(window: isize) {
    let _ = TAKEN.try_with(|taken| taken.borrow_mut().retain(|t| t.window != window));
    unhook_if_idle();
}

fn unhook_if_idle() {
    let empty = TAKEN
        .try_with(|taken| taken.borrow().is_empty())
        .unwrap_or(false);
    if empty && pending().is_none() {
        let hook = HOOK.try_with(|cell| cell.replace(0)).unwrap_or(0);
        if hook != 0 {
            // SAFETY: `hook` is the handle SetWindowsHookExW gave for this thread.
            let _ = unsafe { UnhookWindowsHookEx(HHOOK(hook as *mut c_void)) };
        }
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(taken) = find(window.0 as isize) else {
        // SAFETY: the default procedure takes any message.
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    };
    match message {
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            // SAFETY: for this message `lparam` points to a STYLESTRUCT that Windows lets the
            // procedure change.
            if let Some(styles) = unsafe { (lparam.0 as *mut STYLESTRUCT).as_mut() } {
                styles.styleNew = keep_style(styles.styleNew);
            }
        }
        WM_WINDOWPOSCHANGING => {
            // SAFETY: for this message `lparam` points to a WINDOWPOS that Windows lets the
            // procedure change.
            if let Some(position) = unsafe { (lparam.0 as *mut WINDOWPOS).as_mut() } {
                if position.flags.contains(SWP_SHOWWINDOW) {
                    give_owner(window, taken);
                }
                position.flags |= SWP_NOACTIVATE;
            }
        }
        WM_SHOWWINDOW if wparam.0 != 0 => give_owner(window, taken),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE),
        _ => {}
    }
    let old: WNDPROC =
        // SAFETY: `old_procedure` is what SetWindowLongPtrW gave back when ours was put in. It is
        // only passed on to CallWindowProcW, which takes it as it is.
        unsafe { std::mem::transmute::<isize, WNDPROC>(taken.old_procedure) };
    // SAFETY: calls the previous procedure with the message as it came.
    let result = unsafe { CallWindowProcW(old, window, message, wparam, lparam) };
    if message == WM_NCDESTROY {
        forget(taken.window);
    }
    result
}
