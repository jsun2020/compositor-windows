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
        }
    }

    /// One straight colour (0..1) at a document point. Only Grain reads `at`.
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
        }
    }
}
