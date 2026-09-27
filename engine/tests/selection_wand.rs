//! The Magic Wand (WandPixels.c, MagicWand.swift) and loading a layer's or a mask's pixels as a
//! selection (MaskTracing.swift), ported from Compositor for Mac's MagicWandTests.swift and the
//! load cases of SelectionTests.swift and LayerMaskTests.swift, with their images and values.
use compositor_engine::*;
use std::collections::BTreeSet;
use uuid::Uuid;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
/// Premultiplied RGBA, top row first (MagicWandTests `image`).
fn image(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width { data.extend_from_slice(&color(x, y)); } }
    Raster::from_premultiplied(width, height, data)
}
/// The pixel indices an outline covers: filled without antialiasing, at least half (MagicWandTests `pixels`).
fn pixels(contours: &[Contour], width: u32, height: u32) -> BTreeSet<u32> {
    let c = rasterize(contours, 0.0, 0.0, width, height, false);
    (0..width * height).filter(|i| c.bytes()[*i as usize] >= 128).collect()
}
fn block(columns: std::ops::Range<u32>, rows: std::ops::Range<u32>, width: u32) -> BTreeSet<u32> {
    rows.flat_map(|y| columns.clone().map(move |x| y * width + x)).collect()
}
fn settings(tolerance: u32, sample_radius: u32, contiguous: bool) -> WandSettings { WandSettings { tolerance, sample_radius, contiguous, all_layers: false } }
fn wand(image: &Raster, at: Point, s: WandSettings) -> BTreeSet<u32> {
    magic_wand(image, at, &s).unwrap().map_or_else(BTreeSet::new, |c| pixels(&c, image.width, image.height))
}

#[test]
fn contiguous_stops_at_other_colors_while_non_contiguous_finds_every_match() {
    let stripes = image(10, 4, |x, _| if x < 3 || x >= 6 { RED } else { BLUE });
    let (left, right) = (block(0..3, 0..4, 10), block(6..10, 0..4, 10));
    assert_eq!(wand(&stripes, p(1.5, 2.5), settings(32, 0, true)), left);
    assert_eq!(wand(&stripes, p(1.5, 2.5), settings(32, 0, false)), left.union(&right).copied().collect());
    // Rows stay the right way up: clicking the top row selects the top row.
    let banded = image(4, 3, |_, y| if y == 0 { RED } else { BLUE });
    assert_eq!(wand(&banded, p(1.0, 0.0), settings(32, 0, true)), [0, 1, 2, 3].into());
    assert_eq!(magic_wand(&banded, p(9.0, 0.0), &settings(32, 0, true)).unwrap(), None, "outside the image");
}

#[test]
fn tolerance_applies_to_every_channel_including_alpha() {
    let columns = [[100, 100, 100, 255], [132, 100, 100, 255], [133, 100, 100, 255], [100, 100, 100, 222]];
    let row = image(4, 1, |x, _| columns[x as usize]);
    assert_eq!(wand(&row, p(0.5, 0.5), settings(0, 0, false)), [0].into());
    assert_eq!(wand(&row, p(0.5, 0.5), settings(32, 0, false)), [0, 1].into());
    assert_eq!(wand(&row, p(0.5, 0.5), settings(33, 0, false)), [0, 1, 2, 3].into());
}

#[test]
fn sample_size_averages_the_pixels_around_the_click() {
    let dot = image(5, 5, |x, y| if x == 2 && y == 2 { [255, 255, 255, 255] } else { [0, 0, 0, 255] });
    assert_eq!(wand(&dot, p(2.5, 2.5), settings(10, 0, false)), [12].into());
    // A 3 x 3 average is grey (255 + 4) / 9 = 28: black is within 30 of it, the white centre is not.
    assert_eq!(wand(&dot, p(2.5, 2.5), settings(30, 1, false)), (0..25).filter(|i| *i != 12).collect());
}

#[test]
fn outlines_reproduce_their_pixels_with_holes_and_corner_touches() {
    let (width, height) = (8u32, 6u32);
    let mut mask = vec![0u8; 48];
    // A 3 x 3 ring around a hole, against the image's corner, and two pixels touching only diagonally.
    for y in 0..3 { for x in 0..3 { if !(x == 1 && y == 1) { mask[y * 8 + x] = 255; } } }
    mask[4 * 8 + 5] = 255;
    mask[5 * 8 + 6] = 255;
    let expected: BTreeSet<u32> = (0..48).filter(|i| mask[*i as usize] != 0).collect();
    let loops = trace_pixels(&mask, width as usize, height as usize).unwrap();
    let contours: Vec<Contour> = loops.iter().map(|l| l.iter().map(|q| [q[0] * SUBPIXEL as i32, q[1] * SUBPIXEL as i32]).collect()).collect();
    assert_eq!(pixels(&contours, width, height), expected);
    // Corners only: the ring's outer loop and its hole have four each, and the diagonal pair stays
    // two separate squares (the walk turns right where they touch).
    let mut lengths: Vec<usize> = loops.iter().map(Vec::len).collect();
    lengths.sort();
    assert_eq!(lengths, vec![4, 4, 4, 4]);
    assert!(trace_pixels(&[0u8; 4], 2, 2).unwrap().is_empty());
}

#[test]
fn an_outline_past_eight_million_edges_is_too_detailed() {
    // A 2001 x 2000 checkerboard, non-contiguous: 2,001,000 matching pixels, four edges each,
    // 8,004,000 edges, just past WAND_EDGE_LIMIT. 2000 x 2000 gives exactly 8,000,000, which passes.
    let checker = |w: u32, h: u32| image(w, h, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE });
    assert_eq!(magic_wand(&checker(2001, 2000), p(0.5, 0.5), &settings(0, 0, false)), Err(TraceError::TooDetailed));
    assert!(magic_wand(&checker(2000, 2000), p(0.5, 0.5), &settings(0, 0, false)).unwrap().is_some());
}

/// A document opened from `doc` in a fresh engine, as a project would be.
fn opened(doc: &Document) -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.open_package(&save_package(doc).unwrap(), None).unwrap();
    (e, id)
}
fn selected(e: &Engine, id: Uuid) -> BTreeSet<u32> {
    let d = e.document(id).unwrap();
    d.selection.as_ref().map_or_else(BTreeSet::new, |s| pixels(&s.contours, d.width, d.height))
}
fn coverage(e: &Engine, id: Uuid, x: u32, y: u32) -> u8 {
    let d = e.document(id).unwrap();
    let Some(s) = &d.selection else { return 0 };
    SelectionClip::new(s, d.width, d.height).on_grid(&Affine::IDENTITY, d.width, d.height).bytes()[(y * d.width + x) as usize]
}
fn wand_at(e: &mut Engine, id: Uuid, at: Point, mode: SelectionMode, all_layers: bool) {
    e.execute(id, Command::MagicWand { at, mode, settings: WandSettings { all_layers, ..WandSettings::default() }, antialiased: true }).unwrap();
}

#[test]
fn the_wand_reads_the_active_layer_or_every_visible_layer_and_combines_modes() {
    // MagicWandTests.theWandReadsTheActiveLayerOrEveryVisibleLayerAndCombinesModes: a 20 x 10 red
    // and blue image under a blank active layer.
    let mut doc = Document::new(20, 10);
    let halves = Layer::with_pixels("Halves", image(20, 10, |x, _| if x < 10 { RED } else { BLUE }), p(0.0, 0.0));
    let blank = Layer::blank("Layer 1", doc.size());
    doc.active_layer_id = Some(blank.id);
    doc.layers = vec![halves, blank];
    let (mut e, id) = opened(&doc);
    let left = block(0..10, 0..10, 20);
    // The blank active layer is transparent everywhere, so the whole canvas matches.
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Replace, false);
    assert_eq!(selected(&e, id).len(), 200);
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id), left);
    let count = e.state(id).unwrap().undo_depth;
    wand_at(&mut e, id, p(15.0, 5.0), SelectionMode::Add, true);
    assert_eq!(selected(&e, id).len(), 200);
    assert_eq!(e.state(id).unwrap().undo_depth, count + 1);
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Subtract, true);
    assert_eq!(selected(&e, id), (0..200).filter(|i| !left.contains(i)).collect());
    e.undo(id).unwrap();
    assert_eq!(selected(&e, id).len(), 200);
    // Clicking inside a selection makes a new wand selection rather than deselecting
    // (clickingInsideASelectionMakesANewWandSelectionRatherThanDeselecting).
    e.execute(id, Command::SelectAll).unwrap();
    wand_at(&mut e, id, p(3.5, 5.5), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id), left);
}

#[test]
fn the_wand_reads_the_active_layer_through_its_transform_but_not_its_mask_or_opacity() {
    // A 10 x 5 red layer stretched to 20 x 10 at (4, 2), at 10% opacity, under a mask hiding it all.
    // Through its opacity it would be (26, 0, 0, 26), within the default 32 of the clear canvas
    // around it; through its mask, clear; unscaled, (4, 2)-(14, 7).
    let mut doc = Document::new(40, 20);
    let mut red = Layer::with_pixels("Red", image(10, 5, |_, _| RED), p(4.0, 2.0));
    red.transform.size = Size { width: 20.0, height: 10.0 };
    red.opacity = 0.1;
    red.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![0]), enabled: true, placement: None, linked: None });
    doc.active_layer_id = Some(red.id);
    doc.layers = vec![red];
    let (mut e, id) = opened(&doc);
    wand_at(&mut e, id, p(10.0, 6.0), SelectionMode::Replace, false);
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(4.0, 2.0, 20.0, 10.0)));
    // With All Layers the mask hides it, and the transparent canvas matches everywhere.
    wand_at(&mut e, id, p(10.0, 6.0), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id).len(), 800);
}

#[test]
fn a_point_off_the_canvas_is_refused_and_nothing_matching_deselects() {
    let mut doc = Document::new(8, 8);
    let mut layer = Layer::with_pixels("Dot", image(3, 3, |x, y| if x == 1 && y == 1 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }), p(0.0, 0.0));
    layer.name = "Dot".into();
    doc.active_layer_id = Some(layer.id);
    doc.layers = vec![layer];
    let (mut e, id) = opened(&doc);
    assert!(e.execute(id, Command::MagicWand { at: p(8.0, 1.0), mode: SelectionMode::Replace, settings: WandSettings::default(), antialiased: true }).is_err());
    e.execute(id, Command::SelectAll).unwrap();
    // A 3 x 3 average at the white dot is grey 28: tolerance 10 matches nothing there, the dot included.
    e.execute(id, Command::MagicWand { at: p(1.5, 1.5), mode: SelectionMode::Replace, settings: WandSettings { tolerance: 10, sample_radius: 1, contiguous: true, all_layers: false }, antialiased: true }).unwrap();
    assert!(e.document(id).unwrap().selection.is_none(), "nothing matched: Replace deselects");
}

/// A 100 x 100 mask: white, a black square (20, 30)-(60, 70), and a white hole (30, 40)-(40, 50)
/// inside it (SelectionTests `maskedLayer`). Two interior pixels (I5) sit right at the Mac's 50%
/// threshold: (25, 32) at 127 stays dark (loads), (27, 34) at 128 does not (a one-pixel hole),
/// both well inside the square's own bounds so the outer bounding box is untouched.
fn masked_pixels() -> GrayRaster {
    let mut m = vec![255u8; 10_000];
    for y in 30..70 { for x in 20..60 { m[y * 100 + x] = 0; } }
    for y in 40..50 { for x in 30..40 { m[y * 100 + x] = 255; } }
    m[32 * 100 + 25] = 127;
    m[34 * 100 + 27] = 128;
    GrayRaster::from_bytes(100, 100, m)
}

#[test]
fn ctrl_clicking_a_mask_selects_its_black_areas() {
    // SelectionTests.cmdClickingAMaskSelectsItsBlackAreas.
    let mut doc = Document::new(100, 100);
    let mut layer = Layer::blank("Layer 1", doc.size());
    layer.mask = Some(Mask { pixels: masked_pixels(), enabled: true, placement: None, linked: None });
    let layer_id = layer.id;
    doc.active_layer_id = Some(layer_id);
    doc.layers = vec![layer];
    let (mut e, id) = opened(&doc);
    let load = |e: &mut Engine, mode| e.execute(id, Command::LoadMaskSelection { id: layer_id, mode, antialiased: true }).unwrap();
    load(&mut e, SelectionMode::Replace);
    assert_eq!(coverage(&e, id, 25, 35), 255, "black: selected");
    assert_eq!(coverage(&e, id, 35, 45), 0, "the white hole: not selected");
    assert_eq!(coverage(&e, id, 80, 80), 0, "white surroundings");
    assert_eq!((coverage(&e, id, 59, 69), coverage(&e, id, 60, 70)), (255, 0), "exact pixel edges");
    // I5: the Mac's threshold is mask < 128; 127 loads, 128 does not.
    assert_eq!(coverage(&e, id, 25, 32), 255, "mask 127 is dark enough to load (I5)");
    assert_eq!(coverage(&e, id, 27, 34), 0, "mask 128 is not dark enough to load (I5)");
    let square = vec![p(80.0, 80.0), p(90.0, 80.0), p(90.0, 90.0), p(80.0, 90.0)];
    e.execute(id, Command::SelectShape { kind: SelectionShape::Freehand, points: square, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    load(&mut e, SelectionMode::Add);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 25, 35)), (255, 255));
    load(&mut e, SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 25, 35)), (255, 0));
    assert_eq!(Command::LoadMaskSelection { id: layer_id, mode: SelectionMode::Add, antialiased: true }.action_name(), "Load Mask Selection");
}

#[test]
fn a_mask_selection_follows_the_layer_transform_and_an_all_white_mask_selects_nothing() {
    // SelectionTests.maskSelectionFollowsTheLayerTransformAndIgnoresAllWhiteMasks: the layer and
    // its mask stretched 2x from the canvas origin.
    let mut doc = Document::new(200, 200);
    let mut layer = Layer::blank("Layer 1", Size { width: 100.0, height: 100.0 });
    layer.transform.size = Size { width: 200.0, height: 200.0 };
    layer.mask = Some(Mask { pixels: masked_pixels(), enabled: true, placement: None, linked: None });
    let mut white = Layer::blank("Layer 2", doc.size());
    white.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: true, placement: None, linked: None });
    let (masked_id, white_id) = (layer.id, white.id);
    doc.layers = vec![layer, white];
    let (mut e, id) = opened(&doc);
    e.execute(id, Command::LoadMaskSelection { id: masked_id, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(40.0, 60.0, 80.0, 80.0)));
    e.execute(id, Command::Deselect).unwrap();
    assert!(matches!(e.execute(id, Command::LoadMaskSelection { id: white_id, mode: SelectionMode::Replace, antialiased: true }), Err(CommandError::Refused(_))));
    assert!(e.document(id).unwrap().selection.is_none(), "no black anywhere: nothing to select");
}

#[test]
fn ctrl_clicking_a_layer_selects_its_opaque_pixels() {
    // SelectionTests.cmdClickingALayerSelectsItsOpaquePixels: a 50 x 50 image, an opaque ring
    // (10,10)-(40,40) around a clear (20,20)-(30,30), a 25% corner pixel, shown at 2x from (50, 50).
    // Two pixels inside the hole (I5) sit right at the Mac's 50% threshold: (21, 21) at alpha 127
    // stays clear (does not load), (23, 23) at alpha 128 loads, both interior to the ring's own
    // bounds so the outer bounding box is untouched.
    let mut doc = Document::new(200, 200);
    let ring = image(50, 50, |x, y| {
        if x == 0 && y == 0 { [64, 0, 0, 64] }
        else if x == 21 && y == 21 { [127, 0, 0, 127] }
        else if x == 23 && y == 23 { [128, 0, 0, 128] }
        else if (10..40).contains(&x) && (10..40).contains(&y) && !((20..30).contains(&x) && (20..30).contains(&y)) { RED }
        else { [0, 0, 0, 0] }
    });
    let mut layer = Layer::with_pixels("Ring", ring, p(50.0, 50.0));
    layer.transform.size = Size { width: 100.0, height: 100.0 };
    let blank = Layer::blank("Layer 2", doc.size());
    let (ring_id, blank_id) = (layer.id, blank.id);
    doc.layers = vec![layer, blank];
    let (mut e, id) = opened(&doc);
    let load = |e: &mut Engine, layer, mode| e.execute(id, Command::LoadLayerSelection { id: layer, mode, antialiased: true });
    load(&mut e, ring_id, SelectionMode::Replace).unwrap();
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(70.0, 70.0, 60.0, 60.0)), "2x scale");
    assert_eq!(coverage(&e, id, 75, 75), 255, "the opaque ring");
    assert_eq!(coverage(&e, id, 100, 100), 0, "the clear centre");
    assert_eq!(coverage(&e, id, 50, 50), 0, "the 25% pixel is under the threshold");
    // I5: the Mac's threshold is alpha >= 128; 127 does not load, 128 does.
    assert_eq!(coverage(&e, id, 92, 92), 0, "alpha 127 is under the threshold (I5)");
    assert_eq!(coverage(&e, id, 96, 96), 255, "alpha 128 meets the threshold (I5)");
    let corner = vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0), p(0.0, 20.0)];
    e.execute(id, Command::SelectShape { kind: SelectionShape::Freehand, points: corner, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    load(&mut e, ring_id, SelectionMode::Add).unwrap();
    assert_eq!((coverage(&e, id, 10, 10), coverage(&e, id, 75, 75)), (255, 255));
    let before = e.document(id).unwrap().selection.clone();
    assert!(matches!(load(&mut e, blank_id, SelectionMode::Replace), Err(CommandError::Refused(_))));
    assert_eq!(e.document(id).unwrap().selection, before, "an empty layer selects nothing");
}

#[test]
fn a_folder_mask_loads_its_black_areas_after_an_invert() {
    // LayerMaskTests.folderMaskCanBePaintedInvertedAndLoadedAsASelection, its load: a folder over a
    // 40 x 20 red layer, its mask white but for a black dot, inverted: black everywhere but the dot.
    let mut doc = Document::new(40, 20);
    let mut folder = Layer::blank("Folder", doc.size());
    folder.is_group = true;
    let dot: Vec<u8> = (0..20).flat_map(|y: i32| (0..40).map(move |x: i32| if (x - 10).pow(2) + (y - 10).pow(2) <= 16 { 0 } else { 255 })).collect();
    folder.mask = Some(Mask { pixels: GrayRaster::from_bytes(40, 20, dot), enabled: true, placement: None, linked: None });
    let mut red = Layer::with_pixels("Red", image(40, 20, |_, _| RED), p(0.0, 0.0));
    red.parent_id = Some(folder.id);
    let folder_id = folder.id;
    doc.layers = vec![folder, red];
    let (mut e, id) = opened(&doc);
    e.execute(id, Command::InvertMask { id: folder_id }).unwrap();
    e.execute(id, Command::LoadMaskSelection { id: folder_id, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    let width = e.document(id).unwrap().selection.as_ref().unwrap().bounds().unwrap().width;
    assert!(width > 30.0, "{width}");
    assert_eq!((coverage(&e, id, 10, 10), coverage(&e, id, 30, 10)), (0, 255), "the dot stays out");
}

#[test]
fn the_wand_refuses_an_outline_too_detailed_to_draw() {
    // Through the engine, the Mac's message (MagicWand.swift:28).
    let mut doc = Document::new(2001, 2000);
    let checker = Layer::with_pixels("Checker", image(2001, 2000, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE }), p(0.0, 0.0));
    doc.active_layer_id = Some(checker.id);
    doc.layers = vec![checker];
    let (mut e, id) = opened(&doc);
    let err = e.execute(id, Command::MagicWand { at: p(0.5, 0.5), mode: SelectionMode::Replace, settings: WandSettings { tolerance: 0, sample_radius: 0, contiguous: false, all_layers: false }, antialiased: true }).unwrap_err();
    assert_eq!(err.to_string(), "That selection is too detailed to outline. Try a different Tolerance, or turn on Contiguous.");
    assert!(e.document(id).unwrap().selection.is_none());
}
