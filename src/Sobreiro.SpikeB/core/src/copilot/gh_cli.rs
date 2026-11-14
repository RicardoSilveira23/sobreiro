use super::CopilotToken;
use crate::secret::{Secret, wipe};
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Asks the GitHub CLI for its token (`gh auth token`). Never fails; never prints or logs what it read.
pub struct GhCli;

impl GhCli {
    pub fn token() -> Vec<Secret> {
        match Self::find(std::env::var("PATH").ok().as_deref()) {
            Some(exe) => Self::run(&exe, &["auth", "token", "--hostname", "github.com"], Duration::from_secs(5)),
            None => Vec::new(),
        }
    }

    /// The full path of gh.exe in the first absolute PATH entry that holds one; empty and relative entries are skipped.
    pub fn find(path: Option<&str>) -> Option<String> {
        for entry in path.unwrap_or("").split(';').map(str::trim).filter(|e| !e.is_empty()) {
            if !Path::new(entry).is_absolute() {
                continue;
            }
            let candidate: PathBuf = Path::new(entry).join("gh.exe");
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
        None
    }

    /// Runs the program, waits up to `timeout` for it to end and for its output to drain, and returns the token
    /// found in what it printed (none when it exits non-zero or runs over). A grandchild that keeps the pipe open is
    /// bounded by the same timeout. Only the process itself is killed on a timeout (the C# kills its tree).
    pub fn run(exe: &str, args: &[&str], timeout: Duration) -> Vec<Secret> {
        let Ok(mut child) = Command::new(exe)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        else {
            return Vec::new();
        };

        // Read both pipes on their own threads before waiting, so a full pipe cannot block the child.
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        if let Some(mut out) = child.stdout.take() {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let _ = out.read_to_end(&mut buf);
                let _ = tx.send(buf);
            });
        }
        if let Some(mut err) = child.stderr.take() {
            std::thread::spawn(move || {
                let _ = std::io::copy(&mut err, &mut std::io::sink());
            });
        }

        let clock = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if clock.elapsed() < timeout => std::thread::sleep(Duration::from_millis(10)),
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Vec::new();
                }
            }
        };
        if !status.success() {
            return Vec::new();
        }

        let remaining = timeout.saturating_sub(clock.elapsed());
        let Ok(mut bytes) = rx.recv_timeout(remaining) else { return Vec::new() };
        let token = CopilotToken::find(Some(&String::from_utf8_lossy(&bytes)));
        wipe(&mut bytes);
        token.into_iter().collect()
    }
}
