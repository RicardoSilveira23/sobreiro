//! Ported from `tests/Sobreiro.Tests/AccentPaletteTests.cs`.
use sobreiro_core::colours::{AccentPalette, WINDOWS_BLUE};

const BLUE: [u8; 32] = [
    0x99, 0xEB, 0xFF, 0x00, 0x4C, 0xC2, 0xFF, 0x00, 0x00, 0x91, 0xF8, 0x00, 0x00, 0x78, 0xD4, 0x00,
    0x00, 0x67, 0xC0, 0x00, 0x00, 0x3E, 0x92, 0x00, 0x00, 0x1A, 0x68, 0x00, 0xF7, 0x63, 0x0C, 0x00,
];

#[test]
fn parses_seven_quads_in_order() {
    let p = AccentPalette::from_registry(Some(&BLUE)).unwrap();
    assert_eq!(p.light3.to_hex(), "#99EBFF");
    assert_eq!(p.light2.to_hex(), "#4CC2FF");
    assert_eq!(p.light1.to_hex(), "#0091F8");
    assert_eq!(p.accent.to_hex(), "#0078D4");
    assert_eq!(p.dark1.to_hex(), "#0067C0");
    assert_eq!(p.dark2.to_hex(), "#003E92");
    assert_eq!(p.dark3.to_hex(), "#001A68");
}

#[test]
fn windows_blue_is_the_default_blue_palette() {
    assert_eq!(
        AccentPalette::from_registry(Some(&BLUE)),
        Some(WINDOWS_BLUE)
    );
}

#[test]
fn null_or_short_bytes_give_none() {
    assert_eq!(AccentPalette::from_registry(None), None);
    assert_eq!(AccentPalette::from_registry(Some(&[0; 27])), None);
}

#[test]
fn exactly_twenty_eight_bytes_is_enough() {
    assert!(AccentPalette::from_registry(Some(&BLUE[..28])).is_some());
}

// The rows below were written out by the C# of 1.x, run on 2026-11-15 on made-up bytes and
// turned into Rust by a script, not typed.

/// A palette as such a row holds it: seven colours, Light3 to Dark3.
type Quads = [[u8; 3]; 7];

fn quads_of(palette: Option<AccentPalette>) -> Option<Quads> {
    palette.map(|p| {
        [
            p.light3, p.light2, p.light1, p.accent, p.dark1, p.dark2, p.dark3,
        ]
        .map(|c| [c.r, c.g, c.b])
    })
}

#[test]
fn from_registry_answers_as_the_csharp_did() {
    // The registry's bytes, and the palette read from them.
    let rows: &[(Option<&[u8]>, Option<Quads>)] = &[
        (None, None),
        (Some(&[]), None),
        (Some(&[1]), None),
        (
            Some(&[
                3, 40, 77, 114, 151, 188, 225, 6, 43, 80, 117, 154, 191, 228, 9, 46, 83, 120, 157,
                194, 231, 12, 49, 86, 123, 160, 197,
            ]),
            None,
        ),
        (
            Some(&[
                5, 42, 79, 116, 153, 190, 227, 8, 45, 82, 119, 156, 193, 230, 11, 48, 85, 122, 159,
                196, 233, 14, 51, 88, 125, 162, 199, 236,
            ]),
            Some([
                [5, 42, 79],
                [153, 190, 227],
                [45, 82, 119],
                [193, 230, 11],
                [85, 122, 159],
                [233, 14, 51],
                [125, 162, 199],
            ]),
        ),
        (
            Some(&[
                7, 44, 81, 118, 155, 192, 229, 10, 47, 84, 121, 158, 195, 232, 13, 50, 87, 124,
                161, 198, 235, 16, 53, 90, 127, 164, 201, 238, 19,
            ]),
            Some([
                [7, 44, 81],
                [155, 192, 229],
                [47, 84, 121],
                [195, 232, 13],
                [87, 124, 161],
                [235, 16, 53],
                [127, 164, 201],
            ]),
        ),
        (
            Some(&[
                9, 46, 83, 120, 157, 194, 231, 12, 49, 86, 123, 160, 197, 234, 15, 52, 89, 126,
                163, 200, 237, 18, 55, 92, 129, 166, 203, 240, 21, 58, 95, 132,
            ]),
            Some([
                [9, 46, 83],
                [157, 194, 231],
                [49, 86, 123],
                [197, 234, 15],
                [89, 126, 163],
                [237, 18, 55],
                [129, 166, 203],
            ]),
        ),
        (
            Some(&[
                11, 48, 85, 122, 159, 196, 233, 14, 51, 88, 125, 162, 199, 236, 17, 54, 91, 128,
                165, 202, 239, 20, 57, 94, 131, 168, 205, 242, 23, 60, 97, 134, 171, 208, 245, 26,
                63, 100, 137, 174,
            ]),
            Some([
                [11, 48, 85],
                [159, 196, 233],
                [51, 88, 125],
                [199, 236, 17],
                [91, 128, 165],
                [239, 20, 57],
                [131, 168, 205],
            ]),
        ),
        (
            Some(&[
                13, 50, 87, 124, 161, 198, 235, 16, 53, 90, 127, 164, 201, 238, 19, 56, 93, 130,
                167, 204, 241, 22, 59, 96, 133, 170, 207, 244, 25, 62, 99, 136, 173, 210, 247, 28,
                65, 102, 139, 176, 213, 250, 31, 68, 105, 142, 179, 216, 253, 34, 71, 108, 145,
                182, 219, 0, 37, 74, 111, 148, 185, 222, 3, 40, 77, 114, 151, 188, 225, 6, 43, 80,
                117, 154, 191, 228, 9, 46, 83, 120, 157, 194, 231, 12, 49, 86, 123, 160, 197, 234,
                15, 52, 89, 126, 163, 200, 237, 18, 55, 92,
            ]),
            Some([
                [13, 50, 87],
                [161, 198, 235],
                [53, 90, 127],
                [201, 238, 19],
                [93, 130, 167],
                [241, 22, 59],
                [133, 170, 207],
            ]),
        ),
        (
            Some(&[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
            Some([
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
            ]),
        ),
        (
            Some(&[
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            ]),
            Some([
                [255, 255, 255],
                [255, 255, 255],
                [255, 255, 255],
                [255, 255, 255],
                [255, 255, 255],
                [255, 255, 255],
                [255, 255, 255],
            ]),
        ),
        (
            Some(&[
                0, 37, 74, 111, 148, 185, 222, 3, 40, 77, 114, 151, 188, 225, 6, 43, 80, 117, 154,
                191, 228, 9, 46, 83, 120, 157, 194, 231, 12, 49, 86,
            ]),
            Some([
                [0, 37, 74],
                [148, 185, 222],
                [40, 77, 114],
                [188, 225, 6],
                [80, 117, 154],
                [228, 9, 46],
                [120, 157, 194],
            ]),
        ),
    ];
    for &(bytes, expected) in rows {
        assert_eq!(
            quads_of(AccentPalette::from_registry(bytes)),
            expected,
            "{bytes:?}"
        );
    }
}

#[test]
fn windows_blue_answers_as_the_csharp_did() {
    let rows: &[Option<Quads>] = &[Some([
        [153, 235, 255],
        [76, 194, 255],
        [0, 145, 248],
        [0, 120, 212],
        [0, 103, 192],
        [0, 62, 146],
        [0, 26, 104],
    ])];
    assert_eq!(quads_of(Some(WINDOWS_BLUE)), rows[0]);
}
