//! One diagnostics log entry as text.
use chrono::{DateTime, FixedOffset};

/// The most UTF-16 code units an entry holds before it is cut.
pub const MAX_CHARS: usize = 16384;

/// What is added to an entry that was cut.
const CUT: &str = "\n  (cut)\n";

/// Formats one diagnostics log entry: a header line, then the name of the kind of failure, when
/// there is one.
///
/// An entry holds only the event text the caller gives and a kind name (a type name, nothing
/// the failure said). The text of an error, its data and its debug output are never written,
/// since they can hold paths or tokens. 1.x walked .NET exceptions here; Rust has none, so
/// nothing takes their place.
///
/// Core looks at no clock and has no process of its own: the caller gives the time and the
/// process id, and appends what comes back with `DiagnosticsLog::append`. There is no `write`
/// that does both.
pub fn format(at: DateTime<FixedOffset>, pid: u32, event: &str, kind: Option<&str>) -> String {
    let mut text = String::new();
    text.push_str(&at.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string());
    text.push_str(" pid=");
    text.push_str(&pid.to_string());
    text.push(' ');
    text.extend(
        event
            .chars()
            .map(|c| if c == '\r' || c == '\n' { ' ' } else { c }),
    );
    text.push('\n');

    if let Some(kind) = kind {
        text.push_str("  ");
        text.push_str(kind);
        text.push('\n');
    }

    cut(text)
}

/// Cuts `text` at [`MAX_CHARS`] UTF-16 code units, as 1.x did. A cut that falls inside a
/// character outside the first plane leaves half of it, which 1.x's encoder wrote as U+FFFD:
/// the same here.
fn cut(text: String) -> String {
    if text.encode_utf16().count() <= MAX_CHARS {
        return text;
    }
    let mut kept = String::new();
    let mut units = 0;
    for c in text.chars() {
        let width = c.len_utf16();
        if units + width > MAX_CHARS {
            if units < MAX_CHARS {
                kept.push('\u{FFFD}');
            }
            break;
        }
        kept.push(c);
        units += width;
    }
    kept.push_str(CUT);
    kept
}
