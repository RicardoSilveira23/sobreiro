//! A token held in memory for one request. It has no `Debug`, no `Display`, no `Serialize` and no `Clone`, and
//! its bytes are overwritten when it is dropped.

use std::ptr::write_volatile;

pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Secret {
        Secret(value)
    }

    /// The token's text, for the one request that sends it. Never put it in a message.
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: zero bytes are valid UTF-8, and the string is not read after this.
        unsafe { wipe(self.0.as_bytes_mut()) };
    }
}

/// Overwrites the bytes in a way the compiler may not remove.
pub(crate) fn wipe(bytes: &mut [u8]) {
    for b in bytes.iter_mut() {
        // SAFETY: `b` is a valid, aligned reference.
        unsafe { write_volatile(b, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}
