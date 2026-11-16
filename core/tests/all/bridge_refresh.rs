//! The step taken at every start, with no C# test file behind it.
#![cfg(windows)]
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use sobreiro_core::connect::bridge_command::{command_for, status_line_json};
use sobreiro_core::connect::bridge_refresh::{BridgeRefresh, refresh};

use crate::TestFolder;

/// 5,000 bytes that stand in for the carried bridge.
fn bytes() -> Vec<u8> {
    (0..5000u32).map(|i| (i * 7) as u8).collect()
}

fn source() -> io::Result<Cursor<Vec<u8>>> {
    Ok(Cursor::new(bytes()))
}

/// The folder, the path of `settings.json` and the path of the bridge in the data folder.
fn layout(name: &str) -> (TestFolder, PathBuf, PathBuf) {
    let folder = TestFolder::new(name);
    folder.create();
    let settings = folder.file("settings.json");
    let bridge = folder.path().join("data").join("sobreiro-bridge.exe");
    (folder, settings, bridge)
}

/// A `settings.json` whose status line runs `bridge`.
fn connect(settings: &Path, bridge: &Path) {
    let line = status_line_json(&command_for(bridge), None).unwrap();
    std::fs::write(settings, format!("{{\"statusLine\": {line}}}")).unwrap();
}

fn put(path: &Path, content: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[test]
fn the_words_are_fixed() {
    let words: Vec<_> = [
        BridgeRefresh::NotCarried,
        BridgeRefresh::NotUsed,
        BridgeRefresh::Written,
        BridgeRefresh::Current,
        BridgeRefresh::Failed,
    ]
    .map(BridgeRefresh::word)
    .to_vec();
    assert_eq!(
        words,
        ["not-carried", "not-used", "written", "current", "failed"]
    );
}

#[test]
fn a_bridge_that_is_not_carried_does_nothing() {
    let (_folder, settings, bridge) = layout("bridge-refresh-not-carried");
    connect(&settings, &bridge);

    let none: Option<fn() -> io::Result<&'static [u8]>> = None;
    assert_eq!(refresh(none, &settings, &bridge), BridgeRefresh::NotCarried);
    assert!(!bridge.exists());
}

#[test]
fn a_deleted_copy_is_put_back_and_then_is_current() {
    let (_folder, settings, bridge) = layout("bridge-refresh-deleted");
    connect(&settings, &bridge);

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::Written
    );
    assert_eq!(std::fs::read(&bridge).unwrap(), bytes());
    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::Current
    );
}

#[test]
fn a_stale_copy_is_replaced() {
    let (_folder, settings, bridge) = layout("bridge-refresh-stale");
    connect(&settings, &bridge);
    put(&bridge, b"an older bridge");

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::Written
    );
    assert_eq!(std::fs::read(&bridge).unwrap(), bytes());
}

#[test]
fn a_status_line_that_runs_something_else_leaves_the_copy_alone() {
    let (_folder, settings, bridge) = layout("bridge-refresh-elsewhere");
    connect(&settings, Path::new("C:/elsewhere/sobreiro-bridge.exe"));
    put(&bridge, b"an older bridge");

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::NotUsed
    );
    assert_eq!(std::fs::read(&bridge).unwrap(), b"an older bridge");
}

#[test]
fn a_status_line_that_runs_something_else_creates_nothing() {
    let (_folder, settings, bridge) = layout("bridge-refresh-elsewhere-absent");
    connect(&settings, Path::new("C:/elsewhere/sobreiro-bridge.exe"));

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::NotUsed
    );
    assert!(!bridge.exists());
    assert!(!bridge.parent().unwrap().exists());
}

#[test]
fn no_settings_file_means_the_copy_is_not_used() {
    let (_folder, settings, bridge) = layout("bridge-refresh-no-settings");

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::NotUsed
    );
    assert!(!bridge.exists());
}

#[test]
fn settings_that_are_no_text_fail() {
    let (_folder, settings, bridge) = layout("bridge-refresh-not-utf8");
    std::fs::write(&settings, [0xFF, 0xFE, 0xFD, 0x00, 0x80]).unwrap();

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::Failed
    );
    assert!(!bridge.exists());
}

#[test]
fn a_copy_that_cannot_be_written_fails() {
    let (_folder, settings, bridge) = layout("bridge-refresh-unwritable");
    connect(&settings, &bridge);
    // A folder where the file goes: nothing can be written there.
    std::fs::create_dir_all(bridge.join("inside")).unwrap();

    assert_eq!(
        refresh(Some(source), &settings, &bridge),
        BridgeRefresh::Failed
    );
}
