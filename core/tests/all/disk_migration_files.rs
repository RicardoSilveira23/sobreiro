//! Ported from `tests/Sobreiro.Tests/DiskMigrationFilesTests.cs`, with tests of a full copy,
//! of what is never written over, and of the corners where the copy gives up.
use std::fs;
use std::path::{Path, PathBuf};

use sobreiro_core::migration::{DiskMigrationFiles, MigrationFiles};

use crate::TestFolder;

const FILES: DiskMigrationFiles = DiskMigrationFiles;

/// A folder `name` in `root` holding `config.json` and `sub\log.txt`.
fn make(root: &TestFolder, name: &str) -> PathBuf {
    let dir = root.file(name);
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("config.json"), "{}").unwrap();
    fs::write(dir.join("sub").join("log.txt"), "hello").unwrap();
    dir
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn move_renames_the_folder_with_its_contents() {
    let root = TestFolder::new("disk_migration_move");
    let from = make(&root, "old");
    let to = root.file("new");
    FILES.move_directory(&from, &to).unwrap();

    assert!(!from.exists());
    assert_eq!(read(&to.join("sub").join("log.txt")), "hello");
}

#[cfg(windows)]
#[test]
fn copy_skips_a_locked_file_and_counts_it() {
    let root = TestFolder::new("disk_migration_copy_locked");
    let from = make(&root, "old");
    let to = root.file("new");
    {
        let _held = crate::hold_alone(&from.join("sub").join("log.txt"));
        assert_eq!(FILES.copy_directory(&from, &to), 1);
    }
    assert!(to.join("config.json").exists());
    assert!(!to.join("sub").join("log.txt").exists());
    assert!(from.join("sub").join("log.txt").exists());
}

#[cfg(windows)]
#[test]
fn move_of_a_folder_holding_a_locked_file_fails_and_leaves_it_in_place() {
    let root = TestFolder::new("disk_migration_move_locked");
    let from = make(&root, "old");
    let to = root.file("new");
    {
        let _held = crate::hold_alone(&from.join("sub").join("log.txt"));
        assert!(FILES.move_directory(&from, &to).is_err());
    }
    assert!(from.is_dir());
    assert!(!to.exists());
}

#[test]
fn copy_carries_every_file_and_folder_and_keeps_the_old_folder() {
    let root = TestFolder::new("disk_migration_copy_all");
    let from = make(&root, "old");
    fs::create_dir_all(from.join("sub").join("deeper").join("empty")).unwrap();
    fs::write(from.join("sub").join("deeper").join("usage.json"), "[1]").unwrap();
    let to = root.file("new");

    assert_eq!(FILES.copy_directory(&from, &to), 0);
    assert_eq!(read(&to.join("config.json")), "{}");
    assert_eq!(read(&to.join("sub").join("log.txt")), "hello");
    assert_eq!(
        read(&to.join("sub").join("deeper").join("usage.json")),
        "[1]"
    );
    assert!(to.join("sub").join("deeper").join("empty").is_dir());
    assert_eq!(read(&from.join("sub").join("log.txt")), "hello");
}

#[test]
fn copy_never_writes_over_a_file_already_there() {
    let root = TestFolder::new("disk_migration_copy_no_overwrite");
    let from = make(&root, "old");
    let to = root.file("new");
    fs::create_dir_all(&to).unwrap();
    fs::write(to.join("config.json"), "mine").unwrap();

    assert_eq!(FILES.copy_directory(&from, &to), 1);
    assert_eq!(read(&to.join("config.json")), "mine");
    assert_eq!(read(&to.join("sub").join("log.txt")), "hello");
}

#[test]
fn move_onto_a_folder_that_exists_fails_and_leaves_both() {
    let root = TestFolder::new("disk_migration_move_onto");
    let from = make(&root, "old");
    let to = root.file("new");
    fs::create_dir_all(&to).unwrap();

    assert!(FILES.move_directory(&from, &to).is_err());
    assert_eq!(read(&from.join("config.json")), "{}");
    assert!(to.is_dir());
    assert!(!to.join("config.json").exists());
}

#[test]
fn copy_from_a_folder_that_is_not_there_counts_one() {
    let root = TestFolder::new("disk_migration_copy_missing");
    root.create();
    let to = root.file("new");

    assert_eq!(FILES.copy_directory(&root.file("old"), &to), 1);
    assert!(to.is_dir());
}

#[test]
fn copy_to_a_folder_that_cannot_be_made_counts_one() {
    let root = TestFolder::new("disk_migration_copy_unmade");
    let from = make(&root, "old");
    fs::write(root.file("a file"), "").unwrap();

    assert_eq!(
        FILES.copy_directory(&from, &root.file("a file").join("new")),
        1
    );
}

#[test]
fn a_folder_is_there_and_a_file_or_nothing_is_not() {
    let root = TestFolder::new("disk_migration_exists");
    let from = make(&root, "old");

    assert!(FILES.directory_exists(&from));
    assert!(!FILES.directory_exists(&from.join("config.json")));
    assert!(!FILES.directory_exists(&root.file("nothing")));
}

#[cfg(windows)]
#[test]
fn copy_does_not_follow_a_junction_and_counts_it() {
    let root = TestFolder::new("disk_migration_copy_junction");
    let from = make(&root, "old");
    let elsewhere = make(&root, "elsewhere");
    let made = std::process::Command::new("cmd")
        .arg("/C")
        .arg("mklink")
        .arg("/J")
        .arg(from.join("link"))
        .arg(&elsewhere)
        .output()
        .unwrap();
    assert!(made.status.success());
    let to = root.file("new");

    assert_eq!(FILES.copy_directory(&from, &to), 1);
    assert!(!to.join("link").exists());
    assert_eq!(read(&to.join("sub").join("log.txt")), "hello");
    assert!(elsewhere.join("config.json").exists());
}
