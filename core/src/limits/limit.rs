//! One limit, as the pill and the flyout show it.
use chrono::{DateTime, TimeDelta, Utc};

/// Which limit it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// Claude Code's five-hour window.
    Session,
    /// Claude Code's seven-day window.
    Weekly,
    /// Claude Code's extra usage, paid in dollars.
    Spend,
    /// GitHub Copilot's AI credits for the month.
    Credits,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    /// The service's name, one of those in [`providers`](super::providers).
    pub provider: &'static str,
    pub kind: LimitKind,
    /// The name shown beside the figure: "Session", "Weekly", "Spend" or "AI credits".
    pub label: &'static str,
    /// How much is used, in percent. Zero once the reset time has passed.
    pub percent: f64,
    pub resets_at: Option<DateTime<Utc>>,
    /// When the figure was read.
    pub observed_at: DateTime<Utc>,
    /// The reset time has passed and no newer reading has come in.
    pub is_reset: bool,
    /// How long the limit's period is: five hours, seven days, a month.
    pub window: Option<TimeDelta>,
    /// Dollars spent and the cap, for [`LimitKind::Spend`].
    pub used_usd: Option<f64>,
    pub limit_usd: Option<f64>,
    /// "daily", "weekly" or "monthly", for [`LimitKind::Spend`].
    pub period: Option<String>,
    /// Credits used and the month's budget, for [`LimitKind::Credits`].
    pub used_credits: Option<f64>,
    pub credit_budget: Option<f64>,
}

impl Limit {
    /// A limit with the fields every kind has. Those of one kind only start out empty.
    pub fn new(
        provider: &'static str,
        kind: LimitKind,
        label: &'static str,
        percent: f64,
        resets_at: Option<DateTime<Utc>>,
        observed_at: DateTime<Utc>,
        is_reset: bool,
    ) -> Limit {
        Limit {
            provider,
            kind,
            label,
            percent,
            resets_at,
            observed_at,
            is_reset,
            window: None,
            used_usd: None,
            limit_usd: None,
            period: None,
            used_credits: None,
            credit_budget: None,
        }
    }

    /// The percentage as it is shown: rounded down, and never under 0 or over 100.
    pub fn display_percent(&self) -> i32 {
        // `as i32` drops the ".0". The number is whole and within 0 to 100 by then.
        self.percent.floor().clamp(0.0, 100.0) as i32
    }
}
