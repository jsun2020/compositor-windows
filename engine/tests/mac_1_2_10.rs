//! The Mac's own renders of the probe projects this port wrote (Compositor 1.2.10, exported at 100%
//! as PNG), committed under tests/fixtures/mac-1.2.10-probes (probe results). Each test opens the
//! probe as the Mac did and compares this port's composite with the Mac PNG in straight RGBA8, the
//! form the probe results compared.
use compositor_engine::*;
use std::ops::Range;

fn fixtures() -> String { format!("{}/tests/fixtures/mac-1.2.10-probes", env!("CARGO_MANIFEST_DIR")) }

/// This port's composite of the probe, straight RGBA8.
fn ours(name: &str) -> Vec<u8> {
    let comp = format!("{}/{name}.comp", fixtures());
    let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    let doc = open_package(&Package { manifest_json, images }).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let region = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    composite(&doc, region, doc.width, doc.height).to_straight()
}

/// The Mac's export: width, and straight RGBA8 as the PNG stores it.
fn mac(name: &str) -> (u32, Vec<u8>) {
    let png = std::fs::read(format!("{}/{name}.mac-1.2.10.png", fixtures())).unwrap();
    let rgba = image::load_from_memory(&png).unwrap().to_rgba8();
    (rgba.width(), rgba.into_raw())
}

/// The largest per-channel difference within `columns`, and where it is.
fn worst(a: &[u8], b: &[u8], width: u32, columns: Range<u32>) -> (u8, (u32, u32)) {
    assert_eq!(a.len(), b.len(), "the port and the Mac render the same size");
    let height = a.len() as u32 / 4 / width;
    let mut out = (0u8, (0, 0));
    for y in 0..height { for x in columns.clone() {
        let i = ((y * width + x) * 4) as usize;
        for c in 0..4 { let d = a[i + c].abs_diff(b[i + c]); if d > out.0 { out = (d, (x, y)); } }
    }}
    out
}

#[test]
fn the_new_blend_modes_match_the_mac_render() {
    // Eleven 20-px columns, one per mode in blend_modes_v9.rs NEW order; Soft Light is column 2.
    let (width, theirs) = mac("new-blend-modes");
    let port = ours("new-blend-modes");
    let (d, at) = worst(&port, &theirs, width, 0..40);
    assert_eq!(d, 0, "Linear Burn and Linear Dodge, worst at {at:?}");
    let (d, at) = worst(&port, &theirs, width, 60..width);
    assert_eq!(d, 0, "Hard Light to Divide, worst at {at:?}");
    let (d, at) = worst(&port, &theirs, width, 40..60);
    assert!(d <= 1, "Soft Light within one level: {d} at {at:?}");
}

#[test]
fn grain_at_its_defaults_matches_the_mac_render_exactly() {
    // The probe results found the 1.2.6 kernel equal to the Mac on all 7200 pixels.
    let (width, theirs) = mac("grain");
    let (d, at) = worst(&ours("grain"), &theirs, width, 0..width);
    assert_eq!(d, 0, "worst at {at:?}");
}

#[test]
fn the_probes_this_port_already_matched_still_match() {
    for name in ["folder-opacity", "clipped-in-dimmed-folder", "guides"] {
        let (width, theirs) = mac(name);
        let (d, at) = worst(&ours(name), &theirs, width, 0..width);
        assert_eq!(d, 0, "{name}: worst at {at:?}");
    }
}
