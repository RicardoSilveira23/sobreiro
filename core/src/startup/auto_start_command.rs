//! The text of the Windows Run value that starts the app at sign-in.
use std::path::Path;

/// The name of the value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
pub const VALUE_NAME: &str = "Sobreiro";

/// The command for the exe at `exe_path`: the path between quotes.
pub fn command_for(exe_path: &Path) -> String {
    format!("\"{}\"", exe_path.to_string_lossy())
}

/// True when `value` runs exactly the exe at `exe_path`: white space and quotes around either
/// are left out, and the case of letters does not count.
pub fn matches(value: Option<&str>, exe_path: &Path) -> bool {
    let Some(value) = value else {
        return false;
    };
    let exe = exe_path.to_string_lossy();
    unquoted(value)
        .chars()
        .map(upper)
        .eq(unquoted(&exe).chars().map(upper))
}

fn unquoted(text: &str) -> &str {
    text.trim().trim_matches('"')
}

/// A letter's capital where it is one character, as an ordinal comparison that ignores case
/// takes it; any other character as it is.
fn upper(c: char) -> char {
    let mut capital = c.to_uppercase();
    match (capital.next(), capital.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}
