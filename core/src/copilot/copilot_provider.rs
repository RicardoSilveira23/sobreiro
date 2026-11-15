//! GitHub Copilot's credits as a provider: one poll of GitHub plus a read of the Copilot CLI's
//! cache file, and the keeping of both readings in memory.
//!
//! Nothing it logs holds a token or the words of an error: a line is the poll's own (fixed
//! words, see [`copilot_poll`]) and one word about the cache.
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};

use super::copilot_poll::{self, CopilotPollResult};
use super::copilot_sign_in::CopilotSignIn;
use super::copilot_usage_client::CopilotUsageClient;
use super::{CopilotReading, CopilotSource, copilot_cache, copilot_shown};
use crate::limits::{Limit, limit_list, providers};
use crate::live::live_schedule::LiveSchedule;
use crate::live::live_usage_client::Cancelled;
use crate::polling::{Keep, Provider, ProviderPoll, Trace};

/// What makes one poll: it is handed the schedule, the time, and the flag that gives the
/// poll up.
type Poll = Box<
    dyn Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<CopilotPollResult, Cancelled>
        + Send
        + Sync,
>;

/// What reads the cache file: a reading, or `None` when the file gives none.
type ReadCache = Box<dyn Fn() -> Option<CopilotReading> + Send + Sync>;

/// What the provider keeps in memory.
#[derive(Default)]
struct Kept {
    live: Option<CopilotReading>,
    cache: Option<CopilotReading>,
    /// The last line told to the log, so that a line is told again only when it changes.
    last_logged: Option<String>,
}

impl Kept {
    fn shown(&self) -> Option<&CopilotReading> {
        copilot_shown::pick(self.live.as_ref(), self.cache.as_ref())
    }
}

/// A lock is "poisoned" when a thread stopped with an error while it held it. What is inside
/// is still whole readings or none, so it is used all the same.
fn locked(kept: &Mutex<Kept>) -> MutexGuard<'_, Kept> {
    match kept.lock() {
        Ok(kept) => kept,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// GitHub Copilot's credits, as the poller sees them.
pub struct CopilotProvider {
    poll: Poll,
    read_cache: ReadCache,
    log: Trace,
    /// The readings and the last line logged.
    ///
    /// A `Mutex` lets one thread at a time at what is inside, and an `Arc` lets the provider
    /// and the [`Keep`] of a poll share it: a `Keep` outlives the call that made it, so it
    /// cannot borrow from the provider.
    kept: Arc<Mutex<Kept>>,
}

impl CopilotProvider {
    /// The provider the app uses: each poll goes through the places in `sign_ins`, in order,
    /// and asks through `client`; the cache is read from the file at `cache_path`. `log` is
    /// told one line for each poll that is kept, when the line is not the last one told.
    pub fn new(
        client: CopilotUsageClient,
        sign_ins: Vec<CopilotSignIn>,
        cache_path: &Path,
        log: Trace,
    ) -> CopilotProvider {
        let cache_path = cache_path.to_path_buf();
        CopilotProvider::with_poll(
            move |schedule, now, cancelled| {
                copilot_poll::run(&sign_ins, &client, schedule, now, cancelled)
            },
            move || copilot_cache::read(&cache_path),
            log,
        )
    }

    /// A provider whose poll is `poll` and whose cache is read by `read_cache`. The tests hand
    /// in a poll that asks nobody and a cache that reads no file. The line such a poll hands
    /// back is logged as it is: that a line is made of fixed words holds for
    /// [`copilot_poll::run`], which [`CopilotProvider::new`] uses, and not for any poll.
    pub fn with_poll(
        poll: impl Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<CopilotPollResult, Cancelled>
        + Send
        + Sync
        + 'static,
        read_cache: impl Fn() -> Option<CopilotReading> + Send + Sync + 'static,
        log: Trace,
    ) -> CopilotProvider {
        CopilotProvider {
            poll: Box::new(poll),
            read_cache: Box::new(read_cache),
            log,
            kept: Arc::new(Mutex::new(Kept::default())),
        }
    }

    /// The later of the live and the cached reading, as a copy: what is kept is behind a lock,
    /// and a reference cannot outlive it.
    pub fn shown(&self) -> Option<CopilotReading> {
        locked(&self.kept).shown().cloned()
    }

    /// The cache file, read now. It touches nothing the provider keeps, so it may run away
    /// from the thread that draws.
    pub fn read_cache(&self) -> Option<CopilotReading> {
        (self.read_cache)()
    }

    /// Shows a cache read ahead of the first poll, only while no cached reading is held.
    pub fn take_early(&self, cache: CopilotReading) {
        let mut kept = locked(&self.kept);
        if kept.cache.is_none() {
            kept.cache = Some(cache);
        }
    }
}

impl Provider for CopilotProvider {
    fn name(&self) -> &str {
        providers::COPILOT
    }

    /// One poll, then a read of the cache. The answer always has something to keep, with a
    /// reading or without one: the cache and the line for the log are kept either way.
    ///
    /// The C# turned any failure into a bad answer. A poll here gives an answer or
    /// `Err(Cancelled)`, so there is no failure to turn into one.
    fn poll(
        &self,
        schedule: &LiveSchedule,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<ProviderPoll, Cancelled> {
        // `?` hands a cancellation straight back to the caller.
        let result = (self.poll)(schedule, now, cancelled)?;
        let cache = (self.read_cache)();

        let CopilotPollResult {
            schedule,
            state,
            reading,
            log,
        } = result;
        let told = log.clone();
        let kept = self.kept.clone();
        let log_to = self.log.clone();
        let keep: Keep = Box::new(move |_| keep(&kept, &log_to, reading, cache, &told));
        Ok(ProviderPoll {
            schedule,
            state,
            log,
            keep: Some(keep),
        })
    }

    fn limits(&self, now: DateTime<Utc>) -> Vec<Limit> {
        limit_list::from_copilot(locked(&self.kept).shown(), now)
    }
}

/// Takes a poll in: the live reading when there is one, the cache read unless it found
/// nothing (the one held before stays), and a line for the log when it is not the last told.
fn keep(
    kept: &Mutex<Kept>,
    log: &Trace,
    reading: Option<CopilotReading>,
    cache: Option<CopilotReading>,
    poll_log: &str,
) {
    let line = {
        let mut kept = locked(kept);
        if let Some(reading) = reading {
            kept.live = Some(reading);
        }
        if let Some(cache) = cache {
            kept.cache = Some(cache);
        }
        let cache_word = if kept.shown().map(|shown| shown.source) == Some(CopilotSource::Cache) {
            "used"
        } else if kept.cache.is_none() {
            "none"
        } else {
            "older"
        };
        let line = format!("{poll_log} cache={cache_word}");
        if kept.last_logged.as_deref() == Some(line.as_str()) {
            return;
        }
        kept.last_logged = Some(line.clone());
        line
    };
    // Told with the lock let go, so that a log that is slow does not hold the readings.
    log(&line);
}
