//! The Magic Wand's matcher: `wand_mask` (Compositor for Mac's WandPixels.c:9-85), line for line,
//! over premultiplied RGBA, and `MagicWand.select` (Document/MagicWand.swift:36-52) around it.
use super::trace::{to_contours, trace_pixels, TraceError};
use super::Contour;
use crate::{Point, Raster};
use serde::{Deserialize, Serialize};

/// The Magic Wand's options (`WandSettings`, MagicWand.swift:11-19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WandSettings {
    /// How far (0 to 255) every channel, alpha included, may differ from the sampled colour.
    pub tolerance: u32,
    /// Pixels either side of the click averaged into the colour to match: 0 (Point), 1 (3 by 3), 2 (5 by 5).
    pub sample_radius: u32,
    /// Only matching pixels connected to the clicked one.
    pub contiguous: bool,
    /// Read the visible composite rather than the active layer's own pixels.
    pub all_layers: bool,
}

impl Default for WandSettings {
    fn default() -> WandSettings { WandSettings { tolerance: 32, sample_radius: 0, contiguous: true, all_layers: false } }
}

fn matches(p: &[u8], reference: &[i32; 4], tolerance: i32) -> bool {
    (0..4).all(|c| { let d = p[c] as i32 - reference[c]; d >= -tolerance && d <= tolerance })
}

/// 255 where a pixel matches the colour averaged over the (2 radius + 1)^2 pixels around the seed
/// (rounded, each channel), within `tolerance` on every channel; with `contiguous`, only pixels
/// 4-connected to the seed, by a scanline flood fill. Returns the mask and its count of matches.
/// `wand_mask`, WandPixels.c:17-85.
pub fn wand_mask(rgba: &[u8], width: usize, height: usize, seed_x: usize, seed_y: usize, radius: usize, tolerance: i32, contiguous: bool) -> (Vec<u8>, usize) {
    let mut mask = vec![0u8; width * height];
    if width == 0 || height == 0 || seed_x >= width || seed_y >= height { return (mask, 0); }
    let stride = width * 4;
    let (x0, x1) = (seed_x.saturating_sub(radius), (seed_x + radius).min(width - 1));
    let (y0, y1) = (seed_y.saturating_sub(radius), (seed_y + radius).min(height - 1));
    let (mut sums, mut samples) = ([0u64; 4], 0u64);
    for y in y0..=y1 { for x in x0..=x1 {
        for c in 0..4 { sums[c] += rgba[y * stride + x * 4 + c] as u64; }
        samples += 1;
    }}
    let reference = [0, 1, 2, 3].map(|c| ((sums[c] + samples / 2) / samples) as i32);
    let at = |x: usize, y: usize| &rgba[y * stride + x * 4..y * stride + x * 4 + 4];
    let mut count = 0usize;
    if !contiguous {
        for y in 0..height { for x in 0..width {
            if matches(at(x, y), &reference, tolerance) { mask[y * width + x] = 255; count += 1; }
        }}
        return (mask, count);
    }
    // Scanline flood fill: each popped seed fills its whole horizontal run, then pushes one seed
    // per matching run in the rows directly above and below it.
    let mut stack = vec![(seed_x, seed_y)];
    while let Some((x, y)) = stack.pop() {
        if mask[y * width + x] != 0 || !matches(at(x, y), &reference, tolerance) { continue; }
        let (mut left, mut right) = (x, x);
        while left > 0 && mask[y * width + left - 1] == 0 && matches(at(left - 1, y), &reference, tolerance) { left -= 1; }
        while right + 1 < width && mask[y * width + right + 1] == 0 && matches(at(right + 1, y), &reference, tolerance) { right += 1; }
        mask[y * width + left..=y * width + right].fill(255);
        count += right - left + 1;
        for side in 0..2 {
            if if side == 0 { y == 0 } else { y + 1 >= height } { continue; }
            let ny = if side == 0 { y - 1 } else { y + 1 };
            let mut in_run = false;
            for nx in left..=right {
                let candidate = mask[ny * width + nx] == 0 && matches(at(nx, ny), &reference, tolerance);
                if candidate && !in_run { stack.push((nx, ny)); }
                in_run = candidate;
            }
        }
    }
    (mask, count)
}

/// The outline, in `SUBPIXEL` units of the image's pixels, of the pixels matching the one at
/// `point`; None when the point lies outside the image or nothing matches (`MagicWand.select`).
pub fn magic_wand(image: &Raster, point: Point, settings: &WandSettings) -> Result<Option<Vec<Contour>>, TraceError> {
    if !point.x.is_finite() || !point.y.is_finite() || point.x < 0.0 || point.y < 0.0 { return Ok(None); }
    let (x, y) = (point.x.floor() as usize, point.y.floor() as usize);
    let (w, h) = (image.width as usize, image.height as usize);
    if x >= w || y >= h { return Ok(None); }
    let (mask, count) = wand_mask(image.bytes(), w, h, x, y, settings.sample_radius as usize, settings.tolerance.min(255) as i32, settings.contiguous);
    if count == 0 { return Ok(None); }
    Ok(Some(to_contours(trace_pixels(&mask, w, h)?)))
}
