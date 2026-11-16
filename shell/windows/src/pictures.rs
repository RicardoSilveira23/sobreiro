//! `--pill-pictures <folder>`: draws the pill in every state, theme, bar colour and scale into
//! memory, with the software renderer and no window, and writes each as a PNG with a transparent
//! ground outside the pill's rounded border. Nothing else is read or written: no tray icon, no
//! network, no registry, no `config.json`.
use crate::pill_face::face_data;
use crate::ui::PillPicture;
use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, PhysicalSize, PlatformError};
use sobreiro_core::colours::{BarColourOption, TaskbarColours, WINDOWS_BLUE};
use sobreiro_core::display::pill_model::{PillCopilot, PillLine, PillModel};
use sobreiro_core::display::rings::{Ring, RingsModel};
use sobreiro_core::limits::UsageLevel;
use sobreiro_core::limits::usage_level::for_percent;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

/// The scales the pill is drawn at: 100%, 150% and 200%.
pub const SCALES: [f32; 3] = [1.0, 1.5, 2.0];

/// Gives the toolkit a window that exists only in memory, in place of one on the screen.
struct InMemory(Rc<MinimalSoftwareWindow>);

impl Platform for InMemory {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

/// One state of the pill, by the name its pictures carry. `digits-11` and `digits-88` differ in
/// one figure only: the test of the figure box compares them.
pub fn states() -> Vec<(&'static str, PillModel)> {
    let mut both = claude(15, 45, false);
    both.copilot = Some(copilot(17, false));
    let mut only = claude(15, 45, false);
    only.show_claude = false;
    only.copilot = Some(copilot(17, false));
    let mut stale = claude(15, 45, true);
    stale.copilot = Some(copilot(17, true));
    let mut none = claude(0, 0, false);
    none.session = line("5h", "–".to_string(), false);
    none.weekly = line("7d", "–".to_string(), false);
    none.rings = RingsModel {
        outer: Ring::NONE,
        inner: Ring::NONE,
        has_inner: true,
    };
    vec![
        ("fresh", claude(15, 45, false)),
        ("amber", claude(75, 72, false)),
        ("red", claude(95, 91, false)),
        ("stale", stale),
        ("no-reading", none),
        ("copilot-on", both),
        ("copilot-only", only),
        ("digits-11", claude(15, 11, false)),
        ("digits-88", claude(15, 88, false)),
        // The rings at two levels, so a check on one ring cannot pass on the other.
        ("levels", claude(15, 91, false)),
        // Claude's block alone, stopped: its rings are where the other Claude-only states have them.
        ("stale-claude", claude(15, 45, true)),
    ]
}

/// The themes (the taskbar's, so "light" has dark ink) and bar colours the pictures come in.
pub fn themes() -> [(&'static str, bool); 2] {
    [("light", true), ("dark", false)]
}

pub fn bar_colours() -> [(&'static str, BarColourOption); 2] {
    [
        ("purple", BarColourOption::Purple),
        ("accent", BarColourOption::Accent),
    ]
}

/// The name of one picture: `pill-<state>-<theme>-<colour>-<scale>x.png`.
pub fn file_name(state: &str, theme: &str, colour: &str, scale: f32) -> String {
    format!("pill-{state}-{theme}-{colour}-{scale}x.png")
}

fn line(label: &'static str, figure: String, stale: bool) -> PillLine {
    PillLine {
        label,
        figure,
        stale,
    }
}

fn ring(percent: i32) -> Ring {
    Ring {
        percent,
        level: for_percent(percent),
        has_data: true,
    }
}

fn claude(session: i32, weekly: i32, stale: bool) -> PillModel {
    PillModel {
        session: line("5h", format!("{session}%"), stale),
        weekly: line("7d", format!("{weekly}%"), stale),
        rings: RingsModel {
            outer: ring(session),
            inner: ring(weekly),
            has_inner: true,
        },
        copilot: None,
        show_claude: true,
        claude_stale: stale,
    }
}

/// Copilot's block at a percent of the month's credits, resetting on the first of December.
fn copilot(percent: i32, stale: bool) -> PillCopilot {
    PillCopilot {
        credits: line("AI", format!("{percent}%"), stale),
        reset_date: "1 Dec".to_string(),
        fraction: f64::from(percent) / 100.0,
        level: UsageLevel::Normal,
        stale,
    }
}

pub fn run(folder: Option<std::ffi::OsString>) -> ExitCode {
    let Some(folder) = folder else {
        eprintln!("--pill-pictures needs a folder to write to.");
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
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(InMemory(window.clone()))).map_err(|e| e.to_string())?;
    let picture = PillPicture::new().map_err(|e| e.to_string())?;
    picture.show().map_err(|e| e.to_string())?;

    let mut count = 0;
    for (state, model) in states() {
        for (theme, light) in themes() {
            for (colour, option) in bar_colours() {
                let accent = (option == BarColourOption::Accent).then_some(&WINDOWS_BLUE);
                let colours = TaskbarColours::for_taskbar(light, false, option, accent);
                picture.set_data(face_data(&model, &colours, false));
                for scale in SCALES {
                    let (width, height, pixels) = render(&window, &picture, scale);
                    let path = folder.join(file_name(state, theme, colour, scale));
                    write_png(&path, width, height, &pixels)?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// The picture at one scale: as many pixels as the pill's size in DIPs makes at that scale,
/// rounded up so no edge is cut, as straight (not premultiplied) RGBA.
fn render(
    window: &Rc<MinimalSoftwareWindow>,
    picture: &PillPicture,
    scale: f32,
) -> (u32, u32, Vec<u8>) {
    window
        .window()
        .dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
    // The blocks the pill shows are made on the first draw, so its size is not known before one.
    window.set_size(PhysicalSize::new(1, 1));
    window.window().request_redraw();
    window.draw_if_needed(|renderer| {
        renderer.render(&mut [PremultipliedRgbaColor::default()], 1);
    });
    let width = (picture.get_pill_width() * scale).ceil() as u32;
    let height = (picture.get_pill_height() * scale).ceil() as u32;
    window.set_size(PhysicalSize::new(width, height));
    let mut buffer = vec![PremultipliedRgbaColor::default(); (width * height) as usize];
    window.window().request_redraw();
    window.draw_if_needed(|renderer| {
        renderer.render(&mut buffer, width as usize);
    });
    let mut bytes = Vec::with_capacity(buffer.len() * 4);
    for pixel in &buffer {
        bytes.extend_from_slice(&straight(pixel));
    }
    (width, height, bytes)
}

/// A premultiplied pixel as straight RGBA: each colour divided by the alpha again.
fn straight(pixel: &PremultipliedRgbaColor) -> [u8; 4] {
    let alpha = u32::from(pixel.alpha);
    if alpha == 0 {
        return [0, 0, 0, 0];
    }
    let undo = |c: u8| ((u32::from(c) * 255 + alpha / 2) / alpha).min(255) as u8;
    [
        undo(pixel.red),
        undo(pixel.green),
        undo(pixel.blue),
        pixel.alpha,
    ]
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_set_is_every_state_theme_colour_and_scale() {
        assert_eq!(
            states().len() * themes().len() * bar_colours().len() * SCALES.len(),
            132
        );
    }

    #[test]
    fn a_file_is_named_by_what_it_shows() {
        assert_eq!(
            file_name("amber", "dark", "purple", 1.5),
            "pill-amber-dark-purple-1.5x.png"
        );
        assert_eq!(
            file_name("fresh", "light", "accent", 1.0),
            "pill-fresh-light-accent-1x.png"
        );
    }

    #[test]
    fn a_pixel_is_unpremultiplied_without_losing_its_colour() {
        let half = PremultipliedRgbaColor {
            red: 64,
            green: 64,
            blue: 64,
            alpha: 128,
        };
        assert_eq!(straight(&half), [128, 128, 128, 128]);
        assert_eq!(straight(&PremultipliedRgbaColor::default()), [0, 0, 0, 0]);
    }
}
