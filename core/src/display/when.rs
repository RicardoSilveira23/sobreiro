//! A moment as the clock on the wall shows it, for the reset times.
use chrono::{DateTime, Utc};

use super::{ClockFormat, Zone};

/// The clock time in the user's pattern. Another local day within six days gets its English
/// day name before it, for example `Sat 12:00 AM`; anything further out, or back, gets the day
/// name, the date and the month, for example `Mon 16 Nov 12:00 AM`. Both moments are read on the
/// wall of `zone`.
pub fn format(t: DateTime<Utc>, now: DateTime<Utc>, zone: &Zone, clock: &ClockFormat) -> String {
    let local = zone.local(t);
    let local_now = zone.local(now);
    let time = clock.format(local.time());
    let days = (local.date() - local_now.date()).num_days().abs();
    if days == 0 {
        return time;
    }
    let date = if days <= 6 { "%a" } else { "%a %-d %b" };
    format!("{} {time}", local.format(date))
}
