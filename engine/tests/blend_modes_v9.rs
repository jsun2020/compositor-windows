use compositor_engine::*;
use serde_json::{json, Value};

/// The exact raw strings of LayerBlendMode in Mac 1.2.6 (LayerAppearance.swift:4-13, R 2.4).
/// Note the parentheses in "Linear Dodge (Add)".
const NEW: [&str; 11] = ["Linear Burn", "Linear Dodge (Add)", "Soft Light", "Hard Light", "Vivid Light",
    "Linear Light", "Pin Light", "Hard Mix", "Exclusion", "Subtract", "Divide"];

fn one_layer(version: u32, blend: &str) -> Package {
    let id = "0B6C6B1E-4F1B-4B4E-9E0A-222222222222";
    let manifest = json!({
        "format": "com.compositor.project", "version": version, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 3, "height": 2,
        "layers": [{ "id": id, "name": "L", "isVisible": true, "imageFile": format!("{id}.png"), "blendMode": blend,
            "transform": { "origin": [0, 0], "size": [3, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" } }]
    });
    // Valid premultiplied data: no colour channel above alpha.
    let png = encode_png(&Raster::from_premultiplied(3, 2, [20u8, 80, 40, 128].repeat(6)), 72.0).unwrap();
    Package { manifest_json: manifest.to_string(), images: vec![(format!("{id}.png"), png)] }
}

#[test]
fn every_new_blend_mode_string_round_trips_exactly() {
    for name in NEW {
        let doc = open_package(&one_layer(9, name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
        assert_eq!(saved["layers"][0]["blendMode"], json!(name));
    }
}

#[test]
fn a_new_mode_still_needs_version_3_like_every_non_normal_mode() {
    assert!(matches!(open_package(&one_layer(2, "Soft Light")), Err(ProjectError::Invalid)));
    // The positive control: at v3 the same file opens, so the refusal above is the version rule,
    // not a string that fails to parse.
    assert!(open_package(&one_layer(3, "Soft Light")).is_ok());
}

#[test]
fn undrawn_modes_composite_as_normal_until_phase_3_5b() {
    // Asymmetric colours so Normal and a real Soft Light differ: a red backdrop, a half-alpha green source.
    let mut doc = open_package(&one_layer(9, "Soft Light")).unwrap();
    let mut backdrop = Layer::with_pixels("B", Raster::from_premultiplied(3, 2, [200u8, 40, 40, 255].repeat(6)), Point { x: 0.0, y: 0.0 });
    backdrop.id = uuid::Uuid::new_v4();
    doc.layers.insert(0, backdrop);
    let region = Rect { x: 0.0, y: 0.0, width: 3.0, height: 2.0 };
    let soft = composite(&doc, region, 3, 2);
    doc.layers[1].blend_mode = BlendMode::Normal;
    let normal = composite(&doc, region, 3, 2);
    assert_eq!(soft.pixel(1, 1), normal.pixel(1, 1));
    assert!(!BlendMode::SoftLight.is_drawn() && BlendMode::Overlay.is_drawn());
}
