//! Named events and mutexes, which is how two launches of Sobreiro on one data folder find each
//! other: the mutex says a copy runs, the event is the ask to show its details.
#![allow(unsafe_code)]

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, INFINITE, ReleaseMutex, SetEvent, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};
use windows::core::HSTRING;

/// An auto-reset event with a name, made or opened. A set that nobody waits for yet stays set
/// until the first wait takes it.
pub struct NamedEvent(HANDLE);

// SAFETY: an event handle may be used from any thread; Windows does the locking.
unsafe impl Send for NamedEvent {}

impl NamedEvent {
    /// Creates the event, or opens it when it exists. None when Windows refuses.
    pub fn create(name: &str) -> Option<NamedEvent> {
        let name = HSTRING::from(name);
        // SAFETY: no attributes; the name outlives the call; the handle is ours to close.
        unsafe { CreateEventW(None, false, false, &name) }
            .ok()
            .map(NamedEvent)
    }

    /// Signals the event. False when Windows refuses.
    pub fn set(&self) -> bool {
        // SAFETY: the handle is open for as long as `self` lives.
        unsafe { SetEvent(self.0) }.is_ok()
    }

    /// Waits until the event is signalled, with no limit. False when the wait fails.
    pub fn wait(&self) -> bool {
        // SAFETY: the handle is open for as long as `self` lives.
        unsafe { WaitForSingleObject(self.0, INFINITE) == WAIT_OBJECT_0 }
    }
}

impl Drop for NamedEvent {
    fn drop(&mut self) {
        // SAFETY: the handle is closed once, here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// A named mutex this launch holds. It is released when this is dropped, on the thread that took it.
pub struct NamedMutex(HANDLE);

/// What taking the mutex came to.
pub enum Taken {
    /// Nobody held it: this launch is the first.
    First(NamedMutex),
    /// Another copy holds it, or holds it where this launch may not look.
    Held,
    /// Windows would not make it for any other reason.
    Failed,
}

impl NamedMutex {
    pub fn take(name: &str) -> Taken {
        let name = HSTRING::from(name);
        // SAFETY: no attributes; the name outlives the call; the handle is ours to close.
        let made = unsafe { CreateMutexW(None, true, &name) };
        // SAFETY: read straight after the call, before anything else can change it.
        let last = unsafe { GetLastError() };
        match made {
            Ok(handle) if last == ERROR_ALREADY_EXISTS => {
                // SAFETY: the handle is ours and closed once, here.
                let _ = unsafe { CloseHandle(handle) };
                Taken::Held
            }
            Ok(handle) => Taken::First(NamedMutex(handle)),
            // The mutex exists but this launch may not open it, e.g. an elevated first copy.
            Err(error) if error.code() == ERROR_ACCESS_DENIED.to_hresult() => Taken::Held,
            Err(_) => Taken::Failed,
        }
    }
}

impl Drop for NamedMutex {
    fn drop(&mut self) {
        // SAFETY: the mutex is held by this thread, which drops it; the handle is closed once.
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

/// Lets any process take the foreground, so the first copy's window can come up when the ask
/// reaches it.
pub fn allow_any_foreground() {
    // SAFETY: no pointers; the result says nothing this launch can act on.
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
}
