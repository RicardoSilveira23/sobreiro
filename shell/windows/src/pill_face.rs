//! From Core's `PillModel` and the taskbar's colours to what `ui/pill.slint` draws. The .slint
//! file only draws; every decision about what it draws (which colour a ring takes, what a stale
//! block looks like, where a ring's arc ends) is made here, where it is tested.
use crate::ui::{FaceData, FaceLine};
use slint::{Color, SharedString};
use sobreiro_core::colours::taskbar_colours::{STALE_ALPHA, TRACK_ALPHA};
use sobreiro_core::colours::{Rgb, TaskbarColours};
use sobreiro_core::display::pill_model::{PillLine, PillModel};
use sobreiro_core::display::rings::Ring;

/// The ring canvas is 26 DIPs, the stroke 3 and the gap between the rings 2: the outer ring's
/// centre line is half a stroke inside the canvas, the inner one a stroke and a gap further in.
const RINGS_SIZE: f64 = 26.0;
const STROKE: f64 = 3.0;
const RING_GAP: f64 = 2.0;
const OUTER_RADIUS: f64 = RINGS_SIZE / 2.0 - STROKE / 2.0;
const INNER_RADIUS: f64 = OUTER_RADIUS - STROKE - RING_GAP;

/// The reset date's alpha: the ink at 60%.
const RESET_ALPHA: u8 = 0x99;
/// The ground under the pointer: the ink at 10%.
const HOVER_ALPHA: u8 = 0x1A;
/// The ground at rest: nearly clear, so clicks land on the pill's window and not behind it.
const REST_ALPHA: u8 = 1;

fn colour(rgb: Rgb, alpha: u8) -> Color {
    Color::from_argb_u8(alpha, rgb.r, rgb.g, rgb.b)
}

/// The label and the figure as text; the .slint file sets the figure in a box of one width.
fn face_line(line: &PillLine) -> FaceLine {
    FaceLine {
        label: SharedString::from(line.label),
        figure: SharedString::from(line.figure.as_str()),
        star: line.stale,
    }
}

/// A stopped block's fill is the ink at half strength: the level's colour goes, the amount stays.
fn fill(colours: &TaskbarColours, level: sobreiro_core::limits::UsageLevel, stale: bool) -> Color {
    if stale {
        colour(colours.ink, STALE_ALPHA)
    } else {
        colour(colours.fill(level), 0xFF)
    }
}

/// The degrees of an arc, drawn with round caps so what shows spans the ring's percentage.
fn sweep(ring: &Ring, radius: f64) -> f32 {
    ring.capped_sweep(radius, STROKE) as f32
}

pub fn face_data(model: &PillModel, colours: &TaskbarColours, hovered: bool) -> FaceData {
    let ink = colours.ink;
    let mut data = FaceData {
        show_claude: model.show_claude,
        show_copilot: model.copilot.is_some(),
        hovered,
        ink: colour(ink, 0xFF),
        track: colour(ink, TRACK_ALPHA),
        secondary: colour(ink, RESET_ALPHA),
        ground: colour(ink, REST_ALPHA),
        ground_hover: colour(ink, HOVER_ALPHA),
        session: face_line(&model.session),
        weekly: face_line(&model.weekly),
        outer_sweep: sweep(&model.rings.outer, OUTER_RADIUS),
        outer_fill: fill(colours, model.rings.outer.level, model.claude_stale),
        inner_sweep: sweep(&model.rings.inner, INNER_RADIUS),
        inner_fill: fill(colours, model.rings.inner.level, model.claude_stale),
        ..FaceData::default()
    };
    if let Some(copilot) = &model.copilot {
        data.copilot = face_line(&copilot.credits);
        data.reset_date = SharedString::from(copilot.reset_date.as_str());
        data.bar_fraction = copilot.fraction.clamp(0.0, 1.0) as f32;
        data.bar_fill = fill(colours, copilot.level, copilot.stale);
    }
    data
}

#[cfg(test)]
// A test fails by panicking.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sobreiro_core::colours::{BarColourOption, WINDOWS_BLUE};
    use sobreiro_core::display::pill_model::PillCopilot;
    use sobreiro_core::display::rings::RingsModel;
    use sobreiro_core::limits::UsageLevel;
    use sobreiro_core::limits::usage_level::for_percent;

    fn ring(percent: i32) -> Ring {
        Ring {
            percent,
            level: for_percent(percent),
            has_data: true,
        }
    }

    fn line(label: &'static str, figure: &str, stale: bool) -> PillLine {
        PillLine {
            label,
            figure: figure.to_string(),
            stale,
        }
    }

    fn model(session: i32, weekly: i32, stale: bool) -> PillModel {
        PillModel {
            session: line("5h", &format!("{session}%"), stale),
            weekly: line("7d", &format!("{weekly}%"), stale),
            rings: RingsModel {
                outer: ring(session),
                inner: ring(weekly),
                has_inner: true,
            },
            copilot: None,
            show_claude: true,
            claude_stale: stale,
        }
    }

    fn dark() -> TaskbarColours {
        TaskbarColours::for_taskbar(false, false, BarColourOption::Purple, None)
    }

    #[test]
    fn a_line_is_its_label_its_figure_as_text_and_its_star() {
        let data = face_data(&model(5, 100, false), &dark(), false);
        assert_eq!(data.session.label, "5h");
        assert_eq!(data.session.figure, "5%");
        assert_eq!(data.weekly.figure, "100%");
        assert!(!data.session.star);
    }

    #[test]
    fn a_fresh_reading_takes_the_level_colours_and_the_ink_for_text() {
        let colours = dark();
        let data = face_data(&model(75, 95, false), &colours, false);
        assert!(data.show_claude && !data.show_copilot);
        assert_eq!(data.ink, colour(colours.ink, 0xFF));
        assert_eq!(data.outer_fill, colour(colours.warning, 0xFF));
        assert_eq!(data.inner_fill, colour(colours.error, 0xFF));
        assert_eq!(data.track, colour(colours.ink, 0x26));
        assert!(!data.session.star && !data.weekly.star);
    }

    #[test]
    fn a_stale_block_has_a_star_and_the_ink_at_half_strength_for_a_fill() {
        let colours = dark();
        let data = face_data(&model(95, 45, true), &colours, false);
        assert!(data.session.star && data.weekly.star);
        assert_eq!(data.outer_fill, colour(colours.ink, 0x80));
        assert_eq!(data.inner_fill, colour(colours.ink, 0x80));
    }

    #[test]
    fn the_ground_is_nearly_clear_at_rest_and_the_ink_at_a_tenth_under_the_pointer() {
        let colours = dark();
        let rest = face_data(&model(15, 45, false), &colours, false);
        let over = face_data(&model(15, 45, false), &colours, true);
        assert!(!rest.hovered && over.hovered);
        assert_eq!(rest.ground, colour(colours.ink, 1));
        assert_eq!(rest.ground_hover, colour(colours.ink, 0x1A));
        assert_eq!(rest.secondary, colour(colours.ink, 0x99));
    }

    #[test]
    fn the_arcs_start_at_the_percentage_less_the_caps_and_are_full_at_100() {
        let data = face_data(&model(50, 100, false), &dark(), false);
        // 180 degrees less the caps' half a stroke each side of a 11.5 radius.
        let caps = (STROKE / OUTER_RADIUS * 180.0 / std::f64::consts::PI) as f32;
        assert!((data.outer_sweep - (180.0 - caps)).abs() < 0.01);
        assert_eq!(data.inner_sweep, 360.0);
    }

    #[test]
    fn no_reading_is_a_dash_and_two_empty_tracks() {
        let mut none = model(0, 0, false);
        none.session = line("5h", "–", false);
        none.weekly = line("7d", "–", false);
        none.rings = RingsModel {
            outer: Ring::NONE,
            inner: Ring::NONE,
            has_inner: true,
        };
        let data = face_data(&none, &dark(), false);
        assert_eq!(data.session.figure, "–");
        assert_eq!(data.weekly.figure, "–");
        assert_eq!(data.outer_sweep, 0.0);
        assert_eq!(data.inner_sweep, 0.0);
    }

    #[test]
    fn copilots_block_has_its_figure_date_and_a_bar_in_the_level_colour() {
        let colours =
            TaskbarColours::for_taskbar(true, false, BarColourOption::Accent, Some(&WINDOWS_BLUE));
        let mut both = model(15, 45, false);
        both.copilot = Some(PillCopilot {
            credits: line("AI", "92%", false),
            reset_date: "1 Dec".to_string(),
            fraction: 0.92,
            level: UsageLevel::Error,
            stale: false,
        });
        let data = face_data(&both, &colours, false);
        assert!(data.show_claude && data.show_copilot);
        assert_eq!(data.copilot.label, "AI");
        assert_eq!(data.reset_date, "1 Dec");
        assert!((data.bar_fraction - 0.92).abs() < 1e-6);
        assert_eq!(data.bar_fill, colour(colours.error, 0xFF));

        both.copilot.as_mut().unwrap().stale = true;
        both.copilot.as_mut().unwrap().credits.stale = true;
        let stale = face_data(&both, &colours, false);
        assert!(stale.copilot.star);
        assert_eq!(stale.bar_fill, colour(colours.ink, 0x80));
        assert!(!stale.session.star, "Claude's block is not stale");
    }

    #[test]
    fn without_claudes_block_only_copilots_shows() {
        let mut only = model(15, 45, false);
        only.show_claude = false;
        only.copilot = Some(PillCopilot {
            credits: line("AI", "17%", false),
            reset_date: "1 Dec".to_string(),
            fraction: 0.17,
            level: UsageLevel::Normal,
            stale: false,
        });
        let data = face_data(&only, &dark(), false);
        assert!(!data.show_claude && data.show_copilot);
    }
}
