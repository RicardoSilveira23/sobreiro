//! One copy per data folder. A launch that finds another copy running asks it to show its details
//! and ends; the first copy waits for that ask off the UI thread.
use crate::diagnostics::Diagnostics;
use crate::providers::Locations;
use crate::win::named_objects::{self, NamedEvent, NamedMutex, Taken};
use slint::Timer;
use std::sync::Arc;
use std::time::Duration;

/// What this launch is.
pub enum Start {
    /// Another copy runs on this data folder and has been asked to show its details.
    Second,
    /// This launch runs; the guard keeps the place until it is dropped.
    First(Guard),
}

/// The first copy's place: the mutex, and the event the others ask on.
pub struct Guard {
    // Held for its drop: the mutex is released when the guard is.
    _mutex: Option<NamedMutex>,
    event: Option<NamedEvent>,
    diagnostics: Option<Arc<Diagnostics>>,
}

/// The mutex's name. 1.x's, so that a copy of it still running is found.
fn mutex_name(id: &str) -> String {
    format!("Local\\{id}.Tray")
}

/// The show event's name. 1.x's, as above.
fn event_name(id: &str) -> String {
    format!("Local\\{id}.Show")
}

/// Takes the place of the first copy, or asks the first copy to show and says so.
pub fn start() -> Start {
    let Some(places) = Locations::find() else {
        // `app_host::run` fails right after on its own.
        return Start::First(Guard {
            _mutex: None,
            event: None,
            diagnostics: None,
        });
    };
    let diagnostics = Arc::new(Diagnostics::new(&places.app.log_file()));
    let id = places.app.instance_id();
    // The event first, so it exists whenever a first copy does: a launch that comes while the first
    // is still starting leaves it set, and the first shows once it is ready.
    let event = NamedEvent::create(&event_name(&id));
    if event.is_none() {
        diagnostics.say("show-event-failed");
    }
    match NamedMutex::take(&mutex_name(&id)) {
        Taken::Held => {
            named_objects::allow_any_foreground();
            if let Some(event) = &event
                && !event.set()
            {
                diagnostics.say("show-request-failed");
            }
            drop(event);
            diagnostics.say("already-running");
            Start::Second
        }
        Taken::Failed => {
            diagnostics.say("one-copy-failed");
            Start::First(Guard {
                _mutex: None,
                event,
                diagnostics: Some(diagnostics),
            })
        }
        Taken::First(mutex) => Start::First(Guard {
            _mutex: Some(mutex),
            event,
            diagnostics: Some(diagnostics),
        }),
    }
}

impl Guard {
    /// Starts waiting for the ask to show. The thread starts on the event loop's first turn: until
    /// the toolkit's backend exists (`app_host::run` makes it), `invoke_from_event_loop` has no
    /// loop to answer to. Timers need no backend, so one set now fires then.
    pub fn listen(&mut self) {
        let Some(event) = self.event.take() else {
            return;
        };
        let diagnostics = self.diagnostics.clone();
        Timer::single_shot(Duration::ZERO, move || {
            let started = std::thread::Builder::new()
                .name("show-wait".to_string())
                .spawn(move || {
                    while event.wait() {
                        // An Err means the loop has ended: nothing is left to show.
                        if slint::invoke_from_event_loop(show_by_tray).is_err() {
                            break;
                        }
                    }
                });
            if started.is_err()
                && let Some(diagnostics) = diagnostics
            {
                diagnostics.say("the show wait could not start");
            }
        });
    }
}

/// Opens the details by the tray (the bottom-right corner of the primary monitor's work area), as
/// 1.1's `ShowFlyoutByTray`. The flyout's window is not in the shell yet, and until it is the ask
/// is heard and nothing opens.
fn show_by_tray() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_1x() {
        assert_eq!(mutex_name("Sobreiro"), "Local\\Sobreiro.Tray");
        assert_eq!(event_name("Sobreiro"), "Local\\Sobreiro.Show");
        assert_eq!(
            mutex_name("Sobreiro.0a1b2c3d"),
            "Local\\Sobreiro.0a1b2c3d.Tray"
        );
    }
}
