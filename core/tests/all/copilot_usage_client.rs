//! Ported from `tests/Sobreiro.Tests/CopilotUsageClientTests.cs`. The tests after
//! `the_token_never_appears_in_the_result` are new.
//!
//! No test here reaches the network: the client is handed a sender that answers as the test
//! says. The token is made up. What the real sender does on the wire is in `http.rs`.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::copilot::copilot_usage_client::{self, CopilotUsageClient};
use sobreiro_core::copilot::{CopilotFetch, CopilotOutcome, CopilotSource, GitHubToken};
use sobreiro_core::io::http::{Answer, NoAnswer};
use sobreiro_core::live::live_usage_client::Cancelled;

use crate::{FakeSender, Seen, reply, reply_bytes, time, too_many_requests};

const SECRET: &str = "t0k3n";

const BUSINESS: &str = concat!(
    "{\n",
    "  \"copilot_plan\": \"business\",\n",
    "  \"quota_reset_date_utc\": \"2026-12-01T00:00:00.000Z\",\n",
    "  \"quota_snapshots\": {\n",
    "    \"premium_interactions\": {\n",
    "      \"entitlement\": 15000, \"remaining\": 12355, \"quota_remaining\": 12355.8,\n",
    "      \"unlimited\": false, \"has_quota\": true, \"timestamp_utc\": \"2026-11-10T09:25:19.365Z\"\n",
    "    }\n",
    "  }\n",
    "}",
);

const FREE: &str = "{\"quota_reset_at\":0,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

fn now() -> DateTime<Utc> {
    time("2026-11-10T12:00:00Z")
}

fn token(value: &str) -> GitHubToken {
    GitHubToken::new(value.to_string())
}

/// Asks once, with the made-up token, and gives the result and the sender that was asked.
fn run(
    respond: impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static,
) -> (CopilotFetch, Arc<FakeSender>) {
    let sender = Arc::new(FakeSender::new(respond));
    let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
    let result = client
        .fetch(&token(SECRET), now(), &AtomicBool::new(false))
        .unwrap();
    (result, sender)
}

#[test]
fn sends_the_four_headers_and_nothing_else() {
    let (_, sender) = run(|| reply(200, BUSINESS));

    // The seam has one way to ask, a GET: `http::a_request_goes_out_as_it_was_asked` holds
    // that word on the wire.
    let header = |name: &str, value: &str| (name.to_string(), value.to_string());
    assert_eq!(
        sender.seen(),
        [Seen {
            url: "https://api.github.com/copilot_internal/user".to_string(),
            headers: vec![
                header("Authorization", "Bearer t0k3n"),
                header("Accept", "application/json"),
                header("X-GitHub-Api-Version", "2025-04-01"),
                header("User-Agent", "Sobreiro/0.7.0"),
            ],
        }]
    );
    assert_eq!(
        copilot_usage_client::ENDPOINT,
        "https://api.github.com/copilot_internal/user"
    );
}

#[test]
fn ok_with_a_budget_gives_the_reading() {
    let (result, _) = run(|| reply(200, BUSINESS));

    assert_eq!(result.outcome, CopilotOutcome::Ok);
    let reading = result.reading.unwrap();
    assert!((reading.used - 2644.2).abs() < 0.05, "{}", reading.used);
    assert_eq!(reading.source, CopilotSource::Live);
    assert_eq!(result.retry_after, None);
}

#[test]
fn ok_without_a_budget_is_no_budget() {
    let (result, _) = run(|| reply(200, FREE));

    assert_eq!(result.outcome, CopilotOutcome::NoBudget);
    assert_eq!(result.reading, None);
}

#[test]
fn refusals() {
    for status in [401, 403, 404] {
        let (result, _) = run(move || reply(status, ""));

        assert_eq!(result.outcome, CopilotOutcome::Refused, "{status}");
        assert_eq!(result.reading, None, "{status}");
    }
}

#[test]
fn too_many_requests_carries_retry_after() {
    let (result, _) = run(|| too_many_requests("120"));

    assert_eq!(result.outcome, CopilotOutcome::RateLimited);
    assert_eq!(result.retry_after, Some(TimeDelta::minutes(2)));
}

#[test]
fn a_server_error_is_failed() {
    let (result, _) = run(|| reply(500, ""));

    assert_eq!(result.outcome, CopilotOutcome::Failed);
}

#[test]
fn an_unparsable_200_is_failed() {
    for body in ["{", "[1,2]", ""] {
        let (result, _) = run(move || reply(200, body));

        assert_eq!(result.outcome, CopilotOutcome::Failed, "{body}");
        assert_eq!(result.reading, None, "{body}");
    }
}

#[test]
fn a_redirect_is_failed() {
    // An answer as the client is handed it has no place for the address a redirect names, so
    // the client could not go there if it wanted to. That the real sender does not follow one
    // either is `http::a_redirect_comes_back_as_the_answer_and_is_not_followed`.
    let (result, sender) = run(|| reply(302, ""));

    assert_eq!(result.outcome, CopilotOutcome::Failed);
    assert_eq!(sender.seen().len(), 1);
}

#[test]
fn a_network_error_or_a_timeout_is_unreachable() {
    // The C# tests hand in an error with the token in its words, and a reply that takes too
    // long. Here a sender has no words to hand back, and it keeps the time itself: both come
    // to the client as `Unreachable`.
    let (result, _) = run(|| Err(NoAnswer::Unreachable));

    assert_eq!(result.outcome, CopilotOutcome::Unreachable);
    assert_eq!(result.reading, None);
    assert!(!format!("{result:?}").contains(SECRET));
}

#[test]
fn cancelling_by_the_caller_comes_back_as_cancelled() {
    let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
    let client = CopilotUsageClient::new(sender.clone(), "ua");
    let cancelled = AtomicBool::new(true);

    let result = client.fetch(&token(SECRET), now(), &cancelled);

    assert_eq!(result, Err(Cancelled));
    // Cancelled before it began: nothing was sent.
    assert_eq!(sender.seen(), []);
}

#[test]
fn the_token_never_appears_in_the_result() {
    for status in [200, 401, 429, 500] {
        let (result, _) = run(move || reply(status, BUSINESS));

        assert!(!format!("{result:?}").contains(SECRET), "{status}");
        assert!(!format!("{result:#?}").contains(SECRET), "{status}");
    }
}

// What follows is new.

#[test]
fn the_request_goes_to_the_one_address_whatever_the_answer() {
    let answers: [fn() -> Result<Answer, NoAnswer>; 9] = [
        || reply(200, BUSINESS),
        || reply(200, FREE),
        || reply(302, ""),
        || reply(401, ""),
        || reply(404, ""),
        || too_many_requests("120"),
        || reply(500, ""),
        || Err(NoAnswer::Unreachable),
        || Err(NoAnswer::Failed),
    ];
    for answer in answers {
        let (_, sender) = run(answer);

        // One request, and no second one after it.
        let seen = sender.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].url, "https://api.github.com/copilot_internal/user");
    }
}

#[test]
fn a_sender_that_could_not_make_the_request_is_failed() {
    let (result, _) = run(|| Err(NoAnswer::Failed));

    assert_eq!(result.outcome, CopilotOutcome::Failed);
    assert!(!format!("{result:?}").contains(SECRET));
}

#[test]
fn cancelling_while_the_request_is_under_way_drops_the_answer() {
    // The sender here stands in for a request that was still out when the caller cancelled.
    let cancelled = Arc::new(AtomicBool::new(false));
    let seen_by_sender = cancelled.clone();
    let sender = Arc::new(FakeSender::new(move || {
        seen_by_sender.store(true, Ordering::SeqCst);
        reply(200, BUSINESS)
    }));
    let client = CopilotUsageClient::new(sender, "ua");

    let result = client.fetch(&token(SECRET), now(), &cancelled);

    assert_eq!(result, Err(Cancelled));
}

#[test]
fn a_token_or_an_app_name_that_no_header_can_hold_is_not_sent() {
    // A line end or a zero inside the token: the C# gave `Failed` and sent nothing, as here.
    // The rest it would have handed to its HTTP library as they were. Here a header holds
    // letters, digits, signs and spaces of ASCII, and an empty token is not worth a request.
    for value in [
        "a\nb", "a\rb", "a\0b", "a\tb", "\u{7f}", "\u{e9}", "a\u{80}b", "",
    ] {
        let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
        let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");

        let result = client.fetch(&token(value), now(), &AtomicBool::new(false));

        assert_eq!(
            result.unwrap().outcome,
            CopilotOutcome::Failed,
            "token {value:?}"
        );
        assert_eq!(sender.seen(), [], "token {value:?}");
    }
    // An app name with a line end in it the C# sent on, line end and all.
    for user_agent in ["a\nb", "a\rb", "a\0b", "\u{e9}"] {
        let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
        let client = CopilotUsageClient::new(sender.clone(), user_agent);

        let result = client.fetch(&token(SECRET), now(), &AtomicBool::new(false));

        assert_eq!(
            result.unwrap().outcome,
            CopilotOutcome::Failed,
            "{user_agent:?}"
        );
        assert_eq!(sender.seen(), [], "{user_agent:?}");
    }
}

#[test]
fn a_token_with_spaces_and_signs_is_sent_as_it_is() {
    let made_up = format!("gho_{}", "a".repeat(36));
    for value in [" ", "a b", " a", "a ", "a,b", "a\"b", made_up.as_str()] {
        let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
        let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/1.1.0 (Windows)");

        let result = client.fetch(&token(value), now(), &AtomicBool::new(false));

        assert_eq!(result.unwrap().outcome, CopilotOutcome::Ok, "{value:?}");
        let seen = sender.seen();
        assert_eq!(
            seen[0].headers[0],
            ("Authorization".to_string(), format!("Bearer {value}"))
        );
        assert_eq!(
            seen[0].headers[3],
            (
                "User-Agent".to_string(),
                "Sobreiro/1.1.0 (Windows)".to_string()
            )
        );
    }
}

#[test]
fn a_body_in_utf_16_is_not_read() {
    // JSON between programs is UTF-8 by its standard. The C# also read a body in UTF-16 that
    // began with its mark: a reading out of the first of these, and "no budget" out of the
    // second. Here neither is JSON.
    let utf16 = |text: &str| -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    };
    for body in [utf16(BUSINESS), utf16("{}")] {
        let (result, _) = run(move || reply_bytes(200, &body));

        assert_eq!(result.outcome, CopilotOutcome::Failed);
        assert_eq!(result.reading, None);
    }
}

// The rows below were written out by the C# of 1.1.0, run on 2026-11-14 on made-up tokens and
// made-up replies with a stand-in for the network, and turned into Rust by a script, not typed.

#[test]
fn every_status_answers_as_the_csharp_did() {
    use CopilotOutcome::{Failed, NoBudget, RateLimited, Refused};
    // A status, and the outcome with three bodies: a business seat's, a free plan's, and a
    // list.
    let rows: &[(u16, CopilotOutcome, CopilotOutcome, CopilotOutcome)] = &[
        (100, Failed, Failed, Failed),
        (101, Failed, Failed, Failed),
        (102, Failed, Failed, Failed),
        (103, Failed, Failed, Failed),
        (200, CopilotOutcome::Ok, NoBudget, Failed),
        (201, CopilotOutcome::Ok, NoBudget, Failed),
        (202, CopilotOutcome::Ok, NoBudget, Failed),
        (203, CopilotOutcome::Ok, NoBudget, Failed),
        (204, CopilotOutcome::Ok, NoBudget, Failed),
        (205, CopilotOutcome::Ok, NoBudget, Failed),
        (206, CopilotOutcome::Ok, NoBudget, Failed),
        (207, CopilotOutcome::Ok, NoBudget, Failed),
        (208, CopilotOutcome::Ok, NoBudget, Failed),
        (209, CopilotOutcome::Ok, NoBudget, Failed),
        (210, CopilotOutcome::Ok, NoBudget, Failed),
        (211, CopilotOutcome::Ok, NoBudget, Failed),
        (212, CopilotOutcome::Ok, NoBudget, Failed),
        (213, CopilotOutcome::Ok, NoBudget, Failed),
        (214, CopilotOutcome::Ok, NoBudget, Failed),
        (215, CopilotOutcome::Ok, NoBudget, Failed),
        (216, CopilotOutcome::Ok, NoBudget, Failed),
        (217, CopilotOutcome::Ok, NoBudget, Failed),
        (218, CopilotOutcome::Ok, NoBudget, Failed),
        (219, CopilotOutcome::Ok, NoBudget, Failed),
        (220, CopilotOutcome::Ok, NoBudget, Failed),
        (221, CopilotOutcome::Ok, NoBudget, Failed),
        (222, CopilotOutcome::Ok, NoBudget, Failed),
        (223, CopilotOutcome::Ok, NoBudget, Failed),
        (224, CopilotOutcome::Ok, NoBudget, Failed),
        (225, CopilotOutcome::Ok, NoBudget, Failed),
        (226, CopilotOutcome::Ok, NoBudget, Failed),
        (300, Failed, Failed, Failed),
        (301, Failed, Failed, Failed),
        (302, Failed, Failed, Failed),
        (303, Failed, Failed, Failed),
        (304, Failed, Failed, Failed),
        (305, Failed, Failed, Failed),
        (306, Failed, Failed, Failed),
        (307, Failed, Failed, Failed),
        (308, Failed, Failed, Failed),
        (400, Failed, Failed, Failed),
        (401, Refused, Refused, Refused),
        (402, Failed, Failed, Failed),
        (403, Refused, Refused, Refused),
        (404, Refused, Refused, Refused),
        (405, Failed, Failed, Failed),
        (406, Failed, Failed, Failed),
        (407, Failed, Failed, Failed),
        (408, Failed, Failed, Failed),
        (409, Failed, Failed, Failed),
        (410, Failed, Failed, Failed),
        (411, Failed, Failed, Failed),
        (412, Failed, Failed, Failed),
        (413, Failed, Failed, Failed),
        (414, Failed, Failed, Failed),
        (415, Failed, Failed, Failed),
        (416, Failed, Failed, Failed),
        (417, Failed, Failed, Failed),
        (418, Failed, Failed, Failed),
        (419, Failed, Failed, Failed),
        (420, Failed, Failed, Failed),
        (421, Failed, Failed, Failed),
        (422, Failed, Failed, Failed),
        (423, Failed, Failed, Failed),
        (424, Failed, Failed, Failed),
        (425, Failed, Failed, Failed),
        (426, Failed, Failed, Failed),
        (427, Failed, Failed, Failed),
        (428, Failed, Failed, Failed),
        (429, RateLimited, RateLimited, RateLimited),
        (430, Failed, Failed, Failed),
        (431, Failed, Failed, Failed),
        (500, Failed, Failed, Failed),
        (501, Failed, Failed, Failed),
        (502, Failed, Failed, Failed),
        (503, Failed, Failed, Failed),
        (504, Failed, Failed, Failed),
        (505, Failed, Failed, Failed),
        (506, Failed, Failed, Failed),
        (507, Failed, Failed, Failed),
        (508, Failed, Failed, Failed),
        (509, Failed, Failed, Failed),
        (510, Failed, Failed, Failed),
        (511, Failed, Failed, Failed),
    ];
    for (status, business, free, list) in rows {
        let status = *status;

        let (result, _) = run(move || reply(status, BUSINESS));
        assert_eq!(result.outcome, *business, "{status}");
        // A reading comes with `Ok` and with nothing else.
        assert_eq!(
            result.reading.is_some(),
            *business == CopilotOutcome::Ok,
            "{status}"
        );

        let (result, _) = run(move || reply(status, FREE));
        assert_eq!(result.outcome, *free, "{status}, free");
        assert_eq!(result.reading, None, "{status}, free");

        let (result, _) = run(move || reply(status, "[1,2]"));
        assert_eq!(result.outcome, *list, "{status}, a list");
        assert_eq!(result.reading, None, "{status}, a list");
    }
}

#[test]
fn every_body_answers_as_the_csharp_did() {
    // What a body is, its bytes as they came with a 200, and the outcome.
    let rows: &[(&str, &[u8], CopilotOutcome)] = &[
        ("plain, no type", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Ok),
        ("bom, no type", b"\xef\xbb\xbf{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Ok),
        ("two boms, no type", b"\xef\xbb\xbf\xef\xbb\xbf{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Failed),
        ("bom and nothing", b"\xef\xbb\xbf", CopilotOutcome::Failed),
        ("bom and an empty object", b"\xef\xbb\xbf{}", CopilotOutcome::NoBudget),
        ("utf-16 no bom, no type", b"{\x00\"\x00q\x00u\x00o\x00t\x00a\x00_\x00s\x00n\x00a\x00p\x00s\x00h\x00o\x00t\x00s\x00\"\x00:\x00{\x00\"\x00p\x00r\x00e\x00m\x00i\x00u\x00m\x00_\x00i\x00n\x00t\x00e\x00r\x00a\x00c\x00t\x00i\x00o\x00n\x00s\x00\"\x00:\x00{\x00\"\x00e\x00n\x00t\x00i\x00t\x00l\x00e\x00m\x00e\x00n\x00t\x00\"\x00:\x001\x005\x000\x000\x000\x00,\x00\"\x00q\x00u\x00o\x00t\x00a\x00_\x00r\x00e\x00m\x00a\x00i\x00n\x00i\x00n\x00g\x00\"\x00:\x001\x000\x000\x00}\x00}\x00}\x00", CopilotOutcome::Failed),
        ("bad byte in another text", b"{\"x\":\"a\xffb\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Ok),
        ("bad byte outside a text", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}\xff", CopilotOutcome::Failed),
        ("bad byte in a key", b"{\"quota_snap\xffshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::NoBudget),
        ("bad byte in the stamp", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-10T09:25:19\xffZ\"}}}", CopilotOutcome::Ok),
        ("empty", b"", CopilotOutcome::Failed),
        ("spaces", b"  ", CopilotOutcome::Failed),
        ("nul after", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}\x00", CopilotOutcome::Failed),
        ("line end after", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}\r\n", CopilotOutcome::Ok),
        ("empty object", b"{}", CopilotOutcome::NoBudget),
        ("empty object and a line end", b"{}\n", CopilotOutcome::NoBudget),
        ("empty object and a letter", b"{} x", CopilotOutcome::Failed),
        ("empty object and a bad byte", b"{}\xff", CopilotOutcome::Failed),
        ("empty object twice", b"{}{}", CopilotOutcome::Failed),
        ("object with a comma too many", b"{\"a\":1,}", CopilotOutcome::Failed),
        ("object with a comment", b"{/* c */}", CopilotOutcome::Failed),
        ("object too deep", b"{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}", CopilotOutcome::Failed),
        ("object just deep enough", b"{\"x\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}", CopilotOutcome::NoBudget),
        ("snapshots a number", b"{\"quota_snapshots\":5}", CopilotOutcome::NoBudget),
        ("a list", b"[]", CopilotOutcome::Failed),
        ("a list of an object", b"[{}]", CopilotOutcome::Failed),
        ("null", b"null", CopilotOutcome::Failed),
        ("a number", b"5", CopilotOutcome::Failed),
        ("a text", b"\"x\"", CopilotOutcome::Failed),
        ("true", b"true", CopilotOutcome::Failed),
        ("an open brace", b"{", CopilotOutcome::Failed),
        ("free", b"{\"quota_reset_at\":0,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}", CopilotOutcome::NoBudget),
        ("unlimited", b"{\"quota_snapshots\":{\"premium_interactions\":{\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::NoBudget),
        ("no used field", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000}}}", CopilotOutcome::NoBudget),
        ("half a character in the stamp", b"{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"\\ud800-11-10T09:25:19Z\"}}}", CopilotOutcome::Failed),
        ("half a character in the reset", b"{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Failed),
        ("half a character in the reset, no budget", b"{\"quota_reset_date_utc\":\"\\ud800-12-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0}}}", CopilotOutcome::NoBudget),
        ("half a character in a text nobody reads", b"{\"copilot_plan\":\"\\ud800\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Ok),
        ("half a character in a name", b"{\"\\ud800\":1,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}", CopilotOutcome::Ok),
        ("half a character in a name, no budget", b"{\"\\ud800\":1}", CopilotOutcome::NoBudget),
        ("half a character in a name after, no budget", b"{\"quota_snapshots\":5,\"\\ud800\":1}", CopilotOutcome::NoBudget),
    ];
    for (name, body, outcome) in rows {
        let body = body.to_vec();
        let (result, _) = run(move || reply_bytes(200, &body));

        assert_eq!(result.outcome, *outcome, "{name}");
        assert_eq!(
            result.reading.is_some(),
            *outcome == CopilotOutcome::Ok,
            "{name}"
        );
    }
}

#[test]
fn retry_after_answers_as_the_csharp_did() {
    // What a 429 wrote in `Retry-After`, and the seconds to wait that the C# read: never
    // under zero, never over six hours. `now` is Tuesday 10 November 2026, 12:00:00 UTC. The
    // header is read by the function Claude Code's client reads it with, which
    // `live_usage_client::retry_after_answers_as_the_csharp_did` holds to 149 rows.
    let rows: &[(&str, Option<i64>)] = &[
        ("120", Some(120)),
        ("0", Some(0)),
        ("1", Some(1)),
        ("", None),
        (" ", None),
        ("abc", None),
        ("-1", None),
        ("1.5", None),
        ("21600", Some(21600)),
        ("21601", Some(21600)),
        ("999999999", Some(21600)),
        ("Tue, 10 Nov 2026 12:03:00 GMT", Some(180)),
        ("Tue, 10 Nov 2026 11:59:59 GMT", Some(0)),
        ("Fri, 31 Dec 9999 23:59:59 GMT", Some(21600)),
        ("Tuesday, 10-Nov-26 12:03:00 GMT", Some(180)),
        ("Tue Nov 10 12:03:00 2026", Some(180)),
    ];
    for (written, seconds) in rows {
        let written = written.to_string();
        let shown = written.clone();
        let (result, _) = run(move || too_many_requests(&written));

        assert_eq!(result.outcome, CopilotOutcome::RateLimited, "{shown:?}");
        assert_eq!(
            result.retry_after,
            seconds.map(TimeDelta::seconds),
            "{shown:?}"
        );
    }
}

#[test]
fn retry_after_is_read_off_a_429_and_off_no_other_status() {
    // The C# answered the same: a 403 that says how long to wait is a refusal like any other.
    for (status, outcome) in [
        (200, CopilotOutcome::Ok),
        (403, CopilotOutcome::Refused),
        (503, CopilotOutcome::Failed),
    ] {
        let (result, _) = run(move || {
            Ok(Answer {
                status,
                retry_after: Some("120".to_string()),
                body: BUSINESS.as_bytes().to_vec(),
            })
        });

        assert_eq!(result.outcome, outcome, "{status}");
        assert_eq!(result.retry_after, None, "{status}");
    }
    // A 429 with no such header has no wait.
    let (result, _) = run(|| reply(429, ""));
    assert_eq!(result.outcome, CopilotOutcome::RateLimited);
    assert_eq!(result.retry_after, None);
}
