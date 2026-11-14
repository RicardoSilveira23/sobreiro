use super::{AccentPalette, BarColour, BarColourOption, NightPalette, Rgb};
use crate::limits::UsageLevel;

/// Ink and ring colours for the taskbar pill and tray rings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskbarColours {
    pub ink: Rgb,
    pub base: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
}

impl TaskbarColours {
    /// Alpha of the ring track (the ink at about 15%).
    pub const TRACK_ALPHA: u8 = 0x26;

    /// Alpha of the tray icon's ring track (the ink at about 35%): at 16 px the pill's 15% vanishes.
    pub const ICON_TRACK_ALPHA: u8 = 0x59;

    /// Alpha of a stopped block's ring or bar fill on the pill (the ink at half strength, in place of the level colour).
    pub const STALE_ALPHA: u8 = 0x80;

    /// `TaskbarColours.For(...)`.
    pub fn for_taskbar(light_taskbar: bool, accent_taskbar: bool, option: BarColourOption, accent: Option<&AccentPalette>) -> TaskbarColours {
        let ink = if light_taskbar { Rgb::hex("#1A1A1A") } else { Rgb::hex("#FFFFFF") };
        let night = NightPalette::for_dark(!light_taskbar);
        let base = if accent_taskbar { ink } else { BarColour::pick(option, accent, !light_taskbar).colour };
        TaskbarColours { ink, base, warning: night.warning, error: night.error }
    }

    pub fn fill(&self, level: UsageLevel) -> Rgb {
        match level {
            UsageLevel::Warning => self.warning,
            UsageLevel::Error => self.error,
            UsageLevel::Normal => self.base,
        }
    }
}
