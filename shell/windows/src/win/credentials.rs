//! The Copilot CLI's sign-in in Windows Credential Manager, read as 1.1's `CredentialStore` reads
//! it. Target names are never logged or returned, each copy of a blob is zeroed after use, and a
//! refusal to list is told as a failure and not as finding nothing.
#![allow(unsafe_code)]

use sobreiro_core::copilot::GitHubToken;
use sobreiro_core::copilot::copilot_cli_credential::{
    self as cli, FILTER, ListingAnswer, after_failure, after_listing_everything_failed,
};
use sobreiro_core::copilot::copilot_sign_in::{SignInFailed, TokenSearch};
use sobreiro_core::copilot::copilot_token;
use std::ffi::c_void;
use windows::Win32::Security::Credentials::{
    CRED_TYPE_GENERIC, CREDENTIALW, CredEnumerateW, CredFree,
};
use windows::core::PCWSTR;

/// The Copilot CLI's tokens. `Err` when Credential Manager refuses a listing.
pub fn copilot_tokens(cli_has_been_used: bool) -> Result<TokenSearch, SignInFailed> {
    let filter: Vec<u16> = FILTER.encode_utf16().chain(Some(0)).collect();
    let listed = match List::of(PCWSTR(filter.as_ptr())) {
        Ok(list) => Some(list),
        Err(code) => match after_failure(code, cli_has_been_used) {
            ListingAnswer::NoEntry => None,
            // No target name starts with the entry's name. Older versions of the CLI may name
            // it differently, so every entry is listed and matched here.
            ListingAnswer::ListEverything => match List::of(PCWSTR::null()) {
                Ok(list) => Some(list),
                Err(code) => match after_listing_everything_failed(code) {
                    ListingAnswer::NoEntry => None,
                    _ => return Err(SignInFailed),
                },
            },
            ListingAnswer::Failed => return Err(SignInFailed),
        },
    };
    Ok(match listed {
        Some(list) => list.tokens(),
        None => TokenSearch {
            tokens: Vec::new(),
            entry_found: false,
        },
    })
}

/// What `CredEnumerateW` listed. Freed when dropped.
struct List {
    count: usize,
    entries: *mut *mut CREDENTIALW,
}

impl List {
    /// The entries whose target names match `filter` (all of them for a null filter). `Err` has
    /// the Win32 error code.
    fn of(filter: PCWSTR) -> Result<List, i32> {
        let mut count = 0u32;
        let mut entries: *mut *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: `filter` is null or a null-ended string that outlives the call, and the two
        // out values are alive for it.
        unsafe { CredEnumerateW(filter, None, &raw mut count, &raw mut entries) }
            .map_err(|error| error.code().0 & 0xFFFF)?;
        Ok(List {
            count: count as usize,
            entries,
        })
    }

    fn tokens(&self) -> TokenSearch {
        let mut tokens: Vec<GitHubToken> = Vec::new();
        let mut entry_found = false;
        for index in 0..self.count {
            // SAFETY: Windows listed `count` entries, and each pointer in the list is valid until
            // the list is freed, which happens when `self` is dropped.
            let Some(credential) = (unsafe { (*self.entries.add(index)).as_ref() }) else {
                continue;
            };
            if credential.Type != CRED_TYPE_GENERIC {
                continue;
            }
            // SAFETY: the target name is null or a null-ended string, alive with the list.
            let target = unsafe { credential.TargetName.to_string() }.ok();
            if !cli::is_ours(target.as_deref()) {
                continue;
            }
            entry_found = true;
            if credential.CredentialBlob.is_null() || credential.CredentialBlobSize == 0 {
                continue;
            }
            // SAFETY: the blob has `CredentialBlobSize` bytes, alive with the list.
            let mut blob = unsafe {
                std::slice::from_raw_parts(
                    credential.CredentialBlob,
                    credential.CredentialBlobSize as usize,
                )
            }
            .to_vec();
            if let Some(token) = copilot_token::from_blob(&blob)
                && !tokens.iter().any(|known| known.value() == token.value())
            {
                tokens.push(token);
            }
            blob.fill(0);
            std::hint::black_box(&blob);
        }
        TokenSearch {
            tokens,
            entry_found,
        }
    }
}

impl Drop for List {
    fn drop(&mut self) {
        // SAFETY: the list came from `CredEnumerateW` and is freed once, here.
        unsafe { CredFree(self.entries.cast::<c_void>().cast_const()) };
    }
}
