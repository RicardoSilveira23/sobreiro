//! A program started in a console window of its own that no one sees, which can be shown later as
//! it stands, or ended with everything it started: how the menu's Sign in and Sign out run Claude
//! Code.
//!
//! The process is made suspended, put in a job and only then let run, so nothing it starts is
//! outside the job. While the console is hidden the job ends it all when it is dropped (Sobreiro
//! quits, say); once it is shown the job lets it go, so quitting Sobreiro leaves it to the user.
//! [`HiddenConsole::end`] still ends it, shown or not.
//!
//! Showing it is asked from a worker thread, a while after the menu was used, so Windows usually
//! refuses to bring it to the front; its taskbar button then flashes until the user opens it.
#![allow(unsafe_code)]

use super::job::Job;
use std::cell::Cell;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::System::Threading::{
    CREATE_NEW_CONSOLE, CREATE_SUSPENDED, CreateProcessW, GetExitCodeProcess, PROCESS_INFORMATION,
    ResumeThread, STARTF_USESHOWWINDOW, STARTUPINFOW, TerminateProcess,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FLASHW_ALL, FLASHW_TIMERNOFG, FLASHWINFO, FindWindowExW, FlashWindowEx, GetWindowTextW,
    GetWindowThreadProcessId, SW_HIDE, SW_SHOWNORMAL, SetForegroundWindow, ShowWindow,
};
use windows::core::{PCWSTR, PWSTR, w};

/// The exit code Windows gives a process that is still running.
const STILL_ACTIVE: u32 = 259;

/// A program running in a console of its own.
pub struct HiddenConsole {
    process: HANDLE,
    id: u32,
    job: Job,
    /// The console's window, once found: it lives as long as the console.
    window: Cell<Option<HWND>>,
}

/// The text as Windows takes it: UTF-16, ended by a zero.
fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

impl HiddenConsole {
    /// Starts `program`, by its full path, with `arguments` as they go on its command line after
    /// its quoted path, in `folder`, in a new console that is not shown. None when it cannot be
    /// started or put in a job: nothing runs then.
    pub fn start(program: &Path, arguments: &str, folder: &Path) -> Option<HiddenConsole> {
        let job = Job::kill_on_close()?;
        let application = wide(program.as_os_str());
        let mut line = std::ffi::OsString::from("\"");
        line.push(program.as_os_str());
        line.push("\" ");
        line.push(arguments);
        let mut line = wide(&line);
        let folder = wide(folder.as_os_str());
        let startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            dwFlags: STARTF_USESHOWWINDOW,
            wShowWindow: SW_HIDE.0 as u16,
            ..Default::default()
        };
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: every string is ours, ended by a zero and alive for the call; the command line
        // is a buffer of our own that Windows may write to; no handles are inherited.
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(line.as_mut_ptr())),
                None,
                None,
                false,
                CREATE_NEW_CONSOLE | CREATE_SUSPENDED,
                None,
                PCWSTR(folder.as_ptr()),
                &raw const startup,
                &raw mut info,
            )
        }
        .ok()?;
        let console = HiddenConsole {
            process: info.hProcess,
            id: info.dwProcessId,
            job,
            window: Cell::new(None),
        };
        let placed = console.job.assign_process(console.process);
        if placed {
            // SAFETY: the thread's handle came from `CreateProcessW` and is used once here.
            let _ = unsafe { ResumeThread(info.hThread) };
        } else {
            // SAFETY: the process is ours and has not run a single instruction yet.
            let _ = unsafe { TerminateProcess(console.process, 1) };
        }
        // SAFETY: the thread's handle is ours and closed once, here.
        let _ = unsafe { CloseHandle(info.hThread) };
        placed.then_some(console)
    }

    /// The program's exit code once it has ended; `None` while it runs, or when Windows cannot
    /// say.
    pub fn exit_code(&self) -> Option<u32> {
        let mut code = 0u32;
        // SAFETY: the process handle is open for as long as `self` lives.
        let asked = unsafe { GetExitCodeProcess(self.process, &raw mut code) };
        (asked.is_ok() && code != STILL_ACTIVE).then_some(code)
    }

    /// Whether the program in the console is still running.
    pub fn is_running(&self) -> bool {
        self.exit_code().is_none()
    }

    /// Shows the console as it stands, brings it to the front or flashes it when Windows refuses,
    /// and gives it to the user: dropping this no longer ends it. False when its window was not
    /// found or the job would not let go: nothing was shown, and it still ends with the job.
    pub fn show(&self) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        // Given to the user before it is shown: a console that could not be let go stays hidden.
        if !self.job.let_go() {
            return false;
        }
        // SAFETY: `window` is the console's top-level window, alive while the console is; the
        // flash's structure is ours, alive for the call, with its size.
        unsafe {
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            if !SetForegroundWindow(window).as_bool() {
                let flash = FLASHWINFO {
                    cbSize: size_of::<FLASHWINFO>() as u32,
                    hwnd: window,
                    dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
                    uCount: 0,
                    dwTimeout: 0,
                };
                let _ = FlashWindowEx(&raw const flash);
            }
        }
        true
    }

    /// Ends the program and everything it started, and so its console, shown or not.
    pub fn end(&self) {
        let _ = self.job.end_all();
    }

    /// Whether the console's window title holds `text`. False when its window is not found.
    pub fn title_has(&self, text: &str) -> bool {
        let Some(window) = self.window() else {
            return false;
        };
        // A console's title is at most a few hundred characters; a longer one is cut short.
        let mut buffer = [0u16; 512];
        // SAFETY: the buffer is ours and its length is passed with it; a stale handle gives 0.
        let length = unsafe { GetWindowTextW(window, &mut buffer) };
        let length = usize::try_from(length).unwrap_or(0).min(buffer.len());
        String::from_utf16_lossy(&buffer[..length]).contains(text)
    }

    /// The console's window: the console host says a console window belongs to the first
    /// process attached to it, which is the program started here. Looked for until found once.
    fn window(&self) -> Option<HWND> {
        if let Some(window) = self.window.get() {
            return Some(window);
        }
        let found = self.find_window();
        self.window.set(found);
        found
    }

    fn find_window(&self) -> Option<HWND> {
        let mut after: Option<HWND> = None;
        // A PC has a few console windows at most; the bound only stops a list that keeps
        // changing under the walk.
        for _ in 0..512 {
            // SAFETY: looks a top-level window up by a constant class name, after the last found.
            let found =
                unsafe { FindWindowExW(None, after, w!("ConsoleWindowClass"), PCWSTR::null()) }
                    .ok()?;
            let mut owner = 0u32;
            // SAFETY: `owner` is ours to write; a stale handle gives 0.
            unsafe { GetWindowThreadProcessId(found, Some(&raw mut owner)) };
            if owner == self.id {
                return Some(found);
            }
            after = Some(found);
        }
        None
    }
}

impl Drop for HiddenConsole {
    fn drop(&mut self) {
        // SAFETY: the process handle is closed once, here. The job goes after it, and ends what
        // still runs unless the console was given to the user.
        let _ = unsafe { CloseHandle(self.process) };
    }
}
