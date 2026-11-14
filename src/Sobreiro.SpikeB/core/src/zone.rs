use crate::Time;
use chrono::{FixedOffset, Local, NaiveDate, TimeZone};

/// The counterpart of the `TimeZoneInfo` parameters: the PC's local zone, or a fixed offset (UTC in tests).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zone {
    Local,
    Fixed(FixedOffset),
}

impl Zone {
    pub fn utc() -> Zone {
        Zone::Fixed(FixedOffset::east_opt(0).unwrap())
    }

    /// `TimeZoneInfo.ConvertTime(t, zone)`.
    pub fn convert(&self, t: Time) -> Time {
        match self {
            Zone::Local => t.with_timezone(&Local).fixed_offset(),
            Zone::Fixed(o) => t.with_timezone(o),
        }
    }

    /// The moment a local day begins. A midnight the clock skips begins at the first valid time after it.
    pub fn midnight(&self, date: NaiveDate) -> Time {
        let mut local = date.and_hms_opt(0, 0, 0).unwrap();
        loop {
            let found = match self {
                Zone::Local => Local.from_local_datetime(&local).latest().map(|t| t.fixed_offset()),
                Zone::Fixed(o) => o.from_local_datetime(&local).latest(),
            };
            if let Some(t) = found {
                return t;
            }
            local += chrono::TimeDelta::minutes(30);
        }
    }
}
