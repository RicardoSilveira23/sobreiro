use super::TaskbarAnchors;
use crate::Span;

/// Decides whether a pill tick also asks UI Automation where the taskbar's parts are. `default()` is the fresh
/// schedule: nothing asked, nothing answered. Immutable: each event gives the next schedule. The times are a
/// monotonic clock as a `Span` (time since some start), as the C# passes a `TimeSpan`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaskbarQuerySchedule {
    has_asked: bool,
    last_asked: Span,
    switches_owed: i32,
    unsettled: bool,
    has_answer: bool,
    last_answer: Option<TaskbarAnchors>,
    anchors_seen: bool,
    ask_count: i32,
}

impl TaskbarQuerySchedule {
    /// How long a settled schedule goes without asking: layout changes that raise no foreground switch are noticed
    /// within about 16 seconds (15, landing on a 2-second tick).
    pub fn slow_check() -> Span {
        Span::seconds(15)
    }

    /// Questions a foreground switch owes: the first can land before the new window's button or tray icon exists.
    pub const ASKS_PER_SWITCH: i32 = 2;

    /// Questions (one minute of 2-second ticks) after which missing answers may settle, when no anchors were ever seen.
    pub const ASKS_BEFORE_ANCHORS: i32 = 30;

    /// True when nothing has been asked yet, a foreground switch still owes questions (two each), the last answers
    /// have not agreed yet, or `slow_check` has passed since the last question. A question whose answer never comes
    /// is therefore asked again after the slow check. A clock that went back is not due.
    pub fn due(&self, now: Span) -> bool {
        !self.has_asked || self.switches_owed > 0 || self.unsettled || now - self.last_asked >= Self::slow_check()
    }

    /// A question went out at `now`: it pays one owed switch question and is the one that settles the answers.
    pub fn asked(self, now: Span) -> Self {
        Self {
            has_asked: true,
            last_asked: now,
            switches_owed: (self.switches_owed - 1).max(0),
            unsettled: false,
            ask_count: (self.ask_count + 1).min(Self::ASKS_BEFORE_ANCHORS),
            ..self
        }
    }

    /// The answer to a question; None when the taskbar could not be read, which is an answer like any other. The
    /// first answer, or one that differs from the answer before it, calls for another question. At the start, while
    /// no anchors have ever come and fewer than `ASKS_BEFORE_ANCHORS` questions were asked (the tray may not be
    /// built yet), a missing answer never settles it.
    pub fn answered(self, answer: Option<TaskbarAnchors>) -> Self {
        let anchors_seen = self.anchors_seen || answer.is_some();
        let waiting_for_tray = !anchors_seen && self.ask_count < Self::ASKS_BEFORE_ANCHORS;
        Self {
            unsettled: waiting_for_tray || !self.has_answer || answer != self.last_answer,
            has_answer: true,
            last_answer: answer,
            anchors_seen,
            ..self
        }
    }

    /// The foreground window changed: the next two ticks ask.
    pub fn foreground_switched(self) -> Self {
        Self { switches_owed: Self::ASKS_PER_SWITCH, ..self }
    }
}
