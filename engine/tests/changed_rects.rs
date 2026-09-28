//! Changed rectangles (Phase 4b-1): the revision lineage the GPU asks for partial uploads, the halving
//! it redoes only where pixels changed, and the halving itself.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Deterministic, unequal pixels: no two neighbours alike, alpha varied.
fn pattern(width: u32, height: u32, seed: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let v = (x.wrapping_mul(73) ^ y.wrapping_mul(151) ^ seed.wrapping_mul(29)).wrapping_mul(2654435761);
        let a = (v >> 24) as u8 | 1;
        data.extend_from_slice(&[((v >> 8) as u8).min(a), ((v >> 16) as u8).min(a), (v as u8).min(a), a]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// The 2 x 2 box average written out plainly: every output pixel averages the source pixels of its
/// block that exist, rounding half up (the definition `Raster::halved` must keep).
fn reference_half(r: &Raster) -> Vec<u8> {
    let (w, h) = ((r.width / 2).max(1), (r.height / 2).max(1));
    let mut out = Vec::new();
    for y in 0..h { for x in 0..w {
        let mut sum = [0u32; 4]; let mut n = 0u32;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (sx, sy) = (2 * x + dx, 2 * y + dy);
            if sx < r.width && sy < r.height { let px = r.pixel(sx, sy); for c in 0..4 { sum[c] += px[c] as u32; } n += 1; }
        }
        for c in 0..4 { out.push(((sum[c] + n / 2) / n) as u8); }
    }}
    out
}

#[test]
fn halving_averages_each_block_at_every_edge_and_size() {
    for (w, h) in [(7, 5), (1, 9), (9, 1), (1, 1), (2, 3), (33, 17), (64, 48)] {
        let r = pattern(w, h, w * 31 + h);
        let half = r.halved();
        assert_eq!((half.width, half.height), ((w / 2).max(1), (h / 2).max(1)));
        assert_eq!(half.bytes(), reference_half(&r).as_slice(), "{w} x {h}");
    }
}

/// `parent` with `rect` painted over by another pattern.
fn painted(parent: &Raster, rect: PixelRect) -> Raster {
    let other = pattern(parent.width, parent.height, 999);
    let mut data = parent.bytes().to_vec();
    for y in rect.y..rect.y + rect.height { for x in rect.x..rect.x + rect.width {
        let i = ((y * parent.width + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&other.bytes()[i..i + 4]);
    }}
    Raster::from_premultiplied(parent.width, parent.height, data)
}

#[test]
fn seeded_halvings_equal_halving_from_scratch() {
    // An odd-sized parent halved twice; rectangles at odd offsets, one reaching the far edges.
    for rect in [PixelRect { x: 5, y: 7, width: 9, height: 4 }, PixelRect { x: 30, y: 20, width: 7, height: 9 }, PixelRect { x: 0, y: 0, width: 1, height: 1 }] {
        let parent = pattern(37, 29, 3);
        let _ = parent.halved().halved();
        let child = painted(&parent, rect);
        child.seed_halvings(&parent, rect);
        // Seeded, not made on demand: both levels are there before anyone asks.
        let seeded = child.memoized_half().expect("level 1 seeded");
        assert!(seeded.memoized_half().is_some(), "level 2 seeded");
        let fresh = Raster::from_premultiplied(child.width, child.height, child.bytes().to_vec());
        assert_eq!(seeded.bytes(), fresh.halved().bytes(), "{rect:?}: level 1");
        assert_eq!(seeded.halved().bytes(), fresh.halved().halved().bytes(), "{rect:?}: level 2");
    }
}

/// A 120 x 80 document with one opaque layer of `pattern` pixels, active.
fn document() -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(120, 80, false).unwrap();
    let png = encode_png(&pattern(120, 80, 7), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png, "Pattern", Some(p(60.0, 40.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    (e, id, layer)
}
fn revision(e: &Engine, id: Uuid) -> u64 { e.state(id).unwrap().layers[0].pixels_revision }
fn pixels(e: &Engine, id: Uuid) -> Raster { e.document(id).unwrap().layers[0].pixels.clone().unwrap() }
fn select(e: &mut Engine, id: Uuid, x0: f64, y0: f64, x1: f64, y1: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], mode: SelectionMode::Replace, antialiased: false });
}

#[test]
fn a_clear_inside_a_selection_reports_where_it_changed_and_undo_and_redo_follow() {
    let (mut e, id, layer) = document();
    select(&mut e, id, 30.0, 20.0, 50.0, 44.0);
    let (r0, before) = (revision(&e, id), pixels(&e, id));
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    let (r1, after) = (revision(&e, id), pixels(&e, id));
    // The clip's region is the selection's bounds grown by a pixel (SelectionClip::new); the reported
    // rectangle adds a pixel more for bilinear sampling: 30 - 2 = 28 to 50 + 2 = 52, 20 - 2 to 44 + 2.
    let expected = PixelRect { x: 28, y: 18, width: 24, height: 28 };
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected));
    // Nothing outside it changed; inside, the selected pixels did.
    for y in 0..80 { for x in 0..120 {
        let inside = x >= 28 && x < 52 && y >= 18 && y < 46;
        if !inside { assert_eq!(before.pixel(x, y), after.pixel(x, y), "({x}, {y})"); }
    }}
    assert_eq!(after.pixel(40, 30), [0, 0, 0, 0]);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), Some(PixelRect::default()), "nothing since the current revision");
    e.undo(id).unwrap();
    assert_eq!(revision(&e, id), r0);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), Some(expected), "undo changes the same rectangle back");
    e.redo(id).unwrap();
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected));
    // A second clear elsewhere: from the first revision the two rectangles together.
    select(&mut e, id, 90.0, 60.0, 100.0, 70.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected.union(&PixelRect { x: 88, y: 58, width: 14, height: 14 })));
}

#[test]
fn an_edit_without_a_rectangle_or_that_changes_the_grid_is_taken_whole() {
    let (mut e, id, layer) = document();
    let r0 = revision(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "x".into() });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(PixelRect::default()), "a rename changes no pixels");
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), None, "no selection: the whole layer");
    let r1 = revision(&e, id);
    // A blur spreads past the layer and grows its grid: whole.
    run(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 3.0 } });
    assert!(pixels(&e, id).width > 120);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), None);
    assert_eq!(e.pixels_delta(id, layer, 123_456).unwrap(), None, "a revision never seen");
}

#[test]
fn a_mask_edit_inside_a_selection_reports_its_rectangle_on_the_mask() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
    select(&mut e, id, 60.0, 10.0, 70.0, 30.0);
    let m0 = e.state(id).unwrap().layers[0].mask_revision;
    // The first fill spreads the 1 x 1 mask over the layer's grid: whole.
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(e.mask_delta(id, layer, m0).unwrap(), None);
    let m1 = e.state(id).unwrap().layers[0].mask_revision;
    select(&mut e, id, 5.0, 50.0, 25.0, 60.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(e.mask_delta(id, layer, m1).unwrap(), Some(PixelRect { x: 3, y: 48, width: 24, height: 14 }));
}

#[test]
fn an_edit_inside_a_selection_hands_the_new_pixels_the_old_halvings() {
    let (mut e, id, layer) = document();
    let old = pixels(&e, id);
    let _ = old.halved().halved();
    select(&mut e, id, 33.0, 21.0, 47.0, 39.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    let new = pixels(&e, id);
    let seeded = new.memoized_half().expect("seeded at the edit");
    let fresh = Raster::from_premultiplied(120, 80, new.bytes().to_vec());
    assert_eq!(seeded.bytes(), fresh.halved().bytes());
    assert_eq!(seeded.halved().bytes(), fresh.halved().halved().bytes());
}

// -- Ruling I4: every rectangle above uses a layer at (0, 0) at 1:1, where a rectangle reported in
// document coordinates instead of the buffer's own grid would still happen to match. The two tests
// below use a layer offset and scaled, and a mask placed apart from its layer, and compute the
// expected rectangle in the test through the transform's own inverse (never pasted from a run), so a
// rectangle in the wrong coordinate space fails them.

/// The rectangle `SelectionClip::rect_on_grid` would report for a selection with document bounds
/// (x0, y0)-(x1, y1), on a `width` x `height` grid placed by `transform` on a `canvas_w` x
/// `canvas_h` canvas: the clip's region (the bounds grown by a pixel, cut to the canvas -
/// `SelectionClip::new`) mapped onto the grid through `transform`'s own inverse, grown by one more
/// pixel for sampling and cut to the grid. Computed independently of the engine here so a rectangle
/// reported in the wrong space (document pixels instead of the grid's own) fails the assertion that
/// uses it.
fn expected_region(x0: f64, y0: f64, x1: f64, y1: f64, transform: &LayerTransform, width: u32, height: u32, canvas_w: u32, canvas_h: u32) -> PixelRect {
    let (cx0, cy0) = ((x0 - 1.0).floor().max(0.0), (y0 - 1.0).floor().max(0.0));
    let (cx1, cy1) = ((x1 + 1.0).ceil().min(canvas_w as f64), (y1 + 1.0).ceil().min(canvas_h as f64));
    let inverse = transform.pixel_to_document(width, height).invert().expect("the transform inverts");
    let corners = [(cx0, cy0), (cx1, cy0), (cx1, cy1), (cx0, cy1)].map(|(x, y)| inverse.apply(Point { x, y }));
    let (lx, hx) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), c| (l.min(c.x), h.max(c.x)));
    let (ly, hy) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), c| (l.min(c.y), h.max(c.y)));
    let gx0 = (lx.floor() - 1.0).clamp(0.0, width as f64) as u32;
    let gy0 = (ly.floor() - 1.0).clamp(0.0, height as f64) as u32;
    let gx1 = (hx.ceil() + 1.0).clamp(0.0, width as f64) as u32;
    let gy1 = (hy.ceil() + 1.0).clamp(0.0, height as f64) as u32;
    PixelRect { x: gx0, y: gy0, width: gx1.saturating_sub(gx0), height: gy1.saturating_sub(gy0) }
}

#[test]
fn a_clear_on_a_layer_offset_and_scaled_reports_the_rectangle_on_its_own_grid() {
    let (mut e, id, layer) = document();
    // Move the 120 x 80 layer to a 1.5x scaled, offset placement: document (5, 4) to (95, 64).
    let moved = LayerTransform::axis_aligned(p(5.0, 4.0), Size { width: 180.0, height: 120.0 });
    run(&mut e, id, Command::SetLayerTransform { id: layer, transform: moved });
    select(&mut e, id, 20.0, 10.0, 60.0, 40.0);
    let (r0, before) = (revision(&e, id), pixels(&e, id));
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    let (r1, after) = (revision(&e, id), pixels(&e, id));
    let expected = expected_region(20.0, 10.0, 60.0, 40.0, &moved, 120, 80, 120, 80);
    // The same formula at the layer's original (0, 0), 1:1 placement gives a different rectangle: a
    // rectangle reported in document coordinates instead of the layer's own grid is not this one.
    let unmoved = LayerTransform::axis_aligned(p(0.0, 0.0), Size { width: 120.0, height: 80.0 });
    assert_ne!(expected, expected_region(20.0, 10.0, 60.0, 40.0, &unmoved, 120, 80, 120, 80));
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected));
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), Some(PixelRect::default()), "nothing since the current revision");
    for y in 0..80 { for x in 0..120 {
        let inside = x >= expected.x && x < expected.x + expected.width && y >= expected.y && y < expected.y + expected.height;
        if !inside { assert_eq!(before.pixel(x, y), after.pixel(x, y), "({x}, {y})"); }
    }}
    // Something inside the rectangle changed (else the union check above would pass on an all-empty
    // rectangle too).
    assert_ne!(before.bytes(), after.bytes());
}

#[test]
fn a_mask_fill_on_a_mask_placed_apart_from_its_layer_reports_the_rectangle_on_the_masks_own_grid() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
    // Spread the mask to the layer's 120 x 80 grid (a selection anywhere does it; the corner avoids
    // overlapping the region used below).
    select(&mut e, id, 0.0, 0.0, 2.0, 2.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    // Move the mask 15, 10 apart from the layer (same size, so this isolates translation from scale,
    // which the previous test already covers).
    let apart = LayerTransform::axis_aligned(p(15.0, 10.0), Size { width: 120.0, height: 80.0 });
    run(&mut e, id, Command::SetMaskPlacement { id: layer, placement: apart });
    select(&mut e, id, 50.0, 40.0, 70.0, 60.0);
    let m0 = e.state(id).unwrap().layers[0].mask_revision;
    let before = e.document(id).unwrap().layers[0].mask.as_ref().unwrap().pixels.clone();
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    let after = e.document(id).unwrap().layers[0].mask.as_ref().unwrap().pixels.clone();
    let expected = expected_region(50.0, 40.0, 70.0, 60.0, &apart, 120, 80, 120, 80);
    // The layer's own grid (no offset applied, since `document()`'s layer sits at (0, 0) 1:1) would
    // report a different rectangle: not a coincidence.
    let on_layer = LayerTransform::axis_aligned(p(0.0, 0.0), Size { width: 120.0, height: 80.0 });
    assert_ne!(expected, expected_region(50.0, 40.0, 70.0, 60.0, &on_layer, 120, 80, 120, 80));
    assert_eq!(e.mask_delta(id, layer, m0).unwrap(), Some(expected));
    // Nothing outside the reported rectangle changed, on the MASK's own grid (fix round 1, ruling I4
    // evidence): using the layer's grid here instead of the mask's placement would misplace this
    // check the same way it would misplace the reported rectangle.
    for y in 0..80 { for x in 0..120 {
        let inside = x >= expected.x && x < expected.x + expected.width && y >= expected.y && y < expected.y + expected.height;
        if !inside { assert_eq!(before.bytes()[(y * 120 + x) as usize], after.bytes()[(y * 120 + x) as usize], "({x}, {y})"); }
    }}
}

/// Bug (Task 3 fix round 1, #1): `apply_filter_with` grows a layer's grid to give a spreading blur
/// (Gaussian / Motion) room, blends it back through the selection on that grown grid, then trims it
/// to the alpha bounds and moves the transform (`ops::adjust::grown` / `trimmed`) -- so the result
/// can come back the SAME width and height it started at, but on a SHIFTED grid. A layer whose left
/// `k` columns are transparent and whose content touches its top and bottom rows, blurred inside a
/// selection that crosses only its right edge and spreads past it, is exactly such a case: `k = 6`
/// here empirically moves the origin by 6 while leaving both width and height unchanged (confirmed
/// below), which "same size" alone cannot distinguish from an untouched grid.
#[test]
fn a_blur_inside_a_selection_that_shifts_the_grid_at_the_same_size_is_taken_whole() {
    let mut e = Engine::new();
    let id = e.new_document(100, 40, false).unwrap();
    let (w, h, k) = (40u32, 20u32, 6u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in k..w {
        let i = ((y * w + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&[200, 150, 50, 255]);
    }}
    let raster = Raster::from_premultiplied(w, h, data);
    let png = encode_png(&raster, DEFAULT_RESOLUTION).unwrap();
    // Origin (10, 5), size (40, 20): centre (30, 15). Every opaque column spans the full height, so
    // the content touches both the top and the bottom row.
    e.import_image(Some(id), &png, "L", Some(p(30.0, 15.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    let old_transform = e.state(id).unwrap().layers[0].transform;
    // The selection matches the layer's own vertical extent exactly, so the grown grid's top and
    // bottom padding stays unselected (and so exactly transparent, pinning the height); it starts
    // well inside the opaque content (document x 34, well past the transparent columns which end at
    // document x 16) and crosses only the layer's right edge (document x 50), spreading past it.
    select(&mut e, id, 34.0, 5.0, 90.0, 25.0);
    let r0 = revision(&e, id);
    let old = pixels(&e, id);
    let _ = old.halved(); // Seeded on the OLD grid, before the shift.
    run(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 2.0 } });
    let new_transform = e.state(id).unwrap().layers[0].transform;
    let new = pixels(&e, id);
    // The scenario this test exists to set up: same size, moved grid.
    assert_eq!((new.width, new.height), (old.width, old.height), "same size is the whole point of this test");
    assert_eq!(new_transform.size, old_transform.size);
    assert_ne!(new_transform.origin, old_transform.origin, "and the grid moved");
    // Whole, not a rectangle: the grid changed, so a reported rectangle no longer means what it used
    // to at any pixel index.
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), None);
    // Not seeded from the old grid's halvings either: they are of different content at every index
    // now, so inheriting them (even just outside the reported rectangle) would be wrong pixels at a
    // reduced zoom.
    assert!(new.memoized_half().is_none(), "a shifted grid must not inherit the old grid's halvings");
}

#[test]
fn the_lineage_forgets_changes_past_its_limit() {
    let (mut e, id, layer_a) = document();
    // A second layer at the same (0, 0), 120 x 80, 1:1 placement, sharing the one selection: the ring
    // (`Lineage`) is shared by every layer of a document, so interleaving edits on two layers fills it
    // with roughly half as many edits per layer as editing one layer alone would need (ruling M5: the
    // original version of this test edited a single layer exactly `LINEAGE_LIMIT` times, which makes
    // the ring's real capacity and `Lineage::delta`'s own `for _ in 0..LINEAGE_LIMIT` loop bound the
    // same number -- indistinguishable. Removing the ring's eviction entirely still left that test
    // green, because `delta` gives up on its own once it has walked back `LINEAGE_LIMIT` links for
    // that one layer, whether or not that link was ever evicted).
    let png_b = encode_png(&pattern(120, 80, 13), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png_b, "B", Some(p(60.0, 40.0))).unwrap();
    let layer_b = e.state(id).unwrap().layers[1].id;
    select(&mut e, id, 10.0, 10.0, 12.0, 12.0);
    let identity = LayerTransform::axis_aligned(p(0.0, 0.0), Size { width: 120.0, height: 80.0 });
    let expected = expected_region(10.0, 10.0, 12.0, 12.0, &identity, 120, 80, 120, 80);
    let r0 = revision(&e, id);
    // LINEAGE_LIMIT + 2 total edits, alternating A, B, A, B, ...: layer A itself is touched only
    // about half of LINEAGE_LIMIT times (well under `delta`'s own loop bound), while the ring -
    // shared with B - holds only the last LINEAGE_LIMIT of all LINEAGE_LIMIT + 2 pushes, so exactly
    // the first 2 (A's very first edit, then B's) fall out. A's own edit count staying far below
    // LINEAGE_LIMIT is what makes the `None` below possible only through real eviction.
    let rounds = LINEAGE_LIMIT + 2;
    let mut checkpoint = None;
    for i in 0..rounds {
        let target = if i % 2 == 0 { layer_a } else { layer_b };
        run(&mut e, id, Command::InvertPixels { id: target, mask: false });
        // A snapshot of layer A's revision LINEAGE_LIMIT / 2 rounds before the end: still comfortably
        // inside the ring (only the first 2 pushes were evicted), so its delta to the current
        // revision must walk many interleaved links and land on the exact rectangle, not just "some"
        // value (ruling M5).
        if i == rounds - 1 - LINEAGE_LIMIT / 2 { checkpoint = Some(revision(&e, id)); }
    }
    assert_eq!(e.pixels_delta(id, layer_a, r0).unwrap(), None,
        "layer A's first edit was evicted, though A itself was touched far fewer than LINEAGE_LIMIT times");
    assert_eq!(e.pixels_delta(id, layer_a, checkpoint.expect("captured")).unwrap(), Some(expected),
        "still inside the ring: the exact rectangle every one of these edits reports, not just a value");
}
