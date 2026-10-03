//! FloatingSelection.swift v1.4.5. Temporary pixels are never a history entry;
//! a cancelled or identity transform restores the exact original document.
use crate::*;
use uuid::Uuid;

pub struct FloatingEdit { pub before: Document, pub source: Uuid, pub layer: Uuid, pub original: LayerTransform, pub duplicated: bool }

pub fn begin(doc: &mut Document, source: Uuid, duplicate: bool, clips: &SelectionClips) -> Result<FloatingEdit, CommandError> {
    if doc.selection.is_none() { return Err(CommandError::Refused(ops::selection::NO_SELECTION.into())); }
    let original = doc.layer(source).ok_or(CommandError::NoLayer)?.clone();
    if original.is_group || original.is_adjustment() || original.pixels.is_none() { return Err(CommandError::Refused("Select a pixel layer to transform the selection".into())); }
    let copied = ops::clipboard::copy(doc, Some(source), false, false)?;
    let before = doc.clone();
    if !duplicate { ops::selection::clear_selected(doc, clips, source, false)?; }
    let mut floating = Layer::with_pixels("Floating Selection", copied.raster, copied.origin);
    floating.parent_id = original.parent_id; floating.opacity = original.opacity; floating.blend_mode = original.blend_mode;
    let layer = floating.id; let transform = floating.transform;
    let index = doc.index_of(source).ok_or(CommandError::NoLayer)?;
    // The transient copy is not a saved layer and does not consume the saved-document
    // pixel budget. The final merged layer is checked separately in finish().
    if doc.layers.len() >= MAX_LAYERS || floating.pixels.as_ref().unwrap().width as u64 * floating.pixels.as_ref().unwrap().height as u64 > MAX_PIXELS {
        return Err(CommandError::Project(ProjectError::TooLarge));
    }
    doc.layers.insert(index + 1, floating); doc.active_layer_id = Some(layer);
    Ok(FloatingEdit { before, source, layer, original: transform, duplicated: duplicate })
}

pub fn finish(doc: &Document, edit: &FloatingEdit, draft: LayerTransform, corners: Option<[Point; 4]>) -> Result<Document, CommandError> {
    if !draft.is_valid() { return Err(CommandError::Argument("invalid floating transform".into())); }
    let identity = draft.same_placement(&edit.original) && corners.as_ref().map_or(true, |c| *c == edit.original.corners());
    if identity { return Ok(edit.before.clone()); }
    let floated = doc.layer(edit.layer).ok_or(CommandError::NoLayer)?;
    let pixels = floated.pixels.as_ref().ok_or(CommandError::NoLayer)?;
    let (pixels, transform) = if let Some(c) = &corners { let (p, t, _) = ops::distort::warp_trimmed(pixels, &draft, c, draft.sampling == Sampling::Nearest)?; (p, t) } else { (pixels.clone(), draft) };
    let mut result = doc.clone();
    let source = result.layer(edit.source).ok_or(CommandError::NoLayer)?.clone();
    let own = source.pixels.as_ref().ok_or(CommandError::NoLayer)?;
    let to_document = source.transform.pixel_to_document(own.width, own.height);
    let inverse = to_document.invert().ok_or(ProjectError::Invalid)?;
    let c = transform.corners().map(|p| inverse.apply(p));
    fn snap(x: f64) -> f64 { if (x - x.round()).abs() < 1e-6 { x.round() } else { x } }
    let x0 = snap(c.iter().map(|p| p.x).fold(0.0, f64::min)).floor();
    let y0 = snap(c.iter().map(|p| p.y).fold(0.0, f64::min)).floor();
    let x1 = snap(c.iter().map(|p| p.x).fold(own.width as f64, f64::max)).ceil();
    let y1 = snap(c.iter().map(|p| p.y).fold(own.height as f64, f64::max)).ceil();
    let (w, h) = (x1 - x0, y1 - y0);
    let other = edit.before.used_pixels() - own.width as u64 * own.height as u64;
    if w > MAX_SIDE as f64 || h > MAX_SIDE as f64 || w * h > MAX_PIXELS.saturating_sub(other) as f64 { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let (w, h) = (w as u32, h as u32);
    let grid = ops::raster_edit::EditGrid { width: w, height: h, x: (-x0) as u32, y: (-y0) as u32,
        transform: ops::adjust::placed_like(&source.transform, own.width, own.height, w, h, -x0, -y0) };
    let mut bytes = ops::raster_edit::pixels_on(&grid, Some(own));
    let target_density=own.width as f64/source.transform.size.width.max(1e-9);
    let level=if transform.sampling==Sampling::Nearest{0}else{compositor::prefilter_level(pixels.width,pixels.height,pixels.width as f64/(transform.size.width*target_density).max(1e-9))};
    let pixels=pixels.reduced(level);
    let mapping = transform.pixel_to_document(pixels.width, pixels.height).invert().ok_or(ProjectError::Invalid)?.then(grid.transform.pixel_to_document(w, h));
    for y in 0..h { for x in 0..w {
        let p = mapping.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
        if p.x < 0.0 || p.y < 0.0 || p.x >= pixels.width as f64 || p.y >= pixels.height as f64 { continue; }
        let sample = compositor::sample(&pixels, p.x, p.y, transform.sampling == Sampling::Nearest);
        let i = (y as usize * w as usize + x as usize) * 4;
        for k in 0..4 { bytes[i + k] = (sample[k] * 255.0 + bytes[i + k] as f32 * (1.0 - sample[3])).round().clamp(0.0, 255.0) as u8; }
    }}
    let target = result.layer_mut(edit.source).unwrap();
    target.set_pixels(Some(Raster::from_premultiplied(w, h, bytes))); target.transform = grid.transform;
    if let Some(mask) = &source.mask {
        if mask.placement.is_none() {
            target.mask_mut().unwrap().pixels = ops::raster_edit::followed(&mask.pixels, (own.width, own.height), &grid, (0, 0, w, h));
        }
    }
    // set_pixels drops live text/shape but deliberately retains layer effects.
    result.layers.retain(|l| l.id != edit.layer); result.active_layer_id = Some(edit.source);
    if let Some(s) = &edit.before.selection {
        let from = edit.original.unit_to_document().invert().ok_or(ProjectError::Invalid)?;
        let contours = if let Some(c) = &corners {
            let map = Homography::unit_to(c);
            s.contours.iter().map(|c| c.iter().map(|p| {
                let original = Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL };
                selection::geometry::quantize(map.apply(from.apply(original)))
            }).collect()).collect()
        } else { selection::geometry::transformed(&s.contours, &draft.unit_to_document().then(from)) };
        result.selection = Some(Selection::new(contours, s.antialiased, s.feather));
    }
    Ok(result)
}
