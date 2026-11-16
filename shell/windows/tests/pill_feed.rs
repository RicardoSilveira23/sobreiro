//! The pill's data step: two providers, the settings and the clock in, the model the pill draws (or
//! none) out. The step is driven here with providers of the test's own, which ask nobody, and a
//! made-up clock; no window, no registry and no network is touched.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

// The shell is a program and not a library, so the test takes the two files it exercises. Both
// depend on Core alone; what the program calls from them and the test does not is let go.
#[allow(dead_code)]
#[path = "../src/pill_feed.rs"]
mod pill_feed;
#[allow(dead_code)]
#[path = "../src/theme.rs"]
mod theme;

use chrono::{DateTime, TimeDelta, Utc};
use pill_feed::{Feed, Run, Service, Shown, Snapshot, Surroundings, config_for_display};
use sobreiro_core::colours::{BarColourOption, TaskbarColours};
use sobreiro_core::config::{AppConfig, ShowOption};
use sobreiro_core::connect::ConnectionState;
use sobreiro_core::copilot::copilot_poll::CopilotPollResult;
use sobreiro_core::copilot::copilot_provider::CopilotProvider;
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::display::ClockFormat;
use sobreiro_core::display::pill_model::PillModel;
use sobreiro_core::display::zone::Zone;
use sobreiro_core::limits::{Limit, LimitKind, providers};
use sobreiro_core::live::claude_provider::ClaudeProvider;
use sobreiro_core::live::live_poll::{LivePollResult, PollOutcome};
use sobreiro_core::live::live_schedule::LiveSchedule;
use sobreiro_core::live::live_usage_client::Cancelled;
use sobreiro_core::polling::{Provider, ProviderPoll, ProviderState};
use sobreiro_core::usage::{LimitWindow, RateLimits};
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};
use theme::Theme;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-16T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/// A provider whose poll asks nobody: it answers with the state, the misses and the line it was
/// made with, and shows the limits it was made with.
struct Fake {
    name: &'static str,
    limits: Vec<Limit>,
    state: ProviderState,
    misses: u32,
    line: &'static str,
}

impl Fake {
    fn new(name: &'static str, limits: Vec<Limit>) -> Fake {
        Fake {
            name,
            limits,
            state: ProviderState::Ok,
            misses: 0,
            line: "Fake Ok",
        }
    }

    /// Its polls have failed twice in a row, so its figures are old news.
    fn stopped(mut self) -> Fake {
        self.state = ProviderState::Offline;
        self.misses = 2;
        self.line = "Fake Unreachable";
        self
    }
}

impl Provider for Fake {
    fn name(&self) -> &str {
        self.name
    }

    fn poll(
        &self,
        schedule: &LiveSchedule,
        _now: DateTime<Utc>,
        _cancelled: &AtomicBool,
    ) -> Result<ProviderPoll, Cancelled> {
        Ok(ProviderPoll {
            schedule: LiveSchedule {
                misses: self.misses,
                ..*schedule
            },
            state: self.state,
            log: self.line.to_string(),
            keep: None,
        })
    }

    fn limits(&self, _now: DateTime<Utc>) -> Vec<Limit> {
        self.limits.clone()
    }
}

/// A figure read `age` ago.
fn claude_limits(percent: f64, age: TimeDelta) -> Vec<Limit> {
    let seen = now() - age;
    let resets = Some(now() + TimeDelta::hours(2));
    vec![
        Limit::new(
            providers::CLAUDE,
            LimitKind::Session,
            "Session",
            percent,
            resets,
            seen,
            false,
        ),
        Limit::new(
            providers::CLAUDE,
            LimitKind::Weekly,
            "Weekly",
            percent + 30.0,
            resets,
            seen,
            false,
        ),
    ]
}

fn copilot_limits(age: TimeDelta) -> Vec<Limit> {
    vec![Limit::new(
        providers::COPILOT,
        LimitKind::Credits,
        "AI credits",
        17.0,
        Some(now() + TimeDelta::days(15)),
        now() - age,
        false,
    )]
}

/// Starts both providers and hands every answer back, as the shell does with its threads.
fn settled(claude: Fake, copilot: Fake) -> Feed {
    let mut feed = Feed::new(Arc::new(claude), Arc::new(copilot));
    let runs = feed.start(now());
    assert_eq!(runs.len(), 2, "both providers are due at once");
    answer(&mut feed, runs);
    feed
}

/// Runs the polls where they stand and hands the answers back; the lines come back.
fn answer(feed: &mut Feed, runs: Vec<Run>) -> Vec<Option<String>> {
    runs.into_iter()
        .map(|Run { service, poll }| feed.done(service, poll.run(), now()).log)
        .collect()
}

/// A 12-hour clock, as the US writes it.
static CLOCK: LazyLock<ClockFormat> = LazyLock::new(|| ClockFormat {
    pattern: "h:mm tt".to_string(),
    am: "AM".to_string(),
    pm: "PM".to_string(),
});

fn around(show: ShowOption, credentials: bool) -> Surroundings {
    Surroundings {
        config: AppConfig {
            show,
            ..AppConfig::default()
        },
        colours: TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None),
        zone: Zone::utc(),
        clock: CLOCK.clone(),
        credentials_file_exists: credentials,
    }
}

fn snapshot(feed: &Feed, show: ShowOption, credentials: bool) -> Snapshot {
    feed.snapshot(now(), around(show, credentials), || {
        ConnectionState::NotConnected
    })
}

fn shown(feed: &Feed, show: ShowOption, credentials: bool) -> Option<Shown> {
    snapshot(feed, show, credentials).pill
}

fn model(feed: &Feed, show: ShowOption, credentials: bool) -> Option<PillModel> {
    shown(feed, show, credentials).map(|shown| shown.model)
}

#[test]
fn fresh_readings_show_both_services_unmarked() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(1)),
        ),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let pill = model(&feed, ShowOption::TrayAndPill, true).unwrap();
    assert!(pill.show_claude);
    assert_eq!(pill.session.figure, "15%");
    assert_eq!(pill.weekly.figure, "45%");
    assert!(!pill.claude_stale);
    assert!(!pill.session.stale);
    let copilot = pill.copilot.unwrap();
    assert_eq!(copilot.credits.figure, "17%");
    assert_eq!(copilot.reset_date, "1 Dec");
    assert!(!copilot.stale);
}

#[test]
fn old_figures_of_a_stopped_provider_keep_their_numbers_marked_stale() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(20)),
        )
        .stopped(),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::hours(30))).stopped(),
    );
    let pill = model(&feed, ShowOption::TrayAndPill, true).unwrap();
    assert!(pill.claude_stale);
    assert_eq!(pill.session.figure, "15%");
    assert!(pill.session.stale);
    assert!(pill.weekly.stale);
    let copilot = pill.copilot.unwrap();
    assert_eq!(copilot.credits.figure, "17%");
    assert!(copilot.stale);
}

#[test]
fn a_stopped_provider_with_a_recent_figure_is_not_marked() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(5)),
        )
        .stopped(),
        Fake::new(providers::COPILOT, Vec::new()),
    );
    let pill = model(&feed, ShowOption::TrayAndPill, true).unwrap();
    assert!(!pill.claude_stale);
}

#[test]
fn no_reading_is_a_dash_and_never_stale() {
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()).stopped(),
        Fake::new(providers::COPILOT, Vec::new()).stopped(),
    );
    let pill = model(&feed, ShowOption::TrayAndPill, false).unwrap();
    // Nothing else is shown, so Claude is.
    assert!(pill.show_claude);
    assert_eq!(pill.session.figure, "–");
    assert_eq!(pill.weekly.figure, "–");
    assert!(!pill.session.stale);
    assert!(pill.copilot.is_none());
}

#[test]
fn copilot_alone_shows_when_claude_is_not_on_this_pc() {
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let pill = model(&feed, ShowOption::TrayAndPill, false).unwrap();
    assert!(!pill.show_claude);
    assert!(pill.copilot.is_some());
    // With Claude's sign-in file on the PC, both show.
    let pill = model(&feed, ShowOption::TrayAndPill, true).unwrap();
    assert!(pill.show_claude);
}

#[test]
fn a_connected_status_line_counts_as_claude_on_this_pc() {
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let pill = feed
        .snapshot(now(), around(ShowOption::TrayAndPill, false), || {
            ConnectionState::Connected
        })
        .pill
        .unwrap()
        .model;
    assert!(pill.show_claude);
}

#[test]
fn the_connection_is_read_only_when_claude_has_no_limits() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(1)),
        ),
        Fake::new(providers::COPILOT, Vec::new()),
    );
    let asked = Cell::new(false);
    let _ = feed.snapshot(now(), around(ShowOption::TrayAndPill, true), || {
        asked.set(true);
        ConnectionState::NotConnected
    });
    assert!(!asked.get());
}

#[test]
fn the_show_setting_hides_the_pill() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(1)),
        ),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    assert!(model(&feed, ShowOption::TrayOnly, true).is_none());
    assert!(model(&feed, ShowOption::TrayAndPill, true).is_some());
}

#[test]
fn with_the_pill_off_the_snapshot_still_holds_the_limits() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(1)),
        ),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let off = snapshot(&feed, ShowOption::TrayOnly, true);
    assert!(off.pill.is_none());
    assert_eq!(off.claude, claude_limits(15.0, TimeDelta::minutes(1)));
    assert_eq!(off.copilot, copilot_limits(TimeDelta::minutes(1)));
    assert!(off.show_claude);
    // The same figures as with the pill on.
    let on = snapshot(&feed, ShowOption::TrayAndPill, true);
    assert!(on.pill.is_some());
    assert_eq!(on.claude, off.claude);
    assert_eq!(on.copilot, off.copilot);
    assert_eq!(on.show_claude, off.show_claude);

    // Copilot alone when Claude is not on this PC, also with the pill off.
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let off = snapshot(&feed, ShowOption::TrayOnly, false);
    assert!(off.pill.is_none());
    assert!(off.claude.is_empty());
    assert_eq!(off.copilot.len(), 1);
    assert!(!off.show_claude);
}

#[test]
fn the_snapshot_carries_what_it_was_given() {
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, Vec::new()),
    );
    let mut given = around(ShowOption::TrayOnly, true);
    given.config.bar_colour = BarColourOption::Accent;
    given.colours = TaskbarColours::for_taskbar(true, false, BarColourOption::Accent, None);
    let colours = given.colours;
    let config = given.config.clone();
    let snapshot = feed.snapshot(now(), given, || ConnectionState::NotConnected);
    assert_eq!(snapshot.now, now());
    assert_eq!(snapshot.config, config);
    assert_eq!(snapshot.colours, colours);
    assert_eq!(snapshot.zone, Zone::utc());
    assert_eq!(snapshot.clock, *CLOCK);
}

#[test]
fn a_poll_line_is_logged_once_until_it_changes() {
    let mut feed = Feed::new(
        Arc::new(Fake::new(providers::CLAUDE, Vec::new())),
        Arc::new(Fake::new(providers::COPILOT, Vec::new())),
    );
    let started = feed.start(now());
    let first = answer(&mut feed, started);
    assert_eq!(first, vec![Some("Fake Ok".to_string()); 2]);

    // Five minutes on both are due again, and say the same.
    let later = now() + TimeDelta::minutes(5);
    let runs = feed.tick(later);
    assert_eq!(runs.len(), 2);
    for Run { service, poll } in runs {
        assert!(matches!(service, Service::Claude | Service::Copilot));
        assert_eq!(feed.done(service, poll.run(), later).log, None);
    }
}

#[test]
fn a_poll_under_way_is_not_started_again() {
    let mut feed = Feed::new(
        Arc::new(Fake::new(providers::CLAUDE, Vec::new())),
        Arc::new(Fake::new(providers::COPILOT, Vec::new())),
    );
    let started = feed.start(now());
    assert_eq!(started.len(), 2);
    assert!(feed.tick(now()).is_empty());
    answer(&mut feed, started);
}

#[test]
fn the_tooltip_is_core_s_text_for_each_service_shown() {
    let feed = settled(
        Fake::new(
            providers::CLAUDE,
            claude_limits(15.0, TimeDelta::minutes(1)),
        ),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    // Both: Claude's lines, then Copilot's, one under the other, as 1.1 joins them.
    let both = shown(&feed, ShowOption::TrayAndPill, true).unwrap().tooltip;
    let lines: Vec<&str> = both.lines().collect();
    assert_eq!(
        lines,
        [
            "Session 15% · resets 2:00 PM",
            "Weekly 45% · resets 2:00 PM",
            "as of 11:59 AM",
            "AI credits 17% · resets Tue 1 Dec 12:00 PM",
            "as of 11:59 AM",
        ]
    );
    // Copilot alone, as when Claude is not on this PC: only its lines.
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, copilot_limits(TimeDelta::minutes(1))),
    );
    let alone = shown(&feed, ShowOption::TrayAndPill, false)
        .unwrap()
        .tooltip;
    assert_eq!(
        alone,
        "AI credits 17% · resets Tue 1 Dec 12:00 PM\nas of 11:59 AM"
    );
}

#[test]
fn the_tooltip_with_no_reading_says_so() {
    let feed = settled(
        Fake::new(providers::CLAUDE, Vec::new()),
        Fake::new(providers::COPILOT, Vec::new()),
    );
    let tooltip = shown(&feed, ShowOption::TrayAndPill, true).unwrap().tooltip;
    assert_eq!(tooltip, "Sobreiro · no data yet");
}

/// A folder of the test's own, empty.
fn folder(name: &str) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn a_config_that_cannot_be_read_draws_as_the_default_and_is_left_as_it_is() {
    let folder = folder("pill_feed_config");
    let path = folder.join("config.json");
    std::fs::write(&path, "{ not json").unwrap();
    let (config, unreadable) = config_for_display(&path);
    assert!(unreadable);
    assert_eq!(config, AppConfig::default());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");

    // A missing file is the default and is not made.
    let missing = folder.join("missing.json");
    let (config, unreadable) = config_for_display(&missing);
    assert!(!unreadable);
    assert_eq!(config.show, ShowOption::TrayAndPill);
    assert!(!missing.exists());
}

#[test]
fn the_show_and_bar_colour_keys_are_read() {
    let folder = folder("pill_feed_config_keys");
    let path = folder.join("config.json");
    let text = r#"{ "show": "trayonly", "bar_colour": "accent", "other": 1 }"#;
    std::fs::write(&path, text).unwrap();
    let (config, unreadable) = config_for_display(&path);
    assert!(!unreadable);
    assert_eq!(config.show, ShowOption::TrayOnly);
    assert_eq!(config.bar_colour, BarColourOption::Accent);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn the_theme_picks_the_ink() {
    let dark = Theme::default().colours(BarColourOption::Purple);
    let light = Theme {
        light: true,
        ..Theme::default()
    }
    .colours(BarColourOption::Purple);
    assert_ne!(dark.ink, light.ink);
    assert_eq!(dark, Theme::default().colours(BarColourOption::Purple));
}

/// Core's own providers, with polls that ask nobody: the readings they bring are kept (Claude's in
/// a `usage.json` of the test's own) and come back through `limits()` into the model.
#[test]
fn cores_providers_feed_the_pill() {
    let folder = folder("pill_feed_providers");
    let schedule = LiveSchedule::start(now());
    let claude = ClaudeProvider::with_poll(
        move |_, _, _| {
            Ok(LivePollResult {
                schedule,
                state: ProviderState::Ok,
                limits: Some(RateLimits {
                    five_hour: Some(LimitWindow {
                        used_percentage: 27.0,
                        resets_at: Some(now() + TimeDelta::hours(2)),
                        observed_at: None,
                    }),
                    seven_day: None,
                    spend: None,
                }),
                outcome: PollOutcome::Ok,
            })
        },
        &folder.join("usage.json"),
        None,
    );
    let copilot = CopilotProvider::with_poll(
        move |_, _, _| {
            Ok(CopilotPollResult {
                schedule,
                state: ProviderState::Ok,
                reading: Some(CopilotReading {
                    budget: 100.0,
                    used: 17.0,
                    resets_at: Some(now() + TimeDelta::days(15)),
                    observed_at: now(),
                    source: CopilotSource::Live,
                }),
                log: "Copilot poll ok".to_string(),
            })
        },
        || None,
        Arc::new(|_: &str| {}),
    );
    let mut feed = Feed::new(Arc::new(claude), Arc::new(copilot));

    // Before a poll is kept there is nothing to show: dashes, and no Copilot block.
    let empty = model(&feed, ShowOption::TrayAndPill, false).unwrap();
    assert_eq!(empty.session.figure, "–");
    assert!(empty.copilot.is_none());

    let runs = feed.start(now());
    answer(&mut feed, runs);
    let pill = model(&feed, ShowOption::TrayAndPill, true).unwrap();
    assert_eq!(pill.session.figure, "27%");
    assert!(!pill.claude_stale);
    let copilot = pill.copilot.unwrap();
    assert_eq!(copilot.credits.figure, "17%");
    assert_eq!(copilot.reset_date, "1 Dec");
}
