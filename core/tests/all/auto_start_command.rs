//! Ported from `tests/Sobreiro.Tests/AutoStartCommandTests.cs`.
use std::path::Path;

use sobreiro_core::startup::auto_start_command::{VALUE_NAME, command_for, matches};

const EXE: &str = r"C:\Apps\My Sobreiro\Sobreiro.exe";

fn exe() -> &'static Path {
    Path::new(EXE)
}

#[test]
fn value_name_is_fixed() {
    assert_eq!(VALUE_NAME, "Sobreiro");
}

#[test]
fn for_quotes_the_path() {
    assert_eq!(command_for(exe()), format!("\"{EXE}\""));
}

#[test]
fn matches_the_same_path_in_any_case_with_or_without_quotes() {
    for value in [
        r#""C:\Apps\My Sobreiro\Sobreiro.exe""#,
        r#""c:\apps\my sobreiro\sobreiro.EXE""#,
        r"c:\apps\my sobreiro\sobreiro.exe",
    ] {
        assert!(matches(Some(value), exe()), "{value}");
    }
}

#[test]
fn does_not_match_another_path_or_null() {
    assert!(!matches(Some(r#""C:\Other\Sobreiro.exe""#), exe()));
    assert!(!matches(None, exe()));
}

#[test]
fn matches_with_white_space_around_and_letters_outside_ascii() {
    let exe = Path::new(r"C:\Programas\Ação\Sobreiro.exe");
    assert!(matches(
        Some("  \"c:\\programas\\AÇÃO\\sobreiro.exe\"\t"),
        exe
    ));
    assert!(!matches(Some(r#""C:\Programas\Acao\Sobreiro.exe""#), exe));
}
