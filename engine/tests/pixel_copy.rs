use compositor_engine::*;

fn pattern() -> Raster {
    let color = [
        255, 0, 0, 255, 0, 128, 0, 128, 0, 0, 64, 64,
        30, 20, 10, 255, 40, 50, 60, 255, 70, 80, 90, 255,
    ];
    let mut pixels = vec![0; 5 * 4 * 4];
    for y in 0..2 { pixels[((y + 1) * 5 + 1) * 4..((y + 1) * 5 + 4) * 4].copy_from_slice(&color[y * 12..(y + 1) * 12]); }
    Raster::from_premultiplied(5, 4, pixels)
}

#[test]
fn upright_fractional_layer_copies_pixels_without_changing_its_record() {
    for sampling in [Sampling::Smooth, Sampling::High] {
        let mut doc = Document::new(20, 30);
        let mut layer = Layer::with_pixels("Pattern", pattern(), Point { x: 10.5, y: 20.25 });
        layer.transform.sampling = sampling;
        let transform = layer.transform;
        doc.layers.push(layer);
        let image = render_full(&doc).unwrap();
        assert_eq!(image.cropped(10, 20, 5, 4).bytes(), pattern().bytes());
        assert_eq!(image.pixel(9, 20), [0; 4]);
        assert_eq!(image.pixel(15, 20), [0; 4]);
        assert_eq!(doc.layers[0].transform, transform);
    }
}

#[test]
fn clipping_source_uses_the_same_fractional_pixel_copy_as_visible_layers() {
    let mut doc = Document::new(20, 30);
    let mut source = Layer::with_pixels("Source", pattern(), Point { x: 10.5, y: 20.25 });
    source.visible = false;
    let mut target = Layer::with_pixels("Target", Raster::from_premultiplied(3, 2, vec![255; 24]), Point { x: 11.0, y: 21.0 });
    target.mask_source_id = Some(source.id);
    doc.layers = vec![target, source];
    let image = render_full(&doc).unwrap();
    assert_eq!((0..3).map(|x| image.pixel(x + 11, 21)[3]).collect::<Vec<_>>(), [255, 128, 64]);
}

#[test]
fn rotation_and_enlargement_keep_resampling_instead_of_using_the_copy_path() {
    for (rotation, width) in [(1.0, 5.0), (0.0, 6.0)] {
        let mut doc = Document::new(20, 30);
        let mut layer = Layer::with_pixels("Pattern", pattern(), Point { x: 10.5, y: 20.25 });
        layer.transform.rotation = rotation;
        layer.transform.size.width = width;
        doc.layers.push(layer);
        let smooth = render_full(&doc).unwrap();
        doc.layers[0].transform.sampling = Sampling::Nearest;
        assert_ne!(smooth.bytes(), render_full(&doc).unwrap().bytes());
    }
}

#[test]
fn automatic_copies_antialias_the_rectangle_edge_but_explicit_nearest_does_not() {
    let mut doc = Document::new(5, 4);
    doc.layers.push(Layer::with_pixels("Edge", Raster::from_premultiplied(2, 2, vec![255; 16]), Point { x: 1.5, y: 1.0 }));
    let image = render_full(&doc).unwrap();
    // Mac 1.4.5's returned upright probe establishes 127/255 at half coverage.
    assert_eq!((0..5).map(|x| image.pixel(x, 1)[3]).collect::<Vec<_>>(), [0, 127, 255, 127, 0]);
    doc.layers[0].transform.sampling = Sampling::Nearest;
    let nearest = render_full(&doc).unwrap();
    assert_eq!((0..5).map(|x| nearest.pixel(x, 1)[3]).collect::<Vec<_>>(), [0, 255, 255, 0, 0]);
}
