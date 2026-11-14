use crate::{Span, Time, Zone};
use chrono::{Datelike, NaiveDate, Weekday};

/// Measures time that falls on Monday to Friday in a zone, midnight to midnight.
pub struct WorkingTime;

impl WorkingTime {
    /// The time between the two moments that falls on a weekday in `zone`; zero when `to` is not after `from`.
    pub fn between(from: Time, to: Time, zone: Zone) -> Span {
        let mut total = Span::zero();
        if to <= from {
            return total;
        }

        let mut date = zone.convert(from).date_naive();
        loop {
            let start = zone.midnight(date);
            if start >= to {
                break;
            }
            if is_weekday(date) {
                let end = zone.midnight(date.succ_opt().unwrap());
                let span = end.min(to) - start.max(from);
                if span > Span::zero() {
                    total += span;
                }
            }
            date = date.succ_opt().unwrap();
        }
        total
    }

    /// The moment at which `working` weekday time has passed after `from`; `from` itself when it is not positive.
    pub fn after(from: Time, working: Span, zone: Zone) -> Time {
        if working <= Span::zero() {
            return from;
        }

        let mut remaining = working;
        let mut date = zone.convert(from).date_naive();
        loop {
            if is_weekday(date) {
                let start = zone.midnight(date).max(from);
                let available = zone.midnight(date.succ_opt().unwrap()) - start;
                if available > Span::zero() {
                    if remaining <= available {
                        return start + remaining;
                    }
                    remaining -= available;
                }
            }
            date = date.succ_opt().unwrap();
        }
    }
}

fn is_weekday(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}
