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

#[test]
fn legacy_hsv_fields_resolve_and_range_settings_serialize_with_string_keys() {
    let legacy: LayerAdjustment = serde_json::from_str(r#"{"kind":"Hue/Saturation","hue":120,"saturation":0,"lightness":0,"colorize":false,"levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}]},"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]}}"#).unwrap();
    assert!(legacy.hsv_settings.is_none());
    assert_eq!(legacy.resolved_hsv().adjustment(ColorRange::Master).hue, 120.0);
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    let mut hsv = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Reds);
    hsv.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: 0.0, saturation: -100.0, lightness: 0.0 });
    a.hsv_settings = Some(hsv.clone());
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.contains(r#""adjustments":{"Blues":{"hue":0.0,"lightness":0.0,"saturation":-100.0},"Reds":"#) || json.contains(r#""adjustments":{"Blues":{"hue":0,"lightness":0,"saturation":-100},"Reds":"#), "{json}");
    assert!(json.contains(r#""bands":{"Blues":{"falloffEnd":285"#) || json.contains(r#""bands":{"Blues":{"falloffEnd":285.0"#), "{json}");
    let back: LayerAdjustment = serde_json::from_str(&json).unwrap();
    assert_eq!(back.resolved_hsv(), hsv);
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
    let old = json.replacen("\"version\": 7", "\"version\": 6", 1);
    assert!(matches!(Manifest::parse(&old), Err(ProjectError::Invalid)), "adjustments need version 7");
    let with_image = json.replacen("\"adjustment\": {", "\"imageFile\": \"X.png\", \"adjustment\": {", 1);
    assert!(matches!(Manifest::parse(&with_image), Err(ProjectError::Invalid)));
    let pkg = save_package(&doc).unwrap();
    let back = open_package(&pkg).unwrap();
    assert!(back.layers[0].is_adjustment() && back.layers[0].pixels.is_none());
    assert_eq!(back.layers[0].extra.adjustment, doc.layers[0].extra.adjustment);
}
