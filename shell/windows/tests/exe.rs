//! The build gives one exe with nothing beside it: the fonts and the C runtime are inside, and it
//! runs from a folder that holds nothing else.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use sobreiro_core::connect::bridge_command::{command_for, status_line_json};

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");
const FONTS: [&str; 3] = [
    "Inter-Regular.ttf",
    "Inter-Medium.ttf",
    "Unbounded-Bold.ttf",
];
const TABULAR_FONTS: [&str; 3] = [
    "InterTabular-Regular.ttf",
    "InterTabular-Medium.ttf",
    "UnboundedTabular-Bold.ttf",
];

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// A new folder for one test, with a copy of the exe alone in its `app` folder. The data folder and
/// the Claude config folder are `data` and `claude` beside `app`, so no run of a test can read or
/// write the user's, and nothing is written next to the exe.
struct Layout {
    root: PathBuf,
}

impl Layout {
    fn new(name: &str) -> Layout {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::copy(EXE, root.join("app").join("Sobreiro.exe")).unwrap();
        Layout { root }
    }

    fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    fn bridge(&self) -> PathBuf {
        self.data().join("sobreiro-bridge.exe")
    }

    /// Writes the Claude config folder's `settings.json`.
    fn settings(&self, text: &str) {
        std::fs::create_dir_all(self.root.join("claude")).unwrap();
        std::fs::write(self.root.join("claude").join("settings.json"), text).unwrap();
    }

    /// Writes a bridge into the data folder.
    fn put_bridge(&self, bytes: &[u8]) {
        std::fs::create_dir_all(self.data()).unwrap();
        std::fs::write(self.bridge(), bytes).unwrap();
    }

    fn run(&self, argument: &str) -> Output {
        let app = self.root.join("app");
        let output = Command::new(app.join("Sobreiro.exe"))
            .arg(argument)
            .current_dir(&app)
            .env("SOBREIRO_DATA_DIR", self.data())
            .env("CLAUDE_CONFIG_DIR", self.root.join("claude"))
            .output()
            .unwrap();
        let left: Vec<_> = std::fs::read_dir(&app)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(left, ["Sobreiro.exe"], "the run left files beside the exe");
        output
    }

    /// What `--refresh-bridge` says: the one word of the outcome.
    fn refresh(&self) -> String {
        let output = self.run("--refresh-bridge");
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

/// Runs a copy of the exe that sits alone in a new folder.
fn run_alone(name: &str, argument: &str) -> Output {
    Layout::new(name).run(argument)
}

/// A `settings.json` whose status line runs the bridge at `path`.
fn settings_running(path: &Path) -> String {
    let line = status_line_json(&command_for(path), None).unwrap();
    format!("{{\"statusLine\": {line}}}")
}

/// The bridge this build carries, when it carries one.
fn built_bridge() -> Option<Vec<u8>> {
    option_env!("SOBREIRO_BRIDGE_EXE").map(|path| std::fs::read(path).unwrap())
}

#[test]
fn the_fonts_are_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let folders = [
        root.join("src/Sobreiro/Fonts"),
        root.join("shell/windows/fonts"),
    ];
    for (folder, names) in folders.iter().zip([FONTS, TABULAR_FONTS]) {
        for name in names {
            let font = std::fs::read(folder.join(name)).unwrap();
            assert!(contains(&exe, &font), "{name} is not inside the exe");
        }
    }
}

#[test]
fn the_c_runtime_is_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap().to_ascii_lowercase();
    for dll in ["vcruntime", "api-ms-win-crt"] {
        assert!(
            !contains(&exe, dll.as_bytes()),
            "the exe asks Windows for a {dll} DLL"
        );
    }
}

#[test]
fn alone_in_a_folder_it_says_its_version() {
    let output = run_alone("version", "--version");
    assert!(output.status.success());
    let expected = format!("Sobreiro {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

/// `--specimen` draws one line of text in each font, with no window, and prints how many pixels
/// wide the ink of each line is. Three different faces give three different widths; a font that
/// was not found would be drawn in another one's face.
#[test]
fn alone_in_a_folder_it_draws_each_font() {
    let output = run_alone("specimen", "--specimen");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let widths: Vec<(&str, u32)> = text
        .lines()
        .map(|line| {
            line.split_once(": ")
                .unwrap_or_else(|| panic!("not a line of the specimen: {line}"))
        })
        .map(|(name, width)| (name, width.parse().unwrap()))
        .collect();
    let names: Vec<&str> = widths.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["Inter 400", "Inter 500", "Unbounded 700"]);
    let (regular, medium, bold) = (widths[0].1, widths[1].1, widths[2].1);
    assert!(regular > 0, "Inter 400 drew nothing");
    assert!(
        medium > regular,
        "Inter 500 ({medium}) is not wider than Inter 400 ({regular})"
    );
    assert!(
        bold > medium,
        "Unbounded 700 ({bold}) is not wider than Inter 500 ({medium})"
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn the_bridge_is_inside_the_exe() {
    let bridge = built_bridge().unwrap();
    assert!(bridge.starts_with(b"MZ"), "the bridge is not a PE file");
    let exe = std::fs::read(EXE).unwrap();
    assert!(contains(&exe, &bridge), "the bridge is not inside the exe");
}

#[test]
#[cfg_attr(not(debug_assertions), ignore = "the release build carries the bridge")]
fn without_the_bridge_the_step_says_so() {
    let layout = Layout::new("bridge-not-carried");
    layout.settings(&settings_running(&layout.bridge()));
    assert_eq!(layout.refresh(), "not-carried");
    assert!(!layout.bridge().exists());
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn the_start_writes_the_bridge_and_then_finds_it_current() {
    let layout = Layout::new("bridge-written");
    layout.settings(&settings_running(&layout.bridge()));

    assert_eq!(layout.refresh(), "written");
    assert_eq!(
        std::fs::read(layout.bridge()).unwrap(),
        built_bridge().unwrap()
    );
    assert_eq!(layout.refresh(), "current");
    the_written_bridge_runs(&layout);

    let log = std::fs::read_to_string(layout.data().join("log.txt")).unwrap();
    let line = log
        .lines()
        .find(|line| line.contains("bridge written"))
        .unwrap();
    assert!(!line.contains('\\') && !line.contains(":/"), "{line}");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn a_stale_copy_is_replaced_at_the_start() {
    let layout = Layout::new("bridge-stale");
    layout.settings(&settings_running(&layout.bridge()));
    layout.put_bridge(b"an older bridge");

    assert_eq!(layout.refresh(), "written");
    assert_eq!(
        std::fs::read(layout.bridge()).unwrap(),
        built_bridge().unwrap()
    );
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn a_status_line_that_runs_something_else_leaves_the_copy_alone() {
    let layout = Layout::new("bridge-elsewhere");
    layout.settings(&settings_running(Path::new(
        "C:/elsewhere/sobreiro-bridge.exe",
    )));

    assert_eq!(layout.refresh(), "not-used");
    assert!(!layout.bridge().exists());

    layout.put_bridge(b"an older bridge");
    assert_eq!(layout.refresh(), "not-used");
    assert_eq!(std::fs::read(layout.bridge()).unwrap(), b"an older bridge");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "only the release build carries the bridge")]
fn without_settings_the_copy_is_not_used() {
    let layout = Layout::new("bridge-no-settings");
    assert_eq!(layout.refresh(), "not-used");
    assert!(!layout.bridge().exists());
}

/// Runs the data folder's copy as Claude Code's status line does, with a status line's JSON on
/// its input: it prints the line and writes the limits.
fn the_written_bridge_runs(layout: &Layout) {
    let input = r#"{"rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600}}}"#;
    let mut child = Command::new(layout.bridge())
        .env("SOBREIRO_DATA_DIR", layout.data())
        .env("CLAUDE_CONFIG_DIR", layout.root.join("claude"))
        .env_remove("SOBREIRO_IN_CHAIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("5h 23%"),
        "the bridge printed no status line"
    );
    let usage = std::fs::read_to_string(layout.data().join("usage.json")).unwrap();
    assert!(usage.contains("\"used_percentage\":23.5"), "{usage}");
}
