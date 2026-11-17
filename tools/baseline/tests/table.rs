//! The table row and where it goes. Ported from 1.x's BaselineTableTests.

use sobreiro_baseline::{BaselineRun, DIVIDER, Date, HEADER, IdleFigures, add, row};
use std::time::Duration;

const MB: u64 = 1024 * 1024;

fn run() -> BaselineRun {
    BaselineRun {
        build: "0.10.0".to_string(),
        date: Date {
            year: 2026,
            month: 11,
            day: 13,
        },
        idle: Some(IdleFigures {
            private_bytes: 171 * MB + 300_000,
            working_set_bytes: 301 * MB,
            cpu_seconds_per_hour: 4.24,
        }),
        first_pill_first_start: Some(Duration::from_millis(2314)),
        first_pill_later_starts: Some(Duration::from_millis(1118)),
        tray_exe_bytes: Some(80 * MB + 100_000),
        bridge_exe_bytes: Some(17 * MB + 800_000),
        bridge_run: Some(Duration::from_secs_f64(0.0616)),
    }
}

const RUN_ROW: &str = "| 0.10.0 | 2026-11-13 | 171 MB | 301 MB | 4.2 s | 2.31 s | 1.12 s | 80.1 MB | 17.8 MB | 62 ms |";

fn count(text: &str) -> usize {
    text.matches('|').count()
}

fn doc(release_rows: &[&str]) -> String {
    let mut lines = vec!["# Baseline", "", "## Releases", "", HEADER, DIVIDER];
    lines.extend_from_slice(release_rows);
    lines.extend_from_slice(&[
        "",
        "## Experiments",
        "",
        HEADER,
        DIVIDER,
        "| 0.10.0, other | x |",
        "",
    ]);
    lines.join("\n")
}

#[test]
fn row_has_one_cell_per_header_column() {
    assert_eq!(row(&run()), RUN_ROW);
    assert_eq!(count(HEADER), count(RUN_ROW));
    assert_eq!(count(HEADER), count(DIVIDER));
}

#[test]
fn row_says_what_was_not_measured() {
    let r = BaselineRun {
        first_pill_first_start: None,
        first_pill_later_starts: None,
        bridge_exe_bytes: None,
        bridge_run: None,
        ..run()
    };
    assert_eq!(
        row(&r),
        "| 0.10.0 | 2026-11-13 | 171 MB | 301 MB | 4.2 s | not seen | not seen | 80.1 MB | none | none |"
    );
}

#[test]
fn row_of_a_bridge_only_run_leaves_the_tray_unmeasured() {
    let r = BaselineRun {
        idle: None,
        first_pill_first_start: None,
        first_pill_later_starts: None,
        tray_exe_bytes: None,
        ..run()
    };
    assert_eq!(
        row(&r),
        "| 0.10.0 | 2026-11-13 | not measured | not measured | not measured | not measured | not measured | not measured | 17.8 MB | 62 ms |"
    );
}

#[test]
fn add_appends_to_the_table_under_the_heading() {
    assert_eq!(
        add(&doc(&["| 0.9.0 | x |"]), "Releases", &run()),
        Some(doc(&["| 0.9.0 | x |", RUN_ROW]))
    );
}

#[test]
fn add_fills_an_empty_table() {
    assert_eq!(add(&doc(&[]), "Releases", &run()), Some(doc(&[RUN_ROW])));
}

#[test]
fn add_replaces_the_row_of_the_same_build() {
    let before = doc(&["| 0.9.0 | x |", "| 0.10.0 | old |", "| 0.11.0 | x |"]);
    assert_eq!(
        add(&before, "Releases", &run()),
        Some(doc(&["| 0.9.0 | x |", RUN_ROW, "| 0.11.0 | x |"]))
    );
}

#[test]
fn add_leaves_a_build_whose_name_only_starts_the_same() {
    let added = add(&doc(&[]), "Experiments", &run());
    let wanted = format!("| 0.10.0, other | x |\n{RUN_ROW}\n");
    assert!(added.is_some_and(|a| a.contains(&wanted)));
}

#[test]
fn add_keeps_windows_line_endings() {
    let before = doc(&["| 0.9.0 | x |"]).replace('\n', "\r\n");
    let wanted = doc(&["| 0.9.0 | x |", RUN_ROW]).replace('\n', "\r\n");
    assert_eq!(add(&before, "Releases", &run()), Some(wanted));
}

#[test]
fn add_without_the_heading_or_its_table_changes_nothing() {
    assert_eq!(add(&doc(&[]), "Nightlies", &run()), None);
    assert_eq!(
        add(
            "# Baseline\n\n## Releases\n\nNo table yet.\n",
            "Releases",
            &run()
        ),
        None
    );
}
