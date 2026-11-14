use crate::secret::{Secret, wipe};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CopilotSourceKind {
    Env,
    CredentialManager,
    Gh,
}

/// What one source found. `entry_found` is for a source that saw a sign-in entry but no token in it. No `Debug`:
/// the tokens have none.
pub struct TokenSearch {
    pub tokens: Vec<Secret>,
    pub entry_found: bool,
}

impl TokenSearch {
    pub fn new(tokens: Vec<Secret>, entry_found: bool) -> TokenSearch {
        TokenSearch { tokens, entry_found }
    }
}

/// `implicit operator TokenSearch(List<string>)`.
impl From<Vec<Secret>> for TokenSearch {
    fn from(tokens: Vec<Secret>) -> Self {
        TokenSearch { tokens, entry_found: false }
    }
}

/// A source could not be read (the C# throws; the poll treats it as "error" and goes on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFailed;

/// One place to look for a GitHub token. `tokens` runs only when the poll reaches this source, on the poll's
/// worker thread, and may fail (the poll treats that as no token).
pub struct CopilotSignIn {
    pub kind: CopilotSourceKind,
    pub tokens: Box<dyn Fn() -> Result<TokenSearch, SourceFailed> + Send + Sync>,
}

impl CopilotSignIn {
    pub fn new(kind: CopilotSourceKind, tokens: impl Fn() -> Result<TokenSearch, SourceFailed> + Send + Sync + 'static) -> CopilotSignIn {
        CopilotSignIn { kind, tokens: Box::new(tokens) }
    }
}

pub struct CopilotToken;

const PREFIXES: [&[u8]; 4] = [b"gho", b"ghu", b"ghp", b"github_pat"];

fn is_token_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// The byte range of the first `(gho|ghu|ghp|github_pat)_[A-Za-z0-9_]+` in the text.
fn find_range(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        for prefix in PREFIXES {
            let rest = &bytes[i..];
            if rest.starts_with(prefix) && rest.get(prefix.len()) == Some(&b'_') {
                let start = i + prefix.len() + 1;
                let mut end = start;
                while end < bytes.len() && is_token_char(bytes[end]) {
                    end += 1;
                }
                if end > start {
                    return Some((i, end));
                }
            }
        }
    }
    None
}

impl CopilotToken {
    /// The first GitHub token in the text, or None.
    pub fn find(text: Option<&str>) -> Option<Secret> {
        let text = text.filter(|t| !t.is_empty())?;
        find_range(text).map(|(a, b)| Secret::new(text[a..b].to_string()))
    }

    /// Decodes a credential blob as UTF-8, then as UTF-16LE, and returns the first token found. The decoded text
    /// only ever lives in a buffer that is cleared before this returns: the token is the one string made.
    pub fn from_blob(blob: &[u8]) -> Option<Secret> {
        // UTF-8 first.
        let mut utf8 = String::from_utf8_lossy(blob).into_owned();
        let found = find_range(&utf8).map(|(a, b)| Secret::new(utf8[a..b].to_string()));
        // SAFETY: zero bytes are valid UTF-8, and the string is not read after this.
        unsafe { wipe(utf8.as_bytes_mut()) };
        if found.is_some() {
            return found;
        }

        // Then UTF-16LE (a trailing odd byte is ignored).
        let units: Vec<u16> = blob.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let mut utf16 = String::from_utf16_lossy(&units);
        let found = find_range(&utf16).map(|(a, b)| Secret::new(utf16[a..b].to_string()));
        // SAFETY: as above.
        unsafe { wipe(utf16.as_bytes_mut()) };
        let mut units = units;
        for u in units.iter_mut() {
            // SAFETY: a valid, aligned reference.
            unsafe { std::ptr::write_volatile(u, 0) };
        }
        found
    }

    /// COPILOT_GITHUB_TOKEN, GH_TOKEN, GITHUB_TOKEN in that order: trimmed, non-empty, distinct, as-is.
    pub fn from_environment(get_env: &dyn Fn(&str) -> Option<String>) -> Vec<Secret> {
        let mut seen: Vec<String> = Vec::new();
        let mut tokens = Vec::new();
        for name in ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"] {
            let Some(value) = get_env(name) else { continue };
            let trimmed = value.trim();
            if !trimmed.is_empty() && !seen.iter().any(|s| s == trimmed) {
                seen.push(trimmed.to_string());
                tokens.push(Secret::new(trimmed.to_string()));
            }
        }
        // The distinctness list held copies of the tokens: clear them.
        for s in seen.iter_mut() {
            // SAFETY: zero bytes are valid UTF-8, and the strings are not read after this.
            unsafe { wipe(s.as_bytes_mut()) };
        }
        tokens
    }
}
