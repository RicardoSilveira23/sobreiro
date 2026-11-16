//! Windows' own message boxes, with no owner window: a Yes/No question, a note, and an error.
//! Each waits for the answer, so it is called from a thread that may block.
#![allow(unsafe_code)]

use windows::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_YESNO, MESSAGEBOX_STYLE, MessageBoxW,
};
use windows::core::PCWSTR;

/// The title of an error box.
const ERROR_TITLE: &str = "Sobreiro";

/// Nul-ended UTF-16. A nul inside the text ends it there, so no box ever panics on one.
fn wide(text: &str) -> Vec<u16> {
    let mut units: Vec<u16> = text.encode_utf16().take_while(|unit| *unit != 0).collect();
    units.push(0);
    units
}

fn show(title: &str, text: &str, style: MESSAGEBOX_STYLE) -> bool {
    let title = wide(title);
    let text = wide(text);
    // SAFETY: both strings are nul-ended and alive for the call; no owner window is given.
    let answer = unsafe { MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), style) };
    answer == IDYES
}

/// Asks a Yes/No question, with no icon. True on Yes.
pub fn ask(title: &str, text: &str) -> bool {
    show(title, text, MB_YESNO | MB_SETFOREGROUND)
}

/// Says something, with no icon, and waits for OK.
pub fn tell(title: &str, text: &str) {
    show(title, text, MB_OK | MB_SETFOREGROUND);
}

/// Says what went wrong, under the title Sobreiro, and waits for OK.
pub fn error(text: &str) {
    show(ERROR_TITLE, text, MB_OK | MB_ICONERROR | MB_SETFOREGROUND);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_nul_ended_and_cut_at_a_nul() {
        assert_eq!(wide("ab"), vec![97, 98, 0]);
        assert_eq!(wide("a\0b"), vec![97, 0]);
        assert_eq!(wide(""), vec![0]);
    }
}
