mod fixtures;
use compositor_engine::*;
use fixtures::*;

#[test]
fn png_round_trip_keeps_pixels_and_transparency() {
    let bytes = red_left_png();
    let decoded = decode_image(&bytes).unwrap().raster;
    assert_eq!((decoded.width, decoded.height), (64, 32));
    assert_eq!(decoded.pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(decoded.pixel(63, 0), [0, 0, 0, 0]);
}

#[test]
fn jpeg_tiff_webp_bmp_decode() {
    let raster = red_left_raster();
    for format in [image::ImageFormat::Jpeg, image::ImageFormat::Tiff, image::ImageFormat::WebP, image::ImageFormat::Bmp] {
        let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut out, format).unwrap();
        let decoded = decode_image(&out.into_inner()).unwrap().raster;
        assert_eq!((decoded.width, decoded.height), (64, 32), "{format:?}");
        assert!(decoded.pixel(0, 0)[0] > 240, "{format:?}");
    }
}

#[test]
fn exif_orientation_is_applied() {
    // Build a JPEG with orientation 6 (rotate 90 CW) by writing an APP1 EXIF segment.
    let raster = red_left_raster();
    let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    let with_exif = insert_exif_orientation(jpeg.into_inner(), 6);
    let decoded = decode_image(&with_exif).unwrap().raster;
    assert_eq!((decoded.width, decoded.height), (32, 64));
}

/// Minimal EXIF APP1 with a single Orientation tag, inserted after SOI.
fn insert_exif_orientation(jpeg: Vec<u8>, orientation: u16) -> Vec<u8> {
    let mut tiff = vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08]; // big endian, IFD0 at 8
    tiff.extend_from_slice(&[0x00, 0x01]); // one entry
    tiff.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]); // tag 0x112 SHORT count 1
    tiff.extend_from_slice(&orientation.to_be_bytes());
    tiff.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // pad + next IFD 0
    let mut app1 = b"Exif\0\0".to_vec();
    app1.extend_from_slice(&tiff);
    let len = (app1.len() + 2) as u16;
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&app1);
    out.extend_from_slice(&jpeg[2..]);
    out
}

#[test]
fn unsupported_and_unreadable() {
    let gif = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\x00\x00\x00\xff\xff\xff\x21\xf9\x04\x01\x00\x00\x00\x00\x2c\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02\x44\x01\x00\x3b";
    assert_eq!(decode_image(gif).unwrap_err(), ImportError::Unsupported);
    assert_eq!(decode_image(b"not an image").unwrap_err(), ImportError::Unreadable);
    let mut truncated = red_left_png();
    truncated.truncate(40);
    assert_eq!(decode_image(&truncated).unwrap_err(), ImportError::Unreadable);
}

#[test]
fn package_png_rejects_other_formats_and_deep_bit_depths() {
    let raster = red_left_raster();
    let img = image::RgbaImage::from_raw(64, 32, raster.to_straight()).unwrap();
    let mut jpeg = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img.clone()).to_rgb8().write_to(&mut jpeg, image::ImageFormat::Jpeg).unwrap();
    assert_eq!(decode_package_png(&jpeg.into_inner()).unwrap_err(), ProjectError::MissingImage);
    let sixteen = image::DynamicImage::ImageRgba8(img).to_rgba16();
    let mut png16 = std::io::Cursor::new(Vec::new());
    sixteen.write_to(&mut png16, image::ImageFormat::Png).unwrap();
    assert_eq!(decode_package_png(&png16.into_inner()).unwrap_err(), ProjectError::MissingImage);
    assert!(decode_package_png(&red_left_png()).is_ok());
}

#[test]
fn png_carries_resolution() {
    let bytes = encode_png(&red_left_raster(), 300.0).unwrap();
    let dpi = png_dpi(&bytes).unwrap();
    assert!((dpi - 300.0).abs() < 1.0);
}

#[test]
fn jpeg_uses_matte_quality_and_is_opaque() {
    let transparent = Raster::new_transparent(20, 12);
    let white = encode_jpeg(&transparent, 0.85, [1.0, 1.0, 1.0], 72.0).unwrap();
    let blue = encode_jpeg(&transparent, 1.0, [0.0, 0.0, 1.0], 72.0).unwrap();
    let w = decode_image(&white).unwrap().raster;
    let b = decode_image(&blue).unwrap().raster;
    assert_eq!((w.width, w.height), (20, 12));
    assert!(w.pixel(0, 0)[0] > 247 && w.pixel(0, 0)[3] == 255);
    assert!(b.pixel(0, 0)[2] > 247 && b.pixel(0, 0)[0] < 8);
    let pattern = pattern_raster(128, 128);
    let low = encode_jpeg(&pattern, 0.1, [1.0; 3], 72.0).unwrap();
    let high = encode_jpeg(&pattern, 1.0, [1.0; 3], 72.0).unwrap();
    assert!(low.len() < high.len());
}

#[test]
fn tiles_and_halving() {
    let r = pattern_raster(300, 270);
    let mut tile = vec![0u8; (TILE * TILE * 4) as usize];
    r.tile_rgba(1, 1, &mut tile);
    // Tile (1,1) covers x 256..300, y 256..270: pixel (0,0) of the tile is raster (256,256).
    assert_eq!(&tile[0..4], &r.pixel(256, 256));
    // Outside the raster is zero.
    let outside = ((5 * TILE + 100) * 4) as usize;
    assert_eq!(&tile[outside..outside + 4], &[0, 0, 0, 0]);
    let half = r.halved();
    assert_eq!((half.width, half.height), (150, 135));
    let expected = {
        let a = r.pixel(0, 0); let b = r.pixel(1, 0); let c = r.pixel(0, 1); let d = r.pixel(1, 1);
        ((a[0] as u32 + b[0] as u32 + c[0] as u32 + d[0] as u32 + 2) / 4) as u8
    };
    assert_eq!(half.pixel(0, 0)[0], expected);
}

#[test]
fn halving_a_degenerate_raster_does_not_panic() {
    let zero_width = Raster::new_transparent(0, 5);
    let halved_zw = zero_width.halved();
    assert_eq!((halved_zw.width, halved_zw.height), (0, 5));
    assert!(halved_zw.same_pixels(&zero_width));
    let one_by_one = Raster::new_transparent(1, 1);
    let halved_1x1 = one_by_one.halved();
    assert_eq!((halved_1x1.width, halved_1x1.height), (1, 1));
}

#[test]
fn mask_round_trip_and_non_grayscale_is_invalid() {
    let mask_bytes = vec![0, 128, 255, 255, 128, 0];
    let mask = GrayRaster::from_bytes(3, 2, mask_bytes.clone());
    let encoded = encode_gray_png(&mask).unwrap();
    let decoded = decode_package_mask(&encoded).unwrap();
    assert_eq!((decoded.width, decoded.height), (3, 2));
    assert_eq!(decoded.bytes(), &mask_bytes);
    assert_eq!(mask.is_uniform(), None);
    let uniform_mask = GrayRaster::from_bytes(1, 1, vec![255]);
    let encoded_uniform = encode_gray_png(&uniform_mask).unwrap();
    let decoded_uniform = decode_package_mask(&encoded_uniform).unwrap();
    assert_eq!(decoded_uniform.is_uniform(), Some(255));
    assert_eq!(decode_package_mask(&red_left_png()).unwrap_err(), ProjectError::Invalid);
}

#[test]
fn same_pixels_is_pointer_equality() {
    let r1 = pattern_raster(10, 10);
    let r2 = r1.clone();
    assert!(r1.same_pixels(&r2));
    let bytes = r1.bytes().to_vec();
    let r3 = Raster::from_premultiplied(10, 10, bytes);
    assert!(!r1.same_pixels(&r3));
}
