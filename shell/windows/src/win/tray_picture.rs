//! The tray icon's picture: the rings drawn with GDI+, as 1.1 draws them, so the pixels match.
//! `draw` gives the pixels; `icon` makes an `HICON` from the same bitmap.
#![allow(unsafe_code)]

use sobreiro_core::colours::taskbar_colours::ICON_TRACK_ALPHA;
use sobreiro_core::colours::{Rgb, TaskbarColours};
use sobreiro_core::display::rings::{Ring, RingsModel};
use std::fmt;
use std::sync::OnceLock;
use windows::Win32::Graphics::GdiPlus::{
    BitmapData, GdipBitmapLockBits, GdipBitmapUnlockBits, GdipCreateBitmapFromScan0,
    GdipCreateHICONFromBitmap, GdipCreatePen1, GdipDeleteGraphics, GdipDeletePen, GdipDisposeImage,
    GdipDrawArc, GdipDrawEllipse, GdipGetImageGraphicsContext, GdipGraphicsClear, GdipSetPenEndCap,
    GdipSetPenStartCap, GdipSetPixelOffsetMode, GdipSetSmoothingMode, GdiplusStartup,
    GdiplusStartupInput, GdiplusStartupOutput, GpBitmap, GpGraphics, GpImage, GpPen,
    ImageLockModeRead, LineCapRound, PixelOffsetModeHalf, Rect, SmoothingModeAntiAlias, Status,
    UnitWorld,
};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

/// `PixelFormat32bppARGB`: 32 bits a pixel, straight alpha, bytes in memory as B, G, R, A.
const FORMAT_ARGB: i32 = 0x0026_200A;

/// What went wrong in a GDI+ call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureError(String);

impl fmt::Display for PictureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PictureError {}

/// A GDI+ status other than Ok as an error that names the call.
fn check(status: Status, call: &str) -> Result<(), PictureError> {
    if status == windows::Win32::Graphics::GdiPlus::Ok {
        Ok(())
    } else {
        Err(PictureError(format!("GDI+ {call} failed ({})", status.0)))
    }
}

/// GDI+ is started on the first picture and left running until the process ends.
fn start() -> Result<(), PictureError> {
    static STARTED: OnceLock<Result<(), PictureError>> = OnceLock::new();
    STARTED
        .get_or_init(|| {
            let input = GdiplusStartupInput {
                GdiplusVersion: 1,
                DebugEventCallback: 0,
                SuppressBackgroundThread: false.into(),
                SuppressExternalCodecs: false.into(),
            };
            let mut token = 0usize;
            let mut output = GdiplusStartupOutput::default();
            // SAFETY: the token, the input and the output are alive for the call; the background
            // thread is kept, so no output hook is needed.
            let status = unsafe { GdiplusStartup(&mut token, &input, &mut output) };
            check(status, "GdiplusStartup")
        })
        .clone()
}

/// A GDI+ bitmap, disposed on drop.
struct Bitmap(*mut GpBitmap);

impl Bitmap {
    fn new(size: u32) -> Result<Bitmap, PictureError> {
        let side = i32::try_from(size).map_err(|_| PictureError("size too large".into()))?;
        let mut bitmap: *mut GpBitmap = std::ptr::null_mut();
        // SAFETY: the out pointer is alive for the call; with no pixel memory given (a zero
        // stride and no scan0), GDI+ owns the pixels.
        let status =
            unsafe { GdipCreateBitmapFromScan0(side, side, 0, FORMAT_ARGB, None, &mut bitmap) };
        check(status, "GdipCreateBitmapFromScan0")?;
        Ok(Bitmap(bitmap))
    }
}

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: the bitmap was made by GDI+ and is disposed once, here.
        unsafe { GdipDisposeImage(self.0.cast::<GpImage>()) };
    }
}

/// A GDI+ graphics over a bitmap, deleted on drop.
struct Graphics(*mut GpGraphics);

impl Graphics {
    fn of(bitmap: &Bitmap) -> Result<Graphics, PictureError> {
        let mut graphics: *mut GpGraphics = std::ptr::null_mut();
        // SAFETY: the bitmap is alive; the out pointer is alive for the call.
        let status =
            unsafe { GdipGetImageGraphicsContext(bitmap.0.cast::<GpImage>(), &mut graphics) };
        check(status, "GdipGetImageGraphicsContext")?;
        Ok(Graphics(graphics))
    }
}

impl Drop for Graphics {
    fn drop(&mut self) {
        // SAFETY: the graphics was made by GDI+ and is deleted once, here.
        unsafe { GdipDeleteGraphics(self.0) };
    }
}

/// A GDI+ pen, deleted on drop.
struct Pen(*mut GpPen);

impl Pen {
    /// A pen of that colour (alpha, then the colour) and width, in world units.
    fn new(alpha: u8, colour: Rgb, width: f32) -> Result<Pen, PictureError> {
        let argb = u32::from(alpha) << 24
            | u32::from(colour.r) << 16
            | u32::from(colour.g) << 8
            | u32::from(colour.b);
        let mut pen: *mut GpPen = std::ptr::null_mut();
        // SAFETY: the out pointer is alive for the call.
        let status = unsafe { GdipCreatePen1(argb, width, UnitWorld, &mut pen) };
        check(status, "GdipCreatePen1")?;
        Ok(Pen(pen))
    }

    fn round_caps(&self) -> Result<(), PictureError> {
        // SAFETY: the pen is alive.
        check(
            unsafe { GdipSetPenStartCap(self.0, LineCapRound) },
            "GdipSetPenStartCap",
        )?;
        // SAFETY: the pen is alive.
        check(
            unsafe { GdipSetPenEndCap(self.0, LineCapRound) },
            "GdipSetPenEndCap",
        )
    }
}

impl Drop for Pen {
    fn drop(&mut self) {
        // SAFETY: the pen was made by GDI+ and is deleted once, here.
        unsafe { GdipDeletePen(self.0) };
    }
}

/// The icon's pixels: `size` by `size`, straight (not premultiplied) RGBA, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub size: u32,
    pub rgba: Vec<u8>,
}

/// The rings drawn into a new bitmap: the outer ring is the session, the inner the weekly (none
/// for Copilot's single ring); each fills clockwise from 12 o'clock.
fn paint(model: &RingsModel, colours: &TaskbarColours, size: u32) -> Result<Bitmap, PictureError> {
    start()?;
    let bitmap = Bitmap::new(size)?;
    let graphics = Graphics::of(&bitmap)?;
    // SAFETY: the graphics is alive for these calls. The colour is transparent white, as
    // `Color.Transparent` is.
    unsafe {
        check(
            GdipGraphicsClear(graphics.0, 0x00FF_FFFF),
            "GdipGraphicsClear",
        )?;
        check(
            GdipSetSmoothingMode(graphics.0, SmoothingModeAntiAlias),
            "GdipSetSmoothingMode",
        )?;
        check(
            GdipSetPixelOffsetMode(graphics.0, PixelOffsetModeHalf),
            "GdipSetPixelOffsetMode",
        )?;
    }
    let side = size as f32;
    let stroke = side * 5.0 / 32.0;
    let gap = (size / 16).max(1) as f32;
    let outer = side / 2.0 - stroke / 2.0;
    let centre = side / 2.0;
    draw_ring(&graphics, &model.outer, colours, centre, outer, stroke)?;
    if model.has_inner {
        draw_ring(
            &graphics,
            &model.inner,
            colours,
            centre,
            outer - stroke - gap,
            stroke,
        )?;
    }
    Ok(bitmap)
}

fn draw_ring(
    graphics: &Graphics,
    ring: &Ring,
    colours: &TaskbarColours,
    centre: f32,
    radius: f32,
    stroke: f32,
) -> Result<(), PictureError> {
    let (x, y, side) = (centre - radius, centre - radius, radius * 2.0);
    let track = Pen::new(ICON_TRACK_ALPHA, colours.ink, stroke)?;
    // SAFETY: the graphics and the pen are alive.
    check(
        unsafe { GdipDrawEllipse(graphics.0, track.0, x, y, side, side) },
        "GdipDrawEllipse",
    )?;

    let sweep = ring.capped_sweep(f64::from(radius), f64::from(stroke));
    // As the C#'s `if (sweep <= 0) return;`: not-a-number goes on.
    if sweep <= 0.0 {
        return Ok(());
    }
    let pen = Pen::new(255, colours.fill(ring.level), stroke)?;
    pen.round_caps()?;
    if sweep >= 360.0 {
        // SAFETY: the graphics and the pen are alive.
        check(
            unsafe { GdipDrawEllipse(graphics.0, pen.0, x, y, side, side) },
            "GdipDrawEllipse",
        )
    } else {
        // SAFETY: the graphics and the pen are alive.
        check(
            unsafe { GdipDrawArc(graphics.0, pen.0, x, y, side, side, -90.0, sweep as f32) },
            "GdipDrawArc",
        )
    }
}

/// The picture of the rings at `size` pixels square.
pub fn draw(
    model: &RingsModel,
    colours: &TaskbarColours,
    size: u32,
) -> Result<Picture, PictureError> {
    let bitmap = paint(model, colours, size)?;
    let side = i32::try_from(size).map_err(|_| PictureError("size too large".into()))?;
    let rect = Rect {
        X: 0,
        Y: 0,
        Width: side,
        Height: side,
    };
    let mut data = BitmapData::default();
    // SAFETY: the bitmap is alive; the rectangle and the data are alive for the call.
    let status = unsafe {
        GdipBitmapLockBits(
            bitmap.0,
            &rect,
            ImageLockModeRead.0 as u32,
            FORMAT_ARGB,
            &mut data,
        )
    };
    check(status, "GdipBitmapLockBits")?;
    let width = size as usize;
    let stride = data.Stride;
    let mut rgba = Vec::with_capacity(width * width * 4);
    // A null base or a stride too short for a row is not read.
    let mut failed = (data.Scan0.is_null() || (data.Stride.unsigned_abs() as usize) < width * 4)
        .then(|| PictureError("GDI+ gave no usable pixels".into()));
    let rows = if failed.is_some() { 0 } else { width };
    for row in 0..rows {
        let Some(offset) = isize::try_from(row)
            .ok()
            .and_then(|r| r.checked_mul(stride as isize))
        else {
            failed = Some(PictureError("bitmap too large".into()));
            break;
        };
        // SAFETY: the bitmap is locked; Scan0 is not null and the stride is at least a row, both
        // checked above. Each of its `size` rows holds `size` four-byte pixels from Scan0,
        // `Stride` bytes apart (which is negative for a bitmap stored bottom up).
        let line = unsafe {
            std::slice::from_raw_parts(data.Scan0.cast::<u8>().offset(offset), width * 4)
        };
        for pixel in line.as_chunks::<4>().0 {
            // Memory order is B, G, R, A.
            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    // SAFETY: the bitmap is locked with this data, and `line` is no longer used.
    let unlocked = check(
        unsafe { GdipBitmapUnlockBits(bitmap.0, &mut data) },
        "GdipBitmapUnlockBits",
    );
    if let Some(error) = failed {
        return Err(error);
    }
    unlocked?;
    Ok(Picture { size, rgba })
}

/// A tray icon: an `HICON` that is destroyed on drop.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tray icons use it from #49")
)]
pub struct TrayIcon(HICON);

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tray icons use it from #49")
)]
impl TrayIcon {
    pub fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        // SAFETY: the icon was made by GDI+ for this value alone and is destroyed once, here.
        // A failure leaves nothing to do.
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

/// The icon of the rings at `size` pixels square, made from the same bitmap as `draw`'s.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "the tray icons use it from #49")
)]
pub fn icon(
    model: &RingsModel,
    colours: &TaskbarColours,
    size: u32,
) -> Result<TrayIcon, PictureError> {
    let bitmap = paint(model, colours, size)?;
    let mut handle = HICON::default();
    // SAFETY: the bitmap is alive; the out pointer is alive for the call.
    let status = unsafe { GdipCreateHICONFromBitmap(bitmap.0, &mut handle) };
    check(status, "GdipCreateHICONFromBitmap")?;
    if handle.is_invalid() {
        return Err(PictureError("GDI+ gave no icon".into()));
    }
    Ok(TrayIcon(handle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sobreiro_core::colours::BarColourOption;
    use sobreiro_core::limits::UsageLevel;
    use sobreiro_core::limits::usage_level::for_percent;
    use windows::Win32::Graphics::Gdi::{BITMAP, DeleteObject, GetObjectW, HGDIOBJ};
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    const SIZES: [u32; 4] = [16, 20, 24, 32];

    fn ring(percent: i32) -> Ring {
        Ring {
            percent,
            level: for_percent(percent),
            has_data: true,
        }
    }

    fn claude(session: i32, weekly: i32) -> RingsModel {
        RingsModel {
            outer: ring(session),
            inner: ring(weekly),
            has_inner: true,
        }
    }

    fn themes() -> [TaskbarColours; 2] {
        [true, false]
            .map(|light| TaskbarColours::for_taskbar(light, false, BarColourOption::Purple, None))
    }

    fn pixel(picture: &Picture, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * picture.size + x) * 4) as usize;
        [
            picture.rgba[at],
            picture.rgba[at + 1],
            picture.rgba[at + 2],
            picture.rgba[at + 3],
        ]
    }

    fn near(a: u8, b: u8) -> bool {
        a.abs_diff(b) <= 3
    }

    fn is_colour(pixel: [u8; 4], colour: Rgb, alpha: u8) -> bool {
        near(pixel[0], colour.r)
            && near(pixel[1], colour.g)
            && near(pixel[2], colour.b)
            && near(pixel[3], alpha)
    }

    /// The pixel in the middle of the stroke of the ring at `radius`, on the centre row, on the
    /// right (`true`) or the left.
    fn on_ring(picture: &Picture, size: u32, radius: f32, right: bool) -> [u8; 4] {
        let centre = size as f32 / 2.0;
        let x = if right {
            centre + radius
        } else {
            centre - radius
        };
        pixel(picture, x.floor() as u32, size / 2)
    }

    fn radii(size: u32) -> (f32, f32) {
        let stroke = size as f32 * 5.0 / 32.0;
        let gap = (size / 16).max(1) as f32;
        let outer = size as f32 / 2.0 - stroke / 2.0;
        (outer, outer - stroke - gap)
    }

    #[test]
    fn the_corners_are_clear() {
        for colours in themes() {
            for size in SIZES {
                let picture = draw(&claude(50, 45), &colours, size).unwrap();
                let last = size - 1;
                for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
                    assert_eq!(pixel(&picture, x, y)[3], 0, "{size}px ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn a_half_ring_fills_the_right_and_leaves_the_track_on_the_left() {
        for colours in themes() {
            for size in SIZES {
                let picture = draw(&claude(50, 45), &colours, size).unwrap();
                let (outer, _) = radii(size);
                let fill = colours.fill(UsageLevel::Normal);
                let right = on_ring(&picture, size, outer, true);
                assert!(is_colour(right, fill, 255), "{size}px right {right:?}");
                let left = on_ring(&picture, size, outer, false);
                assert!(
                    is_colour(left, colours.ink, ICON_TRACK_ALPHA),
                    "{size}px left {left:?}"
                );
                // The arc starts at 12 o'clock.
                let stroke = size as f32 * 5.0 / 32.0;
                let (_, inner) = radii(size);
                let right = on_ring(&picture, size, inner, true);
                assert!(
                    is_colour(right, fill, 255),
                    "{size}px inner right {right:?}"
                );
                let left = on_ring(&picture, size, inner, false);
                assert!(
                    is_colour(left, colours.ink, ICON_TRACK_ALPHA),
                    "{size}px inner left {left:?}"
                );
                let top = pixel(&picture, size / 2, (stroke / 2.0).floor() as u32);
                assert!(is_colour(top, fill, 255), "{size}px top {top:?}");
            }
        }
    }

    #[test]
    fn the_fill_takes_the_colour_of_its_level() {
        for colours in themes() {
            for size in SIZES {
                let (outer, _) = radii(size);
                for (percent, level) in [(75, UsageLevel::Warning), (95, UsageLevel::Error)] {
                    let picture = draw(&claude(percent, 10), &colours, size).unwrap();
                    let right = on_ring(&picture, size, outer, true);
                    assert!(
                        is_colour(right, colours.fill(level), 255),
                        "{size}px {percent}% {right:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_inner_ring_takes_the_colour_of_its_level() {
        for colours in themes() {
            for size in SIZES {
                let (_, inner) = radii(size);
                let picture = draw(&claude(10, 75), &colours, size).unwrap();
                let right = on_ring(&picture, size, inner, true);
                assert!(
                    is_colour(right, colours.fill(UsageLevel::Warning), 255),
                    "{size}px {right:?}"
                );
            }
        }
    }

    #[test]
    fn a_full_ring_is_a_whole_circle() {
        for colours in themes() {
            for size in SIZES {
                let picture = draw(&claude(100, 100), &colours, size).unwrap();
                let (outer, inner) = radii(size);
                let error = colours.fill(UsageLevel::Error);
                for radius in [outer, inner] {
                    let left = on_ring(&picture, size, radius, false);
                    assert!(is_colour(left, error, 255), "{size}px left {left:?}");
                }
                let centre = size as f32 / 2.0;
                let bottom = pixel(&picture, size / 2, (centre + outer).floor() as u32);
                assert!(is_colour(bottom, error, 255), "{size}px bottom {bottom:?}");
            }
        }
    }

    #[test]
    fn copilot_has_no_inner_ring() {
        for colours in themes() {
            for size in SIZES {
                let model = RingsModel {
                    outer: ring(17),
                    inner: Ring::NONE,
                    has_inner: false,
                };
                let picture = draw(&model, &colours, size).unwrap();
                let (outer, inner) = radii(size);
                assert_eq!(on_ring(&picture, size, inner, true)[3], 0, "{size}px");
                assert_eq!(on_ring(&picture, size, inner, false)[3], 0, "{size}px");
                let track = on_ring(&picture, size, outer, false);
                assert!(is_colour(track, colours.ink, ICON_TRACK_ALPHA), "{size}px");
            }
        }
    }

    #[test]
    fn no_readings_draw_the_two_tracks_only() {
        for colours in themes() {
            for size in SIZES {
                let model = RingsModel {
                    outer: Ring::NONE,
                    inner: Ring::NONE,
                    has_inner: true,
                };
                let picture = draw(&model, &colours, size).unwrap();
                assert!(
                    picture
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .all(|p| p[3] <= ICON_TRACK_ALPHA + 3),
                    "{size}px"
                );
                let (outer, inner) = radii(size);
                for radius in [outer, inner] {
                    for right in [true, false] {
                        let track = on_ring(&picture, size, radius, right);
                        assert!(
                            is_colour(track, colours.ink, ICON_TRACK_ALPHA),
                            "{size}px r{radius} {track:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn an_icon_is_the_size_asked_for_and_is_destroyed_on_drop() {
        let colours = TaskbarColours::for_taskbar(true, false, BarColourOption::Purple, None);
        for size in SIZES {
            let icon = icon(&claude(50, 45), &colours, size).unwrap();
            let handle = icon.handle();
            assert!(!handle.is_invalid());
            let mut info = ICONINFO::default();
            // SAFETY: the icon is alive; the info is alive for the call.
            unsafe { GetIconInfo(handle, &mut info) }.unwrap();
            let mut bitmap = BITMAP::default();
            // SAFETY: the colour bitmap came from GetIconInfo; the out struct is alive and its
            // size is passed.
            let written = unsafe {
                GetObjectW(
                    HGDIOBJ(info.hbmColor.0),
                    std::mem::size_of::<BITMAP>() as i32,
                    Some(std::ptr::from_mut(&mut bitmap).cast()),
                )
            };
            // SAFETY: GetIconInfo gives the caller both bitmaps to free.
            unsafe {
                let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
                let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            }
            assert!(written > 0, "{size}px");
            assert_eq!(bitmap.bmWidth, size as i32, "{size}px");
            assert_eq!(bitmap.bmHeight, size as i32, "{size}px");
            drop(icon);
            // SAFETY: only asks about a handle that is gone; a destroyed icon gives an error.
            assert!(unsafe { GetIconInfo(handle, &mut ICONINFO::default()) }.is_err());
        }
    }
}
