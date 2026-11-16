//! `--born <owner>`: shows a small window owned by the window with the given handle (a decimal
//! number), never activated. Once it is shown it prints its handle, then shows a second window the
//! same way, while the first is still up, and prints that one's. Both stay up for four seconds
//! after the second is shown. The born test reads the two lines and looks at the windows from
//! outside. An owner that is no window ends the run with code 2 and no window; a second window
//! that is not taken within the deadline, with code 3.
use crate::ui::BornTest;
use crate::win::born;
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::Cell;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

const UP: Duration = Duration::from_secs(4);
/// Both windows must be taken by then.
const DEADLINE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);

pub fn run(owner: Option<&str>) -> ExitCode {
    let Some(owner) = owner.and_then(|owner| owner.parse::<isize>().ok()) else {
        eprintln!("--born needs the owner window's handle, as a decimal number");
        return ExitCode::from(2);
    };
    if !born::is_window(owner) {
        eprintln!("--born: {owner} is not a window");
        return ExitCode::from(2);
    }
    match show(owner) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!("--born: both windows were not taken in time");
            ExitCode::from(3)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Gives back whether both windows were taken before the deadline.
fn show(owner: isize) -> Result<bool, String> {
    let first = BornTest::new().map_err(|error| error.to_string())?;
    born::show(first.window(), owner)?;

    let failed = Rc::new(Cell::new(None::<String>));
    let both = Rc::new(Cell::new(false));
    let poll = Timer::default();
    poll.start(TimerMode::Repeated, POLL, {
        let failed = failed.clone();
        let both = both.clone();
        let mut windows = vec![first];
        let mut said = 0;
        move || {
            let owned = born::owned_windows();
            if owned.len() <= said {
                return;
            }
            println!("{}", owned[said]);
            said += 1;
            // One window is made at a time: the second waits until the first is taken.
            if said == 1 {
                let second =
                    BornTest::new()
                        .map_err(|error| error.to_string())
                        .and_then(|window| {
                            born::show(window.window(), owner)?;
                            Ok(window)
                        });
                match second {
                    Ok(window) => windows.push(window),
                    Err(error) => {
                        failed.set(Some(error));
                        let _ = slint::quit_event_loop();
                    }
                }
            } else {
                both.set(true);
                Timer::single_shot(UP, || {
                    let _ = slint::quit_event_loop();
                });
            }
        }
    });
    Timer::single_shot(DEADLINE, {
        let both = both.clone();
        move || {
            if !both.get() {
                let _ = slint::quit_event_loop();
            }
        }
    });
    slint::run_event_loop_until_quit().map_err(|error| error.to_string())?;
    match failed.take() {
        Some(error) => Err(error),
        None => Ok(both.get()),
    }
}
