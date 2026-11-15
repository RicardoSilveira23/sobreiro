//! Where Claude Code keeps its settings.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::app_paths::is_full_path;

/// The environment variable that moves Claude Code's folder.
pub const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

/// The environment variable that names the user's own folder. Claude Code goes by it, so
/// Sobreiro does too, before it goes by what Windows says.
pub const USER_PROFILE_VARIABLE: &str = "USERPROFILE";

/// Claude Code's folder and the two files in it that Sobreiro touches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePaths {
    config_directory: PathBuf,
}

impl ClaudePaths {
    /// `config_dir` is the value of [`CONFIG_DIR_VARIABLE`] and `user_profile_variable` that
    /// of [`USER_PROFILE_VARIABLE`], when they are set. `user_profile` is the user's own
    /// folder as Windows names it, which the exe asks for: Core makes no call of its own.
    ///
    /// Claude Code's folder is `config_dir` when it is a full path (judged as for Sobreiro's
    /// own folder), and otherwise `.claude` in the user's folder: the variable's when it is
    /// set and not empty, else the one Windows names.
    pub fn new(
        config_dir: Option<&OsStr>,
        user_profile_variable: Option<&OsStr>,
        user_profile: &Path,
    ) -> ClaudePaths {
        let overridden = config_dir.filter(|value| is_full_path(value));
        let config_directory = match overridden {
            Some(dir) => PathBuf::from(dir),
            None => user_profile_variable
                .filter(|variable| !variable.is_empty())
                .map_or(user_profile, Path::new)
                .join(".claude"),
        };
        ClaudePaths { config_directory }
    }

    pub fn config_directory(&self) -> &Path {
        &self.config_directory
    }

    /// Claude Code's sign-in file. Only Live usage reads it.
    pub fn credentials_file(&self) -> PathBuf {
        self.config_directory.join(".credentials.json")
    }

    /// Claude Code's settings, where Connect and Disconnect set the `statusLine` key.
    pub fn settings_file(&self) -> PathBuf {
        self.config_directory.join("settings.json")
    }
}
