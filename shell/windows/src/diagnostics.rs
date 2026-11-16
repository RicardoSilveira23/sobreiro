//! The shell's side of Core's diagnostics log: the file in the data folder, one entry for each
//! line, and a cap on how many a run writes. A line is fixed words that name what happened; the
//! text of an error and a token are never in one.
use crate::clock;
use crate::win::local_zone;
use chrono::FixedOffset;
use sobreiro_core::diagnostics::diagnostics_log::DiagnosticsLog;
use sobreiro_core::diagnostics::log_budget::LogBudget;
use sobreiro_core::diagnostics::log_entry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// How many lines one run writes, the last of them saying that later ones are left out. Polls
/// are logged when their line changes, so a run that behaves stays far below it.
const LINES_PER_RUN: u32 = 500;

/// The log of this run, for the code that has no handle to it: the run's start and exit, and the
/// panic hook.
static RUN_LOG: OnceLock<Arc<Diagnostics>> = OnceLock::new();

/// Keeps the log of this run. Only the first call holds.
pub fn install(diagnostics: Arc<Diagnostics>) {
    let _ = RUN_LOG.set(diagnostics);
}

/// The log of this run, once `install` has kept it.
pub fn installed() -> Option<Arc<Diagnostics>> {
    RUN_LOG.get().cloned()
}

pub struct Diagnostics {
    path: PathBuf,
    log: DiagnosticsLog,
    budget: LogBudget,
}

impl Diagnostics {
    pub fn new(path: &Path) -> Diagnostics {
        Diagnostics {
            path: path.to_path_buf(),
            log: DiagnosticsLog::new(path),
            budget: LogBudget::new(LINES_PER_RUN),
        }
    }

    /// The log file's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Adds a line, unless the run has written its share. A log that cannot be written is let go.
    pub fn say(&self, event: &str) {
        self.write(event, None);
    }

    /// As `say`, and says whether the line was written.
    pub fn say_written(&self, event: &str) -> bool {
        self.write(event, None)
    }

    /// Adds a line with a kind name (or a place in the code) as its second line. As `say`.
    pub fn say_with_kind(&self, event: &str, kind: &str) -> bool {
        self.write(event, Some(kind))
    }

    fn write(&self, event: &str, kind: Option<&str>) -> bool {
        let Some(event) = self.budget.next(event) else {
            return false;
        };
        let now = clock::now();
        let Some(offset) = local_zone::offset_at(now).or_else(|| FixedOffset::east_opt(0)) else {
            return false;
        };
        self.log.append(&log_entry::format(
            now.with_timezone(&offset),
            std::process::id(),
            &event,
            kind,
        ))
    }
}
