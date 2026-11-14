use crate::Time;

#[derive(Clone, Debug, PartialEq)]
pub struct LimitWindow {
    pub used_percentage: f64,
    pub resets_at: Option<Time>,
    pub observed_at: Option<Time>,
}

impl LimitWindow {
    pub fn new(used_percentage: f64, resets_at: Option<Time>) -> Self {
        Self { used_percentage, resets_at, observed_at: None }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpendLimit {
    pub used_percentage: f64,
    pub resets_at: Option<Time>,
    pub used_usd: Option<f64>,
    pub limit_usd: Option<f64>,
    pub period: Option<String>,
    pub observed_at: Option<Time>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RateLimits {
    pub five_hour: Option<LimitWindow>,
    pub seven_day: Option<LimitWindow>,
    pub spend: Option<SpendLimit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsageSnapshot {
    pub limits: RateLimits,
    pub observed_at: Time,
}
