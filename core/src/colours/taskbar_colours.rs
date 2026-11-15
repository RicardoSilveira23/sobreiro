//! The ink and ring colours of the taskbar pill and the tray icon.
use super::Rgb;
use super::accent_palette::AccentPalette;
use super::bar_colour::{self, BarColourOption};
use super::night_palette::NightPalette;
use crate::limits::usage_level::UsageLevel;

/// Alpha of the ring's track: the ink at about 15%.
pub const TRACK_ALPHA: u8 = 0x26;

/// Alpha of the tray icon's ring track: the ink at about 35%. At 16 pixels the pill's 15%
/// vanishes.
pub const ICON_TRACK_ALPHA: u8 = 0x59;

/// Alpha of a stopped block's ring or bar fill on the pill: the ink at half strength, in place
/// of the level colour.
pub const STALE_ALPHA: u8 = 0x80;

/// The ink on the taskbar, the colour of the rings at a normal level, and the two urgent ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskbarColours {
    pub ink: Rgb,
    pub base: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
}

impl TaskbarColours {
    /// The colours for a light or a dark taskbar. On a taskbar that takes the Windows accent as
    /// its colour the rings are the ink, since the accent would not show on itself. Otherwise
    /// they are the bar colour that [`bar_colour::pick`] gives for that taskbar.
    pub fn for_taskbar(
        light_taskbar: bool,
        accent_taskbar: bool,
        option: BarColourOption,
        accent: Option<&AccentPalette>,
    ) -> TaskbarColours {
        let ink = if light_taskbar {
            Rgb::new(0x1A, 0x1A, 0x1A)
        } else {
            Rgb::new(0xFF, 0xFF, 0xFF)
        };
        let night = NightPalette::for_dark(!light_taskbar);
        let base = if accent_taskbar {
            ink
        } else {
            bar_colour::pick(option, accent, !light_taskbar).colour
        };
        TaskbarColours {
            ink,
            base,
            warning: night.warning,
            error: night.error,
        }
    }

    /// The colour of a ring or a bar at that level.
    pub fn fill(&self, level: UsageLevel) -> Rgb {
        match level {
            UsageLevel::Warning => self.warning,
            UsageLevel::Error => self.error,
            UsageLevel::Normal => self.base,
        }
    }
}
