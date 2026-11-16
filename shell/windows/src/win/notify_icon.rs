//! A notification (tray) icon of the app's own: added, changed and removed through
//! `Shell_NotifyIconW`, one icon id at a time, on a window of ours that hears its mouse and
//! keyboard messages. The icon is not named by a GUID: Windows remembers an icon the user moved
//! out of the overflow by the exe's path and the id.
#![allow(unsafe_code)]

use super::tray_picture::TrayIcon;
use std::ffi::c_void;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_NONE, NIM_ADD, NIM_DELETE,
    NIM_MODIFY, NIM_SETVERSION, NIN_SELECT, NOTIFY_ICON_MESSAGE, NOTIFYICON_VERSION_4,
    NOTIFYICONDATAW, NOTIFYICONDATAW_0, Shell_NotifyIconW,
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

/// How long a balloon stays, in milliseconds, as 1.1 shows it.
const BALLOON_MS: u32 = 10_000;

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

/// The text as a field of `N` units holds it: at most `N - 1` UTF-16 units, a surrogate pair never
/// split, and the rest NUL.
pub fn units<const N: usize>(text: &str) -> [u16; N] {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut end = units.len().min(N.saturating_sub(1));
    // A cut that would leave a high surrogate last drops it with the half it lost.
    if end < units.len()
        && end > 0
        && let Some(last) = units.get(end - 1)
        && (0xD800..0xDC00).contains(last)
    {
        end -= 1;
    }
    let mut field = [0u16; N];
    for (slot, unit) in field.iter_mut().zip(units.iter().take(end)) {
        *slot = *unit;
    }
    field
}

/// The tooltip as `szTip` holds it: 127 units and a NUL.
pub fn tip_units(text: &str) -> [u16; 128] {
    units(text)
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

/// What a balloon asks of the shell: the balloon, so the picture and tooltip text stay as they
/// are, and the standard tooltip kept on, which a version 4 icon loses on a change without it.
/// No icon in the balloon, and 1.1's ten seconds.
fn balloon_data(window: isize, id: u32, title: &str, text: &str) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: HWND(window as *mut c_void),
        uID: id,
        uFlags: NIF_INFO | NIF_SHOWTIP,
        szInfoTitle: units(title),
        szInfo: units(text),
        Anonymous: NOTIFYICONDATAW_0 {
            uTimeout: BALLOON_MS,
        },
        dwInfoFlags: NIIF_NONE,
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

/// Shows a balloon from an added icon. False when the shell refuses it. With Windows
/// notifications off the call still succeeds and nothing shows.
pub fn balloon(window: isize, id: u32, title: &str, text: &str) -> bool {
    notify(NIM_MODIFY, &balloon_data(window, id, title, text))
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

    fn field<const N: usize>(units: &[u16; N]) -> Vec<u16> {
        units
            .iter()
            .copied()
            .take_while(|unit| *unit != 0)
            .collect()
    }

    #[test]
    fn the_balloon_asks_for_the_balloon_and_keeps_the_tooltip() {
        let data = balloon_data(0, 2, "Title", "Body");
        assert_eq!(data.uFlags, NIF_INFO | NIF_SHOWTIP);
        // SAFETY: the timeout is what the balloon data sets.
        assert_eq!(unsafe { data.Anonymous.uTimeout }, 10_000);
        assert_eq!(data.dwInfoFlags, NIIF_NONE);
        assert_eq!(data.uID, 2);
        assert_eq!(String::from_utf16_lossy(&field(&data.szInfoTitle)), "Title");
        assert_eq!(String::from_utf16_lossy(&field(&data.szInfo)), "Body");
    }

    #[test]
    fn a_title_is_cut_at_63_units() {
        let title: [u16; 64] = units(&"a".repeat(100));
        assert_eq!(field(&title).len(), 63);
        assert_eq!(title[63], 0);
        // 62 units, then a pair that would end at 64: it goes whole.
        let cut: [u16; 64] = units(&format!("{}\u{1F600}", "a".repeat(62)));
        assert_eq!(field(&cut).len(), 62);
        // 61 units, then a pair that ends exactly at 63: kept.
        let kept: [u16; 64] = units(&format!("{}\u{1F600}b", "a".repeat(61)));
        assert_eq!(field(&kept).len(), 63);
    }

    #[test]
    fn a_text_is_cut_at_255_units() {
        let text: [u16; 256] = units(&"a".repeat(400));
        assert_eq!(field(&text).len(), 255);
        assert_eq!(text[255], 0);
        let cut: [u16; 256] = units(&format!("{}\u{1F600}", "a".repeat(254)));
        assert_eq!(field(&cut).len(), 254);
        let kept: [u16; 256] = units(&format!("{}\u{1F600}b", "a".repeat(253)));
        assert_eq!(field(&kept).len(), 255);
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
