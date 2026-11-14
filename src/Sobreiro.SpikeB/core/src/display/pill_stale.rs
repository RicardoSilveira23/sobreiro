use crate::limits::{Limit, LimitKind};
use crate::polling::ProviderState;
use crate::{Span, Time};

/// Whether a pill block's numbers have stopped: it then keeps its last figures, each marked, and its ring or bar
/// turns grey. The flyout names the cause.
pub struct PillStale;

impl PillStale {
    /// Polls in a row that must bring nothing before a block is marked: one miss after a wake or a start is no reason.
    pub const MISSES_NEEDED: i32 = 2;

    pub fn claude_after() -> Span {
        Span::minutes(15)
    }

    pub fn copilot_after() -> Span {
        Span::hours(24)
    }

    /// True when the poll has stopped and the newest session or weekly figure is old. With no such figure there is
    /// nothing to mark.
    pub fn claude(state: ProviderState, limits: &[Limit], now: Time, misses: i32) -> bool {
        if misses < Self::MISSES_NEEDED || !stopped(state) {
            return false;
        }
        let newest = limits
            .iter()
            .filter(|l| matches!(l.kind, LimitKind::Session | LimitKind::Weekly))
            .map(|l| l.observed_at)
            .max();
        newest.is_some_and(|at| now - at > Self::claude_after())
    }

    pub fn copilot(state: ProviderState, credits: Option<&Limit>, now: Time, misses: i32) -> bool {
        misses >= Self::MISSES_NEEDED
            && stopped(state)
            && credits.is_some_and(|c| now - c.observed_at > Self::copilot_after())
    }
}

fn stopped(state: ProviderState) -> bool {
    use ProviderState::*;
    matches!(state, Expired | NoSignIn | Refused | Offline | BadAnswer | RateLimited)
}
