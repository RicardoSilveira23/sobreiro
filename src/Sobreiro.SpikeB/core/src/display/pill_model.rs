use super::{PillStale, RingsModel};
use crate::limits::{Limit, LimitKind, UsageLevel, UsageLevels};
use crate::polling::ProviderState;
use crate::{Time, Zone};

/// A label and its figure; `stale` when the figure is a last reading that is no longer updated.
#[derive(Clone, Debug, PartialEq)]
pub struct PillLine {
    pub label: String,
    pub figure: String,
    pub stale: bool,
}

/// Copilot's block of the pill: the credits figure, the reset date and the bar.
#[derive(Clone, Debug, PartialEq)]
pub struct PillCopilot {
    pub credits: PillLine,
    pub reset_date: String,
    pub fraction: f64,
    pub level: UsageLevel,
    pub stale: bool,
}

/// What the taskbar pill shows: a 5h line, a 7d line and the rings, plus Copilot's block when there is one.
#[derive(Clone, Debug, PartialEq)]
pub struct PillModel {
    pub session: PillLine,
    pub weekly: PillLine,
    pub rings: RingsModel,
    pub copilot: Option<PillCopilot>,
    pub show_claude: bool,
    pub claude_stale: bool,
}

const DASH: &str = "–";

impl PillModel {
    /// `From(limits, zone, showClaude)`: nothing is stale.
    pub fn from(limits: &[Limit], zone: Zone, show_claude: bool) -> PillModel {
        // Off and Ok are never stale, so the placeholder time is never read.
        Self::from_state(limits, zone, show_claude, ProviderState::Off, 0, ProviderState::Ok, 0, crate::from_unix_seconds(0).unwrap())
    }

    /// `From(limits, zone, showClaude, live, liveMisses, copilot, copilotMisses, now)`: a block whose numbers have
    /// stopped keeps its figures and its fill, marked stale.
    #[allow(clippy::too_many_arguments)]
    pub fn from_state(
        limits: &[Limit], zone: Zone, show_claude: bool, live: ProviderState, live_misses: i32,
        copilot: ProviderState, copilot_misses: i32, now: Time,
    ) -> PillModel {
        let claude_stale = PillStale::claude(live, limits, now, live_misses);
        let credits = limits.iter().find(|l| l.kind == LimitKind::Credits);
        let mut m = Self::build(limits, claude_stale);
        m.copilot = copilot_block(credits, zone, PillStale::copilot(copilot, credits, now, copilot_misses));
        m.show_claude = show_claude;
        m
    }

    /// `From(limits)`.
    pub fn from_limits(limits: &[Limit]) -> PillModel {
        Self::build(limits, false)
    }

    fn build(limits: &[Limit], claude_stale: bool) -> PillModel {
        PillModel {
            session: line("5h", limits.iter().find(|l| l.kind == LimitKind::Session), claude_stale),
            weekly: line("7d", limits.iter().find(|l| l.kind == LimitKind::Weekly), claude_stale),
            rings: RingsModel::from(limits),
            copilot: None,
            show_claude: true,
            claude_stale,
        }
    }
}

fn copilot_block(credits: Option<&Limit>, zone: Zone, stale: bool) -> Option<PillCopilot> {
    let credits = credits?;
    let date = match credits.resets_at {
        Some(at) => zone.convert(at).format("%-d %b").to_string(),
        None => DASH.to_string(),
    };
    Some(PillCopilot {
        credits: line("AI", Some(credits), stale),
        reset_date: date,
        fraction: (credits.percent / 100.0).clamp(0.0, 1.0),
        level: UsageLevels::for_percent(credits.display_percent()),
        stale,
    })
}

// A line with no reading is a dash and never stale: there is no figure to mark.
fn line(label: &str, limit: Option<&Limit>, stale: bool) -> PillLine {
    match limit {
        None => PillLine { label: label.to_string(), figure: DASH.to_string(), stale: false },
        Some(l) => PillLine { label: label.to_string(), figure: format!("{}%", l.display_percent()), stale },
    }
}
