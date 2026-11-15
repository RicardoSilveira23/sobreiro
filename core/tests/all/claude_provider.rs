//! Ported from `tests/Sobreiro.Tests/ClaudeProviderTests.cs`. The tests after
//! `cancellation_passes_through` are new.
//!
//! No test here reaches the network, the token is made up, and the sign-in file and
//! `usage.json` are ones the test writes in its own folder.
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::io::http::{Answer, NoAnswer};
use sobreiro_core::limits::LimitKind;
use sobreiro_core::live::claude_provider::ClaudeProvider;
use sobreiro_core::live::live_poll::{LivePollResult, PollOutcome};
use sobreiro_core::live::live_schedule::LiveSchedule;
use sobreiro_core::live::live_usage_client::{Cancelled, LiveUsageClient};
use sobreiro_core::polling::{Provider, ProviderState, Trace};
use sobreiro_core::usage::{RateLimits, UsageSnapshot, usage_file};

use crate::{
    FakeSender, H, M, NO_EXPIRY, NONE, S, ScheduleRow, TestFolder, at_ticks, hold_alone, reply,
    schedule_of, time, window, window_seen,
};

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

/// A session at 27 percent that starts again in two hours.
fn reading() -> RateLimits {
    RateLimits {
        five_hour: Some(window(27.0, Some(now() + hours(2)))),
        seven_day: None,
        spend: None,
    }
}

fn ok() -> LivePollResult {
    LivePollResult {
        schedule: start(),
        state: ProviderState::Ok,
        limits: Some(reading()),
        outcome: PollOutcome::Ok,
    }
}

/// A folder of the test's own, the lines traced so far, and the path of `usage.json` in it.
struct Fixture {
    folder: TestFolder,
    traced: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    fn new(name: &str) -> Fixture {
        let folder = TestFolder::new(&format!("claude_provider-{name}"));
        folder.create();
        Fixture {
            folder,
            traced: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn usage_path(&self) -> PathBuf {
        self.folder.file("usage.json")
    }

    /// Something to hand a provider as its trace: it keeps every line.
    fn trace(&self) -> Option<Trace> {
        let traced = self.traced.clone();
        Some(Arc::new(move |line: &str| {
            traced.lock().unwrap().push(line.to_string());
        }))
    }

    fn traced(&self) -> Vec<String> {
        self.traced.lock().unwrap().clone()
    }

    /// A provider whose poll gives `result` each time, without reading a sign-in or asking
    /// anyone.
    fn provider(&self, result: LivePollResult) -> ClaudeProvider {
        ClaudeProvider::with_poll(
            move |_, _, _| Ok(result.clone()),
            &self.usage_path(),
            self.trace(),
        )
    }
}

fn not_cancelled() -> AtomicBool {
    AtomicBool::new(false)
}

#[test]
fn it_carries_claudes_name() {
    let fixture = Fixture::new("name");
    assert_eq!(fixture.provider(ok()).name(), "Claude Code");
}

#[test]
fn a_poll_passes_on_the_schedule_the_state_and_the_outcome() {
    let fixture = Fixture::new("passes-on");
    let other = start().after_no_reading(now());
    let poll = fixture
        .provider(LivePollResult {
            schedule: other,
            ..ok()
        })
        .poll(&start(), now(), &not_cancelled())
        .unwrap();

    assert_eq!(poll.schedule, other);
    assert_eq!(poll.state, ProviderState::Ok);
    assert_eq!(poll.log, "Live usage Ok");
}

#[test]
fn a_poll_without_a_reading_has_nothing_to_keep() {
    let fixture = Fixture::new("nothing-to-keep");
    let poll = fixture
        .provider(LivePollResult {
            schedule: start(),
            state: ProviderState::Expired,
            limits: None,
            outcome: PollOutcome::Expired,
        })
        .poll(&start(), now(), &not_cancelled())
        .unwrap();

    assert!(poll.keep.is_none());
    assert_eq!(poll.log, "Live usage Expired");
}

#[test]
fn the_reading_reaches_usage_json_only_through_keep() {
    let fixture = Fixture::new("only-through-keep");
    let provider = fixture.provider(ok());
    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();

    assert!(!fixture.usage_path().exists());
    assert!(provider.limits(now()).is_empty());

    (poll.keep.unwrap())(now());

    let limits = provider.limits(now());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Session);
    assert_eq!(limits[0].percent, 27.0);
}

#[test]
fn keep_merges_over_what_the_file_holds() {
    let fixture = Fixture::new("merges");
    let weekly = RateLimits {
        five_hour: None,
        seven_day: Some(window_seen(
            60.0,
            Some(now() + TimeDelta::days(3)),
            now() - minutes(5),
        )),
        spend: None,
    };
    let stored = UsageSnapshot {
        limits: weekly,
        observed_at: now() - minutes(5),
    };
    usage_file::write(&fixture.usage_path(), &stored).unwrap();
    let provider = fixture.provider(ok());

    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    (poll.keep.unwrap())(now());

    let limits = provider.limits(now());
    assert!(
        limits
            .iter()
            .any(|limit| limit.kind == LimitKind::Weekly && limit.percent == 60.0)
    );
    assert!(
        limits
            .iter()
            .any(|limit| limit.kind == LimitKind::Session && limit.percent == 27.0)
    );
}

#[test]
fn a_reading_replaces_a_higher_stored_one_in_the_same_period() {
    let fixture = Fixture::new("replaces-higher");
    let higher = RateLimits {
        five_hour: Some(window_seen(
            40.0,
            Some(now() + hours(2)),
            now() - minutes(5),
        )),
        seven_day: None,
        spend: None,
    };
    let stored = UsageSnapshot {
        limits: higher,
        observed_at: now() - minutes(5),
    };
    usage_file::write(&fixture.usage_path(), &stored).unwrap();
    let provider = fixture.provider(ok());

    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    (poll.keep.unwrap())(now());

    let limits = provider.limits(now());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].percent, 27.0);
}

#[test]
fn a_held_usage_json_skips_the_reading() {
    let fixture = Fixture::new("held");
    std::fs::write(fixture.usage_path(), "{}").unwrap();
    let before = std::fs::read(fixture.usage_path()).unwrap();
    let poll = fixture
        .provider(ok())
        .poll(&start(), now(), &not_cancelled())
        .unwrap();

    {
        let _held = hold_alone(&fixture.usage_path());
        (poll.keep.unwrap())(now());
    }

    assert_eq!(
        fixture.traced(),
        ["usage.json was busy, so this Live usage reading was not saved"]
    );
    assert_eq!(std::fs::read(fixture.usage_path()).unwrap(), before);
}

#[test]
fn limits_keep_the_last_snapshot_when_a_read_fails() {
    let fixture = Fixture::new("last-snapshot");
    let provider = fixture.provider(ok());
    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    (poll.keep.unwrap())(now());
    assert_eq!(provider.limits(now()).len(), 1);

    let _held = hold_alone(&fixture.usage_path());

    let limits = provider.limits(now());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Session);
}

/// A sign-in file with the made-up token, which runs out at `expires_at`.
fn sign_in(expires_at: DateTime<Utc>) -> String {
    format!(
        "{{\"claudeAiOauth\":{{\"accessToken\":\"{SECRET}\",\"refreshToken\":\"r\",\"expiresAt\":{}}}}}",
        expires_at.timestamp_millis()
    )
}

#[test]
fn the_public_constructor_reads_the_sign_in_and_asks() {
    let fixture = Fixture::new("public");
    let credentials = fixture.folder.file(".credentials.json");
    std::fs::write(&credentials, sign_in(now() + hours(2))).unwrap();
    let sender = Arc::new(FakeSender::new(|| reply(200, BODY)));
    let provider = ClaudeProvider::new(
        LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0"),
        &credentials,
        &fixture.usage_path(),
        fixture.trace(),
    );

    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    let log = poll.log.clone();
    (poll.keep.unwrap())(now());

    let limits = provider.limits(now());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Session);
    assert_eq!(limits[0].percent, 27.0);
    assert_eq!(sender.seen().len(), 1);
    assert!(!log.contains(SECRET));
    assert!(fixture.traced().iter().all(|line| !line.contains(SECRET)));
}

#[test]
fn cancellation_passes_through() {
    let fixture = Fixture::new("cancelled");
    let provider = ClaudeProvider::with_poll(
        |_, _, cancelled| {
            if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Cancelled);
            }
            Ok(ok())
        },
        &fixture.usage_path(),
        fixture.trace(),
    );

    let poll = provider.poll(&start(), now(), &AtomicBool::new(true));

    assert!(matches!(poll, Err(Cancelled)));
}

// ---- New --------------------------------------------------------------------------------------

// A reading is held by the time `keep` is called with, which is when the answer is handled,
// not when the poll began.
#[test]
fn a_kept_reading_carries_the_time_it_was_kept_at() {
    let fixture = Fixture::new("kept-at");
    let provider = fixture.provider(ok());
    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    let kept_at = now() + TimeDelta::seconds(3);
    (poll.keep.unwrap())(kept_at);

    let stored = usage_file::read(&fixture.usage_path()).unwrap();
    assert_eq!(stored.observed_at, kept_at);
    assert_eq!(stored.limits.five_hour.unwrap().observed_at, Some(kept_at));
}

// A `usage.json` that cannot be written is traced by the kind of the failure and never by
// its words, which would name a folder of this PC. Here a file stands where the folder of
// `usage.json` should be.
#[test]
fn a_usage_json_that_cannot_be_written_is_traced_without_its_path() {
    let fixture = Fixture::new("cannot-write");
    let in_the_way = fixture.folder.file("in-the-way");
    std::fs::write(&in_the_way, "a file, not a folder").unwrap();
    let provider = ClaudeProvider::with_poll(
        |_, _, _| Ok(ok()),
        &in_the_way.join("usage.json"),
        fixture.trace(),
    );

    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    (poll.keep.unwrap())(now());

    let traced = fixture.traced();
    assert_eq!(traced.len(), 1, "{traced:?}");
    // After the fixed words comes one word, the kind of the failure: no sentence of the
    // system's, and no path.
    let kind = traced[0]
        .strip_prefix("saving the Live usage reading failed: ")
        .unwrap_or_else(|| panic!("{traced:?}"));
    assert!(
        !kind.is_empty() && kind.chars().all(|letter| letter.is_ascii_alphanumeric()),
        "{traced:?}"
    );
    assert!(!traced[0].contains("in-the-way"), "{traced:?}");
    assert!(provider.limits(now()).is_empty());
}

// A provider without a trace says nothing and still does not stop on a file that is held.
#[test]
fn a_provider_without_a_trace_skips_a_held_usage_json_in_silence() {
    let fixture = Fixture::new("no-trace");
    std::fs::write(fixture.usage_path(), "{}").unwrap();
    let provider = ClaudeProvider::with_poll(|_, _, _| Ok(ok()), &fixture.usage_path(), None);
    let poll = provider.poll(&start(), now(), &not_cancelled()).unwrap();
    {
        let _held = hold_alone(&fixture.usage_path());
        (poll.keep.unwrap())(now());
    }
    assert_eq!(std::fs::read(fixture.usage_path()).unwrap(), b"{}");
}

// The poll is handed the schedule, the time and the flag the provider was handed.
#[test]
fn the_poll_is_handed_what_the_provider_was_handed() {
    let fixture = Fixture::new("handed");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_by_poll = seen.clone();
    let provider = ClaudeProvider::with_poll(
        move |schedule, at, cancelled| {
            let cancelled = cancelled.load(std::sync::atomic::Ordering::SeqCst);
            seen_by_poll
                .lock()
                .unwrap()
                .push((*schedule, at, cancelled));
            Ok(ok())
        },
        &fixture.usage_path(),
        None,
    );
    let schedule = start().after_no_credentials(now());

    provider
        .poll(&schedule, now() + minutes(1), &not_cancelled())
        .unwrap();

    assert_eq!(
        *seen.lock().unwrap(),
        [(schedule, now() + minutes(1), false)]
    );
}

// ---- Rows the C# wrote ---------------------------------------------------------------------
//
// Everything below was written by a script from the answers of the C# of 1.1.0, which was run
// on these made-up cases on 2026-11-14. Nothing below was typed. A moment is a count of ticks
// from noon on 2026-11-09 (UTC), written with the units of `main.rs`: `5 * M + 1` is five
// minutes and one tick after it. `NONE` stands where there is no moment.

/// The sign-in files of the rows, by name. The token in them is made up.
#[rustfmt::skip]
const SIGN_INS: &[(&str, Option<&str>)] = &[
    ("missing", None),
    ("not json", Some("not json")),
    ("no token", Some("{\"claudeAiOauth\":{}}")),
    ("an empty token", Some("{\"claudeAiOauth\":{\"accessToken\":\"\"}}")),
    ("ran out a minute ago", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\",\"expiresAt\":1794225540000}}")),
    ("runs out now", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\",\"expiresAt\":1794225600000}}")),
    ("runs out in a millisecond", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\",\"expiresAt\":1794225600001}}")),
    ("good for two hours", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\",\"expiresAt\":1794232800000}}")),
    ("good for ten hours", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\",\"expiresAt\":1794261600000}}")),
    ("no expiry", Some("{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-SECRETVALUE\",\"refreshToken\":\"r\"}}")),
];

/// The replies of the rows: a name, the status, `Retry-After`, and the body. A status below
/// zero is no reply at all: no way to the host, or the ten seconds ran out.
#[rustfmt::skip]
const REPLIES: &[(&str, i32, Option<&str>, &str)] = &[
    ("200", 200, None, "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}"),
    ("401", 401, None, "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}"),
    ("429 after 600", 429, Some("600"), "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}"),
    ("no route", -1, None, ""),
    ("200 unexpected", 200, None, "{\"unexpected\":true}"),
    ("200 empty", 200, None, ""),
    ("204", 204, None, ""),
    ("302", 302, None, ""),
    ("400", 400, None, "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}"),
    ("403", 403, Some("600"), ""),
    ("404", 404, None, ""),
    ("429", 429, None, ""),
    ("429 after 30", 429, Some("30"), ""),
    ("429 after a day", 429, Some("86400"), ""),
    ("500", 500, None, ""),
    ("timed out", -2, None, ""),
];

/// A sender that gives the reply of that name each time it is asked.
fn sender_replying(name: &str) -> Arc<FakeSender> {
    let &(_, status, retry_after, body) = REPLIES.iter().find(|reply| reply.0 == name).unwrap();
    let retry_after = retry_after.map(str::to_string);
    Arc::new(FakeSender::new(move || -> Result<Answer, NoAnswer> {
        let Ok(status) = u16::try_from(status) else {
            return Err(NoAnswer::Unreachable);
        };
        Ok(Answer {
            status,
            retry_after: retry_after.clone(),
            body: body.as_bytes().to_vec(),
        })
    }))
}

#[test]
fn a_poll_through_the_provider_answers_as_the_csharp_did() {
    use ProviderState as P;
    // A sign-in file, the schedule before, and the reply; then how many requests were sent,
    // the state, the line for the log, the schedule after, and whether there is a reading to
    // keep. Every poll is at noon. Where the sign-in gives nothing to send, the reply cannot
    // matter and one is enough.
    type Row = (
        &'static str,
        ScheduleRow,
        &'static str,
        usize,
        P,
        &'static str,
        ScheduleRow,
        bool,
    );
    #[rustfmt::skip]
    let rows: &[Row] = &[
        ("missing", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::NoSignIn, "Live usage Missing", (M, NONE, 0, NONE, NONE, 1), false),
        ("missing", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::NoSignIn, "Live usage Missing", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("missing", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::NoSignIn, "Live usage Missing", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("missing", (0, NONE, 2, -M, NONE, 2), "200", 0, P::NoSignIn, "Live usage Missing", (M, NONE, 2, NONE, NONE, 3), false),
        ("missing", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::NoSignIn, "Live usage Missing", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("not json", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NONE, 1), false),
        ("not json", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("not json", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("not json", (0, NONE, 2, -M, NONE, 2), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 2, NONE, NONE, 3), false),
        ("not json", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("no token", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NONE, 1), false),
        ("no token", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("no token", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no token", (0, NONE, 2, -M, NONE, 2), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 2, NONE, NONE, 3), false),
        ("no token", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("an empty token", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NONE, 1), false),
        ("an empty token", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("an empty token", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("an empty token", (0, NONE, 2, -M, NONE, 2), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, NONE, 2, NONE, NONE, 3), false),
        ("an empty token", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::NoSignIn, "Live usage Unreadable", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("ran out a minute ago", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, NONE, 1), false),
        ("ran out a minute ago", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("ran out a minute ago", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("ran out a minute ago", (0, NONE, 2, -M, NONE, 2), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 2, NONE, NONE, 3), false),
        ("ran out a minute ago", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::Expired, "Live usage Expired", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("runs out now", (0, NONE, 0, NONE, NONE, 0), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, NONE, 1), false),
        ("runs out now", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("runs out now", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("runs out now", (0, NONE, 2, -M, NONE, 2), "200", 0, P::Expired, "Live usage Expired", (M, NONE, 2, NONE, NONE, 3), false),
        ("runs out now", (0, -5 * M, 0, 0, NONE, 1), "200", 0, P::Expired, "Live usage Expired", (M, -5 * M, 0, NONE, NONE, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NONE, 0), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NONE, 0), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10_000, 1), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NONE, 0), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NONE, 0), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, 2 * H, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("runs out in a millisecond", (0, NONE, 0, NONE, 2 * H, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10_000, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, 2 * H, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, 2 * H, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NO_EXPIRY, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10_000, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NO_EXPIRY, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("runs out in a millisecond", (0, NONE, 0, NONE, NO_EXPIRY, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("runs out in a millisecond", (0, NONE, 2, -M, NONE, 2), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("runs out in a millisecond", (0, NONE, 2, -M, NONE, 2), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 2, NONE, 10_000, 3), false),
        ("runs out in a millisecond", (0, NONE, 2, -M, NONE, 2), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("runs out in a millisecond", (0, NONE, 2, -M, NONE, 2), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("runs out in a millisecond", (0, -5 * M, 0, 0, NONE, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("runs out in a millisecond", (0, -5 * M, 0, 0, NONE, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, -5 * M, 0, NONE, 10_000, 2), false),
        ("runs out in a millisecond", (0, -5 * M, 0, 0, NONE, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, -5 * M, 1, 10 * M, NONE, 2), false),
        ("runs out in a millisecond", (0, -5 * M, 0, 0, NONE, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "200 unexpected", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "200 empty", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "204", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "302", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "400", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 2 * H, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "403", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 2 * H, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "404", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "429", 1, P::RateLimited, "Live usage RateLimited", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "429 after 30", 1, P::RateLimited, "Live usage RateLimited", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "429 after a day", 1, P::RateLimited, "Live usage RateLimited", (6 * H, NONE, 1, 6 * H, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "500", 1, P::BadAnswer, "Live usage Failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, NONE, 0), "timed out", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for two hours", (0, NONE, 0, NONE, 2 * H, 1), "200", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, 2 * H, 1), "401", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, 2 * H, 1), "429 after 600", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, 2 * H, 1), "no route", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for two hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("good for two hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("good for two hours", (0, NONE, 2, -M, NONE, 2), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for two hours", (0, NONE, 2, -M, NONE, 2), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 2, NONE, 2 * H, 3), false),
        ("good for two hours", (0, NONE, 2, -M, NONE, 2), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("good for two hours", (0, NONE, 2, -M, NONE, 2), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("good for two hours", (0, -5 * M, 0, 0, NONE, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for two hours", (0, -5 * M, 0, 0, NONE, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, -5 * M, 0, NONE, 2 * H, 2), false),
        ("good for two hours", (0, -5 * M, 0, 0, NONE, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, -5 * M, 1, 10 * M, NONE, 2), false),
        ("good for two hours", (0, -5 * M, 0, 0, NONE, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, NONE, 0), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for ten hours", (0, NONE, 0, NONE, NONE, 0), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10 * H, 1), false),
        ("good for ten hours", (0, NONE, 0, NONE, NONE, 0), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("good for ten hours", (0, NONE, 0, NONE, NONE, 0), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("good for ten hours", (0, NONE, 0, NONE, 2 * H, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for ten hours", (0, NONE, 0, NONE, 2 * H, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10 * H, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, 2 * H, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, 2 * H, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for ten hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, 10 * H, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("good for ten hours", (0, NONE, 0, NONE, NO_EXPIRY, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("good for ten hours", (0, NONE, 2, -M, NONE, 2), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for ten hours", (0, NONE, 2, -M, NONE, 2), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 2, NONE, 10 * H, 3), false),
        ("good for ten hours", (0, NONE, 2, -M, NONE, 2), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("good for ten hours", (0, NONE, 2, -M, NONE, 2), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("good for ten hours", (0, -5 * M, 0, 0, NONE, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("good for ten hours", (0, -5 * M, 0, 0, NONE, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, -5 * M, 0, NONE, 10 * H, 2), false),
        ("good for ten hours", (0, -5 * M, 0, 0, NONE, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, -5 * M, 1, 10 * M, NONE, 2), false),
        ("good for ten hours", (0, -5 * M, 0, 0, NONE, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("no expiry", (0, NONE, 0, NONE, NONE, 0), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("no expiry", (0, NONE, 0, NONE, NONE, 0), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, NO_EXPIRY, 1), false),
        ("no expiry", (0, NONE, 0, NONE, NONE, 0), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("no expiry", (0, NONE, 0, NONE, NONE, 0), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("no expiry", (0, NONE, 0, NONE, 2 * H, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("no expiry", (0, NONE, 0, NONE, 2 * H, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, NONE, 0, NONE, 2 * H, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, NONE, 1, 10 * M, NONE, 2), false),
        ("no expiry", (0, NONE, 0, NONE, 2 * H, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 0, 2 * M, NONE, 2), false),
        ("no expiry", (0, NONE, 0, NONE, NO_EXPIRY, 1), "200", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, NONE, 0, NONE, NO_EXPIRY, 1), "401", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, NONE, 0, NONE, NO_EXPIRY, 1), "429 after 600", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, NONE, 0, NONE, NO_EXPIRY, 1), "no route", 0, P::Refused, "Live usage Refused", (M, NONE, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, NONE, 2, -M, NONE, 2), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("no expiry", (0, NONE, 2, -M, NONE, 2), "401", 1, P::Refused, "Live usage Unauthorized", (M, NONE, 2, NONE, NO_EXPIRY, 3), false),
        ("no expiry", (0, NONE, 2, -M, NONE, 2), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("no expiry", (0, NONE, 2, -M, NONE, 2), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("no expiry", (0, -5 * M, 0, 0, NONE, 1), "200", 1, P::Ok, "Live usage Ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("no expiry", (0, -5 * M, 0, 0, NONE, 1), "401", 1, P::Refused, "Live usage Unauthorized", (M, -5 * M, 0, NONE, NO_EXPIRY, 2), false),
        ("no expiry", (0, -5 * M, 0, 0, NONE, 1), "429 after 600", 1, P::RateLimited, "Live usage RateLimited", (10 * M, -5 * M, 1, 10 * M, NONE, 2), false),
        ("no expiry", (0, -5 * M, 0, 0, NONE, 1), "no route", 1, P::Offline, "Live usage Unreachable", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
    ];
    assert_eq!(rows.len(), 122);
    for (index, &(sign_in, before, reply_name, sent, state, log, after, keeps)) in
        rows.iter().enumerate()
    {
        let row = format!("row {index}: {sign_in}, {reply_name}");
        let fixture = Fixture::new(&format!("row-{index}"));
        let credentials = fixture.folder.file(".credentials.json");
        let &(_, text) = SIGN_INS.iter().find(|file| file.0 == sign_in).unwrap();
        if let Some(text) = text {
            std::fs::write(&credentials, text).unwrap();
        }
        let sender = sender_replying(reply_name);
        let client = LiveUsageClient::new(sender.clone(), "Sobreiro/0.3.0");
        let provider =
            ClaudeProvider::new(client, &credentials, &fixture.usage_path(), fixture.trace());

        let poll = provider
            .poll(&schedule_of(before), at_ticks(0), &not_cancelled())
            .unwrap();

        let seen = sender.seen();
        assert_eq!(seen.len(), sent, "{row}");
        for request in &seen {
            assert_eq!(
                request.url, "https://api.anthropic.com/api/oauth/usage",
                "{row}"
            );
            assert!(
                request
                    .headers
                    .contains(&("Authorization".to_string(), format!("Bearer {SECRET}"))),
                "{row}"
            );
        }
        assert_eq!(poll.state, state, "{row}");
        assert_eq!(poll.log, log, "{row}");
        assert_eq!(poll.schedule, schedule_of(after), "{row}");
        assert_eq!(poll.keep.is_some(), keeps, "{row}");
        if let Some(keep) = poll.keep {
            keep(at_ticks(3 * S));
            assert_eq!(
                usage_file::read(&fixture.usage_path()).unwrap().observed_at,
                at_ticks(3 * S),
                "{row}"
            );
        }
        assert!(fixture.traced().is_empty(), "{row}");
    }
}

#[test]
fn keep_leaves_usage_json_as_the_csharp_left_it() {
    // What `usage.json` held, and the reply; then the file after the reading was kept, three
    // seconds after noon, and the limits the provider gave before the poll and after.
    type Shown = &'static [(LimitKind, f64)];
    type Row = (
        &'static str,
        Option<&'static str>,
        &'static str,
        &'static str,
        &'static str,
        Shown,
        Shown,
    );
    #[rustfmt::skip]
    let rows: &[Row] = &[
        ("missing", None, "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.0)]),
        ("missing", None, "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("missing", None, "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Weekly, 3.0)]),
        ("an empty object", Some("{}"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.0)]),
        ("an empty object", Some("{}"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("an empty object", Some("{}"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Weekly, 3.0)]),
        ("not json", Some("not json"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.0)]),
        ("not json", Some("not json"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("not json", Some("not json"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[], &[(LimitKind::Weekly, 3.0)]),
        ("a higher session in the same period", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":40,\"resets_at\":1794232800,\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 40.0)], &[(LimitKind::Session, 27.0)]),
        ("a higher session in the same period", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":40,\"resets_at\":1794232800,\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 40.0)], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("a higher session in the same period", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":40,\"resets_at\":1794232800,\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":40,\"resets_at\":1794232800,\"observed_at\":1794225300},\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 40.0)], &[(LimitKind::Session, 40.0), (LimitKind::Weekly, 3.0)]),
        ("a session of a period that is over", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":90,\"resets_at\":1794222000,\"observed_at\":1794207600}},\"observed_at\":1794207600}"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 0.0)], &[(LimitKind::Session, 27.0)]),
        ("a session of a period that is over", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":90,\"resets_at\":1794222000,\"observed_at\":1794207600}},\"observed_at\":1794207600}"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 0.0)], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("a session of a period that is over", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":90,\"resets_at\":1794222000,\"observed_at\":1794207600}},\"observed_at\":1794207600}"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 0.0)], &[(LimitKind::Weekly, 3.0)]),
        ("a weekly and a spend", Some("{\"rate_limits\":{\"seven_day\":{\"used_percentage\":60,\"resets_at\":1794484800,\"observed_at\":1794225300},\"spend\":{\"used_percentage\":12.5,\"resets_at\":1796083200,\"used_usd\":2.5,\"limit_usd\":20,\"period\":\"monthly\",\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":60,\"resets_at\":1794484800,\"observed_at\":1794225300}}}", &[(LimitKind::Weekly, 60.0)], &[(LimitKind::Session, 27.0), (LimitKind::Weekly, 60.0)]),
        ("a weekly and a spend", Some("{\"rate_limits\":{\"seven_day\":{\"used_percentage\":60,\"resets_at\":1794484800,\"observed_at\":1794225300},\"spend\":{\"used_percentage\":12.5,\"resets_at\":1796083200,\"used_usd\":2.5,\"limit_usd\":20,\"period\":\"monthly\",\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[(LimitKind::Weekly, 60.0)], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("a weekly and a spend", Some("{\"rate_limits\":{\"seven_day\":{\"used_percentage\":60,\"resets_at\":1794484800,\"observed_at\":1794225300},\"spend\":{\"used_percentage\":12.5,\"resets_at\":1796083200,\"used_usd\":2.5,\"limit_usd\":20,\"period\":\"monthly\",\"observed_at\":1794225300}},\"observed_at\":1794225300}"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[(LimitKind::Weekly, 60.0)], &[(LimitKind::Weekly, 3.0)]),
        ("a key Sobreiro does not know", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":10}},\"observed_at\":1794225300,\"other\":true}"), "a session", "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27,\"resets_at\":1794232800,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 10.0)], &[(LimitKind::Session, 27.0)]),
        ("a key Sobreiro does not know", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":10}},\"observed_at\":1794225300,\"other\":true}"), "a session and a weekly", "{\"limits\":[{\"group\":\"session\",\"percent\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":27.5,\"resets_at\":1794232800,\"observed_at\":1794225603},\"seven_day\":{\"used_percentage\":74,\"resets_at\":1794528000,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 10.0)], &[(LimitKind::Session, 27.5), (LimitKind::Weekly, 74.0)]),
        ("a key Sobreiro does not know", Some("{\"rate_limits\":{\"five_hour\":{\"used_percentage\":10}},\"observed_at\":1794225300,\"other\":true}"), "a weekly without a reset", "{\"limits\":[{\"group\":\"weekly\",\"percent\":3}]}", "{\"observed_at\":1794225603,\"rate_limits\":{\"five_hour\":{\"used_percentage\":10,\"observed_at\":1794225300},\"seven_day\":{\"used_percentage\":3,\"observed_at\":1794225603}}}", &[(LimitKind::Session, 10.0)], &[(LimitKind::Session, 10.0), (LimitKind::Weekly, 3.0)]),
    ];
    assert_eq!(rows.len(), 21);
    for (index, &(stored_name, stored, body_name, body, file_after, before, after)) in
        rows.iter().enumerate()
    {
        let row = format!("row {index}: {stored_name}, {body_name}");
        let fixture = Fixture::new(&format!("keep-row-{index}"));
        let credentials = fixture.folder.file(".credentials.json");
        std::fs::write(&credentials, sign_in(at_ticks(2 * H))).unwrap();
        if let Some(stored) = stored {
            std::fs::write(fixture.usage_path(), stored).unwrap();
        }
        let body = body.to_string();
        let sender = Arc::new(FakeSender::new(move || reply(200, &body)));
        let client = LiveUsageClient::new(sender, "Sobreiro/0.3.0");
        let provider =
            ClaudeProvider::new(client, &credentials, &fixture.usage_path(), fixture.trace());
        let shown = |at| -> Vec<(LimitKind, f64)> {
            provider
                .limits(at)
                .iter()
                .map(|limit| (limit.kind, limit.percent))
                .collect()
        };

        assert_eq!(shown(at_ticks(0)), before, "{row}: before");
        let poll = provider
            .poll(
                &LiveSchedule::start(at_ticks(0)),
                at_ticks(0),
                &not_cancelled(),
            )
            .unwrap();
        (poll.keep.unwrap())(at_ticks(3 * S));

        assert_eq!(
            std::fs::read_to_string(fixture.usage_path()).unwrap(),
            file_after,
            "{row}"
        );
        assert_eq!(shown(at_ticks(3 * S)), after, "{row}: after");
        assert!(fixture.traced().is_empty(), "{row}");
    }
}
