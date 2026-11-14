use super::{CopilotReading, CopilotSource};
use crate::{Span, Time};
use chrono::DateTime;
use serde_json::{Map, Value};

/// Reads the reply of GET https://api.github.com/copilot_internal/user. The AI credits sit in
/// quota_snapshots.premium_models when that is an object, else premium_interactions. Every field is optional and a
/// field of the wrong type counts as absent; a reply with no usable budget and usage gives None. Never panics.
pub struct CopilotUserJson;

impl CopilotUserJson {
    pub fn parse(body: &str, now: Time) -> Option<CopilotReading> {
        let doc: Value = serde_json::from_str(body).ok()?;
        Self::from_response(&doc, now, CopilotSource::Live)
    }

    pub fn from_response(response: &Value, fallback_observed_at: Time, source: CopilotSource) -> Option<CopilotReading> {
        let response = response.as_object()?;
        let snapshots = response.get("quota_snapshots")?.as_object()?;

        let snapshot = snapshot(snapshots, "premium_models").or_else(|| snapshot(snapshots, "premium_interactions"))?;

        if matches!(snapshot.get("unlimited"), Some(Value::Bool(true))) {
            return None;
        }
        let budget = number(snapshot, "entitlement").filter(|b| *b > 0.0)?;

        let mut used = used(snapshot, budget)?;
        if let Some(overage) = number(snapshot, "overage_count") {
            if overage > 0.0 && used <= budget {
                used = budget + overage;
            }
        }

        let observed_at = date(snapshot, "timestamp_utc").unwrap_or(fallback_observed_at);
        // A reset date far from the reading is nonsense: treat it as absent.
        let mut resets_at = date(response, "quota_reset_date_utc");
        if let Some(reset) = resets_at {
            if (reset - observed_at).abs() > Span::days(400) {
                resets_at = None;
            }
        }

        Some(CopilotReading { budget, used, resets_at, observed_at, source })
    }
}

fn snapshot<'a>(snapshots: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    snapshots.get(key)?.as_object()
}

fn used(s: &Map<String, Value>, budget: f64) -> Option<f64> {
    if let Some(quota_remaining) = number(s, "quota_remaining") {
        return Some(budget - quota_remaining);
    }
    if let Some(remaining) = number(s, "remaining") {
        return Some(budget - remaining);
    }
    if let Some(percent_remaining) = number(s, "percent_remaining") {
        return Some(budget * (100.0 - percent_remaining) / 100.0);
    }
    number(s, "credits_used")
}

fn number(element: &Map<String, Value>, key: &str) -> Option<f64> {
    match element.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

fn date(element: &Map<String, Value>, key: &str) -> Option<Time> {
    match element.get(key) {
        Some(Value::String(s)) => DateTime::parse_from_rfc3339(s).ok(),
        _ => None,
    }
}
