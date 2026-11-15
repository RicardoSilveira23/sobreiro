//! The colour of the bars.
use super::Rgb;
use super::accent_palette::AccentPalette;
use super::contrast;
use super::night_palette::NightPalette;

/// Which colour the bars take: Sobreiro's purple, or the accent colour the user chose in
/// Windows. `config.json` keeps the choice under `bar_colour`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarColourOption {
    Purple,
    Accent,
}

/// Where the colour that was picked came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarColourSource {
    /// The user chose purple.
    Purple,
    /// The accent colour itself.
    Accent,
    /// One of the accent's shades, because the accent itself is too faint on the ground.
    AccentShade,
    /// The user chose the accent, but neither it nor a shade shows, or there is no palette.
    PurpleFallback,
}

/// A colour for the bars and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarColourChoice {
    pub colour: Rgb,
    pub source: BarColourSource,
}

/// The least contrast, to one, that a bar needs against the ground and against its track.
pub const MIN_CONTRAST: f64 = 3.0;

/// Purple, or the Windows accent where it, or one of its shades, reaches 3 to 1 on the ground
/// and on the track. The shades tried on a dark ground are the light ones, Light1 first, and on
/// a light ground the dark ones, Dark1 first.
pub fn pick(
    option: BarColourOption,
    accent: Option<&AccentPalette>,
    dark: bool,
) -> BarColourChoice {
    let night = NightPalette::for_dark(dark);
    if option == BarColourOption::Purple {
        return BarColourChoice {
            colour: night.purple,
            source: BarColourSource::Purple,
        };
    }

    if let Some(accent) = accent {
        let shades = if dark {
            [accent.light1, accent.light2, accent.light3]
        } else {
            [accent.dark1, accent.dark2, accent.dark3]
        };

        if reads(accent.accent, &night) {
            return BarColourChoice {
                colour: accent.accent,
                source: BarColourSource::Accent,
            };
        }
        if let Some(shade) = shades.into_iter().find(|&shade| reads(shade, &night)) {
            return BarColourChoice {
                colour: shade,
                source: BarColourSource::AccentShade,
            };
        }
    }
    BarColourChoice {
        colour: night.purple,
        source: BarColourSource::PurpleFallback,
    }
}

fn reads(colour: Rgb, night: &NightPalette) -> bool {
    contrast::ratio(colour, night.ground) >= MIN_CONTRAST
        && contrast::ratio(colour, night.track) >= MIN_CONTRAST
}
