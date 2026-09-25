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
