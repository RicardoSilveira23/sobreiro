//! Not ported from a C# test: what Connect, Disconnect and Reconnect of 1.1.0 left on the disk.
//!
//! The C# was run on 2026-11-14 on each `settings.json` below, in a folder of its own: first
//! Connect (or Reconnect, from a config that already holds the user's status line), then
//! Disconnect. Every byte it wrote to `settings.json` and to `config.json` is held here, and
//! the Rust must write the same. The samples are the shapes a real file takes: no status line,
//! another tool's, Sobreiro's own, the old `openheadroom-bridge`, odd spacing, comments, a
//! byte order mark, both kinds of line end.
#![cfg(windows)]
use std::path::Path;

use chrono::NaiveDate;
use sobreiro_core::config::{AppConfig, PreviousStatusLine};
use sobreiro_core::connect::connection::{apply, plan_connect, plan_disconnect, read_settings};
use sobreiro_core::connect::settings_json::read_status_line;

use crate::TestFolder;

const BRIDGE: &str = r"C:\Apps\Sobreiro\sobreiro-bridge.exe";

/// One `settings.json` and what the C# made of it.
struct Case {
    name: &'static str,
    /// True when the config already held the user's own status line, as after the first-run
    /// migration: the C# was then asked to connect over a bridge that was there already.
    reconnect: bool,
    /// True when the file starts with a byte order mark. Every text below is without it.
    bom: bool,
    /// The file before, or `None` when there was no file.
    before: Option<&'static str>,
    /// `settings.json` and `config.json` after Connect.
    connected: &'static str,
    config_when_connected: &'static str,
    /// `settings.json` and `config.json` after the Disconnect that followed.
    disconnected: &'static str,
    config_when_disconnected: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "no-file",
        reconnect: false,
        bom: false,
        before: None,
        connected: "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "empty-object",
        reconnect: false,
        bom: false,
        before: Some("{}"),
        connected: "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{}",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "no-status-line",
        reconnect: false,
        bom: false,
        before: Some("{\n  \"model\": \"x\"\n}\n"),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "no-status-line-tabs-crlf",
        reconnect: false,
        bom: false,
        before: Some(
            "{\r\n\t\"model\": \"x\",\r\n\t\"env\": {\r\n\t\t\"A\": \"1\"\r\n\t}\r\n}\r\n",
        ),
        connected: "{\r\n\t\"model\": \"x\",\r\n\t\"env\": {\r\n\t\t\"A\": \"1\"\r\n\t},\r\n\t\"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\r\n}\r\n",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\r\n\t\"model\": \"x\",\r\n\t\"env\": {\r\n\t\t\"A\": \"1\"\r\n\t}\r\n}\r\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "no-status-line-one-line",
        reconnect: false,
        bom: false,
        before: Some("{\"model\":\"x\",\"n\":[1,2,3]}"),
        connected: "{\"model\":\"x\",\"n\":[1,2,3],\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }}",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\"model\":\"x\",\"n\":[1,2,3]}",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "another-tool",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\n  \"theme\": \"dark\"\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\n  \"theme\": \"dark\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "another-tool-many-lines",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"npx -y some-status@latest --flag \\\"a b\\\"\",\n    \"padding\": 0,\n    \"extra\": { \"a\": [1, 2.50, 1e3], \"b\": null, \"c\": {}, \"d\": [] }\n  },\n  \"model\": \"x\"\n}\n",
        ),
        connected: "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 0,\n    \"extra\": {\n        \"a\": [\n            1,\n            2.50,\n            1e3\n        ],\n        \"b\": null,\n        \"c\": {},\n        \"d\": []\n    }\n  },\n  \"model\": \"x\"\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"npx -y some-status@latest --flag \\\"a b\\\"\",\r\n  \"previous_status_line\": {\n    \"type\": \"command\",\n    \"command\": \"npx -y some-status@latest --flag \\\"a b\\\"\",\n    \"padding\": 0,\n    \"extra\": { \"a\": [1, 2.50, 1e3], \"b\": null, \"c\": {}, \"d\": [] }\n  },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"npx -y some-status@latest --flag \\\"a b\\\"\",\n    \"padding\": 0,\n    \"extra\": { \"a\": [1, 2.50, 1e3], \"b\": null, \"c\": {}, \"d\": [] }\n  },\n  \"model\": \"x\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "another-tool-odd-letters",
        reconnect: false,
        bom: false,
        before: Some(
            "{\"statusLine\": {\"type\":\"command\",\"command\":\"echo café \\u00e9 😀 \\ud83d\\ude00 <&'+> \\/ \\t\",\"n\\u00e3o\":\"\u{a0}\u{2028}\\u007f\"}, \"x\": \"café \\u00e9\"}\n",
        ),
        connected: "{\"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"não\": \"\\u00A0\\u2028\\u007F\"\n  }, \"x\": \"café \\u00e9\"}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"echo café é \\uD83D\\uDE00 \\uD83D\\uDE00 <&'+> / \\t\",\r\n  \"previous_status_line\": {\"type\":\"command\",\"command\":\"echo café \\u00e9 😀 \\ud83d\\ude00 <&'+> \\/ \\t\",\"n\\u00e3o\":\"\u{a0}\u{2028}\\u007f\"},\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\"statusLine\": {\"type\":\"command\",\"command\":\"echo café \\u00e9 😀 \\ud83d\\ude00 <&'+> \\/ \\t\",\"n\\u00e3o\":\"\u{a0}\u{2028}\\u007f\"}, \"x\": \"café \\u00e9\"}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "ours",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  },\n  \"theme\": \"dark\"\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  },\n  \"theme\": \"dark\"\n}\n",
        config_when_connected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"theme\": \"dark\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "ours-elsewhere",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        config_when_connected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "ours-elsewhere-reconnect",
        reconnect: true,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Users/a/Downloads/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "legacy",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n",
        config_when_connected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"theme\": \"dark\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "legacy-reconnect",
        reconnect: true,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/OpenHeadroom/openheadroom-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"theme\": \"dark\"\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\n  \"theme\": \"dark\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "odd-spacing",
        reconnect: false,
        bom: false,
        before: Some(
            "{ \"a\" :1 ,\"statusLine\"   :   {\"type\":\"command\",\"command\":\"x\"}   ,  \"b\":[1,2 , 3]\n\n\n   }   \n\n",
        ),
        connected: "{ \"a\" :1 ,\"statusLine\"   :   {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }   ,  \"b\":[1,2 , 3]\n\n\n   }   \n\n",
        config_when_connected: "{\r\n  \"chained_command\": \"x\",\r\n  \"previous_status_line\": {\"type\":\"command\",\"command\":\"x\"},\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{ \"a\" :1 ,\"statusLine\"   :   {\"type\":\"command\",\"command\":\"x\"}   ,  \"b\":[1,2 , 3]\n\n\n   }   \n\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "comments-and-trailing-comma",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  // the model\n  \"model\": \"x\", /* inline */\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }, // mine\n  \"theme\": \"dark\",\n}\n",
        ),
        connected: "{\n  // the model\n  \"model\": \"x\", /* inline */\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }, // mine\n  \"theme\": \"dark\",\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  // the model\n  \"model\": \"x\", /* inline */\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }, // mine\n  \"theme\": \"dark\",\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "bom-crlf",
        reconnect: false,
        bom: true,
        before: Some(
            "{\r\n  \"model\": \"café\",\r\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\r\n}\r\n",
        ),
        connected: "{\r\n  \"model\": \"café\",\r\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }\r\n}\r\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\r\n  \"model\": \"café\",\r\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\r\n}\r\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "bom-no-status-line",
        reconnect: false,
        bom: true,
        before: Some("{\n  \"model\": \"café\"\n}\n"),
        connected: "{\n  \"model\": \"café\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\"\n  }\n}\n",
        config_when_connected: "{\r\n  \"previous_status_line\": null,\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"café\"\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "mixed-line-ends",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"a\": 1,\r\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\n  \"b\": 2\r\n}",
        ),
        connected: "{\n  \"a\": 1,\r\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  },\n  \"b\": 2\r\n}",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"a\": 1,\r\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\n  \"b\": 2\r\n}",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "status-line-twice",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"statusLine\": \"first\",\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}\n",
        ),
        connected: "{\n  \"statusLine\": \"first\",\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"statusLine\": \"first\",\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "no-final-line-end",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}",
        ),
        connected: "{\n  \"model\": \"x\",\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }\n}",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"model\": \"x\",\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
    Case {
        name: "unicode-elsewhere",
        reconnect: false,
        bom: false,
        before: Some(
            "{\n  \"greeting\": \"olá \\u00e9 😀\",\n  \"n\": 1.0e5,\n  \"big\": 12345678901234567890123,\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}\n",
        ),
        connected: "{\n  \"greeting\": \"olá \\u00e9 😀\",\n  \"n\": 1.0e5,\n  \"big\": 12345678901234567890123,\n  \"statusLine\": {\n    \"type\": \"command\",\n    \"command\": \"\\\"C:/Apps/Sobreiro/sobreiro-bridge.exe\\\"\",\n    \"padding\": 1\n  }\n}\n",
        config_when_connected: "{\r\n  \"chained_command\": \"~/.claude/statusline.sh\",\r\n  \"previous_status_line\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 },\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
        disconnected: "{\n  \"greeting\": \"olá \\u00e9 😀\",\n  \"n\": 1.0e5,\n  \"big\": 12345678901234567890123,\n  \"statusLine\": { \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }\n}\n",
        config_when_disconnected: "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\"\r\n}",
    },
];

/// The bytes of a file holding `text`, after a byte order mark when `bom` says so.
fn file_bytes(bom: bool, text: &str) -> Vec<u8> {
    let mut bytes = if bom {
        vec![0xEF, 0xBB, 0xBF]
    } else {
        Vec::new()
    };
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

#[test]
fn connect_and_disconnect_leave_the_bytes_the_csharp_left() {
    let now = NaiveDate::from_ymd_opt(2026, 11, 9)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();
    for case in CASES {
        let name = case.name;
        let folder = TestFolder::new(&format!("csharp-files-{name}"));
        let settings_path = folder.path().join("claude").join("settings.json");
        let config_path = folder.path().join("data").join("config.json");
        std::fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        let before = case.before.map(|text| file_bytes(case.bom, text));
        if let Some(bytes) = &before {
            std::fs::write(&settings_path, bytes).unwrap();
        }
        let config = if case.reconnect {
            AppConfig {
                chained_command: Some("~/.claude/statusline.sh".to_string()),
                previous: Some(PreviousStatusLine {
                    raw_json: Some(
                        "{ \"type\": \"command\", \"command\": \"~/.claude/statusline.sh\", \"padding\": 1 }"
                            .to_string(),
                    ),
                }),
                ..AppConfig::default()
            }
        } else {
            AppConfig::default()
        };

        let settings = read_settings(&settings_path).unwrap();
        let plan = plan_connect(&settings, &config, Path::new(BRIDGE)).unwrap();
        let backup = apply(&plan, &config_path, now).unwrap();

        let connected = file_bytes(case.bom, case.connected);
        assert_eq!(
            std::fs::read(&settings_path).unwrap(),
            connected,
            "{name}: settings.json after Connect"
        );
        assert_eq!(
            std::fs::read_to_string(&config_path).unwrap(),
            case.config_when_connected,
            "{name}: config.json after Connect"
        );
        assert_eq!(
            backup.map(|copy| std::fs::read(copy).unwrap()),
            before,
            "{name}: the backup Connect made"
        );

        let settings = read_settings(&settings_path).unwrap();
        let saved = AppConfig::load(&config_path).unwrap();
        let plan = plan_disconnect(&settings, &saved).unwrap();
        let backup = apply(&plan, &config_path, now).unwrap();

        assert_eq!(
            std::fs::read(&settings_path).unwrap(),
            file_bytes(case.bom, case.disconnected),
            "{name}: settings.json after Disconnect"
        );
        assert_eq!(
            std::fs::read_to_string(&config_path).unwrap(),
            case.config_when_disconnected,
            "{name}: config.json after Disconnect"
        );
        assert_eq!(
            backup.map(|copy| std::fs::read(copy).unwrap()),
            Some(connected),
            "{name}: the backup Disconnect made"
        );
    }
}

/// The promise in its own words, on the same samples: Connect over a status line changes the
/// value of `statusLine` and not one byte before or after it.
#[test]
fn connect_changes_nothing_outside_the_status_line_value() {
    let mut checked = 0;
    for case in CASES {
        let Some(before) = case.before else {
            continue;
        };
        let Some(value) = read_status_line(before).unwrap() else {
            continue;
        };
        // `value` is a piece of `before`: where it starts is how far its first byte is from
        // the first byte of the whole text.
        let start = value.as_ptr() as usize - before.as_ptr() as usize;
        let (head, tail) = (&before[..start], &before[start + value.len()..]);
        assert!(
            case.connected.starts_with(head),
            "{}: what comes before the value",
            case.name
        );
        assert!(
            case.connected.ends_with(tail),
            "{}: what comes after the value",
            case.name
        );
        checked += 1;
    }
    assert!(checked >= 10, "{checked}");
}
