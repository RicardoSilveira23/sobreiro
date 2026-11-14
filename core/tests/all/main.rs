//! Core's tests, built as one program. Each file beside this one holds the tests ported from the
//! C# test file it names in its first line, in the same order and with the same JSON.
//!
//! A C# `[Theory]` with rows became one test that walks the same rows.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod atomic_file;
mod limit_list;
mod rate_limits_json;
mod reset_period;
mod usage_file;
mod usage_level;
mod usage_merge;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use sobreiro_core::usage::{LimitWindow, SpendLimit};

/// A moment written as in the C# tests, for example `"2026-11-09T12:00:00Z"`.
fn time(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// A moment given in seconds since 1970, the way the JSON files hold it.
fn unix(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

/// A session or weekly window that does not say when it was read.
fn window(used_percentage: f64, resets_at: Option<DateTime<Utc>>) -> LimitWindow {
    LimitWindow {
        used_percentage,
        resets_at,
        observed_at: None,
    }
}

/// A session or weekly window read at `observed_at`.
fn window_seen(
    used_percentage: f64,
    resets_at: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
) -> LimitWindow {
    LimitWindow {
        used_percentage,
        resets_at,
        observed_at: Some(observed_at),
    }
}

/// A spend limit that does not say when it was read.
fn spend(
    used_percentage: f64,
    resets_at: Option<DateTime<Utc>>,
    used_usd: Option<f64>,
    limit_usd: Option<f64>,
    period: Option<&str>,
) -> SpendLimit {
    SpendLimit {
        used_percentage,
        resets_at,
        used_usd,
        limit_usd,
        period: period.map(str::to_string),
        observed_at: None,
    }
}

/// A spend limit read at `observed_at`.
fn spend_seen(
    used_percentage: f64,
    resets_at: Option<DateTime<Utc>>,
    used_usd: Option<f64>,
    limit_usd: Option<f64>,
    period: Option<&str>,
    observed_at: DateTime<Utc>,
) -> SpendLimit {
    SpendLimit {
        observed_at: Some(observed_at),
        ..spend(used_percentage, resets_at, used_usd, limit_usd, period)
    }
}

/// A folder for one test, under the build's own temporary folder and never the user's data.
/// It starts empty and is deleted when the test ends.
struct TestFolder(PathBuf);

impl TestFolder {
    /// `name` must be different in every test: tests run at the same time.
    fn new(name: &str) -> TestFolder {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("core")
            .join(name);
        let _ = std::fs::remove_dir_all(&folder);
        TestFolder(folder)
    }

    /// The folder itself. It does not exist until a test or the code under test makes it.
    fn path(&self) -> &Path {
        &self.0
    }

    /// The path of a file inside the folder.
    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Makes the folder, for tests that put a file in it themselves.
    fn create(&self) {
        std::fs::create_dir_all(&self.0).unwrap();
    }

    /// The files in the folder, sorted. Empty when the folder does not exist.
    fn files(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(&self.0) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries.map(|entry| entry.unwrap().path()).collect();
        files.sort();
        files
    }

    /// The files in the folder whose name ends in `.tmp`.
    fn temp_files(&self) -> Vec<PathBuf> {
        self.files()
            .into_iter()
            .filter(|file| file.extension().is_some_and(|ext| ext == "tmp"))
            .collect()
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Opens a file the way a writer that shares it with nobody does, so that no one else can open
/// it until the returned handle is dropped.
#[cfg(windows)]
fn hold_alone(path: &Path) -> std::fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(path)
        .unwrap()
}
