//! Raster edits (Phase 4b-1): Fill and the Gradient, painted as Compositor for Mac paints them
//! (`BrushStroke.paintCanvas`, BrushStroke.swift:645-675): over the layer's original pixels, clipped
//! to the canvas and the selection, at the edit's opacity (source-over, the colour's alpha times the
//! opacity times the selection's coverage). A layer's pixel grid first grows to cover the canvas
//! (`BrushStroke.init`, :153-160); a mask keeps its own grid. The layer's result is trimmed to the
//! pixels left, and a mask covering it follows it, white where the layer grew (`commitRasterEdit`,
//! EditorSession+Brush.swift:154-188; `expandMask`, BrushStroke.swift:858-866).
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Linear runs along the line; Radial spreads from the start with the end on its rim (Gradient.swift:9-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientShape { Linear, Radial }

/// A gradient to paint: its shape, its line in document pixels, its two stops as straight RGBA 0..1
/// (a mask reads the first channel as grey), and the edit's opacity, 0.01 to 1 (Gradient.swift:15-19).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientSpec { pub shape: GradientShape, pub start: Point, pub end: Point, pub from: [f64; 4], pub to: [f64; 4], pub opacity: f64 }

/// The shortest gradient line that paints anything (`GradientEdit.hasLine`, Gradient.swift:32).
pub const MIN_GRADIENT_LINE: f64 = 0.5;

impl GradientSpec {
    fn check(&self) -> Result<(), CommandError> {
        let finite = [self.start.x, self.start.y, self.end.x, self.end.y, self.opacity].iter().chain(&self.from).chain(&self.to).all(|v| v.is_finite());
        if !finite || !(0.01..=1.0).contains(&self.opacity) || self.from.iter().chain(&self.to).any(|v| !(0.0..=1.0).contains(v)) {
            return Err(CommandError::Argument("gradient settings out of range".into()));
        }
        if (self.end.x - self.start.x).hypot(self.end.y - self.start.y) < MIN_GRADIENT_LINE {
            return Err(CommandError::Argument("a gradient needs a line at least half a pixel long".into()));
        }
        Ok(())
    }
    /// Where `p` falls along the gradient, 0 at the start and 1 at the end, extended past both
    /// (`drawsBeforeStartLocation`, `drawsAfterEndLocation`, BrushStroke.swift:611-621).
    pub fn position(&self, p: Point) -> f64 {
        let (dx, dy) = (self.end.x - self.start.x, self.end.y - self.start.y);
        let t = match self.shape {
            GradientShape::Linear => ((p.x - self.start.x) * dx + (p.y - self.start.y) * dy) / (dx * dx + dy * dy),
            GradientShape::Radial => (p.x - self.start.x).hypot(p.y - self.start.y) / dx.hypot(dy),
        };
        t.clamp(0.0, 1.0)
    }
    /// The straight RGBA at position `t`: the two stops mixed.
    pub fn color_at(&self, t: f64) -> [f64; 4] { std::array::from_fn(|c| self.from[c] + (self.to[c] - self.from[c]) * t) }
}

/// What an edit paints: a solid colour (Fill), or a gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint { Fill([f64; 3]), Gradient(GradientSpec) }

impl Paint {
    fn check(&self) -> Result<(), CommandError> {
        match self {
            Paint::Fill(c) if c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) => Ok(()),
            Paint::Fill(_) => Err(CommandError::Argument("fill colour out of range".into())),
            Paint::Gradient(g) => g.check(),
        }
    }
}

/// The grid a layer's edit is painted on: `width` x `height` pixels that `transform` places, with the
/// layer's own pixels at (`x`, `y`) in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditGrid { pub width: u32, pub height: u32, pub x: u32, pub y: u32, pub transform: LayerTransform }

/// The grid for painting `layer`'s pixels: its own grid (its pixels, or its box rounded when it has
/// none) grown to cover the canvas as the layer maps it, rounded out to whole pixels
/// (`originalBounds.union(canvas.applying(inverted).integral)`, BrushStroke.swift:153-156).
pub fn image_grid(doc: &Document, layer: &Layer) -> Result<EditGrid, CommandError> {
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    let inverse = layer.transform.pixel_to_document(w, h).invert().ok_or_else(|| CommandError::Argument("the layer cannot be painted".into()))?;
    let corners = [(0.0, 0.0), (doc.width as f64, 0.0), (doc.width as f64, doc.height as f64), (0.0, doc.height as f64)].map(|(x, y)| inverse.apply(Point { x, y }));
    let x0 = corners.iter().map(|p| p.x).fold(0.0f64, f64::min).floor();
    let y0 = corners.iter().map(|p| p.y).fold(0.0f64, f64::min).floor();
    let x1 = corners.iter().map(|p| p.x).fold(w as f64, f64::max).ceil();
    let y1 = corners.iter().map(|p| p.y).fold(h as f64, f64::max).ceil();
    let (gw, gh) = (x1 - x0, y1 - y0);
    let others: u64 = doc.layers.iter().filter(|l| l.id != layer.id).filter_map(|l| l.pixels.as_ref()).map(|r| r.width as u64 * r.height as u64).sum();
    if gw > MAX_SIDE as f64 || gh > MAX_SIDE as f64 || (gw * gh) as u64 > MAX_PIXELS.saturating_sub(others) {
        return Err(CommandError::Project(ProjectError::TooLarge));
    }
    let (gw, gh, x, y) = (gw as u32, gh as u32, (-x0) as u32, (-y0) as u32);
    Ok(EditGrid { width: gw, height: gh, x, y, transform: ops::adjust::placed_like(&layer.transform, w, h, gw, gh, x as f64, y as f64) })
}

/// `layer`'s pixels placed on `grid`: transparent where it grew.
pub fn pixels_on(grid: &EditGrid, pixels: Option<&Raster>) -> Vec<u8> {
    let mut out = vec![0u8; grid.width as usize * grid.height as usize * 4];
    if let Some(p) = pixels {
        let row = p.width as usize * 4;
        for y in 0..p.height as usize {
            let at = ((y + grid.y as usize) * grid.width as usize + grid.x as usize) * 4;
            out[at..at + row].copy_from_slice(&p.bytes()[y * row..(y + 1) * row]);
        }
    }
    out
}

/// Where a document point falls along a gradient, 0 to 1, with the division done once.
enum Ramp { Solid, Linear { x: f64, y: f64, ux: f64, uy: f64 }, Radial { x: f64, y: f64, inverse: f64 } }

impl Ramp {
    fn of(paint: &Paint) -> Ramp {
        match paint {
            Paint::Fill(_) => Ramp::Solid,
            Paint::Gradient(g) => {
                let (dx, dy) = (g.end.x - g.start.x, g.end.y - g.start.y);
                let len2 = dx * dx + dy * dy;
                match g.shape {
                    GradientShape::Linear => Ramp::Linear { x: g.start.x, y: g.start.y, ux: dx / len2, uy: dy / len2 },
                    GradientShape::Radial => Ramp::Radial { x: g.start.x, y: g.start.y, inverse: 1.0 / len2.sqrt() },
                }
            }
        }
    }
    /// `GradientSpec::position` at (`x`, `y`).
    fn at(&self, x: f64, y: f64) -> f64 {
        match self {
            Ramp::Solid => 0.0,
            Ramp::Linear { x: sx, y: sy, ux, uy } => ((x - sx) * ux + (y - sy) * uy).clamp(0.0, 1.0),
            Ramp::Radial { x: sx, y: sy, inverse } => (((x - sx) * (x - sx) + (y - sy) * (y - sy)).sqrt() * inverse).min(1.0),
        }
    }
}

/// Paints `paint` over `data` (premultiplied RGBA, or grey when `grey`) on a `width` x `height` grid
/// that `transform` places: each pixel at its centre, inside the canvas, through `coverage` (the
/// selection's, None for all), source-over at the paint's alpha times its opacity times the coverage
/// (as a fraction of 255),
/// each channel rounded half up.
pub fn paint_grid(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, grey: bool) {
    let m = transform.pixel_to_document(width, height);
    let (cw, ch) = (doc.width as f64, doc.height as f64);
    let (w, channels) = (width as usize, if grey { 1 } else { 4 });
    let ramp = Ramp::of(paint);
    let (from, to, opacity) = match paint {
        Paint::Fill(c) => ([c[0], c[1], c[2], 1.0], [c[0], c[1], c[2], 1.0], 1.0),
        Paint::Gradient(g) => (g.from, g.to, g.opacity),
    };
    let delta: [f64; 4] = std::array::from_fn(|c| to[c] - from[c]);
    // The selection's coverage as a fraction, once for each of its 256 values.
    let fraction: [f64; 256] = std::array::from_fn(|k| k as f64 / 255.0);
    for y in 0..height as usize {
        // The row's first pixel centre in the document, and the step one pixel to the right.
        let first = m.apply(Point { x: 0.5, y: y as f64 + 0.5 });
        let cover = coverage.map(|c| &c.bytes()[y * w..(y + 1) * w]);
        let line = &mut data[y * w * channels..(y + 1) * w * channels];
        for (x, px) in line.chunks_exact_mut(channels).enumerate() {
            let k = cover.map_or(255, |c| c[x]);
            if k == 0 { continue; }
            let (dx, dy) = (first.x + m.a * x as f64, first.y + m.b * x as f64);
            if dx < 0.0 || dy < 0.0 || dx >= cw || dy >= ch { continue; }
            let t = ramp.at(dx, dy);
            let s = (from[3] + delta[3] * t) * opacity * fraction[k as usize];
            if s <= 0.0 { continue; }
            let keep = 1.0 - s;
            if grey {
                px[0] = ((from[0] + delta[0] * t) * 255.0 * s + px[0] as f64 * keep + 0.5) as u8;
            } else {
                for c in 0..3 { px[c] = ((from[c] + delta[c] * t) * 255.0 * s + px[c] as f64 * keep + 0.5) as u8; }
                px[3] = (255.0 * s + px[3] as f64 * keep + 0.5) as u8;
            }
        }
    }
}

/// A covering mask carried onto the layer's new, trimmed grid (`crop` of `grid`): the old mask
/// stretched over where the layer's own pixels were, white where it grew (`expandMask`,
/// BrushStroke.swift:858-866). A uniform white mask stays one pixel: it shows everything either way.
fn followed(mask: &GrayRaster, old: (u32, u32), grid: &EditGrid, crop: (u32, u32, u32, u32)) -> GrayRaster {
    if mask.is_uniform() == Some(255) { return mask.clone(); }
    let (w, h) = (crop.2 - crop.0, crop.3 - crop.1);
    let mut out = vec![255u8; w as usize * h as usize];
    for y in 0..h { for x in 0..w {
        let (lx, ly) = ((x + crop.0) as i64 - grid.x as i64, (y + crop.1) as i64 - grid.y as i64);
        if lx < 0 || ly < 0 || lx >= old.0 as i64 || ly >= old.1 as i64 { continue; }
        let mx = ((lx as u64 * mask.width as u64) / old.0 as u64) as u32;
        let my = ((ly as u64 * mask.height as u64) / old.1 as u64) as u32;
        out[(y * w + x) as usize] = mask.bytes()[(my * mask.width + mx) as usize];
    }}
    GrayRaster::from_bytes(w, h, out)
}

/// A layer that can be painted: not a folder (its mask can be), not an adjustment layer, an enabled
/// mask when the mask is the target (`canPaint`, EditorSession+Brush.swift:5-11); a selection with
/// something in it when there is one.
fn check_target(doc: &Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if doc.selection.as_ref().is_some_and(|s| s.is_empty()) { return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into())); }
    if mask {
        let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
        if !m.enabled { return Err(CommandError::Argument("the mask is turned off".into())); }
    } else {
        if layer.is_group { return Err(CommandError::Argument("folders have no pixels".into())); }
        if layer.is_adjustment() { return Err(CommandError::Argument("an adjustment layer has no pixels".into())); }
    }
    Ok(())
}

/// A covering mask brought onto the layer's own pixel grid (a mask on its own placement keeps its
/// grid): the Mac paints a mask in the grid it covers (BrushStroke.swift:148-152), so a 1 x 1 or
/// otherwise sized covering mask is stretched onto the layer's pixels first, nearest.
fn mask_on_layer_grid(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
    if m.placement.is_some() { return Ok(()); }
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    if (m.pixels.width, m.pixels.height) == (w, h) { return Ok(()); }
    let others = doc.used_mask_pixels() - m.pixels.width as u64 * m.pixels.height as u64;
    if (w as u64) * (h as u64) > MAX_PIXELS.saturating_sub(others) { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let src = m.pixels.clone();
    let data = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| {
        let (mx, my) = ((x as u64 * src.width as u64 / w as u64) as u32, (y as u64 * src.height as u64 / h as u64) as u32);
        src.bytes()[(my * src.width + mx) as usize]
    }).collect();
    doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
    Ok(())
}

/// Whether `paint_layer` would paint: the paint's values and the target (a preview asks first, and
/// shows nothing the commit would refuse).
pub fn paint_layer_check(doc: &Document, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
    paint.check()?;
    check_target(doc, id, mask)
}

/// Paints `paint` into layer `id`'s pixels (`mask` false) or its mask, as one edit: Fill and the
/// Gradient's commit.
pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
    paint_layer_check(doc, id, mask, paint)?;
    if mask {
        mask_on_layer_grid(doc, id)?;
        let layer = doc.layer(id).unwrap();
        let m = layer.mask.as_ref().unwrap();
        let grid = m.placement.unwrap_or(layer.transform);
        let (w, h) = (m.pixels.width, m.pixels.height);
        let coverage = ops::adjust::edit_coverage(doc, clips, &grid, w, h)?;
        let mut data = m.pixels.bytes().to_vec();
        paint_grid(doc, &mut data, w, h, &grid, coverage.as_ref(), paint, true);
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
        return Ok(());
    }
    let layer = doc.layer(id).unwrap().clone();
    let grid = image_grid(doc, &layer)?;
    let mut data = pixels_on(&grid, layer.pixels.as_ref());
    let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, grid.width, grid.height)?;
    paint_grid(doc, &mut data, grid.width, grid.height, &grid.transform, coverage.as_ref(), paint, false);
    let painted = Raster::from_premultiplied(grid.width, grid.height, data);
    // Trimmed to what is left; nothing left keeps the whole grid, as the Mac's `render` does.
    let crop = compositor::alpha_bounds(&painted).unwrap_or((0, 0, grid.width, grid.height));
    let (cw, ch) = (crop.2 - crop.0, crop.3 - crop.1);
    let (result, transform) = if (cw, ch) == (grid.width, grid.height) { (painted, grid.transform) } else {
        (painted.cropped(crop.0, crop.1, cw, ch), ops::adjust::placed_like(&grid.transform, grid.width, grid.height, cw, ch, -(crop.0 as f64), -(crop.1 as f64)))
    };
    // The layer's own grid before the edit (its box, rounded, when it had no pixels).
    let own = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    let same_grid = crop == (grid.x, grid.y, grid.x + own.0, grid.y + own.1);
    // A covering mask that must grow onto the new grid needs the room: every other path that
    // grows a mask checks it (masks.rs:13, :80; selection.rs:215; `mask_on_layer_grid` above), the
    // Mac shrinks its own paint limit by the other masks already held
    // (EditorSession+Brush.swift:22-25), and a project whose masks add up past MAX_PIXELS refuses
    // to reopen (package.rs:33-40). A uniform white mask stays 1 x 1 (`followed`'s own early
    // return), so it never needs the room.
    if let Some(m) = layer.mask.as_ref().filter(|m| m.placement.is_none() && !same_grid && m.pixels.is_uniform() != Some(255)) {
        let own_mask_pixels = m.pixels.width as u64 * m.pixels.height as u64;
        let others = doc.used_mask_pixels().saturating_sub(own_mask_pixels);
        if (cw as u64) * (ch as u64) > MAX_PIXELS.saturating_sub(others) {
            return Err(CommandError::Project(ProjectError::TooLarge));
        }
    }
    let target = doc.layer_mut(id).unwrap();
    target.set_pixels(Some(result));
    target.transform = if same_grid { layer.transform } else { transform };
    if let Some(m) = target.mask.as_ref().filter(|m| m.placement.is_none() && !same_grid) {
        let mask = followed(&m.pixels, own, &grid, crop);
        target.mask_mut().unwrap().pixels = mask;
    }
    Ok(())
}
