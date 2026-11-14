use crate::{Time, Zone};
use std::cell::Cell;
use windows::Win32::Foundation::SYSTEMTIME;
use windows::Win32::Globalization::{GetLocaleInfoEx, GetTimeFormatEx, LOCALE_SSHORTTIME, TIME_FORMAT_FLAGS};
use windows::core::PCWSTR;
use chrono::{Datelike, Timelike};

/// How a clock time is written. `Windows` is the user's own short time pattern (what a `CultureInfo` gave in C#);
/// the others are fixed, for tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clock {
    Windows,
    /// "9:00 AM", the en-US short pattern.
    English12,
    /// "14:20", the pt-PT / en-GB short pattern.
    Hours24,
}

thread_local! {
    static CLOCK: Cell<Clock> = const { Cell::new(Clock::Windows) };
}

pub struct When;

impl When {
    /// Sets the clock format for the calling thread only (tests set a fixed one).
    pub fn set_clock(clock: Clock) {
        CLOCK.with(|c| c.set(clock));
    }

    /// Clock time in the user's short time pattern. Another local day within six days gets its English day name;
    /// anything further out (or back) gets the day name, date and month.
    pub fn format(t: Time, now: Time, zone: Zone) -> String {
        let local = zone.convert(t);
        let local_now = zone.convert(now);
        let clock = clock_text(&local);
        let days = (local.date_naive() - local_now.date_naive()).num_days().abs();
        if days == 0 {
            return clock;
        }
        let date = if days <= 6 { "%a" } else { "%a %-d %b" };
        format!("{} {}", local.format(date), clock)
    }
}

fn clock_text(local: &Time) -> String {
    match CLOCK.with(|c| c.get()) {
        Clock::English12 => local.format("%-I:%M %p").to_string(),
        Clock::Hours24 => local.format("%H:%M").to_string(),
        // SPIKE: the user's short time pattern through the Windows API; 24-hour HH:mm only if that call fails.
        Clock::Windows => windows_short_time(local).unwrap_or_else(|| local.format("%H:%M").to_string()),
    }
}

/// The user's short time pattern (LOCALE_SSHORTTIME) applied to the time, through GetTimeFormatEx.
fn windows_short_time(local: &Time) -> Option<String> {
    let mut pattern = [0u16; 80];
    // SAFETY: a null locale name means the user's default locale; the buffer is ours.
    let n = unsafe { GetLocaleInfoEx(PCWSTR::null(), LOCALE_SSHORTTIME, Some(&mut pattern)) };
    if n <= 0 {
        return None;
    }
    let st = SYSTEMTIME {
        wYear: local.year() as u16,
        wMonth: local.month() as u16,
        wDayOfWeek: local.weekday().num_days_from_sunday() as u16,
        wDay: local.day() as u16,
        wHour: local.hour() as u16,
        wMinute: local.minute() as u16,
        wSecond: local.second() as u16,
        wMilliseconds: 0,
    };
    let mut out = [0u16; 80];
    // SAFETY: pattern is NUL-terminated by GetLocaleInfoEx; st and out are live locals.
    let written = unsafe {
        GetTimeFormatEx(PCWSTR::null(), TIME_FORMAT_FLAGS(0), Some(&st), PCWSTR(pattern.as_ptr()), Some(&mut out))
    };
    if written <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&out[..(written - 1) as usize]))
}
