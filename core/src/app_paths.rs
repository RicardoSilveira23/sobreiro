//! Where Sobreiro keeps its files, and the name that tells one running copy from another.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The environment variable that moves the data folder: for tests, and for a second copy that
/// must not touch the first one's files.
pub const DATA_DIR_VARIABLE: &str = "SOBREIRO_DATA_DIR";

/// The data folder and the files in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// The override, when it is set and is a full path.
    overridden: Option<PathBuf>,
    default_directory: PathBuf,
    legacy_directory: PathBuf,
}

impl AppPaths {
    /// `overridden` is the value of [`DATA_DIR_VARIABLE`], when it is set. `local_app_data` is
    /// the user's local application data folder, which the exe asks Windows for: Core makes no
    /// call of its own.
    ///
    /// The data folder is the override when it is a full path (see [`is_full_path`]), and
    /// otherwise `Sobreiro` in the local application data folder.
    pub fn new(overridden: Option<&OsStr>, local_app_data: &Path) -> AppPaths {
        AppPaths {
            overridden: overridden
                .filter(|value| is_full_path(value))
                .map(PathBuf::from),
            default_directory: local_app_data.join("Sobreiro"),
            legacy_directory: local_app_data.join("OpenHeadroom"),
        }
    }

    /// `Sobreiro` in the local application data folder, whatever the override says.
    pub fn default_directory(&self) -> &Path {
        &self.default_directory
    }

    /// `OpenHeadroom` in the local application data folder, where the app kept its data
    /// before 0.5.0.
    pub fn legacy_directory(&self) -> &Path {
        &self.legacy_directory
    }

    /// True when the data folder is the default one: no override, or one that names it.
    pub fn is_default_data_directory(&self) -> bool {
        match &self.overridden {
            None => true,
            Some(dir) => normalise(dir) == normalise(&self.default_directory),
        }
    }

    /// The override as it was written, or the default folder.
    pub fn data_directory(&self) -> &Path {
        self.overridden
            .as_deref()
            .unwrap_or(&self.default_directory)
    }

    /// The bridge as installed: the one the status line runs.
    pub fn bridge_file(&self) -> PathBuf {
        self.data_directory().join("sobreiro-bridge.exe")
    }

    /// The limits as last read: written by the bridge and by Live usage, read by the tray.
    pub fn usage_file(&self) -> PathBuf {
        self.data_directory().join("usage.json")
    }

    /// The settings, among them the user's previous status line command.
    pub fn config_file(&self) -> PathBuf {
        self.data_directory().join("config.json")
    }

    /// Which alerts were already shown.
    pub fn alerts_file(&self) -> PathBuf {
        self.data_directory().join("alerts.json")
    }

    /// The log.
    pub fn log_file(&self) -> PathBuf {
        self.data_directory().join("log.txt")
    }

    /// The name by which a running copy finds out that another one runs already.
    ///
    /// `Sobreiro` normally, and when the override names the default folder. With any other
    /// override, `Sobreiro.` and eight digits worked out from the folder: a copy on a folder
    /// of its own gets a name of its own, and two copies never share one folder.
    pub fn instance_id(&self) -> String {
        let Some(dir) = &self.overridden else {
            return "Sobreiro".to_string();
        };
        let key = normalise(dir);
        if key == normalise(&self.default_directory) {
            return "Sobreiro".to_string();
        }
        // SHA-256 turns any text into 32 bytes that look random but are always the same for
        // the same text. The first four, written in hexadecimal, are the eight digits.
        let hash = Sha256::digest(key.as_bytes());
        let mut id = String::from("Sobreiro.");
        for byte in hash.iter().take(4) {
            id.push_str(&format!("{byte:02x}"));
        }
        id
    }
}

/// True when `value` says where a folder is all by itself, as 1.x judges it: it starts with a
/// drive, a colon and a separator (`C:\`), with two separators (`\\server\share`), or with a
/// separator and a question mark. Anything else would depend on the folder the program
/// happens to run in, and is ignored.
///
/// This asks less than the standard library's `Path::is_absolute`, which wants a share after
/// the computer's name. An override that 1.x takes must be taken here too: ignoring it would
/// send a copy that was told to keep to itself into the user's own data.
pub(crate) fn is_full_path(value: &OsStr) -> bool {
    let text = value.to_string_lossy();
    let mut start = text.chars();
    let (first, second, third) = (start.next(), start.next(), start.next());
    let is_separator = |character: Option<char>| matches!(character, Some('\\' | '/'));
    if is_separator(first) {
        return is_separator(second) || second == Some('?');
    }
    first.is_some_and(|letter| letter.is_ascii_alphabetic())
        && second == Some(':')
        && is_separator(third)
}

/// A folder's name as one text, the same however the folder was spelled: the full path as
/// Windows reads it, without the separator at its end, in capitals.
fn normalise(dir: &Path) -> String {
    // Windows' own reading: `/` becomes `\`, `.` and `..` are worked out, a doubled separator
    // becomes one, and dots and spaces at the very end go. The standard library puts this
    // question to Windows; for a full path the answer depends on the text alone, not on the
    // disk or on the folder the program runs in. A path that cannot be read this way stays
    // as it was written.
    let full = std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
    let full = full.to_string_lossy();
    // One separator at the end goes, unless it is the one after a drive's colon: `C:\` is
    // the drive's own folder, and `C:` would be another thing.
    let trimmed = match full.strip_suffix(['\\', '/']) {
        Some(shorter) if !shorter.ends_with(':') => shorter,
        _ => &full,
    };
    trimmed.chars().map(capital).collect()
}

/// The capital of a letter, worked out the way 1.x does, so both give a folder the same name.
///
/// A letter whose capital takes more than one letter (`ß`, which becomes `SS`) stays as it
/// is, and so does the Turkish `ı`, an i without its dot. Two kinds of letter still come out
/// differently from 1.x: 27 letters of ancient Greek, to which 1.x gives a capital of one
/// letter that Rust does not offer, and letters too new for the Windows that 1.x runs on.
///
/// Connect compares the bridge's path with it too.
pub(crate) fn capital(letter: char) -> char {
    if letter == 'ı' {
        return letter;
    }
    let mut capitals = letter.to_uppercase();
    match (capitals.next(), capitals.next()) {
        (Some(one), None) => one,
        _ => letter,
    }
}
