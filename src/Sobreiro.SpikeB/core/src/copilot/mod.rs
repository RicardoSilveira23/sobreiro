//! Copilot: GitHub Copilot's monthly AI credits.

mod copilot_cache;
mod copilot_cli_credential;
mod copilot_poll;
mod copilot_provider;
mod copilot_reading;
mod copilot_state;
mod copilot_token;
mod copilot_usage_client;
mod copilot_user_json;
mod credential_store;
mod gh_cli;

pub use copilot_cache::CopilotCache;
pub use copilot_cli_credential::{CopilotCliCredential, ListingAnswer};
pub use copilot_poll::{CopilotPoll, CopilotPollResult};
pub use copilot_provider::CopilotProvider;
pub use copilot_reading::{CopilotReading, CopilotSource};
pub use copilot_state::{CopilotShown, CopilotStateText};
pub use copilot_token::{CopilotSignIn, CopilotSourceKind, CopilotToken, SourceFailed, TokenSearch};
pub use copilot_usage_client::{CopilotFetch, CopilotOutcome, CopilotUsageClient};
pub use copilot_user_json::CopilotUserJson;
pub use credential_store::CredentialStore;
pub use gh_cli::GhCli;
