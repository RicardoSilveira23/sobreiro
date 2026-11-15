//! `usage.json` on disk: the one file the bridge and the tray share.
use std::io;
use std::path::Path;

use crate::io::atomic_file;
use crate::usage::UsageSnapshot;
use crate::usage::rate_limits_json;

/// Writes the snapshot over `usage.json`, in one step, making the folder when it is missing.
pub fn write(path: &Path, snapshot: &UsageSnapshot) -> io::Result<()> {
    let json = rate_limits_json::write_usage_file(snapshot)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    atomic_file::write_all_text(path, &json)
}

/// Reads `usage.json`. `None` when it is missing, cannot be opened now, or does not parse.
pub fn read(path: &Path) -> Option<UsageSnapshot> {
    rate_limits_json::parse_usage_file(&read_text(path).ok()?)
}

/// What [`read_for_merge`] found.
#[derive(Debug, Clone, PartialEq)]
pub enum ForMerge {
    /// A merge may be written. Inside is the stored snapshot to merge into, or `None` when the
    /// file is missing or its content does not parse: both are safe to write over.
    SafeToWrite(Option<UsageSnapshot>),
    /// `usage.json` is there but cannot be read now (a writer holds it). Writing a merge now
    /// would drop what it holds.
    NotNow,
}

/// Reads the snapshot that a new reading is to be merged into.
pub fn read_for_merge(path: &Path) -> ForMerge {
    match read_text(path) {
        Ok(text) => ForMerge::SafeToWrite(rate_limits_json::parse_usage_file(&text)),
        // Missing a moment ago. Safe unless a file appeared since.
        Err(error) if error.kind() == io::ErrorKind::NotFound && !path.is_file() => {
            ForMerge::SafeToWrite(None)
        }
        Err(_) => ForMerge::NotNow,
    }
}

/// The three bytes that some programs put at the start of a UTF-8 file.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The file's text. Opening it does not stop anyone else from reading, writing or replacing it
/// meanwhile. Bytes that are not UTF-8 are read as the "unknown character" sign, not refused.
fn read_text(path: &Path) -> io::Result<String> {
    let bytes = std::fs::read(path)?;
    let text = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(text).into_owned())
}
