//! A native popup menu at the pointer, as Windows draws its own. The caller describes the entries
//! and gets back the id of the one chosen, or none.
//!
//! `TrackPopupMenu` only dismisses the menu properly when its owner window is in the foreground,
//! so the owner is a hidden window of the app's own (never the pill, which must not be
//! activated): it is made the foreground window before the menu opens, and a `WM_NULL` is posted
//! to it after. It keeps the foreground once the menu has closed, as 1.1's menu window does.
#![allow(unsafe_code)]

use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, IsWindow, MF_CHECKED, MF_POPUP,
    MF_STRING, MF_UNCHECKED, PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_NULL,
};
use windows::core::HSTRING;

/// A choice in the menu. Its id is what `track` gives back, and is never 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub text: String,
    pub checked: bool,
}

/// A line of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    Item(Item),
    /// A line that opens a menu of its own.
    Submenu {
        text: String,
        items: Vec<Item>,
    },
}

/// Opens the menu at the pointer, bottom left corner on it, and waits until it closes. Gives the
/// id of the item chosen. `owner` is a hidden window of this thread.
pub fn track(owner: isize, entries: &[Entry]) -> Option<u32> {
    let owner = HWND(owner as *mut c_void);
    // SAFETY: asks Windows about a handle; any number is allowed.
    if !unsafe { IsWindow(Some(owner)) }.as_bool() {
        return None;
    }
    let menu = build(entries)?;
    let mut at = POINT::default();
    // SAFETY: `at` is ours to write.
    if unsafe { GetCursorPos(&mut at) }.is_err() {
        // SAFETY: `menu` is the menu made above, destroyed once.
        let _ = unsafe { DestroyMenu(menu) };
        return None;
    }
    // SAFETY: plain calls on the menu made above and on a window of this thread; Windows refuses
    // what it will not do.
    let chosen = unsafe {
        let _ = SetForegroundWindow(owner);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            at.x,
            at.y,
            None,
            owner,
            None,
        );
        // Lets the menu close for good (the documented answer to the menu staying up).
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        // Destroys the submenus with it.
        let _ = DestroyMenu(menu);
        chosen
    };
    u32::try_from(chosen.0).ok().filter(|id| *id != 0)
}

/// The menu for the entries, or none when Windows will not make one.
fn build(entries: &[Entry]) -> Option<HMENU> {
    // SAFETY: makes an empty menu.
    let menu = unsafe { CreatePopupMenu() }.ok()?;
    for entry in entries {
        let added = match entry {
            Entry::Item(item) => append(menu, item),
            Entry::Submenu { text, items } => {
                // SAFETY: makes an empty menu.
                let Ok(sub) = (unsafe { CreatePopupMenu() }) else {
                    // SAFETY: `menu` is the menu made above, destroyed once.
                    let _ = unsafe { DestroyMenu(menu) };
                    return None;
                };
                let filled = items.iter().all(|item| append(sub, item));
                // SAFETY: `sub` is a menu of ours; its handle goes in as the item's id, as the
                // call asks for a popup. A menu that was added is destroyed with its parent.
                let added = filled
                    && unsafe {
                        AppendMenuW(
                            menu,
                            MF_POPUP,
                            sub.0 as usize,
                            &HSTRING::from(text.as_str()),
                        )
                    }
                    .is_ok();
                if !added {
                    // SAFETY: `sub` was not added to a parent, so it is destroyed here, once.
                    let _ = unsafe { DestroyMenu(sub) };
                }
                added
            }
        };
        if !added {
            // SAFETY: `menu` is the menu made above, destroyed once.
            let _ = unsafe { DestroyMenu(menu) };
            return None;
        }
    }
    Some(menu)
}

fn append(menu: HMENU, item: &Item) -> bool {
    let check = if item.checked {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    };
    // SAFETY: `menu` is a menu of ours; the text is copied by the call.
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING | check,
            item.id as usize,
            &HSTRING::from(item.text.as_str()),
        )
    }
    .is_ok()
}
