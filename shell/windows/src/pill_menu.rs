//! The pill's right-click menu: what is in it, and what a choice does. The menu itself is Windows'
//! own (`win/menu.rs`); a choice that changes the settings loads `config.json` through Core,
//! changes the one key and saves it through Core, as 1.1's menu does. The text of the box that
//! says why a change was not saved is made here too.
//!
//! The first items connect Claude Code's status line to the bridge or disconnect it, by where it
//! stands now; the connect flow (`connect_flow`) does the work. Above them, while Claude Code is
//! shown, comes Sign in to Claude Code when Live usage stopped for want of a sign-in, and Sign out
//! of Claude Code otherwise (`claude_sign_in` runs Claude Code for both). Open data folder and About
//! Sobreiro follow Start with Windows; `about` does their work.
use crate::win::menu::{Entry, Item};
use sobreiro_core::colours::BarColourOption;
use sobreiro_core::config::{AppConfig, ConfigError, ShowOption};
use sobreiro_core::connect::connect_message::{self, RECONNECT_TITLE};
use sobreiro_core::connect::connection::ConnectionState;
use sobreiro_core::display::{FlyoutStyle, PaceDays};
use sobreiro_core::live::claude_sign_in::{self, Offer};
use std::path::Path;

const BARS: u32 = 1;
const BATTERY: u32 = 2;
const THERMOMETER: u32 = 3;
const PURPLE: u32 = 4;
const ACCENT: u32 = 5;
const EVERY_DAY: u32 = 6;
const WEEKDAYS: u32 = 7;
const TRAY_ONLY: u32 = 8;
const TRAY_AND_PILL: u32 = 9;
const AUTO_START: u32 = 10;
const QUIT: u32 = 11;
const CONNECT: u32 = 12;
const RECONNECT: u32 = 13;
const DISCONNECT: u32 = 14;
const OPEN_DATA_FOLDER: u32 = 15;
const ABOUT: u32 = 16;
const SIGN_IN: u32 = 17;
const SIGN_OUT: u32 = 18;

/// The one key of `config.json` a choice in the menu changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigChange {
    Style(FlyoutStyle),
    BarColour(BarColourOption),
    Pace(PaceDays),
    Show(ShowOption),
}

/// What a choice in the menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Change(ConfigChange),
    /// Runs the connect flow: true to connect (or reconnect), false to disconnect.
    Connect(bool),
    /// Runs Claude Code so it renews its sign-in, and shows it when it needs the user.
    SignIn,
    /// Asks, then signs Claude Code out, so another account can sign in.
    SignOut,
    ToggleAutoStart,
    OpenDataFolder,
    About,
    Quit,
}

/// Why `config.json` was not changed.
#[derive(Debug)]
pub enum ChangeError {
    /// It could not be loaded: not JSON, or not readable.
    Load(ConfigError),
    /// The changed config could not be saved.
    Save(ConfigError),
}

/// The menu, with the current choices checked and Start with Windows checked when it is on. The
/// first items follow the connection: Connect, Disconnect, or Reconnect above Disconnect; Sign in
/// to Claude Code or Sign out of Claude Code comes above them, as `account` offers.
pub fn entries(
    config: &AppConfig,
    auto_start: bool,
    state: ConnectionState,
    account: Option<Offer>,
) -> Vec<Entry> {
    let item = |id: u32, text: &str, checked: bool| Item {
        id,
        text: text.to_string(),
        checked,
    };
    let style = |id: u32, text: &str, option: FlyoutStyle| item(id, text, config.style == option);
    let colour =
        |id: u32, text: &str, option: BarColourOption| item(id, text, config.bar_colour == option);
    let pace = |id: u32, text: &str, option: PaceDays| item(id, text, config.pace_days == option);
    let show = |id: u32, text: &str, option: ShowOption| item(id, text, config.show == option);
    let connect = item(CONNECT, connect_message::title(true), false);
    let reconnect = item(RECONNECT, RECONNECT_TITLE, false);
    let disconnect = item(DISCONNECT, connect_message::title(false), false);
    let connection = match state {
        ConnectionState::NotConnected => vec![Entry::Item(connect)],
        ConnectionState::Connected => vec![Entry::Item(disconnect)],
        ConnectionState::Elsewhere => vec![Entry::Item(reconnect), Entry::Item(disconnect)],
    };
    let mut menu: Vec<Entry> = account
        .map(|offer| match offer {
            Offer::SignIn => Entry::Item(item(SIGN_IN, claude_sign_in::SIGN_IN_TITLE, false)),
            Offer::SignOut => Entry::Item(item(SIGN_OUT, claude_sign_in::SIGN_OUT_TITLE, false)),
        })
        .into_iter()
        .collect();
    menu.extend(connection);
    menu.extend([
        Entry::Submenu {
            text: "Style".to_string(),
            items: vec![
                style(BARS, "Bars", FlyoutStyle::Bars),
                style(BATTERY, "Battery", FlyoutStyle::Battery),
                style(THERMOMETER, "Thermometer", FlyoutStyle::Thermometer),
            ],
        },
        Entry::Submenu {
            text: "Bar colour".to_string(),
            items: vec![
                colour(PURPLE, "Sobreiro purple", BarColourOption::Purple),
                colour(ACCENT, "Windows accent", BarColourOption::Accent),
            ],
        },
        Entry::Submenu {
            text: "Pace".to_string(),
            items: vec![
                pace(EVERY_DAY, "Every day", PaceDays::EveryDay),
                pace(WEEKDAYS, "Weekdays only", PaceDays::Weekdays),
            ],
        },
        Entry::Submenu {
            text: "Show".to_string(),
            items: vec![
                show(TRAY_ONLY, "Tray icon only", ShowOption::TrayOnly),
                show(
                    TRAY_AND_PILL,
                    "Tray icon and taskbar pill",
                    ShowOption::TrayAndPill,
                ),
            ],
        },
        Entry::Separator,
        Entry::Item(item(AUTO_START, "Start with Windows", auto_start)),
        Entry::Item(item(OPEN_DATA_FOLDER, "Open data folder", false)),
        Entry::Item(item(ABOUT, "About Sobreiro", false)),
        Entry::Separator,
        Entry::Item(item(QUIT, "Quit", false)),
    ]);
    menu
}

/// The command an item's id stands for.
pub fn command(id: u32) -> Option<Command> {
    let change = |change: ConfigChange| Some(Command::Change(change));
    match id {
        BARS => change(ConfigChange::Style(FlyoutStyle::Bars)),
        BATTERY => change(ConfigChange::Style(FlyoutStyle::Battery)),
        THERMOMETER => change(ConfigChange::Style(FlyoutStyle::Thermometer)),
        PURPLE => change(ConfigChange::BarColour(BarColourOption::Purple)),
        ACCENT => change(ConfigChange::BarColour(BarColourOption::Accent)),
        EVERY_DAY => change(ConfigChange::Pace(PaceDays::EveryDay)),
        WEEKDAYS => change(ConfigChange::Pace(PaceDays::Weekdays)),
        TRAY_ONLY => change(ConfigChange::Show(ShowOption::TrayOnly)),
        TRAY_AND_PILL => change(ConfigChange::Show(ShowOption::TrayAndPill)),
        CONNECT | RECONNECT => Some(Command::Connect(true)),
        DISCONNECT => Some(Command::Connect(false)),
        SIGN_IN => Some(Command::SignIn),
        SIGN_OUT => Some(Command::SignOut),
        AUTO_START => Some(Command::ToggleAutoStart),
        OPEN_DATA_FOLDER => Some(Command::OpenDataFolder),
        ABOUT => Some(Command::About),
        QUIT => Some(Command::Quit),
        _ => None,
    }
}

/// The config with the change made, or none when it already holds that value: nothing is saved
/// then, as 1.1 saves only a config that changed.
pub fn with_change(config: &AppConfig, change: ConfigChange) -> Option<AppConfig> {
    let mut next = config.clone();
    match change {
        ConfigChange::Style(style) => next.style = style,
        ConfigChange::BarColour(colour) => next.bar_colour = colour,
        ConfigChange::Pace(pace) => next.pace_days = pace,
        ConfigChange::Show(show) => next.show = show,
    }
    (next != *config).then_some(next)
}

/// Loads `config.json`, makes the change and saves it. True when the file now holds a change.
/// A file that cannot be read or is not JSON is not saved over.
pub fn change_config(path: &Path, change: ConfigChange) -> Result<bool, ChangeError> {
    let config = AppConfig::load(path).map_err(ChangeError::Load)?;
    match with_change(&config, change) {
        Some(next) => {
            next.save(path).map_err(ChangeError::Save)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// What the error box says when a change was not saved, in 1.1's words.
pub fn error_text(error: &ChangeError) -> String {
    match error {
        ChangeError::Load(error @ ConfigError::NotJson) => error.to_string(),
        ChangeError::Load(error) => {
            format!("Couldn't read config.json, so nothing was saved.\n\n{error}")
        }
        ChangeError::Save(error) => format!("Couldn't save config.json.\n\n{error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        // One folder for each process, so two runs of the tests never share one.
        let folder =
            std::env::temp_dir().join(format!("sobreiro-pill-menu-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let _ = std::fs::create_dir_all(&folder);
        folder
    }

    fn submenu(entries: &[Entry], at: usize) -> (&str, Vec<(&str, bool)>) {
        match &entries[at] {
            Entry::Submenu { text, items } => (
                text.as_str(),
                items
                    .iter()
                    .map(|item| (item.text.as_str(), item.checked))
                    .collect(),
            ),
            other => panic!("not a submenu: {other:?}"),
        }
    }

    fn item(entries: &[Entry], at: usize) -> (&str, bool) {
        match &entries[at] {
            Entry::Item(item) => (item.text.as_str(), item.checked),
            other => panic!("not an item: {other:?}"),
        }
    }

    const STATES: [ConnectionState; 3] = [
        ConnectionState::NotConnected,
        ConnectionState::Connected,
        ConnectionState::Elsewhere,
    ];

    /// The menu of a connected status line, whose one first item shifts the rest by one.
    fn connected(config: &AppConfig, auto_start: bool) -> Vec<Entry> {
        entries(config, auto_start, ConnectionState::Connected, None)
    }

    /// The first entries' texts up to Style.
    fn first_texts(state: ConnectionState) -> Vec<String> {
        let menu = entries(&AppConfig::default(), false, state, None);
        let at = menu
            .iter()
            .position(|entry| matches!(entry, Entry::Submenu { .. }))
            .unwrap_or(0);
        menu[..at]
            .iter()
            .map(|entry| match entry {
                Entry::Item(item) => item.text.clone(),
                other => panic!("not an item: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn the_connection_items_follow_the_state_and_no_line_comes_after_them() {
        assert_eq!(
            first_texts(ConnectionState::NotConnected),
            ["Connect to Claude Code"]
        );
        assert_eq!(
            first_texts(ConnectionState::Connected),
            ["Disconnect from Claude Code"]
        );
        assert_eq!(
            first_texts(ConnectionState::Elsewhere),
            ["Reconnect to Claude Code", "Disconnect from Claude Code"]
        );
        for state in STATES {
            let menu = entries(&AppConfig::default(), false, state, None);
            let first = first_texts(state).len();
            assert_eq!(submenu(&menu, first).0, "Style");
        }
    }

    #[test]
    fn the_account_item_comes_first_as_offered() {
        for state in STATES {
            let without = entries(&AppConfig::default(), false, state, None);
            for (offer, text) in [
                (Offer::SignIn, "Sign in to Claude Code"),
                (Offer::SignOut, "Sign out of Claude Code"),
            ] {
                let with = entries(&AppConfig::default(), false, state, Some(offer));
                assert_eq!(with.len(), without.len() + 1, "{state:?}");
                assert_eq!(item(&with, 0).0, text, "{state:?}");
                assert_eq!(with[1..], without[..], "{state:?}");
            }
            assert!(
                without.iter().all(|entry| !matches!(
                    entry,
                    Entry::Item(item) if item.id == SIGN_IN || item.id == SIGN_OUT
                )),
                "{state:?}"
            );
        }
    }

    #[test]
    fn the_full_order_holds_in_each_state() {
        for state in STATES {
            let menu = entries(&AppConfig::default(), false, state, None);
            let names: Vec<&str> = menu
                .iter()
                .map(|entry| match entry {
                    Entry::Item(item) => item.text.as_str(),
                    Entry::Submenu { text, .. } => text.as_str(),
                    Entry::Separator => "-",
                })
                .collect();
            let mut expected = match state {
                ConnectionState::NotConnected => vec!["Connect to Claude Code"],
                ConnectionState::Connected => vec!["Disconnect from Claude Code"],
                ConnectionState::Elsewhere => {
                    vec!["Reconnect to Claude Code", "Disconnect from Claude Code"]
                }
            };
            expected.extend([
                "Style",
                "Bar colour",
                "Pace",
                "Show",
                "-",
                "Start with Windows",
                "Open data folder",
                "About Sobreiro",
                "-",
                "Quit",
            ]);
            assert_eq!(names, expected);
        }
    }

    #[test]
    fn the_menu_is_in_1_1s_order_with_its_texts() {
        let menu = connected(&AppConfig::default(), false);
        assert_eq!(menu.len(), 11);
        assert_eq!(menu[5], Entry::Separator);
        assert_eq!(menu[9], Entry::Separator);
        let titles: Vec<_> = (1..5).map(|at| submenu(&menu, at).0).collect();
        assert_eq!(titles, ["Style", "Bar colour", "Pace", "Show"]);
        let names = |at: usize| {
            submenu(&menu, at + 1)
                .1
                .into_iter()
                .map(|(text, _)| text)
                .collect::<Vec<_>>()
        };
        assert_eq!(names(0), ["Bars", "Battery", "Thermometer"]);
        assert_eq!(names(1), ["Sobreiro purple", "Windows accent"]);
        assert_eq!(names(2), ["Every day", "Weekdays only"]);
        assert_eq!(names(3), ["Tray icon only", "Tray icon and taskbar pill"]);
        assert_eq!(item(&menu, 0).0, "Disconnect from Claude Code");
        assert_eq!(item(&menu, 6).0, "Start with Windows");
        assert_eq!(item(&menu, 7).0, "Open data folder");
        assert_eq!(item(&menu, 8).0, "About Sobreiro");
        assert_eq!(item(&menu, 10).0, "Quit");
    }

    #[test]
    fn each_submenu_checks_the_current_value() {
        let checks = |config: &AppConfig, at: usize| -> Vec<bool> {
            submenu(&connected(config, false), at + 1)
                .1
                .into_iter()
                .map(|(_, checked)| checked)
                .collect()
        };
        let config = AppConfig {
            style: FlyoutStyle::Thermometer,
            bar_colour: BarColourOption::Accent,
            pace_days: PaceDays::Weekdays,
            show: ShowOption::TrayOnly,
            ..AppConfig::default()
        };
        assert_eq!(checks(&config, 0), [false, false, true]);
        assert_eq!(checks(&config, 1), [false, true]);
        assert_eq!(checks(&config, 2), [false, true]);
        assert_eq!(checks(&config, 3), [true, false]);
        let config = AppConfig {
            style: FlyoutStyle::Bars,
            bar_colour: BarColourOption::Purple,
            pace_days: PaceDays::EveryDay,
            show: ShowOption::TrayAndPill,
            ..AppConfig::default()
        };
        assert_eq!(checks(&config, 0), [true, false, false]);
        assert_eq!(checks(&config, 1), [true, false]);
        assert_eq!(checks(&config, 2), [true, false]);
        assert_eq!(checks(&config, 3), [false, true]);
    }

    #[test]
    fn every_id_in_the_menu_is_a_command_and_none_is_zero() {
        // The Elsewhere menu with Sign in, and Sign out from the other, hold every id there is.
        let mut ids = Vec::new();
        let [with_sign_in, with_sign_out] = [Offer::SignIn, Offer::SignOut].map(|offer| {
            entries(
                &AppConfig::default(),
                false,
                ConnectionState::Elsewhere,
                Some(offer),
            )
        });
        for entry in with_sign_in
            .into_iter()
            .chain(with_sign_out.into_iter().take(1))
        {
            match entry {
                Entry::Separator => {}
                Entry::Item(item) => ids.push(item.id),
                Entry::Submenu { items, .. } => ids.extend(items.iter().map(|item| item.id)),
            }
        }
        assert_eq!(ids.len(), 17);
        assert!(ids.iter().all(|id| *id != 0 && command(*id).is_some()));
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len());
        assert_eq!(command(CONNECT), Some(Command::Connect(true)));
        assert_eq!(command(RECONNECT), Some(Command::Connect(true)));
        assert_eq!(command(DISCONNECT), Some(Command::Connect(false)));
        assert_eq!(command(SIGN_IN), Some(Command::SignIn));
        assert_eq!(command(SIGN_OUT), Some(Command::SignOut));
        assert_eq!(
            command(BATTERY),
            Some(Command::Change(ConfigChange::Style(FlyoutStyle::Battery)))
        );
        assert_eq!(
            command(ACCENT),
            Some(Command::Change(ConfigChange::BarColour(
                BarColourOption::Accent
            )))
        );
        assert_eq!(
            command(WEEKDAYS),
            Some(Command::Change(ConfigChange::Pace(PaceDays::Weekdays)))
        );
        assert_eq!(
            command(TRAY_ONLY),
            Some(Command::Change(ConfigChange::Show(ShowOption::TrayOnly)))
        );
        assert_eq!(command(AUTO_START), Some(Command::ToggleAutoStart));
        assert_eq!(command(OPEN_DATA_FOLDER), Some(Command::OpenDataFolder));
        assert_eq!(command(ABOUT), Some(Command::About));
        assert_eq!(command(QUIT), Some(Command::Quit));
        assert_eq!(command(0), None);
        assert_eq!(command(99), None);
    }

    #[test]
    fn the_start_with_windows_check_follows_the_flag() {
        let checked = |on: bool| item(&connected(&AppConfig::default(), on), 6).1;
        assert!(checked(true));
        assert!(!checked(false));
    }

    #[test]
    fn a_choice_changes_the_one_key_and_keeps_the_rest() {
        let config = AppConfig {
            chained_command: Some("echo".to_string()),
            extra: vec![("note".to_string(), "1".to_string())],
            style: FlyoutStyle::Bars,
            bar_colour: BarColourOption::Purple,
            pace_days: PaceDays::EveryDay,
            show: ShowOption::TrayAndPill,
            ..AppConfig::default()
        };
        let changes = [
            (
                ConfigChange::Style(FlyoutStyle::Battery),
                AppConfig {
                    style: FlyoutStyle::Battery,
                    ..config.clone()
                },
            ),
            (
                ConfigChange::BarColour(BarColourOption::Accent),
                AppConfig {
                    bar_colour: BarColourOption::Accent,
                    ..config.clone()
                },
            ),
            (
                ConfigChange::Pace(PaceDays::Weekdays),
                AppConfig {
                    pace_days: PaceDays::Weekdays,
                    ..config.clone()
                },
            ),
            (
                ConfigChange::Show(ShowOption::TrayOnly),
                AppConfig {
                    show: ShowOption::TrayOnly,
                    ..config.clone()
                },
            ),
        ];
        for (change, expected) in changes {
            assert_eq!(with_change(&config, change), Some(expected));
        }
        // Already that value: nothing to save.
        for same in [
            ConfigChange::Style(FlyoutStyle::Bars),
            ConfigChange::BarColour(BarColourOption::Purple),
            ConfigChange::Pace(PaceDays::EveryDay),
            ConfigChange::Show(ShowOption::TrayAndPill),
        ] {
            assert_eq!(with_change(&config, same), None);
        }
    }

    #[test]
    fn each_choice_is_saved_through_core_with_everything_else_kept() {
        let text = "{\r\n  \"style\": \"bars\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\",\r\n  \"other\": [1, 2]\r\n}";
        let cases = [
            (
                ConfigChange::Style(FlyoutStyle::Battery),
                "\"style\": \"bars\"",
                "\"style\": \"battery\"",
            ),
            (
                ConfigChange::BarColour(BarColourOption::Accent),
                "\"bar_colour\": \"purple\"",
                "\"bar_colour\": \"accent\"",
            ),
            (
                ConfigChange::Pace(PaceDays::Weekdays),
                "\"pace\": \"everyday\"",
                "\"pace\": \"weekdays\"",
            ),
            (
                ConfigChange::Show(ShowOption::TrayOnly),
                "\"show\": \"trayandpill\"",
                "\"show\": \"trayonly\"",
            ),
        ];
        for (at, (change, from, to)) in cases.into_iter().enumerate() {
            let path = folder(&format!("save{at}")).join("config.json");
            assert!(std::fs::write(&path, text).is_ok());
            assert!(matches!(change_config(&path, change), Ok(true)));
            let saved = std::fs::read_to_string(&path).unwrap_or_default();
            assert_eq!(saved, text.replace(from, to));
            // Chosen again: the file is left as it is.
            assert!(matches!(change_config(&path, change), Ok(false)));
            assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), saved);
        }
    }

    #[test]
    fn a_missing_config_is_made_and_one_that_is_not_json_is_left_alone() {
        let folder = folder("missing");
        let path = folder.join("config.json");
        assert!(matches!(
            change_config(&path, ConfigChange::BarColour(BarColourOption::Accent)),
            Ok(true)
        ));
        let made = AppConfig::load(&path);
        assert_eq!(
            made.map(|config| config.bar_colour).ok(),
            Some(BarColourOption::Accent)
        );

        let broken = folder.join("broken.json");
        assert!(std::fs::write(&broken, "{ not json").is_ok());
        let error = change_config(&broken, ConfigChange::Pace(PaceDays::Weekdays)).err();
        assert_eq!(
            std::fs::read_to_string(&broken).unwrap_or_default(),
            "{ not json"
        );
        let Some(error) = error else {
            panic!("a file that is not JSON was changed");
        };
        assert!(matches!(error, ChangeError::Load(ConfigError::NotJson)));
        assert_eq!(
            error_text(&error),
            "config.json isn't valid JSON, so Sobreiro left it alone."
        );
    }

    #[test]
    fn the_read_and_save_failures_have_1_1s_words() {
        let io = || ConfigError::Io(std::io::Error::other("in use"));
        assert_eq!(
            error_text(&ChangeError::Load(io())),
            "Couldn't read config.json, so nothing was saved.\n\nin use"
        );
        assert_eq!(
            error_text(&ChangeError::Save(io())),
            "Couldn't save config.json.\n\nin use"
        );
    }
}
