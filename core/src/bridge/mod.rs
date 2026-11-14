//! What the bridge does, without the exe around it: read Claude Code's status line JSON, keep
//! its `rate_limits` in `usage.json`, and print a status line (the user's previous one when
//! there is one). The exe is the crate `sobreiro-bridge`, in `bridge/`.
pub mod bash_locator;
pub mod bridge_config;
pub mod bridge_runner;

pub use bridge_config::BridgeConfig;
pub use bridge_runner::{BridgeRunner, ChainResult};
