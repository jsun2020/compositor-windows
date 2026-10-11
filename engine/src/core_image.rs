//! Core Image's independently measured RGBA8 perspective texture filter.
//! It keeps premultiplied CGImage bytes, eight-bit phases and four fractional
//! bits per channel. Camera Raw's straight-colour input path stays separate.
use crate::{GrayRaster, Raster};

fn taps(x: f64, y: f64, width: u32, height: u32) -> Option<(i64, i64, u32, u32)> {
    if !x.is_finite() || !y.is_finite() || x <= -0.5 || y <= -0.5 ||
        x >= width as f64 + 0.5 || y >= height as f64 + 0.5 { return None; }
    let (fx, fy) = ((x as f32 - 0.5) as f64, (y as f32 - 0.5) as f64);
    let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
    Some((ix, iy, ((fx - ix as f64) * 256.0).round() as u32,
        ((fy - iy as f64) * 256.0).round() as u32))
}

fn filtered(a: u32, b: u32, c: u32, d: u32, tx: u32, ty: u32) -> u8 {
    let sum = (a * (256 - tx) + b * tx) * (256 - ty) +
        (c * (256 - tx) + d * tx) * ty;
    let normalized = ((sum + 2048) / 4096) as f32 / 4080.0;
    (normalized as f64 * 255.0).round().clamp(0.0, 255.0) as u8
}

pub(crate) fn sample(raster: &Raster, x: f64, y: f64) -> [u8; 4] {
    let Some((ix, iy, tx, ty)) = taps(x, y, raster.width, raster.height) else { return [0; 4]; };
    let fetch = |sx: i64, sy: i64| {
        if sx < 0 || sy < 0 || sx >= raster.width as i64 || sy >= raster.height as i64 { [0; 4] }
        else { raster.pixel(sx as u32, sy as u32) }
    };
    let (a, b, c, d) = (fetch(ix, iy), fetch(ix + 1, iy), fetch(ix, iy + 1), fetch(ix + 1, iy + 1));
    std::array::from_fn(|k| filtered(a[k] as u32, b[k] as u32, c[k] as u32, d[k] as u32, tx, ty))
}

pub(crate) fn sample_mask(mask: &GrayRaster, x: f64, y: f64) -> u8 {
    let Some((ix, iy, tx, ty)) = taps(x, y, mask.width, mask.height) else { return 0; };
    let fetch = |sx: i64, sy: i64| {
        if sx < 0 || sy < 0 || sx >= mask.width as i64 || sy >= mask.height as i64 { 0 }
        else { mask.bytes()[(sy as u32 * mask.width + sx as u32) as usize] as u32 }
    };
    filtered(fetch(ix, iy), fetch(ix + 1, iy), fetch(ix, iy + 1), fetch(ix + 1, iy + 1), tx, ty)
}
