//! Ported from `tests/Sobreiro.Tests/CopilotPollTests.cs`. The tests after
//! `cancellation_is_the_only_error` are new.
//!
//! No test here reaches the network: the client is handed a sender that answers as the test
//! says, one answer after the other in the order the requests come. Every token is made up.
//! A place that fails is one whose function gives `SignInFailed`; the C# threw an exception
//! there.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::copilot::GitHubToken;
use sobreiro_core::copilot::copilot_poll::{self, CopilotPollResult};
use sobreiro_core::copilot::copilot_sign_in::{
    CopilotSignIn, CopilotSourceKind, SignInFailed, TokenSearch,
};
use sobreiro_core::copilot::copilot_usage_client::CopilotUsageClient;
use sobreiro_core::io::http::{Answer, NoAnswer};
use sobreiro_core::live::live_schedule::LiveSchedule;
use sobreiro_core::live::live_usage_client::Cancelled;
use sobreiro_core::polling::ProviderState;

use crate::{
    FakeSender, H, M, NONE, ScheduleRow, at_ticks, reply, schedule_of, time, too_many_requests,
};

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

fn minutes(count: i64) -> TimeDelta {
    TimeDelta::minutes(count)
}

fn start() -> LiveSchedule {
    LiveSchedule::start(now())
}

fn search(tokens: &[&str], entry_found: bool) -> TokenSearch {
    TokenSearch {
        tokens: tokens
            .iter()
            .map(|token| GitHubToken::new((*token).to_string()))
            .collect(),
        entry_found,
    }
}

/// A place that gives these tokens.
fn source(kind: CopilotSourceKind, tokens: &'static [&'static str]) -> CopilotSignIn {
    CopilotSignIn {
        kind,
        tokens: Box::new(move || Ok(search(tokens, false))),
    }
}

/// A place that saw an entry and no token in it.
fn entry_without_token(kind: CopilotSourceKind) -> CopilotSignIn {
    CopilotSignIn {
        kind,
        tokens: Box::new(|| Ok(search(&[], true))),
    }
}

/// A place that fails.
fn failing(kind: CopilotSourceKind) -> CopilotSignIn {
    CopilotSignIn {
        kind,
        tokens: Box::new(|| Err(SignInFailed)),
    }
}

/// The answers of a sender, one for each request in the order they come: a status and a body.
/// A request past the end of the list is a test that sent more than it should.
fn answers(
    list: &'static [(u16, &'static str)],
) -> impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static {
    let next = AtomicUsize::new(0);
    move || {
        let (status, body) = list[next.fetch_add(1, Ordering::SeqCst)];
        reply(status, body)
    }
}

/// What each request held in its `Authorization` header after `Bearer `, in order.
fn tokens_sent(sender: &FakeSender) -> Vec<String> {
    sender
        .seen()
        .iter()
        .map(|seen| {
            let (_, value) = seen
                .headers
                .iter()
                .find(|(name, _)| name == "Authorization")
                .unwrap();
            value.strip_prefix("Bearer ").unwrap().to_string()
        })
        .collect()
}

/// One poll with these places. Gives the result and the tokens the sender was asked with.
fn run(
    respond: impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static,
    sign_ins: Vec<CopilotSignIn>,
) -> (CopilotPollResult, Vec<String>) {
    let sender = Arc::new(FakeSender::new(respond));
    let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
    let result =
        copilot_poll::run(&sign_ins, &client, &start(), now(), &AtomicBool::new(false)).unwrap();
    (result, tokens_sent(&sender))
}

use CopilotSourceKind::{CredentialManager, Env, Gh};

#[test]
fn first_accepted_token_wins_and_later_sources_are_not_called() {
    let (result, sent) = run(
        || reply(200, BUSINESS),
        // The place after the first must not even be asked: it would fail the poll's log.
        vec![source(Env, &["A"]), failing(Gh)],
    );
    assert_eq!(result.state, ProviderState::Ok);
    assert!(result.reading.is_some());
    assert_eq!(
        result.log,
        "copilot Ok env=ok credential-manager=skipped gh=skipped"
    );
    assert_eq!(result.schedule.last_success, Some(now()));
    assert_eq!(sent, ["A"]);
}

#[test]
fn refused_and_no_budget_fall_through() {
    let (result, sent) = run(
        answers(&[(401, ""), (200, FREE), (200, BUSINESS)]),
        vec![
            source(Env, &["A"]),
            source(CredentialManager, &["B"]),
            source(Gh, &["C"]),
        ],
    );
    assert_eq!(result.state, ProviderState::Ok);
    assert_eq!(
        result.log,
        "copilot Ok env=refused credential-manager=no-budget gh=ok"
    );
    assert_eq!(sent, ["A", "B", "C"]);
}

#[test]
fn all_refused_is_refused_with_a_five_minute_wait() {
    let (result, _) = run(
        || reply(401, ""),
        vec![source(Env, &["A"]), source(Gh, &["C"])],
    );
    assert_eq!(result.state, ProviderState::Refused);
    assert_eq!(result.reading, None);
    assert_eq!(
        result.log,
        "copilot Refused env=refused credential-manager=skipped gh=refused"
    );
    assert_eq!(result.schedule.next_due, now() + minutes(5));
    assert!(!result.schedule.fetch_on_open(now() + minutes(2)));
}

#[test]
fn no_tokens_anywhere_is_no_sign_in() {
    let (result, sent) = run(
        || reply(200, BUSINESS),
        vec![
            source(Env, &[]),
            source(CredentialManager, &[]),
            source(Gh, &[]),
        ],
    );
    assert_eq!(result.state, ProviderState::NoSignIn);
    assert_eq!(
        result.log,
        "copilot NoSignIn env=none credential-manager=none gh=none"
    );
    assert!(sent.is_empty());
    assert_eq!(result.schedule.next_due, now() + minutes(5));
}

#[test]
fn entry_without_a_token_is_logged_as_no_token() {
    let (result, sent) = run(
        || reply(200, BUSINESS),
        vec![
            source(Env, &[]),
            entry_without_token(CredentialManager),
            source(Gh, &[]),
        ],
    );
    assert_eq!(result.state, ProviderState::NoSignIn);
    assert_eq!(
        result.log,
        "copilot NoSignIn env=none credential-manager=no-token gh=none"
    );
    assert!(sent.is_empty());
}

#[test]
fn a_source_that_fails_is_error_and_skipped() {
    let (result, sent) = run(
        || reply(200, BUSINESS),
        vec![
            source(Env, &[]),
            failing(CredentialManager),
            source(Gh, &["C"]),
        ],
    );
    assert_eq!(result.state, ProviderState::Ok);
    assert_eq!(
        result.log,
        "copilot Ok env=none credential-manager=error gh=ok"
    );
    assert_eq!(sent, ["C"]);
}

#[test]
fn only_a_failing_source_is_no_sign_in() {
    let (result, _) = run(|| reply(200, BUSINESS), vec![failing(Gh)]);
    assert_eq!(result.state, ProviderState::NoSignIn);
    assert_eq!(
        result.log,
        "copilot NoSignIn env=skipped credential-manager=skipped gh=error"
    );
}

#[test]
fn rate_limit_stops_the_chain() {
    let (result, sent) = run(
        || too_many_requests("600"),
        vec![source(Env, &["A"]), failing(Gh)],
    );
    assert_eq!(result.state, ProviderState::RateLimited);
    assert_eq!(
        result.log,
        "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped"
    );
    assert_eq!(result.schedule.next_due, now() + minutes(10));
    assert_eq!(sent.len(), 1);
}

#[test]
fn a_500_is_a_bad_answer_and_stops_the_chain() {
    let (result, sent) = run(|| reply(500, ""), vec![source(Env, &["A"]), failing(Gh)]);
    assert_eq!(result.state, ProviderState::BadAnswer);
    assert_eq!(
        result.log,
        "copilot BadAnswer env=failed credential-manager=skipped gh=skipped"
    );
    assert_eq!(result.schedule.next_due, now() + minutes(2));
    assert_eq!(sent.len(), 1);
}

#[test]
fn network_failure_is_offline_and_stops_the_chain() {
    let (result, sent) = run(
        || Err(NoAnswer::Unreachable),
        vec![source(Env, &["A"]), failing(Gh)],
    );
    assert_eq!(result.state, ProviderState::Offline);
    assert_eq!(
        result.log,
        "copilot Offline env=unreachable credential-manager=skipped gh=skipped"
    );
    assert_eq!(result.schedule.next_due, now() + minutes(2));
    assert_eq!(sent.len(), 1);
}

#[test]
fn several_tokens_in_one_source_log_the_last_result() {
    let (result, _) = run(
        answers(&[(401, ""), (200, FREE)]),
        vec![source(Gh, &["A", "B"])],
    );
    assert_eq!(result.state, ProviderState::Refused);
    assert_eq!(
        result.log,
        "copilot Refused env=skipped credential-manager=skipped gh=no-budget"
    );
}

#[test]
fn log_never_holds_a_token() {
    let (result, _) = run(
        || reply(401, ""),
        vec![
            source(Env, &["gho_made_up_one"]),
            source(Gh, &["gho_made_up_two"]),
        ],
    );
    assert!(!result.log.contains("gho_"));
}

#[test]
fn cancellation_is_the_only_error() {
    let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
    let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
    let result = copilot_poll::run(
        &[source(Env, &["A"])],
        &client,
        &start(),
        now(),
        &AtomicBool::new(true),
    );
    assert!(matches!(result, Err(Cancelled)));
    assert!(sender.seen().is_empty());
}

#[test]
fn the_flag_is_looked_at_before_each_source() {
    // The first place gives the poll up while it runs. The second is not run, and nothing is
    // sent.
    let cancelled = Arc::new(AtomicBool::new(false));
    let second_ran = Arc::new(AtomicBool::new(false));
    let (flag, ran) = (cancelled.clone(), second_ran.clone());
    let sign_ins = vec![
        CopilotSignIn {
            kind: Env,
            tokens: Box::new(move || {
                flag.store(true, Ordering::SeqCst);
                Ok(search(&[], false))
            }),
        },
        CopilotSignIn {
            kind: Gh,
            tokens: Box::new(move || {
                ran.store(true, Ordering::SeqCst);
                Ok(search(&["A"], false))
            }),
        },
    ];
    let sender = Arc::new(FakeSender::new(|| reply(200, BUSINESS)));
    let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
    let result = copilot_poll::run(&sign_ins, &client, &start(), now(), &cancelled);
    assert!(matches!(result, Err(Cancelled)));
    assert!(!second_ran.load(Ordering::SeqCst));
    assert!(sender.seen().is_empty());
}

#[test]
fn a_source_that_fails_while_the_poll_is_given_up_is_a_cancellation() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let sign_ins = vec![CopilotSignIn {
        kind: Env,
        tokens: Box::new(move || {
            flag.store(true, Ordering::SeqCst);
            Err(SignInFailed)
        }),
    }];
    let client = CopilotUsageClient::new(
        Arc::new(FakeSender::new(|| reply(200, BUSINESS))),
        "Sobreiro/0.7.0",
    );
    let result = copilot_poll::run(&sign_ins, &client, &start(), now(), &cancelled);
    assert!(matches!(result, Err(Cancelled)));
}

#[test]
fn a_request_that_the_caller_gives_up_comes_back_as_cancelled() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let sender = Arc::new(FakeSender::new(move || {
        flag.store(true, Ordering::SeqCst);
        reply(200, BUSINESS)
    }));
    let client = CopilotUsageClient::new(sender, "Sobreiro/0.7.0");
    let result = copilot_poll::run(&[source(Env, &["A"])], &client, &start(), now(), &cancelled);
    assert!(matches!(result, Err(Cancelled)));
}

#[test]
fn no_sign_ins_at_all_is_no_sign_in_with_every_place_skipped() {
    let (result, _) = run(|| reply(200, BUSINESS), Vec::new());
    assert_eq!(result.state, ProviderState::NoSignIn);
    assert_eq!(
        result.log,
        "copilot NoSignIn env=skipped credential-manager=skipped gh=skipped"
    );
    assert_eq!(result.schedule, start().after_no_reading(now()));
}

#[test]
fn a_good_reading_comes_with_the_schedule_of_a_success() {
    let (result, _) = run(|| reply(200, BUSINESS), vec![source(Env, &["A"])]);
    assert_eq!(result.reading.unwrap().used, 15000.0 - 12355.8);
    assert_eq!(result.schedule.next_due, now() + minutes(5));
}

#[test]
fn a_poll_result_printed_whole_and_its_log_never_show_a_token() {
    // One poll that ends well and one that does not, with the made-up tokens in every place.
    let polls: [(u16, &str); 2] = [(200, BUSINESS), (401, "")];
    for (status, body) in polls {
        let (result, _) = run(
            move || reply(status, body),
            vec![
                source(Env, &["gho_made_up_one"]),
                source(CredentialManager, &["ghp_made_up_two"]),
                source(Gh, &["github_pat_made_up_three"]),
            ],
        );
        let printed = format!("{result:?}");
        for text in [&printed, &result.log] {
            for part in ["gho_", "ghp_", "github_pat_", "made_up"] {
                assert!(!text.contains(part), "{part} in {text}");
            }
        }
    }
}

// ---- Rows the C# wrote ---------------------------------------------------------------------
//
// Everything below was written by a script from the answers of the C# of 1.1.0, which was run
// on these made-up cases on 2026-11-15. Nothing below was typed. A moment is a count of ticks
// from noon on 2026-11-09 (UTC), written with the units of `main.rs`; every poll is at that
// noon. `NONE` stands where there is no moment.

/// What a place found, by name: nothing; an entry with no token in it; a failure; or one or
/// two tokens. The tokens are made up and differ from place to place.
fn place(kind: CopilotSourceKind, spec: &str) -> CopilotSignIn {
    let name = match kind {
        Env => "gho_made_up_env",
        CredentialManager => "ghp_made_up_cm",
        Gh => "ghu_made_up_gh",
    };
    let spec = spec.to_string();
    CopilotSignIn {
        kind,
        tokens: Box::new(move || {
            let first = format!("{name}_1");
            let second = format!("{name}_2");
            let found = |tokens: Vec<&str>, entry_found: bool| TokenSearch {
                tokens: tokens
                    .into_iter()
                    .map(|token| GitHubToken::new(token.to_string()))
                    .collect(),
                entry_found,
            };
            match spec.as_str() {
                "none" => Ok(found(vec![], false)),
                "no token" => Ok(found(vec![], true)),
                "error" => Err(SignInFailed),
                "one token" => Ok(found(vec![first.as_str()], false)),
                "two tokens" => Ok(found(vec![first.as_str(), second.as_str()], false)),
                other => panic!("{other}"),
            }
        }),
    }
}

/// GitHub's answers of the rows, by name: the status, `Retry-After`, and the body. A status
/// below zero is no answer at all: no way to the host.
#[rustfmt::skip]
const REPLIES: &[(&str, i32, Option<&str>, &str)] = &[
    ("ok", 200, None, BUSINESS_ONE_LINE),
    ("401", 401, None, ""),
    ("403", 403, Some("600"), ""),
    ("404", 404, None, ""),
    ("no budget", 200, None, FREE),
    ("429", 429, None, ""),
    ("429 after 30", 429, Some("30"), ""),
    ("429 after 600", 429, Some("600"), ""),
    ("429 after a day", 429, Some("86400"), ""),
    ("no route", -1, None, ""),
    ("500", 500, None, ""),
    ("200 unexpected", 200, None, "{\"unexpected\":true}"),
    ("200 not json", 200, None, "not json"),
    ("200 empty", 200, None, ""),
];

const BUSINESS_ONE_LINE: &str = "{\"copilot_plan\":\"business\",\"quota_reset_date_utc\":\"2026-12-01T00:00:00.000Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"remaining\":12355,\"quota_remaining\":12355.8,\"unlimited\":false,\"has_quota\":true,\"timestamp_utc\":\"2026-11-10T09:25:19.365Z\"}}}";

#[test]
fn a_poll_answers_as_the_csharp_did() {
    use ProviderState as P;
    // The three places (each by what it found), GitHub's answers in the order the requests
    // come, and the schedule before; then how many requests were sent, the state, the line
    // for the log, the schedule after, and whether a reading came. A request past the end of
    // the answers gets a 599, which no row expects.
    type Row = (
        &'static str,
        &'static str,
        &'static str,
        &'static [&'static str],
        ScheduleRow,
        usize,
        P,
        &'static str,
        ScheduleRow,
        bool,
    );
    #[rustfmt::skip]
    let rows: &[Row] = &[
        ("none", "none", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "none", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "none", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("no token", "none", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("no token", "none", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("no token", "none", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("none", "no token", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=no-token gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "no token", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=no-token gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "no token", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=no-token gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("none", "none", "no token", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=no-token", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "none", "no token", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=no-token", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "none", "no token", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=no-token", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("no token", "no token", "no token", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=no-token gh=no-token", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("no token", "no token", "no token", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=no-token gh=no-token", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("no token", "no token", "no token", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=no-token gh=no-token", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("error", "error", "error", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=error gh=error", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("error", "error", "error", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=error gh=error", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("error", "error", "error", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=error gh=error", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("error", "none", "no token", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=no-token", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("error", "none", "no token", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=no-token", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("error", "none", "no token", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=no-token", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("none", "error", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=error gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "error", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=error gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "error", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=error gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("error", "none", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("error", "none", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("error", "none", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=error credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("none", "none", "error", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=error", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "none", "error", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=error", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "none", "error", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=none credential-manager=none gh=error", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("no token", "error", "none", &[], (0, NONE, 0, NONE, NONE, 0), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=error gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("no token", "error", "none", &[], (0, -5 * M, 0, 0, NONE, 1), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=error gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("no token", "error", "none", &[], (0, NONE, 2, -M, NONE, 2), 0, P::NoSignIn, "copilot NoSignIn env=no-token credential-manager=error gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["ok"], (0, NONE, 0, NONE, NONE, 0), 1, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "none", "none", &["ok"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "none", "none", &["ok"], (0, NONE, 2, -M, NONE, 2), 1, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "none", "none", &["401"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["401"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["401"], (0, NONE, 2, -M, NONE, 2), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["403"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["403"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["403"], (0, NONE, 2, -M, NONE, 2), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["404"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["404"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["404"], (0, NONE, 2, -M, NONE, 2), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["no budget"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["no budget"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["no budget"], (0, NONE, 2, -M, NONE, 2), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["429"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["429"], (0, -5 * M, 0, 0, NONE, 1), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (5 * M, -5 * M, 1, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["429"], (0, NONE, 2, -M, NONE, 2), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("one token", "none", "none", &["429 after 30"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["429 after 30"], (0, -5 * M, 0, 0, NONE, 1), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (5 * M, -5 * M, 1, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["429 after 30"], (0, NONE, 2, -M, NONE, 2), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("one token", "none", "none", &["429 after 600"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("one token", "none", "none", &["429 after 600"], (0, -5 * M, 0, 0, NONE, 1), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (10 * M, -5 * M, 1, 10 * M, NONE, 2), false),
        ("one token", "none", "none", &["429 after 600"], (0, NONE, 2, -M, NONE, 2), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("one token", "none", "none", &["429 after a day"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (6 * H, NONE, 1, 6 * H, NONE, 1), false),
        ("one token", "none", "none", &["429 after a day"], (0, -5 * M, 0, 0, NONE, 1), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (6 * H, -5 * M, 1, 6 * H, NONE, 2), false),
        ("one token", "none", "none", &["429 after a day"], (0, NONE, 2, -M, NONE, 2), 1, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (6 * H, NONE, 3, 6 * H, NONE, 3), false),
        ("one token", "none", "none", &["no route"], (0, NONE, 0, NONE, NONE, 0), 1, P::Offline, "copilot Offline env=unreachable credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("one token", "none", "none", &["no route"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Offline, "copilot Offline env=unreachable credential-manager=skipped gh=skipped", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("one token", "none", "none", &["no route"], (0, NONE, 2, -M, NONE, 2), 1, P::Offline, "copilot Offline env=unreachable credential-manager=skipped gh=skipped", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("one token", "none", "none", &["500"], (0, NONE, 0, NONE, NONE, 0), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("one token", "none", "none", &["500"], (0, -5 * M, 0, 0, NONE, 1), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("one token", "none", "none", &["500"], (0, NONE, 2, -M, NONE, 2), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("one token", "none", "none", &["200 unexpected"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "none", &["200 unexpected"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "none", "none", &["200 unexpected"], (0, NONE, 2, -M, NONE, 2), 1, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "none", "none", &["200 not json"], (0, NONE, 0, NONE, NONE, 0), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("one token", "none", "none", &["200 not json"], (0, -5 * M, 0, 0, NONE, 1), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("one token", "none", "none", &["200 not json"], (0, NONE, 2, -M, NONE, 2), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("one token", "none", "none", &["200 empty"], (0, NONE, 0, NONE, NONE, 0), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("one token", "none", "none", &["200 empty"], (0, -5 * M, 0, 0, NONE, 1), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("one token", "none", "none", &["200 empty"], (0, NONE, 2, -M, NONE, 2), 1, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 2, 2 * M, NONE, 3), false),
        ("none", "none", "one token", &["ok"], (0, NONE, 0, NONE, NONE, 0), 1, P::Ok, "copilot Ok env=none credential-manager=none gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("none", "none", "one token", &["401"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=none credential-manager=none gh=refused", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("none", "none", "one token", &["429 after 600"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=none credential-manager=none gh=rate-limited", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("none", "none", "one token", &["no route"], (0, NONE, 0, NONE, NONE, 0), 1, P::Offline, "copilot Offline env=none credential-manager=none gh=unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("none", "none", "one token", &["no budget"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=none credential-manager=none gh=no-budget", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "none", "none", &["401", "ok"], (0, -5 * M, 0, 0, NONE, 1), 2, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "none", "none", &["no budget", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "none", "none", &["no budget", "ok"], (0, -5 * M, 0, 0, NONE, 1), 2, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "none", "none", &["401", "401"], (0, NONE, 0, NONE, NONE, 0), 2, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "401"], (0, NONE, 2, -M, NONE, 2), 2, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("two tokens", "none", "none", &["no budget", "401"], (0, NONE, 0, NONE, NONE, 0), 2, P::Refused, "copilot Refused env=refused credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "429 after 600"], (0, NONE, 0, NONE, NONE, 0), 2, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "429 after 600"], (0, NONE, 2, -M, NONE, 2), 2, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("two tokens", "none", "none", &["401", "429"], (0, NONE, 0, NONE, NONE, 0), 2, P::RateLimited, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "no route"], (0, NONE, 0, NONE, NONE, 0), 2, P::Offline, "copilot Offline env=unreachable credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "no route"], (0, -5 * M, 0, 0, NONE, 1), 2, P::Offline, "copilot Offline env=unreachable credential-manager=skipped gh=skipped", (2 * M, -5 * M, 0, 2 * M, NONE, 2), false),
        ("two tokens", "none", "none", &["401", "500"], (0, NONE, 0, NONE, NONE, 0), 2, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["401", "200 not json"], (0, NONE, 0, NONE, NONE, 0), 2, P::BadAnswer, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["ok", "ok"], (0, NONE, 0, NONE, NONE, 0), 1, P::Ok, "copilot Ok env=ok credential-manager=skipped gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "none", "none", &["no budget", "no budget"], (0, NONE, 0, NONE, NONE, 0), 2, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "none", "none", &["no budget", "no budget"], (0, -5 * M, 0, 0, NONE, 1), 2, P::Refused, "copilot Refused env=no-budget credential-manager=none gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("none", "two tokens", "none", &["404", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=none credential-manager=ok gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("none", "none", "two tokens", &["403", "429 after 30"], (0, NONE, 0, NONE, NONE, 0), 2, P::RateLimited, "copilot RateLimited env=none credential-manager=none gh=rate-limited", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("one token", "one token", "one token", &["401", "401", "401"], (0, NONE, 0, NONE, NONE, 0), 3, P::Refused, "copilot Refused env=refused credential-manager=refused gh=refused", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "one token", "one token", &["401", "401", "401"], (0, -5 * M, 0, 0, NONE, 1), 3, P::Refused, "copilot Refused env=refused credential-manager=refused gh=refused", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "one token", "one token", &["401", "401", "401"], (0, NONE, 2, -M, NONE, 2), 3, P::Refused, "copilot Refused env=refused credential-manager=refused gh=refused", (5 * M, NONE, 2, 5 * M, NONE, 3), false),
        ("one token", "one token", "one token", &["401", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=refused credential-manager=ok gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "one token", "one token", &["401", "ok"], (0, -5 * M, 0, 0, NONE, 1), 2, P::Ok, "copilot Ok env=refused credential-manager=ok gh=skipped", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "one token", "one token", &["no budget", "401", "ok"], (0, NONE, 0, NONE, NONE, 0), 3, P::Ok, "copilot Ok env=no-budget credential-manager=refused gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "one token", "one token", &["401", "401", "ok"], (0, NONE, 0, NONE, NONE, 0), 3, P::Ok, "copilot Ok env=refused credential-manager=refused gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "one token", "one token", &["401", "429 after 600"], (0, NONE, 0, NONE, NONE, 0), 2, P::RateLimited, "copilot RateLimited env=refused credential-manager=rate-limited gh=skipped", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
        ("one token", "one token", "one token", &["401", "429 after 600"], (0, NONE, 2, -M, NONE, 2), 2, P::RateLimited, "copilot RateLimited env=refused credential-manager=rate-limited gh=skipped", (20 * M, NONE, 3, 20 * M, NONE, 3), false),
        ("one token", "one token", "one token", &["401", "401", "no route"], (0, NONE, 0, NONE, NONE, 0), 3, P::Offline, "copilot Offline env=refused credential-manager=refused gh=unreachable", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("one token", "one token", "one token", &["no budget", "no budget", "no budget"], (0, NONE, 0, NONE, NONE, 0), 3, P::Refused, "copilot Refused env=no-budget credential-manager=no-budget gh=no-budget", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "one token", &["401", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=refused credential-manager=none gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("error", "none", "one token", &["ok"], (0, NONE, 0, NONE, NONE, 0), 1, P::Ok, "copilot Ok env=error credential-manager=none gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("error", "none", "one token", &["ok"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Ok, "copilot Ok env=error credential-manager=none gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("error", "error", "one token", &["401"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=error credential-manager=error gh=refused", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("no token", "error", "one token", &["429"], (0, NONE, 0, NONE, NONE, 0), 1, P::RateLimited, "copilot RateLimited env=no-token credential-manager=error gh=rate-limited", (5 * M, NONE, 1, 5 * M, NONE, 1), false),
        ("no token", "no token", "one token", &["500"], (0, NONE, 0, NONE, NONE, 0), 1, P::BadAnswer, "copilot BadAnswer env=no-token credential-manager=no-token gh=failed", (2 * M, NONE, 0, 2 * M, NONE, 1), false),
        ("none", "error", "two tokens", &["401", "ok"], (0, NONE, 0, NONE, NONE, 0), 2, P::Ok, "copilot Ok env=none credential-manager=error gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("one token", "error", "none", &["401"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=refused credential-manager=error gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "error", "none", &["401"], (0, -5 * M, 0, 0, NONE, 1), 1, P::Refused, "copilot Refused env=refused credential-manager=error gh=none", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("one token", "no token", "none", &["no budget"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=no-budget credential-manager=no-token gh=none", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("one token", "none", "error", &["401"], (0, NONE, 0, NONE, NONE, 0), 1, P::Refused, "copilot Refused env=refused credential-manager=none gh=error", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "two tokens", "two tokens", &["401", "401", "401", "401", "401", "401"], (0, NONE, 0, NONE, NONE, 0), 6, P::Refused, "copilot Refused env=refused credential-manager=refused gh=refused", (5 * M, NONE, 0, 5 * M, NONE, 1), false),
        ("two tokens", "two tokens", "two tokens", &["401", "401", "401", "401", "401", "401"], (0, -5 * M, 0, 0, NONE, 1), 6, P::Refused, "copilot Refused env=refused credential-manager=refused gh=refused", (5 * M, -5 * M, 0, 5 * M, NONE, 2), false),
        ("two tokens", "two tokens", "two tokens", &["401", "401", "401", "401", "401", "ok"], (0, NONE, 0, NONE, NONE, 0), 6, P::Ok, "copilot Ok env=refused credential-manager=refused gh=ok", (5 * M, 0, 0, NONE, NONE, 0), true),
        ("two tokens", "one token", "none", &["no budget", "no budget", "429 after 600"], (0, NONE, 0, NONE, NONE, 0), 3, P::RateLimited, "copilot RateLimited env=no-budget credential-manager=rate-limited gh=skipped", (10 * M, NONE, 1, 10 * M, NONE, 1), false),
    ];
    assert_eq!(rows.len(), 125);
    for (index, &(env, credential_manager, gh, answers, before, sent, state, log, after, reads)) in
        rows.iter().enumerate()
    {
        let row = format!("row {index}: {env}, {credential_manager}, {gh}, {answers:?}");
        let next = AtomicUsize::new(0);
        let sender = Arc::new(FakeSender::new(move || -> Result<Answer, NoAnswer> {
            let Some(name) = answers.get(next.fetch_add(1, Ordering::SeqCst)) else {
                return reply(599, "");
            };
            let &(_, status, retry_after, body) =
                REPLIES.iter().find(|reply| reply.0 == *name).unwrap();
            let Ok(status) = u16::try_from(status) else {
                return Err(NoAnswer::Unreachable);
            };
            Ok(Answer {
                status,
                retry_after: retry_after.map(str::to_string),
                body: body.as_bytes().to_vec(),
            })
        }));
        let client = CopilotUsageClient::new(sender.clone(), "Sobreiro/0.7.0");
        let sign_ins = [
            place(Env, env),
            place(CredentialManager, credential_manager),
            place(Gh, gh),
        ];

        let result = copilot_poll::run(
            &sign_ins,
            &client,
            &schedule_of(before),
            at_ticks(0),
            &AtomicBool::new(false),
        )
        .unwrap();

        assert_eq!(sender.seen().len(), sent, "{row}: requests sent");
        assert_eq!(result.state, state, "{row}: state");
        assert_eq!(result.log, log, "{row}: log");
        assert_eq!(result.schedule, schedule_of(after), "{row}: schedule");
        assert_eq!(result.reading.is_some(), reads, "{row}: reading");
    }
}
