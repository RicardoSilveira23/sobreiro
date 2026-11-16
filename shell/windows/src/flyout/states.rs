//! The states of the flyout's card the pictures and their tests draw: fixed limits at a fixed
//! clock in the UTC zone, so the badges and the resets-in text are Core's and never change.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::display::flyout_model::{FlyoutModel, FlyoutOptions};
use sobreiro_core::display::{ClockFormat, Zone};
use sobreiro_core::limits::providers::{CLAUDE, COPILOT};
use sobreiro_core::limits::{Limit, LimitKind};

/// Monday 16 November 2026, 20:00 UTC.
const NOW: i64 = 1_794_859_200;

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(NOW, 0).unwrap_or_default()
}

fn clock() -> ClockFormat {
    ClockFormat {
        pattern: "h:mm tt".to_string(),
        am: "AM".to_string(),
        pm: "PM".to_string(),
    }
}

/// A limit read a minute ago that resets `resets_in` from now, in a window of `window`.
fn limit(
    kind: LimitKind,
    label: &'static str,
    percent: f64,
    resets_in: TimeDelta,
    window: TimeDelta,
) -> Limit {
    let provider = if kind == LimitKind::Credits {
        COPILOT
    } else {
        CLAUDE
    };
    let mut limit = Limit::new(
        provider,
        kind,
        label,
        percent,
        Some(now() + resets_in),
        now() - TimeDelta::minutes(1),
        false,
    );
    limit.window = Some(window);
    limit
}

fn session(percent: f64, resets_in: TimeDelta) -> Limit {
    limit(
        LimitKind::Session,
        "Session",
        percent,
        resets_in,
        TimeDelta::hours(5),
    )
}

fn weekly(percent: f64, resets_in: TimeDelta) -> Limit {
    limit(
        LimitKind::Weekly,
        "Weekly",
        percent,
        resets_in,
        TimeDelta::days(7),
    )
}

/// The AI credits: 17% of 15,000, resetting in 20 days and 10 hours.
fn credits(percent: f64, resets_in: TimeDelta) -> Limit {
    let mut credits = limit(
        LimitKind::Credits,
        "AI credits",
        percent,
        resets_in,
        TimeDelta::days(30),
    );
    credits.credit_budget = Some(15_000.0);
    credits.used_credits = Some(15_000.0 * percent / 100.0);
    credits
}

fn spend() -> Limit {
    let mut spend = limit(
        LimitKind::Spend,
        "Spend",
        42.0,
        TimeDelta::days(12),
        TimeDelta::days(30),
    );
    spend.used_usd = Some(21.0);
    spend.limit_usd = Some(50.0);
    spend.period = Some("monthly".to_string());
    spend
}

fn calm() -> Vec<Limit> {
    vec![
        session(15.0, TimeDelta::minutes(141)),
        weekly(45.0, TimeDelta::hours(74)),
    ]
}

fn model(limits: &[Limit], show_claude: bool, note: Option<&str>) -> FlyoutModel {
    let options = FlyoutOptions {
        show_claude,
        copilot_note: note.map(str::to_string),
        ..FlyoutOptions::default()
    };
    FlyoutModel::from(limits, now(), &Zone::utc(), &clock(), &options)
}

/// Claude's card with the session limit as given, the weekly one on track.
fn badge(percent: f64, resets_in: TimeDelta) -> FlyoutModel {
    model(
        &[
            session(percent, resets_in),
            weekly(45.0, TimeDelta::hours(74)),
        ],
        true,
        None,
    )
}

/// Every state, by the name its pictures carry.
pub fn states() -> Vec<(&'static str, FlyoutModel)> {
    let two_hours = TimeDelta::hours(2);
    let copilot = credits(17.0, TimeDelta::hours(490));
    let mut both = calm();
    both.push(copilot.clone());
    let mut with_spend = calm();
    with_spend.push(spend());
    vec![
        ("claude-only", model(&calm(), true, None)),
        (
            "copilot-only",
            model(std::slice::from_ref(&copilot), false, None),
        ),
        ("both", model(&both, true, None)),
        ("with-spend", model(&with_spend, true, None)),
        ("no-reading", model(&[], true, None)),
        ("badge-on-track", badge(15.0, TimeDelta::minutes(141))),
        ("badge-ends", badge(70.0, two_hours)),
        ("badge-likely-used-up", badge(99.5, two_hours)),
        ("badge-used-up", badge(100.0, two_hours)),
        ("badge-not-used-yet", badge(1.0, TimeDelta::minutes(141))),
        ("badge-too-early", badge(30.0, TimeDelta::minutes(290))),
        ("badge-resetting", badge(30.0, TimeDelta::seconds(20))),
        // 41% at 11 of 30 days runs out in the first days of December, over 17 days on: the
        // badge is long and the resets-in text no longer fits beside it.
        (
            "far-ends",
            model(&[credits(41.0, TimeDelta::hours(442))], false, None),
        ),
        (
            "copilot-note",
            model(
                &[copilot],
                false,
                Some("From the Copilot CLI, as of 09:25."),
            ),
        ),
    ]
}
