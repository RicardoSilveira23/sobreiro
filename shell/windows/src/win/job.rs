//! A job object that ends every process in it when it is closed: what stops `gh`, and anything
//! `gh` started, when it does not answer in time, and Claude Code's hidden console when its
//! sign-in was renewed or Sobreiro quits.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::process::Child;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
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
        job.kill_when_closed(true).then_some(job)
    }

    /// Sets or clears kill-on-close. False when Windows refuses.
    fn kill_when_closed(&self, kill: bool) -> bool {
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if kill {
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        }
        // SAFETY: `info` is the structure this class asks for, alive for the call, with its size.
        unsafe {
            SetInformationJobObject(
                self.0,
                JobObjectExtendedLimitInformation,
                (&raw const info).cast::<c_void>(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .is_ok()
    }

    /// Lets the processes outlive the job: closing it no longer ends them. For a console handed
    /// to the user, which is theirs from then on. False when Windows refuses.
    pub fn let_go(&self) -> bool {
        self.kill_when_closed(false)
    }

    /// Puts the child in the job. False when Windows refuses.
    pub fn assign(&self, child: &Child) -> bool {
        self.assign_process(HANDLE(child.as_raw_handle()))
    }

    /// Puts the process behind `process`, an open handle, in the job. False when Windows refuses.
    pub fn assign_process(&self, process: HANDLE) -> bool {
        // SAFETY: both handles are open: the job is ours and the caller holds the process's.
        unsafe { AssignProcessToJobObject(self.0, process) }.is_ok()
    }

    /// Ends every process in the job now. False when Windows refuses.
    pub fn end_all(&self) -> bool {
        // SAFETY: the job's handle is ours and open; the exit code is any number.
        unsafe { TerminateJobObject(self.0, 1) }.is_ok()
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: the handle is closed once, here; closing it ends the processes in the job.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
