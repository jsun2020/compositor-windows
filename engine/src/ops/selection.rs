//! The selection commands (Phase 4a): Compositor for Mac's `applySelection`, the Marquee and
//! Lasso's `finishLasso`, Select All / Deselect / Inverse, the outline move, Expand / Contract /
//! Feather (Document/Selection.swift:141-359), the Magic Wand (Document/MagicWand.swift:98-137) and
//! loading a layer's or a mask's pixels as a selection (Document/MaskTracing.swift:73-94).
use crate::selection::geometry::{self as g, Boolean};
use crate::*;
use uuid::Uuid;

/// Said when an edit needs a selection with something in it.
pub const EMPTY_SELECTION: &str = "The selection is empty";
/// Said when the Expand or Contract band cannot be made (an outline past the geometry's range).
pub const EXPAND_FAILED: &str = "The selection could not be expanded";
pub const CONTRACT_FAILED: &str = "The selection could not be contracted";
/// Said when an edit needs a selection and there is none.
pub const NO_SELECTION: &str = "Nothing is selected";
/// The Magic Wand's refusal of an outline past `WAND_EDGE_LIMIT` (MagicWand.swift:28).
pub const TOO_DETAILED: &str = "That selection is too detailed to outline. Try a different Tolerance, or turn on Contiguous.";
/// Loading a layer's or a mask's pixels past the same limit (the Mac beeps): no Wand advice, which
/// would not apply to a thumbnail's Ctrl-click (final review M4).
pub const LAYER_TOO_DETAILED: &str = "That layer is too detailed to load as a selection.";
pub const MASK_TOO_DETAILED: &str = "That mask is too detailed to load as a selection.";

fn refused(message: &str) -> CommandError { CommandError::Refused(message.to_string()) }
fn canvas(doc: &Document) -> Vec<Contour> { vec![g::rectangle(Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 })] }
fn check_point(p: Point) -> Result<(), CommandError> {
    if p.x.is_finite() && p.y.is_finite() && p.x.abs() <= SELECTION_COORDINATE_LIMIT && p.y.abs() <= SELECTION_COORDINATE_LIMIT { Ok(()) }
    else { Err(CommandError::Argument("outline points must lie within 1,000,000 pixels of the canvas".into())) }
}
/// The current selection when it has something in it; the refusal otherwise.
fn modifiable(doc: &Document) -> Result<&Selection, CommandError> {
    let s = doc.selection.as_ref().ok_or_else(|| refused(NO_SELECTION))?;
    if s.is_empty() { return Err(refused(EMPTY_SELECTION)); }
    Ok(s)
}

/// `applySelection`: `shape`, cut to the canvas, becomes the selection (Replace), joins it (Add) or
/// is taken out of it (Subtract; with no selection that changes nothing). The result takes the
/// given anti-alias flag and no feather, as the Mac's `DocumentSelection(path:antialiased:)` does.
pub fn apply_selection(doc: &mut Document, shape: &[Contour], mode: SelectionMode, antialiased: bool) {
    let clipped = g::combine(shape, &canvas(doc), Boolean::Intersection);
    let result = match (mode, &doc.selection) {
        (SelectionMode::Replace, _) | (SelectionMode::Add, None) => clipped,
        (SelectionMode::Add, Some(current)) => g::combine(&current.contours, &clipped, Boolean::Union),
        (SelectionMode::Subtract, None) => return,
        (SelectionMode::Subtract, Some(current)) => g::combine(&current.contours, &clipped, Boolean::Difference),
    };
    doc.selection = Some(Selection::new(result, antialiased, 0.0));
}

/// `finishLasso`: the drawn outline closed and combined by `mode`. A Marquee sends its box's four
/// corners; an Ellipse fills that box (`addEllipse(in:)`). A click or a line, enclosing nothing,
/// deselects in Replace and changes nothing otherwise.
pub fn select_shape(doc: &mut Document, kind: SelectionShape, points: &[Point], mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
    for p in points { check_point(*p)?; }
    let (xs, ys) = (points.iter().map(|p| p.x), points.iter().map(|p| p.y));
    let (x0, x1) = (xs.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max));
    let (y0, y1) = (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max));
    let is_ellipse = kind == SelectionShape::Ellipse && points.len() == 4;
    if !(points.len() >= 3 || kind == SelectionShape::Ellipse) || !(x1 - x0 > 0.0) || !(y1 - y0 > 0.0) {
        if mode == SelectionMode::Replace { doc.selection = None; }
        return Ok(());
    }
    let outline = if is_ellipse { g::ellipse(Rect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }) } else { g::polygon(points) };
    apply_selection(doc, &[outline], mode, antialiased);
    Ok(())
}

/// Select All: the canvas, antialiased, no feather.
pub fn select_all(doc: &mut Document) { doc.selection = Some(Selection::new(canvas(doc), true, 0.0)); }

/// Deselect: no selection.
pub fn deselect(doc: &mut Document) { doc.selection = None; }

/// Inverse: the canvas minus the selection, its flags kept; nothing without one. The inverse of
/// everything is no selection at all, as in Photoshop, not an empty one that stops every edit
/// (`invertSelection`, Selection.swift:354-362 at v1.4.5).
pub fn invert_selection(doc: &mut Document) {
    let Some(current) = doc.selection.clone() else { return };
    let inverse = Selection::new(g::combine(&canvas(doc), &current.contours, Boolean::Difference), current.antialiased, current.feather);
    doc.selection = if inverse.is_empty() { None } else { Some(inverse) };
}

/// The outline moved by whole pixels (`moveSelection(by:)`: offsets rounded), not cut to the canvas,
/// so it can leave and come back whole.
pub fn move_selection(doc: &mut Document, dx: f64, dy: f64) -> Result<(), CommandError> {
    if !dx.is_finite() || !dy.is_finite() { return Err(CommandError::Argument("the offset must be finite".into())); }
    let current = modifiable(doc)?;
    let (dx, dy) = (dx.round(), dy.round());
    let b = current.bounds().unwrap_or(Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 });
    for p in [Point { x: b.x + dx, y: b.y + dy }, Point { x: b.max_x() + dx, y: b.max_y() + dy }] { check_point(p)?; }
    doc.selection = Some(current.translated((dx * SUBPIXEL) as i32, (dy * SUBPIXEL) as i32));
    Ok(())
}

/// Expand (`delta` > 0) or Contract: the round-capped, round-joined band `|delta|` wide either
/// side of the outline, added and cut to the canvas, or taken away (so Contract also moves in from
/// the canvas edges, and can leave an empty selection). `resizeSelection`, Selection.swift:333-342.
pub fn resize_selection(doc: &mut Document, delta: i64) -> Result<(), CommandError> {
    if delta == 0 || delta.unsigned_abs() > MAX_RESIZE as u64 { return Err(CommandError::Argument("Expand and Contract take 1 to 500 pixels".into())); }
    let current = modifiable(doc)?.clone();
    let band = g::band(&current.contours, delta.unsigned_abs() as f64).ok_or_else(|| refused(if delta > 0 { EXPAND_FAILED } else { CONTRACT_FAILED }))?;
    let result = if delta > 0 {
        g::combine(&g::combine(&current.contours, &band, Boolean::Union), &canvas(doc), Boolean::Intersection)
    } else {
        g::combine(&current.contours, &band, Boolean::Difference)
    };
    doc.selection = Some(Selection::new(result, current.antialiased, current.feather));
    Ok(())
}

/// Feather: two soft edges together spread as blurs do, sqrt(a^2 + b^2), at most 250.
pub fn feather_selection(doc: &mut Document, amount: u32) -> Result<(), CommandError> {
    if !(1..=MAX_FEATHER as u32).contains(&amount) { return Err(CommandError::Argument("Feather takes 1 to 250 pixels".into())); }
    let current = modifiable(doc)?;
    let softened = (current.feather * current.feather + (amount as f64) * (amount as f64)).sqrt().min(MAX_FEATHER);
    doc.selection = Some(Selection { feather: softened, ..current.clone() });
    Ok(())
}

/// The mirrored selection for Flip Canvas (LayerFlip.swift:69-76).
pub fn flip_selection(doc: &mut Document, horizontal: bool) {
    let (w, h) = ((doc.width as f64 * SUBPIXEL) as i32, (doc.height as f64 * SUBPIXEL) as i32);
    if let Some(s) = &mut doc.selection {
        for c in std::sync::Arc::make_mut(&mut s.contours).iter_mut() { for p in c.iter_mut() { if horizontal { p[0] = w - p[0]; } else { p[1] = h - p[1]; } } }
    }
}

/// What the Magic Wand reads, at canvas size (`selectionSample`): the visible composite, or the
/// active layer's own pixels through its transform, without its mask or opacity. A folder, an
/// adjustment layer or a blank layer reads as transparent.
pub fn wand_sample(doc: &Document, all_layers: bool) -> Raster {
    let canvas = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    if all_layers { return composite(doc, canvas, doc.width, doc.height); }
    let mut target = vec![0u8; (doc.width as usize) * (doc.height as usize) * 4];
    if let Some(layer) = doc.active_layer_id.and_then(|id| doc.layer(id)).filter(|l| !l.is_group && l.pixels.is_some()) {
        let mut alone = layer.clone();
        alone.opacity = 1.0;
        alone.mask = None;
        compositor::render_layer(&mut target, doc.width, doc.height, canvas, &alone);
    }
    Raster::from_premultiplied(doc.width, doc.height, target)
}

/// The Magic Wand at a document point: its outline, combined by `mode`. In Replace the traced
/// outline is taken as it is (it already lies on the canvas); nothing matched deselects in
/// Replace. `magicWand(at:mode:)`, MagicWand.swift:98-123.
pub fn magic_wand_select(doc: &mut Document, at: Point, mode: SelectionMode, settings: &WandSettings, antialiased: bool) -> Result<(), CommandError> {
    if !(at.x >= 0.0 && at.y >= 0.0 && at.x < doc.width as f64 && at.y < doc.height as f64) {
        return Err(CommandError::Argument("the Magic Wand's point must lie on the canvas".into()));
    }
    if settings.tolerance > 255 || settings.sample_radius > 2 { return Err(CommandError::Argument("tolerance is 0 to 255, sample radius 0 to 2".into())); }
    let sample = wand_sample(doc, settings.all_layers);
    match magic_wand(&sample, at, settings) {
        Err(TraceError::TooDetailed) => Err(refused(TOO_DETAILED)),
        Ok(None) => { if mode == SelectionMode::Replace { doc.selection = None; } Ok(()) }
        Ok(Some(outline)) if mode == SelectionMode::Replace => { doc.selection = Some(Selection::new(outline, antialiased, 0.0)); Ok(()) }
        Ok(Some(outline)) => { apply_selection(doc, &outline, mode, antialiased); Ok(()) }
    }
}

/// Ctrl-click on a layer's thumbnail (`loadLayerSelection`): its pixels at least 50% opaque,
/// whatever its mask, through its transform, combined by `mode`.
pub fn load_layer_selection(doc: &mut Document, id: Uuid, mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let raster = layer.pixels.as_ref().filter(|_| !layer.is_group).ok_or_else(|| refused("The layer has no pixels to select"))?;
    let traced = opaque_pixels(raster).map_err(|_| refused(LAYER_TOO_DETAILED))?;
    if traced.is_empty() { return Err(refused("The layer has no pixels at least half opaque")); }
    let outline = g::transformed(&traced, &layer.transform.pixel_to_document(raster.width, raster.height));
    apply_selection(doc, &outline, mode, antialiased);
    Ok(())
}

/// Delete with a selection (`clearSelectedPixels`, SelectionEdits.swift:51-56): the layer's pixels
/// fade to transparent by the selection's coverage (`clearPixels`, BrushStroke.swift:631-637); with
/// the mask targeted, the mask fills white there instead, the mask palette's background ("Fill
/// Mask"). A 1x1 mask takes the layer's pixel grid first. Needs a selection with something in it,
/// and an enabled mask (`canPaint`, EditorSession+Brush.swift:5-11).
pub fn clear_selected(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool) -> Result<(), CommandError> {
    modifiable(doc)?;
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if mask {
        let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
        if !m.enabled { return Err(CommandError::Argument("the mask is turned off".into())); }
        crate::ops::masks::expand_uniform(doc, id)?;
        let layer = doc.layer(id).unwrap();
        let m = layer.mask.as_ref().unwrap();
        let grid = m.placement.unwrap_or(layer.transform);
        let coverage = crate::ops::adjust::edit_coverage(doc, clips, &grid, m.pixels.width, m.pixels.height)?.unwrap();
        let white = GrayRaster::from_bytes(m.pixels.width, m.pixels.height, vec![255; (m.pixels.width * m.pixels.height) as usize]);
        let filled = adjust::apply::blend_gray_by_coverage(&white, &m.pixels, &coverage);
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = filled;
        return Ok(());
    }
    if layer.is_group { return Err(CommandError::Argument("folders have no pixels".into())); }
    let raster = layer.pixels.as_ref().ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
    let coverage = crate::ops::adjust::edit_coverage(doc, clips, &layer.transform, raster.width, raster.height)?.unwrap();
    let mut data = raster.bytes().to_vec();
    for (p, &k) in data.chunks_exact_mut(4).zip(coverage.bytes()) {
        if k == 0 { continue; }
        let keep = 255 - k as u32;
        for v in p.iter_mut() { *v = ((*v as u32 * keep + 127) / 255) as u8; }
    }
    let cleared = Raster::from_premultiplied(raster.width, raster.height, data);
    doc.layer_mut(id).unwrap().set_pixels(Some(cleared));
    Ok(())
}

/// Add Mask with a selection (`addMask(revealing:)`, LayerMask.swift:237-267 at v1.4.5): a mask on
/// the layer's pixel grid (its rectangle when it has none). Reveal Selection fills it black and
/// paints white through the selection's clip -- its coverage with the feather, cut to the canvas
/// (`selection.clip(canvas:)`, :253-257) -- so only the selection shows; Hide Selection (Alt-click)
/// is the reverse. Compositor 1.2.10 had the tones the other way round. The selection is used up in
/// the same step. An empty selection clips everything away: a plain black mask when revealing, a
/// plain white one when hiding.
pub fn add_mask_from_selection(doc: &mut Document, clips: &SelectionClips, id: Uuid, revealing: bool) -> Result<(), CommandError> {
    if doc.selection.is_none() { return Err(refused(NO_SELECTION)); }
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if layer.mask.is_some() { return Err(CommandError::Argument("the layer already has a mask".into())); }
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round() as i64, layer.transform.size.height.round() as i64), |p| (p.width as i64, p.height as i64));
    if w < 1 || h < 1 || w > MAX_SIDE || h > MAX_SIDE || (w * h) as u64 > MAX_PIXELS - doc.used_mask_pixels().min(MAX_PIXELS) {
        return Err(CommandError::Project(ProjectError::TooLarge));
    }
    let (w, h) = (w as u32, h as u32);
    // The clip itself, empty or not (`clip(canvas:)`): an empty selection clips everything away.
    let Some(coverage) = selection_coverage_with(doc, clips, &layer.transform.pixel_to_document(w, h), w, h) else { return Err(refused(NO_SELECTION)) };
    let pixels = if revealing { coverage } else { GrayRaster::from_bytes(w, h, coverage.bytes().iter().map(|c| 255 - c).collect()) };
    doc.layer_mut(id).unwrap().set_mask(Some(Mask { pixels, enabled: true, placement: None, linked: None }));
    doc.selection = None;
    Ok(())
}

/// Ctrl-click on a mask's thumbnail (`loadMaskSelection`): the mask's BLACK areas, darker than 50%
/// grey, which is what it hides (Photoshop loads the white; the Mac, and so this port, the
/// black), through where the mask sits, combined by `mode`.
pub fn load_mask_selection(doc: &mut Document, id: Uuid, mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let mask = layer.mask.as_ref().ok_or_else(|| refused("The layer has no mask"))?;
    let traced = dark_pixels(&mask.pixels).map_err(|_| refused(MASK_TOO_DETAILED))?;
    if traced.is_empty() { return Err(refused("The mask has no black areas to select")); }
    let placement = mask.placement.unwrap_or(layer.transform);
    let outline = g::transformed(&traced, &placement.pixel_to_document(mask.pixels.width, mask.pixels.height));
    apply_selection(doc, &outline, mode, antialiased);
    Ok(())
}
