use super::{CopilotOutcome, CopilotReading, CopilotSignIn, CopilotSourceKind, CopilotUsageClient};
use crate::Time;
use crate::live::{LiveOutcome, LiveResult, LiveSchedule};
use crate::polling::{CancelToken, ProviderState};
use std::collections::HashMap;

/// One poll's answer. `log` names outcomes only; nothing in it holds a token.
#[derive(Clone, Debug, PartialEq)]
pub struct CopilotPollResult {
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    pub reading: Option<CopilotReading>,
    pub log: String,
}

/// One Copilot poll: go through the sign-ins in order and ask GitHub with each token until one is accepted. A
/// refusal or a plan without a budget goes on to the next token; a 429 or a failure stops the chain.
pub struct CopilotPoll;

impl CopilotPoll {
    /// Blocks while it asks. Safe on a worker thread. `cancel` is looked at before each sign-in: a cancelled poll
    /// stops there and its answer is dropped by the poller.
    pub fn run(
        sign_ins: &[CopilotSignIn], client: &CopilotUsageClient, schedule: &LiveSchedule, now: Time, cancel: &CancelToken,
    ) -> CopilotPollResult {
        Self::run_with(sign_ins, |t, at| client.fetch(t, at), schedule, now, cancel)
    }

    /// The poll with the request passed in.
    pub fn run_with(
        sign_ins: &[CopilotSignIn],
        fetch: impl Fn(&crate::secret::Secret, Time) -> super::CopilotFetch,
        schedule: &LiveSchedule,
        now: Time,
        cancel: &CancelToken,
    ) -> CopilotPollResult {
        let mut results: HashMap<CopilotSourceKind, &'static str> = HashMap::new();
        let mut any_token = false;
        let after = |outcome: LiveOutcome, retry: Option<crate::Span>| {
            schedule.after_fetch(&LiveResult { outcome, limits: None, retry_after: retry }, now).expect("not a refusal")
        };

        for sign_in in sign_ins {
            if cancel.is_cancelled() {
                return CopilotPollResult { schedule: *schedule, state: ProviderState::Ok, reading: None, log: "copilot Cancelled".into() };
            }
            let search = match (sign_in.tokens)() {
                Ok(s) => s,
                Err(_) => {
                    results.insert(sign_in.kind, "error"); // never the error: its text could hold a token
                    continue;
                }
            };

            if search.tokens.is_empty() {
                results.insert(sign_in.kind, if search.entry_found { "no-token" } else { "none" });
            }
            for token in &search.tokens {
                any_token = true;
                let f = fetch(token, now);
                match f.outcome {
                    CopilotOutcome::Ok if f.reading.is_some() => {
                        results.insert(sign_in.kind, "ok");
                        return done(after(LiveOutcome::Ok, None), ProviderState::Ok, f.reading, &results);
                    }
                    CopilotOutcome::Refused => {
                        results.insert(sign_in.kind, "refused");
                    }
                    CopilotOutcome::NoBudget => {
                        results.insert(sign_in.kind, "no-budget");
                    }
                    CopilotOutcome::RateLimited => {
                        results.insert(sign_in.kind, "rate-limited");
                        return done(after(LiveOutcome::RateLimited, f.retry_after), ProviderState::RateLimited, None, &results);
                    }
                    CopilotOutcome::Unreachable => {
                        results.insert(sign_in.kind, "unreachable");
                        return done(after(LiveOutcome::Unreachable, None), ProviderState::Offline, None, &results);
                    }
                    _ => {
                        results.insert(sign_in.kind, "failed");
                        return done(after(LiveOutcome::Failed, None), ProviderState::BadAnswer, None, &results);
                    }
                }
            }
        }

        done(
            schedule.after_no_reading(now),
            if any_token { ProviderState::Refused } else { ProviderState::NoSignIn },
            None,
            &results,
        )
    }
}

fn done(schedule: LiveSchedule, state: ProviderState, reading: Option<CopilotReading>, results: &HashMap<CopilotSourceKind, &'static str>) -> CopilotPollResult {
    let of = |kind: CopilotSourceKind| results.get(&kind).copied().unwrap_or("skipped");
    let log = format!(
        "copilot {state:?} env={} credential-manager={} gh={}",
        of(CopilotSourceKind::Env),
        of(CopilotSourceKind::CredentialManager),
        of(CopilotSourceKind::Gh)
    );
    CopilotPollResult { schedule, state, reading, log }
}
