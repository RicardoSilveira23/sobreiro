//! One poll for Copilot's AI credits: go through the places that may hold a GitHub token, in
//! order, and ask GitHub with each token until one is accepted. A refusal, or a plan with no
//! budget, goes on to the next token. A 429 or a failure stops the chain.
//!
//! A token is found for this one poll. It is lent to the request, and is gone when the
//! request is: nothing here or in the result has a place to keep it, and the log names
//! outcomes only.
//!
//! A poll waits for its answers, ten seconds at most for each, so the app runs it away from
//! the thread that draws the window.
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};

use super::CopilotReading;
use super::copilot_sign_in::{CopilotSignIn, CopilotSourceKind};
use super::copilot_usage_client::{CopilotOutcome, CopilotUsageClient};
use crate::live::live_schedule::LiveSchedule;
use crate::live::live_usage_client::{Cancelled, LiveOutcome, LiveResult};
use crate::polling::ProviderState;

/// One poll's answer: the next schedule, the state for the foot of the flyout, the reading
/// when there is one, and the log line. Nothing in it holds a token: the log is built from
/// fixed words only.
#[derive(Debug, Clone, PartialEq)]
pub struct CopilotPollResult {
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    pub reading: Option<CopilotReading>,
    pub log: String,
}

/// What each of the three places came to, for the log. `None` is a place the poll never
/// reached.
#[derive(Default)]
struct Results {
    env: Option<&'static str>,
    credential_manager: Option<&'static str>,
    gh: Option<&'static str>,
}

impl Results {
    fn set(&mut self, kind: CopilotSourceKind, word: &'static str) {
        let slot = match kind {
            CopilotSourceKind::Env => &mut self.env,
            CopilotSourceKind::CredentialManager => &mut self.credential_manager,
            CopilotSourceKind::Gh => &mut self.gh,
        };
        *slot = Some(word);
    }
}

/// One poll, with the places in `sign_ins`, tried in that order. `Err(Cancelled)` only when
/// the caller set `cancelled`: it is looked at before each place, and see
/// [`CopilotUsageClient::fetch`] for a request.
pub fn run(
    sign_ins: &[CopilotSignIn],
    client: &CopilotUsageClient,
    schedule: &LiveSchedule,
    now: DateTime<Utc>,
    cancelled: &AtomicBool,
) -> Result<CopilotPollResult, Cancelled> {
    let mut results = Results::default();
    let mut any_token = false;

    // What a poll comes to when the schedule moves on after `outcome` and no reading came.
    let stopped = |results: &Results, outcome, retry_after, state| {
        // Copilot never asks for a refusal to be remembered, so `after_fetch` always answers.
        // Were it ever to refuse, the poll counts as a failed request.
        let next = schedule
            .after_fetch(
                &LiveResult {
                    outcome,
                    limits: None,
                    retry_after,
                },
                now,
            )
            .unwrap_or_else(|_| schedule.after_failure(now));
        done(next, state, None, results)
    };

    for sign_in in sign_ins {
        if cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }
        let search = match (sign_in.tokens)() {
            Ok(search) => search,
            Err(_) => {
                // A place that failed while the caller gave the poll up is part of the
                // cancellation.
                if cancelled.load(Ordering::SeqCst) {
                    return Err(Cancelled);
                }
                results.set(sign_in.kind, "error");
                continue;
            }
        };

        if search.tokens.is_empty() {
            results.set(
                sign_in.kind,
                if search.entry_found {
                    "no-token"
                } else {
                    "none"
                },
            );
        }
        // Each token is taken out of the list for its own request: `for token in list`, with
        // no `&`, hands the tokens over one by one, where `&list` would only lend them.
        for token in search.tokens {
            any_token = true;
            // `?` hands a cancellation straight back to the caller.
            let fetch = client.fetch(&token, now, cancelled)?;
            match (fetch.outcome, fetch.reading) {
                (CopilotOutcome::Ok, Some(reading)) => {
                    results.set(sign_in.kind, "ok");
                    let next = schedule
                        .after_fetch(
                            &LiveResult {
                                outcome: LiveOutcome::Ok,
                                limits: None,
                                retry_after: None,
                            },
                            now,
                        )
                        .unwrap_or_else(|_| schedule.after_failure(now));
                    return Ok(done(next, ProviderState::Ok, Some(reading), &results));
                }
                (CopilotOutcome::Refused, _) => results.set(sign_in.kind, "refused"),
                (CopilotOutcome::NoBudget, _) => results.set(sign_in.kind, "no-budget"),
                (CopilotOutcome::RateLimited, _) => {
                    results.set(sign_in.kind, "rate-limited");
                    return Ok(stopped(
                        &results,
                        LiveOutcome::RateLimited,
                        fetch.retry_after,
                        ProviderState::RateLimited,
                    ));
                }
                (CopilotOutcome::Unreachable, _) => {
                    results.set(sign_in.kind, "unreachable");
                    return Ok(stopped(
                        &results,
                        LiveOutcome::Unreachable,
                        None,
                        ProviderState::Offline,
                    ));
                }
                // A failure, and an answer that says it is Ok but holds no reading.
                (CopilotOutcome::Failed | CopilotOutcome::Ok, _) => {
                    results.set(sign_in.kind, "failed");
                    return Ok(stopped(
                        &results,
                        LiveOutcome::Failed,
                        None,
                        ProviderState::BadAnswer,
                    ));
                }
            }
            // The token goes out of use here, and Rust frees it before the next is asked with.
        }
    }

    let state = if any_token {
        ProviderState::Refused
    } else {
        ProviderState::NoSignIn
    };
    Ok(done(schedule.after_no_reading(now), state, None, &results))
}

/// The answer, with its log line: `copilot`, the state's name, and what each place came to.
fn done(
    schedule: LiveSchedule,
    state: ProviderState,
    reading: Option<CopilotReading>,
    results: &Results,
) -> CopilotPollResult {
    let of = |word: Option<&'static str>| word.unwrap_or("skipped");
    // `{state:?}` prints the state's name, as the C# enum's does: the log is for the same
    // reader as before.
    let log = format!(
        "copilot {state:?} env={} credential-manager={} gh={}",
        of(results.env),
        of(results.credential_manager),
        of(results.gh),
    );
    CopilotPollResult {
        schedule,
        state,
        reading,
        log,
    }
}
