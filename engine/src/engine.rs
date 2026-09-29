use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub struct Session {
    pub document: Document,
    pub history: History,
    pub path: Option<String>,
    pub preview: Option<PixelPreview>,
    /// The recent changed rectangles of its layers' buffers (`Engine::pixels_delta`).
    pub lineage: Lineage,
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
    /// Entries on the undo stack (at most 100, fewer when their pixels pass 256 MiB).
    pub undo_depth: usize,
    /// The id of the entry an undo would take back (`History::undo_entry_id`), None with nothing to
    /// undo. A gesture that recorded one command reads it straight after and compares it later to
    /// tell whether its own entry is still on top: the depth cannot, once the cap trims the oldest.
    pub undo_entry_id: Option<u64>,
    pub is_modified: bool,
    pub path: Option<String>,
    pub layers: Vec<LayerState>,
    pub guides: Vec<Guide>,
    /// What this project contains that this build does not draw yet (`Document::undrawn`).
    pub undrawn: Vec<String>,
    /// The selection's summary (Phase 4a); None when nothing is selected. Its outline travels
    /// separately, when its revision changes (`Engine::selection_outline`).
    pub selection: Option<SelectionState>,
}

/// Preview revisions start here so they can never collide with a layer's own, which the engine
/// issues from 1 upward as documents are edited (`renew_revisions`).
const PREVIEW_REVISION_BASE: u64 = 1 << 40;

/// Gives every layer of `next` whose pixels or mask differ from `before` (new layer, other buffer,
/// buffer added or dropped, or another revision) a revision `counter` never issued before. A layer
/// opened or made fresh has revision 1, so the counter issues 2 onwards. Undo and redo restore a
/// snapshot with the revisions its content had; without this, a different edit after an undo
/// would count up to a revision the undone content already had, and a renderer keyed by revision
/// (the GPU's textures) would keep showing that content.
fn renew_revisions(before: &Document, next: &mut Document, counter: &mut u64) {
    let old: HashMap<Uuid, &Layer> = before.layers.iter().map(|l| (l.id, l)).collect();
    let mut issue = || { *counter = (*counter).max(1) + 1; debug_assert!(*counter < PREVIEW_REVISION_BASE); *counter };
    for layer in &mut next.layers {
        let was = old.get(&layer.id);
        let pixels_changed = was.map_or(true, |w| w.pixels_revision != layer.pixels_revision || match (&w.pixels, &layer.pixels) {
            (Some(a), Some(b)) => !a.same_pixels(b),
            (a, b) => a.is_some() != b.is_some(),
        });
        let mask_changed = was.map_or(true, |w| w.mask_revision != layer.mask_revision || match (&w.mask, &layer.mask) {
            (Some(a), Some(b)) => !a.pixels.same_pixels(&b.pixels),
            (a, b) => a.is_some() != b.is_some(),
        });
        if pixels_changed { layer.pixels_revision = issue(); }
        if mask_changed { layer.mask_revision = issue(); }
    }
    // The selection's outline is fetched by the app when its revision is new (Phase 4a).
    if next.selection != before.selection { next.selection_revision = issue(); }
}

#[derive(Default)]
pub struct Engine {
    sessions: HashMap<Uuid, Session>, order: Vec<Uuid>, preview_revision: u64, revision: u64, effects: EffectsCache, clips: SelectionClips,
    /// Entries and bytes each document's history keeps; None for the Mac's 100 and 256 MiB.
    history_limits: Option<(usize, usize)>,
}

/// Where the document's selection reaches `layer`'s pixel or mask grid: the clip's rectangle on that
/// grid (`SelectionClip::rect_on_grid`). Nothing without a selection.
fn selection_regions(doc: &Document, clips: &SelectionClips, layer: Uuid, plane: Plane) -> Vec<Region> {
    let (Some(clip), Some(l)) = (clips.clip(doc), doc.layer(layer)) else { return vec![] };
    let (grid, w, h) = match (plane, &l.pixels, &l.mask) {
        (Plane::Pixels, Some(p), _) => (l.transform, p.width, p.height),
        (Plane::Mask, _, Some(m)) => (m.placement.unwrap_or(l.transform), m.pixels.width, m.pixels.height),
        _ => return vec![],
    };
    clip.rect_on_grid(&grid.pixel_to_document(w, h), w, h).map(|rect| vec![Region { layer, plane, rect }]).unwrap_or_default()
}

/// Records in the lineage how a patch preview changed what the canvas shows of a layer's pixels: from
/// the stored pixels to a patch, from one patch to the next (both rectangles), and back. The GPU then
/// uploads only those rectangles (`pixels_delta`, `layer_region`); any other preview goes whole.
fn record_preview(lineage: &mut Lineage, doc: &Document, old: Option<&PixelPreview>, new: Option<&PixelPreview>) {
    let patch = |p: Option<&PixelPreview>| p.and_then(|p| match p.target { PreviewTarget::Patch(r) => Some((p.layer, p.revision, r)), _ => None });
    let stored = |layer: Uuid| doc.layer(layer).map(|l| l.pixels_revision);
    match (patch(old), patch(new)) {
        (Some((a, from, r1)), Some((b, to, r2))) if a == b => lineage.record(a, Plane::Pixels, from, to, Some(r1.union(&r2))),
        (old_patch, new_patch) => {
            if let Some((layer, from, r)) = old_patch { if let Some(to) = stored(layer) { lineage.record(layer, Plane::Pixels, from, to, Some(r)); } }
            if let Some((layer, to, r)) = new_patch { if let Some(from) = stored(layer) { lineage.record(layer, Plane::Pixels, from, to, Some(r)); } }
        }
    }
}

/// Gives each layer's new pixels, changed only within a reported region, the halvings its old pixels
/// had, redone only there (`Raster::seed_halvings`): the GPU at a reduced zoom then uploads the region
/// without halving the whole layer again. Skips a layer whose transform moved between `before` and
/// `after` (bug 1): a spreading filter can grow a buffer and trim it back to the SAME size on a
/// SHIFTED grid, and `Raster::seed_halvings` only checks width and height -- it cannot see that the
/// grid itself moved, so the caller must refuse to seed from a different grid's halvings, which are
/// of different content at every index.
fn seed_halvings(before: &Document, after: &Document, regions: &[Region]) {
    let mut per_layer: HashMap<Uuid, PixelRect> = HashMap::new();
    for r in regions.iter().filter(|r| r.plane == Plane::Pixels) { per_layer.entry(r.layer).and_modify(|u| *u = u.union(&r.rect)).or_insert(r.rect); }
    for (layer, rect) in per_layer {
        let (Some(old_layer), Some(new_layer)) = (before.layer(layer), after.layer(layer)) else { continue };
        if old_layer.transform != new_layer.transform { continue; }
        if let (Some(old), Some(new)) = (&old_layer.pixels, &new_layer.pixels) { new.seed_halvings(old, rect); }
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), CommandError> {
    if !(1..=MAX_SIDE as u32).contains(&width) || !(1..=MAX_SIDE as u32).contains(&height) {
        return Err(CommandError::Argument("width and height must be 1 to 30000".into()));
    }
    Ok(())
}

impl Engine {
    pub fn new() -> Engine { Engine::default() }
    /// An engine keeping its effects images in `effects` (tests give it small limits).
    pub fn with_effects_cache(effects: EffectsCache) -> Engine { Engine { effects, ..Engine::default() } }
    /// An engine whose documents keep at most `entries` history entries and `bytes` of pixels only
    /// history holds (tests give it small limits; `History::default` has the Mac's).
    pub fn with_history_limits(entries: usize, bytes: usize) -> Engine { Engine { history_limits: Some((entries, bytes)), ..Engine::default() } }
    /// The effects images this engine keeps (`EffectsCache`).
    pub fn effects_cache(&self) -> &EffectsCache { &self.effects }
    /// The selection clips this engine keeps (`SelectionClips`), one per document.
    pub fn selection_clips(&self) -> &SelectionClips { &self.clips }
    pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }

    /// Sessions are keyed by a fresh handle, independent of `Document.id` (the stable manifest
    /// id), so the same document can be open more than once and ids reassigned by rare
    /// collisions with a live session never happen.
    fn insert(&mut self, document: Document, path: Option<String>) -> Uuid {
        let handle = Uuid::new_v4();
        let history = self.history_limits.map_or_else(History::default, |(entries, bytes)| History::with_limits(entries, bytes));
        self.sessions.insert(handle, Session { document, history, path, preview: None, lineage: Lineage::default() });
        self.order.push(handle);
        handle
    }
    /// A document of its own, with fresh history: a job's document (`jobs.rs`).
    pub fn insert_document(&mut self, document: Document) -> Uuid { self.insert(document, None) }
    pub(crate) fn session(&self, id: Uuid) -> Result<&Session, CommandError> { self.sessions.get(&id).ok_or(CommandError::NoDocument) }
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
        if let Some(s) = self.sessions.remove(&id) { self.clips.forget(s.document.id); }
        self.order.retain(|d| *d != id);
        self.effects.prune();
    }

    pub fn state(&self, id: Uuid) -> Result<DocumentState, CommandError> {
        let s = self.session(id)?;
        let doc = self.render_document(id)?;
        let d = &*doc;
        Ok(DocumentState {
            id, document_id: d.id, width: d.width, height: d.height, resolution: d.resolution, active_layer_id: d.active_layer_id,
            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), undo_depth: s.history.depth(),
            undo_entry_id: s.history.undo_entry_id(), is_modified: s.history.is_modified(),
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
            guides: d.guides.clone(),
            undrawn: d.undrawn(),
            selection: d.selection.as_ref().map(|s| SelectionState {
                revision: d.selection_revision, empty: s.is_empty(), bounds: s.bounds(),
                antialiased: s.antialiased, feather: s.feather, points: s.point_count(),
            }),
        })
    }

    /// The selection's outline for the marching ants, in document pixels, flat: the number of
    /// contours, then for each its number of points and their x, y. At most `OUTLINE_DETAIL_LIMIT`
    /// points are sent as they are; past that, and when the view shows fewer than one screen pixel
    /// per document pixel (`step` < 1, a power of two), the outline traced at that resolution
    /// (`selection_lod`): never more edges than the screen has pixels for. Empty with no selection.
    pub fn selection_outline(&self, id: Uuid, step: f64) -> Result<Vec<f64>, CommandError> {
        let doc = &self.session(id)?.document;
        let Some(selection) = &doc.selection else { return Ok(Vec::new()) };
        let points = selection.point_count();
        if points > OUTLINE_DETAIL_LIMIT && step < 1.0 {
            let contours = selection_lod(selection, doc.width, doc.height, step);
            let mut out = Vec::with_capacity(1 + contours.len() + 2 * contours.iter().map(Vec::len).sum::<usize>());
            out.push(contours.len() as f64);
            for c in &contours {
                out.push(c.len() as f64);
                for p in c { out.push(p.x); out.push(p.y); }
            }
            return Ok(out);
        }
        // The outline itself, written straight into the one flat buffer (final review F3: a
        // 4-million-point outline went through a Vec<Vec<Point>> first).
        let mut out = Vec::with_capacity(1 + selection.contours.len() + 2 * points);
        out.push(selection.contours.len() as f64);
        for c in selection.contours.iter() {
            out.push(c.len() as f64);
            for p in c { out.push(p[0] as f64 / SUBPIXEL); out.push(p[1] as f64 / SUBPIXEL); }
        }
        Ok(out)
    }

    /// What changed in `layer`'s pixels since revision `from`, as the canvas shows them (a preview's
    /// pixels have revisions of their own): an empty rectangle for nothing, a rectangle of the pixel
    /// grid, or None when the whole raster must be uploaded again (`Lineage::delta`).
    pub fn pixels_delta(&self, id: Uuid, layer: Uuid, from: u64) -> Result<Option<PixelRect>, CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        Ok(self.session(id)?.lineage.delta(layer, Plane::Pixels, from, l.pixels_revision))
    }
    /// `pixels_delta` for the layer's mask as the canvas shows it, in the mask's own grid.
    pub fn mask_delta(&self, id: Uuid, layer: Uuid, from: u64) -> Result<Option<PixelRect>, CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        Ok(self.session(id)?.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
    }
    /// The pixels a Fill or a Gradient on `layer` (its mask when `mask`) paints, from the stored
    /// document (ruling C1; `ops::raster_edit::painted_pixels`): what decides whether the edit goes to the
    /// job worker, never the size the layer stores (a blank layer on a 100 MP canvas paints 100 MP, and so
    /// does a small layer's mask, grown to that canvas: Task 14a). An error where the edit is refused for
    /// its size, which the worker, seeing one layer, could not tell.
    pub fn edit_pixels(&self, id: Uuid, layer: Uuid, mask: bool) -> Result<u64, CommandError> {
        ops::raster_edit::painted_pixels(&self.session(id)?.document, layer, mask)
    }
    /// The preview showing on the canvas, if any.
    pub fn preview(&self, id: Uuid) -> Option<&PixelPreview> { self.sessions.get(&id).and_then(|s| s.preview.as_ref()) }
    /// The pixels the layer stores (its width times its height; 0 without pixels), whatever a preview
    /// shows: `state()` reports a previewed layer at the preview's size, which may be a reduced copy.
    pub fn stored_pixels(&self, id: Uuid, layer: Uuid) -> Result<u64, CommandError> {
        let l = self.session(id)?.document.layer(layer).ok_or(CommandError::NoLayer)?;
        Ok(l.pixels.as_ref().map_or(0, |p| p.width as u64 * p.height as u64))
    }
    /// The layer's mask as the canvas shows it (a gradient's mask preview in its place).
    pub fn mask_pixels(&self, id: Uuid, layer: Uuid) -> Result<Option<GrayRaster>, CommandError> {
        let doc = self.render_document(id)?;
        Ok(doc.layer(layer).ok_or(CommandError::NoLayer)?.mask.as_ref().map(|m| m.pixels.clone()))
    }
    /// The bytes of `rect` (in the raster's grid after `level` halvings) of the layer's pixels as the
    /// canvas shows them: what the GPU's partial upload sends. Through a patch preview, the rectangle
    /// is made from the stored pixels and the patch at full size and halved on its own, which equals
    /// the same part of the whole patched raster halved (every block lies inside the rectangle).
    pub fn layer_region(&self, id: Uuid, layer: Uuid, level: u32, rect: PixelRect) -> Result<Vec<u8>, CommandError> {
        let s = self.session(id)?;
        let patch = s.preview.as_ref().filter(|p| p.layer == layer).and_then(|p| match p.target { PreviewTarget::Patch(r) => Some((r, &p.raster)), _ => None });
        let stored = s.document.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone().ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
        let f = 1u32 << level;
        // Every level of the halving keeps whole 2 x 2 blocks while both sides stay above 1.
        let whole_blocks = (stored.width >> level.saturating_sub(1)) > 1 && (stored.height >> level.saturating_sub(1)) > 1;
        let raster = match patch {
            Some((prect, praster)) if whole_blocks => {
                let (x0, y0, w, h) = (rect.x * f, rect.y * f, rect.width * f, rect.height * f);
                let mut data = stored.cropped(x0, y0, w, h).into_bytes();
                for y in prect.y.max(y0)..(prect.y + prect.height).min(y0 + h) {
                    let (from, to) = (prect.x.max(x0), (prect.x + prect.width).min(x0 + w));
                    if from >= to { continue; }
                    let src = (((y - prect.y) * prect.width + (from - prect.x)) * 4) as usize;
                    let dst = (((y - y0) * w + (from - x0)) * 4) as usize;
                    data[dst..dst + ((to - from) * 4) as usize].copy_from_slice(&praster.bytes()[src..src + ((to - from) * 4) as usize]);
                }
                let mut region = Raster::from_premultiplied(w, h, data);
                for _ in 0..level { region = region.halved(); }
                return Ok(region.into_bytes());
            }
            _ => self.layer_raster(id, layer, level)?.ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?,
        };
        Ok(raster.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes())
    }

    /// Whether a press at `at` lands inside a selection with something in it, by the winding rule:
    /// where a drag in New mode moves the outline instead of drawing (`canMoveSelection(at:)`,
    /// Selection.swift:256-260).
    pub fn selection_contains(&self, id: Uuid, at: Point) -> Result<bool, CommandError> {
        let doc = &self.session(id)?.document;
        Ok(doc.selection.as_ref().map_or(false, |s| !s.is_empty() && s.contains(at)))
    }

    /// The document as the canvas should show it: the stored one, or a copy with the open
    /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
    /// the ops do not, because a preview is not committed. A patch preview keeps the stored pixels
    /// here, under the preview's revision: what reads the bytes asks `render_bytes`.
    pub(crate) fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> { self.displayed(id, false) }
    /// `render_document` with a patch preview drawn into the layer's pixels (made once per preview).
    pub(crate) fn render_bytes(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> { self.displayed(id, true) }
    fn displayed(&self, id: Uuid, bytes: bool) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
        let s = self.session(id)?;
        let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
        let mut doc = s.document.clone();
        match &preview.target {
            PreviewTarget::Mask { pixels, placement } => {
                if let Some(m) = doc.layer_mut(preview.layer).and_then(|l| { l.mask_revision = preview.revision; l.mask.as_mut() }) {
                    m.pixels = pixels.clone();
                    // Where the commit will leave it: a mask gradient grows the mask past its layer (Task 14a).
                    m.placement = *placement;
                }
                return Ok(std::borrow::Cow::Owned(doc));
            }
            PreviewTarget::Patch(_) => {
                if let Some(layer) = doc.layer_mut(preview.layer) {
                    layer.pixels_revision = preview.revision;
                    if bytes { layer.pixels = layer.pixels.as_ref().map(|stored| preview.patched(stored)); }
                }
                return Ok(std::borrow::Cow::Owned(doc));
            }
            PreviewTarget::Pixels => {}
        }
        if let Some(layer) = doc.layer_mut(preview.layer) {
            // A preview may come from a reduced copy (preview.rs): effects, measured in the layer's
            // pixels, shrink with it, so they show at the size the committed layer will draw them.
            let density = |width: u32, t: &LayerTransform| width as f64 / t.size.width;
            let scaled = match (layer.pixels.as_ref(), layer.extra.effects.as_ref()) {
                (Some(stored), Some(effects)) => {
                    let factor = density(preview.raster.width, &preview.transform) / density(stored.width, &layer.transform);
                    (factor != 1.0).then(|| effects.scaled(factor))
                }
                _ => None,
            };
            if let Some(effects) = scaled { layer.extra.effects = Some(effects); }
            layer.pixels = Some(preview.raster.clone());
            layer.pixels_revision = preview.revision;
            layer.transform = preview.transform;
        }
        Ok(std::borrow::Cow::Owned(doc))
    }
    pub fn set_preview(&mut self, id: Uuid, request: Option<PreviewRequest>) -> Result<Dirty, CommandError> {
        // A request that would compute the pixels already showing keeps them: the settled request
        // after a Grain drag (full size either way) would otherwise recompute the whole layer.
        // The key includes what the pixels were computed FROM (the stored layer's revision and
        // placement, and the selection's revision), so an edit that forgets to clear the preview
        // can never be answered with stale pixels (phase 3 open item N3).
        let s = self.session(id)?;
        if let (Some(r), Some(current)) = (&request, &s.preview) {
            if current.answers(r, &PreviewSource::of(&s.document, r.layer())) { return Ok(Dirty::default()); }
        }
        let revision = { self.preview_revision += 1; PREVIEW_REVISION_BASE + self.preview_revision };
        let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
        let layers: Vec<Uuid> = s.preview.iter().map(|p| p.layer).chain(request.iter().map(|r| r.layer())).collect();
        let clips = &self.clips;
        let next = request.as_ref().and_then(|r| preview::compute_preview_with(&s.document, clips, r, revision));
        record_preview(&mut s.lineage, &s.document, s.preview.as_ref(), next.as_ref());
        s.preview = next;
        Ok(Dirty::pixels(layers))
    }
    pub(crate) fn clear_preview(&mut self, id: Uuid) {
        if let Ok(s) = self.session_mut(id) {
            record_preview(&mut s.lineage, &s.document, s.preview.as_ref(), None);
            s.preview = None;
        }
    }

    /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
    /// `f` reads the selection's clip through the engine's cache (`SelectionClips`).
    pub(crate) fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
    where F: FnOnce(&mut Document, &SelectionClips) -> Result<Dirty, CommandError> {
        let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
        let mut next = s.document.clone();
        let dirty = f(&mut next, &self.clips)?;
        // The active layer is selection, not content: a command that only moves it (SetActiveLayer)
        // still applies, but records no history entry and so leaves redo intact, as macOS does.
        let content_changed = !next.same_content(&s.document);
        renew_revisions(&s.document, &mut next, &mut self.revision);
        s.lineage.record_edit(&s.document, &next, &dirty.regions);
        seed_halvings(&s.document, &next, &dirty.regions);
        let before = std::mem::replace(&mut s.document, next);
        if content_changed { s.history.push(before); s.history.trim(&s.document); }
        self.clips.retain_current(&s.document);
        Ok(dirty)
    }

    pub fn execute(&mut self, handle: Uuid, command: Command) -> Result<Dirty, CommandError> {
        self.clear_preview(handle);
        self.edit(handle, |doc, clips| match command {
            Command::AddBlankLayer => { ops::layers::add_blank_layer(doc)?; Ok(Dirty::structure()) }
            Command::RenameLayer { id, name } => { ops::layers::rename_layer(doc, id, &name)?; Ok(Dirty::structure()) }
            Command::SetLayerVisible { id, visible } => { ops::layers::set_layer_visible(doc, id, visible)?; Ok(Dirty::structure()) }
            Command::DeleteLayer { id } => { ops::layers::delete_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::SetActiveLayer { id } => { ops::layers::set_active_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::CanvasSize { width, height, anchor, fill } => {
                *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions { width, height, anchor, fill, content_offset: None })?;
                // Canvas Size, Crop and Image Size make a new document on the Mac, which drops the
                // selection in the same undo step (ImageResizer.swift:109-116).
                doc.selection = None;
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
                doc.selection = None;
                Ok(Dirty::everything())
            }
            Command::FlipCanvas { horizontal } => {
                ops::flip::flip_canvas(doc, horizontal);
                ops::selection::flip_selection(doc, horizontal);
                Ok(Dirty::structure())
            }
            Command::ImageSize { width, height, resolution, sampling } => {
                *doc = ops::image_size::image_size(doc, ops::image_size::ImageSizeOptions { width, height, resolution, sampling })?;
                doc.selection = None;
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
                Ok(Dirty::pixels(baked))
            }
            Command::SetLayerTransform { id, transform } => { ops::transform::set_transform(doc, id, transform)?; Ok(Dirty::structure()) }
            Command::TransformLayers { ids, bounds, draft } => { ops::transform::transform_group(doc, &ids, &bounds, &draft)?; Ok(Dirty::structure()) }
            Command::FlipLayers { ids, horizontal } => { ops::transform::flip_layers(doc, &ids, horizontal)?; Ok(Dirty::structure()) }
            Command::NudgeLayers { ids, dx, dy } => { ops::transform::nudge(doc, &ids, dx, dy)?; Ok(Dirty::structure()) }
            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty::pixels(vec![id])) }
            Command::DistortLayers { ids, bounds, draft, corners } => {
                let touched = ops::transform::members(doc, &ids);
                ops::distort::distort_group(doc, &ids, &bounds, &draft, &corners)?;
                Ok(Dirty::pixels(touched))
            }
            Command::SetMaskPlacement { id, placement } => { ops::transform::set_mask_placement(doc, id, placement)?; Ok(Dirty::structure()) }
            Command::AddMask { id, revealing } => { ops::masks::add_mask(doc, id, revealing)?; Ok(Dirty::structure()) }
            Command::DeleteMask { id } => { ops::masks::delete_mask(doc, id)?; Ok(Dirty::structure()) }
            Command::SetMaskEnabled { id, enabled } => { ops::masks::set_mask_enabled(doc, id, enabled)?; Ok(Dirty::structure()) }
            Command::SetMaskLinked { id, linked } => { ops::masks::set_mask_linked(doc, id, linked)?; Ok(Dirty::structure()) }
            Command::InvertMask { id } => { ops::adjust::invert_layer_with(doc, clips, id, true)?; Ok(Dirty::structure().within(selection_regions(doc, clips, id, Plane::Mask))) }
            Command::FillMask { id, white } => { ops::masks::fill_mask(doc, id, white)?; Ok(Dirty::structure()) }
            Command::BlurMask { id, radius } => { ops::masks::blur_mask(doc, id, radius)?; Ok(Dirty::structure()) }
            Command::CopyMask { from, to } => { ops::masks::copy_mask(doc, from, to)?; Ok(Dirty::structure()) }
            Command::ToggleClipping { id } => { ops::hierarchy::toggle_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::ReleaseClipping { id } => { ops::hierarchy::release_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::LinkMask { source, target } => { ops::hierarchy::link_mask(doc, source, target)?; Ok(Dirty::structure()) }
            Command::MergeLayers { ids } => { let m = ops::merge::merge(doc, &ids)?; Ok(Dirty::pixels(vec![m])) }
            // Inside a selection these change only what the selection reaches on the layer's grid.
            Command::ApplyAdjustment { id, adjustment } => {
                ops::adjust::apply_adjustment_to_layer_with(doc, clips, id, &adjustment)?;
                Ok(Dirty::pixels(vec![id]).within(selection_regions(doc, clips, id, Plane::Pixels)))
            }
            Command::InvertPixels { id, mask } => {
                ops::adjust::invert_layer_with(doc, clips, id, mask)?;
                let plane = if mask { Plane::Mask } else { Plane::Pixels };
                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
            }
            Command::ApplyFilter { id, params } => {
                ops::adjust::apply_filter_with(doc, clips, id, &params)?;
                Ok(Dirty::pixels(vec![id]).within(selection_regions(doc, clips, id, Plane::Pixels)))
            }
            Command::AddAdjustmentLayer { kind, seed, shadows, highlights } => {
                let gradient = match (shadows, highlights) { (Some(s), Some(h)) => Some((s, h)), _ => None };
                ops::adjust::add_adjustment_layer(doc, kind, seed, gradient)?; Ok(Dirty::structure())
            }
            Command::SetAdjustment { id, adjustment } => { ops::adjust::set_adjustment(doc, id, &adjustment)?; Ok(Dirty::structure()) }
            Command::SelectShape { kind, points, mode, antialiased } => { ops::selection::select_shape(doc, kind, &points, mode, antialiased)?; Ok(Dirty::structure()) }
            Command::SelectAll => { ops::selection::select_all(doc); Ok(Dirty::structure()) }
            Command::Deselect => { ops::selection::deselect(doc); Ok(Dirty::structure()) }
            Command::InvertSelection => { ops::selection::invert_selection(doc); Ok(Dirty::structure()) }
            Command::MoveSelection { dx, dy } => { ops::selection::move_selection(doc, dx, dy)?; Ok(Dirty::structure()) }
            Command::ExpandSelection { amount } => { ops::selection::resize_selection(doc, amount as i64)?; Ok(Dirty::structure()) }
            Command::ContractSelection { amount } => { ops::selection::resize_selection(doc, -(amount as i64))?; Ok(Dirty::structure()) }
            Command::FeatherSelection { amount } => { ops::selection::feather_selection(doc, amount)?; Ok(Dirty::structure()) }
            Command::MagicWand { at, mode, settings, antialiased } => { ops::selection::magic_wand_select(doc, at, mode, &settings, antialiased)?; Ok(Dirty::structure()) }
            Command::LoadLayerSelection { id, mode, antialiased } => { ops::selection::load_layer_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
            Command::LoadMaskSelection { id, mode, antialiased } => { ops::selection::load_mask_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
            Command::ClearSelectedPixels { id, mask } => {
                ops::selection::clear_selected(doc, clips, id, mask)?;
                let plane = if mask { Plane::Mask } else { Plane::Pixels };
                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
            }
            Command::AddMaskFromSelection { id, revealing } => { ops::selection::add_mask_from_selection(doc, clips, id, revealing)?; Ok(Dirty::structure()) }
            Command::Fill { id, mask, color } => {
                ops::raster_edit::paint_layer(doc, clips, id, mask, &ops::raster_edit::Paint::Fill(color))?;
                let plane = if mask { Plane::Mask } else { Plane::Pixels };
                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
            }
            Command::Gradient { id, mask, gradient } => {
                ops::raster_edit::paint_layer(doc, clips, id, mask, &ops::raster_edit::Paint::Gradient(gradient))?;
                let plane = if mask { Plane::Mask } else { Plane::Pixels };
                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
            }
            Command::AddShape { shape, color } => { ops::shape::add_shape(doc, &shape, color)?; Ok(Dirty::structure()) }
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
        let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
        if let Some(before) = s.history.undo(&s.document) {
            let after = std::mem::replace(&mut s.document, before);
            s.lineage.record_return(&after, &s.document);
            s.history.trim(&s.document);
        }
        self.clips.retain_current(&s.document);
        Ok(Dirty::everything())
    }
    pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        if self.drop_preview(id) { return Ok(Dirty::everything()); }
        let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
        if let Some(after) = s.history.redo(&s.document) {
            let before = std::mem::replace(&mut s.document, after);
            s.lineage.record_return(&before, &s.document);
            s.history.trim(&s.document);
        }
        self.clips.retain_current(&s.document);
        Ok(Dirty::everything())
    }
    /// Drops the last history entry and returns to the state before it, leaving no redo: for a
    /// gesture the user cancelled, such as Escape during an Alt-drag duplicate. Unlike undo/redo,
    /// an explicit revert always runs even while a preview is showing: it is an unambiguous
    /// request to discard changes, and a no-op here would be a silent failure of that request.
    pub fn revert(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
        self.clear_preview(id);
        let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
        if let Some(before) = s.history.revert() {
            let after = std::mem::replace(&mut s.document, before);
            s.lineage.record_return(&after, &s.document);
        }
        self.clips.retain_current(&s.document);
        Ok(Dirty::everything())
    }

    pub fn import_image(&mut self, id: Option<Uuid>, bytes: &[u8], name: &str, at: Option<Point>) -> Result<Uuid, CommandError> {
        let raster = decode_image(bytes)?.raster;
        match id {
            Some(id) => {
                self.edit(id, |doc, _| { ops::layers::import_raster(doc, raster, name, at)?; Ok(Dirty::structure()) })?;
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

    // Exports and every render below find effects images in, and leave them in, the engine's cache.
    pub fn export_png(&self, id: Uuid) -> Result<Vec<u8>, CommandError> { Ok(compositor::export_png_with(&self.session(id)?.document, &self.effects)?) }
    pub fn export_jpeg(&self, id: Uuid, quality: f64, matte: [f64; 3]) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg_with(&self.session(id)?.document, quality, matte, &self.effects)?)
    }
    pub fn export_jpeg_preview(&self, id: Uuid, quality: f64, matte: [f64; 3], max_side: u32) -> Result<Vec<u8>, CommandError> {
        Ok(compositor::export_jpeg_preview_with(&self.session(id)?.document, quality, matte, max_side, &self.effects)?)
    }
    pub fn composite(&self, id: Uuid, region: Rect, width: u32, height: u32) -> Result<Raster, CommandError> {
        Ok(compositor::composite_edit_with(&*self.render_bytes(id)?, None, region, width, height, &self.effects))
    }

    pub fn render_plan(&self, id: Uuid, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError> { Ok(plan::render_plan(&*self.render_document(id)?, edit)) }
    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit_with(&*self.render_bytes(id)?, edit, region, w, h, &self.effects)) }
    pub fn clip_dependents(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, CommandError> { Ok(ops::hierarchy::clip_dependents(&self.session(id)?.document, ids)) }
    pub fn merge_action(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<&'static str>, CommandError> { Ok(ops::merge::merge_plan(&self.session(id)?.document, ids).map(|p| p.action)) }
    pub fn group_box(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<LayerTransform>, CommandError> { Ok(ops::transform::group_box(&self.session(id)?.document, ids)) }
    pub fn can_toggle_clipping(&self, id: Uuid, layer: Uuid) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_toggle_clipping(&self.session(id)?.document, layer)) }
    pub fn can_place(&self, id: Uuid, layer: Uuid, parent: Option<Uuid>) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_place(&self.session(id)?.document, layer, parent)) }

    /// The layer's raster after `level` sharp halvings, through any open preview.
    pub fn layer_raster(&self, id: Uuid, layer: Uuid, level: u32) -> Result<Option<Raster>, CommandError> {
        let doc = self.render_bytes(id)?;
        let Some(mut raster) = doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone() else { return Ok(None); };
        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
            if raster.width <= 1 || raster.height <= 1 { break; }
            raster = raster.halved();
        }
        Ok(Some(raster))
    }
    /// The raster the plan's draw of `layer` samples, through any open preview and pending edit,
    /// after `level` sharp halvings: the layer with its effects around it when the plan draws them
    /// (`effects_draw`), its pixels otherwise. The GPU uploads exactly this as the layer's texture,
    /// so both renderers sample the same bytes. The engine's cache may drop the image at any later
    /// call, so a caller that hands out a pointer into it keeps the raster itself (the wasm bridge's
    /// `prepare_draw_pixels`).
    pub fn draw_raster(&self, id: Uuid, layer: Uuid, level: u32, edit: Option<&PreviewEdit>) -> Result<Option<Raster>, CommandError> {
        let doc = self.render_bytes(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let raster = match effects_draw(l, edit) { Some(fx) => self.effects.image(l, &fx), None => l.pixels.clone() };
        let Some(mut raster) = raster else { return Ok(None); };
        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
            if raster.width <= 1 || raster.height <= 1 { break; }
            raster = raster.halved();
        }
        Ok(Some(raster))
    }
    /// The stored document with the adjustment layer and everything above it hidden: what renders
    /// beneath it, as macOS renders the layers underneath for its histogram and eyedroppers.
    fn beneath(&self, id: Uuid, layer: Uuid) -> Result<Document, CommandError> {
        let doc = &self.session(id)?.document;
        let order = ops::hierarchy::hierarchy_order(doc);
        let position = order.iter().position(|o| *o == layer).ok_or(CommandError::NoLayer)?;
        let beneath: std::collections::HashSet<Uuid> = order[..position].iter().copied().collect();
        let mut below = doc.clone();
        for l in &mut below.layers { if !l.is_group && !beneath.contains(&l.id) { l.visible = false; } }
        Ok(below)
    }
    /// Everything that renders beneath an adjustment layer, composited at canvas size: what its
    /// histogram reads. A full composite, so a panel computes it once, when it opens.
    pub fn adjustment_source(&self, id: Uuid, layer: Uuid) -> Result<Raster, CommandError> {
        let below = self.beneath(id, layer)?;
        Ok(compositor::composite_edit_with(&below, None, Rect { x: 0.0, y: 0.0, width: below.width as f64, height: below.height as f64 }, below.width, below.height, &self.effects))
    }
    /// A panel's histogram: an adjustment layer reads what lies beneath it, any other layer its
    /// own stored pixels (never the preview, or the graph would chase itself), weighted by the
    /// selection's coverage on those pixels (`LevelsFilter.histogram`, Levels.swift:95-110). An
    /// adjustment layer never takes the selection, so its histogram does not either.
    pub fn histogram(&self, id: Uuid, layer: Uuid) -> Result<Vec<Vec<f64>>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        if target.is_adjustment() { return Ok(adjust::levels::histogram(&self.adjustment_source(id, layer)?, None)); }
        let raster = target.pixels.as_ref().ok_or(CommandError::Argument("the layer has no pixels".into()))?;
        let coverage = selection_coverage_with(doc, &self.clips, &target.transform.pixel_to_document(raster.width, raster.height), raster.width, raster.height);
        Ok(adjust::levels::histogram(raster, coverage.as_ref()))
    }
    pub fn auto_levels(&self, id: Uuid, layer: Uuid, mode: LevelsAuto) -> Result<LevelsSettings, CommandError> {
        Ok(mode.settings(&self.histogram(id, layer)?))
    }
    /// The straight colour of one layer at a document point; None where it is transparent. For an
    /// adjustment layer, the colour of what lies beneath it there: one composited pixel, as
    /// `sample_color` does, rather than the whole canvas for every eyedropper click.
    pub fn sample_layer_color(&self, id: Uuid, layer: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let raster = match (target.pixels.as_ref(), target.is_adjustment()) {
            (Some(r), _) => r,
            (None, true) => {
                if at.x < 0.0 || at.y < 0.0 || at.x >= doc.width as f64 || at.y >= doc.height as f64 { return Ok(None); }
                let below = self.beneath(id, layer)?;
                let pixel = compositor::composite_edit_with(&below, None, Rect { x: at.x.floor(), y: at.y.floor(), width: 1.0, height: 1.0 }, 1, 1, &self.effects).pixel(0, 0);
                if pixel[3] == 0 { return Ok(None); }
                return Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))));
            }
            _ => return Err(CommandError::Argument("the layer has no pixels".into())),
        };
        let source = target.transform;
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
    /// The visible composite's colour at the document pixel under `at`, straight and snapped to
    /// 8 bits as the Mac reads it: `(min(a, v) / a * 255).rounded() / 255` (`sampleCompositeColor`,
    /// ColorPalette.swift:182-203). None off the canvas or over a transparent pixel. It reads the
    /// stored document, never an open preview.
    pub fn sample_color(&self, id: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = &self.session(id)?.document;
        if !(at.x >= 0.0 && at.y >= 0.0 && at.x < doc.width as f64 && at.y < doc.height as f64) { return Ok(None); }
        let region = Rect { x: at.x.floor(), y: at.y.floor(), width: 1.0, height: 1.0 };
        let pixel = compositor::composite_edit_with(doc, None, region, 1, 1, &self.effects).pixel(0, 0);
        if pixel[3] == 0 { return Ok(None); }
        let alpha = pixel[3] as f64;
        Ok(Some([0, 1, 2].map(|c| ((pixel[c] as f64).min(alpha) / alpha * 255.0).round() / 255.0)))
    }
    pub fn levels_sampling(&self, id: Uuid, layer: Uuid, settings: &LevelsSettings, at: Point, mode: LevelsSample) -> Result<LevelsSettings, CommandError> {
        match self.sample_layer_color(id, layer, at)? { Some(rgb) => Ok(settings.sampling(rgb, mode)), None => Ok(settings.clone()) }
    }
}
