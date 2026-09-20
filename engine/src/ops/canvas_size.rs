use crate::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasSizeOptions {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-left (0) through bottom-right (8); 4 is the center.
    pub anchor: u8,
    pub fill: Option<[f64; 3]>,
    /// Crop supplies an explicit document-space translation instead of an anchor.
    pub content_offset: Option<Point>,
}

impl CanvasSizeOptions {
    pub fn offset(&self, from_width: u32, from_height: u32) -> Point {
        if let Some(o) = self.content_offset { return o; }
        // Floor puts the extra pixel on the right/bottom when expanding and removes it from the left/top when shrinking.
        Point {
            x: ((self.width as f64 - from_width as f64) * (self.anchor % 3) as f64 / 2.0).floor(),
            y: ((self.height as f64 - from_height as f64) * (self.anchor / 3) as f64 / 2.0).floor(),
        }
    }
}

pub fn canvas_size(doc: &Document, options: CanvasSizeOptions) -> Result<Document, ProjectError> {
    let w = options.width as i64; let h = options.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || options.anchor > 8 { return Err(ProjectError::TooLarge); }
    let offset = options.offset(doc.width, doc.height);
    if !offset.x.is_finite() || !offset.y.is_finite() || offset.x.abs() > 1_000_000.0 || offset.y.abs() > 1_000_000.0 {
        return Err(ProjectError::Invalid);
    }
    if options.width == doc.width && options.height == doc.height && offset == (Point { x: 0.0, y: 0.0 }) {
        return Ok(doc.clone());
    }
    let mut out = doc.clone();
    out.width = options.width;
    out.height = options.height;
    for layer in &mut out.layers {
        layer.transform.origin.x += offset.x;
        layer.transform.origin.y += offset.y;
        if !layer.transform.is_valid() { return Err(ProjectError::TooLarge); }
        if let Some(mask) = &mut layer.mask {
            if let Some(p) = &mut mask.placement { p.origin.x += offset.x; p.origin.y += offset.y; }
        }
    }
    // A colored extension is separate bottom-layer content; the old canvas area stays transparent.
    if let Some(color) = options.fill {
        if options.width > doc.width || options.height > doc.height {
            let pixels = options.width as u64 * options.height as u64;
            if pixels > MAX_PIXELS - doc.used_pixels() || out.layers.len() >= MAX_LAYERS { return Err(ProjectError::TooLarge); }
            if color.iter().any(|c| !c.is_finite() || !(0.0..=1.0).contains(c)) { return Err(ProjectError::Invalid); }
            let rgb = color.map(|c| (c * 255.0).round() as u8);
            let mut data = vec![0u8; (pixels * 4) as usize];
            let hole_x0 = offset.x.max(0.0) as i64; let hole_y0 = offset.y.max(0.0) as i64;
            let hole_x1 = (offset.x + doc.width as f64).min(options.width as f64) as i64;
            let hole_y1 = (offset.y + doc.height as f64).min(options.height as f64) as i64;
            for y in 0..options.height as i64 {
                for x in 0..options.width as i64 {
                    if x >= hole_x0 && x < hole_x1 && y >= hole_y0 && y < hole_y1 { continue; }
                    let i = ((y * options.width as i64 + x) * 4) as usize;
                    data[i] = rgb[0]; data[i + 1] = rgb[1]; data[i + 2] = rgb[2]; data[i + 3] = 255;
                }
            }
            let raster = Raster::from_premultiplied(options.width, options.height, data);
            out.layers.insert(0, Layer::with_pixels("Canvas Extension", raster, Point { x: 0.0, y: 0.0 }));
        }
    }
    Ok(out)
}
