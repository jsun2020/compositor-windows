//! The feather's Gaussian (`DocumentSelection.coverage`, Selection.swift:24-29): sigma = feather / 2
//! over the coverage region, the region's edge pixels repeated beyond it (`clampedToExtent`).
//!
//! A direct convolution costs about 6 sigma + 1 taps a pixel a pass, which made a feather of 63 on
//! a 24-megapixel canvas take seconds (Phase 4a final review, F1). From `DIRECT_SIGMA_LIMIT` up the
//! blur is Deriche's recursive Gaussian instead (R. Deriche, "Recursively implementing the Gaussian
//! and its derivatives", INRIA RR-1893, 1993): the kernel is fitted by two damped cosines, each run
//! as a causal and an anticausal second-order recursion, so every pixel costs the same whatever
//! sigma is. Repeating the edge is exact in this form: the input before the first pixel is that
//! pixel for ever, so each recursion starts in its steady state for it. Its result is the direct
//! convolution's (`blur_gray`) within 1 level (the randomized sweep in tests/selection_model.rs).
use crate::{blur_gray, GrayRaster};

/// Below this sigma the direct convolution, the formula itself, costs 13 taps a pass at most, no
/// more than the recursion; from here up the recursion is cheaper.
pub const DIRECT_SIGMA_LIMIT: f64 = 2.0;

/// Deriche's fit of exp(-x^2 / 2), for x >= 0 in units of sigma: (a cos(w x) + b sin(w x)) e^(-l x)
/// summed over the two terms (a, b, l, w).
const TERMS: [(f64, f64, f64, f64); 2] = [(1.680, 3.735, 1.783, 0.6318), (-0.6803, -0.2598, 1.723, 1.997)];

/// One term as a causal and an anticausal second-order recursion sharing a denominator:
/// y+[n] = n0 x[n] + n1 x[n-1] - d1 y+[n-1] - d2 y+[n-2] and
/// y-[n] = m1 x[n+1] + m2 x[n+2] - d1 y-[n+1] - d2 y-[n+2]. `causal` and `anticausal` are their
/// responses to a constant 1, the state each starts in at a repeated edge.
#[derive(Clone, Copy)]
struct Section { n0: f64, n1: f64, m1: f64, m2: f64, d1: f64, d2: f64, causal: f64, anticausal: f64 }

/// The two sections for `sigma`, their numerators scaled so the whole kernel sums to 1.
fn sections(sigma: f64) -> [Section; 2] {
    let mut s = TERMS.map(|(a, b, l, w)| {
        let (r, t) = ((-l / sigma).exp(), w / sigma);
        let (n0, n1) = (a, r * (b * t.sin() - a * t.cos()));
        let (m1, m2) = (r * (a * t.cos() + b * t.sin()), -r * r * a);
        let (d1, d2) = (-2.0 * r * t.cos(), r * r);
        let den = 1.0 + d1 + d2;
        Section { n0, n1, m1, m2, d1, d2, causal: (n0 + n1) / den, anticausal: (m1 + m2) / den }
    });
    let total: f64 = s.iter().map(|k| k.causal + k.anticausal).sum();
    for k in &mut s {
        k.n0 /= total; k.n1 /= total; k.m1 /= total; k.m2 /= total; k.causal /= total; k.anticausal /= total;
    }
    s
}

/// Added to every input, so a response dying away past an edge settles on this instead of sinking
/// into subnormal numbers, which the processor handles a hundred times slower. It moves no output
/// by a measurable amount.
const GUARD: f64 = 1e-20;

/// Independent lines run side by side (rows in the first pass, columns in the second), so their
/// recursions overlap instead of each waiting on its own previous output.
const LANES: usize = 64;

/// `LANES` lines of `count` samples blurred together: `load(i, into)` fills sample i of every lane,
/// `store(i, values)` takes sample i of every result. The causal pass keeps its outputs in `causal`
/// (`count` entries); the anticausal pass adds its own and stores.
fn blur_lanes(s: &[Section; 2], count: usize, causal: &mut [[f64; LANES]], load: impl Fn(usize, &mut [f64; LANES]), mut store: impl FnMut(usize, &[f64; LANES])) {
    let (mut x, mut xp) = ([0f64; LANES], [0f64; LANES]);
    let mut y = [[0f64; LANES]; 4];
    load(0, &mut xp);
    for l in 0..LANES { y[0][l] = xp[l] * s[0].causal; y[1][l] = y[0][l]; y[2][l] = xp[l] * s[1].causal; y[3][l] = y[2][l]; }
    for (i, out) in causal.iter_mut().enumerate().take(count) {
        load(i, &mut x);
        for l in 0..LANES {
            let a = s[0].n0 * x[l] + s[0].n1 * xp[l] - s[0].d1 * y[0][l] - s[0].d2 * y[1][l];
            let b = s[1].n0 * x[l] + s[1].n1 * xp[l] - s[1].d1 * y[2][l] - s[1].d2 * y[3][l];
            y[1][l] = y[0][l]; y[0][l] = a; y[3][l] = y[2][l]; y[2][l] = b;
            out[l] = a + b;
        }
        xp = x;
    }
    // Anticausal: the two inputs after each sample, and the two outputs after it of each section.
    let (mut x1, mut x2) = ([0f64; LANES], [0f64; LANES]);
    load(count - 1, &mut x1);
    x2.copy_from_slice(&x1);
    for l in 0..LANES { y[0][l] = x1[l] * s[0].anticausal; y[1][l] = y[0][l]; y[2][l] = x1[l] * s[1].anticausal; y[3][l] = y[2][l]; }
    let mut sum = [0f64; LANES];
    for i in (0..count).rev() {
        for l in 0..LANES {
            let a = s[0].m1 * x1[l] + s[0].m2 * x2[l] - s[0].d1 * y[0][l] - s[0].d2 * y[1][l];
            let b = s[1].m1 * x1[l] + s[1].m2 * x2[l] - s[1].d1 * y[2][l] - s[1].d2 * y[3][l];
            y[1][l] = y[0][l]; y[0][l] = a; y[3][l] = y[2][l]; y[2][l] = b;
            sum[l] = causal[i][l] + a + b;
        }
        store(i, &sum);
        x2 = x1;
        load(i, &mut x1);
    }
}

/// The coverage blurred by a Gaussian of `sigma` pixels, its edge pixels repeated beyond it.
pub fn feather_blur(coverage: &GrayRaster, sigma: f64) -> GrayRaster {
    if coverage.is_uniform().is_some() || !(sigma > 0.0) { return coverage.clone(); }
    if sigma < DIRECT_SIGMA_LIMIT { return blur_gray(coverage, sigma); }
    let s = sections(sigma);
    let (w, h) = (coverage.width as usize, coverage.height as usize);
    let src = coverage.bytes();
    let mut causal = vec![[0f64; LANES]; w.max(h)];
    // Rows first, `LANES` at a time (the last group repeats its last row), into 1/256-level fixed
    // point: half the memory of f32, and far finer than a level.
    let mut rows = vec![0u16; w * h];
    for y0 in (0..h).step_by(LANES) {
        let lanes: [usize; LANES] = std::array::from_fn(|l| (y0 + l).min(h - 1));
        blur_lanes(&s, w, &mut causal, |x, into| for l in 0..LANES { into[l] = src[lanes[l] * w + x] as f64 + GUARD },
            |x, values| for (l, &y) in lanes.iter().enumerate().take(h - y0) { rows[y * w + x] = (values[l] * 256.0 + 0.5).clamp(0.0, 65535.0) as u16; });
    }
    // Then columns, `LANES` side by side along each row (the last group repeats its last column).
    let mut out = vec![0u8; w * h];
    for x0 in (0..w).step_by(LANES) {
        let n = LANES.min(w - x0);
        let rows = &rows;
        blur_lanes(&s, h, &mut causal, |y, into| {
            for (v, &r) in into.iter_mut().zip(&rows[y * w + x0..y * w + x0 + n]) { *v = r as f64 / 256.0 + GUARD; }
            let last = into[n - 1];
            for v in into.iter_mut().skip(n) { *v = last; }
        }, |y, values| for (o, v) in out[y * w + x0..y * w + x0 + n].iter_mut().zip(values) { *o = (v + 0.5).clamp(0.0, 255.0) as u8; });
    }
    GrayRaster::from_bytes(coverage.width, coverage.height, out)
}
