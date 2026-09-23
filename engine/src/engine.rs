use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Session {
    pub document: Document,
    pub history: History,
    pub path: Option<String>,
    pub preview: Option<PixelPreview>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerState {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub is_group: bool,
    #[serde(with = "ids::upper_opt")] pub parent_id: Option<Uuid>,
    pub opacity: f64,
    pub blend_mode: BlendMode,
    pub transform: LayerTransform,
    pub pixels_width: u32,
    pub pixels_height: u32,
    pub pixels_revision: u64,
    pub has_mask: bool,
    pub has_pixels: bool,
    pub mask_width: u32,
    pub mask_height: u32,
    pub mask_revision: u64,
    pub mask_enabled: bool,
    pub mask_linked: bool,
    #[serde(with = "ids::upper_opt")] pub mask_source_id: Option<Uuid>,
    pub mask_placement: Option<LayerTransform>,
    pub mask_background: u8,
    #[serde(skip_serializing_if = "Option::is_none")] pub adjustment: Option<LayerAdjustment>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentState {
    /// The engine session handle used to address this document; not the manifest's document id.
    #[serde(with = "ids::upper")] pub id: Uuid,
    /// The stable id from the project manifest, preserved across save/open round trips.
    #[serde(with = "ids::upper")] pub document_id: Uuid,
    pub width: u32,
    pub height: u32,
    pub resolution: f64,
    #[serde(with = "ids::upper_opt")] pub active_layer_id: Option<Uuid>,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Entries on the undo stack. A gesture that records one command can compare this against the
    /// depth it saw beforehand to tell whether its own entry is still the one on top.
    pub undo_depth: usize,
    pub is_modified: bool,
    pub path: Option<String>,
    pub layers: Vec<LayerState>,
}

/// Preview revisions start here so they can never collide with a layer's own, which counts up
/// from 1 as the document is edited.
const PREVIEW_REVISION_BASE: u64 = 1 << 40;

#[derive(Default)]
pub struct Engine { sessions: HashMap<Uuid, Session>, order: Vec<Uuid>, preview_revision: u64 }

fn check_dimensions(width: u32, height: u32) -> Result<(), CommandError> {
    if !(1..=MAX_SIDE as u32).contains(&width) || !(1..=MAX_SIDE as u32).contains(&height) {
        return Err(CommandError::Argument("width and height must be 1 to 30000".into()));
    }
    Ok(())
}

impl Engine {
    pub fn new() -> Engine { Engine::default() }
    pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }

    /// Sessions are keyed by a fresh handle, independent of `Document.id` (the stable manifest
    /// id), so the same document can be open more than once and ids reassigned by rare
    /// collisions with a live session never happen.
    fn insert(&mut self, document: Document, path: Option<String>) -> Uuid {
        let handle = Uuid::new_v4();
        self.sessions.insert(handle, Session { document, history: History::default(), path, preview: None });
        self.order.push(handle);
        handle
    }
    fn session(&self, id: Uuid) -> Result<&Session, CommandError> { self.sessions.get(&id).ok_or(CommandError::NoDocument) }
    fn session_mut(&mut self, id: Uuid) -> Result<&mut Session, CommandError> { self.sessions.get_mut(&id).ok_or(CommandError::NoDocument) }

    pub fn document_ids(&self) -> Vec<Uuid> { self.order.clone() }
    pub fn document(&self, id: Uuid) -> Option<&Document> { self.sessions.get(&id).map(|s| &s.document) }

    pub fn new_document(&mut self, width: u32, height: u32, empty_layer: bool) -> Result<Uuid, CommandError> {
        check_dimensions(width, height)?;
        let mut doc = Document::new(width, height);
        if empty_layer {
            let layer = Layer::blank("Layer 1", doc.size());
            doc.active_layer_id = Some(layer.id);
            doc.layers.push(layer);
        }
        Ok(self.insert(doc, None))
    }

    pub fn open_package(&mut self, pkg: &Package, path: Option<String>) -> Result<Uuid, CommandError> {
        // `Document.id` is kept exactly as read from the manifest (stable across save/open,
        // to match the macOS format); the session handle from `insert` is what keeps two
        // open sessions from colliding, so no collision guard is needed here.
        let doc = package::open_package(pkg)?;
        Ok(self.insert(doc, path))
    }

    pub fn save_package(&self, id: Uuid) -> Result<Package, CommandError> {
        Ok(package::save_package(&self.session(id)?.document)?)
    }
    pub fn mark_saved(&mut self, id: Uuid, path: Option<String>) {
        if let Some(s) = self.sessions.get_mut(&id) { s.history.mark_saved(); if path.is_some() { s.path = path; } }
    }
    pub fn close_document(&mut self, id: Uuid) {
        self.sessions.remove(&id);
        self.order.retain(|d| *d != id);
    }

    pub fn state(&self, id: Uuid) -> Result<DocumentState, CommandError> {
        let s = self.session(id)?;
        let doc = self.render_document(id)?;
        let d = &*doc;
        Ok(DocumentState {
            id, document_id: d.id, width: d.width, height: d.height, resolution: d.resolution, active_layer_id: d.active_layer_id,
            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), undo_depth: s.history.depth(), is_modified: s.history.is_modified(),
            path: s.path.clone(),
            layers: d.layers.iter().map(|l| LayerState {
                id: l.id, name: l.name.clone(), visible: l.visible, is_group: l.is_group, parent_id: l.parent_id,
                opacity: l.opacity, blend_mode: l.blend_mode, transform: l.transform,
                pixels_width: l.pixels.as_ref().map_or(0, |p| p.width), pixels_height: l.pixels.as_ref().map_or(0, |p| p.height),
                pixels_revision: l.pixels_revision, has_mask: l.mask.is_some(), has_pixels: l.has_pixels(),
                mask_width: l.mask.as_ref().map_or(0, |m| m.pixels.width), mask_height: l.mask.as_ref().map_or(0, |m| m.pixels.height),
                mask_revision: l.mask_revision, mask_enabled: l.mask.as_ref().map_or(true, |m| m.enabled),
                mask_linked: l.mask.as_ref().map_or(true, |m| m.is_linked()), mask_source_id: l.mask_source_id,
                mask_placement: l.mask.as_ref().and_then(|m| m.placement), mask_background: l.mask.as_ref().map_or(255, |m| m.background()),
                adjustment: l.extra.adjustment.clone(),
            }).collect(),
        })
    }

    /// The document as the canvas should show it: the stored one, or a copy with the open
    /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
    /// the ops do not, because a preview is not committed.
    fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
        let s = self.session(id)?;
        let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
        let mut doc = s.document.clone();
        if let Some(layer) = doc.layer_mut(preview.layer) {
            layer.pixels = Some(preview.raster.clone());
            layer.pixels_revision = preview.revision;
            layer.transform = preview.transform;
        }
        Ok(std::borrow::Cow::Owned(doc))
    }
    pub fn set_preview(&mut self, id: Uuid, request: Option<PreviewRequest>) -> Result<Dirty, CommandError> {
        let revision = { self.preview_revision += 1; PREVIEW_REVISION_BASE + self.preview_revision };
        let s = self.session_mut(id)?;
        let layers: Vec<Uuid> = s.preview.iter().map(|p| p.layer).chain(request.iter().map(|r| r.layer())).collect();
        s.preview = request.as_ref().and_then(|r| preview::compute_preview(&s.document, r, revision));
        Ok(Dirty { structure: true, canvas: false, layers })
    }
    fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }

    /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
    fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
    where F: FnOnce(&mut Document) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        let mut next = s.document.clone();
        let dirty = f(&mut next)?;
        // The active layer is selection, not content: a command that only moves it (SetActiveLayer)
        // still applies, but records no history entry and so leaves redo intact, as macOS does.
        let content_changed = !next.same_content(&s.document);
        let before = std::mem::replace(&mut s.document, next);
        if content_changed { s.history.push(before); }
        Ok(dirty)
    }

    pub fn execute(&mut self, handle: Uuid, command: Command) -> Result<Dirty, CommandError> {
        self.clear_preview(handle);
        self.edit(handle, |doc| match command {
            Command::AddBlankLayer => { ops::layers::add_blank_layer(doc)?; Ok(Dirty::structure()) }
            Command::RenameLayer { id, name } => { ops::layers::rename_layer(doc, id, &name)?; Ok(Dirty::structure()) }
            Command::SetLayerVisible { id, visible } => { ops::layers::set_layer_visible(doc, id, visible)?; Ok(Dirty::structure()) }
            Command::DeleteLayer { id } => { ops::layers::delete_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::SetActiveLayer { id } => { ops::layers::set_active_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::CanvasSize { width, height, anchor, fill } => {
                *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions { width, height, anchor, fill, content_offset: None })?;
                Ok(Dirty::everything())
            }
            Command::Crop { x, y, width, height } => {
                if ![x, y, width, height].iter().all(|v| v.is_finite()) || width < 1.0 || height < 1.0 || width > MAX_SIDE as f64 || height > MAX_SIDE as f64 {
                    return Err(CommandError::Argument("crop rectangle out of range".into()));
                }
                // Round both edges of the rect and derive width/height from their difference,
                // instead of rounding x and width independently, so the right and bottom edges
                // land exactly where they were dragged instead of drifting by a pixel.
                let x0 = x.round(); let x1 = (x + width).round();
                let y0 = y.round(); let y1 = (y + height).round();
                *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions {
                    width: (x1 - x0) as u32, height: (y1 - y0) as u32, anchor: 4, fill: None,
                    content_offset: Some(Point { x: -x0, y: -y0 }) })?;
                Ok(Dirty::everything())
            }
            Command::FlipCanvas { horizontal } => { ops::flip::flip_canvas(doc, horizontal); Ok(Dirty::structure()) }
            Command::ImageSize { width, height, resolution, sampling } => {
                *doc = ops::image_size::image_size(doc, ops::image_size::ImageSizeOptions { width, height, resolution, sampling })?;
                Ok(Dirty::everything())
            }
            Command::SetLayerOpacity { id, opacity } => { ops::appearance::set_opacity(doc, id, opacity)?; Ok(Dirty::structure()) }
            Command::SetLayersOpacity { ids, opacity } => { ops::appearance::set_opacity_many(doc, &ids, opacity)?; Ok(Dirty::structure()) }
            Command::SetLayerBlendMode { id, mode } => { ops::appearance::set_blend_mode(doc, id, mode)?; Ok(Dirty::structure()) }
            Command::AddGroup => { ops::hierarchy::add_group(doc)?; Ok(Dirty::structure()) }
            Command::GroupLayers { ids } => { ops::hierarchy::group_layers(doc, &ids)?; Ok(Dirty::structure()) }
            Command::PlaceLayer { id, parent, above, at_bottom } => { ops::hierarchy::place_layer(doc, id, parent, above, at_bottom)?; Ok(Dirty::structure()) }
            Command::MoveLayerBy { id, offset } => { ops::hierarchy::move_layer_by(doc, id, offset)?; Ok(Dirty::structure()) }
            Command::DuplicateLayer { id } => { ops::hierarchy::duplicate_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::DuplicateLayerTo { id, parent, above, at_bottom } => { ops::hierarchy::duplicate_layer_to(doc, id, parent, above, at_bottom)?; Ok(Dirty::structure()) }
            Command::DuplicateLayerTransformed { id, transform } => { let copy = ops::hierarchy::duplicate_layer(doc, id)?; ops::transform::set_transform(doc, copy, transform)?; Ok(Dirty::structure()) }
            Command::DeleteLayers { ids, bake } => {
                let baked: Vec<Uuid> = if bake {
                    ops::hierarchy::clip_dependents(doc, &ids).into_iter().filter(|d| doc.layer(*d).map_or(false, |l| l.has_pixels())).collect()
                } else { Vec::new() };
                ops::hierarchy::delete_layers(doc, &ids, bake)?;
                Ok(Dirty { structure: true, canvas: false, layers: baked })
            }
            Command::SetLayerTransform { id, transform } => { ops::transform::set_transform(doc, id, transform)?; Ok(Dirty::structure()) }
            Command::TransformLayers { ids, bounds, draft } => { ops::transform::transform_group(doc, &ids, &bounds, &draft)?; Ok(Dirty::structure()) }
            Command::FlipLayers { ids, horizontal } => { ops::transform::flip_layers(doc, &ids, horizontal)?; Ok(Dirty::structure()) }
            Command::NudgeLayers { ids, dx, dy } => { ops::transform::nudge(doc, &ids, dx, dy)?; Ok(Dirty::structure()) }
            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::DistortLayers { ids, bounds, draft, corners } => {
                let touched = ops::transform::members(doc, &ids);
                ops::distort::distort_group(doc, &ids, &bounds, &draft, &corners)?;
                Ok(Dirty { structure: true, canvas: false, layers: touched })
            }
            Command::SetMaskPlacement { id, placement } => { ops::transform::set_mask_placement(doc, id, placement)?; Ok(Dirty::structure()) }
            Command::AddMask { id, revealing } => { ops::masks::add_mask(doc, id, revealing)?; Ok(Dirty::structure()) }
            Command::DeleteMask { id } => { ops::masks::delete_mask(doc, id)?; Ok(Dirty::structure()) }
            Command::SetMaskEnabled { id, enabled } => { ops::masks::set_mask_enabled(doc, id, enabled)?; Ok(Dirty::structure()) }
            Command::SetMaskLinked { id, linked } => { ops::masks::set_mask_linked(doc, id, linked)?; Ok(Dirty::structure()) }
            Command::InvertMask { id } => { ops::masks::invert_mask(doc, id)?; Ok(Dirty::structure()) }
            Command::FillMask { id, white } => { ops::masks::fill_mask(doc, id, white)?; Ok(Dirty::structure()) }
            Command::BlurMask { id, radius } => { ops::masks::blur_mask(doc, id, radius)?; Ok(Dirty::structure()) }
            Command::CopyMask { from, to } => { ops::masks::copy_mask(doc, from, to)?; Ok(Dirty::structure()) }
            Command::ToggleClipping { id } => { ops::hierarchy::toggle_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::ReleaseClipping { id } => { ops::hierarchy::release_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::LinkMask { source, target } => { ops::hierarchy::link_mask(doc, source, target)?; Ok(Dirty::structure()) }
            Command::MergeLayers { ids } => { let m = ops::merge::merge(doc, &ids)?; Ok(Dirty { structure: true, canvas: false, layers: vec![m] }) }
            Command::ApplyAdjustment { id, adjustment } => { ops::adjust::apply_adjustment_to_layer(doc, id, &adjustment)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::InvertPixels { id, mask } => { ops::adjust::invert_layer(doc, id, mask)?; Ok(Dirty { structure: true, canvas: false, layers: if mask { vec![] } else { vec![id] } }) }
            Command::ApplyFilter { id, params } => { ops::adjust::apply_filter(doc, id, &params)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::AddAdjustmentLayer { kind, seed, shadows, highlights } => {
                let gradient = match (shadows, highlights) { (Some(s), Some(h)) => Some((s, h)), _ => None };
                ops::adjust::add_adjustment_layer(doc, kind, seed, gradient)?; Ok(Dirty::structure())
            }
            Command::SetAdjustment { id, adjustment } => { ops::adjust::set_adjustment(doc, id, &adjustment)?; Ok(Dirty::structure()) }
        })
    }

    /// A preview is a live, uncommitted gesture: when one is showing, undo and redo spend
    /// themselves cancelling it rather than reaching past it into real history, or the keystroke
    /// that dismisses an open panel would also silently rewind an unrelated earlier edit.
    fn drop_preview(&mut self, id: Uuid) -> bool {
        let had = self.session(id).map_or(false, |s| s.preview.is_some());
        self.clear_preview(id);
        had
    }
    pub fn undo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        if self.drop_preview(id) { return Ok(Dirty::everything()); }
        let s = self.session_mut(id)?;
        if let Some(before) = s.history.undo(&s.document) { s.document = before; }
        Ok(Dirty::everything())
    }
    pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        if self.drop_preview(id) { return Ok(Dirty::everything()); }
        let s = self.session_mut(id)?;
        if let Some(after) = s.history.redo(&s.document) { s.document = after; }
        Ok(Dirty::everything())
    }
    /// Drops the last history entry and returns to the state before it, leaving no redo: for a
    /// gesture the user cancelled, such as Escape during an Alt-drag duplicate. Unlike undo/redo,
    /// an explicit revert always runs even while a preview is showing: it is an unambiguous
    /// request to discard changes, and a no-op here would be a silent failure of that request.
    pub fn revert(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        self.clear_preview(id);
        let s = self.session_mut(id)?;
        if let Some(before) = s.history.revert() { s.document = before; }
        Ok(Dirty::everything())
    }

    pub fn import_image(&mut self, id: Option<Uuid>, bytes: &[u8], name: &str, at: Option<Point>) -> Result<Uuid, CommandError> {
        let raster = decode_image(bytes)?.raster;
        match id {
            Some(id) => {
                self.edit(id, |doc| { ops::layers::import_raster(doc, raster, name, at)?; Ok(Dirty::structure()) })?;
                Ok(id)
            }
            None => {
                let mut doc = Document::new(raster.width, raster.height);
                ops::layers::import_raster(&mut doc, raster, name, None)?;
                let id = self.insert(doc, None);
                // A fresh import has content that is not on disk: mark as modified.
                let s = self.session_mut(id)?;
                s.history.mark_never_saved();
                Ok(id)
            }
        }
    }

    pub fn export_png(&self, id: Uuid) -> Result<Vec<u8>, CommandError> { Ok(compositor::export_png(&self.session(id)?.document)?) }
    pub fn export_jpeg(&self, id: Uuid, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg(&self.session(id)?.document, quality, matte)?)
    }
    pub fn export_jpeg_preview(&self, id: Uuid, quality: f64, matte: [f64; 3], max_side: u32) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg_preview(&self.session(id)?.document, quality, matte, max_side)?)
    }
    pub fn composite(&self, id: Uuid, region: Rect, width: u32, height: u32) -> Result<Raster, CommandError> {
        Ok(compositor::composite(&*self.render_document(id)?, region, width, height))
    }

    pub fn render_plan(&self, id: Uuid, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError> { Ok(plan::render_plan(&*self.render_document(id)?, edit)) }
    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit(&*self.render_document(id)?, edit, region, w, h)) }
    pub fn clip_dependents(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, CommandError> { Ok(ops::hierarchy::clip_dependents(&self.session(id)?.document, ids)) }
    pub fn merge_action(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<&'static str>, CommandError> { Ok(ops::merge::merge_plan(&self.session(id)?.document, ids).map(|p| p.action)) }
    pub fn group_box(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<LayerTransform>, CommandError> { Ok(ops::transform::group_box(&self.session(id)?.document, ids)) }
    pub fn can_toggle_clipping(&self, id: Uuid, layer: Uuid) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_toggle_clipping(&self.session(id)?.document, layer)) }
    pub fn can_place(&self, id: Uuid, layer: Uuid, parent: Option<Uuid>) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_place(&self.session(id)?.document, layer, parent)) }

    /// The layer's raster after `level` sharp halvings, through any open preview.
    pub fn layer_raster(&self, id: Uuid, layer: Uuid, level: u32) -> Result<Option<Raster>, CommandError> {
        let doc = self.render_document(id)?;
        let Some(mut raster) = doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone() else { return Ok(None); };
        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
            if raster.width <= 1 || raster.height <= 1 { break; }
            raster = raster.halved();
        }
        Ok(Some(raster))
    }
    /// Everything that renders beneath an adjustment layer, composited at canvas size: what its
    /// histogram and eyedroppers read, as macOS renders the layers underneath.
    pub fn adjustment_source(&self, id: Uuid, layer: Uuid) -> Result<Raster, CommandError> {
        let doc = &self.session(id)?.document;
        let order = ops::hierarchy::hierarchy_order(doc);
        let position = order.iter().position(|o| *o == layer).ok_or(CommandError::NoLayer)?;
        let beneath: std::collections::HashSet<Uuid> = order[..position].iter().copied().collect();
        let mut below = doc.clone();
        for l in &mut below.layers { if !l.is_group && !beneath.contains(&l.id) { l.visible = false; } }
        Ok(compositor::composite(&below, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height))
    }
    /// A panel's histogram: an adjustment layer reads what lies beneath it, any other layer its
    /// own stored pixels (never the preview, or the graph would chase itself).
    pub fn histogram(&self, id: Uuid, layer: Uuid) -> Result<Vec<Vec<f64>>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        if target.is_adjustment() { return Ok(adjust::levels::histogram(&self.adjustment_source(id, layer)?, None)); }
        let raster = target.pixels.as_ref().ok_or(CommandError::Argument("the layer has no pixels".into()))?;
        Ok(adjust::levels::histogram(raster, None))
    }
    pub fn auto_levels(&self, id: Uuid, layer: Uuid, mode: LevelsAuto) -> Result<LevelsSettings, CommandError> {
        Ok(mode.settings(&self.histogram(id, layer)?))
    }
    /// The straight colour of one layer at a document point; None where it is transparent.
    pub fn sample_layer_color(&self, id: Uuid, layer: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let (raster, source) = match (target.pixels.as_ref(), target.is_adjustment()) {
            (Some(r), _) => (r.clone(), target.transform),
            (None, true) => (self.adjustment_source(id, layer)?, LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, doc.size())),
            _ => return Err(CommandError::Argument("the layer has no pixels".into())),
        };
        let Some(inverse) = source.pixel_to_document(raster.width, raster.height).invert() else { return Ok(None); };
        let p = inverse.apply(at);
        if p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64 { return Ok(None); }
        let pixel = raster.pixel(p.x as u32, p.y as u32);
        if pixel[3] == 0 { return Ok(None); }
        Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))))
    }
    /// The straight colour of the visible composite at a document point (the Hue/Saturation
    /// eyedroppers), read from the STORED document -- never an open preview, for the same reason
    /// `histogram` and `sample_layer_color` do not: a Hue/Saturation eyedropper reading its own
    /// panel's live edit would chase whatever the sliders just did instead of the colour that was
    /// actually there, and could never converge. Mirrors `sampleCompositeColor` in
    /// `ColorPalette.swift`, which draws from the session's `document` property (the committed
    /// model), not the separate live-preview image the canvas shows while an edit is open.
    ///
    /// A caller that deliberately wants "what is on screen right now, preview included" (a test
    /// proving a preview toggle changes the displayed pixels, say) should read the rendered
    /// canvas directly rather than ask a sampling API to special-case it: this crate had exactly
    /// that special case once, under this same name, and it was never used by anything but such a
    /// test -- a live production caller (the Hue/Saturation eyedropper) reused it on the strength
    /// of the name alone and sampled its own preview by mistake (a Critical, Task 15 code review).
    pub fn sample_color(&self, id: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = &self.session(id)?.document;
        let region = Rect { x: at.x.floor(), y: at.y.floor(), width: 1.0, height: 1.0 };
        let pixel = compositor::composite(doc, region, 1, 1).pixel(0, 0);
        if pixel[3] == 0 { return Ok(None); }
        Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))))
    }
    pub fn levels_sampling(&self, id: Uuid, layer: Uuid, settings: &LevelsSettings, at: Point, mode: LevelsSample) -> Result<LevelsSettings, CommandError> {
        match self.sample_layer_color(id, layer, at)? { Some(rgb) => Ok(settings.sampling(rgb, mode)), None => Ok(settings.clone()) }
    }
}
