//! The pill's data step: Claude's and Copilot's pollers, and from what they hold, the settings
//! and the clock, the model the pill draws, or none when the user keeps the pill off.
//!
//! Nothing here starts a thread, touches a window or reads the registry, so a test drives it
//! with Core's providers and a made-up clock. The shell runs the polls this hands out on
//! threads of its own and hands the answers back (see `pill_host`).
use chrono::{DateTime, Utc};
use sobreiro_core::config::{AppConfig, ShowOption};
use sobreiro_core::connect::ConnectionState;
use sobreiro_core::display::pill_model::PillModel;
use sobreiro_core::display::zone::Zone;
use sobreiro_core::limits::{Limit, limit_list};
use sobreiro_core::polling::Provider;
use sobreiro_core::polling::provider_poller::{Finished, PollRun, ProviderPoller};
use std::path::Path;
use std::sync::Arc;

/// Which of the two services a poll belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Claude,
    Copilot,
}

/// A poll to run away from the UI thread, and whose answer to hand back with `service`.
pub struct Run {
    pub service: Service,
    pub poll: PollRun,
}

/// What handing an answer back came to.
pub struct Handled {
    /// A line for the diagnostics log: the poll's own, when it is not the last one told for
    /// this service. It never holds a token.
    pub log: Option<String>,
    /// Polls that are due now and have to be run.
    pub runs: Vec<Run>,
}

/// What the step looks at besides the providers.
pub struct Surroundings<'a> {
    pub show: ShowOption,
    pub zone: &'a Zone,
    /// Claude Code's sign-in file is on this PC.
    pub credentials_file_exists: bool,
}

/// The two pollers and the last line told for each.
pub struct Feed {
    claude: ProviderPoller,
    copilot: ProviderPoller,
    last_logged: [Option<String>; 2],
}

impl Feed {
    pub fn new(claude: Arc<dyn Provider>, copilot: Arc<dyn Provider>) -> Feed {
        Feed {
            claude: ProviderPoller::new(claude),
            copilot: ProviderPoller::new(copilot),
            last_logged: [None, None],
        }
    }

    /// Switches both providers on: a poll of each is due at once.
    pub fn start(&mut self, now: DateTime<Utc>) -> Vec<Run> {
        let claude = self.claude.start(now);
        let copilot = self.copilot.start(now);
        runs(claude, copilot)
    }

    /// The polls that are due at `now`.
    pub fn tick(&mut self, now: DateTime<Utc>) -> Vec<Run> {
        let claude = self.claude.tick(now);
        let copilot = self.copilot.tick(now);
        runs(claude, copilot)
    }

    /// Takes the answer of a poll that was run, at `now`.
    pub fn done(&mut self, service: Service, finished: Finished, now: DateTime<Utc>) -> Handled {
        let poller = match service {
            Service::Claude => &mut self.claude,
            Service::Copilot => &mut self.copilot,
        };
        let done = poller.done(finished, now);
        let next = done.next.map(|poll| Run { service, poll });
        let slot = &mut self.last_logged[service as usize];
        let log = done.log.filter(|line| slot.as_ref() != Some(line));
        if let Some(line) = &log {
            *slot = Some(line.clone());
        }
        Handled {
            log,
            runs: next.into_iter().collect(),
        }
    }

    /// The model the pill draws at `now`, as 1.1's `UpdatePill` makes it: `None` when the
    /// settings keep the pill off. `connection` is asked only when Claude has no limits, as in
    /// 1.1: with limits Claude is on this PC and its settings need not be read.
    pub fn model(
        &self,
        now: DateTime<Utc>,
        around: &Surroundings<'_>,
        connection: impl FnOnce() -> ConnectionState,
    ) -> Option<PillModel> {
        if around.show != ShowOption::TrayAndPill {
            return None;
        }
        let claude = self.claude.provider().limits(now);
        let copilot = self.copilot.provider().limits(now);
        let connection = if claude.is_empty() {
            connection()
        } else {
            ConnectionState::NotConnected
        };
        let on_pc =
            limit_list::claude_on_pc(around.credentials_file_exists, claude.len(), connection);
        let show_claude = limit_list::show_claude(on_pc, !copilot.is_empty());
        let shown: Vec<Limit> = if show_claude {
            claude.into_iter().chain(copilot).collect()
        } else {
            copilot
        };
        Some(PillModel::from_states(
            &shown,
            around.zone,
            show_claude,
            self.claude.state(),
            misses(&self.claude),
            self.copilot.state(),
            misses(&self.copilot),
            now,
        ))
    }
}

fn runs(claude: Option<PollRun>, copilot: Option<PollRun>) -> Vec<Run> {
    let claude = claude.map(|poll| Run {
        service: Service::Claude,
        poll,
    });
    let copilot = copilot.map(|poll| Run {
        service: Service::Copilot,
        poll,
    });
    claude.into_iter().chain(copilot).collect()
}

fn misses(poller: &ProviderPoller) -> i32 {
    i32::try_from(poller.misses()).unwrap_or(i32::MAX)
}

/// `config.json` for drawing, as 1.1's `ConfigForDisplay`: a file that cannot be read, or is not
/// JSON, draws as the default. The second value says it could not be read. It is never written
/// from here.
pub fn config_for_display(path: &Path) -> (AppConfig, bool) {
    match AppConfig::load(path) {
        Ok(config) => (config, false),
        Err(_) => (AppConfig::default(), true),
    }
}
