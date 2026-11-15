//! Ported from `tests/Sobreiro.Tests/TaskbarQueryScheduleTests.cs`. The tests after
//! `the_slow_check_is_15_seconds` are new.
//!
//! A moment is a span since some start, as the shell counts it: no clock is asked.
use chrono::TimeDelta;
use sobreiro_core::taskbar::taskbar_layout::TaskbarAnchors;
use sobreiro_core::taskbar::taskbar_query_schedule::{SLOW_CHECK, TaskbarQuerySchedule};

const A: TaskbarAnchors = TaskbarAnchors {
    tray_left: 2200,
    buttons_right: 900,
    band_top: 1392,
    band_bottom: 1440,
};
const B: TaskbarAnchors = TaskbarAnchors {
    tray_left: 2170,
    ..A
};

fn s(seconds: f64) -> TimeDelta {
    TimeDelta::milliseconds((seconds * 1000.0).round() as i64)
}

fn fresh() -> TaskbarQuerySchedule {
    TaskbarQuerySchedule::default()
}

fn settled(answer: Option<TaskbarAnchors>, asked_at: f64) -> TaskbarQuerySchedule {
    fresh()
        .asked(s(asked_at))
        .answered(answer)
        .asked(s(asked_at))
        .answered(answer)
}

#[test]
fn a_fresh_schedule_is_due() {
    assert!(fresh().due(s(0.0)));
}

#[test]
fn a_question_that_is_out_waits_for_the_slow_check() {
    let asked = fresh().asked(s(0.0));
    assert!(!asked.due(s(2.0)));
    assert!(asked.due(s(15.0)));
}

#[test]
fn one_answer_alone_asks_once_more() {
    assert!(fresh().asked(s(0.0)).answered(Some(A)).due(s(2.0)));
}

#[test]
fn two_answers_that_agree_settle_it() {
    assert!(!settled(Some(A), 0.0).due(s(2.0)));
}

#[test]
fn one_answer_alone_then_waits_for_the_next() {
    assert!(
        !fresh()
            .asked(s(0.0))
            .answered(Some(A))
            .asked(s(2.0))
            .due(s(4.0))
    );
}

#[test]
fn missing_answers_at_the_start_keep_asking() {
    let mut schedule = fresh();
    for i in 0..29 {
        schedule = schedule.asked(s(f64::from(i) * 2.0)).answered(None);
        assert!(schedule.due(s(f64::from(i) * 2.0 + 2.0)), "question {i}");
    }
}

#[test]
fn after_30_questions_without_anchors_it_settles() {
    let mut schedule = fresh();
    for i in 0..30 {
        schedule = schedule.asked(s(f64::from(i) * 2.0)).answered(None);
    }
    assert!(!schedule.due(s(29.0 * 2.0 + 2.0)));
    assert!(schedule.due(s(29.0 * 2.0 + 15.0)));
}

#[test]
fn missing_answers_agree_once_anchors_were_seen() {
    assert!(
        !settled(Some(A), 0.0)
            .asked(s(16.0))
            .answered(None)
            .asked(s(18.0))
            .answered(None)
            .due(s(20.0))
    );
}

#[test]
fn an_answer_that_differs_asks_again() {
    assert!(
        settled(Some(A), 0.0)
            .asked(s(16.0))
            .answered(Some(B))
            .due(s(18.0))
    );
}

#[test]
fn an_answer_going_missing_differs() {
    assert!(
        settled(Some(A), 0.0)
            .asked(s(16.0))
            .answered(None)
            .due(s(18.0))
    );
}

#[test]
fn answers_that_never_agree_keep_asking() {
    let mut schedule = fresh();
    for i in 0..50 {
        let answer = if i % 2 == 0 { A } else { B };
        schedule = schedule.asked(s(f64::from(i) * 2.0)).answered(Some(answer));
        assert!(schedule.due(s(f64::from(i) * 2.0 + 2.0)), "question {i}");
    }
}

#[test]
fn a_settled_schedule_checks_after_15_seconds() {
    for (later, due) in [(14.9, false), (15.0, true), (16.0, true), (3600.0, true)] {
        assert_eq!(
            settled(Some(A), 100.0).due(s(100.0 + later)),
            due,
            "later {later}"
        );
    }
}

#[test]
fn a_foreground_switch_asks_at_the_next_tick() {
    assert!(settled(Some(A), 0.0).foreground_switched().due(s(2.0)));
}

#[test]
fn many_switches_ask_twice() {
    let schedule = settled(Some(A), 0.0)
        .foreground_switched()
        .foreground_switched()
        .foreground_switched()
        .asked(s(2.0))
        .answered(Some(A));
    assert!(schedule.due(s(4.0)));
    assert!(!schedule.asked(s(4.0)).answered(Some(A)).due(s(6.0)));
}

#[test]
fn a_switch_while_a_question_is_out_asks_again_after_the_answer() {
    let mut schedule = settled(Some(A), 0.0)
        .asked(s(16.0))
        .foreground_switched()
        .answered(Some(A));
    assert!(schedule.due(s(18.0)));
    schedule = schedule.asked(s(18.0)).answered(Some(A));
    assert!(schedule.due(s(20.0)));
    assert!(!schedule.asked(s(20.0)).answered(Some(A)).due(s(22.0)));
}

#[test]
fn a_clock_that_went_back_is_not_due() {
    assert!(!settled(Some(A), 100.0).due(s(50.0)));
}

#[test]
fn a_lost_answer_is_asked_again_after_15_seconds() {
    let asked = settled(Some(A), 0.0).asked(s(16.0));
    assert!(!asked.due(s(18.0)));
    assert!(asked.due(s(31.0)));
}

#[test]
fn the_slow_check_is_15_seconds() {
    assert_eq!(SLOW_CHECK, s(15.0));
}

#[test]
fn a_clock_set_back_counts_the_slow_check_from_the_question_asked_after_it() {
    let asked = settled(Some(A), 100.0).asked(s(50.0));
    assert!(!asked.due(s(64.9)));
    assert!(asked.due(s(65.0)));
}

#[test]
fn spans_at_the_ends_do_not_overflow() {
    let asked_at_min = settled(Some(A), 0.0).asked(TimeDelta::MIN);
    assert!(asked_at_min.due(TimeDelta::MAX));
    let asked_at_max = settled(Some(A), 0.0).asked(TimeDelta::MAX);
    assert!(!asked_at_max.due(TimeDelta::MIN));
}

#[test]
fn a_step_gives_a_new_schedule_and_leaves_the_old_one() {
    let before = fresh();
    let after = before.asked(s(0.0));
    assert_eq!(before, TaskbarQuerySchedule::default());
    assert_ne!(before, after);
    assert!(before.due(s(1.0)));
}

#[test]
fn the_count_of_questions_stops_at_30() {
    // 100 questions with no anchors settle at the same point as 30 do.
    let mut schedule = fresh();
    for i in 0..100 {
        schedule = schedule.asked(s(f64::from(i) * 2.0)).answered(None);
    }
    assert!(!schedule.due(s(99.0 * 2.0 + 2.0)));
    assert!(schedule.due(s(99.0 * 2.0 + 15.0)));
}
