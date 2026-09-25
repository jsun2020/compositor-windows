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
/// both copies share. For Soft Light, Hard Light, Linear Light and Pin Light the first Mac probe
/// could not tell the formula apart (a pure-green source); the Task 12 blend-greys probe settles
/// them, and its render joins mac_1_2_10.rs when it comes back.
fn expected_blend(mode: &str, cb: f32, cs: f32) -> f32 {
    let burn = |b: f32, s: f32| if b >= 1.0 { 1.0 } else if s <= 0.0 { 0.0 } else { 1.0 - ((1.0 - b) / s).min(1.0) };
    let dodge = |b: f32, s: f32| if b <= 0.0 { 0.0 } else if s >= 1.0 { 1.0 } else { (b / (1.0 - s)).min(1.0) };
    match mode {
        "Linear Burn" => (cb + cs - 1.0).max(0.0),
        "Linear Dodge (Add)" => (cb + cs).min(1.0),
        "Soft Light" => if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) } else {
            let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
            cb + (2.0 * cs - 1.0) * (d - cb)
        },
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

#[test]
fn an_adjustment_layer_blends_in_the_core_graphics_mode() {
    // LiveMaskRenderer.adjust branches on the layer's OWN mode (LiveMaskRenderer.swift:24): any
    // mode but Normal takes the full-coverage path that keeps the original alpha, and only the
    // draw inside it goes through `cgMode` (:40), which is Normal for the eight modes only Core
    // Image computes (LayerAppearance.swift:46-47). Soft Light is a Core Graphics mode, so it
    // stays. Over an opaque backdrop a per-pixel kind keeps alpha 255 either way, so the eight
    // composite exactly as Normal does.
    let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
    let with_mode = |mode: BlendMode| {
        let mut doc = Document::new(2, 1);
        let under = Layer::with_pixels("B", Raster::from_premultiplied(2, 1, [200u8, 90, 30, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
        let mut adj = Layer::blank("Levels", doc.size());
        adj.extra.adjustment = Some(levels_to_mid_grey());
        adj.blend_mode = mode;
        doc.layers = vec![under, adj];
        (render_plan(&doc, None), composite(&doc, region, 2, 1).pixel(0, 0))
    };
    let (plan, normal) = with_mode(BlendMode::Normal);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
    assert!(!draw.keeps_alpha, "Normal composites through its coverage, alpha and all");
    for mode in CORE_IMAGE_ONLY {
        let (plan, px) = with_mode(mode);
        let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
        assert_eq!(draw.blend, BlendMode::Normal, "{mode:?} reaches both renderers as Normal");
        assert!(draw.keeps_alpha, "{mode:?} is not Normal, so the original alpha is kept");
        assert_eq!(px, normal, "{mode:?}");
    }
    let (plan, soft) = with_mode(BlendMode::SoftLight);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
    assert_eq!(draw.blend, BlendMode::SoftLight);
    assert!(draw.keeps_alpha);
    assert_ne!(soft, normal, "a Core Graphics mode still blends");
}

#[test]
fn a_clipping_stack_composites_in_its_base_core_graphics_mode() {
    // prepareStacks stores `blend(base).cgMode` (LiveMaskRenderer.swift:74) for the group composite.
    let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
    let stack = |mode: BlendMode| {
        let mut doc = Document::new(2, 1);
        let under = Layer::with_pixels("Backdrop", Raster::from_premultiplied(2, 1, [200u8, 90, 30, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
        let mut base = Layer::with_pixels("Base", Raster::from_premultiplied(2, 1, [60u8, 150, 110, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
        base.blend_mode = mode;
        let mut child = Layer::with_pixels("Child", Raster::from_premultiplied(2, 1, [40u8, 20, 90, 128].repeat(2)), Point { x: 0.0, y: 0.0 });
        child.mask_source_id = Some(base.id);
        doc.layers = vec![under, base, child];
        (render_plan(&doc, None), composite(&doc, region, 2, 1).pixel(0, 0))
    };
    let (_, normal) = stack(BlendMode::Normal);
    for mode in CORE_IMAGE_ONLY {
        let (plan, px) = stack(mode);
        let PlanNode::Stack { base, .. } = &plan.nodes[1] else { panic!("base and child form a stack") };
        assert_eq!(base.blend, BlendMode::Normal, "{mode:?}");
        assert_eq!(px, normal, "{mode:?}: Core Image only, so the stack composites as Normal");
    }
    let (_, multiply) = stack(BlendMode::Multiply);
    assert_ne!(multiply, normal, "a Core Graphics mode composites the stack in that mode");
}
