//! The one request Sobreiro makes for Claude Code's usage, and what came of it.
//!
//! `GET https://api.anthropic.com/api/oauth/usage` with Claude Code's access token. The token
//! goes into the `Authorization` header of that one request and nowhere else: not into a
//! result, not into an error, not into a log. The address is fixed here, and an answer that
//! names another one (a redirect) counts as failed and is not followed.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};

use super::{AccessToken, usage_api_json};
use crate::io::http::{Answer, NetworkSender, NoAnswer, Sender};
use crate::io::retry_after::retry_after;
use crate::io::wipe::wipe;
use crate::usage::RateLimits;

/// What a request for the usage came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveOutcome {
    /// An answer with limits in it.
    Ok,
    /// The token was refused (401 or 403).
    Unauthorized,
    /// Too many requests (429).
    RateLimited,
    /// An answer that is no use: another status, or a body with no limits in it.
    Failed,
    /// No answer: the network, or the ten seconds ran out.
    Unreachable,
}

/// The outcome with what it brought: the limits when it is [`LiveOutcome::Ok`], and how long
/// the server asked Sobreiro to wait when it is [`LiveOutcome::RateLimited`] and the answer
/// said so.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveResult {
    pub outcome: LiveOutcome,
    pub limits: Option<RateLimits>,
    pub retry_after: Option<TimeDelta>,
}

impl LiveResult {
    /// An outcome that brought nothing with it.
    fn of(outcome: LiveOutcome) -> LiveResult {
        LiveResult {
            outcome,
            limits: None,
            retry_after: None,
        }
    }
}

/// The one address the token is sent to.
pub const ENDPOINT: &str = "https://api.anthropic.com/api/oauth/usage";

/// How long a request may take, from its start to the last byte of the answer.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// The caller gave the request up. No result comes back, and an answer that had arrived by
/// then is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

/// The sender the app's client must be built with: the real one, which follows no redirect
/// (the token must not follow one) and keeps no cookies, with the ten seconds above.
pub fn network_sender() -> NetworkSender {
    NetworkSender::new(TIMEOUT)
}

/// Asks for Claude Code's usage.
pub struct LiveUsageClient {
    sender: Arc<dyn Sender>,
    user_agent: String,
}

impl LiveUsageClient {
    /// `sender` is what reaches the network: [`network_sender`] in the app, a stand-in in a
    /// test. `user_agent` is the app's name and version, sent as the `User-Agent` header.
    ///
    /// `Arc<dyn Sender>` is a sender that may be shared (`Arc` counts who holds it) and whose
    /// exact type is not fixed here (`dyn`), so the real one and a stand-in both fit.
    pub fn new(sender: Arc<dyn Sender>, user_agent: &str) -> LiveUsageClient {
        LiveUsageClient {
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
        token: &AccessToken,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<LiveResult, Cancelled> {
        if cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }

        // A token or an app name that a header cannot hold is not sent, and neither is a
        // request without a token.
        if token.value().is_empty()
            || !fits_a_header(token.value())
            || !fits_a_header(&self.user_agent)
        {
            return Ok(LiveResult::of(LiveOutcome::Failed));
        }

        // The header's value holds the token, so it stays inside this function: it is handed
        // to the sender and to nobody else, and wiped as soon as the sender is done. It is
        // built with room for all of it, so growing never leaves a copy of a part behind.
        const SCHEME: &str = "Bearer ";
        let mut authorization = String::with_capacity(SCHEME.len() + token.value().len());
        authorization.push_str(SCHEME);
        authorization.push_str(token.value());
        let headers = [
            ("Authorization", authorization.as_str()),
            ("anthropic-beta", "oauth-2025-04-20"),
            ("Accept", "application/json"),
            ("User-Agent", self.user_agent.as_str()),
        ];
        let answer = self.sender.get(ENDPOINT, &headers);
        wipe(&mut authorization.into_bytes());

        if cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }
        Ok(match answer {
            Ok(answer) => interpret(&answer, now),
            Err(NoAnswer::Unreachable) => LiveResult::of(LiveOutcome::Unreachable),
            Err(NoAnswer::Failed) => LiveResult::of(LiveOutcome::Failed),
        })
    }
}

/// Whether a header can hold `value`: letters, digits, signs and spaces of ASCII, and nothing
/// else. A line end inside a header would start a header of its own.
pub(crate) fn fits_a_header(value: &str) -> bool {
    value.bytes().all(|byte| (0x20..=0x7E).contains(&byte))
}

/// What an answer means.
fn interpret(answer: &Answer, now: DateTime<Utc>) -> LiveResult {
    match answer.status {
        401 | 403 => LiveResult::of(LiveOutcome::Unauthorized),
        429 => LiveResult {
            outcome: LiveOutcome::RateLimited,
            limits: None,
            retry_after: retry_after(answer.retry_after.as_deref(), now),
        },
        200..=299 => match usage_api_json::parse(&text_of(&answer.body)) {
            Some(limits) => LiveResult {
                outcome: LiveOutcome::Ok,
                limits: Some(limits),
                retry_after: None,
            },
            None => LiveResult::of(LiveOutcome::Failed),
        },
        // Among these is every redirect (301, 302 and the like).
        _ => LiveResult::of(LiveOutcome::Failed),
    }
}

/// The three bytes that some programs put at the start of a UTF-8 text.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// A body as text. JSON between programs is UTF-8; bytes that are not come out as the
/// "unknown character" sign, as they did in 1.x.
pub(crate) fn text_of(body: &[u8]) -> String {
    let body = body.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(body);
    String::from_utf8_lossy(body).into_owned()
}
