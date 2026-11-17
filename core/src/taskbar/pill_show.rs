//! What placing the pill does to its window, and how to tell whether it is above the taskbar.

/// What placing the pill does to its window: nothing, a move, a raise to the top of the topmost
/// band, or showing a hidden pill (through the toolkit, then raised).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillShowStep {
    None,
    Move,
    Raise,
    Show,
}

/// A hidden pill is shown through the toolkit, so that it is drawn again, and raised: while it
/// is hidden Explorer can raise the taskbar past it (it does when a full-screen window closes).
/// A visible pill below the taskbar is raised, as a window keeps its old place in the z-order.
/// Otherwise it only moves, and not at all when it is in place.
pub fn step(visible: bool, above_taskbar: bool, in_place: bool) -> PillShowStep {
    if !visible {
        PillShowStep::Show
    } else if !above_taskbar {
        PillShowStep::Raise
    } else if in_place {
        PillShowStep::None
    } else {
        PillShowStep::Move
    }
}

/// The most windows a walk through the z-order looks at.
const MAX_STEPS: i32 = 4096;

/// The z-order of windows.
pub struct ZOrder;

impl ZOrder {
    /// True when `window` is above `other`: walks up from `other` with `above` (the next window
    /// up, 0 at the top) looking for it. The walk is bounded, as the z-order can change while
    /// it runs.
    pub fn is_above(window: isize, other: isize, mut above: impl FnMut(isize) -> isize) -> bool {
        let mut current = other;
        for _ in 0..MAX_STEPS {
            current = above(current);
            if current == 0 {
                return false;
            }
            if current == window {
                return true;
            }
        }
        false
    }
}
