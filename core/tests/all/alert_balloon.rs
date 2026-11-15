//! Ported from `tests/Sobreiro.Tests/AlertBalloonTests.cs`.
use sobreiro_core::alerts::alert_balloon;
use sobreiro_core::alerts::alert_policy::Alert;
use sobreiro_core::limits::LimitKind;

fn make(kind: LimitKind, threshold: i32, title: &str, body: &str) -> Alert {
    Alert {
        kind,
        threshold,
        title: title.to_string(),
        body: body.to_string(),
    }
}

fn balloon(title: &str, text: &str) -> Option<(String, String)> {
    Some((title.to_string(), text.to_string()))
}

#[test]
fn no_alerts_give_no_balloon() {
    assert_eq!(alert_balloon::for_alerts(&[]), None);
}

#[test]
fn one_alert_gives_its_title_and_body() {
    let result = alert_balloon::for_alerts(&[make(
        LimitKind::Session,
        80,
        "Session limit at 82%",
        "Resets 1:00 PM \u{b7} in 3h",
    )]);
    assert_eq!(
        result,
        balloon("Session limit at 82%", "Resets 1:00 PM \u{b7} in 3h")
    );
}

#[test]
fn other_alerts_add_their_titles_on_following_lines() {
    let result = alert_balloon::for_alerts(&[
        make(
            LimitKind::Session,
            95,
            "Session limit at 96%",
            "Resets 1:00 PM \u{b7} in 3h",
        ),
        make(
            LimitKind::Weekly,
            80,
            "Weekly limit at 81%",
            "Resets Fri 12:00 AM \u{b7} in 4d 14h",
        ),
        make(LimitKind::Spend, 80, "Spend limit at 80%", ""),
    ]);
    assert_eq!(
        result,
        balloon(
            "Session limit at 96%",
            "Resets 1:00 PM \u{b7} in 3h\nWeekly limit at 81%\nSpend limit at 80%"
        )
    );
}

#[test]
fn an_empty_first_body_is_left_out() {
    let result = alert_balloon::for_alerts(&[
        make(LimitKind::Spend, 95, "Spend limit at 95%", ""),
        make(
            LimitKind::Session,
            80,
            "Session limit at 80%",
            "Resets 1:00 PM \u{b7} in 3h",
        ),
    ]);
    assert_eq!(
        result,
        balloon("Spend limit at 95%", "Session limit at 80%")
    );
}

#[test]
fn a_lone_alert_without_a_body_still_has_some_text() {
    let result = alert_balloon::for_alerts(&[make(LimitKind::Spend, 80, "Spend limit at 80%", "")]);
    assert_eq!(result, balloon("Spend limit at 80%", " "));
}

// ---- oracle ----
/// The alerts (kind, threshold, title, body) and the balloon the C# made of them.
type OracleRow = (
    &'static [(&'static str, i32, &'static str, &'static str)],
    Option<(&'static str, &'static str)>,
);

fn kind_named(name: &str) -> LimitKind {
    match name {
        "Session" => LimitKind::Session,
        "Weekly" => LimitKind::Weekly,
        "Spend" => LimitKind::Spend,
        "Credits" => LimitKind::Credits,
        _ => panic!("no kind named {name}"),
    }
}

#[test]
fn for_alerts_answers_as_the_csharp_did() {
    for (alerts, expected) in ROWS {
        let alerts: Vec<Alert> = alerts
            .iter()
            .map(|&(kind, threshold, title, body)| make(kind_named(kind), threshold, title, body))
            .collect();
        assert_eq!(
            alert_balloon::for_alerts(&alerts),
            expected.map(|(title, text)| (title.to_string(), text.to_string())),
            "{alerts:?}"
        );
    }
}

#[test]
fn the_rows_hold_no_alerts_and_empty_and_multi_line_texts() {
    assert!(ROWS.iter().any(|row| row.0.is_empty() && row.1.is_none()));
    assert!(ROWS.iter().any(|row| row.1.is_some_and(|b| b.1 == " ")));
    assert!(
        ROWS.iter()
            .any(|row| row.1.is_some_and(|b| b.1.contains('\n')))
    );
    assert!(ROWS.iter().any(|row| row.0.len() == 4));
}

#[rustfmt::skip]
const ROWS: &[OracleRow] = &[
    (&[ ], None,),
    (&[ ("Session", 80, "", ""), ], Some(("", " ")),),
    (&[ ("Session", 80, "", "plain"), ], Some(("", "plain")),),
    (&[ ("Session", 80, "", "a\nb"), ], Some(("", "a\nb")),),
    (&[ ("Session", 80, "", "Ünï 🌅"), ], Some(("", "Ünï 🌅")),),
    (&[ ("Session", 80, "", " "), ], Some(("", " ")),),
    (&[ ("Session", 80, "", "\r\n"), ], Some(("", "\r\n")),),
    (&[ ("Session", 80, "", "line\n"), ], Some(("", "line\n")),),
    (&[ ("Session", 80, "", "Session 82% used"), ], Some(("", "Session 82% used")),),
    (&[ ("Session", 80, "", "x\r\ny\nz"), ], Some(("", "x\r\ny\nz")),),
    (&[ ("Session", 80, "", "  lead"), ], Some(("", "  lead")),),
    (&[ ("Session", 80, "", "\u{9}"), ], Some(("", "\u{9}")),),
    (&[ ("Session", 80, "plain", ""), ], Some(("plain", " ")),),
    (&[ ("Session", 80, "plain", "plain"), ], Some(("plain", "plain")),),
    (&[ ("Session", 80, "plain", "a\nb"), ], Some(("plain", "a\nb")),),
    (&[ ("Session", 80, "plain", "Ünï 🌅"), ], Some(("plain", "Ünï 🌅")),),
    (&[ ("Session", 80, "plain", " "), ], Some(("plain", " ")),),
    (&[ ("Session", 80, "plain", "\r\n"), ], Some(("plain", "\r\n")),),
    (&[ ("Session", 80, "plain", "line\n"), ], Some(("plain", "line\n")),),
    (&[ ("Session", 80, "plain", "Session 82% used"), ], Some(("plain", "Session 82% used")),),
    (&[ ("Session", 80, "plain", "x\r\ny\nz"), ], Some(("plain", "x\r\ny\nz")),),
    (&[ ("Session", 80, "plain", "  lead"), ], Some(("plain", "  lead")),),
    (&[ ("Session", 80, "plain", "\u{9}"), ], Some(("plain", "\u{9}")),),
    (&[ ("Session", 80, "a\nb", ""), ], Some(("a\nb", " ")),),
    (&[ ("Session", 80, "a\nb", "plain"), ], Some(("a\nb", "plain")),),
    (&[ ("Session", 80, "a\nb", "a\nb"), ], Some(("a\nb", "a\nb")),),
    (&[ ("Session", 80, "a\nb", "Ünï 🌅"), ], Some(("a\nb", "Ünï 🌅")),),
    (&[ ("Session", 80, "a\nb", " "), ], Some(("a\nb", " ")),),
    (&[ ("Session", 80, "a\nb", "\r\n"), ], Some(("a\nb", "\r\n")),),
    (&[ ("Session", 80, "a\nb", "line\n"), ], Some(("a\nb", "line\n")),),
    (&[ ("Session", 80, "a\nb", "Session 82% used"), ], Some(("a\nb", "Session 82% used")),),
    (&[ ("Session", 80, "a\nb", "x\r\ny\nz"), ], Some(("a\nb", "x\r\ny\nz")),),
    (&[ ("Session", 80, "a\nb", "  lead"), ], Some(("a\nb", "  lead")),),
    (&[ ("Session", 80, "a\nb", "\u{9}"), ], Some(("a\nb", "\u{9}")),),
    (&[ ("Session", 80, "Ünï 🌅", ""), ], Some(("Ünï 🌅", " ")),),
    (&[ ("Session", 80, "Ünï 🌅", "plain"), ], Some(("Ünï 🌅", "plain")),),
    (&[ ("Session", 80, "Ünï 🌅", "a\nb"), ], Some(("Ünï 🌅", "a\nb")),),
    (&[ ("Session", 80, "Ünï 🌅", "Ünï 🌅"), ], Some(("Ünï 🌅", "Ünï 🌅")),),
    (&[ ("Session", 80, "Ünï 🌅", " "), ], Some(("Ünï 🌅", " ")),),
    (&[ ("Session", 80, "Ünï 🌅", "\r\n"), ], Some(("Ünï 🌅", "\r\n")),),
    (&[ ("Session", 80, "Ünï 🌅", "line\n"), ], Some(("Ünï 🌅", "line\n")),),
    (&[ ("Session", 80, "Ünï 🌅", "Session 82% used"), ], Some(("Ünï 🌅", "Session 82% used")),),
    (&[ ("Session", 80, "Ünï 🌅", "x\r\ny\nz"), ], Some(("Ünï 🌅", "x\r\ny\nz")),),
    (&[ ("Session", 80, "Ünï 🌅", "  lead"), ], Some(("Ünï 🌅", "  lead")),),
    (&[ ("Session", 80, "Ünï 🌅", "\u{9}"), ], Some(("Ünï 🌅", "\u{9}")),),
    (&[ ("Session", 80, " ", ""), ], Some((" ", " ")),),
    (&[ ("Session", 80, " ", "plain"), ], Some((" ", "plain")),),
    (&[ ("Session", 80, " ", "a\nb"), ], Some((" ", "a\nb")),),
    (&[ ("Session", 80, " ", "Ünï 🌅"), ], Some((" ", "Ünï 🌅")),),
    (&[ ("Session", 80, " ", " "), ], Some((" ", " ")),),
    (&[ ("Session", 80, " ", "\r\n"), ], Some((" ", "\r\n")),),
    (&[ ("Session", 80, " ", "line\n"), ], Some((" ", "line\n")),),
    (&[ ("Session", 80, " ", "Session 82% used"), ], Some((" ", "Session 82% used")),),
    (&[ ("Session", 80, " ", "x\r\ny\nz"), ], Some((" ", "x\r\ny\nz")),),
    (&[ ("Session", 80, " ", "  lead"), ], Some((" ", "  lead")),),
    (&[ ("Session", 80, " ", "\u{9}"), ], Some((" ", "\u{9}")),),
    (&[ ("Session", 80, "\r\n", ""), ], Some(("\r\n", " ")),),
    (&[ ("Session", 80, "\r\n", "plain"), ], Some(("\r\n", "plain")),),
    (&[ ("Session", 80, "\r\n", "a\nb"), ], Some(("\r\n", "a\nb")),),
    (&[ ("Session", 80, "\r\n", "Ünï 🌅"), ], Some(("\r\n", "Ünï 🌅")),),
    (&[ ("Session", 80, "\r\n", " "), ], Some(("\r\n", " ")),),
    (&[ ("Session", 80, "\r\n", "\r\n"), ], Some(("\r\n", "\r\n")),),
    (&[ ("Session", 80, "\r\n", "line\n"), ], Some(("\r\n", "line\n")),),
    (&[ ("Session", 80, "\r\n", "Session 82% used"), ], Some(("\r\n", "Session 82% used")),),
    (&[ ("Session", 80, "\r\n", "x\r\ny\nz"), ], Some(("\r\n", "x\r\ny\nz")),),
    (&[ ("Session", 80, "\r\n", "  lead"), ], Some(("\r\n", "  lead")),),
    (&[ ("Session", 80, "\r\n", "\u{9}"), ], Some(("\r\n", "\u{9}")),),
    (&[ ("Session", 80, "line\n", ""), ], Some(("line\n", " ")),),
    (&[ ("Session", 80, "line\n", "plain"), ], Some(("line\n", "plain")),),
    (&[ ("Session", 80, "line\n", "a\nb"), ], Some(("line\n", "a\nb")),),
    (&[ ("Session", 80, "line\n", "Ünï 🌅"), ], Some(("line\n", "Ünï 🌅")),),
    (&[ ("Session", 80, "line\n", " "), ], Some(("line\n", " ")),),
    (&[ ("Session", 80, "line\n", "\r\n"), ], Some(("line\n", "\r\n")),),
    (&[ ("Session", 80, "line\n", "line\n"), ], Some(("line\n", "line\n")),),
    (&[ ("Session", 80, "line\n", "Session 82% used"), ], Some(("line\n", "Session 82% used")),),
    (&[ ("Session", 80, "line\n", "x\r\ny\nz"), ], Some(("line\n", "x\r\ny\nz")),),
    (&[ ("Session", 80, "line\n", "  lead"), ], Some(("line\n", "  lead")),),
    (&[ ("Session", 80, "line\n", "\u{9}"), ], Some(("line\n", "\u{9}")),),
    (&[ ("Session", 80, "Session 82% used", ""), ], Some(("Session 82% used", " ")),),
    (&[ ("Session", 80, "Session 82% used", "plain"), ], Some(("Session 82% used", "plain")),),
    (&[ ("Session", 80, "Session 82% used", "a\nb"), ], Some(("Session 82% used", "a\nb")),),
    (&[ ("Session", 80, "Session 82% used", "Ünï 🌅"), ], Some(("Session 82% used", "Ünï 🌅")),),
    (&[ ("Session", 80, "Session 82% used", " "), ], Some(("Session 82% used", " ")),),
    (&[ ("Session", 80, "Session 82% used", "\r\n"), ], Some(("Session 82% used", "\r\n")),),
    (&[ ("Session", 80, "Session 82% used", "line\n"), ], Some(("Session 82% used", "line\n")),),
    (&[ ("Session", 80, "Session 82% used", "Session 82% used"), ], Some(("Session 82% used", "Session 82% used")),),
    (&[ ("Session", 80, "Session 82% used", "x\r\ny\nz"), ], Some(("Session 82% used", "x\r\ny\nz")),),
    (&[ ("Session", 80, "Session 82% used", "  lead"), ], Some(("Session 82% used", "  lead")),),
    (&[ ("Session", 80, "Session 82% used", "\u{9}"), ], Some(("Session 82% used", "\u{9}")),),
    (&[ ("Session", 80, "x\r\ny\nz", ""), ], Some(("x\r\ny\nz", " ")),),
    (&[ ("Session", 80, "x\r\ny\nz", "plain"), ], Some(("x\r\ny\nz", "plain")),),
    (&[ ("Session", 80, "x\r\ny\nz", "a\nb"), ], Some(("x\r\ny\nz", "a\nb")),),
    (&[ ("Session", 80, "x\r\ny\nz", "Ünï 🌅"), ], Some(("x\r\ny\nz", "Ünï 🌅")),),
    (&[ ("Session", 80, "x\r\ny\nz", " "), ], Some(("x\r\ny\nz", " ")),),
    (&[ ("Session", 80, "x\r\ny\nz", "\r\n"), ], Some(("x\r\ny\nz", "\r\n")),),
    (&[ ("Session", 80, "x\r\ny\nz", "line\n"), ], Some(("x\r\ny\nz", "line\n")),),
    (&[ ("Session", 80, "x\r\ny\nz", "Session 82% used"), ], Some(("x\r\ny\nz", "Session 82% used")),),
    (&[ ("Session", 80, "x\r\ny\nz", "x\r\ny\nz"), ], Some(("x\r\ny\nz", "x\r\ny\nz")),),
    (&[ ("Session", 80, "x\r\ny\nz", "  lead"), ], Some(("x\r\ny\nz", "  lead")),),
    (&[ ("Session", 80, "x\r\ny\nz", "\u{9}"), ], Some(("x\r\ny\nz", "\u{9}")),),
    (&[ ("Session", 80, "  lead", ""), ], Some(("  lead", " ")),),
    (&[ ("Session", 80, "  lead", "plain"), ], Some(("  lead", "plain")),),
    (&[ ("Session", 80, "  lead", "a\nb"), ], Some(("  lead", "a\nb")),),
    (&[ ("Session", 80, "  lead", "Ünï 🌅"), ], Some(("  lead", "Ünï 🌅")),),
    (&[ ("Session", 80, "  lead", " "), ], Some(("  lead", " ")),),
    (&[ ("Session", 80, "  lead", "\r\n"), ], Some(("  lead", "\r\n")),),
    (&[ ("Session", 80, "  lead", "line\n"), ], Some(("  lead", "line\n")),),
    (&[ ("Session", 80, "  lead", "Session 82% used"), ], Some(("  lead", "Session 82% used")),),
    (&[ ("Session", 80, "  lead", "x\r\ny\nz"), ], Some(("  lead", "x\r\ny\nz")),),
    (&[ ("Session", 80, "  lead", "  lead"), ], Some(("  lead", "  lead")),),
    (&[ ("Session", 80, "  lead", "\u{9}"), ], Some(("  lead", "\u{9}")),),
    (&[ ("Session", 80, "\u{9}", ""), ], Some(("\u{9}", " ")),),
    (&[ ("Session", 80, "\u{9}", "plain"), ], Some(("\u{9}", "plain")),),
    (&[ ("Session", 80, "\u{9}", "a\nb"), ], Some(("\u{9}", "a\nb")),),
    (&[ ("Session", 80, "\u{9}", "Ünï 🌅"), ], Some(("\u{9}", "Ünï 🌅")),),
    (&[ ("Session", 80, "\u{9}", " "), ], Some(("\u{9}", " ")),),
    (&[ ("Session", 80, "\u{9}", "\r\n"), ], Some(("\u{9}", "\r\n")),),
    (&[ ("Session", 80, "\u{9}", "line\n"), ], Some(("\u{9}", "line\n")),),
    (&[ ("Session", 80, "\u{9}", "Session 82% used"), ], Some(("\u{9}", "Session 82% used")),),
    (&[ ("Session", 80, "\u{9}", "x\r\ny\nz"), ], Some(("\u{9}", "x\r\ny\nz")),),
    (&[ ("Session", 80, "\u{9}", "  lead"), ], Some(("\u{9}", "  lead")),),
    (&[ ("Session", 80, "\u{9}", "\u{9}"), ], Some(("\u{9}", "\u{9}")),),
    (&[ ("Weekly", 95, "\u{9}", "\u{9}"), ("Credits", 80, "", "\r\n"), ("Spend", 80, "line\n", "line\n"), ("Spend", 95, "plain", "a\nb"), ], Some(("\u{9}", "\u{9}\n\nline\n\nplain")),),
    (&[ ("Credits", 80, "", "x\r\ny\nz"), ("Session", 95, "a\nb", ""), ("Session", 80, "x\r\ny\nz", "plain"), ("Spend", 95, "a\nb", "\u{9}"), ], Some(("", "x\r\ny\nz\na\nb\nx\r\ny\nz\na\nb")),),
    (&[ ("Session", 95, "  lead", "\r\n"), ("Spend", 95, "plain", "plain"), ("Credits", 95, "line\n", "\r\n"), ], Some(("  lead", "\r\n\nplain\nline\n")),),
    (&[ ("Spend", 80, "Ünï 🌅", " "), ("Spend", 80, "line\n", "a\nb"), ("Weekly", 95, "x\r\ny\nz", "\u{9}"), ], Some(("Ünï 🌅", " \nline\n\nx\r\ny\nz")),),
    (&[ ("Session", 80, " ", ""), ("Weekly", 80, "Session 82% used", "\u{9}"), ("Credits", 80, " ", "a\nb"), ("Session", 95, "\u{9}", "line\n"), ], Some((" ", "Session 82% used\n \n\u{9}")),),
    (&[ ("Weekly", 95, "\r\n", "\r\n"), ("Spend", 80, "x\r\ny\nz", "a\nb"), ], Some(("\r\n", "\r\n\nx\r\ny\nz")),),
    (&[ ("Session", 80, "Ünï 🌅", "Session 82% used"), ("Session", 80, "line\n", "\u{9}"), ], Some(("Ünï 🌅", "Session 82% used\nline\n")),),
    (&[ ("Credits", 95, "\u{9}", "a\nb"), ("Weekly", 95, "Session 82% used", "  lead"), ("Session", 95, "a\nb", "\r\n"), ], Some(("\u{9}", "a\nb\nSession 82% used\na\nb")),),
    (&[ ("Spend", 95, " ", "a\nb"), ("Session", 80, "a\nb", "Ünï 🌅"), ("Credits", 95, "  lead", "plain"), ], Some((" ", "a\nb\na\nb\n  lead")),),
    (&[ ("Weekly", 95, "", "plain"), ("Weekly", 95, "plain", "  lead"), ], Some(("", "plain\nplain")),),
    (&[ ("Credits", 80, "a\nb", "Ünï 🌅"), ("Spend", 95, "  lead", "Ünï 🌅"), ], Some(("a\nb", "Ünï 🌅\n  lead")),),
    (&[ ("Credits", 95, " ", "line\n"), ("Spend", 80, "\u{9}", ""), ("Session", 95, "Ünï 🌅", ""), ("Session", 80, "", ""), ], Some((" ", "line\n\n\u{9}\nÜnï 🌅\n")),),
    (&[ ("Credits", 95, "\u{9}", "Session 82% used"), ("Spend", 80, "x\r\ny\nz", "line\n"), ("Credits", 80, "", "x\r\ny\nz"), ], Some(("\u{9}", "Session 82% used\nx\r\ny\nz\n")),),
    (&[ ("Weekly", 80, "\r\n", "\u{9}"), ("Credits", 80, "x\r\ny\nz", "Session 82% used"), ("Session", 95, "a\nb", "a\nb"), ("Weekly", 80, "line\n", ""), ], Some(("\r\n", "\u{9}\nx\r\ny\nz\na\nb\nline\n")),),
    (&[ ("Session", 80, "Ünï 🌅", ""), ("Credits", 80, " ", ""), ("Weekly", 80, "Session 82% used", ""), ("Credits", 80, "  lead", "a\nb"), ], Some(("Ünï 🌅", " \nSession 82% used\n  lead")),),
    (&[ ("Weekly", 95, "\r\n", "plain"), ("Session", 95, "a\nb", "  lead"), ("Credits", 80, "Session 82% used", "\r\n"), ], Some(("\r\n", "plain\na\nb\nSession 82% used")),),
    (&[ ("Credits", 80, "\r\n", "plain"), ("Weekly", 95, " ", "Session 82% used"), ], Some(("\r\n", "plain\n ")),),
    (&[ ("Weekly", 80, "  lead", "\r\n"), ("Session", 80, "Session 82% used", "a\nb"), ("Credits", 80, "\u{9}", "\u{9}"), ], Some(("  lead", "\r\n\nSession 82% used\n\u{9}")),),
    (&[ ("Spend", 95, "\r\n", "  lead"), ("Weekly", 80, "a\nb", "  lead"), ], Some(("\r\n", "  lead\na\nb")),),
    (&[ ("Spend", 95, "plain", "Ünï 🌅"), ("Weekly", 95, "Session 82% used", "Session 82% used"), ], Some(("plain", "Ünï 🌅\nSession 82% used")),),
    (&[ ("Credits", 95, "Session 82% used", "\r\n"), ("Weekly", 80, "Session 82% used", " "), ("Session", 80, "x\r\ny\nz", "a\nb"), ("Weekly", 80, "Ünï 🌅", "  lead"), ], Some(("Session 82% used", "\r\n\nSession 82% used\nx\r\ny\nz\nÜnï 🌅")),),
    (&[ ("Credits", 95, "\r\n", " "), ("Weekly", 80, "x\r\ny\nz", " "), ("Weekly", 80, " ", "  lead"), ], Some(("\r\n", " \nx\r\ny\nz\n ")),),
    (&[ ("Session", 95, "", "Ünï 🌅"), ("Session", 80, " ", "a\nb"), ("Session", 80, "a\nb", ""), ("Credits", 80, "plain", "x\r\ny\nz"), ], Some(("", "Ünï 🌅\n \na\nb\nplain")),),
    (&[ ("Session", 95, " ", "\u{9}"), ("Spend", 80, "a\nb", "line\n"), ], Some((" ", "\u{9}\na\nb")),),
    (&[ ("Weekly", 95, "", "  lead"), ("Credits", 80, "\r\n", " "), ("Session", 80, "a\nb", ""), ], Some(("", "  lead\n\r\n\na\nb")),),
    (&[ ("Session", 80, "Ünï 🌅", "Session 82% used"), ("Weekly", 80, "Ünï 🌅", "Ünï 🌅"), ], Some(("Ünï 🌅", "Session 82% used\nÜnï 🌅")),),
    (&[ ("Spend", 95, "a\nb", "a\nb"), ("Spend", 80, "Session 82% used", "\r\n"), ("Weekly", 95, "x\r\ny\nz", "Session 82% used"), ], Some(("a\nb", "a\nb\nSession 82% used\nx\r\ny\nz")),),
    (&[ ("Weekly", 80, "Ünï 🌅", "a\nb"), ("Credits", 95, "line\n", ""), ("Credits", 95, "a\nb", "  lead"), ("Spend", 95, "Session 82% used", "  lead"), ], Some(("Ünï 🌅", "a\nb\nline\n\na\nb\nSession 82% used")),),
    (&[ ("Credits", 80, "plain", "\u{9}"), ("Weekly", 80, "Session 82% used", "x\r\ny\nz"), ("Spend", 80, "Session 82% used", "\r\n"), ("Credits", 80, "Ünï 🌅", "line\n"), ], Some(("plain", "\u{9}\nSession 82% used\nSession 82% used\nÜnï 🌅")),),
    (&[ ("Session", 80, " ", "plain"), ("Weekly", 95, "x\r\ny\nz", "Ünï 🌅"), ], Some((" ", "plain\nx\r\ny\nz")),),
    (&[ ("Session", 80, "Ünï 🌅", "  lead"), ("Spend", 80, "line\n", "x\r\ny\nz"), ("Credits", 80, "line\n", "\u{9}"), ("Session", 95, "Session 82% used", " "), ], Some(("Ünï 🌅", "  lead\nline\n\nline\n\nSession 82% used")),),
    (&[ ("Weekly", 95, "\r\n", "plain"), ("Session", 95, "a\nb", "line\n"), ], Some(("\r\n", "plain\na\nb")),),
    (&[ ("Spend", 95, "line\n", "Session 82% used"), ("Weekly", 80, "line\n", "line\n"), ("Weekly", 95, "\r\n", "x\r\ny\nz"), ("Spend", 95, "\u{9}", ""), ], Some(("line\n", "Session 82% used\nline\n\n\r\n\n\u{9}")),),
    (&[ ("Credits", 80, "a\nb", "  lead"), ("Weekly", 95, "Session 82% used", ""), ("Weekly", 95, "  lead", "Session 82% used"), ], Some(("a\nb", "  lead\nSession 82% used\n  lead")),),
    (&[ ("Session", 80, "Ünï 🌅", "line\n"), ("Weekly", 80, "a\nb", "plain"), ("Session", 80, " ", "line\n"), ("Weekly", 80, "\u{9}", "\r\n"), ], Some(("Ünï 🌅", "line\n\na\nb\n \n\u{9}")),),
    (&[ ("Session", 95, "", "line\n"), ("Credits", 80, "Ünï 🌅", ""), ("Session", 95, "  lead", "Session 82% used"), ("Session", 80, "Session 82% used", "  lead"), ], Some(("", "line\n\nÜnï 🌅\n  lead\nSession 82% used")),),
    (&[ ("Weekly", 80, "\r\n", "  lead"), ("Session", 95, "x\r\ny\nz", "\r\n"), ("Credits", 95, "  lead", "Ünï 🌅"), ], Some(("\r\n", "  lead\nx\r\ny\nz\n  lead")),),
    (&[ ("Spend", 95, "\r\n", "Ünï 🌅"), ("Spend", 80, "line\n", " "), ], Some(("\r\n", "Ünï 🌅\nline\n")),),
    (&[ ("Credits", 95, "line\n", " "), ("Weekly", 80, "\r\n", "  lead"), ], Some(("line\n", " \n\r\n")),),
    (&[ ("Spend", 95, "\u{9}", "line\n"), ("Spend", 95, "plain", "\u{9}"), ("Spend", 80, "", "plain"), ("Weekly", 80, "Session 82% used", "\u{9}"), ], Some(("\u{9}", "line\n\nplain\n\nSession 82% used")),),
    (&[ ("Session", 80, "x\r\ny\nz", "x\r\ny\nz"), ("Weekly", 80, "", "a\nb"), ], Some(("x\r\ny\nz", "x\r\ny\nz\n")),),
    (&[ ("Spend", 80, "  lead", "line\n"), ("Weekly", 80, "\u{9}", "plain"), ("Session", 95, "a\nb", "plain"), ], Some(("  lead", "line\n\n\u{9}\na\nb")),),
    (&[ ("Weekly", 80, "x\r\ny\nz", "Session 82% used"), ("Weekly", 80, "plain", "line\n"), ], Some(("x\r\ny\nz", "Session 82% used\nplain")),),
    (&[ ("Credits", 80, "plain", "a\nb"), ("Weekly", 80, "\u{9}", " "), ("Weekly", 95, " ", "a\nb"), ], Some(("plain", "a\nb\n\u{9}\n ")),),
    (&[ ("Credits", 80, "plain", "x\r\ny\nz"), ("Session", 80, "a\nb", "a\nb"), ("Session", 95, "", ""), ], Some(("plain", "x\r\ny\nz\na\nb\n")),),
    (&[ ("Spend", 95, "Session 82% used", "\u{9}"), ("Spend", 80, "  lead", "\u{9}"), ("Weekly", 80, " ", "a\nb"), ("Spend", 95, "plain", "x\r\ny\nz"), ], Some(("Session 82% used", "\u{9}\n  lead\n \nplain")),),
    (&[ ("Weekly", 95, "  lead", ""), ("Weekly", 95, "line\n", "\r\n"), ("Session", 95, "Ünï 🌅", "Ünï 🌅"), ], Some(("  lead", "line\n\nÜnï 🌅")),),
    (&[ ("Spend", 80, "a\nb", "line\n"), ("Spend", 95, "x\r\ny\nz", "Ünï 🌅"), ("Session", 80, " ", "\r\n"), ("Weekly", 95, "Session 82% used", "line\n"), ], Some(("a\nb", "line\n\nx\r\ny\nz\n \nSession 82% used")),),
    (&[ ("Weekly", 80, "\r\n", "Session 82% used"), ("Weekly", 80, "plain", "Session 82% used"), ("Session", 80, "", " "), ], Some(("\r\n", "Session 82% used\nplain\n")),),
    (&[ ("Session", 95, "Session 82% used", "\r\n"), ("Spend", 80, "\u{9}", "plain"), ("Session", 80, "", "\u{9}"), ], Some(("Session 82% used", "\r\n\n\u{9}\n")),),
    (&[ ("Weekly", 80, "line\n", "\u{9}"), ("Credits", 95, "\u{9}", "\u{9}"), ], Some(("line\n", "\u{9}\n\u{9}")),),
    (&[ ("Session", 95, "plain", "a\nb"), ("Credits", 80, "\u{9}", " "), ("Spend", 95, "\u{9}", "x\r\ny\nz"), ], Some(("plain", "a\nb\n\u{9}\n\u{9}")),),
    (&[ ("Credits", 95, "  lead", ""), ("Weekly", 95, "a\nb", "plain"), ], Some(("  lead", "a\nb")),),
    (&[ ("Weekly", 95, " ", "  lead"), ("Spend", 80, "plain", "Ünï 🌅"), ], Some((" ", "  lead\nplain")),),
    (&[ ("Weekly", 95, "\r\n", " "), ("Credits", 80, " ", " "), ], Some(("\r\n", " \n ")),),
    (&[ ("Session", 95, " ", " "), ("Session", 80, "a\nb", "Session 82% used"), ("Credits", 80, "\u{9}", "\r\n"), ("Weekly", 95, "\r\n", "plain"), ], Some((" ", " \na\nb\n\u{9}\n\r\n")),),
    (&[ ("Weekly", 95, "", "\r\n"), ("Credits", 80, " ", "\r\n"), ("Weekly", 80, "x\r\ny\nz", " "), ], Some(("", "\r\n\n \nx\r\ny\nz")),),
    (&[ ("Spend", 95, "\u{9}", "Session 82% used"), ("Session", 80, "line\n", ""), ("Spend", 80, "Session 82% used", ""), ("Weekly", 80, "Ünï 🌅", "Session 82% used"), ], Some(("\u{9}", "Session 82% used\nline\n\nSession 82% used\nÜnï 🌅")),),
    (&[ ("Credits", 95, "a\nb", "\r\n"), ("Credits", 95, " ", " "), ], Some(("a\nb", "\r\n\n ")),),
    (&[ ("Spend", 95, "\u{9}", "line\n"), ("Session", 80, "a\nb", "  lead"), ("Spend", 80, "\r\n", "Session 82% used"), ], Some(("\u{9}", "line\n\na\nb\n\r\n")),),
    (&[ ("Session", 95, " ", "x\r\ny\nz"), ("Credits", 80, "\r\n", "\r\n"), ("Session", 80, "x\r\ny\nz", " "), ("Weekly", 95, "plain", " "), ], Some((" ", "x\r\ny\nz\n\r\n\nx\r\ny\nz\nplain")),),
    (&[ ("Session", 95, "Ünï 🌅", "Session 82% used"), ("Credits", 95, "plain", ""), ("Credits", 80, "x\r\ny\nz", "\r\n"), ], Some(("Ünï 🌅", "Session 82% used\nplain\nx\r\ny\nz")),),
    (&[ ("Session", 80, "Ünï 🌅", "plain"), ("Spend", 95, "plain", "plain"), ("Credits", 80, "\u{9}", "  lead"), ], Some(("Ünï 🌅", "plain\nplain\n\u{9}")),),
    (&[ ("Credits", 80, "a\nb", "x\r\ny\nz"), ("Spend", 80, "\r\n", "\u{9}"), ], Some(("a\nb", "x\r\ny\nz\n\r\n")),),
    (&[ ("Credits", 80, "", "\r\n"), ("Credits", 80, "x\r\ny\nz", "  lead"), ], Some(("", "\r\n\nx\r\ny\nz")),),
    (&[ ("Spend", 80, "x\r\ny\nz", ""), ("Spend", 80, "a\nb", "plain"), ], Some(("x\r\ny\nz", "a\nb")),),
    (&[ ("Weekly", 80, "Ünï 🌅", "\r\n"), ("Weekly", 95, " ", "Session 82% used"), ("Credits", 95, "plain", "line\n"), ], Some(("Ünï 🌅", "\r\n\n \nplain")),),
    (&[ ("Credits", 80, "  lead", "Ünï 🌅"), ("Credits", 80, "a\nb", "x\r\ny\nz"), ("Credits", 80, "  lead", "  lead"), ("Weekly", 95, "line\n", "  lead"), ], Some(("  lead", "Ünï 🌅\na\nb\n  lead\nline\n")),),
    (&[ ("Session", 95, "\r\n", "line\n"), ("Session", 80, "  lead", " "), ("Session", 80, "", "a\nb"), ], Some(("\r\n", "line\n\n  lead\n")),),
    (&[ ("Credits", 95, "line\n", "\u{9}"), ("Session", 80, "plain", "a\nb"), ("Session", 80, "  lead", "Session 82% used"), ], Some(("line\n", "\u{9}\nplain\n  lead")),),
    (&[ ("Session", 80, "line\n", ""), ("Session", 95, "  lead", "a\nb"), ("Spend", 95, "", ""), ], Some(("line\n", "  lead\n")),),
    (&[ ("Session", 95, "  lead", "x\r\ny\nz"), ("Credits", 80, "\r\n", "\r\n"), ("Weekly", 80, "Ünï 🌅", "Ünï 🌅"), ("Spend", 80, "Session 82% used", " "), ], Some(("  lead", "x\r\ny\nz\n\r\n\nÜnï 🌅\nSession 82% used")),),
    (&[ ("Weekly", 95, "\u{9}", "Session 82% used"), ("Weekly", 80, "  lead", "plain"), ], Some(("\u{9}", "Session 82% used\n  lead")),),
    (&[ ("Session", 80, "", "Ünï 🌅"), ("Session", 95, "\u{9}", "a\nb"), ], Some(("", "Ünï 🌅\n\u{9}")),),
    (&[ ("Session", 80, "Ünï 🌅", "Ünï 🌅"), ("Spend", 80, "\u{9}", "x\r\ny\nz"), ], Some(("Ünï 🌅", "Ünï 🌅\n\u{9}")),),
    (&[ ("Weekly", 95, "Ünï 🌅", "\r\n"), ("Session", 95, "\r\n", "plain"), ], Some(("Ünï 🌅", "\r\n\n\r\n")),),
    (&[ ("Session", 80, "Session 82% used", "\r\n"), ("Session", 95, "line\n", "\u{9}"), ("Session", 80, "", "\u{9}"), ], Some(("Session 82% used", "\r\n\nline\n\n")),),
    (&[ ("Weekly", 80, "plain", "line\n"), ("Session", 95, "", "  lead"), ("Credits", 95, "\r\n", "a\nb"), ("Spend", 80, "  lead", "Session 82% used"), ], Some(("plain", "line\n\n\n\r\n\n  lead")),),
    (&[ ("Weekly", 95, "", "plain"), ("Credits", 80, "line\n", "Session 82% used"), ], Some(("", "plain\nline\n")),),
    (&[ ("Spend", 95, "x\r\ny\nz", "\u{9}"), ("Spend", 95, "Ünï 🌅", "\r\n"), ], Some(("x\r\ny\nz", "\u{9}\nÜnï 🌅")),),
    (&[ ("Credits", 80, "line\n", "Session 82% used"), ("Spend", 80, "\r\n", "Ünï 🌅"), ("Weekly", 80, "Ünï 🌅", "\r\n"), ], Some(("line\n", "Session 82% used\n\r\n\nÜnï 🌅")),),
    (&[ ("Spend", 80, "\u{9}", "plain"), ("Session", 95, "Session 82% used", "line\n"), ("Weekly", 95, "  lead", "  lead"), ], Some(("\u{9}", "plain\nSession 82% used\n  lead")),),
    (&[ ("Weekly", 80, "  lead", "plain"), ("Spend", 95, "", ""), ], Some(("  lead", "plain\n")),),
    (&[ ("Weekly", 95, "line\n", ""), ("Credits", 95, "\r\n", "Session 82% used"), ], Some(("line\n", "\r\n")),),
    (&[ ("Weekly", 95, "a\nb", ""), ("Spend", 80, " ", "line\n"), ("Weekly", 95, "x\r\ny\nz", " "), ], Some(("a\nb", " \nx\r\ny\nz")),),
    (&[ ("Spend", 95, "\r\n", "x\r\ny\nz"), ("Weekly", 80, "Ünï 🌅", " "), ("Weekly", 80, "Ünï 🌅", "line\n"), ("Credits", 80, "line\n", "plain"), ], Some(("\r\n", "x\r\ny\nz\nÜnï 🌅\nÜnï 🌅\nline\n")),),
    (&[ ("Session", 80, "Session 82% used", "plain"), ("Spend", 80, "", "a\nb"), ("Spend", 95, "Session 82% used", "plain"), ("Spend", 80, "a\nb", "line\n"), ], Some(("Session 82% used", "plain\n\nSession 82% used\na\nb")),),
    (&[ ("Credits", 95, "line\n", "a\nb"), ("Session", 80, " ", "\u{9}"), ("Session", 80, "  lead", "\r\n"), ], Some(("line\n", "a\nb\n \n  lead")),),
    (&[ ("Credits", 95, " ", " "), ("Credits", 95, "x\r\ny\nz", "x\r\ny\nz"), ("Session", 95, "\r\n", "\r\n"), ], Some((" ", " \nx\r\ny\nz\n\r\n")),),
    (&[ ("Spend", 95, "Ünï 🌅", "x\r\ny\nz"), ("Weekly", 80, "\u{9}", "line\n"), ("Weekly", 95, "line\n", "x\r\ny\nz"), ("Credits", 95, "Ünï 🌅", "plain"), ], Some(("Ünï 🌅", "x\r\ny\nz\n\u{9}\nline\n\nÜnï 🌅")),),
    (&[ ("Spend", 80, "x\r\ny\nz", "\u{9}"), ("Credits", 95, "x\r\ny\nz", "a\nb"), ("Weekly", 80, "  lead", "line\n"), ], Some(("x\r\ny\nz", "\u{9}\nx\r\ny\nz\n  lead")),),
    (&[ ("Credits", 80, "Session 82% used", "a\nb"), ("Weekly", 95, "Session 82% used", "a\nb"), ("Session", 95, "\r\n", "\u{9}"), ("Spend", 80, "  lead", "line\n"), ], Some(("Session 82% used", "a\nb\nSession 82% used\n\r\n\n  lead")),),
    (&[ ("Credits", 80, "line\n", "Session 82% used"), ("Session", 80, "Session 82% used", "\u{9}"), ("Spend", 95, "\r\n", "\r\n"), ("Credits", 95, "", "\u{9}"), ], Some(("line\n", "Session 82% used\nSession 82% used\n\r\n\n")),),
    (&[ ("Credits", 80, "", "Session 82% used"), ("Weekly", 80, "\r\n", "a\nb"), ("Spend", 80, "Ünï 🌅", ""), ], Some(("", "Session 82% used\n\r\n\nÜnï 🌅")),),
    (&[ ("Credits", 95, "Session 82% used", "line\n"), ("Spend", 80, "", "plain"), ("Credits", 95, "\u{9}", "Ünï 🌅"), ], Some(("Session 82% used", "line\n\n\n\u{9}")),),
    (&[ ("Session", 80, "x\r\ny\nz", "a\nb"), ("Session", 80, "\u{9}", ""), ("Session", 80, "\r\n", "plain"), ("Weekly", 95, "plain", " "), ], Some(("x\r\ny\nz", "a\nb\n\u{9}\n\r\n\nplain")),),
    (&[ ("Session", 80, "\u{9}", "plain"), ("Credits", 95, "  lead", "a\nb"), ("Session", 95, "\u{9}", " "), ], Some(("\u{9}", "plain\n  lead\n\u{9}")),),
    (&[ ("Credits", 80, " ", "Session 82% used"), ("Weekly", 80, "\r\n", " "), ], Some((" ", "Session 82% used\n\r\n")),),
    (&[ ("Credits", 80, "\r\n", "x\r\ny\nz"), ("Spend", 80, " ", "plain"), ], Some(("\r\n", "x\r\ny\nz\n ")),),
    (&[ ("Credits", 95, "Ünï 🌅", "Session 82% used"), ("Spend", 80, "Session 82% used", "plain"), ("Spend", 95, "\u{9}", "\u{9}"), ], Some(("Ünï 🌅", "Session 82% used\nSession 82% used\n\u{9}")),),
    (&[ ("Session", 95, "  lead", "line\n"), ("Weekly", 95, "x\r\ny\nz", "\r\n"), ("Credits", 95, "", "x\r\ny\nz"), ("Weekly", 95, "Session 82% used", "plain"), ], Some(("  lead", "line\n\nx\r\ny\nz\n\nSession 82% used")),),
    (&[ ("Credits", 95, "\u{9}", "plain"), ("Session", 95, "x\r\ny\nz", ""), ("Weekly", 80, "", "Ünï 🌅"), ("Spend", 95, " ", "Ünï 🌅"), ], Some(("\u{9}", "plain\nx\r\ny\nz\n\n ")),),
    (&[ ("Credits", 95, "  lead", ""), ("Spend", 95, " ", ""), ], Some(("  lead", " ")),),
    (&[ ("Credits", 80, "\u{9}", "plain"), ("Weekly", 95, "\u{9}", "Session 82% used"), ], Some(("\u{9}", "plain\n\u{9}")),),
    (&[ ("Session", 95, "Session 82% used", ""), ("Session", 80, "\r\n", "\u{9}"), ], Some(("Session 82% used", "\r\n")),),
    (&[ ("Spend", 80, "", "a\nb"), ("Spend", 80, "Ünï 🌅", "\u{9}"), ("Credits", 80, "\u{9}", ""), ], Some(("", "a\nb\nÜnï 🌅\n\u{9}")),),
    (&[ ("Spend", 80, "  lead", "x\r\ny\nz"), ("Weekly", 80, "Ünï 🌅", "Session 82% used"), ], Some(("  lead", "x\r\ny\nz\nÜnï 🌅")),),
    (&[ ("Weekly", 80, "line\n", "Ünï 🌅"), ("Weekly", 80, "\u{9}", "  lead"), ], Some(("line\n", "Ünï 🌅\n\u{9}")),),
    (&[ ("Weekly", 95, "plain", "Session 82% used"), ("Credits", 80, "Session 82% used", "Session 82% used"), ("Spend", 95, " ", "plain"), ("Spend", 95, "", ""), ], Some(("plain", "Session 82% used\nSession 82% used\n \n")),),
    (&[ ("Spend", 80, "a\nb", "line\n"), ("Weekly", 95, "  lead", "line\n"), ("Credits", 80, "line\n", "Ünï 🌅"), ], Some(("a\nb", "line\n\n  lead\nline\n")),),
    (&[ ("Credits", 80, "", "\r\n"), ("Weekly", 80, "\u{9}", "  lead"), ("Weekly", 95, "plain", "x\r\ny\nz"), ("Spend", 80, "a\nb", "\u{9}"), ], Some(("", "\r\n\n\u{9}\nplain\na\nb")),),
    (&[ ("Weekly", 95, "a\nb", "plain"), ("Credits", 95, "line\n", "plain"), ("Session", 80, "a\nb", ""), ("Credits", 80, "", " "), ], Some(("a\nb", "plain\nline\n\na\nb\n")),),
    (&[ ("Session", 80, "plain", "plain"), ("Session", 95, "", "\r\n"), ("Weekly", 80, "", "  lead"), ], Some(("plain", "plain\n\n")),),
    (&[ ("Weekly", 95, "Session 82% used", "plain"), ("Spend", 80, "", "  lead"), ("Credits", 80, " ", "Ünï 🌅"), ("Weekly", 80, "Session 82% used", "x\r\ny\nz"), ], Some(("Session 82% used", "plain\n\n \nSession 82% used")),),
    (&[ ("Spend", 80, " ", ""), ("Weekly", 95, "  lead", "line\n"), ], Some((" ", "  lead")),),
    (&[ ("Credits", 80, "line\n", "\r\n"), ("Weekly", 95, "Ünï 🌅", "Ünï 🌅"), ], Some(("line\n", "\r\n\nÜnï 🌅")),),
    (&[ ("Spend", 80, "  lead", " "), ("Credits", 80, "Session 82% used", ""), ("Session", 95, "x\r\ny\nz", "x\r\ny\nz"), ("Session", 80, "Session 82% used", ""), ], Some(("  lead", " \nSession 82% used\nx\r\ny\nz\nSession 82% used")),),
    (&[ ("Credits", 80, "x\r\ny\nz", "  lead"), ("Spend", 95, "line\n", " "), ("Session", 95, "a\nb", " "), ("Session", 95, "", ""), ], Some(("x\r\ny\nz", "  lead\nline\n\na\nb\n")),),
    (&[ ("Session", 80, " ", "a\nb"), ("Session", 80, "Ünï 🌅", "Ünï 🌅"), ("Credits", 80, "x\r\ny\nz", " "), ], Some((" ", "a\nb\nÜnï 🌅\nx\r\ny\nz")),),
    (&[ ("Credits", 95, " ", "line\n"), ("Spend", 95, "line\n", "  lead"), ("Credits", 95, "Ünï 🌅", ""), ("Credits", 80, " ", "Session 82% used"), ], Some((" ", "line\n\nline\n\nÜnï 🌅\n ")),),
    (&[ ("Credits", 95, "x\r\ny\nz", "line\n"), ("Weekly", 80, "Session 82% used", "\r\n"), ("Credits", 80, " ", "\r\n"), ("Credits", 95, "x\r\ny\nz", ""), ], Some(("x\r\ny\nz", "line\n\nSession 82% used\n \nx\r\ny\nz")),),
    (&[ ("Session", 80, "\u{9}", "\r\n"), ("Weekly", 80, "\r\n", "x\r\ny\nz"), ("Session", 95, "plain", "a\nb"), ], Some(("\u{9}", "\r\n\n\r\n\nplain")),),
    (&[ ("Spend", 95, "Session 82% used", "\u{9}"), ("Weekly", 80, "line\n", "plain"), ], Some(("Session 82% used", "\u{9}\nline\n")),),
    (&[ ("Spend", 95, "plain", "x\r\ny\nz"), ("Weekly", 80, " ", "plain"), ("Credits", 95, "\u{9}", "\r\n"), ], Some(("plain", "x\r\ny\nz\n \n\u{9}")),),
    (&[ ("Session", 95, "\r\n", "\r\n"), ("Weekly", 80, "  lead", "plain"), ], Some(("\r\n", "\r\n\n  lead")),),
    (&[ ("Spend", 80, "\r\n", "x\r\ny\nz"), ("Spend", 95, "\u{9}", "Ünï 🌅"), ("Spend", 80, "a\nb", "line\n"), ], Some(("\r\n", "x\r\ny\nz\n\u{9}\na\nb")),),
    (&[ ("Session", 80, "line\n", "x\r\ny\nz"), ("Weekly", 80, "", "  lead"), ("Spend", 80, "x\r\ny\nz", "Ünï 🌅"), ("Credits", 80, " ", "  lead"), ], Some(("line\n", "x\r\ny\nz\n\nx\r\ny\nz\n ")),),
    (&[ ("Weekly", 80, " ", "\u{9}"), ("Weekly", 95, "a\nb", "Session 82% used"), ("Spend", 95, "a\nb", "line\n"), ], Some((" ", "\u{9}\na\nb\na\nb")),),
    (&[ ("Weekly", 80, "x\r\ny\nz", "Ünï 🌅"), ("Weekly", 80, "\u{9}", "Session 82% used"), ("Credits", 80, "plain", "x\r\ny\nz"), ("Credits", 95, "Session 82% used", " "), ], Some(("x\r\ny\nz", "Ünï 🌅\n\u{9}\nplain\nSession 82% used")),),
    (&[ ("Credits", 95, "\r\n", ""), ("Session", 95, "a\nb", "x\r\ny\nz"), ("Credits", 80, "Ünï 🌅", "plain"), ("Spend", 95, "  lead", "a\nb"), ], Some(("\r\n", "a\nb\nÜnï 🌅\n  lead")),),
    (&[ ("Credits", 95, "line\n", " "), ("Spend", 80, "\u{9}", "\u{9}"), ("Spend", 95, "a\nb", "a\nb"), ], Some(("line\n", " \n\u{9}\na\nb")),),
    (&[ ("Session", 95, "Ünï 🌅", "\u{9}"), ("Spend", 95, "plain", "\u{9}"), ("Weekly", 80, "line\n", "line\n"), ], Some(("Ünï 🌅", "\u{9}\nplain\nline\n")),),
    (&[ ("Spend", 95, "line\n", "plain"), ("Spend", 80, "line\n", "Ünï 🌅"), ("Session", 80, "  lead", "Ünï 🌅"), ], Some(("line\n", "plain\nline\n\n  lead")),),
    (&[ ("Weekly", 95, "", " "), ("Credits", 95, "line\n", "line\n"), ], Some(("", " \nline\n")),),
    (&[ ("Credits", 95, "x\r\ny\nz", "plain"), ("Session", 80, "x\r\ny\nz", " "), ("Session", 80, "  lead", ""), ], Some(("x\r\ny\nz", "plain\nx\r\ny\nz\n  lead")),),
    (&[ ("Weekly", 80, " ", "Ünï 🌅"), ("Credits", 95, "", ""), ("Weekly", 80, "line\n", "\r\n"), ], Some((" ", "Ünï 🌅\n\nline\n")),),
    (&[ ("Spend", 95, "", "\r\n"), ("Spend", 80, "\u{9}", "Ünï 🌅"), ("Credits", 95, " ", " "), ], Some(("", "\r\n\n\u{9}\n ")),),
    (&[ ("Weekly", 95, "line\n", "plain"), ("Credits", 80, "a\nb", "x\r\ny\nz"), ], Some(("line\n", "plain\na\nb")),),
    (&[ ("Spend", 95, "line\n", "\u{9}"), ("Credits", 80, " ", "\r\n"), ], Some(("line\n", "\u{9}\n ")),),
    (&[ ("Session", 95, "x\r\ny\nz", "line\n"), ("Credits", 80, "line\n", " "), ("Credits", 80, "a\nb", "line\n"), ], Some(("x\r\ny\nz", "line\n\nline\n\na\nb")),),
    (&[ ("Session", 95, "  lead", "a\nb"), ("Spend", 80, "a\nb", "Session 82% used"), ("Session", 80, "Session 82% used", "Ünï 🌅"), ], Some(("  lead", "a\nb\na\nb\nSession 82% used")),),
    (&[ ("Session", 95, "line\n", "plain"), ("Spend", 95, "Ünï 🌅", " "), ], Some(("line\n", "plain\nÜnï 🌅")),),
    (&[ ("Spend", 80, "  lead", "line\n"), ("Session", 80, " ", "Session 82% used"), ], Some(("  lead", "line\n\n ")),),
    (&[ ("Spend", 95, "", "\u{9}"), ("Spend", 80, "Session 82% used", "\u{9}"), ], Some(("", "\u{9}\nSession 82% used")),),
    (&[ ("Session", 95, "", " "), ("Session", 80, "x\r\ny\nz", " "), ("Spend", 95, "Session 82% used", "\r\n"), ("Weekly", 80, "Ünï 🌅", "  lead"), ], Some(("", " \nx\r\ny\nz\nSession 82% used\nÜnï 🌅")),),
    (&[ ("Weekly", 80, "\u{9}", "a\nb"), ("Credits", 80, " ", ""), ], Some(("\u{9}", "a\nb\n ")),),
    (&[ ("Spend", 80, "plain", "a\nb"), ("Weekly", 80, "line\n", "\u{9}"), ], Some(("plain", "a\nb\nline\n")),),
    (&[ ("Spend", 80, "Ünï 🌅", "Ünï 🌅"), ("Spend", 95, "  lead", "x\r\ny\nz"), ], Some(("Ünï 🌅", "Ünï 🌅\n  lead")),),
    (&[ ("Credits", 95, "Ünï 🌅", "Ünï 🌅"), ("Credits", 80, "\r\n", "x\r\ny\nz"), ("Weekly", 95, "x\r\ny\nz", "\r\n"), ], Some(("Ünï 🌅", "Ünï 🌅\n\r\n\nx\r\ny\nz")),),
    (&[ ("Spend", 80, "\u{9}", "line\n"), ("Spend", 80, "\r\n", "Ünï 🌅"), ("Weekly", 95, "Ünï 🌅", "a\nb"), ], Some(("\u{9}", "line\n\n\r\n\nÜnï 🌅")),),
    (&[ ("Spend", 95, "plain", "x\r\ny\nz"), ("Session", 80, "\u{9}", "line\n"), ("Weekly", 80, "x\r\ny\nz", " "), ], Some(("plain", "x\r\ny\nz\n\u{9}\nx\r\ny\nz")),),
    (&[ ("Session", 95, "Session 82% used", "plain"), ("Spend", 80, "line\n", "a\nb"), ("Spend", 80, "Ünï 🌅", ""), ], Some(("Session 82% used", "plain\nline\n\nÜnï 🌅")),),
    (&[ ("Spend", 80, "Ünï 🌅", "Session 82% used"), ("Credits", 95, "", "plain"), ], Some(("Ünï 🌅", "Session 82% used\n")),),
    (&[ ("Spend", 95, "\r\n", "  lead"), ("Weekly", 80, "  lead", "plain"), ("Credits", 80, "a\nb", "Ünï 🌅"), ], Some(("\r\n", "  lead\n  lead\na\nb")),),
    (&[ ("Credits", 95, "plain", "Ünï 🌅"), ("Session", 95, " ", "plain"), ], Some(("plain", "Ünï 🌅\n ")),),
    (&[ ("Credits", 95, "\u{9}", "x\r\ny\nz"), ("Credits", 80, "x\r\ny\nz", "line\n"), ("Credits", 80, "plain", "x\r\ny\nz"), ("Session", 80, "line\n", "x\r\ny\nz"), ], Some(("\u{9}", "x\r\ny\nz\nx\r\ny\nz\nplain\nline\n")),),
    (&[ ("Session", 80, "\r\n", ""), ("Session", 95, "  lead", " "), ("Weekly", 95, "line\n", "\r\n"), ("Credits", 80, "x\r\ny\nz", "a\nb"), ], Some(("\r\n", "  lead\nline\n\nx\r\ny\nz")),),
    (&[ ("Credits", 95, "\u{9}", "Session 82% used"), ("Spend", 95, "Ünï 🌅", "Ünï 🌅"), ("Weekly", 95, "plain", " "), ("Spend", 95, "\r\n", "\r\n"), ], Some(("\u{9}", "Session 82% used\nÜnï 🌅\nplain\n\r\n")),),
    (&[ ("Spend", 80, "Session 82% used", "x\r\ny\nz"), ("Spend", 95, "", "Ünï 🌅"), ("Weekly", 95, "a\nb", "plain"), ], Some(("Session 82% used", "x\r\ny\nz\n\na\nb")),),
    (&[ ("Credits", 80, "\r\n", " "), ("Weekly", 95, "x\r\ny\nz", "\u{9}"), ], Some(("\r\n", " \nx\r\ny\nz")),),
];
