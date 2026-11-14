//! SPIKE (shell candidate B): the borderless toolkit window that holds the flyout card. It hides on Esc and
//! when it loses focus, and is placed by the ported logic beside the point it was opened from. The window
//! is made the first time the flyout is opened, not at start, and is hidden afterwards, not closed.
use crate::{Flyout, FlyoutCard, native, shell, trace};
use slint::ComponentHandle;
use slint::winit_030::{WinitWindowAccessor, EventResult, winit};
use sobreiro_core::display::{FlyoutPlacement, PixelRect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

const HIDE_GRACE: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct FlyoutWindow {
    window: RefCell<Option<Flyout>>,
    visible: Rc<Cell<bool>>,
    hidden_at: Rc<Cell<Option<Instant>>>,
    cursor: Cell<(i32, i32)>,
}

impl FlyoutWindow {
    pub fn is_visible(&self) -> bool {
        self.visible.get()
    }

    /// True for a moment after the flyout closed by losing focus: the click that took the focus is not
    /// also a click to open it again.
    pub fn hidden_just_now(&self) -> bool {
        self.hidden_at.get().is_some_and(|at| at.elapsed() < HIDE_GRACE)
    }

    pub fn hide(&self) {
        if let Some(window) = self.window.borrow().as_ref() {
            self.visible.set(false);
            let _ = window.hide();
        }
    }

    /// Swaps the card. A visible flyout is sized and placed again.
    pub fn render(&self, card: FlyoutCard) {
        self.ensure_window();
        if let Some(window) = self.window.borrow().as_ref() {
            window.set_card(card);
        }
        if self.is_visible() {
            self.place();
        }
    }

    /// Shows the window next to the tray, on the monitor under the cursor (screen pixels), and gives it the focus.
    pub fn show_near(self: &Rc<Self>, x: i32, y: i32) {
        self.cursor.set((x, y));
        self.ensure_window();
        {
            let borrowed = self.window.borrow();
            let Some(window) = borrowed.as_ref() else { return };
            if !self.is_visible() {
                window.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
                if let Err(error) = window.show() {
                    trace!("flyout show failed: {error}");
                    return;
                }
                self.visible.set(true);
                native::add_ex_style(shell::hwnd_of(window.window()), 0x80); // a tool window: no Alt+Tab entry
            }
            window.window().with_winit_window(|w| w.focus_window());
        }
        self.place();
        // Once more after the toolkit has settled its own size.
        let again = Rc::downgrade(self);
        slint::Timer::single_shot(Duration::from_millis(30), move || {
            if let Some(flyout) = again.upgrade() {
                flyout.place();
            }
        });
    }

    fn ensure_window(&self) {
        if self.window.borrow().is_some() {
            return;
        }
        let window = match Flyout::new() {
            Ok(window) => window,
            Err(error) => {
                trace!("flyout window failed: {error}");
                return;
            }
        };
        let weak = window.as_weak();
        let visible = self.visible.clone();
        window.on_escape(move || {
            if let Some(window) = weak.upgrade() {
                visible.set(false);
                let _ = window.hide();
            }
        });
        let weak = window.as_weak();
        let visible = self.visible.clone();
        let hidden_at = self.hidden_at.clone();
        window.window().on_winit_window_event(move |_, event| {
            if let winit::event::WindowEvent::Focused(false) = event {
                if visible.get() {
                    visible.set(false);
                    hidden_at.set(Some(Instant::now()));
                    trace!("flyout hidden: lost focus");
                    let weak = weak.clone();
                    // Not from inside the toolkit's own handling of the event.
                    slint::Timer::single_shot(Duration::ZERO, move || {
                        if let Some(window) = weak.upgrade() {
                            let _ = window.hide();
                        }
                    });
                }
            }
            EventResult::Propagate
        });
        *self.window.borrow_mut() = Some(window);
    }

    fn place(&self) {
        let borrowed = self.window.borrow();
        let Some(window) = borrowed.as_ref() else { return };
        if !self.is_visible() {
            return;
        }
        let hwnd = shell::hwnd_of(window.window());
        let (x, y) = self.cursor.get();
        let (Some((monitor, work, dpi)), Some(own)) = (native::screen_at(x, y), native::window_rect(hwnd)) else { return };
        let margin = (12.0 * dpi / 96.0).round() as i32;
        let (left, top) = FlyoutPlacement::place(rect(monitor), rect(work), x, y, own.width(), own.height(), margin);
        native::place_topmost(hwnd, left, top);
        trace!("flyout placed at {left},{top} size {}x{} scale {}", own.width(), own.height(), window.window().scale_factor());
    }
}

fn rect(r: native::PixelRect) -> PixelRect {
    PixelRect::new(r.left, r.top, r.right, r.bottom)
}
