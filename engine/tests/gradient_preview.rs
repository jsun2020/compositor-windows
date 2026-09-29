//! The gradient's preview (Phase 4b-1): a reduced copy of the grown layer while the line is dragged,
//! a full-size patch inside a small selection on a layer over the canvas, a reduced mask on a mask;
//! the patch reaches the GPU as its rectangle (`pixels_delta`, `layer_region`).
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
fn preview(e: &mut Engine, id: Uuid, layer: Uuid, mask: bool, gradient: &GradientSpec, dragging: bool) {
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask, gradient: gradient.clone(), dragging })).unwrap();
}
/// Unequal opaque pixels.
fn pattern(width: u32, height: u32) -> Raster {
    let data = (0..width * height).flat_map(|i| { let v = i.wrapping_mul(2654435761); [(v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, 255] }).collect();
    Raster::from_premultiplied(width, height, data)
}
/// A document of `canvas` holding one `pattern` layer of `size` at `at`, active, made in its engine.
fn document(canvas: (u32, u32), size: (u32, u32), at: Point) -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(canvas.0, canvas.1);
    let layer = Layer::with_pixels("Pattern", pattern(size.0, size.1), at);
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let handle = e.insert_document(doc);
    (e, handle, id)
}
fn shown(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0) }

#[test]
fn a_dragged_gradient_previews_from_a_reduced_copy_of_the_layer_grown_to_the_canvas() {
    // A 1000 x 800 layer at (500, 400) on 3000 x 2000: the grid grows to the canvas, 3000 x 2000.
    let (mut e, id, layer) = document((3000, 2000), (1000, 800), p(500.0, 400.0));
    // Black at the left fading to nothing at x 1500: the layer shows through on the right.
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 1000.0), end: p(1500.0, 1000.0), from: [0.0, 0.0, 0.0, 1.0], to: [0.0, 0.0, 0.0, 0.0], opacity: 1.0 };
    preview(&mut e, id, layer, false, &g, true);
    let state = e.state(id).unwrap().layers[0].clone();
    // Halved until the longer side is at most 1024: 3000 -> 1500 -> 750.
    assert_eq!((state.pixels_width, state.pixels_height), (750, 500));
    assert!(state.transform.origin.x <= 0.0 && state.transform.origin.y <= 0.0 && state.transform.origin.x + state.transform.size.width >= 3000.0, "{:?}", state.transform);
    // Left of the layer, where there were no pixels: the gradient alone (a quarter of the way: alpha 191).
    let left = shown(&e, id, 375, 1000);
    assert!(left[3].abs_diff(191) <= 2, "{left:?}");
    // The layer's own (halved) pixels land where the layer is: just left of its edge at x 500 only the
    // gradient (495 / 1500 of the way: alpha 171), just right of it the opaque layer under it.
    assert!(shown(&e, id, 495, 1000)[3].abs_diff(171) <= 2);
    assert_eq!(shown(&e, id, 505, 1000)[3], 255);
    // Past the layer's right edge and the gradient's end: nothing.
    assert_eq!(shown(&e, id, 1510, 1000)[3], 0);
    // Past the gradient's end, the layer's own pixels (halved twice), as the stored layer shows them.
    let right = shown(&e, id, 1400, 1000);
    e.set_preview(id, None).unwrap();
    let stored = shown(&e, id, 1400, 1000);
    assert_eq!(right[3], 255);
    // A reduced pixel averages its 4 x 4 block, which holds this one: close only where neighbours
    // agree, so compare the alpha, which is 255 in every pixel of the layer.
    assert_eq!(stored[3], 255);
    // Settled: at most 2048 across (GRADIENT_SETTLED_LIMIT): 3000 -> 1500.
    preview(&mut e, id, layer, false, &g, false);
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.pixels_width, state.pixels_height), (1500, 1000));
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
    assert_eq!(e.state(id).unwrap().undo_depth, 0, "a preview records nothing");
}

#[test]
fn a_gradient_inside_a_small_selection_previews_as_a_patch_the_commit_equals() {
    // A 600 x 400 layer over the 600 x 400 canvas; a 100 x 80 selection off its centre.
    let (mut e, id, layer) = document((600, 400), (600, 400), p(0.0, 0.0));
    select(&mut e, id, 123.0, 77.0, 100.0, 80.0);
    let stored_revision = e.state(id).unwrap().layers[0].pixels_revision;
    let g = GradientSpec { shape: GradientShape::Radial, start: p(150.0, 100.0), end: p(210.0, 140.0), from: [1.0, 0.2, 0.0, 1.0], to: [0.0, 0.0, 1.0, 0.3], opacity: 0.9 };
    preview(&mut e, id, layer, false, &g, true);
    let PreviewTarget::Patch(rect) = e.preview(id).unwrap().target.clone() else { panic!("a patch") };
    // The selection's box, a pixel for its clip and one for sampling on every side.
    assert_eq!(rect, PixelRect { x: 121, y: 75, width: 104, height: 84 });
    let preview_revision = e.state(id).unwrap().layers[0].pixels_revision;
    assert_ne!(preview_revision, stored_revision);
    assert_eq!(e.pixels_delta(id, layer, stored_revision).unwrap(), Some(rect), "the GPU uploads the patch alone");
    let patch = e.preview(id).unwrap().raster.bytes().to_vec();
    assert_eq!(e.layer_region(id, layer, 0, rect).unwrap(), patch);
    // Halved twice, the region is the patched layer's own halving there.
    let mut patched = e.preview(id).unwrap().patched(e.document(id).unwrap().layers[0].pixels.as_ref().unwrap());
    patched = patched.halved().halved();
    let quarter = PixelRect { x: rect.x / 4, y: rect.y / 4, width: (rect.x + rect.width).div_ceil(4) - rect.x / 4, height: (rect.y + rect.height).div_ceil(4) - rect.y / 4 };
    assert_eq!(e.layer_region(id, layer, 2, quarter).unwrap(), patched.cropped(quarter.x, quarter.y, quarter.width, quarter.height).into_bytes());
    // The CPU draws the patch over the stored pixels.
    let through = shown(&e, id, 150, 100);
    // Taken back: the rectangle again, to the stored pixels.
    e.set_preview(id, None).unwrap();
    assert_eq!(e.pixels_delta(id, layer, preview_revision).unwrap(), Some(rect));
    let stored = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    assert_eq!(e.layer_region(id, layer, 0, rect).unwrap(), stored.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes());
    // The commit paints exactly the patch.
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: g });
    let committed = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    assert_eq!(committed.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes(), patch);
    assert_eq!(shown(&e, id, 150, 100), through);
}

#[test]
fn a_large_selection_or_a_layer_that_must_grow_previews_whole() {
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(300.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    // On a layer over the canvas, 720 x 720 selected is a 724 x 724 patch (two pixels a side, as
    // above), within PATCH_LIMIT (1 << 19 = 524288 >= 524176); 730 x 720 is 734 x 724 = 531416, past it.
    let (mut e, id, layer) = document((1200, 1100), (1200, 1100), p(0.0, 0.0));
    select(&mut e, id, 10.0, 10.0, 720.0, 720.0);
    preview(&mut e, id, layer, false, &g, true);
    let target = e.preview(id).map(|p| p.target.clone());
    assert!(matches!(target, Some(PreviewTarget::Patch(PixelRect { width: 724, height: 724, .. }))), "{}", match &target { Some(PreviewTarget::Patch(r)) => format!("{r:?}"), Some(_) => "reduced".into(), None => "none".into() });
    select(&mut e, id, 10.0, 10.0, 730.0, 720.0);
    preview(&mut e, id, layer, false, &g, true);
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
    // A small selection, but the layer does not cover the canvas and grows.
    let (mut e, id, layer) = document((1200, 1100), (400, 300), p(100.0, 100.0));
    select(&mut e, id, 150.0, 150.0, 50.0, 40.0);
    preview(&mut e, id, layer, false, &g, true);
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
}

#[test]
fn a_mask_gradient_previews_the_mask_reduced() {
    let (mut e, id, layer) = document((3000, 2000), (3000, 2000), p(0.0, 0.0));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    let before = e.state(id).unwrap().layers[0].clone();
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(3000.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, id, layer, true, &g, true);
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (750, 500), "the 1 x 1 mask shown on the layer's grid, reduced");
    assert_ne!(state.mask_revision, before.mask_revision);
    assert_eq!(state.pixels_revision, before.pixels_revision, "the pixels are not previewed");
    assert_eq!(e.mask_pixels(id, layer).unwrap().unwrap().width, 750);
    // Half way along: the mask half grey, the layer half shown.
    assert!(shown(&e, id, 1500, 1000)[3].abs_diff(128) <= 2);
    e.set_preview(id, None).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].mask_width, 1);
}

#[test]
fn a_gradient_the_commit_would_refuse_shows_nothing() {
    let (mut e, id, layer) = document((300, 200), (300, 200), p(0.0, 0.0));
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(0.0, 0.0), p(300.0, 0.0), p(300.0, 200.0), p(0.0, 200.0)], mode: SelectionMode::Subtract, antialiased: true });
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(100.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, id, layer, false, &g, true);
    assert!(e.preview(id).is_none(), "an empty selection: nothing");
    run(&mut e, id, Command::Deselect);
    let short = GradientSpec { end: p(0.2, 0.0), ..g };
    preview(&mut e, id, layer, false, &short, true);
    assert!(e.preview(id).is_none(), "a line under half a pixel: nothing");
}

#[test]
fn a_non_uniform_covering_mask_of_a_different_size_gathers_as_the_commit_does() {
    // Fix round 1, item 2: a covering mask genuinely stretched onto the layer's grid (50 x 40 onto
    // 200 x 150, neither axis a multiple of the other -- a swapped width/height or the wrong row
    // stride in the gather would show), on a canvas small enough that the preview is not reduced
    // (level 0): the previewed mask must equal what a commit of the very same gradient leaves behind,
    // so the expected bytes come from the commit, never a pasted run.
    let mut doc = Document::new(200, 150);
    let mut layer = Layer::with_pixels("Pattern", pattern(200, 150), p(0.0, 0.0));
    let (mw, mh) = (50u32, 40u32);
    let mask_data: Vec<u8> = (0..mw * mh).map(|i| (i * 7 % 256) as u8).collect();
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(mw, mh, mask_data), enabled: true, placement: None, linked: None });
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let doc_id = e.insert_document(doc);

    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(200.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, doc_id, id, true, &g, true);
    assert!(matches!(e.preview(doc_id).unwrap().target, PreviewTarget::Mask(_)));
    let state = e.state(doc_id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (200, 150), "the canvas is small: not reduced");
    let previewed = e.mask_pixels(doc_id, id).unwrap().unwrap().bytes().to_vec();
    e.set_preview(doc_id, None).unwrap();
    run(&mut e, doc_id, Command::Gradient { id, mask: true, gradient: g });
    let committed = e.document(doc_id).unwrap().layers[0].mask.as_ref().unwrap().pixels.bytes().to_vec();
    assert_eq!(previewed, committed);
}

#[test]
fn a_mask_gradient_preview_refuses_what_growing_the_covering_mask_would_refuse() {
    // Fix round 1, item 3a: the pixels-side sibling of this is
    // `a_fill_that_would_grow_the_covering_mask_past_the_mask_budget_is_refused` (raster_edits.rs).
    let mut e = Engine::new();
    let id = e.new_document(100, 40, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(100, 40, [0, 0, 200, 255].repeat(4000)), p(0.0, 0.0));
    let small_mask: Vec<u8> = vec![255u8; 200]; // A 20 x 10 covering mask.
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(20, 10, small_mask), enabled: true, placement: None, linked: None });
    let lid = layer.id;
    // Growing this covering mask onto the layer's 100 x 40 grid needs 4,000 pixels of room.
    let needed: u64 = 100 * 40;
    // Another layer's mask holds 99,999,000 of the 100,000,000-pixel mask budget, leaving 1,000 --
    // less than the 4,000 needed above.
    let (big_w, big_h) = (99_999u32, 1_000u32);
    let big_mask_pixels = big_w as u64 * big_h as u64;
    let remaining = MAX_PIXELS - big_mask_pixels;
    assert!(needed > remaining, "the fixture must actually starve the budget");
    let mut big = Layer::blank("Big Folder", doc.size());
    big.is_group = true;
    big.mask = Some(Mask { pixels: GrayRaster::from_bytes(big_w, big_h, vec![255u8; big_mask_pixels as usize]), enabled: true, placement: None, linked: None });
    doc.active_layer_id = Some(lid);
    doc.layers = vec![big, layer];
    let mut e = Engine::new();
    let doc_id = e.insert_document(doc);
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(100.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, doc_id, lid, true, &g, true);
    assert!(e.preview(doc_id).is_none(), "growing the mask past the budget: nothing shows");
}
