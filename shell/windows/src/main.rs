//! Sobreiro's Windows shell. With no argument it shows the pill on the taskbar with live figures,
//! one copy per data folder: a second launch on the same folder asks the first to show its details
//! and exits. The arguments say its version, draw the specimen of its fonts and the pill's
//! pictures, run the born test, bring the bridge in the data folder up to date, and run Connect or
//! Disconnect alone; they take no part in the one-copy rule. The flyout and the tray icons come
//! next.
#![windows_subsystem = "windows"]

mod alerts;
mod app_host;
mod auto_start;
mod born_test;
mod carried_bridge;
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
mod win;

use std::process::ExitCode;

/// What the toolkit writes from the .slint files. That code is the toolkit's, so the workspace's
/// rules on panics are not asked of it.
mod ui {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo)]
    slint::include_modules!();
}

fn main() -> ExitCode {
    licences::keep();
    // Arguments are compared as they come: one that is not Unicode matches nothing.
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else {
        let one_copy::Start::First(mut guard) = one_copy::start() else {
            return ExitCode::SUCCESS;
        };
        guard.listen();
        // After the check: only the first copy refreshes the bridge.
        carried_bridge::start();
        let code = app_host::run();
        drop(guard);
        return code;
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
        Some("--flyout-pictures") => flyout::pictures::run(args.next()),
        _ => ExitCode::from(2),
    }
}
