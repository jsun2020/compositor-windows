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
fn every_fixture_version_opens_resaves_at_v7_and_preserves_later_phase_fields() {
    for (name, json) in FIXTURES {
        let pkg = Package { manifest_json: json.to_string(), images: images_for(json) };
        let doc = open_package(&pkg).unwrap_or_else(|e| panic!("{name}: open_package failed: {e:?}"));
        let resaved = save_package(&doc).unwrap_or_else(|e| panic!("{name}: save_package failed: {e:?}"));

        assert_keys_are_sorted(&resaved.manifest_json);
        assert_all_uuids_uppercase(&resaved.manifest_json);

        let before: serde_json::Value = serde_json::from_str(json).unwrap();
        let after: serde_json::Value = serde_json::from_str(&resaved.manifest_json).unwrap();
        assert_eq!(after["version"], serde_json::json!(7), "{name}: re-saved manifest must be version 7");

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
