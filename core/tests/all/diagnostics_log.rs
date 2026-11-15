//! Ported from `tests/Sobreiro.Tests/DiagnosticsLogTests.cs`.
//!
//! Not ported: `Write_uses_this_process_id`. Core has no `write`: the caller formats the entry
//! with `log_entry::format` and appends it (`app_paths.rs` does that).
use std::fs;
use std::sync::Arc;
use std::thread;

use sobreiro_core::diagnostics::diagnostics_log::{DEFAULT_MAX_BYTES, DiagnosticsLog};

use crate::TestFolder;

fn log_path(dir: &TestFolder) -> std::path::PathBuf {
    dir.file("log.txt")
}

fn previous_path(dir: &TestFolder) -> std::path::PathBuf {
    dir.file("log.1.txt")
}

fn text(path: &std::path::Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn append_creates_the_folder_and_writes_utf8_without_bom() {
    let dir = TestFolder::new("diagnostics_log_creates_folder");
    let log = DiagnosticsLog::new(&dir.file("sub").join("log.txt"));
    assert!(log.append("h\u{e9}llo\n"));
    let bytes = fs::read(log.file_path()).unwrap();
    assert_eq!(bytes, "h\u{e9}llo\n".as_bytes());
    assert_ne!(bytes[0], 0xEF);
}

#[test]
fn append_adds_to_the_end() {
    let dir = TestFolder::new("diagnostics_log_adds_to_the_end");
    let log = DiagnosticsLog::new(&log_path(&dir));
    assert!(log.append("one\n"));
    assert!(log.append("two\n"));
    assert_eq!(text(&log_path(&dir)), "one\ntwo\n");
}

#[test]
fn append_rolls_to_log_1_when_the_cap_would_be_passed() {
    let dir = TestFolder::new("diagnostics_log_rolls");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 100);
    let first = "a".repeat(59) + "\n";
    let second = "b".repeat(59) + "\n";
    assert!(log.append(&first));
    assert!(log.append(&second));
    assert_eq!(text(&log_path(&dir)), second);
    assert_eq!(text(&previous_path(&dir)), first);
}

#[test]
fn append_replaces_an_older_log_1() {
    let dir = TestFolder::new("diagnostics_log_replaces_log_1");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 100);
    dir.create();
    fs::write(previous_path(&dir), "old\n").unwrap();
    let first = "a".repeat(59) + "\n";
    assert!(log.append(&first));
    assert!(log.append(&("b".repeat(59) + "\n")));
    assert_eq!(text(&previous_path(&dir)), first);
}

#[test]
fn append_never_rolls_an_empty_file() {
    let dir = TestFolder::new("diagnostics_log_never_rolls_empty");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 100);
    let big = "a".repeat(300) + "\n";
    assert!(log.append(&big));
    assert_eq!(text(&log_path(&dir)), big);
    assert!(!previous_path(&dir).exists());
}

#[test]
fn append_returns_false_when_the_path_is_a_directory() {
    let dir = TestFolder::new("diagnostics_log_path_is_directory");
    let path = log_path(&dir);
    fs::create_dir_all(&path).unwrap();
    assert!(!DiagnosticsLog::new(&path).append("x\n"));
}

#[test]
fn previous_file_path_sits_beside_the_log() {
    let dir = TestFolder::new("diagnostics_log_previous_path");
    assert_eq!(
        DiagnosticsLog::new(&log_path(&dir)).previous_file_path(),
        previous_path(&dir)
    );
}

// Not in the C# tests.

#[test]
fn a_roll_that_fails_does_not_stop_the_append() {
    let dir = TestFolder::new("diagnostics_log_failed_roll");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 100);
    // A folder that is not empty at the place the old file would be moved to.
    fs::create_dir_all(previous_path(&dir).join("held")).unwrap();
    let first = "a".repeat(59) + "\n";
    let second = "b".repeat(59) + "\n";
    assert!(log.append(&first));
    assert!(log.append(&second));
    assert_eq!(text(&log_path(&dir)), first + &second);
    assert!(previous_path(&dir).join("held").exists());
}

#[test]
fn a_cap_of_zero_rolls_every_entry_after_the_first() {
    let dir = TestFolder::new("diagnostics_log_cap_of_zero");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 0);
    assert!(log.append("one\n"));
    assert!(!previous_path(&dir).exists());
    assert!(log.append("two\n"));
    assert!(log.append("three\n"));
    assert_eq!(text(&log_path(&dir)), "three\n");
    assert_eq!(text(&previous_path(&dir)), "two\n");
}

#[test]
fn an_entry_that_brings_the_file_to_the_cap_exactly_does_not_roll() {
    let dir = TestFolder::new("diagnostics_log_exactly_the_cap");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 100);
    let half = "a".repeat(49) + "\n";
    assert!(log.append(&half));
    assert!(log.append(&half));
    assert_eq!(text(&log_path(&dir)), half.repeat(2));
    assert!(!previous_path(&dir).exists());
}

#[test]
fn the_cap_counts_bytes_not_characters() {
    let dir = TestFolder::new("diagnostics_log_counts_bytes");
    let log = DiagnosticsLog::with_max_bytes(&log_path(&dir), 10);
    // Four characters, eight bytes with the break.
    assert!(log.append("\u{e9}\u{e9}\u{e9}\u{e9}\n"));
    assert!(log.append("\u{e9}\n"));
    assert_eq!(text(&log_path(&dir)), "\u{e9}\n");
    assert_eq!(text(&previous_path(&dir)), "\u{e9}\u{e9}\u{e9}\u{e9}\n");
}

#[test]
fn the_previous_file_keeps_the_extension_and_has_none_without_one() {
    let dir = TestFolder::new("diagnostics_log_previous_names");
    assert_eq!(
        DiagnosticsLog::new(&dir.file("a.b.log")).previous_file_path(),
        dir.file("a.b.1.log")
    );
    assert_eq!(
        DiagnosticsLog::new(&dir.file("log")).previous_file_path(),
        dir.file("log.1")
    );
}

#[test]
fn the_default_cap_is_256_kib() {
    assert_eq!(DEFAULT_MAX_BYTES, 262_144);
}

#[test]
fn threads_appending_together_lose_no_entry() {
    let dir = TestFolder::new("diagnostics_log_threads");
    let log = Arc::new(DiagnosticsLog::new(&log_path(&dir)));
    let workers: Vec<_> = (0..4)
        .map(|n| {
            let log = Arc::clone(&log);
            thread::spawn(move || (0..25).all(|i| log.append(&format!("t{n} entry {i}\n"))))
        })
        .collect();
    for worker in workers {
        assert!(worker.join().unwrap());
    }
    assert_eq!(text(&log_path(&dir)).lines().count(), 100);
}
