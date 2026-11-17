//! `config.json`: read whole, and written back with everything it held, to the byte as 1.x
//! wrote it.
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, Visitor};
use serde_json::value::RawValue;

use crate::colours::BarColourOption;
use crate::display::{FlyoutStyle, PaceDays};
use crate::io::{atomic_file, bounded_read};

/// The status line the user had before Connect: the raw JSON object, or `None` when there was
/// none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousStatusLine {
    pub raw_json: Option<String>,
}

/// What the tray shows: the icon alone, or the icon and the taskbar pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowOption {
    TrayAndPill,
    TrayOnly,
}

/// `config.json`. Keys Sobreiro does not know are kept as raw JSON and written back on save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    /// The user's previous status line command, which the bridge chains to.
    pub chained_command: Option<String>,
    /// `None` when the file has no such key; a key holding `null` is a status line of "none".
    pub previous: Option<PreviousStatusLine>,
    pub style: FlyoutStyle,
    pub bar_colour: BarColourOption,
    pub show: ShowOption,
    pub pace_days: PaceDays,
    /// Each key Sobreiro does not know with its value's own text, in the file's order.
    pub extra: Vec<(String, String)>,
}

impl Default for AppConfig {
    fn default() -> AppConfig {
        AppConfig {
            chained_command: None,
            previous: None,
            style: FlyoutStyle::Bars,
            bar_colour: BarColourOption::Purple,
            show: ShowOption::TrayAndPill,
            pace_days: PaceDays::EveryDay,
            extra: Vec::new(),
        }
    }
}

/// Why `config.json` was not read or not written.
#[derive(Debug)]
pub enum ConfigError {
    /// The file is there but is not JSON, or not a JSON object.
    NotJson,
    /// A raw value to be written back under `key` is not JSON. Nothing was written.
    ValueNotJson { key: String },
    /// The file could not be read or written: another program holds it, say.
    Io(io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotJson => {
                f.write_str("config.json isn't valid JSON, so Sobreiro left it alone.")
            }
            ConfigError::ValueNotJson { key } => write!(
                f,
                "config.json was not saved: the value kept under \"{key}\" isn't valid JSON."
            ),
            ConfigError::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Io(error) => Some(error),
            ConfigError::NotJson | ConfigError::ValueNotJson { .. } => None,
        }
    }
}

const CHAINED_KEY: &str = "chained_command";
const PREVIOUS_KEY: &str = "previous_status_line";
const STYLE_KEY: &str = "style";
const COLOUR_KEY: &str = "bar_colour";
const SHOW_KEY: &str = "show";
const PACE_KEY: &str = "pace";

// The name each choice has in the file. Reading takes them with capitals too (those of the
// English alphabet); writing uses these.
const STYLES: [(&str, FlyoutStyle); 3] = [
    ("bars", FlyoutStyle::Bars),
    ("battery", FlyoutStyle::Battery),
    ("thermometer", FlyoutStyle::Thermometer),
];
const COLOURS: [(&str, BarColourOption); 2] = [
    ("purple", BarColourOption::Purple),
    ("accent", BarColourOption::Accent),
];
const SHOWS: [(&str, ShowOption); 2] = [
    ("trayandpill", ShowOption::TrayAndPill),
    ("trayonly", ShowOption::TrayOnly),
];
const PACES: [(&str, PaceDays); 2] = [
    ("everyday", PaceDays::EveryDay),
    ("weekdays", PaceDays::Weekdays),
];

impl AppConfig {
    /// Reads `config.json`.
    ///
    /// A missing file or folder, or a file that is empty or blank, gives the default. JSON that
    /// does not parse, or is not an object, gives [`ConfigError::NotJson`]; a file that cannot
    /// be read (another program holds it, say) gives [`ConfigError::Io`]. Either way a caller
    /// never saves over a config it could not read. A blank chained command counts as none.
    pub fn load(path: &Path) -> Result<AppConfig, ConfigError> {
        let text = match read_text(path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(AppConfig::default());
            }
            Err(error) => return Err(ConfigError::Io(error)),
        };
        if is_blank(&text) {
            return Ok(AppConfig::default());
        }
        let Members(members) = serde_json::from_str(&text).map_err(|_| ConfigError::NotJson)?;

        let mut config = AppConfig::default();
        for (key, value) in members {
            let raw = value.get();
            match key.as_str() {
                CHAINED_KEY => {
                    if let Some(command) = text_of(raw)?
                        && !is_blank(&command)
                    {
                        config.chained_command = Some(command);
                    }
                }
                PREVIOUS_KEY => {
                    let raw_json = (raw != "null").then(|| raw.to_string());
                    config.previous = Some(PreviousStatusLine { raw_json });
                }
                STYLE_KEY => config.style = named(raw, &STYLES, FlyoutStyle::Bars)?,
                COLOUR_KEY => config.bar_colour = named(raw, &COLOURS, BarColourOption::Purple)?,
                SHOW_KEY => config.show = named(raw, &SHOWS, ShowOption::TrayAndPill)?,
                PACE_KEY => config.pace_days = named(raw, &PACES, PaceDays::EveryDay)?,
                _ => keep(&mut config.extra, key, raw),
            }
        }
        Ok(config)
    }

    /// Writes `config.json` in one step, making the folder when it is missing.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let json = self.to_json()?;
        atomic_file::write_all_text(path, &json).map_err(ConfigError::Io)
    }

    /// The text [`AppConfig::save`] writes. Gives [`ConfigError::ValueNotJson`] when a raw
    /// value is not JSON, before anything touches the disk.
    ///
    /// The text is built by hand, so that it stays to the byte what 1.x wrote: one key to a
    /// line, set in by two spaces, each line ended by a carriage return and a line feed (the
    /// line end of Windows), and nothing after the last brace.
    pub fn to_json(&self) -> Result<String, ConfigError> {
        let mut lines = Vec::new();
        if let Some(command) = &self.chained_command {
            lines.push(line(CHAINED_KEY, &quoted(command)));
        }
        if let Some(previous) = &self.previous {
            let value = match &previous.raw_json {
                Some(raw) => checked(PREVIOUS_KEY, raw)?,
                None => "null",
            };
            lines.push(line(PREVIOUS_KEY, value));
        }
        lines.push(line(STYLE_KEY, &quoted(name_of(self.style, &STYLES))));
        lines.push(line(
            COLOUR_KEY,
            &quoted(name_of(self.bar_colour, &COLOURS)),
        ));
        lines.push(line(SHOW_KEY, &quoted(name_of(self.show, &SHOWS))));
        lines.push(line(PACE_KEY, &quoted(name_of(self.pace_days, &PACES))));
        for (key, raw) in &self.extra {
            lines.push(line(key, checked(key, raw)?));
        }
        Ok(format!("{{\r\n{}\r\n}}", lines.join(",\r\n")))
    }
}

/// The three bytes that some programs put at the start of a UTF-8 file.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The file's text. Bytes that are not UTF-8 are read as the "unknown character" sign, not
/// refused.
fn read_text(path: &Path) -> io::Result<String> {
    let bytes =
        bounded_read::read_at_most(open_while_nobody_writes(path)?, bounded_read::MAX_BYTES)?;
    let text = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&bytes);
    Ok(String::from_utf8_lossy(text).into_owned())
}

/// Opens the file for reading in the way 1.x does: others may read it meanwhile, but nobody
/// may write it. So opening fails while a program has the file open for writing, and no
/// program can start writing before the read is over. A half-written file is never read, and
/// so never taken for an empty one and saved over.
#[cfg(windows)]
fn open_while_nobody_writes(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    // What others may do with the file meanwhile. Windows' number for "read, nothing else".
    const SHARE_READING_ONLY: u32 = 1;
    File::options()
        .read(true)
        .share_mode(SHARE_READING_ONLY)
        .open(path)
}

/// Other systems have no such rule to ask for. Sobreiro runs on Windows only so far.
#[cfg(not(windows))]
fn open_while_nobody_writes(path: &Path) -> io::Result<File> {
    File::open(path)
}

/// True for a text of nothing, or of nothing but spaces of any kind and line ends.
pub(crate) fn is_blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

/// What a `config.json` holds: each key with its value's own text, in the file's order.
///
/// `serde_json` reads JSON into whatever type says how it is to be filled, through the trait
/// `Deserialize`. A ready-made map would lose the order of the keys and would read every
/// value, where an unknown key's value must come back to the byte. So this type says it
/// itself: it asks for an object and takes each value as the raw text it was written in.
struct Members(Vec<(String, Box<RawValue>)>);

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Members, D::Error> {
        deserializer.deserialize_map(MembersVisitor)
    }
}

/// Receives the object's keys and values from `serde_json`, one pair at a time.
struct MembersVisitor;

impl<'de> Visitor<'de> for MembersVisitor {
    type Value = Members;

    /// Completes the error message "expected ..." when the JSON is something else.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Members, A::Error> {
        let mut members = Vec::new();
        while let Some(member) = object.next_entry::<String, Box<RawValue>>()? {
            members.push(member);
        }
        Ok(Members(members))
    }
}

/// The text a raw JSON value holds, or `None` when the value is not a text.
///
/// The JSON was checked when the file was read, with one thing left open: a text may spell
/// out half of a character (`\uD800` with nothing after it), which no text can hold. Where
/// Sobreiro has to read such a text, the file counts as not JSON.
fn text_of(raw: &str) -> Result<Option<String>, ConfigError> {
    if !raw.starts_with('"') {
        return Ok(None);
    }
    match serde_json::from_str(raw) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Err(ConfigError::NotJson),
    }
}

/// The choice a raw JSON value names: the whole name, in capitals or small letters. Anything
/// else, a number or an unknown name, gives `fallback`.
fn named<T: Copy>(raw: &str, names: &[(&str, T)], fallback: T) -> Result<T, ConfigError> {
    let Some(text) = text_of(raw)? else {
        return Ok(fallback);
    };
    let found = names
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&text));
    Ok(found.map_or(fallback, |(_, choice)| *choice))
}

/// The name a choice has in the file.
fn name_of<T: PartialEq>(choice: T, names: &[(&'static str, T)]) -> &'static str {
    let found = names.iter().find(|(_, named)| *named == choice);
    // Every choice is in its list above, so there is always a name.
    found.map_or("", |(name, _)| name)
}

/// Keeps an unknown key's raw value. A key the file holds twice keeps its first place and
/// its last value.
fn keep(extra: &mut Vec<(String, String)>, key: String, raw: &str) {
    match extra.iter_mut().find(|(name, _)| *name == key) {
        Some((_, value)) => *value = raw.to_string(),
        None => extra.push((key, raw.to_string())),
    }
}

/// One line of the file, without its line end: a key and its value as JSON.
fn line(key: &str, value: &str) -> String {
    format!("  {}: {value}", quoted(key))
}

/// `raw` itself when it is one whole JSON value, and an error naming `key` when it is not.
fn checked<'a>(key: &str, raw: &'a str) -> Result<&'a str, ConfigError> {
    // `IgnoredAny` reads a JSON value and keeps nothing of it: what is wanted is only
    // whether reading worked.
    match serde_json::from_str::<IgnoredAny>(raw) {
        Ok(_) => Ok(raw),
        Err(_) => Err(ConfigError::ValueNotJson {
            key: key.to_string(),
        }),
    }
}

/// `text` as a JSON text between quotes, with the same characters spelled out as 1.x spelled
/// out: `\"` and `\\`, a short spelling for five of the characters that give orders to a
/// screen, and `\u` with four hexadecimal digits for the rest of them and for the characters
/// [`written_as_itself`] turns down.
///
/// Connect writes the `statusLine` object with it too: 1.x wrote both with the same writer.
pub(crate) fn quoted(text: &str) -> String {
    let mut json = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{8}' => json.push_str("\\b"),
            '\u{c}' => json.push_str("\\f"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            other if written_as_itself(other) => json.push(other),
            other => {
                // A character past the first 65,536 is written as the two codes that make
                // it up in UTF-16, the form in which C# holds text.
                let mut codes = [0u16; 2];
                for code in other.encode_utf16(&mut codes) {
                    json.push_str(&format!("\\u{code:04X}"));
                }
            }
        }
    }
    json.push('"');
    json
}

/// True for the characters 1.x wrote as themselves. It wrote a code instead for: characters
/// that give orders to a screen, every space but the plain one, the mark some programs start
/// a file with, codes set aside for private use, codes that will never be a character, and
/// everything past the first 65,536.
fn written_as_itself(character: char) -> bool {
    let spelled_out = character.is_control()
        || (character.is_whitespace() && character != ' ')
        || character == '\u{feff}'
        || ('\u{e000}'..='\u{f8ff}').contains(&character)
        || ('\u{fdd0}'..='\u{fdef}').contains(&character)
        || character >= '\u{fffe}';
    !spelled_out
}
