//! The history cap (Phase 4b-1): the Mac's 100 entries and 256 MiB of pixels only history holds
//! (DocumentHistory.swift), a state token for "modified", and an entry id a gesture can check.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn layer_id(e: &Engine, id: Uuid) -> Uuid { e.state(id).unwrap().layers[0].id }
fn name(e: &Engine, id: Uuid) -> String { e.state(id).unwrap().layers[0].name.clone() }

/// A `width` x `height` document with one opaque layer (Canvas Size's fill: one undoable step).
fn filled(e: &mut Engine, width: u32, height: u32) -> Uuid {
    let id = e.new_document(10, 10, false).unwrap();
    run(e, id, Command::CanvasSize { width, height, anchor: 4, fill: Some([0.2, 0.4, 0.6]) });
    id
}

/// A document of `width` x `height` with one layer of distinct pixels, for driving `History` directly.
fn doc_with_pixels(width: u32, height: u32, value: u8) -> Document {
    let mut doc = Document::new(width, height);
    doc.layers = vec![Layer::with_pixels("Layer 1", Raster::from_premultiplied(width, height, vec![value; (width * height * 4) as usize]), Point { x: 0.0, y: 0.0 })];
    doc
}

/// One edit on `doc` recorded the way `Engine::edit` records it: the state before goes to history,
/// then history trims against the new state.
fn record(history: &mut History, doc: &mut Document, change: impl FnOnce(&mut Document)) {
    let before = doc.clone();
    change(doc);
    history.push(before);
    history.trim(doc);
}

#[test]
fn history_bounds_entries_and_unique_retained_pixels() {
    // HistoryTests.historyBoundsEntriesAndUniqueRetainedPixels (HistoryTests.swift:139-158): at most
    // two entries and no retained bytes. Three renames keep two entries and share the one image;
    // deleting the layer leaves an entry that alone holds it, so even that entry goes.
    let mut history = History::with_limits(2, 0);
    let mut doc = doc_with_pixels(64, 32, 90);
    for name in ["A", "B", "C"] { record(&mut history, &mut doc, |d| d.layers[0].name = name.into()); }
    assert_eq!(history.depth(), 2);
    assert_eq!(history.retained_bytes(&doc), 0);
    record(&mut history, &mut doc, |d| d.layers.clear());
    assert_eq!(history.depth(), 0);
    assert_eq!(history.retained_bytes(&doc), 0);
}

#[test]
fn a_hundred_and_one_renames_keep_the_last_hundred() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    for n in 0..=100 { run(&mut e, id, Command::RenameLayer { id: layer, name: format!("n{n}") }); }
    assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT);
    for _ in 0..HISTORY_ENTRY_LIMIT { e.undo(id).unwrap(); }
    // The oldest entry (back to "Layer 1") went; the first rename is as far back as undo reaches.
    assert_eq!(name(&e, id), "n0");
    assert!(!e.state(id).unwrap().can_undo);
    assert_eq!(e.state(id).unwrap().undo_depth, 0);
}

#[test]
fn a_raster_several_entries_share_is_counted_once() {
    let mut history = History::with_limits(100, usize::MAX);
    let mut doc = doc_with_pixels(40, 10, 1);
    let bytes = 40 * 10 * 4;
    let replace = |d: &mut Document, v: u8| d.layers[0].set_pixels(Some(Raster::from_premultiplied(40, 10, vec![v; bytes])));
    record(&mut history, &mut doc, |d| replace(d, 2));
    record(&mut history, &mut doc, |d| d.layers[0].name = "x".into());
    record(&mut history, &mut doc, |d| d.layers[0].name = "y".into());
    // Only the first raster is history's alone: the second is the document's too.
    assert_eq!(history.retained_bytes(&doc), bytes);
    record(&mut history, &mut doc, |d| replace(d, 3));
    // The second raster now sits in three entries and counts once: two rasters, not four.
    assert_eq!(history.retained_bytes(&doc), 2 * bytes);
}

#[test]
fn an_edit_past_the_byte_limit_applies_but_cannot_be_undone() {
    // A 100 x 100 layer holds 40,000 bytes. Under a 30,000-byte cap the Invert applies and even its
    // own entry goes, as a layer over 256 MiB goes on the Mac; under 50,000 it stays undoable.
    for (limit, undoable) in [(30_000, false), (50_000, true)] {
        let mut e = Engine::with_history_limits(100, limit);
        let id = filled(&mut e, 100, 100);
        let layer = layer_id(&e, id);
        let before = e.document(id).unwrap().layers[0].pixels.as_ref().unwrap().pixel(5, 5);
        run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
        let after = e.document(id).unwrap().layers[0].pixels.as_ref().unwrap().pixel(5, 5);
        assert_eq!(after, [255 - before[0], 255 - before[1], 255 - before[2], 255], "limit {limit}: the edit applied");
        let state = e.state(id).unwrap();
        assert_eq!(state.can_undo, undoable, "limit {limit}");
        assert_eq!(state.undo_depth, if undoable { 2 } else { 0 }, "limit {limit}");
    }
}

#[test]
fn trimming_the_front_leaves_the_saved_state_findable() {
    // Saved after the first rename, then three more under a three-entry cap: one entry is trimmed
    // from the front, and three undos still land exactly on the saved state.
    let mut e = Engine::with_history_limits(3, usize::MAX);
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "saved".into() });
    e.mark_saved(id, None);
    for n in ["a", "b", "c"] { run(&mut e, id, Command::RenameLayer { id: layer, name: n.into() }); }
    assert_eq!(e.state(id).unwrap().undo_depth, 3);
    assert!(e.state(id).unwrap().is_modified);
    for _ in 0..3 { e.undo(id).unwrap(); }
    assert_eq!(name(&e, id), "saved");
    assert!(!e.state(id).unwrap().is_modified, "back at the saved state");
    e.redo(id).unwrap();
    assert!(e.state(id).unwrap().is_modified);
}

#[test]
fn the_undo_entry_id_follows_its_entry_and_survives_the_cap() {
    let mut e = Engine::with_history_limits(3, usize::MAX);
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    assert_eq!(e.state(id).unwrap().undo_entry_id, None);
    for n in ["a", "b", "c"] { run(&mut e, id, Command::RenameLayer { id: layer, name: n.into() }); }
    let third = e.state(id).unwrap().undo_entry_id.unwrap();
    // At the cap a duplicate pushes one entry and trims one: the depth does not move, the id does.
    run(&mut e, id, Command::DuplicateLayer { id: layer });
    let state = e.state(id).unwrap();
    assert_eq!(state.undo_depth, 3);
    let duplicate = state.undo_entry_id.unwrap();
    assert_ne!(duplicate, third);
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(third));
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(duplicate), "the same edit keeps its id through undo and redo");
    e.revert(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(third));
    assert_eq!(e.state(id).unwrap().layers.len(), 1, "the copy is gone");
}

#[test]
fn undo_trims_redo_entries_whose_only_raster_the_live_document_no_longer_holds() {
    // Fix round 1, finding 1(a): the byte-limit trim after undo/redo was untested. A rename does
    // not touch pixels, so the entry it pushes shares the layer's raster with the live document
    // and costs nothing; only once TWO undos land back before the layer existed at all does the
    // raster become retained-only, over the limit, and both redo entries that reach it (the
    // renamed state and the just-filled state) go in the trim that follows the second undo.
    let bytes = 100usize * 100 * 4; // one 100 x 100 raster, computed rather than pasted
    let mut e = Engine::with_history_limits(100, bytes - 10_000);
    let id = filled(&mut e, 100, 100);
    let layer = layer_id(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "x".into() });
    e.undo(id).unwrap();
    e.undo(id).unwrap();
    assert!(!e.state(id).unwrap().can_redo, "both redo entries reach the raster the live document (back at its blank start) no longer holds");
}

#[test]
fn undo_can_run_out_before_reaching_a_saved_state_the_front_trim_dropped() {
    // Fix round 1, finding 1(b): no test covered a saved state whose entry was trimmed away.
    // Unlike trimming_the_front_leaves_the_saved_state_findable (where the saved entry survives
    // the trim), the fourth rename here trims the very entry that carries the saved token, so the
    // saved state can never be reached again -- the document reports modified even once undo runs
    // out entirely, rather than (wrongly) reporting "back at the saved state" by coincidence of
    // depth.
    let mut e = Engine::with_history_limits(3, usize::MAX);
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    e.mark_saved(id, None);
    for n in ["a", "b", "c", "d"] { run(&mut e, id, Command::RenameLayer { id: layer, name: n.into() }); }
    assert_eq!(e.state(id).unwrap().undo_depth, 3, "the fourth rename trimmed the entry that held the saved token");
    for _ in 0..3 { e.undo(id).unwrap(); }
    assert!(!e.state(id).unwrap().can_undo);
    assert!(e.state(id).unwrap().is_modified, "undo ran out before reaching the saved state: its entry was trimmed away");
}

#[test]
fn history_lets_go_of_the_halvings_of_rasters_only_it_holds() {
    let mut e = Engine::new();
    let id = filled(&mut e, 64, 48);
    let layer = layer_id(&e, id);
    let old = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let half = old.halved();
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    // The old raster now lives only in history: its halving was dropped, so halving it again makes
    // a new raster rather than handing back the kept one.
    assert!(!old.halved().same_pixels(&half));
    // The document's own raster keeps its halving.
    let current = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let kept = current.halved();
    run(&mut e, id, Command::RenameLayer { id: layer, name: "x".into() });
    assert!(current.halved().same_pixels(&kept));
}
