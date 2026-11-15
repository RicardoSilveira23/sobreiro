//! Which of two readings of the AI credits to show: the one Sobreiro asked GitHub for, or the
//! one the Copilot CLI cached.
//!
//! The sentence under the figures, which tells where a reading comes from, arrives with the
//! area that formats times.
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
