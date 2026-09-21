use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Session {
    pub document: Document,
    pub history: History,
    pub path: Option<String>,
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
    pub is_modified: bool,
    pub path: Option<String>,
    pub layers: Vec<LayerState>,
}

#[derive(Default)]
pub struct Engine { sessions: HashMap<Uuid, Session>, order: Vec<Uuid> }

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
        self.sessions.insert(handle, Session { document, history: History::default(), path });
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
        let d = &s.document;
        Ok(DocumentState {
            id, document_id: d.id, width: d.width, height: d.height, resolution: d.resolution, active_layer_id: d.active_layer_id,
            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), is_modified: s.history.is_modified(),
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
            }).collect(),
        })
    }

    /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
    fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
    where F: FnOnce(&mut Document) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        let mut next = s.document.clone();
        let dirty = f(&mut next)?;
        if next != s.document {
            let before = std::mem::replace(&mut s.document, next);
            s.history.push(before);
        }
        Ok(dirty)
    }

    pub fn execute(&mut self, handle: Uuid, command: Command) -> Result<Dirty, CommandError> {
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
            Command::DeleteLayers { ids, bake } => { ops::hierarchy::delete_layers(doc, &ids, bake)?; Ok(Dirty::structure()) }
            Command::SetLayerTransform { id, transform } => { ops::transform::set_transform(doc, id, transform)?; Ok(Dirty::structure()) }
            Command::TransformLayers { ids, bounds, draft } => { ops::transform::transform_group(doc, &ids, &bounds, &draft)?; Ok(Dirty::structure()) }
            Command::FlipLayers { ids, horizontal } => { ops::transform::flip_layers(doc, &ids, horizontal)?; Ok(Dirty::structure()) }
            Command::NudgeLayers { ids, dx, dy } => { ops::transform::nudge(doc, &ids, dx, dy)?; Ok(Dirty::structure()) }
            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::DistortLayers { ids, bounds, draft, corners } => { ops::distort::distort_group(doc, &ids, &bounds, &draft, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: ids }) }
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
        })
    }

    pub fn undo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        if let Some(before) = s.history.undo(&s.document) { s.document = before; }
        Ok(Dirty::everything())
    }
    pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        let s = self.session_mut(id)?;
        if let Some(after) = s.history.redo(&s.document) { s.document = after; }
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
        Ok(compositor::composite(&self.session(id)?.document, region, width, height))
    }

    pub fn render_plan(&self, id: Uuid, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError> { Ok(plan::render_plan(&self.session(id)?.document, edit)) }
    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit(&self.session(id)?.document, edit, region, w, h)) }
    pub fn clip_dependents(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, CommandError> { Ok(ops::hierarchy::clip_dependents(&self.session(id)?.document, ids)) }
    pub fn merge_action(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<&'static str>, CommandError> { Ok(ops::merge::merge_plan(&self.session(id)?.document, ids).map(|p| p.action)) }
    pub fn group_box(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<LayerTransform>, CommandError> { Ok(ops::transform::group_box(&self.session(id)?.document, ids)) }
    pub fn can_toggle_clipping(&self, id: Uuid, layer: Uuid) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_toggle_clipping(&self.session(id)?.document, layer)) }
    pub fn can_place(&self, id: Uuid, layer: Uuid, parent: Option<Uuid>) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_place(&self.session(id)?.document, layer, parent)) }
}
