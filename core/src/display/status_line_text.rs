//! The line the bridge prints when the user had no status line of their own.
use crate::usage::RateLimits;

/// The bridge's default line, for example `5h 23% · 7d 41% · spend 62%`. It shows exactly what
/// Claude Code just sent, so a window whose reset time has passed is not read as zero here.
pub fn render(limits: Option<&RateLimits>) -> String {
    let five_hour = limits.and_then(|limits| limits.five_hour.as_ref());
    let seven_day = limits.and_then(|limits| limits.seven_day.as_ref());
    let mut line = format!(
        "5h {} · 7d {}",
        percent(five_hour.map(|window| window.used_percentage)),
        percent(seven_day.map(|window| window.used_percentage)),
    );
    if let Some(spend) = limits.and_then(|limits| limits.spend.as_ref()) {
        line.push_str(&format!(
            " · spend {}",
            percent(Some(spend.used_percentage))
        ));
    }
    line
}

/// A percentage rounded down and kept within 0 to 100, or a dash when there is none.
fn percent(used: Option<f64>) -> String {
    match used {
        Some(used) => format!("{}%", used.floor().clamp(0.0, 100.0) as i32),
        None => "–".to_string(),
    }
}
