// Ports of AccentPaletteTests, BarColourTests, ContrastTests, TaskbarColoursTests, AppConfigTests
// (the two BridgeConfig assertions are left out: Bridge is not ported).
mod common;
use common::*;
use sobreiro_core::colours::*;
use sobreiro_core::config::*;
use sobreiro_core::display::{FlyoutStyle, PaceDays};
use sobreiro_core::limits::UsageLevel;
use std::os::windows::fs::OpenOptionsExt;

fn c(hex: &str) -> Rgb {
    Rgb::parse(hex).unwrap()
}

// ---- AccentPaletteTests

const BLUE: [u8; 32] = [
    0x99, 0xEB, 0xFF, 0x00, 0x4C, 0xC2, 0xFF, 0x00, 0x00, 0x91, 0xF8, 0x00, 0x00, 0x78, 0xD4, 0x00,
    0x00, 0x67, 0xC0, 0x00, 0x00, 0x3E, 0x92, 0x00, 0x00, 0x1A, 0x68, 0x00, 0xF7, 0x63, 0x0C, 0x00,
];

#[test]
fn parses_seven_quads_in_order() {
    let p = AccentPalette::from_registry(Some(&BLUE)).unwrap();
    let hexes = [p.light3, p.light2, p.light1, p.accent, p.dark1, p.dark2, p.dark3].map(|x| x.to_hex());
    assert_eq!(hexes, ["#99EBFF", "#4CC2FF", "#0091F8", "#0078D4", "#0067C0", "#003E92", "#001A68"]);
}

#[test]
fn windows_blue_is_the_default_blue_palette() {
    assert_eq!(AccentPalette::from_registry(Some(&BLUE)), Some(AccentPalette::windows_blue()));
}

#[test]
fn null_or_short_bytes_give_null() {
    assert!(AccentPalette::from_registry(None).is_none());
    assert!(AccentPalette::from_registry(Some(&[0u8; 27])).is_none());
}

#[test]
fn exactly_twenty_eight_bytes_is_enough() {
    assert!(AccentPalette::from_registry(Some(&BLUE[..28])).is_some());
}

// ---- BarColourTests

fn palette(a: [&str; 7]) -> AccentPalette {
    let [l3, l2, l1, ac, d1, d2, d3] = a.map(c);
    AccentPalette { light3: l3, light2: l2, light1: l1, accent: ac, dark1: d1, dark2: d2, dark3: d3 }
}
fn blue() -> AccentPalette {
    palette(["#99EBFF", "#4CC2FF", "#0091F8", "#0078D4", "#0067C0", "#003E92", "#001A68"])
}
fn storm() -> AccentPalette {
    palette(["#B5B3B1", "#8F8D8B", "#6B6967", "#4C4A48", "#3A3836", "#2A2826", "#1A1816"])
}
fn yellow() -> AccentPalette {
    palette(["#FFFF00"; 7])
}

#[test]
fn purple_option_ignores_the_accent() {
    for (dark, hex) in [(true, "#A594FF"), (false, "#5A3DD1")] {
        let pick = BarColour::pick(BarColourOption::Purple, Some(&blue()), dark);
        assert_eq!(pick.colour.to_hex(), hex);
        assert_eq!(pick.source, BarColourSource::Purple);
    }
}

#[test]
fn blue_accent_is_used_in_both_themes() {
    for dark in [true, false] {
        let pick = BarColour::pick(BarColourOption::Accent, Some(&blue()), dark);
        assert_eq!(pick.colour.to_hex(), "#0078D4");
        assert_eq!(pick.source, BarColourSource::Accent);
    }
}

#[test]
fn storm_on_dark_falls_to_light2() {
    let pick = BarColour::pick(BarColourOption::Accent, Some(&storm()), true);
    assert_eq!(pick.colour.to_hex(), "#8F8D8B");
    assert_eq!(pick.source, BarColourSource::AccentShade);
}

#[test]
fn storm_on_light_keeps_the_accent() {
    let pick = BarColour::pick(BarColourOption::Accent, Some(&storm()), false);
    assert_eq!(pick.colour.to_hex(), "#4C4A48");
    assert_eq!(pick.source, BarColourSource::Accent);
}

#[test]
fn yellow_on_light_falls_back_to_purple() {
    let pick = BarColour::pick(BarColourOption::Accent, Some(&yellow()), false);
    assert_eq!(pick.colour.to_hex(), "#5A3DD1");
    assert_eq!(pick.source, BarColourSource::PurpleFallback);
}

#[test]
fn yellow_on_dark_is_fine() {
    let pick = BarColour::pick(BarColourOption::Accent, Some(&yellow()), true);
    assert_eq!(pick.colour.to_hex(), "#FFFF00");
    assert_eq!(pick.source, BarColourSource::Accent);
}

#[test]
fn accent_option_without_palette_falls_back_to_purple() {
    for (dark, hex) in [(true, "#A594FF"), (false, "#5A3DD1")] {
        let pick = BarColour::pick(BarColourOption::Accent, None, dark);
        assert_eq!(pick.colour.to_hex(), hex);
        assert_eq!(pick.source, BarColourSource::PurpleFallback);
    }
}

#[test]
fn level_colour_maps_warning_and_error() {
    let n = NightPalette::dark();
    let normal = c("#123456");
    assert_eq!(n.level_colour(UsageLevel::Warning, normal), n.warning);
    assert_eq!(n.level_colour(UsageLevel::Error, normal), n.error);
    assert_eq!(n.level_colour(UsageLevel::Normal, normal), normal);
}

// ---- ContrastTests

#[test]
fn black_on_white_is_twenty_one() {
    assert!((Contrast::ratio(c("#000000"), c("#FFFFFF")) - 21.0).abs() < 0.005);
}

#[test]
fn grey_767676_on_white_is_454() {
    assert!((Contrast::ratio(c("#767676"), c("#FFFFFF")) - 4.54).abs() < 0.005);
}

#[test]
fn ratio_is_symmetric() {
    let (a, b) = (c("#7B62FF"), c("#0B0B0B"));
    assert_eq!(Contrast::ratio(a, b), Contrast::ratio(b, a));
}

#[test]
fn parse_and_to_hex_round_trip_in_upper_case() {
    assert_eq!(c("#7b62ff").to_hex(), "#7B62FF");
    assert!(Rgb::parse("#12345").is_err());
}

// ---- TaskbarColoursTests

#[test]
fn dark_taskbar_purple() {
    assert_eq!(
        TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None),
        TaskbarColours { ink: c("#FFFFFF"), base: c("#A594FF"), warning: c("#ffb020"), error: c("#ff6b6b") }
    );
}

#[test]
fn light_taskbar_purple() {
    assert_eq!(
        TaskbarColours::for_taskbar(true, false, BarColourOption::Purple, None),
        TaskbarColours { ink: c("#1A1A1A"), base: c("#5a3dd1"), warning: c("#a35f00"), error: c("#c4262e") }
    );
}

#[test]
fn accent_option_uses_the_bar_colour_pick() {
    assert_eq!(
        BarColour::pick(BarColourOption::Accent, Some(&AccentPalette::windows_blue()), true).colour,
        TaskbarColours::for_taskbar(false, false, BarColourOption::Accent, Some(&AccentPalette::windows_blue())).base
    );
}

#[test]
fn accent_coloured_taskbar_uses_ink_as_base() {
    for light in [false, true] {
        let t = TaskbarColours::for_taskbar(light, true, BarColourOption::Accent, Some(&AccentPalette::windows_blue()));
        assert_eq!(t.ink, t.base);
    }
}

#[test]
fn fill_follows_the_level() {
    let t = TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None);
    assert_eq!(t.fill(UsageLevel::Normal), t.base);
    assert_eq!(t.fill(UsageLevel::Warning), t.warning);
    assert_eq!(t.fill(UsageLevel::Error), t.error);
}

fn over(ink: u8, ground: u8, alpha: u8) -> u8 {
    ((ink as f64 * alpha as f64 + ground as f64 * (255 - alpha) as f64) / 255.0).round() as u8
}
fn white_over_dark(alpha: u8) -> Rgb {
    let v = over(255, 0x1C, alpha);
    Rgb::new(v, v, v)
}
fn ink_over_light(alpha: u8) -> Rgb {
    let v = over(0x1A, 0xF0, alpha);
    Rgb::new(v, v, v)
}

#[test]
fn dark_purple_reads_on_the_dark_track() {
    let track = white_over_dark(TaskbarColours::TRACK_ALPHA);
    let t = TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None);
    assert!(Contrast::ratio(t.base, track) >= 3.0);
}

#[test]
fn icon_track_shows_on_the_dark_taskbar() {
    assert!(Contrast::ratio(white_over_dark(TaskbarColours::ICON_TRACK_ALPHA), Rgb::new(0x1C, 0x1C, 0x1C)) >= 2.0);
}

#[test]
fn dark_purple_reads_on_the_icon_track() {
    let t = TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None);
    assert!(Contrast::ratio(t.base, white_over_dark(TaskbarColours::ICON_TRACK_ALPHA)) >= 2.0);
}

#[test]
fn the_stale_fill_is_the_ink_at_half_strength() {
    assert_eq!(TaskbarColours::STALE_ALPHA, 0x80);
}

#[test]
fn the_stale_fill_reads_on_the_dark_track() {
    assert!(Contrast::ratio(white_over_dark(TaskbarColours::STALE_ALPHA), white_over_dark(TaskbarColours::TRACK_ALPHA)) >= 3.0);
}

#[test]
fn the_stale_fill_reads_on_the_light_track() {
    assert!(Contrast::ratio(ink_over_light(TaskbarColours::STALE_ALPHA), ink_over_light(TaskbarColours::TRACK_ALPHA)) >= 2.0);
}

// ---- AppConfigTests

fn write(d: &Scratch, text: &str) -> String {
    let p = d.path("config.json");
    std::fs::write(&p, text).unwrap();
    p
}

fn assert_default(c: &AppConfig) {
    assert_eq!(c, &AppConfig::default());
    assert!(c.chained_command.is_none() && c.previous.is_none() && c.extra.is_empty());
    assert_eq!((c.style, c.bar_colour, c.show, c.pace_days), (FlyoutStyle::Bars, BarColourOption::Purple, ShowOption::TrayAndPill, PaceDays::EveryDay));
}

fn extra<'a>(c: &'a AppConfig, key: &str) -> Option<&'a str> {
    c.extra.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

#[test]
fn pace_defaults_to_every_day() {
    let d = Scratch::new();
    assert_eq!(AppConfig::load(&d.path("config.json")).unwrap().pace_days, PaceDays::EveryDay);
}

#[test]
fn pace_loads_and_unknown_values_fall_back() {
    for (json, expected) in [
        ("\"weekdays\"", PaceDays::Weekdays), ("\"Weekdays\"", PaceDays::Weekdays), ("\"everyday\"", PaceDays::EveryDay),
        ("\"workdays\"", PaceDays::EveryDay), ("5", PaceDays::EveryDay), ("null", PaceDays::EveryDay),
    ] {
        let d = Scratch::new();
        let loaded = AppConfig::load(&write(&d, &format!("{{\"pace\":{json}}}"))).unwrap();
        assert_eq!(loaded.pace_days, expected, "{json}");
        assert!(extra(&loaded, "pace").is_none());
    }
}

#[test]
fn pace_round_trips_and_is_written_lower_case() {
    let d = Scratch::new();
    let p = d.path("config.json");
    AppConfig { pace_days: PaceDays::Weekdays, ..AppConfig::default() }.save(&p).unwrap();
    assert!(std::fs::read_to_string(&p).unwrap().contains("\"pace\": \"weekdays\""));
    assert_eq!(AppConfig::load(&p).unwrap().pace_days, PaceDays::Weekdays);
}

#[test]
fn show_defaults_to_tray_and_pill() {
    let d = Scratch::new();
    assert_eq!(AppConfig::load(&d.path("config.json")).unwrap().show, ShowOption::TrayAndPill);
}

#[test]
fn show_loads_and_unknown_values_fall_back() {
    let d = Scratch::new();
    let loaded = AppConfig::load(&write(&d, "{\"show\":\"TrayOnly\"}")).unwrap();
    assert_eq!(loaded.show, ShowOption::TrayOnly);
    assert!(extra(&loaded, "show").is_none());
    assert_eq!(AppConfig::load(&write(&d, "{\"show\":\"everywhere\"}")).unwrap().show, ShowOption::TrayAndPill);
}

#[test]
fn show_round_trips_and_is_written_lower_case() {
    let d = Scratch::new();
    let p = d.path("config.json");
    AppConfig { show: ShowOption::TrayOnly, ..AppConfig::default() }.save(&p).unwrap();
    assert!(std::fs::read_to_string(&p).unwrap().contains("\"show\": \"trayonly\""));
    assert_eq!(AppConfig::load(&p).unwrap().show, ShowOption::TrayOnly);
}

#[test]
fn missing_file_gives_default() {
    let d = Scratch::new();
    assert_default(&AppConfig::load(&d.path("config.json")).unwrap());
}

#[test]
fn empty_or_whitespace_gives_default() {
    for text in ["", "  \n\t "] {
        let d = Scratch::new();
        assert_default(&AppConfig::load(&write(&d, text)).unwrap());
    }
}

#[test]
fn corrupt_or_non_object_fails_instead_of_giving_default() {
    for text in ["{not json", "[1,2]", "\"text\"", "null"] {
        let d = Scratch::new();
        let p = write(&d, text);
        match AppConfig::load(&p) {
            Err(ConfigError::Format(m)) => assert_eq!(m, "config.json isn't valid JSON, so Sobreiro left it alone."),
            other => panic!("{text}: {other:?}"),
        }
        assert_eq!(std::fs::read_to_string(&p).unwrap(), text);
    }
}

#[test]
fn a_blank_chained_command_is_no_command() {
    for command in ["", "   "] {
        let d = Scratch::new();
        let loaded = AppConfig::load(&write(&d, &format!("{{\"chained_command\":\"{command}\"}}"))).unwrap();
        assert!(loaded.chained_command.is_none());
    }
}

#[test]
fn round_trips_every_field() {
    let d = Scratch::new();
    let p = d.path("config.json");
    let raw = "{\"type\":\"command\",\"command\":\"x\",\"padding\":0}";
    let config = AppConfig {
        chained_command: Some("node a.js".into()),
        previous: Some(PreviousStatusLine { raw_json: Some(raw.into()) }),
        style: FlyoutStyle::Thermometer,
        bar_colour: BarColourOption::Accent,
        extra: vec![("future".into(), "[1, 2]".into())],
        ..AppConfig::default()
    };
    config.save(&p).unwrap();
    let loaded = AppConfig::load(&p).unwrap();
    assert_eq!(loaded.chained_command.as_deref(), Some("node a.js"));
    assert_eq!(loaded.previous.as_ref().unwrap().raw_json.as_deref(), Some(raw));
    assert_eq!(loaded.style, FlyoutStyle::Thermometer);
    assert_eq!(loaded.bar_colour, BarColourOption::Accent);
    assert_eq!(extra(&loaded, "future"), Some("[1, 2]"));
}

#[test]
fn an_old_live_usage_key_loads_and_is_kept_untouched_as_an_unknown_key() {
    for old in ["true", "false"] {
        let d = Scratch::new();
        let p = write(&d, &format!("{{\"live_usage\":{old},\"future\":[1]}}"));
        let loaded = AppConfig::load(&p).unwrap();
        assert_eq!(extra(&loaded, "live_usage"), Some(old));
        assert_eq!(extra(&loaded, "future"), Some("[1]"));
        loaded.save(&p).unwrap();
        let again = AppConfig::load(&p).unwrap();
        assert_eq!(extra(&again, "live_usage"), Some(old));
        assert_eq!(extra(&again, "future"), Some("[1]"));
    }
}

#[test]
fn previous_status_line_object_text_comes_back_byte_for_byte() {
    let d = Scratch::new();
    let raw = "{\"type\":\"command\",\"command\":\"x\",\"padding\":0}";
    let p = write(&d, &format!("{{\"previous_status_line\":{raw}}}"));
    let loaded = AppConfig::load(&p).unwrap();
    assert_eq!(loaded.previous.as_ref().unwrap().raw_json.as_deref(), Some(raw));
    loaded.save(&p).unwrap();
    assert_eq!(AppConfig::load(&p).unwrap().previous.unwrap().raw_json.as_deref(), Some(raw));
}

#[test]
fn previous_null_is_distinct_from_absent() {
    let d = Scratch::new();
    let p = write(&d, "{\"previous_status_line\":null}");
    let present = AppConfig::load(&p).unwrap();
    assert_eq!(present.previous, Some(PreviousStatusLine { raw_json: None }));
    present.save(&p).unwrap();
    assert_eq!(AppConfig::load(&p).unwrap().previous, Some(PreviousStatusLine { raw_json: None }));
    write(&d, "{}");
    assert!(AppConfig::load(&p).unwrap().previous.is_none());
}

#[test]
fn unknown_key_survives_load_and_save() {
    let d = Scratch::new();
    let p = write(&d, "{\"future\": [1, 2], \"style\":\"battery\"}");
    AppConfig::load(&p).unwrap().save(&p).unwrap();
    let again = AppConfig::load(&p).unwrap();
    assert_eq!(extra(&again, "future"), Some("[1, 2]"));
    assert_eq!(again.style, FlyoutStyle::Battery);
}

#[test]
fn style_and_colour_load_and_unknown_values_fall_back() {
    let d = Scratch::new();
    let a = AppConfig::load(&write(&d, "{\"style\":\"battery\",\"bar_colour\":\"accent\"}")).unwrap();
    assert_eq!((a.style, a.bar_colour), (FlyoutStyle::Battery, BarColourOption::Accent));
    let b = AppConfig::load(&write(&d, "{\"style\":\"hologram\",\"bar_colour\":\"mauve\"}")).unwrap();
    assert_eq!((b.style, b.bar_colour), (FlyoutStyle::Bars, BarColourOption::Purple));
}

#[test]
fn a_file_that_cannot_be_read_fails_instead_of_giving_default() {
    let d = Scratch::new();
    let p = write(&d, "{\"chained_command\":\"x\"}");
    let _held = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(&p).unwrap();
    assert!(matches!(AppConfig::load(&p), Err(ConfigError::Io(_))));
}

#[test]
fn a_missing_folder_gives_default() {
    let d = Scratch::new();
    assert_default(&AppConfig::load(&d.path("nope/config.json")).unwrap());
}

#[test]
fn numeric_strings_are_not_enum_values() {
    for number in ["1", "0"] {
        let d = Scratch::new();
        let cfg = AppConfig::load(&write(&d, &format!("{{\"style\":\"{number}\",\"bar_colour\":\"{number}\"}}"))).unwrap();
        assert_eq!((cfg.style, cfg.bar_colour), (FlyoutStyle::Bars, BarColourOption::Purple));
    }
}

#[test]
fn commands_are_written_readably() {
    let d = Scratch::new();
    let p = d.path("config.json");
    let command = "a && b \"c\"";
    AppConfig { chained_command: Some(command.into()), ..AppConfig::default() }.save(&p).unwrap();
    assert!(std::fs::read_to_string(&p).unwrap().contains("\"chained_command\": \"a && b \\\"c\\\"\""));
    assert_eq!(AppConfig::load(&p).unwrap().chained_command.as_deref(), Some(command));
}

#[test]
fn theme_reads_do_not_panic() {
    // The Windows registry reads: only that they answer. The values depend on the PC.
    let _ = sobreiro_core::theme::WindowsTheme::is_light();
    let _ = sobreiro_core::theme::WindowsTheme::accent_taskbar();
    let _ = sobreiro_core::theme::WindowsTheme::accent();
}
