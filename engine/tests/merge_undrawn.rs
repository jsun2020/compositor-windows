//! I1: merge refuses to bake an undrawn feature into merged pixels, permanently, without notice
//! (`Document::undrawn`'s per-layer phrases, via `Layer::undrawn_features`).

use compositor_engine::*;
use serde_json::json;

fn pixel_layer() -> Layer { Layer::with_pixels("P", Raster::from_premultiplied(1, 1, vec![9, 9, 9, 255]), Point { x: 0.0, y: 0.0 }) }

#[test]
fn merge_down_bakes_a_new_blend_mode_as_the_canvas_shows_it() {
    let mut doc = Document::new(1, 1);
    let below = Layer::with_pixels("Below", Raster::from_premultiplied(1, 1, vec![200, 90, 30, 255]), Point { x: 0.0, y: 0.0 });
    let mut above = Layer::with_pixels("Above", Raster::from_premultiplied(1, 1, vec![92, 92, 92, 153]), Point { x: 0.0, y: 0.0 });
    above.blend_mode = BlendMode::SoftLight;
    let above_id = above.id;
    doc.layers = vec![below, above];
    doc.active_layer_id = Some(above_id);
    let shown = composite(&doc, Rect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }, 1, 1).pixel(0, 0);
    ops::merge::merge(&mut doc, &[above_id]).expect("a drawn mode no longer blocks a merge");
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.layers[0].pixels.as_ref().unwrap().pixel(0, 0), shown, "the merged pixel is what the canvas showed");
}

#[test]
fn merging_a_folder_holding_a_motion_blur_layer_is_refused_until_the_motion_probe_settles_it() {
    let mut doc = Document::new(4, 4);
    let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true;
    let folder_id = folder.id;
    let mut p = Layer::with_pixels("P", Raster::from_premultiplied(4, 4, [200u8, 40, 40, 255].repeat(16)), Point { x: 0.0, y: 0.0 });
    p.parent_id = Some(folder_id);
    let mut streak = Layer::blank("Streak", doc.size()); streak.parent_id = Some(folder_id);
    streak.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::MotionBlur));
    doc.layers = vec![folder, p, streak];
    doc.active_layer_id = Some(folder_id);
    let err = ops::merge::merge(&mut doc, &[folder_id]).unwrap_err();
    assert_eq!(err, CommandError::Argument("Merging would bake Motion Blur adjustment layers (drawn approximately), which this build does not draw yet, or draws differently".into()));
    assert_eq!(doc.layers.len(), 3, "nothing changed");
}

#[test]
fn merging_a_folder_holding_a_black_and_white_layer_bakes_it() {
    let mut doc = Document::new(4, 4);
    let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true;
    let folder_id = folder.id;
    let mut red = Layer::with_pixels("Red", Raster::from_premultiplied(4, 4, [200u8, 40, 40, 255].repeat(16)), Point { x: 0.0, y: 0.0 });
    red.parent_id = Some(folder_id);
    let mut bw = Layer::blank("BW", doc.size()); bw.parent_id = Some(folder_id);
    bw.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::BlackWhite));
    doc.layers = vec![folder, red, bw];
    doc.active_layer_id = Some(folder_id);
    ops::merge::merge(&mut doc, &[folder_id]).expect("a drawn kind no longer blocks a merge");
    let p = doc.layers[0].pixels.as_ref().unwrap().pixel(1, 1);
    assert!(p[0] == p[1] && p[1] == p[2] && p[0] != 200, "baked to grey: {p:?}");
}

#[test]
fn merging_enabled_effects_is_refused_but_all_disabled_effects_merge_fine() {
    let mut doc = Document::new(4, 4);
    let mut below = pixel_layer();
    below.extra.effects = Some(json!({ "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } }));
    let above = pixel_layer();
    let above_id = above.id;
    doc.layers = vec![below, above];
    doc.active_layer_id = Some(above_id);
    let before = doc.clone();
    let err = ops::merge::merge(&mut doc, &[above_id]).unwrap_err();
    assert_eq!(err, CommandError::Argument("Merging would bake layer effects, which this build does not draw yet, or draws differently".into()));
    assert_eq!(doc, before, "a refused merge leaves the document untouched");

    let mut doc2 = Document::new(4, 4);
    let mut below2 = pixel_layer();
    below2.extra.effects = Some(json!({ "stroke": { "blue": 1, "enabled": false, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 4 } }));
    let above2 = pixel_layer();
    let above2_id = above2.id;
    doc2.layers = vec![below2, above2];
    doc2.active_layer_id = Some(above2_id);
    assert!(ops::merge::merge(&mut doc2, &[above2_id]).is_ok(), "enabled: false effects do not block a merge, as they do not report undrawn (R 2.1)");
}

#[test]
fn merging_plain_layers_with_a_default_grain_adjustment_still_succeeds() {
    let mut doc = Document::new(4, 4);
    let a = pixel_layer(); let a_id = a.id;
    let b = pixel_layer(); let b_id = b.id;
    let mut grain = Layer::blank("Grain", doc.size());
    grain.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Grain));
    let grain_id = grain.id;
    doc.layers = vec![a, b, grain];
    assert!(ops::merge::merge(&mut doc, &[a_id, b_id, grain_id]).is_ok(), "the grain kernel divergence does not block a merge");
}
