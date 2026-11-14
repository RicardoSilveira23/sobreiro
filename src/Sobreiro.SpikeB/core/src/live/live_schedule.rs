use super::LiveResult;
use super::LiveOutcome;
use crate::{Span, Time};

/// The sign-in Anthropic refused, known by its expiry (not secret; None when the file had none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefusedSignIn {
    pub expires_at: Option<Time>,
}

/// `AfterFetch` was given a refusal, which needs the token's expiry (`after_refused`): the C# `ArgumentException`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefusedGoesThroughAfterRefused;

/// When the next Live usage fetch is due. Immutable: each event gives the next schedule. After a 429 the wait is
/// max(Retry-After, step) with step 5, 10, 20, 30, 30... minutes; a success resets the step. `wait_until` is set
/// while a wait after a 429 or a failure runs; opening the flyout does not cut it short. `refused` holds the sign-in
/// a 401 or 403 turned down: it is not sent again, and the file is read every minute until it gives a new one.
/// `misses` counts the polls in a row that brought no reading; a success resets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiveSchedule {
    pub next_due: Time,
    pub last_success: Option<Time>,
    pub rate_limit_step: i32,
    pub wait_until: Option<Time>,
    pub refused: Option<RefusedSignIn>,
    pub misses: i32,
}

const STEP_MINUTES: [i64; 4] = [5, 10, 20, 30];

fn interval() -> Span {
    Span::minutes(5)
}
fn retry_interval() -> Span {
    Span::minutes(2)
}
fn no_credentials_interval() -> Span {
    Span::minutes(1)
}
fn open_freshness() -> Span {
    Span::minutes(1)
}

impl LiveSchedule {
    pub fn start(now: Time) -> LiveSchedule {
        LiveSchedule { next_due: now, last_success: None, rate_limit_step: 0, wait_until: None, refused: None, misses: 0 }
    }

    pub fn is_due(&self, now: Time) -> bool {
        now >= self.next_due
    }

    /// Opening the flyout fetches early when the last success is over a minute old, unless a wait after a 429 or a
    /// failure is running (so a failing request is not resent on every open).
    pub fn fetch_on_open(&self, now: Time) -> bool {
        !self.wait_until.is_some_and(|w| now < w) && self.last_success.is_none_or(|last| now - last > open_freshness())
    }

    /// True when the file still gives the sign-in Anthropic refused: send nothing.
    pub fn refuses(&self, expires_at: Option<Time>) -> bool {
        self.refused.is_some_and(|r| r.expires_at == expires_at)
    }

    /// Any outcome but a refusal, which needs the token's expiry: `after_refused`.
    pub fn after_fetch(&self, result: &LiveResult, now: Time) -> Result<LiveSchedule, RefusedGoesThroughAfterRefused> {
        match result.outcome {
            LiveOutcome::Ok => Ok(LiveSchedule { next_due: now + interval(), last_success: Some(now), rate_limit_step: 0, wait_until: None, refused: None, misses: 0 }),
            LiveOutcome::Unauthorized => Err(RefusedGoesThroughAfterRefused),
            LiveOutcome::RateLimited => {
                let step = Span::minutes(STEP_MINUTES[(self.rate_limit_step as usize).min(STEP_MINUTES.len() - 1)]);
                let wait = match result.retry_after {
                    Some(after) if after > step => after,
                    _ => step,
                };
                Ok(LiveSchedule {
                    next_due: now + wait,
                    wait_until: Some(now + wait),
                    rate_limit_step: (self.rate_limit_step + 1).min(STEP_MINUTES.len() as i32),
                    refused: None,
                    misses: self.misses + 1,
                    ..*self
                })
            }
            _ => Ok(LiveSchedule {
                next_due: now + retry_interval(),
                wait_until: Some(now + retry_interval()),
                refused: None,
                misses: self.misses + 1,
                ..*self
            }),
        }
    }

    /// A 401 or 403 for the token with this expiry: nothing is sent until the file gives another one.
    pub fn after_refused(&self, expires_at: Option<Time>, now: Time) -> LiveSchedule {
        LiveSchedule { refused: Some(RefusedSignIn { expires_at }), ..self.after_no_credentials(now) }
    }

    /// No sign-in to send (none, expired, or the refused one): read the file again in a minute.
    pub fn after_no_credentials(&self, now: Time) -> LiveSchedule {
        LiveSchedule { next_due: now + no_credentials_interval(), wait_until: None, misses: self.misses + 1, ..*self }
    }

    /// A Copilot poll that found nothing to show (no sign-in, or none GitHub accepted): ask again in five minutes,
    /// and opening the flyout does not ask sooner.
    pub fn after_no_reading(&self, now: Time) -> LiveSchedule {
        LiveSchedule { next_due: now + interval(), wait_until: Some(now + interval()), refused: None, misses: self.misses + 1, ..*self }
    }
}
