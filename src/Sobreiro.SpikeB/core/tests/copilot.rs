// Ports of CopilotUserJsonTests, CopilotCacheTests, CopilotCliCredentialTests, CopilotStateTests,
// CopilotProviderTests, and the parts of CopilotUsageClientTests that need no HTTP server.
mod common;
use common::*;
use sobreiro_core::copilot::*;
use sobreiro_core::limits::LimitKind;
use sobreiro_core::live::LiveSchedule;
use sobreiro_core::polling::*;
use sobreiro_core::{Time, Zone};
use std::cell::RefCell;
use std::os::windows::fs::OpenOptionsExt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn approx(expected: f64, actual: f64, tol: f64) {
    assert!((expected - actual).abs() <= tol, "{actual} is not {expected} (+-{tol})");
}

// ---- CopilotUserJsonTests

fn now() -> Time {
    t("2026-11-10T12:00:00Z")
}

const BUSINESS: &str = r#"
{
  "copilot_plan": "business",
  "token_based_billing": true,
  "quota_reset_date_utc": "2026-12-01T00:00:00.000Z",
  "quota_snapshots": {
    "premium_interactions": {
      "entitlement": 15000, "remaining": 12355, "quota_remaining": 12355.8,
      "percent_remaining": 82.3, "unlimited": false, "has_quota": true,
      "overage_permitted": true, "overage_count": 0, "overage_entitlement": 0,
      "token_based_billing": true, "timestamp_utc": "2026-11-10T09:25:19.365Z"
    },
    "chat": { "unlimited": true },
    "completions": { "unlimited": true }
  }
}"#;

fn interactions(fields: &str) -> String {
    format!("{{\"quota_snapshots\":{{\"premium_interactions\":{{{fields}}}}}}}")
}

fn parse(body: &str) -> Option<CopilotReading> {
    CopilotUserJson::parse(body, now())
}

#[test]
fn business_seat_reads_used_from_quota_remaining() {
    let r = parse(BUSINESS).unwrap();
    assert_eq!(r.budget, 15000.0);
    approx(2644.2, r.used, 0.05);
    approx(17.628, r.percent(), 0.0005);
    assert_eq!(r.resets_at, Some(t("2026-12-01T00:00:00Z")));
    assert_eq!(r.observed_at, t("2026-11-10T09:25:19.365Z"));
    assert_eq!(r.source, CopilotSource::Live);
}

#[test]
fn premium_models_wins_over_premium_interactions() {
    let body = r#"{"quota_snapshots":{
      "premium_models":{"entitlement":300,"quota_remaining":150},
      "premium_interactions":{"entitlement":15000,"quota_remaining":12000}}}"#;
    let r = parse(body).unwrap();
    assert_eq!(r.budget, 300.0);
    approx(150.0, r.used, 0.0005);
}

#[test]
fn free_plan_has_no_budget() {
    let body = r#"{"quota_reset_at":0,"quota_snapshots":{
      "premium_interactions":{"entitlement":0,"has_quota":false,"remaining":0},
      "chat":{"entitlement":200,"remaining":150},
      "completions":{"entitlement":2000,"remaining":1900}}}"#;
    assert!(parse(body).is_none());
}

#[test]
fn unlimited_has_no_budget() {
    assert!(parse(&interactions("\"unlimited\":true,\"entitlement\":15000,\"quota_remaining\":100")).is_none());
}
#[test]
fn used_falls_back_to_remaining() {
    approx(2645.0, parse(&interactions("\"entitlement\":15000,\"remaining\":12355")).unwrap().used, 0.0005);
}
#[test]
fn used_falls_back_to_percent_remaining() {
    approx(2655.0, parse(&interactions("\"entitlement\":15000,\"percent_remaining\":82.3")).unwrap().used, 0.0005);
}
#[test]
fn used_falls_back_to_credits_used() {
    approx(2644.2, parse(&interactions("\"entitlement\":15000,\"credits_used\":2644.2")).unwrap().used, 0.0005);
}
#[test]
fn no_used_field_means_nothing() {
    assert!(parse(&interactions("\"entitlement\":15000")).is_none());
}
#[test]
fn overage_count_reads_over_budget() {
    let r = parse(&interactions("\"entitlement\":15000,\"quota_remaining\":0,\"overage_count\":210")).unwrap();
    approx(15210.0, r.used, 0.0005);
    approx(101.4, r.percent(), 0.0005);
}
#[test]
fn negative_quota_remaining_reads_over_budget() {
    approx(15050.0, parse(&interactions("\"entitlement\":15000,\"quota_remaining\":-50")).unwrap().used, 0.0005);
}
#[test]
fn offset_timestamp_is_kept() {
    let r = parse(&interactions("\"entitlement\":15000,\"quota_remaining\":100,\"timestamp_utc\":\"2026-11-09T03:06:03.986-08:00\"")).unwrap();
    assert_eq!(r.observed_at, t("2026-11-09T11:06:03.986Z"));
}
#[test]
fn missing_timestamp_uses_now() {
    assert_eq!(parse(&interactions("\"entitlement\":15000,\"quota_remaining\":100")).unwrap().observed_at, now());
}
#[test]
fn missing_reset_date_gives_null_resets_at() {
    assert!(parse(&interactions("\"entitlement\":15000,\"quota_remaining\":100")).unwrap().resets_at.is_none());
}
#[test]
fn absurd_reset_date_counts_as_absent() {
    for reset in ["9999-12-31T23:59:59Z", "2028-01-01T00:00:00Z"] {
        let json = format!("{{\"quota_reset_date_utc\":\"{reset}\",\"quota_snapshots\":{{\"premium_interactions\":{{\"entitlement\":15000,\"quota_remaining\":100}}}}}}");
        assert!(parse(&json).unwrap().resets_at.is_none(), "{reset}");
    }
}
#[test]
fn reset_date_within_400_days_is_kept() {
    let json = "{\"quota_reset_date_utc\":\"2027-10-01T00:00:00Z\",\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":15000,\"quota_remaining\":100}}}";
    assert_eq!(parse(json).unwrap().resets_at, Some(t("2027-10-01T00:00:00Z")));
}
#[test]
fn malformed_bodies_give_null() {
    for body in ["", "[]", "{", "{\"quota_snapshots\":5}"] {
        assert!(parse(body).is_none(), "{body}");
    }
}

// ---- CopilotCacheTests

const HEADER: &str = "// Copilot user cache\n// do not edit\n";

fn business(remaining: i32) -> String {
    format!("{{\"quota_reset_date_utc\":\"2026-12-01T00:00:00Z\",\"quota_snapshots\":{{\"premium_interactions\":{{\"entitlement\":15000,\"quota_remaining\":{remaining}}}}}}}")
}
const FREE: &str = "{\"quota_snapshots\":{\"premium_interactions\":{\"entitlement\":0,\"has_quota\":false,\"remaining\":0}}}";

fn entry(key: &str, retrieved_at: &str, response: &str) -> String {
    format!("\"{key}\":{{\"schemaVersion\":1,\"generation\":\"g\",\"retrievedAt\":\"{retrieved_at}\",\"response\":{response}}}")
}
fn cache(entries: &[String]) -> String {
    format!("{HEADER}{{\"copilotUserCache\":{{{}}}}}", entries.join(","))
}

#[test]
fn newest_entry_with_a_budget_wins() {
    let text = cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000)), entry("v1:b", "2026-11-10T10:00:00Z", &business(10000))]);
    approx(5000.0, CopilotCache::parse(&text).unwrap().used, 0.0005);
}
#[test]
fn newer_free_entry_does_not_hide_business() {
    let text = cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000)), entry("v1:b", "2026-11-10T11:00:00Z", FREE)]);
    approx(3000.0, CopilotCache::parse(&text).unwrap().used, 0.0005);
}
#[test]
fn reading_is_marked_cache() {
    let text = cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000))]);
    assert_eq!(CopilotCache::parse(&text).unwrap().source, CopilotSource::Cache);
}
#[test]
fn retrieved_at_is_used_when_snapshot_has_no_timestamp() {
    let text = cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000))]);
    assert_eq!(CopilotCache::parse(&text).unwrap().observed_at, t("2026-11-10T09:25:00Z"));
}
#[test]
fn comment_lines_with_leading_spaces_are_dropped() {
    let text = format!("   // x\n{}", cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000))]));
    assert!(CopilotCache::parse(&text).is_some());
}
#[test]
fn no_entries_with_a_budget_gives_null() {
    assert!(CopilotCache::parse(&cache(&[entry("v1:a", "2026-11-10T09:25:00Z", FREE)])).is_none());
}
#[test]
fn malformed_or_truncated_cache_gives_null() {
    for text in ["", "// only a comment", "{\"copilotUserCache\":{\"v1:a\":{\"retrievedAt\":\"x\",\"response\":{"] {
        assert!(CopilotCache::parse(text).is_none(), "{text}");
    }
}
#[test]
fn missing_file_gives_null() {
    let d = Scratch::new();
    assert!(CopilotCache::read(&d.path("none.json")).is_none());
}
#[test]
fn invalid_path_gives_null() {
    assert!(CopilotCache::read("bad\0path.json").is_none());
}
#[test]
fn locked_file_gives_null() {
    let d = Scratch::new();
    let p = d.path("c.json");
    std::fs::write(&p, cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000))])).unwrap();
    let _held = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&p).unwrap();
    assert!(CopilotCache::read(&p).is_none());
}
#[test]
fn read_parses_an_existing_file() {
    let d = Scratch::new();
    let p = d.path("c.json");
    std::fs::write(&p, cache(&[entry("v1:a", "2026-11-10T09:25:00Z", &business(12000))])).unwrap();
    approx(3000.0, CopilotCache::read(&p).unwrap().used, 0.0005);
}
#[test]
fn default_path_is_under_local_app_data() {
    let env = |k: &str| (k == "LOCALAPPDATA").then(|| r"C:\L".to_string());
    assert_eq!(CopilotCache::default_path(&env), r"C:\L\copilot\copilot-user-cache.json");
}

// ---- CopilotCliCredentialTests

const NOT_FOUND: i32 = 1168;
const NO_SUCH_LOGON_SESSION: i32 = 1312;

#[test]
fn the_filter_asks_for_the_cli_entry_by_name() {
    assert_eq!(CopilotCliCredential::FILTER, "copilot-cli*");
}
#[test]
fn the_entry_has_its_name_anywhere() {
    for name in ["copilot-cli/https://github.com:someone", "Legacy copilot-cli token", "COPILOT-CLI"] {
        assert!(CopilotCliCredential::is_ours(Some(name)), "{name}");
    }
}
// The second is a key another Copilot product keeps there, not a sign-in: it must never be read.
#[test]
fn other_entries_are_not_ours() {
    for name in [Some("git:https://github.com"), Some("copilot-language-server/oauth-token-key"), Some("copilot"), Some(""), None] {
        assert!(!CopilotCliCredential::is_ours(name), "{name:?}");
    }
}
#[test]
fn not_found_is_no_entry_where_the_cli_has_not_been_used() {
    assert_eq!(CopilotCliCredential::after_failure(NOT_FOUND, false), ListingAnswer::NoEntry);
}
#[test]
fn not_found_lists_everything_only_where_the_cli_has_been_used() {
    assert_eq!(CopilotCliCredential::after_failure(NOT_FOUND, true), ListingAnswer::ListEverything);
}
#[test]
fn any_other_error_is_a_failure_not_an_empty_answer() {
    for used in [true, false] {
        assert_eq!(CopilotCliCredential::after_failure(NO_SUCH_LOGON_SESSION, used), ListingAnswer::Failed);
    }
}
#[test]
fn a_failed_listing_of_everything_is_judged_the_same_way() {
    assert_eq!(CopilotCliCredential::after_listing_everything_failed(NOT_FOUND), ListingAnswer::NoEntry);
    assert_eq!(CopilotCliCredential::after_listing_everything_failed(NO_SUCH_LOGON_SESSION), ListingAnswer::Failed);
}

// ---- CopilotStateTests

fn snow() -> Time {
    t("2026-11-10T09:45:00Z")
}
fn seen() -> Time {
    t("2026-11-10T09:25:00Z")
}
fn reading_at(observed: Time, source: CopilotSource) -> CopilotReading {
    CopilotReading { budget: 15000.0, used: 2644.2, resets_at: None, observed_at: observed, source }
}
fn line(state: ProviderState, r: &CopilotReading) -> Option<String> {
    sobreiro_core::display::When::set_clock(sobreiro_core::display::Clock::Hours24);
    CopilotStateText::line(state, r, snow(), Zone::utc())
}

#[test]
fn fresh_live_ok_has_no_line() {
    assert_eq!(line(ProviderState::Ok, &reading_at(snow() - mins(15), CopilotSource::Live)), None);
}
#[test]
fn old_live_ok_says_as_of() {
    assert_eq!(line(ProviderState::Ok, &reading_at(seen(), CopilotSource::Live)).as_deref(), Some("As of 09:25."));
}
#[test]
fn cache_says_from_the_copilot_cli() {
    assert_eq!(line(ProviderState::Ok, &reading_at(seen(), CopilotSource::Cache)).as_deref(), Some("From the Copilot CLI, as of 09:25."));
}
#[test]
fn problem_comes_before_source() {
    assert_eq!(
        line(ProviderState::Offline, &reading_at(seen(), CopilotSource::Cache)).as_deref(),
        Some("Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes. From the Copilot CLI, as of 09:25.")
    );
}
#[test]
fn each_problem_has_its_sentence() {
    for (state, sentence) in [
        (ProviderState::NoSignIn, "No GitHub sign-in Sobreiro can use. Sign in with the Copilot CLI or gh."),
        (ProviderState::Refused, "GitHub didn't accept the sign-in. Sign in again with the Copilot CLI or gh."),
        (ProviderState::RateLimited, "GitHub asked Sobreiro to wait. Nothing to do: it asks again by itself."),
        (ProviderState::Offline, "Sobreiro can't reach GitHub. Check the connection; it tries again every 2 minutes."),
        (ProviderState::BadAnswer, "GitHub's usage address refused, or answered in a way this Sobreiro doesn't understand. If it lasts, look for a newer Sobreiro."),
    ] {
        assert_eq!(line(state, &reading_at(snow(), CopilotSource::Live)).as_deref(), Some(sentence));
    }
}
#[test]
fn pick_takes_the_newer_and_live_on_a_tie() {
    let live = reading_at(seen(), CopilotSource::Live);
    let newer_cache = reading_at(seen() + mins(1), CopilotSource::Cache);
    let tied_cache = reading_at(seen(), CopilotSource::Cache);
    assert_eq!(CopilotShown::pick(Some(&live), Some(&newer_cache)), Some(newer_cache.clone()));
    assert_eq!(CopilotShown::pick(Some(&live), Some(&tied_cache)), Some(live.clone()));
    assert_eq!(CopilotShown::pick(Some(&live), Some(&reading_at(seen() - mins(1), CopilotSource::Cache))), Some(live.clone()));
    assert_eq!(CopilotShown::pick(Some(&live), None), Some(live.clone()));
    assert_eq!(CopilotShown::pick(None, Some(&tied_cache)), Some(tied_cache));
    assert_eq!(CopilotShown::pick(None, None), None);
}

// ---- CopilotUsageClientTests (the meaning of an answer; no request is made)

#[test]
fn ok_with_a_budget_gives_the_reading() {
    let r = CopilotUsageClient::interpret(200, None, Some(BUSINESS), now());
    assert_eq!(r.outcome, CopilotOutcome::Ok);
    approx(2644.2, r.reading.unwrap().used, 0.05);
    assert!(r.retry_after.is_none());
}
#[test]
fn ok_without_a_budget_is_no_budget() {
    let r = CopilotUsageClient::interpret(200, None, Some(FREE), now());
    assert_eq!(r.outcome, CopilotOutcome::NoBudget);
    assert!(r.reading.is_none());
}
#[test]
fn refusals() {
    for code in [401, 403, 404] {
        let r = CopilotUsageClient::interpret(code, None, None, now());
        assert_eq!(r.outcome, CopilotOutcome::Refused);
        assert!(r.reading.is_none());
    }
}
#[test]
fn too_many_requests_carries_retry_after() {
    let r = CopilotUsageClient::interpret(429, Some("120"), None, now());
    assert_eq!((r.outcome, r.retry_after), (CopilotOutcome::RateLimited, Some(mins(2))));
}
#[test]
fn server_error_is_failed() {
    assert_eq!(CopilotUsageClient::interpret(500, None, None, now()).outcome, CopilotOutcome::Failed);
}
#[test]
fn unparsable_200_is_failed() {
    for body in ["{", "[1,2]", ""] {
        let r = CopilotUsageClient::interpret(200, None, Some(body), now());
        assert_eq!(r.outcome, CopilotOutcome::Failed, "{body}");
        assert!(r.reading.is_none());
    }
}
#[test]
fn redirect_is_failed() {
    assert_eq!(CopilotUsageClient::interpret(302, None, None, now()).outcome, CopilotOutcome::Failed);
}

// ---- CopilotProviderTests (the poll and the cache read are passed in)

fn pnow() -> Time {
    t("2026-11-10T12:00:00Z")
}
const OK_LOG: &str = "copilot Ok env=ok credential-manager=skipped gh=skipped";
const OFFLINE_LOG: &str = "copilot Offline env=unreachable credential-manager=skipped gh=skipped";

fn live(at: Time) -> CopilotReading {
    CopilotReading { budget: 15000.0, used: 2645.0, resets_at: Some(pnow() + days(18)), observed_at: at, source: CopilotSource::Live }
}
fn cached(at: Time) -> CopilotReading {
    CopilotReading { source: CopilotSource::Cache, ..live(at) }
}
fn poll_result(state: ProviderState, reading: Option<CopilotReading>, log: &str) -> CopilotPollResult {
    CopilotPollResult { schedule: LiveSchedule::start(pnow()), state, reading, log: log.into() }
}

type PollBox = Arc<Mutex<Box<dyn Fn() -> CopilotPollResult + Send>>>;
type CacheBox = Arc<Mutex<Box<dyn Fn() -> Option<CopilotReading> + Send>>>;

struct Rig {
    provider: CopilotProvider,
    poll: PollBox,
    cache: CacheBox,
    logged: Rc<RefCell<Vec<String>>>,
}

impl Rig {
    fn new() -> Rig {
        let poll: PollBox = Arc::new(Mutex::new(Box::new(|| poll_result(ProviderState::Ok, Some(live(pnow())), OK_LOG))));
        let cache: CacheBox = Arc::new(Mutex::new(Box::new(|| None)));
        let logged = Rc::new(RefCell::new(Vec::new()));
        let (p, c, l) = (poll.clone(), cache.clone(), logged.clone());
        let provider = CopilotProvider::with_polls(
            Arc::new(move |_, _, _| (p.lock().unwrap())()),
            Arc::new(move || (c.lock().unwrap())()),
            Box::new(move |s| l.borrow_mut().push(s.to_string())),
        );
        Rig { provider, poll, cache, logged }
    }
    fn returns(&self, result: CopilotPollResult, cache: Option<CopilotReading>) {
        *self.poll.lock().unwrap() = Box::new(move || result.clone());
        *self.cache.lock().unwrap() = Box::new(move || cache.clone());
    }
    fn job(&self) -> ProviderPoll {
        self.provider.poll_job(LiveSchedule::start(pnow()), pnow())(&CancelToken::default())
    }
    fn kept(&mut self) -> ProviderPoll {
        let poll = self.job();
        self.provider.keep(poll.kept.clone().expect("a copilot poll always keeps"), pnow());
        poll
    }
}

#[test]
fn it_carries_copilots_name() {
    assert_eq!(Rig::new().provider.name(), "GitHub Copilot");
}
#[test]
fn nothing_is_shown_before_a_poll() {
    let mut rig = Rig::new();
    assert!(rig.provider.shown().is_none());
    assert!(rig.provider.limits(pnow()).is_empty());
}
#[test]
fn a_poll_passes_on_the_schedule_the_state_and_the_line() {
    let rig = Rig::new();
    let schedule = LiveSchedule::start(pnow());
    rig.returns(CopilotPollResult { schedule, state: ProviderState::Ok, reading: Some(live(pnow())), log: OK_LOG.into() }, None);
    let poll = rig.job();
    assert_eq!((poll.schedule, poll.state, poll.log.as_str()), (schedule, ProviderState::Ok, OK_LOG));
}
#[test]
fn the_reading_is_shown_only_after_keep() {
    let mut rig = Rig::new();
    let l = live(pnow());
    rig.returns(poll_result(ProviderState::Ok, Some(l.clone()), OK_LOG), None);
    let poll = rig.job();
    assert!(rig.provider.shown().is_none());
    rig.provider.keep(poll.kept.unwrap(), pnow());
    assert_eq!(rig.provider.shown(), Some(l));
    let limits = rig.provider.limits(pnow());
    assert_eq!(limits.len(), 1);
    assert_eq!(limits[0].kind, LimitKind::Credits);
}
#[test]
fn the_later_of_live_and_cache_is_shown() {
    let mut rig = Rig::new();
    rig.returns(poll_result(ProviderState::Ok, Some(live(pnow())), OK_LOG), Some(cached(pnow() + mins(5))));
    rig.kept();
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);
    let mut older = Rig::new();
    older.returns(poll_result(ProviderState::Ok, Some(live(pnow())), OK_LOG), Some(cached(pnow() - mins(5))));
    older.kept();
    assert_eq!(older.provider.shown().unwrap().source, CopilotSource::Live);
}
#[test]
fn the_line_says_how_the_cache_was_used() {
    for (cache, ends) in [(None, " cache=none"), (Some(cached(pnow() - mins(5))), " cache=older"), (Some(cached(pnow() + mins(5))), " cache=used")] {
        let mut rig = Rig::new();
        rig.returns(poll_result(ProviderState::Ok, Some(live(pnow())), OK_LOG), cache);
        rig.kept();
        let logged = rig.logged.borrow();
        assert_eq!(logged.len(), 1);
        assert!(logged[0].ends_with(ends), "{}", logged[0]);
    }
}
#[test]
fn the_line_is_logged_only_when_it_changes() {
    let mut rig = Rig::new();
    rig.returns(poll_result(ProviderState::Ok, Some(live(pnow())), OK_LOG), None);
    rig.kept();
    rig.kept();
    assert_eq!(rig.logged.borrow().len(), 1);
    rig.returns(poll_result(ProviderState::NoSignIn, None, "copilot NoSignIn env=none credential-manager=none gh=none"), None);
    rig.kept();
    assert_eq!(rig.logged.borrow().len(), 2);
}
#[test]
fn a_poll_that_is_not_kept_logs_nothing() {
    let rig = Rig::new();
    let _ = rig.job();
    assert!(rig.logged.borrow().is_empty());
}
#[test]
fn a_failed_poll_keeps_the_live_reading() {
    let mut rig = Rig::new();
    let l = live(pnow());
    rig.returns(poll_result(ProviderState::Ok, Some(l.clone()), OK_LOG), None);
    rig.kept();
    rig.returns(poll_result(ProviderState::Offline, None, OFFLINE_LOG), None);
    rig.kept();
    assert_eq!(rig.provider.shown(), Some(l));
}
#[test]
fn a_cache_that_cannot_be_read_keeps_the_one_before() {
    let mut rig = Rig::new();
    let c = cached(pnow());
    rig.returns(poll_result(ProviderState::Offline, None, OFFLINE_LOG), Some(c.clone()));
    rig.kept();
    *rig.cache.lock().unwrap() = Box::new(|| None);
    rig.kept();
    assert_eq!(rig.provider.shown(), Some(c));
}
#[test]
fn the_early_cache_is_taken_only_when_none_is_held() {
    let mut rig = Rig::new();
    rig.provider.take_early(cached(pnow()));
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);
    let mut held = Rig::new();
    let newer = cached(pnow() + mins(5));
    held.returns(poll_result(ProviderState::Offline, None, OFFLINE_LOG), Some(newer.clone()));
    held.kept();
    held.provider.take_early(cached(pnow() - mins(60)));
    assert_eq!(held.provider.shown(), Some(newer));
}
#[test]
fn a_poll_that_panics_counts_as_a_bad_answer() {
    let mut rig = Rig::new();
    *rig.poll.lock().unwrap() = Box::new(|| panic!("ghp_SECRET"));
    let poll = rig.job();
    assert_eq!(poll.state, ProviderState::BadAnswer);
    assert_eq!(poll.log, "copilot BadAnswer Failed");
    assert_eq!(poll.schedule.next_due, pnow() + mins(2));
    assert_eq!(poll.schedule.misses, 1);
    rig.provider.keep(poll.kept.clone().unwrap(), pnow());
    assert_eq!(*rig.logged.borrow(), vec!["copilot BadAnswer Failed cache=none".to_string()]);
    assert!(!poll.log.contains("ghp_SECRET"));
}
#[test]
fn a_cache_read_that_panics_counts_as_a_bad_answer() {
    let mut rig = Rig::new();
    *rig.cache.lock().unwrap() = Box::new(|| panic!("ghp_SECRET"));
    let poll = rig.job();
    assert_eq!((poll.state, poll.log.as_str()), (ProviderState::BadAnswer, "copilot BadAnswer Failed"));
    rig.provider.keep(poll.kept.unwrap(), pnow());
    assert!(rig.provider.shown().is_none());
    assert_eq!(*rig.logged.borrow(), vec!["copilot BadAnswer Failed cache=none".to_string()]);
}
#[test]
fn the_cache_job_reads_off_the_shells_thread() {
    let mut rig = Rig::new();
    *rig.cache.lock().unwrap() = Box::new(|| Some(cached(pnow())));
    let job = rig.provider.read_cache_job();
    let read = std::thread::spawn(job).join().unwrap();
    rig.provider.take_early(read.unwrap());
    assert_eq!(rig.provider.shown().unwrap().source, CopilotSource::Cache);
}
