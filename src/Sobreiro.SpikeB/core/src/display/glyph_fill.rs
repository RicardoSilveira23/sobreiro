use super::LimitView;

/// What the battery and thermometer show for a limit: what is left now, and how much of that the pace will use by
/// the reset (drawn faint). The battery counts tenths, rounded half away from zero.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphFill {
    pub left_cells: i32,
    pub soon_cells: i32,
    pub left_percent: f64,
    pub soon_percent: f64,
}

impl GlyphFill {
    pub const CELLS: i32 = 10;

    pub fn from(view: &LimitView) -> GlyphFill {
        if !view.has_data {
            return GlyphFill { left_cells: 0, soon_cells: 0, left_percent: 0.0, soon_percent: 0.0 };
        }

        let used = view.used_fraction * 100.0;
        let left = Self::CELLS - round(used / 10.0);
        let left_percent = 100.0 - used;
        let Some(fraction) = view.projected_fraction else {
            return GlyphFill { left_cells: left, soon_cells: 0, left_percent, soon_percent: 0.0 };
        };

        let gain = fraction * 100.0 - used;
        GlyphFill {
            left_cells: left,
            soon_cells: left.min(0.max(round(gain / 10.0))),
            left_percent,
            soon_percent: gain.clamp(0.0, left_percent),
        }
    }
}

fn round(v: f64) -> i32 {
    v.round() as i32
}
