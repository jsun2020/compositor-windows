//! The six layer effects drawn as Compositor for Mac draws them: its Metal path
//! (Rendering/MetalLayerEffects.swift:58-392), which is what renders on every Mac with Metal (R 4.2
//! step 4), in f32 as its kernels compute. Everything happens in the padded LAYER-pixel grid: the
//! layer's shown pixels sit `inset` pixels in from every side of the image, as
//! `LayerEffectsRenderer.render` places them (LayerEffects.swift:436-453).
//!
//! The planes stream top to bottom a row at a time. A pass that reads other rows (a blur, a
//! stroke's reach) keeps only the rows it can still read, in a ring, so no full-size plane is ever
//! held: a render holds its output (4 bytes a padded pixel), rings a few hundred rows deep and, for
//! each blur past `EFFECTS_REACH_LIMIT`, one plane reduced by 2^level on each side.
//! tests/effects_render.rs holds a whole-plane transcription of the Metal kernels, which this
//! must equal to the bit up to `EFFECTS_REACH_LIMIT`.
use crate::LayerEffects;

/// A stroke as the Metal path takes it: `reach = max(1, round(size))` whole pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokePass { pub reach: usize, pub inside: bool, pub color: [f32; 3], pub opacity: f32 }
/// A drop or inner shadow: the offset in layer pixels (y down) and `sigma = blur / 2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowPass { pub dx: f32, pub dy: f32, pub sigma: f32, pub color: [f32; 3], pub opacity: f32 }
/// An outer or inner glow: `sigma = size / 2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowPass { pub sigma: f32, pub color: [f32; 3], pub opacity: f32 }
/// A colour overlay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillPass { pub color: [f32; 3], pub opacity: f32 }

/// The effects one render draws.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct EffectPasses {
    pub stroke: Option<StrokePass>,
    pub shadow: Option<ShadowPass>,
    pub overlay: Option<FillPass>,
    pub inner_shadow: Option<ShadowPass>,
    pub outer_glow: Option<GlowPass>,
    pub inner_glow: Option<GlowPass>,
}

impl EffectPasses {
    /// What `MetalLayerEffects.render` draws of `effects`, in Float (MetalLayerEffects.swift:89-191):
    /// every shown effect at an opacity above 0, a stroke and the two glows only at a size above 0.
    pub fn from_effects(effects: &LayerEffects) -> EffectPasses {
        let e = effects.visible();
        let rgb = |r: f64, g: f64, b: f64| [r as f32, g as f32, b as f32];
        // `offset` (LayerEffects.swift:39-43, 79-82): away from the light, y down.
        let offset = |angle: f64, distance: f64| {
            let radians = angle * std::f64::consts::PI / 180.0;
            ((-radians.cos() * distance) as f32, (radians.sin() * distance) as f32)
        };
        EffectPasses {
            stroke: e.stroke.as_ref().filter(|s| s.size > 0.0 && s.opacity > 0.0).map(|s| StrokePass {
                reach: (s.size.round() as i64).max(1) as usize, inside: s.inside, color: rgb(s.red, s.green, s.blue), opacity: s.opacity as f32 }),
            shadow: e.shadow.as_ref().filter(|s| s.opacity > 0.0).map(|s| {
                let (dx, dy) = offset(s.angle, s.distance);
                ShadowPass { dx, dy, sigma: (s.blur / 2.0) as f32, color: rgb(s.red, s.green, s.blue), opacity: s.opacity as f32 }
            }),
            overlay: e.color_overlay.as_ref().filter(|o| o.opacity > 0.0).map(|o| FillPass { color: rgb(o.red, o.green, o.blue), opacity: o.opacity as f32 }),
            inner_shadow: e.inner_shadow.as_ref().filter(|s| s.opacity > 0.0).map(|s| {
                let (dx, dy) = offset(s.angle, s.distance);
                ShadowPass { dx, dy, sigma: (s.blur / 2.0) as f32, color: rgb(s.red, s.green, s.blue), opacity: s.opacity as f32 }
            }),
            outer_glow: e.outer_glow.as_ref().filter(|g| g.size > 0.0 && g.opacity > 0.0)
                .map(|g| GlowPass { sigma: (g.size / 2.0) as f32, color: rgb(g.red, g.green, g.blue), opacity: g.opacity as f32 }),
            inner_glow: e.inner_glow.as_ref().filter(|g| g.size > 0.0 && g.opacity > 0.0)
                .map(|g| GlowPass { sigma: (g.size / 2.0) as f32, color: rgb(g.red, g.green, g.blue), opacity: g.opacity as f32 }),
        }
    }
}

/// A Gaussian whose reach (3 sigma) passes this many layer pixels runs on a copy reduced by a power
/// of two, as 3.5b's blur layers do past SPATIAL_REACH_LIMIT: at most 97 taps a pass at any size.
/// Measured against the exact kernel (tests/effects_render.rs): within 1 level, at any inset --
/// including an inner glow's or inner shadow's, which `LayerEffects::margin` gives no room of its
/// own, so it can sit right at the padded image's edge.
pub const EFFECTS_REACH_LIMIT: f32 = 48.0;

/// How many times a Gaussian of `sigma` layer pixels is halved first: 0 is the exact kernel.
pub fn effects_level(sigma: f32) -> u32 {
    let (mut level, mut reach) = (0, sigma * 3.0);
    while reach > EFFECTS_REACH_LIMIT { reach /= 2.0; level += 1; }
    level
}

/// The layer's shown pixels (premultiplied RGBA8, `width` x `height`), `inset` pixels in from every
/// side of the padded image.
pub struct Padded<'a> { pub pixels: &'a [u8], pub width: usize, pub height: usize, pub inset: usize }

impl Padded<'_> {
    pub fn padded_width(&self) -> usize { self.width + 2 * self.inset }
    pub fn padded_height(&self) -> usize { self.height + 2 * self.inset }
    /// `effects_alpha`: coverage = alpha / 255 along padded row `y`, zero off the layer.
    fn shape_row(&self, y: usize, out: &mut [f32]) {
        out.fill(0.0);
        if y < self.inset || y >= self.inset + self.height { return; }
        let row = (y - self.inset) * self.width * 4;
        for x in 0..self.width { out[self.inset + x] = self.pixels[row + x * 4 + 3] as f32 / 255.0; }
    }
}

/// Metal's `mix(a, b, t)`.
fn mix(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// `effects_shift`: row `y` of the shape moved by (dx, dy), bilinear, zero wherever the source
/// point leaves the image.
fn shifted_row(padded: &Padded, dx: f32, dy: f32, y: usize, top: &mut [f32], bottom: &mut [f32], out: &mut [f32]) {
    // No move is the shape itself: every mix below would weigh its second sample by 0.
    if dx == 0.0 && dy == 0.0 { padded.shape_row(y, out); return; }
    let (w, h) = (padded.padded_width(), padded.padded_height());
    out.fill(0.0);
    let sy = y as f32 - dy;
    if !(sy >= 0.0 && sy <= (h - 1) as f32) { return; }
    let y0 = sy.floor() as usize;
    let y1 = (y0 + 1).min(h - 1);
    let fy = sy - y0 as f32;
    padded.shape_row(y0, top);
    padded.shape_row(y1, bottom);
    for x in 0..w {
        let sx = x as f32 - dx;
        if !(sx >= 0.0 && sx <= (w - 1) as f32) { continue; }
        let x0 = sx.floor() as usize;
        let x1 = (x0 + 1).min(w - 1);
        let fx = sx - x0 as f32;
        out[x] = mix(mix(top[x0], top[x1], fx), mix(bottom[x0], bottom[x1], fx), fy);
    }
}

/// The Gaussian of `effects_blur_rows` and `_columns`: weights exp(-o^2 / (2 sigma^2)) for
/// |o| <= radius, radius = max(1, round(3 sigma)), summed in offset order.
struct Kernel { radius: usize, weights: Vec<f32>, sum: f32 }

impl Kernel {
    fn new(sigma: f32) -> Kernel {
        let radius = ((sigma * 3.0).round() as i64).max(1) as usize;
        let r = radius as i64;
        let weights: Vec<f32> = (-r..=r).map(|o| (-((o * o) as f32) / (2.0 * sigma * sigma)).exp()).collect();
        let mut sum = 0.0f32;
        for w in &weights { sum += *w; }
        Kernel { radius, weights, sum }
    }
}

/// One line through the kernel, the edge sample repeated past each end. A window holding only 0.0
/// or only 1.0 gives that value without the sum: the sum would give exactly the same (its total
/// is then 0, or the very sum it is divided by), so no bit changes.
fn blur_line(k: &Kernel, src: &[f32], out: &mut [f32], zeros: &mut Vec<u32>, ones: &mut Vec<u32>) {
    let n = src.len();
    zeros.clear(); ones.clear(); zeros.push(0); ones.push(0);
    for v in src {
        zeros.push(zeros.last().unwrap() + (*v == 0.0) as u32);
        ones.push(ones.last().unwrap() + (*v == 1.0) as u32);
    }
    let r = k.radius as i64;
    for x in 0..n {
        let lo = (x as i64 - r).max(0) as usize;
        let hi = ((x as i64 + r) as usize).min(n - 1);
        let span = (hi - lo + 1) as u32;
        if zeros[hi + 1] - zeros[lo] == span { out[x] = 0.0; continue; }
        if ones[hi + 1] - ones[lo] == span { out[x] = 1.0; continue; }
        let mut total = 0.0f32;
        for (i, w) in k.weights.iter().enumerate() {
            let s = (x as i64 + i as i64 - r).clamp(0, n as i64 - 1) as usize;
            total += w * src[s];
        }
        out[x] = total / k.sum;
    }
}

/// A plane blurred through `Kernel` on both axes, served a row at a time from a ring holding the
/// row-blurred rows the column pass can still read.
struct ExactBlur { k: Kernel, width: usize, height: usize, ring: Vec<f32>, slots: usize, next: usize, low: usize,
    zeros: Vec<u32>, ones: Vec<u32>, line_zeros: Vec<u32>, line_ones: Vec<u32>, input: Vec<f32> }

impl ExactBlur {
    fn new(k: Kernel, width: usize, height: usize) -> ExactBlur {
        let slots = (2 * k.radius + 1).min(height);
        ExactBlur { width, height, ring: vec![0.0; slots * width], slots, next: 0, low: 0,
            zeros: vec![0; width], ones: vec![0; width], line_zeros: Vec::new(), line_ones: Vec::new(), input: vec![0.0; width], k }
    }
    /// Counts (sign +1) or uncounts (-1) ring row `row` in the per-column tallies of exact 0s and 1s.
    fn count(&mut self, row: usize, sign: i32) {
        let at = (row % self.slots) * self.width;
        for x in 0..self.width {
            let v = self.ring[at + x];
            if v == 0.0 { self.zeros[x] = (self.zeros[x] as i32 + sign) as u32; }
            if v == 1.0 { self.ones[x] = (self.ones[x] as i32 + sign) as u32; }
        }
    }
    /// Row `y` of the blurred plane; `y` counts up from 0. `source(row, out)` fills a source row.
    fn row(&mut self, y: usize, source: &mut dyn FnMut(usize, &mut [f32]), out: &mut [f32]) {
        let r = self.k.radius;
        let lo = y.saturating_sub(r);
        let hi = (y + r).min(self.height - 1);
        // Rows above the window leave first: a new row takes the ring slot of the one leaving.
        while self.low < lo { self.count(self.low, -1); self.low += 1; }
        while self.next <= hi {
            source(self.next, &mut self.input);
            let at = (self.next % self.slots) * self.width;
            blur_line(&self.k, &self.input, &mut self.ring[at..at + self.width], &mut self.line_zeros, &mut self.line_ones);
            self.count(self.next, 1);
            self.next += 1;
        }
        let span = (hi - lo + 1) as u32;
        let rr = r as i64;
        for x in 0..self.width {
            if self.zeros[x] == span { out[x] = 0.0; continue; }
            if self.ones[x] == span { out[x] = 1.0; continue; }
            let mut total = 0.0f32;
            for (i, w) in self.k.weights.iter().enumerate() {
                let s = (y as i64 + i as i64 - rr).clamp(0, self.height as i64 - 1) as usize;
                total += w * self.ring[(s % self.slots) * self.width + x];
            }
            out[x] = total / self.k.sum;
        }
    }
}

/// A Gaussian past EFFECTS_REACH_LIMIT: the plane reduced by 2^level (each cell the mean of the
/// pixels it covers), extended by a halo of the reduced kernel's own radius on every side so its
/// blur clamps to the padded image's edge PIXEL, not to the mean of its edge CELL (`effects_blur_rows`
/// and `_columns` clamp to the padded image itself, whatever grid computes the sum), blurred with
/// sigma / 2^level through the exact kernel, and enlarged bilinearly at each pixel's centre, clamped
/// to the edge, as 3.5b's `blur_for_layer` enlarges. It keeps its blurred reduced plane (4 bytes a
/// cell, a cell being 2^level x 2^level padded pixels) for the whole render; the reduced plane and
/// its halo, and the plane it blurred from, are freed when `new` returns.
struct HalvedBlur { level: u32, width: usize, cells: Vec<f32>, cw: usize, ch: usize }

impl HalvedBlur {
    fn new(sigma: f32, level: u32, width: usize, height: usize, source: &mut dyn FnMut(usize, &mut [f32])) -> HalvedBlur {
        let f = 1usize << level;
        let (cw, ch) = ((width + f - 1) / f, (height + f - 1) / f);
        let mut sums = vec![0.0f32; cw * ch];
        let mut row = vec![0.0f32; width];
        // The padded image's own edge row and column: a halo cell past the image repeats one of
        // these (clamp-to-edge saturates immediately, so every halo cell at a given distance from
        // the interior shares the one value the true edge pixel gives), and a corner halo cell
        // repeats the one corner pixel, clamped on both axes at once.
        let (mut top_row, mut bottom_row) = (vec![0.0f32; width], vec![0.0f32; width]);
        let (mut left_col, mut right_col) = (vec![0.0f32; height], vec![0.0f32; height]);
        for y in 0..height {
            source(y, &mut row);
            let at = (y / f) * cw;
            for x in 0..width { sums[at + x / f] += row[x]; }
            left_col[y] = row[0];
            right_col[y] = row[width - 1];
            if y == 0 { top_row.copy_from_slice(&row); }
            if y == height - 1 { bottom_row.copy_from_slice(&row); }
        }
        for cy in 0..ch { for cx in 0..cw {
            let covered = ((width - cx * f).min(f) * (height - cy * f).min(f)) as f32;
            sums[cy * cw + cx] /= covered;
        }}
        // The edge row or column, reduced along its own length the same way `sums` reduces both
        // axes, gives what a halo cell just off that edge would average to.
        let reduce_1d = |src: &[f32], n: usize, cn: usize| -> Vec<f32> {
            (0..cn).map(|c| {
                let (lo, hi) = (c * f, (c * f + f).min(n));
                src[lo..hi].iter().sum::<f32>() / (hi - lo) as f32
            }).collect()
        };
        let top_reduced = reduce_1d(&top_row, width, cw);
        let bottom_reduced = reduce_1d(&bottom_row, width, cw);
        let left_reduced = reduce_1d(&left_col, height, ch);
        let right_reduced = reduce_1d(&right_col, height, ch);
        let (tl, tr, bl, br) = (top_row[0], top_row[width - 1], bottom_row[0], bottom_row[width - 1]);

        let k = Kernel::new(sigma / f as f32);
        let r = k.radius;
        let (ecw, ech) = (cw + 2 * r, ch + 2 * r);
        let mut extended = vec![0.0f32; ecw * ech];
        for cy in 0..ch { extended[(cy + r) * ecw + r..(cy + r) * ecw + r + cw].copy_from_slice(&sums[cy * cw..(cy + 1) * cw]); }
        for cy in 0..ch { for d in 1..=r {
            extended[(cy + r) * ecw + r - d] = left_reduced[cy];
            extended[(cy + r) * ecw + r + cw - 1 + d] = right_reduced[cy];
        }}
        for cx in 0..cw { for d in 1..=r {
            extended[(r - d) * ecw + cx + r] = top_reduced[cx];
            extended[(r + ch - 1 + d) * ecw + cx + r] = bottom_reduced[cx];
        }}
        for dy in 1..=r { for dx in 1..=r {
            extended[(r - dy) * ecw + r - dx] = tl;
            extended[(r - dy) * ecw + r + cw - 1 + dx] = tr;
            extended[(r + ch - 1 + dy) * ecw + r - dx] = bl;
            extended[(r + ch - 1 + dy) * ecw + r + cw - 1 + dx] = br;
        }}
        // The extended grid's own edge is `r` cells past every real or haloed cell the blur reads,
        // so the clamp-to-edge inside `ExactBlur` never actually fires: it would repeat a value this
        // halo already made exact.
        let mut blur = ExactBlur::new(k, ecw, ech);
        let mut ext_cells = vec![0.0f32; ecw * ech];
        let mut from_ext = |y: usize, out: &mut [f32]| out.copy_from_slice(&extended[y * ecw..(y + 1) * ecw]);
        for y in 0..ech { blur.row(y, &mut from_ext, &mut ext_cells[y * ecw..(y + 1) * ecw]); }
        let mut cells = vec![0.0f32; cw * ch];
        for cy in 0..ch { cells[cy * cw..(cy + 1) * cw].copy_from_slice(&ext_cells[(cy + r) * ecw + r..(cy + r) * ecw + r + cw]); }
        HalvedBlur { level, width, cells, cw, ch }
    }
    fn row(&self, y: usize, out: &mut [f32]) {
        let f = (1u32 << self.level) as f32;
        let fy = (y as f32 + 0.5) / f - 0.5;
        let ty = fy - fy.floor();
        let clamp_y = |v: i64| v.clamp(0, self.ch as i64 - 1) as usize;
        let (ya, yb) = (clamp_y(fy.floor() as i64), clamp_y(fy.floor() as i64 + 1));
        for x in 0..self.width {
            let fx = (x as f32 + 0.5) / f - 0.5;
            let tx = fx - fx.floor();
            let clamp_x = |v: i64| v.clamp(0, self.cw as i64 - 1) as usize;
            let (xa, xb) = (clamp_x(fx.floor() as i64), clamp_x(fx.floor() as i64 + 1));
            let top = mix(self.cells[ya * self.cw + xa], self.cells[ya * self.cw + xb], tx);
            let bottom = mix(self.cells[yb * self.cw + xa], self.cells[yb * self.cw + xb], tx);
            out[x] = mix(top, bottom, ty);
        }
    }
}

enum Soft { Sharp, Exact(ExactBlur), Halved(HalvedBlur) }

/// The shape moved by (dx, dy) and softened by sigma, as the drop shadow, inner shadow and both
/// glows make their planes (MetalLayerEffects.swift:98-174); no blur at sigma <= 0.01.
struct Softened { dx: f32, dy: f32, soft: Soft, top: Vec<f32>, bottom: Vec<f32> }

impl Softened {
    fn new(padded: &Padded, dx: f32, dy: f32, sigma: f32) -> Softened {
        let (w, h) = (padded.padded_width(), padded.padded_height());
        let (mut top, mut bottom) = (vec![0.0; w], vec![0.0; w]);
        let soft = if !(sigma > 0.01) { Soft::Sharp } else {
            match effects_level(sigma) {
                0 => Soft::Exact(ExactBlur::new(Kernel::new(sigma), w, h)),
                level => Soft::Halved(HalvedBlur::new(sigma, level, w, h, &mut |y, out| shifted_row(padded, dx, dy, y, &mut top, &mut bottom, out))),
            }
        };
        Softened { dx, dy, soft, top, bottom }
    }
    fn row(&mut self, padded: &Padded, y: usize, out: &mut [f32]) {
        let (dx, dy) = (self.dx, self.dy);
        let (top, bottom) = (&mut self.top, &mut self.bottom);
        match &mut self.soft {
            Soft::Sharp => shifted_row(padded, dx, dy, y, top, bottom, out),
            Soft::Exact(blur) => blur.row(y, &mut |row, o| shifted_row(padded, dx, dy, row, &mut top[..], &mut bottom[..], o), out),
            Soft::Halved(blur) => blur.row(y, out),
        }
    }
}

/// The stroke's reach (`effects_spread_rows` and `_columns`): the largest (outside) or smallest
/// (inside) coverage within `reach` whole pixels on each axis, a square; past the image there is
/// nothing (0), which an inside stroke's minimum sees. Rows arrive in a ring; each column keeps a
/// monotonic queue of the rows that can still win, so the cost does not grow with the reach.
struct Spread { reach: usize, smallest: bool, width: usize, height: usize, ring: Vec<f32>, slots: usize, next: usize,
    queue: Vec<u32>, head: Vec<usize>, len: Vec<usize>, cap: usize, input: Vec<f32>, line: Vec<f32>, order: Vec<usize> }

impl Spread {
    fn new(reach: usize, smallest: bool, width: usize, height: usize) -> Spread {
        let slots = (2 * reach + 1).min(height);
        let cap = slots + 1;
        Spread { reach, smallest, width, height, ring: vec![0.0; slots * width], slots, next: 0,
            queue: vec![0; cap * width], head: vec![0; width], len: vec![0; width], cap,
            input: vec![0.0; width], line: vec![0.0; width], order: Vec::with_capacity(width) }
    }
    fn beats(smallest: bool, a: f32, b: f32) -> bool { if smallest { a <= b } else { a >= b } }
    /// The row pass over `self.input` into `self.line`.
    fn spread_line(&mut self) {
        let (n, r, smallest) = (self.input.len(), self.reach, self.smallest);
        self.order.clear();
        let (mut head, mut next) = (0, 0);
        for x in 0..n {
            while next <= (x + r).min(n - 1) {
                while self.order.len() > head && Self::beats(smallest, self.input[next], self.input[*self.order.last().unwrap()]) { self.order.pop(); }
                self.order.push(next);
                next += 1;
            }
            while self.order[head] + r < x { head += 1; }
            let outside = x < r || x + r >= n;
            self.line[x] = if smallest && outside { 0.0 } else { self.input[self.order[head]] };
        }
    }
    fn row(&mut self, padded: &Padded, y: usize, out: &mut [f32]) {
        let r = self.reach;
        let hi = (y + r).min(self.height - 1);
        // Rows above the window leave first: a new row takes the ring slot of the one leaving.
        for x in 0..self.width {
            let base = x * self.cap;
            while self.len[x] > 0 && (self.queue[base + self.head[x]] as usize) + r < y {
                self.head[x] = (self.head[x] + 1) % self.cap;
                self.len[x] -= 1;
            }
        }
        while self.next <= hi {
            padded.shape_row(self.next, &mut self.input);
            self.spread_line();
            let at = (self.next % self.slots) * self.width;
            self.ring[at..at + self.width].copy_from_slice(&self.line);
            for x in 0..self.width {
                let v = self.ring[at + x];
                let base = x * self.cap;
                while self.len[x] > 0 {
                    let back = self.queue[base + (self.head[x] + self.len[x] - 1) % self.cap] as usize;
                    if !Self::beats(self.smallest, v, self.ring[(back % self.slots) * self.width + x]) { break; }
                    self.len[x] -= 1;
                }
                self.queue[base + (self.head[x] + self.len[x]) % self.cap] = self.next as u32;
                self.len[x] += 1;
            }
            self.next += 1;
        }
        let outside = y < r || y + r >= self.height;
        for x in 0..self.width {
            let best = self.queue[x * self.cap + self.head[x]] as usize;
            out[x] = if self.smallest && outside { 0.0 } else { self.ring[(best % self.slots) * self.width + x] };
        }
    }
}

/// Premultiplied source-over of a flat colour at `coverage(x) * opacity` along a run of one row (a
/// step of `effects_compose`). Where the coverage is 0 the pixel stays exactly as it was.
fn over(color: &mut [[f32; 4]], rgb: [f32; 3], opacity: f32, coverage: impl Fn(usize) -> f32) {
    for (x, px) in color.iter_mut().enumerate() {
        let c = (coverage(x) * opacity).clamp(0.0, 1.0);
        for k in 0..3 { px[k] = rgb[k] * c + px[k] * (1.0 - c); }
        px[3] = c + px[3] * (1.0 - c);
    }
}

/// The padded image, premultiplied RGBA8 (`effects_compose`, MetalLayerEffects.swift:335-392):
/// drop shadow behind, outer glow over it, an outside stroke over that, the pixels, then colour
/// overlay, inner glow, inner shadow and an inside stroke. Every step above the pixels covers
/// nothing off the layer (its coverage is the shape times something), so those run over the
/// layer's own columns and rows only, which leaves every other byte exactly as the whole-row
/// kernel would. 8-bit output is `clamp(v) * 255 + 0.5`, truncated.
pub fn render_passes(padded: &Padded, passes: &EffectPasses) -> Vec<u8> {
    let (w, h) = (padded.padded_width(), padded.padded_height());
    let (x0, x1) = (padded.inset, padded.inset + padded.width);
    // The halved Gaussians build their reduced planes one after another, before the output
    // exists, so the peak is the output beside those planes, never a plane being built beside it.
    let mut shadow_plane = passes.shadow.map(|s| Softened::new(padded, s.dx, s.dy, s.sigma));
    let mut inner_plane = passes.inner_shadow.map(|s| Softened::new(padded, s.dx, s.dy, s.sigma));
    let mut glow_plane = passes.outer_glow.map(|g| Softened::new(padded, 0.0, 0.0, g.sigma));
    let mut inner_glow_plane = passes.inner_glow.map(|g| Softened::new(padded, 0.0, 0.0, g.sigma));
    let mut spread = passes.stroke.map(|s| Spread::new(s.reach, s.inside, w, h));
    let mut out = vec![0u8; w * h * 4];
    let mut shape = vec![0.0f32; w];
    let (mut ring, mut shadow, mut inner, mut glow, mut inner_glow) = (vec![0.0f32; w], vec![0.0f32; w], vec![0.0f32; w], vec![0.0f32; w], vec![0.0f32; w]);
    let mut color = vec![[0.0f32; 4]; w];
    // `float(byte) / 255.0` for every byte, looked up rather than divided per pixel: the same values.
    let unit: [f32; 256] = std::array::from_fn(|b| b as f32 / 255.0);
    for y in 0..h {
        padded.shape_row(y, &mut shape);
        color.fill([0.0; 4]);
        if let (Some(s), Some(plane)) = (passes.shadow, &mut shadow_plane) {
            plane.row(padded, y, &mut shadow);
            for x in 0..w {
                let c = (shadow[x] * s.opacity).clamp(0.0, 1.0);
                color[x] = [s.color[0] * c, s.color[1] * c, s.color[2] * c, c];
            }
        }
        if let (Some(g), Some(plane)) = (passes.outer_glow, &mut glow_plane) {
            plane.row(padded, y, &mut glow);
            over(&mut color, g.color, g.opacity, |x| glow[x] * (1.0 - shape[x]));
        }
        if let (Some(s), Some(plane)) = (passes.stroke, &mut spread) {
            plane.row(padded, y, &mut ring);
            // `effects_ring`: the band between the shape and its reach.
            for x in 0..w { ring[x] = if s.inside { shape[x] - ring[x] } else { ring[x] - shape[x] }.clamp(0.0, 1.0); }
            if !s.inside { over(&mut color, s.color, s.opacity, |x| ring[x]); }
        }
        if let (Some(_), Some(plane)) = (passes.inner_shadow, &mut inner_plane) {
            plane.row(padded, y, &mut inner);
            // `effects_inside`: what lies outside, moved and softened, kept to the shape.
            for x in 0..w { inner[x] = (shape[x] * (1.0 - inner[x])).clamp(0.0, 1.0); }
        }
        if let (Some(_), Some(plane)) = (passes.inner_glow, &mut inner_glow_plane) {
            plane.row(padded, y, &mut inner_glow);
            for x in 0..w { inner_glow[x] = (shape[x] * (1.0 - inner_glow[x])).clamp(0.0, 1.0); }
        }
        if y >= padded.inset && y < padded.inset + padded.height {
            let from = (y - padded.inset) * padded.width * 4;
            let row = &padded.pixels[from..from + padded.width * 4];
            let layer = &mut color[x0..x1];
            for (x, px) in layer.iter_mut().enumerate() {
                let s = [0, 1, 2, 3].map(|c| unit[row[x * 4 + c] as usize]);
                for k in 0..3 { px[k] = s[k] + px[k] * (1.0 - s[3]); }
                px[3] = s[3] + px[3] * (1.0 - s[3]);
            }
            if let Some(o) = passes.overlay { over(layer, o.color, o.opacity, |x| shape[x0 + x]); }
            if let Some(g) = passes.inner_glow { over(layer, g.color, g.opacity, |x| inner_glow[x0 + x]); }
            if let Some(s) = passes.inner_shadow { over(layer, s.color, s.opacity, |x| inner[x0 + x]); }
            if let Some(s) = passes.stroke { if s.inside { over(layer, s.color, s.opacity, |x| ring[x0 + x]); } }
        }
        let bytes = &mut out[y * w * 4..(y + 1) * w * 4];
        for (x, px) in color.iter().enumerate() {
            for k in 0..4 { bytes[x * 4 + k] = (px[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8; }
        }
    }
    out
}
