//! How the user writes the time of day, as Windows has it set: the short time picture and the
//! words for morning and afternoon. Core's `ClockFormat` takes them as they are: Windows' picture
//! uses the same letters as .NET's patterns (`h`, `HH`, `mm`, `tt`) and the same quoting.
#![allow(unsafe_code)]

use sobreiro_core::display::ClockFormat;
use windows::Win32::Globalization::{
    GetLocaleInfoEx, LOCALE_S1159, LOCALE_S2359, LOCALE_SSHORTTIME,
};
use windows::core::PCWSTR;

/// The room asked for each setting: they are short.
const ROOM: usize = 80;

/// The clock as the user's settings write it now. Read on every call: the user may change it at
/// any time. A setting Windows will not give is the US one.
pub fn clock_format() -> ClockFormat {
    ClockFormat {
        pattern: setting(LOCALE_SSHORTTIME).unwrap_or_else(|| "h:mm tt".to_string()),
        am: setting(LOCALE_S1159).unwrap_or_else(|| "AM".to_string()),
        pm: setting(LOCALE_S2359).unwrap_or_else(|| "PM".to_string()),
    }
}

/// One of the user's locale settings as text, or none when it is not there or is empty.
fn setting(kind: u32) -> Option<String> {
    let mut buffer = [0u16; ROOM];
    // SAFETY: `buffer` is ours to write, and its length is the room the call is told of. A null
    // name means the user's own locale.
    let length = unsafe { GetLocaleInfoEx(PCWSTR::null(), kind, Some(&mut buffer)) };
    // The length counts the ending zero.
    let length = usize::try_from(length).ok()?.checked_sub(1)?;
    let text = String::from_utf16(buffer.get(..length)?).ok()?;
    (!text.is_empty()).then_some(text)
}
