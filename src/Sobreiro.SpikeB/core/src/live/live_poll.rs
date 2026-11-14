use super::{AccessToken, ClaudeCredentials, CredentialRead, CredentialStatus, LiveOutcome, LiveResult, LiveSchedule, LiveUsageClient};
use crate::Time;
use crate::polling::ProviderState;
use crate::usage::RateLimits;

/// One poll's answer: the next schedule, the footer state, the reading to merge (only on success) and the outcome's
/// name for the debug log. Nothing in it holds the token.
#[derive(Clone, Debug, PartialEq)]
pub struct LivePollResult {
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    pub limits: Option<RateLimits>,
    pub outcome: String,
}

/// One Live usage poll: read the sign-in, then ask Anthropic, but only with a token that is there, not expired and
/// not the one Anthropic last refused.
pub struct LivePoll;

impl LivePoll {
    /// Blocks while it asks. Safe on a worker thread.
    pub fn run(credentials_path: &str, client: &LiveUsageClient, schedule: &LiveSchedule, now: Time) -> LivePollResult {
        Self::run_with(|at| ClaudeCredentials::read(credentials_path, at), |t, at| client.fetch(t, at), schedule, now)
    }

    /// The poll with its two outside steps passed in: reading the sign-in, and asking Anthropic with a token.
    pub fn run_with(
        read_credentials: impl FnOnce(Time) -> CredentialRead,
        fetch: impl FnOnce(&AccessToken, Time) -> LiveResult,
        schedule: &LiveSchedule,
        now: Time,
    ) -> LivePollResult {
        let read = read_credentials(now);
        let token = match (read.status, read.token) {
            (CredentialStatus::Ok, Some(token)) => token,
            (status, _) => {
                // No request without a token that is there and not expired.
                let expired = status == CredentialStatus::Expired;
                let name = if status == CredentialStatus::Ok { "Unreadable".to_string() } else { format!("{status:?}") };
                return LivePollResult {
                    schedule: schedule.after_no_credentials(now),
                    state: if expired { ProviderState::Expired } else { ProviderState::NoSignIn },
                    limits: None,
                    outcome: name,
                };
            }
        };

        // The same expiry means the same sign-in Anthropic refused: wait for Claude Code to save a new one.
        if schedule.refuses(token.expires_at()) {
            return LivePollResult { schedule: schedule.after_no_credentials(now), state: ProviderState::Refused, limits: None, outcome: "Refused".into() };
        }

        let result = fetch(&token, now);
        if result.outcome == LiveOutcome::Unauthorized {
            return LivePollResult {
                schedule: schedule.after_refused(token.expires_at(), now),
                state: ProviderState::Refused,
                limits: None,
                outcome: "Unauthorized".into(),
            };
        }

        let state = match result.outcome {
            LiveOutcome::Ok => ProviderState::Ok,
            LiveOutcome::RateLimited => ProviderState::RateLimited,
            LiveOutcome::Unreachable => ProviderState::Offline,
            _ => ProviderState::BadAnswer,
        };
        LivePollResult {
            schedule: schedule.after_fetch(&result, now).expect("a refusal was handled above"),
            state,
            limits: if result.outcome == LiveOutcome::Ok { result.limits } else { None },
            outcome: format!("{:?}", result.outcome),
        }
    }
}
