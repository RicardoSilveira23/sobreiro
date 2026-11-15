//! Decides whether a pill tick also asks UI Automation where the taskbar's parts are.
//!
//! A moment here is a span since some start the shell counts from (the C# `TimeSpan` of a
//! stopwatch), not a date: the schedule never asks a clock.
use chrono::TimeDelta;

use crate::taskbar::taskbar_layout::TaskbarAnchors;

/// Questions a foreground switch owes: the first can land before the new window's button or tray
/// icon exists.
pub const ASKS_PER_SWITCH: i32 = 2;

/// Questions (one minute of 2-second ticks) after which missing answers may settle, when no
/// anchors were ever seen.
pub const ASKS_BEFORE_ANCHORS: i32 = 30;

/// How long a settled schedule goes without asking: layout changes that raise no foreground
/// switch are noticed within about 16 seconds (15, landing on a 2-second tick).
pub const SLOW_CHECK: TimeDelta = TimeDelta::seconds(15);

/// An immutable value: each step returns a new schedule. The default is the fresh schedule:
/// nothing asked, nothing answered.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TaskbarQuerySchedule {
    has_asked: bool,
    last_asked: TimeDelta,
    switches_owed: i32,
    unsettled: bool,
    has_answer: bool,
    last_answer: Option<TaskbarAnchors>,
    anchors_seen: bool,
    ask_count: i32,
}

impl TaskbarQuerySchedule {
    /// True when nothing has been asked yet, a foreground switch still owes questions (two
    /// each), the last answers have not agreed yet, or `SLOW_CHECK` has passed since the last
    /// question. A question whose answer never comes is therefore asked again after the slow
    /// check. A clock that went back is not due.
    pub fn due(&self, now: TimeDelta) -> bool {
        !self.has_asked
            || self.switches_owed > 0
            || self.unsettled
            || since(now, self.last_asked) >= SLOW_CHECK
    }

    /// A question went out at `now`: it pays one owed switch question and is the one that
    /// settles the answers.
    pub fn asked(&self, now: TimeDelta) -> TaskbarQuerySchedule {
        TaskbarQuerySchedule {
            has_asked: true,
            last_asked: now,
            switches_owed: (self.switches_owed - 1).max(0),
            unsettled: false,
            ask_count: (self.ask_count + 1).min(ASKS_BEFORE_ANCHORS),
            ..*self
        }
    }

    /// The answer to a question; None when the taskbar could not be read, which is an answer
    /// like any other. The first answer, or one that differs from the answer before it, calls
    /// for another question. At the start, while no anchors have ever come and fewer than
    /// `ASKS_BEFORE_ANCHORS` questions were asked (the tray may not be built yet), a missing
    /// answer never settles it.
    pub fn answered(&self, answer: Option<TaskbarAnchors>) -> TaskbarQuerySchedule {
        let anchors_seen = self.anchors_seen || answer.is_some();
        let waiting_for_tray = !anchors_seen && self.ask_count < ASKS_BEFORE_ANCHORS;
        TaskbarQuerySchedule {
            unsettled: waiting_for_tray || !self.has_answer || answer != self.last_answer,
            has_answer: true,
            last_answer: answer,
            anchors_seen,
            ..*self
        }
    }

    /// The foreground window changed: the next two ticks ask.
    pub fn foreground_switched(&self) -> TaskbarQuerySchedule {
        TaskbarQuerySchedule {
            switches_owed: ASKS_PER_SWITCH,
            ..*self
        }
    }
}

/// `now - earlier`, where a difference past the ends of a `TimeDelta` stays at that end instead of
/// failing (the C# `TimeSpan` throws there; no clock reaches it).
fn since(now: TimeDelta, earlier: TimeDelta) -> TimeDelta {
    now.checked_sub(&earlier).unwrap_or(if now > earlier {
        TimeDelta::MAX
    } else {
        TimeDelta::MIN
    })
}
