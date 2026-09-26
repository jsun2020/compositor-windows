//! A layer drawn with its effects, as ImageExporter.swift:41-58 draws it: one image in the layer's
//! own pixel grid (the layer's masked pixels with the effects around them), placed by the layer's
//! transform grown in proportion, with the layer's opacity, blend mode and folders (R 4.2).
use compositor_engine::*;
use serde_json::json;

fn effects(value: serde_json::Value) -> LayerEffects { serde_json::from_value(value).unwrap() }
fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Raster { Raster::from_premultiplied(w, h, rgba.repeat((w * h) as usize)) }
fn doc_with(width: u32, height: u32, layers: Vec<Layer>) -> Document { let mut d = Document::new(width, height); d.layers = layers; d }
fn full(doc: &Document) -> Raster { composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height) }
/// A sharp drop shadow (blur 0) at `angle`, `distance` px, in `rgb` at `opacity`.
fn sharp_shadow(angle: f64, distance: f64, rgb: [f64; 3], opacity: f64) -> LayerEffects {
    effects(json!({ "shadow": { "angle": angle, "blur": 0, "distance": distance, "red": rgb[0], "green": rgb[1], "blue": rgb[2], "opacity": opacity } }))
}
fn stroke(size: f64, rgb: [f64; 3]) -> LayerEffects {
    effects(json!({ "stroke": { "inside": false, "size": size, "red": rgb[0], "green": rgb[1], "blue": rgb[2], "opacity": 1 } }))
}

#[test]
fn a_six_pixel_green_stroke_surrounds_a_red_square_as_the_macs_export_test_expects() {
    // ProjectTests.swift:270-311, the Mac's own: a 20 x 20 red square at (20, 20) on 60 x 60.
    let mut square = Layer::with_pixels("Square", solid(20, 20, [255, 0, 0, 255]), Point { x: 20.0, y: 20.0 });
    square.extra.effects = Some(stroke(6.0, [0.0, 1.0, 0.0]));
    let out = full(&doc_with(60, 60, vec![square]));
    assert_eq!(out.pixel(15, 30), [0, 255, 0, 255], "5 px left of the square, inside the stroke");
    assert_eq!(out.pixel(13, 30), [0, 0, 0, 0], "7 px left, past it");
    assert_eq!(out.pixel(30, 30), [255, 0, 0, 255], "the square");
}

/// A 30 x 10 white layer at (40, 40) on 120 x 120 with a sharp black shadow 12 px down (angle 90).
fn shadowed_bar() -> Layer {
    let mut bar = Layer::with_pixels("Bar", solid(30, 10, [255, 255, 255, 255]), Point { x: 40.0, y: 40.0 });
    bar.extra.effects = Some(sharp_shadow(90.0, 12.0, [0.0, 0.0, 0.0], 1.0));
    bar
}

#[test]
fn the_shadow_turns_flips_and_scales_with_its_layer() {
    // The effects are made in the layer's pixel grid and drawn through its transform (R 4.2), so a
    // point 5.5 layer px below the layer's bottom edge is in the shadow wherever the layer puts it.
    let mut turned = shadowed_bar(); turned.transform.rotation = 90.0;
    let mut flipped = shadowed_bar(); flipped.transform.flip_y = true;
    let mut doubled = shadowed_bar(); doubled.transform.size = Size { width: 60.0, height: 20.0 };
    for (name, layer) in [("plain", shadowed_bar()), ("turned 90", turned), ("flipped vertically", flipped), ("doubled", doubled)] {
        let to_doc = layer.transform.pixel_to_document(30, 10);
        let (below, above) = (to_doc.apply(Point { x: 15.5, y: 15.5 }), to_doc.apply(Point { x: 15.5, y: -5.5 }));
        let out = full(&doc_with(120, 120, vec![layer]));
        assert_eq!(out.pixel(below.x as u32, below.y as u32), [0, 0, 0, 255], "{name}: in the shadow at {below:?}");
        assert_eq!(out.pixel(above.x as u32, above.y as u32), [0, 0, 0, 0], "{name}: nothing on the lit side at {above:?}");
    }
}

#[test]
fn the_layers_own_mask_is_baked_first_so_a_stroke_outlines_what_the_mask_leaves() {
    // The right half of a 40 x 20 red layer at (20, 20) is masked out: the stroke runs down the
    // middle, and it is not itself cut by the mask (LayerEffects.swift:434-435, 510-521).
    let hide_right: Vec<u8> = (0..20).flat_map(|_| (0..40u32).map(|x| if x < 20 { 255 } else { 0 })).collect();
    let mut layer = Layer::with_pixels("Half", solid(40, 20, [255, 0, 0, 255]), Point { x: 20.0, y: 20.0 });
    layer.extra.effects = Some(stroke(4.0, [0.0, 1.0, 0.0]));
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(40, 20, hide_right), enabled: true, placement: None, linked: None });
    let out = full(&doc_with(80, 60, vec![layer.clone()]));
    assert_eq!(out.pixel(42, 30), [0, 255, 0, 255], "2 px right of what the mask leaves");
    assert_eq!(out.pixel(50, 30), [0, 0, 0, 0], "past the reach, and masked out");
    assert_eq!(out.pixel(30, 30), [255, 0, 0, 255]);
    layer.mask.as_mut().unwrap().enabled = false;
    let unmasked = full(&doc_with(80, 60, vec![layer]));
    assert_eq!(unmasked.pixel(42, 30), [255, 0, 0, 255], "a disabled mask: the effects follow every pixel");
    assert_eq!(unmasked.pixel(62, 30), [0, 255, 0, 255], "and the stroke goes round the whole layer");
}

#[test]
fn opacity_folders_and_the_blend_mode_apply_to_the_layer_and_its_effects_as_one() {
    let rgb = [0.2, 0.4, 0.6];
    let shadow_px = |opacity: f64| -> [u8; 4] { // the effects image's own shadow pixel (compose kernel)
        let c = opacity as f32;
        let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        [byte(rgb[0] as f32 * c), byte(rgb[1] as f32 * c), byte(rgb[2] as f32 * c), byte(c)]
    };
    // 40% layer in a 50% folder whose mask hides the left half of the canvas.
    let mut folder = Layer::blank("Folder", Size { width: 120.0, height: 120.0 });
    folder.is_group = true;
    folder.opacity = 0.5;
    folder.mask = Some(Mask { pixels: GrayRaster::from_bytes(120, 120, (0..120).flat_map(|_| (0..120u32).map(|x| if x < 45 { 0 } else { 255 })).collect()),
        enabled: true, placement: None, linked: None });
    let mut bar = shadowed_bar();
    bar.extra.effects = Some(sharp_shadow(90.0, 12.0, rgb, 1.0));
    bar.opacity = 0.4;
    bar.parent_id = Some(folder.id);
    let out = full(&doc_with(120, 120, vec![folder, bar]));
    let k = (0.4f64 * 0.5) as f32;
    let want = shadow_px(1.0).map(|v| (v as f32 / 255.0 * k * 255.0).round() as u8);
    assert_eq!(out.pixel(55, 57), want, "the shadow at 40% x 50%");
    assert_eq!(out.pixel(42, 57), [0, 0, 0, 0], "the folder's mask hides the shadow too");
    // Multiply over mid grey, at the shadow's own 50%: the mode applies to the shadow as drawn.
    let grey = Layer::with_pixels("Grey", solid(120, 120, [128, 128, 128, 255]), Point { x: 0.0, y: 0.0 });
    let mut multiplied = shadowed_bar();
    multiplied.extra.effects = Some(sharp_shadow(90.0, 12.0, rgb, 0.5));
    multiplied.blend_mode = BlendMode::Multiply;
    let out = full(&doc_with(120, 120, vec![grey, multiplied]));
    let src = shadow_px(0.5).map(|v| v as f32 / 255.0);
    let (mut multiply, mut normal) = ([128u8, 128, 128, 255], [128u8, 128, 128, 255]);
    compose_u8(&mut multiply, src, BlendMode::Multiply);
    compose_u8(&mut normal, src, BlendMode::Normal);
    assert_ne!(multiply, normal, "the fixture tells the modes apart");
    assert_eq!(out.pixel(55, 57), multiply);
}

#[test]
fn a_clipping_base_shares_its_effects_as_coverage_and_a_clipped_layers_effects_stay_inside() {
    // R 4.1 / 4.2: the stack's coverage is the base as drawOwn draws it, effects included.
    let mut base = Layer::with_pixels("Base", solid(40, 20, [255, 0, 0, 255]), Point { x: 20.0, y: 20.0 });
    base.extra.effects = Some(stroke(4.0, [0.0, 1.0, 0.0]));
    let mut child = Layer::with_pixels("Child", solid(80, 60, [0, 0, 255, 255]), Point { x: 0.0, y: 0.0 });
    child.mask_source_id = Some(base.id);
    let out = full(&doc_with(80, 60, vec![base.clone(), child]));
    assert_eq!(out.pixel(18, 30), [0, 0, 255, 255], "the child shows through the base's stroke");
    assert_eq!(out.pixel(10, 30), [0, 0, 0, 0], "and nowhere past it");
    // A clipped layer's own glow is drawn inside the stack, so it ends at the base's edge.
    let mut glowing = Layer::with_pixels("Glowing", solid(8, 8, [0, 0, 255, 255]), Point { x: 22.0, y: 26.0 });
    glowing.extra.effects = Some(effects(json!({ "outerGlow": { "blue": 1, "green": 1, "opacity": 1, "red": 1, "size": 6 } })));
    glowing.mask_source_id = Some(base.id);
    base.extra.effects = None;
    let out = full(&doc_with(80, 60, vec![base, glowing]));
    assert!(out.pixel(21, 30)[1] > 0, "the glow shows over the base");
    assert_eq!(out.pixel(18, 30), [0, 0, 0, 0], "and not past the base's edge");
}

#[test]
fn folders_adjustment_layers_empty_layers_and_hidden_layers_draw_no_effects() {
    let fx = sharp_shadow(90.0, 12.0, [0.0, 0.0, 0.0], 1.0);
    // The folder and the adjustment layer carry pixels, so only their kind can refuse them.
    let with_pixels = |name: &str| Layer::with_pixels(name, solid(20, 20, [255, 255, 255, 255]), Point { x: 0.0, y: 0.0 });
    let mut adjustment = with_pixels("Levels");
    adjustment.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Levels));
    let mut folder = with_pixels("Folder");
    folder.is_group = true;
    let empty = Layer::blank("Empty", Size { width: 60.0, height: 60.0 });
    for mut layer in [adjustment, folder, empty] {
        layer.extra.effects = Some(fx.clone());
        assert_eq!(effects_draw(&layer, None), None, "{}", layer.name);
    }
    let mut hidden = shadowed_bar();
    hidden.visible = false;
    assert_eq!(full(&doc_with(120, 120, vec![hidden])).pixel(55, 57), [0, 0, 0, 0]);
}

#[test]
fn effects_draw_only_while_the_padded_image_fits_200_megapixels() {
    // LayerEffects.swift:441 (1.2.10): distance 5000 and blur 500 need ceil(5000 + 3 x 500) + 2 on
    // every side; the widest 500-row layer that fits is floor(LIMIT / (500 + 2m)) - 2m.
    let fx = effects(json!({ "shadow": { "angle": 30, "blue": 0, "blur": 500, "distance": 5000, "green": 0, "opacity": 0.5, "red": 0 } }));
    let m = (5000.0f64 + 3.0 * 500.0).ceil() as u64 + 2;
    let widest = EFFECTS_SURFACE_LIMIT / (500 + 2 * m) - 2 * m;
    let layer = |w: u64| { let mut l = Layer::with_pixels("Wide", Raster::new_transparent(w as u32, 500), Point { x: 0.0, y: 0.0 }); l.extra.effects = Some(fx.clone()); l };
    assert!(effects_draw(&layer(widest), None).is_some(), "{widest} x 500 fits");
    assert_eq!(effects_draw(&layer(widest + 1), None), None, "one column more does not, so the layer draws plainly");
}

#[test]
fn an_invalid_shown_effect_leaves_the_layer_drawn_plainly_as_the_mac_does() {
    let mut bar = shadowed_bar();
    let plain = full(&doc_with(120, 120, vec![{ let mut b = bar.clone(); b.extra.effects = None; b }]));
    bar.extra.effects.as_mut().unwrap().shadow.as_mut().unwrap().blur = 501.0;
    assert_eq!(effects_draw(&bar, None), None);
    assert_eq!(full(&doc_with(120, 120, vec![bar])), plain);
}

/// Oblong, translucent right half, colour by column and row; with all six effects, turned 30
/// degrees: every rule above in play at once.
fn busy_document() -> Document {
    let mut data = Vec::new();
    for y in 0..30u32 { for x in 0..40u32 {
        let a = if x < 22 { 255 } else { 150 };
        data.extend_from_slice(&[(x * 6 * a / 255) as u8, (y * 8 * a / 255) as u8, (90 * a / 255) as u8, a as u8]);
    }}
    let mut layer = Layer::with_pixels("Busy", Raster::from_premultiplied(40, 30, data), Point { x: 28.0, y: 21.0 });
    layer.transform.rotation = 30.0;
    layer.extra.effects = Some(effects(json!({
        "stroke": { "blue": 0.2, "green": 0.8, "inside": false, "opacity": 0.9, "red": 0.1, "size": 3 },
        "shadow": { "angle": 120, "blue": 0.3, "blur": 6, "distance": 7, "green": 0.2, "opacity": 0.75, "red": 0.2 },
        "colorOverlay": { "blue": 0.4, "green": 0.1, "opacity": 0.3, "red": 0.9 },
        "innerShadow": { "angle": -35, "blue": 0.05, "blur": 3, "distance": 4, "green": 0.05, "opacity": 0.6, "red": 0.05 },
        "outerGlow": { "blue": 0.2, "green": 0.9, "opacity": 0.6, "red": 1, "size": 5 },
        "innerGlow": { "blue": 1, "green": 1, "opacity": 0.5, "red": 1, "size": 4 } })));
    let mut blur = Layer::blank("Blur", Size { width: 96.0, height: 72.0 });
    let mut a = LayerAdjustment::new(AdjustmentKind::GaussianBlur);
    a.blur_radius = Some(3.0);
    blur.extra.adjustment = Some(a);
    doc_with(96, 72, vec![layer, blur])
}

#[test]
fn a_partial_render_equals_the_same_part_of_the_whole_render_to_the_bit() {
    let doc = busy_document();
    let whole = full(&doc);
    let part = composite(&doc, Rect { x: 13.0, y: 9.0, width: 40.0, height: 30.0 }, 40, 30);
    assert_eq!(part, whole.cropped(13, 9, 40, 30), "a region at 1:1");
    for (x, y) in [(20u32, 15u32), (33, 40), (60, 25), (71, 50)] {
        let one = composite(&doc, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1);
        assert_eq!(one.pixel(0, 0), whole.pixel(x, y), "the eyedroppers' 1 x 1 region at ({x}, {y})");
    }
    // Half size: the effects image is prefiltered like any raster.
    let half = composite(&doc, Rect { x: 0.0, y: 0.0, width: 96.0, height: 72.0 }, 48, 36);
    let half_part = composite(&doc, Rect { x: 16.0, y: 12.0, width: 40.0, height: 30.0 }, 20, 15);
    assert_eq!(half_part, half.cropped(8, 6, 20, 15), "a region at 1:2");
}

#[test]
fn a_pending_distortion_that_keeps_the_rectangle_draws_what_the_plain_transform_draws() {
    let doc = doc_with(120, 120, vec![shadowed_bar()]);
    let layer = &doc.layers[0];
    let edit = PreviewEdit::Layer { id: layer.id, draft: layer.transform, corners: Some(Homography::corners_of(&layer.transform)) };
    let plan = render_plan(&doc, Some(&edit));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!("one plain node") };
    assert!(draw.effects.is_some() && draw.corners.is_some(), "the distortion draws with its effects");
    assert_eq!(composite_edit(&doc, Some(&edit), Rect { x: 0.0, y: 0.0, width: 120.0, height: 120.0 }, 120, 120), full(&doc));
}

/// The whole canvas through `cache`.
fn full_with(doc: &Document, cache: &EffectsCache) -> Raster {
    composite_edit_with(doc, None, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height, cache)
}

#[test]
fn the_cache_makes_each_effects_image_once_for_its_pixels_mask_and_effects() {
    let doc = doc_with(120, 120, vec![shadowed_bar()]);
    let cache = EffectsCache::default();
    let layer = &doc.layers[0];
    let fx = effects_draw(layer, None).unwrap();
    let first = cache.image(layer, &fx).unwrap();
    full_with(&doc, &cache);
    assert_eq!(cache.made(), 1, "a render reuses it");
    assert!(cache.image(&doc.clone().layers[0], &fx).unwrap().same_pixels(&first), "a copy of the document (a history snapshot) shares its buffers, so it finds the image");
    // Other pixels under the same revision number (as an undo followed by a new edit can give):
    // the draw is equal, but the image follows the pixels themselves, not the number.
    let mut other = layer.clone();
    other.pixels = Some(solid(30, 10, [255, 255, 255, 128]));
    let other_fx = effects_draw(&other, None).unwrap();
    assert_eq!(other_fx, fx);
    let remade = cache.image(&other, &other_fx).unwrap();
    assert_eq!(cache.made(), 2);
    assert_eq!(remade, effects_image(&other, other.pixels.as_ref().unwrap(), &other_fx));
    assert_ne!(remade, first);
}

#[test]
fn a_ninth_styled_layer_evicts_the_least_recently_drawn_which_is_then_made_again_exactly() {
    let cache = EffectsCache::default();
    let bars: Vec<Layer> = (0..9).map(|_| shadowed_bar()).collect();
    let draws: Vec<EffectsDraw> = bars.iter().map(|b| effects_draw(b, None).unwrap()).collect();
    let kept: Vec<Raster> = (0..8).map(|i| cache.image(&bars[i], &draws[i]).unwrap()).collect();
    cache.image(&bars[0], &draws[0]);
    cache.image(&bars[8], &draws[8]);
    assert_eq!((cache.len(), cache.made()), (EFFECTS_CACHE_ENTRIES, 9));
    assert!(cache.image(&bars[0], &draws[0]).unwrap().same_pixels(&kept[0]), "the one drawn again stayed");
    assert_eq!(cache.made(), 9);
    let remade = cache.image(&bars[1], &draws[1]).unwrap();
    assert_eq!(cache.made(), 10, "the least recently drawn went");
    assert!(!remade.same_pixels(&kept[1]));
    assert_eq!(remade, kept[1], "and is made again exactly as it was");
}

#[test]
fn images_past_the_byte_limit_evict_and_an_image_larger_than_the_limit_is_never_kept() {
    let bars: Vec<Layer> = (0..3).map(|_| shadowed_bar()).collect();
    let draws: Vec<EffectsDraw> = bars.iter().map(|b| effects_draw(b, None).unwrap()).collect();
    let bytes = ((30 + 2 * draws[0].inset) * (10 + 2 * draws[0].inset) * 4) as usize;
    let two = EffectsCache::new(EFFECTS_CACHE_ENTRIES, 2 * bytes + bytes / 2);
    for i in 0..3 { two.image(&bars[i], &draws[i]); }
    assert_eq!((two.len(), two.bytes()), (2, 2 * bytes), "the third image pushed the first out");
    two.image(&bars[0], &draws[0]);
    assert_eq!(two.made(), 4);
    let none = EffectsCache::new(EFFECTS_CACHE_ENTRIES, bytes - 1);
    let (a, b) = (none.image(&bars[0], &draws[0]).unwrap(), none.image(&bars[0], &draws[0]).unwrap());
    assert_eq!((none.len(), none.made()), (0, 2), "too large to keep: made for each use");
    assert_eq!(a, b);
}

#[test]
fn an_image_larger_than_the_limit_leaves_the_images_already_kept_in_place() {
    // Never stored, so it cannot push anything out (LayerEffects.swift:389): the small bar stays.
    let small = shadowed_bar();
    let mut large = Layer::with_pixels("Large", solid(60, 20, [255, 255, 255, 255]), Point { x: 30.0, y: 40.0 });
    large.extra.effects = small.extra.effects.clone();
    let (small_fx, large_fx) = (effects_draw(&small, None).unwrap(), effects_draw(&large, None).unwrap());
    let small_bytes = ((30 + 2 * small_fx.inset) * (10 + 2 * small_fx.inset) * 4) as usize;
    let large_bytes = ((60 + 2 * large_fx.inset) * (20 + 2 * large_fx.inset) * 4) as usize;
    let cap = small_bytes + small_bytes / 2;
    assert!(large_bytes > cap, "the fixture: the large image alone is over the limit");
    let cache = EffectsCache::new(EFFECTS_CACHE_ENTRIES, cap);
    let kept = cache.image(&small, &small_fx).unwrap();
    let first = cache.image(&large, &large_fx).unwrap();
    assert_eq!((cache.len(), cache.bytes(), cache.made()), (1, small_bytes, 2), "only the small image is kept");
    assert!(cache.image(&small, &small_fx).unwrap().same_pixels(&kept), "and it is still the image first made");
    assert_eq!(cache.made(), 2);
    let second = cache.image(&large, &large_fx).unwrap();
    assert_eq!(cache.made(), 3, "the large image was not kept: made again");
    assert!(!second.same_pixels(&first));
    assert_eq!(second, first, "and made again exactly");
}

/// Final review M1: a CPU reading of a small region makes the effects images of the styled layers
/// it touches, not of every one. With more styled layers than the cache keeps, each eyedropper
/// click would otherwise make them all again.
#[test]
fn an_eyedropper_sample_makes_only_the_effects_images_under_it() {
    let n = EFFECTS_CACHE_ENTRIES + 1;
    // 10 x 10 squares 20 px apart with a 2 px stroke: each padded image spans 18 px (margin 4).
    let squares: Vec<Layer> = (0..n).map(|i| {
        let mut l = Layer::with_pixels("Square", solid(10, 10, [255, 0, 0, 255]), Point { x: 20.0 * i as f64 + 5.0, y: 5.0 });
        l.extra.effects = Some(stroke(2.0, [0.0, 1.0, 0.0]));
        l
    }).collect();
    let doc = doc_with(20 * n as u32, 20, squares);
    let mut engine = Engine::new();
    let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    assert_eq!(engine.sample_color(id, Point { x: 90.5, y: 10.5 }).unwrap(), Some([1.0, 0.0, 0.0]), "the fifth square");
    assert_eq!(engine.effects_cache().made(), 1, "only the image under the point");
    assert_eq!(engine.sample_color(id, Point { x: 83.5, y: 10.5 }).unwrap(), Some([0.0, 1.0, 0.0]), "its stroke");
    assert_eq!(engine.effects_cache().made(), 1, "the same image, kept");
}

/// Final review M2: an image whose pixel or mask buffer only the cache holds can never be found
/// again, so pruning drops it, and keeps every image whose buffers live elsewhere.
#[test]
fn pruning_drops_the_images_whose_buffers_only_the_cache_holds() {
    let cache = EffectsCache::default();
    let live = shadowed_bar();
    let gone = shadowed_bar();
    let mut masked = shadowed_bar();
    masked.mask = Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![255, 0, 0, 255]), enabled: true, placement: None, linked: None });
    for layer in [&live, &gone, &masked] { cache.image(layer, &effects_draw(layer, None).unwrap()); }
    assert_eq!((cache.len(), cache.made()), (3, 3));
    drop(gone);
    masked.mask = Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![0, 255, 255, 0]), enabled: true, placement: None, linked: None });
    let fresh = shadowed_bar();
    cache.image(&fresh, &effects_draw(&fresh, None).unwrap());
    assert_eq!(cache.len(), 2, "making a new image first dropped the dropped layer's and the replaced mask's");
    cache.image(&live, &effects_draw(&live, None).unwrap());
    assert_eq!(cache.made(), 4, "the live one stayed");
}

/// Re-review of the final fix wave: entries made from one pixel buffer (the same layer at two
/// mask placements, or with two effect sets) hold that buffer between them, so counting holders
/// must discount the cache's own entries, or they keep one another alive after the layer is gone.
#[test]
fn entries_that_share_a_buffer_do_not_keep_one_another_alive() {
    let cache = EffectsCache::default();
    let shadowed = shadowed_bar();
    let mut stroked = shadowed.clone();
    stroked.extra.effects = Some(stroke(2.0, [0.0, 1.0, 0.0]));
    assert!(stroked.pixels.as_ref().unwrap().same_pixels(shadowed.pixels.as_ref().unwrap()), "one pixel buffer");
    for layer in [&shadowed, &stroked] { cache.image(layer, &effects_draw(layer, None).unwrap()); }
    assert_eq!(cache.len(), 2);
    cache.prune();
    assert_eq!(cache.len(), 2, "the layers still hold the buffer");
    drop(shadowed);
    drop(stroked);
    cache.prune();
    assert_eq!(cache.len(), 0, "only the two entries held it, so both go");
}

#[test]
fn closing_a_document_drops_its_effects_images() {
    let doc = doc_with(120, 120, vec![shadowed_bar()]);
    let mut engine = Engine::new();
    let (a, b) = (engine.open_package(&save_package(&doc).unwrap(), None).unwrap(), engine.open_package(&save_package(&doc).unwrap(), None).unwrap());
    let draw = |e: &Engine, id| e.composite(id, Rect { x: 0.0, y: 0.0, width: 120.0, height: 120.0 }, 120, 120).unwrap();
    draw(&engine, a);
    draw(&engine, b);
    assert_eq!(engine.effects_cache().len(), 2, "each open copy has its own buffers");
    engine.close_document(a);
    assert_eq!(engine.effects_cache().len(), 1, "the closed one's image went");
    draw(&engine, b);
    assert_eq!(engine.effects_cache().made(), 2, "the open one's stayed");
    engine.close_document(b);
    assert_eq!(engine.effects_cache().len(), 0);
}

#[test]
fn the_engine_keeps_effects_images_across_edits_that_do_not_change_them_and_across_undo() {
    let doc = doc_with(120, 120, vec![shadowed_bar()]);
    let bar = doc.layers[0].id;
    let mut engine = Engine::new();
    let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    let draw = |e: &Engine| e.composite(id, Rect { x: 0.0, y: 0.0, width: 120.0, height: 120.0 }, 120, 120).unwrap();
    let before = draw(&engine);
    engine.execute(id, Command::SetLayerOpacity { id: bar, opacity: 0.5 }).unwrap();
    draw(&engine);
    assert_eq!(engine.effects_cache().made(), 1, "opacity does not remake the image");
    engine.execute(id, Command::InvertPixels { id: bar, mask: false }).unwrap();
    draw(&engine);
    assert_eq!(engine.effects_cache().made(), 2, "new pixels, a new image");
    engine.undo(id).unwrap();
    engine.undo(id).unwrap();
    assert_eq!(draw(&engine), before);
    assert_eq!(engine.effects_cache().made(), 2, "undo brings back pixels whose image is kept");
}

#[test]
fn moving_a_layer_keeps_its_effects_image_unless_its_mask_stays_put() {
    let mut bar = shadowed_bar();
    let draw = |layer: &Layer, edit: Option<&PreviewEdit>| effects_draw(layer, edit).unwrap();
    let mut moved = bar.transform; moved.origin.x += 7.0; moved.rotation = 20.0;
    let drag = PreviewEdit::Layer { id: bar.id, draft: moved, corners: None };
    assert_eq!(draw(&bar, Some(&drag)), draw(&bar, None), "no mask: the image does not depend on where the layer is");
    let mut apart = bar.transform; apart.origin.x += 3.0; apart.origin.y += 2.0;
    bar.mask = Some(Mask { pixels: GrayRaster::from_bytes(4, 4, (0..16).map(|i| (i * 16) as u8).collect()), enabled: true, placement: Some(apart), linked: Some(false) });
    assert_ne!(draw(&bar, Some(&drag)).key, draw(&bar, None).key, "an unlinked mask stays put while the layer moves under it");
    bar.mask.as_mut().unwrap().linked = None;
    assert_eq!(draw(&bar, Some(&drag)), draw(&bar, None), "a linked mask moves and turns with its layer: the same image");
    let mask_drag = PreviewEdit::Mask { id: bar.id, draft: moved };
    assert_ne!(draw(&bar, Some(&mask_drag)).key, draw(&bar, None).key, "dragging the mask itself makes a new one");
}

#[test]
fn a_layer_clipped_to_a_styled_layer_further_down_shows_through_its_effects() {
    // Not directly above its source, so the plan draws it on its own with `clip`, and the
    // compositor reads the source's effects image through `clip_source_rasters`, not a stack.
    let mut base = Layer::with_pixels("Base", solid(40, 20, [255, 0, 0, 255]), Point { x: 20.0, y: 20.0 });
    base.extra.effects = Some(stroke(4.0, [0.0, 1.0, 0.0]));
    let between = Layer::with_pixels("Between", solid(6, 6, [200, 200, 0, 255]), Point { x: 70.0, y: 2.0 });
    let target = Layer::with_pixels("Target", solid(80, 60, [0, 0, 255, 255]), Point { x: 0.0, y: 0.0 });
    let (base_id, target_id) = (base.id, target.id);
    let mut doc = doc_with(80, 60, vec![base, between, target]);
    ops::hierarchy::link_mask(&mut doc, base_id, target_id).unwrap();
    let plan = render_plan(&doc, None);
    assert!(plan.nodes.iter().any(|n| matches!(n, PlanNode::Layer { draw } if draw.id == target_id && draw.clip == Some(base_id))), "drawn on its own, clipped");
    let out = full(&doc);
    assert_eq!(out.pixel(18, 30), [0, 0, 255, 255], "in the source's stroke: the target");
    assert_eq!(out.pixel(15, 30), [0, 0, 0, 0], "just past the stroke: nothing");
}

#[test]
fn the_engine_hands_out_the_raster_each_draw_samples() {
    let mut doc = doc_with(120, 120, vec![shadowed_bar()]);
    let plain = Layer::with_pixels("Plain", solid(7, 5, [9, 90, 200, 255]), Point { x: 3.0, y: 3.0 });
    doc.layers.push(plain);
    let (bar, plain) = (doc.layers[0].id, doc.layers[1].id);
    let mut engine = Engine::new();
    let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    let fx = effects_draw(engine.document(id).unwrap().layer(bar).unwrap(), None).unwrap();
    let image = engine.draw_raster(id, bar, 0, None).unwrap().unwrap();
    assert_eq!((image.width, image.height), (30 + 2 * fx.inset, 10 + 2 * fx.inset), "the padded image");
    let sampled = engine.effects_cache().image(engine.document(id).unwrap().layer(bar).unwrap(), &fx).unwrap();
    assert!(image.same_pixels(&sampled), "the one the CPU samples, from the engine's cache");
    let halved = engine.draw_raster(id, bar, 1, None).unwrap().unwrap();
    assert_eq!(halved, image.halved(), "halved like any layer raster");
    assert_eq!(engine.draw_raster(id, plain, 0, None).unwrap().unwrap(), solid(7, 5, [9, 90, 200, 255]), "a layer without effects: its pixels");
    assert_eq!(engine.layer_raster(id, bar, 0).unwrap().unwrap(), solid(30, 10, [255, 255, 255, 255]), "layer_raster still gives the layer's own pixels");
    // An image the cache cannot keep is made for each call and dropped by the engine, so the wasm
    // bridge keeps the raster it hands the GPU (`prepare_draw_pixels`).
    let mut keeps_none = Engine::with_effects_cache(EffectsCache::new(EFFECTS_CACHE_ENTRIES, 0));
    let id = keeps_none.open_package(&save_package(&doc).unwrap(), None).unwrap();
    let (a, b) = (keeps_none.draw_raster(id, bar, 0, None).unwrap().unwrap(), keeps_none.draw_raster(id, bar, 0, None).unwrap().unwrap());
    assert_eq!((keeps_none.effects_cache().len(), keeps_none.effects_cache().made()), (0, 2));
    assert_eq!(a, image);
    assert_eq!(b, image);
}
