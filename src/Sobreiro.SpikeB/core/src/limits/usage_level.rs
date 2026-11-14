use super::Limit;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageLevel {
    Normal,
    Warning,
    Error,
}

pub struct UsageLevels;

impl UsageLevels {
    pub const WARNING_FROM: i32 = 70;
    pub const ERROR_FROM: i32 = 90;

    pub fn for_percent(display_percent: i32) -> UsageLevel {
        if display_percent >= Self::ERROR_FROM {
            UsageLevel::Error
        } else if display_percent >= Self::WARNING_FROM {
            UsageLevel::Warning
        } else {
            UsageLevel::Normal
        }
    }

    pub fn highest(limits: &[Limit]) -> UsageLevel {
        limits
            .iter()
            .map(|l| Self::for_percent(l.display_percent()))
            .max()
            .unwrap_or(UsageLevel::Normal)
    }
}
