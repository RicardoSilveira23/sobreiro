//! GitHub Copilot: a GitHub token, the one request made with it, and the reading that comes
//! back. The provider, its sign-in and the CLI's cache file arrive with their own area.
pub mod copilot_reading;
pub mod copilot_token;
pub mod copilot_usage_client;
pub mod copilot_user_json;

pub use copilot_reading::{CopilotReading, CopilotSource};
pub use copilot_token::GitHubToken;
pub use copilot_usage_client::{CopilotFetch, CopilotOutcome};
