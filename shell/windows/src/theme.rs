//! What the pill takes from Windows' look: whether the taskbar is light, whether it takes the
//! accent colour, and the accent palette. The registry is read in `win/registry.rs`; this is the
//! judging of what it gave, so a test can drive it with no registry.
use sobreiro_core::colours::{AccentPalette, BarColourOption, TaskbarColours};

/// The three values 1.1's `WindowsTheme` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Theme {
    /// `SystemUsesLightTheme` is set. A taskbar with no such value is dark, as in 1.1.
    pub light: bool,
    /// `ColorPrevalence` is set: the taskbar takes the accent colour.
    pub accent_taskbar: bool,
    /// The accent palette, when the registry holds one.
    pub accent: Option<AccentPalette>,
}

impl Theme {
    /// The colours the pill is drawn in, for the bar colour the user chose.
    pub fn colours(&self, bar_colour: BarColourOption) -> TaskbarColours {
        TaskbarColours::for_taskbar(
            self.light,
            self.accent_taskbar,
            bar_colour,
            self.accent.as_ref(),
        )
    }
}
