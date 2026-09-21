use compositor_engine::ops::distort::*;
use compositor_engine::*;

fn red(w: u32, h: u32) -> Raster { Raster::from_premultiplied(w, h, [255u8, 0, 0, 255].repeat((w * h) as usize)) }
fn t(x: f64, y: f64, w: f64, h: f64) -> LayerTransform { let mut t = LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h }); t.sampling = Sampling::Nearest; t }
const SHAPE: [Point; 4] = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];

#[test]
fn warp_places_pixels_over_the_shape_bounds() {
    let (out, placed) = warp(&red(20, 20), &t(10.0, 10.0, 20.0, 20.0), &SHAPE, true).unwrap();
    assert_eq!((placed.origin, placed.size), (Point { x: 10.0, y: 10.0 }, Size { width: 50.0, height: 20.0 }));
    assert_eq!(placed.rotation, 0.0);
    assert_eq!((out.width, out.height), (50, 20));
    assert_eq!(out.pixel(40, 2)[3], 255, "inside the stretched top-right");
    assert_eq!(out.pixel(5, 15)[3], 255);
    assert_eq!(out.pixel(40, 18)[3], 0, "outside the slanted right edge");
    assert!(warp(&red(2, 2), &t(0.0, 0.0, 2.0, 2.0), &[SHAPE[0], SHAPE[2], SHAPE[1], SHAPE[3]], true).is_err());
}

#[test]
fn warp_trimmed_hugs_visible_pixels_and_mask_keeps_background() {
    let mut data = vec![0u8; 40 * 20 * 4];
    for y in 5..15 { for x in 15..25 { let i = (y * 40 + x) * 4; data[i] = 255; data[i + 3] = 255; } }
    let raster = Raster::from_premultiplied(40, 20, data);
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 50.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let (out, placed, crop) = warp_trimmed(&raster, &t(10.0, 10.0, 40.0, 20.0), &shape, true).unwrap();
    assert!(placed.size.width < 20.0 && placed.size.height <= 12.0, "{placed:?}");
    assert!(placed.origin.x >= 20.0 && placed.origin.y >= 14.0);
    assert_eq!((out.width as f64, out.height as f64), (placed.size.width, placed.size.height));
    assert!(crop.2 > crop.0 && crop.3 > crop.1);
    let uniform = GrayRaster::from_bytes(1, 1, vec![255]);
    let (m, mp) = warp_mask(&uniform, &t(10.0, 10.0, 40.0, 20.0), &shape, 255).unwrap();
    assert_eq!((m.width, m.height), (1, 1));
    assert_eq!(mp.size, Size { width: 50.0, height: 20.0 });
    let black_edge = GrayRaster::from_bytes(4, 2, vec![0; 8]);
    let (m, _) = warp_mask(&black_edge, &t(10.0, 10.0, 40.0, 20.0), &shape, 255).unwrap();
    assert_eq!(m.bytes()[19 * 50 + 49], 255, "outside the shape shows the background");
}

#[test]
fn distort_layer_resamples_pixels_and_linked_mask_as_an_axis_aligned_layer() {
    let mut d = Document::new(100, 60);
    let mut l = Layer::with_pixels("Red", red(20, 20), Point { x: 10.0, y: 10.0 });
    l.transform.sampling = Sampling::Nearest;
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(20, 20, vec![255; 400]), enabled: true, placement: None, linked: None }));
    let id = l.id; d.layers.push(l);
    distort_layer(&mut d, id, &t(10.0, 10.0, 20.0, 20.0), &SHAPE).unwrap();
    let l = d.layer(id).unwrap();
    // Pixel-centre sampling: the slanted right edge sits at x 59.25 on row 0, so column 49 (sample 59.5) is empty and the trim drops it.
    assert_eq!((l.transform.origin, l.transform.size, l.transform.rotation), (Point { x: 10.0, y: 10.0 }, Size { width: 49.0, height: 20.0 }, 0.0));
    assert_eq!(l.pixels_revision, 2);
    let m = l.mask.as_ref().unwrap();
    assert_eq!((m.pixels.width, m.pixels.height), (49, 20), "the mask is warped and cropped with the pixels");
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 100.0, height: 60.0 }, 100, 60);
    assert_eq!(out.pixel(50, 12)[3], 255);
    assert_eq!(out.pixel(50, 28)[3], 0);
    // An unlinked mask stays where it was on the document.
    let mut d2 = Document::new(100, 60);
    let mut l2 = Layer::with_pixels("Red", red(20, 20), Point { x: 10.0, y: 10.0 });
    l2.transform.sampling = Sampling::Nearest;
    l2.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(20, 20, vec![255; 400]), enabled: true, placement: None, linked: Some(false) }));
    let id2 = l2.id; d2.layers.push(l2);
    distort_layer(&mut d2, id2, &t(10.0, 10.0, 20.0, 20.0), &SHAPE).unwrap();
    assert_eq!(d2.layer(id2).unwrap().mask.as_ref().unwrap().placement, Some(t(10.0, 10.0, 20.0, 20.0)));
}
