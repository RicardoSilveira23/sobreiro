//! Where a GitHub token may be looked for, and what a place gives back.
//!
//! The places are the environment, Windows Credential Manager and the GitHub CLI. Looking
//! in the last two is the shell's work, so each place reaches the poll as a function that
//! the shell wrote. The poll runs it only when it comes to that place.
use super::GitHubToken;

/// Which place a sign-in is: the poll's log names a place by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotSourceKind {
    Env,
    CredentialManager,
    Gh,
}

/// What one place found. `entry_found` is for a place that saw a sign-in entry but no token in
/// it.
#[derive(Debug)]
pub struct TokenSearch {
    pub tokens: Vec<GitHubToken>,
    pub entry_found: bool,
}

/// The place could not be searched. It has no text: the C# let the error's own words stand
/// here, and those could hold a token. Nothing but the fact travels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignInFailed;

/// One place to look for a GitHub token.
///
/// `tokens` runs only when the poll reaches this place. `Box<dyn Fn() -> ...>` is a function
/// of any kind put in a box, so that sign-ins with different functions can sit in one list;
/// `Send + Sync` says it may be run from another thread than the one that made it.
pub struct CopilotSignIn {
    pub kind: CopilotSourceKind,
    pub tokens: Box<dyn Fn() -> Result<TokenSearch, SignInFailed> + Send + Sync>,
}
