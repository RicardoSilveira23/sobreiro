use super::Providers;
use crate::connect::ConnectionState;
use crate::copilot::CopilotReading;
use crate::usage::UsageSnapshot;
use crate::{Span, Time};
use chrono::Months;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitKind {
    Session,
    Weekly,
    Spend,
    Credits,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub provider: String,
    pub kind: LimitKind,
    pub label: String,
    pub percent: f64,
    pub resets_at: Option<Time>,
    pub observed_at: Time,
    pub is_reset: bool,
    pub window: Option<Span>,
    pub used_usd: Option<f64>,
    pub limit_usd: Option<f64>,
    pub period: Option<String>,
    pub used_credits: Option<f64>,
    pub credit_budget: Option<f64>,
}

impl Limit {
    /// Floor of `percent`, clamped to 0-100.
    pub fn display_percent(&self) -> i32 {
        self.percent.floor().clamp(0.0, 100.0) as i32
    }
}

/// `end - end.AddMonths(-1)`.
fn month_before(end: Time) -> Span {
    match end.checked_sub_months(Months::new(1)) {
        Some(start) => end - start,
        None => Span::zero(),
    }
}

pub struct LimitList;

impl LimitList {
    pub fn from(snapshot: Option<&UsageSnapshot>, now: Time) -> Vec<Limit> {
        let mut list = Vec::new();
        let Some(snapshot) = snapshot else { return list };

        let l = &snapshot.limits;
        if let Some(five) = &l.five_hour {
            list.push(make(
                LimitKind::Session, "Session", five.used_percentage, five.resets_at,
                five.observed_at.unwrap_or(snapshot.observed_at), now, Some(Span::hours(5)), None, None, None,
            ));
        }
        if let Some(seven) = &l.seven_day {
            list.push(make(
                LimitKind::Weekly, "Weekly", seven.used_percentage, seven.resets_at,
                seven.observed_at.unwrap_or(snapshot.observed_at), now, Some(Span::days(7)), None, None, None,
            ));
        }
        if let Some(spend) = &l.spend {
            list.push(make(
                LimitKind::Spend, "Spend", spend.used_percentage, spend.resets_at,
                spend.observed_at.unwrap_or(snapshot.observed_at), now,
                spend_window(spend.period.as_deref(), spend.resets_at),
                spend.used_usd, spend.limit_usd, spend.period.clone(),
            ));
        }
        list
    }

    pub fn from_copilot(reading: Option<&CopilotReading>, now: Time) -> Vec<Limit> {
        let Some(reading) = reading else { return Vec::new() };
        let resets_at = reading.resets_at;
        let is_reset = resets_at.is_some_and(|r| r <= now);
        let window = resets_at.map(month_before);
        vec![Limit {
            provider: Providers::COPILOT.to_string(),
            kind: LimitKind::Credits,
            label: "AI credits".to_string(),
            percent: if is_reset { 0.0 } else { reading.percent() },
            resets_at,
            observed_at: reading.observed_at,
            is_reset,
            window,
            used_usd: None,
            limit_usd: None,
            period: None,
            used_credits: Some(reading.used),
            credit_budget: Some(reading.budget),
        }]
    }

    /// Claude is shown when it is on this PC, or when nothing else is shown.
    pub fn show_claude(claude_on_pc: bool, copilot_shown: bool) -> bool {
        claude_on_pc || !copilot_shown
    }

    pub fn claude_on_pc(credentials_file_exists: bool, claude_limit_count: usize, connection: ConnectionState) -> bool {
        credentials_file_exists || claude_limit_count > 0 || connection != ConnectionState::NotConnected
    }
}

#[allow(clippy::too_many_arguments)]
fn make(
    kind: LimitKind, label: &str, percent: f64, resets_at: Option<Time>, observed_at: Time, now: Time,
    window: Option<Span>, used_usd: Option<f64>, limit_usd: Option<f64>, period: Option<String>,
) -> Limit {
    let is_reset = resets_at.is_some_and(|r| r <= now);
    Limit {
        provider: Providers::CLAUDE.to_string(),
        kind,
        label: label.to_string(),
        percent: if is_reset { 0.0 } else { percent },
        resets_at,
        observed_at,
        is_reset,
        window,
        used_usd,
        limit_usd,
        period,
        used_credits: None,
        credit_budget: None,
    }
}

fn spend_window(period: Option<&str>, resets_at: Option<Time>) -> Option<Span> {
    match period {
        Some("daily") => Some(Span::days(1)),
        Some("weekly") => Some(Span::days(7)),
        Some("monthly") => resets_at.map(month_before),
        _ => None,
    }
}
