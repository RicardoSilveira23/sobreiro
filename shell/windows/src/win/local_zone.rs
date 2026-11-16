//! The user's time zone as Core's `Zone`: the offset from UTC now, and each change of it in the
//! year before and the year after, found by asking Windows what the clock shows. Windows keeps
//! the rules (daylight saving, its dates); nothing of them is repeated here.
#![allow(unsafe_code)]

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeDelta, Timelike, Utc};
use sobreiro_core::display::zone::{Zone, ZoneChange};
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::System::Time::SystemTimeToTzSpecificLocalTime;

/// How far each way from now changes are looked for, in days.
const SPAN_DAYS: i64 = 370;

/// The zone as Windows has it set now. UTC when Windows will not say.
pub fn local_zone(now: DateTime<Utc>) -> Zone {
    build(offset_at, now)
}

/// The offset from UTC the clock has at `moment`, as Windows says it.
pub fn offset_at(moment: DateTime<Utc>) -> Option<FixedOffset> {
    let utc = system_time(moment)?;
    let mut local = SYSTEMTIME::default();
    // SAFETY: both times are alive for the call; no zone is passed, so the current one is used.
    unsafe { SystemTimeToTzSpecificLocalTime(None, &utc, &mut local) }.ok()?;
    let wall = NaiveDate::from_ymd_opt(
        i32::from(local.wYear),
        u32::from(local.wMonth),
        u32::from(local.wDay),
    )?
    .and_hms_opt(
        u32::from(local.wHour),
        u32::from(local.wMinute),
        u32::from(local.wSecond),
    )?;
    let seconds = wall.signed_duration_since(moment.naive_utc()).num_seconds();
    FixedOffset::east_opt(i32::try_from(seconds).ok()?)
}

fn system_time(moment: DateTime<Utc>) -> Option<SYSTEMTIME> {
    Some(SYSTEMTIME {
        wYear: u16::try_from(moment.year()).ok()?,
        wMonth: u16::try_from(moment.month()).ok()?,
        wDayOfWeek: 0,
        wDay: u16::try_from(moment.day()).ok()?,
        wHour: u16::try_from(moment.hour()).ok()?,
        wMinute: u16::try_from(moment.minute()).ok()?,
        wSecond: u16::try_from(moment.second()).ok()?,
        wMilliseconds: 0,
    })
}

/// A zone made by asking `offset_at` once a day, and then, where the answer changes, for each
/// second of that day until it is found to the second. UTC when it cannot answer.
fn build(offset_at: impl Fn(DateTime<Utc>) -> Option<FixedOffset>, now: DateTime<Utc>) -> Zone {
    // Whole seconds, so that the second a change falls on is found exactly.
    let now = now.with_nanosecond(0).unwrap_or(now);
    let start = now - TimeDelta::days(SPAN_DAYS);
    let Some(first) = offset_at(start) else {
        return Zone::utc();
    };
    let mut changes = Vec::new();
    let mut before = (start, first);
    for day in 1..=2 * SPAN_DAYS {
        let at = start + TimeDelta::days(day);
        let Some(offset) = offset_at(at) else {
            return Zone::utc();
        };
        if offset != before.1 {
            // `low` still has the old offset and `high` has the new one.
            let (mut low, mut high) = (before.0, at);
            while high - low > TimeDelta::seconds(1) {
                let middle = low + TimeDelta::seconds((high - low).num_seconds() / 2);
                match offset_at(middle) {
                    Some(found) if found == before.1 => low = middle,
                    Some(_) => high = middle,
                    None => return Zone::utc(),
                }
            }
            changes.push(ZoneChange { at: high, offset });
        }
        before = (at, offset);
    }
    Zone::new(first, changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .map(|time| time.with_timezone(&Utc))
            .unwrap_or_default()
    }

    /// A zone one hour east, and two hours east from 1 April 01:00 UTC to 1 October 01:00 UTC.
    fn summer_time(moment: DateTime<Utc>) -> Option<FixedOffset> {
        let summer = (5..=9).contains(&moment.month())
            || (moment.month() == 4 && (moment.day() > 1 || moment.hour() >= 1))
            || (moment.month() == 10 && moment.day() == 1 && moment.hour() < 1);
        FixedOffset::east_opt(if summer { 7200 } else { 3600 })
    }

    #[test]
    fn the_changes_are_found_to_the_second() {
        let zone = build(summer_time, time("2026-11-16T12:00:00Z"));
        assert_eq!(
            zone.offset_at(time("2026-11-16T12:00:00Z"))
                .local_minus_utc(),
            3600
        );
        assert_eq!(
            zone.offset_at(time("2026-07-01T00:00:00Z"))
                .local_minus_utc(),
            7200
        );
        assert_eq!(
            zone.offset_at(time("2027-04-01T00:59:59Z"))
                .local_minus_utc(),
            3600
        );
        assert_eq!(
            zone.offset_at(time("2027-04-01T01:00:00Z"))
                .local_minus_utc(),
            7200
        );
    }

    #[test]
    fn a_zone_that_cannot_be_asked_is_utc() {
        let zone = build(|_| None, time("2026-11-16T12:00:00Z"));
        assert_eq!(zone, Zone::utc());
    }
}
