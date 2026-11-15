//! The sentence under the Copilot figures: the problem first, then where the reading comes from.
use chrono::{DateTime, TimeDelta, Utc};

use super::{CopilotReading, CopilotSource};
use crate::display::{ClockFormat, Zone, when};
use crate::polling::ProviderState;

/// A live reading older than this says when it is from.
const STALE: TimeDelta = TimeDelta::minutes(15);

/// The sentence, or none when there is nothing to say.
pub fn line(
    state: ProviderState,
    reading: &CopilotReading,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
) -> Option<String> {
    let problem = match state {
        ProviderState::NoSignIn => {
            Some("No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh.")
        }
        ProviderState::Refused => {
            Some("GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh.")
        }
        ProviderState::RateLimited => {
            Some("GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself.")
        }
        ProviderState::Offline => Some(
            "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes.",
        ),
        ProviderState::BadAnswer => Some(
            "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro.",
        ),
        _ => None,
    };
    let seen = || when::format(reading.observed_at, now, zone, clock);
    let source = if reading.source == CopilotSource::Cache {
        Some(format!("From the Copilot CLI, as of {}.", seen()))
    } else if now.signed_duration_since(reading.observed_at) > STALE {
        Some(format!("As of {}.", seen()))
    } else {
        None
    };

    match (problem, source) {
        (None, source) => source,
        (Some(problem), None) => Some(problem.to_string()),
        (Some(problem), Some(source)) => Some(format!("{problem} {source}")),
    }
}
