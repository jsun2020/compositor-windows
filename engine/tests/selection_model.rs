//! The selection's outline and its coverage (engine/src/selection): Core Graphics' winding fill of
//! the Mac's `DocumentSelection` (Selection.swift:7-49), its booleans and its Expand band. Expected
//! values come from the geometry (areas, Gaussian weights) or from Compositor for Mac's own tests
//! (CompositorTests/SelectionTests.swift, SelectionFeatherTests.swift).
use compositor_engine::selection::geometry::{band, combine, ellipse, polygon, rectangle, Boolean};
use compositor_engine::*;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }

/// The selection's coverage over a `width` x `height` canvas, 0-255, as the Mac's tests read it
/// (`coverage(width:height:)`): the clip placed on the canvas's own pixels.
fn coverage(selection: &Selection, width: u32, height: u32) -> GrayRaster {
    SelectionClip::new(selection, width, height).on_grid(&Affine::IDENTITY, width, height)
}
fn at(c: &GrayRaster, x: u32, y: u32) -> u8 { c.bytes()[(y * c.width + x) as usize] }

#[test]
fn a_whole_pixel_rectangle_covers_exactly_its_pixels_with_or_without_antialiasing() {
    // An oblong box off every axis of symmetry: 40 x 41 at (20, 30), on 100 x 100.
    for antialiased in [true, false] {
        let s = Selection::new(vec![rectangle(rect(20.0, 30.0, 40.0, 41.0))], antialiased, 0.0);
        let c = coverage(&s, 100, 100);
        for (x, y, want) in [(20, 30, 255), (19, 30, 0), (20, 29, 0), (59, 70, 255), (60, 70, 0), (59, 71, 0), (40, 50, 255)] {
            assert_eq!(at(&c, x, y), want, "({x}, {y}) antialiased {antialiased}");
        }
        assert_eq!(c.bytes().iter().filter(|&&v| v == 255).count(), 40 * 41);
        assert!(c.bytes().iter().all(|&v| v == 0 || v == 255));
    }
}

#[test]
fn antialiasing_controls_edge_coverage() {
    // SelectionTests.antialiasingControlsEdgeCoverage: the triangle (0,0), (100,0), (0,100). Its
    // edge x + y = 100 cuts each pixel (x, 99 - x) along its diagonal, so antialiased that pixel is
    // half covered: 255 x 0.5, rounded, 128. Aliased every pixel is 0 or 255.
    let triangle = vec![polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(0.0, 100.0)])];
    let soft = coverage(&Selection::new(triangle.clone(), true, 0.0), 100, 100);
    for x in 0..100 { assert_eq!(at(&soft, x, 99 - x), 128, "antialiased at ({x}, {})", 99 - x); }
    let hard = coverage(&Selection::new(triangle, false, 0.0), 100, 100);
    assert!(hard.bytes().iter().all(|&v| v == 0 || v == 255));
    for x in 0..99 { assert_eq!(at(&hard, x, 98 - x), 255, "inside the edge at ({x}, {})", 98 - x); }
    // A pixel whose centre lies exactly on the edge is left out (half-open, as the fill's own rule).
    for x in 0..100 { assert_eq!(at(&hard, x, 99 - x), 0); }
}

#[test]
fn a_thin_sliver_and_a_contour_off_the_canvas_cover_by_area() {
    // A 0.25-px-wide sliver down column 10 covers a quarter of each pixel: 64. A square from -30 to
    // 5 covers columns 0..5 fully: the part off the canvas folds in as nothing.
    let s = Selection::new(vec![rectangle(rect(10.5, 2.0, 0.25, 6.0)), rectangle(rect(-30.0, 3.0, 35.0, 2.0))], true, 0.0);
    let c = coverage(&s, 20, 10);
    assert_eq!(at(&c, 10, 4), 64);
    assert_eq!((at(&c, 0, 3), at(&c, 4, 4), at(&c, 5, 4)), (255, 255, 0));
}

#[test]
fn antialiased_winding_keeps_fractional_long_spans_holes_and_clipped_edges() {
    // Independent rectangle areas across wide gaps, with reversed holes and edge
    // contributions sharing columns. Consecutive rows also check that the previous
    // row's accumulation cannot leak into a shorter or empty row.
    let boxes = [
        (rect(1024.25, 0.25, 6144.5, 2.5), 1.0),
        (rect(1024.5, 0.75, 3072.0, 1.5), -1.0),
        (rect(-12.0, 0.0, 13.25, 1.5), 1.0),
        (rect(8191.75, 1.0, 12.0, 1.5), 1.0),
    ];
    let contours: Vec<Contour> = boxes.iter().map(|&(b, sign)| {
        let mut c = rectangle(b);
        if sign < 0.0 { c.reverse(); }
        c
    }).collect();
    let filled = rasterize(&contours, 0.0, 0.0, 8192, 4, true);
    for y in 0..4 { for x in 0..8192 {
        let area: f64 = boxes.iter().map(|&(b, sign)| {
            let dx = ((x as f64 + 1.0).min(b.max_x()) - (x as f64).max(b.x)).max(0.0);
            let dy = ((y as f64 + 1.0).min(b.max_y()) - (y as f64).max(b.y)).max(0.0);
            sign * dx * dy
        }).sum();
        let want = (area.abs().min(1.0) * 255.0).round() as u8;
        assert_eq!(at(&filled, x, y), want, "({x}, {y}): rectangle area {area}");
    }}
}

#[test]
fn edges_slanting_across_the_canvas_sides_cover_by_area() {
    // A trapezium whose left side x = 2y - 20.5 crosses x = 0 at y = 10.25 and whose right side
    // x = 50.5 - 2y crosses x = 30 there too, both in the middle of row 10, on 30 x 16: each pixel is
    // the area between the two lines inside it, integrated down the pixel (4000 midpoint rows, far
    // finer than a level for these piecewise-linear widths). Row 10's edge pieces are cut where they
    // leave the canvas, the part left of it folded in as coverage (`accumulate`'s cuts).
    let s = Selection::new(vec![polygon(&[p(-20.5, 0.0), p(50.5, 0.0), p(18.5, 16.0), p(11.5, 16.0)])], true, 0.0);
    let c = coverage(&s, 30, 16);
    for j in 0..16u32 { for i in 0..30u32 {
        let area: f64 = (0..4000).map(|k| {
            let y = j as f64 + (k as f64 + 0.5) / 4000.0;
            ((i as f64 + 1.0).min(50.5 - 2.0 * y) - (i as f64).max(2.0 * y - 20.5)).clamp(0.0, 1.0)
        }).sum::<f64>() / 4000.0;
        let want = area * 255.0;
        assert!((at(&c, i, j) as f64 - want).abs() <= 1.0, "({i}, {j}): {} vs {want:.2}", at(&c, i, j));
    }}
}

/// The fraction of pixel (x, y) inside the ellipse `CGPath.addEllipse` builds in `box`, from a
/// 16 x 16 grid of samples against the four Beziers' implicit ellipse: the true ellipse, whose
/// distance from the Beziers is 0.027% of a half-axis.
fn true_ellipse_coverage(b: Rect, x: u32, y: u32) -> f64 {
    let (cx, cy, rx, ry) = (b.x + b.width / 2.0, b.y + b.height / 2.0, b.width / 2.0, b.height / 2.0);
    let mut inside = 0;
    for j in 0..16 { for i in 0..16 {
        let (sx, sy) = (x as f64 + (i as f64 + 0.5) / 16.0, y as f64 + (j as f64 + 0.5) / 16.0);
        if ((sx - cx) / rx).powi(2) + ((sy - cy) / ry).powi(2) <= 1.0 { inside += 1; }
    }}
    inside as f64 / 256.0
}

#[test]
fn an_ellipse_fills_its_box_as_an_oval_with_soft_edges() {
    // SelectionTests.marqueeEllipseSelectsAnOvalInItsBox: the box (10, 20)-(70, 60).
    let b = rect(10.0, 20.0, 60.0, 40.0);
    let s = Selection::new(vec![ellipse(b)], true, 0.0);
    let bounds = s.bounds().unwrap();
    assert!((bounds.x - 10.0).abs() < 0.5 && (bounds.max_x() - 70.0).abs() < 0.5 && (bounds.y - 20.0).abs() < 0.5 && (bounds.max_y() - 60.0).abs() < 0.5);
    let c = coverage(&s, 80, 80);
    assert_eq!(at(&c, 40, 40), 255, "the middle");
    assert_eq!(at(&c, 11, 21), 0, "the box's corner lies outside the oval");
    // Every pixel against the true ellipse's area coverage. Measured on p4a-scratch (2026-09-27):
    // worst 4 levels: the Beziers' own distance from the ellipse, the flattening (CURVE_TOLERANCE)
    // and the 1/256 sampling grid together. Unflattened chords of 45 degrees measure far more.
    let mut worst = 0u8;
    for y in 0..80 { for x in 0..80 {
        let want = (true_ellipse_coverage(b, x, y) * 255.0).round() as i64;
        worst = worst.max((at(&c, x, y) as i64 - want).unsigned_abs() as u8);
    }}
    assert!(worst <= 4, "worst {worst}");
}

#[test]
fn union_intersection_and_difference_keep_whole_pixel_edges_exact() {
    let a = vec![rectangle(rect(10.0, 10.0, 40.0, 40.0))];
    let b = vec![rectangle(rect(30.0, 20.0, 40.0, 50.0))];
    let union = Selection::new(combine(&a, &b, Boolean::Union), true, 0.0);
    assert_eq!(union.bounds(), Some(rect(10.0, 10.0, 60.0, 60.0)));
    let meet = Selection::new(combine(&a, &b, Boolean::Intersection), true, 0.0);
    assert_eq!(meet.bounds(), Some(rect(30.0, 20.0, 20.0, 30.0)));
    let cut = Selection::new(combine(&a, &b, Boolean::Difference), true, 0.0);
    let c = coverage(&cut, 80, 80);
    assert_eq!((at(&c, 15, 15), at(&c, 35, 35), at(&c, 35, 15), at(&c, 60, 60)), (255, 0, 255, 0));
    assert_eq!(c.bytes().iter().filter(|&&v| v == 255).count(), 40 * 40 - 20 * 30);
    // Subtracting everything leaves an outline with no area: an explicit empty selection.
    let gone = Selection::new(combine(&a, &[rectangle(rect(0.0, 0.0, 80.0, 80.0))], Boolean::Difference), true, 0.0);
    assert!(gone.is_empty() && gone.bounds().is_none());
}

#[test]
fn a_band_round_joined_about_the_outline_grows_and_shrinks_it() {
    // SelectionTests.expandAndContractGrowAndShrinkTheOutline, on the outline: the square
    // (40,40)-(60,60), grown by 5, then that shrunk by 8.
    let square = vec![rectangle(rect(40.0, 40.0, 20.0, 20.0))];
    let grown = combine(&square, &band(&square, 5.0).unwrap(), Boolean::Union);
    let g = Selection::new(grown.clone(), true, 0.0).bounds().unwrap();
    assert!((g.x - 35.0).abs() < 0.01 && (g.width - 30.0).abs() < 0.01, "{g:?}");
    let c = coverage(&Selection::new(grown.clone(), true, 0.0), 100, 100);
    assert_eq!((at(&c, 37, 50), at(&c, 33, 50)), (255, 0));
    // The joins are round: pixel (35, 35) is the square's grown corner, and its nearest point,
    // (36, 36), lies 5.66 px from the corner (40, 40), outside the 5-px arc. A mitred join fills it.
    assert_eq!(at(&c, 35, 35), 0);
    let shrunk = combine(&grown, &band(&grown, 8.0).unwrap(), Boolean::Difference);
    let s = Selection::new(shrunk, true, 0.0).bounds().unwrap();
    assert!((s.x - 43.0).abs() < 0.01 && (s.width - 14.0).abs() < 0.01, "{s:?}");
}

/// The box over columns `from..to` and every row of a `width` x 20 canvas, feathered by `feather`,
/// against the formula on rows 0, 10 and 19, within 1 level: every row is the same, so each is the
/// fill's row blurred by a Gaussian of sigma feather / 2 (radius ceil(3 sigma)) over the region --
/// the box's bounds grown by ceil(2 x feather) and a pixel, cut to the canvas -- with the region's
/// edge pixels repeated beyond it: at column x, 255 x (the kernel's weight on the columns, clamped
/// into the region, that lie in the box) / (all of it); 0 outside the region. Returns the coverage.
fn assert_feather_profile(feather: f64, width: u32, from: u32, to: u32) -> GrayRaster {
    let s = Selection::new(vec![rectangle(rect(from as f64, 0.0, (to - from) as f64, 20.0))], true, feather);
    let c = coverage(&s, width, 20);
    let sigma = feather / 2.0;
    let radius = (3.0 * sigma).ceil() as i64;
    let grow = (2.0 * feather).ceil();
    let (left, right) = ((from as f64 - grow - 1.0).floor().max(0.0) as i64, (to as f64 + grow + 1.0).ceil().min(width as f64) as i64);
    let w = |j: i64| (-((j * j) as f64) / (2.0 * sigma * sigma)).exp();
    let total: f64 = (-radius..=radius).map(w).sum();
    for x in 0..width as i64 {
        let want = if x < left || x >= right { 0.0 } else {
            255.0 * (-radius..=radius).filter(|j| (from as i64..to as i64).contains(&(x + j).clamp(left, right - 1))).map(w).sum::<f64>() / total
        };
        for y in [0, 10, 19] {
            assert!((at(&c, x as u32, y) as f64 - want).abs() <= 1.0, "feather {feather}, ({x}, {y}): {} vs {want:.2}", at(&c, x as u32, y));
        }
    }
    c
}

#[test]
fn a_feather_blurs_the_coverage_by_half_its_amount() {
    // SelectionFeatherTests.featherSoftensTheSelectionAndWhatItClips: the rectangle (20, 0)-(40, 20)
    // on 60 x 20, feather 6.
    let c = assert_feather_profile(6.0, 60, 20, 40);
    let fading = (0..60).filter(|x| { let v = at(&c, *x, 10); v > 8 && v < 247 }).count();
    assert!(fading >= 4, "the Mac's own check: {fading} fading columns");
    // Larger feathers (final review F1), each box reaching the canvas's right edge, where the
    // region's edge is repeated: 255 held to the edge, not the half a transparent beyond would give.
    // The canvas is 20 rows high, so every column is repeated past its top and bottom as well.
    for feather in [6.0f64, 20.0, 63.0] {
        let grow = (2.0 * feather).ceil() as u32;
        let width = 4 * grow + 40;
        let c = assert_feather_profile(feather, width, width / 2, width);
        assert_eq!(at(&c, width - 1, 10), 255, "feather {feather}: the edge repeated");
    }
}

/// A fixed-seed generator for the sweep below (Knuth's MMIX LCG, top bits).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
    fn below(&mut self, n: u64) -> u64 { self.next() % n }
}

/// The feather's formula on `c`: the sampled Gaussian of `sigma` out to ceil(3 sigma), normalized,
/// along rows and then columns, the raster's edge pixels repeated, in f64, rounded once.
fn formula_blur(c: &GrayRaster, sigma: f64) -> Vec<u8> {
    let (w, h) = (c.width as i64, c.height as i64);
    let radius = (3.0 * sigma).ceil() as i64;
    let kernel: Vec<f64> = (-radius..=radius).map(|j| (-((j * j) as f64) / (2.0 * sigma * sigma)).exp()).collect();
    let total: f64 = kernel.iter().sum();
    let mut rows = vec![0f64; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        rows[(y * w + x) as usize] = kernel.iter().enumerate().map(|(k, weight)| c.bytes()[(y * w + (x + k as i64 - radius).clamp(0, w - 1)) as usize] as f64 * weight).sum::<f64>() / total;
    }}
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let v = kernel.iter().enumerate().map(|(k, weight)| rows[((y + k as i64 - radius).clamp(0, h - 1) * w + x) as usize] * weight).sum::<f64>() / total;
        out[(y * w + x) as usize] = v.round().clamp(0.0, 255.0) as u8;
    }}
    out
}

#[test]
fn the_feather_blur_is_the_formula_within_a_level_on_a_randomized_sweep() {
    // LL-071: an approximation with an error bound gets a randomized sweep over its boundaries. 240
    // cases: sizes 1 to 96 on each axis (one-pixel lines included), sigma from 0.3 to 125 (both sides
    // of DIRECT_SIGMA_LIMIT, and kernels far wider than the raster), content of hard boxes, single
    // dots, soft values and noise, every pixel against the formula computed here.
    let mut rng = Lcg(0x5e1ec7);
    let sigmas = [0.3, 0.75, 1.0, 1.5, DIRECT_SIGMA_LIMIT - 0.01, DIRECT_SIGMA_LIMIT, 2.5, 3.0, 4.5, 7.0, 10.0, 16.0, 31.5, 60.0, 125.0];
    let mut worst = (0i32, String::new());
    for case in 0..240 {
        let (w, h) = (1 + rng.below(96) as u32, 1 + rng.below(96) as u32);
        let sigma = if case < sigmas.len() * 8 { sigmas[case % sigmas.len()] } else { 0.3 + rng.below(1250) as f64 / 10.0 };
        let base = [0u8, 255][rng.below(2) as usize];
        let mut data = vec![base; (w * h) as usize];
        match rng.below(4) {
            0 => for _ in 0..1 + rng.below(4) {
                let (x0, y0) = (rng.below(w as u64) as u32, rng.below(h as u64) as u32);
                let (x1, y1) = ((x0 + 1 + rng.below(w as u64) as u32).min(w), (y0 + 1 + rng.below(h as u64) as u32).min(h));
                let v = [0u8, 255, rng.below(256) as u8][rng.below(3) as usize];
                for y in y0..y1 { for x in x0..x1 { data[(y * w + x) as usize] = v; } }
            },
            1 => { let (x, y) = (rng.below(w as u64) as u32, rng.below(h as u64) as u32); data[(y * w + x) as usize] = 255 - base; }
            2 => for (i, v) in data.iter_mut().enumerate() { *v = ((i as u32 % w) * 255 / w.max(1)) as u8; },
            _ => for v in data.iter_mut() { *v = rng.below(256) as u8; },
        }
        let c = GrayRaster::from_bytes(w, h, data);
        let fast = feather_blur(&c, sigma);
        let want = formula_blur(&c, sigma);
        for (i, (&a, &b)) in fast.bytes().iter().zip(&want).enumerate() {
            let d = (a as i32 - b as i32).abs();
            if d > worst.0 { worst = (d, format!("case {case}: {w} x {h}, sigma {sigma}, pixel ({}, {}): {a} vs {b}", i as u32 % w, i as u32 / w)); }
        }
    }
    println!("feather sweep: worst {} ({})", worst.0, worst.1);
    assert!(worst.0 <= 1, "worst {}: {}", worst.0, worst.1);
}

#[test]
fn the_clip_covers_the_coverage_bounds_and_a_pixel_cut_to_the_canvas() {
    // `clip(canvas:)`: a 10 x 6 box at (-3, 20) with feather 2 reaches ceil(4) = 4 px further, and a
    // pixel more; the canvas cuts it at x = 0.
    let s = Selection::new(vec![rectangle(rect(-3.0, 20.0, 10.0, 6.0))], true, 2.0);
    let clip = SelectionClip::new(&s, 100, 50);
    let c = clip.coverage.as_ref().unwrap();
    assert_eq!(clip.origin, (0, 15));
    assert_eq!((c.width, c.height), (7 + 4 + 1, 6 + 2 * (4 + 1)));
    // An explicit empty selection clips everything away.
    let empty = SelectionClip::new(&Selection::new(vec![], true, 0.0), 100, 50);
    assert!(empty.coverage.is_none());
    assert!(empty.on_grid(&Affine::IDENTITY, 100, 50).bytes().iter().all(|&v| v == 0));
}

#[test]
fn coverage_on_a_scaled_layer_samples_the_canvas_coverage_at_each_pixel_centre() {
    // SelectionEditTests.clipFollowsScaledLayersAndSoftensEdges: a 50 x 20 layer stretched to
    // 100 x 40 at the origin, under the triangle (0,0), (100,0), (0,40). Layer pixel (i, j)
    // centres on document (2i + 1, 2j + 1).
    let s = Selection::new(vec![polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(0.0, 40.0)])], true, 0.0);
    let layer = LayerTransform::axis_aligned(p(0.0, 0.0), Size { width: 100.0, height: 40.0 });
    let grid = SelectionClip::new(&s, 100, 40).on_grid(&layer.pixel_to_document(50, 20), 50, 20);
    let canvas = coverage(&s, 100, 40);
    assert_eq!(at(&grid, 5, 5), 255, "inside");
    assert_eq!(at(&grid, 45, 17), 0, "outside");
    // Bilinear between the canvas pixels around each centre: centre (2i + 1, 2j + 1) is the shared
    // corner of canvas pixels 2i..2i+1 and 2j..2j+1, so it is their mean.
    for j in 0..20u32 { for i in 0..50u32 {
        let mean = (at(&canvas, 2 * i, 2 * j) as f64 + at(&canvas, 2 * i + 1, 2 * j) as f64 + at(&canvas, 2 * i, 2 * j + 1) as f64 + at(&canvas, 2 * i + 1, 2 * j + 1) as f64) / 4.0;
        assert!((at(&grid, i, j) as f64 - mean).abs() <= 1.0, "({i}, {j}): {} vs {mean}", at(&grid, i, j));
    }}
    assert!(grid.bytes().iter().any(|&v| v > 0 && v < 255), "the diagonal is soft");
}

#[test]
fn coverage_on_a_layer_moved_by_whole_pixels_is_the_canvas_coverage_cropped() {
    // Ruling M13 (fix round 1): comparing the moved layer's grid only against `coverage(&s, 60, 40)`
    // tests `on_grid`'s whole-pixel copy branch against itself (both go through the same fast path,
    // one with a real (15, -4) translation, the other with the identity, so a bug shared by both
    // offsets could still pass). The independent check below never calls `on_grid` at all: it moves
    // the outline by an exact whole `SUBPIXEL` translation (`translated`, so the ellipse's Beziers
    // are not re-flattened -- no new approximation error to explain away) into the layer's own local
    // frame, then calls `rasterize` and the blur `SelectionClip` itself uses directly, over a region
    // far larger than the outline's feathered extent so no boundary clamp in that blur ever reaches
    // the pixels compared below. The layer's origin (x = 15) is chosen so its left edge (document
    // x = 15) already sits deep inside the ellipse's real coverage, not in its near-zero fringe, so a
    // positive-dx crop bug in `on_grid` (for example using `dx.abs()` for the left edge, or adding a
    // stray constant to `dx`) actually changes real, non-zero values instead of two zeros agreeing.
    let s = Selection::new(vec![ellipse(rect(12.0, 7.0, 31.0, 18.0))], true, 1.5);
    let canvas = coverage(&s, 60, 40);
    let layer = LayerTransform::axis_aligned(p(15.0, -4.0), Size { width: 30.0, height: 25.0 });
    let grid = SelectionClip::new(&s, 60, 40).on_grid(&layer.pixel_to_document(30, 25), 30, 25);
    for j in 0..25u32 { for i in 0..30u32 {
        let want = if j >= 4 { at(&canvas, 15 + i, j - 4) } else { 0 };
        assert_eq!(at(&grid, i, j), want, "({i}, {j})");
    }}
    // Independent reference: never touches `on_grid`, `SelectionClip`, or the canvas's own bounds
    // computation -- only `rasterize` and the blur, called directly, over a region ((-20, -20), 80 x
    // 80) chosen generously enough (the outline's own feathered bounds, shifted into this frame, are
    // (-6, 8)-(31, 32) -- comfortably inside with margin far past the blur's kernel radius) that no
    // edge clamp in `feather_blur` reaches pixel (0, 0)..(30, 25) of this region, so those pixels equal
    // exactly what an unclipped rasterisation and blur of the moved outline would give.
    let shifted = s.translated((-15.0 * SUBPIXEL) as i32, (4.0 * SUBPIXEL) as i32);
    let (region_left, region_top, region_w, region_h) = (-20.0, -20.0, 80u32, 80u32);
    let filled = rasterize(&shifted.contours, region_left, region_top, region_w, region_h, shifted.antialiased || shifted.feather > 0.0);
    let reference = if shifted.feather > 0.0 { feather_blur(&filled, shifted.feather / 2.0) } else { filled };
    for j in 0..25u32 { for i in 0..30u32 {
        let (bx, by) = ((i as i64 - region_left as i64) as u32, (j as i64 - region_top as i64) as u32);
        assert_eq!(at(&grid, i, j), at(&reference, bx, by), "independent reference at ({i}, {j})");
    }}
}

#[test]
fn an_outline_knows_what_it_contains_and_moves_by_whole_units() {
    let s = Selection::new(vec![rectangle(rect(10.0, 10.0, 20.0, 20.0))], true, 0.0);
    assert!(s.contains(p(20.0, 20.0)) && !s.contains(p(60.0, 60.0)) && !s.contains(p(9.5, 20.0)));
    let moved = s.translated(40 * SUBPIXEL as i32, 40 * SUBPIXEL as i32);
    assert_eq!(moved.bounds(), Some(rect(50.0, 50.0, 20.0, 20.0)));
    assert!(!s.is_empty());
    assert!(Selection::new(vec![polygon(&[p(3.0, 3.0), p(9.0, 3.0), p(12.0, 3.0)])], true, 0.0).is_empty(), "no area");
}
