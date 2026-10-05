//! Gaussian and Motion Blur adjustment layers read neighbouring pixels. Both renderers follow the
//! rules here, so they blur the same image: how much of the work runs at full resolution
//! (`spatial_blur`), and the lattice the reduced copies are cut on and how far a render pads
//! (`spatial_grid`, `spatial_span`). The GPU asks for all three through wasm.
use crate::{gaussian_blur, gaussian_blur_in_place, motion_blur, motion_reach, motion_sigma, AdjustmentKind, LayerAdjustment, LayerDraw, PlanNode, Raster, RenderPlan};
use serde::Serialize;

/// Output pixels a blur may reach (3 sigma) at full resolution, a Gaussian or a Motion Blur (since
/// Phase 4a, when the Motion Blur became a Gaussian along its angle run as a row-wise stencil,
/// `motion_blur`; the even streak before it halved past 12). A longer reach runs on a copy
/// halved until it fits, then enlarged, which keeps a blur's cost bounded at any radius and zoom.
/// Measured against the exact kernel (Task 4's bounds test): within 1 level in the interior, and
/// up to 4 along the canvas edge and hard alpha edges.
pub const SPATIAL_REACH_LIMIT: f64 = 48.0;
/// The most output pixels a partial render pads each side by for its blurs (`composite_plan`, and
/// the GPU's frame). A render zoomed far into a very large blur shows its edge within this.
pub const SPATIAL_PAD_LIMIT: f64 = 1024.0;
/// The largest lattice cell, in output pixels: a blur halves at most 8 times (2^8 = 256), however
/// far it reaches; past that its reduced kernel runs a longer reach instead. This keeps a padded
/// frame within the view plus `2 * (SPATIAL_PAD_LIMIT + SPATIAL_CELL_LIMIT)` on each axis at any
/// zoom. A reach that needs more halvings already exceeds SPATIAL_PAD_LIMIT, so such a frame is
/// approximate anyway.
pub const SPATIAL_CELL_LIMIT: f64 = 256.0;

fn level_within(reach: f64, limit: f64) -> u32 {
    let (mut level, mut r) = (0, reach);
    // `2 << level` is the cell one more halving would make.
    while r > limit && (2u32 << level) as f64 <= SPATIAL_CELL_LIMIT { r /= 2.0; level += 1; }
    level
}

/// How many times a blur reaching `reach` output pixels (three sigmas, for a Gaussian and a Motion
/// Blur alike) halves its input first.
pub fn spatial_level(reach: f64) -> u32 { level_within(reach, SPATIAL_REACH_LIMIT) }

/// A blur adjustment at `out_per_doc` output pixels per document pixel: its kernel in output
/// pixels (`sigma` for both kinds, and a Motion Blur's `distance` and `angle`) and how many times
/// its input halves first. The absent settings resolve here (settings.rs:438-440), once.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SpatialBlur { pub level: u32, pub sigma: f64, pub distance: f64, pub angle: f64 }

pub fn spatial_blur(a: &LayerAdjustment, out_per_doc: f64) -> SpatialBlur {
    let (sigma, distance) = match a.kind {
        AdjustmentKind::GaussianBlur => (a.gaussian_radius() * out_per_doc, 0.0),
        AdjustmentKind::MotionBlur => { let d = a.motion_distance_pixels() * out_per_doc; (motion_sigma(d), d) }
        _ => (0.0, 0.0),
    };
    // Both kernels reach three sigmas, and both halve past SPATIAL_REACH_LIMIT.
    SpatialBlur { level: spatial_level(sigma * 3.0), sigma, distance, angle: a.motion_angle_degrees() }
}

/// Every Gaussian or Motion Blur adjustment the plan draws, bottom to top: plain nodes, and the
/// children of clipping stacks.
pub fn spatial_blurs(plan: &RenderPlan) -> Vec<&LayerAdjustment> {
    let mut out = Vec::new();
    for node in &plan.nodes {
        let draws: Vec<&LayerDraw> = match node {
            PlanNode::Layer { draw } => vec![draw],
            PlanNode::Stack { base, children, .. } => std::iter::once(base).chain(children.iter()).collect(),
        };
        out.extend(draws.into_iter().filter_map(|d| d.adjustment.as_ref()).filter(|a| a.kind.is_spatial()));
    }
    out
}

/// The lattice a render's reduced copies are cut on, and how far a render pads, for a plan at
/// `out_per_doc` output pixels per document pixel.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SpatialGrid {
    /// Output pixels per cell: `2^(the largest spatial_blur level among the plan's blurs)`. The lattice
    /// is anchored at the canvas's top-left corner, so every render halves the same blocks.
    pub cell: u32,
    /// Output pixels a render grows its span by on each side: each blur's `sampling_margin` plus
    /// three of its own cells (the 2 x 2 box and the bilinear enlargement read about 1.5 cells
    /// beyond the kernel), summed because stacked blurs compound; at most `SPATIAL_PAD_LIMIT`.
    pub pad: u32,
}

pub fn spatial_grid(plan: &RenderPlan, out_per_doc: f64) -> SpatialGrid {
    let (mut cell, mut pad) = (1u32, 0.0);
    for a in spatial_blurs(plan) {
        let own = 1u32 << spatial_blur(a, out_per_doc).level;
        cell = cell.max(own);
        pad += a.sampling_margin() * out_per_doc + 3.0 * own as f64;
    }
    SpatialGrid { cell, pad: pad.ceil().min(SPATIAL_PAD_LIMIT) as u32 }
}

/// One axis of the output pixels a render holds, as a half-open span measured from the canvas's
/// leading edge (left or top), for a request spanning `near..far` on a canvas `canvas` long:
/// grown by `pad` on each side, never past the canvas's own span rounded out to the lattice
/// (`0..ceil(canvas / cell) * cell`; the part beyond the canvas is transparent), with both ends
/// on the lattice. A request that already starts before the canvas or ends after it keeps that
/// end, rounded out. `composite_plan` and the GPU's frame both call this.
pub fn spatial_span(near: i64, far: i64, canvas: i64, cell: u32, pad: u32) -> (i64, i64) {
    let (cell, pad) = (cell.max(1) as i64, pad as i64);
    let floor = |v: i64| v.div_euclid(cell) * cell;
    let ceil = |v: i64| -(-v).div_euclid(cell) * cell;
    let limit = ceil(canvas.max(0));
    let start = if near > 0 { floor((near - pad).max(0)) } else { floor(near) };
    let end = if far < limit { ceil((far + pad).min(limit)) } else { ceil(far) };
    (start, end)
}

fn reduced(raster: &Raster, level: u32) -> Raster {
    let mut r = raster.clone();
    for _ in 0..level { r = r.halved(); }
    r
}

/// The reduced copy back at the size of `full`, written over `full`'s own pixels (the reduced copy
/// no longer needs them): each pixel samples it bilinearly at its own centre, clamped to the edge,
/// as the GPU's mixing pass does. Colour never exceeds alpha.
fn enlarged(small: &Raster, level: u32, full: Raster) -> Raster {
    let (width, height) = (full.width, full.height);
    let f = (1u32 << level) as f64;
    // Every output row samples the same columns. Keep only the two normalized
    // source rows it needs, instead of repeating clamping and sixteen channel
    // conversions for every pixel or retaining a full frame of float pixels.
    // Keep sample()'s unquantized interpolation and operation order exactly.
    let sw = small.width as usize;
    let mut rows = [vec![[0f32; 4]; sw], vec![[0f32; 4]; sw]];
    let mut row_numbers = [None, None];
    let columns: Vec<(usize, usize, f32)> = (0..width).map(|x| {
        let fx = (x as f64 + 0.5) / f - 0.5;
        let xu = fx.floor() as i64;
        (xu.clamp(0, small.width as i64 - 1) as usize,
         (xu + 1).clamp(0, small.width as i64 - 1) as usize,
         (fx - xu as f64) as f32)
    }).collect();
    let mut data = full.into_bytes();
    for y in 0..height {
        let fy = (y as f64 + 0.5) / f - 0.5;
        let yu = fy.floor() as i64;
        let ty = (fy - yu as f64) as f32;
        let y0 = yu.clamp(0, small.height as i64 - 1) as usize;
        let y1 = (yu + 1).clamp(0, small.height as i64 - 1) as usize;
        // Consecutive source rows use distinct slots; at a clamped edge both
        // samples reuse the same slot. A row is decoded only when it changes.
        for sy in [y0, y1] {
            let slot = sy & 1;
            if row_numbers[slot] != Some(sy) {
                for (out, p) in rows[slot].iter_mut().zip(small.bytes()[sy * sw * 4..(sy + 1) * sw * 4].chunks_exact(4)) {
                    *out = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0];
                }
                row_numbers[slot] = Some(sy);
            }
        }
        let row0 = &rows[y0 & 1];
        let row1 = &rows[y1 & 1];
        for (x, &(x0, x1, tx)) in columns.iter().enumerate() {
            let (a, b, c, d) = (row0[x0], row0[x1], row1[x0], row1[x1]);
            let s: [f32; 4] = std::array::from_fn(|i|
                (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty);
            let i = (y as usize * width as usize + x) * 4;
            let alpha = (s[3] * 255.0).round().clamp(0.0, 255.0);
            data[i + 3] = alpha as u8;
            for c in 0..3 { data[i + c] = (s[c] * 255.0).round().clamp(0.0, alpha) as u8; }
        }
    }
    Raster::from_premultiplied(width, height, data)
}

/// A Gaussian of `sigma` output pixels, transparent beyond the raster (Filters.swift:199-201).
/// Takes the raster to blur its pixels in place: a canvas-size render holds no second full frame.
pub fn blur_for_layer(raster: Raster, sigma: f64) -> Raster {
    let level = spatial_level(sigma * 3.0);
    if level == 0 {
        let (width, height) = (raster.width, raster.height);
        let mut data = raster.into_bytes();
        gaussian_blur_in_place(&mut data, width, height, sigma);
        return Raster::from_premultiplied(width, height, data);
    }
    let small = gaussian_blur(&reduced(&raster, level), sigma / (1u32 << level) as f64);
    enlarged(&small, level, raster)
}

/// A Motion Blur of `distance` output pixels along `angle` degrees: the destructive filter's kernel
/// (`motion_blur`), on a copy halved `spatial_level(motion_reach(distance))` times.
pub fn streak_for_layer(raster: Raster, angle: f64, distance: f64) -> Raster {
    let level = spatial_level(motion_reach(distance));
    if level == 0 { return motion_blur(&raster, angle, distance); }
    let small = motion_blur(&reduced(&raster, level), angle, distance / (1u32 << level) as f64);
    enlarged(&small, level, raster)
}
