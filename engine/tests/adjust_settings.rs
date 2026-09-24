use compositor_engine::*;

/// The exact JSON Swift's JSONEncoder writes for `LayerAdjustment(kind: .levels)` (keys sorted).
const MAC_LEVELS: &str = r#"{"colorize":false,"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]},"hue":0,"kind":"Levels","levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255}]},"lightness":0,"saturation":0}"#;

#[test]
fn levels_adjustment_round_trips_the_mac_json_byte_for_byte() {
    let a: LayerAdjustment = serde_json::from_str(MAC_LEVELS).unwrap();
    assert_eq!(a.kind, AdjustmentKind::Levels);
    assert!(a.is_valid() && a.is_identity());
    let value = serde_json::to_value(&a).unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), MAC_LEVELS, "sorted keys, no optional settings written");
    assert!(a.hsv_settings.is_none() && a.exposure_settings.is_none() && a.gradient_map_settings.is_none() && a.grain_settings.is_none());
}

/// Reds hue 60, Blues saturation -100, Master lightness 12.5, default bands except Reds.
fn edited_hsv() -> HueSaturationSettings {
    let mut hsv = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Reds);
    hsv.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: 0.0, saturation: -100.0, lightness: 0.0 });
    hsv.adjustments.insert(ColorRange::Master, RangeAdjustment { hue: 0.0, saturation: 0.0, lightness: 12.5 });
    hsv.bands.insert(ColorRange::Reds, HueBand { falloff_start: 300.0, range_start: 330.0, range_end: 20.0, falloff_end: 50.0 });
    hsv
}

#[test]
fn legacy_hsv_fields_resolve_to_a_master_adjustment() {
    let legacy: LayerAdjustment = serde_json::from_str(r#"{"kind":"Hue/Saturation","hue":120,"saturation":0,"lightness":0,"colorize":false,"levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}]},"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]}}"#).unwrap();
    assert!(legacy.hsv_settings.is_none());
    assert_eq!(legacy.resolved_hsv().adjustment(ColorRange::Master).hue, 120.0);
}

/// The wasm bridge form (commands, `LayerState`, the render plan): the range maps are objects keyed
/// by range name, because the TS panels and `gl-renderer.ts` read `adjustments[range]`. Only the
/// manifest writes Swift's array form (next test). Whole numbers are written as integers, as Swift
/// does; a dropped `mac_number` on `RangeAdjustment` or `HueBand` writes `0.0` and fails here.
#[test]
fn the_bridge_form_keys_range_maps_by_name_with_mac_numbers() {
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    a.hsv_settings = Some(edited_hsv());
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.contains(r#""adjustments":{"Blues":{"hue":0,"lightness":0,"saturation":-100},"Master":{"hue":0,"lightness":12.5,"saturation":0},"Reds":{"hue":60,"lightness":0,"saturation":0}}"#), "{json}");
    assert!(json.contains(r#""bands":{"Blues":{"falloffEnd":285,"falloffStart":195,"rangeEnd":255,"rangeStart":225},"Cyans":"#), "{json}");
    let back: LayerAdjustment = serde_json::from_str(&json).unwrap();
    assert_eq!(back.resolved_hsv(), edited_hsv());
}

/// Swift encodes `[ColorRange: _]` (a `String` enum key that is not `CodingKeyRepresentable`) as
/// an unkeyed container of alternating key and value, and decodes only that. The manifest writes
/// it in `ColorRange::ALL` order, with integers for whole numbers.
#[test]
fn the_file_form_writes_range_maps_as_swift_key_value_arrays() {
    let mut doc = Document::new(4, 4);
    let mut layer = Layer::blank("Hue/Saturation", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    a.hsv_settings = Some(edited_hsv());
    layer.extra.adjustment = Some(a);
    doc.layers.push(layer);
    let text = doc.manifest().to_json_pretty().unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let hsv = &value["layers"][0]["adjustment"]["hsvSettings"];
    assert_eq!(serde_json::to_string(&hsv["adjustments"]).unwrap(),
        r#"["Master",{"hue":0,"lightness":12.5,"saturation":0},"Reds",{"hue":60,"lightness":0,"saturation":0},"Blues",{"hue":0,"lightness":0,"saturation":-100}]"#);
    assert_eq!(serde_json::to_string(&hsv["bands"]).unwrap(), concat!(
        r#"["Master",{"falloffEnd":360,"falloffStart":0,"rangeEnd":360,"rangeStart":0},"#,
        r#""Reds",{"falloffEnd":50,"falloffStart":300,"rangeEnd":20,"rangeStart":330},"#,
        r#""Yellows",{"falloffEnd":105,"falloffStart":15,"rangeEnd":75,"rangeStart":45},"#,
        r#""Greens",{"falloffEnd":165,"falloffStart":75,"rangeEnd":135,"rangeStart":105},"#,
        r#""Cyans",{"falloffEnd":225,"falloffStart":135,"rangeEnd":195,"rangeStart":165},"#,
        r#""Blues",{"falloffEnd":285,"falloffStart":195,"rangeEnd":255,"rangeStart":225},"#,
        r#""Magentas",{"falloffEnd":345,"falloffStart":255,"rangeEnd":315,"rangeStart":285}]"#));
    let parsed = Manifest::parse(&text).unwrap();
    assert_eq!(parsed.layers[0].adjustment.as_ref().unwrap().resolved_hsv(), edited_hsv());
}

#[test]
fn the_file_form_refuses_malformed_range_arrays() {
    let mut doc = Document::new(4, 4);
    let mut layer = Layer::blank("Hue/Saturation", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    a.hsv_settings = Some(HueSaturationSettings::default());
    layer.extra.adjustment = Some(a);
    doc.layers.push(layer);
    let text = doc.manifest().to_json_pretty().unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
    let hsv = value["layers"][0]["adjustment"]["hsvSettings"].clone();
    for (label, adjustments) in [
        ("odd length", serde_json::json!(["Master"])),
        ("non-string key", serde_json::json!([0, {"hue": 0, "lightness": 0, "saturation": 0}])),
        ("unknown range", serde_json::json!(["Oranges", {"hue": 0, "lightness": 0, "saturation": 0}])),
    ] {
        let mut broken = hsv.clone();
        broken["adjustments"] = adjustments;
        value["layers"][0]["adjustment"]["hsvSettings"] = broken;
        assert!(matches!(Manifest::parse(&value.to_string()), Err(ProjectError::Invalid)), "{label}");
    }
}

#[test]
fn a_non_finite_number_is_refused_rather_than_written_as_null() {
    let mut a = LayerAdjustment::new(AdjustmentKind::Exposure);
    a.exposure_settings = Some(ExposureSettings { exposure: f64::NAN, offset: 0.0, gamma: 1.0 });
    assert!(serde_json::to_string(&a).is_err());
    a.exposure_settings = Some(ExposureSettings { exposure: f64::INFINITY, offset: 0.0, gamma: 1.0 });
    assert!(serde_json::to_value(&a).is_err());
}

#[test]
fn optional_settings_appear_only_when_set_and_validate() {
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 40.0, size: 3.0, roughness: 10.0, seed: 9 });
    let json = serde_json::to_string(&grain).unwrap();
    assert!(json.contains(r#""grainSettings":{"amount":40"#) && !json.contains("exposureSettings"));
    let back: LayerAdjustment = serde_json::from_str(&json).unwrap();
    assert_eq!(back, grain);
    assert!(back.is_valid());
    let mut broken = LayerAdjustment::new(AdjustmentKind::Exposure);
    broken.exposure_settings = Some(ExposureSettings { exposure: 0.0, offset: 0.0, gamma: 0.0 });
    assert!(!broken.is_valid());
    let mut bad_levels = LayerAdjustment::new(AdjustmentKind::Levels);
    bad_levels.levels.ranges[0].black = 300.0;
    assert!(!bad_levels.is_valid(), "levels must be stored normalized");
}

#[test]
fn level_range_normalizes_like_the_mac() {
    let r = LevelRange { black: 300.0, gamma: f64::NAN, white: -1.0, output_black: -100.0, output_white: 400.0 }.normalized();
    assert!(r.black < r.white && r.gamma == 1.0 && r.output_black == 0.0 && r.output_white == 255.0);
    assert_eq!(LevelRange { black: 64.0, gamma: 1.0, white: 128.0, ..LevelRange::default() }.apply(96.0 / 255.0), 0.5);
    let mut s = LevelsSettings::default();
    s.ranges[LevelsChannel::Red.index()] = LevelRange { gamma: 2.0, ..LevelRange::default() };
    s.ranges[0] = LevelRange { black: 40.0, white: 210.0, ..LevelRange::default() };
    let expected = LevelRange { black: 40.0, white: 210.0, ..LevelRange::default() }.apply(LevelRange { gamma: 2.0, ..LevelRange::default() }.apply(64.0 / 255.0));
    assert!((s.apply(64.0 / 255.0, LevelsChannel::Red) - expected).abs() < 1e-12, "channel first, then RGB");
}

#[test]
fn hue_bands_ramp_through_falloff_and_wrap() {
    let reds = ColorRange::Reds.default_band();
    assert!(reds.weight(0.0) == 1.0 && reds.weight(345.0) == 1.0 && reds.weight(15.0) == 1.0);
    assert!((reds.weight(330.0) - 0.5).abs() < 0.001 && (reds.weight(30.0) - 0.5).abs() < 0.001);
    assert!(reds.weight(315.0) == 0.0 && reds.weight(45.0) == 0.0 && reds.weight(180.0) == 0.0);
    assert_eq!(ColorRange::Master.default_band().weight(123.0), 1.0);
    let mut band = ColorRange::Greens.default_band();
    band.set_handle(1, 200.0);
    assert_eq!(band, ColorRange::Greens.default_band(), "crossing moves are refused");
    band.set_handle(1, 110.0);
    assert_eq!(band.range_start, 110.0);
    let centered = ColorRange::Greens.default_band().centered_on(0.0);
    assert!(centered.weight(0.0) == 1.0 && centered.weight(120.0) == 0.0);
    let mut widened = centered; widened.include(240.0);
    assert!(widened.weight(240.0) == 1.0 && widened.weight(0.0) == 1.0);
    widened.exclude(240.0);
    assert_eq!(widened.weight(240.0), 0.0);
}

#[test]
fn curves_validate_and_interpolate_monotonically() {
    let c = CurvesSettings::default();
    assert!(c.is_valid() && c.is_identity());
    assert_eq!(c.value(100.0, 0), 100.0);
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(s.is_valid());
    let mut last = 0.0;
    for x in 0..=255 { let v = s.value(x as f64, 0); assert!(v >= last - 1e-9 && v <= 255.0, "monotone at {x}: {v} < {last}"); last = v; }
    assert!((s.value(128.0, 0) - 190.0).abs() < 1e-9);
    s.channels[1] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 0.0, y: 5.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(!s.is_valid(), "x must be strictly increasing");
}

#[test]
fn manifests_accept_adjustment_layers_only_when_well_formed() {
    let mut doc = Document::new(4, 4);
    let mut layer = Layer::blank("Levels", doc.size());
    layer.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Levels));
    doc.layers.push(layer);
    let json = doc.manifest().to_json_pretty().unwrap();
    assert!(json.contains("\"adjustment\": {") && json.contains("\"kind\": \"Levels\""));
    let parsed = Manifest::parse(&json).unwrap();
    assert_eq!(parsed.layers[0].adjustment.as_ref().unwrap().kind, AdjustmentKind::Levels);
    let group = json.replacen("\"isVisible\": true", "\"isVisible\": true, \"isGroup\": true", 1);
    assert!(matches!(Manifest::parse(&group), Err(ProjectError::Invalid)), "a folder cannot carry an adjustment");
    let old = json.replacen("\"version\": 9", "\"version\": 6", 1);
    assert!(matches!(Manifest::parse(&old), Err(ProjectError::Invalid)), "adjustments need version 7");
    let with_image = json.replacen("\"adjustment\": {", "\"imageFile\": \"X.png\", \"adjustment\": {", 1);
    assert!(matches!(Manifest::parse(&with_image), Err(ProjectError::Invalid)));
    let pkg = save_package(&doc).unwrap();
    let back = open_package(&pkg).unwrap();
    assert!(back.layers[0].is_adjustment() && back.layers[0].pixels.is_none());
    assert_eq!(back.layers[0].extra.adjustment, doc.layers[0].extra.adjustment);
}

/// The per-kind rule the panels ask for (wasm `adjustment_is_identity`), matching the Mac's
/// `commitFilter` skips (`Filters.swift:387-389`) and its Levels/Curves/HSV identities: Gradient
/// Map always applies, Grain only above amount 0, and the channel and range selectors are where
/// the panel is looking, not settings.
#[test]
fn identity_is_per_kind_and_ignores_the_selectors() {
    assert!(!LayerAdjustment::new(AdjustmentKind::GradientMap).is_identity(), "a gradient map always recolours");
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    assert!(!grain.is_identity(), "the default amount is 25");
    grain.grain_settings = Some(GrainSettings { amount: 0.0, ..GrainSettings::default() });
    assert!(grain.is_identity());
    assert!(LayerAdjustment::new(AdjustmentKind::Exposure).is_identity());
    let mut levels = LayerAdjustment::new(AdjustmentKind::Levels);
    levels.levels.channel = LevelsChannel::Red;
    assert!(levels.is_identity());
    let mut curves = LayerAdjustment::new(AdjustmentKind::Curves);
    curves.curves.channel = LevelsChannel::Blue;
    assert!(curves.is_identity());
    let mut hsv = LayerAdjustment::new(AdjustmentKind::Hsv);
    hsv.hsv_settings = Some(HueSaturationSettings::new(0.0, 0.0, 0.0, false, ColorRange::Reds));
    assert!(hsv.is_identity());
}