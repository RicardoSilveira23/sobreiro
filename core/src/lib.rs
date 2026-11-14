//! Sobreiro's logic: providers, polling, limits, config and file formats. No UI, and no calls to the
//! system beyond files and HTTP.
//!
//! Empty for now. Each area of `src/Sobreiro.Core` is ported here with its tests, one at a time.

// No calls to the system means no unsafe code, and no file in here can allow it for itself.
#![forbid(unsafe_code)]
// Core never prints: what it knows goes back to the caller. This also keeps a token off the console.
#![deny(clippy::print_stdout, clippy::print_stderr)]
