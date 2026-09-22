use crate::{GrayRaster, LevelRange, LevelsChannel, LevelsSettings, Raster};

/// R, G, B tables of 256 outputs (0..1), each the channel's range then the composite range.
pub fn levels_tables(s: &LevelsSettings) -> Vec<f32> {
    let mut out = Vec::with_capacity(768);
    for ch in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue] {
        for v in 0..=255u32 { out.push(s.apply(v as f64 / 255.0, ch) as f32); }
    }
    out
}

/// `levels_apply` from LevelsPixels.c: per channel, unpremultiply once, interpolate the table, re-premultiply.
pub fn apply_tables(raster: &Raster, tables: &[f32]) -> Raster {
    assert_eq!(tables.len(), 768);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let alpha = p[3] as f32;
        if alpha == 0.0 { continue; }
        for c in 0..3 {
            let x = (p[c] as f32 * 255.0 / alpha).min(255.0);
            let lo = x as usize; let hi = if lo < 255 { lo + 1 } else { 255 };
            let t = &tables[c * 256..c * 256 + 256];
            let result = t[lo] + (t[hi] - t[lo]) * (x - lo as f32);
            p[c] = (result * alpha).round().max(0.0).min(alpha) as u8;
        }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// `levels_histogram`: 4 x 256 bins (RGB mean of the three, then R, G, B), weighted by alpha and coverage.
pub fn histogram(raster: &Raster, coverage: Option<&GrayRaster>) -> Vec<Vec<f64>> {
    let mut bins = vec![vec![0.0f64; 256]; 4];
    for (i, p) in raster.bytes().chunks_exact(4).enumerate() {
        if p[3] == 0 { continue; }
        let weight = p[3] as f64 / 255.0 * coverage.map_or(1.0, |c| c.bytes()[i] as f64 / 255.0);
        for c in 0..3 {
            let value = ((p[c] as f64 * 255.0 / p[3] as f64).round()).min(255.0) as usize;
            bins[c + 1][value] += weight;
            bins[0][value] += weight / 3.0;
        }
    }
    bins
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LevelsAuto { Contrast, Color, Neutral }

fn endpoints(bins: &[f64]) -> Option<(f64, f64)> {
    let total: f64 = bins.iter().sum();
    if total <= 0.0 { return None; }
    let (mut sum, mut low, mut high) = (0.0, 0usize, 255usize);
    for i in 0..256 { sum += bins[i]; if sum > total * 0.001 { low = i; break; } }
    sum = 0.0;
    for i in (0..256).rev() { sum += bins[i]; if sum > total * 0.001 { high = i; break; } }
    if low < high { Some((low as f64, high as f64)) } else { None }
}

impl LevelsAuto {
    pub fn settings(&self, histogram: &[Vec<f64>]) -> LevelsSettings {
        let mut result = LevelsSettings::default();
        if *self == LevelsAuto::Contrast {
            // A shared interval preserves channel relationships.
            let limits: Vec<(f64, f64)> = histogram[1..4].iter().filter_map(|b| endpoints(b)).collect();
            let low = limits.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
            let high = limits.iter().map(|l| l.1).fold(f64::NEG_INFINITY, f64::max);
            if !limits.is_empty() && low < high { result.ranges[0] = LevelRange { black: low, white: high, ..LevelRange::default() }; }
        } else {
            for c in 1..=3 {
                let Some((low, high)) = endpoints(&histogram[c]) else { continue; };
                let mut range = LevelRange { black: low, white: high, ..LevelRange::default() };
                if *self == LevelsAuto::Neutral {
                    let total: f64 = histogram[c].iter().sum();
                    let mean = histogram[c].iter().enumerate().fold(0.0, |acc, (i, w)| acc + range.apply(i as f64 / 255.0) * w) / total;
                    if mean > 0.0 && mean < 1.0 { range.gamma = (mean.ln() / 0.5f64.ln()).clamp(0.1, 9.99); }
                }
                result.ranges[c] = range;
            }
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LevelsSample { Black, Gray, White }

impl LevelsSettings {
    /// Samples are straight RGB 0..1; all three channels are calibrated together.
    pub fn sampling(&self, rgb: [f64; 3], mode: LevelsSample) -> LevelsSettings {
        let mut result = self.clone();
        result.ranges[0] = LevelRange::default();
        for c in 1..=3 {
            let mut range = result.ranges[c];
            let v = rgb[c - 1] * 255.0;
            match mode {
                LevelsSample::Black => range.black = v.max(0.0).min(range.white - 1.0),
                LevelsSample::White => range.white = v.min(255.0).max(range.black + 1.0),
                LevelsSample::Gray => {
                    let fraction = (v - range.black) / (range.white - range.black);
                    if !(fraction > 0.0 && fraction < 1.0) { continue; }
                    range.gamma = fraction.ln() / 0.5f64.ln();
                }
            }
            range.output_black = 0.0; range.output_white = 255.0;
            result.ranges[c] = range.normalized();
        }
        result
    }
}
