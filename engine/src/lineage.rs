//! Changed rectangles (Phase 4b-1): which part of a layer's pixels or mask changed from one revision
//! to another. The GPU keeps each texture keyed by the revision it uploaded, asks what changed since
//! (`Engine::pixels_delta`, `Engine::mask_delta`) and uploads only that part; a question survives
//! skipped frames, undo and redo, where a per-command notice would not.
use crate::{Document, LayerTransform, PixelRect, Plane, Region};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// How many changes a document remembers. A renderer further behind than this uploads whole.
pub const LINEAGE_LIMIT: usize = 256;

/// One buffer moving from revision `from` to `to`: changed only within `rect` (in the new buffer's
/// grid), or wholly (None: no rectangle was known, or the buffer changed size).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Change { layer: Uuid, plane: Plane, from: u64, to: u64, rect: Option<PixelRect> }

/// A document's recent changes, oldest first.
#[derive(Debug, Default)]
pub struct Lineage { changes: VecDeque<Change> }

/// One layer buffer as `planes` sees it: its revision, size, placement grid, and the buffer itself
/// (its identity, `Raster::buffer_id` / `GrayRaster::buffer_id`).
type PlaneOf = (u64, (u32, u32), LayerTransform, usize);

/// Every layer buffer of `doc`: its revision, size, placement grid (a layer's own transform for its
/// pixels; a mask's own placement, falling back to the layer's transform) and buffer identity. A
/// spreading filter (Gaussian / Motion blur) can grow a buffer and then trim it back to the SAME size
/// on a SHIFTED grid (`ops::adjust::grown`/`trimmed` moving the transform): size equality alone cannot
/// tell that case from an untouched grid, so the grid travels alongside the size wherever "unchanged"
/// matters (Task 3 fix round 1, bug 1). The identity tells a move that kept the very buffer (a nudge
/// of a layer whose mask has its own placement, or its undo) from a new one: the bytes are the same,
/// only where they are drawn moved, and a texture holds only the bytes (the placement is the render
/// plan's). Compared only between two documents both alive, whose buffers cannot share an address.
fn planes(doc: &Document) -> HashMap<(Uuid, Plane), PlaneOf> {
    let mut out = HashMap::new();
    for l in &doc.layers {
        if let Some(p) = &l.pixels { out.insert((l.id, Plane::Pixels), (l.pixels_revision, (p.width, p.height), l.transform, p.buffer_id())); }
        if let Some(m) = &l.mask { out.insert((l.id, Plane::Mask), (l.mask_revision, (m.pixels.width, m.pixels.height), m.placement.unwrap_or(l.transform), m.pixels.buffer_id())); }
    }
    out
}

/// What changed between two states of one buffer when nothing reported where: nothing, when it is
/// the very same buffer at the same size (its placement alone moved: final review minor 9, which made
/// a nudge of a grown 100 MP mask re-upload it whole); `fallback` otherwise.
fn unless_same_buffer(was: &PlaneOf, now: &PlaneOf, fallback: Option<PixelRect>) -> Option<PixelRect> {
    if was.3 == now.3 && was.1 == now.1 { Some(PixelRect::default()) } else { fallback }
}

impl Lineage {
    /// Records one change of a layer's buffer from revision `from` to `to`, within `rect` (None: whole).
    pub fn record(&mut self, layer: Uuid, plane: Plane, from: u64, to: u64, rect: Option<PixelRect>) {
        self.push(Change { layer, plane, from, to, rect });
    }
    fn push(&mut self, change: Change) {
        if self.changes.len() == LINEAGE_LIMIT { self.changes.pop_front(); }
        self.changes.push_back(change);
    }
    /// Records how `after` differs from `before` after an edit: every buffer whose revision moved,
    /// within the union of the `regions` reported for it when it kept both its size AND its grid
    /// (bug 1: a same-size buffer on a moved grid holds different content at every index), wholly
    /// otherwise. The very same buffer, moved or not, changed nothing (`unless_same_buffer`).
    pub fn record_edit(&mut self, before: &Document, after: &Document, regions: &[Region]) {
        let old = planes(before);
        for ((layer, plane), now) in planes(after) {
            let (to, size, grid, _) = now;
            let was = old.get(&(layer, plane));
            if was.map(|w| w.0) == Some(to) { continue; }
            let reported = regions.iter().filter(|r| r.layer == layer && r.plane == plane).map(|r| r.rect).reduce(|a, b| a.union(&b));
            let same_grid = was.map(|w| (w.1, w.2)) == Some((size, grid));
            let rect = if same_grid { reported } else { None };
            let rect = match was { Some(w) => unless_same_buffer(w, &now, rect), None => rect };
            self.push(Change { layer, plane, from: was.map_or(0, |w| w.0), to, rect });
        }
    }
    /// Records an undo, redo or revert from `before` to `after`: each buffer whose revision moved
    /// changed where the edit joining those two revisions changed it (in either direction), or wholly
    /// when that edit is no longer remembered, or when the grid does not match on both ends (bug 1);
    /// nothing when it is the very same buffer (`unless_same_buffer`).
    pub fn record_return(&mut self, before: &Document, after: &Document) {
        let old = planes(before);
        for ((layer, plane), now) in planes(after) {
            let (to, size, grid, _) = now;
            let was = old.get(&(layer, plane));
            let Some(&w) = was else { self.push(Change { layer, plane, from: 0, to, rect: None }); continue };
            let (from, old_size, old_grid, _) = w;
            if from == to { continue; }
            let joined = self.changes.iter().rev().find(|c| c.layer == layer && c.plane == plane
                && ((c.from == from && c.to == to) || (c.from == to && c.to == from)));
            let rect = if old_size == size && old_grid == grid { joined.and_then(|c| c.rect) } else { None };
            self.push(Change { layer, plane, from, to, rect: unless_same_buffer(&w, &now, rect) });
        }
    }
    /// What changed in `layer`'s `plane` from revision `from` to `current`: an empty rectangle when
    /// nothing did, the union of the changes in between when every one of them had a rectangle, and
    /// None when the whole buffer must be taken again.
    pub fn delta(&self, layer: Uuid, plane: Plane, from: u64, current: u64) -> Option<PixelRect> {
        let mut at = current;
        let mut union = PixelRect::default();
        for _ in 0..LINEAGE_LIMIT {
            if at == from { return Some(union); }
            let c = self.changes.iter().rev().find(|c| c.layer == layer && c.plane == plane && c.to == at)?;
            union = union.union(&c.rect?);
            at = c.from;
        }
        None
    }
}
