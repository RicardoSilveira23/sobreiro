//! Not ported from a C# test: files that 1.1.0 itself wrote, in `fixtures/from-1.1/`.
//!
//! Each file that 1.1 writes is read here to the values the C# read from it, and written back
//! to the same bytes. Where 1.1 changes a file (a setting, a reading merged in, an alert
//! marked), the file comes as a pair, `NAME.before.json` and `NAME.after.json`: Core loads the
//! first, makes the same change, and writes the second to the byte. A merge pair also holds the
//! reading that was merged, as `NAME.reading.json`. Each test is named after its fixture.
//! `README.md` beside this folder says how the files were made.
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use sobreiro_core::alerts::alert_policy;
use sobreiro_core::alerts::alert_state::{AlertMark, AlertState};
use sobreiro_core::colours::BarColourOption;
use sobreiro_core::config::{AppConfig, PreviousStatusLine, ShowOption};
use sobreiro_core::display::{FlyoutStyle, PaceDays, Zone};
use sobreiro_core::limits::{Limit, LimitKind};
use sobreiro_core::live::usage_api_json;
use sobreiro_core::usage::rate_limits_json;
use sobreiro_core::usage::usage_merge::{self, ReadingFrom};
use sobreiro_core::usage::{RateLimits, UsageSnapshot, usage_file};

use crate::{TestFolder, en_us, spend_seen, unix, window, window_seen};

// ---------------------------------------------------------------- helpers

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("from-1.1")
        .join(name)
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

fn fixture_text(name: &str) -> String {
    String::from_utf8(fixture(name)).unwrap()
}

/// Compares bytes as text, so that a failure shows where they differ.
fn assert_same_bytes(written: &[u8], expected: &[u8]) {
    assert_eq!(
        String::from_utf8_lossy(written),
        String::from_utf8_lossy(expected)
    );
    assert_eq!(written, expected);
}

// ---------------------------------------------------------------- config.json

fn config_with(change: impl FnOnce(&mut AppConfig)) -> AppConfig {
    let mut config = AppConfig::default();
    change(&mut config);
    config
}

/// Core reads the file to `expected`, and writes `expected` back to the file's bytes.
fn config_reads(name: &str, expected: AppConfig) {
    let loaded = AppConfig::load(&fixture_path(name)).unwrap();
    assert_eq!(loaded, expected);
    assert_same_bytes(loaded.to_json().unwrap().as_bytes(), &fixture(name));
}

const PREVIOUS: &str = r#"{ "type": "command", "command": "node status.js", "padding": 1 }"#;

fn unknown_keys() -> Vec<(String, String)> {
    [
        ("future_theme", r#""dusk""#),
        ("future_list", "[1,  2.50, null]"),
        ("future_nested", r#"{"a": {"b": true}, "c": "x"}"#),
    ]
    .into_iter()
    .map(|(key, raw)| (key.to_string(), raw.to_string()))
    .collect()
}

#[test]
fn config_style_bars() {
    config_reads("config-style-bars.json", AppConfig::default());
}

#[test]
fn config_style_battery() {
    config_reads(
        "config-style-battery.json",
        config_with(|c| c.style = FlyoutStyle::Battery),
    );
}

#[test]
fn config_style_thermometer() {
    config_reads(
        "config-style-thermometer.json",
        config_with(|c| c.style = FlyoutStyle::Thermometer),
    );
}

#[test]
fn config_bar_colour_purple() {
    config_reads("config-bar-colour-purple.json", AppConfig::default());
}

#[test]
fn config_bar_colour_accent() {
    config_reads(
        "config-bar-colour-accent.json",
        config_with(|c| c.bar_colour = BarColourOption::Accent),
    );
}

#[test]
fn config_show_trayandpill() {
    config_reads("config-show-trayandpill.json", AppConfig::default());
}

#[test]
fn config_show_trayonly() {
    config_reads(
        "config-show-trayonly.json",
        config_with(|c| c.show = ShowOption::TrayOnly),
    );
}

#[test]
fn config_pace_everyday() {
    config_reads("config-pace-everyday.json", AppConfig::default());
}

#[test]
fn config_pace_weekdays() {
    config_reads(
        "config-pace-weekdays.json",
        config_with(|c| c.pace_days = PaceDays::Weekdays),
    );
}

#[test]
fn config_connect_previous_status_line() {
    config_reads(
        "config-connect-previous-status-line.json",
        config_with(|c| {
            c.chained_command = Some("node status.js".to_string());
            c.previous = Some(PreviousStatusLine {
                raw_json: Some(PREVIOUS.to_string()),
            });
        }),
    );
}

#[test]
fn config_connect_no_previous_status_line() {
    config_reads(
        "config-connect-no-previous-status-line.json",
        config_with(|c| c.previous = Some(PreviousStatusLine { raw_json: None })),
    );
}

#[test]
fn config_chained_command() {
    config_reads(
        "config-chained-command.json",
        config_with(|c| c.chained_command = Some("node status.js".to_string())),
    );
}

#[test]
fn config_unknown_keys() {
    config_reads(
        "config-unknown-keys.json",
        config_with(|c| c.extra = unknown_keys()),
    );
}

/// The config of every `config-change-*` pair before its change: a previous status line, a
/// chained command, a value off the default for each setting, and keys 1.1 does not know.
fn config_before() -> AppConfig {
    AppConfig {
        chained_command: Some("node status.js".to_string()),
        previous: Some(PreviousStatusLine {
            raw_json: Some(PREVIOUS.to_string()),
        }),
        style: FlyoutStyle::Thermometer,
        bar_colour: BarColourOption::Accent,
        show: ShowOption::TrayOnly,
        pace_days: PaceDays::Weekdays,
        extra: unknown_keys(),
    }
}

/// Loads `before`, makes the change, saves, and compares with `after`.
fn config_pair(name: &str, change: impl FnOnce(&mut AppConfig)) {
    let before =
        AppConfig::load(&fixture_path(&format!("config-change-{name}.before.json"))).unwrap();
    assert_eq!(before, config_before());

    let mut config = before;
    change(&mut config);
    let folder = TestFolder::new(&format!("from-1-1-config-change-{name}"));
    let path = folder.file("config.json");
    config.save(&path).unwrap();
    assert_same_bytes(
        &std::fs::read(&path).unwrap(),
        &fixture(&format!("config-change-{name}.after.json")),
    );
}

#[test]
fn config_change_style() {
    config_pair("style", |c| c.style = FlyoutStyle::Battery);
}

#[test]
fn config_change_bar_colour() {
    config_pair("bar-colour", |c| c.bar_colour = BarColourOption::Purple);
}

#[test]
fn config_change_show() {
    config_pair("show", |c| c.show = ShowOption::TrayAndPill);
}

#[test]
fn config_change_pace() {
    config_pair("pace", |c| c.pace_days = PaceDays::EveryDay);
}

// ---------------------------------------------------------------- usage.json

const FIVE_RESET: i64 = 1_790_010_000;
const SEVEN_RESET: i64 = 1_790_500_000;
const SPEND_RESET: i64 = 1_792_000_000;
const T0: i64 = 1_790_000_000;
const NOW: i64 = 1_790_000_600;

/// Core reads the file to `expected`, and writes `expected` back to the file's bytes.
fn usage_reads(name: &str, expected: UsageSnapshot) {
    let loaded = usage_file::read(&fixture_path(name)).unwrap();
    assert_eq!(loaded, expected);
    let json = rate_limits_json::write_usage_file(&loaded).unwrap();
    assert_same_bytes(json.as_bytes(), &fixture(name));
}

#[test]
fn usage_windows_only() {
    usage_reads(
        "usage-windows-only.json",
        UsageSnapshot {
            limits: RateLimits {
                five_hour: Some(window_seen(42.5, Some(unix(FIVE_RESET)), unix(T0))),
                seven_day: Some(window(18.0, Some(unix(SEVEN_RESET)))),
                spend: None,
            },
            observed_at: unix(T0),
        },
    );
}

#[test]
fn usage_spend_limit() {
    usage_reads(
        "usage-spend-limit.json",
        UsageSnapshot {
            limits: RateLimits {
                five_hour: Some(window_seen(61.0, Some(unix(FIVE_RESET)), unix(T0))),
                seven_day: None,
                spend: Some(spend_seen(
                    27.5,
                    Some(unix(SPEND_RESET)),
                    Some(55.0),
                    Some(200.0),
                    Some("monthly"),
                    unix(T0),
                )),
            },
            observed_at: unix(T0),
        },
    );
}

/// The snapshot every `usage-*` pair starts from, as its `before` file holds it: windows with
/// and without their own time, and a spend limit.
fn stored() -> UsageSnapshot {
    UsageSnapshot {
        limits: RateLimits {
            five_hour: Some(window(60.0, Some(unix(FIVE_RESET)))),
            seven_day: Some(window_seen(30.0, Some(unix(SEVEN_RESET)), unix(T0))),
            spend: Some(spend_seen(
                40.0,
                Some(unix(SPEND_RESET)),
                Some(40.0),
                Some(100.0),
                Some("monthly"),
                unix(T0),
            )),
        },
        observed_at: unix(T0 - 100),
    }
}

fn status_line(text: &str) -> Option<RateLimits> {
    rate_limits_json::parse_status_line(text)
}

/// Loads `before` and the reading, each with its own parser, merges as the bridge or Live usage
/// does, writes, and compares with `after`. Gives back what was merged.
fn merge_pair(
    name: &str,
    parse: fn(&str) -> Option<RateLimits>,
    from: ReadingFrom,
    now: i64,
) -> UsageSnapshot {
    let before = usage_file::read(&fixture_path(&format!("{name}.before.json"))).unwrap();
    assert_eq!(before, stored());
    let reading = parse(&fixture_text(&format!("{name}.reading.json"))).unwrap();

    let merged = usage_merge::merge(Some(&before), &reading, unix(now), from);
    let folder = TestFolder::new(&format!("from-1-1-{name}"));
    let path = folder.file("usage.json");
    usage_file::write(&path, &merged).unwrap();
    assert_same_bytes(
        &std::fs::read(&path).unwrap(),
        &fixture(&format!("{name}.after.json")),
    );
    // What 1.1 wrote is read back to what Core merged.
    assert_eq!(
        usage_file::read(&fixture_path(&format!("{name}.after.json"))).unwrap(),
        merged
    );
    merged
}

#[test]
fn usage_status_line_window_kept() {
    let merged = merge_pair(
        "usage-status-line-window-kept",
        status_line,
        ReadingFrom::StatusLine,
        NOW,
    );
    // Both readings were lower than what is stored: the stored windows stay, and the one
    // that had no time of its own takes the stored snapshot's.
    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(60.0, Some(unix(FIVE_RESET)), unix(T0 - 100)))
    );
    assert_eq!(
        merged.limits.seven_day,
        Some(window_seen(30.0, Some(unix(SEVEN_RESET)), unix(T0)))
    );
    assert_eq!(merged.observed_at, unix(NOW));
}

#[test]
fn usage_status_line_window_replaced() {
    let merged = merge_pair(
        "usage-status-line-window-replaced",
        status_line,
        ReadingFrom::StatusLine,
        NOW,
    );
    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(75.5, Some(unix(FIVE_RESET)), unix(NOW)))
    );
    // A new period replaces the window though its percentage is lower.
    assert_eq!(
        merged.limits.seven_day,
        Some(window_seen(5.0, Some(unix(1_791_104_800)), unix(NOW)))
    );
    // The reading has no spend: the stored one stays.
    assert_eq!(merged.limits.spend, stored().limits.spend);
}

#[test]
fn usage_status_line_spend_cap_changed() {
    let merged = merge_pair(
        "usage-status-line-spend-cap-changed",
        status_line,
        ReadingFrom::StatusLine,
        NOW,
    );
    assert_eq!(
        merged.limits.spend,
        Some(spend_seen(
            25.0,
            Some(unix(SPEND_RESET)),
            Some(50.0),
            Some(200.0),
            Some("monthly"),
            unix(NOW),
        ))
    );
}

#[test]
fn usage_status_line_window_dropped() {
    let merged = merge_pair(
        "usage-status-line-window-dropped",
        status_line,
        ReadingFrom::StatusLine,
        FIVE_RESET + 60,
    );
    // The session window is not in the reading and its reset time has passed.
    assert_eq!(merged.limits.five_hour, None);
}

#[test]
fn usage_live_usage_window_replaced() {
    let merged = merge_pair(
        "usage-live-usage-window-replaced",
        usage_api_json::parse,
        ReadingFrom::LiveUsage,
        NOW,
    );
    // Anthropic's own reading replaces the stored windows, though lower; spend stays.
    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(30.0, Some(unix(FIVE_RESET)), unix(NOW)))
    );
    assert_eq!(
        merged.limits.seven_day,
        Some(window_seen(12.5, Some(unix(SEVEN_RESET)), unix(NOW)))
    );
    assert_eq!(merged.limits.spend, stored().limits.spend);
}

#[test]
fn usage_live_usage_window_kept() {
    let merged = merge_pair(
        "usage-live-usage-window-kept",
        usage_api_json::parse,
        ReadingFrom::LiveUsage,
        NOW,
    );
    // The reading's session period is an earlier one than the stored: the stored stays.
    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(60.0, Some(unix(FIVE_RESET)), unix(T0 - 100)))
    );
    assert_eq!(
        merged.limits.seven_day,
        Some(window_seen(33.0, Some(unix(SEVEN_RESET)), unix(NOW)))
    );
}

// ---------------------------------------------------------------- alerts.json

fn state_of(marks: &[(LimitKind, Option<i64>, i32)]) -> AlertState {
    AlertState {
        marks: marks
            .iter()
            .map(|(kind, resets_at, threshold)| {
                (
                    *kind,
                    AlertMark {
                        resets_at: resets_at.map(unix),
                        threshold: *threshold,
                    },
                )
            })
            .collect(),
    }
}

/// Core reads the file to the marks, and writes them back to the file's bytes.
fn alerts_read(name: &str, marks: &[(LimitKind, Option<i64>, i32)]) {
    let loaded = AlertState::load(&fixture_path(name));
    assert_eq!(loaded, state_of(marks));

    let folder = TestFolder::new(&format!("from-1-1-{name}"));
    let path = folder.file("alerts.json");
    loaded.save(&path).unwrap();
    assert_same_bytes(&std::fs::read(&path).unwrap(), &fixture(name));
}

macro_rules! alerts_mark {
    ($test:ident, $file:literal, $kind:expr, $resets:expr, $threshold:expr) => {
        #[test]
        fn $test() {
            alerts_read($file, &[($kind, $resets, $threshold)]);
        }
    };
}

const MARK_RESET: Option<i64> = Some(1_790_400_000);

alerts_mark!(
    alerts_session_80_resets,
    "alerts-session-80-resets.json",
    LimitKind::Session,
    MARK_RESET,
    80
);
alerts_mark!(
    alerts_session_80_no_resets,
    "alerts-session-80-no-resets.json",
    LimitKind::Session,
    None,
    80
);
alerts_mark!(
    alerts_session_95_resets,
    "alerts-session-95-resets.json",
    LimitKind::Session,
    MARK_RESET,
    95
);
alerts_mark!(
    alerts_session_95_no_resets,
    "alerts-session-95-no-resets.json",
    LimitKind::Session,
    None,
    95
);
alerts_mark!(
    alerts_weekly_80_resets,
    "alerts-weekly-80-resets.json",
    LimitKind::Weekly,
    MARK_RESET,
    80
);
alerts_mark!(
    alerts_weekly_80_no_resets,
    "alerts-weekly-80-no-resets.json",
    LimitKind::Weekly,
    None,
    80
);
alerts_mark!(
    alerts_weekly_95_resets,
    "alerts-weekly-95-resets.json",
    LimitKind::Weekly,
    MARK_RESET,
    95
);
alerts_mark!(
    alerts_weekly_95_no_resets,
    "alerts-weekly-95-no-resets.json",
    LimitKind::Weekly,
    None,
    95
);
alerts_mark!(
    alerts_copilot_credits_80_resets,
    "alerts-copilot-credits-80-resets.json",
    LimitKind::Credits,
    MARK_RESET,
    80
);
alerts_mark!(
    alerts_copilot_credits_80_no_resets,
    "alerts-copilot-credits-80-no-resets.json",
    LimitKind::Credits,
    None,
    80
);
alerts_mark!(
    alerts_copilot_credits_95_resets,
    "alerts-copilot-credits-95-resets.json",
    LimitKind::Credits,
    MARK_RESET,
    95
);
alerts_mark!(
    alerts_copilot_credits_95_no_resets,
    "alerts-copilot-credits-95-no-resets.json",
    LimitKind::Credits,
    None,
    95
);
alerts_mark!(
    alerts_spend_80_resets,
    "alerts-spend-80-resets.json",
    LimitKind::Spend,
    MARK_RESET,
    80
);
alerts_mark!(
    alerts_spend_80_no_resets,
    "alerts-spend-80-no-resets.json",
    LimitKind::Spend,
    None,
    80
);
alerts_mark!(
    alerts_spend_95_resets,
    "alerts-spend-95-resets.json",
    LimitKind::Spend,
    MARK_RESET,
    95
);
alerts_mark!(
    alerts_spend_95_no_resets,
    "alerts-spend-95-no-resets.json",
    LimitKind::Spend,
    None,
    95
);

#[test]
fn alerts_all_kinds() {
    alerts_read(
        "alerts-all-kinds.json",
        &[
            (LimitKind::Session, Some(FIVE_RESET), 80),
            (LimitKind::Weekly, Some(SEVEN_RESET), 95),
            (LimitKind::Spend, None, 80),
            (LimitKind::Credits, Some(SPEND_RESET), 95),
        ],
    );
}

fn limit(kind: LimitKind, label: &'static str, percent: f64, resets: Option<i64>) -> Limit {
    let now: DateTime<Utc> = unix(NOW);
    Limit::new(
        "Claude Code",
        kind,
        label,
        percent,
        resets.map(unix),
        now,
        false,
    )
}

/// Loads `before`, checks the limits as 1.1 does, saves the state it asks to keep, and
/// compares with `after`.
fn alerts_pair(
    name: &str,
    before: &[(LimitKind, Option<i64>, i32)],
    limits: &[Limit],
) -> AlertState {
    let loaded = AlertState::load(&fixture_path(&format!("{name}.before.json")));
    assert_eq!(loaded, state_of(before));

    let (_alerts, next) = alert_policy::check(
        limits,
        &loaded,
        unix(NOW),
        &Zone::utc(),
        &en_us(),
        PaceDays::EveryDay,
    );
    let next = next.expect("an alert was due");
    let folder = TestFolder::new(&format!("from-1-1-{name}"));
    let path = folder.file("alerts.json");
    next.save(&path).unwrap();
    assert_same_bytes(
        &std::fs::read(&path).unwrap(),
        &fixture(&format!("{name}.after.json")),
    );
    next
}

#[test]
fn alerts_mark_raised() {
    // Same period, higher threshold: the session mark goes from 80 to 95. The weekly window
    // crosses nothing new.
    alerts_pair(
        "alerts-mark-raised",
        &[
            (LimitKind::Session, Some(FIVE_RESET), 80),
            (LimitKind::Weekly, Some(SEVEN_RESET), 80),
        ],
        &[
            limit(LimitKind::Session, "Session", 96.0, Some(FIVE_RESET)),
            limit(LimitKind::Weekly, "Weekly", 85.0, Some(SEVEN_RESET)),
        ],
    );
}

#[test]
fn alerts_mark_new_period() {
    // A new reset period replaces the session mark though it is lower; spend, with no reset
    // time, goes from 80 to 95.
    alerts_pair(
        "alerts-mark-new-period",
        &[
            (LimitKind::Session, Some(FIVE_RESET), 95),
            (LimitKind::Spend, None, 80),
        ],
        &[
            limit(
                LimitKind::Session,
                "Session",
                82.0,
                Some(FIVE_RESET + 18_000),
            ),
            limit(LimitKind::Spend, "Spend", 99.0, None),
        ],
    );
}

#[test]
fn alerts_mark_added() {
    // Limits with no mark yet gain one each; the file lists them in its own order.
    alerts_pair(
        "alerts-mark-added",
        &[(LimitKind::Session, Some(FIVE_RESET), 80)],
        &[
            limit(LimitKind::Session, "Session", 81.0, Some(FIVE_RESET)),
            limit(LimitKind::Weekly, "Weekly", 90.0, Some(SEVEN_RESET)),
            limit(LimitKind::Credits, "AI credits", 95.0, Some(SPEND_RESET)),
            limit(LimitKind::Spend, "Spend", 80.0, None),
        ],
    );
}
