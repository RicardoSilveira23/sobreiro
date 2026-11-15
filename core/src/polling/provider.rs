//! A provider of usage, as the app polls it: what one poll does, and the limits that the
//! polls it kept bring.
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use chrono::{DateTime, Utc};

use super::ProviderState;
use crate::limits::Limit;
use crate::live::live_schedule::LiveSchedule;
use crate::live::live_usage_client::Cancelled;

/// Takes a poll's reading in. It is called with the time of the call, once, on the thread
/// that handles the answer.
///
/// `Box<dyn FnOnce(..)>` is a function made while the program runs, which carries what it
/// needs with it (here the reading and where to put it) and can be called one time. `Send`
/// says it may be made on one thread and called on another.
pub type Keep = Box<dyn FnOnce(DateTime<Utc>) + Send>;

/// Something that is told a line for the debug log. A line names what happened and never
/// holds a token.
pub type Trace = Arc<dyn Fn(&str) + Send + Sync>;

/// One poll's answer.
pub struct ProviderPoll {
    /// When the next poll is due.
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    /// A line for the debug log. It names the outcome and never holds a token.
    pub log: String,
    /// Takes the reading in, when the poll brought one. When the poll's answer is dropped,
    /// this is never called.
    pub keep: Option<Keep>,
}

/// A provider of usage that the app polls.
///
/// `Send + Sync` says a provider may be shared between threads: a poll runs away from the
/// thread that draws the window, while that thread asks for the limits.
pub trait Provider: Send + Sync {
    /// The name its limits carry.
    fn name(&self) -> &str;

    /// One poll. It may wait for an answer from the network, so it is run away from the
    /// thread that draws. `cancelled` is a flag the caller may set meanwhile to give the poll
    /// up; `Err(Cancelled)` comes back for that and for nothing else.
    fn poll(
        &self,
        schedule: &LiveSchedule,
        now: DateTime<Utc>,
        cancelled: &AtomicBool,
    ) -> Result<ProviderPoll, Cancelled>;

    /// The limits to show now, from what the kept polls brought.
    fn limits(&self, now: DateTime<Utc>) -> Vec<Limit>;
}
