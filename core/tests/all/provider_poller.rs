//! Ported from `tests/Sobreiro.Tests/ProviderPollerTests.cs`. The tests after
//! `dropping_the_poller_cancels_the_poll_under_way` are new.
//!
//! The C# poller is handed three things to call: one that runs a poll, one that redraws and
//! one that traces. The Rust poller calls nobody. Each call gives back what its caller is to
//! do: a poll to run, and after an answer whether to redraw, the line to trace and perhaps
//! the next poll. So where a C# test counts calls, the test here counts what came back.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::limits::Limit;
use sobreiro_core::live::live_schedule::LiveSchedule;
use sobreiro_core::live::live_usage_client::Cancelled;
use sobreiro_core::live::{LiveOutcome, LiveResult};
use sobreiro_core::polling::provider_poller::{Done, PollRun, ProviderPoller};
use sobreiro_core::polling::{Provider, ProviderPoll, ProviderState};

use crate::{M, NONE, S, at_ticks, maybe_at, time};

fn now() -> DateTime<Utc> {
    time("2026-11-13T12:00:00Z")
}

fn minutes(count: i64) -> TimeDelta {
    TimeDelta::minutes(count)
}

fn seconds(count: i64) -> TimeDelta {
    TimeDelta::seconds(count)
}

fn hours(count: i64) -> TimeDelta {
    TimeDelta::hours(count)
}

fn result(outcome: LiveOutcome) -> LiveResult {
    LiveResult {
        outcome,
        limits: None,
        retry_after: None,
    }
}

/// What a fake poll answers, given what the poll was handed.
type Answer = Box<
    dyn Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<ProviderPoll, Cancelled>
        + Send
        + Sync,
>;

/// A provider that answers as the test says, and remembers what each poll was handed.
struct FakeProvider {
    answer: Mutex<Answer>,
    /// The schedule and the time of each poll, and whether it had been given up by then.
    seen: Mutex<Vec<(LiveSchedule, DateTime<Utc>, bool)>>,
}

impl FakeProvider {
    fn new() -> FakeProvider {
        FakeProvider {
            answer: Mutex::new(Box::new(|_, _, _| Ok(ok(now())))),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn answer_with(
        &self,
        answer: impl Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<ProviderPoll, Cancelled>
        + Send
        + Sync
        + 'static,
    ) {
        *self.answer.lock().unwrap() = Box::new(answer);
    }

    fn seen(&self) -> Vec<(LiveSchedule, DateTime<Utc>, bool)> {
        self.seen.lock().unwrap().clone()
    }
}

impl Provider for FakeProvider {
    fn name(&self) -> &str {
        "Fake"
    }

    fn poll(
        &self,
        schedule: &LiveSchedule,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<ProviderPoll, Cancelled> {
        let answer = (self.answer.lock().unwrap())(schedule, now, cancelled);
        // Noted after the answer, so a test can stop the poller while the poll is under way.
        let given_up = cancelled.load(Ordering::SeqCst);
        self.seen.lock().unwrap().push((*schedule, now, given_up));
        answer
    }

    fn limits(&self, _now: DateTime<Utc>) -> Vec<Limit> {
        Vec::new()
    }
}

/// A poll that went well at `at`.
fn ok(at: DateTime<Utc>) -> ProviderPoll {
    ProviderPoll {
        schedule: LiveSchedule::start(at)
            .after_fetch(&result(LiveOutcome::Ok), at)
            .unwrap(),
        state: ProviderState::Ok,
        log: "fake Ok".to_string(),
        keep: None,
    }
}

fn with_schedule(schedule: LiveSchedule) -> ProviderPoll {
    ProviderPoll {
        schedule,
        state: ProviderState::Ok,
        log: "fake Ok".to_string(),
        keep: None,
    }
}

/// The poller with its fake provider, and everything it gave back so far.
struct Harness {
    provider: Arc<FakeProvider>,
    poller: ProviderPoller,
    /// The polls handed out, in order. One is taken out when it is run.
    runs: Vec<Option<PollRun>>,
    /// How often an answer said to redraw.
    changed: usize,
    traced: Vec<String>,
    /// The times the readings were kept at.
    kept: Arc<Mutex<Vec<DateTime<Utc>>>>,
}

impl Harness {
    fn new() -> Harness {
        let provider = Arc::new(FakeProvider::new());
        Harness {
            poller: ProviderPoller::new(provider.clone()),
            provider,
            runs: Vec::new(),
            changed: 0,
            traced: Vec::new(),
            kept: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn took(&mut self, run: Option<PollRun>) {
        if let Some(run) = run {
            self.runs.push(Some(run));
        }
    }

    fn start(&mut self, at: DateTime<Utc>) {
        let run = self.poller.start(at);
        self.took(run);
    }

    fn tick(&mut self, at: DateTime<Utc>) {
        let run = self.poller.tick(at);
        self.took(run);
    }

    fn flyout_opened(&mut self, at: DateTime<Utc>) {
        let run = self.poller.flyout_opened(at);
        self.took(run);
    }

    /// Something a poll can be given as its `keep`: it notes the time it is called with.
    fn keeper(&self) -> Option<Box<dyn FnOnce(DateTime<Utc>) + Send>> {
        let kept = self.kept.clone();
        Some(Box::new(move |at| kept.lock().unwrap().push(at)))
    }

    fn kept(&self) -> Vec<DateTime<Utc>> {
        self.kept.lock().unwrap().clone()
    }

    /// Runs the poll `index` with `answer` as what the provider says, and hands the answer
    /// back at `at`.
    fn finish(
        &mut self,
        index: usize,
        answer: impl Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<ProviderPoll, Cancelled>
        + Send
        + Sync
        + 'static,
        at: DateTime<Utc>,
    ) {
        self.provider.answer_with(answer);
        let finished = self.runs[index].take().unwrap().run();
        let done = self.poller.done(finished, at);
        self.took_answer(done);
    }

    /// Notes what handing an answer back gave: a redraw, a line, the next poll.
    fn took_answer(&mut self, done: Done) {
        if done.changed {
            self.changed += 1;
        }
        self.traced.extend(done.log);
        self.took(done.next);
    }
}

#[test]
fn before_start_it_is_ok_with_no_misses() {
    let mut h = Harness::new();
    assert_eq!(h.poller.state(), ProviderState::Ok);
    assert_eq!(h.poller.misses(), 0);
    h.tick(now());
    h.flyout_opened(now());
    assert!(h.runs.is_empty());
}

#[test]
fn start_polls_at_once_and_only_once() {
    let mut h = Harness::new();
    h.start(now());
    assert_eq!(h.runs.len(), 1);
    h.start(now());
    assert_eq!(h.runs.len(), 1);
}

#[test]
fn the_poll_gets_the_schedule_and_time_it_started_with() {
    let mut h = Harness::new();
    h.start(now());
    let _finished = h.runs[0].take().unwrap().run();
    let seen = h.provider.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].1, now());
    assert_eq!(seen[0].0.next_due, now());
}

#[test]
fn one_poll_at_a_time() {
    let mut h = Harness::new();
    h.start(now());
    h.tick(now() + minutes(10));
    h.flyout_opened(now() + minutes(10));
    assert_eq!(h.runs.len(), 1);
}

#[test]
fn an_answer_sets_the_state_keeps_the_reading_and_redraws() {
    let mut h = Harness::new();
    h.start(now());
    let kept = h.kept.clone();
    h.finish(
        0,
        move |_, _, _| {
            let kept = kept.clone();
            Ok(ProviderPoll {
                schedule: LiveSchedule::start(now()).after_no_credentials(now()),
                state: ProviderState::RateLimited,
                log: "fake Ok".to_string(),
                keep: Some(Box::new(move |at| kept.lock().unwrap().push(at))),
            })
        },
        now() + seconds(3),
    );

    // By the time the caller is told to redraw, the state is set and the reading kept.
    assert_eq!(h.poller.state(), ProviderState::RateLimited);
    assert_eq!(h.poller.misses(), 1);
    assert_eq!(h.kept(), [now() + seconds(3)]);
    assert_eq!(h.changed, 1);
    assert_eq!(h.traced, ["fake Ok"]);
}

#[test]
fn misses_follow_the_schedule() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(
        0,
        |_, _, _| {
            Ok(with_schedule(
                LiveSchedule::start(now()).after_no_credentials(now()),
            ))
        },
        now(),
    );
    assert_eq!(h.poller.misses(), 1);

    h.tick(now() + minutes(1));
    assert_eq!(h.runs.len(), 2);
    h.finish(1, |_, _, _| Ok(ok(now() + minutes(1))), now() + minutes(1));
    assert_eq!(h.poller.misses(), 0);
}

#[test]
fn tick_waits_for_the_schedule() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(0, |_, _, _| Ok(ok(now())), now());
    h.tick(now() + minutes(4) + seconds(59));
    assert_eq!(h.runs.len(), 1);
    h.tick(now() + minutes(5));
    assert_eq!(h.runs.len(), 2);
}

#[test]
fn opening_the_flyout_asks_early() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(
        0,
        |_, _, _| {
            Ok(with_schedule(
                LiveSchedule::start(now()).after_no_credentials(now()),
            ))
        },
        now(),
    );
    h.tick(now() + seconds(10));
    assert_eq!(h.runs.len(), 1);
    h.flyout_opened(now() + seconds(10));
    assert_eq!(h.runs.len(), 2);
}

#[test]
fn opening_the_flyout_respects_a_wait() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(
        0,
        |_, _, _| {
            Ok(with_schedule(
                LiveSchedule::start(now()).after_failure(now()),
            ))
        },
        now(),
    );
    h.flyout_opened(now() + seconds(10));
    assert_eq!(h.runs.len(), 1);
}

// Two C# tests in one. A C# poll can end with an error of any kind, and one that was
// cancelled although nobody stopped the poller is one of them. A Rust poll gives an answer
// or `Cancelled` and has no third way to end.
#[test]
fn a_poll_given_up_without_a_stop_counts_as_a_bad_answer() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(0, |_, _, _| Err(Cancelled), now() + seconds(1));

    assert_eq!(h.poller.state(), ProviderState::BadAnswer);
    assert_eq!(h.poller.misses(), 1);
    assert_eq!(h.changed, 1);
    assert_eq!(h.traced, ["Fake poll was cancelled"]);
    // Two minutes from when the poll started, not from when its answer was handled.
    h.tick(now() + minutes(1) + seconds(59));
    assert_eq!(h.runs.len(), 1);
    h.tick(now() + minutes(2));
    assert_eq!(h.runs.len(), 2);
}

// The poll is under way on another thread when the poller is stopped, as it is in the app.
#[test]
fn stop_cancels_the_poll_and_turns_it_off() {
    let mut h = Harness::new();
    let (reached, wait_until_reached) = mpsc::channel::<()>();
    let (go_on, wait_to_go_on) = mpsc::channel::<()>();
    let (reached, wait_to_go_on) = (Mutex::new(reached), Mutex::new(wait_to_go_on));
    h.provider.answer_with(move |_, _, _| {
        reached.lock().unwrap().send(()).unwrap();
        wait_to_go_on.lock().unwrap().recv().unwrap();
        Ok(ok(now()))
    });
    h.start(now());
    let run = h.runs[0].take().unwrap();
    let under_way = std::thread::spawn(move || run.run());
    wait_until_reached.recv().unwrap();

    h.poller.stop();
    go_on.send(()).unwrap();
    let _finished = under_way.join().unwrap();

    assert!(
        h.provider.seen()[0].2,
        "the poll was not told it was given up"
    );
    assert_eq!(h.poller.state(), ProviderState::Off);
    assert_eq!(h.poller.misses(), 0);
}

#[test]
fn a_poll_stopped_while_it_ran_is_dropped() {
    let mut h = Harness::new();
    h.start(now());
    h.poller.stop();
    let keeper = Mutex::new(h.keeper());
    h.finish(
        0,
        move |_, _, _| {
            Ok(ProviderPoll {
                keep: keeper.lock().unwrap().take(),
                ..ok(now())
            })
        },
        now() + seconds(1),
    );

    assert!(h.kept().is_empty());
    assert_eq!(h.changed, 0);
    assert!(h.traced.is_empty());
    assert_eq!(h.poller.state(), ProviderState::Off);
    assert_eq!(h.runs.len(), 1);
}

#[test]
fn stop_then_start_while_a_poll_runs_waits_for_it() {
    let mut h = Harness::new();
    h.start(now());
    h.poller.stop();
    h.start(now() + seconds(1));
    assert_eq!(h.runs.len(), 1);

    let keeper = Mutex::new(h.keeper());
    h.finish(
        0,
        move |_, _, _| {
            Ok(ProviderPoll {
                keep: keeper.lock().unwrap().take(),
                ..ok(now())
            })
        },
        now() + seconds(2),
    );
    assert!(h.kept().is_empty());
    // The answer that was dropped brought the next poll with it.
    assert_eq!(h.runs.len(), 2);

    h.finish(
        1,
        |_, _, _| {
            Ok(ProviderPoll {
                state: ProviderState::Expired,
                ..ok(now() + seconds(2))
            })
        },
        now() + seconds(3),
    );
    assert_eq!(h.poller.state(), ProviderState::Expired);
}

// The C# test hands one run back twice. Here an answer can be handed back once: handing it
// over gives it away, and the compiler holds that. What can still go wrong is an answer
// handed to a poller that did not start its poll.
#[test]
fn an_answer_from_another_pollers_poll_does_nothing() {
    let mut h = Harness::new();
    let mut other = Harness::new();
    h.start(now());
    other.start(now());
    let kept = other.kept.clone();
    other.provider.answer_with(move |_, _, _| {
        let kept = kept.clone();
        Ok(ProviderPoll {
            state: ProviderState::RateLimited,
            keep: Some(Box::new(move |at| kept.lock().unwrap().push(at))),
            ..ok(now())
        })
    });
    let strange = other.runs[0].take().unwrap().run();

    let done = h.poller.done(strange, now() + minutes(5));

    assert!(!done.changed);
    assert!(done.log.is_none());
    assert!(done.next.is_none());
    assert_eq!(h.poller.state(), ProviderState::Ok);
    assert!(other.kept().is_empty());
    // Its own poll is still the one under way, and its answer is taken.
    h.tick(now() + hours(5));
    assert_eq!(h.runs.len(), 1);
    h.finish(
        0,
        |_, _, _| {
            Ok(ProviderPoll {
                state: ProviderState::Expired,
                ..ok(now())
            })
        },
        now() + hours(5),
    );
    assert_eq!(h.poller.state(), ProviderState::Expired);
    assert_eq!(h.changed, 1);
}

// The C# poller is told when it is no longer wanted, and after that does nothing. A Rust
// poller that is no longer wanted is dropped, and nothing can be asked of it afterwards.
// What is left to hold: a poll still under way learns that it was given up.
#[test]
fn dropping_the_poller_cancels_the_poll_under_way() {
    let mut h = Harness::new();
    h.start(now());
    let run = h.runs[0].take().unwrap();
    let Harness {
        poller, provider, ..
    } = h;

    drop(poller);
    let _finished = run.run();

    assert!(
        provider.seen()[0].2,
        "the poll was not told it was given up"
    );
}

// ---- New --------------------------------------------------------------------------------------

// After a stop and a start the state stays `Off` until the first answer comes: this is what
// 1.x does, kept as it is.
#[test]
fn after_a_stop_and_a_start_the_state_is_off_until_an_answer() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(0, |_, _, _| Ok(ok(now())), now());
    h.poller.stop();
    assert_eq!(h.poller.state(), ProviderState::Off);

    h.start(now() + minutes(1));
    assert_eq!(h.runs.len(), 2);
    assert_eq!(h.poller.state(), ProviderState::Off);

    h.finish(1, |_, _, _| Ok(ok(now() + minutes(1))), now() + minutes(1));
    assert_eq!(h.poller.state(), ProviderState::Ok);
}

#[test]
fn stop_before_any_start_turns_it_off() {
    let mut h = Harness::new();
    h.poller.stop();
    assert_eq!(h.poller.state(), ProviderState::Off);
    assert_eq!(h.poller.misses(), 0);
    h.tick(now());
    assert!(h.runs.is_empty());
}

// A poll that noticed it was given up answers `Cancelled`. Stopped, the poller drops that as
// it drops an answer, and counts no miss.
#[test]
fn a_stopped_poll_that_gave_up_is_dropped_too() {
    let mut h = Harness::new();
    h.start(now());
    h.poller.stop();
    h.finish(
        0,
        |_, _, cancelled| {
            assert!(cancelled.load(Ordering::SeqCst));
            Err(Cancelled)
        },
        now() + seconds(1),
    );

    assert_eq!(h.changed, 0);
    assert!(h.traced.is_empty());
    assert_eq!(h.poller.state(), ProviderState::Off);
    assert_eq!(h.poller.misses(), 0);
}

// A poll that is due starts when the flyout opens, even where an open by itself would not
// ask: here the last success is ten seconds old and a wait is running. No schedule the C#
// makes looks like this one; the poller is handed it to tell its two reasons apart.
#[test]
fn opening_the_flyout_starts_a_poll_that_is_due_whatever_else_holds() {
    let mut h = Harness::new();
    h.start(now());
    h.finish(
        0,
        |_, _, _| {
            Ok(with_schedule(LiveSchedule {
                next_due: now() + seconds(10),
                last_success: Some(now()),
                wait_until: Some(now() + minutes(5)),
                ..LiveSchedule::start(now())
            }))
        },
        now(),
    );
    h.flyout_opened(now() + seconds(9));
    assert_eq!(h.runs.len(), 1);
    h.flyout_opened(now() + seconds(10));
    assert_eq!(h.runs.len(), 2);
}

#[test]
fn the_poller_gives_its_provider() {
    let h = Harness::new();
    assert_eq!(h.poller.provider().name(), "Fake");
}

// ---- Rows the C# wrote ---------------------------------------------------------------------
//
// Everything below was written by a script from the answers of the C# of 1.1.0, which was run
// on these made-up cases on 2026-11-14. Nothing below was typed. A moment is a count of ticks
// from noon on 2026-11-09 (UTC), written with the units of `main.rs`: `5 * M + 1` is five
// minutes and one tick after it. `NONE` stands where there is no moment.

/// What the fake provider of the C# walks answers, by the name a row gives it.
fn answer_named(
    name: &str,
    kept: &Arc<Mutex<Vec<DateTime<Utc>>>>,
) -> impl Fn(&LiveSchedule, DateTime<Utc>, &AtomicBool) -> Result<ProviderPoll, Cancelled>
+ Send
+ Sync
+ 'static {
    let name = name.to_string();
    let kept = kept.clone();
    move |schedule, now, cancelled| {
        let poll = |schedule, state, log: &str| ProviderPoll {
            schedule,
            state,
            log: log.to_string(),
            keep: None,
        };
        let ok = || {
            let kept = kept.clone();
            ProviderPoll {
                keep: Some(Box::new(move |at| kept.lock().unwrap().push(at))),
                ..poll(
                    schedule.after_fetch(&result(LiveOutcome::Ok), now).unwrap(),
                    ProviderState::Ok,
                    "fake ok",
                )
            }
        };
        match name.as_str() {
            "ok" => Ok(ok()),
            "no sign-in" => Ok(poll(
                schedule.after_no_credentials(now),
                ProviderState::NoSignIn,
                "fake nocred",
            )),
            "failed" => Ok(poll(
                schedule.after_failure(now),
                ProviderState::BadAnswer,
                "fake failed",
            )),
            "limited" => Ok(poll(
                schedule
                    .after_fetch(&result(LiveOutcome::RateLimited), now)
                    .unwrap(),
                ProviderState::RateLimited,
                "fake limited",
            )),
            "refused" => Ok(poll(
                schedule.after_refused(Some(now + hours(2)), now),
                ProviderState::Refused,
                "fake refused",
            )),
            "gave up" => Err(Cancelled),
            "ok unless stopped" if cancelled.load(Ordering::SeqCst) => Err(Cancelled),
            "ok unless stopped" => Ok(ok()),
            other => panic!("a row names an answer this test does not know: {other}"),
        }
    }
}

#[test]
fn walks_of_calls_answer_as_the_csharp_did() {
    use ProviderState as P;
    // Twelve walks of forty calls, picked by a dice in the C#. A row is the call, what the
    // provider answers (for a "run", and for a "done" whose poll was not run before), and
    // when; then what is true afterwards: how many polls were started so far, the state, the
    // misses, how often a redraw was asked for, how many readings were kept and when the
    // last one was, and how many lines were traced with the last of them. "run" is the poll
    // itself, on its own; "done" hands its answer back. Now and then the clock goes back an
    // hour. Where the C# traced "Fake poll threw nothing" the Rust says
    // "Fake poll was cancelled".
    type Call = (
        &'static str,
        &'static str,
        i64,
        usize,
        P,
        u32,
        usize,
        usize,
        i64,
        usize,
        &'static str,
    );
    #[rustfmt::skip]
    let walks: &[&[Call]] = &[
        &[
            ("start", "", 90 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("open", "", 210 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "ok unless stopped", 4 * M, 1, P::Ok, 0, 1, 1, 4 * M, 1, "fake ok"),
            ("tick", "", 299 * S, 1, P::Ok, 0, 1, 1, 4 * M, 1, "fake ok"),
            ("stop", "", 2099 * S, 1, P::Off, 0, 1, 1, 4 * M, 1, "fake ok"),
            ("start", "", 2399 * S, 2, P::Off, 0, 1, 1, 4 * M, 1, "fake ok"),
            ("run", "no sign-in", 2429 * S, 2, P::Off, 0, 1, 1, 4 * M, 1, "fake ok"),
            ("done", "", 2489 * S, 2, P::NoSignIn, 1, 2, 1, 4 * M, 2, "fake nocred"),
            ("tick", "", 2789 * S, 3, P::NoSignIn, 1, 2, 1, 4 * M, 2, "fake nocred"),
            ("run", "failed", 2848 * S, 3, P::NoSignIn, 1, 2, 1, 4 * M, 2, "fake nocred"),
            ("stop", "", -752 * S, 3, P::Off, 0, 2, 1, 4 * M, 2, "fake nocred"),
            ("done", "", 1048 * S, 3, P::Off, 0, 2, 1, 4 * M, 2, "fake nocred"),
            ("start", "", 1107 * S, 4, P::Off, 0, 2, 1, 4 * M, 2, "fake nocred"),
            ("done", "refused", 1117 * S, 4, P::Refused, 1, 3, 1, 4 * M, 3, "fake refused"),
            ("tick", "", 1176 * S, 5, P::Refused, 1, 3, 1, 4 * M, 3, "fake refused"),
            ("done", "failed", 1236 * S + 1, 5, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 1246 * S + 1, 5, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 1247 * S + 1, 5, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("open", "", 1547 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 1667 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 3467 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 3526 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("run", "refused", 3527 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("open", "", 3827 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 3857 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 5657 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("tick", "", 5957 * S + 1, 6, P::BadAnswer, 2, 4, 1, 4 * M, 4, "fake failed"),
            ("done", "", 5958 * S + 1, 6, P::Refused, 3, 5, 1, 4 * M, 5, "fake refused"),
            ("tick", "", 6018 * S + 2, 7, P::Refused, 3, 5, 1, 4 * M, 5, "fake refused"),
            ("done", "no sign-in", 6078 * S + 2, 7, P::NoSignIn, 4, 6, 1, 4 * M, 6, "fake nocred"),
            ("tick", "", 7878 * S + 2, 8, P::NoSignIn, 4, 6, 1, 4 * M, 6, "fake nocred"),
            ("done", "refused", 7888 * S + 2, 8, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("tick", "", 7889 * S + 2, 8, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("tick", "", 7899 * S + 2, 8, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("tick", "", 8199 * S + 2, 9, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("tick", "", 9999 * S + 2, 9, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("run", "limited", 10299 * S + 2, 9, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("start", "", 10309 * S + 2, 9, P::Refused, 5, 7, 1, 4 * M, 7, "fake refused"),
            ("done", "", 10319 * S + 2, 9, P::RateLimited, 6, 8, 1, 4 * M, 8, "fake limited"),
            ("start", "", 10379 * S + 2, 9, P::RateLimited, 6, 8, 1, 4 * M, 8, "fake limited"),
        ],
        &[
            ("start", "", 2 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "failed", 3 * M + 1, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 181 * S + 1, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 182 * S + 1, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 242 * S + 2, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("open", "", 542 * S + 2, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("done", "limited", 602 * S + 3, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2998 * S + 3, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2938 * S + 4, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -1138 * S + 4, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("open", "", -1108 * S + 4, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -1048 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4648 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4589 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4579 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4520 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4400 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("open", "", -4400 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4341 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4341 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -4331 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("open", "", -2531 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2231 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2221 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2211 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2151 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("open", "", -2091 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2081 * S + 5, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2021 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -2021 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("start", "", -1721 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5321 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5320 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5319 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5319 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5199 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5189 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("open", "", -5159 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5099 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
            ("tick", "", -5089 * S + 6, 2, P::RateLimited, 2, 2, 0, NONE, 2, "fake limited"),
        ],
        &[
            ("start", "", 3 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 4 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("run", "failed", 5 * M + 1, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "", 10 * M + 1, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 601 * S + 1, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("done", "refused", 901 * S + 1, 2, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", 2701 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("run", "no sign-in", 3001 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("open", "", 3031 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("open", "", 3090 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("stop", "", 3150 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake refused"),
            ("start", "", 3160 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake refused"),
            ("done", "", 3280 * S + 1, 4, P::Off, 0, 2, 0, NONE, 2, "fake refused"),
            ("open", "", 3580 * S + 1, 4, P::Off, 0, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", 3639 * S + 1, 4, P::Off, 0, 2, 0, NONE, 2, "fake refused"),
            ("done", "refused", 39 * S + 1, 4, P::Refused, 1, 3, 0, NONE, 3, "fake refused"),
            ("tick", "", 69 * S + 1, 4, P::Refused, 1, 3, 0, NONE, 3, "fake refused"),
            ("tick", "", 129 * S + 1, 4, P::Refused, 1, 3, 0, NONE, 3, "fake refused"),
            ("tick", "", -3471 * S + 1, 4, P::Refused, 1, 3, 0, NONE, 3, "fake refused"),
            ("stop", "", -3471 * S + 1, 4, P::Off, 0, 3, 0, NONE, 3, "fake refused"),
            ("start", "", -3461 * S + 1, 5, P::Off, 0, 3, 0, NONE, 3, "fake refused"),
            ("done", "gave up", -3402 * S + 1, 5, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("start", "", -1602 * S + 1, 5, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("start", "", -1302 * S + 1, 5, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("tick", "", -1242 * S + 1, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("tick", "", -942 * S + 1, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("open", "", -4542 * S + 1, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("tick", "", -4482 * S + 2, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("tick", "", -4182 * S + 2, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("open", "", -4122 * S + 3, 6, P::BadAnswer, 1, 4, 0, NONE, 4, "Fake poll was cancelled"),
            ("done", "no sign-in", -4062 * S + 4, 6, P::NoSignIn, 2, 5, 0, NONE, 5, "fake nocred"),
            ("open", "", -4052 * S + 4, 7, P::NoSignIn, 2, 5, 0, NONE, 5, "fake nocred"),
            ("run", "limited", -3992 * S + 5, 7, P::NoSignIn, 2, 5, 0, NONE, 5, "fake nocred"),
            ("done", "", -3692 * S + 5, 7, P::RateLimited, 3, 6, 0, NONE, 6, "fake limited"),
            ("tick", "", -3633 * S + 5, 8, P::RateLimited, 3, 6, 0, NONE, 6, "fake limited"),
            ("done", "no sign-in", -1833 * S + 5, 8, P::NoSignIn, 4, 7, 0, NONE, 7, "fake nocred"),
            ("tick", "", -5433 * S + 5, 8, P::NoSignIn, 4, 7, 0, NONE, 7, "fake nocred"),
            ("tick", "", -5433 * S + 5, 8, P::NoSignIn, 4, 7, 0, NONE, 7, "fake nocred"),
            ("tick", "", -5423 * S + 5, 8, P::NoSignIn, 4, 7, 0, NONE, 7, "fake nocred"),
            ("tick", "", -5123 * S + 5, 8, P::NoSignIn, 4, 7, 0, NONE, 7, "fake nocred"),
        ],
        &[
            ("start", "", 179 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("run", "failed", 209 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 509 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 569 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "", 869 * S, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 1169 * S, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 1169 * S, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("stop", "", 1179 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake failed"),
            ("done", "ok", 1479 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 1489 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 1489 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake failed"),
            ("start", "", 1489 * S, 3, P::Off, 0, 1, 0, NONE, 1, "fake failed"),
            ("done", "no sign-in", 1609 * S, 3, P::NoSignIn, 1, 2, 0, NONE, 2, "fake nocred"),
            ("tick", "", 1639 * S, 4, P::NoSignIn, 1, 2, 0, NONE, 2, "fake nocred"),
            ("tick", "", 1698 * S, 4, P::NoSignIn, 1, 2, 0, NONE, 2, "fake nocred"),
            ("tick", "", 3498 * S, 4, P::NoSignIn, 1, 2, 0, NONE, 2, "fake nocred"),
            ("done", "ok unless stopped", 3558 * S, 4, P::Ok, 0, 3, 1, 3558 * S, 3, "fake ok"),
            ("tick", "", 3568 * S, 5, P::Ok, 0, 3, 1, 3558 * S, 3, "fake ok"),
            ("done", "refused", 3868 * S, 5, P::Refused, 1, 4, 1, 3558 * S, 4, "fake refused"),
            ("tick", "", 3878 * S, 6, P::Refused, 1, 4, 1, 3558 * S, 4, "fake refused"),
            ("tick", "", 3879 * S, 6, P::Refused, 1, 4, 1, 3558 * S, 4, "fake refused"),
            ("run", "limited", 3939 * S, 6, P::Refused, 1, 4, 1, 3558 * S, 4, "fake refused"),
            ("done", "", 3999 * S, 6, P::RateLimited, 2, 5, 1, 3558 * S, 5, "fake limited"),
            ("tick", "", 4059 * S + 1, 6, P::RateLimited, 2, 5, 1, 3558 * S, 5, "fake limited"),
            ("tick", "", 4060 * S + 1, 6, P::RateLimited, 2, 5, 1, 3558 * S, 5, "fake limited"),
            ("tick", "", 4120 * S + 2, 6, P::RateLimited, 2, 5, 1, 3558 * S, 5, "fake limited"),
            ("open", "", 4240 * S + 2, 7, P::RateLimited, 2, 5, 1, 3558 * S, 5, "fake limited"),
            ("done", "ok", 4241 * S + 2, 7, P::Ok, 0, 6, 2, 4241 * S + 2, 6, "fake ok"),
            ("tick", "", 4541 * S + 2, 8, P::Ok, 0, 6, 2, 4241 * S + 2, 6, "fake ok"),
            ("run", "failed", 4571 * S + 2, 8, P::Ok, 0, 6, 2, 4241 * S + 2, 6, "fake ok"),
            ("tick", "", 4572 * S + 2, 8, P::Ok, 0, 6, 2, 4241 * S + 2, 6, "fake ok"),
            ("done", "", 6372 * S + 2, 8, P::BadAnswer, 1, 7, 2, 4241 * S + 2, 7, "fake failed"),
            ("tick", "", 6492 * S + 2, 9, P::BadAnswer, 1, 7, 2, 4241 * S + 2, 7, "fake failed"),
            ("tick", "", 8292 * S + 2, 9, P::BadAnswer, 1, 7, 2, 4241 * S + 2, 7, "fake failed"),
            ("open", "", 4692 * S + 2, 9, P::BadAnswer, 1, 7, 2, 4241 * S + 2, 7, "fake failed"),
            ("done", "failed", 4752 * S + 2, 9, P::BadAnswer, 2, 8, 2, 4241 * S + 2, 8, "fake failed"),
            ("tick", "", 4812 * S + 2, 9, P::BadAnswer, 2, 8, 2, 4241 * S + 2, 8, "fake failed"),
            ("tick", "", 5112 * S + 2, 9, P::BadAnswer, 2, 8, 2, 4241 * S + 2, 8, "fake failed"),
            ("tick", "", 5171 * S + 2, 9, P::BadAnswer, 2, 8, 2, 4241 * S + 2, 8, "fake failed"),
            ("tick", "", 5471 * S + 2, 9, P::BadAnswer, 2, 8, 2, 4241 * S + 2, 8, "fake failed"),
        ],
        &[
            ("start", "", -59 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", -58 * M + 1, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "no sign-in", -53 * M + 1, 1, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("tick", "", -48 * M + 1, 2, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("stop", "", -46 * M + 1, 2, P::Off, 0, 1, 0, NONE, 1, "fake nocred"),
            ("done", "limited", -45 * M + 1, 2, P::Off, 0, 1, 0, NONE, 1, "fake nocred"),
            ("start", "", -2670 * S + 1, 3, P::Off, 0, 1, 0, NONE, 1, "fake nocred"),
            ("tick", "", -44 * M + 1, 3, P::Off, 0, 1, 0, NONE, 1, "fake nocred"),
            ("done", "ok unless stopped", -2581 * S + 1, 3, P::Ok, 0, 2, 1, -2581 * S + 1, 2, "fake ok"),
            ("tick", "", -2521 * S + 1, 3, P::Ok, 0, 2, 1, -2581 * S + 1, 2, "fake ok"),
            ("tick", "", -2462 * S + 1, 3, P::Ok, 0, 2, 1, -2581 * S + 1, 2, "fake ok"),
            ("tick", "", -2342 * S + 1, 4, P::Ok, 0, 2, 1, -2581 * S + 1, 2, "fake ok"),
            ("tick", "", -2283 * S + 1, 4, P::Ok, 0, 2, 1, -2581 * S + 1, 2, "fake ok"),
            ("done", "ok", -2163 * S + 1, 4, P::Ok, 0, 3, 2, -2163 * S + 1, 3, "fake ok"),
            ("open", "", -2104 * S + 1, 5, P::Ok, 0, 3, 2, -2163 * S + 1, 3, "fake ok"),
            ("run", "no sign-in", -2104 * S + 1, 5, P::Ok, 0, 3, 2, -2163 * S + 1, 3, "fake ok"),
            ("tick", "", -2104 * S + 1, 5, P::Ok, 0, 3, 2, -2163 * S + 1, 3, "fake ok"),
            ("tick", "", -2044 * S + 2, 5, P::Ok, 0, 3, 2, -2163 * S + 1, 3, "fake ok"),
            ("done", "", -1744 * S + 2, 5, P::NoSignIn, 1, 4, 2, -2163 * S + 1, 4, "fake nocred"),
            ("tick", "", 56 * S + 2, 6, P::NoSignIn, 1, 4, 2, -2163 * S + 1, 4, "fake nocred"),
            ("done", "no sign-in", 116 * S + 2, 6, P::NoSignIn, 2, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("open", "", -3484 * S + 2, 6, P::NoSignIn, 2, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("open", "", -1684 * S + 2, 7, P::NoSignIn, 2, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("stop", "", -1564 * S + 2, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("start", "", -1444 * S + 2, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("tick", "", -1144 * S + 2, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("stop", "", -844 * S + 2, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("start", "", -784 * S + 3, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("run", "limited", -484 * S + 3, 7, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("done", "", -364 * S + 3, 8, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("stop", "", 1436 * S + 3, 8, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("done", "no sign-in", 1556 * S + 3, 8, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("tick", "", 3356 * S + 3, 8, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("tick", "", 5156 * S + 3, 8, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("start", "", 5276 * S + 3, 9, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("start", "", 5276 * S + 3, 9, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("tick", "", 5396 * S + 3, 9, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("tick", "", 5696 * S + 3, 9, P::Off, 0, 5, 2, -2163 * S + 1, 5, "fake nocred"),
            ("done", "no sign-in", 5755 * S + 3, 9, P::NoSignIn, 1, 6, 2, -2163 * S + 1, 6, "fake nocred"),
            ("tick", "", 7555 * S + 3, 10, P::NoSignIn, 1, 6, 2, -2163 * S + 1, 6, "fake nocred"),
        ],
        &[
            ("start", "", 5 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", -55 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "refused", -3270 * S, 1, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("tick", "", -3210 * S, 1, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("tick", "", -53 * M, 1, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("tick", "", -3179 * S, 1, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("tick", "", -3178 * S, 1, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("open", "", -2878 * S, 2, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("tick", "", -2868 * S, 2, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("open", "", -2748 * S, 2, P::Refused, 1, 1, 0, NONE, 1, "fake refused"),
            ("stop", "", -2628 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake refused"),
            ("done", "failed", -2508 * S, 2, P::Off, 0, 1, 0, NONE, 1, "fake refused"),
            ("start", "", -2507 * S, 3, P::Off, 0, 1, 0, NONE, 1, "fake refused"),
            ("done", "failed", -2448 * S, 3, P::BadAnswer, 1, 2, 0, NONE, 2, "fake failed"),
            ("start", "", -6048 * S, 3, P::BadAnswer, 1, 2, 0, NONE, 2, "fake failed"),
            ("stop", "", -5988 * S, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("tick", "", -5988 * S, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("stop", "", -5978 * S, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("start", "", -5678 * S, 4, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("run", "ok", -5378 * S, 4, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("done", "", -5368 * S, 4, P::Ok, 0, 3, 1, -5368 * S, 3, "fake ok"),
            ("open", "", -5309 * S, 5, P::Ok, 0, 3, 1, -5368 * S, 3, "fake ok"),
            ("tick", "", -5308 * S, 5, P::Ok, 0, 3, 1, -5368 * S, 3, "fake ok"),
            ("done", "refused", -5008 * S, 5, P::Refused, 1, 4, 1, -5368 * S, 4, "fake refused"),
            ("tick", "", -4888 * S, 6, P::Refused, 1, 4, 1, -5368 * S, 4, "fake refused"),
            ("done", "refused", -4588 * S, 6, P::Refused, 2, 5, 1, -5368 * S, 5, "fake refused"),
            ("open", "", -4528 * S, 7, P::Refused, 2, 5, 1, -5368 * S, 5, "fake refused"),
            ("tick", "", -4228 * S, 7, P::Refused, 2, 5, 1, -5368 * S, 5, "fake refused"),
            ("start", "", -3928 * S, 7, P::Refused, 2, 5, 1, -5368 * S, 5, "fake refused"),
            ("stop", "", -7528 * S, 7, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("start", "", -7408 * S, 7, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("run", "no sign-in", -7348 * S + 1, 7, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("done", "", -7348 * S + 1, 8, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("run", "no sign-in", -7348 * S + 1, 8, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("open", "", -7288 * S + 2, 8, P::Off, 0, 5, 1, -5368 * S, 5, "fake refused"),
            ("done", "", -7229 * S + 2, 8, P::NoSignIn, 1, 6, 1, -5368 * S, 6, "fake nocred"),
            ("tick", "", -7228 * S + 2, 9, P::NoSignIn, 1, 6, 1, -5368 * S, 6, "fake nocred"),
            ("done", "ok", -6928 * S + 2, 9, P::Ok, 0, 7, 2, -6928 * S + 2, 7, "fake ok"),
            ("tick", "", -6918 * S + 2, 10, P::Ok, 0, 7, 2, -6928 * S + 2, 7, "fake ok"),
            ("tick", "", -6917 * S + 2, 10, P::Ok, 0, 7, 2, -6928 * S + 2, 7, "fake ok"),
        ],
        &[
            ("start", "", 2 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 32 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "failed", 32 * M, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 33 * M, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("done", "refused", 35 * M, 2, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", 2130 * S, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", 2190 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", 2249 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("run", "gave up", 2309 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("tick", "", -1291 * S + 1, 3, P::Refused, 2, 2, 0, NONE, 2, "fake refused"),
            ("done", "", -1290 * S + 1, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", -1290 * S + 1, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 510 * S + 1, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 810 * S + 1, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 14 * M + 1, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 15 * M + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 17 * M + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 22 * M + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 23 * M + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("open", "", 1410 * S + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("open", "", 1420 * S + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 1421 * S + 2, 3, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("tick", "", 3221 * S + 2, 4, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("open", "", 3231 * S + 2, 4, P::BadAnswer, 3, 3, 0, NONE, 3, "Fake poll was cancelled"),
            ("done", "failed", 3531 * S + 2, 4, P::BadAnswer, 4, 4, 0, NONE, 4, "fake failed"),
            ("tick", "", 3531 * S + 2, 5, P::BadAnswer, 4, 4, 0, NONE, 4, "fake failed"),
            ("tick", "", 5331 * S + 2, 5, P::BadAnswer, 4, 4, 0, NONE, 4, "fake failed"),
            ("done", "ok", 5361 * S + 2, 5, P::Ok, 0, 5, 1, 5361 * S + 2, 5, "fake ok"),
            ("tick", "", 5481 * S + 2, 6, P::Ok, 0, 5, 1, 5361 * S + 2, 5, "fake ok"),
            ("done", "refused", 5781 * S + 2, 6, P::Refused, 1, 6, 1, 5361 * S + 2, 6, "fake refused"),
            ("tick", "", 5840 * S + 2, 7, P::Refused, 1, 6, 1, 5361 * S + 2, 6, "fake refused"),
            ("tick", "", 5960 * S + 2, 7, P::Refused, 1, 6, 1, 5361 * S + 2, 6, "fake refused"),
            ("run", "failed", 5970 * S + 2, 7, P::Refused, 1, 6, 1, 5361 * S + 2, 6, "fake refused"),
            ("done", "", 6030 * S + 2, 7, P::BadAnswer, 2, 7, 1, 5361 * S + 2, 7, "fake failed"),
            ("tick", "", 7830 * S + 2, 8, P::BadAnswer, 2, 7, 1, 5361 * S + 2, 7, "fake failed"),
            ("start", "", 8130 * S + 2, 8, P::BadAnswer, 2, 7, 1, 5361 * S + 2, 7, "fake failed"),
            ("open", "", 8140 * S + 2, 8, P::BadAnswer, 2, 7, 1, 5361 * S + 2, 7, "fake failed"),
            ("done", "failed", 9940 * S + 2, 8, P::BadAnswer, 3, 8, 1, 5361 * S + 2, 8, "fake failed"),
            ("tick", "", 9950 * S + 2, 9, P::BadAnswer, 3, 8, 1, 5361 * S + 2, 8, "fake failed"),
            ("start", "", 9980 * S + 2, 9, P::BadAnswer, 3, 8, 1, 5361 * S + 2, 8, "fake failed"),
        ],
        &[
            ("start", "", 10 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("run", "failed", 310 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 340 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 350 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("open", "", 380 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "", 2180 * S, 1, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("tick", "", 2180 * S, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("start", "", 2240 * S + 1, 2, P::BadAnswer, 1, 1, 0, NONE, 1, "fake failed"),
            ("done", "failed", 2270 * S + 1, 2, P::BadAnswer, 2, 2, 0, NONE, 2, "fake failed"),
            ("stop", "", 2300 * S + 1, 2, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("tick", "", 2360 * S + 1, 2, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("start", "", 2660 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("tick", "", 2960 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("stop", "", 4760 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("run", "refused", 4880 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("done", "", 4939 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("tick", "", 4969 * S + 1, 3, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("start", "", 1369 * S + 1, 4, P::Off, 0, 2, 0, NONE, 2, "fake failed"),
            ("done", "ok unless stopped", 1429 * S + 2, 4, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 1489 * S + 3, 4, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 1789 * S + 3, 5, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("start", "", 2089 * S + 3, 5, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 2209 * S + 3, 5, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 4009 * S + 3, 5, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 4309 * S + 3, 5, P::Ok, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("stop", "", 4339 * S + 3, 5, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("done", "no sign-in", 4340 * S + 3, 5, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 4341 * S + 3, 5, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("tick", "", 4461 * S + 3, 5, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("start", "", 4462 * S + 3, 6, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("run", "gave up", 4522 * S + 3, 6, P::Off, 0, 3, 1, 1429 * S + 2, 3, "fake ok"),
            ("done", "", 4523 * S + 3, 6, P::BadAnswer, 1, 4, 1, 1429 * S + 2, 4, "Fake poll was cancelled"),
            ("tick", "", 4643 * S + 3, 7, P::BadAnswer, 1, 4, 1, 1429 * S + 2, 4, "Fake poll was cancelled"),
            ("done", "failed", 4763 * S + 3, 7, P::BadAnswer, 2, 5, 1, 1429 * S + 2, 5, "fake failed"),
            ("open", "", 4823 * S + 3, 8, P::BadAnswer, 2, 5, 1, 1429 * S + 2, 5, "fake failed"),
            ("run", "limited", 5123 * S + 3, 8, P::BadAnswer, 2, 5, 1, 1429 * S + 2, 5, "fake failed"),
            ("start", "", 5182 * S + 3, 8, P::BadAnswer, 2, 5, 1, 1429 * S + 2, 5, "fake failed"),
            ("done", "", 1582 * S + 3, 8, P::RateLimited, 3, 6, 1, 1429 * S + 2, 6, "fake limited"),
            ("tick", "", 1882 * S + 3, 8, P::RateLimited, 3, 6, 1, 1429 * S + 2, 6, "fake limited"),
            ("tick", "", 2182 * S + 3, 8, P::RateLimited, 3, 6, 1, 1429 * S + 2, 6, "fake limited"),
        ],
        &[
            ("start", "", 6 * M, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "ok unless stopped", 8 * M, 1, P::Ok, 0, 1, 1, 8 * M, 1, "fake ok"),
            ("tick", "", 13 * M, 2, P::Ok, 0, 1, 1, 8 * M, 1, "fake ok"),
            ("done", "limited", 43 * M, 2, P::RateLimited, 1, 2, 1, 8 * M, 2, "fake limited"),
            ("tick", "", 48 * M, 3, P::RateLimited, 1, 2, 1, 8 * M, 2, "fake limited"),
            ("tick", "", -12 * M, 3, P::RateLimited, 1, 2, 1, 8 * M, 2, "fake limited"),
            ("done", "ok unless stopped", -690 * S, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -570 * S, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -569 * S, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -509 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -209 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -150 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("open", "", -90 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -89 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", -89 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 211 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 212 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 242 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 362 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 362 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 372 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 431 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("open", "", 491 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 492 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 493 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 493 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 494 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("open", "", 495 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 615 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 674 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 2474 * S + 1, 3, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 4274 * S + 1, 4, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("run", "refused", 4394 * S + 1, 4, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("tick", "", 4424 * S + 1, 4, P::Ok, 0, 3, 2, -690 * S, 3, "fake ok"),
            ("done", "", 6224 * S + 1, 4, P::Refused, 1, 4, 2, -690 * S, 4, "fake refused"),
            ("open", "", 6225 * S + 1, 5, P::Refused, 1, 4, 2, -690 * S, 4, "fake refused"),
            ("start", "", 6345 * S + 1, 5, P::Refused, 1, 4, 2, -690 * S, 4, "fake refused"),
            ("run", "refused", 6645 * S + 1, 5, P::Refused, 1, 4, 2, -690 * S, 4, "fake refused"),
            ("done", "", 6945 * S + 1, 5, P::Refused, 2, 5, 2, -690 * S, 5, "fake refused"),
            ("tick", "", 7004 * S + 1, 6, P::Refused, 2, 5, 2, -690 * S, 5, "fake refused"),
        ],
        &[
            ("start", "", M + 1, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("open", "", 2 * M + 2, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 179 * S + 2, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("run", "no sign-in", 299 * S + 2, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 359 * S + 2, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "", 359 * S + 2, 1, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("tick", "", 419 * S + 2, 2, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("open", "", 719 * S + 2, 2, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("open", "", 779 * S + 2, 2, P::NoSignIn, 1, 1, 0, NONE, 1, "fake nocred"),
            ("done", "no sign-in", 1079 * S + 2, 2, P::NoSignIn, 2, 2, 0, NONE, 2, "fake nocred"),
            ("tick", "", 18 * M + 2, 3, P::NoSignIn, 2, 2, 0, NONE, 2, "fake nocred"),
            ("open", "", 1081 * S + 2, 3, P::NoSignIn, 2, 2, 0, NONE, 2, "fake nocred"),
            ("run", "refused", 1111 * S + 2, 3, P::NoSignIn, 2, 2, 0, NONE, 2, "fake nocred"),
            ("done", "", 1171 * S + 2, 3, P::Refused, 3, 3, 0, NONE, 3, "fake refused"),
            ("tick", "", 1231 * S + 2, 4, P::Refused, 3, 3, 0, NONE, 3, "fake refused"),
            ("done", "ok unless stopped", 1351 * S + 2, 4, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("tick", "", 1351 * S + 2, 4, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("tick", "", 1381 * S + 2, 4, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("tick", "", 3181 * S + 2, 5, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("tick", "", 3241 * S + 2, 5, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("tick", "", 55 * M + 2, 5, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("open", "", 3330 * S + 2, 5, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("run", "no sign-in", 3331 * S + 2, 5, P::Ok, 0, 4, 1, 1351 * S + 2, 4, "fake ok"),
            ("done", "", -269 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("tick", "", 31 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("tick", "", 32 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("start", "", 42 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("tick", "", 102 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("tick", "", 162 * S + 2, 5, P::NoSignIn, 1, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("stop", "", 462 * S + 2, 5, P::Off, 0, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("start", "", 582 * S + 2, 6, P::Off, 0, 5, 1, 1351 * S + 2, 5, "fake nocred"),
            ("done", "no sign-in", -3018 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2959 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2899 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("start", "", -2839 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2780 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2720 * S + 2, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2660 * S + 3, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2360 * S + 3, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
            ("tick", "", -2300 * S + 3, 6, P::NoSignIn, 1, 6, 1, 1351 * S + 2, 6, "fake nocred"),
        ],
        &[
            ("start", "", 90 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "ok unless stopped", 149 * S, 1, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("tick", "", 209 * S, 1, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("tick", "", 239 * S, 1, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("tick", "", 4 * M, 1, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("tick", "", 4 * M, 1, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("open", "", 9 * M, 2, P::Ok, 0, 1, 1, 149 * S, 1, "fake ok"),
            ("done", "limited", 14 * M, 2, P::RateLimited, 1, 2, 1, 149 * S, 2, "fake limited"),
            ("tick", "", 16 * M, 3, P::RateLimited, 1, 2, 1, 149 * S, 2, "fake limited"),
            ("run", "limited", 16 * M, 3, P::RateLimited, 1, 2, 1, 149 * S, 2, "fake limited"),
            ("tick", "", 21 * M, 3, P::RateLimited, 1, 2, 1, 149 * S, 2, "fake limited"),
            ("done", "", 22 * M + 1, 3, P::RateLimited, 2, 3, 1, 149 * S, 3, "fake limited"),
            ("tick", "", 52 * M + 1, 4, P::RateLimited, 2, 3, 1, 149 * S, 3, "fake limited"),
            ("open", "", 52 * M + 1, 4, P::RateLimited, 2, 3, 1, 149 * S, 3, "fake limited"),
            ("done", "ok", 53 * M + 1, 4, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("tick", "", 3210 * S + 1, 4, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("tick", "", 3220 * S + 1, 4, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("tick", "", 3340 * S + 1, 4, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("tick", "", 5140 * S + 1, 5, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("tick", "", 5440 * S + 1, 5, P::Ok, 0, 4, 2, 53 * M + 1, 4, "fake ok"),
            ("done", "no sign-in", 5740 * S + 1, 5, P::NoSignIn, 1, 5, 2, 53 * M + 1, 5, "fake nocred"),
            ("tick", "", 5741 * S + 1, 6, P::NoSignIn, 1, 5, 2, 53 * M + 1, 5, "fake nocred"),
            ("done", "gave up", 5741 * S + 1, 6, P::BadAnswer, 2, 6, 2, 53 * M + 1, 6, "Fake poll was cancelled"),
            ("tick", "", 7541 * S + 1, 7, P::BadAnswer, 2, 6, 2, 53 * M + 1, 6, "Fake poll was cancelled"),
            ("done", "no sign-in", 7551 * S + 1, 7, P::NoSignIn, 3, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("tick", "", 9351 * S + 1, 8, P::NoSignIn, 3, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("open", "", 9651 * S + 1, 8, P::NoSignIn, 3, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("open", "", 9710 * S + 1, 8, P::NoSignIn, 3, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("start", "", 162 * M + 1, 8, P::NoSignIn, 3, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("stop", "", 164 * M + 1, 8, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("run", "ok unless stopped", 169 * M + 1, 8, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("done", "", 169 * M + 1, 8, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("tick", "", 171 * M + 1, 8, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("start", "", 10290 * S + 1, 9, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("stop", "", 10290 * S + 1, 9, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("run", "refused", 12090 * S + 1, 9, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("done", "", 12100 * S + 1, 9, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("start", "", 8500 * S + 1, 10, P::Off, 0, 7, 2, 53 * M + 1, 7, "fake nocred"),
            ("done", "ok", 8560 * S + 2, 10, P::Ok, 0, 8, 3, 8560 * S + 2, 8, "fake ok"),
            ("tick", "", 8560 * S + 2, 10, P::Ok, 0, 8, 3, 8560 * S + 2, 8, "fake ok"),
        ],
        &[
            ("start", "", 0, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("tick", "", 30 * S, 1, P::Ok, 0, 0, 0, NONE, 0, ""),
            ("done", "ok", 150 * S, 1, P::Ok, 0, 1, 1, 150 * S, 1, "fake ok"),
            ("open", "", 1950 * S, 2, P::Ok, 0, 1, 1, 150 * S, 1, "fake ok"),
            ("tick", "", 2250 * S, 2, P::Ok, 0, 1, 1, 150 * S, 1, "fake ok"),
            ("tick", "", 2250 * S, 2, P::Ok, 0, 1, 1, 150 * S, 1, "fake ok"),
            ("run", "ok unless stopped", 2250 * S, 2, P::Ok, 0, 1, 1, 150 * S, 1, "fake ok"),
            ("done", "", -1350 * S, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", -1050 * S, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("open", "", -990 * S + 1, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", -930 * S + 2, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", -630 * S + 2, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", -570 * S + 2, 2, P::Ok, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("stop", "", -270 * S + 2, 2, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("start", "", -4 * M + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", -4 * M + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("open", "", M + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", 119 * S + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("tick", "", 119 * S + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("run", "limited", 1919 * S + 2, 3, P::Off, 0, 2, 2, -1350 * S, 2, "fake ok"),
            ("done", "", 1929 * S + 2, 3, P::RateLimited, 1, 3, 2, -1350 * S, 3, "fake limited"),
            ("stop", "", 1988 * S + 2, 3, P::Off, 0, 3, 2, -1350 * S, 3, "fake limited"),
            ("tick", "", 3788 * S + 2, 3, P::Off, 0, 3, 2, -1350 * S, 3, "fake limited"),
            ("stop", "", 3848 * S + 3, 3, P::Off, 0, 3, 2, -1350 * S, 3, "fake limited"),
            ("start", "", 3849 * S + 3, 4, P::Off, 0, 3, 2, -1350 * S, 3, "fake limited"),
            ("run", "gave up", 3969 * S + 3, 4, P::Off, 0, 3, 2, -1350 * S, 3, "fake limited"),
            ("done", "", 4089 * S + 3, 4, P::BadAnswer, 1, 4, 2, -1350 * S, 4, "Fake poll was cancelled"),
            ("tick", "", 5889 * S + 3, 5, P::BadAnswer, 1, 4, 2, -1350 * S, 4, "Fake poll was cancelled"),
            ("open", "", 6189 * S + 3, 5, P::BadAnswer, 1, 4, 2, -1350 * S, 4, "Fake poll was cancelled"),
            ("run", "refused", 6248 * S + 3, 5, P::BadAnswer, 1, 4, 2, -1350 * S, 4, "Fake poll was cancelled"),
            ("done", "", 6548 * S + 3, 5, P::Refused, 2, 5, 2, -1350 * S, 5, "fake refused"),
            ("tick", "", 6848 * S + 3, 6, P::Refused, 2, 5, 2, -1350 * S, 5, "fake refused"),
            ("stop", "", 6849 * S + 3, 6, P::Off, 0, 5, 2, -1350 * S, 5, "fake refused"),
            ("run", "gave up", 6879 * S + 3, 6, P::Off, 0, 5, 2, -1350 * S, 5, "fake refused"),
            ("done", "", 6999 * S + 3, 6, P::Off, 0, 5, 2, -1350 * S, 5, "fake refused"),
            ("start", "", 7059 * S + 4, 7, P::Off, 0, 5, 2, -1350 * S, 5, "fake refused"),
            ("done", "gave up", 7060 * S + 4, 7, P::BadAnswer, 1, 6, 2, -1350 * S, 6, "Fake poll was cancelled"),
            ("tick", "", 7090 * S + 4, 7, P::BadAnswer, 1, 6, 2, -1350 * S, 6, "Fake poll was cancelled"),
            ("tick", "", 7100 * S + 4, 7, P::BadAnswer, 1, 6, 2, -1350 * S, 6, "Fake poll was cancelled"),
            ("tick", "", 7100 * S + 4, 7, P::BadAnswer, 1, 6, 2, -1350 * S, 6, "Fake poll was cancelled"),
        ],
    ];
    assert_eq!(walks.len(), 12);
    for (number, walk) in walks.iter().enumerate() {
        let mut h = Harness::new();
        let mut handed_back = 0;
        let mut finished = None;
        for (
            step,
            &(call, answer, at, runs, state, misses, changed, kept, kept_at, traces, last),
        ) in walk.iter().enumerate()
        {
            let at = at_ticks(at);
            match call {
                "start" => h.start(at),
                "tick" => h.tick(at),
                "open" => h.flyout_opened(at),
                "stop" => h.poller.stop(),
                "run" => {
                    h.provider.answer_with(answer_named(answer, &h.kept));
                    finished = Some(h.runs[handed_back].take().unwrap().run());
                }
                "done" => {
                    if finished.is_none() {
                        h.provider.answer_with(answer_named(answer, &h.kept));
                        finished = Some(h.runs[handed_back].take().unwrap().run());
                    }
                    let done = h.poller.done(finished.take().unwrap(), at);
                    h.took_answer(done);
                    handed_back += 1;
                }
                other => panic!("a row names a call this test does not know: {other}"),
            }
            let got = (
                h.runs.len(),
                h.poller.state(),
                h.poller.misses(),
                h.changed,
                h.kept().len(),
                h.kept().last().copied(),
                h.traced.len(),
                h.traced.last().cloned().unwrap_or_default(),
            );
            let expected = (
                runs,
                state,
                misses,
                changed,
                kept,
                maybe_at(kept_at),
                traces,
                last.to_string(),
            );
            assert_eq!(got, expected, "walk {number}, step {step}: {call}");
        }
    }
}

#[test]
fn a_changed_sign_in_asks_at_once_but_respects_a_wait() {
    let mut h = Harness::new();
    let run = h.poller.sign_in_changed(now());
    assert!(run.is_none(), "off: no poll");

    h.start(now());
    h.finish(
        0,
        |_, _, _| {
            Ok(with_schedule(
                LiveSchedule::start(now()).after_no_credentials(now()),
            ))
        },
        now(),
    );
    // Not due, and fresh enough for an open flyout's ask, but the sign-in is new.
    h.tick(now() + seconds(5));
    assert_eq!(h.runs.len(), 1);
    let run = h.poller.sign_in_changed(now() + seconds(5));
    h.took(run);
    assert_eq!(h.runs.len(), 2);
    // One at a time.
    let run = h.poller.sign_in_changed(now() + seconds(6));
    h.took(run);
    assert_eq!(h.runs.len(), 2);

    h.finish(
        1,
        |_, _, _| {
            Ok(with_schedule(
                LiveSchedule::start(now()).after_failure(now()),
            ))
        },
        now() + seconds(7),
    );
    let run = h.poller.sign_in_changed(now() + seconds(8));
    h.took(run);
    assert_eq!(h.runs.len(), 2, "a wait is kept");
}
