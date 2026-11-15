//! Puts the bridge, which ships inside Sobreiro's exe, into the data folder where the status
//! line runs it.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use super::bridge_command::runs;
use super::settings_json::{command_of, read_status_line};
use crate::io::atomic_file;

/// The name of the bridge inside Sobreiro's exe, and of the copy in the data folder.
pub const RESOURCE_NAME: &str = "sobreiro-bridge.exe";

/// Writes the bridge to `path` unless the file there already holds the same bytes. Gives
/// `true` when it wrote.
///
/// `open_source` opens the bridge to install, from its first byte. It is called twice: once
/// to compare and once to write.
///
/// The write waits out a bridge that is running, for about a second. When the file stays
/// held, an error comes back, the bridge that was there is untouched and no temporary file
/// is left beside it.
pub fn ensure<Source: Read>(
    open_source: impl Fn() -> io::Result<Source>,
    path: &Path,
) -> io::Result<bool> {
    if same_bytes(&mut open_source()?, path)? {
        return Ok(false);
    }
    atomic_file::write_from(path, &mut open_source()?)?;
    Ok(true)
}

/// True when the status line in `settings_text` runs `installed_path`: only then is the copy
/// in the data folder in use, and worth bringing up to date.
pub fn should_refresh(settings_text: &str, installed_path: &Path) -> bool {
    match read_status_line(settings_text) {
        Ok(status_line) => runs(command_of(status_line).as_deref(), installed_path),
        Err(_) => false,
    }
}

/// True when the file at `path` is there and holds exactly what `source` gives.
fn same_bytes(source: &mut impl Read, path: &Path) -> io::Result<bool> {
    if !path.is_file() {
        return Ok(false);
    }
    // Opened so that others may read, write and rename the file meanwhile, which is how the
    // standard library opens a file on Windows: a bridge that is running does not stop the
    // comparison.
    let mut existing = File::open(path)?;

    let mut wanted = [0u8; 8192];
    let mut found = [0u8; 8192];
    loop {
        let wanted_count = fill(source, &mut wanted)?;
        let found_count = fill(&mut existing, &mut found)?;
        if wanted.get(..wanted_count) != found.get(..found_count) {
            return Ok(false);
        }
        if wanted_count == 0 {
            return Ok(true);
        }
    }
}

/// Reads until `buffer` is full or `reader` has no more to give, and says how many bytes
/// were read. One call to `read` may give fewer bytes than there is room for, so two readers
/// can only be compared piece by piece once each piece is as long as it can be.
fn fill(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut count = 0;
    while let Some(room) = buffer.get_mut(count..)
        && !room.is_empty()
    {
        match reader.read(room) {
            Ok(0) => break,
            Ok(read) => count += read,
            // The read was cut short by a signal, with nothing lost: it is tried again.
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(count)
}
