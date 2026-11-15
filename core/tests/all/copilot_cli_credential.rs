//! Ported from `tests/Sobreiro.Tests/CopilotCliCredentialTests.cs`. The tests after
//! `a_failed_listing_of_everything_is_judged_the_same_way` are new.
//!
//! Nothing here asks Windows: the entry names and the error numbers are made up, or are the
//! two the C# tests used. A C# `[Theory]` became one test that walks the same rows.
use sobreiro_core::copilot::copilot_cli_credential::{
    self, ListingAnswer, after_failure, after_listing_everything_failed, is_ours,
};

const NOT_FOUND: i32 = 1168;
const NO_SUCH_LOGON_SESSION: i32 = 1312;

#[test]
fn the_filter_asks_for_the_cli_entry_by_name() {
    assert_eq!(copilot_cli_credential::FILTER, "copilot-cli*");
}

#[test]
fn the_entry_has_its_name_anywhere() {
    for name in [
        "copilot-cli/https://github.com:someone",
        "Legacy copilot-cli token",
        "COPILOT-CLI",
    ] {
        assert!(is_ours(Some(name)), "{name}");
    }
}

// The second is a key another Copilot product keeps there, not a sign-in: it must never be read.
#[test]
fn other_entries_are_not_ours() {
    for name in [
        Some("git:https://github.com"),
        Some("copilot-language-server/oauth-token-key"),
        Some("copilot"),
        Some(""),
        None,
    ] {
        assert!(!is_ours(name), "{name:?}");
    }
}

#[test]
fn not_found_is_no_entry_where_the_cli_has_not_been_used() {
    assert_eq!(after_failure(NOT_FOUND, false), ListingAnswer::NoEntry);
}

#[test]
fn not_found_lists_everything_only_where_the_cli_has_been_used() {
    assert_eq!(
        after_failure(NOT_FOUND, true),
        ListingAnswer::ListEverything
    );
}

#[test]
fn any_other_error_is_a_failure_not_an_empty_answer() {
    for cli_has_been_used in [true, false] {
        assert_eq!(
            after_failure(NO_SUCH_LOGON_SESSION, cli_has_been_used),
            ListingAnswer::Failed
        );
    }
}

#[test]
fn a_failed_listing_of_everything_is_judged_the_same_way() {
    assert_eq!(
        after_listing_everything_failed(NOT_FOUND),
        ListingAnswer::NoEntry
    );
    assert_eq!(
        after_listing_everything_failed(NO_SUCH_LOGON_SESSION),
        ListingAnswer::Failed
    );
}

#[test]
fn a_name_shorter_than_the_entrys_is_not_ours_and_does_not_fail() {
    for name in ["c", "copilot-cl", "copilot-cl\u{e9}"] {
        assert!(!is_ours(Some(name)), "{name}");
    }
}

#[test]
fn an_error_of_zero_is_a_failure_too() {
    assert_eq!(after_failure(0, true), ListingAnswer::Failed);
    assert_eq!(after_listing_everything_failed(0), ListingAnswer::Failed);
}
