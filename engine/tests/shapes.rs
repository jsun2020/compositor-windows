//! The Shape tool's layers (Phase 4b-1), ported from the Mac's ShapeToolTests with their numbers,
//! and the shapes' edges against an independent area count.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
const RED: [f64; 3] = [1.0, 0.0, 0.0];

/// The Mac test's session: 100 x 80 with one empty layer, "Layer 1".
fn session() -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(100, 80, true).unwrap();
    (e, id)
}
fn shape(e: &mut Engine, id: Uuid, shape: ShapeSpec, color: [f64; 3]) { run(e, id, Command::AddShape { shape, color }); }
/// The flattened document's pixel: (red, alpha), premultiplied as the Mac's test reads it.
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> (u8, u8) {
    let px = e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0);
    (px[0], px[3])
}
fn names(e: &Engine, id: Uuid) -> Vec<String> { e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect() }

#[test]
fn a_rectangle_fills_a_new_layer_with_the_colour_as_one_undo_step_and_keeps_the_selection() {
    // ShapeToolTests.rectangleFillsANewLayerWithTheForegroundColorAsOneUndoStep.
    let (mut e, id) = session();
    run(&mut e, id, Command::SelectAll);
    let depth = e.state(id).unwrap().undo_depth;
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 30.0, 20.0), corner_radius: 0.0 }, RED);
    let s = e.state(id).unwrap();
    assert_eq!(names(&e, id), ["Layer 1", "Rectangle 1"]);
    let layer = s.layers.iter().find(|l| l.name == "Rectangle 1").unwrap();
    assert_eq!(s.active_layer_id, Some(layer.id));
    assert_eq!(s.undo_depth, depth + 1);
    assert_eq!(Command::AddShape { shape: ShapeSpec::Ellipse { rect: rect(0.0, 0.0, 1.0, 1.0) }, color: RED }.action_name(), "Ellipse", "the undo name is the kind");
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width, layer.transform.size.height), (10.0, 10.0, 30.0, 20.0));
    assert_eq!((layer.pixels_width, layer.pixels_height), (30, 20));
    assert!(s.selection.is_some(), "unlike Paste, drawing a shape keeps the selection");
    for (x, y) in [(25, 20), (10, 10), (39, 29)] { assert_eq!(pixel(&e, id, x, y), (255, 255), "({x}, {y})"); }
    for (x, y) in [(9, 20), (40, 20), (25, 30)] { assert_eq!(pixel(&e, id, x, y).1, 0, "({x}, {y})"); }
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(60.0, 10.0, 10.0, 10.0), corner_radius: 0.0 }, RED);
    assert_eq!(names(&e, id).last().unwrap(), "Rectangle 2");
    e.undo(id).unwrap();
    e.undo(id).unwrap();
    assert_eq!(names(&e, id), ["Layer 1"]);
}

#[test]
fn an_ellipse_leaves_its_corners_clear() {
    // ShapeToolTests.ellipseLeavesItsCornersClearWithShiftCircleAndOptionFromCenter: the drag from
    // (50, 40) to (60, 45) with Shift and Option is the box (40, 30, 20, 20) (the app's DragBox).
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Ellipse { rect: rect(40.0, 30.0, 20.0, 20.0) }, RED);
    let layer = e.state(id).unwrap().layers[1].clone();
    assert_eq!(layer.name, "Ellipse 1");
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width), (40.0, 30.0, 20.0));
    assert_eq!(pixel(&e, id, 50, 40), (255, 255));
    assert!(pixel(&e, id, 41, 40).1 > 0 && pixel(&e, id, 50, 31).1 > 0);
    assert_eq!(pixel(&e, id, 40, 30).1, 0, "outside the circle, inside its box");
    assert_eq!(pixel(&e, id, 59, 49).1, 0);
}

#[test]
fn rounded_rectangles_follow_the_radius_and_clamp_to_a_pill() {
    // ShapeToolTests.roundedRectanglesFollowTheRadiusAndClampToAPill.
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 40.0, 30.0), corner_radius: 8.0 }, RED);
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(55.0, 50.0, 40.0, 20.0), corner_radius: 500.0 }, RED);
    assert_eq!(pixel(&e, id, 10, 10).1, 0, "the corner is cut away");
    assert_eq!(pixel(&e, id, 11, 11).1, 0);
    assert_eq!(pixel(&e, id, 13, 13).1, 255, "inside the rounded corner");
    assert_eq!(pixel(&e, id, 30, 10).1, 255, "straight edges stay full");
    assert_eq!(pixel(&e, id, 30, 25), (255, 255));
    assert_eq!(pixel(&e, id, 55, 50).1, 0, "the pill's corner is round");
    assert_eq!(pixel(&e, id, 75, 60), (255, 255));
    assert_eq!(e.state(id).unwrap().layers.len(), 3);
    // Clamped to 10 (half of 20): the record keeps what was asked, as the Mac's does.
    let doc = e.document(id).unwrap();
    assert_eq!(doc.layers[2].extra.shape.as_ref().unwrap()["cornerRadius"], serde_json::json!(500));
}

/// The fraction of pixel (`x`, `y`) inside `inside`, counted on a 32 x 32 grid of sample points:
/// independent of the engine's outlines and rasteriser.
fn area(x: u32, y: u32, inside: impl Fn(f64, f64) -> bool) -> f64 {
    let n = 32;
    let mut count = 0;
    for j in 0..n { for i in 0..n {
        if inside(x as f64 + (i as f64 + 0.5) / n as f64, y as f64 + (j as f64 + 0.5) / n as f64) { count += 1; }
    }}
    count as f64 / (n * n) as f64
}

#[test]
fn every_edge_pixel_is_the_area_the_shape_covers() {
    // A 37 x 23 ellipse at (5, 7), a 40 x 26 rectangle rounded by 9 at (50, 40), and a line 5 wide
    // from (12.5, 60.25) to (47.75, 71), each alone in a 100 x 80 document; every pixel of its
    // layer against the area count of the true curve (the flattening and the count: 4 levels).
    let cases: Vec<(ShapeSpec, Box<dyn Fn(f64, f64) -> bool>)> = vec![
        (ShapeSpec::Ellipse { rect: rect(5.0, 7.0, 37.0, 23.0) }, Box::new(|x, y| {
            let (u, v) = ((x - 23.5) / 18.5, (y - 18.5) / 11.5);
            u * u + v * v <= 1.0
        })),
        (ShapeSpec::Rectangle { rect: rect(50.0, 40.0, 40.0, 26.0), corner_radius: 9.0 }, Box::new(|x, y| {
            if !(50.0..=90.0).contains(&x) || !(40.0..=66.0).contains(&y) { return false; }
            let cx = x.clamp(59.0, 81.0);
            let cy = y.clamp(49.0, 57.0);
            (x - cx).hypot(y - cy) <= 9.0
        })),
        (ShapeSpec::Line { start: p(12.5, 60.25), end: p(47.75, 71.0), width: 5.0 }, Box::new(|x, y| {
            let (ax, ay, bx, by) = (12.5, 60.25, 47.75, 71.0);
            let t = (((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax).powi(2) + (by - ay).powi(2))).clamp(0.0, 1.0);
            (x - (ax + t * (bx - ax))).hypot(y - (ay + t * (by - ay))) <= 2.5
        })),
    ];
    for (spec, inside) in cases {
        let (mut e, id) = session();
        shape(&mut e, id, spec.clone(), [0.0, 0.0, 1.0]);
        let layer = e.document(id).unwrap().layers[1].clone();
        let pixels = layer.pixels.unwrap();
        let (ox, oy) = (layer.transform.origin.x, layer.transform.origin.y);
        let (mut worst, mut edges) = (0u8, 0);
        for y in 0..pixels.height { for x in 0..pixels.width {
            // The layer's pixel (x, y) covers document [ox + x, ox + x + 1): count there.
            let expected = area(0, 0, |u, v| inside(ox + x as f64 + u, oy + y as f64 + v));
            let want = (expected * 255.0).round() as u8;
            let got = pixels.pixel(x, y)[3];
            if want != 0 && want != 255 { edges += 1; }
            worst = worst.max(got.abs_diff(want));
            // Blue alone, premultiplied: the colour is the coverage.
            assert_eq!(pixels.pixel(x, y)[2], got);
        }}
        assert!(worst <= 4, "{spec:?}: worst {worst}");
        assert!(edges > 20, "{spec:?}: the fixture has soft edges to compare ({edges})");
    }
}

#[test]
fn a_line_lies_in_its_ends_box_grown_by_half_its_width_with_its_ends_as_fractions() {
    // Line 3 of the shapes probe (engine/tests/mac_probes.rs): 15 wide from (262.5, 127.5) to
    // (402.5, 207.5), in a document large enough.
    let mut e = Engine::new();
    let id = e.new_document(440, 280, false).unwrap();
    shape(&mut e, id, ShapeSpec::Line { start: p(262.5, 127.5), end: p(402.5, 207.5), width: 15.0 }, [0.0, 128.0 / 255.0, 0.0]);
    let doc = e.document(id).unwrap();
    let layer = doc.layers.last().unwrap();
    assert_eq!(layer.name, "Line 1");
    // (262.5 - 7.5, 127.5 - 7.5) and 140 + 15 by 80 + 15.
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width, layer.transform.size.height), (255.0, 120.0, 155.0, 95.0));
    // The record as the probe writes it (Swift's whole numbers without a fraction): the ends at
    // 7.5 / 155 and 7.5 / 95 from the box's corners.
    let expected = serde_json::json!({ "kind": "Line", "red": 0, "green": 128.0 / 255.0, "blue": 0, "cornerRadius": 0, "lineWidth": 15,
        "start": [7.5 / 155.0, 7.5 / 95.0], "end": [147.5 / 155.0, 87.5 / 95.0] });
    assert_eq!(layer.extra.shape.as_ref().unwrap(), &expected);
    // A rectangle's record carries no line fields.
    let mut e2 = Engine::new();
    let id2 = e2.new_document(50, 50, false).unwrap();
    shape(&mut e2, id2, ShapeSpec::Rectangle { rect: rect(1.0, 2.0, 30.0, 20.0), corner_radius: 2.5 }, [0.25, 0.5, 1.0]);
    assert_eq!(e2.document(id2).unwrap().layers.last().unwrap().extra.shape.as_ref().unwrap(),
        &serde_json::json!({ "kind": "Rectangle", "red": 0.25, "green": 0.5, "blue": 1, "cornerRadius": 2.5 }));
}

#[test]
fn a_line_of_width_one_is_a_one_pixel_row_with_round_ends() {
    // Line 1 of the probe: 1 wide from (20.5, 120.5) to (140.5, 120.5): a 121 x 1 layer at (20, 120).
    let mut e = Engine::new();
    let id = e.new_document(200, 200, false).unwrap();
    shape(&mut e, id, ShapeSpec::Line { start: p(20.5, 120.5), end: p(140.5, 120.5), width: 1.0 }, [0.0, 0.0, 0.0]);
    let layer = e.document(id).unwrap().layers.last().unwrap().clone();
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.pixels.as_ref().unwrap().width, layer.pixels.as_ref().unwrap().height), (20.0, 120.0, 121, 1));
    let px = layer.pixels.unwrap();
    // Full along the line; each end pixel holds half of a half-pixel cap plus half a pixel of line:
    // 0.5 + pi / 8 of it (0.8927 = 228), where a square cap would fill it and a butt cap halve it.
    assert_eq!(px.pixel(60, 0)[3], 255);
    for x in [0, 120] { assert!(px.pixel(x, 0)[3].abs_diff(228) <= 3, "end pixel {x}: {:?}", px.pixel(x, 0)); }
}

#[test]
fn a_shape_is_named_past_the_names_taken_and_placed_above_the_active_layer() {
    let (mut e, id) = session();
    run(&mut e, id, Command::AddBlankLayer); // "Layer 2", active, on top
    let bottom = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::RenameLayer { id: bottom, name: "Ellipse 1".into() });
    run(&mut e, id, Command::SetActiveLayer { id: Some(bottom) });
    shape(&mut e, id, ShapeSpec::Ellipse { rect: rect(1.0, 1.0, 5.0, 5.0) }, RED);
    assert_eq!(names(&e, id), ["Ellipse 1", "Ellipse 2", "Layer 2"], "above the active layer, not on top; the name taken skipped");
}

#[test]
fn a_click_or_a_shape_too_large_makes_nothing() {
    let (mut e, id) = session();
    let depth = e.state(id).unwrap().undo_depth;
    // Under a pixel on a side: a click, or a line along an axis of width under 1 (the Mac's
    // `rect.width >= 1, rect.height >= 1`).
    for spec in [ShapeSpec::Rectangle { rect: rect(20.0, 20.0, 0.0, 0.0), corner_radius: 0.0 },
        ShapeSpec::Ellipse { rect: rect(20.0, 20.0, 30.0, 0.5) },
        ShapeSpec::Line { start: p(10.0, 10.0), end: p(50.0, 10.0), width: 0.5 }] {
        assert!(matches!(e.execute(id, Command::AddShape { shape: spec, color: RED }), Err(CommandError::Argument(_))));
    }
    // Past the project's 100 megapixels: the Mac's words.
    let big = ShapeSpec::Rectangle { rect: rect(0.0, 0.0, 20_000.0, 6_000.0), corner_radius: 0.0 };
    match e.execute(id, Command::AddShape { shape: big, color: RED }) {
        Err(CommandError::Refused(m)) => assert_eq!(m, SHAPE_TOO_LARGE),
        other => panic!("{other:?}"),
    }
    assert_eq!(e.state(id).unwrap().undo_depth, depth);
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
}

#[test]
fn painting_over_a_shape_layer_drops_its_record_and_a_save_keeps_it_otherwise() {
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 30.0, 20.0), corner_radius: 4.0 }, RED);
    let layer = e.state(id).unwrap().layers[1].id;
    let files = e.save_package(id).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&files.manifest_json).unwrap();
    let saved = manifest["layers"].as_array().unwrap().iter().find(|l| l["name"] == "Rectangle 1").unwrap();
    assert_eq!(saved["shape"]["cornerRadius"], serde_json::json!(4), "the Mac reads the record from the manifest");
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.0, 1.0, 0.0] });
    assert!(e.document(id).unwrap().layers[1].extra.shape.is_none(), "no longer the shape the Mac would redraw");
}
