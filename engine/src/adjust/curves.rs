use crate::{CurvesSettings, LevelsChannel};

/// R, G, B tables of 256 outputs (0..1): the channel's own curve, then the composite RGB curve.
pub fn curves_tables(s: &CurvesSettings) -> Vec<f32> {
    let mut out = Vec::with_capacity(768);
    for ch in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue] {
        for v in 0..=255u32 { out.push((s.value(s.value(v as f64, ch.index()), 0) / 255.0) as f32); }
    }
    out
}
