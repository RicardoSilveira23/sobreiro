//! The menu's Open data folder and About Sobreiro. Open data folder makes the folder and shows it
//! in Explorer. About asks in a message box whether to read the licences, and the answer Yes opens
//! the licence window (`win/licence_window.rs`) on the same thread. The texts are made here, as
//! plain functions, so a test can hold them to 1.1's words.
use crate::licences;
use crate::providers::Locations;
use crate::win::{known_folders, licence_window, message_box};
use sobreiro_core::live::live_usage_message::ABOUT_LINE;
use std::path::Path;
use std::process::Command;

/// The title of the About box.
const TITLE: &str = "About Sobreiro";

/// What the About box says, in 1.1's words with the licence lines this exe carries.
pub fn about_text(version: &str) -> String {
    format!(
        "Sobreiro {version}\n\n\
         Shows Claude Code's session and weekly limits and GitHub Copilot's monthly AI credits. {ABOUT_LINE}\n\n\
         Copyright 2026 Ricardo Silveira. Sobreiro's own code is free software under the GNU GPL v3 or later; this exe, with the libraries built into it, is under the GNU GPL v3. No warranty.\n\n\
         Not affiliated with or endorsed by Anthropic or GitHub.\n\n\
         Fonts: Inter and Unbounded, SIL Open Font License.\n\n\
         Read the licences?"
    )
}

/// The licence window's text: each text with its line ends made CRLF, a blank line between them.
pub fn joined_licences(texts: &[&str]) -> String {
    texts
        .iter()
        .map(|text| crlf(text))
        .collect::<Vec<_>>()
        .join("\r\n\r\n")
}

/// The text with every line ending made CRLF, whether it had LF, CRLF or a lone CR.
fn crlf(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\n', "\r\n")
}

/// Sobreiro's licence, the two fonts' and the libraries' notices, in that order.
fn licence_text() -> String {
    joined_licences(&[
        licences::LICENCE,
        licences::INTER,
        licences::UNBOUNDED,
        licences::NOTICES,
    ])
}

/// Shows the About box on a thread of its own: the toolkit's event loop goes on behind it, and
/// `born`, which hooks the UI thread only, never takes it for a window it waits for. Yes opens
/// the licence window on that thread. When no thread can start, nothing is shown.
pub fn show_about() {
    let _ = std::thread::Builder::new().name("about".into()).spawn(|| {
        let text = about_text(env!("CARGO_PKG_VERSION"));
        if message_box::ask_default_no(TITLE, &text) {
            licence_window::show(&licence_text());
        }
    });
}

/// Explorer, by its full path in the Windows folder, with the folder as its one argument. Never
/// by bare name: Windows would look in the exe's own folder first, and that may be Downloads.
fn explorer_command(windows: &Path, folder: &Path) -> Command {
    let mut command = Command::new(windows.join("explorer.exe"));
    command.arg(folder);
    command
}

/// Makes the data folder when it is missing and opens it in Explorer, without waiting for it.
/// False when any step failed; the caller logs one fixed line.
pub fn open_data_folder() -> bool {
    let Some(places) = Locations::find() else {
        return false;
    };
    let Some(windows) = known_folders::windows() else {
        return false;
    };
    let folder = places.app.data_directory();
    std::fs::create_dir_all(folder).is_ok() && explorer_command(&windows, folder).spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn the_about_text_is_exact() {
        let expected = format!(
            "Sobreiro 2.0.0\n\nShows Claude Code's session and weekly limits and GitHub Copilot's monthly AI credits. {ABOUT_LINE}\n\nCopyright 2026 Ricardo Silveira. Sobreiro's own code is free software under the GNU GPL v3 or later; this exe, with the libraries built into it, is under the GNU GPL v3. No warranty.\n\nNot affiliated with or endorsed by Anthropic or GitHub.\n\nFonts: Inter and Unbounded, SIL Open Font License.\n\nRead the licences?"
        );
        assert_eq!(about_text("2.0.0"), expected);
        assert!(about_text("1").contains("makes no other network calls."));
    }

    #[test]
    fn the_licences_are_joined_with_blank_lines_and_crlf_everywhere() {
        let joined = joined_licences(&["a\nb", "c\r\nd\r", "e"]);
        assert_eq!(joined, "a\r\nb\r\n\r\nc\r\nd\r\n\r\n\r\ne");
        assert_eq!(joined_licences(&[]), "");
        let mut bare = joined.replace("\r\n", "");
        bare.retain(|c| c == '\n' || c == '\r');
        assert!(bare.is_empty());
    }

    #[test]
    fn the_carried_texts_come_in_order_with_crlf_only() {
        let text = licence_text();
        let at = |part: &str| text.find(part.trim().lines().next().unwrap_or(""));
        let (a, b, c, d) = (
            at(licences::LICENCE),
            at(licences::INTER),
            at(licences::UNBOUNDED),
            at(licences::NOTICES),
        );
        assert!(a.is_some() && a < b && b < c && c < d);
        assert!(!text.replace("\r\n", "").contains(['\r', '\n']));
    }

    #[test]
    fn explorer_is_started_by_its_full_path_with_the_folder_as_its_one_argument() {
        let windows = PathBuf::from(r"C:\Windows");
        let folder = PathBuf::from(r"C:\Data\Sobreiro");
        let command = explorer_command(&windows, &folder);
        assert_eq!(
            command.get_program(),
            OsStr::new(r"C:\Windows\explorer.exe")
        );
        assert!(Path::new(command.get_program()).is_absolute());
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [folder.as_os_str()]);
    }
}
