//! How the user writes the time of day, as Windows has it set: the short time picture and the
//! words for morning and afternoon. Windows' picture uses the same letters as .NET's patterns
//! (`h`, `HH`, `mm`, `tt`), but quotes text its own way, so it is turned into the pattern Core's
//! `ClockFormat` reads (`pattern_of`).
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
        pattern: setting(LOCALE_SSHORTTIME)
            .map(|picture| pattern_of(&picture))
            .unwrap_or_else(|| "h:mm tt".to_string()),
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

/// Windows' time picture as the pattern Core's `ClockFormat` reads. Windows writes a quote inside
/// quoted text as `''` (and a lone `''` outside quotes is a quote too), and a backslash is only a
/// backslash; Core reads `\'` for a quote and `\\` for a backslash.
fn pattern_of(picture: &str) -> String {
    let mut pattern = String::new();
    let mut quoted = false;
    let mut chars = picture.chars().peekable();
    while let Some(letter) = chars.next() {
        match letter {
            '\'' if chars.peek() == Some(&'\'') => {
                chars.next();
                pattern.push_str("\\'");
            }
            '\'' => {
                quoted = !quoted;
                pattern.push('\'');
            }
            '\\' => pattern.push_str("\\\\"),
            // Core starts quoted text at a double quote too; Windows' picture never does.
            '"' if !quoted => pattern.push_str("\\\""),
            other => pattern.push(other),
        }
    }
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use sobreiro_core::display::ClockFormat;

    fn written(picture: &str) -> String {
        let clock = ClockFormat {
            pattern: pattern_of(picture),
            am: "AM".to_string(),
            pm: "PM".to_string(),
        };
        clock.format(chrono::NaiveTime::from_hms_opt(14, 5, 0).unwrap_or_default())
    }

    #[test]
    fn the_usual_pictures_are_kept() {
        assert_eq!(pattern_of("h:mm tt"), "h:mm tt");
        assert_eq!(pattern_of("HH:mm"), "HH:mm");
        assert_eq!(written("h:mm tt"), "2:05 PM");
        assert_eq!(written("HH:mm"), "14:05");
    }

    #[test]
    fn a_quote_inside_quoted_text_becomes_an_escaped_quote() {
        assert_eq!(pattern_of("HH 'o''clock'"), "HH 'o\\'clock'");
        assert_eq!(written("HH 'o''clock'"), "14 o'clock");
        // Quoted text is copied as it is, letters included.
        assert_eq!(written("H 'h' mm"), "14 h 05");
    }

    #[test]
    fn a_quote_outside_quotes_is_a_quote() {
        assert_eq!(written("H''mm"), "14'05");
    }

    #[test]
    fn a_backslash_is_doubled() {
        assert_eq!(pattern_of("H\\mm"), "H\\\\mm");
        assert_eq!(written("H\\mm"), "14\\05");
        assert_eq!(written("H '\\' mm"), "14 \\ 05");
        assert_eq!(written("H\"mm"), "14\"05");
    }
}
