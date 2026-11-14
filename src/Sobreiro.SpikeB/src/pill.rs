//! SPIKE (shell candidate B): today's tracker on the toolkit's timer and event loop, and the toolkit's window
//! that holds the pill. Same schedule, same placement from the ported logic. Left out: the message that says
//! the taskbar was made again and the display-change event (the 2-second tick and the taskbar's own DPI
//! catch both, later).
use crate::{Pill, PillFace, native, shell, spike, trace, uia};
use slint::ComponentHandle;
use sobreiro_core::display::PixelRect;
use sobreiro_core::taskbar::{FullScreen, PillPlacement, PillShow, PillShowStep, TaskbarAnchors, TaskbarLayout, TaskbarQuerySchedule};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::HWINEVENTHOOK;

const TICK: Duration = Duration::from_secs(2);

struct Host {
    timer: slint::Timer,
    clock: Instant,
    window: Option<Pill>,
    taskbar: HWND,
    tray_notify: HWND,
    location_hook: HWINEVENTHOOK,
    foreground_hook: HWINEVENTHOOK,
    anchors: Option<TaskbarAnchors>,
    anchors_dpi: u32,
    querying: bool,
    requery: bool,
    generation: u32,
    schedule: TaskbarQuerySchedule,
    enabled: bool,
    face: Option<PillFace>,
    shown_face: Option<PillFace>,
    prepared: bool,
    left_clicked: Option<Rc<dyn Fn()>>,
    right_clicked: Option<Rc<dyn Fn()>>,
}

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host {
        timer: slint::Timer::default(),
        clock: Instant::now(),
        window: None,
        taskbar: HWND::default(),
        tray_notify: HWND::default(),
        location_hook: HWINEVENTHOOK::default(),
        foreground_hook: HWINEVENTHOOK::default(),
        anchors: None,
        anchors_dpi: 0,
        querying: false,
        requery: false,
        generation: 0,
        schedule: TaskbarQuerySchedule::default(),
        enabled: false,
        face: None,
        shown_face: None,
        prepared: false,
        left_clicked: None,
        right_clicked: None,
    });
}

// A window event can arrive while the host is busy (Windows delivers some from inside its own calls): it is
// dropped then, and the next tick sees the same state.
fn with_host(part: &str, action: impl FnOnce(&mut Host)) {
    HOST.with(|host| match host.try_borrow_mut() {
        Ok(mut host) => action(&mut host),
        Err(_) => trace!("pill {part}: skipped, the tracker was busy"),
    });
}

pub fn on_clicks(left: impl Fn() + 'static, right: impl Fn() + 'static) {
    with_host("handlers", |host| {
        host.left_clicked = Some(Rc::new(left));
        host.right_clicked = Some(Rc::new(right));
    });
}

pub fn set_enabled(enabled: bool) {
    with_host("enable", |host| {
        if enabled == host.enabled {
            return;
        }
        host.enabled = enabled;
        if enabled { host.start() } else { host.stop() }
    });
}

pub fn update(face: PillFace) {
    with_host("update", |host| {
        host.face = Some(face);
        if !host.enabled || host.taskbar.is_invalid() {
            return;
        }
        host.ensure_window();
        host.render();
        host.place();
    });
}

pub fn dispose() {
    with_host("dispose", |host| {
        host.enabled = false;
        host.stop();
    });
}

impl Host {
    fn now(&self) -> sobreiro_core::Span {
        sobreiro_core::Span::from_std(self.clock.elapsed()).unwrap_or_default()
    }

    fn start(&mut self) {
        self.foreground_hook = native::hook(native::EVENT_FOREGROUND, Some(on_foreground), 0, 0, true);
        self.timer.start(slint::TimerMode::Repeated, TICK, || with_host("tick", Host::tick));
        self.tick();
    }

    fn stop(&mut self) {
        self.timer.stop();
        native::unhook(&mut self.location_hook);
        native::unhook(&mut self.foreground_hook);
        self.close_window();
        self.anchors = None;
        self.generation += 1;
        self.schedule = TaskbarQuerySchedule::default();
        self.taskbar = HWND::default();
        self.tray_notify = HWND::default();
        self.requery = false;
    }

    fn tick(&mut self) {
        if !self.enabled {
            return;
        }
        let taskbar = native::find_taskbar();
        if taskbar.is_invalid() {
            if let Some(hwnd) = self.hwnd() {
                native::hide(hwnd);
            }
            return;
        }
        if taskbar != self.taskbar {
            self.attach_to(taskbar);
        }
        self.ensure_window();
        if self.schedule.due(self.now()) {
            self.ask_ui_automation();
        }
        self.place();
    }

    fn attach_to(&mut self, taskbar: HWND) {
        native::unhook(&mut self.location_hook);
        self.close_window();
        self.anchors = None;
        self.generation += 1;
        self.schedule = TaskbarQuerySchedule::default();
        self.taskbar = taskbar;
        self.tray_notify = native::find_tray_notify(taskbar);
        let (thread, process) = native::thread_and_process(taskbar);
        self.location_hook = native::hook(native::EVENT_LOCATION, Some(on_location_change), process, thread, false);
    }

    /// The window's handle, once the toolkit has made the window and for as long as Windows still has it.
    fn hwnd(&self) -> Option<HWND> {
        let hwnd = shell::hwnd_of(self.window.as_ref()?.window());
        native::is_window(hwnd).then_some(hwnd)
    }

    // Makes the window when there is a face to show and none yet, or when Windows has destroyed the old one.
    fn ensure_window(&mut self) {
        if self.window.is_some() && self.hwnd().is_none() && self.shown_face.is_some() {
            self.close_window();
        }
        if self.window.is_some() || self.face.is_none() || self.taskbar.is_invalid() {
            return;
        }
        // Born owned by the taskbar and inactive: the toolkit's hook reads this when it makes the window.
        shell::next_window_owned_by(self.taskbar);
        let window = match Pill::new() {
            Ok(window) => window,
            Err(error) => {
                trace!("pill window failed: {error}");
                return;
            }
        };
        let left = self.left_clicked.clone();
        window.on_left_clicked(move || {
            if let Some(left) = &left {
                left()
            }
        });
        let right = self.right_clicked.clone();
        window.on_right_clicked(move || {
            if let Some(right) = &right {
                right()
            }
        });
        window.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
        self.window = Some(window);
        self.shown_face = None;
        self.render();
    }

    /// Swaps the face; the window follows its width. The first call shows the window at its off-screen start.
    fn render(&mut self) {
        let (Some(window), Some(face)) = (&self.window, &self.face) else { return };
        if self.shown_face.as_ref() == Some(face) {
            return;
        }
        let first = self.shown_face.is_none();
        window.set_face(face.clone());
        self.shown_face = Some(face.clone());
        if first {
            self.prepared = false;
            if let Err(error) = window.show() {
                trace!("pill show failed: {error}");
            }
            // Before the event loop runs the toolkit only notes the window; it is made on the loop's first turn.
            slint::Timer::single_shot(Duration::ZERO, || with_host("first place", Host::place));
        }
    }

    // Once per window, as soon as Windows has it: our style bits, and the message that asks the app to leave.
    fn prepare(&mut self, hwnd: HWND) {
        if self.prepared {
            return;
        }
        self.prepared = true;
        native::add_ex_style(hwnd, native::WS_EX_OURS);
        shell::quit_on_end_session(hwnd);
        trace!(
            "pill window {:?} made, owner-is-taskbar={} exstyle=0x{:x} dpi={}",
            hwnd.0,
            native::owner(hwnd) == self.taskbar,
            native::ex_style(hwnd),
            native::dpi(hwnd)
        );
    }

    fn close_window(&mut self) {
        if let Some(window) = self.window.take() {
            let _ = window.hide();
        }
        self.shown_face = None;
    }

    /// One UI Automation question at a time, off the UI thread; the answer comes back through the event loop.
    fn ask_ui_automation(&mut self) {
        if !self.enabled || self.taskbar.is_invalid() {
            return;
        }
        if self.querying {
            self.requery = true;
            return;
        }
        self.querying = true;
        self.schedule = self.schedule.asked(self.now());
        let taskbar = self.taskbar.0 as isize;
        let generation = self.generation;
        let answer = move |answer: Option<uia::Answer>| {
            let _ = slint::invoke_from_event_loop(move || {
                with_host("anchors", |host| host.on_probed(HWND(taskbar as *mut _), generation, answer));
            });
        };
        if spike::env("SOBREIRO_SPIKE_UIA").as_deref() == Some("none") { answer(None) } else { uia::ask(self.taskbar, answer) }
    }

    fn on_probed(&mut self, taskbar: HWND, generation: u32, answer: Option<uia::Answer>) {
        self.querying = false;
        if !self.enabled {
            return;
        }
        let anchors = answer.and_then(|a| {
            let items = a.items.into_iter().map(|(class, rect, process)| (class, pixel_rect(rect), process));
            let buttons = a.buttons.into_iter().map(pixel_rect);
            TaskbarLayout::anchors(pixel_rect(a.bounds), items, buttons, std::process::id() as i32)
        });
        trace!("anchors: {anchors:?}");
        if taskbar == self.taskbar && generation == self.generation {
            self.schedule = self.schedule.answered(anchors);
            if anchors.is_some() {
                self.anchors = anchors;
                self.anchors_dpi = native::dpi(taskbar);
            }
            self.place();
        }
        if self.requery {
            self.requery = false;
            self.ask_ui_automation();
        }
    }

    fn place(&mut self) {
        if !self.enabled {
            return;
        }
        let Some(hwnd) = self.hwnd() else { return };
        self.prepare(hwnd);
        let full_screen = native::notification_state().is_some_and(FullScreen::is_full_screen);
        let (Some(taskbar_rect), Some(monitor)) = (native::window_rect(self.taskbar), native::monitor_rect(self.taskbar)) else {
            native::hide(hwnd);
            return;
        };
        if full_screen {
            native::hide(hwnd);
            return;
        }
        let dpi = native::dpi(self.taskbar);
        if self.anchors.is_some() && dpi != self.anchors_dpi {
            self.anchors = None;
            self.generation += 1;
            self.schedule = TaskbarQuerySchedule::default();
            native::hide(hwnd);
            self.ask_ui_automation();
            return;
        }
        let Some(own) = native::window_rect(hwnd) else { return };
        let gap = (4.0 * dpi as f64 / 96.0).round() as i32;
        let place = PillPlacement::place(to_core(monitor), to_core(taskbar_rect), self.anchors, own.width(), own.height(), gap);
        if place.is_shown() {
            self.move_to(hwnd, place.x, place.y)
        } else {
            if native::is_visible(hwnd) {
                trace!("pill hidden: {:?}", place.hidden);
            }
            native::hide(hwnd)
        }
    }

    /// Moves the window to (x, y) in screen pixels, above the taskbar, and shows it.
    fn move_to(&self, hwnd: HWND, x: i32, y: i32) {
        // The toolkit may rewrite the extended style or the owner when it changes a window: ours go back.
        if native::ex_style(hwnd) & native::WS_EX_OURS != native::WS_EX_OURS {
            native::add_ex_style(hwnd, native::WS_EX_OURS);
            trace!("pill: the extended style was put back");
        }
        if native::owner(hwnd) != self.taskbar {
            native::set_owner(hwnd, self.taskbar);
            trace!("pill: the owner was put back");
        }
        let visible = native::is_visible(hwnd);
        let own = native::window_rect(hwnd).unwrap_or_default();
        let in_place = own.left == x && own.top == y;
        let above = native::is_above(hwnd, self.taskbar);
        let step = PillShow::step(visible, above, in_place);
        if step == PillShowStep::None {
            return;
        }
        native::place(hwnd, x, y, step == PillShowStep::Raise);
        trace!(
            "pill {} to {x},{y} size {}x{} dpi {}",
            if step == PillShowStep::Raise { "raised" } else { "moved" },
            own.width(),
            own.height(),
            native::dpi(hwnd)
        );
    }
}

fn pixel_rect(r: (i32, i32, i32, i32)) -> PixelRect {
    PixelRect::new(r.0, r.1, r.2, r.3)
}

fn to_core(r: native::PixelRect) -> PixelRect {
    PixelRect::new(r.left, r.top, r.right, r.bottom)
}

// Out-of-context events arrive on the thread that set the hook, through its message loop: the UI thread.
unsafe extern "system" fn on_location_change(_hook: HWINEVENTHOOK, _event: u32, hwnd: HWND, id_object: i32, _child: i32, _thread: u32, _time: u32) {
    with_host("location", |host| {
        if !host.enabled || id_object != 0 {
            return;
        }
        if hwnd == host.taskbar {
            host.place();
            if host.anchors.is_none() {
                host.ask_ui_automation();
            }
        } else if !host.tray_notify.is_invalid() && hwnd == host.tray_notify {
            host.ask_ui_automation();
        }
    });
}

unsafe extern "system" fn on_foreground(_hook: HWINEVENTHOOK, _event: u32, _hwnd: HWND, _object: i32, _child: i32, _thread: u32, _time: u32) {
    with_host("foreground", |host| {
        if !host.enabled {
            return;
        }
        host.schedule = host.schedule.foreground_switched();
        host.place();
    });
}
