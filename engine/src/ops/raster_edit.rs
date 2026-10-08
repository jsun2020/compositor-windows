//! Raster edits (Phase 4b-1): Fill and the Gradient, painted as Compositor for Mac paints them
//! (`BrushStroke.paintCanvas`, BrushStroke.swift:645-675): over the layer's original pixels, clipped
//! to the canvas and the selection, at the edit's opacity (source-over, the colour's alpha times the
//! opacity times the selection's coverage). A layer's pixel grid first grows to cover the canvas
//! (`BrushStroke.init`, :153-160). A mask is painted in its own grid (`mask_grid`), grown past its
//! layer to the canvas by a Fill or a Gradient as Compositor 1.3.7 grows it (Task 14a, the user's
//! decision of 2026-09-29). The layer's result is trimmed to the pixels left, and a mask covering it
//! follows it, white where the layer grew (`commitRasterEdit`, EditorSession+Brush.swift:154-188;
//! `expandMask`, BrushStroke.swift:858-866).
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

/// Snaps `v` to the nearest integer when it is within 1e-6 of one, else leaves it as it is. An inverse
/// transform can map an exact document edge to a mask- or layer-grid coordinate a few ulps off an
/// integer (e.g. -5.7e-13 instead of 0.0, or 700.0000000000001 instead of 700.0); left alone, `floor`
/// or `ceil` would round that to the wrong pixel and grow an already-grown grid by one more pixel on a
/// second, otherwise no-op edit (fix round 1, I-1).
pub(crate) fn snap_near_int(v: f64) -> f64 { let r = v.round(); if (v - r).abs() < 1e-6 { r } else { v } }

/// The grid for painting `layer`'s pixels: its own grid (its pixels, or its box rounded when it has
/// none) grown to cover the canvas as the layer maps it, rounded out to whole pixels
/// (`originalBounds.union(canvas.applying(inverted).integral)`, BrushStroke.swift:153-156).
pub fn image_grid(doc: &Document, layer: &Layer) -> Result<EditGrid, CommandError> {
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    let inverse = layer.transform.pixel_to_document(w, h).invert().ok_or_else(|| CommandError::Argument("the layer cannot be painted".into()))?;
    let corners = [(0.0, 0.0), (doc.width as f64, 0.0), (doc.width as f64, doc.height as f64), (0.0, doc.height as f64)].map(|(x, y)| inverse.apply(Point { x, y }));
    let x0 = snap_near_int(corners.iter().map(|p| p.x).fold(0.0f64, f64::min)).floor();
    let y0 = snap_near_int(corners.iter().map(|p| p.y).fold(0.0f64, f64::min)).floor();
    let x1 = snap_near_int(corners.iter().map(|p| p.x).fold(w as f64, f64::max)).ceil();
    let y1 = snap_near_int(corners.iter().map(|p| p.y).fold(h as f64, f64::max)).ceil();
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
}

/// Paints `paint` over `data` (premultiplied RGBA, or grey when `grey`) on a `width` x `height` grid
/// that `transform` places: each pixel at its centre, inside the canvas, through `coverage` (the
/// selection's, None for all), source-over at the paint's alpha times its opacity times the coverage
/// (as a fraction of 255),
/// each channel rounded half up. True when the edit reached any pixel: one inside the canvas that the
/// selection covers at all, whatever the paint's alpha there (the Mac's raster edit keeps a tile for
/// every part of the canvas and the selection's clip it touches, and a transparent end of a gradient
/// still touches it: `paintCanvas`, BrushStroke.swift:645-652).
pub fn paint_grid(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, grey: bool) -> bool {
    // Choose the pixel layout once, so the inner loop has a fixed stride and
    // does not branch between mask and RGBA painting for every pixel.
    if grey {
        paint_grid_channels::<1>(doc, data, width, height, transform, coverage, paint)
    } else {
        paint_grid_channels::<4>(doc, data, width, height, transform, coverage, paint)
    }
}

fn paint_grid_channels<const CHANNELS: usize>(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint) -> bool {
    // The ramp does not change during a stroke. Specialize it once instead of
    // inspecting its enum for every preview/commit pixel. Keep the original
    // Float64 expressions and rounding, including the radial path.
    match Ramp::of(paint) {
        Ramp::Solid => paint_grid_ramp::<CHANNELS, _>(doc, data, width, height, transform, coverage, paint, |_, _| 0.0),
        Ramp::Linear { x: sx, y: sy, ux, uy } => paint_grid_ramp::<CHANNELS, _>(doc, data, width, height, transform, coverage, paint,
            |x, y| ((x - sx) * ux + (y - sy) * uy).clamp(0.0, 1.0)),
        Ramp::Radial { x: sx, y: sy, inverse } => paint_grid_ramp::<CHANNELS, _>(doc, data, width, height, transform, coverage, paint,
            |x, y| (((x - sx) * (x - sx) + (y - sy) * (y - sy)).sqrt() * inverse).min(1.0)),
    }
}

fn paint_grid_ramp<const CHANNELS: usize, F: Fn(f64, f64) -> f64>(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, sample: F) -> bool {
    let opaque = match paint {
        Paint::Fill(_) => true,
        Paint::Gradient(g) => g.from[3] == 1.0 && g.to[3] == 1.0 && g.opacity == 1.0,
    };
    if opaque {
        paint_grid_coverage::<CHANNELS, true, F>(doc, data, width, height, transform, coverage, paint, sample)
    } else {
        paint_grid_coverage::<CHANNELS, false, F>(doc, data, width, height, transform, coverage, paint, sample)
    }
}

fn paint_grid_coverage<const CHANNELS: usize, const OPAQUE: bool, F: Fn(f64, f64) -> f64>(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, sample: F) -> bool {
    // Likewise, the presence of a selection is fixed for the whole grid. The
    // uncovered loop needs no per-pixel Option branch or coverage lookup.
    if coverage.is_some() {
        paint_grid_sample::<CHANNELS, true, OPAQUE, F>(doc, data, width, height, transform, coverage, paint, sample)
    } else {
        paint_grid_sample::<CHANNELS, false, OPAQUE, F>(doc, data, width, height, transform, coverage, paint, sample)
    }
}

fn paint_grid_sample<const CHANNELS: usize, const COVERED: bool, const OPAQUE: bool, F: Fn(f64, f64) -> f64>(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, sample: F) -> bool {
    let m = transform.pixel_to_document(width, height);
    let (cw, ch) = (doc.width as f64, doc.height as f64);
    // These are the exact endpoint expressions used by the loop below. Each
    // coordinate is monotone in x and y, including Float64 rounding, so finite
    // endpoints inside the canvas prove every pixel centre is inside. Overhangs
    // and non-finite transforms retain the original per-pixel clipping path.
    let inside = width > 0 && height > 0 && [0, height - 1].into_iter().all(|y| {
        let first = m.apply(Point { x: 0.5, y: y as f64 + 0.5 });
        [0, width - 1].into_iter().all(|x| {
            let (dx, dy) = (first.x + m.a * x as f64, first.y + m.b * x as f64);
            dx.is_finite() && dy.is_finite() && dx >= 0.0 && dy >= 0.0 && dx < cw && dy < ch
        })
    });
    if inside {
        paint_grid_clipped::<CHANNELS, COVERED, false, OPAQUE, F>(doc, data, width, height, m, coverage, paint, sample)
    } else {
        paint_grid_clipped::<CHANNELS, COVERED, true, OPAQUE, F>(doc, data, width, height, m, coverage, paint, sample)
    }
}

fn paint_grid_clipped<const CHANNELS: usize, const COVERED: bool, const CLIPPED: bool, const OPAQUE: bool, F: Fn(f64, f64) -> f64>(doc: &Document, data: &mut [u8], width: u32, height: u32, m: Affine, coverage: Option<&GrayRaster>, paint: &Paint, sample: F) -> bool {
    let (cw, ch) = (doc.width as f64, doc.height as f64);
    let w = width as usize;
    let (from, to, opacity) = match paint {
        Paint::Fill(c) => ([c[0], c[1], c[2], 1.0], [c[0], c[1], c[2], 1.0], 1.0),
        Paint::Gradient(g) => (g.from, g.to, g.opacity),
    };
    let delta: [f64; 4] = std::array::from_fn(|c| to[c] - from[c]);
    // The selection's coverage as a fraction, once for each of its 256 values.
    let fraction: [f64; 256] = std::array::from_fn(|k| k as f64 / 255.0);
    let mut touched = false;
    for y in 0..height as usize {
        // The row's first pixel centre in the document, and the step one pixel to the right.
        let first = m.apply(Point { x: 0.5, y: y as f64 + 0.5 });
        let cover = if COVERED { &coverage.expect("covered paint grid").bytes()[y * w..(y + 1) * w] } else { &[] };
        let line = &mut data[y * w * CHANNELS..(y + 1) * w * CHANNELS];
        for (x, px) in line.chunks_exact_mut(CHANNELS).enumerate() {
            let k = if COVERED { cover[x] } else { 255 };
            if k == 0 { continue; }
            let (dx, dy) = (first.x + m.a * x as f64, first.y + m.b * x as f64);
            if CLIPPED && (dx < 0.0 || dy < 0.0 || dx >= cw || dy >= ch) { continue; }
            touched = true;
            let t = sample(dx, dy);
            // With full coverage and finite ramp position, source alpha is
            // exactly one. Preserve the original channel expression and half
            // up rounding, while avoiding a destination read and blend. A
            // non-finite public-kernel sample still takes the original path:
            // even opaque stops produce NaN through 0 * NaN there.
            if OPAQUE && k == 255 && t.is_finite() {
                for c in 0..CHANNELS.min(3) { px[c] = ((from[c] + delta[c] * t) * 255.0 + 0.5) as u8; }
                if CHANNELS == 4 { px[3] = 255; }
                continue;
            }
            let s = (from[3] + delta[3] * t) * opacity * fraction[k as usize];
            if s <= 0.0 { continue; }
            let keep = 1.0 - s;
            if CHANNELS == 1 {
                px[0] = ((from[0] + delta[0] * t) * 255.0 * s + px[0] as f64 * keep + 0.5) as u8;
            } else {
                for c in 0..3 { px[c] = ((from[c] + delta[c] * t) * 255.0 * s + px[c] as f64 * keep + 0.5) as u8; }
                px[3] = (255.0 * s + px[3] as f64 * keep + 0.5) as u8;
            }
        }
    }
    touched
}

/// A covering mask carried onto the layer's new, trimmed grid (`crop` of `grid`): the old mask
/// stretched over where the layer's own pixels were, white where it grew (`expandMask`,
/// BrushStroke.swift:858-866). A uniform white mask stays one pixel: it shows everything either way.
pub(crate) fn followed(mask: &GrayRaster, old: (u32, u32), grid: &EditGrid, crop: (u32, u32, u32, u32)) -> GrayRaster {
    if mask.is_uniform() == Some(255) { return mask.clone(); }
    let (w, h) = (crop.2 - crop.0, crop.3 - crop.1);
    let mut out = vec![255u8; w as usize * h as usize];
    // Each output column's source column in the mask is looked up once into `xs` (`usize::MAX` outside
    // the layer's own old footprint, where the background, 255, stays); each row then reuses it with one
    // division for its own source row, instead of a division and a range check for every pixel -- the
    // mask preview path already avoids this the same way (preview.rs's own `source` closure; fix round
    // 3, item 3: called on every drag tick of a reduced pixel-gradient preview over a covering mask).
    let source = |at: i64, offset: u32, own: u32, stored: u32| -> usize {
        let g = at - offset as i64;
        if g < 0 || g >= own as i64 { usize::MAX } else { (g as u64 * stored as u64 / own as u64) as usize }
    };
    let xs: Vec<usize> = (0..w).map(|x| source((x + crop.0) as i64, grid.x, old.0, mask.width)).collect();
    let src = mask.bytes();
    for y in 0..h {
        let sy = source((y + crop.1) as i64, grid.y, old.1, mask.height);
        if sy == usize::MAX { continue; }
        let row_src = &src[sy * mask.width as usize..(sy + 1) * mask.width as usize];
        let row_dst = &mut out[y as usize * w as usize..(y as usize + 1) * w as usize];
        for (dst, &sx) in row_dst.iter_mut().zip(xs.iter()) { if sx != usize::MAX { *dst = row_src[sx]; } }
    }
    GrayRaster::from_bytes(w, h, out)
}

/// A layer that can be painted: not a folder (its mask can be), not an adjustment layer, an enabled
/// mask when the mask is the target (`canPaint`, EditorSession+Brush.swift:5-11); a selection with
/// something in it when there is one.
pub(crate) fn check_target(doc: &Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
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

/// The layer's own pixel grid: its pixels, or its box rounded when it has none.
pub(crate) fn layer_grid(layer: &Layer) -> (u32, u32) {
    layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height))
}

/// The document rectangle (`x0`, `y0`) to (`x1`, `y1`) on the grid `inverse` maps the document onto:
/// the box of its corners there, rounded out to whole pixels (`CGRect.applying(_:).integral`). Each
/// corner is snapped to an integer first when it lands within 1e-6 of one (`snap_near_int`, fix round
/// 1, I-1): otherwise a mask already grown exactly to the canvas can drift a few ulps off an edge and
/// regrow by a pixel on the next no-op edit.
fn rect_on(inverse: &Affine, x0: f64, y0: f64, x1: f64, y1: f64) -> (i64, i64, i64, i64) {
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| inverse.apply(Point { x, y }));
    let (lx, hx) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.x), h.max(p.x)));
    let (ly, hy) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.y), h.max(p.y)));
    (snap_near_int(lx).floor() as i64, snap_near_int(ly).floor() as i64, snap_near_int(hx).ceil() as i64, snap_near_int(hy).ceil() as i64)
}

/// The Mac's raster-edit tile, in grid pixels (`BrushStroke.tileSize`, BrushStroke.swift:144 at v1.3.7).
const TILE: i64 = 256;

/// Where a mask edit paints, and the mask it leaves (Task 14a): the mask's grid before the edit, `w` x
/// `h` pixels that `base` places, and the grid the edit leaves, `width` x `height` pixels that
/// `transform` places, with the old grid at (`x`, `y`) in it. `placement` is the mask's placement
/// afterwards: None while it still covers its layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaskGrid {
    pub w: u32, pub h: u32, pub base: LayerTransform,
    pub width: u32, pub height: u32, pub x: u32, pub y: u32,
    pub transform: LayerTransform, pub placement: Option<LayerTransform>,
}

impl MaskGrid {
    /// The pixels the edit paints and leaves.
    pub fn pixels(&self) -> u64 { self.width as u64 * self.height as u64 }
}

/// The grid a mask edit paints and the mask it leaves, as Compositor 1.3.7 makes them (BrushStroke.swift
/// and EditorSession+Brush.swift at v1.3.7). The mask's own grid: a mask on its own placement is painted
/// in its own pixels, a solid one (at most 2 x 2) at one pixel per document pixel over its place, a
/// covering mask in its layer's grid, stretched onto it (`init`, :157-167). A growing edit (`grows`:
/// the Mac's Fill and Gradient both are, `applyPixelEdit`, SelectionEdits.swift:204, and `beginGradient`,
/// Gradient.swift:48; its other raster edits, not in this port, are not) widens that grid to the canvas
/// as the grid maps it, rounded out (:168),
/// and keeps every 256-pixel tile it paints, counted from the widened grid's corner: the old grid joined
/// with the tiles over the canvas, or over the selection's clip (`paintCanvas`, :658-670; `allocateTile`,
/// :576-583; `committedBounds`, :764). The mask then takes its own place on the document when it had one
/// or grew (`commitRasterEdit`, EditorSession+Brush.swift:180-187; `transform(for:)`, :766-773); a place
/// that did not change is kept exactly. Refused, as the Mac refuses, when a side passes MAX_SIDE (:182,
/// :581), the mask passes what the other layers' masks leave of the budget (EditorSession+Brush.swift:
/// 16-20 with the mask targeted; BrushStroke.swift:582), or its placement is not a valid one
/// (EditorSession+Brush.swift:163, :168).
pub fn mask_grid(doc: &Document, id: Uuid, grows: bool) -> Result<MaskGrid, CommandError> {
    let too_large = || CommandError::Project(ProjectError::TooLarge);
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
    let base = m.placement.unwrap_or(layer.transform);
    let (w, h) = match m.placement {
        Some(p) if m.pixels.width <= 2 && m.pixels.height <= 2 => (p.size.width.round().max(1.0), p.size.height.round().max(1.0)),
        Some(_) => (m.pixels.width as f64, m.pixels.height as f64),
        None => { let (w, h) = layer_grid(layer); (w as f64, h as f64) }
    };
    if w > MAX_SIDE as f64 || h > MAX_SIDE as f64 { return Err(too_large()); }
    let (w, h) = (w as u32, h as u32);
    let (mut x0, mut y0, mut x1, mut y1) = (0i64, 0i64, w as i64, h as i64);
    if grows {
        let inverse = base.pixel_to_document(w, h).invert().ok_or_else(|| CommandError::Argument("the mask cannot be painted".into()))?;
        let canvas = rect_on(&inverse, 0.0, 0.0, doc.width as f64, doc.height as f64);
        let (ex0, ey0, ex1, ey1) = (canvas.0.min(0), canvas.1.min(0), canvas.2.max(w as i64), canvas.3.max(h as i64));
        // What the edit paints: the canvas, or the selection's clip rectangle (cut to the canvas), read
        // from the selection's bounds without filling the clip (`SelectionClip::region`): the app asks
        // this on the UI thread before a job (audit I-5).
        let area = match &doc.selection {
            None => Some((0.0, 0.0, doc.width as f64, doc.height as f64)),
            Some(s) => SelectionClip::region(s, doc.width, doc.height).map(|(x0, y0, x1, y1)| (x0 as f64, y0 as f64, x1 as f64, y1 as f64)),
        };
        if let Some((ax0, ay0, ax1, ay1)) = area {
            let r = rect_on(&inverse, ax0, ay0, ax1, ay1);
            let (rx0, ry0, rx1, ry1) = (r.0.max(ex0), r.1.max(ey0), r.2.min(ex1), r.3.min(ey1));
            if rx0 < rx1 && ry0 < ry1 {
                let (tx0, ty0) = (ex0 + (rx0 - ex0) / TILE * TILE, ey0 + (ry0 - ey0) / TILE * TILE);
                let (tx1, ty1) = ((ex0 + ((rx1 - 1 - ex0) / TILE + 1) * TILE).min(ex1), (ey0 + ((ry1 - 1 - ey0) / TILE + 1) * TILE).min(ey1));
                (x0, y0, x1, y1) = (tx0.min(0), ty0.min(0), tx1.max(w as i64), ty1.max(h as i64));
            }
        }
    }
    let (width, height) = (x1 - x0, y1 - y0);
    let others = doc.used_mask_pixels().saturating_sub(m.pixels.width as u64 * m.pixels.height as u64);
    if width > MAX_SIDE || height > MAX_SIDE || (width as u64) * (height as u64) > MAX_PIXELS.saturating_sub(others) { return Err(too_large()); }
    let (width, height, x, y) = (width as u32, height as u32, (-x0) as u32, (-y0) as u32);
    let grew = (x, y, width, height) != (0, 0, w, h);
    let transform = if grew { ops::adjust::placed_like(&base, w, h, width, height, x as f64, y as f64) } else { base };
    if !transform.is_valid() { return Err(too_large()); }
    let placement = if m.placement.is_some() || grew { Some(transform) } else { None };
    Ok(MaskGrid { w, h, base, width, height, x, y, transform, placement })
}

/// The mask as a mask edit starts from it on `grid`: its background everywhere (white reveals, black
/// hides: `Mask::background`, the Mac's `maskBackground`, BrushStroke.swift:169-170), then its own
/// pixels over the old grid, stretched onto it nearest where they are sized otherwise (a 1 x 1 mask, a
/// solid placed one). Each new tile starts so (`allocateTile`, :585-613), as the commit's canvas does
/// (`BrushCommit.render`, :888-896).
pub(crate) fn mask_on_grid(m: &Mask, grid: &MaskGrid) -> Vec<u8> {
    let mut out = vec![m.background(); grid.width as usize * grid.height as usize];
    let (mw, mh) = (m.pixels.width as u64, m.pixels.height as u64);
    let src = m.pixels.bytes();
    let xs: Vec<usize> = (0..grid.w as u64).map(|x| (x * mw / grid.w as u64) as usize).collect();
    for y in 0..grid.h as u64 {
        let sy = (y * mh / grid.h as u64) as usize;
        let row = &src[sy * mw as usize..(sy + 1) * mw as usize];
        let at = ((y + grid.y as u64) * grid.width as u64 + grid.x as u64) as usize;
        let dst = &mut out[at..at + grid.w as usize];
        if mw == grid.w as u64 { dst.copy_from_slice(row); } else { for (d, &sx) in dst.iter_mut().zip(&xs) { *d = row[sx]; } }
    }
    out
}

/// The pixels a Fill or a Gradient on layer `id` paints and leaves (`paint_layer`): the layer's grid
/// grown to the canvas (`image_grid`), or with `mask` the mask's grid grown to the canvas (`mask_grid`).
/// What decides whether the edit goes to the job worker (ruling C1, `Engine::edit_pixels`: a small
/// layer's mask on a large canvas counts at its grown size); an error where the edit is refused for its
/// size, which the worker, seeing one layer, could not tell (ruling OQ20).
pub fn painted_pixels(doc: &Document, id: Uuid, mask: bool) -> Result<u64, CommandError> {
    if mask { return Ok(mask_grid(doc, id, true)?.pixels()); }
    let grid = image_grid(doc, doc.layer(id).ok_or(CommandError::NoLayer)?)?;
    Ok(grid.width as u64 * grid.height as u64)
}

/// Whether `paint_layer` would paint: the paint's values and the target (a preview asks first, and
/// shows nothing the commit would refuse). On the mask, this also refuses what the mask edit would
/// refuse for its size (`mask_grid`, the grid the commit and the preview both paint: grown to the
/// canvas for a Gradient, so the refusal is at the grown size).
///
/// Known gap (fix round 1, item 3b, a controller ruling): on the PIXELS side, a growing layer's own
/// covering mask may also need to grow once the paint is trimmed back (`paint_layer`'s own check
/// below, near its `followed` call) -- that check depends on the trimmed commit result, which this
/// preview-time check never computes (a preview never actually paints and trims the grid), so it is
/// NOT mirrored here. A rare preview on a growing layer with a non-white covering mask near the mask
/// budget may therefore show a gradient that Return then refuses with "too large". Parked; not fixed
/// this round.
pub fn paint_layer_check(doc: &Document, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
    paint.check()?;
    check_target(doc, id, mask)?;
    if mask { mask_grid(doc, id, true)?; }
    Ok(())
}

/// Paints `paint` into layer `id`'s pixels (`mask` false) or its mask, as one edit: Fill and the
/// Gradient's commit. An edit that reaches no pixel (a selection moved off the canvas) leaves the
/// document exactly as it was, so nothing is recorded: the Mac returns before committing when its edit
/// made no patch (`guard !edit.patches.isEmpty`, SelectionEdits.swift:205), where this port used to grow
/// the layer, trim it and record a step (final review minor 8).
pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
    paint_layer_check(doc, id, mask, paint)?;
    if mask {
        // The mask on the grid the edit leaves (grown to the canvas for a Gradient), painted there, and
        // placed where that grid sits (EditorSession+Brush.swift:180-187 at v1.3.7).
        let grid = mask_grid(doc, id, true)?;
        let mut data = mask_on_grid(doc.layer(id).unwrap().mask.as_ref().unwrap(), &grid);
        let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, grid.width, grid.height)?;
        if !paint_grid(doc, &mut data, grid.width, grid.height, &grid.transform, coverage.as_ref(), paint, true) { return Ok(()); }
        let target = doc.layer_mut(id).unwrap().mask_mut().unwrap();
        target.pixels = GrayRaster::from_bytes(grid.width, grid.height, data);
        target.placement = grid.placement;
        return Ok(());
    }
    let layer = doc.layer(id).unwrap().clone();
    let grid = image_grid(doc, &layer)?;
    let mut data = pixels_on(&grid, layer.pixels.as_ref());
    let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, grid.width, grid.height)?;
    if !paint_grid(doc, &mut data, grid.width, grid.height, &grid.transform, coverage.as_ref(), paint, false) { return Ok(()); }
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
    // grows a mask checks it (masks.rs:13, :80; selection.rs:215; `mask_grid` above), the
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
