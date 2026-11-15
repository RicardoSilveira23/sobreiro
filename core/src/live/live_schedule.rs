//! When the next poll for the usage is due, and what each outcome does to that.
//!
//! Nothing in this file looks at a clock, waits or starts a thread. Every function is handed
//! the time and answers at once, so a test can hand in any time it likes. The same goes for
//! what a schedule holds: moments and counts, and nothing a token could hide in.
use chrono::{DateTime, TimeDelta, Utc};

use super::{LiveOutcome, LiveResult};

/// The sign-in Anthropic refused, known by the moment it runs out. That moment is no secret.
/// `None` when the sign-in file named none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefusedSignIn {
    pub expires_at: Option<DateTime<Utc>>,
}

/// What [`LiveSchedule::after_fetch`] gives for a refused sign-in. The refusal has to be
/// remembered by the sign-in's expiry, which a [`LiveResult`] does not hold:
/// [`LiveSchedule::after_refused`] takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefusalNeedsItsExpiry;

/// When the next poll is due.
///
/// A schedule is never changed. Each thing that happens gives the next schedule, and the old
/// one stays as it was. (`Copy` in the line below says a schedule is small enough to be
/// handed on as a copy, like a number.)
///
/// - After a success the next poll is five minutes on.
/// - After a 429 the wait is the longer of two: what the answer asked for (`Retry-After`),
///   and a step that grows with each 429: 5, 10, 20, 30, 30... minutes. Only a success puts
///   the step back to the start; a failure between two 429s leaves it where it was.
/// - After a request that failed or got no answer, two minutes.
/// - With no sign-in to send, the file is read again in a minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveSchedule {
    /// From this moment on a poll is due.
    pub next_due: DateTime<Utc>,
    /// When the last poll that brought a reading was made.
    pub last_success: Option<DateTime<Utc>>,
    /// How many 429s came since the last success, counted up to four: which step of the
    /// back-off is next.
    pub rate_limit_step: u32,
    /// Set while a wait after a 429 or a failure runs. Until then, opening the flyout does
    /// not ask early.
    pub wait_until: Option<DateTime<Utc>>,
    /// The sign-in a 401 or 403 turned down. It is not sent again; the file is read every
    /// minute until it gives another one.
    pub refused: Option<RefusedSignIn>,
    /// The polls in a row that brought no reading. A success puts it back to zero.
    pub misses: u32,
}

/// Between two polls that go well. `CLAUDE.md` and the About box say "every 5 minutes".
const INTERVAL: TimeDelta = TimeDelta::minutes(5);

/// After a request that failed or got no answer.
const RETRY_INTERVAL: TimeDelta = TimeDelta::minutes(2);

/// With no sign-in to send. Only the sign-in file is read then: no request is made.
const NO_CREDENTIALS_INTERVAL: TimeDelta = TimeDelta::minutes(1);

/// Opening the flyout asks early when the last success is older than this.
const OPEN_FRESHNESS: TimeDelta = TimeDelta::minutes(1);

/// The wait after the first, second, third and every further 429 since the last success, in
/// minutes.
const STEP_MINUTES: [i64; 4] = [5, 10, 20, 30];

/// The longest wait: the last of [`STEP_MINUTES`].
const LAST_STEP_MINUTES: i64 = 30;

impl LiveSchedule {
    /// The schedule the app starts with: a poll is due at once.
    pub fn start(now: DateTime<Utc>) -> LiveSchedule {
        LiveSchedule {
            next_due: now,
            last_success: None,
            rate_limit_step: 0,
            wait_until: None,
            refused: None,
            misses: 0,
        }
    }

    /// Whether a poll is due at `now`.
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        now >= self.next_due
    }

    /// Whether opening the flyout at `now` asks early: when the last success is over a minute
    /// old, unless a wait after a 429 or a failure is running. So a request that keeps failing
    /// is not sent again on every open.
    pub fn fetch_on_open(&self, now: DateTime<Utc>) -> bool {
        let waiting = self.wait_until.is_some_and(|until| now < until);
        let fresh = self
            .last_success
            .is_some_and(|last| now.signed_duration_since(last) <= OPEN_FRESHNESS);
        !waiting && !fresh
    }

    /// True when the sign-in file still gives the sign-in Anthropic refused: send nothing.
    /// `expires_at` is the expiry the file gives now.
    pub fn refuses(&self, expires_at: Option<DateTime<Utc>>) -> bool {
        self.refused
            .is_some_and(|refused| refused.expires_at == expires_at)
    }

    /// The schedule after a request came to `result`. Any outcome but a refusal, which gives
    /// `Err`: see [`RefusalNeedsItsExpiry`].
    pub fn after_fetch(
        &self,
        result: &LiveResult,
        now: DateTime<Utc>,
    ) -> Result<LiveSchedule, RefusalNeedsItsExpiry> {
        match result.outcome {
            LiveOutcome::Ok => Ok(LiveSchedule {
                next_due: later(now, INTERVAL),
                last_success: Some(now),
                rate_limit_step: 0,
                wait_until: None,
                refused: None,
                misses: 0,
            }),
            LiveOutcome::Unauthorized => Err(RefusalNeedsItsExpiry),
            LiveOutcome::RateLimited => Ok(self.after_rate_limit(result.retry_after, now)),
            LiveOutcome::Failed | LiveOutcome::Unreachable => Ok(self.after_failure(now)),
        }
    }

    /// After a 429. `asked` is how long the answer asked Sobreiro to wait, when it said.
    fn after_rate_limit(&self, asked: Option<TimeDelta>, now: DateTime<Utc>) -> LiveSchedule {
        // `get` gives nothing for a step past the end of the list: the last one then.
        let minutes = STEP_MINUTES
            .get(self.rate_limit_step as usize)
            .copied()
            .unwrap_or(LAST_STEP_MINUTES);
        let step = TimeDelta::minutes(minutes);
        let wait = match asked {
            Some(asked) if asked > step => asked,
            _ => step,
        };
        let until = later(now, wait);
        // `..*self` fills in the fields not named here from this schedule.
        LiveSchedule {
            next_due: until,
            wait_until: Some(until),
            rate_limit_step: one_more(self.rate_limit_step).min(STEP_MINUTES.len() as u32),
            refused: None,
            misses: one_more(self.misses),
            ..*self
        }
    }

    /// After a request that failed or got no answer: again in two minutes, and opening the
    /// flyout does not ask sooner. The step of the back-off stays where it was.
    pub fn after_failure(&self, now: DateTime<Utc>) -> LiveSchedule {
        let until = later(now, RETRY_INTERVAL);
        LiveSchedule {
            next_due: until,
            wait_until: Some(until),
            refused: None,
            misses: one_more(self.misses),
            ..*self
        }
    }

    /// After a 401 or 403 for the sign-in with this expiry: nothing is sent until the file
    /// gives another one.
    pub fn after_refused(
        &self,
        expires_at: Option<DateTime<Utc>>,
        now: DateTime<Utc>,
    ) -> LiveSchedule {
        LiveSchedule {
            refused: Some(RefusedSignIn { expires_at }),
            ..self.after_no_credentials(now)
        }
    }

    /// With no sign-in to send (none, one that ran out, or the refused one): read the file
    /// again in a minute. Opening the flyout is free to read it sooner.
    pub fn after_no_credentials(&self, now: DateTime<Utc>) -> LiveSchedule {
        LiveSchedule {
            next_due: later(now, NO_CREDENTIALS_INTERVAL),
            wait_until: None,
            misses: one_more(self.misses),
            ..*self
        }
    }

    /// After a Copilot poll that found nothing to show (no sign-in, or none GitHub accepted):
    /// ask again in five minutes, and opening the flyout does not ask sooner.
    pub fn after_no_reading(&self, now: DateTime<Utc>) -> LiveSchedule {
        let until = later(now, INTERVAL);
        LiveSchedule {
            next_due: until,
            wait_until: Some(until),
            refused: None,
            misses: one_more(self.misses),
            ..*self
        }
    }
}

/// `now` and `wait` after it. A plain `now + wait` stops the program when the sum is past the
/// last moment the calendar holds, which no clock that is right can reach; this gives that
/// last moment instead.
fn later(now: DateTime<Utc>, wait: TimeDelta) -> DateTime<Utc> {
    now.checked_add_signed(wait)
        .unwrap_or(DateTime::<Utc>::MAX_UTC)
}

/// A count and one more. At the top of what the number can hold it stays there, where a plain
/// `count + 1` would stop the program.
fn one_more(count: u32) -> u32 {
    count.saturating_add(1)
}
