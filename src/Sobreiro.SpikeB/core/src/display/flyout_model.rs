use super::{Duration, LimitStatus, Pace, PaceDays, TooltipText, When};
use crate::limits::{Limit, LimitKind, Providers, UsageLevel, UsageLevels};
use crate::live::LiveStateText;
use crate::polling::ProviderState;
use crate::{Span, Time, Zone};

#[derive(Clone, Debug, PartialEq)]
pub struct LimitView {
    pub kind: LimitKind,
    pub label: String,
    pub has_data: bool,
    pub used_figure: String,
    pub left_figure: String,
    pub used_fraction: f64,
    pub projected_fraction: Option<f64>,
    pub level: UsageLevel,
    pub status: Option<LimitStatus>,
    pub resets_in: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CopilotSection {
    pub credits: LimitView,
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlyoutModel {
    pub title: String,
    pub session: LimitView,
    pub weekly: LimitView,
    pub spend: Option<LimitView>,
    pub footer: String,
    pub copilot: Option<CopilotSection>,
    pub show_claude: bool,
}

impl FlyoutModel {
    /// `FlyoutModel.From(...)`. Every argument is required here (C# defaults: live Off, legacy_connection false,
    /// copilot_note None, show_claude true, days EveryDay). The culture is gone: times use the Windows clock.
    #[allow(clippy::too_many_arguments)]
    pub fn from(
        all: &[Limit], now: Time, zone: Zone, live: ProviderState, legacy_connection: bool,
        copilot_note: Option<&str>, show_claude: bool, days: PaceDays,
    ) -> FlyoutModel {
        let credits = all.iter().find(|l| l.kind == LimitKind::Credits);
        let limits: Vec<&Limit> = all.iter().filter(|l| l.kind != LimitKind::Credits).collect();
        let find = |k: LimitKind| limits.iter().copied().find(|l| l.kind == k);
        let footer_limits: Vec<&Limit> = if show_claude { limits.clone() } else { credits.into_iter().collect() };
        FlyoutModel {
            title: Providers::CLAUDE.to_string(),
            session: view(find(LimitKind::Session), LimitKind::Session, "Session", now, zone, days),
            weekly: view(find(LimitKind::Weekly), LimitKind::Weekly, "Weekly", now, zone, days),
            spend: find(LimitKind::Spend).map(|s| view(Some(s), LimitKind::Spend, "Spend", now, zone, days)),
            footer: footer_text(&footer_limits, now, zone, live, legacy_connection, show_claude),
            copilot: credits.map(|c| CopilotSection {
                credits: view(Some(c), LimitKind::Credits, "AI credits", now, zone, days),
                note: copilot_note.map(String::from),
            }),
            show_claude,
        }
    }

    /// "resets in 3d 13h", "reset at 9:00 AM", or None when there is no reset time.
    pub fn resets_in(limit: &Limit, now: Time, zone: Zone) -> Option<String> {
        let at = limit.resets_at?;
        Some(if limit.is_reset {
            format!("reset at {}", When::format(at, now, zone))
        } else {
            format!("resets in {}", Duration::format(at - now))
        })
    }
}

fn view(limit: Option<&Limit>, kind: LimitKind, label: &str, now: Time, zone: Zone, days: PaceDays) -> LimitView {
    let Some(limit) = limit else {
        return LimitView {
            kind, label: label.to_string(), has_data: false, used_figure: "–".into(), left_figure: "–".into(),
            used_fraction: 0.0, projected_fraction: None, level: UsageLevel::Normal, status: None,
            resets_in: Some("No reading yet".into()), detail: None,
        };
    };

    let pace = Pace::project_with(limit, days, zone);
    let used = limit.display_percent();
    LimitView {
        kind: limit.kind,
        label: limit.label.clone(),
        has_data: true,
        used_figure: format!("{used}%"),
        left_figure: format!("{}%", 100 - used),
        used_fraction: (limit.percent / 100.0).clamp(0.0, 1.0),
        projected_fraction: pace.map(|p| p.projected_percent.min(100.0) / 100.0),
        level: UsageLevels::for_percent(used),
        status: LimitStatus::for_limit(limit, now, zone, days),
        resets_in: FlyoutModel::resets_in(limit, now, zone),
        detail: match kind {
            LimitKind::Spend => spend_detail(limit),
            LimitKind::Credits => credits_detail(limit),
            _ => None,
        },
    }
}

fn credits_detail(limit: &Limit) -> Option<String> {
    if limit.is_reset { None } else { TooltipText::credits_figures(limit) }
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

/// "#,0.00" with the invariant culture.
fn money(v: f64) -> String {
    let fixed = format!("{:.2}", v.abs());
    let (int, frac) = fixed.split_once('.').unwrap_or((&fixed, "00"));
    let sign = if v < 0.0 && (int != "0" || frac != "00") { "-" } else { "" };
    format!("${sign}{}.{frac}", group_thousands(int))
}

/// Inserts a comma every three digits.
pub(crate) fn group_thousands(digits: &str) -> String {
    let n = digits.len();
    let mut out = String::with_capacity(n + n / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (n - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn footer_text(limits: &[&Limit], now: Time, zone: Zone, live: ProviderState, legacy_connection: bool, show_claude: bool) -> String {
    if show_claude && limits.is_empty() && legacy_connection {
        return "No numbers yet. The status line still runs the old OpenHeadroom bridge: choose Reconnect in the tray menu.".into();
    }
    let sentence = if show_claude { LiveStateText::footer(live) } else { None };
    // Only Claude's numbers wait for Claude Code's next response.
    let age = age_text(limits, now, zone, if show_claude { live } else { ProviderState::Ok });
    match sentence {
        None => age,
        Some(s) => format!("{age}{}{s}", if age.ends_with('.') { " " } else { ". " }),
    }
}

// With Live usage on, the numbers may come from Anthropic before Claude Code's next response.
fn age_text(limits: &[&Limit], now: Time, zone: Zone, live: ProviderState) -> String {
    let Some(newest) = limits.iter().map(|l| l.observed_at).max() else {
        return if live == ProviderState::Off {
            "No numbers yet. They arrive with Claude Code's next response.".into()
        } else {
            "No numbers yet.".into()
        };
    };
    let age = now - newest;
    if age < Span::minutes(1) {
        "Updated just now".into()
    } else if age < Span::minutes(60) {
        format!("Updated {} min ago", age.num_minutes())
    } else if age < Span::hours(24) {
        format!("Updated {} ago", Duration::format(age))
    } else {
        format!("Updated {}", When::format(newest, now, zone))
    }
}
