//! Gaussian and Motion Blur adjustment layers read neighbouring pixels. Both renderers follow the
//! rules here, so they blur the same image: how much of the work runs at full resolution
//! (`spatial_blur`), and the lattice the reduced copies are cut on and how far a render pads
//! (`spatial_grid`, `spatial_span`). The GPU asks for all three through wasm.
use crate::{compositor::sample, gaussian_blur, motion_blur, AdjustmentKind, LayerAdjustment, LayerDraw, PlanNode, Raster, RenderPlan};
use serde::Serialize;

/// Output pixels a Gaussian may reach (3 sigma) at full resolution. A longer reach runs on a copy
/// halved until it fits, then enlarged, which keeps a blur's cost bounded at any radius and zoom.
/// Measured against the exact kernel (Task 4's bounds test): within 1 level in the interior, and
/// up to 4 along the canvas edge and hard alpha edges.
pub const SPATIAL_REACH_LIMIT: f64 = 48.0;
/// Output pixels a Motion Blur may reach (half the streak) at full resolution. Shorter than a
/// Gaussian's: the exact streak costs one bilinear tap per pixel of its length (a 90 px streak
/// over 3000 x 2000 took 45 s in the release wasm), and Motion Blur layers are already drawn
/// approximately (`Document::undrawn`: the Mac's CIMotionBlur tapers, this port's does not).
/// Halving averages detail ACROSS the streak, which the exact one keeps (the bounds test).
pub const MOTION_REACH_LIMIT: f64 = 12.0;
/// The most output pixels a partial render pads each side by for its blurs (`composite_plan`, and
/// the GPU's frame). A render zoomed far into a very large blur shows its edge within this.
pub const SPATIAL_PAD_LIMIT: f64 = 1024.0;

fn level_within(reach: f64, limit: f64) -> u32 {
    let (mut level, mut r) = (0, reach);
    while r > limit && level < 16 { r /= 2.0; level += 1; }
    level
}

/// How many times a Gaussian reaching `reach` output pixels halves its input first.
pub fn spatial_level(reach: f64) -> u32 { level_within(reach, SPATIAL_REACH_LIMIT) }

/// How many times a Motion Blur reaching `reach` output pixels halves its input first.
pub fn motion_level(reach: f64) -> u32 { level_within(reach, MOTION_REACH_LIMIT) }

/// A blur adjustment at `out_per_doc` output pixels per document pixel: its kernel in output
/// pixels (`sigma` for a Gaussian; `distance` and `angle` for a Motion Blur) and how many times
/// its input halves first. The absent settings resolve here (settings.rs:438-440), once.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SpatialBlur { pub level: u32, pub sigma: f64, pub distance: f64, pub angle: f64 }

pub fn spatial_blur(a: &LayerAdjustment, out_per_doc: f64) -> SpatialBlur {
    let (sigma, distance) = match a.kind {
        AdjustmentKind::GaussianBlur => (a.gaussian_radius() * out_per_doc, 0.0),
        AdjustmentKind::MotionBlur => (0.0, a.motion_distance_pixels() * out_per_doc),
        _ => (0.0, 0.0),
    };
    // The reach: 3 sigma for a Gaussian, half the streak for a Motion Blur.
    let level = match a.kind {
        AdjustmentKind::MotionBlur => motion_level(distance / 2.0),
        _ => spatial_level(sigma * 3.0),
    };
    SpatialBlur { level, sigma, distance, angle: a.motion_angle_degrees() }
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

/// The reduced copy back at `width` x `height`: each pixel samples it bilinearly at its own centre,
/// clamped to the edge, as the GPU's mixing pass does. Colour never exceeds alpha.
fn enlarged(small: &Raster, level: u32, width: u32, height: u32) -> Raster {
    let f = (1u32 << level) as f64;
    let mut data = vec![0u8; (width as usize) * (height as usize) * 4];
    for y in 0..height { for x in 0..width {
        let s = sample(small, (x as f64 + 0.5) / f, (y as f64 + 0.5) / f, false);
        let i = ((y * width + x) * 4) as usize;
        let alpha = (s[3] * 255.0).round().clamp(0.0, 255.0);
        data[i + 3] = alpha as u8;
        for c in 0..3 { data[i + c] = (s[c] * 255.0).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(width, height, data)
}

/// A Gaussian of `sigma` output pixels, transparent beyond the raster (Filters.swift:199-201).
pub fn blur_for_layer(raster: &Raster, sigma: f64) -> Raster {
    let level = spatial_level(sigma * 3.0);
    if level == 0 { return gaussian_blur(raster, sigma); }
    let small = gaussian_blur(&reduced(raster, level), sigma / (1u32 << level) as f64);
    enlarged(&small, level, raster.width, raster.height)
}

/// A streak of `distance` output pixels along `angle` degrees (the Phase 3 filter's even streak).
pub fn streak_for_layer(raster: &Raster, angle: f64, distance: f64) -> Raster {
    let level = motion_level(distance / 2.0);
    if level == 0 { return motion_blur(raster, angle, distance); }
    let small = motion_blur(&reduced(raster, level), angle, distance / (1u32 << level) as f64);
    enlarged(&small, level, raster.width, raster.height)
}
