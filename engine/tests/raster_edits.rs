//! Fill and the Gradient (Phase 4b-1): the Mac's GradientTests and the fill cases of its
//! SelectionEditTests, with their numbers, plus the layer's growth to the canvas, its trim and the
//! mask that follows (BrushStroke.swift:153-160, EditorSession+Brush.swift:154-188).
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }

/// A `width` x `height` document with one blank layer, active (`createDocument(emptyLayer: true)`).
fn blank(width: u32, height: u32) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    (e, id, layer)
}
/// The composite's premultiplied RGBA at one pixel, as the Mac's tests read the exported image.
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] {
    e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0)
}
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
fn linear(start: Point, end: Point, from: [f64; 4], to: [f64; 4], opacity: f64) -> GradientSpec {
    GradientSpec { shape: GradientShape::Linear, start, end, from, to, opacity }
}
/// The ramp's value at pixel column `x` for a line from x 0.5 to 100.5: the pixel's centre, x + 0.5,
/// is (x + 0.5 - 0.5) / 100 of the way along, rounded to 8 bits.
fn ramp(x: u32) -> u8 { (255.0 * x as f64 / 100.0).round() as u8 }

#[test]
fn foreground_to_background_fills_the_canvas_and_commits_one_undo() {
    // GradientTests.foregroundToBackgroundFillsCanvasAndCommitsOneUndo (:37).
    let (mut e, id, layer) = blank(101, 4);
    let count = depth(&e, id);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, WHITE, 1.0) });
    assert_eq!(depth(&e, id), count + 1);
    for x in [0, 1, 37, 50, 99, 100] {
        let v = ramp(x);
        assert_eq!(pixel(&e, id, x, 1 + x % 3), [v, v, v, 255], "column {x}");
    }
    assert_eq!(pixel(&e, id, 50, 1)[0], 128, "the middle is 128 (127.5 rounds up)");
    e.undo(id).unwrap();
    assert!(e.state(id).unwrap().layers[0].has_pixels == false, "undo leaves the blank layer blank");
}

#[test]
fn radial_spreads_from_the_start_to_the_rim_in_every_direction() {
    // GradientTests.radialSpreadsFromStartToRimInEveryDirection (:55).
    let (mut e, id, layer) = blank(101, 101);
    let g = GradientSpec { shape: GradientShape::Radial, start: p(50.5, 50.5), end: p(90.5, 50.5), from: BLACK, to: WHITE, opacity: 1.0 };
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: g });
    assert_eq!(pixel(&e, id, 50, 50), [0, 0, 0, 255]);
    // 20 px out of a 40 px radius, in four directions: 127.5, rounded up.
    for (x, y) in [(70, 50), (30, 50), (50, 70), (50, 30)] { assert_eq!(pixel(&e, id, x, y), [128, 128, 128, 255], "({x}, {y})"); }
    assert_eq!(pixel(&e, id, 100, 50), [255, 255, 255, 255], "past the rim");
    assert_eq!(pixel(&e, id, 0, 0), [255, 255, 255, 255]);
}

#[test]
fn reverse_and_opacity_follow_the_settings() {
    // GradientTests.reverseOpacityAndDirectionFollowSettings (:69): reversed (white first) at 50 %.
    let (mut e, id, layer) = blank(101, 4);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), WHITE, BLACK, 0.5) });
    // White at half alpha over nothing: premultiplied 127.5, rounded up, on both.
    assert_eq!(pixel(&e, id, 0, 2), [128, 128, 128, 128]);
    assert_eq!(pixel(&e, id, 100, 2), [0, 0, 0, 128]);
}

#[test]
fn foreground_to_transparent_keeps_the_pixels_under_it_and_their_alpha() {
    // GradientTests.foregroundToTransparentPreservesUnderlyingPixelsAndAlpha (:83).
    let (mut e, id, layer) = blank(101, 4);
    let red = [1.0, 0.0, 0.0, 1.0];
    // Everything before the start is the first colour: a line at the far right makes a red layer.
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(100.5, 2.0), p(101.0, 2.0), red, WHITE, 1.0) });
    assert_eq!(pixel(&e, id, 50, 2), [255, 0, 0, 255]);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, [0.0, 0.0, 0.0, 0.0], 1.0) });
    assert_eq!(pixel(&e, id, 0, 2), [0, 0, 0, 255]);
    assert_eq!(pixel(&e, id, 100, 2), [255, 0, 0, 255]);
    // Black at half alpha over red: 255 x 0.5 = 127.5, rounded up; alpha stays whole.
    assert_eq!(pixel(&e, id, 50, 2), [128, 0, 0, 255]);
}

#[test]
fn a_mask_gradient_writes_coverage_inside_the_layer() {
    // GradientTests.maskGradientWritesCoverageInsideLayerBounds (:126): an opaque black layer, a
    // revealing mask, and black (hide) to white (reveal) across it.
    let (mut e, id, layer) = blank(101, 4);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.0, 0.0, 0.0] });
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    let count = depth(&e, id);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, WHITE, 1.0) });
    assert_eq!(depth(&e, id), count + 1);
    for x in [0, 50, 100] { assert_eq!(pixel(&e, id, x, 2)[3], ramp(x), "column {x}"); }
    let mask = e.document(id).unwrap().layers[0].mask.clone().unwrap();
    assert_eq!((mask.pixels.width, mask.pixels.height), (101, 4), "the 1 x 1 mask spread over the layer's grid first");
}

#[test]
fn a_gradient_stays_inside_the_selection() {
    // SelectionEditTests.gradientStaysInsideTheSelection (:64).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 0.0, 0.0, 50.0, 40.0);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 20.0), p(99.5, 20.0), BLACK, WHITE, 1.0) });
    assert_eq!(pixel(&e, id, 10, 20)[3], 255);
    assert_eq!(pixel(&e, id, 75, 20)[3], 0);
    // Trimmed to what the selection let through.
    let state = e.state(id).unwrap();
    assert_eq!((state.layers[0].pixels_width, state.layers[0].pixels_height), (50, 40));
}

#[test]
fn fill_uses_the_colour_inside_the_selection_or_the_whole_layer_without_one() {
    // SelectionEditTests.fillUsesPaletteInsideSelectionOrWholeLayerWithoutOne (:77).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let count = depth(&e, id);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    assert_eq!(depth(&e, id), count + 1);
    assert_eq!(Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }.action_name(), "Fill");
    assert_eq!(pixel(&e, id, 30, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&e, id, 5, 5)[3], 0);
    run(&mut e, id, Command::Deselect);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 1.0, 1.0] });
    assert_eq!(pixel(&e, id, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(&e, id, 30, 20), [255, 255, 255, 255]);
}

#[test]
fn a_mask_fill_hides_only_the_selected_area() {
    // SelectionEditTests.maskFillHidesOnlyTheSelectedArea (:111): the mask palette's foreground is
    // black (hide), its background white (reveal).
    let (mut e, id, layer) = blank(100, 40);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    assert_eq!(Command::Fill { id: layer, mask: true, color: [0.0; 3] }.action_name(), "Fill Mask");
    assert_eq!(pixel(&e, id, 30, 20)[3], 0);
    assert_eq!(pixel(&e, id, 5, 5)[3], 255);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(pixel(&e, id, 30, 20)[3], 255);
}

#[test]
fn an_empty_selection_fills_and_paints_nothing() {
    // SelectionEditTests.emptySelectionEditsNothing (:43).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 40.0), p(0.0, 40.0)], mode: SelectionMode::Subtract, antialiased: true });
    assert!(e.state(id).unwrap().selection.unwrap().empty);
    let (before, count) = (e.document(id).unwrap().clone(), depth(&e, id));
    for c in [Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] },
              Command::Gradient { id: layer, mask: false, gradient: linear(p(0.0, 20.0), p(90.0, 20.0), BLACK, WHITE, 1.0) }] {
        assert_eq!(e.execute(id, c), Err(CommandError::Refused("The selection is empty".into())));
    }
    assert!(e.document(id).unwrap().same_content(&before));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn a_gradient_line_under_half_a_pixel_paints_nothing() {
    // Gradient.swift:32, :84-87: a click without a line leaves nothing.
    let (mut e, id, layer) = blank(100, 40);
    let count = depth(&e, id);
    let short = linear(p(10.0, 10.0), p(10.3, 10.3), BLACK, WHITE, 1.0);
    assert!(matches!(e.execute(id, Command::Gradient { id: layer, mask: false, gradient: short }), Err(CommandError::Argument(_))));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn a_layer_smaller_than_the_canvas_grows_to_it_is_trimmed_and_its_mask_follows_with_white() {
    // A 20 x 10 layer at (30, 15) on 100 x 40, under a mask of its own grid: the left half black.
    let mut e = Engine::new();
    let id = e.new_document(100, 40, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(20, 10, [0, 0, 200, 255].repeat(200)), p(30.0, 15.0));
    let mask: Vec<u8> = (0..10).flat_map(|_| (0..20).map(|x| if x < 10 { 0 } else { 255 })).collect();
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(20, 10, mask), enabled: true, placement: None, linked: None });
    let lid = layer.id;
    doc.active_layer_id = Some(lid);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    // A selection reaching past the layer on every side but the bottom.
    select(&mut e, id, 10.0, 5.0, 80.0, 20.0);
    run(&mut e, id, Command::Fill { id: lid, mask: false, color: [1.0, 0.0, 0.0] });
    let l = e.document(id).unwrap().layers[0].clone();
    // Trimmed to the union of the old pixels (30..50 x 15..25) and the filled area (10..90 x 5..25).
    assert_eq!((l.transform.origin.x, l.transform.origin.y, l.transform.size.width, l.transform.size.height), (10.0, 5.0, 80.0, 20.0));
    let pixels = l.pixels.unwrap();
    assert_eq!((pixels.width, pixels.height), (80, 20));
    assert_eq!(pixels.pixel(0, 0), [255, 0, 0, 255], "filled where it grew");
    let m = l.mask.unwrap().pixels;
    assert_eq!((m.width, m.height), (80, 20), "the mask follows the new grid");
    let at = |x: u32, y: u32| m.bytes()[(y * 80 + x) as usize];
    // The old mask where the layer was (its left half black), white where the layer grew.
    assert_eq!((at(25, 12), at(35, 12), at(5, 12), at(70, 2)), (0, 255, 255, 255));
}

#[test]
fn a_fill_inside_a_selection_on_a_layer_over_the_canvas_is_recorded_as_that_rectangle() {
    let (mut e, id, layer) = blank(100, 40);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.2, 0.4, 0.6] });
    let before = e.state(id).unwrap().layers[0].pixels_revision;
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    // The selection's box, a pixel for its clip and one for sampling on every side.
    assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(PixelRect { x: 18, y: 8, width: 34, height: 24 }));
}
