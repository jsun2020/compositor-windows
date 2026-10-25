use compositor_engine::{adjust::camera_raw::CameraRawSettings, retouch::{self, HealingMode}, Raster, GrayRaster};
use std::path::PathBuf;
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mac157").join(name)).unwrap()
}
#[test]
fn patchmatch_and_healing_match_frozen_mac157_c_reference() {
    for case in ["stripes", "ridge", "alpha", "empty", "all-selected"] {
        let raster=Raster::from_premultiplied(96,64,fixture(&format!("{case}-input.rgba")));
        let mask=GrayRaster::from_bytes(96,64,fixture(&format!("{case}-mask.gray")));
        let filled=retouch::fill(&raster,&mask);
        if case=="all-selected" { assert!(filled.is_err()); }
        else { assert_eq!(filled.unwrap().bytes(),fixture(&format!("{case}-fill.rgba")),"{case} fill"); }
        for (i,mode) in [HealingMode::ContentAware,HealingMode::CreateTexture,HealingMode::ProximityMatch].into_iter().enumerate() {
            let result=retouch::heal(&raster,&mask,1.,mode,7).unwrap();
            assert_eq!(result.bytes(),fixture(&format!("{case}-{i}.rgba")),"{case} mode {i}");
            for (pixel, &coverage) in mask.bytes().iter().enumerate() {
                if coverage==0 { assert_eq!(&result.bytes()[pixel*4..pixel*4+4], &raster.bytes()[pixel*4..pixel*4+4]); }
            }
        }
    }
}
#[test]
fn shadows_matches_independent_mac157_reference_at_both_layer_size_regimes() {
    let raster=Raster::from_premultiplied(96,64,fixture("shadows-input.rgba"));
    for set in 0..2 {
        for amount in [-100.,-50.,-12.5,25.,50.,75.,100.] {
            let s=CameraRawSettings { shadows:amount, pixel_scale:if set==0 {1.} else {0.05}, ..Default::default() };
            let result=s.apply(&raster).unwrap();
            assert_eq!(result.bytes(),fixture(&format!("shadows-{set}-{amount}.rgba")),"set {set}, amount {amount}");
        }
    }
    assert_eq!(CameraRawSettings::default().apply(&raster).unwrap(),raster);
}
