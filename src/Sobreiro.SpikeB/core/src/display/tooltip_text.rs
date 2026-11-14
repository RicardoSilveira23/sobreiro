use super::flyout_model::group_thousands;
use super::When;
use crate::limits::{Limit, LimitKind};
use crate::{Time, Zone};

pub struct TooltipText;

impl TooltipText {
    /// The longest text a Windows tray tooltip can show.
    pub const MAX_LENGTH: usize = 127;

    /// "AI credits 17% · resets Dec 1 ...", the credits figures and the age, cut to `MAX_LENGTH`.
    pub fn render_copilot(credits: &Limit, now: Time, zone: Zone) -> String {
        let mut lines = vec![line(credits, now, zone)];
        if !credits.is_reset {
            if let Some(figures) = Self::credits_figures(credits) {
                lines.push(figures);
            }
        }
        lines.push(format!("as of {}", When::format(credits.observed_at, now, zone)));
        cut(lines.join("\n"))
    }

    /// "2,644 of 15,000 credits", or None without both numbers.
    pub fn credits_figures(limit: &Limit) -> Option<String> {
        let used = limit.used_credits?;
        let budget = limit.credit_budget?;
        Some(format!("{} of {} credits", grouped(used.round_ties_even()), grouped(budget.round_ties_even())))
    }

    pub fn render(limits: &[Limit], now: Time, zone: Zone, legacy_connection: bool) -> String {
        let limits: Vec<&Limit> = limits.iter().filter(|l| l.kind != LimitKind::Credits).collect();
        if limits.is_empty() {
            return if legacy_connection {
                "Sobreiro · no data yet\nStill on the old bridge: reconnect from the tray menu.".to_string()
            } else {
                "Sobreiro · no data yet".to_string()
            };
        }

        let mut lines = Vec::new();
        for kind in [LimitKind::Session, LimitKind::Weekly, LimitKind::Spend] {
            match limits.iter().find(|l| l.kind == kind) {
                None => {
                    // Session and weekly are always listed; spend only when present.
                    if kind != LimitKind::Spend {
                        lines.push(format!("{} –", if kind == LimitKind::Session { "Session" } else { "Weekly" }));
                    }
                }
                Some(l) => lines.push(line(l, now, zone)),
            }
        }
        let newest = limits.iter().map(|l| l.observed_at).max().unwrap();
        lines.push(format!("as of {}", When::format(newest, now, zone)));
        cut(lines.join("\n"))
    }
}

fn grouped(v: f64) -> String {
    let digits = format!("{}", v.abs() as i64);
    format!("{}{}", if v < 0.0 { "-" } else { "" }, group_thousands(&digits))
}

/// Cut to the tooltip's length in UTF-16 units (every character here is in the basic plane).
fn cut(text: String) -> String {
    if text.chars().count() <= TooltipText::MAX_LENGTH { text } else { text.chars().take(TooltipText::MAX_LENGTH).collect() }
}

fn line(limit: &Limit, now: Time, zone: Zone) -> String {
    let mut s = format!("{} {}%", limit.label, limit.display_percent());
    if let Some(at) = limit.resets_at {
        s.push_str(&format!(" · {} {}", if limit.is_reset { "reset" } else { "resets" }, When::format(at, now, zone)));
    }
    s
}
