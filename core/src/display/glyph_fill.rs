//! What the battery and thermometer show for a limit: what is left now, and how much of that the
//! pace will use by the reset (drawn faint).
use super::flyout_model::LimitView;

/// What the battery and thermometer show for a limit. The battery counts tenths, rounded half
/// away from zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphFill {
    pub left_cells: i32,
    pub soon_cells: i32,
    pub left_percent: f64,
    pub soon_percent: f64,
}

/// How many cells the battery has.
pub const CELLS: i32 = 10;

impl GlyphFill {
    pub fn from(view: &LimitView) -> GlyphFill {
        if !view.has_data {
            return GlyphFill {
                left_cells: 0,
                soon_cells: 0,
                left_percent: 0.0,
                soon_percent: 0.0,
            };
        }

        let used = view.used_fraction * 100.0;
        let left = CELLS - round(used / 10.0);
        let left_percent = 100.0 - used;
        let Some(fraction) = view.projected_fraction else {
            return GlyphFill {
                left_cells: left,
                soon_cells: 0,
                left_percent,
                soon_percent: 0.0,
            };
        };

        let gain = fraction * 100.0 - used;
        // Written out, as `Math.Clamp` is in the C#, so that nothing in here can panic.
        let soon_percent = if gain < 0.0 {
            0.0
        } else if gain > left_percent {
            left_percent
        } else {
            gain
        };
        GlyphFill {
            left_cells: left,
            soon_cells: left.min(round(gain / 10.0).max(0)),
            left_percent,
            soon_percent,
        }
    }
}

fn round(value: f64) -> i32 {
    value.round() as i32
}
