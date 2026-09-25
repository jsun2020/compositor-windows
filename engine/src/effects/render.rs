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
    let mut out = vec![0u8; w * h * 4];
    let mut shape = vec![0.0f32; w];
    let mut ring = vec![0.0f32; w];
    let mut color = vec![[0.0f32; 4]; w];
    // `float(byte) / 255.0` for every byte, looked up rather than divided per pixel: the same values.
    let unit: [f32; 256] = std::array::from_fn(|b| b as f32 / 255.0);
    let mut spread = passes.stroke.map(|s| Spread::new(s.reach, s.inside, w, h));
    for y in 0..h {
        padded.shape_row(y, &mut shape);
        color.fill([0.0; 4]);
        if let (Some(s), Some(plane)) = (passes.stroke, &mut spread) {
            plane.row(padded, y, &mut ring);
            // `effects_ring`: the band between the shape and its reach.
            for x in 0..w { ring[x] = if s.inside { shape[x] - ring[x] } else { ring[x] - shape[x] }.clamp(0.0, 1.0); }
            if !s.inside { over(&mut color, s.color, s.opacity, |x| ring[x]); }
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
            if let Some(s) = passes.stroke { if s.inside { over(layer, s.color, s.opacity, |x| ring[x0 + x]); } }
        }
        let bytes = &mut out[y * w * 4..(y + 1) * w * 4];
        for (x, px) in color.iter().enumerate() {
            for k in 0..4 { bytes[x * 4 + k] = (px[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8; }
        }
    }
    out
}
