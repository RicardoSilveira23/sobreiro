//! Ported from `tests/Sobreiro.Tests/LiveScheduleTests.cs`. The tests after
//! `three_failures_in_a_row_give_three` are new.
//!
//! Every moment here is made up: the schedule is handed the time and never asks a clock.
use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::live::live_schedule::{LiveSchedule, RefusalNeedsItsExpiry, RefusedSignIn};
use sobreiro_core::live::{LiveOutcome, LiveResult};

use crate::{D, H, M, NO_EXPIRY, NONE, S, ScheduleRow, at_ticks, maybe_at, schedule_of, time};

fn t0() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
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

/// A result with nothing but its outcome.
fn result(outcome: LiveOutcome) -> LiveResult {
    LiveResult {
        outcome,
        limits: None,
        retry_after: None,
    }
}

fn ok() -> LiveResult {
    result(LiveOutcome::Ok)
}

fn failed() -> LiveResult {
    result(LiveOutcome::Failed)
}

/// A 429, with the wait it asked for when it asked for one.
fn limited(after: Option<TimeDelta>) -> LiveResult {
    LiveResult {
        outcome: LiveOutcome::RateLimited,
        limits: None,
        retry_after: after,
    }
}

/// The schedule after a fetch that was not refused.
fn after(schedule: &LiveSchedule, result: &LiveResult, now: DateTime<Utc>) -> LiveSchedule {
    schedule.after_fetch(result, now).unwrap()
}

#[test]
fn start_is_due_now() {
    let s = LiveSchedule::start(t0());
    assert!(s.is_due(t0()));
    assert!(s.fetch_on_open(t0()));
}

#[test]
fn after_ok_the_next_is_five_minutes_on_and_opening_fetches_after_one_minute() {
    let s = after(&LiveSchedule::start(t0()), &ok(), t0());
    assert_eq!(s.next_due, t0() + minutes(5));
    assert_eq!(s.last_success, Some(t0()));
    assert!(!s.is_due(t0() + seconds(294)));
    assert!(s.is_due(t0() + minutes(5)));
    assert!(!s.fetch_on_open(t0()));
    assert!(!s.fetch_on_open(t0() + minutes(1)));
    assert!(s.fetch_on_open(t0() + minutes(1) + seconds(1)));
}

#[test]
fn after_a_failure_the_next_is_two_minutes_on() {
    for outcome in [LiveOutcome::Failed, LiveOutcome::Unreachable] {
        let s = after(&LiveSchedule::start(t0()), &result(outcome), t0());
        assert_eq!(s.next_due, t0() + minutes(2), "{outcome:?}");
        assert_eq!(s.last_success, None, "{outcome:?}");
    }
}

#[test]
fn a_failure_keeps_the_last_success() {
    let s = after(&LiveSchedule::start(t0()), &ok(), t0());
    let s = after(&s, &failed(), t0() + minutes(5));
    assert_eq!(s.last_success, Some(t0()));
    assert!(!s.fetch_on_open(t0() + minutes(5)));
    assert!(s.fetch_on_open(t0() + minutes(7)));
}

// A failing request is not resent on every flyout open: opening waits out the 2-minute retry
// like a 429 wait.
#[test]
fn after_a_failure_opening_waits_for_the_retry() {
    for outcome in [LiveOutcome::Failed, LiveOutcome::Unreachable] {
        let s = after(&LiveSchedule::start(t0()), &result(outcome), t0());
        assert!(!s.fetch_on_open(t0()), "{outcome:?}");
        assert!(
            !s.fetch_on_open(t0() + minutes(2) - seconds(1)),
            "{outcome:?}"
        );
        assert!(!s.is_due(t0() + minutes(2) - seconds(1)), "{outcome:?}");
        assert!(s.is_due(t0() + minutes(2)), "{outcome:?}");
        assert!(s.fetch_on_open(t0() + minutes(2)), "{outcome:?}");
    }
}

#[test]
fn no_credentials_leave_opening_free_to_check_again() {
    let s =
        after(&LiveSchedule::start(t0()), &failed(), t0()).after_no_credentials(t0() + seconds(30));
    assert!(s.fetch_on_open(t0() + seconds(31)));
}

#[test]
fn a_refusal_remembers_the_expiry_and_reads_the_file_again_in_a_minute() {
    let expiry = t0() + hours(2);
    let s = LiveSchedule::start(t0()).after_refused(Some(expiry), t0());
    assert_eq!(
        s.refused,
        Some(RefusedSignIn {
            expires_at: Some(expiry)
        })
    );
    assert_eq!(s.next_due, t0() + minutes(1));
    assert_eq!(s.wait_until, None);
    // Opening only reads the file: the poll sends nothing while it refuses.
    assert!(s.fetch_on_open(t0()));
    assert!(s.refuses(Some(expiry)));
    assert!(!s.refuses(Some(expiry + hours(8))));
    assert!(!s.refuses(None));
}

#[test]
fn a_refusal_without_an_expiry_refuses_only_a_file_without_one() {
    let s = LiveSchedule::start(t0()).after_refused(None, t0());
    assert!(s.refuses(None));
    assert!(!s.refuses(Some(t0() + hours(2))));
    assert!(!LiveSchedule::start(t0()).refuses(None));
}

#[test]
fn a_refusal_lasts_through_rereads_and_a_missing_file() {
    let expiry = t0() + hours(2);
    let s = LiveSchedule::start(t0())
        .after_refused(Some(expiry), t0())
        .after_no_credentials(t0() + minutes(1))
        .after_no_credentials(t0() + minutes(2));
    assert!(s.refuses(Some(expiry)));
    assert_eq!(s.next_due, t0() + minutes(3));
}

#[test]
fn a_fetch_with_another_sign_in_forgets_the_refusal() {
    for outcome in [
        LiveOutcome::Ok,
        LiveOutcome::RateLimited,
        LiveOutcome::Failed,
        LiveOutcome::Unreachable,
    ] {
        let refused = LiveSchedule::start(t0()).after_refused(Some(t0() + hours(2)), t0());
        let s = after(&refused, &result(outcome), t0() + minutes(1));
        assert_eq!(s.refused, None, "{outcome:?}");
    }
}

#[test]
fn a_refusal_needs_the_expiry_so_after_fetch_turns_it_away() {
    assert_eq!(
        LiveSchedule::start(t0()).after_fetch(&result(LiveOutcome::Unauthorized), t0()),
        Err(RefusalNeedsItsExpiry)
    );
}

#[test]
fn rate_limits_back_off_five_ten_twenty_thirty_thirty() {
    let mut s = LiveSchedule::start(t0());
    let mut now = t0();
    for wait in [5, 10, 20, 30, 30] {
        s = after(&s, &limited(None), now);
        assert_eq!(s.next_due, now + minutes(wait));
        assert_eq!(s.wait_until, Some(now + minutes(wait)));
        assert!(!s.fetch_on_open(now + minutes(wait - 1)));
        now += minutes(wait);
    }
}

#[test]
fn a_long_retry_after_wins_over_the_step() {
    let s = after(
        &LiveSchedule::start(t0()),
        &limited(Some(minutes(45))),
        t0(),
    );
    assert_eq!(s.next_due, t0() + minutes(45));
    assert_eq!(s.wait_until, Some(t0() + minutes(45)));
}

#[test]
fn a_short_retry_after_loses_to_the_step() {
    let s = after(
        &LiveSchedule::start(t0()),
        &limited(Some(seconds(30))),
        t0(),
    );
    assert_eq!(s.next_due, t0() + minutes(5));
}

#[test]
fn an_ok_resets_the_step_and_the_wait() {
    let s = after(&LiveSchedule::start(t0()), &limited(None), t0());
    let s = after(&s, &limited(None), t0() + minutes(5));
    let s = after(&s, &ok(), t0() + minutes(15));
    assert_eq!(s.rate_limit_step, 0);
    assert_eq!(s.wait_until, None);
    assert_eq!(s.next_due, t0() + minutes(20));
    let s = after(&s, &limited(None), t0() + minutes(20));
    assert_eq!(s.next_due, t0() + minutes(25));
}

#[test]
fn a_non_rate_limit_failure_does_not_reset_the_step() {
    let s = after(&LiveSchedule::start(t0()), &limited(None), t0());
    let s = after(&s, &failed(), t0() + minutes(5));
    let s = after(&s, &limited(None), t0() + minutes(7));
    assert_eq!(s.next_due, t0() + minutes(17));
}

#[test]
fn no_credentials_check_again_in_a_minute() {
    let s = LiveSchedule::start(t0()).after_no_credentials(t0());
    assert_eq!(s.next_due, t0() + minutes(1));
    assert!(!s.is_due(t0() + seconds(59)));
    assert!(s.is_due(t0() + minutes(1)));
}

#[test]
fn after_no_reading_waits_five_minutes_and_blocks_open() {
    let s = LiveSchedule::start(t0()).after_no_reading(t0());
    assert_eq!(s.next_due, t0() + minutes(5));
    assert!(!s.is_due(t0() + seconds(294)));
    assert!(s.is_due(t0() + minutes(5)));
    assert!(!s.fetch_on_open(t0() + minutes(2)));
    assert!(s.fetch_on_open(t0() + minutes(5)));
}

#[test]
fn start_has_no_misses() {
    assert_eq!(LiveSchedule::start(t0()).misses, 0);
}

#[test]
fn a_success_after_misses_gives_zero() {
    let s = LiveSchedule::start(t0())
        .after_no_reading(t0())
        .after_no_reading(t0() + minutes(5));
    let s = after(&s, &ok(), t0() + minutes(10));
    assert_eq!(s.misses, 0);
}

#[test]
fn each_failed_outcome_adds_a_miss() {
    for outcome in [
        LiveOutcome::RateLimited,
        LiveOutcome::Failed,
        LiveOutcome::Unreachable,
    ] {
        let s = after(&LiveSchedule::start(t0()), &result(outcome), t0());
        assert_eq!(s.misses, 1, "{outcome:?}");
    }
}

#[test]
fn no_credentials_refused_and_no_reading_each_add_a_miss() {
    let start = LiveSchedule::start(t0());
    assert_eq!(start.after_no_credentials(t0()).misses, 1);
    assert_eq!(start.after_refused(Some(t0() + hours(1)), t0()).misses, 1);
    assert_eq!(start.after_no_reading(t0()).misses, 1);
}

#[test]
fn three_failures_in_a_row_give_three() {
    let s = after(&LiveSchedule::start(t0()), &failed(), t0());
    let s = after(&s, &failed(), t0() + minutes(2));
    let s = after(&s, &failed(), t0() + minutes(4));
    assert_eq!(s.misses, 3);
}

// ---- New: what the C# does not test, or cannot do ------------------------------------------

#[test]
fn a_failure_goes_through_after_failure_as_through_after_fetch() {
    let s = LiveSchedule::start(t0()).after_refused(Some(t0() + hours(2)), t0());
    assert_eq!(
        s.after_failure(t0() + minutes(1)),
        after(&s, &failed(), t0() + minutes(1))
    );
}

// A count that could not go on in the C# went round to a number below zero, and a moment past
// the end of its calendar stopped it with an error. Neither can be reached by a clock that is
// right. Here the count stays at its top and the moment at the calendar's last.
#[test]
fn the_counts_stop_at_the_top_and_never_go_round() {
    let full = LiveSchedule {
        misses: u32::MAX,
        rate_limit_step: u32::MAX,
        ..LiveSchedule::start(t0())
    };
    assert_eq!(full.after_failure(t0()).misses, u32::MAX);
    assert_eq!(full.after_no_credentials(t0()).misses, u32::MAX);
    assert_eq!(full.after_no_reading(t0()).misses, u32::MAX);
    assert_eq!(full.after_refused(None, t0()).misses, u32::MAX);
    let limited = after(&full, &limited(None), t0());
    assert_eq!(limited.misses, u32::MAX);
    assert_eq!(limited.rate_limit_step, 4);
    assert_eq!(limited.next_due, t0() + minutes(30));
}

#[test]
fn a_moment_past_the_end_of_the_calendar_is_the_calendars_last() {
    let last = DateTime::<Utc>::MAX_UTC;
    let start = LiveSchedule::start(t0());
    assert_eq!(after(&start, &ok(), last).next_due, last);
    assert_eq!(after(&start, &failed(), last).next_due, last);
    assert_eq!(after(&start, &limited(None), last).wait_until, Some(last));
    assert_eq!(start.after_no_credentials(last).next_due, last);
    assert_eq!(start.after_no_reading(last).next_due, last);
    assert_eq!(start.after_refused(None, last).next_due, last);
    // A wait as long as a span of time can be.
    let longest = after(&start, &limited(Some(TimeDelta::MAX)), t0());
    assert_eq!(longest.next_due, last);
    assert_eq!(longest.wait_until, Some(last));
    // And the two ends of the calendar against each other.
    let first = DateTime::<Utc>::MIN_UTC;
    let at_the_end = LiveSchedule {
        last_success: Some(last),
        ..start
    };
    assert!(!at_the_end.fetch_on_open(first));
    let at_the_start = LiveSchedule {
        last_success: Some(first),
        ..start
    };
    assert!(at_the_start.fetch_on_open(last));
}

// Nothing a schedule holds could be a token: it is moments and counts, and prints as such.
#[test]
fn a_schedule_holds_moments_and_counts_and_prints_them() {
    let s = LiveSchedule::start(t0()).after_refused(Some(t0() + hours(1)), t0());
    assert_eq!(
        format!("{s:?}"),
        "LiveSchedule { next_due: 2026-11-09T12:01:00Z, last_success: None, rate_limit_step: 0, \
         wait_until: None, refused: Some(RefusedSignIn { expires_at: Some(2026-11-09T13:00:00Z) }), \
         misses: 1 }"
    );
    assert_eq!(
        s,
        LiveSchedule::start(t0()).after_refused(Some(t0() + hours(1)), t0())
    );
}

// The clock of a PC can be set back. A schedule made before that waits until the old moment
// comes round again, and opening the flyout does not ask sooner: this is what 1.x does, kept
// as it is. The rows of `is_due_and_fetch_on_open_answer_as_the_csharp_did` hold more of it.
#[test]
fn a_clock_set_back_waits_for_the_old_moment() {
    let s = after(&LiveSchedule::start(t0()), &ok(), t0());
    let yesterday = t0() - TimeDelta::days(1);
    assert!(!s.is_due(yesterday));
    assert!(!s.fetch_on_open(yesterday));
    assert!(!s.is_due(yesterday + hours(23)));
    // A poll that does run then is counted from the clock as it is now.
    let again = after(&s, &ok(), yesterday);
    assert_eq!(again.next_due, yesterday + minutes(5));
    assert_eq!(again.last_success, Some(yesterday));
}

// A clock set forward makes the next poll due at once, and the one after it is counted from
// the new time.
#[test]
fn a_clock_set_forward_is_due_at_once() {
    let s = after(&LiveSchedule::start(t0()), &limited(Some(hours(6))), t0());
    let next_year = t0() + TimeDelta::days(365);
    assert!(s.is_due(next_year));
    assert!(s.fetch_on_open(next_year));
    assert_eq!(after(&s, &ok(), next_year).next_due, next_year + minutes(5));
}

// ---- Rows the C# wrote ---------------------------------------------------------------------
//
// Everything below was written by a script from the answers of the C# of 1.1.0, which was run
// on these made-up cases on 2026-11-14. Nothing below was typed. A moment is a count of ticks
// from noon on 2026-11-09 (UTC), written with the units of `main.rs`: `5 * M + 1` is five
// minutes and one tick after it. `NONE` stands where there is no moment.

/// What happens to a schedule in a row.
#[derive(Debug, Clone, Copy)]
enum E {
    Ok,
    Failed,
    Unreachable,
    Unauthorized,
    /// A 429. The row's next number is the wait it asked for, in ticks.
    Limited,
    /// A refusal. The row's next number is the moment the refused sign-in runs out.
    Refused,
    NoCredentials,
    NoReading,
}

/// The schedule after `event` at `now`, or `None` where `after_fetch` turns the event away.
fn apply(schedule: &LiveSchedule, event: E, with: i64, now: DateTime<Utc>) -> Option<LiveSchedule> {
    let fetched = |outcome| {
        let result = LiveResult {
            outcome,
            limits: None,
            retry_after: (with != NONE).then(|| TimeDelta::nanoseconds(with * 100)),
        };
        schedule.after_fetch(&result, now).ok()
    };
    match event {
        E::Ok => fetched(LiveOutcome::Ok),
        E::Failed => fetched(LiveOutcome::Failed),
        E::Unreachable => fetched(LiveOutcome::Unreachable),
        E::Unauthorized => fetched(LiveOutcome::Unauthorized),
        E::Limited => fetched(LiveOutcome::RateLimited),
        E::Refused => Some(schedule.after_refused(maybe_at(with), now)),
        E::NoCredentials => Some(schedule.after_no_credentials(now)),
        E::NoReading => Some(schedule.after_no_reading(now)),
    }
}

#[test]
fn every_event_answers_as_the_csharp_did() {
    // A schedule, what happens to it, the wait or the expiry that comes with it, when it
    // happens, and the schedule afterwards. `None` afterwards: the C# stopped with an error,
    // and the Rust gives `Err`. A wait is given with an ok and a failure too, to show that it
    // is not looked at.
    #[rustfmt::skip]
    let rows: &[(ScheduleRow, E, i64, i64, Option<ScheduleRow>)] = &[
        ((0, NONE, 0, NONE, NONE, 0), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 0, NONE, NONE, 0), E::Ok, NONE, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((0, NONE, 0, NONE, NONE, 0), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 0, NONE, NONE, 0), E::Ok, 45 * M, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((0, NONE, 0, NONE, NONE, 0), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Failed, NONE, -D, Some((-1438 * M, NONE, 0, -1438 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Failed, 45 * M, -D, Some((-1438 * M, NONE, 0, -1438 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Unreachable, NONE, -D, Some((-1438 * M, NONE, 0, -1438 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, NONE, 0, NONE, NONE, 0), E::Unauthorized, NONE, -D, None),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, NONE, -D, Some((-1435 * M, NONE, 1, -1435 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 0, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 0, -D, Some((-1435 * M, NONE, 1, -1435 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, -M, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, -M, -D, Some((-1435 * M, NONE, 1, -1435 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * S, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * S, -D, Some((-1435 * M, NONE, 1, -1435 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 5 * M, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 5 * M, -D, Some((-1435 * M, NONE, 1, -1435 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 5 * M + 1, 5 * M + 1, Some((10 * M + 2, NONE, 1, 10 * M + 2, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 5 * M + 1, -D, Some((-1435 * M + 1, NONE, 1, -1435 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, NONE, 1, 15 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 10 * M, -D, Some((-1430 * M, NONE, 1, -1430 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, NONE, 1, 15 * M + 2, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 10 * M + 1, -D, Some((-1430 * M + 1, NONE, 1, -1430 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, NONE, 1, 25 * M + 2, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 20 * M + 1, -D, Some((-1420 * M + 1, NONE, 1, -1420 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 1, 35 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * M, -D, Some((-1410 * M, NONE, 1, -1410 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 1, 35 * M + 2, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 30 * M + 1, -D, Some((-1410 * M + 1, NONE, 1, -1410 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 1, 50 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 45 * M, -D, Some((-1395 * M, NONE, 1, -1395 * M, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 1, 365 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 6 * H, -D, Some((-18 * H, NONE, 1, -18 * H, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 1, 10085 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Limited, 7 * D, -D, Some((6 * D, NONE, 1, 6 * D, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, NO_EXPIRY, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, NONE, -D, Some((-1439 * M, NONE, 0, NONE, NO_EXPIRY, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, 2 * H, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, 2 * H, -D, Some((-1439 * M, NONE, 0, NONE, 2 * H, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, 10 * H, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::Refused, 10 * H, -D, Some((-1439 * M, NONE, 0, NONE, 10 * H, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::NoCredentials, NONE, -D, Some((-1439 * M, NONE, 0, NONE, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 0, 10 * M + 1, NONE, 1))),
        ((0, NONE, 0, NONE, NONE, 0), E::NoReading, NONE, -D, Some((-1435 * M, NONE, 0, -1435 * M, NONE, 1))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 1, 7 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 1, 7 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 1, 7 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Unauthorized, NONE, 5 * M + 1, None),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, NONE, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 0, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, -M, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 30 * S, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 5 * M, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 5 * M + 1, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, NONE, 2, 15 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, NONE, 2, 15 * M + 2, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, NONE, 2, 25 * M + 2, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 2, 35 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 2, 35 * M + 2, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 2, 50 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 2, 365 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 2, 10085 * M + 1, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 1, NONE, NO_EXPIRY, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 1, NONE, 2 * H, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 1, NONE, 10 * H, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 1, NONE, NONE, 2))),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((0, NONE, 2, -M, NONE, 2), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 2, -M, NONE, 2), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 2, -M, NONE, 2), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 2, 7 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 2, 7 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 2, 7 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, NONE, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 0, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, -M, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 30 * S, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 5 * M, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 5 * M + 1, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 10 * M, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 10 * M + 1, 5 * M + 1, Some((25 * M + 1, NONE, 3, 25 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, NONE, 3, 25 * M + 2, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 3, 35 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 3, 35 * M + 2, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 3, 50 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 3, 365 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 3, 10085 * M + 1, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 2, NONE, NO_EXPIRY, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 2, NONE, 2 * H, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 2, NONE, 10 * H, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 2, NONE, NONE, 3))),
        ((0, NONE, 2, -M, NONE, 2), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 2, 10 * M + 1, NONE, 3))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Ok, NONE, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Ok, 45 * M, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 3, 7 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Failed, NONE, -D, Some((-1438 * M, NONE, 3, -1438 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 3, 7 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Failed, 45 * M, -D, Some((-1438 * M, NONE, 3, -1438 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 3, 7 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Unreachable, NONE, -D, Some((-1438 * M, NONE, 3, -1438 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Unauthorized, NONE, 5 * M + 1, None),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Unauthorized, NONE, -D, None),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, NONE, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, NONE, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 0, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 0, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, -M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, -M, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * S, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * S, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 5 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 5 * M, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 5 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 5 * M + 1, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 10 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 10 * M, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 10 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 10 * M + 1, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 20 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 20 * M + 1, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * M, -D, Some((-1410 * M, NONE, 4, -1410 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 4, 35 * M + 2, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 30 * M + 1, -D, Some((-1410 * M + 1, NONE, 4, -1410 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 4, 50 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 45 * M, -D, Some((-1395 * M, NONE, 4, -1395 * M, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 4, 365 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 6 * H, -D, Some((-18 * H, NONE, 4, -18 * H, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 4, 10085 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Limited, 7 * D, -D, Some((6 * D, NONE, 4, 6 * D, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 3, NONE, NO_EXPIRY, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, NONE, -D, Some((-1439 * M, NONE, 3, NONE, NO_EXPIRY, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 3, NONE, 2 * H, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, 2 * H, -D, Some((-1439 * M, NONE, 3, NONE, 2 * H, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 3, NONE, 10 * H, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::Refused, 10 * H, -D, Some((-1439 * M, NONE, 3, NONE, 10 * H, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 3, NONE, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::NoCredentials, NONE, -D, Some((-1439 * M, NONE, 3, NONE, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 3, 10 * M + 1, NONE, 4))),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), E::NoReading, NONE, -D, Some((-1435 * M, NONE, 3, -1435 * M, NONE, 4))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 4, 7 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 4, 7 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 4, 7 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Unauthorized, NONE, 5 * M + 1, None),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, NONE, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 0, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, -M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 30 * S, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 5 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 5 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 10 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 10 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 20 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 4, 35 * M + 2, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 4, 50 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 4, 365 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 4, 10085 * M + 1, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 4, NONE, NO_EXPIRY, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 4, NONE, 2 * H, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 4, NONE, 10 * H, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 4, NONE, NONE, 5))),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 4, 10 * M + 1, NONE, 5))),
        ((0, NONE, 5, NONE, NONE, 5), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 5, NONE, NONE, 5), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 5, NONE, NONE, 5), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 5, 7 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 5, 7 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 5, 7 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, NONE, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 0, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, -M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 30 * S, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 5 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 5 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 10 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 10 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 20 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 4, 35 * M + 2, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 4, 50 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 4, 365 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 4, 10085 * M + 1, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 5, NONE, NO_EXPIRY, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 5, NONE, 2 * H, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 5, NONE, 10 * H, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 5, NONE, NONE, 6))),
        ((0, NONE, 5, NONE, NONE, 5), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 5, 10 * M + 1, NONE, 6))),
        ((0, NONE, 9, NONE, NONE, 40), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 9, NONE, NONE, 40), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, NONE, 9, NONE, NONE, 40), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 9, 7 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 9, 7 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 9, 7 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, NONE, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 0, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, -M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 30 * S, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 5 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 5 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 10 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 10 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 20 * M + 1, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 4, 35 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 4, 35 * M + 2, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 4, 50 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 4, 365 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 4, 10085 * M + 1, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 9, NONE, NO_EXPIRY, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 9, NONE, 2 * H, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 9, NONE, 10 * H, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 9, NONE, NONE, 41))),
        ((0, NONE, 9, NONE, NONE, 40), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 9, 10 * M + 1, NONE, 41))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, NONE, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 0, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, -M, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 30 * S, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 5 * M, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 5 * M + 1, 5 * M + 1, Some((10 * M + 2, -5 * M, 1, 10 * M + 2, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, -5 * M, 1, 15 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, -5 * M, 1, 15 * M + 2, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, -5 * M, 1, 25 * M + 2, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, -5 * M, 1, 35 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, -5 * M, 1, 35 * M + 2, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, -5 * M, 1, 50 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, -5 * M, 1, 365 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, -5 * M, 1, 10085 * M + 1, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, NO_EXPIRY, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, 2 * H, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, 10 * H, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, NONE, 1))),
        ((0, -5 * M, 0, NONE, NONE, 0), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, -5 * M, 0, 10 * M + 1, NONE, 1))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, -5 * M, 0, 7 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Unauthorized, NONE, 5 * M + 1, None),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, NONE, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 0, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, -M, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 30 * S, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 5 * M, 5 * M + 1, Some((10 * M + 1, -5 * M, 1, 10 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 5 * M + 1, 5 * M + 1, Some((10 * M + 2, -5 * M, 1, 10 * M + 2, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, -5 * M, 1, 15 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, -5 * M, 1, 15 * M + 2, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, -5 * M, 1, 25 * M + 2, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, -5 * M, 1, 35 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, -5 * M, 1, 35 * M + 2, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, -5 * M, 1, 50 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, -5 * M, 1, 365 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, -5 * M, 1, 10085 * M + 1, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, NO_EXPIRY, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, 2 * H, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, 10 * H, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, -5 * M, 0, NONE, NONE, 2))),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, -5 * M, 0, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, NONE, 0, 7 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Unauthorized, NONE, 5 * M + 1, None),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 0, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, -M, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 30 * S, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 5 * M, 5 * M + 1, Some((10 * M + 1, NONE, 1, 10 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 5 * M + 1, 5 * M + 1, Some((10 * M + 2, NONE, 1, 10 * M + 2, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, NONE, 1, 15 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, NONE, 1, 15 * M + 2, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, NONE, 1, 25 * M + 2, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, NONE, 1, 35 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, NONE, 1, 35 * M + 2, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, NONE, 1, 50 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, NONE, 1, 365 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, NONE, 1, 10085 * M + 1, NONE, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, NO_EXPIRY, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, 2 * H, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, 10 * H, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, NONE, 0, NONE, 2 * H, 2))),
        ((M, NONE, 0, NONE, 2 * H, 1), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, NONE, 0, 10 * M + 1, NONE, 2))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, -H, 0, 7 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, -H, 0, 7 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, -H, 0, 7 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Unauthorized, NONE, 5 * M + 1, None),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, NONE, 5 * M + 1, Some((10 * M + 1, -H, 1, 10 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 0, 5 * M + 1, Some((10 * M + 1, -H, 1, 10 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, -M, 5 * M + 1, Some((10 * M + 1, -H, 1, 10 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 30 * S, 5 * M + 1, Some((10 * M + 1, -H, 1, 10 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 5 * M, 5 * M + 1, Some((10 * M + 1, -H, 1, 10 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 5 * M + 1, 5 * M + 1, Some((10 * M + 2, -H, 1, 10 * M + 2, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 10 * M, 5 * M + 1, Some((15 * M + 1, -H, 1, 15 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 10 * M + 1, 5 * M + 1, Some((15 * M + 2, -H, 1, 15 * M + 2, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, -H, 1, 25 * M + 2, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, -H, 1, 35 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, -H, 1, 35 * M + 2, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, -H, 1, 50 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, -H, 1, 365 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, -H, 1, 10085 * M + 1, NONE, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, -H, 0, NONE, NO_EXPIRY, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, -H, 0, NONE, 2 * H, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, -H, 0, NONE, 10 * H, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, -H, 0, NONE, NO_EXPIRY, 8))),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, -H, 0, 10 * M + 1, NONE, 8))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Ok, NONE, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Ok, NONE, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Ok, 45 * M, 5 * M + 1, Some((10 * M + 1, 5 * M + 1, 0, NONE, NONE, 0))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Ok, 45 * M, -D, Some((-1435 * M, -D, 0, NONE, NONE, 0))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Failed, NONE, 5 * M + 1, Some((7 * M + 1, -D, 2, 7 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Failed, NONE, -D, Some((-1438 * M, -D, 2, -1438 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Failed, 45 * M, 5 * M + 1, Some((7 * M + 1, -D, 2, 7 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Failed, 45 * M, -D, Some((-1438 * M, -D, 2, -1438 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Unreachable, NONE, 5 * M + 1, Some((7 * M + 1, -D, 2, 7 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Unreachable, NONE, -D, Some((-1438 * M, -D, 2, -1438 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Unauthorized, NONE, 5 * M + 1, None),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Unauthorized, NONE, -D, None),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, NONE, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, NONE, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 0, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 0, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, -M, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, -M, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * S, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * S, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 5 * M, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 5 * M, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 5 * M + 1, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 5 * M + 1, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 10 * M, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 10 * M, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 10 * M + 1, 5 * M + 1, Some((25 * M + 1, -D, 3, 25 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 10 * M + 1, -D, Some((-1420 * M, -D, 3, -1420 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 20 * M + 1, 5 * M + 1, Some((25 * M + 2, -D, 3, 25 * M + 2, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 20 * M + 1, -D, Some((-1420 * M + 1, -D, 3, -1420 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * M, 5 * M + 1, Some((35 * M + 1, -D, 3, 35 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * M, -D, Some((-1410 * M, -D, 3, -1410 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * M + 1, 5 * M + 1, Some((35 * M + 2, -D, 3, 35 * M + 2, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 30 * M + 1, -D, Some((-1410 * M + 1, -D, 3, -1410 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 45 * M, 5 * M + 1, Some((50 * M + 1, -D, 3, 50 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 45 * M, -D, Some((-1395 * M, -D, 3, -1395 * M, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 6 * H, 5 * M + 1, Some((365 * M + 1, -D, 3, 365 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 6 * H, -D, Some((-18 * H, -D, 3, -18 * H, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 7 * D, 5 * M + 1, Some((10085 * M + 1, -D, 3, 10085 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Limited, 7 * D, -D, Some((6 * D, -D, 3, 6 * D, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, NONE, 5 * M + 1, Some((6 * M + 1, -D, 2, NONE, NO_EXPIRY, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, NONE, -D, Some((-1439 * M, -D, 2, NONE, NO_EXPIRY, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, 2 * H, 5 * M + 1, Some((6 * M + 1, -D, 2, NONE, 2 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, 2 * H, -D, Some((-1439 * M, -D, 2, NONE, 2 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, 10 * H, 5 * M + 1, Some((6 * M + 1, -D, 2, NONE, 10 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::Refused, 10 * H, -D, Some((-1439 * M, -D, 2, NONE, 10 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::NoCredentials, NONE, 5 * M + 1, Some((6 * M + 1, -D, 2, NONE, 2 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::NoCredentials, NONE, -D, Some((-1439 * M, -D, 2, NONE, 2 * H, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::NoReading, NONE, 5 * M + 1, Some((10 * M + 1, -D, 2, 10 * M + 1, NONE, 4))),
        ((0, -D, 2, 10 * M, 2 * H, 3), E::NoReading, NONE, -D, Some((-1435 * M, -D, 2, -1435 * M, NONE, 4))),
    ];
    assert_eq!(rows.len(), 375);
    for (index, &(before, event, with, now, after)) in rows.iter().enumerate() {
        let got = apply(&schedule_of(before), event, with, at_ticks(now));
        assert_eq!(got, after.map(schedule_of), "row {index}: {event:?}");
    }
}

#[test]
fn several_events_in_a_row_answer_as_the_csharp_did() {
    // Each walk begins with a schedule started at noon. A step is what happens, the wait or
    // the expiry that comes with it, when, and the schedule afterwards, which the next step
    // goes on from.
    type Step = (E, i64, i64, ScheduleRow);
    #[rustfmt::skip]
    let walks: &[(&str, &[Step])] = &[
        ("six 429s without a wait", &[
            (E::Limited, NONE, 0, (5 * M, NONE, 1, 5 * M, NONE, 1)),
            (E::Limited, NONE, 5 * M, (15 * M, NONE, 2, 15 * M, NONE, 2)),
            (E::Limited, NONE, 15 * M, (35 * M, NONE, 3, 35 * M, NONE, 3)),
            (E::Limited, NONE, 35 * M, (65 * M, NONE, 4, 65 * M, NONE, 4)),
            (E::Limited, NONE, 65 * M, (95 * M, NONE, 4, 95 * M, NONE, 5)),
            (E::Limited, NONE, 95 * M, (125 * M, NONE, 4, 125 * M, NONE, 6)),
        ]),
        ("six 429s that ask for 12 minutes", &[
            (E::Limited, 12 * M, 0, (12 * M, NONE, 1, 12 * M, NONE, 1)),
            (E::Limited, 12 * M, 12 * M, (24 * M, NONE, 2, 24 * M, NONE, 2)),
            (E::Limited, 12 * M, 24 * M, (44 * M, NONE, 3, 44 * M, NONE, 3)),
            (E::Limited, 12 * M, 44 * M, (74 * M, NONE, 4, 74 * M, NONE, 4)),
            (E::Limited, 12 * M, 74 * M, (104 * M, NONE, 4, 104 * M, NONE, 5)),
            (E::Limited, 12 * M, 104 * M, (134 * M, NONE, 4, 134 * M, NONE, 6)),
        ]),
        ("429s, a failure between, an ok, a 429", &[
            (E::Limited, NONE, 0, (5 * M, NONE, 1, 5 * M, NONE, 1)),
            (E::Limited, NONE, 5 * M, (15 * M, NONE, 2, 15 * M, NONE, 2)),
            (E::Failed, NONE, 15 * M, (17 * M, NONE, 2, 17 * M, NONE, 3)),
            (E::Limited, NONE, 17 * M, (37 * M, NONE, 3, 37 * M, NONE, 4)),
            (E::Ok, NONE, 37 * M, (42 * M, 37 * M, 0, NONE, NONE, 0)),
            (E::Limited, NONE, 42 * M, (47 * M, 37 * M, 1, 47 * M, NONE, 1)),
        ]),
        ("five failures and an ok", &[
            (E::Failed, NONE, 0, (2 * M, NONE, 0, 2 * M, NONE, 1)),
            (E::Unreachable, NONE, 2 * M, (4 * M, NONE, 0, 4 * M, NONE, 2)),
            (E::Failed, NONE, 4 * M, (6 * M, NONE, 0, 6 * M, NONE, 3)),
            (E::Unreachable, NONE, 6 * M, (8 * M, NONE, 0, 8 * M, NONE, 4)),
            (E::Failed, NONE, 8 * M, (10 * M, NONE, 0, 10 * M, NONE, 5)),
            (E::Ok, NONE, 10 * M, (15 * M, 10 * M, 0, NONE, NONE, 0)),
        ]),
        ("refused, read again three times, a new sign-in refused, an ok", &[
            (E::Refused, 2 * H, 0, (M, NONE, 0, NONE, 2 * H, 1)),
            (E::NoCredentials, NONE, M, (2 * M, NONE, 0, NONE, 2 * H, 2)),
            (E::NoCredentials, NONE, 2 * M, (3 * M, NONE, 0, NONE, 2 * H, 3)),
            (E::NoCredentials, NONE, 3 * M, (4 * M, NONE, 0, NONE, 2 * H, 4)),
            (E::Refused, 10 * H, 4 * M, (5 * M, NONE, 0, NONE, 10 * H, 5)),
            (E::Ok, NONE, 5 * M, (10 * M, 5 * M, 0, NONE, NONE, 0)),
        ]),
        ("no sign-in for four minutes, then an ok and a 429", &[
            (E::NoCredentials, NONE, 0, (M, NONE, 0, NONE, NONE, 1)),
            (E::NoCredentials, NONE, M, (2 * M, NONE, 0, NONE, NONE, 2)),
            (E::NoCredentials, NONE, 2 * M, (3 * M, NONE, 0, NONE, NONE, 3)),
            (E::NoCredentials, NONE, 3 * M, (4 * M, NONE, 0, NONE, NONE, 4)),
            (E::Ok, NONE, 4 * M, (9 * M, 4 * M, 0, NONE, NONE, 0)),
            (E::Limited, NONE, 9 * M, (14 * M, 4 * M, 1, 14 * M, NONE, 1)),
        ]),
        ("no reading three times, then a failure", &[
            (E::NoReading, NONE, 0, (5 * M, NONE, 0, 5 * M, NONE, 1)),
            (E::NoReading, NONE, 5 * M, (10 * M, NONE, 0, 10 * M, NONE, 2)),
            (E::NoReading, NONE, 10 * M, (15 * M, NONE, 0, 15 * M, NONE, 3)),
            (E::Failed, NONE, 15 * M, (17 * M, NONE, 0, 17 * M, NONE, 4)),
        ]),
        ("an ok, the clock goes back a day, an ok, the clock comes back", &[
            (E::Ok, NONE, 0, (5 * M, 0, 0, NONE, NONE, 0)),
            (E::Ok, NONE, -D, (-1435 * M, -D, 0, NONE, NONE, 0)),
            (E::Failed, NONE, -1435 * M, (-1433 * M, -D, 0, -1433 * M, NONE, 1)),
            (E::Ok, NONE, 6 * M, (11 * M, 6 * M, 0, NONE, NONE, 0)),
        ]),
        ("a 429 with a long wait, the clock goes back an hour, a 429", &[
            (E::Limited, 6 * H, 0, (6 * H, NONE, 1, 6 * H, NONE, 1)),
            (E::Limited, NONE, -H, (-50 * M, NONE, 2, -50 * M, NONE, 2)),
            (E::Limited, NONE, -50 * M, (-30 * M, NONE, 3, -30 * M, NONE, 3)),
        ]),
    ];
    assert_eq!(walks.len(), 9);
    for (name, steps) in walks {
        let mut schedule = LiveSchedule::start(at_ticks(0));
        for (index, &(event, with, now, after)) in steps.iter().enumerate() {
            schedule = apply(&schedule, event, with, at_ticks(now)).unwrap();
            assert_eq!(schedule, schedule_of(after), "{name}, step {index}");
        }
    }
}

#[test]
fn is_due_and_fetch_on_open_answer_as_the_csharp_did() {
    // A schedule, a moment, whether a poll is due then, and whether opening the flyout asks
    // then. Among the moments: one tick before and after each moment the schedule holds, a
    // day and an hour before noon (the clock was set back), and ten years on.
    #[rustfmt::skip]
    let rows: &[(ScheduleRow, i64, bool, bool)] = &[
        ((0, NONE, 0, NONE, NONE, 0), -D, false, true),
        ((0, NONE, 0, NONE, NONE, 0), -H, false, true),
        ((0, NONE, 0, NONE, NONE, 0), -S + 9_999_999, false, true),
        ((0, NONE, 0, NONE, NONE, 0), 0, true, true),
        ((0, NONE, 0, NONE, NONE, 0), 1, true, true),
        ((0, NONE, 0, NONE, NONE, 0), 3650 * D, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -D, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -H, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -S + 9_999_999, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 0, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 1, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 299 * S + 9_999_999, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 5 * M, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 5 * M + 1, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 3650 * D, true, true),
        ((0, NONE, 2, -M, NONE, 2), -D, false, false),
        ((0, NONE, 2, -M, NONE, 2), -H, false, false),
        ((0, NONE, 2, -M, NONE, 2), -61 * S + 9_999_999, false, false),
        ((0, NONE, 2, -M, NONE, 2), -M, false, true),
        ((0, NONE, 2, -M, NONE, 2), -M + 1, false, true),
        ((0, NONE, 2, -M, NONE, 2), -S + 9_999_999, false, true),
        ((0, NONE, 2, -M, NONE, 2), 0, true, true),
        ((0, NONE, 2, -M, NONE, 2), 1, true, true),
        ((0, NONE, 2, -M, NONE, 2), 3650 * D, true, true),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), -D, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), -H, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), -S + 9_999_999, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 0, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 1, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 1199 * S + 9_999_999, false, false),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 20 * M, true, true),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 20 * M + 1, true, true),
        ((20 * M, NONE, 3, 20 * M, NONE, 3), 3650 * D, true, true),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), -D, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), -H, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), -S + 9_999_999, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 0, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 1, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 1799 * S + 9_999_999, false, false),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 30 * M, true, true),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 30 * M + 1, true, true),
        ((30 * M, NONE, 4, 30 * M, NONE, 4), 3650 * D, true, true),
        ((0, NONE, 5, NONE, NONE, 5), -D, false, true),
        ((0, NONE, 5, NONE, NONE, 5), -H, false, true),
        ((0, NONE, 5, NONE, NONE, 5), -S + 9_999_999, false, true),
        ((0, NONE, 5, NONE, NONE, 5), 0, true, true),
        ((0, NONE, 5, NONE, NONE, 5), 1, true, true),
        ((0, NONE, 5, NONE, NONE, 5), 3650 * D, true, true),
        ((0, NONE, 9, NONE, NONE, 40), -D, false, true),
        ((0, NONE, 9, NONE, NONE, 40), -H, false, true),
        ((0, NONE, 9, NONE, NONE, 40), -S + 9_999_999, false, true),
        ((0, NONE, 9, NONE, NONE, 40), 0, true, true),
        ((0, NONE, 9, NONE, NONE, 40), 1, true, true),
        ((0, NONE, 9, NONE, NONE, 40), 3650 * D, true, true),
        ((0, -5 * M, 0, NONE, NONE, 0), -D, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -H, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -301 * S + 9_999_999, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -5 * M, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -5 * M + 1, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -241 * S + 9_999_999, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -4 * M, false, false),
        ((0, -5 * M, 0, NONE, NONE, 0), -4 * M + 1, false, true),
        ((0, -5 * M, 0, NONE, NONE, 0), -S + 9_999_999, false, true),
        ((0, -5 * M, 0, NONE, NONE, 0), 0, true, true),
        ((0, -5 * M, 0, NONE, NONE, 0), 1, true, true),
        ((0, -5 * M, 0, NONE, NONE, 0), 3650 * D, true, true),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -D, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -H, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -301 * S + 9_999_999, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -5 * M, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -5 * M + 1, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -241 * S + 9_999_999, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -4 * M, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -4 * M + 1, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), -S + 9_999_999, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 0, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 1, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 119 * S + 9_999_999, false, false),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 2 * M, true, true),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 2 * M + 1, true, true),
        ((2 * M, -5 * M, 0, 2 * M, NONE, 1), 3650 * D, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -D, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -H, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 0, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 1, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 59 * S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), M, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), M + 1, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 3650 * D, true, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -D, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -3601 * S + 9_999_999, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -H, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -H + 1, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -3541 * S + 9_999_999, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -59 * M, false, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -59 * M + 1, false, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), -S + 9_999_999, false, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 0, false, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 1, false, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 59 * S + 9_999_999, false, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), M, true, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), M + 1, true, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 3650 * D, true, true),
        ((0, -D, 2, 10 * M, 2 * H, 3), -86401 * S + 9_999_999, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -D, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -D + 1, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -86341 * S + 9_999_999, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -1439 * M, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -1439 * M + 1, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -H, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), -S + 9_999_999, false, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 0, true, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 1, true, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 599 * S + 9_999_999, true, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 10 * M, true, true),
        ((0, -D, 2, 10 * M, 2 * H, 3), 10 * M + 1, true, true),
        ((0, -D, 2, 10 * M, 2 * H, 3), 3650 * D, true, true),
        ((5 * M, 0, 0, NONE, NONE, 0), -D, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), -H, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), -S + 9_999_999, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), 0, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), 1, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), 59 * S + 9_999_999, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), M, false, false),
        ((5 * M, 0, 0, NONE, NONE, 0), M + 1, false, true),
        ((5 * M, 0, 0, NONE, NONE, 0), 299 * S + 9_999_999, false, true),
        ((5 * M, 0, 0, NONE, NONE, 0), 5 * M, true, true),
        ((5 * M, 0, 0, NONE, NONE, 0), 5 * M + 1, true, true),
        ((5 * M, 0, 0, NONE, NONE, 0), 3650 * D, true, true),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), -D, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), -H, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), -S + 9_999_999, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 0, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 1, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 119 * S + 9_999_999, false, false),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 2 * M, true, true),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 2 * M + 1, true, true),
        ((2 * M, NONE, 0, 2 * M, NONE, 1), 3650 * D, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -D, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -H, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), -S + 9_999_999, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 0, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 1, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 299 * S + 9_999_999, false, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 5 * M, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 5 * M + 1, true, true),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 3650 * D, true, true),
        ((M, NONE, 0, NONE, NONE, 1), -D, false, true),
        ((M, NONE, 0, NONE, NONE, 1), -H, false, true),
        ((M, NONE, 0, NONE, NONE, 1), -S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, NONE, 1), 0, false, true),
        ((M, NONE, 0, NONE, NONE, 1), 1, false, true),
        ((M, NONE, 0, NONE, NONE, 1), 59 * S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, NONE, 1), M, true, true),
        ((M, NONE, 0, NONE, NONE, 1), M + 1, true, true),
        ((M, NONE, 0, NONE, NONE, 1), 3650 * D, true, true),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), -D, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), -H, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), -S + 9_999_999, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 0, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 1, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 299 * S + 9_999_999, false, false),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 5 * M, true, true),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 5 * M + 1, true, true),
        ((5 * M, NONE, 0, 5 * M, NONE, 1), 3650 * D, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -D, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -H, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), -S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 0, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 1, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 59 * S + 9_999_999, false, true),
        ((M, NONE, 0, NONE, 2 * H, 1), M, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), M + 1, true, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 3650 * D, true, true),
        ((7 * M, 0, 0, 7 * M, NONE, 1), -D, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), -H, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), -S + 9_999_999, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 0, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 1, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 59 * S + 9_999_999, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), M, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), M + 1, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 419 * S + 9_999_999, false, false),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 7 * M, true, true),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 7 * M + 1, true, true),
        ((7 * M, 0, 0, 7 * M, NONE, 1), 3650 * D, true, true),
    ];
    assert_eq!(rows.len(), 187);
    for (index, &(schedule, now, due, on_open)) in rows.iter().enumerate() {
        let schedule = schedule_of(schedule);
        assert_eq!(schedule.is_due(at_ticks(now)), due, "row {index}: is_due");
        assert_eq!(
            schedule.fetch_on_open(at_ticks(now)),
            on_open,
            "row {index}: fetch_on_open"
        );
    }
}

#[test]
fn refuses_answers_as_the_csharp_did() {
    // A schedule, the moment a sign-in runs out (`NONE`: its file names none), and whether
    // the schedule refuses that sign-in.
    #[rustfmt::skip]
    let rows: &[(ScheduleRow, i64, bool)] = &[
        ((0, NONE, 0, NONE, NONE, 0), NONE, false),
        ((0, NONE, 0, NONE, NONE, 0), 2 * H, false),
        ((0, NONE, 0, NONE, NONE, 0), 2 * H + 1, false),
        ((0, NONE, 0, NONE, NONE, 0), 7199 * S + 9_999_999, false),
        ((0, NONE, 0, NONE, NONE, 0), 10 * H, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), NONE, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 2 * H, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 2 * H + 1, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 7199 * S + 9_999_999, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 10 * H, false),
        ((M, NONE, 0, NONE, 2 * H, 1), NONE, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 2 * H, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 2 * H + 1, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 7199 * S + 9_999_999, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 10 * H, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), NONE, true),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 2 * H, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 2 * H + 1, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 7199 * S + 9_999_999, false),
        ((M, -H, 0, NONE, NO_EXPIRY, 7), 10 * H, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), NONE, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 2 * H, true),
        ((0, -D, 2, 10 * M, 2 * H, 3), 2 * H + 1, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 7199 * S + 9_999_999, false),
        ((0, -D, 2, 10 * M, 2 * H, 3), 10 * H, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), NONE, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 2 * H, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 2 * H + 1, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 7199 * S + 9_999_999, false),
        ((5 * M, NONE, 1, 5 * M, NONE, 1), 10 * H, false),
        ((M, NONE, 0, NONE, 2 * H, 1), NONE, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 2 * H, true),
        ((M, NONE, 0, NONE, 2 * H, 1), 2 * H + 1, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 7199 * S + 9_999_999, false),
        ((M, NONE, 0, NONE, 2 * H, 1), 10 * H, false),
    ];
    assert_eq!(rows.len(), 35);
    for (index, &(schedule, expiry, refuses)) in rows.iter().enumerate() {
        assert_eq!(
            schedule_of(schedule).refuses(maybe_at(expiry)),
            refuses,
            "row {index}"
        );
    }
}
