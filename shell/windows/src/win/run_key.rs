//! String values under a key of the current user's registry, for Start with Windows: the Run key
//! holds the command that starts the app at sign-in. The key's path is a parameter, so a test can
//! use a scratch key and never the real one. An error carries Windows' own text as its reason.
#![allow(unsafe_code)]

use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_PATH_NOT_FOUND, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW,
    RegGetValueW, RegSetKeyValueW,
};
use windows::core::{Error, HSTRING};

/// Windows' text for a status.
fn reason(status: WIN32_ERROR) -> String {
    Error::from(status.to_hresult()).message()
}

fn missing(status: WIN32_ERROR) -> bool {
    status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND
}

/// The string value `name` under `key`. None when the key or the value is missing. A value of
/// the kind that holds variables comes back expanded, as .NET's `GetValue` gives it.
pub fn read_string(key: &str, name: &str) -> Result<Option<String>, String> {
    let key = HSTRING::from(key);
    let name = HSTRING::from(name);
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ;
    // The value can change between asking its size and reading it, so a read that finds it grown
    // asks again with the new size.
    for _ in 0..8 {
        let mut size = 0u32;
        // SAFETY: only the size is asked for; `size` is alive for the call.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &key,
                &name,
                flags,
                None,
                None,
                Some(&raw mut size),
            )
        };
        if missing(status) {
            return Ok(None);
        }
        if status.0 != 0 {
            return Err(reason(status));
        }
        let mut text = vec![0u16; (size as usize).div_ceil(2) + 1];
        let mut room = (text.len() * 2) as u32;
        // SAFETY: `text` has `room` bytes, and both are alive for the call.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                &key,
                &name,
                flags,
                None,
                Some(text.as_mut_ptr().cast()),
                Some(&raw mut room),
            )
        };
        if missing(status) {
            return Ok(None);
        }
        if status == ERROR_MORE_DATA {
            continue;
        }
        if status.0 != 0 {
            return Err(reason(status));
        }
        let units = (room as usize / 2).min(text.len());
        let value = &text[..units];
        let end = value.iter().position(|unit| *unit == 0).unwrap_or(units);
        return Ok(Some(String::from_utf16_lossy(&value[..end])));
    }
    Err(reason(ERROR_MORE_DATA))
}

/// Writes the string value `name` under `key`, making the key when it is not there.
pub fn write_string(key: &str, name: &str, value: &str) -> Result<(), String> {
    let key = HSTRING::from(key);
    let name = HSTRING::from(name);
    // The terminating zero is part of what is stored.
    let data: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `data` is alive for the call and the size is its length in bytes.
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &key,
            &name,
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    };
    if status.0 == 0 {
        Ok(())
    } else {
        Err(reason(status))
    }
}

/// Deletes the value `name` under `key`. A value or key that is not there is not an error.
pub fn delete_value(key: &str, name: &str) -> Result<(), String> {
    let key = HSTRING::from(key);
    let name = HSTRING::from(name);
    // SAFETY: plain call with strings that are alive for it.
    let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, &key, &name) };
    if status.0 == 0 || missing(status) {
        Ok(())
    } else {
        Err(reason(status))
    }
}

/// Deletes `key` with everything under it, for the tests' scratch keys.
#[cfg(test)]
pub fn delete_key(key: &str) {
    use windows::Win32::System::Registry::{RegDeleteKeyW, RegDeleteTreeW};
    let key = HSTRING::from(key);
    // SAFETY: plain calls with a string that is alive for them.
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &key);
        let _ = RegDeleteKeyW(HKEY_CURRENT_USER, &key);
    }
}
