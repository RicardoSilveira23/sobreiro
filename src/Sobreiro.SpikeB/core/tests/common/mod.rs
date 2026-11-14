#![allow(dead_code)]
use chrono::DateTime;
use sobreiro_core::usage::LimitWindow;
use sobreiro_core::{Span, Time};

pub fn t(s: &str) -> Time {
    DateTime::parse_from_rfc3339(s).unwrap()
}

pub fn unix(s: i64) -> Time {
    sobreiro_core::from_unix_seconds(s).unwrap()
}

pub fn mins(m: i64) -> Span {
    Span::minutes(m)
}

pub fn hours(h: i64) -> Span {
    Span::hours(h)
}

pub fn days(d: i64) -> Span {
    Span::days(d)
}

pub fn win(pct: f64, resets: Option<Time>) -> LimitWindow {
    LimitWindow::new(pct, resets)
}

pub fn win_at(pct: f64, resets: Option<Time>, at: Time) -> LimitWindow {
    LimitWindow { used_percentage: pct, resets_at: resets, observed_at: Some(at) }
}

/// A scratch folder that is removed when dropped.
pub struct Scratch(pub tempfile::TempDir);

impl Scratch {
    pub fn new() -> Self {
        Scratch(tempfile::Builder::new().prefix("sobreiro-tests-").tempdir().unwrap())
    }
    pub fn path(&self, rel: &str) -> String {
        self.0.path().join(rel).to_string_lossy().into_owned()
    }
    pub fn dir(&self) -> String {
        self.0.path().to_string_lossy().into_owned()
    }
}

use sobreiro_core::limits::{Limit, LimitKind};

/// `new Limit(provider, kind, label, pct, resetsAt, observedAt, isReset, window)`.
#[allow(clippy::too_many_arguments)]
pub fn limit(provider: &str, kind: LimitKind, label: &str, pct: f64, resets: Option<Time>, observed: Time, is_reset: bool, window: Option<Span>) -> Limit {
    Limit {
        provider: provider.into(), kind, label: label.into(), percent: pct, resets_at: resets, observed_at: observed,
        is_reset, window, used_usd: None, limit_usd: None, period: None, used_credits: None, credit_budget: None,
    }
}

/// A Claude limit with no reset time (the C# tests' `L(kind, pct)`).
pub fn l(kind: LimitKind, pct: f64, observed: Time) -> Limit {
    limit("Claude Code", kind, &format!("{kind:?}"), pct, None, observed, false, None)
}
