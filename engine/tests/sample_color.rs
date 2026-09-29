//! Sampling the canvas for the palette (Phase 4b-1): the Eyedropper and the colour picker read the
//! visible composite as the Mac's `sampleCompositeColor` does.
use compositor_engine::*;

fn p(x: f64, y: f64) -> Point { Point { x, y } }

#[test]
fn the_composite_is_read_at_the_pixel_under_the_point_and_nothing_off_the_canvas() {
    // ColorPickerTests.canvasSamplingReadsCompositeAndCommitsOnlyOnOK: a 4 x 4 image, red on its
    // top two rows and blue on the bottom two.
    let mut data = Vec::new();
    for y in 0..4 { for _ in 0..4 { data.extend_from_slice(if y < 2 { &[255, 0, 0, 255] } else { &[0, 0, 255, 255] }); } }
    let mut e = Engine::new();
    let id = e.new_document(4, 4, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers = vec![Layer::with_pixels("Split", Raster::from_premultiplied(4, 4, data), p(0.0, 0.0))];
    let id = e.insert_document(doc);
    assert_eq!(e.sample_color(id, p(1.5, 0.5)).unwrap(), Some([1.0, 0.0, 0.0]));
    assert_eq!(e.sample_color(id, p(1.5, 3.5)).unwrap(), Some([0.0, 0.0, 1.0]));
    assert_eq!(e.sample_color(id, p(2.0, 3.0)).unwrap(), Some([0.0, 0.0, 1.0]), "a whole point is the pixel it starts");
    for off in [p(-1.0, 1.0), p(4.0, 1.0), p(1.0, -0.01), p(1.0, 4.0)] { assert_eq!(e.sample_color(id, off).unwrap(), None, "{off:?}"); }
}

#[test]
fn a_translucent_pixel_is_unpremultiplied_and_snapped_to_eight_bits() {
    // Premultiplied (100, 37, 250 > alpha, 180): each channel min(a, v) / a * 255, rounded, / 255 --
    // 100 / 180 * 255 = 141.67 -> 142; 37 / 180 * 255 = 52.42 -> 52; 250 is clamped to 180 -> 255.
    let mut e = Engine::new();
    let id = e.new_document(2, 1, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers = vec![Layer::with_pixels("A", Raster::from_premultiplied(2, 1, vec![100, 37, 250, 180, 0, 0, 0, 0]), p(0.0, 0.0))];
    let id = e.insert_document(doc);
    assert_eq!(e.sample_color(id, p(0.5, 0.5)).unwrap(), Some([142.0 / 255.0, 52.0 / 255.0, 1.0]));
    assert_eq!(e.sample_color(id, p(1.5, 0.5)).unwrap(), None, "a transparent pixel has no colour");
}
