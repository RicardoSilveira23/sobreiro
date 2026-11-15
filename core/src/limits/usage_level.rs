//! How close to its end a limit is: the colour of its figure and of the tray icon.
use crate::limits::Limit;

/// From calm to urgent. The order matters: a later one is "higher" than an earlier one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsageLevel {
    Normal,
    Warning,
    Error,
}

/// The shown percentage from which a limit is a warning.
pub const WARNING_FROM: i32 = 70;
/// The shown percentage from which a limit is an error.
pub const ERROR_FROM: i32 = 90;

/// The level of one shown percentage.
pub fn for_percent(display_percent: i32) -> UsageLevel {
    if display_percent >= ERROR_FROM {
        UsageLevel::Error
    } else if display_percent >= WARNING_FROM {
        UsageLevel::Warning
    } else {
        UsageLevel::Normal
    }
}

/// The worst level among the limits. Normal when there are none.
pub fn highest(limits: &[Limit]) -> UsageLevel {
    limits
        .iter()
        .map(|limit| for_percent(limit.display_percent()))
        .max()
        .unwrap_or(UsageLevel::Normal)
}
