//! The bridge's whole flow. It always prints something and never fails.
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::bridge::BridgeConfig;
use crate::display::status_line_text;
use crate::usage::usage_merge::{self, ReadingFrom};
use crate::usage::{rate_limits_json, usage_file};

/// What the user's previous status line command gave back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainResult {
    pub exit_code: i32,
    /// Everything it printed, to be printed again as it is.
    pub output: String,
}

pub struct BridgeRunner<'a> {
    /// `usage.json`.
    pub usage_path: &'a Path,
    /// `config.json`, for the user's previous status line command.
    pub config_path: &'a Path,
    /// Runs that command with the same stdin: `run_chain(command, stdin)`. `None` when it
    /// cannot be started or does not finish in time.
    pub run_chain: &'a dyn Fn(&str, &str) -> Option<ChainResult>,
    /// This bridge was started by a chained command, so it must not chain again.
    pub inside_chain: bool,
}

impl BridgeRunner<'_> {
    /// Takes what Claude Code gave on stdin, keeps its limits, and prints the status line to
    /// `stdout`. Gives the exit code for the bridge: the chained command's, or 0.
    ///
    /// Of all that stdin holds, only the `rate_limits` fields are kept, with a timestamp.
    pub fn run(&self, stdin: &str, stdout: &mut dyn Write, now: DateTime<Utc>) -> i32 {
        let limits = rate_limits_json::parse_status_line(stdin);
        if let Some(limits) = &limits {
            let stored = usage_file::read(self.usage_path);
            let merged = usage_merge::merge(stored.as_ref(), limits, now, ReadingFrom::StatusLine);
            // A failed write must not blank the user's status line.
            let _ = usage_file::write(self.usage_path, &merged);
        }

        if !self.inside_chain {
            let command = BridgeConfig::load(self.config_path).chained_command;
            let chained = command.and_then(|command| (self.run_chain)(&command, stdin));
            if let Some(chained) = chained {
                let _ = stdout.write_all(chained.output.as_bytes());
                return chained.exit_code;
            }
            // No command, or it could not run: fall through to the default line.
        }

        let line = status_line_text::render(limits.as_ref()) + "\n";
        let _ = stdout.write_all(line.as_bytes());
        0
    }
}
