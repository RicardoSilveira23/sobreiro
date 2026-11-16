//! The flyout's card, drawn by `Sobreiro.exe --flyout-pictures`: each state at the size 1.1's card
//! has, in the colours Core gives, with a row's label, figure and unit on one baseline and a far
//! `Ends` badge's resets-in text on a line of its own.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{DateTime, TimeDelta};
use sobreiro_core::colours::bar_colour::pick;
use sobreiro_core::colours::{BarColourOption, NightPalette, Rgb, WINDOWS_BLUE};
use sobreiro_core::display::flyout_model::{FlyoutModel, FlyoutOptions};
use sobreiro_core::display::glyph_fill::GlyphFill;
use sobreiro_core::display::{ClockFormat, Zone};
use sobreiro_core::limits::{Limit, LimitKind};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");
const SCALES: [(&str, f32); 3] = [("1", 1.0), ("1.5", 1.5), ("2", 2.0)];
const STYLES: [&str; 3] = ["bars", "battery", "thermometer"];
const STATES: [&str; 18] = [
    "claude-only",
    "copilot-only",
    "both",
    "with-spend",
    "no-reading",
    "empty-and-full",
    "badge-on-track",
    "badge-ends",
    "badge-likely-used-up",
    "badge-used-up",
    "badge-not-used-yet",
    "badge-too-early",
    "badge-resetting",
    "far-ends",
    "long-footer",
    "long-note",
    "spend-no-reset",
    "copilot-note",
];
const THEMES: [(&str, bool); 2] = [("light", false), ("dark", true)];
const COLOURS: [&str; 2] = ["purple", "accent"];

/// The card's size in DIPs for the states that 1.1 has stills of
/// (`media/stills/details/*-bars-*@1x.png`).
const CLAUDE_ONLY: (u32, u32) = (330, 289);
const COPILOT_ONLY: (u32, u32) = (330, 219);
const BOTH: (u32, u32) = (330, 452);
const BATTERY_CLAUDE_ONLY: (u32, u32) = (330, 277);
const BATTERY_BOTH: (u32, u32) = (330, 440);
const THERMOMETER_CLAUDE_ONLY: (u32, u32) = (330, 341);
const THERMOMETER_BOTH: (u32, u32) = (330, 504);

/// A row of the card is 94 DIPs tall, the first starts at 54 and its meter lies 42 under its top.
const FIRST_ROW: f32 = 54.0;

struct Picture {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Picture {
    fn pixel(&self, x: usize, y: usize) -> [u8; 3] {
        let at = (y * self.width + x) * 4;
        [self.rgba[at], self.rgba[at + 1], self.rgba[at + 2]]
    }

    /// The pixel at a point in DIPs.
    fn at(&self, scale: f32, x: f32, y: f32) -> [u8; 3] {
        self.pixel((x * scale) as usize, (y * scale) as usize)
    }

    /// Whether a pixel of the rectangle (in DIPs) is `want` within `room` on each channel.
    fn has(&self, scale: f32, x: (f32, f32), y: (f32, f32), want: Rgb, room: u8) -> bool {
        let columns = (x.0 * scale) as usize..((x.1 * scale) as usize).min(self.width);
        ((y.0 * scale) as usize..((y.1 * scale) as usize).min(self.height)).any(|row| {
            columns.clone().any(|column| {
                let [r, g, b] = self.pixel(column, row);
                r.abs_diff(want.r) <= room
                    && g.abs_diff(want.g) <= room
                    && b.abs_diff(want.b) <= room
            })
        })
    }

    /// Whether any pixel of the rectangle (in pixels) is not `ground` by more than `room`.
    fn differs(&self, x: (usize, usize), y: (usize, usize), ground: Rgb, room: u8) -> bool {
        (y.0..y.1.min(self.height)).any(|row| {
            (x.0..x.1.min(self.width)).any(|column| {
                let [r, g, b] = self.pixel(column, row);
                r.abs_diff(ground.r) > room
                    || g.abs_diff(ground.g) > room
                    || b.abs_diff(ground.b) > room
            })
        })
    }

    /// The rows of the rectangle (in DIPs) with a pixel that is not `ground`, as runs of
    /// consecutive rows: the lines of text in it.
    fn runs(&self, scale: f32, x: (f32, f32), y: (f32, f32), ground: Rgb) -> Vec<(usize, usize)> {
        let columns = ((x.0 * scale) as usize, (x.1 * scale) as usize);
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for row in (y.0 * scale) as usize..((y.1 * scale) as usize).min(self.height) {
            let ink = self.differs(columns, (row, row + 1), ground, 40);
            match runs.last_mut() {
                Some(last) if ink && last.1 + 1 == row => last.1 = row,
                _ if ink => runs.push((row, row)),
                _ => {}
            }
        }
        runs
    }

    /// The row (in pixels) most glyph columns of the rectangle (in DIPs) end on, a column ending
    /// on its lowest pixel at least four fifths of the way from `ground` to `ink`: the baseline, where the flat
    /// feet of stems and serifs stand, and not the faint dip of a round glyph below it.
    fn baseline(&self, scale: f32, x: (f32, f32), y: (f32, f32), ground: Rgb, ink: Rgb) -> usize {
        let mut feet = std::collections::BTreeMap::new();
        for column in (x.0 * scale) as usize..((x.1 * scale) as usize).min(self.width) {
            let lowest = ((y.0 * scale) as usize..((y.1 * scale) as usize).min(self.height))
                .rev()
                .find(|&row| self.inked(column, row, ground, ink));
            if let Some(row) = lowest {
                *feet.entry(row).or_insert(0) += 1;
            }
        }
        feet.into_iter()
            .max_by_key(|&(row, count)| (count, row))
            .unwrap()
            .0
    }

    fn inked(&self, column: usize, row: usize, ground: Rgb, ink: Rgb) -> bool {
        let span = |a: u8, b: u8| i32::from(a).abs_diff(i32::from(b)) as i32;
        let whole = span(ground.r, ink.r) + span(ground.g, ink.g) + span(ground.b, ink.b);
        let [r, g, b] = self.pixel(column, row);
        let away = span(r, ground.r) + span(g, ground.g) + span(b, ground.b);
        away * 5 >= whole * 4
    }

    fn ink_rows(
        &self,
        scale: f32,
        x: (f32, f32),
        y: (f32, f32),
        ground: Rgb,
        ink: Rgb,
    ) -> Vec<usize> {
        let columns = (x.0 * scale) as usize..((x.1 * scale) as usize).min(self.width);
        ((y.0 * scale) as usize..((y.1 * scale) as usize).min(self.height))
            .filter(|&row| {
                columns
                    .clone()
                    .any(|column| self.inked(column, row, ground, ink))
            })
            .collect()
    }
}

fn decode(path: &Path) -> Picture {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    Picture {
        width: info.width as usize,
        height: info.height as usize,
        rgba,
    }
}

fn name(state: &str, style: &str, theme: &str, colour: &str, scale: &str) -> String {
    format!("flyout-{state}-{style}-{theme}-{colour}-{scale}x.png")
}

/// Runs a copy of the exe that sits alone in a new folder, with a data folder and a Claude config
/// folder of its own, and returns the folder it wrote the pictures to. Once for all the tests.
fn pictures() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("flyout");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let exe = folder.join("Sobreiro.exe");
        std::fs::copy(EXE, &exe).unwrap();
        let output = Command::new(&exe)
            .arg("--flyout-pictures")
            .arg("pictures")
            .current_dir(&folder)
            .env("SOBREIRO_DATA_DIR", folder.join("data"))
            .env("CLAUDE_CONFIG_DIR", folder.join("claude"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let left: std::collections::BTreeSet<_> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            left,
            std::collections::BTreeSet::from(["Sobreiro.exe".to_string(), "pictures".to_string()]),
            "the run left files beside the exe"
        );
        let written: std::collections::BTreeSet<_> = std::fs::read_dir(folder.join("pictures"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        let mut expected = std::collections::BTreeSet::new();
        for state in STATES {
            for (theme, _) in THEMES {
                for colour in COLOURS {
                    for style in STYLES {
                        for (scale, _) in SCALES {
                            expected.insert(name(state, style, theme, colour, scale));
                        }
                    }
                }
            }
        }
        assert_eq!(expected.len(), STATES.len() * 3 * 2 * 2 * 3);
        assert_eq!(
            written, expected,
            "the run wrote other pictures than states x 3 x 2 x 2 x 3"
        );
        folder.join("pictures")
    })
}

/// A picture in the Bars style.
fn picture(state: &str, theme: &str, colour: &str, scale: &str) -> Picture {
    styled(state, "bars", theme, colour, scale)
}

fn styled(state: &str, style: &str, theme: &str, colour: &str, scale: &str) -> Picture {
    decode(&pictures().join(name(state, style, theme, colour, scale)))
}

fn near(have: [u8; 3], want: Rgb) -> bool {
    have.iter()
        .zip([want.r, want.g, want.b])
        .all(|(a, b)| a.abs_diff(b) <= 2)
}

fn fill(colour: &str, dark: bool) -> Rgb {
    let option = if colour == "purple" {
        BarColourOption::Purple
    } else {
        BarColourOption::Accent
    };
    pick(option, (colour == "accent").then_some(&WINDOWS_BLUE), dark).colour
}

#[test]
fn each_state_is_the_size_of_1_1s_card_at_every_scale() {
    for (state, (width, height)) in [
        ("claude-only", CLAUDE_ONLY),
        ("copilot-only", COPILOT_ONLY),
        ("both", BOTH),
    ] {
        for (theme, _) in THEMES {
            for colour in COLOURS {
                for (scale, factor) in SCALES {
                    let card = picture(state, theme, colour, scale);
                    // 1.1's size times the scale, a pixel for the rounding up of a half.
                    let (w, h) = (width as f32 * factor, height as f32 * factor);
                    let label = format!("{state} {theme} {colour} {scale}x");
                    assert!(
                        (card.width as f32 - w).abs() <= 1.0,
                        "{label}: {}",
                        card.width
                    );
                    assert!(
                        (card.height as f32 - h).abs() <= 1.0,
                        "{label}: {}",
                        card.height
                    );
                }
            }
        }
    }
}

#[test]
fn the_card_carries_the_palette_s_colours() {
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        for colour in COLOURS {
            for (scale, factor) in SCALES {
                let card = picture("claude-only", theme, colour, scale);
                let at = |x: f32, y: f32| card.at(factor, x, y);
                let meter = FIRST_ROW + 45.0;
                let label = format!("{theme} {colour} {scale}x");
                // The border on the middle of the top edge, the ground inside it.
                assert!(near(at(165.0, 0.4), palette.line), "border {label}");
                assert!(near(at(165.0, 8.0), palette.ground), "ground {label}");
                // Session is 15% of the meter: the bar colour to its left, the track past it.
                assert!(near(at(30.0, meter), fill(colour, dark)), "fill {label}");
                assert!(near(at(300.0, meter), palette.track), "track {label}");
                // The "On track" badge, in the padding left of its text.
                let on_track =
                    palette.badge(sobreiro_core::display::limit_status::StatusKind::OnTrack);
                assert!(
                    near(at(23.0, FIRST_ROW + 69.0), on_track.fill),
                    "badge {label}"
                );

                // 70% of the session is the warning colour in the bar and in the badge.
                let warm = picture("badge-ends", theme, colour, scale);
                let at = |x: f32, y: f32| warm.at(factor, x, y);
                assert!(
                    near(at(30.0, meter), palette.warning),
                    "warning bar {label}"
                );
                assert!(
                    near(at(23.0, FIRST_ROW + 69.0), palette.warning),
                    "warning badge {label}"
                );
                let used = picture("badge-used-up", theme, colour, scale);
                assert!(
                    near(used.at(factor, 30.0, meter), palette.error),
                    "error bar {label}"
                );
            }
        }
    }
}

#[test]
fn the_label_the_figure_and_the_unit_share_a_baseline() {
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        for (scale, factor) in SCALES {
            let card = picture("claude-only", theme, "purple", scale);
            let rows = (FIRST_ROW + 8.0, FIRST_ROW + 38.0);
            // "Session" and "used" have no descender; the figure's "%" and digits sit on the line.
            let label = card.baseline(factor, (19.0, 75.0), rows, palette.ground, palette.ink);
            let figure = card.baseline(factor, (215.0, 272.0), rows, palette.ground, palette.ink);
            let unit = card.baseline(factor, (278.0, 311.0), rows, palette.ground, palette.soft);
            assert_eq!(label, figure, "label and figure {theme} {scale}x");
            assert_eq!(unit, figure, "unit and figure {theme} {scale}x");
        }
    }
}

#[test]
fn the_badge_text_and_the_resets_in_text_share_a_baseline() {
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        let badge = palette.badge(sobreiro_core::display::limit_status::StatusKind::OnTrack);
        for (scale, factor) in SCALES {
            let card = picture("claude-only", theme, "purple", scale);
            let rows = (FIRST_ROW + 60.0, FIRST_ROW + 78.0);
            let on_track = card.baseline(factor, (30.0, 80.0), rows, badge.fill, badge.text);
            let resets = card.baseline(factor, (100.0, 210.0), rows, palette.ground, palette.soft);
            assert_eq!(
                on_track, resets,
                "badge and resets-in text {theme} {scale}x"
            );
        }
    }
}

/// The first and the last column (in pixels) of the rectangle (in pixels) with a pixel that is not
/// `ground` by more than 40 on a channel.
fn ink_columns(
    card: &Picture,
    x: (usize, usize),
    y: (usize, usize),
    ground: Rgb,
) -> (usize, usize) {
    let inked: Vec<usize> = (x.0..x.1.min(card.width))
        .filter(|&column| card.differs((column, column + 1), y, ground, 40))
        .collect();
    (*inked.first().unwrap(), *inked.last().unwrap())
}

#[test]
fn the_figures_and_the_resets_in_text_are_where_1_1_puts_them() {
    // The tabular copies of the fonts put the digits' left edges within a pixel of 1.1's, where the
    // proportional ones were up to four off. 1.1's still of the same state, dark, at 100%.
    let still = decode(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../media/stills/details/both-bars-dark@1x.png"),
    );
    let card = picture("both", "dark", "purple", "1");
    assert_eq!((card.width, card.height), (still.width, still.height));
    let ground = NightPalette::for_dark(true).ground;
    // The figures' lines, with the meter's tick below them left out, and the first row's
    // resets-in line, right of its badge.
    for (label, rows) in [
        ("session", (64, 92)),
        ("weekly", (158, 186)),
        ("copilot", (298, 326)),
    ] {
        let ours = ink_columns(&card, (195, 276), rows, ground).0;
        let theirs = ink_columns(&still, (195, 276), rows, ground).0;
        assert!(
            ours.abs_diff(theirs) <= 1,
            "the {label} figure starts at {ours}, 1.1's at {theirs}"
        );
    }
    let ours = ink_columns(&card, (100, 270), (114, 134), ground).1;
    let theirs = ink_columns(&still, (100, 270), (114, 134), ground).1;
    assert!(
        ours.abs_diff(theirs) <= 1,
        "the resets-in text ends at {ours}, 1.1's at {theirs}"
    );
}

#[test]
fn the_columns_figures_and_resets_in_texts_are_where_1_1_puts_them() {
    let ground = NightPalette::for_dark(true).ground;
    // Per style: 1.1's still, then the rows of the figures and of the resets-in texts.
    for (style, still, figures, resets) in [
        (
            "battery",
            "both-battery-dark@1x.png",
            (112, 140),
            (196, 216),
        ),
        (
            "thermometer",
            "both-thermometer-dark@1x.png",
            (172, 200),
            (260, 280),
        ),
    ] {
        let still = decode(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../media/stills/details")
                .join(still),
        );
        let card = styled("both", style, "dark", "purple", "1");
        assert_eq!((card.width, card.height), (still.width, still.height));
        for (column, x) in [("left", (20, 165)), ("right", (170, 320))] {
            let ours = ink_columns(&card, x, figures, ground).0;
            let theirs = ink_columns(&still, x, figures, ground).0;
            assert!(
                ours.abs_diff(theirs) <= 1,
                "{style} {column} figure starts at {ours}, 1.1's at {theirs}"
            );
            let ours = ink_columns(&card, x, resets, ground).1;
            let theirs = ink_columns(&still, x, resets, ground).1;
            assert!(
                ours.abs_diff(theirs) <= 1,
                "{style} {column} resets-in text ends at {ours}, 1.1's at {theirs}"
            );
        }
    }
}

#[test]
fn the_colon_in_a_time_is_raised_off_the_baseline() {
    // Inter's `calt` raises a colon between digits; a colon of the tabular copy that no rule
    // matched would stand on the baseline.
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        let badge = palette.badge(sobreiro_core::display::limit_status::StatusKind::Ends);
        let card = picture("badge-ends", theme, "purple", "2");
        let (x, y) = ((60, 235), (228, 264));
        let baseline = card.baseline(2.0, (30.0, 118.0), (114.0, 132.0), badge.fill, badge.text);
        // The glyphs of the text as runs of columns with ink; the colon is the one that is the
        // least tall.
        let ink = |column: usize| card.differs((column, column + 1), y, badge.fill, 60);
        let mut glyphs: Vec<(usize, usize)> = Vec::new();
        for column in x.0..x.1 {
            match glyphs.last_mut() {
                Some(last) if ink(column) && last.1 + 1 == column => last.1 = column,
                _ if ink(column) => glyphs.push((column, column)),
                _ => {}
            }
        }
        // The colon is the narrow glyph of two dots: two separate runs of rows with ink.
        let colon = glyphs
            .iter()
            .filter(|glyph| glyph.1 - glyph.0 <= 6)
            .filter_map(|glyph| {
                let rows: Vec<usize> = (y.0..y.1)
                    .filter(|&row| {
                        card.differs((glyph.0, glyph.1 + 1), (row, row + 1), badge.fill, 60)
                    })
                    .collect();
                let runs = rows
                    .windows(2)
                    .filter(|pair| pair[1] != pair[0] + 1)
                    .count()
                    + 1;
                (runs == 2).then(|| (rows[0], *rows.last().unwrap()))
            })
            .next()
            .expect("a colon in the time");
        assert!(
            colon.1 + 2 <= baseline,
            "{theme}: the colon ends on row {}, the digits' baseline is row {baseline}",
            colon.1
        );
    }
}

#[test]
fn a_far_ends_badge_puts_the_resets_in_text_on_a_second_line() {
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        let (soft, ground) = (palette.soft, palette.ground);
        for (scale, factor) in SCALES {
            let far = picture("far-ends", theme, "purple", scale);
            let near_badge = picture("copilot-only", theme, "purple", scale);
            let label = format!("{theme} {scale}x");
            // The badge's line is 59 to 79 under the row's top; its text starts at the badge's end
            // when it fits, and under the badge's left edge when it does not.
            let line = (FIRST_ROW + 60.0, FIRST_ROW + 78.0);
            let under = (FIRST_ROW + 82.0, FIRST_ROW + 100.0);
            let rows = |picture: &Picture, x, y| picture.ink_rows(factor, x, y, ground, soft).len();
            assert!(
                rows(&far, (19.0, 60.0), under) as f32 > 5.0 * factor,
                "{label}"
            );
            assert_eq!(rows(&far, (230.0, 311.0), line), 0, "{label}");
            assert!(
                rows(&near_badge, (100.0, 200.0), line) as f32 > 5.0 * factor,
                "{label}"
            );
            // A line more: 23 DIPs, the 20 of the line and its 4 of space less the 1 the badge no
            // longer drops to meet the baseline.
            let more = far.height as f32 - near_badge.height as f32;
            assert!((more - 23.0 * factor).abs() <= 1.0, "{label}: {more}");
        }
    }
}

#[test]
fn no_ink_lies_right_of_a_row_s_edge() {
    // A row ends 1 + 18 + 292 DIPs in. The border and the rounded corners are left out.
    for state in STATES {
        for style in STYLES {
            for (theme, dark) in THEMES {
                let ground = NightPalette::for_dark(dark).ground;
                for (scale, factor) in SCALES {
                    let card = styled(state, style, theme, "purple", scale);
                    let from = (311.0 * factor).ceil() as usize + 1;
                    let to = card.width - factor.ceil() as usize - 1;
                    let corner = (11.0 * factor) as usize;
                    let rows = (corner, card.height - corner);
                    let stray = card.differs((from, to), rows, ground, 8);
                    assert!(
                        !stray,
                        "{state} {style} {theme} {scale}x has ink right of the row"
                    );
                }
            }
        }
    }
}

#[test]
fn wrapped_lines_stack_at_1_45_times_the_size() {
    for (theme, dark) in THEMES {
        let ground = NightPalette::for_dark(dark).ground;
        for (scale, factor) in SCALES {
            // The note is in 14 DIP type: four lines, 20.3 DIPs apart whatever the scale.
            let card = picture("long-note", theme, "purple", scale);
            let note = (FIRST_ROW + 104.0, FIRST_ROW + 190.0);
            let lines = card.runs(factor, (19.0, 311.0), note, ground);
            assert_eq!(lines.len(), 4, "{theme} {scale}x: {lines:?}");
            // The tops of the lines differ by a pixel with their first letter (an ascender or a
            // capital), so the pitch is read over the first line to the last: three of them.
            let pitch = (lines[3].0 as f32 - lines[0].0 as f32) / factor / 3.0;
            assert!((pitch - 20.3).abs() <= 0.5, "{theme} {scale}x: {pitch}");
            let footer = picture("long-footer", theme, "purple", scale);
            let end = footer.height as f32 / factor;
            let lines = footer.runs(factor, (19.0, 311.0), (end - 100.0, end - 12.0), ground);
            assert_eq!(lines.len(), 4, "{theme} {scale}x: {lines:?}");
        }
    }
}

#[test]
fn the_text_is_in_its_colours() {
    for (theme, dark) in THEMES {
        let palette = NightPalette::for_dark(dark);
        for (scale, factor) in SCALES {
            let card = picture("claude-only", theme, "purple", scale);
            let label = format!("{theme} {scale}x");
            let meter = FIRST_ROW + 45.0;
            let heading = ((19.0, 110.0), (19.0, 38.0));
            assert!(
                card.has(factor, heading.0, heading.1, palette.soft, 12),
                "heading {label}"
            );
            assert!(
                !card.has(factor, heading.0, heading.1, palette.ink, 12),
                "heading in ink {label}"
            );
            let session = ((19.0, 75.0), (FIRST_ROW + 12.0, FIRST_ROW + 38.0));
            assert!(
                card.has(factor, session.0, session.1, palette.ink, 12),
                "label {label}"
            );
            assert!(
                near(card.at(factor, 100.0, 53.7), palette.ink),
                "heading rule {label}"
            );
            // The tick is 28% of the way along the meter, 3 above it.
            let tick = 19.0 + 292.0 * 0.28;
            let above = (meter - 5.0, meter - 3.0);
            assert!(
                card.has(factor, (tick - 3.0, tick + 3.0), above, palette.ink, 2),
                "tick {label}"
            );
            let used = picture("badge-used-up", theme, "purple", scale);
            assert!(
                near(used.at(factor, 23.0, FIRST_ROW + 69.0), palette.error),
                "error badge {label}"
            );
        }
    }
}

#[test]
fn a_detail_with_no_line_above_it_has_no_gap_above_it() {
    // A spend reading with no reset time has no badge and no resets-in line: its row is 93 DIPs
    // (14, the 22 line, 6 and the meter's 6 and 10, the 20 detail, 14 and the hairline), not 97.
    for (theme, _) in THEMES {
        let spend = picture("spend-no-reset", theme, "purple", "1");
        let claude = picture("claude-only", theme, "purple", "1");
        assert_eq!(spend.height, claude.height + 93, "{theme}");
    }
}

#[test]
fn the_battery_and_the_thermometer_are_the_size_of_1_1s_card_at_every_scale() {
    for (style, sizes) in [
        ("battery", [BATTERY_CLAUDE_ONLY, BATTERY_BOTH]),
        ("thermometer", [THERMOMETER_CLAUDE_ONLY, THERMOMETER_BOTH]),
    ] {
        for (state, (width, height)) in ["claude-only", "both"].into_iter().zip(sizes) {
            for (theme, _) in THEMES {
                for colour in COLOURS {
                    for (scale, factor) in SCALES {
                        let card = styled(state, style, theme, colour, scale);
                        let label = format!("{state} {style} {theme} {colour} {scale}x");
                        let (w, h) = (width as f32 * factor, height as f32 * factor);
                        assert!(
                            (card.width as f32 - w).abs() <= 1.0,
                            "{label}: {}",
                            card.width
                        );
                        assert!(
                            (card.height as f32 - h).abs() <= 1.0,
                            "{label}: {}",
                            card.height
                        );
                    }
                }
            }
        }
    }
}

/// The glyphs' fills for a session and a weekly limit read as `states.rs` reads them: Core's
/// `GlyphFill` of each, as (left cells, soon cells, left percent, soon percent).
type Fills = [(i32, i32, f64, f64); 2];

fn fills(session: f64, weekly: f64) -> Fills {
    let now = DateTime::from_timestamp(1_794_859_200, 0).unwrap();
    let limit = |kind, label, percent, resets_in: TimeDelta, window| {
        let mut limit = Limit::new(
            "Claude Code",
            kind,
            label,
            percent,
            Some(now + resets_in),
            now - TimeDelta::minutes(1),
            false,
        );
        limit.window = Some(window);
        limit
    };
    let limits = [
        limit(
            LimitKind::Session,
            "Session",
            session,
            TimeDelta::minutes(141),
            TimeDelta::hours(5),
        ),
        limit(
            LimitKind::Weekly,
            "Weekly",
            weekly,
            TimeDelta::hours(74),
            TimeDelta::days(7),
        ),
    ];
    let clock = ClockFormat {
        pattern: "h:mm tt".to_string(),
        am: "AM".to_string(),
        pm: "PM".to_string(),
    };
    let model = FlyoutModel::from(
        &limits,
        now,
        &Zone::utc(),
        &clock,
        &FlyoutOptions::default(),
    );
    [&model.session, &model.weekly].map(|view| {
        let fill = GlyphFill::from(view);
        (
            fill.left_cells,
            fill.soon_cells,
            fill.left_percent,
            fill.soon_percent,
        )
    })
}

/// A colour at 35% over another: how a faint cell or fill is drawn.
fn faint(colour: Rgb, over: Rgb) -> Rgb {
    let mix = |c: u8, o: u8| (f32::from(o) + (f32::from(c) - f32::from(o)) * 0.35).round() as u8;
    Rgb::new(
        mix(colour.r, over.r),
        mix(colour.g, over.g),
        mix(colour.b, over.b),
    )
}

/// The two columns' left edges in DIPs, and the first row's top under the heading and the 16.
const COLUMNS: [f32; 2] = [19.0, 19.0 + 152.0];
const GLYPHS_TOP: f32 = FIRST_ROW + 16.0;

#[test]
fn the_battery_has_the_cells_core_says() {
    // Used 15% and 45% (claude-only), 0% and 100% (empty-and-full).
    for (state, used) in [
        ("claude-only", fills(15.0, 45.0)),
        ("empty-and-full", fills(0.0, 100.0)),
    ] {
        for (theme, dark) in THEMES {
            let palette = NightPalette::for_dark(dark);
            for colour in COLOURS {
                for (scale, factor) in SCALES {
                    let card = styled(state, "battery", theme, colour, scale);
                    for (column, (left, soon, _, _)) in COLUMNS.into_iter().zip(used) {
                        // The bar colour, the warning colour past 70% used, the error colour from 90%.
                        let lit_colour = if left == 0 {
                            palette.error
                        } else {
                            fill(colour, dark)
                        };
                        let (mut lit, mut dim) = (0, 0);
                        for cell in 0..10 {
                            let at = card.at(
                                factor,
                                column + 15.0 + 11.0 * cell as f32,
                                GLYPHS_TOP + 20.0,
                            );
                            if near(at, lit_colour) {
                                lit += 1;
                            } else if near(at, faint(lit_colour, palette.ground)) {
                                dim += 1;
                            } else {
                                assert!(
                                    near(at, palette.track),
                                    "cell {cell} {state} {theme} {colour} {scale}x"
                                );
                            }
                        }
                        let label = format!("{state} {theme} {colour} {scale}x column at {column}");
                        assert_eq!(lit, left - soon, "lit cells {label}");
                        assert_eq!(dim, soon, "faint cells {label}");
                    }
                }
            }
        }
    }
}

#[test]
fn the_thermometer_has_the_fill_core_says() {
    for (state, used) in [
        ("claude-only", fills(15.0, 45.0)),
        ("empty-and-full", fills(0.0, 100.0)),
    ] {
        for (theme, dark) in THEMES {
            let palette = NightPalette::for_dark(dark);
            for colour in COLOURS {
                for (scale, factor) in SCALES {
                    let card = styled(state, "thermometer", theme, colour, scale);
                    for (column, (left, soon, left_percent, soon_percent)) in
                        COLUMNS.into_iter().zip(used)
                    {
                        let paint = if left == 0 {
                            palette.error
                        } else {
                            fill(colour, dark)
                        };
                        let label = format!("{state} {theme} {colour} {scale}x column at {column}");
                        // The bulb is centred on the tube: find the paint across the column there.
                        let bulb = GLYPHS_TOP + 93.0;
                        let xs: Vec<f32> = (0..140)
                            .map(|x| column + x as f32)
                            .filter(|&x| near(card.at(factor, x, bulb), paint))
                            .collect();
                        assert!(!xs.is_empty(), "bulb {label}");
                        let centre = (xs[0] + xs[xs.len() - 1] + 1.0) / 2.0;
                        let (mut solid, mut dim) = (0, 0);
                        for row in
                            (GLYPHS_TOP * factor) as usize..((GLYPHS_TOP + 90.0) * factor) as usize
                        {
                            let at = card.pixel((centre * factor) as usize, row);
                            if near(at, paint) {
                                solid += 1;
                            } else if near(at, faint(paint, palette.track)) {
                                dim += 1;
                            }
                        }
                        // The tube is 80 high from 4 down, and its solid part runs 6 on into the bulb.
                        let want_solid =
                            (6.0 + 0.8 * (left_percent - soon_percent)) as f32 * factor;
                        let want_dim = 0.8 * soon_percent as f32 * factor;
                        assert!(
                            (solid as f32 - want_solid).abs() <= 1.0,
                            "solid {label}: {solid} of {want_solid}"
                        );
                        assert!(
                            (dim as f32 - want_dim).abs() <= 2.0,
                            "faint {label}: {dim} of {want_dim}"
                        );
                        let _ = soon;
                    }
                }
            }
        }
    }
}
