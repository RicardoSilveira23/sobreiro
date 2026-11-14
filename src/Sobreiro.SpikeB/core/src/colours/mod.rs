//! Colours: the palettes and the contrast rules that pick the bar and ring colours.

mod accent_palette;
mod bar_colour;
mod contrast;
mod night_palette;
mod rgb;
mod taskbar_colours;

pub use accent_palette::AccentPalette;
pub use bar_colour::{BarColour, BarColourChoice, BarColourOption, BarColourSource};
pub use contrast::Contrast;
pub use night_palette::{BadgeColours, NightPalette};
pub use rgb::Rgb;
pub use taskbar_colours::TaskbarColours;
