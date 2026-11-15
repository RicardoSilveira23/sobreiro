//! The status line command that runs the bridge, and the `statusLine` object around it.
use std::path::Path;

use super::json_reader::{self, Dialect, Kind, Value};
use super::settings_json::SettingsError;
use crate::app_paths::capital;
use crate::config::app_config::quoted;

const BRIDGE_NAME: &str = "sobreiro-bridge";

/// The bridge the app was called before 0.5.0 (OpenHeadroom).
const LEGACY_BRIDGE_NAME: &str = "openheadroom-bridge";

/// The command for the bridge at `bridge_exe_path`: with forward slashes and between quotes,
/// so Git Bash runs it as one path.
pub fn command_for(bridge_exe_path: &Path) -> String {
    let path = bridge_exe_path.to_string_lossy().replace('\\', "/");
    format!("\"{path}\"")
}

/// True when `command` runs exactly `bridge_exe_path`: spaces around it and one pair of
/// double quotes are taken off, `/` is read as a backslash on both sides, both paths are made
/// full (`..` worked out), and capitals do not count. Anything after the path (a pipe,
/// arguments) makes it false.
pub fn runs(command: Option<&str>, bridge_exe_path: &Path) -> bool {
    let Some(command) = command else {
        return false;
    };
    let trimmed = command.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(trimmed);
    match (
        normalise(unquoted),
        normalise(&bridge_exe_path.to_string_lossy()),
    ) {
        (Some(command_path), Some(bridge_path)) => {
            same_ignoring_capitals(&command_path, &bridge_path)
        }
        // A text Windows cannot read as a path runs nothing.
        _ => false,
    }
}

/// The full path as Windows reads it: `.` and `..` worked out, a doubled separator made one,
/// dots and spaces at the very end gone. `None` for a text that is no path, an empty one say.
///
/// The standard library puts this question to Windows, as 1.x did. For a full path the answer
/// depends on the text alone. A path that is not full is read from the folder the program
/// runs in, here as in 1.x.
fn normalise(path: &str) -> Option<String> {
    let full = std::path::absolute(path.replace('/', "\\")).ok()?;
    Some(full.to_string_lossy().into_owned())
}

/// True when two paths are the same letter for letter, capitals aside.
fn same_ignoring_capitals(one: &str, other: &str) -> bool {
    one.chars()
        .map(capital_for_comparing)
        .eq(other.chars().map(capital_for_comparing))
}

/// The capital 1.x compared by. It is the capital it named folders by, but for the long s
/// (`ſ`), which this comparison leaves as it is where the other makes it an `S`.
fn capital_for_comparing(letter: char) -> char {
    if letter == 'ſ' {
        return letter;
    }
    capital(letter)
}

/// True when `command` holds `name`, in capitals or small letters. Both names are plain
/// English letters and a hyphen, and no other letter counts as one of those when capitals
/// are ignored, so only the English alphabet has to be folded.
fn names(command: &str, name: &str) -> bool {
    command.to_ascii_lowercase().contains(name)
}

/// True for a command running this app's bridge, or the old one: both are our own.
pub fn is_bridge(command: Option<&str>) -> bool {
    command.is_some_and(|command| names(command, BRIDGE_NAME)) || is_legacy_bridge(command)
}

/// True for a command running the old `openheadroom-bridge`: connected, but to the old name.
pub fn is_legacy_bridge(command: Option<&str>) -> bool {
    command
        .is_some_and(|command| names(command, LEGACY_BRIDGE_NAME) && !names(command, BRIDGE_NAME))
}

/// A `statusLine` object running `command`, laid out for a top-level key of `settings.json`:
/// its own keys set in by four spaces, its closing brace by two. The other keys of the
/// existing object (`padding`, say) are kept, after `command`.
///
/// The existing object is read and written anew, not copied: its texts come out with their
/// characters spelled as Sobreiro spells them, and its spacing becomes one key to a line. An
/// existing value that is no object, or no JSON, is dropped. One thing gives an error: a text
/// or a name in it that spells out half a character, which cannot be written again.
pub fn status_line_json(command: &str, existing: Option<&str>) -> Result<String, SettingsError> {
    let mut json = String::from("{\n    \"type\": \"command\",\n    \"command\": ");
    json.push_str(&quoted(command));
    if let Some(existing) = existing
        && let Ok(Value {
            kind: Kind::Object(members),
            ..
        }) = json_reader::read(existing, Dialect::Relaxed)
    {
        for member in &members {
            let name = json_reader::text_of(member.name).ok_or(SettingsError::NotJson)?;
            if name == "type" || name == "command" {
                continue;
            }
            json.push_str(",\n    ");
            json.push_str(&quoted(&name));
            json.push_str(": ");
            write_value(&mut json, &member.value, 1)?;
        }
    }
    json.push_str("\n  }");
    Ok(json)
}

/// Writes `value` at the end of `json`, as 1.x's writer laid it out: every item of an object
/// or a list on a line of its own, four spaces further in for each level, and an empty one as
/// `{}` or `[]`. `level` says how deep the value itself sits. A number is written as it was
/// read, digit for digit.
fn write_value(json: &mut String, value: &Value<'_>, level: usize) -> Result<(), SettingsError> {
    match &value.kind {
        Kind::Null => json.push_str("null"),
        Kind::True => json.push_str("true"),
        Kind::False => json.push_str("false"),
        Kind::Number(raw) => json.push_str(raw),
        Kind::Text(raw) => {
            let text = json_reader::text_of(raw).ok_or(SettingsError::NotJson)?;
            json.push_str(&quoted(&text));
        }
        Kind::List(items) if items.is_empty() => json.push_str("[]"),
        Kind::List(items) => {
            json.push('[');
            for (index, item) in items.iter().enumerate() {
                new_line(json, index, level + 1);
                write_value(json, item, level + 1)?;
            }
            new_line(json, 0, level);
            json.push(']');
        }
        Kind::Object(members) if members.is_empty() => json.push_str("{}"),
        Kind::Object(members) => {
            json.push('{');
            for (index, member) in members.iter().enumerate() {
                new_line(json, index, level + 1);
                let name = json_reader::text_of(member.name).ok_or(SettingsError::NotJson)?;
                json.push_str(&quoted(&name));
                json.push_str(": ");
                write_value(json, &member.value, level + 1)?;
            }
            new_line(json, 0, level);
            json.push('}');
        }
    }
    Ok(())
}

/// Starts a line set in for `level`, after a comma when the item before it needs one.
fn new_line(json: &mut String, items_before: usize, level: usize) {
    if items_before > 0 {
        json.push(',');
    }
    json.push('\n');
    json.push_str(&"    ".repeat(level));
}
