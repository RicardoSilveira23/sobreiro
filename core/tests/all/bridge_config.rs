//! Ported from `tests/Sobreiro.Tests/BridgeConfigTests.cs`.
use sobreiro_core::bridge::BridgeConfig;

use crate::TestFolder;

/// Loads a `config.json` holding `content`, or a path where there is no file.
fn load(test: &str, content: Option<&str>) -> BridgeConfig {
    let folder = TestFolder::new(test);
    let path = folder.file("config.json");
    if let Some(content) = content {
        folder.create();
        std::fs::write(&path, content).unwrap();
    }
    BridgeConfig::load(&path)
}

#[test]
fn load_reads_chained_command() {
    let config = load(
        "bridge-config-reads",
        Some(r#"{"chained_command": "npx ccusage statusline"}"#),
    );
    assert_eq!(
        config.chained_command.as_deref(),
        Some("npx ccusage statusline")
    );
}

#[test]
fn load_missing_file_is_empty() {
    assert_eq!(load("bridge-config-missing", None).chained_command, None);
}

#[test]
fn load_corrupt_or_wrong_type_is_empty() {
    let rows = [r#"{"chained_command": 5}"#, "not json", "[]"];
    for (number, content) in rows.into_iter().enumerate() {
        let config = load(&format!("bridge-config-corrupt-{number}"), Some(content));
        assert_eq!(config.chained_command, None, "{content}");
    }
}

#[test]
fn load_blank_command_is_null() {
    let config = load("bridge-config-blank", Some(r#"{"chained_command": "   "}"#));
    assert_eq!(config.chained_command, None);
}
