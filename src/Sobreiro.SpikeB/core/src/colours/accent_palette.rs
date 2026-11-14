use super::Rgb;

/// The Windows accent palette. The registry value Explorer\Accent\AccentPalette holds eight RGBA quads:
/// Light3, Light2, Light1, Accent, Dark1, Dark2, Dark3, unused. This type only parses bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct AccentPalette {
    pub light3: Rgb,
    pub light2: Rgb,
    pub light1: Rgb,
    pub accent: Rgb,
    pub dark1: Rgb,
    pub dark2: Rgb,
    pub dark3: Rgb,
}

const QUADS: usize = 7;

impl AccentPalette {
    /// Windows' default blue palette, for when the registry holds none.
    pub fn windows_blue() -> AccentPalette {
        AccentPalette {
            light3: Rgb::hex("#99EBFF"),
            light2: Rgb::hex("#4CC2FF"),
            light1: Rgb::hex("#0091F8"),
            accent: Rgb::hex("#0078D4"),
            dark1: Rgb::hex("#0067C0"),
            dark2: Rgb::hex("#003E92"),
            dark3: Rgb::hex("#001A68"),
        }
    }

    pub fn from_registry(bytes: Option<&[u8]>) -> Option<AccentPalette> {
        let bytes = bytes?;
        if bytes.len() < QUADS * 4 {
            return None;
        }
        let at = |i: usize| Rgb::new(bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2]);
        Some(AccentPalette { light3: at(0), light2: at(1), light1: at(2), accent: at(3), dark1: at(4), dark2: at(5), dark3: at(6) })
    }
}
