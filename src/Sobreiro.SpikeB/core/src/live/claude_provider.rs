use super::{LivePoll, LivePollResult, LiveSchedule, LiveUsageClient};
use crate::Time;
use crate::limits::{Limit, LimitList, Providers};
use crate::polling::{CancelToken, Kept, PollJob, Provider, ProviderPoll, ProviderState};
use crate::usage::{RateLimits, UsageFile, UsageMerge, UsageSnapshot};
use std::sync::Arc;
use std::time::Duration;

type PollFn = Arc<dyn Fn(&LiveSchedule, Time) -> LivePollResult + Send + Sync>;

/// Claude Code's usage as a provider: one Live usage poll, and the keeping of its reading in usage.json, where the
/// icon, flyout and alerts read it. Nothing it traces or logs holds a token or an error's text.
pub struct ClaudeProvider {
    poll: PollFn,
    usage_path: String,
    trace: Option<Box<dyn FnMut(&str)>>,
    last: Option<UsageSnapshot>,
}

impl ClaudeProvider {
    /// `trace` is the C# `Action<string>? trace`: a plain closure, called on the shell's thread.
    pub fn new(client: LiveUsageClient, credentials_path: String, usage_path: String, trace: Option<Box<dyn FnMut(&str)>>) -> ClaudeProvider {
        Self::with_poll(Arc::new(move |s, at| LivePoll::run(&credentials_path, &client, s, at)), usage_path, trace)
    }

    /// With the poll passed in (the C# internal constructor, for tests).
    pub fn with_poll(poll: PollFn, usage_path: String, trace: Option<Box<dyn FnMut(&str)>>) -> ClaudeProvider {
        ClaudeProvider { poll, usage_path, trace, last: None }
    }

    // The bridge's merge, so the icon, flyout and alerts read one state, but authoritative: within a period this
    // reading replaces the stored one whatever its percentage. A usage.json the bridge holds is tried once more;
    // still held, this reading is skipped rather than written over what the file keeps (spend).
    fn store(&mut self, limits: &RateLimits, now: Time) {
        let mut stored = None;
        if !UsageFile::try_read_for_merge(&self.usage_path, &mut stored) {
            std::thread::sleep(Duration::from_millis(50));
            if !UsageFile::try_read_for_merge(&self.usage_path, &mut stored) {
                self.trace("usage.json was busy, so this Live usage reading was not saved");
                return;
            }
        }
        let merged = UsageMerge::merge(stored.as_ref(), limits, now, true);
        if UsageFile::write(&self.usage_path, &merged).is_err() {
            self.trace("saving the Live usage reading failed: IoError");
        }
    }

    fn trace(&mut self, line: &str) {
        if let Some(t) = &mut self.trace {
            t(line);
        }
    }
}

impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        Providers::CLAUDE
    }

    fn poll_job(&self, schedule: LiveSchedule, now: Time) -> PollJob {
        let poll = Arc::clone(&self.poll);
        Box::new(move |cancel: &CancelToken| {
            if cancel.is_cancelled() {
                return ProviderPoll { schedule, state: ProviderState::Ok, log: "Live usage Cancelled".into(), kept: None };
            }
            let r = poll(&schedule, now);
            ProviderPoll { schedule: r.schedule, state: r.state, log: format!("Live usage {}", r.outcome), kept: r.limits.map(Kept::Claude) }
        })
    }

    /// An unreadable usage.json keeps the last snapshot that was read.
    fn limits(&mut self, now: Time) -> Vec<Limit> {
        self.last = UsageFile::read(&self.usage_path).or_else(|| self.last.take());
        LimitList::from(self.last.as_ref(), now)
    }

    fn keep(&mut self, kept: Kept, now: Time) {
        if let Kept::Claude(limits) = kept {
            self.store(&limits, now);
        }
    }
}
