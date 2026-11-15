//! A span of time in a few words.
use chrono::TimeDelta;

/// A span as `3d 13h`, `3h 11m`, `45m` or `under 1m`. Seconds are cut off, and the smaller unit
/// is left out when it is zero.
pub fn format(span: TimeDelta) -> String {
    let minutes = span.num_minutes();
    if span <= TimeDelta::zero() || minutes < 1 {
        return "under 1m".to_string();
    }
    let days = minutes / 1440;
    let hours = minutes % 1440 / 60;
    let minutes = minutes % 60;
    if days > 0 {
        if hours == 0 {
            format!("{days}d")
        } else {
            format!("{days}d {hours}h")
        }
    } else if hours > 0 {
        if minutes == 0 {
            format!("{hours}h")
        } else {
            format!("{hours}h {minutes}m")
        }
    } else {
        format!("{minutes}m")
    }
}
