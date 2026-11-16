//! Where Windows puts its folders: the two Core's path types ask the exe for, and the Windows
//! folder itself.
#![allow(unsafe_code)]

use std::ffi::{OsString, c_void};
use std::path::PathBuf;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_LocalAppData, FOLDERID_Profile, FOLDERID_Windows, KF_FLAG_DEFAULT,
    SHGetKnownFolderPath,
};
use windows::core::GUID;

/// The user's local application data folder.
pub fn local_app_data() -> Option<PathBuf> {
    known(&FOLDERID_LocalAppData)
}

/// The user's own folder.
pub fn user_profile() -> Option<PathBuf> {
    known(&FOLDERID_Profile)
}

/// The Windows folder, which holds Explorer.
pub fn windows() -> Option<PathBuf> {
    known(&FOLDERID_Windows)
}

fn known(id: &GUID) -> Option<PathBuf> {
    // SAFETY: `id` is alive for the call; no token is passed, so it is the current user's folder.
    let path = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;
    // SAFETY: Windows gave a null-ended string that is ours to free once, after it is copied.
    let text = unsafe { path.as_wide() };
    let folder = PathBuf::from(<OsString as std::os::windows::ffi::OsStringExt>::from_wide(
        text,
    ));
    // SAFETY: the string came from `SHGetKnownFolderPath`, which asks for it to be freed so.
    unsafe { CoTaskMemFree(Some(path.0.cast::<c_void>().cast_const())) };
    Some(folder)
}
