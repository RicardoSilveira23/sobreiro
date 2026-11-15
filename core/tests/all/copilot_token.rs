//! Ported from `tests/Sobreiro.Tests/CopilotTokenTests.cs`. The tests after
//! `blank_and_duplicate_environment_values_are_skipped` are new.
//!
//! Every token here is made up. No test reads the real environment or a real sign-in: the
//! bytes and the environment are handed in by the test.
use std::cell::RefCell;
use std::collections::HashMap;

use sobreiro_core::copilot::copilot_token::{self, GitHubToken};

/// A made-up token of the shape GitHub gives a signed-in app.
fn fake() -> String {
    format!("gho_{}", "a".repeat(36))
}

/// What a search found, as plain text. A token itself cannot be compared or printed, so a
/// test takes its value out first.
fn value(token: Option<GitHubToken>) -> Option<String> {
    token.map(|token| token.value().to_string())
}

fn values(tokens: Vec<GitHubToken>) -> Vec<String> {
    tokens
        .iter()
        .map(|token| token.value().to_string())
        .collect()
}

/// A text as UTF-16, low byte first, which is how Windows keeps text.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// The bytes some programs put at the start of a text to say which encoding it is in.
const UTF8_MARK: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16_MARK: &[u8] = &[0xFF, 0xFE];

/// An environment made of the pairs given: what `from_environment` is handed in a test.
fn environment(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let pairs: HashMap<String, String> = pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    move |name| pairs.get(name).cloned()
}

#[test]
fn finds_a_bare_token() {
    assert_eq!(value(copilot_token::find(&fake())), Some(fake()));
}

#[test]
fn finds_a_token_inside_json() {
    let json = format!("{{\"oauth_token\":\"{}\",\"user\":\"x\"}}", fake());

    assert_eq!(value(copilot_token::find(&json)), Some(fake()));
}

#[test]
fn finds_a_fine_grained_token() {
    let pat = format!("github_pat_11AB{}_{}", "c".repeat(30), "d".repeat(20));

    assert_eq!(
        value(copilot_token::find(&format!("token={pat} "))),
        Some(pat)
    );
}

#[test]
fn text_without_a_token_gives_none() {
    assert_eq!(value(copilot_token::find("nothing to see here")), None);
    // The C# test also hands in no text at all. In Rust a text is always there; the nearest
    // is an empty one.
    assert_eq!(value(copilot_token::find("")), None);
}

#[test]
fn a_utf16_blob_with_its_mark_is_decoded() {
    let json = format!("{{\"oauth_token\":\"{}\"}}", fake());
    let blob = [UTF16_MARK, &utf16(&json)].concat();

    assert_eq!(value(copilot_token::from_blob(&blob)), Some(fake()));
}

#[test]
fn a_utf8_blob_is_decoded() {
    let json = format!("{{\"oauth_token\":\"{}\"}}", fake());
    let blob = [UTF8_MARK, json.as_bytes()].concat();

    assert_eq!(value(copilot_token::from_blob(&blob)), Some(fake()));
}

#[test]
fn a_utf8_blob_with_a_trailing_nul_is_decoded() {
    let json = format!("{{\"oauth_token\":\"{}\"}}", fake());
    let blob = [json.as_bytes(), &[0]].concat();

    assert_eq!(value(copilot_token::from_blob(&blob)), Some(fake()));
}

#[test]
fn only_the_token_is_kept_of_what_a_blob_holds() {
    let json = format!(
        "{{\"key\":\"other-material\",\"oauth_token\":\"{}\"}}",
        fake()
    );

    assert_eq!(
        value(copilot_token::from_blob(json.as_bytes())),
        Some(fake())
    );
}

#[test]
fn a_blob_that_holds_no_token_gives_none() {
    let blob = utf16("a key, not a token");

    assert_eq!(value(copilot_token::from_blob(&blob)), None);
}

#[test]
fn a_utf16_token_is_found_after_the_utf8_pass() {
    let json = format!("{{\"oauth_token\":\"{}\"}}", fake());

    assert_eq!(value(copilot_token::from_blob(&utf16(&json))), Some(fake()));
}

#[test]
fn odd_contents_give_the_token_or_none() {
    // A blob, and whether the token comes out of it.
    let rows: Vec<(Vec<u8>, bool)> = vec![
        (vec![], false),
        (vec![0xFF], false),
        ([fake().as_bytes(), &[0]].concat(), true),
        (format!("\u{1F600} {}", fake()).into_bytes(), true),
        // Half a character of UTF-16 before the token.
        (
            [&[0x00, 0xD8][..], &utf16(&format!(" {}", fake()))].concat(),
            true,
        ),
        // One byte too many after it.
        ([&utf16(&fake())[..], &[0x41]].concat(), true),
    ];
    for (blob, found) in rows {
        let expected = found.then(fake);

        assert_eq!(value(copilot_token::from_blob(&blob)), expected, "{blob:?}");
    }
}

#[test]
fn environment_order_is_copilot_gh_github() {
    let env = environment(&[
        ("GITHUB_TOKEN", "c"),
        ("GH_TOKEN", "b"),
        ("COPILOT_GITHUB_TOKEN", "a"),
    ]);

    assert_eq!(
        values(copilot_token::from_environment(env)),
        ["a", "b", "c"]
    );
}

#[test]
fn blank_and_duplicate_environment_values_are_skipped() {
    let env = environment(&[
        ("COPILOT_GITHUB_TOKEN", "  "),
        ("GH_TOKEN", " x "),
        ("GITHUB_TOKEN", "x"),
    ]);

    assert_eq!(values(copilot_token::from_environment(env)), ["x"]);
}

// What follows is new.

#[test]
fn a_token_is_never_printed() {
    let token = GitHubToken::new(fake());

    // Rust has two ways to print a value, and a long form of the second. None may show the
    // token.
    assert_eq!(token.to_string(), "GitHubToken(redacted)");
    assert_eq!(format!("{token:?}"), "GitHubToken(redacted)");
    assert!(!format!("{token:#?}").contains(&fake()));

    // Nor may a larger value that holds one and is printed whole.
    let found = copilot_token::find(&fake());
    assert!(found.is_some());
    assert!(!format!("{found:?}").contains(&fake()));
    assert!(!format!("{found:#?}").contains(&fake()));
    let from_blob = copilot_token::from_blob(fake().as_bytes());
    assert!(from_blob.is_some());
    assert!(!format!("{from_blob:?}").contains(&fake()));
    let from_environment = copilot_token::from_environment(|_| Some(fake()));
    assert_eq!(from_environment.len(), 1);
    assert!(!format!("{from_environment:?}").contains(&fake()));
    assert!(!format!("{from_environment:#?}").contains(&fake()));
}

#[test]
fn the_environment_is_asked_for_three_names_and_no_other() {
    // `RefCell` lets the closure below write the names down while it is only borrowed.
    let asked = RefCell::new(Vec::new());

    let tokens = copilot_token::from_environment(|name| {
        asked.borrow_mut().push(name.to_string());
        None
    });

    assert_eq!(values(tokens), [] as [&str; 0]);
    assert_eq!(
        asked.into_inner(),
        ["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]
    );
}

// The rows below were written out by the C# of 1.1.0, run on 2026-11-14 on made-up tokens and
// made-up replies with a stand-in for the network, and turned into Rust by a script, not typed.

#[test]
fn find_answers_as_the_csharp_did() {
    // A text, and the token found in it.
    let rows: &[(&str, Option<&str>)] = &[
        ("", None),
        (" ", None),
        (
            "gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "ghu_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            Some("ghu_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            Some("ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        ),
        (
            "github_pat_11ABcccccccccccccccccccccccccccccc_dddddddddddddddddddd",
            Some("github_pat_11ABcccccccccccccccccccccccccccccc_dddddddddddddddddddd"),
        ),
        ("ghs_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("ghr_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("gha_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("GHO_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("Gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("gho-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("ghoaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", None),
        ("gho_", None),
        ("gho_ ", None),
        ("gho__", Some("gho__")),
        ("gho_a", Some("gho_a")),
        ("gho_A", Some("gho_A")),
        ("gho_0", Some("gho_0")),
        ("gho_-", None),
        ("gho_.", None),
        ("gho_a-b", Some("gho_a")),
        ("gho_a.b", Some("gho_a")),
        ("gho_a b", Some("gho_a")),
        ("gho_a\nb", Some("gho_a")),
        ("gho_a\u{0}b", Some("gho_a")),
        ("gho_a\tb", Some("gho_a")),
        ("gho_a/b", Some("gho_a")),
        ("gho_a+b", Some("gho_a")),
        ("gho_a=b", Some("gho_a")),
        ("gho_a:b", Some("gho_a")),
        ("xgho_abc", Some("gho_abc")),
        ("xxgho_abc", Some("gho_abc")),
        ("_gho_abc", Some("gho_abc")),
        ("agho_abc", Some("gho_abc")),
        ("gho_abc gho_def", Some("gho_abc")),
        ("ghu_first ghp_second", Some("ghu_first")),
        ("ghp_gho_abc", Some("ghp_gho_abc")),
        ("gho_gho_", Some("gho_gho_")),
        ("ghogho_abc", Some("gho_abc")),
        ("ghgho_abc", Some("gho_abc")),
        ("gghhoo_abc", None),
        ("gho_ gho_x", Some("gho_x")),
        ("gh_abc", None),
        ("ghx_abc gho_y", Some("gho_y")),
        ("github_pat_", None),
        ("github_pat_x", Some("github_pat_x")),
        ("github_pat", None),
        ("github_pa_x", None),
        ("github_patx_y", None),
        ("GITHUB_PAT_x", None),
        ("github_PAT_x", None),
        ("xgithub_pat_x", Some("github_pat_x")),
        ("github_pat__", Some("github_pat__")),
        ("github_github_pat_x", Some("github_pat_x")),
        (
            "git github_pat_abc_DEF_123 hub",
            Some("github_pat_abc_DEF_123"),
        ),
        ("gho_\u{e9}", None),
        ("gho_a\u{e9}b", Some("gho_a")),
        ("gho_\u{430}", None),
        ("gho_a\u{430}", Some("gho_a")),
        ("gho_\u{ff11}", None),
        ("gho_a\u{ff11}", Some("gho_a")),
        ("gho_\u{661}", None),
        ("gho_a\u{661}", Some("gho_a")),
        ("gho_\u{212a}", None),
        ("gho_a\u{212a}", Some("gho_a")),
        ("gho_\u{17f}", None),
        ("gho_a\u{131}", Some("gho_a")),
        ("\u{e9}gho_abc", Some("gho_abc")),
        ("gho_a\u{1f600}b", Some("gho_a")),
        ("g\u{127}o_abc", None),
        ("gh\u{3bf}_abc", None),
        ("\u{ff47}ho_abc", None),
        ("token=gho_abc;", Some("gho_abc")),
        ("\"gho_abc\"", Some("gho_abc")),
        ("'gho_abc'", Some("gho_abc")),
        (
            "{\"oauth_token\":\"gho_abc\",\"user\":\"x\"}",
            Some("gho_abc"),
        ),
        ("gho_abc\r\n", Some("gho_abc")),
        ("\r\ngho_abc", Some("gho_abc")),
        ("Bearer gho_abc", Some("gho_abc")),
        ("https://x:gho_abc@github.com", Some("gho_abc")),
        ("gho_abc_", Some("gho_abc_")),
        ("gho_abc__def", Some("gho_abc__def")),
        ("gho_ABCdef0123456789_", Some("gho_ABCdef0123456789_")),
        ("ghu_", None),
        ("ghp_", None),
        ("ghp_x", Some("ghp_x")),
        ("ghu_x", Some("ghu_x")),
        ("gh\u{0}o_abc", None),
        ("gho\u{0}_abc", None),
        ("gho_\u{0}abc", None),
    ];
    for (text, token) in rows {
        assert_eq!(
            value(copilot_token::find(text)).as_deref(),
            *token,
            "{text:?}"
        );
    }
}

#[test]
fn a_long_text_is_searched_to_its_end() {
    // A token of five thousand letters, and a token after five thousand spaces.
    let long = format!("gho_{}", "a".repeat(5000));
    assert_eq!(value(copilot_token::find(&long)), Some(long));

    let late = format!("{}gho_x", " ".repeat(5000));
    assert_eq!(value(copilot_token::find(&late)), Some("gho_x".to_string()));
}

#[test]
fn from_blob_answers_as_the_csharp_did() {
    // The bytes of a sign-in entry, and the token found in them.
    let rows: &[(&[u8], Option<&str>)] = &[
        (b"", None),
        (b"\xff", None),
        (b"\x00", None),
        (b"gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"\xef\xbb\xbf{\"oauth_token\":\"gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"\xff\xfe{\x00\"\x00o\x00a\x00u\x00t\x00h\x00_\x00t\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00\"\x00}\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"{\x00\"\x00o\x00a\x00u\x00t\x00h\x00_\x00t\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00\"\x00}\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00\x00\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00A", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"\x00\xd8 \x00g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"\xf0\x9f\x98\x80 gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"a key, not a token", None),
        (b"a\x00 \x00k\x00e\x00y\x00,\x00 \x00n\x00o\x00t\x00 \x00a\x00 \x00t\x00o\x00k\x00e\x00n\x00", None),
        (b" g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00", None),
        (b"  g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00", Some("gho_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
        (b"   g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00", None),
        (b"\x00g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a", None),
        (b"\xfe\xff\x00g\x00h\x00o\x00_\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a\x00a", None),
        (b"gho_eight g\x00h\x00u\x00_\x00s\x00i\x00x\x00t\x00e\x00e\x00n\x00", Some("gho_eight")),
        (b"g\x00h\x00u\x00_\x00s\x00i\x00x\x00t\x00e\x00e\x00n\x00 \x00gho_eight", Some("gho_eight")),
        (b"g\x00h\x00u\x00_\x00s\x00i\x00x\x00t\x00e\x00e\x00n\x00 gho_eight", Some("gho_eight")),
        (b"gho_one ghp_two", Some("gho_one")),
        (b"g\x00h\x00o\x00_\x00o\x00n\x00e\x00 \x00g\x00h\x00p\x00_\x00t\x00w\x00o\x00", Some("gho_one")),
        (b"\xffgho_abc", Some("gho_abc")),
        (b"gho_abc\xff", Some("gho_abc")),
        (b"gho_ab\xffcd", Some("gho_ab")),
        (b"gho\xff_abcd", None),
        (b"gho_ab\xc3cd", Some("gho_ab")),
        (b"gho_ab\xc3\xa9cd", Some("gho_ab")),
        (b"gho_ab\xe2\x82cd", Some("gho_ab")),
        (b"gho_ab\xf0\x9f\x98cd", Some("gho_ab")),
        (b"gho_ab\xed\xa0\x80cd", Some("gho_ab")),
        (b"gho_ab\xc0\x80cd", Some("gho_ab")),
        (b"gho_ab\x80cd", Some("gho_ab")),
        (b"\xc3gho_abc", Some("gho_abc")),
        (b"\xe2\x82gho_abc", Some("gho_abc")),
        (b"\xf0\x9f\x98gho_abc", Some("gho_abc")),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00c\x01d\x00", Some("gho_ab")),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00c\x00d\x00", Some("gho_abcd")),
        (b"g\x00h\x00o\x01_\x00a\x00b\x00c\x00", None),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00\x00\xd8c\x00d\x00", Some("gho_ab")),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00=\xd8\x00\xdec\x00d\x00", Some("gho_ab")),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00\x00\xdcc\x00d\x00", Some("gho_ab")),
        (b"g\x00h\x00o\x00_\x00\x00\x00a\x00b\x00c\x00", None),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00c\x00\x00\x00d\x00e\x00f\x00", Some("gho_abc")),
        (b"g\x00h\x00o\x00_\x00a\x00b\x00c\x00\x00", Some("gho_abc")),
        (b"g\x00h\x00o\x00_\x00", None),
        (b"gho_", None),
        (b"g\x00h\x00o\x00_\x00a", None),
        (b"github_pat_x\x00", Some("github_pat_x")),
        (b"g\x00i\x00t\x00h\x00u\x00b\x00_\x00p\x00a\x00t\x00_\x00x\x00", Some("github_pat_x")),
        (b"gho_a\x00b", Some("gho_a")),
        (b"gghhoo__aa", None),
        (b"g\x00h\x00o\x00_\x00a\x00", Some("gho_a")),
        (b"g\x00h\x00o\x00_\x00a", None),
        (b"xg\x00h\x00o\x00_\x00a\x00", None),
    ];
    for (blob, token) in rows {
        assert_eq!(
            value(copilot_token::from_blob(blob)).as_deref(),
            *token,
            "{blob:?}"
        );
    }
}

#[test]
fn from_environment_answers_as_the_csharp_did() {
    // What `COPILOT_GITHUB_TOKEN`, `GH_TOKEN` and `GITHUB_TOKEN` hold, and the tokens that
    // come of it.
    let rows: &[([Option<&str>; 3], &[&str])] = &[
        ([None, None, None], &[]),
        ([Some("a"), Some("b"), Some("c")], &["a", "b", "c"]),
        ([Some("a"), None, None], &["a"]),
        ([None, Some("b"), None], &["b"]),
        ([None, None, Some("c")], &["c"]),
        ([Some(""), Some(""), Some("")], &[]),
        ([Some(" "), Some("\t"), Some("\r\n")], &[]),
        ([Some(" a "), Some("b"), Some(" c")], &["a", "b", "c"]),
        ([Some("a"), Some("a"), Some("a")], &["a"]),
        ([Some("a"), Some(" a "), Some("b")], &["a", "b"]),
        ([Some("a"), Some("b"), Some("a")], &["a", "b"]),
        ([Some("a"), Some("A"), Some("a ")], &["a", "A"]),
        ([Some("x y"), Some("x"), Some("y")], &["x y", "x", "y"]),
        (
            [Some("gho_abc"), Some("ghp_def"), Some("github_pat_g")],
            &["gho_abc", "ghp_def", "github_pat_g"],
        ),
        (
            [Some("not a token at all"), None, None],
            &["not a token at all"],
        ),
        ([Some("a\nb"), None, None], &["a\nb"]),
        ([Some("\na\n"), None, None], &["a"]),
        ([Some("a\u{0}"), None, None], &["a\u{0}"]),
        ([Some("\u{0}a"), None, None], &["\u{0}a"]),
        ([Some("\u{a0}a\u{a0}"), None, None], &["a"]),
        ([Some("\u{3000}a"), None, None], &["a"]),
        ([Some("\u{feff}a"), None, None], &["\u{feff}a"]),
        ([Some("\u{200b}a"), None, None], &["\u{200b}a"]),
        ([Some("\u{85}a"), None, None], &["a"]),
        ([Some("\u{1f}a"), None, None], &["\u{1f}a"]),
        ([Some("\"a\""), None, None], &["\"a\""]),
        ([Some("a"), Some("\"a\""), None], &["a", "\"a\""]),
        ([Some("\u{e9}"), None, None], &["\u{e9}"]),
        ([Some("  "), Some(" x "), Some("x")], &["x"]),
    ];
    for (given, tokens) in rows {
        let found = copilot_token::from_environment(|name| {
            let value = match name {
                "COPILOT_GITHUB_TOKEN" => given[0],
                "GH_TOKEN" => given[1],
                "GITHUB_TOKEN" => given[2],
                other => panic!("asked for {other}"),
            };
            value.map(str::to_string)
        });

        assert_eq!(values(found), *tokens, "{given:?}");
    }
}

#[test]
fn the_characters_taken_off_the_ends_of_a_value_are_the_ones_the_csharp_took_off() {
    // The C# was handed every character it holds in one piece, at both ends of a value, and
    // took these off.
    let taken_off: &[u32] = &[
        0x9, 0xa, 0xb, 0xc, 0xd, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004,
        0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000,
    ];
    for code in 0..=0xFFFF {
        // The codes that are half a character are no character.
        let Some(character) = char::from_u32(code) else {
            continue;
        };
        let given = format!("{character}x{character}");
        let expected = if taken_off.contains(&code) {
            "x".to_string()
        } else {
            given.clone()
        };

        let found =
            copilot_token::from_environment(|name| (name == "GH_TOKEN").then(|| given.clone()));

        assert_eq!(values(found), [expected], "{code:x}");
    }
}
