//! When the flyout is open and closed, and when it last closed because it lost the focus. Plain
//! values with time passed in, so a test drives it; the window is `window.rs`. The rules are 1.1's:
//! a click on the pill closes an open flyout and opens a closed one (the pill never takes the
//! focus, so the flyout is still open when that click lands); Esc closes; losing the focus closes
//! and, for `GRACE` after, says so, so a click on a tray icon that took the focus closes the flyout
//! instead of opening it again. The activation message alone has not always closed it, so a change
//! of the foreground window closes it too (`foreground_moved`), once it has had the focus; a change
//! that comes before the flyout was ever focused is a stale one and closes nothing.
//!
//! A select on a tray icon (`tray_click`) follows 1.1's other rule: open closes; closed opens,
//! unless it closed on focus loss less than `GRACE` ago (the click took the focus and that closed
//! it), then nothing. While a flyout a select opened is open, another select less than `REPEAT`
//! after that one is ignored: a key select Windows sends twice, or the second half of a double
//! click that lands before the flyout has the focus, would close what the first is opening.
use std::time::{Duration, Instant};

/// How long after a close on focus loss `closed_just_now` is true.
pub const GRACE: Duration = Duration::from_millis(300);

/// How long after a tray select opened the flyout another one is ignored while it is open.
pub const REPEAT: Duration = Duration::from_millis(500);

/// What a click on the pill is to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Open,
    Close,
}

#[derive(Debug, Default)]
pub struct Toggle {
    open: bool,
    /// Whether it has been given the focus since it opened.
    focused: bool,
    /// When it last closed on focus loss; none after any other close or open.
    lost_at: Option<Instant>,
    /// When a tray select last opened it.
    tray_opened_at: Option<Instant>,
}

impl Toggle {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// What a click on the pill does now. Not told apart by the grace: see the module docs.
    pub fn click(&self) -> Click {
        if self.open { Click::Close } else { Click::Open }
    }

    /// What a select on a tray icon at `now` does: none when it repeats the select that opened the
    /// flyout, or follows a close on focus loss within the grace. See the module docs.
    pub fn tray_click(&mut self, now: Instant) -> Option<Click> {
        if self.open {
            let repeat = self
                .tray_opened_at
                .is_some_and(|at| now.saturating_duration_since(at) < REPEAT);
            return (!repeat).then_some(Click::Close);
        }
        if self.closed_just_now(now) {
            return None;
        }
        self.tray_opened_at = Some(now);
        Some(Click::Open)
    }

    /// It is open now.
    pub fn opened(&mut self) {
        self.open = true;
        self.focused = false;
        self.lost_at = None;
    }

    /// It was given the focus. Only an open flyout notes it.
    pub fn focused(&mut self) {
        self.focused = self.open;
    }

    /// The foreground window changed at `now`. True when that closed the flyout, which then counts
    /// as a close on focus loss; false when it is closed or has not had the focus since it opened
    /// (the change may be an old one that arrived late).
    pub fn foreground_moved(&mut self, now: Instant) -> bool {
        self.focused && self.focus_lost(now)
    }

    /// It was closed by a pill click, Esc or a call; there is no grace. True when it was open. A
    /// close of a flyout that is closed already leaves the grace of an earlier focus loss alone.
    pub fn closed(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        self.focused = false;
        self.lost_at = None;
        true
    }

    /// It lost the focus at `now`. True when that closed it, false when it was not open (it was
    /// closed already and the focus went with the hiding).
    pub fn focus_lost(&mut self, now: Instant) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        self.focused = false;
        self.lost_at = Some(now);
        true
    }

    /// Whether it closed on focus loss less than `GRACE` before `now`.
    pub fn closed_just_now(&self, now: Instant) -> bool {
        self.lost_at
            .is_some_and(|lost| now.saturating_duration_since(lost) < GRACE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: Duration = Duration::from_millis(1);

    #[test]
    fn a_click_opens_a_closed_flyout_and_closes_an_open_one() {
        let mut toggle = Toggle::default();
        assert_eq!(toggle.click(), Click::Open);
        toggle.opened();
        assert!(toggle.is_open());
        assert_eq!(toggle.click(), Click::Close);
        assert!(toggle.closed());
        assert!(!toggle.is_open());
        assert_eq!(toggle.click(), Click::Open);
    }

    #[test]
    fn a_tray_click_closes_an_open_flyout() {
        let mut toggle = Toggle::default();
        toggle.opened();
        assert_eq!(toggle.tray_click(Instant::now()), Some(Click::Close));
    }

    #[test]
    fn a_tray_click_opens_a_closed_flyout() {
        let mut toggle = Toggle::default();
        assert_eq!(toggle.tray_click(Instant::now()), Some(Click::Open));
    }

    #[test]
    fn a_tray_click_within_the_grace_does_nothing() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert_eq!(toggle.tray_click(now + GRACE - STEP), None);
    }

    #[test]
    fn a_tray_click_at_the_grace_opens() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert_eq!(toggle.tray_click(now + GRACE), Some(Click::Open));
    }

    #[test]
    fn a_tray_click_after_the_grace_opens() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert_eq!(toggle.tray_click(now + GRACE + STEP), Some(Click::Open));
    }

    #[test]
    fn a_second_tray_select_inside_the_repeat_is_ignored() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        assert_eq!(toggle.tray_click(now), Some(Click::Open));
        toggle.opened();
        // Enter twice: the second must not close what the first opened.
        assert_eq!(toggle.tray_click(now + REPEAT - STEP), None);
        assert!(toggle.is_open());
    }

    #[test]
    fn a_tray_select_at_the_repeat_acts() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        assert_eq!(toggle.tray_click(now), Some(Click::Open));
        toggle.opened();
        assert_eq!(toggle.tray_click(now + REPEAT), Some(Click::Close));
    }

    #[test]
    fn an_ignored_repeat_does_not_move_the_window() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.tray_click(now);
        toggle.opened();
        assert_eq!(toggle.tray_click(now + REPEAT - STEP), None);
        // Counted from the select that was taken, not from the ignored one.
        assert_eq!(toggle.tray_click(now + REPEAT), Some(Click::Close));
    }

    #[test]
    fn a_select_after_the_grace_opens_inside_the_repeat() {
        let now = Instant::now();
        let lost = now + Duration::from_millis(100);
        let mut toggle = Toggle::default();
        assert_eq!(toggle.tray_click(now), Some(Click::Open));
        toggle.opened();
        // The next click on the tray takes the focus, which closes it; its select is swallowed.
        toggle.focus_lost(lost);
        assert_eq!(toggle.tray_click(lost + STEP), None);
        // Past the grace and closed: opens, though within REPEAT of the first select.
        assert_eq!(toggle.tray_click(lost + GRACE + STEP), Some(Click::Open));
    }

    #[test]
    fn a_select_that_closes_starts_no_repeat() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        assert_eq!(toggle.tray_click(now), Some(Click::Close));
        toggle.closed();
        assert_eq!(toggle.tray_click(now + STEP), Some(Click::Open));
    }

    #[test]
    fn a_tray_click_after_escape_opens_at_once() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.closed();
        assert_eq!(toggle.tray_click(now), Some(Click::Open));
    }

    #[test]
    fn a_click_right_after_a_focus_loss_still_opens() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert_eq!(toggle.click(), Click::Open);
    }

    #[test]
    fn escape_closes_with_no_grace() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        assert!(toggle.closed());
        assert!(!toggle.closed_just_now(now));
    }

    #[test]
    fn closing_on_a_pill_click_has_no_grace() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.closed();
        assert!(!toggle.closed_just_now(now + STEP));
    }

    #[test]
    fn focus_loss_closes_and_is_just_now_for_300_ms() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        assert!(toggle.focus_lost(now));
        assert!(!toggle.is_open());
        assert!(toggle.closed_just_now(now));
        assert!(toggle.closed_just_now(now + GRACE - STEP));
    }

    #[test]
    fn the_grace_ends_at_300_ms() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert!(!toggle.closed_just_now(now + GRACE));
        assert!(!toggle.closed_just_now(now + GRACE + STEP));
    }

    #[test]
    fn focus_loss_while_closed_changes_nothing() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        assert!(!toggle.focus_lost(now));
        assert!(!toggle.closed_just_now(now));
        toggle.opened();
        toggle.closed();
        // The hiding took the focus with it: no grace follows.
        assert!(!toggle.focus_lost(now));
        assert!(!toggle.closed_just_now(now));
    }

    #[test]
    fn opening_clears_the_grace() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        toggle.opened();
        assert!(!toggle.closed_just_now(now + STEP));
    }

    #[test]
    fn a_close_after_a_focus_loss_keeps_the_grace() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focus_lost(now);
        assert!(!toggle.closed());
        assert!(toggle.closed_just_now(now + STEP));
    }

    #[test]
    fn a_foreground_change_closes_a_focused_flyout() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focused();
        assert!(toggle.foreground_moved(now));
        assert!(!toggle.is_open());
    }

    #[test]
    fn a_foreground_change_before_the_first_focus_does_nothing() {
        let mut toggle = Toggle::default();
        toggle.opened();
        assert!(!toggle.foreground_moved(Instant::now()));
        assert!(toggle.is_open());
    }

    #[test]
    fn a_foreground_change_while_closed_does_nothing() {
        let mut toggle = Toggle::default();
        toggle.focused();
        assert!(!toggle.foreground_moved(Instant::now()));
        assert!(!toggle.is_open());
    }

    #[test]
    fn the_grace_follows_a_foreground_close() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focused();
        toggle.foreground_moved(now);
        assert!(toggle.closed_just_now(now + GRACE - STEP));
        assert!(!toggle.closed_just_now(now + GRACE));
    }

    #[test]
    fn reopening_clears_focused() {
        let now = Instant::now();
        let mut toggle = Toggle::default();
        toggle.opened();
        toggle.focused();
        toggle.opened();
        assert!(!toggle.foreground_moved(now));
        toggle.focused();
        assert!(toggle.closed());
        toggle.opened();
        assert!(!toggle.foreground_moved(now));
    }
}
