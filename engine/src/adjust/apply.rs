use crate::*;

/// `coverage * adjusted + (1 - coverage) * original`, both premultiplied and the same size.
pub fn blend_by_coverage(adjusted: &Raster, original: &Raster, coverage: &GrayRaster) -> Raster {
    let mut data = adjusted.bytes().to_vec();
    let base = original.bytes();
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let k = coverage.bytes()[i] as u32;
        if k == 255 { continue; }
        for c in 0..4 {
            let a = p[c] as u32; let b = base[i * 4 + c] as u32;
            p[c] = ((a * k + b * (255 - k) + 127) / 255) as u8;
        }
    }
    Raster::from_premultiplied(adjusted.width, adjusted.height, data)
}

/// A whole raster through one adjustment. `origin` and `units_per_pixel` place the raster in
/// document space (Grain reads them); `selection` limits the change to its coverage.
pub fn apply_adjustment(raster: &Raster, a: &LayerAdjustment, origin: Point, units_per_pixel: f64, selection: Option<&GrayRaster>) -> Raster {
    let adjusted = match a.kind {
        AdjustmentKind::Levels => apply_tables(raster, &levels_tables(&a.levels)),
        AdjustmentKind::Curves => apply_tables(raster, &curves_tables(&a.curves)),
        AdjustmentKind::Exposure => apply_tables(raster, &exposure_table(&a.exposure())),
        AdjustmentKind::GradientMap => apply_gradient_map(raster, &gradient_map_table(&a.gradient_map())),
        AdjustmentKind::Hsv => apply_hsv(raster, &a.resolved_hsv()),
        AdjustmentKind::Grain => apply_grain(raster, &a.grain(), origin, units_per_pixel),
        // Mac 1.2.6 additions, drawn from Phase 3.5b; no UI creates one of these in 3.5a, so the
        // whole-raster path (destructive Layer > Apply) just returns the input unchanged.
        AdjustmentKind::AddNoise | AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur
            | AdjustmentKind::Invert | AdjustmentKind::BlackWhite | AdjustmentKind::ColorBalance => raster.clone(),
    };
    match selection { Some(coverage) => blend_by_coverage(&adjusted, raster, coverage), None => adjusted }
}
