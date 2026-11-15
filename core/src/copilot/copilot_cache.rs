//! The Copilot CLI's own cache of what GitHub last told it:
//! `copilot\copilot-user-cache.json` in the local application data folder.
//!
//! The file starts with a few `// ...` comment lines, then holds
//! `{"copilotUserCache":{"v1:...":{"retrievedAt":"...","response":{...}}}}`. Of the entries
//! that hold a budget (a free seat's entry does not), the one retrieved last wins.
//!
//! Nothing here fails loudly: a file that is not there, cannot be opened, is cut short or is
//! not what the CLI writes gives no reading.
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::{CopilotReading, copilot_user_json};
use crate::connect::json_reader::{self, Dialect, Kind};
use crate::live::usage_api_json::time_in;

/// Where the CLI keeps its cache: in the folder `LOCALAPPDATA` names.
///
/// `get_env` looks an environment variable up. Core does not read the environment by itself:
/// the app hands in the real one, a test a made-up one. When `LOCALAPPDATA` is not set or is
/// empty, `local_app_data` is used instead. It is the folder the exe asks Windows for, as for
/// [`AppPaths::new`](crate::app_paths::AppPaths::new): Core makes no such call of its own.
pub fn default_path(get_env: impl Fn(&str) -> Option<String>, local_app_data: &Path) -> PathBuf {
    let local = match get_env("LOCALAPPDATA") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => local_app_data.to_path_buf(),
    };
    local.join("copilot").join("copilot-user-cache.json")
}

/// The reading in the text of a cache file, or `None` when it holds none.
pub fn parse(text: &str) -> Option<CopilotReading> {
    // The comment lines go: a line whose first characters, after any spaces, are `//`. A
    // text that starts with a byte order mark loses it too, as a file read as text would.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut json = String::with_capacity(text.len());
    for line in text.split('\n') {
        if !line.trim_start().starts_with("//") {
            if !json.is_empty() {
                json.push('\n');
            }
            json.push_str(line);
        }
    }

    let root = json_reader::read(&json, Dialect::Strict).ok()?;
    let Kind::Object(members) = &root.kind else {
        return None;
    };
    let Some(Kind::Object(entries)) = json_reader::member(members, "copilotUserCache") else {
        return None;
    };

    let mut best: Option<(CopilotReading, DateTime<Utc>)> = None;
    for entry in entries {
        let Kind::Object(fields) = &entry.value.kind else {
            continue;
        };
        // An entry with no usable time of its own is left out.
        let Some(Kind::Text(written)) = json_reader::member(fields, "retrievedAt") else {
            continue;
        };
        // A text that spells out half a character, here or in the response below, makes the
        // whole file of no use, as it did in 1.x, whatever the other entries hold.
        let retrieved_at = match time_in(written) {
            Ok(Some(retrieved_at)) => retrieved_at,
            Ok(None) => continue,
            Err(_) => return None,
        };
        let Some(response) = json_reader::member(fields, "response") else {
            continue;
        };
        let Ok(found) = copilot_user_json::from_cached_response(response, retrieved_at) else {
            return None;
        };
        let Some(reading) = found else {
            continue;
        };
        // The one retrieved last wins; of two retrieved together, the first.
        if best
            .as_ref()
            .is_none_or(|(_, best_at)| retrieved_at > *best_at)
        {
            best = Some((reading, retrieved_at));
        }
    }
    best.map(|(reading, _)| reading)
}

/// The most of the file that is read. The CLI's file is a few thousand bytes.
const MAX_BYTES: u64 = 1024 * 1024;

/// The reading in the cache file at `path`. A file that is not there, is locked, or cannot
/// be read gives `None`, like one that holds no reading, and so does a file over a megabyte:
/// another program writes this file, and Sobreiro does not load whatever size it finds.
pub fn read(path: &Path) -> Option<CopilotReading> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_BYTES {
        return None;
    }
    // `take` stops the read at the limit, for a file that grew since its size was asked. What
    // is cut short there is no longer JSON, and gives no reading.
    let mut bytes = Vec::new();
    file.take(MAX_BYTES).read_to_end(&mut bytes).ok()?;
    parse(&String::from_utf8_lossy(&bytes))
}
