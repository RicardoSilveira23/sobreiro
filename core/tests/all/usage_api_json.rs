//! Ported from `tests/Sobreiro.Tests/UsageApiJsonTests.cs`. The tests after
//! `a_window_missing_from_the_list_comes_from_the_keys` are new.
use sobreiro_core::live::usage_api_json;
use sobreiro_core::usage::LimitWindow;

use crate::time;

const LIST_SHAPE: &str = concat!(
    "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"},",
    "{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"},",
    "{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}]}",
);

#[test]
fn the_list_shape_gives_session_and_weekly_and_ignores_scoped_entries() {
    let limits = usage_api_json::parse(LIST_SHAPE).unwrap();

    let five_hour = limits.five_hour.unwrap();
    assert_eq!(five_hour.used_percentage, 27.0);
    assert_eq!(five_hour.resets_at, Some(time("2026-11-09T14:00:00Z")));
    let seven_day = limits.seven_day.unwrap();
    assert_eq!(seven_day.used_percentage, 74.0);
    assert_eq!(seven_day.resets_at, Some(time("2026-11-13T00:00:00Z")));
    assert_eq!(five_hour.observed_at, None);
    assert_eq!(limits.spend, None);
}

#[test]
fn the_keyed_shape_works_and_ignores_per_model_and_app_entries() {
    let json = concat!(
        "{\"five_hour\":{\"utilization\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},",
        "\"seven_day\":{\"utilization\":74,\"resets_at\":\"2026-11-13T00:00:00+00:00\"},",
        "\"seven_day_opus\":{\"utilization\":90,\"resets_at\":null},\"seven_day_oauth_apps\":{\"utilization\":5}}",
    );

    let limits = usage_api_json::parse(json).unwrap();

    assert_eq!(limits.five_hour.unwrap().used_percentage, 27.5);
    let seven_day = limits.seven_day.unwrap();
    assert_eq!(seven_day.used_percentage, 74.0);
    assert!(seven_day.resets_at.is_some());
}

#[test]
fn a_list_with_no_usable_entry_falls_back_to_the_keys() {
    let json = concat!(
        "{\"limits\":[{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}],",
        "\"five_hour\":{\"utilization\":10}}",
    );

    let limits = usage_api_json::parse(json).unwrap();

    assert_eq!(limits.five_hour.unwrap().used_percentage, 10.0);
    assert_eq!(limits.seven_day, None);
}

#[test]
fn a_missing_reset_is_a_null_reset() {
    let limits =
        usage_api_json::parse("{\"limits\":[{\"group\":\"session\",\"percent\":3}]}").unwrap();

    let five_hour = limits.five_hour.unwrap();
    assert_eq!(five_hour.used_percentage, 3.0);
    assert_eq!(five_hour.resets_at, None);
}

#[test]
fn a_string_percent_drops_that_window() {
    let limits = usage_api_json::parse(
        "{\"limits\":[{\"group\":\"session\",\"percent\":\"27\"},{\"group\":\"weekly\",\"percent\":50}]}",
    )
    .unwrap();

    assert_eq!(limits.five_hour, None);
    assert_eq!(limits.seven_day.unwrap().used_percentage, 50.0);
}

#[test]
fn a_bad_reset_text_is_a_null_reset() {
    let limits = usage_api_json::parse(
        "{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"soon\"}]}",
    )
    .unwrap();

    assert_eq!(limits.five_hour.unwrap().resets_at, None);
}

#[test]
fn nothing_usable_is_null() {
    for json in [
        "{}",
        "[]",
        "",
        "{ broken",
        "null",
        "{\"limits\":\"x\",\"five_hour\":5}",
        "{\"limits\":[1,null,\"a\",{\"group\":5}]}",
    ] {
        assert_eq!(usage_api_json::parse(json), None, "{json}");
    }
}

#[test]
fn a_window_missing_from_the_list_comes_from_the_keys() {
    let limits = usage_api_json::parse(
        "{\"limits\":[{\"group\":\"weekly\",\"percent\":60}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
    )
    .unwrap();

    assert_eq!(limits.five_hour.unwrap().used_percentage, 12.0);
    assert_eq!(limits.seven_day.unwrap().used_percentage, 60.0);
}

// What follows is new. These tests hold what the C# of 1.1.0 answered when it was run, on 2026-11-14,
// on made-up replies. The rows were written out by the C# and turned into Rust by a script, not
// typed.

/// A window as a row writes it: the percentage, and the reset time when there is one.
type Window = Option<(f64, Option<&'static str>)>;

/// Holds a window against a row's. The percentage is compared bit for bit, so that a zero
/// with a minus sign is not taken for a plain zero.
fn assert_window(window: Option<LimitWindow>, expected: &Window, json: &str) {
    let found = window.map(|window| (window.used_percentage.to_bits(), window.resets_at));
    let expected = expected.map(|(percent, reset)| (percent.to_bits(), reset.map(time)));
    assert_eq!(found, expected, "{json}");
}

/// A reply with a session window of 3 percent and `written` for its `resets_at`.
fn with_reset(written: &str) -> String {
    "{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":".to_string() + written + "}]}"
}

#[test]
fn resets_at_answers_as_the_csharp_did() {
    // `resets_at` as the reply writes it, and the time read from it: in UTC, to the
    // ten-millionth of a second.
    let rows: &[(&str, Option<&str>)] = &[
        ("\"2026-11-09T14:00:00Z\"", Some("2026-11-09T14:00:00Z")),
        (
            "\"2026-11-09T14:00:00+00:00\"",
            Some("2026-11-09T14:00:00Z"),
        ),
        (
            "\"2026-11-09T14:00:00-00:00\"",
            Some("2026-11-09T14:00:00Z"),
        ),
        (
            "\"2026-11-09T14:00:00+01:00\"",
            Some("2026-11-09T13:00:00Z"),
        ),
        (
            "\"2026-11-09T14:00:00-08:00\"",
            Some("2026-11-09T22:00:00Z"),
        ),
        ("\"2026-11-09T14:00:00+0100\"", None),
        ("\"2026-11-09T14:00:00+01\"", Some("2026-11-09T13:00:00Z")),
        ("\"2026-11-09T14:00:00+1\"", None),
        (
            "\"2026-11-09T14:00:00+14:00\"",
            Some("2026-11-09T00:00:00Z"),
        ),
        ("\"2026-11-09T14:00:00+14:01\"", None),
        (
            "\"2026-11-09T14:00:00-14:00\"",
            Some("2026-11-10T04:00:00Z"),
        ),
        ("\"2026-11-09T14:00:00+15:00\"", None),
        ("\"2026-11-09T14:00:00+01:60\"", None),
        (
            "\"2026-11-09T14:00:00+01:59\"",
            Some("2026-11-09T12:01:00Z"),
        ),
        ("\"2026-11-09T14:00:00+24:00\"", None),
        ("\"2026-11-09T14:00:00+01:0\"", None),
        ("\"2026-11-09T14:00:00+01:00Z\"", None),
        ("\"2026-11-09T14:00:00z\"", None),
        ("\"2026-11-09t14:00:00Z\"", None),
        ("\"2026-11-09 14:00:00Z\"", None),
        ("\"2026-11-09T14:00:00 Z\"", None),
        ("\" 2026-11-09T14:00:00Z\"", None),
        ("\"2026-11-09T14:00:00Z \"", None),
        ("\"2026-11-09T14:00Z\"", Some("2026-11-09T14:00:00Z")),
        ("\"2026-11-09T14Z\"", None),
        (
            "\"2026-11-09T14:00:00.1Z\"",
            Some("2026-11-09T14:00:00.1000000Z"),
        ),
        (
            "\"2026-11-09T14:00:00.123456Z\"",
            Some("2026-11-09T14:00:00.1234560Z"),
        ),
        (
            "\"2026-11-09T14:00:00.1234567Z\"",
            Some("2026-11-09T14:00:00.1234567Z"),
        ),
        (
            "\"2026-11-09T14:00:00.12345678Z\"",
            Some("2026-11-09T14:00:00.1234567Z"),
        ),
        (
            "\"2026-11-09T14:00:00.123456789Z\"",
            Some("2026-11-09T14:00:00.1234567Z"),
        ),
        (
            "\"2026-11-09T14:00:00.1234567890123456Z\"",
            Some("2026-11-09T14:00:00.1234567Z"),
        ),
        ("\"2026-11-09T14:00:00.12345678901234567Z\"", None),
        ("\"2026-11-09T14:00:00.Z\"", Some("2026-11-09T14:00:00Z")),
        ("\"2026-11-09T14:00:00,5Z\"", None),
        (
            "\"2026-11-09T14:00:00.999999999Z\"",
            Some("2026-11-09T14:00:00.9999999Z"),
        ),
        (
            "\"2026-11-09T14:00:00.123456+00:00\"",
            Some("2026-11-09T14:00:00.1234560Z"),
        ),
        (
            "\"2026-11-09T14:00:00.5-03:30\"",
            Some("2026-11-09T17:30:00.5000000Z"),
        ),
        ("\"2026-11-09T24:00:00Z\"", None),
        ("\"2026-11-09T23:59:59Z\"", Some("2026-11-09T23:59:59Z")),
        ("\"2026-11-09T23:59:60Z\"", None),
        ("\"2026-11-09T23:60:00Z\"", None),
        ("\"2026-02-29T00:00:00Z\"", None),
        ("\"2028-02-29T00:00:00Z\"", Some("2028-02-29T00:00:00Z")),
        ("\"2026-13-01T00:00:00Z\"", None),
        ("\"2026-00-01T00:00:00Z\"", None),
        ("\"2026-11-31T00:00:00Z\"", None),
        ("\"2026-11-00T00:00:00Z\"", None),
        ("\"0000-01-01T00:00:00Z\"", None),
        ("\"0001-01-01T00:00:00Z\"", Some("0001-01-01T00:00:00Z")),
        ("\"0001-01-01T00:00:00+01:00\"", None),
        (
            "\"0001-01-01T00:00:00-01:00\"",
            Some("0001-01-01T01:00:00Z"),
        ),
        ("\"9999-12-31T23:59:59Z\"", Some("9999-12-31T23:59:59Z")),
        ("\"9999-12-31T23:59:59-01:00\"", None),
        (
            "\"9999-12-31T23:59:59.9999999Z\"",
            Some("9999-12-31T23:59:59.9999999Z"),
        ),
        (
            "\"9999-12-31T23:59:59+01:00\"",
            Some("9999-12-31T22:59:59Z"),
        ),
        ("\"10000-01-01T00:00:00Z\"", None),
        ("\"999-01-01T00:00:00Z\"", None),
        ("\"2026-11-9T14:00:00Z\"", None),
        ("\"2026-1-09T14:00:00Z\"", None),
        ("\"20261109T140000Z\"", None),
        ("\"2026-11-09T1400Z\"", None),
        ("\"2026-11-09T14:0:00Z\"", None),
        ("\"2026-11-09Z\"", None),
        ("\"2026-11-09+01:00\"", None),
        ("\"2026-11-09T\"", None),
        ("\"2026-11-09T14:00:00+\"", None),
        ("\"2026-11-09T14:00:00-\"", None),
        ("\"2026-11-09T14:00:00ZZ\"", None),
        ("\"-2026-11-09T14:00:00Z\"", None),
        ("\"+2026-11-09T14:00:00Z\"", None),
        ("\"soon\"", None),
        ("\"\"", None),
        ("\"Z\"", None),
        (
            "\"2026-11-09T14:00:00\\u005A\"",
            Some("2026-11-09T14:00:00Z"),
        ),
        (
            "\"\\u0032026-11-09T14:00:00Z\"",
            Some("2026-11-09T14:00:00Z"),
        ),
        (
            "\"2026-11-09T14:00:00.123456789012345678901234567890Z\"",
            None,
        ),
        ("\"\u{662}\u{660}\u{662}\u{666}-11-09T14:00:00Z\"", None),
        ("\"2026-11-09T14:00.5Z\"", None),
        ("\"2026-11-09T14:00:00.\"", None),
        ("\"2026-11-09T14:00+01:00\"", Some("2026-11-09T13:00:00Z")),
        ("\"2026-11-09T14:00-08\"", Some("2026-11-09T22:00:00Z")),
        ("\"2026-11-09T14:00:00+00:00:00\"", None),
        ("\"\\ud800\"", None),
        ("\"\\u0032\\u0030\\u0032\"", None),
        (
            "\"\\u0032\\u0030\\u0032\\u0036-11-09T14:00:00Z\"",
            Some("2026-11-09T14:00:00Z"),
        ),
        (
            "\"2026-11-09T14:00:00Z                                                                                                                                                                                                                                        \"",
            None,
        ),
        (
            "\"2026-11-09T14:00:00Z                                                                                                                                                                                                                                         \"",
            None,
        ),
        (
            "\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\\ud800\"",
            None,
        ),
        (
            "\"2026-11-09T14:00:00.1234567890123456+00:00\"",
            Some("2026-11-09T14:00:00.1234567Z"),
        ),
        ("\"2026-11-09T14:00:00.1234567890123456+00:00 \"", None),
        (
            "\"2026-11-09T14:00:00.0000001Z\"",
            Some("2026-11-09T14:00:00.0000001Z"),
        ),
        (
            "\"2026-11-09T14:00:00.00000019Z\"",
            Some("2026-11-09T14:00:00.0000001Z"),
        ),
        ("\"2026-11-09T14:00:60Z\"", None),
        ("\"2026-11-09T14:60Z\"", None),
        ("\"2026-11-09T14:00:00+00\"", Some("2026-11-09T14:00:00Z")),
        ("\"2026-11-09T14:00:00-14\"", Some("2026-11-10T04:00:00Z")),
        ("\"2026-11-09T14:00:00-15\"", None),
        (
            "\"2026-11-09T14:00:00+13:59\"",
            Some("2026-11-09T00:01:00Z"),
        ),
        ("\"2026-11-09T14:00:00+aa:00\"", None),
        ("\"2026-11-09T14:00:00+01:aa\"", None),
        ("\"2026-11-09T14:00:00+01-00\"", None),
        ("\"2026-11-09T14:00:0aZ\"", None),
        ("\"2026-11-09T1a:00:00Z\"", None),
        ("\"2026-11-0aT14:00:00Z\"", None),
        ("\"2026-1a-09T14:00:00Z\"", None),
        ("\"202a-11-09T14:00:00Z\"", None),
        ("\"2026/11/09T14:00:00Z\"", None),
        ("\"2026-11-09T14.00.00Z\"", None),
        ("\"2026-11-09T14:00:00.5 \"", None),
        ("\"0000-12-31T23:30:00-01:00\"", None),
        ("\"0000-01-01T00:00:00+00:00\"", None),
        ("\"0001-01-01T00:30:00+01:00\"", None),
        ("null", None),
        ("1762696800", None),
        ("true", None),
        ("{}", None),
        ("[]", None),
        ("1.5", None),
    ];
    for (written, reset) in rows {
        let limits = usage_api_json::parse(&with_reset(written)).unwrap();

        assert_window(limits.five_hour, &Some((3.0, *reset)), written);
    }
}

#[test]
fn a_time_that_does_not_say_how_far_it_is_from_utc_is_no_time() {
    // `resets_at` as the reply writes it, and what the C# read: it took such a time to be on
    // the PC's own clock, which was set to UTC when these were run. Core does not know the PC's
    // time zone, and reads no time. The window is still there.
    let rows: &[(&str, &str)] = &[
        ("\"2026-11-09T14:00:00\"", "2026-11-09T14:00:00Z"),
        ("\"2026-11-09\"", "2026-11-09T00:00:00Z"),
        ("\"2026-11-09T14:00\"", "2026-11-09T14:00:00Z"),
        ("\"2026-11-09T14:00:00.5\"", "2026-11-09T14:00:00.5000000Z"),
    ];
    for (written, _on_a_pc_set_to_utc) in rows {
        let limits = usage_api_json::parse(&with_reset(written)).unwrap();

        assert_window(limits.five_hour, &Some((3.0, None)), written);
    }
}

/// What the C# made of a reply whose session window has a given `percent`.
enum Session {
    Percent(f64),
    /// The reply is read, without a session window.
    NoWindow,
    /// The whole reply is not JSON.
    NotJson,
}

#[test]
fn percent_answers_as_the_csharp_did() {
    use Session::{NoWindow, NotJson, Percent};
    // `percent` as the reply writes it, and the answer.
    let rows: &[(&str, Session)] = &[
        ("27", Percent(27.0)),
        ("27.5", Percent(27.5)),
        ("0", Percent(0.0)),
        ("-0", Percent(-0.0)),
        ("-0.0", Percent(-0.0)),
        ("-5", Percent(-5.0)),
        ("100", Percent(100.0)),
        ("250", Percent(250.0)),
        ("1e2", Percent(100.0)),
        ("1E2", Percent(100.0)),
        ("1e-2", Percent(0.01)),
        ("1e999", Percent(f64::INFINITY)),
        ("-1e999", Percent(f64::NEG_INFINITY)),
        ("1e308", Percent(1e308)),
        ("1e309", Percent(f64::INFINITY)),
        ("5e-324", Percent(5e-324)),
        ("1e-400", Percent(0.0)),
        ("0.1", Percent(0.1)),
        ("0.30000000000000004", Percent(0.30000000000000004)),
        (
            "123456789012345678901234567890",
            Percent(1.2345678901234568e29),
        ),
        ("9007199254740993", Percent(9007199254740992.0)),
        ("\"27\"", NoWindow),
        ("null", NoWindow),
        ("true", NoWindow),
        ("[]", NoWindow),
        ("{}", NoWindow),
        ("1.0", Percent(1.0)),
        ("01", NotJson),
        ("1.", NotJson),
        (".5", NotJson),
        ("+5", NotJson),
        ("0x10", NotJson),
        ("NaN", NotJson),
        ("Infinity", NotJson),
        ("4.35", Percent(4.35)),
        ("2.675", Percent(2.675)),
        ("1.7976931348623157e308", Percent(1.7976931348623157e308)),
        ("1.7976931348623159e308", Percent(f64::INFINITY)),
        ("0.000001", Percent(1e-6)),
        ("1e-7", Percent(1e-7)),
    ];
    for (written, answer) in rows {
        let json = "{\"limits\":[{\"group\":\"session\",\"percent\":".to_string()
            + written
            + "},{\"group\":\"weekly\",\"percent\":50}]}";

        let limits = usage_api_json::parse(&json);

        let weekly = Some((50.0, None));
        match (answer, limits) {
            (Percent(percent), Some(limits)) => {
                assert_window(limits.five_hour, &Some((*percent, None)), written);
                assert_window(limits.seven_day, &weekly, written);
            }
            (NoWindow, Some(limits)) => {
                assert_window(limits.five_hour, &None, written);
                assert_window(limits.seven_day, &weekly, written);
            }
            (NotJson, None) => {}
            (_, limits) => panic!("{written}: {limits:?}"),
        }
    }
}

#[test]
fn parse_answers_as_the_csharp_did() {
    // A reply, and the session and the weekly window read from it, or `None` for no limits.
    let rows: &[(&str, Option<(Window, Window)>)] = &[
        ("{}", None),
        ("[]", None),
        ("", None),
        (" ", None),
        ("{ broken", None),
        ("null", None),
        ("1", None),
        ("\"x\"", None),
        ("{\"limits\":\"x\",\"five_hour\":5}", None),
        ("{\"limits\":[1,null,\"a\",{\"group\":5}]}", None),
        ("{\"limits\":[]}", None),
        ("{\"limits\":{}}", None),
        ("{\"limits\":null}", None),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1},{\"group\":\"session\",\"percent\":2}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"weekly\",\"percent\":1},{\"group\":\"weekly\",\"percent\":2},{\"group\":\"session\",\"percent\":3}]}",
            Some((Some((3.0, None)), Some((1.0, None)))),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":\"x\"},{\"group\":\"session\",\"percent\":2}]}",
            Some((Some((2.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"Session\",\"percent\":1},{\"group\":\"WEEKLY\",\"percent\":2}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session \",\"percent\":1}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"\\u0073ession\",\"percent\":1}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"\\u0067roup\":\"session\",\"\\u0070ercent\":1}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"\\ud800\",\"percent\":\"x\"}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"\\ud800\",\"percent\":1,\"scope\":1}]}",
            None,
        ),
        ("{\"limits\":[{\"group\":\"monthly\",\"percent\":1}]}", None),
        (
            "{\"limits\":[{\"group\":\"monthly\",\"percent\":1}],\"five_hour\":{\"utilization\":9}}",
            Some((Some((9.0, None)), None)),
        ),
        ("{\"limits\":[{\"group\":null,\"percent\":1}]}", None),
        ("{\"limits\":[{\"percent\":1}]}", None),
        (
            "{\"limits\":[[{\"group\":\"session\",\"percent\":1}]]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":null}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":false}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":0}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":\"\"}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":{}}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":[]}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":{\"model\":\"x\"},\"scope\":null}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"scope\":null,\"scope\":{\"model\":\"x\"}}]}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"Scope\":{\"model\":\"x\"}}]}",
            Some((Some((1.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1}],\"limits\":[{\"group\":\"session\",\"percent\":2}]}",
            Some((Some((2.0, None)), None)),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1}],\"limits\":5}",
            None,
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"group\":\"weekly\",\"percent\":1,\"percent\":2}]}",
            Some((None, Some((2.0, None)))),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1,\"resets_at\":\"2026-11-09T14:00:00Z\",\"resets_at\":\"2026-11-10T14:00:00Z\"}]}",
            Some((Some((1.0, Some("2026-11-10T14:00:00Z"))), None)),
        ),
        (
            "{\"five_hour\":{\"utilization\":1},\"five_hour\":{\"utilization\":2}}",
            Some((Some((2.0, None)), None)),
        ),
        ("{\"five_hour\":{\"utilization\":1},\"five_hour\":7}", None),
        (
            "{\"five_hour\":{\"utilization\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},\"seven_day\":{\"utilization\":74,\"resets_at\":\"2026-11-13T00:00:00+00:00\"},\"seven_day_opus\":{\"utilization\":90,\"resets_at\":null},\"seven_day_oauth_apps\":{\"utilization\":5}}",
            Some((
                Some((27.5, Some("2026-11-09T14:00:00Z"))),
                Some((74.0, Some("2026-11-13T00:00:00Z"))),
            )),
        ),
        (
            "{\"five_hour\":{\"utilization\":10}}",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"seven_day\":{\"utilization\":10}}",
            Some((None, Some((10.0, None)))),
        ),
        ("{\"five_hour\":{\"percent\":10}}", None),
        ("{\"five_hour\":{\"utilization\":\"10\"}}", None),
        ("{\"five_hour\":[]}", None),
        (
            "{\"five_hour\":null,\"seven_day\":{\"utilization\":1}}",
            Some((None, Some((1.0, None)))),
        ),
        ("{\"Five_Hour\":{\"utilization\":10}}", None),
        (
            "{\"five_hour\":{\"utilization\":10,\"resets_at\":1762696800}}",
            Some((Some((10.0, None)), None)),
        ),
        ("{\"five_hour\":{\"used_percentage\":10}}", None),
        (
            "{\"limits\":[{\"group\":\"weekly\",\"percent\":60}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
            Some((Some((12.0, None)), Some((60.0, None)))),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":60}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
            Some((Some((60.0, None)), Some((99.0, None)))),
        ),
        (
            "{\"limits\":[{\"group\":\"session\",\"percent\":1},{\"group\":\"weekly\",\"percent\":2}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
            Some((Some((1.0, None)), Some((2.0, None)))),
        ),
        (
            "{\"limits\":[{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}],\"five_hour\":{\"utilization\":10}}",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"limits\":7,\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
            Some((Some((12.0, None)), Some((99.0, None)))),
        ),
        ("{\"five_hour\":{\"utilization\":10},}", None),
        ("{\"five_hour\":{\"utilization\":10}} // c", None),
        ("/* c */{\"five_hour\":{\"utilization\":10}}", None),
        ("{\"five_hour\":{\"utilization\":10}} x", None),
        ("\u{feff}{\"five_hour\":{\"utilization\":10}}", None),
        (
            " \r\n\t{\"five_hour\":{\"utilization\":10}}\n",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"five_hour\":{\"utilization\":10},\"x\":\"\\ud800\"}",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"five_hour\":{\"utilization\":10},\"x\":1e999}",
            Some((Some((10.0, None)), None)),
        ),
        ("{\"five_hour\":{\"utilization\":10},\"x\":01}", None),
        (
            "{\"five_hour\":{\"utilization\":10},\"\\ud800\":1}",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"five_hour\":{\"utilization\":10},\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}",
            Some((Some((10.0, None)), None)),
        ),
        (
            "{\"five_hour\":{\"utilization\":10},\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}",
            None,
        ),
        ("{\"five_hour\":{\"utilization\":10},\"x\":\"a\tb\"}", None),
    ];
    for (json, answer) in rows {
        let limits = usage_api_json::parse(json);

        match (answer, limits) {
            (Some((five_hour, seven_day)), Some(limits)) => {
                assert_window(limits.five_hour, five_hour, json);
                assert_window(limits.seven_day, seven_day, json);
                assert_eq!(limits.spend, None, "{json}");
            }
            (None, None) => {}
            (_, limits) => panic!("{json}: {limits:?}"),
        }
    }
}

#[test]
fn half_a_character_where_a_text_is_read_makes_the_reply_no_use() {
    // A group or a reset time that spells out half a character. The C# stopped here with an
    // error, which its caller took for a failed request; a reply with no limits comes to the
    // same. The last row shows that it stopped even after it had found a window.
    let rows: &[&str] = &[
        "{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"2026-11-09T14:00:00\\ud800\"}]}",
        "{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"\\ud800\\ud800\"}]}",
        "{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\\ud800\"}]}",
        "{\"limits\":[{\"group\":\"\\ud800\",\"percent\":1}]}",
        "{\"limits\":[{\"group\":\"session\",\"percent\":1},{\"group\":\"\\ud800\",\"percent\":1}]}",
    ];
    for json in rows {
        assert_eq!(usage_api_json::parse(json), None, "{json}");
    }
}
