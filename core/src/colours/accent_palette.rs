//! The accent colour of Windows and its shades.
use super::Rgb;

/// The Windows accent palette. The registry value `Explorer\Accent\AccentPalette` holds eight
/// quads of four bytes (red, green, blue and one more): Light3, Light2, Light1, Accent, Dark1,
/// Dark2, Dark3, and one that is not used. This type only reads those bytes; the shell reads the
/// registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPalette {
    pub light3: Rgb,
    pub light2: Rgb,
    pub light1: Rgb,
    pub accent: Rgb,
    pub dark1: Rgb,
    pub dark2: Rgb,
    pub dark3: Rgb,
}

/// How many quads are read: the seven that are used.
const QUADS: usize = 7;

/// Windows' own blue palette, for when the registry holds none.
pub const WINDOWS_BLUE: AccentPalette = AccentPalette {
    light3: Rgb::new(0x99, 0xEB, 0xFF),
    light2: Rgb::new(0x4C, 0xC2, 0xFF),
    light1: Rgb::new(0x00, 0x91, 0xF8),
    accent: Rgb::new(0x00, 0x78, 0xD4),
    dark1: Rgb::new(0x00, 0x67, 0xC0),
    dark2: Rgb::new(0x00, 0x3E, 0x92),
    dark3: Rgb::new(0x00, 0x1A, 0x68),
};

impl AccentPalette {
    /// The palette in the registry's bytes, or none when there are no bytes or fewer than
    /// 28 (seven quads). Bytes past the seventh quad are not looked at.
    pub fn from_registry(bytes: Option<&[u8]>) -> Option<AccentPalette> {
        let bytes = bytes?;
        if bytes.len() < QUADS * 4 {
            return None;
        }
        let at = |i: usize| Rgb::new(bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2]);
        Some(AccentPalette {
            light3: at(0),
            light2: at(1),
            light1: at(2),
            accent: at(3),
            dark1: at(4),
            dark2: at(5),
            dark3: at(6),
        })
    }
}
