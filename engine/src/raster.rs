use std::sync::Arc;

pub const TILE: u32 = 256;

/// Premultiplied RGBA8, row-major, top-left origin. Cloning shares the pixels.
#[derive(Clone, Debug)]
pub struct Raster { pub width: u32, pub height: u32, data: Arc<Vec<u8>> }

impl PartialEq for Raster {
    fn eq(&self, other: &Self) -> bool { self.width == other.width && self.height == other.height && self.data == other.data }
}

impl Raster {
    pub fn new_transparent(width: u32, height: u32) -> Raster {
        Raster { width, height, data: Arc::new(vec![0; (width as usize) * (height as usize) * 4]) }
    }
    pub fn from_premultiplied(width: u32, height: u32, data: Vec<u8>) -> Raster {
        assert_eq!(data.len(), (width as usize) * (height as usize) * 4);
        Raster { width, height, data: Arc::new(data) }
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
    pub fn bytes(&self) -> &[u8] { &self.data }
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        [self.data[i], self.data[i + 1], self.data[i + 2], self.data[i + 3]]
    }
    pub fn to_straight(&self) -> Vec<u8> {
        let mut out = self.data.as_ref().clone();
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
    pub fn same_pixels(&self, other: &Raster) -> bool { Arc::ptr_eq(&self.data, &other.data) }
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
            out[dst..dst + w * 4].copy_from_slice(&self.data[src..src + w * 4]);
        }
    }
    /// Box-filtered 2x reduction (odd edges average the pixels that exist).
    pub fn halved(&self) -> Raster {
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
            for c in 0..4 { out[i + c] = ((sum[c] + n / 2) / n) as u8; }
        }}
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
