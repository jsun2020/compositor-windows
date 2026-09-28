//! Jobs (Phase 4b-1): a layer's edit, histogram or effects image made by a second engine from the
//! job's input alone, equal to the same work done in place; an edit put back only onto the layer it
//! was taken from.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Unequal pixels with varied alpha: no two neighbours alike.
fn pattern(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let v = (x.wrapping_mul(73) ^ y.wrapping_mul(151)).wrapping_mul(2654435761);
        let a = (v >> 24) as u8 | 0x40;
        data.extend_from_slice(&[((v >> 8) as u8).min(a), ((v >> 16) as u8).min(a), (v as u8).min(a), a]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// A 160 x 110 canvas; a 90 x 60 layer of `pattern` at (23, 17) whose mask hides it but for an
/// ellipse over part of it; that ellipse, feathered, selected.
fn document() -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(160, 110, false).unwrap();
    let png = encode_png(&pattern(90, 60), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png, "Pattern", Some(p(68.0, 47.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(30.0, 20.0), p(95.0, 20.0), p(95.0, 70.0), p(30.0, 70.0)], mode: SelectionMode::Replace, antialiased: true });
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    run(&mut e, id, Command::FeatherSelection { amount: 4 });
    (e, id, layer)
}

/// The job's input as it crosses to the worker: JSON, and the buffers' bytes - the pixels and mask as
/// the wasm bridge's `job_buffer_ptr` hands them out, the selection's points as `job_points_ptr` does
/// (a flat i32 buffer, never JSON: `JobSelection::flatten`).
fn crossed(e: &Engine, id: Uuid, layer: Uuid) -> (JobInput, Option<Raster>, Option<GrayRaster>, Option<Vec<i32>>) {
    let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
    let input: JobInput = serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
    let pixels = pixels.map(|r| Raster::from_premultiplied(r.width, r.height, r.bytes().to_vec()));
    let mask = mask.map(|m| GrayRaster::from_bytes(m.width, m.height, m.bytes().to_vec()));
    (input, pixels, mask, points)
}

fn levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { black: 20.0, gamma: 1.4, white: 230.0, output_black: 10.0, output_white: 240.0 };
    a
}

#[test]
fn an_edit_made_by_a_job_and_put_back_equals_the_edit_made_in_place() {
    let commands = |layer: Uuid| vec![
        Command::ApplyAdjustment { id: layer, adjustment: levels() },
        // A blur spreads past the layer: its grid grows, and the covering mask with it.
        Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 6.0 } },
        Command::InvertPixels { id: layer, mask: true },
    ];
    for i in 0..3 {
        // The same document twice: one edited in place, one through a job.
        let (mut here, id, layer) = document();
        let (mut there, tid, tlayer) = document();
        let depth = there.state(tid).unwrap().undo_depth;
        run(&mut here, id, commands(layer)[i].clone());
        let (input, pixels, mask, points) = crossed(&there, tid, tlayer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), commands(tlayer)[i].clone()).unwrap();
        there.install_job(tid, tlayer, input.stamp, output, new_pixels, new_mask).unwrap();
        let (a, b) = (&here.document(id).unwrap().layers[0], &there.document(tid).unwrap().layers[0]);
        assert_eq!(a.pixels.as_ref().unwrap().bytes(), b.pixels.as_ref().unwrap().bytes(), "command {i}: pixels");
        assert_eq!(a.transform, b.transform, "command {i}: transform");
        assert_eq!(a.mask.as_ref().unwrap().pixels.bytes(), b.mask.as_ref().unwrap().pixels.bytes(), "command {i}: mask");
        assert_eq!(there.state(tid).unwrap().undo_depth, depth + 1, "command {i}: one step");
    }
}

#[test]
fn a_job_inside_a_selection_brings_its_changed_rectangle_back() {
    let (mut e, id, layer) = document();
    let before = e.state(id).unwrap().layers[0].pixels_revision;
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert!(new_mask.is_none(), "the mask was not touched, so it does not travel back");
    assert_eq!(output.regions.len(), 1);
    e.install_job(id, layer, input.stamp, output.clone(), new_pixels, new_mask).unwrap();
    assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(output.regions[0].1));
}

#[test]
fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
    for change in ["pixels", "transform", "mask"] {
        let (mut e, id, layer) = document();
        let (input, pixels, mask, points) = crossed(&e, id, layer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
        match change {
            "pixels" => run(&mut e, id, Command::InvertPixels { id: layer, mask: false }),
            "transform" => run(&mut e, id, Command::NudgeLayers { ids: vec![layer], dx: 1.0, dy: 0.0 }),
            _ => run(&mut e, id, Command::FillMask { id: layer, white: true }),
        }
        let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
        let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
        assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())), "{change}");
        assert!(e.document(id).unwrap().same_content(&doc), "{change}: untouched");
        assert_eq!(e.state(id).unwrap().undo_depth, depth, "{change}: nothing recorded");
    }
    // A change that leaves the layer's pixels, mask and place alone does not stop it.
    let (mut e, id, layer) = document();
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
    run(&mut e, id, Command::RenameLayer { id: layer, name: "Renamed".into() });
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Renamed");
}

/// Controller fix round 1, finding 2: the stamp also covers the canvas's size and the selection's
/// revision, not only the layer, so a change to either between the job's input and its install is
/// caught too.
#[test]
fn a_selection_change_between_job_input_and_install_refuses_the_result() {
    let (mut e, id, layer) = document();
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
    // Nothing about the layer itself changes: only the selection a clipped job was computed against.
    run(&mut e, id, Command::Deselect);
    let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
    assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())));
    assert!(e.document(id).unwrap().same_content(&doc), "untouched");
    assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
}

/// Controller fix round 1, finding 2: a Canvas Size anchored top-left leaves this layer's transform,
/// pixels and mask exactly as they were (the offset is (0, 0)) and the selection alone (it is already
/// none here), yet the canvas grows - the stamp must catch that too.
#[test]
fn a_top_left_canvas_size_between_job_input_and_install_refuses_the_result() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::Deselect); // isolate the canvas size: no selection change either
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
    let before = e.document(id).unwrap().layers[0].clone();
    run(&mut e, id, Command::CanvasSize { width: 200, height: 150, anchor: 0, fill: None });
    let after = e.document(id).unwrap().layers[0].clone();
    assert_eq!(before.transform, after.transform, "a top-left anchor leaves this layer's transform alone");
    assert_eq!(before.pixels_revision, after.pixels_revision, "and its pixels");
    let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
    assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())));
    assert!(e.document(id).unwrap().same_content(&doc), "untouched");
    assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
}

/// Controller fix round 1, finding 1: a transform with a decimal serde_json's parser cannot
/// round-trip exactly without the `float_roundtrip` feature (measured: "911.6760726776201" comes
/// back as 911.67607267762) must still survive the stamp's and the output's JSON exactly as the wasm
/// bridge sends them - `prepare_job`'s `input_json` to the worker, then `install_job`'s `stamp_json`
/// and `output_json` back - and install onto the unmoved layer bit-identically.
#[test]
fn a_fractional_transform_survives_the_json_the_wasm_bridge_uses_for_install() {
    let mut e = Engine::new();
    let id = e.new_document(160, 110, false).unwrap();
    // `import_image` places by its raster's center, floored to a whole pixel, so it cannot land a
    // fractional origin directly: a 2 x 2 raster centered at (1, 1) floors to origin (0, 0) exactly,
    // then a nudge from exactly zero by the target values lands them bit-identically (adding to 0.0
    // rounds nothing).
    let png = encode_png(&pattern(2, 2), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png, "Pattern", Some(p(1.0, 1.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    assert_eq!(e.document(id).unwrap().layers[0].transform.origin, p(0.0, 0.0), "the fixture starts at exactly zero");
    let origin = p(911.6760726776201, 47.0);
    run(&mut e, id, Command::NudgeLayers { ids: vec![layer], dx: origin.x, dy: origin.y });
    assert_eq!(e.document(id).unwrap().layers[0].transform.origin, origin, "the fixture itself holds the exact literal");

    // `prepare_job`'s JSON (JobInput, including the stamp).
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    assert_eq!(input.stamp.transform.origin, origin, "the stamp survives prepare_job's JSON round trip");

    // The worker's edit (one that does not move the layer) and its own JSON (JobOutput).
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }).unwrap();
    let output: JobOutput = serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();
    assert_eq!(output.transform.origin, origin, "an edit that does not move the layer reports the same origin, exactly");

    // `install_job`'s own JSON of the stamp it is handed back (`stamp_json`).
    let stamp: LayerStamp = serde_json::from_str(&serde_json::to_string(&input.stamp).unwrap()).unwrap();
    e.install_job(id, layer, stamp, output, new_pixels, new_mask).unwrap();
    assert_eq!(e.document(id).unwrap().layers[0].transform.origin, origin, "install keeps the exact origin");
}

#[test]
fn a_histogram_job_equals_the_histogram_in_place() {
    let (e, id, layer) = document();
    let (input, pixels, mask, points) = crossed(&e, id, layer);
    assert_eq!(run_histogram_job(&input, pixels, mask, points.as_deref()).unwrap(), e.histogram(id, layer).unwrap());
}

/// The 90 x 60 `pattern` layer at (23, 17) on 160 x 110 with a stroke and a drop shadow, made in the
/// engine it is edited in (a document moved from another engine would bring that engine's revisions).
fn styled() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(160, 110);
    let mut layer = Layer::with_pixels("Styled", pattern(90, 60), p(23.0, 17.0));
    layer.extra.effects = Some(serde_json::from_value(serde_json::json!({
        "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 6 },
        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 } })).unwrap());
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let handle = e.insert_document(doc);
    (e, handle, id)
}

/// The key the job's own `EffectsDraw` carries (`effects-images.ts`'s `bytesKey`), what
/// `keep_effects_image` now compares against `effects_draw(l, edit)` freshly, instead of the whole
/// stamp (fix round 1, issue 2).
fn key_of(e: &Engine, id: Uuid, layer: Uuid) -> String {
    effects_draw(e.document(id).unwrap().layer(layer).unwrap(), None).unwrap().key
}

#[test]
fn a_full_size_effects_image_a_job_made_is_found_as_if_the_engine_had_made_it() {
    let (e, id, layer) = styled();
    assert!(!e.has_effects_image(id, layer, None).unwrap());
    let (input, pixels, mask, _points) = crossed(&e, id, layer);
    let key = key_of(&e, id, layer);
    let (_, image) = run_effects_job(&input, pixels.unwrap(), mask, 1.0, None).unwrap().unwrap();
    let bytes = image.bytes().to_vec();
    assert!(e.keep_effects_image(id, layer, input.stamp, &key, None, image).unwrap());
    assert!(e.has_effects_image(id, layer, None).unwrap());
    let made = e.effects_cache().made();
    let drawn = e.draw_raster(id, layer, 0, None).unwrap().unwrap();
    assert_eq!(e.effects_cache().made(), made, "found, not made again");
    assert_eq!(drawn.bytes(), bytes.as_slice());
}

#[test]
fn an_effects_image_is_not_kept_for_a_layer_that_changed_or_at_another_size() {
    let (mut e, id, layer) = styled();
    let (input, pixels, mask, _points) = crossed(&e, id, layer);
    let key = key_of(&e, id, layer);
    let (_, image) = run_effects_job(&input, pixels.clone().unwrap(), mask.clone(), 1.0, None).unwrap().unwrap();
    let (_, small) = run_effects_job(&input, pixels.unwrap().halved(), mask, 0.5, None).unwrap().unwrap();
    assert!(!e.keep_effects_image(id, layer, input.stamp, &key, None, small).unwrap(), "a reduced image is not the full one");
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert!(!e.keep_effects_image(id, layer, input.stamp, &key, None, image).unwrap(), "the pixels changed");
    assert!(!e.has_effects_image(id, layer, None).unwrap());
}

/// Fix round 1, issue 2: `keep_effects_image` used to compare the whole `LayerStamp` (transform,
/// canvas size, selection revision too), none of which the effects image depends on, so a move or a
/// new selection made while the job ran refused a perfectly good result; only `pixels_revision`,
/// `mask_revision` and the job's own `EffectsDraw` key are compared now.
#[test]
fn an_effects_image_is_kept_despite_a_move_or_a_new_selection_between_ask_and_keep() {
    let (mut e, id, layer) = styled();
    let (input, pixels, mask, _points) = crossed(&e, id, layer);
    let key = key_of(&e, id, layer);
    let (_, image) = run_effects_job(&input, pixels.unwrap(), mask, 1.0, None).unwrap().unwrap();
    // A move (the transform; not part of the new comparison) and a new selection (the selection
    // revision; not part of it either) made after the job took its input.
    run(&mut e, id, Command::NudgeLayers { ids: vec![layer], dx: 3.0, dy: -2.0 });
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)], mode: SelectionMode::Replace, antialiased: false });
    assert!(e.keep_effects_image(id, layer, input.stamp, &key, None, image).unwrap(), "a move and a new selection do not touch the effects image");
    assert!(e.has_effects_image(id, layer, None).unwrap());
}

/// Fix round 1, issue 2: an effects change between ask and keep is refused even when it happens to
/// leave the padding (`inset`) exactly as it was, which the old width/height check alone would miss.
#[test]
fn an_effects_image_is_refused_when_the_effects_changed_with_the_same_inset() {
    let (e, id, layer) = styled();
    let (input, pixels, mask, _points) = crossed(&e, id, layer);
    let key = key_of(&e, id, layer);
    let (_, image) = run_effects_job(&input, pixels.unwrap(), mask, 1.0, None).unwrap().unwrap();
    // A different stroke colour, the same stroke size (6): the margin, and so the image's own
    // dimensions, stay exactly as they were.
    let mut doc = e.document(id).unwrap().clone();
    doc.layers[0].extra.effects = Some(serde_json::from_value(serde_json::json!({
        "stroke": { "blue": 0.9, "green": 0.1, "inside": false, "opacity": 0.8, "red": 0.1, "size": 6 },
        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 } })).unwrap());
    let mut e2 = Engine::new();
    let id2 = e2.insert_document(doc);
    assert!(!e2.keep_effects_image(id2, layer, input.stamp, &key, None, image).unwrap(), "the effects changed with the same inset");
    assert!(!e2.has_effects_image(id2, layer, None).unwrap());
}

#[test]
fn an_effects_job_at_full_size_equals_the_engines_own_image_and_a_reduced_one_scales_its_effects() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::Deselect);
    let effects: LayerEffects = serde_json::from_value(serde_json::json!({
        "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 6 },
        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 } })).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers[0].extra.effects = Some(effects.clone());
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    let (input, pixels, mask, _points) = crossed(&e, id, layer);
    let (image, raster) = run_effects_job(&input, pixels.clone().unwrap(), mask.clone(), 1.0, None).unwrap().unwrap();
    let own = e.draw_raster(id, layer, 0, None).unwrap().unwrap();
    assert_eq!(raster.bytes(), own.bytes(), "the job's image is the engine's image");
    assert_eq!((image.width, image.height, image.inset), (own.width, own.height, effects.margin()));
    // Halved twice: a quarter of the pixels and effects a quarter the size.
    let quarter = pixels.unwrap().halved().halved();
    let (small, _) = run_effects_job(&input, quarter.clone(), mask, 0.25, None).unwrap().unwrap();
    let inset = effects.scaled(0.25).margin();
    assert!(inset < effects.margin());
    assert_eq!((small.width, small.height, small.inset), (quarter.width + 2 * inset, quarter.height + 2 * inset, inset));
}
