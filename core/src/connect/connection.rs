//! Whether Claude Code's status line runs Sobreiro's bridge.

/// What the `statusLine` entry in Claude Code's `settings.json` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// No status line, or one that is not Sobreiro's.
    NotConnected,
    /// This install's bridge.
    Connected,
    /// The bridge under its old name, from before the app was renamed. It still counts as
    /// connected.
    Legacy,
    /// A Sobreiro bridge in another folder than this install's.
    Elsewhere,
}
