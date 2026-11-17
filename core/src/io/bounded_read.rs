//! Reading that stops at a limit. Files and pipes that Sobreiro reads are written by other
//! programs, and Sobreiro does not load whatever size it finds.
use std::io::{self, Read};
use std::path::Path;

/// The most that Sobreiro reads of a settings, config, usage or alerts file, of stdin and of
/// a chained command's output. Real ones are a few kilobytes.
pub const MAX_BYTES: u64 = 1 << 20;

/// All of `reader`, as long as it is no more than `max` bytes. More than that is an error of
/// kind `InvalidData`, and no more than `max` plus one byte is ever held in memory.
pub fn read_at_most(reader: impl Read, max: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    // One byte past the limit is enough to tell that there was more.
    reader.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "more than the most that is read",
        ));
    }
    Ok(bytes)
}

/// All of the file at `path`, up to [`MAX_BYTES`]. A larger file is an error of kind
/// `InvalidData`, like a file that is not what it should be.
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    read_at_most(std::fs::File::open(path)?, MAX_BYTES)
}
