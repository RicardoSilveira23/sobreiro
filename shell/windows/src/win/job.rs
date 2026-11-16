//! A job object that ends every process in it when it is closed: what stops `gh`, and anything
//! `gh` started, when it does not answer in time.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::process::Child;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows::core::PCWSTR;

/// A job whose processes are ended when it is dropped.
pub struct Job(HANDLE);

impl Job {
    /// A new job with kill-on-close set, or None when Windows will not make one.
    pub fn kill_on_close() -> Option<Job> {
        // SAFETY: no attributes and no name: an unnamed job that only this handle holds.
        let handle = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.ok()?;
        let job = Job(handle);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `info` is the structure this class asks for, alive for the call, with its size.
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                (&raw const info).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .ok()?;
        Some(job)
    }

    /// Puts the child in the job. False when Windows refuses.
    pub fn assign(&self, child: &Child) -> bool {
        let process = HANDLE(child.as_raw_handle());
        // SAFETY: both handles are open: the job is ours and the child holds its own process.
        unsafe { AssignProcessToJobObject(self.0, process) }.is_ok()
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle is closed once, here; closing it ends the processes in the job.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
