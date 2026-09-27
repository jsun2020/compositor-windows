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

/// `blend_by_coverage` for a mask: `coverage * adjusted + (1 - coverage) * original`, the same size.
pub fn blend_gray_by_coverage(adjusted: &GrayRaster, original: &GrayRaster, coverage: &GrayRaster) -> GrayRaster {
    let base = original.bytes();
    let data = adjusted.bytes().iter().zip(coverage.bytes()).enumerate().map(|(i, (&a, &k))| {
        let (a, b, k) = (a as u32, base[i] as u32, k as u32);
        ((a * k + b * (255 - k) + 127) / 255) as u8
    }).collect();
    GrayRaster::from_bytes(adjusted.width, adjusted.height, data)
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
        AdjustmentKind::Invert => invert_raster(raster),
        AdjustmentKind::BlackWhite => apply_black_white(raster, &a.black_white()),
        AdjustmentKind::ColorBalance => if a.color_balance().is_zero() { raster.clone() } else { apply_color_balance(raster, &a.color_balance()) },
        // In the raster's own grid, origin zero, as the Mac's destructive noise_add.
        AdjustmentKind::AddNoise => add_noise_at(raster, a.noise_amount_percent(), a.noise_is_gaussian(), a.noise_is_monochromatic(), a.noise_seed_or_zero(), 0, 0),
        // A blur changes the layer's size: the destructive blurs are FilterParams
        // (ops::adjust::apply_filter), and apply_adjustment_to_layer refuses these kinds.
        AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur => raster.clone(),
    };
    match selection { Some(coverage) => blend_by_coverage(&adjusted, raster, coverage), None => adjusted }
}
