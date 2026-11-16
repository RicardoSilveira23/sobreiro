//! The time on the wall clock, for the polls and the log. `chrono` is built here without its own
//! clock, so it is read from the standard library.
use chrono::{DateTime, Utc};
use std::time::SystemTime;

pub fn now() -> DateTime<Utc> {
    DateTime::<Utc>::from(SystemTime::now())
}
