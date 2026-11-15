//! Sobreiro's own colours, a dark set and a light one.
use super::Rgb;
use crate::display::limit_status::StatusKind;
use crate::limits::usage_level::UsageLevel;

/// The fill and the text of a status badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgeColours {
    pub fill: Rgb,
    pub text: Rgb,
}

/// The colours of one theme: the ground, the ink on it, a softer ink, a line, the track of a
/// bar, the two urgent colours and Sobreiro's purple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// The colours on a dark ground.
pub const DARK: NightPalette = NightPalette {
    ground: Rgb::new(0x0b, 0x0b, 0x0b),
    ink: Rgb::new(0xf4, 0xf4, 0xf4),
    soft: Rgb::new(0x8d, 0x8d, 0x8d),
    line: Rgb::new(0x26, 0x26, 0x26),
    track: Rgb::new(0x26, 0x26, 0x26),
    warning: Rgb::new(0xff, 0xb0, 0x20),
    error: Rgb::new(0xff, 0x6b, 0x6b),
    purple: Rgb::new(0xa5, 0x94, 0xff),
    is_dark: true,
};

/// The colours on a light ground.
pub const LIGHT: NightPalette = NightPalette {
    ground: Rgb::new(0xff, 0xff, 0xff),
    ink: Rgb::new(0x0d, 0x0d, 0x0d),
    soft: Rgb::new(0x76, 0x76, 0x76),
    line: Rgb::new(0xe4, 0xe4, 0xe4),
    track: Rgb::new(0xe4, 0xe4, 0xe4),
    warning: Rgb::new(0xa3, 0x5f, 0x00),
    error: Rgb::new(0xc4, 0x26, 0x2e),
    purple: Rgb::new(0x5a, 0x3d, 0xd1),
    is_dark: false,
};

/// The badge of a status that is on track: the same green in both themes.
const ON_TRACK_FILL: Rgb = Rgb::new(0x1a, 0x7f, 0x37);
const WHITE: Rgb = Rgb::new(0xff, 0xff, 0xff);

impl NightPalette {
    /// The dark colours when `dark`, else the light ones.
    pub const fn for_dark(dark: bool) -> NightPalette {
        if dark { DARK } else { LIGHT }
    }

    /// The fill and the text of a status badge. The text is at least 4.5 to 1 on the fill in
    /// both palettes. The kinds that are not a warning or an error take the track and the ink.
    pub fn badge(&self, kind: StatusKind) -> BadgeColours {
        let on_colour = if self.is_dark { self.ground } else { WHITE };
        match kind {
            StatusKind::OnTrack => BadgeColours {
                fill: ON_TRACK_FILL,
                text: WHITE,
            },
            StatusKind::Ends => BadgeColours {
                fill: self.warning,
                text: on_colour,
            },
            StatusKind::UsedUp => BadgeColours {
                fill: self.error,
                text: on_colour,
            },
            StatusKind::NotUsedYet | StatusKind::TooEarly | StatusKind::Resetting => BadgeColours {
                fill: self.track,
                text: self.ink,
            },
        }
    }

    /// The warning colour or the error colour for those levels, and `normal` for a normal one.
    pub fn level_colour(&self, level: UsageLevel, normal: Rgb) -> Rgb {
        match level {
            UsageLevel::Warning => self.warning,
            UsageLevel::Error => self.error,
            UsageLevel::Normal => normal,
        }
    }
}
