use compositor_engine::*;
use serde_json::{json, Value};

/// The exact raw strings of LayerBlendMode in Mac 1.2.6 (LayerAppearance.swift:4-13, R 2.4).
/// Note the parentheses in "Linear Dodge (Add)".
const NEW: [&str; 11] = ["Linear Burn", "Linear Dodge (Add)", "Soft Light", "Hard Light", "Vivid Light",
    "Linear Light", "Pin Light", "Hard Mix", "Exclusion", "Subtract", "Divide"];

fn one_layer(version: u32, blend: &str) -> Package {
    let id = "0B6C6B1E-4F1B-4B4E-9E0A-222222222222";
    let manifest = json!({
        "format": "com.compositor.project", "version": version, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-111111111111", "width": 3, "height": 2,
        "layers": [{ "id": id, "name": "L", "isVisible": true, "imageFile": format!("{id}.png"), "blendMode": blend,
            "transform": { "origin": [0, 0], "size": [3, 2], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" } }]
    });
    // Valid premultiplied data: no colour channel above alpha.
    let png = encode_png(&Raster::from_premultiplied(3, 2, [20u8, 80, 40, 128].repeat(6)), 72.0).unwrap();
    Package { manifest_json: manifest.to_string(), images: vec![(format!("{id}.png"), png)] }
}

#[test]
fn every_new_blend_mode_string_round_trips_exactly() {
    for name in NEW {
        let doc = open_package(&one_layer(9, name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
        assert_eq!(saved["layers"][0]["blendMode"], json!(name));
    }
}

#[test]
fn a_new_mode_still_needs_version_3_like_every_non_normal_mode() {
    assert!(matches!(open_package(&one_layer(2, "Soft Light")), Err(ProjectError::Invalid)));
    // The positive control: at v3 the same file opens, so the refusal above is the version rule,
    // not a string that fails to parse.
    assert!(open_package(&one_layer(3, "Soft Light")).is_ok());
}

/// The W3C / PDF separable blend functions, plus the edge rules the Mac render showed (probe
/// results "Blend modes"), written out independently of blend.rs, on straight colours in 0..1.
/// This is a second transcription of the same published formulas, so it cannot catch a misreading
/// both copies share; the Mac's blend-greys render (mac_1_2_10.rs) is the oracle for Soft Light,
/// Hard Light, Linear Light, Pin Light, Vivid Light and Hard Mix on grey sources.
fn expected_blend(mode: &str, cb: f32, cs: f32) -> f32 {
    let burn = |b: f32, s: f32| if b >= 1.0 { 1.0 } else if s <= 0.0 { 0.0 } else { 1.0 - ((1.0 - b) / s).min(1.0) };
    let dodge = |b: f32, s: f32| if b <= 0.0 { 0.0 } else if s >= 1.0 { 1.0 } else { (b / (1.0 - s)).min(1.0) };
    match mode {
        "Linear Burn" => (cb + cs - 1.0).max(0.0),
        "Linear Dodge (Add)" => (cb + cs).min(1.0),
        // Core Image's, the W3C / PDF formula, as Compositor 1.4.5 draws it (LayerAppearance.swift:51-58);
        // 1.2.10 drew Pegtop's through Core Graphics.
        "Soft Light" => soft_light_w3c(cb, cs),
        "Hard Light" => if cs <= 0.5 { cb * 2.0 * cs } else { let s = 2.0 * cs - 1.0; cb + s - cb * s },
        "Vivid Light" => if cs <= 0.5 { burn(cb, 2.0 * cs) } else { dodge(cb, 2.0 * cs - 1.0) },
        "Linear Light" => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
        "Pin Light" => if cs <= 0.5 { cb.min(2.0 * cs) } else { cb.max(2.0 * cs - 1.0) },
        "Hard Mix" => if cb + cs > 1.0 + 0.5 / 255.0 { 1.0 } else { 0.0 },
        "Exclusion" => cb + cs - 2.0 * cb * cs,
        "Subtract" => (cb - cs).max(0.0),
        "Divide" => if cs <= 0.0 { if cb > 0.0 { 1.0 } else { 0.0 } } else { (cb / cs).min(1.0) },
        "Normal" => cs,
        other => panic!("no formula for {other}"),
    }
}

/// The W3C / PDF Soft Light, written out from the spec: `D(cb)` is the square root but for a dark
/// backdrop.
fn soft_light_w3c(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) }
    else { cb + (2.0 * cs - 1.0) * ((if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() }) - cb) }
}

/// One opaque `source` grey in `mode` over an opaque `backdrop` grey: the composite's red.
fn grey_over_grey(mode: BlendMode, backdrop: u8, source: u8) -> u8 {
    let mut doc = Document::new(1, 1);
    let under = Layer::with_pixels("B", Raster::from_premultiplied(1, 1, vec![backdrop, backdrop, backdrop, 255]), Point { x: 0.0, y: 0.0 });
    let mut over = Layer::with_pixels("S", Raster::from_premultiplied(1, 1, vec![source, source, source, 255]), Point { x: 0.0, y: 0.0 });
    over.blend_mode = mode;
    doc.layers = vec![under, over];
    composite(&doc, Rect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }, 1, 1).pixel(0, 0)[0]
}

#[test]
fn soft_light_is_the_w3c_formula_compositor_1_4_5_draws() {
    // GPUCanvasTests.softLightMatchesPhotoshop (GPUCanvasTests.swift:522-545 at v1.4.5): a 0.5 backdrop
    // under 0.9 exports 170, within 2. Pegtop's formula, 1.2.10's, gives 178 there.
    let (cb, cs) = (128u8, 230u8);
    let want = (soft_light_w3c(cb as f32 / 255.0, cs as f32 / 255.0) * 255.0).round() as u8;
    assert_eq!(grey_over_grey(BlendMode::SoftLight, cb, cs), want);
    assert!(want.abs_diff(170) <= 2, "the Mac's own bound: {want}");
    let pegtop = |b: f32, s: f32| (1.0 - 2.0 * s) * b * b + 2.0 * s * b;
    assert!(((pegtop(cb as f32 / 255.0, cs as f32 / 255.0) * 255.0).round() as u8).abs_diff(want) >= 6, "the fixture tells the two formulas apart");
    // Over a dark backdrop (cb <= 0.25) under light sources, W3C's D(cb) is not the square root
    // Photoshop uses: the soft-light-dark probe's case. Every pair is the formula, and some pair tells
    // it from the square root. Awaiting B2 (soft-light-dark): the Mac's own comment calls Core Image's
    // Soft Light only "within 5" of Photoshop's square-root D, so this dark branch is pinned as fact
    // from the spec text, not yet checked against a Mac export.
    let mut discriminates = false;
    for backdrop in [0u8, 16, 40, 64] {
        for source in [128u8, 192, 255] {
            let (b, s) = (backdrop as f32 / 255.0, source as f32 / 255.0);
            let want = (soft_light_w3c(b, s) * 255.0).round() as u8;
            assert!(grey_over_grey(BlendMode::SoftLight, backdrop, source).abs_diff(want) <= 1, "{backdrop} under {source}");
            let root = ((b + (2.0 * s - 1.0) * (b.sqrt() - b)) * 255.0).round() as u8;
            discriminates |= root.abs_diff(want) >= 4;
        }
    }
    assert!(discriminates, "the dark backdrops tell W3C's D(cb) from a square root");
}

#[test]
fn each_new_mode_composites_by_its_formula_over_an_opaque_backdrop() {
    // An asymmetric backdrop and three translucent greys at alpha 153 (0.6), premultiplied so that
    // cs = byte / alpha is exactly what the compositor reads: about 0.25, 0.6 and 0.85.
    let backdrop = [200u8, 90, 30, 255];
    let sources: [[u8; 4]; 3] = [[38, 38, 38, 153], [92, 92, 92, 153], [130, 130, 130, 153]];
    let predict = |mode: &str, s: [u8; 4]| -> [i64; 3] {
        let a = s[3] as f32 / 255.0;
        [0, 1, 2].map(|c| {
            let cb = backdrop[c] as f32 / 255.0;
            let cs = s[c] as f32 / s[3] as f32;
            (((1.0 - a) * cb + a * expected_blend(mode, cb, cs).clamp(0.0, 1.0)) * 255.0).round() as i64
        })
    };
    let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
    for mode in NEW {
        let mut discriminates = false;
        for s in sources {
            let mut doc = Document::new(2, 1);
            let under = Layer::with_pixels("B", Raster::from_premultiplied(2, 1, backdrop.repeat(2)), Point { x: 0.0, y: 0.0 });
            let mut over = Layer::with_pixels("S", Raster::from_premultiplied(2, 1, s.repeat(2)), Point { x: 0.0, y: 0.0 });
            over.blend_mode = serde_json::from_value(json!(mode)).unwrap();
            doc.layers = vec![under, over];
            let got = composite(&doc, region, 2, 1).pixel(1, 0);
            let want = predict(mode, s);
            for c in 0..3 {
                assert!((got[c] as i64 - want[c]).abs() <= 1, "{mode} over {s:?}, channel {c}: {} vs {}", got[c], want[c]);
            }
            let normal = predict("Normal", s);
            discriminates |= (0..3).any(|c| (want[c] - normal[c]).abs() >= 4);
        }
        assert!(discriminates, "{mode}: the fixture must tell this mode from Normal, or the test cannot fail");
    }
}

/// A Levels adjustment that sends every channel to 128: Normal gives grey, while Linear Burn or
/// Divide of that grey over the backdrop gives something else, so the two are told apart.
fn levels_to_mid_grey() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0].output_black = 128.0;
    a.levels.ranges[0].output_white = 128.0;
    a
}

const CORE_IMAGE_ONLY: [BlendMode; 8] = [BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::VividLight,
    BlendMode::LinearLight, BlendMode::PinLight, BlendMode::HardMix, BlendMode::Subtract, BlendMode::Divide];

/// A mode's name as the formulas above spell it.
fn name(mode: BlendMode) -> String { serde_json::to_value(mode).unwrap().as_str().unwrap().to_string() }

#[test]
fn an_adjustment_layer_blends_in_its_own_mode() {
    // Compositor 1.4.5: an adjustment layer in any mode but Normal takes the full-coverage path that
    // keeps the original alpha (LiveMaskRenderer.swift:36-61), and blends its result in its real mode,
    // through Core Image for the modes Core Graphics lacks or gets wrong (SeparableBlend.blend,
    // SeparableBlend.swift:50-59; LiveMaskRenderer.swift:52-57). 1.2.10 drew the eight Core-Image-only
    // modes as Normal there (`cgMode`). Levels to mid grey over an opaque backdrop: each channel is the
    // mode's formula of the backdrop and 128.
    let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
    let backdrop = [200u8, 90, 30, 255];
    let with_mode = |mode: BlendMode| {
        let mut doc = Document::new(2, 1);
        let under = Layer::with_pixels("B", Raster::from_premultiplied(2, 1, backdrop.repeat(2)), Point { x: 0.0, y: 0.0 });
        let mut adj = Layer::blank("Levels", doc.size());
        adj.extra.adjustment = Some(levels_to_mid_grey());
        adj.blend_mode = mode;
        doc.layers = vec![under, adj];
        (render_plan(&doc, None), composite(&doc, region, 2, 1).pixel(0, 0))
    };
    let (plan, normal) = with_mode(BlendMode::Normal);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
    assert!(!draw.keeps_alpha, "Normal composites through its coverage, alpha and all");
    for mode in CORE_IMAGE_ONLY.into_iter().chain([BlendMode::SoftLight]) {
        let (plan, px) = with_mode(mode);
        let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
        assert_eq!(draw.blend, mode, "{mode:?} reaches both renderers as itself");
        assert!(draw.keeps_alpha, "{mode:?} is not Normal, so the original alpha is kept");
        let want: Vec<u8> = (0..3).map(|c| (expected_blend(&name(mode), backdrop[c] as f32 / 255.0, 128.0 / 255.0).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
        for c in 0..3 { assert!(px[c].abs_diff(want[c]) <= 1, "{mode:?}, channel {c}: {px:?} against {want:?}"); }
        assert_eq!(px[3], 255);
        assert_ne!(px, normal, "{mode:?} is not drawn as Normal");
    }
}

#[test]
fn a_clipping_stack_composites_in_its_bases_own_mode() {
    // LiveMaskRenderer.swift:89 (`stackModes` holds the real mode) and :126-134 (the group through
    // SeparableBlend.draw when the mode needs a surface), at v1.4.5; 1.2.10 stored `cgMode`. The
    // group is the child over the base, opaque where the base is; it blends over the backdrop by the
    // base's own formula.
    let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
    let (backdrop, base_px, child_px) = ([200u8, 90, 30, 255], [60u8, 150, 110, 255], [40u8, 20, 90, 128]);
    let stack = |mode: BlendMode| {
        let mut doc = Document::new(2, 1);
        let under = Layer::with_pixels("Backdrop", Raster::from_premultiplied(2, 1, backdrop.repeat(2)), Point { x: 0.0, y: 0.0 });
        let mut base = Layer::with_pixels("Base", Raster::from_premultiplied(2, 1, base_px.repeat(2)), Point { x: 0.0, y: 0.0 });
        base.blend_mode = mode;
        let mut child = Layer::with_pixels("Child", Raster::from_premultiplied(2, 1, child_px.repeat(2)), Point { x: 0.0, y: 0.0 });
        child.mask_source_id = Some(base.id);
        doc.layers = vec![under, base, child];
        (render_plan(&doc, None), composite(&doc, region, 2, 1).pixel(0, 0))
    };
    let group: Vec<f32> = (0..3).map(|c| (child_px[c] as f32 + base_px[c] as f32 * (1.0 - child_px[3] as f32 / 255.0)) / 255.0).collect();
    let (_, normal) = stack(BlendMode::Normal);
    for mode in CORE_IMAGE_ONLY {
        let (plan, px) = stack(mode);
        let PlanNode::Stack { base, .. } = &plan.nodes[1] else { panic!("base and child form a stack") };
        assert_eq!(base.blend, mode, "{mode:?}");
        let want: Vec<u8> = (0..3).map(|c| (expected_blend(&name(mode), backdrop[c] as f32 / 255.0, group[c]).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
        for c in 0..3 { assert!(px[c].abs_diff(want[c]) <= 1, "{mode:?}, channel {c}: {px:?} against {want:?}"); }
        assert_ne!(px, normal, "{mode:?} is not drawn as Normal");
    }
}

#[test]
fn a_linear_dodge_stack_exports_the_macs_own_numbers() {
    // GPUCanvasTests.clippingStacksBlendInTheirBasesMode (GPUCanvasTests.swift:477-514 at v1.4.5):
    // Base (0.4, 0.2, 0.1); Blended (0.3, 0.3, 0.3) in Linear Dodge, the stack's base; Clipped (0.2,
    // 0.05, 0), 50 x 50 at the corner, clipped to Blended. Exported: (153, 64, 26) where Clipped
    // covers, (179, 128, 102) where only Blended does, each within 1.
    let byte = |v: f64| (v * 255.0).round() as u8;
    let solid = |name: &str, rgb: [f64; 3], size: u32| Layer::with_pixels(name, Raster::from_premultiplied(size, size, [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255].repeat((size * size) as usize)), Point { x: 0.0, y: 0.0 });
    let mut doc = Document::new(100, 100);
    let base = solid("Base", [0.4, 0.2, 0.1], 100);
    let mut blended = solid("Blended", [0.3, 0.3, 0.3], 100);
    blended.blend_mode = BlendMode::LinearDodge;
    let mut clipped = solid("Clipped", [0.2, 0.05, 0.0], 50);
    clipped.mask_source_id = Some(blended.id);
    doc.layers = vec![base, blended, clipped];
    let at = |x: f64, y: f64| composite(&doc, Rect { x, y, width: 1.0, height: 1.0 }, 1, 1).pixel(0, 0);
    let (covered, bare) = (at(20.0, 20.0), at(80.0, 80.0));
    for (c, want) in [153u8, 64, 26].into_iter().enumerate() { assert!(covered[c].abs_diff(want) <= 1, "covered {covered:?}"); }
    for (c, want) in [179u8, 128, 102].into_iter().enumerate() { assert!(bare[c].abs_diff(want) <= 1, "bare {bare:?}"); }
}
