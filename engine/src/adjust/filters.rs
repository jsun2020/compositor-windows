use crate::Raster;
use serde::{Deserialize, Serialize};

fn clamp_or(n: f64, lo: f64, hi: f64, fallback: f64) -> f64 { if n.is_finite() { n.clamp(lo, hi) } else { fallback } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "filter")]
pub enum FilterParams {
    /// Standard deviation in layer pixels, 0.1 to 250.
    GaussianBlur { radius: f64 },
    /// Direction in degrees counter-clockwise from horizontal (-90 to 90) and streak length in layer pixels (1 to 2000).
    MotionBlur { angle: f64, distance: f64 },
    /// Photoshop's percentage, 0.1 to 400.
    AddNoise { amount: f64, gaussian: bool, monochromatic: bool, seed: u32 },
    /// Remove Distortion, -100 to 100: positive straightens barrel, negative pincushion.
    LensCorrection { distortion: f64 },
}

/// Remove Distortion at +/-100 moves the corners by this share of their distance from the centre.
pub const LENS_STRENGTH: f64 = 0.35;

impl FilterParams {
    pub fn name(&self) -> &'static str {
        match self { FilterParams::GaussianBlur { .. } => "Gaussian Blur", FilterParams::MotionBlur { .. } => "Motion Blur",
            FilterParams::AddNoise { .. } => "Add Noise", FilterParams::LensCorrection { .. } => "Lens Correction" }
    }
    pub fn normalized(&self) -> FilterParams {
        match *self {
            FilterParams::GaussianBlur { radius } => FilterParams::GaussianBlur { radius: clamp_or(radius, 0.1, 250.0, 1.0) },
            FilterParams::MotionBlur { angle, distance } => FilterParams::MotionBlur { angle: clamp_or(angle, -90.0, 90.0, 0.0), distance: clamp_or(distance, 1.0, 2000.0, 10.0) },
            FilterParams::AddNoise { amount, gaussian, monochromatic, seed } => FilterParams::AddNoise { amount: clamp_or(amount, 0.1, 400.0, 10.0), gaussian, monochromatic, seed },
            FilterParams::LensCorrection { distortion } => FilterParams::LensCorrection { distortion: clamp_or(distortion, -100.0, 100.0, 0.0) },
        }
    }
    /// Whether applying this would change nothing.
    pub fn is_identity(&self) -> bool {
        match self.normalized() { FilterParams::LensCorrection { distortion } => distortion == 0.0, _ => false }
    }
    /// The room the filter needs around the layer, in layer pixels: about three standard
    /// deviations, or half a streak.
    pub fn margin(&self) -> f64 {
        match self.normalized() {
            FilterParams::GaussianBlur { radius } => radius * 3.0 + 2.0,
            FilterParams::MotionBlur { distance, .. } => distance / 2.0 + 2.0,
            _ => 0.0,
        }
    }
    pub fn spreads(&self) -> bool { self.margin() > 0.0 }
    /// A preview rendered from a raster reduced by `factor` blurs proportionally less. Noise and
    /// lens correction are relative to the raster's own size, so they do not scale.
    pub fn scaled(&self, factor: f64) -> FilterParams {
        match *self {
            FilterParams::GaussianBlur { radius } => FilterParams::GaussianBlur { radius: radius * factor },
            FilterParams::MotionBlur { angle, distance } => FilterParams::MotionBlur { angle, distance: distance * factor },
            other => other,
        }
    }
}

/// Bilinear sample in premultiplied bytes; zero outside the raster, so a blur fades at the edge
/// instead of smearing the border outwards.
fn sample_zero(raster: &Raster, x: f64, y: f64) -> [f32; 4] {
    let (w, h) = (raster.width as i64, raster.height as i64);
    let fetch = |px: i64, py: i64| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h { return [0.0; 4]; }
        let p = raster.pixel(px as u32, py as u32);
        [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]
    };
    let fx = x - 0.5; let fy = y - 0.5;
    let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    let (a, b, c, d) = (fetch(xu, yu), fetch(xu + 1, yu), fetch(xu, yu + 1), fetch(xu + 1, yu + 1));
    let mut out = [0f32; 4];
    for i in 0..4 { out[i] = (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty; }
    out
}

/// Separable Gaussian on premultiplied channels, transparent beyond the raster.
pub fn gaussian_blur(raster: &Raster, sigma: f64) -> Raster {
    if !(sigma > 0.0) || raster.width == 0 || raster.height == 0 { return raster.clone(); }
    let radius = (sigma * 3.0).ceil() as i64;
    let kernel: Vec<f32> = (-radius..=radius).map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let (w, h) = (raster.width as i64, raster.height as i64);
    let src = raster.bytes();
    let mut tmp = vec![0f32; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sx = x + k as i64 - radius;
            if sx < 0 || sx >= w { continue; }
            let i = ((y * w + sx) * 4) as usize;
            for c in 0..4 { acc[c] += src[i + c] as f32 * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { tmp[i + c] = acc[c] / sum; }
    }}
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sy = y + k as i64 - radius;
            if sy < 0 || sy >= h { continue; }
            let i = ((sy * w + x) * 4) as usize;
            for c in 0..4 { acc[c] += tmp[i + c] * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        let alpha = (acc[3] / sum).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        // Premultiplied colour can never exceed alpha, or the result reads as over-bright.
        for c in 0..3 { out[i + c] = (acc[c] / sum).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(raster.width, raster.height, out)
}

/// An even streak of `distance` pixels along `angle` degrees, counter-clockwise from horizontal on
/// screen (so the direction in top-down pixels is (cos a, -sin a)), as Photoshop smears.
pub fn motion_blur(raster: &Raster, angle: f64, distance: f64) -> Raster {
    let steps = distance.round().max(1.0) as i64;
    if steps <= 1 { return raster.clone(); }
    let radians = angle.to_radians();
    let (dx, dy) = (radians.cos(), -radians.sin());
    let half = (steps - 1) as f64 / 2.0;
    let (w, h) = (raster.width, raster.height);
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for i in 0..steps {
            let t = i as f64 - half;
            let s = sample_zero(raster, x as f64 + 0.5 + dx * t, y as f64 + 0.5 + dy * t);
            for c in 0..4 { acc[c] += s[c]; }
        }
        let i = ((y * w + x) * 4) as usize;
        let alpha = (acc[3] / steps as f32).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        for c in 0..3 { out[i + c] = (acc[c] / steps as f32).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(w, h, out)
}

fn noise_hash(mut x: u32) -> u32 {
    x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}
fn noise_unit(key: u32) -> f32 { (noise_hash(key) >> 8) as f32 * (1.0 / 16_777_216.0) }

/// The per-pixel key of NoisePixels.c:32: the field position hashed, not the pixel's index (Mac
/// 1.2.6; the older kernel hashed y * width + x, R 5.4).
pub fn noise_base(px: u32, py: u32, seed: u32) -> u32 {
    noise_hash(seed ^ noise_hash(px.wrapping_mul(0x9e37_79b9) ^ noise_hash(py.wrapping_mul(0x85eb_ca6b))))
}

/// The noise added to one channel (NoisePixels.c:33-42), before clamping; `spread` is amount / 100 * 127.5.
pub fn noise_offset(base: u32, channel: usize, spread: f32, gaussian: bool, monochromatic: bool) -> f32 {
    let key = if monochromatic { base } else { base.wrapping_add((channel as u32).wrapping_mul(0x9e37_79b9)) };
    if gaussian {
        // Box-Muller: two uniform values make one normally distributed one.
        let (u1, u2) = (noise_unit(key), noise_unit(key ^ 0x68e3_1da4));
        (-2.0 * (1.0 - u1).ln()).sqrt() * (6.2831853 * u2).cos() * spread * (2.0 / 3.0)
    } else {
        (noise_unit(key) * 2.0 - 1.0) * spread
    }
}

/// One premultiplied pixel through `noise_add_at` (NoisePixels.c:26-47) at field position (px, py).
pub fn noise_pixel(p: [u8; 4], px: u32, py: u32, spread: f32, gaussian: bool, monochromatic: bool, seed: u32) -> [u8; 4] {
    if p[3] == 0 { return p; }
    let alpha = p[3] as f32;
    let base = noise_base(px, py, seed);
    let mut out = p;
    for c in 0..3 {
        let value = (p[c] as f32 * 255.0 / alpha + noise_offset(base, c, spread, gaussian, monochromatic)).clamp(0.0, 255.0);
        out[c] = (value * alpha / 255.0).round() as u8;
    }
    out
}

/// `noise_add_at`: the raster's pixel (x, y) takes the noise at (origin_x + x, origin_y + y).
pub fn add_noise_at(raster: &Raster, amount: f64, gaussian: bool, monochromatic: bool, seed: u32, origin_x: i64, origin_y: i64) -> Raster {
    let spread = amount as f32 / 100.0 * 127.5;
    let mut data = raster.bytes().to_vec();
    let width = raster.width as usize;
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((i % width) as i64, (i / width) as i64);
        let out = noise_pixel([p[0], p[1], p[2], p[3]], (origin_x + x) as u32, (origin_y + y) as u32, spread, gaussian, monochromatic, seed);
        p.copy_from_slice(&out);
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// `noise_add` (NoisePixels.c:15-18): the field in the raster's own grid, origin zero.
pub fn add_noise(raster: &Raster, amount: f64, gaussian: bool, monochromatic: bool, seed: u32) -> Raster {
    add_noise_at(raster, amount, gaussian, monochromatic, seed, 0, 0)
}

/// `lens_distort` from LensPixels.c: `scale = 1 - k r^2 / halfDiagonal^2`, bilinear, transparent outside.
pub fn lens_distort(raster: &Raster, k: f64) -> Raster {
    if k == 0.0 { return raster.clone(); }
    let (w, h) = (raster.width, raster.height);
    let (cx, cy) = (w as f64 * 0.5, h as f64 * 0.5);
    let half_diagonal2 = cx * cx + cy * cy;
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..h { for x in 0..w {
        let dx = x as f64 + 0.5 - cx; let dy = y as f64 + 0.5 - cy;
        let scale = 1.0 - k * (dx * dx + dy * dy) / half_diagonal2;
        let s = sample_zero(raster, cx + dx * scale, cy + dy * scale);
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { out[i + c] = s[c].round().clamp(0.0, 255.0) as u8; }
    }}
    Raster::from_premultiplied(w, h, out)
}

/// The filter at its own scale; callers reduce the raster and pass `params.scaled(factor)`.
pub fn apply_filter(raster: &Raster, params: &FilterParams) -> Raster {
    match params.normalized() {
        FilterParams::GaussianBlur { radius } => gaussian_blur(raster, radius),
        FilterParams::MotionBlur { angle, distance } => motion_blur(raster, angle, distance),
        FilterParams::AddNoise { amount, gaussian, monochromatic, seed } => add_noise(raster, amount, gaussian, monochromatic, seed),
        FilterParams::LensCorrection { distortion } => lens_distort(raster, distortion / 100.0 * LENS_STRENGTH),
    }
}
