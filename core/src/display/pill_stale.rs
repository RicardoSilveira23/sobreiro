//! Whether a pill block's numbers have stopped: it then keeps its last figures, each marked, and
//! its ring or bar turns grey. The flyout names the cause.
use chrono::{DateTime, TimeDelta, Utc};

use crate::limits::{Limit, LimitKind};
use crate::polling::provider_state::ProviderState;

/// Polls in a row that must bring nothing before a block is marked: one miss after a wake or a
/// start is no reason.
pub const MISSES_NEEDED: i32 = 2;

pub const CLAUDE_AFTER: TimeDelta = TimeDelta::minutes(15);
pub const COPILOT_AFTER: TimeDelta = TimeDelta::hours(24);

/// True when the poll has stopped and the newest session or weekly figure is old. With no such
/// figure there is nothing to mark.
pub fn claude(state: ProviderState, limits: &[Limit], now: DateTime<Utc>, misses: i32) -> bool {
    if misses < MISSES_NEEDED || !stopped(state) {
        return false;
    }
    let newest = limits
        .iter()
        .filter(|limit| matches!(limit.kind, LimitKind::Session | LimitKind::Weekly))
        .map(|limit| limit.observed_at)
        .max();
    newest.is_some_and(|at| now.signed_duration_since(at) > CLAUDE_AFTER)
}

/// True when the poll has stopped and the credits figure is old. With no figure there is nothing
/// to mark.
pub fn copilot(
    state: ProviderState,
    credits: Option<&Limit>,
    now: DateTime<Utc>,
    misses: i32,
) -> bool {
    misses >= MISSES_NEEDED
        && stopped(state)
        && credits
            .is_some_and(|credits| now.signed_duration_since(credits.observed_at) > COPILOT_AFTER)
}

fn stopped(state: ProviderState) -> bool {
    matches!(
        state,
        ProviderState::Expired
            | ProviderState::NoSignIn
            | ProviderState::Refused
            | ProviderState::Offline
            | ProviderState::BadAnswer
            | ProviderState::RateLimited
    )
}
