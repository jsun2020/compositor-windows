//! The selection commands through `Engine::execute`, ported from Compositor for Mac's
//! CompositorTests/SelectionTests.swift with its own shapes, coordinates and expected values. What
//! those tests drive through the canvas (modifier keys, drags, the M and L keys) is the app's; here
//! the engine receives what the app sends.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
fn square(x: f64, y: f64, size: f64) -> Vec<Point> { vec![p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)] }

/// A `width` x `height` document with one blank layer, as the Mac's `makeSession`.
fn session(width: u32, height: u32) -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, true).unwrap();
    (e, id)
}
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn lasso(e: &mut Engine, id: Uuid, points: Vec<Point>, mode: SelectionMode) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Freehand, points, mode, antialiased: true });
}
fn selection(e: &Engine, id: Uuid) -> Option<Selection> { e.document(id).unwrap().selection.clone() }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
/// Coverage 0-255 at a document pixel, as the Mac's tests read it.
fn coverage(e: &Engine, id: Uuid, x: u32, y: u32) -> u8 {
    let d = e.document(id).unwrap();
    let Some(s) = &d.selection else { return 0 };
    let c = SelectionClip::new(s, d.width, d.height).on_grid(&Affine::IDENTITY, d.width, d.height);
    c.bytes()[(y * d.width + x) as usize]
}

#[test]
fn replace_add_and_subtract_combine_outlines() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 40.0), SelectionMode::Replace);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 70, 70)), (255, 0));
    lasso(&mut e, id, square(50.0, 50.0, 40.0), SelectionMode::Add);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 70, 70)), (255, 255));
    lasso(&mut e, id, square(20.0, 20.0, 20.0), SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 15, 15)), (0, 255));
    lasso(&mut e, id, square(60.0, 10.0, 20.0), SelectionMode::Replace);
    assert_eq!((coverage(&e, id, 70, 20), coverage(&e, id, 70, 70), coverage(&e, id, 15, 15)), (255, 0, 0));
}

#[test]
fn a_selection_is_clipped_to_the_canvas() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(-50.0, -50.0, 100.0), SelectionMode::Replace);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 50.0, 50.0)));
}

#[test]
fn an_empty_selection_is_distinct_from_no_selection() {
    let (mut e, id) = session(100, 100);
    let before = depth(&e, id);
    lasso(&mut e, id, square(0.0, 0.0, 50.0), SelectionMode::Subtract);
    assert!(selection(&e, id).is_none() && depth(&e, id) == before, "nothing to subtract from: no change, no step");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    let s = selection(&e, id).expect("an explicit empty selection");
    assert!(s.is_empty());
    assert_eq!(coverage(&e, id, 20, 20), 0);
    assert!(e.state(id).unwrap().selection.unwrap().empty);
    run(&mut e, id, Command::Deselect);
    assert!(selection(&e, id).is_none() && e.state(id).unwrap().selection.is_none());
}

#[test]
fn a_click_deselects_and_each_selection_change_is_one_undo_step() {
    let (mut e, id) = session(100, 100);
    let count = depth(&e, id);
    lasso(&mut e, id, square(10.0, 10.0, 40.0), SelectionMode::Replace);
    assert_eq!(depth(&e, id), count + 1);
    lasso(&mut e, id, vec![p(5.0, 5.0)], SelectionMode::Replace);
    assert!(selection(&e, id).is_none() && depth(&e, id) == count + 2, "a click deselects, as one step");
    e.undo(id).unwrap();
    assert!(!selection(&e, id).unwrap().is_empty());
    e.undo(id).unwrap();
    assert!(selection(&e, id).is_none());
    e.redo(id).unwrap();
    assert_eq!(coverage(&e, id, 30, 30), 255);
    // In Add or Subtract a click changes nothing and records nothing.
    let now = depth(&e, id);
    lasso(&mut e, id, vec![p(5.0, 5.0), p(9.0, 5.0)], SelectionMode::Add);
    assert_eq!((depth(&e, id), coverage(&e, id, 30, 30)), (now, 255));
}

#[test]
fn a_polygonal_outline_closes_on_its_first_corner() {
    // SelectionTests.polygonalCornersCanBeRemovedAndClosed, after the misplaced corner came off.
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Polygonal, points: vec![p(10.0, 10.0), p(90.0, 10.0), p(90.0, 90.0), p(10.0, 90.0)], mode: SelectionMode::Replace, antialiased: true });
    assert_eq!((coverage(&e, id, 80, 80), coverage(&e, id, 5, 50)), (255, 0));
}

#[test]
fn select_all_and_inverse() {
    let (mut e, id) = session(100, 80);
    run(&mut e, id, Command::SelectAll);
    assert_eq!((coverage(&e, id, 0, 0), coverage(&e, id, 99, 79)), (255, 255));
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 100.0, 80.0)));
    lasso(&mut e, id, square(0.0, 0.0, 50.0), SelectionMode::Replace);
    run(&mut e, id, Command::InvertSelection);
    assert_eq!((coverage(&e, id, 25, 25), coverage(&e, id, 75, 60)), (0, 255));
    // Inverse keeps the flags; with no selection it does nothing.
    run(&mut e, id, Command::FeatherSelection { amount: 3 });
    run(&mut e, id, Command::InvertSelection);
    assert_eq!(selection(&e, id).unwrap().feather, 3.0);
    run(&mut e, id, Command::Deselect);
    let before = depth(&e, id);
    run(&mut e, id, Command::InvertSelection);
    assert!(selection(&e, id).is_none() && depth(&e, id) == before);
}

#[test]
fn dragging_moves_the_outline_in_whole_pixels_as_one_undo_step() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    assert!(selection(&e, id).unwrap().contains(p(20.0, 20.0)) && !selection(&e, id).unwrap().contains(p(60.0, 60.0)));
    let count = depth(&e, id);
    // The drag's last offset, as the Mac's moveSelection(by: 40.2, 40.4) rounds it.
    run(&mut e, id, Command::MoveSelection { dx: 40.2, dy: 40.4 });
    assert_eq!(depth(&e, id), count + 1);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(50.0, 50.0, 20.0, 20.0)));
    assert_eq!((coverage(&e, id, 55, 55), coverage(&e, id, 15, 15)), (255, 0));
    e.undo(id).unwrap();
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(10.0, 10.0, 20.0, 20.0)));
}

#[test]
fn moving_off_the_canvas_and_back_keeps_the_whole_shape() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    run(&mut e, id, Command::MoveSelection { dx: -25.0, dy: 0.0 });
    assert_eq!(coverage(&e, id, 0, 20), 255);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(-15.0, 10.0, 20.0, 20.0)), "not cut to the canvas");
    run(&mut e, id, Command::MoveSelection { dx: 25.0, dy: 0.0 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(10.0, 10.0, 20.0, 20.0)));
}

#[test]
fn nudges_are_one_step_each_and_need_a_real_selection() {
    let (mut e, id) = session(100, 100);
    assert!(matches!(e.execute(id, Command::MoveSelection { dx: 1.0, dy: 0.0 }), Err(CommandError::Refused(_))), "nothing selected");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    let count = depth(&e, id);
    run(&mut e, id, Command::MoveSelection { dx: 1.0, dy: 0.0 });
    run(&mut e, id, Command::MoveSelection { dx: 0.0, dy: -10.0 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(11.0, 0.0, 20.0, 20.0)));
    assert_eq!(depth(&e, id), count + 2);
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    assert!(selection(&e, id).unwrap().is_empty());
    assert_eq!(e.execute(id, Command::MoveSelection { dx: 1.0, dy: 0.0 }).unwrap_err(), CommandError::Refused("The selection is empty".into()));
}

#[test]
fn expand_and_contract_grow_and_shrink_the_outline() {
    let (mut e, id) = session(100, 100);
    assert!(e.execute(id, Command::ExpandSelection { amount: 5 }).is_err(), "nothing to modify");
    lasso(&mut e, id, square(40.0, 40.0, 20.0), SelectionMode::Replace);
    run(&mut e, id, Command::ExpandSelection { amount: 5 });
    let grown = selection(&e, id).unwrap().bounds().unwrap();
    assert!((grown.x - 35.0).abs() < 0.01 && (grown.width - 30.0).abs() < 0.01, "{grown:?}");
    assert_eq!((coverage(&e, id, 37, 50), coverage(&e, id, 33, 50)), (255, 0));
    run(&mut e, id, Command::ContractSelection { amount: 8 });
    let shrunk = selection(&e, id).unwrap().bounds().unwrap();
    assert!((shrunk.x - 43.0).abs() < 0.01 && (shrunk.width - 14.0).abs() < 0.01, "{shrunk:?}");
    e.undo(id).unwrap();
    assert!((selection(&e, id).unwrap().bounds().unwrap().width - 30.0).abs() < 0.01);
    for bad in [0, 501] { assert!(e.execute(id, Command::ExpandSelection { amount: bad }).is_err(), "{bad}"); }
}

#[test]
fn expand_stays_on_the_canvas_and_contract_can_empty_the_selection() {
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectAll);
    run(&mut e, id, Command::ExpandSelection { amount: 10 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 100.0, 100.0)));
    run(&mut e, id, Command::ContractSelection { amount: 10 });
    assert_eq!((coverage(&e, id, 5, 50), coverage(&e, id, 50, 50)), (0, 255), "it pulls in from the canvas edges too");
    run(&mut e, id, Command::ContractSelection { amount: 45 });
    assert!(selection(&e, id).unwrap().is_empty());
    assert!(e.execute(id, Command::ContractSelection { amount: 1 }).is_err(), "an empty selection cannot be modified");
}

#[test]
fn feathers_combine_as_blurs_do_and_stop_at_250() {
    let (mut e, id) = session(60, 20);
    run(&mut e, id, Command::SelectAll);
    run(&mut e, id, Command::FeatherSelection { amount: 6 });
    assert_eq!(selection(&e, id).unwrap().feather, 6.0);
    run(&mut e, id, Command::FeatherSelection { amount: 8 });
    assert_eq!(selection(&e, id).unwrap().feather, (36.0f64 + 64.0).sqrt());
    run(&mut e, id, Command::FeatherSelection { amount: 250 });
    assert_eq!(selection(&e, id).unwrap().feather, 250.0);
    assert!(e.execute(id, Command::FeatherSelection { amount: 251 }).is_err());
}

#[test]
fn the_marquee_selects_its_whole_pixel_box_and_a_click_deselects() {
    // SelectionTests.marqueeDrawsWholePixelRectanglesInAnyDirection, after the app's DragBox has
    // rounded the drag from (60.4, 70.6) to (20.2, 30.3) to the box (20, 30)-(60, 71).
    let (mut e, id) = session(100, 100);
    let marquee = |e: &mut Engine, x0: f64, y0: f64, x1: f64, y1: f64, mode: SelectionMode| {
        run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], mode, antialiased: true });
    };
    marquee(&mut e, 20.0, 30.0, 60.0, 71.0, SelectionMode::Replace);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(20.0, 30.0, 40.0, 41.0)));
    assert_eq!((coverage(&e, id, 20, 30), coverage(&e, id, 19, 30)), (255, 0));
    marquee(&mut e, 80.0, 80.0, 90.0, 90.0, SelectionMode::Add);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 40, 50)), (255, 255));
    marquee(&mut e, 30.0, 40.0, 50.0, 60.0, SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 40, 50), coverage(&e, id, 25, 35)), (0, 255));
    marquee(&mut e, 5.0, 5.0, 5.0, 5.0, SelectionMode::Replace);
    assert!(selection(&e, id).is_none());
}

#[test]
fn the_elliptical_marquee_selects_an_oval_in_its_box() {
    // SelectionTests.marqueeEllipseSelectsAnOvalInItsBoxAndShiftMakesACircle.
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(10.0, 20.0), p(70.0, 20.0), p(70.0, 60.0), p(10.0, 60.0)], mode: SelectionMode::Replace, antialiased: true });
    let b = selection(&e, id).unwrap().bounds().unwrap();
    assert!((b.x - 10.0).abs() < 0.5 && (b.max_x() - 70.0).abs() < 0.5 && (b.y - 20.0).abs() < 0.5 && (b.max_y() - 60.0).abs() < 0.5);
    assert_eq!((coverage(&e, id, 40, 40), coverage(&e, id, 11, 21)), (255, 0));
}

#[test]
fn the_selection_is_part_of_the_document_but_never_saved() {
    let (mut e, id) = session(80, 60);
    let first = e.state(id).unwrap();
    assert!(first.selection.is_none(), "a new document has none");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    let r1 = e.state(id).unwrap().selection.unwrap().revision;
    run(&mut e, id, Command::MoveSelection { dx: 3.0, dy: 0.0 });
    let r2 = e.state(id).unwrap().selection.unwrap().revision;
    assert!(r2 > r1, "a new outline, a new revision");
    let manifest = e.save_package(id).unwrap().manifest_json;
    assert!(!manifest.contains("selection"), "the Mac's manifest has no selection (ProjectStore.swift:13-56)");
    let reopened = e.open_package(&e.save_package(id).unwrap(), None).unwrap();
    assert!(e.state(reopened).unwrap().selection.is_none(), "opening starts with no selection");
    // Undo returns the earlier outline with its own revision; a different edit then gets a new one.
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().selection.unwrap().revision, r1);
    run(&mut e, id, Command::MoveSelection { dx: 0.0, dy: 4.0 });
    let r3 = e.state(id).unwrap().selection.unwrap().revision;
    assert!(r3 > r2, "never a revision an earlier outline had");
    let s = e.state(id).unwrap().selection.unwrap();
    assert_eq!((s.bounds, s.points, s.antialiased, s.feather, s.empty), (Some(rect(10.0, 14.0, 20.0, 20.0)), 4, true, 0.0, false));
}

#[test]
fn history_snapshots_share_the_outline_and_undo_still_restores_a_changed_one() {
    // Final review F2: every snapshot and working copy used to deep-copy the outline (a 4-million-
    // point wand outline, twice per edit). An edit that leaves the selection alone keeps the very
    // same points in the new document and in the snapshot it pushes.
    let (mut e, id) = session(80, 60);
    let layer = e.state(id).unwrap().layers[0].id;
    lasso(&mut e, id, vec![p(10.0, 10.0), p(50.0, 12.0), p(30.0, 40.0)], SelectionMode::Replace);
    let outline = selection(&e, id).unwrap().contours;
    let count = depth(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "Renamed".into() });
    assert_eq!(depth(&e, id), count + 1, "the rename is a step");
    assert!(std::sync::Arc::ptr_eq(&outline, &selection(&e, id).unwrap().contours), "the edited document shares the outline");
    e.undo(id).unwrap();
    assert!(std::sync::Arc::ptr_eq(&outline, &selection(&e, id).unwrap().contours), "so does the snapshot before it");
    // A changed selection is still its own step, and undo brings the old points back.
    lasso(&mut e, id, square(5.0, 5.0, 10.0), SelectionMode::Replace);
    assert_ne!(*selection(&e, id).unwrap().contours, *outline);
    e.undo(id).unwrap();
    assert_eq!(*selection(&e, id).unwrap().contours, *outline);
    // An equal outline built afresh (its own points, the same values) is no change: no step.
    run(&mut e, id, Command::SelectAll);
    let (all, count) = (selection(&e, id).unwrap().contours, depth(&e, id));
    run(&mut e, id, Command::SelectAll);
    assert!(!std::sync::Arc::ptr_eq(&all, &selection(&e, id).unwrap().contours), "a new outline was built");
    assert_eq!(depth(&e, id), count, "equal by content, so no step");
}

#[test]
fn crop_canvas_size_and_image_size_drop_the_selection_and_flip_canvas_mirrors_it() {
    let (mut e, id) = session(100, 80);
    // y 10..60 on an 80-high canvas: asymmetric about the canvas middle (40), so a vertical flip
    // that does nothing would fail this (ruling I4). Mirrored y = 80 - y, so 10..60 -> 20..70.
    lasso(&mut e, id, vec![p(10.0, 10.0), p(40.0, 10.0), p(40.0, 60.0), p(10.0, 60.0)], SelectionMode::Replace);
    run(&mut e, id, Command::FlipCanvas { horizontal: true });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(60.0, 10.0, 30.0, 50.0)));
    run(&mut e, id, Command::FlipCanvas { horizontal: false });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(60.0, 20.0, 30.0, 50.0)), "mirrored across the canvas height (80): 10..60 -> 20..70");
    for command in [Command::CanvasSize { width: 120, height: 90, anchor: 4, fill: None }, Command::Crop { x: 5.0, y: 5.0, width: 50.0, height: 50.0 },
        Command::ImageSize { width: 50, height: 40, resolution: 72.0, sampling: Sampling::High }] {
        run(&mut e, id, command.clone());
        assert!(selection(&e, id).is_none(), "{command:?} drops the selection");
        e.undo(id).unwrap();
        assert!(selection(&e, id).is_some(), "{command:?}: undo brings it back");
    }
}

#[test]
fn the_commands_read_as_json_and_carry_the_macs_undo_names() {
    let c: Command = serde_json::from_str(r#"{"type":"SelectShape","kind":"Ellipse","points":[[1,2],[9,2],[9,7],[1,7]],"mode":"Subtract","antialiased":false}"#).unwrap();
    assert_eq!(c, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(1.0, 2.0), p(9.0, 2.0), p(9.0, 7.0), p(1.0, 7.0)], mode: SelectionMode::Subtract, antialiased: false });
    let names: Vec<(Command, &str)> = vec![
        (Command::SelectShape { kind: SelectionShape::Freehand, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Lasso"),
        (Command::SelectShape { kind: SelectionShape::Polygonal, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Polygonal Lasso"),
        (Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Rectangular Marquee"),
        (c, "Elliptical Marquee"),
        (Command::SelectAll, "Select All"), (Command::Deselect, "Deselect"), (Command::InvertSelection, "Inverse"),
        (Command::MoveSelection { dx: 1.0, dy: 0.0 }, "Move Selection"), (Command::ExpandSelection { amount: 1 }, "Expand Selection"),
        (Command::ContractSelection { amount: 1 }, "Contract Selection"), (Command::FeatherSelection { amount: 1 }, "Feather Selection"),
    ];
    for (command, name) in names { assert_eq!(command.action_name(), name); }
}

#[test]
fn the_outline_travels_flat_and_a_complex_one_at_screen_resolution_below_one_to_one() {
    let (mut e, id) = session(100, 80);
    assert!(e.selection_outline(id, 1.0).unwrap().is_empty(), "nothing selected");
    lasso(&mut e, id, vec![p(10.0, 20.0), p(40.0, 20.0), p(40.0, 60.0)], SelectionMode::Replace);
    let flat = e.selection_outline(id, 0.25).unwrap();
    assert_eq!(flat[0], 1.0, "one contour");
    assert_eq!(flat[1], 3.0, "of three points");
    let mut points: Vec<(f64, f64)> = flat[2..].chunks(2).map(|c| (c[0], c[1])).collect();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(points, vec![(10.0, 20.0), (40.0, 20.0), (40.0, 60.0)], "a simple outline travels as it is, at any zoom");
    // A freehand outline with a fine sawtooth along its top: 24,000 teeth corners, half a pixel
    // deep, over (0, 20)-(90, 20), then down to (90, 60) and back to (0, 60).
    let mut teeth: Vec<Point> = (0..24_000).map(|k| p(k as f64 * 90.0 / 24_000.0, 20.0 + (k % 2) as f64 * 0.5)).collect();
    teeth.extend([p(90.0, 60.0), p(0.0, 60.0)]);
    lasso(&mut e, id, teeth, SelectionMode::Replace);
    let points = selection(&e, id).unwrap().point_count();
    assert!(points > OUTLINE_DETAIL_LIMIT, "{points} points");
    let full = e.selection_outline(id, 1.0).unwrap();
    assert_eq!((full[0], full[1] as usize), (1.0, points), "at 1:1 the outline travels as it is");
    // At a quarter the outline is filled into a 23 x 10 mask (4 document pixels a cell), any
    // coverage counting, and traced: the band (0, 20)-(90, 60) rounded out to the cells, one loop
    // of four corners, (0, 20)-(92, 60) in document pixels.
    let lod = e.selection_outline(id, 0.25).unwrap();
    assert_eq!((lod[0], lod[1]), (1.0, 4.0), "{lod:?}");
    let mut corners: Vec<(f64, f64)> = lod[2..].chunks(2).map(|c| (c[0], c[1])).collect();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners, vec![(0.0, 20.0), (0.0, 60.0), (92.0, 20.0), (92.0, 60.0)]);
}


#[test]
fn a_press_moves_the_outline_only_inside_a_selection_with_something_in_it() {
    // SelectionTests.draggingMovesTheOutlineInWholePixelsAsOneUndo / arrowNudgesAndMoveIsOnlyForNewModeOnARealSelection.
    let (mut e, id) = session(100, 100);
    assert!(!e.selection_contains(id, p(20.0, 20.0)).unwrap(), "nothing selected");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    assert!(e.selection_contains(id, p(20.0, 20.0)).unwrap());
    assert!(!e.selection_contains(id, p(60.0, 60.0)).unwrap());
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    assert!(!e.selection_contains(id, p(20.0, 20.0)).unwrap(), "an empty selection does not move");
}
