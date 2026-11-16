//! Sobreiro's Windows shell. With no argument it shows the pill on the taskbar with live figures;
//! the arguments say its version, draw the specimen of its fonts and the pill's pictures, run the
//! born test, and bring the bridge in the data folder up to date. The flyout and the tray icons
//! come next.
#![windows_subsystem = "windows"]

mod born_test;
mod carried_bridge;
mod clock;
mod diagnostics;
mod flyout;
mod icon_pictures;
mod licences;
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
        carried_bridge::start();
        return pill_host::run();
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
        Some("--pill-pictures") => pictures::run(args.next()),
        Some("--icon-pictures") => icon_pictures::run(args.next()),
        Some("--flyout-pictures") => flyout::pictures::run(args.next()),
        _ => ExitCode::from(2),
    }
}
