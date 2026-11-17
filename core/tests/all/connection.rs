//! Ported from `tests/Sobreiro.Tests/ConnectionTests.cs`.
//!
//! Every test works on a `settings.json` and a `config.json` in a folder of its own. Where the
//! C# expects an exception, the Rust gives an error back: the tests hold which one.
#![cfg(windows)]
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use sobreiro_core::config::{AppConfig, PreviousStatusLine};
use sobreiro_core::connect::bridge_command::status_line_json;
use sobreiro_core::connect::connection::{
    apply, is_connected, plan_connect, plan_disconnect, read_settings, state,
};
use sobreiro_core::connect::settings_json::{command_of, read_status_line};
use sobreiro_core::connect::{ConnectError, ConnectionPlan, ConnectionState, SettingsError};

use crate::TestFolder;

const BRIDGE: &str = r"C:\Apps\Sobreiro\sobreiro-bridge.exe";
const BRIDGE_CMD: &str = "\"C:/Apps/Sobreiro/sobreiro-bridge.exe\"";

const OLD_STATUS_LINE: &str =
    "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }";

/// A settings file with a status line of the user's own between two other keys.
fn original() -> String {
    format!(
        "{{\n  \"model\": \"x\",\n  \"statusLine\": {OLD_STATUS_LINE},\n  \"theme\": \"dark\"\n}}\n"
    )
}

/// The user's clock at the moment of every Connect and Disconnect in these tests.
fn now() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 9)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap()
}

/// One test's folder, with Claude Code's `settings.json` and Sobreiro's `config.json` in it.
struct Scene {
    folder: TestFolder,
}

impl Scene {
    /// `name` must be different in every test: tests run at the same time.
    fn new(name: &str) -> Scene {
        Scene {
            folder: TestFolder::new(name),
        }
    }

    fn settings_path(&self) -> PathBuf {
        self.folder.path().join("claude").join("settings.json")
    }

    fn config_path(&self) -> PathBuf {
        self.folder.path().join("data").join("config.json")
    }

    fn write_settings(&self, text: &str) {
        self.write_settings_bytes(text.as_bytes());
    }

    /// The text after the three bytes some editors put at the start of a UTF-8 file.
    fn write_settings_with_bom(&self, text: &str) {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        self.write_settings_bytes(&bytes);
    }

    fn write_settings_bytes(&self, bytes: &[u8]) {
        let path = self.settings_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn read_settings_text(&self) -> String {
        std::fs::read_to_string(self.settings_path()).unwrap()
    }

    /// The plan to connect, from `settings.json` as it is on the disk now.
    fn connect(&self, config: &AppConfig) -> Result<ConnectionPlan, ConnectError> {
        let settings = read_settings(&self.settings_path())?;
        plan_connect(&settings, config, Path::new(BRIDGE))
    }

    /// The plan to disconnect, from `settings.json` as it is on the disk now.
    fn disconnect(&self, config: &AppConfig) -> Result<ConnectionPlan, ConnectError> {
        let settings = read_settings(&self.settings_path())?;
        plan_disconnect(&settings, config)
    }

    fn load_config(&self) -> AppConfig {
        AppConfig::load(&self.config_path()).unwrap()
    }

    /// The backups beside `settings.json`.
    fn backups(&self) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(self.settings_path().parent().unwrap()) else {
            return Vec::new();
        };
        entries
            .map(|entry| entry.unwrap().path())
            .filter(|file| file.extension().is_some_and(|ext| ext == "bak"))
            .collect()
    }
}

/// A config that holds only a chained command.
fn chained(command: &str) -> AppConfig {
    AppConfig {
        chained_command: Some(command.to_string()),
        ..AppConfig::default()
    }
}

/// A settings file whose status line already runs a bridge, set up by hand.
const MANUAL: &str = "{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n";

#[test]
fn connect_chains_the_old_command_and_saves_the_old_status_line() {
    let scene = Scene::new("connection-chains");
    scene.write_settings(&original());
    let plan = scene.connect(&AppConfig::default()).unwrap();

    assert!(plan.connect);
    assert!(plan.chains);
    assert_eq!(
        plan.new_config.chained_command.as_deref(),
        Some("~/.claude/statusline.sh")
    );
    assert_eq!(
        plan.new_config
            .previous
            .as_ref()
            .unwrap()
            .raw_json
            .as_deref(),
        Some(OLD_STATUS_LINE)
    );
    assert_eq!(plan.before.as_deref(), Some(OLD_STATUS_LINE));
    assert_eq!(
        plan.after,
        Some(status_line_json(BRIDGE_CMD, Some(OLD_STATUS_LINE)).unwrap())
    );

    let backup = apply(&plan, &scene.config_path(), now()).unwrap();

    assert_eq!(
        backup,
        Some(
            scene
                .folder
                .path()
                .join("claude")
                .join("settings.json.sobreiro-20261109-143000.bak")
        )
    );
    assert_eq!(
        std::fs::read(backup.unwrap()).unwrap(),
        original().as_bytes()
    );
    let written = scene.read_settings_text();
    assert_eq!(
        command_of(read_status_line(&written).unwrap()).as_deref(),
        Some(BRIDGE_CMD)
    );
    assert!(written.contains("\"padding\": 1"), "{written}");
    assert!(
        written.starts_with("{\n  \"model\": \"x\",\n  \"statusLine\": {"),
        "{written}"
    );
    assert!(
        written.ends_with(",\n  \"theme\": \"dark\"\n}\n"),
        "{written}"
    );

    let config = scene.load_config();
    assert_eq!(
        config.chained_command.as_deref(),
        Some("~/.claude/statusline.sh")
    );
    assert_eq!(
        config.previous.unwrap().raw_json.as_deref(),
        Some(OLD_STATUS_LINE)
    );
}

#[test]
fn connect_with_no_status_line_has_no_previous_and_does_not_chain() {
    let scene = Scene::new("connection-no-status-line");
    scene.write_settings("{\n  \"model\": \"x\"\n}\n");
    let plan = scene.connect(&AppConfig::default()).unwrap();

    assert!(!plan.chains);
    assert_eq!(plan.before, None);
    assert_eq!(plan.new_config.chained_command, None);
    assert_eq!(
        plan.new_config.previous,
        Some(PreviousStatusLine { raw_json: None })
    );

    apply(&plan, &scene.config_path(), now()).unwrap();
    let config_text = std::fs::read_to_string(scene.config_path()).unwrap();
    assert!(
        config_text.contains("\"previous_status_line\": null"),
        "{config_text}"
    );
}

#[test]
fn connect_without_a_settings_file_creates_it_and_makes_no_backup() {
    let scene = Scene::new("connection-no-file");
    let settings = read_settings(&scene.settings_path()).unwrap();
    assert!(!settings.exists);
    assert_eq!(settings.text, "");
    assert!(!settings.bom);

    let plan = plan_connect(&settings, &AppConfig::default(), Path::new(BRIDGE)).unwrap();
    assert_eq!(apply(&plan, &scene.config_path(), now()).unwrap(), None);

    assert!(scene.settings_path().is_file());
    assert!(is_connected(&scene.read_settings_text()));
    assert!(scene.backups().is_empty());
}

#[test]
fn connect_when_already_pointing_at_a_bridge_keeps_the_saved_config_values() {
    let scene = Scene::new("connection-keeps-config");
    scene.write_settings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/old/sobreiro-bridge.exe\" }\n}\n");
    let saved = AppConfig {
        chained_command: Some("keep-me".to_string()),
        previous: Some(PreviousStatusLine {
            raw_json: Some("{\"command\":\"keep\"}".to_string()),
        }),
        ..AppConfig::default()
    };

    let plan = scene.connect(&saved).unwrap();

    assert_eq!(plan.new_config.chained_command.as_deref(), Some("keep-me"));
    assert_eq!(
        plan.new_config.previous.unwrap().raw_json.as_deref(),
        Some("{\"command\":\"keep\"}")
    );
    assert_eq!(
        command_of(plan.after.as_deref()).as_deref(),
        Some(BRIDGE_CMD)
    );
}

#[test]
fn connect_when_already_a_bridge_with_empty_config_does_not_chain_to_it() {
    let scene = Scene::new("connection-never-chains-to-a-bridge");
    scene.write_settings(
        "{\"statusLine\": { \"type\": \"command\", \"command\": \"C:/old/sobreiro-bridge.exe\" }}",
    );
    let plan = scene.connect(&AppConfig::default()).unwrap();

    assert_eq!(plan.new_config.chained_command, None);
    assert_eq!(plan.new_config.previous, None);
    assert!(!plan.chains);
}

#[test]
fn disconnect_restores_the_original_status_line_exactly_and_clears_the_config_keys() {
    let scene = Scene::new("connection-disconnect-restores");
    scene.write_settings(&original());
    let connect = scene.connect(&AppConfig::default()).unwrap();
    apply(&connect, &scene.config_path(), now()).unwrap();

    let plan = scene.disconnect(&scene.load_config()).unwrap();
    assert!(!plan.connect);
    assert_eq!(plan.after.as_deref(), Some(OLD_STATUS_LINE));

    let backup = apply(&plan, &scene.config_path(), now()).unwrap();

    assert_eq!(scene.read_settings_text(), original());
    assert!(backup.is_some());
    let config = scene.load_config();
    assert_eq!(config.chained_command, None);
    assert_eq!(config.previous, None);
}

#[test]
fn disconnect_after_a_connect_with_no_status_line_removes_it() {
    let scene = Scene::new("connection-disconnect-removes");
    let text = "{\n  \"model\": \"x\"\n}\n";
    scene.write_settings(text);
    let connect = scene.connect(&AppConfig::default()).unwrap();
    apply(&connect, &scene.config_path(), now()).unwrap();

    let plan = scene.disconnect(&scene.load_config()).unwrap();
    apply(&plan, &scene.config_path(), now()).unwrap();

    assert_eq!(scene.read_settings_text(), text);
}

#[test]
fn disconnect_of_a_manual_setup_removes_the_status_line() {
    let scene = Scene::new("connection-manual-removes");
    scene.write_settings("{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\" }\n}\n");
    let plan = scene.disconnect(&AppConfig::default()).unwrap();

    assert_eq!(plan.after, None);
    apply(&plan, &scene.config_path(), now()).unwrap();
    assert_eq!(scene.read_settings_text(), "{\n  \"model\": \"x\"\n}\n");
}

#[test]
fn disconnect_of_a_manual_setup_restores_the_chained_command() {
    let scene = Scene::new("connection-manual-restores-chained");
    scene.write_settings(MANUAL);
    let plan = scene.disconnect(&chained("x")).unwrap();

    apply(&plan, &scene.config_path(), now()).unwrap();

    let written = scene.read_settings_text();
    let status_line = read_status_line(&written).unwrap();
    assert_eq!(command_of(status_line).as_deref(), Some("x"));
    assert!(
        status_line.unwrap().contains("\"type\": \"command\""),
        "{written}"
    );
    assert_eq!(scene.load_config().chained_command, None);
}

#[test]
fn disconnect_never_restores_a_chained_bridge_command() {
    let scene = Scene::new("connection-never-restores-a-bridge");
    scene.write_settings(MANUAL);
    let plan = scene
        .disconnect(&chained("C:/other/sobreiro-bridge.exe"))
        .unwrap();
    assert_eq!(plan.after, None);
}

#[test]
fn disconnect_of_a_manual_setup_keeps_the_other_status_line_properties() {
    let scene = Scene::new("connection-manual-keeps-others");
    scene.write_settings("{\n  \"statusLine\": { \"type\": \"command\", \"command\": \"C:/b/sobreiro-bridge.exe\", \"padding\": 2 }\n}\n");
    let plan = scene.disconnect(&chained("x")).unwrap();
    apply(&plan, &scene.config_path(), now()).unwrap();

    let written = scene.read_settings_text();
    let status_line = read_status_line(&written).unwrap();
    assert_eq!(command_of(status_line).as_deref(), Some("x"));
    assert!(status_line.unwrap().contains("\"padding\": 2"), "{written}");
}

#[test]
fn disconnect_with_a_blank_chained_command_removes_the_status_line() {
    for (row, command) in ["", "   "].into_iter().enumerate() {
        let scene = Scene::new(&format!("connection-blank-chained-{row}"));
        scene.write_settings(MANUAL);
        let plan = scene.disconnect(&chained(command)).unwrap();
        assert_eq!(plan.after, None, "{command:?}");
    }
}

#[test]
fn connect_over_a_status_line_with_a_blank_command_does_not_chain() {
    for (row, command) in ["", "   "].into_iter().enumerate() {
        let scene = Scene::new(&format!("connection-blank-command-{row}"));
        let old = format!("{{ \"type\": \"command\", \"command\": \"{command}\" }}");
        scene.write_settings(&format!("{{\n  \"statusLine\": {old}\n}}\n"));
        let plan = scene.connect(&AppConfig::default()).unwrap();

        assert!(!plan.chains, "{command:?}");
        assert_eq!(plan.new_config.chained_command, None, "{command:?}");
        assert_eq!(
            plan.new_config.previous.unwrap().raw_json,
            Some(old),
            "{command:?}"
        );
    }
}

#[test]
fn a_settings_file_that_is_a_symbolic_link_stays_a_link() {
    let scene = Scene::new("connection-symbolic-link");
    let target = scene.folder.path().join("dotfiles").join("settings.json");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, original()).unwrap();
    let settings_path = scene.settings_path();
    std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
    if std::os::windows::fs::symlink_file(&target, &settings_path).is_err() {
        // No privilege to create symbolic links here: nothing to test against. On a
        // developer's PC that is how Windows is set up unless they changed it. CI may make
        // links, and there this test must not pass by testing nothing.
        assert!(
            std::env::var_os("CI").is_none(),
            "CI could not make a symbolic link"
        );
        return;
    }

    let plan = scene.connect(&AppConfig::default()).unwrap();
    assert_eq!(plan.settings.path, settings_path);
    assert_eq!(plan.before.as_deref(), Some(OLD_STATUS_LINE));
    let backup = apply(&plan, &scene.config_path(), now()).unwrap().unwrap();

    assert_eq!(std::fs::read_link(&settings_path).unwrap(), target);
    let at_target = std::fs::read_to_string(&target).unwrap();
    assert_eq!(
        command_of(read_status_line(&at_target).unwrap()).as_deref(),
        Some(BRIDGE_CMD)
    );
    assert_eq!(backup.parent(), settings_path.parent());
    assert!(!backup.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), original());

    let disconnect = scene.disconnect(&scene.load_config()).unwrap();
    apply(
        &disconnect,
        &scene.config_path(),
        now() + TimeDelta::seconds(1),
    )
    .unwrap();
    assert_eq!(std::fs::read_link(&settings_path).unwrap(), target);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), original());
}

#[test]
fn plan_disconnect_throws_when_not_connected() {
    let scene = Scene::new("connection-not-connected");
    scene.write_settings(&original());
    let result = scene.disconnect(&AppConfig::default());
    assert!(
        matches!(result, Err(ConnectError::NotConnected)),
        "{result:?}"
    );
}

#[test]
fn is_connected_reads_the_command_and_is_false_for_invalid_json() {
    assert!(is_connected(
        "{\"statusLine\": {\"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"}}"
    ));
    assert!(!is_connected(&original()));
    assert!(!is_connected("{oops"));
    assert!(!is_connected(""));
}

#[test]
fn apply_refuses_when_settings_changed_since_the_plan() {
    let scene = Scene::new("connection-changed");
    scene.write_settings(&original());
    let plan = scene.connect(&AppConfig::default()).unwrap();
    let changed = "{\n  \"model\": \"other\"\n}\n";
    scene.write_settings(changed);

    let result = apply(&plan, &scene.config_path(), now());
    assert!(matches!(result, Err(ConnectError::Changed)), "{result:?}");

    assert_eq!(scene.read_settings_text(), changed);
    assert!(!scene.config_path().exists());
    assert!(scene.backups().is_empty());
}

#[test]
fn apply_refuses_when_the_file_appeared_since_the_plan() {
    let scene = Scene::new("connection-appeared");
    let plan = scene.connect(&AppConfig::default()).unwrap();
    scene.write_settings(&original());

    let result = apply(&plan, &scene.config_path(), now());
    assert!(matches!(result, Err(ConnectError::Changed)), "{result:?}");
    assert_eq!(scene.read_settings_text(), original());
}

#[test]
fn a_second_backup_in_the_same_second_does_not_overwrite_the_first() {
    let scene = Scene::new("connection-second-backup");
    scene.write_settings(&original());
    let connect = scene.connect(&AppConfig::default()).unwrap();
    let first = apply(&connect, &scene.config_path(), now())
        .unwrap()
        .unwrap();
    let disconnect = scene.disconnect(&scene.load_config()).unwrap();
    let second = apply(&disconnect, &scene.config_path(), now())
        .unwrap()
        .unwrap();

    assert_ne!(first, second);
    assert!(
        second.to_string_lossy().ends_with("-143000-1.bak"),
        "{second:?}"
    );
    assert_eq!(std::fs::read(first).unwrap(), original().as_bytes());
}

#[test]
fn a_file_with_a_bom_keeps_its_bom() {
    let scene = Scene::new("connection-bom");
    scene.write_settings_with_bom("{\n  \"model\": \"café\"\n}\n");
    let settings = read_settings(&scene.settings_path()).unwrap();
    assert!(settings.bom);
    assert!(settings.text.starts_with('{'), "{}", settings.text);

    let plan = plan_connect(&settings, &AppConfig::default(), Path::new(BRIDGE)).unwrap();
    apply(&plan, &scene.config_path(), now()).unwrap();

    let bytes = std::fs::read(scene.settings_path()).unwrap();
    assert_eq!(bytes[..3], [0xEF, 0xBB, 0xBF]);
    assert_eq!(bytes[3], 0x7B);
    assert!(String::from_utf8(bytes).unwrap().contains("café"));
}

#[test]
fn invalid_json_throws_format_exception_and_leaves_the_file_alone() {
    let scene = Scene::new("connection-invalid-json");
    scene.write_settings("{oops");
    let result = scene.connect(&AppConfig::default());
    assert!(
        matches!(result, Err(ConnectError::Settings(SettingsError::NotJson))),
        "{result:?}"
    );
    assert_eq!(scene.read_settings_text(), "{oops");
}

#[test]
fn a_bridge_path_that_cannot_be_quoted_safely_stops_connect_before_anything_is_written() {
    let scene = Scene::new("connection-unsafe-path");
    scene.write_settings(&original());

    let settings = read_settings(&scene.settings_path()).unwrap();
    let result = plan_connect(
        &settings,
        &AppConfig::default(),
        Path::new(r"C:\Apps\$HOME\sobreiro-bridge.exe"),
    );
    assert!(
        matches!(result, Err(ConnectError::BridgePath(_))),
        "{result:?}"
    );

    assert_eq!(scene.read_settings_text(), original());
    assert!(!scene.config_path().exists());
    assert!(scene.backups().is_empty());
}

#[test]
fn a_comment_inside_the_old_status_line_stops_connect_before_anything_is_written() {
    let scene = Scene::new("connection-comment-inside");
    let text = "{\n  \"statusLine\": { // x\n \"type\": \"command\", \"command\": \"a\" }\n}\n";
    scene.write_settings(text);

    let result = scene.connect(&AppConfig::default());
    assert!(
        matches!(result, Err(ConnectError::StatusLineNotExact)),
        "{result:?}"
    );

    assert_eq!(scene.read_settings_text(), text);
    assert!(!scene.config_path().exists());
    assert!(scene.backups().is_empty());
}

#[test]
fn a_trailing_comma_inside_the_old_status_line_stops_connect() {
    let scene = Scene::new("connection-trailing-comma-inside");
    scene.write_settings("{\"statusLine\": { \"command\": \"a\", }}");
    let result = scene.connect(&AppConfig::default());
    assert!(
        matches!(result, Err(ConnectError::StatusLineNotExact)),
        "{result:?}"
    );
}

#[test]
fn a_config_that_cannot_be_serialised_leaves_no_backup() {
    let scene = Scene::new("connection-config-not-serialised");
    scene.write_settings(&original());
    let plan = ConnectionPlan {
        new_config: AppConfig {
            previous: Some(PreviousStatusLine {
                raw_json: Some("{ // bad\n}".to_string()),
            }),
            ..AppConfig::default()
        },
        ..scene.connect(&AppConfig::default()).unwrap()
    };

    let result = apply(&plan, &scene.config_path(), now());
    assert!(matches!(result, Err(ConnectError::Config(_))), "{result:?}");

    assert_eq!(scene.read_settings_text(), original());
    assert!(scene.backups().is_empty());
}

#[test]
fn a_file_that_is_not_valid_utf8_is_left_alone() {
    let scene = Scene::new("connection-not-utf8");
    let bytes = [0x7B, 0x22, 0x61, 0x22, 0x3A, 0x22, 0xE9, 0x22, 0x7D];
    scene.write_settings_bytes(&bytes);

    let result = read_settings(&scene.settings_path());
    assert!(matches!(result, Err(ConnectError::NotUtf8)), "{result:?}");
    assert_eq!(std::fs::read(scene.settings_path()).unwrap(), bytes);
}

#[test]
fn apply_refuses_when_a_bom_was_added_since_the_plan() {
    let scene = Scene::new("connection-bom-added");
    scene.write_settings(&original());
    let plan = scene.connect(&AppConfig::default()).unwrap();
    scene.write_settings_with_bom(&original());
    let before = std::fs::read(scene.settings_path()).unwrap();

    let result = apply(&plan, &scene.config_path(), now());
    assert!(matches!(result, Err(ConnectError::Changed)), "{result:?}");

    assert_eq!(std::fs::read(scene.settings_path()).unwrap(), before);
    assert!(!scene.config_path().exists());
}

// What follows has no C# test. Each was put to the C# of 1.1.0 on 2026-11-14, and holds what
// it did.

/// The first half of a character that takes two such codes, spelled out with no second half
/// after it: a backslash, `u` and `d800`. JSON can be written so, but no text can hold it.
const HALF: &str = concat!("\\", "ud800");

#[test]
fn half_a_character_in_the_command_stops_connect_and_disconnect() {
    let scene = Scene::new("connection-half-a-character-command");
    let text = format!(
        "{{\"statusLine\":{{\"type\":\"command\",\"command\":\"{HALF} sobreiro-bridge\"}}}}"
    );
    scene.write_settings(&text);

    // The C# stops here too, with an error of another kind and its own words.
    let connect = scene.connect(&AppConfig::default());
    assert!(
        matches!(connect, Err(ConnectError::Settings(SettingsError::NotJson))),
        "{connect:?}"
    );
    let disconnect = scene.disconnect(&AppConfig::default());
    assert!(
        matches!(
            disconnect,
            Err(ConnectError::Settings(SettingsError::NotJson))
        ),
        "{disconnect:?}"
    );
    // The tray of 1.x counts a status line it cannot read as not connected.
    assert_eq!(state(&text, None), ConnectionState::NotConnected);
    assert_eq!(scene.read_settings_text(), text);
    assert!(scene.backups().is_empty());
}

#[test]
fn half_a_character_in_another_key_of_the_status_line_stops_connect() {
    let scene = Scene::new("connection-half-a-character-other-key");
    let text = format!(
        "{{\"statusLine\":{{\"type\":\"command\",\"command\":\"x\",\"note\":\"{HALF}\"}}}}"
    );
    scene.write_settings(&text);

    // The other keys of the status line are written anew, and this one cannot be.
    let connect = scene.connect(&AppConfig::default());
    assert!(
        matches!(connect, Err(ConnectError::Settings(SettingsError::NotJson))),
        "{connect:?}"
    );
    assert_eq!(scene.read_settings_text(), text);
}

#[test]
fn half_a_character_outside_the_status_line_is_left_as_it_is() {
    let scene = Scene::new("connection-half-a-character-elsewhere");
    let text = format!("{{\"note\":\"{HALF}\",\"statusLine\":{OLD_STATUS_LINE}}}");
    scene.write_settings(&text);

    let plan = scene.connect(&AppConfig::default()).unwrap();
    apply(&plan, &scene.config_path(), now()).unwrap();

    let written = scene.read_settings_text();
    assert!(
        written.starts_with(&format!("{{\"note\":\"{HALF}\",\"statusLine\":{{\n")),
        "{written}"
    );
    let disconnect = scene.disconnect(&scene.load_config()).unwrap();
    apply(&disconnect, &scene.config_path(), now()).unwrap();
    assert_eq!(scene.read_settings_text(), text);
}
