use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Writes a file by writing a temp file beside it and renaming it over the target.
pub struct AtomicFile;

const ATTEMPTS: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_millis(20);

// A running bridge holds its exe for about 60 ms; 10 x 100 ms rides that out.
const STREAM_ATTEMPTS: u32 = 10;
const STREAM_RETRY_DELAY: Duration = Duration::from_millis(100);

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl AtomicFile {
    pub fn write_all_text(path: &str, text: &str, utf8_bom: bool) -> io::Result<()> {
        replace(path, |temp| {
            let mut f = fs::File::create_new(temp)?;
            if utf8_bom {
                f.write_all(&[0xEF, 0xBB, 0xBF])?;
            }
            f.write_all(text.as_bytes())
        }, ATTEMPTS, RETRY_DELAY)
    }

    /// Writes `content` to `path`, retrying the rename for about a second.
    pub fn write(path: &str, content: &mut dyn Read) -> io::Result<()> {
        replace(path, |temp| {
            let mut f = fs::File::create_new(temp)?;
            io::copy(content, &mut f).map(|_| ())
        }, STREAM_ATTEMPTS, STREAM_RETRY_DELAY)
    }
}

fn replace(path: &str, write_temp: impl FnOnce(&str) -> io::Result<()>, attempts: u32, delay: Duration) -> io::Result<()> {
    let full = std::path::absolute(path)?;
    if let Some(dir) = full.parent() {
        fs::create_dir_all(dir)?;
    }

    // A unique suffix without a random-number crate: the clock, the process and a counter.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let unique = format!("{:x}{:x}{:x}", nanos, std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed));
    let temp = format!("{path}.{unique}.tmp");

    let result = (|| {
        write_temp(&temp)?;
        let mut attempt = 1;
        loop {
            match fs::rename(&temp, Path::new(path)) {
                Ok(()) => return Ok(()),
                Err(_) if attempt < attempts => {
                    attempt += 1;
                    std::thread::sleep(delay);
                }
                Err(e) => return Err(e),
            }
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
