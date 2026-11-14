use super::Rgb;

pub struct Contrast;

impl Contrast {
    /// WCAG 2.x contrast ratio, 1 to 21.
    pub fn ratio(a: Rgb, b: Rgb) -> f64 {
        let la = luminance(a);
        let lb = luminance(b);
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }
}

fn luminance(c: Rgb) -> f64 {
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

fn linear(channel: u8) -> f64 {
    let s = channel as f64 / 255.0;
    if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
}
