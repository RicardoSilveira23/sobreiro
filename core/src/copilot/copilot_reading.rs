//! One reading of GitHub Copilot's AI credits.
use chrono::{DateTime, Utc};

/// Where a Copilot reading came from: the endpoint just now, or the Copilot CLI's cache file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotSource {
    Live,
    Cache,
}

/// A month's AI credits: the budget, what is used (past the budget when over), and when the month
/// resets.
#[derive(Debug, Clone, PartialEq)]
pub struct CopilotReading {
    pub budget: f64,
    pub used: f64,
    pub resets_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub source: CopilotSource,
}

impl CopilotReading {
    /// How much of the budget is used, in percent. Over 100 when past the budget.
    pub fn percent(&self) -> f64 {
        self.used / self.budget * 100.0
    }
}
