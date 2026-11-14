use crate::colours::BarColourOption;
use crate::display::{FlyoutStyle, PaceDays};
use crate::io::AtomicFile;
use serde::Deserialize;
use serde::de::{MapAccess, Visitor};
use serde_json::value::RawValue;
use std::fmt;
use std::io;

/// The status line the user had before Connect: the raw JSON object, or None when there was none.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviousStatusLine {
    pub raw_json: Option<String>,
}

/// What the tray shows: the icon alone, or the icon and the taskbar pill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowOption {
    TrayAndPill,
    TrayOnly,
}

/// Why `AppConfig::load` or `to_json` failed. Format is `FormatException`; Io is a locked or unreadable file.
#[derive(Debug)]
pub enum ConfigError {
    Format(String),
    Io(io::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Format(m) => f.write_str(m),
            ConfigError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// config.json. Unknown keys are kept as raw JSON and written back on save.
#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    pub chained_command: Option<String>,
    pub previous: Option<PreviousStatusLine>,
    pub style: FlyoutStyle,
    pub bar_colour: BarColourOption,
    /// Unknown keys and their raw JSON text, in file order.
    pub extra: Vec<(String, String)>,
    pub show: ShowOption,
    pub pace_days: PaceDays,
}

const CHAINED_KEY: &str = "chained_command";
const PREVIOUS_KEY: &str = "previous_status_line";
const STYLE_KEY: &str = "style";
const COLOUR_KEY: &str = "bar_colour";
const SHOW_KEY: &str = "show";
const PACE_KEY: &str = "pace";

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            chained_command: None,
            previous: None,
            style: FlyoutStyle::Bars,
            bar_colour: BarColourOption::Purple,
            extra: Vec::new(),
            show: ShowOption::TrayAndPill,
            pace_days: PaceDays::EveryDay,
        }
    }
}

/// The object's members in file order, each value as its raw text.
struct RawMembers(Vec<(String, Box<RawValue>)>);

impl<'de> Deserialize<'de> for RawMembers {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = RawMembers;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<RawMembers, A::Error> {
                let mut v = Vec::new();
                while let Some(entry) = map.next_entry::<String, Box<RawValue>>()? {
                    v.push(entry);
                }
                Ok(RawMembers(v))
            }
        }
        d.deserialize_map(V)
    }
}

fn not_json() -> ConfigError {
    ConfigError::Format("config.json isn't valid JSON, so Sobreiro left it alone.".to_string())
}

trait Named: Sized + Copy + 'static {
    const ALL: &'static [(&'static str, Self)];
}

impl Named for FlyoutStyle {
    const ALL: &'static [(&'static str, Self)] =
        &[("Bars", FlyoutStyle::Bars), ("Battery", FlyoutStyle::Battery), ("Thermometer", FlyoutStyle::Thermometer)];
}
impl Named for BarColourOption {
    const ALL: &'static [(&'static str, Self)] = &[("Purple", BarColourOption::Purple), ("Accent", BarColourOption::Accent)];
}
impl Named for ShowOption {
    const ALL: &'static [(&'static str, Self)] = &[("TrayAndPill", ShowOption::TrayAndPill), ("TrayOnly", ShowOption::TrayOnly)];
}
impl Named for PaceDays {
    const ALL: &'static [(&'static str, Self)] = &[("EveryDay", PaceDays::EveryDay), ("Weekdays", PaceDays::Weekdays)];
}

/// A string that names a variant, ignoring case; anything else is the fallback.
fn parse_enum<T: Named>(raw: &str, fallback: T) -> T {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::String(s)) => {
            T::ALL.iter().find(|(n, _)| n.eq_ignore_ascii_case(&s)).map(|(_, v)| *v).unwrap_or(fallback)
        }
        _ => fallback,
    }
}

fn name_of<T: Named + PartialEq>(v: T) -> String {
    T::ALL.iter().find(|(_, x)| *x == v).map(|(n, _)| n.to_lowercase()).unwrap_or_default()
}

impl AppConfig {
    /// A missing file or folder, or an empty or blank file, gives the default. JSON that doesn't parse, or isn't an
    /// object, is `ConfigError::Format`; any other IO error (a locked file, say) is `ConfigError::Io`. Either way a
    /// caller never saves over a config it could not read. A blank chained command counts as none.
    pub fn load(path: &str) -> Result<AppConfig, ConfigError> {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(AppConfig::default()),
            Err(e) => return Err(ConfigError::Io(e)),
        };
        let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        let text = String::from_utf8_lossy(body);
        if text.trim().is_empty() {
            return Ok(AppConfig::default());
        }
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<AppConfig, ConfigError> {
        let members: RawMembers = serde_json::from_str(text).map_err(|_| not_json())?;
        let mut c = AppConfig::default();
        for (name, value) in members.0 {
            let raw = value.get();
            match name.as_str() {
                CHAINED_KEY => {
                    if let Ok(serde_json::Value::String(s)) = serde_json::from_str(raw) {
                        if !s.trim().is_empty() {
                            c.chained_command = Some(s);
                        }
                    }
                }
                PREVIOUS_KEY => {
                    c.previous = Some(PreviousStatusLine { raw_json: if raw == "null" { None } else { Some(raw.to_string()) } });
                }
                STYLE_KEY => c.style = parse_enum(raw, FlyoutStyle::Bars),
                COLOUR_KEY => c.bar_colour = parse_enum(raw, BarColourOption::Purple),
                SHOW_KEY => c.show = parse_enum(raw, ShowOption::TrayAndPill),
                PACE_KEY => c.pace_days = parse_enum(raw, PaceDays::EveryDay),
                _ => match c.extra.iter_mut().find(|(k, _)| *k == name) {
                    Some(slot) => slot.1 = raw.to_string(),
                    None => c.extra.push((name, raw.to_string())),
                },
            }
        }
        Ok(c)
    }

    pub fn save(&self, path: &str) -> Result<(), ConfigError> {
        let json = self.to_json()?;
        AtomicFile::write_all_text(path, &json, false).map_err(ConfigError::Io)
    }

    /// The text `save` writes (indented, two spaces). Fails when a raw value is not valid JSON, before anything
    /// touches the disk.
    pub fn to_json(&self) -> Result<String, ConfigError> {
        let mut members: Vec<(String, String)> = Vec::new();
        let s = |v: &str| serde_json::to_string(v).unwrap_or_default();
        if let Some(c) = &self.chained_command {
            members.push((CHAINED_KEY.into(), s(c)));
        }
        if let Some(p) = &self.previous {
            members.push((PREVIOUS_KEY.into(), p.raw_json.clone().unwrap_or_else(|| "null".into())));
        }
        members.push((STYLE_KEY.into(), s(&name_of(self.style))));
        members.push((COLOUR_KEY.into(), s(&name_of(self.bar_colour))));
        members.push((SHOW_KEY.into(), s(&name_of(self.show))));
        members.push((PACE_KEY.into(), s(&name_of(self.pace_days))));
        for (k, raw) in &self.extra {
            members.push((k.clone(), raw.clone()));
        }
        for (_, raw) in &members {
            serde_json::from_str::<&RawValue>(raw).map_err(|_| ConfigError::Format("A raw value is not valid JSON.".into()))?;
        }
        let mut out = String::from("{");
        for (i, (k, raw)) in members.iter().enumerate() {
            out.push_str(if i == 0 { "\n  " } else { ",\n  " });
            out.push_str(&s(k));
            out.push_str(": ");
            out.push_str(raw);
        }
        out.push_str("\n}");
        Ok(out)
    }
}
