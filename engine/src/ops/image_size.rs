use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageSizeOptions { pub width: u32, pub height: u32, pub resolution: f64, pub sampling: Sampling }

/// Draws `layer` (with `sampling`) into an upright raster covering document rect `left,top,w,h` after the canvas
/// is scaled by `sx, sy`.
fn rasterize(layer: &Layer, raster: &Raster, sampling: Sampling, sx: f64, sy: f64, left: f64, top: f64, w: u32, h: u32) -> Raster {
    let mut probe = layer.clone();
    probe.transform.sampling = sampling;
    probe.opacity = 1.0;
    probe.visible = true;
    probe.parent_id = None;
    probe.set_pixels(Some(raster.clone()));
    // Output pixel (x, y) sits at document (left + x, top + y) in the new canvas, i.e. ((left + x) / sx, (top + y) / sy) in the old.
    let region = Rect { x: left / sx, y: top / sy, width: w as f64 / sx, height: h as f64 / sy };
    let mut target = vec![0u8; (w as usize) * (h as usize) * 4];
    compositor::render_layer(&mut target, w, h, region, &probe);
    Raster::from_premultiplied(w, h, target)
}

pub fn image_size(doc: &Document, options: ImageSizeOptions) -> Result<Document, ProjectError> {
    let w = options.width as i64; let h = options.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || !options.resolution.is_finite() || !(1.0..=9600.0).contains(&options.resolution) {
        return Err(ProjectError::TooLarge);
    }
    let mut out = doc.clone();
    out.resolution = options.resolution;
    if options.width == doc.width && options.height == doc.height { return Ok(out); }
    if (w * h) as u64 > MAX_PIXELS { return Err(ProjectError::TooLarge); }
    out.width = options.width;
    out.height = options.height;
    let sx = options.width as f64 / doc.width as f64;
    let sy = options.height as f64 / doc.height as f64;
    for g in &mut out.guides {
        match g.axis { GuideAxis::Vertical => g.position *= sx, GuideAxis::Horizontal => g.position *= sy }
        // The Mac's own saveable range (manifest.rs validate, R 1.2): refuse rather than move a
        // guide somewhere this build (which has no guide UI to delete it, only undo) could never
        // save again, the same way a resampled layer transform is already refused just below.
        if !g.position.is_finite() || g.position.abs() > 1_000_000.0 { return Err(ProjectError::TooLarge); }
    }
    let mut used = 0u64; let mut used_masks = 0u64;
    for layer in &mut out.layers {
        let corners = layer.transform.corners().map(|p| Point { x: p.x * sx, y: p.y * sy });
        let left = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min).floor();
        let top = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min).floor();
        let right = corners.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max).ceil();
        let bottom = corners.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max).ceil();
        let width = (right - left).max(1.0) as u32;
        let height = (bottom - top).max(1.0) as u32;
        let mut transform = LayerTransform::axis_aligned(Point { x: left, y: top }, Size { width: width as f64, height: height as f64 });
        transform.sampling = options.sampling;
        if !transform.is_valid() { return Err(ProjectError::TooLarge); }
        if let Some(raster) = layer.pixels.clone() {
            let pixels = width as u64 * height as u64;
            if width as i64 > MAX_SIDE || height as i64 > MAX_SIDE || pixels > MAX_PIXELS - used { return Err(ProjectError::TooLarge); }
            used += pixels;
            let resampled = rasterize(layer, &raster, options.sampling, sx, sy, left, top, width, height);
            layer.set_pixels(Some(resampled));
        }
        // Process mask if it exists; extract data before rasterizing to avoid borrow issues
        if let Some(mask) = &layer.mask {
            if (mask.pixels.width != 1 || mask.pixels.height != 1) && mask.placement.is_none() {
                let pixels = width as u64 * height as u64;
                if pixels > MAX_PIXELS - used_masks { return Err(ProjectError::TooLarge); }
                used_masks += pixels;
                // Coverage resamples like an opaque grey image: expand to RGBA, draw, take the red channel.
                let mask_bytes = mask.pixels.bytes().to_vec();
                let mask_w = mask.pixels.width;
                let mask_h = mask.pixels.height;
                let mut rgba = Vec::with_capacity(mask_bytes.len() * 4);
                for &v in &mask_bytes { rgba.extend_from_slice(&[v, v, v, 255]); }
                let as_raster = Raster::from_premultiplied(mask_w, mask_h, rgba);
                let drawn = rasterize(layer, &as_raster, options.sampling, sx, sy, left, top, width, height);
                let coverage: Vec<u8> = drawn.bytes().chunks_exact(4).map(|p| p[0]).collect();
                if let Some(m) = &mut layer.mask {
                    m.pixels = GrayRaster::from_bytes(width, height, coverage);
                }
            }
        }
        if let Some(m) = &mut layer.mask {
            if let Some(p) = &mut m.placement {
                let c = p.corners().map(|q| Point { x: q.x * sx, y: q.y * sy });
                let l = c.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
                let t = c.iter().map(|q| q.y).fold(f64::INFINITY, f64::min);
                let r = c.iter().map(|q| q.x).fold(f64::NEG_INFINITY, f64::max);
                let b = c.iter().map(|q| q.y).fold(f64::NEG_INFINITY, f64::max);
                p.origin = Point { x: l, y: t };
                p.size = Size { width: (r - l).max(1.0), height: (b - t).max(1.0) };
            }
        }
        layer.transform = transform;
    }
    Ok(out)
}
