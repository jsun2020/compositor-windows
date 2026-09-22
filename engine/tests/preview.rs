use compositor_engine::*;

fn seeded() -> (Engine, uuid::Uuid, uuid::Uuid) {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(8, 8, [128u8, 128, 128, 255].repeat(64)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Gray", Some(Point { x: 10.0, y: 10.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    (e, doc, layer)
}
fn middle(e: &Engine, doc: uuid::Uuid) -> [u8; 4] {
    e.composite_edit(doc, None, Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 }, 20, 20).unwrap().pixel(10, 10)
}

#[test]
fn a_preview_shows_through_every_render_path_without_touching_the_document() {
    let (mut e, doc, layer) = seeded();
    let before = middle(&e, doc);
    let (was_modified, could_undo, could_redo) = {
        let s = e.state(doc).unwrap();
        (s.is_modified, s.can_undo, s.can_redo)
    };
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    let dirty = e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a.clone() })).unwrap();
    assert_eq!(dirty.layers, vec![layer]);
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "the canvas shows the preview");
    let state = e.state(doc).unwrap();
    let l = state.layers.iter().find(|l| l.id == layer).unwrap();
    assert!(l.pixels_revision > 1_000_000, "a preview revision, so the renderer re-uploads");
    assert!(state.is_modified == was_modified && state.can_undo == could_undo && state.can_redo == could_redo,
        "a preview records nothing");
    // The stored document still holds the original pixels.
    assert_eq!(e.document(doc).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [128, 128, 128, 255]);
    assert_eq!(e.export_png(doc).map(|b| b.len() > 0).unwrap(), true);
    e.set_preview(doc, None).unwrap();
    assert_eq!(middle(&e, doc), before, "clearing the preview restores the canvas");
}

#[test]
fn a_command_undo_and_redo_all_end_a_preview() {
    let (mut e, doc, layer) = seeded();
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a.clone() })).unwrap();
    e.execute(doc, Command::ApplyAdjustment { id: layer, adjustment: a.clone() }).unwrap();
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "committed for real");
    assert!(e.state(doc).unwrap().can_undo);
    assert_eq!(e.document(doc).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255]);
    e.undo(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255]);
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a })).unwrap();
    e.undo(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255], "undo drops the preview rather than leaving it stranded");
}

#[test]
fn revert_drops_the_preview_and_still_reverts() {
    // A preview must not turn an explicit revert into a no-op.
    let (mut e, doc, layer) = seeded();
    e.mark_saved(doc, None);
    e.execute(doc, Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert_eq!(middle(&e, doc), [127, 127, 127, 255], "committed for real");
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a })).unwrap();
    e.revert(doc).unwrap();
    let state = e.state(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255], "the preview and the committed invert are both gone");
    assert!(!state.is_modified, "back to the saved state");
    assert!(state.can_undo, "the import that seeded the document is still there to undo");
}

#[test]
fn a_blur_preview_grows_the_layer_and_previews_from_a_reduced_copy() {
    let mut e = Engine::new();
    let doc = e.new_document(400, 400, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(300, 300, [255u8, 255, 255, 255].repeat(90_000)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Big", Some(Point { x: 200.0, y: 200.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    let before = e.state(doc).unwrap().layers[0].transform;
    e.set_preview(doc, Some(PreviewRequest::Filter { layer, params: FilterParams::GaussianBlur { radius: 6.0 } })).unwrap();
    let state = e.state(doc).unwrap();
    let l = &state.layers[0];
    assert!(l.transform.size.width > before.size.width, "the preview shows the grown layer");
    assert!(l.pixels_width <= 2048, "previewed from a reduced copy: {}", l.pixels_width);
    assert!((l.transform.size.width / before.size.width - l.transform.size.height / before.size.height).abs() < 0.01);
    e.set_preview(doc, None).unwrap();
    assert_eq!(e.state(doc).unwrap().layers[0].transform, before);
}

#[test]
fn commands_apply_adjustments_filters_invert_and_adjustment_layers() {
    let (mut e, doc, layer) = seeded();
    e.execute(doc, Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert_eq!(middle(&e, doc), [127, 127, 127, 255]);
    e.execute(doc, Command::ApplyFilter { id: layer, params: FilterParams::AddNoise { amount: 40.0, gaussian: false, monochromatic: true, seed: 3 } }).unwrap();
    assert_ne!(middle(&e, doc), [127, 127, 127, 255]);
    let new = e.execute(doc, Command::AddAdjustmentLayer { kind: AdjustmentKind::Curves, seed: 0, shadows: None, highlights: None }).unwrap();
    assert!(new.structure);
    let state = e.state(doc).unwrap();
    let adjustment_layer = state.layers.iter().find(|l| l.adjustment.is_some()).unwrap();
    assert_eq!(adjustment_layer.adjustment.as_ref().unwrap().kind, AdjustmentKind::Curves);
    assert!(!adjustment_layer.has_pixels);
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains(r#""adjustment":{"#) && json.contains(r#""kind":"Curves""#));
    let mut edited = LayerAdjustment::new(AdjustmentKind::Curves);
    edited.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    e.execute(doc, Command::SetAdjustment { id: adjustment_layer.id, adjustment: edited }).unwrap();
    assert_eq!(middle(&e, doc), [255, 255, 255, 255]);
}

#[test]
fn every_new_command_names_its_own_undo_step() {
    let names = [
        (Command::ApplyAdjustment { id: uuid::Uuid::nil(), adjustment: LayerAdjustment::new(AdjustmentKind::Hsv) }, "Hue/Saturation"),
        (Command::InvertPixels { id: uuid::Uuid::nil(), mask: false }, "Invert"),
        (Command::InvertPixels { id: uuid::Uuid::nil(), mask: true }, "Invert Mask"),
        (Command::ApplyFilter { id: uuid::Uuid::nil(), params: FilterParams::MotionBlur { angle: 0.0, distance: 10.0 } }, "Motion Blur"),
        (Command::AddAdjustmentLayer { kind: AdjustmentKind::Grain, seed: 0, shadows: None, highlights: None }, "New Grain Adjustment"),
        (Command::SetAdjustment { id: uuid::Uuid::nil(), adjustment: LayerAdjustment::new(AdjustmentKind::Levels) }, "Edit Levels Adjustment"),
    ];
    for (command, name) in names { assert_eq!(command.action_name(), name); }
    let json = serde_json::to_string(&Command::ApplyFilter { id: uuid::Uuid::nil(), params: FilterParams::LensCorrection { distortion: -20.0 } }).unwrap();
    assert!(json.contains(r#""type":"ApplyFilter""#) && json.contains(r#""filter":"LensCorrection""#), "{json}");
    assert!(serde_json::from_str::<Command>(&json).is_ok());
}

#[test]
fn histograms_auto_levels_and_the_eyedroppers_read_the_right_pixels() {
    let (mut e, doc, layer) = seeded();
    let bins = e.histogram(doc, layer).unwrap();
    assert_eq!(bins.len(), 4);
    assert_eq!(bins[1][128], 64.0, "every pixel of the 8x8 gray layer");
    let auto = e.auto_levels(doc, layer, LevelsAuto::Contrast).unwrap();
    assert!(auto.ranges[0].black <= 128.0 && auto.ranges[0].white >= 128.0);
    let sampled = e.sample_layer_color(doc, layer, Point { x: 11.0, y: 11.0 }).unwrap().unwrap();
    assert!((sampled[0] - 128.0 / 255.0).abs() < 0.01);
    assert!(e.sample_layer_color(doc, layer, Point { x: 1.0, y: 1.0 }).unwrap().is_none(), "outside the layer");
    let calibrated = e.levels_sampling(doc, layer, &LevelsSettings::default(), Point { x: 11.0, y: 11.0 }, LevelsSample::Gray).unwrap();
    assert!((calibrated.apply(128.0 / 255.0, LevelsChannel::Red) - 0.5).abs() < 0.01, "the sampled tone becomes mid gray");
    assert!(e.sample_color(doc, Point { x: 11.0, y: 11.0 }).unwrap().is_some());
    assert!(e.sample_color(doc, Point { x: 1.0, y: 1.0 }).unwrap().is_none(), "nothing there to sample");
    // An adjustment layer's histogram reads the composite beneath it, not its own (absent) pixels.
    let a = e.execute(doc, Command::AddAdjustmentLayer { kind: AdjustmentKind::Levels, seed: 0, shadows: None, highlights: None }).map(|_| ()).and_then(|_| {
        Ok(e.state(doc).unwrap().layers.iter().find(|l| l.adjustment.is_some()).unwrap().id)
    }).unwrap();
    let under = e.histogram(doc, a).unwrap();
    assert_eq!(under[1][128], 64.0, "the gray layer underneath");
    assert!(e.adjustment_source(doc, a).unwrap().width == 20);
}
