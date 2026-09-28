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

/// Every layer buffer of `doc`: its revision, size and placement grid (a layer's own transform for
/// its pixels; a mask's own placement, falling back to the layer's transform). A spreading filter
/// (Gaussian / Motion blur) can grow a buffer and then trim it back to the SAME size on a SHIFTED
/// grid (`ops::adjust::grown`/`trimmed` moving the transform): size equality alone cannot tell that
/// case from an untouched grid, so the grid travels alongside the size wherever "unchanged" matters
/// (Task 3 fix round 1, bug 1).
fn planes(doc: &Document) -> HashMap<(Uuid, Plane), (u64, (u32, u32), LayerTransform)> {
    let mut out = HashMap::new();
    for l in &doc.layers {
        if let Some(p) = &l.pixels { out.insert((l.id, Plane::Pixels), (l.pixels_revision, (p.width, p.height), l.transform)); }
        if let Some(m) = &l.mask { out.insert((l.id, Plane::Mask), (l.mask_revision, (m.pixels.width, m.pixels.height), m.placement.unwrap_or(l.transform))); }
    }
    out
}

impl Lineage {
    fn push(&mut self, change: Change) {
        if self.changes.len() == LINEAGE_LIMIT { self.changes.pop_front(); }
        self.changes.push_back(change);
    }
    /// Records how `after` differs from `before` after an edit: every buffer whose revision moved,
    /// within the union of the `regions` reported for it when it kept both its size AND its grid
    /// (bug 1: a same-size buffer on a moved grid holds different content at every index), wholly
    /// otherwise.
    pub fn record_edit(&mut self, before: &Document, after: &Document, regions: &[Region]) {
        let old = planes(before);
        for ((layer, plane), (to, size, grid)) in planes(after) {
            let was = old.get(&(layer, plane));
            if was.map(|w| w.0) == Some(to) { continue; }
            let reported = regions.iter().filter(|r| r.layer == layer && r.plane == plane).map(|r| r.rect).reduce(|a, b| a.union(&b));
            let same_grid = was.map(|w| (w.1, w.2)) == Some((size, grid));
            let rect = if same_grid { reported } else { None };
            self.push(Change { layer, plane, from: was.map_or(0, |w| w.0), to, rect });
        }
    }
    /// Records an undo, redo or revert from `before` to `after`: each buffer whose revision moved
    /// changed where the edit joining those two revisions changed it (in either direction), or wholly
    /// when that edit is no longer remembered, or when the grid does not match on both ends (bug 1).
    pub fn record_return(&mut self, before: &Document, after: &Document) {
        let old = planes(before);
        for ((layer, plane), (to, size, grid)) in planes(after) {
            let was = old.get(&(layer, plane));
            let Some(&(from, old_size, old_grid)) = was else { self.push(Change { layer, plane, from: 0, to, rect: None }); continue };
            if from == to { continue; }
            let joined = self.changes.iter().rev().find(|c| c.layer == layer && c.plane == plane
                && ((c.from == from && c.to == to) || (c.from == to && c.to == from)));
            let rect = if old_size == size && old_grid == grid { joined.and_then(|c| c.rect) } else { None };
            self.push(Change { layer, plane, from, to, rect });
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
