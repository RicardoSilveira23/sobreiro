//! The tooltip's window: a toolkit window born owned by the taskbar as the pill's is, made once
//! and then only shown and hidden, so it opens at once and the toolkit has no new window to make
//! while the pointer rests. It is shown and hidden through the toolkit, which then knows to paint
//! it (a window hidden and shown behind the toolkit's back is never painted again), and raised
//! with Windows' own call, never activated and without moving its owner in the z-order
//! (`taskbar::raise_to`): putting a tooltip above the
//! other windows in the usual way carries its owner, the taskbar, along, and the pill loses its
//! owner.
//!
//! The toolkit makes the window a turn of the event loop after it is shown, and `born` lists it
//! then; the caller looks for it with `find` until it is there.
use crate::ui::Tooltip;
use crate::win::{born, taskbar};
use slint::{Color, ComponentHandle, LogicalSize, PhysicalPosition, SharedString};
use sobreiro_core::colours::Rgb;
use std::time::{Duration, Instant};

/// Where a new tooltip's window is made, off every screen.
const OFF_SCREEN: i32 = -32000;
/// The colours of 1.1's tooltip, WPF's own, as measured on the screen. They are the same in the
/// light and the dark theme.
const GROUND: Rgb = Rgb {
    r: 0xF1,
    g: 0xF2,
    b: 0xF7,
};
const EDGE: Rgb = Rgb {
    r: 0x76,
    g: 0x76,
    b: 0x76,
};
const INK: Rgb = Rgb {
    r: 0x57,
    g: 0x57,
    b: 0x57,
};
/// How long a window may be on its way before it is given up.
const GIVE_UP: Duration = Duration::from_secs(2);

pub struct TooltipWindow {
    component: Tooltip,
    /// The window `born` made, once it is listed and its owner holds.
    window: Option<isize>,
    /// The windows `born` listed before this one was shown.
    before: Vec<isize>,
    shown_at: Instant,
}

fn colour(rgb: Rgb) -> Color {
    Color::from_rgb_u8(rgb.r, rgb.g, rgb.b)
}

impl TooltipWindow {
    /// Shows a new window, born owned by `owner`, off the screen. It is hidden once `find` has it.
    /// An error says it could not be made, or that `born` is busy with another window.
    pub fn create(owner: isize) -> Result<TooltipWindow, String> {
        let component = Tooltip::new().map_err(|error| error.to_string())?;
        component.set_ground(colour(GROUND));
        component.set_ink(colour(INK));
        component.set_edge_colour(colour(EDGE));
        component
            .window()
            .set_position(PhysicalPosition::new(OFF_SCREEN, OFF_SCREEN));
        let before = born::owned_windows();
        born::show(component.window(), owner)?;
        Ok(TooltipWindow {
            component,
            window: None,
            before,
            shown_at: Instant::now(),
        })
    }

    /// Notes the window once `born` lists it, and hides it. True when it was found just now.
    pub fn find(&mut self) -> bool {
        if self.window.is_some() {
            return false;
        }
        let found = born::owned_windows()
            .into_iter()
            .rev()
            .find(|window| !self.before.contains(window));
        self.window = found;
        if found.is_some() {
            // It is seen only once the pointer has rested.
            self.hide();
        }
        found.is_some()
    }

    /// The window, once it is known and alive.
    pub fn window(&self) -> Option<isize> {
        self.window.filter(|window| born::is_window(*window))
    }

    /// Whether it is still waiting for its window.
    pub fn is_waiting(&self) -> bool {
        self.window.is_none() && self.shown_at.elapsed() < GIVE_UP
    }

    /// Whether the window has not come in time, or was destroyed (its owner went with Explorer),
    /// so it is to be dropped.
    pub fn is_lost(&self) -> bool {
        match self.window {
            Some(window) => !born::is_window(window),
            None => self.shown_at.elapsed() >= GIVE_UP,
        }
    }

    pub fn set_text(&self, text: &str) {
        self.component.set_text(SharedString::from(text));
    }

    /// The size the text wants, in units.
    pub fn size_units(&self) -> (f32, f32) {
        (
            self.component.get_tip_width(),
            self.component.get_tip_height(),
        )
    }

    /// Sets the window's size to what the text wants, so it is right when it is shown.
    pub fn fit(&self) {
        let (width, height) = self.size_units();
        self.component
            .window()
            .set_size(LogicalSize::new(width, height));
    }

    /// Hides it, through the toolkit.
    pub fn hide(&self) {
        let _ = self.component.hide();
    }

    /// Shows it with its top left corner at (x, y): moved there while hidden, shown through the
    /// toolkit so it paints, and raised above the taskbar without moving the taskbar.
    pub fn present(&self, x: i32, y: i32) {
        let Some(window) = self.window() else {
            return;
        };
        if !taskbar::is_visible(window) {
            taskbar::move_hidden(window, x, y);
        }
        let _ = self.component.show();
        taskbar::raise_to(window, x, y);
        self.component.window().request_redraw();
    }

    /// Hides it and lets its window go: the toolkit destroys the window when the component is
    /// dropped. A window still on its way is cancelled, so `born` does not take the next one.
    pub fn close(self) {
        if self.window.is_none() && born::is_pending() {
            born::cancel_pending();
        }
        // The toolkit keeps a window it was told to show alive past its component, so it is
        // hidden through the toolkit first.
        let _ = self.component.hide();
    }
}
