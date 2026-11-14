//! SPIKE (shell candidate B): turns the ported logic's models into what the .slint files show. No wording
//! and no arithmetic of its own beyond the rings' arcs.
use crate::{FlyoutCard, LimitRow, PillFace, icon};
use slint::{Color, Image, SharedString};
use sobreiro_core::colours::{NightPalette, Rgb, TaskbarColours};
use sobreiro_core::display::{FlyoutModel, FlyoutStyle, GlyphFill, LimitView, PillLine, PillModel, Ring, RingsModel};
use sobreiro_core::limits::UsageLevel;

const RINGS_SIZE: f64 = 26.0;
const STROKE: f64 = 3.0;
const RING_GAP: f64 = 2.0;

fn solid(c: Rgb) -> Color {
    Color::from_rgb_u8(c.r, c.g, c.b)
}

fn with_alpha(c: Rgb, alpha: u8) -> Color {
    Color::from_argb_u8(alpha, c.r, c.g, c.b)
}

fn line(line: &PillLine) -> crate::PillLine {
    crate::PillLine { label: line.label.as_str().into(), figure: line.figure.as_str().into(), stale: line.stale }
}

// A stopped block keeps its fill's shape and loses its colour: the ink at half strength.
fn fill(colours: &TaskbarColours, level: UsageLevel, stale: bool) -> Color {
    if stale { with_alpha(colours.ink, TaskbarColours::STALE_ALPHA) } else { solid(colours.fill(level)) }
}

/// The arc as an SVG path in the rings' 26 x 26 box, clockwise from 12 o'clock. Empty when nothing is drawn.
pub fn arc(ring: &Ring, radius: f64) -> SharedString {
    let centre = RINGS_SIZE / 2.0;
    let sweep = ring.capped_sweep(radius, STROKE);
    let top = centre - radius;
    if sweep <= 0.0 {
        return SharedString::default();
    }
    if sweep >= 360.0 {
        let bottom = centre + radius;
        return format!("M {centre} {top} A {radius} {radius} 0 1 1 {centre} {bottom} A {radius} {radius} 0 1 1 {centre} {top}").into();
    }
    let radians = sweep.to_radians();
    let (x, y) = (centre + radius * radians.sin(), centre - radius * radians.cos());
    let large = if sweep > 180.0 { 1 } else { 0 };
    format!("M {centre} {top} A {radius} {radius} 0 {large} 1 {x:.3} {y:.3}").into()
}

pub fn pill(model: &PillModel, colours: &TaskbarColours, tooltip: &str) -> PillFace {
    let outer = RINGS_SIZE / 2.0 - STROKE / 2.0;
    let inner = outer - STROKE - RING_GAP;
    let mut face = PillFace {
        show_claude: model.show_claude,
        session: line(&model.session),
        weekly: line(&model.weekly),
        outer_arc: arc(&model.rings.outer, outer),
        inner_arc: arc(&model.rings.inner, inner),
        outer_fill: fill(colours, model.rings.outer.level, model.claude_stale),
        inner_fill: fill(colours, model.rings.inner.level, model.claude_stale),
        ink: solid(colours.ink),
        tooltip: tooltip.into(),
        ..Default::default()
    };
    if let Some(copilot) = &model.copilot {
        face.has_copilot = true;
        face.credits = line(&copilot.credits);
        face.reset_date = copilot.reset_date.as_str().into();
        face.fraction = copilot.fraction as f32;
        face.bar_fill = fill(colours, copilot.level, copilot.stale);
    }
    face
}

/// The tray icon's rings at `size` pixels.
pub fn tray_icon(model: &RingsModel, colours: &TaskbarColours, size: u32) -> Image {
    let stroke = size as f64 * 5.0 / 32.0;
    let gap = (size / 16).max(1) as f64;
    let outer = size as f64 / 2.0 - stroke / 2.0;
    let mut radii = vec![(&model.outer, outer)];
    if model.has_inner {
        radii.push((&model.inner, outer - stroke - gap));
    }
    let rings: Vec<icon::Ring> = radii
        .into_iter()
        .map(|(ring, radius)| {
            let c = colours.fill(ring.level);
            icon::Ring { radius: radius as f32, sweep: ring.capped_sweep(radius, stroke) as f32, fill: [c.r, c.g, c.b] }
        })
        .collect();
    let ink = colours.ink;
    let track = [ink.r, ink.g, ink.b, TaskbarColours::ICON_TRACK_ALPHA];
    Image::from_rgba8_premultiplied(icon::render(size, stroke as f32, track, &rings))
}

fn row(view: &LimitView, palette: &NightPalette, normal: Rgb, note: Option<&str>) -> LimitRow {
    let glyph = GlyphFill::from(view);
    let badge = view.status.as_ref().map(|status| palette.badge(status.kind));
    LimitRow {
        label: view.label.as_str().into(),
        has_data: view.has_data,
        used_figure: view.used_figure.as_str().into(),
        left_figure: view.left_figure.as_str().into(),
        used_fraction: view.used_fraction as f32,
        has_projected: view.projected_fraction.is_some(),
        projected: view.projected_fraction.unwrap_or(0.0) as f32,
        level: solid(palette.level_colour(view.level, normal)),
        has_status: view.status.is_some(),
        status: view.status.as_ref().map(|s| s.text.as_str()).unwrap_or_default().into(),
        badge_fill: badge.as_ref().map(|b| solid(b.fill)).unwrap_or_default(),
        badge_ink: badge.as_ref().map(|b| solid(b.text)).unwrap_or_default(),
        resets_in: view.resets_in.as_deref().unwrap_or_default().into(),
        detail: view.detail.as_deref().unwrap_or_default().into(),
        note: note.unwrap_or_default().into(),
        left_cells: glyph.left_cells,
        soon_cells: glyph.soon_cells,
        left_percent: glyph.left_percent as f32,
        soon_percent: glyph.soon_percent as f32,
    }
}

pub fn flyout(model: &FlyoutModel, style: FlyoutStyle, palette: &NightPalette, normal: Rgb) -> FlyoutCard {
    let mut card = FlyoutCard {
        show_claude: model.show_claude,
        title: model.title.as_str().into(),
        style: match style {
            FlyoutStyle::Bars => 0,
            FlyoutStyle::Battery => 1,
            FlyoutStyle::Thermometer => 2,
        },
        session: row(&model.session, palette, normal, None),
        weekly: row(&model.weekly, palette, normal, None),
        footer: model.footer.as_str().into(),
        ground: solid(palette.ground),
        ink: solid(palette.ink),
        soft: solid(palette.soft),
        line: solid(palette.line),
        track: solid(palette.track),
        ..Default::default()
    };
    if let Some(spend) = &model.spend {
        card.has_spend = true;
        card.spend = row(spend, palette, normal, None);
    }
    if let Some(copilot) = &model.copilot {
        card.has_copilot = true;
        card.copilot = row(&copilot.credits, palette, normal, copilot.note.as_deref());
    }
    card
}
