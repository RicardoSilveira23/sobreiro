use crate::usage::{LimitWindow, RateLimits};
use chrono::DateTime;
use serde_json::{Map, Value};

/// Reads the reply of GET /api/oauth/usage. Prefers the "limits" list (group session or weekly, unscoped entries
/// only); a window the list lacks comes from the keyed five_hour and seven_day objects. Anything else is ignored.
/// Windows come back with observed_at None; the caller's merge stamps it.
pub struct UsageApiJson;

impl UsageApiJson {
    pub fn parse(json: &str) -> Option<RateLimits> {
        let doc: Value = serde_json::from_str(json).ok()?;
        let root = doc.as_object()?;

        let (mut five, mut seven) = from_list(root);
        if five.is_none() || seven.is_none() {
            let (kf, ks) = (keyed(root, "five_hour"), keyed(root, "seven_day"));
            five = five.or(kf);
            seven = seven.or(ks);
        }
        if five.is_none() && seven.is_none() { None } else { Some(RateLimits { five_hour: five, seven_day: seven, spend: None }) }
    }
}

fn from_list(root: &Map<String, Value>) -> (Option<LimitWindow>, Option<LimitWindow>) {
    let (mut five, mut seven) = (None, None);
    let Some(Value::Array(list)) = root.get("limits") else { return (None, None) };

    for entry in list {
        let Some(entry) = entry.as_object() else { continue };
        if entry.get("scope").is_some_and(|s| !s.is_null()) {
            continue;
        }
        let Some(Value::String(group)) = entry.get("group") else { continue };
        let Some(window) = window(entry, "percent") else { continue };
        match group.as_str() {
            "session" => five = five.or(Some(window)),
            "weekly" => seven = seven.or(Some(window)),
            _ => {}
        }
    }
    (five, seven)
}

fn keyed(root: &Map<String, Value>, key: &str) -> Option<LimitWindow> {
    match root.get(key) {
        Some(Value::Object(o)) => window(o, "utilization"),
        _ => None,
    }
}

fn window(element: &Map<String, Value>, percent_key: &str) -> Option<LimitWindow> {
    let Some(Value::Number(n)) = element.get(percent_key) else { return None };
    let used = n.as_f64()?;
    let resets_at = match element.get("resets_at") {
        Some(Value::String(s)) => DateTime::parse_from_rfc3339(s).ok(),
        _ => None,
    };
    Some(LimitWindow::new(used, resets_at))
}
