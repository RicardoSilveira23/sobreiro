use crate::Time;

/// Where a Copilot reading came from: the endpoint just now, or the Copilot CLI's cache file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopilotSource {
    Live,
    Cache,
}

/// A month's AI credits: the budget, what is used (past the budget when over), and when the month resets.
#[derive(Clone, Debug, PartialEq)]
pub struct CopilotReading {
    pub budget: f64,
    pub used: f64,
    pub resets_at: Option<Time>,
    pub observed_at: Time,
    pub source: CopilotSource,
}

impl CopilotReading {
    pub fn percent(&self) -> f64 {
        self.used / self.budget * 100.0
    }
}
