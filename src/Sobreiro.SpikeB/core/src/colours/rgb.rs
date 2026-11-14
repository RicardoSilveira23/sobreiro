#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Rgb { r, g, b }
    }

    /// Parses "#RRGGBB" (the hash is optional). The error is the C# FormatException's text.
    pub fn parse(hex: &str) -> Result<Rgb, String> {
        let s = hex.trim_start_matches('#');
        let bad = || format!("Expected #RRGGBB, got '{hex}'.");
        if s.len() != 6 || !s.is_ascii() {
            return Err(bad());
        }
        let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| bad());
        Ok(Rgb { r: p(0)?, g: p(2)?, b: p(4)? })
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// For the constants in this crate, which are known to be well formed.
    pub(crate) fn hex(s: &str) -> Rgb {
        Rgb::parse(s).expect("constant colour")
    }
}
