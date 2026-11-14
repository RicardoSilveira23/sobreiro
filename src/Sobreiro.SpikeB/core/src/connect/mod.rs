//! Only what the shell needs of Connect: where Claude Code keeps its files, and the connection state enum.

mod claude_paths;

pub use claude_paths::ClaudePaths;

/// How the status line stands (the type only; Connect itself is not ported).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    NotConnected,
    Connected,
    Legacy,
    Elsewhere,
}
