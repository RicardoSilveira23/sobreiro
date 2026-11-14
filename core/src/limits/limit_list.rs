//! Turns readings into the list of limits to show.
use chrono::{DateTime, Months, TimeDelta, Utc};

use crate::connect::ConnectionState;
use crate::copilot::CopilotReading;
use crate::limits::{Limit, LimitKind, providers};
use crate::usage::UsageSnapshot;

/// Claude Code's limits, in the order they are shown: session, weekly, spend. Those the snapshot
/// does not hold are left out.
pub fn from(snapshot: Option<&UsageSnapshot>, now: DateTime<Utc>) -> Vec<Limit> {
    let Some(snapshot) = snapshot else {
        return Vec::new();
    };
    let limits = &snapshot.limits;
    // A window without a time of its own was read when the file was written.
    let seen = |own: Option<DateTime<Utc>>| own.unwrap_or(snapshot.observed_at);
    let mut list = Vec::new();

    if let Some(five) = &limits.five_hour {
        list.push(Limit {
            window: Some(TimeDelta::hours(5)),
            ..claude_limit(
                LimitKind::Session,
                "Session",
                five.used_percentage,
                five.resets_at,
                seen(five.observed_at),
                now,
            )
        });
    }
    if let Some(seven) = &limits.seven_day {
        list.push(Limit {
            window: Some(TimeDelta::days(7)),
            ..claude_limit(
                LimitKind::Weekly,
                "Weekly",
                seven.used_percentage,
                seven.resets_at,
                seen(seven.observed_at),
                now,
            )
        });
    }
    if let Some(spend) = &limits.spend {
        list.push(Limit {
            window: spend_window(spend.period.as_deref(), spend.resets_at),
            used_usd: spend.used_usd,
            limit_usd: spend.limit_usd,
            period: spend.period.clone(),
            ..claude_limit(
                LimitKind::Spend,
                "Spend",
                spend.used_percentage,
                spend.resets_at,
                seen(spend.observed_at),
                now,
            )
        });
    }
    list
}

/// GitHub Copilot's one limit: the month's AI credits. Empty without a reading.
pub fn from_copilot(reading: Option<&CopilotReading>, now: DateTime<Utc>) -> Vec<Limit> {
    let Some(reading) = reading else {
        return Vec::new();
    };
    let is_reset = has_passed(reading.resets_at, now);
    let percent = if is_reset { 0.0 } else { reading.percent() };
    vec![Limit {
        window: reading.resets_at.and_then(month_ending_at),
        used_credits: Some(reading.used),
        credit_budget: Some(reading.budget),
        ..Limit::new(
            providers::COPILOT,
            LimitKind::Credits,
            "AI credits",
            percent,
            reading.resets_at,
            reading.observed_at,
            is_reset,
        )
    }]
}

/// Claude is shown when it is on this PC, or when nothing else is shown.
pub fn show_claude(claude_on_pc: bool, copilot_shown: bool) -> bool {
    claude_on_pc || !copilot_shown
}

/// Claude Code counts as being on this PC when its credentials file is there, when there is a
/// reading of its limits, or when its status line is connected in any way.
pub fn claude_on_pc(
    credentials_file_exists: bool,
    claude_limit_count: usize,
    connection: ConnectionState,
) -> bool {
    credentials_file_exists || claude_limit_count > 0 || connection != ConnectionState::NotConnected
}

/// One of Claude Code's limits. Once its reset time has passed it reads zero and says so.
fn claude_limit(
    kind: LimitKind,
    label: &'static str,
    percent: f64,
    resets_at: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Limit {
    let is_reset = has_passed(resets_at, now);
    let percent = if is_reset { 0.0 } else { percent };
    Limit::new(
        providers::CLAUDE,
        kind,
        label,
        percent,
        resets_at,
        observed_at,
        is_reset,
    )
}

/// Whether there is a reset time and it is now or earlier.
fn has_passed(resets_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    resets_at.is_some_and(|reset| reset <= now)
}

/// How long the spend limit's period is. A month has no fixed length, so it is measured back
/// from the reset time; any other word, or no word, gives no length.
fn spend_window(period: Option<&str>, resets_at: Option<DateTime<Utc>>) -> Option<TimeDelta> {
    match period {
        Some("daily") => Some(TimeDelta::days(1)),
        Some("weekly") => Some(TimeDelta::days(7)),
        Some("monthly") => resets_at.and_then(month_ending_at),
        _ => None,
    }
}

/// The length of the month that ends at `end`: from the same day and hour one month earlier.
fn month_ending_at(end: DateTime<Utc>) -> Option<TimeDelta> {
    let start = end.checked_sub_months(Months::new(1))?;
    Some(end - start)
}
