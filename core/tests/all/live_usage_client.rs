//! Ported from `tests/Sobreiro.Tests/LiveUsageClientTests.cs`. The tests after
//! `a_far_future_retry_after_is_capped_at_six_hours` are new.
//!
//! No test here reaches the network: the client is handed a sender that answers as the test
//! says. The token is made up. What the real sender does on the wire is in `http.rs`.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::io::http::{Answer, NoAnswer};
use sobreiro_core::live::live_usage_client::{self, Cancelled, LiveUsageClient};
use sobreiro_core::live::{AccessToken, LiveOutcome, LiveResult};

use crate::{FakeSender, Seen, reply, reply_bytes, time, too_many_requests};

const SECRET: &str = "t0k3n";
const BODY: &str =
    "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

fn now() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
}

/// Pieces of a body, one after the other.
fn joined(pieces: &[&[u8]]) -> Vec<u8> {
    pieces.concat()
}

fn token(value: &str) -> AccessToken {
    AccessToken::new(value.to_string(), None)
}

/// Asks once, with the made-up token, and gives the result and the sender that was asked.
fn run(
    respond: impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static,
) -> (LiveResult, Arc<FakeSender>) {
    let sender = Arc::new(FakeSender::new(respond));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");
    let result = client
        .fetch(&token(SECRET), now(), &AtomicBool::new(false))
        .unwrap();
    (result, sender)
}

#[test]
fn a_200_gives_limits_and_sends_the_exact_request() {
    let (result, sender) = run(|| reply(200, BODY));

    assert_eq!(result.outcome, LiveOutcome::Ok);
    let five_hour = result.limits.unwrap().five_hour.unwrap();
    assert_eq!(five_hour.used_percentage, 27.0);

    // The seam has one way to ask, a GET: `http::a_request_goes_out_as_it_was_asked` holds
    // that word on the wire.
    let header = |name: &str, value: &str| (name.to_string(), value.to_string());
    assert_eq!(
        sender.seen(),
        [Seen {
            url: "https://api.anthropic.com/api/oauth/usage".to_string(),
            headers: vec![
                header("Authorization", "Bearer t0k3n"),
                header("anthropic-beta", "oauth-2025-04-20"),
                header("Accept", "application/json"),
                header("User-Agent", "Sobreiro/0.3.0"),
            ],
        }]
    );
    assert_eq!(
        live_usage_client::ENDPOINT,
        "https://api.anthropic.com/api/oauth/usage"
    );
}

#[test]
fn a_401_or_403_is_unauthorized() {
    for status in [401, 403] {
        let (result, _) = run(move || reply(status, ""));

        assert_eq!(result.outcome, LiveOutcome::Unauthorized, "{status}");
        assert_eq!(result.limits, None, "{status}");
    }
}

#[test]
fn a_429_with_seconds_gives_that_retry_after() {
    let (result, _) = run(|| too_many_requests("120"));

    assert_eq!(result.outcome, LiveOutcome::RateLimited);
    assert_eq!(result.retry_after, Some(TimeDelta::minutes(2)));
}

#[test]
fn a_429_with_an_http_date_gives_the_gap_from_now() {
    // Three minutes after `now`, as HTTP writes a date.
    let (result, _) = run(|| too_many_requests("Mon, 09 Nov 2026 12:03:00 GMT"));

    assert_eq!(result.outcome, LiveOutcome::RateLimited);
    let seconds = result.retry_after.unwrap().num_seconds();
    assert!((179..=181).contains(&seconds), "{seconds}");
}

#[test]
fn a_429_without_retry_after_has_none() {
    let (result, _) = run(|| reply(429, ""));

    assert_eq!(result.outcome, LiveOutcome::RateLimited);
    assert_eq!(result.retry_after, None);
}

#[test]
fn a_500_is_failed() {
    let (result, _) = run(|| reply(500, ""));

    assert_eq!(result.outcome, LiveOutcome::Failed);
}

#[test]
fn a_redirect_is_failed_and_not_followed() {
    // An answer as the client is handed it has no place for the address a redirect names, so
    // the client could not go there if it wanted to. That the real sender does not follow one
    // either is `http::a_redirect_comes_back_as_the_answer_and_is_not_followed`.
    let (result, sender) = run(|| reply(302, ""));

    assert_eq!(result.outcome, LiveOutcome::Failed);
    assert_eq!(sender.seen().len(), 1);
}

#[test]
fn a_200_with_an_unparseable_body_is_failed() {
    for body in ["not json", "{}", ""] {
        let (result, _) = run(move || reply(200, body));

        assert_eq!(result.outcome, LiveOutcome::Failed, "{body}");
        assert_eq!(result.limits, None, "{body}");
    }
}

#[test]
fn network_errors_and_timeouts_are_unreachable_without_the_token_in_the_text() {
    // The C# test hands in three errors with the token in their words. Here a sender has no
    // words to hand back: `NoAnswer` is one of two names and nothing more.
    let (result, _) = run(|| Err(NoAnswer::Unreachable));

    assert_eq!(result.outcome, LiveOutcome::Unreachable);
    assert!(!format!("{result:?}").contains(SECRET));
}

#[test]
fn cancelling_by_the_caller_propagates() {
    let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
    let client = LiveUsageClient::new(sender.clone(), "ua");
    let cancelled = AtomicBool::new(true);

    let result = client.fetch(&token(SECRET), now(), &cancelled);

    assert_eq!(result, Err(Cancelled));
    // Cancelled before it began: nothing was sent.
    assert_eq!(sender.seen(), []);
}

#[test]
fn no_result_text_contains_the_token() {
    for status in [200, 401, 429, 302, 500] {
        let (result, _) = run(move || reply(status, BODY));

        assert!(!format!("{result:?}").contains(SECRET), "{status}");
        assert!(!format!("{result:#?}").contains(SECRET), "{status}");
    }
}

#[test]
fn the_timeout_is_ten_seconds() {
    assert_eq!(live_usage_client::TIMEOUT, Duration::from_secs(10));
}

#[test]
fn a_non_network_exception_while_reading_the_body_is_failed_not_thrown() {
    // A sender that could not make sense of the answer says `Failed`, and so does the client.
    let (result, _) = run(|| Err(NoAnswer::Failed));

    assert_eq!(result.outcome, LiveOutcome::Failed);
    assert!(!format!("{result:?}").contains(SECRET));
}

#[test]
fn a_far_future_retry_after_is_capped_at_six_hours() {
    let (result, _) = run(|| too_many_requests("Fri, 31 Dec 9999 23:59:59 GMT"));
    assert_eq!(result.retry_after, Some(TimeDelta::hours(6)));

    let (seconds, _) = run(|| too_many_requests("999999999"));
    assert_eq!(seconds.retry_after, Some(TimeDelta::hours(6)));
}

// What follows is new.

#[test]
fn the_request_goes_to_the_one_address_whatever_the_answer() {
    let answers: [fn() -> Result<Answer, NoAnswer>; 7] = [
        || reply(200, BODY),
        || reply(302, ""),
        || reply(401, ""),
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
        assert_eq!(seen[0].url, "https://api.anthropic.com/api/oauth/usage");
    }
}

#[test]
fn cancelling_while_the_request_is_under_way_drops_the_answer() {
    // The sender here stands in for a request that was still out when the caller cancelled.
    let cancelled = Arc::new(AtomicBool::new(false));
    let seen_by_sender = cancelled.clone();
    let sender = Arc::new(FakeSender::new(move || {
        seen_by_sender.store(true, Ordering::SeqCst);
        reply(200, BODY)
    }));
    let client = LiveUsageClient::new(sender, "ua");

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
        let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
        let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");

        let result = client.fetch(&token(value), now(), &AtomicBool::new(false));

        assert_eq!(
            result.unwrap().outcome,
            LiveOutcome::Failed,
            "token {value:?}"
        );
        assert_eq!(sender.seen(), [], "token {value:?}");
    }
    for user_agent in ["a\nb", "\u{e9}"] {
        let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
        let client = LiveUsageClient::new(sender.clone(), user_agent);

        let result = client.fetch(&token(SECRET), now(), &AtomicBool::new(false));

        assert_eq!(
            result.unwrap().outcome,
            LiveOutcome::Failed,
            "{user_agent:?}"
        );
        assert_eq!(sender.seen(), [], "{user_agent:?}");
    }
}

#[test]
fn a_token_with_spaces_and_signs_is_sent_as_it_is() {
    for value in [" ", "a b", " a", "a ", "a,b", "a\"b"] {
        let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
        let client = LiveUsageClient::new(sender.clone(), "Sobreiro/1.1.0 (Windows)");

        let result = client.fetch(&token(value), now(), &AtomicBool::new(false));

        assert_eq!(result.unwrap().outcome, LiveOutcome::Ok, "{value:?}");
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
fn a_body_is_read_as_utf_8() {
    let utf16 = |text: &str| -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        bytes
    };
    // A body, and whether limits come out of it. The C# answered the same but where a note
    // says otherwise.
    let rows: Vec<(Vec<u8>, bool)> = vec![
        (BODY.as_bytes().to_vec(), true),
        // The three bytes that may start a UTF-8 text are skipped, once.
        (joined(&[b"\xEF\xBB\xBF", BODY.as_bytes()]), true),
        (
            joined(&[b"\xEF\xBB\xBF\xEF\xBB\xBF", BODY.as_bytes()]),
            false,
        ),
        (joined(&[BODY.as_bytes(), b"\r\n"]), true),
        (joined(&[BODY.as_bytes(), b"\0"]), false),
        (joined(&[BODY.as_bytes(), b"\xFF"]), false),
        // A byte that is no text, inside a text nobody reads.
        (
            b"{\"x\":\"a\xFFb\",\"five_hour\":{\"utilization\":27}}".to_vec(),
            true,
        ),
        // The same inside the group's name: it is no longer `session`.
        (
            b"{\"limits\":[{\"group\":\"sess\xFFion\",\"percent\":27}]}".to_vec(),
            false,
        ),
        (b"  ".to_vec(), false),
        // JSON between programs is UTF-8 by its standard. The C# also read a body in UTF-16
        // or UTF-32 that began with its mark; here it is not JSON.
        (utf16(BODY), false),
    ];
    for (body, has_limits) in rows {
        let shown = String::from_utf8_lossy(&body).into_owned();
        let (result, _) = run(move || reply_bytes(200, &body));

        let expected = if has_limits {
            LiveOutcome::Ok
        } else {
            LiveOutcome::Failed
        };
        assert_eq!(result.outcome, expected, "{shown}");
    }
}

// The rows below were written out by the C# of 1.1.0, run on 2026-11-14 with a stand-in for
// the network, and turned into Rust by a script, not typed.

#[test]
fn every_status_answers_as_the_csharp_did() {
    // A status, with the body of limits, and the outcome.
    let rows: &[(u16, LiveOutcome)] = &[
        (100, LiveOutcome::Failed),
        (101, LiveOutcome::Failed),
        (102, LiveOutcome::Failed),
        (103, LiveOutcome::Failed),
        (200, LiveOutcome::Ok),
        (201, LiveOutcome::Ok),
        (202, LiveOutcome::Ok),
        (203, LiveOutcome::Ok),
        (204, LiveOutcome::Ok),
        (205, LiveOutcome::Ok),
        (206, LiveOutcome::Ok),
        (207, LiveOutcome::Ok),
        (208, LiveOutcome::Ok),
        (209, LiveOutcome::Ok),
        (210, LiveOutcome::Ok),
        (211, LiveOutcome::Ok),
        (212, LiveOutcome::Ok),
        (213, LiveOutcome::Ok),
        (214, LiveOutcome::Ok),
        (215, LiveOutcome::Ok),
        (216, LiveOutcome::Ok),
        (217, LiveOutcome::Ok),
        (218, LiveOutcome::Ok),
        (219, LiveOutcome::Ok),
        (220, LiveOutcome::Ok),
        (221, LiveOutcome::Ok),
        (222, LiveOutcome::Ok),
        (223, LiveOutcome::Ok),
        (224, LiveOutcome::Ok),
        (225, LiveOutcome::Ok),
        (226, LiveOutcome::Ok),
        (300, LiveOutcome::Failed),
        (301, LiveOutcome::Failed),
        (302, LiveOutcome::Failed),
        (303, LiveOutcome::Failed),
        (304, LiveOutcome::Failed),
        (305, LiveOutcome::Failed),
        (306, LiveOutcome::Failed),
        (307, LiveOutcome::Failed),
        (308, LiveOutcome::Failed),
        (400, LiveOutcome::Failed),
        (401, LiveOutcome::Unauthorized),
        (402, LiveOutcome::Failed),
        (403, LiveOutcome::Unauthorized),
        (404, LiveOutcome::Failed),
        (405, LiveOutcome::Failed),
        (406, LiveOutcome::Failed),
        (407, LiveOutcome::Failed),
        (408, LiveOutcome::Failed),
        (409, LiveOutcome::Failed),
        (410, LiveOutcome::Failed),
        (411, LiveOutcome::Failed),
        (412, LiveOutcome::Failed),
        (413, LiveOutcome::Failed),
        (414, LiveOutcome::Failed),
        (415, LiveOutcome::Failed),
        (416, LiveOutcome::Failed),
        (417, LiveOutcome::Failed),
        (418, LiveOutcome::Failed),
        (419, LiveOutcome::Failed),
        (420, LiveOutcome::Failed),
        (421, LiveOutcome::Failed),
        (422, LiveOutcome::Failed),
        (423, LiveOutcome::Failed),
        (424, LiveOutcome::Failed),
        (425, LiveOutcome::Failed),
        (426, LiveOutcome::Failed),
        (427, LiveOutcome::Failed),
        (428, LiveOutcome::Failed),
        (429, LiveOutcome::RateLimited),
        (430, LiveOutcome::Failed),
        (431, LiveOutcome::Failed),
        (500, LiveOutcome::Failed),
        (501, LiveOutcome::Failed),
        (502, LiveOutcome::Failed),
        (503, LiveOutcome::Failed),
        (504, LiveOutcome::Failed),
        (505, LiveOutcome::Failed),
        (506, LiveOutcome::Failed),
        (507, LiveOutcome::Failed),
        (508, LiveOutcome::Failed),
        (509, LiveOutcome::Failed),
        (510, LiveOutcome::Failed),
        (511, LiveOutcome::Failed),
    ];
    for (status, outcome) in rows {
        let status = *status;
        let (result, _) = run(move || reply(status, BODY));

        assert_eq!(result.outcome, *outcome, "{status}");
        // Limits come with `Ok` and with nothing else.
        assert_eq!(
            result.limits.is_some(),
            *outcome == LiveOutcome::Ok,
            "{status}"
        );
    }
}

#[test]
fn retry_after_answers_as_the_csharp_did() {
    // What a 429 wrote in `Retry-After`, and the seconds to wait that the C# read: never
    // under zero, never over six hours. `now` is Monday 9 November 2026, 12:00:00 UTC.
    let rows: &[(&str, Option<i64>)] = &[
        ("120", Some(120)),
        ("0", Some(0)),
        ("1", Some(1)),
        (" 120", Some(120)),
        ("120 ", Some(120)),
        ("\t120", Some(120)),
        ("+120", None),
        ("-1", None),
        ("-0", None),
        ("1.5", None),
        ("abc", None),
        ("", None),
        (" ", None),
        ("007", Some(7)),
        ("21600", Some(21600)),
        ("21601", Some(21600)),
        ("2147483647", Some(21600)),
        ("2147483648", None),
        ("99999999999", None),
        ("999999999", Some(21600)),
        ("120, 130", None),
        ("120;x", None),
        ("\"120\"", None),
        ("120s", None),
        ("1e2", None),
        ("0x10", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:00:00 GMT", Some(0)),
        ("Mon, 09 Nov 2026 11:59:59 GMT", Some(0)),
        ("Mon, 09 Nov 2026 18:00:00 GMT", Some(21600)),
        ("Mon, 09 Nov 2026 18:00:01 GMT", Some(21600)),
        ("Fri, 31 Dec 9999 23:59:59 GMT", Some(21600)),
        ("Monday, 09-Nov-26 12:03:00 GMT", Some(180)),
        ("Mon Nov  9 12:03:00 2026", Some(180)),
        ("Mon Nov 9 12:03:00 2026", Some(180)),
        ("Tue, 09 Nov 2026 12:03:00 GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 UTC", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 +0100", Some(0)),
        ("Mon, 09 Nov 2026 12:03:00 -0800", Some(21600)),
        ("Mon, 09 Nov 2026 12:03:00 Z", None),
        ("Mon, 09 Nov 2026 12:03:00", Some(180)),
        ("09 Nov 2026 12:03:00 GMT", None),
        ("mon, 09 nov 2026 12:03:00 gmt", None),
        ("Mon, 9 Nov 2026 12:03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:03 GMT", None),
        ("Mon, 09 Nov 26 12:03:00 GMT", Some(180)),
        (" Mon, 09 Nov 2026 12:03:00 GMT ", Some(180)),
        ("Mon,09 Nov 2026 12:03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 EST", None),
        ("Mon, 09 Nov 2026 12:03:00 PST", None),
        ("2026-11-09T12:03:00Z", None),
        ("Mon, 09 Nov 2026 12:03:00.5 GMT", None),
        ("Mon, 09 Nov 2026 24:00:00 GMT", None),
        ("Sun, 06 Nov 1994 08:49:37 GMT", Some(0)),
        ("Sunday, 06-Nov-94 08:49:37 GMT", Some(0)),
        ("Sun Nov  6 08:49:37 1994", Some(0)),
        ("Mon, 30 Feb 2026 12:03:00 GMT", None),
        ("Mon, 09 Nov 2026 12:03:60 GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT, 120", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT+1", None),
        ("Mon, 09 Nov 2026 12:03:00 CET", None),
        ("9 Nov 2026 12:03:00 GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 A", None),
        ("Mon, 09 Nov 2026 12:03:00 +00:00", Some(180)),
        ("Monday, 09 Nov 2026 12:03:00 GMT", None),
        ("MON, 09 NOV 2026 12:03:00 GMT", Some(180)),
        ("mon, 09 nov 2026 12:03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 gmt", None),
        ("Mon , 09 Nov 2026 12:03:00 GMT", Some(180)),
        ("Mon,  09  Nov  2026  12:03:00  GMT", Some(180)),
        ("Mon,09Nov2026 12:03:00GMT", Some(180)),
        ("Mon, 09 Nov 2026 1:2:3 GMT", Some(0)),
        ("Mon, 09 Nov 2026 01:02:03 GMT", Some(0)),
        ("Mon, 009 Nov 2026 12:03:00 GMT", None),
        ("Mon, 09 Nov 02026 12:03:00 GMT", None),
        ("Mon, 09 Nov 202 12:03:00 GMT", None),
        ("Mon, 09 November 2026 12:03:00 GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 +1:00", Some(0)),
        ("Mon, 09 Nov 2026 12:03:00 +100", None),
        ("Mon, 09 Nov 2026 12:03:00 +01", None),
        ("Mon, 09 Nov 2026 12:03:00 +1400", Some(0)),
        ("Mon, 09 Nov 2026 12:03:00 +1401", None),
        ("Mon, 09 Nov 2026 12:03:00 -1400", Some(21600)),
        ("Mon, 09 Nov 2026 12:03:00 +0060", None),
        ("Mon, 09 Nov 2026 12:03:00+0100", Some(0)),
        ("Mon, 09 Nov 2026 12:03:00 + 0100", None),
        ("Mon, 09 Nov 26 12:03:00 +0100", None),
        ("Mon, 09 Nov 26 12:03:00 UTC", Some(180)),
        ("Mon, 09 Nov 26 12:03:00", Some(180)),
        ("Monday, 09-Nov-26 12:03:00 UTC", Some(180)),
        ("Monday, 09-Nov-26 12:03:00 +0100", Some(0)),
        ("Monday, 09-Nov-26 12:03:00", Some(180)),
        ("Monday, 09-Nov-2026 12:03:00 GMT", None),
        ("monday, 09-nov-26 12:03:00 GMT", Some(180)),
        ("Monday,09-Nov-26 12:03:00 GMT", Some(180)),
        ("Monday, 9-Nov-26 12:03:00 GMT", Some(180)),
        ("Monday, 09 - Nov - 26 12:03:00 GMT", Some(180)),
        ("Tuesday, 09-Nov-26 12:03:00 GMT", None),
        ("Saturday, 06-Nov-49 08:49:37 GMT", Some(21600)),
        ("Monday, 06-Nov-50 08:49:37 GMT", Some(0)),
        ("Mon, 09-Nov-26 12:03:00 GMT", None),
        ("Mon Nov 09 12:03:00 2026", Some(180)),
        ("Mon Nov 9 12:03:00 26", None),
        ("Mon Nov 9 12:03:00 2026 GMT", None),
        ("mon nov 9 12:03:00 2026", Some(180)),
        ("Mon  Nov  9  12:03:00  2026", Some(180)),
        ("MonNov9 12:03:00 2026", Some(180)),
        ("Mon Nov 9 12:03:00 2026 +0100", None),
        ("Tue Nov 9 12:03:00 2026", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT\t", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00\u{a0}GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 UT", None),
        ("Mon, 01 Jan 0001 00:00:00 GMT", Some(0)),
        ("Mon, 01 Jan 0001 00:00:00 +0100", None),
        ("Fri, 31 Dec 9999 23:59:59 -0100", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 GMT x", None),
        ("Mon, 09 Nov 2026, 12:03:00 GMT", None),
        ("Mon, 09 Nov 2026 12:03:00 -0000", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 +0000", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 0100", None),
        ("Mon, 09 Nov 2026 12 :03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12: 03:00 GMT", Some(180)),
        ("Mon, 32 Nov 2026 12:03:00 GMT", None),
        ("Mon, 0 Nov 2026 12:03:00 GMT", None),
        ("Sun, 29 Feb 2032 12:03:00 GMT", Some(21600)),
        ("Mon, 09 Nov 2026 12:03:00 GMTx", None),
        ("1 2", None),
        ("12\t", Some(12)),
        ("\u{ff11}\u{ff12}\u{ff10}", None),
        ("\u{663}", None),
        ("120\u{a0}", None),
        ("\u{a0}120", None),
        ("0000000000120", None),
        ("00000000002147483648", None),
        ("Monday, 09-Nov-26 12:03:00 +01:00", Some(0)),
        ("Monday, 09-Nov-26 12:03:00 Z", None),
        ("Mon, 09 Nov 2026 12:03:00 +01:0", None),
        ("Mon, 09 Nov 2026 12:03:00 +123", None),
        ("Mon, 09 Nov 2026 12:03:00 +12345", None),
        ("Mon, 09 Nov 2026 12:03:00 +1", None),
        ("Sun, 31 Dec 0000 23:30:00 -0100", None),
        ("Sun, 31 Dec 0000 23:30:00 GMT", None),
        ("0000000120", Some(120)),
        ("0000000000", Some(0)),
        ("4294967296", None),
        ("Mon, 09 Nov 2612:03:00 GMT", Some(180)),
        ("Mon, 09 Nov 2026 12:03:00 -1401", None),
        ("Mon, 09 Nov 2026 12:03:00 +14:00", Some(0)),
    ];
    for (written, seconds) in rows {
        let written = written.to_string();
        let shown = written.clone();
        let (result, _) = run(move || too_many_requests(&written));

        assert_eq!(result.outcome, LiveOutcome::RateLimited, "{shown:?}");
        assert_eq!(
            result.retry_after,
            seconds.map(TimeDelta::seconds),
            "{shown:?}"
        );
    }
}
