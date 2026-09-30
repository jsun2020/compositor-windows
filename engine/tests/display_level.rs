//! F1 (Phase 4.5): the job worker halves an edit's whole result to the level the canvas draws it at
//! and returns it with the result; the installed pixels adopt it, so the UI thread never halves a
//! large result itself (phase4b1-rulings-and-open-items.md, "Open items", F1). Every expected
//! halving is made here from a fresh copy of the bytes, which has no halvings of its own.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Colour and alpha changing every pixel in both directions, so a misplaced or reused halving shows.
fn pattern(width: u32, height: u32) -> Raster {
    let data = (0..height).flat_map(|y| (0..width).flat_map(move |x| {
        let a = 60 + ((x * 7 + y * 3) % 196);
        [((x * 13 + y) % 256).min(a), ((x + y * 11) % 256).min(a), ((x * 5 + y * 9) % 256).min(a), a].map(|v| v as u8)
    })).collect();
    Raster::from_premultiplied(width, height, data)
}

/// `raster` halved `level` times from a fresh copy of its bytes: halvings from scratch, sharing
/// nothing with `raster`'s own.
fn from_scratch(raster: &Raster, level: u32) -> Raster {
    Raster::from_premultiplied(raster.width, raster.height, raster.bytes().to_vec()).reduced(level)
}

/// An odd-sized 1001 x 603 layer on its own canvas, active.
fn document() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(1001, 603);
    let layer = Layer::with_pixels("Pattern", pattern(1001, 603), p(0.0, 0.0));
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let handle = e.insert_document(doc);
    (e, handle, id)
}

/// Levels that changes every channel.
fn levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0].output_white = 200.0;
    a
}

/// The bytes of `raster` in a buffer of their own, as they cross from the worker (the wasm bridge's
/// `raster_of`): no halving made in the worker comes with them.
fn crossed(raster: Option<Raster>) -> Option<Raster> { raster.map(|r| Raster::from_premultiplied(r.width, r.height, r.bytes().to_vec())) }

/// The job for `command` on `layer`, run at `out_per_doc`, its buffers crossed back as bytes.
fn job(e: &Engine, id: Uuid, layer: Uuid, command: Command, out_per_doc: f64) -> (JobInput, JobOutput, Option<Raster>, Option<GrayRaster>, Option<Raster>) {
    let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
    let (output, new_pixels, new_mask, display) = run_edit_job(&input, pixels, mask, points.as_deref(), command, out_per_doc).unwrap();
    (input, output, crossed(new_pixels), new_mask, crossed(display))
}

#[test]
fn a_job_halves_its_whole_result_to_the_canvas_level_and_the_install_adopts_it() {
    let (mut e, id, layer) = document();
    // A tenth of a device pixel per document pixel: halve while one output pixel covers more than 2
    // source pixels, 10 -> 5 -> 2.5 -> 1.25, three times (`prefilterLevel`, the renderers' rule).
    let (input, output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert_eq!(prefilter_level(1001, 603, 10.0), 3);
    let new_pixels = new_pixels.expect("Levels replaces the pixels");
    let display = display.expect("a whole result is halved to the canvas level");
    // 1001 -> 500 -> 250 -> 125 and 603 -> 301 -> 150 -> 75.
    assert_eq!(output.display, Some(DisplayHalving { level: 3, width: 125, height: 75 }));
    assert_eq!(display.bytes(), from_scratch(&new_pixels, 3).bytes(), "the worker's halving is halving from scratch");
    e.install_job(id, layer, input.stamp, output, Some(new_pixels.clone()), new_mask, Some(display.clone())).unwrap();
    // No halving after install: the canvas's level is the very buffer the job made.
    let shown = e.layer_raster(id, layer, 3).unwrap().unwrap();
    assert!(shown.same_pixels(&display), "level 3 is the adopted halving itself");
    // A level past it halves the adopted halving; a level before it halves the pixels: both as from scratch.
    assert_eq!(e.layer_raster(id, layer, 4).unwrap().unwrap().bytes(), from_scratch(&new_pixels, 4).bytes());
    assert_eq!(e.layer_raster(id, layer, 2).unwrap().unwrap().bytes(), from_scratch(&new_pixels, 2).bytes());
    // The CPU compositor prefilters through it too.
    let region = Rect { x: 0.0, y: 0.0, width: 1001.0, height: 603.0 };
    let mut fresh = e.document(id).unwrap().clone();
    fresh.layers[0].pixels = Some(Raster::from_premultiplied(1001, 603, new_pixels.bytes().to_vec()));
    assert_eq!(e.composite(id, region, 100, 60).unwrap().bytes(), composite(&fresh, region, 100, 60).bytes());
}

#[test]
fn a_blank_layer_a_fill_grows_to_the_canvas_is_halved_on_the_grid_it_grew_to() {
    // Ruling C1's case: a new document's blank layer stores no pixels and so has no texture yet; the
    // fill paints the 800 x 600 canvas, and at a quarter of a device pixel per document pixel the
    // canvas draws it after one halving (4 -> 2).
    let mut e = Engine::new();
    let id = e.new_document(800, 600, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    let (_, output, new_pixels, _, display) = job(&e, id, layer, Command::Fill { id: layer, mask: false, color: [0.2, 0.4, 0.6] }, 0.25);
    assert_eq!(output.display, Some(DisplayHalving { level: 1, width: 400, height: 300 }));
    assert_eq!(display.unwrap().bytes(), from_scratch(&new_pixels.unwrap(), 1).bytes());
}

#[test]
fn nothing_is_halved_inside_a_selection_for_nearest_sampling_or_at_full_size() {
    let (mut e, id, layer) = document();
    // At 1:1 and closer the canvas does not prefilter; with no scale nothing is known.
    for scale in [1.0, 4.0, 0.0] {
        let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, scale);
        assert!(output.display.is_none() && display.is_none(), "at {scale}");
    }
    // Nearest is never prefiltered (`compositor::prefilters`).
    let mut nearest = e.document(id).unwrap().layers[0].transform;
    nearest.sampling = Sampling::Nearest;
    e.execute(id, Command::SetLayerTransform { id: layer, transform: nearest }).unwrap();
    let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert!(output.display.is_none() && display.is_none(), "Nearest");
    // Inside a selection the edit reports its rectangle, and the UI thread seeds the halvings.
    nearest.sampling = Sampling::High;
    e.execute(id, Command::SetLayerTransform { id: layer, transform: nearest }).unwrap();
    e.execute(id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(100.0, 100.0), p(300.0, 100.0), p(300.0, 200.0), p(100.0, 200.0)], mode: SelectionMode::Replace, antialiased: false }).unwrap();
    let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert!(!output.regions.is_empty() && output.display.is_none() && display.is_none(), "inside a selection");
}

#[test]
fn an_edit_inside_a_selection_after_a_job_redoes_only_its_rectangle_of_the_adopted_halving() {
    let (mut e, id, layer) = document();
    let (input, output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, display).unwrap();
    // A clear in a selection on the UI thread: an odd rectangle, off every block boundary.
    e.execute(id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(101.0, 51.0), p(333.0, 51.0), p(333.0, 197.0), p(101.0, 197.0)], mode: SelectionMode::Replace, antialiased: true }).unwrap();
    e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }).unwrap();
    let cleared = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let (level, seeded) = cleared.adopted().expect("the new pixels inherit the adopted halving");
    assert_eq!(level, 3);
    assert_eq!(seeded.bytes(), from_scratch(&cleared, 3).bytes(), "equal to halving the cleared pixels from scratch");
    assert!(e.layer_raster(id, layer, 3).unwrap().unwrap().same_pixels(&seeded), "and the canvas's level is it, halved nowhere");
}

#[test]
fn install_refuses_a_display_halving_that_does_not_fit_its_pixels() {
    let (mut e, id, layer) = document();
    let (input, mut output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    let depth = e.state(id).unwrap().undo_depth;
    // The right size, said to be two halvings: 125 x 75 is what three make.
    output.display = output.display.map(|d| DisplayHalving { level: 2, ..d });
    assert!(matches!(e.install_job(id, layer, input.stamp, output.clone(), new_pixels.clone(), new_mask.clone(), display.clone()), Err(CommandError::Argument(_))));
    // A buffer that is not the size the output names.
    output.display = output.display.map(|d| DisplayHalving { level: 3, ..d });
    let small = Some(Raster::from_premultiplied(1, 1, vec![0; 4]));
    assert!(matches!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, small), Err(CommandError::Argument(_))));
    assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing was put back");
}

#[test]
fn letting_go_of_halvings_lets_go_of_an_adopted_one_too() {
    // History drops the halvings of rasters only it holds (ruling OQ1).
    let r = pattern(40, 30);
    assert!(r.adopt(2, from_scratch(&r, 2)));
    assert!(!r.adopt(2, from_scratch(&r, 1)), "the wrong size is refused");
    assert!(!r.adopt(0, r.clone()), "level 0 is the pixels themselves");
    assert_eq!(r.adopted().map(|(level, _)| level), Some(2));
    r.forget_halvings();
    assert!(r.adopted().is_none());
}
