//! SPIKE (shell candidate B): Sobreiro's tray icon, pill and flyout in Rust. Thrown away after measuring.
#![windows_subsystem = "windows"]

mod native;
mod probe;
mod shell;
mod spike;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    shell::select_backend()?;
    if std::env::args().any(|a| a == "--probe") {
        return probe::run();
    }
    Ok(())
}
