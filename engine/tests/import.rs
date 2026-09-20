mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn first_import_creates_a_document_of_the_image_size() {
    let mut e = Engine::new();
    let id = e.import_image(None, &red_left_png(), "photo", None).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (64, 32));
    assert_eq!(s.layers.len(), 1);
    assert_eq!(s.layers[0].name, "photo");
    assert_eq!(s.layers[0].transform.origin, Point { x: 0.0, y: 0.0 });
    assert_eq!(s.active_layer_id, Some(s.layers[0].id));
    assert!(s.is_modified, "an imported document has unsaved content");
    assert!(!s.can_undo, "a fresh import has nothing to undo");
}

#[test]
fn import_into_document_centers_and_is_undoable() {
    let mut e = Engine::new();
    let id = e.new_document(128, 128, false).unwrap();
    e.import_image(Some(id), &red_left_png(), "photo", None).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!(s.layers[0].transform.origin, Point { x: 32.0, y: 48.0 });
    e.import_image(Some(id), &red_left_png(), "photo", Some(Point { x: 300.0, y: 250.0 })).unwrap();
    assert_eq!(e.state(id).unwrap().layers[1].transform.origin, Point { x: 268.0, y: 234.0 });
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
}

#[test]
fn unsupported_and_over_budget_imports_fail_without_changing_the_document() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let err = e.import_image(Some(id), b"not an image", "x", None).unwrap_err();
    assert_eq!(err, CommandError::Import(ImportError::Unreadable));
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
    assert!(!e.state(id).unwrap().is_modified);
}
