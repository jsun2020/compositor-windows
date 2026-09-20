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
                pixels_revision: l.pixels_revision, has_mask: l.mask.is_some(),
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
            Command::CanvasSize { .. } | Command::Crop { .. } | Command::ImageSize { .. } | Command::FlipCanvas { .. } =>
                Err(CommandError::Argument("not implemented".into())),
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

    pub fn import_image(&mut self, _id: Option<Uuid>, _bytes: &[u8], _name: &str, _at: Option<Point>) -> Result<Uuid, CommandError> {
        Err(CommandError::Argument("not implemented".into())) // Task 9
    }

    pub fn export_png(&self, id: Uuid) -> Result<Vec<u8>, CommandError> { Ok(compositor::export_png(&self.session(id)?.document)?) }
    pub fn export_jpeg(&self, id: Uuid, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg(&self.session(id)?.document, quality, matte)?)
    }
    pub fn composite(&self, id: Uuid, region: Rect, width: u32, height: u32) -> Result<Raster, CommandError> {
        Ok(compositor::composite(&self.session(id)?.document, region, width, height))
    }
}
