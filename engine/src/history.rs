use crate::Document;

/// Whole-document snapshots. Rasters are shared, so a snapshot costs only the layer metadata.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Document>,
    redo: Vec<Document>,
    saved_depth: usize,
}

impl History {
    pub fn push(&mut self, before: Document) {
        // The saved state can no longer be reached by redo once the future is discarded.
        // Checked against the pre-push length: the saved depth is the undo length at the
        // moment of saving, so anything deeper than the state we are about to push from
        // (i.e. anything only reachable via the redo stack we are about to clear) is lost.
        if self.saved_depth > self.undo.len() { self.saved_depth = usize::MAX; }
        self.undo.push(before);
        self.redo.clear();
    }
    pub fn undo(&mut self, current: &Document) -> Option<Document> {
        let before = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(before)
    }
    pub fn redo(&mut self, current: &Document) -> Option<Document> {
        let after = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(after)
    }
    /// Returns to the state before the last entry and drops that entry entirely, offering no
    /// redo. This is how macOS closes a transaction the user cancelled: `cancelTransform`
    /// removes the Alt-drag copy and calls `endEdit`, whose `before.document != document` guard
    /// then records nothing, so neither undo nor redo gains an entry (EditorSession.swift).
    /// The popped entry is the one `push` added for the cancelled gesture, so `saved_depth` is
    /// back where it was before that push and needs no adjustment.
    pub fn revert(&mut self) -> Option<Document> { self.undo.pop() }
    /// How many entries deep the undo stack is. A caller that recorded this before starting a
    /// gesture can tell whether the entry on top is still the one its own command pushed.
    pub fn depth(&self) -> usize { self.undo.len() }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn mark_saved(&mut self) { self.saved_depth = self.undo.len(); }
    pub fn is_modified(&self) -> bool { self.saved_depth != self.undo.len() }
    /// Content exists that is not on disk (a fresh import): modified until the first save.
    pub fn mark_never_saved(&mut self) { self.saved_depth = usize::MAX; }
    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.saved_depth = 0; }
}
