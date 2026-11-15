//! Caps how many times one kind of event is logged in a run.
use std::sync::atomic::{AtomicU64, Ordering};

/// Caps how many times one kind of event is logged in a run, so an error that repeats on every
/// frame can't churn the disk. Safe to call from any thread.
#[derive(Debug)]
pub struct LogBudget {
    limit: u32,
    used: AtomicU64,
}

impl LogBudget {
    pub fn new(limit: u32) -> LogBudget {
        LogBudget {
            limit,
            used: AtomicU64::new(0),
        }
    }

    /// The event to log, or `None` once the limit is spent. The last one allowed says later
    /// ones are dropped.
    pub fn next(&self, event: &str) -> Option<String> {
        // A u64 does not wrap in any run: it would take centuries of calls.
        let n = self.used.fetch_add(1, Ordering::Relaxed) + 1;
        let limit = u64::from(self.limit);
        if n < limit {
            Some(event.to_string())
        } else if n == limit {
            Some(format!("{event}; later ones not logged"))
        } else {
            None
        }
    }
}
