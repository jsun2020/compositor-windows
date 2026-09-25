use compositor_engine::*;
use serde_json::json;

fn pixel_layer() -> Layer { Layer::with_pixels("P", Raster::from_premultiplied(1, 1, vec![9, 9, 9, 255]), Point { x: 0.0, y: 0.0 }) }

#[test]
fn a_plain_project_has_nothing_undrawn() {
    let mut doc = Document::new(4, 4); doc.layers = vec![pixel_layer()];
    assert!(doc.undrawn().is_empty());
}

#[test]
fn each_undrawn_feature_is_named_once_and_sorted() {
    let mut doc = Document::new(4, 4);
    let mut a = pixel_layer(); a.blend_mode = BlendMode::SoftLight;
    let mut b = pixel_layer(); b.blend_mode = BlendMode::SoftLight;
    b.extra.effects = Some(json!({ "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } }));
    let mut c = Layer::blank("BW", doc.size()); c.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::BlackWhite));
    let mut d = pixel_layer(); d.extra.unknown.insert("fromTheFuture".into(), json!(1));
    doc.layers = vec![a, b, c, d];
    assert_eq!(doc.undrawn(), vec![
        "Black & White adjustment layers".to_string(),
        "layer effects".to_string(),
        "settings from a newer version of Compositor".to_string(),
    ]);
}

#[test]
fn the_eleven_new_blend_modes_are_not_reported() {
    for mode in ["Linear Burn", "Linear Dodge (Add)", "Soft Light", "Hard Light", "Vivid Light", "Linear Light",
        "Pin Light", "Hard Mix", "Exclusion", "Subtract", "Divide"] {
        let mut doc = Document::new(4, 4);
        let mut layer = pixel_layer();
        layer.blend_mode = serde_json::from_value(json!(mode)).unwrap();
        doc.layers = vec![layer];
        assert!(doc.undrawn().is_empty(), "{mode}");
    }
}

#[test]
fn effects_that_are_all_switched_off_are_not_reported() {
    let mut doc = Document::new(4, 4);
    let mut a = pixel_layer();
    a.extra.effects = Some(json!({ "stroke": { "blue": 1, "enabled": false, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 4 } }));
    doc.layers = vec![a];
    assert!(doc.undrawn().is_empty(), "enabled: false hides the effect on the Mac too (R 2.1)");
}

#[test]
fn a_newer_document_level_setting_is_reported() {
    let mut doc = Document::new(4, 4); doc.layers = vec![pixel_layer()];
    doc.unknown.insert("fromTheFuture".into(), json!(1));
    assert_eq!(doc.undrawn(), vec!["settings from a newer version of Compositor".to_string()]);
}

#[test]
fn a_grain_layer_at_the_default_roughness_reports_nothing() {
    let mut doc = Document::new(4, 4);
    let mut g = Layer::blank("Grain", doc.size());
    g.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Grain)); // grainSettings absent, so roughness defaults to 50
    doc.layers = vec![g];
    assert!(doc.undrawn().is_empty(), "the 1.2.6 kernel is drawn");
}

#[test]
fn a_grain_layer_at_zero_roughness_reports_nothing() {
    let mut doc = Document::new(4, 4);
    let mut g = Layer::blank("Grain", doc.size());
    let mut adjustment = LayerAdjustment::new(AdjustmentKind::Grain);
    adjustment.grain_settings = Some(GrainSettings { roughness: 0.0, ..GrainSettings::default() });
    g.extra.adjustment = Some(adjustment);
    doc.layers = vec![g];
    assert!(doc.undrawn().is_empty());
}
