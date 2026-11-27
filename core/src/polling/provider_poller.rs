//! The timing of one provider's polls: when one is due, one at a time, and what an answer
//! does.
//!
//! The poller starts no thread, waits for nothing and looks at no clock. Its caller, the
//! shell, does three things:
//!
//! 1. It calls [`ProviderPoller::tick`] now and then (1.x does every 30 seconds) and
//!    [`ProviderPoller::flyout_opened`] when the flyout opens, each with the time.
//! 2. When a call gives back a [`PollRun`], it runs it with [`PollRun::run`] on another
//!    thread, because a poll may wait ten seconds for the network.
//! 3. It hands what `run` gave back to [`ProviderPoller::done`], on the first thread again,
//!    with the time it is then. [`Done`] says whether to draw again, what to write to the
//!    debug log, and sometimes the next poll to run. `done` is also where a reading is kept,
//!    and keeping one may wait a moment for a file another program holds.
//!
//! Every poll that is handed out must be run and handed back, the one in [`Done`] too: until
//! then no other one starts, and the poller has no way to tell that a poll was lost.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};

use super::{Provider, ProviderPoll, ProviderState};
use crate::live::live_schedule::LiveSchedule;
use crate::live::live_usage_client::Cancelled;

/// A poll for the shell to run away from its own thread.
pub struct PollRun {
    provider: Arc<dyn Provider>,
    schedule: LiveSchedule,
    started: DateTime<Utc>,
    /// Set by the poller when it is stopped. `Arc` lets the poller and the poll hold the same
    /// flag; an `AtomicBool` is a yes or no that two threads may read and set.
    cancelled: Arc<AtomicBool>,
}

impl PollRun {
    /// The poll itself, with the schedule and the time it was started with. It may wait for
    /// the network. Calling it uses the run up: a poll runs once.
    pub fn run(self) -> Finished {
        let answer = self
            .provider
            .poll(&self.schedule, self.started, &self.cancelled);
        Finished {
            cancelled: self.cancelled,
            answer,
        }
    }
}

/// A poll that has run, to hand back to [`ProviderPoller::done`]. Handing it over gives it
/// away, so the same answer cannot be handed back twice.
pub struct Finished {
    cancelled: Arc<AtomicBool>,
    answer: Result<ProviderPoll, Cancelled>,
}

/// What handing an answer back came to.
#[must_use = "it may hold the next poll, which has to be run and handed back"]
pub struct Done {
    /// The state or the limits may have changed: draw again.
    pub changed: bool,
    /// A line for the debug log.
    pub log: Option<String>,
    /// The next poll to run, when the provider was stopped and started again while the last
    /// poll ran.
    pub next: Option<PollRun>,
}

impl Done {
    /// The answer was dropped: nothing changed, and there is nothing to log.
    fn dropped(next: Option<PollRun>) -> Done {
        Done {
            changed: false,
            log: None,
            next,
        }
    }
}

/// What the poller remembers of the poll that is under way.
struct Running {
    schedule: LiveSchedule,
    started: DateTime<Utc>,
    cancelled: Arc<AtomicBool>,
}

/// The timing of one provider's polls. It is used from one thread; only [`PollRun::run`]
/// happens on another.
pub struct ProviderPoller {
    provider: Arc<dyn Provider>,
    /// `None` while the provider is off.
    schedule: Option<LiveSchedule>,
    /// Set while a poll runs, until its answer is handed back.
    running: Option<Running>,
    state: ProviderState,
}

impl ProviderPoller {
    pub fn new(provider: Arc<dyn Provider>) -> ProviderPoller {
        ProviderPoller {
            provider,
            schedule: None,
            running: None,
            state: ProviderState::Ok,
        }
    }

    pub fn provider(&self) -> &Arc<dyn Provider> {
        &self.provider
    }

    /// `Ok` until the first poll answers, and `Off` from a stop until the next answer.
    pub fn state(&self) -> ProviderState {
        self.state
    }

    /// The polls in a row that brought no reading; zero while off.
    pub fn misses(&self) -> u32 {
        self.schedule.map_or(0, |schedule| schedule.misses)
    }

    /// Switches the provider on. A poll is due at once, unless one from before a stop is
    /// still under way. Does nothing when it is on already.
    #[must_use = "a poll that is handed out has to be run and handed back"]
    pub fn start(&mut self, now: DateTime<Utc>) -> Option<PollRun> {
        if self.schedule.is_some() {
            return None;
        }
        self.schedule = Some(LiveSchedule::start(now));
        self.tick(now)
    }

    /// Switches the provider off. A poll under way is told that it was given up, and its
    /// answer will be dropped.
    pub fn stop(&mut self) {
        self.schedule = None;
        self.state = ProviderState::Off;
        if let Some(running) = &self.running {
            running.cancelled.store(true, Ordering::SeqCst);
        }
    }

    /// Gives a poll to run when one is due at `now`.
    #[must_use = "a poll that is handed out has to be run and handed back"]
    pub fn tick(&mut self, now: DateTime<Utc>) -> Option<PollRun> {
        match self.schedule {
            Some(schedule) if schedule.is_due(now) => self.poll(schedule, now),
            _ => None,
        }
    }

    /// The flyout was opened at `now`: gives a poll to run when one is due, or when the
    /// schedule lets an open ask early.
    #[must_use = "a poll that is handed out has to be run and handed back"]
    pub fn flyout_opened(&mut self, now: DateTime<Utc>) -> Option<PollRun> {
        match self.schedule {
            Some(schedule) if schedule.is_due(now) || schedule.fetch_on_open(now) => {
                self.poll(schedule, now)
            }
            _ => None,
        }
    }

    /// The sign-in changed at `now` (the menu's Sign in saw the file written, or its Sign out
    /// ran): gives a poll to run at once, unless the service asked to wait.
    #[must_use = "a poll that is handed out has to be run and handed back"]
    pub fn sign_in_changed(&mut self, now: DateTime<Utc>) -> Option<PollRun> {
        match self.schedule {
            Some(schedule) if !schedule.wait_until.is_some_and(|until| now < until) => {
                self.poll(schedule, now)
            }
            _ => None,
        }
    }

    /// A new poll, unless one is under way: one at a time.
    fn poll(&mut self, schedule: LiveSchedule, now: DateTime<Utc>) -> Option<PollRun> {
        if self.running.is_some() {
            return None;
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        self.running = Some(Running {
            schedule,
            started: now,
            cancelled: cancelled.clone(),
        });
        Some(PollRun {
            provider: self.provider.clone(),
            schedule,
            started: now,
            cancelled,
        })
    }

    /// Takes the answer of the poll under way, at `now`: the time it is handled, which is
    /// the time a reading is kept with.
    pub fn done(&mut self, finished: Finished, now: DateTime<Utc>) -> Done {
        // The answer belongs to the poll under way when both hold the very same flag. An
        // answer from anywhere else does nothing.
        let Some(running) = self
            .running
            .take_if(|running| Arc::ptr_eq(&running.cancelled, &finished.cancelled))
        else {
            return Done::dropped(None);
        };

        // Stopped while it ran: the answer is dropped. If the provider was started again
        // meanwhile, its first poll is due now.
        if running.cancelled.load(Ordering::SeqCst) || self.schedule.is_none() {
            return Done::dropped(self.tick(now));
        }

        let poll = match finished.answer {
            Ok(poll) => poll,
            // A poll gives itself up only when it is told to, and this one was not told.
            // It counts as a request that failed, from the moment it started.
            Err(Cancelled) => ProviderPoll {
                schedule: running.schedule.after_failure(running.started),
                state: ProviderState::BadAnswer,
                log: format!("{} poll was cancelled", self.provider.name()),
                keep: None,
            },
        };
        self.schedule = Some(poll.schedule);
        self.state = poll.state;
        if let Some(keep) = poll.keep {
            keep(now);
        }
        Done {
            changed: true,
            log: Some(poll.log),
            next: None,
        }
    }
}

/// A poller that is no longer wanted is dropped, and Rust runs this then: a poll still under
/// way learns that it was given up.
impl Drop for ProviderPoller {
    fn drop(&mut self) {
        self.stop();
    }
}
