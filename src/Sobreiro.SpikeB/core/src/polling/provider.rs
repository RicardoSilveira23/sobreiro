use super::ProviderState;
use crate::copilot::CopilotReading;
use crate::limits::Limit;
use crate::live::LiveSchedule;
use crate::usage::RateLimits;
use crate::{Time, polling::CancelToken};

/// What a finished poll hands to `Provider::keep`, on the shell's own thread. It holds the reading, never a token.
#[derive(Clone, Debug, PartialEq)]
pub enum Kept {
    /// Claude: the Live usage reading to merge into usage.json.
    Claude(RateLimits),
    /// Copilot: the poll's reading (if any), the cache file's reading (if any), and the log line.
    Copilot { reading: Option<CopilotReading>, cache: Option<CopilotReading>, log: String },
}

/// One poll's answer. `log` names outcomes only, never a token. `kept` is taken in, on the shell's thread, with the
/// time it is handled; a dropped poll's `kept` is never used.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderPoll {
    pub schedule: LiveSchedule,
    pub state: ProviderState,
    pub log: String,
    pub kept: Option<Kept>,
}

/// The blocking part of one poll: `Send + 'static`, so the shell can move it to a worker thread. It may check the
/// token between steps, and must not touch the provider.
pub type PollJob = Box<dyn FnOnce(&CancelToken) -> ProviderPoll + Send + 'static>;

/// A usage provider the tray polls: what one poll does, and the limits its kept polls bring. The provider itself
/// stays on the shell's thread and need not be `Send`.
pub trait Provider {
    /// The name its limits carry.
    fn name(&self) -> &str;

    /// Builds one poll (the counterpart of `PollAsync`): the returned job does the blocking work and is safe to run
    /// off the shell's thread. Called on the shell's thread.
    fn poll_job(&self, schedule: LiveSchedule, now: Time) -> PollJob;

    /// The limits to show now, from what the kept polls brought. Called on the shell's thread.
    fn limits(&mut self, now: Time) -> Vec<Limit>;

    /// Takes in a finished poll's `kept`, on the shell's thread, with the time it is handled.
    fn keep(&mut self, kept: Kept, now: Time);
}
