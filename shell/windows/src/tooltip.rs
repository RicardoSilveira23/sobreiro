//! The pill's tooltip, as sums: when it opens and closes, and where it goes. Nothing here touches a
//! window or a clock, so a test drives it with made-up times (`Duration`s from any fixed start).
//!
//! As 1.1's does (WPF's tooltip, with the pill's own placement): it opens once the pointer has
//! rested on the pill for half a second, and closes when the pointer leaves, on a click and after
//! five seconds. Once closed with the pointer still on the pill it stays closed until the pointer
//! has left and come back.
use sobreiro_core::display::flyout_placement::PixelRect;
use std::time::Duration;

/// How long the pointer rests on the pill before the tooltip opens.
pub const REST: Duration = Duration::from_millis(500);
/// How long an open tooltip stays.
pub const OPEN_FOR: Duration = Duration::from_secs(5);
/// The gap between the tooltip's bottom and the top of the taskbar band, in units.
const GAP_UNITS: f64 = 8.0;

/// What the window has to do after an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Nothing,
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The pointer is not on the pill.
    Away,
    /// The pointer came onto the pill at this time.
    Resting { since: Duration },
    /// The tooltip opened at this time.
    Open { since: Duration },
    /// The tooltip has closed; the pointer is still on the pill.
    Spent,
}

/// When the tooltip opens and closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hover {
    phase: Phase,
}

impl Default for Hover {
    fn default() -> Hover {
        Hover { phase: Phase::Away }
    }
}

impl Hover {
    /// Whether the tooltip is open now.
    pub fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Open { .. })
    }

    /// The pointer came onto the pill. A second report with the pointer already there changes
    /// nothing.
    pub fn entered(&mut self, now: Duration) -> Step {
        if matches!(self.phase, Phase::Away) {
            self.phase = Phase::Resting { since: now };
        }
        Step::Nothing
    }

    /// The pointer left the pill.
    pub fn left(&mut self) -> Step {
        let was_open = self.is_open();
        self.phase = Phase::Away;
        if was_open { Step::Close } else { Step::Nothing }
    }

    /// A button went down on the pill. It ends a wait or an open tooltip.
    pub fn clicked(&mut self) -> Step {
        match self.phase {
            Phase::Resting { .. } => {
                self.phase = Phase::Spent;
                Step::Nothing
            }
            Phase::Open { .. } => {
                self.phase = Phase::Spent;
                Step::Close
            }
            Phase::Away | Phase::Spent => Step::Nothing,
        }
    }

    /// The time has moved on to `now`.
    pub fn tick(&mut self, now: Duration) -> Step {
        match self.phase {
            Phase::Resting { since } if now.saturating_sub(since) >= REST => {
                self.phase = Phase::Open { since: now };
                Step::Open
            }
            Phase::Open { since } if now.saturating_sub(since) >= OPEN_FOR => {
                self.phase = Phase::Spent;
                Step::Close
            }
            _ => Step::Nothing,
        }
    }

    /// Ends whatever is under way, as when the pill goes away. True when the tooltip was open.
    pub fn reset(&mut self) -> bool {
        let was_open = self.is_open();
        self.phase = Phase::Away;
        was_open
    }

    /// How long until `tick` has something to do, if it will.
    pub fn next_due(&self, now: Duration) -> Option<Duration> {
        let (since, wait) = match self.phase {
            Phase::Resting { since } => (since, REST),
            Phase::Open { since } => (since, OPEN_FOR),
            Phase::Away | Phase::Spent => return None,
        };
        Some((since + wait).saturating_sub(now))
    }
}

/// The gap in pixels at `dpi`: 8 units, rounded as the pill's gap is (halves to even).
pub fn gap_at(dpi: u32) -> i32 {
    (GAP_UNITS * f64::from(dpi) / 96.0).round_ties_even() as i32
}

/// The tooltip's top left corner, in pixels: its bottom `gap` above the top of the taskbar band
/// and its left edge on the pill's, kept inside the monitor. A tooltip wider or taller than the
/// monitor keeps its left and top edges on it.
pub fn place(
    monitor: PixelRect,
    pill_left: i32,
    band_top: i32,
    size: (i32, i32),
    gap: i32,
) -> (i32, i32) {
    let (width, height) = size;
    let x = pill_left.min(monitor.right.saturating_sub(width));
    let y = band_top.saturating_sub(gap).saturating_sub(height);
    (x.max(monitor.left), y.max(monitor.top))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    #[test]
    fn it_opens_when_the_pointer_has_rested_half_a_second() {
        let mut hover = Hover::default();
        assert_eq!(hover.entered(at(1000)), Step::Nothing);
        assert_eq!(hover.next_due(at(1000)), Some(at(500)));
        assert_eq!(hover.tick(at(1499)), Step::Nothing);
        assert!(!hover.is_open());
        assert_eq!(hover.tick(at(1500)), Step::Open);
        assert!(hover.is_open());
        assert_eq!(hover.tick(at(1600)), Step::Nothing);
    }

    #[test]
    fn it_closes_after_five_seconds_and_stays_closed_until_the_pointer_has_been_away() {
        let mut hover = Hover::default();
        hover.entered(at(0));
        assert_eq!(hover.tick(at(500)), Step::Open);
        assert_eq!(hover.next_due(at(2500)), Some(at(3000)));
        assert_eq!(hover.tick(at(5499)), Step::Nothing);
        assert_eq!(hover.tick(at(5500)), Step::Close);
        assert_eq!(hover.next_due(at(5500)), None);
        // Still on the pill: nothing opens again.
        assert_eq!(hover.entered(at(6000)), Step::Nothing);
        assert_eq!(hover.tick(at(9000)), Step::Nothing);
        // Away and back: it waits again.
        assert_eq!(hover.left(), Step::Nothing);
        hover.entered(at(10_000));
        assert_eq!(hover.tick(at(10_500)), Step::Open);
    }

    #[test]
    fn leaving_closes_an_open_tooltip_and_cancels_a_wait() {
        let mut hover = Hover::default();
        hover.entered(at(0));
        assert_eq!(hover.left(), Step::Nothing);
        assert_eq!(hover.tick(at(900)), Step::Nothing);
        hover.entered(at(1000));
        assert_eq!(hover.tick(at(1500)), Step::Open);
        assert_eq!(hover.left(), Step::Close);
        assert!(!hover.is_open());
        assert_eq!(hover.next_due(at(1600)), None);
    }

    #[test]
    fn a_click_closes_an_open_tooltip_and_ends_the_wait() {
        let mut hover = Hover::default();
        hover.entered(at(0));
        assert_eq!(hover.tick(at(500)), Step::Open);
        assert_eq!(hover.clicked(), Step::Close);
        assert_eq!(hover.tick(at(5000)), Step::Nothing);

        let mut hover = Hover::default();
        hover.entered(at(0));
        assert_eq!(hover.clicked(), Step::Nothing);
        assert_eq!(hover.tick(at(900)), Step::Nothing);
        // A click with the pointer away does nothing.
        let mut hover = Hover::default();
        assert_eq!(hover.clicked(), Step::Nothing);
        assert_eq!(hover.tick(at(900)), Step::Nothing);
    }

    #[test]
    fn a_second_enter_does_not_restart_the_wait() {
        let mut hover = Hover::default();
        hover.entered(at(0));
        hover.entered(at(400));
        assert_eq!(hover.tick(at(500)), Step::Open);
    }

    #[test]
    fn a_reset_says_whether_it_was_open() {
        let mut hover = Hover::default();
        hover.entered(at(0));
        assert!(!hover.reset());
        hover.entered(at(0));
        hover.tick(at(500));
        assert!(hover.reset());
        assert_eq!(hover.tick(at(9000)), Step::Nothing);
    }

    #[test]
    fn the_gap_is_eight_units() {
        assert_eq!(gap_at(96), 8);
        assert_eq!(gap_at(120), 10);
        assert_eq!(gap_at(144), 12);
        assert_eq!(gap_at(192), 16);
        assert_eq!(gap_at(108), 9);
    }

    fn monitor() -> PixelRect {
        PixelRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        }
    }

    #[test]
    fn it_sits_above_the_band_with_its_left_edge_on_the_pills() {
        // A taskbar band from 1040: the bottom is 8 above it, so the top is 8 + 60 above.
        assert_eq!(place(monitor(), 1500, 1040, (200, 60), 8), (1500, 972));
    }

    #[test]
    fn it_is_kept_inside_the_monitor() {
        // Too far right: its right edge on the monitor's.
        assert_eq!(place(monitor(), 1900, 1040, (200, 60), 8), (1720, 972));
        // Too far left, as on a monitor left of the origin.
        let left = PixelRect {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1080,
        };
        assert_eq!(place(left, -2000, 1040, (200, 60), 8), (-1920, 972));
        // Too high: its top on the monitor's.
        assert_eq!(place(monitor(), 100, 40, (200, 60), 8), (100, 0));
        // Wider than the monitor: the left edge wins.
        assert_eq!(place(monitor(), 100, 1040, (2000, 60), 8), (0, 972));
    }
}
