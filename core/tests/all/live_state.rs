//! Ported from `tests/Sobreiro.Tests/LiveStateTests.cs`.
use sobreiro_core::live::{live_state, live_usage_message};
use sobreiro_core::polling::ProviderState;

#[test]
fn each_problem_has_a_sentence() {
    let rows = [
        (
            ProviderState::Expired,
            "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it.",
        ),
        (
            ProviderState::NoSignIn,
            "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in.",
        ),
        (
            ProviderState::Refused,
            "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one.",
        ),
        (
            ProviderState::RateLimited,
            "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself.",
        ),
        (
            ProviderState::Offline,
            "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes.",
        ),
        (
            ProviderState::BadAnswer,
            "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
    ];
    for (state, text) in rows {
        assert_eq!(live_state::footer(state), Some(text), "{state:?}");
    }
}

#[test]
fn off_and_ok_have_none() {
    for state in [ProviderState::Off, ProviderState::Ok] {
        assert_eq!(live_state::footer(state), None, "{state:?}");
    }
}

#[test]
fn the_about_line_is_fixed() {
    assert_eq!(
        live_usage_message::ABOUT_LINE,
        "It asks Anthropic for your usage every 5 minutes with Claude Code's sign-in, and makes no other network calls."
    );
}
