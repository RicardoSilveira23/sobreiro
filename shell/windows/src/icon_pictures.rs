//! `--icon-pictures <folder>`: draws the tray icon's rings in every state, theme and size with
//! GDI+ and writes each as a PNG, straight alpha. Nothing else is read or written: no window, no
//! network, no registry, no `config.json`.
use crate::win::tray_picture::{Picture, draw};
use sobreiro_core::colours::{BarColourOption, TaskbarColours};
use sobreiro_core::display::rings::{Ring, RingsModel};
use sobreiro_core::limits::usage_level::for_percent;
use std::path::Path;
use std::process::ExitCode;

/// The sizes the icon is drawn at, in pixels.
pub const SIZES: [u32; 4] = [16, 20, 24, 32];

fn ring(percent: i32) -> Ring {
    Ring {
        percent,
        level: for_percent(percent),
        has_data: true,
    }
}

fn claude(session: i32, weekly: i32) -> RingsModel {
    RingsModel {
        outer: ring(session),
        inner: ring(weekly),
        has_inner: true,
    }
}

fn copilot(percent: i32) -> RingsModel {
    RingsModel {
        outer: ring(percent),
        inner: Ring::NONE,
        has_inner: false,
    }
}

/// One state of the icon, by the name its pictures carry.
pub fn states() -> Vec<(&'static str, RingsModel)> {
    vec![
        ("fresh", claude(15, 45)),
        ("half", claude(50, 45)),
        ("amber", claude(75, 72)),
        ("red", claude(95, 91)),
        ("full", claude(100, 100)),
        (
            "no-reading",
            RingsModel {
                outer: Ring::NONE,
                inner: Ring::NONE,
                has_inner: true,
            },
        ),
        ("copilot", copilot(17)),
        ("copilot-red", copilot(92)),
    ]
}

/// The taskbar themes the pictures come in; `true` is a light taskbar.
pub fn themes() -> [(&'static str, bool); 2] {
    [("light", true), ("dark", false)]
}

/// The name of one picture: `icon-<state>-<theme>-<size>.png`.
pub fn file_name(state: &str, theme: &str, size: u32) -> String {
    format!("icon-{state}-{theme}-{size}.png")
}

pub fn run(folder: Option<std::ffi::OsString>) -> ExitCode {
    let Some(folder) = folder else {
        eprintln!("--icon-pictures needs a folder to write to.");
        return ExitCode::from(2);
    };
    match draw_all(Path::new(&folder)) {
        Ok(count) => {
            println!("{count} pictures");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn draw_all(folder: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut count = 0;
    for (state, model) in states() {
        for (theme, light) in themes() {
            let colours = TaskbarColours::for_taskbar(light, false, BarColourOption::Purple, None);
            for size in SIZES {
                let picture = draw(&model, &colours, size).map_err(|e| e.to_string())?;
                write_png(&folder.join(file_name(state, theme, size)), &picture)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

fn write_png(path: &Path, picture: &Picture) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), picture.size, picture.size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&picture.rgba)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_is_every_state_theme_and_size() {
        assert_eq!(states().len() * themes().len() * SIZES.len(), 64);
    }

    #[test]
    fn a_file_is_named_by_what_it_shows() {
        assert_eq!(
            file_name("copilot-red", "dark", 24),
            "icon-copilot-red-dark-24.png"
        );
    }
}
