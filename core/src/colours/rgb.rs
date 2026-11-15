//! A colour of three bytes, and its `#RRGGBB` text.
use std::fmt;

/// A colour: red, green and blue, a byte each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// The text given to [`Rgb::parse`] is not `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbError {
    text: String,
}

impl fmt::Display for RgbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected #RRGGBB, got '{}'.", self.text)
    }
}

impl std::error::Error for RgbError {}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    /// Reads `#RRGGBB`. The hash is optional, and so is a second one: any number of `#` at the
    /// start are dropped, as 1.x did. What is left must be six characters, two for each byte.
    /// As in 1.x, a byte's two characters may have a space (or a tab or line break) in front
    /// or behind, or null characters behind: `"A 0 0 "` is the colour `#0A0000`.
    pub fn parse(hex: &str) -> Result<Rgb, RgbError> {
        // 1.x counts and cuts in UTF-16 units, so a character outside the first plane is two.
        let units: Vec<u16> = hex.encode_utf16().collect();
        let start = units.iter().take_while(|&&unit| unit == HASH).count();
        let digits = &units[start..];
        if let [a, b, c, d, e, f] = *digits
            && let (Some(r), Some(g), Some(blue)) = (byte([a, b]), byte([c, d]), byte([e, f]))
        {
            return Ok(Rgb { r, g, b: blue });
        }
        Err(RgbError {
            text: hex.to_string(),
        })
    }

    /// `#RRGGBB`, in capitals.
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

const HASH: u16 = b'#' as u16;

/// One byte from two characters, the way `byte.Parse` read hexadecimal in 1.x: optional white
/// space, at least one hex digit, optional white space, and nothing but nulls after that.
fn byte(pair: [u16; 2]) -> Option<u8> {
    let mut i = 0;
    while i < 2 && is_white(pair[i]) {
        i += 1;
    }
    let digits_from = i;
    let mut value: u32 = 0;
    while i < 2 {
        let Some(digit) = char::from_u32(u32::from(pair[i])).and_then(|c| c.to_digit(16)) else {
            break;
        };
        value = value * 16 + digit;
        i += 1;
    }
    if i == digits_from {
        return None;
    }
    while i < 2 && is_white(pair[i]) {
        i += 1;
    }
    if pair[i..].iter().any(|&unit| unit != 0) {
        return None;
    }
    u8::try_from(value).ok()
}

/// The white space a number may have around it: tab to carriage return, and the space.
fn is_white(unit: u16) -> bool {
    unit == 0x20 || (0x09..=0x0D).contains(&unit)
}
