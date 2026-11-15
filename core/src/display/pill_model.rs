//! What the taskbar pill shows: a 5h line, a 7d line and the rings, plus Copilot's block when
//! there is one.
use chrono::{DateTime, Utc};

use super::rings::RingsModel;
use super::{Zone, pill_stale};
use crate::limits::usage_level::for_percent;
use crate::limits::{Limit, LimitKind, UsageLevel};
use crate::polling::ProviderState;

const DASH: &str = "–";

/// A label and its figure. `stale` when the figure is a last reading that is no longer updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PillLine {
    pub label: &'static str,
    pub figure: String,
    pub stale: bool,
}

/// Copilot's block of the pill: the credits figure, the reset date and the bar.
#[derive(Debug, Clone, PartialEq)]
pub struct PillCopilot {
    pub credits: PillLine,
    pub reset_date: String,
    pub fraction: f64,
    pub level: UsageLevel,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PillModel {
    pub session: PillLine,
    pub weekly: PillLine,
    pub rings: RingsModel,
    pub copilot: Option<PillCopilot>,
    pub show_claude: bool,
    pub claude_stale: bool,
}

impl PillModel {
    /// Claude Code's two lines and the rings, nothing marked stale and no Copilot block.
    pub fn from_limits(limits: &[Limit]) -> PillModel {
        from_claude(limits, false)
    }

    /// As [`from_limits`](Self::from_limits), with Copilot's block when there are credits.
    pub fn from(limits: &[Limit], zone: &Zone, show_claude: bool) -> PillModel {
        // Off and Ok are never stale, so the placeholder time is never read.
        PillModel::from_states(
            limits,
            zone,
            show_claude,
            ProviderState::Off,
            0,
            ProviderState::Ok,
            0,
            DateTime::UNIX_EPOCH,
        )
    }

    /// As above, and a block whose numbers have stopped keeps its figures and its fill, marked
    /// stale.
    #[allow(clippy::too_many_arguments)]
    pub fn from_states(
        limits: &[Limit],
        zone: &Zone,
        show_claude: bool,
        live: ProviderState,
        live_misses: i32,
        copilot: ProviderState,
        copilot_misses: i32,
        now: DateTime<Utc>,
    ) -> PillModel {
        let claude_stale = pill_stale::claude(live, limits, now, live_misses);
        let credits = limits.iter().find(|limit| limit.kind == LimitKind::Credits);
        let copilot_stale = pill_stale::copilot(copilot, credits, now, copilot_misses);
        PillModel {
            copilot: credits.map(|credits| copilot_block(credits, zone, copilot_stale)),
            show_claude,
            ..from_claude(limits, claude_stale)
        }
    }
}

fn copilot_block(credits: &Limit, zone: &Zone, stale: bool) -> PillCopilot {
    let date = match credits.resets_at {
        Some(at) => zone.local(at).format("%-d %b").to_string(),
        None => DASH.to_string(),
    };
    PillCopilot {
        credits: line("AI", Some(credits), stale),
        reset_date: date,
        fraction: (credits.percent / 100.0).clamp(0.0, 1.0),
        level: for_percent(credits.display_percent()),
        stale,
    }
}

fn from_claude(limits: &[Limit], claude_stale: bool) -> PillModel {
    let first = |kind| limits.iter().find(|limit| limit.kind == kind);
    PillModel {
        session: line("5h", first(LimitKind::Session), claude_stale),
        weekly: line("7d", first(LimitKind::Weekly), claude_stale),
        rings: RingsModel::from(limits),
        copilot: None,
        show_claude: true,
        claude_stale,
    }
}

// A line with no reading is a dash and never stale: there is no figure to mark.
fn line(label: &'static str, limit: Option<&Limit>, stale: bool) -> PillLine {
    match limit {
        None => PillLine {
            label,
            figure: DASH.to_string(),
            stale: false,
        },
        Some(limit) => PillLine {
            label,
            figure: format!("{}%", limit.display_percent()),
            stale,
        },
    }
}
