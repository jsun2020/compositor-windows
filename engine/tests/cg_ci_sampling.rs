//! Whole-image references returned independently by Compositor 1.4.5 on Mac.
use compositor_engine::*;
use serde::Deserialize;
use std::{collections::HashMap, path::{Path, PathBuf}};

#[derive(Deserialize)]
struct Case { name: String, operation: String, package: String, sampling: Option<Sampling>, corners: Option<[[f64; 2]; 4]> }
#[derive(Deserialize)]
struct Receipt { #[serde(rename="caseCount")] cases: usize, records: Vec<Record> }
#[derive(Deserialize)]
struct Record { name: String, width: u32, height: u32, channels: usize, bytes: usize, transform: Option<LayerTransform> }
fn fixture() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cg-ci-sampling-mac-1.4.5") }
fn document(root: &Path, package: &str) -> Document {
    let path = root.join(package);
    let manifest_json = std::fs::read_to_string(path.join("manifest.json")).unwrap();
    let images = std::fs::read_dir(path.join("images")).unwrap().map(|e| {
        let e = e.unwrap(); (e.file_name().into_string().unwrap(), std::fs::read(e.path()).unwrap())
    }).collect();
    open_package(&Package { manifest_json, images }).unwrap()
}
fn compare(root: &Path, reference: &Record, width: u32, height: u32, channels: usize, bytes: &[u8], placed: Option<LayerTransform>) {
    assert_eq!((width, height, channels, bytes.len()), (reference.width, reference.height, reference.channels, reference.bytes), "{} shape", reference.name);
    assert_eq!(placed, reference.transform, "{} saved transform", reference.name);
    let extension = if channels == 4 { "rgba8" } else { "gray8" };
    let expected = std::fs::read(root.join(format!("mac/{}.{}", reference.name, extension))).unwrap();
    assert_eq!(expected.len(), bytes.len());
    // CG's direct clip byte mask and CI's L8 conversion are separately rounded.
    // Keep ordinary affine images and every perspective RGBA record exact.
    let clip_rounding = reference.name.starts_with("mask-") || channels == 1;
    if !clip_rounding { assert_eq!(bytes, expected, "{} complete raw bytes", reference.name); }
    else {
        let max = bytes.iter().zip(&expected).map(|(a,b)| a.abs_diff(*b)).max().unwrap();
        let total: usize = bytes.iter().zip(&expected).map(|(a,b)| a.abs_diff(*b) as usize).sum();
        let mean = total as f64 / bytes.len() as f64;
        assert!(max <= 1, "{} maximum error {max}", reference.name);
        if channels == 4 { assert!(mean < 0.01, "{} mean error {mean}", reference.name); }
    }
}
#[test]
fn all_42_fixed_mac_cases_match_complete_images_masks_and_placed_transforms() {
    let root = fixture();
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(root.join("cases.json")).unwrap()).unwrap();
    let receipt: Receipt = serde_json::from_slice(&std::fs::read(root.join("mac/result.json")).unwrap()).unwrap();
    assert_eq!(cases.len(), 42); assert_eq!(receipt.cases, 42); assert_eq!(receipt.records.len(), 44);
    let references: HashMap<_,_> = receipt.records.iter().map(|r| (r.name.as_str(), r)).collect();
    assert_eq!(references.len(), 44);
    let mut compared = 0;
    for case in cases {
        let doc = document(&root, &case.package);
        match case.operation.as_str() {
            "layer" => {
                let output = composite(&doc, Rect { x:0.0, y:0.0, width:doc.width as f64, height:doc.height as f64 }, doc.width, doc.height);
                compare(&root, references[case.name.as_str()], output.width, output.height, 4, output.bytes(), None);
                compared += 1;
            }
            "warp" => {
                let layer = &doc.layers[0]; let mut transform = layer.transform;
                transform.sampling = case.sampling.unwrap();
                let corners = case.corners.unwrap().map(|[x,y]| Point { x, y });
                let (output, placed) = ops::distort::warp(layer.pixels.as_ref().unwrap(), &transform, &corners, transform.sampling == Sampling::Nearest).unwrap();
                compare(&root, references[case.name.as_str()], output.width, output.height, 4, output.bytes(), Some(placed));
                compared += 1;
                if let Some(mask) = &layer.mask {
                    let (output, mask_placed) = ops::distort::warp_mask(&mask.pixels, &transform, &corners, mask.background()).unwrap();
                    let name = format!("{}-mask", case.name);
                    assert_eq!(mask_placed, placed, "{} mask follows the colour warp", case.name);
                    compare(&root, references[name.as_str()], output.width, output.height, 1, output.bytes(), None);
                    compared += 1;
                }
            }
            _ => panic!("unknown fixed operation"),
        }
    }
    assert_eq!(compared, 44);
}
