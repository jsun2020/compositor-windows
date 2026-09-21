use std::sync::{Arc, Mutex};

pub const TILE: u32 = 256;

/// Backing storage for a `Raster`: the pixels, plus a memoized single-step box-reduction so
/// `halved()` computed once for a given pixel buffer is shared by every clone of it (and by
/// every clone of the halved result in turn), instead of being recomputed on every frame.
/// `std::sync::Mutex` is fine here: the engine runs single-threaded on wasm32.
struct RasterInner { data: Vec<u8>, half: Mutex<Option<Raster>> }

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
    fn eq(&self, other: &Self) -> bool { self.width == other.width && self.height == other.height && self.inner.data == other.inner.data }
}

impl Raster {
    fn wrap(width: u32, height: u32, data: Vec<u8>) -> Raster {
        Raster { width, height, inner: Arc::new(RasterInner { data, half: Mutex::new(None) }) }
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
        let w = (self.width / 2).max(1); let h = (self.height / 2).max(1);
        let mut out = vec![0u8; (w * h * 4) as usize];
        for y in 0..h { for x in 0..w {
            let mut sum = [0u32; 4]; let mut n = 0;
            for dy in 0..2 { for dx in 0..2 {
                let sx = x * 2 + dx; let sy = y * 2 + dy;
                if sx < self.width && sy < self.height {
                    let p = self.pixel(sx, sy);
                    for c in 0..4 { sum[c] += p[c] as u32; }
                    n += 1;
                }
            }}
            let i = ((y * w + x) * 4) as usize;
            for c in 0..4 { out[i + c] = ((sum[c] + n.max(1) / 2) / n.max(1)) as u8; }
        }}
        let result = Raster::from_premultiplied(w, h, out);
        *cache = Some(result.clone());
        result
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
    pub fn is_uniform(&self) -> Option<u8> {
        let first = *self.data.first()?;
        self.data.iter().all(|&v| v == first).then_some(first)
    }
}
