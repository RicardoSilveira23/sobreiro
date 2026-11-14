// Ports of PillModelTests, PillStaleTests, RingsModelTests, TooltipTextTests, GlyphFillTests, FlyoutPlacementTests,
// FlyoutModelTests, and the display parts of LegacyDisplayTests.
mod common;
use common::*;
use sobreiro_core::app_paths::AppPaths;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::display::*;
use sobreiro_core::limits::*;
use sobreiro_core::polling::ProviderState;
use sobreiro_core::usage::*;
use sobreiro_core::{Span, Time, Zone};

fn en() {
    When::set_clock(Clock::English12);
}

fn approx(expected: f64, actual: f64, tol: f64) {
    assert!((expected - actual).abs() <= tol, "{actual} is not {expected} (+-{tol})");
}

fn pl(label: &str, figure: &str, stale: bool) -> PillLine {
    PillLine { label: label.into(), figure: figure.into(), stale }
}

fn credits_at(now: Time, used: f64, observed: Time) -> Limit {
    let r = CopilotReading { budget: 15000.0, used, resets_at: Some(t("2026-12-01T00:00:00Z")), observed_at: observed, source: CopilotSource::Live };
    LimitList::from_copilot(Some(&r), now).remove(0)
}

// ---- PillModelTests

fn now12() -> Time {
    t("2026-11-09T12:00:00Z")
}

fn lc(kind: LimitKind, pct: f64) -> Limit {
    l(kind, pct, now12())
}

fn credits() -> Limit {
    credits_at(now12(), 2644.0, now12())
}

#[test]
fn pill_both_windows() {
    let m = PillModel::from_limits(&[lc(LimitKind::Session, 37.9), lc(LimitKind::Weekly, 61.0)]);
    assert_eq!(m.session, pl("5h", "37%", false));
    assert_eq!(m.weekly, pl("7d", "61%", false));
}

#[test]
fn pill_missing_windows_read_a_dash() {
    let m = PillModel::from_limits(&[]);
    assert_eq!(m.session, pl("5h", "–", false));
    assert_eq!(m.weekly, pl("7d", "–", false));
}

#[test]
fn pill_full_reads_100() {
    assert_eq!(PillModel::from_limits(&[lc(LimitKind::Session, 100.0)]).session.figure, "100%");
}

#[test]
fn pill_rings_match_the_rings_model() {
    let limits = [lc(LimitKind::Session, 74.0), lc(LimitKind::Weekly, 93.0)];
    assert_eq!(PillModel::from_limits(&limits).rings, RingsModel::from(&limits));
}

#[test]
fn pill_has_copilot_block_when_credits_present() {
    let c = PillModel::from(&[credits()], Zone::utc(), true).copilot.unwrap();
    assert_eq!(c.credits, pl("AI", "17%", false));
    assert_eq!(c.reset_date, "1 Dec");
    approx(0.176, c.fraction, 0.0005);
}

#[test]
fn pill_reset_date_is_local() {
    let m5 = Zone::Fixed(chrono::FixedOffset::west_opt(5 * 3600).unwrap());
    assert_eq!(PillModel::from(&[credits()], m5, true).copilot.unwrap().reset_date, "30 Nov");
}

#[test]
fn pill_without_credits_has_no_copilot_block() {
    assert!(PillModel::from(&[lc(LimitKind::Session, 10.0)], Zone::utc(), true).copilot.is_none());
}

fn words_now() -> Time {
    t("2026-11-13T12:00:00Z")
}
fn aged(kind: LimitKind, age: Span) -> Limit {
    l(kind, 40.0, words_now() - age)
}
fn old_credits(age: Span) -> Limit {
    credits_at(words_now(), 2644.0, words_now() - age)
}
fn state(limits: &[Limit], live: ProviderState, lm: i32, cop: ProviderState, cm: i32) -> PillModel {
    PillModel::from_state(limits, Zone::utc(), true, live, lm, cop, cm, words_now())
}

#[test]
fn stopped_claude_block_keeps_its_figures_and_rings() {
    let limits = [aged(LimitKind::Session, mins(20)), aged(LimitKind::Weekly, mins(20))];
    let m = state(&limits, ProviderState::Expired, 2, ProviderState::Ok, 0);
    assert!(m.claude_stale);
    assert_eq!(m.session, pl("5h", "40%", true));
    assert_eq!(m.weekly, pl("7d", "40%", true));
    assert_eq!(m.rings, RingsModel::from(&limits));
}

#[test]
fn a_stopped_block_marks_only_the_figures_it_has() {
    let m = state(&[aged(LimitKind::Session, mins(20))], ProviderState::Expired, 2, ProviderState::Ok, 0);
    assert!(m.claude_stale);
    assert!(m.session.stale);
    assert_eq!(m.weekly, pl("7d", "–", false));
}

#[test]
fn nothing_to_keep_reads_plain_dashes() {
    let m = state(&[], ProviderState::Expired, 2, ProviderState::Ok, 0);
    assert!(!m.claude_stale);
    assert_eq!(m.session, pl("5h", "–", false));
    assert_eq!(m.weekly, pl("7d", "–", false));
    assert_eq!(m.rings, RingsModel::from(&[]));
}

#[test]
fn one_missed_poll_marks_nothing() {
    let limits = [aged(LimitKind::Session, hours(3)), aged(LimitKind::Weekly, hours(3)), old_credits(days(3))];
    let m = state(&limits, ProviderState::Offline, 1, ProviderState::Offline, 1);
    assert!(!m.claude_stale && !m.session.stale && !m.weekly.stale);
    let c = m.copilot.unwrap();
    assert!(!c.stale && !c.credits.stale);
}

#[test]
fn fresh_claude_block_is_not_stale() {
    let limits = [aged(LimitKind::Session, mins(1)), aged(LimitKind::Weekly, mins(1))];
    let m = state(&limits, ProviderState::Expired, 2, ProviderState::Ok, 0);
    assert!(!m.claude_stale);
    assert_eq!(m.session, pl("5h", "40%", false));
    assert_eq!(m.weekly, pl("7d", "40%", false));
    assert_eq!(m.rings, RingsModel::from(&limits));
}

#[test]
fn stopped_copilot_block_keeps_its_figure_and_bar() {
    let c = state(&[old_credits(hours(25))], ProviderState::Off, 0, ProviderState::Offline, 2).copilot.unwrap();
    assert!(c.stale);
    assert_eq!(c.credits, pl("AI", "17%", true));
    assert_eq!(c.reset_date, "1 Dec");
    approx(0.176, c.fraction, 0.0005);
}

#[test]
fn each_block_decides_alone() {
    let copilot_only = state(
        &[aged(LimitKind::Session, mins(1)), aged(LimitKind::Weekly, mins(1)), old_credits(days(3))],
        ProviderState::Ok, 0, ProviderState::NoSignIn, 2,
    );
    assert!(!copilot_only.claude_stale && !copilot_only.session.stale);
    assert!(copilot_only.copilot.unwrap().stale);
    let claude_only = state(
        &[aged(LimitKind::Session, hours(3)), aged(LimitKind::Weekly, hours(3)), old_credits(mins(1))],
        ProviderState::Refused, 2, ProviderState::Ok, 0,
    );
    assert!(claude_only.claude_stale);
    let c = claude_only.copilot.unwrap();
    assert!(!c.stale && !c.credits.stale);
}

#[test]
fn the_three_argument_from_marks_nothing() {
    let m = PillModel::from(&[aged(LimitKind::Session, days(3)), old_credits(days(3))], Zone::utc(), true);
    assert!(!m.claude_stale && !m.session.stale);
    assert!(!m.copilot.unwrap().stale);
}

#[test]
fn pill_hides_claude_when_told() {
    assert!(!PillModel::from(&[credits()], Zone::utc(), false).show_claude);
    assert!(PillModel::from(&[credits()], Zone::utc(), true).show_claude);
}

// ---- PillStaleTests

const TWO: i32 = PillStale::MISSES_NEEDED;
const STOPPED: [ProviderState; 6] = [
    ProviderState::Expired, ProviderState::NoSignIn, ProviderState::Refused,
    ProviderState::Offline, ProviderState::BadAnswer, ProviderState::RateLimited,
];

fn old() -> Vec<Limit> {
    vec![aged(LimitKind::Session, days(3)), aged(LimitKind::Weekly, days(3))]
}
fn cred_aged(age: Span) -> Limit {
    limit("GitHub Copilot", LimitKind::Credits, "AI credits", 20.0, None, words_now() - age, false, None)
}

#[test]
fn every_stopped_state_marks_claude() {
    for s in STOPPED {
        assert!(PillStale::claude(s, &old(), words_now(), TWO), "{s:?}");
    }
}
#[test]
fn off_and_ok_are_never_stale() {
    for s in [ProviderState::Off, ProviderState::Ok] {
        assert!(!PillStale::claude(s, &old(), words_now(), TWO));
    }
}
#[test]
fn one_missed_poll_is_not_stale() {
    for misses in [0, 1] {
        for s in STOPPED {
            assert!(!PillStale::claude(s, &old(), words_now(), misses));
            assert!(!PillStale::copilot(s, Some(&cred_aged(days(3))), words_now(), misses));
        }
    }
}
#[test]
fn two_missed_polls_are_needed() {
    assert_eq!(PillStale::MISSES_NEEDED, 2);
}
#[test]
fn fresh_numbers_are_not_stale() {
    let edge = mins(15);
    assert!(!PillStale::claude(ProviderState::Expired, &[aged(LimitKind::Session, edge), aged(LimitKind::Weekly, edge)], words_now(), TWO));
    let past = edge + Span::seconds(1);
    assert!(PillStale::claude(ProviderState::Expired, &[aged(LimitKind::Session, past), aged(LimitKind::Weekly, past)], words_now(), TWO));
}
#[test]
fn the_newest_of_the_two_readings_decides() {
    assert!(!PillStale::claude(ProviderState::Offline, &[aged(LimitKind::Session, hours(3)), aged(LimitKind::Weekly, mins(1))], words_now(), TWO));
    assert!(!PillStale::claude(ProviderState::Offline, &[aged(LimitKind::Weekly, mins(1))], words_now(), TWO));
}
#[test]
fn a_spend_reading_does_not_count() {
    let spend = aged(LimitKind::Spend, mins(1));
    assert!(PillStale::claude(ProviderState::Offline, &[spend.clone(), aged(LimitKind::Session, hours(3))], words_now(), TWO));
    assert!(!PillStale::claude(ProviderState::Offline, &[spend], words_now(), TWO));
}
#[test]
fn nothing_to_keep_is_not_stale() {
    for s in STOPPED {
        assert!(!PillStale::claude(s, &[], words_now(), TWO));
    }
}
#[test]
fn a_reading_from_the_future_is_fresh() {
    assert!(!PillStale::claude(ProviderState::Offline, &[aged(LimitKind::Session, mins(-10))], words_now(), TWO));
}
#[test]
fn every_stopped_state_marks_copilot() {
    for s in [ProviderState::NoSignIn, ProviderState::Refused, ProviderState::Offline, ProviderState::BadAnswer, ProviderState::RateLimited] {
        assert!(PillStale::copilot(s, Some(&cred_aged(hours(25))), words_now(), TWO));
    }
}
#[test]
fn copilot_ok_is_not_stale() {
    assert!(!PillStale::copilot(ProviderState::Ok, Some(&cred_aged(hours(25))), words_now(), TWO));
}
#[test]
fn a_copilot_figure_from_today_is_not_stale() {
    assert!(!PillStale::copilot(ProviderState::NoSignIn, Some(&cred_aged(hours(2))), words_now(), TWO));
    assert!(!PillStale::copilot(ProviderState::NoSignIn, Some(&cred_aged(hours(24))), words_now(), TWO));
}
#[test]
fn no_reading_is_not_stale() {
    for s in [ProviderState::Ok, ProviderState::NoSignIn, ProviderState::Refused, ProviderState::Offline, ProviderState::BadAnswer, ProviderState::RateLimited] {
        assert!(!PillStale::copilot(s, None, words_now(), TWO));
    }
}

// ---- RingsModelTests

fn ring(p: i32) -> Ring {
    Ring { percent: p, level: UsageLevel::Normal, has_data: true }
}

#[test]
fn no_limits_gives_two_empty_rings() {
    assert_eq!(RingsModel::from(&[]), RingsModel { outer: Ring::NONE, inner: Ring::NONE, has_inner: true });
}
#[test]
fn outer_is_session_and_inner_is_weekly() {
    assert_eq!(
        RingsModel::from(&[lc(LimitKind::Weekly, 61.0), lc(LimitKind::Session, 37.9)]),
        RingsModel { outer: ring(37), inner: ring(61), has_inner: true }
    );
}
#[test]
fn each_ring_takes_its_own_level() {
    for (pct, level) in [(69.0, UsageLevel::Normal), (70.0, UsageLevel::Warning), (89.0, UsageLevel::Warning), (90.0, UsageLevel::Error)] {
        let m = RingsModel::from(&[lc(LimitKind::Session, pct), lc(LimitKind::Weekly, 10.0)]);
        assert_eq!(m.outer.level, level);
        assert_eq!(m.inner.level, UsageLevel::Normal);
    }
}
#[test]
fn percent_is_clamped_to_100() {
    assert_eq!(RingsModel::from(&[lc(LimitKind::Session, 130.0)]).outer.percent, 100);
}
#[test]
fn a_reset_window_is_zero() {
    let reset = limit("Claude Code", LimitKind::Session, "Session", 0.0, Some(now12()), now12(), true, None);
    assert_eq!(RingsModel::from(&[reset]).outer, ring(0));
}
#[test]
fn spend_is_not_drawn() {
    assert_eq!(RingsModel::from(&[lc(LimitKind::Spend, 50.0)]), RingsModel { outer: Ring::NONE, inner: Ring::NONE, has_inner: true });
}
#[test]
fn sweep_is_clockwise_degrees() {
    for (pct, degrees) in [(0, 0.0), (25, 90.0), (100, 360.0)] {
        assert_eq!(ring(pct).sweep_degrees(), degrees);
    }
}
#[test]
fn no_data_sweeps_nothing() {
    assert_eq!(Ring::NONE.sweep_degrees(), 0.0);
}
#[test]
fn capped_sweep_takes_the_round_caps_off_the_arc() {
    approx(297.93, ring(95).capped_sweep(3.25, 2.5), 0.005);
}
#[test]
fn capped_sweep_is_the_full_circle_at_100() {
    assert_eq!(ring(100).capped_sweep(3.25, 2.5), 360.0);
}
#[test]
fn capped_sweep_is_nothing_at_0_or_without_data() {
    assert_eq!(ring(0).capped_sweep(3.25, 2.5), 0.0);
    assert_eq!(Ring::NONE.capped_sweep(3.25, 2.5), 0.0);
}
#[test]
fn capped_sweep_leaves_a_dot_for_tiny_percentages() {
    assert_eq!(ring(3).capped_sweep(3.25, 2.5), 0.5);
}
#[test]
fn copilot_rings_have_one_ring() {
    let m = RingsModel::copilot(&[credits()]);
    assert!(!m.has_inner);
    assert_eq!(m.outer.percent, 17);
    assert_eq!(m.inner, Ring::NONE);
}

// ---- TooltipTextTests

fn tnow() -> Time {
    t("2026-11-09T12:00:00Z")
}
fn observed() -> Time {
    t("2026-11-09T11:58:00Z")
}
fn render(limits: Option<RateLimits>) -> String {
    en();
    let list = match limits {
        None => vec![],
        Some(l) => LimitList::from(Some(&UsageSnapshot { limits: l, observed_at: observed() }), tnow()),
    };
    TooltipText::render(&list, tnow(), Zone::utc(), false)
}
fn rl(five: Option<LimitWindow>, seven: Option<LimitWindow>, spend: Option<SpendLimit>) -> RateLimits {
    RateLimits { five_hour: five, seven_day: seven, spend }
}

#[test]
fn tooltip_no_limits() {
    assert_eq!(render(None), "Sobreiro · no data yet");
}
#[test]
fn tooltip_session_and_weekly() {
    assert_eq!(
        render(Some(rl(Some(win(8.0, Some(t("2026-11-09T14:20:00Z")))), Some(win(44.0, Some(t("2026-11-14T00:00:00Z")))), None))),
        "Session 8% · resets 2:20 PM\nWeekly 44% · resets Sat 12:00 AM\nas of 11:58 AM"
    );
}
#[test]
fn tooltip_absent_session_shows_dash() {
    assert!(render(Some(rl(None, Some(win(44.0, Some(t("2026-11-14T00:00:00Z")))), None))).starts_with("Session –\n"));
}
#[test]
fn tooltip_reset_session() {
    assert!(render(Some(rl(Some(win(30.0, Some(t("2026-11-09T11:00:00Z")))), None, None))).starts_with("Session 0% · reset 11:00 AM\n"));
}
#[test]
fn tooltip_spend_line_comes_before_as_of() {
    let spend = SpendLimit { used_percentage: 62.8, resets_at: Some(t("2026-11-16T00:00:00Z")), used_usd: None, limit_usd: None, period: None, observed_at: None };
    assert_eq!(
        render(Some(rl(Some(win(8.0, Some(t("2026-11-09T14:20:00Z")))), Some(win(44.0, Some(t("2026-11-14T00:00:00Z")))), Some(spend)))),
        "Session 8% · resets 2:20 PM\nWeekly 44% · resets Sat 12:00 AM\nSpend 62% · resets Mon 16 Nov 12:00 AM\nas of 11:58 AM"
    );
}
#[test]
fn tooltip_limit_without_reset_time_shows_only_percentage() {
    assert!(render(Some(rl(Some(win(8.0, None)), None, None))).starts_with("Session 8%\n"));
}
// NOT PORTED: Long_culture_pattern_is_truncated. It builds a culture with a very long short-time pattern; there is
// no culture to build here (Clock has three fixed shapes and the user's own), so the cut is not reachable by a test.
#[test]
fn claude_tooltip_ignores_copilot() {
    en();
    let claude = limit(Providers::CLAUDE, LimitKind::Session, "Session", 8.0, None, observed(), false, None);
    let text = TooltipText::render(&[credits_at(tnow(), 2644.0, tnow()), claude], tnow(), Zone::utc(), false);
    assert!(!text.contains("AI credits"));
    assert!(text.ends_with("as of 11:58 AM"));
}
#[test]
fn copilot_tooltip_has_three_lines() {
    en();
    let text = TooltipText::render_copilot(&credits_at(tnow(), 2644.0, observed()), tnow(), Zone::utc());
    assert_eq!(text, "AI credits 17% · resets Tue 1 Dec 12:00 AM\n2,644 of 15,000 credits\nas of 11:58 AM");
    assert!(text.chars().count() <= TooltipText::MAX_LENGTH);
}

// ---- GlyphFillTests

fn view(used: f64, projected: Option<f64>, has_data: bool) -> LimitView {
    LimitView {
        kind: LimitKind::Session, label: "Session".into(), has_data, used_figure: "".into(), left_figure: "".into(),
        used_fraction: used, projected_fraction: projected, level: UsageLevel::Normal, status: None, resets_in: None, detail: None,
    }
}

#[test]
fn mockup_session_has_seven_left_and_five_soon() {
    let f = GlyphFill::from(&view(0.27, Some(0.7431), true));
    assert_eq!((f.left_cells, f.soon_cells), (7, 5));
    approx(73.0, f.left_percent, 0.0001);
    approx(47.31, f.soon_percent, 0.0001);
}
#[test]
fn mockup_weekly_has_three_left_all_soon() {
    let f = GlyphFill::from(&view(0.74, Some(1.0), true));
    assert_eq!((f.left_cells, f.soon_cells), (3, 3));
    approx(26.0, f.left_percent, 0.0001);
    approx(26.0, f.soon_percent, 0.0001);
}
#[test]
fn no_projection_has_nothing_soon() {
    let f = GlyphFill::from(&view(0.27, None, true));
    assert_eq!((f.left_cells, f.soon_cells, f.soon_percent), (7, 0, 0.0));
}
#[test]
fn full_use_has_nothing_left() {
    assert_eq!(GlyphFill::from(&view(1.0, None, true)), GlyphFill { left_cells: 0, soon_cells: 0, left_percent: 0.0, soon_percent: 0.0 });
}
#[test]
fn no_data_has_nothing_left() {
    assert_eq!(GlyphFill::from(&view(0.0, None, false)), GlyphFill { left_cells: 0, soon_cells: 0, left_percent: 0.0, soon_percent: 0.0 });
}

// ---- FlyoutPlacementTests

const MONITOR: PixelRect = PixelRect::new(0, 0, 1920, 1080);
const W: i32 = 330;
const H: i32 = 400;
const M: i32 = 12;

fn run(monitor: PixelRect, work: PixelRect, cx: i32, cy: i32) -> (TaskbarEdge, i32, i32) {
    let (x, y) = FlyoutPlacement::place(monitor, work, cx, cy, W, H, M);
    (FlyoutPlacement::edge(monitor, work, cx, cy), x, y)
}

#[test]
fn bottom_taskbar_clamps_to_the_right() {
    assert_eq!(run(MONITOR, PixelRect::new(0, 0, 1920, 1032), 1800, 1060), (TaskbarEdge::Bottom, 1578, 620));
}
#[test]
fn bottom_taskbar_centres_on_the_cursor() {
    assert_eq!(run(MONITOR, PixelRect::new(0, 0, 1920, 1032), 300, 1060).1, 135);
}
#[test]
fn top_taskbar() {
    assert_eq!(run(MONITOR, PixelRect::new(0, 48, 1920, 1080), 1800, 20), (TaskbarEdge::Top, 1578, 60));
}
#[test]
fn left_taskbar() {
    assert_eq!(run(MONITOR, PixelRect::new(48, 0, 1920, 1080), 20, 900), (TaskbarEdge::Left, 60, 668));
}
#[test]
fn right_taskbar() {
    assert_eq!(run(MONITOR, PixelRect::new(0, 0, 1872, 1080), 1900, 500), (TaskbarEdge::Right, 1530, 300));
}
#[test]
fn auto_hide_uses_the_nearest_monitor_edge() {
    let (edge, _, y) = run(MONITOR, MONITOR, 1800, 1075);
    assert_eq!((edge, y), (TaskbarEdge::Bottom, 668));
}
#[test]
fn second_monitor_on_the_left() {
    assert_eq!(run(PixelRect::new(-1920, 0, 0, 1080), PixelRect::new(-1920, 0, 0, 1032), -100, 1060).1, -342);
}
#[test]
fn rect_sizes() {
    let r = PixelRect::new(10, 20, 110, 70);
    assert_eq!((r.width(), r.height()), (100, 50));
}

// ---- FlyoutModelTests

fn fnow() -> Time {
    t("2026-11-09T10:00:00Z")
}

fn flyout(limits: &[Limit], live: ProviderState) -> FlyoutModel {
    en();
    FlyoutModel::from(limits, fnow(), Zone::utc(), live, false, None, true, PaceDays::EveryDay)
}

fn build(limits: Option<RateLimits>, observed: Option<Time>) -> FlyoutModel {
    let list = match limits {
        None => vec![],
        Some(l) => LimitList::from(Some(&UsageSnapshot { limits: l, observed_at: observed.unwrap_or(fnow()) }), fnow()),
    };
    flyout(&list, ProviderState::Off)
}

fn mockup() -> RateLimits {
    rl(Some(win(27.0, Some(fnow() + mins(191)))), Some(win(74.0, Some(fnow() + hours(85)))), None)
}

fn status(kind: StatusKind, text: &str) -> Option<LimitStatus> {
    Some(LimitStatus { kind, text: text.into() })
}

#[test]
fn flyout_mockup_session() {
    let s = build(Some(mockup()), None).session;
    assert_eq!(s.label, "Session");
    assert_eq!(s.used_figure, "27%");
    assert_eq!(s.left_figure, "73%");
    assert_eq!(s.resets_in.as_deref(), Some("resets in 3h 11m"));
    assert_eq!(s.status, status(StatusKind::OnTrack, "On track"));
    approx(0.7431, s.projected_fraction.unwrap(), 0.0001);
    assert_eq!(s.level, UsageLevel::Normal);
}
#[test]
fn flyout_mockup_weekly() {
    let w = build(Some(mockup()), None).weekly;
    assert_eq!(w.level, UsageLevel::Warning);
    assert_eq!(w.resets_in.as_deref(), Some("resets in 3d 13h"));
    assert_eq!(w.status, status(StatusKind::Ends, "Ends Tue 3:09 PM"));
    assert_eq!(w.projected_fraction, Some(1.0));
    approx(0.74, w.used_fraction, 1e-9);
}
#[test]
fn flyout_missing_weekly() {
    let w = build(Some(rl(Some(win(27.0, Some(fnow() + mins(191)))), None, None)), None).weekly;
    assert!(!w.has_data);
    assert_eq!((w.label.as_str(), w.used_figure.as_str(), w.left_figure.as_str()), ("Weekly", "–", "–"));
    assert_eq!(w.resets_in.as_deref(), Some("No reading yet"));
    assert!(w.status.is_none());
    assert_eq!(w.used_fraction, 0.0);
}
#[test]
fn flyout_reset_session() {
    let s = build(Some(rl(Some(win(60.0, Some(t("2026-11-09T09:00:00Z")))), None, None)), None).session;
    assert_eq!(s.used_figure, "0%");
    assert_eq!(s.resets_in.as_deref(), Some("reset at 9:00 AM"));
    assert_eq!(s.status.unwrap().kind, StatusKind::NotUsedYet);
    assert!(s.projected_fraction.is_none());
}
#[test]
fn flyout_window_without_reset_time_has_no_reset_text() {
    let s = build(Some(rl(Some(win(20.0, None)), None, None)), None).session;
    assert!(s.has_data);
    assert!(s.resets_in.is_none() && s.status.is_none());
}
#[test]
fn flyout_spend_monthly_with_amounts() {
    let spend = SpendLimit { used_percentage: 63.0, resets_at: Some(t("2026-12-01T00:00:00Z")), used_usd: Some(31.5), limit_usd: Some(50.0), period: Some("monthly".into()), observed_at: None };
    let v = build(Some(rl(None, None, Some(spend))), None).spend.unwrap();
    assert_eq!(v.label, "Spend");
    assert_eq!(v.detail.as_deref(), Some("$31.50 of $50.00 this month"));
}
#[test]
fn flyout_spend_daily_only_used() {
    let spend = SpendLimit { used_percentage: 10.0, resets_at: None, used_usd: Some(1234.5), limit_usd: None, period: Some("daily".into()), observed_at: None };
    assert_eq!(build(Some(rl(None, None, Some(spend))), None).spend.unwrap().detail.as_deref(), Some("$1,234.50 spent today"));
}
#[test]
fn flyout_spend_without_amounts() {
    let spend = SpendLimit { used_percentage: 10.0, resets_at: None, used_usd: None, limit_usd: None, period: None, observed_at: None };
    assert!(build(Some(rl(None, None, Some(spend))), None).spend.unwrap().detail.is_none());
}
#[test]
fn flyout_no_spend() {
    assert!(build(Some(mockup()), None).spend.is_none());
}
#[test]
fn flyout_footer_age() {
    for (seconds_ago, expected) in [(20, "Updated just now"), (180, "Updated 3 min ago"), (2 * 3600 + 5 * 60, "Updated 2h 5m ago"), (3 * 86400, "Updated Fri 10:00 AM")] {
        assert_eq!(build(Some(mockup()), Some(fnow() - Span::seconds(seconds_ago))).footer, expected);
    }
}
#[test]
fn flyout_footer_without_limits() {
    assert_eq!(build(None, None).footer, "No numbers yet. They arrive with Claude Code's next response.");
}
#[test]
fn flyout_footer_without_limits_with_live_usage_on() {
    for (live, expected) in [
        (ProviderState::Ok, "No numbers yet."),
        (ProviderState::NoSignIn, "No numbers yet. Claude Code isn't signed in on this PC. Run claude in a terminal and sign in."),
        (ProviderState::Refused, "No numbers yet. Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one."),
        (ProviderState::Offline, "No numbers yet. Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes."),
    ] {
        assert_eq!(flyout(&[], live).footer, expected);
    }
}
fn mockup_limits(ago: i64) -> Vec<Limit> {
    LimitList::from(Some(&UsageSnapshot { limits: mockup(), observed_at: fnow() - Span::seconds(ago) }), fnow())
}
#[test]
fn flyout_footer_adds_the_refused_sentence_after_the_age() {
    assert_eq!(
        flyout(&mockup_limits(180), ProviderState::Refused).footer,
        "Updated 3 min ago. Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one."
    );
}
#[test]
fn flyout_footer_adds_the_live_sentence() {
    assert_eq!(
        flyout(&mockup_limits(180), ProviderState::Expired).footer,
        "Updated 3 min ago. Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it."
    );
}
#[test]
fn flyout_footer_has_no_live_sentence_when_ok() {
    assert_eq!(flyout(&mockup_limits(0), ProviderState::Ok).footer, "Updated just now");
}
#[test]
fn flyout_title() {
    assert_eq!(build(None, None).title, "Claude Code");
}

fn cnow() -> Time {
    t("2026-11-09T12:00:00Z")
}
fn claude_session(observed: Time) -> Limit {
    limit(Providers::CLAUDE, LimitKind::Session, "Session", 27.0, None, observed, false, None)
}
fn cfly(limits: &[Limit], live: ProviderState, note: Option<&str>, show_claude: bool, days: PaceDays) -> FlyoutModel {
    en();
    FlyoutModel::from(limits, cnow(), Zone::utc(), live, false, note, show_claude, days)
}

#[test]
fn flyout_copilot_section_has_detail_and_note() {
    let m = cfly(&[credits_at(cnow(), 2644.0, cnow())], ProviderState::Off, Some("Can't reach GitHub."), true, PaceDays::EveryDay);
    let c = m.copilot.unwrap();
    assert_eq!(c.credits.detail.as_deref(), Some("2,644 of 15,000 credits"));
    assert_eq!(c.note.as_deref(), Some("Can't reach GitHub."));
    assert_eq!(c.credits.used_figure, "17%");
}
#[test]
fn flyout_reset_credits_show_no_stale_detail() {
    let r = CopilotReading { budget: 15000.0, used: 2644.0, resets_at: Some(cnow() - hours(1)), observed_at: cnow() - days(3), source: CopilotSource::Live };
    let reset = LimitList::from_copilot(Some(&r), cnow()).remove(0);
    assert!(cfly(&[reset], ProviderState::Off, None, true, PaceDays::EveryDay).copilot.unwrap().credits.detail.is_none());
}
#[test]
fn flyout_title_stays_claude_with_copilot_first_in_list() {
    let m = cfly(&[credits_at(cnow(), 2644.0, cnow()), claude_session(cnow())], ProviderState::Off, None, true, PaceDays::EveryDay);
    assert_eq!(m.title, "Claude Code");
    assert_eq!(m.session.used_figure, "27%");
    assert!(m.copilot.is_some());
}
#[test]
fn flyout_footer_ignores_copilot_age_when_claude_shown() {
    let claude = claude_session(cnow() - hours(3));
    let with = cfly(&[claude.clone(), credits_at(cnow(), 2644.0, cnow())], ProviderState::Off, None, true, PaceDays::EveryDay);
    let without = cfly(&[claude], ProviderState::Off, None, true, PaceDays::EveryDay);
    assert_eq!(with.footer, without.footer);
    assert_ne!(with.footer, "Updated just now");
}
#[test]
fn flyout_copilot_only_footer_uses_copilot_age() {
    let m = cfly(&[credits_at(cnow(), 2644.0, cnow() - mins(2))], ProviderState::Ok, None, false, PaceDays::EveryDay);
    assert_eq!(m.footer, "Updated 2 min ago");
    assert!(!m.show_claude);
}
#[test]
fn the_pace_setting_reaches_the_bar_and_the_badge() {
    en();
    let at = t("2026-11-12T09:00:00Z");
    let weekly = limit(Providers::CLAUDE, LimitKind::Weekly, "Weekly", 58.0, Some(t("2026-11-16T09:00:00Z")), at, false, Some(days(7)));
    let every = FlyoutModel::from(&[weekly.clone()], at, Zone::utc(), ProviderState::Off, false, None, true, PaceDays::EveryDay).weekly;
    let wd = FlyoutModel::from(&[weekly], at, Zone::utc(), ProviderState::Off, false, None, true, PaceDays::Weekdays).weekly;
    assert_eq!(every.projected_fraction, Some(1.0));
    assert_eq!(every.status.unwrap().kind, StatusKind::Ends);
    approx(0.9667, wd.projected_fraction.unwrap(), 0.0001);
    assert_eq!(wd.status.unwrap().kind, StatusKind::OnTrack);
}
#[test]
fn the_pace_setting_reaches_copilot_credits() {
    en();
    let at = t("2026-11-16T00:00:00Z");
    let credits = limit(Providers::COPILOT, LimitKind::Credits, "AI credits", 40.0, Some(t("2026-12-01T00:00:00Z")), at, false, Some(days(30)));
    let view = FlyoutModel::from(&[credits], at, Zone::utc(), ProviderState::Off, false, None, true, PaceDays::Weekdays).copilot.unwrap().credits;
    approx(0.84, view.projected_fraction.unwrap(), 0.0001);
}

// ---- LegacyDisplayTests (the display and path parts; the Connect wording is not ported)

#[test]
fn tooltip_asks_to_reconnect_when_there_is_no_data_on_a_legacy_connection() {
    en();
    let text = TooltipText::render(&[], fnow(), Zone::utc(), true);
    assert_eq!(text, "Sobreiro · no data yet\nStill on the old bridge: reconnect from the tray menu.");
    assert!(text.chars().count() <= TooltipText::MAX_LENGTH);
    assert_eq!(TooltipText::render(&[], fnow(), Zone::utc(), false), "Sobreiro · no data yet");
}
#[test]
fn flyout_footer_asks_to_reconnect_when_there_is_no_data_on_a_legacy_connection() {
    en();
    let m = FlyoutModel::from(&[], fnow(), Zone::utc(), ProviderState::Off, true, None, true, PaceDays::EveryDay);
    assert_eq!(m.footer, "No numbers yet. The status line still runs the old OpenHeadroom bridge: choose Reconnect in the tray menu.");
    assert_eq!(flyout(&[], ProviderState::Off).footer, "No numbers yet. They arrive with Claude Code's next response.");
}
#[test]
fn default_data_directory_is_recognised_with_or_without_an_override_naming_it() {
    let env = |v: Option<&'static str>| move |n: &str| match n {
        "SOBREIRO_DATA_DIR" => v.map(String::from),
        "LOCALAPPDATA" => Some(r"C:\Users\ana\AppData\Local".to_string()),
        _ => None,
    };
    assert!(AppPaths::is_default_data_directory_with(&env(None)));
    assert!(AppPaths::is_default_data_directory_with(&env(Some(r"C:\Users\ana\AppData\Local\SOBREIRO\"))));
    assert!(!AppPaths::is_default_data_directory_with(&env(Some(r"C:\Temp\scratch"))));
}
#[test]
fn default_and_legacy_directories_sit_side_by_side_in_local_app_data() {
    let env = |n: &str| (n == "LOCALAPPDATA").then(|| r"C:\Users\ana\AppData\Local".to_string());
    assert_eq!(AppPaths::default_directory_with(&env), r"C:\Users\ana\AppData\Local\Sobreiro");
    assert_eq!(AppPaths::legacy_directory_with(&env), r"C:\Users\ana\AppData\Local\OpenHeadroom");
}
