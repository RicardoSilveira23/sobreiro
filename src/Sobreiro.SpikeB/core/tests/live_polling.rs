// Ports of LiveScheduleTests, LiveStateTests, UsageApiJsonTests, ProviderPollerTests, ClaudeProviderTests and the
// parts of LiveUsageClientTests that need no HTTP server (what a status, a Retry-After and a body mean).
mod common;
use common::*;
use sobreiro_core::limits::{Limit, LimitKind};
use sobreiro_core::live::*;
use sobreiro_core::polling::*;
use sobreiro_core::usage::*;
use sobreiro_core::{Span, Time};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn t0() -> Time {
    t("2026-11-09T12:00:00Z")
}

fn ok() -> LiveResult {
    LiveResult::of(LiveOutcome::Ok)
}
fn limited(after: Option<Span>) -> LiveResult {
    LiveResult { outcome: LiveOutcome::RateLimited, limits: None, retry_after: after }
}
fn failed(o: LiveOutcome) -> LiveResult {
    LiveResult::of(o)
}
fn fetch(s: LiveSchedule, r: LiveResult, at: Time) -> LiveSchedule {
    s.after_fetch(&r, at).unwrap()
}
fn start() -> LiveSchedule {
    LiveSchedule::start(t0())
}

// ---- LiveScheduleTests

#[test]
fn start_is_due_now() {
    let s = start();
    assert!(s.is_due(t0()) && s.fetch_on_open(t0()));
}

#[test]
fn after_ok_the_next_is_five_minutes_on_and_opening_fetches_after_one_minute() {
    let s = fetch(start(), ok(), t0());
    assert_eq!(s.next_due, t0() + mins(5));
    assert_eq!(s.last_success, Some(t0()));
    assert!(!s.is_due(t0() + Span::seconds(294)));
    assert!(s.is_due(t0() + mins(5)));
    assert!(!s.fetch_on_open(t0()));
    assert!(!s.fetch_on_open(t0() + mins(1)));
    assert!(s.fetch_on_open(t0() + mins(1) + Span::seconds(1)));
}

#[test]
fn after_a_failure_the_next_is_two_minutes_on() {
    for o in [LiveOutcome::Failed, LiveOutcome::Unreachable] {
        let s = fetch(start(), failed(o), t0());
        assert_eq!(s.next_due, t0() + mins(2));
        assert!(s.last_success.is_none());
    }
}

#[test]
fn a_failure_keeps_the_last_success() {
    let s = fetch(fetch(start(), ok(), t0()), failed(LiveOutcome::Failed), t0() + mins(5));
    assert_eq!(s.last_success, Some(t0()));
    assert!(!s.fetch_on_open(t0() + mins(5)));
    assert!(s.fetch_on_open(t0() + mins(7)));
}

// A failing request is not resent on every flyout open: opening waits out the 2-minute retry like a 429 wait.
#[test]
fn after_a_failure_opening_waits_for_the_retry() {
    for o in [LiveOutcome::Failed, LiveOutcome::Unreachable] {
        let s = fetch(start(), failed(o), t0());
        let just_before = t0() + mins(2) - Span::seconds(1);
        assert!(!s.fetch_on_open(t0()));
        assert!(!s.fetch_on_open(just_before));
        assert!(!s.is_due(just_before));
        assert!(s.is_due(t0() + mins(2)));
        assert!(s.fetch_on_open(t0() + mins(2)));
    }
}

#[test]
fn no_credentials_leave_opening_free_to_check_again() {
    let s = fetch(start(), failed(LiveOutcome::Failed), t0()).after_no_credentials(t0() + Span::seconds(30));
    assert!(s.fetch_on_open(t0() + Span::seconds(31)));
}

#[test]
fn a_refusal_remembers_the_expiry_and_reads_the_file_again_in_a_minute() {
    let expiry = t0() + hours(2);
    let s = start().after_refused(Some(expiry), t0());
    assert_eq!(s.refused, Some(RefusedSignIn { expires_at: Some(expiry) }));
    assert_eq!(s.next_due, t0() + mins(1));
    assert!(s.wait_until.is_none());
    assert!(s.fetch_on_open(t0())); // opening only reads the file: the poll sends nothing while it refuses
    assert!(s.refuses(Some(expiry)));
    assert!(!s.refuses(Some(expiry + hours(8))));
    assert!(!s.refuses(None));
}

#[test]
fn a_refusal_without_an_expiry_refuses_only_a_file_without_one() {
    let s = start().after_refused(None, t0());
    assert!(s.refuses(None));
    assert!(!s.refuses(Some(t0() + hours(2))));
    assert!(!start().refuses(None));
}

#[test]
fn a_refusal_lasts_through_rereads_and_a_missing_file() {
    let expiry = t0() + hours(2);
    let s = start().after_refused(Some(expiry), t0()).after_no_credentials(t0() + mins(1)).after_no_credentials(t0() + mins(2));
    assert!(s.refuses(Some(expiry)));
    assert_eq!(s.next_due, t0() + mins(3));
}

#[test]
fn a_fetch_with_another_sign_in_forgets_the_refusal() {
    for o in [LiveOutcome::Ok, LiveOutcome::RateLimited, LiveOutcome::Failed, LiveOutcome::Unreachable] {
        let s = fetch(start().after_refused(Some(t0() + hours(2)), t0()), LiveResult::of(o), t0() + mins(1));
        assert!(s.refused.is_none(), "{o:?}");
    }
}

#[test]
fn a_refusal_needs_the_expiry_so_after_fetch_turns_it_away() {
    assert!(start().after_fetch(&LiveResult::of(LiveOutcome::Unauthorized), t0()).is_err());
}

#[test]
fn rate_limits_back_off_five_ten_twenty_thirty_thirty() {
    let mut s = start();
    let mut now = t0();
    for minutes in [5, 10, 20, 30, 30] {
        s = fetch(s, limited(None), now);
        assert_eq!(s.next_due, now + mins(minutes));
        assert_eq!(s.wait_until, Some(now + mins(minutes)));
        assert!(!s.fetch_on_open(now + mins(minutes - 1)));
        now = now + mins(minutes);
    }
}

#[test]
fn a_long_retry_after_wins_over_the_step() {
    let s = fetch(start(), limited(Some(mins(45))), t0());
    assert_eq!(s.next_due, t0() + mins(45));
    assert_eq!(s.wait_until, Some(t0() + mins(45)));
}

#[test]
fn a_short_retry_after_loses_to_the_step() {
    assert_eq!(fetch(start(), limited(Some(Span::seconds(30))), t0()).next_due, t0() + mins(5));
}

#[test]
fn an_ok_resets_the_step_and_the_wait() {
    let s = fetch(fetch(fetch(start(), limited(None), t0()), limited(None), t0() + mins(5)), ok(), t0() + mins(15));
    assert_eq!(s.rate_limit_step, 0);
    assert!(s.wait_until.is_none());
    assert_eq!(s.next_due, t0() + mins(20));
    let s = fetch(s, limited(None), t0() + mins(20));
    assert_eq!(s.next_due, t0() + mins(25));
}

#[test]
fn a_non_rate_limit_failure_does_not_reset_the_step() {
    let s = fetch(fetch(fetch(start(), limited(None), t0()), failed(LiveOutcome::Failed), t0() + mins(5)), limited(None), t0() + mins(7));
    assert_eq!(s.next_due, t0() + mins(17));
}

#[test]
fn no_credentials_check_again_in_a_minute() {
    let s = start().after_no_credentials(t0());
    assert_eq!(s.next_due, t0() + mins(1));
    assert!(!s.is_due(t0() + Span::seconds(59)));
    assert!(s.is_due(t0() + mins(1)));
}

#[test]
fn after_no_reading_waits_five_minutes_and_blocks_open() {
    let s = start().after_no_reading(t0());
    assert_eq!(s.next_due, t0() + mins(5));
    assert!(!s.is_due(t0() + Span::seconds(294)));
    assert!(s.is_due(t0() + mins(5)));
    assert!(!s.fetch_on_open(t0() + mins(2)));
    assert!(s.fetch_on_open(t0() + mins(5)));
}

#[test]
fn start_has_no_misses() {
    assert_eq!(start().misses, 0);
}

#[test]
fn a_success_after_misses_gives_zero() {
    let s = fetch(start().after_no_reading(t0()).after_no_reading(t0() + mins(5)), ok(), t0() + mins(10));
    assert_eq!(s.misses, 0);
}

#[test]
fn each_failed_outcome_adds_a_miss() {
    for o in [LiveOutcome::RateLimited, LiveOutcome::Failed, LiveOutcome::Unreachable] {
        assert_eq!(fetch(start(), LiveResult::of(o), t0()).misses, 1);
    }
}

#[test]
fn no_credentials_refused_and_no_reading_each_add_a_miss() {
    assert_eq!(start().after_no_credentials(t0()).misses, 1);
    assert_eq!(start().after_refused(Some(t0() + hours(1)), t0()).misses, 1);
    assert_eq!(start().after_no_reading(t0()).misses, 1);
}

#[test]
fn three_failures_in_a_row_give_three() {
    let f = failed(LiveOutcome::Failed);
    let s = fetch(fetch(fetch(start(), f.clone(), t0()), f.clone(), t0() + mins(2)), f, t0() + mins(4));
    assert_eq!(s.misses, 3);
}

// ---- LiveStateTests

#[test]
fn each_problem_has_a_sentence() {
    for (state, text) in [
        (ProviderState::Expired, "Claude Code's sign-in has expired. Use Claude Code in a terminal and it renews it."),
        (ProviderState::NoSignIn, "Claude Code isn't signed in on this PC. Run claude in a terminal and sign in."),
        (ProviderState::Refused, "Anthropic didn't accept Claude Code's sign-in. Use Claude Code in a terminal so it gets a new one."),
        (ProviderState::RateLimited, "Anthropic asked Sobreiro to wait. Nothing to do: it asks again by itself."),
        (ProviderState::Offline, "Sobreiro can't reach Anthropic. Check the connection; it tries again every 2 minutes."),
        (ProviderState::BadAnswer, "Anthropic's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro."),
    ] {
        assert_eq!(LiveStateText::footer(state), Some(text));
    }
}
#[test]
fn off_and_ok_have_none() {
    assert_eq!(LiveStateText::footer(ProviderState::Off), None);
    assert_eq!(LiveStateText::footer(ProviderState::Ok), None);
}
#[test]
fn the_about_line_is_fixed() {
    assert_eq!(
        LiveUsageMessage::ABOUT_LINE,
        "It asks Anthropic for your usage every 5 minutes with Claude Code's sign-in, and makes no other network calls."
    );
}

// ---- UsageApiJsonTests

const LIST_SHAPE: &str = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"},\
{\"group\":\"weekly\",\"percent\":74,\"resets_at\":\"2026-11-13T00:00:00Z\"},\
{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}]}";

#[test]
fn the_list_shape_gives_session_and_weekly_and_ignores_scoped_entries() {
    let l = UsageApiJson::parse(LIST_SHAPE).unwrap();
    let five = l.five_hour.unwrap();
    assert_eq!(five.used_percentage, 27.0);
    assert_eq!(five.resets_at, Some(t("2026-11-09T14:00:00Z")));
    assert!(five.observed_at.is_none());
    let seven = l.seven_day.unwrap();
    assert_eq!((seven.used_percentage, seven.resets_at), (74.0, Some(t("2026-11-13T00:00:00Z"))));
    assert!(l.spend.is_none());
}

#[test]
fn the_keyed_shape_works_and_ignores_per_model_and_app_entries() {
    let json = "{\"five_hour\":{\"utilization\":27.5,\"resets_at\":\"2026-11-09T14:00:00Z\"},\
\"seven_day\":{\"utilization\":74,\"resets_at\":\"2026-11-13T00:00:00+00:00\"},\
\"seven_day_opus\":{\"utilization\":90,\"resets_at\":null},\"seven_day_oauth_apps\":{\"utilization\":5}}";
    let l = UsageApiJson::parse(json).unwrap();
    assert_eq!(l.five_hour.unwrap().used_percentage, 27.5);
    let seven = l.seven_day.unwrap();
    assert_eq!(seven.used_percentage, 74.0);
    assert!(seven.resets_at.is_some());
}

#[test]
fn a_list_with_no_usable_entry_falls_back_to_the_keys() {
    let json = "{\"limits\":[{\"group\":\"weekly\",\"percent\":40,\"scope\":{\"model\":\"Fable\"}}],\"five_hour\":{\"utilization\":10}}";
    let l = UsageApiJson::parse(json).unwrap();
    assert_eq!(l.five_hour.unwrap().used_percentage, 10.0);
    assert!(l.seven_day.is_none());
}

#[test]
fn a_missing_reset_is_a_null_reset() {
    let l = UsageApiJson::parse("{\"limits\":[{\"group\":\"session\",\"percent\":3}]}").unwrap();
    let five = l.five_hour.unwrap();
    assert_eq!(five.used_percentage, 3.0);
    assert!(five.resets_at.is_none());
}

#[test]
fn a_string_percent_drops_that_window() {
    let l = UsageApiJson::parse("{\"limits\":[{\"group\":\"session\",\"percent\":\"27\"},{\"group\":\"weekly\",\"percent\":50}]}").unwrap();
    assert!(l.five_hour.is_none());
    assert_eq!(l.seven_day.unwrap().used_percentage, 50.0);
}

#[test]
fn a_bad_reset_text_is_a_null_reset() {
    let l = UsageApiJson::parse("{\"limits\":[{\"group\":\"session\",\"percent\":3,\"resets_at\":\"soon\"}]}").unwrap();
    assert!(l.five_hour.unwrap().resets_at.is_none());
}

#[test]
fn nothing_usable_is_null() {
    for json in ["{}", "[]", "", "{ broken", "null", "{\"limits\":\"x\",\"five_hour\":5}", "{\"limits\":[1,null,\"a\",{\"group\":5}]}"] {
        assert!(UsageApiJson::parse(json).is_none(), "{json}");
    }
}

#[test]
fn a_window_missing_from_the_list_comes_from_the_keys() {
    let l = UsageApiJson::parse(
        "{\"limits\":[{\"group\":\"weekly\",\"percent\":60}],\"five_hour\":{\"utilization\":12},\"seven_day\":{\"utilization\":99}}",
    )
    .unwrap();
    assert_eq!(l.five_hour.unwrap().used_percentage, 12.0);
    assert_eq!(l.seven_day.unwrap().used_percentage, 60.0);
}

// ---- LiveUsageClientTests (the meaning of an answer; no request is made)

const BODY: &str = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

#[test]
fn a_200_gives_limits() {
    let r = LiveUsageClient::interpret(200, None, Some(BODY), t0());
    assert_eq!(r.outcome, LiveOutcome::Ok);
    assert_eq!(r.limits.unwrap().five_hour.unwrap().used_percentage, 27.0);
}
#[test]
fn a_401_or_403_is_unauthorized() {
    for code in [401, 403] {
        assert_eq!(LiveUsageClient::interpret(code, None, None, t0()), LiveResult::of(LiveOutcome::Unauthorized));
    }
}
#[test]
fn a_429_with_seconds_gives_that_retry_after() {
    let r = LiveUsageClient::interpret(429, Some("600"), None, t0());
    assert_eq!((r.outcome, r.retry_after), (LiveOutcome::RateLimited, Some(mins(10))));
}
#[test]
fn a_429_with_an_http_date_gives_the_gap_from_now() {
    let r = LiveUsageClient::interpret(429, Some("Mon, 09 Nov 2026 12:10:00 GMT"), None, t0());
    assert_eq!(r.retry_after, Some(mins(10)));
}
#[test]
fn a_429_without_retry_after_has_none() {
    assert_eq!(LiveUsageClient::interpret(429, None, None, t0()).retry_after, None);
}
#[test]
fn a_500_is_failed() {
    assert_eq!(LiveUsageClient::interpret(500, None, None, t0()).outcome, LiveOutcome::Failed);
}
#[test]
fn a_redirect_is_failed_and_not_followed() {
    assert_eq!(LiveUsageClient::interpret(302, None, None, t0()).outcome, LiveOutcome::Failed);
}
#[test]
fn a_200_with_an_unparseable_body_is_failed() {
    for body in ["not json", "{}", ""] {
        assert_eq!(LiveUsageClient::interpret(200, None, Some(body), t0()).outcome, LiveOutcome::Failed, "{body}");
    }
}
#[test]
fn a_far_future_retry_after_is_capped_at_six_hours() {
    assert_eq!(LiveUsageClient::retry_after(Some("86400"), t0()), Some(hours(6)));
    assert_eq!(LiveUsageClient::retry_after(Some("-30"), t0()), Some(Span::zero()));
    assert_eq!(LiveUsageClient::retry_after(Some("soon"), t0()), None);
}
#[test]
fn the_timeout_is_ten_seconds() {
    assert_eq!(LiveUsageClient::timeout(), Span::seconds(10));
}

// ---- ProviderPollerTests

fn poll_ok(at: Time) -> ProviderPoll {
    ProviderPoll {
        schedule: fetch(LiveSchedule::start(at), ok(), at),
        state: ProviderState::Ok,
        log: "fake Ok".into(),
        kept: None,
    }
}
fn marker() -> Option<Kept> {
    Some(Kept::Claude(RateLimits { five_hour: Some(win(1.0, None)), seven_day: None, spend: None }))
}

struct Fake {
    answer: Arc<Mutex<Box<dyn Fn() -> ProviderPoll + Send>>>,
    tokens: Arc<Mutex<Vec<CancelToken>>>,
    seen: Arc<Mutex<Vec<(LiveSchedule, Time)>>>,
    kept: Rc<RefCell<Vec<Time>>>,
    order: Rc<RefCell<Vec<String>>>,
}

impl Provider for Fake {
    fn name(&self) -> &str {
        "Fake"
    }
    fn poll_job(&self, schedule: LiveSchedule, now: Time) -> PollJob {
        let (answer, tokens, seen) = (self.answer.clone(), self.tokens.clone(), self.seen.clone());
        Box::new(move |cancel| {
            tokens.lock().unwrap().push(cancel.clone());
            seen.lock().unwrap().push((schedule, now));
            (answer.lock().unwrap())()
        })
    }
    fn limits(&mut self, _now: Time) -> Vec<Limit> {
        vec![]
    }
    fn keep(&mut self, _kept: Kept, now: Time) {
        self.kept.borrow_mut().push(now);
        self.order.borrow_mut().push("keep".into());
    }
}

struct Rig {
    poller: ProviderPoller<Fake>,
    runs: Rc<RefCell<Vec<Option<PollRun>>>>,
    traced: Rc<RefCell<Vec<String>>>,
    order: Rc<RefCell<Vec<String>>>,
    changed: Rc<RefCell<i32>>,
    answer: Arc<Mutex<Box<dyn Fn() -> ProviderPoll + Send>>>,
    tokens: Arc<Mutex<Vec<CancelToken>>>,
    seen: Arc<Mutex<Vec<(LiveSchedule, Time)>>>,
    kept: Rc<RefCell<Vec<Time>>>,
}

fn now13() -> Time {
    t("2026-11-13T12:00:00Z")
}

impl Rig {
    fn new() -> Rig {
        let answer: Arc<Mutex<Box<dyn Fn() -> ProviderPoll + Send>>> = Arc::new(Mutex::new(Box::new(|| poll_ok(now13()))));
        let tokens = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let kept = Rc::new(RefCell::new(Vec::new()));
        let order = Rc::new(RefCell::new(Vec::new()));
        let runs: Rc<RefCell<Vec<Option<PollRun>>>> = Rc::new(RefCell::new(Vec::new()));
        let traced = Rc::new(RefCell::new(Vec::new()));
        let changed = Rc::new(RefCell::new(0));
        let fake = Fake { answer: answer.clone(), tokens: tokens.clone(), seen: seen.clone(), kept: kept.clone(), order: order.clone() };
        let (r2, tr2, ch2, or2) = (runs.clone(), traced.clone(), changed.clone(), order.clone());
        let poller = ProviderPoller::new(
            fake,
            Box::new(move |run| r2.borrow_mut().push(Some(run))),
            Box::new(move || {
                *ch2.borrow_mut() += 1;
                or2.borrow_mut().push("changed".into());
            }),
            Some(Box::new(move |line| tr2.borrow_mut().push(line.to_string()))),
        );
        Rig { poller, runs, traced, order, changed, answer, tokens, seen, kept }
    }
    fn runs(&self) -> usize {
        self.runs.borrow().len()
    }
    /// Runs the poll `i` as the shell's worker would, with this answer, and hands it back at `now`.
    fn finish_with(&mut self, i: usize, poll: ProviderPoll, now: Time) {
        *self.answer.lock().unwrap() = Box::new(move || poll.clone());
        self.finish(i, now);
    }
    fn finish(&mut self, i: usize, now: Time) {
        let run = self.runs.borrow_mut()[i].take().expect("run not yet handed back");
        let done = run.run();
        self.poller.done(done, now);
    }
    fn run_only(&mut self, i: usize) -> PollDone {
        self.runs.borrow_mut()[i].take().unwrap().run()
    }
}

fn with_schedule(schedule: LiveSchedule) -> ProviderPoll {
    ProviderPoll { schedule, state: ProviderState::Ok, log: "fake Ok".into(), kept: None }
}

#[test]
fn poller_before_start_it_is_ok_with_no_misses() {
    let mut rig = Rig::new();
    assert_eq!((rig.poller.state(), rig.poller.misses()), (ProviderState::Ok, 0));
    rig.poller.tick(now13());
    rig.poller.flyout_opened(now13());
    assert_eq!(rig.runs(), 0);
}

#[test]
fn poller_start_polls_at_once_and_only_once() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    assert_eq!(rig.runs(), 1);
    rig.poller.start(now13());
    assert_eq!(rig.runs(), 1);
}

#[test]
fn the_poll_gets_the_schedule_and_time_it_started_with() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    let _ = rig.run_only(0);
    let seen = rig.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].1, now13());
    assert_eq!(seen[0].0.next_due, now13());
}

#[test]
fn one_poll_at_a_time() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.poller.tick(now13() + mins(10));
    rig.poller.flyout_opened(now13() + mins(10));
    assert_eq!(rig.runs(), 1);
}

#[test]
fn an_answer_sets_the_state_keeps_the_reading_and_redraws() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    let poll = ProviderPoll {
        schedule: LiveSchedule::start(now13()).after_no_credentials(now13()),
        state: ProviderState::RateLimited,
        log: "fake Ok".into(),
        kept: marker(),
    };
    rig.finish_with(0, poll, now13() + Span::seconds(3));
    assert_eq!(rig.poller.state(), ProviderState::RateLimited);
    assert_eq!(rig.poller.misses(), 1);
    assert_eq!(*rig.kept.borrow(), vec![now13() + Span::seconds(3)]);
    assert_eq!(*rig.changed.borrow(), 1);
    assert_eq!(*rig.traced.borrow(), vec!["fake Ok".to_string()]);
    assert_eq!(*rig.order.borrow(), vec!["keep".to_string(), "changed".to_string()]);
}

#[test]
fn misses_follow_the_schedule() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.finish_with(0, with_schedule(LiveSchedule::start(now13()).after_no_credentials(now13())), now13());
    assert_eq!(rig.poller.misses(), 1);
    rig.poller.tick(now13() + mins(1));
    assert_eq!(rig.runs(), 2);
    rig.finish_with(1, poll_ok(now13() + mins(1)), now13() + mins(1));
    assert_eq!(rig.poller.misses(), 0);
}

#[test]
fn tick_waits_for_the_schedule() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.finish_with(0, poll_ok(now13()), now13());
    rig.poller.tick(now13() + mins(4) + Span::seconds(59));
    assert_eq!(rig.runs(), 1);
    rig.poller.tick(now13() + mins(5));
    assert_eq!(rig.runs(), 2);
}

#[test]
fn opening_the_flyout_asks_early() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.finish_with(0, with_schedule(LiveSchedule::start(now13()).after_no_credentials(now13())), now13());
    rig.poller.tick(now13() + Span::seconds(10));
    assert_eq!(rig.runs(), 1);
    rig.poller.flyout_opened(now13() + Span::seconds(10));
    assert_eq!(rig.runs(), 2);
}

#[test]
fn opening_the_flyout_respects_a_wait() {
    let mut rig = Rig::new();
    let failed_schedule = fetch(LiveSchedule::start(now13()), failed(LiveOutcome::Failed), now13());
    rig.poller.start(now13());
    rig.finish_with(0, with_schedule(failed_schedule), now13());
    rig.poller.flyout_opened(now13() + Span::seconds(10));
    assert_eq!(rig.runs(), 1);
}

#[test]
fn a_poll_that_threw_counts_as_a_bad_answer() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    *rig.answer.lock().unwrap() = Box::new(|| panic!("secret"));
    rig.finish(0, now13() + Span::seconds(1));
    assert_eq!(rig.poller.state(), ProviderState::BadAnswer);
    assert_eq!(rig.poller.misses(), 1);
    assert_eq!(*rig.changed.borrow(), 1);
    assert_eq!(*rig.traced.borrow(), vec!["Fake poll threw Panic".to_string()]); // never the panic's text
    rig.poller.tick(now13() + mins(1) + Span::seconds(59));
    assert_eq!(rig.runs(), 1);
    rig.poller.tick(now13() + mins(2));
    assert_eq!(rig.runs(), 2);
}

#[test]
fn stop_cancels_the_poll_and_turns_it_off() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    let _ = rig.run_only(0);
    rig.poller.stop();
    assert!(rig.tokens.lock().unwrap()[0].is_cancelled());
    assert_eq!((rig.poller.state(), rig.poller.misses()), (ProviderState::Off, 0));
}

#[test]
fn a_poll_stopped_while_it_ran_is_dropped() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.poller.stop();
    let poll = ProviderPoll { kept: marker(), ..poll_ok(now13()) };
    rig.finish_with(0, poll, now13() + Span::seconds(1));
    assert!(rig.kept.borrow().is_empty());
    assert_eq!(*rig.changed.borrow(), 0);
    assert_eq!(rig.poller.state(), ProviderState::Off);
    assert_eq!(rig.runs(), 1);
}

#[test]
fn stop_then_start_while_a_poll_runs_waits_for_it() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.poller.stop();
    rig.poller.start(now13() + Span::seconds(1));
    assert_eq!(rig.runs(), 1);
    rig.finish_with(0, ProviderPoll { kept: marker(), ..poll_ok(now13()) }, now13() + Span::seconds(2));
    assert!(rig.kept.borrow().is_empty());
    assert_eq!(rig.runs(), 2);
    rig.finish_with(1, ProviderPoll { state: ProviderState::Expired, ..poll_ok(now13() + Span::seconds(2)) }, now13() + Span::seconds(3));
    assert_eq!(rig.poller.state(), ProviderState::Expired);
}

// NOT PORTED: A_run_handed_back_twice_does_nothing_the_second_time. A run's result (PollDone) is consumed by
// `done`, so a second hand-back cannot be written; the id check inside `done` covers a stale result from an
// earlier poll, which the next test does.
#[test]
fn a_stale_result_from_an_earlier_poll_does_nothing() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    rig.poller.stop();
    let stale = rig.run_only(0); // poll 0, finished after a stop
    rig.poller.start(now13() + Span::seconds(1)); // poll 0 is still "running": nothing new yet
    assert_eq!(rig.runs(), 1);
    rig.poller.done(stale, now13() + Span::seconds(2)); // dropped, and the next poll starts
    assert_eq!(rig.runs(), 2);
    *rig.answer.lock().unwrap() = Box::new(|| ProviderPoll { kept: marker(), ..poll_ok(now13()) });
    let second = rig.run_only(1);
    rig.poller.done(second, now13() + Span::seconds(3));
    assert_eq!(*rig.changed.borrow(), 1);
    assert_eq!(rig.kept.borrow().len(), 1);
}

#[test]
fn after_dispose_nothing_happens() {
    let mut rig = Rig::new();
    rig.poller.start(now13());
    let done = rig.run_only(0);
    rig.poller.dispose();
    assert!(rig.tokens.lock().unwrap()[0].is_cancelled());
    rig.poller.done(done, now13() + Span::seconds(1));
    assert!(rig.kept.borrow().is_empty());
    assert_eq!(*rig.changed.borrow(), 0);
    rig.poller.start(now13() + mins(10));
    rig.poller.tick(now13() + mins(10));
    rig.poller.flyout_opened(now13() + mins(10));
    assert_eq!(rig.runs(), 1);
    rig.poller.dispose();
}

#[test]
fn the_run_and_its_result_are_send() {
    fn is_send<T: Send + 'static>() {}
    is_send::<PollRun>();
    is_send::<PollDone>();
    is_send::<LiveUsageClient>();
}

// ---- ClaudeProviderTests (the poll is passed in; the real client is not used)

fn reading() -> RateLimits {
    RateLimits { five_hour: Some(win(27.0, Some(t0() + hours(2)))), seven_day: None, spend: None }
}

struct Claude {
    _dir: Scratch,
    usage: String,
    trace: Rc<RefCell<Vec<String>>>,
    provider: ClaudeProvider,
}

fn claude(result: LivePollResult) -> Claude {
    let dir = Scratch::new();
    let usage = dir.path("usage.json");
    let trace = Rc::new(RefCell::new(Vec::new()));
    let tr = trace.clone();
    let provider = ClaudeProvider::with_poll(Arc::new(move |_, _| result.clone()), usage.clone(), Some(Box::new(move |l| tr.borrow_mut().push(l.to_string()))));
    Claude { _dir: dir, usage, trace, provider }
}

fn ok_result() -> LivePollResult {
    LivePollResult { schedule: start(), state: ProviderState::Ok, limits: Some(reading()), outcome: "Ok".into() }
}

fn poll_of(p: &ClaudeProvider) -> ProviderPoll {
    p.poll_job(start(), t0())(&CancelToken::default())
}

#[test]
fn claude_provider_carries_claudes_name() {
    assert_eq!(claude(ok_result()).provider.name(), "Claude Code");
}

#[test]
fn claude_a_poll_passes_on_the_schedule_the_state_and_the_outcome() {
    let c = claude(ok_result());
    let poll = poll_of(&c.provider);
    assert_eq!(poll.schedule, start());
    assert_eq!(poll.state, ProviderState::Ok);
    assert_eq!(poll.log, "Live usage Ok");
}

#[test]
fn claude_a_poll_without_a_reading_has_nothing_to_keep() {
    let c = claude(LivePollResult { schedule: start(), state: ProviderState::Expired, limits: None, outcome: "Expired".into() });
    let poll = poll_of(&c.provider);
    assert!(poll.kept.is_none());
    assert_eq!(poll.log, "Live usage Expired");
}

#[test]
fn the_reading_reaches_usage_json_only_through_keep() {
    let mut c = claude(ok_result());
    let poll = poll_of(&c.provider);
    assert!(!std::path::Path::new(&c.usage).exists());
    assert!(c.provider.limits(t0()).is_empty());
    c.provider.keep(poll.kept.unwrap(), t0());
    let limits = c.provider.limits(t0());
    assert_eq!(limits.len(), 1);
    assert_eq!((limits[0].kind, limits[0].percent), (LimitKind::Session, 27.0));
}

#[test]
fn keep_merges_over_what_the_file_holds() {
    let mut c = claude(ok_result());
    let weekly = RateLimits { five_hour: None, seven_day: Some(win_at(60.0, Some(t0() + days(3)), t0() - mins(5))), spend: None };
    UsageFile::write(&c.usage, &UsageSnapshot { limits: weekly, observed_at: t0() - mins(5) }).unwrap();
    let kept = poll_of(&c.provider).kept.unwrap();
    c.provider.keep(kept, t0());
    let limits = c.provider.limits(t0());
    assert!(limits.iter().any(|l| l.kind == LimitKind::Weekly && l.percent == 60.0));
    assert!(limits.iter().any(|l| l.kind == LimitKind::Session && l.percent == 27.0));
}

#[test]
fn a_reading_replaces_a_higher_stored_one_in_the_same_period() {
    let mut c = claude(ok_result());
    let higher = RateLimits { five_hour: Some(win_at(40.0, Some(t0() + hours(2)), t0() - mins(5))), seven_day: None, spend: None };
    UsageFile::write(&c.usage, &UsageSnapshot { limits: higher, observed_at: t0() - mins(5) }).unwrap();
    let kept = poll_of(&c.provider).kept.unwrap();
    c.provider.keep(kept, t0());
    let limits = c.provider.limits(t0());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].percent, 27.0);
}

#[test]
fn a_held_usage_json_skips_the_reading() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut c = claude(ok_result());
    std::fs::write(&c.usage, "{}").unwrap();
    let before = std::fs::read(&c.usage).unwrap();
    let kept = poll_of(&c.provider).kept.unwrap();
    {
        let _held = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&c.usage).unwrap();
        c.provider.keep(kept, t0());
    }
    assert!(c.trace.borrow().contains(&"usage.json was busy, so this Live usage reading was not saved".to_string()));
    assert_eq!(std::fs::read(&c.usage).unwrap(), before);
}

#[test]
fn limits_keep_the_last_snapshot_when_a_read_fails() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut c = claude(ok_result());
    let kept = poll_of(&c.provider).kept.unwrap();
    c.provider.keep(kept, t0());
    assert_eq!(c.provider.limits(t0()).len(), 1);
    let _held = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&c.usage).unwrap();
    let limits = c.provider.limits(t0());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Session);
}

#[test]
fn a_poll_cancelled_before_it_starts_asks_nothing() {
    let c = claude(ok_result());
    let cancel = CancelToken::default();
    cancel.cancel();
    let poll = c.provider.poll_job(start(), t0())(&cancel);
    assert!(poll.kept.is_none());
}

#[test]
fn the_clients_build_with_the_platform_tls_and_no_request_is_made() {
    // SPIKE: only that the agent is built (native TLS, no redirects); nothing is sent.
    let _ = LiveUsageClient::new("Sobreiro/0.0.0");
    let _ = sobreiro_core::copilot::CopilotUsageClient::new("Sobreiro/0.0.0");
}
