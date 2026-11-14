// Ports of WhenTests, DurationTests, PaceTests, WorkingTimeTests, LimitStatusTests (with BadgeColourTests),
// StatusLineTextTests. The clock is fixed per test thread (en-US "9:00 AM" or pt-PT "14:20").
mod common;
use common::*;
use sobreiro_core::colours::{Contrast, NightPalette};
use sobreiro_core::display::*;
use sobreiro_core::limits::{Limit, LimitKind};
use sobreiro_core::usage::{LimitWindow, RateLimits, SpendLimit};
use sobreiro_core::{Span, Time, Zone};

fn en() {
    When::set_clock(Clock::English12);
}

fn approx(expected: f64, actual: f64, tol: f64) {
    assert!((expected - actual).abs() <= tol, "{actual} is not {expected} (+-{tol})");
}

// ---- WhenTests

fn f(s: &str) -> String {
    en();
    When::format(t(s), t("2026-11-09T10:00:00Z"), Zone::utc())
}

#[test]
fn same_day() {
    assert_eq!(f("2026-11-09T14:20:00Z"), "2:20 PM");
}
#[test]
fn within_six_days_gets_day_name() {
    assert_eq!(f("2026-11-14T00:00:00Z"), "Sat 12:00 AM");
}
#[test]
fn seven_days_ahead_gets_date() {
    assert_eq!(f("2026-11-16T00:00:00Z"), "Mon 16 Nov 12:00 AM");
}
#[test]
fn seven_days_back_gets_date() {
    assert_eq!(f("2026-11-02T14:02:00Z"), "Mon 2 Nov 2:02 PM");
}
#[test]
fn six_days_back_gets_day_name() {
    assert_eq!(f("2026-11-03T14:02:00Z"), "Tue 2:02 PM");
}
#[test]
fn uses_the_clock_time_pattern() {
    When::set_clock(Clock::Hours24);
    assert_eq!(When::format(t("2026-11-09T14:20:00Z"), t("2026-11-09T10:00:00Z"), Zone::utc()), "14:20");
}
#[test]
fn the_windows_clock_gives_a_time_of_day() {
    // SPIKE: the user's own pattern; only its shape is checked, as it depends on the PC.
    When::set_clock(Clock::Windows);
    let s = When::format(t("2026-11-09T14:20:00Z"), t("2026-11-09T10:00:00Z"), Zone::utc());
    assert!(s.contains("20") && s.chars().any(|c| c.is_ascii_digit()), "{s}");
}

// ---- DurationTests

#[test]
fn formats_minutes() {
    for (minutes, expected) in [(3 * 60 + 11, "3h 11m"), (3 * 1440 + 13 * 60 + 59, "3d 13h"), (2 * 1440 + 30, "2d"), (45, "45m")] {
        assert_eq!(Duration::format(mins(minutes)), expected);
    }
}
#[test]
fn drops_seconds_at_whole_hours() {
    assert_eq!(Duration::format(Span::hours(2) + Span::seconds(30)), "2h");
}
#[test]
fn under_a_minute() {
    assert_eq!(Duration::format(Span::seconds(30)), "under 1m");
}
#[test]
fn negative() {
    assert_eq!(Duration::format(hours(-2)), "under 1m");
}

// ---- PaceTests

fn now() -> Time {
    t("2026-11-09T10:00:00Z")
}

fn make(pct: f64, window: Option<Span>, resets_in: Option<Span>, observed: Option<Time>, is_reset: bool) -> Limit {
    limit("Claude Code", LimitKind::Session, "Session", pct, resets_in.map(|r| now() + r), observed.unwrap_or(now()), is_reset, window)
}

#[test]
fn session_on_track() {
    let p = Pace::project(&make(27.0, Some(hours(5)), Some(mins(191)), None, false)).unwrap();
    approx(74.31, p.projected_percent, 0.01);
    assert!(p.runs_out_before_reset.is_none() && p.runs_out_at.is_none());
}

#[test]
fn weekly_runs_out() {
    let p = Pace::project(&make(74.0, Some(days(7)), Some(hours(85)), None, false)).unwrap();
    approx(149.78, p.projected_percent, 0.01);
    let before = p.runs_out_before_reset.unwrap();
    approx(55.84, before.as_seconds_f64() / 3600.0, 1.0 / 60.0);
    assert_eq!(p.runs_out_at, Some(now() + hours(85) - before));
}

#[test]
fn too_early_is_null() {
    assert!(Pace::project(&make(5.0, Some(hours(5)), Some(hours(5) - mins(12)), None, false)).is_none());
}
#[test]
fn full_is_null() {
    assert!(Pace::project(&make(100.0, Some(hours(5)), Some(hours(2)), None, false)).is_none());
}
#[test]
fn reset_is_null() {
    assert!(Pace::project(&make(0.0, Some(hours(5)), Some(hours(2)), None, true)).is_none());
}
#[test]
fn no_reset_time_is_null() {
    assert!(Pace::project(&make(30.0, Some(hours(5)), None, None, false)).is_none());
}
#[test]
fn no_window_is_null() {
    assert!(Pace::project(&make(30.0, None, Some(hours(2)), None, false)).is_none());
}
#[test]
fn zero_percent_projects_zero() {
    let p = Pace::project(&make(0.0, Some(hours(4)), Some(hours(2)), None, false)).unwrap();
    assert_eq!(p.projected_percent, 0.0);
    assert!(p.runs_out_before_reset.is_none());
}
#[test]
fn uses_observed_time_not_now() {
    let p = Pace::project(&make(27.0, Some(hours(5)), Some(mins(131)), Some(now() - mins(60)), false)).unwrap();
    approx(74.31, p.projected_percent, 0.01);
}

fn long(pct: f64, resets: &str, observed: &str, window_days: i64) -> Limit {
    limit("Claude Code", LimitKind::Weekly, "Weekly", pct, Some(t(resets)), t(observed), false, Some(days(window_days)))
}
fn weekdays(l: &Limit) -> Option<PaceProjection> {
    Pace::project_with(l, PaceDays::Weekdays, Zone::utc())
}
// Run-out times come from a multiplication, so compare to the second.
fn near(expected: Time, actual: Option<Time>) {
    let a = actual.expect("a run-out time");
    assert!((a - expected).abs() < Span::seconds(1), "{a} is not {expected}");
}

#[test]
fn weekdays_remove_the_midweek_false_alarm() {
    // Reset Mon 09:00, read Thu 09:00 at 58%: 72 h of 168 h, or 72 h of 120 h.
    let l = long(58.0, "2026-11-16T09:00:00Z", "2026-11-12T09:00:00Z", 7);
    approx(135.33, Pace::project(&l).unwrap().projected_percent, 0.01);
    let p = weekdays(&l).unwrap();
    approx(96.67, p.projected_percent, 0.01);
    assert!(p.runs_out_at.is_none() && p.runs_out_before_reset.is_none());
}

#[test]
fn weekdays_remove_the_false_calm_after_a_weekend() {
    // Reset Fri 09:00, read Mon 09:00 at 30%: 72 h of 168 h, or 24 h of 120 h.
    let l = long(30.0, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z", 7);
    approx(70.0, Pace::project(&l).unwrap().projected_percent, 0.01);
    let p = weekdays(&l).unwrap();
    approx(150.0, p.projected_percent, 0.01);
    near(t("2026-11-11T17:00:00Z"), p.runs_out_at); // 56 weekday hours on
    approx(40.0, p.runs_out_before_reset.unwrap().as_seconds_f64() / 3600.0, 0.001);
}

#[test]
fn a_run_out_never_lands_on_the_weekend() {
    // Window Wed to Wed, read Fri 00:00 at 48%: 52 weekday hours left, past the weekend.
    let p = weekdays(&long(48.0, "2026-11-11T00:00:00Z", "2026-11-06T00:00:00Z", 7)).unwrap();
    approx(120.0, p.projected_percent, 0.01);
    near(t("2026-11-10T04:00:00Z"), p.runs_out_at);
    approx(20.0, p.runs_out_before_reset.unwrap().as_seconds_f64() / 3600.0, 0.001);
}

#[test]
fn use_on_a_day_off_makes_it_stricter() {
    let saturday = weekdays(&long(70.0, "2026-11-11T00:00:00Z", "2026-11-07T12:00:00Z", 7)).unwrap();
    let sunday = weekdays(&long(75.0, "2026-11-11T00:00:00Z", "2026-11-08T12:00:00Z", 7)).unwrap();
    approx(116.67, saturday.projected_percent, 0.01);
    approx(125.0, sunday.projected_percent, 0.01);
    assert!(saturday.runs_out_at.unwrap() >= t("2026-11-09T00:00:00Z"));
}

#[test]
fn with_only_the_weekend_left_the_projection_is_the_current_figure() {
    let l = long(80.0, "2026-11-16T00:00:00Z", "2026-11-14T10:00:00Z", 7);
    assert!(Pace::project(&l).unwrap().projected_percent > 100.0);
    let p = weekdays(&l).unwrap();
    approx(80.0, p.projected_percent, 0.01);
    assert!(p.runs_out_at.is_none());
}

#[test]
fn after_a_weekend_reset_it_is_too_early_until_five_percent_of_weekday_time() {
    assert!(weekdays(&long(3.0, "2026-11-14T00:00:00Z", "2026-11-08T12:00:00Z", 7)).is_none()); // Sunday: none gone
    assert!(weekdays(&long(3.0, "2026-11-14T00:00:00Z", "2026-11-09T05:00:00Z", 7)).is_none()); // 5 h of 120 h
    approx(51.43, weekdays(&long(3.0, "2026-11-14T00:00:00Z", "2026-11-09T07:00:00Z", 7)).unwrap().projected_percent, 0.01);
}

#[test]
fn a_month_counts_its_weekdays() {
    // November 2026, read Mon 16th 00:00 at 40%: 15 of 30 days, or 10 of 21 weekdays.
    let l = long(40.0, "2026-12-01T00:00:00Z", "2026-11-16T00:00:00Z", 30);
    approx(80.0, Pace::project(&l).unwrap().projected_percent, 0.01);
    approx(84.0, weekdays(&l).unwrap().projected_percent, 0.01);
}

#[test]
fn a_window_of_a_day_or_less_keeps_calendar_time() {
    // A Saturday: a session, then a daily limit.
    let session = limit("Claude Code", LimitKind::Session, "Session", 27.0, Some(t("2026-11-07T13:11:00Z")), t("2026-11-07T10:00:00Z"), false, Some(hours(5)));
    approx(74.31, weekdays(&session).unwrap().projected_percent, 0.01);
    let daily = long(40.0, "2026-11-08T00:00:00Z", "2026-11-07T12:00:00Z", 1);
    approx(80.0, weekdays(&daily).unwrap().projected_percent, 0.01);
}

#[test]
fn a_window_with_no_weekday_in_it_keeps_calendar_time() {
    approx(80.0, weekdays(&long(40.0, "2026-11-09T00:00:00Z", "2026-11-08T00:00:00Z", 2)).unwrap().projected_percent, 0.01);
}

#[test]
fn a_reading_after_the_reset_gives_nothing_in_either_mode() {
    // Reset Sat 00:00, read Sat 06:00 and not marked reset.
    let l = long(60.0, "2026-11-14T00:00:00Z", "2026-11-14T06:00:00Z", 7);
    assert!(Pace::project(&l).is_none());
    assert!(weekdays(&l).is_none());
}

#[test]
fn every_day_with_a_zone_is_the_same_as_before() {
    let l = make(74.0, Some(days(7)), Some(hours(85)), None, false);
    approx(149.78, Pace::project_with(&l, PaceDays::EveryDay, Zone::utc()).unwrap().projected_percent, 0.01);
}

// ---- WorkingTimeTests (2026-11-09 is a Monday)

#[test]
fn weekdays_count_in_full() {
    assert_eq!(WorkingTime::between(t("2026-11-09T09:00:00Z"), t("2026-11-12T09:00:00Z"), Zone::utc()), hours(72));
}
#[test]
fn a_weekend_in_the_middle_is_skipped() {
    assert_eq!(WorkingTime::between(t("2026-11-06T18:00:00Z"), t("2026-11-09T09:00:00Z"), Zone::utc()), hours(15));
}
#[test]
fn inside_a_weekend_is_zero() {
    assert_eq!(WorkingTime::between(t("2026-11-07T12:00:00Z"), t("2026-11-08T12:00:00Z"), Zone::utc()), Span::zero());
}
#[test]
fn backwards_is_zero() {
    assert_eq!(WorkingTime::between(t("2026-11-10T00:00:00Z"), t("2026-11-09T00:00:00Z"), Zone::utc()), Span::zero());
}
#[test]
fn a_whole_november_has_21_weekdays() {
    assert_eq!(WorkingTime::between(t("2026-11-01T00:00:00Z"), t("2026-12-01T00:00:00Z"), Zone::utc()), hours(504));
}
#[test]
fn the_day_of_the_week_is_the_local_one() {
    let plus9 = Zone::Fixed(chrono::FixedOffset::east_opt(9 * 3600).unwrap());
    // Fri 22:00Z to Sat 02:00Z is Sat 07:00 to Sat 11:00 at +09:00.
    assert_eq!(WorkingTime::between(t("2026-11-06T22:00:00Z"), t("2026-11-07T02:00:00Z"), plus9), Span::zero());
    // Sun 20:00Z to Sun 23:00Z is Mon 05:00 to Mon 08:00 at +09:00.
    assert_eq!(WorkingTime::between(t("2026-11-08T20:00:00Z"), t("2026-11-08T23:00:00Z"), plus9), hours(3));
}
// NOT PORTED: A_clock_change_at_the_weekend_moves_midnight. It needs the Europe/Lisbon zone with its clock change;
// Zone has the PC's local zone and fixed offsets only, and no zone database is a dependency.
#[test]
fn after_skips_the_weekend() {
    assert_eq!(WorkingTime::after(t("2026-11-06T00:00:00Z"), hours(52), Zone::utc()), t("2026-11-10T04:00:00Z"));
}
#[test]
fn after_from_a_saturday_starts_on_monday() {
    assert_eq!(WorkingTime::after(t("2026-11-07T12:00:00Z"), hours(1), Zone::utc()), t("2026-11-09T01:00:00Z"));
}
#[test]
fn after_nothing_is_the_start() {
    assert_eq!(WorkingTime::after(t("2026-11-07T12:00:00Z"), Span::zero(), Zone::utc()), t("2026-11-07T12:00:00Z"));
}

// ---- LimitStatusTests

fn lmake(pct: f64, window: Option<Span>, resets_in: Option<Span>, is_reset: bool) -> Limit {
    limit("Claude Code", LimitKind::Session, "Session", pct, resets_in.map(|r| now() + r), now(), is_reset, window)
}
fn status_for(l: &Limit) -> Option<LimitStatus> {
    en();
    LimitStatus::for_limit(l, now(), Zone::utc(), PaceDays::EveryDay)
}
fn st(kind: StatusKind, text: &str) -> Option<LimitStatus> {
    Some(LimitStatus { kind, text: text.into() })
}
fn five() -> Span {
    hours(5)
}

#[test]
fn on_track() {
    assert_eq!(status_for(&lmake(27.0, Some(five()), Some(mins(191)), false)), st(StatusKind::OnTrack, "On track"));
}
#[test]
fn weekly_ends_with_day_name() {
    assert_eq!(status_for(&lmake(74.0, Some(days(7)), Some(hours(85)), false)), st(StatusKind::Ends, "Ends Tue 3:09 PM"));
}
#[test]
fn session_ends_today_without_day_name() {
    assert_eq!(status_for(&lmake(83.0, Some(five()), Some(five() - mins(150)), false)), st(StatusKind::Ends, "Ends 10:30 AM"));
}
#[test]
fn used_up() {
    assert_eq!(status_for(&lmake(100.0, Some(five()), Some(hours(2)), false)), st(StatusKind::UsedUp, "Used up"));
}
#[test]
fn not_used_yet_under_two_percent() {
    assert_eq!(status_for(&lmake(1.0, Some(five()), Some(hours(2)), false)), st(StatusKind::NotUsedYet, "Not used yet"));
}
#[test]
fn not_used_yet_after_a_reset() {
    assert_eq!(status_for(&lmake(0.0, Some(five()), Some(hours(-1)), true)).unwrap().kind, StatusKind::NotUsedYet);
}
#[test]
fn too_early() {
    assert_eq!(status_for(&lmake(5.0, Some(five()), Some(five() - mins(12)), false)), st(StatusKind::TooEarly, "Too early"));
}
#[test]
fn resetting_when_reset_is_seconds_away() {
    assert_eq!(status_for(&lmake(90.0, Some(five()), Some(Span::seconds(20)), false)), st(StatusKind::Resetting, "Resetting"));
}
#[test]
fn stale_reading_whose_run_out_has_passed_is_likely_used_up() {
    // Observed 2 h ago, 2 h into the window, at 83%: runs out about 25 min after the reading, long before now.
    let mut l = lmake(83.0, Some(five()), Some(hours(1)), false);
    l.observed_at = now() - hours(2);
    assert_eq!(status_for(&l), st(StatusKind::Ends, "Likely used up"));
}
#[test]
fn exactly_two_percent_is_not_not_used_yet() {
    assert_ne!(status_for(&lmake(2.0, Some(five()), Some(hours(2)), false)).unwrap().kind, StatusKind::NotUsedYet);
}
#[test]
fn just_under_a_hundred_is_not_used_up() {
    assert_ne!(status_for(&lmake(99.9, Some(five()), Some(hours(2)), false)).unwrap().kind, StatusKind::UsedUp);
}
#[test]
fn exactly_a_hundred_is_used_up() {
    assert_eq!(status_for(&lmake(100.0, Some(five()), Some(hours(2)), false)).unwrap().kind, StatusKind::UsedUp);
}
#[test]
fn reset_exactly_thirty_seconds_away_is_not_resetting() {
    assert_ne!(status_for(&lmake(90.0, Some(five()), Some(Span::seconds(30)), false)).unwrap().kind, StatusKind::Resetting);
}
#[test]
fn reset_twenty_seconds_away_is_resetting() {
    assert_eq!(status_for(&lmake(90.0, Some(five()), Some(Span::seconds(20)), false)).unwrap().kind, StatusKind::Resetting);
}
#[test]
fn is_reset_never_reads_resetting() {
    assert_eq!(status_for(&lmake(0.0, Some(five()), Some(Span::seconds(10)), true)).unwrap().kind, StatusKind::NotUsedYet);
}
#[test]
fn no_window_gives_no_badge() {
    assert!(status_for(&lmake(30.0, None, Some(hours(2)), false)).is_none());
}
#[test]
fn no_reset_time_gives_no_badge() {
    assert!(status_for(&lmake(30.0, Some(five()), None, false)).is_none());
}

fn weekly_at(pct: f64, resets: &str, observed: &str) -> Limit {
    limit("Claude Code", LimitKind::Weekly, "Weekly", pct, Some(t(resets)), t(observed), false, Some(days(7)))
}

#[test]
fn weekdays_turn_a_midweek_alarm_into_on_track() {
    en();
    let l = weekly_at(58.0, "2026-11-16T09:00:00Z", "2026-11-12T09:00:00Z");
    let at = t("2026-11-12T09:00:00Z");
    assert_eq!(LimitStatus::for_limit(&l, at, Zone::utc(), PaceDays::EveryDay).unwrap().kind, StatusKind::Ends);
    assert_eq!(LimitStatus::for_limit(&l, at, Zone::utc(), PaceDays::Weekdays).unwrap().text, "On track");
}

#[test]
fn weekdays_show_a_run_out_on_a_working_day() {
    en();
    // Reset Fri 09:00, read Mon 09:00 at 40%: 24 weekday hours gone, 36 more to run out.
    let l = weekly_at(40.0, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z");
    let at = t("2026-11-09T09:00:00Z");
    assert_eq!(LimitStatus::for_limit(&l, at, Zone::utc(), PaceDays::EveryDay).unwrap().text, "On track");
    assert_eq!(LimitStatus::for_limit(&l, at, Zone::utc(), PaceDays::Weekdays).unwrap().text, "Ends Tue 9:00 PM");
}

#[test]
fn a_stale_reading_past_its_weekday_run_out_is_likely_used_up() {
    en();
    let l = weekly_at(40.0, "2026-11-13T09:00:00Z", "2026-11-09T09:00:00Z");
    assert_eq!(LimitStatus::for_limit(&l, t("2026-11-12T09:00:00Z"), Zone::utc(), PaceDays::Weekdays).unwrap().text, "Likely used up");
}

// ---- BadgeColourTests

#[test]
fn badge_values() {
    use StatusKind::*;
    for (dark, kind, fill, text) in [
        (true, OnTrack, "#1A7F37", "#FFFFFF"), (false, OnTrack, "#1A7F37", "#FFFFFF"),
        (true, Ends, "#FFB020", "#0B0B0B"), (false, Ends, "#A35F00", "#FFFFFF"),
        (true, UsedUp, "#FF6B6B", "#0B0B0B"), (false, UsedUp, "#C4262E", "#FFFFFF"),
        (true, NotUsedYet, "#262626", "#F4F4F4"), (true, TooEarly, "#262626", "#F4F4F4"), (true, Resetting, "#262626", "#F4F4F4"),
        (false, NotUsedYet, "#E4E4E4", "#0D0D0D"), (false, TooEarly, "#E4E4E4", "#0D0D0D"), (false, Resetting, "#E4E4E4", "#0D0D0D"),
    ] {
        let b = NightPalette::for_dark(dark).badge(kind);
        assert_eq!((b.fill.to_hex(), b.text.to_hex()), (fill.to_string(), text.to_string()), "{kind:?} dark={dark}");
    }
}

#[test]
fn badge_text_on_fill_is_readable_in_both_palettes() {
    use StatusKind::*;
    for dark in [true, false] {
        for kind in [OnTrack, Ends, UsedUp, NotUsedYet, TooEarly, Resetting] {
            let b = NightPalette::for_dark(dark).badge(kind);
            assert!(Contrast::ratio(b.text, b.fill) >= 4.5, "{kind:?} dark={dark}");
        }
    }
}

// ---- StatusLineTextTests

fn lw(p: f64) -> Option<LimitWindow> {
    Some(win(p, None))
}

#[test]
fn status_line_both_windows() {
    assert_eq!(StatusLineText::render(Some(&RateLimits { five_hour: lw(8.4), seven_day: lw(44.0), spend: None })), "5h 8% · 7d 44%");
}
#[test]
fn status_line_only_weekly() {
    assert_eq!(StatusLineText::render(Some(&RateLimits { five_hour: None, seven_day: lw(44.0), spend: None })), "5h – · 7d 44%");
}
#[test]
fn status_line_null_limits() {
    assert_eq!(StatusLineText::render(None), "5h – · 7d –");
}
#[test]
fn status_line_with_spend() {
    let spend = SpendLimit { used_percentage: 62.8, resets_at: None, used_usd: None, limit_usd: None, period: None, observed_at: None };
    assert_eq!(
        StatusLineText::render(Some(&RateLimits { five_hour: lw(8.4), seven_day: lw(44.0), spend: Some(spend) })),
        "5h 8% · 7d 44% · spend 62%"
    );
}
