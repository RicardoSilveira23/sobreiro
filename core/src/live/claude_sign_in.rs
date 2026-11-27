//! Sign in to and sign out of Claude Code, from the menu: the way back when Live usage stopped for
//! want of a sign-in, and the way to change accounts.
//!
//! Sobreiro never renews, writes or removes a sign-in itself. To sign in it starts Claude Code in
//! a console no one sees: `claude`, which renews a sign-in that ran out when it starts, or
//! `claude auth login`, which opens the browser by itself, when there is no sign-in or Anthropic
//! refused it. It watches the
//! sign-in file's time of change (the file's contents are not read here) and the console's window
//! title, which `cmd` sets to [`ENDED_TITLE`] when Claude Code has ended.
//!
//! When Claude Code writes the file, the console is closed, shown or not, and Live usage asks at
//! once. The console is shown, as it stands with Claude Code's own words in it, only when Claude
//! Code needs the user: it ended without signing in (it says why), or it has not signed in within
//! its quiet wait ([`QUIET_WAIT`] for `claude`, which may ask whether a folder is trusted;
//! [`LOGIN_QUIET_WAIT`] for a login, whose browser may not have opened, or which may want a code).
//!
//! To sign out it runs `claude auth logout`, out of sight, once the user said yes; Claude Code
//! removes its own sign-in, and the menu offers Sign in again.
//!
//! Core finds `claude` and decides each step, and nothing more: it starts no process and shows
//! no window. The shell runs `cmd.exe` with [`cmd_arguments`], so a sign-in's console stays open
//! with Claude Code's message when Claude Code ends, and hands the watch what it sees.
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::app_paths::is_full_path;
use crate::polling::ProviderState;

/// The menu items' texts.
pub const SIGN_IN_TITLE: &str = "Sign in to Claude Code";
pub const SIGN_OUT_TITLE: &str = "Sign out of Claude Code";

/// What Sign out asks before it runs.
pub const SIGN_OUT_QUESTION: &str = "Sign Claude Code out on this PC?\n\nClaude Code in every \
terminal and editor on this PC will need to sign in again. Then choose Sign in to Claude Code in \
this menu to sign in, with this account or another.";

/// How long `claude auth logout` has to end. One that takes longer is stopped.
pub const SIGN_OUT_TIMEOUT: Duration = Duration::from_secs(20);

/// How long `claude` has to renew its sign-in by itself before its console is shown.
pub const QUIET_WAIT: Duration = Duration::from_secs(20);

/// How long a login has, in the browser it opened, before its console is shown.
pub const LOGIN_QUIET_WAIT: Duration = Duration::from_secs(60);

/// The console's window title once Claude Code has ended: `cmd` sets it then. Plain words, with
/// nothing `cmd` would read as a command or a variable.
pub const ENDED_TITLE: &str = "Sobreiro - Claude Code has ended";

/// How long a shown console is watched for the user to finish signing in. After that Sobreiro
/// stops watching and leaves the console to the user; the next poll sees what came of it.
pub const WATCH_LIMIT: Duration = Duration::from_secs(15 * 60);

/// How often the sign-in file's time of change is looked at.
pub const CHECK_EVERY: Duration = Duration::from_millis(500);

/// What the menu offers for Claude Code's account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    SignIn,
    SignOut,
}

/// What the menu offers, by where Live usage stands: Sign in when it stopped for a sign-in that
/// ran out, that Anthropic refused or that is not there; Sign out otherwise. Nothing while Claude
/// Code is not shown (a PC with Copilot alone never sees either) or Live usage is off.
pub fn offer(state: ProviderState, show_claude: bool) -> Option<Offer> {
    match state {
        _ if !show_claude => None,
        ProviderState::Off => None,
        ProviderState::Expired | ProviderState::NoSignIn | ProviderState::Refused => {
            Some(Offer::SignIn)
        }
        ProviderState::Ok
        | ProviderState::RateLimited
        | ProviderState::Offline
        | ProviderState::BadAnswer => Some(Offer::SignOut),
    }
}

/// What Claude Code is asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeCommand {
    /// `claude`: it starts, and renews a sign-in that ran out.
    Start,
    /// `claude auth login`: a new sign-in, in the browser.
    Login,
    /// `claude auth logout`.
    Logout,
}

impl ClaudeCommand {
    /// The command for signing in from `state`: `claude` renews a sign-in that ran out; one that
    /// is missing, or that Anthropic refused while it had not run out (so `claude` would see no
    /// reason to renew it), takes a new sign-in.
    pub fn sign_in(state: ProviderState) -> ClaudeCommand {
        if state == ProviderState::Expired {
            ClaudeCommand::Start
        } else {
            ClaudeCommand::Login
        }
    }

    /// What follows `claude` on its command line.
    fn arguments(self) -> &'static str {
        match self {
            ClaudeCommand::Start => "",
            ClaudeCommand::Login => " auth login",
            ClaudeCommand::Logout => " auth logout",
        }
    }
}

/// The names Claude Code's command has, in the order they are looked for in each folder: the
/// native installer's exe, then npm's script.
const NAMES: [&str; 2] = ["claude.exe", "claude.cmd"];

/// Where Claude Code's native installer puts it, under the user's own folder.
const NATIVE_FOLDER: [&str; 2] = [".local", "bin"];

/// The full path of Claude Code's command: in the first entry of `path` that holds one, else in
/// the native installer's folder under `user_profile` (a PATH the installer changed after
/// Sobreiro started is not in Sobreiro's own). `path` is the value of the `PATH` variable,
/// whose entries are separated by `;`.
///
/// An empty entry and one that is not a full path (it would depend on the folder the app runs
/// in) are skipped, and so is a path [`cmd_arguments`] cannot pass.
pub fn find(path: Option<&str>, user_profile: &Path) -> Option<PathBuf> {
    let entries = path
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty() && is_full_path(entry.as_ref()))
        .map(PathBuf::from);
    let native = NATIVE_FOLDER
        .iter()
        .fold(user_profile.to_path_buf(), |folder, part| folder.join(part));
    entries
        .chain(std::iter::once(native).filter(|_| is_full_path(user_profile.as_os_str())))
        .flat_map(|folder| NAMES.map(|name| folder.join(name)))
        .find(|candidate| {
            candidate.is_file() && cmd_arguments(candidate, ClaudeCommand::Start).is_some()
        })
}

/// What `cmd.exe` is given to run `claude` with `command`: `/d` skips the AutoRun commands in
/// the registry, `/v:off` keeps `!` as it is, and `/s` with the outer quotes keeps the quoted path
/// whole whatever it holds (`cmd` takes the outer pair off, and the inner pair stays). A sign-in
/// stays open (`/k`) when Claude Code ends, so its last words can be read, and sets the window
/// title to [`ENDED_TITLE`] then; a sign-out ends (`/c`).
///
/// `None` for a path with a quote, which would end the quotes early, or a `%`, which `cmd`
/// would read as a variable. A path that is not a full path is refused too.
pub fn cmd_arguments(claude: &Path, command: ClaudeCommand) -> Option<String> {
    let text = claude.to_str()?;
    if text.contains(['"', '%']) || !is_full_path(claude.as_os_str()) {
        return None;
    }
    let arguments = command.arguments();
    Some(if command == ClaudeCommand::Logout {
        format!("/d /v:off /s /c \"\"{text}\"{arguments}\"")
    } else {
        format!("/d /v:off /s /k \"\"{text}\"{arguments} & title {ENDED_TITLE}\"")
    })
}

/// What the shell does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchStep {
    /// Nothing yet: look again after [`CHECK_EVERY`].
    Wait,
    /// Claude Code wrote its sign-in file: the console is closed with everything in it, shown or
    /// not, and Live usage asks at once.
    Renewed,
    /// Show the console: Claude Code needs the user.
    Show,
    /// Stop watching: the user closed the console, or had [`WATCH_LIMIT`] and did not sign in.
    /// A console still open is left to the user. `quiet` says it ended before it was ever shown.
    Ended { quiet: bool },
}

/// What the shell saw at one look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// How long ago Claude Code was started.
    pub elapsed: Duration,
    /// The sign-in file's time of change, `None` when there is no file.
    pub changed_at: Option<SystemTime>,
    /// The console is still open.
    pub running: bool,
    /// Claude Code has ended in it (its title is [`ENDED_TITLE`]).
    pub claude_ended: bool,
}

/// One run's watch on the sign-in file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignInWatch {
    before: Option<SystemTime>,
    /// How long the console stays hidden while Claude Code is still at work.
    quiet_wait: Duration,
    shown: bool,
}

impl SignInWatch {
    /// A watch on a file whose time of change was `before` when Claude Code was started with
    /// `command` (`Start` or `Login`); `before` is `None` when there was no file.
    pub fn new(before: Option<SystemTime>, command: ClaudeCommand) -> SignInWatch {
        SignInWatch {
            before,
            quiet_wait: if command == ClaudeCommand::Login {
                LOGIN_QUIET_WAIT
            } else {
                QUIET_WAIT
            },
            shown: false,
        }
    }

    /// Whether the console has been shown.
    pub fn shown(&self) -> bool {
        self.shown
    }

    /// The next step, from what the shell saw.
    pub fn step(&mut self, seen: Seen) -> WatchStep {
        if seen.changed_at.is_some() && seen.changed_at != self.before {
            return WatchStep::Renewed;
        }
        if !seen.running {
            return WatchStep::Ended { quiet: !self.shown };
        }
        if !self.shown && (seen.claude_ended || seen.elapsed >= self.quiet_wait) {
            self.shown = true;
            return WatchStep::Show;
        }
        if self.shown && seen.elapsed >= WATCH_LIMIT {
            return WatchStep::Ended { quiet: false };
        }
        WatchStep::Wait
    }
}
