//! One poll for Claude Code's usage: read the sign-in, then ask Anthropic, but only with a
//! token that is there, has not run out and is not the one Anthropic last refused.
//!
//! The token is read from the sign-in file for this one poll. It lives in a variable of
//! [`run_with`], is lent to the request, and is gone when the function returns: nothing here
//! or in the result has a place to keep it for the next poll. Sobreiro never renews a sign-in:
//! one that ran out or was refused leads to no request at all, and the file is read again
//! later to see whether Claude Code has saved a new one.
//!
//! A poll waits for its answer, ten seconds at most, so the app runs it away from the thread
//! that draws the window.
use std::path::Path;
use std::sync::atomic::AtomicBool;

use chrono::{DateTime, Utc};

use super::live_schedule::LiveSchedule;
use super::live_usage_client::{Cancelled, LiveUsageClient};
use super::{CredentialRead, LiveOutcome, claude_credentials};
use crate::polling::ProviderState;
use crate::usage::RateLimits;

/// How a poll ended, for the debug log. An outcome is one of these nine and nothing more: it
/// has no text of its own, so no token and no error's words can travel in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollOutcome {
    /// There is no sign-in file.
    Missing,
    /// The sign-in file gave no token.
    Unreadable,
    /// The sign-in has run out.
    Expired,
    /// The file still gives the sign-in Anthropic refused before: nothing was sent.
    Refused,
    /// Anthropic refused the sign-in just now (401 or 403).
    Unauthorized,
    /// An answer with limits in it.
    Ok,
    /// Too many requests (429).
    RateLimited,
    /// An answer that is no use.
    Failed,
    /// No answer.
    Unreachable,
}

impl PollOutcome {
    /// The name as the log shows it, the same as in 1.x.
    pub fn name(self) -> &'static str {
        match self {
            PollOutcome::Missing => "Missing",
            PollOutcome::Unreadable => "Unreadable",
            PollOutcome::Expired => "Expired",
            PollOutcome::Refused => "Refused",
            PollOutcome::Unauthorized => "Unauthorized",
            PollOutcome::Ok => "Ok",
            PollOutcome::RateLimited => "RateLimited",
            PollOutcome::Failed => "Failed",
            PollOutcome::Unreachable => "Unreachable",
        }
    }
}

/// One poll's answer: the next schedule, the state for the foot of the flyout, the reading to
/// merge (only after a success), and how it ended. Nothing in it holds the token.
#[derive(Debug, Clone, PartialEq)]
pub struct LivePollResult {
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    pub limits: Option<RateLimits>,
    pub outcome: PollOutcome,
}

/// One poll, with the sign-in file at `credentials_path`. `Err(Cancelled)` only when the
/// caller set `cancelled`: see [`LiveUsageClient::fetch`].
pub fn run(
    credentials_path: &Path,
    client: &LiveUsageClient,
    schedule: &LiveSchedule,
    now: DateTime<Utc>,
    cancelled: &AtomicBool,
) -> Result<LivePollResult, Cancelled> {
    run_with(
        |at| claude_credentials::read(credentials_path, at),
        client,
        schedule,
        now,
        cancelled,
    )
}

/// One poll, with the sign-in read by `read_credentials`, which is asked once, with the time
/// of the poll. [`run`] hands in the reader of the sign-in file; a test may hand in its own.
pub fn run_with(
    read_credentials: impl FnOnce(DateTime<Utc>) -> CredentialRead,
    client: &LiveUsageClient,
    schedule: &LiveSchedule,
    now: DateTime<Utc>,
    cancelled: &AtomicBool,
) -> Result<LivePollResult, Cancelled> {
    // What a poll comes to when nothing is sent: the file is read again in a minute.
    let nothing_sent = |state, outcome| LivePollResult {
        schedule: schedule.after_no_credentials(now),
        state,
        limits: None,
        outcome,
    };

    // No request without a token that is there and has not run out.
    let token = match read_credentials(now) {
        CredentialRead::Ok(token) => token,
        CredentialRead::Expired => {
            return Ok(nothing_sent(ProviderState::Expired, PollOutcome::Expired));
        }
        CredentialRead::Missing => {
            return Ok(nothing_sent(ProviderState::NoSignIn, PollOutcome::Missing));
        }
        CredentialRead::Unreadable => {
            return Ok(nothing_sent(
                ProviderState::NoSignIn,
                PollOutcome::Unreadable,
            ));
        }
    };

    // The same expiry means the same sign-in Anthropic refused: wait for Claude Code to save
    // a new one.
    if schedule.refuses(token.expires_at()) {
        return Ok(nothing_sent(ProviderState::Refused, PollOutcome::Refused));
    }

    // `?` hands a cancellation straight back to the caller.
    let result = client.fetch(&token, now, cancelled)?;

    // `after_fetch` takes every outcome but one, a refusal, which is remembered by the
    // expiry of the sign-in that was refused.
    let Ok(next) = schedule.after_fetch(&result, now) else {
        return Ok(LivePollResult {
            schedule: schedule.after_refused(token.expires_at(), now),
            state: ProviderState::Refused,
            limits: None,
            outcome: PollOutcome::Unauthorized,
        });
    };
    let (state, outcome, limits) = match result.outcome {
        LiveOutcome::Ok => (ProviderState::Ok, PollOutcome::Ok, result.limits),
        LiveOutcome::RateLimited => (ProviderState::RateLimited, PollOutcome::RateLimited, None),
        LiveOutcome::Unreachable => (ProviderState::Offline, PollOutcome::Unreachable, None),
        LiveOutcome::Failed | LiveOutcome::Unauthorized => {
            (ProviderState::BadAnswer, PollOutcome::Failed, None)
        }
    };
    Ok(LivePollResult {
        schedule: next,
        state,
        limits,
        outcome,
    })
    // `token` goes out of use here, and Rust frees it: the next poll reads the file again.
}
