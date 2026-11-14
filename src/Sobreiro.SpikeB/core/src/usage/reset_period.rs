use crate::{Span, Time};

/// Decides whether two reset times belong to the same limit period.
pub struct ResetPeriod;

impl ResetPeriod {
    pub fn tolerance() -> Span {
        Span::hours(1)
    }

    pub fn same(a: Option<Time>, b: Option<Time>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => (a - b).abs() <= Self::tolerance(),
            _ => false,
        }
    }
}
