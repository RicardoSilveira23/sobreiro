//! Ported from `tests/Sobreiro.Tests/UsageMergeTests.cs`.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::usage::usage_merge::{self, ReadingFrom};
use sobreiro_core::usage::{LimitWindow, RateLimits, SpendLimit, UsageSnapshot};

use crate::{spend, spend_seen, time, window, window_seen};

/// Now, in every test here.
fn t() -> DateTime<Utc> {
    time("2026-11-09T10:00:00Z")
}

fn session_reset() -> DateTime<Utc> {
    t() + TimeDelta::hours(3)
}

fn weekly_reset() -> DateTime<Utc> {
    t() + TimeDelta::days(4)
}

fn five_minutes_ago() -> DateTime<Utc> {
    t() - TimeDelta::minutes(5)
}

/// What `usage.json` held, read five minutes ago.
fn stored(
    five_hour: Option<LimitWindow>,
    seven_day: Option<LimitWindow>,
    spend: Option<SpendLimit>,
) -> UsageSnapshot {
    stored_at(five_hour, seven_day, spend, five_minutes_ago())
}

fn stored_at(
    five_hour: Option<LimitWindow>,
    seven_day: Option<LimitWindow>,
    spend: Option<SpendLimit>,
    observed_at: DateTime<Utc>,
) -> UsageSnapshot {
    UsageSnapshot {
        limits: incoming(five_hour, seven_day, spend),
        observed_at,
    }
}

/// A new reading.
fn incoming(
    five_hour: Option<LimitWindow>,
    seven_day: Option<LimitWindow>,
    spend: Option<SpendLimit>,
) -> RateLimits {
    RateLimits {
        five_hour,
        seven_day,
        spend,
    }
}

/// Merges a status line reading at `t()`.
fn merge(stored: &UsageSnapshot, incoming: &RateLimits) -> UsageSnapshot {
    usage_merge::merge(Some(stored), incoming, t(), ReadingFrom::StatusLine)
}

/// Merges a reading from Live usage at `t()`: the C# tests' `authoritative: true`.
fn merge_live(stored: &UsageSnapshot, incoming: &RateLimits) -> UsageSnapshot {
    usage_merge::merge(Some(stored), incoming, t(), ReadingFrom::LiveUsage)
}

#[test]
fn merge_with_nothing_stored_takes_reading_and_stamps_now() {
    let reading = incoming(
        Some(window(10.0, Some(session_reset()))),
        Some(window(20.0, Some(weekly_reset()))),
        Some(spend(
            30.0,
            Some(weekly_reset()),
            Some(3.0),
            Some(10.0),
            Some("monthly"),
        )),
    );

    let merged = usage_merge::merge(None, &reading, t(), ReadingFrom::StatusLine);

    assert_eq!(merged.observed_at, t());
    assert_eq!(merged.limits.five_hour.unwrap().observed_at, Some(t()));
    assert_eq!(merged.limits.seven_day.unwrap().observed_at, Some(t()));
    assert_eq!(merged.limits.spend.unwrap().observed_at, Some(t()));
}

#[test]
fn merge_keeps_higher_reading_in_same_period() {
    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        None,
        None,
    );
    let ten_seconds_later = session_reset() + TimeDelta::seconds(10);

    let merged = merge(
        &before,
        &incoming(Some(window(40.0, Some(ten_seconds_later))), None, None),
    );

    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago()))
    );
    assert_eq!(merged.observed_at, t());
}

#[test]
fn merge_takes_equal_or_higher_reading_in_same_period() {
    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        None,
        None,
    );

    let merged = merge(
        &before,
        &incoming(Some(window(60.0, Some(session_reset()))), None, None),
    );

    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(60.0, Some(session_reset()), t()))
    );
}

#[test]
fn merge_takes_lower_reading_from_a_later_period() {
    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        None,
        None,
    );
    let later = t() + TimeDelta::hours(8);

    let merged = merge(
        &before,
        &incoming(Some(window(5.0, Some(later))), None, None),
    );

    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(5.0, Some(later), t()))
    );
}

#[test]
fn merge_ignores_reading_from_an_earlier_period() {
    let later = t() + TimeDelta::hours(8);
    let before = stored(
        Some(window_seen(5.0, Some(later), five_minutes_ago())),
        None,
        None,
    );

    let merged = merge(
        &before,
        &incoming(Some(window(70.0, Some(session_reset()))), None, None),
    );

    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(5.0, Some(later), five_minutes_ago()))
    );
}

#[test]
fn merge_takes_reading_when_either_reset_time_is_missing() {
    let with_reset = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        None,
        None,
    );
    let merged = merge(&with_reset, &incoming(Some(window(10.0, None)), None, None));
    assert_eq!(merged.limits.five_hour, Some(window_seen(10.0, None, t())));

    let without_reset = stored(
        Some(window_seen(60.0, None, five_minutes_ago())),
        None,
        None,
    );
    let merged = merge(
        &without_reset,
        &incoming(Some(window(10.0, Some(session_reset()))), None, None),
    );
    assert_eq!(
        merged.limits.five_hour,
        Some(window_seen(10.0, Some(session_reset()), t()))
    );
}

#[test]
fn merge_keeps_stored_window_missing_from_reading_until_its_reset_passes() {
    let reading = incoming(Some(window(61.0, Some(session_reset()))), None, None);

    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        Some(window_seen(30.0, Some(weekly_reset()), five_minutes_ago())),
        None,
    );
    assert_eq!(
        merge(&before, &reading).limits.seven_day,
        Some(window_seen(30.0, Some(weekly_reset()), five_minutes_ago()))
    );

    let expired = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        Some(window_seen(30.0, Some(t()), five_minutes_ago())),
        None,
    );
    assert_eq!(merge(&expired, &reading).limits.seven_day, None);
}

#[test]
fn merge_fills_window_observed_at_from_old_snapshot() {
    let an_hour_ago = t() - TimeDelta::hours(1);
    let before = stored_at(
        Some(window(60.0, Some(session_reset()))),
        Some(window(30.0, Some(weekly_reset()))),
        None,
        an_hour_ago,
    );

    let merged = merge(
        &before,
        &incoming(Some(window(10.0, Some(session_reset()))), None, None),
    );

    // Kept: lower, in the same period.
    assert_eq!(
        merged.limits.five_hour.unwrap().observed_at,
        Some(an_hour_ago)
    );
    // Kept: absent from the reading.
    assert_eq!(
        merged.limits.seven_day.unwrap().observed_at,
        Some(an_hour_ago)
    );
}

/// A spend limit of the month, capped at `limit_usd` dollars, resetting at `weekly_reset()` and
/// read five minutes ago.
fn stored_spend(used_percentage: f64, used_usd: f64, limit_usd: f64) -> SpendLimit {
    spend_seen(
        used_percentage,
        Some(weekly_reset()),
        Some(used_usd),
        Some(limit_usd),
        Some("monthly"),
        five_minutes_ago(),
    )
}

#[test]
fn merge_applies_the_same_rule_to_spend() {
    let before = stored(None, None, Some(stored_spend(60.0, 6.0, 10.0)));
    let reading = spend(
        40.0,
        Some(weekly_reset()),
        Some(4.0),
        Some(10.0),
        Some("weekly"),
    );

    let merged = merge(&before, &incoming(None, None, Some(reading)));

    assert_eq!(merged.limits.spend, Some(stored_spend(60.0, 6.0, 10.0)));
}

#[test]
fn merge_takes_spend_whose_cap_was_raised_in_the_same_period() {
    let before = stored(None, None, Some(stored_spend(90.0, 45.0, 50.0)));
    let reading = spend(
        45.0,
        Some(weekly_reset()),
        Some(45.0),
        Some(100.0),
        Some("monthly"),
    );

    let merged = merge(&before, &incoming(None, None, Some(reading)));

    assert_eq!(
        merged.limits.spend,
        Some(spend_seen(
            45.0,
            Some(weekly_reset()),
            Some(45.0),
            Some(100.0),
            Some("monthly"),
            t()
        ))
    );
}

#[test]
fn merge_compares_spend_in_dollars_when_the_cap_is_the_same() {
    let before = stored(None, None, Some(stored_spend(60.0, 30.0, 50.0)));

    let fewer_dollars = spend(
        61.0,
        Some(weekly_reset()),
        Some(29.0),
        Some(50.0),
        Some("monthly"),
    );
    let merged = merge(&before, &incoming(None, None, Some(fewer_dollars)));
    assert_eq!(merged.limits.spend, before.limits.spend);

    let more_dollars = spend(
        59.0,
        Some(weekly_reset()),
        Some(31.0),
        Some(50.0),
        Some("monthly"),
    );
    let merged = merge(&before, &incoming(None, None, Some(more_dollars)));
    assert_eq!(
        merged.limits.spend,
        Some(spend_seen(
            59.0,
            Some(weekly_reset()),
            Some(31.0),
            Some(50.0),
            Some("monthly"),
            t()
        ))
    );
}

#[test]
fn merge_keeps_stored_spend_missing_from_reading_until_its_reset_passes() {
    let reading = incoming(Some(window(10.0, Some(session_reset()))), None, None);

    let before = stored(None, None, Some(stored_spend(90.0, 45.0, 50.0)));
    assert_eq!(merge(&before, &reading).limits.spend, before.limits.spend);

    let expired_spend = spend_seen(
        90.0,
        Some(t()),
        Some(45.0),
        Some(50.0),
        Some("monthly"),
        five_minutes_ago(),
    );
    let expired = stored(None, None, Some(expired_spend));
    assert_eq!(merge(&expired, &reading).limits.spend, None);
}

#[test]
fn merge_takes_lower_spend_from_a_later_period() {
    let before = stored(None, None, Some(stored_spend(90.0, 45.0, 50.0)));
    let later = weekly_reset() + TimeDelta::days(30);
    let reading = spend(4.0, Some(later), Some(2.0), Some(50.0), Some("monthly"));

    let merged = merge(&before, &incoming(None, None, Some(reading)));

    assert_eq!(
        merged.limits.spend,
        Some(spend_seen(
            4.0,
            Some(later),
            Some(2.0),
            Some(50.0),
            Some("monthly"),
            t()
        ))
    );
}

#[test]
fn an_authoritative_reading_replaces_a_higher_one_in_the_same_period() {
    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        Some(window_seen(40.0, Some(weekly_reset()), five_minutes_ago())),
        None,
    );
    let ten_seconds_later = session_reset() + TimeDelta::seconds(10);
    let reading = incoming(
        Some(window(12.0, Some(ten_seconds_later))),
        Some(window(39.0, Some(weekly_reset()))),
        None,
    );

    let live = merge_live(&before, &reading);
    assert_eq!(
        live.limits.five_hour,
        Some(window_seen(12.0, Some(ten_seconds_later), t()))
    );
    assert_eq!(
        live.limits.seven_day,
        Some(window_seen(39.0, Some(weekly_reset()), t()))
    );

    let status_line = merge(&before, &reading);
    assert_eq!(status_line.limits.five_hour, before.limits.five_hour);
    assert_eq!(status_line.limits.seven_day, before.limits.seven_day);
}

#[test]
fn an_authoritative_reading_still_ignores_an_earlier_period() {
    let later = t() + TimeDelta::hours(8);
    let before = stored(
        Some(window_seen(5.0, Some(later), five_minutes_ago())),
        None,
        None,
    );

    let merged = merge_live(
        &before,
        &incoming(Some(window(70.0, Some(session_reset()))), None, None),
    );

    assert_eq!(merged.limits.five_hour, before.limits.five_hour);
}

#[test]
fn an_authoritative_reading_leaves_spend_to_its_own_rule() {
    let kept = stored_spend(60.0, 30.0, 50.0);
    let before = stored(
        Some(window_seen(60.0, Some(session_reset()), five_minutes_ago())),
        None,
        Some(kept.clone()),
    );

    let without_spend = incoming(Some(window(10.0, Some(session_reset()))), None, None);
    assert_eq!(
        merge_live(&before, &without_spend).limits.spend,
        Some(kept.clone())
    );

    let fewer_dollars = incoming(
        Some(window(10.0, Some(session_reset()))),
        None,
        Some(spend(
            50.0,
            Some(weekly_reset()),
            Some(25.0),
            Some(50.0),
            Some("monthly"),
        )),
    );
    assert_eq!(merge_live(&before, &fewer_dollars).limits.spend, Some(kept));
}
