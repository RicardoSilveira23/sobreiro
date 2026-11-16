//! `--born <owner>`: shows a small window owned by the window with the given handle (a decimal
//! number), never activated, and stands for the pill. Once it is shown it prints its handle, then
//! makes the pill's tooltip window the way the shell does (`tooltip_window`, through `born`), while
//! the first is still up, and prints that one's. The tooltip's window is hidden when it is taken.
//! Three times it is then shown above the other windows, as the shell opens it, and hidden again,
//! printing "showing" before each show and "shown" after it, "hiding" before each hide and
//! "hidden" after it. Both windows
//! stay up for four seconds after the second is taken. The born test reads the lines and looks at
//! the windows from outside. An owner that is no window ends the run with code 2 and no window; a
//! second window that is not taken within the deadline, with code 3.
use crate::tooltip_window::TooltipWindow;
use crate::ui::BornTest;
use crate::win::{born, taskbar};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::Cell;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

const UP: Duration = Duration::from_secs(4);
/// Both windows must be taken by then.
const DEADLINE: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(10);
/// How long the tooltip's window is shown, and hidden, in each of its three turns.
const TURN: Duration = Duration::from_millis(300);
const TURNS: u32 = 3;
/// Where the tooltip is shown, in pixels: no taskbar here, so any place on the screen.
const AT: (i32, i32) = (500, 500);
const WORDS: &str = "Session 15%
Weekly 45%
as of 11:59 AM";

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

/// The two windows and how far the run has come.
struct Run {
    /// Stands for the pill; held so its window stays up.
    _first: BornTest,
    tooltip: Option<TooltipWindow>,
    said_first: bool,
    said_tooltip: bool,
}

/// Gives back whether both windows were taken before the deadline.
fn show(owner: isize) -> Result<bool, String> {
    let first = BornTest::new().map_err(|error| error.to_string())?;
    born::show(first.window(), owner)?;

    let failed = Rc::new(Cell::new(None::<String>));
    let both = Rc::new(Cell::new(false));
    let poll = Timer::default();
    let turns = Rc::new(Timer::default());
    poll.start(TimerMode::Repeated, POLL, {
        let failed = failed.clone();
        let both = both.clone();
        let turns = turns.clone();
        let mut run = Run {
            _first: first,
            tooltip: None,
            said_first: false,
            said_tooltip: false,
        };
        move || {
            if run.said_tooltip {
                return;
            }
            if !run.said_first {
                let owned = born::owned_windows();
                let Some(window) = owned.first() else {
                    return;
                };
                println!("{window}");
                run.said_first = true;
                // One window is made at a time: the tooltip's waits until the first is taken.
                match TooltipWindow::create(owner) {
                    Ok(made) => {
                        made.set_text(WORDS);
                        run.tooltip = Some(made);
                    }
                    Err(error) => {
                        failed.set(Some(error));
                        let _ = slint::quit_event_loop();
                    }
                }
                return;
            }
            let Some(made) = run.tooltip.as_mut() else {
                return;
            };
            made.find();
            let Some(window) = made.window() else {
                return;
            };
            println!("{window}");
            run.said_tooltip = true;
            both.set(true);
            start_turns(&turns, window);
            Timer::single_shot(UP, || {
                let _ = slint::quit_event_loop();
            });
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

/// Shows the tooltip's window and hides it again, three times, as the shell does: raised with
/// Windows' own call, which leaves its owner where it is, and hidden with Windows' own call.
fn start_turns(timer: &Timer, window: isize) {
    let left = Cell::new(TURNS * 2);
    let shown = Cell::new(false);
    timer.start(TimerMode::Repeated, TURN, move || {
        if left.get() == 0 {
            return;
        }
        left.set(left.get() - 1);
        if shown.get() {
            println!("hiding");
            taskbar::hide(window);
            println!("hidden");
        } else {
            println!("showing");
            taskbar::raise_to(window, AT.0, AT.1);
            println!("shown");
        }
        shown.set(!shown.get());
    });
}
