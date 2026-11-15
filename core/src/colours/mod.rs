//! Colours: a colour and its hex text, the contrast between two, the Windows accent palette,
//! Sobreiro's dark and light palettes with the colours of a status badge, the colour of the
//! bars, and the ink and rings of the taskbar. The whole area of `Sobreiro.Core/Colours`.
pub mod accent_palette;
pub mod bar_colour;
pub mod contrast;
pub mod night_palette;
pub mod rgb;
pub mod taskbar_colours;

pub use accent_palette::{AccentPalette, WINDOWS_BLUE};
pub use bar_colour::{BarColourChoice, BarColourOption, BarColourSource};
pub use night_palette::{BadgeColours, NightPalette};
pub use rgb::{Rgb, RgbError};
pub use taskbar_colours::TaskbarColours;
