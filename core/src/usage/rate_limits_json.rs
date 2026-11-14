//! Reads the limits out of Claude Code's status line JSON and out of `usage.json`, and writes
//! `usage.json`.
//!
//! Only the fields named here are ever read or written. Whatever else the status line JSON holds
//! (the session, the folder, the transcript's path) goes nowhere.
use std::fmt;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::usage::{LimitWindow, RateLimits, SpendLimit, UsageSnapshot};

/// Reads what Claude Code gives a status line command. `None` when it is not JSON or holds no
/// usable `rate_limits`.
pub fn parse_status_line(json: &str) -> Option<RateLimits> {
    let document: Value = serde_json::from_str(json).ok()?;
    let rate_limits = document.as_object()?.get("rate_limits")?;
    // The status line says nothing of when a window was read; a field of that name is not ours.
    parse_rate_limits(rate_limits, WindowTimes::Ignore)
}

/// Reads `usage.json`. `None` without `observed_at` or without usable `rate_limits`.
pub fn parse_usage_file(json: &str) -> Option<UsageSnapshot> {
    let document: Value = serde_json::from_str(json).ok()?;
    let root = document.as_object()?;
    let observed_at = unix_time(root.get("observed_at")?)?;
    let limits = parse_rate_limits(root.get("rate_limits")?, WindowTimes::Read)?;
    Some(UsageSnapshot {
        limits,
        observed_at,
    })
}

/// A percentage or an amount that JSON has no way to write: infinity, or the result of a division
/// of zero by zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotFinite;

impl fmt::Display for NotFinite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number that JSON cannot hold")
    }
}

impl std::error::Error for NotFinite {}

/// The text of `usage.json` for a snapshot: one line, no spaces, the fields in a fixed order,
/// exactly as 1.x wrote it. Times are whole seconds since 1970.
pub fn write_usage_file(snapshot: &UsageSnapshot) -> Result<String, NotFinite> {
    let limits = &snapshot.limits;
    let mut windows = Vec::new();
    if let Some(five) = &limits.five_hour {
        windows.push(field("five_hour", window_json(five)?));
    }
    if let Some(seven) = &limits.seven_day {
        windows.push(field("seven_day", window_json(seven)?));
    }
    if let Some(spend) = &limits.spend {
        windows.push(field("spend_limit", spend_json(spend)?));
    }
    Ok(object(&[
        field("observed_at", seconds(snapshot.observed_at)),
        field("rate_limits", object(&windows)),
    ]))
}

fn window_json(window: &LimitWindow) -> Result<String, NotFinite> {
    let mut fields = vec![field("used_percentage", number(window.used_percentage)?)];
    if let Some(resets_at) = window.resets_at {
        fields.push(field("resets_at", seconds(resets_at)));
    }
    if let Some(observed_at) = window.observed_at {
        fields.push(field("observed_at", seconds(observed_at)));
    }
    Ok(object(&fields))
}

fn spend_json(spend: &SpendLimit) -> Result<String, NotFinite> {
    let mut fields = vec![field("used_percentage", number(spend.used_percentage)?)];
    if let Some(resets_at) = spend.resets_at {
        fields.push(field("resets_at", seconds(resets_at)));
    }
    if let Some(used_usd) = spend.used_usd {
        fields.push(field("used_usd", number(used_usd)?));
    }
    if let Some(limit_usd) = spend.limit_usd {
        fields.push(field("limit_usd", number(limit_usd)?));
    }
    if let Some(period) = &spend.period {
        fields.push(field("period", text(period)));
    }
    if let Some(observed_at) = spend.observed_at {
        fields.push(field("observed_at", seconds(observed_at)));
    }
    Ok(object(&fields))
}

/// `"name":value`, the value being JSON already.
fn field(name: &str, value: String) -> String {
    format!("\"{name}\":{value}")
}

/// `{...}` around fields, with commas between them.
fn object(fields: &[String]) -> String {
    format!("{{{}}}", fields.join(","))
}

/// A number the way 1.x wrote it: the shortest digits that read back as the same number, and no
/// ".0" on a whole one (23.5, 314.12, 500).
///
/// The two part ways only on a number far too small to be a percentage or an amount: a millionth
/// is `0.000001` here and was `1E-06` there. Both read back as the same number in both.
fn number(value: f64) -> Result<String, NotFinite> {
    if value.is_finite() {
        Ok(value.to_string())
    } else {
        Err(NotFinite)
    }
}

/// A time as whole seconds since 1970.
fn seconds(time: DateTime<Utc>) -> String {
    time.timestamp().to_string()
}

/// A text in quotes, with the characters JSON wants escaped.
fn text(value: &str) -> String {
    Value::from(value).to_string()
}

/// Whether to read the `observed_at` inside each window. Sobreiro writes that field in
/// `usage.json`; in the status line's JSON it would be someone else's.
#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowTimes {
    Read,
    Ignore,
}

fn parse_rate_limits(rate_limits: &Value, times: WindowTimes) -> Option<RateLimits> {
    if !rate_limits.is_object() {
        return None;
    }
    let five_hour = parse_window(rate_limits, "five_hour", times);
    let seven_day = parse_window(rate_limits, "seven_day", times);
    let spend = parse_spend(rate_limits, times);
    if five_hour.is_none() && seven_day.is_none() && spend.is_none() {
        return None;
    }
    Some(RateLimits {
        five_hour,
        seven_day,
        spend,
    })
}

/// A window counts only when it is an object with a number for `used_percentage`.
fn parse_window(parent: &Value, name: &str, times: WindowTimes) -> Option<LimitWindow> {
    let window = parent.get(name)?;
    Some(LimitWindow {
        used_percentage: number_in(window, "used_percentage")?,
        resets_at: time_in(window, "resets_at"),
        observed_at: observed_at_in(window, times),
    })
}

fn parse_spend(parent: &Value, times: WindowTimes) -> Option<SpendLimit> {
    let spend = parent.get("spend_limit")?;
    // Only the three periods Claude Code names are kept; any other word is read as none.
    let period = spend
        .get("period")
        .and_then(Value::as_str)
        .filter(|period| matches!(*period, "daily" | "weekly" | "monthly"))
        .map(str::to_string);
    Some(SpendLimit {
        used_percentage: number_in(spend, "used_percentage")?,
        resets_at: time_in(spend, "resets_at"),
        used_usd: number_in(spend, "used_usd"),
        limit_usd: number_in(spend, "limit_usd"),
        period,
        observed_at: observed_at_in(spend, times),
    })
}

fn observed_at_in(window: &Value, times: WindowTimes) -> Option<DateTime<Utc>> {
    match times {
        WindowTimes::Read => time_in(window, "observed_at"),
        WindowTimes::Ignore => None,
    }
}

/// The number under `name`. `None` when `parent` is not an object, the name is missing, or its
/// value is anything but a number (a number in quotes is a text).
fn number_in(parent: &Value, name: &str) -> Option<f64> {
    parent.as_object()?.get(name)?.as_f64()
}

/// The time under `name`, read as seconds since 1970.
fn time_in(parent: &Value, name: &str) -> Option<DateTime<Utc>> {
    unix_time(parent.as_object()?.get(name)?)
}

/// The first and the last second the C# of 1.x could hold: the start of the year 1 and the end
/// of the year 9999. A time outside them is read as no time, as it was there.
const FIRST_SECOND: f64 = -62_135_596_800.0;
const LAST_SECOND: f64 = 253_402_300_799.0;

/// A JSON number as a time: seconds since 1970, a fraction cut off.
fn unix_time(value: &Value) -> Option<DateTime<Utc>> {
    let seconds = value.as_f64()?.trunc();
    if !(FIRST_SECOND..=LAST_SECOND).contains(&seconds) {
        return None;
    }
    // Whole and within the range above, so `as i64` loses nothing.
    DateTime::from_timestamp(seconds as i64, 0)
}
