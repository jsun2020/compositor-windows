//! SelectionClipboard.swift at v1.4.5: document-space, selection-clipped PNG
//! pixels. The ordinary Copy path excludes the layer's appearance; Copy Merged
//! includes the visible composite. Reading never changes history or selection.
use crate::*;
use uuid::Uuid;

pub struct CopiedPixels { pub raster: Raster, pub origin: Point }

pub fn region(doc: &Document) -> Result<Rect, CommandError> {
    let b = match &doc.selection {
        Some(s) => s.coverage_bounds().ok_or_else(|| CommandError::Refused(ops::selection::EMPTY_SELECTION.into()))?,
        None => Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 },
    };
    fn snap(v: f64) -> f64 { if (v - v.round()).abs() < 0.001 { v.round() } else { v } }
    let (x, y) = (snap(b.x).floor().max(0.0), snap(b.y).floor().max(0.0));
    let (right, bottom) = (snap(b.max_x()).ceil().min(doc.width as f64), snap(b.max_y()).ceil().min(doc.height as f64));
    if right <= x || bottom <= y { return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into())); }
    Ok(Rect { x, y, width: right - x, height: bottom - y })
}

pub fn copy(doc: &Document, id: Option<Uuid>, mask: bool, merged: bool) -> Result<CopiedPixels, CommandError> {
    let rect = region(doc)?;
    let (w, h) = (rect.width as u32, rect.height as u32);
    let mut bytes = if merged { compositor::composite(doc, rect, w, h).bytes().to_vec() } else {
        let layer = doc.layer(id.ok_or(CommandError::NoLayer)?).ok_or(CommandError::NoLayer)?;
        if mask {
            let m = layer.mask.as_ref().ok_or_else(|| CommandError::Refused("That layer has no mask".into()))?;
            let placement = m.placement.unwrap_or(layer.transform);
            let inverse = placement.pixel_to_document(m.pixels.width, m.pixels.height).invert().ok_or_else(|| CommandError::Argument("invalid mask placement".into()))?;
            let mut out = vec![0; w as usize * h as usize * 4];
            let background = if m.placement.is_none() { 0.0 } else { m.background() as f32 / 255.0 };
            for y in 0..h { for x in 0..w {
                let p = inverse.apply(Point { x: rect.x + x as f64 + 0.5, y: rect.y + y as f64 + 0.5 });
                let v = if p.x < 0.0 || p.y < 0.0 || p.x >= m.pixels.width as f64 || p.y >= m.pixels.height as f64 { background }
                    else { compositor::gray_sample(&m.pixels, p.x, p.y, placement.sampling == Sampling::Nearest) };
                let v = (v * 255.0).round() as u8;
                let i = (y as usize * w as usize + x as usize) * 4;
                out[i..i + 4].copy_from_slice(&[v, v, v, 255]);
            }}
            out
        } else {
            if layer.is_group || layer.extra.adjustment.is_some() { return Err(CommandError::Refused("Select a pixel layer to copy".into())); }
            let mut raw = layer.clone(); raw.opacity = 1.0;
            let mut out = vec![0; w as usize * h as usize * 4];
            compositor::render_layer(&mut out, w, h, rect, &raw);
            out
        }
    };
    if let Some(s) = &doc.selection {
        let clip = SelectionClip::new(s, doc.width, doc.height);
        for y in 0..h { for x in 0..w {
            let a = clip.at(Point { x: rect.x + x as f64 + 0.5, y: rect.y + y as f64 + 0.5 });
            let i = (y as usize * w as usize + x as usize) * 4;
            for v in &mut bytes[i..i + 4] { *v = (*v as f32 * a).round() as u8; }
        }}
    }
    Ok(CopiedPixels { raster: Raster::from_premultiplied(w, h, bytes), origin: Point { x: rect.x, y: rect.y } })
}

pub fn paste(doc: &mut Document, raster: Raster, origin: Option<Point>) -> Result<Uuid, CommandError> {
    if raster.width as i64 > MAX_SIDE || raster.height as i64 > MAX_SIDE || raster.width as u64 * raster.height as u64 > MAX_PIXELS.saturating_sub(doc.used_pixels()) {
        return Err(CommandError::Import(ImportError::TooLarge));
    }
    let origin = origin.unwrap_or(Point { x: ((doc.width as f64 - raster.width as f64) / 2.0).floor(), y: ((doc.height as f64 - raster.height as f64) / 2.0).floor() });
    if !origin.x.is_finite() || !origin.y.is_finite() || origin.x.abs() > 1_000_000.0 || origin.y.abs() > 1_000_000.0 {
        return Err(CommandError::Argument("invalid clipboard origin".into()));
    }
    let layer = Layer::with_pixels(&ops::layers::next_layer_name(doc), raster, origin);
    let id = ops::layers::insert_above_active(doc, layer)?;
    doc.selection = None;
    Ok(id)
}

pub fn layer_via_copy(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    if doc.selection.is_none() { ops::hierarchy::duplicate_layer(doc, id)?; return Ok(()); }
    let copied = copy(doc, Some(id), mask, false)?;
    // Unlike Paste, Layer via Copy keeps the selection.
    let selection = doc.selection.clone();
    paste(doc, copied.raster, Some(copied.origin))?;
    doc.selection = selection;
    Ok(())
}
