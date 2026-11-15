//! What the flyout shows: one view per limit, Copilot's section and the footer.
use chrono::{DateTime, TimeDelta, Utc};

use super::limit_status::{self, LimitStatus};
use super::tooltip_text::{credits_figures, grouped};
use super::{ClockFormat, PaceDays, Zone, duration, pace, when};
use crate::limits::providers;
use crate::limits::usage_level::for_percent;
use crate::limits::{Limit, LimitKind, UsageLevel};
use crate::live::live_state;
use crate::polling::ProviderState;

/// One limit as the flyout draws it: the figures, the bar and the words under it.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitView {
    pub kind: LimitKind,
    pub label: &'static str,
    pub has_data: bool,
    pub used_figure: String,
    pub left_figure: String,
    /// How much of the bar is used, from 0 to 1.
    pub used_fraction: f64,
    /// Where the pace ends up by the reset, from 0 to 1, when it can be told.
    pub projected_fraction: Option<f64>,
    pub level: UsageLevel,
    pub status: Option<LimitStatus>,
    pub resets_in: Option<String>,
    pub detail: Option<String>,
}

/// Copilot's block of the flyout: the credits and a note about where they come from.
#[derive(Debug, Clone, PartialEq)]
pub struct CopilotSection {
    pub credits: LimitView,
    pub note: Option<String>,
}

/// What the flyout shows.
#[derive(Debug, Clone, PartialEq)]
pub struct FlyoutModel {
    pub title: &'static str,
    pub session: LimitView,
    pub weekly: LimitView,
    pub spend: Option<LimitView>,
    pub footer: String,
    pub copilot: Option<CopilotSection>,
    pub show_claude: bool,
}

/// The choices [`FlyoutModel::from`] takes beside the limits and the clock.
#[derive(Debug, Clone, PartialEq)]
pub struct FlyoutOptions {
    /// Where Live usage stands.
    pub live: ProviderState,
    /// The status line still runs the old OpenHeadroom bridge.
    pub legacy_connection: bool,
    /// The sentence under Copilot's figures.
    pub copilot_note: Option<String>,
    pub show_claude: bool,
    pub days: PaceDays,
}

impl Default for FlyoutOptions {
    fn default() -> FlyoutOptions {
        FlyoutOptions {
            live: ProviderState::Off,
            legacy_connection: false,
            copilot_note: None,
            show_claude: true,
            days: PaceDays::EveryDay,
        }
    }
}

impl FlyoutModel {
    pub fn from(
        all: &[Limit],
        now: DateTime<Utc>,
        zone: &Zone,
        clock: &ClockFormat,
        options: &FlyoutOptions,
    ) -> FlyoutModel {
        let credits = all.iter().find(|limit| limit.kind == LimitKind::Credits);
        let limits: Vec<&Limit> = all
            .iter()
            .filter(|limit| limit.kind != LimitKind::Credits)
            .collect();
        let of_kind = |kind| limits.iter().find(|limit| limit.kind == kind).copied();
        let view = |limit, kind, label| view(limit, kind, label, now, zone, clock, options.days);

        let footer_limits: Vec<&Limit> = if options.show_claude {
            limits.clone()
        } else {
            credits.into_iter().collect()
        };
        FlyoutModel {
            title: providers::CLAUDE,
            session: view(of_kind(LimitKind::Session), LimitKind::Session, "Session"),
            weekly: view(of_kind(LimitKind::Weekly), LimitKind::Weekly, "Weekly"),
            spend: of_kind(LimitKind::Spend)
                .map(|spend| view(Some(spend), LimitKind::Spend, "Spend")),
            footer: footer_text(&footer_limits, now, zone, clock, options),
            copilot: credits.map(|credits| CopilotSection {
                credits: view(Some(credits), LimitKind::Credits, "AI credits"),
                note: options.copilot_note.clone(),
            }),
            show_claude: options.show_claude,
        }
    }

    /// "resets in 3d 13h", "reset at 9:00 AM", or none when there is no reset time.
    pub fn resets_in(
        limit: &Limit,
        now: DateTime<Utc>,
        zone: &Zone,
        clock: &ClockFormat,
    ) -> Option<String> {
        let at = limit.resets_at?;
        Some(if limit.is_reset {
            format!("reset at {}", when::format(at, now, zone, clock))
        } else {
            format!(
                "resets in {}",
                duration::format(at.signed_duration_since(now))
            )
        })
    }
}

fn view(
    limit: Option<&Limit>,
    kind: LimitKind,
    label: &'static str,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
    days: PaceDays,
) -> LimitView {
    let Some(limit) = limit else {
        return LimitView {
            kind,
            label,
            has_data: false,
            used_figure: "–".to_string(),
            left_figure: "–".to_string(),
            used_fraction: 0.0,
            projected_fraction: None,
            level: UsageLevel::Normal,
            status: None,
            resets_in: Some("No reading yet".to_string()),
            detail: None,
        };
    };

    let pace = pace::project(limit, days, zone);
    let used = limit.display_percent();
    LimitView {
        kind: limit.kind,
        label: limit.label,
        has_data: true,
        used_figure: format!("{used}%"),
        left_figure: format!("{}%", 100 - used),
        used_fraction: (limit.percent / 100.0).clamp(0.0, 1.0),
        projected_fraction: pace.map(|pace| at_most_100(pace.projected_percent) / 100.0),
        level: for_percent(used),
        status: limit_status::for_limit(limit, now, zone, clock, days),
        resets_in: FlyoutModel::resets_in(limit, now, zone, clock),
        detail: match kind {
            LimitKind::Spend => spend_detail(limit),
            LimitKind::Credits => credits_detail(limit),
            _ => None,
        },
    }
}

/// `Math.Min(value, 100)`: not-a-number stays so, where `f64::min` would give the 100.
fn at_most_100(value: f64) -> f64 {
    if value.is_nan() {
        value
    } else {
        value.min(100.0)
    }
}

fn credits_detail(limit: &Limit) -> Option<String> {
    if limit.is_reset {
        None
    } else {
        credits_figures(limit)
    }
}

fn spend_detail(limit: &Limit) -> Option<String> {
    let used = limit.used_usd?;
    let period = match limit.period.as_deref() {
        Some("daily") => " today",
        Some("weekly") => " this week",
        Some("monthly") => " this month",
        _ => "",
    };
    Some(match limit.limit_usd {
        Some(cap) => format!("{} of {}{period}", money(used), money(cap)),
        None => format!("{} spent{period}", money(used)),
    })
}

fn money(value: f64) -> String {
    format!("${}", grouped(value, 2))
}

fn footer_text(
    limits: &[&Limit],
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
    options: &FlyoutOptions,
) -> String {
    let show_claude = options.show_claude;
    if show_claude && limits.is_empty() && options.legacy_connection {
        return "No numbers yet. The status line still runs the old OpenHeadroom bridge: choose Reconnect in the tray menu.".to_string();
    }
    let sentence = if show_claude {
        live_state::footer(options.live)
    } else {
        None
    };
    // Only Claude's numbers wait for Claude Code's next response.
    let live = if show_claude {
        options.live
    } else {
        ProviderState::Ok
    };
    let age = age_text(limits, now, zone, clock, live);
    match sentence {
        None => age,
        Some(sentence) => {
            let join = if age.ends_with('.') { " " } else { ". " };
            format!("{age}{join}{sentence}")
        }
    }
}

// With Live usage on, the numbers may come from Anthropic before Claude Code's next response.
fn age_text(
    limits: &[&Limit],
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
    live: ProviderState,
) -> String {
    let Some(newest) = limits.iter().map(|limit| limit.observed_at).max() else {
        return if live == ProviderState::Off {
            "No numbers yet. They arrive with Claude Code's next response."
        } else {
            "No numbers yet."
        }
        .to_string();
    };
    let age = now.signed_duration_since(newest);
    if age < TimeDelta::minutes(1) {
        "Updated just now".to_string()
    } else if age < TimeDelta::minutes(60) {
        format!("Updated {} min ago", age.num_minutes())
    } else if age < TimeDelta::hours(24) {
        format!("Updated {} ago", duration::format(age))
    } else {
        format!("Updated {}", when::format(newest, now, zone, clock))
    }
}
