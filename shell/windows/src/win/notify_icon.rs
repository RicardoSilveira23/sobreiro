//! A notification (tray) icon of the app's own: added, changed and removed through
//! `Shell_NotifyIconW`, one icon id at a time, on a window of ours that hears its mouse and
//! keyboard messages. The icon is not named by a GUID: Windows remembers an icon the user moved
//! out of the overflow by the exe's path and the id.
#![allow(unsafe_code)]

use super::tray_picture::TrayIcon;
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NIN_SELECT, NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, NOTIFYICONDATAW_0,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{WM_APP, WM_CONTEXTMENU};

/// The message the icons' events come as, to the window they were added on.
pub const TRAY_MESSAGE: u32 = WM_APP + 1;

/// The id of Claude Code's icon.
pub const CLAUDE_ID: u32 = 1;
/// The id of Copilot's icon.
pub const COPILOT_ID: u32 = 2;

/// Enter or Space on a focused icon (`NIN_SELECT` with the key flag).
pub const NIN_KEYSELECT: u32 = NIN_SELECT | 0x1;
/// The menu key, or a right click, on the icon.
pub const CONTEXT_MENU: u32 = WM_CONTEXTMENU;
/// A left click on the icon.
pub const SELECT: u32 = NIN_SELECT;

/// How many UTF-16 units a tooltip holds, leaving room for the NUL.
const TIP_UNITS: usize = 127;

/// What an icon's message says, as version 4 sends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayEvent {
    /// The notification: a mouse message, `NIN_SELECT`, `NIN_KEYSELECT`, `WM_CONTEXTMENU`...
    pub event: u32,
    /// The icon's id.
    pub id: u32,
    /// Where the event is anchored, in screen coordinates.
    pub x: i32,
    pub y: i32,
}

/// Reads the message's two parameters (version 4): the low word of `lparam` is the event, the high
/// word the icon id; `wparam` holds the anchor, x in its low 16 bits and y in its high 16 bits,
/// each signed.
pub fn decode(wparam: usize, lparam: isize) -> TrayEvent {
    let low = |value: usize| (value & 0xFFFF) as u32;
    let high = |value: usize| ((value >> 16) & 0xFFFF) as u32;
    let lparam = lparam as usize;
    TrayEvent {
        event: low(lparam),
        id: high(lparam),
        x: i32::from(low(wparam) as u16 as i16),
        y: i32::from(high(wparam) as u16 as i16),
    }
}

/// The tooltip as `szTip` holds it: at most 127 UTF-16 units, a surrogate pair never split, and
/// the rest NUL.
pub fn tip_units(text: &str) -> [u16; 128] {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut end = units.len().min(TIP_UNITS);
    // A cut that would leave a high surrogate last drops it with the half it lost.
    if end < units.len()
        && end > 0
        && let Some(last) = units.get(end - 1)
        && (0xD800..0xDC00).contains(last)
    {
        end -= 1;
    }
    let mut tip = [0u16; 128];
    for (slot, unit) in tip.iter_mut().zip(units.iter().take(end)) {
        *slot = *unit;
    }
    tip
}

fn data(window: isize, id: u32, icon: &TrayIcon, tip: &str) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(window as *mut c_void),
        uID: id,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP,
        uCallbackMessage: TRAY_MESSAGE,
        hIcon: icon.handle(),
        szTip: tip_units(tip),
        ..Default::default()
    }
}

fn notify(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
    // SAFETY: `data` is alive for the call with its size filled in as the call asks.
    unsafe { Shell_NotifyIconW(message, data) }.as_bool()
}

/// Adds the icon. A failure can be a timeout of the shell with the icon added all the same, so a
/// modify is tried after it: either one working counts. Then the icon is moved to version 4
/// messages; one left on the older messages would not answer its clicks, so it is removed and
/// the add counts as failed.
pub fn add(window: isize, id: u32, icon: &TrayIcon, tip: &str) -> bool {
    let data = data(window, id, icon, tip);
    if !notify(NIM_ADD, &data) && !notify(NIM_MODIFY, &data) {
        return false;
    }
    let version = NOTIFYICONDATAW {
        Anonymous: NOTIFYICONDATAW_0 {
            uVersion: NOTIFYICON_VERSION_4,
        },
        ..data
    };
    if !notify(NIM_SETVERSION, &version) {
        delete(window, id);
        return false;
    }
    true
}

/// Changes an added icon's picture and tooltip.
pub fn modify(window: isize, id: u32, icon: &TrayIcon, tip: &str) -> bool {
    notify(NIM_MODIFY, &data(window, id, icon, tip))
}

/// Removes the icon; one that is not there is not an error.
pub fn delete(window: isize, id: u32) {
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(window as *mut c_void),
        uID: id,
        ..Default::default()
    };
    notify(NIM_DELETE, &data);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(tip: &[u16; 128]) -> Vec<u16> {
        tip.iter().copied().take_while(|unit| *unit != 0).collect()
    }

    #[test]
    fn a_short_tip_is_kept_and_terminated() {
        let tip = tip_units("Claude");
        assert_eq!(String::from_utf16_lossy(&text(&tip)), "Claude");
        assert_eq!(tip[127], 0);
    }

    #[test]
    fn a_tip_of_127_units_is_kept_whole() {
        let long = "a".repeat(127);
        let tip = tip_units(&long);
        assert_eq!(text(&tip).len(), 127);
        assert_eq!(tip[127], 0);
    }

    #[test]
    fn a_longer_tip_is_cut_at_127() {
        let tip = tip_units(&"a".repeat(300));
        assert_eq!(text(&tip).len(), 127);
        assert_eq!(tip[127], 0);
    }

    #[test]
    fn a_surrogate_pair_at_the_boundary_is_not_split() {
        // 126 units, then a pair: the pair would end at 128, so it goes whole.
        let cut = format!("{}\u{1F600}", "a".repeat(126));
        let tip = tip_units(&cut);
        assert_eq!(text(&tip).len(), 126);
        // 125 units, then a pair that ends exactly at 127: kept.
        let kept = format!("{}\u{1F600}b", "a".repeat(125));
        let tip = tip_units(&kept);
        assert_eq!(text(&tip).len(), 127);
        assert_eq!(
            String::from_utf16_lossy(&text(&tip)).chars().last(),
            Some('\u{1F600}')
        );
    }

    #[test]
    fn decodes_the_version_4_message() {
        let wparam = (200usize << 16) | 100;
        let lparam = ((2isize) << 16) | NIN_SELECT as isize;
        assert_eq!(
            decode(wparam, lparam),
            TrayEvent {
                event: NIN_SELECT,
                id: 2,
                x: 100,
                y: 200
            }
        );
    }

    #[test]
    fn decodes_negative_anchors() {
        // A second monitor to the left of the primary has negative x.
        let x = (-30i16) as u16 as usize;
        let y = (-5i16) as u16 as usize;
        let event = decode((y << 16) | x, ((1isize) << 16) | CONTEXT_MENU as isize);
        assert_eq!(
            (event.event, event.id, event.x, event.y),
            (CONTEXT_MENU, 1, -30, -5)
        );
    }
}
