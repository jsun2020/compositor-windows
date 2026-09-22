use compositor_engine::*;
use uuid::Uuid;

fn record(id: Uuid) -> LayerRecord {
    LayerRecord::new(id, "Layer 1", LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 100.0, height: 80.0 }), None)
}

fn manifest() -> Manifest {
    let id = Uuid::new_v4();
    Manifest::new(Uuid::new_v4(), 100, 80, Some(id), vec![record(id)])
}

#[test]
fn round_trips_json_with_swift_shapes() {
    let m = manifest();
    let json = m.to_json_pretty().unwrap();
    assert!(json.contains("\"format\": \"com.compositor.project\""));
    assert!(json.contains("\"version\": 7"));
    assert!(json.contains("\"colorSpace\": \"sRGB\""));
    assert!(!json.contains("\"parentID\""), "absent optionals are omitted");
    let upper = ids::upper_string(&m.layers[0].id);
    assert!(json.contains(&upper) && upper == upper.to_uppercase());
    let back = Manifest::parse(&json).unwrap();
    assert_eq!(back, m);
}

#[test]
fn future_version_is_rejected_with_its_number() {
    let mut m = manifest();
    m.version = 42;
    let json = serde_json::to_string(&m).unwrap();
    assert_eq!(Manifest::parse(&json), Err(ProjectError::Version(42)));
}

#[test]
fn corrupt_and_wrong_format_are_invalid() {
    assert_eq!(Manifest::parse("not json"), Err(ProjectError::Invalid));
    let mut m = manifest();
    m.format = "com.other".into();
    assert_eq!(Manifest::parse(&serde_json::to_string(&m).unwrap()), Err(ProjectError::Invalid));
}

#[test]
fn image_file_must_be_the_layer_uuid_png() {
    let mut m = manifest();
    m.layers[0].image_file = Some("../../outside.png".into());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
    m.layers[0].image_file = Some(format!("{}.png", ids::upper_string(&m.layers[0].id)));
    assert_eq!(m.validate(), Ok(()));
}

#[test]
fn limits_are_too_large() {
    let mut m = manifest();
    m.width = 30_001;
    assert_eq!(m.validate(), Err(ProjectError::TooLarge));
    m.width = 100;
    m.resolution = Some(9601.0);
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn version_gates_optional_features() {
    let mut m = manifest();
    m.version = 2;
    m.layers[0].opacity = Some(0.5);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "opacity needs version 3");
    m.version = 3;
    assert_eq!(m.validate(), Ok(()));
    m.layers[0].mask_file = Some(format!("{}.mask.png", ids::upper_string(&m.layers[0].id)));
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "layer masks need version 4");
    m.version = 4;
    assert_eq!(m.validate(), Ok(()));
    m.layers[0].mask_file = Some("wrong.png".into());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn hierarchy_rules() {
    let mut m = manifest();
    let group = Uuid::new_v4();
    let mut g = record(group);
    g.is_group = Some(true);
    m.layers[0].parent_id = Some(group);
    m.layers.push(g);
    m.version = 2;
    assert_eq!(m.validate(), Ok(()));
    m.version = 1;
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "groups need version 2");
    m.version = 2;
    m.layers[1].parent_id = Some(m.layers[0].id);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "parent must be a group");
    m.layers[1].parent_id = Some(group);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "cycle");
    m.layers[1].parent_id = None;
    m.layers[1].image_file = Some(format!("{}.png", ids::upper_string(&group)));
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "groups carry no image");
}

#[test]
fn clipping_mask_rules() {
    let mut m = manifest();
    let other = Uuid::new_v4();
    m.layers.push(record(other));
    m.layers[0].mask_source_id = Some(other);
    m.version = 4;
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "needs version 5");
    m.version = 5;
    assert_eq!(m.validate(), Ok(()));
    m.layers[1].mask_source_id = Some(m.layers[0].id);
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "cycle");
    m.layers[1].mask_source_id = None;
    m.layers[0].mask_source_id = Some(Uuid::new_v4());
    assert_eq!(m.validate(), Err(ProjectError::Invalid), "missing source");
}

#[test]
fn active_layer_must_exist_and_ids_unique() {
    let mut m = manifest();
    m.active_layer_id = Some(Uuid::new_v4());
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
    m.active_layer_id = None;
    let dup = m.layers[0].clone();
    m.layers.push(dup);
    assert_eq!(m.validate(), Err(ProjectError::Invalid));
}

#[test]
fn unknown_later_phase_fields_survive() {
    let mut m = manifest();
    m.layers[0].shape = Some(serde_json::json!({"kind": "Rectangle", "corner": 0}));
    let json = m.to_json_pretty().unwrap();
    let back = Manifest::parse(&json).unwrap();
    assert_eq!(back.layers[0].shape, m.layers[0].shape);
}
