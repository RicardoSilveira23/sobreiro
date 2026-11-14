use super::{CopilotReading, CopilotSource};
use crate::display::When;
use crate::polling::ProviderState;
use crate::{Span, Time, Zone};

/// Which of the Live and the Copilot CLI's cached readings to show.
pub struct CopilotShown;

impl CopilotShown {
    /// The later observed_at wins; a tie goes to live.
    pub fn pick(live: Option<&CopilotReading>, cache: Option<&CopilotReading>) -> Option<CopilotReading> {
        match (live, cache) {
            (None, c) => c.cloned(),
            (l, None) => l.cloned(),
            (Some(l), Some(c)) => Some(if c.observed_at > l.observed_at { c.clone() } else { l.clone() }),
        }
    }
}

/// The sentence under the Copilot figures: the problem first, then where the reading comes from.
pub struct CopilotStateText;

impl CopilotStateText {
    fn stale() -> Span {
        Span::minutes(15)
    }

    pub fn line(state: ProviderState, reading: &CopilotReading, now: Time, zone: Zone) -> Option<String> {
        let problem = match state {
            ProviderState::NoSignIn => Some("No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh."),
            ProviderState::Refused => Some("GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh."),
            ProviderState::RateLimited => Some("GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself."),
            ProviderState::Offline => Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
            ProviderState::BadAnswer => Some("GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro."),
            _ => None,
        };
        let source = if reading.source == CopilotSource::Cache {
            Some(format!("From the Copilot CLI, as of {}.", When::format(reading.observed_at, now, zone)))
        } else if now - reading.observed_at > Self::stale() {
            Some(format!("As of {}.", When::format(reading.observed_at, now, zone)))
        } else {
            None
        };

        match (problem, source) {
            (None, source) => source,
            (Some(p), None) => Some(p.to_string()),
            (Some(p), Some(s)) => Some(format!("{p} {s}")),
        }
    }
}
