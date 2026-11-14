//! SPIKE (shell candidate B): the borderless toolkit window that holds the flyout card. It hides on Esc and
//! when it loses focus, and is placed by the ported logic beside the point it was opened from. The window
//! is made the first time the flyout is opened, not at start, and is hidden afterwards, not closed.
use crate::{Flyout, FlyoutCard, native, shell, trace};
use slint::ComponentHandle;
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
    born: Cell<bool>,
    listening: Cell<isize>,
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
                if self.born.replace(true) {
                    if let Err(error) = window.show() {
                        trace!("flyout show failed: {error}");
                        return;
                    }
                } else {
                    // A tool window from its birth: no taskbar button and no Alt+Tab entry. It may be activated.
                    let want = shell::Want { owner: Default::default(), ex_style: native::WS_EX_TOOL, inactive: false };
                    if !shell::show_born(shell::Born::Ours, want, || window.show()) {
                        self.born.set(false);
                        return;
                    }
                }
                self.visible.set(true);
            }
        }
        self.focus_and_place();
        // Once more after the toolkit has made the window, or settled its own size.
        let again = Rc::downgrade(self);
        slint::Timer::single_shot(Duration::from_millis(30), move || {
            if let Some(flyout) = again.upgrade() {
                flyout.focus_and_place();
            }
        });
    }

    // The window is given the focus and its place once Windows has it, which is a turn of the loop after it
    // was first shown.
    fn focus_and_place(&self) {
        let hwnd = {
            let borrowed = self.window.borrow();
            let Some(window) = borrowed.as_ref() else { return };
            let hwnd = shell::hwnd_of(window.window());
            if !native::is_window(hwnd) || !self.is_visible() {
                return;
            }
            if self.listening.replace(hwnd.0 as isize) != hwnd.0 as isize {
                trace!("flyout window {:?} made, birth={:?} exstyle=0x{:x}", hwnd.0, shell::birth_of(hwnd), native::ex_style(hwnd));
                self.hide_when_deactivated(hwnd, window);
            }
            hwnd
        };
        if native::foreground() != hwnd {
            trace!("flyout focus: asked={} foreground-is-flyout={}", native::focus(hwnd), native::foreground() == hwnd);
        }
        self.place();
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
        *self.window.borrow_mut() = Some(window);
    }

    // Windows says when the window stops being the active one; the toolkit's stable API does not.
    fn hide_when_deactivated(&self, hwnd: windows::Win32::Foundation::HWND, window: &Flyout) {
        let weak = window.as_weak();
        let visible = self.visible.clone();
        let hidden_at = self.hidden_at.clone();
        shell::on_deactivated(hwnd, move || {
            if visible.get() {
                visible.set(false);
                hidden_at.set(Some(Instant::now()));
                trace!("flyout hidden: lost focus");
                let weak = weak.clone();
                // Not from inside the window's own handling of the message.
                slint::Timer::single_shot(Duration::ZERO, move || {
                    if let Some(window) = weak.upgrade() {
                        let _ = window.hide();
                    }
                });
            }
        });
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
