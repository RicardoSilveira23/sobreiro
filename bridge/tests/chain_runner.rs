//! Ported from `tests/Sobreiro.Tests/ChainRunnerTests.cs`. The last test is new.
//!
//! Where the C# tests look at a process's start settings, these look at the command that would
//! be started. Whether its input and output are piped cannot be read off a command here; the
//! tests that run one show it.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used)]

use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, Instant};

use sobreiro_bridge::chain_runner::{self, DEFAULT_TIMEOUT};
use sobreiro_core::bridge::{ChainResult, bash_locator};

/// Git Bash on this PC, if it has one. The tests that need it pass without it, as in the C#:
/// there is nothing to run against.
fn git_bash() -> Option<String> {
    bash_locator::find(
        |name| std::env::var(name).ok(),
        |file| Path::new(file).is_file(),
    )
}

fn arguments(command: &std::process::Command) -> Vec<&OsStr> {
    command.get_args().collect()
}

#[test]
fn start_info_uses_bash_dash_c() {
    let command = chain_runner::command("echo hi", Some(r"C:\Git\bin\bash.exe"));

    assert_eq!(command.get_program(), r"C:\Git\bin\bash.exe");
    assert_eq!(arguments(&command), ["-c", "echo hi"]);
}

#[test]
fn start_info_falls_back_to_powershell() {
    let command = chain_runner::command("echo hi", None);

    assert_eq!(command.get_program(), "powershell.exe");
    assert_eq!(
        arguments(&command),
        [
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); echo hi"
        ]
    );
}

#[test]
fn start_info_marks_the_child_as_inside_a_chain() {
    for bash_path in [Some(r"C:\Git\bin\bash.exe"), None] {
        let command = chain_runner::command("echo hi", bash_path);

        let marked = command
            .get_envs()
            .any(|(name, value)| name == "SOBREIRO_IN_CHAIN" && value == Some(OsStr::new("1")));
        assert!(marked, "{bash_path:?}");
    }
}

#[test]
fn run_passes_stdin_and_returns_stdout_and_exit_code() {
    let Some(bash) = git_bash() else { return };

    let result = chain_runner::run("cat; exit 3", "hello", Some(&bash), DEFAULT_TIMEOUT);

    assert_eq!(
        result,
        Some(ChainResult {
            exit_code: 3,
            output: "hello".to_string()
        })
    );
}

#[test]
fn run_returns_null_when_executable_missing() {
    let result = chain_runner::run("echo hi", "", Some(r"C:\nope\bash.exe"), DEFAULT_TIMEOUT);

    assert_eq!(result, None);
}

#[test]
fn run_times_out_and_returns_null() {
    let Some(bash) = git_bash() else { return };
    let watch = Instant::now();

    let result = chain_runner::run("sleep 20", "", Some(&bash), Duration::from_millis(500));

    assert_eq!(result, None);
    assert!(
        watch.elapsed() < Duration::from_secs(5),
        "took {:?}",
        watch.elapsed()
    );
}

#[test]
fn run_powershell_output_is_utf8() {
    // This checks the encoding, not the speed: a cold PowerShell on a CI runner can take
    // longer than the default 5 s to start, so give it room.
    let result = chain_runner::run("Write-Output '5h · 7d'", "", None, Duration::from_secs(30));

    assert_eq!(result.unwrap().output.trim_end(), "5h · 7d");
}

#[test]
fn run_times_out_when_the_child_never_reads_a_large_stdin() {
    let Some(bash) = git_bash() else { return };
    let watch = Instant::now();

    let large = "x".repeat(64 * 1024);
    let result = chain_runner::run("sleep 20", &large, Some(&bash), Duration::from_millis(500));

    assert_eq!(result, None);
    assert!(
        watch.elapsed() < Duration::from_secs(5),
        "took {:?}",
        watch.elapsed()
    );
}

#[test]
fn run_times_out_when_a_background_child_keeps_stdout_open() {
    let Some(bash) = git_bash() else { return };
    let watch = Instant::now();

    let result = chain_runner::run(
        "sleep 20 & echo hi",
        "",
        Some(&bash),
        Duration::from_millis(1000),
    );

    assert_eq!(result, None);
    assert!(
        watch.elapsed() < Duration::from_secs(5),
        "took {:?}",
        watch.elapsed()
    );
}

// Not in the C# tests.

/// The C# ends the command and every process it started when its time is up. Here a process the
/// command left in the background would write a file two seconds in; the command is ended after
/// a third of a second, and three seconds later there is still no file.
#[test]
fn a_command_out_of_time_takes_the_processes_it_started_with_it() {
    let Some(bash) = git_bash() else { return };
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("chain-ends-the-family");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let marker = folder.join("late.txt");
    // Git Bash takes a Windows path written with forward slashes.
    let marker_for_bash = marker.to_string_lossy().replace('\\', "/");

    let result = chain_runner::run(
        &format!("(sleep 2; echo late > '{marker_for_bash}') & sleep 20"),
        "",
        Some(&bash),
        Duration::from_millis(300),
    );
    std::thread::sleep(Duration::from_secs(3));

    assert_eq!(result, None);
    assert!(
        !marker.exists(),
        "the background process outlived the command"
    );
    let _ = std::fs::remove_dir_all(&folder);
}
