//! The sentence the flyout shows when Live usage is on and not working.
use crate::polling::ProviderState;

/// The sentence at the foot of the flyout for `state`. `None` for `Off` and `Ok`, which need
/// none.
pub fn footer(state: ProviderState) -> Option<&'static str> {
    match state {
        ProviderState::Off | ProviderState::Ok => None,
        ProviderState::Expired => Some(
            "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.",
        ),
        ProviderState::NoSignIn => {
            Some("Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.")
        }
        ProviderState::Refused => Some(
            "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.",
        ),
        ProviderState::RateLimited => {
            Some("Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.")
        }
        ProviderState::Offline => Some(
            "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.",
        ),
        ProviderState::BadAnswer => Some(
            "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
    }
}
