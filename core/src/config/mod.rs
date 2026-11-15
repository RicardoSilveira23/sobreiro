//! The user's settings, kept in `config.json`.
pub mod app_config;

pub use app_config::{AppConfig, ConfigError, PreviousStatusLine, ShowOption};
