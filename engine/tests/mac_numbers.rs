use compositor_engine::*;
use serde_json::Value;

#[test]
fn a_mac_project_re_saves_its_transforms_and_resolution_as_the_mac_wrote_them() {
    // Mac-test-for-windows.comp (R 6) writes origin [-479, 0], size [1920, 1080], rotation 0 and
    // resolution 72 as whole numbers. serde_json's Value keeps integers and floats apart, so 72 and
    // 72.0 compare unequal: this is Swift's JSONEncoder output, not just the same number.
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mac-1.2.6/Mac-test-for-windows.comp");
    let manifest_json = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    let name = "C739274B-CF54-4B5A-AB7C-EFA6A76F1746.png".to_string();
    let bytes = std::fs::read(format!("{dir}/images/{name}")).unwrap();
    let mac: Value = serde_json::from_str(&manifest_json).unwrap();
    let doc = open_package(&Package { manifest_json, images: vec![(name, bytes)] }).unwrap();
    let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    assert_eq!(saved["resolution"], mac["resolution"]);
    for i in 0..2 { assert_eq!(saved["layers"][i]["transform"], mac["layers"][i]["transform"], "layer {i}"); }
}

#[test]
fn fractions_keep_their_fraction() {
    let t = LayerTransform { origin: Point { x: 120.0, y: 0.5 }, size: Size { width: 60.0, height: 40.25 },
        rotation: 15.5, flip_x: false, flip_y: true, sampling: Sampling::Smooth };
    let json = serde_json::to_string(&t).unwrap();
    assert!(json.contains("\"origin\":[120,0.5]") && json.contains("\"size\":[60,40.25]") && json.contains("\"rotation\":15.5"), "{json}");
    assert_eq!(serde_json::from_str::<LayerTransform>(&json).unwrap(), t, "reads back the same");
}

#[test]
fn flipping_an_unrotated_layer_leaves_its_rotation_at_zero_not_minus_zero() {
    let t = LayerTransform::axis_aligned(Point { x: 3.0, y: 4.0 }, Size { width: 10.0, height: 6.0 });
    assert!(!t.mirrored(true, 20.0).rotation.is_sign_negative());
    assert_eq!(LayerTransform { rotation: 30.0, ..t }.mirrored(false, 20.0).rotation, -30.0, "a real rotation still mirrors");
}

#[test]
fn a_layer_at_opacity_0_saves_it_as_the_mac_writes_it() {
    let mut doc = Document::new(3, 2);
    let mut clear = Layer::with_pixels("Clear", Raster::from_premultiplied(3, 2, [40u8, 20, 10, 255].repeat(6)), Point { x: 0.0, y: 0.0 });
    clear.opacity = 0.0;
    let mut half = Layer::with_pixels("Half", Raster::from_premultiplied(3, 2, [40u8, 20, 10, 255].repeat(6)), Point { x: 0.0, y: 0.0 });
    half.opacity = 0.25;
    doc.layers = vec![clear, half];
    let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    // Value keeps 0 and 0.0 apart: Swift's JSONEncoder writes the whole number 0.
    assert_eq!(saved["layers"][0]["opacity"], serde_json::json!(0));
    assert_eq!(saved["layers"][1]["opacity"], serde_json::json!(0.25), "a fraction keeps its fraction");
    assert_eq!(open_package(&save_package(&doc).unwrap()).unwrap().layers[0].opacity, 0.0, "and reads back");
}