//! Every path that turns the renderer's output into data or a reading (LL-070): delete-with-bake,
//! export, the histogram's source and the eyedroppers see layer effects as the canvas draws them.
//! Merge is in merge_undrawn.rs.
use compositor_engine::*;
use serde_json::json;

fn effects(value: serde_json::Value) -> LayerEffects { serde_json::from_value(value).unwrap() }
fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Raster { Raster::from_premultiplied(w, h, rgba.repeat((w * h) as usize)) }
fn full(doc: &Document) -> Raster { composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height) }

/// A 30 x 10 white bar at (40, 40) on 120 x 120 with a sharp drop shadow 12 px down, opaque, in
/// (0.2, 0.4, 0.6): so the shadow at (55, 57) is exactly that colour.
fn shadowed_document() -> Document {
    let mut bar = Layer::with_pixels("Bar", solid(30, 10, [255, 255, 255, 255]), Point { x: 40.0, y: 40.0 });
    bar.extra.effects = Some(effects(json!({ "shadow": { "angle": 90, "blue": 0.6, "blur": 0, "distance": 12, "green": 0.4, "opacity": 1, "red": 0.2 } })));
    let mut doc = Document::new(120, 120);
    doc.layers = vec![bar];
    doc
}
/// The compose kernel's byte for a colour at full coverage.
fn byte(v: f64) -> u8 { ((v as f32).clamp(0.0, 1.0) * 255.0 + 0.5) as u8 }

#[test]
fn deleting_a_clipping_base_bakes_its_effects_into_the_clipped_layer_as_they_were_drawn() {
    // The canvas's coverage includes the base's stroke (R 4.1), so "Bake keeps the current masked
    // appearance" (LiveLayerMask.swift:110) holds only if the bake includes it too. The Mac's own
    // LiveMaskBaker draws the base without its effects (:88-90): this port keeps the appearance.
    let mut base = Layer::with_pixels("Base", solid(40, 20, [255, 0, 0, 255]), Point { x: 20.0, y: 20.0 });
    base.extra.effects = Some(effects(json!({ "stroke": { "blue": 0, "green": 1, "inside": false, "opacity": 1, "red": 0, "size": 4 } })));
    let base_id = base.id;
    let mut child = Layer::with_pixels("Child", solid(80, 60, [0, 0, 255, 255]), Point { x: 0.0, y: 0.0 });
    child.mask_source_id = Some(base_id);
    let mut doc = Document::new(80, 60);
    doc.layers = vec![base, child];
    let before = full(&doc);
    assert_eq!(before.pixel(18, 30), [0, 0, 255, 255], "the child shows through the stroke");
    ops::hierarchy::delete_layers(&mut doc, &[base_id], true).unwrap();
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(full(&doc), before, "the baked layer looks exactly as the stack did");
}

#[test]
fn export_draws_the_effects() {
    let doc = shadowed_document();
    let png = export_png(&doc).unwrap();
    let exported = decode_image(&png).unwrap().raster;
    assert_eq!(exported.pixel(55, 57), [byte(0.2), byte(0.4), byte(0.6), 255], "the shadow");
    assert_eq!(exported.pixel(55, 35), [0, 0, 0, 0], "above the bar");
}

/// The shadowed bar with a Levels adjustment layer on top, opened in an engine; returns the engine,
/// the document handle and the adjustment layer's id.
fn under_levels() -> (Engine, uuid::Uuid, uuid::Uuid) {
    let mut doc = shadowed_document();
    let mut levels = Layer::blank("Levels", doc.size());
    levels.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Levels));
    let levels_id = levels.id;
    doc.layers.push(levels);
    let mut engine = Engine::new();
    let id = engine.open_package(&save_package(&doc).unwrap(), None).unwrap();
    (engine, id, levels_id)
}

#[test]
fn the_histogram_reads_what_lies_beneath_with_its_effects() {
    let (engine, id, levels) = under_levels();
    let source = engine.adjustment_source(id, levels).unwrap();
    assert_eq!(source.pixel(55, 57), [byte(0.2), byte(0.4), byte(0.6), 255]);
}

#[test]
fn the_eyedroppers_read_the_effects_as_drawn() {
    let (engine, id, levels) = under_levels();
    let want = [byte(0.2), byte(0.4), byte(0.6)].map(|v| v as f64 / 255.0);
    assert_eq!(engine.sample_color(id, Point { x: 55.5, y: 57.5 }).unwrap(), Some(want), "the Hue/Saturation eyedropper: the composite");
    assert_eq!(engine.sample_layer_color(id, levels, Point { x: 55.5, y: 57.5 }).unwrap(), Some(want), "an adjustment layer's eyedropper: what lies beneath");
    // A pixel layer's own eyedropper reads its own pixels (destructive Levels works on those), and
    // the shadow is not in them.
    let bar = engine.state(id).unwrap().layers[0].id;
    assert_eq!(engine.sample_layer_color(id, bar, Point { x: 55.5, y: 57.5 }).unwrap(), None);
}
