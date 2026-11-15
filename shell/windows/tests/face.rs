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

const STATES: [&str; 9] = [
    "fresh",
    "amber",
    "red",
    "stale",
    "no-reading",
    "copilot-on",
    "copilot-only",
    "digits-11",
    "digits-88",
];
const THEMES: [(&str, bool); 2] = [("light", true), ("dark", false)];
const COLOURS: [&str; 2] = ["purple", "accent"];
const SCALES: [(&str, f32); 3] = [("1", 1.0), ("1.5", 1.5), ("2", 2.0)];

/// The pill's size in DIPs for the states that 1.1 has stills of (`media/stills/pill/*@1x.png`,
/// "Calm" readings: 15% and 45%, Copilot at 17% resetting on 1 Dec): Claude's block alone is
/// 99 x 38, both blocks 167 x 38, Copilot's alone 72 x 36 (the @2x stills are exactly twice that).
/// Claude's two lines are 15 DIPs tall each, Copilot's 15 and 13, with 4 of padding above and below.
///
/// The heights are the same. Each figure is set in a box as wide as the widest of 00% to 99%, which
/// is wider than 1.1's figure for 15%, 45% and 17%, so each block is wider by the box's extra. The
/// extras were measured: the picture of each block alone, less 1.1's width of the same block, from
/// the pictures at 100% (Claude's 103 - 99, Copilot's 79 - 72). They hold at 150% and 200% within
/// the 1 px allowed, and are the box's width less the width of those figures, so they change when
/// the font or its size does.
const FRESH: (f32, f32) = (99.0, 38.0);
const BOTH: (f32, f32) = (167.0, 38.0);
const COPILOT_ONLY: (f32, f32) = (72.0, 36.0);
const CLAUDE_BOX_EXTRA: f32 = 4.0;
const COPILOT_BOX_EXTRA: f32 = 7.0;

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
    assert_eq!(expected.len(), 9 * 2 * 2 * 3);
    let written: BTreeSet<_> = std::fs::read_dir(pictures())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(written, expected);
}

#[test]
fn each_picture_is_the_size_of_1_1s_pill_at_that_scale() {
    let sizes = [
        ("fresh", FRESH, CLAUDE_BOX_EXTRA),
        ("amber", FRESH, CLAUDE_BOX_EXTRA),
        ("red", FRESH, CLAUDE_BOX_EXTRA),
        ("copilot-on", BOTH, CLAUDE_BOX_EXTRA + COPILOT_BOX_EXTRA),
        ("copilot-only", COPILOT_ONLY, COPILOT_BOX_EXTRA),
    ];
    for (state, (width, height), extra) in sizes {
        for (theme, _) in THEMES {
            for colour in COLOURS {
                for (label, scale) in SCALES {
                    let picture = picture(state, theme, colour, label);
                    let want = |dips: f32| (dips * scale).round() as isize;
                    let name = name(state, theme, colour, label);
                    let expected = want(width + extra);
                    assert!(
                        (picture.width as isize - expected).abs() <= 1,
                        "{name} is {} wide, 1.1's plus the box is {expected}",
                        picture.width
                    );
                    assert!(
                        (picture.height as isize - want(height)).abs() <= 1,
                        "{name} is {} tall, 1.1's is {}",
                        picture.height,
                        want(height)
                    );
                }
            }
        }
    }
}

#[test]
fn the_ring_fills_carry_the_level_colours_in_every_theme_and_bar_colour() {
    // The outer ring's stroke runs from 6 to 9 DIPs down from the top (4 of padding, 2 to centre
    // the 26 DIP rings in the 30 DIP text), the inner ring's from 11 to 14.
    let outer = (6.0, 9.0);
    let inner = (11.0, 14.0);
    let all_width = (0.0, 400.0);
    for (theme, light) in THEMES {
        for colour in COLOURS {
            let set = colours(light, colour);
            for (label, scale) in SCALES {
                let check = |state: &str, band: (f32, f32), want: Rgb, alpha: u8| {
                    let picture = picture(state, theme, colour, label);
                    let found = picture.count(scale, all_width, band, want, alpha);
                    assert!(
                        found >= scale.ceil() as usize,
                        "{}: {found} pixels of {} at alpha {alpha}",
                        name(state, theme, colour, label),
                        want.to_hex()
                    );
                };
                check("fresh", outer, set.base, 255);
                check("fresh", inner, set.base, 255);
                check("amber", outer, set.warning, 255);
                check("amber", inner, set.warning, 255);
                check("red", outer, set.error, 255);
                check("red", inner, set.error, 255);
                // A stopped block: the ink at half strength, in place of the level colour.
                check("stale", outer, set.ink, STALE_OVER_TRACK);
                check("stale", inner, set.ink, STALE_OVER_TRACK);
                // Text and tracks: the ink in full, and the track at 0x26.
                check("fresh", (0.0, 38.0), set.ink, 255);
                check("fresh", (0.0, 38.0), set.ink, 0x26);
            }
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
fn a_line_with_11_percent_and_one_with_88_percent_leave_everything_from_the_percent_sign_on_in_one_column()
 {
    // Each figure ends at the box's right edge, so only the digits differ. At 200%, the weekly line is the 30 DIPs of text's second 15: rows 19 to 34 DIPs. Its text
    // ends 42 DIPs before the right edge (8 of padding, the rings' 26, 8 between).
    let scale = 2.0;
    let profile = |state: &str| {
        let picture = picture(state, "light", "purple", "2");
        let rows = (19.0 * scale) as usize..(34.0 * scale) as usize;
        let columns = 0..picture.width - (42.0 * scale) as usize;
        let ink: Vec<bool> = columns
            .map(|x| rows.clone().any(|y| picture.pixel(x, y)[3] > 64))
            .collect();
        (picture.width, ink)
    };
    let (width_11, ink_11) = profile("digits-11");
    let (width_88, ink_88) = profile("digits-88");
    assert_eq!(width_11, width_88, "the pills are one width");
    let first = |ink: &[bool]| ink.iter().position(|&on| on).unwrap();
    let last = |ink: &[bool]| ink.iter().rposition(|&on| on).unwrap();
    assert_eq!(first(&ink_11), first(&ink_88), "the label moves");
    assert_eq!(last(&ink_11), last(&ink_88), "the line's end moves");
    // The percent sign is the last 10 DIPs of ink: the same columns in both.
    let sign = (10.0 * scale) as usize;
    let tail = |ink: &[bool]| ink[last(ink) + 1 - sign..=last(ink)].to_vec();
    assert_eq!(tail(&ink_11), tail(&ink_88), "the percent sign moves");
    // And the figure is not empty: it has ink between the label and the sign.
    assert!(ink_11.iter().filter(|&&on| on).count() > sign);
}
