//! Measures time that falls on Monday to Friday in a zone, midnight to midnight.
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, TimeDelta, Utc, Weekday};

use super::Zone;

/// The time between the two moments that falls on a weekday in `zone`; zero when `to` is not
/// after `from`.
pub fn between(from: DateTime<Utc>, to: DateTime<Utc>, zone: &Zone) -> TimeDelta {
    let mut total = TimeDelta::zero();
    if to <= from {
        return total;
    }
    let mut date = zone.local(from).date();
    while let Some(start) = midnight(date, zone) {
        if start >= to {
            break;
        }
        let Some(next) = date.succ_opt() else {
            break;
        };
        if is_weekday(date)
            && let Some(end) = midnight(next, zone)
        {
            let span = end.min(to) - start.max(from);
            if span > TimeDelta::zero() {
                total += span;
            }
        }
        date = next;
    }
    total
}

/// The moment at which `working` weekday time has passed after `from`; `from` itself when it
/// is not positive.
pub fn after(from: DateTime<Utc>, working: TimeDelta, zone: &Zone) -> DateTime<Utc> {
    if working <= TimeDelta::zero() {
        return from;
    }
    // Weekday time cannot be longer than all the time up to the last moment chrono holds.
    let to_the_end = DateTime::<Utc>::MAX_UTC.signed_duration_since(from);
    if working > to_the_end {
        return DateTime::<Utc>::MAX_UTC;
    }
    let mut remaining = working;
    let mut date = zone.local(from).date();
    loop {
        let Some(next) = date.succ_opt() else {
            return DateTime::<Utc>::MAX_UTC;
        };
        if is_weekday(date)
            && let (Some(begins), Some(ends)) = (midnight(date, zone), midnight(next, zone))
        {
            let start = begins.max(from);
            let available = ends - start;
            if available > TimeDelta::zero() {
                if remaining <= available {
                    return start
                        .checked_add_signed(remaining)
                        .unwrap_or(DateTime::<Utc>::MAX_UTC);
                }
                remaining -= available;
            }
        }
        date = next;
    }
}

fn is_weekday(date: NaiveDate) -> bool {
    !matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
}

/// The moment a local day begins. A midnight the clock skips begins at the first valid time
/// after it, found in steps of half an hour.
fn midnight(date: NaiveDate, zone: &Zone) -> Option<DateTime<Utc>> {
    let mut local = date.and_time(NaiveTime::MIN);
    loop {
        if let Some(moment) = zone.moment(local) {
            return Some(moment);
        }
        local = local.checked_add_signed(TimeDelta::minutes(30))?;
    }
}
