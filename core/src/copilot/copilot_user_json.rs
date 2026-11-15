//! Reads the reply of `GET https://api.github.com/copilot_internal/user`.
//!
//! The AI credits sit in `quota_snapshots.premium_models` when that is an object, else in
//! `quota_snapshots.premium_interactions`. Every field is optional, and a field of the wrong
//! type counts as absent. Anything else in the reply is ignored.
use chrono::{DateTime, TimeDelta, Utc};

use super::{CopilotReading, CopilotSource};
use crate::connect::json_reader::{self, Dialect, Kind, Member};
use crate::live::usage_api_json::{HalfACharacter, time_in};

/// What a reply holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    /// A budget of AI credits and what is used of it.
    Reading(CopilotReading),
    /// An account's reply with no budget to show: a free plan, a plan without a limit, or a
    /// reply that does not say what is used.
    NoBudget,
    /// Not a reply that can be read: not JSON, JSON that is not an object, or a time in it
    /// that spells out half a character.
    NoUse,
}

/// What the reply holds. `now` is the moment the reading is taken to be from when the reply
/// does not name one.
pub fn parse(body: &str, now: DateTime<Utc>) -> Reply {
    let Ok(root) = json_reader::read(body, Dialect::Strict) else {
        return Reply::NoUse;
    };
    let Kind::Object(response) = &root.kind else {
        return Reply::NoUse;
    };
    match from_response(response, now, CopilotSource::Live) {
        Ok(Some(reading)) => Reply::Reading(reading),
        Ok(None) => Reply::NoBudget,
        // 1.x stopped with an error where it had to read such a text, and the request counted
        // as failed.
        Err(HalfACharacter) => Reply::NoUse,
    }
}

/// How far from the reading a reset may be. A date further off than this is nonsense.
const FURTHEST_RESET: TimeDelta = TimeDelta::days(400);

/// The reading in a reply that is a JSON object, whose members are `response`. `Ok(None)`
/// when it has no budget to show.
fn from_response(
    response: &[Member],
    fallback_observed_at: DateTime<Utc>,
    source: CopilotSource,
) -> Result<Option<CopilotReading>, HalfACharacter> {
    let Some(Kind::Object(snapshots)) = json_reader::member(response, "quota_snapshots") else {
        return Ok(None);
    };
    // The first of the two that is an object counts, with or without a budget in it.
    let snapshot =
        object(snapshots, "premium_models").or_else(|| object(snapshots, "premium_interactions"));
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };

    if matches!(json_reader::member(snapshot, "unlimited"), Some(Kind::True)) {
        return Ok(None);
    }
    let Some(budget) = number(snapshot, "entitlement") else {
        return Ok(None);
    };
    if budget <= 0.0 {
        return Ok(None);
    }

    let Some(mut used) = used(snapshot, budget) else {
        return Ok(None);
    };
    // Past the budget the reply counts what is over in a field of its own.
    if let Some(overage) = number(snapshot, "overage_count")
        && overage > 0.0
        && used <= budget
    {
        used = budget + overage;
    }

    let observed_at = time(snapshot, "timestamp_utc")?.unwrap_or(fallback_observed_at);
    let mut resets_at = time(response, "quota_reset_date_utc")?;
    // A reset date far from the reading is nonsense: it counts as absent.
    if let Some(reset) = resets_at
        && (reset - observed_at).abs() > FURTHEST_RESET
    {
        resets_at = None;
    }

    Ok(Some(CopilotReading {
        budget,
        used,
        resets_at,
        observed_at,
        source,
    }))
}

/// The members of the object under `key`, when what is there is an object.
fn object<'a>(members: &'a [Member<'a>], key: &str) -> Option<&'a [Member<'a>]> {
    match json_reader::member(members, key) {
        Some(Kind::Object(value)) => Some(value),
        _ => None,
    }
}

/// What is used of the budget: worked out from the first of three fields that says what is
/// left, or read from the one that says what is used.
fn used(snapshot: &[Member], budget: f64) -> Option<f64> {
    if let Some(left) = number(snapshot, "quota_remaining") {
        return Some(budget - left);
    }
    if let Some(left) = number(snapshot, "remaining") {
        return Some(budget - left);
    }
    if let Some(percent_left) = number(snapshot, "percent_remaining") {
        return Some(budget * (100.0 - percent_left) / 100.0);
    }
    number(snapshot, "credits_used")
}

/// The number under `key`, when what is there is a number.
fn number(object: &[Member], key: &str) -> Option<f64> {
    match json_reader::member(object, key) {
        // The reader has checked that this is a JSON number, and Rust reads every one of
        // those: a number too large to hold becomes infinity, as it did in 1.x.
        Some(Kind::Number(written)) => written.parse().ok(),
        _ => None,
    }
}

/// The time under `key`, when what is there is a text that reads as a time. A time that does
/// not say how far it is from UTC is no time, as in Claude Code's reply.
fn time(object: &[Member], key: &str) -> Result<Option<DateTime<Utc>>, HalfACharacter> {
    match json_reader::member(object, key) {
        Some(Kind::Text(written)) => time_in(written),
        _ => Ok(None),
    }
}
