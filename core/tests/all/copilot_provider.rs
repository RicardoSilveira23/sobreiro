//! Ported from `tests/Sobreiro.Tests/CopilotProviderTests.cs`. The tests after
//! `cancellation_passes_through` are new.
//!
//! No test here reaches the network, every token is made up, and the cache file is one the
//! test writes in its own folder. The two C# tests where the poll or the cache read threw an
//! exception have no Rust test: a poll here gives an answer or `Cancelled`, and a cache read
//! gives a reading or none, so there is nothing to catch.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::copilot::copilot_poll::CopilotPollResult;
use sobreiro_core::copilot::copilot_provider::CopilotProvider;
use sobreiro_core::copilot::copilot_sign_in::{CopilotSignIn, CopilotSourceKind, TokenSearch};
use sobreiro_core::copilot::copilot_usage_client::CopilotUsageClient;
use sobreiro_core::copilot::{CopilotReading, CopilotSource, GitHubToken};
use sobreiro_core::limits::LimitKind;
use sobreiro_core::live::live_schedule::LiveSchedule;
use sobreiro_core::live::live_usage_client::Cancelled;
use sobreiro_core::polling::{Provider, ProviderPoll, ProviderState, Trace};

use crate::{FakeSender, TestFolder, reply, time};

const SECRET: &str = "ghp_SECRETVALUE";

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

const OK_LOG: &str = "copilot Ok env=ok credential-manager=skipped gh=skipped";
const OFFLINE_LOG: &str = "copilot Offline env=unreachable credential-manager=skipped gh=skipped";

fn now() -> DateTime<Utc> {
    time("2026-11-10T12:00:00Z")
}

fn start() -> LiveSchedule {
    LiveSchedule::start(now())
}

fn reading_from(at: DateTime<Utc>, source: CopilotSource) -> CopilotReading {
    CopilotReading {
        budget: 15000.0,
        used: 2645.0,
        resets_at: Some(now() + TimeDelta::days(18)),
        observed_at: at,
        source,
    }
}

fn live(at: DateTime<Utc>) -> CopilotReading {
    reading_from(at, CopilotSource::Live)
}

fn cache(at: DateTime<Utc>) -> CopilotReading {
    reading_from(at, CopilotSource::Cache)
}

fn minutes(count: i64) -> TimeDelta {
    TimeDelta::minutes(count)
}

fn poll_result(
    state: ProviderState,
    reading: Option<CopilotReading>,
    log: &str,
) -> CopilotPollResult {
    CopilotPollResult {
        schedule: start(),
        state,
        reading,
        log: log.to_string(),
    }
}

fn trace_into(lines: &Arc<Mutex<Vec<String>>>) -> Trace {
    let lines = lines.clone();
    Arc::new(move |line: &str| lines.lock().unwrap().push(line.to_string()))
}

/// A provider whose poll and cache read give what the test puts in `poll` and `cache`, and
/// the lines it was told.
struct Rig {
    poll: Arc<Mutex<CopilotPollResult>>,
    cache: Arc<Mutex<Option<CopilotReading>>>,
    logged: Arc<Mutex<Vec<String>>>,
    provider: CopilotProvider,
}

impl Rig {
    fn new() -> Rig {
        let poll = Arc::new(Mutex::new(poll_result(
            ProviderState::Ok,
            Some(live(now())),
            OK_LOG,
        )));
        let cache = Arc::new(Mutex::new(None));
        let logged = Arc::new(Mutex::new(Vec::new()));
        let (poll_of, cache_of) = (poll.clone(), cache.clone());
        let provider = CopilotProvider::with_poll(
            move |_, _, _| Ok(poll_of.lock().unwrap().clone()),
            move || cache_of.lock().unwrap().clone(),
            trace_into(&logged),
        );
        Rig {
            poll,
            cache,
            logged,
            provider,
        }
    }

    fn returns(&self, result: CopilotPollResult, cache: Option<CopilotReading>) {
        *self.poll.lock().unwrap() = result;
        *self.cache.lock().unwrap() = cache;
    }

    fn logged(&self) -> Vec<String> {
        self.logged.lock().unwrap().clone()
    }

    fn poll(&self) -> ProviderPoll {
        self.provider
            .poll(&start(), now(), &AtomicBool::new(false))
            .unwrap()
    }

    /// A poll, kept.
    fn kept(&self) -> ProviderPoll {
        let mut poll = self.poll();
        (poll.keep.take().unwrap())(now());
        poll
    }
}

#[test]
fn it_carries_copilots_name() {
    assert_eq!(Rig::new().provider.name(), "GitHub Copilot");
}

#[test]
fn nothing_is_shown_before_a_poll() {
    let rig = Rig::new();
    assert!(rig.provider.shown().is_none());
    assert!(rig.provider.limits(now()).is_empty());
}

#[test]
fn a_poll_passes_on_the_schedule_the_state_and_the_line() {
    let rig = Rig::new();
    let schedule = start().after_no_reading(now());
    rig.returns(
        CopilotPollResult {
            schedule,
            ..poll_result(ProviderState::Ok, Some(live(now())), OK_LOG)
        },
        None,
    );
    let poll = rig.poll();
    assert_eq!(poll.schedule, schedule);
    assert_eq!(poll.state, ProviderState::Ok);
    assert_eq!(
        poll.log,
        "copilot Ok env=ok credential-manager=skipped gh=skipped"
    );
}

#[test]
fn the_reading_is_shown_only_after_keep() {
    let rig = Rig::new();
    let reading = live(now());
    rig.returns(
        poll_result(ProviderState::Ok, Some(reading.clone()), OK_LOG),
        None,
    );
    let poll = rig.poll();
    assert!(rig.provider.shown().is_none());
    (poll.keep.unwrap())(now());
    assert_eq!(rig.provider.shown(), Some(reading));
    let limits = rig.provider.limits(now());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Credits);
}

#[test]
fn the_later_of_live_and_cache_is_shown() {
    let rig = Rig::new();
    rig.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        Some(cache(now() + minutes(5))),
    );
    rig.kept();
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);

    let older = Rig::new();
    older.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        Some(cache(now() - minutes(5))),
    );
    older.kept();
    assert_eq!(older.provider.shown().unwrap().source, CopilotSource::Live);
}

#[test]
fn the_line_says_how_the_cache_was_used() {
    let none = Rig::new();
    none.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        None,
    );
    none.kept();
    assert_eq!(none.logged(), [format!("{OK_LOG} cache=none")]);

    let older = Rig::new();
    older.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        Some(cache(now() - minutes(5))),
    );
    older.kept();
    assert_eq!(older.logged(), [format!("{OK_LOG} cache=older")]);

    let used = Rig::new();
    used.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        Some(cache(now() + minutes(5))),
    );
    used.kept();
    assert_eq!(used.logged(), [format!("{OK_LOG} cache=used")]);
}

#[test]
fn the_line_is_logged_only_when_it_changes() {
    let rig = Rig::new();
    rig.returns(
        poll_result(ProviderState::Ok, Some(live(now())), OK_LOG),
        None,
    );
    rig.kept();
    rig.kept();
    assert_eq!(rig.logged().len(), 1);

    rig.returns(
        poll_result(
            ProviderState::NoSignIn,
            None,
            "copilot NoSignIn env=none credential-manager=none gh=none",
        ),
        None,
    );
    rig.kept();
    assert_eq!(rig.logged().len(), 2);
}

#[test]
fn a_poll_that_is_not_kept_logs_nothing() {
    let rig = Rig::new();
    rig.poll();
    assert!(rig.logged().is_empty());
}

#[test]
fn a_failed_poll_keeps_the_live_reading() {
    let rig = Rig::new();
    let reading = live(now());
    rig.returns(
        poll_result(ProviderState::Ok, Some(reading.clone()), OK_LOG),
        None,
    );
    rig.kept();
    rig.returns(poll_result(ProviderState::Offline, None, OFFLINE_LOG), None);
    rig.kept();
    assert_eq!(rig.provider.shown(), Some(reading));
}

#[test]
fn a_cache_that_cannot_be_read_keeps_the_one_before() {
    let rig = Rig::new();
    let held = cache(now());
    rig.returns(
        poll_result(ProviderState::Offline, None, OFFLINE_LOG),
        Some(held.clone()),
    );
    rig.kept();
    *rig.cache.lock().unwrap() = None;
    rig.kept();
    assert_eq!(rig.provider.shown(), Some(held));
}

#[test]
fn the_early_cache_is_taken_only_when_none_is_held() {
    let rig = Rig::new();
    rig.provider.take_early(cache(now()));
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);

    let held = Rig::new();
    let newer = cache(now() + minutes(5));
    held.returns(
        poll_result(ProviderState::Offline, None, OFFLINE_LOG),
        Some(newer.clone()),
    );
    held.kept();
    held.provider.take_early(cache(now() - minutes(60)));
    assert_eq!(held.provider.shown(), Some(newer));
}

#[test]
fn cancellation_passes_through() {
    let provider = CopilotProvider::with_poll(
        |_, _, cancelled| {
            if cancelled.load(Ordering::SeqCst) {
                return Err(Cancelled);
            }
            Ok(poll_result(ProviderState::Ok, Some(live(now())), OK_LOG))
        },
        || None,
        trace_into(&Arc::new(Mutex::new(Vec::new()))),
    );

    let poll = provider.poll(&start(), now(), &AtomicBool::new(true));

    assert!(matches!(poll, Err(Cancelled)));
}

/// A place that gives the made-up token.
fn env_with_secret() -> CopilotSignIn {
    CopilotSignIn {
        kind: CopilotSourceKind::Env,
        tokens: Box::new(|| {
            Ok(TokenSearch {
                tokens: vec![GitHubToken::new(SECRET.to_string())],
                entry_found: false,
            })
        }),
    }
}

#[test]
fn the_public_constructor_asks_github_and_reads_the_cache_file() {
    let folder = TestFolder::new("copilot_provider-public");
    let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
    let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
    let logged = Arc::new(Mutex::new(Vec::new()));
    let provider = CopilotProvider::new(
        client,
        vec![env_with_secret()],
        &folder.file("cache.json"),
        trace_into(&logged),
    );

    let poll = provider
        .poll(&start(), now(), &AtomicBool::new(false))
        .unwrap();
    let log = poll.log.clone();
    (poll.keep.unwrap())(now());

    assert_eq!(provider.shown().unwrap().source, CopilotSource::Live);
    assert_eq!(sender.seen().len(), 1);
    assert!(!log.contains(SECRET));
    assert!(
        logged
            .lock()
            .unwrap()
            .iter()
            .all(|line| !line.contains(SECRET))
    );
    assert!(provider.read_cache().is_none());
}

// ---- New --------------------------------------------------------------------------------------

// A poll that brought nothing still has something to keep: the cache that was read, and the
// line for the log.
#[test]
fn a_poll_without_a_reading_still_has_something_to_keep() {
    let rig = Rig::new();
    rig.returns(
        poll_result(ProviderState::Offline, None, OFFLINE_LOG),
        Some(cache(now())),
    );
    let poll = rig.poll();
    assert!(poll.keep.is_some());
    (poll.keep.unwrap())(now());
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);
    assert_eq!(rig.logged(), [format!("{OFFLINE_LOG} cache=used")]);
}

// A cancelled poll leaves the cache unread.
#[test]
fn a_cancelled_poll_reads_no_cache() {
    let read = Arc::new(AtomicBool::new(false));
    let read_by_provider = read.clone();
    let provider = CopilotProvider::with_poll(
        |_, _, _| Err(Cancelled),
        move || {
            read_by_provider.store(true, Ordering::SeqCst);
            None
        },
        trace_into(&Arc::new(Mutex::new(Vec::new()))),
    );
    assert!(
        provider
            .poll(&start(), now(), &AtomicBool::new(false))
            .is_err()
    );
    assert!(!read.load(Ordering::SeqCst));
}

// The poll is handed the schedule, the time and the flag the provider was handed.
#[test]
fn the_poll_is_handed_what_the_provider_was_handed() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_by_poll = seen.clone();
    let provider = CopilotProvider::with_poll(
        move |schedule, at, cancelled| {
            seen_by_poll
                .lock()
                .unwrap()
                .push((*schedule, at, cancelled.load(Ordering::SeqCst)));
            Ok(poll_result(ProviderState::Ok, None, OK_LOG))
        },
        || None,
        trace_into(&Arc::new(Mutex::new(Vec::new()))),
    );
    let schedule = start().after_no_reading(now());

    provider
        .poll(&schedule, now() + minutes(1), &AtomicBool::new(false))
        .unwrap();

    assert_eq!(
        *seen.lock().unwrap(),
        [(schedule, now() + minutes(1), false)]
    );
}

// The token of a poll is in no line the provider tells and in nothing that can be printed of
// what the poll hands back, whatever GitHub answers, even a body that holds the token.
#[test]
fn no_token_reaches_a_log_line_or_a_printed_answer() {
    let answers: &[(u16, &str)] = &[
        (200, BUSINESS),
        (401, ""),
        (429, ""),
        (500, "{\"token\":\"ghp_SECRETVALUE\"}"),
        (200, "ghp_SECRETVALUE"),
    ];
    for (index, &(status, body)) in answers.iter().enumerate() {
        let folder = TestFolder::new(&format!("copilot_provider-secret-{index}"));
        let sender = Arc::new(FakeSender::new(move || reply(status, body)));
        let logged = Arc::new(Mutex::new(Vec::new()));
        let provider = CopilotProvider::new(
            CopilotUsageClient::new(sender, "Sobreiro/0.7.0"),
            vec![env_with_secret()],
            &folder.file("cache.json"),
            trace_into(&logged),
        );

        let poll = provider
            .poll(&start(), now(), &AtomicBool::new(false))
            .unwrap();
        let printed = format!("{:?} {:?} {}", poll.schedule, poll.state, poll.log);
        (poll.keep.unwrap())(now());
        let shown = format!("{:?} {:?}", provider.shown(), provider.limits(now()));

        assert!(!printed.contains(SECRET), "{status}: {printed}");
        assert!(!shown.contains(SECRET), "{status}: {shown}");
        let logged = logged.lock().unwrap();
        assert_eq!(logged.len(), 1);
        assert!(
            logged.iter().all(|line| !line.contains(SECRET)),
            "{logged:?}"
        );
    }
}
