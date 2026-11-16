//! Start with Windows: the `Sobreiro` value under the current user's Run key holds the command
//! that starts this exe at sign-in. Its name and form are Core's, shared with 1.x. The key's path
//! is a parameter so the tests never touch the real one.
use crate::win::run_key;
use sobreiro_core::startup::auto_start_command::{VALUE_NAME, command_for, matches};
use std::path::{Path, PathBuf};

/// The key Windows runs at sign-in.
pub const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// True when the value starts this exe. A read that fails counts as off.
pub fn is_on(key: &str, exe: &Path) -> bool {
    let value = run_key::read_string(key, VALUE_NAME).ok().flatten();
    matches(value.as_deref(), exe)
}

/// Turns it on for this exe, or off. Off removes the value only when it starts this exe, so
/// another copy's entry stays. The error is the reason, in words for the user.
pub fn set(key: &str, exe: &Path, on: bool) -> Result<(), String> {
    if on {
        run_key::write_string(key, VALUE_NAME, &command_for(exe))
    } else if is_on(key, exe) {
        run_key::delete_value(key, VALUE_NAME)
    } else {
        Ok(())
    }
}

/// The exe that is running.
pub fn this_exe() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|_| "Can't tell where Sobreiro.exe is.".to_string())
}

/// Whether Start with Windows is on for this exe, on the real Run key.
pub fn is_on_now() -> bool {
    this_exe().is_ok_and(|exe| is_on(RUN_KEY, &exe))
}

/// Flips Start with Windows for this exe, on the real Run key.
pub fn toggle() -> Result<(), String> {
    let exe = this_exe()?;
    set(RUN_KEY, &exe, !is_on(RUN_KEY, &exe))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Tells apart the scratch keys of tests that run at once in this process.
    static NEXT: AtomicU32 = AtomicU32::new(0);

    /// A scratch key under HKCU\Software, deleted whole when the guard drops, even after a failed
    /// assert.
    struct Scratch(String);

    impl Scratch {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let number = NEXT.fetch_add(1, Ordering::Relaxed);
            Scratch(format!(
                r"Software\SobreiroTest-{}-{nanos}-{number}",
                std::process::id()
            ))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            run_key::delete_key(&self.0);
        }
    }

    fn exe() -> PathBuf {
        PathBuf::from(r"C:\Programs\Sobreiro\Sobreiro.exe")
    }

    #[test]
    fn a_missing_key_or_value_reads_as_none_and_is_off() {
        let scratch = Scratch::new();
        assert_eq!(run_key::read_string(&scratch.0, VALUE_NAME), Ok(None));
        assert!(!is_on(&scratch.0, &exe()));
        run_key::write_string(&scratch.0, "Other", "x").unwrap();
        assert_eq!(run_key::read_string(&scratch.0, VALUE_NAME), Ok(None));
        assert!(!is_on(&scratch.0, &exe()));
    }

    #[test]
    fn a_string_is_written_and_read_back_whole() {
        let scratch = Scratch::new();
        let long = "é".repeat(5000);
        run_key::write_string(&scratch.0, "Long", &long).unwrap();
        assert_eq!(
            run_key::read_string(&scratch.0, "Long"),
            Ok(Some(long.clone()))
        );
        run_key::write_string(&scratch.0, "Empty", "").unwrap();
        assert_eq!(
            run_key::read_string(&scratch.0, "Empty"),
            Ok(Some(String::new()))
        );
    }

    #[test]
    fn on_writes_the_command_and_reads_as_on() {
        let scratch = Scratch::new();
        set(&scratch.0, &exe(), true).unwrap();
        assert_eq!(
            run_key::read_string(&scratch.0, VALUE_NAME),
            Ok(Some(command_for(&exe())))
        );
        assert!(is_on(&scratch.0, &exe()));
    }

    #[test]
    fn another_exes_value_is_off_and_stays_when_set_off() {
        let scratch = Scratch::new();
        let other = r#""C:\Elsewhere\Sobreiro.exe""#;
        run_key::write_string(&scratch.0, VALUE_NAME, other).unwrap();
        assert!(!is_on(&scratch.0, &exe()));
        set(&scratch.0, &exe(), false).unwrap();
        assert_eq!(
            run_key::read_string(&scratch.0, VALUE_NAME),
            Ok(Some(other.to_string()))
        );
    }

    #[test]
    fn off_removes_our_own_value() {
        let scratch = Scratch::new();
        set(&scratch.0, &exe(), true).unwrap();
        set(&scratch.0, &exe(), false).unwrap();
        assert_eq!(run_key::read_string(&scratch.0, VALUE_NAME), Ok(None));
        assert!(!is_on(&scratch.0, &exe()));
    }

    #[test]
    fn deleting_a_missing_value_or_key_is_fine() {
        let scratch = Scratch::new();
        assert_eq!(run_key::delete_value(&scratch.0, VALUE_NAME), Ok(()));
        run_key::write_string(&scratch.0, "Other", "x").unwrap();
        assert_eq!(run_key::delete_value(&scratch.0, VALUE_NAME), Ok(()));
        assert_eq!(set(&scratch.0, &exe(), false), Ok(()));
    }
}
