use crate::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Bilinear or nearest sample in premultiplied float RGBA (0..1). Outside the raster clamps to the edge; coverage is decided by the caller.
pub fn sample(raster: &Raster, x: f64, y: f64, nearest: bool) -> [f32; 4] {
    let w = raster.width as i64; let h = raster.height as i64;
    let fetch = |px: i64, py: i64| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h { return [0.0; 4]; }
        let p = raster.pixel(px as u32, py as u32);
        [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0, p[3] as f32 / 255.0]
    };
    if nearest {
        let px = x.floor() as i64; let py = y.floor() as i64;
        return fetch(px, py);
    }
    let fx = x - 0.5; let fy = y - 0.5;
    let x0 = fx.floor(); let y0 = fy.floor();
    let tx = (fx - x0) as f32; let ty = (fy - y0) as f32;
    let (mut x0, mut y0) = (x0 as i64, y0 as i64);
    x0 = x0.clamp(0, w - 1); let x1 = (x0 + 1).clamp(0, w - 1);
    y0 = y0.clamp(0, h - 1); let y1 = (y0 + 1).clamp(0, h - 1);
    let a = fetch(x0, y0); let b = fetch(x1, y0); let c = fetch(x0, y1); let d = fetch(x1, y1);
    let mut out = [0f32; 4];
    for i in 0..4 {
        out[i] = (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty;
    }
    out
}

/// Sharp halvings for large reductions: reduce until one output pixel covers at most 2 source pixels.
fn prefiltered(raster: &Raster, pixels_per_output: f64) -> (Raster, f64) {
    let mut current = raster.clone();
    let mut scale = 1.0;
    let mut factor = pixels_per_output;
    while factor > 2.0 && current.width > 1 && current.height > 1 {
        current = current.halved();
        scale *= 0.5;
        factor /= 2.0;
    }
    (current, scale)
}

/// Draws one layer into `target` (premultiplied RGBA8, `tw` x `th`) covering document `region`.
pub fn render_layer(target: &mut [u8], tw: u32, th: u32, region: Rect, layer: &Layer) {
    let Some(raster) = &layer.pixels else { return; };
    let out_per_doc_x = tw as f64 / region.width;
    let out_per_doc_y = th as f64 / region.height;
    let nearest = layer.transform.sampling == Sampling::Nearest;
    // Source pixels per output pixel along the layer's width, for prefiltering.
    let source_per_output = raster.width as f64 / (layer.transform.size.width * out_per_doc_x);
    let (source, scale) = if layer.transform.sampling == Sampling::High && source_per_output > 2.0 {
        prefiltered(raster, source_per_output)
    } else { (raster.clone(), 1.0) };
    let Some(inverse) = layer.transform.pixel_to_document(raster.width, raster.height).invert() else { return; };
    let opacity = layer.opacity.clamp(0.0, 1.0) as f32;
    // Bounding box of the layer in output pixels.
    let b = layer.transform.bounds();
    let x0 = (((b.x - region.x) * out_per_doc_x).floor() as i64 - 1).max(0) as u32;
    let y0 = (((b.y - region.y) * out_per_doc_y).floor() as i64 - 1).max(0) as u32;
    let x1 = (((b.max_x() - region.x) * out_per_doc_x).ceil() as i64 + 1).min(tw as i64).max(0) as u32;
    let y1 = (((b.max_y() - region.y) * out_per_doc_y).ceil() as i64 + 1).min(th as i64).max(0) as u32;
    for oy in y0..y1 {
        for ox in x0..x1 {
            let doc = Point { x: region.x + (ox as f64 + 0.5) / out_per_doc_x, y: region.y + (oy as f64 + 0.5) / out_per_doc_y };
            let p = inverse.apply(doc);
            if p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64 { continue; }
            let s = sample(&source, p.x * scale, p.y * scale, nearest);
            if s[3] <= 0.0 { continue; }
            let i = ((oy * tw + ox) * 4) as usize;
            let src_a = s[3] * opacity;
            for c in 0..3 {
                let dst = target[i + c] as f32 / 255.0;
                target[i + c] = ((s[c] * opacity + dst * (1.0 - src_a)) * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            let dst_a = target[i + 3] as f32 / 255.0;
            target[i + 3] = ((src_a + dst_a * (1.0 - src_a)) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Layers to draw, bottom to top: visible, with every ancestor visible, groups excluded.
pub fn render_layers(doc: &Document) -> Vec<&Layer> {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    doc.layers.iter().filter(|layer| {
        if layer.is_group { return false; }
        let mut node = Some(*layer);
        let mut steps = 0;
        while let Some(n) = node {
            if !n.visible || steps > MAX_NESTING { return false; }
            steps += 1;
            node = n.parent_id.and_then(|p| by_id.get(&p).copied());
        }
        true
    }).collect()
}

pub fn composite(doc: &Document, region: Rect, out_width: u32, out_height: u32) -> Raster {
    let mut target = vec![0u8; (out_width as usize) * (out_height as usize) * 4];
    for layer in render_layers(doc) {
        render_layer(&mut target, out_width, out_height, region, layer);
    }
    Raster::from_premultiplied(out_width, out_height, target)
}

fn check_export_size(doc: &Document) -> Result<(), ExportError> {
    let w = doc.width as i64; let h = doc.height as i64;
    if !(1..=MAX_SIDE).contains(&w) || !(1..=MAX_SIDE).contains(&h) || (w * h) as u64 > MAX_PIXELS { return Err(ExportError::TooLarge); }
    Ok(())
}

pub fn render_full(doc: &Document) -> Result<Raster, ExportError> {
    check_export_size(doc)?;
    Ok(composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height))
}

pub fn export_png(doc: &Document) -> Result<Vec<u8>, ExportError> {
    let raster = render_full(doc)?;
    encode_png(&raster, doc.resolution).map_err(|_| ExportError::Encode)
}

pub fn export_jpeg(doc: &Document, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, ExportError> {
    let raster = render_full(doc)?;
    encode_jpeg(&raster, quality, matte, doc.resolution)
}
