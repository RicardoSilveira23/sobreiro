//! The flyout's window: the card in a borderless, transparent window of its own, opened beside the
//! pill (or a tray icon) and closed on Esc, on loss of focus and on a second click on the pill.
//!
//! The window is made once, on the first open, and then only shown and hidden through the toolkit
//! (a window hidden and shown behind the toolkit's back is never painted again; see
//! `tooltip_window`). It is born off screen, as a tool window with no owner that may be activated
//! (`born::show_focusable`): no taskbar button and not in Alt+Tab, and, unlike the pill's windows,
//! it takes the focus, which is how it learns to close. It is not owned by the taskbar: activating
//! a window the taskbar owns carries the taskbar up and the pill loses its owner.
//!
//! It is placed with Core's `flyout_placement::place` at the size the card has in pixels at the
//! DPI of the monitor it opens on. A repeating timer, while it is open, compares the window's DPI
//! and pixel size with what it saw after the last placement, and places it again when they
//! changed (the scale changed, or the window landed on a monitor at another scale).
//!
//! The state lives in a value of the UI thread. A call that comes while it is busy runs a turn of
//! the event loop later.
use super::face::flyout_data;
use super::states::states;
use super::toggle::{Click, Toggle};
use crate::ui::{FlyoutData, FlyoutWindow};
use crate::win::{born, monitor, registry, taskbar};
use slint::{ComponentHandle, PhysicalPosition, PhysicalSize, Timer, TimerMode};
use sobreiro_core::colours::bar_colour::pick;
use sobreiro_core::colours::{BarColourOption, NightPalette};
use sobreiro_core::display::FlyoutStyle;
use sobreiro_core::display::flyout_placement::place;
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// Where the window is made, off every screen.
const OFF_SCREEN: i32 = -32000;
/// How often a new window is looked for, and how often the flyout waits for `born`.
const FIND_POLL: Duration = Duration::from_millis(15);
/// How many times an open waits for `born` to finish another window, at one try every `FIND_POLL`.
const BORN_TRIES: u32 = 100;
/// How many times a show waits for the window to take the target monitor's scale, at one try
/// every `FIND_POLL`, before it is placed anyway.
const SCALE_TRIES: u32 = 20;
/// How many times a refused activation is tried again, at one try every `FIND_POLL`.
const FOCUS_RETRIES: u32 = 3;
/// How long a window may be on its way before it is given up.
const GIVE_UP: Duration = Duration::from_secs(2);
/// How often an open flyout looks for a change of DPI or size.
const WATCH: Duration = Duration::from_millis(100);
/// The gap kept to the work area, in units.
const MARGIN_UNITS: f64 = 12.0;

thread_local! {
    static FLYOUT: RefCell<Option<Flyout>> = const { RefCell::new(None) };
}

/// What was seen of the window after it was placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seen {
    dpi: u32,
    width: u32,
    height: u32,
    /// The size the card wants in pixels at the window's DPI; it moves when the card's own height
    /// does.
    wanted: (i32, i32),
}

#[derive(Default)]
struct Flyout {
    toggle: Toggle,
    component: Option<FlyoutWindow>,
    /// The window's handle, once the toolkit has made it.
    window: Option<isize>,
    made_at: Option<Instant>,
    /// Where it opens, in physical pixels.
    point: (i32, i32),
    seen: Option<Seen>,
    watch: Timer,
    /// Hidden to take a new scale and not focused again yet: a loss of focus then is its own doing.
    rescaling: bool,
    /// The monitor's DPI it was last hidden and shown again for, so it is done once per change.
    rescaled_for: Option<u32>,
}

/// Runs `step` on the flyout now, or a turn of the event loop later if it is busy.
fn with_flyout(step: impl FnOnce(&mut Flyout) + 'static) {
    let busy = FLYOUT.try_with(|cell| match cell.try_borrow_mut() {
        Ok(mut flyout) => {
            step(flyout.get_or_insert_with(Flyout::default));
            None
        }
        Err(_) => Some(step),
    });
    if let Ok(Some(step)) = busy {
        Timer::single_shot(Duration::ZERO, move || with_flyout(step));
    }
}

/// What the flyout answers now, or none when it is busy or not made.
#[allow(dead_code)] // Used by the calls the tray icons (#49) make.
fn flyout_value<T>(step: impl FnOnce(&Flyout) -> T) -> Option<T> {
    FLYOUT
        .try_with(|cell| {
            let flyout = cell.try_borrow().ok()?;
            Some(step(flyout.as_ref()?))
        })
        .ok()
        .flatten()
}

/// The card's size in pixels at `dpi`, from its size in units: rounded up, so no edge is cut.
fn pixels_at(units: f32, dpi: u32) -> i32 {
    // Less a hair: a whole number of pixels held as f32 units can come out just over.
    (f64::from(units) * f64::from(dpi) / 96.0 - 0.001).ceil() as i32
}

/// The margin to the work area in pixels at `dpi`, rounded as 1.1 rounds it (halves to even).
fn margin_at(dpi: u32) -> i32 {
    (MARGIN_UNITS * f64::from(dpi) / 96.0).round_ties_even() as i32
}

/// The model the card shows until #38 makes it live: a fixed state with Claude and Copilot, drawn
/// as bars in Windows' theme (dark unless the system theme is light) and the purple bar colour.
fn sample() -> FlyoutData {
    let all = states();
    let model = all
        .iter()
        .find(|(name, _)| *name == "both")
        .or_else(|| all.first())
        .map(|(_, model)| model);
    let dark = !registry::theme().light;
    let fill = pick(BarColourOption::Purple, None, dark).colour;
    match model {
        Some(model) => flyout_data(
            model,
            FlyoutStyle::Bars,
            &NightPalette::for_dark(dark),
            fill,
        ),
        None => FlyoutData::default(),
    }
}

impl Flyout {
    /// The flyout is to be open at the point; makes the window if it is not made.
    fn open(&mut self, x: i32, y: i32) {
        self.point = (x, y);
        self.rescaled_for = None;
        self.toggle.opened();
        self.start(BORN_TRIES);
    }

    /// Makes the window when it does not exist, waiting for `born` if it is busy, then shows it.
    fn start(&mut self, tries: u32) {
        if !self.toggle.is_open() {
            return;
        }
        if self.component.is_none() {
            if born::is_pending() {
                if tries == 0 {
                    self.toggle.closed();
                    return;
                }
                Timer::single_shot(FIND_POLL, move || {
                    with_flyout(move |flyout| flyout.start(tries - 1));
                });
                return;
            }
            if self.create().is_err() {
                self.toggle.closed();
                return;
            }
            self.made_at = Some(Instant::now());
        }
        self.present(SCALE_TRIES);
    }

    fn create(&mut self) -> Result<(), String> {
        let component = FlyoutWindow::new().map_err(|error| error.to_string())?;
        component.set_data(sample());
        component.on_escape(close);
        component
            .window()
            .set_position(PhysicalPosition::new(OFF_SCREEN, OFF_SCREEN));
        born::on_deactivated(deactivated);
        born::show_focusable(component.window())?;
        self.component = Some(component);
        Ok(())
    }

    /// Shows the window once the toolkit has made it; looks again shortly until then. A window
    /// that is not shown is first moved, hidden, to its place until it has the target monitor's
    /// scale (a window shown at another scale is drawn once at the wrong size), as the tooltip is.
    fn present(&mut self, scale_tries: u32) {
        if !self.toggle.is_open() {
            return;
        }
        if self.window.is_none() {
            self.window = born::focusable_window();
            if self.window.is_some() {
                // Hidden through the toolkit, so the first open shows like every later one.
                self.hide();
            }
        }
        let Some(window) = self.window else {
            let late = self.made_at.is_none_or(|at| at.elapsed() >= GIVE_UP);
            if late {
                self.abandon();
            } else {
                Timer::single_shot(FIND_POLL, || {
                    with_flyout(|flyout| flyout.present(SCALE_TRIES))
                });
            }
            return;
        };
        if !born::is_window(window) {
            self.abandon();
            return;
        }
        let Some(target) = self.target() else {
            self.close();
            return;
        };
        let Some(component) = &self.component else {
            return;
        };
        component.set_data(sample());
        if !component.window().is_visible() {
            if scale_tries > 0 && taskbar::dpi_of(window) != target.dpi {
                taskbar::move_hidden(window, target.left, target.top);
                Timer::single_shot(FIND_POLL, move || {
                    with_flyout(move |flyout| flyout.present(scale_tries - 1));
                });
                return;
            }
            self.place(window);
            let shown = self.component.as_ref().is_some_and(|c| c.show().is_ok());
            if !shown {
                self.close();
                return;
            }
        } else {
            self.place(window);
        }
        if let Some(component) = &self.component {
            component.window().request_redraw();
        }
        self.watch
            .start(TimerMode::Repeated, WATCH, || with_flyout(Flyout::check));
        self.focus(FOCUS_RETRIES);
    }

    /// Gives the window the focus. Windows may refuse; it is tried again a few times, and then the
    /// flyout closes, so it is never left open, unfocused and deaf to Esc and to focus loss.
    fn focus(&mut self, retries: u32) {
        let Some(window) = self.window.filter(|_| self.toggle.is_open()) else {
            return;
        };
        if let Some(component) = &self.component {
            component.invoke_take_focus();
        }
        if born::activate(window) {
            self.rescaling = false;
            return;
        }
        if retries == 0 {
            self.close();
            return;
        }
        Timer::single_shot(FIND_POLL, move || {
            with_flyout(move |flyout| flyout.focus(retries - 1));
        });
    }

    /// Where the window goes and at what size, for the monitor under the point.
    fn target(&self) -> Option<Target> {
        let component = self.component.as_ref()?;
        let (x, y) = self.point;
        let screen = monitor::at_point(x, y)?;
        let width = pixels_at(component.get_card_width(), screen.dpi);
        let height = pixels_at(component.get_card_height(), screen.dpi);
        let (left, top) = place(
            screen.monitor,
            screen.work_area,
            x,
            y,
            width,
            height,
            margin_at(screen.dpi),
        );
        Some(Target {
            dpi: screen.dpi,
            width,
            height,
            left,
            top,
        })
    }

    /// Sizes the window for the monitor under the point and moves it to its place.
    fn place(&mut self, window: isize) {
        let Some(target) = self.target() else {
            self.close();
            return;
        };
        let Some(component) = &self.component else {
            return;
        };
        let handle = component.window();
        handle.set_size(PhysicalSize::new(
            target.width.max(1) as u32,
            target.height.max(1) as u32,
        ));
        handle.set_position(PhysicalPosition::new(target.left, target.top));
        self.seen = Some(observe(window, component));
    }

    /// Places the window again when its DPI or pixel size is not what was seen after the last
    /// placement. When its monitor's scale changed and the window was not told (Windows tells a
    /// shown window of the shell only when it is shown again), it is hidden and shown afresh, once
    /// per change.
    fn check(&mut self) {
        let Some(window) = self.window.filter(|_| self.toggle.is_open()) else {
            self.watch.stop();
            return;
        };
        if let Some(target) = self.target()
            && target.dpi != taskbar::dpi_of(window)
            && self.rescaled_for != Some(target.dpi)
        {
            self.rescaled_for = Some(target.dpi);
            self.rescaling = true;
            self.hide();
            self.present(SCALE_TRIES);
            return;
        }
        let Some(component) = &self.component else {
            return;
        };
        let now = observe(window, component);
        if now.dpi != 0 && Some(now) != self.seen {
            self.place(window);
        }
    }

    fn hide(&mut self) {
        self.watch.stop();
        if let Some(component) = &self.component {
            let _ = component.hide();
        }
    }

    /// Closes it when it is open.
    fn close(&mut self) {
        self.rescaling = false;
        if self.toggle.closed() {
            self.hide();
        }
    }

    /// Lets the window go: it did not come in time, or is gone.
    fn abandon(&mut self) {
        if self.window.is_none() {
            born::cancel_pending_focusable();
        }
        self.hide();
        self.component = None;
        self.window = None;
        self.made_at = None;
        self.seen = None;
        self.toggle.closed();
    }
}

/// Where the window goes, and at what size, in pixels.
struct Target {
    dpi: u32,
    width: i32,
    height: i32,
    left: i32,
    top: i32,
}

fn observe(window: isize, component: &FlyoutWindow) -> Seen {
    let size = component.window().size();
    let dpi = taskbar::dpi_of(window);
    Seen {
        dpi,
        width: size.width,
        height: size.height,
        wanted: (
            pixels_at(component.get_card_width(), dpi),
            pixels_at(component.get_card_height(), dpi),
        ),
    }
}

/// Told by `born`, inside the window's procedure, that a focusable window lost the activation.
/// Hiding the window deactivates it synchronously, so the work is left to a later turn. By then
/// the flyout may have the focus again (hidden on its first open, then shown and focused in the
/// same turn): a loss that no longer holds closes nothing, nor does one while it is hidden to take
/// a new scale.
fn deactivated(window: isize) {
    Timer::single_shot(Duration::ZERO, move || {
        with_flyout(move |flyout| {
            if flyout.window == Some(window)
                && !flyout.rescaling
                && !born::is_foreground(window)
                && flyout.toggle.focus_lost(Instant::now())
            {
                flyout.hide();
            }
        });
    });
}

/// A left click on the pill: closes the flyout when it is open, else opens it at the pointer.
pub fn on_pill_click() {
    with_flyout(|flyout| match flyout.toggle.click() {
        Click::Close => flyout.close(),
        Click::Open => {
            if let Some((x, y)) = monitor::cursor() {
                flyout.open(x, y);
            }
        }
    });
}

/// Opens the flyout beside the point (physical pixels). Open already: placed again there and
/// given the focus again.
#[allow(dead_code)] // Called by the tray icons (#49) and the second launch (#51).
pub fn open_at(x: i32, y: i32) {
    with_flyout(move |flyout| flyout.open(x, y));
}

/// Opens the flyout at the bottom right corner of the primary monitor's work area, where the tray
/// is; at the pointer when the monitor cannot be read.
#[allow(dead_code)] // Called by the tray icons (#49) and the second launch (#51).
pub fn open_by_tray() {
    let corner = monitor::primary()
        .map(|screen| (screen.work_area.right - 1, screen.work_area.bottom - 1))
        .or_else(monitor::cursor);
    if let Some((x, y)) = corner {
        open_at(x, y);
    }
}

/// Closes the flyout when it is open.
pub fn close() {
    with_flyout(Flyout::close);
}

/// Whether the flyout is open. False when it is busy this instant.
#[allow(dead_code)] // Called by the tray icons (#49).
pub fn is_open() -> bool {
    flyout_value(|flyout| flyout.toggle.is_open()).unwrap_or(false)
}

/// Whether the flyout closed because it lost the focus less than 300 ms ago, so a click that took
/// the focus is to leave it closed.
#[allow(dead_code)] // Called by the tray icons (#49).
pub fn closed_just_now() -> bool {
    flyout_value(|flyout| flyout.toggle.closed_just_now(Instant::now())).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_is_rounded_up_to_whole_pixels() {
        assert_eq!(pixels_at(330.0, 96), 330);
        assert_eq!(pixels_at(330.0, 144), 495);
        assert_eq!(pixels_at(100.5, 96), 101);
        assert_eq!(pixels_at(100.0, 120), 125);
    }

    #[test]
    fn the_margin_rounds_halves_to_even() {
        assert_eq!(margin_at(96), 12);
        assert_eq!(margin_at(144), 18);
        assert_eq!(margin_at(120), 15);
        // 12 * 100 / 96 = 12.5, which goes to the even 12.
        assert_eq!(margin_at(100), 12);
    }
}
