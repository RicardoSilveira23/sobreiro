use super::Rgb;
use crate::display::StatusKind;
use crate::limits::UsageLevel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadgeColours {
    pub fill: Rgb,
    pub text: Rgb,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NightPalette {
    pub ground: Rgb,
    pub ink: Rgb,
    pub soft: Rgb,
    pub line: Rgb,
    pub track: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
    pub purple: Rgb,
    pub is_dark: bool,
}

impl NightPalette {
    pub fn dark() -> NightPalette {
        NightPalette {
            ground: Rgb::hex("#0b0b0b"), ink: Rgb::hex("#f4f4f4"), soft: Rgb::hex("#8d8d8d"),
            line: Rgb::hex("#262626"), track: Rgb::hex("#262626"),
            warning: Rgb::hex("#ffb020"), error: Rgb::hex("#ff6b6b"), purple: Rgb::hex("#a594ff"), is_dark: true,
        }
    }

    pub fn light() -> NightPalette {
        NightPalette {
            ground: Rgb::hex("#ffffff"), ink: Rgb::hex("#0d0d0d"), soft: Rgb::hex("#767676"),
            line: Rgb::hex("#e4e4e4"), track: Rgb::hex("#e4e4e4"),
            warning: Rgb::hex("#a35f00"), error: Rgb::hex("#c4262e"), purple: Rgb::hex("#5a3dd1"), is_dark: false,
        }
    }

    /// `NightPalette.For(dark)`.
    pub fn for_dark(dark: bool) -> NightPalette {
        if dark { Self::dark() } else { Self::light() }
    }

    /// Fill and text of a status badge. Text on fill is at least 4.5:1 in both palettes.
    pub fn badge(&self, kind: StatusKind) -> BadgeColours {
        let white = Rgb::hex("#ffffff");
        let on = if self.is_dark { self.ground } else { white };
        match kind {
            StatusKind::OnTrack => BadgeColours { fill: Rgb::hex("#1a7f37"), text: white },
            StatusKind::Ends => BadgeColours { fill: self.warning, text: on },
            StatusKind::UsedUp => BadgeColours { fill: self.error, text: on },
            _ => BadgeColours { fill: self.track, text: self.ink },
        }
    }

    pub fn level_colour(&self, level: UsageLevel, normal: Rgb) -> Rgb {
        match level {
            UsageLevel::Warning => self.warning,
            UsageLevel::Error => self.error,
            UsageLevel::Normal => normal,
        }
    }
}
