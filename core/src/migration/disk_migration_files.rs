//! The real folder operations of the migration.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::legacy_migration::MigrationFiles;

pub struct DiskMigrationFiles;

impl MigrationFiles for DiskMigrationFiles {
    fn directory_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }

    /// Fails when `to` exists, as .NET's `Directory.Move` did, so a folder is never moved onto
    /// another one.
    fn move_directory(&self, from: &Path, to: &Path) -> io::Result<()> {
        if fs::symlink_metadata(to).is_ok() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        fs::rename(from, to)
    }

    /// Never writes over a file already in `to`. A link inside `from` is not followed: it is
    /// skipped and counted. A folder that cannot be read or made counts as one, and what it holds
    /// is skipped.
    fn copy_directory(&self, from: &Path, to: &Path) -> usize {
        if fs::create_dir_all(to).is_err() {
            return 1;
        }
        let mut missed = 0;
        let mut folders: Vec<(PathBuf, PathBuf)> = vec![(from.to_path_buf(), to.to_path_buf())];
        while let Some((from, to)) = folders.pop() {
            let Ok(entries) = fs::read_dir(&from) else {
                missed += 1;
                continue;
            };
            for entry in entries {
                let Ok(entry) = entry else {
                    missed += 1;
                    continue;
                };
                let target = to.join(entry.file_name());
                match entry.file_type() {
                    Ok(kind) if kind.is_dir() => {
                        if fs::create_dir_all(&target).is_ok() {
                            folders.push((entry.path(), target));
                        } else {
                            missed += 1;
                        }
                    }
                    Ok(kind) if kind.is_file() => {
                        if copy_file(&entry.path(), &target).is_err() {
                            missed += 1;
                        }
                    }
                    _ => missed += 1,
                }
            }
        }
        missed
    }
}

fn copy_file(from: &Path, to: &Path) -> io::Result<()> {
    if fs::symlink_metadata(to).is_ok() {
        return Err(io::ErrorKind::AlreadyExists.into());
    }
    fs::copy(from, to).map(|_| ())
}
