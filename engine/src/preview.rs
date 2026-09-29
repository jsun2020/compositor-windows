use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What the open panel is showing. The engine computes it from the layer's stored pixels every
/// time, so dragging a slider never accumulates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "preview")]
pub enum PreviewRequest {
    Adjustment { #[serde(with = "ids::upper")] layer: Uuid, adjustment: LayerAdjustment },
    /// The same adjustment while a slider is still moving: previewed from a smaller copy
    /// (`COLOUR_DRAG_LIMIT`) and followed by an `Adjustment` request once input settles.
    DragAdjustment { #[serde(with = "ids::upper")] layer: Uuid, adjustment: LayerAdjustment },
    Filter { #[serde(with = "ids::upper")] layer: Uuid, params: FilterParams },
    /// A gradient not yet applied (Phase 4b-1), on the layer's pixels or its mask: while `dragging`
    /// from a copy at most `GRADIENT_DRAG_LIMIT` across, then at most `GRADIENT_SETTLED_LIMIT`; inside a
    /// selection on pixels that already cover the canvas, at full size as a patch (`PATCH_LIMIT`).
    Gradient { #[serde(with = "ids::upper")] layer: Uuid, #[serde(default)] mask: bool, gradient: GradientSpec, #[serde(default)] dragging: bool },
}

impl PreviewRequest {
    pub fn layer(&self) -> Uuid {
        match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::DragAdjustment { layer, .. } | PreviewRequest::Filter { layer, .. } | PreviewRequest::Gradient { layer, .. } => *layer }
    }
    /// Whether two requests compute the same pixels: the same layer and settings at the same
    /// effective limit (a Grain drag and a settled Grain are both full size).
    fn same_output(&self, other: &PreviewRequest) -> bool {
        use PreviewRequest::*;
        let content = match (self, other) {
            (Adjustment { adjustment: a, .. } | DragAdjustment { adjustment: a, .. }, Adjustment { adjustment: b, .. } | DragAdjustment { adjustment: b, .. }) => a == b,
            (Filter { params: a, .. }, Filter { params: b, .. }) => a == b,
            (Gradient { mask: a, gradient: g, .. }, Gradient { mask: b, gradient: h, .. }) => a == b && g == h,
            _ => false,
        };
        content && self.layer() == other.layer() && preview_limit(self) == preview_limit(other)
    }
}

/// What a preview was computed from besides its request: the stored layer's pixels revision and
/// placement, and the selection's revision (0 for a missing layer).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewSource { pub pixels_revision: u64, pub transform: Option<LayerTransform>, pub selection_revision: u64 }

impl PreviewSource {
    pub fn of(doc: &Document, layer: Uuid) -> PreviewSource {
        let l = doc.layer(layer);
        PreviewSource { pixels_revision: l.map_or(0, |l| l.pixels_revision), transform: l.map(|l| l.transform), selection_revision: doc.selection_revision }
    }
}

/// What a preview stands in for: the layer's pixels (`raster`, placed by `transform`), a patch of them
/// (`raster` is that rectangle of the stored pixels' grid, drawn over them), or the layer's mask.
#[derive(Clone, Debug)]
pub enum PreviewTarget { Pixels, Patch(PixelRect), Mask(GrayRaster) }

/// The substituted pixels for one layer while a panel is open, the request that made them and
/// what they were made from.
#[derive(Clone, Debug)]
pub struct PixelPreview {
    pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest, pub source: PreviewSource,
    pub target: PreviewTarget,
    /// A patch drawn over the stored pixels, made once and only when something reads the pixels whole
    /// (the CPU compositor, a whole texture upload): the GPU's partial path reads the patch alone.
    patched: std::sync::OnceLock<Raster>,
}

impl PixelPreview {
    /// Whether `request`, on a document that is now `source`, would compute exactly these pixels
    /// again, so they can be kept.
    pub fn answers(&self, request: &PreviewRequest, source: &PreviewSource) -> bool { self.request.same_output(request) && self.source == *source }
    fn new(layer: Uuid, raster: Raster, transform: LayerTransform, revision: u64, request: &PreviewRequest, source: PreviewSource, target: PreviewTarget) -> PixelPreview {
        PixelPreview { layer, raster, transform, revision, request: request.clone(), source, target, patched: std::sync::OnceLock::new() }
    }
    /// The stored pixels with this preview's patch drawn over them, made on first use.
    pub fn patched(&self, stored: &Raster) -> Raster {
        let PreviewTarget::Patch(rect) = self.target else { return self.raster.clone() };
        self.patched.get_or_init(|| {
            let mut data = stored.bytes().to_vec();
            let row = rect.width as usize * 4;
            for y in 0..rect.height as usize {
                let at = ((rect.y as usize + y) * stored.width as usize + rect.x as usize) * 4;
                data[at..at + row].copy_from_slice(&self.raster.bytes()[y * row..(y + 1) * row]);
            }
            Raster::from_premultiplied(stored.width, stored.height, data)
        }).clone()
    }
}

// Preview sizes. A colour adjustment's preview costs about 0.1 us per preview pixel in release
// wasm and runs on the main thread on every slider tick; the kernels are memory-bound, so only
// fewer pixels make a tick cheaper. `reduced` halves until the longest side fits the limit, so a
// preview's longest side always lands between limit/2 and limit: a limit of 1024 leaves an
// 800x600 layer at full size (about 52 ms a tick for Levels, 67 ms for Hue/Saturation), while 512
// halves it (about 13 and 18 ms). 512 is visibly soft once shown larger than itself (4x at 100%
// zoom on a 1600-pixel layer), so it is used only while a slider moves: the store sends a
// `DragAdjustment` on each tick and an ordinary `Adjustment` about 150 ms after the last one,
// which previews at `COLOUR_PREVIEW_LIMIT`, the quality that shipped before the drag cap existed.
// Filters keep the Mac's 2048 and are not debounced.

/// The longest side a colour adjustment previews from while a slider is being dragged.
pub const COLOUR_DRAG_LIMIT: u32 = 512;
/// The longest side a colour adjustment previews from once input has settled.
pub const COLOUR_PREVIEW_LIMIT: u32 = 4096;
/// The longest side a filter previews from, as the Mac's `FilterEdit.previewLimit`.
pub const FILTER_PREVIEW_LIMIT: u32 = 2048;
/// The longest side a gradient previews from while its line is being dragged (ruling OQ9).
pub const GRADIENT_DRAG_LIMIT: u32 = 1024;
/// The longest side a gradient previews from once its line rests (ruling OQ9): the filters' 2048,
/// so the settled preview stays near 100 ms at 24 and 100 MP; the applied gradient is full size.
pub const GRADIENT_SETTLED_LIMIT: u32 = 2048;
/// The most pixels a gradient's patch preview may hold (about 724 x 724; ruling OQ10): painting it
/// takes about 25 ms in the release wasm, inside a drag tick's 50. Larger selections preview from a
/// reduced copy as a whole-layer gradient does.
pub const PATCH_LIMIT: u64 = 1 << 19;

/// Previews render from a copy no larger than this on its longest side. Grain and Add Noise are
/// made at full size, dragged or not: their pattern is per pixel, and a small copy enlarged
/// looks coarse.
pub fn preview_limit(request: &PreviewRequest) -> u32 {
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_PREVIEW_LIMIT },
        PreviewRequest::DragAdjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_DRAG_LIMIT },
        PreviewRequest::Filter { params, .. } => if matches!(params, FilterParams::AddNoise { .. }) { u32::MAX } else { FILTER_PREVIEW_LIMIT },
        PreviewRequest::Gradient { dragging, .. } => if *dragging { GRADIENT_DRAG_LIMIT } else { GRADIENT_SETTLED_LIMIT },
    }
}

/// Sharp halvings until the longest side fits `limit`, and the factor that reached it.
fn reduced(raster: &Raster, limit: u32) -> (Raster, f64) {
    let mut current = raster.clone();
    let mut factor = 1.0;
    while current.width.max(current.height) > limit && current.width > 1 && current.height > 1 {
        current = current.halved();
        factor *= 0.5;
    }
    (current, factor)
}

/// The preview raster for a request, or None when there is nothing to show (an empty selection
/// included: every edit refuses it). The selection's coverage is taken on the grid the preview is
/// computed on, reduced or grown, as the Mac's `previewMapping` does.
pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
    compute_preview_with(doc, &SelectionClips::default(), request, revision)
}

/// `compute_preview` with the selection's clip from `clips`: the engine's, so every tick of a drag
/// under one selection reuses one clip (final review F1).
pub fn compute_preview_with(doc: &Document, clips: &SelectionClips, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
    if let PreviewRequest::Gradient { layer, mask, gradient, .. } = request {
        return gradient_preview(doc, clips, request, *layer, *mask, gradient, revision);
    }
    let layer = doc.layer(request.layer())?;
    let raster = layer.pixels.as_ref()?;
    let limit = preview_limit(request);
    let (source, factor) = reduced(raster, limit);
    let made_from = PreviewSource::of(doc, layer.id);
    match request {
        PreviewRequest::Adjustment { adjustment, .. } | PreviewRequest::DragAdjustment { adjustment, .. } => {
            if !adjustment.is_valid() { return None; }
            let coverage = ops::adjust::edit_coverage(doc, clips, &layer.transform, source.width, source.height).ok()?;
            // Grain and the tonal kernels read document space, which the reduced grid still covers.
            let units = layer.transform.size.width / source.width.max(1) as f64;
            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, coverage.as_ref());
            Some(PixelPreview::new(layer.id, result, layer.transform, revision, request, made_from, PreviewTarget::Pixels))
        }
        PreviewRequest::Filter { params, .. } => {
            let params = params.normalized();
            if params.is_identity() { return None; }
            // Grown but never trimmed: trimming mid-drag would make the layer jump about.
            let scaled = params.scaled(factor);
            let (grid, placed) = match params.spreads() {
                true => ops::adjust::grown(&source, &layer.transform, scaled.margin())?,
                false => (source, layer.transform),
            };
            let coverage = ops::adjust::edit_coverage(doc, clips, &placed, grid.width, grid.height).ok()?;
            let filtered = adjust::filters::apply_filter(&grid, &scaled);
            let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &grid, &c), None => filtered };
            Some(PixelPreview::new(layer.id, result, placed, revision, request, made_from, PreviewTarget::Pixels))
        }
        PreviewRequest::Gradient { .. } => None,
    }
}

/// The fewest halvings that bring `width` x `height` to at most `limit` on its longer side.
fn level_for(width: u32, height: u32, limit: u32) -> u32 {
    let mut level = 0;
    while (width >> level).max(height >> level) > limit && (width >> level) > 1 && (height >> level) > 1 { level += 1; }
    level
}

/// A gradient's preview (Phase 4b-1). On a mask: the mask reduced to the request's limit, painted.
/// On pixels inside a selection whose rectangle is small (`PATCH_LIMIT`), when the layer already
/// covers the canvas and draws no effects: that rectangle at full size, as a patch. Otherwise the
/// layer's grid grown to the canvas (`raster_edit::image_grid`), reduced to the limit with its
/// origin on the reduced pixels, the layer's own halvings placed in it, painted; never trimmed.
fn gradient_preview(doc: &Document, clips: &SelectionClips, request: &PreviewRequest, id: Uuid, mask: bool, gradient: &GradientSpec, revision: u64) -> Option<PixelPreview> {
    use ops::raster_edit::{image_grid, paint_grid, Paint};
    let layer = doc.layer(id)?;
    let paint = Paint::Gradient(gradient.clone());
    // What the commit would refuse, the preview does not show.
    if ops::raster_edit::paint_layer_check(doc, id, mask, &paint).is_err() { return None; }
    let made_from = PreviewSource::of(doc, id);
    let limit = preview_limit(request);
    if mask {
        // The grid the commit paints: a placed mask's own, else the layer's pixels (a covering mask
        // of another size is stretched onto it), reduced; the mask sampled onto it, nearest.
        let m = layer.mask.as_ref()?;
        let placement = m.placement.unwrap_or(layer.transform);
        let (gw, gh) = if m.placement.is_some() { (m.pixels.width, m.pixels.height) } else {
            layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height))
        };
        let level = level_for(gw, gh, limit);
        let (rw, rh) = ((gw >> level).max(1), (gh >> level).max(1));
        let (mw, mh) = (m.pixels.width as u64, m.pixels.height as u64);
        // A uniform mask (freshly added, or filled) needs no per-pixel resample: every tick of a drag
        // would otherwise redo this gather from scratch, unlike the pixel path's memoized halving.
        let mut data: Vec<u8> = match m.pixels.is_uniform() {
            Some(v) => vec![v; rw as usize * rh as usize],
            None => (0..rh as u64).flat_map(|y| (0..rw as u64).map(move |x| (x, y)))
                .map(|(x, y)| m.pixels.bytes()[((y * mh / rh as u64) * mw + x * mw / rw as u64) as usize]).collect(),
        };
        let coverage = ops::adjust::edit_coverage(doc, clips, &placement, rw, rh).ok()?;
        paint_grid(doc, &mut data, rw, rh, &placement, coverage.as_ref(), &paint, true);
        let shown = GrayRaster::from_bytes(rw, rh, data);
        let pixels = layer.pixels.clone().unwrap_or_else(|| Raster::new_transparent(1, 1));
        return Some(PixelPreview::new(id, pixels, layer.transform, revision, request, made_from, PreviewTarget::Mask(shown)));
    }
    let grid = image_grid(doc, layer).ok()?;
    let stored = layer.pixels.as_ref();
    // A patch: the layer covers the canvas already, the selection bounds a small rectangle of it.
    if let (Some(pixels), Some(clip)) = (stored, clips.clip(doc)) {
        let covers = (grid.width, grid.height) == (pixels.width, pixels.height);
        let rect = clip.rect_on_grid(&layer.transform.pixel_to_document(pixels.width, pixels.height), pixels.width, pixels.height)?;
        if covers && layer.extra.effects.is_none() && (rect.width as u64) * (rect.height as u64) <= PATCH_LIMIT && !rect.is_empty() {
            let placed = ops::adjust::placed_like(&layer.transform, pixels.width, pixels.height, rect.width, rect.height, -(rect.x as f64), -(rect.y as f64));
            let mut data = pixels.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes();
            let coverage = ops::adjust::edit_coverage(doc, clips, &placed, rect.width, rect.height).ok()?;
            paint_grid(doc, &mut data, rect.width, rect.height, &placed, coverage.as_ref(), &paint, false);
            let patch = Raster::from_premultiplied(rect.width, rect.height, data);
            return Some(PixelPreview::new(id, patch, layer.transform, revision, request, made_from, PreviewTarget::Patch(rect)));
        }
    }
    // Reduced: the grown grid at `level` halvings, its origin rounded out so the layer's own halved
    // pixels land on whole reduced pixels.
    let level = level_for(grid.width, grid.height, limit);
    let f = 1u32 << level;
    let (left, top) = (grid.x.div_ceil(f), grid.y.div_ceil(f));
    let (w, h) = stored.map_or((grid.width - grid.x, grid.height - grid.y), |p| (p.width, p.height));
    let (rw, rh) = (left + (grid.width - grid.x).div_ceil(f), top + (grid.height - grid.y).div_ceil(f));
    let placed = ops::adjust::placed_like(&layer.transform, w, h, rw * f, rh * f, (left * f) as f64, (top * f) as f64);
    let mut data = vec![0u8; rw as usize * rh as usize * 4];
    if let Some(p) = stored {
        let mut halved = p.clone();
        for _ in 0..level { halved = halved.halved(); }
        let row = halved.width.min(rw - left) as usize * 4;
        for y in 0..halved.height.min(rh - top) as usize {
            let at = ((y + top as usize) * rw as usize + left as usize) * 4;
            data[at..at + row].copy_from_slice(&halved.bytes()[y * halved.width as usize * 4..y * halved.width as usize * 4 + row]);
        }
    }
    let coverage = ops::adjust::edit_coverage(doc, clips, &placed, rw, rh).ok()?;
    paint_grid(doc, &mut data, rw, rh, &placed, coverage.as_ref(), &paint, false);
    Some(PixelPreview::new(id, Raster::from_premultiplied(rw, rh, data), placed, revision, request, made_from, PreviewTarget::Pixels))
}
