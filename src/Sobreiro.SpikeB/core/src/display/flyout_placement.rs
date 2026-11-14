#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PixelRect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> PixelRect {
        PixelRect { left, top, right, bottom }
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskbarEdge {
    Bottom,
    Top,
    Left,
    Right,
}

pub struct FlyoutPlacement;

impl FlyoutPlacement {
    /// The side the taskbar is on: where the work area is inset from the monitor. With no inset (auto-hide), the
    /// monitor edge nearest the cursor.
    pub fn edge(monitor: PixelRect, work_area: PixelRect, cursor_x: i32, cursor_y: i32) -> TaskbarEdge {
        let bottom = monitor.bottom - work_area.bottom;
        let top = work_area.top - monitor.top;
        let left = work_area.left - monitor.left;
        let right = monitor.right - work_area.right;

        let max = bottom.max(top).max(left.max(right));
        if max > 0 {
            return if bottom == max {
                TaskbarEdge::Bottom
            } else if top == max {
                TaskbarEdge::Top
            } else if left == max {
                TaskbarEdge::Left
            } else {
                TaskbarEdge::Right
            };
        }

        let d_bottom = monitor.bottom - cursor_y;
        let d_top = cursor_y - monitor.top;
        let d_left = cursor_x - monitor.left;
        let d_right = monitor.right - cursor_x;
        let min = d_bottom.min(d_top).min(d_left.min(d_right));
        if d_bottom == min {
            TaskbarEdge::Bottom
        } else if d_top == min {
            TaskbarEdge::Top
        } else if d_left == min {
            TaskbarEdge::Left
        } else {
            TaskbarEdge::Right
        }
    }

    pub fn place(
        monitor: PixelRect, work_area: PixelRect, cursor_x: i32, cursor_y: i32, width: i32, height: i32, margin: i32,
    ) -> (i32, i32) {
        let clamp_x = |x: i32| clamp(x, work_area.left + margin, work_area.right - width - margin);
        let clamp_y = |y: i32| clamp(y, work_area.top + margin, work_area.bottom - height - margin);

        match Self::edge(monitor, work_area, cursor_x, cursor_y) {
            TaskbarEdge::Bottom => (clamp_x(cursor_x - width / 2), work_area.bottom - height - margin),
            TaskbarEdge::Top => (clamp_x(cursor_x - width / 2), work_area.top + margin),
            TaskbarEdge::Left => (work_area.left + margin, clamp_y(cursor_y - height / 2)),
            TaskbarEdge::Right => (work_area.right - width - margin, clamp_y(cursor_y - height / 2)),
        }
    }
}

// Math.Clamp throws when max < min (a flyout larger than the work area); keep the start on screen instead.
fn clamp(v: i32, min: i32, max: i32) -> i32 {
    if max < min { min } else { v.clamp(min, max) }
}
