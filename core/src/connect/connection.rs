//! Connect and Disconnect: whether Claude Code's status line runs Sobreiro's bridge, the plan
//! to change that, and carrying the plan out.
//!
//! Only the `statusLine` key of `settings.json` ever changes, and never without a backup of
//! the file as it was.
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;

use super::bridge_command::{command_for, is_bridge, runs, status_line_json};
use super::json_reader::{self, Dialect};
use super::settings_json::{
    SettingsError, command_of, command_or_error, read_status_line, with_status_line,
};
use crate::config::app_config::is_blank;
use crate::config::{AppConfig, ConfigError, PreviousStatusLine};
use crate::io::atomic_file;

/// `settings.json` as read from the disk: whether it exists, its text (without the byte order
/// mark) and whether it had a byte order mark. `path` is the path the user sees, even when
/// it is a symbolic link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub path: PathBuf,
    pub exists: bool,
    pub text: String,
    pub bom: bool,
}

/// What Connect or Disconnect would do. `before` and `after` are the `statusLine` value as
/// written in the file (`None` when absent or removed), to show the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionPlan {
    /// True for Connect, false for Disconnect.
    pub connect: bool,
    pub settings: SettingsSnapshot,
    pub before: Option<String>,
    pub after: Option<String>,
    pub new_settings_text: String,
    pub new_config: AppConfig,
    /// True when the user's own status line keeps running, through the bridge.
    pub chains: bool,
    /// True when the status line already ran a bridge, but not this Sobreiro's: a
    /// `sobreiro-bridge` in another folder.
    pub reconnects: bool,
}

/// What the `statusLine` entry in Claude Code's `settings.json` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// No status line, or one that is not Sobreiro's.
    NotConnected,
    /// This install's bridge.
    Connected,
    /// A Sobreiro bridge in another folder than this install's.
    Elsewhere,
}

/// Why a plan was not made or not carried out. The words are shown as they are in the Connect
/// flow's error box.
#[derive(Debug)]
pub enum ConnectError {
    /// `settings.json` is not JSON, or not an object.
    Settings(SettingsError),
    /// `settings.json` holds bytes that are no text.
    NotUtf8,
    /// The user's own status line holds a comment or a comma after its last item, so it
    /// cannot be saved for Disconnect to put back as it was.
    StatusLineNotExact,
    /// Disconnect was asked for, but the status line does not run the bridge.
    NotConnected,
    /// `settings.json` is no longer what the plan was made from.
    Changed,
    /// `config.json` could not be written.
    Config(ConfigError),
    /// A file could not be read, copied or written.
    Io(io::Error),
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectError::Settings(error) => error.fmt(f),
            ConnectError::NotUtf8 => {
                f.write_str("settings.json isn't valid UTF-8, so Sobreiro left it alone.")
            }
            ConnectError::StatusLineNotExact => f.write_str(
                "statusLine has comments or trailing commas, so Sobreiro can't keep an exact copy to restore. Remove them and try again.",
            ),
            ConnectError::NotConnected => {
                f.write_str("The status line does not run the Sobreiro bridge.")
            }
            ConnectError::Changed => f.write_str("settings.json changed since the plan was made."),
            ConnectError::Config(error) => error.fmt(f),
            ConnectError::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ConnectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConnectError::Settings(error) => Some(error),
            ConnectError::Config(error) => Some(error),
            ConnectError::Io(error) => Some(error),
            ConnectError::NotUtf8
            | ConnectError::StatusLineNotExact
            | ConnectError::NotConnected
            | ConnectError::Changed => None,
        }
    }
}

// These three let `?` turn an error of a smaller kind into a `ConnectError`.
impl From<SettingsError> for ConnectError {
    fn from(error: SettingsError) -> ConnectError {
        ConnectError::Settings(error)
    }
}

impl From<ConfigError> for ConnectError {
    fn from(error: ConfigError) -> ConnectError {
        ConnectError::Config(error)
    }
}

impl From<io::Error> for ConnectError {
    fn from(error: io::Error) -> ConnectError {
        ConnectError::Io(error)
    }
}

/// The three bytes that some programs put at the start of a UTF-8 file.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Reads `settings.json`. A file that is not there gives a snapshot that says so; one whose
/// bytes are no text gives [`ConnectError::NotUtf8`].
pub fn read_settings(path: &Path) -> Result<SettingsSnapshot, ConnectError> {
    let target = target(path)?;
    if !target.is_file() {
        return Ok(SettingsSnapshot {
            path: path.to_path_buf(),
            exists: false,
            text: String::new(),
            bom: false,
        });
    }

    let bytes = std::fs::read(&target)?;
    let without_mark = bytes.strip_prefix(&BYTE_ORDER_MARK);
    let text =
        std::str::from_utf8(without_mark.unwrap_or(&bytes)).map_err(|_| ConnectError::NotUtf8)?;
    Ok(SettingsSnapshot {
        path: path.to_path_buf(),
        exists: true,
        text: text.to_string(),
        bom: without_mark.is_some(),
    })
}

/// How many links may point one to the next before the chain is given up on.
const MOST_LINKS: usize = 63;

/// The file a symbolic link finally points at, so that it is read and written there and the
/// link stays a link; any other path as it is.
fn target(path: &Path) -> io::Result<PathBuf> {
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|about| about.file_type().is_symlink());
    if !is_link {
        return Ok(path.to_path_buf());
    }
    // Windows names the file at the end of the chain, when there is one.
    if let Ok(end) = std::fs::canonicalize(path) {
        return Ok(end);
    }
    // The chain ends at a file that is not there. It is walked by hand, to the name the
    // missing file would have.
    let mut current = path.to_path_buf();
    for _ in 0..MOST_LINKS {
        let Ok(next) = std::fs::read_link(&current) else {
            return Ok(current);
        };
        // A link may name its target from the folder it is in. `join` leaves a full path
        // as it is.
        current = match current.parent() {
            Some(folder) => folder.join(next),
            None => next,
        };
    }
    Err(io::Error::other(
        "settings.json is a symbolic link to a link to a link, without end.",
    ))
}

/// A command that is nothing but spaces is no command.
fn command_or_none(command: Option<String>) -> Option<String> {
    command.filter(|command| !is_blank(command))
}

/// True when the status line runs a Sobreiro bridge, this install's or one elsewhere.
pub fn is_connected(settings_text: &str) -> bool {
    state(settings_text, None) != ConnectionState::NotConnected
}

/// Connected or not connected. A `settings.json` that cannot be read counts as not
/// connected.
///
/// With `own_bridge_path`, a `sobreiro-bridge` that runs from anywhere else is `Elsewhere`
/// (Reconnect points it at this Sobreiro's). `None` means "cannot tell": any
/// `sobreiro-bridge` is `Connected`.
pub fn state(settings_text: &str, own_bridge_path: Option<&Path>) -> ConnectionState {
    let Ok(status_line) = read_status_line(settings_text) else {
        return ConnectionState::NotConnected;
    };
    let command = command_of(status_line);
    let command = command.as_deref();
    if !is_bridge(command) {
        return ConnectionState::NotConnected;
    }
    match own_bridge_path {
        Some(own) if !runs(command, own) => ConnectionState::Elsewhere,
        _ => ConnectionState::Connected,
    }
}

/// The plan to point the status line at the bridge at `bridge_exe_path`.
///
/// Gives an error, and no plan, when `settings.json` is not a JSON object, and when the
/// user's own status line could not be put back exactly as it was.
pub fn plan_connect(
    settings: &SettingsSnapshot,
    config: &AppConfig,
    bridge_exe_path: &Path,
) -> Result<ConnectionPlan, ConnectError> {
    let before = read_status_line(&settings.text)?;
    let old_command = command_or_none(command_or_error(before)?);
    let after = status_line_json(&command_for(bridge_exe_path), before)?;

    // Already pointing at a bridge, this one or an older copy elsewhere (a Reconnect): never
    // chain to it, and keep what the config already saved, so
    // Disconnect still restores the user's own status line.
    let already_a_bridge = is_bridge(old_command.as_deref());
    if !already_a_bridge
        && let Some(before) = before
        && json_reader::read(before, Dialect::Strict).is_err()
    {
        return Err(ConnectError::StatusLineNotExact);
    }

    let new_config = if already_a_bridge {
        config.clone()
    } else {
        AppConfig {
            chained_command: old_command.clone(),
            previous: Some(PreviousStatusLine {
                raw_json: before.map(str::to_string),
            }),
            ..config.clone()
        }
    };

    Ok(ConnectionPlan {
        connect: true,
        settings: settings.clone(),
        before: before.map(str::to_string),
        new_settings_text: with_status_line(&settings.text, Some(&after))?,
        after: Some(after),
        chains: new_config.chained_command.is_some(),
        reconnects: already_a_bridge && !runs(old_command.as_deref(), bridge_exe_path),
        new_config,
    })
}

/// The plan to take the bridge out of the status line. Gives [`ConnectError::NotConnected`]
/// when the status line is not the bridge.
pub fn plan_disconnect(
    settings: &SettingsSnapshot,
    config: &AppConfig,
) -> Result<ConnectionPlan, ConnectError> {
    let before = read_status_line(&settings.text)?;
    if !is_bridge(command_or_error(before)?.as_deref()) {
        return Err(ConnectError::NotConnected);
    }

    // After an in-app Connect the saved statusLine comes back exactly. A bridge set up by
    // hand gets the chained command back in the current object (padding and the rest kept),
    // or loses statusLine.
    let after = match &config.previous {
        Some(previous) => previous.raw_json.clone(),
        None => match command_or_none(config.chained_command.clone()) {
            Some(chained) if !is_bridge(Some(&chained)) => {
                Some(status_line_json(&chained, before)?)
            }
            _ => None,
        },
    };

    Ok(ConnectionPlan {
        connect: false,
        settings: settings.clone(),
        before: before.map(str::to_string),
        new_settings_text: with_status_line(&settings.text, after.as_deref())?,
        after,
        new_config: AppConfig {
            chained_command: None,
            previous: None,
            ..config.clone()
        },
        chains: false,
        reconnects: false,
    })
}

/// Writes the plan, after checking that `settings.json` has not changed since it was read.
/// The old file is copied beside it first; the copy's path comes back, or `None` when there
/// was no file. A symbolic link is written through, with the backup beside the link.
///
/// `local_now` is the date and time on the user's clock, which names the backup.
pub fn apply(
    plan: &ConnectionPlan,
    config_path: &Path,
    local_now: NaiveDateTime,
) -> Result<Option<PathBuf>, ConnectError> {
    let path = &plan.settings.path;
    let current = read_settings(path)?;
    let target = target(path)?;
    if current.exists != plan.settings.exists
        || current.text != plan.settings.text
        || current.bom != plan.settings.bom
    {
        return Err(ConnectError::Changed);
    }

    let config_text = plan.new_config.to_json()?; // build everything before touching the disk

    let mut backup = None;
    if current.exists {
        let copy = backup_path(path, local_now);
        std::fs::copy(&target, &copy)?;
        backup = Some(copy);
    }

    // Connect saves the user's status line before it replaces it, and Disconnect puts it
    // back before it forgets it: whichever write fails, the status line is not lost.
    if plan.connect {
        atomic_file::write_all_text(config_path, &config_text)?;
        write_settings(&target, plan)?;
    } else {
        write_settings(&target, plan)?;
        atomic_file::write_all_text(config_path, &config_text)?;
    }
    Ok(backup)
}

/// Writes the plan's text to `settings.json`, after a byte order mark when the file had one.
fn write_settings(target: &Path, plan: &ConnectionPlan) -> io::Result<()> {
    if plan.settings.bom {
        atomic_file::write_all_text_with_bom(target, &plan.new_settings_text)
    } else {
        atomic_file::write_all_text(target, &plan.new_settings_text)
    }
}

/// `settings.json.sobreiro-<date>-<time>.bak` beside `settings.json`, or the same with `-1`,
/// `-2` and so on after the time when a backup of that second is already there.
fn backup_path(settings_path: &Path, local_now: NaiveDateTime) -> PathBuf {
    let stamp = local_now.format("%Y%m%d-%H%M%S");
    let named = |ending: String| {
        let mut name = OsString::from(settings_path);
        name.push(ending);
        PathBuf::from(name)
    };
    let mut candidate = named(format!(".sobreiro-{stamp}.bak"));
    let mut n = 1;
    while candidate.is_file() {
        candidate = named(format!(".sobreiro-{stamp}-{n}.bak"));
        n += 1;
    }
    candidate
}
