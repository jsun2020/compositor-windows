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

/// The selection's coverage on a `width` x `height` grid that `transform` places (a layer's pixels,
/// a grown copy of them, or a mask): None when nothing is selected, so the edit reaches the whole
/// grid. An empty selection refuses the edit, as the Mac's `canAdjustColors` / `canInvert` do. The
/// canvas-grid clip comes from `clips` (the engine's, so a commit reuses its preview's).
pub fn edit_coverage(doc: &Document, clips: &SelectionClips, transform: &LayerTransform, width: u32, height: u32) -> Result<Option<GrayRaster>, CommandError> {
    if doc.selection.as_ref().map_or(false, |s| s.is_empty()) {
        return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into()));
    }
    Ok(selection_coverage_with(doc, clips, &transform.pixel_to_document(width, height), width, height))
}

/// `apply_adjustment_to_layer_with`, making the selection's clip afresh.
pub fn apply_adjustment_to_layer(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    apply_adjustment_to_layer_with(doc, &SelectionClips::default(), id, a)
}

pub fn apply_adjustment_to_layer_with(doc: &mut Document, clips: &SelectionClips, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
    if a.kind.is_spatial() { return Err(CommandError::Argument("a blur is applied with Filter > Gaussian Blur or Motion Blur".into())); }
    let layer = pixel_layer(doc, id)?;
    let (origin, units) = placement(layer);
    let raster = layer.pixels.as_ref().unwrap();
    let coverage = edit_coverage(doc, clips, &layer.transform, raster.width, raster.height)?;
    // Kernel function, not this module's own `apply_filter`: adjust::apply::apply_adjustment.
    let adjusted = adjust::apply::apply_adjustment(raster, a, origin, units, coverage.as_ref());
    doc.layer_mut(id).unwrap().set_pixels(Some(adjusted));
    Ok(())
}

/// Image > Invert on the layer's pixels or its mask, inside the selection when there is one
/// (`invertPixels`, SelectionEdits.swift:85-122). A uniform 1x1 mask cannot hold a partial
/// selection, so it takes the layer's pixel grid first.
pub fn invert_layer(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    invert_layer_with(doc, &SelectionClips::default(), id, mask)
}

/// `invert_layer` with the selection's clip from `clips`.
pub fn invert_layer_with(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool) -> Result<(), CommandError> {
    if mask {
        let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
        if layer.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
        if doc.selection.as_ref().map_or(false, |s| s.is_empty()) { return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into())); }
        if doc.selection.is_some() { ops::masks::expand_uniform(doc, id)?; }
        let layer = doc.layer(id).unwrap();
        let m = layer.mask.as_ref().unwrap();
        let grid = m.placement.unwrap_or(layer.transform);
        let coverage = edit_coverage(doc, clips, &grid, m.pixels.width, m.pixels.height)?;
        let inverted = adjust::tonal::invert_gray(&m.pixels);
        let result = match coverage { Some(c) => adjust::apply::blend_gray_by_coverage(&inverted, &m.pixels, &c), None => inverted };
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = result;
        return Ok(());
    }
    let layer = pixel_layer(doc, id)?;
    let raster = layer.pixels.as_ref().unwrap();
    let coverage = edit_coverage(doc, clips, &layer.transform, raster.width, raster.height)?;
    let inverted = adjust::tonal::invert_raster(raster);
    let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&inverted, raster, &c), None => inverted };
    doc.layer_mut(id).unwrap().set_pixels(Some(result));
    Ok(())
}

/// The raster padded by `margin` pixels on every side, with the transform that keeps the old
/// pixels exactly where they were.
pub fn grown(raster: &Raster, transform: &LayerTransform, margin: f64) -> Option<(Raster, LayerTransform)> {
    let m = margin.ceil().max(0.0) as u32;
    if m == 0 { return None; }
    let (w, h) = (raster.width + 2 * m, raster.height + 2 * m);
    if w as i64 > MAX_SIDE || h as i64 > MAX_SIDE || (w as u64) * (h as u64) > MAX_PIXELS { return None; }
    Some((raster.padded(w,h,m,m), placed_like(transform, raster.width, raster.height, w, h, m as f64, m as f64)))
}

pub fn filter_canvas_sides(doc:&Document,transform:&LayerTransform,w:u32,h:u32,p:&FilterParams)->adjust::filters::CanvasSides {
 use adjust::filters::CanvasSides;
 if transform.rotation!=0. || !matches!(p,FilterParams::GaussianBlur{preserve_transparency:false,..}|FilterParams::MotionBlur{..}|FilterParams::ProgressiveBlur{..}|FilterParams::Sharpen{..}) {return CanvasSides::default();}
 let Some(inv)=transform.pixel_to_document(w,h).invert()else{return CanvasSides::default();};
 let a=inv.apply(Point{x:0.,y:0.});let b=inv.apply(Point{x:doc.width as f64,y:doc.height as f64});
 CanvasSides{left:a.x.min(b.x)>=-0.5,right:a.x.max(b.x)<=w as f64+0.5,top:a.y.min(b.y)>=-0.5,bottom:a.y.max(b.y)<=h as f64+0.5}
}
pub fn grown_filter(r:&Raster,t:&LayerTransform,margin:f64,sides:adjust::filters::CanvasSides)->Option<(Raster,LayerTransform)>{
 let m=margin.ceil().max(0.)as u32;
 let(l,rr,top,bottom)=(if sides.left{0}else{m},if sides.right{0}else{m},if sides.top{0}else{m},if sides.bottom{0}else{m});
 let(w,h)=(r.width+l+rr,r.height+top+bottom);
 if w as i64>MAX_SIDE || h as i64>MAX_SIDE || w as u64*h as u64>MAX_PIXELS{return None;}
 Some((r.padded(w,h,l,top),placed_like(t,r.width,r.height,w,h,l as f64,top as f64)))
}

/// The transform for a `new_w` x `new_h` grid whose old grid sits at (`offset_x`, `offset_y`),
/// keeping every old pixel over the same document point.
pub(crate) fn placed_like(transform: &LayerTransform, old_w: u32, old_h: u32, new_w: u32, new_h: u32, offset_x: f64, offset_y: f64) -> LayerTransform {
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

/// One filter on a layer: a blur is given room to spread, run, blended back through the selection
/// on that grown grid, then cut back to what it left (`commitFilter`, Filters.swift:399-413).
pub fn apply_filter(doc: &mut Document, id: Uuid, params: &FilterParams) -> Result<(), CommandError> {
    apply_filter_with(doc, &SelectionClips::default(), id, params)
}

/// `apply_filter` with the selection's clip from `clips`.
pub fn apply_filter_with(doc: &mut Document, clips: &SelectionClips, id: Uuid, params: &FilterParams) -> Result<(), CommandError> {
    let params = params.normalized();
    if params.is_identity() { return Ok(()); }
    let original = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if original.pixels.is_none() && !original.is_group && !original.is_adjustment() {
        if let FilterParams::Vignette { settings } = &params {
            let (result, placed) = blank_vignette(doc, clips, settings, u32::MAX)?;
            let old = original.clone();
            let mask = old.mask.as_ref().map(|m| {
                if m.placement.is_none() && !m.is_uniform() && placed != old.transform {
                    Mask { pixels: carry_mask(m, &old.transform, &placed), ..m.clone() }
                } else { m.clone() }
            });
            let target = doc.layer_mut(id).unwrap();
            target.set_pixels(Some(result));
            target.transform = placed;
            if mask != old.mask { target.set_mask(mask); }
            return Ok(());
        }
    }
    let layer = pixel_layer(doc, id)?.clone();
    let raster = layer.pixels.as_ref().unwrap();
    let sides=filter_canvas_sides(doc,&layer.transform,raster.width,raster.height,&params);
    let (source, placed) = match params.spreads() {
        true => grown_filter(raster, &layer.transform, params.margin(),sides).ok_or(CommandError::Project(ProjectError::TooLarge))?,
        false => (raster.clone(), layer.transform),
    };
    let coverage = edit_coverage(doc, clips, &placed, source.width, source.height)?;
    // Kernel function, not this module's own `apply_filter`: adjust::filters::apply_filter.
    let filtered = adjust::filters::apply_filter_edges(&source, &params.mapped(&placed,source.width,source.height),sides)?;
    let filtered = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &source, &c), None => filtered };
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

/// Empty means no stored asset, not a transparent pixel asset. Only that case
/// paints the whole canvas; an existing transparent image remains transparent.
/// Preview allocates the reduced canvas directly, without a full-size clear copy.
pub(crate) fn blank_vignette(doc: &Document, clips: &SelectionClips,
    settings: &adjust::finishing::VignetteLayerSettings, limit: u32,
) -> Result<(Raster, LayerTransform), CommandError> {
    let scale = (limit as f64 / doc.width.max(doc.height).max(1) as f64).min(1.);
    let w = (doc.width as f64 * scale).round().max(1.) as u32;
    let h = (doc.height as f64 * scale).round().max(1.) as u32;
    if w as u64 * h as u64 > MAX_PIXELS { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let placed = LayerTransform::axis_aligned(Point { x: 0., y: 0. }, doc.size());
    let source = Raster::new_transparent(w, h);
    let coverage = edit_coverage(doc, clips, &placed, w, h)?;
    let output = adjust::finishing::vignette(&source, settings,
        Rect { x: 0., y: 0., width: w as f64, height: h as f64 }, true);
    let result = match coverage {
        Some(c) => adjust::apply::blend_by_coverage(&output, &source, &c),
        None => output,
    };
    Ok((result, placed))
}

/// A new adjustment layer above the active layer (inside it when a folder is active).
pub fn add_adjustment_layer(doc: &mut Document, kind: AdjustmentKind, seed: u32, gradient: Option<([f64; 3], [f64; 3])>) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut adjustment = LayerAdjustment::new(kind);
    match kind {
        AdjustmentKind::ProgressiveBlur | AdjustmentKind::RgbSplit => {
            let mut settings=adjust::filter_layer::FilterLayerSettings::default();
            settings.progressive_start=Some(Point{x:doc.width as f64/2.,y:doc.height as f64*0.35});
            settings.progressive_end=Some(Point{x:doc.width as f64/2.,y:doc.height as f64});
            settings.rgb_split_center=Some(Point{x:doc.width as f64/2.,y:doc.height as f64/2.});
            settings.rgb_split_reach=(doc.width as f64).hypot(doc.height as f64)/2.;
            adjustment.filter_layer_settings=Some(settings);
        }
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
