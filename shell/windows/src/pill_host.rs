//! Keeps one pill over the taskbar, just left of the tray icons, as 1.1's tracker does: finds the
//! taskbar, follows it through auto-hide, scale and display changes and Explorer restarts, and
//! hides the pill while a full-screen app runs. Everything runs on the UI thread except the UI
//! Automation question, which is asked when Core's schedule says a tick should ask.
//!
//! Windows calls back with no data of ours, so the tracker lives in a value of the UI thread.
//! A callback that comes while the tracker is busy (Windows can call back inside a call the
//! tracker makes) runs a turn of the event loop later. Each step that fails is let go.
use crate::pictures;
use crate::pill_face::face_data;
use crate::ui::Pill;
use crate::win::born;
use crate::win::probe::Prober;
use crate::win::taskbar::{self, Broadcast, MessageWindow, WinEvent, WinEventHook};
use chrono::TimeDelta;
use slint::{ComponentHandle, Timer, TimerMode};
use sobreiro_core::colours::{BarColourOption, TaskbarColours};
use sobreiro_core::display::pill_model::PillModel;
use sobreiro_core::taskbar::full_screen::is_full_screen;
use sobreiro_core::taskbar::pill_placement;
use sobreiro_core::taskbar::pill_show::{self, PillShowStep, ZOrder};
use sobreiro_core::taskbar::taskbar_layout::TaskbarAnchors;
use sobreiro_core::taskbar::taskbar_query_schedule::TaskbarQuerySchedule;
use std::cell::RefCell;
use std::process::ExitCode;
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_secs(2);
/// How often a new pill's window is looked for until the toolkit has made it.
const FIND_POLL: Duration = Duration::from_millis(15);
/// How long after a pill shown at another scale it is placed again, at its new size.
const RESIZE_SETTLE: Duration = Duration::from_millis(250);
/// How many times it looks for that new size before placing anyway.
const RESIZE_TRIES: u32 = 4;
/// Where a new pill's window is made, off every screen.
const OFF_SCREEN: i32 = -32000;
/// The gap between the pill and the tray icons, in units.
const GAP_UNITS: f64 = 4.0;

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// Runs the pill with a fixed sample model and colours until the process ends.
pub fn run() -> ExitCode {
    taskbar::become_dpi_aware();
    // Made before the first pill: while `born` has a window pending it takes the next top-level
    // window this thread makes.
    let messages = MessageWindow::create(on_broadcast);
    let foreground = WinEventHook::foreground(on_win_event);
    let host = Host {
        started: Instant::now(),
        model: sample_model(),
        colours: sample_colours(),
        timer: Timer::default(),
        find_timer: Timer::default(),
        _messages: messages,
        _foreground_hook: foreground,
        location_hook: None,
        pill: None,
        taskbar: 0,
        tray_notify: 0,
        anchors: None,
        anchors_dpi: 0,
        querying: false,
        requery: false,
        generation: 0,
        schedule: TaskbarQuerySchedule::default(),
        prober: Prober::default(),
    };
    if HOST.try_with(|cell| cell.replace(Some(host))).is_err() {
        return ExitCode::FAILURE;
    }
    with_host(|host| {
        host.tick();
        host.timer
            .start(TimerMode::Repeated, TICK, || with_host(Host::tick));
    });
    // Runs until asked to quit: the pill's window can be destroyed under it (Explorer restarts).
    let ended = slint::run_event_loop_until_quit();
    // The tracker, its pill and its hooks go here, on the UI thread, before the thread's values
    // are torn down.
    let host = HOST
        .try_with(|cell| cell.try_borrow_mut().map(|mut host| host.take()))
        .ok()
        .and_then(Result::ok)
        .flatten();
    drop(host);
    match ended {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// Both services, as the pictures draw them. Live figures replace it later.
fn sample_model() -> PillModel {
    pictures::both_services()
}

/// A dark taskbar's colours with the purple bar. The taskbar's own theme replaces them later.
fn sample_colours() -> TaskbarColours {
    TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None)
}

/// Runs `step` on the tracker now, or a turn of the event loop later if it is busy.
fn with_host(step: impl FnOnce(&mut Host) + 'static) {
    let busy = HOST.try_with(|cell| match cell.try_borrow_mut() {
        Ok(mut host) => {
            if let Some(host) = host.as_mut() {
                step(host);
            }
            None
        }
        Err(_) => Some(step),
    });
    if let Ok(Some(step)) = busy {
        Timer::single_shot(Duration::ZERO, move || with_host(step));
    }
}

fn on_win_event(event: WinEvent) {
    with_host(move |host| host.on_win_event(event));
}

fn on_broadcast(broadcast: Broadcast) {
    // A broadcast is sent to every top-level window, and its sender may wait on the answer. Work
    // done inside it (hiding and dropping the old pill, making a new one) can block on the
    // sender, so it runs a turn of the event loop later, once the message has been answered.
    Timer::single_shot(Duration::ZERO, move || {
        with_host(move |host| host.on_broadcast(broadcast));
    });
}

/// The pill's size in pixels at `dpi`, from its size in units: rounded up, so no edge is cut.
fn size_at(width_units: f32, height_units: f32, dpi: u32) -> (i32, i32) {
    let scale = f64::from(dpi) / 96.0;
    let pixels = |units: f32| (f64::from(units) * scale).ceil() as i32;
    (pixels(width_units), pixels(height_units))
}

/// A window's size in pixels.
fn window_size(window: isize) -> Option<(i32, i32)> {
    taskbar::window_rect(window).map(|rect| (rect.width(), rect.height()))
}

/// The gap in pixels at `dpi`, rounded as 1.1 rounds it (halves to even).
fn gap_at(dpi: u32) -> i32 {
    (GAP_UNITS * f64::from(dpi) / 96.0).round_ties_even() as i32
}

/// One pill window: a toolkit window born owned by the taskbar.
struct PillWindow {
    component: Pill,
    /// The taskbar it was born owned by.
    owner: isize,
    /// Its window, once the toolkit has made it and its owner holds.
    window: Option<isize>,
    /// The windows `born` listed before this one was shown.
    before: Vec<isize>,
    shown_at: Instant,
}

impl PillWindow {
    /// Whether the pill may be dropped. The toolkit makes the window a turn of the event loop
    /// after it is shown, `born` takes it then, and it is known here once its owner holds. Until
    /// a tick has passed it may still be on its way and is kept; after that it is given up (if
    /// `born` is still waiting for it, that wait is cancelled as it is dropped).
    fn can_go(&self) -> bool {
        self.window.is_some() || self.shown_at.elapsed() >= TICK
    }
}

struct Host {
    started: Instant,
    model: PillModel,
    colours: TaskbarColours,
    timer: Timer,
    find_timer: Timer,
    _messages: Option<MessageWindow>,
    _foreground_hook: Option<WinEventHook>,
    location_hook: Option<WinEventHook>,
    pill: Option<PillWindow>,
    taskbar: isize,
    tray_notify: isize,
    anchors: Option<TaskbarAnchors>,
    /// The taskbar's DPI when the anchors were read.
    anchors_dpi: u32,
    querying: bool,
    requery: bool,
    generation: u64,
    schedule: TaskbarQuerySchedule,
    prober: Prober,
}

impl Host {
    /// Monotonic, so a change of the wall clock cannot make a question early or late.
    fn now(&self) -> TimeDelta {
        TimeDelta::from_std(self.started.elapsed()).unwrap_or(TimeDelta::MAX)
    }

    fn tick(&mut self) {
        let taskbar = taskbar::find_taskbar();
        if taskbar == 0 {
            self.hide();
            return;
        }
        if taskbar != self.taskbar {
            self.attach_to(taskbar);
        }
        self.find_window();
        if self.pill_is_gone() {
            self.close_window();
        }
        if self.pill.is_none() {
            self.create_window();
        }
        // Conditional: asking UI Automation on every tick was most of 1.1's idle processor time.
        if self.schedule.due(self.now()) {
            self.ask();
        }
        self.place();
    }

    fn attach_to(&mut self, taskbar: isize) {
        self.location_hook = None;
        self.close_window();
        self.drop_anchors();
        self.taskbar = taskbar;
        self.tray_notify = taskbar::find_tray_notify(taskbar);
        let (process, thread) = taskbar::process_and_thread(taskbar);
        self.location_hook = WinEventHook::location(process, thread, on_win_event);
    }

    fn drop_anchors(&mut self) {
        self.anchors = None;
        self.generation = self.generation.wrapping_add(1);
        self.schedule = TaskbarQuerySchedule::default();
    }

    /// The pill's window died (its owner went with Explorer), it belongs to another taskbar, or
    /// it never came.
    fn pill_is_gone(&self) -> bool {
        self.pill.as_ref().is_some_and(|pill| match pill.window {
            Some(window) => !born::is_window(window) || pill.owner != self.taskbar,
            None => pill.can_go(),
        })
    }

    fn create_window(&mut self) {
        if self.taskbar == 0 || self.pill.is_some() {
            return;
        }
        let Ok(component) = Pill::new() else {
            return;
        };
        component.set_data(face_data(&self.model, &self.colours, false));
        // Born off the screen, as 1.1's is, so it never shows a frame at the toolkit's default
        // place before it is placed or hidden. The toolkit keeps a position set before its
        // window exists and makes the window there.
        component
            .window()
            .set_position(slint::PhysicalPosition::new(OFF_SCREEN, OFF_SCREEN));
        let before = born::owned_windows();
        if born::show(component.window(), self.taskbar).is_err() {
            return;
        }
        self.pill = Some(PillWindow {
            component,
            owner: self.taskbar,
            window: None,
            before,
            shown_at: Instant::now(),
        });
        self.find_timer.start(TimerMode::Repeated, FIND_POLL, || {
            with_host(Host::on_find_poll)
        });
    }

    /// Hides and drops the pill, which destroys its window, unless its window may still be on its
    /// way.
    /// A pill given up before its window came leaves `born` waiting for it, which would refuse
    /// every later pill and take the next window this thread makes, so that wait is cancelled.
    fn close_window(&mut self) {
        let Some(pill) = self.pill.as_ref() else {
            return;
        };
        if !pill.can_go() {
            return;
        }
        // Only the pill is made through `born`, so a window still pending is this one's.
        if pill.window.is_none() && born::is_pending() {
            born::cancel_pending();
        }
        // The toolkit keeps a window it was told to show alive past its component, so it is
        // hidden through the toolkit first; only then does dropping the component destroy it.
        let _ = pill.component.hide();
        self.pill = None;
        self.find_timer.stop();
    }

    /// Notes the pill's window once `born` lists it. True when it was found just now.
    fn find_window(&mut self) -> bool {
        let Some(pill) = self.pill.as_mut() else {
            return false;
        };
        if pill.window.is_some() {
            return false;
        }
        let found = born::owned_windows()
            .into_iter()
            .rev()
            .find(|window| !pill.before.contains(window));
        pill.window = found;
        found.is_some()
    }

    fn on_find_poll(&mut self) {
        if self.find_window() {
            self.find_timer.stop();
            self.place();
        } else if self.pill.as_ref().is_none_or(PillWindow::can_go) {
            self.find_timer.stop();
        }
    }

    /// The pill's window, if it is known and alive.
    fn live_window(&self) -> Option<isize> {
        self.pill
            .as_ref()
            .and_then(|pill| pill.window)
            .filter(|window| born::is_window(*window))
    }

    fn hide(&self) {
        if let Some(window) = self.live_window() {
            taskbar::hide(window);
        }
    }

    /// One UI Automation question at a time, on the worker; the answer comes back through the
    /// event loop.
    fn ask(&mut self) {
        if self.taskbar == 0 {
            return;
        }
        if self.querying {
            self.requery = true;
            return;
        }
        self.querying = true;
        self.schedule = self.schedule.asked(self.now());
        let taskbar = self.taskbar;
        let generation = self.generation;
        let sent = self.prober.ask(
            taskbar,
            Box::new(move |answer| {
                let _ = slint::invoke_from_event_loop(move || {
                    with_host(move |host| host.on_probed(taskbar, generation, answer));
                });
            }),
        );
        if !sent {
            self.querying = false;
        }
    }

    fn on_probed(&mut self, taskbar: isize, generation: u64, answer: Option<TaskbarAnchors>) {
        self.querying = false;
        // The taskbar changed or the anchors were dropped while the question ran: the answer is
        // stale. No answer (auto-hidden, restarting) keeps the last good anchors.
        if taskbar == self.taskbar && generation == self.generation {
            self.schedule = self.schedule.answered(answer);
            if answer.is_some() {
                self.anchors = answer;
                self.anchors_dpi = taskbar::dpi_of(taskbar);
            }
            self.place();
        }
        if self.requery {
            self.requery = false;
            self.ask();
        }
    }

    fn place(&mut self) {
        let Some(pill) = self.pill.as_ref() else {
            return;
        };
        let Some(window) = pill.window else {
            return;
        };
        if !born::is_window(window) {
            // Dropped here; the next tick makes a new one.
            self.close_window();
            return;
        }
        let full_screen = taskbar::notification_state().is_some_and(is_full_screen);
        let (Some(taskbar_rect), Some(monitor)) = (
            taskbar::window_rect(self.taskbar),
            taskbar::monitor_rect(self.taskbar),
        ) else {
            taskbar::hide(window);
            return;
        };
        let dpi = taskbar::dpi_of(self.taskbar);
        if full_screen || dpi == 0 {
            taskbar::hide(window);
            return;
        }
        if self.anchors.is_some() && dpi != self.anchors_dpi {
            // The taskbar's scale changed (its location event can beat the display change): the
            // anchors are in the old scale, so drop them and ask again.
            self.drop_anchors();
            taskbar::hide(window);
            self.ask();
            return;
        }
        // A window at another scale than the taskbar is placed at the size it will have at the
        // taskbar's: Windows tells it its new scale when it is shown, and the toolkit resizes it.
        let other_scale = taskbar::dpi_of(window) != dpi;
        let size = match taskbar::window_rect(window) {
            Some(rect) if !other_scale => (rect.width(), rect.height()),
            _ => size_at(
                pill.component.get_pill_width(),
                pill.component.get_pill_height(),
                dpi,
            ),
        };
        let place = pill_placement::place(
            monitor,
            taskbar_rect,
            self.anchors,
            size.0,
            size.1,
            gap_at(dpi),
        );
        if !place.is_shown() {
            taskbar::hide(window);
            return;
        }
        let old_size = window_size(window);
        let changed = self.show_at(window, place.x, place.y);
        if changed && other_scale {
            // Placed again once the toolkit has resized it.
            Timer::single_shot(RESIZE_SETTLE, move || {
                with_host(move |host| host.settle(window, old_size, RESIZE_TRIES));
            });
        }
    }

    /// Places the pill again once its window is no longer old_size, looking again a few times
    /// before placing anyway. The tick is the backstop.
    fn settle(&mut self, window: isize, old_size: Option<(i32, i32)>, tries: u32) {
        if self.live_window() != Some(window) {
            return;
        }
        if window_size(window) == old_size && tries > 1 {
            Timer::single_shot(RESIZE_SETTLE, move || {
                with_host(move |host| host.settle(window, old_size, tries - 1));
            });
            return;
        }
        self.place();
    }

    /// Shows the pill at (x, y) as 1.1 does. A hidden pill, or one below the taskbar, goes back
    /// to the top of the topmost band (while it was hidden Explorer may have raised the taskbar
    /// past it), without moving the taskbar; otherwise it only moves, and not at all when it is in
    /// place. True when a call was made.
    fn show_at(&self, window: isize, x: i32, y: i32) -> bool {
        let visible = taskbar::is_visible(window);
        let in_place = taskbar::window_rect(window).is_some_and(|r| r.left == x && r.top == y);
        let above = ZOrder::is_above(window, self.taskbar, taskbar::window_above);
        match pill_show::step(visible, above, in_place) {
            PillShowStep::Raise => taskbar::raise_to(window, x, y),
            PillShowStep::Move => taskbar::move_to(window, x, y),
            PillShowStep::None => return false,
        }
        true
    }

    fn on_win_event(&mut self, event: WinEvent) {
        match event {
            WinEvent::Location { window, object } => {
                if object != 0 {
                    return;
                }
                if window == self.taskbar {
                    self.place();
                    if self.anchors.is_none() {
                        self.ask();
                    }
                } else if self.tray_notify != 0 && window == self.tray_notify {
                    self.ask();
                }
            }
            WinEvent::Foreground => {
                // No question here: the schedule notes the switch and the next due tick asks.
                self.schedule = self.schedule.foreground_switched();
                self.place();
            }
        }
    }

    fn on_broadcast(&mut self, broadcast: Broadcast) {
        match broadcast {
            Broadcast::TaskbarCreated => self.tick(),
            Broadcast::DisplayChanged => {
                self.drop_anchors();
                self.ask();
                self.place();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_at_a_scale_is_rounded_up() {
        assert_eq!(size_at(76.0, 38.0, 96), (76, 38));
        assert_eq!(size_at(76.0, 38.0, 144), (114, 57));
        assert_eq!(size_at(75.5, 37.2, 144), (114, 56));
        assert_eq!(size_at(76.0, 38.0, 120), (95, 48));
    }

    #[test]
    fn the_gap_is_four_units() {
        assert_eq!(gap_at(96), 4);
        assert_eq!(gap_at(120), 5);
        assert_eq!(gap_at(144), 6);
        assert_eq!(gap_at(192), 8);
        // Halves go to the even neighbour, as 1.1 rounds.
        assert_eq!(gap_at(60), 2);
        assert_eq!(gap_at(108), 4);
    }
}
