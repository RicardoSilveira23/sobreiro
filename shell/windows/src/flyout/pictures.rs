//! `--flyout-pictures <folder>`: draws the flyout's card in every state, theme, bar colour and
//! scale into memory, with the software renderer and no window, and writes each as a PNG with a
//! transparent ground outside the card's rounded border. Nothing else is read or written: no
//! tray icon, no network, no registry, no `config.json`.
use super::face::flyout_data;
use super::states::states;
use crate::ui::FlyoutPicture;
use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{ComponentHandle, PhysicalSize, PlatformError};
use sobreiro_core::colours::bar_colour::pick;
use sobreiro_core::colours::{BarColourOption, NightPalette, WINDOWS_BLUE};
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

/// The scales the card is drawn at: 100%, 150% and 200%.
pub const SCALES: [f32; 3] = [1.0, 1.5, 2.0];

/// Gives the toolkit a window that exists only in memory, in place of one on the screen.
struct InMemory(Rc<MinimalSoftwareWindow>);

impl Platform for InMemory {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

/// The themes the pictures come in, and whether each is the dark palette.
pub fn themes() -> [(&'static str, bool); 2] {
    [("light", false), ("dark", true)]
}

/// The bar colours the pictures come in.
pub fn bar_colours() -> [(&'static str, BarColourOption); 2] {
    [
        ("purple", BarColourOption::Purple),
        ("accent", BarColourOption::Accent),
    ]
}

/// The name of one picture: `flyout-<state>-<theme>-<colour>-<scale>x.png`.
pub fn file_name(state: &str, theme: &str, colour: &str, scale: f32) -> String {
    format!("flyout-{state}-{theme}-{colour}-{scale}x.png")
}

pub fn run(folder: Option<std::ffi::OsString>) -> ExitCode {
    let Some(folder) = folder.filter(|folder| !folder.is_empty()) else {
        eprintln!("--flyout-pictures needs a folder to write to.");
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
    let picture = FlyoutPicture::new().map_err(|e| e.to_string())?;
    picture.show().map_err(|e| e.to_string())?;

    let mut count = 0;
    for (state, model) in states() {
        for (theme, dark) in themes() {
            for (colour, option) in bar_colours() {
                let accent = (option == BarColourOption::Accent).then_some(&WINDOWS_BLUE);
                let fill = pick(option, accent, dark).colour;
                picture.set_data(flyout_data(&model, &NightPalette::for_dark(dark), fill));
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

/// The picture at one scale: as many pixels as the card's size in DIPs makes at that scale,
/// rounded up so no edge is cut, as straight (not premultiplied) RGBA.
fn render(
    window: &Rc<MinimalSoftwareWindow>,
    picture: &FlyoutPicture,
    scale: f32,
) -> (u32, u32, Vec<u8>) {
    window
        .window()
        .dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: scale,
        });
    let width = (picture.get_card_width() * scale).ceil() as u32;
    let height = (picture.get_card_height() * scale).ceil() as u32;
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
            17 * 2 * 2 * 3
        );
    }

    #[test]
    fn a_file_is_named_by_what_it_shows() {
        assert_eq!(
            file_name("both", "dark", "purple", 1.5),
            "flyout-both-dark-purple-1.5x.png"
        );
        assert_eq!(
            file_name("far-ends", "light", "accent", 1.0),
            "flyout-far-ends-light-accent-1x.png"
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
