//! SPIKE (shell candidate B): Sobreiro's tray icon, pill and flyout in Rust. Thrown away after measuring.
#![windows_subsystem = "windows"]

mod app;
mod faces;
mod flyout;
mod icon;
mod native;
mod pill;
mod png;
mod probe;
mod shell;
mod snapshot;
mod spike;
mod uia;
mod watch;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let args: Vec<String> = std::env::args().collect();
    let tray = !matches!(args.get(1).map(String::as_str), Some("--probe" | "--snapshot"));
    // The spike never runs on an installed copy's data: without a scratch folder it leaves.
    if tray && spike::env("SOBREIRO_DATA_DIR").is_none() {
        std::process::exit(3);
    }
    shell::select_backend()?;
    if args.get(1).map(String::as_str) == Some("--probe") {
        return probe::run();
    }
    if let (Some("--snapshot"), Some(folder)) = (args.get(1).map(String::as_str), args.get(2)) {
        return snapshot::run(folder);
    }
    // One tray per data folder, as today.
    if !shell::single_instance(&format!(r"Local\{}.Tray", sobreiro_core::app_paths::AppPaths::instance_id())) {
        return Ok(());
    }
    trace!("start render={} uia={}", if spike::gpu() { "gpu" } else { "software" }, spike::env("SOBREIRO_SPIKE_UIA").unwrap_or("com".into()));
    app::run()
}
