//! Ported from `tests/Sobreiro.Tests/CopilotUserJsonTests.cs`. The tests after
//! `malformed_bodies_give_no_reading` are new.
use chrono::{DateTime, Utc};
use sobreiro_core::copilot::copilot_user_json::{self, Reply};
use sobreiro_core::copilot::{CopilotReading, CopilotSource};

use crate::time;

fn now() -> DateTime<Utc> {
    time("2026-11-10T12:00:00Z")
}

const BUSINESS: &str = concat!(
    "{\n",
    "  \"copilot_plan\": \"business\",\n",
    "  \"token_based_billing\": true,\n",
    "  \"quota_reset_date_utc\": \"2026-12-01T00:00:00.000Z\",\n",
    "  \"quota_snapshots\": {\n",
    "    \"premium_interactions\": {\n",
    "      \"entitlement\": 15000, \"remaining\": 12355, \"quota_remaining\": 12355.8,\n",
    "      \"percent_remaining\": 82.3, \"unlimited\": false, \"has_quota\": true,\n",
    "      \"overage_permitted\": true, \"overage_count\": 0, \"overage_entitlement\": 0,\n",
    "      \"token_based_billing\": true, \"timestamp_utc\": \"2026-11-10T09:25:19.365Z\"\n",
    "    },\n",
    "    \"chat\": { \"unlimited\": true },\n",
    "    \"completions\": { \"unlimited\": true }\n",
    "  }\n",
    "}",
);

/// A reply whose `premium_interactions` holds these fields.
fn interactions(fields: &str) -> String {
    "{\"quota_snapshots\":{\"premium_interactions\":{".to_string() + fields + "}}}"
}

/// A reply with a budget, and this for its `quota_reset_date_utc`.
fn with_reset(reset: &str) -> String {
    "{\"quota_reset_date_utc\":\"".to_string()
        + reset
        + "\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}"
}

/// The reading in a reply. The test fails when there is none.
fn reading(body: &str) -> CopilotReading {
    match copilot_user_json::parse(body, now()) {
        Reply::Reading(reading) => reading,
        other => panic!("{body}: {other:?}"),
    }
}

/// Holds two numbers to be the same once both are rounded to `places` after the point, as
/// the C# tests do.
fn assert_close(actual: f64, expected: f64, places: i32) {
    let rounded = |number: f64| (number * 10f64.powi(places)).round() / 10f64.powi(places);
    assert_eq!(
        rounded(actual),
        rounded(expected),
        "{actual} and {expected}"
    );
}

#[test]
fn a_business_seat_reads_used_from_quota_remaining() {
    let r = reading(BUSINESS);

    assert_eq!(r.budget, 15000.0);
    assert_close(r.used, 2644.2, 1);
    assert_close(r.percent(), 17.628, 3);
    assert_eq!(r.resets_at, Some(time("2026-12-01T00:00:00Z")));
    assert_eq!(r.observed_at, time("2026-11-10T09:25:19.365Z"));
    assert_eq!(r.source, CopilotSource::Live);
}

#[test]
fn premium_models_wins_over_premium_interactions() {
    let body = concat!(
        "{\"quota_snapshots\":{\n",
        "  \"premium_models\":{\"entitlement\":300,\"quota_remaining\":150},\n",
        "  \"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12000}}}",
    );

    let r = reading(body);

    assert_eq!(r.budget, 300.0);
    assert_close(r.used, 150.0, 3);
}

#[test]
fn a_free_plan_has_no_budget() {
    let body = concat!(
        "{\"quota_reset_at\":0,\"quota_snapshots\":{\n",
        "  \"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0},\n",
        "  \"chat\":{\"entitlement\":200,\"remaining\":150},\n",
        "  \"completions\":{\"entitlement\":2000,\"remaining\":1900}}}",
    );

    assert_eq!(copilot_user_json::parse(body, now()), Reply::NoBudget);
}

#[test]
fn unlimited_has_no_budget() {
    let body = interactions("\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100");

    assert_eq!(copilot_user_json::parse(&body, now()), Reply::NoBudget);
}

#[test]
fn used_falls_back_to_remaining() {
    let r = reading(&interactions("\"entitlement\":15000,\"remaining\":12355"));

    assert_close(r.used, 2645.0, 3);
}

#[test]
fn used_falls_back_to_percent_remaining() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"percent_remaining\":82.3",
    ));

    assert_close(r.used, 2655.0, 3);
}

#[test]
fn used_falls_back_to_credits_used() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"credits_used\":2644.2",
    ));

    assert_close(r.used, 2644.2, 3);
}

#[test]
fn no_used_field_means_nothing() {
    let body = interactions("\"entitlement\":15000");

    assert_eq!(copilot_user_json::parse(&body, now()), Reply::NoBudget);
}

#[test]
fn overage_count_reads_over_budget() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":210",
    ));

    assert_close(r.used, 15210.0, 3);
    assert_close(r.percent(), 101.4, 3);
}

#[test]
fn negative_quota_remaining_reads_over_budget() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"quota_remaining\":-50",
    ));

    assert_close(r.used, 15050.0, 3);
}

#[test]
fn an_offset_timestamp_is_kept() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T03:06:03.986-08:00\"",
    ));

    assert_eq!(r.observed_at, time("2026-11-09T11:06:03.986Z"));
}

#[test]
fn a_missing_timestamp_uses_now() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"quota_remaining\":100",
    ));

    assert_eq!(r.observed_at, now());
}

#[test]
fn a_missing_reset_date_gives_no_reset() {
    let r = reading(&interactions(
        "\"entitlement\":15000,\"quota_remaining\":100",
    ));

    assert_eq!(r.resets_at, None);
}

#[test]
fn an_absurd_reset_date_counts_as_absent() {
    for reset in ["9999-12-31T23:59:59Z", "2028-01-01T00:00:00Z"] {
        assert_eq!(reading(&with_reset(reset)).resets_at, None, "{reset}");
    }
}

#[test]
fn a_reset_date_within_400_days_is_kept() {
    let r = reading(&with_reset("2027-10-01T00:00:00Z"));

    assert_eq!(r.resets_at, Some(time("2027-10-01T00:00:00Z")));
}

#[test]
fn malformed_bodies_give_no_reading() {
    // The C# reader gave nothing for all four, and its client asked a second question to
    // tell them apart: is the body a JSON object at all. Here the reader answers both.
    let rows = [
        ("", Reply::NoUse),
        ("[]", Reply::NoUse),
        ("{", Reply::NoUse),
        ("{\"quota_snapshots\":5}", Reply::NoBudget),
    ];
    for (body, expected) in rows {
        assert_eq!(copilot_user_json::parse(body, now()), expected, "{body}");
    }
}

// The rows below were written out by the C# of 1.1.0, run on 2026-11-14 on made-up tokens and
// made-up replies with a stand-in for the network, and turned into Rust by a script, not typed.

/// What the C# made of a reply.
enum Expected {
    /// The budget, what is used, when the month resets, and when the reply was read.
    Reading(f64, f64, Option<&'static str>, &'static str),
    NoBudget,
    NoUse,
}

/// Holds two numbers to be the same bit for bit, so that a zero with a minus sign is not
/// taken for a plain zero. "Not a number" is the same as itself here.
fn assert_same_number(actual: f64, expected: f64, body: &str) {
    if expected.is_nan() {
        assert!(actual.is_nan(), "{body}: {actual}");
    } else {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{body}: {actual}");
    }
}

#[test]
fn parse_answers_as_the_csharp_did() {
    use Expected::{NoBudget, NoUse, Reading};
    // A reply, and what it holds. `now` is 2026-11-10T12:00:00Z. Where the C#'s reader gave
    // nothing, its client said which of the two it was: a reply with no budget, or no reply
    // at all.
    let rows: &[(&str, Expected)] = &[
        ("", NoUse),
        (" ", NoUse),
        ("[]", NoUse),
        ("{", NoUse),
        ("{}", NoBudget),
        ("null", NoUse),
        ("5", NoUse),
        ("\"x\"", NoUse),
        ("true", NoUse),
        ("[{}]", NoUse),
        ("{\"quota_snapshots\":5}", NoBudget),
        ("{\"quota_snapshots\":null}", NoBudget),
        ("{\"quota_snapshots\":[]}", NoBudget),
        ("{\"quota_snapshots\":{}}", NoBudget),
        ("{\"quota_snapshots\":\"x\"}", NoBudget),
        ("{\"quota_snapshots\":true}", NoBudget),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":5}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":null}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":[]}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":[{\"entitlement\":15000,\"quota_remaining\":100}]}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":5,\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":null,\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":[],\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":\"x\",\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{},\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{\"entitlement\":0,\"quota_remaining\":0},\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{\"unlimited\":true,\"entitlement\":300,\"quota_remaining\":1},\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{\"entitlement\":300},\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100},\"premium_models\":{\"entitlement\":300,\"quota_remaining\":150}}}",
            Reading(300.0, 150.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{\"entitlement\":300,\"quota_remaining\":150}}}",
            Reading(300.0, 150.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"chat\":{\"entitlement\":15000,\"quota_remaining\":100},\"completions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":5,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}},\"quota_snapshots\":5}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1,\"quota_remaining\":1},\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100},\"premium_interactions\":5}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_models\":{\"entitlement\":300,\"quota_remaining\":150},\"premium_models\":5,\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"entitlement\":\"x\",\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"quota_remaining\":\"x\",\"remaining\":200}}}",
            Reading(15000.0, 14800.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":\"x\",\"quota_remaining\":100,\"remaining\":200}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":true,\"unlimited\":false,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":false,\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"\\u0071uota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium\\u005finteractions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"\\u0065ntitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"Quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"Premium_Interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"Entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement \":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":false,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":\"true\",\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":1,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":null,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":[true],\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"unlimited\":true}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"quota_remaining\":0}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":-1,\"quota_remaining\":0}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":-0.0,\"quota_remaining\":0}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0.5,\"quota_remaining\":0.25}}}",
            Reading(0.5, 0.25, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1e999,\"quota_remaining\":5}}}",
            Reading(f64::INFINITY, f64::INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":-1e999,\"quota_remaining\":5}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1e999,\"quota_remaining\":1e999}}}",
            Reading(f64::INFINITY, f64::NAN, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1e999,\"percent_remaining\":50}}}",
            Reading(f64::INFINITY, f64::INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1e999,\"credits_used\":7}}}",
            Reading(f64::INFINITY, 7.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":\"15000\",\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":null,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":true,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1e-400,\"quota_remaining\":0}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":5e-324,\"quota_remaining\":0}}}",
            Reading(5e-324, 5e-324, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000.0,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1.5e4,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":1.5E+4,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":01,\"quota_remaining\":0}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":[15000],\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":{\"value\":15000},\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0.1,\"quota_remaining\":0.3}}}",
            Reading(0.1, -0.19999999999999998, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":9007199254740993,\"quota_remaining\":1}}}",
            Reading(
                9007199254740992.0,
                9007199254740991.0,
                None,
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":12355.8,\"remaining\":12355,\"percent_remaining\":82.3,\"credits_used\":1}}}",
            Reading(15000.0, 2644.2000000000007, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":\"12355.8\",\"remaining\":12355,\"percent_remaining\":82.3,\"credits_used\":1}}}",
            Reading(15000.0, 2645.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":null,\"remaining\":null,\"percent_remaining\":82.3,\"credits_used\":1}}}",
            Reading(15000.0, 2655.0000000000005, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":true,\"remaining\":[1],\"percent_remaining\":{},\"credits_used\":1}}}",
            Reading(15000.0, 1.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":\"1\"}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":null}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"remaining\":12355,\"quota_remaining\":12355.8}}}",
            Reading(15000.0, 2644.2000000000007, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":1,\"percent_remaining\":82.3}}}",
            Reading(15000.0, 2655.0000000000005, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":1e999}}}",
            Reading(15000.0, f64::NEG_INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":-1e999}}}",
            Reading(15000.0, f64::INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":15000}}}",
            Reading(15000.0, 0.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":15001}}}",
            Reading(15000.0, -1.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":-0.0}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":100}}}",
            Reading(15000.0, 0.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":0}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":150}}}",
            Reading(15000.0, -7500.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":-10}}}",
            Reading(15000.0, 16500.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":33.3}}}",
            Reading(15000.0, 10005.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":300,\"percent_remaining\":66.6}}}",
            Reading(300.0, 100.20000000000002, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":1e999}}}",
            Reading(15000.0, f64::NEG_INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":0}}}",
            Reading(15000.0, 0.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":-5}}}",
            Reading(15000.0, -5.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":20000}}}",
            Reading(15000.0, 20000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":1e999}}}",
            Reading(15000.0, f64::INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"used\":5}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_used\":5}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":210}}}",
            Reading(15000.0, 15210.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":-50,\"overage_count\":210}}}",
            Reading(15000.0, 15050.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"overage_count\":5}}}",
            Reading(15000.0, 15005.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":0}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":-5}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":\"210\"}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":null}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":0.5}}}",
            Reading(15000.0, 15000.5, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":1e999}}}",
            Reading(15000.0, f64::INFINITY, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":15000,\"overage_count\":10}}}",
            Reading(15000.0, 15010.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"credits_used\":15001,\"overage_count\":10}}}",
            Reading(15000.0, 15001.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"percent_remaining\":0,\"overage_count\":10}}}",
            Reading(15000.0, 15010.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"overage_count\":10}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":0,\"overage_entitlement\":10}}}",
            Reading(15000.0, 15000.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19.365Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19.365Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T03:06:03.986-08:00\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-09T11:06:03.986Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19.1234567Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19.1234567Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19.12345678Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19.1234567Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19+01\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T08:25:19Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19+0100\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10 09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10t09:25:19z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"yesterday\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":null}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":1794302719}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":1794302719365}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":true}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"1794302719\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\\u0032026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19\\u005a\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T09:25:19Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\" 2026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19Z \"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"0001-01-01T00:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "0001-01-01T00:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"9999-12-31T23:59:59Z\"}}}",
            Reading(15000.0, 14900.0, None, "9999-12-31T23:59:59Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-02-30T00:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T24:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp\":\"2026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"Timestamp_utc\":\"2026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"timestamp_utc\":\"2026-11-10T09:25:19Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00.000Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00+01:00\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-11-30T23:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-15T12:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2027-12-15T12:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-15T12:00:00.0000001Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-15T12:00:01Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-15T11:59:59Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2027-12-15T11:59:59Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2025-10-06T12:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2025-10-06T12:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2025-10-06T11:59:59.9999999Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2025-10-06T11:59:59Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2025-10-06T12:00:01Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2025-10-06T12:00:01Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"9999-12-31T23:59:59Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"0001-01-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2028-01-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-11-10T12:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-11-10T12:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-11-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-11-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":null,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":0,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":1796083200,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"soon\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":true,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":[],\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"\\u0032026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-15T12:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T12:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-09T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-14T12:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T12:00:00Z\"}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2027-12-14T12:00:00Z"),
                "2026-11-09T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2027-12-14T12:00:00.0000001Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T12:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-09T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2020-01-01T00:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2020-01-01T00:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"bad\"}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date\":\"2026-12-01\",\"quota_reset_at\":1796083200,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_reset_date_utc\":\"2026-12-02T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-02T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_reset_date_utc\":5,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}},\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\"}",
            Reading(
                15000.0,
                14900.0,
                Some("2026-12-01T00:00:00Z"),
                "2026-11-10T12:00:00Z",
            ),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\\ud800\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\\ud83d\\ude00 2026-11-10T09:25:19Z\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"\\ud800\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"quota_remaining\":0}}}",
            NoBudget,
        ),
        (
            "{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000}}}",
            NoBudget,
        ),
        (
            "{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoBudget,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"timestamp_utc\":\"\\ud800-11-10T09:25:19Z\"}}}",
            NoBudget,
        ),
        (
            "{\"copilot_plan\":\"\\ud800\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"\\ud800\":1,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"\\ud800\":1,\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"\\ud800\":1,\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"\\ud800\":1}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":\"\\udc00\"}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}},\"\\ud800\":1}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        ("{\"\\ud800\":1}", NoBudget),
        ("{\"\\ud800\":1,\"quota_snapshots\":5}", NoBudget),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}} // c",
            NoUse,
        ),
        (
            "/* c */{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}} x",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}{}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}\r\n",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            " \t\r\n{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{'quota_snapshots':{'premium_interactions':{'entitlement':15000,'quota_remaining':100}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":NaN}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":01}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":+1}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":.5}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":1.}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":\"a\tb\"}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":\"a\\qb\"}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":Infinity}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":NaN}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":Infinity,\"quota_remaining\":1}}}",
            NoUse,
        ),
        // The mark that may start a text is not JSON. The client takes it off first.
        (
            "\u{feff}{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}\u{0}",
            NoUse,
        ),
        (
            "\u{0}{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}}}",
            NoUse,
        ),
        (
            "{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]],\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Reading(15000.0, 14900.0, None, "2026-11-10T12:00:00Z"),
        ),
        (
            "{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]],\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            NoUse,
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}},\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}",
            NoUse,
        ),
        (
            "{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}",
            NoUse,
        ),
        (
            "{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}",
            NoBudget,
        ),
    ];
    for (body, expected) in rows {
        let reply = copilot_user_json::parse(body, now());
        match expected {
            Reading(budget, used, resets_at, observed_at) => {
                let Reply::Reading(r) = reply else {
                    panic!("{body}: {reply:?}");
                };
                assert_same_number(r.budget, *budget, body);
                assert_same_number(r.used, *used, body);
                assert_eq!(r.resets_at, resets_at.map(time), "{body}");
                assert_eq!(r.observed_at, time(observed_at), "{body}");
                assert_eq!(r.source, CopilotSource::Live, "{body}");
            }
            NoBudget => assert_eq!(reply, Reply::NoBudget, "{body}"),
            NoUse => assert_eq!(reply, Reply::NoUse, "{body}"),
        }
    }
}

#[test]
fn half_a_character_where_a_time_is_read_makes_the_reply_no_use() {
    // A time that spells out half a character. The C#'s reader stopped here with an error,
    // which its client took for a failed request.
    let rows: &[&str] = &[
        "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\\ud800-11-10T09:25:19Z\"}}}",
        "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19Z\\ud800\"}}}",
        "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19\\udc00\"}}}",
        "{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
        "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\\ud800\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
    ];
    for body in rows {
        assert_eq!(
            copilot_user_json::parse(body, now()),
            Reply::NoUse,
            "{body}"
        );
    }
}

#[test]
fn a_time_that_does_not_say_how_far_it_is_from_utc_is_no_time() {
    // A reply, and the reset and the moment of reading the C# took from it: it took such a
    // time to be on the PC's own clock, which was set to UTC when these were run. Core does
    // not know the PC's time zone, and reads no time: no reset, and `now` for the moment of
    // reading. The reading is still there.
    let rows: &[(&str, Option<&str>, &str)] = &[
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19\"}}}",
            None,
            "2026-11-10T09:25:19Z",
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19.365\"}}}",
            None,
            "2026-11-10T09:25:19.365Z",
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25\"}}}",
            None,
            "2026-11-10T09:25:00Z",
        ),
        (
            "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10\"}}}",
            None,
            "2026-11-10T00:00:00Z",
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Some("2026-12-01T00:00:00Z"),
            "2026-11-10T12:00:00Z",
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Some("2026-12-01T00:00:00Z"),
            "2026-11-10T12:00:00Z",
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}",
            Some("2026-12-01T00:00:00Z"),
            "2026-11-10T12:00:00Z",
        ),
        (
            "{\"quota_reset_date_utc\":\"2026-12-01T00:00:00\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19\"}}}",
            Some("2026-12-01T00:00:00Z"),
            "2026-11-10T09:25:19Z",
        ),
    ];
    for (body, _reset_on_a_pc_set_to_utc, _read_on_a_pc_set_to_utc) in rows {
        let r = reading(body);

        assert_eq!(r.resets_at, None, "{body}");
        assert_eq!(r.observed_at, now(), "{body}");
    }
}
