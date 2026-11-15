//! How the Copilot CLI's sign-in is looked for in Windows Credential Manager: the filter to ask
//! with, which target names are its entry, and what a failed listing means. It is the only
//! entry Sobreiro reads there.
//!
//! This is the judging only. Asking Credential Manager is the shell's work: Core makes no
//! call to Windows beyond files and HTTP.
use crate::app_paths::capital;

/// The name the CLI gives its entry.
const NAME: &str = "copilot-cli";

/// ERROR_NOT_FOUND: no entry matches the filter.
const ERROR_NOT_FOUND: i32 = 1168;

/// What Credential Manager is asked for: target names that start with the entry's name.
pub const FILTER: &str = "copilot-cli*";

/// What to do when Credential Manager answers a listing with an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingAnswer {
    /// There is no such entry.
    NoEntry,
    /// List every entry and keep the ones that are ours.
    ListEverything,
    /// The listing itself failed: not the same as finding nothing.
    Failed,
}

/// True when the target name is the CLI's entry: it has the entry's name anywhere, in any
/// case, because older versions of the CLI may put text before it. No name is not ours.
pub fn is_ours(target_name: Option<&str>) -> bool {
    let Some(target_name) = target_name else {
        return false;
    };
    // Letters are compared by their capitals, as the rest of Core does where 1.x ignored case.
    let ours: Vec<char> = NAME.chars().map(capital).collect();
    let theirs: Vec<char> = target_name.chars().map(capital).collect();
    theirs.windows(ours.len()).any(|window| window == ours)
}

/// The answer to a failed listing by filter. `win32_error` is the error Windows gave.
/// Every entry is listed only when nothing matched the filter, and only on a PC where the
/// Copilot CLI has left its folder.
pub fn after_failure(win32_error: i32, cli_has_been_used: bool) -> ListingAnswer {
    if win32_error != ERROR_NOT_FOUND {
        ListingAnswer::Failed
    } else if cli_has_been_used {
        ListingAnswer::ListEverything
    } else {
        ListingAnswer::NoEntry
    }
}

/// The answer to a failed listing of every entry: an empty Credential Manager is no entry,
/// anything else a failure.
pub fn after_listing_everything_failed(win32_error: i32) -> ListingAnswer {
    if win32_error == ERROR_NOT_FOUND {
        ListingAnswer::NoEntry
    } else {
        ListingAnswer::Failed
    }
}
