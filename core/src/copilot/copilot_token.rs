//! A GitHub token, as Sobreiro holds it, and how one is found in what a sign-in left behind:
//! in a text, in the bytes of a sign-in entry, and in the environment.
//!
//! Nothing here reads anything by itself. The text, the bytes and the environment are handed
//! in by whoever looked them up, which is the work of the area that has Copilot's sign-in.
use std::fmt;

/// A GitHub token: the one that lets Sobreiro ask for Copilot's AI credits.
///
/// Nothing prints the token. Rust has two ways to turn a value into text: `Display`, which
/// `{}` and `to_string` use, and `Debug`, which `{:?}` uses. Most types get their `Debug`
/// from the line `#[derive(Debug)]`, which prints every field. This type writes both by hand,
/// and both say only that a token is there. So a larger value that holds a token and is
/// printed whole, in a log line or in a failed test, does not show it either.
///
/// A token cannot be copied as a token (the type has no `Clone`) nor compared with another
/// (no `PartialEq`): whoever finds one hands it to the request, and it is gone when the
/// request is. Its text can still be read, with `value`. That is for the request alone, and
/// nothing but this rule stops other code from asking.
pub struct GitHubToken {
    value: String,
}

impl GitHubToken {
    pub fn new(value: String) -> GitHubToken {
        GitHubToken { value }
    }

    /// The token itself. Only the request asks for it, to put it in its `Authorization` header.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// What both ways of printing a token give.
const REDACTED: &str = "GitHubToken(redacted)";

impl fmt::Display for GitHubToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl fmt::Debug for GitHubToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

/// How a GitHub token starts. `gho_` and `ghu_` are given to an app the user signed in to,
/// `ghp_` and `github_pat_` are tokens a user made by hand.
const STARTS: [&str; 4] = ["gho_", "ghu_", "ghp_", "github_pat_"];

/// The first GitHub token in the text: one of the four starts, and after it one or more
/// letters, digits and underscores of ASCII, as many as there are.
pub fn find(text: &str) -> Option<GitHubToken> {
    let bytes = text.as_bytes();
    // A character outside ASCII is two bytes or more in UTF-8, none of them an ASCII byte: it
    // ends a token, as it should.
    first_token(bytes.len(), |index| bytes.get(index).copied())
}

/// The first GitHub token in the bytes of a sign-in entry, read as UTF-8 and, when that finds
/// none, as UTF-16 with the low byte first (which is how Windows keeps text).
///
/// The bytes are searched where they lie. Nothing is decoded into a text of its own, so the
/// rest of the entry is never copied: the token is the one text made.
pub fn from_blob(blob: &[u8]) -> Option<GitHubToken> {
    let as_utf8 = first_token(blob.len(), |index| blob.get(index).copied());
    as_utf8.or_else(|| {
        // In UTF-16 a character is two bytes. A letter of ASCII is its own byte and then a
        // zero. Two bytes that are anything else are no part of a token, and the search needs
        // to know no more about them than that. A last byte without a second is left out.
        first_token(blob.len() / 2, |index| {
            let low = *blob.get(index * 2)?;
            let high = *blob.get(index * 2 + 1)?;
            (high == 0).then_some(low)
        })
    })
}

/// The tokens the environment holds: what `COPILOT_GITHUB_TOKEN`, `GH_TOKEN` and
/// `GITHUB_TOKEN` are set to, in that order, without the white space at their ends: spaces,
/// tabs, line ends and the like. A name that is not set or holds only white space gives none,
/// and a value that came up already is not given twice. A value is taken as it is, whatever
/// it looks like.
///
/// `get_env` looks a name up. Core does not read the environment by itself: the app hands in
/// the real one, a test a made-up one.
pub fn from_environment(get_env: impl Fn(&str) -> Option<String>) -> Vec<GitHubToken> {
    let mut tokens: Vec<GitHubToken> = Vec::new();
    for name in ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"] {
        let Some(value) = get_env(name) else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() && !tokens.iter().any(|token| token.value() == value) {
            tokens.push(GitHubToken::new(value.to_string()));
        }
    }
    tokens
}

/// The first token in a text of `length` characters, which is asked for one character at a
/// time: `byte_at` gives the character at an index as a byte, or `None` when it is past the
/// end or is no single byte. That way one search serves a text in UTF-8 and one in UTF-16,
/// and neither has to be copied first.
fn first_token(length: usize, byte_at: impl Fn(usize) -> Option<u8>) -> Option<GitHubToken> {
    for start in 0..length {
        for prefix in STARTS {
            let starts_here = prefix
                .bytes()
                .enumerate()
                .all(|(offset, byte)| byte_at(start + offset) == Some(byte));
            if !starts_here {
                continue;
            }
            // The token runs on for as long as its characters do.
            let first = start + prefix.len();
            let mut end = first;
            while byte_at(end).is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
                end += 1;
            }
            // A start with nothing after it is no token. The search goes on behind it.
            if end > first {
                let value = (start..end).filter_map(&byte_at).map(char::from).collect();
                return Some(GitHubToken::new(value));
            }
        }
    }
    None
}
