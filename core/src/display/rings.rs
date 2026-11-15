//! The rings on the taskbar and in the tray: how full each is and what colour it takes.
use crate::limits::usage_level::for_percent;
use crate::limits::{Limit, LimitKind, UsageLevel};

/// One taskbar or tray ring: how full it is and what colour level it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ring {
    pub percent: i32,
    pub level: UsageLevel,
    pub has_data: bool,
}

impl Ring {
    /// A ring with nothing to show.
    pub const NONE: Ring = Ring {
        percent: 0,
        level: UsageLevel::Normal,
        has_data: false,
    };

    /// Clockwise sweep from 12 o'clock; 0 without data.
    pub fn sweep_degrees(&self) -> f64 {
        if self.has_data {
            f64::from(self.percent) * 360.0 / 100.0
        } else {
            0.0
        }
    }

    /// Degrees to draw with round caps so the visible arc spans the ring's percentage: the caps
    /// add stroke / radius radians. 0 without data or at 0%, 360 (a full circle) at 100%, never
    /// under 0.5.
    pub fn capped_sweep(&self, radius: f64, stroke: f64) -> f64 {
        if !self.has_data || self.percent <= 0 {
            return 0.0;
        }
        if self.percent >= 100 {
            return 360.0;
        }
        let degrees = self.sweep_degrees() - stroke / radius * 180.0 / std::f64::consts::PI;
        // Not-a-number stays so, as in the C#; `f64::max` would drop it for the 0.5.
        if degrees.is_nan() {
            degrees
        } else {
            degrees.max(0.5)
        }
    }

    /// The ring of a limit; none for no limit.
    pub fn from(limit: Option<&Limit>) -> Ring {
        match limit {
            None => Ring::NONE,
            Some(limit) => Ring {
                percent: limit.display_percent(),
                level: for_percent(limit.display_percent()),
                has_data: true,
            },
        }
    }
}

/// Outer ring is the session (5h) window, inner ring the weekly (7d). Spend is not drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingsModel {
    pub outer: Ring,
    pub inner: Ring,
    pub has_inner: bool,
}

impl RingsModel {
    /// Copilot's credits fill one ring.
    pub fn copilot(limits: &[Limit]) -> RingsModel {
        RingsModel {
            outer: Ring::from(first_of(limits, LimitKind::Credits)),
            inner: Ring::NONE,
            has_inner: false,
        }
    }

    /// Claude Code's session and weekly windows.
    pub fn from(limits: &[Limit]) -> RingsModel {
        RingsModel {
            outer: Ring::from(first_of(limits, LimitKind::Session)),
            inner: Ring::from(first_of(limits, LimitKind::Weekly)),
            has_inner: true,
        }
    }
}

fn first_of(limits: &[Limit], kind: LimitKind) -> Option<&Limit> {
    limits.iter().find(|limit| limit.kind == kind)
}
