//! Connect and Disconnect: the `statusLine` key of Claude Code's `settings.json`, and the
//! bridge it runs.
pub mod bridge_command;
pub mod bridge_install;
pub mod bridge_locator;
pub mod bridge_refresh;
pub mod claude_paths;
pub mod connect_message;
pub mod connection;
pub(crate) mod json_reader;
pub mod settings_json;

pub use claude_paths::ClaudePaths;
pub use connection::{ConnectError, ConnectionPlan, ConnectionState, SettingsSnapshot};
pub use settings_json::SettingsError;
