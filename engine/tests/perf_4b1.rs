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
        // Ruling I2: the same ticks on a freshly added, 1 x 1 uniform mask.
        run(&mut e, id, Command::AddMask { id: layer, revealing: true });
        let mask_drag = worst(&mut e, true, true);
        let mask_settled = worst(&mut e, false, true);
        e.set_preview(id, None).unwrap();
        run(&mut e, id, Command::DeleteMask { id: layer });
        // Fix round 1, item 1: a non-uniform full-size mask (an ellipse selection's), which the
        // gather must resample without a per-pixel division.
        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![Point { x: w as f64 * 0.1, y: h as f64 * 0.1 }, Point { x: w as f64 * 0.9, y: h as f64 * 0.1 }, Point { x: w as f64 * 0.9, y: h as f64 * 0.9 }, Point { x: w as f64 * 0.1, y: h as f64 * 0.9 }], mode: SelectionMode::Replace, antialiased: true });
        run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
        let nonuniform_drag = worst(&mut e, true, true);
        let nonuniform_settled = worst(&mut e, false, true);
        e.set_preview(id, None).unwrap();
        run(&mut e, id, Command::DeleteMask { id: layer });
        // Fix round 1, item 1: a full-size UNIFORM mask (a Fill on the targeted mask, which grows the
        // 1 x 1 mask onto the layer's grid first): the fast path must be an O(1) size check, never a
        // scan of the mask's content.
        run(&mut e, id, Command::AddMask { id: layer, revealing: true });
        run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.5, 0.5, 0.5] });
        let uniform_full_drag = worst(&mut e, true, true);
        let uniform_full_settled = worst(&mut e, false, true);
        e.set_preview(id, None).unwrap();
        let (x, y) = (w as f64 / 2.0 - 350.0, h as f64 / 2.0 - 350.0);
        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![Point { x, y }, Point { x: x + 700.0, y }, Point { x: x + 700.0, y: y + 700.0 }, Point { x, y: y + 700.0 }], mode: SelectionMode::Replace, antialiased: false });
        let patch = worst(&mut e, true, false);
        println!("{label}: gradient preview tick dragging {drag:.0} ms, settled {settled:.0} ms, 1x1 mask dragging {mask_drag:.0} ms, settled {mask_settled:.0} ms, non-uniform full mask dragging {nonuniform_drag:.0} ms, settled {nonuniform_settled:.0} ms, uniform full mask dragging {uniform_full_drag:.0} ms, settled {uniform_full_settled:.0} ms, patch in a 700 px selection {patch:.0} ms");
        assert!(e.preview(id).is_some());
    }
}

#[test]
#[ignore]
fn a_shape_over_the_whole_canvas_at_24_and_100_mp() {
    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
        // An empty document: the shape's own pixels are the project's only ones.
        let mut e = Engine::new();
        let id = e.new_document(w, h, false).unwrap();
        let mut times = Vec::new();
        for (name, shape) in [("rectangle", ShapeSpec::Rectangle { rect: Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, corner_radius: 400.0 }),
            ("ellipse", ShapeSpec::Ellipse { rect: Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 } })] {
            let t = Instant::now();
            run(&mut e, id, Command::AddShape { shape, color: [0.2, 0.4, 0.6] });
            times.push(format!("{name} {:.0} ms", ms(t)));
            e.undo(id).unwrap();
        }
        println!("{label}: a shape over the canvas: {}", times.join(", "));
        assert_eq!(e.state(id).unwrap().undo_depth, 0);
    }
}

#[test]
#[ignore]
fn a_mask_gradient_and_a_mask_fill_grown_to_the_canvas_at_24_and_100_mp() {
    // Task 14a: a 1500 x 1000 layer in the middle of the canvas under a checkered mask of its own grid;
    // a gradient, or a fill, grows the mask to the whole canvas. Budgets: the gradient no slower than the
    // pixel gradient's own commit over the whole canvas, measured natively on this machine in Task 8 (647
    // ms at 24 MP, 2706 ms at 100 MP, `a_gradient_and_a_fill_at_24_and_100_mp`): the mask paints one grey
    // byte where that paints four. The fill twice Task 8's native fill there (133 / 343 ms): that one
    // copied its 24 / 100 MP layer but painted a 3 MP ellipse; this one computes every pixel of the
    // canvas. Above JOB_PIXELS both run in the job worker (ruling C1: `edit_pixels` counts the grown mask).
    // The fill's are ceilings: under two thirds of one, the assert is tightened to 1.5x the measurement.
    for (label, w, h, budget, fill_budget) in [("24 MP", 6000u32, 4000u32, 650.0, 266.0), ("100 MP", 10000, 10000, 2710.0, 686.0)] {
        let mut doc = Document::new(w, h);
        let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(1500, 1000, [60, 90, 120, 255].repeat(1500 * 1000)), Point { x: ((w - 1500) / 2) as f64, y: ((h - 1000) / 2) as f64 });
        let checks: Vec<u8> = (0..1000u32).flat_map(|y| (0..1500u32).map(move |x| if (x / 50 + y / 50) % 2 == 0 { 0 } else { 255 })).collect();
        layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(1500, 1000, checks), enabled: true, placement: None, linked: None });
        let lid = layer.id;
        doc.active_layer_id = Some(lid);
        doc.layers = vec![layer];
        let mut e = Engine::new();
        let id = e.insert_document(doc);
        let gradient = |i: f64| GradientSpec { shape: GradientShape::Linear, start: Point { x: w as f64 * 0.2 + i, y: h as f64 * 0.3 }, end: Point { x: w as f64 * 0.8, y: h as f64 * 0.7 - i },
            from: [0.0, 0.0, 0.0, 1.0], to: [0.0, 0.0, 0.0, 0.0], opacity: 0.9 };
        // The preview ticks, to locate their cost (the release wasm's budgets are in perf-4b1.spec.ts):
        // the worst of five after a first.
        let worst = |e: &mut Engine, dragging: bool| {
            e.set_preview(id, Some(PreviewRequest::Gradient { layer: lid, mask: true, gradient: gradient(0.0), dragging })).unwrap();
            (1..6).map(|i| {
                let t = Instant::now();
                e.set_preview(id, Some(PreviewRequest::Gradient { layer: lid, mask: true, gradient: gradient(i as f64 * 7.0), dragging })).unwrap();
                ms(t)
            }).fold(0.0, f64::max)
        };
        let drag = worst(&mut e, true);
        let settled = worst(&mut e, false);
        e.set_preview(id, None).unwrap();
        let t = Instant::now();
        run(&mut e, id, Command::Gradient { id: lid, mask: true, gradient: gradient(0.0) });
        let commit = ms(t);
        let state = e.state(id).unwrap().layers[0].clone();
        assert_eq!((state.mask_width, state.mask_height), (w, h), "the gradient grew the mask to the canvas");
        e.undo(id).unwrap();
        let t = Instant::now();
        run(&mut e, id, Command::Fill { id: lid, mask: true, color: [0.0, 0.0, 0.0] });
        let fill = ms(t);
        let state = e.state(id).unwrap().layers[0].clone();
        assert_eq!((state.mask_width, state.mask_height), (w, h), "the fill grew the mask to the canvas");
        println!("{label}: mask grown to the canvas: gradient preview tick dragging {drag:.0} ms, settled {settled:.0} ms; gradient applied {commit:.0} ms; fill {fill:.0} ms");
        assert!(commit <= budget, "{label}: the grown mask gradient took {commit:.0} ms, budget {budget} ms");
        assert!(fill <= fill_budget, "{label}: the grown mask fill took {fill:.0} ms, budget {fill_budget} ms");
    }
}
