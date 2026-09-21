// Composite order and history hygiene: the three Phase 2 gestures that leave the layer array
// out of hierarchy order, and the selection command that must not touch history.
use compositor_engine::ops::{hierarchy, layers};
use compositor_engine::*;

fn doc() -> Document { Document::new(40, 40) }

/// An opaque square of one colour, 8x8, placed at the origin so all of them overlap exactly.
fn square(rgb: [u8; 3]) -> Raster {
    Raster::from_premultiplied(8, 8, [rgb[0], rgb[1], rgb[2], 255].repeat(64))
}
fn add(d: &mut Document, name: &str, rgb: [u8; 3]) -> uuid::Uuid {
    layers::import_raster(d, square(rgb), name, Some(Point { x: 4.0, y: 4.0 })).unwrap()
}
/// The colour the compositor puts at the top-left pixel: whichever layer drew last wins.
fn top_pixel(d: &Document) -> [u8; 3] {
    let r = compositor::composite(d, Rect { x: 0.0, y: 0.0, width: 8.0, height: 8.0 }, 1, 1);
    let p = r.pixel(0, 0);
    [p[0], p[1], p[2]]
}

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];

#[test]
fn grouping_the_bottom_layers_keeps_the_top_layer_on_top() {
    let mut d = doc();
    let a = add(&mut d, "A", RED);
    let b = add(&mut d, "B", GREEN);
    let c = add(&mut d, "C", BLUE);
    let g = hierarchy::group_layers(&mut d, &[a, b]).unwrap();
    // The array is Mac-compatible: the wrapper takes the selection's place and the children
    // are appended after it. Hierarchy order, not array order, is what draws.
    assert_eq!(d.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![g, c, a, b]);
    assert_eq!(d.render_ids(), vec![a, b, c], "the folder's contents draw at the folder's place");
    assert_eq!(top_pixel(&d), BLUE, "C was on top before grouping and stays on top");
}

#[test]
fn a_layer_dropped_onto_a_folder_draws_inside_it() {
    let mut d = doc();
    let p = add(&mut d, "P", RED);
    let q = add(&mut d, "Q", GREEN);
    d.active_layer_id = Some(q);
    let f = hierarchy::group_layers(&mut d, &[q]).unwrap();
    let r = add(&mut d, "R", BLUE);
    d.active_layer_id = Some(r);
    // The drag path: dropping onto a folder row is parent = folder, no target, not at bottom.
    hierarchy::place_layer(&mut d, r, Some(f), None, false).unwrap();
    assert_eq!(d.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![p, f, q, r]);
    assert_eq!(d.render_ids(), vec![p, q, r], "R lands at the top of the folder, above Q");
    assert_eq!(top_pixel(&d), BLUE);
}

#[test]
fn an_import_into_a_folder_draws_inside_it() {
    let mut d = doc();
    let h1 = add(&mut d, "H1", RED);
    d.active_layer_id = Some(h1);
    let f = hierarchy::group_layers(&mut d, &[h1]).unwrap();
    d.active_layer_id = None;
    let h2 = add(&mut d, "H2", GREEN);
    // A folder is active, so the import goes inside it, at the end of the array.
    d.active_layer_id = Some(f);
    let img = add(&mut d, "IMG", BLUE);
    assert_eq!(d.layer(img).unwrap().parent_id, Some(f));
    assert_eq!(d.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![f, h1, h2, img]);
    assert_eq!(d.render_ids(), vec![h1, img, h2], "the import draws inside the folder, under H2");
    assert_eq!(top_pixel(&d), GREEN, "H2 sits above the whole folder");
}

#[test]
fn hidden_folders_and_nesting_still_drop_out_of_the_render_order() {
    let mut d = doc();
    let a = add(&mut d, "A", RED);
    d.active_layer_id = Some(a);
    let f = hierarchy::group_layers(&mut d, &[a]).unwrap();
    d.layer_mut(f).unwrap().visible = false;
    assert_eq!(d.render_ids(), Vec::<uuid::Uuid>::new(), "a hidden folder hides its contents");
    d.layer_mut(f).unwrap().visible = true;
    assert_eq!(d.render_ids(), vec![a]);
    assert!(!d.render_ids().contains(&f), "folders never draw themselves");
}

#[test]
fn selecting_a_layer_records_no_history_and_keeps_redo() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, true).unwrap();
    let first = e.state(doc).unwrap().active_layer_id.unwrap();
    e.execute(doc, Command::AddBlankLayer).unwrap();
    let second = e.state(doc).unwrap().active_layer_id.unwrap();
    e.undo(doc).unwrap();
    let s = e.state(doc).unwrap();
    assert_eq!((s.layers.len(), s.can_redo), (1, true));
    e.execute(doc, Command::SetActiveLayer { id: Some(first) }).unwrap();
    let s = e.state(doc).unwrap();
    assert_eq!(s.active_layer_id, Some(first), "the selection still moves");
    assert!(s.can_redo, "selecting must not discard the redo stack");
    assert!(!s.can_undo, "selecting must not add an undo entry");
    assert!(!s.is_modified, "selecting must not mark the document modified");
    e.redo(doc).unwrap();
    let s = e.state(doc).unwrap();
    assert_eq!((s.layers.len(), s.active_layer_id), (2, Some(second)), "redo restores the layer and its selection");
}

#[test]
fn revert_drops_the_last_entry_without_offering_a_redo() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, true).unwrap();
    let first = e.state(doc).unwrap().active_layer_id.unwrap();
    e.execute(doc, Command::DuplicateLayer { id: first }).unwrap();
    assert_eq!(e.state(doc).unwrap().layers.len(), 2);
    e.revert(doc).unwrap();
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 1, "the copy is gone");
    assert!(!s.can_redo, "a cancelled gesture leaves nothing to redo");
    assert!(!s.can_undo, "and nothing to undo");
}

#[test]
fn replacing_a_layers_pixels_drops_its_shape_record() {
    let mut d = doc();
    let id = add(&mut d, "Shape", RED);
    d.layer_mut(id).unwrap().extra.shape = Some(serde_json::json!({ "kind": "roundedRect", "radius": 8 }));
    let corners = [Point { x: 0.0, y: 0.0 }, Point { x: 8.0, y: 1.0 }, Point { x: 7.0, y: 9.0 }, Point { x: 0.0, y: 8.0 }];
    let t = d.layer(id).unwrap().transform;
    ops::distort::distort_layer(&mut d, id, &t, &corners).unwrap();
    assert_eq!(d.layer(id).unwrap().extra.shape, None, "warped pixels are no longer the shape's pixels");
}

#[test]
fn undo_depth_counts_the_entries_a_gesture_can_take_back() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, true).unwrap();
    let first = e.state(doc).unwrap().active_layer_id.unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
    e.execute(doc, Command::DuplicateLayer { id: first }).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 1);
    // A selection records nothing, so the depth does not move.
    e.execute(doc, Command::SetActiveLayer { id: Some(first) }).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 1);
    // An interleaved edit does move it: this is what tells a cancelled gesture that the entry
    // on top is no longer its own.
    e.execute(doc, Command::SetLayersOpacity { ids: vec![first], opacity: 0.5 }).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 2);
    e.revert(doc).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 1);
    e.undo(doc).unwrap();
    assert_eq!(e.state(doc).unwrap().undo_depth, 0);
}

#[test]
fn a_clipping_source_prefilters_like_any_other_draw() {
    // A 64x64 source that is opaque only on every fourth pixel in each axis, so one sixteenth
    // of it is covered. Averaged down 4x that is a uniform ~16/255; sampled at full resolution
    // the output pixel centres all land two pixels away from the opaque columns and read zero.
    let mut data = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64u32 { for x in 0..64u32 {
        let v = if x % 4 == 0 && y % 4 == 0 { 255u8 } else { 0 };
        data.extend_from_slice(&[v, v, v, v]);
    }}
    let mut d = Document::new(64, 64);
    let base = layers::import_raster(&mut d, Raster::from_premultiplied(64, 64, data), "base", Some(Point { x: 32.0, y: 32.0 })).unwrap();
    let top = layers::import_raster(&mut d, square([0, 0, 255]), "top", Some(Point { x: 32.0, y: 32.0 })).unwrap();
    // Stretch the clipped layer over the whole canvas so every output pixel asks the source.
    let t = d.layer(top).unwrap().transform;
    ops::transform::set_transform(&mut d, top, LayerTransform { origin: Point { x: 0.0, y: 0.0 }, size: Size { width: 64.0, height: 64.0 }, ..t }).unwrap();
    hierarchy::link_mask(&mut d, base, top).unwrap();
    // A visible base and its clipped layer become a Stack node, which shares the base's alpha
    // directly and never consults the source. Hiding the base is what routes the draw through
    // `source_coverage_at`, the path the GL renderer's `applyClip` mirrors; coverage uses the
    // source's alpha regardless of its visibility, on both platforms.
    d.layer_mut(base).unwrap().visible = false;
    // One output pixel per four source pixels in each axis: one halving.
    let out = compositor::composite(&d, Rect { x: 0.0, y: 0.0, width: 64.0, height: 64.0 }, 16, 16);
    let alphas: Vec<u8> = (0..16).map(|x| out.pixel(x, 8)[3]).collect();
    // Prefiltered, the coverage carries the block average; unreduced it is zero everywhere.
    assert!(alphas.iter().all(|a| (8..=32).contains(a)), "clip coverage is not prefiltered: {alphas:?}");
}

#[test]
fn prefilter_level_matches_the_halving_rule() {
    // At most two source pixels per output pixel after the halvings.
    assert_eq!(compositor::prefilter_level(1024, 1024, 1.0), 0);
    assert_eq!(compositor::prefilter_level(1024, 1024, 2.0), 0);
    assert_eq!(compositor::prefilter_level(1024, 1024, 4.0), 1);
    assert_eq!(compositor::prefilter_level(1024, 1024, 16.0), 3);
    // A raster that has run out of pixels stops halving.
    assert_eq!(compositor::prefilter_level(2, 2, 1024.0), 1);
    assert_eq!(compositor::prefilter_level(1, 1, 1024.0), 0);
    // Smooth prefilters like High quality, as macOS does; Nearest and distortions never do.
    assert!(compositor::prefilters(Sampling::Smooth, false));
    assert!(compositor::prefilters(Sampling::High, false));
    assert!(!compositor::prefilters(Sampling::Nearest, false));
    assert!(!compositor::prefilters(Sampling::High, true));
}
