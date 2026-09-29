//! Native release timings of Phase 4b-1's engine paths (LL-073). Ignored: run them with
//! `cargo test --release -p compositor-engine --test perf_4b1 -- --ignored --nocapture --test-threads=1`
//! and read the printed numbers. The budgets are the release wasm's (app/tests/e2e/perf-4b1.spec.ts);
//! these native numbers locate the cost. Each test checks only that it measured what it says.
use compositor_engine::*;
use std::time::Instant;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

/// A `width` x `height` document with one opaque layer over it (Canvas Size's fill), active.
fn filled(width: u32, height: u32) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, false).unwrap();
    run(&mut e, id, Command::CanvasSize { width, height, anchor: 4, fill: Some([0.5, 0.4, 0.3]) });
    let layer = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::SetActiveLayer { id: Some(layer) });
    (e, id, layer)
}

#[test]
#[ignore]
fn halving_and_a_clear_in_a_selection_at_24_and_100_mp() {
    // Budgets (ruling I2): 1.5x a measurement taken on this machine before this assertion existed --
    // 59 ms at 24 MP and 245 ms at 100 MP (`cargo test --release -p compositor-engine --test perf_4b1
    // -- --ignored --nocapture --test-threads=1 halving_and_a_clear_in_a_selection_at_24_and_100_mp`,
    // Task 3 fix round 1). Not the release wasm's own budget (that is `perf-4b1.spec.ts`'s, per
    // LL-073): this only catches this native path regressing hard from what it measured at.
    for (label, w, h, budget) in [("24 MP", 6000u32, 4000u32, 88.5), ("100 MP", 10000, 10000, 367.5)] {
        let (mut e, id, layer) = filled(w, h);
        let raster = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
        let t = Instant::now();
        let _ = raster.halved().halved().halved();
        let chain = ms(t);
        // A 1024 x 1024 selection cleared: the edit copies the layer; the new pixels are handed the
        // old halvings (three levels) with only the cleared part redone.
        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![Point { x: 1000.0, y: 1000.0 }, Point { x: 2024.0, y: 1000.0 }, Point { x: 2024.0, y: 2024.0 }, Point { x: 1000.0, y: 2024.0 }], mode: SelectionMode::Replace, antialiased: false });
        let t = Instant::now();
        run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
        let clear = ms(t);
        let new = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
        let t = Instant::now();
        let seeded = new.halved().halved().halved();
        let after = ms(t);
        println!("{label}: halving chain to level 3 {chain:.0} ms; clear in a 1024 px selection {clear:.0} ms; level 3 after it {after:.2} ms");
        assert_eq!(seeded.width, w / 8);
        assert!(clear <= budget, "{label}: clear in a 1024 px selection took {clear:.0} ms, budget {budget} ms");
    }
}

#[test]
#[ignore]
fn a_gradient_and_a_fill_at_24_and_100_mp() {
    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
        let (mut e, id, layer) = filled(w, h);
        let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: 0.0, y: 0.0 }, end: Point { x: w as f64, y: h as f64 },
            from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 0.0], opacity: 0.8 };
        let t = Instant::now();
        run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient });
        let whole = ms(t);
        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![Point { x: 500.0, y: 500.0 }, Point { x: 2500.0, y: 500.0 }, Point { x: 2500.0, y: 2000.0 }, Point { x: 500.0, y: 2000.0 }], mode: SelectionMode::Replace, antialiased: true });
        let t = Instant::now();
        run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
        let fill = ms(t);
        println!("{label}: gradient over the whole layer {whole:.0} ms; fill in a 2000 x 1500 ellipse {fill:.0} ms");
        assert_eq!(e.state(id).unwrap().layers[0].pixels_width, w);
    }
}

#[test]
#[ignore]
fn a_rename_at_the_history_cap_with_a_thousand_layers() {
    let mut e = Engine::new();
    let id = e.new_document(200, 200, false).unwrap();
    let png = {
        let mut small = Engine::new();
        let s = small.new_document(8, 8, true).unwrap();
        small.export_png(s).unwrap()
    };
    for i in 0..1000 { e.import_image(Some(id), &png, &format!("L{i}"), Some(Point { x: 100.0, y: 100.0 })).unwrap(); }
    let layer = e.state(id).unwrap().layers[0].id;
    for i in 0..100 { run(&mut e, id, Command::RenameLayer { id: layer, name: format!("n{i}") }); }
    let mut times = Vec::new();
    for i in 0..10 {
        let t = Instant::now();
        run(&mut e, id, Command::RenameLayer { id: layer, name: format!("m{i}") });
        times.push(ms(t));
    }
    let doc = e.document(id).unwrap().clone();
    let mut history = History::default();
    for _ in 0..100 { history.push(doc.clone()); }
    let t = Instant::now();
    for _ in 0..10 { history.trim(&doc); }
    let trim = ms(t) / 10.0;
    println!("rename at the cap, 1000 layers: {:?} ms; trim alone {trim:.2} ms", times.iter().map(|t| (t * 10.0).round() / 10.0).collect::<Vec<_>>());
    assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT);
    // Ruling I2: undo at the cap gets a timing alongside the push above, no looser a budget (the
    // release wasm test asserts the actual budget; this locates the native cost).
    let mut undo_times = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        e.undo(id).unwrap();
        undo_times.push(ms(t));
    }
    println!("undo at the cap, 1000 layers: {:?} ms", undo_times.iter().map(|t| (t * 10.0).round() / 10.0).collect::<Vec<_>>());
    assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT - 10);
}

#[test]
#[ignore]
fn gradient_previews_dragged_settled_and_patched_at_24_and_100_mp() {
    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
        let (mut e, id, layer) = filled(w, h);
        let gradient = |i: f64| GradientSpec { shape: GradientShape::Linear, start: Point { x: w as f64 * 0.2 + i, y: h as f64 * 0.3 }, end: Point { x: w as f64 * 0.8, y: h as f64 * 0.7 - i },
            from: [1.0, 0.2, 0.0, 1.0], to: [0.0, 0.0, 1.0, 0.3], opacity: 0.9 };
        // The worst of five ticks after a first one (which halves the layer once for all).
        let worst = |e: &mut Engine, dragging: bool, mask: bool| {
            e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask, gradient: gradient(0.0), dragging })).unwrap();
            (1..6).map(|i| {
                let t = Instant::now();
                e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask, gradient: gradient(i as f64 * 7.0), dragging })).unwrap();
                ms(t)
            }).fold(0.0, f64::max)
        };
        let drag = worst(&mut e, true, false);
        let settled = worst(&mut e, false, false);
        e.set_preview(id, None).unwrap();
        // Ruling I2: the same ticks on the layer's mask.
        run(&mut e, id, Command::AddMask { id: layer, revealing: true });
        let mask_drag = worst(&mut e, true, true);
        let mask_settled = worst(&mut e, false, true);
        e.set_preview(id, None).unwrap();
        let (x, y) = (w as f64 / 2.0 - 350.0, h as f64 / 2.0 - 350.0);
        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![Point { x, y }, Point { x: x + 700.0, y }, Point { x: x + 700.0, y: y + 700.0 }, Point { x, y: y + 700.0 }], mode: SelectionMode::Replace, antialiased: false });
        let patch = worst(&mut e, true, false);
        println!("{label}: gradient preview tick dragging {drag:.0} ms, settled {settled:.0} ms, mask dragging {mask_drag:.0} ms, mask settled {mask_settled:.0} ms, patch in a 700 px selection {patch:.0} ms");
        assert!(e.preview(id).is_some());
    }
}
