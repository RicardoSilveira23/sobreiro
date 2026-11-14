use super::{CopilotReading, CopilotUserJson};
use crate::http::{self, Failure};
use crate::live::LiveUsageClient;
use crate::secret::Secret;
use crate::{Span, Time};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopilotOutcome {
    Ok,
    NoBudget,
    Refused,
    RateLimited,
    Failed,
    Unreachable,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CopilotFetch {
    pub outcome: CopilotOutcome,
    pub reading: Option<CopilotReading>,
    pub retry_after: Option<Span>,
}

impl CopilotFetch {
    pub fn of(outcome: CopilotOutcome) -> CopilotFetch {
        CopilotFetch { outcome, reading: None, retry_after: None }
    }
}

/// GET https://api.github.com/copilot_internal/user with one GitHub token. Redirects are never followed (a 3xx is
/// Failed). The token goes only into the Authorization header, never into a result or an error. Cheap to clone;
/// `Send + Sync`.
#[derive(Clone)]
pub struct CopilotUsageClient {
    agent: ureq::Agent,
    user_agent: String,
}

impl CopilotUsageClient {
    pub const ENDPOINT: &'static str = "https://api.github.com/copilot_internal/user";

    pub fn new(user_agent: &str) -> CopilotUsageClient {
        CopilotUsageClient {
            agent: http::agent(LiveUsageClient::timeout().to_std().unwrap()),
            user_agent: user_agent.to_string(),
        }
    }

    /// Blocks up to the timeout. Never returns or logs the token.
    pub fn fetch(&self, token: &Secret, now: Time) -> CopilotFetch {
        let auth = format!("Bearer {}", token.expose());
        let headers = [
            ("Authorization", auth.as_str()),
            ("Accept", "application/json"),
            ("X-GitHub-Api-Version", "2025-04-01"),
            ("User-Agent", self.user_agent.as_str()),
        ];
        match http::get(&self.agent, Self::ENDPOINT, &headers) {
            Ok(answer) => Self::interpret(answer.status, answer.retry_after.as_deref(), answer.body.as_deref(), now),
            Err(Failure::Unreachable) => CopilotFetch::of(CopilotOutcome::Unreachable),
            Err(Failure::Failed) => CopilotFetch::of(CopilotOutcome::Failed),
        }
    }

    /// What a status, a Retry-After header and a body mean. Pure, so tests need no server.
    pub fn interpret(status: u16, retry_after: Option<&str>, body: Option<&str>, now: Time) -> CopilotFetch {
        match status {
            401 | 403 | 404 => return CopilotFetch::of(CopilotOutcome::Refused),
            429 => {
                return CopilotFetch {
                    outcome: CopilotOutcome::RateLimited,
                    reading: None,
                    retry_after: LiveUsageClient::retry_after(retry_after, now),
                };
            }
            _ => {}
        }
        if !(200..300).contains(&status) {
            return CopilotFetch::of(CopilotOutcome::Failed);
        }
        let body = body.unwrap_or("");
        if let Some(reading) = CopilotUserJson::parse(body, now) {
            return CopilotFetch { outcome: CopilotOutcome::Ok, reading: Some(reading), retry_after: None };
        }
        CopilotFetch::of(if is_json_object(body) { CopilotOutcome::NoBudget } else { CopilotOutcome::Failed })
    }
}

fn is_json_object(body: &str) -> bool {
    matches!(serde_json::from_str::<serde_json::Value>(body), Ok(serde_json::Value::Object(_)))
}
