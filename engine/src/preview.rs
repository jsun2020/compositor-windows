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
}

impl PreviewRequest {
    pub fn layer(&self) -> Uuid {
        match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::DragAdjustment { layer, .. } | PreviewRequest::Filter { layer, .. } => *layer }
    }
    /// Whether two requests compute the same pixels: the same layer and settings at the same
    /// effective limit (a Grain drag and a settled Grain are both full size).
    fn same_output(&self, other: &PreviewRequest) -> bool {
        use PreviewRequest::*;
        let content = match (self, other) {
            (Adjustment { adjustment: a, .. } | DragAdjustment { adjustment: a, .. }, Adjustment { adjustment: b, .. } | DragAdjustment { adjustment: b, .. }) => a == b,
            (Filter { params: a, .. }, Filter { params: b, .. }) => a == b,
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

/// The substituted pixels for one layer while a panel is open, the request that made them and
/// what they were made from.
#[derive(Clone, Debug)]
pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest, pub source: PreviewSource }

impl PixelPreview {
    /// Whether `request`, on a document that is now `source`, would compute exactly these pixels
    /// again, so they can be kept.
    pub fn answers(&self, request: &PreviewRequest, source: &PreviewSource) -> bool { self.request.same_output(request) && self.source == *source }
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

/// Previews render from a copy no larger than this on its longest side. Grain and Add Noise are
/// made at full size, dragged or not: their pattern is per pixel, and a small copy enlarged
/// looks coarse.
pub fn preview_limit(request: &PreviewRequest) -> u32 {
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_PREVIEW_LIMIT },
        PreviewRequest::DragAdjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_DRAG_LIMIT },
        PreviewRequest::Filter { params, .. } => if matches!(params, FilterParams::AddNoise { .. }) { u32::MAX } else { FILTER_PREVIEW_LIMIT },
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
    let layer = doc.layer(request.layer())?;
    let raster = layer.pixels.as_ref()?;
    let limit = preview_limit(request);
    let (source, factor) = reduced(raster, limit);
    let made_from = PreviewSource::of(doc, layer.id);
    match request {
        PreviewRequest::Adjustment { adjustment, .. } | PreviewRequest::DragAdjustment { adjustment, .. } => {
            if !adjustment.is_valid() { return None; }
            let coverage = ops::adjust::edit_coverage(doc, &layer.transform, source.width, source.height).ok()?;
            // Grain and the tonal kernels read document space, which the reduced grid still covers.
            let units = layer.transform.size.width / source.width.max(1) as f64;
            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, coverage.as_ref());
            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision, request: request.clone(), source: made_from })
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
            let coverage = ops::adjust::edit_coverage(doc, &placed, grid.width, grid.height).ok()?;
            let filtered = adjust::filters::apply_filter(&grid, &scaled);
            let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &grid, &c), None => filtered };
            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision, request: request.clone(), source: made_from })
        }
    }
}
