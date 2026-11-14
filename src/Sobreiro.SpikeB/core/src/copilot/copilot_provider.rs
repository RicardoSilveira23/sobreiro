use super::{CopilotCache, CopilotPoll, CopilotPollResult, CopilotReading, CopilotShown, CopilotSignIn, CopilotSource, CopilotUsageClient};
use crate::Time;
use crate::limits::{Limit, LimitList, Providers};
use crate::live::{LiveOutcome, LiveResult, LiveSchedule};
use crate::polling::{CancelToken, Kept, PollJob, Provider, ProviderPoll, ProviderState};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

type PollFn = Arc<dyn Fn(&LiveSchedule, Time, &CancelToken) -> CopilotPollResult + Send + Sync>;
type CacheFn = Arc<dyn Fn() -> Option<CopilotReading> + Send + Sync>;

/// GitHub Copilot's credits as a provider: one poll of GitHub plus a read of the Copilot CLI's cache file, and the
/// keeping of both readings in memory. Nothing it logs holds a token or an error's text.
pub struct CopilotProvider {
    poll: PollFn,
    read_cache: CacheFn,
    log: Box<dyn FnMut(&str)>,
    live: Option<CopilotReading>,
    cache: Option<CopilotReading>,
    last_logged: Option<String>,
}

impl CopilotProvider {
    /// `log` is the C# `Action<string> log`: a plain closure, called on the shell's thread.
    pub fn new(client: CopilotUsageClient, sign_ins: Vec<CopilotSignIn>, cache_path: String, log: Box<dyn FnMut(&str)>) -> CopilotProvider {
        let sign_ins = Arc::new(sign_ins);
        Self::with_polls(
            Arc::new(move |s, at, cancel| CopilotPoll::run(&sign_ins, &client, s, at, cancel)),
            Arc::new(move || CopilotCache::read(&cache_path)),
            log,
        )
    }

    /// With the poll and the cache read passed in (the C# internal constructor, for tests).
    pub fn with_polls(poll: PollFn, read_cache: CacheFn, log: Box<dyn FnMut(&str)>) -> CopilotProvider {
        CopilotProvider { poll, read_cache, log, live: None, cache: None, last_logged: None }
    }

    /// The later of the live and the cached reading. Read on the shell's thread.
    pub fn shown(&self) -> Option<CopilotReading> {
        CopilotShown::pick(self.live.as_ref(), self.cache.as_ref())
    }

    /// The cache file read, as a job safe to run off the shell's thread (the C# `ReadCache`). Hand its answer to
    /// `take_early` on the shell's thread.
    pub fn read_cache_job(&self) -> Box<dyn FnOnce() -> Option<CopilotReading> + Send + 'static> {
        let read = Arc::clone(&self.read_cache);
        Box::new(move || read())
    }

    /// Shows a cache read ahead of the first poll, only while no cached reading is held.
    pub fn take_early(&mut self, cache: CopilotReading) {
        if self.cache.is_none() {
            self.cache = Some(cache);
        }
    }

    fn keep_poll(&mut self, reading: Option<CopilotReading>, cache: Option<CopilotReading>, log: &str) {
        if let Some(r) = reading {
            self.live = Some(r);
        }
        if cache.is_some() {
            self.cache = cache; // a read that found nothing keeps the previous one
        }

        let cache_word = if self.shown().is_some_and(|s| s.source == CopilotSource::Cache) {
            "used"
        } else if self.cache.is_none() {
            "none"
        } else {
            "older"
        };
        let line = format!("{log} cache={cache_word}");
        if self.last_logged.as_deref() == Some(line.as_str()) {
            return;
        }
        (self.log)(&line);
        self.last_logged = Some(line);
    }
}

impl Provider for CopilotProvider {
    fn name(&self) -> &str {
        Providers::COPILOT
    }

    fn poll_job(&self, schedule: LiveSchedule, now: Time) -> PollJob {
        let poll = Arc::clone(&self.poll);
        let read_cache = Arc::clone(&self.read_cache);
        Box::new(move |cancel: &CancelToken| {
            let attempt = catch_unwind(AssertUnwindSafe(|| {
                let result = poll(&schedule, now, cancel);
                let cache = read_cache();
                (result, cache)
            }));
            // Anything but a result counts as a bad answer; a panic's text could hold a token.
            let (result, cache) = attempt.unwrap_or_else(|_| {
                (
                    CopilotPollResult {
                        schedule: schedule
                            .after_fetch(&LiveResult::of(LiveOutcome::Failed), now)
                            .expect("Failed is not a refusal"),
                        state: ProviderState::BadAnswer,
                        reading: None,
                        log: "copilot BadAnswer Failed".into(),
                    },
                    None,
                )
            });
            ProviderPoll {
                schedule: result.schedule,
                state: result.state,
                log: result.log.clone(),
                kept: Some(Kept::Copilot { reading: result.reading, cache, log: result.log }),
            }
        })
    }

    fn limits(&mut self, now: Time) -> Vec<Limit> {
        LimitList::from_copilot(self.shown().as_ref(), now)
    }

    fn keep(&mut self, kept: Kept, _now: Time) {
        if let Kept::Copilot { reading, cache, log } = kept {
            self.keep_poll(reading, cache, &log);
        }
    }
}
