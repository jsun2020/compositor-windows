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
    e.close_document(id);
    assert_eq!(e.document_ids(), vec![reopened]);
}

#[test]
fn command_json_shape() {
    let c: Command = serde_json::from_str(r#"{"type":"RenameLayer","id":"E621E1F8-C36C-495A-93FC-0C247A3E6E5F","name":"X"}"#).unwrap();
    assert!(matches!(c, Command::RenameLayer { .. }));
    let s = serde_json::to_string(&Command::AddBlankLayer).unwrap();
    assert_eq!(s, r#"{"type":"AddBlankLayer"}"#);
}
