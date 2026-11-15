//! Ported from `tests/Sobreiro.Tests/AppConfigTests.cs`.
use std::path::PathBuf;

use sobreiro_core::bridge::BridgeConfig;
use sobreiro_core::colours::BarColourOption;
use sobreiro_core::config::{AppConfig, ConfigError, PreviousStatusLine, ShowOption};
use sobreiro_core::display::{FlyoutStyle, PaceDays};

use crate::TestFolder;

/// A folder for the test named `test`, and the path of the `config.json` in it. Neither exists
/// until the test, or the code under test, writes it.
fn folder(test: &str) -> (TestFolder, PathBuf) {
    let folder = TestFolder::new(&format!("app-config-{test}"));
    let path = folder.file("config.json");
    (folder, path)
}

/// A `config.json` holding `text`. The folder must be kept until the test ends.
fn write(test: &str, text: &str) -> (TestFolder, PathBuf) {
    write_bytes(test, text.as_bytes())
}

fn write_bytes(test: &str, bytes: &[u8]) -> (TestFolder, PathBuf) {
    let (folder, path) = folder(test);
    folder.create();
    std::fs::write(&path, bytes).unwrap();
    (folder, path)
}

/// Loads a `config.json` holding `text`.
fn load(test: &str, text: &str) -> Result<AppConfig, ConfigError> {
    let (_folder, path) = write(test, text);
    AppConfig::load(&path)
}

/// The raw JSON kept for a key Sobreiro does not know.
fn extra<'a>(config: &'a AppConfig, key: &str) -> Option<&'a str> {
    config
        .extra
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, raw)| raw.as_str())
}

fn assert_default(config: &AppConfig) {
    assert_eq!(config.chained_command, None);
    assert_eq!(config.previous, None);
    assert_eq!(config.style, FlyoutStyle::Bars);
    assert_eq!(config.bar_colour, BarColourOption::Purple);
    assert_eq!(config.show, ShowOption::TrayAndPill);
    assert_eq!(config.pace_days, PaceDays::EveryDay);
    assert!(config.extra.is_empty());
}

fn assert_not_json(result: &Result<AppConfig, ConfigError>, row: &str) {
    assert!(matches!(result, Err(ConfigError::NotJson)), "{row}");
}

#[test]
fn pace_defaults_to_every_day() {
    let (_folder, path) = folder("pace-default");

    assert_eq!(
        AppConfig::load(&path).unwrap().pace_days,
        PaceDays::EveryDay
    );
}

#[test]
fn pace_loads_and_unknown_values_fall_back() {
    let rows = [
        (r#""weekdays""#, PaceDays::Weekdays),
        (r#""Weekdays""#, PaceDays::Weekdays),
        (r#""everyday""#, PaceDays::EveryDay),
        (r#""workdays""#, PaceDays::EveryDay),
        ("5", PaceDays::EveryDay),
        ("null", PaceDays::EveryDay),
    ];
    for (number, (json, expected)) in rows.into_iter().enumerate() {
        let text = format!(r#"{{"pace":{json}}}"#);

        let loaded = load(&format!("pace-loads-{number}"), &text).unwrap();

        assert_eq!(loaded.pace_days, expected, "{json}");
        assert_eq!(extra(&loaded, "pace"), None, "{json}");
    }
}

#[test]
fn pace_round_trips_and_is_written_lower_case() {
    let (_folder, path) = folder("pace-round-trip");
    let config = AppConfig {
        pace_days: PaceDays::Weekdays,
        ..AppConfig::default()
    };

    config.save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""pace": "weekdays""#), "{text}");
    assert_eq!(
        AppConfig::load(&path).unwrap().pace_days,
        PaceDays::Weekdays
    );
}

#[test]
fn show_defaults_to_tray_and_pill() {
    let (_folder, path) = folder("show-default");

    assert_eq!(
        AppConfig::load(&path).unwrap().show,
        ShowOption::TrayAndPill
    );
}

#[test]
fn show_loads_and_unknown_values_fall_back() {
    let loaded = load("show-loads", r#"{"show":"TrayOnly"}"#).unwrap();
    assert_eq!(loaded.show, ShowOption::TrayOnly);
    assert_eq!(extra(&loaded, "show"), None);

    let unknown = load("show-unknown", r#"{"show":"everywhere"}"#).unwrap();
    assert_eq!(unknown.show, ShowOption::TrayAndPill);
}

#[test]
fn show_round_trips_and_is_written_lower_case() {
    let (_folder, path) = folder("show-round-trip");
    let config = AppConfig {
        show: ShowOption::TrayOnly,
        ..AppConfig::default()
    };

    config.save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(r#""show": "trayonly""#), "{text}");
    assert_eq!(AppConfig::load(&path).unwrap().show, ShowOption::TrayOnly);
}

#[test]
fn missing_file_gives_default() {
    let (_folder, path) = folder("missing-file");

    assert_default(&AppConfig::load(&path).unwrap());
}

#[test]
fn empty_or_whitespace_gives_default() {
    for (number, text) in ["", "  \n\t "].into_iter().enumerate() {
        assert_default(&load(&format!("blank-{number}"), text).unwrap());
    }
}

#[test]
fn corrupt_or_non_object_throws_instead_of_giving_default() {
    let rows = ["{not json", "[1,2]", r#""text""#, "null"];
    for (number, text) in rows.into_iter().enumerate() {
        let (_folder, path) = write(&format!("corrupt-{number}"), text);

        let error = AppConfig::load(&path).unwrap_err();

        assert!(matches!(error, ConfigError::NotJson), "{text}");
        assert_eq!(
            error.to_string(),
            "config.json isn't valid JSON, so Sobreiro left it alone."
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn a_blank_chained_command_is_no_command() {
    for (number, command) in ["", "   "].into_iter().enumerate() {
        let text = format!(r#"{{"chained_command":"{command}"}}"#);

        let loaded = load(&format!("blank-command-{number}"), &text).unwrap();

        assert_eq!(loaded.chained_command, None, "{command:?}");
    }
}

#[test]
fn round_trips_every_field() {
    const RAW: &str = r#"{"type":"command","command":"x","padding":0}"#;
    let (_folder, path) = folder("every-field");
    let config = AppConfig {
        chained_command: Some("node a.js".to_string()),
        previous: Some(PreviousStatusLine {
            raw_json: Some(RAW.to_string()),
        }),
        style: FlyoutStyle::Thermometer,
        bar_colour: BarColourOption::Accent,
        extra: vec![("future".to_string(), "[1, 2]".to_string())],
        ..AppConfig::default()
    };
    config.save(&path).unwrap();

    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(loaded.chained_command.as_deref(), Some("node a.js"));
    assert_eq!(loaded.previous.unwrap().raw_json.as_deref(), Some(RAW));
    assert_eq!(loaded.style, FlyoutStyle::Thermometer);
    assert_eq!(loaded.bar_colour, BarColourOption::Accent);
    assert_eq!(loaded.extra, [("future".to_string(), "[1, 2]".to_string())]);
}

#[test]
fn an_old_live_usage_key_loads_and_is_kept_untouched_as_an_unknown_key() {
    for old in ["true", "false"] {
        let text = format!(r#"{{"live_usage":{old},"future":[1]}}"#);
        let (_folder, path) = write(&format!("live-usage-{old}"), &text);

        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(extra(&loaded, "live_usage"), Some(old));
        assert_eq!(extra(&loaded, "future"), Some("[1]"));

        loaded.save(&path).unwrap();
        let again = AppConfig::load(&path).unwrap();
        assert_eq!(extra(&again, "live_usage"), Some(old));
        assert_eq!(extra(&again, "future"), Some("[1]"));
    }
}

#[test]
fn previous_status_line_object_text_comes_back_byte_for_byte() {
    const RAW: &str = r#"{"type":"command","command":"x","padding":0}"#;
    let text = format!(r#"{{"previous_status_line":{RAW}}}"#);
    let (_folder, path) = write("previous-bytes", &text);

    let loaded = AppConfig::load(&path).unwrap();
    assert_eq!(
        loaded.previous.as_ref().unwrap().raw_json.as_deref(),
        Some(RAW)
    );

    loaded.save(&path).unwrap();
    let again = AppConfig::load(&path).unwrap();
    assert_eq!(again.previous.unwrap().raw_json.as_deref(), Some(RAW));
}

#[test]
fn previous_null_is_distinct_from_absent() {
    let (_folder, path) = write("previous-null", r#"{"previous_status_line":null}"#);
    let present = AppConfig::load(&path).unwrap();
    assert_eq!(
        present.previous,
        Some(PreviousStatusLine { raw_json: None })
    );

    present.save(&path).unwrap();
    let again = AppConfig::load(&path).unwrap();
    assert_eq!(again.previous, Some(PreviousStatusLine { raw_json: None }));

    std::fs::write(&path, "{}").unwrap();
    assert_eq!(AppConfig::load(&path).unwrap().previous, None);
}

#[test]
fn unknown_key_survives_load_and_save() {
    let (_folder, path) = write("unknown-key", r#"{"future": [1, 2], "style":"battery"}"#);

    AppConfig::load(&path).unwrap().save(&path).unwrap();

    let again = AppConfig::load(&path).unwrap();
    assert_eq!(extra(&again, "future"), Some("[1, 2]"));
    assert_eq!(again.style, FlyoutStyle::Battery);
}

#[test]
fn style_and_colour_load_and_unknown_values_fall_back() {
    let known = load(
        "style-known",
        r#"{"style":"battery","bar_colour":"accent"}"#,
    )
    .unwrap();
    assert_eq!(known.style, FlyoutStyle::Battery);
    assert_eq!(known.bar_colour, BarColourOption::Accent);

    let unknown = load(
        "style-unknown",
        r#"{"style":"hologram","bar_colour":"mauve"}"#,
    )
    .unwrap();
    assert_eq!(unknown.style, FlyoutStyle::Bars);
    assert_eq!(unknown.bar_colour, BarColourOption::Purple);
}

#[test]
fn saved_chained_command_is_read_by_bridge_config() {
    let (_folder, path) = folder("read-by-bridge");
    let config = AppConfig {
        chained_command: Some("echo hi".to_string()),
        ..AppConfig::default()
    };

    config.save(&path).unwrap();

    assert_eq!(
        BridgeConfig::load(&path).chained_command.as_deref(),
        Some("echo hi")
    );
}

#[cfg(windows)]
#[test]
fn a_file_that_cannot_be_read_throws_instead_of_giving_default() {
    let (_folder, path) = write("held", r#"{"chained_command":"x"}"#);
    let _held = crate::hold_alone(&path);

    assert!(matches!(AppConfig::load(&path), Err(ConfigError::Io(_))));
}

/// Not in the C# tests. A program that is writing the file, and lets others read it meanwhile,
/// may be half done: the C# refuses to read then, and so must this, or a half-written file
/// could be taken for an empty one and saved over.
#[cfg(windows)]
#[test]
fn a_file_someone_is_writing_is_not_read() {
    let (_folder, path) = write("being-written", r#"{"pace":"weekdays"}"#);

    {
        let _writer = std::fs::File::options().write(true).open(&path).unwrap();
        assert!(matches!(AppConfig::load(&path), Err(ConfigError::Io(_))));
    }
    {
        let _reader = std::fs::File::open(&path).unwrap();
        assert_eq!(
            AppConfig::load(&path).unwrap().pace_days,
            PaceDays::Weekdays
        );
    }
}

/// Not in the C# tests. Every choice has the name 1.x wrote: the C# name in small letters.
#[test]
fn every_choice_is_written_by_its_name_and_read_back() {
    let styles = [
        (FlyoutStyle::Bars, "bars"),
        (FlyoutStyle::Battery, "battery"),
        (FlyoutStyle::Thermometer, "thermometer"),
    ];
    for (style, name) in styles {
        let config = AppConfig {
            style,
            ..AppConfig::default()
        };
        let json = config.to_json().unwrap();
        assert!(json.contains(&format!(r#""style": "{name}""#)), "{json}");
        assert_eq!(load(&format!("choice-{name}"), &json).unwrap(), config);
    }

    let colours = [
        (BarColourOption::Purple, "purple"),
        (BarColourOption::Accent, "accent"),
    ];
    for (bar_colour, name) in colours {
        let config = AppConfig {
            bar_colour,
            ..AppConfig::default()
        };
        let json = config.to_json().unwrap();
        assert!(
            json.contains(&format!(r#""bar_colour": "{name}""#)),
            "{json}"
        );
        assert_eq!(load(&format!("choice-{name}"), &json).unwrap(), config);
    }

    let shows = [
        (ShowOption::TrayAndPill, "trayandpill"),
        (ShowOption::TrayOnly, "trayonly"),
    ];
    for (show, name) in shows {
        let config = AppConfig {
            show,
            ..AppConfig::default()
        };
        let json = config.to_json().unwrap();
        assert!(json.contains(&format!(r#""show": "{name}""#)), "{json}");
        assert_eq!(load(&format!("choice-{name}"), &json).unwrap(), config);
    }

    let paces = [
        (PaceDays::EveryDay, "everyday"),
        (PaceDays::Weekdays, "weekdays"),
    ];
    for (pace_days, name) in paces {
        let config = AppConfig {
            pace_days,
            ..AppConfig::default()
        };
        let json = config.to_json().unwrap();
        assert!(json.contains(&format!(r#""pace": "{name}""#)), "{json}");
        assert_eq!(load(&format!("choice-{name}"), &json).unwrap(), config);
    }
}

#[test]
fn a_missing_folder_gives_default() {
    let (folder, _path) = folder("missing-folder");

    let loaded = AppConfig::load(&folder.path().join("nope").join("config.json"));

    assert_default(&loaded.unwrap());
}

#[test]
fn numeric_strings_are_not_enum_values() {
    for number in ["1", "0"] {
        let text = format!(r#"{{"style":"{number}","bar_colour":"{number}"}}"#);

        let loaded = load(&format!("numeric-{number}"), &text).unwrap();

        assert_eq!(loaded.style, FlyoutStyle::Bars, "{number}");
        assert_eq!(loaded.bar_colour, BarColourOption::Purple, "{number}");
    }
}

#[test]
fn commands_are_written_readably() {
    const COMMAND: &str = r#"a && b "c""#;
    let (_folder, path) = folder("readable");
    let config = AppConfig {
        chained_command: Some(COMMAND.to_string()),
        ..AppConfig::default()
    };

    config.save(&path).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(r#""chained_command": "a && b \"c\"""#),
        "{text}"
    );
    assert_eq!(
        BridgeConfig::load(&path).chained_command.as_deref(),
        Some(COMMAND)
    );
    assert_eq!(
        AppConfig::load(&path).unwrap().chained_command.as_deref(),
        Some(COMMAND)
    );
}

// Not in the C# tests. What these hold was read off the C# of 1.1.0, run on 2026-11-14 in a
// data folder of its own.

/// An installed user's `config.json` must not change with the language. These are files the C#
/// wrote: each line ends in a carriage return and a line feed, the values are set in by two
/// spaces, and nothing follows the last brace.
#[test]
fn save_gives_the_bytes_the_csharp_writes() {
    const PREVIOUS: &str = r#"{"type":"command","command":"x","padding":0}"#;
    let rows = [
        (
            AppConfig::default(),
            "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        ),
        (
            AppConfig {
                chained_command: Some("node a.js".to_string()),
                previous: Some(PreviousStatusLine {
                    raw_json: Some(PREVIOUS.to_string()),
                }),
                style: FlyoutStyle::Thermometer,
                bar_colour: BarColourOption::Accent,
                show: ShowOption::TrayOnly,
                pace_days: PaceDays::Weekdays,
                extra: vec![
                    ("future".to_string(), "[1, 2]".to_string()),
                    ("live_usage".to_string(), "true".to_string()),
                ],
            },
            "{\r\n  \"chained_command\": \"node a.js\",\r\n  \"previous_status_line\": {\"type\":\"command\",\"command\":\"x\",\"padding\":0},\r\n  \"style\": \"thermometer\",\r\n  \"bar_colour\": \"accent\",\r\n  \"show\": \"trayonly\",\r\n  \"pace\": \"weekdays\",\r\n  \"future\": [1, 2],\r\n  \"live_usage\": true\r\n}",
        ),
        (
            AppConfig {
                previous: Some(PreviousStatusLine { raw_json: None }),
                ..AppConfig::default()
            },
            "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        ),
    ];
    for (number, (config, written_by_csharp)) in rows.into_iter().enumerate() {
        let (_folder, path) = folder(&format!("csharp-bytes-{number}"));

        config.save(&path).unwrap();

        // The bytes on disk, with nothing before them.
        assert_eq!(
            std::fs::read(&path).unwrap(),
            written_by_csharp.as_bytes(),
            "row {number}"
        );
        // And the file the C# wrote, read and saved again, stays as it was.
        let loaded = AppConfig::load(&path).unwrap();
        assert_eq!(loaded, config, "row {number}");
        assert_eq!(loaded.to_json().unwrap(), written_by_csharp, "row {number}");
    }
}

/// A command and a key are written with the same characters spelled out as codes as the C#
/// spells out, and every other character as itself.
#[test]
fn a_command_is_escaped_as_the_csharp_escapes_it() {
    let rows = [
        // Written as themselves, also where a stricter writer would use a code.
        ("a && b 'c' <d> + `e` /f", "a && b 'c' <d> + `e` /f"),
        ("é ß 日本", "é ß 日本"),
        ("\u{ad}\u{200b}", "\u{ad}\u{200b}"),
        // The two that JSON itself needs marked.
        (r#"""#, r#"\""#),
        (r"C:\x", r"C:\\x"),
        // The characters that give orders to a screen: five have a short spelling.
        ("\t\n\r\u{8}\u{c}", r"\t\n\r\b\f"),
        (
            "\u{1}\u{1f}\u{7f}\u{80}\u{9f}",
            r"\u0001\u001F\u007F\u0080\u009F",
        ),
        // Every space but the plain one, and the two that break a line.
        ("\u{a0}\u{3000}", r"\u00A0\u3000"),
        ("\u{2028}\u{2029}", r"\u2028\u2029"),
        // The mark some programs start a file with, and codes no letter will ever have.
        ("\u{feff}\u{fdd0}\u{ffff}", r"\uFEFF\uFDD0\uFFFF"),
        // Codes set aside for private use.
        ("\u{e000}\u{f8ff}", r"\uE000\uF8FF"),
        // Characters past the first 65,536, as the pair of codes that makes them up.
        ("😀", r"\uD83D\uDE00"),
    ];
    for (command, written_by_csharp) in rows {
        let as_command = AppConfig {
            chained_command: Some(command.to_string()),
            ..AppConfig::default()
        };
        let json = as_command.to_json().unwrap();
        let line = format!("  \"chained_command\": \"{written_by_csharp}\",\r\n");
        assert!(json.contains(&line), "{command:?} gave {json}");

        let as_key = AppConfig {
            extra: vec![(command.to_string(), "1".to_string())],
            ..AppConfig::default()
        };
        let json = as_key.to_json().unwrap();
        let line = format!(",\r\n  \"{written_by_csharp}\": 1\r\n}}");
        assert!(json.ends_with(&line), "{command:?} gave {json}");
    }
}

/// The C# throws on a kept value that is not JSON, before anything touches the disk.
#[test]
fn save_refuses_a_kept_value_that_is_not_json() {
    let rows = ["[1, 2", "1 2", "", "1 /* x */", "{oops"];
    for (number, raw) in rows.into_iter().enumerate() {
        let (folder, path) = folder(&format!("not-json-{number}"));

        let unknown = AppConfig {
            extra: vec![("future".to_string(), raw.to_string())],
            ..AppConfig::default()
        };
        let error = unknown.save(&path).unwrap_err();
        assert!(
            matches!(&error, ConfigError::ValueNotJson { key } if key == "future"),
            "{raw}"
        );

        let previous = AppConfig {
            previous: Some(PreviousStatusLine {
                raw_json: Some(raw.to_string()),
            }),
            ..AppConfig::default()
        };
        let error = previous.save(&path).unwrap_err();
        assert!(
            matches!(&error, ConfigError::ValueNotJson { key } if key == "previous_status_line"),
            "{raw}"
        );

        assert!(folder.files().is_empty(), "{raw}");
    }
}

/// A value Sobreiro does not know is never read, only carried: spaces, line ends and spellings
/// inside it come back as they were, as do a number no program could hold and half a character.
#[test]
fn a_kept_value_is_written_as_it_was_read() {
    let text = "{ \"future\" :  { \"a\" : [ 1 ,\r\n 2 ] }  , \"x\":\"\\u00e9\\/\", \"big\":1e999, \"half\":\"\\uD800\" }";
    let (_folder, path) = write("kept-as-read", text);

    let loaded = AppConfig::load(&path).unwrap();

    assert_eq!(extra(&loaded, "future"), Some("{ \"a\" : [ 1 ,\r\n 2 ] }"));
    assert_eq!(extra(&loaded, "x"), Some(r#""\u00e9\/""#));
    assert_eq!(extra(&loaded, "big"), Some("1e999"));
    assert_eq!(extra(&loaded, "half"), Some(r#""\uD800""#));
    assert_eq!(
        loaded.to_json().unwrap(),
        "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\",\r\n  \"future\": { \"a\" : [ 1 ,\r\n 2 ] },\r\n  \"x\": \"\\u00e9\\/\",\r\n  \"big\": 1e999,\r\n  \"half\": \"\\uD800\"\r\n}"
    );
}

/// A file may hold a key twice. For most keys the last one counts; a command is only replaced
/// by another command; a key Sobreiro does not know keeps its first place and its last value.
#[test]
fn a_key_written_twice() {
    let loaded = load(
        "twice-command",
        r#"{"chained_command":"a","chained_command":5,"chained_command":"  "}"#,
    );
    assert_eq!(loaded.unwrap().chained_command.as_deref(), Some("a"));

    let loaded = load("twice-style", r#"{"style":"battery","style":"x"}"#);
    assert_eq!(loaded.unwrap().style, FlyoutStyle::Bars);

    let loaded = load(
        "twice-previous",
        r#"{"previous_status_line":{"a":1},"previous_status_line":null}"#,
    );
    assert_eq!(
        loaded.unwrap().previous,
        Some(PreviousStatusLine { raw_json: None })
    );

    let loaded = load("twice-unknown", r#"{"a":1,"b":2,"a":3}"#);
    assert_eq!(
        loaded.unwrap().extra,
        [
            ("a".to_string(), "3".to_string()),
            ("b".to_string(), "2".to_string())
        ]
    );
}

/// A key is known by its exact letters, however they are spelled in the file; a value names a
/// choice whole, with capitals or without.
#[test]
fn keys_are_exact_and_names_ignore_only_capitals() {
    let loaded = load("names-key-case", r#"{"Style":"battery"}"#).unwrap();
    assert_eq!(loaded.style, FlyoutStyle::Bars);
    assert_eq!(extra(&loaded, "Style"), Some(r#""battery""#));

    let loaded = load("names-key-spelled", r#"{"st\u0079le":"b\u0061ttery"}"#).unwrap();
    assert_eq!(loaded.style, FlyoutStyle::Battery);
    assert!(loaded.extra.is_empty());

    let loaded = load(
        "names-capitals",
        r#"{"style":"BATTERY","bar_colour":"ACCENT","show":"TRAYONLY","pace":"WEEKDAYS"}"#,
    )
    .unwrap();
    assert_eq!(loaded.style, FlyoutStyle::Battery);
    assert_eq!(loaded.bar_colour, BarColourOption::Accent);
    assert_eq!(loaded.show, ShowOption::TrayOnly);
    assert_eq!(loaded.pace_days, PaceDays::Weekdays);

    // Spaces around a name, two names, and two signs that only look like a K and an s (the
    // kelvin sign and the long s of old print).
    let rows = [
        r#"{"style":" battery "}"#,
        r#"{"style":"Bars,Battery"}"#,
        "{\"pace\":\"wee\u{212a}days\"}",
        "{\"pace\":\"weekday\u{17f}\"}",
    ];
    for (number, text) in rows.into_iter().enumerate() {
        let loaded = load(&format!("names-not-{number}"), text).unwrap();
        assert_eq!(loaded.style, FlyoutStyle::Bars, "{text}");
        assert_eq!(loaded.pace_days, PaceDays::EveryDay, "{text}");
    }
}

/// What the file's bytes are read as, before any JSON: the mark some programs start a file
/// with is skipped, bytes that are no text become the "unknown character" sign, and "blank"
/// takes in every kind of space.
#[test]
fn load_reads_the_file_as_the_csharp_reads_it() {
    let loaded = load("read-mark", "\u{feff}{\"style\":\"battery\"}").unwrap();
    assert_eq!(loaded.style, FlyoutStyle::Battery);

    let blank = ["\u{feff}", "\u{feff}  \n", "\u{a0}\u{2003}\u{3000}\u{85}"];
    for (number, text) in blank.into_iter().enumerate() {
        assert_default(&load(&format!("read-blank-{number}"), text).unwrap());
    }

    let mut bytes = br#"{"chained_command":"a"#.to_vec();
    bytes.extend([0xFF, 0xC3]);
    bytes.extend(br#"b"}"#);
    let (_folder, path) = write_bytes("read-not-text", &bytes);
    assert_eq!(
        AppConfig::load(&path).unwrap().chained_command.as_deref(),
        Some("a\u{fffd}\u{fffd}b")
    );

    let not_json = [
        // Not spaces, though they show as nothing.
        "\u{1c}\u{1d}\u{1e}\u{1f}",
        "\u{200b}",
        "\0",
        // What looser readers of JSON let through.
        r#"{/* x */"style":"battery"}"#,
        r#"{"style":"battery",}"#,
        "{} {}",
        r#"{"future":01}"#,
    ];
    for (number, text) in not_json.into_iter().enumerate() {
        assert_not_json(&load(&format!("read-not-json-{number}"), text), text);
    }
}

/// `\uD800` is half of a character. In a text Sobreiro has to read, or in a key, the C# stops
/// with an error of another kind; here the file is "not JSON". Either way nothing is saved
/// over it.
#[test]
fn half_a_character_where_sobreiro_reads_is_not_json() {
    let rows = [
        r#"{"chained_command":"\uD800"}"#,
        r#"{"style":"\uD800"}"#,
        r#"{"\uD800":1}"#,
    ];
    for (number, text) in rows.into_iter().enumerate() {
        assert_not_json(&load(&format!("half-{number}"), text), text);
    }
}
