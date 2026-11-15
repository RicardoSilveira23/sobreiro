//! The bridge as Claude Code runs it: the exe itself, given a status line's JSON on its input.
//! The C# had no tests of the exe; these are new.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use sobreiro_core::bridge::bash_locator;

const EXE: &str = env!("CARGO_BIN_EXE_sobreiro-bridge");

/// What Claude Code gives a status line command, with fields the bridge must never write.
const STATUS_LINE: &str = r#"{"session_id":"abc","transcript_path":"C:/x.jsonl","cwd":"C:/p","rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},"seven_day":{"used_percentage":41.2,"resets_at":1738857600},"spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly"}}}"#;

/// A data folder for one test, under the build's own temporary folder. It starts out empty.
fn data_folder(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// Runs the bridge with `input` on its stdin, a data folder of its own and an empty Claude
/// config folder, so no run of a test can read or write the user's.
fn run_bridge(data: &Path, input: &str, inside_chain: bool) -> Output {
    let claude_config = data.join("claude-config");
    std::fs::create_dir_all(&claude_config).unwrap();
    let mut command = Command::new(EXE);
    command
        .env("SOBREIRO_DATA_DIR", data)
        .env("CLAUDE_CONFIG_DIR", &claude_config)
        .env_remove("SOBREIRO_IN_CHAIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if inside_chain {
        command.env("SOBREIRO_IN_CHAIN", "1");
    }
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// The file with every time in it replaced by `T`, so two runs at different moments compare.
fn without_times(usage_json: &str, observed_at: i64) -> String {
    usage_json.replace(
        &format!("\"observed_at\":{observed_at}"),
        "\"observed_at\":T",
    )
}

#[test]
fn prints_the_default_line_and_keeps_only_the_limits() {
    let data = data_folder("bridge-exe-default-line");

    let output = run_bridge(&data, STATUS_LINE, false);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5h 23% · 7d 41% · spend 62%\n"
    );
    assert!(output.stderr.is_empty());

    let usage = std::fs::read_to_string(data.join("usage.json")).unwrap();
    for leaked in [
        "session_id",
        "transcript_path",
        "cwd",
        "abc",
        "C:/x.jsonl",
        "C:/p",
    ] {
        assert!(!usage.contains(leaked), "{leaked} is in {usage}");
    }
    // The file the C# bridge of 1.1.0 wrote for the same input on 2026-11-14, at the second
    // 1794676233. But for the times, this one is the same to the byte.
    let written_by_csharp = r#"{"observed_at":1794676233,"rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600,"observed_at":1794676233},"seven_day":{"used_percentage":41.2,"resets_at":1738857600,"observed_at":1794676233},"spend_limit":{"used_percentage":62.8,"resets_at":1740787200,"used_usd":314.12,"limit_usd":500,"period":"monthly","observed_at":1794676233}}}"#;
    let document: serde_json::Value = serde_json::from_str(&usage).unwrap();
    let observed_at = document["observed_at"].as_i64().unwrap();
    assert_eq!(
        without_times(&usage, observed_at),
        without_times(written_by_csharp, 1794676233)
    );
    // Nothing else was left in the data folder: no temporary file, no log.
    let mut names: Vec<String> = std::fs::read_dir(&data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["claude-config", "usage.json"]);
}

#[test]
fn prints_a_line_and_writes_nothing_when_the_input_is_not_a_status_line() {
    let data = data_folder("bridge-exe-not-json");

    let output = run_bridge(&data, "not json at all", false);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "5h – · 7d –\n");
    assert!(!data.join("usage.json").exists());
}

#[test]
fn runs_the_previous_status_line_command_and_passes_on_what_it_gives() {
    // Without Git Bash the command would run in PowerShell, which can take longer to start on
    // a CI runner than the bridge waits.
    let has_bash = bash_locator::find(
        |name| std::env::var(name).ok(),
        |file| Path::new(file).is_file(),
    )
    .is_some();
    if !has_bash {
        return;
    }
    let data = data_folder("bridge-exe-chain");
    std::fs::write(
        data.join("config.json"),
        r#"{"chained_command": "cat > /dev/null; echo theirs; exit 7"}"#,
    )
    .unwrap();

    let output = run_bridge(&data, STATUS_LINE, false);

    assert_eq!(output.status.code(), Some(7));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "theirs\n");
    assert!(data.join("usage.json").exists());
}

#[test]
fn does_not_chain_again_inside_a_chain() {
    let data = data_folder("bridge-exe-inside-chain");
    std::fs::write(
        data.join("config.json"),
        r#"{"chained_command": "echo theirs; exit 7"}"#,
    )
    .unwrap();

    let output = run_bridge(&data, STATUS_LINE, true);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "5h 23% · 7d 41% · spend 62%\n"
    );
}
