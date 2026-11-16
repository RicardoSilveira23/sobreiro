//! The bridge that the release build carries inside the exe, and the step that keeps the copy in
//! the data folder current. A debug build carries none and finds the bridge beside the exe.
use crate::diagnostics::Diagnostics;
use crate::providers::Locations;
use sobreiro_core::app_paths::AppPaths;
use sobreiro_core::connect::bridge_locator;
use sobreiro_core::connect::bridge_refresh::{self, BridgeRefresh};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread::JoinHandle;

/// The bridge's exe, as built in the same build as this one.
#[cfg(carries_bridge)]
const BYTES: Option<&[u8]> = Some(include_bytes!(env!("SOBREIRO_BRIDGE_EXE")));
#[cfg(not(carries_bridge))]
const BYTES: Option<&[u8]> = None;

/// True when this exe holds the bridge.
pub fn carried() -> bool {
    BYTES.is_some()
}

/// Opens the carried bridge from its first byte: the opener Core's install takes. An error when
/// there is none.
pub fn open() -> io::Result<&'static [u8]> {
    BYTES.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no bridge is carried"))
}

/// The bridge Sobreiro owns: the data folder's copy when the bridge is carried, else the one
/// beside the exe. None when there is none. When the exe's folder cannot be found, nothing is
/// looked for beside it, and a carried bridge is still the data folder's copy.
pub fn own(app: &AppPaths) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok();
    match exe.as_deref().and_then(Path::parent) {
        Some(folder) => bridge_locator::own(carried(), &app.bridge_file(), folder, Path::is_file),
        None => bridge_locator::own(carried(), &app.bridge_file(), Path::new(""), |_| false),
    }
}

/// The start's step: brings the data folder's copy up to date when `settings.json` runs it, and
/// logs one word for the outcome.
fn step(places: &Locations, diagnostics: &Diagnostics) -> BridgeRefresh {
    let outcome = bridge_refresh::refresh(
        carried().then_some(open),
        &places.claude.settings_file(),
        &places.app.bridge_file(),
    );
    diagnostics.say(&format!("bridge {}", outcome.word()));
    outcome
}

/// Runs the step on a thread of its own, so the start does not wait on the disk. The thread
/// gives the outcome back, or None when Windows names no folders. A thread that cannot start
/// gives None, and the start goes on without the step.
pub fn start() -> Option<JoinHandle<Option<BridgeRefresh>>> {
    std::thread::Builder::new()
        .name("bridge-refresh".into())
        .spawn(|| {
            let places = Locations::find()?;
            Some(step(&places, &Diagnostics::new(&places.app.log_file())))
        })
        .ok()
}

/// Hidden: the start's thread, waited for, with its word on stdout.
pub fn refresh_now() -> ExitCode {
    let outcome = start().and_then(|thread| thread.join().ok()).flatten();
    let Some(outcome) = outcome else {
        return ExitCode::FAILURE;
    };
    println!("{}", outcome.word());
    ExitCode::SUCCESS
}
