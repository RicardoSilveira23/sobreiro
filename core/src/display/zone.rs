//! A time zone as Core knows it: a list of offsets and the moments they begin.
use chrono::{DateTime, FixedOffset, NaiveDateTime, Offset, Utc};

/// A moment from which a zone keeps another offset from UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneChange {
    pub at: DateTime<Utc>,
    pub offset: FixedOffset,
}

/// The user's time zone, handed in by the shell: the offset before the first change and each
/// change after it, which the shell builds from Windows' rules for the years around now. Past
/// the last change the last offset holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    first: FixedOffset,
    changes: Vec<ZoneChange>,
}

impl Zone {
    /// The zone of the whole world: no offset, and no change.
    pub fn utc() -> Zone {
        Zone::fixed(Utc.fix())
    }

    /// A zone that keeps one offset at all times.
    pub fn fixed(offset: FixedOffset) -> Zone {
        Zone {
            first: offset,
            changes: Vec::new(),
        }
    }

    /// A zone that begins at `first` and changes as `changes` say, in whatever order they come.
    pub fn new(first: FixedOffset, mut changes: Vec<ZoneChange>) -> Zone {
        changes.sort_by_key(|change| change.at);
        Zone { first, changes }
    }

    /// The offset from UTC at a moment. A change holds from its moment on.
    pub fn offset_at(&self, moment: DateTime<Utc>) -> FixedOffset {
        let begun = self.changes.partition_point(|change| change.at <= moment);
        match begun.checked_sub(1).and_then(|last| self.changes.get(last)) {
            Some(change) => change.offset,
            None => self.first,
        }
    }

    /// The clock on the wall at a moment. Within its offset of the first or last moment chrono
    /// holds, the wall would lie outside what it holds: the nearest one it holds is given.
    pub fn local(&self, moment: DateTime<Utc>) -> NaiveDateTime {
        let offset = self.offset_at(moment);
        moment
            .naive_utc()
            .checked_add_offset(offset)
            .unwrap_or(if offset.local_minus_utc() > 0 {
                NaiveDateTime::MAX
            } else {
                NaiveDateTime::MIN
            })
    }

    /// The moment at which the wall shows `local`. None when the clock skips that time. Of two
    /// moments, when the clock shows it twice, the later one: what the C# got from .NET, the
    /// standard offset after a fall-back.
    pub fn moment(&self, local: NaiveDateTime) -> Option<DateTime<Utc>> {
        std::iter::once(self.first)
            .chain(self.changes.iter().map(|change| change.offset))
            .filter_map(|offset| {
                let moment = local.checked_sub_offset(offset)?.and_utc();
                (self.offset_at(moment) == offset).then_some(moment)
            })
            .max()
    }
}
