//! Ported from `tests/Sobreiro.Tests/ClaudePathsTests.cs`.
//!
//! The C# tests hand in a function standing in for the environment. Here the two variables
//! are handed in themselves, with the folder Windows names for the user.
use std::ffi::OsStr;
use std::path::Path;

use sobreiro_core::connect::ClaudePaths;

/// The paths with `CLAUDE_CONFIG_DIR` set to `config_dir` and `USERPROFILE` to `C:\Users\ana`.
/// The folder Windows would name is another one, so a test shows which of the two was used.
fn env(config_dir: Option<&str>) -> ClaudePaths {
    ClaudePaths::new(
        config_dir.map(OsStr::new),
        Some(OsStr::new(r"C:\Users\ana")),
        Path::new(r"C:\Users\from-windows"),
    )
}

#[cfg(windows)]
#[test]
fn credentials_file_sits_in_the_config_dir() {
    assert_eq!(
        env(Some(r"D:\cfg\claude")).credentials_file(),
        Path::new(r"D:\cfg\claude").join(".credentials.json")
    );
    assert_eq!(
        env(None).credentials_file(),
        Path::new(r"C:\Users\ana")
            .join(".claude")
            .join(".credentials.json")
    );
}

#[cfg(windows)]
#[test]
fn config_dir_variable_is_used_when_fully_qualified() {
    let paths = env(Some(r"D:\cfg\claude"));

    assert_eq!(paths.config_directory(), Path::new(r"D:\cfg\claude"));
    assert_eq!(
        paths.settings_file(),
        Path::new(r"D:\cfg\claude").join("settings.json")
    );
}

#[test]
fn relative_or_empty_config_dir_is_ignored() {
    for value in [None, Some(""), Some(r"relative\dir")] {
        let paths = env(value);

        assert_eq!(
            paths.config_directory(),
            Path::new(r"C:\Users\ana").join(".claude"),
            "{value:?}"
        );
        assert_eq!(
            paths.settings_file(),
            Path::new(r"C:\Users\ana")
                .join(".claude")
                .join("settings.json"),
            "{value:?}"
        );
    }
}

// Not in the C# tests.

/// The C# asks Windows for the user's folder when `USERPROFILE` is not set or is empty.
#[test]
fn without_the_user_profile_variable_the_folder_windows_names_is_used() {
    for variable in [None, Some(OsStr::new(""))] {
        let paths = ClaudePaths::new(None, variable, Path::new(r"C:\Users\from-windows"));

        assert_eq!(
            paths.config_directory(),
            Path::new(r"C:\Users\from-windows").join(".claude"),
            "{variable:?}"
        );
    }
}

/// A full path is judged as for Sobreiro's own folder: two separators at the start are
/// enough, as they are for the C#.
#[test]
fn a_config_dir_that_starts_with_two_separators_is_taken() {
    assert_eq!(
        env(Some(r"\\server")).config_directory(),
        Path::new(r"\\server")
    );
}

#[cfg(windows)]
#[test]
fn global_config_file_sits_in_the_overriding_folder_or_else_the_users_folder() {
    assert_eq!(
        env(Some(r"D:\cfg\claude")).global_config_file(),
        Path::new(r"D:\cfg\claude").join(".claude.json")
    );
    // Not inside `.claude`.
    assert_eq!(
        env(None).global_config_file(),
        Path::new(r"C:\Users\ana").join(".claude.json")
    );
    // A relative override is no override.
    assert_eq!(
        env(Some(r"relative\dir")).global_config_file(),
        Path::new(r"C:\Users\ana").join(".claude.json")
    );
}
