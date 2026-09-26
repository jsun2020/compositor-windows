//! The CPU compositor's peak heap on the destructive paths (export, merge, the histogram's source)
//! with a blur adjustment layer in the document. wasm32 holds at most 4 GiB, and a failed
//! allocation traps the engine, so the bytes per canvas pixel a render needs above the document
//! decide how large a canvas can be exported at all.
//!
//! Its own test binary: the counting global allocator below sees every allocation in the process,
//! and the tests take turns behind `LOCK` so no two measure at once.
use compositor_engine::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

struct Counting;
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grew(bytes: usize) { let now = CURRENT.fetch_add(bytes, Relaxed) + bytes; PEAK.fetch_max(now, Relaxed); }

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() { grew(layout.size()); }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() { grew(layout.size()); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        System.dealloc(p, layout);
        CURRENT.fetch_sub(layout.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Counted as the move it may be: both blocks live at once, then the old one goes.
        let q = System.realloc(p, layout, new_size);
        if !q.is_null() { grew(new_size); CURRENT.fetch_sub(layout.size(), Relaxed); }
        q
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;
static LOCK: Mutex<()> = Mutex::new(());

/// Oblong and odd on both axes, so no halving divides it evenly.
const W: u32 = 601;
const H: u32 = 401;

/// The most bytes `f` held at once beyond what was live when it started, per canvas pixel.
fn peak_per_pixel<T>(f: impl FnOnce() -> T) -> f64 {
    let base = CURRENT.load(Relaxed);
    PEAK.store(base, Relaxed);
    let kept = f();
    let peak = PEAK.load(Relaxed) - base;
    drop(kept);
    peak as f64 / (W * H) as f64
}

/// Colour by column and row, translucent in its right half: a blur has something to move.
fn pattern() -> Raster {
    let mut data = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H { for x in 0..W {
        let a: u32 = if x < W / 2 { 255 } else { 170 };
        data.extend_from_slice(&[(x * 255 / W * a / 255) as u8, (y * 255 / H * a / 255) as u8, ((x + 2 * y) % 256 * a / 255) as u8, a as u8]);
    }}
    Raster::from_premultiplied(W, H, data)
}
fn gaussian(radius: f64) -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::GaussianBlur);
    a.blur_radius = Some(radius);
    a
}
fn motion(distance: f64) -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::MotionBlur);
    a.motion_angle = Some(30.0); a.motion_distance = Some(distance);
    a
}
/// The pattern under one adjustment layer (none: the pattern alone). Returns the document and the
/// top layer's id.
fn document(adjustment: Option<LayerAdjustment>) -> (Document, uuid::Uuid) {
    let mut doc = Document::new(W, H);
    let mut layers = vec![Layer::with_pixels("P", pattern(), Point { x: 0.0, y: 0.0 })];
    if let Some(a) = adjustment {
        let mut layer = Layer::blank("Blur", doc.size());
        layer.extra.adjustment = Some(a);
        layers.push(layer);
    }
    let top = layers.last().unwrap().id;
    doc.layers = layers;
    doc.active_layer_id = Some(top);
    (doc, top)
}

fn composite_peak(adjustment: Option<LayerAdjustment>) -> f64 {
    let (doc, _) = document(adjustment);
    peak_per_pixel(|| render_full(&doc).unwrap())
}
fn export_peak(adjustment: Option<LayerAdjustment>) -> f64 {
    let (doc, _) = document(adjustment);
    peak_per_pixel(|| export_png(&doc).unwrap())
}
fn merge_peak(adjustment: LayerAdjustment) -> f64 {
    let (mut doc, top) = document(Some(adjustment));
    let peak = peak_per_pixel(|| ops::merge::merge(&mut doc, &[top]).unwrap());
    assert_eq!(doc.layers.len(), 1, "the blur merged down");
    peak
}
/// The histogram's source for a Levels layer above the blur: everything beneath it at canvas size.
fn histogram_source_peak(adjustment: LayerAdjustment) -> f64 {
    let (mut doc, _) = document(Some(adjustment));
    let levels = Layer::blank("Levels", doc.size());
    let levels_id = levels.id;
    doc.layers.push(levels);
    doc.layers[2].extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Levels));
    let mut engine = Engine::new();
    let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    peak_per_pixel(|| engine.adjustment_source(id, levels_id).unwrap())
}

/// The measured peak, with room for allocator noise: about 60 KB on this canvas.
const SLACK: f64 = 0.25;

fn at_most(what: &str, measure: impl FnOnce() -> f64, budget: f64) {
    let _turn = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bytes = measure();
    println!("{what}: {bytes:.2} B/px (budget {budget:.2})");
    assert!(bytes <= budget + SLACK, "{what} held {bytes:.2} bytes per pixel at its peak, over its {budget:.2}");
}

// Budgets measured 2026-09-25 on this 601 x 401 canvas. A canvas-size render holds its target
// (4 B/px). A blur adds one working copy (4 B/px) that it writes its result over, and the
// Gaussian's band: 2 * radius + 1 rows of f32 RGBA, 61 rows here, 2.4 B/px at this height (0.1 at
// 10000 x 10000). Before the band, a level-0 Gaussian also held a full-frame f32 copy (16 B/px)
// and a separate result (4 B/px): 28 B/px.

#[test]
fn a_composite_without_a_blur_holds_one_frame() { at_most("composite, no blur", || composite_peak(None), 4.01); }

#[test]
fn a_level_0_gaussian_blur_layer_holds_a_working_copy_and_a_band_not_a_full_frame_of_floats() {
    at_most("composite, Gaussian 10 (level 0)", || composite_peak(Some(gaussian(10.0))), 10.45);
}

#[test]
fn a_halved_gaussian_blur_layer_enlarges_over_its_working_copy() {
    // The lattice (cell 4) pads both frames to 604 x 404; the halvings add 1.25 B/px. Was 13.68.
    at_most("composite, Gaussian 40 (level 2)", || composite_peak(Some(gaussian(40.0))), 10.25);
}

#[test]
fn a_level_0_motion_blur_layer_holds_a_working_copy_and_its_result() {
    // The streak reads in every direction, so it cannot write over what it still reads. Unchanged.
    at_most("composite, Motion 20 (level 0)", || composite_peak(Some(motion(20.0))), 12.01);
}

#[test]
fn a_halved_motion_blur_layer_enlarges_over_its_working_copy() {
    at_most("composite, Motion 60 (level 2)", || composite_peak(Some(motion(60.0))), 9.63); // was 13.68
}

#[test]
fn exporting_a_gaussian_blur_layer_peaks_no_higher_than_the_png_encoder_does_without_one() {
    // The encoder's own buffers set this export's peak, with or without the blur. Was 28.02.
    at_most("export PNG, no blur", || export_peak(None), 13.34);
    at_most("export PNG, Gaussian 10", || export_peak(Some(gaussian(10.0))), 13.54);
}

#[test]
fn merging_a_gaussian_blur_layer_down_holds_a_working_copy_and_a_band() {
    at_most("merge down, Gaussian 10", || merge_peak(gaussian(10.0)), 10.47); // was 28.03
}

#[test]
fn the_histogram_source_beneath_a_gaussian_blur_holds_a_working_copy_and_a_band() {
    at_most("histogram source, Gaussian 10", || histogram_source_peak(gaussian(10.0)), 10.46); // was 28.03
}

// Layer effects (Phase 3.5c). A 437 x 237 layer at (82, 82) under effects whose margin is 82 has
// a padded image of exactly this 601 x 401 canvas, so bytes per canvas pixel are bytes per padded
// pixel. A render without an engine makes each image once and keeps none afterwards; an engine
// keeps up to its cache's limits (`EffectsCache`).

/// The pattern's colours on a 437 x 237 layer.
fn patch() -> Raster {
    let (w, h) = (437u32, 237u32);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h { for x in 0..w {
        let a: u32 = if x < w / 2 { 255 } else { 170 };
        data.extend_from_slice(&[(x * 255 / w * a / 255) as u8, (y * 255 / h * a / 255) as u8, ((x + 2 * y) % 256 * a / 255) as u8, a as u8]);
    }}
    Raster::from_premultiplied(w, h, data)
}
/// The default drop shadow (distance 20, blur 20: margin 82).
fn drop_shadow() -> serde_json::Value { serde_json::json!({ "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } }) }
/// All six, the drop shadow setting the margin (82).
fn all_six() -> serde_json::Value {
    serde_json::json!({
        "stroke": { "blue": 0.3, "green": 0.9, "inside": false, "opacity": 0.8, "red": 0.1, "size": 10 },
        "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 },
        "colorOverlay": { "blue": 0.5, "green": 0.2, "opacity": 0.35, "red": 0.9 },
        "innerShadow": { "angle": -140, "blue": 0, "blur": 6, "distance": 5, "green": 0, "opacity": 0.7, "red": 0.2 },
        "outerGlow": { "blue": 0.2, "green": 0.9, "opacity": 0.75, "red": 1, "size": 20 },
        "innerGlow": { "blue": 1, "green": 1, "opacity": 0.6, "red": 1, "size": 10 } })
}
fn styled(effects: serde_json::Value) -> (Document, uuid::Uuid) {
    let mut doc = Document::new(W, H);
    let below = Layer::with_pixels("Below", pattern(), Point { x: 0.0, y: 0.0 });
    let mut layer = Layer::with_pixels("Styled", patch(), Point { x: 82.0, y: 82.0 });
    layer.extra.effects = Some(serde_json::from_value(effects).unwrap());
    let id = layer.id;
    doc.layers = vec![below, layer];
    doc.active_layer_id = Some(id);
    (doc, id)
}
/// The effects image alone, per padded pixel.
fn effects_image_peak(effects: serde_json::Value) -> f64 {
    let (doc, id) = styled(effects);
    let layer = doc.layer(id).unwrap();
    let fx = effects_draw(layer, None).unwrap();
    let padded = (437 + 2 * fx.inset) as f64 * (237 + 2 * fx.inset) as f64;
    peak_per_pixel(|| effects_image(layer, layer.pixels.as_ref().unwrap(), &fx)) * (W * H) as f64 / padded
}

#[test]
fn an_effects_image_holds_its_output_and_rings_of_rows_not_whole_planes() {
    at_most("effects image, drop shadow", || effects_image_peak(drop_shadow()), 4.80);
    at_most("effects image, all six", || effects_image_peak(all_six()), 6.67);
    at_most("effects image, stroke 500", || effects_image_peak(serde_json::json!({ "stroke": { "blue": 1, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 500 } })), 10.51);
    at_most("effects image, drop shadow blur 90 (halved twice)", || effects_image_peak(serde_json::json!({ "shadow": { "angle": 90, "blue": 0, "blur": 90, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } })), 4.31);
}

#[test]
fn a_composite_with_layer_effects_holds_its_target_and_the_effects_image() {
    at_most("composite, drop shadow", || { let (doc, _) = styled(drop_shadow()); peak_per_pixel(|| render_full(&doc).unwrap()) }, 8.81);
    at_most("composite, all six", || { let (doc, _) = styled(all_six()); peak_per_pixel(|| render_full(&doc).unwrap()) }, 10.68);
}

#[test]
fn exporting_and_merging_layer_effects() {
    at_most("export PNG, drop shadow", || { let (doc, _) = styled(drop_shadow()); peak_per_pixel(|| export_png(&doc).unwrap()) }, 17.46);
    at_most("merge down, drop shadow", || { let (mut doc, id) = styled(drop_shadow()); peak_per_pixel(|| ops::merge::merge(&mut doc, &[id]).unwrap()) }, 8.83);
}

/// The four Gaussian effects, each at level 1 (sigma 20: 3 sigma = 60 > 48).
fn four_gaussians() -> serde_json::Value {
    serde_json::json!({
        "shadow": { "angle": 90, "blue": 0, "blur": 40, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 },
        "innerShadow": { "angle": -140, "blue": 0, "blur": 40, "distance": 5, "green": 0, "opacity": 0.7, "red": 0.2 },
        "outerGlow": { "blue": 0.2, "green": 0.9, "opacity": 0.75, "red": 1, "size": 40 },
        "innerGlow": { "blue": 1, "green": 1, "opacity": 0.6, "red": 1, "size": 40 } })
}

#[test]
fn halved_effect_blurs_hold_one_reduced_plane_each_beside_the_output() {
    at_most("effects image, four Gaussians at level 1", || effects_image_peak(four_gaussians()), 8.15);
}

/// Ten styled layers, each with its own pixels, through one engine: what its cache keeps.
fn ten_styled() -> (Document, uuid::Uuid) {
    let (mut doc, _) = styled(drop_shadow());
    for i in 0..9 {
        let mut layer = Layer::with_pixels(&format!("Styled {i}"), patch(), Point { x: 82.0, y: 82.0 });
        layer.extra.effects = Some(serde_json::from_value(drop_shadow()).unwrap());
        doc.layers.push(layer);
    }
    let top = doc.layers.last().unwrap().id;
    (doc, top)
}

#[test]
fn an_engine_keeps_effects_images_only_up_to_its_cache_limits() {
    let render = |engine: &mut Engine| {
        let (doc, _) = ten_styled();
        let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
        peak_per_pixel(|| engine.composite(id, Rect { x: 0.0, y: 0.0, width: W as f64, height: H as f64 }, W, H).unwrap())
    };
    at_most("composite, ten styled layers, 8 entries kept", || render(&mut Engine::new()), 40.96);
    let image = (W * H * 4) as usize;
    at_most("composite, ten styled layers, 2.5 images of bytes kept", || render(&mut Engine::with_effects_cache(EffectsCache::new(EFFECTS_CACHE_ENTRIES, 2 * image + image / 2))), 16.92);
}