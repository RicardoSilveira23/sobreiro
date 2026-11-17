//! Not in the C# tests. Files that other programs write are not read past a megabyte: the
//! helper itself, and what each reader does with a file over it. Each over-size file is valid
//! but for its size (spaces after the JSON), so the size is the only thing that is wrong.
use std::io::{Cursor, ErrorKind};

use sobreiro_core::alerts::alert_state::AlertState;
use sobreiro_core::bridge::BridgeConfig;
use sobreiro_core::config::app_config::{AppConfig, ConfigError};
use sobreiro_core::connect::connection::{self, ConnectError};
use sobreiro_core::io::bounded_read::{self, MAX_BYTES};
use sobreiro_core::usage::usage_file::{self, ForMerge};

use crate::TestFolder;

/// `json` followed by spaces up to one byte over the cap.
fn over_the_cap(json: &str) -> Vec<u8> {
    let mut bytes = json.as_bytes().to_vec();
    bytes.resize(MAX_BYTES as usize + 1, b' ');
    bytes
}

/// A file named `name` in a folder of its own, holding `bytes`. The folder goes when the
/// first value of the pair is dropped.
fn file_with(test: &str, name: &str, bytes: &[u8]) -> (TestFolder, std::path::PathBuf) {
    let folder = TestFolder::new(test);
    folder.create();
    let path = folder.file(name);
    std::fs::write(&path, bytes).unwrap();
    (folder, path)
}

#[test]
fn the_helper_reads_what_is_under_the_cap() {
    assert_eq!(
        bounded_read::read_at_most(Cursor::new(b"abc"), 10).unwrap(),
        b"abc"
    );
}

#[test]
fn the_helper_reads_what_is_exactly_the_cap() {
    assert_eq!(
        bounded_read::read_at_most(Cursor::new(b"abcde"), 5).unwrap(),
        b"abcde"
    );
}

#[test]
fn the_helper_refuses_what_is_over_the_cap() {
    let error = bounded_read::read_at_most(Cursor::new(b"abcdef"), 5).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn the_helper_does_not_read_a_stream_that_never_ends() {
    let endless = std::io::repeat(b'x');

    let error = bounded_read::read_at_most(endless, 1000).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidData);
}

#[test]
fn the_file_variant_has_the_same_cap() {
    let (_at_folder, at) = file_with("bounded-at", "a.json", &vec![b' '; MAX_BYTES as usize]);
    let (_over_folder, over) = file_with(
        "bounded-over",
        "a.json",
        &vec![b' '; MAX_BYTES as usize + 1],
    );

    assert_eq!(
        bounded_read::read_file(&at).unwrap().len() as u64,
        MAX_BYTES
    );
    assert_eq!(
        bounded_read::read_file(&over).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn an_oversize_usage_file_reads_as_none_and_is_written_over_like_one_that_does_not_parse() {
    let (_path_folder, path) = file_with("bounded-usage", "usage.json", &over_the_cap("{}"));

    assert_eq!(usage_file::read(&path), None);
    // Like content that does not parse: a new reading replaces it, so the tray is not left
    // waiting for a writer that does not exist.
    assert_eq!(
        usage_file::read_for_merge(&path),
        ForMerge::SafeToWrite(None)
    );
}

#[test]
fn an_oversize_config_is_unreadable_and_so_never_saved_over() {
    let (_path_folder, path) = file_with(
        "bounded-config",
        "config.json",
        &over_the_cap(r#"{"chained_command": "echo hi"}"#),
    );

    let result = AppConfig::load(&path);

    assert!(matches!(result, Err(ConfigError::Io(_))), "{result:?}");
}

#[test]
fn an_oversize_config_gives_the_bridge_no_chained_command() {
    let (_path_folder, path) = file_with(
        "bounded-bridge-config",
        "config.json",
        &over_the_cap(r#"{"chained_command": "echo hi"}"#),
    );

    assert_eq!(BridgeConfig::load(&path).chained_command, None);
}

#[test]
fn oversize_settings_are_refused_not_read() {
    let (_path_folder, path) = file_with("bounded-settings", "settings.json", &over_the_cap("{}"));

    let result = connection::read_settings(&path);

    assert!(matches!(result, Err(ConnectError::Io(_))), "{result:?}");
}

#[test]
fn an_oversize_alert_file_gives_no_marks() {
    let marks = r#"{"session": {"threshold": 80, "resets_at": 1738425600}}"#;
    let (_small_folder, small) = file_with("bounded-alerts-small", "alerts.json", marks.as_bytes());
    let (_path_folder, path) = file_with("bounded-alerts", "alerts.json", &over_the_cap(marks));

    // The same marks within the cap are read, so only the size makes the difference.
    assert!(!AlertState::load(&small).marks.is_empty());
    assert!(AlertState::load(&path).marks.is_empty());
}
