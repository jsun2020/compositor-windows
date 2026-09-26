//! Layer effects through Image Size, Canvas Size, Crop and reduced previews (Phase 3.5c rulings).
//! The Mac drops effects on Image Size, Canvas Size, Crop and Trim (R 5, 1.2.10 delta 3); this port
//! keeps them looking as they did.
use compositor_engine::*;
use serde_json::json;

fn effects(value: serde_json::Value) -> LayerEffects { serde_json::from_value(value).unwrap() }
fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Raster { Raster::from_premultiplied(w, h, rgba.repeat((w * h) as usize)) }
fn full(doc: &Document) -> Raster { composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height) }
fn resize(doc: &Document, width: u32, height: u32) -> Document {
    ops::image_size::image_size(doc, ops::image_size::ImageSizeOptions { width, height, resolution: 72.0, sampling: Sampling::High }).unwrap()
}

/// Every effect, lengths not at their defaults.
fn all_six() -> LayerEffects {
    effects(json!({
        "stroke": { "blue": 0, "green": 1, "inside": false, "opacity": 1, "red": 0, "size": 6 },
        "shadow": { "angle": 30, "blue": 0, "blur": 8, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 },
        "colorOverlay": { "blue": 0.4, "green": 0.1, "opacity": 0.3, "red": 0.9 },
        "innerShadow": { "angle": -35, "blue": 0, "blur": 2, "distance": 4, "green": 0, "opacity": 0.6, "red": 0 },
        "outerGlow": { "blue": 1, "green": 1, "opacity": 0.75, "red": 1, "size": 10 },
        "innerGlow": { "blue": 1, "green": 1, "opacity": 0.75, "red": 1, "size": 6 } }))
}

#[test]
fn image_size_scales_every_effect_with_the_layer() {
    let mut doc = Document::new(200, 100);
    let mut layer = Layer::with_pixels("L", solid(80, 40, [200, 90, 30, 255]), Point { x: 10.0, y: 20.0 });
    layer.extra.effects = Some(all_six());
    doc.layers = vec![layer];
    let half = resize(&doc, 100, 50);
    let (was, now) = (all_six(), half.layers[0].extra.effects.clone().unwrap());
    let k = 100.0 / 200.0;
    assert_eq!(now.stroke.as_ref().unwrap().size, was.stroke.as_ref().unwrap().size * k);
    let (s, t) = (now.shadow.as_ref().unwrap(), was.shadow.as_ref().unwrap());
    assert_eq!((s.angle, s.distance, s.blur), (t.angle, t.distance * k, t.blur * k), "the direction is kept");
    let (s, t) = (now.inner_shadow.as_ref().unwrap(), was.inner_shadow.as_ref().unwrap());
    assert_eq!((s.angle, s.distance, s.blur), (t.angle, t.distance * k, t.blur * k));
    assert_eq!(now.outer_glow.as_ref().unwrap().size, was.outer_glow.as_ref().unwrap().size * k);
    assert_eq!(now.inner_glow.as_ref().unwrap().size, was.inner_glow.as_ref().unwrap().size * k);
    assert_eq!(now.color_overlay, was.color_overlay, "no length, no change");
}

/// Fix round 1 (task-6-review.md): every fixture above resizes by the same factor on both axes, so
/// a `resampled` that swapped `sx` and `sy`, or scaled both axes by one of them, would pass
/// unnoticed. `sx` and `sy` are made clearly different (2 and 1.5) below.
#[test]
fn image_size_with_a_non_uniform_scale_resamples_each_axis_by_its_own_factor() {
    let (doc_w, doc_h) = (300u32, 100u32);
    let (new_w, new_h) = (600u32, 150u32);
    let (sx, sy) = (new_w as f64 / doc_w as f64, new_h as f64 / doc_h as f64);
    assert_ne!(sx, sy, "the point of this test is a resize that is not uniform");

    // A square, unrotated, unflipped layer: `Layer::with_pixels` gives it a transform whose size
    // equals its raster (LayerTransform::axis_aligned), so its own pixel_to_document is the
    // identity map (a = d = 1, b = c = 0, no translation in the linear part). What resamples the
    // effects is then sx and sy alone.
    let mut doc = Document::new(doc_w, doc_h);
    let mut layer = Layer::with_pixels("L", solid(100, 100, [10, 20, 30, 255]), Point { x: 100.0, y: 0.0 });
    let (angle, distance, blur, stroke_size) = (30.0f64, 20.0, 8.0, 6.0);
    layer.extra.effects = Some(effects(json!({
        "stroke": { "blue": 0, "green": 1, "inside": false, "opacity": 1, "red": 0, "size": stroke_size },
        "shadow": { "angle": angle, "blue": 0, "blur": blur, "distance": distance, "green": 0, "opacity": 0.5, "red": 0 } })));
    doc.layers = vec![layer];
    let now = resize(&doc, new_w, new_h).layers[0].extra.effects.clone().unwrap();

    // A length with no direction (stroke size, a shadow's blur) scales by the square root of the
    // area sx * sy: neither axis factor alone would give this value unless sx happened to equal sy.
    let factor = (sx * sy).sqrt();
    assert_eq!(now.stroke.as_ref().unwrap().size, stroke_size * factor);
    assert_eq!(now.shadow.as_ref().unwrap().blur, blur * factor);

    // The shadow's offset goes through the diagonal map [[sx, 0], [0, sy]] whole: each axis scaled
    // by its own factor, then reassembled into a new angle and distance. Swapping sx and sy here
    // would send this offset off at a different angle and a different length (unless it happened
    // to land on a multiple of 90 degrees, which 30 degrees does not).
    let r = angle.to_radians();
    let (ox, oy) = (-r.cos() * distance, r.sin() * distance);
    let (nx, ny) = (sx * ox, sy * oy);
    let (want_angle, want_distance) = (ny.atan2(-nx).to_degrees(), nx.hypot(ny));
    let s = now.shadow.as_ref().unwrap();
    assert!((s.angle - want_angle).abs() < 1e-6, "angle: {} vs {want_angle}", s.angle);
    assert!((s.distance - want_distance).abs() < 1e-6, "distance: {} vs {want_distance}", s.distance);
}

/// The same non-uniform resize, through a layer that is also flipped or turned, so the linear map
/// `resampled` receives is not diagonal: it exercises `[m.a * sx, m.b * sy, m.c * sx, m.d * sy]`
/// with every one of its four entries doing real work, not just the two on the diagonal.
#[test]
fn image_size_with_a_non_uniform_scale_carries_a_flipped_or_turned_shadow_through_the_full_map() {
    let (doc_w, doc_h) = (120u32, 120u32);
    let (new_w, new_h) = (360u32, 120u32);
    let (sx, sy) = (new_w as f64 / doc_w as f64, new_h as f64 / doc_h as f64);
    assert_ne!(sx, sy, "the point of this test is a resize that is not uniform");

    let mut flipped = bar(30.0); flipped.transform.flip_x = true;
    let mut turned = bar(0.0); turned.transform.rotation = 90.0;
    for (name, layer) in [("flipped", flipped), ("turned", turned)] {
        let s = layer.extra.effects.as_ref().unwrap().shadow.clone().unwrap();
        let r = s.angle.to_radians();
        let m = layer.transform.pixel_to_document(30, 10);
        let (a, b, c, d) = (m.a * sx, m.b * sy, m.c * sx, m.d * sy);
        let (ox, oy) = (-r.cos() * s.distance, r.sin() * s.distance);
        let (vx, vy) = (a * ox + c * oy, b * ox + d * oy);
        let mut doc = Document::new(doc_w, doc_h);
        doc.layers = vec![layer];
        let big = resize(&doc, new_w, new_h);
        let now = big.layers[0].extra.effects.as_ref().unwrap().shadow.clone().unwrap();
        let now_r = now.angle.to_radians();
        let (nx, ny) = (-now_r.cos() * now.distance, now_r.sin() * now.distance);
        assert!((nx - vx).abs() < 1e-6 && (ny - vy).abs() < 1e-6, "{name}: offset ({nx}, {ny}), want ({vx}, {vy})");
        assert!((-360.0..=360.0).contains(&now.angle), "{name}: a valid angle");
    }
}

/// A 30 x 10 white bar at (40, 40) on 120 x 120 with a sharp black shadow at `angle`, 12 px.
fn bar(angle: f64) -> Layer {
    let mut bar = Layer::with_pixels("Bar", solid(30, 10, [255, 255, 255, 255]), Point { x: 40.0, y: 40.0 });
    bar.extra.effects = Some(effects(json!({ "shadow": { "angle": angle, "blue": 0, "blur": 0, "distance": 12, "green": 0, "opacity": 1, "red": 0 } })));
    bar
}

#[test]
fn image_size_keeps_a_flipped_or_turned_layers_shadow_falling_where_it_fell() {
    let mut flipped = bar(30.0); flipped.transform.flip_x = true;
    let mut turned = bar(0.0); turned.transform.rotation = 90.0;
    for (name, layer) in [("flipped", flipped), ("turned", turned)] {
        // Where the shadow falls on the canvas: its layer-pixel offset through the layer's transform.
        let s = layer.extra.effects.as_ref().unwrap().shadow.clone().unwrap();
        let r = s.angle.to_radians();
        let m = layer.transform.pixel_to_document(30, 10);
        let (ox, oy) = (-r.cos() * s.distance, r.sin() * s.distance);
        let (vx, vy) = (2.0 * (m.a * ox + m.c * oy), 2.0 * (m.b * ox + m.d * oy));
        let mut doc = Document::new(120, 120);
        doc.layers = vec![layer];
        let big = resize(&doc, 240, 240);
        let now = big.layers[0].extra.effects.as_ref().unwrap().shadow.clone().unwrap();
        let now_r = now.angle.to_radians();
        let (nx, ny) = (-now_r.cos() * now.distance, now_r.sin() * now.distance);
        assert!((nx - vx).abs() < 1e-6 && (ny - vy).abs() < 1e-6, "{name}: offset ({nx}, {ny}), want ({vx}, {vy})");
        assert!((-360.0..=360.0).contains(&now.angle), "{name}: a valid angle");
    }
    // Pixels: the flipped bar's shadow is centred on the bar's centre plus its canvas offset, and
    // on the doubled canvas at twice both; the mirror image of that point stays empty.
    let mut flipped = bar(30.0); flipped.transform.flip_x = true;
    let r = 30f64.to_radians();
    let (vx, vy) = (r.cos() * 12.0, r.sin() * 12.0); // (-cos, sin) x 12, mirrored by flip_x
    let c = flipped.transform.center();
    let mut doc = Document::new(120, 120);
    doc.layers = vec![flipped];
    let (before, after) = (full(&doc), full(&resize(&doc, 240, 240)));
    assert_eq!(before.pixel((c.x + vx) as u32, (c.y + vy) as u32), [0, 0, 0, 255], "before");
    assert_eq!(after.pixel((2.0 * (c.x + vx)) as u32, (2.0 * (c.y + vy)) as u32), [0, 0, 0, 255], "after: the same place, doubled");
    assert_eq!(after.pixel((2.0 * (c.x - vx)) as u32, (2.0 * (c.y + vy)) as u32), [0, 0, 0, 0], "not mirrored back");
}

#[test]
fn image_size_keeps_effects_within_the_macs_ranges_so_they_stay_drawn() {
    let mut doc = Document::new(100, 100);
    let mut layer = Layer::with_pixels("L", solid(20, 20, [0, 0, 255, 255]), Point { x: 40.0, y: 40.0 });
    layer.extra.effects = Some(effects(json!({
        "stroke": { "blue": 0, "green": 1, "inside": true, "opacity": 1, "red": 0, "size": 400 },
        "shadow": { "angle": 90, "blue": 0, "blur": 300, "distance": 4000, "green": 0, "opacity": 0.5, "red": 0 },
        "outerGlow": { "blue": 1, "green": 1, "opacity": 0.75, "red": 1, "size": 300 } })));
    doc.layers = vec![layer];
    let now = resize(&doc, 200, 200).layers[0].extra.effects.clone().unwrap();
    assert_eq!(now.stroke.as_ref().unwrap().size, 500.0);
    let s = now.shadow.as_ref().unwrap();
    assert_eq!((s.distance, s.blur), (5000.0, 500.0));
    assert_eq!(now.outer_glow.as_ref().unwrap().size, 500.0);
    assert!(now.drawn().is_some(), "clamped, not dropped as invalid");
}

#[test]
fn canvas_size_and_crop_keep_the_effects_with_their_layer() {
    let mut doc = Document::new(120, 120);
    doc.layers = vec![bar(90.0)];
    let before = full(&doc);
    let bigger = ops::canvas_size::canvas_size(&doc, ops::canvas_size::CanvasSizeOptions { width: 140, height: 130, anchor: 0, fill: None, content_offset: Some(Point { x: 20.0, y: 10.0 }) }).unwrap();
    assert_eq!(bigger.layers[0].extra.effects, doc.layers[0].extra.effects);
    assert_eq!(full(&bigger).pixel(55 + 20, 57 + 10), before.pixel(55, 57), "the shadow moved with the bar");
    let cropped = ops::canvas_size::canvas_size(&doc, ops::canvas_size::CanvasSizeOptions { width: 70, height: 60, anchor: 4, fill: None, content_offset: Some(Point { x: -30.0, y: -20.0 }) }).unwrap();
    assert_eq!(full(&cropped).pixel(55 - 30, 57 - 20), before.pixel(55, 57));
}

#[test]
fn a_reduced_preview_draws_its_effects_at_the_size_the_committed_layer_will() {
    // A colour drag previews from a copy no longer than 512 px (preview.rs): 1600 wide halves twice.
    let mut doc = Document::new(1600, 400);
    let mut layer = Layer::with_pixels("Wide", solid(1600, 200, [255, 255, 255, 255]), Point { x: 0.0, y: 0.0 });
    layer.extra.effects = Some(effects(json!({ "shadow": { "angle": 90, "blue": 0, "blur": 0, "distance": 40, "green": 0, "opacity": 1, "red": 0 } })));
    let id = layer.id;
    doc.layers = vec![layer];
    let mut engine = Engine::new();
    let handle = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    let at = |e: &Engine, y: f64| e.composite(handle, Rect { x: 800.0, y, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0);
    let committed = (at(&engine, 230.0), at(&engine, 300.0));
    engine.set_preview(handle, Some(PreviewRequest::DragAdjustment { layer: id, adjustment: LayerAdjustment::new(AdjustmentKind::Levels) })).unwrap();
    assert!(engine.state(handle).unwrap().layers[0].pixels_width <= 512, "the preview is a reduced copy");
    assert_eq!(committed, ([0, 0, 0, 255], [0, 0, 0, 0]), "the shadow reaches 40 px past the layer");
    assert_eq!((at(&engine, 230.0), at(&engine, 300.0)), committed, "and so does the preview's, not 160");
}
