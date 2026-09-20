#![allow(dead_code)]
use compositor_engine::*;

/// 64x32: left half opaque red, right half transparent. Same shape as the macOS test fixture.
pub fn red_left_raster() -> Raster {
    let mut data = vec![0u8; 64 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let i = (y * 64 + x) * 4;
            data[i] = 255; data[i + 3] = 255;
        }
    }
    Raster::from_premultiplied(64, 32, data)
}

pub fn red_left_png() -> Vec<u8> { encode_png(&red_left_raster(), 72.0).unwrap() }

/// Opaque noise-like pattern with distinct neighbours, for JPEG quality comparisons.
pub fn pattern_raster(w: u32, h: u32) -> Raster {
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let i = ((y * w + x) * 4) as usize;
        data[i] = ((x * 37 + y * 17) % 256) as u8;
        data[i + 1] = ((x * 11 + y * 53) % 256) as u8;
        data[i + 2] = ((x * 79 + y * 7) % 256) as u8;
        data[i + 3] = 255;
    }}
    Raster::from_premultiplied(w, h, data)
}
