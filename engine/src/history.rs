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
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn mark_saved(&mut self) { self.saved_depth = self.undo.len(); }
    pub fn is_modified(&self) -> bool { self.saved_depth != self.undo.len() }
    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.saved_depth = 0; }
}
