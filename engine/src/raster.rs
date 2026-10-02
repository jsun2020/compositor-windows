use crate::compositor::MAX_PREFILTER_LEVEL;
use std::sync::{Arc, Mutex};

pub const TILE: u32 = 256;

/// A rectangle of a pixel grid (a layer's pixels, or its mask's), in whole pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PixelRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }

impl PixelRect {
    pub fn is_empty(&self) -> bool { self.width == 0 || self.height == 0 }
    /// The smallest rectangle holding both; an empty one adds nothing.
    pub fn union(&self, other: &PixelRect) -> PixelRect {
        if self.is_empty() { return *other; }
        if other.is_empty() { return *self; }
        let (x0, y0) = (self.x.min(other.x), self.y.min(other.y));
        let (x1, y1) = ((self.x + self.width).max(other.x + other.width), (self.y + self.height).max(other.y + other.height));
        PixelRect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
    }
    /// The pixels of a grid halved to `width` x `height` whose 2 x 2 block meets this rectangle.
    pub fn halved(&self, width: u32, height: u32) -> PixelRect {
        if self.is_empty() { return PixelRect::default(); }
        let (x0, y0) = ((self.x / 2).min(width), (self.y / 2).min(height));
        let (x1, y1) = ((self.x + self.width).div_ceil(2).min(width), (self.y + self.height).div_ceil(2).min(height));
        PixelRect { x: x0, y: y0, width: x1.saturating_sub(x0), height: y1.saturating_sub(y0) }
    }
}

/// Writes the 2 x 2 box average of `src` (`sw` x `sh`, premultiplied RGBA) into the pixels of `out`
/// (the halved grid, `sw / 2` x `sh / 2`, at least 1) that `region` covers. A block past an odd edge
/// averages the pixels that exist (only a side of 1 has one), rounding half up.
fn halve_into(src: &[u8], sw: u32, sh: u32, out: &mut [u8], region: PixelRect) {
    let w = (sw / 2).max(1) as usize;
    let (sw, sh) = (sw as usize, sh as usize);
    let (x0, x1) = (region.x as usize, (region.x + region.width) as usize);
    if sw >= 2 && sh >= 2 {
        // Every block is whole (an odd last row or column is left out): four pixels, (sum + 2) / 4.
        for y in region.y as usize..(region.y + region.height) as usize {
            let top = &src[2 * y * sw * 4 + 8 * x0..2 * y * sw * 4 + 8 * x1];
            let bottom = &src[(2 * y + 1) * sw * 4 + 8 * x0..(2 * y + 1) * sw * 4 + 8 * x1];
            let line = &mut out[(y * w + x0) * 4..(y * w + x1) * 4];
            for ((a, b), o) in top.chunks_exact(8).zip(bottom.chunks_exact(8)).zip(line.chunks_exact_mut(4)) {
                for c in 0..4 { o[c] = ((a[c] as u16 + a[4 + c] as u16 + b[c] as u16 + b[4 + c] as u16 + 2) >> 2) as u8; }
            }
        }
        return;
    }
    for y in region.y as usize..(region.y + region.height) as usize {
        let (r0, r1) = (2 * y, (2 * y + 1).min(sh - 1));
        let rows = if 2 * y + 1 < sh { 2 } else { 1 };
        let top = &src[r0 * sw * 4..(r0 + 1) * sw * 4];
        let bottom = &src[r1 * sw * 4..(r1 + 1) * sw * 4];
        let line = &mut out[y * w * 4..(y + 1) * w * 4];
        for x in region.x as usize..(region.x + region.width) as usize {
            let (c0, c1) = (2 * x * 4, (2 * x + 1).min(sw - 1) * 4);
            let cols = if 2 * x + 1 < sw { 2 } else { 1 };
            let n = (rows * cols) as u32;
            for c in 0..4 {
                let mut sum = top[c0 + c] as u32;
                if cols == 2 { sum += top[c1 + c] as u32; }
                if rows == 2 { sum += bottom[c0 + c] as u32; if cols == 2 { sum += bottom[c1 + c] as u32; } }
                line[x * 4 + c] = ((sum + n / 2) / n) as u8;
            }
        }
    }
}

/// Backing storage for a `Raster`: the pixels, plus a memoized single-step box-reduction so
/// `halved()` computed once for a given pixel buffer is shared by every clone of it (and by
/// every clone of the halved result in turn), instead of being recomputed on every frame.
/// `adopted` is one deeper halving made elsewhere (the job worker, Phase 4.5 F1): the raster after
/// that many halvings, handed over with the pixels so the UI thread never halves them itself.
/// `std::sync::Mutex` is fine here: the engine runs single-threaded on wasm32.
struct RasterInner { data: Vec<u8>, half: Mutex<Option<Raster>>, adopted: Mutex<Option<(u32, Raster)>> }

impl std::fmt::Debug for RasterInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RasterInner").field("data_len", &self.data.len()).finish()
    }
}

/// Premultiplied RGBA8, row-major, top-left origin. Cloning shares the pixels and the memoized
/// halving cache.
#[derive(Clone, Debug)]
pub struct Raster { pub width: u32, pub height: u32, inner: Arc<RasterInner> }

impl PartialEq for Raster {
    /// The same buffer is equal without reading it: `Document::same_content` compares every layer on
    /// every edit, and a 100 MP layer's bytes took a full scan to compare with themselves.
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height && (Arc::ptr_eq(&self.inner, &other.inner) || self.inner.data == other.inner.data)
    }
}

impl Raster {
    fn wrap(width: u32, height: u32, data: Vec<u8>) -> Raster {
        Raster { width, height, inner: Arc::new(RasterInner { data, half: Mutex::new(None), adopted: Mutex::new(None) }) }
    }
    pub fn new_transparent(width: u32, height: u32) -> Raster {
        Raster::wrap(width, height, vec![0; (width as usize) * (height as usize) * 4])
    }
    pub fn from_premultiplied(width: u32, height: u32, data: Vec<u8>) -> Raster {
        assert_eq!(data.len(), (width as usize) * (height as usize) * 4);
        Raster::wrap(width, height, data)
    }
    pub fn from_straight(width: u32, height: u32, straight: &[u8]) -> Raster {
        let mut data = straight.to_vec();
        for px in data.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a < 255 {
                px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
                px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
                px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
            }
        }
        Raster::from_premultiplied(width, height, data)
    }
    pub fn bytes(&self) -> &[u8] { &self.inner.data }
    /// The pixels themselves when nothing else shares them (the halving cache goes with the
    /// raster), else a copy.
    pub fn into_bytes(self) -> Vec<u8> {
        match Arc::try_unwrap(self.inner) { Ok(inner) => inner.data, Err(shared) => shared.data.clone() }
    }
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        let d = &self.inner.data;
        [d[i], d[i + 1], d[i + 2], d[i + 3]]
    }
    pub fn to_straight(&self) -> Vec<u8> {
        let mut out = self.inner.data.clone();
        for px in out.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                px[0] = ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8;
                px[1] = ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8;
                px[2] = ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
        out
    }
    pub fn same_pixels(&self, other: &Raster) -> bool { Arc::ptr_eq(&self.inner, &other.inner) }
    /// The pixel buffer's identity: equal for every clone sharing it (history counts buffers by it).
    pub fn buffer_id(&self) -> usize { Arc::as_ptr(&self.inner) as *const u8 as usize }
    /// Drops the memoized halving of this buffer, and a halving it adopted, for every clone that shares
    /// it (history lets go of the halvings of buffers only it holds: they are not counted against its
    /// limit).
    pub fn forget_halvings(&self) {
        *self.inner.half.lock().unwrap() = None;
        *self.inner.adopted.lock().unwrap() = None;
    }
    /// Whether another clone of this raster holds the same pixel buffer.
    pub fn shared(&self) -> bool { Arc::strong_count(&self.inner) > 1 }
    /// How many handles hold these pixels, this one included.
    pub fn holders(&self) -> usize { Arc::strong_count(&self.inner) }
    pub fn tiles_across(&self) -> u32 { (self.width + TILE - 1) / TILE }
    pub fn tiles_down(&self) -> u32 { (self.height + TILE - 1) / TILE }
    /// Copies tile (tx, ty) into `out` (TILE*TILE*4 bytes), zero beyond the raster.
    pub fn tile_rgba(&self, tx: u32, ty: u32, out: &mut [u8]) {
        assert_eq!(out.len(), (TILE * TILE * 4) as usize);
        out.fill(0);
        let x0 = tx * TILE; let y0 = ty * TILE;
        if x0 >= self.width || y0 >= self.height { return; }
        let w = (self.width - x0).min(TILE) as usize;
        for row in 0..(self.height - y0).min(TILE) as usize {
            let src = (((y0 as usize + row) * self.width as usize) + x0 as usize) * 4;
            let dst = row * TILE as usize * 4;
            out[dst..dst + w * 4].copy_from_slice(&self.inner.data[src..src + w * 4]);
        }
    }
    /// Box-filtered 2x reduction (odd edges average the pixels that exist). Memoized per pixel
    /// buffer: every clone of `self` and every render frame that reaches the same underlying
    /// buffer at the same halving step reuses the same result instead of recomputing it.
    pub fn halved(&self) -> Raster {
        if self.width == 0 || self.height == 0 { return self.clone(); }
        let mut cache = self.inner.half.lock().unwrap();
        if let Some(r) = cache.as_ref() { return r.clone(); }
        let (w, h) = self.half_size();
        let mut out = vec![0u8; (w * h * 4) as usize];
        halve_into(&self.inner.data, self.width, self.height, &mut out, PixelRect { x: 0, y: 0, width: w, height: h });
        let result = Raster::from_premultiplied(w, h, out);
        *cache = Some(result.clone());
        result
    }
    /// The size `halved` makes: half of each side, at least 1.
    pub fn half_size(&self) -> (u32, u32) { ((self.width / 2).max(1), (self.height / 2).max(1)) }
    /// The memoized halving, if it has been made (and not let go).
    pub fn memoized_half(&self) -> Option<Raster> { self.inner.half.lock().unwrap().clone() }
    /// The size after `level` halvings: half of each side each time, at least 1, stopping once a side
    /// is 1 (the rule `reduced` and both renderers' prefilter follow).
    pub fn size_at_level(&self, level: u32) -> (u32, u32) {
        let (mut w, mut h) = (self.width, self.height);
        for _ in 0..level.min(MAX_PREFILTER_LEVEL) {
            if w <= 1 || h <= 1 { break; }
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        (w, h)
    }
    /// The raster after `level` halvings, stopping once a side is 1: from the halving this buffer
    /// adopted when that is `level` or fewer halvings, else from the pixels; each step is memoized.
    pub fn reduced(&self, level: u32) -> Raster {
        let level = level.min(MAX_PREFILTER_LEVEL);
        let (mut current, mut at) = match self.adopted() { Some((l, r)) if l <= level => (r, l), _ => (self.clone(), 0) };
        while at < level && current.width > 1 && current.height > 1 {
            current = current.halved();
            at += 1;
        }
        current
    }
    /// Keeps `reduced`, this raster after `level` halvings made elsewhere (the job worker), for every
    /// clone of it: `reduced(level)` then returns it without halving anything. Refused (false) at level
    /// 0, when it is not the size `level` halvings make, or when `level` is past the point where a side
    /// already reached 1 (fix round 1, minor b): `size_at_level` freezes there, so a later level would
    /// report the very same size as an earlier, genuine one, and `seed_adopted`'s own arithmetic (which
    /// does not freeze the same way) would divide a frozen side to 0 and panic on the length mismatch.
    pub fn adopt(&self, level: u32, reduced: Raster) -> bool {
        if level == 0 || level > MAX_PREFILTER_LEVEL || (reduced.width, reduced.height) != self.size_at_level(level) { return false; }
        if (self.width >> (level - 1)) < 2 || (self.height >> (level - 1)) < 2 { return false; }
        *self.inner.adopted.lock().unwrap() = Some((level, reduced));
        true
    }
    /// The halving this buffer adopted, and after how many halvings it is.
    pub fn adopted(&self) -> Option<(u32, Raster)> { self.inner.adopted.lock().unwrap().clone() }
    /// Gives this raster the halvings `parent` has already made, for a raster that equals `parent`
    /// outside `rect` (a changed rectangle: an edit that kept the grid). Each kept level is copied and
    /// only the part `rect` reaches is halved again, so the result is bit-identical to halving from
    /// scratch at a fraction of the cost. Levels the parent never made are left to be made on demand.
    ///
    /// This only checks that the two buffers are the same WIDTH and HEIGHT: a `Raster` does not know
    /// the document transform that places its pixels, so it cannot tell a buffer that merely kept its
    /// size from one that was also cropped back onto a SHIFTED grid (a spreading filter's
    /// grow-then-trim can do exactly that, same size, moved origin). The caller is responsible for
    /// only calling this when the grid itself -- not just its size -- is unchanged (`engine.rs`'s
    /// `seed_halvings`, Task 3 fix round 1, bug 1).
    pub fn seed_halvings(&self, parent: &Raster, rect: PixelRect) {
        // A worker already supplied the displayed level. Rebuilding the parent's
        // entire cached ladder would copy up to a third of the full raster here.
        if self.adopted().is_some(){return;}
        if self.width != parent.width || self.height != parent.height || self.buffer_id()==parent.buffer_id() { return; }
        self.seed_adopted(parent, rect);
        let Some(parent_half) = parent.memoized_half() else { return };
        let (w, h) = self.half_size();
        // Every output pixel whose 2 x 2 block meets `rect`.
        let reach = rect.halved(w, h);
        let mut data = parent_half.bytes().to_vec();
        if !reach.is_empty() { halve_into(&self.inner.data, self.width, self.height, &mut data, reach); }
        let half = Raster::from_premultiplied(w, h, data);
        half.seed_halvings(&parent_half, reach);
        *self.inner.half.lock().unwrap() = Some(half);
    }
    /// `seed_halvings` for a halving `parent` adopted (F1): the same halving for this raster, with the
    /// part `rect` reaches made again from this raster's pixels, so an edit inside a selection after a
    /// job's result still halves only its rectangle. Only while every step halves whole 2 x 2 blocks
    /// (both sides at least 2 before the last step), where a block-aligned part halves exactly as the
    /// whole raster does.
    fn seed_adopted(&self, parent: &Raster, rect: PixelRect) {
        let Some((level, parent_reduced)) = parent.adopted() else { return };
        if (self.width >> (level - 1)) < 2 || (self.height >> (level - 1)) < 2 { return; }
        let (mut reach, mut w, mut h) = (rect, self.width, self.height);
        for _ in 0..level {
            w /= 2;
            h /= 2;
            reach = reach.halved(w, h);
        }
        let mut data = parent_reduced.bytes().to_vec();
        if !reach.is_empty() {
            let f = 1u32 << level;
            let mut part = self.cropped(reach.x * f, reach.y * f, reach.width * f, reach.height * f);
            for _ in 0..level { part = part.halved(); }
            let row = reach.width as usize * 4;
            for y in 0..reach.height as usize {
                let at = ((reach.y as usize + y) * w as usize + reach.x as usize) * 4;
                data[at..at + row].copy_from_slice(&part.bytes()[y * row..(y + 1) * row]);
            }
        }
        *self.inner.adopted.lock().unwrap() = Some((level, Raster::from_premultiplied(w, h, data)));
    }
    /// A copy of the `w` x `h` rectangle at (x, y); clamped to the raster.
    pub fn cropped(&self, x: u32, y: u32, w: u32, h: u32) -> Raster {
        let x1 = (x + w).min(self.width); let y1 = (y + h).min(self.height);
        let (w, h) = (x1.saturating_sub(x).max(1), y1.saturating_sub(y).max(1));
        let mut out = vec![0u8; (w * h * 4) as usize];
        for row in 0..h {
            let sy = y + row; if sy >= self.height { break; }
            let src = ((sy * self.width + x) * 4) as usize;
            let n = (w.min(self.width - x) * 4) as usize;
            out[(row * w * 4) as usize..(row * w * 4) as usize + n].copy_from_slice(&self.inner.data[src..src + n]);
        }
        Raster::from_premultiplied(w, h, out)
    }
}

/// 8-bit coverage, no alpha (white reveals, black hides).
#[derive(Clone, Debug, PartialEq)]
pub struct GrayRaster { pub width: u32, pub height: u32, data: Arc<Vec<u8>> }

impl GrayRaster {
    pub fn from_bytes(width: u32, height: u32, data: Vec<u8>) -> GrayRaster {
        assert_eq!(data.len(), (width as usize) * (height as usize));
        GrayRaster { width, height, data: Arc::new(data) }
    }
    pub fn bytes(&self) -> &[u8] { &self.data }
    /// The very same pixel buffer (a clone of this raster), not merely equal pixels.
    pub fn same_pixels(&self, other: &GrayRaster) -> bool { Arc::ptr_eq(&self.data, &other.data) }
    /// The pixel buffer's identity: equal for every clone sharing it.
    pub fn buffer_id(&self) -> usize { Arc::as_ptr(&self.data) as *const u8 as usize }
    /// Whether another clone of this raster holds the same pixel buffer.
    pub fn shared(&self) -> bool { Arc::strong_count(&self.data) > 1 }
    /// How many handles hold these pixels, this one included.
    pub fn holders(&self) -> usize { Arc::strong_count(&self.data) }
    pub fn is_uniform(&self) -> Option<u8> {
        let first = *self.data.first()?;
        self.data.iter().all(|&v| v == first).then_some(first)
    }
}
