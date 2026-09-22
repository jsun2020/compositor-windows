use crate::{AdjustmentColor, ExposureSettings, GradientMapSettings, GrayRaster, Raster};

fn srgb_to_linear(v: f64) -> f64 { if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) } }
fn linear_to_srgb(v: f64) -> f64 { if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 } }

/// Photoshop's Exposure: scale linear light by 2^stops, add the offset, then correct gamma.
/// The same curve on every channel, so `apply_tables` gets three copies of it.
pub fn exposure_table(s: &ExposureSettings) -> Vec<f32> {
    let s = s.normalized();
    let scale = 2f64.powf(s.exposure);
    let one: Vec<f32> = (0..=255u32).map(|i| {
        let encoded = i as f64 / 255.0;
        let linear = (srgb_to_linear(encoded) * scale + s.offset).max(0.0).powf(1.0 / s.gamma);
        linear_to_srgb(linear).clamp(0.0, 1.0) as f32
    }).collect();
    let mut out = Vec::with_capacity(768);
    for _ in 0..3 { out.extend_from_slice(&one); }
    out
}

/// 256 RGB triples interpolating between the dark and light ends.
pub fn gradient_map_table(s: &GradientMapSettings) -> Vec<u8> {
    let s = s.normalized();
    let (dark, light) = s.ends();
    let channel = |d: f64, l: f64, t: f64| ((d + (l - d) * t) * 255.0).round().clamp(0.0, 255.0) as u8;
    let mut out = Vec::with_capacity(768);
    for i in 0..=255u32 {
        let t = i as f64 / 255.0;
        out.push(channel(dark.red, light.red, t));
        out.push(channel(dark.green, light.green, t));
        out.push(channel(dark.blue, light.blue, t));
    }
    out
}

/// `adjust_gradient_map` from AdjustPixels.c: Rec. 709 integer luma of the unpremultiplied
/// colour picks a table entry, which is written back premultiplied.
pub fn apply_gradient_map(raster: &Raster, table: &[u8]) -> Raster {
    assert_eq!(table.len(), 768);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a == 0 { continue; }
        let (mut r, mut g, mut b) = (p[0] as u32, p[1] as u32, p[2] as u32);
        if a < 255 {
            r = ((r * 255 + a / 2) / a).min(255);
            g = ((g * 255 + a / 2) / a).min(255);
            b = ((b * 255 + a / 2) / a).min(255);
        }
        let level = ((2126 * r + 7152 * g + 722 * b + 5000) / 10000).min(255) as usize;
        for c in 0..3 { p[c] = ((table[level * 3 + c] as u32 * a + 127) / 255) as u8; }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// Premultiplied invert: each colour becomes alpha minus colour, so transparency is kept.
pub fn invert_raster(raster: &Raster) -> Raster {
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) { let a = p[3]; for c in 0..3 { p[c] = a.saturating_sub(p[c]); } }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
pub fn invert_gray(mask: &GrayRaster) -> GrayRaster {
    GrayRaster::from_bytes(mask.width, mask.height, mask.bytes().iter().map(|v| 255 - v).collect())
}

/// The colour a new Gradient Map starts from, as the Mac takes the palette's two colours.
pub fn gradient_map_from(shadows: [f64; 3], highlights: [f64; 3]) -> GradientMapSettings {
    GradientMapSettings {
        shadows: AdjustmentColor { red: shadows[0], green: shadows[1], blue: shadows[2] }.clamped(),
        highlights: AdjustmentColor { red: highlights[0], green: highlights[1], blue: highlights[2] }.clamped(),
        reversed: false,
    }
}
