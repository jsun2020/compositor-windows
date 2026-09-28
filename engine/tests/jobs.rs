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

/// The job's input as it crosses to the worker: JSON, and the buffers' bytes.
fn crossed(e: &Engine, id: Uuid, layer: Uuid) -> (JobInput, Option<Raster>, Option<GrayRaster>) {
    let (input, pixels, mask) = e.job_input(id, layer).unwrap();
    let input: JobInput = serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
    let pixels = pixels.map(|r| Raster::from_premultiplied(r.width, r.height, r.bytes().to_vec()));
    let mask = mask.map(|m| GrayRaster::from_bytes(m.width, m.height, m.bytes().to_vec()));
    (input, pixels, mask)
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
        let (input, pixels, mask) = crossed(&there, tid, tlayer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, commands(tlayer)[i].clone()).unwrap();
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
    let (input, pixels, mask) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert!(new_mask.is_none(), "the mask was not touched, so it does not travel back");
    assert_eq!(output.regions.len(), 1);
    e.install_job(id, layer, input.stamp, output.clone(), new_pixels, new_mask).unwrap();
    assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(output.regions[0].1));
}

#[test]
fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
    for change in ["pixels", "transform", "mask"] {
        let (mut e, id, layer) = document();
        let (input, pixels, mask) = crossed(&e, id, layer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
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
    let (input, pixels, mask) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
    run(&mut e, id, Command::RenameLayer { id: layer, name: "Renamed".into() });
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Renamed");
}

#[test]
fn a_histogram_job_equals_the_histogram_in_place() {
    let (e, id, layer) = document();
    let (input, pixels, mask) = crossed(&e, id, layer);
    assert_eq!(run_histogram_job(&input, pixels, mask).unwrap(), e.histogram(id, layer).unwrap());
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
    let (input, pixels, mask) = crossed(&e, id, layer);
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
