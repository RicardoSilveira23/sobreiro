//! A program the shell starts with no window and waits for within a limit: `gh auth token` for
//! Copilot's sign-in, and `claude auth logout` for the menu's Sign out.
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

/// Keeps a console window from opening for a program started with it.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// How `child` ended, looked at every `step` until `deadline`. None when it had not ended by
/// then, or could not be asked; it is not stopped here.
pub fn wait_until(child: &mut Child, deadline: Instant, step: Duration) -> Option<ExitStatus> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(step),
            _ => return None,
        }
    }
}
