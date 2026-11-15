//! Ported from `tests/Sobreiro.Tests/GhCliTests.cs`: the PATH lookup. The C# tests that start
//! a process (a missing exe, a hanging one, a grandchild holding the pipe open) are not
//! ported, because Core starts none: the shell runs `gh`. The tests after
//! `the_path_lookup_finds_gh_exe_...` are new, and test what Core does with a finished run.
//!
//! Every token is made up, and no test runs `gh`.
use std::time::Duration;

use sobreiro_core::copilot::GitHubToken;
use sobreiro_core::copilot::gh_cli;

use crate::TestFolder;

fn fake() -> String {
    format!("gho_{}", "a".repeat(36))
}

fn values(tokens: Vec<GitHubToken>) -> Vec<String> {
    tokens
        .iter()
        .map(|token| token.value().to_string())
        .collect()
}

#[test]
fn the_path_lookup_finds_gh_exe_in_an_absolute_entry_and_skips_empty_and_relative_ones() {
    let folder = TestFolder::new("gh_cli-find");
    folder.create();
    let exe = folder.file("gh.exe");
    std::fs::write(&exe, "").unwrap();
    let dir = folder.path().to_str().unwrap();
    let missing = folder.file("missing");

    assert_eq!(gh_cli::find(Some(&format!(";relative;{dir}"))), Some(exe));
    assert_eq!(gh_cli::find(Some(";relative")), None);
    assert_eq!(gh_cli::find(Some(missing.to_str().unwrap())), None);
    assert_eq!(gh_cli::find(None), None);
}

#[test]
fn the_path_lookup_takes_the_first_entry_that_holds_gh_exe() {
    let first = TestFolder::new("gh_cli-first");
    let second = TestFolder::new("gh_cli-second");
    for folder in [&first, &second] {
        folder.create();
        std::fs::write(folder.file("gh.exe"), "").unwrap();
    }
    let path = format!(
        "{};{}",
        first.path().to_str().unwrap(),
        second.path().to_str().unwrap()
    );
    assert_eq!(gh_cli::find(Some(&path)), Some(first.file("gh.exe")));
}

#[test]
fn the_path_lookup_ignores_the_spaces_around_an_entry_and_a_folder_called_gh_exe() {
    let folder = TestFolder::new("gh_cli-spaces");
    folder.create();
    std::fs::write(folder.file("gh.exe"), "").unwrap();
    let padded = format!("  {}  ", folder.path().to_str().unwrap());
    assert_eq!(gh_cli::find(Some(&padded)), Some(folder.file("gh.exe")));

    let impostor = TestFolder::new("gh_cli-folder");
    std::fs::create_dir_all(impostor.file("gh.exe")).unwrap();
    assert_eq!(gh_cli::find(Some(impostor.path().to_str().unwrap())), None);
}

#[test]
fn gh_is_asked_for_the_token_of_github_com_in_five_seconds() {
    assert_eq!(
        gh_cli::ARGUMENTS,
        ["auth", "token", "--hostname", "github.com"]
    );
    assert_eq!(gh_cli::TIMEOUT, Duration::from_secs(5));
}

#[test]
fn a_printed_token_is_returned() {
    let output = format!("{}\r\n", fake());
    assert_eq!(
        values(gh_cli::tokens_from_run(Some(0), output.as_bytes())),
        [fake()]
    );
}

#[test]
fn a_non_zero_exit_gives_no_token() {
    let output = format!("{} \r\n", fake());
    assert!(gh_cli::tokens_from_run(Some(1), output.as_bytes()).is_empty());
    assert!(gh_cli::tokens_from_run(Some(-1), output.as_bytes()).is_empty());
}

#[test]
fn a_run_that_gave_no_exit_code_gives_no_token() {
    assert!(gh_cli::tokens_from_run(None, fake().as_bytes()).is_empty());
}

#[test]
fn no_token_printed_gives_no_token() {
    for output in [
        &b""[..],
        b"gh: not logged in\r\n",
        b"gho_",
        b"not a token: ghx_abc",
    ] {
        assert!(gh_cli::tokens_from_run(Some(0), output).is_empty());
    }
}

#[test]
fn only_the_first_token_is_returned_and_text_around_it_is_left_out() {
    let output = format!("note\r\n{}\r\nghp_second\r\n", fake());
    assert_eq!(
        values(gh_cli::tokens_from_run(Some(0), output.as_bytes())),
        [fake()]
    );
}

#[test]
fn output_that_is_not_valid_utf_8_ends_a_token_where_the_bad_byte_is() {
    let mut output = b"\xff junk ".to_vec();
    output.extend_from_slice(b"ghp_abc\xffdef");
    assert_eq!(
        values(gh_cli::tokens_from_run(Some(0), &output)),
        ["ghp_abc".to_string()]
    );
}
