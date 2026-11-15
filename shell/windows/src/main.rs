//! Sobreiro's Windows shell. So far it is only the build: an exe that says its version and draws
//! the specimen of its fonts. The pill, the flyout and the tray icons come next.
#![windows_subsystem = "windows"]

mod pictures;
mod pill_face;
mod specimen;

use std::process::ExitCode;

/// What the toolkit writes from the .slint files. That code is the toolkit's, so the workspace's
/// rules on panics are not asked of it.
mod ui {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo)]
    slint::include_modules!();
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("--version") => {
            println!("Sobreiro {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--specimen") => specimen::run(),
        Some("--pill-pictures") => pictures::run(std::env::args().nth(2)),
        // Nothing to show yet.
        _ => ExitCode::from(2),
    }
}
