use crate::{Document, Raster};
use std::collections::{HashMap, HashSet};

/// The most entries history keeps, undo and redo together (DocumentHistory.swift:28).
pub const HISTORY_ENTRY_LIMIT: usize = 100;
/// The most bytes of pixels that only history holds (DocumentHistory.swift:28): a layer or mask
/// buffer the current document does not share, counted once however many entries reach it.
pub const HISTORY_BYTE_LIMIT: usize = 256 * 1024 * 1024;

/// One snapshot: the document as it was, the id of the edit it belongs to (kept as the entry moves
/// between undo and redo, never reused), and the state token the document had then.
#[derive(Debug)]
struct Entry { id: u64, doc: Document, state: u64 }

/// A pixel buffer some entry reaches: its size, how many entries reach it, and (for layer pixels)
/// a handle to drop its halving with.
#[derive(Debug)]
struct Held { bytes: usize, entries: usize, raster: Option<Raster> }

/// Whole-document snapshots. Rasters and the selection's outline are shared, so a snapshot costs only
/// the layer metadata plus whatever pixels no other snapshot or the document itself holds. Bounded as
/// the Mac bounds it (DocumentHistory.swift): after every push, undo and redo, the oldest undo entry
/// and then the farthest redo entry go while there are more than `entry_limit` entries or the pixels
/// only history holds exceed `byte_limit` -- even the entry just made, so an edit to a layer larger
/// than the limit cannot be undone, as on the Mac.
#[derive(Debug)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The token of the current document's state; `saved` is the token when it was saved (the Mac's
    /// revision UUIDs, DocumentHistory.swift:20-21): trimming the front never changes either.
    state: u64,
    saved: Option<u64>,
    /// Issues entry ids and state tokens alike; never reused.
    next: u64,
    entry_limit: usize,
    byte_limit: usize,
    /// Every buffer the entries reach, kept up to date as entries come and go, so a trim costs the
    /// distinct buffers and the entries it drops, not every entry's every layer.
    held: HashMap<usize, Held>,
}

impl Default for History {
    fn default() -> History { History::with_limits(HISTORY_ENTRY_LIMIT, HISTORY_BYTE_LIMIT) }
}

/// Each pixel buffer a document holds (layer pixels and masks), once: its identity, its size and,
/// for layer pixels, the raster.
fn buffers(doc: &Document) -> Vec<(usize, usize, Option<Raster>)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for l in &doc.layers {
        if let Some(r) = &l.pixels { if seen.insert(r.buffer_id()) { out.push((r.buffer_id(), r.bytes().len(), Some(r.clone()))); } }
        if let Some(m) = &l.mask { if seen.insert(m.pixels.buffer_id()) { out.push((m.pixels.buffer_id(), m.pixels.bytes().len(), None)); } }
    }
    out
}
fn live(doc: &Document) -> HashSet<usize> { buffers(doc).into_iter().map(|(id, _, _)| id).collect() }

impl History {
    pub fn with_limits(entry_limit: usize, byte_limit: usize) -> History {
        History { undo: Vec::new(), redo: Vec::new(), state: 1, saved: Some(1), next: 2, entry_limit, byte_limit, held: HashMap::new() }
    }
    fn issue(&mut self) -> u64 { let v = self.next; self.next += 1; v }
    fn hold(&mut self, doc: &Document) {
        for (id, bytes, raster) in buffers(doc) { self.held.entry(id).or_insert(Held { bytes, entries: 0, raster }).entries += 1; }
    }
    /// Lets go of `doc`'s buffers; returns those no entry reaches any more.
    fn release(&mut self, doc: &Document) -> Vec<(usize, usize)> {
        let mut gone = Vec::new();
        for (id, _, _) in buffers(doc) {
            if let Some(h) = self.held.get_mut(&id) {
                h.entries -= 1;
                if h.entries == 0 { gone.push((id, h.bytes)); self.held.remove(&id); }
            }
        }
        gone
    }

    /// Records `before` as the state an edit left; the future is discarded. Call `trim` with the new
    /// current document afterwards.
    pub fn push(&mut self, before: Document) {
        for e in std::mem::take(&mut self.redo) { self.release(&e.doc); }
        self.hold(&before);
        let id = self.issue();
        self.undo.push(Entry { id, doc: before, state: self.state });
        self.state = self.issue();
    }
    pub fn undo(&mut self, current: &Document) -> Option<Document> {
        let entry = self.undo.pop()?;
        self.release(&entry.doc);
        self.hold(current);
        self.redo.push(Entry { id: entry.id, doc: current.clone(), state: self.state });
        self.state = entry.state;
        Some(entry.doc)
    }
    pub fn redo(&mut self, current: &Document) -> Option<Document> {
        let entry = self.redo.pop()?;
        self.release(&entry.doc);
        self.hold(current);
        self.undo.push(Entry { id: entry.id, doc: current.clone(), state: self.state });
        self.state = entry.state;
        Some(entry.doc)
    }
    /// Returns to the state before the last entry and drops that entry entirely, offering no
    /// redo. This is how macOS closes a transaction the user cancelled: `cancelTransform`
    /// removes the Alt-drag copy and calls `endEdit`, whose `before.document != document` guard
    /// then records nothing, so neither undo nor redo gains an entry (EditorSession.swift).
    pub fn revert(&mut self) -> Option<Document> {
        let entry = self.undo.pop()?;
        self.release(&entry.doc);
        self.state = entry.state;
        Some(entry.doc)
    }
    /// How many entries deep the undo stack is.
    pub fn depth(&self) -> usize { self.undo.len() }
    /// The id of the entry on top of the undo stack: the edit an undo would take back. It follows the
    /// entry through undo and redo and is never reused, so a gesture that recorded it can tell whether
    /// its own entry is still on top even after trimming dropped older ones.
    pub fn undo_entry_id(&self) -> Option<u64> { self.undo.last().map(|e| e.id) }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn mark_saved(&mut self) { self.saved = Some(self.state); }
    pub fn is_modified(&self) -> bool { self.saved != Some(self.state) }
    /// Content exists that is not on disk (a fresh import): modified until the first save.
    pub fn mark_never_saved(&mut self) { self.saved = None; }
    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.held.clear(); self.saved = Some(self.state); }

    /// Bytes of pixels that only history holds: every layer and mask buffer an entry reaches and
    /// `current` does not, each counted once (`retainedBytes`, DocumentHistory.swift:88-110).
    pub fn retained_bytes(&self, current: &Document) -> usize {
        let live = live(current);
        self.held.iter().filter(|(id, _)| !live.contains(id)).map(|(_, h)| h.bytes).sum()
    }

    /// Drops the oldest undo entry, then the farthest redo entry, while there are more than
    /// `entry_limit` entries or the pixels only history holds exceed `byte_limit`
    /// (DocumentHistory.swift:112-118). The halvings of the buffers only history keeps are let go:
    /// they are not counted, and a buffer that comes back through undo halves again.
    pub fn trim(&mut self, current: &Document) {
        let live = live(current);
        let mut total: usize = self.held.iter().filter(|(id, _)| !live.contains(id)).map(|(_, h)| h.bytes).sum();
        while self.undo.len() + self.redo.len() > self.entry_limit || total > self.byte_limit {
            let gone = if !self.undo.is_empty() { self.undo.remove(0) } else if !self.redo.is_empty() { self.redo.remove(0) } else { break };
            for (id, bytes) in self.release(&gone.doc) { if !live.contains(&id) { total -= bytes; } }
        }
        for (id, h) in &self.held {
            if let Some(r) = &h.raster { if !live.contains(id) { r.forget_halvings(); } }
        }
    }
}
