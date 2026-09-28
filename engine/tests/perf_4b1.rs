//! Native release timings of Phase 4b-1's engine paths (LL-073). Ignored: run them with
//! `cargo test --release -p compositor-engine --test perf_4b1 -- --ignored --nocapture --test-threads=1`
//! and read the printed numbers. The budgets are the release wasm's (app/tests/e2e/perf-4b1.spec.ts);
//! these native numbers locate the cost. Each test checks only that it measured what it says.
use compositor_engine::*;
use std::time::Instant;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

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
