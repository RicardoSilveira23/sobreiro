//! SPIKE (shell candidate B): Sobreiro's tray icon, pill and flyout in Rust. Thrown away after measuring.
#![windows_subsystem = "windows"]

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

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    shell::select_backend()?;
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--probe") {
        return probe::run();
    }
    if let (Some("--snapshot"), Some(folder)) = (args.get(1).map(String::as_str), args.get(2)) {
        return snapshot::run(folder);
    }
    Ok(())
}
