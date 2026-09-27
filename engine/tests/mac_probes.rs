//! Probe `.comp` projects for the user to open in Compositor for Mac (1.2.10 or later) (Task 7).
//! Each one is built and saved through this build's own `save_package`, so it is a real, valid v9
//! project; the ignored test below re-opens every one with `open_package` as a sanity floor,
//! then writes it (and a README telling the user what to do with it) to
//! `build-artifacts/mac-probes/`, which is git-ignored.
//!
//! The Mac's renders of the first seven are committed under tests/fixtures/mac-1.2.10-probes
//! and compared in mac_1_2_10.rs. The rest settle what those could not (the light blend modes,
//! Color Balance, the blurs, Add Noise, the cgMode path of adjustment layers and clipped groups)
//! and join mac_1_2_10.rs when their renders come back.

use compositor_engine::*;
use std::fs;
use std::path::{Path, PathBuf};

fn probes_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../build-artifacts/mac-probes"))
}

fn solid_rect(name: &str, rgba: [u8; 4], width: u32, height: u32, x: f64, y: f64) -> Layer {
    Layer::with_pixels(name, Raster::from_premultiplied(width, height, rgba.repeat((width * height) as usize)), Point { x, y })
}

fn folder(doc: &Document, opacity: f64) -> Layer {
    let mut f = Layer::blank("Folder", doc.size());
    f.is_group = true;
    f.opacity = opacity;
    f
}

/// A straight red-to-blue horizontal gradient, alpha 255 throughout.
fn red_to_blue_gradient(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _y in 0..height {
        for x in 0..width {
            let t = x as f64 / (width - 1).max(1) as f64;
            let r = (255.0 * (1.0 - t)).round() as u8;
            let b = (255.0 * t).round() as u8;
            data.extend_from_slice(&[r, 0, b, 255]);
        }
    }
    Raster::from_premultiplied(width, height, data)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (((r1 + m) * 255.0).round() as u8, ((g1 + m) * 255.0).round() as u8, ((b1 + m) * 255.0).round() as u8)
}

/// A full-hue sweep, for the two adjustment-layer probes: colourful enough that a Black & White
/// or Grain adjustment on top has something visible to act on.
fn colourful_gradient(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _y in 0..height {
        for x in 0..width {
            let (r, g, b) = hsv_to_rgb(360.0 * x as f64 / width as f64, 0.85, 0.95);
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Raster::from_premultiplied(width, height, data)
}

/// 1. The overlapping red/blue children in a 50% folder (Task 4's first test, at 120 x 60).
fn folder_opacity_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let f = folder(&doc, 0.5);
    let fid = f.id;
    let mut red = solid_rect("Red", [255, 0, 0, 255], 80, 60, 0.0, 0.0);
    red.parent_id = Some(fid);
    let mut blue = solid_rect("Blue", [0, 0, 255, 255], 80, 60, 40.0, 0.0);
    blue.parent_id = Some(fid);
    doc.layers = vec![f, red, blue];
    doc
}

/// 2. Task 4's double-dim case (a clipped child in a dimmed folder), at 120 x 60.
fn clipped_in_dimmed_folder_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let f = folder(&doc, 0.5);
    let fid = f.id;
    let mut base = solid_rect("Base", [255, 255, 255, 255], 120, 60, 0.0, 0.0);
    base.parent_id = Some(fid);
    let base_id = base.id;
    let mut child = solid_rect("Child", [255, 0, 0, 255], 120, 60, 0.0, 0.0);
    child.parent_id = Some(fid);
    child.mask_source_id = Some(base_id);
    doc.layers = vec![f, base, child];
    doc
}

/// 3. A white layer with a vertical guide at 30 and a horizontal guide at 45.5.
fn guides_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let white = solid_rect("Layer 1", [255, 255, 255, 255], 120, 60, 0.0, 0.0);
    doc.layers = vec![white];
    doc.guides = vec![
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Vertical, position: 30.0 },
        Guide { id: uuid::Uuid::new_v4(), axis: GuideAxis::Horizontal, position: 45.5 },
    ];
    doc
}

/// 4. Eleven half-alpha green columns, one per new blend mode in `NEW` order (see
/// `blend_modes_v9.rs`), over a red-to-blue gradient. A Phase 3.5b oracle.
fn new_blend_modes_doc() -> Document {
    const MODES: [BlendMode; 11] = [
        BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::VividLight, BlendMode::LinearLight, BlendMode::PinLight, BlendMode::HardMix,
        BlendMode::Exclusion, BlendMode::Subtract, BlendMode::Divide,
    ];
    let mut doc = Document::new(220, 60);
    let backdrop = Layer::with_pixels("Gradient", red_to_blue_gradient(220, 60), Point { x: 0.0, y: 0.0 });
    let mut layers = vec![backdrop];
    for (i, mode) in MODES.iter().enumerate() {
        let mut column = solid_rect(&format!("Column {i}"), [0, 128, 0, 128], 20, 60, (i as f64) * 20.0, 0.0);
        column.blend_mode = *mode;
        layers.push(column);
    }
    doc.layers = layers;
    doc
}

/// 5. A colourful gradient with a default Black & White adjustment layer on top. Its Mac render
/// is black_and_white_at_its_defaults_matches_the_mac_render_exactly (mac_1_2_10.rs).
fn new_adjustment_layers_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let backdrop = Layer::with_pixels("Gradient", colourful_gradient(120, 60), Point { x: 0.0, y: 0.0 });
    let mut adjustment = Layer::blank("Black & White", doc.size());
    adjustment.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::BlackWhite));
    doc.layers = vec![backdrop, adjustment];
    doc
}

/// 6. A colourful gradient with a default Grain adjustment layer on top. Its Mac render is
/// grain_at_its_defaults_matches_the_mac_render_exactly (mac_1_2_10.rs).
fn grain_doc() -> Document {
    let mut doc = Document::new(120, 60);
    let backdrop = Layer::with_pixels("Gradient", colourful_gradient(120, 60), Point { x: 0.0, y: 0.0 });
    let mut adjustment = Layer::blank("Grain", doc.size());
    adjustment.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Grain));
    doc.layers = vec![backdrop, adjustment];
    doc
}

/// A hue sweep across x with brightness rising down y, for the adjustment probes that act on
/// shadows, midtones and highlights differently.
fn tonal_sweep(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let (r, g, b) = hsv_to_rgb(360.0 * x as f64 / width as f64, 0.8, 0.08 + 0.9 * y as f64 / (height - 1).max(1) as f64);
        data.extend_from_slice(&[r, g, b, 255]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// 8. Six 60-row bands over a 240 x 360 hue sweep, one per mode the first render could not separate
/// (probe results "Blend modes"): Soft Light, Hard Light, Linear Light, Pin Light, Vivid Light,
/// Hard Mix. Each band holds six 40-px grey columns: 25%, 50%, 75% opaque, then the same at half alpha.
fn blend_greys_doc() -> Document {
    const MODES: [BlendMode; 6] = [BlendMode::SoftLight, BlendMode::HardLight, BlendMode::LinearLight, BlendMode::PinLight, BlendMode::VividLight, BlendMode::HardMix];
    const GREYS: [(u32, u32); 6] = [(64, 255), (128, 255), (191, 255), (64, 128), (128, 128), (191, 128)];
    let mut doc = Document::new(240, 360);
    let mut layers = vec![Layer::with_pixels("Hue sweep", colourful_gradient(240, 360), Point { x: 0.0, y: 0.0 })];
    for (band, mode) in MODES.iter().enumerate() {
        for (column, (grey, alpha)) in GREYS.iter().enumerate() {
            let v = ((grey * alpha + 127) / 255) as u8;
            let mut l = solid_rect(&format!("{mode:?} {column}"), [v, v, v, *alpha as u8], 40, 60, column as f64 * 40.0, band as f64 * 60.0);
            l.blend_mode = *mode;
            layers.push(l);
        }
    }
    doc.layers = layers;
    doc
}

/// 9-10. Color Balance with every tone moved, Preserve Luminosity on or off.
fn color_balance_doc(preserve: bool) -> Document {
    let mut doc = Document::new(240, 120);
    let mut adjustment = Layer::blank("Color Balance", doc.size());
    let mut a = LayerAdjustment::new(AdjustmentKind::ColorBalance);
    a.color_balance_settings = Some(ColorBalanceSettings { shadow_cyan_red: 40.0, shadow_magenta_green: -20.0, shadow_yellow_blue: 30.0,
        mid_cyan_red: -35.0, mid_magenta_green: 25.0, mid_yellow_blue: -15.0, highlight_cyan_red: 20.0, highlight_magenta_green: 45.0,
        highlight_yellow_blue: -50.0, preserve_luminosity: preserve });
    adjustment.extra.adjustment = Some(a);
    doc.layers = vec![Layer::with_pixels("Tonal sweep", tonal_sweep(240, 120), Point { x: 0.0, y: 0.0 }), adjustment];
    doc
}

/// What the blur probes blur: a hue-sweep block running 12 px off the canvas's left edge (the part
/// off the canvas must not spread back in, R 3.4) and a half-alpha grey bar, over nothing.
fn blur_probe(adjustment: LayerAdjustment) -> Document {
    let mut doc = Document::new(160, 100);
    let block = Layer::with_pixels("Block", colourful_gradient(92, 60), Point { x: -12.0, y: 20.0 });
    let bar = solid_rect("Bar", [128, 128, 128, 128], 30, 80, 110.0, 10.0);
    let mut layer = Layer::blank(adjustment.kind.name(), doc.size());
    layer.extra.adjustment = Some(adjustment);
    doc.layers = vec![block, bar, layer];
    doc
}
/// 11-12. Gaussian Blur at radius 6 (the exact kernel) and 40 (a halved copy, spatial_level 2).
fn gaussian_doc(radius: f64) -> Document {
    let mut a = LayerAdjustment::new(AdjustmentKind::GaussianBlur); a.blur_radius = Some(radius); blur_probe(a)
}
/// 13. Motion Blur, 30 degrees, 24 px: settles CIMotionBlur's taper against this port's even streak.
fn motion_doc() -> Document {
    let mut a = LayerAdjustment::new(AdjustmentKind::MotionBlur); a.motion_angle = Some(30.0); a.motion_distance = Some(24.0); blur_probe(a)
}
/// 14-15. Add Noise over the tonal sweep: uniform colour noise, and Gaussian monochromatic noise.
fn noise_doc(amount: f64, gaussian: bool, monochromatic: bool, seed: u32) -> Document {
    let mut doc = Document::new(120, 60);
    let mut a = LayerAdjustment::new(AdjustmentKind::AddNoise);
    a.noise_amount = Some(amount); a.noise_gaussian = Some(gaussian); a.noise_monochromatic = Some(monochromatic); a.noise_seed = Some(seed);
    let mut layer = Layer::blank("Add Noise", doc.size());
    layer.extra.adjustment = Some(a);
    doc.layers = vec![Layer::with_pixels("Tonal sweep", tonal_sweep(120, 60), Point { x: 0.0, y: 0.0 }), layer];
    doc
}

/// One adjustment layer over the tonal sweep, in `mode` at `opacity`.
fn over_sweep(adjustment: LayerAdjustment, mode: BlendMode, opacity: f64) -> Document {
    let mut doc = Document::new(120, 60);
    let mut layer = Layer::blank(adjustment.kind.name(), doc.size());
    layer.extra.adjustment = Some(adjustment);
    layer.blend_mode = mode;
    layer.opacity = opacity;
    doc.layers = vec![Layer::with_pixels("Tonal sweep", tonal_sweep(120, 60), Point { x: 0.0, y: 0.0 }), layer];
    doc
}
/// Levels sending every channel to mid grey: in any mode but Normal the blend shows.
fn levels_to_mid_grey() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0].output_black = 128.0;
    a.levels.ranges[0].output_white = 128.0;
    a
}
/// 16. A Gaussian Blur layer (radius 6) in Linear Burn, a Core-Image-only mode: the Mac takes the
/// full-coverage path that keeps the original alpha (LiveMaskRenderer.swift:24) and draws it in
/// Normal (cgMode, :40), so the canvas edge should not fade (ruling E-I1).
fn cgmode_blur_doc() -> Document {
    let mut doc = gaussian_doc(6.0);
    doc.layers[2].blend_mode = BlendMode::LinearBurn;
    doc
}
/// 19. Two clipping stacks over the hue sweep, each a translucent child on an opaque base: the base
/// in Subtract (Core Image only, so the group composites as Normal) and in Color Burn (Core
/// Graphics' own formula, which the Mac calls wrong, LayerAppearance.swift:51-52).
fn cgmode_stack_bases_doc() -> Document {
    let mut doc = Document::new(240, 120);
    let mut layers = vec![Layer::with_pixels("Hue sweep", colourful_gradient(240, 120), Point { x: 0.0, y: 0.0 })];
    for (i, mode) in [BlendMode::Subtract, BlendMode::ColorBurn].into_iter().enumerate() {
        let mut base = solid_rect(&format!("{mode:?} base"), [60, 150, 110, 255], 100, 100, 10.0 + 120.0 * i as f64, 10.0);
        base.blend_mode = mode;
        let mut child = solid_rect(&format!("{mode:?} child"), [40, 20, 90, 128], 60, 100, 30.0 + 120.0 * i as f64, 10.0);
        child.mask_source_id = Some(base.id);
        layers.push(base);
        layers.push(child);
    }
    doc.layers = layers;
    doc
}
/// 22. The radius-6 blur at 60% under a horizontal ramp mask: a soft mask and partial opacity
/// together (toward, R 3.4 steps 4-5).
fn blur_soft_mask_doc() -> Document {
    let mut doc = gaussian_doc(6.0);
    let ramp: Vec<u8> = (0..100).flat_map(|_| (0..160u32).map(|x| (x * 255 / 159) as u8)).collect();
    doc.layers[2].mask = Some(Mask { pixels: GrayRaster::from_bytes(160, 100, ramp), enabled: true, placement: None, linked: None });
    doc.layers[2].opacity = 0.6;
    doc
}

/// Oblong and soft-edged with a hole off-centre, coloured by column and row (the kernel tests'
/// `blob`): edges running every way, nothing symmetric, alpha taking many values.
fn soft_blob(width: u32, height: u32) -> Raster {
    let (w, h) = (width as f64, height as f64);
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let (cx, cy) = (x as f64 - w * 0.4, y as f64 - h * 0.55);
        let r = (cx * cx / (w * 0.3).powi(2) + cy * cy / (h * 0.35).powi(2)).sqrt();
        let hole = ((x as f64 - w * 0.6).powi(2) + (y as f64 - h * 0.4).powi(2)).sqrt() < h * 0.08;
        let a = if hole { 0 } else { (((1.1 - r) * 4.0).clamp(0.0, 1.0) * 255.0).round() as u32 };
        let (r8, g8, b8) = (x * 255 / width, y * 255 / height, 120u32);
        data.extend_from_slice(&[(r8 * a / 255) as u8, (g8 * a / 255) as u8, (b8 * a / 255) as u8, a as u8]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// The 60 x 36 blob at (50, 32) on 160 x 100 with `effects`, over `backdrop` when given.
fn styled_doc(effects: serde_json::Value, backdrop: bool) -> Document {
    let mut doc = Document::new(160, 100);
    let mut layer = Layer::with_pixels("Styled", soft_blob(60, 36), Point { x: 50.0, y: 32.0 });
    layer.extra.effects = Some(serde_json::from_value(effects).unwrap());
    doc.layers = if backdrop { vec![Layer::with_pixels("Tonal sweep", tonal_sweep(160, 100), Point { x: 0.0, y: 0.0 }), layer] } else { vec![layer] };
    doc
}

/// Every effect not at its defaults, both angles off the axes.
fn all_six() -> serde_json::Value {
    serde_json::json!({
        "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 5 },
        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 },
        "colorOverlay": { "blue": 0.3, "green": 0.7, "opacity": 0.35, "red": 0.1 },
        "innerShadow": { "angle": -35, "blue": 0, "blur": 4, "distance": 6.5, "green": 0.05, "opacity": 0.8, "red": 0.1 },
        "outerGlow": { "blue": 0.1, "green": 0.6, "opacity": 0.9, "red": 1, "size": 13 },
        "innerGlow": { "blue": 1, "green": 0.9, "opacity": 0.85, "red": 0.2, "size": 9 } })
}

/// 23-38: one probe per effect and per rule of how the Mac draws effects (Phase 3.5c).
fn effects_probes() -> Vec<(&'static str, Document)> {
    let effect = |key: &str| serde_json::json!({ key: all_six()[key].clone() });
    let mut probes = vec![
        ("effects-stroke-outside.comp", styled_doc(serde_json::json!({ "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 7.4 } }), false)),
        ("effects-stroke-inside.comp", styled_doc(serde_json::json!({ "stroke": { "blue": 0.9, "green": 0.3, "inside": true, "opacity": 0.6, "red": 0.1, "size": 5 } }), true)),
        ("effects-drop-shadow.comp", styled_doc(effect("shadow"), false)),
        ("effects-inner-shadow.comp", styled_doc(effect("innerShadow"), false)),
        ("effects-outer-glow.comp", styled_doc(effect("outerGlow"), false)),
        ("effects-inner-glow.comp", styled_doc(effect("innerGlow"), false)),
        ("effects-color-overlay.comp", styled_doc(effect("colorOverlay"), false)),
    ];
    // Compose order, and opacity and blend mode on the layer and its effects as one.
    let mut all = styled_doc(all_six(), true);
    all.layers[1].opacity = 0.7;
    all.layers[1].blend_mode = BlendMode::Multiply;
    // One hidden (`enabled: false`) beside the five shown: the Mac draws the others alone.
    all.layers[1].extra.effects.as_mut().unwrap().inner_glow.as_mut().unwrap().enabled = Some(false);
    probes.push(("effects-all-six.comp", all));
    // The shadow turns, flips and scales with its layer.
    let shadow = serde_json::json!({ "shadow": { "angle": 30, "blue": 0, "blur": 6, "distance": 12, "green": 0, "opacity": 0.8, "red": 0 } });
    let mut transformed = Document::new(200, 120);
    let mut flipped = Layer::with_pixels("Flipped", soft_blob(60, 36), Point { x: 20.0, y: 20.0 });
    flipped.transform.flip_y = true;
    let mut turned = Layer::with_pixels("Turned", soft_blob(60, 36), Point { x: 110.0, y: 40.0 });
    turned.transform.rotation = 25.0;
    turned.transform = turned.transform.scaled_to_percent(150.0, Size { width: 60.0, height: 36.0 });
    for l in [&mut flipped, &mut turned] { l.extra.effects = Some(serde_json::from_value(shadow.clone()).unwrap()); }
    transformed.layers = vec![flipped, turned];
    probes.push(("effects-transformed.comp", transformed));
    // The mask first, then the effects around what it leaves.
    let mut masked = styled_doc(serde_json::json!({ "stroke": { "blue": 0, "green": 0.8, "inside": false, "opacity": 1, "red": 0.2, "size": 4 },
        "shadow": { "angle": 60, "blue": 0, "blur": 5, "distance": 8, "green": 0, "opacity": 0.6, "red": 0 } }), false);
    let ramp: Vec<u8> = (0..36u32).flat_map(|y| (0..60u32).map(move |x| ((x + y) * 255 / 94) as u8)).collect();
    masked.layers[0].mask = Some(Mask { pixels: GrayRaster::from_bytes(60, 36, ramp), enabled: true, placement: None, linked: None });
    probes.push(("effects-masked.comp", masked));
    // A mask off the layer's grid: unlinked, moved and of another size, so the port resamples it
    // into the layer's pixels by its own rules (ruling 14; ruling OQ3).
    let mut placed = styled_doc(serde_json::json!({ "stroke": { "blue": 0, "green": 0.8, "inside": false, "opacity": 1, "red": 0.2, "size": 4 },
        "shadow": { "angle": 60, "blue": 0, "blur": 5, "distance": 8, "green": 0, "opacity": 0.6, "red": 0 } }), false);
    let ramp: Vec<u8> = (0..25u32).flat_map(|y| (0..40u32).map(move |x| ((x * 3 + y * 5) * 255 / 242) as u8)).collect();
    placed.layers[0].mask = Some(Mask { pixels: GrayRaster::from_bytes(40, 25, ramp), enabled: true,
        placement: Some(LayerTransform::axis_aligned(Point { x: 62.0, y: 40.0 }, Size { width: 48.0, height: 30.0 })), linked: Some(false) });
    probes.push(("effects-mask-placed.comp", placed));
    // A clipping base's effects are its clipped layer's coverage.
    let mut clipping = styled_doc(serde_json::json!({ "stroke": { "blue": 1, "green": 1, "inside": false, "opacity": 1, "red": 1, "size": 8 },
        "shadow": { "angle": 120, "blue": 0, "blur": 9, "distance": 14, "green": 0, "opacity": 0.7, "red": 0 } }), false);
    let mut child = Layer::with_pixels("Clipped", tonal_sweep(160, 100), Point { x: 0.0, y: 0.0 });
    child.mask_source_id = Some(clipping.layers[0].id);
    clipping.layers.push(child);
    probes.push(("effects-clipping-base.comp", clipping));
    // A clipped layer's own effects end at its base's alpha (R 4.2, read from code only).
    let mut clipped = styled_doc(serde_json::json!({}), false);
    clipped.layers[0].extra.effects = None;
    let mut styled_child = Layer::with_pixels("Styled child", soft_blob(30, 20), Point { x: 60.0, y: 40.0 });
    styled_child.extra.effects = Some(serde_json::from_value(serde_json::json!({ "stroke": { "blue": 1, "green": 0.2, "inside": false, "opacity": 1, "red": 0.9, "size": 6 },
        "outerGlow": { "blue": 0.1, "green": 0.6, "opacity": 0.9, "red": 1, "size": 13 } })).unwrap());
    styled_child.mask_source_id = Some(clipped.layers[0].id);
    clipped.layers.push(styled_child);
    probes.push(("effects-clipped-child.comp", clipped));
    // Blurs past the reach limit, which this port halves.
    probes.push(("effects-large-blur.comp", styled_doc(serde_json::json!({
        "shadow": { "angle": 120, "blue": 0.4, "blur": 90, "distance": 10, "green": 0, "opacity": 0.9, "red": 0.1 },
        "outerGlow": { "blue": 0.2, "green": 1, "opacity": 0.8, "red": 0.6, "size": 60 } }), false)));
    // Inside a 50% folder whose mask hides a band.
    let mut folded = styled_doc(effect("shadow"), false);
    let mut f = folder(&folded, 0.5);
    let band: Vec<u8> = (0..100u32).flat_map(|y| (0..160u32).map(move |_| if (40..55).contains(&y) { 0 } else { 255 })).collect();
    f.mask = Some(Mask { pixels: GrayRaster::from_bytes(160, 100, band), enabled: true, placement: None, linked: None });
    folded.layers[0].parent_id = Some(f.id);
    folded.layers.insert(0, f);
    probes.push(("effects-folder.comp", folded));
    // An invalid shown effect: the Mac draws the layer without any of them.
    probes.push(("effects-invalid.comp", styled_doc(serde_json::json!({ "stroke": { "blue": 0, "green": 0, "inside": false, "opacity": 1, "red": 1, "size": 600 },
        "shadow": { "angle": 90, "blue": 0, "blur": 9, "distance": 14, "green": 0, "opacity": 0.7, "red": 0 } }), false)));
    probes
}

const README_TXT: &str = "\
This folder holds test projects for Compositor on the Mac.

For each project listed below:

1. Open it in Compositor for Mac (1.2.10 or later).
2. Confirm it opens without an error.
3. File > Export > PNG, at 100%, into a folder named mac-exports, using the file name given below.
4. Send the mac-exports folder back.

Projects:

- folder-opacity.comp           -> folder-opacity.png
- clipped-in-dimmed-folder.comp -> clipped-in-dimmed-folder.png
- guides.comp                   -> guides.png
- new-blend-modes.comp          -> new-blend-modes.png
- new-adjustment-layers.comp    -> new-adjustment-layers.png
- grain.comp                    -> grain.png
- edited-rich-file.comp         -> edited-rich-file.png (confirm it opens, and export a PNG)

New in this set (Phase 3.5b follow-up):

- blend-greys.comp               -> blend-greys.png
- color-balance-preserve.comp    -> color-balance-preserve.png
- color-balance-no-preserve.comp -> color-balance-no-preserve.png
- gaussian-blur-6.comp           -> gaussian-blur-6.png
- gaussian-blur-40.comp          -> gaussian-blur-40.png
- motion-blur-30-24.comp         -> motion-blur-30-24.png
- add-noise-uniform.comp         -> add-noise-uniform.png
- add-noise-gaussian-mono.comp   -> add-noise-gaussian-mono.png
- cgmode-blur-linear-burn.comp   -> cgmode-blur-linear-burn.png
- cgmode-levels-divide.comp      -> cgmode-levels-divide.png
- color-dodge-adjustment.comp    -> color-dodge-adjustment.png
- cgmode-stack-bases.comp        -> cgmode-stack-bases.png
- black-white-tint.comp          -> black-white-tint.png
- invert.comp                    -> invert.png
- blur-soft-mask.comp            -> blur-soft-mask.png

New in this set (Phase 3.5c, layer effects):

- effects-stroke-outside.comp -> effects-stroke-outside.png
- effects-stroke-inside.comp  -> effects-stroke-inside.png
- effects-drop-shadow.comp    -> effects-drop-shadow.png
- effects-inner-shadow.comp   -> effects-inner-shadow.png
- effects-outer-glow.comp     -> effects-outer-glow.png
- effects-inner-glow.comp     -> effects-inner-glow.png
- effects-color-overlay.comp  -> effects-color-overlay.png
- effects-all-six.comp        -> effects-all-six.png
- effects-transformed.comp    -> effects-transformed.png
- effects-masked.comp         -> effects-masked.png
- effects-mask-placed.comp    -> effects-mask-placed.png
- effects-clipping-base.comp  -> effects-clipping-base.png
- effects-clipped-child.comp  -> effects-clipped-child.png
- effects-large-blur.comp     -> effects-large-blur.png
- effects-folder.comp         -> effects-folder.png
- effects-invalid.comp        -> effects-invalid.png

One more, made on the Mac: create a new 200 x 120 document, import any small image, give that
layer all six effects from the Effects panel (Stroke, Drop Shadow, Color Overlay, Inner Shadow,
Outer Glow, Inner Glow) with settings of your choice, hide one of them with its eye, then save it
as mac-effects.comp and export it as mac-effects.png. Send both back with the rest.

New in this set (how an enlarged, turned or shrunk layer is resampled):

- sampling-high-400.comp       -> sampling-high-400.png
- sampling-smooth-400.comp     -> sampling-smooth-400.png
- sampling-nearest-400.comp    -> sampling-nearest-400.png
- sampling-high-150.comp       -> sampling-high-150.png
- sampling-high-rotated.comp   -> sampling-high-rotated.png
- sampling-high-shrink-65.comp -> sampling-high-shrink-65.png
- sampling-high-mask-400.comp  -> sampling-high-mask-400.png

New in this set (the blend curve at many sub-pixel positions):

- sampling-steps-high-1600.comp   -> sampling-steps-high-1600.png
- sampling-steps-smooth-1600.comp -> sampling-steps-smooth-1600.png
- sampling-steps-high-700.comp    -> sampling-steps-high-700.png
";

/// 7. RULING (F5, replacing the M9 tautological final-existence loop): the Mac acceptance probe.
/// The other six probes are rendering oracles for documents this build only opened (or built)
/// and re-saved unmodified; none of them sends the Mac a file this build has actually EDITED
/// while carrying effects, live text, an unknown-version adjustment kind and unknown keys at
/// both levels -- the case this phase's "the Mac accepts what we write" constraint is really
/// about. Starts from the same kind of rich v9 document as `preservation_through_commands.rs`,
/// restricted to features the Mac's own validation/decode accepts (no free-form `shape`, whose
/// exact Mac schema this build does not know), then edits it through `Engine::execute` before
/// saving, exactly as a user would.
fn edited_rich_file_doc() -> Document {
    let folder_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE1";
    let text_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE2";
    let adj_id = "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE3";
    let image_file = format!("{text_id}.png");
    let manifest = serde_json::json!({
        "format": "com.compositor.project", "version": 9, "colorSpace": "sRGB",
        "documentID": "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE0", "width": 120, "height": 60,
        "activeLayerID": text_id,
        "guides": [ { "axis": "vertical", "id": "0B6C6B1E-4F1B-4B4E-9E0A-EEEEEEEEEEE4", "position": 30 } ],
        "futureDocumentKey": { "nested": [1, 2.5, "x"] },
        "layers": [
            { "id": folder_id, "name": "Folder", "isVisible": true, "isGroup": true, "opacity": 0.5,
              "transform": { "origin": [0, 0], "size": [120, 60], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "futureFolderKey": true },
            { "id": text_id, "name": "Titled", "isVisible": true, "imageFile": image_file,
              "transform": { "origin": [10, 10], "size": [60, 40], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "effects": { "shadow": { "angle": 90, "blue": 0, "blur": 20, "distance": 20, "green": 0, "opacity": 0.5, "red": 0 } },
              "text": { "alignment": "Left", "blue": 0, "content": "Hi", "fontName": "Helvetica", "fontSize": 72,
                        "green": 0, "leading": 0, "red": 0, "tracking": 0 },
              "futureLayerKey": 7 },
            { "id": adj_id, "name": "Blur", "isVisible": true,
              "transform": { "origin": [0, 0], "size": [120, 60], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" },
              "adjustment": { "kind": "Gaussian Blur", "hue": 0, "saturation": 0, "lightness": 0, "colorize": false,
                  "levels": { "channel": "RGB", "ranges": [ {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}, {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},
                      {"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255} ] },
                  "curves": { "channel": "RGB", "channels": [ [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}],
                      [{"x":0,"y":0},{"x":255,"y":255}], [{"x":0,"y":0},{"x":255,"y":255}] ] },
                  "blurRadius": 24 },
              "futureAdjLayerKey": "z" }
        ]
    }).to_string();
    let image = encode_png(&colourful_gradient(60, 40), 72.0).unwrap();
    let package = Package { manifest_json: manifest, images: vec![(image_file, image)] };

    let mut e = Engine::new();
    let id = e.open_package(&package, None).unwrap_or_else(|err| panic!("edited-rich-file.comp: does not open: {err:?}"));
    let text_uuid = uuid::Uuid::parse_str(text_id).unwrap();
    let adj_uuid = uuid::Uuid::parse_str(adj_id).unwrap();
    e.execute(id, Command::DuplicateLayer { id: text_uuid }).unwrap();
    let dup_id = e.state(id).unwrap().active_layer_id.unwrap();
    e.execute(id, Command::GroupLayers { ids: vec![dup_id, adj_uuid] }).unwrap();
    e.execute(id, Command::CanvasSize { width: 160, height: 80, anchor: 4, fill: None }).unwrap();
    e.execute(id, Command::FlipCanvas { horizontal: true }).unwrap();
    e.document(id).unwrap().clone()
}

/// Hard edges every way for fitting a resampling filter: 1, 2 and 3 px colour stripes on the left,
/// an opaque green bar top right, a half-alpha magenta block and a transparent notch bottom right.
fn sampling_pattern() -> Raster {
    const STRIPES: [(u32, [u8; 3]); 5] = [(1, [230, 40, 30]), (1, [20, 20, 20]), (2, [240, 240, 240]), (3, [30, 90, 220]), (1, [250, 200, 40])];
    let (width, height) = (16u32, 10u32);
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let px = if x < 8 {
            let mut left = x;
            let mut colour = STRIPES[0].1;
            for (w, c) in STRIPES { if left < w { colour = c; break; } left -= w; }
            [colour[0], colour[1], colour[2], 255]
        } else if y < 4 { [40, 180, 90, 255] }
        else if x >= 12 && y >= 6 { [0, 0, 0, 0] }
        else { [(200 * 128 + 127) / 255, (60 * 128 + 127) / 255, (160 * 128 + 127) / 255, 128].map(|v: u32| v as u8) };
        data.extend_from_slice(&px);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// Single-pixel detail for a reduction: colours that change every pixel, no two neighbours alike.
fn fine_pattern(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let k = (x * 37 + y * 91) % 7;
        data.extend_from_slice(&[(k * 36) as u8, (255 - k * 30) as u8, ((k * 97) % 256) as u8, 255]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// The sampling pattern drawn `percent` of its size, turned `rotation` degrees, with `sampling`.
/// The Mac enlarges with Core Graphics' filter for the layer's setting (High quality: `.high`,
/// Smooth: `.low`) and shrinks with `.low` (LayerRenderer.swift:42-44); the port samples
/// bilinearly for both. `effects-transformed` found the difference; these settle the filter.
fn sampling_doc(sampling: Sampling, percent: f64, rotation: f64, origin: Point) -> Document {
    let mut doc = Document::new(96, 72);
    let mut layer = Layer::with_pixels("Pattern", sampling_pattern(), origin);
    layer.transform.size = Size { width: 16.0 * percent / 100.0, height: 10.0 * percent / 100.0 };
    layer.transform.rotation = rotation;
    layer.transform.sampling = sampling;
    doc.layers = vec![layer];
    doc
}

fn sampling_probes() -> Vec<(&'static str, Document)> {
    let at = Point { x: 8.0, y: 8.0 };
    let mut probes = vec![
        ("sampling-high-400.comp", sampling_doc(Sampling::High, 400.0, 0.0, at)),
        ("sampling-smooth-400.comp", sampling_doc(Sampling::Smooth, 400.0, 0.0, at)),
        ("sampling-nearest-400.comp", sampling_doc(Sampling::Nearest, 400.0, 0.0, at)),
        ("sampling-high-150.comp", sampling_doc(Sampling::High, 150.0, 0.0, at)),
        ("sampling-high-rotated.comp", sampling_doc(Sampling::High, 300.0, 25.0, Point { x: 24.0, y: 20.0 })),
    ];
    let mut shrink = Document::new(96, 72);
    let mut small = Layer::with_pixels("Fine", fine_pattern(60, 40), at);
    small.transform.size = Size { width: 39.0, height: 26.0 };
    shrink.layers = vec![small];
    probes.push(("sampling-high-shrink-65.comp", shrink));
    // A mask resamples with the layer's own setting even when shrinking (LayerRenderer.drawCoverage).
    let mut masked = sampling_doc(Sampling::High, 400.0, 0.0, at);
    let solid = Raster::from_premultiplied(16, 10, [60u8, 120, 200, 255].repeat(160));
    masked.layers[0].set_pixels(Some(solid));
    let grey: Vec<u8> = sampling_pattern().bytes().chunks(4).map(|p| if p[3] == 0 { 0 } else if p[3] < 255 { 128 } else { p[0].max(p[1]) }).collect();
    masked.layers[0].mask = Some(Mask { pixels: GrayRaster::from_bytes(16, 10, grey), enabled: true, placement: None, linked: None });
    probes.push(("sampling-high-mask-400.comp", masked));
    probes
}

/// Four 1-px columns, identical rows: every column boundary is a vertical step, so a large
/// enlargement shows the blend curve at `percent / 100` phases per source pixel. The 400 % and
/// 150 % renders showed a two-tap curve that is not bilinear and looks quantised in phase.
fn steps_doc(sampling: Sampling, percent: f64) -> Document {
    const COLUMNS: [[u8; 4]; 4] = [[230, 40, 30, 255], [20, 20, 20, 255], [240, 240, 240, 255], [30, 90, 220, 255]];
    let raster = Raster::from_premultiplied(4, 3, (0..3).flat_map(|_| COLUMNS.concat()).collect());
    let mut doc = Document::new(96, 72);
    let mut layer = Layer::with_pixels("Steps", raster, Point { x: 8.0, y: 8.0 });
    layer.transform.size = Size { width: 4.0 * percent / 100.0, height: 3.0 * percent / 100.0 };
    layer.transform.sampling = sampling;
    doc.layers = vec![layer];
    doc
}

fn step_probes() -> Vec<(&'static str, Document)> {
    vec![
        ("sampling-steps-high-1600.comp", steps_doc(Sampling::High, 1600.0)),
        ("sampling-steps-smooth-1600.comp", steps_doc(Sampling::Smooth, 1600.0)),
        ("sampling-steps-high-700.comp", steps_doc(Sampling::High, 700.0)),
    ]
}

/// Saves `doc` as `<dir>/<filename>/manifest.json` plus its `images/`, then re-opens the saved
/// package with `open_package` -- every probe must be openable by this build's own reader before
/// it is ever sent to a Mac.
fn write_probe(dir: &Path, filename: &str, doc: &Document) {
    let package = save_package(doc).unwrap_or_else(|e| panic!("{filename}: failed to save: {e:?}"));
    let comp_dir = dir.join(filename);
    if comp_dir.exists() { fs::remove_dir_all(&comp_dir).unwrap_or_else(|e| panic!("{filename}: failed to clear {comp_dir:?}: {e}")); }
    let images_dir = comp_dir.join("images");
    fs::create_dir_all(&images_dir).unwrap_or_else(|e| panic!("{filename}: failed to create {images_dir:?}: {e}"));
    fs::write(comp_dir.join("manifest.json"), &package.manifest_json).unwrap_or_else(|e| panic!("{filename}: failed to write manifest.json: {e}"));
    for (name, bytes) in &package.images {
        fs::write(images_dir.join(name), bytes).unwrap_or_else(|e| panic!("{filename}: failed to write image {name}: {e}"));
    }
    open_package(&package).unwrap_or_else(|e| panic!("{filename}: does not re-open with open_package: {e:?}"));
}

#[test]
#[ignore]
fn write_mac_probes() {
    let dir = probes_dir();
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("failed to create {dir:?}: {e}"));

    write_probe(&dir, "folder-opacity.comp", &folder_opacity_doc());
    write_probe(&dir, "clipped-in-dimmed-folder.comp", &clipped_in_dimmed_folder_doc());
    write_probe(&dir, "guides.comp", &guides_doc());
    write_probe(&dir, "new-blend-modes.comp", &new_blend_modes_doc());
    write_probe(&dir, "new-adjustment-layers.comp", &new_adjustment_layers_doc());
    write_probe(&dir, "grain.comp", &grain_doc());
    write_probe(&dir, "edited-rich-file.comp", &edited_rich_file_doc());

    write_probe(&dir, "blend-greys.comp", &blend_greys_doc());
    write_probe(&dir, "color-balance-preserve.comp", &color_balance_doc(true));
    write_probe(&dir, "color-balance-no-preserve.comp", &color_balance_doc(false));
    write_probe(&dir, "gaussian-blur-6.comp", &gaussian_doc(6.0));
    write_probe(&dir, "gaussian-blur-40.comp", &gaussian_doc(40.0));
    write_probe(&dir, "motion-blur-30-24.comp", &motion_doc());
    write_probe(&dir, "add-noise-uniform.comp", &noise_doc(25.0, false, false, 12_345));
    write_probe(&dir, "add-noise-gaussian-mono.comp", &noise_doc(40.0, true, true, 777));
    // 16-22: the cgMode path (ruling E-I1), Core Graphics' Color Dodge and Color Burn (audit E-M2),
    // and what else 3.5b draws without a Mac render yet (audit G-M5).
    write_probe(&dir, "cgmode-blur-linear-burn.comp", &cgmode_blur_doc());
    write_probe(&dir, "cgmode-levels-divide.comp", &over_sweep(levels_to_mid_grey(), BlendMode::Divide, 0.6));
    write_probe(&dir, "color-dodge-adjustment.comp", &over_sweep(levels_to_mid_grey(), BlendMode::ColorDodge, 1.0));
    write_probe(&dir, "cgmode-stack-bases.comp", &cgmode_stack_bases_doc());
    let mut tinted = LayerAdjustment::new(AdjustmentKind::BlackWhite);
    tinted.black_white_settings = Some(BlackWhiteSettings { reds: 115.0, yellows: -40.0, greens: 70.0, cyans: 180.0, blues: -90.0, magentas: 20.0,
        tint: true, tint_hue: 205.0, tint_saturation: 45.0 });
    write_probe(&dir, "black-white-tint.comp", &over_sweep(tinted, BlendMode::Normal, 1.0));
    write_probe(&dir, "invert.comp", &over_sweep(LayerAdjustment::new(AdjustmentKind::Invert), BlendMode::Normal, 1.0));
    write_probe(&dir, "blur-soft-mask.comp", &blur_soft_mask_doc());
    for (name, doc) in effects_probes() { write_probe(&dir, name, &doc); }
    for (name, doc) in sampling_probes() { write_probe(&dir, name, &doc); }
    for (name, doc) in step_probes() { write_probe(&dir, name, &doc); }

    fs::write(dir.join("README.txt"), README_TXT).unwrap_or_else(|e| panic!("failed to write README.txt: {e}"));
    assert!(README_TXT.is_ascii(), "README.txt must be ASCII only");
}

#[test]
fn writing_a_probe_replaces_what_an_earlier_run_left() {
    // An earlier run wrote images under other UUIDs; the Mac opened the folder with them in it.
    let dir = std::env::temp_dir().join(format!("compositor-probe-test-{}", std::process::id()));
    let stale = dir.join("guides.comp").join("images").join("STALE.png");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, b"left over").unwrap();
    write_probe(&dir, "guides.comp", &guides_doc());
    assert!(!stale.exists(), "an image from an earlier run is gone");
    assert!(dir.join("guides.comp").join("manifest.json").exists());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn every_effects_probe_is_listed_and_draws_what_it_is_named_for() {
    let probes = effects_probes();
    assert_eq!(probes.len(), 16);
    for (name, doc) in &probes {
        assert!(README_TXT.contains(&format!("- {name}")), "{name} is in the README");
        let drawn = doc.layers.iter().filter(|l| effects_draw(l, None).is_some()).count();
        let want = match *name { "effects-invalid.comp" => 0, "effects-transformed.comp" => 2, _ => 1 };
        assert_eq!(drawn, want, "{name}: layers drawn with their effects");
    }
}

#[test]
fn every_sampling_probe_is_listed_and_resamples_as_named() {
    let probes = sampling_probes();
    assert_eq!(probes.len(), 7);
    for (name, doc) in &probes {
        assert!(README_TXT.contains(&format!("- {name}")), "{name} is in the README");
        let t = doc.layers[0].transform;
        let want = if name.contains("nearest") { Sampling::Nearest } else if name.contains("smooth") { Sampling::Smooth } else { Sampling::High };
        assert_eq!(t.sampling, want, "{name}: sampling");
        let pixels = doc.layers[0].pixels.as_ref().unwrap();
        let factor = t.size.width / pixels.width as f64;
        let named = if name.contains("400") { 4.0 } else if name.contains("150") { 1.5 } else if name.contains("rotated") { 3.0 } else { 0.65 };
        assert!((factor - named).abs() < 1e-9, "{name}: drawn at {factor}x");
        assert_eq!(t.rotation != 0.0, name.contains("rotated"), "{name}: rotation");
    }
}

#[test]
fn every_step_probe_is_listed_and_enlarged_as_named() {
    for (name, doc) in step_probes() {
        assert!(README_TXT.contains(&format!("- {name}")), "{name} is in the README");
        let t = doc.layers[0].transform;
        let factor = t.size.width / 4.0;
        let named = if name.contains("1600") { 16.0 } else { 7.0 };
        assert_eq!(factor, named, "{name}: drawn at {factor}x");
        assert_eq!(t.sampling == Sampling::Smooth, name.contains("smooth"), "{name}: sampling");
        assert!(t.origin.x + t.size.width <= doc.width as f64 && t.origin.y + t.size.height <= doc.height as f64, "{name}: on the canvas");
    }
}

#[test]
fn the_readme_names_the_mac_version_the_probes_are_for() {
    assert!(README_TXT.contains("Compositor for Mac (1.2.10 or later)") && !README_TXT.contains("1.2.6"));
}
