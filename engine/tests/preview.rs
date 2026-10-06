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
fn camera_raw_diagnostic_preview_is_never_reused_by_apply(){
    let(mut e,doc,layer)=seeded();let depth=e.state(doc).unwrap().undo_depth;
    let mut settings=adjust::camera_raw::CameraRawSettings::default();settings.exposure=1.0;
    e.set_preview(doc,Some(PreviewRequest::CameraRawView{layer,settings:Box::new(settings.clone()),clipping:1,visualize:-1,sharpen_mask:false,shadow_overlay:false,highlight_overlay:false})).unwrap();
    assert_eq!(middle(&e,doc),[0,0,0,255]);assert_eq!(e.state(doc).unwrap().undo_depth,depth);
    e.execute(doc,Command::ApplyFilter{id:layer,params:FilterParams::CameraRaw{settings:Box::new(settings)}}).unwrap();
    let result=middle(&e,doc);assert!(result[0].abs_diff(176)<=2);assert_eq!(e.state(doc).unwrap().undo_depth,depth+1);
    e.undo(doc).unwrap();assert_eq!(middle(&e,doc),[128,128,128,255]);
}

#[test]
fn camera_raw_sampling_keeps_original_and_prepared_layer_pixels_separate_from_the_composite(){
    let(mut e,doc,layer)=seeded();
    e.execute(doc,Command::AddMask{id:layer,revealing:false}).unwrap();
    let cover=encode_png(&Raster::from_premultiplied(8,8,[255u8,0,0,255].repeat(64)),72.).unwrap();
    e.import_image(Some(doc),&cover,"Cover",Some(Point{x:10.,y:10.})).unwrap();
    let at=Point{x:10.,y:10.};let original=e.sample_camera_raw_color(doc,layer,at,false).unwrap().unwrap();
    assert_eq!(original,[128./255.;3]);assert_eq!(e.sample_color(doc,at).unwrap(),Some([1.,0.,0.]));
    let mut settings=adjust::camera_raw::CameraRawSettings::default();settings.exposure=1.;
    e.set_preview(doc,Some(PreviewRequest::Filter{layer,params:FilterParams::CameraRaw{settings:Box::new(settings)}})).unwrap();
    assert_eq!(e.sample_camera_raw_color(doc,layer,at,false).unwrap(),Some(original));
    let prepared=e.sample_camera_raw_color(doc,layer,at,true).unwrap().unwrap();assert!(prepared[0]>original[0]);assert_eq!(prepared,[prepared[0];3]);
    assert_eq!(e.sample_color(doc,at).unwrap(),Some([1.,0.,0.]));
    for at in [Point{x:-1.,y:10.},Point{x:f64::NAN,y:10.},Point{x:19.,y:19.}]{assert!(e.sample_camera_raw_color(doc,layer,at,true).unwrap().is_none());}
    e.set_preview(doc,None).unwrap();assert_eq!(e.sample_camera_raw_color(doc,layer,at,true).unwrap(),Some(original));
}
#[test]
fn a_preview_shows_through_every_render_path_without_touching_the_document() {
    let (mut e, doc, layer) = seeded();
    let before = middle(&e, doc);
    let (was_modified, could_undo, could_redo, depth) = {
        let s = e.state(doc).unwrap();
        (s.is_modified, s.can_undo, s.can_redo, s.undo_depth)
    };
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    let dirty = e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a.clone() })).unwrap();
    assert_eq!(dirty.layers, vec![layer]);
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "the canvas shows the preview");
    let state = e.state(doc).unwrap();
    let l = state.layers.iter().find(|l| l.id == layer).unwrap();
    assert!(l.pixels_revision > 1_000_000, "a preview revision, so the renderer re-uploads");
    assert!(state.is_modified == was_modified && state.can_undo == could_undo
        && state.can_redo == could_redo && state.undo_depth == depth,
        "a preview records nothing");
    // The stored document still holds the original pixels.
    assert_eq!(e.document(doc).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [128, 128, 128, 255]);
    // Export writes the stored document: the gray, not the previewed black.
    let exported = decode_image(&e.export_png(doc).unwrap()).unwrap().raster;
    assert_eq!(exported.pixel(10, 10), [128, 128, 128, 255], "export ignores the open preview");
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
    // Redo too: with a preview showing, it cancels the preview and redoes nothing yet.
    let mut white = LayerAdjustment::new(AdjustmentKind::Levels);
    white.levels.ranges[0] = LevelRange { output_black: 255.0, ..LevelRange::default() };
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: white })).unwrap();
    assert_eq!(middle(&e, doc), [255, 255, 255, 255], "the preview is showing");
    e.redo(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255], "redo drops the preview rather than redoing past it");
    assert!(e.state(doc).unwrap().can_redo, "the redo step is still waiting");
    e.redo(doc).unwrap();
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "the next redo redoes");
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
    // 2100 is just past the 2048 filter-preview limit and 100 is nowhere near it, so exactly one
    // halving runs (factor 0.5) without allocating a multi-thousand-square raster.
    let mut e = Engine::new();
    let doc = e.new_document(2200, 200, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(2100, 100, [255u8, 255, 255, 255].repeat(2100 * 100)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Wide", Some(Point { x: 1100.0, y: 100.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    let before_state = e.state(doc).unwrap();
    let before = before_state.layers[0].transform;
    let before_pixels_width = before_state.layers[0].pixels_width;
    let requested = FilterParams::GaussianBlur { radius: 6.0 };
    e.set_preview(doc, Some(PreviewRequest::Filter { layer, params: requested.clone() })).unwrap();
    let state = e.state(doc).unwrap();
    let l = &state.layers[0];
    assert!(l.transform.size.width > before.size.width, "the preview shows the grown layer");
    assert!(l.pixels_width <= 2048, "previewed from a reduced copy: {}", l.pixels_width);
    assert!(l.pixels_width < before_pixels_width, "the halving demonstrably happened: {} vs {}", l.pixels_width, before_pixels_width);
    // Pin the parameter scaling: the grow step must use the margin for the HALVED radius (3.0),
    // not the requested one (6.0), scaled back up by 1/factor into document units.
    let factor = 0.5;
    let margin_pixels = requested.scaled(factor).margin().ceil();
    let expected_width = before.size.width + 2.0 * margin_pixels / factor;
    let expected_height = before.size.height + 2.0 * margin_pixels / factor;
    assert!((l.transform.size.width - expected_width).abs() < 0.01,
        "grown width should match the halved-radius margin: got {} want {}", l.transform.size.width, expected_width);
    assert!((l.transform.size.height - expected_height).abs() < 0.01,
        "grown height should match the halved-radius margin: got {} want {}", l.transform.size.height, expected_height);
    // Shape-independent, unlike an aspect-ratio comparison: a grow that shifted off-centre would
    // pass the width/height checks above but still make the layer jump when the panel opened.
    let c0 = before.center();
    let c1 = l.transform.center();
    assert!((c0.x - c1.x).abs() < 1e-6 && (c0.y - c1.y).abs() < 1e-6, "a blur grows the layer in place, keeping its centre");
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

#[test]
fn sample_color_reads_the_stored_document_not_an_open_preview() {
    // The Hue/Saturation eyedroppers need this: chasing their own panel's live preview (a
    // Critical from Task 15's code review) would centre a band on a hue the preview just
    // produced, not the colour actually stored on the layer -- the same reason `histogram` and
    // `sample_layer_color` never read the preview either. `sample_color` once did read through
    // the preview (a special case for a test proving a preview toggle changes the displayed
    // pixels); that test now reads the rendered canvas directly instead, so every sampler in this
    // module agrees on "the stored document" with no special case left to misuse.
    let (mut e, doc, layer) = seeded();
    let at = Point { x: 11.0, y: 11.0 };
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a })).unwrap();
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "the preview is showing");
    let stored = e.sample_color(doc, at).unwrap().unwrap();
    assert!((stored[0] - 128.0 / 255.0).abs() < 0.01, "unaffected by the open preview");
    // The Levels histogram and eyedroppers read the stored pixels too.
    assert_eq!(e.histogram(doc, layer).unwrap()[1][128], 64.0, "the gray, not the previewed black");
    let own = e.sample_layer_color(doc, layer, at).unwrap().unwrap();
    assert!((own[0] - 128.0 / 255.0).abs() < 0.01, "sample_layer_color ignores the open preview: {own:?}");
}

#[test]
fn auto_levels_stretches_a_two_tone_layer_to_its_tones() {
    // Two tones, so there is a real stretch to find: a single-valued histogram makes every Auto
    // return the identity, which a broken or no-op auto_levels would also return.
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let mut data = Vec::new();
    for i in 0..64 { let v = if i % 2 == 0 { 64u8 } else { 192 }; data.extend_from_slice(&[v, v, v, 255]); }
    let bytes = encode_png(&Raster::from_premultiplied(8, 8, data), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Two tones", Some(Point { x: 10.0, y: 10.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    let auto = e.auto_levels(doc, layer, LevelsAuto::Contrast).unwrap();
    assert_eq!((auto.ranges[0].black, auto.ranges[0].white), (64.0, 192.0));
}

/// A distinct colour per pixel, opaque except for a transparent column, so a misplaced or
/// mis-scaled sample reads a visibly different value.
fn pattern(w: u32, h: u32) -> Vec<u8> {
    let mut data = Vec::new();
    for y in 0..h { for x in 0..w {
        if x == 3 { data.extend_from_slice(&[0, 0, 0, 0]); continue; }
        data.extend_from_slice(&[(x * 37 % 256) as u8, (y * 53 % 256) as u8, ((x + y) * 29 % 256) as u8, 255]);
    }}
    encode_png(&Raster::from_premultiplied(w, h, data), 72.0).unwrap()
}

#[test]
fn an_adjustment_layers_eyedropper_reads_one_pixel_of_what_lies_beneath() {
    // The one-pixel composite must agree with the full canvas-size composite of everything beneath
    // the adjustment layer (what it used to read, and what its histogram still reads) at every
    // probe: under a rotated, scaled layer, on its transparent column, off the layer entirely,
    // and never through the opaque layer above.
    let mut e = Engine::new();
    let doc = e.new_document(40, 30, false).unwrap();
    e.import_image(Some(doc), &pattern(16, 12), "Pattern", Some(Point { x: 20.0, y: 15.0 })).unwrap();
    let under = e.state(doc).unwrap().active_layer_id.unwrap();
    let mut t = e.state(doc).unwrap().layers[0].transform;
    t.rotation = 20.0;
    t.size = Size { width: 24.0, height: 18.0 };
    t.origin = Point { x: 8.0, y: 6.0 };
    e.execute(doc, Command::SetLayerTransform { id: under, transform: t }).unwrap();
    e.execute(doc, Command::AddAdjustmentLayer { kind: AdjustmentKind::Levels, seed: 0, shadows: None, highlights: None }).unwrap();
    let a = e.state(doc).unwrap().active_layer_id.unwrap();
    let red = encode_png(&Raster::from_premultiplied(40, 30, [255u8, 0, 0, 255].repeat(40 * 30)), 72.0).unwrap();
    e.import_image(Some(doc), &red, "Above", Some(Point { x: 20.0, y: 15.0 })).unwrap();

    let source = e.adjustment_source(doc, a).unwrap();
    let mut compared = 0;
    for y in 0..30 { for x in 0..40 {
        let at = Point { x: x as f64 + 0.3, y: y as f64 + 0.7 };
        let p = source.pixel(x, y);
        let want = if p[3] == 0 { None } else { Some([0, 1, 2].map(|c| (p[c] as f64 / p[3] as f64).min(1.0))) };
        assert_eq!(e.sample_layer_color(doc, a, at).unwrap(), want, "at {x},{y}");
        compared += want.is_some() as u32;
    }}
    assert!(compared > 100, "the probes cover the layer beneath: {compared}");
    assert_eq!(e.sample_layer_color(doc, a, Point { x: -1.0, y: 5.0 }).unwrap(), None, "off the canvas");
    assert_eq!(e.sample_layer_color(doc, a, Point { x: 40.0, y: 5.0 }).unwrap(), None, "off the canvas");
}
#[test]
fn a_drag_preview_reduces_a_colour_adjustment_and_a_settled_one_does_not() {
    let mut e = Engine::new();
    let doc = e.new_document(1000, 800, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(800, 600, [90u8, 120, 150, 255].repeat(800 * 600)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Photo", Some(Point { x: 500.0, y: 400.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    let pixels = |e: &Engine| { let l = &e.state(doc).unwrap().layers[0]; (l.pixels_width, l.pixels_height, l.pixels_revision) };
    let mut levels = LayerAdjustment::new(AdjustmentKind::Levels);
    levels.levels.ranges[0] = LevelRange { white: 200.0, ..LevelRange::default() };
    // 800 is over the 512 drag limit, so one halving; under the 4096 settled limit, so none.
    e.set_preview(doc, Some(PreviewRequest::DragAdjustment { layer, adjustment: levels.clone() })).unwrap();
    assert_eq!((pixels(&e).0, pixels(&e).1), (400, 300), "a drag previews from a halved copy");
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: levels })).unwrap();
    assert_eq!((pixels(&e).0, pixels(&e).1), (800, 600), "the settled preview is full size, as before the drag cap");
    // Grain is full size dragged or not, so the settled request after a drag keeps its pixels.
    let grain = LayerAdjustment::new(AdjustmentKind::Grain);
    e.set_preview(doc, Some(PreviewRequest::DragAdjustment { layer, adjustment: grain.clone() })).unwrap();
    let dragged = pixels(&e);
    assert_eq!((dragged.0, dragged.1), (800, 600));
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: grain })).unwrap();
    assert_eq!(pixels(&e), dragged, "not recomputed: same pixels, same revision");
}
