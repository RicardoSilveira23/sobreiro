//! The tray app's registry reads for the Windows theme (src/Sobreiro/WindowsTheme.cs).

use crate::colours::AccentPalette;
use std::ffi::c_void;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD, RegGetValueW};
use windows::core::{PCWSTR, w};

const PERSONALIZE: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
const ACCENT: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent");

fn read_dword(subkey: PCWSTR, name: PCWSTR) -> Option<u32> {
    let mut data: u32 = 0;
    let mut size: u32 = 4;
    // SAFETY: data and size are live locals; the sizes match.
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, subkey, name, RRF_RT_REG_DWORD, None, Some(&mut data as *mut u32 as *mut c_void), Some(&mut size))
    };
    r.is_ok().then_some(data)
}

fn read_binary(subkey: PCWSTR, name: PCWSTR) -> Option<Vec<u8>> {
    let mut size: u32 = 0;
    // SAFETY: the first call only asks for the size.
    let r = unsafe { RegGetValueW(HKEY_CURRENT_USER, subkey, name, RRF_RT_REG_BINARY, None, None, Some(&mut size)) };
    if !r.is_ok() || size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    // SAFETY: buf holds `size` bytes.
    let r = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, subkey, name, RRF_RT_REG_BINARY, None, Some(buf.as_mut_ptr() as *mut c_void), Some(&mut size))
    };
    if !r.is_ok() {
        return None;
    }
    buf.truncate(size as usize);
    Some(buf)
}

pub struct WindowsTheme;

impl WindowsTheme {
    pub fn is_light() -> bool {
        read_dword(PERSONALIZE, w!("SystemUsesLightTheme")).is_some_and(|v| v != 0)
    }

    /// True when the taskbar takes the accent colour (Personalize's ColorPrevalence).
    pub fn accent_taskbar() -> bool {
        read_dword(PERSONALIZE, w!("ColorPrevalence")).is_some_and(|v| v != 0)
    }

    /// The Windows accent palette, or None when the registry holds none.
    pub fn accent() -> Option<AccentPalette> {
        AccentPalette::from_registry(read_binary(ACCENT, w!("AccentPalette")).as_deref())
    }
}
