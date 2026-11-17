//! `alerts.json`: which alerts were already shown, so each fires once per window per reset
//! period. Written by hand, to the byte as 1.x wrote it.
use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::connect::json_reader::{self, Dialect, Kind};
use crate::io::{atomic_file, bounded_read};
use crate::limits::limit::LimitKind;

/// The highest threshold already alerted for a limit, and the reset period it was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlertMark {
    pub resets_at: Option<DateTime<Utc>>,
    pub threshold: i32,
}

/// The limits in the file, in the order the file lists them.
const KINDS: [LimitKind; 4] = [
    LimitKind::Session,
    LimitKind::Weekly,
    LimitKind::Spend,
    LimitKind::Credits,
];

const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The first and the last second that 1.x's `FromUnixTimeSeconds` takes: years 1 to 9999.
const FIRST_SECOND: i64 = -62_135_596_800;
const LAST_SECOND: i64 = 253_402_300_799;

fn key(kind: LimitKind) -> &'static str {
    match kind {
        LimitKind::Session => "session",
        LimitKind::Weekly => "weekly",
        LimitKind::Credits => "copilot_credits",
        LimitKind::Spend => "spend",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertState {
    pub marks: BTreeMap<LimitKind, AlertMark>,
}

impl AlertState {
    /// No alert shown yet.
    pub fn empty() -> AlertState {
        AlertState::default()
    }

    /// Missing, corrupt or wrong-shaped files give [`AlertState::empty`]. So does a file with a
    /// reset time that is no date: the whole file is then left unread, as 1.x did.
    pub fn load(path: &Path) -> AlertState {
        read(path).unwrap_or_default()
    }

    /// Writes the file as 1.x did: two-space indent, `\r\n` line ends, no byte order mark and
    /// no end of line after the last brace.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let entries: Vec<String> = KINDS
            .iter()
            .filter_map(|kind| {
                let mark = self.marks.get(kind)?;
                let resets_at = match mark.resets_at {
                    Some(at) => at.timestamp().to_string(),
                    None => "null".to_string(),
                };
                Some(format!(
                    "  \"{}\": {{\r\n    \"resets_at\": {resets_at},\r\n    \"threshold\": {}\r\n  }}",
                    key(*kind),
                    mark.threshold
                ))
            })
            .collect();
        let text = if entries.is_empty() {
            "{}".to_string()
        } else {
            format!("{{\r\n{}\r\n}}", entries.join(",\r\n"))
        };
        atomic_file::write_all_text(path, &text)
    }
}

/// A number as `TryGetInt32` and `TryGetInt64` read it: written as digits with an optional
/// minus, no fraction and no exponent, and small enough. `-0` is 0.
fn whole_number(kind: Option<&Kind>) -> Option<i64> {
    match kind? {
        Kind::Number(text) if !text.contains(['.', 'e', 'E']) => text.parse().ok(),
        _ => None,
    }
}

/// The marks in the file, or none when 1.x would have given up on it. The file is read as UTF-8
/// with its byte order mark dropped; a file in another encoding is no JSON and gives none.
fn read(path: &Path) -> Option<AlertState> {
    let bytes = bounded_read::read_file(path).ok()?;
    let bytes = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(&bytes);
    let text = String::from_utf8_lossy(bytes);
    let Kind::Object(root) = json_reader::read(&text, Dialect::Strict).ok()?.kind else {
        return None;
    };

    let mut marks = BTreeMap::new();
    for kind in KINDS {
        let Some(Kind::Object(entry)) = json_reader::member(&root, key(kind)) else {
            continue;
        };
        let Some(threshold) = whole_number(json_reader::member(entry, "threshold"))
            .and_then(|n| i32::try_from(n).ok())
        else {
            continue;
        };
        // A number that is no whole number of seconds leaves the reset time out of the mark; one
        // that is no date at all empties the file.
        let resets_at = match whole_number(json_reader::member(entry, "resets_at")) {
            Some(seconds) if (FIRST_SECOND..=LAST_SECOND).contains(&seconds) => {
                Some(DateTime::from_timestamp(seconds, 0)?)
            }
            Some(_) => return None,
            None => None,
        };
        marks.insert(
            kind,
            AlertMark {
                resets_at,
                threshold,
            },
        );
    }
    Some(AlertState { marks })
}
