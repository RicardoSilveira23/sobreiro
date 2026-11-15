//! An append-only text log.
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The size past which the file is rolled, unless the caller says another.
pub const DEFAULT_MAX_BYTES: u64 = 262_144;

/// An append-only text log, UTF-8 with no byte order mark. When an entry would push a file
/// that is not empty past the cap, the file is moved to `log.1.txt` (replacing it) and a new
/// one starts. Nothing here ever panics.
///
/// Core looks at no clock and knows no process id, so there is no `write` that formats an
/// entry for now: the caller formats it with `log_entry::format` and appends it.
#[derive(Debug)]
pub struct DiagnosticsLog {
    path: PathBuf,
    previous_path: PathBuf,
    max_bytes: u64,
    gate: Mutex<()>,
}

impl DiagnosticsLog {
    pub fn new(path: &Path) -> DiagnosticsLog {
        DiagnosticsLog::with_max_bytes(path, DEFAULT_MAX_BYTES)
    }

    pub fn with_max_bytes(path: &Path, max_bytes: u64) -> DiagnosticsLog {
        let mut name = OsString::new();
        if let Some(stem) = path.file_stem() {
            name.push(stem);
        }
        name.push(".1");
        if let Some(extension) = path.extension() {
            name.push(".");
            name.push(extension);
        }
        DiagnosticsLog {
            path: path.to_path_buf(),
            previous_path: path.with_file_name(name),
            max_bytes,
            gate: Mutex::new(()),
        }
    }

    pub fn file_path(&self) -> &Path {
        &self.path
    }

    /// Where the file goes when it is rolled: `log.1.txt` beside `log.txt`.
    pub fn previous_file_path(&self) -> &Path {
        &self.previous_path
    }

    /// Adds the entry as given. Returns false if it could not be written.
    pub fn append(&self, entry: &str) -> bool {
        let bytes = entry.as_bytes();
        // A thread that panicked holding the gate left nothing half done that matters here.
        let _held = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(folder) = self.path.parent()
            && !folder.as_os_str().is_empty()
            && fs::create_dir_all(folder).is_err()
        {
            return false;
        }
        self.roll(bytes.len() as u64);
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.path)
            .and_then(|mut file| file.write_all(bytes))
            .is_ok()
    }

    fn roll(&self, incoming: u64) {
        // A failed roll must not stop the append.
        if let Ok(info) = fs::metadata(&self.path)
            && info.is_file()
            && info.len() > 0
            && info.len().saturating_add(incoming) > self.max_bytes
        {
            let _ = fs::rename(&self.path, &self.previous_path);
        }
    }
}
