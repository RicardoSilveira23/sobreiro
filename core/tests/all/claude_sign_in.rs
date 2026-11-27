//! New in 2.x, with no C# behind it: the menu's Sign in to and Sign out of Claude Code. No test
//! starts Claude Code or a console; the shell does that.
use std::path::Path;
use std::time::{Duration, SystemTime};

use sobreiro_core::live::claude_sign_in::{
    self, ClaudeCommand, LOGIN_QUIET_WAIT, Offer, QUIET_WAIT, Seen, SignInWatch, WATCH_LIMIT,
    WatchStep,
};
use sobreiro_core::polling::ProviderState;

use crate::TestFolder;

#[test]
fn sign_in_is_offered_for_a_missing_run_out_or_refused_sign_in_and_sign_out_otherwise() {
    let rows = [
        (ProviderState::Expired, Some(Offer::SignIn)),
        (ProviderState::NoSignIn, Some(Offer::SignIn)),
        (ProviderState::Refused, Some(Offer::SignIn)),
        (ProviderState::Ok, Some(Offer::SignOut)),
        (ProviderState::RateLimited, Some(Offer::SignOut)),
        (ProviderState::Offline, Some(Offer::SignOut)),
        (ProviderState::BadAnswer, Some(Offer::SignOut)),
        (ProviderState::Off, None),
    ];
    for (state, offer) in rows {
        assert_eq!(claude_sign_in::offer(state, true), offer, "{state:?}");
        assert_eq!(
            claude_sign_in::offer(state, false),
            None,
            "{state:?} hidden"
        );
    }
}

#[test]
fn a_sign_in_that_ran_out_starts_claude_and_any_other_logs_in() {
    assert_eq!(
        ClaudeCommand::sign_in(ProviderState::Expired),
        ClaudeCommand::Start
    );
    for state in [ProviderState::NoSignIn, ProviderState::Refused] {
        assert_eq!(
            ClaudeCommand::sign_in(state),
            ClaudeCommand::Login,
            "{state:?}"
        );
    }
}

#[test]
fn claude_is_found_on_the_path_exe_before_cmd_and_skipping_relative_entries() {
    let folder = TestFolder::new("claude_sign_in-find");
    folder.create();
    let npm = folder.file("npm");
    let native = folder.file("native");
    std::fs::create_dir_all(&npm).unwrap();
    std::fs::create_dir_all(&native).unwrap();
    std::fs::write(npm.join("claude.cmd"), "").unwrap();
    std::fs::write(native.join("claude.exe"), "").unwrap();
    std::fs::write(native.join("claude.cmd"), "").unwrap();
    let nobody = folder.file("nobody");
    let path = format!(
        ";relative;{};{}",
        npm.to_str().unwrap(),
        native.to_str().unwrap()
    );

    assert_eq!(
        claude_sign_in::find(Some(&path), &nobody),
        Some(npm.join("claude.cmd"))
    );
    assert_eq!(
        claude_sign_in::find(Some(native.to_str().unwrap()), &nobody),
        Some(native.join("claude.exe"))
    );
    assert_eq!(claude_sign_in::find(Some(";relative"), &nobody), None);
    assert_eq!(claude_sign_in::find(None, &nobody), None);
}

#[test]
fn claude_is_found_in_the_native_installers_folder_when_the_path_has_none() {
    let folder = TestFolder::new("claude_sign_in-native");
    folder.create();
    let bin = folder.path().join(".local").join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("claude.exe"), "").unwrap();

    assert_eq!(
        claude_sign_in::find(Some(""), folder.path()),
        Some(bin.join("claude.exe"))
    );
    // A user folder that is not a full path is not looked in.
    assert_eq!(claude_sign_in::find(None, Path::new("relative")), None);
}

#[test]
fn cmd_is_given_the_path_in_its_own_quotes_inside_the_outer_pair() {
    let claude = Path::new(r"C:\Program Files (x86)\a&b\claude.exe");
    assert_eq!(
        claude_sign_in::cmd_arguments(claude, ClaudeCommand::Start),
        Some(
            r#"/d /v:off /s /k ""C:\Program Files (x86)\a&b\claude.exe" & title Sobreiro - Claude Code has ended""#
                .to_string()
        )
    );
    assert_eq!(
        claude_sign_in::cmd_arguments(claude, ClaudeCommand::Login),
        Some(
            r#"/d /v:off /s /k ""C:\Program Files (x86)\a&b\claude.exe" auth login & title Sobreiro - Claude Code has ended""#
                .to_string()
        )
    );
    // A sign-out leaves no console behind.
    assert_eq!(
        claude_sign_in::cmd_arguments(claude, ClaudeCommand::Logout),
        Some(
            r#"/d /v:off /s /c ""C:\Program Files (x86)\a&b\claude.exe" auth logout""#.to_string()
        )
    );
}

#[test]
fn cmd_is_given_no_path_with_a_quote_or_a_percent_or_that_is_not_full() {
    for path in [
        r"C:\a%PATH%\claude.exe",
        "C:\\a\"\\claude.exe",
        r"bin\claude.exe",
    ] {
        assert_eq!(
            claude_sign_in::cmd_arguments(Path::new(path), ClaudeCommand::Start),
            None,
            "{path}"
        );
    }
}

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000 + seconds)
}

const SECOND: Duration = Duration::from_secs(1);

/// A look `elapsed` in, with the file changed at `changed_at` seconds (none: no file).
fn seen(elapsed: Duration, changed_at: Option<u64>) -> Seen {
    Seen {
        elapsed,
        changed_at: changed_at.map(at),
        running: true,
        claude_ended: false,
    }
}

fn start(before: Option<u64>) -> SignInWatch {
    SignInWatch::new(before.map(at), ClaudeCommand::Start)
}

#[test]
fn a_quiet_renewal_closes_the_console_without_showing_it() {
    let mut watch = start(Some(0));
    assert_eq!(watch.step(seen(SECOND, Some(0))), WatchStep::Wait);
    assert_eq!(watch.step(seen(3 * SECOND, Some(3))), WatchStep::Renewed);
    assert!(!watch.shown());
}

#[test]
fn a_file_that_appears_is_a_renewal_and_one_that_goes_is_not() {
    let mut appears = start(None);
    assert_eq!(appears.step(seen(SECOND, None)), WatchStep::Wait);
    assert_eq!(appears.step(seen(2 * SECOND, Some(2))), WatchStep::Renewed);

    let mut goes = start(Some(0));
    assert_eq!(goes.step(seen(SECOND, None)), WatchStep::Wait);
}

#[test]
fn claude_is_shown_once_after_its_quiet_wait_and_closed_when_it_signs_in() {
    let mut watch = start(Some(0));
    assert_eq!(
        watch.step(seen(QUIET_WAIT - SECOND, Some(0))),
        WatchStep::Wait
    );
    assert_eq!(watch.step(seen(QUIET_WAIT, Some(0))), WatchStep::Show);
    assert!(watch.shown());
    assert_eq!(
        watch.step(seen(QUIET_WAIT + SECOND, Some(0))),
        WatchStep::Wait
    );
    // Shown or not, a sign-in that is done closes the console.
    assert_eq!(
        watch.step(seen(5 * QUIET_WAIT, Some(90))),
        WatchStep::Renewed
    );
}

#[test]
fn a_login_stays_hidden_for_its_longer_wait_while_the_browser_is_used() {
    let mut watch = SignInWatch::new(None, ClaudeCommand::Login);
    assert_eq!(watch.step(seen(QUIET_WAIT, None)), WatchStep::Wait);
    assert_eq!(
        watch.step(seen(LOGIN_QUIET_WAIT - SECOND, None)),
        WatchStep::Wait
    );
    assert_eq!(watch.step(seen(LOGIN_QUIET_WAIT, None)), WatchStep::Show);
    assert_eq!(
        watch.step(seen(LOGIN_QUIET_WAIT * 2, Some(130))),
        WatchStep::Renewed
    );
}

#[test]
fn a_login_done_in_the_browser_is_never_shown() {
    let mut watch = SignInWatch::new(None, ClaudeCommand::Login);
    assert_eq!(watch.step(seen(30 * SECOND, Some(30))), WatchStep::Renewed);
    assert!(!watch.shown());
}

#[test]
fn claude_that_ended_without_signing_in_is_shown_at_once() {
    for command in [ClaudeCommand::Start, ClaudeCommand::Login] {
        let mut watch = SignInWatch::new(Some(at(0)), command);
        let ended = Seen {
            claude_ended: true,
            ..seen(2 * SECOND, Some(0))
        };
        assert_eq!(watch.step(ended), WatchStep::Show, "{command:?}");
        assert_eq!(watch.step(ended), WatchStep::Wait, "{command:?} once");
    }
}

#[test]
fn a_closed_console_ends_the_watch() {
    let closed = |elapsed| Seen {
        running: false,
        ..seen(elapsed, Some(0))
    };
    let mut quiet = start(Some(0));
    assert_eq!(quiet.step(closed(SECOND)), WatchStep::Ended { quiet: true });

    let mut shown = start(Some(0));
    assert_eq!(shown.step(seen(QUIET_WAIT, Some(0))), WatchStep::Show);
    assert_eq!(
        shown.step(closed(QUIET_WAIT + SECOND)),
        WatchStep::Ended { quiet: false }
    );
}

#[test]
fn a_renewal_counts_even_when_the_console_closed_with_it() {
    let mut watch = start(Some(0));
    let closed = Seen {
        running: false,
        ..seen(SECOND, Some(1))
    };
    assert_eq!(watch.step(closed), WatchStep::Renewed);
}

#[test]
fn a_shown_console_is_watched_until_the_limit() {
    let mut watch = start(Some(0));
    assert_eq!(watch.step(seen(QUIET_WAIT, Some(0))), WatchStep::Show);
    assert_eq!(
        watch.step(seen(WATCH_LIMIT - SECOND, Some(0))),
        WatchStep::Wait
    );
    assert_eq!(
        watch.step(seen(WATCH_LIMIT, Some(0))),
        WatchStep::Ended { quiet: false }
    );
}
