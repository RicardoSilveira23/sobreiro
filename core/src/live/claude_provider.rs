//! Claude Code's usage as a provider: one poll for it, and the keeping of its reading in
//! `usage.json`, where the pill, the flyout and the alerts read it.
//!
//! Nothing it hands to the log or the trace holds a token or the words of an error.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::live_poll::{self, LivePollResult};
use super::live_schedule::LiveSchedule;
use super::live_usage_client::{Cancelled, LiveUsageClient};
use crate::limits::{Limit, limit_list, providers};
use crate::polling::{Keep, Provider, ProviderPoll, Trace};
use crate::usage::usage_file::{self, ForMerge};
use crate::usage::usage_merge::{self, ReadingFrom};
use crate::usage::{RateLimits, UsageSnapshot};

/// How long to wait before a second try at a `usage.json` that the bridge holds.
const STORE_RETRY_DELAY: Duration = Duration::from_millis(50);

/// What makes one poll: it is handed the schedule, the time, and the flag that gives the
/// poll up.
type Poll = Box<
    dyn Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<LivePollResult, Cancelled>
        + Send
        + Sync,
>;

/// Claude Code's usage, as the poller sees it.
pub struct ClaudeProvider {
    poll: Poll,
    usage_path: PathBuf,
    trace: Option<Trace>,
    /// The last `usage.json` that could be read, shown again while the file cannot be.
    ///
    /// A `Mutex` lets one thread at a time at what is inside. The provider is shared between
    /// the thread that draws and the one a poll runs on, and Rust lets a shared value be
    /// changed only through something like this. Only [`Provider::limits`] uses it.
    last: Mutex<Option<UsageSnapshot>>,
}

impl ClaudeProvider {
    /// The provider the app uses: each poll reads the sign-in file at `credentials_path` and
    /// asks through `client`. Readings are kept in the `usage.json` at `usage_path`. `trace`
    /// is told, in a line, when a reading could not be saved.
    pub fn new(
        client: LiveUsageClient,
        credentials_path: &Path,
        usage_path: &Path,
        trace: Option<Trace>,
    ) -> ClaudeProvider {
        let credentials_path = credentials_path.to_path_buf();
        ClaudeProvider::with_poll(
            move |schedule, now, cancelled| {
                live_poll::run(&credentials_path, &client, schedule, now, cancelled)
            },
            usage_path,
            trace,
        )
    }

    /// A provider whose poll is `poll`. The tests hand in polls that read no sign-in and ask
    /// nobody.
    pub fn with_poll(
        poll: impl Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<LivePollResult, Cancelled>
        + Send
        + Sync
        + 'static,
        usage_path: &Path,
        trace: Option<Trace>,
    ) -> ClaudeProvider {
        ClaudeProvider {
            poll: Box::new(poll),
            usage_path: usage_path.to_path_buf(),
            trace,
            last: Mutex::new(None),
        }
    }
}

impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        providers::CLAUDE
    }

    fn poll(
        &self,
        schedule: &LiveSchedule,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<ProviderPoll, Cancelled> {
        let result = (self.poll)(schedule, now, cancelled)?;
        // A poll that brought a reading gives the caller a way to keep it. The reading, the
        // path and the trace move into that function, which is all it needs.
        let keep = result.limits.map(|limits| -> Keep {
            let usage_path = self.usage_path.clone();
            let trace = self.trace.clone();
            Box::new(move |at| store(&usage_path, &limits, at, trace.as_ref()))
        });
        Ok(ProviderPoll {
            schedule: result.schedule,
            state: result.state,
            log: format!("Live usage {}", result.outcome.name()),
            keep,
        })
    }

    fn limits(&self, now: DateTime<Utc>) -> Vec<Limit> {
        // A lock is "poisoned" when a thread stopped with an error while it held it. What is
        // inside is still a whole snapshot or none, so it is used all the same.
        let mut last = match self.last.lock() {
            Ok(last) => last,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(read) = usage_file::read(&self.usage_path) {
            *last = Some(read);
        }
        limit_list::from(last.as_ref(), now)
    }
}

/// Merges a reading into `usage.json`, the way the bridge does, so the pill, the flyout and
/// the alerts read one state. A Live usage reading comes from Anthropic itself: within a
/// period it replaces the stored session and weekly windows whatever their percentage.
///
/// A `usage.json` the bridge holds is tried once more, fifty milliseconds later: the one
/// place where a provider waits, on the thread that handles the answer, as in 1.x. Still
/// held, this reading is skipped and not written over what the file keeps, the spend among
/// it.
fn store(usage_path: &Path, limits: &RateLimits, now: DateTime<Utc>, trace: Option<&Trace>) {
    let say = |line: &str| {
        if let Some(trace) = trace {
            trace(line);
        }
    };
    let stored = match usage_file::read_for_merge(usage_path) {
        ForMerge::SafeToWrite(stored) => stored,
        ForMerge::NotNow => {
            std::thread::sleep(STORE_RETRY_DELAY);
            match usage_file::read_for_merge(usage_path) {
                ForMerge::SafeToWrite(stored) => stored,
                ForMerge::NotNow => {
                    say("usage.json was busy, so this Live usage reading was not saved");
                    return;
                }
            }
        }
    };
    let merged = usage_merge::merge(stored.as_ref(), limits, now, ReadingFrom::LiveUsage);
    if let Err(error) = usage_file::write(usage_path, &merged) {
        // The kind of the failure and not its words, which can name a folder of this PC.
        say(&format!(
            "saving the Live usage reading failed: {:?}",
            error.kind()
        ));
    }
}
