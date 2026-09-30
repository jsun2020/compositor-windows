//! The Mac's own renders of the probe projects this port wrote (Compositor 1.2.10, exported at 100%
//! as PNG), committed under tests/fixtures/mac-1.2.10-probes (probe results). Each test opens the
//! probe as the Mac did and compares this port's composite with the Mac PNG in straight RGBA8, the
//! form the probe results compared.
use compositor_engine::*;
use std::ops::Range;

fn fixtures() -> String { format!("{}/tests/fixtures/mac-1.2.10-probes", env!("CARGO_MANIFEST_DIR")) }

/// This port's composite of the probe, premultiplied.
fn composite_of(name: &str) -> Raster {
    let comp = format!("{}/{name}.comp", fixtures());
    let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    let doc = open_package(&Package { manifest_json, images }).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let region = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    composite(&doc, region, doc.width, doc.height)
}

/// This port's composite of the probe, straight RGBA8.
fn ours(name: &str) -> Vec<u8> { composite_of(name).to_straight() }

/// The Mac's export: width, and straight RGBA8 as the PNG stores it.
fn mac(name: &str) -> (u32, Vec<u8>) {
    let png = std::fs::read(format!("{}/{name}.mac-1.2.10.png", fixtures())).unwrap();
    let rgba = image::load_from_memory(&png).unwrap().to_rgba8();
    (rgba.width(), rgba.into_raw())
}

/// The largest per-channel difference within `columns`, and where it is.
fn worst(a: &[u8], b: &[u8], width: u32, columns: Range<u32>) -> (u8, (u32, u32)) {
    assert_eq!(a.len(), b.len(), "the port and the Mac render the same size");
    let height = a.len() as u32 / 4 / width;
    let mut out = (0u8, (0, 0));
    for y in 0..height { for x in columns.clone() {
        let i = ((y * width + x) * 4) as usize;
        for c in 0..4 { let d = a[i + c].abs_diff(b[i + c]); if d > out.0 { out = (d, (x, y)); } }
    }}
    out
}

#[test]
fn the_new_blend_modes_match_the_mac_render() {
    // Eleven 20-px columns, one per mode in blend_modes_v9.rs NEW order; Soft Light is column 2.
    let (width, theirs) = mac("new-blend-modes");
    let port = ours("new-blend-modes");
    let (d, at) = worst(&port, &theirs, width, 0..40);
    assert_eq!(d, 0, "Linear Burn and Linear Dodge, worst at {at:?}");
    let (d, at) = worst(&port, &theirs, width, 60..width);
    assert_eq!(d, 0, "Hard Light to Divide, worst at {at:?}");
    let (d, at) = worst(&port, &theirs, width, 40..60);
    assert!(d <= 1, "Soft Light within one level: {d} at {at:?}");
}

#[test]
fn grain_at_its_defaults_matches_the_mac_render_exactly() {
    // The probe results found the 1.2.6 kernel equal to the Mac on all 7200 pixels.
    let (width, theirs) = mac("grain");
    let (d, at) = worst(&ours("grain"), &theirs, width, 0..width);
    assert_eq!(d, 0, "worst at {at:?}");
}

#[test]
fn black_and_white_at_its_defaults_matches_the_mac_render_exactly() {
    // The probe results found the Photoshop-default weights equal to the Mac on all 7200 pixels.
    let (width, theirs) = mac("new-adjustment-layers");
    let (d, at) = worst(&ours("new-adjustment-layers"), &theirs, width, 0..width);
    assert_eq!(d, 0, "worst at {at:?}");
}

#[test]
fn the_edited_rich_file_with_two_drop_shadows_matches_the_mac_render() {
    // Compared premultiplied: the Mac's straight colour is ill-conditioned where its alpha is a few
    // units (probe results, edited-rich-file). Measured 2026-09-25 with this kernel (scratch crate
    // p35c-scratch): colour 1, alpha 3; the alpha is the radius-24 blur layer's halved path (3.5b),
    // which the exact blur held to 2. Without the shadows the alpha is 43 off; with sigma = blur
    // instead of blur / 2, 10; with the shadow cast upward, 41.
    let (width, theirs) = mac("edited-rich-file");
    let theirs = Raster::from_straight(width, theirs.len() as u32 / 4 / width, &theirs);
    let comp = format!("{}/edited-rich-file.comp", fixtures());
    let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    let doc = open_package(&Package { manifest_json, images }).unwrap();
    let ours = composite(&doc, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height);
    assert_eq!(ours.bytes().len(), theirs.bytes().len(), "the port and the Mac render the same size");
    let (mut colour, mut alpha) = (0u8, 0u8);
    for (i, (a, b)) in ours.bytes().iter().zip(theirs.bytes()).enumerate() {
        let d = a.abs_diff(*b);
        if i % 4 == 3 { alpha = alpha.max(d); } else { colour = colour.max(d); }
    }
    assert!(colour <= 1, "colour {colour}");
    assert!(alpha <= 3, "alpha {alpha}");
}

#[test]
fn the_probes_this_port_already_matched_still_match() {
    for name in ["folder-opacity", "clipped-in-dimmed-folder", "guides"] {
        let (width, theirs) = mac(name);
        let (d, at) = worst(&ours(name), &theirs, width, 0..width);
        assert_eq!(d, 0, "{name}: worst at {at:?}");
    }
}

/// This port's composite of the probe and the Mac's export, both PREMULTIPLIED RGBA8: the worst
/// colour and alpha differences, and the mean difference over every byte. The Mac's straight
/// colour is ill-conditioned where its alpha is a few units (probe results), so blurs compare here.
fn premultiplied(name: &str) -> (u8, u8, f64) {
    let (width, theirs) = mac(name);
    let theirs = Raster::from_straight(width, theirs.len() as u32 / 4 / width, &theirs);
    let ours = composite_of(name);
    assert_eq!(ours.bytes().len(), theirs.bytes().len(), "{name}: the port and the Mac render the same size");
    let (mut colour, mut alpha, mut sum) = (0u8, 0u8, 0u64);
    for (i, (a, b)) in ours.bytes().iter().zip(theirs.bytes()).enumerate() {
        let d = a.abs_diff(*b);
        if i % 4 == 3 { alpha = alpha.max(d); } else { colour = colour.max(d); }
        sum += d as u64;
    }
    (colour, alpha, sum as f64 / ours.bytes().len() as f64)
}

// The Phase 3.5b follow-up probes, exported by Compositor for Mac 1.2.10 on 2026-09-27 (probe
// results, "Phase 3.5b follow-up probes"). Every bound below was measured on p4a-scratch against
// this build (2026-09-27), and matches the probe results' own comparison at 12a0e34.

#[test]
fn the_follow_up_probes_this_port_draws_exactly_match_the_mac_render_bit_for_bit() {
    // Color Balance without Preserve Luminosity, both Add Noise modes, a Levels layer in Color Dodge
    // (Core Graphics' own formula agrees with this port's W3C one here, and 1.4.5 draws it through Core
    // Image), and an Invert layer. `cgmode-levels-divide` and `cgmode-stack-bases` left this list in
    // Phase 4.5: Compositor 1.4.5 blends them in their real modes (below).
    for name in ["color-balance-no-preserve", "add-noise-uniform", "add-noise-gaussian-mono", "color-dodge-adjustment", "invert"] {
        let (width, theirs) = mac(name);
        let (d, at) = worst(&ours(name), &theirs, width, 0..width);
        assert_eq!(d, 0, "{name}: worst at {at:?}");
    }
}

#[test]
fn a_tinted_black_and_white_layer_matches_the_mac_render_within_one_level() {
    // Measured 1, on two pixels.
    let (width, theirs) = mac("black-white-tint");
    let (d, at) = worst(&ours("black-white-tint"), &theirs, width, 0..width);
    assert!(d <= 1, "worst {d} at {at:?}");
}

/// The largest difference over the colour channels of the pixels `inside` picks, straight RGBA8.
fn worst_where(a: &[u8], b: &[u8], width: u32, inside: impl Fn(u32, u32) -> bool) -> u8 {
    let mut d = 0u8;
    for (p, (x, y)) in (0..a.len() / 4).map(|p| (p, ((p as u32) % width, (p as u32) / width))) {
        if !inside(x, y) { continue; }
        for c in 0..3 { d = d.max(a[p * 4 + c].abs_diff(b[p * 4 + c])); }
    }
    d
}

// Compositor 1.4.5 blends adjustment layers and clipping stacks in their real modes (LiveMaskRenderer.swift
// :52-57, :89, :126-134 at v1.4.5), where 1.2.10, whose exports these are, drew the eight modes only Core
// Image computes as Normal. Until their 1.4.5 re-exports (B1) arrive, the parts 1.4.5 draws differently
// are held to the formulas and shown to differ from these exports; the rest still matches them.

#[test]
fn a_levels_layer_in_divide_blends_in_divide() {
    // Levels to mid grey (128) at 60% over the tonal sweep: each channel moves 60% of the way to the
    // backdrop divided by 128/255 (at most 1).
    let (width, theirs) = mac("cgmode-levels-divide");
    let port = ours("cgmode-levels-divide");
    let doc = open(&format!("{}/cgmode-levels-divide.comp", fixtures()));
    let sweep = doc.layers[0].pixels.as_ref().unwrap();
    let grey = 128.0 / 255.0;
    let mut want = port.clone();
    for y in 0..sweep.height { for x in 0..sweep.width {
        let p = sweep.pixel(x, y);
        for c in 0..3 {
            let cb = p[c] as f64 / 255.0;
            let divided = (cb / grey).min(1.0);
            want[((y * width + x) * 4) as usize + c] = ((cb + (divided - cb) * 0.6) * 255.0).round() as u8;
        }
    }}
    assert!(worst_where(&port, &want, width, |_, _| true) <= 1, "Divide at 60%");
    assert!(worst_where(&port, &theirs, width, |_, _| true) >= 4, "1.2.10 drew it as Normal");
}

#[test]
fn a_clipping_stack_based_in_subtract_blends_in_subtract() {
    // Two stacks over the hue sweep: an opaque (60, 150, 110) base, 100 x 100 at (10, 10), in Subtract,
    // then one in Color Burn at (130, 10), each under a half-alpha (40, 20, 90) child 60 px wide, 20 px
    // in from the base's left edge. The Color Burn stack is what 1.2.10 drew too (Core Graphics' own
    // formula agreed with this port's there): bit for bit. The Subtract one is the backdrop minus the
    // group (the child over the base), at least 0.
    let (width, theirs) = mac("cgmode-stack-bases");
    let port = ours("cgmode-stack-bases");
    assert_eq!(worst_where(&port, &theirs, width, |x, _| x >= 120), 0, "the Color Burn stack");
    let doc = open(&format!("{}/cgmode-stack-bases.comp", fixtures()));
    let sweep = doc.layers[0].pixels.as_ref().unwrap();
    let (base, child) = ([60.0f64, 150.0, 110.0], [40.0f64, 20.0, 90.0, 128.0]);
    let mut want = port.clone();
    for y in 10..110u32 { for x in 10..110u32 {
        let p = sweep.pixel(x, y);
        for c in 0..3 {
            let group = if (30..90).contains(&x) { child[c] + base[c] * (1.0 - child[3] / 255.0) } else { base[c] };
            want[((y * width + x) * 4) as usize + c] = (p[c] as f64 - group).max(0.0).round() as u8;
        }
    }}
    assert!(worst_where(&port, &want, width, |x, _| x < 120) <= 1, "Subtract");
    assert!(worst_where(&port, &theirs, width, |x, _| x < 120) >= 4, "1.2.10 drew the Subtract stack as Normal");
}

#[test]
fn a_blur_layer_in_linear_burn_keeps_the_original_alpha_as_the_mac_does() {
    // Ruling E-I1 confirmed by the Mac: the canvas edge does not fade (alpha exact against the 1.2.10
    // export; 1.4.5 keeps the original alpha the same way, LiveMaskRenderer.swift:58). The colour is now
    // Linear Burn of the original and its blur, both made opaque, the original alpha put back
    // (`blended_keeping_alpha`), worked out here from this port's own composite without the blur layer
    // and with it in Normal (the blur itself).
    let (width, theirs) = mac("cgmode-blur-linear-burn");
    let port = composite_of("cgmode-blur-linear-burn");
    let straight = port.to_straight();
    let alpha = (0..straight.len() / 4).map(|p| straight[p * 4 + 3].abs_diff(theirs[p * 4 + 3])).max().unwrap();
    assert_eq!(alpha, 0, "alpha");
    assert_eq!(width, 160);
    let comp = format!("{}/cgmode-blur-linear-burn.comp", fixtures());
    let region = Rect { x: 0.0, y: 0.0, width: 160.0, height: 100.0 };
    let mut doc = open(&comp);
    doc.layers[2].visible = false;
    let original = composite(&doc, region, 160, 100);
    doc.layers[2].visible = true;
    doc.layers[2].blend_mode = BlendMode::Normal;
    let blurred = composite(&doc, region, 160, 100);
    let opaque = |p: &[u8], c: usize| { let a = p[3] as u32; if a == 0 { 0.0 } else { ((p[c] as u32 * 255 + a / 2) / a).min(255) as f64 / 255.0 } };
    let mut colour = 0u8;
    for ((o, b), got) in original.bytes().chunks_exact(4).zip(blurred.bytes().chunks_exact(4)).zip(port.bytes().chunks_exact(4)) {
        for c in 0..3 {
            let burnt = (opaque(o, c) + opaque(b, c) - 1.0).max(0.0);
            let want = (((burnt * 255.0).round() as u32 * o[3] as u32 + 127) / 255) as u8;
            colour = colour.max(got[c].abs_diff(want));
        }
    }
    assert!(colour <= 1, "Linear Burn of the original and its blur: {colour}");
    assert!(worst_where(&straight, &theirs, width, |_, _| true) >= 4, "1.2.10 drew the blur in Normal");
}
#[test]
fn the_gaussian_blur_probes_match_the_mac_render_premultiplied() {
    // Measured colour and alpha: radius 6 (the exact kernel) 2 and 2; radius 40 (the halved path) 2
    // and 3; radius 6 at 60% under a ramp mask 1 and 1.
    for (name, colour, alpha) in [("gaussian-blur-6", 2, 2), ("gaussian-blur-40", 2, 3), ("blur-soft-mask", 1, 1)] {
        let (c, a, _) = premultiplied(name);
        assert!(c <= colour && a <= alpha, "{name}: colour {c} alpha {a}, measured {colour} and {alpha}");
    }
}

#[test]
fn every_band_of_blend_greys_matches_the_mac_render() {
    // Six 60-row bands over a hue sweep, one per mode, each six 40-px grey columns: 25%, 50% and
    // 75% grey, opaque then at half alpha. Measured 1 in Hard Light, 0 in the other four; Soft Light
    // below.
    let (width, theirs) = mac("blend-greys");
    let port = ours("blend-greys");
    let worst_in = |rows: std::ops::Range<u32>, columns: &dyn Fn(u32) -> bool| {
        let mut d = 0u8;
        for y in rows { for x in (0..width).filter(|x| columns(*x)) { for c in 0..4 {
            let i = ((y * width + x) * 4 + c) as usize;
            d = d.max(port[i].abs_diff(theirs[i]));
        }}}
        d
    };
    for (band, mode, measured) in [(1u32, "Hard Light", 1u8), (2, "Linear Light", 0), (3, "Pin Light", 0), (4, "Vivid Light", 0), (5, "Hard Mix", 0)] {
        let d = worst_in(band * 60..band * 60 + 60, &|_| true);
        assert!(d <= measured, "{mode}: {d}, measured {measured}");
    }
    // Soft Light (band 0) is Core Image's W3C formula in Compositor 1.4.5 (blend.rs `soft_light`), where
    // 1.2.10 drew Pegtop's. The two agree for a source at or under half grey, so the 25% and 50%
    // columns still match this 1.2.10 export (measured 1); the 75% ones (x 80-120 and 200-240) are
    // checked against the formula below, and differ from this export, until its 1.4.5 re-export (B1).
    let bright = |x: u32| (80..120).contains(&x) || (200..240).contains(&x);
    assert!(worst_in(0..60, &|x| !bright(x)) <= 1, "Soft Light at 25% and 50% grey");
    assert!(worst_in(0..60, &bright) >= 4, "1.2.10's Pegtop at 75% grey is not what 1.4.5 draws");
    let doc = open(&format!("{}/blend-greys.comp", fixtures()));
    let sweep = doc.layers[0].pixels.as_ref().unwrap();
    let w3c = |cb: f64, cs: f64| if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) } else { cb + (2.0 * cs - 1.0) * ((if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() }) - cb) };
    let mut worst = 0u8;
    for column in [2u32, 5] {
        // The column's grey as the compositor reads it: premultiplied over its alpha.
        let source = doc.layers[1 + column as usize].pixels.as_ref().unwrap().pixel(0, 0);
        let (cs, a) = (source[0] as f64 / source[3] as f64, source[3] as f64 / 255.0);
        for y in 0..60 { for x in column * 40..column * 40 + 40 {
            let under = sweep.pixel(x, y);
            for c in 0..3 {
                let cb = under[c] as f64 / 255.0;
                let want = (((1.0 - a) * cb + a * w3c(cb, cs)) * 255.0).round() as u8;
                worst = worst.max(port[((y * width + x) * 4 + c as u32) as usize].abs_diff(want));
            }
        }}
    }
    assert!(worst <= 1, "Soft Light at 75% grey is the W3C formula: {worst}");
}

/// A probe project opened as the Mac would.
fn open(comp: &str) -> Document {
    let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    open_package(&Package { manifest_json, images }).unwrap()
}

#[test]
fn the_motion_blur_probe_matches_the_mac_render_as_a_gaussian_along_its_angle() {
    // 30 degrees, 24 px over a block running off the canvas and a translucent bar. The Mac passes
    // CIMotionBlur a radius of 24 / sqrt(12) and CIMotionBlur is a Gaussian of that sigma along the
    // angle (Filters.swift:170-208; probe results). Measured with `motion_blur` on p4a-scratch
    // (2026-09-27): colour 6, alpha 6, mean 0.195 per byte. The even streak this port drew before
    // measured 26, 28 and 3.108; a Gaussian cut at 4 sigmas instead of 3 was 6, 6 and 0.138, and one
    // sampled every quarter pixel 4, 4 and 0.103, at 4 to 8 times the taps.
    let (colour, alpha, mean) = premultiplied("motion-blur-30-24");
    assert!(colour <= 6 && alpha <= 6, "colour {colour} alpha {alpha}");
    assert!(mean <= 0.2, "mean {mean:.3}");
}

// The Phase 3.5c effects probes and color-balance-preserve, exported by Compositor for Mac 1.2.10 on
// 2026-09-27 (probe results, "Phase 3.5c effects probes and color-balance-preserve"). Measured
// premultiplied on p4a-scratch against this build (2026-09-27); the same as the controller's
// measurement at 00189cc.

#[test]
fn the_effects_probes_this_port_draws_exactly_match_the_mac_render_bit_for_bit() {
    for name in ["color-balance-preserve", "effects-stroke-outside", "effects-drop-shadow", "effects-inner-shadow",
        "effects-outer-glow", "effects-inner-glow", "effects-color-overlay", "effects-masked", "effects-clipping-base",
        "effects-clipped-child", "effects-folder", "effects-invalid"] {
        let (colour, alpha, _) = premultiplied(name);
        assert!(colour == 0 && alpha == 0, "{name}: colour {colour} alpha {alpha}");
    }
}

#[test]
fn the_other_effects_probes_match_the_mac_render_within_their_measured_bounds() {
    // Measured colour and alpha: an inside stroke 1 and 0; all six effects 1 and 0; a large blur (the
    // halved path) 1 and 1; a placed mask 3 and 2, where Core Graphics resamples it (rulings 14 / OQ3).
    for (name, colour, alpha) in [("effects-stroke-inside", 1, 0), ("effects-all-six", 1, 0), ("effects-large-blur", 1, 1), ("effects-mask-placed", 3, 2)] {
        let (c, a, _) = premultiplied(name);
        assert!(c <= colour && a <= alpha, "{name}: colour {c} alpha {a}, measured {colour} and {alpha}");
    }
}

#[test]
fn the_flipped_layer_of_effects_transformed_matches_the_mac_render_exactly() {
    // Columns 0-73 hold only the flipped layer and its effects: bit-identical. From column 74 the
    // turned layer (rotation 25, drawn at 150 %) and its shadow differ, by up to colour 29 and alpha
    // 52, because the Mac enlarges a High quality layer with Core Graphics' .high filter and this
    // port samples bilinearly (LayerRenderer.swift:42-44): an open item, until Phase 3.5d ports the
    // Mac's resampling filter (probe results, "Step probes"), not in Phase 4a's scope.
    let (width, theirs) = mac("effects-transformed");
    let (d, at) = worst(&ours("effects-transformed"), &theirs, width, 0..74);
    assert_eq!(d, 0, "worst at {at:?}");
    assert_eq!(width, 200);
}
