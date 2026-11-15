//! Ported from `tests/Sobreiro.Tests/LivePollTests.cs`. The tests after
//! `the_callers_cancellation_leaves_as_a_cancellation` are new.
//!
//! No test here reaches the network: the client is handed a sender that answers as the test
//! says. The token is made up, and the sign-in file is one the test writes in its own folder.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::io::http::{Answer, NoAnswer};
use sobreiro_core::live::live_poll::{self, LivePollResult, PollOutcome};
use sobreiro_core::live::live_schedule::{LiveSchedule, RefusedSignIn};
use sobreiro_core::live::live_usage_client::{self, Cancelled, LiveUsageClient};
use sobreiro_core::live::{AccessToken, CredentialRead};
use sobreiro_core::polling::ProviderState;

use crate::{FakeSender, Seen, TestFolder, reply, time, too_many_requests};

const SECRET: &str = "sk-ant-oat01-SECRETVALUE";
const BODY: &str =
    "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

fn now() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
}

fn minutes(count: i64) -> TimeDelta {
    TimeDelta::minutes(count)
}

fn hours(count: i64) -> TimeDelta {
    TimeDelta::hours(count)
}

fn start() -> LiveSchedule {
    LiveSchedule::start(now())
}

fn credentials_path(folder: &TestFolder) -> PathBuf {
    folder.file(".credentials.json")
}

fn write_credentials(folder: &TestFolder, text: &str) -> PathBuf {
    folder.create();
    let path = credentials_path(folder);
    std::fs::write(&path, text).unwrap();
    path
}

/// A sign-in file with the made-up token `token`, which runs out at `expires_at`.
fn sign_in(token: &str, expires_at: Option<DateTime<Utc>>) -> String {
    let expiry = expires_at.map_or(String::new(), |at| {
        format!(",\"expiresAt\":{}", at.timestamp_millis())
    });
    format!("{{\"claudeAiOauth\":{{\"accessToken\":\"{token}\",\"refreshToken\":\"r\"{expiry}}}}}")
}

fn credentials_expiring(folder: &TestFolder, expires_at: Option<DateTime<Utc>>) -> PathBuf {
    write_credentials(folder, &sign_in(SECRET, expires_at))
}

fn valid_credentials(folder: &TestFolder) -> PathBuf {
    credentials_expiring(folder, Some(now() + hours(2)))
}

/// One poll, with a sender that answers as `respond` says. Gives the result and what the
/// sender was asked.
fn run(
    path: &Path,
    respond: impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static,
    schedule: &LiveSchedule,
    at: DateTime<Utc>,
) -> (LivePollResult, Vec<Seen>) {
    let sender = Arc::new(FakeSender::new(respond));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");
    let result = live_poll::run(path, &client, schedule, at, &AtomicBool::new(false)).unwrap();
    (result, sender.seen())
}

/// A sender for a poll that must send nothing. The test fails when it is asked.
fn must_not_be_called() -> Result<Answer, NoAnswer> {
    panic!("Live usage sent a request without a usable token.")
}

/// What the one request held in its `Authorization` header.
fn authorization(seen: &[Seen]) -> String {
    assert_eq!(seen.len(), 1);
    seen[0]
        .headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| value.clone())
        .unwrap()
}

#[test]
fn missing_credentials_send_nothing_and_say_no_sign_in() {
    let folder = TestFolder::new("live_poll-missing");
    let (result, seen) = run(
        &credentials_path(&folder),
        must_not_be_called,
        &start(),
        now(),
    );

    assert!(seen.is_empty());
    assert_eq!(result.state, ProviderState::NoSignIn);
    assert_eq!(result.limits, None);
    assert_eq!(result.schedule, start().after_no_credentials(now()));
    assert_eq!(result.outcome.name(), "Missing");
}

#[test]
fn unreadable_credentials_send_nothing_and_say_no_sign_in() {
    for (index, text) in [
        "not json",
        "{\"claudeAiOauth\":{}}",
        "{\"claudeAiOauth\":{\"accessToken\":\"\"}}",
    ]
    .into_iter()
    .enumerate()
    {
        let folder = TestFolder::new(&format!("live_poll-unreadable-{index}"));
        let path = write_credentials(&folder, text);
        let (result, seen) = run(&path, must_not_be_called, &start(), now());

        assert!(seen.is_empty(), "{text}");
        assert_eq!(result.state, ProviderState::NoSignIn, "{text}");
        assert_eq!(result.limits, None, "{text}");
        assert_eq!(
            result.schedule,
            start().after_no_credentials(now()),
            "{text}"
        );
        assert_eq!(result.outcome.name(), "Unreadable", "{text}");
    }
}

#[test]
fn expired_credentials_send_nothing_and_say_expired() {
    let folder = TestFolder::new("live_poll-expired");
    let text = format!(
        "{{\"claudeAiOauth\":{{\"accessToken\":\"{SECRET}\",\"expiresAt\":{}}}}}",
        (now() - minutes(1)).timestamp_millis()
    );
    let path = write_credentials(&folder, &text);
    let (result, seen) = run(&path, must_not_be_called, &start(), now());

    assert!(seen.is_empty());
    assert_eq!(result.state, ProviderState::Expired);
    assert_eq!(result.limits, None);
    assert_eq!(result.schedule, start().after_no_credentials(now()));
    assert_eq!(result.outcome.name(), "Expired");
}

#[test]
fn the_read_is_asked_with_the_poll_time() {
    let client = LiveUsageClient::new(
        Arc::new(FakeSender::new(must_not_be_called)),
        "Sobreiro/0.3.0",
    );
    let mut asked = None;
    live_poll::run_with(
        |at| {
            asked = Some(at);
            CredentialRead::Missing
        },
        &client,
        &start(),
        now(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(asked, Some(now()));
}

#[test]
fn a_good_reply_gives_the_limits_and_state_ok() {
    let folder = TestFolder::new("live_poll-good");
    let (result, seen) = run(
        &valid_credentials(&folder),
        || reply(200, BODY),
        &start(),
        now(),
    );

    assert_eq!(authorization(&seen), format!("Bearer {SECRET}"));
    assert_eq!(result.state, ProviderState::Ok);
    let session = result.limits.unwrap().five_hour.unwrap();
    assert_eq!(session.used_percentage, 27.0);
    assert_eq!(result.schedule.last_success, Some(now()));
    assert_eq!(result.schedule.next_due, now() + minutes(5));
    assert_eq!(result.outcome.name(), "Ok");
}

#[test]
fn a_429_is_rate_limited_and_waits() {
    let folder = TestFolder::new("live_poll-429");
    let (result, _) = run(
        &valid_credentials(&folder),
        || too_many_requests("600"),
        &start(),
        now(),
    );

    assert_eq!(result.state, ProviderState::RateLimited);
    assert_eq!(result.limits, None);
    assert_eq!(result.schedule.next_due, now() + minutes(10));
    assert_eq!(result.schedule.wait_until, Some(now() + minutes(10)));
    assert_eq!(result.outcome.name(), "RateLimited");
}

#[test]
fn other_replies_are_a_bad_answer_and_retry_in_two_minutes() {
    for status in [500, 302] {
        let folder = TestFolder::new(&format!("live_poll-other-{status}"));
        let path = valid_credentials(&folder);
        let (result, seen) = run(&path, move || reply(status, ""), &start(), now());

        assert_eq!(seen.len(), 1, "{status}");
        assert_eq!(result.state, ProviderState::BadAnswer, "{status}");
        assert_eq!(result.limits, None, "{status}");
        assert_eq!(result.schedule.next_due, now() + minutes(2), "{status}");
        assert_eq!(result.outcome.name(), "Failed", "{status}");

        // The same sign-in is tried again once the retry is due.
        let (_, seen) = run(
            &path,
            move || reply(status, ""),
            &result.schedule,
            now() + minutes(2),
        );
        assert_eq!(seen.len(), 1, "{status}");
    }
}

#[test]
fn a_200_with_an_unexpected_body_is_a_bad_answer() {
    let folder = TestFolder::new("live_poll-unexpected");
    let (result, _) = run(
        &valid_credentials(&folder),
        || reply(200, "{\"unexpected\":true}"),
        &start(),
        now(),
    );

    assert_eq!(result.state, ProviderState::BadAnswer);
    assert_eq!(result.limits, None);
    assert_eq!(result.outcome.name(), "Failed");
}

// Two C# tests in one: a request that timed out and a host that could not be reached are the
// same answer from a Rust sender.
#[test]
fn an_unreachable_host_or_a_request_timing_out_is_offline() {
    let folder = TestFolder::new("live_poll-offline");
    let path = valid_credentials(&folder);
    let (result, _) = run(&path, || Err(NoAnswer::Unreachable), &start(), now());

    assert_eq!(result.state, ProviderState::Offline);
    assert_eq!(result.limits, None);
    assert_eq!(result.schedule.next_due, now() + minutes(2));
    assert_eq!(result.outcome.name(), "Unreachable");

    let (_, seen) = run(
        &path,
        || Err(NoAnswer::Unreachable),
        &result.schedule,
        now() + minutes(2),
    );
    assert_eq!(seen.len(), 1);
}

#[test]
fn a_refused_sign_in_is_not_sent_again_until_the_file_gives_a_new_one() {
    for status in [401, 403] {
        let folder = TestFolder::new(&format!("live_poll-refused-{status}"));
        let path = credentials_expiring(&folder, Some(now() + hours(2)));
        let (mut result, seen) = run(&path, move || reply(status, ""), &start(), now());

        assert_eq!(seen.len(), 1, "{status}");
        assert_eq!(result.state, ProviderState::Refused, "{status}");
        assert_eq!(result.limits, None, "{status}");
        assert_eq!(
            result.schedule.refused,
            Some(RefusedSignIn {
                expires_at: Some(now() + hours(2))
            }),
            "{status}"
        );
        assert_eq!(result.schedule.next_due, now() + minutes(1), "{status}");
        assert_eq!(result.outcome.name(), "Unauthorized", "{status}");

        // Each due tick reads the file again but sends nothing while it gives the same expiry.
        let mut schedule = result.schedule;
        for wait in [1, 2, 3] {
            let at = now() + minutes(wait);
            assert!(schedule.is_due(at), "{status}");
            let (silent, seen) = run(&path, must_not_be_called, &schedule, at);
            result = silent;

            assert!(seen.is_empty(), "{status}");
            assert_eq!(result.state, ProviderState::Refused, "{status}");
            assert_eq!(result.outcome.name(), "Refused", "{status}");
            assert_eq!(result.schedule.next_due, at + minutes(1), "{status}");
            schedule = result.schedule;
        }

        // Claude Code saved a new sign-in: it is sent at once.
        credentials_expiring(&folder, Some(now() + hours(10)));
        let (result, seen) = run(&path, || reply(200, BODY), &schedule, now() + minutes(4));

        assert_eq!(seen.len(), 1, "{status}");
        assert_eq!(result.state, ProviderState::Ok, "{status}");
        assert_eq!(result.schedule.refused, None, "{status}");
    }
}

#[test]
fn a_refused_sign_in_without_an_expiry_waits_for_a_file_with_one() {
    let folder = TestFolder::new("live_poll-refused-no-expiry");
    let path = credentials_expiring(&folder, None);
    let (result, _) = run(&path, || reply(401, ""), &start(), now());
    assert_eq!(
        result.schedule.refused,
        Some(RefusedSignIn { expires_at: None })
    );

    let (result, seen) = run(
        &path,
        must_not_be_called,
        &result.schedule,
        now() + minutes(1),
    );
    assert!(seen.is_empty());
    assert_eq!(result.state, ProviderState::Refused);

    credentials_expiring(&folder, Some(now() + hours(2)));
    let (result, seen) = run(
        &path,
        || reply(200, BODY),
        &result.schedule,
        now() + minutes(2),
    );
    assert_eq!(seen.len(), 1);
    assert_eq!(result.state, ProviderState::Ok);
}

#[test]
fn a_success_after_a_429_resets_the_wait() {
    let folder = TestFolder::new("live_poll-reset");
    let waiting = LiveSchedule {
        rate_limit_step: 2,
        wait_until: Some(now() - minutes(1)),
        ..start()
    };
    let (result, _) = run(
        &valid_credentials(&folder),
        || reply(200, BODY),
        &waiting,
        now(),
    );

    assert_eq!(result.schedule.rate_limit_step, 0);
    assert_eq!(result.schedule.wait_until, None);
}

#[test]
fn the_token_never_appears_in_a_result() {
    for status in [200, 401, 429] {
        let folder = TestFolder::new(&format!("live_poll-printed-{status}"));
        let (result, _) = run(
            &valid_credentials(&folder),
            move || reply(status, BODY),
            &start(),
            now(),
        );
        // The short way of printing a value and the long one.
        assert!(!format!("{result:?}").contains(SECRET), "{status}");
        assert!(!format!("{result:#?}").contains(SECRET), "{status}");
    }
}

#[test]
fn the_callers_cancellation_leaves_as_a_cancellation() {
    let folder = TestFolder::new("live_poll-cancelled");
    let path = valid_credentials(&folder);
    let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");

    let result = live_poll::run(&path, &client, &start(), now(), &AtomicBool::new(true));

    assert_eq!(result, Err(Cancelled));
    assert!(sender.seen().is_empty());
}

// ---- New --------------------------------------------------------------------------------------

// "Per request": the token is read from the sign-in file for each poll and kept nowhere
// between two. One client and one schedule, two polls: each sends what the file holds then,
// and with the file gone nothing is sent.
#[test]
fn each_poll_reads_the_sign_in_file_again_and_sends_what_it_holds_then() {
    let folder = TestFolder::new("live_poll-per-request");
    let path = write_credentials(&folder, &sign_in("first-made-up", Some(now() + hours(2))));
    let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");
    let not_cancelled = AtomicBool::new(false);

    let first = live_poll::run(&path, &client, &start(), now(), &not_cancelled).unwrap();
    write_credentials(&folder, &sign_in("second-made-up", Some(now() + hours(9))));
    let at = now() + minutes(5);
    let second = live_poll::run(&path, &client, &first.schedule, at, &not_cancelled).unwrap();
    std::fs::remove_file(&path).unwrap();
    let at = now() + minutes(10);
    let third = live_poll::run(&path, &client, &second.schedule, at, &not_cancelled).unwrap();

    let sent: Vec<String> = sender
        .seen()
        .iter()
        .map(|seen| authorization(std::slice::from_ref(seen)))
        .collect();
    assert_eq!(sent, ["Bearer first-made-up", "Bearer second-made-up"]);
    assert_eq!(third.state, ProviderState::NoSignIn);
    assert_eq!(third.schedule.last_success, Some(now() + minutes(5)));
}

// "Never renew": a sign-in that ran out or was refused leads to no request at all, so to none
// that could renew it, and the file is left byte for byte as it was. Whatever is sent goes to
// the one address.
#[test]
fn a_sign_in_that_ran_out_or_was_refused_is_neither_sent_nor_touched() {
    let folder = TestFolder::new("live_poll-never-renew");
    let expired = sign_in(SECRET, Some(now() - minutes(1)));
    let path = write_credentials(&folder, &expired);
    let (result, seen) = run(&path, must_not_be_called, &start(), now());
    assert_eq!(result.state, ProviderState::Expired);
    assert!(seen.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expired);

    let good = sign_in(SECRET, Some(now() + hours(2)));
    write_credentials(&folder, &good);
    let (refused, seen) = run(
        &path,
        || reply(401, ""),
        &result.schedule,
        now() + minutes(1),
    );
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].url, live_usage_client::ENDPOINT);
    assert_eq!(refused.state, ProviderState::Refused);

    let mut schedule = refused.schedule;
    for wait in 2..30 {
        let at = now() + minutes(wait);
        let (again, seen) = run(&path, must_not_be_called, &schedule, at);
        assert!(seen.is_empty());
        schedule = again.schedule;
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), good);
    assert_eq!(folder.files(), [path]);
}

#[test]
fn a_sender_that_could_not_make_the_request_is_a_bad_answer() {
    let folder = TestFolder::new("live_poll-sender-failed");
    let (result, seen) = run(
        &valid_credentials(&folder),
        || Err(NoAnswer::Failed),
        &start(),
        now(),
    );

    assert_eq!(seen.len(), 1);
    assert_eq!(result.state, ProviderState::BadAnswer);
    assert_eq!(result.outcome, PollOutcome::Failed);
    assert_eq!(result.schedule, start().after_failure(now()));
}

// A request under way cannot be stopped. When the caller gave it up meanwhile, its answer is
// dropped: no result, so no reading and no new schedule.
#[test]
fn cancelling_while_the_request_is_under_way_drops_the_answer() {
    let folder = TestFolder::new("live_poll-cancelled-meanwhile");
    let path = valid_credentials(&folder);
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let sender = Arc::new(FakeSender::new(move || {
        flag.store(true, Ordering::SeqCst);
        reply(200, BODY)
    }));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");

    let result = live_poll::run(&path, &client, &start(), now(), &cancelled);

    assert_eq!(result, Err(Cancelled));
    assert_eq!(sender.seen().len(), 1);
}

// A poll with nothing to send asks nobody, so there is nothing to give up: it answers as it
// would have, as in the C#.
#[test]
fn a_cancelled_poll_with_no_sign_in_to_send_still_answers() {
    let folder = TestFolder::new("live_poll-cancelled-missing");
    let client = LiveUsageClient::new(
        Arc::new(FakeSender::new(must_not_be_called)),
        "Sobreiro/0.3.0",
    );
    let result = live_poll::run(
        &credentials_path(&folder),
        &client,
        &start(),
        now(),
        &AtomicBool::new(true),
    );
    assert_eq!(result.unwrap().state, ProviderState::NoSignIn);
}

// The token is handed to the request and to nothing else: the poll's own reader may hand
// over any token, and the result still holds none.
#[test]
fn a_token_handed_in_by_the_reader_goes_to_the_request_only() {
    let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
    let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");
    let result = live_poll::run_with(
        |_| CredentialRead::Ok(AccessToken::new("handed-in".to_string(), None)),
        &client,
        &start(),
        now(),
        &AtomicBool::new(false),
    )
    .unwrap();

    assert_eq!(authorization(&sender.seen()), "Bearer handed-in");
    assert!(!format!("{result:?}").contains("handed-in"));
    assert_eq!(result.state, ProviderState::Ok);
}

#[test]
fn every_outcome_has_the_name_the_csharp_logged() {
    let names: Vec<&str> = [
        PollOutcome::Missing,
        PollOutcome::Unreadable,
        PollOutcome::Expired,
        PollOutcome::Refused,
        PollOutcome::Unauthorized,
        PollOutcome::Ok,
        PollOutcome::RateLimited,
        PollOutcome::Failed,
        PollOutcome::Unreachable,
    ]
    .into_iter()
    .map(PollOutcome::name)
    .collect();
    assert_eq!(
        names,
        [
            "Missing",
            "Unreadable",
            "Expired",
            "Refused",
            "Unauthorized",
            "Ok",
            "RateLimited",
            "Failed",
            "Unreachable"
        ]
    );
}
