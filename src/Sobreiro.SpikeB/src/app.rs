//! SPIKE (shell candidate B): today's tray controller cut down to what the spike shows. The ported logic does
//! the reading, the polling and every word; this draws the two tray icons, the pill and the flyout.
//! Left out: Connect and Disconnect, the alerts, the settings in the menu other than Style, the first-run
//! migration, the bridge, the log, and the theme, time and display events (a redraw every 30 seconds stands in).
use crate::flyout::FlyoutWindow;
use crate::{Tray, faces, native, pill, trace, watch};
use sobreiro_core::colours::{BarColour, NightPalette, TaskbarColours};
use sobreiro_core::config::{AppConfig, ShowOption};
use sobreiro_core::connect::{ClaudePaths, ConnectionState};
use sobreiro_core::copilot::{
    CopilotCache, CopilotProvider, CopilotSignIn, CopilotSourceKind, CopilotStateText, CopilotToken, CopilotUsageClient, CredentialStore,
    GhCli, TokenSearch,
};
use sobreiro_core::display::{FlyoutModel, FlyoutStyle, PillModel, RingsModel, TooltipText};
use sobreiro_core::limits::{Limit, LimitKind, LimitList};
use sobreiro_core::live::{ClaudeProvider, LiveUsageClient};
use sobreiro_core::polling::{PollRun, ProviderPoller};
use sobreiro_core::theme::WindowsTheme;
use sobreiro_core::Zone;
use sobreiro_core::app_paths::AppPaths;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const USER_AGENT: &str = "Sobreiro/1.0.0";

struct App {
    claude_icon: Tray,
    copilot_icon: Tray,
    claude: ProviderPoller<ClaudeProvider>,
    copilot: ProviderPoller<CopilotProvider>,
    flyout: Rc<FlyoutWindow>,
    tick: slint::Timer,
    debounce: slint::Timer,
    // What each icon was last drawn from: an icon is drawn again only when that changes.
    claude_drawn: Option<(RingsModel, TaskbarColours, u32)>,
    copilot_drawn: Option<(RingsModel, TaskbarColours, u32)>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

fn with_app(part: &str, action: impl FnOnce(&mut App)) {
    APP.with(|app| match app.try_borrow_mut() {
        Ok(mut app) => {
            if let Some(app) = app.as_mut() {
                action(app)
            }
        }
        Err(_) => trace!("{part}: skipped, the controller was busy"),
    });
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

// The poll runs on a thread of its own; its answer is handled on the UI thread.
fn runner(which: fn(&mut App, sobreiro_core::polling::PollDone)) -> Box<dyn FnMut(PollRun)> {
    Box::new(move |run: PollRun| {
        let _ = std::thread::Builder::new().name("usage poll".into()).spawn(move || {
            let done = run.run();
            let _ = slint::invoke_from_event_loop(move || with_app("poll answer", |app| which(app, done)));
        });
    })
}

// A provider says its numbers changed from inside a call the controller made: the redraw waits for that call to end.
fn changed() -> Box<dyn FnMut()> {
    Box::new(|| {
        slint::Timer::single_shot(Duration::ZERO, || with_app("changed", |app| app.refresh(None)));
    })
}

pub fn run() -> Result<(), slint::PlatformError> {
    let data = AppPaths::data_directory();
    let _ = std::fs::create_dir_all(&data);

    let claude = ProviderPoller::new(
        ClaudeProvider::new(LiveUsageClient::new(USER_AGENT), ClaudePaths::credentials_file(&env), AppPaths::usage_file(), None),
        runner(|app, done| app.claude.done(done, sobreiro_core::now())),
        changed(),
        None,
    );
    let sign_ins = vec![
        CopilotSignIn::new(CopilotSourceKind::Env, || Ok(TokenSearch::from(CopilotToken::from_environment(&env)))),
        CopilotSignIn::new(CopilotSourceKind::CredentialManager, CredentialStore::copilot_tokens),
        CopilotSignIn::new(CopilotSourceKind::Gh, || Ok(TokenSearch::from(GhCli::token()))),
    ];
    let copilot = ProviderPoller::new(
        CopilotProvider::new(CopilotUsageClient::new(USER_AGENT), sign_ins, CopilotCache::default_path(&env), Box::new(|_| {})),
        runner(|app, done| app.copilot.done(done, sobreiro_core::now())),
        changed(),
        None,
    );

    let claude_icon = tray()?;
    let copilot_icon = tray()?;

    pill::on_clicks(
        || with_app("pill click", App::clicked),
        || trace!("pill right click: the toolkit's tray menu cannot be opened from here"),
    );

    let mut app = App {
        claude_icon,
        copilot_icon,
        claude,
        copilot,
        flyout: Rc::new(FlyoutWindow::default()),
        tick: slint::Timer::default(),
        debounce: slint::Timer::default(),
        claude_drawn: None,
        copilot_drawn: None,
    };
    app.tick.start(slint::TimerMode::Repeated, Duration::from_secs(30), || {
        with_app("tick", |app| {
            let now = sobreiro_core::now();
            app.claude.tick(now);
            app.copilot.tick(now);
            app.refresh(None);
        })
    });
    watch::folder(&data, || {
        let _ = slint::invoke_from_event_loop(|| {
            with_app("file event", |app| {
                app.debounce.start(slint::TimerMode::SingleShot, Duration::from_millis(250), || {
                    with_app("file change", |app| app.refresh(None))
                });
            })
        });
    });

    app.refresh(None);
    let now = sobreiro_core::now();
    app.claude.start(now);
    app.copilot.start(now);
    let read_cache = app.copilot.provider().read_cache_job();
    let _ = std::thread::Builder::new().name("copilot cache".into()).spawn(move || {
        if let Some(cache) = read_cache() {
            let _ = slint::invoke_from_event_loop(move || {
                with_app("copilot cache", |app| {
                    app.copilot.provider_mut().take_early(cache);
                    app.refresh(None);
                })
            });
        }
    });
    APP.set(Some(app));

    // SOBREIRO_SPIKE_SELFTEST=flyout opens the flyout by itself after 5 seconds and closes it 3 seconds later,
    // with no click on the pill, to tell what an opening costs from what a click costs.
    if crate::spike::env("SOBREIRO_SPIKE_SELFTEST").as_deref() == Some("flyout") {
        slint::Timer::single_shot(Duration::from_secs(5), || with_app("selftest open", App::clicked));
        slint::Timer::single_shot(Duration::from_secs(8), || with_app("selftest close", App::clicked));
    }

    let result = slint::run_event_loop_until_quit();
    pill::dispose();
    if let Some(mut app) = APP.take() {
        app.claude.dispose();
        app.copilot.dispose();
    }
    result
}

// The toolkit's tray element. Its menu is the system's own. The toolkit makes the icon once, on the event
// loop's first turn, from the picture it has then, and gives up without one: so each starts with empty rings.
fn tray() -> Result<Tray, slint::PlatformError> {
    let tray = Tray::new()?;
    let colours = TaskbarColours::for_taskbar(WindowsTheme::is_light(), false, sobreiro_core::colours::BarColourOption::Purple, None);
    tray.set_rings(faces::tray_icon(&RingsModel::from(&[]), &colours, native::small_icon_size()));
    tray.on_icon_clicked(|| with_app("icon click", App::clicked));
    tray.on_pick_style(|index| {
        with_app("style", |app| {
            let path = AppPaths::config_file();
            let mut config = AppConfig::load(&path).unwrap_or_default();
            config.style = match index {
                1 => FlyoutStyle::Battery,
                2 => FlyoutStyle::Thermometer,
                _ => FlyoutStyle::Bars,
            };
            if config.save(&path).is_err() {
                trace!("style: the config was not saved");
            }
            app.refresh(None);
        })
    });
    tray.on_open_folder(|| {
        let _ = std::process::Command::new("explorer.exe").arg(AppPaths::data_directory()).spawn();
    });
    tray.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
    Ok(tray)
}

impl App {
    fn clicked(&mut self) {
        if self.flyout.is_visible() {
            self.flyout.hide();
            return;
        }
        // The click that took the focus from the flyout has already closed it.
        if self.flyout.hidden_just_now() {
            return;
        }
        let now = sobreiro_core::now();
        self.claude.flyout_opened(now);
        self.copilot.flyout_opened(now);
        self.refresh(Some(native::cursor()));
    }

    fn refresh(&mut self, show_at: Option<(i32, i32)>) {
        let now = sobreiro_core::now();
        let claude = self.claude.limits(now);
        let copilot = self.copilot.limits(now);
        let credentials_exist = std::path::Path::new(&ClaudePaths::credentials_file(&env)).exists();
        // The spike never reads settings.json: with no numbers it counts as not connected.
        let show_claude =
            LimitList::show_claude(LimitList::claude_on_pc(credentials_exist, claude.len(), ConnectionState::NotConnected), !copilot.is_empty());
        let config = AppConfig::load(&AppPaths::config_file()).unwrap_or_default();
        let shown: Vec<Limit> = if show_claude { claude.iter().chain(&copilot).cloned().collect() } else { copilot.clone() };

        let light = WindowsTheme::is_light();
        let accent = WindowsTheme::accent();
        let colours = TaskbarColours::for_taskbar(light, WindowsTheme::accent_taskbar(), config.bar_colour, accent.as_ref());
        let size = native::small_icon_size();
        if let Some(credits) = copilot.first() {
            let drawn = (RingsModel::copilot(&copilot), colours.clone(), size);
            if self.copilot_drawn.as_ref() != Some(&drawn) {
                self.copilot_icon.set_rings(faces::tray_icon(&drawn.0, &colours, size));
                self.copilot_drawn = Some(drawn);
            }
            self.copilot_icon.set_tip(TooltipText::render_copilot(credits, now, Zone::Local).into());
        }
        let copilot_visible = !copilot.is_empty() && self.copilot_drawn.is_some();
        // With nothing to show for either service, Claude's icon stays, with empty rings: the toolkit makes
        // no tray icon without a picture.
        let claude_visible = show_claude || !copilot_visible;
        if claude_visible {
            let drawn = (RingsModel::from(&claude), colours.clone(), size);
            if self.claude_drawn.as_ref() != Some(&drawn) {
                self.claude_icon.set_rings(faces::tray_icon(&drawn.0, &colours, size));
                self.claude_drawn = Some(drawn);
            }
            self.claude_icon.set_tip(TooltipText::render(&claude, now, Zone::Local, false).into());
        }
        self.copilot_icon.set_shown(copilot_visible);
        self.claude_icon.set_shown(claude_visible);

        let enabled = config.show == ShowOption::TrayAndPill;
        pill::set_enabled(enabled);
        if enabled {
            let mut parts = Vec::new();
            if show_claude {
                parts.push(TooltipText::render(&shown, now, Zone::Local, false));
            }
            if let Some(credits) = shown.iter().find(|l| l.kind == LimitKind::Credits) {
                parts.push(TooltipText::render_copilot(credits, now, Zone::Local));
            }
            let model = PillModel::from_state(
                &shown,
                Zone::Local,
                show_claude,
                self.claude.state(),
                self.claude.misses(),
                self.copilot.state(),
                self.copilot.misses(),
                now,
            );
            pill::update(faces::pill(&model, &colours, &parts.join("\n")));
        }

        // The flyout's window is made the first time it is opened, not at start.
        if show_at.is_none() && !self.flyout.is_visible() {
            return;
        }
        let dark = !light;
        let fill = BarColour::pick(config.bar_colour, accent.as_ref(), dark).colour;
        let note = self.copilot.provider().shown().and_then(|reading| CopilotStateText::line(self.copilot.state(), &reading, now, Zone::Local));
        let model = FlyoutModel::from(&shown, now, Zone::Local, self.claude.state(), false, note.as_deref(), show_claude, config.pace_days);
        self.flyout.render(faces::flyout(&model, config.style, &NightPalette::for_dark(dark), fill));
        if let Some((x, y)) = show_at {
            self.flyout.show_near(x, y);
        }
    }
}
