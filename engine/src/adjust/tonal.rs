use crate::{AdjustmentColor, BlackWhiteSettings, ColorBalanceSettings, ExposureSettings, GradientMapSettings, GrayRaster, Raster};

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

/// `adjust_black_white` (AdjustPixels.c:110-152) on one straight colour in 0..1: min(r,g,b) of
/// grey, plus (mid - min) of the secondary family and (max - mid) of the primary, weighted by the
/// six sliders; with Tint, an HSL colour at the tint hue whose lightness is that grey.
pub fn black_white_rgb(rgb: [f32; 3], s: &BlackWhiteSettings) -> [f32; 3] {
    let w = [s.reds, s.yellows, s.greens, s.cyans, s.blues, s.magentas].map(|v| (v / 100.0) as f32);
    let [r, g, b] = rgb.map(|v| v.min(1.0));
    let mx = r.max(g.max(b)); let mn = r.min(g.min(b)); let md = r + g + b - mx - mn;
    let (primary, secondary) = if mx == r { (0, if g >= b { 1 } else { 5 }) }
        else if mx == g { (2, if r >= b { 1 } else { 3 }) } else { (4, if g >= r { 3 } else { 5 }) };
    let gray = (mn + (md - mn) * w[secondary] + (mx - md) * w[primary]).clamp(0.0, 1.0);
    let saturation = s.tint_saturation / 100.0;
    if !(s.tint && saturation > 0.0) { return [gray; 3]; }
    let c = (1.0 - (2.0 * gray as f64 - 1.0).abs()) * saturation;
    let hp = (s.tint_hue % 360.0) / 60.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if hp < 1.0 { (c, x, 0.0) } else if hp < 2.0 { (x, c, 0.0) } else if hp < 3.0 { (0.0, c, x) }
        else if hp < 4.0 { (0.0, x, c) } else if hp < 5.0 { (x, 0.0, c) } else { (c, 0.0, x) };
    let m = gray as f64 - c / 2.0;
    [(r1 + m).clamp(0.0, 1.0) as f32, (g1 + m).clamp(0.0, 1.0) as f32, (b1 + m).clamp(0.0, 1.0) as f32]
}

/// The C routines' unpremultiply (`fminf(255, p * 255 / alpha) / 255`) and write-back
/// (`min(alpha, round(v * alpha))`), shared by the two byte kernels below.
fn through_straight(p: [u8; 4], f: impl Fn([f32; 3]) -> [f32; 3]) -> [u8; 4] {
    if p[3] == 0 { return p; }
    let alpha = p[3] as f32;
    let out = f([0, 1, 2].map(|c| (p[c] as f32 * 255.0 / alpha).min(255.0) / 255.0));
    let v = |x: f32| (x * alpha).round().clamp(0.0, alpha) as u8;
    [v(out[0]), v(out[1]), v(out[2]), p[3]]
}

fn map_pixels(raster: &Raster, f: impl Fn([u8; 4]) -> [u8; 4]) -> Raster {
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) { let out = f([p[0], p[1], p[2], p[3]]); p.copy_from_slice(&out); }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

pub fn black_white_pixel(p: [u8; 4], s: &BlackWhiteSettings) -> [u8; 4] { through_straight(p, |rgb| black_white_rgb(rgb, s)) }
pub fn apply_black_white(raster: &Raster, s: &BlackWhiteSettings) -> Raster { map_pixels(raster, |p| black_white_pixel(p, s)) }

/// How much a tone belongs to the shadows, midtones and highlights (`tonal_weights`, AdjustPixels.c:156-167).
fn tonal_weights(v: f32) -> (f32, f32, f32) {
    let (a, b, scale) = (0.25f32, 0.333f32, 0.7f32);
    let s = ((v - b) / -a + 0.5).clamp(0.0, 1.0);
    let h = ((v + b - 1.0) / a + 0.5).clamp(0.0, 1.0);
    let m1 = ((v - b) / a + 0.5).clamp(0.0, 1.0);
    let m2 = ((v + b - 1.0) / -a + 0.5).clamp(0.0, 1.0);
    (s * scale, m1 * m2 * scale, h * scale)
}

/// `adjust_color_balance` (AdjustPixels.c:169-196) on one straight colour in 0..1.
pub fn color_balance_rgb(rgb: [f32; 3], s: &ColorBalanceSettings) -> [f32; 3] {
    let d = |v: [f64; 3]| v.map(|x| (x / 100.0) as f32);
    let shadows = d([s.shadow_cyan_red, s.shadow_magenta_green, s.shadow_yellow_blue]);
    let midtones = d([s.mid_cyan_red, s.mid_magenta_green, s.mid_yellow_blue]);
    let highlights = d([s.highlight_cyan_red, s.highlight_magenta_green, s.highlight_yellow_blue]);
    let mut c = rgb.map(|v| v.min(1.0));
    let before = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    for i in 0..3 {
        let (ws, wm, wh) = tonal_weights(c[i]);
        c[i] += shadows[i] * ws + midtones[i] * wm + highlights[i] * wh;
        c[i] = c[i].clamp(0.0, 1.0);
    }
    if s.preserve_luminosity {
        let after = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        if after > 0.0001 { let ratio = before / after; for v in &mut c { *v = (*v * ratio).clamp(0.0, 1.0); } }
    }
    c
}

pub fn color_balance_pixel(p: [u8; 4], s: &ColorBalanceSettings) -> [u8; 4] { through_straight(p, |rgb| color_balance_rgb(rgb, s)) }
pub fn apply_color_balance(raster: &Raster, s: &ColorBalanceSettings) -> Raster { map_pixels(raster, |p| color_balance_pixel(p, s)) }

/// The colour a new Gradient Map starts from, as the Mac takes the palette's two colours.
pub fn gradient_map_from(shadows: [f64; 3], highlights: [f64; 3]) -> GradientMapSettings {
    GradientMapSettings {
        shadows: AdjustmentColor { red: shadows[0], green: shadows[1], blue: shadows[2] }.clamped(),
        highlights: AdjustmentColor { red: highlights[0], green: highlights[1], blue: highlights[2] }.clamped(),
        reversed: false,
    }
}
