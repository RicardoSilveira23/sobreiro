//! SPIKE (shell candidate B): `--snapshot folder` renders the pill in both taskbar themes, and the flyout, to
//! pictures, from made-up figures. No tray, no network. The scale is the one the toolkit is started at
//! (`SLINT_SCALE_FACTOR`), so one run gives one scale.
use crate::{Flyout, PillFace, Sheet, faces, png, shell, trace};
use chrono::{Datelike, Months};
use slint::{Color, ComponentHandle, ModelRc, VecModel};
use sobreiro_core::colours::{BarColour, BarColourOption, NightPalette, TaskbarColours};
use sobreiro_core::copilot::{CopilotReading, CopilotSource};
use sobreiro_core::display::{FlyoutModel, FlyoutStyle, PaceDays, PillModel, PillStale};
use sobreiro_core::limits::{Limit, LimitKind, LimitList};
use sobreiro_core::polling::ProviderState;
use sobreiro_core::{Span, Time, Zone};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn claude(now: Time, observed: Time) -> Vec<Limit> {
    let limit = |kind, label: &str, percent, resets_at, window| Limit {
        provider: "Claude Code".into(),
        kind,
        label: label.into(),
        percent,
        resets_at: Some(resets_at),
        observed_at: observed,
        is_reset: false,
        window: Some(window),
        used_usd: None,
        limit_usd: None,
        period: None,
        used_credits: None,
        credit_budget: None,
    };
    vec![
        limit(LimitKind::Session, "Session", 27.0, now + Span::minutes(191), Span::hours(5)),
        limit(LimitKind::Weekly, "Weekly", 74.0, now + Span::hours(85), Span::days(7)),
    ]
}

pub fn run(folder: &str) -> Result<(), slint::PlatformError> {
    let folder = PathBuf::from(folder);
    let _ = std::fs::create_dir_all(&folder);
    let now = sobreiro_core::now();
    let observed = now - Span::minutes(3);
    let first = now.date_naive().with_day(1).and_then(|d| d.checked_add_months(Months::new(1)));
    let reading = |observed_at| CopilotReading {
        budget: 15000.0,
        used: 2644.0,
        resets_at: first.map(|date| Zone::Local.midnight(date)),
        observed_at,
        source: CopilotSource::Live,
    };
    let mut both = claude(now, observed);
    both.extend(LimitList::from_copilot(Some(&reading(observed)), now));
    let old = now - Span::days(3);
    let mut stale = claude(now, old);
    stale.extend(LimitList::from_copilot(Some(&reading(old)), now));

    let mut windows: Vec<(String, Box<dyn Fn() -> Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>>>)> = Vec::new();
    for light in [false, true] {
        let colours = TaskbarColours::for_taskbar(light, false, BarColourOption::Purple, None);
        let list: Vec<PillFace> = vec![
            faces::pill(&PillModel::from(&[], Zone::Local, true), &colours, ""),
            faces::pill(&PillModel::from(&claude(now, observed), Zone::Local, true), &colours, ""),
            faces::pill(&PillModel::from(&both, Zone::Local, true), &colours, ""),
            faces::pill(
                &PillModel::from_state(
                    &stale,
                    Zone::Local,
                    true,
                    ProviderState::Refused,
                    PillStale::MISSES_NEEDED,
                    ProviderState::Offline,
                    PillStale::MISSES_NEEDED,
                    now,
                ),
                &colours,
                "",
            ),
        ];
        shell::next_window_inactive();
        let sheet = Sheet::new()?;
        sheet.set_faces(ModelRc::new(VecModel::from(list)));
        sheet.set_ground(if light { Color::from_rgb_u8(0xF0, 0xF0, 0xF0) } else { Color::from_rgb_u8(0x1C, 0x1C, 0x1C) });
        sheet.window().set_size(slint::LogicalSize::new(sheet.get_sheet_width().ceil(), 52.0));
        sheet.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
        sheet.show()?;
        let name = format!("pill-{}", if light { "light" } else { "dark" });
        windows.push((name, Box::new(move || sheet.window().take_snapshot().ok())));
    }
    for style in [FlyoutStyle::Bars, FlyoutStyle::Battery, FlyoutStyle::Thermometer] {
        let model = FlyoutModel::from(&both, now, Zone::Local, ProviderState::Ok, false, None, true, PaceDays::EveryDay);
        let fill = BarColour::pick(BarColourOption::Purple, None, true).colour;
        shell::next_window_inactive();
        let flyout = Flyout::new()?;
        flyout.set_card(faces::flyout(&model, style, &NightPalette::for_dark(true), fill));
        flyout.window().set_position(slint::PhysicalPosition::new(-32000, -32000));
        flyout.show()?;
        let name = format!("flyout-{}", format!("{style:?}").to_lowercase());
        windows.push((name, Box::new(move || flyout.window().take_snapshot().ok())));
    }

    slint::Timer::single_shot(Duration::from_millis(600), move || {
        for (name, take) in &windows {
            match take() {
                Some(picture) => save(&folder, name, &picture),
                None => trace!("{name}: no picture"),
            }
        }
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop()
}

fn save(folder: &Path, name: &str, picture: &slint::SharedPixelBuffer<slint::Rgba8Pixel>) {
    let scale = crate::spike::env("SLINT_SCALE_FACTOR").and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
    let file = folder.join(format!("{name}-{:.0}.png", scale * 100.0));
    let result = png::write(&file, picture.width(), picture.height(), picture.as_bytes());
    trace!("{}: {}x{} px, saved={}", file.display(), picture.width(), picture.height(), result.is_ok());
}
