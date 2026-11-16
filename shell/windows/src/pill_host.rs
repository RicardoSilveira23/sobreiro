//! Keeps one pill over the taskbar, just left of the tray icons, as 1.1's tracker does: finds the
//! taskbar, follows it through auto-hide, scale and display changes and Explorer restarts, and
//! hides the pill while a full-screen app runs. It draws what the app host hands it: each redraw
//! brings a snapshot (see `pill_feed`), and the pill's model, colours and tooltip text are taken
//! from it. The polls, the refresh, the settings and the theme are the app host's (`app_host`).
//! The UI Automation question is asked on a worker, when Core's schedule says a tick should ask.
//!
//! The pill answers the pointer as 1.1's does: a tooltip in a window of its own (`tooltip_window`)
//! opens when the pointer has rested on it (`tooltip` says when and where), a left click calls the
//! flyout's hook, and a right click opens Windows' own menu (`pill_menu`).
//!
//! Windows calls back with no data of ours, so the tracker lives in a value of the UI thread.
//! A callback that comes while the tracker is busy (Windows can call back inside a call the
//! tracker makes) runs a turn of the event loop later. Each step that fails is let go.
use crate::about;
use crate::app_host;
use crate::auto_start;
use crate::connect_flow;
use crate::diagnostics::Diagnostics;
use crate::pill_face::face_data;
use crate::pill_feed::{self, Snapshot};
use crate::pill_menu::{self, Command, ConfigChange};
use crate::tooltip::{self, Hover, Step};
use crate::tooltip_window::TooltipWindow;
use crate::ui::Pill;
use crate::win::probe::Prober;
use crate::win::taskbar::{self, Broadcast, WinEvent, WinEventHook};
use crate::win::{born, menu, message_box};
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
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
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
/// How many times the tooltip waits for its window to take the taskbar's scale before it shows
/// anyway, at one try every `FIND_POLL`.
const SCALE_TRIES: u32 = 20;
/// How many times a right click waits for `born` to finish a window, at one try every `FIND_POLL`.
const MENU_TRIES: u32 = 100;
/// How many times a menu that was refused is tried again.
const MENU_REFUSALS: u32 = 3;
/// A menu that closes sooner than this, with no choice, did not open.
const MENU_REFUSED: Duration = Duration::from_millis(150);

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// What the app host gives the pill's host when it starts.
pub struct Setup {
    /// The app host's hidden window, which the pill's menu is owned by.
    pub messages: Option<isize>,
    pub diagnostics: Arc<Diagnostics>,
    /// `config.json`, which the menu reads and saves its choices to.
    pub config_file: PathBuf,
}

/// Makes the pill's host and starts following the taskbar. The first pill comes with the first
/// snapshot. False when the host could not be kept.
pub fn start(setup: Setup) -> bool {
    let foreground = WinEventHook::foreground(on_win_event);
    if foreground.is_none() {
        setup
            .diagnostics
            .say("the foreground hook could not be set");
    }
    let host = Host {
        started: Instant::now(),
        model: None,
        colours: initial_colours(),
        diagnostics: setup.diagnostics,
        config_file: setup.config_file,
        component_failed: false,
        window_failed: false,
        tooltip_failed: false,
        timer: Timer::default(),
        find_timer: Timer::default(),
        hover_timer: Timer::default(),
        messages: setup.messages,
        _foreground_hook: foreground,
        location_hook: None,
        pill: None,
        tooltip: None,
        tooltip_text: String::new(),
        hover: Hover::default(),
        hovered: false,
        band_top: 0,
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
        return false;
    }
    with_host(|host| {
        host.timer
            .start(TimerMode::Repeated, TICK, || with_host(Host::tick));
    });
    true
}

/// Lets the tracker, its pill and its hooks go. Called on the UI thread before the thread's
/// values are torn down.
pub fn stop() {
    let host = HOST
        .try_with(|cell| cell.try_borrow_mut().map(|mut host| host.take()))
        .ok()
        .and_then(Result::ok)
        .flatten();
    drop(host);
}

/// Draws the pill from a snapshot, now or a turn of the event loop later if the tracker is busy.
/// A pill that fails to draw logs once and keeps its last face (see `Host::show`).
pub fn show(snapshot: Rc<Snapshot>) {
    with_host(move |host| host.show(snapshot));
}

/// Hands on a broadcast the taskbar work answers to.
pub fn on_broadcast(broadcast: Broadcast) {
    with_host(move |host| host.on_broadcast(broadcast));
}

/// A dark taskbar's colours with the purple bar, until the theme is read.
fn initial_colours() -> TaskbarColours {
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

/// Runs `step` on the tracker now and gives back what it answers, or none when the tracker is
/// busy or gone.
fn host_value<T>(step: impl FnOnce(&mut Host) -> T) -> Option<T> {
    HOST.try_with(|cell| {
        let mut host = cell.try_borrow_mut().ok()?;
        host.as_mut().map(step)
    })
    .ok()
    .flatten()
}

/// What the tracker gives for opening the menu.
enum MenuRequest {
    Ready(isize, Vec<menu::Entry>),
    Wait,
    Never,
}

/// Opens the menu at the pointer: the tray icons' right click opens the same one.
pub fn open_menu_at_pointer() {
    Timer::single_shot(Duration::ZERO, || open_menu(MENU_TRIES, MENU_REFUSALS));
}

/// Opens the pill's menu at the pointer. The tracker is asked for what the menu needs and let go
/// before the menu opens: `TrackPopupMenu` runs its own loop until the menu closes, and the
/// tracker's other work (win events, timers) would pile up behind a borrow held that long. The
/// choice is handed back to the tracker afterwards. The toolkit's own handler, which called us
/// through a timer, still waits for the loop; nothing cheaper is known, as the menu has to be
/// opened from a thread that pumps messages.
fn open_menu(tries: u32, refusals: u32) {
    let again = move |tries: u32, refusals: u32| {
        if tries > 0 {
            Timer::single_shot(FIND_POLL, move || open_menu(tries - 1, refusals));
        }
    };
    match host_value(|host| host.menu_request()) {
        // The tracker was busy: the click waits for it.
        None => again(tries, refusals),
        Some(MenuRequest::Never) => {}
        Some(MenuRequest::Wait) => again(tries, refusals),
        Some(MenuRequest::Ready(owner, entries)) => {
            let opened = Instant::now();
            let chosen = menu::track(owner, &entries);
            // A menu that was gone before anyone could have read it never opened (Windows refused
            // the foreground, say): it is tried again, a few times, so a right click opens one.
            if chosen.is_none() && opened.elapsed() < MENU_REFUSED && refusals > 0 {
                again(tries, refusals - 1);
                return;
            }
            with_host(move |host| host.run_choice(chosen));
        }
    }
}

/// Flips Start with Windows, reading the value afresh, and says so in a box when it cannot.
fn toggle_auto_start() {
    if let Err(reason) = auto_start::toggle() {
        error_box(
            format!("Couldn't change Start with Windows.\n\n{reason}"),
            MENU_TRIES,
        );
    }
}

/// Shows an error box on a thread of its own, as the Connect flow's boxes are: the event loop goes
/// on behind it (the figures, the timers, the menu), as 1.1's does, and `born`, which hooks this
/// thread only, never takes it for a window it waits for. When no thread can start, it shows here.
fn error_box(text: String, tries: u32) {
    let on_its_own = text.clone();
    let started = std::thread::Builder::new()
        .name("error-box".into())
        .spawn(move || message_box::error(&on_its_own));
    if started.is_err() {
        error_box_here(text, tries);
    }
}

/// Shows an error box on this thread once `born` waits for no window: the box is a top-level
/// window this thread makes, which `born` would take for the one it waits for. After the last try
/// it shows anyway.
fn error_box_here(text: String, tries: u32) {
    if born::is_pending() && tries > 0 {
        Timer::single_shot(FIND_POLL, move || error_box_here(text, tries - 1));
        return;
    }
    message_box::error(&text);
}

fn on_win_event(event: WinEvent) {
    with_host(move |host| host.on_win_event(event));
}

/// The pill's size in pixels at `dpi`, from its size in units: rounded up, so no edge is cut.
fn size_at(width_units: f32, height_units: f32, dpi: u32) -> (i32, i32) {
    let scale = f64::from(dpi) / 96.0;
    // Less a hair: a whole number of pixels (56 at 150%) held as f32 units can come out just over.
    let pixels = |units: f32| (f64::from(units) * scale - 0.001).ceil() as i32;
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
    /// What the pill shows; none while the settings keep it off.
    model: Option<PillModel>,
    colours: TaskbarColours,
    diagnostics: Arc<Diagnostics>,
    /// `config.json`, which the menu reads and saves its choices to.
    config_file: PathBuf,
    /// The pill's component or window could not be made at the last try, so it is told once.
    component_failed: bool,
    window_failed: bool,
    tooltip_failed: bool,
    timer: Timer,
    find_timer: Timer,
    /// Wakes the tooltip's timing at the next moment it has something to do.
    hover_timer: Timer,
    /// The app host's hidden window, which the pill's menu is owned by.
    messages: Option<isize>,
    _foreground_hook: Option<WinEventHook>,
    location_hook: Option<WinEventHook>,
    pill: Option<PillWindow>,
    /// Made once the pill's window is known, and kept hidden until the pointer has rested.
    tooltip: Option<TooltipWindow>,
    /// What the tooltip says, which is also the pill's name for a screen reader.
    tooltip_text: String,
    hover: Hover,
    /// The pointer is on the pill.
    hovered: bool,
    /// The top of the taskbar's band as the pill was last placed, in pixels.
    band_top: i32,
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
        if self.model.is_none() {
            // The settings keep the pill off: no window.
            self.close_window();
            return;
        }
        self.find_window();
        if self.pill_is_gone() {
            self.close_window();
        }
        if self.pill.is_none() {
            self.create_window();
        }
        self.keep_tooltip();
        // Conditional: asking UI Automation on every tick was most of 1.1's idle processor time.
        if self.schedule.due(self.now()) {
            self.ask();
        }
        self.place();
    }

    /// Takes what a snapshot says of the pill: the model, the colours and the tooltip's text, and
    /// draws them. A pill that is not there while there is a model is made at once, so a pill
    /// switched back on does not wait for the next tick. The pill's steps log their own failures
    /// and keep the last face, so nothing here stops the app host's other parts.
    fn show(&mut self, snapshot: Rc<Snapshot>) {
        let (model, text) = match &snapshot.pill {
            Some(shown) => (Some(shown.model.clone()), shown.tooltip.clone()),
            None => (None, String::new()),
        };
        self.colours = snapshot.colours;
        self.model = model;
        let text_changed = text != self.tooltip_text;
        self.tooltip_text = text;
        if self.model.is_some() && self.pill.is_none() {
            self.tick();
        }
        self.show_model();
        if text_changed && self.hover.is_open() {
            // Open now: it takes the new words, and the size they want.
            self.open_tooltip(SCALE_TRIES);
        }
    }

    /// Puts the model and the colours on the pill, or takes the pill away when there is no model.
    fn show_model(&mut self) {
        if self.model.is_none() {
            self.close_window();
            return;
        }
        if self.draw() {
            self.place();
        }
    }

    /// Draws the model, the pointer's ground and the name on the pill. True when there is a pill.
    fn draw(&self) -> bool {
        let (Some(model), Some(pill)) = (self.model.as_ref(), self.pill.as_ref()) else {
            return false;
        };
        pill.component
            .set_data(face_data(model, &self.colours, self.hovered));
        pill.component
            .set_name(slint::SharedString::from(self.tooltip_text.as_str()));
        true
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
        let Some(model) = self.model.as_ref() else {
            return;
        };
        let component = Pill::new();
        let failed = component.is_err();
        if failed && !self.component_failed {
            self.diagnostics
                .say("the pill's component could not be made");
        }
        self.component_failed = failed;
        let Ok(component) = component else {
            return;
        };
        component.set_data(face_data(model, &self.colours, false));
        component.set_name(slint::SharedString::from(self.tooltip_text.as_str()));
        component.on_hover_changed(|over| with_host(move |host| host.on_hover(over)));
        component.on_pressed(|| with_host(Host::on_pressed));
        component.on_left_clicked(|| with_host(Host::on_left_click));
        component.on_right_clicked(|| with_host(Host::on_right_click));
        // Born off the screen, as 1.1's is, so it never shows a frame at the toolkit's default
        // place before it is placed or hidden. The toolkit keeps a position set before its
        // window exists and makes the window there.
        component
            .window()
            .set_position(slint::PhysicalPosition::new(OFF_SCREEN, OFF_SCREEN));
        let before = born::owned_windows();
        let shown = born::show(component.window(), self.taskbar);
        if shown.is_err() && !self.window_failed {
            self.diagnostics.say("the pill's window could not be shown");
        }
        self.window_failed = shown.is_err();
        if shown.is_err() {
            return;
        }
        self.hovered = false;
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
        // The tooltip is made only once the pill's window is known, so a window still pending
        // while the pill has none is the pill's.
        if pill.window.is_none() && born::is_pending() {
            born::cancel_pending();
        }
        // The toolkit keeps a window it was told to show alive past its component, so it is
        // hidden through the toolkit first; only then does dropping the component destroy it.
        let _ = pill.component.hide();
        self.pill = None;
        self.hovered = false;
        // The tooltip is owned by the taskbar the pill was, and goes with it.
        self.drop_tooltip();
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
            self.place();
        }
        // Only one window is made at a time: the tooltip's is made once the pill's is there, and
        // looked for until `born` lists it.
        self.keep_tooltip();
        let tooltip_waits = self.tooltip.as_ref().is_some_and(TooltipWindow::is_waiting);
        let pill_waits = self.pill.as_ref().is_some_and(|pill| !pill.can_go());
        let pill_comes = self.pill.as_ref().is_some_and(|pill| pill.window.is_none());
        if !tooltip_waits && !(pill_comes && pill_waits) {
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

    fn hide(&mut self) {
        if let Some(window) = self.live_window() {
            self.hide_window(window);
        }
    }

    /// Hides the pill's window, and with it an open tooltip: the pointer is no longer on it.
    fn hide_window(&mut self, window: isize) {
        taskbar::hide(window);
        self.hover.reset();
        self.hover_timer.stop();
        if let Some(tooltip) = self.tooltip.as_ref() {
            tooltip.hide();
        }
        if self.hovered {
            self.hovered = false;
            self.draw();
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
            self.hide_window(window);
            return;
        };
        let dpi = taskbar::dpi_of(self.taskbar);
        if full_screen || dpi == 0 {
            self.hide_window(window);
            return;
        }
        if self.anchors.is_some() && dpi != self.anchors_dpi {
            // The taskbar's scale changed (its location event can beat the display change): the
            // anchors are in the old scale, so drop them and ask again.
            self.drop_anchors();
            self.hide_window(window);
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
            self.hide_window(window);
            return;
        }
        self.band_top = place.band_top;
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

    /// Makes the tooltip's window once the pill's is known, looks for it until `born` lists it,
    /// and lets it go when it is lost.
    fn keep_tooltip(&mut self) {
        let (found, lost, never_came) = match self.tooltip.as_mut() {
            Some(tooltip) => {
                let found = tooltip.find();
                (found, tooltip.is_lost(), tooltip.window().is_none())
            }
            None => (false, false, false),
        };
        if lost {
            if never_came {
                // Said once until a window comes, so a window that never does is not written
                // about every two seconds.
                if !self.tooltip_failed {
                    self.diagnostics
                        .say("the tooltip's window did not come, so a new one is made");
                }
                self.tooltip_failed = true;
            }
            self.drop_tooltip();
        }
        if found {
            self.tooltip_failed = false;
            // The pointer rested while the window was on its way: it opens now.
            if self.hover.is_open() {
                self.open_tooltip(SCALE_TRIES);
            }
        }
        if self.tooltip.is_some() || self.taskbar == 0 || born::is_pending() {
            return;
        }
        if self.pill.as_ref().is_none_or(|pill| pill.window.is_none()) {
            return;
        }
        match TooltipWindow::create(self.taskbar) {
            Ok(tooltip) => {
                self.tooltip_failed = false;
                tooltip.set_text(&self.tooltip_text);
                self.tooltip = Some(tooltip);
                self.find_timer.start(TimerMode::Repeated, FIND_POLL, || {
                    with_host(Host::on_find_poll)
                });
            }
            Err(_) => {
                if !self.tooltip_failed {
                    self.diagnostics
                        .say("the tooltip's window could not be made");
                }
                self.tooltip_failed = true;
            }
        }
    }

    /// Hides and drops the tooltip, and forgets any wait for it.
    fn drop_tooltip(&mut self) {
        self.hover.reset();
        self.hover_timer.stop();
        if let Some(tooltip) = self.tooltip.take() {
            tooltip.close();
        }
    }

    /// The pointer came onto the pill or left it.
    fn on_hover(&mut self, over: bool) {
        self.hovered = over;
        self.draw();
        let now = self.started.elapsed();
        let step = if over {
            self.hover.entered(now)
        } else {
            self.hover.left()
        };
        self.apply(step);
        self.arm_hover();
    }

    /// A button went down on the pill: the tooltip closes.
    fn on_pressed(&mut self) {
        let step = self.hover.clicked();
        self.apply(step);
        self.arm_hover();
    }

    fn on_hover_due(&mut self) {
        let step = self.hover.tick(self.started.elapsed());
        self.apply(step);
        self.arm_hover();
    }

    fn apply(&mut self, step: Step) {
        match step {
            Step::Open => self.open_tooltip(SCALE_TRIES),
            Step::Close => self.close_tooltip(),
            Step::Nothing => {}
        }
    }

    /// Wakes `on_hover_due` at the next moment the tooltip's timing has something to do.
    fn arm_hover(&mut self) {
        match self.hover.next_due(self.started.elapsed()) {
            Some(wait) => self.hover_timer.start(TimerMode::SingleShot, wait, || {
                with_host(Host::on_hover_due)
            }),
            None => self.hover_timer.stop(),
        }
    }

    fn close_tooltip(&self) {
        if let Some(tooltip) = self.tooltip.as_ref() {
            tooltip.hide();
        }
    }

    /// Shows the tooltip above the pill: its bottom 8 units above the taskbar's band, its left
    /// edge on the pill's, kept inside the monitor. The window is first moved to the taskbar's
    /// monitor, hidden, until it has that monitor's scale (a window shown at another scale is
    /// drawn once at the wrong size), and then raised above the taskbar without moving its owner.
    fn open_tooltip(&self, tries: u32) {
        if !self.hover.is_open() {
            return;
        }
        let Some(tooltip) = self.tooltip.as_ref() else {
            return;
        };
        let (Some(window), Some(pill)) = (tooltip.window(), self.live_window()) else {
            return;
        };
        let dpi = taskbar::dpi_of(self.taskbar);
        let (Some(pill_rect), Some(monitor)) = (
            taskbar::window_rect(pill),
            taskbar::monitor_rect(self.taskbar),
        ) else {
            return;
        };
        if dpi == 0 || !taskbar::is_visible(pill) {
            return;
        }
        tooltip.set_text(&self.tooltip_text);
        let (width, height) = tooltip.size_units();
        let wanted = size_at(width, height, dpi);
        let gap = tooltip::gap_at(dpi);
        let at_scale = taskbar::dpi_of(window) == dpi;
        if !at_scale && tries > 0 {
            let (x, y) = tooltip::place(monitor, pill_rect.left, self.band_top, wanted, gap);
            taskbar::move_hidden(window, x, y);
            Timer::single_shot(FIND_POLL, move || {
                with_host(move |host| host.open_tooltip(tries - 1));
            });
            return;
        }
        tooltip.fit();
        let size = if at_scale {
            window_size(window).unwrap_or(wanted)
        } else {
            wanted
        };
        let (x, y) = tooltip::place(monitor, pill_rect.left, self.band_top, size, gap);
        tooltip.present(x, y);
    }

    /// A left click on the pill.
    fn on_left_click(&mut self) {
        self.flyout_hook();
    }

    /// A left click on the pill opens the flyout, or closes it, once the pointer's own handler has
    /// returned.
    fn flyout_hook(&mut self) {
        Timer::single_shot(Duration::ZERO, crate::flyout::window::on_pill_click);
    }

    /// A right click on the pill opens the menu, once the pointer's own handler has returned.
    fn on_right_click(&mut self) {
        Timer::single_shot(Duration::ZERO, || open_menu(MENU_TRIES, MENU_REFUSALS));
    }

    /// What `open_menu` needs from the tracker: the window the menu is owned by and the entries,
    /// or that a window of `born`'s is still being made.
    fn menu_request(&self) -> MenuRequest {
        let Some(owner) = self.messages else {
            self.diagnostics
                .say("the menu has no window of ours to open from");
            return MenuRequest::Never;
        };
        // A menu is a window the thread makes, and `born` would take it for a window it waits for
        // (the tooltip's, made when the pill came back): the click waits for it, and is not lost.
        if born::is_pending() {
            return MenuRequest::Wait;
        }
        let (config, _) = pill_feed::config_for_display(&self.config_file);
        // Read afresh each time: another run may have changed the value.
        let auto_start = auto_start::is_on_now();
        let state = connect_flow::current_state();
        MenuRequest::Ready(owner, pill_menu::entries(&config, auto_start, state))
    }

    /// Does what the menu's choice says.
    fn run_choice(&mut self, chosen: Option<u32>) {
        match chosen.and_then(pill_menu::command) {
            Some(Command::Change(change)) => self.change_config(change),
            Some(Command::Connect(connect)) => self.connect(connect),
            // The error box is a modal loop, so it runs once this tracker is let go.
            Some(Command::ToggleAutoStart) => Timer::single_shot(Duration::ZERO, toggle_auto_start),
            Some(Command::OpenDataFolder) => self.open_data_folder(),
            Some(Command::About) => about::show_about(),
            Some(Command::Quit) => {
                let _ = slint::quit_event_loop();
            }
            None => {}
        }
    }

    /// Opens the data folder in Explorer; a failure is logged as one fixed line.
    fn open_data_folder(&self) {
        if !about::open_data_folder() {
            self.diagnostics.say("the data folder could not be opened");
        }
    }

    /// Runs the connect flow on its own thread, as it has boxes to wait on. When it is done the
    /// UI thread is asked for a redraw, so the new state shows at once. A flow already open, or a
    /// thread that cannot start, means this choice does nothing.
    fn connect(&self, connect: bool) {
        let started = connect_flow::start(connect, || {
            let _ = slint::invoke_from_event_loop(app_host::redraw);
        });
        if started.is_none() {
            self.diagnostics
                .say("the connect flow was not started: one is open, or its thread did not start");
        }
    }

    /// Saves the change through Core and asks the app host to draw it at once. The redraw is
    /// asked for once this tracker is let go: made now, its snapshot would wait for the tracker
    /// and could land after a newer one. So is the error box, a modal loop.
    fn change_config(&mut self, change: ConfigChange) {
        match pill_menu::change_config(&self.config_file, change) {
            Ok(true) => Timer::single_shot(Duration::ZERO, app_host::redraw),
            Ok(false) => {}
            Err(error) => {
                self.diagnostics
                    .say("config.json was not changed: it could not be read or saved");
                let text = pill_menu::error_text(&error);
                Timer::single_shot(Duration::ZERO, move || error_box(text, MENU_TRIES));
            }
        }
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
            // The figures and the theme are the app host's; it redraws on these.
            Broadcast::SettingChanged | Broadcast::TimeChanged => {}
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
