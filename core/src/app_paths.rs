//! Where Sobreiro keeps its files.
//!
//! Only what the bridge needs is here so far: the data folder, `usage.json` and `config.json`.
//! The other files, the old folder and the instance's name arrive with "Paths and config".
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The environment variable that moves the data folder: for tests, and for a second copy that
/// must not touch the first one's files.
pub const DATA_DIR_VARIABLE: &str = "SOBREIRO_DATA_DIR";

/// The data folder and the files in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    data_directory: PathBuf,
}

impl AppPaths {
    /// `overridden` is the value of [`DATA_DIR_VARIABLE`], when it is set. `local_app_data` is
    /// the user's local application data folder, which the exe asks Windows for: Core makes no
    /// call of its own.
    ///
    /// The data folder is the override when it is a full path (drive and all), and otherwise
    /// `Sobreiro` in the local application data folder.
    pub fn new(overridden: Option<&OsStr>, local_app_data: &Path) -> AppPaths {
        let data_directory = overridden
            .map(Path::new)
            .filter(|path| path.is_absolute())
            .map_or_else(|| local_app_data.join("Sobreiro"), Path::to_path_buf);
        AppPaths { data_directory }
    }

    pub fn data_directory(&self) -> &Path {
        &self.data_directory
    }

    /// The limits as last read: written by the bridge and by Live usage, read by the tray.
    pub fn usage_file(&self) -> PathBuf {
        self.data_directory.join("usage.json")
    }

    /// The settings, among them the user's previous status line command.
    pub fn config_file(&self) -> PathBuf {
        self.data_directory.join("config.json")
    }
}
