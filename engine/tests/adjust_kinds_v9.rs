use compositor_engine::*;
use serde_json::{json, Value};

fn with_adjustment(version: u32, adjustment: Value) -> Package {
    let manifest = json!({
        "format": "com.compositor.project", "version": version, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 5, "height": 4,
        "layers": [{ "id": "0B6C6B1E-4F1B-4B4E-9E0A-555555555555", "name": "Adj", "isVisible": true,
            "transform": { "origin": [0, 0], "size": [5, 4], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
            "adjustment": adjustment }]
    });
    Package { manifest_json: manifest.to_string(), images: vec![] }
}

/// The fields every LayerAdjustment always writes (non-optional with defaults, R 3.2).
fn base(kind: &str) -> Value {
    json!({ "kind": kind, "hue": 0, "saturation": 0, "lightness": 0, "colorize": false,
        "levels": { "channel": "RGB", "ranges": [ {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
            {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}, {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
            {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255} ] },
        "curves": { "channel": "RGB", "channels": [ [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}],
            [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}] ] } })
}

fn saved_adjustment(p: &Package) -> Value {
    let doc = open_package(p).unwrap();
    let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
    saved["layers"][0]["adjustment"].clone()
}

#[test]
fn every_new_kind_string_round_trips() {
    for kind in ["Add Noise", "Gaussian Blur", "Motion Blur", "Invert", "Black & White", "Color Balance"] {
        assert_eq!(saved_adjustment(&with_adjustment(9, base(kind)))["kind"], json!(kind), "{kind}");
    }
}

#[test]
fn only_blur_and_noise_need_version_9() {
    for kind in ["Gaussian Blur", "Motion Blur", "Add Noise"] {
        assert!(matches!(open_package(&with_adjustment(8, base(kind))), Err(ProjectError::Invalid)), "{kind} at v8");
    }
    for kind in ["Invert", "Black & White", "Color Balance"] {
        assert!(open_package(&with_adjustment(7, base(kind))).is_ok(), "{kind} at v7 (1.2.2 wrote these into v8 files, R 1.1)");
    }
}

#[test]
fn black_and_white_and_colour_balance_write_every_field_as_the_mac_does() {
    let mut a = base("Black & White");
    a["blackWhiteSettings"] = json!({ "blues": 20, "cyans": 60, "greens": 40, "magentas": 80, "reds": 35.5,
        "tint": true, "tintHue": 40, "tintSaturation": 20, "yellows": 60 });
    assert_eq!(saved_adjustment(&with_adjustment(9, a.clone()))["blackWhiteSettings"], a["blackWhiteSettings"]);
    let mut b = base("Color Balance");
    b["colorBalanceSettings"] = json!({ "highlightCyanRed": 10, "highlightMagentaGreen": -5, "highlightYellowBlue": 0,
        "midCyanRed": 0, "midMagentaGreen": 0, "midYellowBlue": 33, "preserveLuminosity": false,
        "shadowCyanRed": 0, "shadowMagentaGreen": 0, "shadowYellowBlue": -100 });
    assert_eq!(saved_adjustment(&with_adjustment(9, b.clone()))["colorBalanceSettings"], b["colorBalanceSettings"]);
}

#[test]
fn flat_blur_and_noise_fields_round_trip_and_are_omitted_when_absent() {
    let mut a = base("Add Noise");
    a["noiseAmount"] = json!(12.5); a["noiseGaussian"] = json!(true); a["noiseMonochromatic"] = json!(false);
    a["noiseSeed"] = json!(4294967295u64);
    // base() is exactly what the port writes, so the WHOLE saved object must equal the input: a
    // misspelled rename would drop a key the Mac writes, and this comparison catches it.
    let out = saved_adjustment(&with_adjustment(9, a.clone()));
    assert_eq!(out, a, "every Add Noise key survives, spelled as the Mac spells it");
    assert!(out.get("blurRadius").is_none(), "absent optionals stay absent, never null");
    let mut m = base("Motion Blur"); m["motionAngle"] = json!(-30.5); m["motionDistance"] = json!(40);
    assert_eq!(saved_adjustment(&with_adjustment(9, m.clone())), m, "every Motion Blur key survives");
    let mut g = base("Gaussian Blur"); g["blurRadius"] = json!(24);
    let text = save_package(&open_package(&with_adjustment(9, g)).unwrap()).unwrap().manifest_json;
    assert!(text.contains("\"blurRadius\": 24,") || text.contains("\"blurRadius\": 24\n"), "24, not 24.0");
}

#[test]
fn a_noise_seed_outside_u32_makes_the_file_invalid() {
    for seed in [json!(-1), json!(4294967296u64)] {
        let mut a = base("Add Noise"); a["noiseSeed"] = seed;
        assert!(matches!(open_package(&with_adjustment(9, a)), Err(ProjectError::Invalid)));
    }
}

#[test]
fn the_new_ranges_are_checked_on_every_adjustment_whatever_its_kind() {
    // The Mac validates the RESOLVED values of every adjustment (LayerAdjustment.swift:136-140), so
    // even a Levels layer carrying an out-of-range blurRadius is refused.
    let cases = [("Levels", "blurRadius", json!(0.05)), ("Motion Blur", "motionAngle", json!(91)),
        ("Motion Blur", "motionDistance", json!(0.5)), ("Add Noise", "noiseAmount", json!(401))];
    for (kind, key, value) in cases {
        let mut a = base(kind); a[key] = value;
        assert!(matches!(open_package(&with_adjustment(9, a)), Err(ProjectError::Invalid)), "{kind} {key}");
    }
    let mut bw = base("Black & White");
    bw["blackWhiteSettings"] = json!({ "blues": 20, "cyans": 60, "greens": 40, "magentas": 80, "reds": 301,
        "tint": false, "tintHue": 40, "tintSaturation": 20, "yellows": 60 });
    assert!(matches!(open_package(&with_adjustment(9, bw)), Err(ProjectError::Invalid)), "B&W weight above 300");
    for (key, value) in [("tintHue", json!(361)), ("tintSaturation", json!(101))] {
        let mut t = base("Black & White");
        t["blackWhiteSettings"] = json!({ "blues": 20, "cyans": 60, "greens": 40, "magentas": 80, "reds": 40,
            "tint": true, "tintHue": 40, "tintSaturation": 20, "yellows": 60 });
        t["blackWhiteSettings"][key] = value;
        assert!(matches!(open_package(&with_adjustment(9, t)), Err(ProjectError::Invalid)), "B&W {key}");
    }
    let mut cb = base("Color Balance");
    cb["colorBalanceSettings"] = json!({ "highlightCyanRed": 0, "highlightMagentaGreen": 0, "highlightYellowBlue": 0,
        "midCyanRed": 101, "midMagentaGreen": 0, "midYellowBlue": 0, "preserveLuminosity": true,
        "shadowCyanRed": 0, "shadowMagentaGreen": 0, "shadowYellowBlue": 0 });
    assert!(matches!(open_package(&with_adjustment(9, cb)), Err(ProjectError::Invalid)), "Color Balance above 100");
}

#[test]
fn absent_settings_resolve_to_the_mac_defaults() {
    let a = LayerAdjustment::new(AdjustmentKind::BlackWhite);
    let bw = a.black_white();
    assert_eq!((bw.reds, bw.yellows, bw.greens, bw.cyans, bw.blues, bw.magentas), (40.0, 60.0, 40.0, 60.0, 20.0, 80.0),
        "an unedited B&W layer is Photoshop's default mix, not identity (R 3.2)");
    assert_eq!((a.gaussian_radius(), a.motion_angle_degrees(), a.motion_distance_pixels(), a.noise_amount_percent()), (10.0, 0.0, 10.0, 10.0));
    assert!(!a.is_identity());
    assert!(LayerAdjustment::new(AdjustmentKind::ColorBalance).is_identity(), "all-zero Color Balance changes nothing");
    let mut cb = LayerAdjustment::new(AdjustmentKind::ColorBalance);
    cb.color_balance_settings = Some(ColorBalanceSettings { mid_yellow_blue: 33.0, ..Default::default() });
    assert!(!cb.is_identity(), "a non-zero Color Balance changes the image");
}
