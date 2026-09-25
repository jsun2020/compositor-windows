use crate::*;

/// An adjustment with its tables built, able to map one straight colour at a time. Both
/// compositors use this for adjustment layers; the whole-raster kernels in `apply.rs` use the
/// same tables for destructive edits.
#[derive(Clone, Debug)]
pub enum PreparedAdjustment {
    /// 768 outputs in 0..1: R, G, B tables (levels, curves, exposure).
    Tables(Vec<f32>),
    /// 768 bytes: 256 RGB triples.
    GradientMap(Vec<u8>),
    Hsv { settings: HueSaturationSettings, response: Vec<[f64; 3]> },
    Grain { settings: GrainSettings },
    Invert,
    BlackWhite(BlackWhiteSettings),
    ColorBalance(ColorBalanceSettings),
    AddNoise { spread: f32, gaussian: bool, monochromatic: bool, seed: u32 },
    Identity,
}

impl PreparedAdjustment {
    pub fn prepare(a: &LayerAdjustment) -> PreparedAdjustment {
        match a.kind {
            AdjustmentKind::Levels => PreparedAdjustment::Tables(levels_tables(&a.levels)),
            AdjustmentKind::Curves => PreparedAdjustment::Tables(curves_tables(&a.curves)),
            AdjustmentKind::Exposure => PreparedAdjustment::Tables(exposure_table(&a.exposure())),
            AdjustmentKind::GradientMap => PreparedAdjustment::GradientMap(gradient_map_table(&a.gradient_map())),
            AdjustmentKind::Hsv => {
                let settings = a.resolved_hsv();
                if settings.is_identity() { return PreparedAdjustment::Identity; }
                let response = hue_response(&settings);
                PreparedAdjustment::Hsv { settings, response }
            }
            AdjustmentKind::Grain => {
                let settings = a.grain().normalized();
                if !(settings.amount > 0.0) { return PreparedAdjustment::Identity; }
                PreparedAdjustment::Grain { settings }
            }
            AdjustmentKind::Invert => PreparedAdjustment::Invert,
            AdjustmentKind::BlackWhite => PreparedAdjustment::BlackWhite(a.black_white()),
            // All zero changes nothing on the Mac either (ImageAdjustments.swift:167).
            AdjustmentKind::ColorBalance => if a.color_balance().is_zero() { PreparedAdjustment::Identity } else { PreparedAdjustment::ColorBalance(a.color_balance()) },
            AdjustmentKind::AddNoise => PreparedAdjustment::AddNoise { spread: a.noise_amount_percent() as f32 / 100.0 * 127.5,
                gaussian: a.noise_is_gaussian(), monochromatic: a.noise_is_monochromatic(), seed: a.noise_seed_or_zero() },
            // Spatial: drawn by compositor::spatial_target, never one colour at a time.
            AdjustmentKind::GaussianBlur | AdjustmentKind::MotionBlur => PreparedAdjustment::Identity,
        }
    }

    /// One straight colour (0..1) at a document point. Only Grain and Add Noise read `at`.
    pub fn color(&self, rgb: [f32; 3], at: Point) -> [f32; 3] {
        match self {
            PreparedAdjustment::Identity => rgb,
            PreparedAdjustment::Tables(tables) => {
                let mut out = [0f32; 3];
                for c in 0..3 {
                    let x = (rgb[c] * 255.0).clamp(0.0, 255.0);
                    let lo = x as usize; let hi = if lo < 255 { lo + 1 } else { 255 };
                    let t = &tables[c * 256..c * 256 + 256];
                    out[c] = t[lo] + (t[hi] - t[lo]) * (x - lo as f32);
                }
                out
            }
            PreparedAdjustment::GradientMap(table) => {
                // The same integer luma the kernel computes, so both paths pick the same entry.
                let r = (rgb[0] * 255.0).round().clamp(0.0, 255.0) as u32;
                let g = (rgb[1] * 255.0).round().clamp(0.0, 255.0) as u32;
                let b = (rgb[2] * 255.0).round().clamp(0.0, 255.0) as u32;
                let level = (((2126 * r + 7152 * g + 722 * b + 5000) / 10000).min(255)) as usize;
                [table[level * 3] as f32 / 255.0, table[level * 3 + 1] as f32 / 255.0, table[level * 3 + 2] as f32 / 255.0]
            }
            PreparedAdjustment::Hsv { settings, response } => {
                let out = adjust_rgb([rgb[0] as f64, rgb[1] as f64, rgb[2] as f64], settings, response);
                [out[0] as f32, out[1] as f32, out[2] as f32]
            }
            PreparedAdjustment::Grain { settings } => {
                let noise = grain_noise(at.x, at.y, settings.size, settings.roughness, settings.seed);
                let level = (0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]).min(1.0);
                let delta = noise * grain_strength(settings.amount) * grain_weight(level);
                let mut out = [0f32; 3];
                for c in 0..3 { out[c] = (rgb[c] * 255.0 + delta).clamp(0.0, 255.0) / 255.0; }
                out
            }
            PreparedAdjustment::Invert => [1.0 - rgb[0], 1.0 - rgb[1], 1.0 - rgb[2]],
            PreparedAdjustment::BlackWhite(s) => black_white_rgb(rgb, s),
            PreparedAdjustment::ColorBalance(s) => color_balance_rgb(rgb, s),
            PreparedAdjustment::AddNoise { spread, gaussian, monochromatic, seed } => {
                // The field position is the document pixel, as noise_add_at's is at export (origin 0).
                let base = noise_base(at.x.floor() as i64 as u32, at.y.floor() as i64 as u32, *seed);
                [0, 1, 2].map(|c| (rgb[c] * 255.0 + noise_offset(base, c, *spread, *gaussian, *monochromatic)).clamp(0.0, 255.0) / 255.0)
            }
        }
    }

    /// One premultiplied pixel through the Mac's own 8-bit kernel, for the kinds that have one
    /// (AdjustPixels.c, NoisePixels.c, PixelInvert.swift). The compositor uses it wherever the layer
    /// applies at full strength in Normal, so an export matches the Mac to the level; `color` serves
    /// every other case (partial strength, a blend mode, the GPU's mirror). None for the kinds whose
    /// Mac kernel is not a per-pixel byte routine.
    pub fn pixel(&self, p: [u8; 4], at: Point) -> Option<[u8; 4]> {
        match self {
            PreparedAdjustment::Grain { settings } => Some(grain_pixel(p, at.x, at.y, settings)),
            // PixelInvert.swift:28-36: premultiplied alpha minus colour.
            PreparedAdjustment::Invert => Some([p[3] - p[0].min(p[3]), p[3] - p[1].min(p[3]), p[3] - p[2].min(p[3]), p[3]]),
            PreparedAdjustment::BlackWhite(s) => Some(black_white_pixel(p, s)),
            PreparedAdjustment::ColorBalance(s) => Some(color_balance_pixel(p, s)),
            PreparedAdjustment::AddNoise { spread, gaussian, monochromatic, seed } =>
                Some(noise_pixel(p, at.x.floor() as i64 as u32, at.y.floor() as i64 as u32, *spread, *gaussian, *monochromatic, *seed)),
            _ => None,
        }
    }
}

/// The GPU's colour table for an adjustment layer: 256 RGBA rows for the kinds that map colour
/// through one (Levels, Curves, Exposure, Gradient Map), empty for the others. Also empty for
/// settings that fail `is_valid`: the table builders index `ranges[channel]` and a curve's
/// neighbouring points, so a malformed adjustment from a caller would trap the wasm instance.
pub fn gpu_lut(a: &LayerAdjustment) -> Vec<u8> {
    let mut out = Vec::new();
    if !a.is_valid() { return out; }
    match PreparedAdjustment::prepare(a) {
        PreparedAdjustment::Tables(tables) => {
            for i in 0..256 { for c in 0..3 { out.push((tables[c * 256 + i] * 255.0).round().clamp(0.0, 255.0) as u8); } out.push(255); }
        }
        PreparedAdjustment::GradientMap(table) => {
            for i in 0..256 { out.extend_from_slice(&table[i * 3..i * 3 + 3]); out.push(255); }
        }
        _ => {}
    }
    out
}

/// The GPU's hue response: 361 entries of (hue shift, saturation, lightness, 0) for a
/// Hue/Saturation adjustment, empty for any other kind and for settings that fail `is_valid`.
pub fn gpu_hue_response(a: &LayerAdjustment) -> Vec<f32> {
    if a.kind != AdjustmentKind::Hsv || !a.is_valid() { return Vec::new(); }
    let mut out = Vec::with_capacity(361 * 4);
    for entry in hue_response(&a.resolved_hsv()) { out.extend_from_slice(&[entry[0] as f32, entry[1] as f32, entry[2] as f32, 0.0]); }
    out
}