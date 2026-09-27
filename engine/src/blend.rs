use crate::BlendMode;

fn clamp01(v: f32) -> f32 { v.clamp(0.0, 1.0) }

/// Hard Mix is 1 only when backdrop plus source exceeds 1; a sum of exactly 1 gives 0 (probe
/// results "Blend modes"). Half an 8-bit level of margin keeps a sum that is exactly 1 in 8-bit
/// values at 0 whatever the float rounding of `cs = premultiplied / alpha`.
pub const HARD_MIX_MARGIN: f32 = 0.5 / 255.0;

fn color_dodge(cb: f32, cs: f32) -> f32 { if cb <= 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) } }
fn color_burn(cb: f32, cs: f32) -> f32 { if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) } }

/// Pegtop's Soft Light, `(1 - 2 cs) cb^2 + 2 cs cb`: what Compositor for Mac 1.2.10 draws. The
/// blend-greys probe fits it within 1 level at every grey and alpha, where the W3C / PDF formula is
/// 14 levels off at a 75% grey source (probe results, "Phase 3.5b follow-up probes").
fn soft_light(cb: f32, cs: f32) -> f32 { (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb }

/// PDF separable blend function B(cb, cs) on straight (unpremultiplied) channel values.
pub fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => cb + cs - cb * cs,
        // Overlay is HardLight with the arguments swapped (PDF spec); BLEND_GLSL in
        // app/src/canvas/gl/programs.ts mirrors this.
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::Darken => cb.min(cs),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::ColorDodge => color_dodge(cb, cs),
        BlendMode::ColorBurn => color_burn(cb, cs),
        // Non-separable modes are handled by blend_rgb; per channel they fall back to Normal.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => cs,
        BlendMode::LinearBurn => (cb + cs - 1.0).max(0.0),
        BlendMode::LinearDodge => (cb + cs).min(1.0),
        BlendMode::SoftLight => soft_light(cb, cs),
        BlendMode::HardLight => hard_light(cb, cs),
        // Colour burn at 2s and colour dodge at 2s - 1, with their W3C edge rules (a black backdrop
        // under a white source gives 0).
        BlendMode::VividLight => if cs <= 0.5 { color_burn(cb, 2.0 * cs) } else { color_dodge(cb, 2.0 * cs - 1.0) },
        BlendMode::LinearLight => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
        BlendMode::PinLight => if cs <= 0.5 { cb.min(2.0 * cs) } else { cb.max(2.0 * cs - 1.0) },
        BlendMode::HardMix => if cb + cs > 1.0 + HARD_MIX_MARGIN { 1.0 } else { 0.0 },
        BlendMode::Exclusion => cb + cs - 2.0 * cb * cs,
        // Backdrop minus source, never the reverse (probe results).
        BlendMode::Subtract => (cb - cs).max(0.0),
        // A black source divides to 1 over any lit backdrop and to 0 over black (probe results, x = 219).
        BlendMode::Divide => if cs <= 0.0 { if cb > 0.0 { 1.0 } else { 0.0 } } else { (cb / cs).min(1.0) },
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { cb * 2.0 * cs } else { let s = 2.0 * cs - 1.0; cb + s - cb * s }
}

/// Luminance from PDF spec: Lum(C) = 0.3 * R + 0.59 * G + 0.11 * B
fn lum(c: [f32; 3]) -> f32 { 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2] }

/// ClipColor from PDF spec: clips channels to [0, 1] while preserving luminance
fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 { for v in &mut out { *v = l + (*v - l) * l / (l - n).max(1e-6); } }
    if x > 1.0 { for v in &mut out { *v = l + (*v - l) * (1.0 - l) / (x - l).max(1e-6); } }
    out
}

/// SetLum from PDF spec: sets luminance while preserving hue and saturation
fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

/// Saturation from PDF spec: Sat(C) = max(C) - min(C)
fn sat(c: [f32; 3]) -> f32 { c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]) }

/// SetSat from PDF spec: sets saturation while preserving luminance and hue
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&a, &b| c[a].partial_cmp(&c[b]).unwrap_or(std::cmp::Ordering::Equal));
    let (imin, imid, imax) = (idx[0], idx[1], idx[2]);
    let mut out = [0.0f32; 3];
    let range = c[imax] - c[imin];
    if range > 0.0 {
        out[imid] = (c[imid] - c[imin]) * s / range;
        out[imax] = s;
    }
    out[imin] = 0.0;
    out
}

/// Blends straight RGB colours (0..1) with any mode.
pub fn blend_rgb(mode: BlendMode, backdrop: [f32; 3], source: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(source, sat(backdrop)), lum(backdrop)),
        BlendMode::Saturation => set_lum(set_sat(backdrop, sat(source)), lum(backdrop)),
        BlendMode::Color => set_lum(source, lum(backdrop)),
        BlendMode::Luminosity => set_lum(backdrop, lum(source)),
        _ => [separable(mode, backdrop[0], source[0]), separable(mode, backdrop[1], source[1]), separable(mode, backdrop[2], source[2])],
    }
}

/// Source-over with a blend mode, both premultiplied RGBA (0..1), result premultiplied.
pub fn compose(dst: [f32; 4], src: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let ad = dst[3]; let a_s = src[3];
    if a_s <= 0.0 { return dst; }
    let out_a = a_s + ad * (1.0 - a_s);
    if mode == BlendMode::Normal || ad <= 0.0 {
        return [src[0] + dst[0] * (1.0 - a_s), src[1] + dst[1] * (1.0 - a_s), src[2] + dst[2] * (1.0 - a_s), out_a];
    }
    let cb = [dst[0] / ad, dst[1] / ad, dst[2] / ad];
    let cs = [src[0] / a_s, src[1] / a_s, src[2] / a_s];
    let b = blend_rgb(mode, cb, cs);
    let mut out = [0.0f32; 4];
    for i in 0..3 {
        out[i] = clamp01(src[i] * (1.0 - ad) + dst[i] * (1.0 - a_s) + a_s * ad * clamp01(b[i]));
    }
    out[3] = out_a;
    out
}

pub fn compose_u8(dst: &mut [u8], src: [f32; 4], mode: BlendMode) {
    let d = [dst[0] as f32 / 255.0, dst[1] as f32 / 255.0, dst[2] as f32 / 255.0, dst[3] as f32 / 255.0];
    let out = compose(d, src, mode);
    for i in 0..4 { dst[i] = (out[i] * 255.0).round().clamp(0.0, 255.0) as u8; }
}
