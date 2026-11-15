//! The GitHub CLI as a place for a token: `gh auth token --hostname github.com`.
//!
//! Core finds `gh.exe` and reads what the run printed, and nothing more: it starts no process
//! and no thread and makes no call to the OS beyond files and HTTP, so the shell runs `gh`,
//! with [`ARGUMENTS`] and [`TIMEOUT`], and hands what came of it to [`tokens_from_run`].
use std::path::PathBuf;
use std::time::Duration;

use super::GitHubToken;
use super::copilot_token;
use crate::app_paths::is_full_path;

/// What `gh` is asked, one argument each.
pub const ARGUMENTS: [&str; 4] = ["auth", "token", "--hostname", "github.com"];

/// How long `gh` has to answer. A run that takes longer is stopped and gives no token.
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// The full path of `gh.exe` in the first entry of `path` that holds one. `path` is the value
/// of the `PATH` variable, whose entries are separated by `;`. An empty entry, one that is
/// not a full path (it would depend on the folder the app runs in) and one with no `gh.exe`
/// are skipped.
pub fn find(path: Option<&str>) -> Option<PathBuf> {
    for entry in path.unwrap_or("").split(';') {
        let entry = entry.trim();
        if entry.is_empty() || !is_full_path(entry.as_ref()) {
            continue;
        }
        let candidate = PathBuf::from(entry).join("gh.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The tokens a finished run of `gh` gave. `exit_code` is how it ended, `None` when it did
/// not say. No exit code, or one that is not zero, gives none, whatever it printed. Otherwise
/// the first token in `stdout`, read as UTF-8, is the one.
///
/// The output is searched where it lies, so no second copy of it is made.
pub fn tokens_from_run(exit_code: Option<i32>, stdout: &[u8]) -> Vec<GitHubToken> {
    if exit_code != Some(0) {
        return Vec::new();
    }
    copilot_token::find_in_utf8(stdout).into_iter().collect()
}
