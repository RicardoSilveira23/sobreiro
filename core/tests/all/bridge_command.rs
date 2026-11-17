//! Ported from `tests/Sobreiro.Tests/BridgeCommandTests.cs`.
use std::path::Path;

use sobreiro_core::connect::bridge_command::{
    UnsafeBridgePath, command_for, is_bridge, runs, status_line_json,
};

#[test]
fn for_uses_forward_slashes_and_quotes() {
    assert_eq!(
        command_for(Path::new(r"C:\Dev\my sobreiro\sobreiro-bridge.exe")).unwrap(),
        "\"C:/Dev/my sobreiro/sobreiro-bridge.exe\""
    );
}

#[test]
fn for_keeps_a_single_quote_and_a_unicode_letter() {
    assert_eq!(
        command_for(Path::new(
            "D:\\Apps\\O'Brien\\Ren\u{e9}\\sobreiro-bridge.exe"
        ))
        .unwrap(),
        "\"D:/Apps/O'Brien/Ren\u{e9}/sobreiro-bridge.exe\""
    );
}

#[test]
fn for_keeps_an_exclamation_mark_and_a_percent_sign() {
    assert_eq!(
        command_for(Path::new("D:\\Apps\\Rick!\\100%\\sobreiro-bridge.exe")).unwrap(),
        "\"D:/Apps/Rick!/100%/sobreiro-bridge.exe\""
    );
}

#[test]
fn for_refuses_a_path_that_would_not_be_taken_literally() {
    let refused = [
        '$', '`', '"', '\n', '\r', '\t', '\0', '\u{1b}', '\u{201C}', '\u{201D}', '\u{201E}',
    ];
    for character in refused {
        let path = format!("D:\\Apps\\a{character}b\\sobreiro-bridge.exe");
        assert_eq!(
            command_for(Path::new(&path)),
            Err(UnsafeBridgePath),
            "{character:?}"
        );
    }
}

#[test]
fn is_bridge_recognises_the_bridge() {
    let rows = [
        (Some("\"C:/x/Sobreiro-Bridge.exe\""), true),
        (Some("~/bin/sobreiro-bridge"), true),
        (Some("~/.claude/statusline.sh"), false),
        (None, false),
    ];
    for (command, expected) in rows {
        assert_eq!(is_bridge(command), expected, "{command:?}");
    }
}

#[test]
fn status_line_json_builds_an_indented_object_for_a_top_level_property() {
    let json = status_line_json("\"C:/x/sobreiro-bridge.exe\"", None).unwrap();
    assert_eq!(
        json,
        "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }"
    );
}

#[test]
fn status_line_json_keeps_other_properties_after_command() {
    let json = status_line_json(
        "\"C:/x/sobreiro-bridge.exe\"",
        Some("{\"type\":\"command\",\"command\":\"old\",\"padding\":2}"),
    )
    .unwrap();
    assert_eq!(
        json,
        "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"padding\": 2\n  }"
    );
}

#[cfg(windows)]
#[test]
fn runs_matches_the_path_however_it_is_written() {
    let rows = [
        (
            Some("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\""),
            true,
        ),
        (
            Some("C:\\Users\\a\\AppData\\Local\\Sobreiro\\sobreiro-bridge.exe"),
            true,
        ),
        (
            Some("  \"c:/users/A/appdata/local/sobreiro/SOBREIRO-BRIDGE.EXE\" "),
            true,
        ),
        (
            Some("\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\""),
            false,
        ),
        (
            Some("\"C:/Users/a/AppData/Local/Sobreiro/sobreiro-bridge.exe\" | tee x"),
            false,
        ),
        (None, false),
    ];
    for (command, expected) in rows {
        assert_eq!(
            runs(
                command,
                Path::new(r"C:\Users\a\AppData\Local\Sobreiro\sobreiro-bridge.exe")
            ),
            expected,
            "{command:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn runs_normalises_both_sides_the_same_way() {
    let rows = [
        (
            "C:/data/sobreiro-bridge.exe",
            "\"C:/data/sobreiro-bridge.exe\"",
            true,
        ),
        (
            "C:/data\\sobreiro-bridge.exe",
            "\"C:/data/sobreiro-bridge.exe\"",
            true,
        ),
        (
            "C:/data/x/../sobreiro-bridge.exe",
            "\"C:/data/sobreiro-bridge.exe\"",
            true,
        ),
        (
            r"C:\data\sobreiro-bridge.exe",
            "\"C:/data/y/../sobreiro-bridge.exe\"",
            true,
        ),
        (
            "C:/data/sobreiro-bridge.exe",
            "\"C:/other/sobreiro-bridge.exe\"",
            false,
        ),
    ];
    for (bridge_exe_path, command, expected) in rows {
        assert_eq!(
            runs(Some(command), Path::new(bridge_exe_path)),
            expected,
            "{bridge_exe_path} {command}"
        );
    }
}

// What follows has no C# test. Each row was put to the C# of 1.1.0 on 2026-11-14, and holds
// the answer it gave.

#[test]
fn status_line_json_answers_as_the_csharp_did() {
    // The command, the status line that was there, and the object that comes back. `None`
    // stands for an error.
    let rows: &[(&str, Option<&str>, Option<&str>)] = &[
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some(
                "{\"type\":\"command\",\"command\":\"old\",\"a\":{\"b\":[1,2,{\"c\":null}],\"d\":{},\"e\":[]},\"n\":[1.0,1e5,-0,1E+2,0.10,12345678901234567890123]}",
            ),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"a\": {\n        \"b\": [\n            1,\n            2,\n            {\n                \"c\": null\n            }\n        ],\n        \"d\": {},\n        \"e\": []\n    },\n    \"n\": [\n        1.0,\n        1e5,\n        -0,\n        1E+2,\n        0.10,\n        12345678901234567890123\n    ]\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some(
                "{\"s\":\"\\u00e9 é < > & ' + \\/ \\ud83d\\ude00 😀 \\u2028 \u{2028} \\u00a0 \\t \\u007f \\\\ \\\" \\b \\f \\n \\r \u{feff} \u{e000}\"}",
            ),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"s\": \"é é < > & ' + / \\uD83D\\uDE00 \\uD83D\\uDE00 \\u2028 \\u2028 \\u00A0 \\t \\u007F \\\\ \\\" \\b \\f \\n \\r \\uFEFF \\uE000\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"type\":1,\"x\":1,\"type\":2,\"x\":2,\"command\":3,\"x\":3}"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"x\": 1,\n    \"x\": 2,\n    \"x\": 3\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"k\\u00e9y é \\\"q\\\"\":true,\"f\":false}"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"kéy é \\\"q\\\"\": true,\n    \"f\": false\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{ // c\n \"padding\": 1, /* c */ \"x\" : [ 1 , 2 , ], }"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1,\n    \"x\": [\n        1,\n        2\n    ]\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"Type\":1,\"COMMAND\":2}"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"Type\": 1,\n    \"COMMAND\": 2\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"t\\u0079pe\":1,\"comm\\u0061nd\":2,\"padding\":3}"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"padding\": 3\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{  }"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("[1,2]"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("\"x\""),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{oops"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some(""),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\"\n  }",
            ),
        ),
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"s\":\"\\ud800\"}"),
            None,
        ), // the C# stops with an error of another kind
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"\\ud800\":1}"),
            None,
        ), // the C# stops with an error of another kind
        (
            "\"C:/x/sobreiro-bridge.exe\"",
            Some("{\"command\":\"\\ud800\",\"type\":\"\\ud800\",\"padding\":1}"),
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/x/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }",
            ),
        ),
        (
            "aé😀\u{2028}<&'+\t\u{7f}\u{a0}\\\"/",
            None,
            Some(
                "{\n    \"type\": \"command\",\n    \"command\": \"aé\\uD83D\\uDE00\\u2028<&'+\\t\\u007F\\u00A0\\\\\\\"/\"\n  }",
            ),
        ),
        (
            "",
            None,
            Some("{\n    \"type\": \"command\",\n    \"command\": \"\"\n  }"),
        ),
    ];
    for (command, existing, expected) in rows {
        assert_eq!(
            status_line_json(command, *existing).ok().as_deref(),
            *expected,
            "{command:?} {existing:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn runs_answers_as_the_csharp_did() {
    // The command, the bridge's path, and whether the command runs that bridge.
    let rows: &[(Option<&str>, &str, bool)] = &[
        (
            Some("C:\\Data\\sobreiro-bridge.exe "),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:\\Data\\sobreiro-bridge.exe."),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("\"C:\\Data\\sobreiro-bridge.exe . \""),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:\\Data\\sobreiro-bridge.exe\\"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("\"\"C:/Data/sobreiro-bridge.exe\"\""),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("\"C:/Data/sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (Some("\""), "C:\\Data\\sobreiro-bridge.exe", false),
        (Some("\"\""), "C:\\Data\\sobreiro-bridge.exe", false),
        (Some(""), "C:\\Data\\sobreiro-bridge.exe", false),
        (Some("  "), "C:\\Data\\sobreiro-bridge.exe", false),
        (
            Some("C:/Data//sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:/Data/./sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:/Data/x/y/../../sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:/../../Data/sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("c:/DATA/Sobreiro-Bridge.EXE"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("D:/Data/sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("\\\\?\\C:\\Data\\sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("\\\\.\\C:\\Data\\sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("\\\\?\\C:\\Data\\sobreiro-bridge.exe"),
            "\\\\?\\C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("\\\\?\\C:\\Data\\SOBREIRO-bridge.exe"),
            "\\\\?\\C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("\\\\?\\C:\\Data\\x\\..\\sobreiro-bridge.exe"),
            "\\\\?\\C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("//server/share/Sobreiro/sobreiro-bridge.exe"),
            "\\\\SERVER\\share\\Sobreiro\\sobreiro-bridge.exe",
            true,
        ),
        (
            Some("C:/Data/sobreiro-bridge.exe\u{0}"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/sobreiro-bridge.exe|"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/sobreiro-bridge.exe:"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/sobreiro-bridge.exe arg"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("bash C:/Data/sobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/ſobreiro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/sobreıro-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (
            Some("C:/Data/SOBREİRO-bridge.exe"),
            "C:\\Data\\sobreiro-bridge.exe",
            false,
        ),
        (Some("C:/Datá/x.exe"), "C:/DATÁ/x.exe", true),
        (Some("C:/ς/x.exe"), "C:/Σ/x.exe", true),
        (Some("C:/σ/x.exe"), "C:/ς/x.exe", true),
        (Some("C:/ß/x.exe"), "C:/SS/x.exe", false),
        (Some("C:/ẞ/x.exe"), "C:/ß/x.exe", false),
        (Some("C:/ÿ/x.exe"), "C:/Ÿ/x.exe", true),
        (Some("C:/µ/x.exe"), "C:/Μ/x.exe", true),
        (Some("C:/ǆ/x.exe"), "C:/ǅ/x.exe", true),
        (Some("C:/𐐨/x.exe"), "C:/𐐀/x.exe", true),
        (
            Some("\u{a0}C:/Data/sobreiro-bridge.exe\u{2003}"),
            "C:\\Data\\sobreiro-bridge.exe",
            true,
        ),
        (Some("\"C:/Data/sobreiro-bridge.exe\""), "", false),
        (None, "C:\\Data\\sobreiro-bridge.exe", false),
    ];
    for (command, bridge_exe_path, expected) in rows {
        assert_eq!(
            runs(*command, Path::new(bridge_exe_path)),
            *expected,
            "{command:?} {bridge_exe_path:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn a_command_that_is_not_a_full_path_is_read_from_the_folder_the_program_runs_in() {
    let here = std::env::current_dir().unwrap();
    assert!(runs(
        Some("sobreiro-bridge.exe"),
        &here.join("sobreiro-bridge.exe")
    ));
    assert!(runs(
        Some("\"x/../sobreiro-bridge.exe\""),
        &here.join("sobreiro-bridge.exe")
    ));
    assert!(!runs(
        Some("sobreiro-bridge.exe"),
        &here.join("elsewhere").join("sobreiro-bridge.exe")
    ));
}

#[test]
fn is_bridge_answers_as_the_csharp_did() {
    // The command, and whether it runs a bridge of ours.
    let rows: &[(Option<&str>, bool)] = &[
        (Some("SOBREIRO-BRIDGE"), true),
        (Some("sobreiro_bridge"), false),
        (Some("sobreiro bridge"), false),
        (Some("ſobreiro-bridge"), false),
        (Some("sobreıro-bridge"), false),
        (Some("SOBREİRO-BRIDGE"), false),
        (Some(""), false),
        (None, false),
    ];
    for (command, ours) in rows {
        assert_eq!(is_bridge(*command), *ours, "{command:?}");
    }
}
