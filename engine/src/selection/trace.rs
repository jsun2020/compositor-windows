//! Pixels back into outlines: `wand_trace` (Compositor for Mac's WandPixels.c:87-174), line for
//! line, and the Mac's `MaskTracing` (Document/MaskTracing.swift:4-70), which traces a layer's
//! opaque pixels or a mask's dark ones by the same pixel-edge rule. One tracer serves both here.
use super::{Contour, SUBPIXEL};
use crate::{GrayRaster, Raster};

const EAST: u8 = 1;
const SOUTH: u8 = 2;
const WEST: u8 = 4;
const NORTH: u8 = 8;
/// Outlines with more pixel edges than this are refused: the path would be too slow to draw
/// (`wand_edge_limit`, WandPixels.c:7).
pub const WAND_EDGE_LIMIT: usize = 8_000_000;

/// Why a trace gave no outline (`wand_trace` returns -2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceError { TooDetailed }

// Headings, clockwise on screen (y grows downward): east, south, west, north.
fn turn_right(d: u8) -> u8 { if d == NORTH { EAST } else { d << 1 } }
fn turn_left(d: u8) -> u8 { if d == EAST { NORTH } else { d >> 1 } }
fn lowest(bits: u8) -> u8 { bits & bits.wrapping_neg() }

/// The outline of `mask`'s nonzero pixels along exact pixel edges, as corner points in pixel
/// units: one loop per boundary, outer boundaries clockwise on screen and holes the other way, so
/// the winding rule fills exactly the traced pixels. Where two loops touch at a corner the walk
/// turns right, which keeps them apart. `wand_trace`, WandPixels.c:91-174.
pub fn trace_pixels(mask: &[u8], width: usize, height: usize) -> Result<Vec<Vec<[i32; 2]>>, TraceError> {
    if width == 0 || height == 0 { return Ok(Vec::new()); }
    // Each vertex of the (width + 1) x (height + 1) grid records the directed boundary edges
    // leaving it: a selected pixel's unselected sides, walked clockwise around the pixel.
    let stride = width + 1;
    let vertices = stride * (height + 1);
    let mut out = vec![0u8; vertices];
    let mut edges = 0usize;
    for y in 0..height {
        let row = &mask[y * width..(y + 1) * width];
        for x in 0..width {
            if row[x] == 0 { continue; }
            if y == 0 || mask[(y - 1) * width + x] == 0 { out[y * stride + x] |= EAST; edges += 1; }
            if x + 1 == width || row[x + 1] == 0 { out[y * stride + x + 1] |= SOUTH; edges += 1; }
            if y + 1 == height || mask[(y + 1) * width + x] == 0 { out[(y + 1) * stride + x + 1] |= WEST; edges += 1; }
            if x == 0 || row[x - 1] == 0 { out[(y + 1) * stride + x] |= NORTH; edges += 1; }
        }
        if edges > WAND_EDGE_LIMIT { return Err(TraceError::TooDetailed); }
    }
    let mut loops = Vec::new();
    for start in 0..vertices {
        while out[start] != 0 {
            let mut points: Vec<[i32; 2]> = Vec::new();
            let (mut v, mut heading, mut initial) = (start, 0u8, 0u8);
            loop {
                let bits = out[v];
                let d = if heading == 0 { lowest(bits) }
                    else if bits & turn_right(heading) != 0 { turn_right(heading) }
                    else if bits & heading != 0 { heading }
                    else if bits & turn_left(heading) != 0 { turn_left(heading) }
                    else { lowest(bits) };
                if d == 0 { break; }
                out[v] &= !d;
                if d != heading { points.push([(v % stride) as i32, (v / stride) as i32]); }
                if heading == 0 { initial = d; }
                heading = d;
                v = match d { EAST => v + 1, WEST => v - 1, SOUTH => v + stride, _ => v - stride };
                if v == start { break; }
            }
            // The start is a corner unless the loop arrives on the heading it left with.
            if heading == initial && !points.is_empty() { points.remove(0); }
            loops.push(points);
        }
    }
    Ok(loops)
}

/// Pixel-unit loops as selection contours, `SUBPIXEL` units, pixel (0, 0)'s corner at the origin.
pub fn to_contours(loops: Vec<Vec<[i32; 2]>>) -> Vec<Contour> {
    let s = SUBPIXEL as i32;
    loops.into_iter().filter(|l| l.len() >= 3).map(|l| l.into_iter().map(|p| [p[0] * s, p[1] * s]).collect()).collect()
}

/// A layer's pixels at least 50% opaque, traced (`MaskTracing.opaquePixels`): alpha 128 and up.
pub fn opaque_pixels(raster: &Raster) -> Result<Vec<Contour>, TraceError> {
    let mask: Vec<u8> = raster.bytes().chunks_exact(4).map(|p| (p[3] >= 128) as u8).collect();
    Ok(to_contours(trace_pixels(&mask, raster.width as usize, raster.height as usize)?))
}

/// A mask's pixels darker than 50% grey, traced (`MaskTracing.darkPixels`): below 128.
pub fn dark_pixels(mask: &GrayRaster) -> Result<Vec<Contour>, TraceError> {
    let dark: Vec<u8> = mask.bytes().iter().map(|&v| (v < 128) as u8).collect();
    Ok(to_contours(trace_pixels(&dark, mask.width as usize, mask.height as usize)?))
}
