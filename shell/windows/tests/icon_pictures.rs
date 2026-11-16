//! The tray icon's pictures, drawn by `Sobreiro.exe --icon-pictures`: the full set, each a square
//! of the size in its name with clear corners, and the argument it needs.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");

/// A copy of the exe alone in a new folder, with a data folder and a Claude config folder of its
/// own, run with `args`.
fn run(name: &str, args: &[&str]) -> (PathBuf, Output) {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let exe = folder.join("Sobreiro.exe");
    std::fs::copy(EXE, &exe).unwrap();
    let output = Command::new(&exe)
        .args(args)
        .current_dir(&folder)
        .env("SOBREIRO_DATA_DIR", folder.join("data"))
        .env("CLAUDE_CONFIG_DIR", folder.join("claude"))
        .output()
        .unwrap();
    (folder, output)
}

/// The run that writes the pictures, once for all the tests.
fn pictures() -> &'static (PathBuf, Output) {
    static RUN: OnceLock<(PathBuf, Output)> = OnceLock::new();
    RUN.get_or_init(|| run("icons", &["--icon-pictures", "pictures"]))
}

fn files() -> Vec<PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(pictures().0.join("pictures"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    files.sort();
    files
}

/// A picture decoded: its side and its straight RGBA bytes.
fn decode(path: &Path) -> (usize, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut bytes).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "{path:?}");
    assert_eq!(info.bit_depth, png::BitDepth::Eight, "{path:?}");
    assert_eq!(info.width, info.height, "{path:?}");
    bytes.truncate(info.buffer_size());
    (info.width as usize, bytes)
}

#[test]
fn the_run_writes_the_whole_set() {
    let (_, output) = pictures();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "64 pictures"
    );
    let files = files();
    assert_eq!(files.len(), 64);
    for file in files {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("icon-") && name.ends_with(".png"),
            "{name}"
        );
    }
}

#[test]
fn each_picture_is_a_square_of_its_size_with_clear_corners() {
    for file in files() {
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let size: usize = name.rsplit('-').next().unwrap().parse().unwrap();
        let (side, rgba) = decode(&file);
        assert_eq!(side, size, "{name}");
        let last = side - 1;
        for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
            assert_eq!(rgba[(y * side + x) * 4 + 3], 0, "{name} ({x}, {y})");
        }
    }
}

#[test]
fn the_half_picture_is_filled_on_the_outer_rings_right() {
    let (side, rgba) = decode(&pictures().0.join("pictures").join("icon-half-light-20.png"));
    assert_eq!(side, 20);
    // The outer ring's stroke at 20 px spans x = 16.9 to 20 on the centre row.
    assert_eq!(rgba[(10 * side + 18) * 4 + 3], 255);
}

#[test]
fn without_a_folder_the_run_stops_with_code_2() {
    let (_, output) = run("icons-no-folder", &["--icon-pictures"]);
    assert_eq!(output.status.code(), Some(2));
}
