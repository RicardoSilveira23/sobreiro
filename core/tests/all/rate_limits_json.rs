//! Ported from `tests/Sobreiro.Tests/RateLimitsJsonTests.cs`. The tests after the line "Not in
//! the C# tests" are new: they pin down things the C# does that no C# test names.
use sobreiro_core::usage::{RateLimits, UsageSnapshot, rate_limits_json};

use crate::{spend, spend_seen, unix, window, window_seen};

/// What Claude Code gives a status line command, with fields the bridge must never write.
const FULL: &str = r#"{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/p",
 "rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},
                "seven_day":{"used_percentage":41.2,"resets_at":1738857600},
                "spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly"}}}"#;

/// The limits in `FULL`, read at the given second.
fn full_snapshot(observed_at: i64) -> UsageSnapshot {
    UsageSnapshot {
        limits: rate_limits_json::parse_status_line(FULL).unwrap(),
        observed_at: unix(observed_at),
    }
}

#[test]
fn parse_status_line_reads_all_three_windows() {
    let limits = rate_limits_json::parse_status_line(FULL).unwrap();

    assert_eq!(limits.five_hour, Some(window(23.5, Some(unix(1738425600)))));
    assert_eq!(limits.seven_day, Some(window(41.2, Some(unix(1738857600)))));
    assert_eq!(
        limits.spend,
        Some(spend(
            62.8,
            Some(unix(1740787200)),
            Some(314.12),
            Some(500.0),
            Some("monthly")
        ))
    );
}

#[test]
fn parse_status_line_returns_null_without_rate_limits() {
    assert_eq!(
        rate_limits_json::parse_status_line(r#"{"session_id":"abc"}"#),
        None
    );
}

#[test]
fn parse_status_line_returns_null_for_non_json() {
    for input in ["not json", "", "[]"] {
        assert_eq!(rate_limits_json::parse_status_line(input), None, "{input}");
    }
}

#[test]
fn parse_status_line_returns_null_when_no_window_has_a_percentage() {
    assert_eq!(
        rate_limits_json::parse_status_line(r#"{"rate_limits":{}}"#),
        None
    );
}

#[test]
fn parse_status_line_keeps_windows_independently() {
    let limits = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"seven_day":{"used_percentage":10,"resets_at":1738857600}}}"#,
    )
    .unwrap();

    assert_eq!(limits.five_hour, None);
    assert_eq!(limits.seven_day, Some(window(10.0, Some(unix(1738857600)))));
}

#[test]
fn parse_status_line_drops_window_with_non_numeric_percentage() {
    let limits = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"five_hour":{"used_percentage":"8","resets_at":1738425600},"seven_day":{"used_percentage":5}}}"#,
    )
    .unwrap();

    assert_eq!(limits.five_hour, None);
    assert!(limits.seven_day.is_some());
}

#[test]
fn parse_status_line_keeps_window_without_valid_resets_at() {
    for resets_at in ["null", "\"soon\""] {
        let json = "{\"rate_limits\":{\"five_hour\":{\"used_percentage\":8,\"resets_at\":"
            .to_string()
            + resets_at
            + "}}}";

        let limits = rate_limits_json::parse_status_line(&json).unwrap();

        assert_eq!(limits.five_hour, Some(window(8.0, None)), "{resets_at}");
    }
}

#[test]
fn parse_status_line_drops_unknown_period() {
    let limits = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"spend_limit":{"used_percentage":50,"period":"hourly"}}}"#,
    )
    .unwrap();

    assert_eq!(limits.spend, Some(spend(50.0, None, None, None, None)));
}

#[test]
fn write_usage_file_contains_only_rate_limit_fields_and_timestamp() {
    let json = rate_limits_json::write_usage_file(&full_snapshot(1738420000)).unwrap();

    assert!(!json.contains("session_id"));
    assert!(!json.contains("transcript_path"));
    assert!(!json.contains("cwd"));
    let document: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut names: Vec<&String> = document.as_object().unwrap().keys().collect();
    names.sort();
    assert_eq!(names, ["observed_at", "rate_limits"]);
}

#[test]
fn usage_file_round_trips() {
    let snapshot = full_snapshot(1738420000);

    let json = rate_limits_json::write_usage_file(&snapshot).unwrap();

    assert_eq!(rate_limits_json::parse_usage_file(&json), Some(snapshot));
}

#[test]
fn parse_usage_file_returns_null_without_observed_at_or_rate_limits() {
    let rows = [
        r#"{"rate_limits":{"five_hour":{"used_percentage":1}}}"#,
        r#"{"observed_at":1738420000}"#,
        r#"{"observed_at":1738420000,"rate_limits":{}}"#,
        "nope",
    ];
    for input in rows {
        assert_eq!(rate_limits_json::parse_usage_file(input), None, "{input}");
    }
}

#[test]
fn write_usage_file_and_parse_usage_file_round_trip_window_observed_at() {
    let at = unix(1738400000);
    let snapshot = UsageSnapshot {
        limits: RateLimits {
            five_hour: Some(window_seen(10.0, Some(unix(1738425600)), at)),
            seven_day: Some(window(20.0, Some(unix(1738857600)))),
            spend: Some(spend_seen(
                30.0,
                Some(unix(1740787200)),
                Some(3.0),
                Some(10.0),
                Some("monthly"),
                at,
            )),
        },
        observed_at: unix(1738500000),
    };

    let json = rate_limits_json::write_usage_file(&snapshot).unwrap();

    let document: serde_json::Value = serde_json::from_str(&json).unwrap();
    let rate_limits = &document["rate_limits"];
    assert_eq!(
        rate_limits["five_hour"]["observed_at"].as_i64(),
        Some(1738400000)
    );
    assert!(rate_limits["seven_day"].get("observed_at").is_none());
    assert_eq!(rate_limits_json::parse_usage_file(&json), Some(snapshot));
}

#[test]
fn parse_usage_file_reads_a_milestone_1_file_without_window_times() {
    let snapshot = rate_limits_json::parse_usage_file(
        r#"{"observed_at":1738500000,"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600}}}"#,
    )
    .unwrap();

    assert_eq!(snapshot.limits.five_hour.unwrap().observed_at, None);
}

#[test]
fn parse_status_line_ignores_observed_at_inside_windows() {
    let limits = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600,"observed_at":1738400000},"spend_limit":{"used_percentage":5,"observed_at":1738400000}}}"#,
    )
    .unwrap();

    assert_eq!(limits.five_hour.unwrap().observed_at, None);
    assert_eq!(limits.spend.unwrap().observed_at, None);
}

// Not in the C# tests.

/// An installed user's `usage.json` must not change with the language. This is the file the C#
/// bridge of 1.1.0 wrote on 2026-11-14 when given `FULL`, in a data folder of its own: read here
/// and written again, it comes out the same to the byte.
#[test]
fn write_usage_file_gives_the_bytes_the_csharp_bridge_writes() {
    let written_by_csharp = r#"{"observed_at":1794676233,"rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600,"observed_at":1794676233},"seven_day":{"used_percentage":41.2,"resets_at":1738857600,"observed_at":1794676233},"spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly","observed_at":1794676233}}}"#;

    let snapshot = rate_limits_json::parse_usage_file(written_by_csharp).unwrap();

    assert_eq!(
        rate_limits_json::write_usage_file(&snapshot).unwrap(),
        written_by_csharp
    );
}

/// The C# writer throws on a number that JSON cannot hold, so nothing is written.
#[test]
fn write_usage_file_refuses_a_number_json_cannot_hold() {
    let mut snapshot = full_snapshot(1738420000);
    snapshot.limits.five_hour = Some(window(f64::NAN, None));
    assert!(rate_limits_json::write_usage_file(&snapshot).is_err());

    let mut snapshot = full_snapshot(1738420000);
    snapshot.limits.spend = Some(spend(50.0, None, Some(f64::INFINITY), None, None));
    assert!(rate_limits_json::write_usage_file(&snapshot).is_err());
}

/// The C# cuts a time with a fraction down to its whole second.
#[test]
fn a_time_with_a_fraction_is_cut_to_the_second() {
    let limits = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"five_hour":{"used_percentage":8,"resets_at":1738425600.9}}}"#,
    )
    .unwrap();

    assert_eq!(limits.five_hour, Some(window(8.0, Some(unix(1738425600)))));
}

/// The C# holds no date past the year 9999 or before the year 1: such a reset time is dropped,
/// and a file observed at such a time is not read.
#[test]
fn a_time_outside_the_years_1_to_9999_is_not_a_time() {
    for resets_at in ["253402300800", "-62135596801", "1e30"] {
        let json = "{\"rate_limits\":{\"five_hour\":{\"used_percentage\":8,\"resets_at\":"
            .to_string()
            + resets_at
            + "}}}";

        let limits = rate_limits_json::parse_status_line(&json).unwrap();

        assert_eq!(limits.five_hour, Some(window(8.0, None)), "{resets_at}");
    }
    assert_eq!(
        rate_limits_json::parse_usage_file(
            r#"{"observed_at":253402300800,"rate_limits":{"five_hour":{"used_percentage":1}}}"#
        ),
        None
    );
    // The last second the C# holds is still read.
    let last = rate_limits_json::parse_status_line(
        r#"{"rate_limits":{"five_hour":{"used_percentage":8,"resets_at":253402300799}}}"#,
    )
    .unwrap();
    assert_eq!(last.five_hour, Some(window(8.0, Some(unix(253402300799)))));
}
