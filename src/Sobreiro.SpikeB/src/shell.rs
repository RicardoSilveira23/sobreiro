//! SPIKE (shell candidate B): choosing the toolkit's backend and renderer, reaching its window handle, and
//! taking a window over at its birth with Windows' own means, so that nothing leans on the toolkit's
//! unstable hook. That hook is still here behind the `toolkit-hook` feature, for the comparison.
use crate::{native, spike, trace};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::cell::{Cell, RefCell};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CBT_CREATEWNDW, CallNextHookEx, GWL_EXSTYLE, HCBT_ACTIVATE, HCBT_CREATEWND, HCBT_DESTROYWND, STYLESTRUCT, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetWindowsHookExW, WH_CBT, WINDOWPOS, WM_ACTIVATE, WM_NCDESTROY, WM_STYLECHANGING, WM_WINDOWPOSCHANGING,
    WS_CHILD, WS_EX_APPWINDOW,
};
use std::rc::Rc;

/// How a window comes to be owned and left inactive. `SOBREIRO_SPIKE_BORN` picks one; `Ours` is the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Born {
    /// Nothing at birth: the control. Owner and style bits are put on once the window is shown.
    Plain,
    /// Windows is told not to activate the window; owner and style bits once it is shown.
    Refuse,
    /// Owner, style bits and a window procedure of ours from the moment Windows makes the window.
    Ours,
    /// The toolkit's own hook on a window's attributes, which it marks unstable.
    #[cfg(feature = "toolkit-hook")]
    Hook,
}

impl Born {
    pub fn chosen() -> Born {
        match spike::env("SOBREIRO_SPIKE_BORN").as_deref() {
            Some("plain") => Born::Plain,
            Some("refuse") => Born::Refuse,
            #[cfg(feature = "toolkit-hook")]
            Some("hook") => Born::Hook,
            _ => Born::Ours,
        }
    }

    /// True when the owner and the style bits still have to be put on after the window is shown.
    pub fn owned_afterwards(self) -> bool {
        matches!(self, Born::Plain | Born::Refuse)
    }
}

/// What happened to one window that was born through [`show_born`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Birth {
    pub hwnd: isize,
    /// Times Windows was about to activate the window, and how many of those were refused.
    pub activations: u32,
    pub refused: u32,
    /// Times something tried to write the extended style without our bits.
    pub styles_kept: u32,
    /// The owner it is given right before it is first shown, and whether that has been done.
    pub owner: isize,
    pub owned: bool,
    pub inactive: bool,
}

/// What a window is to be from its birth on.
#[derive(Clone, Copy)]
pub struct Want {
    /// The window that owns it, or none.
    pub owner: HWND,
    /// Extended style bits it has from birth and keeps whatever the toolkit writes.
    pub ex_style: isize,
    /// Never activated, whoever shows, moves or raises it.
    pub inactive: bool,
}

struct Next {
    born: Born,
    want: Want,
}

const INACTIVE: usize = 1 << 32;

thread_local! {
    static NEXT: RefCell<Option<Next>> = const { RefCell::new(None) };
    static BIRTHS: RefCell<Vec<(Born, Birth)>> = const { RefCell::new(Vec::new()) };
    static WATCHED: Cell<bool> = const { Cell::new(false) };
    static NONE_ACTIVATED: Cell<bool> = const { Cell::new(false) };
    static DEACTIVATED: RefCell<Vec<(isize, Rc<dyn Fn()>)>> = const { RefCell::new(Vec::new()) };
    #[cfg(feature = "toolkit-hook")]
    static NEXT_OWNER: Cell<Option<isize>> = const { Cell::new(None) };
}

/// Shows one of the toolkit's windows for the first time. The toolkit only notes the request and makes the
/// window on a later turn of the event loop, so the next top-level window Windows makes on this thread is
/// taken to be this one: with [`Born::Ours`] it is what `want` says from its birth. One window at a time:
/// false means another is still waiting to be made, and the caller asks again later. [`birth_of`] says
/// afterwards whether the window that was made is the one that was meant.
pub fn show_born(born: Born, want: Want, show: impl FnOnce() -> Result<(), slint::PlatformError>) -> bool {
    watch_this_thread();
    if NEXT.with_borrow(|next| next.is_some()) {
        return false;
    }
    NEXT.set(Some(Next { born, want }));
    if let Err(error) = show() {
        trace!("show failed: {error}");
        NEXT.set(None);
    }
    true
}

/// From now on Windows activates no window of this thread. For a run that only renders pictures.
pub fn no_window_is_activated() {
    watch_this_thread();
    NONE_ACTIVATED.set(true);
}

/// Calls `action` when a window born as [`Born::Ours`] stops being the active window.
pub fn on_deactivated(hwnd: HWND, action: impl Fn() + 'static) {
    DEACTIVATED.with_borrow_mut(|list| list.push((hwnd.0 as isize, Rc::new(action))));
}

/// The record of a window born through [`show_born`], as it stands now.
pub fn birth_of(hwnd: HWND) -> Option<Birth> {
    BIRTHS.with_borrow(|births| births.iter().find(|(_, b)| b.hwnd == hwnd.0 as isize).map(|(_, b)| *b))
}

// One hook per thread, for as long as the thread lives: Windows calls it for windows of this thread only.
fn watch_this_thread() {
    if WATCHED.replace(true) {
        return;
    }
    if unsafe { SetWindowsHookExW(WH_CBT, Some(on_cbt), None, GetCurrentThreadId()) }.is_err() {
        trace!("born: the window hook could not be set");
    }
}

// Windows still calls the two procedures below while the thread is ending and its thread-local values are
// already gone: every read of one from there is a `try_with`, and finds nothing then.
fn with_birth<T>(hwnd: HWND, action: impl FnOnce(Born, &mut Birth) -> T) -> Option<T> {
    BIRTHS
        .try_with(|births| {
            let mut births = births.try_borrow_mut().ok()?;
            let (born, birth) = births.iter_mut().find(|(_, b)| b.hwnd == hwnd.0 as isize)?;
            Some(action(*born, birth))
        })
        .ok()
        .flatten()
}

unsafe extern "system" fn on_cbt(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hwnd = HWND(wparam.0 as *mut _);
    if code == HCBT_CREATEWND as i32 {
        // Windows has made the window and has not yet sent it a message. Only a top-level window is ours.
        let create = unsafe { &*(*(lparam.0 as *const CBT_CREATEWNDW)).lpcs };
        let top_level = create.hwndParent.0.is_null() && create.style & WS_CHILD.0 as i32 == 0;
        if top_level && let Some(next) = NEXT.try_with(|next| next.take()).ok().flatten() {
            let ours = next.born == Born::Ours;
            let birth = Birth {
                hwnd: hwnd.0 as isize,
                owner: next.want.owner.0 as isize,
                owned: !ours || next.want.owner.0.is_null(),
                inactive: ours && next.want.inactive,
                ..Default::default()
            };
            let _ = BIRTHS.try_with(|births| births.borrow_mut().push((next.born, birth)));
            if ours {
                // The owner cannot be given here: Windows has not settled it yet, and what is written is lost.
                let data = next.want.ex_style as usize | if next.want.inactive { INACTIVE } else { 0 };
                unsafe {
                    let _ = SetWindowSubclass(hwnd, Some(born_ours), 2, data);
                }
                native::add_ex_style(hwnd, next.want.ex_style);
            }
        }
    } else if code == HCBT_ACTIVATE as i32 {
        let refuse = with_birth(hwnd, |born, birth| {
            birth.activations += 1;
            let refuse = born == Born::Refuse || birth.inactive && spike::env("SOBREIRO_SPIKE_OURS").as_deref() != Some("lenient");
            if refuse {
                birth.refused += 1;
            }
            refuse
        });
        if refuse == Some(true) || NONE_ACTIVATED.try_with(Cell::get).unwrap_or(false) {
            return LRESULT(1);
        }
    } else if code == HCBT_DESTROYWND as i32 {
        let _ = BIRTHS.try_with(|births| births.borrow_mut().retain(|(_, b)| b.hwnd != hwnd.0 as isize));
        let _ = DEACTIVATED.try_with(|list| list.borrow_mut().retain(|(h, _)| *h != hwnd.0 as isize));
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

// Read before the toolkit's own window procedure, from the window's first message on. `data` holds the
// extended style bits to keep and whether the window is never activated.
unsafe extern "system" fn born_ours(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
    let keep = (data & 0xFFFF_FFFF) as u32;
    match message {
        WM_WINDOWPOSCHANGING => {
            let position = unsafe { &mut *(lparam.0 as *mut WINDOWPOS) };
            // Whoever shows, moves or raises the window, it is not activated by it.
            if data & INACTIVE != 0 {
                position.flags |= SWP_NOACTIVATE;
            }
            // Right before it is first shown, the window is given its owner.
            if position.flags.contains(SWP_SHOWWINDOW) {
                let owner = with_birth(hwnd, |_, birth| (!std::mem::replace(&mut birth.owned, true)).then_some(birth.owner));
                if let Some(Some(owner)) = owner {
                    native::set_owner(hwnd, HWND(owner as *mut _));
                }
            }
        }
        // The toolkit writes the whole extended style when one of its own window settings changes: ours stay,
        // and the bit that asks for a taskbar button goes.
        WM_STYLECHANGING if wparam.0 as i32 == GWL_EXSTYLE.0 => {
            let style = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
            let wanted = (style.styleNew | keep) & !WS_EX_APPWINDOW.0;
            if style.styleNew != wanted {
                style.styleNew = wanted;
                with_birth(hwnd, |_, birth| birth.styles_kept += 1);
            }
        }
        WM_ACTIVATE if wparam.0 & 0xFFFF == 0 => {
            let action = DEACTIVATED.try_with(|list| list.borrow().iter().find(|(h, _)| *h == hwnd.0 as isize).map(|(_, a)| a.clone()));
            if let Ok(Some(action)) = action {
                action();
            }
        }
        WM_NCDESTROY => unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(born_ours), 2);
        },
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// With the toolkit's hook: the next window the toolkit makes on this thread is born owned by `owner` and is
/// not activated when shown. Called right before the component is made.
#[cfg(feature = "toolkit-hook")]
pub fn next_window_owned_by(owner: HWND) {
    NEXT_OWNER.set(Some(owner.0 as isize));
}

/// Picks the window backend and the renderer before any window exists.
pub fn select_backend() -> Result<(), slint::PlatformError> {
    let renderer = if cfg!(feature = "gpu") && spike::gpu() { "femtovg" } else { "software" };
    let selector = slint::BackendSelector::new().backend_name("winit".into()).renderer_name(renderer.into());
    #[cfg(feature = "toolkit-hook")]
    let selector = {
        use slint::winit_030::winit::platform::windows::WindowAttributesExtWindows;
        selector.with_winit_window_attributes_hook(|attributes| match NEXT_OWNER.take() {
            Some(owner) => attributes.with_active(false).with_owner_window(owner),
            None => attributes,
        })
    };
    selector.select()
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
