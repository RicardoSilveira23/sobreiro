//! Where the process looks for DLLs it loads from now on.
#![allow(unsafe_code)]

use windows::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};

/// Makes every later DLL load come from System32 only, not from the exe's folder. A failure is
/// ignored: the link flag already covers the imports, and there is nothing to do about it.
pub fn system32_only() {
    // SAFETY: a plain call with a flag Windows defines; it touches no memory of ours.
    let _ = unsafe { SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32) };
}
