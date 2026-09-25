use compositor_engine::*;
use serde_json::json;

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in bytes { c ^= b as u32; for _ in 0..8 { c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 }; } }
    !c
}
fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut body = kind.to_vec(); body.extend_from_slice(data);
    let mut out = (data.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
    out
}
/// A PNG whose header claims `width` x `height` (colour type 6 RGBA, or 0 grey for a mask) but whose
/// data is one empty zlib block: a reader that sizes images from their headers before decoding
/// refuses it for its size; one that decodes first fails with MissingImage instead.
fn claimed_png(width: u32, height: u32, colour: u8) -> Vec<u8> {
    let mut ihdr = width.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, colour, 0, 0, 0]);
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    png.extend(chunk(b"IHDR", &ihdr));
    png.extend(chunk(b"IDAT", &[0x78, 0x01, 0x01, 0x00, 0x00, 0xff, 0xff, 0x00, 0x00, 0x00, 0x01]));
    png.extend(chunk(b"IEND", &[]));
    png
}

const A: &str = "0B6C6B1E-4F1B-4B4E-9E0A-AAAAAAAAAAA1";
const B: &str = "0B6C6B1E-4F1B-4B4E-9E0A-AAAAAAAAAAA2";

/// Two pixel layers with claimed images of the given sizes; `mask` puts a claimed grey mask of that
/// size on layer A.
fn package(a: (u32, u32), b: (u32, u32), mask: Option<(u32, u32)>) -> Package {
    let transform = json!({ "origin": [0, 0], "size": [100, 100], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" });
    let mut first = json!({ "id": A, "name": "A", "isVisible": true, "imageFile": format!("{A}.png"), "transform": transform });
    if mask.is_some() { first["maskFile"] = json!(format!("{A}.mask.png")); }
    let manifest = json!({ "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-AAAAAAAAAAA0", "width": 100, "height": 100,
        "layers": [first, { "id": B, "name": "B", "isVisible": true, "imageFile": format!("{B}.png"), "transform": transform }] });
    let mut images = vec![(format!("{A}.png"), claimed_png(a.0, a.1, 6)), (format!("{B}.png"), claimed_png(b.0, b.1, 6))];
    if let Some((w, h)) = mask { images.push((format!("{A}.mask.png"), claimed_png(w, h, 0))); }
    Package { manifest_json: manifest.to_string(), images }
}

#[test]
fn a_project_over_100_megapixels_is_refused_before_any_image_is_decoded() {
    // 60 MP + 60 MP. Decoding layer A first would fail with MissingImage (its data is empty).
    assert_eq!(open_package(&package((10_000, 6_000), (10_000, 6_000), None)).unwrap_err(), ProjectError::OverBudget);
}

#[test]
fn the_budget_is_exactly_100_megapixels() {
    // 1 + 100,000,000 pixels is over; 1 + 99,990,000 is not, so the reader moves on to decoding A
    // (a 1 x 1 claim, cheap to try) and fails there.
    assert_eq!(open_package(&package((1, 1), (10_000, 10_000), None)).unwrap_err(), ProjectError::OverBudget);
    assert_eq!(open_package(&package((1, 1), (10_000, 9_999), None)).unwrap_err(), ProjectError::MissingImage);
}

#[test]
fn masks_have_a_budget_of_their_own() {
    // 99.99 MP of layer images and 99.99 MP of masks: summed they would be over.
    assert_eq!(open_package(&package((1, 1), (10_000, 9_999), Some((10_000, 9_999)))).unwrap_err(), ProjectError::MissingImage);
}

#[test]
fn the_refusal_names_this_app_and_its_limit() {
    let text = ProjectError::OverBudget.to_string();
    assert!(text.contains("larger than Compositor for Windows supports") && text.contains("100 megapixels"), "{text}");
    assert!(text.is_ascii());
}

#[test]
fn a_text_box_up_to_200_million_square_pixels_is_valid_as_in_mac_1_2_10() {
    let png = encode_png(&Raster::from_premultiplied(2, 2, [9u8, 9, 9, 255].repeat(4)), 72.0).unwrap();
    let with_box = |w: u32, h: u32| {
        let manifest = json!({ "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
            "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-AAAAAAAAAAA0", "width": 100, "height": 100,
            "layers": [{ "id": A, "name": "Text", "isVisible": true, "imageFile": format!("{A}.png"),
                "transform": { "origin": [0, 0], "size": [2, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
                "text": { "alignment": "Left", "blue": 0, "boxSize": [w, h], "content": "Hi", "fontName": "Helvetica",
                    "fontSize": 72, "green": 0, "leading": 0, "red": 0, "tracking": 0 } }] });
        open_package(&Package { manifest_json: manifest.to_string(), images: vec![(format!("{A}.png"), png.clone())] })
    };
    assert!(with_box(20_000, 9_000).is_ok(), "180 M: valid in 1.2.10");
    assert!(matches!(with_box(20_000, 10_001), Err(ProjectError::Invalid)), "just over 200 M");
}
