//! The one balloon for a refresh's alerts, so none is dropped silently.
use super::alert_policy::Alert;

/// The balloon's title and text: the first alert's title, and its body and then the other alerts'
/// titles, one per line. None when there are no alerts. The text is never empty (Windows refuses
/// an empty balloon): a single space at least.
pub fn for_alerts(alerts: &[Alert]) -> Option<(String, String)> {
    let first = alerts.first()?;
    let mut lines: Vec<&str> = Vec::new();
    if !first.body.is_empty() {
        lines.push(&first.body);
    }
    lines.extend(alerts.iter().skip(1).map(|alert| alert.title.as_str()));
    let text = if lines.is_empty() {
        " ".to_string()
    } else {
        lines.join("\n")
    };
    Some((first.title.clone(), text))
}
