//! Claude Code's access token, as Sobreiro holds it: in memory, for one request.
use std::fmt;

use chrono::{DateTime, Utc};

/// Claude Code's OAuth access token, and the moment it stops working when the sign-in file
/// names one.
///
/// Nothing prints the token. Rust has two ways to turn a value into text: `Display`, which
/// `{}` and `to_string` use, and `Debug`, which `{:?}` uses. Most types get their `Debug`
/// from the line `#[derive(Debug)]`, which prints every field. This type writes both by hand,
/// and both say only that a token is there. So a larger value that holds a token and is
/// printed whole, in a log line or in a failed test, does not show it either.
///
/// There is no way to copy a token (the type has no `Clone`): whoever reads it hands it to
/// the request, and it is gone when the request is.
pub struct AccessToken {
    value: String,
    expires_at: Option<DateTime<Utc>>,
}

impl AccessToken {
    pub fn new(value: String, expires_at: Option<DateTime<Utc>>) -> AccessToken {
        AccessToken { value, expires_at }
    }

    /// The token itself. Only the request asks for it, to put it in its `Authorization` header.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// When the token stops working. `None` when the sign-in file does not say.
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }
}

/// What both ways of printing a token give.
const REDACTED: &str = "AccessToken(redacted)";

impl fmt::Display for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}
