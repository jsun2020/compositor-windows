mod fixtures;
use compositor_engine::*;

/// 2x2 image: left column red, right column transparent, placed at (1,1) as 4x4 on a 6x6 canvas.
fn doc(rotation: f64, flip: bool) -> Document {
    let mut d = Document::new(6, 6);
    let raster = Raster::from_premultiplied(2, 2, vec![255,0,0,255, 0,0,0,0, 255,0,0,255, 0,0,0,0]);
    let mut layer = Layer::with_pixels("Red", raster, Point { x: 1.0, y: 1.0 });
    layer.transform.size = Size { width: 4.0, height: 4.0 };
    layer.transform.rotation = rotation;
    layer.transform.flip_x = flip;
    layer.transform.sampling = Sampling::Nearest;
    d.active_layer_id = Some(layer.id);
    d.layers.push(layer);
    d
}

#[test]
fn png_preserves_dimensions_alpha_orientation_and_transforms() {
    for (rotation, flip, red, clear) in [(0.0, false, (1, 1), (4, 1)), (0.0, true, (4, 1), (1, 1)), (90.0, false, (1, 1), (1, 4))] {
        let bytes = export_png(&doc(rotation, flip)).unwrap();
        let img = decode_image(&bytes).unwrap().raster;
        assert_eq!((img.width, img.height), (6, 6));
        let r = img.pixel(red.0, red.1);
        assert!(r[0] > 252 && r[3] == 255, "rot {rotation} flip {flip}: {r:?}");
        assert_eq!(img.pixel(clear.0, clear.1)[3], 0, "rot {rotation} flip {flip}");
        assert_eq!(img.pixel(0, 0)[3], 0);
    }
}

#[test]
fn order_and_visibility() {
    let mut d = doc(0.0, false);
    let blue = Raster::from_premultiplied(1, 1, vec![0, 0, 255, 255]);
    let mut top = Layer::with_pixels("Blue", blue, Point { x: -2.0, y: -2.0 });
    top.transform.size = Size { width: 10.0, height: 10.0 };
    for visible in [true, false] {
        top.visible = visible;
        d.layers.truncate(1);
        d.layers.push(top.clone());
        let img = decode_image(&export_png(&d).unwrap()).unwrap().raster;
        let px = img.pixel(1, 1);
        if visible { assert!(px[2] > 252); } else { assert!(px[0] > 252); }
    }
}

#[test]
fn hidden_group_hides_children() {
    let mut d = doc(0.0, false);
    let mut group = Layer::blank("Folder", d.size());
    group.is_group = true;
    group.visible = false;
    d.layers[0].parent_id = Some(group.id);
    d.layers.insert(0, group);
    let img = decode_image(&export_png(&d).unwrap()).unwrap().raster;
    assert_eq!(img.pixel(1, 1)[3], 0);
}

#[test]
fn opacity_scales_alpha() {
    let mut d = doc(0.0, false);
    d.layers[0].opacity = 0.5;
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 6.0, height: 6.0 }, 6, 6);
    let px = out.pixel(1, 1);
    assert!((px[3] as i32 - 128).abs() <= 1 && (px[0] as i32 - 128).abs() <= 1);
}

#[test]
fn blank_and_oversized_canvas() {
    let blank = Document::new(2, 2);
    let img = decode_image(&export_png(&blank).unwrap()).unwrap().raster;
    assert_eq!(img.pixel(1, 1)[3], 0);
    let huge = Document::new(30_000, 30_000);
    assert_eq!(export_png(&huge).unwrap_err(), ExportError::TooLarge);
}

#[test]
fn smooth_sampling_blends_at_edges_and_high_prefilters_when_shrinking() {
    let mut d = Document::new(4, 4);
    let checker = fixtures::pattern_raster(64, 64);
    let mut layer = Layer::with_pixels("P", checker, Point { x: 0.0, y: 0.0 });
    layer.transform.size = Size { width: 4.0, height: 4.0 };
    layer.transform.sampling = Sampling::High;
    d.layers.push(layer);
    let high = composite(&d, Rect { x: 0.0, y: 0.0, width: 4.0, height: 4.0 }, 4, 4);
    d.layers[0].transform.sampling = Sampling::Nearest;
    let nearest = composite(&d, Rect { x: 0.0, y: 0.0, width: 4.0, height: 4.0 }, 4, 4);
    // Prefiltered output averages 16x16 source blocks; nearest picks one texel. They differ.
    assert_ne!(high.bytes(), nearest.bytes());
    assert_eq!(high.pixel(0, 0)[3], 255);
}

#[test]
fn jpeg_export_flattens_on_matte() {
    let d = Document::new(20, 12);
    let bytes = export_jpeg(&d, 0.85, [0.0, 0.0, 1.0]).unwrap();
    let img = decode_image(&bytes).unwrap().raster;
    assert_eq!((img.width, img.height), (20, 12));
    assert!(img.pixel(0, 0)[2] > 247 && img.pixel(0, 0)[3] == 255);
}

#[test]
fn magnified_layer_is_solid_to_its_edge() {
    let mut d = Document::new(8, 8);
    let blue = Raster::from_premultiplied(1, 1, vec![0, 0, 255, 255]);
    let mut layer = Layer::with_pixels("Blue", blue, Point { x: 0.0, y: 0.0 });
    layer.transform.size = Size { width: 8.0, height: 8.0 };
    layer.transform.sampling = Sampling::High;
    d.layers.push(layer);
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 8.0, height: 8.0 }, 8, 8);
    assert!(out.pixel(0, 0)[2] > 252 && out.pixel(0, 0)[3] == 255, "corner (0,0)");
    assert!(out.pixel(7, 7)[2] > 252 && out.pixel(7, 7)[3] == 255, "corner (7,7)");

    let mut d = Document::new(10, 10);
    let blue2 = Raster::from_premultiplied(1, 1, vec![0, 0, 255, 255]);
    let mut layer = Layer::with_pixels("Blue", blue2, Point { x: 0.0, y: 0.0 });
    layer.transform.size = Size { width: 8.0, height: 8.0 };
    d.layers.push(layer);
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 10.0, height: 10.0 }, 10, 10);
    assert_eq!(out.pixel(9, 9)[3], 0, "outside layer");

    // Test 2x2 raster with left column red, right column blue (exercises two-index bilinear clamp)
    let mut d = Document::new(16, 16);
    let raster2x2 = Raster::from_premultiplied(2, 2, vec![
        255, 0, 0, 255,    0, 0, 255, 255,     // row 0: red, blue
        255, 0, 0, 255,    0, 0, 255, 255,     // row 1: red, blue
    ]);
    let mut layer = Layer::with_pixels("RedBlue", raster2x2, Point { x: 0.0, y: 0.0 });
    layer.transform.size = Size { width: 16.0, height: 16.0 };
    layer.transform.sampling = Sampling::Smooth;
    d.layers.push(layer);
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 16.0, height: 16.0 }, 16, 16);
    assert!(out.pixel(0, 0)[0] > 252 && out.pixel(0, 0)[2] < 3, "corner (0,0) is red");
    assert!(out.pixel(15, 15)[2] > 252 && out.pixel(15, 15)[0] < 3, "corner (15,15) is blue");
    assert!(out.pixel(0, 15)[0] > 252 && out.pixel(0, 15)[2] < 3, "corner (0,15) is red");
}
