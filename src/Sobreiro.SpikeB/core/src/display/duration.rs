use crate::Span;

pub struct Duration;

impl Duration {
    /// "3d 13h", "3h 11m", "45m", "under 1m"; the smaller unit is dropped when zero.
    pub fn format(span: Span) -> String {
        let minutes = span.num_minutes();
        if span <= Span::zero() || minutes < 1 {
            return "under 1m".to_string();
        }
        let d = minutes / 1440;
        let h = minutes % 1440 / 60;
        let m = minutes % 60;
        if d > 0 {
            return if h == 0 { format!("{d}d") } else { format!("{d}d {h}h") };
        }
        if h > 0 {
            return if m == 0 { format!("{h}h") } else { format!("{h}h {m}m") };
        }
        format!("{m}m")
    }
}
