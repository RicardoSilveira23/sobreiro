//! SPIKE (shell candidate B): the tray icon's rings. The toolkit draws into windows only, so the rings are
//! worked out pixel by pixel here: 16 samples a pixel, each ring a track with an arc and its round caps.
use slint::{Rgba8Pixel, SharedPixelBuffer};

/// One ring: its radius to the middle of the stroke, the degrees to draw clockwise from 12 o'clock
/// (already shortened for the round caps), and the arc's colour.
pub struct Ring {
    pub radius: f32,
    pub sweep: f32,
    pub fill: [u8; 3],
}

const GRID: u32 = 4;

/// A square picture of `size` pixels, not premultiplied: every ring's track in `track` (red, green, blue, alpha)
/// and its arc on top.
pub fn render(size: u32, stroke: f32, track: [u8; 4], rings: &[Ring]) -> SharedPixelBuffer<Rgba8Pixel> {
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(size, size);
    let centre = size as f32 / 2.0;
    let half = stroke / 2.0;
    let samples = (GRID * GRID) as f32;
    for (index, pixel) in buffer.make_mut_slice().iter_mut().enumerate() {
        let (px, py) = ((index as u32 % size) as f32, (index as u32 / size) as f32);
        let mut sum = [0f32; 4];
        for sy in 0..GRID {
            for sx in 0..GRID {
                let x = px + (sx as f32 + 0.5) / GRID as f32 - centre;
                let y = py + (sy as f32 + 0.5) / GRID as f32 - centre;
                let mut colour = [0f32; 4];
                for ring in rings {
                    if ((x * x + y * y).sqrt() - ring.radius).abs() <= half {
                        colour = over(straight(track), colour);
                    }
                    if on_arc(x, y, ring, half) {
                        colour = over([ring.fill[0] as f32, ring.fill[1] as f32, ring.fill[2] as f32, 255.0], colour);
                    }
                }
                for (total, part) in sum.iter_mut().zip(colour) {
                    *total += part;
                }
            }
        }
        // The tray element wants colours that are not premultiplied.
        let alpha = sum[3] / samples;
        let straight = |part: f32| if alpha > 0.0 { (part / samples * 255.0 / alpha).round().min(255.0) as u8 } else { 0 };
        *pixel = Rgba8Pixel { r: straight(sum[0]), g: straight(sum[1]), b: straight(sum[2]), a: alpha.round() as u8 };
    }
    buffer
}

fn straight(c: [u8; 4]) -> [f32; 4] {
    [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32]
}

// Source over destination; the source is straight, the result premultiplied.
fn over(source: [f32; 4], under: [f32; 4]) -> [f32; 4] {
    let alpha = source[3] / 255.0;
    [
        source[0] * alpha + under[0] * (1.0 - alpha),
        source[1] * alpha + under[1] * (1.0 - alpha),
        source[2] * alpha + under[2] * (1.0 - alpha),
        source[3] + under[3] * (1.0 - alpha),
    ]
}

fn on_arc(x: f32, y: f32, ring: &Ring, half: f32) -> bool {
    if ring.sweep <= 0.0 {
        return false;
    }
    let in_band = ((x * x + y * y).sqrt() - ring.radius).abs() <= half;
    if ring.sweep >= 360.0 {
        return in_band;
    }
    // Degrees clockwise from 12 o'clock.
    let mut angle = x.atan2(-y).to_degrees();
    if angle < 0.0 {
        angle += 360.0;
    }
    if in_band && angle <= ring.sweep {
        return true;
    }
    let end = ring.sweep.to_radians();
    let caps = [(0.0, -ring.radius), (ring.radius * end.sin(), -ring.radius * end.cos())];
    caps.iter().any(|(cx, cy)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() <= half)
}
