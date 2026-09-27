//! The outline the marching ants draw (TransformOverlay.swift:92-177): the selection's own
//! contours, or, for a complex outline seen below 1:1, one traced at screen resolution so a
//! redraw never strokes more edges than the screen has pixels for.
use super::coverage::rasterize;
use super::trace::trace_pixels;
use super::{Selection, SUBPIXEL};
use crate::Point;

/// Outlines with more points than this are sent as they are only at 1:1 and closer
/// (`fullDetailLimit`, TransformOverlay.swift:100, counts path elements).
pub const OUTLINE_DETAIL_LIMIT: usize = 20_000;
/// The largest mask a screen-resolution outline is traced from (about 40 megapixels, as the Mac's).
pub const OUTLINE_MASK_LIMIT: u64 = 40_000_000;

/// The selection traced at `step` screen pixels per document pixel (a power of two below 1): its
/// outline filled into a mask that fine, over its bounds cut to the canvas, any coverage counting,
/// and traced along that mask's pixel edges, in document pixels. `traceOutline`,
/// TransformOverlay.swift:141-177. A mask over `OUTLINE_MASK_LIMIT`, or a trace over
/// `WAND_EDGE_LIMIT`, is made at half the step instead.
pub fn selection_lod(selection: &Selection, canvas_width: u32, canvas_height: u32, step: f64) -> Vec<Vec<Point>> {
    let Some(b) = selection.bounds() else { return Vec::new() };
    let (x0, y0) = (b.x.floor().max(0.0), b.y.floor().max(0.0));
    let (x1, y1) = (b.max_x().ceil().min(canvas_width as f64), b.max_y().ceil().min(canvas_height as f64));
    if !(x1 - x0 >= 1.0 && y1 - y0 >= 1.0) { return Vec::new(); }
    let mut step = step.clamp(1.0 / 4096.0, 1.0);
    loop {
        let (w, h) = (((x1 - x0) * step).ceil().max(1.0) as u32, ((y1 - y0) * step).ceil().max(1.0) as u32);
        if (w as u64) * (h as u64) > OUTLINE_MASK_LIMIT && step > 1.0 / 4096.0 { step /= 2.0; continue; }
        // The contours in the mask's pixels: moved to the region's corner, scaled by the step.
        let scaled: Vec<_> = selection.contours.iter().map(|c| c.iter().map(|p| [
            ((p[0] as f64 - x0 * SUBPIXEL) * step).round() as i32, ((p[1] as f64 - y0 * SUBPIXEL) * step).round() as i32,
        ]).collect()).collect();
        let filled = rasterize(&scaled, 0.0, 0.0, w, h, true);
        let mask: Vec<u8> = filled.bytes().iter().map(|&v| (v > 0) as u8).collect();
        match trace_pixels(&mask, w as usize, h as usize) {
            Ok(loops) => return loops.into_iter().map(|l| l.into_iter().map(|q| Point { x: x0 + q[0] as f64 / step, y: y0 + q[1] as f64 / step }).collect()).collect(),
            Err(_) if step > 1.0 / 4096.0 => step /= 2.0,
            Err(_) => return Vec::new(),
        }
    }
}
