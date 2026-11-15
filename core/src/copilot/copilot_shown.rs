//! Which of two readings of the AI credits to show: the one Sobreiro asked GitHub for, or the
//! one the Copilot CLI cached.
use super::CopilotReading;

/// The later reading, by when it was observed. When both were observed together, the live
/// one. The reading is handed back, not copied.
pub fn pick<'a>(
    live: Option<&'a CopilotReading>,
    cache: Option<&'a CopilotReading>,
) -> Option<&'a CopilotReading> {
    match (live, cache) {
        (None, cache) => cache,
        (live, None) => live,
        (Some(live), Some(cache)) => Some(if cache.observed_at > live.observed_at {
            cache
        } else {
            live
        }),
    }
}
