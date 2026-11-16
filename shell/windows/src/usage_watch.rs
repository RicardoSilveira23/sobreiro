//! Redraws at once when the bridge writes `usage.json`, instead of at the next 30 s refresh, as
//! 1.1 does. The watch runs on its own thread; changes are gathered for 250 ms, then the app host
//! redraws once on the UI thread.
use crate::app_host;
use crate::diagnostics::Diagnostics;
use crate::win::dir_watch::{DirWatch, Event, StopEvent};
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long the file must stay quiet before the redraw, as 1.1's debounce.
const QUIET: Duration = Duration::from_millis(250);

/// The file the bridge writes, by name: the data folder holds other files, the log among them.
const USAGE_FILE: &str = "usage.json";

/// Gathers changes into one redraw. It never reads a clock: the caller says what time it is.
struct Debounce {
    last: Option<Instant>,
}

impl Debounce {
    fn new() -> Debounce {
        Debounce { last: None }
    }

    /// A change at `now`. It restarts the quiet time.
    fn changed(&mut self, now: Instant) {
        self.last = Some(now);
    }

    /// True once, when the quiet time after the last change is over.
    fn due(&mut self, now: Instant) -> bool {
        match self.last {
            Some(last) if now.saturating_duration_since(last) >= QUIET => {
                self.last = None;
                true
            }
            _ => false,
        }
    }

    /// How long until `due` can be true; None with no change waiting.
    fn wait(&self, now: Instant) -> Option<Duration> {
        self.last
            .map(|last| (last + QUIET).saturating_duration_since(now))
    }
}

struct Running {
    stop: Arc<StopEvent>,
    thread: JoinHandle<()>,
}

thread_local! {
    static WATCH: RefCell<Option<Running>> = const { RefCell::new(None) };
}

/// Starts watching `data_directory`, which is made when it is missing. Call it from the event
/// loop's first turn: the redraws are handed back with `invoke_from_event_loop`. A watch that
/// cannot be set is logged once, and the periodic redraw carries on.
pub fn start(data_directory: &Path, diagnostics: Arc<Diagnostics>) {
    if std::fs::create_dir_all(data_directory).is_err() {
        diagnostics.say("the data folder could not be made for the usage watch");
        return;
    }
    let Some(stop) = StopEvent::new().map(Arc::new) else {
        diagnostics.say("the usage watch could not be set up");
        return;
    };
    let directory = data_directory.to_path_buf();
    let for_thread = stop.clone();
    let log = diagnostics.clone();
    let started = std::thread::Builder::new()
        .name("usage-watch".to_string())
        .spawn(move || run(&directory, &for_thread, &log));
    match started {
        Ok(thread) => {
            let _ = WATCH.try_with(|cell| cell.replace(Some(Running { stop, thread })));
        }
        Err(_) => diagnostics.say("a thread for the usage watch could not start"),
    }
}

/// Stops the watch and waits for its thread. Nothing to do when it never started.
pub fn stop() {
    let running = WATCH
        .try_with(|cell| {
            cell.try_borrow_mut()
                .ok()
                .and_then(|mut watch| watch.take())
        })
        .ok()
        .flatten();
    if let Some(running) = running {
        // A set that fails leaves the thread waiting; joining it would hang the quit.
        if running.stop.set() {
            let _ = running.thread.join();
        }
    }
}

/// The thread: waits for changes, debounces them, asks for a redraw.
fn run(directory: &Path, stop: &StopEvent, diagnostics: &Diagnostics) {
    let Some(mut watch) = DirWatch::open(directory, USAGE_FILE) else {
        diagnostics.say("the data folder could not be watched");
        return;
    };
    let mut debounce = Debounce::new();
    loop {
        match watch.wait(stop, debounce.wait(Instant::now())) {
            Event::Changed => debounce.changed(Instant::now()),
            Event::Other | Event::Timeout => {}
            Event::Stopped => return,
            Event::Failed => {
                diagnostics.say("the usage watch failed");
                return;
            }
        }
        if debounce.due(Instant::now()) && slint::invoke_from_event_loop(app_host::redraw).is_err()
        {
            // The loop has ended: nothing is left to redraw.
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many redraws the debounce asks for, given changes at these offsets and a check at
    /// each offset in `checks`, in order.
    fn redraws(changes: &[u64], checks: &[u64]) -> usize {
        let base = Instant::now();
        let at = |millis: u64| base + Duration::from_millis(millis);
        let mut debounce = Debounce::new();
        let mut events: Vec<(u64, bool)> = changes.iter().map(|&t| (t, true)).collect();
        events.extend(checks.iter().map(|&t| (t, false)));
        events.sort_by_key(|&(t, is_change)| (t, !is_change));
        let mut count = 0;
        for (time, is_change) in events {
            if is_change {
                debounce.changed(at(time));
            } else if debounce.due(at(time)) {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn changes_within_the_quiet_time_give_one_redraw() {
        // Each change restarts the wait, so the checks before the end of the last find nothing.
        assert_eq!(redraws(&[0, 100, 200], &[240, 449, 450, 700]), 1);
    }

    #[test]
    fn a_change_a_quiet_time_after_the_last_gives_a_second() {
        assert_eq!(redraws(&[0, 500], &[250, 750, 1000]), 2);
    }

    #[test]
    fn no_change_gives_no_redraw() {
        assert_eq!(redraws(&[], &[0, 250, 1000]), 0);
        assert_eq!(Debounce::new().wait(Instant::now()), None);
    }

    #[test]
    fn the_wait_is_what_is_left_of_the_quiet_time() {
        let base = Instant::now();
        let mut debounce = Debounce::new();
        debounce.changed(base);
        assert_eq!(
            debounce.wait(base + Duration::from_millis(100)),
            Some(Duration::from_millis(150))
        );
        assert_eq!(
            debounce.wait(base + Duration::from_millis(400)),
            Some(Duration::ZERO)
        );
    }
}
