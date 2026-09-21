# Compositor for Windows, Phase 2 (Layers) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Full layer editing on the Phase 1 base: folders, opacity, all 13 blend modes, raster layer masks and folder masks, clipping masks, non-destructive move/scale/rotate/flip and free distort with snapping, multi-layer and folder transforms, merge, duplicate, drag reorder and nest, all rendered identically by the engine's CPU compositor and the WebGL2 renderer.

**Architecture:** The engine gains a `RenderPlan`: a serialisable description of exactly what to draw (per-layer transform or distortion corners, opacity, blend mode, a list of coverage masks, a clipping source, and clipping "stacks"). The CPU compositor and the WebGL2 renderer both execute that one plan, so they cannot drift. Live previews never mutate the document: the UI passes a `PreviewEdit` (a dragged layer, group box, mask or distortion) into `render_plan`, and the engine computes every displayed transform. Commands stay whole-document undo steps as in Phase 1.

**Tech Stack:** unchanged from Phase 1 (Rust 1.95, wasm-pack 0.15.0, Vite 6, React 18, zustand, vitest, Playwright, Tauri 2).

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md` (section 3 "Phase 2, layers", sections 4 to 8).

## Global Constraints

- Everything in the Phase 1 plan's Global Constraints still applies (limits, ASCII strings, `.comp` format v1 to v7, premultiplied RGBA8 in the engine, straight PNG on disk, 127.0.0.1 dev URLs, edge rounding of the document rect).
- Blend modes are the 13 manifest values: Normal, Multiply, Screen, Overlay, Darken, Lighten, Difference, Color Dodge, Color Burn, Hue, Saturation, Color, Luminosity. Separable modes follow the PDF/Photoshop formulas (Color Burn and Color Dodge included, not Core Graphics' variants); non-separable modes follow the PDF `SetLum`/`SetSat` definitions. Compositing in premultiplied space: `a = as + ad(1 - as)`, `C = Cs(1 - ad) + Cd(1 - as) + as ad B(Cd/ad, Cs/as)`.
- Groups are pass-through: opacity 1 and Normal only (validated on save); a folder mask multiplies the coverage of every descendant pixel layer together with the layer's own mask and enclosing folders' masks. Visibility of a hidden ancestor hides the subtree.
- A layer mask is 8-bit grey stretched over the layer's rectangle (its grid may differ from the pixel grid; 1x1 means uniform). `maskPlacement` (a `LayerTransform`) places it apart from the layer; beyond its pixels it shows its background: white or black, whichever most of its edge pixels are. `maskLinked` nil means linked. Disabled masks are kept but do not affect compositing.
- Mask placement follows the macOS rule when a layer moves from `old` to `new`: a uniform mask never gets a placement; linked, an existing placement follows (`placement.following(old, new)`); unlinked, the mask stays where it was on the document (`placement ?? old`); a placement equal to the layer's new transform collapses to none.
- Clipping (`maskSourceID`): the target's coverage is multiplied by the source's coverage, where source coverage is the source's alpha through its transform, times its opacity, times its own enabled mask, times its own clipping chain; the source's visibility and colour are ignored; chains are at most 256 deep; groups are never sources or targets.
- Clipping stacks: when a base layer (not itself clipped) is directly followed in render order by visible siblings clipped to it, the stack composites as one unit: the base drawn onto a transparent buffer, its alpha recorded, its colours made opaque where alpha > 0, the children drawn over it with their own blend modes and opacity, the base alpha restored, and the result composited with the base's blend mode under the base's folder masks.
- Free distortion: four convex, non-degenerate corners (cross-product sign test with `|cross| > 0.01`, coordinates finite and at most 1,000,000); previewed as a projective mapping of the unit square; Apply resamples pixels (and a linked mask) into the shape's whole-pixel bounds, trimmed to visible alpha, leaving an axis-aligned layer with rotation 0; bounds must be at most 30,000 per side and 100 million pixels.
- Transform drags match `TransformDrag.updated` on macOS: move (Shift locks the dominant axis); rotate around the centre (Shift snaps to 15 degrees); resize keeps the opposite handle fixed (Alt: the centre), Shift toggles ratio lock with the projection-onto-diagonal rule for corners; results are `rounded()` to whole pixels and degrees on mouse-up; invalid results keep the original. Snapping pulls to canvas edges and centre and other visible layers' upright bounds and centres within 10 screen pixels.
- Layer selection is UI state (never saved): `selectedLayerIds` plus the engine's `activeLayerId`. Commands always take explicit ids.
- Mask painting with the brush belongs to Phase 4 (the brush tool); Phase 2 masks support add (reveal all / hide all), delete, enable/disable, invert, fill, blur/feather, link/unlink, moving an unlinked mask alone, and copying a mask to another layer. This is a controller ruling on the spec's "paint ... them" wording; distorting an unlinked mask alone is also deferred.
- Every commit compiles and passes `cargo test`, `pnpm test`, `pnpm build` and `pnpm e2e`.

---

## File Structure

```
engine/src/blend.rs                  blend-mode maths (separable, non-separable, premultiplied compose)
engine/src/geometry.rs               + Affine::mul, LayerTransform::{unit_to_document, placing, following, same_placement, scale_percent, scaled_to_percent, rounded, contains}, Homography
engine/src/document.rs               + Mask::{is_linked, background, follow}, Layer::{set_mask, mask_revision}, Document::{descendants, siblings, render_ids}
engine/src/plan.rs                   RenderPlan, LayerDraw, Coverage, PlanNode, PreviewEdit, render_plan()
engine/src/compositor.rs             rewritten to execute a RenderPlan (blend, coverages, clipping, stacks, homography)
engine/src/ops/appearance.rs         opacity, blend mode
engine/src/ops/hierarchy.rs          add group, group layers, place layer, move by, duplicate, delete with bake, clipping links
engine/src/ops/transform.rs          set transform, transform group, flip layers, nudge, mask placement
engine/src/ops/distort.rs            homography warp, trim, distort commands
engine/src/ops/masks.rs              add/delete/enable/link/invert/fill/blur/copy
engine/src/ops/merge.rs              merge down / layers / group
engine/src/command.rs                + Phase 2 commands
engine/src/engine.rs                 + render_plan, composite with edit, clip_dependents, state additions
engine/tests/{blend,geometry2,plan,compositor2,hierarchy,transform,distort,masks,merge}.rs
engine-wasm/src/lib.rs               + render_plan, composite_edit, mask_pixels_ptr/len, clip_dependents
app/src/engine/types.ts              + LayerState mask fields, RenderPlan types, PreviewEdit, commands
app/src/engine/client.ts             + renderPlan, compositeEdit, maskPixels, clipDependents
app/src/canvas/gl/programs.ts        GLSL sources and program cache
app/src/canvas/gl/framebuffers.ts    view-sized FBO pool (main A/B, coverage, clip, stack A/B, base alpha)
app/src/canvas/gl/mask-textures.ts   R8 mask textures keyed by doc:layer:maskRevision
app/src/canvas/gl-renderer.ts        executes the RenderPlan
app/src/canvas/cpu-renderer.ts       passes the PreviewEdit to compositeEdit
app/src/canvas/renderer.ts           Renderer.render takes the plan + edit
app/src/tools/transform-geometry.ts  point(), corners, TransformDrag port, overlay geometry + hit test, snapping
app/src/tools/transform-session.ts   drag state machine for the move tool (move/resize/rotate/distort/duplicate)
app/src/state/store.ts               + selectedLayerIds, maskSelected, collapsed, transformEdit, snapGuides, plan cache
app/src/state/selection.ts           transformsAsGroup, groupMembers, groupBox
app/src/canvas/CanvasView.tsx        move-tool pointer handling, transform overlay drawing
app/src/canvas/overlay.ts            + transform handles, distortion outline, rotation handle
app/src/panels/LayersList.tsx        rewritten: rows, multi-select, collapse, drag/drop, mask target, context menu
app/src/panels/LayerProperties.tsx   opacity slider, blend select, mask buttons
app/src/panels/TransformInspector.tsx X, Y, Scale %, angle, sampling, Apply/Cancel for distortion
app/src/panels/MenuBar.tsx           + Layer menu
app/src/shortcuts/keymap.ts          + layer shortcuts
app/src/shortcuts/useShortcuts.ts    + layer actions
app/src/actions/layers.ts            UI-side layer actions (delete with bake prompt, group, merge, masks)
app/tests/unit/{transform-geometry,transform-session,selection,keymap}.test.ts
app/tests/e2e/{layers,transform,masks,blend}.spec.ts
```

---

### Task 1: Blend-mode maths

**Files:**
- Create: `engine/src/blend.rs`
- Modify: `engine/src/lib.rs` (add `pub mod blend; pub use blend::*;`)
- Test: `engine/tests/blend.rs`

**Interfaces:**
- Produces: `blend::separable(mode: BlendMode, cb: f32, cs: f32) -> f32` (straight channel values 0..1; Normal returns `cs`); `blend::blend_rgb(mode, backdrop: [f32;3], source: [f32;3]) -> [f32;3]` (straight colours; handles non-separable modes); `blend::compose(dst: [f32;4], src: [f32;4], mode) -> [f32;4]` (both premultiplied 0..1, returns premultiplied source-over with the mode); `blend::compose_u8(dst: &mut [u8], src: [f32;4], mode)` writes four bytes.

- [ ] **Step 1: Write the failing tests**

`engine/tests/blend.rs`:
```rust
use compositor_engine::*;

fn near(a: f32, b: f32) -> bool { (a - b).abs() < 0.02 }

#[test]
fn separable_modes_match_known_values() {
    // Backdrop 0.4 grey, source 0.8 grey (the macOS LayerAppearanceTests table).
    for (mode, expected) in [
        (BlendMode::Normal, 0.8), (BlendMode::Multiply, 0.32), (BlendMode::Screen, 0.88), (BlendMode::Overlay, 0.64),
        (BlendMode::Darken, 0.4), (BlendMode::Lighten, 0.8), (BlendMode::Difference, 0.4),
        (BlendMode::ColorDodge, 1.0), (BlendMode::ColorBurn, 0.25),
    ] {
        let out = compose([0.4, 0.4, 0.4, 1.0], [0.8, 0.8, 0.8, 1.0], mode);
        assert!(near(out[0], expected), "{mode:?}: {} vs {expected}", out[0]);
        assert!(near(out[3], 1.0));
    }
}

#[test]
fn non_separable_modes_follow_pdf_definitions() {
    let backdrop = [0.8, 0.2, 0.2]; // reddish
    let source = [0.2, 0.2, 0.8];   // bluish
    let hue = blend_rgb(BlendMode::Hue, backdrop, source);
    assert!(hue[2] > hue[0], "hue takes the source's hue (blue)");
    let lum = blend_rgb(BlendMode::Luminosity, backdrop, source);
    let l = |c: [f32; 3]| 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
    assert!(near(l(lum), l(source)), "luminosity takes the source's lum");
    assert!(lum[0] > lum[2], "but keeps the backdrop's hue");
    let sat = blend_rgb(BlendMode::Saturation, backdrop, [0.5, 0.5, 0.5]);
    assert!(near(sat[0], sat[1]) && near(sat[1], sat[2]), "zero saturation source greys the backdrop");
    let color = blend_rgb(BlendMode::Color, [0.5, 0.5, 0.5], source);
    assert!(color[2] > color[0] && near(l(color), 0.5));
}

#[test]
fn compose_is_source_over_for_normal_and_respects_alpha() {
    let out = compose([0.4, 0.0, 0.0, 0.4], [0.0, 0.0, 0.5, 0.5], BlendMode::Normal);
    assert!(near(out[3], 0.7));
    assert!(near(out[0], 0.2) && near(out[2], 0.5));
    // Multiply over a transparent backdrop is just the source.
    let out = compose([0.0; 4], [0.3, 0.3, 0.3, 0.6], BlendMode::Multiply);
    assert!(near(out[0], 0.3) && near(out[3], 0.6));
    // A translucent source multiplies only in proportion to its alpha.
    let out = compose([0.5, 0.5, 0.5, 1.0], [0.25, 0.25, 0.25, 0.5], BlendMode::Multiply);
    assert!(near(out[0], 0.5 * 0.5 + 0.5 * 0.5 * 0.5), "{}", out[0]);
    let mut bytes = [128u8, 128, 128, 255];
    compose_u8(&mut bytes, [0.25, 0.25, 0.25, 0.5], BlendMode::Multiply);
    assert!((bytes[0] as i32 - 96).abs() <= 1 && bytes[3] == 255);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test blend`
Expected: compile error, `compose` not found.

- [ ] **Step 3: Implement**

`engine/src/blend.rs`:
```rust
use crate::BlendMode;

fn clamp01(v: f32) -> f32 { v.clamp(0.0, 1.0) }

/// PDF separable blend function B(cb, cs) on straight (unpremultiplied) channel values.
pub fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::Overlay => hard_light(cs, cb),
        BlendMode::Darken => cb.min(cs),
        BlendMode::Lighten => cb.max(cs),
        BlendMode::Difference => (cb - cs).abs(),
        BlendMode::ColorDodge => if cb <= 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) },
        BlendMode::ColorBurn => if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) },
        // Non-separable modes are handled by blend_rgb; per channel they fall back to Normal.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => cs,
    }
}

fn hard_light(cb: f32, cs: f32) -> f32 {
    if cs <= 0.5 { cb * 2.0 * cs } else { let s = 2.0 * cs - 1.0; cb + s - cb * s }
}

fn lum(c: [f32; 3]) -> f32 { 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2] }

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = lum(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 { for v in &mut out { *v = l + (*v - l) * l / (l - n).max(1e-6); } }
    if x > 1.0 { for v in &mut out { *v = l + (*v - l) * (1.0 - l) / (x - l).max(1e-6); } }
    out
}

fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - lum(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn sat(c: [f32; 3]) -> f32 { c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]) }

fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|&a, &b| c[a].partial_cmp(&c[b]).unwrap_or(std::cmp::Ordering::Equal));
    let (imin, imid, imax) = (idx[0], idx[1], idx[2]);
    let mut out = [0.0f32; 3];
    let range = c[imax] - c[imin];
    if range > 0.0 {
        out[imid] = (c[imid] - c[imin]) * s / range;
        out[imax] = s;
    }
    out[imin] = 0.0;
    out
}

/// Blends straight RGB colours (0..1) with any mode.
pub fn blend_rgb(mode: BlendMode, backdrop: [f32; 3], source: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(source, sat(backdrop)), lum(backdrop)),
        BlendMode::Saturation => set_lum(set_sat(backdrop, sat(source)), lum(backdrop)),
        BlendMode::Color => set_lum(source, lum(backdrop)),
        BlendMode::Luminosity => set_lum(backdrop, lum(source)),
        _ => [separable(mode, backdrop[0], source[0]), separable(mode, backdrop[1], source[1]), separable(mode, backdrop[2], source[2])],
    }
}

/// Source-over with a blend mode, both premultiplied RGBA (0..1), result premultiplied.
pub fn compose(dst: [f32; 4], src: [f32; 4], mode: BlendMode) -> [f32; 4] {
    let ad = dst[3]; let a_s = src[3];
    if a_s <= 0.0 { return dst; }
    let out_a = a_s + ad * (1.0 - a_s);
    if mode == BlendMode::Normal || ad <= 0.0 {
        return [src[0] + dst[0] * (1.0 - a_s), src[1] + dst[1] * (1.0 - a_s), src[2] + dst[2] * (1.0 - a_s), out_a];
    }
    let cb = [dst[0] / ad, dst[1] / ad, dst[2] / ad];
    let cs = [src[0] / a_s, src[1] / a_s, src[2] / a_s];
    let b = blend_rgb(mode, cb, cs);
    let mut out = [0.0f32; 4];
    for i in 0..3 {
        out[i] = clamp01(src[i] * (1.0 - ad) + dst[i] * (1.0 - a_s) + a_s * ad * clamp01(b[i]));
    }
    out[3] = out_a;
    out
}

pub fn compose_u8(dst: &mut [u8], src: [f32; 4], mode: BlendMode) {
    let d = [dst[0] as f32 / 255.0, dst[1] as f32 / 255.0, dst[2] as f32 / 255.0, dst[3] as f32 / 255.0];
    let out = compose(d, src, mode);
    for i in 0..4 { dst[i] = (out[i] * 255.0).round().clamp(0.0, 255.0) as u8; }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test blend`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): blend-mode maths for all 13 modes in premultiplied space"
```

---

### Task 2: Transform placement maths, mask placement rule, homography

**Files:**
- Modify: `engine/src/geometry.rs`, `engine/src/document.rs`
- Test: `engine/tests/geometry2.rs`

**Interfaces:**
- Produces on `Affine`: `mul(self, other) -> Affine` (matrix product: apply `other` first, then `self`; identical to `then`), `unit_to_document` on `LayerTransform` (= `pixel_to_document(1, 1)`).
- `LayerTransform::placing(&self, map: Affine) -> LayerTransform` (port of Swift `placing`), `following(&self, old: &LayerTransform, new: &LayerTransform) -> LayerTransform`, `same_placement(&self, other: &LayerTransform) -> bool` (equal ignoring sampling), `scale_percent(&self, pixel: Size) -> f64`, `scaled_to_percent(&self, percent: f64, pixel: Size) -> LayerTransform`, `rounded(&self) -> LayerTransform`, `contains(&self, p: Point) -> bool`.
- `Homography { m: [[f64;3];3] }` with `Homography::unit_to(corners: &[Point;4]) -> Homography` (unit square to corners, Swift `homography` formula), `apply(&self, p: Point) -> Point`, `invert(&self) -> Option<Homography>`, `is_usable(corners: &[Point;4]) -> bool`, `corners_of(t: &LayerTransform) -> [Point;4]` (handle order TL, TR, BR, BL via `t.point`), `carried(placement: &LayerTransform, by: &LayerTransform, to: &[Point;4]) -> [Point;4]`.
- `Mask::is_linked(&self) -> bool` (`linked != Some(false)`), `Mask::background(&self) -> u8` (255 or 0 by edge majority), `Mask::follow(&self, old: &LayerTransform, new: &LayerTransform) -> Option<LayerTransform>` (the placement rule), `Mask::is_uniform(&self) -> bool` (1x1).
- `Layer::mask_revision: u64` bumped by `Layer::set_mask(Option<Mask>)` (and every mutation path in later tasks uses `set_mask` or `mask_mut()` which bumps too); `Layer::mask_mut(&mut self) -> Option<&mut Mask>` bumps the revision.
- `Document::descendants(&self, id) -> Vec<Uuid>`, `Document::siblings(&self, parent: Option<Uuid>) -> Vec<Uuid>` (in array order), `Document::visible_ids(&self) -> HashSet<Uuid>` (effective visibility incl. groups), `Document::render_ids(&self) -> Vec<Uuid>` (visible pixel layers, groups excluded, array order).

- [ ] **Step 1: Write the failing tests**

`engine/tests/geometry2.rs`:
```rust
use compositor_engine::*;

fn t(x: f64, y: f64, w: f64, h: f64, rot: f64) -> LayerTransform {
    let mut t = LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h });
    t.rotation = rot; t
}
fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }

#[test]
fn following_a_plain_move_translates() {
    let mask = t(10.0, 10.0, 50.0, 20.0, 15.0);
    let moved = mask.following(&t(0.0, 0.0, 100.0, 80.0, 0.0), &t(30.0, -5.0, 100.0, 80.0, 0.0));
    assert!(near(moved.origin.x, 40.0) && near(moved.origin.y, 5.0) && near(moved.rotation, 15.0));
}

#[test]
fn following_a_scale_and_rotation_keeps_relative_placement() {
    let old = t(0.0, 0.0, 100.0, 50.0, 0.0);
    let new = t(20.0, 20.0, 200.0, 100.0, 90.0);
    let inner = t(25.0, 0.0, 50.0, 50.0, 0.0); // the right half of `old`
    let moved = inner.following(&old, &new);
    // The inner box's centre (50, 25) is at unit (0.5, 0.5) of old, so it lands on new's centre.
    let c = moved.center();
    assert!(near(c.x, 120.0) && near(c.y, 70.0), "{c:?}");
    assert!(near(moved.size.width, 100.0) && near(moved.size.height, 100.0), "{:?}", moved.size);
    assert!(near(moved.rotation, 90.0), "{}", moved.rotation);
    assert!(!moved.flip_y);
}

#[test]
fn placing_keeps_horizontal_flip_and_nearest_turn() {
    let mut base = t(0.0, 0.0, 10.0, 10.0, 350.0);
    base.flip_x = true;
    let map = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 10.0, height: 10.0 }).unit_to_document();
    let placed = base.placing(map);
    assert!(placed.flip_x && near(placed.rotation, 360.0), "{}", placed.rotation);
}

#[test]
fn scale_percent_and_rounding() {
    let mut s = t(0.0, 0.0, 200.0, 300.0, 30.0);
    let pixel = Size { width: 100.0, height: 100.0 };
    assert!(near(s.scale_percent(pixel), 200.0));
    let half = s.scaled_to_percent(50.0, pixel);
    assert!(near(half.size.width, 50.0) && near(half.size.height, 50.0));
    assert!(near(half.center().x, s.center().x) && near(half.center().y, s.center().y) && near(half.rotation, 30.0));
    s.origin = Point { x: 1.4, y: -2.6 }; s.rotation = 29.6; s.size.width = 0.3;
    let r = s.rounded();
    assert_eq!((r.origin.x, r.origin.y, r.rotation, r.size.width), (1.0, -3.0, 30.0, 1.0));
    assert!(t(100.0, 200.0, 100.0, 50.0, 90.0).contains(Point { x: 150.0, y: 265.0 }));
    assert!(!t(100.0, 200.0, 100.0, 50.0, 90.0).contains(Point { x: 190.0, y: 225.0 }));
}

#[test]
fn homography_hits_corners_and_rejects_twisted_shapes() {
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let h = Homography::unit_to(&shape);
    for (u, c) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].iter().zip(shape.iter()) {
        let p = h.apply(Point { x: u.0, y: u.1 });
        assert!(near(p.x, c.x) && near(p.y, c.y), "{u:?} -> {p:?}");
    }
    let inv = h.invert().unwrap();
    let back = inv.apply(Point { x: 45.0, y: 20.0 });
    let fwd = h.apply(back);
    assert!(near(fwd.x, 45.0) && near(fwd.y, 20.0));
    assert!(Homography::is_usable(&shape));
    assert!(!Homography::is_usable(&[shape[0], shape[2], shape[1], shape[3]]));
    assert!(!Homography::is_usable(&[shape[0], shape[0], shape[2], shape[3]]));
}

#[test]
fn mask_placement_rule() {
    let old = t(0.0, 0.0, 400.0, 200.0, 0.0);
    let new = t(100.0, 0.0, 400.0, 200.0, 0.0);
    let pixels = |w, h| GrayRaster::from_bytes(w, h, vec![255; (w * h) as usize]);
    let linked = Mask { pixels: pixels(4, 2), enabled: true, placement: None, linked: None };
    assert_eq!(linked.follow(&old, &new), None, "a linked mask covering its layer keeps covering it");
    let unlinked = Mask { pixels: pixels(4, 2), enabled: true, placement: None, linked: Some(false) };
    assert_eq!(unlinked.follow(&old, &new), Some(old), "an unlinked mask stays on the canvas");
    let placed = Mask { pixels: pixels(4, 2), enabled: true, placement: Some(t(50.0, 0.0, 400.0, 200.0, 0.0)), linked: None };
    let moved = placed.follow(&old, &new).unwrap();
    assert!(near(moved.origin.x, 150.0));
    let uniform = Mask { pixels: pixels(1, 1), enabled: true, placement: None, linked: Some(false) };
    assert_eq!(uniform.follow(&old, &new), None, "a uniform mask looks the same anywhere");
    let back = Mask { pixels: pixels(4, 2), enabled: true, placement: Some(new), linked: Some(false) };
    assert_eq!(back.follow(&old, &new), None, "a placement equal to the layer collapses");
}

#[test]
fn mask_background_is_the_edge_majority() {
    let mut data = vec![0u8; 16];
    for i in [0, 1, 2, 3, 4, 7, 8, 11, 12, 13, 14] { data[i] = 255; } // 11 of 12 edge pixels white
    assert_eq!(Mask { pixels: GrayRaster::from_bytes(4, 4, data.clone()), enabled: true, placement: None, linked: None }.background(), 255);
    for v in &mut data { *v = 0; }
    assert_eq!(Mask { pixels: GrayRaster::from_bytes(4, 4, data), enabled: true, placement: None, linked: None }.background(), 0);
}

#[test]
fn document_hierarchy_helpers() {
    let mut d = Document::new(10, 10);
    let mut g = Layer::blank("Folder", d.size()); g.is_group = true;
    let mut a = Layer::blank("A", d.size()); a.parent_id = Some(g.id);
    let mut b = Layer::with_pixels("B", Raster::new_transparent(2, 2), Point { x: 0.0, y: 0.0 }); b.parent_id = Some(g.id);
    let c = Layer::with_pixels("C", Raster::new_transparent(2, 2), Point { x: 0.0, y: 0.0 });
    let (gid, aid, bid, cid) = (g.id, a.id, b.id, c.id);
    d.layers = vec![g, a, b, c];
    assert_eq!(d.descendants(gid), vec![aid, bid]);
    assert_eq!(d.siblings(Some(gid)), vec![aid, bid]);
    assert_eq!(d.siblings(None), vec![gid, cid]);
    assert_eq!(d.render_ids(), vec![aid, bid, cid]);
    d.layer_mut(gid).unwrap().visible = false;
    assert_eq!(d.render_ids(), vec![cid]);
    assert!(!d.visible_ids().contains(&aid) && d.visible_ids().contains(&cid));
    let mut l = d.layer_mut(cid).unwrap().clone();
    let before = l.mask_revision;
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: true, placement: None, linked: None }));
    assert_eq!(l.mask_revision, before + 1);
    l.mask_mut().unwrap().enabled = false;
    assert_eq!(l.mask_revision, before + 2);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test geometry2`
Expected: compile errors.

- [ ] **Step 3: Implement geometry additions**

Append to `engine/src/geometry.rs`:
```rust
impl Affine {
    /// Matrix product: `self` applied after `other` (same as `then`, named for readability in chains).
    pub fn mul(self, other: Affine) -> Affine { self.then(other) }
}

impl LayerTransform {
    /// The unit square (0..1, y down) mapped where this transform places a layer.
    pub fn unit_to_document(&self) -> Affine { self.pixel_to_document(1, 1) }

    /// A transform placing the unit square as `map` does: a rotated, maybe flipped rectangle (shear is dropped).
    /// Keeps this transform's horizontal flip and the rotation nearest this one's, and its sampling.
    pub fn placing(&self, map: Affine) -> LayerTransform {
        let sign = if self.flip_x { -1.0 } else { 1.0 };
        let angle = (map.b * sign).atan2(map.a * sign);
        let along = -map.c * angle.sin() + map.d * angle.cos();
        let middle = map.apply(Point { x: 0.5, y: 0.5 });
        let mut result = *self;
        result.size = Size { width: (map.a * map.a + map.b * map.b).sqrt(), height: along.abs() };
        let degrees = angle * 180.0 / std::f64::consts::PI;
        result.rotation = degrees + ((self.rotation - degrees) / 360.0).round() * 360.0;
        result.flip_y = along < 0.0;
        result.origin = Point { x: middle.x - result.size.width / 2.0, y: middle.y - result.size.height / 2.0 };
        result
    }

    /// This placement carried along as a layer moves from `old` to `new`.
    pub fn following(&self, old: &LayerTransform, new: &LayerTransform) -> LayerTransform {
        if old.same_placement(new) { return *self; }
        if old.size == new.size && old.rotation == new.rotation && old.flip_x == new.flip_x && old.flip_y == new.flip_y {
            let mut moved = *self;
            moved.origin.x += new.origin.x - old.origin.x;
            moved.origin.y += new.origin.y - old.origin.y;
            return moved;
        }
        let Some(old_inv) = old.unit_to_document().invert() else { return *self; };
        // Apply self's unit map, then old's inverse, then new's map.
        let map = new.unit_to_document().then(old_inv).then(self.unit_to_document());
        self.placing(map)
    }

    /// The same place on the document, whatever the sampling.
    pub fn same_placement(&self, other: &LayerTransform) -> bool {
        self.origin == other.origin && self.size == other.size && self.rotation == other.rotation
            && self.flip_x == other.flip_x && self.flip_y == other.flip_y
    }
    /// Width as a percentage of the pixel size it places (100% draws pixels 1:1).
    pub fn scale_percent(&self, pixel: Size) -> f64 { self.size.width / pixel.width.max(1.0) * 100.0 }
    /// Both sides set to `percent` of `pixel`, keeping the centre, rotation and flips.
    pub fn scaled_to_percent(&self, percent: f64, pixel: Size) -> LayerTransform {
        let center = self.center();
        let mut result = *self;
        result.size = Size { width: pixel.width * percent / 100.0, height: pixel.height * percent / 100.0 };
        result.origin = Point { x: center.x - result.size.width / 2.0, y: center.y - result.size.height / 2.0 };
        result
    }
    /// Whole pixels and whole degrees, what dragging leaves behind.
    pub fn rounded(&self) -> LayerTransform {
        let mut r = *self;
        r.origin = Point { x: self.origin.x.round(), y: self.origin.y.round() };
        r.size = Size { width: self.size.width.round().max(1.0), height: self.size.height.round().max(1.0) };
        r.rotation = self.rotation.round();
        r
    }
    pub fn contains(&self, p: Point) -> bool {
        let c = self.center();
        let (s, co) = self.radians().sin_cos();
        let x = p.x - c.x; let y = p.y - c.y;
        (x * co + y * s).abs() <= self.size.width / 2.0 && (-x * s + y * co).abs() <= self.size.height / 2.0
    }
}

/// A projective map of the plane, row-major 3x3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Homography { pub m: [[f64; 3]; 3] }

impl Homography {
    /// Maps the unit square (0,0),(1,0),(1,1),(0,1) onto `c` (handle order TL, TR, BR, BL).
    pub fn unit_to(c: &[Point; 4]) -> Homography {
        let sx = c[0].x - c[1].x + c[2].x - c[3].x;
        let sy = c[0].y - c[1].y + c[2].y - c[3].y;
        let (mut g, mut h) = (0.0, 0.0);
        if sx.abs() > 1e-9 || sy.abs() > 1e-9 {
            let dx1 = c[1].x - c[2].x; let dx2 = c[3].x - c[2].x;
            let dy1 = c[1].y - c[2].y; let dy2 = c[3].y - c[2].y;
            let den = dx1 * dy2 - dx2 * dy1;
            if den.abs() > 1e-12 {
                g = (sx * dy2 - dx2 * sy) / den;
                h = (dx1 * sy - sx * dy1) / den;
            }
        }
        let a = c[1].x - c[0].x + g * c[1].x; let b = c[3].x - c[0].x + h * c[3].x;
        let d = c[1].y - c[0].y + g * c[1].y; let e = c[3].y - c[0].y + h * c[3].y;
        Homography { m: [[a, b, c[0].x], [d, e, c[0].y], [g, h, 1.0]] }
    }
    pub fn apply(&self, p: Point) -> Point {
        let m = &self.m;
        let w = m[2][0] * p.x + m[2][1] * p.y + m[2][2];
        let w = if w.abs() < 1e-12 { 1e-12 } else { w };
        Point { x: (m[0][0] * p.x + m[0][1] * p.y + m[0][2]) / w, y: (m[1][0] * p.x + m[1][1] * p.y + m[1][2]) / w }
    }
    pub fn invert(&self) -> Option<Homography> {
        let m = &self.m;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if det.abs() < 1e-12 || !det.is_finite() { return None; }
        let inv = |r: usize, c: usize| -> f64 {
            let (r1, r2) = ((r + 1) % 3, (r + 2) % 3); let (c1, c2) = ((c + 1) % 3, (c + 2) % 3);
            (m[c1][r1] * m[c2][r2] - m[c1][r2] * m[c2][r1]) / det
        };
        Some(Homography { m: [[inv(0, 0), inv(0, 1), inv(0, 2)], [inv(1, 0), inv(1, 1), inv(1, 2)], [inv(2, 0), inv(2, 1), inv(2, 2)]] })
    }
    /// Four finite corners making a convex, non-degenerate shape.
    pub fn is_usable(c: &[Point; 4]) -> bool {
        if !c.iter().all(|p| p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 1_000_000.0 && p.y.abs() <= 1_000_000.0) { return false; }
        let mut sign = 0.0;
        for i in 0..4 {
            let a = c[i]; let b = c[(i + 1) % 4]; let d = c[(i + 2) % 4];
            let cross = (b.x - a.x) * (d.y - b.y) - (b.y - a.y) * (d.x - b.x);
            if cross.abs() <= 0.01 { return false; }
            if sign == 0.0 { sign = if cross < 0.0 { -1.0 } else { 1.0 }; } else if (cross < 0.0) != (sign < 0.0) { return false; }
        }
        true
    }
    /// A transform's corners in handle order (unit corners through `point`, flips not applied).
    pub fn corners_of(t: &LayerTransform) -> [Point; 4] {
        [t.point(Point { x: 0.0, y: 0.0 }), t.point(Point { x: 1.0, y: 0.0 }), t.point(Point { x: 1.0, y: 1.0 }), t.point(Point { x: 0.0, y: 1.0 })]
    }
    /// Where `placement`'s corners land when the perspective taking `by`'s corners to `to` is applied around it too.
    pub fn carried(placement: &LayerTransform, by: &LayerTransform, to: &[Point; 4]) -> [Point; 4] {
        let c = by.center();
        // to_unit: translate(-0.5,-0.5) . scale(size) . rotate . translate(center), then inverted.
        let forward = Affine::translation(c.x, c.y).then(Affine::rotation(by.radians()))
            .then(Affine::scale(by.size.width, by.size.height)).then(Affine::translation(-0.5, -0.5));
        let to_unit = forward.invert().unwrap_or(Affine::IDENTITY);
        let map = Homography::unit_to(to);
        let corners = Homography::corners_of(placement);
        [map.apply(to_unit.apply(corners[0])), map.apply(to_unit.apply(corners[1])), map.apply(to_unit.apply(corners[2])), map.apply(to_unit.apply(corners[3]))]
    }
}
```

Note on `carried`: the Swift version builds `toUnit` as `translate(-0.5,-0.5).concat(scale).concat(rotate).concat(translate(center))` and inverts it; concatenation applies left to right, so a unit point is translated by (-0.5, -0.5), scaled, rotated, then translated to the centre. In this crate `x.then(inner)` applies `inner` first, so the chain above reads right to left with the same effect.

- [ ] **Step 4: Implement document additions**

In `engine/src/document.rs`, add `pub mask_revision: u64` to `Layer` (initialised to 1 in `base` and `from_record`), and:
```rust
impl Mask {
    pub fn is_linked(&self) -> bool { self.linked != Some(false) }
    pub fn is_uniform(&self) -> bool { self.pixels.width == 1 && self.pixels.height == 1 }
    /// What the mask shows beyond its pixels: white or black, whichever most of its edge is.
    pub fn background(&self) -> u8 {
        let (w, h) = (self.pixels.width as usize, self.pixels.height as usize);
        let d = self.pixels.bytes();
        let (mut total, mut count) = (0u64, 0u64);
        for y in 0..h { for x in 0..w {
            if y == 0 || y == h - 1 || x == 0 || x == w - 1 { total += d[y * w + x] as u64; count += 1; }
        }}
        if count == 0 || total * 2 >= count * 255 { 255 } else { 0 }
    }
    /// Where the mask sits once its layer moves from `old` to `new` (the macOS placement rule).
    pub fn follow(&self, old: &LayerTransform, new: &LayerTransform) -> Option<LayerTransform> {
        if self.is_uniform() { return None; }
        let moved = if self.is_linked() { self.placement.map(|p| p.following(old, new)) } else { Some(self.placement.unwrap_or(*old)) };
        moved.filter(|m| !m.same_placement(new))
    }
}

impl Layer {
    pub fn set_mask(&mut self, mask: Option<Mask>) { self.mask = mask; self.mask_revision += 1; }
    pub fn mask_mut(&mut self) -> Option<&mut Mask> { self.mask_revision += 1; self.mask.as_mut() }
    pub fn has_pixels(&self) -> bool { self.pixels.is_some() }
}

impl Document {
    pub fn descendants(&self, id: Uuid) -> Vec<Uuid> {
        let mut result = Vec::new();
        let mut pending = vec![id];
        while let Some(parent) = pending.pop() {
            for l in &self.layers {
                if l.parent_id == Some(parent) && !result.contains(&l.id) { result.push(l.id); pending.push(l.id); }
            }
        }
        // Array order, so callers can rely on bottom-to-top.
        self.layers.iter().filter(|l| result.contains(&l.id)).map(|l| l.id).collect()
    }
    pub fn siblings(&self, parent: Option<Uuid>) -> Vec<Uuid> {
        self.layers.iter().filter(|l| l.parent_id == parent).map(|l| l.id).collect()
    }
    /// Layers whose every ancestor is visible (groups included).
    pub fn visible_ids(&self) -> std::collections::HashSet<Uuid> {
        let by_id: std::collections::HashMap<Uuid, &Layer> = self.layers.iter().map(|l| (l.id, l)).collect();
        self.layers.iter().filter(|layer| {
            let mut node = Some(*layer); let mut steps = 0;
            while let Some(n) = node { if !n.visible || steps > crate::MAX_NESTING { return false; } steps += 1; node = n.parent_id.and_then(|p| by_id.get(&p).copied()); }
            true
        }).map(|l| l.id).collect()
    }
    /// Visible pixel layers in array order, groups excluded: what the compositor draws.
    pub fn render_ids(&self) -> Vec<Uuid> {
        let visible = self.visible_ids();
        self.layers.iter().filter(|l| !l.is_group && visible.contains(&l.id)).map(|l| l.id).collect()
    }
}
```
`Layer::clone` derives already; keep `PartialEq` deriving `mask_revision` too (undo snapshots restore the old revision, which is fine because the texture cache keys on `(id, revision)` and old revisions map to the same old pixels).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass (geometry2 8 tests plus the existing suite).

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): placement maths, mask placement rule, homography and hierarchy helpers"
```

---

### Task 3: The render plan

**Files:**
- Create: `engine/src/plan.rs`
- Modify: `engine/src/lib.rs` (add `pub mod plan; pub use plan::*;`)
- Test: `engine/tests/plan.rs`

**Interfaces:**
- Produces (all `Serialize`, camelCase, UUIDs uppercase):
  - `Coverage { layer_id: Uuid, mask_revision: u64, placement: LayerTransform, corners: Option<[Point;4]>, width: u32, height: u32, background: u8, nearest: bool }`: one grey mask placed on the document; `corners` means the mask grid is warped by the same distortion as its layer.
  - `LayerDraw { id: Uuid, transform: LayerTransform, corners: Option<[Point;4]>, pixels_width, pixels_height, pixels_revision, opacity: f64, blend: BlendMode, coverages: Vec<Coverage>, clip: Option<Uuid> }`.
  - `PlanNode` = `Layer { draw }` | `Stack { base: LayerDraw, children: Vec<LayerDraw>, folder_coverages: Vec<Coverage> }` (serde tag `kind`, values `"layer"` and `"stack"`). In a stack, `base.coverages` and each child's `coverages` hold only their own masks; `folder_coverages` are the base's enclosing folder masks; children carry `clip: None`.
  - `RenderPlan { nodes: Vec<PlanNode>, sources: Vec<LayerDraw> }`: `sources` holds one draw per clipping source (own mask only, own clip kept, visibility ignored), including sources of sources.
  - `PreviewEdit` (`Deserialize`, tag `kind`): `Layer { id, draft: LayerTransform, corners: Option<[Point;4]> }`, `Group { ids: Vec<Uuid>, box: LayerTransform, draft: LayerTransform, corners: Option<[Point;4]> }` (the JSON field is `box`; in Rust name it `bounds` with `#[serde(rename = "box")]`), `Mask { id, draft: LayerTransform }`.
  - `displayed_transform(layer: &Layer, edit: Option<&PreviewEdit>) -> (LayerTransform, Option<[Point;4]>)`, `displayed_mask_placement(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<LayerTransform>`.
  - `render_plan(doc: &Document, edit: Option<&PreviewEdit>) -> RenderPlan`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/plan.rs`:
```rust
use compositor_engine::*;
use uuid::Uuid;

fn px(name: &str) -> Layer { Layer::with_pixels(name, Raster::new_transparent(4, 4), Point { x: 0.0, y: 0.0 }) }
fn mask(w: u32, h: u32, v: u8) -> Mask { Mask { pixels: GrayRaster::from_bytes(w, h, vec![v; (w * h) as usize]), enabled: true, placement: None, linked: None } }
fn draws(plan: &RenderPlan) -> Vec<Uuid> {
    plan.nodes.iter().flat_map(|n| match n { PlanNode::Layer { draw } => vec![draw.id], PlanNode::Stack { base, children, .. } => std::iter::once(base.id).chain(children.iter().map(|c| c.id)).collect() }).collect()
}

#[test]
fn plain_layers_in_order_with_folder_and_own_coverages() {
    let mut d = Document::new(10, 10);
    let mut outer = Layer::blank("Outer", d.size()); outer.is_group = true; outer.set_mask(Some(mask(1, 1, 0)));
    let mut inner = Layer::blank("Inner", d.size()); inner.is_group = true; inner.parent_id = Some(outer.id);
    let mut a = px("A"); a.parent_id = Some(inner.id); a.set_mask(Some(mask(2, 2, 128)));
    let b = px("B");
    let (oid, aid, bid) = (outer.id, a.id, b.id);
    d.layers = vec![outer, inner, a, b];
    let plan = render_plan(&d, None);
    assert_eq!(draws(&plan), vec![aid, bid]);
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.coverages.len(), 2, "own mask then the outer folder's mask");
    assert_eq!(draw.coverages[0].layer_id, aid);
    assert_eq!((draw.coverages[0].width, draw.coverages[0].height), (2, 2));
    assert_eq!(draw.coverages[1].layer_id, oid);
    assert_eq!(draw.coverages[1].background, 0);
    assert_eq!(draw.coverages[1].placement, d.layer(oid).unwrap().transform);
    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!() };
    assert!(draw.coverages.is_empty());
    assert!(plan.sources.is_empty());
}

#[test]
fn disabled_masks_and_hidden_layers_are_left_out() {
    let mut d = Document::new(10, 10);
    let mut a = px("A"); a.set_mask(Some(Mask { enabled: false, ..mask(2, 2, 0) }));
    let mut b = px("B"); b.visible = false;
    let aid = a.id;
    d.layers = vec![a, b];
    let plan = render_plan(&d, None);
    assert_eq!(draws(&plan), vec![aid]);
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert!(draw.coverages.is_empty());
}

#[test]
fn clipping_stack_and_non_stack_sources() {
    let mut d = Document::new(10, 10);
    let base = px("Base");
    let mut c1 = px("C1"); c1.mask_source_id = Some(base.id);
    let mut c2 = px("C2"); c2.mask_source_id = Some(base.id); c2.blend_mode = BlendMode::Multiply;
    let mut hidden_src = px("Hidden"); hidden_src.visible = false;
    let mut t = px("T"); t.mask_source_id = Some(hidden_src.id);
    let (bid, c1id, c2id, hid, tid) = (base.id, c1.id, c2.id, hidden_src.id, t.id);
    d.layers = vec![t, hidden_src, base, c1, c2];
    let plan = render_plan(&d, None);
    // T draws clipped to the hidden source (not a stack: the source is above it and hidden).
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!("first node: {:?}", plan.nodes[0]) };
    assert_eq!(draw.id, tid);
    assert_eq!(draw.clip, Some(hid));
    // Base + C1 + C2 form a stack.
    let PlanNode::Stack { base, children, .. } = &plan.nodes[1] else { panic!("second node: {:?}", plan.nodes[1]) };
    assert_eq!(base.id, bid);
    assert_eq!(children.iter().map(|c| c.id).collect::<Vec<_>>(), vec![c1id, c2id]);
    assert_eq!(children[1].blend, BlendMode::Multiply);
    assert!(children.iter().all(|c| c.clip.is_none()));
    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.sources.iter().map(|s| s.id).collect::<Vec<_>>(), vec![hid], "hidden source still supplies coverage");
}

#[test]
fn a_hidden_child_breaks_the_stack_and_chains_add_sources() {
    let mut d = Document::new(10, 10);
    let base = px("Base");
    let mut c1 = px("C1"); c1.mask_source_id = Some(base.id); c1.visible = false;
    let mut c2 = px("C2"); c2.mask_source_id = Some(base.id);
    let mut s2 = px("S2");
    s2.mask_source_id = Some(c2.id);
    let mut t = px("T"); t.mask_source_id = Some(s2.id);
    let (bid, c2id, s2id, tid) = (base.id, c2.id, s2.id, t.id);
    d.layers = vec![base, c1, c2, s2, t];
    let plan = render_plan(&d, None);
    // C1 hidden: the stack is base + C2 (C2 directly follows base in render order).
    let PlanNode::Stack { base, children, .. } = &plan.nodes[0] else { panic!() };
    assert_eq!((base.id, children.len()), (bid, 1));
    assert_eq!(children[0].id, c2id);
    // S2 is clipped to C2 (a stacked child, so S2 is not part of the stack); T clips to S2 which clips to C2.
    let ids: Vec<Uuid> = plan.sources.iter().map(|s| s.id).collect();
    assert!(ids.contains(&s2id) && ids.contains(&c2id));
    let s2 = plan.sources.iter().find(|s| s.id == s2id).unwrap();
    assert_eq!(s2.clip, Some(c2id));
    let _ = tid;
}

#[test]
fn preview_edits_change_displayed_transforms_and_masks() {
    let mut d = Document::new(100, 100);
    let mut a = px("A");
    a.set_mask(Some(mask(4, 4, 255)));
    let mut b = px("B"); b.transform.origin = Point { x: 10.0, y: 0.0 };
    let (aid, bid) = (a.id, b.id);
    d.layers = vec![a, b];
    let draft = LayerTransform::axis_aligned(Point { x: 20.0, y: 30.0 }, Size { width: 8.0, height: 8.0 });
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft, corners: None }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, draft);
    assert_eq!(draw.coverages[0].placement, draft, "a linked covering mask follows");
    // Unlinked: the mask stays where it was.
    d.layer_mut(aid).unwrap().mask_mut().unwrap().linked = Some(false);
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft, corners: None }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.coverages[0].placement, d.layer(aid).unwrap().transform);
    // Mask edit moves only the mask.
    let plan = render_plan(&d, Some(&PreviewEdit::Mask { id: aid, draft }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, d.layer(aid).unwrap().transform);
    assert_eq!(draw.coverages[0].placement, draft);
    // Group edit: both layers follow the box.
    let bounds = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 14.0, height: 4.0 });
    let moved = LayerTransform::axis_aligned(Point { x: 0.0, y: 0.0 }, Size { width: 28.0, height: 8.0 });
    let plan = render_plan(&d, Some(&PreviewEdit::Group { ids: vec![aid, bid], bounds, draft: moved, corners: None }));
    let PlanNode::Layer { draw: db } = &plan.nodes[1] else { panic!() };
    assert_eq!(db.id, bid);
    assert!((db.transform.origin.x - 20.0).abs() < 1e-6 && (db.transform.size.width - 8.0).abs() < 1e-6);
    // Distortion: corners carried; a linked covering mask carries the corners too.
    d.layer_mut(aid).unwrap().mask_mut().unwrap().linked = None;
    let shape = [Point { x: 0.0, y: 0.0 }, Point { x: 8.0, y: 0.0 }, Point { x: 6.0, y: 4.0 }, Point { x: 0.0, y: 4.0 }];
    let plan = render_plan(&d, Some(&PreviewEdit::Layer { id: aid, draft: d.layer(aid).unwrap().transform, corners: Some(shape) }));
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.corners, Some(shape));
    assert_eq!(draw.coverages[0].corners, Some(shape));
}

#[test]
fn plan_serialises_camel_case_with_uppercase_ids() {
    let mut d = Document::new(10, 10);
    let a = px("A");
    let upper = ids::upper_string(&a.id);
    d.layers = vec![a];
    let json = serde_json::to_string(&render_plan(&d, None)).unwrap();
    assert!(json.contains("\"kind\":\"layer\""));
    assert!(json.contains("\"pixelsRevision\""));
    assert!(json.contains(&upper));
    let edit: PreviewEdit = serde_json::from_str(&format!(r#"{{"kind":"group","ids":["{upper}"],"box":{{"origin":[0,0],"size":[4,4]}},"draft":{{"origin":[1,1],"size":[4,4]}}}}"#)).unwrap();
    assert!(matches!(edit, PreviewEdit::Group { .. }));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test plan`
Expected: compile error.

- [ ] **Step 3: Implement the plan**

`engine/src/plan.rs`:
```rust
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    #[serde(with = "ids::upper")] pub layer_id: Uuid,
    pub mask_revision: u64,
    pub placement: LayerTransform,
    pub corners: Option<[Point; 4]>,
    pub width: u32,
    pub height: u32,
    pub background: u8,
    pub nearest: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerDraw {
    #[serde(with = "ids::upper")] pub id: Uuid,
    pub transform: LayerTransform,
    pub corners: Option<[Point; 4]>,
    pub pixels_width: u32,
    pub pixels_height: u32,
    pub pixels_revision: u64,
    pub opacity: f64,
    pub blend: BlendMode,
    pub coverages: Vec<Coverage>,
    #[serde(with = "ids::upper_opt")] pub clip: Option<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PlanNode {
    Layer { draw: LayerDraw },
    Stack { base: LayerDraw, children: Vec<LayerDraw>, folder_coverages: Vec<Coverage> },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPlan { pub nodes: Vec<PlanNode>, pub sources: Vec<LayerDraw> }

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PreviewEdit {
    Layer { #[serde(with = "ids::upper")] id: Uuid, draft: LayerTransform, #[serde(default)] corners: Option<[Point; 4]> },
    Group { #[serde(deserialize_with = "deserialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform, #[serde(default)] corners: Option<[Point; 4]> },
    Mask { #[serde(with = "ids::upper")] id: Uuid, draft: LayerTransform },
}

fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Uuid>, D::Error> {
    let v: Vec<String> = Vec::deserialize(d)?;
    v.iter().map(|t| Uuid::parse_str(t).map_err(serde::de::Error::custom)).collect()
}

/// The layer's transform and distortion corners as the pending edit shows them.
pub fn displayed_transform(layer: &Layer, edit: Option<&PreviewEdit>) -> (LayerTransform, Option<[Point; 4]>) {
    match edit {
        Some(PreviewEdit::Layer { id, draft, corners }) if *id == layer.id => (*draft, *corners),
        Some(PreviewEdit::Group { ids, bounds, draft, corners }) if ids.contains(&layer.id) => {
            let moved = layer.transform.following(bounds, draft);
            let carried = corners.map(|c| Homography::carried(&moved, draft, &c)).filter(Homography::is_usable);
            (moved, carried)
        }
        _ => (layer.transform, None),
    }
}

/// Where the layer's mask shows: nil while it covers the (displayed) layer rectangle.
pub fn displayed_mask_placement(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<LayerTransform> {
    let mask = layer.mask.as_ref()?;
    match edit {
        Some(PreviewEdit::Mask { id, draft }) if *id == layer.id => {
            if draft.same_placement(&layer.transform) { None } else { Some(*draft) }
        }
        Some(PreviewEdit::Layer { id, draft, corners }) if *id == layer.id => {
            if corners.is_some() { if mask.is_linked() && mask.placement.is_none() { None } else { Some(mask.placement.unwrap_or(layer.transform)) } }
            else { mask.follow(&layer.transform, draft) }
        }
        Some(PreviewEdit::Group { ids, bounds, draft, corners }) if ids.contains(&layer.id) => {
            if corners.is_some() { if mask.is_linked() && mask.placement.is_none() { None } else { Some(mask.placement.unwrap_or(layer.transform)) } }
            else { mask.follow(&layer.transform, &layer.transform.following(bounds, draft)) }
        }
        _ => mask.placement,
    }
}

fn own_coverage(layer: &Layer, edit: Option<&PreviewEdit>) -> Option<Coverage> {
    let mask = layer.mask.as_ref()?;
    if !mask.enabled { return None; }
    let (transform, corners) = displayed_transform(layer, edit);
    let placement = displayed_mask_placement(layer, edit);
    // A mask covering its layer takes the layer's distortion; a placed mask keeps its affine placement.
    let corners = if placement.is_none() { corners } else { None };
    Some(Coverage {
        layer_id: layer.id, mask_revision: layer.mask_revision,
        placement: placement.unwrap_or(transform), corners,
        width: mask.pixels.width, height: mask.pixels.height,
        background: mask.background(), nearest: transform.sampling == Sampling::Nearest,
    })
}

fn folder_coverages(doc: &Document, by_id: &HashMap<Uuid, &Layer>, layer: &Layer, edit: Option<&PreviewEdit>) -> Vec<Coverage> {
    let mut result = Vec::new();
    let mut parent = layer.parent_id; let mut depth = 0;
    while let Some(pid) = parent {
        if depth >= MAX_NESTING { break; }
        let Some(folder) = by_id.get(&pid) else { break; };
        if let Some(c) = own_coverage(folder, edit) { result.push(c); }
        parent = folder.parent_id; depth += 1;
    }
    let _ = doc;
    result
}

fn draw_for(doc: &Document, by_id: &HashMap<Uuid, &Layer>, layer: &Layer, edit: Option<&PreviewEdit>, with_folders: bool) -> LayerDraw {
    let (transform, corners) = displayed_transform(layer, edit);
    let mut coverages: Vec<Coverage> = own_coverage(layer, edit).into_iter().collect();
    if with_folders { coverages.extend(folder_coverages(doc, by_id, layer, edit)); }
    let (pw, ph) = layer.pixels.as_ref().map_or((0, 0), |p| (p.width, p.height));
    LayerDraw {
        id: layer.id, transform, corners, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
        opacity: layer.opacity.clamp(0.0, 1.0), blend: layer.blend_mode, coverages, clip: layer.mask_source_id,
    }
}

pub fn render_plan(doc: &Document, edit: Option<&PreviewEdit>) -> RenderPlan {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    let ids = doc.render_ids();
    let source_of = |id: Uuid| by_id.get(&id).and_then(|l| l.mask_source_id);
    let parent_of = |id: Uuid| by_id.get(&id).and_then(|l| l.parent_id);
    // Stacks: a base (not clipped) followed by siblings clipped to it.
    let mut stacked: HashSet<Uuid> = HashSet::new();
    let mut stacks: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (index, base) in ids.iter().enumerate() {
        if source_of(*base).is_some() { continue; }
        let mut children = Vec::new();
        for child in &ids[index + 1..] {
            if source_of(*child) == Some(*base) && parent_of(*child) == parent_of(*base) { children.push(*child); } else { break; }
        }
        if !children.is_empty() { stacked.extend(children.iter().copied()); stacks.insert(*base, children); }
    }
    let mut nodes = Vec::new();
    let mut needed_sources: Vec<Uuid> = Vec::new();
    let mut note_source = |draw: &LayerDraw, needed: &mut Vec<Uuid>| { if let Some(s) = draw.clip { if !needed.contains(&s) { needed.push(s); } } };
    for id in &ids {
        if stacked.contains(id) { continue; }
        let layer = by_id[id];
        if let Some(children) = stacks.get(id) {
            let base = draw_for(doc, &by_id, layer, edit, false);
            let folder = folder_coverages(doc, &by_id, layer, edit);
            let kids: Vec<LayerDraw> = children.iter().map(|c| { let mut d = draw_for(doc, &by_id, by_id[c], edit, false); d.clip = None; d }).collect();
            note_source(&base, &mut needed_sources);
            nodes.push(PlanNode::Stack { base, children: kids, folder_coverages: folder });
        } else {
            let draw = draw_for(doc, &by_id, layer, edit, true);
            note_source(&draw, &mut needed_sources);
            nodes.push(PlanNode::Layer { draw });
        }
    }
    // Sources, including sources of sources, chains capped at 256.
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    let mut i = 0;
    while i < needed_sources.len() && seen.len() < 256 {
        let id = needed_sources[i]; i += 1;
        if !seen.insert(id) { continue; }
        if let Some(layer) = by_id.get(&id) {
            if layer.is_group { continue; }
            let draw = draw_for(doc, &by_id, layer, edit, false);
            if let Some(s) = draw.clip { if !needed_sources.contains(&s) { needed_sources.push(s); } }
            sources.push(draw);
        }
    }
    RenderPlan { nodes, sources }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test plan`
Expected: 6 passed.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): render plan with coverages, clipping stacks and preview edits"
```

---

### Task 4: Compositor executes the plan

**Files:**
- Modify: `engine/src/compositor.rs` (rewrite; keep the public functions Phase 1 callers use)
- Test: `engine/tests/compositor2.rs`

**Interfaces:**
- Keeps: `sample`, `render_layer(target, tw, th, region, &Layer)` (single layer, Normal blend, own opacity, no masks: used by `image_size.rs`), `composite(doc, region, w, h) -> Raster`, `render_full`, `export_png`, `export_jpeg`, `export_jpeg_preview`, `render_layers` (unchanged behaviour).
- Adds: `composite_plan(doc, plan: &RenderPlan, region, w, h) -> Raster`, `composite_edit(doc, edit: Option<&PreviewEdit>, region, w, h) -> Raster`, `source_coverage_at(doc, plan, source: Uuid, p: Point) -> f32` (used by the delete-with-bake op), `coverage_at(doc, cov: &Coverage, p: Point) -> f32`, `sample_draw(doc, draw: &LayerDraw, p: Point) -> [f32;4]` (premultiplied 0..1 at a document point, no opacity or coverage applied), `alpha_bounds(raster) -> Option<(u32,u32,u32,u32)>` (x0, y0, x1, y1 exclusive of the non-transparent pixels), `Raster::cropped(&self, x, y, w, h) -> Raster`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/compositor2.rs`:
```rust
mod fixtures;
use compositor_engine::*;

fn alphas(r: &Raster) -> Vec<u8> { r.bytes().chunks_exact(4).map(|p| p[3]).collect() }
fn reds(r: &Raster) -> Vec<u8> { r.bytes().chunks_exact(4).map(|p| p[0]).collect() }
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height) }
fn gray_mask(w: u32, h: u32, values: Vec<u8>) -> Mask { Mask { pixels: GrayRaster::from_bytes(w, h, values), enabled: true, placement: None, linked: None } }
fn premul_red(alphas: &[u8], w: u32, h: u32) -> Raster {
    let mut data = Vec::new();
    for &a in alphas { data.extend_from_slice(&[a, 0, 0, a]); }
    Raster::from_premultiplied(w, h, data)
}
fn nearest(layer: &mut Layer) { layer.transform.sampling = Sampling::Nearest; }

/// A 2x2 opaque red layer on a 2x2 canvas, nearest sampling.
fn red_doc() -> Document {
    let mut d = Document::new(2, 2);
    let mut l = Layer::with_pixels("Red", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 });
    nearest(&mut l);
    d.active_layer_id = Some(l.id);
    d.layers.push(l);
    d
}

#[test]
fn masks_coverage_opacity_disabled_and_solid() {
    let mut d = red_doc();
    assert_eq!(alphas(&full(&d)), [255; 4]);
    d.layers[0].set_mask(Some(gray_mask(1, 1, vec![0])));
    assert_eq!(alphas(&full(&d)), [0; 4], "hide-all");
    d.layers[0].mask_mut().unwrap().enabled = false;
    assert_eq!(alphas(&full(&d)), [255; 4], "disabled mask is ignored");
    d.layers[0].set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layers[0].opacity = 0.5;
    let a = alphas(&full(&d));
    for (v, e) in a.iter().zip([128u8, 0, 64, 128]) { assert!((*v as i32 - e as i32).abs() <= 1, "{a:?}"); }
}

#[test]
fn masks_follow_flips_and_rotation() {
    let mut d = red_doc();
    d.layers[0].set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    d.layers[0].transform.flip_x = true;
    assert_eq!(alphas(&full(&d)), [0, 255, 255, 128]);
    d.layers[0].transform.flip_x = false;
    d.layers[0].transform.rotation = 90.0;
    let mut a = alphas(&full(&d)); a.sort();
    assert_eq!(a, [0, 128, 255, 255]);
}

#[test]
fn folder_masks_clip_descendants_and_multiply() {
    let mut d = red_doc();
    let red = d.layers[0].id;
    let mut folder = Layer::blank("Folder", d.size()); folder.is_group = true; nearest(&mut folder);
    let fid = folder.id;
    d.layers[0].parent_id = Some(fid);
    d.layers.insert(0, folder);
    d.layer_mut(fid).unwrap().set_mask(Some(gray_mask(1, 1, vec![0])));
    assert_eq!(alphas(&full(&d)), [0; 4]);
    d.layer_mut(fid).unwrap().mask_mut().unwrap().enabled = false;
    assert_eq!(alphas(&full(&d)), [255; 4]);
    d.layer_mut(fid).unwrap().set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layer_mut(red).unwrap().set_mask(Some(gray_mask(2, 2, vec![255, 0, 128, 255])));
    let a = alphas(&full(&d));
    for (v, e) in a.iter().zip([255u8, 0, 64, 255]) { assert!((*v as i32 - e as i32).abs() <= 1, "{a:?}"); }
    // An enclosing folder's mask applies too.
    d.layer_mut(red).unwrap().set_mask(None);
    let mut outer = Layer::blank("Outer", d.size()); outer.is_group = true; outer.set_mask(Some(gray_mask(1, 1, vec![0])));
    let oid = outer.id;
    d.layer_mut(fid).unwrap().parent_id = Some(oid);
    d.layers.insert(0, outer);
    assert_eq!(alphas(&full(&d)), [0; 4]);
    d.layer_mut(oid).unwrap().set_mask(None);
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
}

#[test]
fn clipping_uses_hidden_source_alpha_opacity_masks_movement_and_chains() {
    let mut d = Document::new(2, 2);
    let mut target = Layer::with_pixels("T", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut target);
    let mut source = Layer::with_pixels("S", premul_red(&[255, 0, 128, 255], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut source);
    source.visible = false;
    target.mask_source_id = Some(source.id);
    let sid = source.id;
    d.layers = vec![target, source];
    assert_eq!(alphas(&full(&d)), [255, 0, 128, 255]);
    d.layer_mut(sid).unwrap().opacity = 0.5;
    let a = alphas(&full(&d));
    assert!((a[0] as i32 - 128).abs() <= 1 && a[1] == 0 && (a[2] as i32 - 64).abs() <= 1, "{a:?}");
    d.layers[0].set_mask(Some(gray_mask(1, 1, vec![128])));
    assert!(alphas(&full(&d))[0] < a[0]);
    d.layers[0].set_mask(None);
    d.layer_mut(sid).unwrap().opacity = 1.0;
    d.layer_mut(sid).unwrap().transform.origin.x += 1.0;
    assert_eq!(alphas(&full(&d)), [0, 255, 0, 128], "a moved source changes coverage");
    d.layer_mut(sid).unwrap().transform.origin.x -= 1.0;
    let mut s2 = Layer::with_pixels("S2", premul_red(&[0, 255, 255, 255], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut s2);
    s2.visible = false;
    d.layer_mut(sid).unwrap().mask_source_id = Some(s2.id);
    d.layers.push(s2);
    assert_eq!(alphas(&full(&d)), [0, 0, 128, 255], "chains multiply");
}

#[test]
fn clipping_stack_keeps_the_base_alpha_and_colours_without_fringe() {
    let mut d = Document::new(2, 2);
    let mut base = Layer::with_pixels("Base", premul_red(&[255, 128, 32, 0], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut base);
    // The child is opaque red everywhere (its own alpha 255).
    let mut child = Layer::with_pixels("Child", premul_red(&[255; 4], 2, 2), Point { x: 0.0, y: 0.0 }); nearest(&mut child);
    child.mask_source_id = Some(base.id);
    d.layers = vec![base, child];
    let out = full(&d);
    assert_eq!(alphas(&out), [255, 128, 32, 0]);
    for px in out.bytes().chunks_exact(4) { assert_eq!(px[0], px[3]); assert_eq!((px[1], px[2]), (0, 0)); }
    d.layers[1].opacity = 0.5;
    assert_eq!(alphas(&full(&d)), [255, 128, 32, 0]);
    d.layers[1].opacity = 1.0;
    let mut white = Layer::with_pixels("White", Raster::from_premultiplied(2, 2, vec![255; 16]), Point { x: 0.0, y: 0.0 }); nearest(&mut white);
    d.layers.insert(0, white);
    let out = full(&d);
    assert_eq!(alphas(&out), [255; 4]);
    assert_eq!(reds(&out), [255; 4]);
}

#[test]
fn blend_modes_match_known_values_through_the_document() {
    let grey = |v: u8| Raster::from_premultiplied(4, 4, [v, v, v, 255].repeat(16));
    let mut d = Document::new(4, 4);
    d.layers.push(Layer::with_pixels("Back", grey(102), Point { x: 0.0, y: 0.0 }));
    d.layers.push(Layer::with_pixels("Front", grey(204), Point { x: 0.0, y: 0.0 }));
    for (mode, expected) in [(BlendMode::Normal, 0.8), (BlendMode::Multiply, 0.32), (BlendMode::Screen, 0.88), (BlendMode::Overlay, 0.64),
        (BlendMode::Darken, 0.4), (BlendMode::Lighten, 0.8), (BlendMode::Difference, 0.4), (BlendMode::ColorDodge, 1.0), (BlendMode::ColorBurn, 0.25)] {
        d.layers[1].blend_mode = mode;
        let px = full(&d).pixel(1, 1);
        assert!(((px[0] as f64 / 255.0) - expected).abs() < 0.02, "{mode:?}: {}", px[0]);
        assert_eq!(px[3], 255);
    }
    d.layers[1].blend_mode = BlendMode::Normal;
    d.layers[1].opacity = 0.5;
    assert!(((full(&d).pixel(1, 1)[0] as f64 / 255.0) - 0.6).abs() < 0.02);
}

#[test]
fn distortion_preview_warps_into_the_shape() {
    let mut d = Document::new(100, 60);
    let mut l = Layer::with_pixels("Red", premul_red(&[255; 400], 20, 20), Point { x: 10.0, y: 10.0 }); nearest(&mut l);
    let id = l.id;
    d.layers.push(l);
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let edit = PreviewEdit::Layer { id, draft: d.layers[0].transform, corners: Some(shape) };
    let out = composite_edit(&d, Some(&edit), Rect { x: 0.0, y: 0.0, width: 100.0, height: 60.0 }, 100, 60);
    assert_eq!(out.pixel(50, 12)[3], 255);
    assert_eq!(out.pixel(15, 25)[3], 255);
    assert_eq!(out.pixel(50, 28)[3], 0);
    assert_eq!(out.pixel(80, 12)[3], 0);
}

#[test]
fn alpha_bounds_and_crop() {
    let mut data = vec![0u8; 40 * 20 * 4];
    for y in 5..15 { for x in 15..25 { let i = (y * 40 + x) * 4; data[i] = 255; data[i + 3] = 255; } }
    let r = Raster::from_premultiplied(40, 20, data);
    assert_eq!(alpha_bounds(&r), Some((15, 5, 25, 15)));
    let c = r.cropped(15, 5, 10, 10);
    assert_eq!((c.width, c.height), (10, 10));
    assert_eq!(c.pixel(0, 0)[3], 255);
    assert_eq!(alpha_bounds(&Raster::new_transparent(3, 3)), None);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test compositor2`
Expected: compile errors (`composite_edit`, `alpha_bounds`, `cropped` missing); once those exist, the mask and clipping tests fail because the Phase 1 compositor ignores them.

- [ ] **Step 3: Add raster helpers**

Append to `engine/src/raster.rs` (inside `impl Raster`):
```rust
    /// A copy of the `w` x `h` rectangle at (x, y); clamped to the raster.
    pub fn cropped(&self, x: u32, y: u32, w: u32, h: u32) -> Raster {
        let x1 = (x + w).min(self.width); let y1 = (y + h).min(self.height);
        let (w, h) = (x1.saturating_sub(x).max(1), y1.saturating_sub(y).max(1));
        let mut out = vec![0u8; (w * h * 4) as usize];
        for row in 0..h {
            let sy = y + row; if sy >= self.height { break; }
            let src = ((sy * self.width + x) * 4) as usize;
            let n = (w.min(self.width - x) * 4) as usize;
            out[(row * w * 4) as usize..(row * w * 4) as usize + n].copy_from_slice(&self.inner.data[src..src + n]);
        }
        Raster::from_premultiplied(w, h, out)
    }
```
And a free function in `engine/src/compositor.rs`:
```rust
/// (x0, y0, x1, y1) of the pixels with alpha > 0, x1/y1 exclusive; None when fully transparent.
pub fn alpha_bounds(r: &Raster) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for y in 0..r.height { for x in 0..r.width {
        if r.pixel(x, y)[3] > 0 { x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x + 1); y1 = y1.max(y + 1); }
    }}
    if x1 == 0 { None } else { Some((x0, y0, x1, y1)) }
}
```

- [ ] **Step 4: Rewrite the compositor**

Replace `engine/src/compositor.rs` (keep `sample`, `prefiltered`, `check_export_size`, `render_full`, `export_*` bodies; `render_layers` stays for callers). New core:
```rust
use crate::*;
use std::collections::HashMap;
use uuid::Uuid;

// ... `sample` and `prefiltered` as in Phase 1 ...

/// Maps a document point into a layer's pixel grid: through the distortion when there is one, else the affine inverse.
fn to_pixels(transform: &LayerTransform, corners: Option<&[Point; 4]>, w: u32, h: u32, p: Point) -> Option<Point> {
    if let Some(c) = corners {
        let inv = Homography::unit_to(c).invert()?;
        let u = inv.apply(p);
        let ux = if transform.flip_x { 1.0 - u.x } else { u.x };
        let uy = if transform.flip_y { 1.0 - u.y } else { u.y };
        Some(Point { x: ux * w as f64, y: uy * h as f64 })
    } else {
        transform.pixel_to_document(w, h).invert().map(|inv| inv.apply(p))
    }
}

/// The layer's premultiplied colour at a document point (no opacity, no coverage); transparent outside.
pub fn sample_draw(doc: &Document, draw: &LayerDraw, p: Point) -> [f32; 4] {
    let Some(raster) = doc.layer(draw.id).and_then(|l| l.pixels.as_ref()) else { return [0.0; 4]; };
    let Some(px) = to_pixels(&draw.transform, draw.corners.as_ref(), raster.width, raster.height, p) else { return [0.0; 4]; };
    if px.x < 0.0 || px.y < 0.0 || px.x >= raster.width as f64 || px.y >= raster.height as f64 { return [0.0; 4]; }
    sample(raster, px.x, px.y, draw.transform.sampling == Sampling::Nearest)
}

fn gray_sample(mask: &GrayRaster, x: f64, y: f64, nearest: bool) -> f32 {
    let w = mask.width as i64; let h = mask.height as i64;
    let fetch = |px: i64, py: i64| mask.bytes()[(py.clamp(0, h - 1) * w + px.clamp(0, w - 1)) as usize] as f32 / 255.0;
    if nearest { return fetch(x.floor() as i64, y.floor() as i64); }
    let fx = x - 0.5; let fy = y - 0.5;
    let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    let a = fetch(xu, yu); let b = fetch(xu + 1, yu); let c = fetch(xu, yu + 1); let d = fetch(xu + 1, yu + 1);
    (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty
}

/// One coverage mask's value at a document point: its pixels inside, its background outside.
pub fn coverage_at(doc: &Document, cov: &Coverage, p: Point) -> f32 {
    let Some(mask) = doc.layer(cov.layer_id).and_then(|l| l.mask.as_ref()) else { return 1.0; };
    let Some(px) = to_pixels(&cov.placement, cov.corners.as_ref(), cov.width, cov.height, p) else { return 1.0; };
    if px.x < 0.0 || px.y < 0.0 || px.x >= cov.width as f64 || px.y >= cov.height as f64 { return cov.background as f32 / 255.0; }
    gray_sample(&mask.pixels, px.x, px.y, cov.nearest)
}

fn coverages_at(doc: &Document, covs: &[Coverage], p: Point) -> f32 {
    covs.iter().fold(1.0, |k, c| k * coverage_at(doc, c, p))
}

/// A clipping source's coverage at a document point: its alpha times opacity, own mask and its own clipping chain.
pub fn source_coverage_at(doc: &Document, plan: &RenderPlan, source: Uuid, p: Point) -> f32 {
    fn inner(doc: &Document, plan: &RenderPlan, source: Uuid, p: Point, depth: u32) -> f32 {
        if depth > 256 { return 1.0; }
        let Some(draw) = plan.sources.iter().find(|s| s.id == source) else { return 1.0; };
        let a = sample_draw(doc, draw, p)[3] * draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        match draw.clip { Some(c) => a * inner(doc, plan, c, p, depth + 1), None => a }
    }
    inner(doc, plan, source, p, 0)
}

struct Target<'a> { data: &'a mut [u8], w: u32, h: u32, region: Rect }

impl<'a> Target<'a> {
    fn doc_point(&self, ox: u32, oy: u32) -> Point {
        Point { x: self.region.x + (ox as f64 + 0.5) * self.region.width / self.w as f64, y: self.region.y + (oy as f64 + 0.5) * self.region.height / self.h as f64 }
    }
    /// Output-pixel bounding box of a draw, padded by one.
    fn bbox(&self, draw: &LayerDraw) -> (u32, u32, u32, u32) {
        let b = match draw.corners { Some(c) => { let xs = c.iter().map(|p| p.x); let ys = c.iter().map(|p| p.y);
            let (x0, x1) = (xs.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max));
            let (y0, y1) = (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max));
            Rect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 } } None => draw.transform.bounds() };
        let sx = self.w as f64 / self.region.width; let sy = self.h as f64 / self.region.height;
        let x0 = (((b.x - self.region.x) * sx).floor() as i64 - 1).clamp(0, self.w as i64) as u32;
        let y0 = (((b.y - self.region.y) * sy).floor() as i64 - 1).clamp(0, self.h as i64) as u32;
        let x1 = (((b.max_x() - self.region.x) * sx).ceil() as i64 + 1).clamp(0, self.w as i64) as u32;
        let y1 = (((b.max_y() - self.region.y) * sy).ceil() as i64 + 1).clamp(0, self.h as i64) as u32;
        (x0, y0, x1, y1)
    }
}

/// Draws one layer with its opacity, coverages and clip, blended with its mode.
fn draw_layer(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &LayerDraw, blend: BlendMode, use_clip: bool) {
    let Some(raster) = doc.layer(draw.id).and_then(|l| l.pixels.as_ref()) else { return; };
    // Prefilter large affine reductions with High sampling (never for distortions).
    let out_per_doc = target.w as f64 / target.region.width;
    let source_per_output = raster.width as f64 / (draw.transform.size.width * out_per_doc);
    let (source, scale) = if draw.corners.is_none() && draw.transform.sampling == Sampling::High && source_per_output > 2.0 { prefiltered(raster, source_per_output) } else { (raster.clone(), 1.0) };
    let nearest = draw.transform.sampling == Sampling::Nearest;
    let (x0, y0, x1, y1) = target.bbox(draw);
    for oy in y0..y1 { for ox in x0..x1 {
        let p = target.doc_point(ox, oy);
        let Some(px) = to_pixels(&draw.transform, draw.corners.as_ref(), raster.width, raster.height, p) else { continue; };
        if px.x < 0.0 || px.y < 0.0 || px.x >= raster.width as f64 || px.y >= raster.height as f64 { continue; }
        let mut s = sample(&source, px.x * scale, px.y * scale, nearest);
        if s[3] <= 0.0 { continue; }
        let mut k = draw.opacity as f32 * coverages_at(doc, &draw.coverages, p);
        if use_clip { if let Some(c) = draw.clip { k *= source_coverage_at(doc, plan, c, p); } }
        if k <= 0.0 { continue; }
        for v in &mut s { *v *= k; }
        let i = ((oy * target.w + ox) * 4) as usize;
        compose_u8(&mut target.data[i..i + 4], s, blend);
    }}
}

fn draw_stack(doc: &Document, plan: &RenderPlan, target: &mut Target, base: &LayerDraw, children: &[LayerDraw], folder: &[Coverage]) {
    let (w, h) = (target.w, target.h);
    let mut temp = vec![0u8; (w * h * 4) as usize];
    {
        let mut t = Target { data: &mut temp, w, h, region: target.region };
        draw_layer(doc, plan, &mut t, base, BlendMode::Normal, true);
    }
    let base_alpha: Vec<u8> = temp.chunks_exact(4).map(|p| p[3]).collect();
    // Opaque where the base has any coverage, so children blend against its colours.
    for px in temp.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 { for c in 0..3 { px[c] = ((px[c] as u32 * 255 + a / 2) / a).min(255) as u8; } px[3] = 255; }
    }
    {
        let mut t = Target { data: &mut temp, w, h, region: target.region };
        for child in children { draw_layer(doc, plan, &mut t, child, child.blend, false); }
    }
    // Restore the base alpha, then composite with the base's blend mode under its folder masks.
    for (i, px) in temp.chunks_exact_mut(4).enumerate() {
        let a = base_alpha[i] as u32;
        for c in 0..3 { px[c] = ((px[c] as u32 * a + 127) / 255) as u8; }
        px[3] = a as u8;
    }
    for oy in 0..h { for ox in 0..w {
        let i = ((oy * w + ox) * 4) as usize;
        if temp[i + 3] == 0 { continue; }
        let p = target.doc_point(ox, oy);
        let k = coverages_at(doc, folder, p);
        if k <= 0.0 { continue; }
        let s = [temp[i] as f32 / 255.0 * k, temp[i + 1] as f32 / 255.0 * k, temp[i + 2] as f32 / 255.0 * k, temp[i + 3] as f32 / 255.0 * k];
        compose_u8(&mut target.data[i..i + 4], s, base.blend);
    }}
}

pub fn composite_plan(doc: &Document, plan: &RenderPlan, region: Rect, out_width: u32, out_height: u32) -> Raster {
    let mut data = vec![0u8; (out_width as usize) * (out_height as usize) * 4];
    {
        let mut target = Target { data: &mut data, w: out_width, h: out_height, region };
        for node in &plan.nodes {
            match node {
                PlanNode::Layer { draw } => draw_layer(doc, plan, &mut target, draw, draw.blend, true),
                PlanNode::Stack { base, children, folder_coverages } => draw_stack(doc, plan, &mut target, base, children, folder_coverages),
            }
        }
    }
    Raster::from_premultiplied(out_width, out_height, data)
}

pub fn composite_edit(doc: &Document, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Raster {
    composite_plan(doc, &render_plan(doc, edit), region, w, h)
}

pub fn composite(doc: &Document, region: Rect, w: u32, h: u32) -> Raster { composite_edit(doc, None, region, w, h) }

/// Draws a single layer with Normal blend and its opacity, ignoring masks and clipping (Image Size resampling).
pub fn render_layer(target: &mut [u8], tw: u32, th: u32, region: Rect, layer: &Layer) {
    let doc = Document { id: Uuid::nil(), width: 1, height: 1, resolution: 72.0, layers: vec![layer.clone()], active_layer_id: None };
    let plan = RenderPlan { nodes: vec![], sources: vec![] };
    let (pw, ph) = layer.pixels.as_ref().map_or((0, 0), |p| (p.width, p.height));
    let draw = LayerDraw { id: layer.id, transform: layer.transform, corners: None, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
        opacity: layer.opacity.clamp(0.0, 1.0), blend: BlendMode::Normal, coverages: vec![], clip: None };
    let mut t = Target { data: target, w: tw, h: th, region };
    draw_layer(&doc, &plan, &mut t, &draw, BlendMode::Normal, false);
}
```
Keep `render_layers(doc) -> Vec<&Layer>` (Phase 1) as is; export helpers call `composite`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including the 8 in `compositor2.rs` and every Phase 1 test (export, image_size, canvas_size, interop).

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): compositor executes the render plan (blend modes, masks, clipping, stacks, distortion)"
```

---

### Task 5: Appearance and hierarchy operations

**Files:**
- Create: `engine/src/ops/appearance.rs`, `engine/src/ops/hierarchy.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod appearance; pub mod hierarchy;`), `engine/src/plan.rs` (add `ensure_source`)
- Test: `engine/tests/hierarchy.rs`

**Interfaces:**
- `plan::ensure_source(doc: &Document, plan: &mut RenderPlan, id: Uuid)`: appends draws for `id` and its clipping chain to `plan.sources` when absent (own mask only, visibility ignored).
- `ops::appearance::{set_opacity(doc, id, opacity) -> Result<(), CommandError>, set_opacity_many(doc, ids, opacity), set_blend_mode(doc, id, mode)}`: opacity must be finite, clamped to 0..1; groups are rejected with `CommandError::Argument("folders have no opacity or blend mode")` (skipped silently in `set_opacity_many`).
- `ops::hierarchy::{next_folder_name(doc) -> String, add_group(doc) -> Result<Uuid>, group_layers(doc, ids: &[Uuid]) -> Result<Uuid>, can_place(doc, id, parent: Option<Uuid>) -> bool, place_layer(doc, id, parent, above: Option<Uuid>, at_bottom: bool) -> Result<()>, move_layer_by(doc, id, offset: i32) -> Result<()>, duplicate_layer(doc, id) -> Result<Uuid>, duplicate_layer_to(doc, id, parent, above, at_bottom) -> Result<Uuid>, clip_dependents(doc, ids: &[Uuid]) -> Vec<Uuid>, bake_clip(doc, target: Uuid) -> Option<Raster>, delete_layers(doc, ids: &[Uuid], bake: bool) -> Result<()>, can_link_mask(doc, source, target) -> bool, link_mask(doc, source, target) -> Result<()>, release_clipping(doc, target) -> Result<()>, can_toggle_clipping(doc, id) -> bool, toggle_clipping(doc, id) -> Result<()>, hierarchy_order(doc) -> Vec<Uuid>, validate(doc) -> Result<(), CommandError>}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/hierarchy.rs`:
```rust
use compositor_engine::ops::{appearance, hierarchy, layers};
use compositor_engine::*;

fn doc() -> Document { let mut d = Document::new(100, 100); d.active_layer_id = None; d }
fn names(d: &Document) -> Vec<String> { d.layers.iter().map(|l| l.name.clone()).collect() }
fn parent(d: &Document, id: uuid::Uuid) -> Option<uuid::Uuid> { d.layer(id).unwrap().parent_id }
fn red(w: u32, h: u32, a: &[u8]) -> Raster { let mut v = Vec::new(); for &x in a { v.extend_from_slice(&[x, 0, 0, x]); } Raster::from_premultiplied(w, h, v) }

#[test]
fn opacity_and_blend_mode() {
    let mut d = doc();
    let id = layers::add_blank_layer(&mut d).unwrap();
    appearance::set_opacity(&mut d, id, 0.25).unwrap();
    appearance::set_blend_mode(&mut d, id, BlendMode::Multiply).unwrap();
    assert_eq!((d.layer(id).unwrap().opacity, d.layer(id).unwrap().blend_mode), (0.25, BlendMode::Multiply));
    assert!(appearance::set_opacity(&mut d, id, f64::NAN).is_err());
    appearance::set_opacity(&mut d, id, 7.0).unwrap();
    assert_eq!(d.layer(id).unwrap().opacity, 1.0);
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(appearance::set_opacity(&mut d, g, 0.5).is_err());
    appearance::set_opacity_many(&mut d, &[id, g], 0.5).unwrap();
    assert_eq!((d.layer(id).unwrap().opacity, d.layer(g).unwrap().opacity), (0.5, 1.0));
}

#[test]
fn nested_groups_place_and_move_out() {
    let mut d = doc();
    let outer = hierarchy::add_group(&mut d).unwrap();
    let inner = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    assert_eq!(parent(&d, inner), Some(outer));
    assert_eq!(parent(&d, child), Some(inner));
    assert!(!hierarchy::can_place(&d, outer, Some(inner)));
    assert!(!hierarchy::can_place(&d, inner, Some(inner)));
    assert!(hierarchy::place_layer(&mut d, outer, Some(inner), None, false).is_err());
    // Move the child out of the inner folder, above it in the outer folder.
    hierarchy::place_layer(&mut d, child, Some(outer), Some(inner), false).unwrap();
    assert_eq!(parent(&d, child), Some(outer));
    assert_eq!(d.active_layer_id, Some(child));
    assert_eq!(names(&d), ["Folder 1", "Folder 2", "Layer 1"]);
    hierarchy::place_layer(&mut d, child, None, None, true).unwrap();
    assert_eq!((parent(&d, child), d.layers[0].id), (None, child));
}

#[test]
fn group_layers_wraps_selection_at_the_common_parent_in_order() {
    let mut d = doc();
    let folder = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let sibling = layers::add_blank_layer(&mut d).unwrap();
    let wrapper = hierarchy::group_layers(&mut d, &[folder, child, sibling]).unwrap();
    assert_eq!(parent(&d, folder), Some(wrapper));
    assert_eq!(parent(&d, sibling), Some(wrapper));
    assert_eq!(parent(&d, child), Some(folder), "a selected folder keeps its contents");
    assert_eq!(parent(&d, wrapper), None);
    assert_eq!(d.render_ids(), vec![child, sibling]);
    assert_eq!(d.active_layer_id, Some(wrapper));
    // Single layer then single folder: wrapped, not a child folder.
    let mut d = doc();
    let layer = layers::add_blank_layer(&mut d).unwrap();
    let inner = hierarchy::group_layers(&mut d, &[layer]).unwrap();
    let outer = hierarchy::group_layers(&mut d, &[inner]).unwrap();
    assert_eq!((parent(&d, layer), parent(&d, inner), parent(&d, outer)), (Some(inner), Some(outer), None));
    // Different folders: the common parent is the root; empty selection makes an empty folder.
    let mut d = doc();
    let first = hierarchy::group_layers(&mut d, &[]).unwrap();
    let a = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let second = hierarchy::group_layers(&mut d, &[]).unwrap();
    let b = layers::add_blank_layer(&mut d).unwrap();
    let g = hierarchy::group_layers(&mut d, &[a, b]).unwrap();
    assert_eq!((parent(&d, g), parent(&d, a), parent(&d, b)), (None, Some(g), Some(g)));
    assert_eq!((parent(&d, first), parent(&d, second)), (None, None));
    assert_eq!(names(&d)[d.index_of(g).unwrap()], "Folder 3");
}

#[test]
fn move_by_swaps_siblings_and_duplicate_copies_above() {
    let mut d = doc();
    let ids: Vec<_> = (0..3).map(|_| layers::add_blank_layer(&mut d).unwrap()).collect();
    hierarchy::move_layer_by(&mut d, ids[0], 1).unwrap();
    assert_eq!(names(&d), ["Layer 2", "Layer 1", "Layer 3"]);
    assert!(hierarchy::move_layer_by(&mut d, ids[2], 1).is_err(), "top layer cannot move up");
    let copy = hierarchy::duplicate_layer(&mut d, ids[0]).unwrap();
    assert_eq!(names(&d), ["Layer 2", "Layer 1", "Layer 1 copy", "Layer 3"]);
    assert_eq!(d.active_layer_id, Some(copy));
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(hierarchy::duplicate_layer(&mut d, g).is_err(), "folders are not duplicated");
    let top = ids[2];
    let dropped = hierarchy::duplicate_layer_to(&mut d, ids[1], None, Some(top), false).unwrap();
    assert_eq!(d.layers.last().map(|l| l.name.clone()).unwrap_or_default(), "Folder 1");
    assert_eq!(d.index_of(dropped).unwrap(), d.index_of(top).unwrap() + 1);
}

#[test]
fn clipping_links_stacks_and_detachment() {
    let mut d = doc();
    let ids: Vec<_> = (0..3).map(|i| { let l = Layer::with_pixels(&format!("L{i}"), red(2, 2, &[255; 4]), Point { x: 0.0, y: 0.0 }); let id = l.id; d.layers.push(l); id }).collect();
    assert!(!hierarchy::can_toggle_clipping(&d, ids[0]) && hierarchy::can_toggle_clipping(&d, ids[1]));
    hierarchy::toggle_clipping(&mut d, ids[0]).unwrap_err();
    hierarchy::toggle_clipping(&mut d, ids[1]).unwrap();
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    assert_eq!(d.layer(ids[1]).unwrap().mask_source_id, Some(ids[0]));
    assert_eq!(d.layer(ids[2]).unwrap().mask_source_id, Some(ids[0]), "shares the base");
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    assert_eq!(d.layer(ids[2]).unwrap().mask_source_id, None);
    assert_eq!(d.layer(ids[1]).unwrap().mask_source_id, Some(ids[0]));
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    hierarchy::release_clipping(&mut d, ids[1]).unwrap();
    assert!(d.layer(ids[1]).unwrap().mask_source_id.is_none() && d.layer(ids[2]).unwrap().mask_source_id.is_none(), "releasing the lowest child releases the ones above");
    hierarchy::toggle_clipping(&mut d, ids[1]).unwrap();
    hierarchy::toggle_clipping(&mut d, ids[2]).unwrap();
    hierarchy::place_layer(&mut d, ids[2], None, None, true).unwrap();
    assert_eq!(d.layers[0].id, ids[2]);
    assert_eq!(d.layers[0].mask_source_id, None, "dragged out of the stack, it stops clipping");
    assert_eq!(d.layers.last().unwrap().mask_source_id, Some(ids[0]));
    assert!(!hierarchy::can_link_mask(&d, ids[1], ids[0]), "cycle");
    assert!(!hierarchy::can_link_mask(&d, ids[0], ids[0]));
}

#[test]
fn deleting_sources_bakes_or_unlinks() {
    let mut d = Document::new(2, 2);
    let mut target = Layer::with_pixels("T", red(2, 2, &[255; 4]), Point { x: 0.0, y: 0.0 }); target.transform.sampling = Sampling::Nearest;
    let mut source = Layer::with_pixels("S", red(2, 2, &[255, 0, 128, 255]), Point { x: 0.0, y: 0.0 }); source.transform.sampling = Sampling::Nearest;
    source.visible = false;
    target.mask_source_id = Some(source.id);
    let (tid, sid) = (target.id, source.id);
    d.layers = vec![target, source];
    assert_eq!(hierarchy::clip_dependents(&d, &[sid]), vec![tid]);
    assert!(hierarchy::clip_dependents(&d, &[tid]).is_empty());
    let before: Vec<u8> = composite(&d, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).bytes().chunks_exact(4).map(|p| p[3]).collect();
    let mut baked = d.clone();
    hierarchy::delete_layers(&mut baked, &[sid], true).unwrap();
    let after: Vec<u8> = composite(&baked, Rect { x: 0.0, y: 0.0, width: 2.0, height: 2.0 }, 2, 2).bytes().chunks_exact(4).map(|p| p[3]).collect();
    assert_eq!(before, after);
    assert_eq!(baked.layers.len(), 1);
    assert_eq!(baked.layers[0].mask_source_id, None);
    assert_eq!(baked.layers[0].pixels_revision, 2);
    let mut unlinked = d.clone();
    hierarchy::delete_layers(&mut unlinked, &[sid], false).unwrap();
    assert_eq!(unlinked.layers[0].pixels_revision, 1);
    assert_eq!(unlinked.layers[0].mask_source_id, None);
}

#[test]
fn deleting_a_folder_removes_its_contents_and_picks_a_neighbour() {
    let mut d = doc();
    let keep = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let folder = hierarchy::add_group(&mut d).unwrap();
    let child = layers::add_blank_layer(&mut d).unwrap();
    d.active_layer_id = None;
    let top = layers::add_blank_layer(&mut d).unwrap();
    hierarchy::delete_layers(&mut d, &[folder, top], false).unwrap();
    assert_eq!(d.layers.iter().map(|l| l.id).collect::<Vec<_>>(), vec![keep]);
    assert_eq!(d.active_layer_id, Some(keep));
    let _ = child;
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test hierarchy`
Expected: compile errors.

- [ ] **Step 3: Implement appearance ops**

`engine/src/ops/appearance.rs`:
```rust
use crate::{BlendMode, CommandError, Document};
use uuid::Uuid;

fn pixel_layer<'a>(doc: &'a mut Document, id: Uuid) -> Result<&'a mut crate::Layer, CommandError> {
    let layer = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if layer.is_group { return Err(CommandError::Argument("folders have no opacity or blend mode".into())); }
    Ok(layer)
}

pub fn set_opacity(doc: &mut Document, id: Uuid, opacity: f64) -> Result<(), CommandError> {
    if !opacity.is_finite() { return Err(CommandError::Argument("opacity must be a number".into())); }
    pixel_layer(doc, id)?.opacity = opacity.clamp(0.0, 1.0);
    Ok(())
}

/// Every pixel layer among `ids`; folders are skipped.
pub fn set_opacity_many(doc: &mut Document, ids: &[Uuid], opacity: f64) -> Result<(), CommandError> {
    if !opacity.is_finite() { return Err(CommandError::Argument("opacity must be a number".into())); }
    for id in ids {
        if let Some(layer) = doc.layer_mut(*id) { if !layer.is_group { layer.opacity = opacity.clamp(0.0, 1.0); } }
    }
    Ok(())
}

pub fn set_blend_mode(doc: &mut Document, id: Uuid, mode: BlendMode) -> Result<(), CommandError> {
    pixel_layer(doc, id)?.blend_mode = mode;
    Ok(())
}
```

- [ ] **Step 4: Implement hierarchy ops**

Add to `engine/src/plan.rs`:
```rust
/// Appends draws for `id` and its clipping chain to `plan.sources` when they are missing.
pub fn ensure_source(doc: &Document, plan: &mut RenderPlan, id: Uuid) {
    let by_id: HashMap<Uuid, &Layer> = doc.layers.iter().map(|l| (l.id, l)).collect();
    let mut next = Some(id); let mut depth = 0;
    while let (Some(sid), true) = (next, depth < 256) {
        depth += 1;
        if plan.sources.iter().any(|s| s.id == sid) { break; }
        let Some(layer) = by_id.get(&sid) else { break; };
        if layer.is_group { break; }
        let draw = draw_for(doc, &by_id, layer, None, false);
        next = draw.clip;
        plan.sources.push(draw);
    }
}
```

`engine/src/ops/hierarchy.rs`:
```rust
use crate::*;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn validate(doc: &Document) -> Result<(), CommandError> { doc.manifest().validate().map_err(CommandError::Project) }

pub fn next_folder_name(doc: &Document) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("Folder {n}")) { n += 1; }
    format!("Folder {n}")
}

fn ancestors(doc: &Document, id: Uuid) -> Vec<Option<Uuid>> {
    let mut result = Vec::new();
    let mut parent = doc.layer(id).and_then(|l| l.parent_id);
    let mut steps = 0;
    while let Some(p) = parent { if steps > MAX_NESTING { break; } result.push(Some(p)); parent = doc.layer(p).and_then(|l| l.parent_id); steps += 1; }
    result.push(None);
    result
}

/// Depth-first, bottom-first: root siblings in array order, each folder's contents right after it.
pub fn hierarchy_order(doc: &Document) -> Vec<Uuid> {
    fn visit(doc: &Document, parent: Option<Uuid>, depth: usize, out: &mut Vec<Uuid>) {
        if depth > MAX_NESTING { return; }
        for id in doc.siblings(parent) {
            out.push(id);
            if doc.layer(id).map_or(false, |l| l.is_group) { visit(doc, Some(id), depth + 1, out); }
        }
    }
    let mut out = Vec::new();
    visit(doc, None, 0, &mut out);
    out
}

pub fn add_group(doc: &mut Document) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut group = Layer::blank(&next_folder_name(doc), doc.size());
    group.is_group = true;
    let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
    group.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
    let insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
    let id = group.id;
    doc.layers.insert(insertion, group);
    validate(doc)?;
    doc.active_layer_id = Some(id);
    Ok(id)
}

/// Wraps `ids` (a selected folder carries its subtree) in a new folder at their common parent.
pub fn group_layers(doc: &mut Document, ids: &[Uuid]) -> Result<Uuid, CommandError> {
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let selected: HashSet<Uuid> = ids.iter().copied().filter(|id| doc.layer(*id).is_some()).collect();
    let root_ids: HashSet<Uuid> = selected.iter().copied().filter(|id| !ancestors(doc, *id).iter().any(|a| a.map_or(false, |a| selected.contains(&a)))).collect();
    let ordered: Vec<Uuid> = hierarchy_order(doc).into_iter().filter(|id| root_ids.contains(id)).collect();
    let parent: Option<Uuid> = ordered.first().and_then(|first| {
        ancestors(doc, *first).into_iter().find(|candidate| ordered.iter().all(|id| ancestors(doc, *id).contains(candidate)))
    }).flatten();
    let mut group = Layer::blank(&next_folder_name(doc), doc.size());
    group.is_group = true;
    group.parent_id = parent;
    let branches: Vec<Uuid> = ordered.iter().map(|id| {
        let mut branch = *id;
        let mut steps = 0;
        while let Some(next) = doc.layer(branch).and_then(|l| l.parent_id) { if next == parent.unwrap_or(Uuid::nil()) && parent.is_some() { break; } if parent.is_none() && false { break; } if Some(next) == parent { break; } branch = next; steps += 1; if steps > MAX_NESTING { break; } }
        branch
    }).collect();
    let highest = doc.layers.iter().rposition(|l| branches.contains(&l.id));
    let insertion = highest.map(|h| doc.layers[..=h].iter().filter(|l| !root_ids.contains(&l.id)).count()).unwrap_or(doc.layers.len());
    let mut layers: Vec<Layer> = doc.layers.iter().filter(|l| !root_ids.contains(&l.id)).cloned().collect();
    let gid = group.id;
    layers.insert(insertion.min(layers.len()), group);
    for id in &ordered {
        if let Some(mut child) = doc.layer(*id).cloned() { child.parent_id = Some(gid); layers.push(child); }
    }
    let previous = std::mem::replace(&mut doc.layers, layers);
    if let Err(e) = validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(gid);
    Ok(gid)
}

pub fn can_place(doc: &Document, id: Uuid, parent: Option<Uuid>) -> bool {
    if doc.layer(id).is_none() { return false; }
    let Some(parent) = parent else { return true; };
    parent != id && !doc.descendants(id).contains(&parent) && doc.layer(parent).map_or(false, |l| l.is_group)
}

/// A layer dropped between a base and a layer clipped to it joins the clipping group.
fn adopt_clipping(layers: &mut [Layer], id: Uuid) {
    let Some(layer) = layers.iter().find(|l| l.id == id).cloned() else { return; };
    if layer.is_group { return; }
    let siblings: Vec<Layer> = layers.iter().filter(|l| l.parent_id == layer.parent_id).cloned().collect();
    let Some(index) = siblings.iter().position(|l| l.id == id) else { return; };
    if index == 0 || index + 1 >= siblings.len() { return; }
    let Some(source) = siblings[index + 1].mask_source_id else { return; };
    if source == id { return; }
    let below = &siblings[index - 1];
    if below.id == source || below.mask_source_id == Some(source) {
        if let Some(l) = layers.iter_mut().find(|l| l.id == id) { l.mask_source_id = Some(source); }
    }
}

/// A clipped layer that no longer sits in the contiguous stack above its base stops clipping.
fn release_detached_clipping(layers: &mut [Layer]) {
    let mut release = HashSet::new();
    let parents: HashSet<Option<Uuid>> = layers.iter().map(|l| l.parent_id).collect();
    for parent in parents {
        let mut base: Option<Uuid> = None;
        for layer in layers.iter().filter(|l| l.parent_id == parent) {
            if let Some(source) = layer.mask_source_id {
                if Some(source) != base { release.insert(layer.id); base = Some(layer.id); }
            } else { base = if layer.is_group { None } else { Some(layer.id) }; }
        }
    }
    for l in layers.iter_mut() { if release.contains(&l.id) { l.mask_source_id = None; } }
}

pub fn place_layer(doc: &mut Document, id: Uuid, parent: Option<Uuid>, above: Option<Uuid>, at_bottom: bool) -> Result<(), CommandError> {
    if !can_place(doc, id, parent) || above == Some(id) { return Err(CommandError::Argument("cannot place the layer there".into())); }
    let mut layers = doc.layers.clone();
    let index = layers.iter().position(|l| l.id == id).ok_or(CommandError::NoLayer)?;
    let mut layer = layers.remove(index);
    layer.parent_id = parent;
    let mut insertion = if at_bottom { 0 } else { layers.len() };
    if let Some(target) = above {
        let t = layers.iter().position(|l| l.id == target && l.parent_id == parent).ok_or(CommandError::Argument("drop target is not in that folder".into()))?;
        insertion = t + 1;
    }
    layers.insert(insertion, layer);
    adopt_clipping(&mut layers, id);
    release_detached_clipping(&mut layers);
    let previous = std::mem::replace(&mut doc.layers, layers);
    if let Err(e) = validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(id);
    Ok(())
}

/// Swaps the layer with its sibling `offset` steps up (+) or down (-) the stack.
pub fn move_layer_by(doc: &mut Document, id: Uuid, offset: i32) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == id).ok_or(CommandError::NoLayer)? as i32;
    let other = index + offset;
    if other < 0 || other >= siblings.len() as i32 { return Err(CommandError::Argument("no sibling in that direction".into())); }
    let a = doc.index_of(id).unwrap(); let b = doc.index_of(siblings[other as usize]).unwrap();
    doc.layers.swap(a, b);
    Ok(())
}

pub fn duplicate_layer(doc: &mut Document, id: Uuid) -> Result<Uuid, CommandError> {
    let index = doc.index_of(id).ok_or(CommandError::NoLayer)?;
    if doc.layers[index].is_group { return Err(CommandError::Argument("folders are not duplicated this way".into())); }
    if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
    let mut copy = doc.layers[index].clone();
    copy.id = Uuid::new_v4();
    copy.name = format!("{} copy", copy.name);
    let cid = copy.id;
    doc.layers.insert(index + 1, copy);
    doc.active_layer_id = Some(cid);
    Ok(cid)
}

pub fn duplicate_layer_to(doc: &mut Document, id: Uuid, parent: Option<Uuid>, above: Option<Uuid>, at_bottom: bool) -> Result<Uuid, CommandError> {
    if !can_place(doc, id, parent) { return Err(CommandError::Argument("cannot place the copy there".into())); }
    let copy = duplicate_layer(doc, id)?;
    place_layer(doc, copy, parent, above, at_bottom)?;
    Ok(copy)
}

/// Layers outside `ids` (and their descendants) that clip to something being removed.
pub fn clip_dependents(doc: &Document, ids: &[Uuid]) -> Vec<Uuid> {
    let mut removed: HashSet<Uuid> = HashSet::new();
    for id in ids { removed.insert(*id); removed.extend(doc.descendants(*id)); }
    doc.layers.iter().filter(|l| !removed.contains(&l.id) && l.mask_source_id.map_or(false, |s| removed.contains(&s))).map(|l| l.id).collect()
}

/// The target's pixels multiplied by its clipping source's coverage, in the target's own grid.
pub fn bake_clip(doc: &Document, target: Uuid) -> Option<Raster> {
    let layer = doc.layer(target)?;
    let source = layer.mask_source_id?;
    let raster = layer.pixels.as_ref()?;
    let mut plan = render_plan(doc, None);
    ensure_source(doc, &mut plan, source);
    let to_doc = layer.transform.pixel_to_document(raster.width, raster.height);
    let mut data = raster.bytes().to_vec();
    for y in 0..raster.height { for x in 0..raster.width {
        let p = to_doc.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
        let k = source_coverage_at(doc, &plan, source, p).clamp(0.0, 1.0);
        let i = ((y * raster.width + x) * 4) as usize;
        for c in 0..4 { data[i + c] = (data[i + c] as f32 * k).round() as u8; }
    }}
    Some(Raster::from_premultiplied(raster.width, raster.height, data))
}

/// Deletes `ids` with their contents as one change. `bake` keeps dependants' masked look in their pixels; otherwise the links are removed.
pub fn delete_layers(doc: &mut Document, ids: &[Uuid], bake: bool) -> Result<(), CommandError> {
    let mut baked: HashMap<Uuid, Raster> = HashMap::new();
    if bake { for t in clip_dependents(doc, ids) { if let Some(r) = bake_clip(doc, t) { baked.insert(t, r); } } }
    for id in ids {
        if doc.layer(*id).is_none() { continue; }
        let index = doc.index_of(*id).unwrap();
        let mut removed: HashSet<Uuid> = HashSet::from([*id]);
        removed.extend(doc.descendants(*id));
        doc.layers.retain(|l| !removed.contains(&l.id));
        for l in &mut doc.layers {
            if l.mask_source_id.map_or(false, |s| removed.contains(&s)) {
                l.mask_source_id = None;
                if let Some(r) = baked.remove(&l.id) { l.set_pixels(Some(r)); }
            }
        }
        if doc.active_layer_id.map_or(false, |a| removed.contains(&a)) {
            doc.active_layer_id = if doc.layers.is_empty() { None } else if index > 0 { Some(doc.layers[(index - 1).min(doc.layers.len() - 1)].id) } else { Some(doc.layers[0].id) };
        }
    }
    Ok(())
}

pub fn can_link_mask(doc: &Document, source: Uuid, target: Uuid) -> bool {
    if source == target { return false; }
    let (Some(s), Some(t)) = (doc.layer(source), doc.layer(target)) else { return false; };
    if s.is_group || t.is_group || s.extra.adjustment.is_some() { return false; }
    let mut probe = doc.clone();
    probe.layer_mut(target).unwrap().mask_source_id = Some(source);
    validate(&probe).is_ok()
}

pub fn link_mask(doc: &mut Document, source: Uuid, target: Uuid) -> Result<(), CommandError> {
    if !can_link_mask(doc, source, target) { return Err(CommandError::Argument("cannot clip to that layer".into())); }
    doc.layer_mut(target).unwrap().mask_source_id = Some(source);
    Ok(())
}

/// Releasing a layer releases the clipped siblings above it that share its base.
pub fn release_clipping(doc: &mut Document, target: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(target).ok_or(CommandError::NoLayer)?.clone();
    let source = layer.mask_source_id.ok_or(CommandError::Argument("the layer is not clipped".into()))?;
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == target).unwrap();
    let mut releases = Vec::new();
    for id in &siblings[index..] {
        let l = doc.layer(*id).unwrap();
        if *id == target || l.mask_source_id == Some(source) { releases.push(*id); } else { break; }
    }
    for id in releases { doc.layer_mut(id).unwrap().mask_source_id = None; }
    Ok(())
}

pub fn can_toggle_clipping(doc: &Document, id: Uuid) -> bool {
    let Some(layer) = doc.layer(id) else { return false; };
    if layer.is_group { return false; }
    if layer.mask_source_id.is_some() { return true; }
    let siblings = doc.siblings(layer.parent_id);
    let Some(index) = siblings.iter().position(|s| *s == id) else { return false; };
    if index == 0 { return false; }
    let below = doc.layer(siblings[index - 1]).unwrap();
    if below.is_group { return false; }
    can_link_mask(doc, below.mask_source_id.unwrap_or(below.id), id)
}

/// Clips to the next lower sibling (sharing its base when it is clipped), or releases.
pub fn toggle_clipping(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    if layer.mask_source_id.is_some() { return release_clipping(doc, id); }
    if !can_toggle_clipping(doc, id) { return Err(CommandError::Argument("nothing below to clip to".into())); }
    let siblings = doc.siblings(layer.parent_id);
    let index = siblings.iter().position(|s| *s == id).unwrap();
    let below = doc.layer(siblings[index - 1]).unwrap().clone();
    link_mask(doc, below.mask_source_id.unwrap_or(below.id), id)
}
```

Simplify the `branches` closure in `group_layers` before running the tests to exactly this (the sketch above carries dead conditions):
```rust
    let branches: Vec<Uuid> = ordered.iter().map(|id| {
        let mut branch = *id; let mut steps = 0;
        while let Some(next) = doc.layer(branch).and_then(|l| l.parent_id) {
            if Some(next) == parent || steps > MAX_NESTING { break; }
            branch = next; steps += 1;
        }
        branch
    }).collect();
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including the 7 in `hierarchy.rs`.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): opacity, blend mode, folders, placement, duplicate, delete with bake, clipping links"
```

---

### Task 6: Transform and distortion operations

**Files:**
- Create: `engine/src/ops/transform.rs`, `engine/src/ops/distort.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod transform; pub mod distort;`)
- Test: `engine/tests/transform.rs`, `engine/tests/distort.rs`

**Interfaces:**
- `ops::transform::{set_transform(doc, id, transform: LayerTransform) -> Result<(), CommandError>` (rejects groups and invalid transforms; the mask placement follows `Mask::follow`), `transform_group(doc, ids: &[Uuid], bounds: &LayerTransform, draft: &LayerTransform) -> Result<()>` (each pixel layer in `ids` moves by `original.following(bounds, draft)`; masks follow), `flip_layers(doc, ids, horizontal: bool) -> Result<()>` (one layer flips about its centre; several about the centre of the upright box around them; masks follow), `nudge(doc, ids, dx, dy) -> Result<()>`, `set_mask_placement(doc, id, placement: LayerTransform) -> Result<()>` (an unlinked mask moved alone; a placement equal to the layer's transform stores None), `group_box(doc, ids) -> Option<LayerTransform>` (upright box around the pixel layers' corners, min size 1)}`.
- `ops::distort::{warp(raster: &Raster, transform: &LayerTransform, corners: &[Point;4], nearest: bool) -> Result<(Raster, LayerTransform), ProjectError>` (resamples into the shape's whole-pixel bounds; `TooLarge` beyond limits; `Invalid` for unusable corners), `warp_mask(mask: &GrayRaster, transform, corners, background: u8) -> Result<(GrayRaster, LayerTransform), ProjectError>` (a 1x1 mask passes through unchanged with the placed transform; background outside the shape), `warp_trimmed(raster, transform, corners, nearest) -> Result<(Raster, LayerTransform, (u32,u32,u32,u32)), ProjectError>` (cropped to alpha bounds; the crop rect is returned for masks), `distort_layer(doc, id, transform: &LayerTransform, corners: &[Point;4]) -> Result<(), CommandError>`, `distort_group(doc, ids, bounds, draft, corners) -> Result<()>` (each layer through `following` plus `Homography::carried`)}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/transform.rs`:
```rust
use compositor_engine::ops::transform::*;
use compositor_engine::*;

fn near(a: f64, b: f64) -> bool { (a - b).abs() < 1e-6 }
fn layer(name: &str, x: f64, y: f64, w: u32, h: u32) -> Layer { Layer::with_pixels(name, Raster::new_transparent(w, h), Point { x, y }) }
fn t(x: f64, y: f64, w: f64, h: f64) -> LayerTransform { LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h }) }
fn gray(w: u32, h: u32) -> GrayRaster { GrayRaster::from_bytes(w, h, vec![255; (w * h) as usize]) }

#[test]
fn set_transform_moves_linked_mask_and_leaves_unlinked_mask() {
    let mut d = Document::new(400, 200);
    let mut l = layer("L", 0.0, 0.0, 400, 200);
    l.set_mask(Some(Mask { pixels: gray(400, 200), enabled: true, placement: None, linked: None }));
    let id = l.id; d.layers.push(l);
    let moved = t(100.0, 0.0, 400.0, 200.0);
    set_transform(&mut d, id, moved).unwrap();
    assert_eq!(d.layer(id).unwrap().transform, moved);
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, None, "a linked mask keeps covering its layer");
    d.layer_mut(id).unwrap().mask_mut().unwrap().linked = Some(false);
    let moved2 = t(150.0, 0.0, 400.0, 200.0);
    set_transform(&mut d, id, moved2).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, Some(moved), "an unlinked mask stays on the canvas");
    let mut bad = moved2; bad.size.width = 0.0;
    assert!(set_transform(&mut d, id, bad).is_err());
    let mut g = Layer::blank("G", d.size()); g.is_group = true; let gid = g.id; d.layers.push(g);
    assert!(set_transform(&mut d, gid, moved).is_err());
}

#[test]
fn mask_moves_alone_and_relinks() {
    let mut d = Document::new(400, 200);
    let mut l = layer("L", 0.0, 0.0, 400, 200);
    l.set_mask(Some(Mask { pixels: gray(400, 200), enabled: true, placement: None, linked: Some(false) }));
    let id = l.id; d.layers.push(l);
    let moved = t(100.0, 0.0, 400.0, 200.0);
    set_mask_placement(&mut d, id, moved).unwrap();
    assert_eq!(d.layer(id).unwrap().transform, t(0.0, 0.0, 400.0, 200.0));
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, Some(moved));
    set_mask_placement(&mut d, id, t(0.0, 0.0, 400.0, 200.0)).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().placement, None, "back on its layer collapses to none");
    set_mask_placement(&mut d, id, moved).unwrap();
    d.layer_mut(id).unwrap().mask_mut().unwrap().linked = None;
    set_transform(&mut d, id, t(50.0, 0.0, 400.0, 200.0)).unwrap();
    let p = d.layer(id).unwrap().mask.as_ref().unwrap().placement.unwrap();
    assert!(near(p.origin.x, 150.0) && near(p.origin.y, 0.0), "relinked, the placed mask follows: {p:?}");
}

#[test]
fn group_transform_and_box() {
    let mut d = Document::new(400, 200);
    let a = layer("A", 0.0, 0.0, 100, 50); let b = layer("B", 100.0, 50.0, 100, 50);
    let (aid, bid) = (a.id, b.id);
    d.layers.push(a); d.layers.push(b);
    let bounds = group_box(&d, &[aid, bid]).unwrap();
    assert_eq!(bounds, t(0.0, 0.0, 200.0, 100.0));
    let draft = t(10.0, 10.0, 400.0, 200.0);
    transform_group(&mut d, &[aid, bid], &bounds, &draft).unwrap();
    let ta = d.layer(aid).unwrap().transform; let tb = d.layer(bid).unwrap().transform;
    assert!(near(ta.origin.x, 10.0) && near(ta.size.width, 200.0) && near(ta.size.height, 100.0));
    assert!(near(tb.origin.x, 210.0) && near(tb.origin.y, 110.0));
    nudge(&mut d, &[aid, bid], 1.0, -2.0).unwrap();
    assert!(near(d.layer(aid).unwrap().transform.origin.x, 11.0) && near(d.layer(bid).unwrap().transform.origin.y, 108.0));
}

#[test]
fn flip_one_layer_about_itself_and_several_about_their_box() {
    let mut d = Document::new(400, 200);
    let mut a = layer("A", 0.0, 0.0, 100, 50); a.transform.rotation = 30.0;
    let b = layer("B", 300.0, 0.0, 100, 50);
    let (aid, bid) = (a.id, b.id);
    d.layers.push(a); d.layers.push(b);
    flip_layers(&mut d, &[aid], true).unwrap();
    let ta = d.layer(aid).unwrap().transform;
    assert!(ta.flip_x && near(ta.rotation, -30.0) && near(ta.origin.x, 0.0));
    flip_layers(&mut d, &[aid, bid], true).unwrap();
    // The box spans x 0..400 (centre 200): A's centre 50 -> 350, B's 350 -> 50.
    assert!(near(d.layer(aid).unwrap().transform.center().x, 350.0));
    assert!(near(d.layer(bid).unwrap().transform.center().x, 50.0));
    assert!(!d.layer(aid).unwrap().transform.flip_x && d.layer(bid).unwrap().transform.flip_x);
}
```

`engine/tests/distort.rs`:
```rust
use compositor_engine::ops::distort::*;
use compositor_engine::*;

fn red(w: u32, h: u32) -> Raster { Raster::from_premultiplied(w, h, [255u8, 0, 0, 255].repeat((w * h) as usize)) }
fn t(x: f64, y: f64, w: f64, h: f64) -> LayerTransform { let mut t = LayerTransform::axis_aligned(Point { x, y }, Size { width: w, height: h }); t.sampling = Sampling::Nearest; t }
const SHAPE: [Point; 4] = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];

#[test]
fn warp_places_pixels_over_the_shape_bounds() {
    let (out, placed) = warp(&red(20, 20), &t(10.0, 10.0, 20.0, 20.0), &SHAPE, true).unwrap();
    assert_eq!((placed.origin, placed.size), (Point { x: 10.0, y: 10.0 }, Size { width: 50.0, height: 20.0 }));
    assert_eq!(placed.rotation, 0.0);
    assert_eq!((out.width, out.height), (50, 20));
    assert_eq!(out.pixel(40, 2)[3], 255, "inside the stretched top-right");
    assert_eq!(out.pixel(5, 15)[3], 255);
    assert_eq!(out.pixel(40, 18)[3], 0, "outside the slanted right edge");
    assert!(warp(&red(2, 2), &t(0.0, 0.0, 2.0, 2.0), &[SHAPE[0], SHAPE[2], SHAPE[1], SHAPE[3]], true).is_err());
}

#[test]
fn warp_trimmed_hugs_visible_pixels_and_mask_keeps_background() {
    let mut data = vec![0u8; 40 * 20 * 4];
    for y in 5..15 { for x in 15..25 { let i = (y * 40 + x) * 4; data[i] = 255; data[i + 3] = 255; } }
    let raster = Raster::from_premultiplied(40, 20, data);
    let shape = [Point { x: 10.0, y: 10.0 }, Point { x: 60.0, y: 10.0 }, Point { x: 50.0, y: 30.0 }, Point { x: 10.0, y: 30.0 }];
    let (out, placed, crop) = warp_trimmed(&raster, &t(10.0, 10.0, 40.0, 20.0), &shape, true).unwrap();
    assert!(placed.size.width < 20.0 && placed.size.height <= 12.0, "{placed:?}");
    assert!(placed.origin.x >= 20.0 && placed.origin.y >= 14.0);
    assert_eq!((out.width as f64, out.height as f64), (placed.size.width, placed.size.height));
    assert!(crop.2 > crop.0 && crop.3 > crop.1);
    let uniform = GrayRaster::from_bytes(1, 1, vec![255]);
    let (m, mp) = warp_mask(&uniform, &t(10.0, 10.0, 40.0, 20.0), &shape, 255).unwrap();
    assert_eq!((m.width, m.height), (1, 1));
    assert_eq!(mp.size, Size { width: 50.0, height: 20.0 });
    let black_edge = GrayRaster::from_bytes(4, 2, vec![0; 8]);
    let (m, _) = warp_mask(&black_edge, &t(10.0, 10.0, 40.0, 20.0), &shape, 255).unwrap();
    assert_eq!(m.bytes()[(19 * 50 + 49)], 255, "outside the shape shows the background");
}

#[test]
fn distort_layer_resamples_pixels_and_linked_mask_as_an_axis_aligned_layer() {
    let mut d = Document::new(100, 60);
    let mut l = Layer::with_pixels("Red", red(20, 20), Point { x: 10.0, y: 10.0 });
    l.transform.sampling = Sampling::Nearest;
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(20, 20, vec![255; 400]), enabled: true, placement: None, linked: None }));
    let id = l.id; d.layers.push(l);
    distort_layer(&mut d, id, &t(10.0, 10.0, 20.0, 20.0), &SHAPE).unwrap();
    let l = d.layer(id).unwrap();
    assert_eq!((l.transform.origin, l.transform.size, l.transform.rotation), (Point { x: 10.0, y: 10.0 }, Size { width: 50.0, height: 20.0 }, 0.0));
    assert_eq!(l.pixels_revision, 2);
    let m = l.mask.as_ref().unwrap();
    assert_eq!((m.pixels.width, m.pixels.height), (50, 20), "the mask is warped and cropped with the pixels");
    let out = composite(&d, Rect { x: 0.0, y: 0.0, width: 100.0, height: 60.0 }, 100, 60);
    assert_eq!(out.pixel(50, 12)[3], 255);
    assert_eq!(out.pixel(50, 28)[3], 0);
    // An unlinked mask stays where it was on the document.
    let mut d2 = Document::new(100, 60);
    let mut l2 = Layer::with_pixels("Red", red(20, 20), Point { x: 10.0, y: 10.0 });
    l2.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(20, 20, vec![255; 400]), enabled: true, placement: None, linked: Some(false) }));
    let id2 = l2.id; d2.layers.push(l2);
    distort_layer(&mut d2, id2, &t(10.0, 10.0, 20.0, 20.0), &SHAPE).unwrap();
    assert_eq!(d2.layer(id2).unwrap().mask.as_ref().unwrap().placement, Some(t(10.0, 10.0, 20.0, 20.0)));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test transform --test distort`
Expected: compile errors.

- [ ] **Step 3: Implement transform ops**

`engine/src/ops/transform.rs`:
```rust
use crate::*;
use uuid::Uuid;

fn pixel_layer(doc: &Document, id: Uuid) -> Result<&Layer, CommandError> {
    let l = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if l.is_group { return Err(CommandError::Argument("folders are not transformed directly".into())); }
    Ok(l)
}

/// Moves one layer; its mask follows the placement rule.
pub fn set_transform(doc: &mut Document, id: Uuid, transform: LayerTransform) -> Result<(), CommandError> {
    pixel_layer(doc, id)?;
    if !transform.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    let layer = doc.layer_mut(id).unwrap();
    let old = layer.transform;
    if let Some(mask) = &layer.mask {
        let placement = mask.follow(&old, &transform);
        if placement != mask.placement { layer.mask_mut().unwrap().placement = placement; }
    }
    layer.transform = transform;
    Ok(())
}

/// The upright box around the pixel layers among `ids` (folders contribute their pixel descendants).
pub fn group_box(doc: &Document, ids: &[Uuid]) -> Option<LayerTransform> {
    let members = members(doc, ids);
    let mut pts = Vec::new();
    for id in &members { pts.extend(Homography::corners_of(&doc.layer(*id)?.transform)); }
    if pts.is_empty() { return None; }
    let min_x = pts.iter().map(|p| p.x).fold(f64::INFINITY, f64::min); let max_x = pts.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = pts.iter().map(|p| p.y).fold(f64::INFINITY, f64::min); let max_y = pts.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    Some(LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: (max_x - min_x).max(1.0), height: (max_y - min_y).max(1.0) }))
}

/// Visible pixel layers selected directly or inside selected folders, in array order.
pub fn members(doc: &Document, ids: &[Uuid]) -> Vec<Uuid> {
    let visible = doc.visible_ids();
    doc.layers.iter().filter(|l| {
        if l.is_group || l.pixels.is_none() || !visible.contains(&l.id) { return false; }
        let mut current = Some(l.id); let mut steps = 0;
        while let Some(id) = current { if ids.contains(&id) { return true; } current = doc.layer(id).and_then(|x| x.parent_id); steps += 1; if steps > MAX_NESTING { break; } }
        false
    }).map(|l| l.id).collect()
}

pub fn transform_group(doc: &mut Document, ids: &[Uuid], bounds: &LayerTransform, draft: &LayerTransform) -> Result<(), CommandError> {
    if !draft.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    for id in members(doc, ids) {
        let moved = doc.layer(id).unwrap().transform.following(bounds, draft);
        if moved.is_valid() { set_transform(doc, id, moved)?; }
    }
    Ok(())
}

pub fn nudge(doc: &mut Document, ids: &[Uuid], dx: f64, dy: f64) -> Result<(), CommandError> {
    for id in members(doc, ids) {
        let mut t = doc.layer(id).unwrap().transform;
        t.origin.x += dx; t.origin.y += dy;
        set_transform(doc, id, t)?;
    }
    Ok(())
}

pub fn flip_layers(doc: &mut Document, ids: &[Uuid], horizontal: bool) -> Result<(), CommandError> {
    let members = members(doc, ids);
    if members.is_empty() { return Err(CommandError::Argument("nothing to flip".into())); }
    let axis = if members.len() == 1 {
        let c = doc.layer(members[0]).unwrap().transform.center();
        if horizontal { c.x } else { c.y }
    } else {
        let b = group_box(doc, ids).unwrap().center();
        if horizontal { b.x } else { b.y }
    };
    for id in members {
        let flipped = doc.layer(id).unwrap().transform.mirrored(horizontal, axis);
        set_transform(doc, id, flipped)?;
    }
    Ok(())
}

/// An unlinked mask moved on its own: the new placement, pixels untouched.
pub fn set_mask_placement(doc: &mut Document, id: Uuid, placement: LayerTransform) -> Result<(), CommandError> {
    if !placement.is_valid() { return Err(CommandError::Argument("transform out of range".into())); }
    let layer = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if layer.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
    let value = if placement.same_placement(&layer.transform) { None } else { Some(placement) };
    if layer.mask.as_ref().unwrap().placement != value { layer.mask_mut().unwrap().placement = value; }
    Ok(())
}
```

- [ ] **Step 4: Implement distortion**

`engine/src/ops/distort.rs`:
```rust
use crate::*;
use uuid::Uuid;

fn shape_bounds(corners: &[Point; 4]) -> Result<(f64, f64, u32, u32), ProjectError> {
    if !Homography::is_usable(corners) { return Err(ProjectError::Invalid); }
    let xs = corners.iter().map(|p| p.x); let ys = corners.iter().map(|p| p.y);
    let min_x = xs.clone().fold(f64::INFINITY, f64::min).floor(); let max_x = xs.fold(f64::NEG_INFINITY, f64::max).ceil();
    let min_y = ys.clone().fold(f64::INFINITY, f64::min).floor(); let max_y = ys.fold(f64::NEG_INFINITY, f64::max).ceil();
    let w = (max_x - min_x).max(1.0); let h = (max_y - min_y).max(1.0);
    if w > MAX_SIDE as f64 || h > MAX_SIDE as f64 || w * h > MAX_PIXELS as f64 { return Err(ProjectError::TooLarge); }
    Ok((min_x, min_y, w as u32, h as u32))
}

/// Pixel coordinates (in a `w` x `h` grid shown through `transform`, flips applied) for each output pixel of the shape's bounds.
fn inverse_map(transform: &LayerTransform, corners: &[Point; 4], w: u32, h: u32) -> Option<impl Fn(f64, f64) -> Point> {
    let inv = Homography::unit_to(corners).invert()?;
    let (fx, fy) = (transform.flip_x, transform.flip_y);
    Some(move |x: f64, y: f64| {
        let u = inv.apply(Point { x, y });
        let ux = if fx { 1.0 - u.x } else { u.x }; let uy = if fy { 1.0 - u.y } else { u.y };
        Point { x: ux * w as f64, y: uy * h as f64 }
    })
}

/// `raster`, shown through `transform`, resampled so its corners land on `corners`.
pub fn warp(raster: &Raster, transform: &LayerTransform, corners: &[Point; 4], nearest: bool) -> Result<(Raster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    let map = inverse_map(transform, corners, raster.width, raster.height).ok_or(ProjectError::Invalid)?;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for y in 0..h { for x in 0..w {
        let p = map(min_x + x as f64 + 0.5, min_y + y as f64 + 0.5);
        if p.x < 0.0 || p.y < 0.0 || p.x >= raster.width as f64 || p.y >= raster.height as f64 { continue; }
        let s = compositor::sample(raster, p.x, p.y, nearest);
        let i = ((y * w + x) * 4) as usize;
        for c in 0..4 { data[i + c] = (s[c] * 255.0).round().clamp(0.0, 255.0) as u8; }
    }}
    Ok((Raster::from_premultiplied(w, h, data), placed))
}

/// A mask warped like `warp`, `background` outside the shape; a uniform mask passes through.
pub fn warp_mask(mask: &GrayRaster, transform: &LayerTransform, corners: &[Point; 4], background: u8) -> Result<(GrayRaster, LayerTransform), ProjectError> {
    let (min_x, min_y, w, h) = shape_bounds(corners)?;
    let mut placed = LayerTransform::axis_aligned(Point { x: min_x, y: min_y }, Size { width: w as f64, height: h as f64 });
    placed.sampling = transform.sampling;
    if mask.width == 1 && mask.height == 1 { return Ok((mask.clone(), placed)); }
    let map = inverse_map(transform, corners, mask.width, mask.height).ok_or(ProjectError::Invalid)?;
    let nearest = transform.sampling == Sampling::Nearest;
    let mut data = vec![background; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let p = map(min_x + x as f64 + 0.5, min_y + y as f64 + 0.5);
        if p.x < 0.0 || p.y < 0.0 || p.x >= mask.width as f64 || p.y >= mask.height as f64 { continue; }
        let v = if nearest { mask.bytes()[(p.y as u32 * mask.width + p.x as u32) as usize] as f32 }
                else { gray_bilinear(mask, p.x, p.y) };
        data[(y * w + x) as usize] = v.round().clamp(0.0, 255.0) as u8;
    }}
    Ok((GrayRaster::from_bytes(w, h, data), placed))
}

fn gray_bilinear(mask: &GrayRaster, x: f64, y: f64) -> f32 {
    let w = mask.width as i64; let h = mask.height as i64;
    let fetch = |px: i64, py: i64| mask.bytes()[(py.clamp(0, h - 1) * w + px.clamp(0, w - 1)) as usize] as f32;
    let fx = x - 0.5; let fy = y - 0.5; let xu = fx.floor() as i64; let yu = fy.floor() as i64;
    let tx = (fx - xu as f64) as f32; let ty = (fy - yu as f64) as f32;
    (fetch(xu, yu) * (1.0 - tx) + fetch(xu + 1, yu) * tx) * (1.0 - ty) + (fetch(xu, yu + 1) * (1.0 - tx) + fetch(xu + 1, yu + 1) * tx) * ty
}

/// A warp cropped to its visible pixels; the crop (x0, y0, x1, y1) is in the warp's pixels.
pub fn warp_trimmed(raster: &Raster, transform: &LayerTransform, corners: &[Point; 4], nearest: bool) -> Result<(Raster, LayerTransform, (u32, u32, u32, u32)), ProjectError> {
    let (warped, placed) = warp(raster, transform, corners, nearest)?;
    let full = (0, 0, warped.width, warped.height);
    match compositor::alpha_bounds(&warped) {
        Some(crop) if crop != full => {
            let cropped = warped.cropped(crop.0, crop.1, crop.2 - crop.0, crop.3 - crop.1);
            let mut t = placed;
            t.origin = Point { x: placed.origin.x + crop.0 as f64, y: placed.origin.y + crop.1 as f64 };
            t.size = Size { width: (crop.2 - crop.0) as f64, height: (crop.3 - crop.1) as f64 };
            Ok((cropped, t, crop))
        }
        _ => Ok((warped, placed, full)),
    }
}

fn distort_at(doc: &mut Document, id: Uuid, transform: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?.clone();
    let Some(raster) = &layer.pixels else { return Ok(()); };
    let nearest = transform.sampling == Sampling::Nearest;
    let (pixels, placed, crop) = warp_trimmed(raster, transform, corners, nearest)?;
    let mask = match &layer.mask {
        Some(m) if m.placement.is_none() && m.is_linked() => {
            let (warped, _) = warp_mask(&m.pixels, transform, corners, m.background())?;
            let pixels = if warped.width == 1 && warped.height == 1 { warped } else {
                let (x0, y0, x1, y1) = crop;
                let mut data = Vec::with_capacity(((x1 - x0) * (y1 - y0)) as usize);
                for y in y0..y1 { data.extend_from_slice(&warped.bytes()[(y * warped.width + x0) as usize..(y * warped.width + x1) as usize]); }
                GrayRaster::from_bytes(x1 - x0, y1 - y0, data)
            };
            Some(Mask { pixels, enabled: m.enabled, placement: None, linked: m.linked })
        }
        Some(m) if m.is_linked() => {
            // A linked mask placed apart takes the same perspective over its own bounds.
            let placement = m.placement.unwrap().following(&layer.transform, transform);
            let carried = Homography::carried(&placement, transform, corners);
            if Homography::is_usable(&carried) {
                let (warped, moved) = warp_mask(&m.pixels, &placement, &carried, m.background())?;
                Some(Mask { pixels: warped, enabled: m.enabled, placement: Some(moved), linked: m.linked })
            } else { Some(m.clone()) }
        }
        Some(m) => Some(Mask { placement: Some(m.placement.unwrap_or(layer.transform)), ..m.clone() }),
        None => None,
    };
    let l = doc.layer_mut(id).unwrap();
    l.set_pixels(Some(pixels));
    l.transform = placed;
    l.set_mask(mask);
    Ok(())
}

pub fn distort_layer(doc: &mut Document, id: Uuid, transform: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    if doc.layer(id).map_or(true, |l| l.is_group) { return Err(CommandError::NoLayer); }
    if !Homography::is_usable(corners) { return Err(CommandError::Argument("twisted or collapsed shape".into())); }
    distort_at(doc, id, transform, corners)
}

pub fn distort_group(doc: &mut Document, ids: &[Uuid], bounds: &LayerTransform, draft: &LayerTransform, corners: &[Point; 4]) -> Result<(), CommandError> {
    if !Homography::is_usable(corners) { return Err(CommandError::Argument("twisted or collapsed shape".into())); }
    for id in super::transform::members(doc, ids) {
        let moved = doc.layer(id).unwrap().transform.following(bounds, draft);
        let carried = Homography::carried(&moved, draft, corners);
        if Homography::is_usable(&carried) { distort_at(doc, id, &moved, &carried)?; }
    }
    Ok(())
}
```
`compositor::sample` and `compositor::alpha_bounds` must be `pub` (they are).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 4 in `transform.rs` and 3 in `distort.rs`.

- [ ] **Step 6: Commit**

```
git add engine
git commit -m "feat(engine): layer transforms with mask placement, group transforms, flips, free distortion"
```

---

### Task 7: Mask operations

**Files:**
- Create: `engine/src/ops/masks.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod masks;`)
- Test: `engine/tests/masks.rs`

**Interfaces:**
- `ops::masks::{add_mask(doc, id, revealing: bool) -> Result<(), CommandError>` (layers and folders alike; a uniform 1x1 mask, 255 or 0; rejects a layer that already has one), `delete_mask(doc, id)`, `set_mask_enabled(doc, id, enabled)`, `set_mask_linked(doc, id, linked)`, `invert_mask(doc, id)`, `fill_mask(doc, id, white: bool)` (keeps the grid), `blur_mask(doc, id, radius: f64)` (Gaussian, sigma = radius, kernel to 3 sigma; a uniform mask stays uniform; radius 0.1 to 1000), `copy_mask(doc, from, to)` (a copy placed where it sits on the document, replacing any mask on `to`; folders cannot receive one), `expand_uniform(doc, id) -> Result<(), CommandError>` (turns a 1x1 mask into the layer's pixel grid, needed before blur when painting arrives in Phase 4; `blur_mask` calls it first when the mask is 1x1 and the layer has pixels: the result is still uniform, so `blur_mask` simply returns)}`.
- `blur_gray(mask: &GrayRaster, sigma: f64) -> GrayRaster` (public helper).

- [ ] **Step 1: Write the failing tests**

`engine/tests/masks.rs`:
```rust
use compositor_engine::ops::{hierarchy, masks};
use compositor_engine::*;

fn red_doc() -> (Document, uuid::Uuid) {
    let mut d = Document::new(2, 2);
    let mut l = Layer::with_pixels("Red", Raster::from_premultiplied(2, 2, [255u8, 0, 0, 255].repeat(4)), Point { x: 0.0, y: 0.0 });
    l.transform.sampling = Sampling::Nearest;
    let id = l.id; d.active_layer_id = Some(id); d.layers.push(l);
    (d, id)
}
fn alphas(d: &Document) -> Vec<u8> { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height).bytes().chunks_exact(4).map(|p| p[3]).collect() }

#[test]
fn add_disable_delete_and_folder_masks() {
    let (mut d, id) = red_doc();
    masks::add_mask(&mut d, id, false).unwrap();
    let m = d.layer(id).unwrap().mask.as_ref().unwrap();
    assert_eq!((m.pixels.width, m.pixels.height, m.pixels.bytes()[0]), (1, 1, 0));
    assert!(masks::add_mask(&mut d, id, true).is_err(), "never overwrite an existing mask");
    assert_eq!(alphas(&d), [0; 4]);
    masks::set_mask_enabled(&mut d, id, false).unwrap();
    assert_eq!(alphas(&d), [255; 4]);
    masks::delete_mask(&mut d, id).unwrap();
    assert!(d.layer(id).unwrap().mask.is_none());
    let g = hierarchy::group_layers(&mut d, &[id]).unwrap();
    masks::add_mask(&mut d, g, false).unwrap();
    assert!(d.layer(g).unwrap().mask.is_some());
    assert_eq!(alphas(&d), [0; 4]);
}

#[test]
fn invert_fill_and_link() {
    let (mut d, id) = red_doc();
    d.layer_mut(id).unwrap().set_mask(Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![255, 0, 128, 255]), enabled: true, placement: None, linked: None }));
    masks::invert_mask(&mut d, id).unwrap();
    assert_eq!(alphas(&d), [0, 255, 127, 0]);
    masks::fill_mask(&mut d, id, true).unwrap();
    assert_eq!(alphas(&d), [255; 4]);
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().pixels.width, 2, "fill keeps the grid");
    masks::set_mask_linked(&mut d, id, false).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().linked, Some(false));
    masks::set_mask_linked(&mut d, id, true).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().linked, Some(true));
    let rev = d.layer(id).unwrap().mask_revision;
    masks::fill_mask(&mut d, id, false).unwrap();
    assert!(d.layer(id).unwrap().mask_revision > rev);
}

#[test]
fn blur_softens_edges_and_keeps_uniform_masks() {
    let mut data = vec![0u8; 20 * 20];
    for y in 0..20 { for x in 10..20 { data[y * 20 + x] = 255; } }
    let blurred = blur_gray(&GrayRaster::from_bytes(20, 20, data), 2.0);
    let row: Vec<u8> = blurred.bytes()[10 * 20..11 * 20].to_vec();
    assert!(row[9] > 20 && row[9] < 235 && row[10] > 20 && row[10] < 235, "{row:?}");
    assert!(row[0] < 3 && row[19] > 252);
    assert!(row.windows(2).all(|w| w[0] <= w[1]));
    let (mut d, id) = red_doc();
    masks::add_mask(&mut d, id, true).unwrap();
    masks::blur_mask(&mut d, id, 3.0).unwrap();
    assert_eq!(d.layer(id).unwrap().mask.as_ref().unwrap().pixels.width, 1);
    assert!(masks::blur_mask(&mut d, id, 0.0).is_err());
}

#[test]
fn copy_mask_places_it_where_it_sits() {
    let (mut d, a) = red_doc();
    let mut b = Layer::with_pixels("B", Raster::new_transparent(2, 2), Point { x: 1.0, y: 0.0 });
    let bid = b.id;
    b.transform.sampling = Sampling::Nearest;
    d.layers.push(b);
    d.layer_mut(a).unwrap().set_mask(Some(Mask { pixels: GrayRaster::from_bytes(2, 2, vec![255, 0, 128, 255]), enabled: true, placement: None, linked: None }));
    masks::copy_mask(&mut d, a, bid).unwrap();
    let m = d.layer(bid).unwrap().mask.as_ref().unwrap();
    assert_eq!(m.placement, Some(d.layer(a).unwrap().transform), "sits where the source mask sits");
    assert_eq!(m.pixels.bytes(), &[255, 0, 128, 255]);
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(masks::copy_mask(&mut d, a, g).is_err());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test masks`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/ops/masks.rs`:
```rust
use crate::*;
use uuid::Uuid;

fn layer_with_mask<'a>(doc: &'a mut Document, id: Uuid) -> Result<&'a mut Layer, CommandError> {
    let l = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if l.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
    Ok(l)
}

pub fn add_mask(doc: &mut Document, id: Uuid, revealing: bool) -> Result<(), CommandError> {
    let l = doc.layer_mut(id).ok_or(CommandError::NoLayer)?;
    if l.mask.is_some() { return Err(CommandError::Argument("the layer already has a mask".into())); }
    if doc.used_mask_pixels() + 1 > MAX_PIXELS { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let l = doc.layer_mut(id).unwrap();
    l.set_mask(Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![if revealing { 255 } else { 0 }]), enabled: true, placement: None, linked: None }));
    Ok(())
}

pub fn delete_mask(doc: &mut Document, id: Uuid) -> Result<(), CommandError> { layer_with_mask(doc, id)?.set_mask(None); Ok(()) }

pub fn set_mask_enabled(doc: &mut Document, id: Uuid, enabled: bool) -> Result<(), CommandError> {
    layer_with_mask(doc, id)?.mask_mut().unwrap().enabled = enabled; Ok(())
}

pub fn set_mask_linked(doc: &mut Document, id: Uuid, linked: bool) -> Result<(), CommandError> {
    layer_with_mask(doc, id)?.mask_mut().unwrap().linked = Some(linked); Ok(())
}

fn replace_pixels(doc: &mut Document, id: Uuid, f: impl FnOnce(&GrayRaster) -> GrayRaster) -> Result<(), CommandError> {
    let l = layer_with_mask(doc, id)?;
    let next = f(&l.mask.as_ref().unwrap().pixels);
    l.mask_mut().unwrap().pixels = next;
    Ok(())
}

pub fn invert_mask(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    replace_pixels(doc, id, |m| GrayRaster::from_bytes(m.width, m.height, m.bytes().iter().map(|v| 255 - v).collect()))
}

pub fn fill_mask(doc: &mut Document, id: Uuid, white: bool) -> Result<(), CommandError> {
    replace_pixels(doc, id, |m| GrayRaster::from_bytes(m.width, m.height, vec![if white { 255 } else { 0 }; (m.width * m.height) as usize]))
}

/// Separable Gaussian blur with edge clamping.
pub fn blur_gray(mask: &GrayRaster, sigma: f64) -> GrayRaster {
    if mask.is_uniform().is_some() || sigma <= 0.0 { return mask.clone(); }
    let radius = (sigma * 3.0).ceil() as i64;
    let kernel: Vec<f32> = (-radius..=radius).map(|i| (-(i * i) as f64 / (2.0 * sigma * sigma)).exp() as f32).collect();
    let sum: f32 = kernel.iter().sum();
    let (w, h) = (mask.width as i64, mask.height as i64);
    let src = mask.bytes();
    let mut tmp = vec![0f32; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = 0f32;
        for (k, weight) in kernel.iter().enumerate() { let sx = (x + k as i64 - radius).clamp(0, w - 1); acc += src[(y * w + sx) as usize] as f32 * weight; }
        tmp[(y * w + x) as usize] = acc / sum;
    }}
    let mut out = vec![0u8; (w * h) as usize];
    for y in 0..h { for x in 0..w {
        let mut acc = 0f32;
        for (k, weight) in kernel.iter().enumerate() { let sy = (y + k as i64 - radius).clamp(0, h - 1); acc += tmp[(sy * w + x) as usize] * weight; }
        out[(y * w + x) as usize] = (acc / sum).round().clamp(0.0, 255.0) as u8;
    }}
    GrayRaster::from_bytes(mask.width, mask.height, out)
}

pub fn blur_mask(doc: &mut Document, id: Uuid, radius: f64) -> Result<(), CommandError> {
    if !radius.is_finite() || !(0.1..=1000.0).contains(&radius) { return Err(CommandError::Argument("blur radius must be 0.1 to 1000".into())); }
    replace_pixels(doc, id, |m| blur_gray(m, radius))
}

/// A 1x1 mask expanded to the layer's pixel grid (the layer's rectangle when it has no pixels).
pub fn expand_uniform(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let l = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let Some(m) = &l.mask else { return Err(CommandError::Argument("the layer has no mask".into())); };
    if !m.is_uniform() { return Ok(()); }
    let (w, h) = l.pixels.as_ref().map_or((l.transform.size.width.round().max(1.0) as u32, l.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    if (w as u64) * (h as u64) > MAX_PIXELS - doc.used_mask_pixels() { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let v = m.pixels.bytes()[0];
    doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, vec![v; (w * h) as usize]);
    Ok(())
}

pub fn copy_mask(doc: &mut Document, from: Uuid, to: Uuid) -> Result<(), CommandError> {
    if from == to { return Err(CommandError::Argument("same layer".into())); }
    let source = doc.layer(from).ok_or(CommandError::NoLayer)?.clone();
    let mask = source.mask.clone().ok_or(CommandError::Argument("the source has no mask".into()))?;
    let target = doc.layer(to).ok_or(CommandError::NoLayer)?;
    if target.is_group { return Err(CommandError::Argument("folders do not take a copied mask".into())); }
    let placement = mask.placement.unwrap_or(source.transform);
    let value = if mask.is_uniform() { None } else { Some(placement) };
    doc.layer_mut(to).unwrap().set_mask(Some(Mask { pixels: mask.pixels, enabled: mask.enabled, placement: value, linked: mask.linked }));
    Ok(())
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 4 in `masks.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): layer and folder mask operations"
```

---

### Task 8: Merge down, merge layers, merge group

**Files:**
- Create: `engine/src/ops/merge.rs`
- Modify: `engine/src/ops/mod.rs` (add `pub mod merge;`)
- Test: `engine/tests/merge.rs`

**Interfaces:**
- `ops::merge::{MergePlan { ids: Vec<Uuid>, removed: Vec<Uuid>, name: String, parent: Option<Uuid>, anchor: Uuid, action: &'static str }`, `merge_plan(doc, selected: &[Uuid]) -> Option<MergePlan>` (several selected: they and their folders' contents merge, named after the topmost selected; one folder: its contents, folder removed, named after the folder; one layer: with the layer beneath it in the same folder, named after the lower one; `None` when nothing can merge), `merge(doc, selected: &[Uuid]) -> Result<Uuid, CommandError>` (composites the subset as the canvas shows it, trims, inserts one pixel layer at the anchor's slot, re-points clipping links to the result, active = result)}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/merge.rs`:
```rust
use compositor_engine::ops::{hierarchy, merge};
use compositor_engine::*;

fn solid(w: u32, h: u32, rgb: [u8; 3]) -> Raster { Raster::from_premultiplied(w, h, [rgb[0], rgb[1], rgb[2], 255].repeat((w * h) as usize)) }
fn full(d: &Document) -> Raster { composite(d, Rect { x: 0.0, y: 0.0, width: d.width as f64, height: d.height as f64 }, d.width, d.height) }

#[test]
fn merge_down_bakes_blend_and_trims() {
    let mut d = Document::new(10, 10);
    let mut below = Layer::with_pixels("Below", solid(4, 4, [102, 102, 102]), Point { x: 2.0, y: 2.0 }); below.transform.sampling = Sampling::Nearest;
    let mut above = Layer::with_pixels("Above", solid(2, 2, [204, 204, 204]), Point { x: 3.0, y: 3.0 }); above.transform.sampling = Sampling::Nearest;
    above.blend_mode = BlendMode::Multiply;
    let (bid, aid) = (below.id, above.id);
    d.layers = vec![below, above];
    d.active_layer_id = Some(aid);
    let before = full(&d);
    let plan = merge::merge_plan(&d, &[aid]).unwrap();
    assert_eq!((plan.action, plan.name.as_str(), plan.anchor), ("Merge Down", "Below", aid));
    let merged = merge::merge(&d.clone(), &[aid]).map(|_| ()).err();
    assert!(merged.is_none());
    let result = merge::merge(&mut d, &[aid]).unwrap();
    assert_eq!(d.layers.len(), 1);
    assert_eq!(d.layers[0].id, result);
    assert_eq!(d.layers[0].name, "Below");
    assert_eq!(d.active_layer_id, Some(result));
    assert_eq!(d.layers[0].transform.origin, Point { x: 2.0, y: 2.0 });
    assert_eq!(d.layers[0].pixels.as_ref().unwrap().width, 4);
    assert_eq!(full(&d).bytes(), before.bytes(), "the merged layer looks the same");
    assert!(((full(&d).pixel(3, 3)[0] as f64 / 255.0) - 0.32).abs() < 0.02);
    let _ = bid;
}

#[test]
fn merge_group_removes_the_folder_and_merge_layers_keeps_position() {
    let mut d = Document::new(10, 10);
    let bottom = Layer::with_pixels("Bottom", solid(10, 10, [0, 0, 255]), Point { x: 0.0, y: 0.0 });
    let a = Layer::with_pixels("A", solid(2, 2, [255, 0, 0]), Point { x: 0.0, y: 0.0 });
    let b = Layer::with_pixels("B", solid(2, 2, [0, 255, 0]), Point { x: 5.0, y: 5.0 });
    let top = Layer::with_pixels("Top", solid(1, 1, [255, 255, 255]), Point { x: 9.0, y: 9.0 });
    let (aid, bid, tid) = (a.id, b.id, top.id);
    d.layers = vec![bottom, a, b, top];
    let folder = hierarchy::group_layers(&mut d, &[aid, bid]).unwrap();
    let before = full(&d);
    assert_eq!(merge::merge_plan(&d, &[folder]).unwrap().action, "Merge Group");
    let result = merge::merge(&mut d, &[folder]).unwrap();
    assert!(d.layer(folder).is_none() && d.layer(aid).is_none());
    assert_eq!(d.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Bottom", "Folder 1", "Top"]);
    assert_eq!(d.layer(result).unwrap().transform.size, Size { width: 7.0, height: 7.0 }, "trimmed to A..B");
    assert_eq!(full(&d).bytes(), before.bytes());
    // Merge Layers with a multi-selection keeps the topmost selected layer's slot and name.
    let plan = merge::merge_plan(&d, &[result, tid]).unwrap();
    assert_eq!((plan.action, plan.name.as_str()), ("Merge Layers", "Top"));
    let r2 = merge::merge(&mut d, &[result, tid]).unwrap();
    assert_eq!(d.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Bottom", "Top"]);
    assert_eq!(d.layers[1].id, r2);
}

#[test]
fn clipped_layers_repoint_and_nothing_to_merge_is_none() {
    let mut d = Document::new(4, 4);
    let base = Layer::with_pixels("Base", solid(4, 4, [255, 0, 0]), Point { x: 0.0, y: 0.0 });
    let mid = Layer::with_pixels("Mid", solid(4, 4, [0, 255, 0]), Point { x: 0.0, y: 0.0 });
    let mut clipped = Layer::with_pixels("Clipped", solid(4, 4, [0, 0, 255]), Point { x: 0.0, y: 0.0 });
    clipped.mask_source_id = Some(mid.id);
    let (bid, mid_id, cid) = (base.id, mid.id, clipped.id);
    d.layers = vec![base, mid, clipped];
    assert!(merge::merge_plan(&d, &[bid]).is_none(), "nothing beneath the bottom layer");
    let result = merge::merge(&mut d, &[mid_id]).unwrap();
    assert_eq!(d.layer(cid).unwrap().mask_source_id, Some(result), "clipping follows the merged result");
    let g = hierarchy::add_group(&mut d).unwrap();
    assert!(merge::merge_plan(&d, &[g]).is_none(), "an empty folder has nothing to merge");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test merge`
Expected: compile errors.

- [ ] **Step 3: Implement**

`engine/src/ops/merge.rs`:
```rust
use crate::*;
use std::collections::HashSet;
use uuid::Uuid;

pub struct MergePlan { pub ids: Vec<Uuid>, pub removed: Vec<Uuid>, pub name: String, pub parent: Option<Uuid>, pub anchor: Uuid, pub action: &'static str }

pub fn merge_plan(doc: &Document, selected: &[Uuid]) -> Option<MergePlan> {
    let selected: Vec<Uuid> = selected.iter().copied().filter(|id| doc.layer(*id).is_some()).collect();
    if selected.len() > 1 {
        let mut picked: HashSet<Uuid> = selected.iter().copied().collect();
        for id in &selected { picked.extend(doc.descendants(*id)); }
        let ordered: Vec<Uuid> = doc.layers.iter().filter(|l| picked.contains(&l.id)).map(|l| l.id).collect();
        if !ordered.iter().any(|id| !doc.layer(*id).unwrap().is_group) { return None; }
        let top = *ordered.iter().rev().find(|id| selected.contains(id))?;
        let top_layer = doc.layer(top)?;
        return Some(MergePlan { ids: ordered.clone(), removed: ordered, name: top_layer.name.clone(), parent: top_layer.parent_id, anchor: top, action: "Merge Layers" });
    }
    let active = doc.layer(*selected.first()?)?;
    if active.is_group {
        let inside = doc.descendants(active.id);
        if !inside.iter().any(|id| !doc.layer(*id).unwrap().is_group) { return None; }
        let ids: Vec<Uuid> = doc.layers.iter().filter(|l| l.id == active.id || inside.contains(&l.id)).map(|l| l.id).collect();
        return Some(MergePlan { ids: ids.clone(), removed: ids, name: active.name.clone(), parent: active.parent_id, anchor: active.id, action: "Merge Group" });
    }
    let index = doc.index_of(active.id)?;
    let below = doc.layers[..index].iter().rev().find(|l| l.parent_id == active.parent_id)?;
    if below.is_group { return None; }
    Some(MergePlan { ids: vec![below.id, active.id], removed: vec![below.id, active.id], name: below.name.clone(), parent: active.parent_id, anchor: active.id, action: "Merge Down" })
}

/// The layers composited as the canvas shows them into one pixel layer, trimmed, in their place.
pub fn merge(doc: &mut Document, selected: &[Uuid]) -> Result<Uuid, CommandError> {
    let plan = merge_plan(doc, selected).ok_or(CommandError::Argument("nothing to merge".into()))?;
    let kept: HashSet<Uuid> = plan.ids.iter().copied().collect();
    let mut subset = doc.clone();
    subset.layers = doc.layers.iter().filter(|l| kept.contains(&l.id)).cloned().map(|mut l| {
        if let Some(p) = l.parent_id { if !kept.contains(&p) { l.parent_id = None; } }
        if let Some(s) = l.mask_source_id { if !kept.contains(&s) { l.mask_source_id = None; } }
        l
    }).collect();
    let canvas = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
    let flat = composite(&subset, canvas, doc.width, doc.height);
    let (raster, origin) = match compositor::alpha_bounds(&flat) {
        Some((x0, y0, x1, y1)) => (flat.cropped(x0, y0, x1 - x0, y1 - y0), Point { x: x0 as f64, y: y0 as f64 }),
        None => (flat, Point { x: 0.0, y: 0.0 }),
    };
    let mut merged = Layer::with_pixels(&plan.name, raster, origin);
    merged.parent_id = plan.parent;
    let mid = merged.id;
    let removed: HashSet<Uuid> = plan.removed.iter().copied().collect();
    let mut next: Vec<Layer> = doc.layers.iter().filter(|l| !removed.contains(&l.id)).cloned().collect();
    for l in &mut next { if l.mask_source_id.map_or(false, |s| removed.contains(&s)) { l.mask_source_id = Some(mid); } }
    let slot = doc.index_of(plan.anchor).unwrap_or(doc.layers.len());
    let insertion = slot - doc.layers[..slot].iter().filter(|l| removed.contains(&l.id)).count();
    next.insert(insertion.min(next.len()), merged);
    let previous = std::mem::replace(&mut doc.layers, next);
    if let Err(e) = super::hierarchy::validate(doc) { doc.layers = previous; return Err(e); }
    doc.active_layer_id = Some(mid);
    Ok(mid)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine`
Expected: all pass, including 3 in `merge.rs`.

- [ ] **Step 5: Commit**

```
git add engine
git commit -m "feat(engine): merge down, merge layers and merge group"
```

---

### Task 9: Commands, engine facade, wasm and client

**Files:**
- Modify: `engine/src/command.rs`, `engine/src/engine.rs`, `engine-wasm/src/lib.rs`, `app/src/engine/types.ts`, `app/src/engine/client.ts`
- Test: `engine/tests/engine2.rs`, `app/tests/e2e/smoke.spec.ts` (extend)

**Interfaces:**
- New `Command` variants (serde tag `type`, UUID fields uppercase, `ids` as arrays of strings, transforms as `LayerTransform` JSON, corners as `[[x,y],[x,y],[x,y],[x,y]]`):
  `SetLayerOpacity { id, opacity }`, `SetLayersOpacity { ids, opacity }`, `SetLayerBlendMode { id, mode: BlendMode }`, `AddGroup`, `GroupLayers { ids }`, `PlaceLayer { id, parent: Option<Uuid>, above: Option<Uuid>, at_bottom: bool }` (JSON `atBottom`), `MoveLayerBy { id, offset: i32 }`, `DuplicateLayer { id }`, `DuplicateLayerTo { id, parent, above, at_bottom }`, `DeleteLayers { ids, bake: bool }`, `SetLayerTransform { id, transform }`, `TransformLayers { ids, bounds (JSON "box"), draft }`, `FlipLayers { ids, horizontal }`, `NudgeLayers { ids, dx, dy }`, `DistortLayer { id, transform, corners }`, `DistortLayers { ids, bounds, draft, corners }`, `SetMaskPlacement { id, placement }`, `AddMask { id, revealing }`, `DeleteMask { id }`, `SetMaskEnabled { id, enabled }`, `SetMaskLinked { id, linked }`, `InvertMask { id }`, `FillMask { id, white }`, `BlurMask { id, radius }`, `CopyMask { from, to }`, `ToggleClipping { id }`, `ReleaseClipping { id }`, `LinkMask { source, target }`, `MergeLayers { ids }`. `action_name()` covers each (used later for the Edit menu's Undo label; return `&'static str`).
- `Engine`: `render_plan(handle, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError>`, `composite_edit(handle, edit, region, w, h) -> Result<Raster, CommandError>`, `clip_dependents(handle, ids) -> Result<Vec<Uuid>, CommandError>`, `merge_action(handle, ids) -> Result<Option<&'static str>, CommandError>`, `group_box(handle, ids) -> Result<Option<LayerTransform>, CommandError>`, `can_toggle_clipping(handle, id)`, `can_place(handle, id, parent)`.
- `LayerState` additions (camelCase): `hasPixels: bool`, `maskWidth`, `maskHeight`, `maskRevision`, `maskEnabled: bool`, `maskLinked: bool`, `maskPlacement: LayerTransform | null`, `maskSourceId: string | null`, `maskBackground: number`.
- `WasmEngine`: `render_plan(doc, edit_json: Option<String>) -> String`, `composite_edit(doc, edit_json, x, y, w, h, out_w, out_h) -> Uint8Array`, `mask_pixels_ptr(doc, layer) -> *const u8`, `mask_pixels_len(doc, layer) -> usize`, `clip_dependents(doc, ids_json) -> String` (JSON array), `merge_action(doc, ids_json) -> Option<String>`, `group_box(doc, ids_json) -> Option<String>` (JSON transform), `can_toggle_clipping(doc, id) -> bool`, `can_place(doc, id, parent: Option<String>) -> bool`.
- `EngineClient`: `renderPlan(doc, edit: PreviewEdit | null): RenderPlan`, `compositeEdit(doc, edit, region, w, h): Uint8Array`, `maskPixels(doc, layer): Uint8Array | null` (view on wasm memory, valid until the next engine call), `clipDependents(doc, ids): string[]`, `mergeAction(doc, ids): string | null`, `groupBox(doc, ids): LayerTransform | null`, `canToggleClipping(doc, id): boolean`, `canPlace(doc, id, parent): boolean`.
- `types.ts`: `Coverage`, `LayerDraw`, `PlanNode`, `RenderPlan`, `PreviewEdit` mirroring the Rust JSON; `Command` union extended; `PointTuple = [number, number]`; `Corners = [PointTuple, PointTuple, PointTuple, PointTuple]`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/engine2.rs`:
```rust
use compositor_engine::*;

fn red(w: u32, h: u32) -> Raster { Raster::from_premultiplied(w, h, [255u8, 0, 0, 255].repeat((w * h) as usize)) }
fn seed(e: &mut Engine, id: uuid::Uuid, name: &str, x: f64, y: f64) -> uuid::Uuid {
    let bytes = encode_png(&red(4, 4), 72.0).unwrap();
    e.import_image(Some(id), &bytes, name, Some(Point { x: x + 2.0, y: y + 2.0 })).unwrap();
    e.state(id).unwrap().active_layer_id.unwrap()
}

#[test]
fn phase2_commands_round_trip_json_and_undo() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let b = seed(&mut e, doc, "B", 8.0, 8.0);
    let ida = ids::upper_string(&a); let idb = ids::upper_string(&b);
    let run = |e: &mut Engine, json: String| { let c: Command = serde_json::from_str(&json).unwrap(); e.execute(doc, c).unwrap() };
    run(&mut e, format!(r#"{{"type":"SetLayerOpacity","id":"{ida}","opacity":0.5}}"#));
    run(&mut e, format!(r#"{{"type":"SetLayerBlendMode","id":"{ida}","mode":"Multiply"}}"#));
    run(&mut e, format!(r#"{{"type":"GroupLayers","ids":["{ida}","{idb}"]}}"#));
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 3);
    let folder = s.layers.iter().find(|l| l.is_group).unwrap();
    assert_eq!(s.layers.iter().filter(|l| l.parent_id == Some(folder.id)).count(), 2);
    run(&mut e, format!(r#"{{"type":"AddMask","id":"{ida}","revealing":false}}"#));
    let la = e.state(doc).unwrap().layers.iter().find(|l| l.id == a).unwrap().clone();
    assert!(la.has_mask && !la.mask_linked == false && la.mask_enabled && la.mask_width == 1 && la.mask_background == 0);
    run(&mut e, format!(r#"{{"type":"SetLayerTransform","id":"{idb}","transform":{{"origin":[10,10],"size":[4,4],"rotation":45,"flipX":false,"flipY":false,"sampling":"Nearest"}}}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().transform.rotation, 45.0);
    run(&mut e, format!(r#"{{"type":"DistortLayer","id":"{idb}","transform":{{"origin":[10,10],"size":[4,4]}},"corners":[[10,10],[18,10],[16,14],[10,14]]}}"#));
    let lb = e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().clone();
    assert_eq!((lb.transform.rotation, lb.pixels_revision), (0.0, 2));
    run(&mut e, format!(r#"{{"type":"ToggleClipping","id":"{idb}"}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().mask_source_id, Some(a));
    assert_eq!(e.clip_dependents(doc, &[a]).unwrap(), vec![b]);
    run(&mut e, format!(r#"{{"type":"DeleteLayers","ids":["{ida}"],"bake":true}}"#));
    assert_eq!(e.state(doc).unwrap().layers.iter().find(|l| l.id == b).unwrap().pixels_revision, 3);
    let steps = 8;
    for _ in 0..steps { e.undo(doc).unwrap(); }
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 2);
    assert!(!s.can_undo);
    assert_eq!(e.merge_action(doc, &[b]).unwrap(), Some("Merge Down"));
    assert!(e.group_box(doc, &[a, b]).unwrap().is_some());
}

#[test]
fn render_plan_and_composite_edit_through_the_facade() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let draft = LayerTransform::axis_aligned(Point { x: 10.0, y: 10.0 }, Size { width: 4.0, height: 4.0 });
    let plan = e.render_plan(doc, Some(&PreviewEdit::Layer { id: a, draft, corners: None })).unwrap();
    let PlanNode::Layer { draw } = &plan.nodes[0] else { panic!() };
    assert_eq!(draw.transform, draft);
    let out = e.composite_edit(doc, Some(&PreviewEdit::Layer { id: a, draft, corners: None }), Rect { x: 0.0, y: 0.0, width: 20.0, height: 20.0 }, 20, 20).unwrap();
    assert_eq!(out.pixel(11, 11)[3], 255);
    assert_eq!(out.pixel(1, 1)[3], 0);
    let json = serde_json::to_string(&plan).unwrap();
    assert!(json.contains("\"coverages\":[]"));
}
```

Extend `app/tests/e2e/smoke.spec.ts` with:
```ts
test("phase 2 client calls reach the engine", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const result = await page.evaluate(() => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const doc = api.engine.newDocument(10, 10, true);
    const state = api.engine.state(doc);
    const id = state.layers[0].id;
    api.engine.execute(doc, { type: "AddMask", id, revealing: true });
    const after = api.engine.state(doc);
    const plan = api.engine.renderPlan(doc, null);
    const pixels = api.engine.maskPixels(doc, id);
    return { hasMask: after.layers[0].hasMask, maskWidth: after.layers[0].maskWidth, nodes: plan.nodes.length, maskBytes: pixels ? pixels.length : -1, merge: api.engine.mergeAction(doc, [id]) };
  });
  expect(result.hasMask).toBe(true);
  expect(result.maskWidth).toBe(1);
  expect(result.nodes).toBe(0); // a blank layer has no pixels to draw
  expect(result.maskBytes).toBe(1);
  expect(result.merge).toBeNull();
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p compositor-engine --test engine2`
Expected: compile errors (unknown variants).

- [ ] **Step 3: Extend the command enum**

In `engine/src/command.rs` add the variants (keep the Phase 1 ones):
```rust
    SetLayerOpacity { #[serde(with = "ids::upper")] id: Uuid, opacity: f64 },
    SetLayersOpacity { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, opacity: f64 },
    SetLayerBlendMode { #[serde(with = "ids::upper")] id: Uuid, mode: BlendMode },
    AddGroup,
    GroupLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid> },
    PlaceLayer { #[serde(with = "ids::upper")] id: Uuid, #[serde(default, with = "ids::upper_opt")] parent: Option<Uuid>, #[serde(default, with = "ids::upper_opt")] above: Option<Uuid>, #[serde(default, rename = "atBottom")] at_bottom: bool },
    MoveLayerBy { #[serde(with = "ids::upper")] id: Uuid, offset: i32 },
    DuplicateLayer { #[serde(with = "ids::upper")] id: Uuid },
    DuplicateLayerTo { #[serde(with = "ids::upper")] id: Uuid, #[serde(default, with = "ids::upper_opt")] parent: Option<Uuid>, #[serde(default, with = "ids::upper_opt")] above: Option<Uuid>, #[serde(default, rename = "atBottom")] at_bottom: bool },
    DeleteLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(default)] bake: bool },
    SetLayerTransform { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform },
    TransformLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform },
    FlipLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, horizontal: bool },
    NudgeLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, dx: f64, dy: f64 },
    DistortLayer { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform, corners: [Point; 4] },
    DistortLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid>, #[serde(rename = "box")] bounds: LayerTransform, draft: LayerTransform, corners: [Point; 4] },
    SetMaskPlacement { #[serde(with = "ids::upper")] id: Uuid, placement: LayerTransform },
    AddMask { #[serde(with = "ids::upper")] id: Uuid, revealing: bool },
    DeleteMask { #[serde(with = "ids::upper")] id: Uuid },
    SetMaskEnabled { #[serde(with = "ids::upper")] id: Uuid, enabled: bool },
    SetMaskLinked { #[serde(with = "ids::upper")] id: Uuid, linked: bool },
    InvertMask { #[serde(with = "ids::upper")] id: Uuid },
    FillMask { #[serde(with = "ids::upper")] id: Uuid, white: bool },
    BlurMask { #[serde(with = "ids::upper")] id: Uuid, radius: f64 },
    CopyMask { #[serde(with = "ids::upper")] from: Uuid, #[serde(with = "ids::upper")] to: Uuid },
    ToggleClipping { #[serde(with = "ids::upper")] id: Uuid },
    ReleaseClipping { #[serde(with = "ids::upper")] id: Uuid },
    LinkMask { #[serde(with = "ids::upper")] source: Uuid, #[serde(with = "ids::upper")] target: Uuid },
    MergeLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid> },
```
Import `BlendMode`, `LayerTransform`, `Point` at the top. `serialize_ids`/`deserialize_ids` already exist in this file (make them usable for these fields: they take `&[Uuid]`/return `Vec<Uuid>`; adjust the serializer signature to `&Vec<Uuid>` if the compiler asks). Extend `action_name` with one arm per variant ("Layer Opacity", "Layer Blend Mode", "New Folder", "Group Layers", "Move Layer", "Reorder Layers", "Duplicate Layer", "Duplicate Layer", "Delete Layers", "Transform Layer", "Transform Layers", "Flip Layers", "Nudge", "Distort", "Distort Layers", "Transform Layer Mask", "Add Mask", "Delete Layer Mask", "Enable Layer Mask", "Link Layer Mask", "Invert Mask", "Fill Mask", "Blur Mask", "Copy Layer Mask", "Clipping Mask", "Release Clipping Mask", "Create Clipping Mask", "Merge Layers").

- [ ] **Step 4: Wire the engine facade**

In `engine/src/engine.rs` `execute`, add arms (all inside the existing `self.edit(handle, |doc| match command { ... })`):
```rust
            Command::SetLayerOpacity { id, opacity } => { ops::appearance::set_opacity(doc, id, opacity)?; Ok(Dirty::structure()) }
            Command::SetLayersOpacity { ids, opacity } => { ops::appearance::set_opacity_many(doc, &ids, opacity)?; Ok(Dirty::structure()) }
            Command::SetLayerBlendMode { id, mode } => { ops::appearance::set_blend_mode(doc, id, mode)?; Ok(Dirty::structure()) }
            Command::AddGroup => { ops::hierarchy::add_group(doc)?; Ok(Dirty::structure()) }
            Command::GroupLayers { ids } => { ops::hierarchy::group_layers(doc, &ids)?; Ok(Dirty::structure()) }
            Command::PlaceLayer { id, parent, above, at_bottom } => { ops::hierarchy::place_layer(doc, id, parent, above, at_bottom)?; Ok(Dirty::structure()) }
            Command::MoveLayerBy { id, offset } => { ops::hierarchy::move_layer_by(doc, id, offset)?; Ok(Dirty::structure()) }
            Command::DuplicateLayer { id } => { ops::hierarchy::duplicate_layer(doc, id)?; Ok(Dirty::structure()) }
            Command::DuplicateLayerTo { id, parent, above, at_bottom } => { ops::hierarchy::duplicate_layer_to(doc, id, parent, above, at_bottom)?; Ok(Dirty::structure()) }
            Command::DeleteLayers { ids, bake } => { ops::hierarchy::delete_layers(doc, &ids, bake)?; Ok(Dirty::structure()) }
            Command::SetLayerTransform { id, transform } => { ops::transform::set_transform(doc, id, transform)?; Ok(Dirty::structure()) }
            Command::TransformLayers { ids, bounds, draft } => { ops::transform::transform_group(doc, &ids, &bounds, &draft)?; Ok(Dirty::structure()) }
            Command::FlipLayers { ids, horizontal } => { ops::transform::flip_layers(doc, &ids, horizontal)?; Ok(Dirty::structure()) }
            Command::NudgeLayers { ids, dx, dy } => { ops::transform::nudge(doc, &ids, dx, dy)?; Ok(Dirty::structure()) }
            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
            Command::DistortLayers { ids, bounds, draft, corners } => { ops::distort::distort_group(doc, &ids, &bounds, &draft, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: ids }) }
            Command::SetMaskPlacement { id, placement } => { ops::transform::set_mask_placement(doc, id, placement)?; Ok(Dirty::structure()) }
            Command::AddMask { id, revealing } => { ops::masks::add_mask(doc, id, revealing)?; Ok(Dirty::structure()) }
            Command::DeleteMask { id } => { ops::masks::delete_mask(doc, id)?; Ok(Dirty::structure()) }
            Command::SetMaskEnabled { id, enabled } => { ops::masks::set_mask_enabled(doc, id, enabled)?; Ok(Dirty::structure()) }
            Command::SetMaskLinked { id, linked } => { ops::masks::set_mask_linked(doc, id, linked)?; Ok(Dirty::structure()) }
            Command::InvertMask { id } => { ops::masks::invert_mask(doc, id)?; Ok(Dirty::structure()) }
            Command::FillMask { id, white } => { ops::masks::fill_mask(doc, id, white)?; Ok(Dirty::structure()) }
            Command::BlurMask { id, radius } => { ops::masks::blur_mask(doc, id, radius)?; Ok(Dirty::structure()) }
            Command::CopyMask { from, to } => { ops::masks::copy_mask(doc, from, to)?; Ok(Dirty::structure()) }
            Command::ToggleClipping { id } => { ops::hierarchy::toggle_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::ReleaseClipping { id } => { ops::hierarchy::release_clipping(doc, id)?; Ok(Dirty::structure()) }
            Command::LinkMask { source, target } => { ops::hierarchy::link_mask(doc, source, target)?; Ok(Dirty::structure()) }
            Command::MergeLayers { ids } => { let m = ops::merge::merge(doc, &ids)?; Ok(Dirty { structure: true, canvas: false, layers: vec![m] }) }
```
Add the facade methods:
```rust
    pub fn render_plan(&self, id: Uuid, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError> { Ok(plan::render_plan(&self.session(id)?.document, edit)) }
    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit(&self.session(id)?.document, edit, region, w, h)) }
    pub fn clip_dependents(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, CommandError> { Ok(ops::hierarchy::clip_dependents(&self.session(id)?.document, ids)) }
    pub fn merge_action(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<&'static str>, CommandError> { Ok(ops::merge::merge_plan(&self.session(id)?.document, ids).map(|p| p.action)) }
    pub fn group_box(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<LayerTransform>, CommandError> { Ok(ops::transform::group_box(&self.session(id)?.document, ids)) }
    pub fn can_toggle_clipping(&self, id: Uuid, layer: Uuid) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_toggle_clipping(&self.session(id)?.document, layer)) }
    pub fn can_place(&self, id: Uuid, layer: Uuid, parent: Option<Uuid>) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_place(&self.session(id)?.document, layer, parent)) }
```
Extend `LayerState` with `has_pixels: bool`, `mask_width: u32`, `mask_height: u32`, `mask_revision: u64`, `mask_enabled: bool`, `mask_linked: bool`, `#[serde(with = "ids::upper_opt")] mask_source_id: Option<Uuid>`, `mask_placement: Option<LayerTransform>`, `mask_background: u8` (all camelCase via the struct attribute) and fill them in `state()` (`mask_enabled`/`mask_linked` default true/true without a mask; `mask_background` 255 without a mask).

- [ ] **Step 5: Expose through wasm and the client**

`engine-wasm/src/lib.rs` additions:
```rust
    fn parse_edit(json: Option<String>) -> Result<Option<PreviewEdit>, JsError> {
        match json { Some(j) => Ok(Some(serde_json::from_str(&j).map_err(js_err)?)), None => Ok(None) }
    }
    fn parse_ids(json: &str) -> Result<Vec<Uuid>, JsError> {
        let v: Vec<String> = serde_json::from_str(json).map_err(js_err)?;
        v.iter().map(|s| Uuid::parse_str(s).map_err(js_err)).collect()
    }
    pub fn render_plan(&self, doc: &str, edit_json: Option<String>) -> Result<String, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        serde_json::to_string(&self.engine.render_plan(parse_id(doc)?, edit.as_ref()).map_err(js_err)?).map_err(js_err)
    }
    pub fn composite_edit(&self, doc: &str, edit_json: Option<String>, x: f64, y: f64, w: f64, h: f64, out_w: u32, out_h: u32) -> Result<Uint8Array, JsError> {
        let edit = Self::parse_edit(edit_json)?;
        let raster = self.engine.composite_edit(parse_id(doc)?, edit.as_ref(), Rect { x, y, width: w, height: h }, out_w, out_h).map_err(js_err)?;
        Ok(Uint8Array::from(raster.bytes()))
    }
    pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.mask.as_ref().map_or(std::ptr::null(), |m| m.pixels.bytes().as_ptr()))
    }
    pub fn mask_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
        Ok(l.mask.as_ref().map_or(0, |m| m.pixels.bytes().len()))
    }
    pub fn clip_dependents(&self, doc: &str, ids_json: &str) -> Result<String, JsError> {
        let ids = self.engine.clip_dependents(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)?;
        serde_json::to_string(&ids.iter().map(ids::upper_string).collect::<Vec<_>>()).map_err(js_err)
    }
    pub fn merge_action(&self, doc: &str, ids_json: &str) -> Result<Option<String>, JsError> {
        Ok(self.engine.merge_action(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)?.map(|s| s.to_string()))
    }
    pub fn group_box(&self, doc: &str, ids_json: &str) -> Result<Option<String>, JsError> {
        match self.engine.group_box(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)? { Some(t) => Ok(Some(serde_json::to_string(&t).map_err(js_err)?)), None => Ok(None) }
    }
    pub fn can_toggle_clipping(&self, doc: &str, id: &str) -> Result<bool, JsError> { self.engine.can_toggle_clipping(parse_id(doc)?, parse_id(id)?).map_err(js_err) }
    pub fn can_place(&self, doc: &str, id: &str, parent: Option<String>) -> Result<bool, JsError> {
        let p = match parent { Some(p) => Some(parse_id(&p)?), None => None };
        self.engine.can_place(parse_id(doc)?, parse_id(id)?, p).map_err(js_err)
    }
```

`app/src/engine/types.ts` additions:
```ts
export type PointTuple = [number, number];
export type Corners = [PointTuple, PointTuple, PointTuple, PointTuple];

export interface LayerState { /* existing fields, plus: */
  hasPixels: boolean; maskWidth: number; maskHeight: number; maskRevision: number; maskEnabled: boolean; maskLinked: boolean;
  maskSourceId: string | null; maskPlacement: LayerTransform | null; maskBackground: number;
}

export interface Coverage { layerId: string; maskRevision: number; placement: LayerTransform; corners: Corners | null; width: number; height: number; background: number; nearest: boolean; }
export interface LayerDraw { id: string; transform: LayerTransform; corners: Corners | null; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; opacity: number; blend: BlendMode; coverages: Coverage[]; clip: string | null; }
export type PlanNode = { kind: "layer"; draw: LayerDraw } | { kind: "stack"; base: LayerDraw; children: LayerDraw[]; folderCoverages: Coverage[] };
export interface RenderPlan { nodes: PlanNode[]; sources: LayerDraw[]; }
export type PreviewEdit =
  | { kind: "layer"; id: string; draft: LayerTransform; corners?: Corners | null }
  | { kind: "group"; ids: string[]; box: LayerTransform; draft: LayerTransform; corners?: Corners | null }
  | { kind: "mask"; id: string; draft: LayerTransform };
```
and the `Command` union gains one member per new variant with the JSON shapes above (`{ type: "PlaceLayer"; id: string; parent: string | null; above: string | null; atBottom: boolean }`, `{ type: "TransformLayers"; ids: string[]; box: LayerTransform; draft: LayerTransform }`, `{ type: "DistortLayer"; id: string; transform: LayerTransform; corners: Corners }`, etc.).

`app/src/engine/client.ts` additions:
```ts
  renderPlan(doc: string, edit: PreviewEdit | null): RenderPlan { return JSON.parse(this.wasm.render_plan(doc, edit ? JSON.stringify(edit) : undefined)) as RenderPlan; }
  compositeEdit(doc: string, edit: PreviewEdit | null, region: { x: number; y: number; width: number; height: number }, outWidth: number, outHeight: number): Uint8Array {
    return this.wasm.composite_edit(doc, edit ? JSON.stringify(edit) : undefined, region.x, region.y, region.width, region.height, outWidth, outHeight);
  }
  /** A view on wasm memory; valid only until the next engine call. */
  maskPixels(doc: string, layer: string): Uint8Array | null {
    const len = this.wasm.mask_pixels_len(doc, layer);
    if (len === 0) return null;
    return new Uint8Array(this.memory.buffer, this.wasm.mask_pixels_ptr(doc, layer), len);
  }
  clipDependents(doc: string, ids: string[]): string[] { return JSON.parse(this.wasm.clip_dependents(doc, JSON.stringify(ids))) as string[]; }
  mergeAction(doc: string, ids: string[]): string | null { return this.wasm.merge_action(doc, JSON.stringify(ids)) ?? null; }
  groupBox(doc: string, ids: string[]): LayerTransform | null { const t = this.wasm.group_box(doc, JSON.stringify(ids)); return t ? (JSON.parse(t) as LayerTransform) : null; }
  canToggleClipping(doc: string, id: string): boolean { return this.wasm.can_toggle_clipping(doc, id); }
  canPlace(doc: string, id: string, parent: string | null): boolean { return this.wasm.can_place(doc, id, parent ?? undefined); }
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
Expected: engine tests all pass (engine2 adds 2); the wasm crate checks; the smoke spec's new test passes along with the existing 12 e2e tests.

- [ ] **Step 7: Commit**

```
git add engine engine-wasm app
git commit -m "feat(engine,app): phase 2 commands, render plan and mask access through wasm and the client"
```

---

### Task 10: Transform geometry in TypeScript, selection helpers and store state

**Files:**
- Create: `app/src/tools/transform-geometry.ts`, `app/src/state/selection.ts`
- Modify: `app/src/state/store.ts`, `engine/src/command.rs`, `engine/src/engine.rs`, `app/src/engine/types.ts` (one extra command)
- Test: `app/tests/unit/transform-geometry.test.ts`, `app/tests/unit/selection.test.ts`, `engine/tests/engine2.rs` (one more test)

**Interfaces:**
- Engine: `Command::DuplicateLayerTransformed { id, transform }` = duplicate then `set_transform` on the copy, one undo step (Alt-drag on the canvas); JSON `{ type: "DuplicateLayerTransformed", id, transform }`; `action_name` "Duplicate Layer".
- `transform-geometry.ts` (all on the `LayerTransform` JSON shape, `PointLike` points): `center(t)`, `radians(t)`, `pointOf(t, unit)`, `cornersOf(t): [P,P,P,P]` (TL, TR, BR, BL), `boundsOf(t): Rect`, `containsPoint(t, p)`, `isValidTransform(t)`, `roundedTransform(t)`, `scalePercent(t, pixel)`, `scaledToPercent(t, percent, pixel)`, `mirrored(t, horizontal, axis)`; `type Mat3 = number[]` (9 numbers, row-major), `mat3Mul(a, b)`, `mat3Invert(m): Mat3 | null`, `mat3Apply(m, p)`, `pixelToDocument(t, w, h): Mat3`, `homographyUnitTo(corners): Mat3`, `isUsableCorners(corners)`, `carriedCorners(placement, by, to)`; `type TransformDragMode = { kind: "move" } | { kind: "resize"; index: number } | { kind: "rotate" } | { kind: "distort"; index: number }`; `transformDrag(original, start, mode).updated(point, opts: { lockRatio: boolean; shift: boolean; alt: boolean }): LayerTransform` (port of `TransformDrag.updated`; distort mode returns the original); `cornersDrag(originalCorners, start, mode: "move" | number).updated(point, shift): Corners`; `overlayGeometry(transformOrCorners, viewport, docSize): { handles: P[]; rotationHandle: P; showsRotation: boolean }` (rotation handle 28 px beyond the top-middle handle along the transform's up direction), `hitOverlay(geometry, point): TransformDragMode | null` (rotation within 10 px, handles within 10 px, then edges within 10 px, else null); `snapTargets(state, movingIds): { xs: number[]; ys: number[] }` (canvas edges and centre plus every other visible pixel layer's rounded upright bounds and centre), `snapOffset(box: Rect, xs, ys, tolerance): { dx: number; dy: number; x: number | null; y: number | null }` (smallest move putting the box's left/centre/right on a target, per axis).
- `selection.ts`: `transformsAsGroup(state, selected: string[]): boolean`, `groupMembers(state, selected): LayerState[]` (visible pixel layers selected or inside selected folders), `groupBox(state, selected): LayerTransform | null`, `canTransform(state, selected, maskSelected): boolean`, `activeLayer(state): LayerState | null`.
- Store additions: `selectedLayerIds: string[]`, `maskSelected: boolean`, `collapsed: Record<string, string[]>` (per document handle), `transformEdit: TransformEdit | null` where `TransformEdit = { kind: "layer" | "group" | "mask"; id: string; ids: string[]; box: LayerTransform; original: LayerTransform; draft: LayerTransform; corners: Corners | null; persistent: boolean; duplicated: boolean }`, `snapGuides: { xs: number[]; ys: number[] }`, `blendPreview: BlendMode | null`; actions `selectLayers(ids, primary)`, `setMaskSelected(v)`, `toggleCollapsed(id)`, `beginTransform(opts: { persistent: boolean; duplicate?: boolean }): boolean`, `previewTransform(draft, corners?)`, `beginDistort()`, `commitTransform()`, `cancelTransform()`, `setSnapGuides(g)`, `setBlendPreview(m)`, `previewEdit(): PreviewEdit | null`. `selectLayers` also runs `SetActiveLayer` when the primary differs from the engine's active layer. `refresh` reconciles: selected ids that no longer exist are dropped; if the engine's active id is not in the selection, the selection becomes `[active]`.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/transform-geometry.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { cornersOf, containsPoint, hitOverlay, homographyUnitTo, isUsableCorners, mat3Apply, mat3Invert, overlayGeometry, pixelToDocument, snapOffset, transformDrag } from "../../src/tools/transform-geometry";
import { HANDLES } from "../../src/tools/crop-geometry";
import { Viewport } from "../../src/canvas/viewport";
import type { LayerTransform } from "../../src/engine/types";

const t = (x: number, y: number, w: number, h: number, rotation = 0): LayerTransform => ({ origin: [x, y], size: [w, h], rotation, flipX: false, flipY: false, sampling: "High quality" });
const near = (a: number, b: number) => Math.abs(a - b) < 1e-4;

describe("transform geometry", () => {
  it("rotated resize keeps the opposite anchor at every handle", () => {
    const original = t(31, -19, 200, 100, 37);
    HANDLES.forEach((handle, index) => {
      const opposite = { x: 1 - handle.x, y: 1 - handle.y };
      const start = pointOfT(original, handle);
      const drag = transformDrag(original, start, { kind: "resize", index });
      for (const lockRatio of [true, false]) {
        const changed = drag.updated({ x: start.x + 34, y: start.y + 17 }, { lockRatio, shift: false, alt: false });
        const a = pointOfT(original, opposite), b = pointOfT(changed, opposite);
        expect(near(a.x, b.x) && near(a.y, b.y)).toBe(true);
        if (lockRatio) expect(near(changed.size[0] / changed.size[1], 2)).toBe(true);
      }
    });
  });
  it("move, rotate and shift constraints", () => {
    const original = t(0, 0, 100, 50);
    const moved = transformDrag(original, { x: 40, y: 20 }, { kind: "move" }).updated({ x: 60, y: 25 }, { lockRatio: true, shift: true, alt: false });
    expect(moved.origin).toEqual([20, 0]);
    const rotated = transformDrag(original, { x: 100, y: 25 }, { kind: "rotate" }).updated({ x: 50, y: 75 }, { lockRatio: true, shift: false, alt: false });
    expect(near(rotated.rotation, 90)).toBe(true);
    const snapped = transformDrag(original, { x: 100, y: 25 }, { kind: "rotate" }).updated({ x: 99, y: 45 }, { lockRatio: true, shift: true, alt: false });
    expect(snapped.rotation % 15).toBe(0);
    const free = transformDrag(original, { x: 100, y: 50 }, { kind: "resize", index: 4 }).updated({ x: 150, y: 50 }, { lockRatio: true, shift: true, alt: false });
    expect(free.size).toEqual([150, 50]);
    const centred = transformDrag(original, { x: 100, y: 25 }, { kind: "resize", index: 3 }).updated({ x: 120, y: 25 }, { lockRatio: false, shift: false, alt: true });
    expect(near(centred.size[0], 140) && near(centred.origin[0], -20)).toBe(true);
  });
  it("overlay hit regions match rotated edges, corners and the rotation handle", () => {
    const vp = new Viewport(); const size = { width: 1000, height: 800 };
    vp.resize({ width: 1000, height: 800 }, 1, size);
    const g = overlayGeometry(t(100, 100, 400, 300, 37), vp, size);
    for (const [s, e, expected] of [[0, 2, 1], [2, 4, 3], [4, 6, 5], [6, 0, 7]] as const) {
      const p = { x: g.handles[s].x * 0.75 + g.handles[e].x * 0.25, y: g.handles[s].y * 0.75 + g.handles[e].y * 0.25 };
      expect(hitOverlay(g, p)).toEqual({ kind: "resize", index: expected });
    }
    for (const i of [0, 2, 4, 6]) expect(hitOverlay(g, g.handles[i])).toEqual({ kind: "resize", index: i });
    expect(hitOverlay(g, g.rotationHandle)).toEqual({ kind: "rotate" });
    expect(hitOverlay(g, vp.viewPoint({ x: 300, y: 250 }, size))).toBeNull();
    expect(containsPoint(t(100, 200, 100, 50, 90), { x: 150, y: 265 })).toBe(true);
    expect(containsPoint(t(100, 200, 100, 50, 90), { x: 190, y: 225 })).toBe(false);
  });
  it("homography and pixel mapping", () => {
    const shape = [{ x: 10, y: 10 }, { x: 60, y: 10 }, { x: 30, y: 30 }, { x: 10, y: 30 }] as const;
    const h = homographyUnitTo([...shape]);
    expect(mat3Apply(h, { x: 1, y: 1 })).toEqual(expect.objectContaining({ x: expect.closeTo(30, 6), y: expect.closeTo(30, 6) }));
    expect(isUsableCorners([shape[0], shape[2], shape[1], shape[3]])).toBe(false);
    const m = pixelToDocument(t(10, 20, 100, 50), 64, 32);
    const c = mat3Apply(m, { x: 32, y: 16 });
    expect(near(c.x, 60) && near(c.y, 45)).toBe(true);
    const inv = mat3Invert(m)!;
    const back = mat3Apply(inv, c);
    expect(near(back.x, 32) && near(back.y, 16)).toBe(true);
    expect(cornersOf(t(0, 0, 10, 10))[2]).toEqual({ x: 10, y: 10 });
  });
  it("snap offset picks the smallest move per axis", () => {
    const r = snapOffset({ x: 3, y: 96, width: 10, height: 10 }, [0, 20], [100], 5);
    expect(r).toEqual({ dx: -3, dy: 4, x: 0, y: 100 });
    expect(snapOffset({ x: 50, y: 50, width: 10, height: 10 }, [0], [0], 5)).toEqual({ dx: 0, dy: 0, x: null, y: null });
  });
});

function pointOfT(tr: LayerTransform, unit: { x: number; y: number }) {
  const [ox, oy] = tr.origin; const [w, h] = tr.size; const cx = ox + w / 2, cy = oy + h / 2;
  const r = (tr.rotation % 360) * Math.PI / 180; const x = (unit.x - 0.5) * w, y = (unit.y - 0.5) * h;
  return { x: cx + x * Math.cos(r) - y * Math.sin(r), y: cy + x * Math.sin(r) + y * Math.cos(r) };
}
```

`app/tests/unit/selection.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { canTransform, groupBox, groupMembers, transformsAsGroup } from "../../src/state/selection";
import type { DocumentState, LayerState } from "../../src/engine/types";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, path: null, layers });

describe("selection helpers", () => {
  const f = layer("F", { isGroup: true, hasPixels: false, pixelsWidth: 0 });
  const a = layer("A", { parentId: "F" });
  const b = layer("B", { parentId: "F", visible: false });
  const c = layer("C", { transform: { origin: [20, 20], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } });
  const blank = layer("K", { hasPixels: false, pixelsWidth: 0 });
  const d = doc([f, a, b, c, blank], "C");
  it("decides when a transform is a group transform", () => {
    expect(transformsAsGroup(d, ["C"])).toBe(false);
    expect(transformsAsGroup(d, ["F"])).toBe(true);
    expect(transformsAsGroup(d, ["A", "C"])).toBe(true);
  });
  it("collects visible pixel members and their box", () => {
    expect(groupMembers(d, ["F", "C"]).map((l) => l.id)).toEqual(["A", "C"]);
    expect(groupBox(d, ["F", "C"])).toEqual(expect.objectContaining({ origin: [0, 0], size: [30, 30] }));
    expect(groupBox(d, ["K"])).toBeNull();
  });
  it("blank layers and hidden layers cannot be transformed alone", () => {
    expect(canTransform(doc([blank], "K"), ["K"], false)).toBe(false);
    expect(canTransform(d, ["C"], false)).toBe(true);
    expect(canTransform(doc([f, b], "B"), ["B"], false)).toBe(false);
  });
});
```

Add to `engine/tests/engine2.rs`:
```rust
#[test]
fn duplicate_transformed_is_one_undo_step() {
    let mut e = Engine::new();
    let doc = e.new_document(20, 20, false).unwrap();
    let a = seed(&mut e, doc, "A", 0.0, 0.0);
    let moved = LayerTransform::axis_aligned(Point { x: 5.0, y: 5.0 }, Size { width: 4.0, height: 4.0 });
    e.execute(doc, Command::DuplicateLayerTransformed { id: a, transform: moved }).unwrap();
    let s = e.state(doc).unwrap();
    assert_eq!(s.layers.len(), 2);
    assert_eq!(s.layers[1].transform.origin, Point { x: 5.0, y: 5.0 });
    assert_eq!(s.layers[0].transform.origin, Point { x: 0.0, y: 0.0 });
    e.undo(doc).unwrap();
    assert_eq!(e.state(doc).unwrap().layers.len(), 1);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `cargo test -p compositor-engine --test engine2`
Expected: module not found; unknown variant.

- [ ] **Step 3: Engine addition**

`command.rs`: `DuplicateLayerTransformed { #[serde(with = "ids::upper")] id: Uuid, transform: LayerTransform }` (action "Duplicate Layer"); `engine.rs` arm: `Command::DuplicateLayerTransformed { id, transform } => { let copy = ops::hierarchy::duplicate_layer(doc, id)?; ops::transform::set_transform(doc, copy, transform)?; Ok(Dirty::structure()) }`; `types.ts`: `{ type: "DuplicateLayerTransformed"; id: string; transform: LayerTransform }`.

- [ ] **Step 4: Implement transform-geometry.ts**

`app/src/tools/transform-geometry.ts`:
```ts
import type { Corners, DocumentState, LayerTransform, PointTuple } from "../engine/types";
import type { PointLike, RectLike, SizeLike, Viewport } from "../canvas/viewport";
import { HANDLES } from "./crop-geometry";

export type P = PointLike;
export type Mat3 = number[]; // row-major 3x3

export function center(t: LayerTransform): P { return { x: t.origin[0] + t.size[0] / 2, y: t.origin[1] + t.size[1] / 2 }; }
export function radians(t: LayerTransform): number { return (t.rotation % 360) * Math.PI / 180; }
export function pointOf(t: LayerTransform, unit: P): P {
  const x = (unit.x - 0.5) * t.size[0], y = (unit.y - 0.5) * t.size[1]; const r = radians(t); const c = center(t);
  return { x: c.x + x * Math.cos(r) - y * Math.sin(r), y: c.y + x * Math.sin(r) + y * Math.cos(r) };
}
export function cornersOf(t: LayerTransform): [P, P, P, P] { return [pointOf(t, { x: 0, y: 0 }), pointOf(t, { x: 1, y: 0 }), pointOf(t, { x: 1, y: 1 }), pointOf(t, { x: 0, y: 1 })]; }
export function boundsOf(t: LayerTransform): RectLike { return boundsOfPoints(cornersOf(t)); }
export function boundsOfPoints(pts: P[]): RectLike {
  const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y);
  const x = Math.min(...xs), y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}
export function containsPoint(t: LayerTransform, p: P): boolean {
  const c = center(t); const r = radians(t); const x = p.x - c.x, y = p.y - c.y;
  return Math.abs(x * Math.cos(r) + y * Math.sin(r)) <= t.size[0] / 2 && Math.abs(-x * Math.sin(r) + y * Math.cos(r)) <= t.size[1] / 2;
}
export function isValidTransform(t: LayerTransform): boolean {
  const vals = [t.origin[0], t.origin[1], t.size[0], t.size[1], t.rotation];
  return vals.every(Number.isFinite) && t.size[0] >= 1 && t.size[0] <= 300_000 && t.size[1] >= 1 && t.size[1] <= 300_000 && Math.abs(t.origin[0]) <= 1_000_000 && Math.abs(t.origin[1]) <= 1_000_000;
}
export function roundedTransform(t: LayerTransform): LayerTransform {
  return { ...t, origin: [Math.round(t.origin[0]), Math.round(t.origin[1])], size: [Math.max(1, Math.round(t.size[0])), Math.max(1, Math.round(t.size[1]))], rotation: Math.round(t.rotation) };
}
export function scalePercent(t: LayerTransform, pixel: SizeLike): number { return t.size[0] / Math.max(1, pixel.width) * 100; }
export function scaledToPercent(t: LayerTransform, percent: number, pixel: SizeLike): LayerTransform {
  const c = center(t); const w = pixel.width * percent / 100, h = pixel.height * percent / 100;
  return { ...t, size: [w, h], origin: [c.x - w / 2, c.y - h / 2] };
}
export function mirrored(t: LayerTransform, horizontal: boolean, axis: number): LayerTransform {
  const c = center(t);
  return horizontal ? { ...t, flipX: !t.flipX, origin: [2 * axis - c.x - t.size[0] / 2, t.origin[1]], rotation: -t.rotation }
    : { ...t, flipY: !t.flipY, origin: [t.origin[0], 2 * axis - c.y - t.size[1] / 2], rotation: -t.rotation };
}

export function mat3Mul(a: Mat3, b: Mat3): Mat3 {
  const o = new Array(9).fill(0);
  for (let r = 0; r < 3; r++) for (let c = 0; c < 3; c++) o[r * 3 + c] = a[r * 3] * b[c] + a[r * 3 + 1] * b[3 + c] + a[r * 3 + 2] * b[6 + c];
  return o;
}
export function mat3Apply(m: Mat3, p: P): P {
  const w = m[6] * p.x + m[7] * p.y + m[8]; const d = Math.abs(w) < 1e-12 ? 1e-12 : w;
  return { x: (m[0] * p.x + m[1] * p.y + m[2]) / d, y: (m[3] * p.x + m[4] * p.y + m[5]) / d };
}
export function mat3Invert(m: Mat3): Mat3 | null {
  const [a, b, c, d, e, f, g, h, i] = m;
  const det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
  if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return null;
  return [(e * i - f * h) / det, (c * h - b * i) / det, (b * f - c * e) / det, (f * g - d * i) / det, (a * i - c * g) / det, (c * d - a * f) / det, (d * h - e * g) / det, (b * g - a * h) / det, (a * e - b * d) / det];
}
const translation = (x: number, y: number): Mat3 => [1, 0, x, 0, 1, y, 0, 0, 1];
const scaling = (x: number, y: number): Mat3 => [x, 0, 0, 0, y, 0, 0, 0, 1];
const rotation = (r: number): Mat3 => [Math.cos(r), -Math.sin(r), 0, Math.sin(r), Math.cos(r), 0, 0, 0, 1];
/** Layer pixel (x, y) in a w x h grid to document coordinates (flips applied). */
export function pixelToDocument(t: LayerTransform, w: number, h: number): Mat3 {
  const c = center(t);
  return mat3Mul(mat3Mul(mat3Mul(translation(c.x, c.y), rotation(radians(t))), scaling(t.size[0] / Math.max(1, w) * (t.flipX ? -1 : 1), t.size[1] / Math.max(1, h) * (t.flipY ? -1 : 1))), translation(-w / 2, -h / 2));
}
/** Unit square (TL, TR, BR, BL) onto `c`. */
export function homographyUnitTo(c: P[]): Mat3 {
  const sx = c[0].x - c[1].x + c[2].x - c[3].x, sy = c[0].y - c[1].y + c[2].y - c[3].y;
  let g = 0, h = 0;
  if (Math.abs(sx) > 1e-9 || Math.abs(sy) > 1e-9) {
    const dx1 = c[1].x - c[2].x, dx2 = c[3].x - c[2].x, dy1 = c[1].y - c[2].y, dy2 = c[3].y - c[2].y;
    const den = dx1 * dy2 - dx2 * dy1;
    if (Math.abs(den) > 1e-12) { g = (sx * dy2 - dx2 * sy) / den; h = (dx1 * sy - sx * dy1) / den; }
  }
  const a = c[1].x - c[0].x + g * c[1].x, b = c[3].x - c[0].x + h * c[3].x;
  const d = c[1].y - c[0].y + g * c[1].y, e = c[3].y - c[0].y + h * c[3].y;
  return [a, b, c[0].x, d, e, c[0].y, g, h, 1];
}
export function isUsableCorners(c: P[]): boolean {
  if (c.length !== 4 || !c.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y) && Math.abs(p.x) <= 1e6 && Math.abs(p.y) <= 1e6)) return false;
  let sign = 0;
  for (let i = 0; i < 4; i++) {
    const a = c[i], b = c[(i + 1) % 4], d = c[(i + 2) % 4];
    const cross = (b.x - a.x) * (d.y - b.y) - (b.y - a.y) * (d.x - b.x);
    if (Math.abs(cross) <= 0.01) return false;
    if (sign === 0) sign = cross < 0 ? -1 : 1; else if ((cross < 0) !== (sign < 0)) return false;
  }
  return true;
}
export function carriedCorners(placement: LayerTransform, by: LayerTransform, to: P[]): P[] {
  const c = center(by);
  const forward = mat3Mul(mat3Mul(mat3Mul(translation(c.x, c.y), rotation(radians(by))), scaling(by.size[0], by.size[1])), translation(-0.5, -0.5));
  const toUnit = mat3Invert(forward) ?? [1, 0, 0, 0, 1, 0, 0, 0, 1];
  const map = homographyUnitTo(to);
  return cornersOf(placement).map((p) => mat3Apply(map, mat3Apply(toUnit, p)));
}
export const toTuple = (p: P): PointTuple => [p.x, p.y];
export const fromTuple = (t: PointTuple): P => ({ x: t[0], y: t[1] });
export const cornersToTuples = (c: P[]): Corners => [toTuple(c[0]), toTuple(c[1]), toTuple(c[2]), toTuple(c[3])];

export type TransformDragMode = { kind: "move" } | { kind: "resize"; index: number } | { kind: "rotate" } | { kind: "distort"; index: number };
export interface DragOptions { lockRatio: boolean; shift: boolean; alt: boolean; }

/** Port of TransformDrag.updated. */
export function transformDrag(original: LayerTransform, start: P, mode: TransformDragMode) {
  return {
    updated(point: P, opts: DragOptions): LayerTransform {
      let result: LayerTransform = { ...original, origin: [...original.origin] as PointTuple, size: [...original.size] as PointTuple };
      switch (mode.kind) {
        case "distort": return original;
        case "move": {
          let dx = point.x - start.x, dy = point.y - start.y;
          if (opts.shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
          result.origin = [original.origin[0] + dx, original.origin[1] + dy];
          break;
        }
        case "rotate": {
          const c = center(original);
          const delta = Math.atan2(point.y - c.y, point.x - c.x) - Math.atan2(start.y - c.y, start.x - c.x);
          let r = original.rotation + delta * 180 / Math.PI;
          if (opts.shift) r = Math.round(r / 15) * 15;
          result.rotation = r;
          break;
        }
        case "resize": {
          const handle = HANDLES[mode.index];
          const anchorUnit = opts.alt ? { x: 0.5, y: 0.5 } : { x: 1 - handle.x, y: 1 - handle.y };
          const anchor = pointOf(original, anchorUnit);
          const initial = pointOf(original, handle);
          const dx = initial.x + point.x - start.x - anchor.x, dy = initial.y + point.y - start.y - anchor.y;
          const span = opts.alt ? 2 : 1; const r = radians(original);
          const localX = (dx * Math.cos(r) + dy * Math.sin(r)) * span, localY = (-dx * Math.sin(r) + dy * Math.cos(r)) * span;
          const sx = handle.x * 2 - 1, sy = handle.y * 2 - 1;
          const [ow, oh] = original.size;
          let width = sx === 0 ? ow : Math.max(1, localX * sx), height = sy === 0 ? oh : Math.max(1, localY * sy);
          if (opts.lockRatio !== opts.shift) {
            let factor: number;
            if (sx === 0) factor = height / oh; else if (sy === 0) factor = width / ow;
            else factor = Math.max(1 / Math.min(ow, oh), (localX * sx * ow + localY * sy * oh) / (ow * ow + oh * oh));
            width = ow * factor; height = oh * factor;
          }
          const offX = (0.5 - anchorUnit.x) * width, offY = (0.5 - anchorUnit.y) * height;
          const cx = anchor.x + offX * Math.cos(r) - offY * Math.sin(r), cy = anchor.y + offX * Math.sin(r) + offY * Math.cos(r);
          result.size = [width, height]; result.origin = [cx - width / 2, cy - height / 2];
          break;
        }
      }
      return isValidTransform(result) ? result : original;
    },
  };
}

/** Moving distortion corners: a corner handle moves its corner, an edge handle both of that edge's corners, "move" all four. */
export function cornersDrag(original: P[], start: P, mode: "move" | number) {
  return {
    updated(point: P, shift: boolean): P[] {
      let dx = point.x - start.x, dy = point.y - start.y;
      if (shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
      const moved = mode === "move" ? [0, 1, 2, 3] : mode % 2 === 0 ? [mode / 2] : [(mode - 1) / 2, ((mode - 1) / 2 + 1) % 4];
      return original.map((p, i) => (moved.includes(i) ? { x: p.x + dx, y: p.y + dy } : p));
    },
  };
}

export interface OverlayGeometry { handles: P[]; rotationHandle: P; showsRotation: boolean; }
export function overlayGeometry(shape: LayerTransform | P[], viewport: Viewport, docSize: SizeLike): OverlayGeometry {
  if (Array.isArray(shape)) {
    const v = shape.map((p) => viewport.viewPoint(p, docSize));
    const mid = (a: P, b: P) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
    return { handles: [v[0], mid(v[0], v[1]), v[1], mid(v[1], v[2]), v[2], mid(v[2], v[3]), v[3], mid(v[3], v[0])], rotationHandle: mid(v[0], v[1]), showsRotation: false };
  }
  const handles = HANDLES.map((h) => viewport.viewPoint(pointOf(shape, h), docSize));
  const r = radians(shape);
  return { handles, rotationHandle: { x: handles[1].x + Math.sin(r) * 28, y: handles[1].y - Math.cos(r) * 28 }, showsRotation: true };
}
export function hitOverlay(g: OverlayGeometry, point: P): TransformDragMode | null {
  const near = (o: P) => Math.hypot(point.x - o.x, point.y - o.y) <= 10;
  if (g.showsRotation && near(g.rotationHandle)) return { kind: "rotate" };
  const index = g.handles.findIndex(near);
  if (index >= 0) return { kind: "resize", index };
  for (const [s, e, handle] of [[0, 2, 1], [2, 4, 3], [4, 6, 5], [6, 0, 7]] as const) {
    const a = g.handles[s], b = g.handles[e]; const dx = b.x - a.x, dy = b.y - a.y; const len2 = dx * dx + dy * dy;
    if (len2 <= 0) continue;
    const t = ((point.x - a.x) * dx + (point.y - a.y) * dy) / len2;
    if (t >= 0 && t <= 1 && Math.hypot(point.x - a.x - t * dx, point.y - a.y - t * dy) <= 10) return { kind: "resize", index: handle };
  }
  return null;
}

/** Canvas edges and centre plus every other visible pixel layer's rounded upright bounds and centre. */
export function snapTargets(state: DocumentState, movingIds: string[]): { xs: number[]; ys: number[] } {
  const xs = [0, state.width / 2, state.width], ys = [0, state.height / 2, state.height];
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  for (const layer of state.layers) {
    if (layer.isGroup || !layer.hasPixels || movingIds.includes(layer.id)) continue;
    let node: typeof layer | undefined = layer; let visible = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { visible = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (!visible) continue;
    const b = boundsOf(layer.transform);
    xs.push(Math.round(b.x), Math.round(b.x + b.width / 2), Math.round(b.x + b.width));
    ys.push(Math.round(b.y), Math.round(b.y + b.height / 2), Math.round(b.y + b.height));
  }
  return { xs, ys };
}
function shift(guides: number[], targets: number[], tolerance: number): { move: number; target: number | null } {
  let best: { move: number; target: number } | null = null;
  for (const g of guides) for (const t of targets) { const move = t - g; if (Math.abs(move) > tolerance) continue; if (best && Math.abs(best.move) <= Math.abs(move)) continue; best = { move, target: t }; }
  return { move: best?.move ?? 0, target: best?.target ?? null };
}
export function snapOffset(box: RectLike, xs: number[], ys: number[], tolerance: number): { dx: number; dy: number; x: number | null; y: number | null } {
  const h = shift([box.x, box.x + box.width / 2, box.x + box.width], xs, tolerance);
  const v = shift([box.y, box.y + box.height / 2, box.y + box.height], ys, tolerance);
  return { dx: h.move, dy: v.move, x: h.target, y: v.target };
}
```

- [ ] **Step 5: Implement selection.ts and the store additions**

`app/src/state/selection.ts`:
```ts
import type { DocumentState, LayerState, LayerTransform } from "../engine/types";
import { boundsOfPoints, cornersOf } from "../tools/transform-geometry";

export function activeLayer(state: DocumentState): LayerState | null { return state.layers.find((l) => l.id === state.activeLayerId) ?? null; }
export function visibleIds(state: DocumentState): Set<string> {
  const byId = new Map(state.layers.map((l) => [l.id, l]));
  const out = new Set<string>();
  for (const layer of state.layers) {
    let node: LayerState | undefined = layer; let ok = true; let steps = 0;
    while (node && steps++ < 65) { if (!node.visible) { ok = false; break; } node = node.parentId ? byId.get(node.parentId) : undefined; }
    if (ok) out.add(layer.id);
  }
  return out;
}
export function transformsAsGroup(state: DocumentState, selected: string[]): boolean {
  if (selected.length > 1) return true;
  return selected.length === 1 && (state.layers.find((l) => l.id === selected[0])?.isGroup ?? false);
}
export function groupMembers(state: DocumentState, selected: string[]): LayerState[] {
  const visible = visibleIds(state); const byId = new Map(state.layers.map((l) => [l.id, l]));
  return state.layers.filter((l) => {
    if (l.isGroup || !l.hasPixels || !visible.has(l.id)) return false;
    let current: string | null = l.id; let steps = 0;
    while (current && steps++ < 65) { if (selected.includes(current)) return true; current = byId.get(current)?.parentId ?? null; }
    return false;
  });
}
export function groupBox(state: DocumentState, selected: string[]): LayerTransform | null {
  const pts = groupMembers(state, selected).flatMap((l) => cornersOf(l.transform));
  if (pts.length === 0) return null;
  const b = boundsOfPoints(pts);
  return { origin: [b.x, b.y], size: [Math.max(1, b.width), Math.max(1, b.height)], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
}
export function canTransform(state: DocumentState, selected: string[], maskSelected: boolean): boolean {
  if (transformsAsGroup(state, selected)) return groupMembers(state, selected).length > 0;
  const layer = activeLayer(state);
  if (!layer || layer.isGroup) return false;
  if (maskSelected && layer.hasMask && !layer.maskLinked) return visibleIds(state).has(layer.id);
  return layer.hasPixels && visibleIds(state).has(layer.id);
}
```

Store additions in `app/src/state/store.ts` (fields and actions listed in Interfaces). Key implementations:
```ts
  selectLayers: (ids, primary) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return;
    const state = get().documents[activeId];
    const valid = ids.filter((id) => state.layers.some((l) => l.id === id));
    const active = primary && valid.includes(primary) ? primary : valid[0] ?? null;
    get().commitTransform();
    if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
    set({ selectedLayerIds: valid, maskSelected: false });
    get().refresh(activeId);
  },
  beginTransform: ({ persistent, duplicate }) => {
    const { engine, activeId, selectedLayerIds, maskSelected } = get(); if (!engine || !activeId) return false;
    const state = get().documents[activeId];
    if (!canTransform(state, selectedLayerIds, maskSelected) || get().transformEdit) return false;
    if (transformsAsGroup(state, selectedLayerIds)) {
      const box = groupBox(state, selectedLayerIds)!;
      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false } });
      return true;
    }
    let layer = activeLayer(state)!;
    if (duplicate) { engine.execute(activeId, { type: "DuplicateLayer", id: layer.id }); get().refresh(activeId); layer = activeLayer(get().documents[activeId])!; set({ selectedLayerIds: [layer.id] }); }
    const maskAlone = maskSelected && layer.hasMask && !layer.maskLinked;
    const t = maskAlone ? layer.maskPlacement ?? layer.transform : layer.transform;
    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: !!duplicate } });
    return true;
  },
  previewTransform: (draft, corners) => { const e = get().transformEdit; if (!e || !isValidTransform(draft)) return; set({ transformEdit: { ...e, draft, corners: corners === undefined ? e.corners : corners } }); get().invalidate(); },
  beginDistort: () => { const e = get().transformEdit; if (!e || e.corners || e.kind === "mask") return; set({ transformEdit: { ...e, corners: cornersToTuples(cornersOf(e.draft)), persistent: true } }); },
  commitTransform: () => {
    const e = get().transformEdit; const { engine, activeId } = get(); if (!e || !engine || !activeId) return;
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    const draft = roundedTransform(e.draft);
    const unchanged = JSON.stringify(draft) === JSON.stringify(roundedTransform(e.original)) && !e.corners;
    if (unchanged) { if (e.duplicated) { /* keep the duplicate in place */ } get().invalidate(); return; }
    if (e.kind === "mask") get().run({ type: "SetMaskPlacement", id: e.id, placement: draft });
    else if (e.kind === "group") get().run(e.corners ? { type: "DistortLayers", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners } : { type: "TransformLayers", ids: e.ids, box: e.box, draft });
    else get().run(e.corners ? { type: "DistortLayer", id: e.id, transform: e.draft, corners: e.corners } : { type: "SetLayerTransform", id: e.id, transform: draft });
  },
  cancelTransform: () => {
    const e = get().transformEdit; if (!e) return;
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    if (e.duplicated) { get().undo(); }
    get().invalidate();
  },
  previewEdit: () => {
    const e = get().transformEdit; if (!e) return null;
    if (e.kind === "group") return { kind: "group", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners };
    if (e.kind === "mask") return { kind: "mask", id: e.id, draft: e.draft };
    return { kind: "layer", id: e.id, draft: e.draft, corners: e.corners };
  },
```
`refresh` reconciliation (inside `refresh` after reading `state`): `const keep = s.selectedLayerIds.filter((id) => state.layers.some((l) => l.id === id)); const selected = state.activeLayerId && !keep.includes(state.activeLayerId) ? [state.activeLayerId] : keep;` and include `selectedLayerIds: selected` in the `set`. `openDocument` sets `selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : []`, `maskSelected: false`, `transformEdit: null`. `setActive` and `closeDocument` also reset `selectedLayerIds` from the target document's active layer and clear `transformEdit`. `setTool` commits a pending transform when the tool changes away from "move".

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm test`, `pnpm build`, `cargo test -p compositor-engine --test engine2`
Expected: unit tests pass (transform-geometry 5, selection 3, plus the earlier 15); build clean; engine2 3 tests.

- [ ] **Step 7: Commit**

```
git add app engine
git commit -m "feat(app,engine): transform geometry, selection helpers, transform-edit store state"
```

---

### Task 11: WebGL2 renderer executes the render plan

**Files:**
- Create: `app/src/canvas/gl/programs.ts`, `app/src/canvas/gl/framebuffers.ts`, `app/src/canvas/gl/mask-textures.ts`
- Modify: `app/src/canvas/gl-renderer.ts` (rewrite), `app/src/canvas/renderer.ts`, `app/src/canvas/cpu-renderer.ts`, `app/src/canvas/CanvasView.tsx` (pass the edit and engine to `render`)
- Test: `app/tests/e2e/render.spec.ts` (extend), `app/tests/e2e/blend.spec.ts`

**Interfaces:**
- `Renderer.render(engine: EngineClient, state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions, edit: PreviewEdit | null)`; `sync(engine, state)` stays (layer chunk textures); mask textures are synced inside `render` from the plan's coverages.
- `programs.ts`: `BLEND_INDEX: Record<BlendMode, number>` (Normal 0 ... Luminosity 12 in manifest order), `createPrograms(gl): Programs` with `layer` (uniforms `unitToClip` mat3, `uvRect` vec4, `flipX`/`flipY` bool, `tex`, `opacity`, `mode` int, `backdrop`, `coverage`, `useCoverage` bool), `coverage` (`deviceToMask` mat3, `maskSize` vec2, `background` float, `mask` sampler), `alphaOf` (`src` sampler: out.r = src.a), `opaque` (`src`: rgb/a, a = 1 where a > 0), `restore` (`src`, `alpha` samplers), `blit` (`src`), `checker` (Phase 1); each program has its attribute set up on one shared VAO; `disposePrograms`.
- `framebuffers.ts`: `class FboPool { constructor(gl); resize(W, H); get(name: string, format: "rgba" | "r8"): { fbo: WebGLFramebuffer; tex: WebGLTexture }; swap(a, b); dispose() }` where textures are `RGBA8` or `R8` with NEAREST filtering and CLAMP_TO_EDGE; `blit(from, to)` copies with `blitFramebuffer`.
- `mask-textures.ts`: `class MaskTextures { sync(docId, layerId, revision, width, height, pixels: Uint8Array): WebGLTexture; setFilter(tex, nearest); retainOnly(docId, keys: Set<string>); dispose() }` (R8, `UNPACK_ALIGNMENT` 1).
- `GlRenderer.render` algorithm (all passes view-sized, in device pixels):
  1. `plan = engine.renderPlan(state.id, edit)`; sync mask textures for every coverage in the plan (`engine.maskPixels`).
  2. Clear `main` (rgba) to transparent. For each node: `Layer` -> `drawInto("main", draw, draw.blend, useClip = true, level 0)`; `Stack` -> clear `stackA`; `drawInto("stack", base, Normal, true, 0)`; `alphaOf(stackA -> baseAlpha)`; `opaque(stackA -> stackB)`, swap; each child `drawInto("stack", child, child.blend, false, 0)`; `restore(stackA, baseAlpha -> stackB)`, swap; coverage pass for `folderCoverages`; blit main -> mainB; draw the stack texture as a full-view quad with `flipY = true`, opacity 1, `mode = base.blend`, then swap.
  3. `drawInto(pair, draw, mode, useClip, level)`: coverage FBO `coverage{level}` cleared to 1.0; one full-view multiply pass per coverage; if `useClip && draw.clip`: render the source draw (from `plan.sources`) into `clip{level}` (cleared) with backdrop = a 1x1 transparent texture, Normal, its own coverages and clip at `level + 1` (levels beyond 3 skip the clip); then multiply `coverage{level}` by `clip{level}`'s alpha. Then blit A -> B of the pair, bind B, draw each chunk of the layer texture with the `layer` program: `unitToClip = viewToClip * homographyUnitTo(cornersOf(draw.transform) or draw.corners, in view CSS px)`, `uvRect` = the chunk's rectangle in layer-unit space, flips from the transform, `backdrop = A`, `coverage = coverage{level}`; swap.
  4. Screen pass: bind the default framebuffer, draw the checkerboard (or clear the document rect to transparent), then draw `main` over it with the `blit` program under premultiplied blending, scissored to the document rect.
  5. `readPixels` reads the default framebuffer (Phase 1 behaviour).
- `CpuRenderer.render` uses `engine.compositeEdit(state.id, edit, region, w, h)`.
- Device-to-mask matrix for a coverage: `deviceToMask = maskFromDoc * docFromDevice`, where `docFromDevice` maps `gl_FragCoord` (x right, y up from the bottom, device px) to document coordinates: `docX = (fx / dpr - rect.x) / ppp`, `docY = ((H - fy) / dpr - rect.y) / ppp`; `maskFromDoc` = `mat3Invert(pixelToDocument(placement, w, h))` for an affine coverage, or `flipScale * mat3Invert(homographyUnitTo(corners))` for a distorted one (`flipScale` maps unit to mask pixels with the placement's flips: `[fx ? -w : w, 0, fx ? w : 0, 0, fy ? -h : h, fy ? h : 0, 0, 0, 1]`).

- [ ] **Step 1: Write the failing e2e tests**

Extend `app/tests/e2e/render.spec.ts` with a shared helper `matchesCpu(page, label)` that reads the on-screen document pixels and compares with `engine.composite` (or `compositeEdit` when an edit is given) within 2/255 per byte, and add to `app/tests/e2e/blend.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64 } from "./helpers";

async function setup(page: Page): Promise<{ doc: string; a: string; b: string }> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  return page.evaluate(async (b64) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const png = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const doc = api.engine.newDocument(8, 8, false);
    api.engine.importImage(doc, png, "a", { x: 2, y: 2 });
    const a = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, png, "b", { x: 3, y: 3 });
    const b = api.engine.state(doc).activeLayerId;
    for (const id of [a, b]) { const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform; api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, sampling: "Nearest" } }); }
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return { doc, a, b };
  }, redSquarePngBase64());
}

async function expectMatchesCpu(page: Page, label: string, edit: unknown = null) {
  const r = await page.evaluate(async (edit) => {
    const api = (window as unknown as { __compositor: any }).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.compositeEdit(d.id, edit, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return { gl, cpu, kind: s.rendererKind };
  }, edit);
  expect(r.gl.length).toBe(r.cpu.length);
  const worst = r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0);
  expect(worst, `${label} (${r.kind}) max byte diff`).toBeLessThanOrEqual(2);
}

test("blend modes, opacity, masks, folder masks and clipping match the CPU compositor", async ({ page }) => {
  const { doc, a, b } = await setup(page);
  const run = (cmd: unknown) => page.evaluate(({ cmd }) => { const api = (window as any).__compositor; api.engine.execute(api.store.getState().activeId, cmd); api.store.getState().refresh(); }, { cmd });
  for (const mode of ["Multiply", "Screen", "Overlay", "Difference", "Color Dodge", "Color Burn", "Hue", "Luminosity"]) {
    await run({ type: "SetLayerBlendMode", id: b, mode });
    await expectMatchesCpu(page, mode);
  }
  await run({ type: "SetLayerBlendMode", id: b, mode: "Normal" });
  await run({ type: "SetLayerOpacity", id: b, opacity: 0.5 });
  await expectMatchesCpu(page, "opacity");
  await run({ type: "AddMask", id: b, revealing: true });
  await run({ type: "InvertMask", id: b });
  await expectMatchesCpu(page, "hide-all mask");
  await run({ type: "DeleteMask", id: b });
  await run({ type: "GroupLayers", ids: [a, b] });
  const folder = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return s.documents[s.activeId].layers.find((l: any) => l.isGroup).id; });
  await run({ type: "AddMask", id: folder, revealing: false });
  await expectMatchesCpu(page, "folder hide-all");
  await run({ type: "DeleteMask", id: folder });
  await run({ type: "ToggleClipping", id: b });
  await expectMatchesCpu(page, "clipping stack");
  await run({ type: "SetLayerVisible", id: a, visible: false });
  await expectMatchesCpu(page, "clipped to a hidden base");
  const _ = doc;
});

test("distortion preview and group transform preview match the CPU compositor", async ({ page }) => {
  const { a, b } = await setup(page);
  const edit = { kind: "layer", id: a, draft: { origin: [2, 2], size: [2, 2], rotation: 0, flipX: false, flipY: false, sampling: "Nearest" }, corners: [[1, 1], [7, 1], [6, 5], [1, 5]] };
  await page.evaluate((edit) => { const api = (window as any).__compositor; api.store.getState().selectLayers([edit.id], edit.id); api.store.setState({ transformEdit: { kind: "layer", id: edit.id, ids: [edit.id], box: edit.draft, original: edit.draft, draft: edit.draft, corners: edit.corners, persistent: true, duplicated: false } }); api.store.getState().invalidate(); }, edit);
  await expectMatchesCpu(page, "distort preview", edit);
  const group = { kind: "group", ids: [a, b], box: { origin: [2, 2], size: [3, 3], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, draft: { origin: [1, 1], size: [6, 6], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, corners: null };
  await page.evaluate((g) => { const api = (window as any).__compositor; api.store.setState({ transformEdit: { kind: "group", id: g.ids[0], ids: g.ids, box: g.box, original: g.box, draft: g.draft, corners: null, persistent: false, duplicated: false } }); api.store.getState().invalidate(); }, group);
  await expectMatchesCpu(page, "group preview", group);
});
```
(The CPU fallback test in `render.spec.ts` keeps passing: the same spec runs against both renderers when `webgl2` is stubbed out.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm e2e --grep "blend modes"`
Expected: fails (mismatch on Multiply: the Phase 1 renderer ignores blend modes).

- [ ] **Step 3: Implement programs.ts**

`app/src/canvas/gl/programs.ts`:
```ts
import type { BlendMode } from "../../engine/types";

export const BLEND_INDEX: Record<BlendMode, number> = { Normal: 0, Multiply: 1, Screen: 2, Overlay: 3, Darken: 4, Lighten: 5, Difference: 6, "Color Dodge": 7, "Color Burn": 8, Hue: 9, Saturation: 10, Color: 11, Luminosity: 12 };

const VERT_UNIT = `#version 300 es
in vec2 unit;
uniform mat3 unitToClip;
uniform vec4 uvRect;
uniform bool flipX;
uniform bool flipY;
out vec2 uv;
void main() {
  vec2 f = uvRect.xy + unit * uvRect.zw;
  vec2 lu = vec2(flipX ? 1.0 - f.x : f.x, flipY ? 1.0 - f.y : f.y);
  vec3 p = unitToClip * vec3(lu, 1.0);
  gl_Position = vec4(p.xy, 0.0, p.z);
  uv = unit;
}`;
const VERT_SCREEN = `#version 300 es
in vec2 unit;
out vec2 uv;
void main() { gl_Position = vec4(unit * 2.0 - 1.0, 0.0, 1.0); uv = unit; }`;

const BLEND_GLSL = `
float lum(vec3 c) { return dot(c, vec3(0.3, 0.59, 0.11)); }
vec3 clipColor(vec3 c) {
  float l = lum(c); float n = min(c.r, min(c.g, c.b)); float x = max(c.r, max(c.g, c.b));
  if (n < 0.0) c = l + (c - l) * l / max(l - n, 1e-6);
  if (x > 1.0) c = l + (c - l) * (1.0 - l) / max(x - l, 1e-6);
  return c;
}
vec3 setLum(vec3 c, float l) { return clipColor(c + (l - lum(c))); }
float sat(vec3 c) { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
vec3 setSat(vec3 c, float s) {
  float mx = max(c.r, max(c.g, c.b)); float mn = min(c.r, min(c.g, c.b));
  return mx > mn ? (c - mn) * s / (mx - mn) : vec3(0.0);
}
float sep(int mode, float cb, float cs) {
  if (mode == 1) return cb * cs;
  if (mode == 2) return cb + cs - cb * cs;
  if (mode == 3) { if (cb <= 0.5) return cs * 2.0 * cb; float s = 2.0 * cb - 1.0; return cs + s - cs * s; }
  if (mode == 4) return min(cb, cs);
  if (mode == 5) return max(cb, cs);
  if (mode == 6) return abs(cb - cs);
  if (mode == 7) return cb <= 0.0 ? 0.0 : (cs >= 1.0 ? 1.0 : min(1.0, cb / (1.0 - cs)));
  if (mode == 8) return cb >= 1.0 ? 1.0 : (cs <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / cs));
  return cs;
}
vec3 blendRgb(int mode, vec3 cb, vec3 cs) {
  if (mode == 9) return setLum(setSat(cs, sat(cb)), lum(cb));
  if (mode == 10) return setLum(setSat(cb, sat(cs)), lum(cb));
  if (mode == 11) return setLum(cs, lum(cb));
  if (mode == 12) return setLum(cb, lum(cs));
  return vec3(sep(mode, cb.r, cs.r), sep(mode, cb.g, cs.g), sep(mode, cb.b, cs.b));
}
vec4 compose(vec4 dst, vec4 src, int mode) {
  float ad = dst.a; float as = src.a;
  if (as <= 0.0) return dst;
  float outA = as + ad * (1.0 - as);
  if (mode == 0 || ad <= 0.0) return vec4(src.rgb + dst.rgb * (1.0 - as), outA);
  vec3 cb = dst.rgb / ad; vec3 cs = src.rgb / as;
  vec3 b = clamp(blendRgb(mode, cb, cs), 0.0, 1.0);
  return vec4(clamp(src.rgb * (1.0 - ad) + dst.rgb * (1.0 - as) + as * ad * b, 0.0, 1.0), outA);
}`;

const FRAG_LAYER = `#version 300 es
precision highp float;
in vec2 uv;
uniform sampler2D tex;
uniform sampler2D backdrop;
uniform sampler2D coverage;
uniform bool useCoverage;
uniform float opacity;
uniform int mode;
out vec4 color;
${BLEND_GLSL}
void main() {
  ivec2 at = ivec2(gl_FragCoord.xy);
  float k = opacity * (useCoverage ? texelFetch(coverage, at, 0).r : 1.0);
  vec4 s = texture(tex, uv) * k;
  vec4 d = texelFetch(backdrop, at, 0);
  color = compose(d, s, mode);
}`;
const FRAG_COVERAGE = `#version 300 es
precision highp float;
uniform mat3 deviceToMask;
uniform vec2 maskSize;
uniform float background;
uniform sampler2D mask;
out vec4 color;
void main() {
  vec3 p = deviceToMask * vec3(gl_FragCoord.xy, 1.0);
  vec2 m = p.xy / p.z;
  float v = (m.x < 0.0 || m.y < 0.0 || m.x >= maskSize.x || m.y >= maskSize.y) ? background : texture(mask, m / maskSize).r;
  color = vec4(v, v, v, 1.0);
}`;
const FRAG_ALPHA_OF = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { float a = texelFetch(src, ivec2(gl_FragCoord.xy), 0).a; color = vec4(a, a, a, 1.0); }`;
const FRAG_OPAQUE = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0); color = c.a > 0.0 ? vec4(c.rgb / c.a, 1.0) : vec4(0.0); }`;
const FRAG_RESTORE = `#version 300 es
precision highp float;
uniform sampler2D src;
uniform sampler2D alpha;
out vec4 color;
void main() { ivec2 at = ivec2(gl_FragCoord.xy); float a = texelFetch(alpha, at, 0).r; vec4 c = texelFetch(src, at, 0); color = vec4(c.rgb * a, a); }`;
const FRAG_BLIT = `#version 300 es
precision highp float;
uniform sampler2D src;
out vec4 color;
void main() { color = texelFetch(src, ivec2(gl_FragCoord.xy), 0); }`;
const FRAG_CHECKER = `#version 300 es
precision highp float;
in vec2 uv;
uniform vec2 sizePx;
uniform float cell;
out vec4 color;
void main() { vec2 p = floor(uv * sizePx / cell); float c = mod(p.x + p.y, 2.0) < 1.0 ? 0.80 : 0.95; color = vec4(c, c, c, 1.0); }`;

export interface Program { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null>; }
export interface Programs { layer: Program; coverage: Program; alphaOf: Program; opaque: Program; restore: Program; blit: Program; checker: Program; vao: WebGLVertexArrayObject; buffer: WebGLBuffer; }

function compile(gl: WebGL2RenderingContext, vert: string, frag: string, uniforms: string[]): Program {
  const make = (type: number, src: string) => { const s = gl.createShader(type)!; gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader"); return s; };
  const program = gl.createProgram()!;
  gl.attachShader(program, make(gl.VERTEX_SHADER, vert)); gl.attachShader(program, make(gl.FRAGMENT_SHADER, frag)); gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program) ?? "link");
  const u: Record<string, WebGLUniformLocation | null> = {};
  for (const name of uniforms) u[name] = gl.getUniformLocation(program, name);
  return { program, uniforms: u };
}

export function createPrograms(gl: WebGL2RenderingContext): Programs {
  const vao = gl.createVertexArray()!; gl.bindVertexArray(vao);
  const buffer = gl.createBuffer()!; gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
  const programs: Programs = {
    layer: compile(gl, VERT_UNIT, FRAG_LAYER, ["unitToClip", "uvRect", "flipX", "flipY", "tex", "backdrop", "coverage", "useCoverage", "opacity", "mode"]),
    coverage: compile(gl, VERT_SCREEN, FRAG_COVERAGE, ["deviceToMask", "maskSize", "background", "mask"]),
    alphaOf: compile(gl, VERT_SCREEN, FRAG_ALPHA_OF, ["src"]),
    opaque: compile(gl, VERT_SCREEN, FRAG_OPAQUE, ["src"]),
    restore: compile(gl, VERT_SCREEN, FRAG_RESTORE, ["src", "alpha"]),
    blit: compile(gl, VERT_SCREEN, FRAG_BLIT, ["src"]),
    checker: compile(gl, VERT_UNIT, FRAG_CHECKER, ["unitToClip", "uvRect", "flipX", "flipY", "sizePx", "cell"]),
    vao, buffer,
  };
  for (const p of [programs.layer, programs.coverage, programs.alphaOf, programs.opaque, programs.restore, programs.blit, programs.checker]) {
    const loc = gl.getAttribLocation(p.program, "unit"); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
  }
  return programs;
}
export function disposePrograms(gl: WebGL2RenderingContext, p: Programs): void {
  for (const q of [p.layer, p.coverage, p.alphaOf, p.opaque, p.restore, p.blit, p.checker]) gl.deleteProgram(q.program);
  gl.deleteVertexArray(p.vao); gl.deleteBuffer(p.buffer);
}
```

- [ ] **Step 4: Implement framebuffers.ts and mask-textures.ts**

`app/src/canvas/gl/framebuffers.ts`:
```ts
export interface Target { fbo: WebGLFramebuffer; tex: WebGLTexture; format: "rgba" | "r8"; }

export class FboPool {
  private targets = new Map<string, Target>();
  private W = 0; private H = 0;
  constructor(private readonly gl: WebGL2RenderingContext) {}
  resize(W: number, H: number): void { if (W === this.W && H === this.H) return; this.W = W; this.H = H; this.dispose(); }
  get(name: string, format: "rgba" | "r8"): Target {
    const existing = this.targets.get(name);
    if (existing && existing.format === format) return existing;
    if (existing) this.remove(name);
    const gl = this.gl;
    const tex = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, tex);
    if (format === "rgba") gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, this.W, this.H, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    else gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, this.W, this.H, 0, gl.RED, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer()!; gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error(`framebuffer ${name} incomplete`);
    const t = { fbo, tex, format }; this.targets.set(name, t); return t;
  }
  swap(a: string, b: string): void { const ta = this.targets.get(a), tb = this.targets.get(b); if (ta && tb) { this.targets.set(a, tb); this.targets.set(b, ta); } }
  blit(from: string, to: string, format: "rgba" | "r8" = "rgba"): void {
    const gl = this.gl; const f = this.get(from, format), t = this.get(to, format);
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, f.fbo); gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, t.fbo);
    gl.blitFramebuffer(0, 0, this.W, this.H, 0, 0, this.W, this.H, gl.COLOR_BUFFER_BIT, gl.NEAREST);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }
  clear(name: string, format: "rgba" | "r8", value: number): void {
    const gl = this.gl; gl.bindFramebuffer(gl.FRAMEBUFFER, this.get(name, format).fbo); gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(value, value, value, value); gl.clear(gl.COLOR_BUFFER_BIT);
  }
  private remove(name: string): void { const t = this.targets.get(name); if (!t) return; this.gl.deleteFramebuffer(t.fbo); this.gl.deleteTexture(t.tex); this.targets.delete(name); }
  dispose(): void { for (const k of [...this.targets.keys()]) this.remove(k); }
}
```

`app/src/canvas/gl/mask-textures.ts`:
```ts
interface Entry { tex: WebGLTexture; revision: number; }
export class MaskTextures {
  private masks = new Map<string, Entry>();
  constructor(private readonly gl: WebGL2RenderingContext) {}
  private key(docId: string, layerId: string): string { return `${docId}:${layerId}`; }
  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array): WebGLTexture {
    const k = this.key(docId, layerId); const e = this.masks.get(k);
    if (e && e.revision === revision) return e.tex;
    if (e) this.gl.deleteTexture(e.tex);
    const gl = this.gl; const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, pixels);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    this.masks.set(k, { tex, revision }); return tex;
  }
  setFilter(tex: WebGLTexture, nearest: boolean): void {
    const gl = this.gl; gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, nearest ? gl.NEAREST : gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, nearest ? gl.NEAREST : gl.LINEAR);
  }
  retainOnly(docId: string, layerIds: Set<string>): void {
    for (const k of [...this.masks.keys()]) { const [d, l] = k.split(":"); if (d !== docId || !layerIds.has(l)) { this.gl.deleteTexture(this.masks.get(k)!.tex); this.masks.delete(k); } }
  }
  dispose(): void { for (const e of this.masks.values()) this.gl.deleteTexture(e.tex); this.masks.clear(); }
}
```

- [ ] **Step 5: Rewrite gl-renderer.ts**

`app/src/canvas/gl-renderer.ts` (full):
```ts
import type { Coverage, DocumentState, LayerDraw, PreviewEdit, RenderPlan } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { Viewport } from "./viewport";
import { LayerTextures } from "./layer-textures";
import type { RenderOptions, Renderer } from "./renderer";
import { BLEND_INDEX, createPrograms, disposePrograms, type Programs } from "./gl/programs";
import { FboPool } from "./gl/framebuffers";
import { MaskTextures } from "./gl/mask-textures";
import { cornersOf, fromTuple, homographyUnitTo, mat3Invert, mat3Mul, pixelToDocument, type Mat3, type P } from "../tools/transform-geometry";

const MAX_CLIP_LEVELS = 3;

export class GlRenderer implements Renderer {
  readonly kind = "gl" as const;
  private textures: LayerTextures;
  private masks: MaskTextures;
  private fbos: FboPool;
  private programs: Programs;
  private white: WebGLTexture;
  private transparent: WebGLTexture;
  private W = 0; private H = 0;
  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext) {
    this.textures = new LayerTextures(gl); this.masks = new MaskTextures(gl); this.fbos = new FboPool(gl); this.programs = createPrograms(gl);
    this.white = this.solid(gl.R8, gl.RED, [255]); this.transparent = this.solid(gl.RGBA8, gl.RGBA, [0, 0, 0, 0]);
  }
  private solid(internal: number, format: number, bytes: number[]): WebGLTexture {
    const gl = this.gl; const t = gl.createTexture()!; gl.bindTexture(gl.TEXTURE_2D, t);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.texImage2D(gl.TEXTURE_2D, 0, internal, 1, 1, 0, format, gl.UNSIGNED_BYTE, new Uint8Array(bytes)); gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    return t;
  }

  sync(engine: EngineClient, state: DocumentState): void {
    const keep = new Set<string>();
    for (const layer of state.layers) { keep.add(layer.id); const pixels = layer.pixelsWidth > 0 ? engine.layerPixels(state.id, layer.id) : null; this.textures.sync(state.id, layer, pixels); }
    this.textures.retainOnly(state.id, keep);
  }

  render(engine: EngineClient, state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions, edit: PreviewEdit | null): void {
    const gl = this.gl;
    const W = Math.max(1, Math.round(viewport.viewSize.width * dpr)), H = Math.max(1, Math.round(viewport.viewSize.height * dpr));
    if (this.canvas.width !== W || this.canvas.height !== H) { this.canvas.width = W; this.canvas.height = H; }
    this.W = W; this.H = H; this.fbos.resize(W, H);
    const plan = engine.renderPlan(state.id, edit);
    this.syncMasks(engine, state, plan);
    gl.bindVertexArray(this.programs.vao);
    gl.viewport(0, 0, W, H);
    gl.disable(gl.BLEND);
    const ctx: Ctx = { state, plan, viewport, dpr };
    this.fbos.clear("mainA", "rgba", 0);
    for (const node of plan.nodes) {
      if (node.kind === "layer") this.drawInto(ctx, "main", node.draw, node.draw.blend, true, 0);
      else {
        this.fbos.clear("stackA", "rgba", 0);
        this.drawInto(ctx, "stack", node.base, "Normal", true, 0);
        this.pass(this.programs.alphaOf, "baseAlpha", "r8", { src: this.fbos.get("stackA", "rgba").tex });
        this.pass(this.programs.opaque, "stackB", "rgba", { src: this.fbos.get("stackA", "rgba").tex }); this.fbos.swap("stackA", "stackB");
        for (const child of node.children) this.drawInto(ctx, "stack", child, child.blend, false, 0);
        this.pass(this.programs.restore, "stackB", "rgba", { src: this.fbos.get("stackA", "rgba").tex, alpha: this.fbos.get("baseAlpha", "r8").tex }); this.fbos.swap("stackA", "stackB");
        this.buildCoverage(ctx, node.folderCoverages, 0);
        this.composeTexture(ctx, "main", this.fbos.get("stackA", "rgba").tex, this.viewCorners(viewport), { x: 0, y: 0, w: 1, h: 1 }, false, true, 1, BLEND_INDEX[node.base.blend], node.folderCoverages.length > 0 ? 0 : null);
      }
    }
    this.screenPass(state, viewport, dpr, options);
  }

  private syncMasks(engine: EngineClient, state: DocumentState, plan: RenderPlan): void {
    const keep = new Set<string>();
    const visit = (c: Coverage) => { keep.add(c.layerId); const px = engine.maskPixels(state.id, c.layerId); if (px) this.masks.sync(state.id, c.layerId, c.maskRevision, c.width, c.height, px); };
    for (const n of plan.nodes) { if (n.kind === "layer") n.draw.coverages.forEach(visit); else { n.base.coverages.forEach(visit); n.children.forEach((c) => c.coverages.forEach(visit)); n.folderCoverages.forEach(visit); } }
    for (const s of plan.sources) s.coverages.forEach(visit);
    this.masks.retainOnly(state.id, keep);
  }

  /** Document -> view CSS px. */
  private docToView(viewport: Viewport, state: DocumentState): Mat3 {
    const rect = viewport.documentRect({ width: state.width, height: state.height }); const ppp = viewport.pointsPerPixel;
    return [ppp, 0, rect.x, 0, ppp, rect.y, 0, 0, 1];
  }
  /** View CSS px -> clip space. */
  private viewToClip(viewport: Viewport): Mat3 { const vw = viewport.viewSize.width, vh = viewport.viewSize.height; return [2 / vw, 0, -1, 0, -2 / vh, 1, 0, 0, 1]; }
  /** gl_FragCoord (device px, y up) -> document. */
  private deviceToDoc(viewport: Viewport, state: DocumentState, dpr: number): Mat3 {
    const rect = viewport.documentRect({ width: state.width, height: state.height }); const ppp = viewport.pointsPerPixel;
    return [1 / (dpr * ppp), 0, -rect.x / ppp, 0, -1 / (dpr * ppp), (this.H / dpr - rect.y) / ppp, 0, 0, 1];
  }
  private viewCorners(viewport: Viewport): P[] { const w = viewport.viewSize.width, h = viewport.viewSize.height; return [{ x: 0, y: 0 }, { x: w, y: 0 }, { x: w, y: h }, { x: 0, y: h }]; }

  private buildCoverage(ctx: Ctx, coverages: Coverage[], level: number): void {
    const gl = this.gl; const name = `coverage${level}`;
    this.fbos.clear(name, "r8", 1);
    if (coverages.length === 0) return;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(name, "r8").fbo);
    gl.enable(gl.BLEND); gl.blendFunc(gl.DST_COLOR, gl.ZERO);
    const p = this.programs.coverage; gl.useProgram(p.program);
    const d2d = this.deviceToDoc(ctx.viewport, ctx.state, ctx.dpr);
    for (const c of coverages) {
      const tex = this.masks.sync(ctx.state.id, c.layerId, c.maskRevision, c.width, c.height, new Uint8Array(0)); // already synced; returns the cached texture
      this.masks.setFilter(tex, c.nearest);
      let maskFromDoc: Mat3 | null;
      if (c.corners) {
        const inv = mat3Invert(homographyUnitTo(c.corners.map(fromTuple)));
        const fx = c.placement.flipX, fy = c.placement.flipY;
        maskFromDoc = inv ? mat3Mul([fx ? -c.width : c.width, 0, fx ? c.width : 0, 0, fy ? -c.height : c.height, fy ? c.height : 0, 0, 0, 1], inv) : null;
      } else maskFromDoc = mat3Invert(pixelToDocument(c.placement, c.width, c.height));
      if (!maskFromDoc) continue;
      const m = mat3Mul(maskFromDoc, d2d);
      gl.uniformMatrix3fv(p.uniforms.deviceToMask, true, new Float32Array(m));
      gl.uniform2f(p.uniforms.maskSize, c.width, c.height);
      gl.uniform1f(p.uniforms.background, c.background / 255);
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms.mask, 0);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    }
    gl.disable(gl.BLEND);
  }

  /** Multiplies coverage{level} by the alpha of the source drawn into clip{level}. */
  private applyClip(ctx: Ctx, sourceId: string, level: number): void {
    const gl = this.gl;
    const source = ctx.plan.sources.find((s) => s.id === sourceId);
    if (!source || level >= MAX_CLIP_LEVELS) return;
    this.fbos.clear(`clip${level}`, "rgba", 0);
    this.buildCoverage(ctx, source.coverages, level + 1);
    if (source.clip) this.applyClip(ctx, source.clip, level + 1);
    this.drawLayer(ctx, `clip${level}`, this.transparent, source, 0, source.coverages.length > 0 || !!source.clip ? level + 1 : null);
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(`coverage${level}`, "r8").fbo);
    gl.enable(gl.BLEND); gl.blendFunc(gl.DST_COLOR, gl.ZERO);
    const p = this.programs.alphaOf; gl.useProgram(p.program);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get(`clip${level}`, "rgba").tex); gl.uniform1i(p.uniforms.src, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.disable(gl.BLEND);
  }

  private drawInto(ctx: Ctx, pair: "main" | "stack", draw: LayerDraw, blend: string, useClip: boolean, level: number): void {
    const t = this.textures.get(ctx.state.id, draw.id);
    if (!t) return;
    const hasCoverage = draw.coverages.length > 0 || (useClip && !!draw.clip);
    if (hasCoverage) { this.buildCoverage(ctx, draw.coverages, level); if (useClip && draw.clip) this.applyClip(ctx, draw.clip, level); }
    this.fbos.blit(`${pair}A`, `${pair}B`);
    const backdrop = this.fbos.get(`${pair}A`, "rgba").tex;
    this.drawLayer(ctx, `${pair}B`, backdrop, draw, BLEND_INDEX[blend as keyof typeof BLEND_INDEX], hasCoverage ? level : null);
    this.fbos.swap(`${pair}A`, `${pair}B`);
  }

  /** Draws every chunk of a layer texture into `target` reading `backdrop`. */
  private drawLayer(ctx: Ctx, target: string, backdrop: WebGLTexture, draw: LayerDraw, mode: number, coverageLevel: number | null): void {
    const t = this.textures.get(ctx.state.id, draw.id); if (!t) return;
    const d2v = this.docToView(ctx.viewport, ctx.state);
    const cornersDoc = draw.corners ? draw.corners.map(fromTuple) : cornersOf(draw.transform);
    const cornersView = cornersDoc.map((p) => ({ x: d2v[0] * p.x + d2v[1] * p.y + d2v[2], y: d2v[3] * p.x + d2v[4] * p.y + d2v[5] }));
    for (const chunk of t.chunks) {
      const rect = { x: chunk.x / t.width, y: chunk.y / t.height, w: chunk.width / t.width, h: chunk.height / t.height };
      this.composeTexture(ctx, target, chunk.texture, cornersView, rect, draw.transform.flipX, draw.transform.flipY, draw.opacity, mode, coverageLevel, backdrop);
    }
  }

  private composeTexture(ctx: Ctx, target: string, tex: WebGLTexture, cornersView: P[], uvRect: { x: number; y: number; w: number; h: number }, flipX: boolean, flipY: boolean, opacity: number, mode: number, coverageLevel: number | null, backdrop?: WebGLTexture): void {
    const gl = this.gl;
    if (target === "main") { this.fbos.blit("mainA", "mainB"); backdrop = this.fbos.get("mainA", "rgba").tex; target = "mainB"; }
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(target, "rgba").fbo);
    const p = this.programs.layer; gl.useProgram(p.program);
    const unitToClip = mat3Mul(this.viewToClip(ctx.viewport), homographyUnitTo(cornersView));
    gl.uniformMatrix3fv(p.uniforms.unitToClip, true, new Float32Array(unitToClip));
    gl.uniform4f(p.uniforms.uvRect, uvRect.x, uvRect.y, uvRect.w, uvRect.h);
    gl.uniform1i(p.uniforms.flipX, flipX ? 1 : 0); gl.uniform1i(p.uniforms.flipY, flipY ? 1 : 0);
    gl.uniform1f(p.uniforms.opacity, opacity); gl.uniform1i(p.uniforms.mode, mode);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms.tex, 0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, backdrop ?? this.transparent); gl.uniform1i(p.uniforms.backdrop, 1);
    gl.activeTexture(gl.TEXTURE2); gl.bindTexture(gl.TEXTURE_2D, coverageLevel === null ? this.white : this.fbos.get(`coverage${coverageLevel}`, "r8").tex); gl.uniform1i(p.uniforms.coverage, 2);
    gl.uniform1i(p.uniforms.useCoverage, coverageLevel === null ? 0 : 1);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    if (target === "mainB") this.fbos.swap("mainA", "mainB");
  }

  private pass(p: { program: WebGLProgram; uniforms: Record<string, WebGLUniformLocation | null> }, target: string, format: "rgba" | "r8", textures: Record<string, WebGLTexture>): void {
    const gl = this.gl; gl.bindFramebuffer(gl.FRAMEBUFFER, this.fbos.get(target, format).fbo); gl.useProgram(p.program);
    let unit = 0;
    for (const [name, tex] of Object.entries(textures)) { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex); gl.uniform1i(p.uniforms[name], unit); unit++; }
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  private screenPass(state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions): void {
    const gl = this.gl; const W = this.W, H = this.H;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null); gl.viewport(0, 0, W, H); gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(0.16, 0.16, 0.16, 1); gl.clear(gl.COLOR_BUFFER_BIT);
    const rect = viewport.documentRect({ width: state.width, height: state.height });
    const x0 = Math.round(rect.x * dpr), x1 = Math.round((rect.x + rect.width) * dpr), y0 = Math.round(rect.y * dpr), y1 = Math.round((rect.y + rect.height) * dpr);
    gl.enable(gl.SCISSOR_TEST); gl.scissor(x0, H - y1, x1 - x0, y1 - y0);
    if (options.checkerboard) {
      const p = this.programs.checker; gl.useProgram(p.program);
      gl.uniformMatrix3fv(p.uniforms.unitToClip, true, new Float32Array(mat3Mul(this.viewToClip(viewport), homographyUnitTo([{ x: rect.x, y: rect.y }, { x: rect.x + rect.width, y: rect.y }, { x: rect.x + rect.width, y: rect.y + rect.height }, { x: rect.x, y: rect.y + rect.height }]))));
      gl.uniform4f(p.uniforms.uvRect, 0, 0, 1, 1); gl.uniform1i(p.uniforms.flipX, 0); gl.uniform1i(p.uniforms.flipY, 0);
      gl.uniform2f(p.uniforms.sizePx, rect.width * dpr, rect.height * dpr); gl.uniform1f(p.uniforms.cell, 8 * dpr);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    } else { gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); }
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    const b = this.programs.blit; gl.useProgram(b.program);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.fbos.get("mainA", "rgba").tex); gl.uniform1i(b.uniforms.src, 0);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.disable(gl.BLEND); gl.disable(gl.SCISSOR_TEST);
  }

  readPixels(): Uint8Array {
    const gl = this.gl; const W = this.canvas.width, H = this.canvas.height;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    const out = new Uint8Array(W * H * 4); gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, out);
    const row = W * 4; const flipped = new Uint8Array(W * H * 4);
    for (let y = 0; y < H; y++) flipped.set(out.subarray(y * row, (y + 1) * row), (H - 1 - y) * row);
    return flipped;
  }
  dispose(): void { this.textures.dispose(); this.masks.dispose(); this.fbos.dispose(); disposePrograms(this.gl, this.programs); this.gl.deleteTexture(this.white); this.gl.deleteTexture(this.transparent); }
}

interface Ctx { state: DocumentState; plan: RenderPlan; viewport: Viewport; dpr: number; }
```
Notes for the implementer: `MaskTextures.sync` with an empty array must return the cached texture when the revision matches (it does: the early return happens before any upload; when no entry exists yet, `syncMasks` has already uploaded it). `uniformMatrix3fv(..., true, ...)` transposes the row-major arrays into GL's column-major layout. The screen-space quad programs use `VERT_SCREEN`, which ignores `unitToClip`. Remove the Phase 1 `unitToClip()` helper and the old `layerToView`.

Update `renderer.ts` (`render(engine, state, viewport, dpr, options, edit)`), `cpu-renderer.ts` (`engine.compositeEdit(state.id, edit, region, outW, outH)`) and `CanvasView.tsx` (the draw effect passes `engine` and `useEditor.getState().previewEdit()`, and subscribes to `transformEdit` so previews redraw).

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm build && pnpm e2e`
Expected: all e2e pass, including `blend.spec.ts` (2 tests) and the existing render specs. If a blend mode is off by more than 2, compare `BLEND_GLSL` against `blend.rs` term by term before touching tolerances; if only Color Dodge/Burn differ at exactly-1.0 inputs, check the `<=`/`>=` edge conditions match.

- [ ] **Step 7: Commit**

```
git add app
git commit -m "feat(app): WebGL2 renderer executes the render plan (blend modes, masks, clipping stacks, distortion previews)"
```

---

### Task 12: Layers panel: rows, multi-select, folders, drag and drop, masks, properties, context menu

**Files:**
- Create: `app/src/actions/layers.ts`, `app/src/panels/LayerProperties.tsx`, `app/src/panels/ContextMenu.tsx`, `app/src/panels/layer-rows.ts`
- Modify: `app/src/panels/LayersList.tsx` (rewrite), `app/src/styles.css`, `app/src/App.tsx` (mount `LayerProperties` above the list)
- Test: `app/tests/unit/layer-rows.test.ts`, `app/tests/e2e/layers.spec.ts`

**Interfaces:**
- `layer-rows.ts`: `Row { layer: LayerState; depth: number; visible: boolean; collapsed: boolean }`, `layerRows(state, collapsedIds: string[]): Row[]` (top-first depth-first; a collapsed folder hides its subtree), `dropTarget(rows, index, zone: "above" | "below" | "into"): { parent: string | null; above: string | null; atBottom: boolean }` where `above` means "insert directly above this layer id in that parent": zone "above" on row i inserts above `rows[i]` in its parent; zone "below" on row i inserts above the next row in the same parent when there is one, else at the bottom of that parent (`above: null, atBottom: false` inside a folder means the top; so "below the last child" resolves to `parent: rows[i].parentId, above: <the row itself> ... `). Define precisely: `below` = the layer directly beneath `rows[i]` in the flat top-first list of the same parent: if `rows[j]` (j > i, same parent, not a descendant of rows[i]) exists then `{ parent, above: rows[j].id }`, otherwise `{ parent, above: null, atBottom: parent === null }` (root bottom) or, inside a folder, the folder's lowest position expressed as `{ parent: folder, above: null, atBottom: false }` is the TOP, so instead return `{ parent: rows[i].parentId, above: rows[i].id, atBottom: false, belowOf: rows[i].id }`; the engine's `PlaceLayer` takes `above` = "insert directly above this id", so to place directly BELOW `rows[i]` we pass `above` = the sibling under it, or when none, `atBottom: true` for root, or for a folder `parent: folder, above: null, atBottom: false` followed by... To keep it simple and testable: `dropTarget` returns `{ parent, above, atBottom }` with the rule "zone below row i" = `{ parent: rows[i].layer.parentId, above: nextSiblingBelow(rows, i) ?? null, atBottom: nextSiblingBelow == null && rows[i].layer.parentId == null }`; when there is no sibling below inside a folder, `above: null, atBottom: false` places at the TOP of that folder, which is wrong, so `dropTarget` returns `{ parent, above: null, atBottom: false, bottomOfFolder: folderId }` and `placeDropped` in `actions/layers.ts` handles `bottomOfFolder` by placing at root bottom then... (also wrong). Resolution: add an engine-side option: `PlaceLayer.above = null` with `atBottom = true` and `parent = Some(folder)` means the bottom of that folder (insertion index = the folder's index + 1, i.e. directly above the folder record in the array, which is the folder's lowest child slot). Adjust `hierarchy::place_layer`: when `at_bottom && parent.is_some()`, `insertion = index_of(parent) + 1` (after removing the moved layer). Then `dropTarget` is clean: zone "below" with no sibling below returns `{ parent, above: null, atBottom: true }` for any parent.
- `actions/layers.ts` (all operate on the store's active document and selection; each is a plain exported function): `deleteSelected()` (prompts with `window.confirm` when `engine.clipDependents` is non-empty: OK = bake, Cancel = keep links; sends `DeleteLayers`), `duplicateSelected()`, `groupSelected()`, `mergeSelected()` (uses `mergeAction` to decide; no-op when null), `addFolder()`, `addMaskToActive(revealing)`, `deleteMaskOfActive()`, `toggleMaskEnabled()`, `toggleMaskLink()`, `invertMaskOfActive()`, `fillMaskOfActive(white)`, `blurMaskOfActive()` (prompts for a radius with `window.prompt`, default 4), `toggleClippingOfActive()`, `flipSelected(horizontal)`, `moveActiveBy(offset)`, `placeDropped(id, target, copy: boolean)`, `setOpacityOfSelected(opacity)`, `setBlendModeOfActive(mode)`, `cycleBlendMode(forward)`.
- `LayerProperties`: opacity slider (0..100, `aria-label="Opacity"`, applies `SetLayersOpacity` on change for the selection, committing once per drag: engine commands are whole steps, so the slider sends one command on `pointerup`/`change` and previews through `blendPreview`-like local state only for the number readout), blend mode `<select aria-label="Blend mode">` (all 13, disabled for folders), mask buttons row (Add reveal / Add hide when no mask; Disable/Enable, Link/Unlink, Invert, Delete when there is one).
- `ContextMenu`: a positioned menu with `{ label, run, disabled? }` items and separators, closed on click outside or Escape; items get `data-testid="ctx-<id>"`.
- `LayersList` rows: `data-testid="layer-row"`, `data-layer-id`, `data-depth`, `aria-selected`; a disclosure button `data-testid="collapse-<id>"` for folders; a visibility checkbox; a pixels chip `data-testid="target-pixels-<id>"` and, when masked, a mask chip `data-testid="target-mask-<id>"` (chip `aria-pressed` reflects `maskSelected`); a clipping arrow glyph (ASCII `>`) when `maskSourceId` is set; rows are `draggable`; the row under a drag shows `data-drop-zone="above" | "below" | "into"`. Footer buttons: `layer-add`, `layer-add-folder`, `layer-add-mask`, `layer-delete`.

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/layer-rows.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { dropTarget, layerRows } from "../../src/panels/layer-rows";
import type { DocumentState, LayerState } from "../../src/engine/types";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 0, pixelsHeight: 0, pixelsRevision: 1, hasPixels: false, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, path: null, layers });

describe("layer rows", () => {
  // Bottom-to-top array: A, F(group) { B, C }, D
  const d = state([layer("A"), layer("F", { isGroup: true }), layer("B", { parentId: "F" }), layer("C", { parentId: "F" }), layer("D")]);
  it("lists top-first with depth and collapses folders", () => {
    expect(layerRows(d, []).map((r) => `${r.layer.id}:${r.depth}`)).toEqual(["D:0", "F:0", "C:1", "B:1", "A:0"]);
    expect(layerRows(d, ["F"]).map((r) => r.layer.id)).toEqual(["D", "F", "A"]);
    expect(layerRows(d, ["F"])[1].collapsed).toBe(true);
  });
  it("computes drop targets", () => {
    const rows = layerRows(d, []);
    expect(dropTarget(rows, 0, "above")).toEqual({ parent: null, above: "D", atBottom: false });
    expect(dropTarget(rows, 1, "into")).toEqual({ parent: "F", above: null, atBottom: false });
    expect(dropTarget(rows, 2, "below")).toEqual({ parent: "F", above: "B", atBottom: false });
    expect(dropTarget(rows, 3, "below")).toEqual({ parent: "F", above: null, atBottom: true });
    expect(dropTarget(rows, 4, "below")).toEqual({ parent: null, above: null, atBottom: true });
    expect(dropTarget(rows, 1, "below")).toEqual({ parent: null, above: "A", atBottom: false });
  });
});
```

`app/tests/e2e/layers.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu } from "./helpers";

async function fresh(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await clickMenu(page, "File", "new");
  await page.getByRole("button", { name: "Create" }).click();
}
const names = (page: Page) => page.getByTestId("layer-row").allInnerTexts();
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });

test("multi-select, group, collapse, rename and delete", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add").click();
  expect(await names(page)).toEqual(["Layer 3", "Layer 2", "Layer 1"]);
  await page.getByTestId("layer-row").nth(2).click();
  await page.getByTestId("layer-row").nth(0).click({ modifiers: ["Shift"] });
  const selected = await page.evaluate(() => (window as any).__compositor.store.getState().selectedLayerIds.length);
  expect(selected).toBe(3);
  await page.getByTestId("layer-row").nth(1).click({ modifiers: ["Control"] });
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().selectedLayerIds.length)).toBe(2);
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await page.getByTestId("ctx-group").click();
  expect((await names(page))[0]).toContain("Folder 1");
  const d = await state(page);
  const folder = d.layers.find((l: any) => l.isGroup);
  expect(d.layers.filter((l: any) => l.parentId === folder.id).length).toBe(2);
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await names(page)).length).toBe(2);
  await page.getByTestId(`collapse-${folder.id}`).click();
  expect((await names(page)).length).toBe(4);
  await page.getByTestId("layer-row").nth(3).click();
  await page.getByTestId("layer-delete").click();
  expect((await names(page)).length).toBe(3);
});

test("drag reorders and nests, alt-drag copies", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-add-folder").click();
  // Rows: Folder 1, Layer 2, Layer 1. Drag Layer 1 into the folder.
  const rows = page.getByTestId("layer-row");
  await rows.nth(2).dragTo(rows.nth(0), { targetPosition: { x: 60, y: 14 } });
  let d = await state(page);
  const folder = d.layers.find((l: any) => l.isGroup);
  expect(d.layers.find((l: any) => l.name === "Layer 1").parentId).toBe(folder.id);
  // Drag Layer 2 above Layer 1 inside the folder (upper quarter of the row).
  const target = page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" });
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 2" }).dragTo(target, { targetPosition: { x: 60, y: 3 } });
  d = await state(page);
  expect(d.layers.find((l: any) => l.name === "Layer 2").parentId).toBe(folder.id);
  expect(d.layers.findIndex((l: any) => l.name === "Layer 2")).toBeGreaterThan(d.layers.findIndex((l: any) => l.name === "Layer 1"));
  await page.keyboard.down("Alt");
  await page.locator('[data-testid="layer-row"]', { hasText: "Layer 1" }).dragTo(page.locator('[data-testid="layer-row"]', { hasText: "Folder 1" }), { targetPosition: { x: 60, y: 3 } });
  await page.keyboard.up("Alt");
  d = await state(page);
  expect(d.layers.map((l: any) => l.name)).toContain("Layer 1 copy");
  expect(d.layers.find((l: any) => l.name === "Layer 1 copy").parentId).toBeNull();
});

test("layer properties and masks", async ({ page }) => {
  await fresh(page);
  await page.getByLabel("Opacity").fill("40");
  await page.getByLabel("Blend mode").selectOption("Multiply");
  let d = await state(page);
  expect(d.layers[0].opacity).toBeCloseTo(0.4, 5);
  expect(d.layers[0].blendMode).toBe("Multiply");
  await page.getByTestId("layer-add-mask").click();
  d = await state(page);
  expect(d.layers[0].hasMask).toBe(true);
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().maskSelected)).toBe(true);
  await page.getByTestId(`target-pixels-${d.layers[0].id}`).click();
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().maskSelected)).toBe(false);
  await page.getByRole("button", { name: "Disable mask" }).click();
  d = await state(page);
  expect(d.layers[0].maskEnabled).toBe(false);
  await page.getByRole("button", { name: "Unlink mask" }).click();
  expect((await state(page)).layers[0].maskLinked).toBe(false);
  await page.getByRole("button", { name: "Delete mask" }).click();
  expect((await state(page)).layers[0].hasMask).toBe(false);
});

test("clipping via the context menu asks before deleting a source", async ({ page }) => {
  await fresh(page);
  await page.getByTestId("layer-add").click();
  await page.getByTestId("layer-row").nth(0).click({ button: "right" });
  await page.getByTestId("ctx-clip").click();
  let d = await state(page);
  expect(d.layers[1].maskSourceId).toBe(d.layers[0].id);
  await page.getByTestId("layer-row").nth(1).click();
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.getByTestId("layer-delete").click();
  d = await state(page);
  expect(d.layers.length).toBe(1);
  expect(d.layers[0].maskSourceId).toBeNull();
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "multi-select"`
Expected: module not found; `ctx-group` missing.

- [ ] **Step 3: Engine tweak for "bottom of a folder"**

In `engine/src/ops/hierarchy.rs::place_layer`, after removing the layer: `let mut insertion = if at_bottom { match parent { Some(p) => layers.iter().position(|l| l.id == p).map(|i| i + 1).unwrap_or(0), None => 0 } } else { layers.len() };`. Add to `engine/tests/hierarchy.rs` in `nested_groups_place_and_move_out`: after moving `child` to the root bottom, `hierarchy::place_layer(&mut d, child, Some(outer), None, true).unwrap(); assert_eq!(d.index_of(child).unwrap(), d.index_of(outer).unwrap() + 1, "the bottom of a folder is right above the folder record");`.

- [ ] **Step 4: Implement layer-rows.ts and the actions**

`app/src/panels/layer-rows.ts`:
```ts
import type { DocumentState, LayerState } from "../engine/types";

export interface Row { layer: LayerState; depth: number; visible: boolean; collapsed: boolean; }

export function layerRows(state: DocumentState, collapsedIds: string[]): Row[] {
  const children = new Map<string | null, LayerState[]>();
  for (const l of state.layers) { const list = children.get(l.parentId) ?? []; list.push(l); children.set(l.parentId, list); }
  const out: Row[] = [];
  const visit = (parent: string | null, depth: number, visible: boolean) => {
    if (depth > 64) return;
    for (const layer of [...(children.get(parent) ?? [])].reverse()) {
      const effective = visible && layer.visible; const collapsed = collapsedIds.includes(layer.id);
      out.push({ layer, depth, visible: effective, collapsed });
      if (layer.isGroup && !collapsed) visit(layer.id, depth + 1, effective);
    }
  };
  visit(null, 0, true);
  return out;
}

export interface DropTarget { parent: string | null; above: string | null; atBottom: boolean; }
export function dropTarget(rows: Row[], index: number, zone: "above" | "below" | "into"): DropTarget {
  const row = rows[index];
  if (zone === "into" && row.layer.isGroup) return { parent: row.layer.id, above: null, atBottom: false };
  if (zone === "above") return { parent: row.layer.parentId, above: row.layer.id, atBottom: false };
  // Below: the next row in the same parent, skipping this row's descendants.
  for (let j = index + 1; j < rows.length; j++) {
    if (rows[j].depth < row.depth) break;
    if (rows[j].depth === row.depth && rows[j].layer.parentId === row.layer.parentId) return { parent: row.layer.parentId, above: rows[j].layer.id, atBottom: false };
  }
  return { parent: row.layer.parentId, above: null, atBottom: true };
}
```

`app/src/actions/layers.ts` (the store, engine and active document are read from `useEditor.getState()`; every function returns early without a document):
```ts
import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import type { BlendMode } from "../engine/types";
import type { DropTarget } from "../panels/layer-rows";

export const BLEND_MODES: BlendMode[] = ["Normal", "Multiply", "Screen", "Overlay", "Darken", "Lighten", "Difference", "Color Dodge", "Color Burn", "Hue", "Saturation", "Color", "Luminosity"];

function ctx() { const s = useEditor.getState(); const doc = s.activeId ? s.documents[s.activeId] : null; return doc && s.engine ? { s, doc, engine: s.engine, selected: s.selectedLayerIds, active: activeLayer(doc) } : null; }

export function deleteSelected(): void {
  const c = ctx(); if (!c || c.selected.length === 0) return;
  c.s.commitTransform();
  const dependents = c.engine.clipDependents(c.doc.id, c.selected);
  let bake = false;
  if (dependents.length > 0) bake = window.confirm("This layer supplies a clipping mask.\n\nOK bakes the masked look into the dependent layers. Cancel removes the links instead.");
  c.s.run({ type: "DeleteLayers", ids: c.selected, bake });
}
export function duplicateSelected(): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.commitTransform(); c.s.run({ type: "DuplicateLayer", id: c.active.id }); }
export function groupSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "GroupLayers", ids: c.selected }); }
export function mergeSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); if (c.engine.mergeAction(c.doc.id, c.selected)) c.s.run({ type: "MergeLayers", ids: c.selected }); }
export function mergeTitle(): string { const c = ctx(); return (c && c.engine.mergeAction(c.doc.id, c.selected)) || "Merge Down"; }
export function addFolder(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "AddGroup" }); }
export function addMaskToActive(revealing: boolean): void { const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.run({ type: "AddMask", id: c.active.id, revealing }); c.s.setMaskSelected(true); }
export function deleteMaskOfActive(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "DeleteMask", id: c.active.id }); c.s.setMaskSelected(false); }
export function toggleMaskEnabled(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "SetMaskEnabled", id: c.active.id, enabled: !c.active.maskEnabled }); }
export function toggleMaskLink(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.commitTransform(); c.s.run({ type: "SetMaskLinked", id: c.active.id, linked: !c.active.maskLinked }); }
export function invertMaskOfActive(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "InvertMask", id: c.active.id }); }
export function fillMaskOfActive(white: boolean): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "FillMask", id: c.active.id, white }); }
export function blurMaskOfActive(): void {
  const c = ctx(); if (!c?.active?.hasMask) return;
  const text = window.prompt("Blur radius (pixels)", "4"); if (text === null) return;
  const radius = Number(text); if (!Number.isFinite(radius) || radius <= 0) { c.s.setError("Enter a radius greater than 0."); return; }
  c.s.run({ type: "BlurMask", id: c.active.id, radius });
}
export function toggleClippingOfActive(): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.run({ type: "ToggleClipping", id: c.active.id }); }
export function canClipActive(): boolean { const c = ctx(); return !!c?.active && !c.active.isGroup && c.engine.canToggleClipping(c.doc.id, c.active.id); }
export function flipSelected(horizontal: boolean): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "FlipLayers", ids: c.selected, horizontal }); }
export function moveActiveBy(offset: number): void { const c = ctx(); if (!c?.active) return; c.s.run({ type: "MoveLayerBy", id: c.active.id, offset }); }
export function placeDropped(id: string, target: DropTarget, copy: boolean): void {
  const c = ctx(); if (!c) return;
  if (!c.engine.canPlace(c.doc.id, id, target.parent)) return;
  c.s.run(copy ? { type: "DuplicateLayerTo", id, parent: target.parent, above: target.above, atBottom: target.atBottom } : { type: "PlaceLayer", id, parent: target.parent, above: target.above, atBottom: target.atBottom });
}
export function setOpacityOfSelected(opacity: number): void { const c = ctx(); if (!c) return; c.s.run({ type: "SetLayersOpacity", ids: c.selected, opacity }); }
export function setBlendModeOfActive(mode: BlendMode): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.run({ type: "SetLayerBlendMode", id: c.active.id, mode }); }
export function cycleBlendMode(forward: boolean): void {
  const c = ctx(); if (!c?.active || c.active.isGroup) return;
  const i = BLEND_MODES.indexOf(c.active.blendMode);
  setBlendModeOfActive(BLEND_MODES[(i + (forward ? 1 : BLEND_MODES.length - 1)) % BLEND_MODES.length]);
}
```

- [ ] **Step 5: Implement the panel components**

`app/src/panels/ContextMenu.tsx`:
```tsx
import { useEffect } from "react";
export type MenuItem = { id: string; label: string; run(): void; disabled?: boolean } | "separator";
export function ContextMenu({ at, items, onClose }: { at: { x: number; y: number }; items: MenuItem[]; onClose(): void }) {
  useEffect(() => {
    const close = () => onClose(); const key = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("pointerdown", close); window.addEventListener("keydown", key);
    return () => { window.removeEventListener("pointerdown", close); window.removeEventListener("keydown", key); };
  }, [onClose]);
  return (
    <div className="context-menu" style={{ left: at.x, top: at.y }} onPointerDown={(e) => e.stopPropagation()} role="menu">
      {items.map((it, i) => it === "separator" ? <hr key={i} /> : <button key={it.id} role="menuitem" data-testid={`ctx-${it.id}`} disabled={it.disabled} onClick={() => { onClose(); it.run(); }}>{it.label}</button>)}
    </div>
  );
}
```

`app/src/panels/LayerProperties.tsx`:
```tsx
import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { activeLayer } from "../state/selection";
import { BLEND_MODES, addMaskToActive, deleteMaskOfActive, invertMaskOfActive, setBlendModeOfActive, setOpacityOfSelected, toggleMaskEnabled, toggleMaskLink } from "../actions/layers";
import type { BlendMode } from "../engine/types";

export function LayerProperties() {
  const doc = useEditor((s) => (s.activeId ? s.documents[s.activeId] : null));
  const layer = doc ? activeLayer(doc) : null;
  const [opacity, setOpacity] = useState(100);
  useEffect(() => { setOpacity(Math.round((layer?.opacity ?? 1) * 100)); }, [layer?.id, layer?.opacity]);
  if (!doc || !layer) return <div className="layer-properties" />;
  const folder = layer.isGroup;
  return (
    <div className="layer-properties">
      <label>Opacity <input aria-label="Opacity" type="number" min={0} max={100} value={opacity} disabled={folder}
        onChange={(e) => { const v = Number(e.target.value); setOpacity(v); if (Number.isFinite(v)) setOpacityOfSelected(Math.min(100, Math.max(0, v)) / 100); }} /> %</label>
      <label>Blend <select aria-label="Blend mode" value={layer.blendMode} disabled={folder} onChange={(e) => setBlendModeOfActive(e.target.value as BlendMode)}>
        {BLEND_MODES.map((m) => <option key={m} value={m}>{m}</option>)}
      </select></label>
      <div className="mask-buttons">
        {!layer.hasMask && <><button onClick={() => addMaskToActive(true)}>Add mask (reveal)</button><button onClick={() => addMaskToActive(false)}>Add mask (hide)</button></>}
        {layer.hasMask && <>
          <button onClick={toggleMaskEnabled}>{layer.maskEnabled ? "Disable mask" : "Enable mask"}</button>
          <button onClick={toggleMaskLink}>{layer.maskLinked ? "Unlink mask" : "Link mask"}</button>
          <button onClick={invertMaskOfActive}>Invert mask</button>
          <button onClick={deleteMaskOfActive}>Delete mask</button>
        </>}
      </div>
    </div>
  );
}
```
A number input rather than a range slider keeps the e2e `fill("40")` deterministic; style it as a compact field next to a `<input type="range">` bound to the same value if desired (the range must also call `setOpacityOfSelected` only on `change`, not on every `input` event, so a drag is one undo step).

`app/src/panels/LayersList.tsx` (rewrite):
```tsx
import { useEffect, useState, type DragEvent } from "react";
import { useEditor } from "../state/store";
import { layerRows, dropTarget, type Row } from "./layer-rows";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, flipSelected, groupSelected, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive } from "../actions/layers";
import { isEditableTarget } from "../shortcuts/target";

type Zone = "above" | "below" | "into";
function zoneFor(e: DragEvent, row: Row): Zone {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect(); const y = (e.clientY - r.top) / r.height;
  if (row.layer.isGroup && y > 0.25 && y < 0.75) return "into";
  return y < 0.5 ? "above" : "below";
}

export function LayersList() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const collapsed = s.activeId ? s.collapsed[s.activeId] ?? [] : [];
  const [renaming, setRenaming] = useState<{ id: string; name: string } | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [drag, setDrag] = useState<{ id: string } | null>(null);
  const [over, setOver] = useState<{ index: number; zone: Zone } | null>(null);
  useEffect(() => {
    const key = (e: KeyboardEvent) => { if ((e.key === "Delete" || e.key === "Backspace") && doc?.activeLayerId && !isEditableTarget(e.target) && !s.sheet) deleteSelected(); };
    window.addEventListener("keydown", key); return () => window.removeEventListener("keydown", key);
  }, [doc?.activeLayerId, s.sheet]);
  if (!doc) return <div className="layers" />;
  const rows = layerRows(doc, collapsed);
  const select = (e: React.MouseEvent, row: Row) => {
    const id = row.layer.id;
    if (e.shiftKey && doc.activeLayerId) {
      const a = rows.findIndex((r) => r.layer.id === doc.activeLayerId), b = rows.findIndex((r) => r.layer.id === id);
      const [lo, hi] = a < b ? [a, b] : [b, a];
      s.selectLayers(rows.slice(lo, hi + 1).map((r) => r.layer.id), doc.activeLayerId);
    } else if (e.ctrlKey || e.metaKey) {
      const next = s.selectedLayerIds.includes(id) ? s.selectedLayerIds.filter((x) => x !== id) : [...s.selectedLayerIds, id];
      s.selectLayers(next, next.includes(id) ? id : next[0] ?? null);
    } else if (!s.selectedLayerIds.includes(id) || s.selectedLayerIds.length !== 1) s.selectLayers([id], id);
  };
  const menuItems = (): MenuItem[] => [
    { id: "duplicate", label: "Duplicate Layer", run: duplicateSelected },
    { id: "group", label: "Group Layers", run: groupSelected },
    { id: "merge", label: mergeTitle(), run: mergeSelected },
    "separator",
    { id: "mask-reveal", label: "Add Mask (Reveal All)", run: () => addMaskToActive(true) },
    { id: "mask-hide", label: "Add Mask (Hide All)", run: () => addMaskToActive(false) },
    { id: "clip", label: "Create/Release Clipping Mask", run: toggleClippingOfActive, disabled: !canClipActive() },
    "separator",
    { id: "flip-h", label: "Flip Horizontal", run: () => flipSelected(true) },
    { id: "flip-v", label: "Flip Vertical", run: () => flipSelected(false) },
    "separator",
    { id: "delete", label: "Delete", run: deleteSelected },
  ];
  return (
    <div className="layers">
      <div className="layers-header"><span>Layers</span></div>
      <div className="layer-rows" onDragLeave={() => setOver(null)}>
        {rows.map((row, index) => {
          const l = row.layer; const selected = s.selectedLayerIds.includes(l.id);
          return (
            <div key={l.id} data-testid="layer-row" data-layer-id={l.id} data-depth={row.depth} aria-selected={selected}
              data-drop-zone={over?.index === index ? over.zone : undefined}
              className={"layer-row" + (selected ? " selected" : "") + (l.id === doc.activeLayerId ? " active" : "") + (row.visible ? "" : " dimmed")}
              style={{ paddingLeft: 8 + row.depth * 14 }} draggable
              onClick={(e) => select(e, row)}
              onDoubleClick={() => setRenaming({ id: l.id, name: l.name })}
              onContextMenu={(e) => { e.preventDefault(); if (!selected) s.selectLayers([l.id], l.id); setMenu({ x: e.clientX, y: e.clientY }); }}
              onDragStart={(e) => { setDrag({ id: l.id }); e.dataTransfer.setData("text/plain", l.id); e.dataTransfer.effectAllowed = "copyMove"; }}
              onDragOver={(e) => { if (!drag || drag.id === l.id) return; e.preventDefault(); setOver({ index, zone: zoneFor(e, row) }); }}
              onDrop={(e) => { e.preventDefault(); if (!drag) return; const zone = zoneFor(e, row); placeDropped(drag.id, dropTarget(rows, index, zone), e.altKey); setDrag(null); setOver(null); }}
              onDragEnd={() => { setDrag(null); setOver(null); }}>
              {l.isGroup ? <button data-testid={`collapse-${l.id}`} className="disclosure" onClick={(e) => { e.stopPropagation(); s.toggleCollapsed(l.id); }}>{row.collapsed ? ">" : "v"}</button> : <span className="disclosure-space" />}
              <input type="checkbox" aria-label={`Visible ${l.name}`} checked={l.visible} onClick={(e) => e.stopPropagation()} onChange={(e) => s.run({ type: "SetLayerVisible", id: l.id, visible: e.target.checked })} />
              {l.maskSourceId && <span className="clip-arrow" title="Clipped to the layer below">{">"}</span>}
              <button data-testid={`target-pixels-${l.id}`} className="chip" aria-pressed={l.id === doc.activeLayerId && !s.maskSelected} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); s.setMaskSelected(false); }}>{l.isGroup ? "F" : "P"}</button>
              {l.hasMask && <button data-testid={`target-mask-${l.id}`} className={"chip" + (l.maskEnabled ? "" : " disabled")} aria-pressed={l.id === doc.activeLayerId && s.maskSelected} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); s.setMaskSelected(true); }}>M</button>}
              {renaming?.id === l.id ? (
                <input autoFocus value={renaming.name} onClick={(e) => e.stopPropagation()} onChange={(e) => setRenaming({ id: l.id, name: e.target.value })}
                  onBlur={() => { if (renaming.name.trim()) s.run({ type: "RenameLayer", id: l.id, name: renaming.name.trim() }); setRenaming(null); }}
                  onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); if (e.key === "Escape") setRenaming(null); }} />
              ) : <span className="layer-name">{l.name}</span>}
            </div>
          );
        })}
      </div>
      <div className="layers-footer">
        <button data-testid="layer-add" title="New layer" onClick={() => s.run({ type: "AddBlankLayer" })}>+</button>
        <button data-testid="layer-add-folder" title="New folder" onClick={addFolder}>[ ]</button>
        <button data-testid="layer-add-mask" title="Add mask" onClick={() => addMaskToActive(true)}>M</button>
        <button data-testid="layer-delete" title="Delete" onClick={deleteSelected}>x</button>
      </div>
      {menu && <ContextMenu at={menu} items={menuItems()} onClose={() => setMenu(null)} />}
    </div>
  );
}
```
Store additions used here: `collapsed: Record<string, string[]>` and `toggleCollapsed(id)` (when collapsing a folder whose descendant is active, select the folder first, as macOS does). Styles: `.layer-row[data-drop-zone="above"]` a 2 px top border in the accent colour, `"below"` bottom border, `"into"` a highlighted background; `.context-menu` fixed, dark, 1 px border; `.chip[aria-pressed="true"]` outlined.

Mount `<LayerProperties />` above `<LayersList />` inside the right column in `App.tsx`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p compositor-engine --test hierarchy && pnpm test && pnpm build && pnpm e2e`
Expected: all pass, including `layers.spec.ts` (4 tests). If Playwright's `dragTo` does not fire React's `onDragOver` in headless Chromium, replace `dragTo` in the test with a manual sequence (`hover` the source, `mouse.down`, `hover` the target position twice, `mouse.up`) and keep the assertions.

- [ ] **Step 7: Commit**

```
git add app engine
git commit -m "feat(app): layers panel with multi-select, folders, drag and drop, masks, properties and context menu"
```

---

### Task 13: Move tool: drag, handles, rotate, distort, snapping, duplicate, inspector

**Files:**
- Create: `app/src/tools/transform-session.ts`, `app/src/panels/TransformInspector.tsx`
- Modify: `app/src/canvas/CanvasView.tsx`, `app/src/canvas/overlay.ts`, `app/src/App.tsx` (mount `TransformInspector` in the tool options row when the move tool is active), `app/src/shortcuts/keymap.ts` + `useShortcuts.ts` (arrow nudges, `transform-apply`, `transform-cancel`)
- Test: `app/tests/unit/transform-session.test.ts`, `app/tests/e2e/transform.spec.ts`

**Interfaces:**
- `transform-session.ts`: `TransformSession` created on pointer down with `{ mode: TransformDragMode; startDoc: P; original: LayerTransform; originalCorners: P[] | null; snap: { xs: number[]; ys: number[]; tolerance: number } | null }`; `update(pointDoc: P, mods: { shift: boolean; alt: boolean; ctrl: boolean }): { draft: LayerTransform; corners: P[] | null; guides: { xs: number[]; ys: number[] } }` (move applies `snapOffset` to the draft's upright bounds; resize/rotate through `transformDrag`; distort through `cornersDrag` with the index, or "move" when the whole shape is dragged while corners exist); `startMode(hit: TransformDragMode | null, inside: boolean, ctrl: boolean, hasCorners: boolean): TransformDragMode | null` (Ctrl on a corner handle starts distortion; with corners pending, any handle moves corners and the body moves all four; a press outside the shape still moves the layer, as macOS does); `nudgeDelta(key, shift): { dx, dy } | null`.
- `overlay.ts` gains `transform: { handles: P[]; rotationHandle: P; showsRotation: boolean } | null` and `guides` are already drawn; handles: 7 px white squares with an accent border, the outline through the four corners, a line to the rotation handle and a circle there.
- CanvasView, move tool: on `pointerdown` (button 0, not space): compute `geometry` from the current edited transform (`transformEdit?.draft` and corners, else the active layer's transform / group box / mask placement), `hit = hitOverlay(geometry, viewPoint)`, `inside = containsPoint(edited, docPoint)`; `mode = startMode(...)`; if `mode` is null and `canTransform` is false, ignore; otherwise `beginTransform({ persistent: !!transformEdit?.corners, duplicate: e.altKey && mode.kind === "move" && !transformsAsGroup })` (a pending distortion keeps editing), create the session and capture the pointer; `pointermove` calls `session.update` and `previewTransform(draft, corners)` plus `setSnapGuides`; `pointerup` commits unless corners are pending (then the edit stays until Enter/Escape). The overlay is drawn when the move tool is active and `canTransform`.
- `TransformInspector` (shown with the move tool when `canTransform`): fields X, Y (origin), Scale % (against the pixel size of the active layer, or the group box for groups), angle, and a sampling select; typing a value and pressing Enter (or blurring) applies through `beginTransform({persistent:false})` + `previewTransform` + `commitTransform` in one go; when a distortion is pending shows Apply and Cancel buttons (`data-testid="transform-apply"`, `"transform-cancel"`). Field labels via `aria-label` "X", "Y", "Scale", "Angle", "Sampling".
- Keymap additions: `ArrowLeft/Right/Up/Down` (`nudge-left`, ...; Shift = 10 px), `Enter` = `transform-apply` and `Escape` = `transform-cancel` when a transform edit is pending and the crop tool is not active (the existing crop bindings stay: `runAction` decides by tool and state).

- [ ] **Step 1: Write the failing tests**

`app/tests/unit/transform-session.test.ts`:
```ts
import { describe, expect, it } from "vitest";
import { TransformSession, nudgeDelta, startMode } from "../../src/tools/transform-session";
import type { LayerTransform } from "../../src/engine/types";

const t = (x: number, y: number, w: number, h: number): LayerTransform => ({ origin: [x, y], size: [w, h], rotation: 0, flipX: false, flipY: false, sampling: "High quality" });

describe("transform session", () => {
  it("moves with snapping and reports guides", () => {
    const s = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 50, y: 50 }, original: t(40, 40, 20, 20), originalCorners: null, snap: { xs: [0, 100, 200], ys: [0, 100], tolerance: 4 } });
    const r = s.update({ x: 57, y: 53 }, { shift: false, alt: false, ctrl: false });
    // Unsnapped origin would be (47, 43): its right edge 67 and centre 57 are far from targets; left 47 is 3 from... no target at 50, so no x snap; top 43 is 3 from... no y target; nothing snaps.
    expect(r.draft.origin).toEqual([47, 43]);
    const s2 = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 50, y: 50 }, original: t(40, 40, 20, 20), originalCorners: null, snap: { xs: [83], ys: [0], tolerance: 4 } });
    const r2 = s2.update({ x: 71, y: 50 }, { shift: false, alt: false, ctrl: false });
    expect(r2.draft.origin[0]).toBe(63); // right edge 81 -> 83
    expect(r2.guides.xs).toEqual([83]);
  });
  it("shift constrains a move and distort drags corners", () => {
    const s = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 0, y: 0 }, original: t(0, 0, 10, 10), originalCorners: null, snap: null });
    expect(s.update({ x: 10, y: 3 }, { shift: true, alt: false, ctrl: false }).draft.origin).toEqual([10, 0]);
    const corners = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }, { x: 0, y: 10 }];
    const d = new TransformSession({ mode: { kind: "distort", index: 4 }, startDoc: { x: 10, y: 10 }, original: t(0, 0, 10, 10), originalCorners: corners, snap: null });
    const r = d.update({ x: 14, y: 12 }, { shift: false, alt: false, ctrl: false });
    expect(r.corners![2]).toEqual({ x: 14, y: 12 });
    expect(r.corners![0]).toEqual({ x: 0, y: 0 });
  });
  it("chooses the start mode", () => {
    expect(startMode({ kind: "resize", index: 0 }, true, true, false)).toEqual({ kind: "distort", index: 0 });
    expect(startMode({ kind: "resize", index: 1 }, true, true, false)).toEqual({ kind: "resize", index: 1 });
    expect(startMode({ kind: "resize", index: 3 }, true, false, true)).toEqual({ kind: "distort", index: 3 });
    expect(startMode(null, true, false, true)).toEqual({ kind: "distort", index: -1 });
    expect(startMode(null, false, false, false)).toEqual({ kind: "move" });
    expect(startMode({ kind: "rotate" }, false, false, false)).toEqual({ kind: "rotate" });
    expect(nudgeDelta("ArrowLeft", true)).toEqual({ dx: -10, dy: 0 });
    expect(nudgeDelta("x", false)).toBeNull();
  });
});
```

`app/tests/e2e/transform.spec.ts`:
```ts
import { test, expect, type Page } from "@playwright/test";
import { redSquarePngBase64 } from "./helpers";

async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async (b64) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(400, 300, false);
    api.engine.importImage(doc, png, "red", { x: 200, y: 150 });
    const id = api.engine.state(doc).activeLayerId;
    const t = api.engine.state(doc).layers[0].transform;
    api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [150, 100], size: [100, 100] } });
    api.store.getState().openDocument(doc);
    api.store.getState().setTool("move");
    await api.setZoom(1);
  }, redSquarePngBase64());
}
const layer = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers[0]; });
const viewPoint = (page: Page, x: number, y: number) => page.evaluate(({ x, y }) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const p = vp.viewPoint({ x, y }, { width: d.width, height: d.height }); const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + p.x, y: r.top + p.y };
}, { x, y });
async function drag(page: Page, from: { x: number; y: number }, to: { x: number; y: number }, mods: string[] = []) {
  const a = await viewPoint(page, from.x, from.y), b = await viewPoint(page, to.x, to.y);
  for (const m of mods) await page.keyboard.down(m);
  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move((a.x + b.x) / 2, (a.y + b.y) / 2); await page.mouse.move(b.x, b.y); await page.mouse.up();
  for (const m of mods) await page.keyboard.up(m);
}

test("dragging moves the layer, even from outside it, with snapping to the canvas edge", async ({ page }) => {
  await setup(page);
  await drag(page, { x: 20, y: 20 }, { x: 40, y: 30 });
  expect((await layer(page)).transform.origin).toEqual([170, 110]);
  await drag(page, { x: 200, y: 150 }, { x: 32, y: 150 }); // origin would be 2: snaps to 0
  expect((await layer(page)).transform.origin[0]).toBe(0);
});

test("handles resize and rotate; alt-drag duplicates; arrows nudge", async ({ page }) => {
  await setup(page);
  await drag(page, { x: 250, y: 200 }, { x: 300, y: 250 }); // bottom-right handle
  let l = await layer(page);
  expect(l.transform.size).toEqual([150, 150]);
  await drag(page, { x: 225, y: 72 }, { x: 350, y: 175 }, ["Shift"]); // rotation handle 28px above the top middle
  l = await layer(page);
  expect(l.transform.rotation % 15).toBe(0);
  expect(l.transform.rotation).not.toBe(0);
  await page.keyboard.press("ArrowRight");
  expect((await layer(page)).transform.origin[0]).toBe(l.transform.origin[0] + 1);
  await page.keyboard.press("Shift+ArrowDown");
  expect((await layer(page)).transform.origin[1]).toBe(l.transform.origin[1] + 10);
  await drag(page, { x: 20, y: 20 }, { x: 60, y: 20 }, ["Alt"]);
  const d = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
  expect(d.layers.length).toBe(2);
  expect(d.layers[1].name).toBe("red copy");
});

test("ctrl-drag on a corner distorts; Enter applies; the inspector edits values", async ({ page }) => {
  await setup(page);
  await drag(page, { x: 250, y: 100 }, { x: 300, y: 100 }, ["Control"]); // top-right corner pulled right
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().transformEdit?.corners?.[1])).toEqual([300, 100]);
  await expect(page.getByTestId("transform-apply")).toBeVisible();
  await page.keyboard.press("Enter");
  let l = await layer(page);
  expect(l.transform.rotation).toBe(0);
  expect(l.pixelsRevision).toBe(2);
  expect(l.transform.size[0]).toBeGreaterThan(100);
  await page.getByLabel("X").fill("10");
  await page.keyboard.press("Enter");
  l = await layer(page);
  expect(l.transform.origin[0]).toBe(10);
  await page.getByLabel("Angle").fill("45");
  await page.keyboard.press("Enter");
  expect((await layer(page)).transform.rotation).toBe(45);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "dragging moves"`
Expected: module not found; the drag does nothing (origin unchanged).

- [ ] **Step 3: Implement transform-session.ts**

`app/src/tools/transform-session.ts`:
```ts
import type { LayerTransform } from "../engine/types";
import { boundsOf, cornersDrag, snapOffset, transformDrag, type P, type TransformDragMode } from "./transform-geometry";

export interface SnapTargetsWithTolerance { xs: number[]; ys: number[]; tolerance: number; }
export interface SessionInit { mode: TransformDragMode; startDoc: P; original: LayerTransform; originalCorners: P[] | null; snap: SnapTargetsWithTolerance | null; }
export interface SessionResult { draft: LayerTransform; corners: P[] | null; guides: { xs: number[]; ys: number[] }; }

export class TransformSession {
  constructor(private readonly init: SessionInit) {}
  update(point: P, mods: { shift: boolean; alt: boolean; ctrl: boolean }): SessionResult {
    const { mode, startDoc, original, originalCorners, snap } = this.init;
    const guides = { xs: [] as number[], ys: [] as number[] };
    if (mode.kind === "distort" && originalCorners) {
      const corners = cornersDrag(originalCorners, startDoc, mode.index < 0 ? "move" : mode.index).updated(point, mods.shift);
      return { draft: original, corners, guides };
    }
    let draft = transformDrag(original, startDoc, mode).updated(point, { lockRatio: mode.kind === "resize", shift: mods.shift, alt: mods.alt });
    if (mode.kind === "move" && snap) {
      const s = snapOffset(boundsOf(draft), snap.xs, snap.ys, snap.tolerance);
      if (s.dx !== 0 || s.dy !== 0) draft = { ...draft, origin: [draft.origin[0] + s.dx, draft.origin[1] + s.dy] };
      if (s.x !== null) guides.xs.push(s.x); if (s.y !== null) guides.ys.push(s.y);
    }
    return { draft, corners: originalCorners, guides };
  }
}

/** What a press starts. Ctrl on a corner handle begins a distortion; with corners pending every handle moves corners. */
export function startMode(hit: TransformDragMode | null, inside: boolean, ctrl: boolean, hasCorners: boolean): TransformDragMode {
  if (hasCorners) { if (hit && hit.kind === "resize") return { kind: "distort", index: hit.index }; return { kind: "distort", index: -1 }; }
  if (hit && hit.kind === "resize" && ctrl && hit.index % 2 === 0) return { kind: "distort", index: hit.index };
  if (hit) return hit;
  const _ = inside; // a press anywhere moves the active layer, as macOS does
  return { kind: "move" };
}

export function nudgeDelta(key: string, shift: boolean): { dx: number; dy: number } | null {
  const step = shift ? 10 : 1;
  switch (key) { case "ArrowLeft": return { dx: -step, dy: 0 }; case "ArrowRight": return { dx: step, dy: 0 }; case "ArrowUp": return { dx: 0, dy: -step }; case "ArrowDown": return { dx: 0, dy: step }; default: return null; }
}
```
Distortion with `index % 2 === 1` (an edge handle) under Ctrl stays a resize, matching macOS where only corners start a distortion; once corners exist, edge handles move both corners of that edge via `cornersDrag`.

- [ ] **Step 4: Wire the canvas, overlay, inspector and shortcuts**

CanvasView (move tool effect), in outline with the exact decision points:
```ts
useEffect(() => {
  const el = glRef.current?.parentElement; if (!el) return;
  let session: TransformSession | null = null;
  const docPoint = (e: PointerEvent) => { const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!]; const r = el.getBoundingClientRect(); return vp.documentPoint({ x: e.clientX - r.left, y: e.clientY - r.top }, { width: d.width, height: d.height }); };
  const down = (e: PointerEvent) => {
    const s = useEditor.getState();
    if (s.tool !== "move" || e.button !== 0 || spaceRef.current || !s.activeId) return;
    const d = s.documents[s.activeId]; const vp = s.viewports[s.activeId];
    if (!canTransform(d, s.selectedLayerIds, s.maskSelected) && !s.transformEdit) return;
    const edited = editedShape(s, d); // { transform, corners } from transformEdit or the selection (see below)
    const geometry = overlayGeometry(edited.corners ?? edited.transform, vp, { width: d.width, height: d.height });
    const r = el.getBoundingClientRect(); const view = { x: e.clientX - r.left, y: e.clientY - r.top };
    const mode = startMode(hitOverlay(geometry, view), containsPoint(edited.transform, docPoint(e)), e.ctrlKey, !!edited.corners);
    if (!s.transformEdit && !s.beginTransform({ persistent: false, duplicate: e.altKey && mode.kind === "move" && !transformsAsGroup(d, s.selectedLayerIds) })) return;
    const te = useEditor.getState().transformEdit!;
    if (mode.kind === "distort" && !te.corners) useEditor.getState().beginDistort();
    const after = useEditor.getState().transformEdit!;
    const tolerance = 10 / vp.pointsPerPixel; const targets = snapTargets(d, after.ids);
    session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners?.map(fromTuple) ?? null, snap: mode.kind === "move" ? { ...targets, tolerance } : null });
    el.setPointerCapture(e.pointerId);
  };
  const move = (e: PointerEvent) => { if (!session) return; const r = session.update(docPoint(e), { shift: e.shiftKey, alt: e.altKey, ctrl: e.ctrlKey }); const s = useEditor.getState(); s.previewTransform(r.draft, r.corners ? cornersToTuples(r.corners) : null); s.setSnapGuides(r.guides); };
  const up = () => { if (!session) return; session = null; const s = useEditor.getState(); s.setSnapGuides({ xs: [], ys: [] }); if (!s.transformEdit?.corners) s.commitTransform(); else s.invalidate(); };
  el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
}, []);
```
`editedShape(s, d)`: with a `transformEdit`, `{ transform: draft, corners }`; else for a group selection the `groupBox`; else the active layer's transform, or its mask placement when `maskSelected` and the mask is unlinked. Export `editedShape` from `state/selection.ts` (it needs the store's `transformEdit`, `selectedLayerIds`, `maskSelected`: pass them as arguments). The draw effect passes `transform: overlayGeometry(...)` to `drawOverlay` when `tool === "move"` and (`transformEdit` or `canTransform`), and `guides: s.snapGuides` (drop `guidesRef`). Cancel a pending transform when the tool changes (`setTool` already commits; for a pending distortion, commit as well: Photoshop applies on tool switch).

`TransformInspector`:
```tsx
export function TransformInspector() {
  const s = useEditor(); const doc = s.activeId ? s.documents[s.activeId] : null;
  if (!doc || s.tool !== "move") return null;
  const shape = editedShape(s, doc); if (!shape) return null;
  const pixel = pixelSizeFor(s, doc); // active layer's pixel size, or the box size for a group, or null for a mask
  const apply = (mutate: (t: LayerTransform) => LayerTransform) => {
    const st = useEditor.getState();
    const had = !!st.transformEdit;
    if (!had && !st.beginTransform({ persistent: false })) return;
    const draft = mutate(useEditor.getState().transformEdit!.draft);
    st.previewTransform(draft, useEditor.getState().transformEdit!.corners);
    if (!had) st.commitTransform();
  };
  const field = (label: string, value: number, set: (v: number, t: LayerTransform) => LayerTransform) => (
    <NumberField key={label} label={label} value={value} onCommit={(v) => apply((t) => set(v, t))} />
  );
  const t = shape.transform;
  return (
    <div className="tool-options" data-testid="transform-inspector">
      <span>{s.maskSelected && s.transformEdit?.kind === "mask" ? "Transform Mask" : "Transform"}</span>
      {field("X", t.origin[0], (v, t) => ({ ...t, origin: [v, t.origin[1]] }))}
      {field("Y", t.origin[1], (v, t) => ({ ...t, origin: [t.origin[0], v] }))}
      {pixel && field("Scale", scalePercent(t, pixel), (v, t) => scaledToPercent(t, v, pixel))}
      {field("Angle", t.rotation, (v, t) => ({ ...t, rotation: v % 360 }))}
      <select aria-label="Sampling" value={t.sampling} onChange={(e) => apply((t) => ({ ...t, sampling: e.target.value as Sampling }))}><option>Nearest</option><option>Smooth</option><option>High quality</option></select>
      {s.transformEdit?.corners && <><button data-testid="transform-cancel" onClick={s.cancelTransform}>Cancel</button><button data-testid="transform-apply" className="primary" onClick={s.commitTransform}>Apply</button></>}
    </div>
  );
}
```
`NumberField` keeps local text state, commits on Enter or blur when the parsed number is finite and differs from the last committed value. Mount `<TransformInspector />` in the same options row as `CropOptions` in `App.tsx`.

Keymap: add `"nudge-left" | "nudge-right" | "nudge-up" | "nudge-down"` (`ArrowLeft` etc. with and without Shift; give the Shift variants `shift: true`), and in `runAction`: nudges call `s.run({ type: "NudgeLayers", ids: selected, dx, dy })` via `nudgeDelta` when the move tool is active (with a pending edit, nudge the draft instead: `previewTransform` with the origin moved and corners shifted); `crop-apply`/`crop-cancel` handling stays; when the crop tool is not active and `transformEdit` exists, `Enter` commits and `Escape` cancels (map both keys in the keymap to actions `apply` and `cancel`, and let `runAction` route by tool: crop tool -> crop, otherwise transform; rename the existing ids accordingly and update `keymap.test.ts` expectations).

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm test && pnpm build && pnpm e2e`
Expected: all pass, including `transform.spec.ts` (3 tests). If the rotation-handle drag misses because the handle sits 28 CSS px above the top-middle handle, compute the drag start in the test through `overlayGeometry` exposed on the test API (`api.transformGeometry()`) instead of the hard-coded (225, 72).

- [ ] **Step 6: Commit**

```
git add app
git commit -m "feat(app): move tool with handles, rotation, snapping, alt-duplicate, free distortion and a transform inspector"
```

---

### Task 14: Layer menu, shortcuts and flip

**Files:**
- Modify: `app/src/panels/MenuBar.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`
- Test: `app/tests/unit/keymap.test.ts` (extend), `app/tests/e2e/layers.spec.ts` (extend)

**Interfaces:**
- Menu "Layer" between Edit and Image, items (test ids `menu-<id>`): `layer-new` (New Layer, Ctrl+Shift+N), `layer-new-folder` (New Folder), `layer-duplicate` (Duplicate Layer, Ctrl+J), `layer-group` (Group Layers, Ctrl+G), `layer-merge` (label from `mergeTitle()`, Ctrl+E), separator, `layer-mask-reveal`, `layer-mask-hide`, `layer-mask-delete`, `layer-mask-toggle` (Disable/Enable), `layer-mask-link` (Link/Unlink), `layer-mask-invert`, `layer-mask-fill-white`, `layer-mask-fill-black`, `layer-mask-blur` (Blur/Feather Mask...), separator, `layer-clip` (Create/Release Clipping Mask, Ctrl+Alt+G), separator, `layer-flip-h`, `layer-flip-v` (Flip Layer Horizontal/Vertical), `layer-up` (Bring Forward, Ctrl+]), `layer-down` (Send Backward, Ctrl+[), separator, `layer-delete` (Delete Layer, Delete).
- Keymap additions: `new-layer` exists; add `new-folder` (none), `duplicate` Ctrl+J, `group` Ctrl+G, `merge` Ctrl+E, `clip` Ctrl+Alt+G, `layer-up` Ctrl+], `layer-down` Ctrl+[, `blend-next` Shift+= (also Shift++), `blend-prev` Shift+- (also Shift+_), `opacity-0..9` digit keys 0..9 without modifiers while the move tool is active (1 = 10% ... 9 = 90%, 0 = 100%; two digits within 600 ms combine, e.g. 2 then 5 = 25%), `delete-layer` Delete/Backspace (replacing the LayersList window listener from Task 12, which is removed so the key is handled in exactly one place).
- `runAction` gains the corresponding arms, calling the functions in `actions/layers.ts`; digit handling keeps a small module-level `{ digit, at }` buffer.

- [ ] **Step 1: Write the failing tests**

Add to `app/tests/unit/keymap.test.ts`:
```ts
  it("maps layer shortcuts", () => {
    expect(matchShortcut(ev("j", { ctrlKey: true }))).toBe("duplicate");
    expect(matchShortcut(ev("g", { ctrlKey: true }))).toBe("group");
    expect(matchShortcut(ev("g", { ctrlKey: true, altKey: true }))).toBe("clip");
    expect(matchShortcut(ev("e", { ctrlKey: true }))).toBe("merge");
    expect(matchShortcut(ev("]", { ctrlKey: true }))).toBe("layer-up");
    expect(matchShortcut(ev("+", { shiftKey: true }))).toBe("blend-next");
    expect(matchShortcut(ev("_", { shiftKey: true }))).toBe("blend-prev");
    expect(matchShortcut(ev("5"))).toBe("opacity-5");
    expect(matchShortcut(ev("Delete"))).toBe("delete-layer");
  });
```
Add to `app/tests/e2e/layers.spec.ts`:
```ts
test("layer menu and shortcuts: duplicate, blend cycling, opacity digits, merge, flip", async ({ page }) => {
  await fresh(page);
  await page.keyboard.press("Control+j");
  expect((await names(page)).length).toBe(2);
  await page.keyboard.press("Shift+=");
  expect((await state(page)).layers[1].blendMode).toBe("Multiply");
  await page.keyboard.press("Shift+-");
  await page.keyboard.press("Shift+-");
  expect((await state(page)).layers[1].blendMode).toBe("Luminosity");
  await page.getByTestId("tool-move").click();
  await page.keyboard.press("5");
  expect((await state(page)).layers[1].opacity).toBeCloseTo(0.5, 5);
  await page.keyboard.press("2"); await page.keyboard.press("5");
  expect((await state(page)).layers[1].opacity).toBeCloseTo(0.25, 5);
  await clickMenu(page, "Layer", "layer-flip-h");
  expect((await state(page)).layers[1].transform.flipX).toBe(true);
  await page.keyboard.press("Control+e");
  expect((await names(page)).length).toBe(1);
  await page.keyboard.press("Control+z");
  expect((await names(page)).length).toBe(2);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm test` and `pnpm e2e --grep "layer menu"`
Expected: keymap assertions fail; Ctrl+J does nothing.

- [ ] **Step 3: Implement**

`keymap.ts`: extend `ActionId` and `SHORTCUTS`:
```ts
  "new-folder": [], "duplicate": [{ key: "j", ctrl: true }], "group": [{ key: "g", ctrl: true }], "merge": [{ key: "e", ctrl: true }],
  "clip": [{ key: "g", ctrl: true, alt: true }], "layer-up": [{ key: "]", ctrl: true }], "layer-down": [{ key: "[", ctrl: true }],
  "blend-next": [{ key: "=", shift: true }, { key: "+", shift: true }], "blend-prev": [{ key: "-", shift: true }, { key: "_", shift: true }],
  "delete-layer": [{ key: "Delete" }, { key: "Backspace" }],
  ...Object.fromEntries(Array.from({ length: 10 }, (_, i) => [`opacity-${i}`, [{ key: String(i) }]])),
```
(`ActionId` becomes a template-literal-friendly union: add `` `opacity-${number}` `` or list `opacity-0` ... `opacity-9` explicitly.)

`useShortcuts.ts` `runAction` additions:
```ts
    case "new-folder": addFolder(); break;
    case "duplicate": duplicateSelected(); break;
    case "group": groupSelected(); break;
    case "merge": mergeSelected(); break;
    case "clip": toggleClippingOfActive(); break;
    case "layer-up": moveActiveBy(1); break;
    case "layer-down": moveActiveBy(-1); break;
    case "blend-next": cycleBlendMode(true); break;
    case "blend-prev": cycleBlendMode(false); break;
    case "delete-layer": if (doc && !s.sheet) deleteSelected(); break;
    default:
      if (id.startsWith("opacity-")) { if (s.tool === "move" && doc) typeOpacityDigit(Number(id.slice(8))); }
```
with
```ts
let digitBuffer: { digit: number; at: number } | null = null;
export function typeOpacityDigit(digit: number, now = Date.now()): void {
  let percent: number;
  if (digitBuffer && now - digitBuffer.at < 600) { percent = digitBuffer.digit * 10 + digit; digitBuffer = null; }
  else { percent = digit === 0 ? 100 : digit * 10; digitBuffer = { digit, at: now }; }
  setOpacityOfSelected(Math.min(100, percent) / 100);
}
```
Note: the first digit applies immediately (5 -> 50%); a second digit within 600 ms replaces it (2 then 5 -> 25%), matching the macOS `typeOpacityDigit`. Add a unit test for `typeOpacityDigit` timing in `keymap.test.ts` by injecting `now` (export the function; a store stub is unnecessary if `setOpacityOfSelected` is injectable: give `typeOpacityDigit` an optional third parameter `apply = setOpacityOfSelected`).

`MenuBar.tsx`: add the Layer menu with the items above; mask items are disabled when the active layer has no mask (or has one, for the add items); `layer-merge`'s label is `mergeTitle()`; the `Delete` listener in `LayersList` is removed.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `pnpm test && pnpm build && pnpm e2e`
Expected: all pass.

- [ ] **Step 5: Commit**

```
git add app
git commit -m "feat(app): Layer menu, layer shortcuts, blend cycling and opacity digits"
```

---

### Task 15: Docs, spec fix, portable build

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-09-20-windows-port-design.md` (section 2: "versions 1 to 6" becomes "versions 1 to 7"), `package.json` (version 0.2.0), `src-tauri/tauri.conf.json` (version 0.2.0), `src-tauri/Cargo.toml` and workspace version 0.2.0

- [ ] **Step 1: Update the docs**

README: add a "Phase 2: layers" feature list (folders, opacity, 13 blend modes, layer and folder masks with invert/fill/blur/link, clipping masks, move/scale/rotate/flip/free distort with snapping, multi-layer and folder transforms, merge, duplicate, drag reorder and nest), the Layer menu shortcuts, and the note that mask painting arrives with the brush in Phase 4. Fix the spec's section 2 wording (controller ruling: the code, section 5 and the macOS source all say 1 to 7). Bump the version to 0.2.0 in the four places above (`cargo build` must still succeed; `Cargo.lock` updates).

- [ ] **Step 2: Full verification and portable build**

```
cargo test
pnpm test
pnpm build
pnpm e2e
pnpm build:portable
```
Expected: every suite green; a zip `build-artifacts/windows-x64/Compositor-portable-0.2.0-<stamp>.zip` with the marker verified and "Smoke launch OK". Then unzip to a fresh folder and launch; a human must still exercise: import two images, move and rotate one with the handles, Ctrl-drag a corner and Apply, add a mask and invert it, group the two layers, drag one into and out of the folder, set Multiply, Ctrl+E to merge, save, reopen.

- [ ] **Step 3: Commit**

```
git add README.md docs package.json src-tauri Cargo.toml Cargo.lock
git commit -m "docs,build: phase 2 documentation, spec version fix, 0.2.0 portable build"
```

---

## Self-review notes

Spec coverage, section 3 Phase 2: layer stack with folders (Tasks 5, 12), visibility (Phase 1), opacity and blend modes (1, 4, 5, 11, 12), raster layer masks and folder masks with fill, invert, blur/feather, link/unlink (7, 12, 14; painting deferred to Phase 4 by ruling), clipping masks (3, 4, 5, 11, 12), non-destructive move/scale/rotate/flip/free distort with snapping and guides (6, 10, 13), exact transform values (13), transforming several layers or a folder together (6, 10, 13), merge down/layers/group (8, 14), duplicate (5, 13, 14), rename (Phase 1), reorder and nest by drag (5, 12), undo/redo (engine snapshots, unchanged).

Cross-task consistency: `Coverage`/`LayerDraw`/`PlanNode` shapes are identical in `plan.rs`, `types.ts` and the GL renderer (`kind: "layer" | "stack"`, `folderCoverages`, `corners` as tuples); `PreviewEdit.box` is the JSON name for Rust `bounds` in both `PreviewEdit::Group` and `Command::TransformLayers`/`DistortLayers`; `Mask::follow` is the single placement rule used by `set_transform`, `transform_group`, `flip_layers`, and the plan's `displayed_mask_placement`; `hierarchy::place_layer`'s `at_bottom` inside a folder means the folder's lowest slot (Task 12 tweak) and `layer-rows.ts::dropTarget` relies on it; the LayersList Delete listener from Task 12 is removed again in Task 14 so `delete-layer` has one handler.

Known simplifications: the GL renderer applies at most 3 nested clipping levels (the CPU path handles 256); distortion previews on the CPU path resample per frame without prefiltering; the opacity control is a number field plus optional range; the macOS "auto-select layer on click" option is not ported; deleting picks the layer below (Phase 1 rule) where macOS picks the one above; Alt-drag duplication on the canvas is two undo steps only when cancelled mid-drag (a committed one uses `DuplicateLayerTransformed`: adjust `commitTransform` so a `duplicated` edit sends `DuplicateLayerTransformed` for the source id after undoing the interim `DuplicateLayer`; simpler ruling: keep the interim duplicate and send `SetLayerTransform`, accepting two undo steps, and note it in the README).

Ruling folded into the plan text for `commitTransform` with `duplicated`: keep the interim duplicate and send `SetLayerTransform` (two undo steps). `DuplicateLayerTransformed` therefore stays available for the inspector/menus but is not used by the canvas drag; keep its test.

