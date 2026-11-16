//! The pill's right-click menu: what is in it, and what a choice does. The menu itself is Windows'
//! own (`win/menu.rs`); a choice that changes the settings loads `config.json` through Core,
//! changes the one key and saves it through Core, as 1.1's menu does.
//!
//! The rest of 1.1's menu comes with the tray icons.
use crate::win::menu::{Entry, Item};
use sobreiro_core::colours::BarColourOption;
use sobreiro_core::config::{AppConfig, ConfigError};
use std::path::Path;

const PURPLE: u32 = 1;
const ACCENT: u32 = 2;
const QUIT: u32 = 3;

/// What a choice in the menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    SetBarColour(BarColourOption),
    Quit,
}

/// The menu, with the bar colour in use checked.
pub fn entries(current: BarColourOption) -> Vec<Entry> {
    let colour = |id: u32, text: &str, option: BarColourOption| Item {
        id,
        text: text.to_string(),
        checked: current == option,
    };
    vec![
        Entry::Submenu {
            text: "Bar colour".to_string(),
            items: vec![
                colour(PURPLE, "Sobreiro purple", BarColourOption::Purple),
                colour(ACCENT, "Windows accent", BarColourOption::Accent),
            ],
        },
        Entry::Item(Item {
            id: QUIT,
            text: "Quit".to_string(),
            checked: false,
        }),
    ]
}

/// The command an item's id stands for.
pub fn command(id: u32) -> Option<Command> {
    match id {
        PURPLE => Some(Command::SetBarColour(BarColourOption::Purple)),
        ACCENT => Some(Command::SetBarColour(BarColourOption::Accent)),
        QUIT => Some(Command::Quit),
        _ => None,
    }
}

/// The config with the bar colour set, or none when it already is that: nothing is saved then, as
/// 1.1 saves only a config that changed.
pub fn with_bar_colour(config: &AppConfig, colour: BarColourOption) -> Option<AppConfig> {
    (config.bar_colour != colour).then(|| AppConfig {
        bar_colour: colour,
        ..config.clone()
    })
}

/// Loads `config.json`, sets the bar colour and saves it. True when the file now holds a change.
/// A file that cannot be read or is not JSON is not saved over.
pub fn change_bar_colour(path: &Path, colour: BarColourOption) -> Result<bool, ConfigError> {
    let config = AppConfig::load(path)?;
    match with_bar_colour(&config, colour) {
        Some(next) => {
            next.save(path)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("sobreiro-pill-menu-{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        let _ = std::fs::create_dir_all(&folder);
        folder
    }

    #[test]
    fn the_menu_has_bar_colour_with_two_choices_and_quit() {
        let accent = entries(BarColourOption::Accent);
        assert_eq!(accent.len(), 2);
        match &accent[0] {
            Entry::Submenu { text, items } => {
                assert_eq!(text, "Bar colour");
                let names: Vec<_> = items.iter().map(|item| item.text.as_str()).collect();
                assert_eq!(names, ["Sobreiro purple", "Windows accent"]);
                // The one in use is checked.
                let checks: Vec<_> = items.iter().map(|item| item.checked).collect();
                assert_eq!(checks, [false, true]);
            }
            other => panic!("not the submenu: {other:?}"),
        }
        match &accent[1] {
            Entry::Item(item) => assert_eq!(item.text, "Quit"),
            other => panic!("not Quit: {other:?}"),
        }
        let purple = entries(BarColourOption::Purple);
        match &purple[0] {
            Entry::Submenu { items, .. } => {
                let checks: Vec<_> = items.iter().map(|item| item.checked).collect();
                assert_eq!(checks, [true, false]);
            }
            other => panic!("not the submenu: {other:?}"),
        }
    }

    #[test]
    fn every_id_in_the_menu_is_a_command_and_none_is_zero() {
        let mut ids = Vec::new();
        for entry in entries(BarColourOption::Purple) {
            match entry {
                Entry::Item(item) => ids.push(item.id),
                Entry::Submenu { items, .. } => ids.extend(items.iter().map(|item| item.id)),
            }
        }
        assert_eq!(ids.len(), 3);
        assert!(ids.iter().all(|id| *id != 0 && command(*id).is_some()));
        assert_eq!(
            command(PURPLE),
            Some(Command::SetBarColour(BarColourOption::Purple))
        );
        assert_eq!(
            command(ACCENT),
            Some(Command::SetBarColour(BarColourOption::Accent))
        );
        assert_eq!(command(QUIT), Some(Command::Quit));
        assert_eq!(command(0), None);
        assert_eq!(command(99), None);
    }

    #[test]
    fn a_choice_changes_the_one_key_and_keeps_the_rest() {
        let config = AppConfig {
            chained_command: Some("echo".to_string()),
            extra: vec![("note".to_string(), "1".to_string())],
            ..AppConfig::default()
        };
        let next = with_bar_colour(&config, BarColourOption::Accent);
        assert_eq!(
            next,
            Some(AppConfig {
                bar_colour: BarColourOption::Accent,
                ..config.clone()
            })
        );
        // Already that colour: nothing to save.
        assert_eq!(with_bar_colour(&config, BarColourOption::Purple), None);
    }

    #[test]
    fn the_choice_is_saved_through_core_with_everything_else_kept() {
        let path = folder("save").join("config.json");
        let text = "{\r\n  \"style\": \"battery\",\r\n  \"bar_colour\": \"purple\",\r\n  \"show\": \"trayandpill\",\r\n  \"pace\": \"everyday\",\r\n  \"other\": [1, 2]\r\n}";
        assert!(std::fs::write(&path, text).is_ok());
        assert!(matches!(
            change_bar_colour(&path, BarColourOption::Accent),
            Ok(true)
        ));
        let saved = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(saved, text.replace("\"purple\"", "\"accent\""));
        // Chosen again: the file is left as it is.
        assert!(matches!(
            change_bar_colour(&path, BarColourOption::Accent),
            Ok(false)
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), saved);
    }

    #[test]
    fn a_missing_config_is_made_and_one_that_is_not_json_is_left_alone() {
        let folder = folder("missing");
        let path = folder.join("config.json");
        assert!(matches!(
            change_bar_colour(&path, BarColourOption::Accent),
            Ok(true)
        ));
        let made = AppConfig::load(&path);
        assert_eq!(
            made.map(|config| config.bar_colour).ok(),
            Some(BarColourOption::Accent)
        );

        let broken = folder.join("broken.json");
        assert!(std::fs::write(&broken, "{ not json").is_ok());
        assert!(change_bar_colour(&broken, BarColourOption::Accent).is_err());
        assert_eq!(
            std::fs::read_to_string(&broken).unwrap_or_default(),
            "{ not json"
        );
    }
}
