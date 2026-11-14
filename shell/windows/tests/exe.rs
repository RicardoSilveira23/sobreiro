//! The build gives one exe with nothing beside it: the fonts and the C runtime are inside, and it
//! runs from a folder that holds nothing else.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");
const FONTS: [&str; 3] = [
    "Inter-Regular.ttf",
    "Inter-Medium.ttf",
    "Unbounded-Bold.ttf",
];

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Runs a copy of the exe that sits alone in a new folder, with a data folder and a Claude config
/// folder of its own, so no run of a test can read or write the user's.
fn run_alone(name: &str, argument: &str) -> Output {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let exe: PathBuf = folder.join("Sobreiro.exe");
    std::fs::copy(EXE, &exe).unwrap();
    let output = Command::new(&exe)
        .arg(argument)
        .current_dir(&folder)
        .env("SOBREIRO_DATA_DIR", folder.join("data"))
        .env("CLAUDE_CONFIG_DIR", folder.join("claude"))
        .output()
        .unwrap();
    let left: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["Sobreiro.exe"], "the run left files beside the exe");
    output
}

#[test]
fn the_fonts_are_inside_the_exe() {
    let exe = std::fs::read(EXE).unwrap();
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/Sobreiro/Fonts");
    for name in FONTS {
        let font = std::fs::read(fonts.join(name)).unwrap();
        assert!(contains(&exe, &font), "{name} is not inside the exe");
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
