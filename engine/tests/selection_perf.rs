//! Release timings of the selection's hot paths (Phase 4a final review, findings F1-F3). Ignored:
//! run them with `cargo test --release -p compositor-engine --test selection_perf -- --ignored
//! --nocapture --test-threads=1` and read the printed numbers. They assert nothing about time (a
//! timing bound on a shared machine is noise); each checks only that it measured what it says.
use compositor_engine::selection::geometry::ellipse;
use compositor_engine::*;
use std::time::Instant;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

/// A `width` x `height` document whose one layer is opaque grey over the whole canvas (Canvas
/// Size's fill makes it without decoding an image, as perf-spatial.spec.ts does).
fn filled(width: u32, height: u32) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, false).unwrap();
    run(&mut e, id, Command::CanvasSize { width, height, anchor: 4, fill: Some([0.5, 0.4, 0.3]) });
    let layer = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::SetActiveLayer { id: Some(layer) });
    (e, id, layer)
}

fn levels(white: f64) -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: white, ..LevelRange::default() };
    a
}

#[test]
#[ignore]
fn levels_drag_ticks_on_a_6000_by_4000_layer() {
    // The reviewer's measurement: 512-px Levels drag ticks (`DragAdjustment`) on a 6000 x 4000 layer,
    // with no selection, a plain ellipse, and that ellipse feathered 20 and 63. Each tick asks for
    // other settings, so it recomputes; tick 1 is the first after the selection changed.
    for (label, feather) in [("no selection", None), ("plain ellipse", Some(0)), ("ellipse feather 20", Some(20)), ("ellipse feather 63", Some(63))] {
        let (mut e, id, layer) = filled(6000, 4000);
        if let Some(f) = feather {
            run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(700.0, 500.0), p(5300.0, 500.0), p(5300.0, 3500.0), p(700.0, 3500.0)], mode: SelectionMode::Replace, antialiased: true });
            if f > 0 { run(&mut e, id, Command::FeatherSelection { amount: f }); }
        }
        let mut ticks = Vec::new();
        for i in 0..4 {
            let t = Instant::now();
            e.set_preview(id, Some(PreviewRequest::DragAdjustment { layer, adjustment: levels(200.0 + i as f64) })).unwrap();
            ticks.push(ms(t));
        }
        let t = Instant::now();
        e.histogram(id, layer).unwrap();
        let histogram = ms(t);
        let t = Instant::now();
        run(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: levels(180.0) });
        let commit = ms(t);
        println!("F1 drag tick, {label}: ticks {:?} ms (mean of ticks 2-4 {:.1} ms); histogram {histogram:.1} ms; commit {commit:.1} ms",
            ticks.iter().map(|v| (v * 10.0).round() / 10.0).collect::<Vec<_>>(), ticks[1..].iter().sum::<f64>() / 3.0);
        assert_eq!(ticks.len(), 4);
    }
}

#[test]
#[ignore]
fn the_clip_of_a_100_megapixel_ellipse() {
    // `SelectionClip::new` on a 10000 x 10000 canvas: the ellipse inset 100 px, feather 0, 10, 50.
    for feather in [0.0, 10.0, 50.0] {
        let s = Selection::new(vec![ellipse(Rect { x: 100.0, y: 100.0, width: 9800.0, height: 9800.0 })], true, feather);
        let t = Instant::now();
        let clip = SelectionClip::new(&s, 10_000, 10_000);
        println!("F1 clip build, 100 MP, feather {feather}: {:.0} ms", ms(t));
        let c = clip.coverage.as_ref().unwrap();
        assert!(c.width >= 9800 && c.height >= 9800, "the region covers the ellipse: {}x{}", c.width, c.height);
    }
}

#[test]
#[ignore]
fn a_detailed_wand_outline_through_history_and_the_ants() {
    // The reviewer's F2 / F3 fixture: a non-contiguous wand at tolerance 16 on a noisy 4000 x 3000
    // image, then ten layer renames (each an undo snapshot), then the ants' outline at 1:1 and 1:2.
    let (mut e, id, layer) = filled(4000, 3000);
    run(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::AddNoise { amount: 25.0, gaussian: true, monochromatic: false, seed: 7 } });
    let t = Instant::now();
    run(&mut e, id, Command::MagicWand { at: p(1000.0, 1000.0), mode: SelectionMode::Replace, settings: WandSettings { tolerance: 16, sample_radius: 0, contiguous: false, all_layers: false }, antialiased: true });
    let wand = ms(t);
    let points = e.state(id).unwrap().selection.unwrap().points;
    let t = Instant::now();
    for i in 0..10 { run(&mut e, id, Command::RenameLayer { id: layer, name: format!("Layer {i}") }); }
    let renames = ms(t);
    let t = Instant::now();
    let full = e.selection_outline(id, 1.0).unwrap();
    let outline = ms(t);
    let t = Instant::now();
    let half = e.selection_outline(id, 0.5).unwrap();
    let lod = ms(t);
    println!("F2/F3 wand outline: {points} points (wand {wand:.0} ms); ten renames {renames:.1} ms; outline at 1:1 {outline:.1} ms ({} f64); LOD trace at 1:2 {lod:.1} ms ({} f64)", full.len(), half.len());
    assert!(points > 100_000, "a detailed outline: {points} points");
}
