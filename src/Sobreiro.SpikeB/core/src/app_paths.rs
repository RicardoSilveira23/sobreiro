use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Where the app keeps its files. Every function has a `_with` twin that takes the environment, for tests.
pub struct AppPaths;

type GetEnv<'a> = &'a dyn Fn(&str) -> Option<String>;

fn std_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

const DATA_DIR_VARIABLE: &str = "SOBREIRO_DATA_DIR";

fn override_dir(get_env: GetEnv) -> Option<String> {
    let v = get_env(DATA_DIR_VARIABLE)?;
    if v.is_empty() || !Path::new(&v).is_absolute() { None } else { Some(v) }
}

/// Environment.GetFolderPath(LocalApplicationData): LOCALAPPDATA, else USERPROFILE\AppData\Local.
fn local_app_data(get_env: GetEnv) -> PathBuf {
    if let Some(v) = get_env("LOCALAPPDATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(v);
    }
    PathBuf::from(get_env("USERPROFILE").unwrap_or_default()).join("AppData").join("Local")
}

fn s(p: PathBuf) -> String {
    p.to_string_lossy().into_owned()
}

impl AppPaths {
    /// %LocalAppData%\Sobreiro, whatever the override says.
    pub fn default_directory() -> String {
        Self::default_directory_with(&std_env)
    }
    pub fn default_directory_with(get_env: GetEnv) -> String {
        s(local_app_data(get_env).join("Sobreiro"))
    }

    /// %LocalAppData%\OpenHeadroom, where the app kept its data before 0.5.0.
    pub fn legacy_directory() -> String {
        Self::legacy_directory_with(&std_env)
    }
    pub fn legacy_directory_with(get_env: GetEnv) -> String {
        s(local_app_data(get_env).join("OpenHeadroom"))
    }

    /// True when the data directory is the default folder: no override, or one that names it.
    pub fn is_default_data_directory() -> bool {
        Self::is_default_data_directory_with(&std_env)
    }
    pub fn is_default_data_directory_with(get_env: GetEnv) -> bool {
        match override_dir(get_env) {
            None => true,
            Some(dir) => normalise(&dir) == normalise(&Self::default_directory_with(get_env)),
        }
    }

    pub fn data_directory() -> String {
        Self::data_directory_with(&std_env)
    }
    pub fn data_directory_with(get_env: GetEnv) -> String {
        override_dir(get_env).unwrap_or_else(|| Self::default_directory_with(get_env))
    }

    /// The bridge copied out of the Sobreiro exe: the one the status line runs.
    pub fn bridge_file() -> String {
        Self::in_data("sobreiro-bridge.exe", &std_env)
    }
    pub fn usage_file() -> String {
        Self::in_data("usage.json", &std_env)
    }
    pub fn usage_file_with(get_env: GetEnv) -> String {
        Self::in_data("usage.json", get_env)
    }
    pub fn config_file() -> String {
        Self::in_data("config.json", &std_env)
    }
    pub fn config_file_with(get_env: GetEnv) -> String {
        Self::in_data("config.json", get_env)
    }
    pub fn alerts_file() -> String {
        Self::in_data("alerts.json", &std_env)
    }
    pub fn log_file() -> String {
        Self::in_data("log.txt", &std_env)
    }

    fn in_data(name: &str, get_env: GetEnv) -> String {
        s(PathBuf::from(Self::data_directory_with(get_env)).join(name))
    }

    /// "Sobreiro" normally, and when the data directory override names the default folder; with any other
    /// override, "Sobreiro." plus 8 hex digits of its case-insensitive hash, so scratch instances get their own
    /// single-instance name and two trays never share one folder.
    pub fn instance_id() -> String {
        Self::instance_id_with(&std_env)
    }
    pub fn instance_id_with(get_env: GetEnv) -> String {
        let Some(dir) = override_dir(get_env) else { return "Sobreiro".to_string() };
        let key = normalise(&dir);
        if key == normalise(&Self::default_directory_with(get_env)) {
            return "Sobreiro".to_string();
        }
        let hash = Sha256::digest(key.as_bytes());
        let hex: String = hash[..4].iter().map(|b| format!("{b:02x}")).collect();
        format!("Sobreiro.{hex}")
    }
}

/// GetFullPath, trailing separator trimmed, upper-cased.
fn normalise(dir: &str) -> String {
    let full = std::path::absolute(dir).map(s).unwrap_or_else(|_| dir.to_string());
    let trimmed = if full.len() > 3 { full.trim_end_matches(['\\', '/']) } else { full.as_str() };
    trimmed.to_uppercase()
}
