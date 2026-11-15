//! The three things the bridge asks of Windows that Rust's standard library has no way to ask:
//! to end a process together with every process it started, to wait for a process no longer
//! than a given time, and to say where the user's local application data folder is.
//!
//! Rust cannot check what happens on the other side of such a call, so each one is marked
//! `unsafe` and says beside it why it is sound. The workspace denies unsafe code; this file is
//! the one place in the bridge that is let to hold it.
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::WaitForSingleObject;
use windows_sys::Win32::UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath};

/// A process and every process it starts from now on, so that all of them can be ended in one
/// go. Windows calls this a job.
///
/// A process the child started before it was put in the job is not in it. The bridge puts the
/// child in right after starting it, long before a shell has read its first command.
pub struct ProcessFamily {
    /// The job, or null when Windows would not make one or would not take the child into it.
    /// Then only the child itself can be ended.
    job: HANDLE,
}

impl ProcessFamily {
    pub fn of(child: &Child) -> ProcessFamily {
        // SAFETY: both arguments may be null, meaning default security and no name.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return ProcessFamily { job };
        }
        // SAFETY: `job` was made just above, and the child's handle stays open while `child`
        // is alive, which it is for the length of this call.
        let assigned = unsafe { AssignProcessToJobObject(job, child.as_raw_handle()) } != 0;
        if !assigned {
            // SAFETY: `job` is a handle this function opened and has given to no one.
            unsafe { CloseHandle(job) };
            return ProcessFamily {
                job: std::ptr::null_mut(),
            };
        }
        ProcessFamily { job }
    }

    /// Ends the child and all it started. Without a job, ends the child alone.
    pub fn end(&self, child: &mut Child) {
        if !self.job.is_null() {
            // SAFETY: `job` is open until this value is dropped.
            let ended = unsafe { TerminateJobObject(self.job, 1) } != 0;
            if ended {
                return;
            }
        }
        // It may have ended by itself in the meantime; then there is nothing left to end.
        let _ = child.kill();
    }
}

impl Drop for ProcessFamily {
    /// Lets go of the job. The processes in it carry on: only `end` stops them.
    fn drop(&mut self) {
        if !self.job.is_null() {
            // SAFETY: `job` is a handle this value opened, and it is closed only here.
            unsafe { CloseHandle(self.job) };
        }
    }
}

/// Waits until the child has ended, but no longer than `timeout`. True when it has ended.
pub fn wait_for_exit(child: &Child, timeout: Duration) -> bool {
    // The largest number means "for ever" to Windows, so stop one short of it (49 days).
    let milliseconds = u32::try_from(timeout.as_millis())
        .unwrap_or(u32::MAX)
        .min(u32::MAX - 1);
    // SAFETY: the child's handle stays open while `child` is alive, and waiting changes nothing.
    let outcome = unsafe { WaitForSingleObject(child.as_raw_handle(), milliseconds) };
    outcome == WAIT_OBJECT_0
}

/// The user's local application data folder (`C:\Users\<name>\AppData\Local`), asked of Windows
/// the way 1.x asked: not read from the environment, which whoever started the bridge may have
/// changed. `None` when Windows cannot say.
pub fn local_app_data() -> Option<PathBuf> {
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: the folder's identifier is a constant of Windows', 0 asks for no special
    // treatment, a null token means the current user, and `text` is a valid place for the
    // answer.
    let result =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut text) };

    let succeeded = result >= 0 && !text.is_null();
    let folder = succeeded.then(|| {
        // SAFETY: on success `text` points at 16-bit characters that end with a zero. The loop
        // stops at that zero, and the slice covers exactly the characters before it.
        let characters = unsafe {
            let mut length = 0;
            while *text.add(length) != 0 {
                length += 1;
            }
            std::slice::from_raw_parts(text, length)
        };
        PathBuf::from(OsString::from_wide(characters))
    });

    // SAFETY: the text is Windows' to free, with this call, after a success or a failure
    // alike. Null is allowed. Nothing reads `text` after this line.
    unsafe { CoTaskMemFree(text.cast()) };
    folder
}

#[cfg(test)]
mod tests {
    use super::local_app_data;

    /// Only asks where the folder is; nothing in it is read or written.
    #[test]
    fn windows_says_where_the_local_application_data_folder_is() {
        let folder = local_app_data().unwrap();

        assert!(folder.is_absolute(), "{folder:?}");
        assert!(folder.is_dir(), "{folder:?}");
        // On a PC whose environment nobody has changed, the two agree.
        if let Some(from_environment) = std::env::var_os("LOCALAPPDATA") {
            assert_eq!(
                folder.to_string_lossy().to_uppercase(),
                from_environment.to_string_lossy().to_uppercase()
            );
        }
    }
}
