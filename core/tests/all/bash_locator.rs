//! Ported from `tests/Sobreiro.Tests/BashLocatorTests.cs`. Windows paths, so Windows only.
#![cfg(windows)]
use std::collections::HashMap;

use sobreiro_core::bridge::bash_locator;

/// Looks for Git Bash on a made-up PC: `env` is its environment and `files` the files it has.
/// As on Windows, a file is found whatever the case of its name.
fn find(env: &[(&str, &str)], files: &[&str]) -> Option<String> {
    let env: HashMap<&str, &str> = env.iter().copied().collect();
    let existing: Vec<String> = files.iter().map(|file| file.to_uppercase()).collect();
    bash_locator::find(
        |name| env.get(name).map(|value| value.to_string()),
        |file| existing.contains(&file.to_uppercase()),
    )
}

#[test]
fn prefers_claude_code_git_bash_path_when_it_exists() {
    let found = find(
        &[
            ("CLAUDE_CODE_GIT_BASH_PATH", r"D:\git\bash.exe"),
            ("PATH", r"C:\Program Files\Git\usr\bin"),
        ],
        &[r"D:\git\bash.exe", r"C:\Program Files\Git\usr\bin\bash.exe"],
    );
    assert_eq!(found.as_deref(), Some(r"D:\git\bash.exe"));
}

#[test]
fn finds_bash_on_path() {
    let found = find(
        &[
            ("PATH", r"C:\Program Files\Git\usr\bin;C:\Windows\System32"),
            ("SystemRoot", r"C:\Windows"),
        ],
        &[
            r"C:\Program Files\Git\usr\bin\bash.exe",
            r"C:\Windows\System32\bash.exe",
        ],
    );
    assert_eq!(
        found.as_deref(),
        Some(r"C:\Program Files\Git\usr\bin\bash.exe")
    );
}

#[test]
fn skips_system32_and_windowsapps_bash() {
    let found = find(
        &[
            (
                "PATH",
                r"C:\Windows\System32\;C:\Users\me\AppData\Local\Microsoft\WindowsApps",
            ),
            ("SystemRoot", r"C:\Windows"),
        ],
        &[
            r"C:\Windows\System32\bash.exe",
            r"C:\Users\me\AppData\Local\Microsoft\WindowsApps\bash.exe",
        ],
    );
    assert_eq!(found, None);
}

#[test]
fn derives_bash_from_git_on_path() {
    let found = find(
        &[("PATH", r"C:\Windows;;C:\Program Files\Git\cmd")],
        &[
            r"C:\Program Files\Git\cmd\git.exe",
            r"C:\Program Files\Git\bin\bash.exe",
        ],
    );
    assert_eq!(found.as_deref(), Some(r"C:\Program Files\Git\bin\bash.exe"));
}

#[test]
fn falls_back_to_programfiles_git() {
    let found = find(
        &[
            ("PATH", r"C:\Windows"),
            ("ProgramFiles", r"C:\Program Files"),
        ],
        &[r"C:\Program Files\Git\bin\bash.exe"],
    );
    assert_eq!(found.as_deref(), Some(r"C:\Program Files\Git\bin\bash.exe"));
}

#[test]
fn returns_null_when_nothing_found() {
    let found = find(
        &[
            ("PATH", r"C:\Windows"),
            ("ProgramFiles", r"C:\Program Files"),
        ],
        &[],
    );
    assert_eq!(found, None);
}
