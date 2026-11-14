//! Reads the Copilot CLI's sign-in from Windows Credential Manager (src/Sobreiro/Copilot/CredentialStore.cs).
//! Target names are never logged or returned, each blob copy is zeroed after use, and the decoded text is cleared
//! by `CopilotToken::from_blob`.

use super::{CopilotCliCredential, CopilotToken, ListingAnswer, SourceFailed, TokenSearch};
use crate::secret::{Secret, wipe};
use std::path::Path;
use windows::Win32::Security::Credentials::{CREDENTIALW, CRED_TYPE_GENERIC, CredEnumerateW, CredFree};
use windows::core::{PCWSTR, w};

pub struct CredentialStore;

impl CredentialStore {
    /// The Copilot CLI's tokens. Fails when Credential Manager refuses a listing, so the poll logs "error" and not
    /// "none": a refusal is not the same as finding nothing.
    pub fn copilot_tokens() -> Result<TokenSearch, SourceFailed> {
        let mut count: u32 = 0;
        let mut list: *mut *mut CREDENTIALW = std::ptr::null_mut();

        // SAFETY: the filter is a static wide string; count and list are live locals.
        let first = unsafe { CredEnumerateW(w!("copilot-cli*"), None, &mut count, &mut list) };
        if let Err(e) = first {
            match CopilotCliCredential::after_failure(win32_code(&e), copilot_cli_has_been_used()) {
                ListingAnswer::NoEntry => return Ok(TokenSearch::new(Vec::new(), false)),
                ListingAnswer::ListEverything => {
                    // No target name starts with copilot-cli. Older versions of the CLI may name it differently,
                    // so list everything and match here.
                    // SAFETY: a null filter lists every entry; count and list are live locals.
                    if let Err(e2) = unsafe { CredEnumerateW(PCWSTR::null(), None, &mut count, &mut list) } {
                        return match CopilotCliCredential::after_listing_everything_failed(win32_code(&e2)) {
                            ListingAnswer::NoEntry => Ok(TokenSearch::new(Vec::new(), false)),
                            _ => Err(SourceFailed),
                        };
                    }
                }
                ListingAnswer::Failed => return Err(SourceFailed),
            }
        }

        let mut tokens: Vec<Secret> = Vec::new();
        let mut entry_found = false;
        for i in 0..count as usize {
            // SAFETY: CredEnumerateW gave `count` valid entry pointers, which live until CredFree.
            let credential = unsafe { &**list.add(i) };
            if credential.Type != CRED_TYPE_GENERIC {
                continue;
            }
            // SAFETY: TargetName is a NUL-terminated wide string or null.
            let target = if credential.TargetName.is_null() { None } else { unsafe { credential.TargetName.to_string().ok() } };
            if !CopilotCliCredential::is_ours(target.as_deref()) {
                continue;
            }
            entry_found = true;
            if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
                continue;
            }

            // SAFETY: the blob holds CredentialBlobSize bytes.
            let mut blob = unsafe { std::slice::from_raw_parts(credential.CredentialBlob, credential.CredentialBlobSize as usize) }.to_vec();
            if let Some(token) = CopilotToken::from_blob(&blob) {
                if !tokens.iter().any(|t| t.expose() == token.expose()) {
                    tokens.push(token);
                }
            }
            wipe(&mut blob);
        }
        // SAFETY: `list` came from CredEnumerateW and is freed once.
        unsafe { CredFree(list as *const _) };
        Ok(TokenSearch::new(tokens, entry_found))
    }
}

/// The Win32 error behind a windows-crate error (its HRESULT's low 16 bits when it wraps a Win32 error).
fn win32_code(e: &windows::core::Error) -> i32 {
    (e.code().0 as u32 & 0xFFFF) as i32
}

fn copilot_cli_has_been_used() -> bool {
    let folder = |variable: &str, fallback_under_profile: &[&str]| -> std::path::PathBuf {
        match std::env::var(variable) {
            Ok(v) if !v.is_empty() => v.into(),
            _ => {
                let mut p = std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default());
                for part in fallback_under_profile {
                    p.push(part);
                }
                p
            }
        }
    };
    Path::new(&folder("LOCALAPPDATA", &["AppData", "Local"])).join("copilot").is_dir()
        || Path::new(&folder("USERPROFILE", &[])).join(".copilot").is_dir()
}
