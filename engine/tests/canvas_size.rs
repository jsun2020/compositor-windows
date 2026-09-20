mod fixtures;
use compositor_engine::*;
use compositor_engine::ops::canvas_size::*;
use compositor_engine::ops::flip::flip_canvas;
use fixtures::*;

fn imported() -> Document {
    let mut d = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    layer.transform.rotation = 37.0;
    layer.transform.flip_x = true;
    d.active_layer_id = Some(layer.id);
    d.layers.push(layer);
    d
}

#[test]
fn every_anchor_preserves_source_and_transform() {
    let source = imported();
    let transform = source.layers[0].transform;
    for delta in [5i32, -5] {
        for anchor in 0..9u8 {
            let out = canvas_size(&source, CanvasSizeOptions { width: (64 + delta) as u32, height: (32 + delta) as u32, anchor, fill: None, content_offset: None }).unwrap();
            let layer = &out.layers[0];
            let expected = [0, if delta == 5 { 2 } else { -3 }, delta];
            assert_eq!(layer.transform.origin.x, transform.origin.x + expected[(anchor % 3) as usize] as f64, "anchor {anchor} delta {delta}");
            assert_eq!(layer.transform.origin.y, transform.origin.y + expected[(anchor / 3) as usize] as f64);
            assert_eq!(layer.transform.size, transform.size);
            assert_eq!(layer.transform.rotation, 37.0);
            assert!(layer.transform.flip_x);
            assert_eq!(layer.id, source.layers[0].id);
            assert!(layer.pixels.as_ref().unwrap().same_pixels(source.layers[0].pixels.as_ref().unwrap()));
        }
    }
}

#[test]
fn colored_extension_keeps_old_area_transparent() {
    let mut d = Document::new(4, 4);
    d.layers.push(Layer::blank("Layer 1", d.size()));
    d.active_layer_id = Some(d.layers[0].id);
    let out = canvas_size(&d, CanvasSizeOptions { width: 8, height: 2, anchor: 4, fill: Some([1.0, 0.0, 0.0]), content_offset: None }).unwrap();
    assert_eq!(out.layers.len(), 2);
    assert_eq!(out.layers[0].name, "Canvas Extension");
    assert_eq!(out.active_layer_id, d.active_layer_id);
    let png = export_png(&out).unwrap();
    let img = decode_image(&png).unwrap().raster;
    assert_eq!(img.pixel(0, 0)[3], 255);
    assert!(img.pixel(0, 0)[0] > 252);
    assert_eq!(img.pixel(3, 0)[3], 0);
    assert_eq!(img.pixel(7, 1)[3], 255);
}

#[test]
fn transparent_resize_allocates_nothing_and_shrink_adds_no_fill() {
    let d = Document::new(4, 4);
    let large = canvas_size(&d, CanvasSizeOptions { width: 30_000, height: 30_000, anchor: 4, fill: None, content_offset: None }).unwrap();
    assert!(large.layers.is_empty());
    let small = canvas_size(&d, CanvasSizeOptions { width: 2, height: 2, anchor: 4, fill: Some([1.0; 3]), content_offset: None }).unwrap();
    assert!(small.layers.is_empty());
    let err = canvas_size(&d, CanvasSizeOptions { width: 30_000, height: 30_000, anchor: 4, fill: Some([1.0; 3]), content_offset: None }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
}

#[test]
fn crop_translates_without_resampling_and_undo_restores() {
    let mut e = Engine::new();
    let id = e.new_document(64, 32, false).unwrap();
    let raster = red_left_raster();
    let doc = e.document(id).unwrap().clone();
    // Seed a layer directly through the engine's import path substitute: use canvas_size on a hand-built doc.
    let mut seeded = doc.clone();
    seeded.layers.push(Layer::with_pixels("Red", raster.clone(), Point { x: 0.0, y: 0.0 }));
    let pkg = save_package(&seeded).unwrap();
    let id = e.open_package(&pkg, None).unwrap();
    e.execute(id, Command::Crop { x: 8.0, y: 4.0, width: 32.0, height: 16.0 }).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (32, 16));
    assert_eq!(s.layers[0].transform.origin, Point { x: -8.0, y: -4.0 });
    assert_eq!(s.layers[0].pixels_revision, 1, "pixels untouched");
    e.undo(id).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (64, 32));
    assert_eq!(s.layers[0].transform.origin, Point { x: 0.0, y: 0.0 });
}

#[test]
fn same_size_offset_crop_and_expansion_use_exact_bounds() {
    let mut e = Engine::new();
    let id = e.new_document(100, 50, true).unwrap();
    e.execute(id, Command::Crop { x: -20.0, y: 10.0, width: 100.0, height: 50.0 }).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].transform.origin, Point { x: 20.0, y: -10.0 });
    e.execute(id, Command::Crop { x: -10.0, y: -10.0, width: 140.0, height: 80.0 }).unwrap();
    let s = e.state(id).unwrap();
    assert_eq!((s.width, s.height), (140, 80));
    assert_eq!(s.layers[0].transform.origin, Point { x: 30.0, y: 0.0 });
    let img = decode_image(&e.export_png(id).unwrap()).unwrap().raster;
    assert_eq!((img.width, img.height), (140, 80));
    assert_eq!(img.pixel(0, 0)[3], 0);
}

#[test]
fn flip_canvas_mirrors_every_layer() {
    let mut d = imported();
    d.layers.push(Layer::blank("Layer 2", d.size()));
    let before = d.layers[0].transform;
    flip_canvas(&mut d, true);
    let l = &d.layers[0].transform;
    assert_eq!(l.flip_x, !before.flip_x);
    assert_eq!(l.rotation, -before.rotation);
    // Center x 32 stays at 32 on a 64-wide canvas; origin unchanged.
    assert_eq!(l.origin.x, before.origin.x);
    let blank = &d.layers[1].transform;
    assert!(blank.flip_x && blank.origin.x == 0.0);
    flip_canvas(&mut d, false);
    assert!(d.layers[0].transform.flip_y);
    assert_eq!(d.layers[0].transform.rotation, before.rotation);
}
