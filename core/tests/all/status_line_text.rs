//! Ported from `tests/Sobreiro.Tests/StatusLineTextTests.cs`.
use sobreiro_core::display::status_line_text;
use sobreiro_core::usage::RateLimits;

use crate::{spend, window};

#[test]
fn both_windows() {
    let limits = RateLimits {
        five_hour: Some(window(8.4, None)),
        seven_day: Some(window(44.0, None)),
        spend: None,
    };
    assert_eq!(status_line_text::render(Some(&limits)), "5h 8% · 7d 44%");
}

#[test]
fn only_weekly() {
    let limits = RateLimits {
        five_hour: None,
        seven_day: Some(window(44.0, None)),
        spend: None,
    };
    assert_eq!(status_line_text::render(Some(&limits)), "5h – · 7d 44%");
}

#[test]
fn null_limits() {
    assert_eq!(status_line_text::render(None), "5h – · 7d –");
}

#[test]
fn with_spend() {
    let limits = RateLimits {
        five_hour: Some(window(8.4, None)),
        seven_day: Some(window(44.0, None)),
        spend: Some(spend(62.8, None, None, None, None)),
    };
    assert_eq!(
        status_line_text::render(Some(&limits)),
        "5h 8% · 7d 44% · spend 62%"
    );
}
