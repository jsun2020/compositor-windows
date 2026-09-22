use crate::{GrainSettings, Point, Raster};

fn mix32(mut x: u32) -> u32 {
    x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// A value in -1..1 for an integer lattice point, fixed by the point and the seed. Two uniform
/// halves summed give a triangular spread, closer to film grain than flat noise.
fn lattice(ix: i64, iy: i64, seed: u32) -> f32 {
    let h = mix32((ix as u32).wrapping_mul(0x9E37_79B1) ^ mix32((iy as u32).wrapping_mul(0x85EB_CA77) ^ seed));
    (h & 0xFFFF) as f32 / 65535.0 + (h >> 16) as f32 / 65535.0 - 1.0
}

/// The noise at a document point: a smoothstep-interpolated lattice, mixed with per-pixel fine
/// noise by `roughness`. Document coordinates, so the pattern stays put as the canvas redraws.
pub fn grain_noise(u: f64, v: f64, size: f64, roughness: f64, seed: u32) -> f32 {
    let size = if size > 0.0 { size } else { 1.0 };
    let rough = (roughness / 100.0).clamp(0.0, 1.0) as f32;
    let fine_seed = mix32(seed ^ 0xA511_E9B3);
    let cell_x = (u / size).floor(); let cell_y = (v / size).floor();
    let smoothstep = |t: f32| t * t * (3.0 - 2.0 * t);
    let tx = smoothstep((u / size - cell_x) as f32);
    let ty = smoothstep((v / size - cell_y) as f32);
    let (ix, iy) = (cell_x as i64, cell_y as i64);
    let n00 = lattice(ix, iy, seed); let n10 = lattice(ix + 1, iy, seed);
    let n01 = lattice(ix, iy + 1, seed); let n11 = lattice(ix + 1, iy + 1, seed);
    let top = n00 + (n10 - n00) * tx; let bottom = n01 + (n11 - n01) * tx;
    // Blending neighbours narrows the spread; scaling restores about the lattice's own.
    let smooth = (top + (bottom - top) * ty) * 1.6;
    let fine = lattice(u.floor() as i64, v.floor() as i64, fine_seed);
    smooth + (fine - smooth) * rough
}

pub fn grain_strength(amount: f64) -> f32 { (if amount > 100.0 { 1.0 } else { amount / 100.0 }) as f32 * 0.35 * 255.0 }
/// Film grain shows most in the midtones.
pub fn grain_weight(level: f32) -> f32 { 0.4 + 2.4 * level * (1.0 - level) }

/// `adjust_grain` from AdjustPixels.c. `origin` and `units_per_pixel` place the raster's pixels in
/// document space (a whole layer at 1:1 is origin zero, one unit per pixel).
pub fn apply_grain(raster: &Raster, settings: &GrainSettings, origin: Point, units_per_pixel: f64) -> Raster {
    let s = settings.normalized();
    if !(s.amount > 0.0) || !(units_per_pixel > 0.0) { return raster.clone(); }
    let strength = grain_strength(s.amount);
    let mut data = raster.bytes().to_vec();
    let width = raster.width;
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let a = p[3] as u32;
        if a == 0 { continue; }
        let x = (i as u32 % width) as f64; let y = (i as u32 / width) as f64;
        let u = origin.x + (x + 0.5) * units_per_pixel;
        let v = origin.y + (y + 0.5) * units_per_pixel;
        let noise = grain_noise(u, v, s.size, s.roughness, s.seed);
        let unpremultiply = if a == 255 { 1.0 } else { 255.0 / a as f32 };
        let (r, g, b) = (p[0] as f32 * unpremultiply, p[1] as f32 * unpremultiply, p[2] as f32 * unpremultiply);
        let level = ((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0).min(1.0);
        let delta = noise * strength * grain_weight(level);
        let coverage = a as f32 / 255.0;
        for (c, value) in [r, g, b].iter().enumerate() {
            p[c] = ((value + delta).clamp(0.0, 255.0) * coverage + 0.5) as u8;
        }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
