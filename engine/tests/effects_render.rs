//! The effects kernel (engine/src/effects/render.rs) against `metal`, a whole-plane, line-by-line
//! transcription of MetalLayerEffects.swift's kernels (:219-392) with no shortcut, and against
//! values computed here from the Mac's formulas.
use compositor_engine::*;

/// MetalLayerEffects.swift, kernel by kernel, over whole f32 planes.
mod metal {
    use compositor_engine::EffectPasses;

    fn alpha(pixels: &[u8], w: usize, h: usize, inset: usize) -> (Vec<f32>, usize, usize) {
        let (pw, ph) = (w + 2 * inset, h + 2 * inset);
        let mut a = vec![0.0f32; pw * ph];
        for y in 0..h { for x in 0..w { a[(y + inset) * pw + x + inset] = pixels[(y * w + x) * 4 + 3] as f32 / 255.0; } }
        (a, pw, ph)
    }
    fn spread(src: &[f32], w: usize, h: usize, reach: usize, smallest: bool) -> Vec<f32> {
        let reach = reach as i64;
        let pass = |src: &[f32], along_rows: bool| {
            let mut out = vec![0.0f32; w * h];
            for y in 0..h { for x in 0..w {
                let mut best = if smallest { 1.0f32 } else { 0.0 };
                for o in -reach..=reach {
                    let (sx, sy) = if along_rows { (x as i64 + o, y as i64) } else { (x as i64, y as i64 + o) };
                    let v = if sx < 0 || sy < 0 || sx >= w as i64 || sy >= h as i64 { 0.0 } else { src[sy as usize * w + sx as usize] };
                    best = if smallest { best.min(v) } else { best.max(v) };
                }
                out[y * w + x] = best;
            }}
            out
        };
        pass(&pass(src, true), false)
    }
    fn shift(src: &[f32], w: usize, h: usize, dx: f32, dy: f32) -> Vec<f32> {
        let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let mut out = vec![0.0f32; w * h];
        for y in 0..h { for x in 0..w {
            let (sx, sy) = (x as f32 - dx, y as f32 - dy);
            if sx >= 0.0 && sy >= 0.0 && sx <= (w - 1) as f32 && sy <= (h - 1) as f32 {
                let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
                let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
                let (fx, fy) = (sx - x0 as f32, sy - y0 as f32);
                let top = mix(src[y0 * w + x0], src[y0 * w + x1], fx);
                let bottom = mix(src[y1 * w + x0], src[y1 * w + x1], fx);
                out[y * w + x] = mix(top, bottom, fy);
            }
        }}
        out
    }
    pub fn blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
        let radius = ((sigma * 3.0).round() as i64).max(1);
        let pass = |src: &[f32], along_rows: bool| {
            let mut out = vec![0.0f32; w * h];
            for y in 0..h { for x in 0..w {
                let (mut total, mut sum) = (0.0f32, 0.0f32);
                for o in -radius..=radius {
                    let weight = (-((o * o) as f32) / (2.0 * sigma * sigma)).exp();
                    let s = if along_rows { y * w + (x as i64 + o).clamp(0, w as i64 - 1) as usize } else { (y as i64 + o).clamp(0, h as i64 - 1) as usize * w + x };
                    total += weight * src[s];
                    sum += weight;
                }
                out[y * w + x] = total / sum;
            }}
            out
        };
        pass(&pass(src, true), false)
    }
    /// `effects_shift` then, above sigma 0.01, the two blur passes (a zero move is the shape itself).
    fn soften(shape: &[f32], w: usize, h: usize, dx: f32, dy: f32, sigma: f32) -> Vec<f32> {
        let moved = if dx == 0.0 && dy == 0.0 { shape.to_vec() } else { shift(shape, w, h, dx, dy) };
        if sigma > 0.01 { blur(&moved, w, h, sigma) } else { moved }
    }
    pub fn render(pixels: &[u8], w: usize, h: usize, inset: usize, p: &EffectPasses) -> Vec<u8> {
        let (shape, pw, ph) = alpha(pixels, w, h, inset);
        let n = pw * ph;
        let inside_of = |m: Vec<f32>| (0..n).map(|i| (shape[i] * (1.0 - m[i])).clamp(0.0, 1.0)).collect::<Vec<f32>>();
        let ring = p.stroke.map(|s| { let m = spread(&shape, pw, ph, s.reach, s.inside);
            (0..n).map(|i| if s.inside { shape[i] - m[i] } else { m[i] - shape[i] }.clamp(0.0, 1.0)).collect::<Vec<f32>>() });
        let shadow = p.shadow.map(|s| soften(&shape, pw, ph, s.dx, s.dy, s.sigma));
        let inner = p.inner_shadow.map(|s| inside_of(soften(&shape, pw, ph, s.dx, s.dy, s.sigma)));
        let glow = p.outer_glow.map(|g| soften(&shape, pw, ph, 0.0, 0.0, g.sigma));
        let inner_glow = p.inner_glow.map(|g| inside_of(soften(&shape, pw, ph, 0.0, 0.0, g.sigma)));
        let over = |c: &mut [f32; 3], a: &mut f32, rgb: [f32; 3], k: f32| { for i in 0..3 { c[i] = rgb[i] * k + c[i] * (1.0 - k); } *a = k + *a * (1.0 - k); };
        let mut out = vec![0u8; n * 4];
        for y in 0..ph { for x in 0..pw {
            let i = y * pw + x;
            let (mut c, mut a) = ([0.0f32; 3], 0.0f32);
            if let (Some(s), Some(v)) = (p.shadow, &shadow) { let k = (v[i] * s.opacity).clamp(0.0, 1.0); c = s.color.map(|v| v * k); a = k; }
            if let (Some(g), Some(v)) = (p.outer_glow, &glow) { over(&mut c, &mut a, g.color, (v[i] * (1.0 - shape[i]) * g.opacity).clamp(0.0, 1.0)); }
            let stroke = match (p.stroke, &ring) { (Some(s), Some(r)) => (r[i] * s.opacity).clamp(0.0, 1.0), _ => 0.0 };
            if let Some(s) = p.stroke { if !s.inside { over(&mut c, &mut a, s.color, stroke); } }
            let src: [f32; 4] = if x >= inset && y >= inset && x < inset + w && y < inset + h {
                let j = ((y - inset) * w + x - inset) * 4; [0, 1, 2, 3].map(|k| pixels[j + k] as f32 / 255.0)
            } else { [0.0; 4] };
            for k in 0..3 { c[k] = src[k] + c[k] * (1.0 - src[3]); }
            a = src[3] + a * (1.0 - src[3]);
            if let Some(o) = p.overlay { over(&mut c, &mut a, o.color, (shape[i] * o.opacity).clamp(0.0, 1.0)); }
            if let (Some(g), Some(v)) = (p.inner_glow, &inner_glow) { over(&mut c, &mut a, g.color, (v[i] * g.opacity).clamp(0.0, 1.0)); }
            if let (Some(s), Some(v)) = (p.inner_shadow, &inner) { over(&mut c, &mut a, s.color, (v[i] * s.opacity).clamp(0.0, 1.0)); }
            if let Some(s) = p.stroke { if s.inside { over(&mut c, &mut a, s.color, stroke); } }
            for k in 0..3 { out[i * 4 + k] = (c[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8; }
            out[i * 4 + 3] = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }}
        out
    }
}

/// Oblong and soft-edged, with a hole off-centre: coverage takes many values, and no axis mirrors
/// another. Premultiplied.
fn blob(w: usize, h: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(w * h * 4);
    for y in 0..h { for x in 0..w {
        let (cx, cy) = (x as f64 - w as f64 * 0.4, y as f64 - h as f64 * 0.55);
        let r = (cx * cx / (w as f64 * 0.3).powi(2) + cy * cy / (h as f64 * 0.35).powi(2)).sqrt();
        let hole = ((x as f64 - w as f64 * 0.6).powi(2) + (y as f64 - h as f64 * 0.4).powi(2)).sqrt() < h as f64 * 0.08;
        let a = if hole { 0 } else { (((1.1 - r) * 4.0).clamp(0.0, 1.0) * 255.0).round() as u32 };
        let (r8, g8, b8) = ((x * 255 / w) as u32, (y * 255 / h) as u32, 120u32);
        v.extend_from_slice(&[(r8 * a / 255) as u8, (g8 * a / 255) as u8, (b8 * a / 255) as u8, a as u8]);
    }}
    v
}
fn rect(w: usize, h: usize, rgba: [u8; 4]) -> Vec<u8> { rgba.repeat(w * h) }

fn render(pixels: &[u8], w: usize, h: usize, inset: usize, passes: &EffectPasses) -> Vec<u8> {
    render_passes(&Padded { pixels, width: w, height: h, inset }, passes)
}
/// One premultiplied pixel of a padded image `pw` wide.
fn at(out: &[u8], pw: usize, x: usize, y: usize) -> [u8; 4] { let i = (y * pw + x) * 4; [out[i], out[i + 1], out[i + 2], out[i + 3]] }
/// The compose kernel's 8-bit output.
fn byte(v: f32) -> u8 { (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 }
fn worst(a: &[u8], b: &[u8]) -> u8 { assert_eq!(a.len(), b.len()); a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap() }

fn stroke(reach: usize, inside: bool) -> StrokePass { StrokePass { reach, inside, color: [0.1, 0.9, 0.3], opacity: 0.8 } }
const OVERLAY: FillPass = FillPass { color: [0.9, 0.2, 0.5], opacity: 0.35 };

#[test]
fn the_stroke_and_the_colour_overlay_equal_the_metal_transcription_to_the_bit() {
    let fixtures = [("blob", blob(53, 37), 53, 37), ("translucent rect", rect(41, 29, [150, 90, 20, 200]), 41, 29), ("tiny", blob(9, 5), 9, 5)];
    for (name, px, w, h) in &fixtures {
        for (reach, inside, overlay, inset) in [(1, false, false, 2), (4, true, true, 40), (12, false, true, 40), (30, true, false, 40), (20, true, true, 2)] {
            let p = EffectPasses { stroke: Some(stroke(reach, inside)), overlay: overlay.then_some(OVERLAY), ..Default::default() };
            let d = worst(&render(px, *w, *h, inset, &p), &metal::render(px, *w, *h, inset, &p));
            assert_eq!(d, 0, "{name}: reach {reach} inside {inside} overlay {overlay} inset {inset}");
        }
    }
}

#[test]
fn an_outside_stroke_of_six_is_green_five_pixels_from_a_red_square_as_the_macs_export_test_expects() {
    // ProjectTests.swift:270-311: a 20 x 20 red square, a 6 px green stroke outside, opacity 1.
    let (w, inset) = (20, 8);
    let p = EffectPasses { stroke: Some(StrokePass { reach: 6, inside: false, color: [0.0, 1.0, 0.0], opacity: 1.0 }), ..Default::default() };
    let out = render(&rect(w, w, [255, 0, 0, 255]), w, w, inset, &p);
    let pw = w + 2 * inset;
    let mid = inset + 10;
    assert_eq!(at(&out, pw, inset - 5, mid), [0, 255, 0, 255], "5 px left of the square");
    assert_eq!(at(&out, pw, inset - 6, inset - 6), [0, 255, 0, 255], "the reach is square, so the corner is covered");
    assert_eq!(at(&out, pw, inset - 7, mid), [0, 0, 0, 0], "7 px out is past the reach");
    assert_eq!(at(&out, pw, mid, mid), [255, 0, 0, 255], "the square itself stays red");
}

#[test]
fn an_inside_stroke_covers_the_band_inside_the_edge_at_its_opacity() {
    let (w, h, inset) = (24, 16, 2);
    let (rgb, opacity) = ([0.0f32, 0.0, 1.0], 0.5f32);
    let p = EffectPasses { stroke: Some(StrokePass { reach: 3, inside: true, color: rgb, opacity }), ..Default::default() };
    let out = render(&rect(w, h, [255, 0, 0, 255]), w, h, inset, &p);
    let pw = w + 2 * inset;
    // Two pixels in from the left edge: the eroded shape is 0 there, so the ring is 1.
    let c = opacity;
    let want = [byte(rgb[0] * c + 1.0 * (1.0 - c)), byte(rgb[1] * c), byte(rgb[2] * c), byte(c + 1.0 * (1.0 - c))];
    assert_eq!(at(&out, pw, inset + 2, inset + 8), want);
    assert_eq!(at(&out, pw, inset + 3, inset + 8), [255, 0, 0, 255], "3 px in: the reach holds only the shape");
    assert_eq!(at(&out, pw, inset - 1, inset + 8), [0, 0, 0, 0], "an inside stroke draws nothing outside");
}

#[test]
fn a_colour_overlay_covers_what_the_layer_shows_at_its_opacity() {
    let (w, h, inset) = (5, 3, 2);
    let px = rect(w, h, [60, 30, 90, 128]);
    let p = EffectPasses { overlay: Some(FillPass { color: [0.0, 1.0, 0.0], opacity: 0.8 }), ..Default::default() };
    let out = render(&px, w, h, inset, &p);
    let src = [60.0f32 / 255.0, 30.0 / 255.0, 90.0 / 255.0, 128.0 / 255.0];
    let c = (src[3] * 0.8f32).clamp(0.0, 1.0);
    let want = [byte(0.0 * c + src[0] * (1.0 - c)), byte(1.0 * c + src[1] * (1.0 - c)), byte(0.0 * c + src[2] * (1.0 - c)), byte(c + src[3] * (1.0 - c))];
    assert_eq!(at(&out, w + 2 * inset, inset + 1, inset + 1), want);
    assert_eq!(at(&out, w + 2 * inset, 0, 0), [0, 0, 0, 0], "nothing off the layer");
}

#[test]
fn the_metal_path_draws_a_stroke_only_at_a_size_and_opacity_above_zero_and_rounds_its_reach() {
    let effects = |size: f64, opacity: f64| -> LayerEffects {
        serde_json::from_value(serde_json::json!({ "stroke": { "blue": 0, "green": 0, "inside": false, "opacity": opacity, "red": 1, "size": size } })).unwrap()
    };
    assert_eq!(EffectPasses::from_effects(&effects(0.0, 1.0)).stroke, None);
    assert_eq!(EffectPasses::from_effects(&effects(4.0, 0.0)).stroke, None);
    assert_eq!(EffectPasses::from_effects(&effects(0.4, 1.0)).stroke.unwrap().reach, 1, "max(1, round(size))");
    assert_eq!(EffectPasses::from_effects(&effects(2.5, 1.0)).stroke.unwrap().reach, 3, "Swift rounds half away from zero");
    let mut hidden = effects(4.0, 1.0);
    hidden.stroke.as_mut().unwrap().enabled = Some(false);
    assert_eq!(EffectPasses::from_effects(&hidden), EffectPasses::default());
}

fn shadow(dx: f32, dy: f32, sigma: f32) -> ShadowPass { ShadowPass { dx, dy, sigma, color: [0.0, 0.1, 0.4], opacity: 0.6 } }
fn glow(sigma: f32) -> GlowPass { GlowPass { sigma, color: [1.0, 0.9, 0.2], opacity: 0.75 } }
fn all_six(sigma: f32, reach: usize, inside: bool) -> EffectPasses {
    EffectPasses { stroke: Some(stroke(reach, inside)), shadow: Some(shadow(-7.3, 11.6, sigma)), overlay: Some(OVERLAY),
        inner_shadow: Some(ShadowPass { dx: 4.25, dy: -3.5, sigma: sigma * 0.6, color: [0.2, 0.0, 0.0], opacity: 0.7 }),
        outer_glow: Some(glow(sigma * 0.8)), inner_glow: Some(GlowPass { sigma: sigma * 0.5, color: [1.0, 1.0, 1.0], opacity: 0.6 }) }
}

#[test]
fn shadows_and_glows_equal_the_metal_transcription_to_the_bit_up_to_the_reach_limit() {
    // Sigma 16 is the last exact one: 3 x 16 = 48 = EFFECTS_REACH_LIMIT.
    let fixtures = [("blob", blob(53, 37), 53, 37), ("translucent rect", rect(41, 29, [150, 90, 20, 200]), 41, 29)];
    for (name, px, w, h) in &fixtures {
        for (sigma, reach, inside) in [(0.005f32, 1usize, false), (3.0, 4, false), (5.5, 7, true), (10.0, 30, true), (16.0, 12, false)] {
            let p = all_six(sigma, reach, inside);
            let d = worst(&render(px, *w, *h, 60, &p), &metal::render(px, *w, *h, 60, &p));
            assert_eq!(d, 0, "{name}: sigma {sigma}");
        }
    }
    // A ring deeper than the image is tall.
    let p = all_six(8.0, 20, true);
    assert_eq!(worst(&render(&blob(9, 5), 9, 5, 2, &p), &metal::render(&blob(9, 5), 9, 5, 2, &p)), 0, "tiny");
}

#[test]
fn a_blur_past_the_reach_limit_stays_within_one_level_of_the_exact_kernel() {
    // Measured 2026-09-25 (scratch p35c-scratch, sigma 16.5 to 125, both fixtures): 1 at most.
    // Small layers keep the whole-plane reference quick (the insets below, not the layer, dominate
    // its cost) but do NOT catch a sampling-phase error (`fy = y / f` or `fx = x / f`): only the
    // 160 x 100 case below is wide enough for that, so it alone carries that bite (re-review N1).
    for (name, px, w, h) in [("rect", rect(18, 12, [200, 60, 30, 255]), 18, 12), ("blob", blob(18, 12), 18, 12)] {
        for sigma in [16.5f32, 25.0, 40.0] {
            let inset = (sigma * 3.0).ceil() as usize + 20;
            let black = [0.0f32; 3];
            let cases = [
                ("drop shadow", EffectPasses { shadow: Some(ShadowPass { dx: -7.3, dy: 11.6, sigma, color: black, opacity: 1.0 }), ..Default::default() }),
                ("inner shadow", EffectPasses { inner_shadow: Some(ShadowPass { dx: -7.3, dy: 11.6, sigma, color: black, opacity: 1.0 }), ..Default::default() }),
                ("outer glow", EffectPasses { outer_glow: Some(GlowPass { sigma, color: [1.0; 3], opacity: 1.0 }), ..Default::default() }),
                ("inner glow", EffectPasses { inner_glow: Some(GlowPass { sigma, color: [1.0; 3], opacity: 1.0 }), ..Default::default() }),
            ];
            for (label, p) in cases {
                let d = worst(&render(&px, w, h, inset, &p), &metal::render(&px, w, h, inset, &p));
                assert!(d <= 1, "{name} {label} sigma {sigma} (level {}): {d}", effects_level(sigma));
            }
        }
    }
    // A layer several kernels wide at the first halved sigma, so the blurred edge reaches its full
    // slope, where a halved blur's error is largest (pre-flight D-M2; 1 at most, measured on the
    // p35c-planfix copy).
    let (px, sigma) = (rect(160, 100, [200, 60, 30, 255]), 16.5f32);
    for p in [EffectPasses { shadow: Some(ShadowPass { dx: -7.3, dy: 11.6, sigma, color: [0.0; 3], opacity: 1.0 }), ..Default::default() },
              EffectPasses { inner_glow: Some(GlowPass { sigma, color: [1.0; 3], opacity: 1.0 }), ..Default::default() }] {
        let d = worst(&render(&px, 160, 100, 70, &p), &metal::render(&px, 160, 100, 70, &p));
        assert!(d <= 1, "160 x 100, sigma {sigma}: {d}");
    }
    // Three halvings.
    let p = EffectPasses { shadow: Some(ShadowPass { dx: 5.0, dy: 9.5, sigma: 65.0, color: [0.0; 3], opacity: 1.0 }), ..Default::default() };
    let px = rect(24, 16, [30, 200, 90, 255]);
    assert_eq!(effects_level(65.0), 3);
    assert!(worst(&render(&px, 24, 16, 225, &p), &metal::render(&px, 24, 16, 225, &p)) <= 1);
}

#[test]
fn a_halved_blur_repeats_the_padded_edge_pixel_not_the_reduced_edge_cell() {
    // C1: the halved blur must clamp to the padded image's own edge PIXEL, as Metal's
    // `effects_blur_rows` and `_columns` do, not to the mean of the reduced grid's edge CELL.
    // Inner glow and inner shadow get no margin of their own (`LayerEffects::margin`), so an
    // ordinary layer that only sets one of them can sit at inset 2. Expected values come from
    // `metal::render`, the whole-plane transcription with no shortcut.
    let opaque = rect(60, 40, [200, 60, 30, 255]);

    // Inner glow at its real margin (inset 2), at two halving levels.
    for sigma in [40.0f32, 70.0] {
        let p = EffectPasses { inner_glow: Some(GlowPass { sigma, color: [1.0; 3], opacity: 1.0 }), ..Default::default() };
        let d = worst(&render(&opaque, 60, 40, 2, &p), &metal::render(&opaque, 60, 40, 2, &p));
        assert!(d <= 1, "inner glow sigma {sigma} at inset 2 (level {}): {d}", effects_level(sigma));
    }

    // Inner shadow at its real margin (inset 8), off-axis fractional offset.
    let inner_shadow = |sigma: f32| EffectPasses {
        inner_shadow: Some(ShadowPass { dx: -7.3, dy: 11.6, sigma, color: [0.0; 3], opacity: 1.0 }), ..Default::default() };
    let d = worst(&render(&opaque, 60, 40, 8, &inner_shadow(20.0)), &metal::render(&opaque, 60, 40, 8, &inner_shadow(20.0)));
    assert!(d <= 1, "inner shadow sigma 20 at inset 8 (level {}): {d}", effects_level(20.0));

    // The same offset, one level deeper: at inset 8 the shifted edge sits at x = 0.7, inside the
    // reduced grid's very first cell whatever the cell size, so this stresses a wider halo too.
    let d = worst(&render(&opaque, 60, 40, 8, &inner_shadow(45.0)), &metal::render(&opaque, 60, 40, 8, &inner_shadow(45.0)));
    assert!(d <= 1, "inner shadow sigma 45 at inset 8 (level {}): {d}", effects_level(45.0));

    // A soft, off-centre fixture: the cells near the edge are not simply 0 or 1 to start with.
    let blob60 = blob(60, 40);
    let d = worst(&render(&blob60, 60, 40, 2, &inner_shadow(70.0)), &metal::render(&blob60, 60, 40, 2, &inner_shadow(70.0)));
    assert!(d <= 1, "blob inner shadow sigma 70 at inset 2 (level {}): {d}", effects_level(70.0));
}

#[test]
fn an_inner_glow_on_a_wide_opaque_layer_stays_within_one_level_at_its_real_margin() {
    // Re-review of 86ef16b..712a976 (C1 not yet addressed): the fix above still left the halved
    // blur up to 3 levels off `metal::render` right where an inner glow or inner shadow really
    // lands -- inset 2, its real margin -- once the layer is wide enough for the blurred edge to
    // reach its full slope. The committed 60x40 fixture above is too narrow for that; this one
    // (and the padded height 204 = 25 x 8 + 4 it produces at level 3, not a multiple of 2^level) is
    // exactly the re-review's counterexample.
    let opaque = rect(300, 200, [180, 90, 40, 255]);
    for sigma in [64.5f32, 70.0] {
        let p = EffectPasses { inner_glow: Some(GlowPass { sigma, color: [1.0; 3], opacity: 1.0 }), ..Default::default() };
        let d = worst(&render(&opaque, 300, 200, 2, &p), &metal::render(&opaque, 300, 200, 2, &p));
        assert!(d <= 1, "inner glow sigma {sigma} on 300x200 at inset 2 (level {}): {d}", effects_level(sigma));
    }
}

/// A tiny, fixed-seed PRNG (SplitMix64) so the sweep below is exactly reproducible: same cases,
/// same order, on every run and every machine.
struct Lcg(u64);
impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn f32(&mut self) -> f32 { (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32 }
    fn range(&mut self, lo: f32, hi: f32) -> f32 { lo + self.f32() * (hi - lo) }
    fn usize_range(&mut self, lo: usize, hi: usize) -> usize { lo + (self.f32() * (hi - lo) as f32) as usize }
    fn bool(&mut self) -> bool { self.f32() < 0.5 }
}

#[test]
fn a_randomized_sweep_of_the_four_gaussian_effects_stays_within_one_level() {
    // Re-review: fixing only the partial-last-cell average (`HalvedBlur::new`) or only the
    // enlargement's halo ring (`HalvedBlur::row`) alone still left 3 levels of error; the reviewer
    // measured <= 1 over 5164 randomized cases only with both fixes. This is a much smaller, fully
    // deterministic version of that sweep: a fixed seed, ~180 cases, over all four Gaussian effects,
    // levels 0-3, insets from 0 (inner effects) to the Mac's own margin (drop shadow, outer glow),
    // fractional and negative offsets, rect/blob/translucent shapes, and sizes not a multiple of
    // 2^level. Small layers and modest sigmas within each level keep this well under 30 s in debug:
    // the exact `metal::render` reference is O(padded area x radius) with no shortcut, and a real
    // margin at level 3 (sigma > 64, so radius > 192) makes the reference itself the expensive part,
    // not the fix being tested -- the drop-shadow/outer-glow loop below only reaches level 1 for
    // that reason (see the fix round 2 report for the cost measurement and why that is still enough
    // coverage: the reviewer's own larger sweep already found both effects safe at every level and
    // every margin, this round's regression is entirely in the two inner effects).
    let mut rng = Lcg(0xC0FFEE_5EED_u64);
    let sigma_range = |level: u32| match level { 0 => (0.5f32, 16.0), 1 => (16.5, 20.0), 2 => (33.0, 40.0), 3 => (65.0, 72.0), _ => unreachable!() };
    let mut worst_overall = 0u8;

    // The bulk: inner glow and inner shadow, which get no margin of their own (`LayerEffects::margin`)
    // and so are where the Mac's own layers actually reach the halved edge (re-review C1).
    for i in 0..176 {
        let level = (i % 4) as u32;
        let (lo, hi) = sigma_range(level);
        let sigma = rng.range(lo, hi);
        let f = 1usize << level;
        let mut w = rng.usize_range(18, 50);
        let mut h = rng.usize_range(14, 40);
        if w % f == 0 { w += 1; }
        if h % f == 0 { h += 1; }
        let inset = rng.usize_range(0, 21);
        let (dx, dy) = (rng.range(-15.0, 15.0), rng.range(-15.0, 15.0));
        let opacity = rng.range(0.3, 1.0);
        let color = [rng.f32(), rng.f32(), rng.f32()];
        let px = match rng.usize_range(0, 3) {
            0 => rect(w, h, [200, 60, 30, 255]),
            1 => rect(w, h, [200, 60, 30, rng.usize_range(1, 255) as u8]),
            _ => blob(w, h),
        };
        let inner_glow = i % 2 == 0;
        let p = if inner_glow {
            EffectPasses { inner_glow: Some(GlowPass { sigma, color, opacity }), ..Default::default() }
        } else {
            EffectPasses { inner_shadow: Some(ShadowPass { dx, dy, sigma, color, opacity }), ..Default::default() }
        };
        let d = worst(&render(&px, w, h, inset, &p), &metal::render(&px, w, h, inset, &p));
        assert!(d <= 1, "case {i}: {} sigma {sigma} level {level} {w}x{h} inset {inset} offset ({dx}, {dy}): {d}",
            if inner_glow { "inner glow" } else { "inner shadow" });
        worst_overall = worst_overall.max(d);
    }

    // Drop shadow and outer glow at the Mac's own margin (`LayerEffects::margin`, settings.rs:177-184):
    // distance + 3 x blur (= 6 x sigma), or 3 x the glow's size (= 6 x sigma) alone, rounded up, plus
    // 2. Both stayed within 1 level even before this round's fix (the plane is flat 0 near the edge
    // at their real margin), so a handful is enough to keep them covered; levels 2 and 3 are left to
    // the reviewer's own larger, one-off sweep (see the comment above `worst_overall`).
    for level in 0..2u32 {
        let (lo, hi) = sigma_range(level);
        for outer_glow in [false, true] {
            let sigma = rng.range(lo, hi);
            let f = 1usize << level;
            let (mut w, mut h) = (rng.usize_range(16, 30), rng.usize_range(12, 24));
            if w % f == 0 { w += 1; }
            if h % f == 0 { h += 1; }
            let (dx, dy) = if outer_glow { (0.0, 0.0) } else { (rng.range(-9.0, 9.0), rng.range(-9.0, 9.0)) };
            let distance = (dx * dx + dy * dy).sqrt();
            let opacity = rng.range(0.3, 1.0);
            let color = [rng.f32(), rng.f32(), rng.f32()];
            let margin = if outer_glow { (sigma * 6.0).ceil() as usize + 2 } else { (distance + sigma * 6.0).ceil() as usize + 2 };
            let px = if rng.bool() { rect(w, h, [200, 60, 30, 255]) } else { blob(w, h) };
            let p = if outer_glow {
                EffectPasses { outer_glow: Some(GlowPass { sigma, color, opacity }), ..Default::default() }
            } else {
                EffectPasses { shadow: Some(ShadowPass { dx, dy, sigma, color, opacity }), ..Default::default() }
            };
            let d = worst(&render(&px, w, h, margin, &p), &metal::render(&px, w, h, margin, &p));
            assert!(d <= 1, "{} sigma {sigma} level {level} margin {margin}: {d}", if outer_glow { "outer glow" } else { "drop shadow" });
            worst_overall = worst_overall.max(d);
        }
    }
    println!("sweep worst level: {worst_overall}");
}

#[test]
fn from_effects_scales_blur_and_size_to_sigma_by_half() {
    // I1: nothing else pins `sigma = blur / 2` for the shadows or `sigma = size / 2` for the outer
    // glow through `from_effects` (every other test there uses blur 0, size 0 or opacity 0).
    // Non-zero, non-round values so `s.blur as f32` or `s.blur / 3.0` would not pass by accident.
    let shadow: LayerEffects = serde_json::from_value(serde_json::json!({ "shadow": {
        "angle": 45, "blue": 0.2, "blur": 15.0, "distance": 5.0, "green": 0.4, "opacity": 0.9, "red": 0.1 } })).unwrap();
    assert_eq!(EffectPasses::from_effects(&shadow).shadow.unwrap().sigma, 7.5, "sigma = blur / 2");

    let inner: LayerEffects = serde_json::from_value(serde_json::json!({ "innerShadow": {
        "angle": 45, "blue": 0.2, "blur": 23.0, "distance": 5.0, "green": 0.4, "opacity": 0.9, "red": 0.1 } })).unwrap();
    assert_eq!(EffectPasses::from_effects(&inner).inner_shadow.unwrap().sigma, 11.5, "sigma = blur / 2");

    let glow: LayerEffects = serde_json::from_value(serde_json::json!({ "outerGlow": {
        "blue": 1, "green": 0.5, "opacity": 0.6, "red": 0.2, "size": 33.0 } })).unwrap();
    assert_eq!(EffectPasses::from_effects(&glow).outer_glow.unwrap().sigma, 16.5, "sigma = size / 2");
}

#[test]
fn the_reach_limit_halves_until_three_sigma_fits_it() {
    // The smallest k with 3 sigma / 2^k <= 48.
    let want = |sigma: f32| ((3.0 * sigma / EFFECTS_REACH_LIMIT).log2().ceil().max(0.0)) as u32;
    for sigma in [1.0f32, 16.0, 16.01, 32.0, 32.1, 100.0, 250.0] { assert_eq!(effects_level(sigma), want(sigma), "sigma {sigma}"); }
    assert_eq!(effects_level(250.0), 4, "a 500 px blur, the Mac's largest, halves four times");
}

/// The drop shadow as the Mac's settings give it (`from_effects`): angle, distance, blur 0.
fn drop_shadow(angle: f64, distance: f64) -> EffectPasses {
    let e: LayerEffects = serde_json::from_value(serde_json::json!({ "shadow": {
        "angle": angle, "blue": 0.5, "blur": 0, "distance": distance, "green": 0.25, "opacity": 0.8, "red": 0 } })).unwrap();
    EffectPasses::from_effects(&e)
}

#[test]
fn the_drop_shadow_falls_away_from_the_light() {
    // An oblong opaque layer; angle 90 is light from above (shadow below), angle 0 light from the
    // right (shadow to the left): LayerEffects.swift:28-43.
    let (w, h, inset) = (30, 10, 20);
    let px = rect(w, h, [255, 255, 255, 255]);
    let pw = w + 2 * inset;
    let shadowed = [byte(0.0 * 0.8), byte(0.25 * 0.8), byte(0.5 * 0.8), byte(0.8)];
    let below = render(&px, w, h, inset, &drop_shadow(90.0, 12.0));
    assert_eq!(at(&below, pw, inset + 15, inset + h + 5), shadowed, "5 px below the bottom edge");
    assert_eq!(at(&below, pw, inset + 15, inset - 5), [0, 0, 0, 0], "nothing above the top edge");
    let left = render(&px, w, h, inset, &drop_shadow(0.0, 12.0));
    assert_eq!(at(&left, pw, inset - 5, inset + 5), shadowed, "5 px left of the left edge");
    assert_eq!(at(&left, pw, inset + w + 5, inset + 5), [0, 0, 0, 0], "nothing right of the right edge");
    assert_eq!(at(&left, pw, inset + 15, inset + 5), [255, 255, 255, 255], "the opaque layer covers its own shadow");
}

#[test]
fn an_inner_shadow_darkens_the_edge_the_light_comes_from() {
    let (w, h, inset) = (30, 20, 2);
    let e: LayerEffects = serde_json::from_value(serde_json::json!({ "innerShadow": {
        "angle": 90, "blue": 0, "blur": 0, "distance": 6, "green": 0, "opacity": 1, "red": 0 } })).unwrap();
    let out = render(&rect(w, h, [255, 255, 255, 255]), w, h, inset, &EffectPasses::from_effects(&e));
    let pw = w + 2 * inset;
    assert_eq!(at(&out, pw, inset + 15, inset + 2), [0, 0, 0, 255], "the top rows, which the moved shape leaves");
    assert_eq!(at(&out, pw, inset + 15, inset + 12), [255, 255, 255, 255], "lower down the moved shape still covers");
}

#[test]
fn an_outer_glow_reaches_every_side_alike() {
    // Square on purpose: the four sides must match (OuterGlowTests.swift, omnidirectional).
    let (w, inset) = (20, 32);
    let p = EffectPasses { outer_glow: Some(GlowPass { sigma: 5.0, color: [0.0, 1.0, 0.0], opacity: 1.0 }), ..Default::default() };
    let out = render(&rect(w, w, [255, 255, 255, 255]), w, w, inset, &p);
    let pw = w + 2 * inset;
    let (c, near) = (inset + 10, inset - 5);
    let far = inset + w + 4;
    let sides = [at(&out, pw, near, c), at(&out, pw, far, c), at(&out, pw, c, near), at(&out, pw, c, far)];
    assert!(sides[0][3] > 25 && sides[0][1] == sides[0][3], "a green glow 5 px out: {:?}", sides[0]);
    for s in &sides[1..] { assert!(s[3].abs_diff(sides[0][3]) <= 1, "{sides:?}"); }
    assert_eq!(at(&out, pw, c, c), [255, 255, 255, 255], "the glow stays outside");
}

#[test]
fn an_inner_glow_lights_the_edge_and_leaves_the_middle() {
    let (w, inset) = (40, 2);
    let e: LayerEffects = serde_json::from_value(serde_json::json!({ "innerGlow": {
        "blue": 1, "green": 1, "opacity": 1, "red": 1, "size": 8 } })).unwrap();
    let out = render(&rect(w, w, [0, 0, 255, 255]), w, w, inset, &EffectPasses::from_effects(&e));
    let pw = w + 2 * inset;
    let edge = at(&out, pw, inset, inset + 20);
    assert!(edge[0] > 100 && edge[2] == 255, "white glow at the edge: {edge:?}");
    assert_eq!(at(&out, pw, inset + 20, inset + 20), [0, 0, 255, 255], "the blurred shape is 1 in the middle, so no glow");
}

#[test]
fn the_metal_path_skips_effects_at_opacity_zero_and_glows_of_size_zero() {
    let e: LayerEffects = serde_json::from_value(serde_json::json!({
        "shadow": { "angle": 90, "blue": 0, "blur": 0, "distance": 0, "green": 0, "opacity": 0.5, "red": 0 },
        "innerShadow": { "angle": 90, "blue": 0, "blur": 10, "distance": 10, "green": 0, "opacity": 0, "red": 0 },
        "outerGlow": { "blue": 1, "green": 1, "opacity": 0.75, "red": 1, "size": 0 },
        "innerGlow": { "blue": 1, "green": 1, "opacity": 0.75, "red": 1, "size": 0.5 } })).unwrap();
    let p = EffectPasses::from_effects(&e);
    assert!(p.shadow.is_some(), "a shadow with no distance and no blur still draws, under the layer");
    assert_eq!((p.inner_shadow, p.outer_glow), (None, None));
    assert_eq!(p.inner_glow.unwrap().sigma, 0.25);
}
