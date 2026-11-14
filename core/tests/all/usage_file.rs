//! Ported from `tests/Sobreiro.Tests/UsageFileTests.cs`, but for its three tests of `AppPaths`,
//! which come with the area "Paths and config". The last test here is new.
use sobreiro_core::usage::usage_file::{self, ForMerge};
use sobreiro_core::usage::{RateLimits, UsageSnapshot};

use crate::{TestFolder, hold_alone, unix, window};

fn sample() -> UsageSnapshot {
    UsageSnapshot {
        limits: RateLimits {
            five_hour: Some(window(23.5, Some(unix(1738425600)))),
            seven_day: Some(window(41.2, Some(unix(1738857600)))),
            spend: None,
        },
        observed_at: unix(1738400000),
    }
}

#[test]
fn write_then_read_round_trips() {
    let folder = TestFolder::new("usage-round-trip");
    let path = folder.file("usage.json");

    usage_file::write(&path, &sample()).unwrap();

    assert_eq!(usage_file::read(&path), Some(sample()));
}

#[test]
fn write_creates_missing_directory() {
    let folder = TestFolder::new("usage-missing-directory");
    let path = folder.path().join("a").join("b").join("usage.json");

    usage_file::write(&path, &sample()).unwrap();

    assert!(path.is_file());
}

#[test]
fn write_replaces_existing_file_and_leaves_no_temp_files() {
    let folder = TestFolder::new("usage-replace");
    let path = folder.file("usage.json");

    usage_file::write(&path, &sample()).unwrap();
    usage_file::write(&path, &sample()).unwrap();

    assert_eq!(folder.files(), [path]);
}

#[test]
fn read_for_merge_gives_the_stored_snapshot() {
    let folder = TestFolder::new("usage-merge-stored");
    let path = folder.file("usage.json");
    usage_file::write(&path, &sample()).unwrap();

    assert_eq!(
        usage_file::read_for_merge(&path),
        ForMerge::SafeToWrite(Some(sample()))
    );
}

#[test]
fn read_for_merge_of_a_missing_file_is_safe_to_write() {
    let folder = TestFolder::new("usage-merge-missing");

    assert_eq!(
        usage_file::read_for_merge(&folder.file("usage.json")),
        ForMerge::SafeToWrite(None)
    );
}

#[test]
fn read_for_merge_of_unparseable_content_is_safe_to_write_over() {
    let folder = TestFolder::new("usage-merge-unparseable");
    folder.create();
    let path = folder.file("usage.json");
    std::fs::write(&path, "not json").unwrap();

    assert_eq!(
        usage_file::read_for_merge(&path),
        ForMerge::SafeToWrite(None)
    );
}

/// A writer holding the file must not make a merge drop what the file holds.
#[test]
fn read_for_merge_of_a_file_held_by_a_writer_is_not_safe() {
    let folder = TestFolder::new("usage-merge-held");
    let path = folder.file("usage.json");
    usage_file::write(&path, &sample()).unwrap();
    let _held = hold_alone(&path);

    assert_eq!(usage_file::read_for_merge(&path), ForMerge::NotNow);
}

#[test]
fn read_returns_null_for_missing_file() {
    let folder = TestFolder::new("usage-read-missing");

    assert_eq!(usage_file::read(&folder.file("usage.json")), None);
}

#[test]
fn read_returns_null_for_corrupt_file() {
    let folder = TestFolder::new("usage-read-corrupt");
    folder.create();
    let path = folder.file("usage.json");
    std::fs::write(&path, "{").unwrap();

    assert_eq!(usage_file::read(&path), None);
}

#[test]
fn read_returns_null_while_file_is_locked() {
    let folder = TestFolder::new("usage-read-locked");
    let path = folder.file("usage.json");
    usage_file::write(&path, &sample()).unwrap();
    let _locked = hold_alone(&path);

    assert_eq!(usage_file::read(&path), None);
}

// Not in the C# tests.

/// The C# reads past the three bytes some editors put at the start of a UTF-8 file.
#[test]
fn read_skips_a_byte_order_mark() {
    let folder = TestFolder::new("usage-read-bom");
    let path = folder.file("usage.json");
    usage_file::write(&path, &sample()).unwrap();
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(std::fs::read(&path).unwrap());
    std::fs::write(&path, bytes).unwrap();

    assert_eq!(usage_file::read(&path), Some(sample()));
}
