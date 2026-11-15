//! Ported from `tests/Sobreiro.Tests/BridgeInstallTests.cs`.
#![cfg(windows)]
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sobreiro_core::connect::bridge_install::{ensure, should_refresh};

use crate::TestFolder;

/// 5,000 bytes that stand in for the bridge's exe.
fn bytes() -> Vec<u8> {
    (0..5000u32).map(|i| (i * 7) as u8).collect()
}

/// The same bytes with one in the middle turned over: same length, other content.
fn flipped() -> Vec<u8> {
    let mut other = bytes();
    other[2500] ^= 0xFF;
    other
}

/// Opens the bridge to install, the way the exe that carries it would.
fn source() -> io::Result<Cursor<Vec<u8>>> {
    Ok(Cursor::new(bytes()))
}

/// One test's folder and the path of the bridge in its `data` folder.
fn folder(name: &str) -> (TestFolder, PathBuf) {
    let folder = TestFolder::new(name);
    let bridge_path = folder.path().join("data").join("sobreiro-bridge.exe");
    (folder, bridge_path)
}

/// Puts a bridge of `content` where the test's bridge goes.
fn install(bridge_path: &Path, content: &[u8]) {
    std::fs::create_dir_all(bridge_path.parent().unwrap()).unwrap();
    std::fs::write(bridge_path, content).unwrap();
}

/// Opens the file the way Windows holds a program that is running: others may read it, but
/// nobody may write it, rename it or put another file in its place.
fn hold_as_running(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    // What others may do with the file meanwhile. Windows' number for "read, nothing else".
    const SHARE_READING_ONLY: u32 = 1;
    std::fs::File::options()
        .read(true)
        .share_mode(SHARE_READING_ONLY)
        .open(path)
        .unwrap()
}

#[test]
fn ensure_writes_the_bridge_and_its_folder_when_missing() {
    let (_folder, bridge_path) = folder("bridge-install-missing");

    assert!(ensure(source, &bridge_path).unwrap());
    assert_eq!(std::fs::read(&bridge_path).unwrap(), bytes());
}

#[test]
fn ensure_leaves_an_identical_bridge_alone() {
    let (_folder, bridge_path) = folder("bridge-install-identical");
    install(&bridge_path, &bytes());
    // 1 January 2020, in seconds since 1970.
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_577_836_800);
    std::fs::File::options()
        .write(true)
        .open(&bridge_path)
        .unwrap()
        .set_modified(old)
        .unwrap();

    assert!(!ensure(source, &bridge_path).unwrap());
    let written = std::fs::metadata(&bridge_path).unwrap().modified().unwrap();
    assert_eq!(written, old);
}

#[test]
fn ensure_replaces_other_bytes_of_the_same_length() {
    let (_folder, bridge_path) = folder("bridge-install-other-bytes");
    install(&bridge_path, &flipped());

    assert!(ensure(source, &bridge_path).unwrap());
    assert_eq!(std::fs::read(&bridge_path).unwrap(), bytes());
}

#[test]
fn ensure_waits_for_a_bridge_that_is_running() {
    let (_folder, bridge_path) = folder("bridge-install-waits");
    install(&bridge_path, &flipped());
    let running = hold_as_running(&bridge_path);
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        drop(running);
    });

    assert!(ensure(source, &bridge_path).unwrap());
    release.join().unwrap();
    assert_eq!(std::fs::read(&bridge_path).unwrap(), bytes());
}

#[test]
fn ensure_throws_and_leaves_no_temp_file_when_the_bridge_stays_locked() {
    let (_folder, bridge_path) = folder("bridge-install-locked");
    let other = flipped();
    install(&bridge_path, &other);
    let running = hold_as_running(&bridge_path);

    let result = ensure(source, &bridge_path);

    // The two errors Windows gives for a file someone holds: "access denied" (5) and
    // "used by another process" (32). They are the two kinds of exception the C# test takes.
    let error = result.unwrap_err();
    assert!(matches!(error.raw_os_error(), Some(5 | 32)), "{error:?}");
    let files: Vec<PathBuf> = std::fs::read_dir(bridge_path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files, std::slice::from_ref(&bridge_path));
    drop(running);
    assert_eq!(std::fs::read(&bridge_path).unwrap(), other);
}

#[test]
fn should_refresh_only_when_the_status_line_runs_the_installed_copy() {
    let rows = [
        (
            "{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Data/sobreiro-bridge.exe\\\"\"}}",
            true,
        ),
        (
            "{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/Old/sobreiro-bridge.exe\\\"\"}}",
            false,
        ),
        ("{}", false),
        ("not json", false),
        ("", false),
    ];
    for (settings, expected) in rows {
        assert_eq!(
            should_refresh(settings, Path::new(r"C:\Data\sobreiro-bridge.exe")),
            expected,
            "{settings}"
        );
    }
}

#[test]
fn should_refresh_when_the_installed_path_has_forward_slashes() {
    assert!(should_refresh(
        "{\"statusLine\":{\"type\":\"command\",\"command\":\"\\\"C:/data/sobreiro-bridge.exe\\\"\"}}",
        Path::new("C:/data\\sobreiro-bridge.exe")
    ));
}

// What follows has no C# test. The C# of 1.1.0 was put through the same on 2026-11-14 and did
// the same: it waited 0.9 s, gave "access denied" and left one file in the folder.

/// The C# tests stand in for a running bridge with a file held open. This one runs a real
/// program from the file, which Windows guards in another way: the file may be read, but not
/// written, deleted or replaced.
#[test]
fn ensure_gives_up_on_a_program_that_really_runs_and_leaves_it_as_it_was() {
    let (_folder, bridge_path) = folder("bridge-install-really-running");
    // Any small program of Windows will do. `ping` with these arguments runs for ten seconds
    // or until it is ended below.
    let Some(windows) = std::env::var_os("SystemRoot") else {
        return;
    };
    let Ok(program) = std::fs::read(Path::new(&windows).join("System32").join("ping.exe")) else {
        return;
    };
    install(&bridge_path, &program);
    let mut running = std::process::Command::new(&bridge_path)
        .args(["-n", "10", "127.0.0.1"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // A running program can still be read, so the same bytes are found to be the same.
    let same = ensure(|| Ok(Cursor::new(program.clone())), &bridge_path);
    let started = std::time::Instant::now();
    let other = ensure(source, &bridge_path);
    let waited = started.elapsed();
    let files: Vec<PathBuf> = std::fs::read_dir(bridge_path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    let left = std::fs::read(&bridge_path).unwrap();
    running.kill().unwrap();
    running.wait().unwrap();

    assert!(!same.unwrap());
    assert_eq!(other.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    // Ten tries with 100 ms between them.
    assert!(waited >= Duration::from_millis(850), "{waited:?}");
    assert_eq!(files, std::slice::from_ref(&bridge_path));
    assert_eq!(left, program);

    // Once the program has ended, the bridge is written.
    assert!(ensure(source, &bridge_path).unwrap());
    assert_eq!(std::fs::read(&bridge_path).unwrap(), bytes());
}
