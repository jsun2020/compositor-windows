//! Project format 11 (Phase 4.5): Compositor for Mac 1.4.5 writes version 11 (ProjectStore.swift:15);
//! version 10 added a text's colour runs and 11 its font runs (TypeTool.swift:30-62, :190-202;
//! ProjectStore.swift:208-214). This build reads 1-11, keeps a text's runs verbatim (the layer's PNG
//! already shows them), refuses what the Mac refuses, and writes 11.
//!
//! The real Mac save here is `mac-4b1-probes/shapes.mac-1.3.7.comp` (format 11, no text). The two text
//! fixtures under `hand-written-1.4.5` are HAND-WRITTEN from the Mac's Codable encoding of
//! `LayerTextStyle` and its runs (TypeTool.swift:7-34, :190-202) and `ProjectLayerRecord`
//! (ProjectStore.swift:33-56), sorted and pretty printed as `JSONEncoder` writes them; no Mac saved
//! them (LL-069). Replace them with the user's A1 `text-runs.comp` and A2 `text-colour-only.comp` when
//! those come back.
use compositor_engine::*;
use serde_json::{json, Value};

fn fixtures() -> String { format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")) }

/// A package folder's manifest and its `images/` (a Mac save's `QuickLook/` is never read, as the Mac
/// reads only these two, ProjectStore.swift:146-196).
fn package_dir(dir: &str) -> Package {
    let manifest_json = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{dir}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    Package { manifest_json, images }
}

/// A package for a hand-written manifest: each layer's image a plain PNG of its transform's size (the
/// rendered text's pixels do not matter here).
fn package_of(manifest: &Value) -> Package {
    let images = manifest["layers"].as_array().unwrap().iter().filter_map(|l| {
        let file = l["imageFile"].as_str()?;
        let (w, h) = (l["transform"]["size"][0].as_u64().unwrap() as u32, l["transform"]["size"][1].as_u64().unwrap() as u32);
        Some((file.to_string(), encode_png(&Raster::from_premultiplied(w, h, [20u8, 30, 40, 255].repeat((w * h) as usize)), 72.0).unwrap()))
    }).collect();
    Package { manifest_json: manifest.to_string(), images }
}
fn hand_written(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{}/hand-written-1.4.5/{name}.manifest.json", fixtures())).unwrap()).unwrap()
}
/// The manifest with its first text layer's `text` changed by `edit`, at `version`.
fn with_text(mut manifest: Value, version: u32, edit: impl Fn(&mut Value)) -> Package {
    manifest["version"] = json!(version);
    let layer = manifest["layers"].as_array_mut().unwrap().iter_mut().find(|l| l.get("text").is_some()).unwrap();
    edit(&mut layer["text"]);
    package_of(&manifest)
}
fn saved(doc: &Document) -> Value { serde_json::from_str(&save_package(doc).unwrap().manifest_json).unwrap() }
/// Each layer's `text`, by name.
fn texts(manifest: &Value) -> Vec<(String, Value)> {
    manifest["layers"].as_array().unwrap().iter().filter(|l| l.get("text").is_some()).map(|l| (l["name"].as_str().unwrap().to_string(), l["text"].clone())).collect()
}

#[test]
fn a_real_format_11_save_opens_and_saves_as_11_with_its_shape_records_kept() {
    // Compositor for Mac 1.3.7's save of shapes.comp (Phase 4b-1's probe), QuickLook folder and all.
    let dir = format!("{}/mac-4b1-probes/shapes.mac-1.3.7.comp", fixtures());
    assert!(std::path::Path::new(&format!("{dir}/QuickLook/Preview.jpg")).exists(), "the Mac's save carries its Quick Look preview");
    let package = package_dir(&dir);
    let original: Value = serde_json::from_str(&package.manifest_json).unwrap();
    assert_eq!(original["version"], 11);
    let doc = open_package(&package).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(doc.layers.len(), 7);
    let again = saved(&doc);
    assert_eq!(again["version"], json!(CURRENT_VERSION));
    assert_eq!(CURRENT_VERSION, 11);
    for (a, b) in original["layers"].as_array().unwrap().iter().zip(again["layers"].as_array().unwrap()) {
        assert_eq!(a.get("shape"), b.get("shape"), "{}: the shape record comes back as the Mac wrote it", a["name"]);
    }
}

#[test]
fn text_runs_open_and_come_back_exactly_as_they_were() {
    for name in ["text-runs", "text-colour-only"] {
        let manifest = hand_written(name);
        let doc = open_package(&package_of(&manifest)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let again = saved(&doc);
        assert_eq!(again["version"], json!(11), "{name}");
        assert_eq!(texts(&again), texts(&manifest), "{name}: every run, integer for integer");
        // Nothing to notice: the layer's PNG already shows the runs (docs/project-format.md at v1.4.5).
        assert!(doc.undrawn().is_empty(), "{name}: {:?}", doc.undrawn());
    }
}

#[test]
fn colour_runs_need_format_10_and_font_runs_format_11() {
    // ProjectStore.swift:211-212: a run older than its format makes the project invalid.
    let colour_only = hand_written("text-colour-only");
    assert!(open_package(&with_text(colour_only.clone(), 10, |_| {})).is_ok(), "colour runs at 10");
    assert_eq!(open_package(&with_text(colour_only.clone(), 9, |_| {})).unwrap_err(), ProjectError::Invalid, "colour runs at 9");
    let both = hand_written("text-runs");
    assert_eq!(open_package(&with_text(both.clone(), 10, |_| {})).unwrap_err(), ProjectError::Invalid, "font runs at 10");
    assert!(open_package(&with_text(both.clone(), 11, |_| {})).is_ok());
    // The same text without its runs opens at 9 (so the refusals above are the runs' gates).
    assert!(open_package(&with_text(both, 9, |t| { t.as_object_mut().unwrap().remove("colorRuns"); t.as_object_mut().unwrap().remove("fontRuns"); })).is_ok());
    assert!(open_package(&with_text(colour_only, 9, |t| { t["colorRuns"] = Value::Null; })).is_ok(), "null is none");
}

#[test]
fn runs_the_mac_refuses_are_refused() {
    // TypeTool.swift:43-62 (`colorRunsAreValid`, `fontRunsAreValid`) and the synthesized decoder.
    let colour = |location: Value, length: Value| json!({ "location": location, "length": length, "red": 1, "green": 0, "blue": 0 });
    let font = |name: &str| json!({ "location": 0, "length": 5, "fontName": name });
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("out of order", Box::new(move |t| t["colorRuns"] = json!([colour(json!(6), json!(2)), colour(json!(0), json!(2))]))),
        ("overlapping", Box::new(move |t| t["colorRuns"] = json!([colour(json!(0), json!(4)), colour(json!(3), json!(2))]))),
        ("an empty list", Box::new(|t| t["colorRuns"] = json!([]))),
        ("a zero length", Box::new(move |t| t["colorRuns"] = json!([colour(json!(2), json!(0))]))),
        ("a negative location", Box::new(move |t| t["colorRuns"] = json!([colour(json!(-1), json!(2))]))),
        ("past the end", Box::new(move |t| t["colorRuns"] = json!([colour(json!(6), json!(6))]))),
        ("a fractional location", Box::new(move |t| t["colorRuns"] = json!([colour(json!(1.5), json!(2))]))),
        ("a channel past 1", Box::new(|t| t["colorRuns"] = json!([{ "location": 0, "length": 1, "red": 1.5, "green": 0, "blue": 0 }]))),
        ("a missing channel", Box::new(|t| t["colorRuns"] = json!([{ "location": 0, "length": 1, "red": 1, "green": 0 }]))),
        ("a run that is not an object", Box::new(|t| t["colorRuns"] = json!([6]))),
        ("runs that are not a list", Box::new(|t| t["colorRuns"] = json!({ "location": 0 }))),
        ("an empty face", Box::new(move |t| t["fontRuns"] = json!([font("")]))),
        ("a line feed in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\nBold")]))),
        ("a carriage return in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\rBold")]))),
        ("a line separator in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\u{2028}Bold")]))),
        ("a face of 201 characters", Box::new(move |t| t["fontRuns"] = json!([font(&"H".repeat(201))]))),
        ("a face that is not a string", Box::new(|t| t["fontRuns"] = json!([{ "location": 0, "length": 5, "fontName": 7 }]))),
    ];
    for (what, edit) in &cases {
        assert_eq!(open_package(&with_text(hand_written("text-runs"), 11, |t| edit(t))).unwrap_err(), ProjectError::Invalid, "{what}");
    }
    // And what the Mac takes: runs meeting end to end, a whole number written with a fraction (Swift's
    // `Int(exactly:)`), a key it does not know inside a run, a face of 200 characters.
    for (what, edit) in [
        ("runs meeting", Box::new(move |t: &mut Value| t["colorRuns"] = json!([colour(json!(0), json!(6)), colour(json!(6), json!(5))])) as Box<dyn Fn(&mut Value)>),
        ("6.0 for 6", Box::new(move |t: &mut Value| t["colorRuns"] = json!([colour(json!(6.0), json!(5))]))),
        ("an unknown key", Box::new(|t: &mut Value| t["fontRuns"] = json!([{ "location": 0, "length": 5, "fontName": "Times-Roman", "tracking": 3 }]))),
        ("200 characters", Box::new(move |t: &mut Value| t["fontRuns"] = json!([font(&"H".repeat(200))]))),
    ] {
        assert!(open_package(&with_text(hand_written("text-runs"), 11, |t| edit(t))).is_ok(), "{what}");
    }
}

#[test]
fn runs_are_measured_in_utf16_units_of_the_content() {
    // TypeTool.swift:30: offsets into `content.utf16`. A grinning face is two units, an accented e
    // written as e and a combining acute is two, so "e\u{301}\u{1F600}!" is 5 units long.
    let content = |t: &mut Value| t["content"] = json!("e\u{301}\u{1F600}!");
    let run = |location: i64, length: i64| json!([{ "location": location, "length": length, "red": 0, "green": 0, "blue": 1 }]);
    assert!(open_package(&with_text(hand_written("text-colour-only"), 11, |t| { content(t); t["colorRuns"] = run(2, 3); })).is_ok(), "the face and the ! end at 5");
    assert_eq!(open_package(&with_text(hand_written("text-colour-only"), 11, |t| { content(t); t["colorRuns"] = run(2, 4); })).unwrap_err(), ProjectError::Invalid, "6 is past the end");
}

#[test]
fn a_newer_format_is_refused_with_the_versions_this_build_reads() {
    let mut manifest = hand_written("text-runs");
    manifest["version"] = json!(12);
    let refused = open_package(&package_of(&manifest)).unwrap_err();
    assert_eq!(refused, ProjectError::Version(12));
    assert_eq!(refused.to_string(), "This project uses format version 12. This app supports versions 1-11, which Compositor for Mac saves up to version 1.4.5.");
}
