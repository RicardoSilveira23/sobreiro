/// Which days count when projecting pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaceDays {
    EveryDay,
    Weekdays,
}
