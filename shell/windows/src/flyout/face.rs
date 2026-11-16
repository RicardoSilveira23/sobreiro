//! From Core's `FlyoutModel` and the Night palette to what `ui/flyout.slint` draws. The .slint
//! file only draws; every decision about what it draws (which colour a bar takes, whether a pace
//! tick shows, what a badge says and in which colours) is made here, where it is tested.
use crate::ui::{BarData, CardColours, FlyoutData};
use slint::{Color, SharedString};
use sobreiro_core::colours::{NightPalette, Rgb};
use sobreiro_core::display::FlyoutStyle;
use sobreiro_core::display::flyout_model::{CopilotSection, FlyoutModel, LimitView};
use sobreiro_core::display::glyph_fill::GlyphFill;
use sobreiro_core::limits::providers::COPILOT;

fn colour(rgb: Rgb) -> Color {
    Color::from_rgb_u8(rgb.r, rgb.g, rgb.b)
}

fn text(value: Option<&String>) -> SharedString {
    SharedString::from(value.map_or("", String::as_str))
}

/// How full a meter is, from 0 to 1; a number that is not one fills nothing.
fn fraction(value: f64) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0) as f32
    }
}

/// One limit's row. `fill` is the colour of the bars below 70%: the level's own colour is
/// Core's to pick, from the palette. The tick shows only where the pace ends before the reset.
fn bar(limit: &LimitView, note: Option<&String>, palette: &NightPalette, fill: Rgb) -> BarData {
    let glyph = GlyphFill::from(limit);
    let mut data = BarData {
        has_data: limit.has_data,
        left_figure: SharedString::from(limit.left_figure.as_str()),
        left_cells: glyph.left_cells,
        soon_cells: glyph.soon_cells,
        left_percent: glyph.left_percent as f32,
        soon_percent: glyph.soon_percent as f32,
        label: SharedString::from(limit.label),
        figure: SharedString::from(limit.used_figure.as_str()),
        fraction: fraction(limit.used_fraction),
        tick: -1.0,
        fill: colour(palette.level_colour(limit.level, fill)),
        resets: text(limit.resets_in.as_ref()),
        detail: text(limit.detail.as_ref()),
        note: text(note),
        ..BarData::default()
    };
    if let Some(projected) = limit.projected_fraction
        && projected < 1.0
    {
        data.tick = fraction(projected);
    }
    // With no reading there is no badge, only the words for it.
    if limit.has_data
        && let Some(status) = &limit.status
    {
        let badge = palette.badge(status.kind);
        data.badge = SharedString::from(status.text.as_str());
        data.badge_fill = colour(badge.fill);
        data.badge_ink = colour(badge.text);
    }
    data
}

/// The card's data. `style` is how session and weekly are drawn; spend and credits stay bars.
pub fn flyout_data(
    model: &FlyoutModel,
    style: FlyoutStyle,
    palette: &NightPalette,
    fill: Rgb,
) -> FlyoutData {
    let row = |limit: &LimitView| bar(limit, None, palette, fill);
    let mut data = FlyoutData {
        style: match style {
            FlyoutStyle::Bars => 0,
            FlyoutStyle::Battery => 1,
            FlyoutStyle::Thermometer => 2,
        },
        show_claude: model.show_claude,
        show_spend: model.spend.is_some(),
        show_copilot: model.copilot.is_some(),
        claude_title: SharedString::from(model.title),
        copilot_title: SharedString::from(COPILOT),
        session: row(&model.session),
        weekly: row(&model.weekly),
        footer: SharedString::from(model.footer.as_str()),
        colours: CardColours {
            ground: colour(palette.ground),
            line: colour(palette.line),
            ink: colour(palette.ink),
            soft: colour(palette.soft),
            track: colour(palette.track),
        },
        ..FlyoutData::default()
    };
    if let Some(spend) = &model.spend {
        data.spend = row(spend);
    }
    if let Some(CopilotSection { credits, note }) = &model.copilot {
        data.credits = bar(credits, note.as_ref(), palette, fill);
    }
    data
}

#[cfg(test)]
// A test fails by panicking.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::states::states;
    use super::*;
    use sobreiro_core::colours::bar_colour::pick;
    use sobreiro_core::colours::{BarColourOption, WINDOWS_BLUE};

    fn data(state: &str, dark: bool) -> FlyoutData {
        let model = states()
            .into_iter()
            .find(|(name, _)| *name == state)
            .unwrap()
            .1;
        let fill = pick(BarColourOption::Purple, None, dark).colour;
        flyout_data(
            &model,
            FlyoutStyle::Bars,
            &NightPalette::for_dark(dark),
            fill,
        )
    }

    #[test]
    fn what_the_model_shows_is_what_the_card_shows() {
        let both = data("both", true);
        assert!(both.show_claude && both.show_copilot && !both.show_spend);
        assert_eq!(both.claude_title, "Claude Code");
        assert_eq!(both.copilot_title, "GitHub Copilot");
        assert_eq!(both.footer, "Updated 1 min ago");

        let only = data("copilot-only", true);
        assert!(!only.show_claude && only.show_copilot);
        assert!(data("claude-only", true).show_claude);
        assert!(!data("claude-only", true).show_copilot);
        assert!(data("with-spend", true).show_spend);
        assert_eq!(
            data("with-spend", true).spend.detail,
            "$21.00 of $50.00 this month"
        );
    }

    #[test]
    fn a_row_carries_core_s_words() {
        let session = data("claude-only", true).session;
        assert_eq!(session.label, "Session");
        assert_eq!(session.figure, "15%");
        assert_eq!(session.badge, "On track");
        assert_eq!(session.resets, "resets in 2h 21m");
        let credits = data("both", true).credits;
        assert_eq!(credits.detail, "2,550 of 15,000 credits");
        assert_eq!(credits.note, "");
        assert_eq!(
            data("copilot-note", true).credits.note,
            "From the Copilot CLI, as of 09:25."
        );
    }

    #[test]
    fn the_seven_badges_say_what_core_says_in_the_palette_s_colours() {
        let words = [
            ("badge-on-track", "On track"),
            ("badge-ends", "Ends 9:15 PM"),
            ("badge-likely-used-up", "Likely used up"),
            ("badge-used-up", "Used up"),
            ("badge-not-used-yet", "Not used yet"),
            ("badge-too-early", "Too early"),
            ("badge-resetting", "Resetting"),
        ];
        let palette = NightPalette::for_dark(true);
        for (state, said) in words {
            let session = data(state, true).session;
            assert_eq!(session.badge, said, "{state}");
        }
        let ends = data("badge-ends", true).session;
        assert_eq!(ends.badge_fill, colour(palette.warning));
        assert_eq!(ends.badge_ink, colour(palette.ground));
        let used = data("badge-used-up", false).session;
        assert_eq!(used.badge_fill, colour(NightPalette::for_dark(false).error));
    }

    #[test]
    fn no_reading_is_a_dash_and_words_with_no_badge() {
        let session = data("no-reading", true).session;
        assert_eq!(session.figure, "–");
        assert_eq!(session.badge, "");
        assert_eq!(session.resets, "No reading yet");
        assert_eq!(session.fraction, 0.0);
        assert!(session.tick < 0.0);
    }

    #[test]
    fn the_tick_shows_where_the_pace_ends_before_the_reset() {
        let session = data("claude-only", true).session;
        assert!((session.tick - 0.28).abs() < 0.01, "{}", session.tick);
        // At 70% of the session after 3 of 5 hours the pace is past the end: no tick.
        assert!(data("badge-ends", true).session.tick < 0.0);
        assert!((session.fraction - 0.15).abs() < 1e-6);
    }

    #[test]
    fn the_fill_is_the_level_s_colour_and_below_70_the_bar_colour() {
        let palette = NightPalette::for_dark(true);
        let purple = colour(palette.purple);
        assert_eq!(data("claude-only", true).session.fill, purple);
        let accent = pick(BarColourOption::Accent, Some(&WINDOWS_BLUE), true).colour;
        let model = states().into_iter().next().unwrap().1;
        let blue = flyout_data(&model, FlyoutStyle::Bars, &palette, accent);
        assert_eq!(blue.session.fill, colour(accent));
        assert_eq!(
            data("badge-used-up", true).session.fill,
            colour(palette.error)
        );
        assert_eq!(
            data("badge-likely-used-up", true).session.fill,
            colour(palette.error)
        );
    }

    #[test]
    fn a_glyph_is_filled_as_core_s_glyph_fill_says() {
        // 45% used, the pace ending at 80%: 5 cells left of which 4 are the pace's, 55% of which
        // 35% is.
        let weekly = data("claude-only", true).weekly;
        assert_eq!(weekly.left_figure, "55%");
        assert!(weekly.has_data);
        assert_eq!((weekly.left_cells, weekly.soon_cells), (5, 4));
        assert!((weekly.left_percent - 55.0).abs() < 1e-3);
        assert!((weekly.soon_percent - 35.0).abs() < 0.5);
        // With no reading nothing is lit and the figure is a dash.
        let none = data("no-reading", true).session;
        assert!(!none.has_data);
        assert_eq!(
            (none.left_figure.as_str(), none.left_cells, none.soon_cells),
            ("–", 0, 0)
        );
        assert_eq!(none.left_percent, 0.0);
    }

    #[test]
    fn the_style_is_what_the_card_is_told() {
        let model = states().into_iter().next().unwrap().1;
        let palette = NightPalette::for_dark(true);
        let fill = palette.purple;
        let style = |style| flyout_data(&model, style, &palette, fill).style;
        assert_eq!(style(FlyoutStyle::Bars), 0);
        assert_eq!(style(FlyoutStyle::Battery), 1);
        assert_eq!(style(FlyoutStyle::Thermometer), 2);
    }
}
