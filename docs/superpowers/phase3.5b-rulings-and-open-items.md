# Phase 3.5b: rulings and open items

Phase 3.5b (draw what 3.5a preserved) was executed from
`docs/superpowers/plans/2026-09-24-phase3.5b-draw-blend-modes-and-adjustments.md` by subagent-driven
development on branch `phase3.5b-draw` (faa3831..7aa5b1e, then this file). The oracle is
Compositor 1.2.10. This file keeps the decisions and open items after the SDD workspace is deleted.

## What shipped (0.3.6)

- The eleven blend modes Mac 1.2.6 added, on CPU and GPU; all 24 in the Blend select in the Mac's
  `allCases` order (also the Shift+=/- cycle). The render plan carries `cg_mode` (the eight
  Core-Image-only modes act as Normal for adjustment layers and stack bases) and `keeps_alpha`
  (an adjustment layer whose stored mode is not Normal keeps the backdrop's alpha,
  LiveMaskRenderer.swift:24 vs :40).
- The 1.2.6 Grain detail kernel and the position-based Add Noise hash, for adjustment layers and
  destructive filters, CPU and GPU. Grain and Black & White match the Mac 1.2.10 renders bit for bit.
- Adjustment layers for Invert, Black & White, Color Balance, Add Noise, Gaussian Blur and Motion
  Blur, with panels and Layer > New Adjustment entries in the Mac's order; destructive Image >
  Black & White and Color Balance.
- Blur layers on a padded composite over a halving lattice anchored at the canvas corner; the GPU
  asks the engine (wasm `spatial_blur`, `spatial_grid`, `spatial_span`) for every spatial number.
  A partial render equals the whole render to the bit.
- The 100 MP limit in the user's words, checked from image headers before any decode, on package
  open and on single-image import and paste.
- Whole-number doubles written as the Mac writes them (transforms, resolution, opacity), no `-0.0`.
- The probe generator writes 22 projects for the user's Mac.

## Rulings

Planning:
- Phase 3.5 split into 3.5a (open/save), 3.5b (blend modes, adjustment kinds, kernels) and 3.5c
  (layer effects).
- Oracle is Compositor 1.2.10 (upstream robbietilton 4306206): format 9, no blend, adjustment,
  effect or kernel maths changed from 1.2.6.
- OQ1: destructive Image > Black & White and Color Balance are in scope. OQ2: destructive Filter >
  Gaussian Blur stays exact at every radius. OQ3 (superseded): the halving grid is anchored at the
  canvas on CPU and GPU alike. OQ4: 1.2.10's 200 M text-box area is adopted.
- Pre-flight (35 findings applied): halving bounds are MEASURED, not asserted in advance;
  `keeps_alpha` beside `blend = cg_mode`; Motion Blur layers stay in the undrawn notice ("drawn
  approximately") and keep blocking merge until the motion probe measures the gap to CIMotionBlur.

Execution:
- Task 3: a Gaussian Blur layer drew as identity for one task without notice; accepted as transient
  (Task 4 drew it next; nothing shipped between).
- Task 4: `MOTION_REACH_LIMIT = 12` output px for Motion Blur layers (an exact 90 px streak took 45 s
  to export 6 Mpx; now 2.9 s). Re-measured streak bounds: 2 inside a smooth ramp, 17 at its canvas
  edge, 20 over per-pixel noise.
- Task 4 review: region origins are ROUNDED to the canvas grid (the CPU renderer's arithmetic lands
  one ulp below whole pixels); `SPATIAL_CELL_LIMIT = 256` caps the level at 8 (an unbounded cell made
  67-268 Mpx padded targets at zoom 32).
- Task 5: `decode_image` refuses an over-budget import from its header; the message says "This image
  is larger than Compositor for Windows supports".
- Task 7: `spatial_margin` summed from `-0.0` fixed to fold from 0.
- Task 9: Color Balance and Add Noise got absolute GPU checks (an independent transcription of
  AdjustPixels.c; noise sd from the kernel's scaling).
- Task 10: a step edge on the lattice gives closed-form absolute GPU checks for halved Gaussian and
  Motion Blur (worst 0.84 from the formula).
- Final review I1: the Gaussian's horizontal pass runs in a rolling band of rows; peak heap with a
  Gaussian Blur layer 28 -> 10.5 bytes per pixel (about 0.8 GB at 100 MP), output bit-identical,
  pinned by `engine/tests/peak_heap.rs`.
- Commit attribution: each implementer's Co-Authored-By names its own model, as its instructions say.

## Open items

For Phase 3.5c (layer effects):
- Draw the six layer effects on CPU and GPU, type their JSON (check its shape as the Mac's decode does),
  then lift the merge refusal and the notice entry for effects. Delete-with-bake ignores a clipping
  base's effects today.
- The `edited-rich-file` probe can only be compared once effects are drawn.

From the follow-up probes: settled by the Mac 1.2.10 exports of 2026-09-27 (probe results;
pinned in engine/tests/mac_1_2_10.rs by Phase 4a Tasks 1 and 2):
- Soft Light is Pegtop's formula (the W3C one was 14 levels off at 75 % grey); now within 1.
- Hard Light, Linear Light and Pin Light on non-pure sources and the Hard Mix edge: within 1.
- Color Burn and Color Dodge on adjustment layers and clipped groups: bit-identical to the Mac.
- Motion Blur: CIMotionBlur is a Gaussian along the angle, sigma = distance / sqrt(12); ported, the
  notice entry and the merge refusal dropped; within 6 levels of the Mac's render (26 before).
- Color Balance with preserve on and off, Invert, both noise modes: bit-identical; B&W tint within 1;
  Gaussian 6 and 40 within 2 and 3 premultiplied.

Engineering:
- A level-0 Gaussian still holds the target plus one working copy (8 B/px); a streaming mix in
  `spatial_target` would reach about 4 B/px. A level-0 Motion Blur costs 12 B/px.
- Deferred Minors from the task and final reviews are recorded in the final review; none was
  promoted.
