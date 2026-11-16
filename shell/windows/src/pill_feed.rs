//! The data step: Claude's and Copilot's pollers, and from what they hold, the settings and the
//! clock, one snapshot per redraw. The pill's model is a part of it, none when the user keeps the
//! pill off. The flyout's view is made from the snapshot on demand (`Snapshot::flyout`), so a
//! closed flyout costs nothing.
//!
//! Nothing here starts a thread, touches a window or reads the registry, so a test drives it
//! with Core's providers and a made-up clock. The shell runs the polls this hands out on
//! threads of its own and hands the answers back (see `app_host`).
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use sobreiro_core::colours::bar_colour::pick;
use sobreiro_core::colours::{Rgb, TaskbarColours};
use sobreiro_core::config::{AppConfig, ShowOption};
use sobreiro_core::connect::ConnectionState;
use sobreiro_core::copilot::CopilotReading;
use sobreiro_core::copilot::copilot_state_text;
use sobreiro_core::display::ClockFormat;
use sobreiro_core::display::FlyoutStyle;
use sobreiro_core::display::flyout_model::{FlyoutModel, FlyoutOptions};
use sobreiro_core::display::pill_model::PillModel;
use sobreiro_core::display::tooltip_text;
use sobreiro_core::display::zone::Zone;
use sobreiro_core::limits::{Limit, LimitKind, limit_list};
use sobreiro_core::polling::provider_poller::{Finished, PollRun, ProviderPoller};
use sobreiro_core::polling::{Provider, ProviderState};
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

/// What a snapshot is made from besides the providers, taken by value so the snapshot owns it.
pub struct Surroundings {
    pub config: AppConfig,
    /// Windows' look; the pill's colours and the flyout's palette are made from it.
    pub theme: Theme,
    pub zone: Zone,
    /// How the user writes a time of day, for the tooltip.
    pub clock: ClockFormat,
    /// Claude Code's sign-in file is on this PC.
    pub credentials_file_exists: bool,
    /// The Copilot reading shown (the provider's `shown`), for the flyout's note.
    pub copilot_reading: Option<CopilotReading>,
}

/// What the pill shows: the model it draws and the text of its tooltip.
pub struct Shown {
    pub model: PillModel,
    pub tooltip: String,
}

/// What one redraw draws from: the figures, what is shown of them, and the settings and
/// surroundings they were read with. Each part of the app (the pill, later the tray icons, the
/// flyout and the toasts) takes its own view of it.
pub struct Snapshot {
    pub now: DateTime<Utc>,
    pub claude: Vec<Limit>,
    pub copilot: Vec<Limit>,
    /// Where each poller stands, for the flyout's sentences.
    pub claude_state: ProviderState,
    pub copilot_state: ProviderState,
    /// The Copilot reading shown, if any.
    pub copilot_reading: Option<CopilotReading>,
    pub theme: Theme,
    /// Whether Claude is shown next to, or instead of, Copilot (Core's `limit_list`).
    pub show_claude: bool,
    pub config: AppConfig,
    pub colours: TaskbarColours,
    pub zone: Zone,
    pub clock: ClockFormat,
    /// The pill's model and tooltip; none while the settings keep the pill off.
    pub pill: Option<Shown>,
}

/// What the flyout's card is drawn from.
pub struct FlyoutView {
    pub model: FlyoutModel,
    pub style: FlyoutStyle,
    /// The palette is the dark one.
    pub dark: bool,
    /// The bar colour, picked for the theme.
    pub fill: Rgb,
}

impl Snapshot {
    /// The flyout's view, as 1.1's `UpdateFlyout` makes it: Claude's limits when Claude is shown,
    /// then Copilot's; Claude's state; Copilot's note only when a reading is shown.
    pub fn flyout(&self) -> FlyoutView {
        let limits: Vec<Limit> = if self.show_claude {
            self.claude
                .iter()
                .chain(self.copilot.iter())
                .cloned()
                .collect()
        } else {
            self.copilot.clone()
        };
        let copilot_note = self.copilot_reading.as_ref().and_then(|reading| {
            copilot_state_text::line(
                self.copilot_state,
                reading,
                self.now,
                &self.zone,
                &self.clock,
            )
        });
        let options = FlyoutOptions {
            live: self.claude_state,
            copilot_note,
            show_claude: self.show_claude,
            days: self.config.pace_days,
        };
        let dark = !self.theme.light;
        FlyoutView {
            model: FlyoutModel::from(&limits, self.now, &self.zone, &self.clock, &options),
            style: self.config.style,
            dark,
            fill: pick(self.config.bar_colour, self.theme.accent.as_ref(), dark).colour,
        }
    }
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

    /// The flyout was opened at `now`: the polls of a reading over a minute old, or that are due.
    pub fn flyout_opened(&mut self, now: DateTime<Utc>) -> Vec<Run> {
        let claude = self.claude.flyout_opened(now);
        let copilot = self.copilot.flyout_opened(now);
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

    /// Everything one redraw draws from, made once at `now`, as 1.1's `Refresh` reads it. The
    /// limits and whether Claude shows are filled whatever Show says, as the tray icons and the
    /// flyout need them too; only the pill's part is none when the settings keep the pill off.
    /// `connection` is asked only when Claude has no limits, as in 1.1: with limits Claude is on
    /// this PC and its settings need not be read.
    pub fn snapshot(
        &self,
        now: DateTime<Utc>,
        around: Surroundings,
        connection: impl FnOnce() -> ConnectionState,
    ) -> Snapshot {
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
        let pill = (around.config.show == ShowOption::TrayAndPill).then(|| {
            let shown: Vec<Limit> = if show_claude {
                claude.iter().chain(copilot.iter()).cloned().collect()
            } else {
                copilot.clone()
            };
            let model = PillModel::from_states(
                &shown,
                &around.zone,
                show_claude,
                self.claude.state(),
                misses(&self.claude),
                self.copilot.state(),
                misses(&self.copilot),
                now,
            );
            let tooltip = tooltip(&shown, show_claude, now, &around.zone, &around.clock);
            Shown { model, tooltip }
        });
        Snapshot {
            now,
            claude,
            copilot,
            claude_state: self.claude.state(),
            copilot_state: self.copilot.state(),
            copilot_reading: around.copilot_reading,
            show_claude,
            colours: around.theme.colours(around.config.bar_colour),
            theme: around.theme,
            config: around.config,
            zone: around.zone,
            clock: around.clock,
            pill,
        }
    }
}

/// The tooltip's text as 1.1 puts it together: Claude's lines when Claude is shown, then
/// Copilot's when there are credits, one under the other.
fn tooltip(
    shown: &[Limit],
    show_claude: bool,
    now: DateTime<Utc>,
    zone: &Zone,
    clock: &ClockFormat,
) -> String {
    let mut parts = Vec::new();
    if show_claude {
        parts.push(tooltip_text::render(shown, now, zone, clock));
    }
    if let Some(credits) = shown.iter().find(|limit| limit.kind == LimitKind::Credits) {
        parts.push(tooltip_text::render_copilot(credits, now, zone, clock));
    }
    parts.join(
        "
",
    )
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
