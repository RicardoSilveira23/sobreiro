//! First run after the rename from OpenHeadroom: moves the data folder and the Start with
//! Windows entry. It never touches Claude Code's `settings.json`: a status line that still runs
//! `openheadroom-bridge` counts as connected, and Reconnect repoints it.
use std::io;
use std::path::Path;

use crate::startup::auto_start_command;

/// The old app's name, which is also the name of its Run value.
pub const OLD_NAME: &str = "OpenHeadroom";

/// The old app's single-instance mutex for the default data folder.
pub const OLD_MUTEX_NAME: &str = r"Local\OpenHeadroom.Tray";

pub const STILL_RUNNING_MESSAGE: &str =
    "OpenHeadroom is still running. Quit it from its tray menu, then start Sobreiro again.";

/// The file system, as far as the migration needs it.
pub trait MigrationFiles {
    fn directory_exists(&self, path: &Path) -> bool;

    /// Renames the folder. Fails when it can't (a locked file, say), leaving it where it was.
    fn move_directory(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Copies what can be copied and skips the rest; returns how many files were skipped.
    fn copy_directory(&self, from: &Path, to: &Path) -> usize;
}

/// The values under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
pub trait RunValues {
    fn get(&self, name: &str) -> io::Result<Option<String>>;

    fn set(&self, name: &str, command: &str) -> io::Result<()>;

    fn delete(&self, name: &str) -> io::Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFolderOutcome {
    /// A data directory override is in use: nothing is touched.
    NotDefault,
    NothingToMove,
    BothExist,
    Moved,
    CopiedAfterMoveFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunValueOutcome {
    Absent,
    Migrated,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationResult {
    pub old_app_running: bool,
    pub data_folder: DataFolderOutcome,
    pub files_not_copied: usize,
    pub run_value: RunValueOutcome,
}

pub struct MigrationContext<'a> {
    /// False when a data directory override is in use.
    pub is_default_data_directory: bool,
    pub old_data_directory: &'a Path,
    pub new_data_directory: &'a Path,
    pub exe_path: &'a Path,
    /// True while the old app's single-instance mutex is held.
    pub old_app_running: &'a dyn Fn() -> bool,
    pub files: &'a dyn MigrationFiles,
    pub run_values: &'a dyn RunValues,
}

/// Runs the migration. Nothing is touched while a data directory override is in use or while
/// the old app runs; otherwise the folder moves first, then the Run value.
pub fn run(c: &MigrationContext) -> MigrationResult {
    if !c.is_default_data_directory {
        return MigrationResult {
            old_app_running: false,
            data_folder: DataFolderOutcome::NotDefault,
            files_not_copied: 0,
            run_value: RunValueOutcome::Absent,
        };
    }
    if (c.old_app_running)() {
        return MigrationResult {
            old_app_running: true,
            data_folder: DataFolderOutcome::NothingToMove,
            files_not_copied: 0,
            run_value: RunValueOutcome::Absent,
        };
    }

    let (data_folder, files_not_copied) = move_data_folder(c);
    MigrationResult {
        old_app_running: false,
        data_folder,
        files_not_copied,
        run_value: migrate_run_value(c),
    }
}

fn move_data_folder(c: &MigrationContext) -> (DataFolderOutcome, usize) {
    if !c.files.directory_exists(c.old_data_directory) {
        return (DataFolderOutcome::NothingToMove, 0);
    }
    if c.files.directory_exists(c.new_data_directory) {
        return (DataFolderOutcome::BothExist, 0);
    }
    match c
        .files
        .move_directory(c.old_data_directory, c.new_data_directory)
    {
        Ok(()) => (DataFolderOutcome::Moved, 0),
        // Carry on with a copy; the old folder stays where it is.
        Err(_) => (
            DataFolderOutcome::CopiedAfterMoveFailed,
            c.files
                .copy_directory(c.old_data_directory, c.new_data_directory),
        ),
    }
}

fn migrate_run_value(c: &MigrationContext) -> RunValueOutcome {
    let migrate = || -> io::Result<RunValueOutcome> {
        if c.run_values.get(OLD_NAME)?.is_none() {
            return Ok(RunValueOutcome::Absent);
        }
        c.run_values.set(
            auto_start_command::VALUE_NAME,
            &auto_start_command::command_for(c.exe_path),
        )?;
        c.run_values.delete(OLD_NAME)?;
        Ok(RunValueOutcome::Migrated)
    };
    migrate().unwrap_or(RunValueOutcome::Failed)
}
