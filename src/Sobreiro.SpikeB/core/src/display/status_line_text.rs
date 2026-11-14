use crate::usage::RateLimits;

pub struct StatusLineText;

impl StatusLineText {
    /// The bridge's default line. Shows exactly what Claude Code just sent, so no reset logic.
    pub fn render(limits: Option<&RateLimits>) -> String {
        let pct = |w: Option<f64>| percent(w);
        let mut line = format!(
            "5h {} · 7d {}",
            pct(limits.and_then(|l| l.five_hour.as_ref()).map(|w| w.used_percentage)),
            pct(limits.and_then(|l| l.seven_day.as_ref()).map(|w| w.used_percentage)),
        );
        if let Some(spend) = limits.and_then(|l| l.spend.as_ref()) {
            line.push_str(&format!(" · spend {}", percent(Some(spend.used_percentage))));
        }
        line
    }
}

fn percent(used: Option<f64>) -> String {
    match used {
        Some(u) => format!("{}%", u.floor().clamp(0.0, 100.0) as i32),
        None => "–".to_string(),
    }
}
