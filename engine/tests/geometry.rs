use compositor_engine::*;

fn transform() -> LayerTransform {
    LayerTransform { origin: Point { x: 10.0, y: 20.0 }, size: Size { width: 100.0, height: 50.0 },
        rotation: 0.0, flip_x: false, flip_y: false, sampling: Sampling::High }
}

#[test]
fn serializes_like_swift() {
    let json = serde_json::to_value(transform()).unwrap();
    assert_eq!(json["origin"], serde_json::json!([10, 20]));
    assert_eq!(json["size"], serde_json::json!([100, 50]));
    assert_eq!(json["sampling"], "High quality");
    assert_eq!(json["flipX"], false);
    let back: LayerTransform = serde_json::from_value(json).unwrap();
    assert_eq!(back, transform());
}

#[test]
fn deserializes_defaults_for_missing_fields() {
    let t: LayerTransform = serde_json::from_str(r#"{"origin":[1,2],"size":[3,4]}"#).unwrap();
    assert_eq!(t.rotation, 0.0);
    assert_eq!(t.sampling, Sampling::High);
    assert!(!t.flip_x && !t.flip_y);
}

#[test]
fn point_rotates_clockwise_around_center() {
    let mut t = transform();
    t.rotation = 90.0;
    let c = t.center();
    assert_eq!(c, Point { x: 60.0, y: 45.0 });
    // The top-left unit corner (-50,-25 from center) turns to (25,-50): up and right of center.
    let p = t.point(Point { x: 0.0, y: 0.0 });
    assert!((p.x - 85.0).abs() < 1e-9 && (p.y - -5.0).abs() < 1e-9);
}

#[test]
fn validity_limits() {
    let mut t = transform();
    assert!(t.is_valid());
    t.size.width = 0.5;
    assert!(!t.is_valid());
    t.size.width = 300_001.0;
    assert!(!t.is_valid());
    t.size.width = 10.0;
    t.origin.x = 1_000_001.0;
    assert!(!t.is_valid());
    t.origin.x = f64::NAN;
    assert!(!t.is_valid());
}

#[test]
fn mirrored_flips_across_axis() {
    let mut t = transform();
    t.rotation = 30.0;
    let m = t.mirrored(true, 200.0);
    assert!(m.flip_x && !m.flip_y);
    assert_eq!(m.rotation, -30.0);
    // Center x 60 crosses to 340; origin = 340 - 50.
    assert!((m.origin.x - 290.0).abs() < 1e-9);
    assert_eq!(m.origin.y, 20.0);
}

#[test]
fn pixel_to_document_and_inverse_round_trip() {
    let mut t = transform();
    t.rotation = 38.0;
    t.flip_x = true;
    let m = t.pixel_to_document(64, 32);
    let inv = m.invert().unwrap();
    let doc = m.apply(Point { x: 5.0, y: 7.0 });
    let back = inv.apply(doc);
    assert!((back.x - 5.0).abs() < 1e-9 && (back.y - 7.0).abs() < 1e-9);
    // Pixel center of the image maps to the transform center.
    let c = m.apply(Point { x: 32.0, y: 16.0 });
    assert!((c.x - 60.0).abs() < 1e-9 && (c.y - 45.0).abs() < 1e-9);
}
