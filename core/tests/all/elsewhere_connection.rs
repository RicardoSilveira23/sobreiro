//! Ported in part from `tests/Sobreiro.Tests/LegacyConnectionTests.cs`: the tests of a status
//! line that runs a Sobreiro bridge from another folder than this install's.
#![cfg(windows)]
use std::path::{Path, PathBuf};

use sobreiro_core::config::{AppConfig, PreviousStatusLine};
use sobreiro_core::connect::connection::{plan_connect, read_settings, state};
use sobreiro_core::connect::settings_json::command_of;
use sobreiro_core::connect::{ConnectionPlan, ConnectionState, SettingsSnapshot};

use crate::TestFolder;

const NEW_BRIDGE: &str = r"C:\Apps\Sobreiro\sobreiro-bridge.exe";

// The user's own status line, as an earlier Connect saved it.
const USER_STATUS_LINE: &str =
    "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }";

/// The config an earlier Connect left: the user's own status line, chained and saved.
fn saved_config() -> AppConfig {
    AppConfig {
        chained_command: Some("~/.claude/statusline.sh".to_string()),
        previous: Some(PreviousStatusLine {
            raw_json: Some(USER_STATUS_LINE.to_string()),
        }),
        ..AppConfig::default()
    }
}

#[test]
fn a_fresh_connect_is_not_a_reconnect() {
    let folder = TestFolder::new("elsewhere-fresh-connect");
    let settings_path = folder.path().join("claude").join("settings.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    std::fs::write(
        &settings_path,
        format!("{{\"statusLine\":{USER_STATUS_LINE}}}"),
    )
    .unwrap();

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_connect(&settings, &AppConfig::default(), Path::new(NEW_BRIDGE)).unwrap();
    assert!(!plan.reconnects);
}

/// A settings file whose only key is a status line running `command`, with each quote in it
/// written as its code, as the C# test's JSON writer does. The commands used here hold no
/// other character that JSON would have to write differently.
fn settings(command: &str) -> String {
    let escaped = command.replace('"', "\\u0022");
    format!("{{\"statusLine\":{{\"type\":\"command\",\"command\":\"{escaped}\"}}}}")
}

#[test]
fn state_with_an_own_bridge_tells_connected_from_elsewhere() {
    let own = Some(Path::new(r"C:\Data\sobreiro-bridge.exe"));
    assert_eq!(
        state(&settings("\"C:/Data/sobreiro-bridge.exe\""), own),
        ConnectionState::Connected
    );
    assert_eq!(
        state(
            &settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""),
            own
        ),
        ConnectionState::Elsewhere
    );
    assert_eq!(
        state(&settings("~/x.sh"), own),
        ConnectionState::NotConnected
    );
    assert_eq!(
        state(
            &settings("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""),
            None
        ),
        ConnectionState::Connected
    );
}

#[test]
fn plan_connect_from_another_sobreiro_bridge_is_a_reconnect_that_keeps_the_config() {
    let plan = elsewhere_plan();

    assert!(plan.reconnects);
    assert_eq!(plan.new_config, saved_config());
    assert_eq!(
        command_of(plan.after.as_deref()).as_deref(),
        Some("\"C:/Data/sobreiro-bridge.exe\"")
    );
}

/// The plan to connect when the status line runs a Sobreiro bridge from another folder.
/// The tests of the Connect flow's words use it too.
pub(crate) fn elsewhere_plan() -> ConnectionPlan {
    let settings = SettingsSnapshot {
        path: PathBuf::from(r"C:\c\settings.json"),
        exists: true,
        text: settings("\"C:/Old/sobreiro-bridge.exe\""),
        bom: false,
    };
    plan_connect(
        &settings,
        &saved_config(),
        Path::new(r"C:\Data\sobreiro-bridge.exe"),
    )
    .unwrap()
}
