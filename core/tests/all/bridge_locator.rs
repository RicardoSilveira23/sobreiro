//! Ported from `tests/Sobreiro.Tests/BridgeLocatorTests.cs`.
//!
//! The paths are Windows paths, and Windows is asked to read them, so these run on Windows.
#![cfg(windows)]
use std::path::{Path, PathBuf};

use sobreiro_core::connect::bridge_locator::{find, own};

const APP: &str = r"C:\Apps\Sobreiro\app";
const DATA_BRIDGE: &str = r"C:\Data\sobreiro-bridge.exe";

/// The bridge beside the app, as a full path.
fn beside() -> PathBuf {
    std::path::absolute(Path::new(APP).join("sobreiro-bridge.exe")).unwrap()
}

#[test]
fn finds_the_bridge_beside_the_app() {
    let beside = beside();
    assert_eq!(
        find(Path::new(APP), |path| path == beside),
        Some(beside.clone())
    );
}

#[test]
fn finds_the_bridge_in_the_sibling_bridge_folder() {
    let sibling = Path::new(r"C:\Apps\Sobreiro\bridge\sobreiro-bridge.exe");
    assert_eq!(
        find(Path::new(APP), |path| path == sibling),
        Some(sibling.to_path_buf())
    );
}

#[test]
fn prefers_the_bridge_beside_the_app() {
    assert_eq!(find(Path::new(APP), |_| true), Some(beside()));
}

#[test]
fn returns_null_when_neither_exists() {
    assert_eq!(find(Path::new(APP), |_| false), None);
}

#[test]
fn own_is_the_installed_copy_when_the_exe_carries_the_bridge() {
    assert_eq!(
        own(true, Path::new(DATA_BRIDGE), Path::new(APP), |_| true),
        Some(PathBuf::from(DATA_BRIDGE))
    );
}

#[test]
fn own_is_the_bridge_beside_the_app_otherwise() {
    let beside = beside();
    assert_eq!(
        own(false, Path::new(DATA_BRIDGE), Path::new(APP), |path| path
            == beside),
        Some(beside.clone())
    );
}

#[test]
fn own_is_null_when_not_carried_and_none_beside() {
    assert_eq!(
        own(false, Path::new(DATA_BRIDGE), Path::new(APP), |_| false),
        None
    );
}
