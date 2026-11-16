//! The system's own colours that the shell reads: the tooltip's, which 1.1's WPF tooltip takes
//! from Windows as they are set.
#![allow(unsafe_code)]

use sobreiro_core::colours::Rgb;
use windows::Win32::Graphics::Gdi::{COLOR_INFOBK, COLOR_INFOTEXT, GetSysColor, SYS_COLOR_INDEX};

/// The tooltip's ground and its text, as Windows has them set now.
pub fn tooltip() -> (Rgb, Rgb) {
    (colour(COLOR_INFOBK), colour(COLOR_INFOTEXT))
}

fn colour(index: SYS_COLOR_INDEX) -> Rgb {
    // SAFETY: asks Windows for one of its colours; an index it does not know gives 0.
    let value = unsafe { GetSysColor(index) };
    // A COLORREF is 0x00BBGGRR.
    Rgb {
        r: (value & 0xFF) as u8,
        g: (value >> 8 & 0xFF) as u8,
        b: (value >> 16 & 0xFF) as u8,
    }
}
