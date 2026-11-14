//! Writes a file by writing a temporary file beside it and renaming that over the target, so a
//! reader sees the old file or the new one and never half of one.
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How often the rename is tried, and the wait between tries, when someone holds the target.
const ATTEMPTS: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(20);

// A running bridge holds its exe for about 60 ms; 10 x 100 ms rides that out.
const STREAM_ATTEMPTS: u32 = 10;
const STREAM_RETRY_DELAY: Duration = Duration::from_millis(100);

/// The three bytes that mark a file as UTF-8 for the programs that look for them.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Writes `text` to `path` as UTF-8, with nothing before it.
pub fn write_all_text(path: &Path, text: &str) -> io::Result<()> {
    replace(
        path,
        |temp| temp.write_all(text.as_bytes()),
        ATTEMPTS,
        RETRY_DELAY,
    )
}

/// Writes `text` to `path` as UTF-8, after a byte order mark.
pub fn write_all_text_with_bom(path: &Path, text: &str) -> io::Result<()> {
    replace(
        path,
        |temp| {
            temp.write_all(&BYTE_ORDER_MARK)?;
            temp.write_all(text.as_bytes())
        },
        ATTEMPTS,
        RETRY_DELAY,
    )
}

/// Writes everything `content` gives to `path`, retrying the rename for about a second.
pub fn write_from(path: &Path, content: &mut dyn Read) -> io::Result<()> {
    replace(
        path,
        |temp| io::copy(content, temp).map(|_bytes_copied| ()),
        STREAM_ATTEMPTS,
        STREAM_RETRY_DELAY,
    )
}

/// Makes the folder, writes the temporary file and renames it over `path`. Whatever fails, the
/// temporary file is removed and the first error is the one given back.
fn replace(
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> io::Result<()>,
    attempts: u32,
    delay: Duration,
) -> io::Result<()> {
    if let Some(folder) = std::path::absolute(path)?.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let temp = temp_beside(path);
    let result = write_then_rename(&temp, path, write_temp, attempts, delay);
    if result.is_err() {
        // Best effort: the error worth reporting is the one above.
        let _ = std::fs::remove_file(&temp);
    }
    result
}

fn write_then_rename(
    temp: &Path,
    path: &Path,
    write_temp: impl FnOnce(&mut File) -> io::Result<()>,
    attempts: u32,
    delay: Duration,
) -> io::Result<()> {
    {
        // `create_new` refuses a name that exists, so two writers can never share a temporary
        // file. The file closes at the end of this block, before the rename.
        let mut file = File::options().write(true).create_new(true).open(temp)?;
        write_temp(&mut file)?;
    }
    let mut attempt = 1;
    loop {
        // On Windows a rename over an existing file replaces it in one step.
        match std::fs::rename(temp, path) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= attempts => return Err(error),
            Err(_) => {
                std::thread::sleep(delay);
                attempt += 1;
            }
        }
    }
}

/// Counts the temporary files this process has named, so no two get the same name.
static NAMED_SO_FAR: AtomicU32 = AtomicU32::new(0);

/// `<path>.<24 hex digits>.tmp`, in the same folder as `path`. The digits are this process's
/// number, the clock's nanoseconds and a count, so neither two writes of one process nor two
/// processes at the same moment get the same name.
fn temp_beside(path: &Path) -> PathBuf {
    let process = std::process::id();
    let nanoseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since_1970| since_1970.subsec_nanos());
    let count = NAMED_SO_FAR.fetch_add(1, Ordering::Relaxed);

    let mut name = OsString::from(path);
    name.push(format!(".{process:08x}{nanoseconds:08x}{count:08x}.tmp"));
    PathBuf::from(name)
}
