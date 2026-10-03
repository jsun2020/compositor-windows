//! Fills and gradients on a mask grow the mask past its layer to the canvas (Task 14a): Compositor
//! 1.3.7's rule, chosen by the user on 2026-09-29 over the plan's 1.2.10 oracle (Gradient.swift:48;
//! SelectionEdits.swift:200-210; BrushStroke.swift:157-173, :576-589, :639-670, :764-773;
//! EditorSession+Brush.swift:13-20, :180-187; LayerMask.swift:54-76; all at v1.3.7). Every expected value
//! is worked out here from those rules, never pasted.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
fn linear(start: Point, end: Point, from: [f64; 4], to: [f64; 4]) -> GradientSpec {
    GradientSpec { shape: GradientShape::Linear, start, end, from, to, opacity: 1.0 }
}
const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
const CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

/// An upright transform over (`origin`) of `size` document pixels.
fn at(origin: (f64, f64), size: (f64, f64)) -> LayerTransform { LayerTransform::axis_aligned(p(origin.0, origin.1), Size { width: size.0, height: size.1 }) }
/// A mask of its layer's grid (no placement, linked), `value(x, y)` at each pixel.
fn covering(w: u32, h: u32, value: impl Fn(u32, u32) -> u8) -> Mask {
    let data = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| value(x, y)).collect();
    Mask { pixels: GrayRaster::from_bytes(w, h, data), enabled: true, placement: None, linked: None }
}
/// A `canvas` document whose last layer, active, is `size` opaque blue pixels placed by `transform` under
/// `mask`; with `hog`, a folder below it whose mask holds that many of the mask budget's pixels (a
/// folder has no pixels, so the pixel budget is untouched).
fn document(canvas: (u32, u32), size: (u32, u32), transform: LayerTransform, mask: Mask, hog: Option<(u32, u32)>) -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(canvas.0, canvas.1);
    let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(size.0, size.1, [0, 0, 200, 255].repeat((size.0 * size.1) as usize)), p(0.0, 0.0));
    layer.transform = transform;
    layer.mask = Some(mask);
    let lid = layer.id;
    let mut layers = Vec::new();
    if let Some((w, h)) = hog {
        let mut big = Layer::blank("Big Folder", doc.size());
        big.is_group = true;
        big.mask = Some(Mask { pixels: GrayRaster::from_bytes(w, h, vec![255u8; (w * h) as usize]), enabled: true, placement: None, linked: None });
        layers.push(big);
    }
    layers.push(layer);
    doc.active_layer_id = Some(lid);
    doc.layers = layers;
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    (e, id, lid)
}
fn mask_of(e: &Engine, id: Uuid) -> Mask { e.document(id).unwrap().layers.last().unwrap().mask.clone().unwrap() }
fn value(m: &Mask, x: u32, y: u32) -> u8 { m.pixels.bytes()[(y * m.pixels.width + x) as usize] }
fn shown(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0) }

#[test]
fn a_mask_gradient_grows_the_mask_to_the_canvas_places_it_there_and_undoes() {
    // A 20 x 10 layer at (30, 15) on 100 x 40, one document pixel a pixel, under a mask of its own grid
    // whose left half is black. The mask's grid is the layer's; the canvas on it runs from (-30, -15) to
    // (70, 25), so the grown grid is the canvas itself: 100 x 40, the old grid at (30, 15) in it.
    let half = covering(20, 10, |x, _| if x < 10 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), half.clone(), None);
    // What the app decides the job worker by (ruling C1): the mask grown to the canvas, not its 200 pixels.
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 100 * 40);
    let count = depth(&e, id);
    // Black to white from x 0.5 to 100.5: the pixel at column i (centre i + 0.5) is i / 100 along.
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, WHITE) });
    assert_eq!(depth(&e, id), count + 1);
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (100, 40), "grown to the canvas");
    let ramp = |i: u32| (255.0 * i as f64 / 100.0).round() as u8;
    // Past the layer on every side, and over its old black half (37) and white half (45).
    for (x, y) in [(0, 0), (1, 39), (37, 20), (45, 20), (99, 5)] { assert_eq!(value(&m, x, y), ramp(x), "({x}, {y})"); }
    let placed = m.placement.expect("placed on its own: it grew (EditorSession+Brush.swift:185)");
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 100.0, height: 40.0 }), "over the canvas, a document pixel a pixel");
    assert!(m.is_linked());
    // The layer shows through the grown mask where the layer is.
    assert_eq!(shown(&e, id, 37, 20)[3], ramp(37));
    // Linked, it moves with its layer (LayerMask.swift:54-59): 5 right and 3 down.
    run(&mut e, id, Command::SetLayerTransform { id: layer, transform: at((35.0, 18.0), (20.0, 10.0)) });
    assert_eq!(mask_of(&e, id).placement.unwrap().origin, p(5.0, 3.0));
    // Undone: the old mask, covering its layer again.
    e.undo(id).unwrap();
    e.undo(id).unwrap();
    let back = mask_of(&e, id);
    assert_eq!((back.pixels.bytes(), back.placement), (half.pixels.bytes(), None));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn the_grown_area_starts_as_the_masks_background_white_or_black() {
    // Black fading to nothing from x 0.5 to 100.5: at column i the black's alpha is 1 - i / 100, over
    // what the mask held there, which where it grew is its background: background x i / 100, rounded.
    let g = linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, CLEAR);
    let over = |under: f64, i: u32| (under * i as f64 / 100.0 + 0.5).floor() as u8;
    // White edges: the background reveals (LayerMask.background, LayerMask.swift:61-76).
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(20, 10, |_, _| 255), None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g.clone() });
    let m = mask_of(&e, id);
    for i in [5, 20, 80, 95] { assert_eq!(value(&m, i, 3), over(255.0, i), "white background, column {i}"); }
    // Black edges round a white middle: the background hides; the old middle is still white under it.
    let ring = covering(20, 10, |x, y| if x == 0 || y == 0 || x == 19 || y == 9 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), ring, None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g });
    let m = mask_of(&e, id);
    for i in [5, 20, 80, 95] { assert_eq!(value(&m, i, 3), 0, "black background, column {i}"); }
    // Column 40, row 20 is the layer's pixel (10, 5), in its white middle.
    assert_eq!(value(&m, 40, 20), over(255.0, 40));
}

#[test]
fn inside_a_selection_the_mask_keeps_its_old_grid_and_the_whole_tiles_the_edit_touched() {
    // A 100 x 100 layer at (450, 250) on 1000 x 600, under a white mask of its own grid. Grown, its grid
    // would be the canvas: 1000 x 600 from (-450, -250). A selection from (50, 50) to (150, 110), its clip
    // a pixel wider, lies well inside the first 256 x 256 tile counted from that corner (BrushStroke.swift:
    // 666-670): the mask keeps the old grid joined with that tile, from (-450, -250) to (100, 100) in the
    // layer's grid: 550 x 350, placed over (0, 0) to (550, 350).
    let (mut e, id, layer) = document((1000, 600), (100, 100), at((450.0, 250.0), (100.0, 100.0)), covering(100, 100, |_, _| 255), None);
    select(&mut e, id, 50.0, 50.0, 100.0, 60.0);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 550 * 350);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.0, 0.0), p(1000.0, 0.0), BLACK, BLACK) });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (550, 350));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 550.0, height: 350.0 }));
    assert_eq!(value(&m, 100, 80), 0, "painted inside the selection");
    assert_eq!(value(&m, 200, 200), 255, "the background, in the tile past the selection");
    assert_eq!(value(&m, 300, 100), 255, "the background, past the tile and the old grid");
    assert_eq!(value(&m, 500, 300), 255, "the old mask, unpainted");
}

#[test]
fn a_scaled_and_flipped_layers_mask_grows_to_the_canvas_along_its_own_grid() {
    // A 20 x 10 layer drawn at twice its size over (30, 10)-(70, 30), flipped both ways, on 100 x 40: a
    // mask pixel is 2 x 2 document pixels, and the grid runs right to left and bottom to top. Its pixel
    // (u, v) has its centre at x = 50 - 2 (u + 0.5 - 10), y = 20 - 2 (v + 0.5 - 5), so the canvas on the
    // grid runs from u = -15 to 35 and v = -5 to 15 (BrushStroke.swift:168): grown, 50 x 20 with the old
    // grid at (15, 5), placed over the canvas (100 x 40 at (0, 0)) and flipped as the layer is. The grown
    // pixel i then has its centre at x = 50 - 2 (i - 15 + 0.5 - 10) = 99 - 2 i.
    let mut t = at((30.0, 10.0), (40.0, 20.0));
    t.flip_x = true;
    t.flip_y = true;
    let (mut e, id, layer) = document((100, 40), (20, 10), t, covering(20, 10, |_, _| 255), None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.0, 20.0), p(100.0, 20.0), BLACK, WHITE) });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (50, 20));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size, placed.flip_x, placed.flip_y), (p(0.0, 0.0), Size { width: 100.0, height: 40.0 }, true, true));
    for i in [0u32, 10, 24, 49] {
        let x = 99.0 - 2.0 * i as f64;
        assert_eq!(value(&m, i, 7), (255.0 * x / 100.0 + 0.5).floor() as u8, "column {i}, centred at x {x}");
    }
}

#[test]
fn a_solid_mask_on_its_own_placement_is_painted_at_one_pixel_per_document_pixel() {
    // A 1 x 1 white mask placed on its own over (10, 5)-(40.4, 25), on a 100 x 40 layer over its canvas.
    // Painted, its grid is round(30.4) x 20 = 30 x 20 pixels over its place (BrushStroke.swift:160-165),
    // where 1.2.10 painted its single pixel; a fill then grows that grid to the canvas: x from
    // (0 - 10) x 30 / 30.4 to (100 - 10) x 30 / 30.4, rounded out, and y from -5 to 35 (the canvas is less
    // than a tile across, so a selection's fill grows it all the same). The old grid lands at (10, 5).
    let place = at((10.0, 5.0), (30.4, 20.0));
    let mut solid = covering(1, 1, |_, _| 255);
    solid.placement = Some(place);
    solid.linked = Some(false);
    let (mut e, id, layer) = document((100, 40), (100, 40), at((0.0, 0.0), (100.0, 40.0)), solid, None);
    // The left half of the place selected: the old grid's pixels are 30.4 / 30 wide, so its column 2
    // (centre x 12.5) is inside x < 25 and its column 27 (centre x 37.9) outside.
    select(&mut e, id, 10.0, 5.0, 15.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    let (x0, x1) = ((-10.0f64 * 30.0 / 30.4).floor(), (90.0f64 * 30.0 / 30.4).ceil());
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), ((x1 - x0) as u32, 40));
    let placed = m.placement.unwrap();
    let unit = 30.4 / 30.0;
    assert!((placed.size.width - (x1 - x0) * unit).abs() < 1e-9 && placed.size.height == 40.0, "{placed:?}");
    assert!((placed.origin.x - (10.0 + x0 * unit)).abs() < 1e-9 && placed.origin.y.abs() < 1e-9, "{placed:?}");
    // Old column 2, row 10 (selected), old column 27 (not), and old column -7 (grown, the background).
    assert_eq!((value(&m, 12, 15), value(&m, 37, 15), value(&m, 3, 15)), (0, 255, 255));
}

#[test]
fn a_fill_on_a_mask_without_a_selection_grows_it_to_the_whole_grown_grid() {
    // SelectionEdits.swift:200-210: a fill on a mask covers the whole canvas past the mask's own area. A
    // 20 x 10 layer at (90, 35) overhangs the 100 x 40 canvas by 10 and 5, under a covering 1 x 1 mask of
    // 128 (its background: 128 x 2 >= 255, white). On the layer's grid the canvas runs from (-90, -35) to
    // (10, 5); joined with the old grid, (-90, -35) to (20, 10): 110 x 45, the old grid at (90, 35), placed
    // over (0, 0) to (110, 45). Black paints every pixel whose centre is on the canvas; the old grid keeps
    // its 128 past the canvas; the corner that is neither, right of x 100 and above y 35, is the background.
    let (mut e, id, layer) = document((100, 40), (20, 10), at((90.0, 35.0), (20.0, 10.0)), covering(1, 1, |_, _| 128), None);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 110 * 45);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (110, 45));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 110.0, height: 45.0 }));
    assert_eq!((value(&m, 5, 5), value(&m, 95, 38)), (0, 0), "on the canvas");
    assert_eq!((value(&m, 105, 38), value(&m, 95, 42)), (128, 128), "the old mask past the canvas");
    assert_eq!(value(&m, 105, 10), 255, "neither: the background");
    assert_eq!(Command::Fill { id: layer, mask: true, color: [0.0; 3] }.action_name(), "Fill Mask");
}

#[test]
fn a_fill_on_a_mask_inside_a_selection_grows_it_by_the_tiles_the_clip_touches() {
    // A 100 x 100 layer at (450, 250) on 1000 x 600 under a covering black 1 x 1 mask (a hide-all mask:
    // its background is black). Grown, its grid would run from (-450, -250) to (550, 350); tiles count from
    // that corner. A selection from (600, 400) to (700, 450), its clip (599, 399)-(701, 451) a pixel wider,
    // is (149, 149)-(251, 201) on the layer's grid, (599, 399)-(701, 451) from the corner: tile columns 2
    // (512-768) and row 1 (256-512), so the tiles reach (62, 6)-(318, 262) on the layer's grid. Joined with
    // the old grid, (0, 0)-(318, 262): the mask grows right and down only, 318 x 262, the old grid at
    // (0, 0), placed at (450, 250).
    let (mut e, id, layer) = document((1000, 600), (100, 100), at((450.0, 250.0), (100.0, 100.0)), covering(1, 1, |_, _| 0), None);
    select(&mut e, id, 600.0, 400.0, 100.0, 50.0);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 318 * 262);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [1.0, 1.0, 1.0] });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (318, 262));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(450.0, 250.0), Size { width: 318.0, height: 262.0 }));
    assert_eq!(value(&m, 200, 175), 255, "painted inside the selection");
    assert_eq!(value(&m, 300, 250), 0, "the background, in the tiles past the selection");
    assert_eq!(value(&m, 50, 50), 0, "the old mask, unpainted");
}

#[test]
fn a_mask_fill_or_gradient_is_refused_past_the_mask_budget_at_its_grown_size() {
    // The same 20 x 10 layer at (30, 15) and mask; a folder's mask holds 99,999,000 of the mask budget,
    // leaving 1,000 besides this mask's own 200 (EditorSession+Brush.swift:16-20): room for the mask as it
    // is, not for the mask grown to the canvas, 100 x 40 = 4,000 (BrushStroke.swift:582). The canvas is
    // less than a tile across, so a selection's fill grows it all the same.
    let (hog_w, hog_h) = (99_999u32, 1_000u32);
    let room = MAX_PIXELS - hog_w as u64 * hog_h as u64;
    assert!(100 * 40 > room && 20 * 10 <= room, "the fixture must starve only the grown mask");
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(20, 10, |_, _| 255), Some((hog_w, hog_h)));
    let g = linear(p(0.0, 0.0), p(100.0, 0.0), BLACK, WHITE);
    let too_large = || CommandError::Project(ProjectError::TooLarge);
    let (before, count) = (e.document(id).unwrap().clone(), depth(&e, id));
    assert_eq!(e.execute(id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] }), Err(too_large()));
    assert_eq!(e.execute(id, Command::Gradient { id: layer, mask: true, gradient: g.clone() }), Err(too_large()));
    select(&mut e, id, 32.0, 17.0, 5.0, 5.0);
    let (before_selected, count_selected) = (e.document(id).unwrap().clone(), depth(&e, id));
    assert_eq!(e.execute(id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] }), Err(too_large()));
    assert!(e.document(id).unwrap().same_content(&before_selected));
    assert_eq!(depth(&e, id), count_selected);
    // The preview shows nothing, and the size the app decides the worker by refuses too.
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g, dragging: true })).unwrap();
    assert!(e.preview(id).is_none());
    assert_eq!(e.edit_pixels(id, layer, true), Err(too_large()));
    run(&mut e, id, Command::Deselect);
    assert!(e.document(id).unwrap().same_content(&before));
    assert_eq!(depth(&e, id), count + 2, "the selection and its removal, nothing else");
}

#[test]
fn the_preview_shows_the_grown_mask_where_the_commit_leaves_it() {
    // Small enough not to be reduced (under GRADIENT_DRAG_LIMIT): the previewed mask is the committed
    // one byte for byte, at the same placement.
    let pattern = |w: u32, h: u32| covering(w, h, |x, y| ((x * 7 + y * 13) % 256) as u8);
    let (mut e, id, layer) = document((300, 200), (60, 40), at((100.0, 80.0), (60.0, 40.0)), pattern(60, 40), None);
    let g = GradientSpec { shape: GradientShape::Radial, start: p(130.0, 100.0), end: p(230.0, 150.0), from: BLACK, to: CLEAR, opacity: 0.8 };
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g.clone(), dragging: true })).unwrap();
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Mask { .. }));
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (300, 200), "grown to the canvas, not reduced");
    let previewed = (e.mask_pixels(id, layer).unwrap().unwrap().bytes().to_vec(), state.mask_placement);
    let through = shown(&e, id, 120, 90);
    e.set_preview(id, None).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].mask_placement, None, "taken back: covering its layer again");
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g.clone() });
    let m = mask_of(&e, id);
    assert_eq!(previewed, (m.pixels.bytes().to_vec(), m.placement));
    assert_eq!(shown(&e, id, 120, 90), through);
    // Reduced: the same layer on 3000 x 2000 grows to 3000 x 2000, shown while dragged at 750 x 500
    // (halved until at most 1024 across) over the whole canvas.
    let (mut e, id, layer) = document((3000, 2000), (60, 40), at((1000.0, 800.0), (60.0, 40.0)), pattern(60, 40), None);
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g, dragging: true })).unwrap();
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (750, 500));
    assert_eq!(state.mask_placement, Some(at((0.0, 0.0), (3000.0, 2000.0))));
    // Fix round 1, M-2: a uniform mid-grey (128) mask. Its background (`Mask::background`) is white
    // (128 x 2 >= 255), different from its own byte, so the uniform fast path (preview.rs) must show
    // 128 only over the old grid's footprint and the background past it -- not fill the whole grid with
    // 128, which is what dropping its `!grew` guard would do. A gradient whose line runs from x 0 to 1
    // paints nothing at x >= 1 (`GradientSpec::position` clamps to 1, so its alpha there is `to`'s: 0):
    // the sampled mask is left exactly as it was at both this test's points.
    let (mut e, id, layer) = document((300, 200), (60, 40), at((100.0, 80.0), (60.0, 40.0)), covering(1, 1, |_, _| 128), None);
    let flat = linear(p(0.0, 0.0), p(1.0, 0.0), BLACK, CLEAR);
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: flat, dragging: true })).unwrap();
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (300, 200), "grown to the canvas, not reduced");
    let grey = e.mask_pixels(id, layer).unwrap().unwrap();
    assert_eq!(grey.bytes()[90 * 300 + 120], 128, "inside the old grid (the layer's own box): the mask's own byte");
    assert_eq!(grey.bytes()[10 * 300 + 10], 255, "past the old grid: the background, not the mask's own byte");
}

#[test]
fn a_mask_gradient_through_a_job_leaves_what_it_leaves_in_place() {
    let half = covering(20, 10, |x, _| if x < 10 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), half, None);
    let command = Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, CLEAR) };
    let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), command.clone(), 0.0).unwrap();
    assert_eq!(output.mask, Some((100, 40)));
    assert_eq!(output.mask_placement.map(|t| (t.origin, t.size)), Some((p(0.0, 0.0), Size { width: 100.0, height: 40.0 })));
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None).unwrap();
    let through_job = mask_of(&e, id);
    e.undo(id).unwrap();
    run(&mut e, id, command);
    assert_eq!(mask_of(&e, id), through_job);
}

#[test]
fn a_second_fill_on_an_already_grown_mask_keeps_its_size_and_placement_and_reports_a_rectangle() {
    // Fix round 1, I-1: a float drift regrows an already-grown mask. A 100 x 70 layer drawn at 0.7x
    // (document size 70 x 49, origin (0, 1.4)) on a 700 x 490 canvas: the mask's own grid is the layer's,
    // 100 x 70, and the canvas maps through the 0.7x scale to mask-grid (0, 0)-(1000, 700) -- already
    // holding the layer's own grid, so a Fill grows the mask to exactly 1000 x 700. Reviewer's repro: a
    // float drift in `rect_on`'s inverse mapping of the canvas edge can round that to 1000 x 701 on a
    // SECOND fill of the mask already at that size (the first fill's own drift, if any, is absorbed by
    // `union`-ing with the still-unchanged 100 x 70 own grid, so only the second fill sees it; this
    // origin was found by sweeping multiples of 0.7 for one whose first fill lands exactly on 1000 x 700
    // -- an origin of (0, 0) happens not to drift at all, so it does not exercise this bug).
    let (mut e, id, layer) = document((700, 490), (100, 70), at((0.0, 1.4), (70.0, 49.0)), covering(1, 1, |_, _| 255), None);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    let grown = mask_of(&e, id);
    assert_eq!((grown.pixels.width, grown.pixels.height), (1000, 700), "grown to the canvas");
    let first_placement = grown.placement.expect("placed: it grew");
    let from = e.state(id).unwrap().layers[0].mask_revision;
    // A second Fill, inside a selection this time, on the already-grown mask: it must not regrow.
    select(&mut e, id, 140.0, 98.0, 140.0, 98.0);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [1.0, 1.0, 1.0] });
    let after = mask_of(&e, id);
    assert_eq!((after.pixels.width, after.pixels.height), (1000, 700), "unchanged: no float-drift regrowth");
    // Kept exactly, even though the mask did not grow this time -- the untested `m.placement.is_some()`
    // half of the placement rule (mask_grid, raster_edit.rs): dropping it would clear the placement
    // whenever an edit does not itself grow the mask.
    assert_eq!(after.placement, Some(first_placement), "the same placement, exactly");
    // Its lineage reports a rectangle, not None: the buffer kept its size and grid (`Lineage::record_edit`
    // records whole only when either changed), so only the selection's own rectangle on the mask's grid
    // is dirty -- computed the same way the app reads it (`SelectionClip::rect_on_grid`) from the
    // selection just made and the placement the mask kept.
    let doc = e.document(id).unwrap();
    let selection = doc.selection.as_ref().expect("a selection");
    let clip = SelectionClip::new(selection, doc.width, doc.height);
    let expected = clip.rect_on_grid(&first_placement.pixel_to_document(1000, 700), 1000, 700);
    assert!(expected.is_some(), "the selection reaches the grown mask");
    assert_eq!(e.mask_delta(id, layer, from).unwrap(), expected, "a rectangle, not a whole re-upload");
}

#[test]
fn a_masks_background_is_the_majority_of_its_edge_pixels_each_counted_once() {
    // `Mask::background` reads only the edge (a grown mask is canvas-sized, and the plan asks for it every
    // frame): each edge pixel counted once, white when their mean is at least half of 255.
    let by_definition = |m: &GrayRaster| {
        let (w, h) = (m.width, m.height);
        let edge: Vec<u64> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| x == 0 || y == 0 || x == w - 1 || y == h - 1)
            .map(|(x, y)| m.bytes()[(y * w + x) as usize] as u64).collect();
        if edge.iter().sum::<u64>() * 2 >= edge.len() as u64 * 255 { 255u8 } else { 0 }
    };
    // A 6 x 5 whose edge is exactly half white only when every edge pixel counts once: the top row and
    // the right column black, the bottom row and the left column white (18 edge pixels, 9 white).
    let half = covering(6, 5, |x, y| if y == 4 || (x == 0 && y > 0) { 255 } else { 0 });
    // A 3 x 3 whose four edge middles are white and four corners black (exactly half: white) turns
    // black if a corner is counted twice; a 1 x 4 with white ends and a black middle (half: white)
    // turns black if a one-pixel column's middle is counted twice (audit M-8).
    let cases = [covering(1, 1, |_, _| 127), covering(1, 1, |_, _| 128), covering(5, 1, |x, _| if x < 2 { 255 } else { 0 }),
        covering(1, 4, |_, y| if y < 3 { 255 } else { 0 }), covering(2, 2, |x, y| if x == y { 255 } else { 0 }), half.clone(),
        covering(3, 3, |x, y| if (x == 1) != (y == 1) { 255 } else { 0 }), covering(1, 4, |_, y| if y == 0 || y == 3 { 255 } else { 0 })];
    for m in &cases { assert_eq!(m.background(), by_definition(&m.pixels), "{} x {}", m.pixels.width, m.pixels.height); }
    assert_eq!(half.background(), 255);
}

#[test]
fn a_selections_region_is_the_rectangle_its_clip_fills() {
    // Pre-flight audit I-5: a mask edit reads where it paints from `SelectionClip::region` (the
    // selection's bounds grown by a pixel, rounded out and cut to the canvas), never filling the clip
    // on the UI thread; it must be exactly the rectangle `SelectionClip::new` fills, and None where
    // that has no coverage. Inside the canvas at fractional edges, feathered, part off the canvas, and
    // wholly off it.
    let (mut e, id, _) = document((300, 200), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(1, 1, |_, _| 255), None);
    let shape = |kind: SelectionShape, x: f64, y: f64, w: f64, h: f64| Command::SelectShape { kind, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true };
    let mut seen = (0, 0);
    for (select, feather) in [(shape(SelectionShape::Rectangle, 10.5, 20.25, 100.0, 50.0), 0), (shape(SelectionShape::Ellipse, 40.0, 30.0, 120.0, 90.0), 12),
        (shape(SelectionShape::Rectangle, 250.0, -40.0, 120.0, 100.0), 0), (shape(SelectionShape::Rectangle, 400.0, 300.0, 50.0, 50.0), 0)] {
        run(&mut e, id, select.clone());
        if feather > 0 { run(&mut e, id, Command::FeatherSelection { amount: feather }); }
        let doc = e.document(id).unwrap();
        let s = doc.selection.as_ref().expect("a selection, maybe off the canvas");
        let clip = SelectionClip::new(s, doc.width, doc.height);
        let filled = clip.coverage.as_ref().map(|k| (clip.origin.0, clip.origin.1, clip.origin.0 + k.width as i64, clip.origin.1 + k.height as i64));
        assert_eq!(SelectionClip::region(s, doc.width, doc.height), filled, "{select:?}, feathered {feather}");
        if filled.is_some() { seen.0 += 1 } else { seen.1 += 1 }
    }
    assert_eq!(seen, (3, 1), "three selections reach the canvas, one does not");
}
