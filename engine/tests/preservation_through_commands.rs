//! M4: a permanent guard that preservation survives the real command path (`Engine::execute`,
//! undo/redo, save), not just `Layer::record`/`from_record`/`set_pixels` in isolation
//! (format_v9.rs). A future op that rebuilds a `Layer` field by field, instead of mutating a
//! clone of the existing one, would drop effects/text/unknown with every one of those tests
//! still green; this one drives a rich document through DuplicateLayer, GroupLayers, CanvasSize,
//! Crop, FlipCanvas, SetLayerOpacity and SetLayerTransform, undo and redo, and checks the saved
//! JSON.

use compositor_engine::*;
use serde_json::{json, Value};

fn tiny_png() -> Vec<u8> {
    encode_png(&Raster::from_premultiplied(3, 2, [200u8, 40, 40, 255].repeat(6)), 72.0).unwrap()
}

const FOLDER_ID: &str = "0B6C6B1E-4F1B-4B4E-9E0A-AAAAAAAAAAAA";
const TEXT_ID: &str = "0B6C6B1E-4F1B-4B4E-9E0A-BBBBBBBBBBBB";
const ADJ_ID: &str = "0B6C6B1E-4F1B-4B4E-9E0A-CCCCCCCCCCCC";
const GUIDE_ID: &str = "0B6C6B1E-4F1B-4B4E-9E0A-DDDDDDDDDDDD";

/// A rich v9 document, in `format_v9.rs`'s `rich_manifest` style: a 50% folder with a layer-level
/// unknown key, a pixel layer with effects, live text and an unknown key, a Gaussian Blur
/// adjustment layer (blurRadius 24) with its own unknown key, a guide, and a manifest-level
/// unknown key.
fn rich_command_manifest() -> String {
    let image_file = format!("{TEXT_ID}.png");
    json!({
        "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 40, "height": 30,
        "activeLayerID": TEXT_ID,
        "guides": [ { "axis": "vertical", "id": GUIDE_ID, "position": 16 } ],
        "futureDocumentKey": { "nested": [1, 2.5, "x"] },
        "layers": [
            { "id": FOLDER_ID, "name": "Folder", "isVisible": true, "isGroup": true, "opacity": 0.5,
              "transform": { "origin": [0, 0], "size": [40, 30], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "futureFolderKey": true },
            { "id": TEXT_ID, "name": "Titled", "isVisible": true, "imageFile": image_file,
              "transform": { "origin": [4, 5], "size": [3, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "effects": { "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 },
                           "stroke": { "blue": 1, "enabled": false, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 4 } },
              "text": { "alignment": "Left", "blue": 0, "content": "Hi", "fontName": "Helvetica", "fontSize": 72,
                        "green": 0, "leading": 0, "red": 0, "tracking": 0 },
              "futureLayerKey": 7 },
            { "id": ADJ_ID, "name": "Blur", "isVisible": true,
              "transform": { "origin": [0, 0], "size": [40, 30], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "adjustment": { "kind": "Gaussian Blur", "hue": 0, "saturation": 0, "lightness": 0, "colorize": false,
                  "levels": { "channel": "RGB", "ranges": [ {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}, {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255} ] },
                  "curves": { "channel": "RGB", "channels": [ [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}],
                      [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}] ] },
                  "blurRadius": 24 },
              "futureAdjLayerKey": "z" }
        ]
    }).to_string()
}

fn rich_command_package() -> Package {
    Package { manifest_json: rich_command_manifest(), images: vec![(format!("{TEXT_ID}.png"), tiny_png())] }
}

#[test]
fn preservation_survives_duplicate_group_canvas_crop_flip_opacity_transform_undo_redo() {
    let mut e = Engine::new();
    let id = e.open_package(&rich_command_package(), None).unwrap();
    let text_id = uuid::Uuid::parse_str(TEXT_ID).unwrap();
    let adj_id = uuid::Uuid::parse_str(ADJ_ID).unwrap();

    e.execute(id, Command::DuplicateLayer { id: text_id }).unwrap();
    let dup_id = e.state(id).unwrap().active_layer_id.unwrap();
    assert_ne!(dup_id, text_id, "DuplicateLayer produced a new layer");

    e.execute(id, Command::GroupLayers { ids: vec![dup_id, adj_id] }).unwrap();
    let group_id = e.state(id).unwrap().active_layer_id.unwrap();

    // Anchor 2 (top-right): the width grows by 20, all of it on the left (offset.x = 20).
    e.execute(id, Command::CanvasSize { width: 60, height: 30, anchor: 2, fill: None }).unwrap();
    // content_offset {-5, 0}: the crop's left edge moves 5px into the (now 60-wide) canvas.
    e.execute(id, Command::Crop { x: 5.0, y: 0.0, width: 50.0, height: 30.0 }).unwrap();
    e.execute(id, Command::FlipCanvas { horizontal: true }).unwrap();
    e.execute(id, Command::SetLayerOpacity { id: group_id, opacity: 0.3 }).unwrap();

    let before_transform = e.state(id).unwrap().layers.iter().find(|l| l.id == dup_id).unwrap().transform;
    let mut moved = before_transform;
    moved.origin.x += 2.0;
    moved.origin.y += 3.0;
    e.execute(id, Command::SetLayerTransform { id: dup_id, transform: moved }).unwrap();
    assert_eq!(e.state(id).unwrap().layers.iter().find(|l| l.id == dup_id).unwrap().transform.origin, moved.origin);

    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.iter().find(|l| l.id == dup_id).unwrap().transform.origin, before_transform.origin, "undo restores the transform");
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.iter().find(|l| l.id == dup_id).unwrap().transform.origin, moved.origin, "redo re-applies it");

    let saved: Value = serde_json::from_str(&e.save_package(id).unwrap().manifest_json).unwrap();
    let layers = saved["layers"].as_array().unwrap();
    let by_id = |target: uuid::Uuid| -> &Value {
        layers.iter().find(|l| l["id"].as_str().unwrap().eq_ignore_ascii_case(&target.to_string()))
            .unwrap_or_else(|| panic!("missing layer {target} in saved manifest"))
    };

    // The folder: its own layer-level unknown key survives, untouched by any of the above.
    assert_eq!(by_id(uuid::Uuid::parse_str(FOLDER_ID).unwrap())["futureFolderKey"], json!(true));

    // The original text layer: effects, live text and its unknown key are all still present.
    let original = by_id(text_id);
    assert_eq!(original["effects"]["shadow"]["blur"], json!(20));
    assert_eq!(original["effects"]["stroke"]["enabled"], json!(false));
    assert_eq!(original["text"]["content"], json!("Hi"));
    assert_eq!(original["futureLayerKey"], json!(7));

    // The duplicate: the same three, proving DuplicateLayer and SetLayerTransform do not drop them.
    let dup = by_id(dup_id);
    assert_eq!(dup["effects"]["shadow"]["blur"], json!(20));
    assert_eq!(dup["text"]["content"], json!("Hi"));
    assert_eq!(dup["futureLayerKey"], json!(7));

    // The Gaussian Blur adjustment layer: blurRadius and its own unknown key, now grouped.
    let adj = by_id(adj_id);
    assert_eq!(adj["adjustment"]["blurRadius"].as_f64(), Some(24.0));
    assert_eq!(adj["futureAdjLayerKey"], json!("z"));

    // The manifest-level unknown key.
    assert_eq!(saved["futureDocumentKey"], json!({ "nested": [1, 2.5, "x"] }));

    // The guide: CanvasSize moves it by the anchor-2 offset (+20), Crop by its content offset
    // (-5), then the horizontal flip mirrors it across the resulting 50-wide canvas
    // (axis = 25, position = 2*25 - position).
    let after_canvas_size = 16.0 + 20.0;
    let after_crop = after_canvas_size - 5.0;
    let expected_guide = 2.0 * 25.0 - after_crop;
    let guide = saved["guides"].as_array().unwrap().iter().find(|g| g["axis"] == "vertical").unwrap();
    assert_eq!(guide["position"].as_f64(), Some(expected_guide));

    assert_eq!(saved["width"], json!(50));
    assert_eq!(saved["height"], json!(30));

    // Now Image Size: it resamples every pixel layer, which drops live text (it describes the old
    // pixels) but keeps effects, scaled with the layer (Phase 3.5c ruling: 50 x 30 to 100 x 60
    // doubles them), and unknown keys (not pixel data).
    e.execute(id, Command::ImageSize { width: 100, height: 60, resolution: 72.0, sampling: Sampling::Smooth }).unwrap();
    let saved2: Value = serde_json::from_str(&e.save_package(id).unwrap().manifest_json).unwrap();
    let layers2 = saved2["layers"].as_array().unwrap();
    let by_id2 = |target: uuid::Uuid| -> &Value {
        layers2.iter().find(|l| l["id"].as_str().unwrap().eq_ignore_ascii_case(&target.to_string()))
            .unwrap_or_else(|| panic!("missing layer {target} in re-saved manifest"))
    };

    let original2 = by_id2(text_id);
    assert!(original2.get("text").is_none(), "Image Size resamples pixels, so live text (which describes the old ones) is dropped");
    assert_eq!(original2["effects"]["shadow"]["blur"], json!(40), "the effects scale with the layer");
    assert_eq!(original2["effects"]["shadow"]["angle"], json!(90), "and keep their direction, whole");
    assert_eq!(original2["effects"]["stroke"]["enabled"], json!(false));
    assert_eq!(original2["futureLayerKey"], json!(7), "unknown keys are not pixel data");

    let dup2 = by_id2(dup_id);
    assert!(dup2.get("text").is_none());
    assert_eq!(dup2["effects"]["shadow"]["blur"], json!(40));
    assert_eq!(dup2["futureLayerKey"], json!(7));

    assert_eq!(saved2["width"], json!(100));
    assert_eq!(saved2["height"], json!(60));
}
