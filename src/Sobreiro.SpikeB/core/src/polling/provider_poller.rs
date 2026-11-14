use super::{PollJob, Provider, ProviderPoll, ProviderState};
use crate::Time;
use crate::live::{LiveOutcome, LiveResult, LiveSchedule};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The counterpart of a `CancellationToken`: a flag a poll may look at between its steps. Clones share the flag.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// A poll's finished work, to hand back on the shell's thread with `ProviderPoller::done`. `Send`.
pub struct PollDone {
    id: u64,
    schedule: LiveSchedule,
    started: Time,
    name: String,
    /// Err when the job panicked: only the fact is kept, as a panic's text could hold anything.
    outcome: Result<ProviderPoll, ()>,
}

/// A poll the shell runs off its thread (`run`), then hands back on it (`ProviderPoller::done`). It owns everything
/// it needs and is `Send + 'static`.
pub struct PollRun {
    id: u64,
    job: PollJob,
    cancel: CancelToken,
    schedule: LiveSchedule,
    started: Time,
    name: String,
}

impl PollRun {
    /// The poll itself, with the schedule and time it started with. Blocks; call it on a worker thread. A panic in
    /// the job is caught and counts as a failed poll.
    pub fn run(self) -> PollDone {
        let PollRun { id, job, cancel, schedule, started, name } = self;
        let outcome = catch_unwind(AssertUnwindSafe(|| job(&cancel))).map_err(|_| ());
        PollDone { id, schedule, started, name, outcome }
    }
}

/// The timing of one provider's polls: when one is due, one at a time, and what an answer does. It starts no thread:
/// the shell runs each `PollRun` and hands it back. Not `Send`: every member but `PollRun::run` is called on one
/// thread.
pub struct ProviderPoller<P: Provider> {
    provider: P,
    run: Box<dyn FnMut(PollRun)>,
    changed: Box<dyn FnMut()>,
    trace: Option<Box<dyn FnMut(&str)>>,
    schedule: Option<LiveSchedule>, // None while off
    poll: Option<(u64, CancelToken)>, // set while a poll runs, until its answer is handled
    next_id: u64,
    disposed: bool,
    state: ProviderState,
}

impl<P: Provider> ProviderPoller<P> {
    /// `run` is called with each poll to start: move it to a worker thread. `changed` is called after an answer is
    /// kept. `trace` gets each answer's log line.
    pub fn new(provider: P, run: Box<dyn FnMut(PollRun)>, changed: Box<dyn FnMut()>, trace: Option<Box<dyn FnMut(&str)>>) -> Self {
        ProviderPoller { provider, run, changed, trace, schedule: None, poll: None, next_id: 1, disposed: false, state: ProviderState::Ok }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    /// `Provider.Limits(now)`.
    pub fn limits(&mut self, now: Time) -> Vec<crate::limits::Limit> {
        self.provider.limits(now)
    }

    /// Ok until the first poll answers.
    pub fn state(&self) -> ProviderState {
        self.state
    }

    /// The schedule's; 0 while off.
    pub fn misses(&self) -> i32 {
        self.schedule.map(|s| s.misses).unwrap_or(0)
    }

    pub fn start(&mut self, now: Time) {
        if self.disposed || self.schedule.is_some() {
            return;
        }
        self.schedule = Some(LiveSchedule::start(now));
        self.tick(now);
    }

    /// Drops a running poll's answer; the poll itself is cancelled.
    pub fn stop(&mut self) {
        self.schedule = None;
        self.state = ProviderState::Off;
        if let Some((_, cancel)) = &self.poll {
            cancel.cancel();
        }
    }

    pub fn tick(&mut self, now: Time) {
        if let Some(schedule) = self.schedule {
            if schedule.is_due(now) {
                self.start_poll(schedule, now);
            }
        }
    }

    pub fn flyout_opened(&mut self, now: Time) {
        if let Some(schedule) = self.schedule {
            if schedule.is_due(now) || schedule.fetch_on_open(now) {
                self.start_poll(schedule, now);
            }
        }
    }

    fn start_poll(&mut self, schedule: LiveSchedule, now: Time) {
        if self.disposed || self.poll.is_some() {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        let cancel = CancelToken::default();
        self.poll = Some((id, cancel.clone()));
        let job = self.provider.poll_job(schedule, now);
        (self.run)(PollRun { id, job, cancel, schedule, started: now, name: self.provider.name().to_string() });
    }

    /// Hands a finished poll back, on the shell's thread, with the time it is handled. A poll already handed back
    /// does nothing.
    pub fn done(&mut self, done: PollDone, now: Time) {
        let Some((id, cancel)) = self.poll.take_if(|(id, _)| *id == done.id) else { return }; // already handed back
        let _ = id;
        let dropped = cancel.is_cancelled();
        if self.disposed {
            return;
        }
        if dropped || self.schedule.is_none() {
            // Stopped while it ran: the answer is dropped.
            self.tick(now);
            return;
        }

        let poll = match done.outcome {
            Ok(poll) => poll,
            // A provider does not panic; if one does it counts as a failure, and its text stays out of the log.
            Err(()) => ProviderPoll {
                schedule: done
                    .schedule
                    .after_fetch(&LiveResult::of(LiveOutcome::Failed), done.started)
                    .expect("Failed is not a refusal"),
                state: ProviderState::BadAnswer,
                log: format!("{} poll threw Panic", done.name),
                kept: None,
            },
        };

        self.schedule = Some(poll.schedule);
        self.state = poll.state;
        if let Some(kept) = poll.kept {
            self.provider.keep(kept, now);
        }
        if let Some(trace) = &mut self.trace {
            trace(&poll.log);
        }
        (self.changed)();
    }

    pub fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;
        self.stop();
    }
}
