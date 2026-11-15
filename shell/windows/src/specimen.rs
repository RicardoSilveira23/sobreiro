//! `--specimen`: draws the specimen of the three fonts into memory, with the software renderer and
//! no window, and prints how many pixels wide the ink of each line is. The build's tests read it.
use crate::ui::Specimen;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, PhysicalSize, PlatformError, Rgb8Pixel};
use std::process::ExitCode;
use std::rc::Rc;

const WIDTH: usize = 480;
const BAND: usize = 40;
const LINES: [&str; 3] = ["Inter 400", "Inter 500", "Unbounded 700"];

/// Gives the toolkit a window that exists only in memory, in place of one on the screen.
struct InMemory(Rc<MinimalSoftwareWindow>);

impl Platform for InMemory {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

pub fn run() -> ExitCode {
    match draw() {
        Ok(pixels) => {
            for (line, name) in LINES.iter().enumerate() {
                println!(
                    "{name}: {}",
                    ink_width(&pixels[line * BAND * WIDTH..(line + 1) * BAND * WIDTH])
                );
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn draw() -> Result<Vec<Rgb8Pixel>, PlatformError> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(InMemory(window.clone())))
        .map_err(|error| PlatformError::Other(error.to_string()))?;
    let specimen = Specimen::new()?;
    specimen.show()?;
    window.set_size(PhysicalSize::new(WIDTH as u32, (LINES.len() * BAND) as u32));
    let mut pixels = vec![Rgb8Pixel::default(); WIDTH * LINES.len() * BAND];
    window.draw_if_needed(|renderer| {
        renderer.render(&mut pixels, WIDTH);
    });
    Ok(pixels)
}

/// From the first dark pixel to the last, over every row of one band.
fn ink_width(band: &[Rgb8Pixel]) -> usize {
    let dark = |x: usize| {
        band.iter()
            .skip(x)
            .step_by(WIDTH)
            .any(|pixel| pixel.r < 128)
    };
    let Some(first) = (0..WIDTH).find(|&x| dark(x)) else {
        return 0;
    };
    let last = (0..WIDTH).rev().find(|&x| dark(x)).unwrap_or(first);
    last - first + 1
}
