//! Reads Claude Code's access token out of its sign-in file, `.credentials.json`.
//!
//! Two values are taken from the file and no other: `claudeAiOauth.accessToken` and
//! `claudeAiOauth.expiresAt`. The refresh token sits in the same object and is never looked
//! at. The file is opened for reading only and never written, the token goes into no message,
//! and the bytes read from the disk are overwritten with zeros before [`read`] returns.
use std::borrow::Cow;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use chrono::{DateTime, Utc};

use super::AccessToken;
use crate::connect::json_reader::{self, Dialect, Kind};
use crate::io::wipe::wipe;

/// What reading the sign-in file came to. Only `Ok` carries a token.
#[derive(Debug)]
pub enum CredentialRead {
    /// The file holds a token whose time has not run out.
    Ok(AccessToken),
    /// There is no such file, or no such folder: Claude Code is not signed in on this PC.
    Missing,
    /// The file is there and gave no token: it cannot be opened now, it is not JSON, or it
    /// holds no token where one is expected.
    Unreadable,
    /// The token's time has run out. Claude Code renews it the next time it runs; Sobreiro
    /// never does.
    Expired,
}

/// A sign-in file is a few hundred bytes; anything far bigger is not one.
const MAX_BYTES: u64 = 1 << 20;

/// Reads the token from the sign-in file at `path`. `now` decides whether it has run out.
///
/// The file is read each time this is called: nothing is kept from one call to the next.
pub fn read(path: &Path, now: DateTime<Utc>) -> CredentialRead {
    let mut bytes = match read_bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return CredentialRead::Unreadable,
        Err(error) if is_missing(&error) => return CredentialRead::Missing,
        Err(_) => return CredentialRead::Unreadable,
    };
    let read = parse(&bytes, now);
    wipe(&mut bytes);
    read
}

/// The file's bytes, or `None` for a file over [`MAX_BYTES`].
///
/// Opening it does not stop anyone else from reading, writing, replacing or deleting it
/// meanwhile, so Claude Code can always renew its sign-in while Sobreiro reads.
fn read_bytes(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length > MAX_BYTES {
        return Ok(None);
    }
    // One block of memory of the file's size, filled in place: it never has to grow, so no
    // second copy of the file is left behind somewhere that `read` cannot wipe. `length` is a
    // megabyte at most, so it fits the type.
    let mut bytes = vec![0u8; length as usize];
    let mut filled = 0;
    while let Some(rest) = bytes.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        match file.read(rest) {
            // The file ended early: someone made it shorter since its size was asked.
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                wipe(&mut bytes);
                return Err(error);
            }
        }
    }
    bytes.truncate(filled);
    Ok(Some(bytes))
}

/// Whether opening failed because the file, or a folder on the way to it, is not there.
///
/// 1.x went by the two errors Windows has for that: number 2, "file not found", and number 3,
/// "path not found". Every other error (no permission, a folder in the file's place, a name no
/// file can have, a program holding the file for itself) is `Unreadable`.
#[cfg(windows)]
fn is_missing(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(2 | 3))
}

/// Other systems name the same thing in one way. Sobreiro runs on Windows only so far.
#[cfg(not(windows))]
fn is_missing(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
}

/// The three bytes that some programs put at the start of a UTF-8 file.
const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// The sign Rust puts where bytes are not UTF-8 text.
const UNKNOWN_CHARACTER: char = '\u{FFFD}';

fn parse(bytes: &[u8], now: DateTime<Utc>) -> CredentialRead {
    let bytes = bytes.strip_prefix(&BYTE_ORDER_MARK).unwrap_or(bytes);
    // JSON is text, and text in Rust is UTF-8. When the bytes are UTF-8, as they always are
    // when Claude Code wrote them, `from_utf8_lossy` hands the same bytes back without copying
    // them. When some are not, it makes a copy with the "unknown character" sign in their
    // place, and that copy is wiped below as the bytes themselves are.
    let text = String::from_utf8_lossy(bytes);
    let mended = matches!(text, Cow::Owned(_));
    let read = token_in(&text, mended, now);
    if let Cow::Owned(copy) = text {
        wipe(&mut copy.into_bytes());
    }
    read
}

/// Finds the token in the file's text. `mended` says that some bytes of the file were not
/// text and were replaced.
fn token_in(text: &str, mended: bool, now: DateTime<Utc>) -> CredentialRead {
    use CredentialRead::Unreadable;

    // The reader walks the text and copies nothing out of it: each value stays a piece of
    // `text` until someone asks what it says, and here only the access token is asked.
    let Ok(root) = json_reader::read(text, Dialect::Strict) else {
        return Unreadable;
    };
    let Kind::Object(root) = &root.kind else {
        return Unreadable;
    };
    let Some(Kind::Object(oauth)) = json_reader::member(root, "claudeAiOauth") else {
        return Unreadable;
    };
    let Some(Kind::Text(token)) = json_reader::member(oauth, "accessToken") else {
        return Unreadable;
    };
    // `None` for a token that spells out half a character, which no text can hold.
    let Some(value) = json_reader::text_of(token) else {
        return Unreadable;
    };
    // A token with bytes in it that were not text is not the token Claude Code was given.
    if value.is_empty() || (mended && value.contains(UNKNOWN_CHARACTER)) {
        return Unreadable;
    }

    let expires_at = match json_reader::member(oauth, "expiresAt") {
        None | Some(Kind::Null) => None,
        Some(Kind::Number(number)) => match expiry(number) {
            Some(at) => Some(at),
            None => return Unreadable,
        },
        Some(_) => return Unreadable,
    };

    if expires_at.is_some_and(|at| at <= now) {
        return CredentialRead::Expired;
    }
    CredentialRead::Ok(AccessToken::new(value, expires_at))
}

/// The first and the last millisecond 1.x could hold as a time: the start of the year 1 and
/// the end of the year 9999.
const FIRST_MILLISECOND: i64 = -62_135_596_800_000;
const LAST_MILLISECOND: i64 = 253_402_300_799_999;

/// `expiresAt` as a time. It is a count of whole milliseconds since 1970, and `number` is that
/// count as the file writes it. `None` for a number written with a point or an exponent
/// (`1.0`, `1e3`), which 1.x did not take either, and for a time outside the years 1 to 9999.
fn expiry(number: &str) -> Option<DateTime<Utc>> {
    let milliseconds: i64 = number.parse().ok()?;
    if !(FIRST_MILLISECOND..=LAST_MILLISECOND).contains(&milliseconds) {
        return None;
    }
    DateTime::from_timestamp_millis(milliseconds)
}
