//! Ported from `tests/Sobreiro.Tests/ConnectMessageTests.cs`.
use std::path::{Path, PathBuf};

use sobreiro_core::config::AppConfig;
use sobreiro_core::connect::connect_message::{RECONNECT_TITLE, confirm, done, title};
use sobreiro_core::connect::{ConnectionPlan, SettingsSnapshot};

const PATH: &str = r"C:\Users\me\.claude\settings.json";
const BRIDGE: &str =
    "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/sobreiro-bridge.exe\\\"\"\n  }";
const OLD: &str = "{ \"type\": \"command\", \"command\": \"bash ~/.claude/line.sh\" }";

fn plan(
    connect: bool,
    before: Option<&str>,
    after: Option<&str>,
    chains: bool,
    exists: bool,
) -> ConnectionPlan {
    ConnectionPlan {
        connect,
        settings: SettingsSnapshot {
            path: PathBuf::from(PATH),
            exists,
            text: if exists { "{}" } else { "" }.to_string(),
            bom: false,
        },
        before: before.map(str::to_string),
        after: after.map(str::to_string),
        new_settings_text: "{}".to_string(),
        new_config: AppConfig::default(),
        chains,
        reconnects: false,
    }
}

#[test]
fn titles_name_the_action() {
    assert_eq!(title(true), "Connect to Claude Code");
    assert_eq!(title(false), "Disconnect from Claude Code");
}

#[test]
fn connect_without_a_status_line_says_not_set_and_shows_the_bridge() {
    let text = confirm(&plan(true, None, Some(BRIDGE), false, true));
    assert_eq!(
        text,
        format!(
            "{PATH}\n\nstatusLine now: not set\nstatusLine after: {BRIDGE}\n\nA backup is saved first."
        )
    );
}

#[test]
fn connect_over_a_status_line_says_it_keeps_running() {
    let before = format!("  {OLD}\n");
    let text = confirm(&plan(true, Some(&before), Some(BRIDGE), true, true));
    assert_eq!(
        text,
        format!(
            "{PATH}\n\nstatusLine now: {OLD}\nstatusLine after: {BRIDGE}\n\nYour current status line keeps running: the bridge passes its output through.\n\nA backup is saved first."
        )
    );
}

#[test]
fn disconnect_without_a_previous_status_line_says_removed() {
    let text = confirm(&plan(false, Some(BRIDGE), None, false, true));
    assert_eq!(
        text,
        format!(
            "{PATH}\n\nstatusLine now: {BRIDGE}\nstatusLine after: removed\n\nA backup is saved first."
        )
    );
}

#[test]
fn no_backup_line_when_there_is_no_file_to_back_up() {
    let text = confirm(&plan(true, None, Some(BRIDGE), false, false));
    assert!(!text.contains("backup"), "{text}");
}

#[test]
fn done_texts() {
    assert_eq!(
        done(true, None, false),
        "Connected. The numbers arrive with Claude Code's next response."
    );
    assert_eq!(done(false, None, false), "Disconnected.");
}

#[test]
fn done_names_the_backup_when_there_is_one() {
    let backup = format!("{PATH}.sobreiro-20261109-143000.bak");
    assert_eq!(
        done(true, Some(Path::new(&backup)), false),
        format!(
            "Connected. The numbers arrive with Claude Code's next response.\n\nBackup: {backup}"
        )
    );
    assert_eq!(
        done(false, Some(Path::new(&backup)), false),
        format!("Disconnected.\n\nBackup: {backup}")
    );
}

#[cfg(windows)]
#[test]
fn confirm_for_a_reconnect_from_another_folder_says_so() {
    let text = confirm(&crate::elsewhere_connection::elsewhere_plan());
    assert!(
        text.contains("The status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses."),
        "{text}"
    );
}

// What follows has no C# test. The words are the C#'s.

#[test]
fn reconnect_has_its_own_words() {
    assert_eq!(RECONNECT_TITLE, "Reconnect to Claude Code");
    assert_eq!(
        done(true, None, true),
        "Reconnected. The numbers arrive with Claude Code's next response."
    );

    let elsewhere =
        "{ \"type\": \"command\", \"command\": \"\\\"C:/Old/sobreiro-bridge.exe\\\"\" }";
    let reconnect = ConnectionPlan {
        reconnects: true,
        ..plan(true, Some(elsewhere), Some(BRIDGE), true, true)
    };
    assert_eq!(
        confirm(&reconnect),
        format!(
            "{PATH}\n\nstatusLine now: {elsewhere}\nstatusLine after: {BRIDGE}\n\nThe status line runs a Sobreiro bridge from another folder. This points it at the one this Sobreiro uses.\n\nYour current status line keeps running: the bridge passes its output through.\n\nA backup is saved first."
        )
    );
}
