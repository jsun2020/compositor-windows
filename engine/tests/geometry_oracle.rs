use compositor_engine::{adjust::camera_raw::CameraRawSettings, Raster};
use serde::Deserialize;

#[derive(Deserialize)]
struct Ramp {
    #[serde(rename = "phase256")]
    phase: u32,
    red: Vec<u8>,
    alpha: Vec<u8>,
}

#[test]
fn geometry_matches_independent_core_image_gray_ramp_phases() {
    let cases: Vec<Ramp> =
        serde_json::from_str(include_str!("fixtures/core-image-geometry-ramp.json")).unwrap();
    assert_eq!(cases.len(), 15);
    let mut bytes = Vec::new();
    for _ in 0..4 {
        for level in 0..256 {
            bytes.extend([level as u8, level as u8, level as u8, 255]);
        }
    }
    let source = Raster::from_premultiplied(256, 4, bytes.clone());
    for case in cases {
        assert_eq!(case.red.len(), 256);
        assert_eq!(case.alpha.len(), 256);
        let mut settings = CameraRawSettings::default();
        settings.geometry.offset_x = (case.phase as f64 / 256.0) * 100.0 / (256.0 * 0.15);
        let output = settings.apply(&source).unwrap();
        for x in 0..256 {
            let v = case.red[x as usize];
            assert_eq!(
                output.pixel(x, 1),
                [v, v, v, case.alpha[x as usize]],
                "phase={}/256 x={}",
                case.phase,
                x
            );
        }
    }
    assert_eq!(source.bytes(), bytes);
}
