//! Polling: what a provider of usage is to the app, and the timing of its polls. Nothing in
//! here starts a thread, waits or looks at a clock.
pub mod provider;
pub mod provider_poller;
pub mod provider_state;

pub use provider::{Keep, Provider, ProviderPoll, Trace};
pub use provider_state::ProviderState;
