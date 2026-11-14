use super::{RateLimitsJson, UsageSnapshot};
use crate::io::AtomicFile;
use std::io::{self, Read};

pub struct UsageFile;

impl UsageFile {
    pub fn write(path: &str, snapshot: &UsageSnapshot) -> io::Result<()> {
        AtomicFile::write_all_text(path, &RateLimitsJson::write_usage_file(snapshot), false)
    }

    /// None when the file is missing, locked, or does not hold a usable snapshot.
    pub fn read(path: &str) -> Option<UsageSnapshot> {
        RateLimitsJson::parse_usage_file(&read_text(path).ok()?)
    }

    /// The snapshot to merge a new reading into. False when usage.json is there but can't be read now (a writer
    /// holds it): writing a merge then would drop what it holds. A missing file, or content that doesn't parse, is
    /// safe to write over and gives true with no snapshot.
    pub fn try_read_for_merge(path: &str, stored: &mut Option<UsageSnapshot>) -> bool {
        *stored = None;
        match read_text(path) {
            Ok(text) => {
                *stored = RateLimitsJson::parse_usage_file(&text);
                true
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => !std::path::Path::new(path).exists(),
            Err(_) => false,
        }
    }
}

/// Opened shared for reading, writing and deleting (std's default on Windows); a BOM is dropped.
fn read_text(path: &str) -> io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(body).into_owned())
}
