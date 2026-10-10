//! Independent Mac exports of generated sampling probes, not implementation mirrors.
use compositor_engine::*;
use std::path::{Path, PathBuf};
fn fixture(name: &str) -> (Document, PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cg-sampling-mac-1.2.10");
    let package = root.join(format!("{name}.comp"));
    let manifest_json = std::fs::read_to_string(package.join("manifest.json")).unwrap();
    let images = std::fs::read_dir(package.join("images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    (open_package(&Package { manifest_json, images }).unwrap(), root)
}
fn compare(name: &str) -> (u8, u8, f64) {
    let (doc, root) = fixture(name);
    let ours = composite(&doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height);
    let mac = decode_package_png(&std::fs::read(root.join(format!("{name}.mac-1.2.10.png"))).unwrap()).unwrap();
    assert_eq!((ours.width, ours.height), (mac.width, mac.height));
    let mut colour = 0; let mut alpha = 0; let mut total = 0u64;
    for (a, b) in ours.bytes().chunks_exact(4).zip(mac.bytes().chunks_exact(4)) {
        for c in 0..4 { let d = a[c].abs_diff(b[c]); total += d as u64;
            if c == 3 { alpha = alpha.max(d); } else { colour = colour.max(d); }
        }
    }
    (colour, alpha, total as f64 / ours.bytes().len() as f64)
}
#[test]
fn cg_rotated_rectangle_edges_match_the_independent_mac_export_within_a_byte() {
    let (colour, alpha, mean) = compare("sampling-high-rotated");
    assert!(colour <= 1 && alpha <= 1 && mean < 0.01, "colour {colour}, alpha {alpha}, mean {mean}");
}
#[test]
fn cg_affine_mask_phases_match_the_independent_mac_export() {
    assert_eq!(compare("sampling-high-mask-400"), (0, 0, 0.0));
}
#[test]
fn cg_reduction_phases_match_the_independent_mac_export_within_a_byte() {
    let (colour, alpha, mean) = compare("sampling-high-shrink-65");
    assert!(colour <= 1 && alpha == 0 && mean < 0.01, "colour {colour}, alpha {alpha}, mean {mean}");
}

#[test]
fn adding_whole_turns_preserves_the_complete_rotated_image_and_coverage() {
    let (mut doc, _) = fixture("sampling-high-rotated");
    let region = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    let expected = composite(&doc, region, doc.width, doc.height);
    assert_eq!(doc.layers[0].transform.rotation, 25.0);
    for angle in [385.0, 360_000_000_025.0, 360_000_000_000_025.0] {
        doc.layers[0].transform.rotation = angle;
        assert_eq!(composite(&doc, region, doc.width, doc.height).bytes(), expected.bytes(), "angle {angle}");
    }
}
