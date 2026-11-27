//! The menu's Sign in to Claude Code and Sign out of Claude Code, each on a thread of its own, one
//! at a time (`sobreiro_core::live::claude_sign_in` decides each step).
//!
//! Sign in runs `claude` (or `claude auth login` when there is no sign-in at all) in a console no
//! one sees, and watches Claude Code's sign-in file as Core's `SignInWatch` says. When Claude Code
//! writes the file, its console is closed with everything it started and Live usage asks at once.
//! When it needs the user (to sign in in the browser, to trust a folder, or to read an error), its
//! console is shown as it stands and the user goes on there; the watch goes on, and Live usage
//! asks once the file is written, and the console is closed then too. A login stays hidden for
//! longer, while the browser it opened is used; Claude Code ending without a sign-in shows its
//! console at once, so its words can be read.
//!
//! Sign out asks first, then runs `claude auth logout` with no window, and Live usage asks at
//! once, so the menu offers Sign in again.
//!
//! Sobreiro renews, writes and removes no sign-in: Claude Code does, as when the user runs it.
//! Only the file's time of change is looked at; its contents are read by Live usage alone.
use crate::child::{self, CREATE_NO_WINDOW};
use crate::diagnostics::Diagnostics;
use crate::win::console::HiddenConsole;
use crate::win::job::Job;
use crate::win::{known_folders, message_box};
use sobreiro_core::live::claude_sign_in::{
    self, CHECK_EVERY, ClaudeCommand, ENDED_TITLE, SIGN_IN_TITLE, SIGN_OUT_QUESTION,
    SIGN_OUT_TIMEOUT, SIGN_OUT_TITLE, Seen, SignInWatch, WatchStep,
};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Set while a sign-in or a sign-out is under way.
static RUNNING: AtomicBool = AtomicBool::new(false);

/// What the box says when Claude Code is not on this PC.
const NOT_FOUND: &str = "Sobreiro couldn't find Claude Code on this PC: no claude.exe or \
claude.cmd on the PATH or in .local\\bin.\n\nInstall Claude Code, or open a terminal and run \
claude there.";

/// What the box says when a sign-in could not be started.
const NOT_STARTED: &str = "Sobreiro couldn't start Claude Code.\n\nOpen a terminal and run \
claude there to sign in.";

/// What the box says when a sign-in or a sign-out is asked for while one is under way.
const UNDER_WAY: &str = "Sobreiro is already signing Claude Code in or out.

If Claude Code's window is open, finish there or close it, then try again.";

/// What the box says when a sign-out did not end well.
const NOT_SIGNED_OUT: &str = "Claude Code didn't sign out.\n\nOpen a terminal and run \
claude auth logout there to see why.";

/// What the box says when a sign-out ended well.
const SIGNED_OUT: &str = "Claude Code is signed out on this PC.\n\nChoose Sign in to Claude Code \
in Sobreiro's menu to sign in, with this account or another.";

/// What a run needs: where Claude Code's sign-in file and the user's folder are, the log, and
/// what to call, on the run's thread, each time the sign-in changed.
pub struct Places {
    pub credentials_file: PathBuf,
    pub user_profile: PathBuf,
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
            work(&places);
            RUNNING.store(false, Ordering::SeqCst);
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

/// `cmd.exe`, by its full path in the Windows folder (never by bare name, which Windows would
/// look for beside Sobreiro first), and what it is given to run `command`. None, with a log line
/// and a box, when Claude Code or the Windows folder is not found.
fn cmd_for(places: &Places, command: ClaudeCommand) -> Option<(PathBuf, String)> {
    let path = std::env::var("PATH").ok();
    let Some(claude) = claude_sign_in::find(path.as_deref(), &places.user_profile) else {
        places.diagnostics.say("sign-in: Claude Code was not found");
        message_box::error(NOT_FOUND);
        return None;
    };
    // `find` gives only a path `cmd_arguments` takes, so only the Windows folder can be missing.
    let found = claude_sign_in::cmd_arguments(&claude, command).zip(known_folders::windows());
    let Some((arguments, windows)) = found else {
        places
            .diagnostics
            .say("sign-in: cmd.exe could not be found, so Claude Code was not started");
        message_box::error(NOT_STARTED);
        return None;
    };
    Some((windows.join("System32").join("cmd.exe"), arguments))
}

fn run_sign_in(places: &Places, command: ClaudeCommand) {
    let Some((cmd, arguments)) = cmd_for(places, command) else {
        return;
    };
    let diagnostics = &places.diagnostics;
    let Some(console) = HiddenConsole::start(&cmd, &arguments, &places.user_profile) else {
        diagnostics.say("sign-in: Claude Code could not be started");
        message_box::error(NOT_STARTED);
        return;
    };
    diagnostics.say("sign-in: Claude Code started out of sight");
    let started = Instant::now();
    let mut watch = SignInWatch::new(changed_at(&places.credentials_file), command);
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
                let found = console.show();
                diagnostics.say(if found {
                    "sign-in: Claude Code needs the user; its console was shown"
                } else {
                    "sign-in: Claude Code needs the user; its console's window was not found"
                });
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
    let Some((cmd, arguments)) = cmd_for(places, ClaudeCommand::Logout) else {
        return;
    };
    let ended_well = run_hidden(&cmd, &arguments, &places.user_profile);
    // Asked either way: whatever Claude Code did to its sign-in shows at once.
    (places.changed)();
    if ended_well {
        places.diagnostics.say("sign-out: Claude Code signed out");
        message_box::tell(SIGN_OUT_TITLE, SIGNED_OUT);
    } else {
        places
            .diagnostics
            .say("sign-out: claude auth logout failed or did not end in time");
        message_box::error(NOT_SIGNED_OUT);
    }
}

/// Runs `cmd` with `arguments` in `folder`, with no window, and waits for it within
/// [`SIGN_OUT_TIMEOUT`]. True when it ended in time with exit code 0. It runs in a job object, so
/// on a timeout it and anything it started are ended.
fn run_hidden(cmd: &Path, arguments: &str, folder: &Path) -> bool {
    let Ok(mut child) = Command::new(cmd)
        .raw_arg(arguments)
        .current_dir(folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
    else {
        return false;
    };
    let job = Job::kill_on_close();
    if let Some(job) = &job {
        let _ = job.assign(&child);
    }
    let deadline = Instant::now() + SIGN_OUT_TIMEOUT;
    let status = child::wait_until(&mut child, deadline, Duration::from_millis(100));
    // Ended in every case: one that finished has nothing left to end, and one that did not is
    // stopped with what it started.
    let _ = child.kill();
    drop(job);
    let _ = child.wait();
    status.is_some_and(|status| status.success())
}
