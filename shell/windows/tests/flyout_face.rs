//! The flyout's card, drawn by `Sobreiro.exe --flyout-pictures`: each state at the size 1.1's card
//! has, in the colours Core gives, with a row's label, figure and unit on one baseline and a far
//! `Ends` badge's resets-in text on a line of its own.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sobreiro_core::colours::bar_colour::pick;
use sobreiro_core::colours::{BarColourOption, NightPalette, Rgb, WINDOWS_BLUE};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");
const SCALES: [(&str, f32); 3] = [("1", 1.0), ("1.5", 1.5), ("2", 2.0)];
const STATES: [&str; 17] = [
    "claude-only",
    "copilot-only",
    "both",
    "with-spend",
    "no-reading",
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

fn name(state: &str, theme: &str, colour: &str, scale: &str) -> String {
    format!("flyout-{state}-{theme}-{colour}-{scale}x.png")
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
                    for (scale, _) in SCALES {
                        expected.insert(name(state, theme, colour, scale));
                    }
                }
            }
        }
        assert_eq!(expected.len(), STATES.len() * 2 * 2 * 3);
        assert_eq!(
            written, expected,
            "the run wrote other pictures than states x 2 x 2 x 3"
        );
        folder.join("pictures")
    })
}

fn picture(state: &str, theme: &str, colour: &str, scale: &str) -> Picture {
    decode(&pictures().join(name(state, theme, colour, scale)))
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
        for (theme, dark) in THEMES {
            let ground = NightPalette::for_dark(dark).ground;
            for (scale, factor) in SCALES {
                let card = picture(state, theme, "purple", scale);
                let from = (311.0 * factor).ceil() as usize + 1;
                let to = card.width - factor.ceil() as usize - 1;
                let corner = (11.0 * factor) as usize;
                let stray = card.differs((from, to), (corner, card.height - corner), ground, 8);
                assert!(!stray, "{state} {theme} {scale}x has ink right of the row");
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
