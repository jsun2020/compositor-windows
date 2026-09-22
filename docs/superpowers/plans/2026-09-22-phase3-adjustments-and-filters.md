# Compositor for Windows, Phase 3 (Adjustments and Filters) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every colour adjustment and filter of the macOS app on the Phase 2 base: Levels (with Auto and eyedroppers), Curves, Hue/Saturation (per-range, colorize, eyedroppers), Exposure, Gradient Map, Grain, Invert, Gaussian Blur and Motion Blur that spread past the layer's edges, Add Noise, Lens Correction; live previews that never touch the document; and adjustment layers (Hue/Saturation, Levels, Curves, Exposure, Gradient Map, Grain) that round-trip the `.comp` format with the Mac and render identically on the CPU compositor and the WebGL2 renderer.

**Architecture:** One Rust module tree `engine/src/adjust/` holds the pixel maths as ports of the Mac's C kernels and Swift settings structs, in two forms: whole-raster kernels for destructive edits (integer-exact ports) and a `PreparedAdjustment` that maps one straight colour at one document point (float), used by both compositors for adjustment layers. The `RenderPlan` gains adjustment draws (`LayerDraw.adjustment`), a `PreviewEdit::Adjustment` variant for editing an adjustment layer live, and the engine session gains a pixel preview (a substituted raster for the layer being filtered) that every render entry point reads through `render_document`. Commands stay whole-document undo steps. The app gets one floating, non-modal adjustment panel per kind and a WebGL2 `adjust` pass.

**Tech Stack:** unchanged (Rust 1.95, wasm-pack 0.15.0, Vite 6, React 18, zustand, vitest, Playwright, Tauri 2).

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md` (section 3 "Phase 3, adjustments and filters", sections 4 to 8).

## Global Constraints

- Everything in the Phase 1 and Phase 2 Global Constraints still applies (limits, ASCII-only source and strings, `.comp` v1 to v7, premultiplied RGBA8 in the engine, straight PNG on disk, 127.0.0.1 dev URLs, hierarchy render order, selection outside undo history, `store.run` commits a pending transform first).
- The macOS source at `C:\Users\sr9rfx\.claude-project\Compositor` is the oracle. Kernels are ported line for line: `Rendering/AdjustPixels.c` (gradient map, grain), `Rendering/LevelsPixels.c` (`levels_apply`, histogram), `Rendering/NoisePixels.c`, `Rendering/LensPixels.c`; settings from `Document/Levels.swift`, `Document/LevelsAutomatic.swift`, `Document/Curves.swift`, `Document/HueSaturation.swift`, `Document/ImageAdjustments.swift`, `Document/LayerAdjustment.swift`; blur growth and trim from `Document/Filters.swift`; adjustment-layer compositing from `Rendering/LiveMaskRenderer.swift`.
- Adjustment JSON (`LayerRecord.adjustment`) is the Swift `Codable` shape of `LayerAdjustment`, byte-compatible both ways: keys `kind`, `hue`, `saturation`, `lightness`, `colorize`, `levels`, `curves` always written; `hsvSettings`, `exposureSettings`, `gradientMapSettings`, `grainSettings` written only when present. `kind` is one of `"Hue/Saturation"`, `"Levels"`, `"Curves"`, `"Exposure"`, `"Gradient Map"`, `"Grain"`. Channels are `"RGB"`, `"Red"`, `"Green"`, `"Blue"`; colour ranges are `"Master"`, `"Reds"`, `"Yellows"`, `"Greens"`, `"Cyans"`, `"Blues"`, `"Magentas"`. An adjustment layer needs manifest version 7, is not a group, and has no `imageFile`; its settings must validate (Mac `LayerAdjustment.isValid`).
- Levels: `LevelRange.apply(v) = (outputBlack + clamp((v*255 - black)/(white - black), 0, 1)^(1/gamma) * (outputWhite - outputBlack)) / 255`; a channel's table is `ranges[0].apply(ranges[channel].apply(v))`. The pixel kernel is `levels_apply` alone: it unpremultiplies each channel once (`x = min(255, p*255/alpha)`), interpolates the 256-entry float table and re-premultiplies with `min(alpha, max(0, round(result*alpha)))`. Ruling: the Mac's `LevelsFilter.run` wraps that kernel in an extra integer unpremultiply/premultiply pair, which double-unpremultiplies edge pixels; the Mac's own `LevelsTests.inputClippingGammaOutputInversionAndAlpha` expects the single-pass result (`[64,96,128,128]` for the inverted `[64,32,0,128]` pixel), so the port applies the kernel once and the extra pair is not ported.
- Curves: shape-preserving cubic Hermite (Fritsch-Carlson harmonic slopes, zero slope at a sign change, endpoint slopes = end secants), 2 to 32 points per channel, first x = 0, last x = 255, x strictly increasing; the table is `value(value(v, channel), RGB)/255` applied with `levels_apply`.
- Hue/Saturation: seven ranges (Master + six Photoshop bands with linear falloff shoulders, wrapping at 360, span at most 350); a per-degree response table (361 entries) sums every range's hue shift, saturation and lightness weighted by that range's band weight (inverted for the selected range when `invertRange`); per pixel, RGB to HSL, hue += shift (wrapped), saturation *= (1 + sat/100) clamped, lightness pulled toward white (amount > 0) or black (amount < 0), back to RGB; colorize replaces hue and saturation outright. Ruling: the Mac bakes this into a 33-point colour cube; the port evaluates the exact function per pixel (its tests allow 8 levels for cube interpolation; ours allow 2), and the WebGL2 shader evaluates the same function.
- Exposure: per-channel table `encode(pow(max(0, decode(v) * 2^exposure + offset), 1/gamma))` with sRGB decode/encode, applied with `levels_apply`. Gradient Map: 256-entry linear RGB table between the two end colours (reversed swaps them), looked up by Rec. 709 integer luma of the unpremultiplied colour. Grain: `adjust_grain` port, fixed in document space (`origin`, `unitsPerPixel`), triangular lattice noise with smoothstep interpolation and a per-pixel fine noise mixed by roughness, strength `amount/100 * 0.35 * 255`, midtone weight `0.4 + 2.4 L (1 - L)`. Invert: premultiplied `colour = alpha - colour`.
- Filters: Gaussian Blur sigma = radius (layer pixels), separable, transparent beyond the raster (edges fade and spread); Motion Blur is an even streak of length `distance` along `angle` degrees counter-clockwise from horizontal on screen (direction `(cos a, -sin a)` in top-down pixels), bilinear samples one pixel apart, transparent outside. Ruling: the Mac approximates this with Core Image's tapered `CIMotionBlur` at radius `distance/sqrt(12)`; the even streak is what Photoshop does and what that radius was chosen to match. Add Noise: `noise_add` port (uniform or Box-Muller Gaussian x 2/3, per-channel key offset `0x9e3779b9`, monochromatic shares one key), seed fixed while the panel is open. Lens Correction: `lens_distort` port with `k = distortion/100 * 0.35`.
- Blur growth: before a blur the layer's raster is padded on every side by `margin = ceil(radius*3 + 2)` (Gaussian) or `ceil(distance/2 + 2)` (Motion), the transform enlarged in place (size scaled by the grid ratio, centre kept), capped at 30,000 per side and 100 million pixels; after the blur the result is trimmed to `alpha_bounds` and the transform recomputed the same way; a covering (placement `None`), non-uniform mask is carried onto the final grid with its background beyond the old edge. Ruling: the Mac's `FilterTests.gaussianBlur...` asserts `alpha(0) == 255` on the committed layer, which predates edge spreading; the Windows test asserts the trimmed layer extends past the old left edge with a soft edge on both sides.
- Selection limiting: every whole-raster kernel takes `selection: Option<&GrayRaster>` (coverage on the layer's pixel grid) and blends `coverage * adjusted + (1 - coverage) * original`; Phase 3 always passes `None` (selections arrive in Phase 4), and the panels show no selection note.
- Previews never mutate the document. A destructive preview (`Engine::set_preview`) computes the kernel on a preview source: the raster halved until its longest side is at most 4096 for Levels, Curves, Hue/Saturation, Exposure, Gradient Map and Invert, at most 2048 for Gaussian Blur, Motion Blur and Lens Correction (radius and distance scale with the halving), and at full size for Add Noise and Grain (as on the Mac, where grain made small looks coarse). The preview raster replaces the layer's pixels and transform in a session-level `render_document` that `render_plan`, `composite_edit`, `layer_pixels_*` and `histogram` all read; `execute` clears it and then executes. Ruling (Task 10): `undo` and `redo` clear it and are otherwise INERT while a preview is up - the brief's own test pins this (an undo during a preview must leave the canvas unchanged, and the only history step left there is the fixture's import), and it matches the app rule that undo and redo are inert while a panel is open. `revert` clears it and then reverts, because a no-op revert would silently swallow an explicit request to discard changes. An adjustment layer previews through `PreviewEdit::Adjustment { id, adjustment }`, which substitutes the layer's settings in `render_plan`.
- Adjustment layers render as the Mac's `LiveMaskRenderer.adjust`: the adjustment maps the composite beneath it in render order, everywhere its coverage (own mask, folder masks, times opacity) reaches. Ruling (Task 9): a folder does NOT isolate its contents from an adjustment inside it - `LiveMaskRenderer.adjust` takes `context.makeImage()` for the whole context, and the only limits are `adjustmentClip` (the adjustment's own mask) and the enclosing folders' masks that `FolderMaskClip.draw` wraps around every `drawComposite` call. An adjustment inside an unmasked folder reaches layers outside it; a folder mask is what scopes one. Earlier wording here said "within its folder" and was wrong; with a blend mode other than Normal the adjusted colour is `blend(original, adjusted)` on opaque colours with the original alpha restored; an adjustment layer clipped to a base renders only as a member of that base's clipping stack (it maps the stack's buffer) and is otherwise not drawn; an adjustment layer is never a stack base and never a clipping source. Undo names: "Levels", "Curves", "Hue/Saturation", "Exposure", "Gradient Map", "Grain", "Invert", "Gaussian Blur", "Motion Blur", "Add Noise", "Lens Correction", "New <kind> Adjustment", "Edit <kind> Adjustment".
- Panels are non-modal floating panels (`data-testid="adjust-panel"`), not sheets, because the eyedroppers click the canvas. While a panel is open, `store.run` refuses other history-recording commands with the banner "Apply or cancel the open adjustment first", undo and redo are inert, and the Layer/Image/Filter menu items grey out (the Mac's `canEditLayers`/`canUseHistory`). Enter applies, Escape cancels, an unchanged (identity) commit records nothing.
- CPU/GPU parity: the WebGL2 `adjust` pass and the CPU compositor apply identical float maths; e2e compares them within 2/255 (3/255 for Grain, whose f32 hash path may round differently on the GPU) using fixtures whose hues stay clear of half-degree boundaries.
- Every commit compiles and passes `cargo test`, `pnpm test`, `pnpm build` and `pnpm e2e`. After any engine or engine-wasm change, `pnpm wasm:dev` runs before `pnpm build`/`pnpm e2e`.

---

## File Structure

```
engine/src/adjust/mod.rs              pub mod settings, levels, curves, hsv, tonal, grain, filters, prepared, apply
engine/src/adjust/settings.rs         LayerAdjustment, AdjustmentKind, LevelsSettings, LevelRange, LevelsChannel, CurvesSettings, CurvePoint,
                                      HueSaturationSettings, HueBand, ColorRange, RangeAdjustment, ExposureSettings, GradientMapSettings,
                                      GrainSettings, AdjustmentColor (serde = Mac JSON), validation
engine/src/adjust/levels.rs           levels_tables, levels_apply (raster), histogram, LevelsAuto, sampling
engine/src/adjust/curves.rs           value(), curves_tables
engine/src/adjust/hsv.rs              hue_response, adjust_rgb, to_hsl/to_rgb, apply_hsv (raster)
engine/src/adjust/tonal.rs            exposure_table, gradient_map_table, apply_gradient_map, invert (raster)
engine/src/adjust/grain.rs            mix32, lattice, grain_delta (per point), apply_grain (raster)
engine/src/adjust/filters.rs          FilterParams, gaussian_blur, motion_blur, add_noise, lens_distort, blur_margin, grown/trimmed helpers
engine/src/adjust/prepared.rs         PreparedAdjustment::prepare(&LayerAdjustment), color(rgb, doc point) for both compositors
engine/src/adjust/apply.rs            apply_adjustment(raster, &LayerAdjustment, origin, units, selection), blend_by_coverage
engine/src/ops/adjust.rs              apply_adjustment_to_layer, invert_layer, apply_filter (grow/trim/mask carry), add_adjustment_layer, set_adjustment
engine/src/document.rs                LayerExtra.adjustment: Option<LayerAdjustment>; Layer::is_adjustment
engine/src/manifest.rs                LayerRecord.adjustment typed; validate() rule for adjustment layers
engine/src/plan.rs                    LayerDraw.adjustment, PreviewEdit::Adjustment, stack/clip rules for adjustment layers
engine/src/compositor.rs              adjust_target for Layer and Stack nodes
engine/src/preview.rs                 PixelPreview, PreviewRequest, preview source scaling, histogram target, sample_color
engine/src/engine.rs                  Session.preview, render_document, set_preview, histogram, sample_color, auto_levels, new commands, LayerState.adjustment
engine/src/command.rs                 ApplyAdjustment, InvertPixels, ApplyFilter, AddAdjustmentLayer, SetAdjustment
engine/tests/{adjust_settings,levels,curves_tonal,hsv,filters,adjust_ops,adjust_plan,preview}.rs
engine-wasm/src/lib.rs                set_preview, histogram, auto_levels, levels_sampling, sample_color
app/src/engine/types.ts               LayerAdjustment types, FilterParams, new commands, PreviewEdit adjustment, LayerDraw.adjustment, LayerState.adjustment
app/src/engine/client.ts              setPreview, histogram, autoLevels, levelsSampling, sampleColor
app/src/canvas/gl/programs.ts         adjust program (LUT, gradient map, HSL, grain, invert; blend modes; coverage)
app/src/canvas/gl/adjust-textures.ts  LUT / gradient / hue-response textures cached by adjustment JSON
app/src/canvas/gl-renderer.ts         adjustment draws in main and stacks; texture sync from the plan's draws
app/src/canvas/layer-textures.ts      sync keyed by draw id/revision (preview rasters)
app/src/state/store.ts                adjustEdit state, beginAdjust/updateAdjust/setAdjustPreview/commitAdjust/cancelAdjust, sample modes, run gating
app/src/state/adjust-edit.ts          AdjustEdit types, identity checks, default params per kind
app/src/tools/levels-tools.ts         histogram display scale, LevelsAuto port, sampling port (TS mirrors for the panel)
app/src/tools/curves-editor.ts        point hit/insert/move/remove rules
app/src/tools/hue-band.ts             HueBand port (weight, centered, include, exclude, setHandle), rgbToHue
app/src/panels/AdjustPanel.tsx        floating panel shell (title, Preview toggle, Reset, Cancel, OK, Enter/Escape)
app/src/panels/LevelsPanel.tsx        histogram canvas, channel, fields, Auto, eyedroppers
app/src/panels/CurvesPanel.tsx        curve editor canvas, channel, point fields
app/src/panels/HueSaturationPanel.tsx range, sliders, colorize, invert, band handles, eyedroppers
app/src/panels/FilterPanel.tsx        Exposure, Gradient Map, Grain, Gaussian Blur, Motion Blur, Add Noise, Lens Correction
app/src/panels/MenuBar.tsx            Image (adjustments, Invert), Filter menu, Layer (New Adjustment Layer..., Edit Adjustment...)
app/src/panels/LayersList.tsx         adjustment row glyph, double-click edits an adjustment layer
app/src/shortcuts/keymap.ts           levels Ctrl+L, curves Ctrl+M, hue-saturation Ctrl+U, invert Ctrl+I
app/src/canvas/CanvasView.tsx         eyedropper clicks while a panel is armed
app/tests/unit/{levels-tools,curves-editor,hue-band,adjust-store}.test.ts
app/tests/e2e/{adjust-layers,adjust-panels,filters}.spec.ts
```

---

### Task 1: Adjustment settings types, Mac-compatible JSON, manifest validation

**Files:**
- Create: `engine/src/adjust/mod.rs`, `engine/src/adjust/settings.rs`
- Modify: `engine/src/lib.rs` (add `pub mod adjust; pub use adjust::settings::*;`), `engine/src/document.rs` (`LayerExtra.adjustment: Option<LayerAdjustment>`, `Layer::is_adjustment`), `engine/src/manifest.rs` (`LayerRecord.adjustment: Option<LayerAdjustment>`, validation)
- Test: `engine/tests/adjust_settings.rs`

**Interfaces:**
- Produces: `AdjustmentKind` (serde strings above, `name() -> &'static str` = the same string), `LevelsChannel { Rgb, Red, Green, Blue }` with `index()`, `LevelRange { black, gamma, white, output_black, output_white }` with `normalized()`, `apply(f64) -> f64`, `LevelsSettings { channel, ranges: Vec<LevelRange> }` with `is_identity()`, `is_valid()`, `apply(value, channel)`, `CurvePoint { x, y }`, `CurvesSettings { channel, channels: Vec<Vec<CurvePoint>> }` with `is_valid()`, `is_identity()`, `value(x, channel_index)`, `ColorRange` (7 variants, `Ord` so maps serialize in a fixed order), `HueBand` with `forward`, `weight`, `centered_on`, `include`, `exclude`, `set_handle`, `RangeAdjustment`, `HueSaturationSettings { range, colorize, invert_range, adjustments: BTreeMap<ColorRange, RangeAdjustment>, bands: BTreeMap<ColorRange, HueBand> }` with `new(hue, sat, light, colorize, range)`, `adjustment(range)`, `band(range)`, `weight(range, hue)`, `is_identity()`, `is_valid()`, `AdjustmentColor { red, green, blue }`, `ExposureSettings { exposure, offset, gamma }` with `normalized()`, `is_valid()`, `GradientMapSettings { shadows, highlights, reversed }` with `ends()`, `GrainSettings { amount, size, roughness, seed: u32 }`, `LayerAdjustment { kind, hue, saturation, lightness, colorize, hsv_settings, levels, curves, exposure_settings, gradient_map_settings, grain_settings }` with `new(kind)`, `resolved_hsv()`, `exposure()`, `gradient_map()`, `grain()`, `is_valid()`, `is_identity()`.
- Consumes: `Layer`, `LayerRecord`, `Manifest::validate`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/adjust_settings.rs`:
```rust
use compositor_engine::*;

/// The exact JSON Swift's JSONEncoder writes for `LayerAdjustment(kind: .levels)` (keys sorted).
const MAC_LEVELS: &str = r#"{"colorize":false,"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]},"hue":0,"kind":"Levels","levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255}]},"lightness":0,"saturation":0}"#;

#[test]
fn levels_adjustment_round_trips_the_mac_json_byte_for_byte() {
    let a: LayerAdjustment = serde_json::from_str(MAC_LEVELS).unwrap();
    assert_eq!(a.kind, AdjustmentKind::Levels);
    assert!(a.is_valid() && a.is_identity());
    let value = serde_json::to_value(&a).unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), MAC_LEVELS, "sorted keys, no optional settings written");
    assert!(a.hsv_settings.is_none() && a.exposure_settings.is_none() && a.gradient_map_settings.is_none() && a.grain_settings.is_none());
}

#[test]
fn legacy_hsv_fields_resolve_and_range_settings_serialize_with_string_keys() {
    let legacy: LayerAdjustment = serde_json::from_str(r#"{"kind":"Hue/Saturation","hue":120,"saturation":0,"lightness":0,"colorize":false,"levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255},{"black":0,"gamma":1,"white":255,"outputBlack":0,"outputWhite":255}]},"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]}}"#).unwrap();
    assert!(legacy.hsv_settings.is_none());
    assert_eq!(legacy.resolved_hsv().adjustment(ColorRange::Master).hue, 120.0);
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    let mut hsv = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Reds);
    hsv.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: 0.0, saturation: -100.0, lightness: 0.0 });
    a.hsv_settings = Some(hsv.clone());
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.contains(r#""adjustments":{"Blues":{"hue":0.0,"lightness":0.0,"saturation":-100.0},"Reds":"#) || json.contains(r#""adjustments":{"Blues":{"hue":0,"lightness":0,"saturation":-100},"Reds":"#), "{json}");
    assert!(json.contains(r#""bands":{"Blues":{"falloffEnd":285"#) || json.contains(r#""bands":{"Blues":{"falloffEnd":285.0"#), "{json}");
    let back: LayerAdjustment = serde_json::from_str(&json).unwrap();
    assert_eq!(back.resolved_hsv(), hsv);
}

#[test]
fn optional_settings_appear_only_when_set_and_validate() {
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 40.0, size: 3.0, roughness: 10.0, seed: 9 });
    let json = serde_json::to_string(&grain).unwrap();
    assert!(json.contains(r#""grainSettings":{"amount":40"#) && !json.contains("exposureSettings"));
    let back: LayerAdjustment = serde_json::from_str(&json).unwrap();
    assert_eq!(back, grain);
    assert!(back.is_valid());
    let mut broken = LayerAdjustment::new(AdjustmentKind::Exposure);
    broken.exposure_settings = Some(ExposureSettings { exposure: 0.0, offset: 0.0, gamma: 0.0 });
    assert!(!broken.is_valid());
    let mut bad_levels = LayerAdjustment::new(AdjustmentKind::Levels);
    bad_levels.levels.ranges[0].black = 300.0;
    assert!(!bad_levels.is_valid(), "levels must be stored normalized");
}

#[test]
fn level_range_normalizes_like_the_mac() {
    let r = LevelRange { black: 300.0, gamma: f64::NAN, white: -1.0, output_black: -100.0, output_white: 400.0 }.normalized();
    assert!(r.black < r.white && r.gamma == 1.0 && r.output_black == 0.0 && r.output_white == 255.0);
    assert_eq!(LevelRange { black: 64.0, gamma: 1.0, white: 128.0, ..LevelRange::default() }.apply(96.0 / 255.0), 0.5);
    let mut s = LevelsSettings::default();
    s.ranges[LevelsChannel::Red.index()] = LevelRange { gamma: 2.0, ..LevelRange::default() };
    s.ranges[0] = LevelRange { black: 40.0, white: 210.0, ..LevelRange::default() };
    let expected = LevelRange { black: 40.0, white: 210.0, ..LevelRange::default() }.apply(LevelRange { gamma: 2.0, ..LevelRange::default() }.apply(64.0 / 255.0));
    assert!((s.apply(64.0 / 255.0, LevelsChannel::Red) - expected).abs() < 1e-12, "channel first, then RGB");
}

#[test]
fn hue_bands_ramp_through_falloff_and_wrap() {
    let reds = ColorRange::Reds.default_band();
    assert!(reds.weight(0.0) == 1.0 && reds.weight(345.0) == 1.0 && reds.weight(15.0) == 1.0);
    assert!((reds.weight(330.0) - 0.5).abs() < 0.001 && (reds.weight(30.0) - 0.5).abs() < 0.001);
    assert!(reds.weight(315.0) == 0.0 && reds.weight(45.0) == 0.0 && reds.weight(180.0) == 0.0);
    assert_eq!(ColorRange::Master.default_band().weight(123.0), 1.0);
    let mut band = ColorRange::Greens.default_band();
    band.set_handle(1, 200.0);
    assert_eq!(band, ColorRange::Greens.default_band(), "crossing moves are refused");
    band.set_handle(1, 110.0);
    assert_eq!(band.range_start, 110.0);
    let centered = ColorRange::Greens.default_band().centered_on(0.0);
    assert!(centered.weight(0.0) == 1.0 && centered.weight(120.0) == 0.0);
    let mut widened = centered; widened.include(240.0);
    assert!(widened.weight(240.0) == 1.0 && widened.weight(0.0) == 1.0);
    widened.exclude(240.0);
    assert_eq!(widened.weight(240.0), 0.0);
}

#[test]
fn curves_validate_and_interpolate_monotonically() {
    let c = CurvesSettings::default();
    assert!(c.is_valid() && c.is_identity());
    assert_eq!(c.value(100.0, 0), 100.0);
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(s.is_valid());
    let mut last = 0.0;
    for x in 0..=255 { let v = s.value(x as f64, 0); assert!(v >= last - 1e-9 && v <= 255.0, "monotone at {x}: {v} < {last}"); last = v; }
    assert!((s.value(128.0, 0) - 190.0).abs() < 1e-9);
    s.channels[1] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 0.0, y: 5.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(!s.is_valid(), "x must be strictly increasing");
}

#[test]
fn manifests_accept_adjustment_layers_only_when_well_formed() {
    let mut doc = Document::new(4, 4);
    let mut layer = Layer::blank("Levels", doc.size());
    layer.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::Levels));
    doc.layers.push(layer);
    let json = doc.manifest().to_json_pretty().unwrap();
    assert!(json.contains("\"adjustment\": {") && json.contains("\"kind\": \"Levels\""));
    let parsed = Manifest::parse(&json).unwrap();
    assert_eq!(parsed.layers[0].adjustment.as_ref().unwrap().kind, AdjustmentKind::Levels);
    let group = json.replacen("\"isVisible\": true", "\"isVisible\": true, \"isGroup\": true", 1);
    assert!(matches!(Manifest::parse(&group), Err(ProjectError::Invalid)), "a folder cannot carry an adjustment");
    let old = json.replacen("\"version\": 7", "\"version\": 6", 1);
    assert!(matches!(Manifest::parse(&old), Err(ProjectError::Invalid)), "adjustments need version 7");
    let with_image = json.replacen("\"adjustment\": {", "\"imageFile\": \"X.png\", \"adjustment\": {", 1);
    assert!(matches!(Manifest::parse(&with_image), Err(ProjectError::Invalid)));
    let pkg = save_package(&doc).unwrap();
    let back = open_package(&pkg).unwrap();
    assert!(back.layers[0].is_adjustment() && back.layers[0].pixels.is_none());
    assert_eq!(back.layers[0].extra.adjustment, doc.layers[0].extra.adjustment);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test adjust_settings`
Expected: compile errors (`LayerAdjustment` and friends do not exist).

- [ ] **Step 3: Implement the settings module**

`engine/src/adjust/mod.rs`:
```rust
pub mod settings;
```
(later tasks add `levels`, `curves`, `hsv`, `tonal`, `grain`, `filters`, `prepared`, `apply`).

`engine/src/adjust/settings.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn clamp_or(n: f64, lo: f64, hi: f64, fallback: f64) -> f64 { if n.is_finite() { n.clamp(lo, hi) } else { fallback } }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AdjustmentKind {
    #[default] #[serde(rename = "Hue/Saturation")] Hsv,
    #[serde(rename = "Levels")] Levels,
    #[serde(rename = "Curves")] Curves,
    #[serde(rename = "Exposure")] Exposure,
    #[serde(rename = "Gradient Map")] GradientMap,
    #[serde(rename = "Grain")] Grain,
}
impl AdjustmentKind {
    pub const ALL: [AdjustmentKind; 6] = [AdjustmentKind::Hsv, AdjustmentKind::Levels, AdjustmentKind::Curves, AdjustmentKind::Exposure, AdjustmentKind::GradientMap, AdjustmentKind::Grain];
    pub fn name(self) -> &'static str {
        match self { AdjustmentKind::Hsv => "Hue/Saturation", AdjustmentKind::Levels => "Levels", AdjustmentKind::Curves => "Curves", AdjustmentKind::Exposure => "Exposure", AdjustmentKind::GradientMap => "Gradient Map", AdjustmentKind::Grain => "Grain" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LevelsChannel { #[default] #[serde(rename = "RGB")] Rgb, #[serde(rename = "Red")] Red, #[serde(rename = "Green")] Green, #[serde(rename = "Blue")] Blue }
impl LevelsChannel {
    pub const ALL: [LevelsChannel; 4] = [LevelsChannel::Rgb, LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue];
    pub fn index(self) -> usize { self as usize }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelRange {
    pub black: f64,
    pub gamma: f64,
    pub white: f64,
    #[serde(rename = "outputBlack")] pub output_black: f64,
    #[serde(rename = "outputWhite")] pub output_white: f64,
}
impl Default for LevelRange { fn default() -> Self { LevelRange { black: 0.0, gamma: 1.0, white: 255.0, output_black: 0.0, output_white: 255.0 } } }
impl LevelRange {
    pub fn normalized(&self) -> LevelRange {
        let black = clamp_or(self.black, 0.0, 254.0, 0.0);
        LevelRange { black, white: clamp_or(self.white, black + 1.0, 255.0, 255.0), gamma: clamp_or(self.gamma, 0.1, 9.99, 1.0),
            output_black: clamp_or(self.output_black, 0.0, 255.0, 0.0), output_white: clamp_or(self.output_white, 0.0, 255.0, 255.0) }
    }
    pub fn apply(&self, value: f64) -> f64 {
        let s = self.normalized();
        let input = ((value * 255.0 - s.black) / (s.white - s.black)).clamp(0.0, 1.0);
        (s.output_black + input.powf(1.0 / s.gamma) * (s.output_white - s.output_black)) / 255.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelsSettings { pub channel: LevelsChannel, pub ranges: Vec<LevelRange> }
impl Default for LevelsSettings { fn default() -> Self { LevelsSettings { channel: LevelsChannel::Rgb, ranges: vec![LevelRange::default(); 4] } } }
impl LevelsSettings {
    pub fn is_valid(&self) -> bool { self.ranges.len() == 4 && self.ranges.iter().all(|r| *r == r.normalized()) }
    pub fn is_identity(&self) -> bool { self.ranges.len() == 4 && self.ranges.iter().all(|r| r.normalized() == LevelRange::default()) }
    /// The channel's own range, then the composite RGB range.
    pub fn apply(&self, value: f64, channel: LevelsChannel) -> f64 { self.ranges[0].apply(self.ranges[channel.index()].apply(value)) }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvePoint { pub x: f64, pub y: f64 }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvesSettings { pub channel: LevelsChannel, pub channels: Vec<Vec<CurvePoint>> }
impl Default for CurvesSettings {
    fn default() -> Self { CurvesSettings { channel: LevelsChannel::Rgb, channels: vec![vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 255.0, y: 255.0 }]; 4] } }
}
impl CurvesSettings {
    pub fn is_valid(&self) -> bool {
        self.channels.len() == 4 && self.channels.iter().all(|p| {
            (2..=32).contains(&p.len()) && p[0].x == 0.0 && p[p.len() - 1].x == 255.0
                && p.iter().all(|q| q.x.is_finite() && q.y.is_finite() && (0.0..=255.0).contains(&q.x) && (0.0..=255.0).contains(&q.y))
                && p.windows(2).all(|w| w[0].x < w[1].x)
        })
    }
    pub fn is_identity(&self) -> bool { *self == CurvesSettings { channel: self.channel, ..CurvesSettings::default() } }
    /// Shape-preserving cubic Hermite interpolation (Fritsch-Carlson), so the curve never overshoots its handles.
    pub fn value(&self, x: f64, channel: usize) -> f64 {
        let p = &self.channels[channel];
        let i = p.iter().rposition(|q| q.x <= x).unwrap_or(0).min(p.len() - 2);
        let d: Vec<f64> = p.windows(2).map(|w| (w[1].y - w[0].y) / (w[1].x - w[0].x)).collect();
        let slope = |j: usize| -> f64 {
            if j == 0 { d[0] } else if j == p.len() - 1 { d[d.len() - 1] }
            else if d[j - 1] * d[j] <= 0.0 { 0.0 } else { 2.0 / (1.0 / d[j - 1] + 1.0 / d[j]) }
        };
        let h = p[i + 1].x - p[i].x;
        let t = ((x - p[i].x) / h).clamp(0.0, 1.0);
        let y = (2.0 * t * t * t - 3.0 * t * t + 1.0) * p[i].y + (t * t * t - 2.0 * t * t + t) * h * slope(i)
            + (-2.0 * t * t * t + 3.0 * t * t) * p[i + 1].y + (t * t * t - t * t) * h * slope(i + 1);
        y.clamp(0.0, 255.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub enum ColorRange { #[default] Master, Reds, Yellows, Greens, Cyans, Blues, Magentas }
impl ColorRange {
    pub const ALL: [ColorRange; 7] = [ColorRange::Master, ColorRange::Reds, ColorRange::Yellows, ColorRange::Greens, ColorRange::Cyans, ColorRange::Blues, ColorRange::Magentas];
    pub const COLORS: [ColorRange; 6] = [ColorRange::Reds, ColorRange::Yellows, ColorRange::Greens, ColorRange::Cyans, ColorRange::Blues, ColorRange::Magentas];
    /// Photoshop's starting hue band: falloff start, range start, range end, falloff end.
    pub fn default_band(self) -> HueBand {
        let b = |a: f64, b: f64, c: f64, d: f64| HueBand { falloff_start: a, range_start: b, range_end: c, falloff_end: d };
        match self {
            ColorRange::Master => b(0.0, 0.0, 360.0, 360.0), ColorRange::Reds => b(315.0, 345.0, 15.0, 45.0), ColorRange::Yellows => b(15.0, 45.0, 75.0, 105.0),
            ColorRange::Greens => b(75.0, 105.0, 135.0, 165.0), ColorRange::Cyans => b(135.0, 165.0, 195.0, 225.0), ColorRange::Blues => b(195.0, 225.0, 255.0, 285.0),
            ColorRange::Magentas => b(255.0, 285.0, 315.0, 345.0),
        }
    }
}

fn wrap360(v: f64) -> f64 { let r = v % 360.0; if r < 0.0 { r + 360.0 } else { r } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HueBand {
    #[serde(rename = "falloffStart")] pub falloff_start: f64,
    #[serde(rename = "rangeStart")] pub range_start: f64,
    #[serde(rename = "rangeEnd")] pub range_end: f64,
    #[serde(rename = "falloffEnd")] pub falloff_end: f64,
}
impl HueBand {
    /// Degrees from `from` forward to `to`, always 0..360.
    pub fn forward(from: f64, to: f64) -> f64 { wrap360(to - from) }
    /// 1 inside the range, ramping linearly through each shoulder, 0 outside.
    pub fn weight(&self, hue: f64) -> f64 {
        let span = Self::forward(self.falloff_start, self.falloff_end);
        if span <= 0.0 { return 1.0; }
        let position = Self::forward(self.falloff_start, hue);
        if position > span { return 0.0; }
        let ramp_in = Self::forward(self.falloff_start, self.range_start);
        let plateau_end = Self::forward(self.falloff_start, self.range_end);
        if position < ramp_in { return if ramp_in > 0.0 { position / ramp_in } else { 1.0 }; }
        if position <= plateau_end { return 1.0; }
        let ramp_out = span - plateau_end;
        if ramp_out > 0.0 { (span - position) / ramp_out } else { 1.0 }
    }
    pub fn handles(&self) -> [f64; 4] { [self.falloff_start, self.range_start, self.range_end, self.falloff_end] }
    pub fn is_finite(&self) -> bool { self.handles().iter().all(|h| h.is_finite()) }
    pub fn centered_on(&self, hue: f64) -> HueBand {
        let core = Self::forward(self.range_start, self.range_end);
        let leading = Self::forward(self.falloff_start, self.range_start);
        let trailing = Self::forward(self.range_end, self.falloff_end);
        let start = wrap360(hue - core / 2.0);
        HueBand { falloff_start: wrap360(start - leading), range_start: start, range_end: wrap360(start + core), falloff_end: wrap360(start + core + trailing) }
    }
    pub fn include(&mut self, hue: f64) {
        if self.weight(hue) >= 1.0 { return; }
        let shoulder_in = Self::forward(self.falloff_start, self.range_start);
        let shoulder_out = Self::forward(self.range_end, self.falloff_end);
        if Self::forward(hue, self.range_start) <= Self::forward(self.range_end, hue) { self.range_start = hue; self.falloff_start = hue - shoulder_in; }
        else { self.range_end = hue; self.falloff_end = hue + shoulder_out; }
        self.normalize();
    }
    pub fn exclude(&mut self, hue: f64) {
        if self.weight(hue) <= 0.0 { return; }
        let shoulder_in = Self::forward(self.falloff_start, self.range_start);
        let shoulder_out = Self::forward(self.range_end, self.falloff_end);
        if Self::forward(self.falloff_start, hue) <= Self::forward(hue, self.falloff_end) { self.falloff_start = hue + 1.0; self.range_start = hue + 1.0 + shoulder_in; }
        else { self.falloff_end = hue - 1.0; self.range_end = hue - 1.0 - shoulder_out; }
        self.normalize();
    }
    fn normalize(&mut self) {
        self.falloff_start = wrap360(self.falloff_start); self.range_start = wrap360(self.range_start);
        self.range_end = wrap360(self.range_end); self.falloff_end = wrap360(self.falloff_end);
        if Self::forward(self.falloff_start, self.falloff_end) > 350.0 { self.falloff_end = wrap360(self.falloff_start + 350.0); }
    }
    /// Moves one handle, keeping the four in order and the band under a full circle; refused otherwise.
    pub fn set_handle(&mut self, index: usize, degrees: f64) {
        let mut updated = *self;
        let value = wrap360(degrees);
        match index { 0 => updated.falloff_start = value, 1 => updated.range_start = value, 2 => updated.range_end = value, _ => updated.falloff_end = value }
        let span = Self::forward(updated.falloff_start, updated.falloff_end);
        let to_start = Self::forward(updated.falloff_start, updated.range_start);
        let to_end = Self::forward(updated.falloff_start, updated.range_end);
        if span > 1.0 && span <= 350.0 && to_start <= to_end && to_end <= span { *self = updated; }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct RangeAdjustment { #[serde(default)] pub hue: f64, #[serde(default)] pub saturation: f64, #[serde(default)] pub lightness: f64 }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HueSaturationSettings {
    pub range: ColorRange,
    pub colorize: bool,
    #[serde(rename = "invertRange")] pub invert_range: bool,
    pub adjustments: BTreeMap<ColorRange, RangeAdjustment>,
    pub bands: BTreeMap<ColorRange, HueBand>,
}
impl Default for HueSaturationSettings { fn default() -> Self { HueSaturationSettings::new(0.0, 0.0, 0.0, false, ColorRange::Master) } }
impl HueSaturationSettings {
    pub fn new(hue: f64, saturation: f64, lightness: f64, colorize: bool, range: ColorRange) -> Self {
        let mut adjustments = BTreeMap::new();
        adjustments.insert(range, RangeAdjustment { hue, saturation, lightness });
        HueSaturationSettings { range, colorize, invert_range: false, adjustments, bands: ColorRange::ALL.iter().map(|r| (*r, r.default_band())).collect() }
    }
    /// Photoshop's starting point when Colorize is switched on.
    pub fn colorize_start() -> Self { HueSaturationSettings::new(0.0, 25.0, 0.0, true, ColorRange::Master) }
    pub fn adjustment(&self, range: ColorRange) -> RangeAdjustment { self.adjustments.get(&range).copied().unwrap_or_default() }
    pub fn band(&self, range: ColorRange) -> HueBand { self.bands.get(&range).copied().unwrap_or_else(|| range.default_band()) }
    pub fn is_identity(&self) -> bool { !self.colorize && self.adjustments.values().all(|a| *a == RangeAdjustment::default()) }
    /// How much a range applies to one hue: Master everywhere, others through their band.
    pub fn weight(&self, range: ColorRange, hue: f64) -> f64 {
        if range == ColorRange::Master { return 1.0; }
        let w = self.band(range).weight(hue);
        if self.invert_range && range == self.range { 1.0 - w } else { w }
    }
    pub fn is_valid(&self) -> bool {
        self.adjustments.values().all(|a| a.hue.is_finite() && a.hue.abs() <= 360.0 && a.saturation.is_finite() && a.saturation.abs() <= 100.0 && a.lightness.is_finite() && a.lightness.abs() <= 100.0)
            && self.bands.values().all(HueBand::is_finite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AdjustmentColor { pub red: f64, pub green: f64, pub blue: f64 }
impl AdjustmentColor {
    pub fn is_valid(&self) -> bool { [self.red, self.green, self.blue].iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) }
    pub fn clamped(&self) -> Self { AdjustmentColor { red: clamp_or(self.red, 0.0, 1.0, 0.0), green: clamp_or(self.green, 0.0, 1.0, 0.0), blue: clamp_or(self.blue, 0.0, 1.0, 0.0) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExposureSettings { pub exposure: f64, pub offset: f64, pub gamma: f64 }
impl Default for ExposureSettings { fn default() -> Self { ExposureSettings { exposure: 0.0, offset: 0.0, gamma: 1.0 } } }
impl ExposureSettings {
    pub fn is_valid(&self) -> bool { (-20.0..=20.0).contains(&self.exposure) && (-0.5..=0.5).contains(&self.offset) && (0.01..=9.99).contains(&self.gamma) }
    pub fn normalized(&self) -> Self { ExposureSettings { exposure: clamp_or(self.exposure, -20.0, 20.0, 0.0), offset: clamp_or(self.offset, -0.5, 0.5, 0.0), gamma: clamp_or(self.gamma, 0.01, 9.99, 1.0) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GradientMapSettings { pub shadows: AdjustmentColor, pub highlights: AdjustmentColor, pub reversed: bool }
impl Default for GradientMapSettings {
    fn default() -> Self { GradientMapSettings { shadows: AdjustmentColor { red: 0.0, green: 0.0, blue: 0.0 }, highlights: AdjustmentColor { red: 1.0, green: 1.0, blue: 1.0 }, reversed: false } }
}
impl GradientMapSettings {
    pub fn is_valid(&self) -> bool { self.shadows.is_valid() && self.highlights.is_valid() }
    pub fn normalized(&self) -> Self { GradientMapSettings { shadows: self.shadows.clamped(), highlights: self.highlights.clamped(), reversed: self.reversed } }
    /// The colours for the darkest and lightest tones, in the order they apply.
    pub fn ends(&self) -> (AdjustmentColor, AdjustmentColor) { if self.reversed { (self.highlights, self.shadows) } else { (self.shadows, self.highlights) } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrainSettings { pub amount: f64, pub size: f64, pub roughness: f64, pub seed: u32 }
impl Default for GrainSettings { fn default() -> Self { GrainSettings { amount: 25.0, size: 1.5, roughness: 50.0, seed: 0 } } }
impl GrainSettings {
    pub fn is_valid(&self) -> bool { (0.0..=100.0).contains(&self.amount) && (0.5..=20.0).contains(&self.size) && (0.0..=100.0).contains(&self.roughness) }
    pub fn normalized(&self) -> Self { GrainSettings { amount: clamp_or(self.amount, 0.0, 100.0, 25.0), size: clamp_or(self.size, 0.5, 20.0, 1.5), roughness: clamp_or(self.roughness, 0.0, 100.0, 50.0), seed: self.seed } }
}

/// The Mac's `LayerAdjustment`: legacy scalar HSV fields plus optional range-aware settings, so
/// files written before those settings existed decode and re-encode byte for byte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerAdjustment {
    pub kind: AdjustmentKind,
    #[serde(default)] pub hue: f64,
    #[serde(default)] pub saturation: f64,
    #[serde(default)] pub lightness: f64,
    #[serde(default)] pub colorize: bool,
    #[serde(rename = "hsvSettings", default, skip_serializing_if = "Option::is_none")] pub hsv_settings: Option<HueSaturationSettings>,
    #[serde(default)] pub levels: LevelsSettings,
    #[serde(default)] pub curves: CurvesSettings,
    #[serde(rename = "exposureSettings", default, skip_serializing_if = "Option::is_none")] pub exposure_settings: Option<ExposureSettings>,
    #[serde(rename = "gradientMapSettings", default, skip_serializing_if = "Option::is_none")] pub gradient_map_settings: Option<GradientMapSettings>,
    #[serde(rename = "grainSettings", default, skip_serializing_if = "Option::is_none")] pub grain_settings: Option<GrainSettings>,
}
impl LayerAdjustment {
    pub fn new(kind: AdjustmentKind) -> Self {
        LayerAdjustment { kind, hue: 0.0, saturation: 0.0, lightness: 0.0, colorize: false, hsv_settings: None, levels: LevelsSettings::default(),
            curves: CurvesSettings::default(), exposure_settings: None, gradient_map_settings: None, grain_settings: None }
    }
    pub fn resolved_hsv(&self) -> HueSaturationSettings {
        self.hsv_settings.clone().unwrap_or_else(|| HueSaturationSettings::new(self.hue, self.saturation, self.lightness, self.colorize, ColorRange::Master))
    }
    pub fn exposure(&self) -> ExposureSettings { self.exposure_settings.unwrap_or_default() }
    pub fn gradient_map(&self) -> GradientMapSettings { self.gradient_map_settings.unwrap_or_default() }
    pub fn grain(&self) -> GrainSettings { self.grain_settings.unwrap_or_default() }
    pub fn is_valid(&self) -> bool {
        self.hue.is_finite() && self.saturation.is_finite() && self.lightness.is_finite() && self.hue.abs() <= 360.0 && self.saturation.abs() <= 100.0 && self.lightness.abs() <= 100.0
            && self.resolved_hsv().is_valid() && self.levels.is_valid() && self.curves.is_valid()
            && self.exposure().is_valid() && self.gradient_map().is_valid() && self.grain().is_valid()
    }
    /// Whether applying this adjustment would change nothing (used to skip no-op commits).
    pub fn is_identity(&self) -> bool {
        match self.kind {
            AdjustmentKind::Hsv => self.resolved_hsv().is_identity(),
            AdjustmentKind::Levels => self.levels.is_identity(),
            AdjustmentKind::Curves => self.curves.is_identity(),
            AdjustmentKind::Exposure => self.exposure() == ExposureSettings::default(),
            AdjustmentKind::GradientMap => false,
            AdjustmentKind::Grain => self.grain().amount <= 0.0,
        }
    }
}
```

`engine/src/document.rs`: change `LayerExtra.adjustment` to `Option<LayerAdjustment>` (import it), and add to the `impl Layer` block near `has_pixels`:
```rust
    pub fn is_adjustment(&self) -> bool { self.extra.adjustment.is_some() }
```
`Layer::record` and `Layer::from_record` already copy the field; they compile unchanged once `LayerRecord.adjustment` is typed.

`engine/src/manifest.rs`: `pub adjustment: Option<LayerAdjustment>` (same serde attributes), import `crate::LayerAdjustment`; in `validate()`, inside the per-layer loop (after the version check), add:
```rust
            if let Some(adjustment) = &layer.adjustment {
                if self.version < 7 || layer.is_group() || layer.image_file.is_some() || !adjustment.is_valid() { return Err(Invalid); }
            }
```
`engine/src/lib.rs`: `pub mod adjust;` and `pub use adjust::settings::*;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass including 7 in `adjust_settings.rs`; the Phase 1/2 interop and package tests still pass (no fixture carries an adjustment).

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): adjustment settings with Mac-compatible JSON and manifest validation"
```

---

### Task 2: Levels kernel, histogram, Auto and eyedropper calibration

**Files:**
- Create: `engine/src/adjust/levels.rs`
- Modify: `engine/src/adjust/mod.rs` (`pub mod levels;`), `engine/src/lib.rs` (`pub use adjust::levels::*;`)
- Test: `engine/tests/levels.rs`

**Interfaces:**
- Produces: `levels_tables(&LevelsSettings) -> Vec<f32>` (768 = R,G,B tables), `apply_tables(&Raster, &[f32]) -> Raster` (the `levels_apply` port, used by Levels, Curves and Exposure), `histogram(&Raster, Option<&GrayRaster>) -> Vec<Vec<f64>>` (4 x 256: RGB mean, R, G, B), `LevelsAuto { Contrast, Color, Neutral }` with `settings(&self, &[Vec<f64>]) -> LevelsSettings`, `LevelsSample { Black, Gray, White }`, `LevelsSettings::sampling(&self, rgb: [f64; 3], mode) -> LevelsSettings`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/levels.rs`:
```rust
use compositor_engine::*;

fn ramp() -> Raster { Raster::from_premultiplied(6, 1, vec![0,0,0,255, 64,64,64,255, 128,128,128,255, 255,255,255,255, 64,32,0,128, 0,0,0,0]) }

#[test]
fn input_clipping_gamma_output_inversion_and_alpha() {
    let source = ramp();
    let mut s = LevelsSettings::default();
    s.ranges[0] = LevelRange { black: 64.0, gamma: 1.0, white: 128.0, ..LevelRange::default() };
    let clipped = apply_tables(&source, &levels_tables(&s));
    assert_eq!(&clipped.bytes()[0..12], &[0,0,0,255, 0,0,0,255, 255,255,255,255]);
    s.ranges[0] = LevelRange { gamma: 2.0, ..LevelRange::default() };
    let b = apply_tables(&source, &levels_tables(&s)).bytes().to_vec();
    assert!((b[4] as i32 - 128).abs() <= 1 && (b[8] as i32 - 181).abs() <= 1);
    assert!(b[19] == 128 && b[23] == 0 && b[16] <= 128 && b[17] <= 128);
    s.ranges[0] = LevelRange { output_black: 255.0, output_white: 0.0, ..LevelRange::default() };
    let inv = apply_tables(&source, &levels_tables(&s)).bytes().to_vec();
    assert!(inv[0] == 255 && inv[12] == 0);
    assert_eq!(&inv[16..20], &[64, 96, 128, 128], "one unpremultiply inside the kernel, as LevelsTests expects");
}

#[test]
fn identity_tables_are_an_exact_no_op() {
    let source = ramp();
    let out = apply_tables(&source, &levels_tables(&LevelsSettings::default()));
    assert_eq!(out.bytes(), source.bytes());
}

#[test]
fn histogram_excludes_transparency_and_weights_coverage() {
    let source = Raster::from_premultiplied(3, 1, vec![255,0,0,255, 0,128,0,128, 0,0,0,0]);
    let bins = histogram(&source, None);
    assert!(bins[1][255] == 1.0 && (bins[2][255] - 128.0 / 255.0).abs() < 1e-5);
    let total: f64 = bins[0].iter().sum();
    assert!((total - (1.0 + 128.0 / 255.0)).abs() < 1e-5);
    let cov = GrayRaster::from_bytes(3, 1, vec![255, 0, 0]);
    let selected = histogram(&source, Some(&cov));
    assert!(selected[1][255] == 1.0 && selected[2][255] == 0.0);
}

#[test]
fn auto_algorithms_and_eyedropper_calibration() {
    let mut bins = vec![vec![0.0; 256]; 4];
    for c in 1..=3 { bins[c][20 * c] = 100.0; bins[c][200 + c * 10] = 100.0; }
    let linked = LevelsAuto::Contrast.settings(&bins);
    assert!(linked.ranges[0].black == 20.0 && linked.ranges[0].white == 230.0);
    let color = LevelsAuto::Color.settings(&bins);
    assert!(color.ranges[1].black == 20.0 && color.ranges[3].black == 60.0 && color.ranges[0] == LevelRange::default());
    assert_eq!(LevelsAuto::Neutral.settings(&bins).ranges[1].gamma, 1.0);
    let empty = vec![vec![0.0; 256]; 4];
    for mode in [LevelsAuto::Contrast, LevelsAuto::Color, LevelsAuto::Neutral] { assert!(mode.settings(&empty).is_identity()); }
    let rgb = [0.25, 0.4, 0.6];
    for mode in [LevelsSample::Black, LevelsSample::Gray, LevelsSample::White] {
        let s = LevelsSettings::default().sampling(rgb, mode);
        let target = match mode { LevelsSample::Black => 0.0, LevelsSample::White => 1.0, LevelsSample::Gray => 0.5 };
        for (i, ch) in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue].iter().enumerate() {
            assert!((s.apply(rgb[i], *ch) - target).abs() < 1e-4, "{mode:?} {ch:?}");
        }
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test levels`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/adjust/levels.rs`:
```rust
use crate::{GrayRaster, LevelRange, LevelsChannel, LevelsSettings, Raster};

/// R, G, B tables of 256 outputs (0..1), each the channel's range then the composite range.
pub fn levels_tables(s: &LevelsSettings) -> Vec<f32> {
    let mut out = Vec::with_capacity(768);
    for ch in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue] {
        for v in 0..=255u32 { out.push(s.apply(v as f64 / 255.0, ch) as f32); }
    }
    out
}

/// `levels_apply` from LevelsPixels.c: per channel, unpremultiply once, interpolate the table, re-premultiply.
pub fn apply_tables(raster: &Raster, tables: &[f32]) -> Raster {
    assert_eq!(tables.len(), 768);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let alpha = p[3] as f32;
        if alpha == 0.0 { continue; }
        for c in 0..3 {
            let x = (p[c] as f32 * 255.0 / alpha).min(255.0);
            let lo = x as usize; let hi = if lo < 255 { lo + 1 } else { 255 };
            let t = &tables[c * 256..c * 256 + 256];
            let result = t[lo] + (t[hi] - t[lo]) * (x - lo as f32);
            p[c] = (result * alpha).round().max(0.0).min(alpha) as u8;
        }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// `levels_histogram`: 4 x 256 bins (RGB mean of the three, then R, G, B), weighted by alpha and coverage.
pub fn histogram(raster: &Raster, coverage: Option<&GrayRaster>) -> Vec<Vec<f64>> {
    let mut bins = vec![vec![0.0f64; 256]; 4];
    for (i, p) in raster.bytes().chunks_exact(4).enumerate() {
        if p[3] == 0 { continue; }
        let weight = p[3] as f64 / 255.0 * coverage.map_or(1.0, |c| c.bytes()[i] as f64 / 255.0);
        for c in 0..3 {
            let value = ((p[c] as f64 * 255.0 / p[3] as f64).round()).min(255.0) as usize;
            bins[c + 1][value] += weight;
            bins[0][value] += weight / 3.0;
        }
    }
    bins
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LevelsAuto { Contrast, Color, Neutral }

fn endpoints(bins: &[f64]) -> Option<(f64, f64)> {
    let total: f64 = bins.iter().sum();
    if total <= 0.0 { return None; }
    let (mut sum, mut low, mut high) = (0.0, 0usize, 255usize);
    for i in 0..256 { sum += bins[i]; if sum > total * 0.001 { low = i; break; } }
    sum = 0.0;
    for i in (0..256).rev() { sum += bins[i]; if sum > total * 0.001 { high = i; break; } }
    if low < high { Some((low as f64, high as f64)) } else { None }
}

impl LevelsAuto {
    pub fn settings(&self, histogram: &[Vec<f64>]) -> LevelsSettings {
        let mut result = LevelsSettings::default();
        if *self == LevelsAuto::Contrast {
            // A shared interval preserves channel relationships.
            let limits: Vec<(f64, f64)> = histogram[1..4].iter().filter_map(|b| endpoints(b)).collect();
            let low = limits.iter().map(|l| l.0).fold(f64::INFINITY, f64::min);
            let high = limits.iter().map(|l| l.1).fold(f64::NEG_INFINITY, f64::max);
            if !limits.is_empty() && low < high { result.ranges[0] = LevelRange { black: low, white: high, ..LevelRange::default() }; }
        } else {
            for c in 1..=3 {
                let Some((low, high)) = endpoints(&histogram[c]) else { continue; };
                let mut range = LevelRange { black: low, white: high, ..LevelRange::default() };
                if *self == LevelsAuto::Neutral {
                    let total: f64 = histogram[c].iter().sum();
                    let mean = histogram[c].iter().enumerate().fold(0.0, |acc, (i, w)| acc + range.apply(i as f64 / 255.0) * w) / total;
                    if mean > 0.0 && mean < 1.0 { range.gamma = (mean.ln() / 0.5f64.ln()).clamp(0.1, 9.99); }
                }
                result.ranges[c] = range;
            }
        }
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LevelsSample { Black, Gray, White }

impl LevelsSettings {
    /// Samples are straight RGB 0..1; all three channels are calibrated together.
    pub fn sampling(&self, rgb: [f64; 3], mode: LevelsSample) -> LevelsSettings {
        let mut result = self.clone();
        result.ranges[0] = LevelRange::default();
        for c in 1..=3 {
            let mut range = result.ranges[c];
            let v = rgb[c - 1] * 255.0;
            match mode {
                LevelsSample::Black => range.black = v.max(0.0).min(range.white - 1.0),
                LevelsSample::White => range.white = v.min(255.0).max(range.black + 1.0),
                LevelsSample::Gray => {
                    let fraction = (v - range.black) / (range.white - range.black);
                    if !(fraction > 0.0 && fraction < 1.0) { continue; }
                    range.gamma = fraction.ln() / 0.5f64.ln();
                }
            }
            range.output_black = 0.0; range.output_white = 255.0;
            result.ranges[c] = range.normalized();
        }
        result
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 4 in `levels.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): levels tables, kernel, histogram, auto levels and eyedropper calibration"
```

---

### Task 3: Curves tables, Exposure, Gradient Map and Invert

**Files:**
- Create: `engine/src/adjust/curves.rs`, `engine/src/adjust/tonal.rs`
- Modify: `engine/src/adjust/mod.rs` (`pub mod curves; pub mod tonal;`), `engine/src/lib.rs` (re-export both)
- Test: `engine/tests/curves_tonal.rs`

**Interfaces:**
- Produces: `curves_tables(&CurvesSettings) -> Vec<f32>` (768), `exposure_table(&ExposureSettings) -> Vec<f32>` (768: the same 256-entry curve three times), `gradient_map_table(&GradientMapSettings) -> Vec<u8>` (768), `apply_gradient_map(&Raster, &[u8]) -> Raster`, `invert_raster(&Raster) -> Raster`, `invert_gray(&GrayRaster) -> GrayRaster`.
- Consumes: `apply_tables` from Task 2.

- [ ] **Step 1: Write the failing tests**

`engine/tests/curves_tonal.rs`:
```rust
use compositor_engine::*;

fn solid(r: u8, g: u8, b: u8, a: u8) -> Raster {
    let (r, g, b) = if a == 255 { (r, g, b) } else { ((r as u32 * a as u32 / 255) as u8, (g as u32 * a as u32 / 255) as u8, (b as u32 * a as u32 / 255) as u8) };
    Raster::from_premultiplied(2, 2, [r, g, b, a].repeat(4))
}
/// Straight (unpremultiplied) bytes of the first pixel.
fn straight(raster: &Raster) -> [u32; 4] {
    let p = raster.pixel(0, 0);
    let a = p[3] as u32;
    if a == 0 { return [0, 0, 0, 0]; }
    [(p[0] as u32 * 255 + a / 2) / a, (p[1] as u32 * 255 + a / 2) / a, (p[2] as u32 * 255 + a / 2) / a, a]
}

#[test]
fn exposure_works_in_linear_light_with_offset_and_gamma() {
    let gray = solid(128, 128, 128, 255);
    assert_eq!(apply_tables(&gray, &exposure_table(&ExposureSettings::default())).bytes(), gray.bytes(), "defaults change nothing");
    let brighter = straight(&apply_tables(&gray, &exposure_table(&ExposureSettings { exposure: 1.0, ..Default::default() })));
    assert!((brighter[0] as i64 - 176).abs() <= 2, "+1 stop doubles linear light: {brighter:?}");
    assert_eq!(brighter[0], brighter[2]);
    let lifted = straight(&apply_tables(&gray, &exposure_table(&ExposureSettings { gamma: 2.0, ..Default::default() })));
    assert!((lifted[0] as i64 - 181).abs() <= 2, "gamma 2 takes the square root of linear light: {lifted:?}");
    let offset = straight(&apply_tables(&solid(0, 0, 0, 255), &exposure_table(&ExposureSettings { offset: 0.1, ..Default::default() })));
    assert!((offset[0] as i64 - 89).abs() <= 2, "offset adds linear light: {offset:?}");
    let translucent = apply_tables(&solid(128, 128, 128, 128), &exposure_table(&ExposureSettings { exposure: 1.0, ..Default::default() }));
    assert_eq!(translucent.pixel(0, 0)[3], 128, "alpha kept");
}

#[test]
fn gradient_map_colors_by_brightness_and_reverses() {
    let mut s = GradientMapSettings { shadows: AdjustmentColor { red: 1.0, green: 0.0, blue: 0.0 }, highlights: AdjustmentColor { red: 0.0, green: 0.0, blue: 1.0 }, reversed: false };
    let table = gradient_map_table(&s);
    assert_eq!(straight(&apply_gradient_map(&solid(0, 0, 0, 255), &table)), [255, 0, 0, 255]);
    assert_eq!(straight(&apply_gradient_map(&solid(255, 255, 255, 255), &table)), [0, 0, 255, 255]);
    let middle = straight(&apply_gradient_map(&solid(128, 128, 128, 255), &table));
    assert!((middle[0] as i64 - 127).abs() <= 2 && (middle[2] as i64 - 128).abs() <= 2 && middle[1] == 0, "{middle:?}");
    let translucent = straight(&apply_gradient_map(&solid(255, 255, 255, 128), &table));
    assert!(translucent[2] >= 250 && translucent[0] <= 5 && translucent[3] == 128, "{translucent:?}");
    s.reversed = true;
    assert_eq!(straight(&apply_gradient_map(&solid(0, 0, 0, 255), &gradient_map_table(&s))), [0, 0, 255, 255]);
}

#[test]
fn curves_identity_is_exact_and_a_flat_curve_whitens_while_keeping_alpha() {
    let source = Raster::from_premultiplied(2, 2, vec![102,179,26,255, 51,89,13,128, 13,22,3,32, 0,0,0,0]);
    assert_eq!(apply_tables(&source, &curves_tables(&CurvesSettings::default())).bytes(), source.bytes());
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let out = apply_tables(&source, &curves_tables(&s));
    assert_eq!(out.bytes(), &[255,255,255,255, 128,128,128,128, 32,32,32,32, 0,0,0,0], "white at every input, premultiplied by the original alpha");
}

#[test]
fn a_curve_point_moves_the_tone_it_names() {
    let mut s = CurvesSettings::default();
    s.channels[0] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let out = straight(&apply_tables(&solid(128, 128, 128, 255), &curves_tables(&s)));
    assert!((out[0] as i64 - 190).abs() <= 1, "{out:?}");
    let mut red_only = CurvesSettings::default();
    red_only.channels[LevelsChannel::Red.index()] = vec![CurvePoint { x: 0.0, y: 0.0 }, CurvePoint { x: 255.0, y: 128.0 }];
    let halved = straight(&apply_tables(&solid(200, 200, 200, 255), &curves_tables(&red_only)));
    assert!(halved[0] < halved[1] && halved[1] == halved[2], "the red channel alone: {halved:?}");
}

#[test]
fn invert_keeps_transparency_and_round_trips() {
    let source = Raster::from_premultiplied(2, 2, vec![255,0,0,255, 64,32,0,128, 0,0,0,0, 10,20,30,40]);
    let inverted = invert_raster(&source);
    assert_eq!(&inverted.bytes()[0..8], &[0,255,255,255, 64,96,128,128], "premultiplied colour becomes alpha minus colour");
    assert_eq!(&inverted.bytes()[8..12], &[0,0,0,0], "clear pixels stay clear");
    assert_eq!(invert_raster(&inverted).bytes(), source.bytes());
    let gray = GrayRaster::from_bytes(2, 1, vec![0, 200]);
    assert_eq!(invert_gray(&gray).bytes(), &[255, 55]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test curves_tonal`
Expected: compile errors.

- [ ] **Step 3: Implement curves.rs**

`engine/src/adjust/curves.rs`:
```rust
use crate::{CurvesSettings, LevelsChannel};

/// R, G, B tables of 256 outputs (0..1): the channel's own curve, then the composite RGB curve.
pub fn curves_tables(s: &CurvesSettings) -> Vec<f32> {
    let mut out = Vec::with_capacity(768);
    for ch in [LevelsChannel::Red, LevelsChannel::Green, LevelsChannel::Blue] {
        for v in 0..=255u32 { out.push((s.value(s.value(v as f64, ch.index()), 0) / 255.0) as f32); }
    }
    out
}
```

- [ ] **Step 4: Implement tonal.rs**

`engine/src/adjust/tonal.rs`:
```rust
use crate::{AdjustmentColor, ExposureSettings, GradientMapSettings, GrayRaster, Raster};

fn srgb_to_linear(v: f64) -> f64 { if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) } }
fn linear_to_srgb(v: f64) -> f64 { if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 } }

/// Photoshop's Exposure: scale linear light by 2^stops, add the offset, then correct gamma.
/// The same curve on every channel, so `apply_tables` gets three copies of it.
pub fn exposure_table(s: &ExposureSettings) -> Vec<f32> {
    let s = s.normalized();
    let scale = 2f64.powf(s.exposure);
    let one: Vec<f32> = (0..=255u32).map(|i| {
        let encoded = i as f64 / 255.0;
        let linear = (srgb_to_linear(encoded) * scale + s.offset).max(0.0).powf(1.0 / s.gamma);
        linear_to_srgb(linear).clamp(0.0, 1.0) as f32
    }).collect();
    let mut out = Vec::with_capacity(768);
    for _ in 0..3 { out.extend_from_slice(&one); }
    out
}

/// 256 RGB triples interpolating between the dark and light ends.
pub fn gradient_map_table(s: &GradientMapSettings) -> Vec<u8> {
    let s = s.normalized();
    let (dark, light) = s.ends();
    let channel = |d: f64, l: f64, t: f64| ((d + (l - d) * t) * 255.0).round().clamp(0.0, 255.0) as u8;
    let mut out = Vec::with_capacity(768);
    for i in 0..=255u32 {
        let t = i as f64 / 255.0;
        out.push(channel(dark.red, light.red, t));
        out.push(channel(dark.green, light.green, t));
        out.push(channel(dark.blue, light.blue, t));
    }
    out
}

/// `adjust_gradient_map` from AdjustPixels.c: Rec. 709 integer luma of the unpremultiplied
/// colour picks a table entry, which is written back premultiplied.
pub fn apply_gradient_map(raster: &Raster, table: &[u8]) -> Raster {
    assert_eq!(table.len(), 768);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as u32;
        if a == 0 { continue; }
        let (mut r, mut g, mut b) = (p[0] as u32, p[1] as u32, p[2] as u32);
        if a < 255 {
            r = ((r * 255 + a / 2) / a).min(255);
            g = ((g * 255 + a / 2) / a).min(255);
            b = ((b * 255 + a / 2) / a).min(255);
        }
        let level = ((2126 * r + 7152 * g + 722 * b + 5000) / 10000).min(255) as usize;
        for c in 0..3 { p[c] = ((table[level * 3 + c] as u32 * a + 127) / 255) as u8; }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// Premultiplied invert: each colour becomes alpha minus colour, so transparency is kept.
pub fn invert_raster(raster: &Raster) -> Raster {
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) { let a = p[3]; for c in 0..3 { p[c] = a.saturating_sub(p[c]); } }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
pub fn invert_gray(mask: &GrayRaster) -> GrayRaster {
    GrayRaster::from_bytes(mask.width, mask.height, mask.bytes().iter().map(|v| 255 - v).collect())
}

/// The colour a new Gradient Map starts from, as the Mac takes the palette's two colours.
pub fn gradient_map_from(shadows: [f64; 3], highlights: [f64; 3]) -> GradientMapSettings {
    GradientMapSettings {
        shadows: AdjustmentColor { red: shadows[0], green: shadows[1], blue: shadows[2] }.clamped(),
        highlights: AdjustmentColor { red: highlights[0], green: highlights[1], blue: highlights[2] }.clamped(),
        reversed: false,
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 5 in `curves_tonal.rs`.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): curves tables, exposure, gradient map and invert"
```

---

### Task 4: Hue/Saturation maths

**Files:**
- Create: `engine/src/adjust/hsv.rs`
- Modify: `engine/src/adjust/mod.rs`, `engine/src/lib.rs`
- Test: `engine/tests/hsv.rs`

**Interfaces:**
- Produces: `hue_response(&HueSaturationSettings) -> Vec<[f64; 3]>` (361 entries of shift, saturation, lightness), `adjust_rgb(rgb: [f64; 3], &HueSaturationSettings, &[[f64; 3]]) -> [f64; 3]`, `shifted_hue(hue, &HueSaturationSettings) -> f64` (for the panel's after-bar), `rgb_to_hsl([f64;3]) -> [f64;3]`, `hsl_to_rgb([f64;3]) -> [f64;3]`, `apply_hsv(&Raster, &HueSaturationSettings) -> Raster`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/hsv.rs`:
```rust
use compositor_engine::*;

fn straight(raster: &Raster, index: usize) -> [i64; 4] {
    let d = &raster.bytes()[index * 4..index * 4 + 4];
    let a = d[3] as i64;
    if a == 0 { return [0, 0, 0, 0]; }
    [(d[0] as i64 * 255 + a / 2) / a, (d[1] as i64 * 255 + a / 2) / a, (d[2] as i64 * 255 + a / 2) / a, a]
}
fn near(v: [i64; 4], t: [i64; 4]) -> bool { v.iter().zip(t.iter()).all(|(a, b)| (a - b).abs() <= 2) }
/// Red, mid gray, pure blue, and a half-transparent blue.
fn fixture() -> Raster { Raster::from_premultiplied(4, 1, vec![255,0,0,255, 128,128,128,255, 0,0,255,255, 0,0,128,128]) }

#[test]
fn hue_rotates_saturation_and_lightness_follow_photoshop() {
    let out = apply_hsv(&fixture(), &HueSaturationSettings::new(120.0, 0.0, 0.0, false, ColorRange::Master));
    assert!(near(straight(&out, 0), [0, 255, 0, 255]), "red to green: {:?}", straight(&out, 0));
    assert!(near(straight(&out, 1), [128, 128, 128, 255]), "gray is unchanged: {:?}", straight(&out, 1));
    let flat = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, -100.0, 0.0, false, ColorRange::Master));
    let gray = straight(&flat, 0);
    assert!(gray[0] == gray[1] && gray[1] == gray[2] && gray[3] == 255, "{gray:?}");
    let white = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, 0.0, 100.0, false, ColorRange::Master));
    assert!(near(straight(&white, 0), [255, 255, 255, 255]));
    let black = apply_hsv(&fixture(), &HueSaturationSettings::new(0.0, 0.0, -100.0, false, ColorRange::Master));
    assert!(near(straight(&black, 0), [0, 0, 0, 255]));
}

#[test]
fn identity_settings_change_nothing_and_alpha_is_kept() {
    let source = fixture();
    assert_eq!(apply_hsv(&source, &HueSaturationSettings::default()).bytes(), source.bytes());
    let colorized = apply_hsv(&source, &HueSaturationSettings::new(240.0, 100.0, 0.0, true, ColorRange::Master));
    let strip = straight(&colorized, 3);
    assert_eq!(strip[3], 128, "half-transparent pixels keep their alpha");
    for i in 0..3 { let p = straight(&colorized, i); assert!(p[2] > p[0], "everything turns blue-ish: {p:?}"); }
}

#[test]
fn ranges_adjust_independently_and_invert_flips_the_band() {
    let mut s = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Reds);
    s.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: 0.0, saturation: -100.0, lightness: 0.0 });
    let out = apply_hsv(&fixture(), &s);
    assert!(near(straight(&out, 0), [255, 255, 0, 255]), "reds rotate to yellow: {:?}", straight(&out, 0));
    let blue = straight(&out, 2);
    assert!(blue[0] == blue[1] && blue[1] == blue[2], "blues desaturate: {blue:?}");
    let reds_only = HueSaturationSettings::new(0.0, 0.0, -100.0, false, ColorRange::Reds);
    let out = apply_hsv(&fixture(), &reds_only);
    assert!(near(straight(&out, 0), [0, 0, 0, 255]) && near(straight(&out, 2), [0, 0, 255, 255]));
    let mut inverted = reds_only.clone();
    inverted.invert_range = true;
    let out = apply_hsv(&fixture(), &inverted);
    assert!(near(straight(&out, 0), [255, 0, 0, 255]) && near(straight(&out, 2), [0, 0, 0, 255]));
}

#[test]
fn the_response_table_and_after_bar_follow_hue_shifts() {
    let mut greens = HueSaturationSettings::new(60.0, 0.0, 0.0, false, ColorRange::Greens);
    greens.adjustments.insert(ColorRange::Master, RangeAdjustment::default());
    assert!((shifted_hue(120.0, &greens) - 180.0).abs() < 0.001);
    assert!((shifted_hue(0.0, &greens) - 0.0).abs() < 0.001);
    let response = hue_response(&greens);
    assert_eq!(response.len(), 361);
    assert!((response[120][0] - 60.0).abs() < 0.001 && response[0][0] == 0.0);
}

#[test]
fn hsl_round_trips() {
    for rgb in [[1.0, 0.0, 0.0], [0.2, 0.7, 0.4], [0.5, 0.5, 0.5], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0]] {
        let back = hsl_to_rgb(rgb_to_hsl(rgb));
        for c in 0..3 { assert!((back[c] - rgb[c]).abs() < 1e-9, "{rgb:?} -> {back:?}"); }
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test hsv`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/adjust/hsv.rs`:
```rust
use crate::{ColorRange, HueSaturationSettings, Raster};

pub fn rgb_to_hsl(rgb: [f64; 3]) -> [f64; 3] {
    let (r, g, b) = (rgb[0], rgb[1], rgb[2]);
    let high = r.max(g).max(b); let low = r.min(g).min(b);
    let lightness = (high + low) / 2.0;
    let delta = high - low;
    if delta <= 0.0 { return [0.0, 0.0, lightness]; }
    let saturation = (delta / (1.0 - (2.0 * lightness - 1.0).abs())).min(1.0);
    let mut hue = if high == r { (g - b) / delta } else if high == g { (b - r) / delta + 2.0 } else { (r - g) / delta + 4.0 };
    hue *= 60.0;
    if hue < 0.0 { hue += 360.0; }
    [hue, saturation, lightness]
}

pub fn hsl_to_rgb(hsl: [f64; 3]) -> [f64; 3] {
    let (hue, saturation, lightness) = (hsl[0], hsl[1], hsl[2]);
    if saturation <= 0.0 { return [lightness, lightness, lightness]; }
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let base = lightness - chroma / 2.0;
    let (r, g, b) = match sector as i64 {
        0 => (chroma, second, 0.0), 1 => (second, chroma, 0.0), 2 => (0.0, chroma, second),
        3 => (0.0, second, chroma), 4 => (second, 0.0, chroma), _ => (chroma, 0.0, second),
    };
    [(r + base).clamp(0.0, 1.0), (g + base).clamp(0.0, 1.0), (b + base).clamp(0.0, 1.0)]
}

/// How much every range shifts each whole degree of hue: built once per settings so a per-pixel
/// adjustment does not re-evaluate all seven ranges.
pub fn hue_response(settings: &HueSaturationSettings) -> Vec<[f64; 3]> {
    (0..=360).map(|degree| {
        let mut response = [0.0f64; 3];
        for (range, adjustment) in &settings.adjustments {
            if *adjustment == Default::default() { continue; }
            let weight = settings.weight(*range, degree as f64);
            if weight <= 0.0 { continue; }
            response[0] += adjustment.hue * weight;
            response[1] += adjustment.saturation * weight;
            response[2] += adjustment.lightness * weight;
        }
        response
    }).collect()
}

/// The hue a spectrum swatch becomes, for the panel's "after" bar.
pub fn shifted_hue(hue: f64, settings: &HueSaturationSettings) -> f64 {
    let mut shift = 0.0;
    for (range, adjustment) in &settings.adjustments {
        if adjustment.hue != 0.0 { shift += adjustment.hue * settings.weight(*range, hue); }
    }
    let shifted = (hue + shift) % 360.0;
    if shifted < 0.0 { shifted + 360.0 } else { shifted }
}

/// One straight colour through the settings. `response` is `hue_response`, passed in so a whole
/// raster shares it.
pub fn adjust_rgb(rgb: [f64; 3], settings: &HueSaturationSettings, response: &[[f64; 3]]) -> [f64; 3] {
    let [mut hue, mut saturation, lightness] = rgb_to_hsl(rgb);
    let mut lightness_amount = 0.0;
    if settings.colorize {
        let master = settings.adjustment(ColorRange::Master);
        hue = master.hue % 360.0;
        if hue < 0.0 { hue += 360.0; }
        saturation = (master.saturation / 100.0).clamp(0.0, 1.0);
        lightness_amount = master.lightness / 100.0;
    } else {
        let sampled = response[(hue.round() as usize).min(response.len() - 1)];
        lightness_amount = sampled[2] / 100.0;
        hue = (hue + sampled[0]) % 360.0;
        if hue < 0.0 { hue += 360.0; }
        // Multiplicative, so neutral grays stay neutral.
        saturation = (saturation * (1.0 + sampled[1] / 100.0)).clamp(0.0, 1.0);
    }
    // Lightness pulls toward white above 0 and toward black below, reaching either at +/-100.
    let amount = lightness_amount.clamp(-1.0, 1.0);
    let lightness = if amount >= 0.0 { lightness + (1.0 - lightness) * amount } else { lightness * (1.0 + amount) };
    hsl_to_rgb([hue, saturation, lightness.clamp(0.0, 1.0)])
}

/// A whole raster: unpremultiplied, adjusted, premultiplied again (what CIColorCube does on macOS).
pub fn apply_hsv(raster: &Raster, settings: &HueSaturationSettings) -> Raster {
    if settings.is_identity() { return raster.clone(); }
    let response = hue_response(settings);
    let mut data = raster.bytes().to_vec();
    for p in data.chunks_exact_mut(4) {
        let a = p[3] as f64;
        if a == 0.0 { continue; }
        let rgb = [(p[0] as f64 * 255.0 / a).min(255.0) / 255.0, (p[1] as f64 * 255.0 / a).min(255.0) / 255.0, (p[2] as f64 * 255.0 / a).min(255.0) / 255.0];
        let out = adjust_rgb(rgb, settings, &response);
        for c in 0..3 { p[c] = (out[c] * a).round().clamp(0.0, a) as u8; }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 5 in `hsv.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): hue/saturation with per-range bands, colorize and the response table"
```

---

### Task 5: Grain

**Files:**
- Create: `engine/src/adjust/grain.rs`
- Modify: `engine/src/adjust/mod.rs`, `engine/src/lib.rs`
- Test: `engine/tests/grain.rs`

**Interfaces:**
- Produces: `grain_noise(u: f64, v: f64, size: f64, roughness: f64, seed: u32) -> f32` (the -1..1 noise at a document point), `grain_strength(amount: f64) -> f32`, `grain_weight(level: f32) -> f32` (the midtone curve), `apply_grain(&Raster, &GrainSettings, origin: Point, units_per_pixel: f64) -> Raster`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/grain.rs`:
```rust
use compositor_engine::*;

fn gray(w: u32, h: u32, alpha: u8) -> Raster {
    let v = (128u32 * alpha as u32 / 255) as u8;
    Raster::from_premultiplied(w, h, [v, v, v, alpha].repeat((w * h) as usize))
}
fn at(raster: &Raster, x: u32, y: u32) -> [u8; 4] { raster.pixel(x, y) }

#[test]
fn grain_is_fixed_in_document_space_and_leaves_transparency_alone() {
    let s = GrainSettings { amount: 60.0, size: 2.0, roughness: 40.0, seed: 7 };
    let whole = apply_grain(&gray(40, 40, 255), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let values: std::collections::HashSet<u8> = whole.bytes().chunks_exact(4).map(|p| p[0]).collect();
    assert!(values.len() > 5, "grain varies the brightness");
    assert!(whole.bytes().chunks_exact(4).all(|p| p[0] == p[1] && p[1] == p[2]), "the same change on every channel");
    // A 20 x 20 piece drawn at its place in the document gets that part of the same pattern.
    let part = apply_grain(&gray(20, 20, 255), &s, Point { x: 10.0, y: 10.0 }, 1.0);
    for y in 0..20 { for x in 0..20 { assert_eq!(at(&part, x, y), at(&whole, x + 10, y + 10), "({x}, {y})"); } }
    let reseeded = apply_grain(&gray(40, 40, 255), &GrainSettings { seed: 8, ..s }, Point { x: 0.0, y: 0.0 }, 1.0);
    assert_ne!(reseeded.bytes(), whole.bytes(), "another seed, another pattern");
    let none = apply_grain(&gray(4, 4, 255), &GrainSettings { amount: 0.0, ..s }, Point { x: 0.0, y: 0.0 }, 1.0);
    assert_eq!(none.bytes(), gray(4, 4, 255).bytes(), "no amount, no change");
    let clear = apply_grain(&gray(4, 4, 0), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    assert!(clear.bytes().chunks_exact(4).all(|p| p[3] == 0 && p[0] == 0), "clear pixels stay clear");
}

#[test]
fn grain_is_strongest_in_the_midtones_and_bounded_by_its_amount() {
    let s = GrainSettings { amount: 100.0, size: 1.5, roughness: 50.0, seed: 3 };
    let mid = apply_grain(&gray(24, 24, 255), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let black = apply_grain(&Raster::from_premultiplied(24, 24, [0, 0, 0, 255].repeat(576)), &s, Point { x: 0.0, y: 0.0 }, 1.0);
    let spread = |r: &Raster| -> i64 {
        let v: Vec<i64> = r.bytes().chunks_exact(4).map(|p| p[0] as i64).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    assert!(spread(&mid) > spread(&black), "midtones take more grain than the shadows: {} vs {}", spread(&mid), spread(&black));
    // strength is amount/100 * 0.35 * 255, and the midtone weight peaks at 1.0.
    assert!(spread(&mid) <= 2 * (0.35 * 255.0) as i64 + 2);
    assert!((grain_weight(0.5) - 1.0).abs() < 1e-6 && (grain_weight(0.0) - 0.4).abs() < 1e-6);
}

#[test]
fn the_noise_field_is_smooth_and_repeatable() {
    let a = grain_noise(10.25, 4.75, 2.0, 40.0, 7);
    assert_eq!(a, grain_noise(10.25, 4.75, 2.0, 40.0, 7), "the same point is the same value");
    assert_ne!(a, grain_noise(10.25, 4.75, 2.0, 40.0, 8));
    assert!((-2.0..=2.0).contains(&a));
    // With no roughness the field varies smoothly inside a cell.
    let b = grain_noise(10.26, 4.75, 2.0, 0.0, 7);
    assert!((a as f64 - b as f64).abs() < 0.5, "{a} vs {b}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test grain`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/adjust/grain.rs`:
```rust
use crate::{GrainSettings, Point, Raster};

fn mix32(mut x: u32) -> u32 {
    x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// A value in -1..1 for an integer lattice point, fixed by the point and the seed. Two uniform
/// halves summed give a triangular spread, closer to film grain than flat noise.
fn lattice(ix: i64, iy: i64, seed: u32) -> f32 {
    let h = mix32((ix as u32).wrapping_mul(0x9E37_79B1) ^ mix32((iy as u32).wrapping_mul(0x85EB_CA77) ^ seed));
    (h & 0xFFFF) as f32 / 65535.0 + (h >> 16) as f32 / 65535.0 - 1.0
}

/// The noise at a document point: a smoothstep-interpolated lattice, mixed with per-pixel fine
/// noise by `roughness`. Document coordinates, so the pattern stays put as the canvas redraws.
pub fn grain_noise(u: f64, v: f64, size: f64, roughness: f64, seed: u32) -> f32 {
    let size = if size > 0.0 { size } else { 1.0 };
    let rough = (roughness / 100.0).clamp(0.0, 1.0) as f32;
    let fine_seed = mix32(seed ^ 0xA511_E9B3);
    let cell_x = (u / size).floor(); let cell_y = (v / size).floor();
    let smoothstep = |t: f32| t * t * (3.0 - 2.0 * t);
    let tx = smoothstep((u / size - cell_x) as f32);
    let ty = smoothstep((v / size - cell_y) as f32);
    let (ix, iy) = (cell_x as i64, cell_y as i64);
    let n00 = lattice(ix, iy, seed); let n10 = lattice(ix + 1, iy, seed);
    let n01 = lattice(ix, iy + 1, seed); let n11 = lattice(ix + 1, iy + 1, seed);
    let top = n00 + (n10 - n00) * tx; let bottom = n01 + (n11 - n01) * tx;
    // Blending neighbours narrows the spread; scaling restores about the lattice's own.
    let smooth = (top + (bottom - top) * ty) * 1.6;
    let fine = lattice(u.floor() as i64, v.floor() as i64, fine_seed);
    smooth + (fine - smooth) * rough
}

pub fn grain_strength(amount: f64) -> f32 { (if amount > 100.0 { 1.0 } else { amount / 100.0 }) as f32 * 0.35 * 255.0 }
/// Film grain shows most in the midtones.
pub fn grain_weight(level: f32) -> f32 { 0.4 + 2.4 * level * (1.0 - level) }

/// `adjust_grain` from AdjustPixels.c. `origin` and `units_per_pixel` place the raster's pixels in
/// document space (a whole layer at 1:1 is origin zero, one unit per pixel).
pub fn apply_grain(raster: &Raster, settings: &GrainSettings, origin: Point, units_per_pixel: f64) -> Raster {
    let s = settings.normalized();
    if !(s.amount > 0.0) || !(units_per_pixel > 0.0) { return raster.clone(); }
    let strength = grain_strength(s.amount);
    let mut data = raster.bytes().to_vec();
    let width = raster.width;
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let a = p[3] as u32;
        if a == 0 { continue; }
        let x = (i as u32 % width) as f64; let y = (i as u32 / width) as f64;
        let u = origin.x + (x + 0.5) * units_per_pixel;
        let v = origin.y + (y + 0.5) * units_per_pixel;
        let noise = grain_noise(u, v, s.size, s.roughness, s.seed);
        let unpremultiply = if a == 255 { 1.0 } else { 255.0 / a as f32 };
        let (r, g, b) = (p[0] as f32 * unpremultiply, p[1] as f32 * unpremultiply, p[2] as f32 * unpremultiply);
        let level = ((0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0).min(1.0);
        let delta = noise * strength * grain_weight(level);
        let coverage = a as f32 / 255.0;
        for (c, value) in [r, g, b].iter().enumerate() {
            p[c] = ((value + delta).clamp(0.0, 255.0) * coverage + 0.5) as u8;
        }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 3 in `grain.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): film grain fixed in document space"
```

---

### Task 6: Gaussian Blur, Motion Blur, Add Noise and Lens Correction

**Files:**
- Create: `engine/src/adjust/filters.rs`
- Modify: `engine/src/adjust/mod.rs`, `engine/src/lib.rs`
- Test: `engine/tests/filters.rs`

**Interfaces:**
- Produces: `FilterParams` (serde tag `filter`: `GaussianBlur { radius }`, `MotionBlur { angle, distance }`, `AddNoise { amount, gaussian, monochromatic, seed }`, `LensCorrection { distortion }`) with `name() -> &'static str`, `normalized()`, `is_identity()`, `margin() -> f64`, `scaled(factor: f64)`, `spreads() -> bool`; `gaussian_blur(&Raster, sigma) -> Raster`, `motion_blur(&Raster, angle, distance) -> Raster`, `add_noise(&Raster, amount, gaussian, monochromatic, seed) -> Raster`, `lens_distort(&Raster, k) -> Raster`, `apply_filter(&Raster, &FilterParams) -> Raster`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/filters.rs`:
```rust
use compositor_engine::*;

/// `width` x `height`, opaque white in the left `solid` columns, transparent elsewhere.
fn half(width: u32, height: u32, solid: u32) -> Raster {
    let mut data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height { for x in 0..solid {
        let i = ((y * width + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
    }}
    Raster::from_premultiplied(width, height, data)
}
fn alpha(raster: &Raster, x: u32, y: u32) -> i64 { raster.pixel(x, y)[3] as i64 }

#[test]
fn a_gaussian_blur_softens_a_hard_edge_and_fades_into_empty_space() {
    let blurred = gaussian_blur(&half(40, 20, 20), 3.0);
    assert!(alpha(&blurred, 20, 10) > 20 && alpha(&blurred, 20, 10) < 235, "the hard edge is now soft");
    assert!(alpha(&blurred, 24, 10) > 0, "it spreads into the empty half");
    assert!(alpha(&blurred, 38, 10) == 0, "but not six sigma away");
    assert!(alpha(&blurred, 0, 10) > 100 && alpha(&blurred, 0, 10) < 160, "the raster's own border fades: nothing lies beyond it");
    assert_eq!(gaussian_blur(&half(8, 8, 4), 0.0).bytes(), half(8, 8, 4).bytes(), "no sigma, no change");
}

#[test]
fn motion_blur_streaks_along_its_angle_counterclockwise_from_horizontal() {
    // One opaque dot in the middle of a transparent raster.
    let mut data = vec![0u8; 41 * 41 * 4];
    let i = ((20 * 41 + 20) * 4) as usize;
    data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
    let dot = Raster::from_premultiplied(41, 41, data);
    let horizontal = motion_blur(&dot, 0.0, 16.0);
    assert!(alpha(&horizontal, 24, 20) > 0 && alpha(&horizontal, 16, 20) > 0 && alpha(&horizontal, 20, 24) == 0);
    let vertical = motion_blur(&dot, 90.0, 16.0);
    assert!(alpha(&vertical, 20, 24) > 0 && alpha(&vertical, 20, 16) > 0 && alpha(&vertical, 24, 20) == 0);
    // 45 degrees runs up-right and down-left on screen, never up-left.
    let diagonal = motion_blur(&dot, 45.0, 16.0);
    assert!(alpha(&diagonal, 23, 17) > 0 && alpha(&diagonal, 17, 23) > 0 && alpha(&diagonal, 17, 17) == 0);
    // An even streak: the dot's alpha is spread over about `distance` pixels, not tapered to a point.
    let total: i64 = (0..41).map(|x| alpha(&horizontal, x, 20)).sum();
    assert!((total - 255).abs() <= 8, "energy is preserved: {total}");
    assert!(alpha(&horizontal, 20, 20) < 40, "no spike at the centre");
}

#[test]
fn add_noise_changes_color_but_never_alpha_and_monochromatic_keeps_grays() {
    let gray = Raster::from_premultiplied(32, 8, {
        let mut d = vec![0u8; 32 * 8 * 4];
        for y in 0..8 { for x in 0..16 { let i = ((y * 32 + x) * 4) as usize; d[i..i + 4].copy_from_slice(&[128, 128, 128, 255]); } }
        d
    });
    let color = add_noise(&gray, 10.0, false, false, 7);
    assert_eq!(color.bytes(), add_noise(&gray, 10.0, false, false, 7).bytes(), "the same seed gives the same grain");
    let opaque: Vec<[u8; 4]> = (0..8).flat_map(|y| (0..16).map(move |x| (x, y))).map(|(x, y)| color.pixel(x, y)).collect();
    assert!(opaque.iter().all(|p| p[3] == 255 && (112..=144).contains(&(p[0] as i64))));
    assert!(opaque.iter().map(|p| p[0]).collect::<std::collections::HashSet<_>>().len() > 5);
    assert!(opaque.iter().any(|p| p[0] != p[1]), "colour noise differs per channel");
    assert!((16..32).all(|x| { let p = color.pixel(x, 0); p[3] == 0 && p[0] == 0 }), "clear pixels stay clear");
    let mono = add_noise(&gray, 10.0, true, true, 7);
    assert!((0..16).all(|x| { let p = mono.pixel(x, 0); p[0] == p[1] && p[1] == p[2] && p[3] == 255 }));
}

#[test]
fn remove_distortion_bends_about_the_center_and_only_pincushion_opens_the_corners() {
    // Four quadrants of distinct opaque colours.
    let mut data = vec![0u8; 40 * 30 * 4];
    for y in 0..30u32 { for x in 0..40u32 {
        let index = (if y < 15 { 0 } else { 2 }) + if x < 20 { 0 } else { 1 };
        let i = ((y * 40 + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&[(index as f64 / 3.0 * 255.0) as u8, 128, (255.0 - index as f64 / 3.0 * 255.0) as u8, 255]);
    }}
    let source = Raster::from_premultiplied(40, 30, data);
    assert_eq!(lens_distort(&source, 0.0).bytes(), source.bytes(), "no distortion, no change");
    let barrel = lens_distort(&source, 100.0 / 100.0 * 0.35);
    assert!(alpha(&barrel, 0, 0) == 255 && alpha(&barrel, 39, 29) == 255, "straightening barrel stretches outward, nothing opens up");
    let pincushion = lens_distort(&source, -100.0 / 100.0 * 0.35);
    assert!(alpha(&pincushion, 0, 0) == 0 && alpha(&pincushion, 39, 29) == 0, "straightening pincushion pulls the edges in");
    assert_eq!(pincushion.pixel(20, 15), source.pixel(20, 15), "the middle stays put");
}

#[test]
fn filter_params_carry_their_name_margin_and_preview_scaling() {
    let blur = FilterParams::GaussianBlur { radius: 4.0 };
    assert_eq!(blur.name(), "Gaussian Blur");
    assert_eq!(blur.margin(), 14.0);
    assert_eq!(blur.scaled(0.5), FilterParams::GaussianBlur { radius: 2.0 });
    assert!(blur.spreads());
    let motion = FilterParams::MotionBlur { angle: 30.0, distance: 20.0 };
    assert_eq!(motion.margin(), 12.0);
    assert!(motion.spreads());
    let noise = FilterParams::AddNoise { amount: 10.0, gaussian: false, monochromatic: false, seed: 1 };
    assert!(!noise.spreads() && noise.margin() == 0.0);
    assert_eq!(noise.scaled(0.5), noise, "noise and lens correction do not scale with a preview");
    assert!(FilterParams::LensCorrection { distortion: 0.0 }.is_identity());
    assert_eq!(FilterParams::GaussianBlur { radius: 500.0 }.normalized(), FilterParams::GaussianBlur { radius: 250.0 });
    let json = serde_json::to_string(&motion).unwrap();
    assert_eq!(json, r#"{"filter":"MotionBlur","angle":30.0,"distance":20.0}"#);
    assert_eq!(serde_json::from_str::<FilterParams>(&json).unwrap(), motion);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test filters`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/adjust/filters.rs`:
```rust
use crate::Raster;
use serde::{Deserialize, Serialize};

fn clamp_or(n: f64, lo: f64, hi: f64, fallback: f64) -> f64 { if n.is_finite() { n.clamp(lo, hi) } else { fallback } }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "filter")]
pub enum FilterParams {
    /// Standard deviation in layer pixels, 0.1 to 250.
    GaussianBlur { radius: f64 },
    /// Direction in degrees counter-clockwise from horizontal (-90 to 90) and streak length in layer pixels (1 to 2000).
    MotionBlur { angle: f64, distance: f64 },
    /// Photoshop's percentage, 0.1 to 400.
    AddNoise { amount: f64, gaussian: bool, monochromatic: bool, seed: u32 },
    /// Remove Distortion, -100 to 100: positive straightens barrel, negative pincushion.
    LensCorrection { distortion: f64 },
}

/// Remove Distortion at +/-100 moves the corners by this share of their distance from the centre.
pub const LENS_STRENGTH: f64 = 0.35;

impl FilterParams {
    pub fn name(&self) -> &'static str {
        match self { FilterParams::GaussianBlur { .. } => "Gaussian Blur", FilterParams::MotionBlur { .. } => "Motion Blur",
            FilterParams::AddNoise { .. } => "Add Noise", FilterParams::LensCorrection { .. } => "Lens Correction" }
    }
    pub fn normalized(&self) -> FilterParams {
        match *self {
            FilterParams::GaussianBlur { radius } => FilterParams::GaussianBlur { radius: clamp_or(radius, 0.1, 250.0, 1.0) },
            FilterParams::MotionBlur { angle, distance } => FilterParams::MotionBlur { angle: clamp_or(angle, -90.0, 90.0, 0.0), distance: clamp_or(distance, 1.0, 2000.0, 10.0) },
            FilterParams::AddNoise { amount, gaussian, monochromatic, seed } => FilterParams::AddNoise { amount: clamp_or(amount, 0.1, 400.0, 10.0), gaussian, monochromatic, seed },
            FilterParams::LensCorrection { distortion } => FilterParams::LensCorrection { distortion: clamp_or(distortion, -100.0, 100.0, 0.0) },
        }
    }
    /// Whether applying this would change nothing.
    pub fn is_identity(&self) -> bool {
        match self.normalized() { FilterParams::LensCorrection { distortion } => distortion == 0.0, _ => false }
    }
    /// The room the filter needs around the layer, in layer pixels: about three standard
    /// deviations, or half a streak.
    pub fn margin(&self) -> f64 {
        match self.normalized() {
            FilterParams::GaussianBlur { radius } => radius * 3.0 + 2.0,
            FilterParams::MotionBlur { distance, .. } => distance / 2.0 + 2.0,
            _ => 0.0,
        }
    }
    pub fn spreads(&self) -> bool { self.margin() > 0.0 }
    /// A preview rendered from a raster reduced by `factor` blurs proportionally less. Noise and
    /// lens correction are relative to the raster's own size, so they do not scale.
    pub fn scaled(&self, factor: f64) -> FilterParams {
        match *self {
            FilterParams::GaussianBlur { radius } => FilterParams::GaussianBlur { radius: radius * factor },
            FilterParams::MotionBlur { angle, distance } => FilterParams::MotionBlur { angle, distance: distance * factor },
            other => other,
        }
    }
}

/// Bilinear sample in premultiplied bytes; zero outside the raster, so a blur fades at the edge
/// instead of smearing the border outwards.
fn sample_zero(raster: &Raster, x: f64, y: f64) -> [f32; 4] {
    let (w, h) = (raster.width as i64, raster.height as i64);
    let fetch = |px: i64, py: i64| -> [f32; 4] {
        if px < 0 || py < 0 || px >= w || py >= h { return [0.0; 4]; }
        let p = raster.pixel(px as u32, py as u32);
        [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32]
    };
    let fx = x - 0.5; let fy = y - 0.5;
    let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    let (a, b, c, d) = (fetch(xu, yu), fetch(xu + 1, yu), fetch(xu, yu + 1), fetch(xu + 1, yu + 1));
    let mut out = [0f32; 4];
    for i in 0..4 { out[i] = (a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty; }
    out
}

/// Separable Gaussian on premultiplied channels, transparent beyond the raster.
pub fn gaussian_blur(raster: &Raster, sigma: f64) -> Raster {
    if !(sigma > 0.0) || raster.width == 0 || raster.height == 0 { return raster.clone(); }
    let radius = (sigma * 3.0).ceil() as i64;
    let kernel: Vec<f32> = (-radius..=radius).map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let (w, h) = (raster.width as i64, raster.height as i64);
    let src = raster.bytes();
    let mut tmp = vec![0f32; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sx = x + k as i64 - radius;
            if sx < 0 || sx >= w { continue; }
            let i = ((y * w + sx) * 4) as usize;
            for c in 0..4 { acc[c] += src[i + c] as f32 * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { tmp[i + c] = acc[c] / sum; }
    }}
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for (k, weight) in kernel.iter().enumerate() {
            let sy = y + k as i64 - radius;
            if sy < 0 || sy >= h { continue; }
            let i = ((sy * w + x) * 4) as usize;
            for c in 0..4 { acc[c] += tmp[i + c] * weight; }
        }
        let i = ((y * w + x) * 4) as usize;
        let alpha = (acc[3] / sum).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        // Premultiplied colour can never exceed alpha, or the result reads as over-bright.
        for c in 0..3 { out[i + c] = (acc[c] / sum).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(raster.width, raster.height, out)
}

/// An even streak of `distance` pixels along `angle` degrees, counter-clockwise from horizontal on
/// screen (so the direction in top-down pixels is (cos a, -sin a)), as Photoshop smears.
pub fn motion_blur(raster: &Raster, angle: f64, distance: f64) -> Raster {
    let steps = distance.round().max(1.0) as i64;
    if steps <= 1 { return raster.clone(); }
    let radians = angle.to_radians();
    let (dx, dy) = (radians.cos(), -radians.sin());
    let half = (steps - 1) as f64 / 2.0;
    let (w, h) = (raster.width, raster.height);
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..h { for x in 0..w {
        let mut acc = [0f32; 4];
        for i in 0..steps {
            let t = i as f64 - half;
            let s = sample_zero(raster, x as f64 + 0.5 + dx * t, y as f64 + 0.5 + dy * t);
            for c in 0..4 { acc[c] += s[c]; }
        }
        let i = ((y * w + x) * 4) as usize;
        let alpha = (acc[3] / steps as f32).round().clamp(0.0, 255.0);
        out[i + 3] = alpha as u8;
        for c in 0..3 { out[i + c] = (acc[c] / steps as f32).round().clamp(0.0, alpha) as u8; }
    }}
    Raster::from_premultiplied(w, h, out)
}

fn noise_hash(mut x: u32) -> u32 {
    x ^= x >> 16; x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15; x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}
fn noise_unit(key: u32) -> f32 { (noise_hash(key) >> 8) as f32 * (1.0 / 16_777_216.0) }

/// `noise_add` from NoisePixels.c.
pub fn add_noise(raster: &Raster, amount: f64, gaussian: bool, monochromatic: bool, seed: u32) -> Raster {
    let spread = amount as f32 / 100.0 * 127.5;
    let width = raster.width;
    let mut data = raster.bytes().to_vec();
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let alpha = p[3] as f32;
        if alpha == 0.0 { continue; }
        let base = noise_hash(seed ^ noise_hash(i as u32));
        let _ = width;
        for c in 0..3 {
            let key = if monochromatic { base } else { base.wrapping_add(c as u32 * 0x9e37_79b9) };
            let n = if gaussian {
                // Box-Muller: two uniform values make one normally distributed one.
                let (u1, u2) = (noise_unit(key), noise_unit(key ^ 0x68e3_1da4));
                (-2.0 * (1.0 - u1).ln()).sqrt() * (6.2831853 * u2).cos() * spread * (2.0 / 3.0)
            } else {
                (noise_unit(key) * 2.0 - 1.0) * spread
            };
            let value = (p[c] as f32 * 255.0 / alpha + n).clamp(0.0, 255.0);
            p[c] = (value * alpha / 255.0).round() as u8;
        }
    }
    Raster::from_premultiplied(raster.width, raster.height, data)
}

/// `lens_distort` from LensPixels.c: `scale = 1 - k r^2 / halfDiagonal^2`, bilinear, transparent outside.
pub fn lens_distort(raster: &Raster, k: f64) -> Raster {
    if k == 0.0 { return raster.clone(); }
    let (w, h) = (raster.width, raster.height);
    let (cx, cy) = (w as f64 * 0.5, h as f64 * 0.5);
    let half_diagonal2 = cx * cx + cy * cy;
    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..h { for x in 0..w {
        let dx = x as f64 + 0.5 - cx; let dy = y as f64 + 0.5 - cy;
        let scale = 1.0 - k * (dx * dx + dy * dy) / half_diagonal2;
        let s = sample_zero(raster, cx + dx * scale, cy + dy * scale);
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { out[i + c] = s[c].round().clamp(0.0, 255.0) as u8; }
    }}
    Raster::from_premultiplied(w, h, out)
}

/// The filter at its own scale; callers reduce the raster and pass `params.scaled(factor)`.
pub fn apply_filter(raster: &Raster, params: &FilterParams) -> Raster {
    match params.normalized() {
        FilterParams::GaussianBlur { radius } => gaussian_blur(raster, radius),
        FilterParams::MotionBlur { angle, distance } => motion_blur(raster, angle, distance),
        FilterParams::AddNoise { amount, gaussian, monochromatic, seed } => add_noise(raster, amount, gaussian, monochromatic, seed),
        FilterParams::LensCorrection { distortion } => lens_distort(raster, distortion / 100.0 * LENS_STRENGTH),
    }
}
```

Note for the implementer: `sample_zero`'s indices are clamped per axis from the unclamped floor (LL-064), and out-of-range taps contribute zero rather than the edge colour. The blur's second pass clamps each premultiplied channel to the blurred alpha, which `rgba_clamp_premultiplied` does on macOS.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 5 in `filters.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): gaussian blur, motion blur, add noise and lens correction"
```

---

### Task 7: One adjustment applied two ways: whole rasters and single colours

**Files:**
- Create: `engine/src/adjust/prepared.rs`, `engine/src/adjust/apply.rs`
- Modify: `engine/src/adjust/mod.rs`, `engine/src/lib.rs`
- Test: `engine/tests/prepared.rs`

**Interfaces:**
- Produces: `PreparedAdjustment` with `prepare(&LayerAdjustment) -> PreparedAdjustment` and `color(&self, rgb: [f32; 3], at: Point) -> [f32; 3]` (straight colour in 0..1, straight colour out; `at` is the document point, which only Grain reads), `apply_adjustment(&Raster, &LayerAdjustment, origin: Point, units_per_pixel: f64, selection: Option<&GrayRaster>) -> Raster`, `blend_by_coverage(&Raster, &Raster, &GrayRaster) -> Raster`.
- Consumes: Tasks 2 to 6.

- [ ] **Step 1: Write the failing tests**

`engine/tests/prepared.rs`:
```rust
use compositor_engine::*;

/// A 16-step gray ramp plus three primaries, opaque.
fn ramp() -> Raster {
    let mut data = Vec::new();
    for i in 0..16u32 { let v = (i * 17) as u8; data.extend_from_slice(&[v, v, v, 255]); }
    data.extend_from_slice(&[255, 0, 0, 255]); data.extend_from_slice(&[0, 255, 0, 255]); data.extend_from_slice(&[0, 0, 255, 255]);
    data.extend_from_slice(&[40, 90, 200, 255]);
    Raster::from_premultiplied(20, 1, data)
}

fn each_kind() -> Vec<LayerAdjustment> {
    let mut levels = LayerAdjustment::new(AdjustmentKind::Levels);
    levels.levels.ranges[0] = LevelRange { black: 20.0, gamma: 1.4, white: 230.0, output_black: 10.0, output_white: 250.0 };
    let mut curves = LayerAdjustment::new(AdjustmentKind::Curves);
    curves.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 10.0 }, CurvePoint { x: 128.0, y: 190.0 }, CurvePoint { x: 255.0, y: 245.0 }];
    let mut exposure = LayerAdjustment::new(AdjustmentKind::Exposure);
    exposure.exposure_settings = Some(ExposureSettings { exposure: 0.8, offset: 0.05, gamma: 1.3 });
    let mut gradient = LayerAdjustment::new(AdjustmentKind::GradientMap);
    gradient.gradient_map_settings = Some(GradientMapSettings { shadows: AdjustmentColor { red: 0.1, green: 0.0, blue: 0.4 }, highlights: AdjustmentColor { red: 1.0, green: 0.9, blue: 0.2 }, reversed: false });
    let mut hsv = LayerAdjustment::new(AdjustmentKind::Hsv);
    let mut settings = HueSaturationSettings::new(35.0, 20.0, -10.0, false, ColorRange::Master);
    settings.adjustments.insert(ColorRange::Blues, RangeAdjustment { hue: -20.0, saturation: 40.0, lightness: 0.0 });
    hsv.hsv_settings = Some(settings);
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 50.0, size: 2.0, roughness: 30.0, seed: 11 });
    vec![levels, curves, exposure, gradient, hsv, grain]
}

#[test]
fn the_single_color_path_matches_the_whole_raster_kernel() {
    let source = ramp();
    for adjustment in each_kind() {
        let whole = apply_adjustment(&source, &adjustment, Point { x: 0.0, y: 0.0 }, 1.0, None);
        let prepared = PreparedAdjustment::prepare(&adjustment);
        for i in 0..source.width {
            let p = source.pixel(i, 0);
            let rgb = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
            let at = Point { x: i as f64 + 0.5, y: 0.5 };
            let out = prepared.color(rgb, at);
            let expected = whole.pixel(i, 0);
            for c in 0..3 {
                let got = (out[c] * 255.0).round() as i64;
                assert!((got - expected[c] as i64).abs() <= 1, "{:?} pixel {i} channel {c}: {got} vs {}", adjustment.kind, expected[c]);
            }
        }
    }
}

#[test]
fn an_identity_adjustment_is_an_exact_no_op_and_coverage_blends() {
    let source = ramp();
    for kind in [AdjustmentKind::Levels, AdjustmentKind::Curves, AdjustmentKind::Exposure, AdjustmentKind::Hsv] {
        let a = LayerAdjustment::new(kind);
        assert_eq!(apply_adjustment(&source, &a, Point { x: 0.0, y: 0.0 }, 1.0, None).bytes(), source.bytes(), "{kind:?}");
    }
    let mut white = LayerAdjustment::new(AdjustmentKind::Curves);
    white.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    let adjusted = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, None);
    let coverage = GrayRaster::from_bytes(20, 1, (0..20).map(|i| if i < 10 { 255 } else { 0 }).collect());
    let blended = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, Some(&coverage));
    assert_eq!(blended.pixel(0, 0), adjusted.pixel(0, 0), "full coverage takes the adjusted colour");
    assert_eq!(blended.pixel(19, 0), source.pixel(19, 0), "no coverage leaves the original");
    let half = GrayRaster::from_bytes(20, 1, vec![128; 20]);
    let mixed = apply_adjustment(&source, &white, Point { x: 0.0, y: 0.0 }, 1.0, Some(&half));
    let expected = (source.pixel(5, 0)[0] as i64 + adjusted.pixel(5, 0)[0] as i64) / 2;
    assert!((mixed.pixel(5, 0)[0] as i64 - expected).abs() <= 1);
}

#[test]
fn grain_reads_the_document_point_so_two_tiles_agree() {
    let mut grain = LayerAdjustment::new(AdjustmentKind::Grain);
    grain.grain_settings = Some(GrainSettings { amount: 80.0, size: 3.0, roughness: 20.0, seed: 5 });
    let prepared = PreparedAdjustment::prepare(&grain);
    let rgb = [0.5, 0.5, 0.5];
    let a = prepared.color(rgb, Point { x: 12.5, y: 7.5 });
    assert_eq!(a, prepared.color(rgb, Point { x: 12.5, y: 7.5 }));
    assert_ne!(a, prepared.color(rgb, Point { x: 40.5, y: 7.5 }));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test prepared`
Expected: compile errors.

- [ ] **Step 3: Implement prepared.rs**

`engine/src/adjust/prepared.rs`:
```rust
use crate::*;

/// An adjustment with its tables built, able to map one straight colour at a time. Both
/// compositors use this for adjustment layers; the whole-raster kernels in `apply.rs` use the
/// same tables for destructive edits.
#[derive(Clone, Debug)]
pub enum PreparedAdjustment {
    /// 768 outputs in 0..1: R, G, B tables (levels, curves, exposure).
    Tables(Vec<f32>),
    /// 768 bytes: 256 RGB triples.
    GradientMap(Vec<u8>),
    Hsv { settings: HueSaturationSettings, response: Vec<[f64; 3]> },
    Grain { settings: GrainSettings },
    Identity,
}

impl PreparedAdjustment {
    pub fn prepare(a: &LayerAdjustment) -> PreparedAdjustment {
        match a.kind {
            AdjustmentKind::Levels => PreparedAdjustment::Tables(levels_tables(&a.levels)),
            AdjustmentKind::Curves => PreparedAdjustment::Tables(curves_tables(&a.curves)),
            AdjustmentKind::Exposure => PreparedAdjustment::Tables(exposure_table(&a.exposure())),
            AdjustmentKind::GradientMap => PreparedAdjustment::GradientMap(gradient_map_table(&a.gradient_map())),
            AdjustmentKind::Hsv => {
                let settings = a.resolved_hsv();
                if settings.is_identity() { return PreparedAdjustment::Identity; }
                let response = hue_response(&settings);
                PreparedAdjustment::Hsv { settings, response }
            }
            AdjustmentKind::Grain => {
                let settings = a.grain().normalized();
                if !(settings.amount > 0.0) { return PreparedAdjustment::Identity; }
                PreparedAdjustment::Grain { settings }
            }
        }
    }

    /// One straight colour (0..1) at a document point. Only Grain reads `at`.
    pub fn color(&self, rgb: [f32; 3], at: Point) -> [f32; 3] {
        match self {
            PreparedAdjustment::Identity => rgb,
            PreparedAdjustment::Tables(tables) => {
                let mut out = [0f32; 3];
                for c in 0..3 {
                    let x = (rgb[c] * 255.0).clamp(0.0, 255.0);
                    let lo = x as usize; let hi = if lo < 255 { lo + 1 } else { 255 };
                    let t = &tables[c * 256..c * 256 + 256];
                    out[c] = t[lo] + (t[hi] - t[lo]) * (x - lo as f32);
                }
                out
            }
            PreparedAdjustment::GradientMap(table) => {
                // The same integer luma the kernel computes, so both paths pick the same entry.
                let r = (rgb[0] * 255.0).round().clamp(0.0, 255.0) as u32;
                let g = (rgb[1] * 255.0).round().clamp(0.0, 255.0) as u32;
                let b = (rgb[2] * 255.0).round().clamp(0.0, 255.0) as u32;
                let level = (((2126 * r + 7152 * g + 722 * b + 5000) / 10000).min(255)) as usize;
                [table[level * 3] as f32 / 255.0, table[level * 3 + 1] as f32 / 255.0, table[level * 3 + 2] as f32 / 255.0]
            }
            PreparedAdjustment::Hsv { settings, response } => {
                let out = adjust_rgb([rgb[0] as f64, rgb[1] as f64, rgb[2] as f64], settings, response);
                [out[0] as f32, out[1] as f32, out[2] as f32]
            }
            PreparedAdjustment::Grain { settings } => {
                let noise = grain_noise(at.x, at.y, settings.size, settings.roughness, settings.seed);
                let level = (0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]).min(1.0);
                let delta = noise * grain_strength(settings.amount) * grain_weight(level);
                let mut out = [0f32; 3];
                for c in 0..3 { out[c] = (rgb[c] * 255.0 + delta).clamp(0.0, 255.0) / 255.0; }
                out
            }
        }
    }
}
```

- [ ] **Step 4: Implement apply.rs**

`engine/src/adjust/apply.rs`:
```rust
use crate::*;

/// `coverage * adjusted + (1 - coverage) * original`, both premultiplied and the same size.
pub fn blend_by_coverage(adjusted: &Raster, original: &Raster, coverage: &GrayRaster) -> Raster {
    let mut data = adjusted.bytes().to_vec();
    let base = original.bytes();
    for (i, p) in data.chunks_exact_mut(4).enumerate() {
        let k = coverage.bytes()[i] as u32;
        if k == 255 { continue; }
        for c in 0..4 {
            let a = p[c] as u32; let b = base[i * 4 + c] as u32;
            p[c] = ((a * k + b * (255 - k) + 127) / 255) as u8;
        }
    }
    Raster::from_premultiplied(adjusted.width, adjusted.height, data)
}

/// A whole raster through one adjustment. `origin` and `units_per_pixel` place the raster in
/// document space (Grain reads them); `selection` limits the change to its coverage.
pub fn apply_adjustment(raster: &Raster, a: &LayerAdjustment, origin: Point, units_per_pixel: f64, selection: Option<&GrayRaster>) -> Raster {
    let adjusted = match a.kind {
        AdjustmentKind::Levels => apply_tables(raster, &levels_tables(&a.levels)),
        AdjustmentKind::Curves => apply_tables(raster, &curves_tables(&a.curves)),
        AdjustmentKind::Exposure => apply_tables(raster, &exposure_table(&a.exposure())),
        AdjustmentKind::GradientMap => apply_gradient_map(raster, &gradient_map_table(&a.gradient_map())),
        AdjustmentKind::Hsv => apply_hsv(raster, &a.resolved_hsv()),
        AdjustmentKind::Grain => apply_grain(raster, &a.grain(), origin, units_per_pixel),
    };
    match selection { Some(coverage) => blend_by_coverage(&adjusted, raster, coverage), None => adjusted }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 3 in `prepared.rs`.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): one adjustment applied as a whole raster or a single colour"
```

---

### Task 8: Layer operations: apply, invert, filter with growth and trim, adjustment layers

**Files:**
- Create: `engine/src/ops/adjust.rs`
- Modify: `engine/src/ops/mod.rs` (`pub mod adjust;`), `engine/src/ops/hierarchy.rs` (`can_toggle_clipping` refuses an adjustment layer as a source)
- Test: `engine/tests/adjust_ops.rs`

**Interfaces:**
- Produces: `ops::adjust::{apply_adjustment_to_layer(doc, id, &LayerAdjustment), invert_layer(doc, id, mask: bool), apply_filter(doc, id, &FilterParams), add_adjustment_layer(doc, kind, seed: u32, gradient: Option<([f64;3],[f64;3])>) -> Uuid, set_adjustment(doc, id, &LayerAdjustment), grown(&Raster, &LayerTransform, margin: f64) -> Option<(Raster, LayerTransform)>, trimmed(&Raster, &LayerTransform) -> (Raster, LayerTransform)}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/adjust_ops.rs`:
```rust
use compositor_engine::ops::{adjust, hierarchy};
use compositor_engine::*;

fn doc_with_layer() -> (Document, uuid::Uuid) {
    let mut d = Document::new(40, 40);
    let mut data = vec![0u8; 20 * 20 * 4];
    for y in 0..20 { for x in 0..10 { let i = ((y * 20 + x) * 4) as usize; data[i..i + 4].copy_from_slice(&[255, 255, 255, 255]); } }
    let l = Layer::with_pixels("Half", Raster::from_premultiplied(20, 20, data), Point { x: 10.0, y: 10.0 });
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}

#[test]
fn an_adjustment_replaces_the_layers_pixels_in_place() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().pixels_revision;
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::apply_adjustment_to_layer(&mut d, id, &a).unwrap();
    let l = d.layer(id).unwrap();
    assert_eq!(l.pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255], "white becomes black");
    assert_eq!(l.transform.size, Size { width: 20.0, height: 20.0 }, "the layer keeps its place");
    assert!(l.pixels_revision > before);
    let folder = hierarchy::add_group(&mut d).unwrap();
    assert!(adjust::apply_adjustment_to_layer(&mut d, folder, &a).is_err(), "folders have no pixels to adjust");
}

#[test]
fn invert_works_on_pixels_and_on_a_mask() {
    let (mut d, id) = doc_with_layer();
    adjust::invert_layer(&mut d, id, false).unwrap();
    assert_eq!(d.layer(id).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255]);
    assert!(adjust::invert_layer(&mut d, id, true).is_err(), "no mask to invert");
    compositor_engine::ops::masks::add_mask(&mut d, id, true).unwrap();
    adjust::invert_layer(&mut d, id, true).unwrap();
    let mask = d.layer(id).unwrap().mask.as_ref().unwrap();
    assert_eq!(mask.pixels.bytes()[0], 0, "a revealing mask inverts to hiding");
}

#[test]
fn a_blur_grows_the_layer_spreads_past_its_edge_and_trims_back() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().transform;
    adjust::apply_filter(&mut d, id, &FilterParams::GaussianBlur { radius: 3.0 }).unwrap();
    let l = d.layer(id).unwrap();
    assert!(l.transform.origin.x < before.origin.x, "the blur spreads past the old left edge: {:?}", l.transform);
    assert!(l.transform.size.width > before.size.width && l.transform.size.height > before.size.height);
    let raster = l.pixels.as_ref().unwrap();
    assert!(raster.pixel(0, raster.height / 2)[3] > 0, "trimmed to where the blur actually reaches");
    let middle = raster.pixel(raster.width / 2, raster.height / 2)[3];
    assert!(middle > 20 && middle < 235, "the hard edge is soft");
    // The document area the layer covers grew by the same amount on each side.
    let grew_left = before.origin.x - l.transform.origin.x;
    let grew_right = (l.transform.origin.x + l.transform.size.width) - (before.origin.x + before.size.width);
    assert!((grew_left - grew_right).abs() < 1.5, "{grew_left} vs {grew_right}");
}

#[test]
fn a_blur_carries_a_covering_mask_onto_the_new_grid() {
    let (mut d, id) = doc_with_layer();
    d.layer_mut(id).unwrap().set_mask(Some(Mask {
        pixels: GrayRaster::from_bytes(20, 20, (0..400).map(|i| if (i % 20) < 10 { 255 } else { 0 }).collect()),
        enabled: true, placement: None, linked: None }));
    let before = d.layer(id).unwrap().transform;
    adjust::apply_filter(&mut d, id, &FilterParams::GaussianBlur { radius: 2.0 }).unwrap();
    let l = d.layer(id).unwrap();
    let mask = l.mask.as_ref().unwrap();
    assert!(mask.placement.is_none(), "still a covering mask");
    // The mask's white half still covers the same document area: sample its middle-left and middle-right.
    let to_doc = l.transform.pixel_to_document(mask.pixels.width, mask.pixels.height);
    let mut left_white = 0; let mut right_white = 0;
    for x in 0..mask.pixels.width {
        let p = to_doc.apply(Point { x: x as f64 + 0.5, y: mask.pixels.height as f64 / 2.0 });
        let value = mask.pixels.bytes()[(mask.pixels.height / 2 * mask.pixels.width + x) as usize];
        if p.x < 20.0 && value > 200 { left_white += 1; }
        if p.x > 20.0 && value > 200 { right_white += 1; }
    }
    assert!(left_white > 5 && right_white == 0, "left {left_white}, right {right_white}");
}

#[test]
fn a_filter_that_does_not_spread_keeps_the_layers_grid() {
    let (mut d, id) = doc_with_layer();
    let before = d.layer(id).unwrap().transform;
    adjust::apply_filter(&mut d, id, &FilterParams::AddNoise { amount: 20.0, gaussian: false, monochromatic: false, seed: 3 }).unwrap();
    let l = d.layer(id).unwrap();
    assert_eq!(l.transform, before);
    assert_eq!(l.pixels.as_ref().unwrap().width, 20);
}

#[test]
fn adjustment_layers_are_created_above_the_active_layer_and_edited_in_place() {
    let (mut d, id) = doc_with_layer();
    let new = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Curves, 0, None).unwrap();
    assert_eq!(d.active_layer_id, Some(new));
    assert_eq!(d.index_of(new).unwrap(), d.index_of(id).unwrap() + 1);
    let layer = d.layer(new).unwrap();
    assert!(layer.is_adjustment() && layer.pixels.is_none() && layer.name == "Curves");
    assert_eq!(layer.transform.size, d.size());
    let mut edited = layer.extra.adjustment.clone().unwrap();
    edited.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    adjust::set_adjustment(&mut d, new, &edited).unwrap();
    assert_eq!(d.layer(new).unwrap().extra.adjustment.as_ref().unwrap().curves, edited.curves);
    let mut broken = edited.clone();
    broken.curves.channels[0] = vec![CurvePoint { x: 5.0, y: 0.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    assert!(adjust::set_adjustment(&mut d, new, &broken).is_err(), "invalid settings are refused");
    assert!(adjust::set_adjustment(&mut d, id, &edited).is_err(), "a pixel layer is not an adjustment layer");
    // A Gradient Map takes the palette's colours; each Grain layer gets its own pattern.
    let gradient = adjust::add_adjustment_layer(&mut d, AdjustmentKind::GradientMap, 0, Some(([1.0, 0.0, 0.0], [0.0, 0.0, 1.0]))).unwrap();
    let g = d.layer(gradient).unwrap().extra.adjustment.as_ref().unwrap().gradient_map();
    assert_eq!(g.shadows, AdjustmentColor { red: 1.0, green: 0.0, blue: 0.0 });
    let grain = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Grain, 4242, None).unwrap();
    assert_eq!(d.layer(grain).unwrap().extra.adjustment.as_ref().unwrap().grain().seed, 4242);
}

#[test]
fn an_adjustment_layer_is_never_a_clipping_source() {
    let (mut d, id) = doc_with_layer();
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    // The adjustment can clip to the pixel layer below it.
    assert!(hierarchy::can_toggle_clipping(&d, a));
    hierarchy::toggle_clipping(&mut d, a).unwrap();
    assert_eq!(d.layer(a).unwrap().mask_source_id, Some(id));
    // A layer above the adjustment cannot clip to the adjustment.
    let mut top = Layer::with_pixels("Top", Raster::new_transparent(4, 4), Point { x: 0.0, y: 0.0 });
    let top_id = top.id; top.parent_id = None; d.layers.push(top);
    assert!(!hierarchy::can_toggle_clipping(&d, top_id), "an adjustment layer supplies no coverage");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test adjust_ops`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/ops/adjust.rs`:
```rust
use crate::*;
use uuid::Uuid;

fn pixel_layer(doc: &Document, id: Uuid) -> Result<&Layer, CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if layer.is_group() { return Err(CommandError::Argument("folders have no pixels".into())); }
    if layer.pixels.is_none() { return Err(CommandError::Argument("the layer has no pixels".into())); }
    Ok(layer)
}

/// Where the layer's pixel grid sits in document space: the origin of pixel (0, 0) and how many
/// document units one pixel covers. Grain is fixed in document space, so it reads these.
fn placement(layer: &Layer) -> (Point, f64) {
    let (w, _h) = layer.pixels.as_ref().map_or((1, 1), |p| (p.width, p.height));
    (layer.transform.origin, layer.transform.size.width / w.max(1) as f64)
}

pub fn apply_adjustment_to_layer(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
    let layer = pixel_layer(doc, id)?;
    let (origin, units) = placement(layer);
    let raster = layer.pixels.as_ref().unwrap();
    let adjusted = adjust::apply::apply_adjustment(raster, a, origin, units, None);
    doc.layer_mut(id).unwrap().set_pixels(Some(adjusted));
    Ok(())
}

pub fn invert_layer(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    if mask {
        let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
        let Some(m) = &layer.mask else { return Err(CommandError::Argument("the layer has no mask".into())); };
        let inverted = adjust::tonal::invert_gray(&m.pixels);
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = inverted;
        return Ok(());
    }
    let layer = pixel_layer(doc, id)?;
    let inverted = adjust::tonal::invert_raster(layer.pixels.as_ref().unwrap());
    doc.layer_mut(id).unwrap().set_pixels(Some(inverted));
    Ok(())
}

/// The raster padded by `margin` pixels on every side, with the transform that keeps the old
/// pixels exactly where they were.
pub fn grown(raster: &Raster, transform: &LayerTransform, margin: f64) -> Option<(Raster, LayerTransform)> {
    let m = margin.ceil().max(0.0) as u32;
    if m == 0 { return None; }
    let (w, h) = (raster.width + 2 * m, raster.height + 2 * m);
    if w as i64 > MAX_SIDE || h as i64 > MAX_SIDE || (w as u64) * (h as u64) > MAX_PIXELS { return None; }
    let mut data = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..raster.height {
        let src = ((y * raster.width) * 4) as usize;
        let dst = (((y + m) * w + m) * 4) as usize;
        data[dst..dst + (raster.width * 4) as usize].copy_from_slice(&raster.bytes()[src..src + (raster.width * 4) as usize]);
    }
    Some((Raster::from_premultiplied(w, h, data), placed_like(transform, raster.width, raster.height, w, h, m as f64, m as f64)))
}

/// The transform for a `new_w` x `new_h` grid whose old grid sits at (`offset_x`, `offset_y`),
/// keeping every old pixel over the same document point.
fn placed_like(transform: &LayerTransform, old_w: u32, old_h: u32, new_w: u32, new_h: u32, offset_x: f64, offset_y: f64) -> LayerTransform {
    let sx = transform.size.width / old_w as f64;
    let sy = transform.size.height / old_h as f64;
    let mut result = *transform;
    result.size = Size { width: new_w as f64 * sx, height: new_h as f64 * sy };
    // The old grid's centre in document space stays put; the new centre is offset from it.
    let to_document = transform.pixel_to_document(old_w, old_h);
    let middle = to_document.apply(Point { x: new_w as f64 / 2.0 - offset_x, y: new_h as f64 / 2.0 - offset_y });
    result.origin = Point { x: middle.x - result.size.width / 2.0, y: middle.y - result.size.height / 2.0 };
    result
}

/// The raster cropped to the pixels that are actually there, with the transform that keeps them in place.
pub fn trimmed(raster: &Raster, transform: &LayerTransform) -> (Raster, LayerTransform) {
    let full = (0, 0, raster.width, raster.height);
    match compositor::alpha_bounds(raster) {
        Some(crop) if crop != full => {
            let (w, h) = (crop.2 - crop.0, crop.3 - crop.1);
            let cropped = raster.cropped(crop.0, crop.1, w, h);
            let placed = placed_like(transform, raster.width, raster.height, w, h, -(crop.0 as f64), -(crop.1 as f64));
            (cropped, placed)
        }
        _ => (raster.clone(), *transform),
    }
}

/// A covering mask resampled onto the grid `new` places, its background beyond the old edge.
fn carry_mask(mask: &Mask, old: &LayerTransform, new: &LayerTransform) -> GrayRaster {
    let ratio_w = new.size.width / old.size.width;
    let ratio_h = new.size.height / old.size.height;
    let w = ((mask.pixels.width as f64 * ratio_w).round() as u32).clamp(1, MAX_SIDE as u32);
    let h = ((mask.pixels.height as f64 * ratio_h).round() as u32).clamp(1, MAX_SIDE as u32);
    let background = mask.background();
    let to_document = new.pixel_to_document(w, h);
    let from_document = old.pixel_to_document(mask.pixels.width, mask.pixels.height).invert();
    let mut data = vec![background; (w as usize) * (h as usize)];
    if let Some(inverse) = from_document {
        for y in 0..h { for x in 0..w {
            let p = inverse.apply(to_document.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 }));
            if p.x < 0.0 || p.y < 0.0 || p.x >= mask.pixels.width as f64 || p.y >= mask.pixels.height as f64 { continue; }
            data[(y * w + x) as usize] = mask.pixels.bytes()[(p.y as u32 * mask.pixels.width + p.x as u32) as usize];
        }}
    }
    GrayRaster::from_bytes(w, h, data)
}

/// One filter on a layer: a blur is given room to spread, run, then cut back to what it left.
pub fn apply_filter(doc: &mut Document, id: Uuid, params: &FilterParams) -> Result<(), CommandError> {
    let params = params.normalized();
    if params.is_identity() { return Ok(()); }
    let layer = pixel_layer(doc, id)?.clone();
    let raster = layer.pixels.as_ref().unwrap();
    let (source, placed) = match params.spreads() {
        true => grown(raster, &layer.transform, params.margin()).ok_or(CommandError::Project(ProjectError::TooLarge))?,
        false => (raster.clone(), layer.transform),
    };
    let filtered = adjust::filters::apply_filter(&source, &params);
    let (result, transform) = if params.spreads() { trimmed(&filtered, &placed) } else { (filtered, placed) };
    let mask = match &layer.mask {
        Some(m) if m.placement.is_none() && !m.is_uniform() && transform != layer.transform => {
            Some(Mask { pixels: carry_mask(m, &layer.transform, &transform), ..m.clone() })
        }
        other => other.clone(),
    };
    let target = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    target.set_pixels(Some(result));
    target.transform = transform;
    if mask != layer.mask { target.set_mask(mask); }
    Ok(())
}

/// A new adjustment layer above the active layer (inside it when a folder is active).
pub fn add_adjustment_layer(doc: &mut Document, kind: AdjustmentKind, seed: u32, gradient: Option<([f64; 3], [f64; 3])>) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut adjustment = LayerAdjustment::new(kind);
    match kind {
        AdjustmentKind::GradientMap => {
            let (shadows, highlights) = gradient.unwrap_or(([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]));
            adjustment.gradient_map_settings = Some(adjust::tonal::gradient_map_from(shadows, highlights));
        }
        AdjustmentKind::Grain => adjustment.grain_settings = Some(GrainSettings { seed, ..GrainSettings::default() }),
        _ => {}
    }
    let mut layer = Layer::blank(kind.name(), doc.size());
    layer.extra.adjustment = Some(adjustment);
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    let id = layer.id;
    doc.layers.insert(insertion, layer);
    doc.active_layer_id = Some(id);
    Ok(id)
}

pub fn set_adjustment(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if !layer.is_adjustment() { return Err(CommandError::Argument("not an adjustment layer".into())); }
    if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
    doc.layer_mut(id).unwrap().extra.adjustment = Some(a.clone());
    Ok(())
}
```

`engine/src/ops/hierarchy.rs`: in `can_toggle_clipping`, refuse a base that is an adjustment layer (it supplies no coverage). Find the check that the layer below exists and add `&& !below.is_adjustment()`; the same rule goes in `link_mask`'s validation (`source` must not be an adjustment layer).

`engine/src/ops/mod.rs`: `pub mod adjust;`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 7 in `adjust_ops.rs`; the Phase 2 hierarchy tests still pass.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): layer adjustments, invert, filters with growth and trim, adjustment layers"
```

---

### Task 9: Adjustment layers in the render plan and the CPU compositor

**Files:**
- Modify: `engine/src/plan.rs`, `engine/src/compositor.rs`
- Test: `engine/tests/adjust_plan.rs`

**Interfaces:**
- Produces: `LayerDraw.adjustment: Option<LayerAdjustment>` (JSON `adjustment`), `PreviewEdit::Adjustment { id, adjustment }` (JSON `{"kind":"adjustment","id":...,"adjustment":{...}}`), `plan::displayed_adjustment(&Layer, Option<&PreviewEdit>) -> Option<LayerAdjustment>`.
- Rules: an adjustment layer is never a stack base, never in `sources`, and is skipped entirely when it is clipped but did not join its base's stack.

- [ ] **Step 1: Write the failing tests**

`engine/tests/adjust_plan.rs`:
```rust
use compositor_engine::ops::{adjust, hierarchy};
use compositor_engine::*;

fn canvas(color: [u8; 4]) -> (Document, uuid::Uuid) {
    let mut d = Document::new(2, 2);
    let l = Layer::with_pixels("Base", Raster::from_premultiplied(2, 2, color.repeat(4)), Point { x: 0.0, y: 0.0 });
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2) }

#[test]
fn a_global_adjustment_changes_what_is_below_it_but_not_what_is_above() {
    let (mut d, base) = canvas([255, 255, 255, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    assert_eq!(full(&d).bytes(), &[0, 0, 0, 255].repeat(4), "white is mapped to black");
    // A layer above the adjustment is untouched.
    let mut top = Layer::with_pixels("Top", Raster::from_premultiplied(1, 1, vec![255, 0, 0, 255]), Point { x: 0.0, y: 0.0 });
    top.transform.sampling = Sampling::Nearest;
    d.layers.push(top);
    assert_eq!(full(&d).pixel(0, 0), [255, 0, 0, 255]);
    assert_eq!(full(&d).pixel(1, 1), [0, 0, 0, 255]);
    // Hiding the adjustment brings the original back.
    d.layer_mut(a).unwrap().visible = false;
    assert_eq!(full(&d).pixel(1, 1), [255, 255, 255, 255]);
    let _ = base;
}

#[test]
fn opacity_and_a_mask_limit_an_adjustment_without_touching_the_pixels_below() {
    let (mut d, base) = canvas([255, 255, 255, 255]);
    let original = d.layer(base).unwrap().pixels.clone();
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    d.layer_mut(a).unwrap().opacity = 0.5;
    let half = full(&d).pixel(0, 0);
    assert!((half[0] as i64 - 128).abs() <= 2 && half[3] == 255, "{half:?}");
    d.layer_mut(a).unwrap().opacity = 1.0;
    compositor_engine::ops::masks::add_mask(&mut d, a, false).unwrap();
    assert_eq!(full(&d).pixel(0, 0), [255, 255, 255, 255], "a hiding mask blocks the adjustment entirely");
    assert_eq!(d.layer(base).unwrap().pixels, original, "the layer below is never rewritten");
}

#[test]
fn a_clipped_adjustment_changes_only_its_base() {
    let mut d = Document::new(2, 1);
    let bottom = Layer::with_pixels("Bottom", Raster::from_premultiplied(2, 1, vec![0, 0, 255, 255, 0, 0, 255, 255]), Point { x: 0.0, y: 0.0 });
    let bottom_id = bottom.id;
    let mut middle = Layer::with_pixels("Middle", Raster::from_premultiplied(1, 1, vec![0, 255, 0, 255]), Point { x: 0.0, y: 0.0 });
    middle.transform.sampling = Sampling::Nearest;
    let middle_id = middle.id;
    d.layers.push(bottom); d.layers.push(middle);
    d.active_layer_id = Some(middle_id);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Curves, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Curves);
    settings.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 0.0 }];
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    hierarchy::toggle_clipping(&mut d, a).unwrap();
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [255, 0, 255, 255], "green inverts to magenta where the base covers");
    assert_eq!(out.pixel(1, 0), [0, 0, 255, 255], "the layer beside the base is untouched");
    // With the base hidden the clipped adjustment draws nothing at all.
    d.layer_mut(middle_id).unwrap().visible = false;
    assert_eq!(full(&d).pixel(0, 0), [0, 0, 255, 255]);
    let _ = bottom_id;
}

#[test]
fn an_adjustment_in_a_folder_reaches_beneath_it_and_a_folder_mask_limits_it() {
    // macOS's FolderMaskClip behaviour: a folder limits an adjustment through its mask,
    // it does not isolate its contents from one.
    let mut d = Document::new(2, 1);
    let outside = Layer::with_pixels("Outside", Raster::from_premultiplied(1, 1, vec![255, 255, 255, 255]), Point { x: 1.0, y: 0.0 });
    let outside_id = outside.id;
    let inside = Layer::with_pixels("Inside", Raster::from_premultiplied(1, 1, vec![255, 255, 255, 255]), Point { x: 0.0, y: 0.0 });
    let inside_id = inside.id;
    d.layers.push(outside); d.layers.push(inside);
    let folder = hierarchy::group_layers(&mut d, &[inside_id]).unwrap();
    d.active_layer_id = Some(inside_id);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    assert_eq!(d.layer(a).unwrap().parent_id, Some(folder), "created inside the folder");
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [0, 0, 0, 255], "the layer in the folder is adjusted");
    assert_eq!(out.pixel(1, 0), [0, 0, 0, 255], "an unmasked folder does not hold the adjustment in");
    // A folder mask is what scopes it: white over the left half, black over the right.
    d.layer_mut(folder).unwrap().mask = Some(Mask {
        pixels: GrayRaster::from_bytes(2, 1, vec![255, 0]),
        enabled: true, placement: None, linked: None,
    });
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [0, 0, 0, 255], "still adjusted where the folder mask is white");
    assert_eq!(out.pixel(1, 0), [255, 255, 255, 255], "the adjustment does not reach past the folder mask");
    let _ = outside_id;
}

#[test]
fn the_plan_carries_the_adjustment_and_a_preview_edit_replaces_it() {
    let (mut d, _) = canvas([128, 128, 128, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Exposure, 0, None).unwrap();
    let plan = render_plan(&d, None);
    assert_eq!(plan.nodes.len(), 2);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("{:?}", plan.nodes[1]) };
    assert_eq!(draw.id, a);
    assert_eq!(draw.adjustment.as_ref().unwrap().kind, AdjustmentKind::Exposure);
    assert!(plan.sources.is_empty());
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains(r#""adjustment":{"#) && json.contains(r#""kind":"Exposure""#));
    // A preview edit substitutes the settings without touching the document.
    let mut preview = LayerAdjustment::new(AdjustmentKind::Exposure);
    preview.exposure_settings = Some(ExposureSettings { exposure: 2.0, ..Default::default() });
    let edit = PreviewEdit::Adjustment { id: a, adjustment: preview.clone() };
    let previewed = render_plan(&d, Some(&edit));
    let PlanNode::Layer { draw } = &previewed.nodes[1] else { panic!() };
    assert_eq!(draw.adjustment.as_ref().unwrap().exposure().exposure, 2.0);
    let out = composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2);
    assert!(out.pixel(0, 0)[0] > 200, "the preview brightens: {:?}", out.pixel(0, 0));
    assert_eq!(d.layer(a).unwrap().extra.adjustment.as_ref().unwrap().exposure().exposure, 0.0, "the document is unchanged");
    let parsed: PreviewEdit = serde_json::from_str(&format!(r#"{{"kind":"adjustment","id":"{}","adjustment":{}}}"#, ids::upper_string(&a), serde_json::to_string(&preview).unwrap())).unwrap();
    assert_eq!(parsed, edit);
}

#[test]
fn an_adjustment_blends_with_its_own_blend_mode() {
    let (mut d, _) = canvas([200, 100, 50, 255]);
    let a = adjust::add_adjustment_layer(&mut d, AdjustmentKind::Levels, 0, None).unwrap();
    let mut settings = LayerAdjustment::new(AdjustmentKind::Levels);
    settings.levels.ranges[0] = LevelRange { output_black: 255.0, output_white: 255.0, ..LevelRange::default() };
    adjust::set_adjustment(&mut d, a, &settings).unwrap();
    d.layer_mut(a).unwrap().blend_mode = BlendMode::Multiply;
    // White adjusted, multiplied back over the original, leaves the original.
    let out = full(&d);
    assert_eq!(out.pixel(0, 0), [200, 100, 50, 255]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test adjust_plan`
Expected: compile errors (`LayerDraw.adjustment` missing).

- [ ] **Step 3: Extend the plan**

`engine/src/plan.rs`:
- Add to `LayerDraw`, after `clip`:
```rust
    #[serde(skip_serializing_if = "Option::is_none")] pub adjustment: Option<LayerAdjustment>,
```
- Add to `PreviewEdit`:
```rust
    Adjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
```
- Add the accessor and use it in `draw_for`:
```rust
/// The layer's adjustment as the pending edit shows it.
pub fn displayed_adjustment(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<LayerAdjustment> {
    match edit {
        Some(PreviewEdit::Adjustment { id, adjustment }) if *id == layer.id => Some(adjustment.clone()),
        _ => layer.extra.adjustment.clone(),
    }
}
```
In `draw_for`, set `adjustment: displayed_adjustment(layer, edit)` in the returned `LayerDraw`; `render_layer` in `compositor.rs` builds a `LayerDraw` by hand and needs `adjustment: None` added.
- In `render_plan`, three rules:
```rust
    // An adjustment layer is never a stack base (it has no alpha of its own to share).
    for (index, base) in ids.iter().enumerate() {
        if source_of(*base).is_some() || by_id[base].is_adjustment() { continue; }
        ...
    }
```
```rust
    for id in &ids {
        if stacked.contains(id) { continue; }
        let layer = by_id[id];
        // A clipped adjustment renders only as part of its base's stack: with the base hidden or
        // out of reach there is nothing beneath it to adjust, as macOS's drawComposite does.
        if layer.is_adjustment() && layer.mask_source_id.is_some() { continue; }
        ...
    }
```
```rust
        if let Some(layer) = by_id.get(&id) {
            if layer.is_group || layer.is_adjustment() { continue; }   // never a clipping source
```
(the same `is_adjustment` guard goes in `ensure_source`).

- [ ] **Step 4: Apply adjustments in the CPU compositor**

`engine/src/compositor.rs`, at the top of `draw_layer`:
```rust
fn draw_layer(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, blend: BlendMode, use_clip: bool) {
    if draw.adjustment.is_some() { return adjust_target(doc, plan, target, draw, blend, use_clip); }
    let Some(raster) = doc.layer(draw.id).and_then(|l| l.pixels.as_ref()) else { return; };
    ...
```
and the new function beside it:
```rust
/// An adjustment layer: the colours already in the target, mapped where this layer's coverage
/// reaches. Nothing is sampled from the layer itself - it has no pixels - and the target's alpha
/// is kept, so a soft edge below stays exactly as soft (macOS's LiveMaskRenderer.adjust).
fn adjust_target(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, blend: BlendMode, use_clip: bool) {
    let Some(adjustment) = &draw.adjustment else { return; };
    let prepared = PreparedAdjustment::prepare(adjustment);
    let out_per_doc = target.w as f64 / target.region.width;
    let clip_sources = match (use_clip, draw.clip) {
        (true, Some(c)) => clip_source_rasters(doc, plan, c, out_per_doc),
        _ => SourceRasters::new(),
    };
    for oy in 0..target.h { for ox in 0..target.w {
        let i = ((oy * target.w + ox) * 4) as usize;
        let alpha = target.data[i + 3] as f32;
        if alpha <= 0.0 { continue; }
        let p = target.doc_point(ox, oy);
        let mut k = draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        if use_clip { if let Some(c) = draw.clip { k *= source_coverage_at(doc, plan, c, p, &clip_sources); } }
        if k <= 0.0 { continue; }
        let original = [target.data[i] as f32 / alpha, target.data[i + 1] as f32 / alpha, target.data[i + 2] as f32 / alpha];
        let mut adjusted = prepared.color(original, p);
        if blend != BlendMode::Normal { adjusted = blend_rgb(blend, original, adjusted); }
        for c in 0..3 {
            let mixed = original[c] + (adjusted[c].clamp(0.0, 1.0) - original[c]) * k;
            target.data[i + c] = (mixed * alpha).round().clamp(0.0, alpha) as u8;
        }
    }}
}
```
Notes for the implementer: `blend_rgb(mode, cb, cs)` in `blend.rs` takes the backdrop first; here the backdrop is the original colour and the source is the adjusted one, both fully opaque, which is what the Mac composites. An adjustment covers the whole target rather than its own rectangle (its transform is ignored, as on macOS, where an adjustment layer cannot be transformed at all). Inside `draw_stack` the children are drawn after the base has been made opaque, so an adjustment child maps exactly the stack's colours.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 6 in `adjust_plan.rs`; every Phase 2 plan and compositor test still passes.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): adjustment layers in the render plan and the CPU compositor"
```

---

### Task 10: Live previews, commands and the engine facade

**Files:**
- Create: `engine/src/preview.rs`
- Modify: `engine/src/lib.rs`, `engine/src/command.rs`, `engine/src/engine.rs`
- Test: `engine/tests/preview.rs`

**Interfaces:**
- Produces: `PreviewRequest` (serde tag `preview`: `Adjustment { layer, adjustment }`, `Filter { layer, params }`), `PixelPreview { layer, raster, transform, revision }`, `preview_limit(&PreviewRequest) -> u32`, `compute_preview(&Document, &PreviewRequest, revision) -> Option<PixelPreview>`.
- New commands: `ApplyAdjustment { id, adjustment }`, `InvertPixels { id, mask }`, `ApplyFilter { id, params }`, `AddAdjustmentLayer { kind, seed, shadows, highlights }`, `SetAdjustment { id, adjustment }`.
- New `Engine` methods: `set_preview(id, Option<PreviewRequest>) -> Result<Dirty, _>`, `layer_raster(id, layer, level) -> Result<Option<Raster>, _>`, `histogram(id, layer) -> Result<Vec<Vec<f64>>, _>`, `auto_levels(id, layer, LevelsAuto) -> Result<LevelsSettings, _>`, `levels_sampling(id, layer, &LevelsSettings, Point, LevelsSample) -> Result<LevelsSettings, _>`, `sample_layer_color(id, layer, Point) -> Result<Option<[f64; 3]>, _>`, `sample_color(id, Point) -> Result<Option<[f64; 3]>, _>`, `adjustment_source(id, layer) -> Result<Raster, _>`.
- `LayerState` gains `adjustment: Option<LayerAdjustment>` (JSON `adjustment`, omitted when absent).

- [ ] **Step 1: Write the failing tests**

`engine/tests/preview.rs`:
```rust
use compositor_engine::*;

fn seeded() -> (Engine, uuid::Uuid, uuid::Uuid) {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(8, 8, [128u8, 128, 128, 255].repeat(64)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Gray", Some(Point { x: 10.0, y: 10.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    (e, doc, layer)
}
fn middle(e: &Engine, doc: uuid::Uuid) -> [u8; 4] {
    e.composite_edit(doc, None, Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 }, 20, 20).unwrap().pixel(10, 10)
}

#[test]
fn a_preview_shows_through_every_render_path_without_touching_the_document() {
    let (mut e, doc, layer) = seeded();
    let before = middle(&e, doc);
    let (was_modified, could_undo, could_redo) = {
        let s = e.state(doc).unwrap();
        (s.is_modified, s.can_undo, s.can_redo)
    };
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    let dirty = e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a.clone() })).unwrap();
    assert_eq!(dirty.layers, vec![layer]);
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "the canvas shows the preview");
    let state = e.state(doc).unwrap();
    let l = state.layers.iter().find(|l| l.id == layer).unwrap();
    assert!(l.pixels_revision > 1_000_000, "a preview revision, so the renderer re-uploads");
    // Ruling (Task 10): a delta, not an absolute. seeded() imports into an existing document
    // through Engine::edit, so both flags are legitimately true before any preview exists.
    assert!(state.is_modified == was_modified && state.can_undo == could_undo && state.can_redo == could_redo,
        "a preview records nothing");
    // The stored document still holds the original pixels.
    assert_eq!(e.document(doc).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [128, 128, 128, 255]);
    assert_eq!(e.export_png(doc).map(|b| b.len() > 0).unwrap(), true);
    e.set_preview(doc, None).unwrap();
    assert_eq!(middle(&e, doc), before, "clearing the preview restores the canvas");
}

#[test]
fn a_command_undo_and_redo_all_end_a_preview() {
    let (mut e, doc, layer) = seeded();
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_white: 0.0, ..LevelRange::default() };
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a.clone() })).unwrap();
    e.execute(doc, Command::ApplyAdjustment { id: layer, adjustment: a.clone() }).unwrap();
    assert_eq!(middle(&e, doc), [0, 0, 0, 255], "committed for real");
    assert!(e.state(doc).unwrap().can_undo);
    assert_eq!(e.document(doc).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap().pixel(0, 0), [0, 0, 0, 255]);
    e.undo(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255]);
    e.set_preview(doc, Some(PreviewRequest::Adjustment { layer, adjustment: a })).unwrap();
    e.undo(doc).unwrap();
    assert_eq!(middle(&e, doc), [128, 128, 128, 255], "undo drops the preview rather than leaving it stranded");
}

#[test]
fn a_blur_preview_grows_the_layer_and_previews_from_a_reduced_copy() {
    let mut e = Engine::new();
    let doc = e.new_document(400, 400, false).unwrap();
    let bytes = encode_png(&Raster::from_premultiplied(300, 300, [255u8, 255, 255, 255].repeat(90_000)), 72.0).unwrap();
    e.import_image(Some(doc), &bytes, "Big", Some(Point { x: 200.0, y: 200.0 })).unwrap();
    let layer = e.state(doc).unwrap().active_layer_id.unwrap();
    let before = e.state(doc).unwrap().layers[0].transform;
    e.set_preview(doc, Some(PreviewRequest::Filter { layer, params: FilterParams::GaussianBlur { radius: 6.0 } })).unwrap();
    let state = e.state(doc).unwrap();
    let l = &state.layers[0];
    assert!(l.transform.size.width > before.size.width, "the preview shows the grown layer");
    assert!(l.pixels_width <= 2048, "previewed from a reduced copy: {}", l.pixels_width);
    assert!((l.transform.size.width / before.size.width - l.transform.size.height / before.size.height).abs() < 0.01);
    e.set_preview(doc, None).unwrap();
    assert_eq!(e.state(doc).unwrap().layers[0].transform, before);
}

#[test]
fn commands_apply_adjustments_filters_invert_and_adjustment_layers() {
    let (mut e, doc, layer) = seeded();
    e.execute(doc, Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert_eq!(middle(&e, doc), [127, 127, 127, 255]);
    e.execute(doc, Command::ApplyFilter { id: layer, params: FilterParams::AddNoise { amount: 40.0, gaussian: false, monochromatic: true, seed: 3 } }).unwrap();
    assert_ne!(middle(&e, doc), [127, 127, 127, 255]);
    let new = e.execute(doc, Command::AddAdjustmentLayer { kind: AdjustmentKind::Curves, seed: 0, shadows: None, highlights: None }).unwrap();
    assert!(new.structure);
    let state = e.state(doc).unwrap();
    let adjustment_layer = state.layers.iter().find(|l| l.adjustment.is_some()).unwrap();
    assert_eq!(adjustment_layer.adjustment.as_ref().unwrap().kind, AdjustmentKind::Curves);
    assert!(!adjustment_layer.has_pixels);
    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains(r#""adjustment":{"#) && json.contains(r#""kind":"Curves""#));
    let mut edited = LayerAdjustment::new(AdjustmentKind::Curves);
    edited.curves.channels[0] = vec![CurvePoint { x: 0.0, y: 255.0 }, CurvePoint { x: 255.0, y: 255.0 }];
    e.execute(doc, Command::SetAdjustment { id: adjustment_layer.id, adjustment: edited }).unwrap();
    assert_eq!(middle(&e, doc), [255, 255, 255, 255]);
}

#[test]
fn every_new_command_names_its_own_undo_step() {
    let names = [
        (Command::ApplyAdjustment { id: uuid::Uuid::nil(), adjustment: LayerAdjustment::new(AdjustmentKind::Hsv) }, "Hue/Saturation"),
        (Command::InvertPixels { id: uuid::Uuid::nil(), mask: false }, "Invert"),
        (Command::InvertPixels { id: uuid::Uuid::nil(), mask: true }, "Invert Mask"),
        (Command::ApplyFilter { id: uuid::Uuid::nil(), params: FilterParams::MotionBlur { angle: 0.0, distance: 10.0 } }, "Motion Blur"),
        (Command::AddAdjustmentLayer { kind: AdjustmentKind::Grain, seed: 0, shadows: None, highlights: None }, "New Grain Adjustment"),
        (Command::SetAdjustment { id: uuid::Uuid::nil(), adjustment: LayerAdjustment::new(AdjustmentKind::Levels) }, "Edit Levels Adjustment"),
    ];
    for (command, name) in names { assert_eq!(command.action_name(), name); }
    let json = serde_json::to_string(&Command::ApplyFilter { id: uuid::Uuid::nil(), params: FilterParams::LensCorrection { distortion: -20.0 } }).unwrap();
    assert!(json.contains(r#""type":"ApplyFilter""#) && json.contains(r#""filter":"LensCorrection""#), "{json}");
    assert!(serde_json::from_str::<Command>(&json).is_ok());
}

#[test]
fn histograms_auto_levels_and_the_eyedroppers_read_the_right_pixels() {
    let (mut e, doc, layer) = seeded();
    let bins = e.histogram(doc, layer).unwrap();
    assert_eq!(bins.len(), 4);
    assert_eq!(bins[1][128], 64.0, "every pixel of the 8x8 gray layer");
    let auto = e.auto_levels(doc, layer, LevelsAuto::Contrast).unwrap();
    assert!(auto.ranges[0].black <= 128.0 && auto.ranges[0].white >= 128.0);
    let sampled = e.sample_layer_color(doc, layer, Point { x: 11.0, y: 11.0 }).unwrap().unwrap();
    assert!((sampled[0] - 128.0 / 255.0).abs() < 0.01);
    assert!(e.sample_layer_color(doc, layer, Point { x: 1.0, y: 1.0 }).unwrap().is_none(), "outside the layer");
    let calibrated = e.levels_sampling(doc, layer, &LevelsSettings::default(), Point { x: 11.0, y: 11.0 }, LevelsSample::Gray).unwrap();
    assert!((calibrated.apply(128.0 / 255.0, LevelsChannel::Red) - 0.5).abs() < 0.01, "the sampled tone becomes mid gray");
    assert!(e.sample_color(doc, Point { x: 11.0, y: 11.0 }).unwrap().is_some());
    assert!(e.sample_color(doc, Point { x: 1.0, y: 1.0 }).unwrap().is_none(), "nothing there to sample");
    // An adjustment layer's histogram reads the composite beneath it, not its own (absent) pixels.
    let a = e.execute(doc, Command::AddAdjustmentLayer { kind: AdjustmentKind::Levels, seed: 0, shadows: None, highlights: None }).map(|_| ()).and_then(|_| {
        Ok(e.state(doc).unwrap().layers.iter().find(|l| l.adjustment.is_some()).unwrap().id)
    }).unwrap();
    let under = e.histogram(doc, a).unwrap();
    assert_eq!(under[1][128], 64.0, "the gray layer underneath");
    assert!(e.adjustment_source(doc, a).unwrap().width == 20);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test preview`
Expected: compile errors.

- [ ] **Step 3: Implement preview.rs**

`engine/src/preview.rs`:
```rust
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What the open panel is showing. The engine computes it from the layer's stored pixels every
/// time, so dragging a slider never accumulates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "preview")]
pub enum PreviewRequest {
    Adjustment { #[serde(with = "ids::upper")] layer: Uuid, adjustment: LayerAdjustment },
    Filter { #[serde(with = "ids::upper")] layer: Uuid, params: FilterParams },
}

impl PreviewRequest {
    pub fn layer(&self) -> Uuid { match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::Filter { layer, .. } => *layer } }
}

/// The substituted pixels for one layer while a panel is open.
#[derive(Clone, Debug)]
pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64 }

/// Previews render from a copy no larger than this on its longest side. Grain and Add Noise are
/// made at full size: their pattern is per pixel, and a small copy enlarged looks coarse.
pub fn preview_limit(request: &PreviewRequest) -> u32 {
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => if adjustment.kind == AdjustmentKind::Grain { u32::MAX } else { 4096 },
        PreviewRequest::Filter { params, .. } => if matches!(params, FilterParams::AddNoise { .. }) { u32::MAX } else { 2048 },
    }
}

/// Sharp halvings until the longest side fits `limit`, and the factor that reached it.
fn reduced(raster: &Raster, limit: u32) -> (Raster, f64) {
    let mut current = raster.clone();
    let mut factor = 1.0;
    while current.width.max(current.height) > limit && current.width > 1 && current.height > 1 {
        current = current.halved();
        factor *= 0.5;
    }
    (current, factor)
}

/// The preview raster for a request, or None when there is nothing to show.
pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
    let layer = doc.layer(request.layer())?;
    let raster = layer.pixels.as_ref()?;
    let limit = preview_limit(request);
    let (source, factor) = reduced(raster, limit);
    match request {
        PreviewRequest::Adjustment { adjustment, .. } => {
            if !adjustment.is_valid() { return None; }
            // Grain and the tonal kernels read document space, which the reduced grid still covers.
            let units = layer.transform.size.width / source.width.max(1) as f64;
            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, None);
            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision })
        }
        PreviewRequest::Filter { params, .. } => {
            let params = params.normalized();
            if params.is_identity() { return None; }
            // Grown but never trimmed: trimming mid-drag would make the layer jump about.
            let scaled = params.scaled(factor);
            let (grid, placed) = match params.spreads() {
                true => ops::adjust::grown(&source, &layer.transform, scaled.margin())?,
                false => (source, layer.transform),
            };
            let result = adjust::filters::apply_filter(&grid, &scaled);
            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision })
        }
    }
}
```

- [ ] **Step 4: Add the commands**

`engine/src/command.rs`: import `AdjustmentKind`, `FilterParams`, `LayerAdjustment`; add the variants
```rust
    ApplyAdjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
    InvertPixels { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool },
    ApplyFilter { #[serde(with = "ids::upper")] id: Uuid, params: FilterParams },
    AddAdjustmentLayer { kind: AdjustmentKind, #[serde(default)] seed: u32, #[serde(default)] shadows: Option<[f64; 3]>, #[serde(default)] highlights: Option<[f64; 3]> },
    SetAdjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
```
and the `action_name` arms
```rust
            Command::ApplyAdjustment { adjustment, .. } => adjustment.kind.name(),
            Command::InvertPixels { mask: false, .. } => "Invert",
            Command::InvertPixels { mask: true, .. } => "Invert Mask",
            Command::ApplyFilter { params, .. } => params.name(),
            Command::AddAdjustmentLayer { kind, .. } => kind.new_action_name(),
            Command::SetAdjustment { adjustment, .. } => adjustment.kind.edit_action_name(),
```
In `adjust/settings.rs`, beside `name()`:
```rust
    /// Undo names, static so `Command::action_name` can stay `&'static str`.
    pub fn new_action_name(self) -> &'static str {
        match self { AdjustmentKind::Hsv => "New Hue/Saturation Adjustment", AdjustmentKind::Levels => "New Levels Adjustment", AdjustmentKind::Curves => "New Curves Adjustment",
            AdjustmentKind::Exposure => "New Exposure Adjustment", AdjustmentKind::GradientMap => "New Gradient Map Adjustment", AdjustmentKind::Grain => "New Grain Adjustment" }
    }
    pub fn edit_action_name(self) -> &'static str {
        match self { AdjustmentKind::Hsv => "Edit Hue/Saturation Adjustment", AdjustmentKind::Levels => "Edit Levels Adjustment", AdjustmentKind::Curves => "Edit Curves Adjustment",
            AdjustmentKind::Exposure => "Edit Exposure Adjustment", AdjustmentKind::GradientMap => "Edit Gradient Map Adjustment", AdjustmentKind::Grain => "Edit Grain Adjustment" }
    }
```

- [ ] **Step 5: Wire the engine facade**

`engine/src/engine.rs`:
- `Session` gains `pub preview: Option<PixelPreview>` (and `insert` sets it to `None`).
- `Engine` gains `preview_revision: u64` (a counter well above any real `pixels_revision`, so a preview never reuses a texture):
```rust
/// Preview revisions start here so they can never collide with a layer's own, which counts up
/// from 1 as the document is edited.
const PREVIEW_REVISION_BASE: u64 = 1 << 40;
```
- The render document:
```rust
    /// The document as the canvas should show it: the stored one, or a copy with the open
    /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
    /// the ops do not, because a preview is not committed.
    fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
        let s = self.session(id)?;
        let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
        let mut doc = s.document.clone();
        if let Some(layer) = doc.layer_mut(preview.layer) {
            layer.pixels = Some(preview.raster.clone());
            layer.pixels_revision = preview.revision;
            layer.transform = preview.transform;
        }
        Ok(std::borrow::Cow::Owned(doc))
    }
    pub fn set_preview(&mut self, id: Uuid, request: Option<PreviewRequest>) -> Result<Dirty, CommandError> {
        let revision = { self.preview_revision += 1; PREVIEW_REVISION_BASE + self.preview_revision };
        let s = self.session_mut(id)?;
        let layers: Vec<Uuid> = s.preview.iter().map(|p| p.layer).chain(request.iter().map(|r| r.layer())).collect();
        s.preview = request.as_ref().and_then(|r| preview::compute_preview(&s.document, r, revision));
        Ok(Dirty { structure: true, canvas: false, layers })
    }
    fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }
```
  (`layers` may list the same id twice; that is harmless, the renderer de-duplicates by key.)
- `state`, `render_plan`, `composite_edit`, `composite` and the new `layer_raster` read `self.render_document(id)?`; `export_png`, `export_jpeg`, `export_jpeg_preview`, `save_package` and `execute` keep reading `self.session(id)?.document`.
- `state` also fills `adjustment: l.extra.adjustment.clone()` for each layer (add the field to `LayerState` with `#[serde(skip_serializing_if = "Option::is_none")]`).
- `execute`, `undo`, `redo` and `revert` each call `self.clear_preview(handle)` first, and `execute` gains the arms:
```rust
            Command::ApplyAdjustment { id, adjustment } => { ops::adjust::apply_adjustment_to_layer(doc, id, &adjustment)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::InvertPixels { id, mask } => { ops::adjust::invert_layer(doc, id, mask)?; Ok(Dirty { structure: true, canvas: false, layers: if mask { vec![] } else { vec![id] } }) }
            Command::ApplyFilter { id, params } => { ops::adjust::apply_filter(doc, id, &params)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::AddAdjustmentLayer { kind, seed, shadows, highlights } => {
                let gradient = match (shadows, highlights) { (Some(s), Some(h)) => Some((s, h)), _ => None };
                ops::adjust::add_adjustment_layer(doc, kind, seed, gradient)?; Ok(Dirty::structure())
            }
            Command::SetAdjustment { id, adjustment } => { ops::adjust::set_adjustment(doc, id, &adjustment)?; Ok(Dirty::structure()) }
```
- The panel helpers:
```rust
    /// The layer's raster after `level` sharp halvings, through any open preview.
    pub fn layer_raster(&self, id: Uuid, layer: Uuid, level: u32) -> Result<Option<Raster>, CommandError> {
        let doc = self.render_document(id)?;
        let Some(mut raster) = doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone() else { return Ok(None); };
        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
            if raster.width <= 1 || raster.height <= 1 { break; }
            raster = raster.halved();
        }
        Ok(Some(raster))
    }
    /// Everything that renders beneath an adjustment layer, composited at canvas size: what its
    /// histogram and eyedroppers read, as macOS renders the layers underneath.
    pub fn adjustment_source(&self, id: Uuid, layer: Uuid) -> Result<Raster, CommandError> {
        let doc = &self.session(id)?.document;
        let order = ops::hierarchy::hierarchy_order(doc);
        let position = order.iter().position(|o| *o == layer).ok_or(CommandError::NoLayer)?;
        let beneath: std::collections::HashSet<Uuid> = order[..position].iter().copied().collect();
        let mut below = doc.clone();
        for l in &mut below.layers { if !l.is_group && !beneath.contains(&l.id) { l.visible = false; } }
        Ok(compositor::composite(&below, Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 }, doc.width, doc.height))
    }
    /// A panel's histogram: an adjustment layer reads what lies beneath it, any other layer its
    /// own stored pixels (never the preview, or the graph would chase itself).
    pub fn histogram(&self, id: Uuid, layer: Uuid) -> Result<Vec<Vec<f64>>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        if target.is_adjustment() { return Ok(adjust::levels::histogram(&self.adjustment_source(id, layer)?, None)); }
        let raster = target.pixels.as_ref().ok_or(CommandError::Argument("the layer has no pixels".into()))?;
        Ok(adjust::levels::histogram(raster, None))
    }
    pub fn auto_levels(&self, id: Uuid, layer: Uuid, mode: LevelsAuto) -> Result<LevelsSettings, CommandError> {
        Ok(mode.settings(&self.histogram(id, layer)?))
    }
    /// The straight colour of one layer at a document point; None where it is transparent.
    pub fn sample_layer_color(&self, id: Uuid, layer: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = &self.session(id)?.document;
        let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let (raster, source) = match (target.pixels.as_ref(), target.is_adjustment()) {
            (Some(r), _) => (r.clone(), target.transform),
            (None, true) => (self.adjustment_source(id, layer)?, LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, doc.size())),
            _ => return Err(CommandError::Argument("the layer has no pixels".into())),
        };
        let Some(inverse) = source.pixel_to_document(raster.width, raster.height).invert() else { return Ok(None); };
        let p = inverse.apply(at);
        if p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64 { return Ok(None); }
        let pixel = raster.pixel(p.x as u32, p.y as u32);
        if pixel[3] == 0 { return Ok(None); }
        Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))))
    }
    /// The straight colour of the visible composite at a document point (the Hue/Saturation eyedroppers).
    pub fn sample_color(&self, id: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
        let doc = self.render_document(id)?;
        let region = Rect { x: at.x.floor(), y: at.y.floor(), width: 1.0, height: 1.0 };
        let pixel = compositor::composite(&doc, region, 1, 1).pixel(0, 0);
        if pixel[3] == 0 { return Ok(None); }
        Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))))
    }
    pub fn levels_sampling(&self, id: Uuid, layer: Uuid, settings: &LevelsSettings, at: Point, mode: LevelsSample) -> Result<LevelsSettings, CommandError> {
        match self.sample_layer_color(id, layer, at)? { Some(rgb) => Ok(settings.sampling(rgb, mode)), None => Ok(settings.clone()) }
    }
```

`engine/src/lib.rs`: `pub mod preview;` and `pub use preview::*;`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine` then `cargo check -p compositor-engine-wasm --target wasm32-unknown-unknown`
Expected: all pass, including 6 in `preview.rs`; the wasm crate still compiles (its exports change in Task 11).

- [ ] **Step 7: Commit**

```
git add engine
git commit -m "feat(engine): live previews, adjustment and filter commands, panel helpers"
```

---

### Task 11: The wasm bridge and the TypeScript types

**Files:**
- Modify: `engine-wasm/src/lib.rs`, `app/src/engine/types.ts`, `app/src/engine/client.ts`
- Test: `engine/tests/engine2.rs` (one more test), `app/tests/e2e/smoke.spec.ts` (extend)

**Interfaces:**
- `WasmEngine`: `set_preview(doc, request_json: Option<String>) -> String`, `histogram(doc, layer) -> String`, `auto_levels(doc, layer, mode) -> String`, `levels_sampling(doc, layer, settings_json, x, y, mode) -> String`, `sample_layer_color(doc, layer, x, y) -> Option<String>`, `sample_color(doc, x, y) -> Option<String>`; `layer_pixels_ptr`/`layer_pixels_len` now read through any open preview.
- `EngineClient`: `setPreview(doc, PreviewRequest | null): Dirty`, `histogram(doc, layer): number[][]`, `autoLevels(doc, layer, mode): LevelsSettings`, `levelsSampling(doc, layer, settings, point, mode): LevelsSettings`, `sampleLayerColor(doc, layer, point): [number, number, number] | null`, `sampleColor(doc, point): [number, number, number] | null`.
- `types.ts`: the full mirror of the adjustment JSON, `FilterParams`, `PreviewRequest`, five new `Command` members, `LayerState.adjustment`, `LayerDraw.adjustment`, `PreviewEdit` adjustment variant.

- [ ] **Step 1: Write the failing tests**

Add to `engine/tests/engine2.rs`:
```rust
#[test]
fn phase3_commands_round_trip_json() {
    let mut e = Engine::new();
    let doc = e.new_document(8, 8, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let ida = ids::upper_string(&a);
    let run = |e: &mut Engine, json: String| { let c: Command = serde_json::from_str(&json).unwrap(); e.execute(doc, c).unwrap() };
    run(&mut e, format!(r#"{{"type":"ApplyFilter","id":"{ida}","params":{{"filter":"AddNoise","amount":25,"gaussian":false,"monochromatic":true,"seed":5}}}}"#));
    run(&mut e, format!(r#"{{"type":"InvertPixels","id":"{ida}","mask":false}}"#));
    run(&mut e, r#"{"type":"AddAdjustmentLayer","kind":"Gradient Map","seed":0,"shadows":[1,0,0],"highlights":[0,0,1]}"#.to_string());
    let state = e.state(doc).unwrap();
    let adjustment = state.layers.iter().find(|l| l.adjustment.is_some()).unwrap();
    assert_eq!(adjustment.adjustment.as_ref().unwrap().gradient_map().shadows.red, 1.0);
    run(&mut e, format!(r#"{{"type":"ApplyAdjustment","id":"{ida}","adjustment":{}}}"#, serde_json::to_string(&LayerAdjustment::new(AdjustmentKind::Exposure)).unwrap()));
    let request: PreviewRequest = serde_json::from_str(&format!(r#"{{"preview":"Filter","layer":"{ida}","params":{{"filter":"GaussianBlur","radius":2}}}}"#)).unwrap();
    assert_eq!(request.layer(), a);
    e.set_preview(doc, Some(request)).unwrap();
    assert!(e.layer_raster(doc, a, 0).unwrap().is_some());
}
```

Extend `app/tests/e2e/smoke.spec.ts`'s client test (inside the existing `page.evaluate`, after the mask assertions):
```ts
    api.engine.execute(doc, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
    const withAdjustment = api.engine.state(doc);
    const adjustmentLayer = withAdjustment.layers.find((l: any) => l.adjustment);
    const bins = api.engine.histogram(doc, adjustmentLayer.id);
    const auto = api.engine.autoLevels(doc, adjustmentLayer.id, "Contrast");
    const planWithAdjustment = api.engine.renderPlan(doc, null);
    const adjustmentDraw = planWithAdjustment.nodes.map((n: any) => n.draw).find((d: any) => d && d.adjustment);
```
and after the evaluate:
```ts
  expect(result.adjustmentKind).toBe("Levels");
  expect(result.bins).toBe(4);
  expect(result.autoIsIdentity).toBe(true);   // a blank document has nothing to stretch
  expect(result.drawHasAdjustment).toBe(true);
```
(return `adjustmentKind: adjustmentLayer.adjustment.kind, bins: bins.length, autoIsIdentity: auto.ranges[0].black === 0 && auto.ranges[0].white === 255, drawHasAdjustment: !!adjustmentDraw` from the evaluate.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test engine2`
Expected: the new test fails to compile.

- [ ] **Step 3: Extend the wasm bindings**

`engine-wasm/src/lib.rs`: `level_raster` becomes a call to the engine, and the panel helpers are exposed.
```rust
    fn level_raster(&self, doc: &str, layer: &str, level: u32) -> Result<Option<Raster>, JsError> {
        self.engine.layer_raster(parse_id(doc)?, parse_id(layer)?, level).map_err(js_err)
    }
    pub fn set_preview(&mut self, doc: &str, request_json: Option<String>) -> Result<String, JsError> {
        let request = match request_json { Some(j) => Some(serde_json::from_str::<PreviewRequest>(&j).map_err(js_err)?), None => None };
        serde_json::to_string(&self.engine.set_preview(parse_id(doc)?, request).map_err(js_err)?).map_err(js_err)
    }
    pub fn histogram(&self, doc: &str, layer: &str) -> Result<String, JsError> {
        serde_json::to_string(&self.engine.histogram(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?).map_err(js_err)
    }
    pub fn auto_levels(&self, doc: &str, layer: &str, mode: &str) -> Result<String, JsError> {
        let mode: LevelsAuto = serde_json::from_str(&format!("\"{mode}\"")).map_err(js_err)?;
        serde_json::to_string(&self.engine.auto_levels(parse_id(doc)?, parse_id(layer)?, mode).map_err(js_err)?).map_err(js_err)
    }
    pub fn levels_sampling(&self, doc: &str, layer: &str, settings_json: &str, x: f64, y: f64, mode: &str) -> Result<String, JsError> {
        let settings: LevelsSettings = serde_json::from_str(settings_json).map_err(js_err)?;
        let mode: LevelsSample = serde_json::from_str(&format!("\"{mode}\"")).map_err(js_err)?;
        let out = self.engine.levels_sampling(parse_id(doc)?, parse_id(layer)?, &settings, Point { x, y }, mode).map_err(js_err)?;
        serde_json::to_string(&out).map_err(js_err)
    }
    pub fn sample_layer_color(&self, doc: &str, layer: &str, x: f64, y: f64) -> Result<Option<String>, JsError> {
        match self.engine.sample_layer_color(parse_id(doc)?, parse_id(layer)?, Point { x, y }).map_err(js_err)? {
            Some(rgb) => Ok(Some(serde_json::to_string(&rgb).map_err(js_err)?)), None => Ok(None),
        }
    }
    pub fn sample_color(&self, doc: &str, x: f64, y: f64) -> Result<Option<String>, JsError> {
        match self.engine.sample_color(parse_id(doc)?, Point { x, y }).map_err(js_err)? {
            Some(rgb) => Ok(Some(serde_json::to_string(&rgb).map_err(js_err)?)), None => Ok(None),
        }
    }
```

- [ ] **Step 4: Mirror the JSON in types.ts**

`app/src/engine/types.ts` additions:
```ts
export type AdjustmentKind = "Hue/Saturation" | "Levels" | "Curves" | "Exposure" | "Gradient Map" | "Grain";
export const ADJUSTMENT_KINDS: AdjustmentKind[] = ["Hue/Saturation", "Levels", "Curves", "Exposure", "Gradient Map", "Grain"];
export type LevelsChannel = "RGB" | "Red" | "Green" | "Blue";
export type ColorRange = "Master" | "Reds" | "Yellows" | "Greens" | "Cyans" | "Blues" | "Magentas";
export type LevelsAuto = "Contrast" | "Color" | "Neutral";
export type LevelsSample = "Black" | "Gray" | "White";

export interface LevelRange { black: number; gamma: number; white: number; outputBlack: number; outputWhite: number; }
export interface LevelsSettings { channel: LevelsChannel; ranges: LevelRange[]; }
export interface CurvePoint { x: number; y: number; }
export interface CurvesSettings { channel: LevelsChannel; channels: CurvePoint[][]; }
export interface HueBand { falloffStart: number; rangeStart: number; rangeEnd: number; falloffEnd: number; }
export interface RangeAdjustment { hue: number; saturation: number; lightness: number; }
export interface HueSaturationSettings {
  range: ColorRange; colorize: boolean; invertRange: boolean;
  adjustments: Partial<Record<ColorRange, RangeAdjustment>>;
  bands: Partial<Record<ColorRange, HueBand>>;
}
export interface AdjustmentColor { red: number; green: number; blue: number; }
export interface ExposureSettings { exposure: number; offset: number; gamma: number; }
export interface GradientMapSettings { shadows: AdjustmentColor; highlights: AdjustmentColor; reversed: boolean; }
export interface GrainSettings { amount: number; size: number; roughness: number; seed: number; }
/** The Mac's LayerAdjustment: the optional settings are written only when present, so a project
 * saved by either app re-encodes byte for byte. */
export interface LayerAdjustment {
  kind: AdjustmentKind; hue: number; saturation: number; lightness: number; colorize: boolean;
  hsvSettings?: HueSaturationSettings; levels: LevelsSettings; curves: CurvesSettings;
  exposureSettings?: ExposureSettings; gradientMapSettings?: GradientMapSettings; grainSettings?: GrainSettings;
}

export type FilterParams =
  | { filter: "GaussianBlur"; radius: number }
  | { filter: "MotionBlur"; angle: number; distance: number }
  | { filter: "AddNoise"; amount: number; gaussian: boolean; monochromatic: boolean; seed: number }
  | { filter: "LensCorrection"; distortion: number };
export type FilterKind = FilterParams["filter"];

export type PreviewRequest =
  | { preview: "Adjustment"; layer: string; adjustment: LayerAdjustment }
  | { preview: "Filter"; layer: string; params: FilterParams };
```
`LayerState` gains `adjustment?: LayerAdjustment;`, `LayerDraw` gains `adjustment?: LayerAdjustment;`, `PreviewEdit` gains `| { kind: "adjustment"; id: string; adjustment: LayerAdjustment }`, and `Command` gains:
```ts
  | { type: "ApplyAdjustment"; id: string; adjustment: LayerAdjustment }
  | { type: "InvertPixels"; id: string; mask: boolean }
  | { type: "ApplyFilter"; id: string; params: FilterParams }
  | { type: "AddAdjustmentLayer"; kind: AdjustmentKind; seed: number; shadows: [number, number, number] | null; highlights: [number, number, number] | null }
  | { type: "SetAdjustment"; id: string; adjustment: LayerAdjustment }
```

- [ ] **Step 5: Extend the client**

`app/src/engine/client.ts`:
```ts
  /** Substitutes one layer's pixels with the open panel's result. Cleared by any command, undo
   * or redo, and by passing null. */
  setPreview(doc: string, request: PreviewRequest | null): Dirty {
    return JSON.parse(this.wasm.set_preview(doc, request ? JSON.stringify(request) : undefined)) as Dirty;
  }
  /** Four arrays of 256 bins: the mean of the channels, then red, green and blue. */
  histogram(doc: string, layer: string): number[][] { return JSON.parse(this.wasm.histogram(doc, layer)) as number[][]; }
  autoLevels(doc: string, layer: string, mode: LevelsAuto): LevelsSettings { return JSON.parse(this.wasm.auto_levels(doc, layer, mode)) as LevelsSettings; }
  levelsSampling(doc: string, layer: string, settings: LevelsSettings, at: { x: number; y: number }, mode: LevelsSample): LevelsSettings {
    return JSON.parse(this.wasm.levels_sampling(doc, layer, JSON.stringify(settings), at.x, at.y, mode)) as LevelsSettings;
  }
  sampleLayerColor(doc: string, layer: string, at: { x: number; y: number }): [number, number, number] | null {
    const json = this.wasm.sample_layer_color(doc, layer, at.x, at.y);
    return json ? (JSON.parse(json) as [number, number, number]) : null;
  }
  sampleColor(doc: string, at: { x: number; y: number }): [number, number, number] | null {
    const json = this.wasm.sample_color(doc, at.x, at.y);
    return json ? (JSON.parse(json) as [number, number, number]) : null;
  }
```

- [ ] **Step 6: Run everything**

```
cargo test -p compositor-engine
cargo check -p compositor-engine-wasm --target wasm32-unknown-unknown
pnpm wasm:dev
pnpm build
pnpm test
pnpm e2e
```
Expected: all green; the smoke spec's extended test passes.

- [ ] **Step 7: Commit**

```
git add engine engine-wasm app
git commit -m "feat(engine,app): adjustment types and panel helpers through wasm and the client"
```

---

### Task 12: Panel state in the store

**Files:**
- Create: `app/src/state/adjust-edit.ts`
- Modify: `app/src/state/store.ts`
- Test: `app/tests/unit/adjust-store.test.ts`

**Interfaces:**
- `adjust-edit.ts`: `AdjustEdit { kind: AdjustmentKind | FilterKind; target: "layer" | "adjustmentLayer"; layerId: string; adjustment: LayerAdjustment | null; params: FilterParams | null; original: LayerAdjustment | null; preview: boolean; sampleMode: SampleMode | null; histogram: number[][] | null }`, `SampleMode = "Black" | "Gray" | "White" | "replace" | "add" | "remove"`, `defaultAdjustment(kind): LayerAdjustment`, `defaultFilterParams(kind): FilterParams`, `adjustTitle(edit): string`, `isAdjustIdentity(edit): boolean`, `previewRequestFor(edit): PreviewRequest | null`.
- Store additions: `adjustEdit: AdjustEdit | null`; actions `beginAdjust(opts: { kind; layerId?; target? }): boolean`, `updateAdjust(patch: { adjustment?; params? }): void`, `setAdjustPreview(on: boolean): void`, `setAdjustSample(mode: SampleMode | null): void`, `commitAdjust(): void`, `cancelAdjust(): void`, `canAdjust(): boolean`. `run`, `undo` and `redo` refuse while a panel is open; `previewEdit()` returns the adjustment-layer preview.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/adjust-store.test.ts`:
```ts
import { beforeEach, describe, expect, it } from "vitest";
import { useEditor } from "../../src/state/store";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, previewRequestFor } from "../../src/state/adjust-edit";
import type { Command, DocumentState, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
    maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
function document(layers: LayerState[], active: string): DocumentState {
  return { id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: active, canUndo: false,
    canRedo: false, isModified: false, undoDepth: 0, path: null, layers };
}

function install(layers: LayerState[], active: string) {
  const calls: Command[] = [];
  const previews: (PreviewRequest | null)[] = [];
  const state = document(layers, active);
  const engine = {
    state: () => state,
    execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => [new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1)],
    undo: () => ({ structure: true, canvas: false, layers: [] }),
    redo: () => ({ structure: true, canvas: false, layers: [] }),
  } as unknown as EngineClient;
  useEditor.setState({ engine, activeId: "D", documents: { D: state }, selectedLayerIds: [active], maskSelected: false,
    transformEdit: null, adjustEdit: null, error: null, tool: "move" });
  return { calls, previews };
}

describe("adjustment panels", () => {
  beforeEach(() => useEditor.setState({ adjustEdit: null, error: null }));

  it("opening a panel on a pixel layer previews without recording anything", () => {
    const { calls, previews } = install([layer("A")], "A");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(true);
    const edit = useEditor.getState().adjustEdit!;
    expect(edit.target).toBe("layer");
    expect(edit.histogram?.length).toBe(4);
    expect(previews.at(-1)).toBeNull();   // an identity adjustment previews nothing
    const next = defaultAdjustment("Levels");
    next.levels.ranges[0] = { ...next.levels.ranges[0], outputWhite: 0 };
    useEditor.getState().updateAdjust({ adjustment: next });
    expect((previews.at(-1) as any).preview).toBe("Adjustment");
    expect(calls).toEqual([]);
    useEditor.getState().commitAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([{ type: "ApplyAdjustment", id: "A", adjustment: next }]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("an unchanged panel commits nothing and cancel clears the preview", () => {
    const { calls, previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
    useEditor.getState().beginAdjust({ kind: "Gaussian Blur" as never });
    useEditor.getState().updateAdjust({ params: { filter: "GaussianBlur", radius: 4 } });
    useEditor.getState().cancelAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("editing an adjustment layer previews through the plan and commits SetAdjustment", () => {
    const adjustment = defaultAdjustment("Curves");
    const { calls, previews } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment })], "J");
    expect(useEditor.getState().beginAdjust({ kind: "Curves", layerId: "J", target: "adjustmentLayer" })).toBe(true);
    const next = defaultAdjustment("Curves");
    next.curves.channels[0] = [{ x: 0, y: 255 }, { x: 255, y: 255 }];
    useEditor.getState().updateAdjust({ adjustment: next });
    expect(previews).toEqual([]);   // no pixel preview: the plan carries it
    expect(useEditor.getState().previewEdit()).toEqual({ kind: "adjustment", id: "J", adjustment: next });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([{ type: "SetAdjustment", id: "J", adjustment: next }]);
    expect(useEditor.getState().previewEdit()).toBeNull();
  });

  it("a panel owns the document: other commands, undo and redo are refused while it is open", () => {
    const { calls } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().run({ type: "AddBlankLayer" });
    useEditor.getState().undo();
    useEditor.getState().redo();
    expect(calls).toEqual([]);
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    expect(useEditor.getState().adjustEdit).not.toBeNull();
  });

  it("refuses to open on a folder, a hidden layer or a layer with no pixels", () => {
    install([layer("F", { isGroup: true, hasPixels: false })], "F");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("H", { visible: false })], "H");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("B", { hasPixels: false, pixelsWidth: 0 })], "B");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    expect(useEditor.getState().canAdjust()).toBe(false);
  });

  it("knows the default settings and which ones do nothing", () => {
    expect(defaultAdjustment("Grain").grainSettings?.amount).toBe(25);
    expect(defaultFilterParams("MotionBlur")).toEqual({ filter: "MotionBlur", angle: 0, distance: 10 });
    expect(isAdjustIdentity({ kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null } as never)).toBe(true);
    expect(previewRequestFor({ target: "layer", layerId: "A", kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null, preview: true } as never)).toBeNull();
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test`
Expected: module not found.

- [ ] **Step 3: Implement adjust-edit.ts**

`app/src/state/adjust-edit.ts`:
```ts
import type { AdjustmentKind, FilterKind, FilterParams, LayerAdjustment, PreviewRequest } from "../engine/types";

/** Which eyedropper is armed: the Levels three, or the Hue/Saturation band tools. */
export type SampleMode = "Black" | "Gray" | "White" | "replace" | "add" | "remove";
/** Everything a panel edits. `kind` is an adjustment kind or a filter kind; the two never mix. */
export interface AdjustEdit {
  kind: AdjustmentKind | FilterKind;
  /** "layer" rewrites a pixel layer on OK; "adjustmentLayer" edits an adjustment layer's settings. */
  target: "layer" | "adjustmentLayer";
  layerId: string;
  adjustment: LayerAdjustment | null;
  params: FilterParams | null;
  /** The settings to put back when an adjustment-layer edit is cancelled. */
  original: LayerAdjustment | null;
  preview: boolean;
  sampleMode: SampleMode | null;
  histogram: number[][] | null;
}

const IDENTITY_LEVELS = { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 };
const IDENTITY_CURVE = [{ x: 0, y: 0 }, { x: 255, y: 255 }];

export function defaultAdjustment(kind: AdjustmentKind): LayerAdjustment {
  const base: LayerAdjustment = {
    kind, hue: 0, saturation: 0, lightness: 0, colorize: false,
    levels: { channel: "RGB", ranges: [0, 1, 2, 3].map(() => ({ ...IDENTITY_LEVELS })) },
    curves: { channel: "RGB", channels: [0, 1, 2, 3].map(() => IDENTITY_CURVE.map((p) => ({ ...p }))) },
  };
  if (kind === "Exposure") base.exposureSettings = { exposure: 0, offset: 0, gamma: 1 };
  if (kind === "Gradient Map") base.gradientMapSettings = { shadows: { red: 0, green: 0, blue: 0 }, highlights: { red: 1, green: 1, blue: 1 }, reversed: false };
  if (kind === "Grain") base.grainSettings = { amount: 25, size: 1.5, roughness: 50, seed: 0 };
  return base;
}

export function defaultFilterParams(kind: FilterKind): FilterParams {
  switch (kind) {
    case "GaussianBlur": return { filter: "GaussianBlur", radius: 1 };
    case "MotionBlur": return { filter: "MotionBlur", angle: 0, distance: 10 };
    case "AddNoise": return { filter: "AddNoise", amount: 10, gaussian: false, monochromatic: false, seed: Math.floor(Math.random() * 0xffffffff) };
    default: return { filter: "LensCorrection", distortion: 0 };
  }
}

export const FILTER_TITLES: Record<FilterKind, string> = {
  GaussianBlur: "Gaussian Blur", MotionBlur: "Motion Blur", AddNoise: "Add Noise", LensCorrection: "Lens Correction",
};
export function adjustTitle(edit: Pick<AdjustEdit, "kind" | "params">): string {
  return edit.params ? FILTER_TITLES[edit.params.filter] : (edit.kind as string);
}
export const isFilterKind = (kind: string): kind is FilterKind => kind in FILTER_TITLES;

/** Whether the panel's current settings would change nothing (so OK records no undo step). */
export function isAdjustIdentity(edit: Pick<AdjustEdit, "kind" | "adjustment" | "params">): boolean {
  if (edit.params) {
    const p = edit.params;
    return p.filter === "LensCorrection" ? p.distortion === 0 : false;
  }
  const a = edit.adjustment;
  if (!a) return true;
  return JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind));
}

/** The pixel preview a destructive panel asks the engine for; null when there is nothing to show. */
export function previewRequestFor(edit: AdjustEdit): PreviewRequest | null {
  if (edit.target !== "layer" || !edit.preview || isAdjustIdentity(edit)) return null;
  if (edit.params) return { preview: "Filter", layer: edit.layerId, params: edit.params };
  return edit.adjustment ? { preview: "Adjustment", layer: edit.layerId, adjustment: edit.adjustment } : null;
}
```

- [ ] **Step 4: Implement the store additions**

`app/src/state/store.ts`: add `adjustEdit: AdjustEdit | null` to the state (initial `null`) and these actions. `canAdjust` mirrors the Mac's `canAdjustColors`.
```ts
  canAdjust: () => {
    const { activeId, documents, selectedLayerIds, maskSelected, adjustEdit } = get();
    if (!activeId || adjustEdit) return false;
    const state = documents[activeId];
    const layer = activeLayer(state);
    return !!layer && !layer.isGroup && layer.hasPixels && !maskSelected && selectedLayerIds.length === 1 && visibleIds(state).has(layer.id);
  },
  beginAdjust: ({ kind, layerId, target }) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return false;
    get().commitTransform();
    const state = get().documents[activeId];
    const id = layerId ?? state.activeLayerId;
    const layer = id ? state.layers.find((l) => l.id === id) : null;
    if (!layer || get().adjustEdit) return false;
    const editing = target === "adjustmentLayer";
    if (editing ? !layer.adjustment : !get().canAdjust()) return false;
    const filter = isFilterKind(kind as string);
    const adjustment = filter ? null : (editing ? layer.adjustment! : defaultAdjustment(kind as AdjustmentKind));
    const edit: AdjustEdit = {
      kind, target: editing ? "adjustmentLayer" : "layer", layerId: layer.id,
      adjustment, params: filter ? defaultFilterParams(kind as FilterKind) : null,
      original: editing ? layer.adjustment! : null, preview: true, sampleMode: null,
      // Levels and Curves draw a histogram of what they are about to change.
      histogram: kind === "Levels" || kind === "Curves" ? engine.histogram(activeId, layer.id) : null,
    };
    set({ adjustEdit: edit });
    get().applyAdjustPreview();
    return true;
  },
  updateAdjust: (patch) => {
    const edit = get().adjustEdit; if (!edit) return;
    set({ adjustEdit: { ...edit, ...patch } });
    get().applyAdjustPreview();
  },
  setAdjustPreview: (preview) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, preview } }); get().applyAdjustPreview(); },
  setAdjustSample: (sampleMode) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, sampleMode } }); },
  /** Pushes the panel's settings to the engine: a pixel preview for a destructive edit, or a
   * plan-level preview (through `previewEdit`) when an adjustment layer is being edited. */
  applyAdjustPreview: () => {
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId) return;
    if (!adjustEdit || adjustEdit.target === "adjustmentLayer") { if (!adjustEdit) engine.setPreview(activeId, null); get().invalidate(); return; }
    engine.setPreview(activeId, previewRequestFor(adjustEdit));
    get().refresh(activeId);
  },
  commitAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    set({ adjustEdit: null });
    engine.setPreview(activeId, null);
    if (isAdjustIdentity(edit)) { get().refresh(activeId); get().invalidate(); return; }
    if (edit.target === "adjustmentLayer") { get().run({ type: "SetAdjustment", id: edit.layerId, adjustment: edit.adjustment! }); return; }
    if (edit.params) get().run({ type: "ApplyFilter", id: edit.layerId, params: edit.params });
    else get().run({ type: "ApplyAdjustment", id: edit.layerId, adjustment: edit.adjustment! });
  },
  cancelAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    set({ adjustEdit: null });
    engine.setPreview(activeId, null);
    get().refresh(activeId);
    get().invalidate();
  },
```
`run` gains, above the transform commit:
```ts
    // A panel owns the document while it is open, as macOS's canEditLayers does. Its own commit
    // clears `adjustEdit` before calling this, so OK is never refused.
    if (get().adjustEdit) { set({ error: "Apply or cancel the open adjustment first" }); return; }
```
`undo` and `redo` gain the same guard (without the banner: the menu items are already disabled).
`previewEdit()` gains, before the transform cases:
```ts
    const a = get().adjustEdit;
    if (a && a.target === "adjustmentLayer" && a.preview && a.adjustment) return { kind: "adjustment", id: a.layerId, adjustment: a.adjustment };
```
`openDocument`, `setActive` and `closeDocument` clear `adjustEdit` and call `engine.setPreview(id, null)` alongside the transform commit; `setTool` leaves it alone (a panel and a tool change coexist, as on macOS).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test` and `pnpm build`
Expected: unit tests pass (6 new), tsc clean.

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): adjustment panel state, previews and the open-panel guard"
```

---

### Task 13: The WebGL2 adjustment pass

**Files:**
- Create: `app/src/canvas/gl/adjust-textures.ts`
- Modify: `app/src/canvas/gl/programs.ts`, `app/src/canvas/gl-renderer.ts`, `engine-wasm/src/lib.rs`, `app/src/engine/client.ts`, `app/src/engine/types.ts`
- Test: `app/tests/e2e/adjust-render.spec.ts`

**Interfaces:**
- `WasmEngine`/`EngineClient`: `adjustment_lut(adjustment_json) -> Uint8Array` / `adjustmentLut(adjustment): Uint8Array` (1024 bytes, 256 RGBA rows; empty for Hue/Saturation and Grain) and `hue_response_table(adjustment_json) -> Float32Array` / `hueResponse(adjustment): Float32Array` (361 x 4 floats: shift, saturation, lightness, 0; empty for other kinds). The tables come from the engine so there is exactly one implementation of the maths.
- `adjust-textures.ts`: `class AdjustTextures { lut(engine, adjustment): WebGLTexture | null; response(engine, adjustment): WebGLTexture | null; retain(keys: Set<string>); dispose() }`, keyed by the adjustment's JSON.
- `programs.ts`: an `adjust` program (uniforms `src`, `coverage`, `useCoverage`, `lut`, `response`, `opacity`, `mode`, `kind`, `deviceToDoc`, `colorize`, `colorizeAmounts`, `grain`, `grainSeed`), `ADJUST_KIND: Record<AdjustmentKind, number>`.

- [ ] **Step 1: Write the failing e2e test**

`app/tests/e2e/adjust-render.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";

// The canvas and the CPU compositor must agree on every adjustment. The fixture is per-pixel
// noise: flat colour would hide a wrong LUT index, and a smooth ramp would hide a wrong hue.
test.use({ viewport: { width: 900, height: 720 } });

async function setup(page: Page): Promise<string> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  return page.evaluate(async (data) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const png = Uint8Array.from(atob(data), (c) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return doc as string;
  }, b64);
}

async function expectMatchesCpu(page: Page, label: string) {
  const r = await page.evaluate(async () => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.compositeEdit(d.id, s.previewEdit(), { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  });
  expect(r.gl.length).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  // Grain hashes in f32 on both sides but rounds at different points; 3 covers that one level.
  expect(worst, `${label} (${r.kind}) max byte diff`).toBeLessThanOrEqual(label === "Grain" ? 3 : 2);
}

test("every adjustment layer renders the same on the GPU and the CPU", async ({ page }) => {
  await setup(page);
  const add = (kind: string, settings: unknown) => page.evaluate(({ kind, settings }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind, seed: 7, shadows: [0.9, 0.1, 0.2], highlights: [0.1, 0.4, 1] });
    const state = api.engine.state(s.activeId);
    const layer = state.layers.filter((l: any) => l.adjustment).at(-1);
    if (settings) api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, ...(settings as object) } });
    s.refresh(); s.invalidate();
    return layer.id as string;
  }, { kind, settings });
  const remove = (id: string) => page.evaluate((id) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "DeleteLayers", ids: [id], bake: false }); s.refresh(); s.invalidate();
  }, id);

  const cases: [string, unknown][] = [
    ["Levels", { levels: { channel: "RGB", ranges: [{ black: 30, gamma: 1.6, white: 220, outputBlack: 12, outputWhite: 243 }, { black: 0, gamma: 1.2, white: 255, outputBlack: 0, outputWhite: 255 }, { black: 10, gamma: 1, white: 250, outputBlack: 0, outputWhite: 255 }, { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 }] } }],
    ["Curves", { curves: { channel: "RGB", channels: [[{ x: 0, y: 15 }, { x: 96, y: 150 }, { x: 255, y: 240 }], [{ x: 0, y: 0 }, { x: 255, y: 255 }], [{ x: 0, y: 0 }, { x: 128, y: 100 }, { x: 255, y: 255 }], [{ x: 0, y: 0 }, { x: 255, y: 255 }]] } }],
    ["Exposure", { exposureSettings: { exposure: 0.7, offset: 0.06, gamma: 1.4 } }],
    ["Gradient Map", { gradientMapSettings: { shadows: { red: 0.9, green: 0.1, blue: 0.2 }, highlights: { red: 0.1, green: 0.4, blue: 1 }, reversed: false } }],
    ["Hue/Saturation", null],
    ["Grain", { grainSettings: { amount: 65, size: 2.5, roughness: 35, seed: 7 } }],
  ];
  for (const [kind, settings] of cases) {
    const id = await add(kind, settings);
    await expectMatchesCpu(page, kind);
    await remove(id);
  }
});

test("hue/saturation ranges, opacity, masks, blend modes and clipping match the CPU", async ({ page }) => {
  await setup(page);
  const id = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind: "Hue/Saturation", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(s.activeId).layers.filter((l: any) => l.adjustment).at(-1);
    const hsv = { range: "Reds", colorize: false, invertRange: false,
      adjustments: { Master: { hue: 25, saturation: 15, lightness: -5 }, Blues: { hue: -40, saturation: 60, lightness: 10 } },
      bands: layer.adjustment.hsvSettings?.bands ?? {} };
    api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, hsvSettings: hsv } });
    s.refresh(); s.invalidate();
    return layer.id as string;
  });
  await expectMatchesCpu(page, "hue ranges");
  const run = (cmd: unknown) => page.evaluate((cmd) => { const api = (window as any).__compositor; const s = api.store.getState(); api.engine.execute(s.activeId, cmd); s.refresh(); s.invalidate(); }, cmd);
  await run({ type: "SetLayerOpacity", id, opacity: 0.45 });
  await expectMatchesCpu(page, "opacity");
  await run({ type: "SetLayerBlendMode", id, mode: "Multiply" });
  await expectMatchesCpu(page, "blend mode");
  await run({ type: "SetLayerBlendMode", id, mode: "Normal" });
  await run({ type: "AddMask", id, revealing: true });
  await run({ type: "BlurMask", id, radius: 4 });
  await expectMatchesCpu(page, "soft mask");
  await run({ type: "ToggleClipping", id });
  await expectMatchesCpu(page, "clipped to the layer below");
});

test("a live panel preview draws the same as the CPU", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    s.beginAdjust({ kind: "Curves" });
    const next = JSON.parse(JSON.stringify(s.adjustEdit.adjustment));
    next.curves.channels[0] = [{ x: 0, y: 40 }, { x: 128, y: 200 }, { x: 255, y: 250 }];
    api.store.getState().updateAdjust({ adjustment: next });
  });
  await expectMatchesCpu(page, "destructive preview");
  await page.evaluate(() => (window as any).__compositor.store.getState().cancelAdjust());
  await expectMatchesCpu(page, "preview cancelled");
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm e2e --grep "every adjustment layer"`
Expected: fails (the GL renderer ignores adjustment draws, so the canvas shows the unadjusted image).

- [ ] **Step 3: Expose the tables**

`engine-wasm/src/lib.rs`:
```rust
    /// 256 RGBA rows for the kinds that map colour through a table; empty for the others.
    pub fn adjustment_lut(&self, adjustment_json: &str) -> Result<Uint8Array, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        let mut out = Vec::new();
        match PreparedAdjustment::prepare(&a) {
            PreparedAdjustment::Tables(tables) => {
                for i in 0..256 { for c in 0..3 { out.push((tables[c * 256 + i] * 255.0).round().clamp(0.0, 255.0) as u8); } out.push(255); }
            }
            PreparedAdjustment::GradientMap(table) => {
                for i in 0..256 { out.extend_from_slice(&table[i * 3..i * 3 + 3]); out.push(255); }
            }
            _ => {}
        }
        Ok(Uint8Array::from(out.as_slice()))
    }
    /// 361 entries of (hue shift, saturation, lightness, 0); empty unless this is a Hue/Saturation adjustment.
    pub fn hue_response_table(&self, adjustment_json: &str) -> Result<js_sys::Float32Array, JsError> {
        let a: LayerAdjustment = serde_json::from_str(adjustment_json).map_err(js_err)?;
        if a.kind != AdjustmentKind::Hsv { return Ok(js_sys::Float32Array::new_with_length(0)); }
        let mut out = Vec::with_capacity(361 * 4);
        for entry in hue_response(&a.resolved_hsv()) { out.extend_from_slice(&[entry[0] as f32, entry[1] as f32, entry[2] as f32, 0.0]); }
        Ok(js_sys::Float32Array::from(out.as_slice()))
    }
```
`client.ts`:
```ts
  adjustmentLut(adjustment: LayerAdjustment): Uint8Array { return this.wasm.adjustment_lut(JSON.stringify(adjustment)); }
  hueResponse(adjustment: LayerAdjustment): Float32Array { return this.wasm.hue_response_table(JSON.stringify(adjustment)); }
```

- [ ] **Step 4: Implement adjust-textures.ts**

`app/src/canvas/gl/adjust-textures.ts`:
```ts
import type { EngineClient } from "../../engine/client";
import type { LayerAdjustment } from "../../engine/types";

interface Entry { lut: WebGLTexture | null; response: WebGLTexture | null; }

/** LUT and hue-response textures, keyed by the adjustment's own JSON: the same settings reuse
 * the same textures frame after frame, and changed settings make new ones. */
export class AdjustTextures {
  private entries = new Map<string, Entry>();
  constructor(private readonly gl: WebGL2RenderingContext) {}

  static key(adjustment: LayerAdjustment): string { return JSON.stringify(adjustment); }

  private entry(engine: EngineClient, adjustment: LayerAdjustment): Entry {
    const key = AdjustTextures.key(adjustment);
    const existing = this.entries.get(key);
    if (existing) return existing;
    const gl = this.gl;
    let lut: WebGLTexture | null = null;
    const bytes = engine.adjustmentLut(adjustment);
    if (bytes.length === 1024) {
      lut = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, lut);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, 256, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, bytes);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
      // LINEAR, so a value between two entries interpolates exactly as the CPU's table does.
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    let response: WebGLTexture | null = null;
    const table = engine.hueResponse(adjustment);
    if (table.length === 361 * 4) {
      response = gl.createTexture()!;
      gl.bindTexture(gl.TEXTURE_2D, response);
      // Sampled with texelFetch at whole degrees, so no filtering is needed (and RGBA16F is not
      // filterable in core WebGL2 anyway).
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, 361, 1, 0, gl.RGBA, gl.FLOAT, table);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    const made = { lut, response };
    this.entries.set(key, made);
    return made;
  }
  lut(engine: EngineClient, adjustment: LayerAdjustment): WebGLTexture | null { return this.entry(engine, adjustment).lut; }
  response(engine: EngineClient, adjustment: LayerAdjustment): WebGLTexture | null { return this.entry(engine, adjustment).response; }
  /** Drops every entry whose settings are no longer in the plan. */
  retain(keys: Set<string>): void {
    for (const [key, entry] of [...this.entries]) {
      if (keys.has(key)) continue;
      if (entry.lut) this.gl.deleteTexture(entry.lut);
      if (entry.response) this.gl.deleteTexture(entry.response);
      this.entries.delete(key);
    }
  }
  dispose(): void { this.retain(new Set()); }
}
```

- [ ] **Step 5: Add the adjust program**

`app/src/canvas/gl/programs.ts`: add the kind codes and the shader. `ADJUST_GLSL` ports `rgb_to_hsl`/`hsl_to_rgb`/`adjust_rgb` from `engine/src/adjust/hsv.rs` and `mix32`/`lattice`/the midtone weight from `engine/src/adjust/grain.rs`; every constant must match those files exactly.
```ts
export const ADJUST_KIND: Record<string, number> = { identity: 0, tables: 1, gradientMap: 2, hsv: 3, grain: 4 };

const ADJUST_GLSL = `
vec3 rgbToHsl(vec3 c) {
  float high = max(c.r, max(c.g, c.b)), low = min(c.r, min(c.g, c.b));
  float lightness = (high + low) * 0.5, delta = high - low;
  if (delta <= 0.0) return vec3(0.0, 0.0, lightness);
  float saturation = min(1.0, delta / (1.0 - abs(2.0 * lightness - 1.0)));
  float hue = high == c.r ? (c.g - c.b) / delta : (high == c.g ? (c.b - c.r) / delta + 2.0 : (c.r - c.g) / delta + 4.0);
  hue *= 60.0;
  if (hue < 0.0) hue += 360.0;
  return vec3(hue, saturation, lightness);
}
vec3 hslToRgb(vec3 hsl) {
  if (hsl.y <= 0.0) return vec3(hsl.z);
  float chroma = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
  float sector = hsl.x / 60.0;
  float second = chroma * (1.0 - abs(mod(sector, 2.0) - 1.0));
  float base = hsl.z - chroma * 0.5;
  vec3 rgb = sector < 1.0 ? vec3(chroma, second, 0.0) : sector < 2.0 ? vec3(second, chroma, 0.0)
    : sector < 3.0 ? vec3(0.0, chroma, second) : sector < 4.0 ? vec3(0.0, second, chroma)
    : sector < 5.0 ? vec3(second, 0.0, chroma) : vec3(chroma, 0.0, second);
  return clamp(rgb + base, 0.0, 1.0);
}
uint mix32(uint x) {
  x ^= x >> 16; x *= 0x7feb352du;
  x ^= x >> 15; x *= 0x846ca68bu;
  x ^= x >> 16;
  return x;
}
float lattice(int ix, int iy, uint seed) {
  uint h = mix32(uint(ix) * 0x9E3779B1u ^ mix32(uint(iy) * 0x85EBCA77u ^ seed));
  return float(h & 0xFFFFu) / 65535.0 + float(h >> 16) / 65535.0 - 1.0;
}`;

const FRAG_ADJUST = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform sampler2D coverage;
uniform sampler2D lut;
uniform sampler2D response;
uniform bool useCoverage;
uniform bool colorize;
uniform float opacity;
uniform int mode;
uniform int kind;
uniform mat3 deviceToDoc;
uniform vec3 colorizeAmounts;   // hue, saturation, lightness of the Master range
uniform vec3 grain;             // size, roughness, strength
uniform uint grainSeed;
out vec4 color;
${BLEND_GLSL}
${ADJUST_GLSL}
vec3 throughTables(vec3 c) {
  return vec3(texture(lut, vec2((c.r * 255.0 + 0.5) / 256.0, 0.5)).r,
              texture(lut, vec2((c.g * 255.0 + 0.5) / 256.0, 0.5)).g,
              texture(lut, vec2((c.b * 255.0 + 0.5) / 256.0, 0.5)).b);
}
vec3 throughGradientMap(vec3 c) {
  int level = (2126 * int(floor(c.r * 255.0 + 0.5)) + 7152 * int(floor(c.g * 255.0 + 0.5)) + 722 * int(floor(c.b * 255.0 + 0.5)) + 5000) / 10000;
  return texture(lut, vec2((float(min(level, 255)) + 0.5) / 256.0, 0.5)).rgb;
}
vec3 throughHsl(vec3 c) {
  vec3 hsl = rgbToHsl(c);
  float lightnessAmount;
  if (colorize) {
    hsl.x = mod(colorizeAmounts.x, 360.0);
    if (hsl.x < 0.0) hsl.x += 360.0;
    hsl.y = clamp(colorizeAmounts.y / 100.0, 0.0, 1.0);
    lightnessAmount = colorizeAmounts.z / 100.0;
  } else {
    vec4 sampled = texelFetch(response, ivec2(clamp(int(floor(hsl.x + 0.5)), 0, 360), 0), 0);
    lightnessAmount = sampled.z / 100.0;
    hsl.x = mod(hsl.x + sampled.x, 360.0);
    if (hsl.x < 0.0) hsl.x += 360.0;
    hsl.y = clamp(hsl.y * (1.0 + sampled.y / 100.0), 0.0, 1.0);
  }
  float amount = clamp(lightnessAmount, -1.0, 1.0);
  hsl.z = amount >= 0.0 ? hsl.z + (1.0 - hsl.z) * amount : hsl.z * (1.0 + amount);
  return hslToRgb(vec3(hsl.x, hsl.y, clamp(hsl.z, 0.0, 1.0)));
}
vec3 throughGrain(vec3 c, vec2 at) {
  float size = grain.x > 0.0 ? grain.x : 1.0;
  float rough = clamp(grain.y / 100.0, 0.0, 1.0);
  uint fineSeed = mix32(grainSeed ^ 0xA511E9B3u);
  vec2 cell = floor(at / size);
  vec2 t = at / size - cell;
  t = t * t * (3.0 - 2.0 * t);
  int ix = int(cell.x), iy = int(cell.y);
  float n00 = lattice(ix, iy, grainSeed), n10 = lattice(ix + 1, iy, grainSeed);
  float n01 = lattice(ix, iy + 1, grainSeed), n11 = lattice(ix + 1, iy + 1, grainSeed);
  float top = n00 + (n10 - n00) * t.x, bottom = n01 + (n11 - n01) * t.x;
  float smoothNoise = (top + (bottom - top) * t.y) * 1.6;
  float fine = lattice(int(floor(at.x)), int(floor(at.y)), fineSeed);
  float noise = smoothNoise + (fine - smoothNoise) * rough;
  float level = min(1.0, dot(c, vec3(0.2126, 0.7152, 0.0722)));
  float delta = noise * grain.z * (0.4 + 2.4 * level * (1.0 - level)) / 255.0;
  return clamp(c + delta, 0.0, 1.0);
}
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  vec4 d = texelFetch(src, at, 0);
  if (d.a <= 0.0) { color = d; return; }
  float k = opacity * (useCoverage ? texelFetch(coverage, at, 0).r : 1.0);
  if (k <= 0.0) { color = d; return; }
  vec3 original = clamp(d.rgb / d.a, 0.0, 1.0);
  vec3 adjusted = original;
  if (kind == 1) adjusted = throughTables(original);
  else if (kind == 2) adjusted = throughGradientMap(original);
  else if (kind == 3) adjusted = throughHsl(original);
  else if (kind == 4) { vec3 p = deviceToDoc * vec3(gl_FragCoord.xy, 1.0); adjusted = throughGrain(original, p.xy / p.z); }
  if (mode != 0) adjusted = clamp(blendRgb(mode, original, adjusted), 0.0, 1.0);
  vec3 mixed = mix(original, clamp(adjusted, 0.0, 1.0), k);
  color = vec4(mixed * d.a, d.a);
}`;
```
Compile it in `createPrograms` as `adjust` with the uniform list above, add it to the attribute loop and to `disposePrograms`, and extend the `Programs` interface.

- [ ] **Step 6: Draw adjustments in the renderer**

`app/src/canvas/gl-renderer.ts`:
- Hold `private adjustTextures: AdjustTextures` (constructed and disposed with the others).
- `syncTextures`'s `note` already skips draws with `pixelsWidth === 0`, so an adjustment draw uploads no layer texture; collect the adjustment keys instead and call `this.adjustTextures.retain(keys)` after the plan is walked.
- `drawInto` handles an adjustment before looking for a layer texture:
```ts
  private drawInto(ctx: Ctx, pair: "main" | "stack", draw: LayerDraw, blend: string, useClip: boolean, level: number): void {
    const hasCoverage = draw.coverages.length > 0 || (useClip && !!draw.clip);
    if (draw.adjustment) {
      if (hasCoverage) { this.buildCoverage(ctx, draw.coverages, level); if (useClip && draw.clip) this.applyClip(ctx, draw.clip, level); }
      this.adjustPass(ctx, pair, draw, blend, hasCoverage ? level : null);
      this.fbos.swap(`${pair}A`, `${pair}B`);
      return;
    }
    const t = this.textures.get(ctx.state.id, draw.id);
    if (!t) return;
    ...
  }
```
- The pass itself maps the A buffer into B:
```ts
  /** Maps what is already in the pair's A buffer through the adjustment, into B. Nothing is
   * sampled from the layer: an adjustment layer has no pixels of its own. */
  private adjustPass(ctx: Ctx, pair: "main" | "stack", draw: LayerDraw, blend: string, coverageLevel: number | null): void {
    const gl = this.gl; const adjustment = draw.adjustment!;
    const p = this.programs.adjust;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(`${pair}B`, "rgba").fbo);
    gl.useProgram(p.program);
    const kind = adjustment.kind === "Hue/Saturation" ? 3 : adjustment.kind === "Grain" ? 4 : adjustment.kind === "Gradient Map" ? 2 : 1;
    gl.uniform1i(p.uniforms.kind, kind);
    gl.uniform1f(p.uniforms.opacity, draw.opacity);
    gl.uniform1i(p.uniforms.mode, BLEND_INDEX[blend as keyof typeof BLEND_INDEX]);
    gl.uniform1i(p.uniforms.useCoverage, coverageLevel === null ? 0 : 1);
    gl.uniformMatrix3fv(p.uniforms.deviceToDoc, true, new Float32Array(this.deviceToDoc(ctx.viewport, ctx.state, ctx.dpr)));
    const hsv = adjustment.hsvSettings;
    const master = hsv?.adjustments?.Master ?? { hue: adjustment.hue, saturation: adjustment.saturation, lightness: adjustment.lightness };
    gl.uniform1i(p.uniforms.colorize, (hsv?.colorize ?? adjustment.colorize) ? 1 : 0);
    gl.uniform3f(p.uniforms.colorizeAmounts, master.hue, master.saturation, master.lightness);
    const grain = adjustment.grainSettings ?? { amount: 25, size: 1.5, roughness: 50, seed: 0 };
    // strength mirrors `grain_strength` in engine/src/adjust/grain.rs.
    gl.uniform3f(p.uniforms.grain, grain.size, grain.roughness, Math.min(1, grain.amount / 100) * 0.35 * 255);
    gl.uniform1ui(p.uniforms.grainSeed, grain.seed >>> 0);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get(`${pair}A`, "rgba").tex); gl.uniform1i(p.uniforms.src, 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, coverageLevel === null ? this.white : this.fbos.get(`coverage${coverageLevel}`, "r8").tex); gl.uniform1i(p.uniforms.coverage, 1);
    gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, this.adjustTextures.lut(ctx.engine, adjustment) ?? this.white); gl.uniform1i(p.uniforms.lut, 2);
    gl.activeTexture(gl.TEXTURE3); gl.bindTexture(gl.TEXTURE_2D, this.adjustTextures.response(ctx.engine, adjustment) ?? this.white); gl.uniform1i(p.uniforms.response, 3);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }
```
  `Ctx` gains `engine: EngineClient` (set in `render`) so the pass can ask for its tables.
- The Stack branch needs no change: an adjustment child reaches `drawInto(ctx, "stack", child, child.blend, false, 0)` and maps the stack buffer, which is exactly where the CPU compositor applies it.

- [ ] **Step 7: Run everything**

```
pnpm wasm:dev
pnpm build
pnpm test
pnpm e2e
```
Expected: all pass, including the 3 new tests in `adjust-render.spec.ts`. If one kind is off by more than the tolerance, compare its GLSL against the Rust file named beside it line by line before touching the tolerance; a constant typed differently (0.2126 vs 0.2125) shows up as a uniform shift, a wrong LUT index as banding.

- [ ] **Step 8: Commit**

```
git add app engine-wasm
git commit -m "feat(app): WebGL2 adjustment pass matching the CPU compositor"
```

---

### Task 14: The panel shell, Levels and Curves

**Files:**
- Create: `app/src/panels/AdjustPanel.tsx`, `app/src/panels/LevelsPanel.tsx`, `app/src/panels/CurvesPanel.tsx`, `app/src/tools/levels-tools.ts`, `app/src/tools/curves-editor.ts`
- Modify: `app/src/App.tsx` (mount `AdjustPanel`), `app/src/styles.css`
- Test: `app/tests/unit/levels-tools.test.ts`, `app/tests/unit/curves-editor.test.ts`, `app/tests/e2e/adjust-panels.spec.ts`

**Interfaces:**
- `levels-tools.ts`: `histogramScale(bins: number[]): number` (the display cap), `clampRange(range: LevelRange): LevelRange`, `LEVELS_CHANNELS`.
- `curves-editor.ts`: `curveValue(points: CurvePoint[], x: number): number` (the Hermite port, drawing only), `curveSamples(points, count): number[]`, `nearestPoint(points, at, tolerance): number | null`, `insertPoint(points, at): CurvePoint[]`, `movePoint(points, index, to): CurvePoint[]`, `removePoint(points, index): CurvePoint[]`.
- Panel test ids: `adjust-panel`, `adjust-title`, `adjust-preview`, `adjust-reset`, `adjust-cancel`, `adjust-ok`; Levels: `levels-histogram`, `levels-channel`, `levels-auto`, `levels-sample-<mode>`, fields labelled "Black point", "Gamma", "White point", "Output black", "Output white"; Curves: `curves-editor`, `curves-channel`, `curves-point-x`, `curves-point-y`.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/levels-tools.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { clampRange, histogramScale } from "../../src/tools/levels-tools";

describe("levels tools", () => {
  it("keeps the distribution visible beside a clipping spike", () => {
    const bins = new Array(256).fill(100);
    bins[255] = 100_000;
    expect(histogramScale(bins)).toBe(400);
    bins[0] = 200_000;
    expect(histogramScale(bins)).toBe(400);
    bins[128] = 500_000;
    expect(histogramScale(bins)).toBe(400);
    expect(bins[128]).toBe(500_000);
    expect(histogramScale(new Array(256).fill(100))).toBe(100);
    const sparse = new Array(256).fill(0);
    expect(histogramScale(sparse)).toBe(0);
    sparse[255] = 50; expect(histogramScale(sparse)).toBe(50);
    sparse[0] = 100; expect(histogramScale(sparse)).toBe(100);
    sparse[128] = 200; expect(histogramScale(sparse)).toBe(200);
  });
  it("clamps a range the way the engine does", () => {
    const r = clampRange({ black: 300, gamma: Number.NaN, white: -1, outputBlack: -100, outputWhite: 400 });
    expect(r.black).toBeLessThan(r.white);
    expect(r).toEqual({ black: 254, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 });
  });
});
```

`app/tests/unit/curves-editor.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { curveSamples, curveValue, insertPoint, movePoint, nearestPoint, removePoint } from "../../src/tools/curves-editor";

const identity = [{ x: 0, y: 0 }, { x: 255, y: 255 }];

describe("curves editor", () => {
  it("interpolates the way the engine does", () => {
    expect(curveValue(identity, 100)).toBeCloseTo(100, 6);
    const s = [{ x: 0, y: 0 }, { x: 128, y: 190 }, { x: 255, y: 255 }];
    expect(curveValue(s, 128)).toBeCloseTo(190, 6);
    let last = -1;
    for (let x = 0; x <= 255; x++) { const v = curveValue(s, x); expect(v).toBeGreaterThanOrEqual(last - 1e-9); expect(v).toBeLessThanOrEqual(255); last = v; }
    expect(curveSamples(s, 256).length).toBe(256);
  });
  it("adds, moves and removes points under the editor's rules", () => {
    const three = insertPoint(identity, { x: 128, y: 190 });
    expect(three.map((p) => p.x)).toEqual([0, 128, 255]);
    expect(insertPoint(three, { x: 128, y: 10 })).toEqual(three, "a point already sits on that input");
    expect(nearestPoint(three, { x: 130, y: 188 }, 8)).toBe(1);
    expect(nearestPoint(three, { x: 60, y: 60 }, 8)).toBeNull();
    expect(movePoint(three, 1, { x: 200, y: 20 })[1]).toEqual({ x: 200, y: 20 });
    expect(movePoint(three, 1, { x: 300, y: 300 })[1]).toEqual({ x: 254, y: 255 }, "kept inside its neighbours and the box");
    expect(movePoint(three, 0, { x: 40, y: 30 })[0]).toEqual({ x: 0, y: 30 }, "an endpoint keeps its input");
    expect(removePoint(three, 1).map((p) => p.x)).toEqual([0, 255]);
    expect(removePoint(identity, 0)).toEqual(identity, "the endpoints stay");
    const full = Array.from({ length: 32 }, (_, i) => ({ x: Math.round((i * 255) / 31), y: 0 }));
    expect(insertPoint(full, { x: 3, y: 3 })).toEqual(full, "32 points is the limit");
  });
});
```

`app/tests/e2e/adjust-panels.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const open = (page: Page, kind: string) => page.evaluate((kind) => (window as any).__compositor.store.getState().beginAdjust({ kind }), kind);

test("the levels panel previews, applies once, and leaves the pixels alone until OK", async ({ page }) => {
  await setup(page);
  const before = (await state(page)).layers[0].pixelsRevision;
  await open(page, "Levels");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  await expect(page.getByTestId("adjust-title")).toHaveText("Levels");
  await expect(page.getByTestId("levels-histogram")).toBeVisible();
  await page.getByLabel("White point").fill("128");
  await page.getByLabel("White point").press("Enter");
  // The preview is on screen, but the document has recorded nothing.
  let d = await state(page);
  expect(d.canUndo).toBe(false);
  expect(await page.evaluate(() => !!(window as any).__compositor.store.getState().adjustEdit)).toBe(true);
  // Preview off shows the original again.
  await page.getByTestId("adjust-preview").uncheck();
  await page.getByTestId("adjust-preview").check();
  await page.getByTestId("adjust-ok").click();
  d = await state(page);
  expect(d.canUndo).toBe(true);
  expect(d.layers[0].pixelsRevision).toBeGreaterThan(before);
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  await page.keyboard.press("Control+z");
  expect((await state(page)).canUndo).toBe(false);
});

test("cancel and escape record nothing", async ({ page }) => {
  await setup(page);
  await open(page, "Levels");
  await page.getByLabel("Gamma").fill("2");
  await page.getByLabel("Gamma").press("Enter");
  await page.getByTestId("adjust-cancel").click();
  expect((await state(page)).canUndo).toBe(false);
  await open(page, "Curves");
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0);
  expect((await state(page)).canUndo).toBe(false);
});

test("auto levels and the reset button", async ({ page }) => {
  await setup(page);
  await open(page, "Levels");
  await page.getByTestId("levels-auto").selectOption("Contrast");
  const stretched = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.levels.ranges[0]);
  expect(stretched.black > 0 || stretched.white < 255).toBe(true);
  await page.getByTestId("adjust-reset").click();
  const reset = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.levels.ranges[0]);
  expect(reset).toEqual({ black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 });
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).canUndo).toBe(false, "an identity adjustment records nothing");
});

test("the curves editor adds a point by clicking and applies it", async ({ page }) => {
  await setup(page);
  await open(page, "Curves");
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.25);
  const points = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.curves.channels[0]);
  expect(points.length).toBe(3);
  expect(points[1].y).toBeGreaterThan(points[1].x, "clicking above the diagonal brightens");
  await page.getByTestId("adjust-ok").click();
  const d = await state(page);
  expect(d.canUndo).toBe(true);
  const _ = clickMenu;
});

test("a panel owns the document while it is open", async ({ page }) => {
  await setup(page);
  await open(page, "Levels");
  await page.getByTestId("layer-add").click();
  expect((await state(page)).layers.length).toBe(1);
  await expect(page.getByTestId("error-banner")).toContainText("Apply or cancel");
  await page.getByTestId("adjust-cancel").click();
  await page.getByTestId("layer-add").click();
  expect((await state(page)).layers.length).toBe(2);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "the levels panel"`
Expected: modules not found; no panel in the DOM.

- [ ] **Step 3: Implement levels-tools.ts and curves-editor.ts**

`app/src/tools/levels-tools.ts`:
```ts
import type { LevelRange, LevelsChannel } from "../engine/types";

export const LEVELS_CHANNELS: LevelsChannel[] = ["RGB", "Red", "Green", "Blue"];

/** Display-only vertical scaling, ported from LevelsHistogramDisplay on macOS: keep linear bin
 * ratios, but cap isolated spikes so a large solid background cannot flatten the graph. */
export function histogramScale(bins: number[]): number {
  const positive = bins.filter((v) => Number.isFinite(v) && v > 0);
  const peak = positive.length ? Math.max(...positive) : 0;
  if (peak <= 0) return 0;
  const interior = bins.slice(1, -1).filter((v) => Number.isFinite(v) && v > 0).sort((a, b) => a - b);
  if (interior.length === 0) return peak;
  const typical = interior[Math.floor((interior.length - 1) * 0.95)];
  return Math.min(peak, typical * 4);
}

const clamp = (v: number, lo: number, hi: number, fallback: number) => (Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback);

/** The same normalisation `LevelRange::normalized` applies in the engine, so the fields show
 * what will actually be used. */
export function clampRange(range: LevelRange): LevelRange {
  const black = clamp(range.black, 0, 254, 0);
  return {
    black,
    white: clamp(range.white, black + 1, 255, 255),
    gamma: clamp(range.gamma, 0.1, 9.99, 1),
    outputBlack: clamp(range.outputBlack, 0, 255, 0),
    outputWhite: clamp(range.outputWhite, 0, 255, 255),
  };
}
```

`app/src/tools/curves-editor.ts`:
```ts
import type { CurvePoint } from "../engine/types";

export const MAX_POINTS = 32;

/** The engine's `CurvesSettings::value` (engine/src/adjust/settings.rs), for drawing the curve
 * and its handles. The applied result always comes from the engine; this only draws. */
export function curveValue(points: CurvePoint[], x: number): number {
  let i = 0;
  for (let j = 0; j < points.length; j++) if (points[j].x <= x) i = j;
  i = Math.min(points.length - 2, Math.max(0, i));
  const d = points.slice(0, -1).map((p, j) => (points[j + 1].y - p.y) / (points[j + 1].x - p.x));
  const slope = (j: number): number => {
    if (j === 0) return d[0];
    if (j === points.length - 1) return d[d.length - 1];
    if (d[j - 1] * d[j] <= 0) return 0;
    return 2 / (1 / d[j - 1] + 1 / d[j]);
  };
  const h = points[i + 1].x - points[i].x;
  const t = Math.min(1, Math.max(0, (x - points[i].x) / h));
  const y = (2 * t ** 3 - 3 * t ** 2 + 1) * points[i].y + (t ** 3 - 2 * t ** 2 + t) * h * slope(i)
    + (-2 * t ** 3 + 3 * t ** 2) * points[i + 1].y + (t ** 3 - t ** 2) * h * slope(i + 1);
  return Math.min(255, Math.max(0, y));
}
export function curveSamples(points: CurvePoint[], count = 256): number[] {
  return Array.from({ length: count }, (_, i) => curveValue(points, (i * 255) / (count - 1)));
}
/** The index of a handle within `tolerance` of a point in curve space, or null. */
export function nearestPoint(points: CurvePoint[], at: CurvePoint, tolerance: number): number | null {
  let best: number | null = null; let bestDistance = tolerance;
  points.forEach((p, i) => { const distance = Math.hypot(p.x - at.x, p.y - at.y); if (distance <= bestDistance) { best = i; bestDistance = distance; } });
  return best;
}
export function insertPoint(points: CurvePoint[], at: CurvePoint): CurvePoint[] {
  const x = Math.round(Math.min(255, Math.max(0, at.x)));
  const y = Math.min(255, Math.max(0, at.y));
  if (points.length >= MAX_POINTS || points.some((p) => p.x === x)) return points;
  return [...points, { x, y }].sort((a, b) => a.x - b.x);
}
/** Moves one handle, keeping the inputs strictly increasing; the two endpoints keep their input. */
export function movePoint(points: CurvePoint[], index: number, to: CurvePoint): CurvePoint[] {
  const next = points.map((p) => ({ ...p }));
  const y = Math.min(255, Math.max(0, to.y));
  if (index === 0 || index === points.length - 1) { next[index].y = y; return next; }
  const low = points[index - 1].x + 1;
  const high = points[index + 1].x - 1;
  next[index] = { x: Math.round(Math.min(high, Math.max(low, to.x))), y };
  return next;
}
export function removePoint(points: CurvePoint[], index: number): CurvePoint[] {
  if (index <= 0 || index >= points.length - 1 || points.length <= 2) return points;
  return points.filter((_, i) => i !== index);
}
```

- [ ] **Step 4: Implement the panel shell**

`app/src/panels/AdjustPanel.tsx`: a floating, non-modal panel (the eyedroppers need the canvas clickable, so it must not be a `Sheet`). It routes by kind and owns Preview, Reset, Cancel and OK plus Enter and Escape.
```tsx
import { useEffect, type ReactNode } from "react";
import { useEditor } from "../state/store";
import { adjustTitle, defaultAdjustment, defaultFilterParams, isFilterKind } from "../state/adjust-edit";
import { LevelsPanel } from "./LevelsPanel";
import { CurvesPanel } from "./CurvesPanel";
import { HueSaturationPanel } from "./HueSaturationPanel";
import { FilterPanel } from "./FilterPanel";
import type { AdjustmentKind, FilterKind } from "../engine/types";

export function AdjustPanel() {
  const s = useEditor();
  const edit = s.adjustEdit;
  useEffect(() => {
    if (!edit) return;
    const key = (e: KeyboardEvent) => {
      // A field keeps its own Enter (committing the value); the panel answers the second one.
      if (e.key === "Escape") { e.preventDefault(); useEditor.getState().cancelAdjust(); }
      if (e.key === "Enter" && !(e.target instanceof HTMLInputElement)) { e.preventDefault(); useEditor.getState().commitAdjust(); }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [edit]);
  if (!edit) return null;
  const reset = () => {
    if (edit.params) s.updateAdjust({ params: defaultFilterParams(edit.params.filter) });
    else s.updateAdjust({ adjustment: defaultAdjustment(edit.adjustment!.kind) });
  };
  let body: ReactNode = null;
  if (edit.params) body = <FilterPanel />;
  else if (edit.adjustment!.kind === "Levels") body = <LevelsPanel />;
  else if (edit.adjustment!.kind === "Curves") body = <CurvesPanel />;
  else if (edit.adjustment!.kind === "Hue/Saturation") body = <HueSaturationPanel />;
  else body = <FilterPanel />;   // Exposure, Gradient Map and Grain share the filter panel's fields
  return (
    <div className="adjust-panel" data-testid="adjust-panel" role="dialog" aria-label={adjustTitle(edit)}>
      <div className="adjust-header" data-testid="adjust-title">{adjustTitle(edit)}</div>
      <div className="adjust-body">{body}</div>
      <div className="adjust-footer">
        <label><input type="checkbox" data-testid="adjust-preview" checked={edit.preview} onChange={(e) => s.setAdjustPreview(e.target.checked)} /> Preview</label>
        <button data-testid="adjust-reset" onClick={reset}>Reset</button>
        <button data-testid="adjust-cancel" onClick={s.cancelAdjust}>Cancel</button>
        <button data-testid="adjust-ok" className="primary" onClick={s.commitAdjust}>OK</button>
      </div>
    </div>
  );
}
/** The kinds the Image menu opens directly; the Filter menu opens the rest. */
export const IMAGE_ADJUSTMENTS: (AdjustmentKind | FilterKind)[] = ["Levels", "Curves", "Hue/Saturation", "Exposure", "Gradient Map", "Grain"];
export const isFilter = isFilterKind;
```
Mount it in `App.tsx` beside the sheets (`{s.adjustEdit && <AdjustPanel />}` is unnecessary: the component returns null itself). Style `.adjust-panel` as fixed, top right under the tool options row, dark, 1 px border, `z-index: 50` (below the modal sheets' 100, since a sheet must still cover it).

- [ ] **Step 5: Implement LevelsPanel and CurvesPanel**

`app/src/panels/LevelsPanel.tsx`: a canvas histogram (drawn from `edit.histogram[channelIndex]` scaled by `histogramScale`), the channel picker, the five number fields, Auto and the three eyedropper buttons.
```tsx
import { useEffect, useRef } from "react";
import { useEditor } from "../state/store";
import { LEVELS_CHANNELS, clampRange, histogramScale } from "../tools/levels-tools";
import type { LevelRange, LevelsAuto, LevelsChannel } from "../engine/types";

const FIELDS: [keyof LevelRange, string][] = [["black", "Black point"], ["gamma", "Gamma"], ["white", "White point"], ["outputBlack", "Output black"], ["outputWhite", "Output white"]];

export function LevelsPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  const settings = edit.adjustment!.levels;
  const channel = settings.channel;
  const index = LEVELS_CHANNELS.indexOf(channel);
  const range = settings.ranges[index];
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current; const bins = edit.histogram?.[index]; if (!element || !bins) return;
    const ctx = element.getContext("2d")!;
    const scale = histogramScale(bins);
    ctx.clearRect(0, 0, element.width, element.height);
    ctx.fillStyle = "#8a8a8a";
    for (let i = 0; i < 256; i++) {
      const h = scale > 0 ? Math.min(1, bins[i] / scale) * element.height : 0;
      ctx.fillRect((i * element.width) / 256, element.height - h, Math.ceil(element.width / 256), h);
    }
  }, [edit.histogram, index]);
  const update = (patch: Partial<LevelRange>) => {
    const ranges = settings.ranges.map((r, i) => (i === index ? clampRange({ ...r, ...patch }) : r));
    s.updateAdjust({ adjustment: { ...edit.adjustment!, levels: { ...settings, ranges } } });
  };
  return (
    <>
      <label>Channel <select data-testid="levels-channel" value={channel} onChange={(e) => s.updateAdjust({ adjustment: { ...edit.adjustment!, levels: { ...settings, channel: e.target.value as LevelsChannel } } })}>
        {LEVELS_CHANNELS.map((c) => <option key={c} value={c}>{c}</option>)}
      </select></label>
      <canvas data-testid="levels-histogram" ref={canvas} width={256} height={100} className="histogram" />
      {FIELDS.map(([key, label]) => (
        <label key={key}>{label} <input aria-label={label} type="number" step={key === "gamma" ? 0.01 : 1} value={range[key]}
          onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) update({ [key]: v } as Partial<LevelRange>); }} /></label>
      ))}
      <label>Auto <select data-testid="levels-auto" value="" onChange={(e) => { if (e.target.value) s.autoLevels(e.target.value as LevelsAuto); }}>
        <option value="">Choose...</option>
        <option value="Contrast">Contrast</option>
        <option value="Color">Color</option>
        <option value="Neutral">Color + neutral midtones</option>
      </select></label>
      <div className="eyedroppers">
        {(["Black", "Gray", "White"] as const).map((mode) => (
          <button key={mode} data-testid={`levels-sample-${mode.toLowerCase()}`} aria-pressed={edit.sampleMode === mode}
            onClick={() => s.setAdjustSample(edit.sampleMode === mode ? null : mode)}>{mode}</button>
        ))}
      </div>
    </>
  );
}
```
The store gains `autoLevels(mode)`, which asks the engine and replaces the settings:
```ts
  autoLevels: (mode) => {
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId || !adjustEdit?.adjustment) return;
    const levels = engine.autoLevels(activeId, adjustEdit.layerId, mode);
    get().updateAdjust({ adjustment: { ...adjustEdit.adjustment, levels } });
  },
```

`app/src/panels/CurvesPanel.tsx`: a 256 x 256 canvas drawing the diagonal, the curve from `curveSamples` and the handles; pointer down picks or inserts a point, drag moves it, a right-click or Delete removes it; the two number fields edit the selected point.
```tsx
import { useEffect, useRef, useState } from "react";
import { useEditor } from "../state/store";
import { LEVELS_CHANNELS } from "../tools/levels-tools";
import { curveSamples, insertPoint, movePoint, nearestPoint, removePoint } from "../tools/curves-editor";
import type { CurvePoint, LevelsChannel } from "../engine/types";

const SIZE = 256;

export function CurvesPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  const settings = edit.adjustment!.curves;
  const index = LEVELS_CHANNELS.indexOf(settings.channel);
  const points = settings.channels[index];
  const canvas = useRef<HTMLCanvasElement>(null);
  const [selected, setSelected] = useState(0);
  const [dragging, setDragging] = useState(false);
  useEffect(() => {
    const element = canvas.current; if (!element) return;
    const ctx = element.getContext("2d")!;
    ctx.clearRect(0, 0, SIZE, SIZE);
    ctx.strokeStyle = "#3a3a3a"; ctx.beginPath(); ctx.moveTo(0, SIZE); ctx.lineTo(SIZE, 0); ctx.stroke();
    ctx.strokeStyle = "#e0e0e0"; ctx.beginPath();
    curveSamples(points, SIZE).forEach((y, x) => (x === 0 ? ctx.moveTo(x, SIZE - y) : ctx.lineTo(x, SIZE - y)));
    ctx.stroke();
    points.forEach((p, i) => { ctx.fillStyle = i === selected ? "#4da3ff" : "#e0e0e0"; ctx.fillRect(p.x - 3, SIZE - p.y - 3, 6, 6); });
  }, [points, selected]);
  const pointAt = (e: React.PointerEvent | React.MouseEvent): CurvePoint => {
    const r = canvas.current!.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * 255, y: 255 - ((e.clientY - r.top) / r.height) * 255 };
  };
  const setPoints = (next: CurvePoint[]) => {
    const channels = settings.channels.map((c, i) => (i === index ? next : c));
    s.updateAdjust({ adjustment: { ...edit.adjustment!, curves: { ...settings, channels } } });
  };
  return (
    <>
      <label>Channel <select data-testid="curves-channel" value={settings.channel} onChange={(e) => s.updateAdjust({ adjustment: { ...edit.adjustment!, curves: { ...settings, channel: e.target.value as LevelsChannel } } })}>
        {LEVELS_CHANNELS.map((c) => <option key={c} value={c}>{c}</option>)}
      </select></label>
      <canvas data-testid="curves-editor" ref={canvas} width={SIZE} height={SIZE} className="curves"
        onPointerDown={(e) => {
          const at = pointAt(e);
          const hit = nearestPoint(points, at, 8);
          if (hit === null) { const next = insertPoint(points, at); setPoints(next); setSelected(next.findIndex((p) => p.x === Math.round(at.x))); }
          else setSelected(hit);
          setDragging(true); (e.target as HTMLElement).setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => { if (dragging) setPoints(movePoint(points, selected, pointAt(e))); }}
        onPointerUp={() => setDragging(false)}
        onContextMenu={(e) => { e.preventDefault(); const hit = nearestPoint(points, pointAt(e), 8); if (hit !== null) { setPoints(removePoint(points, hit)); setSelected(0); } }} />
      <label>Input <input aria-label="Input" data-testid="curves-point-x" type="number" value={Math.round(points[selected]?.x ?? 0)}
        onChange={(e) => setPoints(movePoint(points, selected, { x: Number(e.target.value), y: points[selected].y }))} /></label>
      <label>Output <input aria-label="Output" data-testid="curves-point-y" type="number" value={Math.round(points[selected]?.y ?? 0)}
        onChange={(e) => setPoints(movePoint(points, selected, { x: points[selected].x, y: Number(e.target.value) }))} /></label>
    </>
  );
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm test`, `pnpm build`, `pnpm e2e`
Expected: all pass, including the 2 new unit files and the 5 tests in `adjust-panels.spec.ts`.

- [ ] **Step 7: Commit**

```
git add app
git commit -m "feat(app): the adjustment panel shell with Levels and Curves"
```

---

### Task 15: Hue/Saturation and the canvas eyedroppers

**Files:**
- Create: `app/src/panels/HueSaturationPanel.tsx`, `app/src/tools/hue-band.ts`
- Modify: `app/src/canvas/CanvasView.tsx` (a click samples while a panel has an eyedropper armed), `app/src/state/store.ts` (`sampleAt`)
- Test: `app/tests/unit/hue-band.test.ts`, `app/tests/e2e/adjust-panels.spec.ts` (extend)

**Interfaces:**
- `hue-band.ts`: `forward(from, to)`, `bandWeight(band, hue)`, `centeredOn(band, hue)`, `includeHue(band, hue)`, `excludeHue(band, hue)`, `setHandle(band, index, degrees)`, `hueOf(rgb: [number, number, number]): number | null` (null when too neutral to have a hue), `DEFAULT_BANDS: Record<ColorRange, HueBand>`, `COLOR_RANGES`.
- Store: `sampleAt(point: { x: number; y: number }): void` routes the armed eyedropper to the engine (Levels samples the layer, Hue/Saturation samples the composite) and updates the panel.
- Panel test ids: `hue-range`, `hue-colorize`, `hue-invert`, `hue-sample-<mode>`, `hue-band-<index>`; sliders labelled "Hue", "Saturation", "Lightness".

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/hue-band.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { DEFAULT_BANDS, bandWeight, centeredOn, excludeHue, forward, hueOf, includeHue, setHandle } from "../../src/tools/hue-band";

describe("hue bands", () => {
  it("ramps through the falloff shoulders and wraps", () => {
    const reds = DEFAULT_BANDS.Reds;
    expect(bandWeight(reds, 0)).toBe(1);
    expect(bandWeight(reds, 345)).toBe(1);
    expect(bandWeight(reds, 330)).toBeCloseTo(0.5, 3);
    expect(bandWeight(reds, 30)).toBeCloseTo(0.5, 3);
    expect(bandWeight(reds, 180)).toBe(0);
    expect(bandWeight(DEFAULT_BANDS.Master, 123)).toBe(1);
    expect(forward(350, 10)).toBe(20);
  });
  it("re-centres, widens and narrows", () => {
    const centered = centeredOn(DEFAULT_BANDS.Greens, 0);
    expect(bandWeight(centered, 0)).toBe(1);
    expect(bandWeight(centered, 120)).toBe(0);
    const wider = includeHue(centered, 240);
    expect(bandWeight(wider, 240)).toBe(1);
    expect(bandWeight(excludeHue(wider, 240), 240)).toBe(0);
  });
  it("refuses a handle move that crosses its neighbours", () => {
    const greens = DEFAULT_BANDS.Greens;
    expect(setHandle(greens, 1, 200)).toEqual(greens);
    expect(setHandle(greens, 1, 110).rangeStart).toBe(110);
  });
  it("reads a hue from a colour, and nothing from a neutral one", () => {
    expect(hueOf([1, 0, 0])).toBeCloseTo(0, 3);
    expect(hueOf([0, 0, 1])).toBeCloseTo(240, 3);
    expect(hueOf([0.5, 0.5, 0.5])).toBeNull();
  });
});
```

Add to `app/tests/e2e/adjust-panels.spec.ts`:
```ts
test("hue/saturation edits one range at a time and the eyedropper retargets a band", async ({ page }) => {
  await setup(page);
  await open(page, "Hue/Saturation");
  await page.getByLabel("Hue").fill("120");
  let settings = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.hsvSettings);
  expect(settings.adjustments.Master.hue).toBe(120);
  await page.getByTestId("hue-range").selectOption("Reds");
  expect(await page.getByLabel("Hue").inputValue()).toBe("0", "each range keeps its own values");
  await page.getByLabel("Saturation").fill("-100");
  settings = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.hsvSettings);
  expect(settings.adjustments.Reds.saturation).toBe(-100);
  expect(settings.adjustments.Master.hue).toBe(120);
  // The eyedropper re-centres the selected range on the colour under the cursor.
  await page.getByTestId("hue-sample-replace").click();
  const view = page.getByTestId("canvas-view");
  const box = (await view.boundingBox())!;
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
  const band = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.hsvSettings.bands.Reds);
  expect(band).not.toEqual({ falloffStart: 315, rangeStart: 345, rangeEnd: 15, falloffEnd: 45 });
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).canUndo).toBe(true);
});

test("colorize gives everything one hue", async ({ page }) => {
  await setup(page);
  await open(page, "Hue/Saturation");
  await page.getByTestId("hue-colorize").check();
  const settings = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment.hsvSettings);
  expect(settings.colorize).toBe(true);
  expect(settings.adjustments.Master.saturation).toBe(25, "Photoshop's starting point");
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).canUndo).toBe(true);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "hue/saturation edits"`
Expected: module not found; no panel fields.

- [ ] **Step 3: Implement hue-band.ts**

`app/src/tools/hue-band.ts` is a direct port of `HueBand` from `engine/src/adjust/settings.rs` (the same four handles, the same 350-degree cap, the same refusal rules) plus `hueOf`, which mirrors `rgb_to_hsl`'s hue with the Mac's 0.02 saturation floor for "too neutral to sample". Write `forward`, `bandWeight`, `centeredOn`, `includeHue`, `excludeHue`, `setHandle` as pure functions over `HueBand`, `DEFAULT_BANDS` with the seven Photoshop bands, and `COLOR_RANGES` (the six colours) plus `ALL_RANGES` (Master first).

- [ ] **Step 4: Implement the panel and the canvas sampling**

`app/src/panels/HueSaturationPanel.tsx`: the range picker, three sliders (Hue -180..180, or 0..360 when colorizing; Saturation -100..100, or 0..100 colorizing; Lightness -100..100), Colorize and "Invert range" checkboxes, the three eyedropper buttons, and four number fields for the selected range's band handles (`hue-band-0` to `hue-band-3`, disabled for Master and while colorizing). Every edit goes through `s.updateAdjust({ adjustment: { ...adjustment, hsvSettings: next } })`, where `next` always carries a complete `adjustments`/`bands` map so the engine sees explicit settings. Switching Colorize on replaces the settings with the colorize starting point (hue 0, saturation 25, lightness 0) as macOS does.

Store:
```ts
  /** A click on the canvas while an eyedropper is armed. Levels calibrates from the layer's own
   * pixels (as macOS's sampleLevels does); the Hue/Saturation tools read the visible composite. */
  sampleAt: (at) => {
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId || !adjustEdit?.sampleMode) return;
    const mode = adjustEdit.sampleMode;
    if (mode === "Black" || mode === "Gray" || mode === "White") {
      const levels = engine.levelsSampling(activeId, adjustEdit.layerId, adjustEdit.adjustment!.levels, at, mode);
      get().updateAdjust({ adjustment: { ...adjustEdit.adjustment!, levels } });
      return;
    }
    const rgb = engine.sampleColor(activeId, at);
    const hue = rgb ? hueOf(rgb) : null;
    if (hue === null) return;
    const settings = adjustEdit.adjustment!.hsvSettings ?? defaultHsv();
    if (settings.range === "Master" || settings.colorize) return;
    const band = settings.bands[settings.range] ?? DEFAULT_BANDS[settings.range];
    const next = mode === "replace" ? centeredOn(band, hue) : mode === "add" ? includeHue(band, hue) : excludeHue(band, hue);
    get().updateAdjust({ adjustment: { ...adjustEdit.adjustment!, hsvSettings: { ...settings, bands: { ...settings.bands, [settings.range]: next } } } });
  },
```
`CanvasView.tsx`: in the existing pointer-down effect chain, add a first handler that runs when `useEditor.getState().adjustEdit?.sampleMode` is set: it converts the point to document space, calls `sampleAt`, and stops there (no tool gesture starts). The cursor is set to `crosshair` while an eyedropper is armed.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test`, `pnpm build`, `pnpm e2e`
Expected: all pass, including 4 in `hue-band.test.ts` and the 2 new e2e tests.

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): the hue/saturation panel, colour ranges and the canvas eyedroppers"
```

---

### Task 16: The filter panel, the Image and Filter menus, and the shortcuts

**Files:**
- Create: `app/src/panels/FilterPanel.tsx`
- Modify: `app/src/panels/MenuBar.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`, `app/tests/e2e/helpers.ts` (the `clickMenu` title union gains "Filter")
- Test: `app/tests/unit/keymap.test.ts` (extend), `app/tests/e2e/filters.spec.ts`

**Interfaces:**
- `FilterPanel` renders the fields of whichever kind is open: Exposure (Exposure, Offset, Gamma), Gradient Map (two colours plus Reverse), Grain (Amount, Size, Roughness), Gaussian Blur (Radius), Motion Blur (Angle, Distance), Add Noise (Amount, Distribution, Monochromatic), Lens Correction (Remove Distortion). Every field is a number input with an `aria-label` equal to its visible label, paired with a range slider.
- Menus: Image gains `image-levels`, `image-curves`, `image-hue-saturation`, `image-exposure`, `image-gradient-map`, `image-grain`, `image-invert`; a new Filter menu between Layer and Image holds `filter-gaussian-blur`, `filter-motion-blur`, `filter-add-noise`, `filter-lens-correction`.
- Shortcuts: `levels` Ctrl+L, `curves` Ctrl+M, `hue-saturation` Ctrl+U, `invert` Ctrl+I. Every menu item and shortcut is disabled while a panel is open or `canAdjust()` is false (Invert also accepts a selected mask, which `canInvert()` reports).

- [ ] **Step 1: Write the failing tests**

Add to `app/tests/unit/keymap.test.ts`:
```ts
  it("maps the adjustment shortcuts", () => {
    expect(matchShortcut(ev("l", { ctrlKey: true }))).toBe("levels");
    expect(matchShortcut(ev("m", { ctrlKey: true }))).toBe("curves");
    expect(matchShortcut(ev("u", { ctrlKey: true }))).toBe("hue-saturation");
    expect(matchShortcut(ev("i", { ctrlKey: true }))).toBe("invert");
    expect(matchShortcut(ev("i", { ctrlKey: true, altKey: true }))).toBe("image-size", "still its own binding");
  });
```

`app/tests/e2e/filters.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const layer = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers[0]; });

test("a gaussian blur grows the layer, previews, and applies as one undo step", async ({ page }) => {
  await setup(page);
  const before = await layer(page);
  await clickMenu(page, "Filter", "filter-gaussian-blur");
  await expect(page.getByTestId("adjust-title")).toHaveText("Gaussian Blur");
  await page.getByLabel("Radius").fill("5");
  await page.getByLabel("Radius").press("Enter");
  const previewed = await layer(page);
  expect(previewed.transform.size[0]).toBeGreaterThan(before.transform.size[0]);
  expect(await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].canUndo; })).toBe(false);
  await page.getByTestId("adjust-ok").click();
  const after = await layer(page);
  expect(after.transform.size[0]).toBeGreaterThan(before.transform.size[0]);
  expect(after.transform.origin[0]).toBeLessThan(before.transform.origin[0]);
  await page.keyboard.press("Control+z");
  const undone = await layer(page);
  expect(undone.transform).toEqual(before.transform);
  expect(undone.pixelsWidth).toBe(before.pixelsWidth);
});

test("motion blur, add noise and lens correction each apply once", async ({ page }) => {
  await setup(page);
  for (const [id, label, value] of [["filter-motion-blur", "Distance", "24"], ["filter-add-noise", "Amount", "30"], ["filter-lens-correction", "Remove distortion", "-60"]] as const) {
    const before = (await layer(page)).pixelsRevision;
    await clickMenu(page, "Filter", id);
    await page.getByLabel(label).fill(value);
    await page.getByLabel(label).press("Enter");
    await page.getByTestId("adjust-ok").click();
    expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
  }
  const d = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect(d.canUndo).toBe(true);
});

test("the image menu opens every adjustment and Invert applies at once", async ({ page }) => {
  await setup(page);
  for (const [id, title] of [["image-levels", "Levels"], ["image-curves", "Curves"], ["image-hue-saturation", "Hue/Saturation"], ["image-exposure", "Exposure"], ["image-gradient-map", "Gradient Map"], ["image-grain", "Grain"]] as const) {
    await clickMenu(page, "Image", id);
    await expect(page.getByTestId("adjust-title")).toHaveText(title);
    await page.getByTestId("adjust-cancel").click();
  }
  const before = (await layer(page)).pixelsRevision;
  await clickMenu(page, "Image", "image-invert");
  await expect(page.getByTestId("adjust-panel")).toHaveCount(0, "Invert has no panel");
  expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
});

test("the shortcuts open their panels and the menu items grey out while one is open", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("Control+l");
  await expect(page.getByTestId("adjust-title")).toHaveText("Levels");
  await page.getByRole("button", { name: "Image", exact: true }).click();
  await expect(page.getByTestId("menu-image-curves")).toBeDisabled();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+m");
  await expect(page.getByTestId("adjust-title")).toHaveText("Curves");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Control+u");
  await expect(page.getByTestId("adjust-title")).toHaveText("Hue/Saturation");
  await page.keyboard.press("Escape");
  const before = (await layer(page)).pixelsRevision;
  await page.keyboard.press("Control+i");
  expect((await layer(page)).pixelsRevision).toBeGreaterThan(before);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "a gaussian blur grows"`
Expected: keymap assertions fail; no Filter menu.

- [ ] **Step 3: Implement the filter panel**

`app/src/panels/FilterPanel.tsx`: one `field` helper (label, range, step) writing back through `updateAdjust`, and a switch over the open kind. Exposure, Gradient Map and Grain edit `edit.adjustment`; the four filters edit `edit.params`.
```tsx
import { useEditor } from "../state/store";
import type { AdjustmentColor, ExposureSettings, FilterParams, GrainSettings } from "../engine/types";

function NumberField(props: { label: string; value: number; min: number; max: number; step: number; onChange(v: number): void }) {
  return (
    <label>{props.label}
      <input type="range" aria-hidden min={props.min} max={props.max} step={props.step} value={props.value}
        onChange={(e) => props.onChange(Number(e.target.value))} />
      <input aria-label={props.label} type="number" min={props.min} max={props.max} step={props.step} value={props.value}
        onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) props.onChange(v); }} />
    </label>
  );
}
const hex = (c: AdjustmentColor) => "#" + [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("");
const fromHex = (text: string): AdjustmentColor => ({ red: parseInt(text.slice(1, 3), 16) / 255, green: parseInt(text.slice(3, 5), 16) / 255, blue: parseInt(text.slice(5, 7), 16) / 255 });

export function FilterPanel() {
  const s = useEditor();
  const edit = s.adjustEdit!;
  if (edit.params) {
    const p = edit.params;
    const set = (patch: Partial<FilterParams>) => s.updateAdjust({ params: { ...p, ...patch } as FilterParams });
    if (p.filter === "GaussianBlur") return <NumberField label="Radius" value={p.radius} min={0.1} max={250} step={0.1} onChange={(radius) => set({ radius } as never)} />;
    if (p.filter === "MotionBlur") return (<>
      <NumberField label="Angle" value={p.angle} min={-90} max={90} step={1} onChange={(angle) => set({ angle } as never)} />
      <NumberField label="Distance" value={p.distance} min={1} max={2000} step={1} onChange={(distance) => set({ distance } as never)} />
    </>);
    if (p.filter === "AddNoise") return (<>
      <NumberField label="Amount" value={p.amount} min={0.1} max={400} step={0.1} onChange={(amount) => set({ amount } as never)} />
      <label>Distribution <select aria-label="Distribution" value={p.gaussian ? "Gaussian" : "Uniform"} onChange={(e) => set({ gaussian: e.target.value === "Gaussian" } as never)}>
        <option>Uniform</option><option>Gaussian</option></select></label>
      <label><input type="checkbox" aria-label="Monochromatic" checked={p.monochromatic} onChange={(e) => set({ monochromatic: e.target.checked } as never)} /> Monochromatic</label>
    </>);
    return <NumberField label="Remove distortion" value={p.distortion} min={-100} max={100} step={1} onChange={(distortion) => set({ distortion } as never)} />;
  }
  const a = edit.adjustment!;
  const setAdjustment = (patch: Partial<typeof a>) => s.updateAdjust({ adjustment: { ...a, ...patch } });
  if (a.kind === "Exposure") {
    const e: ExposureSettings = a.exposureSettings ?? { exposure: 0, offset: 0, gamma: 1 };
    const set = (patch: Partial<ExposureSettings>) => setAdjustment({ exposureSettings: { ...e, ...patch } });
    return (<>
      <NumberField label="Exposure" value={e.exposure} min={-20} max={20} step={0.01} onChange={(exposure) => set({ exposure })} />
      <NumberField label="Offset" value={e.offset} min={-0.5} max={0.5} step={0.001} onChange={(offset) => set({ offset })} />
      <NumberField label="Gamma" value={e.gamma} min={0.01} max={9.99} step={0.01} onChange={(gamma) => set({ gamma })} />
    </>);
  }
  if (a.kind === "Gradient Map") {
    const g = a.gradientMapSettings ?? { shadows: { red: 0, green: 0, blue: 0 }, highlights: { red: 1, green: 1, blue: 1 }, reversed: false };
    const set = (patch: Partial<typeof g>) => setAdjustment({ gradientMapSettings: { ...g, ...patch } });
    return (<>
      <label>Shadows <input aria-label="Shadows" type="color" value={hex(g.shadows)} onChange={(e) => set({ shadows: fromHex(e.target.value) })} /></label>
      <label>Highlights <input aria-label="Highlights" type="color" value={hex(g.highlights)} onChange={(e) => set({ highlights: fromHex(e.target.value) })} /></label>
      <label><input type="checkbox" aria-label="Reverse" checked={g.reversed} onChange={(e) => set({ reversed: e.target.checked })} /> Reverse</label>
    </>);
  }
  const grain: GrainSettings = a.grainSettings ?? { amount: 25, size: 1.5, roughness: 50, seed: 0 };
  const set = (patch: Partial<GrainSettings>) => setAdjustment({ grainSettings: { ...grain, ...patch } });
  return (<>
    <NumberField label="Amount" value={grain.amount} min={0} max={100} step={1} onChange={(amount) => set({ amount })} />
    <NumberField label="Size" value={grain.size} min={0.5} max={20} step={0.1} onChange={(size) => set({ size })} />
    <NumberField label="Roughness" value={grain.roughness} min={0} max={100} step={1} onChange={(roughness) => set({ roughness })} />
  </>);
}
```

- [ ] **Step 4: Menus and shortcuts**

`keymap.ts`: `ActionId` gains `"levels" | "curves" | "hue-saturation" | "invert"`, and `SHORTCUTS` gains
```ts
  "levels": [{ key: "l", ctrl: true }], "curves": [{ key: "m", ctrl: true }],
  "hue-saturation": [{ key: "u", ctrl: true }], "invert": [{ key: "i", ctrl: true }],
```
`useShortcuts.ts` `runAction` gains
```ts
    case "levels": s.beginAdjust({ kind: "Levels" }); break;
    case "curves": s.beginAdjust({ kind: "Curves" }); break;
    case "hue-saturation": s.beginAdjust({ kind: "Hue/Saturation" }); break;
    case "invert": invertActive(); break;
```
with `invertActive()` in `app/src/actions/layers.ts`:
```ts
/** Image > Invert: immediate, one undo step, on the mask when the mask chip is selected. */
export function invertActive(): void {
  const c = ctx(); if (!c?.active) return;
  const mask = c.s.maskSelected && c.active.hasMask;
  if (!mask && !c.active.hasPixels) return;
  c.s.run({ type: "InvertPixels", id: c.active.id, mask });
}
export function canInvert(): boolean {
  const c = ctx(); if (!c?.active || c.active.isGroup) return false;
  return (c.s.maskSelected && c.active.hasMask) || c.active.hasPixels;
}
```
`MenuBar.tsx`: a Filter menu between Layer and Image, and the Image menu's new items. Every one is `enabled: hasDoc && s.canAdjust()` (Invert uses `canInvert() && !s.adjustEdit`), so they grey out while a panel is open.
```tsx
    { title: "Filter", items: [
      { id: "filter-gaussian-blur", label: "Gaussian Blur...", run: () => s.beginAdjust({ kind: "GaussianBlur" }), enabled: s.canAdjust() },
      { id: "filter-motion-blur", label: "Motion Blur...", run: () => s.beginAdjust({ kind: "MotionBlur" }), enabled: s.canAdjust() },
      { id: "filter-add-noise", label: "Add Noise...", run: () => s.beginAdjust({ kind: "AddNoise" }), enabled: s.canAdjust() },
      { id: "filter-lens-correction", label: "Lens Correction...", run: () => s.beginAdjust({ kind: "LensCorrection" }), enabled: s.canAdjust() },
    ] },
```
and in Image, above Canvas Size:
```tsx
      { id: "image-levels", label: "Levels...", run: () => runAction("levels"), enabled: s.canAdjust() },
      { id: "image-curves", label: "Curves...", run: () => runAction("curves"), enabled: s.canAdjust() },
      { id: "image-hue-saturation", label: "Hue/Saturation...", run: () => runAction("hue-saturation"), enabled: s.canAdjust() },
      { id: "image-exposure", label: "Exposure...", run: () => s.beginAdjust({ kind: "Exposure" }), enabled: s.canAdjust() },
      { id: "image-gradient-map", label: "Gradient Map...", run: () => s.beginAdjust({ kind: "Gradient Map" }), enabled: s.canAdjust() },
      { id: "image-grain", label: "Grain...", run: () => s.beginAdjust({ kind: "Grain" }), enabled: s.canAdjust() },
      { id: "image-invert", label: s.maskSelected ? "Invert Mask" : "Invert", run: () => runAction("invert"), enabled: canInvert() && !s.adjustEdit },
      "separator",
```
`app/tests/e2e/helpers.ts`: widen `clickMenu`'s title union to include `"Filter"`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test`, `pnpm build`, `pnpm e2e`
Expected: all pass, including the 4 tests in `filters.spec.ts`.

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): the filter panel, the Image and Filter menus, and the adjustment shortcuts"
```

---

### Task 17: Adjustment layers in the layers panel, the Layer menu, and the round trip

**Files:**
- Modify: `app/src/panels/LayersList.tsx`, `app/src/panels/MenuBar.tsx`, `app/src/actions/layers.ts`, `app/src/styles.css`
- Test: `app/tests/e2e/adjust-layers.spec.ts`, `engine/tests/interop.rs` (extend)

**Interfaces:**
- `actions/layers.ts`: `addAdjustmentLayer(kind: AdjustmentKind)` (passes a fresh random seed for Grain and the default black-to-white gradient), `editAdjustmentLayer(id?)` (opens the panel on an adjustment layer), `canEditAdjustment(): boolean`.
- Layers panel: an adjustment row shows a kind glyph (`data-testid="adjustment-chip-<id>"`, `aria-label="<kind> adjustment"`) instead of the pixel chip, and double-clicking the row opens its panel rather than renaming it.
- Layer menu: six `layer-adjustment-<kind>` items ("New Levels Adjustment...", and so on) plus `layer-edit-adjustment` ("Edit Adjustment..."). A submenu would be closer to the Mac; flat items keep the existing one-level menu bar and are noted as the simplification they are.

- [ ] **Step 1: Write the failing tests**

`app/tests/e2e/adjust-layers.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, noisePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

test("a new adjustment layer appears above the active layer and never rewrites its pixels", async ({ page }) => {
  await setup(page);
  const before = (await state(page)).layers[0];
  await clickMenu(page, "Layer", "layer-adjustment-levels");
  const d = await state(page);
  expect(d.layers.length).toBe(2);
  expect(d.layers[1].adjustment.kind).toBe("Levels");
  expect(d.layers[1].hasPixels).toBe(false);
  expect(d.layers[0].pixelsRevision).toBe(before.pixelsRevision);
  await expect(page.getByTestId(`adjustment-chip-${d.layers[1].id}`)).toBeVisible();
  expect(await page.getByTestId("layer-row").nth(0).innerText()).toContain("Levels");
});

test("double-clicking an adjustment row edits it live and OK records one step", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-curves");
  const id = (await state(page)).layers[1].id;
  await page.getByTestId("layer-row").nth(0).dblclick();
  await expect(page.getByTestId("adjust-title")).toHaveText("Curves");
  const editor = page.getByTestId("curves-editor");
  const box = (await editor.boundingBox())!;
  await page.mouse.click(box.x + box.width * 0.5, box.y + box.height * 0.2);
  // The live preview goes through the plan: the document has no new undo step yet.
  expect((await state(page)).canUndo).toBe(true, "only the New Curves Adjustment step so far");
  const depthBefore = (await state(page)).undoDepth;
  await page.getByTestId("adjust-ok").click();
  expect((await state(page)).undoDepth).toBe(depthBefore + 1);
  const points = (await state(page)).layers.find((l: any) => l.id === id).adjustment.curves.channels[0];
  expect(points.length).toBe(3);
  await page.keyboard.press("Control+z");
  expect((await state(page)).layers.find((l: any) => l.id === id).adjustment.curves.channels[0].length).toBe(2);
});

test("cancelling an edit restores the settings and records nothing", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-exposure");
  const depth = (await state(page)).undoDepth;
  await page.getByTestId("layer-row").nth(0).dblclick();
  await page.getByLabel("Exposure").fill("2");
  await page.getByLabel("Exposure").press("Enter");
  await page.getByTestId("adjust-cancel").click();
  const d = await state(page);
  expect(d.undoDepth).toBe(depth);
  expect(d.layers[1].adjustment.exposureSettings.exposure).toBe(0);
});

test("an adjustment layer saves, reopens and still renders the same", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Layer", "layer-adjustment-gradient-map");
  const saved = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const before = Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[];
    const files = api.engine.savePackage(s.activeId);
    const reopened = api.engine.openPackage(files, null);
    const after = Array.from(api.engine.composite(reopened, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[];
    const manifest = JSON.parse(files.manifest);
    api.engine.closeDocument(reopened);
    return { same: before.every((v, i) => v === after[i]), version: manifest.version, adjustment: manifest.layers[1].adjustment };
  });
  expect(saved.same).toBe(true);
  expect(saved.version).toBe(7);
  expect(saved.adjustment.kind).toBe("Gradient Map");
  expect(saved.adjustment.gradientMapSettings).toBeTruthy();
  expect(saved.adjustment.exposureSettings).toBeUndefined();
});
```

Add to `engine/tests/interop.rs`:
```rust
#[test]
fn a_macos_adjustment_layer_opens_and_saves_unchanged() {
    // A v7 manifest as macOS writes it: an adjustment layer with only the keys that kind uses.
    let manifest = r#"{
  "activeLayerID": "11111111-1111-4111-8111-111111111111",
  "colorSpace": "sRGB",
  "documentID": "22222222-2222-4222-8222-222222222222",
  "format": "com.compositor.project",
  "height": 4,
  "layers": [
    {
      "adjustment": {"colorize":false,"curves":{"channel":"RGB","channels":[[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}],[{"x":0,"y":0},{"x":255,"y":255}]]},"hue":120,"kind":"Hue/Saturation","levels":{"channel":"RGB","ranges":[{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255},{"black":0,"gamma":1,"outputBlack":0,"outputWhite":255,"white":255}]},"lightness":0,"saturation":0},
      "id": "11111111-1111-4111-8111-111111111111",
      "isVisible": true,
      "name": "Hue/Saturation",
      "transform": {"origin":[0,0],"size":[4,4]}
    }
  ],
  "version": 7,
  "width": 4
}"#;
    let doc = open_package(&Package { manifest_json: manifest.to_string(), images: vec![] }).unwrap();
    let adjustment = doc.layers[0].extra.adjustment.as_ref().unwrap();
    assert_eq!(adjustment.kind, AdjustmentKind::Hsv);
    assert_eq!(adjustment.resolved_hsv().adjustment(ColorRange::Master).hue, 120.0, "the legacy scalar still drives the adjustment");
    let saved = save_package(&doc).unwrap();
    let written: serde_json::Value = serde_json::from_str(&saved.manifest_json).unwrap();
    let original: serde_json::Value = serde_json::from_str(manifest).unwrap();
    assert_eq!(written["layers"][0]["adjustment"], original["layers"][0]["adjustment"], "re-saved byte for byte");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test interop` and `pnpm e2e --grep "a new adjustment layer"`
Expected: the interop test fails only if the serde shape drifted (it should already pass after Task 1: keep it as the guard it is); the e2e fails with no menu item.

- [ ] **Step 3: Implement**

`app/src/actions/layers.ts`:
```ts
/** Layer > New <kind> Adjustment. Each Grain layer gets its own pattern, and a Gradient Map
 * starts from black to white (the Mac takes the palette, which Phase 4 adds). */
export function addAdjustmentLayer(kind: AdjustmentKind): void {
  const c = ctx(); if (!c) return;
  const seed = Math.floor(Math.random() * 0xffffffff);
  c.s.run({ type: "AddAdjustmentLayer", kind, seed, shadows: null, highlights: null });
  // The new layer is active; open its panel straight away, as macOS does.
  const created = activeLayer(useEditor.getState().documents[c.doc.id]);
  if (created?.adjustment) useEditor.getState().beginAdjust({ kind, layerId: created.id, target: "adjustmentLayer" });
}
export function editAdjustmentLayer(id?: string): void {
  const c = ctx(); if (!c) return;
  const layer = id ? c.doc.layers.find((l) => l.id === id) : c.active;
  if (!layer?.adjustment) return;
  c.s.beginAdjust({ kind: layer.adjustment.kind, layerId: layer.id, target: "adjustmentLayer" });
}
export function canEditAdjustment(): boolean { const c = ctx(); return !!c?.active?.adjustment && !c.s.adjustEdit; }
```
`MenuBar.tsx`, in the Layer menu after "Group Layers":
```tsx
      "separator",
      ...ADJUSTMENT_KINDS.map((kind) => ({
        id: `layer-adjustment-${kind.toLowerCase().replace(/[^a-z]+/g, "-")}`,
        label: `New ${kind} Adjustment...`, run: () => addAdjustmentLayer(kind), enabled: hasDoc && !s.adjustEdit,
      })),
      { id: "layer-edit-adjustment", label: "Edit Adjustment...", run: () => editAdjustmentLayer(), enabled: canEditAdjustment() },
```
(`"Hue/Saturation"` becomes `layer-adjustment-hue-saturation`, `"Gradient Map"` becomes `layer-adjustment-gradient-map`.)

`LayersList.tsx`: where the pixel chip is rendered, an adjustment layer shows its own chip instead, and the row's double-click opens the panel:
```tsx
              onDoubleClick={() => { if (l.adjustment) editAdjustmentLayer(l.id); else setRenaming({ id: l.id, name: l.name }); }}
...
              {l.adjustment
                ? <button data-testid={`adjustment-chip-${l.id}`} className="chip chip-adjustment" aria-label={`${l.adjustment.kind} adjustment`}
                    aria-pressed={l.id === doc.activeLayerId} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); }} />
                : <button data-testid={`target-pixels-${l.id}`} ... />}
```
Style `.chip-adjustment` distinctly (a half-filled circle reads as "adjustment" without a glyph, keeping the row text equal to the layer name for the existing `allInnerTexts` assertions).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`, `pnpm test`, `pnpm build`, `pnpm e2e`
Expected: all pass, including the 4 tests in `adjust-layers.spec.ts` and the interop guard.

- [ ] **Step 5: Commit**

```
git add app engine
git commit -m "feat(app): adjustment layers in the layers panel and the Layer menu"
```

---

### Task 18: Docs, spec and the 0.3.0 portable build

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-09-20-windows-port-design.md`, `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml` (workspace version), `Cargo.lock`, `app/tests/e2e/smoke.spec.ts` (the version literal)

- [ ] **Step 1: Update the docs**

README: a "Phase 3: adjustments and filters" section listing Levels (with Auto and the three eyedroppers), Curves, Hue/Saturation (seven colour ranges, colorize, band eyedroppers), Exposure, Gradient Map, Grain, Invert, Gaussian Blur, Motion Blur (both spreading past the layer's edges), Add Noise, Lens Correction, live previews that leave the document untouched until OK, and adjustment layers for the six kinds (masked, clipped, limited by an enclosing folder's mask, reopened by double-clicking the row). Add the shortcuts (Ctrl+L, Ctrl+M, Ctrl+U, Ctrl+I) and two notes: selection-limited adjustments arrive with selections in Phase 4, and an adjustment layer is never a clipping source.

Spec: section 3's Phase 3 paragraph already matches what shipped; add to section 4 a short "Adjustments" subsection naming `engine/src/adjust/` as the port of the macOS kernels and recording that Hue/Saturation is evaluated per pixel rather than through a 33-point colour cube, and that Motion Blur is an even streak rather than Core Image's tapered one. Both are rulings from this plan's Global Constraints, so the spec should carry them.

Bump the version to 0.3.0 in `package.json`, `src-tauri/tauri.conf.json` and the workspace `Cargo.toml` (`src-tauri/Cargo.toml` inherits it), and update the `0.2.0` literal in `app/tests/e2e/smoke.spec.ts`.

- [ ] **Step 2: Full verification and portable build**

```
cargo test
pnpm wasm
pnpm test
pnpm build
pnpm e2e
pnpm build:portable
```
Expected: every suite green; a zip `build-artifacts/windows-x64/Compositor-portable-0.3.0-<stamp>.zip` with its marker verified and "Smoke launch OK". (`pnpm wasm`, not `wasm:dev`: the shipped build wants the release wasm.)

- [ ] **Step 3: Commit**

```
git add README.md docs package.json src-tauri Cargo.toml Cargo.lock app
git commit -m "docs,build: phase 3 documentation, spec notes and the 0.3.0 portable build"
```

Then a human exercises the zip: open an image, Levels with Auto and an eyedropper, Curves with a dragged point, Hue/Saturation on the Reds only, Exposure, a Gradient Map, Grain, Invert, a Gaussian Blur big enough to spread past the edge, Motion Blur, Add Noise, Lens Correction, an adjustment layer masked and clipped, save, reopen, and ideally open the file in the macOS app.

---

## Self-review notes

Spec coverage, section 3 Phase 3: Levels with Auto (Tasks 2, 14), Curves (3, 14), Hue/Saturation (4, 15), Exposure (3, 16), Gradient Map (3, 16), Grain (5, 16), Invert (3, 8, 16), Gaussian Blur and Motion Blur spreading past layer edges (6, 8, 16), Add Noise (6, 16), Lens Correction (6, 16), live previews (10, 12, 13), adjustment layers for the six kinds (8, 9, 13, 17). "Limited to the selection when there is one" is carried as far as it can go without selections: every kernel takes an optional coverage mask and `blend_by_coverage` is implemented and tested (Task 7), with `None` passed everywhere until Phase 4 supplies a selection. That is a deliberate deferral, recorded in the Global Constraints.

Cross-task consistency: `LayerAdjustment`'s JSON shape is fixed in Task 1 and mirrored verbatim in `types.ts` (Task 11); `PreparedAdjustment` (Task 7) is the single definition of what a colour becomes, used by the CPU compositor (Task 9) and mirrored in GLSL (Task 13) with the tables themselves coming from the engine so only the HSL and grain arithmetic is written twice, guarded by the parity e2e; `FilterParams` is one type across the engine, the commands, the preview requests and the panel; `AdjustEdit.target` is the one flag that decides pixel preview versus plan preview, and both are cleared on the same three paths (commit, cancel, document switch); the undo names in Task 10's `action_name` are exactly the strings the Mac records.

Known simplifications: Hue/Saturation is evaluated per pixel instead of through a 33-point cube (more accurate, and the reason our tolerance is 2 levels where the Mac's tests allow 8); Motion Blur is an even streak rather than Core Image's Gaussian taper; the targeted-adjustment drag (Command-drag on the canvas to push a range's saturation) is not ported, since it needs the canvas gesture plumbing that Phase 4's tools bring; "New Adjustment Layer" is six flat menu items rather than a submenu; the Gradient Map starts black to white because the colour palette arrives with the Phase 4 picker; Remove Background and Content-Aware Fill stay out of scope (v1 excludes them).

