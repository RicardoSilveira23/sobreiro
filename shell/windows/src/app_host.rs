//! Runs what the app needs besides the taskbar: the places, the diagnostics log, the feed and its
//! polls (each on a thread of its own, whose answer comes back through the event loop), the early
//! read of Copilot's cache, the 30 second refresh, the theme and `config.json` read for drawing,
//! and the broadcasts of the message window.
//!
//! Each redraw makes one snapshot (`pill_feed::Snapshot`) and hands it to each part in turn: for
//! now the pill (`pill_host`, which keeps the taskbar work) and the tray icons (`tray_icons`); the
//! flyout and the toasts come next. The snapshot is made whatever Show says, so a part that is
//! not the pill has the figures also when the pill is off. A part takes the snapshot on its own
//! and handles its own failures, keeping what it last drew, so one part's failure never stops the
//! next, as 1.1's `Guard` does. (A panic is not caught: the release build aborts on one.)
//!
//! Redraws come at the start, every 30 seconds, when a poll answers, when Copilot's cache is
//! read early, after the menu changes the bar colour, and when Windows says a setting, a display
//! or the time changed.
//!
//! Windows calls back with no data of ours, so the host lives in a value of the UI thread. A
//! callback that comes while the host is busy runs a turn of the event loop later.
use crate::carried_bridge;
use crate::clock;
use crate::diagnostics::Diagnostics;
use crate::pill_feed::{self, Feed, Run, Service, Snapshot, Surroundings};
use crate::pill_host;
use crate::providers::{self, Locations};
use crate::tray_icons;
use crate::win::local_zone::local_zone;
use crate::win::taskbar::{self, Broadcast, MessageWindow};
use crate::win::{locale, registry};
use slint::{Timer, TimerMode};
use sobreiro_core::connect::connection;
use sobreiro_core::copilot::copilot_provider::CopilotProvider;
use sobreiro_core::polling::provider_poller::Finished;
use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// How often the figures and the theme are read again, as 1.1 redraws.
const REFRESH: Duration = Duration::from_secs(30);

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// Runs the app with live figures until the process ends.
pub fn run() -> ExitCode {
    let Some(places) = Locations::find() else {
        return ExitCode::FAILURE;
    };
    let diagnostics = Arc::new(Diagnostics::new(&places.app.log_file()));
    let made = providers::make(&places, &diagnostics);
    let copilot = made.copilot.clone();
    let feed = Feed::new(made.claude, made.copilot);
    taskbar::become_dpi_aware();
    // Made before the first pill: while `born` has a window pending it takes the next top-level
    // window this thread makes.
    let messages = MessageWindow::create(on_broadcast, tray_icons::on_event);
    if messages.is_none() {
        diagnostics.say("the message window could not be made");
    }
    tray_icons::start(
        messages.as_ref().map(MessageWindow::handle),
        diagnostics.clone(),
    );
    let setup = pill_host::Setup {
        messages: messages.as_ref().map(MessageWindow::handle),
        diagnostics: diagnostics.clone(),
        config_file: places.app.config_file(),
    };
    let host = Host {
        places,
        diagnostics,
        feed,
        copilot,
        config_unreadable: false,
        refresh_timer: Timer::default(),
        _messages: messages,
    };
    if HOST.try_with(|cell| cell.replace(Some(host))).is_err() {
        return ExitCode::FAILURE;
    }
    if !pill_host::start(setup) {
        return ExitCode::FAILURE;
    }
    with_host(|host| {
        // Drawn once with what is there, as 1.1 does.
        host.redraw();
        host.refresh_timer
            .start(TimerMode::Repeated, REFRESH, || with_host(Host::refresh));
    });
    // The polls start from the loop's first turn: a poll that answers at once hands its answer
    // back with `invoke_from_event_loop`, which fails until the toolkit has an event loop.
    Timer::single_shot(Duration::ZERO, || with_host(Host::start_polls));
    // Runs until asked to quit: the pill's window can be destroyed under it (Explorer restarts).
    let ended = slint::run_event_loop_until_quit();
    // The pill and its hooks go first, then the tray icons, then this host with the message window
    // the icons and the pill's menu belong to, here on the UI thread, before the thread's values
    // are torn down.
    pill_host::stop();
    tray_icons::stop();
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

/// Asks for a redraw: the host makes a snapshot and hands it to each part, now or a turn of the
/// event loop later if it is busy.
pub fn redraw() {
    with_host(Host::redraw);
}

/// Runs `step` on the host now, or a turn of the event loop later if it is busy.
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

fn on_broadcast(broadcast: Broadcast) {
    // A broadcast is sent to every top-level window, and its sender may wait on the answer. Work
    // done inside it (hiding and dropping the old pill, making a new one) can block on the
    // sender, so it runs a turn of the event loop later, once the message has been answered.
    Timer::single_shot(Duration::ZERO, move || {
        // The taskbar work first, so the pill is placed before it is drawn again.
        if matches!(
            broadcast,
            Broadcast::TaskbarCreated | Broadcast::DisplayChanged
        ) {
            pill_host::on_broadcast(broadcast);
        }
        if broadcast == Broadcast::TaskbarCreated {
            tray_icons::on_taskbar_created();
        }
        if matches!(
            broadcast,
            Broadcast::TaskbarCreated
                | Broadcast::SettingChanged
                | Broadcast::DisplayChanged
                | Broadcast::TimeChanged
        ) {
            redraw();
        }
    });
}

struct Host {
    places: Locations,
    diagnostics: Arc<Diagnostics>,
    feed: Feed,
    /// Kept to read its cache ahead of the first poll.
    copilot: Arc<CopilotProvider>,
    /// `config.json` could not be read at the last redraw, so it is told once.
    config_unreadable: bool,
    refresh_timer: Timer,
    /// The hidden window the broadcasts are heard on and the pill's menu is owned by; kept for as
    /// long as the host lives.
    _messages: Option<MessageWindow>,
}

impl Host {
    /// Switches both providers on, and reads Copilot's cache file ahead of the first poll.
    fn start_polls(&mut self) {
        let runs = self.feed.start(clock::now());
        self.spawn(runs);
        let copilot = self.copilot.clone();
        let read = std::thread::Builder::new()
            .name("copilot-cache".to_string())
            .spawn(move || {
                if let Some(cache) = copilot.read_cache() {
                    // As below: the loop exists, or has ended.
                    let _ = slint::invoke_from_event_loop(move || {
                        with_host(move |host| {
                            host.copilot.take_early(cache);
                            host.redraw();
                        });
                    });
                }
            });
        if read.is_err() {
            self.diagnostics
                .say("a thread for the Copilot cache could not start");
        }
    }

    /// Every 30 seconds: polls that are due start, and everything is drawn again.
    fn refresh(&mut self) {
        let runs = self.feed.tick(clock::now());
        self.spawn(runs);
        self.redraw();
    }

    /// Each poll runs on a thread of its own, which may wait ten seconds for the network, and
    /// its answer is handled on this thread again.
    fn spawn(&self, runs: Vec<Run>) {
        for Run { service, poll } in runs {
            let started = std::thread::Builder::new()
                .name("poll".to_string())
                .spawn(move || {
                    let finished = poll.run();
                    // Runs from the loop's first turn (see `run`), so the toolkit has its event
                    // loop and this fails only when the loop has ended: nothing is left to draw.
                    let _ = slint::invoke_from_event_loop(move || {
                        with_host(move |host| host.on_poll_done(service, finished));
                    });
                });
            if started.is_err() {
                self.diagnostics.say("a thread for a poll could not start");
            }
        }
    }

    fn on_poll_done(&mut self, service: Service, finished: Finished) {
        let handled = self.feed.done(service, finished, clock::now());
        // Copilot's provider writes its own line, with the cache word, through the trace.
        if service == Service::Claude
            && let Some(line) = handled.log
        {
            self.diagnostics.say(&line);
        }
        self.spawn(handled.runs);
        self.redraw();
    }

    /// Reads `config.json`, the theme and the rest of what is drawn from, makes the snapshot and
    /// hands it to each part. `config.json` is only read, and a file that cannot be read draws
    /// as the default.
    fn redraw(&mut self) {
        let now = clock::now();
        let (config, unreadable) = pill_feed::config_for_display(&self.places.app.config_file());
        if unreadable != self.config_unreadable {
            self.config_unreadable = unreadable;
            if unreadable {
                self.diagnostics
                    .say("config.json could not be read, so the defaults draw");
            }
        }
        let around = Surroundings {
            colours: registry::theme().colours(config.bar_colour),
            config,
            zone: local_zone(now),
            clock: locale::clock_format(),
            credentials_file_exists: self.places.claude.credentials_file().is_file(),
        };
        let settings = self.places.claude.settings_file();
        let bridge = carried_bridge::own(&self.places.app);
        let snapshot = Rc::new(self.feed.snapshot(now, around, || {
            let text = std::fs::read_to_string(settings).unwrap_or_default();
            connection::state(&text, bridge.as_deref())
        }));
        // Each part takes the snapshot on its own and answers for its own failures.
        let parts: [fn(Rc<Snapshot>); 2] = [pill_host::show, tray_icons::show];
        for part in parts {
            part(snapshot.clone());
        }
    }
}
