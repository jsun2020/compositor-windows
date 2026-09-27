//! Delete with a selection and Add Mask from Selection (Phase 4a), ported from Compositor for
//! Mac's SelectionEditTests: deleteClearsSelectedPixelsOrDeletesTheLayerWithoutASelection,
//! clipFollowsScaledLayersAndSoftensEdges, maskButtonAddsWhiteMaskOrHidesTheSelection,
//! layerMenuMasksUseTheSelection, maskFromSelectionLinesUpOnScaledLayers and (adapted: Fill is not
//! in this phase) maskFillHidesOnlyTheSelectedArea, with their sizes, points and expectations.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn corners(x: f64, y: f64, w: f64, h: f64) -> Vec<Point> { vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)] }
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(x, y, w, h), mode: SelectionMode::Replace, antialiased: true });
}
fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Raster { Raster::from_premultiplied(width, height, rgba.repeat((width * height) as usize)) }
/// A `width` x `height` document holding `layer` stretched over the whole canvas.
fn session(width: u32, height: u32, layer: &Raster) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, false).unwrap();
    e.import_image(Some(id), &encode_png(layer, 72.0).unwrap(), "Layer", None).unwrap();
    let lid = e.state(id).unwrap().active_layer_id.unwrap();
    let mut t = e.document(id).unwrap().layer(lid).unwrap().transform;
    t.origin = p(0.0, 0.0);
    t.size = Size { width: width as f64, height: height as f64 };
    run(&mut e, id, Command::SetLayerTransform { id: lid, transform: t });
    (e, id, lid)
}
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] {
    let s = e.state(id).unwrap();
    e.composite(id, Rect { x: 0.0, y: 0.0, width: s.width as f64, height: s.height as f64 }, s.width, s.height).unwrap().pixel(x, y)
}
fn has_selection(e: &Engine, id: Uuid) -> bool { e.state(id).unwrap().selection.is_some() }
fn has_mask(e: &Engine, id: Uuid, layer: Uuid) -> bool { e.document(id).unwrap().layer(layer).unwrap().mask.is_some() }
const RED: [u8; 4] = [255, 0, 0, 255];

/// Standard normal CDF via the Abramowitz-Stegun 7.1.26 erf approximation (max error ~1.5e-7),
/// used to compute the feathered mask's expected values from the Mac's own feather formula
/// (ruling M11) instead of pasting hand-computed numbers.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let (a1, a2, a3, a4, a5, k) = (0.254829592, -0.284496736, 1.421413741, -1.453152027, 1.061405429, 0.3275911);
    let t = 1.0 / (1.0 + k * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}
fn phi(z: f64) -> f64 { 0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2)) }

#[test]
fn delete_clears_the_selected_pixels_as_one_step() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    assert!(matches!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }), Err(CommandError::Refused(_))), "nothing selected: the app deletes the layer instead");
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(Command::ClearSelectedPixels { id: layer, mask: false }.action_name(), "Clear");
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1);
    assert_eq!(pixel(&e, id, 30, 20)[3], 0);
    assert_eq!(pixel(&e, id, 5, 5), RED);
    e.undo(id).unwrap();
    assert_eq!(pixel(&e, id, 30, 20), RED);
}

#[test]
fn the_clear_follows_a_scaled_layer_and_softens_a_slanted_edge() {
    // A 50x20 image stretched 2x over the 100x40 canvas, a triangle selected.
    let (mut e, id, layer) = session(100, 40, &solid(50, 20, [0, 0, 255, 255]));
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Freehand, points: vec![p(0.0, 0.0), p(100.0, 0.0), p(0.0, 40.0)], mode: SelectionMode::Replace, antialiased: true });
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(pixel(&e, id, 10, 10)[3], 0, "inside the triangle: cleared");
    assert_eq!(pixel(&e, id, 90, 35)[3], 255, "outside: untouched");
    let edge: Vec<u8> = (0..100).map(|x| pixel(&e, id, x, ((1.0 - x as f64 / 100.0) * 40.0).min(39.0) as u32)[3]).collect();
    assert!(edge.iter().any(|a| *a > 0 && *a < 255), "along the diagonal some pixels are partly cleared: {edge:?}");
}

#[test]
fn delete_on_a_mask_fills_the_selection_white() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert!(pixel(&e, id, 30, 20)[3] == 0 && pixel(&e, id, 5, 5)[3] == 255, "the selected area hidden");
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(Command::ClearSelectedPixels { id: layer, mask: true }.action_name(), "Fill Mask");
    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "white, the mask's background, reveals again");
    run(&mut e, id, Command::SetMaskEnabled { id: layer, enabled: false });
    assert!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: true }).is_err(), "a disabled mask takes no edit");
}

#[test]
fn add_mask_with_a_selection_hides_the_selection_and_uses_it_up() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1, "one step");
    assert!(!has_selection(&e, id), "the selection is used up");
    assert_eq!(pixel(&e, id, 30, 20)[3], 0, "selected area: black, hidden");
    assert_eq!(pixel(&e, id, 5, 5)[3], 255, "everything else: white, visible");
    e.undo(id).unwrap();
    assert!(!has_mask(&e, id, layer) && has_selection(&e, id));
}

#[test]
fn add_black_mask_with_a_selection_shows_only_the_selection() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: false });
    assert!(!has_selection(&e, id));
    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "selected area: white, visible");
    assert_eq!(pixel(&e, id, 5, 5)[3], 0, "everything else: black, hidden");
}

#[test]
fn a_mask_from_a_selection_lines_up_on_a_scaled_layer() {
    let (mut e, id, layer) = session(100, 100, &solid(50, 50, [0, 0, 255, 255]));
    select(&mut e, id, 0.0, 0.0, 50.0, 50.0);
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().mask.as_ref().unwrap().pixels.width, 50, "the mask uses the layer's pixel grid");
    assert_eq!(pixel(&e, id, 25, 25)[3], 0);
    assert_eq!(pixel(&e, id, 75, 75)[3], 255);
    assert_eq!(pixel(&e, id, 75, 25)[3], 255);
}

#[test]
fn a_mask_from_a_feathered_selection_takes_the_feather() {
    // Feather 4 is a Gaussian of sigma 2 across the edge at x = 20 (global constraint: feather
    // sigma = feather / 2). The pixel centred at x = 18 sits 1.5 px outside the edge, x = 21 sits
    // 1.5 px inside; coverage there is Phi(distance / sigma) and a revealing mask paints
    // 255 * (1 - coverage) through the clip (LayerMask.swift:245-249).
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::FeatherSelection { amount: 4 });
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
    let (outside, inside) = (mask.bytes()[20 * 100 + 18] as f64, mask.bytes()[20 * 100 + 21] as f64);
    let sigma = 2.0;
    let expected_outside = 255.0 * (1.0 - phi(-1.5 / sigma));
    let expected_inside = 255.0 * (1.0 - phi(1.5 / sigma));
    // Tolerance, not measured: the feather's formula truncates the Gaussian kernel at 3 sigma
    // (radius = ceil(sigma * 3)) and renormalizes, which redistributes under 0.3% of the mass (the
    // two-tailed mass beyond 3 sigma), and it convolves discrete pixel samples rather than
    // integrating the continuous Gaussian, whose Poisson-summation error is negligible at sigma =
    // 2 px; the production blur (selection/feather.rs) is that formula within 1 level. Together they
    // bound the deviation from the continuous Phi formula to under 3 of 255 levels; the +-8 in the
    // brief was a hand-computed round number.
    let tol = 3.0;
    assert!((outside - expected_outside).abs() <= tol, "outside {outside}, expected {expected_outside}");
    assert!((inside - expected_inside).abs() <= tol, "inside {inside}, expected {expected_inside}");
}

#[test]
fn an_empty_selection_clears_nothing_but_still_makes_a_plain_mask() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 100.0, 40.0), mode: SelectionMode::Subtract, antialiased: true });
    assert!(matches!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }), Err(CommandError::Refused(m)) if m == compositor_engine::ops::selection::EMPTY_SELECTION));
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert!(!has_selection(&e, id) && pixel(&e, id, 30, 20)[3] == 255, "a plain white mask, the empty selection used up (LayerMask.swift:245)");
    // Ruling I6: an opaque layer composites white regardless of whether a mask exists at all, so
    // the pixel check above alone would pass with the mask creation dropped. Assert the mask was
    // actually made, and that it is the plain white mask the brief describes (LayerMask.swift:245:
    // an empty selection clips everything away, so a revealing mask is entirely white).
    assert!(has_mask(&e, id, layer), "Add Mask from Selection must still create a mask");
    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
    assert!(mask.bytes().iter().all(|&b| b == 255), "the mask is plain white everywhere, not just at the one sampled pixel");
}
