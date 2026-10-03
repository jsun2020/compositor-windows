//! The Phase 4b-1 Mac probes (Phase 4.5, Task 1): `shapes.png`, `gradient-linear.png` and
//! `gradient-radial.png` exported by Compositor for Mac 1.3.7 on 2026-09-29, and
//! `gradient-over-colour-2.png` exported on 2026-09-30 by 1.3.7 or 1.4.5 (the user moved to 1.4.5
//! that day; neither the Shape tool nor the Gradient changed between the two, mac-1.4.5-delta.md 2.5
//! and 2.9), committed under tests/fixtures/mac-4b1-probes with the saved `shapes.comp`. That project
//! is format 11, which this build opens only from Task 5 on, so its manifest is read here as JSON.
//! The shapes are drawn by this port's `shape_raster` at the boxes the Mac saved; the gradients'
//! lines were dragged by hand and are not saved, so each is fitted from the export itself.
use compositor_engine::ops::shape::shape_raster;
use compositor_engine::*;
use serde_json::Value;

fn fixture(name: &str) -> String { format!("{}/tests/fixtures/mac-4b1-probes/{name}", env!("CARGO_MANIFEST_DIR")) }

/// The Mac's export: width, height and straight RGBA8 as the PNG stores it.
fn mac(name: &str) -> (u32, u32, Vec<u8>) {
    let rgba = image::load_from_memory(&std::fs::read(fixture(name)).unwrap()).unwrap().to_rgba8();
    (rgba.width(), rgba.height(), rgba.into_raw())
}

/// Each shape layer of the saved `shapes.comp`, bottom to top: its name, the shape its box and its
/// `shape` record describe (a line's ends are fractions of its box, ShapeTool.swift:136-145), and its
/// colour.
fn saved_shapes() -> Vec<(String, ShapeSpec, [f64; 3])> {
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(fixture("shapes.mac-1.3.7.comp/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], 11, "saved by a Mac that writes format 11");
    manifest["layers"].as_array().unwrap().iter().filter(|l| l.get("shape").is_some()).map(|l| {
        let (t, s) = (&l["transform"], &l["shape"]);
        let n = |v: &Value| v.as_f64().unwrap();
        let rect = Rect { x: n(&t["origin"][0]), y: n(&t["origin"][1]), width: n(&t["size"][0]), height: n(&t["size"][1]) };
        let spec = match s["kind"].as_str().unwrap() {
            "Ellipse" => ShapeSpec::Ellipse { rect },
            "Rectangle" => ShapeSpec::Rectangle { rect, corner_radius: n(&s["cornerRadius"]) },
            "Line" => {
                let at = |key: &str| Point { x: rect.x + n(&s[key][0]) * rect.width, y: rect.y + n(&s[key][1]) * rect.height };
                ShapeSpec::Line { start: at("start"), end: at("end"), width: n(&s["lineWidth"]) }
            }
            other => panic!("no shape {other}"),
        };
        (l["name"].as_str().unwrap().to_string(), spec, [n(&s["red"]), n(&s["green"]), n(&s["blue"])])
    }).collect()
}

#[test]
fn the_mac_saved_the_shapes_the_probe_asked_for() {
    // mac_probes.rs `shapes_doc`: the boxes it wrote, which the Mac redrew and saved as they were.
    let shapes = saved_shapes();
    let boxes: Vec<(&str, [f64; 4])> = shapes.iter().map(|(name, spec, _)| { let b = spec.bounds(); (name.as_str(), [b.x, b.y, b.width, b.height]) }).collect();
    assert_eq!(boxes, [("Ellipse 1", [10.0, 10.0, 101.0, 61.0]), ("Rectangle 1", [130.0, 10.0, 120.0, 80.0]), ("Rectangle 2", [270.0, 10.0, 150.0, 40.0]),
        ("Line 1", [20.0, 120.0, 121.0, 1.0]), ("Line 2", [158.0, 108.0, 74.0, 74.0]), ("Line 3", [255.0, 120.0, 155.0, 95.0])]);
    let ShapeSpec::Line { start, end, width } = &shapes[5].1 else { panic!("Line 3 is a line") };
    assert!((start.x - 262.5).abs() < 1e-9 && (start.y - 127.5).abs() < 1e-9 && (end.x - 402.5).abs() < 1e-9 && (end.y - 207.5).abs() < 1e-9 && *width == 15.0);
}

#[test]
fn every_shape_matches_the_mac_render_within_its_own_measured_edge_bound() {
    // This port's shapes over white, composited as the Mac exported them. Per-shape bounds, measured
    // on p45-scratch (2026-09-30): the ellipse 10, the two rounded rectangles 4, Line 2 6 and Line 3
    // 7 (all on anti-aliased edge pixels; 4b-1's OQ11 held the port within 4 of an exact area count, and Core
    // Graphics' own coverage is not an exact area); Line 1 is exact but at its two end pixels, where
    // Core Graphics fills the round cap's sub-pixel sliver fully (29 levels darker). Nothing differs
    // outside the shapes' boxes.
    let (w, h, theirs) = mac("shapes.mac-1.3.7.png");
    let shapes = saved_shapes();
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::with_pixels("Background", Raster::from_premultiplied(w, h, vec![255; (w * h * 4) as usize]), Point { x: 0.0, y: 0.0 })];
    for (name, spec, color) in &shapes {
        let b = spec.bounds();
        doc.layers.push(Layer::with_pixels(name, shape_raster(spec, *color), Point { x: b.x, y: b.y }));
    }
    let ours = composite(&doc, Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, w, h).to_straight();
    let at = |x: u32, y: u32, c: usize| ((y * w + x) * 4) as usize + c;
    let inside = |spec: &ShapeSpec, x: u32, y: u32| { let b = spec.bounds(); x as f64 + 1.0 > b.x && (x as f64) < b.max_x() && y as f64 + 1.0 > b.y && (y as f64) < b.max_y() };
    let bound = |name: &str| match name { "Ellipse 1" => 10, "Rectangle 1" | "Rectangle 2" => 4, "Line 2" => 6, "Line 3" => 7, "Line 1" => 0, other => panic!("{other}") };
    for (name, spec, _) in &shapes {
        let mut worst = (0u8, (0, 0));
        for y in 0..h { for x in 0..w {
            if !inside(spec, x, y) { continue; }
            // Line 1's two end pixels are checked below.
            if name == "Line 1" && (x == 20 || x == 140) { continue; }
            for c in 0..4 { let d = ours[at(x, y, c)].abs_diff(theirs[at(x, y, c)]); if d > worst.0 { worst = (d, (x, y)); } }
        }}
        assert!(worst.0 <= bound(name), "{name}: {} at {:?}, bound {}", worst.0, worst.1, bound(name));
    }
    // Line 1 (red, 1 px, y 120.5 from x 20.5 to 140.5): each end pixel holds half a pixel of line and a
    // half-disc cap of radius 0.5, 0.5 + pi / 8 of the pixel, which the port covers within 4b-1's OQ11's 4
    // levels; the Mac covers more.
    for x in [20, 140] {
        let (mac_green, port_green) = (theirs[at(x, 120, 1)], ours[at(x, 120, 1)]);
        let exact = 255.0 * (1.0 - (0.5 + std::f64::consts::PI / 8.0));
        assert!((port_green as f64 - exact).abs() <= 4.0, "the port's end pixel at x {x}: {port_green}, the area {exact:.1} (4b-1's OQ11: within 4)");
        assert!(mac_green < port_green && port_green - mac_green <= 29, "Line 1 at x {x}: the Mac {mac_green}, the port {port_green}");
    }
    let outside = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| !shapes.iter().any(|(_, s, _)| inside(s, x, y)))
        .map(|(x, y)| (0..4).map(|c| ours[at(x, y, c)].abs_diff(theirs[at(x, y, c)])).max().unwrap()).max().unwrap();
    assert_eq!(outside, 0, "white where no shape is");
}

/// Least squares of `ys` on `xs`, each row of `xs` one sample's regressors: the coefficients.
fn least_squares(xs: &[Vec<f64>], ys: &[f64]) -> Vec<f64> {
    let n = xs[0].len();
    // The normal equations, solved by Gauss-Jordan elimination with partial pivoting.
    let mut m = vec![vec![0.0; n + 1]; n];
    for (row, y) in xs.iter().zip(ys) {
        for i in 0..n { for j in 0..n { m[i][j] += row[i] * row[j]; } m[i][n] += row[i] * y; }
    }
    for col in 0..n {
        let pivot = (col..n).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs())).unwrap();
        m.swap(col, pivot);
        for r in 0..n { if r != col { let f = m[r][col] / m[col][col]; for c in col..=n { m[r][c] -= f * m[col][c]; } } }
    }
    (0..n).map(|i| m[i][n] / m[i][i]).collect()
}

/// The line a linear gradient runs along, from `t`, the fraction of the way along it, at each pixel
/// column's centre: `t = (x + 0.5 - s) / (e - s)` fitted to the columns where `t` lies inside (0, 1).
fn fit_ends(ts: &[(u32, f64)]) -> (f64, f64) {
    let inner: Vec<&(u32, f64)> = ts.iter().filter(|(_, t)| *t > 0.03 && *t < 0.97).collect();
    let c = least_squares(&inner.iter().map(|(x, _)| vec![*x as f64 + 0.5, 1.0]).collect::<Vec<_>>(), &inner.iter().map(|(_, t)| *t).collect::<Vec<_>>());
    let s = -c[1] / c[0];
    (s, s + 1.0 / c[0])
}

/// This port's composite of `doc` after the gradient, straight RGBA8, and the worst difference from
/// the Mac's export.
fn paint_and_compare(mut doc: Document, gradient: GradientSpec, theirs: &[u8]) -> u8 {
    let (w, h) = (doc.width, doc.height);
    let layer = doc.layers[0].id;
    doc.active_layer_id = Some(layer);
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    e.execute(id, Command::Gradient { id: layer, mask: false, gradient }).unwrap();
    let ours = e.composite(id, Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, w, h).unwrap().to_straight();
    assert_eq!(ours.len(), theirs.len());
    ours.iter().zip(theirs).map(|(a, b)| a.abs_diff(*b)).max().unwrap()
}

const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

#[test]
fn the_linear_gradient_matches_the_mac_render_along_the_line_fitted_from_it() {
    // Black to white on a blank 512 x 32 layer, dragged by hand from the left edge to the right with
    // Shift: every row alike, the line found from the ramp. Core Graphics steps its ramp (runs of four
    // equal levels); this port's exact ramp is within 1 level of it everywhere (measured on
    // p45-scratch, 2026-09-30).
    let (w, h, theirs) = mac("gradient-linear.mac-1.3.7.png");
    assert_eq!((w, h), (512, 32));
    let ts: Vec<(u32, f64)> = (0..w).map(|x| (x, theirs[((16 * w + x) * 4) as usize] as f64 / 255.0)).collect();
    let (s, e) = fit_ends(&ts);
    assert!(s.abs() < 8.0 && (e - 512.0).abs() < 12.0, "dragged from edge to edge: {s:.2} to {e:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::blank("Layer 1", doc.size())];
    let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: s, y: 16.0 }, end: Point { x: e, y: 16.0 }, from: BLACK, to: WHITE, opacity: 1.0 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 1, "worst {worst} along {s:.3} to {e:.3}");
}

#[test]
fn the_radial_gradient_matches_the_mac_render_about_the_centre_and_rim_fitted_from_it() {
    // Black at the centre to white at the rim on a blank 256 x 256 layer, dragged by hand from where
    // the guides cross (128, 128) to the guide at x 228. The value at a pixel is 255 d / r, d its
    // centre's distance from the gradient's: (x - cx)^2 + (y - cy)^2 = (v r / 255)^2, which is linear in
    // cx, cy, cx^2 + cy^2 and r^2, fitted where the ramp is inside (0, 1). Measured 2 levels at worst
    // (p45-scratch, 2026-09-30): Core Graphics steps its radial ramp as it does the linear one.
    let (w, h, theirs) = mac("gradient-radial.mac-1.3.7.png");
    assert_eq!((w, h), (256, 256));
    let (mut rows, mut ys) = (Vec::new(), Vec::new());
    for y in 0..h { for x in 0..w {
        let v = theirs[((y * w + x) * 4) as usize] as f64 / 255.0;
        if v <= 0.06 || v >= 0.94 { continue; }
        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
        rows.push(vec![2.0 * px, 2.0 * py, -1.0, v * v]);
        ys.push(px * px + py * py);
    }}
    let c = least_squares(&rows, &ys);
    let (cx, cy, r) = (c[0], c[1], c[3].sqrt());
    assert!((cx - 128.0).abs() < 5.0 && (cy - 128.0).abs() < 5.0 && (r - 100.0).abs() < 5.0, "dragged from the guides' crossing to the guide 100 px right: ({cx:.2}, {cy:.2}) r {r:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::blank("Layer 1", doc.size())];
    let gradient = GradientSpec { shape: GradientShape::Radial, start: Point { x: cx, y: cy }, end: Point { x: cx + r, y: cy }, from: BLACK, to: WHITE, opacity: 1.0 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 2, "worst {worst} about ({cx:.3}, {cy:.3}) r {r:.3}");
}

#[test]
fn the_translucent_gradient_over_a_colour_matches_the_mac_render_along_the_line_fitted_from_it() {
    // 0000FF to transparent at 37 % opacity, dragged by hand across a solid (204, 77, 51) layer,
    // painted into that layer (source-over, as the Mac's raster edit paints). The blue's alpha at a
    // column is 0.37 (1 - t), which the red channel shows: red = 204 (1 - alpha). Measured 1 level at
    // worst (p45-scratch, 2026-09-30).
    let (w, h, theirs) = mac("gradient-over-colour-2.mac-1.3.7-or-1.4.5.png");
    assert_eq!((w, h), (256, 32));
    let ts: Vec<(u32, f64)> = (0..w).map(|x| (x, 1.0 - (1.0 - theirs[((16 * w + x) * 4) as usize] as f64 / 204.0) / 0.37)).collect();
    let (s, e) = fit_ends(&ts);
    assert!(s.abs() < 16.0 && (e - 256.0).abs() < 16.0, "dragged from edge to edge: {s:.2} to {e:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::with_pixels("Layer 1", Raster::from_premultiplied(w, h, [204u8, 77, 51, 255].repeat((w * h) as usize)), Point { x: 0.0, y: 0.0 })];
    let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: s, y: 16.0 }, end: Point { x: e, y: 16.0 },
        from: [0.0, 0.0, 1.0, 1.0], to: [0.0, 0.0, 1.0, 0.0], opacity: 0.37 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 1, "worst {worst} along {s:.3} to {e:.3}");
}
