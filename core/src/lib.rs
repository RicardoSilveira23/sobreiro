//! Sobreiro's logic: providers, polling, limits, config and file formats. No UI, and no calls to the
//! system beyond files and HTTP.
//!
//! Each area of `src/Sobreiro.Core` is ported here with its tests, one at a time, and the folders
//! keep the names they have there. So far: the limits, `usage.json` and how readings are merged
//! into it, the way files are written, what the bridge does, where the files are,
//! `config.json`, Connect and Disconnect, the two requests Sobreiro makes (Claude Code's
//! token and the request made with it, a GitHub token and the request for Copilot's AI
//! credits, and the one way both reach the network), and polling: when a poll for Claude
//! Code's usage is due and what each outcome does to that, one poll, Claude Code as a
//! provider that keeps its readings in `usage.json`, and the timing of a provider's polls.
//! The tests are in `core/tests`.
//!
//! Core starts no thread, waits for no poll and looks at no clock. The shell tells it the
//! time, runs each poll it is handed away from the thread that draws, and hands the answer
//! back: `polling::provider_poller` says how. The one thing Core does wait for is a file that
//! another program holds: a moment, and then it tries again.

// No calls to the system means no unsafe code, and no file in here can allow it for itself.
#![forbid(unsafe_code)]
// Core never prints: what it knows goes back to the caller. This also keeps a token off the console.
#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod app_paths;
pub mod bridge;
pub mod colours;
pub mod config;
pub mod connect;
pub mod copilot;
pub mod display;
pub mod io;
pub mod limits;
pub mod live;
pub mod polling;
pub mod usage;
