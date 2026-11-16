//! A watch on one folder for writes to one file name, with Windows' directory change reads. The
//! wait also ends at once when a stop event is set, so the thread that waits can be joined.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::path::Path;
use std::time::Duration;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_NOTIFY_ENUM_DIR, GetLastError, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForMultipleObjects};
use windows::core::HSTRING;

/// The record's action codes this watch looks at: a file added, changed, or a name it was
/// renamed to. Core's atomic write ends in a rename.
const ADDED: u32 = 1;
const MODIFIED: u32 = 3;
const RENAMED_TO: u32 = 5;

/// 64 KiB, in 32-bit words: the read wants a buffer aligned to four bytes.
const BUFFER_WORDS: usize = 16 * 1024;

/// An event that ends the wait of a watch. It may be set from any thread.
pub struct StopEvent(HANDLE);

// SAFETY: an event handle may be used from any thread; Windows does the locking.
unsafe impl Send for StopEvent {}
// SAFETY: as above; setting and waiting on an event need no outside lock.
unsafe impl Sync for StopEvent {}

impl StopEvent {
    /// A manual-reset event, not set. None when Windows refuses.
    pub fn new() -> Option<StopEvent> {
        // SAFETY: no attributes and no name; the handle is ours to close.
        unsafe { CreateEventW(None, true, false, None) }
            .ok()
            .map(StopEvent)
    }

    /// Sets the event, and it stays set. False when Windows refuses.
    pub fn set(&self) -> bool {
        // SAFETY: the handle is open for as long as `self` lives.
        unsafe { SetEvent(self.0) }.is_ok()
    }
}

impl Drop for StopEvent {
    fn drop(&mut self) {
        // SAFETY: the handle is closed once, here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// What one wait came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// The file was written, added or renamed to, or the changes were too many to be listed.
    Changed,
    /// Something else in the folder changed.
    Other,
    /// The time ran out.
    Timeout,
    /// The stop event was set.
    Stopped,
    /// The watch cannot go on.
    Failed,
}

/// The open folder, and the read that is pending on it.
pub struct DirWatch {
    dir: HANDLE,
    io_event: HANDLE,
    // Boxed and kept: the pending read holds its address.
    overlapped: Box<OVERLAPPED>,
    buffer: Vec<u32>,
    name: String,
    pending: bool,
}

impl DirWatch {
    /// Opens the folder to watch for `name`. None when it cannot be opened.
    pub fn open(dir: &Path, name: &str) -> Option<DirWatch> {
        let path = HSTRING::from(dir.as_os_str());
        // SAFETY: the path outlives the call; no attributes or template; the handle is closed
        // in `drop`.
        let dir = unsafe {
            CreateFileW(
                &path,
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
                None,
            )
        }
        .ok()?;
        // SAFETY: no attributes and no name; the handle is closed in `drop`.
        let Ok(io_event) = (unsafe { CreateEventW(None, true, false, None) }) else {
            // SAFETY: the folder's handle is ours and closed once, here.
            let _ = unsafe { CloseHandle(dir) };
            return None;
        };
        let overlapped = Box::new(OVERLAPPED {
            hEvent: io_event,
            ..OVERLAPPED::default()
        });
        Some(DirWatch {
            dir,
            io_event,
            overlapped,
            buffer: vec![0; BUFFER_WORDS],
            name: name.to_string(),
            pending: false,
        })
    }

    /// Waits for a change in the folder, the stop event, or the time to run out (no limit for
    /// None). One call reports one batch of changes; the next read is already pending when it
    /// returns.
    pub fn wait(&mut self, stop: &StopEvent, timeout: Option<Duration>) -> Event {
        if !self.pending && !self.read() {
            return Event::Failed;
        }
        let millis = timeout.map_or(u32::MAX, |time| {
            // Rounded up, and short of the "no limit" value.
            u32::try_from(time.as_millis().saturating_add(1))
                .unwrap_or(u32::MAX - 1)
                .min(u32::MAX - 1)
        });
        // The stop event first: the wait names the first of the two that is set, so a folder that
        // never stops changing cannot keep the stop from being seen.
        // SAFETY: both handles are open for as long as `self` and `stop` live.
        let waited = unsafe { WaitForMultipleObjects(&[stop.0, self.io_event], false, millis) };
        if waited == WAIT_TIMEOUT {
            return Event::Timeout;
        }
        if waited == WAIT_OBJECT_0 {
            return Event::Stopped;
        }
        if waited.0 != WAIT_OBJECT_0.0 + 1 {
            return Event::Failed;
        }
        self.pending = false;
        let mut bytes = 0u32;
        // SAFETY: the read on this handle and overlapped is complete; `bytes` is ours.
        let done = unsafe { GetOverlappedResult(self.dir, &*self.overlapped, &mut bytes, false) };
        let changed = match done {
            Ok(()) if bytes == 0 => true, // more changes than the buffer holds
            Ok(()) => matches(&self.buffer, bytes as usize, &self.name),
            // SAFETY: read straight after the call, before anything else can change it.
            Err(_) if unsafe { GetLastError() } == ERROR_NOTIFY_ENUM_DIR => true,
            Err(_) => return Event::Failed,
        };
        // Again at once, so the gap where changes go unseen stays short.
        if !self.read() {
            return Event::Failed;
        }
        if changed {
            Event::Changed
        } else {
            Event::Other
        }
    }

    /// Starts a read. False when Windows refuses.
    fn read(&mut self) -> bool {
        let size = u32::try_from(self.buffer.len() * 4).unwrap_or(u32::MAX);
        // SAFETY: the buffer and the overlapped are owned by `self` and stay where they are until
        // the read is done or cancelled in `drop`; no completion routine.
        let started = unsafe {
            ReadDirectoryChangesW(
                self.dir,
                self.buffer.as_mut_ptr().cast::<c_void>(),
                size,
                false,
                FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_SIZE,
                None,
                Some(&mut *self.overlapped),
                None,
            )
        };
        self.pending = started.is_ok();
        self.pending
    }
}

impl Drop for DirWatch {
    fn drop(&mut self) {
        if self.pending {
            let mut bytes = 0u32;
            // SAFETY: the read is cancelled and waited out before the buffer and the overlapped
            // are freed; the wait ends when the cancelled read is reported.
            unsafe {
                let _ = CancelIoEx(self.dir, Some(&*self.overlapped));
                let _ = GetOverlappedResult(self.dir, &*self.overlapped, &mut bytes, true);
            }
        }
        // SAFETY: both handles are ours and closed once, here.
        unsafe {
            let _ = CloseHandle(self.io_event);
            let _ = CloseHandle(self.dir);
        }
    }
}

/// Whether the records in the first `bytes` of the buffer add, change or rename to `name`
/// (any letter case). The records are 32-bit words: next offset, action, name length in bytes,
/// then the name in UTF-16.
fn matches(words: &[u32], bytes: usize, name: &str) -> bool {
    let limit = (bytes / 4).min(words.len());
    let mut at = 0usize;
    loop {
        let (Some(&next), Some(&action), Some(&length)) =
            (words.get(at), words.get(at + 1), words.get(at + 2))
        else {
            return false;
        };
        let units = (length as usize) / 2;
        let start = (at + 3) * 2;
        let mut text: Vec<u16> = Vec::with_capacity(units);
        for unit in start..start + units {
            let Some(&word) = words.get(unit / 2) else {
                return false;
            };
            // Little-endian: the first unit of a word is its low half.
            text.push(if unit % 2 == 0 {
                (word & 0xFFFF) as u16
            } else {
                (word >> 16) as u16
            });
        }
        if matches!(action, ADDED | MODIFIED | RENAMED_TO)
            && String::from_utf16_lossy(&text).eq_ignore_ascii_case(name)
        {
            return true;
        }
        // 0 ends the list; less than a record's three header words would never move on.
        let step = (next as usize) / 4;
        if step < 3 {
            return false;
        }
        at += step;
        if at >= limit {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One record as the words the read writes.
    fn record(action: u32, name: &str, last: bool) -> Vec<u32> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let mut words = vec![0, action, (units.len() * 2) as u32];
        for pair in units.chunks(2) {
            let high = pair.get(1).copied().unwrap_or(0);
            words.push(u32::from(pair[0]) | (u32::from(high) << 16));
        }
        if !last {
            words[0] = (words.len() * 4) as u32;
        }
        words
    }

    #[test]
    fn a_rename_to_the_name_matches_in_any_case() {
        let mut words = record(4, "usage.json.1f.tmp", false);
        words.extend(record(RENAMED_TO, "Usage.JSON", true));
        assert!(matches(&words, words.len() * 4, "usage.json"));
    }

    #[test]
    fn a_write_to_the_file_is_seen_and_the_stop_ends_the_wait() {
        let dir = std::env::temp_dir().join(format!("sobreiro-dir-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let stop = StopEvent::new().unwrap();
        let mut watch = DirWatch::open(&dir, "usage.json").unwrap();
        // The first wait starts the read; nothing has changed yet.
        let quiet = Some(Duration::from_millis(50));
        assert_eq!(watch.wait(&stop, quiet), Event::Timeout);
        std::fs::write(dir.join("log.txt"), "x").unwrap();
        std::fs::write(dir.join("usage.json"), "{}").unwrap();
        // The log's change may come in a batch of its own first.
        let mut seen = Event::Other;
        for _ in 0..10 {
            seen = watch.wait(&stop, Some(Duration::from_secs(5)));
            if seen == Event::Changed {
                break;
            }
        }
        assert_eq!(seen, Event::Changed);
        // A read is pending again; the stop ends the wait at once and the drop cancels the read.
        assert!(stop.set());
        assert_eq!(watch.wait(&stop, None), Event::Stopped);
        drop(watch);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_offset_shorter_than_a_record_ends_the_walk() {
        let mut words = record(MODIFIED, "sobreiro.log", false);
        words[0] = 2;
        words.extend(record(MODIFIED, "usage.json", true));
        assert!(!matches(&words, words.len() * 4, "usage.json"));
    }

    #[test]
    fn other_names_and_removals_do_not() {
        let mut words = record(MODIFIED, "sobreiro.log", false);
        words.extend(record(2, "usage.json", true));
        assert!(!matches(&words, words.len() * 4, "usage.json"));
    }
}
