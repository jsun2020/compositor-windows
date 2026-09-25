use crate::*;
use uuid::Uuid;

fn pixel_layer(doc: &Document, id: Uuid) -> Result<&Layer, CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if layer.is_group { return Err(CommandError::Argument("folders have no pixels".into())); }
    if layer.pixels.is_none() { return Err(CommandError::Argument("the layer has no pixels".into())); }
    Ok(layer)
}

/// Where the layer's pixel grid sits in document space: the origin of pixel (0, 0) and how many
/// document units one pixel covers. Grain is fixed in document space, so it reads these.
fn placement(layer: &Layer) -> (Point, f64) {
    let (w, _h) = layer.pixels.as_ref().map_or((1, 1), |p| (p.width, p.height));
    (layer.transform.origin, layer.transform.size.width / w.max(1) as f64)
}

pub fn apply_adjustment_to_layer(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
    if a.kind.is_spatial() { return Err(CommandError::Argument("a blur is applied with Filter > Gaussian Blur or Motion Blur".into())); }
    let layer = pixel_layer(doc, id)?;
    let (origin, units) = placement(layer);
    let raster = layer.pixels.as_ref().unwrap();
    // Kernel function, not this module's own `apply_filter`: adjust::apply::apply_adjustment.
    let adjusted = adjust::apply::apply_adjustment(raster, a, origin, units, None);
    doc.layer_mut(id).unwrap().set_pixels(Some(adjusted));
    Ok(())
}

pub fn invert_layer(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    if mask {
        let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
        let Some(m) = &layer.mask else { return Err(CommandError::Argument("the layer has no mask".into())); };
        let inverted = adjust::tonal::invert_gray(&m.pixels);
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = inverted;
        return Ok(());
    }
    let layer = pixel_layer(doc, id)?;
    let inverted = adjust::tonal::invert_raster(layer.pixels.as_ref().unwrap());
    doc.layer_mut(id).unwrap().set_pixels(Some(inverted));
    Ok(())
}

/// The raster padded by `margin` pixels on every side, with the transform that keeps the old
/// pixels exactly where they were.
pub fn grown(raster: &Raster, transform: &LayerTransform, margin: f64) -> Option<(Raster, LayerTransform)> {
    let m = margin.ceil().max(0.0) as u32;
    if m == 0 { return None; }
    let (w, h) = (raster.width + 2 * m, raster.height + 2 * m);
    if w as i64 > MAX_SIDE || h as i64 > MAX_SIDE || (w as u64) * (h as u64) > MAX_PIXELS { return None; }
    let mut data = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..raster.height {
        let src = ((y * raster.width) * 4) as usize;
        let dst = (((y + m) * w + m) * 4) as usize;
        data[dst..dst + (raster.width * 4) as usize].copy_from_slice(&raster.bytes()[src..src + (raster.width * 4) as usize]);
    }
    Some((Raster::from_premultiplied(w, h, data), placed_like(transform, raster.width, raster.height, w, h, m as f64, m as f64)))
}

/// The transform for a `new_w` x `new_h` grid whose old grid sits at (`offset_x`, `offset_y`),
/// keeping every old pixel over the same document point.
fn placed_like(transform: &LayerTransform, old_w: u32, old_h: u32, new_w: u32, new_h: u32, offset_x: f64, offset_y: f64) -> LayerTransform {
    let sx = transform.size.width / old_w as f64;
    let sy = transform.size.height / old_h as f64;
    let mut result = *transform;
    result.size = Size { width: new_w as f64 * sx, height: new_h as f64 * sy };
    // The old grid's centre in document space stays put; the new centre is offset from it.
    let to_document = transform.pixel_to_document(old_w, old_h);
    let middle = to_document.apply(Point { x: new_w as f64 / 2.0 - offset_x, y: new_h as f64 / 2.0 - offset_y });
    result.origin = Point { x: middle.x - result.size.width / 2.0, y: middle.y - result.size.height / 2.0 };
    result
}

/// The raster cropped to the pixels that are actually there, with the transform that keeps them in place.
pub fn trimmed(raster: &Raster, transform: &LayerTransform) -> (Raster, LayerTransform) {
    let full = (0, 0, raster.width, raster.height);
    match compositor::alpha_bounds(raster) {
        Some(crop) if crop != full => {
            let (w, h) = (crop.2 - crop.0, crop.3 - crop.1);
            let cropped = raster.cropped(crop.0, crop.1, w, h);
            let placed = placed_like(transform, raster.width, raster.height, w, h, -(crop.0 as f64), -(crop.1 as f64));
            (cropped, placed)
        }
        _ => (raster.clone(), *transform),
    }
}

/// A covering mask resampled onto the grid `new` places, its background beyond the old edge.
fn carry_mask(mask: &Mask, old: &LayerTransform, new: &LayerTransform) -> GrayRaster {
    let ratio_w = new.size.width / old.size.width;
    let ratio_h = new.size.height / old.size.height;
    let w = ((mask.pixels.width as f64 * ratio_w).round() as u32).clamp(1, MAX_SIDE as u32);
    let h = ((mask.pixels.height as f64 * ratio_h).round() as u32).clamp(1, MAX_SIDE as u32);
    let background = mask.background();
    let to_document = new.pixel_to_document(w, h);
    let from_document = old.pixel_to_document(mask.pixels.width, mask.pixels.height).invert();
    let mut data = vec![background; (w as usize) * (h as usize)];
    if let Some(inverse) = from_document {
        for y in 0..h { for x in 0..w {
            let p = inverse.apply(to_document.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 }));
            if p.x < 0.0 || p.y < 0.0 || p.x >= mask.pixels.width as f64 || p.y >= mask.pixels.height as f64 { continue; }
            data[(y * w + x) as usize] = mask.pixels.bytes()[(p.y as u32 * mask.pixels.width + p.x as u32) as usize];
        }}
    }
    GrayRaster::from_bytes(w, h, data)
}

/// One filter on a layer: a blur is given room to spread, run, then cut back to what it left.
pub fn apply_filter(doc: &mut Document, id: Uuid, params: &FilterParams) -> Result<(), CommandError> {
    let params = params.normalized();
    if params.is_identity() { return Ok(()); }
    let layer = pixel_layer(doc, id)?.clone();
    let raster = layer.pixels.as_ref().unwrap();
    let (source, placed) = match params.spreads() {
        true => grown(raster, &layer.transform, params.margin()).ok_or(CommandError::Project(ProjectError::TooLarge))?,
        false => (raster.clone(), layer.transform),
    };
    // Kernel function, not this module's own `apply_filter`: adjust::filters::apply_filter.
    let filtered = adjust::filters::apply_filter(&source, &params);
    let (result, transform) = if params.spreads() { trimmed(&filtered, &placed) } else { (filtered, placed) };
    let mask = match &layer.mask {
        Some(m) if m.placement.is_none() && !m.is_uniform() && transform != layer.transform => {
            Some(Mask { pixels: carry_mask(m, &layer.transform, &transform), ..m.clone() })
        }
        other => other.clone(),
    };
    let target = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    target.set_pixels(Some(result));
    target.transform = transform;
    if mask != layer.mask { target.set_mask(mask); }
    Ok(())
}

/// A new adjustment layer above the active layer (inside it when a folder is active).
pub fn add_adjustment_layer(doc: &mut Document, kind: AdjustmentKind, seed: u32, gradient: Option<([f64; 3], [f64; 3])>) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut adjustment = LayerAdjustment::new(kind);
    match kind {
        AdjustmentKind::GradientMap => {
            let (shadows, highlights) = gradient.unwrap_or(([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
            adjustment.gradient_map_settings = Some(adjust::tonal::gradient_map_from(shadows, highlights));
        }
        AdjustmentKind::Grain => adjustment.grain_settings = Some(GrainSettings { seed, ..GrainSettings::default() }),
        // Each Add Noise layer gets a pattern of its own (LayerAdjustment.swift:194).
        AdjustmentKind::AddNoise => adjustment.noise_seed = Some(seed),
        _ => {}
    }
    let mut layer = Layer::blank(kind.name(), doc.size());
    layer.extra.adjustment = Some(adjustment);
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    let id = layer.id;
    doc.layers.insert(insertion, layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}

pub fn set_adjustment(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if !layer.is_adjustment() { return Err(CommandError::Argument("not an adjustment layer".into())); }
    if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
    doc.layer_mut(id).unwrap().extra.adjustment = Some(a.clone());
    Ok(())
}
