//! Ported from `tests/Sobreiro.Tests/AtomicFileTests.cs`.
use std::io::Cursor;

use sobreiro_core::io::atomic_file;

use crate::TestFolder;

#[test]
fn writes_the_text_without_a_bom_by_default() {
    let folder = TestFolder::new("atomic-no-bom");
    let path = folder.file("a.txt");

    atomic_file::write_all_text(&path, "héllo").unwrap();

    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), "héllo");
    assert_ne!(bytes[0], 0xEF);
}

#[test]
fn writes_a_bom_when_asked() {
    let folder = TestFolder::new("atomic-bom");
    let path = folder.file("a.txt");

    atomic_file::write_all_text_with_bom(&path, "x").unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), [0xEF, 0xBB, 0xBF, b'x']);
}

#[test]
fn creates_a_missing_folder_and_replaces_an_existing_file() {
    let folder = TestFolder::new("atomic-missing-folder");
    let path = folder.path().join("deep").join("er").join("a.txt");

    atomic_file::write_all_text(&path, "one").unwrap();
    atomic_file::write_all_text(&path, "two").unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
}

#[cfg(windows)]
#[test]
fn a_failed_rename_leaves_no_temp_file_and_throws_the_original_error() {
    use std::os::windows::fs::OpenOptionsExt;

    let folder = TestFolder::new("atomic-failed-rename");
    folder.create();
    let path = folder.file("a.txt");
    std::fs::write(&path, "old").unwrap();
    // Open for reading and shared with nobody, as the C# test holds it.
    let _held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    assert!(atomic_file::write_all_text(&path, "new").is_err());
    assert!(folder.temp_files().is_empty());
}

#[test]
fn leaves_no_temp_file_behind() {
    let folder = TestFolder::new("atomic-no-temp");

    atomic_file::write_all_text(&folder.file("a.txt"), "x").unwrap();

    assert!(folder.temp_files().is_empty());
}

#[test]
fn write_from_a_stream_replaces_the_file() {
    let folder = TestFolder::new("atomic-stream");
    folder.create();
    let path = folder.file("a.bin");
    std::fs::write(&path, "old").unwrap();

    atomic_file::write_from(&path, &mut Cursor::new(b"new")).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    assert!(folder.temp_files().is_empty());
}
