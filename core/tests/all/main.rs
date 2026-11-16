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

mod accent_palette;
mod alert_balloon;
mod alert_policy;
mod alert_state;
mod app_config;
mod app_paths;
mod atomic_file;
mod auto_start_command;
mod bar_colour;
mod bash_locator;
mod bridge_command;
mod bridge_config;
mod bridge_install;
mod bridge_locator;
mod bridge_refresh;
mod bridge_runner;
mod claude_credentials;
mod claude_paths;
mod claude_provider;
mod connect_message;
mod connection;
mod contrast;
mod copilot_cache;
mod copilot_cli_credential;
mod copilot_poll;
mod copilot_provider;
mod copilot_shown;
mod copilot_state_text;
mod copilot_token;
mod copilot_usage_client;
mod copilot_user_json;
mod csharp_files;
mod diagnostics_log;
mod duration;
mod elsewhere_connection;
mod flyout_model;
mod flyout_placement;
mod full_screen;
mod gh_cli;
mod glyph_fill;
mod http;
mod limit_list;
mod limit_status;
mod live_poll;
mod live_schedule;
mod live_state;
mod live_usage_client;
mod log_budget;
mod log_entry;
mod pace;
mod pill_model;
mod pill_placement;
mod pill_show;
mod pill_stale;
mod provider_poller;
mod rate_limits_json;
mod reset_period;
mod rings_model;
mod settings_json;
mod status_line_text;
mod taskbar_colours;
mod taskbar_layout;
mod taskbar_query_schedule;
mod tooltip_text;
mod usage_api_json;
mod usage_file;
mod usage_level;
mod usage_merge;
mod when;
mod working_time;
mod zone;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use sobreiro_core::display::{ClockFormat, Zone, ZoneChange};
use sobreiro_core::io::http::{Answer, NoAnswer, Sender};
use sobreiro_core::limits::{Limit, LimitKind};
use sobreiro_core::live::live_schedule::{LiveSchedule, RefusedSignIn};
use sobreiro_core::polling::ProviderState;
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

/// A span given in ticks, the way the C# counts them: ten-millionths of a second.
fn span(ticks: i64) -> TimeDelta {
    TimeDelta::new(ticks.div_euclid(S), (ticks.rem_euclid(S) * 100) as u32).unwrap()
}

/// The offset of a zone that is so many seconds east of UTC.
fn east(seconds: i32) -> FixedOffset {
    FixedOffset::east_opt(seconds).unwrap()
}

/// How a clock is written with the culture en-US, as .NET 10 on this PC gives it.
fn en_us() -> ClockFormat {
    clock("h:mm tt", "AM", "PM")
}

/// How a clock is written with the culture en-GB.
fn en_gb() -> ClockFormat {
    clock("HH:mm", "am", "pm")
}

/// How a clock is written with the culture pt-PT.
fn pt_pt() -> ClockFormat {
    clock("HH:mm", "da manh\u{e3}", "da tarde")
}

fn clock(pattern: &str, am: &str, pm: &str) -> ClockFormat {
    ClockFormat {
        pattern: pattern.to_string(),
        am: am.to_string(),
        pm: pm.to_string(),
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

/// The zones that the rows of the `..._as_the_csharp_did` tests name, as the C# printed their
/// changes from 2025-12-01 to 2027-02-01.
fn oracle_zone(name: &str) -> Zone {
    match name {
        "UTC" => Zone::utc(),
        "+09:00" => Zone::fixed(east(32400)),
        "-05:00" => Zone::fixed(east(-18000)),
        "+05:45" => Zone::fixed(east(20700)),
        "Europe/Lisbon" => Zone::new(
            east(0),
            vec![
                ZoneChange {
                    at: time("2026-03-29T01:00:00.0000000+00:00"),
                    offset: east(3600),
                },
                ZoneChange {
                    at: time("2026-10-25T01:00:00.0000000+00:00"),
                    offset: east(0),
                },
            ],
        ),
        "America/Santiago" => Zone::new(
            east(-10800),
            vec![
                ZoneChange {
                    at: time("2026-04-05T03:00:00.0000000+00:00"),
                    offset: east(-14400),
                },
                ZoneChange {
                    at: time("2026-09-06T04:00:00.0000000+00:00"),
                    offset: east(-10800),
                },
            ],
        ),
        "America/Havana" => Zone::new(
            east(-14400),
            vec![
                ZoneChange {
                    at: time("2025-11-02T05:00:00.0000000+00:00"),
                    offset: east(-18000),
                },
                ZoneChange {
                    at: time("2026-03-08T05:00:00.0000000+00:00"),
                    offset: east(-14400),
                },
            ],
        ),
        "Australia/Lord_Howe" => Zone::new(
            east(39600),
            vec![
                ZoneChange {
                    at: time("2026-04-04T15:00:00.0000000+00:00"),
                    offset: east(37800),
                },
                ZoneChange {
                    at: time("2026-10-03T15:30:00.0000000+00:00"),
                    offset: east(39600),
                },
            ],
        ),
        _ => panic!("no zone named {name}"),
    }
}

/// The names `oracle_zone` knows.
const ORACLE_ZONES: [&str; 8] = [
    "UTC",
    "+09:00",
    "-05:00",
    "+05:45",
    "Europe/Lisbon",
    "America/Santiago",
    "America/Havana",
    "Australia/Lord_Howe",
];

/// How a made-up or system culture is written, as the rows of the `..._as_the_csharp_did` tests
/// of the models name them. The last four are made up: a pattern with a long quoted text, and
/// designators that are long, non-ASCII or outside the first plane, so that texts pass the
/// tooltip's 127 units.
fn oracle_clock(name: &str) -> ClockFormat {
    match name {
        "en-US" => en_us(),
        "en-GB" => en_gb(),
        "pt-PT" => pt_pt(),
        "long" => clock(
            "HH:mm:ss 'oclock and then some more words to make it long'",
            "AM",
            "PM",
        ),
        "wide" => clock(
            "h:mm tt",
            "\u{c0}\u{c9}\u{ce}\u{d5}\u{dc} ante meridiem do dia",
            "post meridiem do dia \u{1f307}",
        ),
        "astral" => clock(
            "tt h:mm",
            "\u{1f305}\u{1f305}\u{1f305}\u{1f305}\u{1f305}\u{1f305}",
            "\u{1f307}\u{1f307}\u{1f307}\u{1f307}\u{1f307}\u{1f307}",
        ),
        "astral2" => clock(
            "tt h:mm",
            "x\u{1f305}\u{1f305}\u{1f305}\u{1f305}\u{1f305}",
            "y\u{1f307}\u{1f307}\u{1f307}\u{1f307}\u{1f307}",
        ),
        _ => panic!("no clock named {name}"),
    }
}

/// A provider state by the name of its C# enum member.
fn state_of(name: &str) -> ProviderState {
    match name {
        "Off" => ProviderState::Off,
        "Ok" => ProviderState::Ok,
        "Expired" => ProviderState::Expired,
        "NoSignIn" => ProviderState::NoSignIn,
        "Refused" => ProviderState::Refused,
        "RateLimited" => ProviderState::RateLimited,
        "Offline" => ProviderState::Offline,
        "BadAnswer" => ProviderState::BadAnswer,
        _ => panic!("no state named {name}"),
    }
}

/// A limit as the rows of the model tests hold it: the kind (0 session, 1 weekly, 2 spend,
/// 3 credits), the percentage, when it resets, when it was read, whether it is reset, the window,
/// the dollars used and the cap, the period (empty for none), and the credits used and the budget.
type LimitRow = (
    u8,
    f64,
    i64,
    i64,
    bool,
    i64,
    Option<f64>,
    Option<f64>,
    &'static str,
    Option<f64>,
    Option<f64>,
);

fn limit_of(row: &LimitRow) -> Limit {
    let (
        kind,
        percent,
        resets,
        observed,
        is_reset,
        window,
        used_usd,
        limit_usd,
        period,
        used,
        budget,
    ) = *row;
    let (kind, label) = match kind {
        0 => (LimitKind::Session, "Session"),
        1 => (LimitKind::Weekly, "Weekly"),
        2 => (LimitKind::Spend, "Spend"),
        _ => (LimitKind::Credits, "AI credits"),
    };
    let provider = if kind == LimitKind::Credits {
        "GitHub Copilot"
    } else {
        "Claude Code"
    };
    Limit {
        window: (window != NONE).then(|| span(window)),
        used_usd,
        limit_usd,
        period: (!period.is_empty()).then(|| period.to_string()),
        used_credits: used,
        credit_budget: budget,
        ..Limit::new(
            provider,
            kind,
            label,
            percent,
            maybe_at(resets),
            at_ticks(observed),
            is_reset,
        )
    }
}

fn limits_of(rows: &[LimitRow]) -> Vec<Limit> {
    rows.iter().map(limit_of).collect()
}
