//! Polling: the timing of one provider's polls, with the blocking work left to the shell's worker thread.

mod provider;
mod provider_poller;
mod provider_state;

pub use provider::{Kept, PollJob, Provider, ProviderPoll};
pub use provider_poller::{CancelToken, PollDone, PollRun, ProviderPoller};
pub use provider_state::ProviderState;
