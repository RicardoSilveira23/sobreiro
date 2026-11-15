//! Ported from `tests/Sobreiro.Tests/CopilotCacheTests.cs`. The tests after
//! `default_path_is_under_local_app_data` are new.
//!
//! The cache files here are written by the tests in their own folders, and every figure is
//! made up. A C# `[Theory]` became one test that walks the same rows.
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use sobreiro_core::copilot::CopilotSource;
use sobreiro_core::copilot::copilot_cache;

use crate::{H, M, NONE, S, TestFolder, at_ticks, time};

const HEADER: &str = "// Copilot user cache\n// do not edit\n";

fn business(remaining: i32, timestamp: Option<&str>) -> String {
    let stamp = timestamp.map_or(String::new(), |at| format!(",\"timestamp_utc\":\"{at}\""));
    format!(
        "{{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{{\"premium_interactions\":{{\"entitlement\":15000,\"quota_remaining\":{remaining}{stamp}}}}}}}"
    )
}

const FREE: &str = "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

fn entry(key: &str, retrieved_at: &str, response: &str) -> String {
    format!(
        "\"{key}\":{{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"{retrieved_at}\",\"response\":{response}}}"
    )
}

fn cache(entries: &[String]) -> String {
    format!("{HEADER}{{\"copilotUserCache\":{{{}}}}}", entries.join(","))
}

fn used(text: &str) -> f64 {
    copilot_cache::parse(text).unwrap().used
}

#[test]
fn newest_entry_with_a_budget_wins() {
    let text = cache(&[
        entry("v1:a", "2026-11-10T09:25:00Z", &business(12000, None)),
        entry("v1:b", "2026-11-10T10:00:00Z", &business(10000, None)),
    ]);
    assert!((used(&text) - 5000.0).abs() < 1e-3);
}

#[test]
fn newer_free_entry_does_not_hide_business() {
    let text = cache(&[
        entry("v1:a", "2026-11-10T09:25:00Z", &business(12000, None)),
        entry("v1:b", "2026-11-10T11:00:00Z", FREE),
    ]);
    assert!((used(&text) - 3000.0).abs() < 1e-3);
}

#[test]
fn reading_is_marked_cache() {
    let text = cache(&[entry(
        "v1:a",
        "2026-11-10T09:25:00Z",
        &business(12000, None),
    )]);
    assert_eq!(
        copilot_cache::parse(&text).unwrap().source,
        CopilotSource::Cache
    );
}

#[test]
fn retrieved_at_is_used_when_snapshot_has_no_timestamp() {
    let text = cache(&[entry(
        "v1:a",
        "2026-11-10T09:25:00Z",
        &business(12000, None),
    )]);
    assert_eq!(
        copilot_cache::parse(&text).unwrap().observed_at,
        time("2026-11-10T09:25:00Z")
    );
}

#[test]
fn comment_lines_with_leading_spaces_are_dropped() {
    let text = format!(
        "   // x\n{}",
        cache(&[entry(
            "v1:a",
            "2026-11-10T09:25:00Z",
            &business(12000, None)
        )])
    );
    assert!(copilot_cache::parse(&text).is_some());
}

#[test]
fn no_entries_with_a_budget_gives_none() {
    let text = cache(&[entry("v1:a", "2026-11-10T09:25:00Z", FREE)]);
    assert_eq!(copilot_cache::parse(&text), None);
}

#[test]
fn malformed_or_truncated_cache_gives_none() {
    for text in [
        "",
        "// only a comment",
        "{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"x\",\"response\":{",
    ] {
        assert_eq!(copilot_cache::parse(text), None, "{text}");
    }
}

/// A path in a folder of its own that is never made: nothing is there.
fn nowhere(name: &str) -> PathBuf {
    TestFolder::new(name).file("none.json")
}

#[test]
fn missing_file_gives_none() {
    assert_eq!(copilot_cache::read(&nowhere("copilot_cache-missing")), None);
}

#[test]
fn invalid_path_gives_none() {
    assert_eq!(copilot_cache::read(Path::new("bad\0path.json")), None);
}

#[cfg(windows)]
#[test]
fn locked_file_gives_none() {
    let folder = TestFolder::new("copilot_cache-locked");
    folder.create();
    let path = folder.file("cache.json");
    std::fs::write(
        &path,
        cache(&[entry(
            "v1:a",
            "2026-11-10T09:25:00Z",
            &business(12000, None),
        )]),
    )
    .unwrap();
    let _held = crate::hold_alone(&path);
    assert_eq!(copilot_cache::read(&path), None);
}

#[test]
fn read_parses_an_existing_file() {
    let folder = TestFolder::new("copilot_cache-read");
    folder.create();
    let path = folder.file("cache.json");
    std::fs::write(
        &path,
        cache(&[entry(
            "v1:a",
            "2026-11-10T09:25:00Z",
            &business(12000, None),
        )]),
    )
    .unwrap();
    assert!((copilot_cache::read(&path).unwrap().used - 3000.0).abs() < 1e-3);
}

fn no_environment(_: &str) -> Option<String> {
    None
}

#[test]
fn default_path_is_under_local_app_data() {
    let path = copilot_cache::default_path(
        |name| (name == "LOCALAPPDATA").then(|| r"C:\L".to_string()),
        Path::new(r"C:\Asked"),
    );
    assert_eq!(path, Path::new(r"C:\L\copilot\copilot-user-cache.json"));
}

#[test]
fn default_path_asks_the_folder_handed_in_when_the_variable_is_not_set_or_is_empty() {
    let expected = Path::new(r"C:\Asked\copilot\copilot-user-cache.json");
    assert_eq!(
        copilot_cache::default_path(no_environment, Path::new(r"C:\Asked")),
        expected
    );
    assert_eq!(
        copilot_cache::default_path(
            |name| (name == "LOCALAPPDATA").then(String::new),
            Path::new(r"C:\Asked")
        ),
        expected
    );
}

#[test]
fn of_two_entries_retrieved_together_the_first_wins() {
    let text = cache(&[
        entry("v1:a", "2026-11-10T09:25:00Z", &business(12000, None)),
        entry("v1:b", "2026-11-10T09:25:00Z", &business(10000, None)),
    ]);
    assert!((used(&text) - 3000.0).abs() < 1e-3);
}

#[test]
fn the_snapshots_own_timestamp_wins_over_retrieved_at() {
    let text = cache(&[entry(
        "v1:a",
        "2026-11-10T09:25:00Z",
        &business(12000, Some("2026-11-10T09:20:00Z")),
    )]);
    assert_eq!(
        copilot_cache::parse(&text).unwrap().observed_at,
        time("2026-11-10T09:20:00Z")
    );
}

#[test]
fn entries_that_cannot_be_used_are_skipped() {
    let good = entry("v1:z", "2026-11-10T08:00:00Z", &business(12000, None));
    for bad in [
        // No time, a time that is no text, a time that names no offset, no response, a
        // response that is no object, an entry that is no object.
        "\"v1:a\":{\"response\":".to_string() + &business(1, None) + "}",
        "\"v1:a\":{\"retrievedAt\":5,\"response\":".to_string() + &business(1, None) + "}",
        entry("v1:a", "2026-11-10T10:00:00", &business(1, None)),
        "\"v1:a\":{\"retrievedAt\":\"2026-11-10T10:00:00Z\"}".to_string(),
        entry("v1:a", "2026-11-10T10:00:00Z", "[]"),
        "\"v1:a\":7".to_string(),
    ] {
        let text = cache(&[bad.clone(), good.clone()]);
        assert!((used(&text) - 3000.0).abs() < 1e-3, "{bad}");
    }
}

#[test]
fn a_cache_that_is_not_the_expected_shape_gives_none() {
    for text in [
        "[]",
        "{}",
        "{\"copilotUserCache\":[]}",
        "{\"copilotUserCache\":5}",
    ] {
        assert_eq!(copilot_cache::parse(text), None, "{text}");
    }
}

#[test]
fn a_byte_order_mark_and_windows_line_ends_are_no_trouble() {
    let text = format!(
        "\u{feff}// a\r\n{{\"copilotUserCache\":{{\r\n{}\r\n}}}}\r\n",
        entry("v1:a", "2026-11-10T09:25:00Z", &business(12000, None))
    );
    assert!((used(&text) - 3000.0).abs() < 1e-3);
}

// The C# read a file as text, which drops a byte order mark at its start.
#[test]
fn a_file_that_starts_with_a_byte_order_mark_is_read() {
    let folder = TestFolder::new("copilot_cache-bom");
    folder.create();
    let path = folder.file("cache.json");
    let text = cache(&[entry(
        "v1:a",
        "2026-11-10T09:25:00Z",
        &business(12000, None),
    )]);
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!((copilot_cache::read(&path).unwrap().used - 3000.0).abs() < 1e-3);
}

// The C# took a time with no offset to be on the PC's own clock; Core has no clock to ask.
#[test]
fn an_entry_whose_time_has_no_offset_is_left_out() {
    for at in ["2026-11-10T09:25:00", "2026-11-10T09:25:00.5", "2026-11-10"] {
        let alone = cache(&[entry("v1:a", at, &business(12000, None))]);
        assert_eq!(copilot_cache::parse(&alone), None, "{at}");
        // It does not hide an entry that is good.
        let beside = cache(&[
            entry("v1:a", at, &business(12000, None)),
            entry("v1:b", "2026-11-10T09:00:00Z", &business(10000, None)),
        ]);
        assert!((used(&beside) - 5000.0).abs() < 1e-3, "{at}");
    }
}

// The C# `Parse` threw on such a time, whatever else the file held, and `Read` gave no reading.
#[test]
fn a_time_with_half_a_character_makes_the_whole_file_no_use() {
    let half = entry(
        "v1:a",
        "2026-11-10T09:25:00\\ud800Z",
        &business(12000, None),
    );
    let good = entry("v1:b", "2026-11-10T09:00:00Z", &business(10000, None));
    assert_eq!(
        copilot_cache::parse(&cache(std::slice::from_ref(&half))),
        None
    );
    assert_eq!(
        copilot_cache::parse(&cache(&[half.clone(), good.clone()])),
        None
    );
    assert_eq!(copilot_cache::parse(&cache(&[good, half])), None);
}

// Another program writes the file. The real one is a few thousand bytes.
#[test]
fn a_file_over_a_megabyte_is_not_read() {
    let folder = TestFolder::new("copilot_cache-large");
    folder.create();
    let path = folder.file("cache.json");
    let text = cache(&[entry(
        "v1:a",
        "2026-11-10T09:25:00Z",
        &business(12000, None),
    )]);
    // The same cache, read when a comment line fills it up to the limit and not when the
    // line is one byte longer.
    let room = 1024 * 1024 - text.len() - "//\n".len();
    for (filler, read) in [(room, true), (room + 1, false)] {
        let padded = format!("//{}\n{text}", "x".repeat(filler));
        std::fs::write(&path, padded).unwrap();
        assert_eq!(copilot_cache::read(&path).is_some(), read, "{filler}");
    }
}

#[test]
fn a_file_that_is_not_utf_8_does_not_fail_loudly() {
    let folder = TestFolder::new("copilot_cache-bytes");
    folder.create();
    let path = folder.file("cache.json");
    std::fs::write(&path, [0xff, 0xfe, 0x00, 0x80]).unwrap();
    assert_eq!(copilot_cache::read(&path), None);
}

#[test]
fn a_folder_where_the_file_should_be_gives_none() {
    let folder = TestFolder::new("copilot_cache-folder");
    folder.create();
    assert_eq!(copilot_cache::read(folder.path()), None);
}

// ---- Rows the C# wrote ---------------------------------------------------------------------
//
// Everything below was written by a script from the answers of the C# of 1.1.0, which was run
// on these made-up cache texts on 2026-11-15. Nothing below was typed. A moment is a count of
// ticks from noon on 2026-11-09 (UTC), written with the units of `main.rs`: `5 * M + 1` is five
// minutes and one tick after it. `NONE` stands where there is no moment. A text where the C#
// `Parse` threw (half of a character, written as an escape, in `retrievedAt`) is a row of
// `None`: the C# `Read` turned that into no reading, and this `parse` has no exception to
// turn. That row holds one entry; `a_time_with_half_a_character_makes_the_whole_file_no_use`
// holds the same beside a good entry.
//
// Four texts are left out, where a `retrievedAt` does not say how far it is from UTC (a date
// and time with no offset, one with a fraction and no offset, and a date alone, twice as the
// only entry and once beside an entry that does say). The C# took such a time to be on the PC's
// own clock, so its answer depended on the PC's time zone. Core does not know it: the entry is
// left out, as for a time of Claude Code's reply. `an_entry_whose_time_has_no_offset_is_left_out`
// checks that.
//
// A fifth is left out: one that starts with a byte order mark, which the C# `Parse` gives no
// reading for. `Read` never hands `Parse` such a text, since reading a file as text drops the
// mark first, and this `parse` drops it itself so that `read` does the same: the two agree on
// a file, which `a_file_that_starts_with_a_byte_order_mark_is_read` checks.

/// What the C# read from a text: the budget, what is used, when the month resets and when the
/// entry was retrieved.
type Found = (f64, f64, i64, i64);

/// A moment as such a row holds it: ticks from noon on 2026-11-09. Counted by hand, since
/// `at_ticks` cannot reach the year 1 or the year 9999.
fn ticks(moment: DateTime<Utc>) -> i64 {
    let since = moment - at_ticks(0);
    since.num_seconds() * S + i64::from(since.subsec_nanos()) / 100
}

#[test]
fn parse_answers_as_the_csharp_did() {
    #[rustfmt::skip]
    let rows: &[(&str, Option<Found>)] = &[
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12355.8,\"timestamp_utc\":\"2026-11-10T09:25:19.365Z\"}}}}}}", Some((15000.0, 2644.2000000000007, 516 * H, 771_193_650_000))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 5000.0, 516 * H, 22 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 22 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}}}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T12:00:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}}}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T08:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":9000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:c\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T08:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":9000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":7000}}}},\"v1:c\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":8000}}}}}}", Some((15000.0, 8000.0, 516 * H, 23 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 5000.0, 516 * H, 22 * H))),
        ("// only a comment\n", None),
        ("", None),
        ("\n\n", None),
        ("   // indented comment\n\t// tab comment\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// c\r\n// d\r\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}\r\n", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// c\r\n{\r\n  \"copilotUserCache\": {\r\n    \"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}\r\n  }\r\n}\r\n", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("{\n  // between\n  \"copilotUserCache\": {\n  //x\n\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}\n  }\n}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}} // trailing", None),
        ("{\"copilotUserCache\":{\"v1:a//b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"note\":\"// not a comment\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("//{\"copilotUserCache\":{}}\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("/ not a comment\n{\"copilotUserCache\":{}}", None),
        ("{}", None),
        ("[]", None),
        ("null", None),
        ("\"text\"", None),
        ("12", None),
        ("not json", None),
        ("{\"copilotUserCache\":", None),
        ("{\"other\":{}}", None),
        ("{\"copilotUserCache\":[]}", None),
        ("{\"copilotUserCache\":\"x\"}", None),
        ("{\"copilotUserCache\":null}", None),
        ("{\"copilotUserCache\":{}}", None),
        ("{\"copilotUserCache\":{\"a\":\"x\",\"b\":1,\"c\":null,\"d\":[],\"e\":true,\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("{\"copilotUserCache\":{\"a\":\"x\"}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":1789000000,\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":null,\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":true,\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":{},\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00+02:00\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1165 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00-03:30\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1495 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00.123Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 771_001_230_000))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00.1234567Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 771_001_234_567))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00.12345678Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 771_001_234_567))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00.123456789+01:00\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 735_001_234_567))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10 09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:60Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T24:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-02-30T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"not a time\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10t09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00+0200\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00+02\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1165 * M))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\" 2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z \",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"0001-01-01T00:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, NONE, -17758284 * H))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"9999-12-31T23:59:59Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, NONE, 251608075199 * S))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00.0000000Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:30:00+02:00\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T11:00:00+02:00\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00.1Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00.0999999Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 756_001_000_000))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":null,\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":10000}}}}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"retrievedAt\":\"2026-11-10T10:00:00Z\"}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":null}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":\"x\"}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:00:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}},\"v1:b\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T10:00:00Z\",\"response\":[]}}}", Some((15000.0, 3000.0, 516 * H, 21 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, NONE, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_at\":1796083200,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, NONE, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0}}}}}}", Some((15000.0, 15000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":-250}}}}}}", Some((15000.0, 15250.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":20000}}}}}}", Some((15000.0, -5000.0, 516 * H, 1285 * M))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":15000,\"timestamp_utc\":\"2026-11-10T08:00:00Z\"}}}}}}", Some((15000.0, 0.0, 516 * H, 20 * H))),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"unlimited\":true,\"has_quota\":true,\"remaining\":15000}}}}}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_snapshots\":{}}}}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{}}}}", None),
        ("// Copilot user cache\n// do not edit\n{\"copilotUserCache\":{\"v1:a\":{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"remaining\":12355,\"quota_remaining\":12355.8,\"has_quota\":true,\"timestamp_utc\":\"2026-11-10T09:25:19.365Z\"},\"chat\":{\"entitlement\":100}},\"copilot_plan\":\"business\"}}}}", Some((15000.0, 2644.2000000000007, NONE, 771_193_650_000))),
        ("{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"2026-11-10T09:25:00\\ud800Z\",\"response\":{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", None),
        ("{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"2026-11-10T09:25:00Z\",\"response\":{\"copilot_plan\":\"\\ud800\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}}}}", Some((15000.0, 3000.0, NONE, 1285 * M))),
    ];
    assert_eq!(rows.len(), 85);
    for (index, &(text, expected)) in rows.iter().enumerate() {
        let found = copilot_cache::parse(text);
        match (found, expected) {
            (None, None) => {}
            (Some(reading), Some((budget, used, resets_at, observed_at))) => {
                assert_eq!(reading.budget, budget, "row {index}: budget");
                assert_eq!(reading.used, used, "row {index}: used");
                assert_eq!(
                    reading.resets_at.map_or(NONE, ticks),
                    resets_at,
                    "row {index}: resets_at"
                );
                assert_eq!(
                    ticks(reading.observed_at),
                    observed_at,
                    "row {index}: observed_at"
                );
                assert_eq!(reading.source, CopilotSource::Cache, "row {index}: source");
            }
            (found, expected) => panic!("row {index}: {found:?}, the C# gave {expected:?}"),
        }
    }
}
