//! Decides whether two reset times belong to the same limit period.
use chrono::{DateTime, TimeDelta, Utc};

/// Two readings of one period can name reset times a little apart. Up to this far apart they are
/// the same period.
pub const TOLERANCE: TimeDelta = TimeDelta::hours(1);

/// Whether two reset times are of the same period. Two missing times are; one missing is not.
pub fn same(a: Option<DateTime<Utc>>, b: Option<DateTime<Utc>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => (a - b).abs() <= TOLERANCE,
        _ => false,
    }
}
