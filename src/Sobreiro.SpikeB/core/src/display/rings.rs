use crate::limits::{Limit, LimitKind, UsageLevel, UsageLevels};

/// One taskbar/tray ring: how full it is and what colour level it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    pub percent: i32,
    pub level: UsageLevel,
    pub has_data: bool,
}

impl Ring {
    pub const NONE: Ring = Ring { percent: 0, level: UsageLevel::Normal, has_data: false };

    /// Clockwise sweep from 12 o'clock; 0 without data.
    pub fn sweep_degrees(&self) -> f64 {
        if self.has_data { self.percent as f64 * 360.0 / 100.0 } else { 0.0 }
    }

    /// Degrees to draw with round caps so the visible arc spans the ring's percentage: the caps add
    /// stroke / radius radians. 0 without data or at 0%, 360 (a full circle) at 100%, never under 0.5.
    pub fn capped_sweep(&self, radius: f64, stroke: f64) -> f64 {
        if !self.has_data || self.percent <= 0 {
            return 0.0;
        }
        if self.percent >= 100 {
            return 360.0;
        }
        (self.sweep_degrees() - stroke / radius * 180.0 / std::f64::consts::PI).max(0.5)
    }

    pub fn from(limit: Option<&Limit>) -> Ring {
        match limit {
            None => Ring::NONE,
            Some(l) => Ring { percent: l.display_percent(), level: UsageLevels::for_percent(l.display_percent()), has_data: true },
        }
    }
}

/// Outer ring is the session (5h) window, inner ring the weekly (7d). Spend is not drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingsModel {
    pub outer: Ring,
    pub inner: Ring,
    pub has_inner: bool,
}

impl RingsModel {
    /// Copilot's credits fill one ring.
    pub fn copilot(limits: &[Limit]) -> RingsModel {
        RingsModel { outer: Ring::from(limits.iter().find(|l| l.kind == LimitKind::Credits)), inner: Ring::NONE, has_inner: false }
    }

    pub fn from(limits: &[Limit]) -> RingsModel {
        RingsModel {
            outer: Ring::from(limits.iter().find(|l| l.kind == LimitKind::Session)),
            inner: Ring::from(limits.iter().find(|l| l.kind == LimitKind::Weekly)),
            has_inner: true,
        }
    }
}
