//! SPIKE (shell candidate B): the switches of the experiments and the trace file.
use std::io::Write;

pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// `SOBREIRO_SPIKE_RENDER=gpu` picks the toolkit's OpenGL renderer; anything else its software renderer.
pub fn gpu() -> bool {
    env("SOBREIRO_SPIKE_RENDER").as_deref() == Some("gpu")
}

/// Appends a line to the file named by `SOBREIRO_SPIKE_TRACE`, if any. Never holds a token.
pub fn trace(line: &str) {
    let Some(path) = env("SOBREIRO_SPIKE_TRACE") else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => { $crate::spike::trace(&format!($($arg)*)) };
}
