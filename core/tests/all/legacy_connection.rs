//! Ported from `tests/Sobreiro.Tests/LegacyConnectionTests.cs`.
//!
//! A status line that still runs the old `openheadroom-bridge` counts as this app's own.
#![cfg(windows)]
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime};
use sobreiro_core::config::{AppConfig, PreviousStatusLine};
use sobreiro_core::connect::bridge_command::{is_bridge, is_legacy_bridge};
use sobreiro_core::connect::connection::{
    apply, is_connected, plan_connect, plan_disconnect, read_settings, state,
};
use sobreiro_core::connect::settings_json::{command_of, read_status_line};
use sobreiro_core::connect::{ConnectionPlan, ConnectionState, SettingsSnapshot};

use crate::TestFolder;

const NEW_BRIDGE: &str = r"C:\Apps\Sobreiro\sobreiro-bridge.exe";
const NEW_BRIDGE_CMD: &str = "\"C:/Apps/Sobreiro/sobreiro-bridge.exe\"";
const OLD_BRIDGE_CMD: &str = "\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\"";

// What an in-app Connect with the old build left behind.
const USER_STATUS_LINE: &str =
    "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }";
const LEGACY_SETTINGS: &str = "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n";

/// The user's clock at the moment of every Connect and Disconnect in these tests.
fn now() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 9)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap()
}

/// One test's folder with the old build's `settings.json` in it. Gives back the folder, the
/// path of `settings.json` and that of `config.json`.
fn legacy_scene(name: &str) -> (TestFolder, PathBuf, PathBuf) {
    let folder = TestFolder::new(name);
    let settings_path = folder.path().join("claude").join("settings.json");
    let config_path = folder.path().join("data").join("config.json");
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    std::fs::write(&settings_path, LEGACY_SETTINGS).unwrap();
    (folder, settings_path, config_path)
}

/// The config the first-run migration carried over from the old build.
fn migrated_config() -> AppConfig {
    AppConfig {
        chained_command: Some("~/.claude/statusline.sh".to_string()),
        previous: Some(PreviousStatusLine {
            raw_json: Some(USER_STATUS_LINE.to_string()),
        }),
        ..AppConfig::default()
    }
}

#[test]
fn is_legacy_bridge_recognises_only_the_old_bridge() {
    let rows = [
        (Some("\"C:/x/openheadroom-bridge.exe\""), true),
        (Some("~/bin/OpenHeadroom-Bridge"), true),
        (Some("\"C:/x/sobreiro-bridge.exe\""), false),
        (Some("~/.claude/statusline.sh"), false),
        (None, false),
    ];
    for (command, expected) in rows {
        assert_eq!(is_legacy_bridge(command), expected, "{command:?}");
    }
}

#[test]
fn is_bridge_counts_the_old_bridge_as_our_own() {
    let rows = [
        ("\"C:/x/openheadroom-bridge.exe\"", true),
        ("\"C:/x/sobreiro-bridge.exe\"", true),
        ("~/.claude/statusline.sh", false),
    ];
    for (command, expected) in rows {
        assert_eq!(is_bridge(Some(command)), expected, "{command}");
    }
}

#[test]
fn state_tells_not_connected_connected_and_legacy_apart() {
    assert_eq!(
        state("{\"statusLine\":{\"command\":\"~/x.sh\"}}", None),
        ConnectionState::NotConnected
    );
    assert_eq!(state("{}", None), ConnectionState::NotConnected);
    assert_eq!(
        state(
            "{\"statusLine\":{\"command\":\"C:/a/sobreiro-bridge.exe\"}}",
            None
        ),
        ConnectionState::Connected
    );
    assert_eq!(state(LEGACY_SETTINGS, None), ConnectionState::Legacy);
    assert_eq!(state("not json", None), ConnectionState::NotConnected);
}

#[test]
fn is_connected_is_true_for_a_legacy_connection() {
    assert!(is_connected(LEGACY_SETTINGS));
}

#[test]
fn reconnect_points_the_status_line_at_the_new_bridge_and_keeps_the_original_backup() {
    let (_folder, settings_path, config_path) = legacy_scene("legacy-reconnect");
    let config = migrated_config();

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_connect(&settings, &config, Path::new(NEW_BRIDGE)).unwrap();

    assert!(plan.connect);
    assert!(plan.reconnects);
    assert!(plan.chains);
    assert_eq!(plan.new_config, config);
    assert_eq!(
        plan.new_config
            .previous
            .as_ref()
            .unwrap()
            .raw_json
            .as_deref(),
        Some(USER_STATUS_LINE)
    );
    assert_eq!(
        command_of(plan.after.as_deref()).as_deref(),
        Some(NEW_BRIDGE_CMD)
    );
    assert!(!plan.new_config.to_json().unwrap().contains(OLD_BRIDGE_CMD));

    let backup = apply(&plan, &config_path, now()).unwrap().unwrap();

    // the file as it was, before the change
    assert_eq!(std::fs::read_to_string(backup).unwrap(), LEGACY_SETTINGS);
    let written = std::fs::read_to_string(&settings_path).unwrap();
    assert_eq!(
        command_of(read_status_line(&written).unwrap()).as_deref(),
        Some(NEW_BRIDGE_CMD)
    );
    assert!(written.contains("\"padding\": 1"), "{written}");
    assert!(written.contains("\"theme\": \"dark\""), "{written}");
    let saved = AppConfig::load(&config_path).unwrap();
    assert_eq!(
        saved.previous.unwrap().raw_json.as_deref(),
        Some(USER_STATUS_LINE)
    );
    assert_eq!(
        saved.chained_command.as_deref(),
        Some("~/.claude/statusline.sh")
    );
}

#[test]
fn reconnect_without_a_saved_config_never_saves_the_old_bridge_as_the_users_own() {
    let (_folder, settings_path, _config_path) = legacy_scene("legacy-reconnect-no-config");

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_connect(&settings, &AppConfig::default(), Path::new(NEW_BRIDGE)).unwrap();

    assert!(plan.reconnects);
    assert_eq!(plan.new_config.chained_command, None);
    assert_eq!(plan.new_config.previous, None);
    assert!(!plan.chains);
}

#[test]
fn a_fresh_connect_is_not_a_reconnect() {
    let (_folder, settings_path, _config_path) = legacy_scene("legacy-fresh-connect");
    std::fs::write(
        &settings_path,
        format!("{{\"statusLine\":{USER_STATUS_LINE}}}"),
    )
    .unwrap();

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_connect(&settings, &AppConfig::default(), Path::new(NEW_BRIDGE)).unwrap();
    assert!(!plan.reconnects);
}

#[test]
fn disconnect_works_on_a_legacy_connection_and_restores_the_original_status_line() {
    let (_folder, settings_path, config_path) = legacy_scene("legacy-disconnect");

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_disconnect(&settings, &migrated_config()).unwrap();

    assert!(!plan.connect);
    assert_eq!(plan.after.as_deref(), Some(USER_STATUS_LINE));
    apply(&plan, &config_path, now()).unwrap();
    assert_eq!(
        std::fs::read_to_string(&settings_path).unwrap(),
        format!(
            "{{\n  \"model\": \"x\",\n  \"statusLine\": {USER_STATUS_LINE},\n  \"theme\": \"dark\"\n}}\n"
        )
    );
    assert_eq!(AppConfig::load(&config_path).unwrap().previous, None);
}

#[test]
fn disconnect_of_a_legacy_connection_with_no_saved_config_removes_the_status_line() {
    let (_folder, settings_path, _config_path) = legacy_scene("legacy-disconnect-no-config");

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_disconnect(&settings, &AppConfig::default()).unwrap();

    assert_eq!(plan.after, None);
}

#[test]
fn reconnect_then_disconnect_still_restores_the_original() {
    let (_folder, settings_path, config_path) = legacy_scene("legacy-reconnect-then-disconnect");
    let settings = read_settings(&settings_path).unwrap();
    let reconnect = plan_connect(&settings, &migrated_config(), Path::new(NEW_BRIDGE)).unwrap();
    apply(&reconnect, &config_path, now()).unwrap();

    let settings = read_settings(&settings_path).unwrap();
    let plan = plan_disconnect(&settings, &AppConfig::load(&config_path).unwrap()).unwrap();

    assert_eq!(plan.after.as_deref(), Some(USER_STATUS_LINE));
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
    assert_eq!(state(LEGACY_SETTINGS, own), ConnectionState::Legacy);
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
    assert_eq!(plan.new_config, migrated_config());
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
        &migrated_config(),
        Path::new(r"C:\Data\sobreiro-bridge.exe"),
    )
    .unwrap()
}
