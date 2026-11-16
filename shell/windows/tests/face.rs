//! The pill's face, drawn by `Sobreiro.exe --pill-pictures`: the full set of pictures, each at the
//! size 1.1's pill has (plus the figure box), in the colours Core gives, with every figure ending
//! where the others do.
// A test fails by panicking, in the helpers below as in the tests themselves.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sobreiro_core::colours::{BarColourOption, Rgb, TaskbarColours, WINDOWS_BLUE};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const EXE: &str = env!("CARGO_BIN_EXE_Sobreiro");

const STATES: [&str; 11] = [
    "fresh",
    "amber",
    "red",
    "stale",
    "no-reading",
    "copilot-on",
    "copilot-only",
    "digits-11",
    "digits-88",
    "levels",
    "stale-claude",
];
const THEMES: [(&str, bool); 2] = [("light", true), ("dark", false)];
const COLOURS: [&str; 2] = ["purple", "accent"];
const SCALES: [(&str, f32); 3] = [("1", 1.0), ("1.5", 1.5), ("2", 2.0)];

/// The pill's width in pixels in each state at 100%, 150% and 200%, from 1.1's own layout at those
/// scales (`--pill-pictures` of the 1.x app). 1.1 sets a line as one text and rounds its width once
/// to the nearest device pixel, so a figure is as wide as its digits and a block as wide as its
/// widest line; the widths do not scale linearly, hence the three.
const WIDTHS: [(&str, [usize; 3]); 11] = [
    ("fresh", [99, 149, 199]),
    ("amber", [99, 148, 198]),
    ("red", [100, 150, 200]),
    ("stale", [179, 270, 359]),
    ("no-reading", [75, 112, 150]),
    ("copilot-on", [167, 252, 335]),
    ("copilot-only", [72, 109, 144]),
    ("digits-11", [96, 145, 193]),
    ("digits-88", [101, 151, 202]),
    ("levels", [96, 145, 193]),
    ("stale-claude", [105, 158, 211]),
];
/// The pill's height in pixels at 100%, 150% and 200%: with Claude's block, and with Copilot's alone.
/// Claude's two lines are 15 DIPs tall each at 100%, Copilot's 15 and 13, with 4 of padding above
/// and below (at another scale a line is the font's line height rounded to whole pixels, as 1.1's
/// layout rounding gives: 22 and 20 px at 150%, 29 and 27 px at 200%).
const HEIGHTS: [(usize, usize); 3] = [(38, 36), (56, 54), (74, 72)];

/// A stopped fill is the ink at alpha 0x80, drawn over the track (the ink at 0x26), as 1.1 draws it:
/// 0x80 + 0x26 * (1 - 0x80 / 255) = 147.
const STALE_OVER_TRACK: u8 = 147;

struct Picture {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

impl Picture {
    fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let at = (y * self.width + x) * 4;
        [
            self.rgba[at],
            self.rgba[at + 1],
            self.rgba[at + 2],
            self.rgba[at + 3],
        ]
    }

    /// How many pixels in the rectangle (in DIPs, at `scale`) are `rgb` at `alpha`, give or take 4.
    fn count(&self, scale: f32, x: (f32, f32), y: (f32, f32), rgb: Rgb, alpha: u8) -> usize {
        let range = |(from, to): (f32, f32), limit: usize| {
            let from = (from * scale).floor() as usize;
            let to = ((to * scale).ceil() as usize).min(limit);
            from..to
        };
        let mut found = 0;
        for row in range(y, self.height) {
            for column in range(x, self.width) {
                let [r, g, b, a] = self.pixel(column, row);
                // A faint pixel is stored premultiplied in 8 bits, so its colour is coarser.
                let room = 4 + 255 / u8::max(alpha, 1);
                let near = |have: u8, want: u8, room: u8| have.abs_diff(want) <= room;
                if near(r, rgb.r, room)
                    && near(g, rgb.g, room)
                    && near(b, rgb.b, room)
                    && near(a, alpha, 4)
                {
                    found += 1;
                }
            }
        }
        found
    }
}

fn decode(path: &Path) -> Picture {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    Picture {
        width: info.width as usize,
        height: info.height as usize,
        rgba,
    }
}

fn name(state: &str, theme: &str, colour: &str, scale: &str) -> String {
    format!("pill-{state}-{theme}-{colour}-{scale}x.png")
}

/// Runs a copy of the exe that sits alone in a new folder, with a data folder and a Claude config
/// folder of its own, and returns the folder it wrote the pictures to. Once for all the tests.
fn pictures() -> &'static Path {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    FOLDER.get_or_init(|| {
        let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("face");
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let exe = folder.join("Sobreiro.exe");
        std::fs::copy(EXE, &exe).unwrap();
        let output = Command::new(&exe)
            .arg("--pill-pictures")
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
        let left: BTreeSet<_> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            left,
            BTreeSet::from(["Sobreiro.exe".to_string(), "pictures".to_string()]),
            "the run left files beside the exe"
        );
        folder.join("pictures")
    })
}

fn picture(state: &str, theme: &str, colour: &str, scale: &str) -> Picture {
    decode(&pictures().join(name(state, theme, colour, scale)))
}

fn colours(light: bool, colour: &str) -> TaskbarColours {
    match colour {
        "purple" => TaskbarColours::for_taskbar(light, false, BarColourOption::Purple, None),
        _ => {
            TaskbarColours::for_taskbar(light, false, BarColourOption::Accent, Some(&WINDOWS_BLUE))
        }
    }
}

#[test]
fn it_writes_every_state_in_both_themes_both_colours_and_three_scales() {
    let mut expected = BTreeSet::new();
    for state in STATES {
        for (theme, _) in THEMES {
            for colour in COLOURS {
                for (scale, _) in SCALES {
                    expected.insert(name(state, theme, colour, scale));
                }
            }
        }
    }
    assert_eq!(expected.len(), 11 * 2 * 2 * 3);
    let written: BTreeSet<_> = std::fs::read_dir(pictures())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(written, expected);
}

#[test]
fn each_picture_is_the_size_of_1_1s_pill_at_that_scale() {
    for (state, widths) in WIDTHS {
        for (theme, _) in THEMES {
            for colour in COLOURS {
                for (at, (label, _)) in SCALES.into_iter().enumerate() {
                    let height = if state == "copilot-only" {
                        HEIGHTS[at].1
                    } else {
                        HEIGHTS[at].0
                    };
                    let picture = picture(state, theme, colour, label);
                    let name = name(state, theme, colour, label);
                    assert_eq!(
                        picture.width, widths[at],
                        "{name} is {} wide, 1.1's is {}",
                        picture.width, widths[at]
                    );
                    assert_eq!(
                        picture.height, height,
                        "{name} is {} tall, 1.1's is {height}",
                        picture.height
                    );
                }
            }
        }
    }
}

#[test]
fn a_label_and_its_figure_share_a_baseline_at_every_scale() {
    // "5h" and "15%" have no descenders, so the bottom row of ink in each is the baseline (the
    // label is the first 10 DIPs of ink in the line, the figure the 18 DIPs before 6 DIPs of the
    // text's end, which lies 42 DIPs from the right edge). The rows are the same as 1.1's.
    let bottom = |picture: &Picture, columns: (usize, usize), rows: (usize, usize)| {
        (rows.0..rows.1)
            .rev()
            .find(|&y| (columns.0..columns.1).any(|x| picture.pixel(x, y)[3] > 110))
            .unwrap()
    };
    // The rows of 1.1's two lines at each scale, and the baseline of each.
    let lines = [
        [((0, 19), 15), ((19, 38), 30)],
        [((0, 28), 22), ((28, 56), 44)],
        [((0, 37), 30), ((37, 74), 59)],
    ];
    for (theme, _) in THEMES {
        for (at, (label, scale)) in SCALES.into_iter().enumerate() {
            let picture = picture("fresh", theme, "purple", label);
            let end = picture.width - (42.0 * scale) as usize;
            for (rows, baseline) in lines[at] {
                let start = (0..end)
                    .find(|&x| (rows.0..rows.1).any(|y| picture.pixel(x, y)[3] > 110))
                    .unwrap();
                let label_columns = (start, start + (10.0 * scale) as usize);
                let figure_columns = (end - (24.0 * scale) as usize, end - (6.0 * scale) as usize);
                let (a, b) = (
                    bottom(&picture, label_columns, rows),
                    bottom(&picture, figure_columns, rows),
                );
                assert_eq!(
                    a, b,
                    "{theme} {label}x: the label ends on row {a}, the figure on {b}"
                );
                assert_eq!(
                    a, baseline,
                    "{theme} {label}x: 1.1's baseline is row {baseline}"
                );
            }
        }
    }
}

/// The summed alpha of the text column in nine states, in the purple colour, at 100%, 150% and 200%,
/// from 1.1's own pictures (`--pill-pictures` of the 1.x app), the same in both themes: per pixel
/// the alpha above the ground's, over the columns before the rings (before the bar for
/// copilot-only). Before the second pass 2.0 had 0.88, 0.85 and 0.96 of it.
const INK_1_1: [(&str, [u64; 3]); 9] = [
    ("fresh", [94_563, 208_935, 345_049]),
    ("amber", [93_755, 206_407, 340_189]),
    ("red", [97_497, 214_535, 354_615]),
    ("no-reading", [27_740, 59_140, 100_088]),
    ("digits-11", [83_775, 185_449, 307_666]),
    ("digits-88", [99_812, 220_568, 365_352]),
    ("levels", [90_784, 199_983, 331_422]),
    ("stale-claude", [100_174, 220_382, 366_569]),
    ("copilot-only", [49_448, 109_240, 182_527]),
];

/// The summed alpha of a picture's text column, as `INK_1_1` measures 1.1's.
fn text_ink(picture: &Picture, state: &str, scale: f32) -> u64 {
    let rings = if state == "copilot-only" { 13.0 } else { 34.0 };
    let end = picture.width - (rings * scale).round() as usize;
    let ground = picture.pixel((4.0 * scale).floor() as usize, picture.height / 2)[3];
    let mut sum = 0;
    for y in 0..picture.height {
        for x in 0..end {
            sum += u64::from(picture.pixel(x, y)[3].saturating_sub(ground));
        }
    }
    sum
}

#[test]
fn the_text_carries_1_1s_ink_in_every_state_at_every_scale() {
    // Each state within 3%: the labels and the figures have second passes of their own, so a
    // thin text (the dash, a label) is held as close as a bold one.
    for (theme, _) in THEMES {
        for (at, (label, scale)) in SCALES.into_iter().enumerate() {
            let (mut have, mut want) = (0, 0);
            for (state, ink) in INK_1_1 {
                let sum = text_ink(&picture(state, theme, "purple", label), state, scale);
                let ratio = sum as f64 / ink[at] as f64;
                assert!(
                    (0.97..=1.03).contains(&ratio),
                    "{state} {theme} {label}x: 2.0's text ink is {ratio:.3} of 1.1's"
                );
                have += sum;
                want += ink[at];
            }
            let ratio = have as f64 / want as f64;
            println!("{theme} {label}x: 2.0's text ink is {ratio:.3} of 1.1's");
            assert!(
                (0.98..=1.02).contains(&ratio),
                "{theme} {label}x: 2.0's text ink is {ratio:.3} of 1.1's"
            );
        }
    }
}

/// The ring canvas of a Claude-only picture: its left and top in pixels, and its size, at a scale.
/// The canvas is the last 26 DIPs before 8 of padding, centred between the 4 DIPs of padding above
/// and below (6 DIPs down at 100%; 8.5 px at 150%, where the two lines are 44 px and the rings 39).
fn canvas(picture: &Picture, scale: f32) -> (f32, f32, f32) {
    (
        picture.width as f32 - (8.0 + 26.0) * scale,
        (picture.height as f32 - 26.0 * scale) / 2.0,
        26.0 * scale,
    )
}

/// What a ring's fill covers in the canvas: the rows and columns it reaches and its pixel count.
#[derive(Debug)]
struct Extent {
    rows: (usize, usize),
    columns: (usize, usize),
    count: usize,
}

/// The pixels of `rgb` (opaque) in the outer ring (`outer`) or the inner one, told apart by their
/// distance from the canvas centre: the outer centre line is 11.5 DIPs out, the inner one 6.5, so
/// 9 DIPs is between them.
fn extent(picture: &Picture, scale: f32, rgb: Rgb, outer: bool) -> Extent {
    let (left, top, size) = canvas(picture, scale);
    let mut found = Extent {
        rows: (usize::MAX, 0),
        columns: (usize::MAX, 0),
        count: 0,
    };
    for row in 0..size.round() as usize {
        for column in 0..size.round() as usize {
            let (x, y) = (
                (left.round() as usize) + column,
                (top.round() as usize) + row,
            );
            let [r, g, b, a] = picture.pixel(x, y);
            let near = |have: u8, want: u8| have.abs_diff(want) <= 6;
            if a < 200 || !(near(r, rgb.r) && near(g, rgb.g) && near(b, rgb.b)) {
                continue;
            }
            let (dx, dy) = (
                column as f32 + 0.5 - size / 2.0,
                row as f32 + 0.5 - size / 2.0,
            );
            if ((dx * dx + dy * dy).sqrt() > 9.0 * scale) != outer {
                continue;
            }
            found.rows = (found.rows.0.min(row), found.rows.1.max(row));
            found.columns = (found.columns.0.min(column), found.columns.1.max(column));
            found.count += 1;
        }
    }
    found
}

/// The pixel on a ring's centre line, `degrees` clockwise from 12 o'clock, of a Claude-only picture.
fn on_ring(picture: &Picture, scale: f32, radius: f32, degrees: f32) -> [u8; 4] {
    let (left, top, size) = canvas(picture, scale);
    let angle = degrees.to_radians();
    let x = left + size / 2.0 + radius * scale * angle.sin();
    let y = top + size / 2.0 - radius * scale * angle.cos();
    picture.pixel(x as usize, y as usize)
}

const OUTER_RADIUS: f32 = 11.5;
const INNER_RADIUS: f32 = 6.5;

fn is(pixel: [u8; 4], rgb: Rgb, alpha: u8) -> bool {
    let room = 4 + 255 / u8::max(alpha, 1);
    pixel[..3]
        .iter()
        .zip([rgb.r, rgb.g, rgb.b])
        .all(|(have, want)| have.abs_diff(want) <= room)
        && pixel[3].abs_diff(alpha) <= 4
}

#[test]
fn the_ring_fills_lie_where_1_1_draws_them() {
    // 1.1's stills at 100% and 200%, at 15% and 45%: the rows and columns each fill reaches, and
    // how many pixels it covers, within a pixel and 12% (and 4 pixels).
    for (theme, light) in THEMES {
        let base = colours(light, "purple").base;
        for (label, scale) in [("1", 1.0), ("2", 2.0)] {
            let still = decode(
                &Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../media/stills/pill")
                    .join(format!("claude-{theme}@{label}x.png")),
            );
            let ours = picture("fresh", theme, "purple", label);
            for outer in [true, false] {
                let (want, have) = (
                    extent(&still, scale, base, outer),
                    extent(&ours, scale, base, outer),
                );
                let which = if outer { "outer" } else { "inner" };
                eprintln!("{theme} {label}x {which}: 1.1 {want:?}, 2.0 {have:?}");
                let near = |a: (usize, usize), b: (usize, usize)| {
                    a.0.abs_diff(b.0) <= 1 && a.1.abs_diff(b.1) <= 1
                };
                assert!(
                    near(want.rows, have.rows) && near(want.columns, have.columns),
                    "{theme} {label}x {which}: 1.1 reaches {want:?}, 2.0 {have:?}"
                );
                let (a, b) = (want.count as f32, have.count as f32);
                assert!(
                    (a - b).abs() <= a * 0.12 + 4.0,
                    "{theme} {label}x {which}: 1.1 covers {a} pixels, 2.0 {b}"
                );
            }
        }
    }
}

#[test]
fn each_ring_fill_is_on_its_own_centre_line_in_its_own_level_colour() {
    // 15% outer and 91% inner in one picture, so a ring cannot pass for the other: the outer arc
    // ends near 54 degrees and the inner one near 327.
    for (theme, light) in THEMES {
        for colour in COLOURS {
            let set = colours(light, colour);
            for (label, scale) in SCALES {
                let at = |state: &str| picture(state, theme, colour, label);
                let name = format!("{theme} {colour} {label}x");
                let levels = at("levels");
                assert!(
                    is(on_ring(&levels, scale, OUTER_RADIUS, 20.0), set.base, 255),
                    "{name} outer 20"
                );
                assert!(
                    is(on_ring(&levels, scale, OUTER_RADIUS, 120.0), set.ink, 0x26),
                    "{name} outer track"
                );
                assert!(
                    is(on_ring(&levels, scale, INNER_RADIUS, 20.0), set.error, 255),
                    "{name} inner 20"
                );
                assert!(
                    is(on_ring(&levels, scale, INNER_RADIUS, 200.0), set.error, 255),
                    "{name} inner 200"
                );
                assert!(
                    is(on_ring(&levels, scale, INNER_RADIUS, 332.0), set.ink, 0x26),
                    "{name} inner track"
                );
                // Both rings in the same level, a quarter round.
                let fresh = at("fresh");
                assert!(
                    is(on_ring(&fresh, scale, OUTER_RADIUS, 20.0), set.base, 255),
                    "{name} fresh outer"
                );
                assert!(
                    is(on_ring(&fresh, scale, INNER_RADIUS, 90.0), set.base, 255),
                    "{name} fresh inner"
                );
                assert!(
                    is(on_ring(&fresh, scale, INNER_RADIUS, 250.0), set.ink, 0x26),
                    "{name} fresh inner track"
                );
                for (state, level) in [("amber", set.warning), ("red", set.error)] {
                    let picture = at(state);
                    assert!(
                        is(on_ring(&picture, scale, OUTER_RADIUS, 90.0), level, 255),
                        "{name} {state} outer"
                    );
                    assert!(
                        is(on_ring(&picture, scale, INNER_RADIUS, 90.0), level, 255),
                        "{name} {state} inner"
                    );
                }
                // A stopped block: the ink at half strength over the track, in place of the level.
                let stale = at("stale-claude");
                assert!(
                    is(
                        on_ring(&stale, scale, OUTER_RADIUS, 20.0),
                        set.ink,
                        STALE_OVER_TRACK
                    ),
                    "{name} stale outer"
                );
                assert!(
                    is(
                        on_ring(&stale, scale, INNER_RADIUS, 90.0),
                        set.ink,
                        STALE_OVER_TRACK
                    ),
                    "{name} stale inner"
                );
            }
        }
    }
}

#[test]
fn text_and_the_ground_are_in_the_ink() {
    for (theme, light) in THEMES {
        let set = colours(light, "purple");
        for (label, scale) in SCALES {
            let found = picture("fresh", theme, "purple", label).count(
                scale,
                (0.0, 60.0),
                (4.0, 34.0),
                set.ink,
                255,
            );
            assert!(
                found >= 20,
                "{theme} {label}x: {found} pixels of ink in the text"
            );
        }
    }
}

#[test]
fn the_bar_fill_has_a_round_bottom_end() {
    // 1.1's bottom row at 100% is 29, 199, 255, 211, 41 across the five columns: the corners are
    // nearly clear. The toolkit's own clip would leave all five at 255.
    for (theme, _) in THEMES {
        // The bar's bottom row in 1.1's pictures: 31 at 100%, 62 at 200% (its pill is 74 tall).
        for (label, scale, row) in [("1", 1.0, 31), ("2", 2.0, 62)] {
            let picture = picture("copilot-on", theme, "purple", label);
            let from = picture.width - (14.0 * scale) as usize;
            let alphas: Vec<u8> = (from..picture.width)
                .map(|x| picture.pixel(x, row)[3])
                .collect();
            let bar: Vec<u8> = alphas.iter().copied().skip_while(|&a| a < 12).collect();
            let bar: Vec<u8> = bar.iter().copied().take_while(|&a| a >= 12).collect();
            let middle = bar[bar.len() / 2];
            assert!(
                middle >= 190,
                "{theme} {label}x: the middle of the bottom row is {middle}: {alphas:?}"
            );
            assert!(
                u16::from(bar[0]) * 2 < u16::from(middle)
                    && u16::from(bar[bar.len() - 1]) * 2 < u16::from(middle),
                "{theme} {label}x: the corners are not rounded: {bar:?}"
            );
        }
    }
}

#[test]
fn copilots_bar_fills_from_the_bottom_in_the_level_colour_and_goes_grey_when_stale() {
    for (theme, light) in THEMES {
        for colour in COLOURS {
            let set = colours(light, colour);
            // The bar is the last 5 DIPs before the 8 of padding on the right; at 17% its fill is
            // the lowest 4.4 of its 26 DIPs, which end 6 DIPs above the bottom edge of the pill.
            for (label, scale) in SCALES {
                let on = picture("copilot-on", theme, colour, label);
                let right = on.width as f32 / scale;
                let low = (right - 12.0, right - 9.0);
                let found = on.count(scale, low, (29.0, 31.0), set.base, 255);
                assert!(found >= 2, "copilot-on {theme} {colour} {label}x: {found}");
                let high = on.count(
                    scale,
                    (right - 12.0, right - 9.0),
                    (7.0, 9.0),
                    set.ink,
                    0x26,
                );
                assert!(high >= 2, "the bar's track shows above its fill: {high}");

                let stale = picture("stale", theme, colour, label);
                let right = stale.width as f32 / scale;
                let found = stale.count(
                    scale,
                    (right - 12.0, right - 9.0),
                    (29.0, 31.0),
                    set.ink,
                    STALE_OVER_TRACK,
                );
                assert!(found >= 2, "stale {theme} {colour} {label}x: {found}");
            }
        }
    }
}

#[test]
fn the_ground_is_nearly_clear_and_the_corners_are_round() {
    let picture = picture("fresh", "dark", "purple", "2");
    // At rest the ground is the ink at alpha 1 and outside the border radius there is none.
    assert_eq!(picture.pixel(20, picture.height / 2)[3], 1);
    assert_eq!(picture.pixel(0, 0)[3], 0);
    assert_eq!(picture.pixel(picture.width - 1, picture.height - 1)[3], 0);
}

#[test]
fn a_line_with_11_percent_and_one_with_88_percent_end_in_one_column_and_the_pills_differ_by_their_figures()
 {
    // Each line is right-aligned in its column, so the percent sign is in the same columns whatever
    // the digits are, and the pill is as wide as its figures. At 200%, the weekly line is the second
    // of two 29 px lines under 8 px of padding: rows 37 to 66. Its text ends 42 DIPs before the
    // right edge (8 of padding, the rings' 26, 8 between).
    let scale = 2.0;
    let profile = |state: &str| {
        let picture = picture(state, "light", "purple", "2");
        let rows = 37..66;
        let end = picture.width - (42.0 * scale) as usize;
        let ink: Vec<bool> = (0..end)
            .map(|x| rows.clone().any(|y| picture.pixel(x, y)[3] > 64))
            .collect();
        (picture.width, ink)
    };
    let (width_11, ink_11) = profile("digits-11");
    let (width_88, ink_88) = profile("digits-88");
    assert!(
        width_11 < width_88,
        "11% is narrower than 88%: {width_11} and {width_88}"
    );
    let last = |ink: &[bool]| ink.iter().rposition(|&on| on).unwrap();
    assert_eq!(last(&ink_11), ink_11.len() - 1, "the line's end moves");
    assert_eq!(last(&ink_88), ink_88.len() - 1, "the line's end moves");
    // The percent sign is the last 10 DIPs of ink: the same columns in both.
    let sign = (10.0 * scale) as usize;
    let tail = |ink: &[bool]| ink[ink.len() - sign..].to_vec();
    assert_eq!(tail(&ink_11), tail(&ink_88), "the percent sign moves");
    // And the figure is not empty: it has ink between the label and the sign.
    assert!(ink_11.iter().filter(|&&on| on).count() > sign);
}

#[test]
fn the_rings_lie_on_whole_pixels_whatever_the_figures_say() {
    // The text column is rounded once to a device pixel, so the rings' canvas (the last 26 DIPs
    // before 8 of padding) starts on a whole pixel: on the middle row the outer track is then as
    // strong in its leftmost column as in its rightmost. A canvas half a pixel off leaves one end
    // at about half. In these states the outer fill stops short of 3 o'clock, so both ends are track.
    for state in [
        "fresh",
        "stale-claude",
        "digits-11",
        "digits-88",
        "levels",
        "no-reading",
    ] {
        for (label, scale) in SCALES {
            let picture = picture(state, "dark", "purple", label);
            let size = (26.0 * scale) as usize;
            let left = picture.width - (8.0 * scale) as usize - size;
            let row = picture.height / 2;
            let (first, last) = (
                picture.pixel(left, row)[3],
                picture.pixel(left + size - 1, row)[3],
            );
            assert!(first > 16, "{state} {label}x: no ring at column {left}");
            assert!(
                first.abs_diff(last) <= 4,
                "{state} {label}x: the track is {first} at its left end and {last} at its right"
            );
            let gap = (4.0 * scale) as usize;
            assert!(
                (left - gap..left).all(|x| picture.pixel(x, row)[3] < 64),
                "{state} {label}x: ink just before the rings"
            );
        }
    }
}
