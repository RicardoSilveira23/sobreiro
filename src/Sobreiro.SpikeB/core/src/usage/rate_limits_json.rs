use super::{LimitWindow, RateLimits, SpendLimit, UsageSnapshot};
use crate::{Time, from_unix_seconds};
use serde_json::{Map, Value};

/// Reads the status line / usage.json shape and writes usage.json.
pub struct RateLimitsJson;

impl RateLimitsJson {
    /// Parses Claude Code's status line stdin. None when there is no usable rate_limits.
    pub fn parse_status_line(json: &str) -> Option<RateLimits> {
        let doc: Value = serde_json::from_str(json).ok()?;
        let rl = doc.as_object()?.get("rate_limits")?;
        parse_rate_limits(rl, false)
    }

    /// Parses usage.json. None without observed_at or usable rate_limits.
    pub fn parse_usage_file(json: &str) -> Option<UsageSnapshot> {
        let doc: Value = serde_json::from_str(json).ok()?;
        let root = doc.as_object()?;
        let observed_at = try_unix(root.get("observed_at")?)?;
        let limits = parse_rate_limits(root.get("rate_limits")?, true)?;
        Some(UsageSnapshot { limits, observed_at })
    }

    pub fn write_usage_file(s: &UsageSnapshot) -> String {
        let mut out = String::new();
        out.push('{');
        out.push_str(&format!("\"observed_at\":{}", s.observed_at.timestamp()));
        out.push_str(",\"rate_limits\":{");
        let mut first = true;
        let mut sep = |out: &mut String| {
            if !first {
                out.push(',');
            }
            first = false;
        };
        if let Some(five) = &s.limits.five_hour {
            sep(&mut out);
            write_window(&mut out, "five_hour", five.used_percentage, five.resets_at, five.observed_at);
        }
        if let Some(seven) = &s.limits.seven_day {
            sep(&mut out);
            write_window(&mut out, "seven_day", seven.used_percentage, seven.resets_at, seven.observed_at);
        }
        if let Some(spend) = &s.limits.spend {
            sep(&mut out);
            out.push_str("\"spend_limit\":{");
            out.push_str(&format!("\"used_percentage\":{}", num(spend.used_percentage)));
            if let Some(r) = spend.resets_at {
                out.push_str(&format!(",\"resets_at\":{}", r.timestamp()));
            }
            if let Some(u) = spend.used_usd {
                out.push_str(&format!(",\"used_usd\":{}", num(u)));
            }
            if let Some(l) = spend.limit_usd {
                out.push_str(&format!(",\"limit_usd\":{}", num(l)));
            }
            if let Some(p) = &spend.period {
                out.push_str(&format!(",\"period\":{}", Value::String(p.clone())));
            }
            if let Some(o) = spend.observed_at {
                out.push_str(&format!(",\"observed_at\":{}", o.timestamp()));
            }
            out.push('}');
        }
        out.push_str("}}");
        out
    }
}

/// A double the way a JSON writer prints it: whole numbers without a fraction.
fn num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else if v.is_finite() {
        format!("{v}")
    } else {
        "0".to_string()
    }
}

fn write_window(out: &mut String, name: &str, pct: f64, resets_at: Option<Time>, observed_at: Option<Time>) {
    out.push_str(&format!("\"{name}\":{{\"used_percentage\":{}", num(pct)));
    if let Some(r) = resets_at {
        out.push_str(&format!(",\"resets_at\":{}", r.timestamp()));
    }
    if let Some(o) = observed_at {
        out.push_str(&format!(",\"observed_at\":{}", o.timestamp()));
    }
    out.push('}');
}

fn parse_rate_limits(rl: &Value, read_observed_at: bool) -> Option<RateLimits> {
    let rl = rl.as_object()?;
    let five = parse_window(rl, "five_hour", read_observed_at);
    let seven = parse_window(rl, "seven_day", read_observed_at);
    let spend = parse_spend(rl, read_observed_at);
    if five.is_none() && seven.is_none() && spend.is_none() {
        None
    } else {
        Some(RateLimits { five_hour: five, seven_day: seven, spend })
    }
}

fn parse_window(parent: &Map<String, Value>, name: &str, read_observed_at: bool) -> Option<LimitWindow> {
    let el = parent.get(name)?.as_object()?;
    let pct = try_number(el, "used_percentage")?;
    Some(LimitWindow {
        used_percentage: pct,
        resets_at: el.get("resets_at").and_then(try_unix),
        observed_at: if read_observed_at { el.get("observed_at").and_then(try_unix) } else { None },
    })
}

fn parse_spend(parent: &Map<String, Value>, read_observed_at: bool) -> Option<SpendLimit> {
    let el = parent.get("spend_limit")?.as_object()?;
    let pct = try_number(el, "used_percentage")?;
    let period = match el.get("period") {
        Some(Value::String(s)) if matches!(s.as_str(), "daily" | "weekly" | "monthly") => Some(s.clone()),
        _ => None,
    };
    Some(SpendLimit {
        used_percentage: pct,
        resets_at: el.get("resets_at").and_then(try_unix),
        used_usd: try_number(el, "used_usd"),
        limit_usd: try_number(el, "limit_usd"),
        period,
        observed_at: if read_observed_at { el.get("observed_at").and_then(try_unix) } else { None },
    })
}

fn try_number(parent: &Map<String, Value>, name: &str) -> Option<f64> {
    match parent.get(name) {
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

pub(crate) fn try_unix(el: &Value) -> Option<Time> {
    let Value::Number(n) = el else { return None };
    let d = n.as_f64()?;
    if !d.is_finite() {
        return None;
    }
    from_unix_seconds(d as i64)
}
