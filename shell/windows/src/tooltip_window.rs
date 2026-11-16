//! The tooltip's window: a toolkit window born owned by the taskbar as the pill's is, made once
//! and then only shown and hidden, so it opens at once and the toolkit has no new window to make
//! while the pointer rests. It is shown with Windows' own calls, never activated and raised
//! without moving its owner in the z-order (`taskbar::raise_to`): putting a tooltip above the
//! other windows in the usual way carries its owner, the taskbar, along, and the pill loses its
//! owner.
//!
//! The toolkit makes the window a turn of the event loop after it is shown, and `born` lists it
//! then; the caller looks for it with `find` until it is there.
use crate::ui::Tooltip;
use crate::win::system_colours;
use crate::win::{born, taskbar};
use slint::{Color, ComponentHandle, LogicalSize, PhysicalPosition, SharedString};
use sobreiro_core::colours::Rgb;
use std::time::{Duration, Instant};

/// Where a new tooltip's window is made, off every screen.
const OFF_SCREEN: i32 = -32000;
/// The edge of 1.1's tooltip, WPF's own.
const EDGE: Rgb = Rgb {
    r: 0x76,
    g: 0x76,
    b: 0x76,
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
        let (ground, ink) = system_colours::tooltip();
        component.set_ground(colour(ground));
        component.set_ink(colour(ink));
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
        if let Some(window) = found {
            // It is seen only once the pointer has rested.
            taskbar::hide(window);
        }
        self.window = found;
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
