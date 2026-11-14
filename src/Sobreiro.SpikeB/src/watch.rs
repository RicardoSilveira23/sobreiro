//! SPIKE (shell candidate B): tells the shell when a file in the data folder is written, renamed or made.
//! Rust's standard library has no file watcher: a thread waits on the system's change notice for the folder.
use windows::Win32::Foundation::{INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FindFirstChangeNotificationW,
    FindNextChangeNotification,
};
use windows::Win32::System::Threading::{INFINITE, WaitForSingleObject};
use windows::core::HSTRING;

/// Calls `changed` on a thread of its own after each change in `folder` (any file: the caller reads again
/// and compares). The thread lives as long as the app.
pub fn folder(folder: &str, changed: impl Fn() + Send + 'static) {
    let folder = HSTRING::from(folder);
    let _ = std::thread::Builder::new().name("data folder watch".into()).spawn(move || unsafe {
        let filter = FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_SIZE;
        let Ok(handle) = FindFirstChangeNotificationW(&folder, false, filter) else { return };
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
        loop {
            if WaitForSingleObject(handle, INFINITE) != WAIT_OBJECT_0 {
                return;
            }
            changed();
            if FindNextChangeNotification(handle).is_err() {
                return;
            }
        }
    });
}
