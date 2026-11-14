// Ports of ClaudeCredentialsTests, LivePollTests, CopilotPollTests, CopilotTokenTests, GhCliTests: the ones that
// look at a token's text, which only this crate may do. Tokens here are fixture strings; no test sends a request.

use crate::copilot::*;
use crate::live::*;
use crate::polling::{CancelToken, ProviderState};
use crate::secret::Secret;
use crate::{Span, Time};
use chrono::DateTime;
use std::cell::{Cell, RefCell};
use std::time::Duration;

fn t(s: &str) -> Time {
    DateTime::parse_from_rfc3339(s).unwrap()
}
fn mins(m: i64) -> Span {
    Span::minutes(m)
}

/// Compiles only when `$t` implements none of Debug, Display, Serialize and Clone.
macro_rules! assert_silent {
    ($t:ty) => {{
        trait AmbiguousIfDebug<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfDebug<()> for T {}
        impl<T: ?Sized + std::fmt::Debug> AmbiguousIfDebug<u8> for T {}
        trait AmbiguousIfDisplay<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfDisplay<()> for T {}
        impl<T: ?Sized + std::fmt::Display> AmbiguousIfDisplay<u8> for T {}
        trait AmbiguousIfSerialize<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfSerialize<()> for T {}
        impl<T: ?Sized + serde::Serialize> AmbiguousIfSerialize<u8> for T {}
        trait AmbiguousIfClone<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfClone<()> for T {}
        impl<T: ?Sized + Clone> AmbiguousIfClone<u8> for T {}
        <$t as AmbiguousIfDebug<_>>::check();
        <$t as AmbiguousIfDisplay<_>>::check();
        <$t as AmbiguousIfSerialize<_>>::check();
        <$t as AmbiguousIfClone<_>>::check();
    }};
}

#[test]
fn a_token_type_cannot_be_printed_serialised_or_copied() {
    assert_silent!(AccessToken);
    assert_silent!(Secret);
    assert_silent!(CredentialRead);
    assert_silent!(TokenSearch);
}

// ---- ClaudeCredentialsTests

const SECRET: &str = "sk-ant-oat01-SECRETVALUE";

fn now() -> Time {
    t("2026-11-09T12:00:00Z")
}

fn creds(expires_at: Option<&str>) -> String {
    format!(
        "{{\"claudeAiOauth\":{{\"accessToken\":\"{SECRET}\",\"refreshToken\":\"r\"{}}}}}",
        expires_at.map(|e| format!(",\"expiresAt\":{e}")).unwrap_or_default()
    )
}

struct Dir(tempfile::TempDir);
impl Dir {
    fn new() -> Dir {
        Dir(tempfile::Builder::new().prefix("sobreiro-tests-").tempdir().unwrap())
    }
    fn write(&self, text: &str) -> String {
        let p = self.0.path().join(".credentials.json");
        std::fs::write(&p, text).unwrap();
        p.to_string_lossy().into_owned()
    }
    fn path(&self) -> String {
        self.0.path().join(".credentials.json").to_string_lossy().into_owned()
    }
}

#[test]
fn a_future_expiry_gives_the_token() {
    let d = Dir::new();
    let future = (now() + Span::hours(2)).timestamp_millis().to_string();
    let read = ClaudeCredentials::read(&d.write(&creds(Some(&future))), now());
    assert_eq!(read.status, CredentialStatus::Ok);
    let token = read.token.unwrap();
    assert_eq!(token.value(), SECRET);
    assert_eq!(token.expires_at(), Some(now() + Span::hours(2)));
}

#[test]
fn an_expiry_at_or_before_now_is_expired_and_returns_no_token() {
    for offset_ms in [-1i64, 0] {
        let d = Dir::new();
        let at = (now().timestamp_millis() + offset_ms).to_string();
        let read = ClaudeCredentials::read(&d.write(&creds(Some(&at))), now());
        assert_eq!(read.status, CredentialStatus::Expired);
        assert!(read.token.is_none());
    }
}

#[test]
fn no_expiry_is_ok_with_a_null_expiry() {
    let d = Dir::new();
    let read = ClaudeCredentials::read(&d.write(&creds(None)), now());
    assert_eq!(read.status, CredentialStatus::Ok);
    assert!(read.token.unwrap().expires_at().is_none());
}

#[test]
fn a_missing_file_is_missing() {
    let d = Dir::new();
    let read = ClaudeCredentials::read(&d.path(), now());
    assert_eq!(read.status, CredentialStatus::Missing);
    assert!(read.token.is_none());
}

#[test]
fn anything_without_a_token_string_is_unreadable() {
    for text in [
        "{}", "[]", "{ broken", "", "{\"claudeAiOauth\":{\"accessToken\":12345}}", "{\"claudeAiOauth\":{\"accessToken\":\"\"}}",
        "{\"claudeAiOauth\":[]}",
    ] {
        let d = Dir::new();
        let read = ClaudeCredentials::read(&d.write(text), now());
        assert_eq!(read.status, CredentialStatus::Unreadable, "{text}");
        assert!(read.token.is_none());
    }
}

#[test]
fn a_locked_file_is_unreadable() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = Dir::new();
    let p = d.write(&creds(None));
    let _hold = std::fs::OpenOptions::new().read(true).share_mode(0).open(&p).unwrap();
    assert_eq!(ClaudeCredentials::read(&p, now()).status, CredentialStatus::Unreadable);
}

// Claude Code may hold the file open for writing during its own refresh; reading must not need it closed.
#[test]
fn a_file_held_open_for_writing_still_reads() {
    use std::os::windows::fs::OpenOptionsExt;
    let d = Dir::new();
    let p = d.write(&creds(None));
    let _writer = std::fs::OpenOptions::new().write(true).share_mode(1).open(&p).unwrap(); // FILE_SHARE_READ
    let read = ClaudeCredentials::read(&p, now());
    assert_eq!(read.status, CredentialStatus::Ok);
    assert_eq!(read.token.unwrap().value(), SECRET);
}

#[test]
fn a_byte_order_mark_is_skipped() {
    let d = Dir::new();
    let p = d.path();
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(creds(None).as_bytes());
    std::fs::write(&p, bytes).unwrap();
    assert_eq!(ClaudeCredentials::read(&p, now()).status, CredentialStatus::Ok);
}

#[test]
fn a_directory_in_the_way_is_unreadable_not_a_panic() {
    let d = Dir::new();
    let dir = d.0.path().join("d");
    std::fs::create_dir(&dir).unwrap();
    assert_eq!(ClaudeCredentials::read(&dir.to_string_lossy(), now()).status, CredentialStatus::Unreadable);
}

#[test]
fn a_present_expiry_that_is_not_a_number_is_unreadable() {
    for expiry in ["\"soon\"", "true", "{}"] {
        let d = Dir::new();
        let read = ClaudeCredentials::read(&d.write(&creds(Some(expiry))), now());
        assert_eq!(read.status, CredentialStatus::Unreadable, "{expiry}");
        assert!(read.token.is_none());
    }
}

#[test]
fn a_null_expiry_counts_as_none() {
    let d = Dir::new();
    assert_eq!(ClaudeCredentials::read(&d.write(&creds(Some("null"))), now()).status, CredentialStatus::Ok);
}

#[test]
fn the_token_is_not_in_a_result_text() {
    let d = Dir::new();
    let read = ClaudeCredentials::read(&d.write(&creds(None)), now());
    assert!(!format!("{:?}", LiveResult::of(LiveOutcome::Failed)).contains(SECRET));
    assert!(read.token.is_some());
}

// ---- LivePollTests (the sign-in file is real, in a scratch folder; the request is a fake that answers by status)

const BODY: &str = "{\"limits\":[{\"group\":\"session\",\"percent\":27,\"resets_at\":\"2026-11-09T14:00:00Z\"}]}";

fn start() -> LiveSchedule {
    LiveSchedule::start(now())
}

struct Net {
    calls: Cell<usize>,
    answer: Box<dyn Fn() -> LiveResult>,
}

impl Net {
    fn status(code: u16, retry_after: Option<&'static str>, body: &'static str) -> Net {
        Net { calls: Cell::new(0), answer: Box::new(move || LiveUsageClient::interpret(code, retry_after, Some(body), now())) }
    }
    fn outcome(o: LiveOutcome) -> Net {
        Net { calls: Cell::new(0), answer: Box::new(move || LiveResult::of(o)) }
    }
    /// A fake that fails the test if it is called: nothing may be sent without a usable token.
    fn must_not_be_called() -> Net {
        Net { calls: Cell::new(0), answer: Box::new(|| panic!("Live usage sent a request without a usable token.")) }
    }
    fn run(&self, path: &str, schedule: &LiveSchedule, at: Time) -> LivePollResult {
        LivePoll::run_with(
            |a| ClaudeCredentials::read(path, a),
            |token, _| {
                self.calls.set(self.calls.get() + 1);
                assert_eq!(token.value(), SECRET); // the fixture token is what would be sent
                (self.answer)()
            },
            schedule,
            at,
        )
    }
}

fn credentials_expiring(d: &Dir, at: Option<Time>) -> String {
    d.write(&creds(at.map(|a| a.timestamp_millis().to_string()).as_deref()))
}

#[test]
fn missing_credentials_send_nothing_and_say_no_sign_in() {
    let d = Dir::new();
    let net = Net::must_not_be_called();
    let r = net.run(&d.path(), &start(), now());
    assert_eq!(net.calls.get(), 0);
    assert_eq!((r.state, r.outcome.as_str()), (ProviderState::NoSignIn, "Missing"));
    assert!(r.limits.is_none());
    assert_eq!(r.schedule, start().after_no_credentials(now()));
}

#[test]
fn unreadable_credentials_send_nothing_and_say_no_sign_in() {
    for text in ["not json", "{\"claudeAiOauth\":{}}", "{\"claudeAiOauth\":{\"accessToken\":\"\"}}"] {
        let d = Dir::new();
        let net = Net::must_not_be_called();
        let r = net.run(&d.write(text), &start(), now());
        assert_eq!(net.calls.get(), 0);
        assert_eq!((r.state, r.outcome.as_str()), (ProviderState::NoSignIn, "Unreadable"));
        assert_eq!(r.schedule, start().after_no_credentials(now()));
    }
}

#[test]
fn expired_credentials_send_nothing_and_say_expired() {
    let d = Dir::new();
    let net = Net::must_not_be_called();
    let p = credentials_expiring(&d, Some(now() - mins(1)));
    let r = net.run(&p, &start(), now());
    assert_eq!(net.calls.get(), 0);
    assert_eq!((r.state, r.outcome.as_str()), (ProviderState::Expired, "Expired"));
    assert!(r.limits.is_none());
    assert_eq!(r.schedule, start().after_no_credentials(now()));
}

#[test]
fn an_ok_read_without_a_token_sends_nothing() {
    let r = LivePoll::run_with(
        |_| CredentialRead { status: CredentialStatus::Ok, token: None },
        |_, _| panic!("sent"),
        &start(),
        now(),
    );
    assert_eq!((r.state, r.outcome.as_str()), (ProviderState::NoSignIn, "Unreadable"));
    assert_eq!(r.schedule, start().after_no_credentials(now()));
}

#[test]
fn the_read_is_asked_with_the_poll_time() {
    let asked = Cell::new(None);
    LivePoll::run_with(
        |at| {
            asked.set(Some(at));
            CredentialRead { status: CredentialStatus::Missing, token: None }
        },
        |_, _| panic!("sent"),
        &start(),
        now(),
    );
    assert_eq!(asked.get(), Some(now()));
}

#[test]
fn a_good_reply_gives_the_limits_and_state_ok() {
    let d = Dir::new();
    let net = Net::status(200, None, BODY);
    let r = net.run(&credentials_expiring(&d, Some(now() + Span::hours(2))), &start(), now());
    assert_eq!(net.calls.get(), 1);
    assert_eq!(r.state, ProviderState::Ok);
    assert_eq!(r.limits.unwrap().five_hour.unwrap().used_percentage, 27.0);
    assert_eq!(r.schedule.last_success, Some(now()));
    assert_eq!(r.schedule.next_due, now() + mins(5));
    assert_eq!(r.outcome, "Ok");
}

#[test]
fn a_429_is_rate_limited_and_waits() {
    let d = Dir::new();
    let net = Net::status(429, Some("600"), "");
    let r = net.run(&credentials_expiring(&d, Some(now() + Span::hours(2))), &start(), now());
    assert_eq!(r.state, ProviderState::RateLimited);
    assert!(r.limits.is_none());
    assert_eq!((r.schedule.next_due, r.schedule.wait_until), (now() + mins(10), Some(now() + mins(10))));
    assert_eq!(r.outcome, "RateLimited");
}

#[test]
fn other_replies_are_a_bad_answer_and_retry_in_two_minutes() {
    for code in [500u16, 302] {
        let d = Dir::new();
        let p = credentials_expiring(&d, Some(now() + Span::hours(2)));
        let net = Net::status(code, None, "");
        let r = net.run(&p, &start(), now());
        assert_eq!(net.calls.get(), 1);
        assert_eq!((r.state, r.outcome.as_str()), (ProviderState::BadAnswer, "Failed"));
        assert!(r.limits.is_none());
        assert_eq!(r.schedule.next_due, now() + mins(2));
        // The same sign-in is tried again once the retry is due.
        let retry = Net::status(code, None, "");
        retry.run(&p, &r.schedule, now() + mins(2));
        assert_eq!(retry.calls.get(), 1);
    }
}

#[test]
fn a_200_with_an_unexpected_body_is_a_bad_answer() {
    let d = Dir::new();
    let net = Net::status(200, None, "{\"unexpected\":true}");
    let r = net.run(&credentials_expiring(&d, Some(now() + Span::hours(2))), &start(), now());
    assert_eq!((r.state, r.outcome.as_str()), (ProviderState::BadAnswer, "Failed"));
    assert!(r.limits.is_none());
}

#[test]
fn a_request_that_cannot_reach_anthropic_is_offline() {
    // A timeout and an unreachable host both come back from the client as Unreachable.
    let d = Dir::new();
    let p = credentials_expiring(&d, Some(now() + Span::hours(2)));
    let net = Net::outcome(LiveOutcome::Unreachable);
    let r = net.run(&p, &start(), now());
    assert_eq!((r.state, r.outcome.as_str()), (ProviderState::Offline, "Unreachable"));
    assert!(r.limits.is_none());
    assert_eq!(r.schedule.next_due, now() + mins(2));
    let retry = Net::outcome(LiveOutcome::Unreachable);
    retry.run(&p, &r.schedule, now() + mins(2));
    assert_eq!(retry.calls.get(), 1);
}

#[test]
fn a_refused_sign_in_is_not_sent_again_until_the_file_gives_a_new_one() {
    for code in [401u16, 403] {
        let d = Dir::new();
        let p = credentials_expiring(&d, Some(now() + Span::hours(2)));
        let refusing = Net::status(code, None, "");
        let mut r = refusing.run(&p, &start(), now());
        assert_eq!(refusing.calls.get(), 1);
        assert_eq!((r.state, r.outcome.as_str()), (ProviderState::Refused, "Unauthorized"));
        assert_eq!(r.schedule.refused, Some(RefusedSignIn { expires_at: Some(now() + Span::hours(2)) }));
        assert_eq!(r.schedule.next_due, now() + mins(1));
        // Each due tick reads the file again but sends nothing while it gives the same expiry.
        let mut schedule = r.schedule;
        for minutes in [1, 2, 3] {
            let at = now() + mins(minutes);
            assert!(schedule.is_due(at));
            let silent = Net::must_not_be_called();
            r = silent.run(&p, &schedule, at);
            assert_eq!(silent.calls.get(), 0);
            assert_eq!((r.state, r.outcome.as_str()), (ProviderState::Refused, "Refused"));
            assert_eq!(r.schedule.next_due, at + mins(1));
            schedule = r.schedule;
        }
        // Claude Code saved a new sign-in: it is sent at once.
        credentials_expiring(&d, Some(now() + Span::hours(10)));
        let ok = Net::status(200, None, BODY);
        let r = ok.run(&p, &schedule, now() + mins(4));
        assert_eq!(ok.calls.get(), 1);
        assert_eq!(r.state, ProviderState::Ok);
        assert!(r.schedule.refused.is_none());
    }
}

#[test]
fn a_refused_sign_in_without_an_expiry_waits_for_a_file_with_one() {
    let d = Dir::new();
    let p = credentials_expiring(&d, None);
    let r = Net::status(401, None, "").run(&p, &start(), now());
    assert_eq!(r.schedule.refused, Some(RefusedSignIn { expires_at: None }));
    let silent = Net::must_not_be_called();
    let r = silent.run(&p, &r.schedule, now() + mins(1));
    assert_eq!((silent.calls.get(), r.state), (0, ProviderState::Refused));
    credentials_expiring(&d, Some(now() + Span::hours(2)));
    let ok = Net::status(200, None, BODY);
    let r = ok.run(&p, &r.schedule, now() + mins(2));
    assert_eq!((ok.calls.get(), r.state), (1, ProviderState::Ok));
}

#[test]
fn a_success_after_a_429_resets_the_wait() {
    let d = Dir::new();
    let waiting = LiveSchedule { rate_limit_step: 2, wait_until: Some(now() - mins(1)), ..start() };
    let r = Net::status(200, None, BODY).run(&credentials_expiring(&d, Some(now() + Span::hours(2))), &waiting, now());
    assert_eq!(r.schedule.rate_limit_step, 0);
    assert!(r.schedule.wait_until.is_none());
}

#[test]
fn the_token_never_appears_in_a_result() {
    let d = Dir::new();
    let p = credentials_expiring(&d, Some(now() + Span::hours(2)));
    for code in [200u16, 401, 429] {
        let r = Net::status(code, None, BODY).run(&p, &start(), now());
        assert!(!format!("{r:?}").contains(SECRET));
    }
}

// ---- CopilotTokenTests

fn fake() -> String {
    format!("gho_{}", "a".repeat(36))
}
fn s(t: Option<Secret>) -> Option<String> {
    t.map(|t| t.expose().to_string())
}

#[test]
fn finds_a_bare_token() {
    assert_eq!(s(CopilotToken::find(Some(&fake()))), Some(fake()));
}
#[test]
fn finds_a_token_inside_json() {
    assert_eq!(s(CopilotToken::find(Some(&format!("{{\"oauth_token\":\"{}\",\"user\":\"x\"}}", fake())))), Some(fake()));
}
#[test]
fn finds_a_fine_grained_token() {
    let pat = format!("github_pat_11AB{}_{}", "c".repeat(30), "d".repeat(20));
    assert_eq!(s(CopilotToken::find(Some(&format!("token={pat} ")))), Some(pat));
}
#[test]
fn text_without_a_token_gives_null() {
    assert!(CopilotToken::find(Some("nothing to see here")).is_none());
    assert!(CopilotToken::find(None).is_none());
    assert!(CopilotToken::find(Some("gho_")).is_none());
}

fn utf16(text: &str, bom: bool) -> Vec<u8> {
    let mut v = if bom { vec![0xFF, 0xFE] } else { vec![] };
    for u in text.encode_utf16() {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

#[test]
fn utf16_blob_with_bom_is_decoded() {
    assert_eq!(s(CopilotToken::from_blob(&utf16(&format!("{{\"oauth_token\":\"{}\"}}", fake()), true))), Some(fake()));
}
#[test]
fn utf8_blob_is_decoded() {
    let mut blob = vec![0xEF, 0xBB, 0xBF];
    blob.extend_from_slice(format!("{{\"oauth_token\":\"{}\"}}", fake()).as_bytes());
    assert_eq!(s(CopilotToken::from_blob(&blob)), Some(fake()));
}
#[test]
fn utf8_blob_with_a_trailing_nul_is_decoded() {
    let mut blob = format!("{{\"oauth_token\":\"{}\"}}", fake()).into_bytes();
    blob.push(0);
    assert_eq!(s(CopilotToken::from_blob(&blob)), Some(fake()));
}
// NOT PORTED: The_decoded_text_is_cleared_and_only_the_token_is_kept, ..._when_it_holds_no_token,
// A_utf16_token_is_found_after_the_utf8_pass_and_the_text_is_cleared, Text_already_in_the_buffer_is_never_searched.
// They look at a caller-supplied scratch buffer that the C# API exposes for the test; this crate decodes into its
// own strings and overwrites them (secret::wipe) before returning, with no buffer to hand in, so there is nothing
// to inspect afterwards.
#[test]
fn odd_contents_decode_the_same_way() {
    let f = fake();
    let mut utf8_nul = f.clone().into_bytes();
    utf8_nul.push(0);
    let mut emoji = "😀 ".to_string().into_bytes();
    emoji.extend_from_slice(f.as_bytes());
    let mut lone_surrogate = vec![0x00, 0xD8];
    lone_surrogate.extend(utf16(&format!(" {f}"), false));
    let mut odd_byte = utf16(&f, false);
    odd_byte.push(0x41);
    let cases: Vec<(Vec<u8>, Option<String>)> = vec![
        (vec![], None),
        (vec![0xFF], None),
        (utf8_nul, Some(f.clone())),
        (emoji, Some(f.clone())),
        (lone_surrogate, Some(f.clone())),
        (odd_byte, Some(f)),
    ];
    for (blob, token) in cases {
        assert_eq!(s(CopilotToken::from_blob(&blob)), token);
    }
}

#[test]
fn environment_order_is_copilot_gh_github() {
    let env = |k: &str| match k {
        "GITHUB_TOKEN" => Some("c".to_string()),
        "GH_TOKEN" => Some("b".to_string()),
        "COPILOT_GITHUB_TOKEN" => Some("a".to_string()),
        _ => None,
    };
    let got: Vec<String> = CopilotToken::from_environment(&env).iter().map(|t| t.expose().to_string()).collect();
    assert_eq!(got, ["a", "b", "c"]);
}

#[test]
fn blank_and_duplicate_environment_values_are_skipped() {
    let env = |k: &str| match k {
        "COPILOT_GITHUB_TOKEN" => Some("  ".to_string()),
        "GH_TOKEN" => Some(" x ".to_string()),
        "GITHUB_TOKEN" => Some("x".to_string()),
        _ => None,
    };
    let got: Vec<String> = CopilotToken::from_environment(&env).iter().map(|t| t.expose().to_string()).collect();
    assert_eq!(got, ["x"]);
}

// ---- CopilotPollTests (the request is a fake that answers by the token it is given)

fn cnow() -> Time {
    t("2026-11-10T12:00:00Z")
}

const CBUSINESS: &str = r#"{"copilot_plan":"business","quota_reset_date_utc":"2026-12-01T00:00:00.000Z",
"quota_snapshots":{"premium_interactions":{"entitlement":15000,"remaining":12355,"quota_remaining":12355.8,
"unlimited":false,"has_quota":true,"timestamp_utc":"2026-11-10T09:25:19.365Z"}}}"#;
const CFREE: &str = "{\"quota_reset_at\":0,\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

fn source(kind: CopilotSourceKind, tokens: &[&str]) -> CopilotSignIn {
    let tokens: Vec<String> = tokens.iter().map(|t| t.to_string()).collect();
    CopilotSignIn::new(kind, move || Ok(TokenSearch::from(tokens.iter().map(|t| Secret::new(t.clone())).collect::<Vec<_>>())))
}
fn entry_without_token(kind: CopilotSourceKind) -> CopilotSignIn {
    CopilotSignIn::new(kind, || Ok(TokenSearch::new(Vec::new(), true)))
}
fn throwing(kind: CopilotSourceKind) -> CopilotSignIn {
    CopilotSignIn::new(kind, || Err(SourceFailed))
}

type Answers<'a> = &'a dyn Fn(&str) -> CopilotFetch;

fn run(answer: Answers, sign_ins: Vec<CopilotSignIn>) -> (CopilotPollResult, Vec<String>) {
    let seen = RefCell::new(Vec::new());
    let result = CopilotPoll::run_with(
        &sign_ins,
        |token, _| {
            seen.borrow_mut().push(token.expose().to_string());
            answer(token.expose())
        },
        &LiveSchedule::start(cnow()),
        cnow(),
        &CancelToken::default(),
    );
    (result, seen.into_inner())
}

fn reply(code: u16, body: &str) -> CopilotFetch {
    CopilotUsageClient::interpret(code, None, Some(body), cnow())
}

use CopilotSourceKind::{CredentialManager, Env, Gh};

#[test]
fn first_accepted_token_wins_and_later_sources_are_not_called() {
    let (r, seen) = run(&|_| reply(200, CBUSINESS), vec![source(Env, &["A"]), throwing(Gh)]);
    assert_eq!(r.state, ProviderState::Ok);
    assert!(r.reading.is_some());
    assert_eq!(r.log, "copilot Ok env=ok credential-manager=skipped gh=skipped");
    assert_eq!(r.schedule.last_success, Some(cnow()));
    assert_eq!(seen, ["A"]);
}

#[test]
fn refused_and_no_budget_fall_through() {
    let answers = |tok: &str| match tok {
        "A" => reply(401, ""),
        "B" => reply(200, CFREE),
        _ => reply(200, CBUSINESS),
    };
    let (r, seen) = run(&answers, vec![source(Env, &["A"]), source(CredentialManager, &["B"]), source(Gh, &["C"])]);
    assert_eq!(r.state, ProviderState::Ok);
    assert_eq!(r.log, "copilot Ok env=refused credential-manager=no-budget gh=ok");
    assert_eq!(seen, ["A", "B", "C"]);
}

#[test]
fn all_refused_is_refused_with_a_five_minute_wait() {
    let (r, _) = run(&|_| reply(401, ""), vec![source(Env, &["A"]), source(Gh, &["C"])]);
    assert_eq!(r.state, ProviderState::Refused);
    assert!(r.reading.is_none());
    assert_eq!(r.log, "copilot Refused env=refused credential-manager=skipped gh=refused");
    assert_eq!(r.schedule.next_due, cnow() + mins(5));
    assert!(!r.schedule.fetch_on_open(cnow() + mins(2)));
}

#[test]
fn no_tokens_anywhere_is_no_sign_in() {
    let (r, seen) = run(&|_| reply(200, CBUSINESS), vec![source(Env, &[]), source(CredentialManager, &[]), source(Gh, &[])]);
    assert_eq!(r.state, ProviderState::NoSignIn);
    assert_eq!(r.log, "copilot NoSignIn env=none credential-manager=none gh=none");
    assert!(seen.is_empty());
    assert_eq!(r.schedule.next_due, cnow() + mins(5));
}

#[test]
fn entry_without_a_token_is_logged_as_no_token() {
    let (r, seen) = run(&|_| reply(200, CBUSINESS), vec![source(Env, &[]), entry_without_token(CredentialManager), source(Gh, &[])]);
    assert_eq!(r.state, ProviderState::NoSignIn);
    assert_eq!(r.log, "copilot NoSignIn env=none credential-manager=no-token gh=none");
    assert!(seen.is_empty());
}

#[test]
fn throwing_source_is_error_and_skipped() {
    let (r, seen) = run(&|_| reply(200, CBUSINESS), vec![source(Env, &[]), throwing(CredentialManager), source(Gh, &["C"])]);
    assert_eq!(r.state, ProviderState::Ok);
    assert_eq!(r.log, "copilot Ok env=none credential-manager=error gh=ok");
    assert_eq!(seen, ["C"]);
}

#[test]
fn only_a_throwing_source_is_no_sign_in() {
    let (r, _) = run(&|_| reply(200, CBUSINESS), vec![throwing(Gh)]);
    assert_eq!(r.state, ProviderState::NoSignIn);
    assert_eq!(r.log, "copilot NoSignIn env=skipped credential-manager=skipped gh=error");
}

#[test]
fn rate_limit_stops_the_chain() {
    let (r, seen) = run(&|_| CopilotUsageClient::interpret(429, Some("600"), None, cnow()), vec![source(Env, &["A"]), throwing(Gh)]);
    assert_eq!(r.state, ProviderState::RateLimited);
    assert_eq!(r.log, "copilot RateLimited env=rate-limited credential-manager=skipped gh=skipped");
    assert_eq!(r.schedule.next_due, cnow() + mins(10));
    assert_eq!(seen.len(), 1);
}

#[test]
fn a_500_is_a_bad_answer_and_stops_the_chain() {
    let (r, seen) = run(&|_| reply(500, ""), vec![source(Env, &["A"]), throwing(Gh)]);
    assert_eq!(r.state, ProviderState::BadAnswer);
    assert_eq!(r.log, "copilot BadAnswer env=failed credential-manager=skipped gh=skipped");
    assert_eq!(r.schedule.next_due, cnow() + mins(2));
    assert_eq!(seen.len(), 1);
}

#[test]
fn network_failure_is_offline_and_stops_the_chain() {
    let (r, seen) = run(&|_| CopilotFetch::of(CopilotOutcome::Unreachable), vec![source(Env, &["A"]), throwing(Gh)]);
    assert_eq!(r.state, ProviderState::Offline);
    assert_eq!(r.log, "copilot Offline env=unreachable credential-manager=skipped gh=skipped");
    assert_eq!(r.schedule.next_due, cnow() + mins(2));
    assert_eq!(seen.len(), 1);
}

#[test]
fn several_tokens_in_one_source_log_the_last_result() {
    let answers = |tok: &str| if tok == "A" { reply(401, "") } else { reply(200, CFREE) };
    let (r, _) = run(&answers, vec![source(Gh, &["A", "B"])]);
    assert_eq!(r.state, ProviderState::Refused);
    assert_eq!(r.log, "copilot Refused env=skipped credential-manager=skipped gh=no-budget");
}

#[test]
fn log_never_holds_a_token() {
    let (r, _) = run(&|_| reply(401, ""), vec![source(Env, &["gho_made_up_one"]), source(Gh, &["gho_made_up_two"])]);
    assert!(!r.log.contains("gho_"));
}

#[test]
fn a_cancelled_poll_asks_nothing() {
    let cancel = CancelToken::default();
    cancel.cancel();
    let asked = Cell::new(false);
    let r = CopilotPoll::run_with(
        &[source(Env, &["A"])],
        |_, _| {
            asked.set(true);
            reply(200, CBUSINESS)
        },
        &LiveSchedule::start(cnow()),
        cnow(),
        &cancel,
    );
    assert!(!asked.get());
    assert!(r.reading.is_none());
}

// ---- GhCliTests

#[test]
fn path_lookup_finds_gh_exe_in_an_absolute_entry_and_skips_empty_and_relative_ones() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().to_string_lossy().into_owned();
    let exe = d.path().join("gh.exe");
    std::fs::write(&exe, "").unwrap();
    assert_eq!(GhCli::find(Some(&format!(";relative;{dir}"))), Some(exe.to_string_lossy().into_owned()));
    assert_eq!(GhCli::find(Some(";relative")), None);
    assert_eq!(GhCli::find(Some(&format!("{dir}\\missing"))), None);
    assert_eq!(GhCli::find(None), None);
}

#[test]
fn missing_exe_gives_no_token() {
    assert!(GhCli::run(r"C:\nope\gh-missing.exe", &["auth", "token"], Duration::from_secs(5)).is_empty());
}

#[test]
fn printed_token_is_returned() {
    let got = GhCli::run("cmd.exe", &["/c", "echo", &fake()], Duration::from_secs(5));
    assert_eq!(got.iter().map(|t| t.expose().to_string()).collect::<Vec<_>>(), [fake()]);
}

#[test]
fn non_zero_exit_gives_no_token() {
    assert!(GhCli::run("cmd.exe", &["/c", "echo", &fake(), "&", "exit", "1"], Duration::from_secs(5)).is_empty());
}

#[test]
fn grandchild_holding_the_pipe_open_does_not_stall_the_read() {
    let watch = std::time::Instant::now();
    let args = ["/c", "start", "/b", "ping", "-n", "30", "127.0.0.1", "&", "echo", &fake()];
    assert!(GhCli::run("cmd.exe", &args, Duration::from_secs(1)).is_empty());
    assert!(watch.elapsed() < Duration::from_secs(4), "took {:?}", watch.elapsed());
}

#[test]
fn hanging_exe_is_killed_after_the_timeout() {
    let watch = std::time::Instant::now();
    assert!(GhCli::run("ping.exe", &["-n", "30", "127.0.0.1"], Duration::from_secs(1)).is_empty());
    assert!(watch.elapsed() < Duration::from_secs(4), "took {:?}", watch.elapsed());
}
