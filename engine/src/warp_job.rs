//! A dedicated worker owns the snapshot for one gesture. Interactive previews
//! sample sparse GPU tiles at bounded resolution; completion keeps the full grid.
use crate::*;
pub type WarpOutput = (
    JobOutput,
    Option<Raster>,
    Option<GrayRaster>,
    Option<Raster>,
);
pub struct WarpJob {
    input: JobInput,
    document: Document,
    source: Raster,
    clips: SelectionClips,
}
impl WarpJob {
    pub fn new(
        input: JobInput,
        pixels: Option<Raster>,
        mask: Option<GrayRaster>,
        points: Option<&[i32]>,
    ) -> Result<Self, CommandError> {
        jobs::check_warp_canvas(&input, false)?;
        let document = input.document(pixels, mask, points)?;
        let source = ops::warp::source_plane(&document, input.layer.id)?;
        Ok(Self {
            input,
            document,
            source,
            clips: SelectionClips::default(),
        })
    }
    pub fn source(&self) -> Raster {
        self.source.clone()
    }
    pub fn reference(&self, spec: &WarpSpec, out_per_doc: f64) -> Result<WarpOutput, CommandError> {
        let mut doc = self.document.clone();
        let old = doc.layer(self.input.layer.id).unwrap().clone();
        let dirty = ops::warp::apply_reference(&mut doc, &self.clips, old.id, false, spec)?;
        jobs::edit_output(
            &doc,
            old.id,
            old.pixels,
            old.mask.map(|m| m.pixels),
            dirty,
            out_per_doc,
        )
    }
    pub fn preview(
        &self,
        spec: &WarpSpec,
        tiles: Vec<(WarpTileRect, Vec<u8>)>,
    ) -> Result<Option<WarpOutput>, CommandError> {
        let dabs = ops::warp::footprint(spec)?;
        jobs::check_warp_tiles(self.input.width, self.input.height, &tiles)?;
        if dabs.is_empty() || tiles.is_empty() {
            return Ok(None);
        }
        let own = self.document.layer(self.input.layer.id).unwrap();
        let pixels = own.pixels.as_ref().unwrap();
        let radius = spec.diameter * 0.5 + 3.0;
        let left = (dabs
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min)
            .floor()
            - radius)
            .max(0.0);
        let top = (dabs
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min)
            .floor()
            - radius)
            .max(0.0);
        let right = (dabs
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            + radius)
            .min(self.input.width as f64);
        let bottom = (dabs
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            + radius)
            .min(self.input.height as f64);
        if right <= left || bottom <= top {
            return Ok(None);
        }
        let grid = ops::brush::grid_for(
            &self.document,
            own,
            Rect {
                x: left,
                y: top,
                width: right - left,
                height: bottom - top,
            },
        )?;
        let (mut w, mut h, mut level) = (grid.width, grid.height, 0);
        while w.max(h) > 1024 {
            w = (w / 2).max(1);
            h = (h / 2).max(1);
            level += 1;
        }
        let brush = BrushSpec {
            diameter: spec.diameter + 4.0,
            hardness: 1.0,
            opacity: 1.0,
            points: dabs,
            color: [0.0; 4],
            erasing: false,
            operation: BrushOperation::Paint,
        };
        let (rect, coverage) =
            ops::brush::coverage(&brush, &self.document, &self.clips, grid.transform, w, h);
        let to_doc = grid.transform.pixel_to_document(w, h);
        let inverse = own
            .transform
            .pixel_to_document(pixels.width, pixels.height)
            .invert()
            .unwrap();
        let mut base = pixels.clone();
        for _ in 0..level {
            base = base.halved();
        }
        // Reuse the cached reduction when the pixel grid did not grow. Only
        // the brush rectangle needs virtual-source samples on each update.
        let mut data = if grid.transform == own.transform && (w, h) == (base.width, base.height) {
            base.bytes().to_vec()
        } else {
            let mut data = vec![0u8; w as usize * h as usize * 4];
            for y in 0..h {
                for x in 0..w {
                    let local = inverse.apply(to_doc.apply(Point {
                        x: x as f64 + 0.5,
                        y: y as f64 + 0.5,
                    }));
                    if local.x >= 0.0
                        && local.y >= 0.0
                        && local.x < pixels.width as f64
                        && local.y < pixels.height as f64
                    {
                        let color = compositor::sample(
                            &base,
                            local.x * base.width as f64 / pixels.width as f64,
                            local.y * base.height as f64 / pixels.height as f64,
                            own.transform.sampling == Sampling::Nearest,
                        );
                        let at = ((y * w + x) * 4) as usize;
                        for k in 0..4 {
                            data[at + k] = (color[k] * 255.0).round().clamp(0.0, 255.0) as u8;
                        }
                    }
                }
            }
            data
        };
        let fetch = |x: i64, y: i64| {
            let x = x.clamp(0, self.source.width as i64 - 1) as u32;
            let y = y.clamp(0, self.source.height as i64 - 1) as u32;
            for (r, b) in &tiles {
                if x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height {
                    let i = (((y - r.y) * r.width + x - r.x) * 4) as usize;
                    return [b[i], b[i + 1], b[i + 2], b[i + 3]];
                }
            }
            self.source.pixel(x, y)
        };
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                let amount = coverage[((y - rect.y) * rect.width + x - rect.x) as usize] as f64;
                if amount <= 0.0 {
                    continue;
                }
                let p = to_doc.apply(Point {
                    x: x as f64 + 0.5,
                    y: y as f64 + 0.5,
                });
                let mut color = [0.0; 4];
                if amount > 0.0 {
                    let fx = p.x - 0.5;
                    let fy = p.y - 0.5;
                    let ix = fx.floor() as i64;
                    let iy = fy.floor() as i64;
                    let tx = (fx - ix as f64) as f32;
                    let ty = (fy - iy as f64) as f32;
                    let a = fetch(ix, iy);
                    let b = fetch(ix + 1, iy);
                    let c = fetch(ix, iy + 1);
                    let d = fetch(ix + 1, iy + 1);
                    for k in 0..4 {
                        color[k] = (a[k] as f32 / 255.0 * (1.0 - tx) + b[k] as f32 / 255.0 * tx)
                            * (1.0 - ty)
                            + (c[k] as f32 / 255.0 * (1.0 - tx) + d[k] as f32 / 255.0 * tx) * ty;
                    }
                }
                let at = ((y * w + x) * 4) as usize;
                for k in 0..4 {
                    data[at + k] = (color[k] as f64 * 255.0 * amount
                        + data[at + k] as f64 * (1.0 - amount))
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
            }
        }
        let followed = own
            .mask
            .as_ref()
            .filter(|m| {
                grid.transform != own.transform
                    && m.placement.is_none()
                    && m.pixels.is_uniform() != Some(255)
            })
            .map(|m| {
                let mut b = vec![255; w as usize * h as usize];
                for y in 0..h {
                    for x in 0..w {
                        let p = inverse.apply(to_doc.apply(Point {
                            x: x as f64 + 0.5,
                            y: y as f64 + 0.5,
                        }));
                        if p.x >= 0.0
                            && p.y >= 0.0
                            && p.x < pixels.width as f64
                            && p.y < pixels.height as f64
                        {
                            let sx =
                                (p.x * m.pixels.width as f64 / pixels.width as f64).floor() as u32;
                            let sy = (p.y * m.pixels.height as f64 / pixels.height as f64).floor()
                                as u32;
                            b[(y * w + x) as usize] =
                                m.pixels.bytes()[(sy * m.pixels.width + sx) as usize];
                        }
                    }
                }
                GrayRaster::from_bytes(w, h, b)
            });
        let output = JobOutput {
            transform: grid.transform,
            mask_placement: own.mask.as_ref().and_then(|m| m.placement),
            pixels: Some((w, h)),
            mask: followed.as_ref().map(|m| (m.width, m.height)),
            regions: vec![],
            display: None,
            witness: None,
        };
        Ok(Some((
            output,
            Some(Raster::from_premultiplied(w, h, data)),
            followed,
            None,
        )))
    }
    pub fn finish(
        mut self,
        spec: &WarpSpec,
        tiles: Vec<(WarpTileRect, Vec<u8>)>,
        out_per_doc: f64,
    ) -> Result<WarpOutput, CommandError> {
        let dabs = ops::warp::footprint(spec)?;
        jobs::check_warp_tiles(self.input.width, self.input.height, &tiles)?;
        let old = self.document.layer(self.input.layer.id).unwrap().clone();
        let dirty = if dabs.is_empty() || tiles.is_empty() {
            Dirty::pixels(vec![])
        } else {
            let mut bytes = self.source.into_bytes();
            jobs::overlay_warp_tiles(&mut bytes, self.input.width, tiles);
            let result = Raster::from_premultiplied(self.input.width, self.input.height, bytes);
            ops::warp::writeback(
                &mut self.document,
                &self.clips,
                old.id,
                spec,
                &dabs,
                &result,
                false,
            )?
        };
        jobs::edit_output(
            &self.document,
            old.id,
            old.pixels,
            old.mask.map(|m| m.pixels),
            dirty,
            out_per_doc,
        )
    }
}
