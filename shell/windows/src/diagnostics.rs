//! The shell's side of Core's diagnostics log: the file in the data folder, one entry for each
//! line, and a cap on how many a run writes. A line is fixed words that name what happened; the
//! text of an error and a token are never in one.
use crate::clock;
use crate::win::local_zone;
use chrono::FixedOffset;
use sobreiro_core::diagnostics::diagnostics_log::DiagnosticsLog;
use sobreiro_core::diagnostics::log_budget::LogBudget;
use sobreiro_core::diagnostics::log_entry;
use std::path::Path;

/// How many lines one run writes, the last of them saying that later ones are left out. Polls
/// are logged when their line changes, so a run that behaves stays far below it.
const LINES_PER_RUN: u32 = 500;

pub struct Diagnostics {
    log: DiagnosticsLog,
    budget: LogBudget,
}

impl Diagnostics {
    pub fn new(path: &Path) -> Diagnostics {
        Diagnostics {
            log: DiagnosticsLog::new(path),
            budget: LogBudget::new(LINES_PER_RUN),
        }
    }

    /// Adds a line, unless the run has written its share. A log that cannot be written is let go.
    pub fn say(&self, event: &str) {
        let Some(event) = self.budget.next(event) else {
            return;
        };
        let now = clock::now();
        let Some(offset) = local_zone::offset_at(now).or_else(|| FixedOffset::east_opt(0)) else {
            return;
        };
        let _ = self.log.append(&log_entry::format(
            now.with_timezone(&offset),
            std::process::id(),
            &event,
            None,
        ));
    }
}
