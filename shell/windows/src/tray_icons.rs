//! The tray icons, one for Claude Code and one for Copilot: drawn from each redraw's snapshot,
//! added and removed as Core's rule says, added again when Explorer restarts, and removed when the
//! app quits. A click on one is handed on a turn of the event loop later. A balloon can be shown
//! from either one.
//!
//! An icon that cannot be drawn, or whose change the shell refuses, keeps its last good picture
//! and tooltip and is tried again at the next redraw. The state lives in a value of the UI thread
//! and is used through the functions below.
use crate::diagnostics::Diagnostics;
use crate::pill_feed::{Service, Snapshot};
use crate::pill_host;
use crate::win::notify_icon::{self, CLAUDE_ID, COPILOT_ID, TrayEvent};
use crate::win::taskbar;
use crate::win::tray_picture::{self, TrayIcon};
use slint::Timer;
use sobreiro_core::colours::TaskbarColours;
use sobreiro_core::display::rings::RingsModel;
use sobreiro_core::display::tooltip_text::{render, render_copilot};
use sobreiro_core::taskbar::tray_icons::visibility;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// Used when Windows gives no size.
const FALLBACK_SIZE: u32 = 16;

thread_local! {
    static TRAY: RefCell<Option<Tray>> = const { RefCell::new(None) };
}

/// What an icon is drawn from, kept to tell when it has to be drawn again.
#[derive(Clone, PartialEq, Eq)]
struct Source {
    model: RingsModel,
    colours: TaskbarColours,
    size: u32,
}

/// A picture and tooltip made from a source.
struct Made {
    icon: TrayIcon,
    tip: String,
    source: Source,
}

impl Made {
    fn is(&self, source: &Source, tip: &str) -> bool {
        self.source == *source && self.tip == tip
    }
}

struct Slot {
    id: u32,
    name: &'static str,
    /// Whether the shell has the icon, as far as is known.
    added: bool,
    /// The last good picture and tooltip: what the shell shows once added.
    current: Option<Made>,
    /// A picture could not be made, and none has been made since: told once.
    told_draw: bool,
    /// The shell refused an add or a change, and has taken none since: told once.
    told_shell: bool,
    /// Why the last balloon was let go; None once one is shown.
    told_balloon: Option<LetGo>,
}

/// Why a balloon was let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LetGo {
    /// The icon is hidden, or not added yet.
    Hidden,
    /// The shell refused the balloon.
    Refused,
}

impl LetGo {
    fn why(self) -> &'static str {
        match self {
            LetGo::Hidden => "is hidden or not there yet",
            LetGo::Refused => "was refused by the shell",
        }
    }
}

/// What a balloon's outcome has to tell: the reason it was let go, unless the last balloon was
/// let go for the same one. So each reason is told once until a balloon is shown.
fn to_tell(told: &mut Option<LetGo>, outcome: Option<LetGo>) -> Option<LetGo> {
    let tell = outcome.filter(|why| *told != Some(*why));
    *told = outcome;
    tell
}

impl Slot {
    fn new(id: u32, name: &'static str) -> Slot {
        Slot {
            id,
            name,
            added: false,
            current: None,
            told_draw: false,
            told_shell: false,
            told_balloon: None,
        }
    }

    /// Draws the icon again when its source or tooltip changed. None when nothing changed or the
    /// picture could not be made (told once).
    fn prepare(&mut self, diagnostics: &Diagnostics, source: Source, tip: String) -> Option<Made> {
        if self
            .current
            .as_ref()
            .is_some_and(|made| made.is(&source, &tip))
        {
            return None;
        }
        match tray_picture::icon(&source.model, &source.colours, source.size) {
            Ok(icon) => {
                self.told_draw = false;
                Some(Made { icon, tip, source })
            }
            Err(_) => {
                if !self.told_draw {
                    self.told_draw = true;
                    diagnostics.say(&format!("the {} tray icon could not be drawn", self.name));
                }
                None
            }
        }
    }

    /// Puts the shell's icon right: removed when it is not to be shown, added when it is not there
    /// yet, changed when something new was made.
    fn apply(
        &mut self,
        window: isize,
        diagnostics: &Diagnostics,
        shown: bool,
        candidate: Option<Made>,
    ) {
        if !shown {
            if self.added {
                notify_icon::delete(window, self.id);
                self.added = false;
            }
            return;
        }
        // None when the shell was not asked anything.
        let taken = if self.added {
            candidate.map(|made| {
                let changed = notify_icon::modify(window, self.id, &made.icon, &made.tip);
                // Refused, the new picture is dropped and the last good one stays. The icon may
                // be gone, so the next redraw adds it again (an add falls back to a modify).
                if changed {
                    self.current = Some(made);
                } else {
                    self.added = false;
                }
                changed
            })
        } else {
            if candidate.is_some() {
                self.current = candidate;
            }
            // Nothing is added before a picture exists.
            self.current.as_ref().map(|made| {
                self.added = notify_icon::add(window, self.id, &made.icon, &made.tip);
                self.added
            })
        };
        match taken {
            Some(true) => self.told_shell = false,
            Some(false) if !self.told_shell => {
                self.told_shell = true;
                diagnostics.say(&format!(
                    "the {} tray icon could not be added or changed",
                    self.name
                ));
            }
            _ => {}
        }
    }
}

struct Tray {
    window: isize,
    diagnostics: Arc<Diagnostics>,
    claude: Slot,
    copilot: Slot,
}

/// Runs `step` on the icons now, or a turn of the event loop later if they are busy.
fn with_tray(step: impl FnOnce(&mut Tray) + 'static) {
    let busy = TRAY.try_with(|cell| match cell.try_borrow_mut() {
        Ok(mut tray) => {
            if let Some(tray) = tray.as_mut() {
                step(tray);
            }
            None
        }
        Err(_) => Some(step),
    });
    if let Ok(Some(step)) = busy {
        Timer::single_shot(Duration::ZERO, move || with_tray(step));
    }
}

/// Starts the icons on the message window. Without one nothing is added.
pub fn start(messages: Option<isize>, diagnostics: Arc<Diagnostics>) {
    let Some(window) = messages else {
        return;
    };
    let tray = Tray {
        window,
        diagnostics,
        claude: Slot::new(CLAUDE_ID, "Claude Code"),
        copilot: Slot::new(COPILOT_ID, "Copilot"),
    };
    let _ = TRAY.try_with(|cell| cell.replace(Some(tray)));
}

/// Draws the icons from the snapshot, in 1.1's order: Copilot's picture, which icons are shown,
/// Claude's picture, then the shell's icons.
pub fn show(snapshot: Rc<Snapshot>) {
    with_tray(move |tray| tray.show(&snapshot));
}

/// Explorer made a new taskbar, and the icons are gone from it: the next redraw adds them again.
pub fn on_taskbar_created() {
    with_tray(|tray| {
        tray.claude.added = false;
        tray.copilot.added = false;
    });
}

/// Removes both icons, while the window they belong to is alive. Both are asked, added or not:
/// an add that timed out may have left one.
pub fn stop() {
    let tray = TRAY
        .try_with(|cell| cell.try_borrow_mut().ok().and_then(|mut tray| tray.take()))
        .ok()
        .flatten();
    if let Some(tray) = tray {
        for slot in [&tray.claude, &tray.copilot] {
            notify_icon::delete(tray.window, slot.id);
        }
    }
}

/// Shows a balloon from the service's icon, as 1.1 does. An icon that is hidden or not there yet,
/// or a balloon the shell refuses, is let go and told once. The icon's state is not touched, so the
/// redraw goes on as before. Called on the UI thread; elsewhere it does nothing.
pub fn balloon(service: Service, title: &str, text: &str) {
    let (title, text) = (title.to_owned(), text.to_owned());
    with_tray(move |tray| {
        let window = tray.window;
        let slot = match service {
            Service::Claude => &mut tray.claude,
            Service::Copilot => &mut tray.copilot,
        };
        let outcome = if !slot.added {
            Some(LetGo::Hidden)
        } else if notify_icon::balloon(window, slot.id, &title, &text) {
            None
        } else {
            Some(LetGo::Refused)
        };
        if let Some(why) = to_tell(&mut slot.told_balloon, outcome) {
            tray.diagnostics.say(&format!(
                "a balloon from the {} tray icon {}",
                slot.name,
                why.why()
            ));
        }
    });
}

/// A click on an icon. Run a turn of the event loop later, not inside the window procedure.
pub fn on_event(event: TrayEvent) {
    Timer::single_shot(Duration::ZERO, move || match event.event {
        notify_icon::SELECT | notify_icon::NIN_KEYSELECT => flyout_hook(event.x, event.y),
        notify_icon::CONTEXT_MENU => pill_host::open_menu_at_pointer(),
        _ => {}
    });
}

/// #37 opens the flyout at the pointer here, or closes it when it is open.
fn flyout_hook(_x: i32, _y: i32) {}

impl Tray {
    fn show(&mut self, snapshot: &Snapshot) {
        let size = match taskbar::small_icon_size() {
            0 => FALLBACK_SIZE,
            size => size,
        };
        let source = |model: RingsModel| Source {
            model,
            colours: snapshot.colours,
            size,
        };
        let now = snapshot.now;
        let (zone, clock) = (&snapshot.zone, &snapshot.clock);

        let copilot = snapshot.copilot.first().and_then(|credits| {
            let tip = render_copilot(credits, now, zone, clock);
            let model = RingsModel::copilot(&snapshot.copilot);
            self.copilot.prepare(&self.diagnostics, source(model), tip)
        });
        let shown = visibility(
            snapshot.show_claude,
            !snapshot.copilot.is_empty(),
            copilot.is_some() || self.copilot.current.is_some(),
        );
        let claude = if shown.claude {
            let tip = render(&snapshot.claude, now, zone, clock);
            let model = RingsModel::from(&snapshot.claude);
            self.claude.prepare(&self.diagnostics, source(model), tip)
        } else {
            None
        };
        self.copilot
            .apply(self.window, &self.diagnostics, shown.copilot, copilot);
        self.claude
            .apply(self.window, &self.diagnostics, shown.claude, claude);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_is_told_once_until_a_balloon_is_shown() {
        let mut told = None;
        assert_eq!(to_tell(&mut told, Some(LetGo::Hidden)), Some(LetGo::Hidden));
        assert_eq!(to_tell(&mut told, Some(LetGo::Hidden)), None);
        assert_eq!(to_tell(&mut told, None), None);
        assert_eq!(to_tell(&mut told, Some(LetGo::Hidden)), Some(LetGo::Hidden));
    }

    #[test]
    fn a_new_reason_is_told() {
        let mut told = None;
        assert_eq!(to_tell(&mut told, Some(LetGo::Hidden)), Some(LetGo::Hidden));
        assert_eq!(
            to_tell(&mut told, Some(LetGo::Refused)),
            Some(LetGo::Refused)
        );
        assert_eq!(to_tell(&mut told, Some(LetGo::Refused)), None);
    }

    #[test]
    fn a_shown_balloon_tells_nothing() {
        let mut told = None;
        assert_eq!(to_tell(&mut told, None), None);
        assert_eq!(told, None);
    }
}
