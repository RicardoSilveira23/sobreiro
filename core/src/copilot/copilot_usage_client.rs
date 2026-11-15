//! The one request Sobreiro makes for Copilot's AI credits, and what came of it.
//!
//! `GET https://api.github.com/copilot_internal/user` with one GitHub token. The token goes
//! into the `Authorization` header of that one request and nowhere else: not into a result,
//! not into an error, not into a log. The address is fixed here, and an answer that names
//! another one (a redirect) counts as failed and is not followed.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, TimeDelta, Utc};

use super::copilot_user_json::{self, Reply};
use super::{CopilotReading, GitHubToken};
use crate::io::http::{Answer, NoAnswer, Sender};
use crate::io::retry_after::retry_after;
use crate::live::live_usage_client::{Cancelled, fits_a_header, text_of};

/// What a request for the AI credits came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotOutcome {
    /// An answer with a budget in it.
    Ok,
    /// An answer from an account with no budget to show: a free plan, or one without a limit.
    NoBudget,
    /// The token was refused (401 or 403), or GitHub knows no Copilot for it (404).
    Refused,
    /// Too many requests (429).
    RateLimited,
    /// An answer that is no use: another status, or a body that is not a reply.
    Failed,
    /// No answer: the network, or the ten seconds ran out.
    Unreachable,
}

/// The outcome with what it brought: the reading when it is [`CopilotOutcome::Ok`], and how
/// long the server asked Sobreiro to wait when it is [`CopilotOutcome::RateLimited`] and the
/// answer said so.
#[derive(Debug, Clone, PartialEq)]
pub struct CopilotFetch {
    pub outcome: CopilotOutcome,
    pub reading: Option<CopilotReading>,
    pub retry_after: Option<TimeDelta>,
}

impl CopilotFetch {
    /// An outcome that brought nothing with it.
    fn of(outcome: CopilotOutcome) -> CopilotFetch {
        CopilotFetch {
            outcome,
            reading: None,
            retry_after: None,
        }
    }
}

/// The one address a GitHub token is sent to.
pub const ENDPOINT: &str = "https://api.github.com/copilot_internal/user";

/// Asks for Copilot's AI credits.
pub struct CopilotUsageClient {
    sender: Arc<dyn Sender>,
    user_agent: String,
}

impl CopilotUsageClient {
    /// `sender` is what reaches the network: in the app the one that
    /// [`network_sender`](crate::live::live_usage_client::network_sender) makes, which follows
    /// no redirect and gives up after ten seconds, and in a test a stand-in. `user_agent` is
    /// the app's name and version, sent as the `User-Agent` header.
    pub fn new(sender: Arc<dyn Sender>, user_agent: &str) -> CopilotUsageClient {
        CopilotUsageClient {
            sender,
            user_agent: user_agent.to_string(),
        }
    }

    /// Makes the request and waits for its answer, ten seconds at most with the real sender.
    ///
    /// `cancelled` is a flag the caller may set from another thread. Set before the call, it
    /// stops the request from being sent. A request that is already under way cannot be
    /// stopped: it ends by itself within its ten seconds, and if the flag was set meanwhile
    /// its answer is dropped. Either way `Err(Cancelled)` comes back.
    pub fn fetch(
        &self,
        token: &GitHubToken,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<CopilotFetch, Cancelled> {
        if cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }

        // A token or an app name that a header cannot hold is not sent, and neither is a
        // request without a token.
        if token.value().is_empty()
            || !fits_a_header(token.value())
            || !fits_a_header(&self.user_agent)
        {
            return Ok(CopilotFetch::of(CopilotOutcome::Failed));
        }

        // The header's value holds the token, so it stays inside this function: it is handed
        // to the sender and to nobody else.
        let authorization = format!("Bearer {}", token.value());
        let headers = [
            ("Authorization", authorization.as_str()),
            ("Accept", "application/json"),
            ("X-GitHub-Api-Version", "2025-04-01"),
            ("User-Agent", self.user_agent.as_str()),
        ];
        let answer = self.sender.get(ENDPOINT, &headers);

        if cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }
        Ok(match answer {
            Ok(answer) => interpret(&answer, now),
            Err(NoAnswer::Unreachable) => CopilotFetch::of(CopilotOutcome::Unreachable),
            Err(NoAnswer::Failed) => CopilotFetch::of(CopilotOutcome::Failed),
        })
    }
}

/// What an answer means.
fn interpret(answer: &Answer, now: DateTime<Utc>) -> CopilotFetch {
    match answer.status {
        401 | 403 | 404 => CopilotFetch::of(CopilotOutcome::Refused),
        429 => CopilotFetch {
            outcome: CopilotOutcome::RateLimited,
            reading: None,
            retry_after: retry_after(answer.retry_after.as_deref(), now),
        },
        200..=299 => match copilot_user_json::parse(&text_of(&answer.body), now) {
            Reply::Reading(reading) => CopilotFetch {
                outcome: CopilotOutcome::Ok,
                reading: Some(reading),
                retry_after: None,
            },
            Reply::NoBudget => CopilotFetch::of(CopilotOutcome::NoBudget),
            Reply::NoUse => CopilotFetch::of(CopilotOutcome::Failed),
        },
        // Among these is every redirect (301, 302 and the like).
        _ => CopilotFetch::of(CopilotOutcome::Failed),
    }
}
