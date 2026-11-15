//! The limits as Claude Code reports them: the shape of the status line's `rate_limits` and of
//! `usage.json`.
use chrono::{DateTime, Utc};

/// The session window (five hours) or the weekly one (seven days).
#[derive(Debug, Clone, PartialEq)]
pub struct LimitWindow {
    /// How much of the window is used, in percent.
    pub used_percentage: f64,
    /// When the window starts again, when the reading says so.
    pub resets_at: Option<DateTime<Utc>>,
    /// When this window was read. `None` in a reading that has not been merged yet, and in a
    /// `usage.json` from before windows kept their own time.
    pub observed_at: Option<DateTime<Utc>>,
}

/// The spend limit: extra usage paid in dollars, with a cap for the period.
#[derive(Debug, Clone, PartialEq)]
pub struct SpendLimit {
    /// How much of the cap is spent, in percent.
    pub used_percentage: f64,
    /// When the period starts again, when the reading says so.
    pub resets_at: Option<DateTime<Utc>>,
    /// Dollars spent in the period.
    pub used_usd: Option<f64>,
    /// The cap, in dollars.
    pub limit_usd: Option<f64>,
    /// `"daily"`, `"weekly"` or `"monthly"` when read from JSON; anything else is read as `None`.
    pub period: Option<String>,
    /// When this limit was read. `None` as in [`LimitWindow::observed_at`].
    pub observed_at: Option<DateTime<Utc>>,
}

/// One reading of the limits. Each of the three is there only when the reading held it.
#[derive(Debug, Clone, PartialEq)]
pub struct RateLimits {
    pub five_hour: Option<LimitWindow>,
    pub seven_day: Option<LimitWindow>,
    pub spend: Option<SpendLimit>,
}

/// What `usage.json` holds: the limits and when the file was last written.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageSnapshot {
    pub limits: RateLimits,
    pub observed_at: DateTime<Utc>,
}
