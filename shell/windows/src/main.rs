//! Sobreiro's Windows shell. With no argument it shows the pill on the taskbar with live figures,
//! one copy per data folder: a second launch on the same folder asks the first to show its details
//! and exits. The arguments say its version, draw the specimen of its fonts and the pill's
//! pictures, run the born test, bring the bridge in the data folder up to date, and run Connect or
//! Disconnect alone; they take no part in the one-copy rule. The flyout and the tray icons come
//! next.
#![windows_subsystem = "windows"]

mod about;
mod alerts;
mod app_host;
mod auto_start;
mod born_test;
mod carried_bridge;
mod child;
mod claude_sign_in;
mod clock;
mod connect_flow;
mod diagnostics;
mod flyout;
mod icon_pictures;
mod licences;
mod one_copy;
mod pictures;
mod pill_face;
mod pill_feed;
mod pill_host;
mod pill_menu;
mod providers;
mod specimen;
mod theme;
mod tooltip;
mod tooltip_window;
mod tray_icons;
mod usage_watch;
mod win;

use std::process::ExitCode;

/// What the toolkit writes from the .slint files. That code is the toolkit's, so the workspace's
/// rules on panics are not asked of it.
mod ui {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo)]
    slint::include_modules!();
}

fn main() -> ExitCode {
    // Before anything loads a DLL: system DLLs come from System32 only.
    win::dll_search::system32_only();
    licences::keep();
    // Arguments are compared as they come: one that is not Unicode matches nothing.
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else {
        let one_copy::Start::First(mut guard) = one_copy::start() else {
            // A second copy ends here, as 1.1's does: after `already-running`.
            say(&exit_line(true));
            return ExitCode::SUCCESS;
        };
        guard.listen();
        std::panic::set_hook(Box::new(on_panic));
        // Main is the UI thread: the message window is made on it and hears the session's end.
        win::taskbar::on_session_ending(on_session_ending);
        // After the check: only the first copy refreshes the bridge.
        carried_bridge::start();
        return match app_host::run() {
            Ok(code) => {
                // The mutex first; the tray icons went with the run.
                drop(guard);
                say(&exit_line(code == ExitCode::SUCCESS));
                code
            }
            Err(kind) => {
                let written = diagnostics::installed()
                    .is_some_and(|log| log.say_with_kind("startup-failed", kind));
                drop(guard);
                show_startup_failed(kind, written);
                say(&exit_line(false));
                ExitCode::FAILURE
            }
        };
    };
    match first.to_str() {
        Some("--version") => {
            println!("Sobreiro {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--specimen") => specimen::run(),
        // Hidden: what the born test launches.
        Some("--born") => born_test::run(args.next().as_deref().and_then(|arg| arg.to_str())),
        // Hidden: what the exe test launches.
        Some("--refresh-bridge") => carried_bridge::refresh_now(),
        // Hidden: the Connect and Disconnect flow alone, for a live run before the menu calls it.
        Some("--connect") => connect_flow::run_now(true),
        Some("--disconnect") => connect_flow::run_now(false),
        Some("--pill-pictures") => pictures::run(args.next()),
        Some("--icon-pictures") => icon_pictures::run(args.next()),
        Some("--flyout-pictures") => flyout::pictures::run(args.next(), args),
        _ => ExitCode::from(2),
    }
}

/// Writes a line to this run's log, when there is one.
fn say(event: &str) -> bool {
    diagnostics::installed().is_some_and(|log| log.say_written(event))
}

/// The last entry of a run, as 1.1's.
fn exit_line(success: bool) -> String {
    format!("exit code={}", u8::from(!success))
}

/// The word 1.1 writes for why the session ends: a sign-out is `Logoff`, anything else `Shutdown`.
fn session_end_line(logoff: bool) -> &'static str {
    if logoff {
        "session-ending reason=Logoff"
    } else {
        "session-ending reason=Shutdown"
    }
}

/// Windows asks to end the session. Says so at once and quits, even if the end is then cancelled.
fn on_session_ending(logoff: bool) {
    say(session_end_line(logoff));
    let _ = slint::quit_event_loop();
}

/// A panic's place as `<file name>:<line>:<column>`: the file's own name, so no folder of the
/// machine that built the exe is written.
fn panic_place(file: &str, line: u32, column: u32) -> String {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    format!("{name}:{line}:{column}")
}

/// Writes the panic's place, never its message. The release build aborts after this.
fn on_panic(info: &std::panic::PanicHookInfo<'_>) {
    let Some(log) = diagnostics::installed() else {
        return;
    };
    match info.location() {
        Some(at) => log.say_with_kind(
            "unhandled-exception terminating=true",
            &panic_place(at.file(), at.line(), at.column()),
        ),
        None => log.say_written("unhandled-exception terminating=true"),
    };
}

/// Tells that the start failed: the box names the log, or the data folder when the log could not
/// be written.
fn show_startup_failed(kind: &str, logged: bool) {
    use sobreiro_core::diagnostics::diagnostics_message::{
        startup_failed, startup_failed_unlogged,
    };
    let log = diagnostics::installed();
    let text = match (&log, logged) {
        (Some(log), true) => startup_failed(kind, &log.path().display().to_string()),
        _ => {
            let folder = log
                .as_ref()
                .and_then(|log| log.path().parent().map(|dir| dir.display().to_string()))
                .unwrap_or_else(|| "%LOCALAPPDATA%\\Sobreiro".to_string());
            startup_failed_unlogged(kind, &folder)
        }
    };
    win::message_box::error(&text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exit_line_has_the_code() {
        assert_eq!(exit_line(true), "exit code=0");
        assert_eq!(exit_line(false), "exit code=1");
    }

    #[test]
    fn the_reason_is_logoff_or_shutdown() {
        assert_eq!(session_end_line(true), "session-ending reason=Logoff");
        assert_eq!(session_end_line(false), "session-ending reason=Shutdown");
    }

    #[test]
    fn the_place_keeps_the_file_name_only() {
        assert_eq!(panic_place("src/main.rs", 3, 9), "main.rs:3:9");
        assert_eq!(
            panic_place("C:\\build\\shell\\src\\app_host.rs", 10, 1),
            "app_host.rs:10:1"
        );
        assert_eq!(panic_place("lib.rs", 1, 2), "lib.rs:1:2");
    }
}
