//! Reads and replaces the top-level `statusLine` value of Claude Code's `settings.json`.
//!
//! The file belongs to Claude Code and to the user. So nothing here writes the file anew from
//! what was read: the text is only ever cut and pasted, byte by byte, and everything outside
//! that one key comes back exactly as it was, with its spacing, its order and its comments.
use std::fmt;

use super::json_reader::{self, Dialect, Kind, NotJson, Reader};
use crate::config::app_config::is_blank;

const KEY: &str = "statusLine";

/// Why `settings.json` was left alone. The words are shown as they are in the Connect flow's
/// error box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    /// The file is JSON, but something else than an object: a list, say.
    NotAnObject,
    /// The file is not JSON, even allowing for comments and a comma after the last item.
    NotJson,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SettingsError::NotAnObject => {
                "settings.json isn't a JSON object, so Sobreiro left it alone."
            }
            SettingsError::NotJson => "settings.json isn't valid JSON, so Sobreiro left it alone.",
        })
    }
}

impl std::error::Error for SettingsError {}

impl From<NotJson> for SettingsError {
    fn from(_: NotJson) -> SettingsError {
        SettingsError::NotJson
    }
}

/// One key of the file's outermost object, and where it sits in the text: `name_start` is the
/// name's opening quote, `value_start` the value's first byte and `value_end` the byte after
/// its last.
struct Prop {
    name: String,
    name_start: usize,
    value_start: usize,
    value_end: usize,
}

/// The keys of the outermost object in the file's order, with where its braces sit.
struct Layout {
    props: Vec<Prop>,
    open: usize,
    close: usize,
}

impl Layout {
    /// Where the `statusLine` key is in `props`. A file that names it twice means the last
    /// one, as Claude Code's own reader would.
    fn status_line(&self) -> Option<usize> {
        self.props.iter().rposition(|prop| prop.name == KEY)
    }
}

/// The text of the top-level `statusLine` value as it is written in the file, or `None` when
/// there is none. A blank text counts as `{}`.
pub fn read_status_line(text: &str) -> Result<Option<&str>, SettingsError> {
    if is_blank(text) {
        return Ok(None);
    }
    let layout = parse(text)?;
    let Some(prop) = layout
        .status_line()
        .and_then(|index| layout.props.get(index))
    else {
        return Ok(None);
    };
    Ok(text.get(prop.value_start..prop.value_end))
}

/// The `command` text of a `statusLine` object, or `None`: when there is no status line, when
/// it is not an object, and when it has no `command` that is a text.
pub fn command_of(status_line_json: Option<&str>) -> Option<String> {
    command_or_error(status_line_json).ok().flatten()
}

/// As [`command_of`], for Connect and Disconnect, which must not go on when the command cannot
/// be read: a `command` that spells out half a character gives an error, as it did in 1.x.
pub(crate) fn command_or_error(
    status_line_json: Option<&str>,
) -> Result<Option<String>, SettingsError> {
    let Some(json) = status_line_json else {
        return Ok(None);
    };
    let Ok(value) = json_reader::read(json, Dialect::Relaxed) else {
        return Ok(None);
    };
    let Kind::Object(members) = value.kind else {
        return Ok(None);
    };
    // An object that names `command` twice means the last one.
    let command = members
        .iter()
        .rev()
        .find(|member| json_reader::text_of(member.name).as_deref() == Some("command"));
    match command.map(|member| &member.value.kind) {
        Some(Kind::Text(raw)) => match json_reader::text_of(raw) {
            Some(command) => Ok(Some(command)),
            None => Err(SettingsError::NotJson),
        },
        _ => Ok(None),
    }
}

/// `text` with the `statusLine` value replaced by `value_json`. `None` removes the key (and
/// its comma); a key that is not there yet is added as the last one.
pub fn with_status_line(text: &str, value_json: Option<&str>) -> Result<String, SettingsError> {
    if is_blank(text) {
        return Ok(match value_json {
            Some(value) => format!("{{\n  \"{KEY}\": {value}\n}}\n"),
            None => text.to_string(),
        });
    }

    let bytes = text.as_bytes();
    let layout = parse(text)?;
    let changed = match (layout.status_line(), value_json) {
        (Some(index), None) => remove(bytes, &layout, index),
        (Some(index), Some(value)) => match layout.props.get(index) {
            Some(prop) => splice(bytes, prop.value_start, prop.value_end, value.as_bytes()),
            None => bytes.to_vec(),
        },
        (None, None) => return Ok(text.to_string()),
        (None, Some(value)) => insert(bytes, text, &layout, value),
    };
    // Every cut was made beside a quote, a brace or a comma, never inside a character, so
    // the bytes are still text.
    Ok(String::from_utf8_lossy(&changed).into_owned())
}

/// Reads the whole file and notes where each key of its outermost object sits.
fn parse(text: &str) -> Result<Layout, SettingsError> {
    let mut reader = Reader::new(text, Dialect::Relaxed);
    reader.skip_trivia()?;
    if reader.peek() != Some(b'{') {
        // 1.x looked at the first piece only before it said "not an object", so a list that
        // is broken further on gets these words too, and not "not valid JSON".
        reader.first_piece()?;
        return Err(SettingsError::NotAnObject);
    }
    let root = reader.value()?;
    reader.end()?; // anything after the object is an error

    let Kind::Object(members) = root.kind else {
        return Err(SettingsError::NotAnObject);
    };
    let mut props = Vec::new();
    for member in members {
        props.push(Prop {
            // A name that spells out half a character is no name at all. 1.x stopped with
            // an error here as well, of another kind.
            name: json_reader::text_of(member.name).ok_or(SettingsError::NotJson)?,
            name_start: member.name_start,
            value_start: member.value.start,
            value_end: member.value.end,
        });
    }
    Ok(Layout {
        props,
        open: root.start,
        close: root.end.saturating_sub(1),
    })
}

fn is_space_or_tab(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// The byte at `index`, or `None` past either end.
fn at(b: &[u8], index: usize) -> Option<u8> {
    b.get(index).copied()
}

/// The byte before `index`, or `None` at the start.
fn before(b: &[u8], index: usize) -> Option<u8> {
    index.checked_sub(1).and_then(|previous| at(b, previous))
}

/// The bytes without the `statusLine` key at `index`: its name, its value, one comma, and the
/// line it stood on when it had a line to itself.
fn remove(b: &[u8], layout: &Layout, index: usize) -> Vec<u8> {
    let Some(prop) = layout.props.get(index) else {
        return b.to_vec();
    };
    let start = prop.name_start;
    let mut end = prop.value_end;
    let after_value = skip_trivia(b, end);
    let has_comma = at(b, after_value) == Some(b',');
    if has_comma {
        end = after_value + 1;
    }

    // The only key, with nothing but spaces around it: the object becomes `{}`.
    if layout.props.len() == 1
        && all_whitespace(b, layout.open + 1, start)
        && all_whitespace(b, end, layout.close)
    {
        return splice(b, layout.open + 1, layout.close, &[]);
    }

    // The last key leaves the comma of the key before it with nothing to separate.
    let last = index == layout.props.len() - 1;
    let mut previous_comma = None;
    if last
        && !has_comma
        && let Some(previous) = index.checked_sub(1).and_then(|i| layout.props.get(i))
    {
        let comma = skip_trivia(b, previous.value_end);
        if at(b, comma) == Some(b',') {
            previous_comma = Some(comma);
        }
    }

    let mut line_start = start;
    while before(b, line_start).is_some_and(is_space_or_tab) {
        line_start -= 1;
    }
    let mut line_end = end;
    while at(b, line_end).is_some_and(is_space_or_tab) {
        line_end += 1;
    }
    let own_line = matches!(before(b, line_start), None | Some(b'\n'))
        && matches!(at(b, line_end), Some(b'\n' | b'\r'));

    if own_line {
        if at(b, line_end) == Some(b'\r') {
            line_end += 1;
        }
        if at(b, line_end) == Some(b'\n') {
            line_end += 1;
        }
        let result = splice(b, line_start, line_end, &[]);
        // the previous comma sits before line_start, so its index is unchanged
        return match previous_comma {
            Some(comma) => splice(&result, comma, comma + 1, &[]),
            None => result,
        };
    }

    if let Some(comma) = previous_comma {
        return splice(b, comma, prop.value_end, &[]);
    }
    splice(b, start, line_end, &[])
}

/// The bytes with a `statusLine` key added after the last key, on a line of its own, set in
/// as that key is and with the file's kind of line end.
fn insert(b: &[u8], text: &str, layout: &Layout, value_json: &str) -> Vec<u8> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let property = format!("\"{KEY}\": {value_json}");

    let Some(last) = layout.props.last() else {
        let inside = layout.open + 1;
        if all_whitespace(b, inside, layout.close) {
            let lines = format!("{newline}  {property}{newline}");
            return splice(b, inside, layout.close, lines.as_bytes());
        }
        // An object that holds only a comment keeps it, after the new key.
        let line = format!("{newline}  {property}");
        return splice(b, inside, inside, line.as_bytes());
    };

    let mut line_start = last.name_start;
    while before(b, line_start).is_some_and(is_space_or_tab) {
        line_start -= 1;
    }
    let indent = if matches!(before(b, line_start), None | Some(b'\n')) {
        text.get(line_start..last.name_start).unwrap_or_default()
    } else {
        "  "
    };

    let after = skip_trivia(b, last.value_end);
    if at(b, after) == Some(b',') {
        let line = format!("{newline}{indent}{property},");
        return splice(b, after + 1, after + 1, line.as_bytes());
    }
    let line = format!(",{newline}{indent}{property}");
    splice(b, last.value_end, last.value_end, line.as_bytes())
}

/// `b` with the bytes from `start` up to `end` replaced by `insert`.
fn splice(b: &[u8], start: usize, end: usize, insert: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(b.len() + insert.len());
    result.extend_from_slice(b.get(..start).unwrap_or_default());
    result.extend_from_slice(insert);
    result.extend_from_slice(b.get(end..).unwrap_or_default());
    result
}

fn all_whitespace(b: &[u8], start: usize, end: usize) -> bool {
    // An empty stretch (and one whose end comes before its start) holds nothing but spaces.
    let stretch = b.get(start..end).unwrap_or_default();
    stretch
        .iter()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
}

/// Index of the next byte that is not whitespace or part of a comment.
fn skip_trivia(b: &[u8], mut i: usize) -> usize {
    loop {
        match (at(b, i), at(b, i + 1)) {
            (Some(b' ' | b'\t' | b'\r' | b'\n'), _) => i += 1,
            (Some(b'/'), Some(b'/')) => {
                while at(b, i).is_some_and(|byte| byte != b'\n') {
                    i += 1;
                }
            }
            (Some(b'/'), Some(b'*')) => {
                i += 2;
                while i + 1 < b.len() && !(at(b, i) == Some(b'*') && at(b, i + 1) == Some(b'/')) {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            _ => return i,
        }
    }
}
