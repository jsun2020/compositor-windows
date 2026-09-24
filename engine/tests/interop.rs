//! Round-trip tests against fixtures shaped like Swift's `JSONEncoder` output (`.prettyPrinted,
//! .sortedKeys`), one per manifest format version. The macOS repo ships no `.comp` fixtures to
//! copy, so these are hand-written to the SHAPE of that encoder: uppercase hyphenated UUIDs,
//! `origin`/`size` as `[x, y]` arrays, enum values as their display strings, and only the
//! fields each version may contain. Spec section 5: "Round-trip tests open every fixture
//! version and re-save it." -- this is that promise, verified against manifests this codebase
//! did not write, not just its own encoder round-tripping with itself (`manifest.rs`,
//! `package.rs` cover that already).
mod fixtures;
use compositor_engine::*;
use fixtures::*;

const FIXTURES: [(&str, &str); 7] = [
    ("v1", include_str!("fixtures/manifests/v1.json")),
    ("v2", include_str!("fixtures/manifests/v2.json")),
    ("v3", include_str!("fixtures/manifests/v3.json")),
    ("v4", include_str!("fixtures/manifests/v4.json")),
    ("v5", include_str!("fixtures/manifests/v5.json")),
    ("v6", include_str!("fixtures/manifests/v6.json")),
    ("v7", include_str!("fixtures/manifests/v7.json")),
];

/// Fields carried through untouched by later phases, per the fixtures' own bullet points:
/// v2's `isGroup`/`parentID`, v3's `opacity`/`blendMode`, v4's mask file (implicitly, via the
/// asset round trip below), v5's `maskSourceID`, v6's group mask (asset round trip), v7's
/// `adjustment`, `maskPlacement`, `maskLinked` and `shape`.
const CARRIED_FIELDS: [&str; 9] =
    ["adjustment", "shape", "maskPlacement", "maskLinked", "parentID", "isGroup", "opacity", "blendMode", "maskSourceID"];

/// A pixel asset small enough that the specific content never matters to these tests -- only
/// that decoding and re-encoding it round-trips through the package layer at all.
fn tiny_image_bytes() -> Vec<u8> { encode_png(&pattern_raster(4, 4), 72.0).unwrap() }
fn tiny_mask_bytes() -> Vec<u8> { encode_gray_png(&GrayRaster::from_bytes(4, 4, vec![200; 16])).unwrap() }

/// Builds the package assets a fixture's `imageFile`/`maskFile` names require, generated at
/// test time rather than checked in as binary fixtures.
fn images_for(json: &str) -> Vec<(String, Vec<u8>)> {
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let mut out = Vec::new();
    for layer in value["layers"].as_array().unwrap() {
        if let Some(name) = layer.get("imageFile").and_then(|v| v.as_str()) { out.push((name.to_string(), tiny_image_bytes())); }
        if let Some(name) = layer.get("maskFile").and_then(|v| v.as_str()) { out.push((name.to_string(), tiny_mask_bytes())); }
    }
    out
}

/// True for a 36-character `8-4-4-4-12` hex UUID shape, ignoring case.
fn looks_like_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36 && b.iter().enumerate().all(|(i, &c)| if matches!(i, 8 | 13 | 18 | 23) { c == b'-' } else { c.is_ascii_hexdigit() })
}

/// Every UUID-shaped quoted string in the manifest text is uppercase, matching
/// `UUID().uuidString` on macOS. Scans quoted string content by splitting on `"`, which is
/// exact for this text: `serde_json::to_string_pretty` only escapes a `"` inside a string
/// value, and none of these fixtures' names or string fields contain one.
fn assert_all_uuids_uppercase(text: &str) {
    for (i, part) in text.split('"').enumerate() {
        if i % 2 == 1 && looks_like_uuid(part) {
            assert_eq!(part, part.to_uppercase(), "UUID must be uppercase in the manifest: {part}");
        }
    }
}

/// The manifest's object keys are sorted at every nesting level. `serde_json`'s `Map` is a
/// `BTreeMap` here (no `preserve_order` feature in `Cargo.lock`), so re-parsing the text into a
/// `Value` and pretty-printing it again always yields the canonical sorted form; sorted output
/// is therefore exactly the text that round-trips to itself byte for byte.
fn assert_keys_are_sorted(text: &str) {
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    let canonical = serde_json::to_string_pretty(&value).unwrap();
    assert_eq!(text, canonical, "manifest JSON keys must be sorted at every nesting level");
}

#[test]
fn every_fixture_version_opens_resaves_at_v9_and_preserves_later_phase_fields() {
    for (name, json) in FIXTURES {
        let pkg = Package { manifest_json: json.to_string(), images: images_for(json) };
        let doc = open_package(&pkg).unwrap_or_else(|e| panic!("{name}: open_package failed: {e:?}"));
        let resaved = save_package(&doc).unwrap_or_else(|e| panic!("{name}: save_package failed: {e:?}"));

        assert_keys_are_sorted(&resaved.manifest_json);
        assert_all_uuids_uppercase(&resaved.manifest_json);

        let before: serde_json::Value = serde_json::from_str(json).unwrap();
        let after: serde_json::Value = serde_json::from_str(&resaved.manifest_json).unwrap();
        assert_eq!(after["version"], serde_json::json!(9), "{name}: re-saved manifest must be version 9");

        let before_layers = before["layers"].as_array().unwrap();
        let after_layers = after["layers"].as_array().unwrap();
        assert_eq!(before_layers.len(), after_layers.len(), "{name}: layer count changed on round trip");
        for b in before_layers {
            let id = b["id"].as_str().unwrap().to_uppercase();
            let a = after_layers.iter().find(|l| l["id"] == serde_json::json!(id))
                .unwrap_or_else(|| panic!("{name}: layer {id} missing after round trip"));
            for field in CARRIED_FIELDS {
                if let Some(before_value) = b.get(field) {
                    let after_value = a.get(field).unwrap_or_else(|| panic!("{name}: layer {id} lost field {field}"));
                    assert_eq!(after_value, before_value, "{name}: layer {id} field {field} changed on round trip");
                }
            }
        }
    }
}

#[test]
fn a_fixture_with_lowercase_uuids_parses_and_resaves_uppercase() {
    // `id` is written lowercase (unlike the real Swift encoder, which never emits lowercase --
    // this fixture exists only to prove parsing does not care). `imageFile` still has to equal
    // the uppercase name `LayerRecord::image_filename` computes from the parsed id, exactly as
    // validation requires regardless of how the fixture spelled `id`.
    let doc_id = "b0000000-0000-4000-8000-00000000000b";
    let layer_id_lower = "b0000000-0000-4000-8000-00000000000c";
    let layer_id_upper = layer_id_lower.to_uppercase();
    let json = format!(
        r#"{{
  "format": "com.compositor.project",
  "version": 1,
  "colorSpace": "sRGB",
  "documentID": "{doc_id}",
  "width": 10,
  "height": 10,
  "layers": [
    {{
      "id": "{layer_id_lower}",
      "name": "Lowercase",
      "isVisible": true,
      "transform": {{
        "origin": [0, 0],
        "size": [10, 10],
        "rotation": 0,
        "flipX": false,
        "flipY": false,
        "sampling": "High quality"
      }},
      "imageFile": "{layer_id_upper}.png"
    }}
  ]
}}"#
    );
    let pkg = Package { manifest_json: json.clone(), images: images_for(&json) };
    let doc = open_package(&pkg).unwrap();
    assert_eq!(doc.id.to_string(), doc_id, "a lowercase documentID still parses to the same id");
    let resaved = save_package(&doc).unwrap();
    assert_all_uuids_uppercase(&resaved.manifest_json);
    assert!(resaved.manifest_json.contains(&doc_id.to_uppercase()), "the re-saved documentID must be uppercase");
}

#[test]
fn a_macos_adjustment_layer_opens_and_saves_unchanged() {
    // A v7 manifest as macOS writes it: an adjustment layer with only the keys that kind uses.
    let manifest = r#"{
  "activeLayerID": "11111111-1111-4111-8111-111111111111",
  "colorSpace": "sRGB",
  "documentID": "22222222-2222-4222-8222-222222222222",
  "format": "com.compositor.project",
  "height": 4,
  "layers": [
    {
      "adjustment": {"colorize":false,"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]},"hue":120,"kind":"Hue/Saturation","levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255}]},"lightness":0,"saturation":0},
      "id": "11111111-1111-4111-8111-111111111111",
      "isVisible": true,
      "name": "Hue/Saturation",
      "transform": {"origin":[0,0],"size":[4,4]}
    }
  ],
  "version": 7,
  "width": 4
}"#;
    let doc = open_package(&Package { manifest_json: manifest.to_string(), images: vec![] }).unwrap();
    let adjustment = doc.layers[0].extra.adjustment.as_ref().unwrap();
    assert_eq!(adjustment.kind, AdjustmentKind::Hsv);
    assert_eq!(adjustment.resolved_hsv().adjustment(ColorRange::Master).hue, 120.0, "the legacy scalar still drives the adjustment");
    let saved = save_package(&doc).unwrap();
    let written: serde_json::Value = serde_json::from_str(&saved.manifest_json).unwrap();
    let original: serde_json::Value = serde_json::from_str(manifest).unwrap();
    assert_eq!(written["layers"][0]["adjustment"], original["layers"][0]["adjustment"], "re-saved byte for byte");
}

/// HAND-WRITTEN, not Mac-produced: no Mac was available when this was added. It follows Swift's
/// documented `Dictionary` encoding for a key type that is neither `String`/`Int` nor
/// `CodingKeyRepresentable` (`ColorRange`): an unkeyed container of alternating key and value, in
/// the hash order of that process. The keys below are deliberately NOT in `ColorRange::ALL` order,
/// so the reader cannot depend on order. `.prettyPrinted, .sortedKeys` layout, as `ProjectStore`
/// writes it. To be confirmed against a real Mac save.
const MAC_HSV_ARRAY_FORM: &str = r#"{
  "activeLayerID" : "33333333-3333-4333-8333-333333333333",
  "colorSpace" : "sRGB",
  "documentID" : "44444444-4444-4444-8444-444444444444",
  "format" : "com.compositor.project",
  "height" : 4,
  "layers" : [
    {
      "adjustment" : {
        "colorize" : false,
        "curves" : {
          "channel" : "RGB",
          "channels" : [
            [ { "x" : 0, "y" : 0 }, { "x" : 255, "y" : 255 } ],
            [ { "x" : 0, "y" : 0 }, { "x" : 255, "y" : 255 } ],
            [ { "x" : 0, "y" : 0 }, { "x" : 255, "y" : 255 } ],
            [ { "x" : 0, "y" : 0 }, { "x" : 255, "y" : 255 } ]
          ]
        },
        "hsvSettings" : {
          "adjustments" : [
            "Blues",
            { "hue" : 0, "lightness" : 0, "saturation" : -40 },
            "Master",
            { "hue" : 0, "lightness" : 10, "saturation" : 0 },
            "Reds",
            { "hue" : 30.5, "lightness" : 0, "saturation" : 0 }
          ],
          "bands" : [
            "Greens",
            { "falloffEnd" : 165, "falloffStart" : 75, "rangeEnd" : 135, "rangeStart" : 105 },
            "Magentas",
            { "falloffEnd" : 345, "falloffStart" : 255, "rangeEnd" : 315, "rangeStart" : 285 },
            "Reds",
            { "falloffEnd" : 50, "falloffStart" : 300, "rangeEnd" : 20, "rangeStart" : 330 },
            "Master",
            { "falloffEnd" : 360, "falloffStart" : 0, "rangeEnd" : 360, "rangeStart" : 0 },
            "Cyans",
            { "falloffEnd" : 225, "falloffStart" : 135, "rangeEnd" : 195, "rangeStart" : 165 },
            "Yellows",
            { "falloffEnd" : 105, "falloffStart" : 15, "rangeEnd" : 75, "rangeStart" : 45 },
            "Blues",
            { "falloffEnd" : 285, "falloffStart" : 195, "rangeEnd" : 255, "rangeStart" : 225 }
          ],
          "colorize" : false,
          "invertRange" : true,
          "range" : "Reds"
        },
        "hue" : 0,
        "kind" : "Hue/Saturation",
        "levels" : {
          "channel" : "RGB",
          "ranges" : [
            { "black" : 0, "gamma" : 1, "outputBlack" : 0, "outputWhite" : 255, "white" : 255 },
            { "black" : 0, "gamma" : 1, "outputBlack" : 0, "outputWhite" : 255, "white" : 255 },
            { "black" : 0, "gamma" : 1, "outputBlack" : 0, "outputWhite" : 255, "white" : 255 },
            { "black" : 0, "gamma" : 1, "outputBlack" : 0, "outputWhite" : 255, "white" : 255 }
          ]
        },
        "lightness" : 0,
        "saturation" : 0
      },
      "id" : "33333333-3333-4333-8333-333333333333",
      "isVisible" : true,
      "name" : "Hue/Saturation",
      "transform" : { "origin" : [ 0, 0 ], "size" : [ 4, 4 ] }
    }
  ],
  "version" : 7,
  "width" : 4
}"#;

fn assert_mac_hsv_settings(doc: &Document) {
    let hsv = doc.layers[0].extra.adjustment.as_ref().unwrap().hsv_settings.clone().expect("hsvSettings survives");
    assert_eq!(hsv.range, ColorRange::Reds);
    assert!(hsv.invert_range && !hsv.colorize);
    assert_eq!(hsv.adjustments.len(), 3);
    assert_eq!(hsv.adjustment(ColorRange::Blues), RangeAdjustment { hue: 0.0, saturation: -40.0, lightness: 0.0 });
    assert_eq!(hsv.adjustment(ColorRange::Master), RangeAdjustment { hue: 0.0, saturation: 0.0, lightness: 10.0 });
    assert_eq!(hsv.adjustment(ColorRange::Reds), RangeAdjustment { hue: 30.5, saturation: 0.0, lightness: 0.0 });
    assert_eq!(hsv.bands.len(), 7);
    assert_eq!(hsv.band(ColorRange::Reds), HueBand { falloff_start: 300.0, range_start: 330.0, range_end: 20.0, falloff_end: 50.0 });
    assert_eq!(hsv.band(ColorRange::Cyans), ColorRange::Cyans.default_band());
}

#[test]
fn a_macos_hue_saturation_layer_in_swift_array_form_opens_and_resaves_as_arrays() {
    let doc = open_package(&Package { manifest_json: MAC_HSV_ARRAY_FORM.to_string(), images: vec![] })
        .expect("a Mac project with an edited Hue/Saturation layer must open");
    assert_mac_hsv_settings(&doc);

    let saved = save_package(&doc).unwrap();
    assert_keys_are_sorted(&saved.manifest_json);
    let written: serde_json::Value = serde_json::from_str(&saved.manifest_json).unwrap();
    let hsv = &written["layers"][0]["adjustment"]["hsvSettings"];
    assert_eq!(hsv["adjustments"], serde_json::json!([
        "Master", {"hue": 0, "lightness": 10, "saturation": 0},
        "Reds", {"hue": 30.5, "lightness": 0, "saturation": 0},
        "Blues", {"hue": 0, "lightness": 0, "saturation": -40},
    ]), "re-saved in Swift's array form, ColorRange::ALL order");
    let keys: Vec<&str> = hsv["bands"].as_array().expect("bands re-saved as an array").iter().step_by(2).map(|k| k.as_str().unwrap()).collect();
    assert_eq!(keys, ["Master", "Reds", "Yellows", "Greens", "Cyans", "Blues", "Magentas"]);

    let reopened = open_package(&saved).unwrap();
    assert_mac_hsv_settings(&reopened);
}

#[test]
fn a_hue_saturation_layer_in_the_0_3_0_object_form_still_opens() {
    // What a 0.3.0 development build wrote: the same maps as objects keyed by range name.
    let mut value: serde_json::Value = serde_json::from_str(MAC_HSV_ARRAY_FORM).unwrap();
    let hsv = &mut value["layers"][0]["adjustment"]["hsvSettings"];
    for name in ["adjustments", "bands"] {
        let pairs = hsv[name].as_array().unwrap().clone();
        let object: serde_json::Map<String, serde_json::Value> =
            pairs.chunks(2).map(|p| (p[0].as_str().unwrap().to_string(), p[1].clone())).collect();
        hsv[name] = serde_json::Value::Object(object);
    }
    let doc = open_package(&Package { manifest_json: value.to_string(), images: vec![] })
        .expect("an object-form file from a 0.3.0 build must open");
    assert_mac_hsv_settings(&doc);
    let saved = save_package(&doc).unwrap();
    let written: serde_json::Value = serde_json::from_str(&saved.manifest_json).unwrap();
    assert!(written["layers"][0]["adjustment"]["hsvSettings"]["adjustments"].is_array(), "re-saved in the Mac's form");
}
