//! Layer effects as Compositor for Mac reads and writes them (R 2.1; LayerEffects.swift:5-210):
//! typed, every member but `enabled` required on decode, Mac-shaped numbers, nothing read lost.
use compositor_engine::*;
use serde_json::{json, Value};

const ID: &str = "0B6C6B1E-4F1B-4B4E-9E0A-5E5E5E5E5E51";

fn tiny_png() -> Vec<u8> { encode_png(&Raster::from_premultiplied(3, 2, [200u8, 40, 40, 255].repeat(6)), 72.0).unwrap() }

/// All six, as the Mac's own round-trip test sets four of them (ProjectTests.swift:220-225) plus
/// both glows: fractional and whole numbers, no member at its default, and `enabled` present on
/// one effect only (the Mac writes it only once the user has toggled that effect).
fn six() -> Value {
    json!({
        "colorOverlay": { "blue": 0.4, "green": 0.1, "opacity": 0.65, "red": 0.9 },
        "innerGlow": { "blue": 0.3, "enabled": false, "green": 1, "opacity": 0.6, "red": 0.25, "size": 12.5 },
        "innerShadow": { "angle": 135, "blue": 0.05, "blur": 4, "distance": 6, "green": 0.05, "opacity": 0.5, "red": 0.05 },
        "outerGlow": { "blue": 1, "green": 0.8, "opacity": 0.6, "red": 0.2, "size": 35 },
        "shadow": { "angle": 45, "blue": 0.3, "blur": 10, "distance": 15, "green": 0.2, "opacity": 0.75, "red": 0.2 },
        "stroke": { "blue": 0.2, "green": 0.8, "inside": false, "opacity": 0.9, "red": 0.1, "size": 8 }
    })
}

fn package(effects: Value) -> Package {
    let manifest = json!({
        "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-5E5E5E5E5E50", "width": 40, "height": 30,
        "layers": [{ "id": ID, "name": "Styled", "isVisible": true, "imageFile": format!("{ID}.png"),
            "transform": { "origin": [4, 5], "size": [3, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
            "effects": effects }]
    });
    Package { manifest_json: manifest.to_string(), images: vec![(format!("{ID}.png"), tiny_png())] }
}

/// The saved layer's `effects`, and the whole manifest text.
fn saved(doc: &Document) -> (Value, String) {
    let text = save_package(doc).unwrap().manifest_json;
    (serde_json::from_str::<Value>(&text).unwrap()["layers"][0]["effects"].clone(), text)
}

#[test]
fn all_six_effects_read_as_typed_values_and_save_back_as_the_mac_writes_them() {
    let doc = open_package(&package(six())).unwrap();
    let effects = doc.layers[0].extra.effects.as_ref().unwrap();
    assert_eq!(effects.stroke.as_ref().unwrap().size, 8.0);
    assert!(!effects.stroke.as_ref().unwrap().inside);
    assert_eq!(effects.shadow.as_ref().unwrap().angle, 45.0);
    assert_eq!(effects.inner_glow.as_ref().unwrap().enabled, Some(false));
    assert_eq!(effects.outer_glow.as_ref().unwrap().enabled, None, "an absent enabled stays absent");
    let (written, text) = saved(&doc);
    assert_eq!(written, six(), "every key and value as read");
    // Swift writes a whole-number Double without a fraction (R 0): `15`, never `15.0`.
    assert!(text.contains("\"distance\": 15,") && !text.contains("15.0"), "{text}");
}

#[test]
fn the_effects_the_mac_opened_in_edited_rich_file_save_back_unchanged() {
    // This port wrote the probe; Compositor for Mac 1.2.10 opened it and drew both shadows.
    let dir = format!("{}/tests/fixtures/mac-1.2.10-probes/edited-rich-file.comp", env!("CARGO_MANIFEST_DIR"));
    let manifest_json = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{dir}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    let input: Value = serde_json::from_str(&manifest_json).unwrap();
    let doc = open_package(&Package { manifest_json, images }).unwrap();
    let output: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    let styled = |m: &Value| m["layers"].as_array().unwrap().iter().filter(|l| l.get("effects").is_some())
        .map(|l| (l["id"].clone(), l["effects"].clone())).collect::<Vec<_>>();
    assert_eq!(styled(&input).len(), 2, "the probe carries two drop shadows");
    assert_eq!(styled(&output), styled(&input));
}

#[test]
fn an_effect_missing_any_member_but_enabled_is_refused_as_the_macs_decode_refuses_it() {
    let all = six();
    let mut refused = 0;
    for (kind, effect) in all.as_object().unwrap() {
        for key in effect.as_object().unwrap().keys().filter(|k| *k != "enabled") {
            let mut broken = all.clone();
            broken[kind].as_object_mut().unwrap().remove(key);
            assert_eq!(open_package(&package(broken)).unwrap_err(), ProjectError::Invalid, "{kind} without {key}");
            refused += 1;
        }
    }
    // stroke 6, shadow 7, colorOverlay 4, innerShadow 7, outerGlow 5, innerGlow 5 (enabled aside).
    assert_eq!(refused, 34, "every required member was tried");
    let mut no_enabled = all.clone();
    no_enabled["innerGlow"].as_object_mut().unwrap().remove("enabled");
    assert!(open_package(&package(no_enabled)).is_ok(), "enabled alone is optional");
}

#[test]
fn a_member_of_the_wrong_type_is_refused() {
    for (kind, key, value) in [("stroke", "inside", json!("false")), ("stroke", "size", json!(true)), ("shadow", "enabled", json!(1)), ("outerGlow", "red", json!("1"))] {
        let mut broken = six();
        broken[kind][key] = value.clone();
        assert_eq!(open_package(&package(broken)).unwrap_err(), ProjectError::Invalid, "{kind}.{key} = {value}");
    }
}

#[test]
fn keys_a_later_mac_may_write_are_kept_and_named_in_the_notice() {
    let mut effects = six();
    effects["shadow"]["spread"] = json!(12);
    effects["bevel"] = json!({ "depth": 3 });
    let doc = open_package(&package(effects.clone())).unwrap();
    assert_eq!(saved(&doc).0, effects, "nothing lost, inside an effect or beside the six");
    assert!(doc.undrawn().contains(&"settings from a newer version of Compositor".to_string()));
    let plain = open_package(&package(six())).unwrap();
    assert!(!plain.undrawn().contains(&"settings from a newer version of Compositor".to_string()));
}

#[test]
fn out_of_range_effects_open_and_save_unchanged_because_the_mac_does_not_validate_them() {
    let mut effects = six();
    effects["stroke"]["size"] = json!(600);
    effects["shadow"]["opacity"] = json!(1.5);
    let doc = open_package(&package(effects.clone())).unwrap();
    assert_eq!(saved(&doc).0, effects);
    assert!(!doc.layers[0].extra.effects.as_ref().unwrap().is_valid());
}

#[test]
fn an_empty_effects_object_and_a_null_enabled_read_as_the_mac_reads_them() {
    let doc = open_package(&package(json!({}))).unwrap();
    assert_eq!(saved(&doc).0, json!({}), "an empty object stays an object");
    let mut effects = six();
    effects["shadow"]["enabled"] = Value::Null;
    let doc = open_package(&package(effects)).unwrap();
    assert_eq!(doc.layers[0].extra.effects.as_ref().unwrap().shadow.as_ref().unwrap().enabled, None, "null is absent (decodeIfPresent)");
    assert!(saved(&doc).0["shadow"].get("enabled").is_none());
}

#[test]
fn validity_follows_the_macs_ranges_at_each_edge() {
    let e: LayerEffects = serde_json::from_value(six()).unwrap();
    let mut s = e.stroke.clone().unwrap();
    s.size = 500.0; assert!(s.is_valid());
    s.size = 500.5; assert!(!s.is_valid());
    s.size = -0.5; assert!(!s.is_valid());
    let mut d = e.shadow.clone().unwrap();
    d.angle = -360.0; assert!(d.is_valid());
    d.angle = 360.5; assert!(!d.is_valid());
    d.angle = 45.0; d.distance = 5000.0; assert!(d.is_valid());
    d.distance = 5000.5; assert!(!d.is_valid());
    d.distance = 15.0; d.blur = 500.5; assert!(!d.is_valid());
    let mut g = e.outer_glow.clone().unwrap();
    g.opacity = 1.0; assert!(g.is_valid());
    g.opacity = 1.01; assert!(!g.is_valid());
    g.opacity = 0.6; g.red = -0.01; assert!(!g.is_valid());
    g.red = f64::NAN; assert!(!g.is_valid());
    let mut o = e.color_overlay.clone().unwrap(); o.blue = 1.2; assert!(!o.is_valid());
    let mut i = e.inner_shadow.clone().unwrap(); i.blur = -1.0; assert!(!i.is_valid());
    let mut ig = e.inner_glow.clone().unwrap(); ig.size = 501.0; assert!(!ig.is_valid());
    // A hidden invalid effect does not stop the shown ones (`visible` comes before `isValid`).
    let mut hidden_bad = e.clone();
    hidden_bad.inner_glow.as_mut().unwrap().size = 900.0; // enabled: false in six()
    assert!(hidden_bad.drawn().is_some());
    hidden_bad.stroke.as_mut().unwrap().size = 900.0;
    assert!(hidden_bad.drawn().is_none(), "an invalid shown effect stops them all, as `cached` does");
}

#[test]
fn the_margin_is_the_macs_largest_reach_rounded_up_plus_two() {
    let mac_margin = |reaches: &[f64]| reaches.iter().cloned().fold(0.0f64, f64::max).ceil() as u32 + 2;
    let e: LayerEffects = serde_json::from_value(six()).unwrap();
    // Outside stroke 8, shadow 15 + 3 x 10, outer glow 3 x 35; the inner effects reach nothing.
    assert_eq!(e.margin(), mac_margin(&[8.0, 15.0 + 3.0 * 10.0, 3.0 * 35.0]));
    let mut inside = e.clone();
    inside.outer_glow = None;
    inside.shadow = None;
    inside.stroke.as_mut().unwrap().inside = true;
    inside.stroke.as_mut().unwrap().size = 7.4;
    assert_eq!(inside.margin(), 2, "an inside stroke reaches nothing outside");
    let mut outside = inside.clone();
    outside.stroke.as_mut().unwrap().inside = false;
    assert_eq!(outside.margin(), mac_margin(&[7.4]), "7.4 rounds up");
    let mut hidden = e.clone();
    hidden.outer_glow.as_mut().unwrap().enabled = Some(false);
    assert_eq!(hidden.margin(), mac_margin(&[8.0, 45.0]), "a hidden glow reaches nothing");
}

#[test]
fn the_macs_own_decode_samples_open_with_their_values() {
    // Verbatim from Compositor for Mac's tests: OuterGlowTests.swift:69-73
    // (`layerEffectsCodableBackwardCompatibility`, a stroke without `enabled`) and
    // InnerGlowTests.swift:55-67 (`innerGlowBackwardCompatibility`, a shadow without `enabled`, keys
    // in declaration order). Mac-authored JSON, so these do not rest on files this port wrote.
    let older: LayerEffects = serde_json::from_str(r#"
        {
            "stroke": { "size": 3, "red": 0, "green": 0, "blue": 0, "opacity": 1, "inside": false }
        }
        "#).unwrap();
    assert_eq!(older.stroke.as_ref().unwrap().size, 3.0);
    assert!(older.outer_glow.is_none() && older.is_valid());
    let shadow: LayerEffects = serde_json::from_str(r#"
        {
            "shadow": {
                "angle": 90,
                "distance": 10,
                "blur": 15,
                "red": 0,
                "green": 0,
                "blue": 0,
                "opacity": 0.5
            }
        }
        "#).unwrap();
    let s = shadow.shadow.as_ref().unwrap();
    assert_eq!((s.angle, s.distance, s.blur, s.opacity, s.enabled), (90.0, 10.0, 15.0, 0.5, None));
    assert!(shadow.inner_glow.is_none() && shadow.is_valid());
}
