use compositor_engine::*;
use serde_json::{json, Value};

fn tiny_png() -> Vec<u8> {
    encode_png(&Raster::from_premultiplied(3, 2, [200u8, 40, 40, 255].repeat(6)), 72.0).unwrap()
}

/// The real project the Mac app saved (R 6): version 9, an empty layer and an image layer.
fn mac_fixture() -> Package {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mac-1.2.6/Mac-test-for-windows.comp");
    let manifest_json = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    let name = "C739274B-CF54-4B5A-AB7C-EFA6A76F1746.png".to_string();
    let bytes = std::fs::read(format!("{dir}/images/{name}")).unwrap();
    Package { manifest_json, images: vec![(name, bytes)] }
}

#[test]
fn a_project_saved_by_the_mac_app_opens() {
    let doc = open_package(&mac_fixture()).expect("the Mac 1.2.6 fixture opens");
    assert_eq!((doc.width, doc.height), (962, 1080));
    assert_eq!(doc.layers.len(), 2);
    assert!(doc.layers[0].pixels.is_none(), "Layer 1 is an empty layer");
    let image = doc.layers[1].pixels.as_ref().unwrap();
    assert_eq!((image.width, image.height), (962, 1708));
    assert_eq!(doc.active_layer_id, Some(doc.layers[1].id));
}

#[test]
fn saving_writes_version_11_like_the_mac_1_4_5() {
    let doc = open_package(&mac_fixture()).unwrap();
    let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    // Compositor for Mac 1.4.5 writes every save at 11, a version 9 project too (ProjectStore.swift:15, :21).
    assert_eq!(saved["version"], json!(11));
    // Round-trip the saved file: it must re-open.
    let again = Package { manifest_json: saved.to_string(), images: save_package(&doc).unwrap().images };
    assert_eq!(open_package(&again).unwrap().layers.len(), 2);
}

/// A v9 manifest carrying everything this build does not interpret: layer effects, live text,
/// guides, and keys from a future version at both levels. Hand-written on purpose: this test is
/// about PRESERVATION, not about the Mac's encoding (the Mac fixture covers that).
fn rich_manifest(version: u32) -> String {
    json!({
        "format": "com.compositor.project", "version": version, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 40, "height": 30,
        "activeLayerID": "0B6C6B1E-4F1B-4B4E-9E0A-222222222222",
        "guides": [
            { "axis": "vertical", "id": "0B6C6B1E-4F1B-4B4E-9E0A-333333333333", "position": 16 },
            { "axis": "horizontal", "id": "0B6C6B1E-4F1B-4B4E-9E0A-444444444444", "position": 12.5 }
        ],
        "futureDocumentKey": { "nested": [1, 2.5, "x"] },
        "layers": [{
            "id": "0B6C6B1E-4F1B-4B4E-9E0A-222222222222", "name": "Titled", "isVisible": true,
            "imageFile": "0B6C6B1E-4F1B-4B4E-9E0A-222222222222.png",
            "transform": { "origin": [4, 5], "size": [3, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
            "effects": { "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 },
                         "stroke": { "blue": 1, "enabled": false, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 4 } },
            "text": { "alignment": "Left", "blue": 0, "content": "Hi", "fontName": "Helvetica", "fontSize": 72,
                      "green": 0, "leading": 0, "red": 0, "tracking": 0 },
            "futureLayerKey": 7
        }]
    }).to_string()
}

fn rich_package(version: u32) -> Package {
    Package { manifest_json: rich_manifest(version),
        images: vec![("0B6C6B1E-4F1B-4B4E-9E0A-222222222222.png".into(), tiny_png())] }
}

#[test]
fn nothing_read_is_dropped_on_save() {
    let input: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    let doc = open_package(&rich_package(9)).unwrap();
    let output: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    for key in ["guides", "futureDocumentKey"] {
        assert_eq!(output[key], input[key], "manifest key {key}");
    }
    for key in ["effects", "text", "futureLayerKey"] {
        assert_eq!(output["layers"][0][key], input["layers"][0][key], "layer key {key}");
    }
}

#[test]
fn guides_are_written_as_the_mac_writes_them() {
    let doc = open_package(&rich_package(9)).unwrap();
    let text = save_package(&doc).unwrap().manifest_json;
    assert!(text.contains("\"position\": 16\n") || text.contains("\"position\": 16,"), "whole numbers without .0: {text}");
    assert!(text.contains("\"axis\": \"vertical\""));
}

#[test]
fn an_empty_guide_list_is_omitted_not_written() {
    let doc = open_package(&mac_fixture()).unwrap();
    let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    assert!(saved.get("guides").is_none(), "the Mac omits guides when there are none (R 1.2)");
}

#[test]
fn version_7_cannot_carry_guides() {
    assert!(matches!(open_package(&rich_package(7)), Err(ProjectError::Invalid)));
    assert!(open_package(&rich_package(8)).is_ok());
}

#[test]
fn guide_limits_match_the_mac() {
    let mut m: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    m["guides"][1]["id"] = m["guides"][0]["id"].clone();
    let dup = Package { manifest_json: m.to_string(), images: rich_package(9).images };
    assert!(matches!(open_package(&dup), Err(ProjectError::Invalid)), "duplicate guide ids");
    let mut far: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    far["guides"][0]["position"] = json!(1_000_001);
    let far = Package { manifest_json: far.to_string(), images: rich_package(9).images };
    assert!(matches!(open_package(&far), Err(ProjectError::Invalid)), "beyond 1,000,000");
}

fn with_guide_count(count: usize) -> Package {
    let mut m: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    m["guides"] = Value::Array((0..count).map(|i| json!({ "axis": "vertical",
        "id": uuid::Uuid::new_v4().to_string().to_uppercase(), "position": i })).collect());
    Package { manifest_json: m.to_string(), images: rich_package(9).images }
}

#[test]
fn more_than_1000_guides_is_too_large() {
    assert!(open_package(&with_guide_count(MAX_GUIDES)).is_ok(), "exactly the limit opens");
    assert_eq!(open_package(&with_guide_count(MAX_GUIDES + 1)).unwrap_err(), ProjectError::TooLarge);
}

#[test]
fn live_text_must_sit_on_a_pixel_layer() {
    let mut m: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    m["layers"][0].as_object_mut().unwrap().remove("imageFile");
    let no_pixels = Package { manifest_json: m.to_string(), images: vec![] };
    assert!(matches!(open_package(&no_pixels), Err(ProjectError::Invalid)), "text without imageFile (R 2.2)");
}

fn with_text(text: Value) -> Package {
    let mut m: Value = serde_json::from_str(&rich_manifest(9)).unwrap();
    m["layers"][0]["text"] = text;
    Package { manifest_json: m.to_string(), images: rich_package(9).images }
}

#[test]
fn live_text_must_meet_the_macs_text_limits() {
    // LayerTextStyle.isValid (TypeTool.swift:25-36), checked by ProjectStore.validate (R 2.2).
    let valid = serde_json::from_str::<Value>(&rich_manifest(9)).unwrap()["layers"][0]["text"].clone();
    assert!(open_package(&with_text(valid.clone())).is_ok(), "the rich fixture's text is valid");
    let mut big = valid.clone();
    big["fontSize"] = json!(2001);
    assert!(matches!(open_package(&with_text(big)), Err(ProjectError::Invalid)), "fontSize above 2000");
    // 50,001 characters but 100,002 UTF-16 code units: counting chars instead of UTF-16 units passes it.
    let mut long = valid.clone();
    long["content"] = json!("\u{1F600}".repeat(50_001));
    assert!(matches!(open_package(&with_text(long)), Err(ProjectError::Invalid)), "content above 100,000 UTF-16 units");
}

#[test]
fn replacing_pixels_drops_live_text_and_shape_but_keeps_effects_and_unknown_keys() {
    let mut doc = open_package(&rich_package(9)).unwrap();
    doc.layers[0].extra.shape = Some(json!({ "kind": "Rectangle" }));
    doc.layers[0].set_pixels(Some(Raster::from_premultiplied(3, 2, vec![0; 24])));
    let layer = &doc.layers[0];
    assert!(layer.extra.text.is_none() && layer.extra.shape.is_none(), "text and shape describe the old pixels");
    assert!(layer.extra.effects.is_some(), "effects do not depend on the pixels");
    assert_eq!(layer.extra.unknown.get("futureLayerKey"), Some(&json!(7)));
}

#[test]
fn guides_and_unknown_document_keys_are_content_for_undo() {
    let a = open_package(&rich_package(9)).unwrap();
    let mut b = a.clone();
    b.guides[0].position += 1.0;
    assert!(!a.same_content(&b));
    let mut c = a.clone();
    c.unknown.insert("another".into(), json!(true));
    assert!(!a.same_content(&c));
}
