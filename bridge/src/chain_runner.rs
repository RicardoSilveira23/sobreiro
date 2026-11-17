//! Runs the user's previous status line command with the same stdin.
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sobreiro_core::bridge::ChainResult;
use sobreiro_core::io::bounded_read;

use crate::windows_calls::{self, ProcessFamily};

/// How long the command gets before it is ended and the bridge prints its own line.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Tells Windows not to open a console window for the command.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The three bytes that some programs put at the start of UTF-8 text.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Where Windows PowerShell is, under the system folder `system_directory`. Started by its
/// full path, so that no `powershell.exe` earlier on the search path, or in the folder the
/// bridge runs in, is found first.
pub fn powershell_in(system_directory: &Path) -> PathBuf {
    system_directory
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe")
}

/// The process to start for `chained`: Git Bash when there is one, as Claude Code itself would
/// use, and Windows PowerShell otherwise. `None` when there is no Git Bash and Windows does
/// not say where its system folder is, and so where PowerShell is.
pub fn command(chained: &str, bash_path: Option<&str>) -> Option<Command> {
    let powershell = || windows_calls::system_directory().map(|folder| powershell_in(&folder));
    command_with(chained, bash_path, powershell)
}

/// [`command`], with the lookup of PowerShell's path handed in.
fn command_with(
    chained: &str,
    bash_path: Option<&str>,
    powershell: impl FnOnce() -> Option<PathBuf>,
) -> Option<Command> {
    let mut command = match bash_path {
        Some(bash) => {
            let mut command = Command::new(bash);
            command.arg("-c").arg(chained);
            command
        }
        None => {
            let mut command = Command::new(powershell()?);
            // PowerShell is first told to print UTF-8 with no byte order mark, like Git Bash.
            command
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg(format!(
                    "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); {chained}"
                ));
            command
        }
    };
    command
        // Lets a bridge started by the chained command see it is inside a chain and not chain
        // again.
        .env("SOBREIRO_IN_CHAIN", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // What the command prints as errors goes where the bridge's own would, so Claude Code's
        // --debug still sees it.
        .stderr(Stdio::inherit())
        .creation_flags(CREATE_NO_WINDOW);
    Some(command)
}

/// Runs `chained`, gives it `stdin`, and gives back what it printed and its exit code. `None`
/// when the process cannot be started or does not finish within `timeout`; it is then ended,
/// with every process it started.
pub fn run(
    chained: &str,
    stdin: &str,
    bash_path: Option<&str>,
    timeout: Duration,
) -> Option<ChainResult> {
    let mut child = command(chained, bash_path)?.spawn().ok()?;
    let started = Instant::now();
    let remaining = || timeout.saturating_sub(started.elapsed());
    let family = ProcessFamily::of(&child);

    // What the command prints is read on a thread of its own, and handed over when the command
    // and everything else holding its output have let go of it.
    let (hand_over, output) = mpsc::channel();
    let child_stdout = child.stdout.take();
    let reading = std::thread::Builder::new().spawn(move || {
        if let Some(mut child_stdout) = child_stdout {
            // More than the cap is an error, and the pipe is let go of at once, so a command
            // that keeps printing is not waited for; the time limit ends it.
            let read = bounded_read::read_at_most(&mut child_stdout, bounded_read::MAX_BYTES);
            drop(child_stdout);
            let _ = hand_over.send(read);
        }
    });
    // Stdin is written on another thread, so a command that never reads it cannot hold the
    // bridge past the time limit. A command that ends without reading all of it is fine: what
    // it printed still counts.
    let child_stdin = child.stdin.take();
    let stdin = stdin.to_string();
    let writing = std::thread::Builder::new().spawn(move || {
        if let Some(mut child_stdin) = child_stdin {
            let _ = child_stdin.write_all(stdin.as_bytes());
            // `child_stdin` closes here, which tells the command there is no more to read.
        }
    });
    // `Builder::spawn` gives an error where `thread::spawn` would end the whole bridge with
    // nothing printed. Without both threads the command cannot be heard: end it and say so.
    if reading.is_err() || writing.is_err() {
        family.end(&mut child);
        return None;
    }

    // Waiting for the output bounds the rest too: a process left in the background that still
    // holds the output open cannot hang the bridge.
    let finished = windows_calls::wait_for_exit(&child, remaining());
    let printed = if finished {
        output.recv_timeout(remaining()).ok()
    } else {
        None
    };
    let Some(Ok(bytes)) = printed else {
        family.end(&mut child);
        return None;
    };

    let exit_code = child.wait().ok()?.code()?;
    let text = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&bytes);
    Some(ChainResult {
        exit_code,
        output: String::from_utf8_lossy(text).into_owned(),
    })
}
