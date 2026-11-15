//! Ported from `tests/Sobreiro.Tests/ClaudeCredentialsTests.cs`. The tests after
//! `a_null_expiry_counts_as_none` are new.
//!
//! Every token here is made up, and every file is written into a folder of the test's own:
//! no test reads Claude Code's real sign-in file.
use std::path::PathBuf;

use chrono::{DateTime, TimeDelta, Utc};
use sobreiro_core::live::claude_credentials::{self, CredentialRead};
use sobreiro_core::live::{LiveOutcome, LiveResult};

use crate::{TestFolder, hold_alone, time};

const SECRET: &str = "sk-ant-oat01-SECRETVALUE";

fn now() -> DateTime<Utc> {
    time("2026-11-09T12:00:00Z")
}

/// Writes `text` as the sign-in file in the test's folder and gives its path.
fn write(folder: &TestFolder, text: &str) -> PathBuf {
    write_bytes(folder, text.as_bytes())
}

fn write_bytes(folder: &TestFolder, bytes: &[u8]) -> PathBuf {
    folder.create();
    let path = folder.file(".credentials.json");
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A sign-in file with the made-up token, and with `expires_at` as its expiry when one is given.
fn creds(expires_at: Option<&str>) -> String {
    let expiry = match expires_at {
        Some(at) => ",\"expiresAt\":".to_string() + at,
        None => String::new(),
    };
    "{\"claudeAiOauth\":{\"accessToken\":\"".to_string()
        + SECRET
        + "\",\"refreshToken\":\"r\""
        + &expiry
        + "}}"
}

/// What a read came to, as one word.
fn status(read: &CredentialRead) -> &'static str {
    match read {
        CredentialRead::Ok(_) => "Ok",
        CredentialRead::Missing => "Missing",
        CredentialRead::Unreadable => "Unreadable",
        CredentialRead::Expired => "Expired",
    }
}

#[test]
fn a_future_expiry_gives_the_token() {
    let folder = TestFolder::new("credentials-future-expiry");
    let future = (now() + TimeDelta::hours(2)).timestamp_millis().to_string();

    let read = claude_credentials::read(&write(&folder, &creds(Some(&future))), now());

    let CredentialRead::Ok(token) = read else {
        panic!("{read:?}");
    };
    assert_eq!(token.value(), SECRET);
    assert_eq!(token.expires_at(), Some(now() + TimeDelta::hours(2)));
}

#[test]
fn an_expiry_at_or_before_now_is_expired_and_returns_no_token() {
    let folder = TestFolder::new("credentials-expired");
    for offset_ms in [-1, 0] {
        let at = (now().timestamp_millis() + offset_ms).to_string();

        let read = claude_credentials::read(&write(&folder, &creds(Some(&at))), now());

        // `Expired` carries no token: there is nothing more to look at.
        assert!(matches!(read, CredentialRead::Expired), "{at}: {read:?}");
    }
}

#[test]
fn no_expiry_is_ok_with_a_null_expiry() {
    let folder = TestFolder::new("credentials-no-expiry");

    let read = claude_credentials::read(&write(&folder, &creds(None)), now());

    let CredentialRead::Ok(token) = read else {
        panic!("{read:?}");
    };
    assert_eq!(token.expires_at(), None);
}

#[test]
fn a_missing_file_is_missing() {
    let folder = TestFolder::new("credentials-missing");

    let read = claude_credentials::read(&folder.file("nope.json"), now());

    assert!(matches!(read, CredentialRead::Missing), "{read:?}");
}

#[test]
fn anything_without_a_token_string_is_unreadable() {
    let folder = TestFolder::new("credentials-no-token");
    for text in [
        "{}",
        "[]",
        "{ broken",
        "",
        "{\"claudeAiOauth\":{\"accessToken\":12345}}",
        "{\"claudeAiOauth\":{\"accessToken\":\"\"}}",
        "{\"claudeAiOauth\":[]}",
    ] {
        let read = claude_credentials::read(&write(&folder, text), now());

        assert!(
            matches!(read, CredentialRead::Unreadable),
            "{text}: {read:?}"
        );
    }
}

#[test]
fn a_locked_file_is_unreadable() {
    let folder = TestFolder::new("credentials-locked");
    let path = write(&folder, &creds(None));
    let _hold = hold_alone(&path);

    let read = claude_credentials::read(&path, now());

    assert!(matches!(read, CredentialRead::Unreadable), "{read:?}");
}

// Claude Code may hold the file open for writing during its own refresh; reading must not need
// it closed.
#[test]
fn a_file_held_open_for_writing_still_reads() {
    use std::os::windows::fs::OpenOptionsExt;
    let folder = TestFolder::new("credentials-held-for-writing");
    let path = write(&folder, &creds(None));
    // Open for writing, letting others read: 1 is what Windows calls FILE_SHARE_READ.
    let _writer = std::fs::OpenOptions::new()
        .write(true)
        .share_mode(1)
        .open(&path)
        .unwrap();

    let read = claude_credentials::read(&path, now());

    let CredentialRead::Ok(token) = read else {
        panic!("{read:?}");
    };
    assert_eq!(token.value(), SECRET);
}

#[test]
fn a_byte_order_mark_is_skipped() {
    let folder = TestFolder::new("credentials-byte-order-mark");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(creds(None).as_bytes());

    let read = claude_credentials::read(&write_bytes(&folder, &bytes), now());

    assert_eq!(status(&read), "Ok");
}

#[test]
fn a_directory_in_the_way_is_unreadable_not_a_throw() {
    let folder = TestFolder::new("credentials-directory");
    let directory = folder.path().join("d");
    std::fs::create_dir_all(&directory).unwrap();

    let read = claude_credentials::read(&directory, now());

    assert_eq!(status(&read), "Unreadable");
}

#[test]
fn token_never_appears_in_to_string_or_results() {
    let folder = TestFolder::new("credentials-never-printed");

    let read = claude_credentials::read(&write(&folder, &creds(None)), now());

    let CredentialRead::Ok(token) = &read else {
        panic!("{read:?}");
    };
    assert_eq!(token.to_string(), "AccessToken(redacted)");
    assert!(!format!("{read:?}").contains(SECRET));
    let failed = LiveResult {
        outcome: LiveOutcome::Failed,
        limits: None,
        retry_after: None,
    };
    assert!(!format!("{failed:?}").contains(SECRET));
    assert!(!format!("{token}").contains(SECRET));
    // Rust has a second way to print a value, for debugging, and a long form of it. Neither
    // may show the token.
    assert_eq!(format!("{token:?}"), "AccessToken(redacted)");
    assert!(!format!("{token:#?}").contains(SECRET));
    assert!(!format!("{read:#?}").contains(SECRET));
}

#[test]
fn a_present_expiry_that_is_not_a_number_is_unreadable() {
    let folder = TestFolder::new("credentials-expiry-not-a-number");
    for expiry in ["\"soon\"", "true", "{}"] {
        let read = claude_credentials::read(&write(&folder, &creds(Some(expiry))), now());

        assert!(
            matches!(read, CredentialRead::Unreadable),
            "{expiry}: {read:?}"
        );
    }
}

#[test]
fn a_null_expiry_counts_as_none() {
    let folder = TestFolder::new("credentials-null-expiry");

    let read = claude_credentials::read(&write(&folder, &creds(Some("null"))), now());

    assert_eq!(status(&read), "Ok");
}

// What follows is new. These tests hold what the C# of 1.1.0 answered when it was run, on 2026-11-14,
// on made-up files in a folder of its own. The rows were written out by the C# and turned into
// Rust by a script, not typed.

/// What the C# made of a file.
enum Answer {
    /// The token, and its expiry in milliseconds since 1970 when the file gives one.
    Token(&'static str, Option<i64>),
    Unreadable,
    Expired,
}

#[test]
fn read_answers_as_the_csharp_did() {
    use Answer::{Expired, Token, Unreadable};
    // The bytes of a sign-in file, and the answer. `now` is 1794225600000 in milliseconds.
    let rows: &[(&[u8], Answer)] = &[
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\"}}", Token("sk-ant-oat01-MADEUP", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":null}}", Token("sk-ant-oat01-MADEUP", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":0}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-0}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-1}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1.0}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1e3}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1E3}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1.5}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":\"1\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":true}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":false}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":[]}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":{}}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225599999}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225600000}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225600001}}", Token("sk-ant-oat01-MADEUP", Some(1794225600001))),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794232800000}}", Token("sk-ant-oat01-MADEUP", Some(1794232800000))),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":253402300799999}}", Token("sk-ant-oat01-MADEUP", Some(253402300799999))),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":253402300800000}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-62135596800000}}", Expired),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-62135596800001}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":9223372036854775807}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":9223372036854775808}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-9223372036854775808}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":-9223372036854775809}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1e400}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":00}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":01}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225600000.0}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225600000e0}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\",\"expiresAt\":1794225600000.000}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\" \"}}", Token(" ", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"a\"}}", Token("a", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":12345}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":null}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":true}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":[]}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":{}}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"a\\nb\"}}", Token("a\nb", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"\\u0061\\u0062\"}}", Token("ab", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"\\ud83d\\ude00\"}}", Token("\u{1f600}", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"\xc3\xa9\"}}", Token("\u{e9}", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"a\\/b\"}}", Token("a/b", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"\\u0000\"}}", Token("\u{0}", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"a\\qb\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"tab\there\"}}", Unreadable),
        (b"{}", Unreadable),
        (b"[]", Unreadable),
        (b"{ broken", Unreadable),
        (b"", Unreadable),
        (b" ", Unreadable),
        (b"null", Unreadable),
        (b"1", Unreadable),
        (b"\"x\"", Unreadable),
        (b"{\"claudeAiOauth\":[]}", Unreadable),
        (b"{\"claudeAiOauth\":null}", Unreadable),
        (b"{\"claudeAiOauth\":\"x\"}", Unreadable),
        (b"{\"claudeAiOauth\":{}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"first\"},\"claudeAiOauth\":{\"accessToken\":\"second\"}}", Token("second", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"first\"},\"claudeAiOauth\":5}", Unreadable),
        (b"{\"claudeAiOauth\":5,\"claudeAiOauth\":{\"accessToken\":\"second\"}}", Token("second", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"first\",\"accessToken\":\"second\"}}", Token("second", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"first\",\"accessToken\":7}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\",\"expiresAt\":1,\"expiresAt\":null}}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\",\"expiresAt\":null,\"expiresAt\":1}}", Expired),
        (b"{\"\\u0063laudeAiOauth\":{\"accessToken\":\"t\"}}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"access\\u0054oken\":\"t\"}}", Token("t", None)),
        (b"{\"ClaudeAiOauth\":{\"accessToken\":\"t\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"AccessToken\":\"t\"}}", Unreadable),
        (b"{\"claudeAiOauth \":{\"accessToken\":\"t\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\",}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}} // c", Unreadable),
        (b"/* c */{\"claudeAiOauth\":{\"accessToken\":\"t\"}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}} x", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}}{}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}}\r\n", Token("t", None)),
        (b" \t\r\n{\"claudeAiOauth\":{\"accessToken\":\"t\"}}", Token("t", None)),
        (b"{'claudeAiOauth':{'accessToken':'t'}}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":01}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":\"\\ud800\"}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":1e999}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":NaN}", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"\\ud800\":1}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]}", Unreadable),
        (b"{\"other\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]],\"claudeAiOauth\":{\"accessToken\":\"t\"}}", Unreadable),
        (b"\xef\xbb\xbf{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\"}}", Token("sk-ant-oat01-MADEUP", None)),
        (b"\xef\xbb\xbf\xef\xbb\xbf{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\"}}", Unreadable),
        (b" \xef\xbb\xbf{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\"}}", Unreadable),
        (b"\xef\xbb\xbf  {\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-MADEUP\",\"refreshToken\":\"r\"}}", Token("sk-ant-oat01-MADEUP", None)),
        (b"\xef\xbb\xbf", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"},\"other\":\"a\xffb\"}", Token("t", None)),
        (b"{\"oth\xffer\":1,\"claudeAiOauth\":{\"accessToken\":\"t\"}}", Token("t", None)),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}}\xff", Unreadable),
        (b"{\"claudeAiOauth\":{\"accessToken\":\"t\"}}\x00", Unreadable),
        (b"\x00{\"claudeAiOauth\":{\"accessToken\":\"t\"}}", Unreadable),
        (b"\xff\xfe{\x00\"\x00c\x00l\x00a\x00u\x00d\x00e\x00A\x00i\x00O\x00a\x00u\x00t\x00h\x00\"\x00:\x00{\x00\"\x00a\x00c\x00c\x00e\x00s\x00s\x00T\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00s\x00k\x00-\x00a\x00n\x00t\x00-\x00o\x00a\x00t\x000\x001\x00-\x00M\x00A\x00D\x00E\x00U\x00P\x00\"\x00,\x00\"\x00r\x00e\x00f\x00r\x00e\x00s\x00h\x00T\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00r\x00\"\x00}\x00}\x00", Unreadable),
        (b"{\x00\"\x00c\x00l\x00a\x00u\x00d\x00e\x00A\x00i\x00O\x00a\x00u\x00t\x00h\x00\"\x00:\x00{\x00\"\x00a\x00c\x00c\x00e\x00s\x00s\x00T\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00s\x00k\x00-\x00a\x00n\x00t\x00-\x00o\x00a\x00t\x000\x001\x00-\x00M\x00A\x00D\x00E\x00U\x00P\x00\"\x00,\x00\"\x00r\x00e\x00f\x00r\x00e\x00s\x00h\x00T\x00o\x00k\x00e\x00n\x00\"\x00:\x00\"\x00r\x00\"\x00}\x00}\x00", Unreadable),
    ];
    let folder = TestFolder::new("credentials-csharp-answers");
    for (bytes, answer) in rows {
        let read = claude_credentials::read(&write_bytes(&folder, bytes), now());

        let shown = String::from_utf8_lossy(bytes);
        match (answer, &read) {
            (Token(value, expiry), CredentialRead::Ok(token)) => {
                assert_eq!(token.value(), *value, "{shown}");
                let expires_at = token.expires_at().map(|at| at.timestamp_millis());
                assert_eq!(expires_at, *expiry, "{shown}");
            }
            (Unreadable, CredentialRead::Unreadable) | (Expired, CredentialRead::Expired) => {}
            _ => panic!("{shown}: {read:?}"),
        }
    }
}

#[test]
fn a_token_that_is_no_text_is_unreadable() {
    // Half a character spelled out in the token, or bytes in it that are not UTF-8. The C#
    // stopped here with an error of another kind, which its caller took for a failed request.
    // Either way no token comes out.
    let rows: &[&[u8]] = &[
        b"{\"claudeAiOauth\":{\"accessToken\":\"\\ud800\"}}",
        b"{\"claudeAiOauth\":{\"accessToken\":\"\\udc00\"}}",
        b"{\"claudeAiOauth\":{\"accessToken\":\"x\\ud800y\"}}",
        b"{\"claudeAiOauth\":{\"accessToken\":\"a}}\xffb\"}}",
        b"{\"claudeAiOauth\":{\"accessToken\":\"a}}\xc3b\"}}",
        b"{\"claudeAiOauth\":{\"accessToken\":\"a}}\xed\xa0\x80b\"}}",
    ];
    let folder = TestFolder::new("credentials-token-no-text");
    for bytes in rows {
        let read = claude_credentials::read(&write_bytes(&folder, bytes), now());

        let shown = String::from_utf8_lossy(bytes);
        assert!(
            matches!(read, CredentialRead::Unreadable),
            "{shown}: {read:?}"
        );
    }
}

#[test]
fn a_megabyte_is_read_and_one_byte_more_is_not() {
    let folder = TestFolder::new("credentials-megabyte");
    let mut text = creds(None);
    text.push_str(&" ".repeat((1 << 20) - text.len()));

    let read = claude_credentials::read(&write(&folder, &text), now());
    assert_eq!(status(&read), "Ok");

    text.push(' ');
    let read = claude_credentials::read(&write(&folder, &text), now());
    assert_eq!(status(&read), "Unreadable");
}

#[test]
fn a_missing_folder_is_missing_and_a_name_no_file_can_have_is_unreadable() {
    let folder = TestFolder::new("credentials-paths");
    let file = write(&folder, &creds(None));

    let missing_folder = folder.path().join("no").join("such").join("x.json");
    let read = claude_credentials::read(&missing_folder, now());
    assert_eq!(status(&read), "Missing");

    // A file where a folder was expected.
    let read = claude_credentials::read(&file.join("x.json"), now());
    assert_eq!(status(&read), "Missing");

    let read = claude_credentials::read(&folder.file("a<b>.json"), now());
    assert_eq!(status(&read), "Unreadable");

    // The C# stopped with an error on a path of nothing. Nothing hands one in: the path is
    // always Claude Code's folder with the file's name after it.
    let read = claude_credentials::read(std::path::Path::new(""), now());
    assert_eq!(status(&read), "Missing");
}
