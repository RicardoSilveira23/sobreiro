//! Reads the reply of `GET /api/oauth/usage`.
//!
//! The reply's `limits` list comes first: its entries of the group `session` or `weekly` that
//! name no `scope`. A window the list does not give comes from the keyed objects `five_hour`
//! and `seven_day`. Anything else in the reply is ignored. The windows come back without the
//! time they were read at; merging them into `usage.json` stamps it.
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};

use crate::connect::json_reader::{self, Dialect, Kind, Member};
use crate::usage::{LimitWindow, RateLimits};

/// The limits in a reply. `None` when it is not JSON or holds neither window.
pub fn parse(json: &str) -> Option<RateLimits> {
    let root = json_reader::read(json, Dialect::Strict).ok()?;
    let Kind::Object(root) = &root.kind else {
        return None;
    };

    let (mut five_hour, mut seven_day) = from_list(root).ok()?;
    if five_hour.is_none() || seven_day.is_none() {
        let keyed_five_hour = keyed(root, "five_hour").ok()?;
        let keyed_seven_day = keyed(root, "seven_day").ok()?;
        five_hour = five_hour.or(keyed_five_hour);
        seven_day = seven_day.or(keyed_seven_day);
    }
    if five_hour.is_none() && seven_day.is_none() {
        return None;
    }
    Some(RateLimits {
        five_hour,
        seven_day,
        spend: None,
    })
}

/// A text in the reply spells out half a character (`\ud800` with no second half after it),
/// which no text can hold. 1.x stopped with an error where it had to read such a text, and the
/// request counted as failed. Here the whole reply is of no use, which comes to the same.
pub(crate) struct HalfACharacter;

/// The session and the weekly window, in that order, as the `limits` list gives them.
type Windows = (Option<LimitWindow>, Option<LimitWindow>);

fn from_list(root: &[Member]) -> Result<Windows, HalfACharacter> {
    let (mut five_hour, mut seven_day) = (None, None);
    let Some(Kind::List(list)) = json_reader::member(root, "limits") else {
        return Ok((None, None));
    };

    for entry in list {
        let Kind::Object(entry) = &entry.kind else {
            continue;
        };
        // An entry with a scope is the limit of one model, not of the whole account.
        if !matches!(json_reader::member(entry, "scope"), None | Some(Kind::Null)) {
            continue;
        }
        let Some(Kind::Text(group)) = json_reader::member(entry, "group") else {
            continue;
        };
        let Some(window) = window(entry, "percent")? else {
            continue;
        };
        // The first entry of each group counts; a later one of the same group is ignored.
        match text_of(group)?.as_str() {
            "session" => five_hour = five_hour.or(Some(window)),
            "weekly" => seven_day = seven_day.or(Some(window)),
            _ => {}
        }
    }
    Ok((five_hour, seven_day))
}

/// The window under `key` at the top of the reply, when it is an object with a percentage.
fn keyed(root: &[Member], key: &str) -> Result<Option<LimitWindow>, HalfACharacter> {
    match json_reader::member(root, key) {
        Some(Kind::Object(value)) => window(value, "utilization"),
        _ => Ok(None),
    }
}

/// A window out of an object: the number under `percent_key`, and the time under `resets_at`
/// when it is a text that reads as a time. `None` without the number.
fn window(object: &[Member], percent_key: &str) -> Result<Option<LimitWindow>, HalfACharacter> {
    let Some(Kind::Number(percent)) = json_reader::member(object, percent_key) else {
        return Ok(None);
    };
    // The reader has checked that this is a JSON number, and Rust reads every one of those: a
    // number too large to hold becomes infinity, as it did in 1.x.
    let Ok(used_percentage) = percent.parse::<f64>() else {
        return Ok(None);
    };

    let resets_at = match json_reader::member(object, "resets_at") {
        Some(Kind::Text(reset)) => time_in(reset)?,
        _ => None,
    };
    Ok(Some(LimitWindow {
        used_percentage,
        resets_at,
        observed_at: None,
    }))
}

fn text_of(raw: &str) -> Result<String, HalfACharacter> {
    json_reader::text_of(raw).ok_or(HalfACharacter)
}

/// The shortest and the longest a time may be as the reply writes it, between its quotes,
/// with every `A` counted as six. 1.x did not look inside a text outside these lengths.
const SHORTEST_TIME: usize = 10;
const LONGEST_TIME_AS_WRITTEN: usize = 252;

/// The time a text of the reply names. `raw` is the text as written, with its quotes.
/// Copilot's reply is read with this too, so the two replies' times are read alike, as they
/// were in 1.x.
pub(crate) fn time_in(raw: &str) -> Result<Option<DateTime<Utc>>, HalfACharacter> {
    let written = raw.len().saturating_sub(2);
    if !(SHORTEST_TIME..=LONGEST_TIME_AS_WRITTEN).contains(&written) {
        return Ok(None);
    }
    Ok(time_of(&text_of(raw)?))
}

/// Reads a time the way 1.x read one out of JSON:
///
/// ```text
/// 2026-11-09T14:00:00Z
/// 2026-11-09T14:00:00.123456+00:00
/// 2026-11-09T14:00-08
/// ```
///
/// A date, a `T`, hours and minutes; then seconds or not, and after seconds a point and up to
/// sixteen digits or not; then `Z`, or a `+` or `-` with hours and maybe minutes. Every part
/// has exactly as many digits as shown. Digits after the seventh of a fraction are dropped:
/// 1.x counted time in ten-millionths of a second.
///
/// `None` for anything else, and for a time that does not say how far it is from UTC. 1.x
/// took such a time to be on the PC's own clock; Core does not know the PC's time zone, and
/// the usage address has never left the distance out.
fn time_of(text: &str) -> Option<DateTime<Utc>> {
    let bytes = text.as_bytes();
    let year = digits(bytes, 0, 4)?;
    let month = digits(bytes, 5, 2)?;
    let day = digits(bytes, 8, 2)?;
    let hour = digits(bytes, 11, 2)?;
    let minute = digits(bytes, 14, 2)?;
    if at(bytes, 4) != Some(b'-')
        || at(bytes, 7) != Some(b'-')
        || at(bytes, 10) != Some(b'T')
        || at(bytes, 13) != Some(b':')
    {
        return None;
    }

    // What has been read ends at `next`.
    let mut next = 16;
    let mut second = 0;
    let mut ten_millionths = 0;
    if at(bytes, next) == Some(b':') {
        second = digits(bytes, 17, 2)?;
        next = 19;
        if at(bytes, next) == Some(b'.') {
            next = 20;
            let mut read = 0;
            while read < 16 && at(bytes, next).is_some_and(|byte| byte.is_ascii_digit()) {
                if read < 7 {
                    ten_millionths = ten_millionths * 10 + u32::from(at(bytes, next)? - b'0');
                }
                read += 1;
                next += 1;
            }
            // `.5` is five tenths: the digits that were not written are zeros.
            for _ in read..7 {
                ten_millionths *= 10;
            }
        }
    }

    // How far the clock that wrote the time is ahead of UTC, in minutes.
    let ahead_of_utc = match at(bytes, next)? {
        b'Z' if bytes.len() == next + 1 => 0,
        sign @ (b'+' | b'-') => {
            let hours = digits(bytes, next + 1, 2)?;
            let minutes = match bytes.len() - (next + 1) {
                2 => 0,
                5 if at(bytes, next + 3) == Some(b':') => digits(bytes, next + 4, 2)?,
                _ => return None,
            };
            // No clock is more than fourteen hours from UTC.
            if minutes > 59 || hours * 60 + minutes > 14 * 60 {
                return None;
            }
            let distance = i64::from(hours * 60 + minutes);
            if sign == b'+' { distance } else { -distance }
        }
        _ => return None,
    };

    // The year 0 is no year. The other checks (a thirteenth month, a 30th of February, an hour
    // 24, a second 60) are the ones `chrono` makes.
    if year == 0 {
        return None;
    }
    let on_its_clock = NaiveDate::from_ymd_opt(i32::try_from(year).ok()?, month, day)?
        .and_hms_nano_opt(hour, minute, second, ten_millionths * 100)?;
    let time = on_its_clock
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

/// The byte at `index`, or `None` past the end.
fn at(bytes: &[u8], index: usize) -> Option<u8> {
    bytes.get(index).copied()
}

/// The number that `count` digits starting at `start` spell. `None` when one of them is
/// missing or is not a digit.
fn digits(bytes: &[u8], start: usize, count: usize) -> Option<u32> {
    let mut number = 0;
    for byte in bytes.get(start..start + count)? {
        if !byte.is_ascii_digit() {
            return None;
        }
        number = number * 10 + u32::from(byte - b'0');
    }
    Some(number)
}
