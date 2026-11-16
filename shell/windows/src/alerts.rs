//! The toasts at 80% and 95% of a limit, once per reset period, from the tray icon of the
//! service the limit belongs to: Claude Code's from its icon and Copilot's credits from its own.
//!
//! On every redraw Core's rule runs on the limits shown, whether or not the pill is. The state
//! (`alerts.json`, read as 1.x wrote it) is loaded at start and kept in memory before it is saved,
//! so a save that fails never makes a toast come back every 30 seconds. The data step below
//! touches no window and writes no file but `alerts.json`; the functions after it keep it in a
//! value of the UI thread and hand its toasts to the tray icons.
use crate::diagnostics::Diagnostics;
use crate::pill_feed::{Service, Snapshot};
use crate::tray_icons;
use chrono::{DateTime, Utc};
use sobreiro_core::alerts::alert_balloon::for_alerts;
use sobreiro_core::alerts::alert_policy::{Alert, check};
use sobreiro_core::alerts::alert_state::AlertState;
use sobreiro_core::display::{ClockFormat, PaceDays, Zone};
use sobreiro_core::limits::{Limit, LimitKind};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

thread_local! {
    static ALERTS: RefCell<Option<Running>> = const { RefCell::new(None) };
}

struct Running {
    step: Step,
    diagnostics: Arc<Diagnostics>,
}

/// One toast to show: from which service's icon, and its title and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub service: Service,
    pub title: String,
    pub text: String,
}

/// What a step looks at: the limits of both services, and what the texts are made from.
pub struct Figures<'a> {
    pub claude: &'a [Limit],
    pub copilot: &'a [Limit],
    /// Whether Claude Code's limits are shown; if not they raise no toast.
    pub show_claude: bool,
    pub now: DateTime<Utc>,
    pub zone: &'a Zone,
    pub clock: &'a ClockFormat,
    pub days: PaceDays,
}

/// The state of the alerts and the file it is kept in.
pub struct Step {
    state: AlertState,
    path: PathBuf,
    /// The last save failed, and none has worked since: told once.
    told_save: bool,
}

impl Step {
    /// Loads the state from the file; a missing or unreadable file gives an empty one.
    pub fn load(path: PathBuf) -> Step {
        Step {
            state: AlertState::load(&path),
            path,
            told_save: false,
        }
    }

    /// Runs Core's rule on the limits shown and returns the toasts: at most one for each service.
    /// A changed state is kept first and saved after; a failed save is told once through `say`,
    /// with the file's name and nothing else.
    pub fn run(&mut self, figures: &Figures, say: &dyn Fn(&str)) -> Vec<Toast> {
        let mut shown: Vec<Limit> = Vec::new();
        if figures.show_claude {
            shown.extend_from_slice(figures.claude);
        }
        shown.extend_from_slice(figures.copilot);
        let (alerts, next) = check(
            &shown,
            &self.state,
            figures.now,
            figures.zone,
            figures.clock,
            figures.days,
        );
        let Some(next) = next else {
            return Vec::new();
        };
        self.state = next;
        match self.state.save(&self.path) {
            Ok(()) => self.told_save = false,
            Err(_) => {
                if !self.told_save {
                    self.told_save = true;
                    let name = self
                        .path
                        .file_name()
                        .map_or_else(|| "alerts.json".into(), |n| n.to_string_lossy());
                    say(&format!("{name} could not be saved"));
                }
            }
        }
        toasts(alerts)
    }
}

/// The alerts of Copilot's credits make one toast and every other kind's another.
fn toasts(alerts: Vec<Alert>) -> Vec<Toast> {
    let (copilot, claude): (Vec<Alert>, Vec<Alert>) = alerts
        .into_iter()
        .partition(|alert| alert.kind == LimitKind::Credits);
    [(Service::Claude, claude), (Service::Copilot, copilot)]
        .into_iter()
        .filter_map(|(service, alerts)| {
            let (title, text) = for_alerts(&alerts)?;
            Some(Toast {
                service,
                title,
                text,
            })
        })
        .collect()
}

/// Loads the state from `path`. Called once, on the UI thread, before the first redraw.
pub fn start(path: PathBuf, diagnostics: Arc<Diagnostics>) {
    let running = Running {
        step: Step::load(path),
        diagnostics,
    };
    let _ = ALERTS.try_with(|cell| cell.replace(Some(running)));
}

/// Takes a redraw's snapshot, and shows the toasts it raises from the tray icons.
pub fn show(snapshot: Rc<Snapshot>) {
    let figures = Figures {
        claude: &snapshot.claude,
        copilot: &snapshot.copilot,
        show_claude: snapshot.show_claude,
        now: snapshot.now,
        zone: &snapshot.zone,
        clock: &snapshot.clock,
        days: snapshot.config.pace_days,
    };
    // The balloons are shown after the state is let go of.
    let toasts = ALERTS
        .try_with(|cell| {
            let mut cell = cell.try_borrow_mut().ok()?;
            let running = cell.as_mut()?;
            let diagnostics = running.diagnostics.clone();
            Some(running.step.run(&figures, &|line| diagnostics.say(line)))
        })
        .ok()
        .flatten()
        .unwrap_or_default();
    for toast in toasts {
        tray_icons::balloon(toast.service, &toast.title, &toast.text);
    }
}

#[cfg(test)]
mod tests {
    // A test fails by panicking, in the helpers below as in the tests themselves.
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use chrono::TimeDelta;
    use sobreiro_core::limits::providers;
    use std::cell::RefCell;
    use std::path::Path;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-16T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn resets() -> DateTime<Utc> {
        now() + TimeDelta::hours(2)
    }

    fn clock() -> ClockFormat {
        ClockFormat {
            pattern: "H:mm".to_string(),
            am: "AM".to_string(),
            pm: "PM".to_string(),
        }
    }

    fn session(percent: f64, resets_at: DateTime<Utc>) -> Vec<Limit> {
        vec![Limit::new(
            providers::CLAUDE,
            LimitKind::Session,
            "Session",
            percent,
            Some(resets_at),
            now(),
            false,
        )]
    }

    fn credits(percent: f64, resets_at: DateTime<Utc>) -> Vec<Limit> {
        vec![Limit::new(
            providers::COPILOT,
            LimitKind::Credits,
            "AI credits",
            percent,
            Some(resets_at),
            now(),
            false,
        )]
    }

    /// A folder of the test's own, empty.
    fn folder(name: &str) -> PathBuf {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/tmp")
            .join(format!("alerts-{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn run(step: &mut Step, claude: &[Limit], copilot: &[Limit], show_claude: bool) -> Vec<Toast> {
        let zone = Zone::utc();
        let clock = clock();
        let figures = Figures {
            claude,
            copilot,
            show_claude,
            now: now(),
            zone: &zone,
            clock: &clock,
            days: PaceDays::EveryDay,
        };
        step.run(&figures, &|_| {})
    }

    #[test]
    fn claude_toasts_come_once_at_80_and_once_at_95() {
        let path = folder("claude").join("alerts.json");
        let mut step = Step::load(path);
        assert!(run(&mut step, &session(79.0, resets()), &[], true).is_empty());
        let at_80 = run(&mut step, &session(82.0, resets()), &[], true);
        assert_eq!(at_80.len(), 1);
        assert_eq!(at_80[0].service, Service::Claude);
        assert!(at_80[0].title.contains("82%"), "{}", at_80[0].title);
        assert!(run(&mut step, &session(83.0, resets()), &[], true).is_empty());
        let at_95 = run(&mut step, &session(96.0, resets()), &[], true);
        assert_eq!(at_95.len(), 1);
        assert!(at_95[0].title.contains("96%"), "{}", at_95[0].title);
        assert!(run(&mut step, &session(97.0, resets()), &[], true).is_empty());
    }

    #[test]
    fn copilot_toasts_come_from_the_copilot_icon() {
        let path = folder("copilot").join("alerts.json");
        let mut step = Step::load(path);
        assert!(run(&mut step, &[], &credits(79.0, resets()), true).is_empty());
        let at_80 = run(&mut step, &[], &credits(82.0, resets()), true);
        assert_eq!(at_80.len(), 1);
        assert_eq!(at_80[0].service, Service::Copilot);
        assert!(at_80[0].title.starts_with("Copilot "), "{}", at_80[0].title);
        let at_95 = run(&mut step, &[], &credits(96.0, resets()), true);
        assert_eq!(at_95.len(), 1);
        assert_eq!(at_95[0].service, Service::Copilot);
    }

    #[test]
    fn each_service_has_its_own_toast() {
        let path = folder("both").join("alerts.json");
        let mut step = Step::load(path);
        let toasts = run(
            &mut step,
            &session(82.0, resets()),
            &credits(82.0, resets()),
            true,
        );
        let services: Vec<Service> = toasts.iter().map(|t| t.service).collect();
        assert_eq!(services, [Service::Claude, Service::Copilot]);
    }

    #[test]
    fn a_restart_with_the_same_file_asks_for_no_toast() {
        let path = folder("restart").join("alerts.json");
        let mut step = Step::load(path.clone());
        assert_eq!(run(&mut step, &session(96.0, resets()), &[], true).len(), 1);
        let mut again = Step::load(path);
        assert!(run(&mut again, &session(96.0, resets()), &[], true).is_empty());
    }

    #[test]
    fn a_new_period_raises_the_toast_again() {
        let path = folder("reset").join("alerts.json");
        let mut step = Step::load(path);
        assert_eq!(run(&mut step, &session(96.0, resets()), &[], true).len(), 1);
        let later = resets() + TimeDelta::hours(5);
        let toasts = run(&mut step, &session(82.0, later), &[], true);
        assert_eq!(toasts.len(), 1);
        assert!(toasts[0].title.contains("82%"), "{}", toasts[0].title);
    }

    #[test]
    fn claude_is_ignored_when_it_is_not_shown() {
        let path = folder("hidden").join("alerts.json");
        let mut step = Step::load(path);
        assert!(run(&mut step, &session(96.0, resets()), &[], false).is_empty());
        let toasts = run(
            &mut step,
            &session(96.0, resets()),
            &credits(96.0, resets()),
            false,
        );
        assert_eq!(toasts.len(), 1);
        assert_eq!(toasts[0].service, Service::Copilot);
    }

    #[test]
    fn a_file_written_by_1x_is_read() {
        let path = folder("one-x").join("alerts.json");
        let file = format!(
            "{{\r\n  \"session\": {{\r\n    \"resets_at\": {},\r\n    \"threshold\": 80\r\n  }}\r\n}}",
            resets().timestamp()
        );
        std::fs::write(&path, file).unwrap();
        let mut step = Step::load(path);
        assert!(run(&mut step, &session(82.0, resets()), &[], true).is_empty());
        let toasts = run(&mut step, &session(96.0, resets()), &[], true);
        assert_eq!(toasts.len(), 1);
        assert!(toasts[0].title.contains("96%"), "{}", toasts[0].title);
    }

    #[test]
    fn a_failed_save_keeps_the_state_and_is_told_once() {
        // The save makes its folder, so the folder's place is taken by a file.
        let blocker = folder("unsaved").join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let path = blocker.join("alerts.json");
        let mut step = Step::load(path);
        let lines = RefCell::new(Vec::new());
        let say = |line: &str| lines.borrow_mut().push(line.to_string());
        let zone = Zone::utc();
        let clock = clock();
        let limits = session(82.0, resets());
        let figures = Figures {
            claude: &limits,
            copilot: &[],
            show_claude: true,
            now: now(),
            zone: &zone,
            clock: &clock,
            days: PaceDays::EveryDay,
        };
        assert_eq!(step.run(&figures, &say).len(), 1);
        assert!(step.run(&figures, &say).is_empty());
        let higher = session(96.0, resets());
        let figures = Figures {
            claude: &higher,
            ..figures
        };
        assert_eq!(step.run(&figures, &say).len(), 1);
        let lines = lines.into_inner();
        assert_eq!(lines, ["alerts.json could not be saved"]);
    }
}
