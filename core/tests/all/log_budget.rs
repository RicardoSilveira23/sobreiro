//! Ported from `tests/Sobreiro.Tests/LogBudgetTests.cs`.
use std::sync::Arc;
use std::thread;

use sobreiro_core::diagnostics::log_budget::LogBudget;

#[test]
fn log_budget_lets_the_limit_through_then_stays_quiet() {
    let budget = LogBudget::new(3);
    assert_eq!(budget.next("e").as_deref(), Some("e"));
    assert_eq!(budget.next("e").as_deref(), Some("e"));
    assert_eq!(
        budget.next("e").as_deref(),
        Some("e; later ones not logged")
    );
    assert_eq!(budget.next("e"), None);
    assert_eq!(budget.next("e"), None);
}

// Not in the C# tests.

#[test]
fn a_budget_of_zero_logs_nothing() {
    let budget = LogBudget::new(0);
    assert_eq!(budget.next("e"), None);
    assert_eq!(budget.next("e"), None);
}

#[test]
fn threads_together_get_exactly_the_limit() {
    let budget = Arc::new(LogBudget::new(10));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let budget = Arc::clone(&budget);
            thread::spawn(move || (0..50).filter(|_| budget.next("e").is_some()).count())
        })
        .collect();
    let logged: usize = workers.into_iter().map(|w| w.join().unwrap()).sum();
    assert_eq!(logged, 10);
}
