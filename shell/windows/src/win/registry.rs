//! The registry values 1.1's `WindowsTheme` reads, read the same way: the taskbar's theme, whether
//! it takes the accent, and the accent palette. Read only; a value that is missing or of another
//! kind is none.
#![allow(unsafe_code)]

use crate::theme::Theme;
use sobreiro_core::colours::AccentPalette;
use std::ffi::c_void;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD, RegGetValueW,
};
use windows::core::{PCWSTR, w};

const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
const ACCENT: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent");

/// The room asked for the palette: it holds 32 bytes.
const PALETTE_ROOM: usize = 64;

/// The theme now. Read on every call: the user may change it at any time.
pub fn theme() -> Theme {
    let flag = |name: PCWSTR| dword(PERSONALIZE, name).is_some_and(|value| value != 0);
    let palette = binary(ACCENT, w!("AccentPalette"));
    Theme {
        light: flag(w!("SystemUsesLightTheme")),
        accent_taskbar: flag(w!("ColorPrevalence")),
        accent: AccentPalette::from_registry(palette.as_deref()),
    }
}

fn dword(subkey: PCWSTR, name: PCWSTR) -> Option<u32> {
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `value` and `size` are alive for the call, and `size` is the room `value` has.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            name,
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut value).cast::<c_void>()),
            Some(&raw mut size),
        )
    };
    (status.0 == 0).then_some(value)
}

fn binary(subkey: PCWSTR, name: PCWSTR) -> Option<Vec<u8>> {
    let mut bytes = vec![0u8; PALETTE_ROOM];
    let mut size = PALETTE_ROOM as u32;
    // SAFETY: `bytes` has `size` bytes of room, and both are alive for the call.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey,
            name,
            RRF_RT_REG_BINARY,
            None,
            Some(bytes.as_mut_ptr().cast::<c_void>()),
            Some(&raw mut size),
        )
    };
    if status.0 != 0 {
        return None;
    }
    bytes.truncate(size as usize);
    Some(bytes)
}
