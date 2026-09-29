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
    // The state shows the reduced copy; the stored layer (which decides the job worker) is unchanged.
    assert_eq!(e.stored_pixels(id, layer).unwrap(), 1000 * 800);
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
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels { .. }));
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
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels { .. }));
    // A small selection, but the layer does not cover the canvas and grows.
    let (mut e, id, layer) = document((1200, 1100), (400, 300), p(100.0, 100.0));
    select(&mut e, id, 150.0, 150.0, 50.0, 40.0);
    preview(&mut e, id, layer, false, &g, true);
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels { .. }));
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
    assert!(matches!(e.preview(doc_id).unwrap().target, PreviewTarget::Mask { .. }));
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

#[test]
fn t9_6_a_pixel_gradient_preview_carries_a_covering_mask_onto_the_grown_grid_instead_of_stretching_it() {
    // Task 15 fix round 1, T9-6: a PIXEL gradient (mask targeted false) on a layer smaller than the
    // canvas, under a covering (un-placed), non-uniform mask, must not stretch that small mask over
    // the whole grown preview box -- the mask stays where the layer's own pixels land in the grown
    // grid, exactly as the commit's `paint_layer`/`followed` leave it (mask.rs, PreviewTarget::Pixels).
    // A 100 x 100 canvas; a 20 x 20 opaque layer at (40, 40): well inside it, so its grid grows to the
    // whole canvas for the preview. A covering 20 x 20 mask, left half black (hides), right half white
    // (reveals).
    let mut doc = Document::new(100, 100);
    let data: Vec<u8> = [255u8, 0, 0, 255].repeat(400);
    let mut layer = Layer::with_pixels("L", Raster::from_premultiplied(20, 20, data), p(40.0, 40.0));
    let id = layer.id;
    let mask_data: Vec<u8> = (0..400u32).map(|i| if i % 20 < 10 { 0u8 } else { 255u8 }).collect();
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(20, 20, mask_data), enabled: true, placement: None, linked: None });
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let doc_id = e.insert_document(doc);
    // An opaque, constant red: alpha alone tells whether the mask hid or revealed a point.
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(1.0, 0.0), from: [1.0, 0.0, 0.0, 1.0], to: [1.0, 0.0, 0.0, 1.0], opacity: 1.0 };
    for dragging in [true, false] {
        preview(&mut e, doc_id, id, false, &g, dragging);
        // (12, 50) is well outside the mask's own 40..60 footprint: the stretched-over bug (a
        // document point mapped through the GROWN transform onto the mask's small, un-grown pixel
        // count) would land it in the mask's black half and hide it; carried correctly, the mask's
        // background (white, reveal) applies there instead.
        assert_eq!(shown(&e, doc_id, 12, 50)[3], 255, "dragging={dragging}: outside the mask's footprint, its background reveals");
        assert_eq!(shown(&e, doc_id, 45, 50)[3], 0, "dragging={dragging}: inside the mask's own black half");
        assert_eq!(shown(&e, doc_id, 55, 50)[3], 255, "dragging={dragging}: inside the mask's own white half");
    }
    e.set_preview(doc_id, None).unwrap();
    run(&mut e, doc_id, Command::Gradient { id, mask: false, gradient: g });
    assert_eq!(shown(&e, doc_id, 12, 50)[3], 255, "the preview showed exactly what Return then applies");
    assert_eq!(shown(&e, doc_id, 45, 50)[3], 0, "the preview showed exactly what Return then applies");
    assert_eq!(shown(&e, doc_id, 55, 50)[3], 255, "the preview showed exactly what Return then applies");
}

#[test]
fn t9_6_a_reduced_pixel_gradient_preview_matches_the_committed_mask_sampled_at_its_own_grid() {
    // Task 15 fix round 3, item 2: the test above only covers level 0 (the preview at full resolution).
    // A 3000 x 2000 canvas forces `level_for` to reduce a small layer's grown grid: 2 halvings while
    // dragging (limit 1024: 3000 -> 1500 -> 750) and 1 once settled (limit 2048: 3000 -> 1500). A 100 x
    // 100 opaque layer well inside it, with a covering 100 x 100 mask, left half black (hides), right
    // half white (reveals).
    let (cw, ch) = (3000u32, 2000u32);
    let (lw, lh) = (100u32, 100u32);
    let origin = p(1400.0, 950.0);
    let data: Vec<u8> = [255u8, 0, 0, 255].repeat((lw * lh) as usize);
    let mut layer = Layer::with_pixels("L", Raster::from_premultiplied(lw, lh, data), origin);
    let id = layer.id;
    let original_transform = layer.transform;
    let mask_data: Vec<u8> = (0..lw * lh).map(|i| if i % lw < lw / 2 { 0u8 } else { 255u8 }).collect();
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(lw, lh, mask_data), enabled: true, placement: None, linked: None });
    let mut doc = Document::new(cw, ch);
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let doc_id = e.insert_document(doc);
    // An opaque, constant red: alpha alone tells whether the mask hid or revealed a point.
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(1.0, 0.0), from: [1.0, 0.0, 0.0, 1.0], to: [1.0, 0.0, 0.0, 1.0], opacity: 1.0 };

    // Three named points in the ORIGINAL layer's own local pixel space (before any growth): well
    // outside its own box (to its left), and well either side of the black/white edge at local column
    // lw / 2 (a quarter of the layer's width in from each edge, so the two stay in different reduced
    // pixels even at the coarsest level this test reaches, level 2 -- one pixel adjacent to the edge
    // itself would collapse into the same reduced pixel as its neighbour there), mapped once through
    // the original (unreduced) transform to document space.
    let to_document = original_transform.pixel_to_document(lw, lh);
    let outside_doc = to_document.apply(p(-20.0, lh as f64 / 2.0 + 0.5));
    let black_side_doc = to_document.apply(p(lw as f64 / 4.0, lh as f64 / 2.0 + 0.5));
    let white_side_doc = to_document.apply(p(lw as f64 * 3.0 / 4.0, lh as f64 / 2.0 + 0.5));

    for dragging in [true, false] {
        preview(&mut e, doc_id, id, false, &g, dragging);
        let state = e.state(doc_id).unwrap().layers[0].clone();
        assert!(state.pixels_width < cw, "dragging={dragging}: expected a genuinely reduced preview, got {}", state.pixels_width);
        let preview_mask = e.mask_pixels(doc_id, id).unwrap().unwrap();
        let (rw, rh) = (preview_mask.width, preview_mask.height);
        let preview_to_doc = state.transform.pixel_to_document(rw, rh);
        let doc_to_preview = preview_to_doc.invert().unwrap();
        let preview_at = |doc_pt: Point| -> u8 {
            let pp = doc_to_preview.apply(doc_pt);
            let (px, py) = (pp.x.floor().clamp(0.0, (rw - 1) as f64) as u32, pp.y.floor().clamp(0.0, (rh - 1) as f64) as u32);
            preview_mask.bytes()[(py * rw + px) as usize]
        };
        assert_eq!(preview_at(outside_doc), 255, "dragging={dragging}: outside the old layer's bounds, the background reveals");
        assert_eq!(preview_at(black_side_doc), 0, "dragging={dragging}: just left of the edge, the mask's black half");
        assert_eq!(preview_at(white_side_doc), 255, "dragging={dragging}: just right of the edge, the mask's white half");

        // The whole reduced grid must match the committed mask sampled at each of its own pixel
        // centres, derived from a real commit (never a pasted run): commit the very same gradient, then
        // invert through the committed layer's own transform to find what each reduced pixel's centre
        // lands on there.
        e.set_preview(doc_id, None).unwrap();
        run(&mut e, doc_id, Command::Gradient { id, mask: false, gradient: g.clone() });
        let committed = e.state(doc_id).unwrap().layers[0].clone();
        let committed_mask = e.mask_pixels(doc_id, id).unwrap().unwrap();
        let (cmw, cmh) = (committed_mask.width, committed_mask.height);
        let doc_to_committed = committed.transform.pixel_to_document(cmw, cmh).invert().unwrap();
        let mut mismatches: Vec<(u32, u32, u8, u8)> = Vec::new();
        for y in 0..rh {
            for x in 0..rw {
                let doc_pt = preview_to_doc.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
                let cp = doc_to_committed.apply(doc_pt);
                let (mx, my) = (cp.x.floor().clamp(0.0, (cmw - 1) as f64) as u32, cp.y.floor().clamp(0.0, (cmh - 1) as f64) as u32);
                let expected = committed_mask.bytes()[(my * cmw + mx) as usize];
                let actual = preview_mask.bytes()[(y * rw + x) as usize];
                if actual != expected { mismatches.push((x, y, actual, expected)); }
            }
        }
        // `followed` maps a reduced pixel's INDEX to the mask nearest by integer scaling; this test
        // maps its CONTINUOUS centre through two affine round trips instead (document space, then the
        // committed layer's own transform) -- an independent path, not the same formula, but the two
        // conventions can disagree by one reduced pixel exactly astride the mask's hard edge under a
        // coarse reduction (level 2 here: measured 25 of 375,750 pixels, one whole column the height of
        // the layer's own reduced footprint, and only at level 2 -- level 1 below matches exactly, 0 of
        // 1,500,000). A real bug (the wrong grid offset or the wrong "old" dimensions) would show as a
        // much larger, structural mismatch, not a hairline seam at the transition alone.
        assert!(mismatches.len() <= 30, "dragging={dragging}: {} mismatches of {} pixels, more than the hairline seam this reduction level can explain: {:?}", mismatches.len(), rw * rh, &mismatches[..mismatches.len().min(20)]);
        e.undo(doc_id).unwrap();
    }
}
