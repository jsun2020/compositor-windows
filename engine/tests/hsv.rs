use compositor_engine::*;

fn straight(raster: &Raster, index: usize) -> [i64; 4] {
    let d = &raster.bytes()[index * 4..index * 4 + 4];
    let a = d[3] as i64;
    if a == 0 { return [0, 0, 0, 0]; }
    [(d[0] as i64 * 255 + a / 2) / a, (d[1] as i64 * 255 + a / 2) / a, (d[2] as i64 * 255 + a / 2) / a, a]
}
fn near(v: [i64; 4], t: [i64; 4]) -> bool { v.iter().zip(t.iter()).all(|(a, b)| (a - b).abs() <= 2) }
/// Red, mid gray, pure blue, and a half-transparent blue.
fn fixture() -> Raster { Raster::from_premultiplied(4, 1, vec![255,0,0,255, 128,128,128,255, 0,0,255,255, 0,0,128,128]) }

#[test]
fn hue_rotates_saturation_and_lightness_follow_photoshop() {
    let out = apply_hsv(&fixture(), &HueSaturationSettings::new(120.0, 0.0, 0.0, false, ColorRange::Master));
    assert!(near(straight(&out, 0), [0, 255, 0, 255]), "red to green: {:?}", straight(&out, 0));
    assert!(near(straight(&out, 1), [128, 128, 128, 255]), "gray is unchanged: {:?}", straight(&out, 1));
    let flat = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, -100.0, 0.0, false, ColorRange::Master));
    let gray = straight(&flat, 0);
    assert!(gray[0] == gray[1] && gray[1] == gray[2] && gray[3] == 255, "{gray:?}");
    let white = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, 0.0, 100.0, false, ColorRange::Master));
    assert!(near(straight(&white, 0), [255, 255, 255, 255]));
    let black = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, 0.0, -100.0, false, ColorRange::Master));
    assert!(near(straight(&black, 0), [0, 0, 0, 255]));
}

#[test]
fn identity_settings_change_nothing_and_alpha_is_kept() {
    let source = fixture();
    assert_eq!(apply_hsv(&source, &HueSaturationSettings::default()).bytes(), source.bytes());
    let colorized = apply_hsv(&source, &HueSaturationSettings::new(240.0, 100.0, 0.0, true, ColorRange::Master));
    let strip = straight(&colorized, 3);
    assert_eq!(strip[3], 128, "half-transparent pixels keep their alpha");
    for i in 0..3 { let p = straight(&colorized, i); assert!(p[2] > p[0], "everything turns blue-ish: {p:?}"); }
}

#[test]
fn ranges_adjust_independently_and_invert_flips_the_band() {
    let mut s = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Reds);
    s.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: 0.0, saturation: -100.0, lightness: 0.0 });
    let out = apply_hsv(&fixture(), &s);
    assert!(near(straight(&out, 0), [255, 255, 0, 255]), "reds rotate to yellow: {:?}", straight(&out, 0));
    let blue = straight(&out, 2);
    assert!(blue[0] == blue[1] && blue[1] == blue[2], "blues desaturate: {blue:?}");
    let reds_only = HueSaturationSettings::new(0.0, 0.0, -100.0, false, ColorRange::Reds);
    let out = apply_hsv(&fixture(), &reds_only);
    assert!(near(straight(&out, 0), [0, 0, 0, 255]) && near(straight(&out, 2), [0, 0, 255, 255]));
    let mut inverted = reds_only.clone();
    inverted.invert_range = true;
    let out = apply_hsv(&fixture(), &inverted);
    assert!(near(straight(&out, 0), [255, 0, 0, 255]) && near(straight(&out, 2), [0, 0, 0, 255]));
}

#[test]
fn the_response_table_and_after_bar_follow_hue_shifts() {
    let mut greens = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Greens);
    greens.adjustments.insert(ColorRange::Master, RangeAdjustment::default());
    assert!((shifted_hue(120.0, &greens) - 180.0).abs() < 0.001);
    assert!((shifted_hue(0.0, &greens) - 0.0).abs() < 0.001);
    let response = hue_response(&greens);
    assert_eq!(response.len(), 361);
    assert!((response[120][0] - 60.0).abs() < 0.001 && response[0][0] == 0.0);
}

#[test]
fn hsl_round_trips() {
    for rgb in [[1.0, 0.0, 0.0], [0.2, 0.7, 0.4], [0.5, 0.5, 0.5], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]] {
        let back = hsl_to_rgb(rgb_to_hsl(rgb));
        for c in 0..3 { assert!((back[c] - rgb[c]).abs() < 1e-9, "{rgb:?} -> {back:?}"); }
    }
}
