use crate::*;

/// `coverage * adjusted + (1 - coverage) * original`, both premultiplied and the same size.
pub fn blend_by_coverage(adjusted: &Raster, original: &Raster, coverage: &GrayRaster) -> Raster {
    if adjusted.deep_pixels().is_some() || original.deep_pixels().is_some() {
        let depth=if adjusted.depth()==PixelDepth::Hdr || original.depth()==PixelDepth::Hdr {PixelDepth::Hdr}else{PixelDepth::Sixteen};
        let a=adjusted.to_depth(depth);let b=original.to_depth(depth);
        let mut samples=a.deep_pixels().unwrap().samples.clone();let base=&b.deep_pixels().unwrap().samples;
        for (i,p) in samples.chunks_exact_mut(4).enumerate() {
            let k=coverage.bytes()[i] as u64;
            for c in 0..4 {p[c]=if depth==PixelDepth::Sixteen {((p[c] as u64*k+base[i*4+c] as u64*(255-k)+127)/255) as u16} else {
                half::f16::from_f32(half::f16::from_bits(p[c]).to_f32()*(k as f32/255.)+half::f16::from_bits(base[i*4+c]).to_f32()*((255-k) as f32/255.)).to_bits()
            };}
        }
        return Raster::from_deep(adjusted.width,adjusted.height,DeepPixels{depth,samples});
    }
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
    if raster.deep_pixels().is_some() {
        let adjusted=if raster.depth()==PixelDepth::Hdr && a.kind==AdjustmentKind::Exposure {
            let e=a.exposure().normalized();raster.hdr_exposure(e.exposure,e.offset,e.gamma)
        } else {
            let mixes=matches!(a.kind,AdjustmentKind::Hsv|AdjustmentKind::BlackWhite|AdjustmentKind::ColorBalance|AdjustmentKind::GradientMap);
            raster.map_deep::<std::convert::Infallible>(mixes,|r|Ok(apply_adjustment(r,a,origin,units_per_pixel,None))).unwrap()
        };
        return match selection {Some(coverage)=>blend_by_coverage(&adjusted,raster,coverage),None=>adjusted};
    }
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
        AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur | AdjustmentKind::ProgressiveBlur | AdjustmentKind::BloomGlow | AdjustmentKind::RgbSplit | AdjustmentKind::Vignette | AdjustmentKind::LensEffects | AdjustmentKind::TonalContrast | AdjustmentKind::Sharpen => raster.clone(),
    };
    match selection { Some(coverage) => blend_by_coverage(&adjusted, raster, coverage), None => adjusted }
}
