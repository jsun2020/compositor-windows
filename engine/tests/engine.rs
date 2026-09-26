mod fixtures;
use compositor_engine::*;

#[test]
fn new_document_with_empty_layer_and_state() {
    let mut e = Engine::new();
    let id = e.new_document(1920, 1080, true).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (1920, 1080));
    assert_eq!(s.layers.len(), 1);
    assert_eq!(s.layers[0].name, "Layer 1");
    assert_eq!(s.active_layer_id, Some(s.layers[0].id));
    assert!(!s.can_undo && !s.is_modified);
    assert_eq!(e.new_document(0, 10, false).unwrap_err(), CommandError::Argument("width and height must be 1 to 30000".into()));
}

#[test]
fn commands_are_undoable_and_track_modification() {
    let mut e = Engine::new();
    let id = e.new_document(100, 80, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    let dirty = e.execute(id, Command::RenameLayer { id: layer, name: "Edited".into() }).unwrap();
    assert!(dirty.structure);
    assert_eq!(e.state(id).unwrap().layers[0].name, "Edited");
    assert!(e.state(id).unwrap().is_modified);
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Layer 1");
    assert!(!e.state(id).unwrap().is_modified);
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Edited");
    e.mark_saved(id, Some("C:/x/Test.comp".into()));
    assert!(!e.state(id).unwrap().is_modified);
    assert_eq!(e.state(id).unwrap().path.as_deref(), Some("C:/x/Test.comp"));
}

#[test]
fn edit_after_undo_past_saved_point_is_modified() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    e.execute(id, Command::RenameLayer { id: layer, name: "A".into() }).unwrap();
    e.execute(id, Command::RenameLayer { id: layer, name: "B".into() }).unwrap();
    e.execute(id, Command::RenameLayer { id: layer, name: "C".into() }).unwrap();
    e.mark_saved(id, None);
    e.undo(id).unwrap();
    assert!(e.state(id).unwrap().is_modified);
    e.execute(id, Command::RenameLayer { id: layer, name: "D".into() }).unwrap();
    assert!(e.state(id).unwrap().is_modified);
    assert!(!e.state(id).unwrap().can_redo);
}

#[test]
fn undo_back_to_saved_point_is_not_modified() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    e.execute(id, Command::RenameLayer { id: layer, name: "A".into() }).unwrap();
    e.mark_saved(id, None);
    e.execute(id, Command::RenameLayer { id: layer, name: "B".into() }).unwrap();
    assert!(e.state(id).unwrap().is_modified);
    e.undo(id).unwrap();
    assert!(!e.state(id).unwrap().is_modified);
    e.redo(id).unwrap();
    assert!(e.state(id).unwrap().is_modified);
}

#[test]
fn blank_layers_number_from_first_free_name() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let names: Vec<_> = e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, ["Layer 1", "Layer 2", "Layer 3"]);
    let second = e.state(id).unwrap().layers[1].id;
    e.execute(id, Command::DeleteLayer { id: second }).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let names: Vec<_> = e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect();
    assert_eq!(names, ["Layer 1", "Layer 3", "Layer 2"]);
    // New layer goes just above the active layer.
    let active = e.state(id).unwrap().active_layer_id.unwrap();
    assert_eq!(active, e.state(id).unwrap().layers[2].id);
}

#[test]
fn visibility_delete_and_active_layer() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let l1 = e.state(id).unwrap().layers[0].id;
    e.execute(id, Command::SetLayerVisible { id: l1, visible: false }).unwrap();
    assert!(!e.state(id).unwrap().layers[0].visible);
    e.execute(id, Command::DeleteLayer { id: l1 }).unwrap();
    assert!(e.state(id).unwrap().layers.is_empty());
    assert_eq!(e.state(id).unwrap().active_layer_id, None);
    assert_eq!(e.execute(id, Command::DeleteLayer { id: l1 }).unwrap_err(), CommandError::NoLayer);
    assert_eq!(e.execute(uuid::Uuid::new_v4(), Command::AddBlankLayer).unwrap_err(), CommandError::NoDocument);
}

#[test]
fn open_and_save_round_trip_through_engine_resets_history() {
    let mut e = Engine::new();
    let id = e.new_document(64, 32, true).unwrap();
    e.execute(id, Command::AddBlankLayer).unwrap();
    let pkg = e.save_package(id).unwrap();
    let reopened = e.open_package(&pkg, Some("C:/p/A.comp".into())).unwrap();
    let s = e.state(reopened).unwrap();
    assert_eq!(s.layers.len(), 2);
    assert!(!s.can_undo && !s.is_modified);
    assert_eq!(e.document_ids().len(), 2);
    assert_eq!(e.document(reopened).unwrap().id, e.document(id).unwrap().id, "manifest document id is preserved on reopen");
    e.close_document(id);
    assert_eq!(e.document_ids(), vec![reopened]);
}

/// Undo restores the revisions its content had, so a different edit after an undo must never be
/// given one the undone content had: the GPU keys its textures by revision and would keep showing
/// the undone pixels or mask (final review I1).
#[test]
fn an_edit_after_an_undo_never_reuses_a_pixels_or_mask_revision() {
    let mut e = Engine::new();
    let id = e.new_document(6, 4, false).unwrap();
    let png = encode_png(&Raster::from_premultiplied(6, 4, [200u8, 40, 10, 255].repeat(24)), 72.0).unwrap();
    e.import_image(Some(id), &png, "L", None).unwrap();
    let layer = e.state(id).unwrap().active_layer_id.unwrap();
    let revisions = |e: &Engine| { let l = &e.state(id).unwrap().layers[0]; (l.pixels_revision, l.mask_revision) };

    let mut pixels_seen = vec![revisions(&e).0];
    e.execute(id, Command::InvertPixels { id: layer, mask: false }).unwrap();
    pixels_seen.push(revisions(&e).0);
    e.undo(id).unwrap();
    assert_eq!(revisions(&e).0, pixels_seen[0], "undo brings back the revision its pixels had");
    e.execute(id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 1.0 } }).unwrap();
    let now = revisions(&e).0;
    assert!(!pixels_seen.contains(&now), "other pixels, a revision never used before: {now} after {pixels_seen:?}");

    e.execute(id, Command::AddMask { id: layer, revealing: false }).unwrap();
    let mut masks_seen = vec![revisions(&e).1];
    e.execute(id, Command::FillMask { id: layer, white: true }).unwrap();
    masks_seen.push(revisions(&e).1);
    e.undo(id).unwrap();
    assert_eq!(revisions(&e).1, masks_seen[0], "undo brings back the revision its mask had");
    e.execute(id, Command::InvertMask { id: layer }).unwrap();
    let now = revisions(&e).1;
    assert!(!masks_seen.contains(&now), "another mask, a revision never used before: {now} after {masks_seen:?}");
}

#[test]
fn command_json_shape() {
    let c: Command = serde_json::from_str(r#"{"type":"RenameLayer","id":"E621E1F8-C36C-495A-93FC-0C247A3E6E5F","name":"X"}"#).unwrap();
    assert!(matches!(c, Command::RenameLayer { .. }));
    let s = serde_json::to_string(&Command::AddBlankLayer).unwrap();
    assert_eq!(s, r#"{"type":"AddBlankLayer"}"#);
}
