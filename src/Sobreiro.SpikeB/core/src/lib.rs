//! SPIKE (shell candidate B): the part of Sobreiro.Core the shell needs, in Rust.
//! One module per C# folder. Times are `Time` (a `DateTime<FixedOffset>`, like a `DateTimeOffset`).

pub mod app_paths;
pub mod colours;
pub mod config;
pub mod display;
pub mod live;
pub mod polling;
pub mod secret;
pub mod taskbar;
pub mod theme;
pub mod connect;
pub mod copilot;
mod http;
pub mod io;
pub mod limits;
pub mod usage;
pub mod zone;

/// A moment with an offset: the counterpart of `DateTimeOffset`.
pub type Time = chrono::DateTime<chrono::FixedOffset>;

/// A length of time: the counterpart of `TimeSpan`.
pub type Span = chrono::TimeDelta;

pub use zone::Zone;

/// `DateTimeOffset.UtcNow`-style helpers.
pub fn now() -> Time {
    chrono::Local::now().fixed_offset()
}

pub fn from_unix_seconds(s: i64) -> Option<Time> {
    chrono::DateTime::from_timestamp(s, 0).map(|t| t.fixed_offset())
}

pub fn from_unix_millis(ms: i64) -> Option<Time> {
    chrono::DateTime::from_timestamp_millis(ms).map(|t| t.fixed_offset())
}


#[cfg(test)]
mod internal_tests;
