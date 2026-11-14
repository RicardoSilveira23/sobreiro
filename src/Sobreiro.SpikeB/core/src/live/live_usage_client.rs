use super::{AccessToken, UsageApiJson};
use crate::http::{self, Failure};
use crate::usage::RateLimits;
use crate::{Span, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiveOutcome {
    Ok,
    Unauthorized,
    RateLimited,
    Failed,
    Unreachable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveResult {
    pub outcome: LiveOutcome,
    pub limits: Option<RateLimits>,
    pub retry_after: Option<Span>,
}

impl LiveResult {
    pub fn of(outcome: LiveOutcome) -> LiveResult {
        LiveResult { outcome, limits: None, retry_after: None }
    }
}

/// The app's one network call: GET /api/oauth/usage with Claude Code's access token. Redirects are never followed
/// (a 3xx is Failed). The token goes only into the Authorization header, never into a result or an error. The
/// client is cheap to clone and is `Send + Sync`: clone it into the poll that runs on a worker thread.
#[derive(Clone)]
pub struct LiveUsageClient {
    agent: ureq::Agent,
    user_agent: String,
}

impl LiveUsageClient {
    pub const ENDPOINT: &'static str = "https://api.anthropic.com/api/oauth/usage";

    pub fn timeout() -> Span {
        Span::seconds(10)
    }

    pub fn new(user_agent: &str) -> LiveUsageClient {
        LiveUsageClient { agent: http::agent(Self::timeout().to_std().unwrap()), user_agent: user_agent.to_string() }
    }

    /// Blocks up to the timeout. Never returns or logs the token.
    pub fn fetch(&self, token: &AccessToken, now: Time) -> LiveResult {
        let auth = format!("Bearer {}", token.value());
        let headers = [
            ("Authorization", auth.as_str()),
            ("anthropic-beta", "oauth-2025-04-20"),
            ("Accept", "application/json"),
            ("User-Agent", self.user_agent.as_str()),
        ];
        match http::get(&self.agent, Self::ENDPOINT, &headers) {
            Ok(answer) => Self::interpret(answer.status, answer.retry_after.as_deref(), answer.body.as_deref(), now),
            Err(Failure::Unreachable) => LiveResult::of(LiveOutcome::Unreachable),
            Err(Failure::Failed) => LiveResult::of(LiveOutcome::Failed),
        }
    }

    /// What a status, a Retry-After header and a body mean. Pure, so tests need no server.
    pub fn interpret(status: u16, retry_after: Option<&str>, body: Option<&str>, now: Time) -> LiveResult {
        match status {
            401 | 403 => return LiveResult::of(LiveOutcome::Unauthorized),
            429 => return LiveResult { outcome: LiveOutcome::RateLimited, limits: None, retry_after: Self::retry_after(retry_after, now) },
            _ => {}
        }
        if !(200..300).contains(&status) {
            return LiveResult::of(LiveOutcome::Failed);
        }
        match body.and_then(UsageApiJson::parse) {
            None => LiveResult::of(LiveOutcome::Failed),
            Some(limits) => LiveResult { outcome: LiveOutcome::Ok, limits: Some(limits), retry_after: None },
        }
    }

    /// A Retry-After value (seconds or an HTTP date), never negative and never over six hours.
    pub fn retry_after(value: Option<&str>, now: Time) -> Option<Span> {
        http::retry_after(value, now)
    }
}

