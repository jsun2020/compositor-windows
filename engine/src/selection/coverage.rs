//! A selection as the 8-bit coverage every selection-limited edit blends through: Core Graphics'
//! winding fill of the outline, antialiased when the selection is or has a feather
//! (`DocumentSelection.coverage`, Selection.swift:15-29), then a Gaussian of sigma feather / 2
//! clamped to the region's edge; over the region the selection can reach (`clip(canvas:)`,
//! :39-48); and that coverage on a layer's own pixel grid (`PixelAdjust.coverage`,
//! PixelAdjust.swift:23-34).
use super::{Contour, Selection, SUBPIXEL};
use super::feather::feather_blur;
use crate::{Affine, Document, GrayRaster, Point};

/// One edge of an outline in region pixels, `y0 < y1`, `dir` +1 when the contour runs down.
#[derive(Clone, Copy)]
struct Edge { x0: f64, y0: f64, x1: f64, y1: f64, dir: f32 }

/// The outline's edges in the pixel grid of a `width` x `height` region whose top-left corner is
/// at document pixel (`left`, `top`), horizontal edges dropped.
fn edges(contours: &[Contour], left: f64, top: f64) -> Vec<Edge> {
    let mut out = Vec::new();
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let (ax, ay) = (a[0] as f64 / SUBPIXEL - left, a[1] as f64 / SUBPIXEL - top);
            let (bx, by) = (b[0] as f64 / SUBPIXEL - left, b[1] as f64 / SUBPIXEL - top);
            if ay == by { continue; }
            out.push(if ay < by { Edge { x0: ax, y0: ay, x1: bx, y1: by, dir: 1.0 } } else { Edge { x0: bx, y0: by, x1: ax, y1: ay, dir: -1.0 } });
        }
    }
    out.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    out
}

/// Adds one edge's share of row `y` to `acc` (`width + 2` cells): the signed area it covers to its
/// right, spread over the cells it crosses (the accumulation rasteriser). Cells left of 0 fold into
/// cell 0 and cells right of `width` are dropped: neither changes a pixel inside the row.
fn accumulate(acc: &mut [f32], width: usize, e: &Edge, y: f64) {
    let (top, bottom) = (e.y0.max(y), e.y1.min(y + 1.0));
    if !(bottom > top) { return; }
    let dxdy = (e.x1 - e.x0) / (e.y1 - e.y0);
    // The piece of the edge inside this row, split where it crosses x = 0 and x = width.
    let (xa, xb) = (e.x0 + (top - e.y0) * dxdy, e.x0 + (bottom - e.y0) * dxdy);
    // At most four points, in a fixed array: this runs for every active edge on every row.
    let mut cuts = [(top, xa); 4];
    let mut count = 1;
    for bound in [0.0, width as f64] {
        if (xa - bound) * (xb - bound) < 0.0 { let t = (bound - xa) / (xb - xa); cuts[count] = (top + (bottom - top) * t, bound); count += 1; }
    }
    cuts[count] = (bottom, xb);
    count += 1;
    // In order down the row; a stable insertion sort, so equal heights keep their order.
    for i in 1..count {
        let mut j = i;
        while j > 0 && cuts[j - 1].0.total_cmp(&cuts[j].0) == std::cmp::Ordering::Greater { cuts.swap(j - 1, j); j -= 1; }
    }
    for pair in cuts[..count].windows(2) {
        let ((ya, mut x0), (yb, mut x1)) = (pair[0], pair[1]);
        let dy = yb - ya;
        if !(dy > 0.0) { continue; }
        if (x0 + x1) * 0.5 >= width as f64 { continue; }
        if (x0 + x1) * 0.5 <= 0.0 { x0 = 0.0; x1 = 0.0; }
        let d = (dy as f32) * e.dir;
        let (lo, hi) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (lo_floor, hi_ceil) = (lo.floor(), hi.ceil());
        let (i0, i1) = (lo_floor as usize, hi_ceil as usize);
        if i1 <= i0 + 1 {
            // Within one column: the area right of the piece's middle.
            let xm = (0.5 * (x0 + x1) - lo_floor) as f32;
            acc[i0] += d - d * xm;
            acc[i0 + 1] += d * xm;
        } else {
            let s = (1.0 / (hi - lo)) as f32;
            let f0 = (lo - lo_floor) as f32;
            let a0 = 0.5 * s * (1.0 - f0) * (1.0 - f0);
            let f1 = (hi - hi_ceil + 1.0) as f32;
            let am = 0.5 * s * f1 * f1;
            acc[i0] += d * a0;
            if i1 == i0 + 2 {
                acc[i0 + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = s * (1.5 - f0);
                acc[i0 + 1] += d * (a1 - a0);
                for cell in acc.iter_mut().take(i1 - 1).skip(i0 + 2) { *cell += d * s; }
                let a2 = a1 + (i1 - i0 - 3) as f32 * s;
                acc[i1 - 1] += d * (1.0 - a2 - am);
            }
            acc[i1] += d * am;
        }
    }
}

/// The outline filled into a `width` x `height` region whose top-left corner is document pixel
/// (`left`, `top`), by the nonzero winding rule, 255 inside and 0 outside. `antialiased`: each
/// pixel is the area of it the outline covers (Core Graphics' antialiased fill); else a pixel is
/// covered when its centre is (the aliased fill). Rows one at a time, so it holds one row of f32.
pub fn rasterize(contours: &[Contour], left: f64, top: f64, width: u32, height: u32, antialiased: bool) -> GrayRaster {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; w * h];
    let all = edges(contours, left, top);
    let mut next = 0usize;
    let mut active: Vec<Edge> = Vec::new();
    let mut acc = vec![0f32; w + 2];
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for y in 0..h {
        let (row_top, row_bottom) = (y as f64, y as f64 + 1.0);
        while next < all.len() && all[next].y0 < row_bottom { active.push(all[next]); next += 1; }
        active.retain(|e| e.y1 > row_top);
        let line = &mut out[y * w..(y + 1) * w];
        if antialiased {
            for e in &active { accumulate(&mut acc, w, e, row_top); }
            let mut sum = 0f32;
            // Each cell is read once and cleared for the next row. Rounded half up in f64, where
            // v + 0.5 is exact: the same as f32::round on 0..=255, without its library call.
            for (px, cell) in line.iter_mut().zip(acc.iter_mut()) {
                sum += *cell;
                *cell = 0.0;
                *px = ((sum.abs().min(1.0) * 255.0) as f64 + 0.5) as u8;
            }
            acc[w] = 0.0;
            acc[w + 1] = 0.0;
        } else {
            // Nonzero winding at the row's centre line, half-open at each edge's ends.
            let yc = y as f64 + 0.5;
            crossings.clear();
            for e in &active {
                if e.y0 <= yc && yc < e.y1 { crossings.push((e.x0 + (yc - e.y0) * (e.x1 - e.x0) / (e.y1 - e.y0), e.dir as i32)); }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for i in 0..crossings.len() {
                winding += crossings[i].1;
                if winding == 0 || i + 1 == crossings.len() { continue; }
                // Pixels whose centres x + 0.5 lie in [this crossing, the next).
                let from = (crossings[i].0 - 0.5).ceil().max(0.0) as usize;
                let to = ((crossings[i + 1].0 - 0.5).ceil().max(0.0) as usize).min(w);
                for px in line.iter_mut().take(to).skip(from) { *px = 255; }
            }
        }
    }
    GrayRaster::from_bytes(width, height, out)
}

/// The selection's coverage over the part of the canvas it can reach: `clip(canvas:)`. `origin`
/// is the region's top-left document pixel; `coverage` None is an empty selection, which clips
/// every edit away.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionClip { pub origin: (i64, i64), pub coverage: Option<GrayRaster> }

impl SelectionClip {
    /// The coverage bounds grown by a pixel, rounded out and cut to the canvas; there, the outline
    /// filled (antialiased when the selection is, or has a feather) and blurred by sigma feather / 2
    /// with the region's edge pixels repeated beyond it (`clampedToExtent`).
    pub fn new(selection: &Selection, canvas_width: u32, canvas_height: u32) -> SelectionClip {
        let none = SelectionClip { origin: (0, 0), coverage: None };
        if selection.is_empty() { return none; }
        let Some(b) = selection.coverage_bounds() else { return none };
        let (x0, y0) = (((b.x - 1.0).floor()).max(0.0), ((b.y - 1.0).floor()).max(0.0));
        let (x1, y1) = (((b.max_x() + 1.0).ceil()).min(canvas_width as f64), ((b.max_y() + 1.0).ceil()).min(canvas_height as f64));
        if !(x1 - x0 >= 1.0) || !(y1 - y0 >= 1.0) { return none; }
        let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
        let filled = rasterize(&selection.contours, x0, y0, w, h, selection.antialiased || selection.feather > 0.0);
        let coverage = if selection.feather > 0.0 { feather_blur(&filled, selection.feather / 2.0) } else { filled };
        SelectionClip { origin: (x0 as i64, y0 as i64), coverage: Some(coverage) }
    }

    /// The coverage at a document point: 0 outside the region, else sampled bilinearly between the
    /// region's pixel centres, its edge pixels repeated.
    pub fn at(&self, p: Point) -> f32 {
        let Some(c) = &self.coverage else { return 0.0 };
        let (x, y) = (p.x - self.origin.0 as f64, p.y - self.origin.1 as f64);
        if x < 0.0 || y < 0.0 || x >= c.width as f64 || y >= c.height as f64 { return 0.0; }
        crate::compositor::gray_sample(c, x, y, false)
    }

    /// The coverage on a `width` x `height` pixel grid that `to_document` places on the canvas (a
    /// layer's pixels or its mask's, `pixel_to_document`), each pixel sampled at its centre
    /// (`PixelAdjust.coverage`). A grid on the canvas's own pixels, moved by whole pixels, copies
    /// them exactly.
    pub fn on_grid(&self, to_document: &Affine, width: u32, height: u32) -> GrayRaster {
        let (w, h) = (width as usize, height as usize);
        let mut out = vec![0u8; w * h];
        let Some(c) = &self.coverage else { return GrayRaster::from_bytes(width, height, out) };
        let whole = |v: f64| v.fract() == 0.0;
        if to_document.a == 1.0 && to_document.d == 1.0 && to_document.b == 0.0 && to_document.c == 0.0 && whole(to_document.tx) && whole(to_document.ty) {
            let (dx, dy) = (to_document.tx as i64 - self.origin.0, to_document.ty as i64 - self.origin.1);
            let (cw, ch) = (c.width as i64, c.height as i64);
            for y in 0..h as i64 {
                let sy = y + dy;
                if sy < 0 || sy >= ch { continue; }
                let (x0, x1) = ((-dx).max(0), (cw - dx).min(w as i64));
                if x0 >= x1 { continue; }
                let src = &c.bytes()[(sy * cw + x0 + dx) as usize..(sy * cw + x1 + dx) as usize];
                out[(y * w as i64 + x0) as usize..(y * w as i64 + x1) as usize].copy_from_slice(src);
            }
        } else {
            for y in 0..h { for x in 0..w {
                let p = to_document.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
                out[y * w + x] = (self.at(p) * 255.0).round().clamp(0.0, 255.0) as u8;
            }}
        }
        GrayRaster::from_bytes(width, height, out)
    }
}

/// The document's selection on a pixel grid (`SelectionClip::on_grid`): None when nothing is
/// selected (an edit then reaches the whole grid), all zero for an empty selection.
pub fn selection_coverage(doc: &Document, to_document: &Affine, width: u32, height: u32) -> Option<GrayRaster> {
    let selection = doc.selection.as_ref()?;
    Some(SelectionClip::new(selection, doc.width, doc.height).on_grid(to_document, width, height))
}
