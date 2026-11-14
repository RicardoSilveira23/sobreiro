// Ports of LimitListTests, UsageLevelTests, ResetPeriodTests, RateLimitsJsonTests, UsageMergeTests, UsageFileTests.
mod common;
use common::*;
use sobreiro_core::connect::ConnectionState;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::limits::*;
use sobreiro_core::usage::*;
use sobreiro_core::{Span, Time};
use std::os::windows::fs::OpenOptionsExt;

fn now() -> Time {
    t("2026-11-09T12:00:00Z")
}

fn snap(l: RateLimits) -> UsageSnapshot {
    UsageSnapshot { limits: l, observed_at: now() - mins(2) }
}

fn rl(five: Option<LimitWindow>, seven: Option<LimitWindow>, spend: Option<SpendLimit>) -> RateLimits {
    RateLimits { five_hour: five, seven_day: seven, spend }
}

fn spend(pct: f64, resets: Option<Time>, used: Option<f64>, limit: Option<f64>, period: Option<&str>) -> SpendLimit {
    SpendLimit { used_percentage: pct, resets_at: resets, used_usd: used, limit_usd: limit, period: period.map(String::from), observed_at: None }
}

// ---- LimitListTests

#[test]
fn from_null_is_empty() {
    assert!(LimitList::from(None, now()).is_empty());
}

#[test]
fn from_maps_windows_in_order_with_labels() {
    let five = now() + hours(2);
    let seven = now() + days(5);
    let sp = now() + days(7);
    let s = snap(rl(Some(win(8.0, Some(five))), Some(win(44.0, Some(seven))), Some(spend(62.8, Some(sp), Some(6.28), Some(10.0), Some("monthly")))));
    let list = LimitList::from(Some(&s), now());
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].kind, LimitKind::Session);
    assert_eq!(list[0].label, "Session");
    assert_eq!(list[0].provider, "Claude Code");
    assert_eq!(list[0].percent, 8.0);
    assert_eq!(list[0].resets_at, Some(five));
    assert!(!list[0].is_reset);
    assert_eq!(list[0].observed_at, now() - mins(2));
    assert_eq!((list[1].kind, list[1].label.as_str(), list[1].percent, list[1].resets_at), (LimitKind::Weekly, "Weekly", 44.0, Some(seven)));
    assert_eq!((list[2].kind, list[2].label.as_str(), list[2].percent, list[2].resets_at), (LimitKind::Spend, "Spend", 62.8, Some(sp)));
}

#[test]
fn from_marks_passed_window_as_reset_at_zero() {
    let resets = t("2026-11-09T11:00:00Z");
    let list = LimitList::from(Some(&snap(rl(Some(win(30.0, Some(resets))), None, None))), now());
    assert_eq!(list.len(), 1);
    assert!(list[0].is_reset);
    assert_eq!(list[0].percent, 0.0);
    assert_eq!(list[0].resets_at, Some(resets));
}

#[test]
fn from_skips_absent_windows() {
    let list = LimitList::from(Some(&snap(rl(None, Some(win(44.0, Some(now() + days(1)))), None))), now());
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind, LimitKind::Weekly);
}

#[test]
fn display_percent_floors_and_clamps() {
    for (percent, expected) in [(8.9, 8), (99.6, 99), (100.0, 100), (130.0, 100), (-5.0, 0)] {
        let l = Limit {
            provider: "Claude Code".into(), kind: LimitKind::Session, label: "Session".into(), percent, resets_at: None,
            observed_at: now(), is_reset: false, window: None, used_usd: None, limit_usd: None, period: None,
            used_credits: None, credit_budget: None,
        };
        assert_eq!(l.display_percent(), expected, "{percent}");
    }
}

#[test]
fn from_uses_the_windows_own_observed_at_when_it_has_one() {
    let own = now() - hours(3);
    let s = snap(rl(Some(win_at(10.0, Some(now() + hours(2)), own)), Some(win(20.0, Some(now() + days(2)))), None));
    let list = LimitList::from(Some(&s), now());
    assert_eq!(list[0].observed_at, own);
    assert_eq!(list[1].observed_at, now() - mins(2));
}

#[test]
fn from_fills_windows_and_spend_fields() {
    let resets = t("2026-12-01T00:00:00Z");
    let s = snap(rl(Some(win(8.0, Some(now() + hours(2)))), Some(win(44.0, Some(now() + days(5)))), Some(spend(63.0, Some(resets), Some(31.5), Some(50.0), Some("monthly")))));
    let list = LimitList::from(Some(&s), now());
    assert_eq!(list[0].window, Some(hours(5)));
    assert_eq!(list[1].window, Some(days(7)));
    assert_eq!(list[2].window, Some(days(30)));
    assert_eq!(list[2].used_usd, Some(31.5));
    assert_eq!(list[2].limit_usd, Some(50.0));
    assert_eq!(list[2].period.as_deref(), Some("monthly"));
}

#[test]
fn spend_window_follows_the_period() {
    for (period, d) in [(Some("daily"), Some(1)), (Some("weekly"), Some(7)), (Some("yearly"), None), (None, None)] {
        let s = snap(rl(None, None, Some(spend(10.0, Some(now() + days(1)), None, None, period))));
        let list = LimitList::from(Some(&s), now());
        assert_eq!(list[0].window, d.map(days), "{period:?}");
    }
}

fn reading(used: f64, resets: Option<Time>) -> CopilotReading {
    CopilotReading { budget: 15000.0, used, resets_at: resets, observed_at: now() - mins(1), source: CopilotSource::Live }
}

#[test]
fn copilot_reading_becomes_one_credits_limit() {
    let r = reading(2644.2, Some(t("2026-12-01T00:00:00Z")));
    let list = LimitList::from_copilot(Some(&r), now());
    assert_eq!(list.len(), 1);
    let l = &list[0];
    assert_eq!(l.provider, "GitHub Copilot");
    assert_eq!(l.kind, LimitKind::Credits);
    assert_eq!(l.label, "AI credits");
    assert!((l.percent - 17.628).abs() < 0.001);
    assert_eq!(l.used_credits, Some(2644.2));
    assert_eq!(l.credit_budget, Some(15000.0));
    assert_eq!(l.window, Some(days(30)));
    assert_eq!(l.resets_at, Some(t("2026-12-01T00:00:00Z")));
    assert!(!l.is_reset);
    assert_eq!(l.observed_at, now() - mins(1));
}

#[test]
fn no_copilot_reading_gives_no_limits() {
    assert!(LimitList::from_copilot(None, now()).is_empty());
}

#[test]
fn credits_without_reset_time_have_no_window() {
    let list = LimitList::from_copilot(Some(&reading(2644.2, None)), now());
    assert_eq!(list[0].window, None);
    assert!(!list[0].is_reset);
}

#[test]
fn passed_reset_reads_zero_and_reset() {
    let list = LimitList::from_copilot(Some(&reading(2644.2, Some(t("2026-11-01T00:00:00Z")))), now());
    assert_eq!(list[0].percent, 0.0);
    assert!(list[0].is_reset);
}

#[test]
fn claude_shows_when_on_pc_or_when_nothing_else_shows() {
    for (on_pc, copilot, expected) in [(true, true, true), (true, false, true), (false, false, true), (false, true, false)] {
        assert_eq!(LimitList::show_claude(on_pc, copilot), expected);
    }
}

#[test]
fn claude_is_on_pc_by_credentials_readings_or_connection() {
    use ConnectionState::*;
    for (file, count, state, expected) in [
        (false, 0, NotConnected, false), (true, 0, NotConnected, true), (false, 1, NotConnected, true),
        (false, 0, Connected, true), (false, 0, Legacy, true),
    ] {
        assert_eq!(LimitList::claude_on_pc(file, count, state), expected);
    }
}

// ---- UsageLevelTests

fn lim(k: LimitKind, pct: f64) -> Limit {
    Limit {
        provider: "Claude Code".into(), kind: k, label: format!("{k:?}"), percent: pct, resets_at: None, observed_at: now(),
        is_reset: false, window: None, used_usd: None, limit_usd: None, period: None, used_credits: None, credit_budget: None,
    }
}

#[test]
fn level_for_maps_thresholds() {
    for (pct, expected) in [(0, UsageLevel::Normal), (69, UsageLevel::Normal), (70, UsageLevel::Warning), (89, UsageLevel::Warning), (90, UsageLevel::Error), (100, UsageLevel::Error)] {
        assert_eq!(UsageLevels::for_percent(pct), expected);
    }
}

#[test]
fn highest_picks_worst() {
    assert_eq!(UsageLevels::highest(&[lim(LimitKind::Session, 72.0), lim(LimitKind::Weekly, 91.0)]), UsageLevel::Error);
}

#[test]
fn highest_of_none_is_normal() {
    assert_eq!(UsageLevels::highest(&[]), UsageLevel::Normal);
}

// ---- ResetPeriodTests

#[test]
fn reset_period_both_null_is_same() {
    assert!(ResetPeriod::same(None, None));
}

#[test]
fn reset_period_one_null_is_not_same() {
    let x = t("2026-11-09T10:00:00Z");
    assert!(!ResetPeriod::same(None, Some(x)));
    assert!(!ResetPeriod::same(Some(x), None));
}

#[test]
fn fifty_nine_minutes_apart_is_same() {
    let x = t("2026-11-09T10:00:00Z");
    assert!(ResetPeriod::same(Some(x), Some(x + mins(59))));
    assert!(ResetPeriod::same(Some(x + mins(59)), Some(x)));
}

#[test]
fn sixty_one_minutes_apart_is_not_same() {
    let x = t("2026-11-09T10:00:00Z");
    assert!(!ResetPeriod::same(Some(x), Some(x + mins(61))));
}

// ---- RateLimitsJsonTests

const FULL: &str = r#"
{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/p",
 "rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},
                "seven_day":{"used_percentage":41.2,"resets_at":1738857600},
                "spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly"}}}"#;

#[test]
fn parse_status_line_reads_all_three_windows() {
    let l = RateLimitsJson::parse_status_line(FULL).unwrap();
    assert_eq!(l.five_hour, Some(win(23.5, Some(unix(1738425600)))));
    assert_eq!(l.seven_day, Some(win(41.2, Some(unix(1738857600)))));
    assert_eq!(l.spend, Some(spend(62.8, Some(unix(1740787200)), Some(314.12), Some(500.0), Some("monthly"))));
}

#[test]
fn parse_status_line_returns_null_without_rate_limits() {
    assert!(RateLimitsJson::parse_status_line(r#"{"session_id":"abc"}"#).is_none());
}

#[test]
fn parse_status_line_returns_null_for_non_json() {
    for input in ["not json", "", "[]"] {
        assert!(RateLimitsJson::parse_status_line(input).is_none(), "{input}");
    }
}

#[test]
fn parse_status_line_returns_null_when_no_window_has_a_percentage() {
    assert!(RateLimitsJson::parse_status_line(r#"{"rate_limits":{}}"#).is_none());
}

#[test]
fn parse_status_line_keeps_windows_independently() {
    let l = RateLimitsJson::parse_status_line(r#"{"rate_limits":{"seven_day":{"used_percentage":10,"resets_at":1738857600}}}"#).unwrap();
    assert!(l.five_hour.is_none());
    assert_eq!(l.seven_day, Some(win(10.0, Some(unix(1738857600)))));
}

#[test]
fn parse_status_line_drops_window_with_non_numeric_percentage() {
    let l = RateLimitsJson::parse_status_line(r#"{"rate_limits":{"five_hour":{"used_percentage":"8","resets_at":1738425600},"seven_day":{"used_percentage":5}}}"#).unwrap();
    assert!(l.five_hour.is_none());
    assert!(l.seven_day.is_some());
}

#[test]
fn parse_status_line_keeps_window_without_valid_resets_at() {
    for resets_at in ["null", "\"soon\""] {
        let l = RateLimitsJson::parse_status_line(&format!("{{\"rate_limits\":{{\"five_hour\":{{\"used_percentage\":8,\"resets_at\":{resets_at}}}}}}}")).unwrap();
        assert_eq!(l.five_hour, Some(win(8.0, None)));
    }
}

#[test]
fn parse_status_line_drops_unknown_period() {
    let l = RateLimitsJson::parse_status_line(r#"{"rate_limits":{"spend_limit":{"used_percentage":50,"period":"hourly"}}}"#).unwrap();
    assert_eq!(l.spend, Some(spend(50.0, None, None, None, None)));
}

#[test]
fn write_usage_file_contains_only_rate_limit_fields_and_timestamp() {
    let snapshot = UsageSnapshot { limits: RateLimitsJson::parse_status_line(FULL).unwrap(), observed_at: unix(1738420000) };
    let json = RateLimitsJson::write_usage_file(&snapshot);
    for banned in ["session_id", "transcript_path", "cwd"] {
        assert!(!json.contains(banned));
    }
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut names: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    names.sort();
    assert_eq!(names, ["observed_at", "rate_limits"]);
}

#[test]
fn usage_file_round_trips() {
    let snapshot = UsageSnapshot { limits: RateLimitsJson::parse_status_line(FULL).unwrap(), observed_at: unix(1738420000) };
    assert_eq!(RateLimitsJson::parse_usage_file(&RateLimitsJson::write_usage_file(&snapshot)), Some(snapshot));
}

#[test]
fn parse_usage_file_returns_null_without_observed_at_or_rate_limits() {
    for input in [
        r#"{"rate_limits":{"five_hour":{"used_percentage":1}}}"#,
        r#"{"observed_at":1738420000}"#,
        r#"{"observed_at":1738420000,"rate_limits":{}}"#,
        "nope",
    ] {
        assert!(RateLimitsJson::parse_usage_file(input).is_none(), "{input}");
    }
}

#[test]
fn write_and_parse_usage_file_round_trip_window_observed_at() {
    let at = unix(1738400000);
    let mut sp = spend(30.0, Some(unix(1740787200)), Some(3.0), Some(10.0), Some("monthly"));
    sp.observed_at = Some(at);
    let snap = UsageSnapshot {
        limits: rl(Some(win_at(10.0, Some(unix(1738425600)), at)), Some(win(20.0, Some(unix(1738857600)))), Some(sp)),
        observed_at: unix(1738500000),
    };
    let json = RateLimitsJson::write_usage_file(&snap);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["rate_limits"]["five_hour"]["observed_at"].as_i64(), Some(1738400000));
    assert!(v["rate_limits"]["seven_day"].get("observed_at").is_none());
    assert_eq!(RateLimitsJson::parse_usage_file(&json), Some(snap));
}

#[test]
fn parse_usage_file_reads_a_milestone_1_file_without_window_times() {
    let snap = RateLimitsJson::parse_usage_file(r#"{"observed_at":1738500000,"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600}}}"#).unwrap();
    assert!(snap.limits.five_hour.unwrap().observed_at.is_none());
}

#[test]
fn parse_status_line_ignores_observed_at_inside_windows() {
    let l = RateLimitsJson::parse_status_line(r#"{"rate_limits":{"five_hour":{"used_percentage":10,"resets_at":1738425600,"observed_at":1738400000},"spend_limit":{"used_percentage":5,"observed_at":1738400000}}}"#).unwrap();
    assert!(l.five_hour.unwrap().observed_at.is_none());
    assert!(l.spend.unwrap().observed_at.is_none());
}

// ---- UsageMergeTests

fn tm() -> Time {
    t("2026-11-09T10:00:00Z")
}
fn session_reset() -> Time {
    tm() + hours(3)
}
fn weekly_reset() -> Time {
    tm() + days(4)
}

fn stored(five: Option<LimitWindow>, seven: Option<LimitWindow>, sp: Option<SpendLimit>, at: Option<Time>) -> UsageSnapshot {
    UsageSnapshot { limits: rl(five, seven, sp), observed_at: at.unwrap_or(tm() - mins(5)) }
}

fn sp_at(pct: f64, resets: Time, used: f64, limit: f64, period: &str, at: Option<Time>) -> SpendLimit {
    SpendLimit { observed_at: at, ..spend(pct, Some(resets), Some(used), Some(limit), Some(period)) }
}

#[test]
fn merge_with_nothing_stored_takes_reading_and_stamps_now() {
    let m = UsageMerge::merge(None, &rl(Some(win(10.0, Some(session_reset()))), Some(win(20.0, Some(weekly_reset()))), Some(spend(30.0, Some(weekly_reset()), Some(3.0), Some(10.0), Some("monthly")))), tm(), false);
    assert_eq!(m.observed_at, tm());
    assert_eq!(m.limits.five_hour.unwrap().observed_at, Some(tm()));
    assert_eq!(m.limits.seven_day.unwrap().observed_at, Some(tm()));
    assert_eq!(m.limits.spend.unwrap().observed_at, Some(tm()));
}

#[test]
fn merge_keeps_higher_reading_in_same_period() {
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(40.0, Some(session_reset() + Span::seconds(10)))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(60.0, Some(session_reset()), tm() - mins(5))));
    assert_eq!(m.observed_at, tm());
}

#[test]
fn merge_takes_equal_or_higher_reading_in_same_period() {
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(60.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(60.0, Some(session_reset()), tm())));
}

#[test]
fn merge_takes_lower_reading_from_a_later_period() {
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(5.0, Some(tm() + hours(8)))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(5.0, Some(tm() + hours(8)), tm())));
}

#[test]
fn merge_ignores_reading_from_an_earlier_period() {
    let s = stored(Some(win_at(5.0, Some(tm() + hours(8)), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(70.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(5.0, Some(tm() + hours(8)), tm() - mins(5))));
}

#[test]
fn merge_takes_reading_when_either_reset_time_is_missing() {
    let with_reset = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&with_reset), &rl(Some(win(10.0, None)), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(10.0, None, tm())));
    let without = stored(Some(win_at(60.0, None, tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&without), &rl(Some(win(10.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour, Some(win_at(10.0, Some(session_reset()), tm())));
}

#[test]
fn merge_keeps_stored_window_missing_from_reading_until_its_reset_passes() {
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), Some(win_at(30.0, Some(weekly_reset()), tm() - mins(5))), None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(61.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.seven_day, Some(win_at(30.0, Some(weekly_reset()), tm() - mins(5))));

    let expired = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), Some(win_at(30.0, Some(tm()), tm() - mins(5))), None, None);
    let m = UsageMerge::merge(Some(&expired), &rl(Some(win(61.0, Some(session_reset()))), None, None), tm(), false);
    assert!(m.limits.seven_day.is_none());
}

#[test]
fn merge_fills_window_observed_at_from_old_snapshot() {
    let s = stored(Some(win(60.0, Some(session_reset()))), Some(win(30.0, Some(weekly_reset()))), None, Some(tm() - hours(1)));
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(10.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.five_hour.unwrap().observed_at, Some(tm() - hours(1)));
    assert_eq!(m.limits.seven_day.unwrap().observed_at, Some(tm() - hours(1)));
}

#[test]
fn merge_applies_the_same_rule_to_spend() {
    let s = stored(None, None, Some(sp_at(60.0, weekly_reset(), 6.0, 10.0, "monthly", Some(tm() - mins(5)))), None);
    let m = UsageMerge::merge(Some(&s), &rl(None, None, Some(spend(40.0, Some(weekly_reset()), Some(4.0), Some(10.0), Some("weekly")))), tm(), false);
    assert_eq!(m.limits.spend, Some(sp_at(60.0, weekly_reset(), 6.0, 10.0, "monthly", Some(tm() - mins(5)))));
}

#[test]
fn merge_takes_spend_whose_cap_was_raised_in_the_same_period() {
    let s = stored(None, None, Some(sp_at(90.0, weekly_reset(), 45.0, 50.0, "monthly", Some(tm() - mins(5)))), None);
    let m = UsageMerge::merge(Some(&s), &rl(None, None, Some(spend(45.0, Some(weekly_reset()), Some(45.0), Some(100.0), Some("monthly")))), tm(), false);
    assert_eq!(m.limits.spend, Some(sp_at(45.0, weekly_reset(), 45.0, 100.0, "monthly", Some(tm()))));
}

#[test]
fn merge_compares_spend_in_dollars_when_the_cap_is_the_same() {
    let s = stored(None, None, Some(sp_at(60.0, weekly_reset(), 30.0, 50.0, "monthly", Some(tm() - mins(5)))), None);
    let fewer = UsageMerge::merge(Some(&s), &rl(None, None, Some(spend(61.0, Some(weekly_reset()), Some(29.0), Some(50.0), Some("monthly")))), tm(), false);
    assert_eq!(fewer.limits.spend, s.limits.spend);
    let more = UsageMerge::merge(Some(&s), &rl(None, None, Some(spend(59.0, Some(weekly_reset()), Some(31.0), Some(50.0), Some("monthly")))), tm(), false);
    assert_eq!(more.limits.spend, Some(sp_at(59.0, weekly_reset(), 31.0, 50.0, "monthly", Some(tm()))));
}

#[test]
fn merge_keeps_stored_spend_missing_from_reading_until_its_reset_passes() {
    let s = stored(None, None, Some(sp_at(90.0, weekly_reset(), 45.0, 50.0, "monthly", Some(tm() - mins(5)))), None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(10.0, Some(session_reset()))), None, None), tm(), false);
    assert_eq!(m.limits.spend, s.limits.spend);
    let expired = stored(None, None, Some(sp_at(90.0, tm(), 45.0, 50.0, "monthly", Some(tm() - mins(5)))), None);
    let m = UsageMerge::merge(Some(&expired), &rl(Some(win(10.0, Some(session_reset()))), None, None), tm(), false);
    assert!(m.limits.spend.is_none());
}

#[test]
fn merge_takes_lower_spend_from_a_later_period() {
    let s = stored(None, None, Some(sp_at(90.0, weekly_reset(), 45.0, 50.0, "monthly", Some(tm() - mins(5)))), None);
    let later = weekly_reset() + days(30);
    let m = UsageMerge::merge(Some(&s), &rl(None, None, Some(spend(4.0, Some(later), Some(2.0), Some(50.0), Some("monthly")))), tm(), false);
    assert_eq!(m.limits.spend, Some(sp_at(4.0, later, 2.0, 50.0, "monthly", Some(tm()))));
}

#[test]
fn an_authoritative_reading_replaces_a_higher_one_in_the_same_period() {
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), Some(win_at(40.0, Some(weekly_reset()), tm() - mins(5))), None, None);
    let incoming = rl(Some(win(12.0, Some(session_reset() + Span::seconds(10)))), Some(win(39.0, Some(weekly_reset()))), None);
    let live = UsageMerge::merge(Some(&s), &incoming, tm(), true);
    assert_eq!(live.limits.five_hour, Some(win_at(12.0, Some(session_reset() + Span::seconds(10)), tm())));
    assert_eq!(live.limits.seven_day, Some(win_at(39.0, Some(weekly_reset()), tm())));
    let status_line = UsageMerge::merge(Some(&s), &incoming, tm(), false);
    assert_eq!(status_line.limits.five_hour, s.limits.five_hour);
    assert_eq!(status_line.limits.seven_day, s.limits.seven_day);
}

#[test]
fn an_authoritative_reading_still_ignores_an_earlier_period() {
    let s = stored(Some(win_at(5.0, Some(tm() + hours(8)), tm() - mins(5))), None, None, None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(70.0, Some(session_reset()))), None, None), tm(), true);
    assert_eq!(m.limits.five_hour, s.limits.five_hour);
}

#[test]
fn an_authoritative_reading_leaves_spend_to_its_own_rule() {
    let sp = sp_at(60.0, weekly_reset(), 30.0, 50.0, "monthly", Some(tm() - mins(5)));
    let s = stored(Some(win_at(60.0, Some(session_reset()), tm() - mins(5))), None, Some(sp.clone()), None);
    let m = UsageMerge::merge(Some(&s), &rl(Some(win(10.0, Some(session_reset()))), None, None), tm(), true);
    assert_eq!(m.limits.spend, Some(sp.clone()));
    let fewer = rl(Some(win(10.0, Some(session_reset()))), None, Some(spend(50.0, Some(weekly_reset()), Some(25.0), Some(50.0), Some("monthly"))));
    assert_eq!(UsageMerge::merge(Some(&s), &fewer, tm(), true).limits.spend, Some(sp));
}

// ---- UsageFileTests

fn sample() -> UsageSnapshot {
    UsageSnapshot {
        limits: rl(Some(win(23.5, Some(unix(1738425600)))), Some(win(41.2, Some(unix(1738857600)))), None),
        observed_at: unix(1738400000),
    }
}

#[test]
fn write_then_read_round_trips() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    assert_eq!(UsageFile::read(&path), Some(sample()));
}

#[test]
fn write_creates_missing_directory() {
    let d = Scratch::new();
    let path = d.path("a/b/usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    assert!(std::path::Path::new(&path).exists());
}

#[test]
fn write_replaces_existing_file_and_leaves_no_temp_files() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    UsageFile::write(&path, &sample()).unwrap();
    let files: Vec<_> = std::fs::read_dir(d.dir()).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files, vec![std::path::PathBuf::from(&path)]);
}

#[test]
fn read_for_merge_gives_the_stored_snapshot() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    let mut stored = None;
    assert!(UsageFile::try_read_for_merge(&path, &mut stored));
    assert_eq!(stored, Some(sample()));
}

#[test]
fn read_for_merge_of_a_missing_file_is_safe_to_write() {
    let d = Scratch::new();
    let mut stored = None;
    assert!(UsageFile::try_read_for_merge(&d.path("usage.json"), &mut stored));
    assert!(stored.is_none());
}

#[test]
fn read_for_merge_of_unparseable_content_is_safe_to_write_over() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    std::fs::write(&path, "not json").unwrap();
    let mut stored = None;
    assert!(UsageFile::try_read_for_merge(&path, &mut stored));
    assert!(stored.is_none());
}

fn hold(path: &str) -> std::fs::File {
    std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(path).unwrap()
}

#[test]
fn read_for_merge_of_a_file_held_by_a_writer_is_not_safe() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    let _held = hold(&path);
    let mut stored = None;
    assert!(!UsageFile::try_read_for_merge(&path, &mut stored));
    assert!(stored.is_none());
}

#[test]
fn read_returns_null_for_missing_file() {
    let d = Scratch::new();
    assert!(UsageFile::read(&d.path("usage.json")).is_none());
}

#[test]
fn read_returns_null_for_corrupt_file() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    std::fs::write(&path, "{").unwrap();
    assert!(UsageFile::read(&path).is_none());
}

#[test]
fn read_returns_null_while_file_is_locked() {
    let d = Scratch::new();
    let path = d.path("usage.json");
    UsageFile::write(&path, &sample()).unwrap();
    let _locked = hold(&path);
    assert!(UsageFile::read(&path).is_none());
}
