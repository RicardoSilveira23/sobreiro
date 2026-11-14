//! SPIKE (shell candidate B): the pill's tooltip in a window of its own, so that it opens above the taskbar.
//! The toolkit's tooltip is drawn inside the window it belongs to, which for the pill is 76 by 38 pixels.
//! This one is a second small window of the toolkit: never activated, owned by the taskbar, made the first
//! time it opens and hidden afterwards. It opens when the pointer has rested on the pill, with its bottom
//! edge a gap above the taskbar's band and its left edge on the pill's, as today's does.
use crate::{Tip, native, shell, trace};
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use windows::Win32::Foundation::HWND;

/// How long the pointer rests before the tooltip opens, and how long it stays open: as today's.
const OPEN_AFTER: Duration = Duration::from_millis(500);
const CLOSE_AFTER: Duration = Duration::from_secs(5);
/// Between the tooltip's bottom edge and the taskbar's band, at 100%.
const GAP: f64 = 8.0;

/// Where the pill is, in screen pixels, when the pointer comes to rest on it.
#[derive(Clone, Copy)]
pub struct Place {
    pub pill: native::PixelRect,
    /// The top of the taskbar's band.
    pub band_top: i32,
    pub taskbar: HWND,
}

#[derive(Default)]
pub struct TipWindow {
    window: RefCell<Option<Tip>>,
    born: Cell<bool>,
    visible: Cell<bool>,
    text: RefCell<String>,
    place: Cell<Option<Place>>,
    wait: slint::Timer,
    stay: slint::Timer,
}

impl TipWindow {
    /// The text the tooltip shows. An open tooltip follows it.
    pub fn set_text(self: &Rc<Self>, text: &str) {
        if *self.text.borrow() == text {
            return;
        }
        *self.text.borrow_mut() = text.to_owned();
        if let Some(window) = self.window.borrow().as_ref() {
            window.set_text(text.into());
        }
        if text.is_empty() {
            self.pointer_off();
        } else if self.visible.get() {
            self.put_soon();
        }
    }

    /// The pointer is on the pill: the tooltip opens if it stays.
    pub fn pointer_on(self: &Rc<Self>, place: Place) {
        self.place.set(Some(place));
        let tip = Rc::downgrade(self);
        self.wait.start(slint::TimerMode::SingleShot, OPEN_AFTER, move || {
            if let Some(tip) = tip.upgrade() {
                tip.open();
            }
        });
    }

    /// The pointer left the pill, or clicked it, or the pill went away.
    pub fn pointer_off(&self) {
        self.wait.stop();
        self.stay.stop();
        if self.visible.replace(false) {
            if let Some(window) = self.window.borrow().as_ref() {
                // The toolkit's own hide: a window hidden behind its back keeps it trying to repaint.
                let _ = window.hide();
            }
            trace!("tip closed");
        }
    }

    fn open(self: &Rc<Self>) {
        let (Some(place), false) = (self.place.get(), self.text.borrow().is_empty()) else { return };
        if self.window.borrow().is_none() {
            match Tip::new() {
                Ok(window) => {
                    window.set_text(self.text.borrow().as_str().into());
                    *self.window.borrow_mut() = Some(window);
                }
                Err(error) => {
                    trace!("tip window failed: {error}");
                    return;
                }
            }
        }
        {
            let borrowed = self.window.borrow();
            let Some(window) = borrowed.as_ref() else { return };
            if self.born.replace(true) {
                // Put where it belongs while still hidden, at the size it had last time.
                self.visible.set(true);
                self.put();
                if let Err(error) = window.show() {
                    trace!("tip show failed: {error}");
                }
            } else {
                window.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
                let want = shell::Want { owner: place.taskbar, ex_style: native::WS_EX_OURS, inactive: true };
                if !shell::show_born(shell::Born::Ours, want, || window.show()) {
                    self.born.set(false);
                    return;
                }
                self.visible.set(true);
            }
        }
        self.put_soon();
        let tip = Rc::downgrade(self);
        self.stay.start(slint::TimerMode::SingleShot, CLOSE_AFTER, move || {
            if let Some(tip) = tip.upgrade() {
                tip.pointer_off();
            }
        });
    }

    // Once Windows has the window, and again when the toolkit has settled its size for the text and the scale.
    fn put_soon(self: &Rc<Self>) {
        for after in [0, 40] {
            let tip = Rc::downgrade(self);
            slint::Timer::single_shot(Duration::from_millis(after), move || {
                if let Some(tip) = tip.upgrade() {
                    tip.put();
                }
            });
        }
    }

    fn put(&self) {
        let borrowed = self.window.borrow();
        let (Some(window), Some(place), true) = (borrowed.as_ref(), self.place.get(), self.visible.get()) else { return };
        let hwnd = shell::hwnd_of(window.window());
        let (Some(own), Some(monitor)) = (native::window_rect(hwnd), native::monitor_rect(place.taskbar)) else { return };
        let dpi = native::dpi(place.taskbar);
        let gap = (GAP * dpi as f64 / 96.0).round() as i32;
        let x = place.pill.left.min(monitor.right - own.width()).max(monitor.left);
        let y = place.band_top - gap - own.height();
        if own.left == x && own.top == y {
            return;
        }
        native::place_topmost(hwnd, x, y);
        trace!(
            "tip at {x},{y} size {}x{} window-dpi {} taskbar-dpi {dpi} scale {} wholly-above-the-taskbar={} birth={:?} exstyle=0x{:x} \
             active={}",
            own.width(),
            own.height(),
            native::dpi(hwnd),
            window.window().scale_factor(),
            y + own.height() <= place.band_top,
            shell::birth_of(hwnd),
            native::ex_style(hwnd),
            native::active() == hwnd,
        );
    }
}
