use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What the open panel is showing. The engine computes it from the layer's stored pixels every
/// time, so dragging a slider never accumulates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "preview")]
pub enum PreviewRequest {
    Adjustment { #[serde(with = "ids::upper")] layer: Uuid, adjustment: LayerAdjustment },
    Filter { #[serde(with = "ids::upper")] layer: Uuid, params: FilterParams },
}

impl PreviewRequest {
    pub fn layer(&self) -> Uuid { match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::Filter { layer, .. } => *layer } }
}

/// The substituted pixels for one layer while a panel is open.
#[derive(Clone, Debug)]
pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64 }

/// Previews render from a copy no larger than this on its longest side. Grain and Add Noise are
/// made at full size: their pattern is per pixel, and a small copy enlarged looks coarse.
pub fn preview_limit(request: &PreviewRequest) -> u32 {
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => if adjustment.kind == AdjustmentKind::Grain { u32::MAX } else { 4096 },
        PreviewRequest::Filter { params, .. } => if matches!(params, FilterParams::AddNoise { .. }) { u32::MAX } else { 2048 },
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

/// The preview raster for a request, or None when there is nothing to show.
pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
    let layer = doc.layer(request.layer())?;
    let raster = layer.pixels.as_ref()?;
    let limit = preview_limit(request);
    let (source, factor) = reduced(raster, limit);
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => {
            if !adjustment.is_valid() { return None; }
            // Grain and the tonal kernels read document space, which the reduced grid still covers.
            let units = layer.transform.size.width / source.width.max(1) as f64;
            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, None);
            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision })
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
            let result = adjust::filters::apply_filter(&grid, &scaled);
            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision })
        }
    }
}
