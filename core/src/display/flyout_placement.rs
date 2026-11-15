//! Where the flyout opens on the screen, beside the taskbar. Coordinates are in pixels. The sums
//! wrap around at the ends of an `i32`, as the C# of 1.x does, in a debug build as in a release.
use std::cmp::{max, min};

/// A rectangle in pixels, as its four edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PixelRect {
    pub fn width(&self) -> i32 {
        self.right.wrapping_sub(self.left)
    }

    pub fn height(&self) -> i32 {
        self.bottom.wrapping_sub(self.top)
    }
}

/// The side of the screen the taskbar is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarEdge {
    Bottom,
    Top,
    Left,
    Right,
}

/// The side the taskbar is on: where the work area is inset from the monitor. With no inset
/// (auto-hide), the monitor edge nearest the cursor.
pub fn edge(monitor: PixelRect, work_area: PixelRect, cursor_x: i32, cursor_y: i32) -> TaskbarEdge {
    let bottom = monitor.bottom.wrapping_sub(work_area.bottom);
    let top = work_area.top.wrapping_sub(monitor.top);
    let left = work_area.left.wrapping_sub(monitor.left);
    let right = monitor.right.wrapping_sub(work_area.right);

    let widest = max(max(bottom, top), max(left, right));
    if widest > 0 {
        return if bottom == widest {
            TaskbarEdge::Bottom
        } else if top == widest {
            TaskbarEdge::Top
        } else if left == widest {
            TaskbarEdge::Left
        } else {
            TaskbarEdge::Right
        };
    }

    let d_bottom = monitor.bottom.wrapping_sub(cursor_y);
    let d_top = cursor_y.wrapping_sub(monitor.top);
    let d_left = cursor_x.wrapping_sub(monitor.left);
    let d_right = monitor.right.wrapping_sub(cursor_x);
    let nearest = min(min(d_bottom, d_top), min(d_left, d_right));
    if d_bottom == nearest {
        TaskbarEdge::Bottom
    } else if d_top == nearest {
        TaskbarEdge::Top
    } else if d_left == nearest {
        TaskbarEdge::Left
    } else {
        TaskbarEdge::Right
    }
}

/// The top left corner of a flyout `width` by `height`: against the taskbar's edge, centred on
/// the cursor along it, and kept `margin` inside the work area.
pub fn place(
    monitor: PixelRect,
    work_area: PixelRect,
    cursor_x: i32,
    cursor_y: i32,
    width: i32,
    height: i32,
    margin: i32,
) -> (i32, i32) {
    // `near` is `margin` inside an edge; `far` is a flyout of `size` and `margin` inside it.
    let near = |edge: i32| edge.wrapping_add(margin);
    let far = |edge: i32, size: i32| edge.wrapping_sub(size).wrapping_sub(margin);
    let clamp_x = |x: i32| clamp(x, near(work_area.left), far(work_area.right, width));
    let clamp_y = |y: i32| clamp(y, near(work_area.top), far(work_area.bottom, height));
    let across = cursor_x.wrapping_sub(width / 2);
    let down = cursor_y.wrapping_sub(height / 2);

    match edge(monitor, work_area, cursor_x, cursor_y) {
        TaskbarEdge::Bottom => (clamp_x(across), far(work_area.bottom, height)),
        TaskbarEdge::Top => (clamp_x(across), near(work_area.top)),
        TaskbarEdge::Left => (near(work_area.left), clamp_y(down)),
        TaskbarEdge::Right => (far(work_area.right, width), clamp_y(down)),
    }
}

/// `value` within `low` to `high`. A flyout larger than the work area leaves `high` under `low`;
/// the start then stays on the screen.
fn clamp(value: i32, low: i32, high: i32) -> i32 {
    if high < low {
        low
    } else {
        value.clamp(low, high)
    }
}
