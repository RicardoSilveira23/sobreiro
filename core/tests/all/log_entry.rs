//! Ported from `tests/Sobreiro.Tests/LogEntryTests.cs`.
//!
//! Not ported, because Rust has no exceptions and Core walks none:
//! `Format_adds_the_type_and_stack_trace`, `Format_never_includes_the_message`,
//! `Format_walks_inner_and_aggregate_exceptions`, `Format_stops_after_eight_exceptions` and
//! `Format_marks_a_missing_stack_trace`.
use chrono::{DateTime, FixedOffset};
use sobreiro_core::diagnostics::diagnostics_message::{startup_failed, startup_failed_unlogged};
use sobreiro_core::diagnostics::log_entry::{MAX_CHARS, format};

/// 2026-11-09 14:03:12.345 at +01:00.
fn at() -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339("2026-11-09T14:03:12.345+01:00").unwrap()
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The units of an entry's first line before the event, and so how many more fit under the cap
/// once its line break is counted.
fn header_units() -> usize {
    units(&format(at(), 1, "", None)) - 1
}

#[test]
fn format_starts_with_iso_time_pid_and_event() {
    assert_eq!(
        format(at(), 1234, "start version=0.2.0 instance=Sobreiro", None),
        "2026-11-09T14:03:12.345+01:00 pid=1234 start version=0.2.0 instance=Sobreiro\n"
    );
}

#[test]
fn format_flattens_line_breaks_in_the_event() {
    assert!(
        format(at(), 1234, "a\r\nb", None)
            .starts_with("2026-11-09T14:03:12.345+01:00 pid=1234 a  b\n")
    );
}

#[test]
fn format_records_a_non_exception_by_type_only() {
    let text = format(
        at(),
        1,
        "unhandled-exception terminating=true",
        Some("System.String"),
    );
    assert!(text.contains("  System.String\n"));
    assert!(!text.contains("boom"));
}

#[test]
fn format_cuts_an_entry_over_the_cap() {
    let text = format(at(), 1, &"x".repeat(20000), None);
    assert!(units(&text) <= MAX_CHARS + 10);
    assert!(text.ends_with("  (cut)\n"));
}

#[test]
fn startup_failed_names_the_type_and_the_log() {
    assert_eq!(
        startup_failed("InvalidOperationException", r"C:\x\log.txt"),
        "Sobreiro couldn't start (InvalidOperationException).\n\nThe details are in C:\\x\\log.txt"
    );
}

#[test]
fn startup_failed_unlogged_names_the_type_and_the_folder() {
    assert_eq!(
        startup_failed_unlogged("IOException", r"C:\x"),
        "Sobreiro couldn't start (IOException).\n\nIt couldn't write its log either. Check that this folder can be created and written to: C:\\x"
    );
}

// Not in the C# tests.

#[test]
fn a_cut_inside_a_surrogate_pair_leaves_the_replacement_character() {
    // One unit of room is left for a character of two units.
    let event = format!(
        "{}\u{1F600}tail",
        "x".repeat(MAX_CHARS - header_units() - 1)
    );
    let text = format(at(), 1, &event, None);
    assert!(text.ends_with("x\u{FFFD}\n  (cut)\n"));
    assert!(!text.contains('\u{1F600}'));
    assert_eq!(units(&text), MAX_CHARS + "\n  (cut)\n".len());
}

#[test]
fn a_pair_that_fits_exactly_is_kept_whole_before_the_cut() {
    let event = format!(
        "{}\u{1F600}tail",
        "x".repeat(MAX_CHARS - header_units() - 2)
    );
    let text = format(at(), 1, &event, None);
    assert!(text.ends_with("x\u{1F600}\n  (cut)\n"));
}

#[test]
fn an_entry_of_exactly_the_cap_is_not_cut() {
    let event = "x".repeat(MAX_CHARS - header_units() - 1);
    let text = format(at(), 1, &event, None);
    assert_eq!(units(&text), MAX_CHARS);
    assert!(!text.contains("(cut)"));
}

#[test]
fn an_offset_west_of_utc_is_written_with_a_minus() {
    let west = DateTime::parse_from_rfc3339("2026-11-09T14:03:12.005-05:30").unwrap();
    assert!(format(west, 7, "e", None).starts_with("2026-11-09T14:03:12.005-05:30 pid=7 e\n"));
}
