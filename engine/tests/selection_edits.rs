//! Edits limited to the selection (Phase 4a), ported from Compositor for Mac's
//! HueSaturationTests.adjustmentStaysInsideTheSelectionAndIsOneUndoStep,
//! LevelsTests.histogramExcludesTransparencyAndWeightsSelection /
//! selectionPreviewCancelCommitUndoAndPersistence and
//! SelectionEditTests.invertKeepsTransparencyStaysInSelectionAndWorksOnMasks /
//! invertIsFastOnLargeImagesAndHandlesUniformMasksWithASelection, with their fixtures, points and
//! tolerances; plus the blur on its grown grid (Filters.swift:399-413) and preview key (N3).
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn corners(x: f64, y: f64, w: f64, h: f64) -> Vec<Point> { vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)] }
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64, antialiased: bool) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(x, y, w, h), mode: SelectionMode::Replace, antialiased });
}
/// A document of the raster's size holding it as its one layer, placed over the whole canvas.
fn session_with(raster: &Raster) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(raster.width, raster.height, false).unwrap();
    e.import_image(Some(id), &encode_png(raster, 72.0).unwrap(), "Layer", None).unwrap();
    let layer = e.state(id).unwrap().active_layer_id.unwrap();
    (e, id, layer)
}
fn canvas(e: &Engine, id: Uuid) -> Raster {
    let s = e.state(id).unwrap();
    e.composite(id, Rect { x: 0.0, y: 0.0, width: s.width as f64, height: s.height as f64 }, s.width, s.height).unwrap()
}
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { canvas(e, id).pixel(x, y) }
fn near(value: [u8; 4], target: [u8; 4], tolerance: i32) -> bool { value.iter().zip(target).all(|(a, b)| (*a as i32 - b as i32).abs() <= tolerance) }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
fn refused(e: &mut Engine, id: Uuid, c: Command) -> bool {
    matches!(e.execute(id, c), Err(CommandError::Refused(m)) if m == compositor_engine::ops::selection::EMPTY_SELECTION)
}
/// `rows` rows of `width` pixels, each `f(x, y)` premultiplied.
fn raster(width: u32, height: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width { data.extend_from_slice(&f(x, y)); } }
    Raster::from_premultiplied(width, height, data)
}
/// HueSaturationTests.makeSession: 40x20, left half red, right half grey 0.5, and rows 16-19 half
/// transparent blue across the width.
fn colors() -> Raster {
    raster(40, 20, |x, y| if y >= 16 { [0, 0, 128, 128] } else if x < 20 { [255, 0, 0, 255] } else { [128, 128, 128, 255] })
}
/// SelectionEditTests.twoColorLayer: 100x40, red left half, blue right half.
fn two_colors() -> Raster { raster(100, 40, |x, _| if x < 50 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }) }
fn hue(degrees: f64) -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    a.hsv_settings = Some(HueSaturationSettings::new(degrees, 0.0, 0.0, false, ColorRange::Master));
    a
}
fn inverting_levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_black: 255.0, output_white: 0.0, ..LevelRange::default() };
    a
}

#[test]
fn a_hue_rotation_stays_inside_the_selection_and_is_one_undo_step() {
    let (mut e, id, layer) = session_with(&colors());
    select(&mut e, id, 0.0, 0.0, 10.0, 20.0, true);
    let before = depth(&e, id);
    run(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: hue(120.0) });
    assert_eq!(depth(&e, id), before + 1, "one undo step");
    assert!(near(pixel(&e, id, 5, 5), [0, 255, 0, 255], 8), "inside: rotated, {:?}", pixel(&e, id, 5, 5));
    assert!(near(pixel(&e, id, 15, 5), [255, 0, 0, 255], 8), "outside: untouched, {:?}", pixel(&e, id, 15, 5));
    e.undo(id).unwrap();
    assert!(near(pixel(&e, id, 5, 5), [255, 0, 0, 255], 8));
}

#[test]
fn the_histogram_excludes_transparency_and_weights_the_selection() {
    let source = raster(3, 1, |x, _| [[255, 0, 0, 255], [0, 128, 0, 128], [0, 0, 0, 0]][x as usize]);
    let (mut e, id, layer) = session_with(&source);
    let bins = e.histogram(id, layer).unwrap();
    assert!(bins[1][255] == 1.0 && (bins[2][255] - 128.0 / 255.0).abs() < 0.00001);
    let total: f64 = bins[0].iter().sum();
    assert!((total - (1.0 + 128.0 / 255.0)).abs() < 0.00001);
    select(&mut e, id, 0.0, 0.0, 1.0, 1.0, false);
    let selected = e.histogram(id, layer).unwrap();
    assert!(selected[1][255] == 1.0 && selected[2][255] == 0.0, "only the selected red pixel counts");
    // An adjustment layer never takes the selection: its histogram reads everything beneath it.
    run(&mut e, id, Command::AddAdjustmentLayer { kind: AdjustmentKind::Levels, seed: 0, shadows: None, highlights: None });
    let adjustment = e.state(id).unwrap().active_layer_id.unwrap();
    let beneath = e.histogram(id, adjustment).unwrap();
    assert!((beneath[2][255] - 128.0 / 255.0).abs() < 0.00001, "the green pixel counts beneath an adjustment layer");
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 3.0, 1.0), mode: SelectionMode::Subtract, antialiased: false });
    assert!(e.document(id).unwrap().selection.as_ref().unwrap().is_empty());
    assert!(e.histogram(id, layer).unwrap().iter().flatten().all(|v| *v == 0.0), "an empty selection counts nothing");
}

#[test]
fn a_levels_preview_and_commit_stay_in_the_selection_through_undo_and_saving() {
    let ramp = raster(6, 1, |x, _| [[0, 0, 0, 255], [64, 64, 64, 255], [128, 128, 128, 255], [255, 255, 255, 255], [64, 32, 0, 128], [0, 0, 0, 0]][x as usize]);
    let (mut e, id, layer) = session_with(&ramp);
    select(&mut e, id, 0.0, 0.0, 2.0, 1.0, true);
    let before = depth(&e, id);
    e.set_preview(id, Some(PreviewRequest::Adjustment { layer, adjustment: inverting_levels() })).unwrap();
    let preview = e.layer_raster(id, layer, 0).unwrap().unwrap();
    assert_eq!((preview.bytes()[0], preview.bytes()[8]), (255, 128), "inside inverted, outside untouched");
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap(), &ramp, "the document is untouched");
    e.set_preview(id, None).unwrap();
    assert_eq!(depth(&e, id), before);
    run(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: inverting_levels() });
    assert_eq!(depth(&e, id), before + 1);
    let committed = e.document(id).unwrap().layer(layer).unwrap().pixels.clone().unwrap();
    assert_eq!(committed, preview, "the commit is the preview");
    e.undo(id).unwrap();
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap(), &ramp);
    e.redo(id).unwrap();
    let package = e.save_package(id).unwrap();
    let reopened = e.open_package(&package, None).unwrap();
    assert!(e.state(reopened).unwrap().selection.is_none(), "a selection is never saved");
    assert_eq!(canvas(&e, reopened), canvas(&e, id), "the pixels are");
}

#[test]
fn invert_stays_in_the_selection_and_works_on_masks() {
    let (mut e, id, layer) = session_with(&two_colors());
    select(&mut e, id, 0.0, 0.0, 100.0, 20.0, true);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert_eq!(pixel(&e, id, 10, 5), [0, 255, 255, 255], "red to cyan");
    assert_eq!(pixel(&e, id, 90, 5), [255, 255, 0, 255], "blue to yellow");
    assert_eq!(pixel(&e, id, 10, 30), [255, 0, 0, 255], "outside the selection: unchanged");
    run(&mut e, id, Command::Deselect);
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert_eq!(pixel(&e, id, 50, 30)[3], 0, "an all-white mask inverts to hide everything");
}

#[test]
fn a_uniform_mask_inverts_only_the_selected_part() {
    let (mut e, id, layer) = session_with(&raster(100, 40, |_, _| [255, 0, 0, 255]));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 0.0, 0.0, 30.0, 40.0, true);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert!(pixel(&e, id, 10, 20)[3] == 0 && pixel(&e, id, 80, 20)[3] == 255);
    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap();
    assert_eq!((mask.pixels.width, mask.pixels.height), (100, 40), "the 1x1 mask took the layer's pixel grid");
    // Layer > Mask > Invert Mask is the same edit.
    run(&mut e, id, Command::InvertMask { id: layer });
    assert!(pixel(&e, id, 10, 20)[3] == 255 && pixel(&e, id, 80, 20)[3] == 255);
}

#[test]
fn a_blur_spreads_only_where_the_selection_reaches() {
    // A 20x20 opaque red block over x 20..40 of a 60x20 canvas; the selection covers x 0..30.
    let mut e = Engine::new();
    let id = e.new_document(60, 20, false).unwrap();
    let block = raster(20, 20, |_, _| [255, 0, 0, 255]);
    e.import_image(Some(id), &encode_png(&block, 72.0).unwrap(), "Block", Some(p(30.0, 10.0))).unwrap();
    let layer = e.state(id).unwrap().active_layer_id.unwrap();
    select(&mut e, id, 0.0, 0.0, 30.0, 20.0, true);
    let blur = FilterParams::GaussianBlur { radius: 3.0 };
    e.set_preview(id, Some(PreviewRequest::Filter { layer, params: blur.clone() })).unwrap();
    let previewed = canvas(&e, id);
    run(&mut e, id, Command::ApplyFilter { id: layer, params: blur });
    let t = e.document(id).unwrap().layer(layer).unwrap().transform;
    assert!(t.origin.x < 20.0, "the blur spread left, into the selection: {t:?}");
    assert_eq!(t.origin.x + t.size.width, 40.0, "and not right, outside it: {t:?}");
    assert_eq!(pixel(&e, id, 38, 10), [255, 0, 0, 255], "the unselected edge stays hard");
    assert!(pixel(&e, id, 20, 10)[3] < 255 && pixel(&e, id, 19, 10)[3] > 0, "the selected edge is soft");
    assert_eq!(canvas(&e, id), previewed, "the preview blended on the same grown grid");
}

#[test]
fn an_empty_selection_refuses_every_edit() {
    let (mut e, id, layer) = session_with(&two_colors());
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0, true);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 100.0, 40.0), mode: SelectionMode::Subtract, antialiased: true });
    let (before, shown) = (depth(&e, id), canvas(&e, id));
    assert!(refused(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: hue(120.0) }));
    assert!(refused(&mut e, id, Command::InvertPixels { id: layer, mask: false }));
    assert!(refused(&mut e, id, Command::InvertPixels { id: layer, mask: true }));
    assert!(refused(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 2.0 } }));
    e.set_preview(id, Some(PreviewRequest::Adjustment { layer, adjustment: hue(120.0) })).unwrap();
    assert_eq!((depth(&e, id), canvas(&e, id)), (before, shown), "nothing changed, nothing previewed");
}

#[test]
fn a_kept_preview_must_have_been_made_from_the_same_pixels_and_selection() {
    let mut doc = Document::new(8, 8);
    let layer = Layer::with_pixels("Grey", raster(8, 8, |_, _| [128, 128, 128, 255]), p(0.0, 0.0));
    let lid = layer.id;
    doc.layers.push(layer);
    let request = PreviewRequest::Adjustment { layer: lid, adjustment: inverting_levels() };
    let preview = compute_preview(&doc, &request, 1).unwrap();
    assert!(preview.answers(&request, &PreviewSource::of(&doc, lid)));
    let mut edited = doc.clone();
    edited.layer_mut(lid).unwrap().set_pixels(Some(raster(8, 8, |_, _| [0, 0, 0, 255])));
    assert!(!preview.answers(&request, &PreviewSource::of(&edited, lid)), "other pixels: recompute");
    let mut selected = doc.clone();
    selected.selection_revision += 1;
    assert!(!preview.answers(&request, &PreviewSource::of(&selected, lid)), "another selection: recompute");
}
