/// What to do when Credential Manager answers a listing with an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListingAnswer {
    /// There is no such entry.
    NoEntry,
    /// List every entry and keep the ones that are ours.
    ListEverything,
    /// The listing itself failed: not the same as finding nothing.
    Failed,
}

/// How the Copilot CLI's sign-in is looked for in Windows Credential Manager: the filter to ask with, which target
/// names are its entry, and what a failed listing means. It is the only entry Sobreiro reads there.
pub struct CopilotCliCredential;

const NAME: &str = "copilot-cli";

/// ERROR_NOT_FOUND: no entry matches the filter.
const ERROR_NOT_FOUND: i32 = 1168;

impl CopilotCliCredential {
    /// What Credential Manager is asked for: target names that start with the entry's name.
    pub const FILTER: &'static str = "copilot-cli*";

    /// The entry has its name anywhere in the target name: older versions of the CLI may put text before it.
    pub fn is_ours(target_name: Option<&str>) -> bool {
        target_name.is_some_and(|t| t.to_lowercase().contains(NAME))
    }

    /// The answer to a failed listing by filter. Every entry is listed only when nothing matched the filter, and
    /// only on a PC where the Copilot CLI has left its folder.
    pub fn after_failure(win32_error: i32, cli_has_been_used: bool) -> ListingAnswer {
        if win32_error != ERROR_NOT_FOUND {
            ListingAnswer::Failed
        } else if cli_has_been_used {
            ListingAnswer::ListEverything
        } else {
            ListingAnswer::NoEntry
        }
    }

    /// The answer to a failed listing of every entry: an empty Credential Manager is no entry, anything else a failure.
    pub fn after_listing_everything_failed(win32_error: i32) -> ListingAnswer {
        if win32_error == ERROR_NOT_FOUND { ListingAnswer::NoEntry } else { ListingAnswer::Failed }
    }
}
