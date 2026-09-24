# Phase 3.5a: rulings and open items

Phase 3.5a (open and save Mac Compositor 1.2.6 projects without loss) was executed from
`docs/superpowers/plans/2026-09-24-phase3.5a-open-and-save-mac-1.2.6.md` by subagent-driven
development on branch `phase3.5a-mac-compat` (1d977d1..fefc8e3). This file keeps the decisions and
the open items after the SDD workspace is deleted.

## What shipped

- Reads format versions 1-9 and always writes 9, like the Mac 1.2.6. Validation mirrors
  `ProjectStore.validate` rule for rule (guides, v8 folder opacity, v9 blur/noise kinds, live text
  bounds).
- Nothing read is lost on save: layer `effects`, live `text`, `shape`, and unknown keys at the layer and
  manifest levels are carried verbatim. `text` and `shape` are dropped only when a layer's pixels are
  replaced (`set_pixels`), as on the Mac.
- The eleven new blend modes and six new adjustment kinds are parsed, validated and preserved. They draw
  as Normal / identity until Phase 3.5b and cannot be edited in a panel.
- Folder opacity is drawn as the Mac draws it (multiplied into every descendant; a clipped child in a
  dimmed folder dims twice).
- Saved guides move with Canvas Size, Crop, Image Size and Flip, are drawn under the crop dimming, and
  View > Show/Hide Guides toggles them.
- A dismissible notice lists what the project uses that this build draws differently or not at all,
  including the 1.2.6 Grain roughness kernel.
- Merge refuses to bake a layer carrying an undrawn feature (except Grain roughness), naming the feature.
- The tsconfig is split into three programs (app/src, unit tests, e2e) so `@types/node` never reaches
  browser code.

## Rulings

Pre-flight (23 findings against the plan, all accepted and fixed before Task 1):
- G-B1: Task 1 updates the five tests that pinned version 7.
- G-B2: Task 4 reverses hierarchy.rs's folder-opacity refusal while keeping the blend refusal.
- G-B3: Tasks 5 and 6 list every unit-test `DocumentState` literal.
- G-I1: `editAdjustmentLayer` itself refuses undrawn kinds (the plan wrongly claimed it already did).
- G-I2 / G-I4: real names (`SetLayerBlendMode`, `Sampling::Smooth`).
- G-I3: the noise/motion round trip compares the whole saved adjustment.
- G-I5: permanent Color Balance / B&W tint range and identity tests.
- G-I6: error.rs "1-7" message updated; README 117-120 rewritten, not contradicted.
- G-M10: `LayerTextStyle.isValid` bounds ported as a JSON check.
- G-M11: the constraint is "no information is lost" (default-valued keys may be omitted).

During execution:
- Task 2: the blend-select e2e asserts `toHaveAttribute("disabled", "")`, because Playwright retargets an
  option inside a label-wrapped select to the select.
- Task 5: saved guides are drawn before the crop dimming and handles (TransformOverlay.swift:92-100).
- Task 6: Grain adjustment layers with resolved roughness > 0 are disclosed as "the Compositor 1.2.6
  grain roughness" (AdjustPixels.c: `noise = smooth + (fine - smooth) * rough`, so the kernels agree only
  at roughness 0). The notice no longer sits under the adjust panel.
- Task 7: a sixth probe, `grain.comp`, is the 3.5b oracle for the grain kernel. `@types/node` (^22) is a
  dev dependency, confined to the e2e tsconfig program.
- Final review I1: merge refuses when any composited layer (folder descendants included) carries a
  per-layer undrawn feature; Grain roughness is excluded, because that grain is drawn and baking what the
  screen shows is the Phase 3 behaviour. Cost if wrong: a Mac-authored Grain layer merged here bakes the
  older texture.
- Final review M7: Canvas Size and Image Size refuse (as `ProjectError::TooLarge`, the ops' error type)
  when a moved guide would leave +/-1,000,000.
- Final review M9: a seventh probe, `edited-rich-file.comp`, sends the Mac a file this port has edited.
- Final review M1 closed: an unknown key cannot collide with a named field (flatten receives only
  unclaimed keys, and `to_value` de-duplicates).

## Open items for Phase 3.5b

- Draw the eleven blend modes (CPU and GL), then replace the Soft-Light-equals-Normal pixel assertion in
  `engine/tests/blend_modes_v9.rs` with a real Soft Light expectation; the `compose()` `is_drawn` guard is
  behaviourally redundant today (final review M3).
- Draw the six adjustment kinds, with Black & White and Color Balance panels.
- Draw the six layer effects (CPU and GPU), then decide whether merge may bake them. Delete-with-bake
  of a clipped dependant ignores the clipping base's effects (M8).
- Port the 1.2.6 Grain roughness kernel (adjustment layer AND the destructive Grain filter) and the
  position-based Add Noise hash; then drop the grain phrase from `Document::undrawn` and the merge
  exclusion.
- Type `effects` and check its shape as the Mac's decode does (M5); today a malformed `effects` object
  from another writer is accepted and re-saved.
- Write whole-number doubles in `LayerTransform` and `resolution` as `120`, not `120.0`, and normalise
  `-0.0` rotation after Flip Canvas (M2; the Mac decodes both identically).
- Give the Canvas Size guide-range refusal its own test (Image Size has one).
- Unknown keys INSIDE an adjustment object are still not preserved (Phase 3 M10); every key Mac 1.2.6
  writes there is typed.

## Owed by the user

- Open the seven probes in `build-artifacts/mac-probes/` on the Mac (Compositor 1.2.6), confirm each
  opens, export each as PNG at 100% into `mac-exports`, and send the folder back. The folder-opacity and
  clipped-in-dimmed-folder exports confirm two behaviours read from Mac code but never seen rendered.
- A Mac project saved via Layer > New Adjustment Layer > Hue/Saturation with an edited colour range, to
  confirm the Phase 3 C1 map encoding against a real file.
- A walkthrough of the 0.3.5 portable build.
