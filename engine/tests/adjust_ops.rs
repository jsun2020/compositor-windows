use compositor_engine::ops::{adjust, hierarchy};
use compositor_engine::*;

fn doc_with_layer() -> (Document, uuid::Uuid) {
    let mut d = Document::new(40, 40);
    let mut data = vec![0u8; 20 * 20 * 4];
    for y in 0..20 { for x in 0..10 { let i = ((y * 20 + x) * 4) as usize; data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]); } }
    let l = Layer::with_pixels("Half", Raster::from_premultiplied(20, 20, data), Point { x: 10.0, y: 10.0 });
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}

#[test]
fn an_adjustment_replaces_the_layers_pixels_in_place() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().pixels_revision;
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::apply_adjustment_to_layer(&mut d, id, &a).unwrap();
    let l = d.layer(id).unwrap();
    assert_eq!(l.pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255], "white becomes black");
    assert_eq!(l.transform.size, Size { width: 20.0, height: 20.0 }, "the layer keeps its place");
    assert!(l.pixels_revision > before);
    let folder = hierarchy::add_group(&mut d).unwrap();
    assert!(adjust::apply_adjustment_to_layer(&mut d, folder, &a).is_err(), "folders have no pixels to adjust");
}

#[test]
fn invert_works_on_pixels_and_on_a_mask() {
    let (mut d, id) = doc_with_layer();
    adjust::invert_layer(&mut d, id, false).unwrap();
    assert_eq!(d.layer(id).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255]);
    assert!(adjust::invert_layer(&mut d, id, true).is_err(), "no mask to invert");
    compositor_engine::ops::masks::add_mask(&mut d, id, true).unwrap();
    adjust::invert_layer(&mut d, id, true).unwrap();
    let mask = d.layer(id).unwrap().mask.as_ref().unwrap();
    assert_eq!(mask.pixels.bytes()[0], 0, "a revealing mask inverts to hiding");
}

#[test]
fn a_blur_grows_the_layer_spreads_past_its_edge_and_trims_back() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().transform;
    adjust::apply_filter(&mut d, id, &FilterParams::GaussianBlur { radius: 3.0 }).unwrap();
    let l = d.layer(id).unwrap();
    assert!(l.transform.origin.x < before.origin.x, "the blur spreads past the old left edge: {:?}", l.transform);
    assert!(l.transform.size.width > before.size.width && l.transform.size.height > before.size.height);
    let raster = l.pixels.as_ref().unwrap();
    assert!(raster.pixel(0, raster.height / 2)[3] > 0, "trimmed to where the blur actually reaches");
    let middle = raster.pixel(raster.width / 2, raster.height / 2)[3];
    assert!(middle > 20 && middle < 235, "the hard edge is soft");
    // The content spans the layer's full height flush to both edges, so the blur grows the top
    // and bottom by the same amount. It cannot do that horizontally: the opaque block is flush
    // on the left but leaves ten transparent columns before the right edge, so the blur clears
    // the left edge while the trim cuts the right one back to where the alpha actually reaches.
    let grew_top = before.origin.y - l.transform.origin.y;
    let grew_bottom = (l.transform.origin.y + l.transform.size.height) - (before.origin.y + before.size.height);
    assert!((grew_top - grew_bottom).abs() < 1.5, "top {grew_top} vs bottom {grew_bottom}");
    assert!(grew_top > 1.0, "the blur spread past the top edge: {grew_top}");
    let grew_left = before.origin.x - l.transform.origin.x;
    assert!(grew_left > 1.0, "and past the left edge, where the content is flush: {grew_left}");
    let grew_right = (l.transform.origin.x + l.transform.size.width) - (before.origin.x + before.size.width);
    assert!(grew_right < 0.0, "nothing reached the right edge, so the trim pulled it in: {grew_right}");
}

#[test]
fn a_blur_carries_a_covering_mask_onto_the_new_grid() {
    let (mut d, id) = doc_with_layer();
    d.layer_mut(id).unwrap().set_mask(Some(Mask {
        pixels: GrayRaster::from_bytes(20, 20, (0..400).map(|i| if (i % 20) < 10 { 255 } else { 0 }).collect()),
        enabled: true, placement: None, linked: None }));
    adjust::apply_filter(&mut d, id, &FilterParams::GaussianBlur { radius: 2.0 }).unwrap();
    let l = d.layer(id).unwrap();
    let mask = l.mask.as_ref().unwrap();
    assert!(mask.placement.is_none(), "still a covering mask");
    // The mask's white half still covers the same document area: sample its middle-left and middle-right.
    let to_doc = l.transform.pixel_to_document(mask.pixels.width, mask.pixels.height);
    let mut left_white = 0; let mut right_white = 0;
    for x in 0..mask.pixels.width {
        let p = to_doc.apply(Point { x: x as f64 + 0.5, y: mask.pixels.height as f64 / 2.0 });
        let value = mask.pixels.bytes()[(mask.pixels.height / 2 * mask.pixels.width + x) as usize];
        if p.x < 20.0 && value > 200 { left_white += 1; }
        if p.x > 20.0 && value > 200 { right_white += 1; }
    }
    assert!(left_white > 5 && right_white == 0, "left {left_white}, right {right_white}");
}

#[test]
fn a_filter_that_does_not_spread_keeps_the_layers_grid() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().transform;
    adjust::apply_filter(&mut d, id, &FilterParams::AddNoise { amount: 20.0, gaussian: false, monochromatic: false, seed: 3 }).unwrap();
    let l = d.layer(id).unwrap();
    assert_eq!(l.transform, before);
    assert_eq!(l.pixels.as_ref().unwrap().width, 20);
}

#[test]
fn adjustment_layers_are_created_above_the_active_layer_and_edited_in_place() {
    let (mut d, id) = doc_with_layer();
    let new = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Curves, 0, None).unwrap();
    assert_eq!(d.active_layer_id, Some(new));
    assert_eq!(d.index_of(new).unwrap(), d.index_of(id).unwrap() + 1);
    let layer = d.layer(new).unwrap();
    assert!(layer.is_adjustment() && layer.pixels.is_none() && layer.name == "Curves");
    assert_eq!(layer.transform.size, d.size());
    let mut edited = layer.extra.adjustment.clone().unwrap();
    edited.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    adjust::set_adjustment(&mut d, new, &edited).unwrap();
    assert_eq!(d.layer(new).unwrap().extra.adjustment.as_ref().unwrap().curves, edited.curves);
    let mut broken = edited.clone();
    broken.curves.channels[0] = vec![CurvePoint { x: 5.0, y: 0.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(adjust::set_adjustment(&mut d, new, &broken).is_err(), "invalid settings are refused");
    assert!(adjust::set_adjustment(&mut d, id, &edited).is_err(), "a pixel layer is not an adjustment layer");
    // A Gradient Map takes the palette's colours; each Grain layer gets its own pattern.
    let gradient = adjust::add_adjustment_layer(&mut d, AdjustmentKind::GradientMap, 0, Some(([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]))).unwrap();
    let g = d.layer(gradient).unwrap().extra.adjustment.as_ref().unwrap().gradient_map();
    assert_eq!(g.shadows, AdjustmentColor { red: 1.0, green: 0.0, blue: 0.0 });
    let grain = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Grain, 4242, None).unwrap();
    assert_eq!(d.layer(grain).unwrap().extra.adjustment.as_ref().unwrap().grain().seed, 4242);
}

#[test]
fn an_adjustment_layer_is_never_a_clipping_source() {
    let (mut d, id) = doc_with_layer();
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    // The adjustment can clip to the pixel layer below it.
    assert!(hierarchy::can_toggle_clipping(&d, a));
    hierarchy::toggle_clipping(&mut d, a).unwrap();
    assert_eq!(d.layer(a).unwrap().mask_source_id, Some(id));
    // A layer above the adjustment cannot clip to the adjustment.
    let mut top = Layer::with_pixels("Top", Raster::new_transparent(4, 4), Point { x: 0.0, y: 0.0 });
    let top_id = top.id; top.parent_id = None; d.layers.push(top);
    assert!(!hierarchy::can_toggle_clipping(&d, top_id), "an adjustment layer supplies no coverage");
}
