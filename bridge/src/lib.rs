//! The bridge: the command Claude Code's status line runs, many times a minute while a session
//! is open. It reads the status line's JSON from stdin, keeps the `rate_limits` in it for the
//! tray, and prints a status line: the user's previous one when they had one, its own otherwise.
//!
//! What it does is Core's `BridgeRunner`. This crate adds what an exe must bring: stdin and
//! stdout, the environment, the clock, and the process that runs the previous command.
//!
//! It always prints something and never fails: a status line that goes blank is worse than one
//! that is a reading behind.

pub mod chain_runner;
mod windows_calls;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use sobreiro_core::app_paths::{AppPaths, DATA_DIR_VARIABLE};
use sobreiro_core::bridge::{BridgeRunner, ChainResult, bash_locator};
use sobreiro_core::display::status_line_text;

/// The three bytes that some programs put at the start of UTF-8 text.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// One run of the bridge, from reading stdin to the exit code.
pub fn run() -> i32 {
    let mut stdout = std::io::stdout().lock();

    let mut input = Vec::new();
    if std::io::stdin().lock().read_to_end(&mut input).is_err() {
        // Nothing could be read: print the line for "no limits known" and leave it at that.
        let line = status_line_text::render(None) + "\n";
        let _ = stdout.write_all(line.as_bytes());
        let _ = stdout.flush();
        return 0;
    }
    let input = input.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&input);
    let stdin = String::from_utf8_lossy(input);

    let overridden = std::env::var_os(DATA_DIR_VARIABLE);
    let paths = AppPaths::new(overridden.as_deref(), &local_app_data());
    let usage_path = paths.usage_file();
    let config_path = paths.config_file();
    let runner = BridgeRunner {
        usage_path: &usage_path,
        config_path: &config_path,
        run_chain: &run_chain,
        inside_chain: std::env::var_os("SOBREIRO_IN_CHAIN").is_some_and(|value| value == "1"),
    };

    let now = DateTime::<Utc>::from(SystemTime::now());
    let exit_code = runner.run(&stdin, &mut stdout, now);
    let _ = stdout.flush();
    exit_code
}

/// Runs the user's previous status line command in Git Bash, or in PowerShell without one.
fn run_chain(command: &str, stdin: &str) -> Option<ChainResult> {
    let bash = bash_locator::find(
        |name| std::env::var(name).ok(),
        |file| Path::new(file).is_file(),
    );
    chain_runner::run(
        command,
        stdin,
        bash.as_deref(),
        chain_runner::DEFAULT_TIMEOUT,
    )
}

/// The user's local application data folder. Windows is asked first; should it not answer, the
/// environment is; and with neither, the data folder ends up beside wherever the bridge runs,
/// as it did in 1.x.
fn local_app_data() -> PathBuf {
    windows_calls::local_app_data()
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_default()
}
