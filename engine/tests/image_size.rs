mod fixtures;
use compositor_engine::*;
use compositor_engine::ops::image_size::*;
use fixtures::*;

#[test]
fn resize_resamples_pixels_and_keeps_identity() {
    let mut d = Document::new(64, 32);
    let layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    let id = layer.id;
    d.active_layer_id = Some(id);
    d.layers.push(layer);
    let out = image_size(&d, ImageSizeOptions { width: 128, height: 96, resolution: 300.0, sampling: Sampling::Nearest }).unwrap();
    assert_eq!((out.width, out.height, out.resolution), (128, 96, 300.0));
    assert_eq!(out.active_layer_id, Some(id));
    let px = out.layers[0].pixels.as_ref().unwrap();
    assert_eq!((px.width, px.height), (128, 96));
    assert!(px.pixel(0, 0)[0] > 242);
    assert_eq!(px.pixel(127, 0)[3], 0);
    assert_eq!(out.layers[0].pixels_revision, 2);
}

#[test]
fn resolution_only_keeps_pixels_and_transforms() {
    let mut d = Document::new(32, 16);
    d.layers.push(Layer::blank("Layer 1", d.size()));
    d.layers.push(Layer::with_pixels("Red", red_left_raster(), Point { x: -10.0, y: 3.0 }));
    let out = image_size(&d, ImageSizeOptions { width: 32, height: 16, resolution: 300.0, sampling: Sampling::High }).unwrap();
    assert_eq!(out.layers[0].transform, d.layers[0].transform);
    assert!(out.layers[1].pixels.as_ref().unwrap().same_pixels(d.layers[1].pixels.as_ref().unwrap()));
    assert_eq!(out.resolution, 300.0);
    let pkg = save_package(&out).unwrap();
    assert!(pkg.manifest_json.contains("\"resolution\": 300"));
    assert!((png_dpi(&export_png(&out).unwrap()).unwrap() - 300.0).abs() < 1.0);
}

#[test]
fn rotated_hidden_layer_scales_in_document_axes_and_invalid_size_is_rejected() {
    let mut d = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: -16.0, y: 4.0 });
    layer.transform.rotation = 90.0;
    layer.visible = false;
    d.layers.push(layer);
    let out = image_size(&d, ImageSizeOptions { width: 128, height: 96, resolution: 72.0, sampling: Sampling::Nearest }).unwrap();
    let t = out.layers[0].transform;
    assert!(!out.layers[0].visible);
    assert_eq!(t.rotation, 0.0);
    // A 90-degree 64x32 layer becomes 32x64, then scales 2x horizontally and 3x vertically.
    assert!((t.size.width - 64.0).abs() <= 1.0);
    assert!((t.size.height - 192.0).abs() <= 1.0);
    assert!(t.origin.y < 0.0);
    let err = image_size(&d, ImageSizeOptions { width: 30_000, height: 30_000, resolution: 72.0, sampling: Sampling::High }).unwrap_err();
    assert_eq!(err, ProjectError::TooLarge);
}

/// Final review I2: the covering mask of a turned or flipped layer is redrawn upright on the new
/// grid, so its revision must move on, or a renderer keyed by it keeps the old, flipped mask.
#[test]
fn resize_redraws_a_flipped_layers_covering_mask_under_a_new_revision() {
    let mut d = Document::new(64, 32);
    let mut layer = Layer::with_pixels("Red", red_left_raster(), Point { x: 0.0, y: 0.0 });
    layer.transform.flip_x = true;
    layer.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(4, 2, vec![0, 80, 160, 255, 0, 80, 160, 255]), enabled: true, placement: None, linked: None }));
    let before = layer.mask_revision;
    d.layers.push(layer);
    let out = image_size(&d, ImageSizeOptions { width: 128, height: 64, resolution: 72.0, sampling: Sampling::Nearest }).unwrap();
    let m = &out.layers[0].mask.as_ref().unwrap().pixels;
    assert_eq!((m.width, m.height), (128, 64), "redrawn on the new grid");
    assert_eq!((m.bytes()[0], m.bytes()[127]), (255, 0), "upright: the flip is drawn into it");
    assert!(out.layers[0].mask_revision > before, "a new revision for the new mask");
}

#[test]
fn image_size_command_is_undoable() {
    let mut e = Engine::new();
    let id = e.new_document(40, 20, true).unwrap();
    e.execute(id, Command::ImageSize { width: 80, height: 40, resolution: 72.0, sampling: Sampling::High }).unwrap();
    assert_eq!(e.state(id).unwrap().width, 80);
    assert_eq!(e.state(id).unwrap().layers[0].transform.size, Size { width: 80.0, height: 40.0 });
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().width, 40);
}
