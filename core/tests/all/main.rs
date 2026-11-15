//! Core's tests, built as one program. Each file beside this one holds the tests ported from the
//! C# test file it names in its first line, in the same order and with the same JSON.
//!
//! A C# `[Theory]` with rows became one test that walks the same rows.
//!
//! Two files have no C# test file behind them: `csharp_files.rs` holds what the C# itself
//! wrote to the disk, for the Rust to write the same, and `http.rs` runs the real HTTP sender
//! against a server on this PC.
//!
//! Tests named `..._as_the_csharp_did` walk rows that the C# of 1.x wrote out itself, run on
//! made-up cases; `README.md` beside this folder says when.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod app_config;
mod app_paths;
mod atomic_file;
mod bash_locator;
mod bridge_command;
mod bridge_config;
mod bridge_install;
mod bridge_locator;
mod bridge_runner;
mod claude_credentials;
mod claude_paths;
mod claude_provider;
mod connect_message;
mod connection;
mod copilot_cache;
mod copilot_cli_credential;
mod copilot_poll;
mod copilot_provider;
mod copilot_shown;
mod copilot_token;
mod copilot_usage_client;
mod copilot_user_json;
mod csharp_files;
mod gh_cli;
mod http;
mod legacy_connection;
mod limit_list;
mod live_poll;
mod live_schedule;
mod live_state;
mod live_usage_client;
mod provider_poller;
mod rate_limits_json;
mod reset_period;
mod settings_json;
mod status_line_text;
mod usage_api_json;
mod usage_file;
mod usage_level;
mod usage_merge;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use sobreiro_core::io::http::{Answer, NoAnswer, Sender};
use sobreiro_core::live::live_schedule::{LiveSchedule, RefusedSignIn};
use sobreiro_core::usage::{LimitWindow, SpendLimit};

/// A moment written as in the C# tests, for example `"2026-11-09T12:00:00Z"`.
fn time(text: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

/// A moment given in seconds since 1970, the way the JSON files hold it.
fn unix(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).unwrap()
}

/// The rows that the C# wrote give a moment as a count of ticks, the ten-millionths of a second
/// its clock counts in, from noon on 2026-11-09 (UTC): 600_000_000 is a minute after it.
fn at_ticks(ticks: i64) -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z") + chrono::TimeDelta::nanoseconds(ticks * 100)
}

/// A second, a minute, an hour and a day in ticks, for such rows.
const S: i64 = 10_000_000;
const M: i64 = 60 * S;
const H: i64 = 60 * M;
const D: i64 = 24 * H;

/// In such a row, where there is no moment.
const NONE: i64 = i64::MIN;

/// In such a row, a sign-in that was refused and named no expiry.
const NO_EXPIRY: i64 = i64::MIN + 1;

/// A moment of such a row, or none.
fn maybe_at(ticks: i64) -> Option<DateTime<Utc>> {
    (ticks != NONE).then(|| at_ticks(ticks))
}

/// A schedule as such a row holds it: when the next poll is due, the last success, the step
/// of the back-off, the end of a wait, the expiry of a refused sign-in, and the misses.
type ScheduleRow = (i64, i64, u32, i64, i64, u32);

fn schedule_of(row: ScheduleRow) -> LiveSchedule {
    let (next_due, last_success, rate_limit_step, wait_until, refused, misses) = row;
    LiveSchedule {
        next_due: at_ticks(next_due),
        last_success: maybe_at(last_success),
        rate_limit_step,
        wait_until: maybe_at(wait_until),
        refused: match refused {
            NONE => None,
            NO_EXPIRY => Some(RefusedSignIn { expires_at: None }),
            expiry => Some(RefusedSignIn {
                expires_at: Some(at_ticks(expiry)),
            }),
        },
        misses,
    }
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

/// One request as a sender was asked for it.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    url: String,
    headers: Vec<(String, String)>,
}

/// Stands in for the network in the tests of the two clients: it remembers what it was asked,
/// and answers as the test says. No request leaves the test.
struct FakeSender {
    respond: Box<dyn Fn() -> Result<Answer, NoAnswer> + Send + Sync>,
    seen: Mutex<Vec<Seen>>,
}

impl FakeSender {
    fn new(respond: impl Fn() -> Result<Answer, NoAnswer> + Send + Sync + 'static) -> FakeSender {
        FakeSender {
            respond: Box::new(respond),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

impl Sender for FakeSender {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Answer, NoAnswer> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        self.seen.lock().unwrap().push(Seen {
            url: url.to_string(),
            headers,
        });
        (self.respond)()
    }
}

/// An answer with a status and a body.
fn reply(status: u16, body: &str) -> Result<Answer, NoAnswer> {
    reply_bytes(status, body.as_bytes())
}

fn reply_bytes(status: u16, body: &[u8]) -> Result<Answer, NoAnswer> {
    Ok(Answer {
        status,
        retry_after: None,
        body: body.to_vec(),
    })
}

/// A 429 that says how long to wait.
fn too_many_requests(retry_after: &str) -> Result<Answer, NoAnswer> {
    Ok(Answer {
        status: 429,
        retry_after: Some(retry_after.to_string()),
        body: Vec::new(),
    })
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
