//! GitHub Copilot: a GitHub token, the one request made with it, and the reading that comes
//! back; the places a token is looked for, the poll over them, the CLI's cache file, and the
//! provider that keeps the readings of the poll and of the cache, and the sentence under the
//! figures.
pub mod copilot_cache;
pub mod copilot_cli_credential;
pub mod copilot_poll;
pub mod copilot_provider;
pub mod copilot_reading;
pub mod copilot_shown;
pub mod copilot_sign_in;
pub mod copilot_state_text;
pub mod copilot_token;
pub mod copilot_usage_client;
pub mod copilot_user_json;
pub mod gh_cli;

pub use copilot_reading::{CopilotReading, CopilotSource};
pub use copilot_token::GitHubToken;
pub use copilot_usage_client::{CopilotFetch, CopilotOutcome};
