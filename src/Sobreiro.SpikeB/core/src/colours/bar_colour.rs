use super::{AccentPalette, Contrast, NightPalette, Rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarColourOption {
    Purple,
    Accent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarColourSource {
    Purple,
    Accent,
    AccentShade,
    PurpleFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarColourChoice {
    pub colour: Rgb,
    pub source: BarColourSource,
}

pub struct BarColour;

impl BarColour {
    pub const MIN_CONTRAST: f64 = 3.0;

    /// Purple, or the Windows accent where it (or one of its shades) reaches 3:1 on the ground and the track.
    pub fn pick(option: BarColourOption, accent: Option<&AccentPalette>, dark: bool) -> BarColourChoice {
        let night = NightPalette::for_dark(dark);
        if option == BarColourOption::Purple {
            return BarColourChoice { colour: night.purple, source: BarColourSource::Purple };
        }

        if let Some(accent) = accent {
            let shades = if dark {
                [accent.light1, accent.light2, accent.light3]
            } else {
                [accent.dark1, accent.dark2, accent.dark3]
            };
            if reads(accent.accent, &night) {
                return BarColourChoice { colour: accent.accent, source: BarColourSource::Accent };
            }
            for shade in shades {
                if reads(shade, &night) {
                    return BarColourChoice { colour: shade, source: BarColourSource::AccentShade };
                }
            }
        }
        BarColourChoice { colour: night.purple, source: BarColourSource::PurpleFallback }
    }
}

fn reads(c: Rgb, night: &NightPalette) -> bool {
    Contrast::ratio(c, night.ground) >= BarColour::MIN_CONTRAST && Contrast::ratio(c, night.track) >= BarColour::MIN_CONTRAST
}
