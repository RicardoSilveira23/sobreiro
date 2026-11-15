//! Which days count when projecting pace.

/// Which days count when projecting pace. `config.json` keeps the user's choice under `pace`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaceDays {
    EveryDay,
    Weekdays,
}
