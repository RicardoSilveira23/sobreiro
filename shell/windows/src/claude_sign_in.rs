//! The menu's Sign in to Claude Code and Sign out of Claude Code, each on a thread of its own, one
//! at a time (`sobreiro_core::live::claude_sign_in` decides each step). Both run Claude Code in a
//! hidden console (`win::console`), in Sobreiro's data folder, so a folder Claude Code asks to
//! trust is never the user's whole profile.
//!
//! Sign in runs `claude` for a sign-in that ran out, or `claude auth login` otherwise, and watches
//! Claude Code's sign-in file as Core's `SignInWatch` says. When Claude Code writes the file, its
//! console is closed with everything it started and Live usage asks at once. When it needs the
//! user (to sign in in the browser, to trust a folder, or to read an error), its console is shown
//! as it stands and the user goes on there; it is still closed once the file is written.
//!
//! Sign out asks first, then runs `claude auth logout`, never shown, and Live usage asks at once,
//! so the menu offers Sign in again.
//!
//! Sobreiro renews, writes and removes no sign-in: Claude Code does, as when the user runs it.
//! Only the file's time of change is looked at; its contents are read by Live usage alone.
use crate::diagnostics::Diagnostics;
use crate::win::console::HiddenConsole;
use crate::win::{known_folders, message_box};
use sobreiro_core::live::claude_sign_in::{
    self, CHECK_EVERY, ClaudeCommand, ENDED_TITLE, SIGN_IN_TITLE, SIGN_OUT_QUESTION,
    SIGN_OUT_TIMEOUT, SIGN_OUT_TITLE, Seen, SignInWatch, WatchStep,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Set while a sign-in or a sign-out is under way.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// How often a sign-out is looked at while it runs.
const SIGN_OUT_CHECK: Duration = Duration::from_millis(100);

/// What the box says when Claude Code is not on this PC.
const NOT_FOUND: &str = "Sobreiro couldn't find Claude Code on this PC: no claude.exe or \
claude.cmd on the PATH or in .local\\bin.\n\nInstall Claude Code, or open a terminal and run \
claude there.";

/// What the box says when Claude Code could not be started, or its console could not be shown.
const NOT_STARTED: &str = "Sobreiro couldn't start Claude Code, or show its window.\n\nOpen a \
terminal and run claude there.";

/// What the box says when a sign-in or a sign-out is asked for while one is under way.
const UNDER_WAY: &str = "Sobreiro is already signing Claude Code in or out.\n\nIf Claude Code's \
window is open, finish there or close it, then try again.";

/// What the box says when a sign-out did not end well.
const NOT_SIGNED_OUT: &str = "Claude Code didn't sign out.\n\nOpen a terminal and run \
claude auth logout there to see why.";

/// What the box says when a sign-out ended well.
const SIGNED_OUT: &str = "Claude Code is signed out on this PC.\n\nChoose Sign in to Claude Code \
in Sobreiro's menu to sign in, with this account or another.";

/// What a run needs: Claude Code's sign-in file, the user's folder (where Claude Code's installer
/// puts it), the folder Claude Code runs in, the log, and what to call, on the run's thread,
/// each time the sign-in changed.
pub struct Places {
    pub credentials_file: PathBuf,
    pub user_profile: PathBuf,
    pub working_folder: PathBuf,
    pub diagnostics: Arc<Diagnostics>,
    pub changed: Box<dyn Fn() + Send>,
}

/// Starts a sign-in with `command` (`Start` or `Login`), unless a run is under way.
pub fn sign_in(places: Places, command: ClaudeCommand) {
    spawn(places, "claude-sign-in", move |places| {
        run_sign_in(places, command);
    });
}

/// Asks, then signs Claude Code out, unless a run is under way.
pub fn sign_out(places: Places) {
    spawn(places, "claude-sign-out", run_sign_out);
}

/// Lets the next run start when it is dropped: when the run returns, and when it panics.
struct Running;

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

/// Runs `work` on a thread named `name`, unless a run is under way.
fn spawn(places: Places, name: &str, work: impl FnOnce(&Places) + Send + 'static) {
    if RUNNING.swap(true, Ordering::SeqCst) {
        places
            .diagnostics
            .say("sign-in: a sign-in or sign-out is already under way");
        // On a thread of its own, as every box of the menu's is: the event loop goes on.
        let _ = std::thread::Builder::new()
            .name("claude-under-way".to_string())
            .spawn(|| message_box::tell(SIGN_IN_TITLE, UNDER_WAY));
        return;
    }
    let diagnostics = places.diagnostics.clone();
    let started = std::thread::Builder::new()
        .name(name.to_string())
        .spawn(move || {
            let _running = Running;
            work(&places);
        });
    if started.is_err() {
        RUNNING.store(false, Ordering::SeqCst);
        diagnostics.say("sign-in: its thread could not start");
    }
}

/// The time of change of the sign-in file, or none when there is no file.
fn changed_at(file: &Path) -> Option<SystemTime> {
    std::fs::metadata(file)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// Starts Claude Code with `command` in a hidden console: `cmd.exe` by its full path in the
/// Windows folder (never by bare name, which Windows would look for beside Sobreiro first), in
/// the working folder. None, with a log line and a box, when anything is missing or fails.
fn start(places: &Places, command: ClaudeCommand) -> Option<HiddenConsole> {
    let path = std::env::var("PATH").ok();
    let Some(claude) = claude_sign_in::find(path.as_deref(), &places.user_profile) else {
        places.diagnostics.say("sign-in: Claude Code was not found");
        message_box::error(NOT_FOUND);
        return None;
    };
    // `find` gives only a path `cmd_arguments` takes.
    let console = claude_sign_in::cmd_arguments(&claude, command)
        .zip(known_folders::windows())
        .filter(|_| std::fs::create_dir_all(&places.working_folder).is_ok())
        .and_then(|(arguments, windows)| {
            let cmd = windows.join("System32").join("cmd.exe");
            HiddenConsole::start(&cmd, &arguments, &places.working_folder)
        });
    if console.is_none() {
        places
            .diagnostics
            .say("sign-in: Claude Code could not be started");
        message_box::error(NOT_STARTED);
    }
    console
}

fn run_sign_in(places: &Places, command: ClaudeCommand) {
    let diagnostics = &places.diagnostics;
    // Read before Claude Code starts, so a write it makes at once is not taken for the old file.
    let before = changed_at(&places.credentials_file);
    let Some(console) = start(places, command) else {
        return;
    };
    diagnostics.say("sign-in: Claude Code started out of sight");
    let started = Instant::now();
    let mut watch = SignInWatch::new(before, command);
    loop {
        std::thread::sleep(CHECK_EVERY);
        let running = console.is_running();
        let step = watch.step(Seen {
            elapsed: started.elapsed(),
            changed_at: changed_at(&places.credentials_file),
            running,
            claude_ended: running && console.title_has(ENDED_TITLE),
        });
        match step {
            WatchStep::Wait => {}
            WatchStep::Renewed => {
                console.end();
                diagnostics.say("sign-in: Claude Code signed in; its console was closed");
                (places.changed)();
                return;
            }
            WatchStep::Show => {
                if console.show() {
                    diagnostics.say("sign-in: Claude Code needs the user; its console was shown");
                } else {
                    // Never left running where no one can see it.
                    console.end();
                    diagnostics.say("sign-in: Claude Code's console could not be shown; ended");
                    message_box::error(NOT_STARTED);
                    return;
                }
            }
            WatchStep::Ended { quiet } => {
                diagnostics.say("sign-in: the watch ended without a new sign-in");
                if quiet {
                    message_box::error(NOT_STARTED);
                }
                return;
            }
        }
    }
}

fn run_sign_out(places: &Places) {
    if !message_box::ask_default_no(SIGN_OUT_TITLE, SIGN_OUT_QUESTION) {
        return;
    }
    let Some(console) = start(places, ClaudeCommand::Logout) else {
        return;
    };
    let deadline = Instant::now() + SIGN_OUT_TIMEOUT;
    let code = loop {
        match console.exit_code() {
            None if Instant::now() < deadline => std::thread::sleep(SIGN_OUT_CHECK),
            code => break code,
        }
    };
    // Ended in every case: one that finished has nothing left, and one that did not answer in
    // time is stopped with everything it started.
    console.end();
    // Asked either way: whatever Claude Code did to its sign-in shows at once.
    (places.changed)();
    if code == Some(0) {
        places.diagnostics.say("sign-out: Claude Code signed out");
        message_box::tell(SIGN_OUT_TITLE, SIGNED_OUT);
    } else {
        places
            .diagnostics
            .say("sign-out: claude auth logout failed or did not end in time");
        message_box::error(NOT_SIGNED_OUT);
    }
}
