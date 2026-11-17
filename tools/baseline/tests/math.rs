//! The median and the processor time per hour. Ported from 1.x's BaselineMathTests.

use sobreiro_baseline::{cpu_seconds_per_hour, median};
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn cpu_per_hour_scales_the_window_to_an_hour() {
    let got = cpu_seconds_per_hour(
        Duration::from_secs_f64(2.0),
        Duration::from_secs_f64(2.9),
        Duration::from_secs(9 * 60),
    );
    assert!(got.is_some_and(|v| (v - 6.0).abs() < 1e-6), "{got:?}");
}

#[test]
fn cpu_per_hour_of_an_idle_process_is_zero() {
    let got = cpu_seconds_per_hour(
        Duration::from_secs(2),
        Duration::from_secs(2),
        Duration::from_secs(9 * 60),
    );
    assert_eq!(got, Some(0.0));
}

#[test]
fn cpu_per_hour_needs_a_window() {
    let got = cpu_seconds_per_hour(Duration::ZERO, Duration::from_secs(1), Duration::ZERO);
    assert_eq!(got, None);
}

#[test]
fn median_of_an_odd_count_is_the_middle_one() {
    assert_eq!(median(&[ms(900), ms(20), ms(30)]), Some(ms(30)));
}

#[test]
fn median_of_an_even_count_is_between_the_middle_two() {
    assert_eq!(median(&[ms(40), ms(10), ms(20), ms(30)]), Some(ms(25)));
}

#[test]
fn median_of_nothing_is_nothing() {
    assert_eq!(median(&[]), None);
}
