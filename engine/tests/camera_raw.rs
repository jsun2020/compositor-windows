use compositor_engine::{
    adjust::{camera_raw::*, settings::CurvePoint},
    Raster,
};

fn rgba(values: &[[u8; 4]]) -> Raster {
    Raster::from_straight(values.len() as u32, 1, &values.concat())
}
fn gray(v: u8) -> Raster {
    rgba(&[[v, v, v, 255]])
}
#[test]
fn identity_keeps_exact_premultiplied_pixels_and_ignores_idle_controls() {
    let image = rgba(&[[128, 70, 200, 128], [0, 0, 0, 0], [44, 80, 20, 255]]);
    let mut s = CameraRawSettings::default();
    assert!(s.is_identity());
    assert_eq!(s.apply(&image).unwrap().bytes(), image.bytes());
    s.glow_spread = 80.0;
    s.detail.sharpen_radius = 90.0;
    s.optics.profile_distortion = 20.0;
    s.grading.global.hue = 90.0;
    s.calibration.process = ProcessVersion::Version1;
    assert!(s.is_identity());
    assert_eq!(s.apply(&image).unwrap().bytes(), image.bytes());
}
#[test]
fn upstream_exposure_and_contrast_oracles() {
    let mut s = CameraRawSettings::default();
    s.exposure = 1.0;
    let p = s.apply(&gray(128)).unwrap().pixel(0, 0);
    assert!(p[0].abs_diff(176) <= 2);
    assert_eq!(p, [p[0], p[0], p[0], 255]);
    s = CameraRawSettings::default();
    s.contrast = 100.0;
    let p = s
        .apply(&rgba(&[[64, 64, 64, 255], [192, 192, 192, 255]]))
        .unwrap();
    assert!(p.pixel(0, 0)[0] < 10 && p.pixel(1, 0)[0] > 250);
    s.contrast = -100.0;
    let p = s
        .apply(&rgba(&[[64, 64, 64, 255], [192, 192, 192, 255]]))
        .unwrap();
    assert!(p.pixel(0, 0)[0].abs_diff(128) <= 2 && p.pixel(1, 0)[0].abs_diff(128) <= 2);
}
#[test]
fn white_balance_gains_and_alpha_are_preserved() {
    let mut s = CameraRawSettings::default();
    s.temperature = 100.0;
    let p = s.apply(&gray(128)).unwrap().pixel(0, 0);
    assert!(p[0] > 128 && p[2] < 128);
    s.temperature = 0.0;
    s.tint = 100.0;
    let p = s.apply(&gray(128)).unwrap().pixel(0, 0);
    assert!(p[1] < 128 && p[1] < p[0] && p[1] < p[2]);
    let image = rgba(&[[120, 60, 100, 128], [10, 20, 30, 0], [40, 50, 60, 255]]);
    let out = s.apply(&image).unwrap();
    for i in 0..3 {
        assert_eq!(out.pixel(i, 0)[3], image.pixel(i, 0)[3]);
        let p = out.pixel(i, 0);
        assert!(p[..3].iter().all(|v| *v <= p[3]));
    }
}
#[test]
fn clipping_is_a_preview_and_does_not_mutate_source() {
    let image = rgba(&[[230, 230, 230, 255], [128, 128, 128, 255]]);
    let original = image.bytes().to_vec();
    let mut s = CameraRawSettings::default();
    s.whites = 100.0;
    let out = s.apply_preview(&image, 1, -1, false, false, false).unwrap();
    assert_eq!(out.pixel(0, 0), [255; 4]);
    assert_eq!(out.pixel(1, 0), [0, 0, 0, 255]);
    assert_eq!(image.bytes(), original);
    let normal = s.apply(&image).unwrap();
    assert!(normal.pixel(1, 0)[0].abs_diff(128) <= 2);
}
#[test]
fn parametric_curve_is_monotone_and_uses_upstream_bend() {
    let mut s = CameraRawSettings::default();
    s.curve.darks = -51.0;
    let t = s.curve.tone_table();
    assert!((t[64] as f64 - 0.15).abs() < 0.025);
    assert_eq!(t[0], 0.0);
    assert_eq!(t[255], 1.0);
    assert!(t.windows(2).all(|p| p[0] <= p[1]));
    s.curve.rgb = vec![
        CurvePoint { x: 0.0, y: 0.0 },
        CurvePoint { x: 0.5, y: 0.8 },
        CurvePoint { x: 1.0, y: 1.0 },
    ];
    let out = s.apply(&gray(128)).unwrap();
    assert!(out.pixel(0, 0)[0] > 128);
}
#[test]
fn settings_normalization_repairs_nonfinite_ranges_and_curves() {
    let mut s = CameraRawSettings::default();
    s.exposure = f64::NAN;
    s.temperature = 400.0;
    s.curve.shadow_split = 90.0;
    s.curve.dark_split = -10.0;
    s.curve.light_split = -10.0;
    s.curve.rgb = vec![];
    s.optics.purple_hue_low = 350.0;
    s.optics.purple_hue_high = 200.0;
    let n = s.normalized();
    assert_eq!(n.exposure, 0.0);
    assert_eq!(n.temperature, 100.0);
    assert!(
        n.curve.dark_split >= n.curve.shadow_split + 2.0
            && n.curve.light_split >= n.curve.dark_split + 2.0
    );
    assert_eq!(n.curve.rgb.len(), 2);
    assert!(n.optics.purple_hue_low <= n.optics.purple_hue_high);
    assert_eq!(n, n.normalized());
}
#[test]
fn auto_balance_solves_the_same_linear_gains() {
    let (r, g, b) = (0.2, 0.3, 0.4);
    let (temp, tint) = CameraRawSettings::neutralize(r, g, b).unwrap();
    let red = r * (1.0 + 0.35 * temp / 100.0 + 0.15 * tint / 100.0);
    let green = g * (1.0 - 0.30 * tint / 100.0);
    let blue = b * (1.0 - 0.35 * temp / 100.0 + 0.15 * tint / 100.0);
    assert!((red - green).abs() < 1e-12 && (green - blue).abs() < 1e-12);
    assert!(CameraRawSettings::auto_balance(&rgba(&[[0, 0, 0, 0]])).is_none());
}
#[test]
fn all_pixel_groups_keep_valid_storage_and_grain_is_deterministic() {
    let data: Vec<u8> = (0..32 * 32)
        .flat_map(|i| {
            [
                (i % 256) as u8,
                ((i * 7) % 256) as u8,
                ((i * 13) % 256) as u8,
                if i % 5 == 0 { 128 } else { 255 },
            ]
        })
        .collect();
    let image = Raster::from_straight(32, 32, &data);
    let mut s = CameraRawSettings::default();
    s.exposure = 0.2;
    s.texture = 25.0;
    s.clarity = 30.0;
    s.dehaze = 10.0;
    s.glow = 15.0;
    s.grain_amount = 12.0;
    s.seed = 17;
    s.vignette_amount = -20.0;
    s.mixer.hue[0] = 10.0;
    s.grading.shadows.saturation = 15.0;
    s.detail.sharpen_amount = 30.0;
    s.detail.noise_color = 15.0;
    s.optics.purple_amount = 20.0;
    s.calibration.red_hue = 10.0;
    let a = s.apply(&image).unwrap();
    let b = s.apply(&image).unwrap();
    assert_eq!(a.bytes(), b.bytes());
    assert_ne!(a.bytes(), image.bytes());
    for (p, q) in a.bytes().chunks_exact(4).zip(image.bytes().chunks_exact(4)) {
        assert_eq!(p[3], q[3]);
        assert!(p[..3].iter().all(|v| *v <= p[3]));
    }
}
#[test]
fn geometry_changes_pixels_without_resizing_and_refuses_collapsed_output() {
    let image = Raster::from_straight(
        8,
        8,
        &(0..64)
            .flat_map(|i| [i * 3, 20, 40, 255])
            .collect::<Vec<_>>(),
    );
    let mut s = CameraRawSettings::default();
    s.geometry.rotate = 15.0;
    let out = s.apply(&image).unwrap();
    assert_eq!((out.width, out.height), (8, 8));
    assert_ne!(out.bytes(), image.bytes());
    s.geometry.scale = -100.0;
    assert!(s.apply(&image).is_err());
}
#[test]
fn serialized_filter_settings_roundtrip_and_reject_unknown_keys() {
    let s = CameraRawSettings::default();
    let json = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<CameraRawSettings>(&json).unwrap(), s);
    assert!(serde_json::from_str::<CameraRawSettings>(r#"{"bogus":1}"#).is_err());
}
