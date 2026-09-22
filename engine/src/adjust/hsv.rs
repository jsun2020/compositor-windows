use crate::{HueSaturationSettings, Raster};

pub fn rgb_to_hsl(rgb: [f64; 3]) -> [f64; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let high = r.max(g).max(b); let low = r.min(g).min(b);
    let lightness = (high + low) / 2.0;
    let delta = high - low;
    if delta <= 0.0 { return [0.0, 0.0, lightness]; }
    let saturation = (delta / (1.0 - (2.0 * lightness - 1.0).abs())).min(1.0);
    let mut hue = if high == r { (g - b) / delta } else if high == g { (b - r) / delta + 2.0 } else { (r - g) / delta + 4.0 };
    hue *= 60.0;
    if hue < 0.0 { hue += 360.0; }
    [hue, saturation, lightness]
}

pub fn hsl_to_rgb(hsl: [f64; 3]) -> [f64; 3] {
    let (hue, saturation, lightness) = (hsl[0], hsl[1], hsl[2]);
    if saturation <= 0.0 { return [lightness, lightness, lightness]; }
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let base = lightness - chroma / 2.0;
    let (r, g, b) = match sector as i64 {
        0 => (chroma, second, 0.0), 1 => (second, chroma, 0.0), 2 => (0.0, chroma, second),
        3 => (0.0, second, chroma), 4 => (second, 0.0, chroma), _ => (chroma, 0.0, second),
    };
    [(r + base).clamp(0.0, 1.0), (g + base).clamp(0.0, 1.0), (b + base).clamp(0.0, 1.0)]
}

/// How much every range shifts each whole degree of hue: built once per settings so a per-pixel
/// adjustment does not re-evaluate all seven ranges.
pub fn hue_response(settings: &HueSaturationSettings) -> Vec<[f64; 3]> {
    (0..=360).map(|degree| {
        let mut response = [0.0f64; 3];
        for (range, adjustment) in &settings.adjustments {
            if *adjustment == Default::default() { continue; }
            let weight = settings.weight(*range, degree as f64);
            if weight <= 0.0 { continue; }
            response[0] += adjustment.hue * weight;
            response[1] += adjustment.saturation * weight;
            response[2] += adjustment.lightness * weight;
        }
        response
    }).collect()
}

/// The hue a spectrum swatch becomes, for the panel's "after" bar.
pub fn shifted_hue(hue: f64, settings: &HueSaturationSettings) -> f64 {
    let mut shift = 0.0;
    for (range, adjustment) in &settings.adjustments {
        if adjustment.hue != 0.0 { shift += adjustment.hue * settings.weight(*range, hue); }
    }
    let shifted = (hue + shift) % 360.0;
    if shifted < 0.0 { shifted + 360.0 } else { shifted }
}

/// One straight colour through the settings. `response` is `hue_response`, passed in so a whole
/// raster shares it.
pub fn adjust_rgb(rgb: [f64; 3], settings: &HueSaturationSettings, response: &[[f64; 3]]) -> [f64; 3] {
    let [mut hue, mut saturation, lightness] = rgb_to_hsl(rgb);
    let lightness_amount;
    if settings.colorize {
        let selected = settings.adjustment(settings.range);
        hue = selected.hue % 360.0;
        saturation = (selected.saturation / 100.0).clamp(0.0, 1.0);
        lightness_amount = selected.lightness / 100.0;
    } else {
        let sampled = response[(hue.round() as usize).min(response.len() - 1)];
        lightness_amount = sampled[2] / 100.0;
        hue = (hue + sampled[0]) % 360.0;
        if hue < 0.0 { hue += 360.0; }
        // Multiplicative, so neutral grays stay neutral.
        saturation = (saturation * (1.0 + sampled[1] / 100.0)).clamp(0.0, 1.0);
    }
    // Lightness pulls toward white above 0 and toward black below, reaching either at +/-100.
    let amount = lightness_amount.clamp(-1.0, 1.0);
    let lightness = if amount >= 0.0 { lightness + (1.0 - lightness) * amount } else { lightness * (1.0 + amount) };
    hsl_to_rgb([hue, saturation, lightness.clamp(0.0, 1.0)])
}

/// A whole raster: unpremultiplied, adjusted, premultiplied again (what CIColorCube does on macOS).
pub fn apply_hsv(raster: &Raster, settings: &HueSaturationSettings) -> Raster {
    if settings.is_identity() { return raster.clone(); }
    let response = hue_response(settings);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as f64;
        if a == 0.0 { continue; }
        let rgb = [(p[0] as f64 * 255.0 / a).min(255.0) / 255.0, (p[1] as f64 * 255.0 / a).min(255.0) / 255.0, (p[2] as f64 * 255.0 / a).min(255.0) / 255.0];
        let out = adjust_rgb(rgb, settings, &response);
        for c in 0..3 { p[c] = (out[c] * a).round().clamp(0.0, a) as u8; }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
