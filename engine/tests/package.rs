mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn round_trip_keeps_metadata_pixels_and_blank_layers() {
    let mut doc = Document::new(200, 100);
    let mut image = Layer::with_pixels("Paint & sky \u{1F33B}", red_left_raster(), Point { x: -27.5, y: 88.25 });
    image.transform.size = Size { width: 123.0, height: 47.0 };
    image.transform.rotation = 38.0;
    image.transform.flip_x = true; image.transform.flip_y = true;
    image.transform.sampling = Sampling::Nearest;
    image.visible = false;
    doc.layers.push(image);
    doc.layers.push(Layer::blank("Layer 2", doc.size()));
    doc.active_layer_id = Some(doc.layers[1].id);
    doc.resolution = 300.0;

    let pkg = save_package(&doc).unwrap();
    assert_eq!(pkg.images.len(), 1);
    assert_eq!(pkg.images[0].0, LayerRecord::image_filename(&doc.layers[0].id));
    let back = open_package(&pkg).unwrap();
    assert_eq!(back.id, doc.id);
    assert_eq!((back.width, back.height, back.resolution), (200, 100, 300.0));
    assert_eq!(back.layers.iter().map(|l| l.id).collect::<Vec<_>>(), doc.layers.iter().map(|l| l.id).collect::<Vec<_>>());
    assert_eq!(back.layers[0].name, doc.layers[0].name);
    assert_eq!(back.layers[0].transform, doc.layers[0].transform);
    assert!(!back.layers[0].visible);
    assert!(back.layers[1].pixels.is_none());
    assert_eq!(back.active_layer_id, doc.active_layer_id);
    let px = back.layers[0].pixels.as_ref().unwrap();
    assert_eq!(px.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(px.pixel(63, 0)[3], 0);
}

#[test]
fn missing_image_is_rejected() {
    let mut doc = Document::new(64, 32);
    doc.layers.push(Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 }));
    let mut pkg = save_package(&doc).unwrap();
    pkg.images.clear();
    assert_eq!(open_package(&pkg).unwrap_err(), ProjectError::MissingImage);
}

#[test]
fn corrupt_future_and_unsafe_manifests_are_rejected() {
    let mut doc = Document::new(100, 80);
    doc.layers.push(Layer::blank("Layer 1", doc.size()));
    let pkg = save_package(&doc).unwrap();
    let mut future = pkg.clone();
    future.manifest_json = pkg.manifest_json.replace("\"version\": 9", "\"version\": 42");
    assert_eq!(open_package(&future).unwrap_err(), ProjectError::Version(42));
    let mut unsafe_pkg = pkg.clone();
    unsafe_pkg.manifest_json = pkg.manifest_json.replace("\"name\": \"Layer 1\"", "\"imageFile\": \"../../outside.png\", \"name\": \"Layer 1\"");
    assert_eq!(open_package(&unsafe_pkg).unwrap_err(), ProjectError::Invalid);
    let mut corrupt = pkg.clone();
    corrupt.manifest_json = "not json".into();
    assert_eq!(open_package(&corrupt).unwrap_err(), ProjectError::Invalid);
}

#[test]
fn masks_and_later_phase_fields_survive_round_trip() {
    let mut doc = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: false, placement: None, linked: Some(true) });
    layer.extra.shape = Some(serde_json::json!({"kind": "rectangle", "red": 1.0, "green": 0.0, "blue": 0.0, "cornerRadius": 0.0}));
    layer.opacity = 0.5;
    layer.blend_mode = BlendMode::Multiply;
    doc.layers.push(layer);
    let pkg = save_package(&doc).unwrap();
    assert!(pkg.images.iter().any(|(n, _)| n.ends_with(".mask.png")));
    let back = open_package(&pkg).unwrap();
    let l = &back.layers[0];
    assert_eq!(l.mask.as_ref().unwrap().pixels.is_uniform(), Some(255));
    assert!(!l.mask.as_ref().unwrap().enabled);
    assert_eq!(l.extra.shape, doc.layers[0].extra.shape);
    assert_eq!((l.opacity, l.blend_mode), (0.5, BlendMode::Multiply));
}

#[test]
fn unsupported_save_state_fails_before_encoding() {
    let mut doc = Document::new(100, 80);
    doc.layers.push(Layer::blank("", doc.size()));
    assert_eq!(save_package(&doc).unwrap_err(), ProjectError::Invalid);
}

#[test]
fn duplicate_image_names_are_invalid() {
    let mut doc = Document::new(64, 32);
    doc.layers.push(Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 }));
    let mut pkg = save_package(&doc).unwrap();
    assert_eq!(pkg.images.len(), 1);
    let dup_name = pkg.images[0].0.clone();
    pkg.images.push((dup_name, vec![1, 2, 3]));
    assert_eq!(open_package(&pkg).unwrap_err(), ProjectError::Invalid);
}
