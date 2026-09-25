//! The adjustment kinds Mac 1.2.6 added, drawn: the four per-pixel ones here (Gaussian and Motion
//! Blur are spatial, spatial_adjustments.rs). Expected values come from the Mac's C kernels,
//! transcribed below independently of the engine.
use compositor_engine::*;

/// Eight asymmetric premultiplied pixels: saturated and dull colours, several alphas, one clear.
const PIXELS: [[u8; 4]; 8] = [[200, 40, 90, 255], [30, 180, 60, 255], [90, 90, 200, 200], [250, 240, 20, 255],
    [10, 10, 10, 255], [128, 64, 32, 128], [0, 0, 0, 0], [60, 200, 200, 255]];

fn source() -> Raster { Raster::from_premultiplied(8, 1, PIXELS.concat()) }

/// The eight pixels with an adjustment layer of `a` over them, composited.
fn composited(a: LayerAdjustment) -> Raster {
    let mut doc = Document::new(8, 1);
    let mut adj = Layer::blank("Adjustment", doc.size());
    adj.extra.adjustment = Some(a);
    doc.layers = vec![Layer::with_pixels("P", source(), Point { x: 0.0, y: 0.0 }), adj];
    composite(&doc, Rect { x: 0.0, y: 0.0, width: 8.0, height: 1.0 }, 8, 1)
}

/// AdjustPixels.c:110-152.
fn c_black_white(p: [u8; 4], w: [f32; 6], tint: bool, tint_hue: f64, tint_saturation: f64) -> [u8; 4] {
    if p[3] == 0 { return p; }
    let alpha = p[3] as f32;
    let ch = |c: usize| (p[c] as f32 * 255.0 / alpha).min(255.0) / 255.0;
    let (r, g, b) = (ch(0), ch(1), ch(2));
    let mx = r.max(g.max(b)); let mn = r.min(g.min(b)); let md = r + g + b - mx - mn;
    let (primary, secondary) = if mx == r { (0, if g >= b { 1 } else { 5 }) }
        else if mx == g { (2, if r >= b { 1 } else { 3 }) } else { (4, if g >= r { 3 } else { 5 }) };
    let gray = (mn + (md - mn) * w[secondary] + (mx - md) * w[primary]).clamp(0.0, 1.0);
    let mut out = [gray; 3];
    if tint && tint_saturation > 0.0 {
        let c = (1.0 - (2.0 * gray as f64 - 1.0).abs()) * tint_saturation;
        let hp = (tint_hue % 360.0) / 60.0;
        let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
        let (r1, g1, b1) = if hp < 1.0 { (c, x, 0.0) } else if hp < 2.0 { (x, c, 0.0) } else if hp < 3.0 { (0.0, c, x) }
            else if hp < 4.0 { (0.0, x, c) } else if hp < 5.0 { (x, 0.0, c) } else { (c, 0.0, x) };
        let m = gray as f64 - c / 2.0;
        out = [(r1 + m).clamp(0.0, 1.0) as f32, (g1 + m).clamp(0.0, 1.0) as f32, (b1 + m).clamp(0.0, 1.0) as f32];
    }
    let v = |x: f32| (x * alpha).round().clamp(0.0, alpha) as u8;
    [v(out[0]), v(out[1]), v(out[2]), p[3]]
}

/// AdjustPixels.c:156-196.
fn c_color_balance(p: [u8; 4], s: [f32; 3], m: [f32; 3], h: [f32; 3], preserve: bool) -> [u8; 4] {
    if p[3] == 0 { return p; }
    let alpha = p[3] as f32;
    let weights = |v: f32| {
        let (a, b, scale) = (0.25f32, 0.333f32, 0.7f32);
        let sh = ((v - b) / -a + 0.5).clamp(0.0, 1.0);
        let hi = ((v + b - 1.0) / a + 0.5).clamp(0.0, 1.0);
        let m1 = ((v - b) / a + 0.5).clamp(0.0, 1.0);
        let m2 = ((v + b - 1.0) / -a + 0.5).clamp(0.0, 1.0);
        (sh * scale, m1 * m2 * scale, hi * scale)
    };
    let mut c = [0, 1, 2].map(|i| (p[i] as f32 * 255.0 / alpha).min(255.0) / 255.0);
    let before = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    for i in 0..3 {
        let (ws, wm, wh) = weights(c[i]);
        c[i] += s[i] * ws + m[i] * wm + h[i] * wh;
        c[i] = c[i].clamp(0.0, 1.0);
    }
    if preserve {
        let after = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        if after > 0.0001 { let ratio = before / after; for v in &mut c { *v = (*v * ratio).clamp(0.0, 1.0); } }
    }
    let v = |x: f32| (x * alpha).round().clamp(0.0, alpha) as u8;
    [v(c[0]), v(c[1]), v(c[2]), p[3]]
}

/// NoisePixels.c:26-47 for one pixel at field position (px, py).
fn c_noise(p: [u8; 4], px: u32, py: u32, amount: f32, gaussian: bool, mono: bool, seed: u32) -> [u8; 4] {
    if p[3] == 0 { return p; }
    let hash = |mut x: u32| { x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d); x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b); x ^= x >> 16; x };
    let unit = |key: u32| (hash(key) >> 8) as f32 * (1.0 / 16_777_216.0);
    let spread = amount / 100.0 * 127.5;
    let base = hash(seed ^ hash(px.wrapping_mul(0x9e37_79b9) ^ hash(py.wrapping_mul(0x85eb_ca6b))));
    let alpha = p[3] as f32;
    let mut out = p;
    for c in 0..3 {
        let key = if mono { base } else { base.wrapping_add((c as u32).wrapping_mul(0x9e37_79b9)) };
        let n = if gaussian { (-2.0 * (1.0 - unit(key)).ln()).sqrt() * (6.2831853 * unit(key ^ 0x68e3_1da4)).cos() * spread * (2.0 / 3.0) }
            else { (unit(key) * 2.0 - 1.0) * spread };
        out[c] = ((p[c] as f32 * 255.0 / alpha + n).clamp(0.0, 255.0) * alpha / 255.0).round() as u8;
    }
    out
}

fn custom_black_white(tint: bool) -> BlackWhiteSettings {
    BlackWhiteSettings { reds: 115.0, yellows: -40.0, greens: 70.0, cyans: 180.0, blues: -90.0, magentas: 20.0,
        tint, tint_hue: 205.0, tint_saturation: 45.0 }
}
fn weights(s: &BlackWhiteSettings) -> [f32; 6] { [s.reds, s.yellows, s.greens, s.cyans, s.blues, s.magentas].map(|w| (w / 100.0) as f32) }
fn custom_balance(preserve: bool) -> ColorBalanceSettings {
    ColorBalanceSettings { shadow_cyan_red: 40.0, shadow_magenta_green: -20.0, shadow_yellow_blue: 30.0,
        mid_cyan_red: -35.0, mid_magenta_green: 25.0, mid_yellow_blue: -15.0,
        highlight_cyan_red: 20.0, highlight_magenta_green: 45.0, highlight_yellow_blue: -50.0, preserve_luminosity: preserve }
}

#[test]
fn black_and_white_matches_the_mac_kernel_with_and_without_tint() {
    let mut outputs = Vec::new();
    for tint in [false, true] {
        let s = custom_black_white(tint);
        let mut a = LayerAdjustment::new(AdjustmentKind::BlackWhite);
        a.black_white_settings = Some(s);
        let out = composited(a);
        for (x, p) in PIXELS.iter().enumerate() {
            assert_eq!(out.pixel(x as u32, 0), c_black_white(*p, weights(&s), s.tint, s.tint_hue, s.tint_saturation / 100.0), "tint {tint}, pixel {x}");
        }
        outputs.push(out);
    }
    assert_ne!(outputs[0].bytes(), outputs[1].bytes(), "the fixture shows the tint");
}

#[test]
fn color_balance_matches_the_mac_kernel_and_preserve_luminosity_changes_it() {
    let mut outputs = Vec::new();
    for preserve in [true, false] {
        let s = custom_balance(preserve);
        let mut a = LayerAdjustment::new(AdjustmentKind::ColorBalance);
        a.color_balance_settings = Some(s);
        let out = composited(a);
        let d = |v: [f64; 3]| v.map(|x| (x / 100.0) as f32);
        for (x, p) in PIXELS.iter().enumerate() {
            let want = c_color_balance(*p, d([s.shadow_cyan_red, s.shadow_magenta_green, s.shadow_yellow_blue]),
                d([s.mid_cyan_red, s.mid_magenta_green, s.mid_yellow_blue]),
                d([s.highlight_cyan_red, s.highlight_magenta_green, s.highlight_yellow_blue]), preserve);
            assert_eq!(out.pixel(x as u32, 0), want, "preserve {preserve}, pixel {x}");
        }
        outputs.push(out);
    }
    assert_ne!(outputs[0].bytes(), outputs[1].bytes(), "the fixture shows Preserve Luminosity");
}

#[test]
fn invert_is_alpha_minus_colour() {
    let out = composited(LayerAdjustment::new(AdjustmentKind::Invert));
    for (x, p) in PIXELS.iter().enumerate() {
        assert_eq!(out.pixel(x as u32, 0), [p[3] - p[0], p[3] - p[1], p[3] - p[2], p[3]], "pixel {x}");
    }
}

#[test]
fn add_noise_is_a_field_fixed_in_the_document() {
    for (amount, gaussian, mono, seed) in [(35.0, false, false, 7u32), (60.0, true, true, 4242)] {
        let mut a = LayerAdjustment::new(AdjustmentKind::AddNoise);
        a.noise_amount = Some(amount); a.noise_gaussian = Some(gaussian); a.noise_monochromatic = Some(mono); a.noise_seed = Some(seed);
        let out = composited(a.clone());
        for (x, p) in PIXELS.iter().enumerate() {
            assert_eq!(out.pixel(x as u32, 0), c_noise(*p, x as u32, 0, amount as f32, gaussian, mono, seed), "pixel {x}");
        }
        // Composited from x = 3 on, those pixels take the same noise: the field is the document's.
        let mut doc = Document::new(8, 1);
        let mut adj = Layer::blank("Noise", doc.size());
        adj.extra.adjustment = Some(a);
        doc.layers = vec![Layer::with_pixels("P", source(), Point { x: 0.0, y: 0.0 }), adj];
        let part = composite(&doc, Rect { x: 3.0, y: 0.0, width: 5.0, height: 1.0 }, 5, 1);
        assert_eq!(part.bytes(), out.cropped(3, 0, 5, 1).bytes());
    }
}

#[test]
fn a_partial_strength_layer_moves_each_colour_part_way() {
    // Inside a 40% folder the Mac blends the adjusted image with the original by 0.4 (R 3.4 step 4).
    let s = custom_black_white(false);
    let mut doc = Document::new(8, 1);
    let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true; folder.opacity = 0.4;
    let mut adj = Layer::blank("BW", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::BlackWhite); a.black_white_settings = Some(s);
    adj.extra.adjustment = Some(a); adj.parent_id = Some(folder.id);
    doc.layers = vec![Layer::with_pixels("P", source(), Point { x: 0.0, y: 0.0 }), folder, adj];
    let out = composite(&doc, Rect { x: 0.0, y: 0.0, width: 8.0, height: 1.0 }, 8, 1);
    for (x, p) in PIXELS.iter().enumerate() {
        let full = c_black_white(*p, weights(&s), false, 0.0, 0.0);
        for c in 0..4 {
            let want = p[c] as f32 + (full[c] as f32 - p[c] as f32) * 0.4;
            assert!((out.pixel(x as u32, 0)[c] as f32 - want).abs() <= 1.0, "pixel {x} channel {c}");
        }
    }
}

#[test]
fn applying_one_to_a_layer_gives_what_the_adjustment_layer_shows() {
    let mut bw = LayerAdjustment::new(AdjustmentKind::BlackWhite); bw.black_white_settings = Some(custom_black_white(true));
    let mut cb = LayerAdjustment::new(AdjustmentKind::ColorBalance); cb.color_balance_settings = Some(custom_balance(true));
    for a in [bw, cb, LayerAdjustment::new(AdjustmentKind::Invert)] {
        let mut doc = Document::new(8, 1);
        let layer = Layer::with_pixels("P", source(), Point { x: 0.0, y: 0.0 });
        let id = layer.id;
        doc.layers = vec![layer];
        ops::adjust::apply_adjustment_to_layer(&mut doc, id, &a).unwrap();
        assert_eq!(doc.layers[0].pixels.as_ref().unwrap().bytes(), composited(a.clone()).bytes(), "{:?}", a.kind);
    }
}

#[test]
fn a_blur_is_not_applied_through_apply_adjustment() {
    let mut doc = Document::new(8, 1);
    let layer = Layer::with_pixels("P", source(), Point { x: 0.0, y: 0.0 });
    let id = layer.id;
    doc.layers = vec![layer];
    for kind in [AdjustmentKind::GaussianBlur, AdjustmentKind::MotionBlur] {
        assert!(ops::adjust::apply_adjustment_to_layer(&mut doc, id, &LayerAdjustment::new(kind)).is_err(), "{kind:?}");
    }
}

#[test]
fn a_new_add_noise_layer_takes_its_seed_and_the_kinds_say_what_they_are() {
    let mut doc = Document::new(8, 1);
    ops::adjust::add_adjustment_layer(&mut doc, AdjustmentKind::AddNoise, 3_000_000_007, None).unwrap();
    assert_eq!(doc.layers[0].extra.adjustment.as_ref().unwrap().noise_seed, Some(3_000_000_007));
    assert!(!AdjustmentKind::Invert.is_editable() && AdjustmentKind::BlackWhite.is_editable());
    assert!(AdjustmentKind::GaussianBlur.is_spatial() && AdjustmentKind::MotionBlur.is_spatial() && !AdjustmentKind::AddNoise.is_spatial());
}
