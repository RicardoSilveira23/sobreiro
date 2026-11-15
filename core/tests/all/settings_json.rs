//! Ported from `tests/Sobreiro.Tests/SettingsJsonTests.cs`.
//!
//! Where the C# expects an exception, the Rust gives an error back: the tests hold which one.
use sobreiro_core::connect::SettingsError;
use sobreiro_core::connect::settings_json::{command_of, read_status_line, with_status_line};

const V: &str = "{\"type\":\"command\"}";

/// `with_status_line` on a text that is known to be good JSON.
fn with(text: &str, value_json: Option<&str>) -> String {
    with_status_line(text, value_json).unwrap()
}

#[test]
fn read_absent_is_null() {
    assert_eq!(read_status_line("{\"model\": \"x\"}"), Ok(None));
}

#[test]
fn read_blank_counts_as_empty_object() {
    assert_eq!(read_status_line(""), Ok(None));
    assert_eq!(read_status_line("  \n"), Ok(None));
}

#[test]
fn read_returns_the_raw_value_text() {
    let text = "{\"statusLine\": {\"type\":\"command\",\"command\":\"x\"}}";
    assert_eq!(
        read_status_line(text),
        Ok(Some("{\"type\":\"command\",\"command\":\"x\"}"))
    );
}

// The messages are shown as they are in the Connect flow's error box.
#[test]
fn read_throws_format_exception_for_non_objects_and_bad_json() {
    let rows = [
        (
            "[]",
            SettingsError::NotAnObject,
            "settings.json isn't a JSON object, so Sobreiro left it alone.",
        ),
        (
            "{oops",
            SettingsError::NotJson,
            "settings.json isn't valid JSON, so Sobreiro left it alone.",
        ),
        (
            "{} {}",
            SettingsError::NotJson,
            "settings.json isn't valid JSON, so Sobreiro left it alone.",
        ),
    ];
    for (text, error, message) in rows {
        assert_eq!(read_status_line(text), Err(error), "{text}");
        assert_eq!(error.to_string(), message, "{text}");
    }
}

#[test]
fn read_tolerates_comments_and_trailing_commas() {
    let text = "{\n  // a comment\n  \"statusLine\": \"x\", // more\n}";
    assert_eq!(read_status_line(text), Ok(Some("\"x\"")));
}

#[test]
fn replace_changes_only_the_value() {
    let text = "{\n    \"model\": \"café\",\n    // keep me\n    \"statusLine\": { \"type\": \"command\", \"command\": \"old\" },\n    \"theme\": \"dark\"\n}\n";
    let expected = "{\n    \"model\": \"café\",\n    // keep me\n    \"statusLine\": NEW,\n    \"theme\": \"dark\"\n}\n";
    assert_eq!(with(text, Some("NEW")), expected);
}

#[test]
fn replace_keeps_comments_between_properties() {
    let text = "{\"statusLine\": 1, /* b */ \"a\": 2}";
    assert_eq!(
        with(text, Some("9")),
        "{\"statusLine\": 9, /* b */ \"a\": 2}"
    );
}

#[test]
fn remove_first_property() {
    let text = format!("{{\n  \"statusLine\": {V},\n  \"a\": 1,\n  \"b\": 2\n}}\n");
    assert_eq!(with(&text, None), "{\n  \"a\": 1,\n  \"b\": 2\n}\n");
}

#[test]
fn remove_middle_property() {
    let text = format!("{{\n  \"a\": 1,\n  \"statusLine\": {V},\n  \"b\": 2\n}}\n");
    assert_eq!(with(&text, None), "{\n  \"a\": 1,\n  \"b\": 2\n}\n");
}

#[test]
fn remove_last_property_drops_the_previous_comma() {
    let text = format!("{{\n  \"a\": 1,\n  \"b\": 2,\n  \"statusLine\": {V}\n}}\n");
    assert_eq!(with(&text, None), "{\n  \"a\": 1,\n  \"b\": 2\n}\n");
}

#[test]
fn remove_only_property_gives_empty_object_and_keeps_the_trailing_newline() {
    let text = format!("{{\n  \"statusLine\": {V}\n}}\n");
    assert_eq!(with(&text, None), "{}\n");
}

#[test]
fn remove_on_one_line() {
    assert_eq!(with("{\"a\":1,\"statusLine\":2}", None), "{\"a\":1}");
    assert_eq!(with("{\"statusLine\":2, \"b\":1}", None), "{\"b\":1}");
    assert_eq!(
        with("{\"a\":1, \"statusLine\":2, \"b\":1}", None),
        "{\"a\":1, \"b\":1}"
    );
}

#[test]
fn remove_when_absent_changes_nothing() {
    let text = "{\"a\": 1}\n";
    assert_eq!(with(text, None), text);
}

#[test]
fn insert_appends_as_the_last_property() {
    assert_eq!(
        with("{\n  \"model\": \"x\"\n}\n", Some("V")),
        "{\n  \"model\": \"x\",\n  \"statusLine\": V\n}\n"
    );
}

#[test]
fn insert_into_blank_text_creates_an_object() {
    assert_eq!(with("", Some("V")), "{\n  \"statusLine\": V\n}\n");
}

#[test]
fn insert_into_an_empty_object() {
    assert_eq!(with("{}", Some("V")), "{\n  \"statusLine\": V\n}");
}

#[test]
fn insert_keeps_crlf_and_indent() {
    assert_eq!(
        with("{\r\n    \"a\": 1\r\n}\r\n", Some("V")),
        "{\r\n    \"a\": 1,\r\n    \"statusLine\": V\r\n}\r\n"
    );
}

#[test]
fn command_of_reads_the_command_string() {
    assert_eq!(
        command_of(Some("{\"type\":\"command\",\"command\":\"x\"}")).as_deref(),
        Some("x")
    );
    assert_eq!(command_of(None), None);
    assert_eq!(command_of(Some("\"x\"")), None);
    assert_eq!(command_of(Some("{\"type\":\"command\"}")), None);
}

// What follows has no C# test. Each row was put to the C# of 1.1.0 on 2026-11-14, and holds
// the answer it gave.

#[test]
fn a_comment_may_sit_anywhere_but_between_a_name_and_its_colon() {
    let fine = [
        "// c\n{\"statusLine\":1}",
        "{// c\r\n\"statusLine\":1}",
        "{\"statusLine\":// c\r\n1}",
        "{\"statusLine\":1 // c\r\n}",
        "{\"statusLine\":1 /* c */ , \"a\": 2}",
        "{\"statusLine\":1}// c",
        "{\"statusLine\":1 /*/ */}",
        "{\"statusLine\":1 /***/}",
        // A carriage return alone ends a line comment too.
        "{\"statusLine\":1, // c\r\"a\":2}",
    ];
    for text in fine {
        assert_eq!(read_status_line(text), Ok(Some("1")), "{text}");
    }
    let refused = [
        "{\"statusLine\"// c\r\n:1}",
        "{\"statusLine\" /* c */ :1}",
        "{\"statusLine\":1 //}",
        "{\"statusLine\":1}/*",
        "{\"statusLine\":1 / }",
    ];
    for text in refused {
        assert_eq!(
            read_status_line(text),
            Err(SettingsError::NotJson),
            "{text}"
        );
    }
}

#[test]
fn read_status_line_answers_as_the_csharp_did() {
    // The text of a `settings.json`, and the answer: the value as written, none, or the error.
    let rows: &[(&str, Result<Option<&str>, SettingsError>)] = &[
        ("{}", Ok(None)),
        ("  {}  ", Ok(None)),
        ("{}\n\n", Ok(None)),
        ("// c\n{}", Ok(None)),
        ("{} // c", Ok(None)),
        ("{} /* c */ ", Ok(None)),
        ("\u{a0}", Ok(None)),
        ("\u{85}", Ok(None)),
        ("\u{2028}", Ok(None)),
        ("\u{3000} \t\r\n", Ok(None)),
        ("\u{1f}", Err(SettingsError::NotJson)),
        ("\u{200b}", Err(SettingsError::NotJson)),
        ("\u{feff}{}", Err(SettingsError::NotJson)),
        ("// c", Err(SettingsError::NotJson)),
        ("/* c */", Err(SettingsError::NotJson)),
        ("[]", Err(SettingsError::NotAnObject)),
        ("[oops", Err(SettingsError::NotAnObject)),
        ("null", Err(SettingsError::NotAnObject)),
        ("nullx", Err(SettingsError::NotAnObject)),
        ("true", Err(SettingsError::NotAnObject)),
        ("12", Err(SettingsError::NotAnObject)),
        ("12 34", Err(SettingsError::NotAnObject)),
        ("\"abc\"", Err(SettingsError::NotAnObject)),
        ("\"abc\" x", Err(SettingsError::NotAnObject)),
        ("nul", Err(SettingsError::NotJson)),
        ("12x", Err(SettingsError::NotJson)),
        ("01", Err(SettingsError::NotJson)),
        ("-", Err(SettingsError::NotJson)),
        ("\"abc", Err(SettingsError::NotJson)),
        ("]", Err(SettingsError::NotJson)),
        ("}", Err(SettingsError::NotJson)),
        (",", Err(SettingsError::NotJson)),
        ("{} {}", Err(SettingsError::NotJson)),
        ("{} x", Err(SettingsError::NotJson)),
        ("{}/*", Err(SettingsError::NotJson)),
        ("{oops", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1,}", Ok(Some("1"))),
        ("{,}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1,,}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":[1,]}", Ok(Some("[1,]"))),
        ("{\"statusLine\":[,]}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":[1,,2]}", Err(SettingsError::NotJson)),
        ("{\"a\":1 \"statusLine\":2}", Err(SettingsError::NotJson)),
        ("{\"statusLine\" 1}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":}", Err(SettingsError::NotJson)),
        ("{statusLine:1}", Err(SettingsError::NotJson)),
        ("{'statusLine':1}", Err(SettingsError::NotJson)),
        ("{\"status\\u004cine\":5}", Ok(Some("5"))),
        ("{\"statusLine\":1,\"statusLine\":2}", Ok(Some("2"))),
        ("{\"statusline\":1}", Ok(None)),
        ("{\"StatusLine\":1}", Ok(None)),
        ("{\"a\":{\"statusLine\":1}}", Ok(None)),
        ("{\"statusLine\":-0}", Ok(Some("-0"))),
        ("{\"statusLine\":1.5e+3}", Ok(Some("1.5e+3"))),
        ("{\"statusLine\":1E5}", Ok(Some("1E5"))),
        ("{\"statusLine\":0.10}", Ok(Some("0.10"))),
        (
            "{\"statusLine\":12345678901234567890123}",
            Ok(Some("12345678901234567890123")),
        ),
        ("{\"statusLine\":1.}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":.5}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":+5}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1e}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":0x10}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":01}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1\"a\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":NaN}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":true}", Ok(Some("true"))),
        ("{\"statusLine\":null}", Ok(Some("null"))),
        ("{\"statusLine\":truex}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":True}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":nul}", Err(SettingsError::NotJson)),
        (
            "{\"statusLine\":\"a\\n\\t\\\"\\\\\\/\\b\\f\\r\\u00e9\"}",
            Ok(Some("\"a\\n\\t\\\"\\\\\\/\\b\\f\\r\\u00e9\"")),
        ),
        ("{\"statusLine\":\"café 😀\"}", Ok(Some("\"café 😀\""))),
        ("{\"statusLine\":\"\\x\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":\"\\u12\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":\"\\u12G4\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":\"a\tb\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":\"a\nb\"}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":\"\u{7f}\"}", Ok(Some("\"\u{7f}\""))),
        ("{\"\\ud800\":1}", Err(SettingsError::NotJson)), // the C# stops with an error of another kind
        (
            "{\"\\udc00\":1,\"statusLine\":2}",
            Err(SettingsError::NotJson),
        ), // the C# stops with an error of another kind
        ("{\"a\":\"\\ud800\",\"statusLine\":3}", Ok(Some("3"))),
        ("{\"statusLine\":\"\\ud800\"}", Ok(Some("\"\\ud800\""))),
        (
            "{\"statusLine\":{\"\\ud800\":1}}",
            Ok(Some("{\"\\ud800\":1}")),
        ),
        ("{\"statusLine\":1 // c\n}", Ok(Some("1"))),
        ("{\"statusLine\":1 // c\r\n}", Ok(Some("1"))),
        ("{\"statusLine\":1, // c\r\"a\":2}", Ok(Some("1"))),
        ("{// c\n\"statusLine\":1}", Ok(Some("1"))),
        ("{\"statusLine\":// c\n1}", Ok(Some("1"))),
        ("{\"statusLine\"// c\n:1}", Err(SettingsError::NotJson)),
        ("{\"statusLine\" /* c */ :1}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1 /*/ */}", Ok(Some("1"))),
        ("{\"statusLine\":1 /***/}", Ok(Some("1"))),
        ("{\"statusLine\":1 /* \n */}", Ok(Some("1"))),
        ("{\"statusLine\":1 //}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1 / }", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1 /*}", Err(SettingsError::NotJson)),
        ("{\"statusLine\":1 // café €\n}", Ok(Some("1"))),
        (
            "{\"statusLine\":1 // \u{2028}\n}",
            Err(SettingsError::NotJson),
        ),
        (
            "{\"statusLine\":1 // \u{2029}\n}",
            Err(SettingsError::NotJson),
        ),
        ("{\"statusLine\":1 /* \u{2028} */}", Ok(Some("1"))),
        ("{\"statusLine\":\"\u{2028}\"}", Ok(Some("\"\u{2028}\""))),
        (
            "{\"statusLine\":[1, // c\n 2 /* c */ ]}",
            Ok(Some("[1, // c\n 2 /* c */ ]")),
        ),
    ];
    for (text, expected) in rows {
        assert_eq!(read_status_line(text), *expected, "{text:?}");
    }
}

#[test]
fn objects_and_lists_may_sit_64_deep() {
    // The file's own object with so many lists inside one another.
    let nested = |lists: usize| {
        format!(
            "{{\"statusLine\":{}{}}}",
            "[".repeat(lists),
            "]".repeat(lists)
        )
    };
    assert_eq!(
        read_status_line(&nested(63)).map(|value| value.map(str::len)),
        Ok(Some(126))
    );
    assert_eq!(read_status_line(&nested(64)), Err(SettingsError::NotJson));
}

#[test]
fn with_status_line_answers_as_the_csharp_did() {
    // The text, the new value (`None` takes the key out), and the text that comes back.
    let rows: &[(&str, Option<&str>, &str)] = &[
        (
            "{\"statusLine\": 1 /* c */ , \"a\":2}",
            Some("9"),
            "{\"statusLine\": 9 /* c */ , \"a\":2}",
        ),
        ("{\"statusLine\": 1 /* c */ , \"a\":2}", None, "{\"a\":2}"),
        (
            "{\"a\":2, /* c */ \"statusLine\": 1 // d\n}",
            None,
            "{\"a\":2 // d\n}",
        ),
        (
            "{\"a\":2 /* c */ , \"statusLine\": 1 // d\n}",
            None,
            "{\"a\":2 /* c */  // d\n}",
        ),
        ("{\"a\":2,\"statusLine\": 1,}", None, "{\"a\":2,}"),
        ("{\"statusLine\": 1,}", None, "{}"),
        ("{ \"statusLine\": 1 }", None, "{}"),
        ("{ // c\n \"statusLine\": 1 }", None, "{ // c\n }"),
        (
            "{\"statusLine\":1,\"statusLine\":2}",
            None,
            "{\"statusLine\":1}",
        ),
        (
            "{\"statusLine\":1,\"statusLine\":2}",
            Some("9"),
            "{\"statusLine\":1,\"statusLine\":9}",
        ),
        ("{\"a\":1,}", Some("V"), "{\"a\":1,\n  \"statusLine\": V,}"),
        (
            "{\"a\":1, // c\n}",
            Some("V"),
            "{\"a\":1,\n  \"statusLine\": V, // c\n}",
        ),
        (
            "{\"a\":1 // c\n}",
            Some("V"),
            "{\"a\":1,\n  \"statusLine\": V // c\n}",
        ),
        ("{ }", Some("V"), "{\n  \"statusLine\": V\n}"),
        ("{ /* c */ }", Some("V"), "{\n  \"statusLine\": V /* c */ }"),
        ("{\r\n}", Some("V"), "{\r\n  \"statusLine\": V\r\n}"),
        (
            "{\n\t\"a\": 1\n}",
            Some("V"),
            "{\n\t\"a\": 1,\n\t\"statusLine\": V\n}",
        ),
        ("{\"a\": 1}", Some("V"), "{\"a\": 1,\n  \"statusLine\": V}"),
        ("  ", Some("V"), "{\n  \"statusLine\": V\n}\n"),
        ("  ", None, "  "),
        ("\u{a0}", Some("V"), "{\n  \"statusLine\": V\n}\n"),
        (
            "\n{\n  \"a\": 1,\r\n  \"b\": 2\n}",
            Some("V"),
            "\n{\n  \"a\": 1,\r\n  \"b\": 2,\r\n  \"statusLine\": V\n}",
        ),
        (
            "{\n  \"a\": 1,\n  \"statusLine\": 1\r\n}",
            None,
            "{\n  \"a\": 1\n}",
        ),
        (
            "{\r\n  \"a\": 1,\r\n  \"statusLine\": 1\r\n}\r\n",
            None,
            "{\r\n  \"a\": 1\r\n}\r\n",
        ),
        (
            "{\r\n  \"statusLine\": 1,\r\n  \"a\": 1\r\n}\r\n",
            None,
            "{\r\n  \"a\": 1\r\n}\r\n",
        ),
        (
            "{\n  \"statusLine\": 1,   \n  \"a\": 1\n}",
            None,
            "{\n  \"a\": 1\n}",
        ),
        (
            "{\n  \"statusLine\": 1, // c\n  \"a\": 1\n}",
            None,
            "{\n  // c\n  \"a\": 1\n}",
        ),
        (
            "{\n  \"a\": 1, \"statusLine\": 1\n}",
            None,
            "{\n  \"a\": 1\n}",
        ),
        (
            "{\n  \"a\": 1,\n  \"statusLine\": 1 }",
            None,
            "{\n  \"a\": 1 }",
        ),
        (
            "{\n  \"a\": 1,\n\t \"statusLine\": 1 \t\n}",
            None,
            "{\n  \"a\": 1\n}",
        ),
        (
            "{\n  \"a\": {\"statusLine\": 1}\n}",
            Some("V"),
            "{\n  \"a\": {\"statusLine\": 1},\n  \"statusLine\": V\n}",
        ),
        (
            "{\n  \"a\": {\"statusLine\": 1}\n}",
            None,
            "{\n  \"a\": {\"statusLine\": 1}\n}",
        ),
        (
            "{\n    \"a\": 1,\n    \"b\": {\n        \"c\": 2\n    }\n}\n",
            Some("V"),
            "{\n    \"a\": 1,\n    \"b\": {\n        \"c\": 2\n    },\n    \"statusLine\": V\n}\n",
        ),
        (
            "{\"a\": \"café 😀\", \"statusLine\": \"é\", \"b\": \"é\"}",
            Some("{\"x\": \"á\"}"),
            "{\"a\": \"café 😀\", \"statusLine\": {\"x\": \"á\"}, \"b\": \"é\"}",
        ),
        (
            "{\"a\": \"café 😀\", \"statusLine\": \"é\", \"b\": \"é\"}",
            None,
            "{\"a\": \"café 😀\", \"b\": \"é\"}",
        ),
    ];
    for (text, value, expected) in rows {
        assert_eq!(
            with_status_line(text, *value).as_deref(),
            Ok(*expected),
            "{text:?} {value:?}"
        );
    }
}

#[test]
fn command_of_answers_as_the_csharp_did() {
    // A status line, and its command.
    let rows: &[(&str, Option<&str>)] = &[
        ("{\"command\":\"a\",\"command\":\"b\"}", Some("b")),
        ("{\"command\":\"a\",\"command\":1}", None),
        ("{\"command\":1,\"command\":\"b\"}", Some("b")),
        ("{\"comm\\u0061nd\":\"x\"}", Some("x")),
        ("{\"Command\":\"x\"}", None),
        (
            "{\"command\":\"\\ud83d\\ude00 \\u00e9 \\n\"}",
            Some("😀 é \n"),
        ),
        ("{ // c\n\"command\":\"x\",}", Some("x")),
        ("{\"command\":\"x\"} x", None),
        ("", None),
        ("  ", None),
        ("[\"command\"]", None),
        ("{\"command\":{\"command\":\"x\"}}", None),
        ("{\"command\":null}", None),
        ("{\"command\":\"\\ud800\"}", None), // the C# stops with an error
        ("{\"command\":\"\\ud800\",\"command\":\"x\"}", Some("x")),
        ("{\"\\ud800\":1,\"command\":\"x\"}", Some("x")),
        ("{\"command\":\"x\",\"\\ud800\":1}", Some("x")),
    ];
    for (json, expected) in rows {
        assert_eq!(command_of(Some(json)).as_deref(), *expected, "{json:?}");
    }
}
