//! Gaussian and Motion Blur adjustment layers (R 3.4): everything composited so far, blurred as a
//! whole with the canvas edge as a transparent boundary, then put back through the layer's coverage.
use compositor_engine::*;

/// An oblong premultiplied pattern: colour varies by column and row, a third of the cells are
/// clear and the right half is translucent, so a blur that skipped clear pixels, ignored alpha or
/// flipped an axis would show.
fn pattern(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let a: u32 = if (x / 5 + y / 3) % 3 == 0 { 0 } else if x < width / 2 { 255 } else { 170 };
        let (r, g, b) = (x * 255 / width, y * 255 / height, (x + 2 * y) * 7 % 256);
        data.extend_from_slice(&[(r * a / 255) as u8, (g * a / 255) as u8, (b * a / 255) as u8, a as u8]);
    }}
    Raster::from_premultiplied(width, height, data)
}
fn blur(doc: &Document, radius: f64) -> Layer {
    let mut layer = Layer::blank("Blur", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::GaussianBlur);
    a.blur_radius = Some(radius);
    layer.extra.adjustment = Some(a);
    layer
}
fn streak(doc: &Document, angle: f64, distance: f64) -> Layer {
    let mut layer = Layer::blank("Streak", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::MotionBlur);
    a.motion_angle = Some(angle); a.motion_distance = Some(distance);
    layer.extra.adjustment = Some(a);
    layer
}
fn full(doc: &Document) -> Raster {
    composite(doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height)
}
/// What lies beneath the top layer (the adjustment under test).
fn beneath(doc: &Document) -> Raster { let mut d = doc.clone(); d.layers.pop(); full(&d) }
fn patterned(top: impl FnOnce(&Document) -> Vec<Layer>) -> Document {
    let mut doc = Document::new(40, 24);
    let mut layers = vec![Layer::with_pixels("P", pattern(40, 24), Point { x: 0.0, y: 0.0 })];
    layers.extend(top(&doc));
    doc.layers = layers;
    doc
}
/// A smooth premultiplied ramp: red rises across x, clear above row `edge` (None: opaque).
fn ramp(width: u32, height: u32, edge: Option<u32>) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let a: u32 = match edge { Some(e) if y < e => 0, _ => 255 };
        data.extend_from_slice(&[(x * 255 / (width - 1) * a / 255) as u8, (90 * a / 255) as u8, (40 * a / 255) as u8, a as u8]);
    }}
    Raster::from_premultiplied(width, height, data)
}
/// Per-pixel noise from a fixed linear congruential generator, half of it at alpha 90: detail
/// finer than any reduced copy. The measured bounds below hold for exactly these bytes.
fn lcg_noise(width: u32, height: u32) -> Raster {
    let mut s: u32 = 12345;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width * height {
        let mut next = || { s = s.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7fff_ffff; ((s >> 16) & 0xff) as u8 };
        let (r, g, b) = (next(), next(), next());
        let a: u32 = if r & 1 == 0 { 255 } else { 90 };
        data.extend_from_slice(&[(r as u32 * a / 255) as u8, (g as u32 * a / 255) as u8, (b as u32 * a / 255) as u8, a as u8]);
    }
    Raster::from_premultiplied(width, height, data)
}
/// `canvas` at the origin of a document its own size, under one layer built by `top`.
fn over(canvas: Raster, top: impl FnOnce(&Document) -> Layer) -> Document {
    let mut doc = Document::new(canvas.width, canvas.height);
    let layer = top(&doc);
    doc.layers = vec![Layer::with_pixels("Canvas", canvas, Point { x: 0.0, y: 0.0 }), layer];
    doc
}

#[test]
fn a_gaussian_blur_layer_blurs_everything_beneath_it_alpha_included() {
    // Radius 3 reaches 9 output pixels, under SPATIAL_REACH_LIMIT: the exact kernel runs.
    let doc = patterned(|d| vec![blur(d, 3.0)]);
    let under = beneath(&doc);
    assert_eq!(full(&doc).bytes(), gaussian_blur(&under, 3.0).bytes());
    assert_ne!(full(&doc).bytes(), under.bytes(), "the blur changed something");
}

#[test]
fn a_motion_blur_layer_streaks_everything_beneath_it() {
    let doc = patterned(|d| vec![streak(d, 30.0, 9.0)]);
    assert_eq!(full(&doc).bytes(), motion_blur(&beneath(&doc), 30.0, 9.0).bytes());
}

#[test]
fn what_lies_off_the_canvas_never_reaches_the_blur() {
    // An opaque white block from x = -16 to 32, over a 64 x 20 canvas. The Mac blurs a canvas-sized
    // context, so the off-canvas part is not there to spread back in, and the canvas edge fades.
    let mut doc = Document::new(64, 20);
    let block = Layer::with_pixels("Block", Raster::from_premultiplied(48, 20, [255u8, 255, 255, 255].repeat(960)), Point { x: -16.0, y: 0.0 });
    doc.layers = vec![block, blur(&doc, 4.0)];
    let out = full(&doc);
    let radius = 12i64;
    let weight = |i: i64| (-(i * i) as f64 / 32.0).exp();
    let total: f64 = (-radius..=radius).map(weight).sum();
    let share = |at: i64, len: i64| (-radius..=radius).filter(|i| (0..len).contains(&(at + i))).map(weight).sum::<f64>() / total;
    for x in [0u32, 5, 20, 31, 32, 40] {
        let want = 255.0 * share(x as i64, 32) * share(10, 20);
        let got = out.pixel(x, 10)[3] as f64;
        assert!((got - want).abs() <= 1.0, "alpha at x {x}: {got} vs {want}");
    }
    assert!(out.pixel(0, 10)[3] < 200, "the canvas edge fades");
}

#[test]
fn a_region_reaching_past_the_canvas_blurs_only_the_canvas_part() {
    // The same block, composited over x = -8..72 (the engine API accepts a region past the
    // canvas). Its canvas part is the whole-canvas composite: the off-canvas white never reaches
    // the blur. And the blur writes nothing outside the canvas, so what lies there is exactly
    // what the layers put there.
    let mut doc = Document::new(64, 20);
    let block = Layer::with_pixels("Block", Raster::from_premultiplied(48, 20, [255u8, 255, 255, 255].repeat(960)), Point { x: -16.0, y: 0.0 });
    doc.layers = vec![block, blur(&doc, 4.0)];
    let region = Rect { x: -8.0, y: 0.0, width: 80.0, height: 20.0 };
    let wide = composite(&doc, region, 80, 20);
    let mut plain = doc.clone(); plain.layers.pop();
    let bare = composite(&plain, region, 80, 20);
    assert_eq!(wide.cropped(8, 0, 64, 20).bytes(), full(&doc).bytes(), "the canvas part");
    assert_ne!(wide.cropped(8, 0, 64, 20).bytes(), bare.cropped(8, 0, 64, 20).bytes(), "the blur changed the canvas part");
    for y in 0..20 { for x in (0..8).chain(72..80) {
        assert_eq!(wide.pixel(x, y), bare.pixel(x, y), "outside the canvas at ({x}, {y})");
    }}
}

#[test]
fn a_part_of_the_canvas_composites_exactly_as_that_part_of_the_whole() {
    let doc = patterned(|d| vec![blur(d, 2.0), streak(d, 30.0, 7.0)]);
    let whole = full(&doc);
    for (x, y, w, h) in [(10u32, 5u32, 12u32, 8u32), (0, 0, 7, 24), (33, 17, 7, 7), (20, 12, 1, 1)] {
        let part = composite(&doc, Rect { x: x as f64, y: y as f64, width: w as f64, height: h as f64 }, w, h);
        assert_eq!(part.bytes(), whole.cropped(x, y, w, h).bytes(), "({x}, {y}) {w} x {h}");
    }
    // The streak pads by its three sigmas (7 / sqrt(12) each) plus 2, as far as its kernel reads.
    assert_eq!(render_plan(&doc, None).spatial_margin, (2.0 * 3.0 + 2.0) + (7.0 / 12f64.sqrt() * 3.0 + 2.0));
    let mut hidden = doc.clone();
    hidden.layers[2].visible = false;
    assert_eq!(render_plan(&hidden, None).spatial_margin, 2.0 * 3.0 + 2.0, "a hidden blur reaches nothing");
}

#[test]
fn a_part_of_the_canvas_halves_on_the_same_lattice_as_the_whole() {
    // Ruling F-I1 + F-I2. Every region here starts on an odd output pixel, at one to three halvings,
    // single and stacked, and the lattice anchored at the canvas corner still cuts it into the
    // whole canvas's blocks. Measured 0 on every one (p35b-halving, 2026-09-24); with the lattice
    // anchored at the padded region's own corner instead, (77, 41) at radius 20 was off by 1.
    let mut docs = vec![
        over(lcg_noise(200, 150), |d| blur(d, 20.0)),
        over(lcg_noise(200, 150), |d| blur(d, 40.0)),
        over(lcg_noise(200, 150), |d| streak(d, 30.0, 150.0)),
    ];
    let mut stacked = over(lcg_noise(200, 150), |d| blur(d, 7.0));
    let second = blur(&stacked, 40.0); stacked.layers.push(second);
    docs.push(stacked);
    // Wide enough that the pad (169) reaches no canvas edge from the middle regions.
    let mut wide = over(lcg_noise(600, 400), |d| blur(d, 20.0));
    let second = streak(&wide, 30.0, 150.0); wide.layers.push(second);
    docs.push(wide);
    for doc in &docs {
        let whole = full(doc);
        let (w, h) = (doc.width, doc.height);
        for (x, y, rw, rh) in [(77u32, 41u32, 40u32, 30u32), (33, 17, 1, 1), (13, 5, 21, 11), (0, 0, 7, h), (w - 7, h - 7, 7, 7),
            (301, 199, 1, 1), (299, 177, 3, 5), (277, 141, 40, 30)] {
            if x + rw > w || y + rh > h { continue; }
            let part = composite(doc, Rect { x: x as f64, y: y as f64, width: rw as f64, height: rh as f64 }, rw, rh);
            assert_eq!(part.bytes(), whole.cropped(x, y, rw, rh).bytes(), "{w} x {h}, {} layers: ({x}, {y}) {rw} x {rh}", doc.layers.len());
        }
    }
}

#[test]
fn the_grid_and_the_span_follow_the_plans_blurs() {
    let one = over(lcg_noise(200, 150), |d| blur(d, 20.0));
    assert_eq!(spatial_blur(one.layers[1].extra.adjustment.as_ref().unwrap(), 1.0), SpatialBlur { level: 1, sigma: 20.0, distance: 0.0, angle: 0.0 });
    // Margin 3 x 20 + 2 = 62, plus three cells of 2.
    assert_eq!(spatial_grid(&render_plan(&one, None), 1.0), SpatialGrid { cell: 2, pad: 68 });
    // A quarter-size render: sigma 5 reaches 15, no halving; 62 / 4 + 3 = 18.5, rounded up.
    assert_eq!(spatial_grid(&render_plan(&one, None), 0.25), SpatialGrid { cell: 1, pad: 19 });
    let mut two = one.clone();
    let second = streak(&two, 30.0, 150.0); two.layers.push(second);
    // Stacked blurs compound: 68 + (motion_reach(150) + 2 + 3 x 4) = 211.9, rounded up; the streak's
    // reach of 129.9 (three sigmas of 150 / sqrt(12)) halves twice past SPATIAL_REACH_LIMIT, and
    // the cell is the larger level's.
    let reach = motion_reach(150.0);
    assert_eq!(spatial_level(reach), 2);
    assert_eq!(spatial_grid(&render_plan(&two, None), 1.0), SpatialGrid { cell: 4, pad: (68.0 + reach + 2.0 + 12.0).ceil() as u32 });
    let mut far = one.clone();
    far.layers[1].extra.adjustment.as_mut().unwrap().blur_radius = Some(250.0);
    // At 2 output pixels per document pixel, radius 250 reaches 1500: five halvings, and the pad
    // (2 x 752 + 3 x 32) stops at SPATIAL_PAD_LIMIT.
    assert_eq!(spatial_grid(&render_plan(&far, None), 2.0), SpatialGrid { cell: 32, pad: 1024 });
    // spatial_span, in output pixels from the canvas's leading edge.
    assert_eq!(spatial_span(77, 117, 200, 4, 10), (64, 128), "grown by the pad, then out to the lattice");
    assert_eq!(spatial_span(77, 117, 200, 4, 100), (0, 200), "never past the canvas");
    assert_eq!(spatial_span(0, 63, 63, 2, 0), (0, 64), "the canvas's own far edge rounds out to the lattice, transparent beyond");
    assert_eq!(spatial_span(-8, 72, 64, 1, 5), (-8, 72), "a request past the canvas keeps its ends");
    assert_eq!(spatial_span(-7, 10, 64, 4, 5), (-8, 16), "... rounded out to the lattice");
}

#[test]
fn a_mask_limits_where_the_blur_lands_but_not_what_it_reads() {
    let mut doc = patterned(|d| vec![blur(d, 3.0)]);
    let mask: Vec<u8> = (0..24).flat_map(|_| (0..40).map(|x| if x < 20 { 0u8 } else { 255 })).collect();
    doc.layers[1].mask = Some(Mask { pixels: GrayRaster::from_bytes(40, 24, mask), enabled: true, placement: None, linked: None });
    let (under, out) = (beneath(&doc), full(&doc));
    let blurred = gaussian_blur(&under, 3.0);
    for y in 0..24 { for x in 0..40 {
        let want = if x < 20 { under.pixel(x, y) } else { blurred.pixel(x, y) };
        assert_eq!(out.pixel(x, y), want, "({x}, {y})");
    }}
}

#[test]
fn a_dimmed_folder_moves_each_pixel_part_way_to_the_blur() {
    let mut doc = Document::new(40, 24);
    let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true; folder.opacity = 0.4;
    let mut b = blur(&doc, 3.0); b.parent_id = Some(folder.id);
    doc.layers = vec![Layer::with_pixels("P", pattern(40, 24), Point { x: 0.0, y: 0.0 }), folder, b];
    let mut plain = doc.clone(); plain.layers.truncate(1);
    let under = full(&plain);
    let blurred = gaussian_blur(&under, 3.0);
    let out = full(&doc);
    for (i, (o, (u, b))) in out.bytes().iter().zip(under.bytes().iter().zip(blurred.bytes())).enumerate() {
        let want = (*u as f32 + (*b as f32 - *u as f32) * 0.4).round();
        assert!((*o as f32 - want).abs() <= 1.0, "byte {i}: {o} vs {want}");
    }
}

#[test]
fn a_blend_mode_blends_at_full_coverage_and_keeps_the_original_alpha() {
    // LiveMaskRenderer.swift:24-46: both images made opaque (a clear pixel becomes black), blended in
    // the layer's mode, then the original alpha put back. The expectation below restates
    // `blended_keeping_alpha` step for step, so it cannot catch a misreading the two share; the
    // Task 12 cgmode probes are the Mac's word on it.
    let mut doc = patterned(|d| vec![blur(d, 3.0)]);
    doc.layers[1].blend_mode = BlendMode::Multiply;
    let (under, out) = (beneath(&doc), full(&doc));
    let blurred = gaussian_blur(&under, 3.0);
    let opaque = |p: [u8; 4]| -> [f32; 3] {
        let a = p[3] as u32;
        [0, 1, 2].map(|c| if a == 0 { 0.0 } else { ((p[c] as u32 * 255 + a / 2) / a).min(255) as f32 / 255.0 })
    };
    for y in 0..24 { for x in 0..40 {
        let (o, b) = (under.pixel(x, y), blurred.pixel(x, y));
        let (co, cb) = (opaque(o), opaque(b));
        let a = o[3] as u32;
        let want = [0, 1, 2].map(|c| ((((co[c] * cb[c]).clamp(0.0, 1.0) * 255.0).round() as u32 * a + 127) / 255) as u8);
        assert_eq!(out.pixel(x, y), [want[0], want[1], want[2], o[3]], "({x}, {y})");
    }}
}

#[test]
fn a_blur_in_a_core_image_only_mode_keeps_the_original_alpha_as_the_mac_does() {
    // Ruling E-I1. LiveMaskRenderer.swift:36-61 (v1.4.5) branches on the layer's OWN mode: a blur
    // layer in Linear Burn takes the full-coverage path, blended in Linear Burn itself (through Core
    // Image, :52-57; 1.2.10 drew it as Normal) over the opaque original, then gets the original alpha
    // back. So the canvas edge does not fade, the colour is Linear Burn of the colour and its blur (of
    // a flat colour: that colour, so 2 c - 255, at least 0), and nothing spreads where the original is
    // clear. Taking the Normal path instead gives alpha 139 at x = 0 and [3, 1, 0, 4] at x = 40
    // (measured).
    let mut doc = Document::new(64, 20);
    let block = Layer::with_pixels("Block", Raster::from_premultiplied(48, 20, [200u8, 90, 30, 255].repeat(960)), Point { x: -16.0, y: 0.0 });
    let mut b = blur(&doc, 4.0);
    b.blend_mode = BlendMode::LinearBurn;
    doc.layers = vec![block, b];
    let plan = render_plan(&doc, None);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the blur is a plain node") };
    assert!(draw.keeps_alpha && draw.blend == BlendMode::LinearBurn);
    let out = full(&doc);
    for x in [0u32, 5, 31] {
        let p = out.pixel(x, 10);
        assert_eq!(p[3], 255, "alpha at x {x} is the original's: {p:?}");
        for (c, want) in [200i32, 90, 30].map(|v| (2 * v - 255).max(0)).into_iter().enumerate() {
            assert!((p[c] as i32 - want).abs() <= 1, "colour at x {x}, channel {c}: {p:?}");
        }
    }
    assert_eq!(out.pixel(40, 10), [0, 0, 0, 0], "where the original is clear it stays clear");
}

#[test]
fn a_long_reach_blurs_a_halved_copy_within_the_measured_bound_of_the_exact_kernel() {
    assert_eq!([spatial_level(48.0), spatial_level(48.5), spatial_level(96.0), spatial_level(96.5), spatial_level(750.0)], [0, 1, 1, 2, 4]);
    // Ruling D-B1. The exact kernel is level 0 on the canvas-sized composite, the Mac's own
    // definition. Each bound was MEASURED on exactly these fixtures with this task's spatial.rs on
    // the canvas-anchored lattice (plan-fix scratch crate p35b-halving, 2026-09-24), not chosen;
    // the assertion allows 1 more for float ordering. `inset` keeps the comparison reach + 2 cells
    // away from every canvas edge (0: the whole canvas). Radius 20 reaches 60 output pixels (one
    // halving), radius 40 reaches 120 (two). A 150 px Motion Blur reaches three sigmas, 129.9 (two
    // halvings, as a Gaussian would: Phase 4a's kernel, bounds re-measured on p4a-scratch
    // 2026-09-27; the even streak before it halved three times, and measured 2, 17 and 20).
    // Ruling M12: the 150 px Motion Blur's inset by the same formula as the others (reach + 2 cells,
    // rounded up) instead of the literal 138.
    let motion_150_reach = motion_reach(150.0);
    let motion_150_inset = (motion_150_reach + 2.0 * (1u32 << spatial_level(motion_150_reach)) as f64).ceil() as u32;
    let cases: [(&str, Document, u32, u8); 9] = [
        ("the interior of an opaque ramp, radius 20", over(ramp(200, 150, None), |d| blur(d, 20.0)), 64, 1),
        ("the same ramp out to its canvas edge, radius 20", over(ramp(200, 150, None), |d| blur(d, 20.0)), 0, 3),
        ("the same ramp out to its canvas edge, radius 40", over(ramp(200, 150, None), |d| blur(d, 40.0)), 0, 4),
        ("an alpha edge off the halving lattice (row 7), radius 20", over(ramp(96, 64, Some(7)), |d| blur(d, 20.0)), 0, 3),
        ("odd sizes, 95 x 63, alpha edge at row 7, radius 20", over(ramp(95, 63, Some(7)), |d| blur(d, 20.0)), 0, 3),
        ("odd sizes, radius 40", over(ramp(95, 63, Some(7)), |d| blur(d, 40.0)), 0, 2),
        // Most of a halved Motion Blur's error lies at the canvas edge, where the blur fades.
        ("the interior of an opaque ramp under a 150 px Motion Blur", over(ramp(400, 300, None), |d| streak(d, -60.0, 150.0)), motion_150_inset, 1),
        ("a 150 px Motion Blur over the odd-sized ramp", over(ramp(95, 63, Some(7)), |d| streak(d, -60.0, 150.0)), 0, 9),
        // A halved Motion Blur also averages the detail ACROSS its angle, which the exact one keeps.
        ("a 150 px Motion Blur over per-pixel noise", over(lcg_noise(200, 150), |d| streak(d, 30.0, 150.0)), 0, 19),
    ];
    for (name, doc, inset, measured) in cases {
        let (under, out) = (beneath(&doc), full(&doc));
        let a = doc.layers[1].extra.adjustment.as_ref().unwrap();
        let exact = match a.kind {
            AdjustmentKind::GaussianBlur => gaussian_blur(&under, a.gaussian_radius()),
            _ => motion_blur(&under, a.motion_angle_degrees(), a.motion_distance_pixels()),
        };
        assert_ne!(out.bytes(), exact.bytes(), "{name}: the halved path ran; the exact kernel would match to the bit");
        let mut worst = 0u8;
        for y in inset..doc.height - inset { for x in inset..doc.width - inset {
            let (p, q) = (out.pixel(x, y), exact.pixel(x, y));
            for c in 0..4 { worst = worst.max(p[c].abs_diff(q[c])); }
        }}
        assert!(worst <= measured + 1, "{name}: {worst} against the exact kernel, measured {measured}");
    }
}

#[test]
fn a_motion_blur_layer_halves_past_the_same_reach_as_a_gaussian() {
    // Phase 4a: the Motion Blur is CIMotionBlur's Gaussian along its angle, sigma = distance /
    // sqrt(12), reaching three sigmas, and runs as a row-wise stencil cheap enough to share the
    // Gaussian's SPATIAL_REACH_LIMIT (48). A 55 px blur reaches 47.6 (exact); 56 px reaches 48.5
    // (one halving).
    let exact = patterned(|d| vec![streak(d, 30.0, 55.0)]);
    let b = spatial_blur(exact.layers[1].extra.adjustment.as_ref().unwrap(), 1.0);
    assert_eq!((b.level, b.sigma, b.distance, b.angle), (0, 55.0 / 12f64.sqrt(), 55.0, 30.0));
    assert_eq!(full(&exact).bytes(), motion_blur(&beneath(&exact), 30.0, 55.0).bytes(), "the exact kernel on the composite");
    let halved = patterned(|d| vec![streak(d, 30.0, 56.0)]);
    assert_eq!(spatial_blur(halved.layers[1].extra.adjustment.as_ref().unwrap(), 1.0).level, 1);
    assert_ne!(full(&halved).bytes(), motion_blur(&beneath(&halved), 30.0, 56.0).bytes(), "drawn from a halved copy");
}

#[test]
fn a_part_at_the_cpu_renderers_own_zoom_halves_on_the_whole_frames_lattice() {
    // Review I1. cpu-renderer.ts:29-31 shows a 640 px wide document 217 device pixels across and
    // asks for the region `(vx0 - x) * docPerPx`, `docPerPx = 640 / 217`. At origin 5 that lands
    // one ulp below 5 output pixels; a floored origin put the lattice one pixel off, and the part
    // came out 2 to 4 levels from the whole (measured by the review).
    let doc = over(lcg_noise(640, 400), |d| blur(d, 80.0));
    let dpp = 640.0 / 217.0;
    // The whole document on screen, 217 x 136 device pixels, requested as the renderer does.
    let (w, h) = (217u32, 136u32);
    let whole = composite(&doc, Rect { x: 0.0, y: 0.0, width: w as f64 * dpp, height: h as f64 * dpp }, w, h);
    // Panned 5 and 27 device pixels into the document, the view shows the remaining 212 x 109.
    let (x, y, pw, ph) = (5u32, 27u32, 212u32, 109u32);
    let region = Rect { x: x as f64 * dpp, y: y as f64 * dpp, width: pw as f64 * dpp, height: ph as f64 * dpp };
    let (sx, sy) = (pw as f64 / region.width, ph as f64 / region.height);
    assert!(region.x * sx < 5.0 && region.y * sy < 27.0, "the renderer's arithmetic lands below the whole pixel: {} {}", region.x * sx, region.y * sy);
    assert_eq!(spatial_grid(&render_plan(&doc, None), sx).cell, 2, "radius 80 at this zoom halves once");
    let part = composite(&doc, region, pw, ph);
    assert_eq!(part.bytes(), whole.cropped(x, y, pw, ph).bytes());
}

#[test]
fn a_blur_zoomed_far_in_pads_its_frame_within_the_cell_limit() {
    // Review I2. A 2000 px streak at zoom 32 on a 2x display reaches 64000 output pixels. With no
    // cap on the halvings its cell was 8192, and a 1280 x 720 view padded out to 268 Mpx. Capped at
    // SPATIAL_CELL_LIMIT, the frame stays within the view plus the pad and a cell on each side.
    let (vw, vh) = (1280i64, 720i64);
    let side = |v: i64| v as f64 + 2.0 * SPATIAL_PAD_LIMIT + 2.0 * SPATIAL_CELL_LIMIT;
    let bound = side(vw) * side(vh);
    let mut streaked = Document::new(3000, 2000);
    let s = streak(&streaked, 30.0, 2000.0); streaked.layers = vec![s];
    let mut blurred = Document::new(3000, 2000);
    let b = blur(&blurred, 250.0); blurred.layers = vec![b];
    for doc in [&streaked, &blurred] {
        let plan = render_plan(doc, None);
        let a = doc.layers[0].extra.adjustment.as_ref().unwrap();
        for out_per_doc in [32.0, 64.0] {
            let level = spatial_blur(a, out_per_doc).level;
            assert!(level <= 8, "{:?} at {out_per_doc} out px per doc px: level {level}", a.kind);
            let grid = spatial_grid(&plan, out_per_doc);
            let (cw, ch) = ((3000.0 * out_per_doc) as i64, (2000.0 * out_per_doc) as i64);
            for k in 0..64i64 {
                let (x0, y0) = (cw / 2 + k * 97, ch / 3 + k * 61);
                let (xs, xe) = spatial_span(x0, x0 + vw, cw, grid.cell, grid.pad);
                let (ys, ye) = spatial_span(y0, y0 + vh, ch, grid.cell, grid.pad);
                let area = ((xe - xs) * (ye - ys)) as f64;
                assert!(area <= bound, "{:?} at {out_per_doc}, origin ({x0}, {y0}): {area} px against {bound}", a.kind);
            }
        }
    }
}

#[test]
fn a_blur_clipped_to_a_layer_changes_nothing_outside_that_layer() {
    // A clipped adjustment runs on the stack's own surface (LiveMaskRenderer.swift:98-101).
    let mut doc = Document::new(40, 24);
    let backdrop = Layer::with_pixels("Backdrop", Raster::from_premultiplied(40, 24, [200u8, 40, 40, 255].repeat(960)), Point { x: 0.0, y: 0.0 });
    let base = Layer::with_pixels("Base", pattern(16, 12), Point { x: 12.0, y: 6.0 });
    let mut b = blur(&doc, 2.0);
    b.mask_source_id = Some(base.id);
    doc.layers = vec![backdrop, base, b];
    let mut plain = doc.clone(); plain.layers.truncate(2);
    let (before, after) = (full(&plain), full(&doc));
    for y in 0..24 { for x in 0..40 {
        if !((12..28).contains(&x) && (6..18).contains(&y)) {
            assert_eq!(after.pixel(x, y), before.pixel(x, y), "outside the base at ({x}, {y})");
        }
    }}
    assert_ne!(after.bytes(), before.bytes(), "the blur did something inside the base");
}

#[test]
fn a_clipped_blur_reads_the_stack_surface_as_opaque_black_beyond_the_base() {
    // Audit E-M1. The Mac makes the whole stack surface opaque before the children draw
    // (layer_unpremultiply_opaque, BrushPixels.c:23-35: a clear pixel becomes opaque black), so a
    // clipped blur in a mode other than Normal (Screen here: keeps_alpha) spreads black, not
    // transparency, in from beyond the base. The expectation blurs that surface with the exact
    // kernel (radius 2 stays at level 0) and screens it over the base colour. Measured: at the
    // four edge pixels the transparent surround the port used before gives [243, 148, 56] instead.
    let mut doc = Document::new(40, 24);
    let base = Layer::with_pixels("Base", Raster::from_premultiplied(16, 12, [200u8, 90, 30, 255].repeat(192)), Point { x: 12.0, y: 6.0 });
    let mut b = blur(&doc, 2.0);
    b.mask_source_id = Some(base.id);
    b.blend_mode = BlendMode::Screen;
    doc.layers = vec![base, b];
    let inside = |x: u32, y: u32| (12..28).contains(&x) && (6..18).contains(&y);
    let surface: Vec<u8> = (0..24u32).flat_map(|y| (0..40u32).flat_map(move |x| if inside(x, y) { [200u8, 90, 30, 255] } else { [0, 0, 0, 255] })).collect();
    let blurred = gaussian_blur(&Raster::from_premultiplied(40, 24, surface), 2.0);
    let out = full(&doc);
    let base_colour = [200.0f32, 90.0, 30.0].map(|v| v / 255.0);
    for (x, y) in [(12u32, 6u32), (12, 11), (20, 6), (27, 17)] {
        let s = blurred.pixel(x, y);
        let want = [0, 1, 2].map(|c| { let cs = s[c] as f32 / 255.0; ((base_colour[c] + cs - base_colour[c] * cs) * 255.0).round() as i32 });
        let got = out.pixel(x, y);
        assert_eq!(got[3], 255, "({x}, {y})");
        for c in 0..3 { assert!((got[c] as i32 - want[c]).abs() <= 1, "({x}, {y}) channel {c}: {got:?} vs {want:?}"); }
    }
}

#[test]
fn merging_a_folder_with_a_blur_inside_keeps_what_the_canvas_showed() {
    let mut doc = Document::new(40, 24);
    let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true;
    let folder_id = folder.id;
    let mut p = Layer::with_pixels("P", pattern(40, 24), Point { x: 0.0, y: 0.0 }); p.parent_id = Some(folder_id);
    let mut b = blur(&doc, 3.0); b.parent_id = Some(folder_id);
    doc.layers = vec![folder, p, b];
    doc.active_layer_id = Some(folder_id);
    let shown = full(&doc);
    let mut without_blur = doc.clone(); without_blur.layers.pop();
    assert_ne!(shown.bytes(), full(&without_blur).bytes(), "the blur shows, so the merge has something to keep");
    ops::merge::merge(&mut doc, &[folder_id]).expect("a drawn blur does not block a merge");
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(full(&doc).bytes(), shown.bytes());
}
