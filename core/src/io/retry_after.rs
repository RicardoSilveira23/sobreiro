//! How long a server asks to be left alone: the `Retry-After` header of an answer.
//!
//! The header holds either a count of seconds or a date. What counts as either is what 1.x's
//! framework took, held row by row in the tests.
use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};

/// The longest wait that is taken from a server.
const MAX_RETRY_AFTER: TimeDelta = TimeDelta::hours(6);

/// The wait a `Retry-After` header asks for, counted from `now`: never under zero and never
/// over six hours. `None` without the header, and for one that is neither seconds nor a date.
pub fn retry_after(value: Option<&str>, now: DateTime<Utc>) -> Option<TimeDelta> {
    let value = value?.trim_matches([' ', '\t']);
    let wait = if value.starts_with(|first: char| first.is_ascii_digit()) {
        seconds(value)?
    } else {
        http_date(value)? - now
    };
    Some(wait.clamp(TimeDelta::zero(), MAX_RETRY_AFTER))
}

/// A count of seconds: digits only, ten at most, and no more than the largest number 1.x
/// read here (2147483647, some 68 years).
fn seconds(text: &str) -> Option<TimeDelta> {
    if text.len() > 10 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let seconds: i32 = text.parse().ok()?;
    Some(TimeDelta::seconds(i64::from(seconds)))
}

/// The days of the week, short and long, from Monday as `chrono` counts them.
const DAYS: [(&str, &str); 7] = [
    ("Mon", "Monday"),
    ("Tue", "Tuesday"),
    ("Wed", "Wednesday"),
    ("Thu", "Thursday"),
    ("Fri", "Friday"),
    ("Sat", "Saturday"),
    ("Sun", "Sunday"),
];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A date as HTTP writes one. There are three ways, and all three start with the day of the
/// week:
///
/// ```text
/// Mon, 09 Nov 2026 12:03:00 GMT      today's way
/// Monday, 09-Nov-26 12:03:00 GMT     an old way, with a year of two digits
/// Mon Nov  9 12:03:00 2026           an older way, with no time zone
/// ```
///
/// As in 1.x, names may be in capitals or not, spaces between the pieces may be many or none,
/// a day or an hour may have one digit, and in the first two ways `GMT` may be `UTC`, a
/// distance from UTC (`+0100`, `+01:00`) or missing. The day of the week must be the date's
/// own.
fn http_date(text: &str) -> Option<DateTime<Utc>> {
    let mut pieces = Pieces { rest: text };
    let (weekday, long_name) = pieces.day_name()?;

    let (year, month, day, time, ahead_of_utc);
    if pieces.sign(",") {
        day = pieces.number(2)?.0;
        if long_name {
            pieces.sign("-").then_some(())?;
            month = pieces.month()?;
            pieces.sign("-").then_some(())?;
            (year, time, ahead_of_utc) = pieces.year_time_zone(2, true)?;
        } else {
            month = pieces.month()?;
            // A year of four digits or, failing that, of two. After a year of two digits no
            // distance from UTC is taken.
            (year, time, ahead_of_utc) = pieces
                .year_time_zone(4, true)
                .or_else(|| pieces.year_time_zone(2, false))?;
        }
    } else if !long_name {
        month = pieces.month()?;
        day = pieces.number(2)?.0;
        time = pieces.time()?;
        year = pieces.year(4)?;
        ahead_of_utc = 0;
        pieces.end()?;
    } else {
        return None;
    }

    // The year 0 is no year. A 30th of February, an hour 24 and a second 60 are refused by
    // `chrono`.
    if year == 0 {
        return None;
    }
    let (hour, minute, second) = time;
    let date = NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)?;
    if date.weekday().num_days_from_monday() != weekday {
        return None;
    }
    let time = date
        .and_hms_opt(hour, minute, second)?
        .checked_sub_signed(TimeDelta::minutes(ahead_of_utc))?
        .and_utc();
    // 1.x held no time before the year 1 or after the year 9999.
    (FIRST_TIME..=LAST_TIME)
        .contains(&time.timestamp())
        .then_some(time)
}

/// The first and the last second 1.x could hold: the start of the year 1 and the end of the
/// year 9999.
const FIRST_TIME: i64 = -62_135_596_800;
const LAST_TIME: i64 = 253_402_300_799;

/// A year written with two digits: 00 to 49 are 2000 to 2049, and 50 to 99 are 1950 to 1999.
fn century_of(two_digits: u32) -> u32 {
    if two_digits < 50 {
        2000 + two_digits
    } else {
        1900 + two_digits
    }
}

/// What is left of a date while it is read, piece by piece. Each piece may have spaces in
/// front of it.
#[derive(Clone, Copy)]
struct Pieces<'a> {
    rest: &'a str,
}

impl Pieces<'_> {
    fn spaces(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// Moves past `word` when the text goes on with it. With `any_case`, capitals and small
    /// letters count as the same.
    fn word(&mut self, word: &str, any_case: bool) -> bool {
        let Some(start) = self.rest.get(..word.len()) else {
            return false;
        };
        let same = if any_case {
            start.eq_ignore_ascii_case(word)
        } else {
            start == word
        };
        if same {
            self.rest = self.rest.get(word.len()..).unwrap_or_default();
        }
        same
    }

    /// Moves past a comma, a dash or a colon.
    fn sign(&mut self, sign: &str) -> bool {
        self.spaces();
        self.word(sign, false)
    }

    /// The day of the week, counted from Monday as 0, and whether its name was written out
    /// in full.
    fn day_name(&mut self) -> Option<(u32, bool)> {
        self.spaces();
        for (index, (short, long)) in (0..).zip(DAYS) {
            if self.word(long, true) {
                return Some((index, true));
            }
            if self.word(short, true) {
                return Some((index, false));
            }
        }
        None
    }

    /// The month, from 1.
    fn month(&mut self) -> Option<u32> {
        self.spaces();
        (1..)
            .zip(MONTHS)
            .find_map(|(number, name)| self.word(name, true).then_some(number))
    }

    /// Up to `at_most` digits with nothing in front of them: the number they spell, and how
    /// many they were. `None` when the text does not go on with a digit.
    fn digits(&mut self, at_most: usize) -> Option<(u32, usize)> {
        let count = self
            .rest
            .bytes()
            .take(at_most)
            .take_while(u8::is_ascii_digit)
            .count();
        if count == 0 {
            return None;
        }
        let number = self.rest.get(..count)?.parse().ok()?;
        self.rest = self.rest.get(count..)?;
        Some((number, count))
    }

    /// As [`Pieces::digits`], after any spaces.
    fn number(&mut self, at_most: usize) -> Option<(u32, usize)> {
        self.spaces();
        self.digits(at_most)
    }

    /// A year written with exactly `digits` digits, two or four.
    fn year(&mut self, digits: usize) -> Option<u32> {
        match self.number(digits)? {
            (number, count) if count == digits && digits == 2 => Some(century_of(number)),
            (number, count) if count == digits => Some(number),
            _ => None,
        }
    }

    /// What ends the first two ways of writing a date: the year, the time and what may
    /// follow it, with nothing after that. `self` is a copy, so a try that fails leaves the
    /// caller where it was, free to try another way.
    fn year_time_zone(
        mut self,
        year_digits: usize,
        distance_allowed: bool,
    ) -> Option<(u32, (u32, u32, u32), i64)> {
        let year = self.year(year_digits)?;
        let time = self.time()?;
        let ahead_of_utc = self.zone(distance_allowed)?;
        self.end()?;
        Some((year, time, ahead_of_utc))
    }

    /// `Some` when nothing but spaces is left.
    fn end(&mut self) -> Option<()> {
        self.spaces();
        self.rest.is_empty().then_some(())
    }

    /// Hours, minutes and seconds with colons between them.
    fn time(&mut self) -> Option<(u32, u32, u32)> {
        let hour = self.number(2)?.0;
        self.sign(":").then_some(())?;
        let minute = self.number(2)?.0;
        self.sign(":").then_some(())?;
        let second = self.number(2)?.0;
        Some((hour, minute, second))
    }

    /// What may follow the time, as the minutes its clock is ahead of UTC: `GMT` or `UTC`,
    /// a distance from UTC when `distance_allowed`, or nothing, which is taken for UTC.
    fn zone(&mut self, distance_allowed: bool) -> Option<i64> {
        self.spaces();
        if self.word("GMT", false) || self.word("UTC", false) {
            return Some(0);
        }
        if !distance_allowed {
            return Some(0);
        }
        let ahead = if self.word("+", false) {
            true
        } else if self.word("-", false) {
            false
        } else {
            return Some(0);
        };
        // One or two digits of hours, a colon or not, and two digits of minutes, with no
        // space anywhere.
        let hours = self.digits(2)?.0;
        self.word(":", false);
        let minutes = match self.digits(2)? {
            (number, 2) => number,
            _ => return None,
        };
        // No clock is more than fourteen hours from UTC.
        if minutes > 59 || hours * 60 + minutes > 14 * 60 {
            return None;
        }
        let distance = i64::from(hours * 60 + minutes);
        Some(if ahead { distance } else { -distance })
    }
}
