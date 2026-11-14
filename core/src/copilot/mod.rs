//! GitHub Copilot. Only the reading itself is here so far, because the list of limits is made
//! from it; the provider, its sign-in and the CLI's cache file arrive with their own area.
pub mod copilot_reading;

pub use copilot_reading::{CopilotReading, CopilotSource};
