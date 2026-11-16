//! When the flyout is open and closed, and when it last closed because it lost the focus. Plain
//! values with time passed in, so a test drives it; the window is `window.rs`. The rules are 1.1's:
//! a click on the pill closes an open flyout and opens a closed one (the pill never takes the
//! focus, so the flyout is still open when that click lands); Esc closes; losing the focus closes
//! and, for `GRACE` after, says so, so a click on a tray icon that took the focus closes the flyout
//! instead of opening it again.
use std::time::{Duration, Instant};

/// How long after a close on focus loss `closed_just_now` is true.
pub const GRACE: Duration = Duration::from_millis(300);

/// What a click on the pill is to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Open,
    Close,
}

#[derive(Debug, Default)]
pub struct Toggle {
    open: bool,
    /// When it last closed on focus loss; none after any other close or open.
    lost_at: Option<Instant>,
}

impl Toggle {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// What a click on the pill does now. Not told apart by the grace: see the module docs.
    pub fn click(&self) -> Click {
        if self.open { Click::Close } else { Click::Open }
    }

    /// It is open now.
    pub fn opened(&mut self) {
        self.open = true;
        self.lost_at = None;
    }

    /// It was closed by a pill click, Esc or a call; there is no grace. True when it was open. A
    /// close of a flyout that is closed already leaves the grace of an earlier focus loss alone.
    pub fn closed(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
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
}
