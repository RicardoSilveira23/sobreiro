//! Ported from `tests/Sobreiro.Tests/BridgeRunnerTests.cs`, but for
//! `Still_prints_when_chain_throws`: a Rust function has no exception to throw. A chain that
//! fails gives `None`, which `falls_back_to_default_line_when_chain_cannot_start` covers.
use std::cell::RefCell;
use std::path::Path;

use chrono::{DateTime, Utc};
use sobreiro_core::bridge::{BridgeRunner, ChainResult};
use sobreiro_core::usage::usage_file;

use crate::{TestFolder, unix};

const WITH_LIMITS: &str = r#"{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/x","rate_limits":{"five_hour":{"used_percentage":23.4,"resets_at":1999999999},"seven_day":{"used_percentage":41,"resets_at":1999999999},"spend_limit":{"used_percentage":62,"resets_at":1999999999}}}"#;
const DEFAULT_LINE: &str = "5h 23% · 7d 41% · spend 62%\n";

fn now() -> DateTime<Utc> {
    unix(1_800_000_000)
}

/// A chain that cannot start: what the bridge gets when there is no command to run.
fn no_chain(_command: &str, _stdin: &str) -> Option<ChainResult> {
    None
}

/// Runs the bridge's flow over the files of `folder`. Gives the exit code and what was printed.
fn run(
    folder: &TestFolder,
    stdin: &str,
    chain: &dyn Fn(&str, &str) -> Option<ChainResult>,
) -> (i32, String) {
    run_with(folder, stdin, chain, &folder.file("usage.json"), false)
}

fn run_with(
    folder: &TestFolder,
    stdin: &str,
    chain: &dyn Fn(&str, &str) -> Option<ChainResult>,
    usage_path: &Path,
    inside_chain: bool,
) -> (i32, String) {
    let config_path = folder.file("config.json");
    let runner = BridgeRunner {
        usage_path,
        config_path: &config_path,
        run_chain: chain,
        inside_chain,
    };
    let mut printed = Vec::new();
    let code = runner.run(stdin, &mut printed, now());
    (code, String::from_utf8(printed).unwrap())
}

/// A folder whose `config.json` chains to the command `my-line`.
fn folder_with_chain(name: &str) -> TestFolder {
    let folder = TestFolder::new(name);
    folder.create();
    std::fs::write(
        folder.file("config.json"),
        r#"{"chained_command": "my-line"}"#,
    )
    .unwrap();
    folder
}

#[test]
fn writes_usage_and_prints_default_line_without_chain() {
    let folder = TestFolder::new("bridge-writes-usage");

    let (code, output) = run(&folder, WITH_LIMITS, &no_chain);

    assert_eq!(code, 0);
    assert_eq!(output, DEFAULT_LINE);
    let stored = usage_file::read(&folder.file("usage.json")).unwrap();
    assert_eq!(stored.observed_at, now());
}

#[test]
fn does_not_write_without_rate_limits() {
    let folder = TestFolder::new("bridge-no-rate-limits");

    let (code, output) = run(&folder, r#"{"session_id":"abc"}"#, &no_chain);

    assert_eq!(code, 0);
    assert_eq!(output, "5h – · 7d –\n");
    assert!(!folder.file("usage.json").exists());
}

#[test]
fn keeps_previous_usage_when_rate_limits_absent() {
    let folder = TestFolder::new("bridge-keeps-previous");
    run(&folder, WITH_LIMITS, &no_chain);
    let before = std::fs::read_to_string(folder.file("usage.json")).unwrap();

    run(&folder, "not json at all", &no_chain);

    assert_eq!(
        std::fs::read_to_string(folder.file("usage.json")).unwrap(),
        before
    );
}

#[test]
fn runs_chain_with_full_stdin_and_passes_output_and_exit_code() {
    let folder = folder_with_chain("bridge-runs-chain");
    let calls = RefCell::new(Vec::new());
    let chain = |command: &str, stdin: &str| {
        calls
            .borrow_mut()
            .push((command.to_string(), stdin.to_string()));
        Some(ChainResult {
            exit_code: 7,
            output: "theirs".to_string(),
        })
    };

    let (code, output) = run(&folder, WITH_LIMITS, &chain);

    assert_eq!(code, 7);
    assert_eq!(output, "theirs");
    assert_eq!(
        *calls.borrow(),
        [("my-line".to_string(), WITH_LIMITS.to_string())]
    );
    assert!(usage_file::read(&folder.file("usage.json")).is_some());
}

#[test]
fn does_not_run_chain_when_already_inside_a_chain() {
    let folder = folder_with_chain("bridge-inside-chain");
    let called = RefCell::new(false);
    let chain = |_command: &str, _stdin: &str| {
        *called.borrow_mut() = true;
        Some(ChainResult {
            exit_code: 7,
            output: "theirs".to_string(),
        })
    };

    let usage_path = folder.file("usage.json");
    let (code, output) = run_with(&folder, WITH_LIMITS, &chain, &usage_path, true);

    assert!(!*called.borrow());
    assert_eq!(code, 0);
    assert_eq!(output, DEFAULT_LINE);
    assert_eq!(usage_file::read(&usage_path).unwrap().observed_at, now());
}

#[test]
fn falls_back_to_default_line_when_chain_cannot_start() {
    let folder = folder_with_chain("bridge-chain-cannot-start");

    let (code, output) = run(&folder, WITH_LIMITS, &no_chain);

    assert_eq!(code, 0);
    assert_eq!(output, DEFAULT_LINE);
}

#[test]
fn still_prints_when_usage_write_fails() {
    let folder = TestFolder::new("bridge-write-fails");
    folder.create();

    // The usage file's path is an existing folder, so nothing can be written there.
    let (code, output) = run_with(&folder, WITH_LIMITS, &no_chain, folder.path(), false);

    assert_eq!(code, 0);
    assert_eq!(output, DEFAULT_LINE);
}

#[test]
fn usage_file_never_contains_other_stdin_fields() {
    let folder = TestFolder::new("bridge-no-other-fields");

    run(&folder, WITH_LIMITS, &no_chain);

    let text = std::fs::read_to_string(folder.file("usage.json")).unwrap();
    for leaked in ["session_id", "transcript_path", "cwd", "abc", "C:/x.jsonl"] {
        assert!(!text.contains(leaked), "{leaked} is in {text}");
    }
    // The two names, in this order, and nothing else at the top of the file.
    assert!(text.starts_with(r#"{"observed_at":1800000000,"rate_limits":{"#));
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(document.as_object().unwrap().len(), 2);
}

#[test]
fn run_does_not_lower_same_period_reading() {
    let folder = TestFolder::new("bridge-does-not-lower");
    run(&folder, &WITH_LIMITS.replace("23.4", "60"), &no_chain);

    let (_, output) = run(&folder, &WITH_LIMITS.replace("23.4", "40"), &no_chain);

    assert_eq!(output, "5h 40% · 7d 41% · spend 62%\n");
    let stored = usage_file::read(&folder.file("usage.json")).unwrap();
    assert_eq!(stored.limits.five_hour.unwrap().used_percentage, 60.0);
}
