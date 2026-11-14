/// What placing the pill does to its window: nothing, a move, or a raise to the top of the topmost band.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillShowStep {
    None,
    Move,
    Raise,
}

pub struct PillShow;

impl PillShow {
    /// A pill that is hidden, or shown below the taskbar, is raised: while it is hidden Explorer can raise the
    /// taskbar past it (it does when a full-screen window closes), and a window shown again keeps its old place in
    /// the z-order, under the taskbar. Otherwise it only moves, and not at all when it is in place.
    pub fn step(visible: bool, above_taskbar: bool, in_place: bool) -> PillShowStep {
        if !visible || !above_taskbar {
            PillShowStep::Raise
        } else if in_place {
            PillShowStep::None
        } else {
            PillShowStep::Move
        }
    }
}

pub struct ZOrder;

impl ZOrder {
    const MAX_STEPS: usize = 4096;

    /// True when `window` is above `other`: walks up from `other` with `above` (the next window up, 0 at the top)
    /// looking for it. The walk is bounded, as the z-order can change while it runs. Handles are `isize` (nint).
    pub fn is_above(window: isize, other: isize, above: impl Fn(isize) -> isize) -> bool {
        let mut current = other;
        for _ in 0..Self::MAX_STEPS {
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
