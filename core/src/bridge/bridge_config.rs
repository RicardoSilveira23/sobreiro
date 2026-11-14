//! The one setting the bridge reads from `config.json`.
use std::path::Path;

use serde_json::Value;

/// `config.json` as the bridge sees it: the user's previous status line command, which the
/// bridge chains to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeConfig {
    pub chained_command: Option<String>,
}

impl BridgeConfig {
    /// Reads `chained_command`. A file that is missing or not JSON, a value that is not a text,
    /// and a text of nothing but spaces all give no command.
    pub fn load(path: &Path) -> BridgeConfig {
        BridgeConfig {
            chained_command: read_chained_command(path),
        }
    }
}

/// The three bytes that some programs put at the start of a UTF-8 file.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

fn read_chained_command(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&bytes);
    let document: Value = serde_json::from_slice(text).ok()?;
    let command = document.as_object()?.get("chained_command")?.as_str()?;
    if command.trim().is_empty() {
        return None;
    }
    Some(command.to_string())
}
