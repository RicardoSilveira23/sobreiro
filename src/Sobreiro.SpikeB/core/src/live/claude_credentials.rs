use super::AccessToken;
use crate::secret::wipe;
use crate::{Time, from_unix_millis};
use serde::Deserialize;
use std::io::{self, Read};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialStatus {
    Ok,
    Missing,
    Unreadable,
    Expired,
}

/// The token is Some only when the status is `CredentialStatus::Ok`. No `Debug`: the token has none.
pub struct CredentialRead {
    pub status: CredentialStatus,
    pub token: Option<AccessToken>,
}

/// Reads claudeAiOauth.accessToken and claudeAiOauth.expiresAt from Claude Code's .credentials.json, into memory
/// only. Nothing else in the file is looked at, and the token never goes into a message.
pub struct ClaudeCredentials;

// A sign-in file is a few hundred bytes; anything far bigger is not one.
const MAX_BYTES: u64 = 1 << 20;

#[derive(Deserialize)]
struct Root {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<Oauth>,
}

#[derive(Deserialize)]
struct Oauth {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "expiresAt", default)]
    expires_at: Option<i64>,
}

fn status(s: CredentialStatus) -> CredentialRead {
    CredentialRead { status: s, token: None }
}

impl ClaudeCredentials {
    pub fn read(path: &str, now: Time) -> CredentialRead {
        let mut bytes = Vec::new();
        // Opened shared for writing and deleting (std's default on Windows), so Claude Code can always replace the
        // file while it refreshes.
        let read = (|| -> io::Result<bool> {
            let file = std::fs::File::open(path)?;
            if file.metadata()?.len() > MAX_BYTES {
                return Ok(false);
            }
            (&file).take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
            Ok(true)
        })();
        let result = match read {
            Ok(true) => Self::parse(&bytes, now),
            Ok(false) => status(CredentialStatus::Unreadable),
            Err(e) if e.kind() == io::ErrorKind::NotFound => status(CredentialStatus::Missing),
            Err(_) => status(CredentialStatus::Unreadable),
        };
        wipe(&mut bytes);
        result
    }

    /// The same parse as `read`, from bytes (a BOM is dropped).
    pub fn parse(json: &[u8], now: Time) -> CredentialRead {
        let json = json.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(json);
        let Ok(root) = serde_json::from_slice::<Root>(json) else { return status(CredentialStatus::Unreadable) };
        let Some(oauth) = root.oauth else { return status(CredentialStatus::Unreadable) };
        let Some(value) = oauth.access_token.filter(|v| !v.is_empty()) else { return status(CredentialStatus::Unreadable) };

        let expires_at = match oauth.expires_at {
            None => None,
            Some(ms) => match from_unix_millis(ms) {
                Some(t) => Some(t),
                None => return status(CredentialStatus::Unreadable),
            },
        };

        if expires_at.is_some_and(|at| at <= now) {
            return status(CredentialStatus::Expired);
        }
        CredentialRead { status: CredentialStatus::Ok, token: Some(AccessToken::new(value, expires_at)) }
    }
}
