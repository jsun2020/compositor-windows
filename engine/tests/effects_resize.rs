//! Layer effects through Image Size, Canvas Size, Crop and reduced previews (Phase 3.5c rulings).
//! Mac v1.4.9 ImageResizer.swift retains effects and scales lengths by sqrt(sx*sy),
//! keeping effect angles after rasterization. This replaces the earlier Windows full-map offset rule.
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

/// Unequal factors distinguish geometric-mean styles from either individual canvas axis.
#[test]
fn image_size_nonuniform_styles_use_mac149_geometric_mean_and_keep_angle() {
    let mut doc = Document::new(300, 100);
    let mut layer = Layer::with_pixels(
        "L",
        solid(100, 100, [10, 20, 30, 255]),
        Point { x: 100., y: 0. },
    );
    layer.extra.effects = Some(effects(
        json!({"stroke":{"red":0,"green":1,"blue":0,"size":6,"opacity":1,"inside":false},"shadow":{"red":0,"green":0,"blue":0,"angle":30,"distance":20,"blur":8,"opacity":0.5}}),
    ));
    doc.layers = vec![layer];
    let now = resize(&doc, 600, 150).layers[0]
        .extra
        .effects
        .clone()
        .unwrap();
    let factor = 3f64.sqrt();
    assert_eq!(now.stroke.as_ref().unwrap().size, 6. * factor);
    let shadow = now.shadow.as_ref().unwrap();
    assert_eq!(shadow.angle, 30.);
    assert_eq!(shadow.distance, 20. * factor);
    assert_eq!(shadow.blur, 8. * factor);
}

/// v1.4.9 bakes flips/rotations into pixels, retaining the effect parameter direction.
#[test]
fn image_size_nonuniform_rotated_flipped_layers_keep_mac149_effect_parameter_direction() {
    let mut flipped = bar(30.);
    flipped.transform.flip_x = true;
    let mut turned = bar(0.);
    turned.transform.rotation = 90.;
    for (name, layer) in [("flipped", flipped), ("turned", turned)] {
        let original = layer
            .extra
            .effects
            .as_ref()
            .unwrap()
            .shadow
            .clone()
            .unwrap();
        let mut doc = Document::new(120, 120);
        doc.layers = vec![layer];
        let big = resize(&doc, 360, 120);
        let now = big.layers[0]
            .extra
            .effects
            .as_ref()
            .unwrap()
            .shadow
            .clone()
            .unwrap();
        assert_eq!(now.angle, original.angle, "{name}");
        assert_eq!(now.distance, original.distance * 3f64.sqrt(), "{name}");
        assert_eq!(big.layers[0].transform.rotation, 0.);
        assert!(!big.layers[0].transform.flip_x);
    }
}

/// A 30 x 10 white bar at (40, 40) on 120 x 120 with a sharp black shadow at `angle`, 12 px.
fn bar(angle: f64) -> Layer {
    let mut bar = Layer::with_pixels("Bar", solid(30, 10, [255, 255, 255, 255]), Point { x: 40.0, y: 40.0 });
    bar.extra.effects = Some(effects(json!({ "shadow": { "angle": angle, "blue": 0, "blur": 0, "distance": 12, "green": 0, "opacity": 1, "red": 0 } })));
    bar
}

#[test]
fn image_size_bakes_pixel_transform_and_keeps_mac149_style_angle_after_uniform_resize() {
    let mut flipped = bar(30.);
    flipped.transform.flip_x = true;
    let mut turned = bar(0.);
    turned.transform.rotation = 90.;
    for (name, layer) in [("flipped", flipped), ("turned", turned)] {
        let old = layer
            .extra
            .effects
            .as_ref()
            .unwrap()
            .shadow
            .clone()
            .unwrap();
        let mut doc = Document::new(120, 120);
        doc.layers = vec![layer];
        let big = resize(&doc, 240, 240);
        let now = big.layers[0]
            .extra
            .effects
            .as_ref()
            .unwrap()
            .shadow
            .clone()
            .unwrap();
        assert_eq!(now.angle, old.angle, "{name}");
        assert_eq!(now.distance, old.distance * 2., "{name}");
        assert_eq!(big.layers[0].transform.rotation, 0.);
        assert!(!big.layers[0].transform.flip_x);
    }
    let mut flipped = bar(30.);
    flipped.transform.flip_x = true;
    let center = flipped.transform.center();
    let r = 30f64.to_radians();
    let (vx, vy) = (r.cos() * 12., r.sin() * 12.);
    let mut doc = Document::new(120, 120);
    doc.layers = vec![flipped];
    assert_eq!(
        full(&doc).pixel((center.x + vx) as u32, (center.y + vy) as u32),
        [0, 0, 0, 255]
    );
    let after = full(&resize(&doc, 240, 240));
    assert_eq!(
        after.pixel((2. * (center.x - vx)) as u32, (2. * (center.y + vy)) as u32),
        [0, 0, 0, 255],
        "v1.4.9 keeps the style angle on the rasterized layer"
    );
    assert_eq!(
        after.pixel((2. * (center.x + vx)) as u32, (2. * (center.y + vy)) as u32),
        [0, 0, 0, 0]
    );
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

/// The `effects-invalid.comp` probe's effects: a stroke past the Mac's 500 and a valid shadow. The
/// Mac draws such a layer without any of them (`isValid`, LayerEffects.swift:135-139).
fn invalid_stroke_and_a_shadow() -> LayerEffects {
    effects(json!({ "stroke": { "blue": 0, "green": 0, "inside": false, "opacity": 1, "red": 1, "size": 600 },
        "shadow": { "angle": 90, "blue": 0, "blur": 9, "distance": 14, "green": 0, "opacity": 0.7, "red": 0 } }))
}

/// Final review M4: clamping the stroke into range (600 x 2 -> 500, 600 x 0.5 -> 300) would make the
/// set valid, and the layer would suddenly draw all its effects and save the new values.
#[test]
fn image_size_leaves_an_invalid_effect_invalid_so_the_layer_stays_plain() {
    let mut doc = Document::new(200, 100);
    let mut layer = Layer::with_pixels("L", solid(80, 40, [0, 0, 255, 255]), Point { x: 60.0, y: 30.0 });
    layer.extra.effects = Some(invalid_stroke_and_a_shadow());
    doc.layers = vec![layer];
    for (w, h) in [(400, 200), (100, 50)] {
        let k = w as f64 / 200.0;
        let out = resize(&doc, w, h);
        let now = out.layers[0].extra.effects.clone().unwrap();
        assert_eq!(now.stroke.as_ref().unwrap().size, 600.0, "x{k}: the invalid stroke is carried as it is");
        assert_eq!(now.shadow.as_ref().unwrap().distance, 14.0 * k, "x{k}: the valid shadow still scales");
        assert!(now.drawn().is_none() && effects_draw(&out.layers[0], None).is_none(), "x{k}: still undrawn");
        let left = (60.0 * k) as u32 - 2;
        assert_eq!(full(&out).pixel(left, (50.0 * k) as u32), [0, 0, 0, 0], "x{k}: no stroke left of the layer");
    }
    // A shadow's angle past 360 is invalid too; aimed through the resize it would come out in range.
    doc.layers[0].extra.effects = Some(effects(json!({ "shadow": { "angle": 400, "blue": 0, "blur": 0, "distance": 14, "green": 0, "opacity": 1, "red": 0 } })));
    let now = resize(&doc, 400, 200).layers[0].extra.effects.clone().unwrap();
    assert_eq!((now.shadow.as_ref().unwrap().angle, now.shadow.as_ref().unwrap().distance), (400.0, 14.0), "the invalid shadow is carried as it is");
    assert!(now.drawn().is_none());
}

#[test]
fn a_reduced_preview_leaves_an_invalid_effect_invalid_so_the_layer_stays_plain() {
    // A colour drag previews from a copy no longer than 512 px (preview.rs): 1600 wide halves twice,
    // so the stroke would come out at 150 reduced pixels, valid, reaching 600 document px.
    let mut doc = Document::new(1600, 400);
    let mut layer = Layer::with_pixels("Wide", solid(1600, 200, [255, 255, 255, 255]), Point { x: 0.0, y: 100.0 });
    layer.extra.effects = Some(invalid_stroke_and_a_shadow());
    let id = layer.id;
    doc.layers = vec![layer];
    let mut engine = Engine::new();
    let handle = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    let above = |e: &Engine| e.composite(handle, Rect { x: 800.0, y: 50.0, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0);
    assert_eq!(above(&engine), [0, 0, 0, 0], "committed: drawn plainly");
    engine.set_preview(handle, Some(PreviewRequest::DragAdjustment { layer: id, adjustment: LayerAdjustment::new(AdjustmentKind::Levels) })).unwrap();
    assert!(engine.state(handle).unwrap().layers[0].pixels_width <= 512, "the preview is a reduced copy");
    let plan = engine.render_plan(handle, None).unwrap();
    assert!(plan.nodes.iter().all(|n| !matches!(n, PlanNode::Layer { draw } if draw.effects.is_some())), "the preview draws no effects");
    assert_eq!(above(&engine), [0, 0, 0, 0], "and no stroke 50 px above the layer");
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
