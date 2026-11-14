use std::path::{Path, PathBuf};

/// Where Claude Code keeps its settings. `get_env` is `Environment.GetEnvironmentVariable`.
pub struct ClaudePaths;

const CONFIG_DIR_VARIABLE: &str = "CLAUDE_CONFIG_DIR";

impl ClaudePaths {
    /// CLAUDE_CONFIG_DIR when set and fully qualified, else USERPROFILE\.claude.
    pub fn config_directory(get_env: &dyn Fn(&str) -> Option<String>) -> String {
        if let Some(v) = get_env(CONFIG_DIR_VARIABLE) {
            if !v.is_empty() && Path::new(&v).is_absolute() {
                return v;
            }
        }
        let profile = get_env("USERPROFILE").filter(|p| !p.is_empty()).unwrap_or_default();
        PathBuf::from(profile).join(".claude").to_string_lossy().into_owned()
    }

    /// Claude Code's sign-in file. Only Live usage reads it.
    pub fn credentials_file(get_env: &dyn Fn(&str) -> Option<String>) -> String {
        join(Self::config_directory(get_env), ".credentials.json")
    }

    pub fn settings_file(get_env: &dyn Fn(&str) -> Option<String>) -> String {
        join(Self::config_directory(get_env), "settings.json")
    }
}

fn join(dir: String, name: &str) -> String {
    PathBuf::from(dir).join(name).to_string_lossy().into_owned()
}
