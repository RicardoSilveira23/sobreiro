//! The tray tooltip: what the pill shows, as plain text, cut to the length Windows allows.
//! Also the two number formats the flyout shares with it: money and credits, grouped by
//! thousands as .NET writes them with the invariant culture.
use chrono::{DateTime, Utc};

use super::{ClockFormat, Zone, when};
use crate::limits::{Limit, LimitKind};

/// The longest text a Windows tray tooltip can show, in UTF-16 units.
pub const MAX_LENGTH: usize = 127;

/// "AI credits 17% · resets Dec 1 ...", the credits figures and the age, cut to [`MAX_LENGTH`].
pub fn render_copilot(
    credits: &Limit,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
) -> String {
    let mut lines = vec![line(credits, now, zone, clock)];
    if !credits.is_reset
        && let Some(figures) = credits_figures(credits)
    {
        lines.push(figures);
    }
    lines.push(format!(
        "as of {}",
        when::format(credits.observed_at, now, zone, clock)
    ));
    cut(lines.join("\n"))
}

/// "2,644 of 15,000 credits", or none without both numbers.
pub fn credits_figures(limit: &Limit) -> Option<String> {
    let used = limit.used_credits?;
    let budget = limit.credit_budget?;
    Some(format!(
        "{} of {} credits",
        grouped(used.round_ties_even(), 0),
        grouped(budget.round_ties_even(), 0)
    ))
}

/// The tooltip for Claude Code's limits. Copilot's credits are not in it.
pub fn render(limits: &[Limit], now: DateTime<Utc>, zone: &Zone, clock: &ClockFormat) -> String {
    let limits: Vec<&Limit> = limits
        .iter()
        .filter(|limit| limit.kind != LimitKind::Credits)
        .collect();
    if limits.is_empty() {
        return "Sobreiro · no data yet".to_string();
    }

    let mut lines = Vec::new();
    for kind in [LimitKind::Session, LimitKind::Weekly, LimitKind::Spend] {
        match limits.iter().find(|limit| limit.kind == kind) {
            // Session and weekly are always listed; spend only when present.
            None if kind != LimitKind::Spend => {
                let name = if kind == LimitKind::Session {
                    "Session"
                } else {
                    "Weekly"
                };
                lines.push(format!("{name} –"));
            }
            None => {}
            Some(limit) => lines.push(line(limit, now, zone, clock)),
        }
    }
    let newest = limits.iter().map(|limit| limit.observed_at).max();
    if let Some(newest) = newest {
        lines.push(format!("as of {}", when::format(newest, now, zone, clock)));
    }
    cut(lines.join("\n"))
}

fn line(limit: &Limit, now: DateTime<Utc>, zone: &Zone, clock: &ClockFormat) -> String {
    let mut line = format!("{} {}%", limit.label, limit.display_percent());
    if let Some(at) = limit.resets_at {
        let word = if limit.is_reset { "reset" } else { "resets" };
        line.push_str(&format!(" · {word} {}", when::format(at, now, zone, clock)));
    }
    line
}

/// The text cut to [`MAX_LENGTH`] UTF-16 units, as the C# `text[..127]` does. The C# can leave
/// half of a surrogate pair at the end; here the half is dropped.
fn cut(text: String) -> String {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    if units.len() <= MAX_LENGTH {
        return text;
    }
    units.truncate(MAX_LENGTH);
    if units
        .last()
        .is_some_and(|unit| (0xD800..0xDC00).contains(unit))
    {
        units.pop();
    }
    String::from_utf16_lossy(&units)
}

/// A number written as .NET writes a double with the pattern `#,0` or `#,0.00` and the invariant
/// culture: rounded to 15 significant digits first, then to `decimals` places with halves away
/// from zero, grouped by thousands. Not-a-number is written "NaN", the infinities "Infinity" and
/// "-Infinity". A negative that rounds to zero keeps its minus sign.
///
/// Rust's own `{:.2}` rounds the exact binary value instead, and gives 2.67 for 2.675.
pub(crate) fn grouped(value: f64, decimals: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    // "d.dddddddddddddde<exp>": fifteen correctly rounded digits and the power of ten.
    let sci = format!("{:.14e}", value.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();

    // The digits that stay: all of the integer part and `decimals` of the fraction.
    let keep = exponent + 1 + decimals as i32;
    let mut kept: Vec<u8> = Vec::new();
    if keep >= 0 {
        for at in 0..keep as usize {
            kept.push(digits.get(at).map_or(0, |digit| digit - b'0'));
        }
        if digits
            .get(keep as usize)
            .is_some_and(|&digit| digit >= b'5')
        {
            let mut at = kept.len();
            loop {
                if at == 0 {
                    kept.insert(0, 1);
                    break;
                }
                at -= 1;
                if kept[at] == 9 {
                    kept[at] = 0;
                } else {
                    kept[at] += 1;
                    break;
                }
            }
        }
    }
    let mut text: String = kept.iter().map(|digit| (b'0' + digit) as char).collect();
    let trimmed = text.trim_start_matches('0').len();
    text.drain(..text.len() - trimmed);
    while text.len() < decimals + 1 {
        text.insert(0, '0');
    }
    let (whole, fraction) = text.split_at(text.len() - decimals);
    let mut out = String::from(sign);
    for (at, digit) in whole.chars().enumerate() {
        if at > 0 && (whole.len() - at) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    if decimals > 0 {
        out.push('.');
        out.push_str(fraction);
    }
    out
}
