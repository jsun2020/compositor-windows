# Phase 4a: Selections - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compositor for Windows gets Compositor for Mac 1.2.10's selections. A selection is a vector outline with an anti-alias flag and a feather, kept in the document so undo covers it and never saved. The Rectangular and Elliptical Marquee, the Freehand and Polygonal Lasso and the Magic Wand make it, New / Add / Subtract (and Shift / Alt) combine outlines, and the outline moves by dragging and by the arrow keys. Select All, Deselect, Inverse, Expand, Contract and Feather change it, and a layer's pixels or a mask's black areas load as one. Marching ants show it. Destructive adjustments, filters and Invert stay inside it; the Levels and Curves histograms are weighted by it; Delete clears the selected pixels; Add Mask paints through it; the Crop tool starts at its bounds. Adjustment layers never take it. Task 1 and Task 2 also settle what the Mac 1.2.10 follow-up probes found: Soft Light is Pegtop's formula, and Motion Blur is CIMotionBlur's Gaussian.

**Architecture:** The engine owns the selection. `engine/src/selection/` holds the outline (`Selection`: closed contours in fixed point, 256 units a document pixel, i32), its geometry (polygons, the Marquee's rectangle and Core Graphics' four-Bezier ellipse, the booleans and the round-joined band Expand and Contract stroke, through the i_overlay crate on its integer engine), its coverage (an exact-area accumulation rasteriser, the Mac's feather as a Gaussian of sigma feather / 2 clamped at the region's edge, and that coverage on any layer's pixel grid), the tracer that turns pixels back into outlines (a line-for-line port of WandPixels.c), the Magic Wand's matcher, and a level-of-detail outline for the ants. `Document` gains `selection: Option<Selection>` and `selection_revision`; `same_content` compares the selection, so every change is one undo step, and `renew_revisions` issues a fresh revision from the engine-wide counter whenever it changes (LL-071). Every selection change is a `Command` with the Mac's undo name; the selection-limited edits take no selection argument because the selection is document state, which also fixes Phase 3's N3 (a kept preview is keyed on what it was made from). `DocumentState` carries a small `SelectionState` summary; the outline travels only when its revision changes (`selection_outline`, simplified below 1:1 past 20,000 points). The app adds three tools, a pure drawing-session module (the Mac's `DragBox`, `LassoDraft`, modifier modes), store state for the options, marching ants on the 2D overlay on a 120 ms timer that repaints only the overlay, the selection options bar, the Select menu, its amount sheet, keys and Ctrl-click on thumbnails. The GPU renderer is untouched: every selection-limited edit is destructive and already drawn from the engine's pixels.

**Tech Stack:** Rust (`engine`, `engine-wasm`), i_overlay 9.0.0 (new), serde / serde_json, TypeScript + React + zustand (`app`), WebGL2 and a 2D overlay canvas, vitest, Playwright.

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md`, section 3, Phase 4 ("4a Selections").

**Research this plan argues from:** `docs/superpowers/research/phase4-mac-1.2.10-selection-and-retouching.md` ("research"), `docs/superpowers/research/mac-1.2.10-probe-results.md` ("probe results"), and the rulings files for Phases 3, 3.5a, 3.5b and 3.5c (Phase 3's open item N3 is fixed in Task 6).

**Oracle:** Compositor 1.2.10, `C:\Users\sr9rfx\.claude-project\Compositor-1.2.10\Compositor` (sources) and `...\CompositorTests` (tests). Citations without a path are in that tree. Do NOT read `C:\Users\sr9rfx\.claude-project\Compositor`: it is an older checkout (its `addMask` ignores the feather, its Crop tool ignores the selection, its SelectionFeatherTests do not exist).

**Pixel oracles:** the 31 Mac 1.2.10 exports in `engine/tests/fixtures/mac-1.2.10-probes/` that Task 1 commits (the 14 Phase 3.5b follow-up probes, color-balance-preserve and the 16 `effects-*` probes), and the Mac's own tests ported with their numbers: SelectionTests, MagicWandTests, SelectionFeatherTests, the selection cases of SelectionEditTests, LevelsTests (histogram and selection), HueSaturationTests (`adjustmentStaysInsideTheSelectionAndIsOneUndoStep`), LayerMaskTests and HistoryTests.

**Measured for this plan (2026-09-27).** Every number below that is not quoted from the Mac was measured, not assumed. The measurements ran on one scratch copy of this repository (`C:\Users\sr9rfx\AppData\Local\Temp\claude\p4a-scratch`, its own target directory, the release wasm) with this plan's code applied. The code was then replayed onto the plan's base one task at a time (tags `t0` to `t14` there), and at each task the whole engine test tree compiled, the task's own tests passed and, from Task 8 on, `pnpm build` and vitest passed; the whole suites passed at the end (Task 14 Step 3 gives the counts). Every Rust and TypeScript block below is copied from that replay by a script, not retyped: a diff block is `git diff t<N-1> t<N>` for its file and a new file is `git show t<N>:<path>`. Not compiled: the README text; not run: `pnpm build:portable`.

**Base:** `phase4a-selections` at 97c7add (00189cc plus the sampling probe generators in `engine/tests/mac_probes.rs` and their results section). This plan never edits `mac_probes.rs`. Leave the seven `sampling-*` Mac exports in `engine/tests/fixtures/mac-1.2.10-probes/` untracked: resampling is a separate task after the next Mac exports.

**Out of scope (later parts of Phase 4, or not ported):** Object Selection and Select Subject (Apple Vision); moving or duplicating selected pixels (Ctrl-drag, Ctrl-arrow) and Transform Selection (the floating selection); fills, the clipboard, the palette and the colour picker; the brushes; Content-Aware Fill; scrolling the view while a Marquee or an outline is dragged past the window's edge (the Mac's `marqueeAutoscroll`, ruling OQ15); a shortcut editor.

## Global Constraints

- ASCII only in every source file, test, doc and commit message.
- The selection is `Document.selection: Option<Selection>`: `None` is no selection (an edit reaches the whole layer), `Some` with an empty outline or bounds of no area is an explicit empty selection (every edit refuses it with `CommandError::Refused("The selection is empty")`). It is content: `same_content` compares it, so a change is one undo step and a command that leaves it equal records none. It is never saved and never read from a file: `open_package`, `Document::new` and every other `Document` literal set `selection: None, selection_revision: 1`.
- Outlines are `Vec<Contour>`, `Contour = Vec<[i32; 2]>` in `SUBPIXEL = 256.0` units a document pixel, filled by the nonzero winding rule. Points lie within `SELECTION_COORDINATE_LIMIT = 1_000_000` document pixels of the origin; a command that would pass it is refused as an argument error. Booleans and the Expand / Contract band go through i_overlay 9 (`Overlay::from_subj_and_clip(..).overlay(rule, FillRule::NonZero)`, `IntStrokeOffset::stroke`) and nothing else; curves are flattened within `CURVE_TOLERANCE = 0.01` document pixel.
- The Mac's numbers, exactly: feather sigma = feather / 2, the coverage region = the outline's bounds grown by ceil(2 x feather) and a further pixel, rounded out and cut to the canvas, blurred with its edge repeated (`clampedToExtent`); Feather adds as sqrt(a^2 + b^2), at most 250; Expand and Contract take 1 to 500 px and stroke a band of twice that width with round caps and joins; Expand clips to the canvas, Contract does not; Feather's amount is 1 to 250; the Marquee's box is whole pixels (`DragBox.rect`); the Freehand Lasso skips points closer than 0.25 px; the Polygonal Lasso closes within 8 view px of its first corner (with 3 corners down) or on a double-click; the Magic Wand's tolerance is 0-255 per channel including alpha, its sample radius 0, 1 or 2, its outline refused past 8,000,000 edges; Layer's Pixels loads alpha >= 128, Mask's Black Areas loads mask < 128.
- Coverage on a layer's grid samples the canvas-grid coverage bilinearly at each layer pixel's centre (`SelectionClip::on_grid`); a grid on the canvas's own pixels moved by whole pixels copies it exactly. Blends are `coverage x adjusted + (1 - coverage) x original` on premultiplied bytes, rounded, as `CIBlendWithMask`.
- Adjustment layers never read the selection, in the render or in their histogram.
- `DocumentState` gains `selection: SelectionState | null` (revision, empty, bounds, antialiased, feather, points); the outline itself is fetched only when `selection.revision` changes. Every TypeScript `DocumentState` literal gains `selection: null` (eight unit tests, Task 8).
- The GPU renderer, the render plan and the compositor do not change for selections.
- Tests: for EVERY assertion, be able to name the production change that makes it fail. Compute expected values in the test from the formula or take them from the Mac test being ported; never paste them from a passing run. Measured bounds are the exception, and each names its measurement. Prefer asymmetric fixtures. Each task's Step 5 introduces a bug, watches the test fail, and reverts (LL-067, LL-068).
- E2E: prove commits with `undoDepth` against a baseline, never `canUndo`; drive the canvas with real pointer events at points computed from the viewport (`viewPoint`), never hard-coded screen offsets.
- tsconfig is three programs (`app/src`, `app/tests/unit`, `app/tests/e2e`), each with `noUnusedLocals`.
- Build and test from the repository root, in the FOREGROUND, PowerShell 5.1 (no `&&`; chain with `;`): `cargo test -p compositor-engine` (timeout at least 900 s); after any change under `engine/src` or `engine-wasm`, `pnpm wasm:dev` before `pnpm build` and `pnpm e2e`; `pnpm test`; `pnpm build`; `pnpm e2e` (server 127.0.0.1:1420, one run at a time). Do not use `2>&1` on native executables; redirect with `*> file`.
- A source file restored after a Step 5 bug must get a new modification time (PowerShell `(Get-Item <file>).LastWriteTime = Get-Date`), or cargo keeps the build with the bug in it: `git checkout -- <file>` does this, a copy from a backup does not (found on the scratch copy).
- Timings come only from the release wasm (`pnpm wasm`), never `wasm:dev`.
- Do NOT run `pnpm build:portable`: the controller builds the zip.
- Commit with an explicit pathspec (`git add -- <new files>` first, then `git commit -- <paths>`). End every commit message with the Co-Authored-By line your own instructions give, as its own `-m` paragraph. The Step 6 commands below show the line for Claude Opus 5.5 (1M context); an implementer on another model writes its own.
- Baseline before Task 1, measured at 97c7add's state on the scratch copy: `cargo test -p compositor-engine` 382 passed and 1 ignored; vitest 114; `pnpm build` clean; e2e 117 passed and 2 skipped. Task 1 Step 1 records them on the real repository before any change.

## Rulings made for this plan (OQ)

Each was ruled and measured here, as the brief asked; "cost if wrong" is what changing it later touches.

1. **OQ1 Polygon booleans: the i_overlay crate, 9.0.0, on its i32 integer engine, fixed point 256 units a document pixel.** MIT OR Apache-2.0, pure Rust, `rust-version` 1.88 (the repository builds with 1.95), builds for `wasm32-unknown-unknown` (checked). The Mac's selection is a vector `CGPath` combined with `union` / `intersection` / `subtracting` and widened with `copy(strokingWithWidth:)` (Selection.swift:233-342), so the port keeps a vector outline rather than a raster mask: a move is exact, an Expand is round-joined, and memory does not grow with the canvas. Fixed point keeps whole-pixel shapes exact through every boolean, so equal selections compare equal and undo records nothing for a no-op. Wasm size, release, measured for all of Phase 4a (i_overlay is most of it; not isolated): 2,210,211 bytes at the base, 2,542,562 after Task 14 (+332,351, +15.0 %; 888.7 kB gzipped). Cost if wrong: `engine/src/selection/geometry.rs` alone calls the crate (`combine`, `band`); a replacement changes that file and its two tests.
2. **OQ2 The ellipse is Core Graphics' `addEllipse(in:)`: four cubic Beziers with the circle constant kappa, flattened within `CURVE_TOLERANCE = 0.01` px (Wang's bound).** Measured against the true ellipse's area coverage (16 x 16 samples a pixel) in the Mac test's box (10, 20)-(70, 60): at most 4 levels on any pixel (`an_ellipse_fills_its_box_as_an_oval_with_soft_edges`). Cost if wrong: edge pixels of an elliptical selection move by a few levels; `ellipse()` only.
3. **OQ3 Coverage is an exact-area accumulation rasteriser for an antialiased outline and centre sampling (half-open) for a hard one.** Core Graphics' antialiased fill is also an area coverage (a whole-pixel rectangle is exactly 0 or 255 either way, a 45-degree diagonal is 128 on its pixels: measured). No Mac render of a selection's coverage exists to compare (below, OQ20), so CG's own rounding at soft edges is unverified; the Mac's tests' expectations (`antialiasingControlsEdgeCoverage`, `clipFollowsScaledLayersAndSoftensEdges`) hold. Cost if wrong: a level or two along soft edges; `coverage.rs::rasterize` only.
4. **OQ4 Feather is the Mac's: a Gaussian of sigma feather / 2 over the coverage region cut to the canvas, its edge pixels repeated beyond it; amounts add as sqrt(a^2 + b^2) up to 250.** The kernel reaches 3 sigma = 1.5 x feather, inside the ceil(2 x feather) the region adds (Selection.swift:24-48). Measured: the profile across a straight edge is the formula's within 1 level at feather 6 (`a_feather_blurs_the_coverage_by_half_its_amount`). Cost if wrong: `SelectionClip::new`.
5. **OQ5 Expand and Contract are the Mac's round-joined band.** i_overlay's stroke with round joins and caps, its arc step chosen so each chord strays at most `CURVE_TOLERANCE` from the arc (i_overlay's default step, 45 degrees, is a visible polygon). The round join is pinned at the Mac test's square: pixel (35, 35) stays out of a 5 px Expand, which a mitred join would fill. Expand is cut to the canvas, Contract is not and can leave an explicit empty selection (Selection.swift:316-342). Cost if wrong: `band()` only.
6. **OQ6 The Magic Wand is WandPixels.c ported line for line** (`wand_mask`, `wand_trace`): tolerance per channel including alpha on premultiplied bytes, the sample averaged over a 1, 3 or 5 px square clamped to the image, 4-connected flood when Contiguous, the traced outline refused past 8,000,000 edges (MagicWand.swift:28). Measured: a 2001 x 2000 checkerboard is refused and a 2000 x 2000 one traces. "This Layer" reads the active layer's own pixels through its transform, without its mask or opacity; "All Layers" the visible composite (MagicWand.swift:60-96). In Replace the outline is taken as traced (it already lies on the canvas); Add and Subtract go through the canvas clip as every outline does. Loading a layer's pixels or a mask's black areas uses the same tracer, as the Mac's `MaskTracing` does, and refuses with the banner where the Mac beeps (no pixels, too detailed). Cost if wrong: `wand.rs`, `trace.rs`.
7. **OQ7 The selection is document state, fetched as a summary plus an outline.** `DocumentState.selection` is a `SelectionState` (revision, empty, bounds, antialiased, feather, points); `Engine::selection_outline(doc, step)` sends the outline flat (`[contours, n, x, y, ...]`) and, past `OUTLINE_DETAIL_LIMIT = 20,000` points with `step` < 1, the outline traced from its own coverage at that resolution (TransformOverlay.swift:92-177: the Mac re-traces at screen resolution past 20,000 path elements below 1:1). The revision comes from the engine-wide counter in `renew_revisions`, never from a counter a snapshot restores (LL-071). Measured: a 24,000-tooth outline comes back as its 4 corners at step 0.25. Cost if wrong: a slow ants redraw on huge outlines; `outline.rs` and `selection_outline`.
8. **OQ8 Commands take no selection argument; refusals are `CommandError::Refused`.** Every selection change is one `Command` with the Mac's undo name (Selection.swift:228-358; MagicWand.swift:119; MaskTracing.swift:82-93); the edits read `doc.selection` themselves, so the app cannot pass a stale one. `Refused(String)` carries the user-facing words ("The selection is empty", "Nothing is selected", the Mac's too-detailed message) for the banner. Cost if wrong: the JSON of 13 command variants.
9. **OQ9 Coverage on a layer's grid is the canvas-grid coverage sampled bilinearly at each layer pixel's centre.** The Mac fills the path afresh in each layer's grid (PixelAdjust.swift:23-34), which on a scaled or turned layer is a fresh area coverage. Sampling instead means one rasterisation per edit whatever the layer. On the canvas's own grid and on whole-pixel moves it is exact (measured: bit-equal); on a 2x layer the Mac test `clipFollowsScaledLayersAndSoftensEdges` holds. Cost if wrong: soft edges on scaled layers a level or two different; `on_grid`.
10. **OQ10 Phase 3's N3 is fixed: a kept preview is keyed on what it was made from.** `PixelPreview.source` is the stored layer's pixels revision and placement and the selection's revision (`PreviewSource`), compared in `answers`, so a mutation that forgets to clear the preview can no longer be answered with stale pixels. Cost if wrong: none known; `preview.rs`.
11. **OQ11 Crop, Canvas Size and Image Size drop the selection; Flip Canvas mirrors it** (ImageResizer / Crop `applyDocumentSize` make a new document; LayerFlip.swift:69-76). Cost if wrong: one line each in `Engine::execute`.
12. **OQ12 Add Mask with a selection is the Mac's `addMask(revealing:)`: the selection's clip (feather included, cut to the canvas) painted in the opposite tone on the layer's pixel grid, and the selection used up in the same step** (LayerMask.swift:233-260, 1.2.10; the older checkout filled the bare path). Every Add Mask entry point in the app goes through it, as on the Mac (the panel's button, the context menu, the Layer menu); with no selection it is the plain `AddMask`. An empty selection makes a plain mask and is used up too. Cost if wrong: `add_mask_from_selection`.
13. **OQ13 Delete with a selection clears the selected pixels ("Clear"), or fills a targeted mask white ("Fill Mask")** (SelectionEdits.swift:49-63). White is the mask palette's default background; the palette arrives with fills. It needs what `canPaint` needs: one layer targeted and shown, pixels or an enabled mask, and a selection with something in it; otherwise it does nothing, as on the Mac. Without a selection Delete deletes the selected layers, as before this plan (the Mac's mask-targeted Delete, which deletes the mask, is unchanged here: out of this plan's scope). Cost if wrong: `deleteKeyPressed`, `clear_selected`.
14. **OQ14 Port-only mask items:** Layer > Mask > Invert Mask is the same edit as Image > Invert with the mask targeted (the Mac has only that one), so it stays inside the selection; Fill Mask White / Black and Blur Mask ignore the selection until fills arrive. Cost if wrong: a user expecting them to follow the selection; `ops::masks`.
15. **OQ15 The app side.** Tools `marquee` (M), `lasso` (L), `wand` (W); Tab switches Rectangle / Ellipse and Freehand / Polygonal (Wand / Object does not exist here: Object is Vision). The outline being drawn is a `SelectionDraft` in the store and repaints only the overlay (`overlayTick`), never the picture. Dragging the outline shows it offset on the overlay and sends one `MoveSelection` on release (the Mac moves the document's selection live inside one undo step: the same result). The ants march every 120 ms only while a selection with something in it exists, a white line under a black 4/4 dash, and repaint only the overlay; the outline is fetched again only for a new revision or zoom step (`OutlineCache`). Where a press lands inside the selection comes from the engine (`selection_contains`, the exact winding rule). The Marquee's Shift squares only when pressed afresh during the drag, and Shift pressed or released mid-drag reshapes it at once (EditorCanvas.swift:611-619, :1949-1956). Not ported: autoscroll while dragging a Marquee or an outline past the window's edge (EditorCanvas.swift:1899-1931); the port's other drags (crop, move) do not autoscroll either. Cost if wrong: a drag that runs out of window; `CanvasView.tsx`.
16. **OQ16 Motion Blur is CIMotionBlur: a Gaussian along the angle with sigma = distance / sqrt(12), one kernel for the filter, the adjustment layer and the GPU.** The Mac passes `inputRadius = distance / sqrt(12)` (Filters.swift:170-208), and the probe shows the Gaussian, not the even streak. Taps at whole pixels out to ceil(3 sigma), bilinear, zero outside, on the CPU as a precomputed row-wise stencil (equal to one-tap-at-a-time within float ordering: measured at most 1 level on 2 of 23,668 bytes). Measured against the Mac's `motion-blur-30-24` export, premultiplied: colour 6, alpha 6, mean 0.195 per byte (the even streak was 26, 28 and 3.108). Measured alternatives: 4 sigma 6 / 6 / 0.138; quarter-pixel steps at 3 sigma 4 / 4 / 0.103, at 4 to 8 times the taps; kept at 3 sigma and whole pixels. Its reach is 3 sigma, so Motion Blur shares the Gaussian's `SPATIAL_REACH_LIMIT = 48` (the separate `MOTION_REACH_LIMIT = 12` goes): exact up to a 55 px distance at export. The destructive filter keeps the Mac's padding, distance / 2 + 2 (`FilterEdit.blurMargin`), so the filter clips as the Mac's does; the adjustment layer's sampling margin is 3 sigma + 2. The notice entry ("drawn approximately") and the merge refusal go. Release-wasm timings on 3000 x 2000 at 30 degrees, before and after: layer export at 24 px 12,143 -> 9,491 ms; at 90 px 2,676 -> 3,345 ms; at 2000 px 2,128 -> 2,089 ms; destructive filter at 24 px 10,784 -> 3,200 ms; at 90 px 40,643 -> 10,910 ms. Peak heap: 12.06 bytes a pixel at level 0 (20 px), 9.66 at level 2 (120 px). Cost if wrong: `motion_blur`, `FRAG_MOTION` and the spatial levels.
17. **OQ17 Soft Light is Pegtop's formula, `(1 - 2cs) cb^2 + 2 cs cb`, on the CPU and the GPU.** Measured on `blend-greys`: 1 level at every grey and alpha (W3C was 14 off at 75 % grey). Cost if wrong: `soft_light` and GLSL mode 15.
18. **OQ18 Probes: all 31 new exports pinned at their measured bounds; `effects-transformed` only where it agrees.** Its columns 0-73 (the flipped layer and its effects) are bit-identical; from column 74 the turned layer's shadow and the layer itself differ (alpha 1-2 at columns 74-89, up to colour 29 and alpha 52 further right), because the Mac enlarges a High quality layer with Core Graphics' `.high` filter and this port samples bilinearly (LayerRenderer.swift:42-44). The controller's note put the edge at 83; measured here, the shadow reaches 74. The resampling is an open item for the sampling probes, not this phase.
19. **OQ19 Version 0.4.0**, in `package.json`, `src-tauri/tauri.conf.json`, the workspace `Cargo.toml`, `Cargo.lock` and the smoke test's literal; the README gains a Phase 4a section. The controller builds the zip.
20. **OQ20 No new Mac probe for Phase 4a.** A selection is never saved, so a probe project cannot carry one: a Mac render of a selection-limited edit would need the user to draw the selection by hand on the Mac, and would then compare the rasteriser and the feather (OQ3, OQ4) rather than anything this plan decides. The Mac's own tests, ported with their numbers, are the oracle here. A later hand-made export (Invert through a feathered ellipse) would pin OQ3 / OQ4.
21. **OQ21 The Marquee draws whole pixels and needs no Anti-alias; the Lasso, the Magic Wand and the ellipse show it** (LassoControls.swift:48-52), and every outline takes the current setting, as `applySelection` does.

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `engine/tests/fixtures/mac-1.2.10-probes/` (31 pairs) | Add | The Mac 1.2.10 exports Task 1 pins (untracked in the working tree today) |
| `engine/src/blend.rs`, `app/src/canvas/gl/programs.ts` | Modify | Soft Light is Pegtop's (Task 1); `FRAG_MOTION` is the Gaussian (Task 2) |
| `engine/src/adjust/filters.rs` | Modify | `motion_sigma`, `motion_reach`, `motion_stencil`; `motion_blur` is CIMotionBlur's Gaussian |
| `engine/src/adjust/spatial.rs`, `engine/src/adjust/settings.rs` | Modify | Motion Blur's level and margin from its 3 sigma; `MOTION_REACH_LIMIT` goes |
| `engine/src/document.rs` | Modify | The Motion Blur notice entry goes (Task 2); `selection`, `selection_revision`, `same_content` (Task 3) |
| `app/src/canvas/gl-renderer.ts` | Modify | The motion pass's uniforms |
| `engine/Cargo.toml`, `Cargo.lock` | Modify | `i_overlay = "9"` (Task 3); 0.4.0 (Task 14) |
| `engine/src/selection/mod.rs` | Create | `Selection`, `Contour`, `SelectionMode`, `SelectionShape`, `SelectionState`, `SUBPIXEL`, limits |
| `engine/src/selection/geometry.rs` | Create | Polygons, rectangle, ellipse, `transformed`, `combine`, `band` (i_overlay) |
| `engine/src/selection/coverage.rs` | Create | `rasterize`, `SelectionClip` (`new`, `at`, `on_grid`), `selection_coverage` |
| `engine/src/selection/trace.rs` | Create | `trace_pixels` (WandPixels.c `wand_trace`), `opaque_pixels`, `dark_pixels`, `WAND_EDGE_LIMIT` |
| `engine/src/selection/outline.rs` | Create | `selection_lod`, `OUTLINE_DETAIL_LIMIT`, `OUTLINE_MASK_LIMIT` |
| `engine/src/selection/wand.rs` | Create | `WandSettings`, `wand_mask` (WandPixels.c), `magic_wand` |
| `engine/src/ops/selection.rs` | Create | Every selection command, Delete and Add Mask from Selection |
| `engine/src/lib.rs`, `engine/src/ops/mod.rs` | Modify | Register and re-export |
| `engine/src/package.rs`, `engine/src/compositor.rs` | Modify | Their `Document` literals: no selection |
| `engine/src/error.rs` | Modify | `CommandError::Refused` |
| `engine/src/command.rs` | Modify | 13 commands and their undo names |
| `engine/src/engine.rs` | Modify | `DocumentState.selection`, `selection_outline`, `selection_contains`, revisions, the commands, the weighted histogram, the preview key |
| `engine/src/ops/adjust.rs`, `engine/src/adjust/apply.rs`, `engine/src/ops/masks.rs`, `engine/src/preview.rs` | Modify | `edit_coverage`; adjustments, Invert and filters inside the selection; `blend_gray_by_coverage`; `PreviewSource` |
| `engine-wasm/src/lib.rs` | Modify | `selection_outline`, `selection_contains` |
| `app/src/engine/types.ts`, `app/src/engine/client.ts` | Modify | The commands, `SelectionState`, `WandSettings`; `selectionOutline`, `selectionContains` |
| `app/src/tools/selection-draft.ts` | Create | `SelectionDraft`, `dragBox`, `selectionMode`, `outlineOffset` |
| `app/src/state/store.ts` | Modify | Tools, `SelectionOptions`, the draft, the outline move, `modifySelection`, `cycleToolMode`, the crop seed, `canAdjust` |
| `app/src/actions/layers.ts` | Modify | `addMaskToActive` with a selection, `deleteKeyPressed`, `canClearSelected`, `loadSelection`, `canInvert` |
| `app/src/tools/crop-tool.ts` | Modify | `cropSeed` |
| `app/src/canvas/ants.ts` | Create | `OutlineCache`, `outlineStep`, `nextPhase`, `ANTS_INTERVAL_MS` |
| `app/src/canvas/overlay.ts`, `app/src/canvas/CanvasView.tsx` | Modify | Ants and drafts on the overlay; the selection tools' pointer handling |
| `app/src/panels/SelectionOptions.tsx` | Create | The options bar |
| `app/src/sheets/SelectionAmountSheet.tsx` | Create | Expand / Contract / Feather amount |
| `app/src/panels/MenuBar.tsx`, `ToolRail.tsx`, `LayersList.tsx`, `app/src/App.tsx`, `app/src/styles.css` | Modify | Select menu, tools, Ctrl-click on thumbnails, mounting |
| `app/src/shortcuts/keymap.ts`, `useShortcuts.ts` | Modify | M, L, W, Ctrl+A, Ctrl+D, Ctrl+Shift+I, Tab; arrows, Enter, Escape, Backspace, Delete |
| Engine tests created | Create | `selection_model.rs`, `selection_commands.rs`, `selection_wand.rs`, `selection_edits.rs`, `selection_masks.rs` |
| Engine tests modified | Modify | `mac_1_2_10.rs`, `blend_modes_v9.rs`, `filters.rs`, `spatial_adjustments.rs`, `undrawn.rs`, `merge_undrawn.rs`, `peak_heap.rs` |
| App tests created | Create | `selection-draft.test.ts`, `selection-store.test.ts`, `ants.test.ts`, `app/tests/e2e/selection.spec.ts` |
| App tests modified | Modify | `engine-client.test.ts`, `keymap.test.ts`, `crop-seed.test.ts`, the seven other `DocumentState` literals; `mac-1.2.10.spec.ts`, `mac-1.2.6.spec.ts`, `helpers.ts`, `smoke.spec.ts` |
| `README.md`, `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`, spec 4.5, 3.5b rulings | Modify | Docs and 0.4.0 |

---

### Task 1: The Mac 1.2.10 follow-up and effects probes, and Soft Light as the Mac draws it

The user exported 31 more probes from Compositor for Mac 1.2.10: the 14 Phase 3.5b follow-up probes, color-balance-preserve and the 16 `effects-*` probes. They sit untracked in `engine/tests/fixtures/mac-1.2.10-probes/` (beside seven `sampling-*` exports that stay untracked). This task commits the 31 and pins each at the bound measured against this port's CPU compositor (probe results, "Phase 3.5b follow-up probes" and "Phase 3.5c effects probes and color-balance-preserve"). One probe shows a real difference: `blend-greys` puts Soft Light 14 levels off at the 75 % grey, and fits Pegtop's formula within 1 (probe results, the Soft Light table), so Soft Light becomes Pegtop's on the CPU and the GPU. The motion probe is pinned in Task 2, which ports the Gaussian it shows.

**Files:**
- Add: the 31 probe pairs listed in Step 6
- Modify: `engine/src/blend.rs` (`soft_light`), `app/src/canvas/gl/programs.ts` (mode 15)
- Modify tests: `engine/tests/mac_1_2_10.rs`, `engine/tests/blend_modes_v9.rs`, `app/tests/e2e/mac-1.2.10.spec.ts`

**Interfaces:**
- Produces: `mac_1_2_10.rs` helpers `composite_of(name) -> Raster` (this port's premultiplied composite; `ours` becomes its straight form) and `premultiplied(name) -> (u8, u8, f64)` (worst colour, worst alpha, mean over every byte, both premultiplied). Task 2 uses both. The e2e helper `onWholePixels(page)`.

- [ ] **Step 1: Record the baseline, then write the tests**

Before touching anything, run and write down: `cargo test -p compositor-engine` (expect 382 passed, 1 ignored), `pnpm test` (expect 114), `pnpm build` (clean), then `pnpm wasm:dev` and `pnpm e2e` (expect 117 passed, 2 skipped; Task 14 reports against these).

Check that the 31 exports are there: `Get-ChildItem engine/tests/fixtures/mac-1.2.10-probes -Filter *.mac-1.2.10.png` lists them among the rest (Step 6 names each).

In `engine/tests/mac_1_2_10.rs`: this port's composite as a premultiplied raster, a premultiplied comparison, and a test per probe or group of probes, each at its measured bound. Every bound was measured on the scratch copy against this build, and each equals the controller's measurement at 00189cc except `effects-transformed` (ruling OQ18):

```diff
--- a/engine/tests/mac_1_2_10.rs
+++ b/engine/tests/mac_1_2_10.rs
@@ -7,8 +7,8 @@ use std::ops::Range;
 
 fn fixtures() -> String { format!("{}/tests/fixtures/mac-1.2.10-probes", env!("CARGO_MANIFEST_DIR")) }
 
-/// This port's composite of the probe, straight RGBA8.
-fn ours(name: &str) -> Vec<u8> {
+/// This port's composite of the probe, premultiplied.
+fn composite_of(name: &str) -> Raster {
     let comp = format!("{}/{name}.comp", fixtures());
     let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
     let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
@@ -17,9 +17,12 @@ fn ours(name: &str) -> Vec<u8> {
     }).collect();
     let doc = open_package(&Package { manifest_json, images }).unwrap_or_else(|e| panic!("{name}: {e:?}"));
     let region = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
-    composite(&doc, region, doc.width, doc.height).to_straight()
+    composite(&doc, region, doc.width, doc.height)
 }
 
+/// This port's composite of the probe, straight RGBA8.
+fn ours(name: &str) -> Vec<u8> { composite_of(name).to_straight() }
+
 /// The Mac's export: width, and straight RGBA8 as the PNG stores it.
 fn mac(name: &str) -> (u32, Vec<u8>) {
     let png = std::fs::read(format!("{}/{name}.mac-1.2.10.png", fixtures())).unwrap();
@@ -103,3 +106,127 @@ fn the_probes_this_port_already_matched_still_match() {
         assert_eq!(d, 0, "{name}: worst at {at:?}");
     }
 }
+
+/// This port's composite of the probe and the Mac's export, both PREMULTIPLIED RGBA8: the worst
+/// colour and alpha differences, and the mean difference over every byte. The Mac's straight
+/// colour is ill-conditioned where its alpha is a few units (probe results), so blurs compare here.
+fn premultiplied(name: &str) -> (u8, u8, f64) {
+    let (width, theirs) = mac(name);
+    let theirs = Raster::from_straight(width, theirs.len() as u32 / 4 / width, &theirs);
+    let ours = composite_of(name);
+    assert_eq!(ours.bytes().len(), theirs.bytes().len(), "{name}: the port and the Mac render the same size");
+    let (mut colour, mut alpha, mut sum) = (0u8, 0u8, 0u64);
+    for (i, (a, b)) in ours.bytes().iter().zip(theirs.bytes()).enumerate() {
+        let d = a.abs_diff(*b);
+        if i % 4 == 3 { alpha = alpha.max(d); } else { colour = colour.max(d); }
+        sum += d as u64;
+    }
+    (colour, alpha, sum as f64 / ours.bytes().len() as f64)
+}
+
+// The Phase 3.5b follow-up probes, exported by Compositor for Mac 1.2.10 on 2026-09-27 (probe
+// results, "Phase 3.5b follow-up probes"). Every bound below was measured on p4a-scratch against
+// this build (2026-09-27), and matches the probe results' own comparison at 12a0e34.
+
+#[test]
+fn the_follow_up_probes_this_port_draws_exactly_match_the_mac_render_bit_for_bit() {
+    // Color Balance without Preserve Luminosity, both Add Noise modes, a Levels layer in Divide and
+    // one in Color Dodge (Core Graphics' own formulas agree with this port's here), both clipping
+    // stack bases, and an Invert layer.
+    for name in ["color-balance-no-preserve", "add-noise-uniform", "add-noise-gaussian-mono", "cgmode-levels-divide",
+        "color-dodge-adjustment", "cgmode-stack-bases", "invert"] {
+        let (width, theirs) = mac(name);
+        let (d, at) = worst(&ours(name), &theirs, width, 0..width);
+        assert_eq!(d, 0, "{name}: worst at {at:?}");
+    }
+}
+
+#[test]
+fn a_tinted_black_and_white_layer_matches_the_mac_render_within_one_level() {
+    // Measured 1, on two pixels.
+    let (width, theirs) = mac("black-white-tint");
+    let (d, at) = worst(&ours("black-white-tint"), &theirs, width, 0..width);
+    assert!(d <= 1, "worst {d} at {at:?}");
+}
+
+#[test]
+fn a_blur_layer_in_linear_burn_keeps_the_original_alpha_as_the_mac_does() {
+    // Ruling E-I1 confirmed by the Mac: the canvas edge does not fade (alpha exact). Measured colour
+    // 3, on 28 pixels over 2, all at the canvas edge.
+    let (width, theirs) = mac("cgmode-blur-linear-burn");
+    let port = ours("cgmode-blur-linear-burn");
+    let (mut colour, mut alpha) = (0u8, 0u8);
+    for (i, (a, b)) in port.iter().zip(&theirs).enumerate() {
+        if i % 4 == 3 { alpha = alpha.max(a.abs_diff(*b)); } else { colour = colour.max(a.abs_diff(*b)); }
+    }
+    assert_eq!(alpha, 0, "alpha");
+    assert!(colour <= 3, "colour {colour}");
+    assert_eq!(width, 160);
+}
+
+#[test]
+fn the_gaussian_blur_probes_match_the_mac_render_premultiplied() {
+    // Measured colour and alpha: radius 6 (the exact kernel) 2 and 2; radius 40 (the halved path) 2
+    // and 3; radius 6 at 60% under a ramp mask 1 and 1.
+    for (name, colour, alpha) in [("gaussian-blur-6", 2, 2), ("gaussian-blur-40", 2, 3), ("blur-soft-mask", 1, 1)] {
+        let (c, a, _) = premultiplied(name);
+        assert!(c <= colour && a <= alpha, "{name}: colour {c} alpha {a}, measured {colour} and {alpha}");
+    }
+}
+
+#[test]
+fn every_band_of_blend_greys_matches_the_mac_render() {
+    // Six 60-row bands over a hue sweep, one per mode, each six 40-px grey columns: 25%, 50% and
+    // 75% grey, opaque then at half alpha. Soft Light is Pegtop's formula: measured 1 there and in
+    // Hard Light, 0 in the other four. The W3C Soft Light this port drew before was 14 levels off at
+    // the 75% grey (probe results).
+    let (width, theirs) = mac("blend-greys");
+    let port = ours("blend-greys");
+    for (band, mode, measured) in [(0u32, "Soft Light", 1u8), (1, "Hard Light", 1), (2, "Linear Light", 0), (3, "Pin Light", 0), (4, "Vivid Light", 0), (5, "Hard Mix", 0)] {
+        let rows = band * 60..band * 60 + 60;
+        let mut d = 0u8;
+        for y in rows { for x in 0..width { for c in 0..4 {
+            let i = ((y * width + x) * 4 + c) as usize;
+            d = d.max(port[i].abs_diff(theirs[i]));
+        }}}
+        assert!(d <= measured, "{mode}: {d}, measured {measured}");
+    }
+}
+
+// The Phase 3.5c effects probes and color-balance-preserve, exported by Compositor for Mac 1.2.10 on
+// 2026-09-27 (probe results, "Phase 3.5c effects probes and color-balance-preserve"). Measured
+// premultiplied on p4a-scratch against this build (2026-09-27); the same as the controller's
+// measurement at 00189cc.
+
+#[test]
+fn the_effects_probes_this_port_draws_exactly_match_the_mac_render_bit_for_bit() {
+    for name in ["color-balance-preserve", "effects-stroke-outside", "effects-drop-shadow", "effects-inner-shadow",
+        "effects-outer-glow", "effects-inner-glow", "effects-color-overlay", "effects-masked", "effects-clipping-base",
+        "effects-clipped-child", "effects-folder", "effects-invalid"] {
+        let (colour, alpha, _) = premultiplied(name);
+        assert!(colour == 0 && alpha == 0, "{name}: colour {colour} alpha {alpha}");
+    }
+}
+
+#[test]
+fn the_other_effects_probes_match_the_mac_render_within_their_measured_bounds() {
+    // Measured colour and alpha: an inside stroke 1 and 0; all six effects 1 and 0; a large blur (the
+    // halved path) 1 and 1; a placed mask 3 and 2, where Core Graphics resamples it (rulings 14 / OQ3).
+    for (name, colour, alpha) in [("effects-stroke-inside", 1, 0), ("effects-all-six", 1, 0), ("effects-large-blur", 1, 1), ("effects-mask-placed", 3, 2)] {
+        let (c, a, _) = premultiplied(name);
+        assert!(c <= colour && a <= alpha, "{name}: colour {c} alpha {a}, measured {colour} and {alpha}");
+    }
+}
+
+#[test]
+fn the_flipped_layer_of_effects_transformed_matches_the_mac_render_exactly() {
+    // Columns 0-73 hold only the flipped layer and its effects: bit-identical. From column 74 the
+    // turned layer (rotation 25, drawn at 150 %) and its shadow differ, by up to colour 29 and alpha
+    // 52, because the Mac enlarges a High quality layer with Core Graphics' .high filter and this
+    // port samples bilinearly (LayerRenderer.swift:42-44): an open item, waiting on the sampling-*
+    // probes, not in Phase 4a's scope.
+    let (width, theirs) = mac("effects-transformed");
+    let (d, at) = worst(&ours("effects-transformed"), &theirs, width, 0..74);
+    assert_eq!(d, 0, "worst at {at:?}");
+    assert_eq!(width, 200);
+}
\ No newline at end of file
```

In `engine/tests/blend_modes_v9.rs`, Soft Light's expected value is Pegtop's:

```diff
--- a/engine/tests/blend_modes_v9.rs
+++ b/engine/tests/blend_modes_v9.rs
@@ -39,19 +39,16 @@ fn a_new_mode_still_needs_version_3_like_every_non_normal_mode() {
 /// The W3C / PDF separable blend functions, plus the edge rules the Mac render showed (probe
 /// results "Blend modes"), written out independently of blend.rs, on straight colours in 0..1.
 /// This is a second transcription of the same published formulas, so it cannot catch a misreading
-/// both copies share. For Soft Light, Hard Light, Linear Light and Pin Light the first Mac probe
-/// could not tell the formula apart (a pure-green source); the Task 12 blend-greys probe settles
-/// them, and its render joins mac_1_2_10.rs when it comes back.
+/// both copies share; the Mac's blend-greys render (mac_1_2_10.rs) is the oracle for Soft Light,
+/// Hard Light, Linear Light, Pin Light, Vivid Light and Hard Mix on grey sources.
 fn expected_blend(mode: &str, cb: f32, cs: f32) -> f32 {
     let burn = |b: f32, s: f32| if b >= 1.0 { 1.0 } else if s <= 0.0 { 0.0 } else { 1.0 - ((1.0 - b) / s).min(1.0) };
     let dodge = |b: f32, s: f32| if b <= 0.0 { 0.0 } else if s >= 1.0 { 1.0 } else { (b / (1.0 - s)).min(1.0) };
     match mode {
         "Linear Burn" => (cb + cs - 1.0).max(0.0),
         "Linear Dodge (Add)" => (cb + cs).min(1.0),
-        "Soft Light" => if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) } else {
-            let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
-            cb + (2.0 * cs - 1.0) * (d - cb)
-        },
+        // Pegtop's, which the blend-greys probe fits within 1 level (probe results).
+        "Soft Light" => (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb,
         "Hard Light" => if cs <= 0.5 { cb * 2.0 * cs } else { let s = 2.0 * cs - 1.0; cb + s - cb * s },
         "Vivid Light" => if cs <= 0.5 { burn(cb, 2.0 * cs) } else { dodge(cb, 2.0 * cs - 1.0) },
         "Linear Light" => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
```

In `app/tests/e2e/mac-1.2.10.spec.ts`, the GPU draws `blend-greys` within 1 of the Mac (measured 1 on the release wasm):

```diff
--- a/app/tests/e2e/mac-1.2.10.spec.ts
+++ b/app/tests/e2e/mac-1.2.10.spec.ts
@@ -196,6 +196,25 @@ test("the new-blend-modes probe draws on the GPU as the Mac exported it", async
   expect(worstOf(await glPixels(page), await macPixels(page, "new-blend-modes"))).toBeLessThanOrEqual(2);
 });
 
+/** Whether the document's device rect starts on whole device pixels (LL-065(6)): checked, not assumed. */
+async function onWholePixels(page: Page): Promise<boolean> {
+  return page.evaluate(async () => {
+    const api = (window as any).__compositor;
+    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+    const s = api.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
+    const rect = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
+    return Number.isInteger(rect.x * dpr) && Number.isInteger(rect.y * dpr);
+  });
+}
+
+test("the blend-greys probe draws on the GPU as the Mac exported it, Soft Light included", async ({ page }) => {
+  await openProbe(page, "blend-greys");
+  expect(await onWholePixels(page), "the document sits on whole device pixels").toBe(true);
+  // The CPU is within 1 of the Mac (mac_1_2_10.rs); W3C's Soft Light was 14 off at the 75% grey.
+  // Measured 1 on p4a-scratch (2026-09-27).
+  expect(worstOf(await glPixels(page), await macPixels(page, "blend-greys"))).toBeLessThanOrEqual(1);
+});
+
 /** A new adjustment layer of `kind` on the active document, with `settings` merged into it; it becomes the active layer. */
 async function addAdjustment(page: Page, kind: string, settings: object | null): Promise<string> {
   return page.evaluate(({ kind, settings }) => {
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test mac_1_2_10 --test blend_modes_v9`. Expect exactly two failures: `every_band_of_blend_greys_matches_the_mac_render` (Soft Light: 14, measured 1) and `blend_modes_v9.rs`'s `each_new_mode_composites_by_its_formula_over_an_opaque_backdrop` (Soft Light). Every other new pin passes at once: those probes record what this port already draws.

- [ ] **Step 3: Soft Light is Pegtop's formula**

```diff
--- a/engine/src/blend.rs
+++ b/engine/src/blend.rs
@@ -10,15 +10,10 @@ pub const HARD_MIX_MARGIN: f32 = 0.5 / 255.0;
 fn color_dodge(cb: f32, cs: f32) -> f32 { if cb <= 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) } }
 fn color_burn(cb: f32, cs: f32) -> f32 { if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) } }
 
-/// W3C / PDF Soft Light, which Core Graphics draws (R 4.4). The first Mac render could not tell it
-/// from Photoshop's or Pegtop's (probe results); the Task 12 probe settles it.
-fn soft_light(cb: f32, cs: f32) -> f32 {
-    if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) }
-    else {
-        let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
-        cb + (2.0 * cs - 1.0) * (d - cb)
-    }
-}
+/// Pegtop's Soft Light, `(1 - 2 cs) cb^2 + 2 cs cb`: what Compositor for Mac 1.2.10 draws. The
+/// blend-greys probe fits it within 1 level at every grey and alpha, where the W3C / PDF formula is
+/// 14 levels off at a 75% grey source (probe results, "Phase 3.5b follow-up probes").
+fn soft_light(cb: f32, cs: f32) -> f32 { (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb }
 
 /// PDF separable blend function B(cb, cs) on straight (unpremultiplied) channel values.
 pub fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
```

```diff
--- a/app/src/canvas/gl/programs.ts
+++ b/app/src/canvas/gl/programs.ts
@@ -50,11 +50,7 @@ float sep(int mode, float cb, float cs) {
   if (mode == 8) return cb >= 1.0 ? 1.0 : (cs <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / cs));
   if (mode == 13) return max(0.0, cb + cs - 1.0);
   if (mode == 14) return min(1.0, cb + cs);
-  if (mode == 15) {
-    if (cs <= 0.5) return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
-    float d = cb <= 0.25 ? ((16.0 * cb - 12.0) * cb + 4.0) * cb : sqrt(cb);
-    return cb + (2.0 * cs - 1.0) * (d - cb);
-  }
+  if (mode == 15) return (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb;   // Pegtop, as blend.rs soft_light
   if (mode == 16) { if (cs <= 0.5) return cb * 2.0 * cs; float s = 2.0 * cs - 1.0; return cb + s - cb * s; }
   if (mode == 17) {
     if (cs <= 0.5) { float s = 2.0 * cs; return cb >= 1.0 ? 1.0 : (s <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / s)); }
```

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test mac_1_2_10 --test blend_modes_v9` (mac_1_2_10: 13 tests), then the whole engine suite: 390 passed, 1 ignored (+8). `pnpm wasm:dev`, then `npx playwright test app/tests/e2e/mac-1.2.10.spec.ts` (the new blend-greys test passes).

- [ ] **Step 5: Prove it bites**

(1) Put W3C's Soft Light back in `blend.rs` (`cs <= 0.5 ? cb - (1 - 2cs) cb (1 - cb) : cb + (2cs - 1)(d(cb) - cb)`): `every_band_of_blend_greys_matches_the_mac_render` (Soft Light 14 off in band 0) and `blend_modes_v9.rs`'s `each_new_mode_composites_by_its_formula_over_an_opaque_backdrop` fail. Restore. (2) Put it back in GLSL mode 15 only (TypeScript: no wasm rebuild): the e2e blend-greys test fails at 14 (measured on the scratch copy). Restore. (3) Loosen nothing: change one pin to 0 where it measured 1 (`effects-stroke-inside`): it fails at 1. Restore.

- [ ] **Step 6: Commit**

```
$probes = @("add-noise-gaussian-mono", "add-noise-uniform", "black-white-tint", "blend-greys", "blur-soft-mask",
  "cgmode-blur-linear-burn", "cgmode-levels-divide", "cgmode-stack-bases", "color-balance-no-preserve", "color-balance-preserve",
  "color-dodge-adjustment", "effects-all-six", "effects-clipped-child", "effects-clipping-base", "effects-color-overlay",
  "effects-drop-shadow", "effects-folder", "effects-inner-glow", "effects-inner-shadow", "effects-invalid", "effects-large-blur",
  "effects-mask-placed", "effects-masked", "effects-outer-glow", "effects-stroke-inside", "effects-stroke-outside",
  "effects-transformed", "gaussian-blur-40", "gaussian-blur-6", "invert", "motion-blur-30-24")
$fixtures = $probes | ForEach-Object { "engine/tests/fixtures/mac-1.2.10-probes/$_.comp"; "engine/tests/fixtures/mac-1.2.10-probes/$_.mac-1.2.10.png" }
git add -- $fixtures
git commit -- $fixtures engine/src/blend.rs app/src/canvas/gl/programs.ts engine/tests/mac_1_2_10.rs engine/tests/blend_modes_v9.rs app/tests/e2e/mac-1.2.10.spec.ts -m "test: pin the Mac 1.2.10 follow-up and effects probes; Soft Light is Pegtop's" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

`git status` afterwards still lists the seven `sampling-*` pairs as untracked, and nothing else.

---

### Task 2: Motion Blur is CIMotionBlur's Gaussian

The Mac passes Core Image's `CIMotionBlur` a radius of distance / sqrt(12) (Filters.swift:170-208, for the filter and the adjustment layer alike), and the `motion-blur-30-24` probe shows a Gaussian along the angle with that sigma, not the even streak this port draws (probe results: the streak is 26 colour and 28 alpha off; the Gaussian 6 and 6). This task makes Motion Blur that Gaussian: one kernel for the destructive filter, the adjustment layer and the GPU (ruling OQ16). It reaches three sigmas, so it halves past the same `SPATIAL_REACH_LIMIT` as the Gaussian Blur, and the separate `MOTION_REACH_LIMIT = 12` goes. The notice entry and the merge refusal the streak needed go too. The canvas-edge rule (the blur fades at the canvas edge, ruling E-I1) and the probe generator (`mac_probes.rs`) do not change. The destructive filter keeps the Mac's padding, distance / 2 + 2, because the Mac's filter clips there.

On the CPU the kernel is a precomputed stencil: every tap's bilinear shares, summed per pixel offset, applied a row at a time over a row of f32, which equals applying the taps one at a time within float ordering (measured: at most 1 level on 2 of 23,668 bytes). A per-tap loop took 8.7 s at 13 px on 6 Mpx native; the stencil 2.5 s.

**Files:**
- Modify: `engine/src/adjust/filters.rs` (`motion_sigma`, `motion_reach`, `motion_stencil`, `motion_blur`), `engine/src/adjust/spatial.rs` (levels), `engine/src/adjust/settings.rs` (`sampling_margin`), `engine/src/document.rs` (`undrawn_features`), `app/src/canvas/gl/programs.ts` (`FRAG_MOTION`), `app/src/canvas/gl-renderer.ts` (its uniforms)
- Modify tests: `engine/tests/filters.rs`, `engine/tests/spatial_adjustments.rs`, `engine/tests/undrawn.rs`, `engine/tests/merge_undrawn.rs`, `engine/tests/peak_heap.rs`, `engine/tests/mac_1_2_10.rs`, `app/tests/e2e/mac-1.2.10.spec.ts`, `app/tests/e2e/mac-1.2.6.spec.ts`

**Interfaces:**
- Consumes: Task 1's `premultiplied`.
- Produces: `pub fn motion_sigma(distance: f64) -> f64` (distance / sqrt(12)), `pub fn motion_reach(distance: f64) -> f64` (3 sigma), `pub fn motion_stencil(angle: f64, distance: f64) -> (Vec<(i64, i64, f32)>, f32)` (offsets and weights, and their sum), all re-exported through `adjust::filters::*`. `SpatialBlur.sigma` is now the Motion Blur's sigma too (the GPU reads it); `MOTION_REACH_LIMIT` is removed.

- [ ] **Step 1: Write the failing tests**

The kernel against the formula, the stencil against a tap-at-a-time transcription, the level and reach:

```diff
--- a/engine/tests/filters.rs
+++ b/engine/tests/filters.rs
@@ -35,10 +35,16 @@ fn motion_blur_streaks_along_its_angle_counterclockwise_from_horizontal() {
     // 45 degrees runs up-right and down-left on screen, never up-left.
     let diagonal = motion_blur(&dot, 45.0, 16.0);
     assert!(alpha(&diagonal, 23, 17) > 0 && alpha(&diagonal, 17, 23) > 0 && alpha(&diagonal, 17, 17) == 0);
-    // An even streak: the dot's alpha is spread over about `distance` pixels, not tapered to a point.
-    let total: i64 = (0..41).map(|x| alpha(&horizontal, x, 20)).sum();
-    assert!((total - 255).abs() <= 8, "energy is preserved: {total}");
-    assert!(alpha(&horizontal, 20, 20) < 40, "no spike at the centre");
+    // CIMotionBlur's taper (probe results): along the row the dot's alpha follows a Gaussian of
+    // sigma 16 / sqrt(12), cut at ceil(3 sigma) = 14, each column its tap's share of the weights.
+    // An even 16-px streak would give 16 columns of 16 each.
+    let sigma = 16.0 / 12f64.sqrt();
+    let weight = |t: i64| (-((t * t) as f64) / (2.0 * sigma * sigma)).exp();
+    let total: f64 = (-14..=14).map(weight).sum();
+    for x in 0..41i64 {
+        let want = if (x - 20).abs() <= 14 { 255.0 * weight(x - 20) / total } else { 0.0 };
+        assert!((alpha(&horizontal, x as u32, 20) as f64 - want).abs() <= 1.0, "column {x}: {} vs {want:.2}", alpha(&horizontal, x as u32, 20));
+    }
 }
 
 #[test]
@@ -230,10 +236,59 @@ fn a_blur_layer_written_over_its_input_equals_the_full_frame_path_to_the_bit() {
         let expected = full_frame_halved(&source, level, |r| full_frame_gaussian(r, sigma / (1u32 << level) as f64));
         assert_eq!(blur_for_layer(source.clone(), sigma).bytes(), expected.bytes(), "Gaussian at level {level}");
     }
-    // A Motion Blur halves past a reach of 12: 40 px is level 1, 100 px level 3.
-    for (distance, level) in [(40.0, 1), (100.0, 3)] {
-        assert_eq!(motion_level(distance / 2.0), level);
+    // A Motion Blur reaches three sigmas and halves past the same 48: 100 px (86.6) is level 1,
+    // 250 px (216.5) level 3.
+    for (distance, level) in [(100.0, 1), (250.0, 3)] {
+        assert_eq!(spatial_level(motion_reach(distance)), level);
         let expected = full_frame_halved(&source, level, |r| motion_blur(r, 30.0, distance / (1u32 << level) as f64));
         assert_eq!(streak_for_layer(source.clone(), 30.0, distance).bytes(), expected.bytes(), "Motion Blur at level {level}");
     }
+}
+
+/// The Motion Blur as CIMotionBlur's kernel reads, one tap at a time: `2 ceil(3 sigma) + 1` taps
+/// along (cos a, -sin a), each a bilinear sample (zero beyond the raster) weighted
+/// exp(-t^2 / 2 sigma^2), sigma = distance / sqrt(12). What the GPU's FRAG_MOTION runs, and the
+/// reference the row-wise stencil must equal.
+fn per_tap_motion(raster: &Raster, angle: f64, distance: f64) -> Raster {
+    let sigma = distance / 12f64.sqrt();
+    let radius = (sigma * 3.0).ceil() as i64;
+    let (dx, dy) = (angle.to_radians().cos(), -angle.to_radians().sin());
+    let fetch = |x: i64, y: i64| -> [f32; 4] {
+        if x < 0 || y < 0 || x >= raster.width as i64 || y >= raster.height as i64 { return [0.0; 4]; }
+        raster.pixel(x as u32, y as u32).map(|v| v as f32)
+    };
+    let kernel: Vec<f32> = (-radius..=radius).map(|t| (-((t * t) as f64) / (2.0 * sigma * sigma)).exp() as f32).collect();
+    let sum: f32 = kernel.iter().sum();
+    let mut out = vec![0u8; (raster.width * raster.height * 4) as usize];
+    for y in 0..raster.height { for x in 0..raster.width {
+        let mut acc = [0f32; 4];
+        for (k, weight) in kernel.iter().enumerate() {
+            let t = (k as i64 - radius) as f64;
+            let (fx, fy) = (x as f64 + dx * t, y as f64 + dy * t);
+            let (x0, y0) = (fx.floor(), fy.floor());
+            let (tx, ty) = ((fx - x0) as f32, (fy - y0) as f32);
+            let (a, b) = (fetch(x0 as i64, y0 as i64), fetch(x0 as i64 + 1, y0 as i64));
+            let (c, d) = (fetch(x0 as i64, y0 as i64 + 1), fetch(x0 as i64 + 1, y0 as i64 + 1));
+            for i in 0..4 { acc[i] += ((a[i] * (1.0 - tx) + b[i] * tx) * (1.0 - ty) + (c[i] * (1.0 - tx) + d[i] * tx) * ty) * weight; }
+        }
+        let i = ((y * raster.width + x) * 4) as usize;
+        let alpha = (acc[3] / sum).round().clamp(0.0, 255.0);
+        out[i + 3] = alpha as u8;
+        for c in 0..3 { out[i + c] = (acc[c] / sum).round().clamp(0.0, alpha) as u8; }
+    }}
+    Raster::from_premultiplied(raster.width, raster.height, out)
+}
+
+#[test]
+fn the_row_wise_motion_stencil_equals_one_tap_at_a_time_within_float_ordering() {
+    // The stencil regroups the same products in another order. Measured on p4a-scratch
+    // (2026-09-27): 2 of 23668 bytes one level apart at 30 degrees / 24 px (a sum landing on .5),
+    // every other case identical. Angles on and off the axes, lengths from a no-op to a long reach.
+    let source = noisy(97, 61, 7);
+    for (angle, distance) in [(30.0, 24.0), (0.0, 13.0), (90.0, 7.0), (-60.0, 40.0), (45.0, 1.0), (12.5, 33.0)] {
+        let (a, b) = (motion_blur(&source, angle, distance), per_tap_motion(&source, angle, distance));
+        let worst = a.bytes().iter().zip(b.bytes()).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
+        let count = a.bytes().iter().zip(b.bytes()).filter(|(x, y)| x != y).count();
+        assert!(worst <= 1 && count <= 2, "{angle} degrees, {distance} px: worst {worst} over {count} bytes");
+    }
 }
\ No newline at end of file
```

The halving follows 3 sigma: the lattice and pad of a 150 px blur, the measured bounds of a halved Motion Blur against the exact one (re-measured here: 1 inside, 9 at the canvas edge, 19 over pixel noise; the streak measured 2, 17 and 20), and the reach at which it first halves (55 px exact, 56 px one halving):

```diff
--- a/engine/tests/spatial_adjustments.rs
+++ b/engine/tests/spatial_adjustments.rs
@@ -133,7 +133,8 @@ fn a_part_of_the_canvas_composites_exactly_as_that_part_of_the_whole() {
         let part = composite(&doc, Rect { x: x as f64, y: y as f64, width: w as f64, height: h as f64 }, w, h);
         assert_eq!(part.bytes(), whole.cropped(x, y, w, h).bytes(), "({x}, {y}) {w} x {h}");
     }
-    assert_eq!(render_plan(&doc, None).spatial_margin, (2.0 * 3.0 + 2.0) + (7.0 / 2.0 + 2.0));
+    // The streak pads by its three sigmas (7 / sqrt(12) each) plus 2, as far as its kernel reads.
+    assert_eq!(render_plan(&doc, None).spatial_margin, (2.0 * 3.0 + 2.0) + (7.0 / 12f64.sqrt() * 3.0 + 2.0));
     let mut hidden = doc.clone();
     hidden.layers[2].visible = false;
     assert_eq!(render_plan(&hidden, None).spatial_margin, 2.0 * 3.0 + 2.0, "a hidden blur reaches nothing");
@@ -179,9 +180,12 @@ fn the_grid_and_the_span_follow_the_plans_blurs() {
     assert_eq!(spatial_grid(&render_plan(&one, None), 0.25), SpatialGrid { cell: 1, pad: 19 });
     let mut two = one.clone();
     let second = streak(&two, 30.0, 150.0); two.layers.push(second);
-    // Stacked blurs compound: 68 + (150 / 2 + 2 + 3 x 8) = 169, the streak's reach of 75 halving
-    // three times past MOTION_REACH_LIMIT; the cell is the larger level's.
-    assert_eq!(spatial_grid(&render_plan(&two, None), 1.0), SpatialGrid { cell: 8, pad: 169 });
+    // Stacked blurs compound: 68 + (motion_reach(150) + 2 + 3 x 4) = 211.9, rounded up; the streak's
+    // reach of 129.9 (three sigmas of 150 / sqrt(12)) halves twice past SPATIAL_REACH_LIMIT, and
+    // the cell is the larger level's.
+    let reach = motion_reach(150.0);
+    assert_eq!(spatial_level(reach), 2);
+    assert_eq!(spatial_grid(&render_plan(&two, None), 1.0), SpatialGrid { cell: 4, pad: (68.0 + reach + 2.0 + 12.0).ceil() as u32 });
     let mut far = one.clone();
     far.layers[1].extra.adjustment.as_mut().unwrap().blur_radius = Some(250.0);
     // At 2 output pixels per document pixel, radius 250 reaches 1500: five halvings, and the pad
@@ -282,9 +286,9 @@ fn a_long_reach_blurs_a_halved_copy_within_the_measured_bound_of_the_exact_kerne
     // the canvas-anchored lattice (plan-fix scratch crate p35b-halving, 2026-09-24), not chosen;
     // the assertion allows 1 more for float ordering. `inset` keeps the comparison reach + 2 cells
     // away from every canvas edge (0: the whole canvas). Radius 20 reaches 60 output pixels (one
-    // halving), radius 40 reaches 120 (two), a 150 px streak reaches 75 (three: a Motion Blur
-    // halves past MOTION_REACH_LIMIT, 12). The streak bounds were re-measured here for that limit
-    // (Task 4 fix round 1, 2026-09-25): at one halving they were 2 and 15.
+    // halving), radius 40 reaches 120 (two). A 150 px Motion Blur reaches three sigmas, 129.9 (two
+    // halvings, as a Gaussian would: Phase 4a's kernel, bounds re-measured on p4a-scratch
+    // 2026-09-27; the even streak before it halved three times, and measured 2, 17 and 20).
     let cases: [(&str, Document, u32, u8); 9] = [
         ("the interior of an opaque ramp, radius 20", over(ramp(200, 150, None), |d| blur(d, 20.0)), 64, 1),
         ("the same ramp out to its canvas edge, radius 20", over(ramp(200, 150, None), |d| blur(d, 20.0)), 0, 3),
@@ -292,11 +296,11 @@ fn a_long_reach_blurs_a_halved_copy_within_the_measured_bound_of_the_exact_kerne
         ("an alpha edge off the halving lattice (row 7), radius 20", over(ramp(96, 64, Some(7)), |d| blur(d, 20.0)), 0, 3),
         ("odd sizes, 95 x 63, alpha edge at row 7, radius 20", over(ramp(95, 63, Some(7)), |d| blur(d, 20.0)), 0, 3),
         ("odd sizes, radius 40", over(ramp(95, 63, Some(7)), |d| blur(d, 40.0)), 0, 2),
-        // Three halvings put most of a streak's error at the canvas edge, where the streak fades.
-        ("the interior of an opaque ramp under a 150 px streak", over(ramp(400, 300, None), |d| streak(d, -60.0, 150.0)), 91, 2),
-        ("a 150 px streak over the odd-sized ramp", over(ramp(95, 63, Some(7)), |d| streak(d, -60.0, 150.0)), 0, 17),
-        // A halved streak also averages the detail ACROSS the streak, which the exact one keeps.
-        ("a 150 px streak over per-pixel noise", over(lcg_noise(200, 150), |d| streak(d, 30.0, 150.0)), 0, 20),
+        // Most of a halved Motion Blur's error lies at the canvas edge, where the blur fades.
+        ("the interior of an opaque ramp under a 150 px Motion Blur", over(ramp(400, 300, None), |d| streak(d, -60.0, 150.0)), 138, 1),
+        ("a 150 px Motion Blur over the odd-sized ramp", over(ramp(95, 63, Some(7)), |d| streak(d, -60.0, 150.0)), 0, 9),
+        // A halved Motion Blur also averages the detail ACROSS its angle, which the exact one keeps.
+        ("a 150 px Motion Blur over per-pixel noise", over(lcg_noise(200, 150), |d| streak(d, 30.0, 150.0)), 0, 19),
     ];
     for (name, doc, inset, measured) in cases {
         let (under, out) = (beneath(&doc), full(&doc));
@@ -316,19 +320,18 @@ fn a_long_reach_blurs_a_halved_copy_within_the_measured_bound_of_the_exact_kerne
 }
 
 #[test]
-fn a_motion_blur_layer_halves_past_its_own_shorter_reach() {
-    // Task 4 fix round 1. The exact streak costs one tap per pixel of its length (a 90 px streak
-    // over 3000 x 2000 took 45 s in the release wasm), and Motion Blur layers are drawn
-    // approximately anyway, so they halve past MOTION_REACH_LIMIT (12 output pixels). A Gaussian
-    // keeps SPATIAL_REACH_LIMIT (48).
-    let doc = patterned(|d| vec![streak(d, 30.0, 30.0)]);
-    assert_eq!(spatial_blur(doc.layers[1].extra.adjustment.as_ref().unwrap(), 1.0).level, 1, "a 30 px streak reaches 15");
-    assert_eq!([motion_level(12.0), motion_level(12.5), motion_level(24.5), motion_level(45.0)], [0, 1, 2, 2]);
-    let mut gaussian = LayerAdjustment::new(AdjustmentKind::GaussianBlur);
-    gaussian.blur_radius = Some(5.0);
-    assert_eq!(spatial_blur(&gaussian, 1.0).level, 0, "a Gaussian reaching 15 stays exact");
-    // The compositor draws the streak halved too: the exact kernel would match it to the bit.
-    assert_ne!(full(&doc).bytes(), motion_blur(&beneath(&doc), 30.0, 30.0).bytes());
+fn a_motion_blur_layer_halves_past_the_same_reach_as_a_gaussian() {
+    // Phase 4a: the Motion Blur is CIMotionBlur's Gaussian along its angle, sigma = distance /
+    // sqrt(12), reaching three sigmas, and runs as a row-wise stencil cheap enough to share the
+    // Gaussian's SPATIAL_REACH_LIMIT (48). A 55 px blur reaches 47.6 (exact); 56 px reaches 48.5
+    // (one halving).
+    let exact = patterned(|d| vec![streak(d, 30.0, 55.0)]);
+    let b = spatial_blur(exact.layers[1].extra.adjustment.as_ref().unwrap(), 1.0);
+    assert_eq!((b.level, b.sigma, b.distance, b.angle), (0, 55.0 / 12f64.sqrt(), 55.0, 30.0));
+    assert_eq!(full(&exact).bytes(), motion_blur(&beneath(&exact), 30.0, 55.0).bytes(), "the exact kernel on the composite");
+    let halved = patterned(|d| vec![streak(d, 30.0, 56.0)]);
+    assert_eq!(spatial_blur(halved.layers[1].extra.adjustment.as_ref().unwrap(), 1.0).level, 1);
+    assert_ne!(full(&halved).bytes(), motion_blur(&beneath(&halved), 30.0, 56.0).bytes(), "drawn from a halved copy");
 }
 
 #[test]
```

The notice and merge:

```diff
--- a/engine/tests/undrawn.rs
+++ b/engine/tests/undrawn.rs
@@ -19,10 +19,7 @@ fn each_undrawn_feature_is_named_once_and_sorted() {
     let mut d = pixel_layer(); d.extra.unknown.insert("fromTheFuture".into(), json!(1));
     let mut e = Layer::blank("Streak", doc.size()); e.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::MotionBlur));
     doc.layers = vec![a, b, c, d, e];
-    assert_eq!(doc.undrawn(), vec![
-        "Motion Blur adjustment layers (drawn approximately)".to_string(),
-        "settings from a newer version of Compositor".to_string(),
-    ]);
+    assert_eq!(doc.undrawn(), vec!["settings from a newer version of Compositor".to_string()]);
 }
 
 #[test]
@@ -36,17 +33,16 @@ fn layer_effects_are_drawn_so_they_are_not_reported() {
 }
 
 #[test]
-fn of_the_six_new_kinds_only_motion_blur_is_reported() {
-    // Ruling G-I2: this port draws Motion Blur as an even streak, Core Image as a tapered one
-    // (Filters.swift:170-208), and the motion probe has not measured the gap yet.
+fn none_of_the_six_new_kinds_is_reported() {
+    // Phase 4a: the Motion Blur is drawn as CIMotionBlur's Gaussian, within 6 levels of the Mac's
+    // render (mac_1_2_10.rs), so it left the notice with the rest (ruling G-I2 closed).
     for kind in [AdjustmentKind::Invert, AdjustmentKind::BlackWhite, AdjustmentKind::ColorBalance, AdjustmentKind::GaussianBlur,
         AdjustmentKind::MotionBlur, AdjustmentKind::AddNoise] {
         let mut doc = Document::new(4, 4);
         let mut layer = Layer::blank("A", doc.size());
         layer.extra.adjustment = Some(LayerAdjustment::new(kind));
         doc.layers = vec![layer];
-        let want: Vec<String> = if kind == AdjustmentKind::MotionBlur { vec!["Motion Blur adjustment layers (drawn approximately)".into()] } else { vec![] };
-        assert_eq!(doc.undrawn(), want, "{kind:?}");
+        assert!(doc.undrawn().is_empty(), "{kind:?}: {:?}", doc.undrawn());
     }
 }
 
```

```diff
--- a/engine/tests/merge_undrawn.rs
+++ b/engine/tests/merge_undrawn.rs
@@ -22,19 +22,25 @@ fn merge_down_bakes_a_new_blend_mode_as_the_canvas_shows_it() {
 }
 
 #[test]
-fn merging_a_folder_holding_a_motion_blur_layer_is_refused_until_the_motion_probe_settles_it() {
-    let mut doc = Document::new(4, 4);
+fn merging_a_folder_holding_a_motion_blur_layer_bakes_what_the_canvas_showed() {
+    // Phase 4a: the Motion Blur is drawn as the Mac draws it (mac_1_2_10.rs), so merge no longer
+    // refuses it. A 6 x 4 block on 12 x 8, blurred 10 px at 30 degrees, so the blur spreads past it.
+    let mut doc = Document::new(12, 8);
     let mut folder = Layer::blank("Folder", doc.size()); folder.is_group = true;
     let folder_id = folder.id;
-    let mut p = Layer::with_pixels("P", Raster::from_premultiplied(4, 4, [200u8, 40, 40, 255].repeat(16)), Point { x: 0.0, y: 0.0 });
+    let mut p = Layer::with_pixels("P", Raster::from_premultiplied(6, 4, [200u8, 40, 40, 255].repeat(24)), Point { x: 3.0, y: 2.0 });
     p.parent_id = Some(folder_id);
     let mut streak = Layer::blank("Streak", doc.size()); streak.parent_id = Some(folder_id);
-    streak.extra.adjustment = Some(LayerAdjustment::new(AdjustmentKind::MotionBlur));
+    let mut a = LayerAdjustment::new(AdjustmentKind::MotionBlur);
+    a.motion_angle = Some(30.0); a.motion_distance = Some(10.0);
+    streak.extra.adjustment = Some(a);
     doc.layers = vec![folder, p, streak];
     doc.active_layer_id = Some(folder_id);
-    let err = ops::merge::merge(&mut doc, &[folder_id]).unwrap_err();
-    assert_eq!(err, CommandError::Argument("Merging would bake Motion Blur adjustment layers (drawn approximately), which this build does not draw yet, or draws differently".into()));
-    assert_eq!(doc.layers.len(), 3, "nothing changed");
+    let shown = composite(&doc, Rect { x: 0.0, y: 0.0, width: 12.0, height: 8.0 }, 12, 8);
+    ops::merge::merge(&mut doc, &[folder_id]).expect("a drawn Motion Blur no longer blocks a merge");
+    assert_eq!(doc.layers.len(), 1);
+    assert_eq!(composite(&doc, Rect { x: 0.0, y: 0.0, width: 12.0, height: 8.0 }, 12, 8).bytes(), shown.bytes(), "the merged pixels are what the canvas showed");
+    assert!(doc.layers[0].pixels.as_ref().unwrap().width > 6, "the blur spread past the block, and the merge kept it");
 }
 
 #[test]
```

Peak heap, re-measured for the stencil (12.06 bytes a pixel at level 0, 9.66 at level 2):

```diff
--- a/engine/tests/peak_heap.rs
+++ b/engine/tests/peak_heap.rs
@@ -150,13 +150,16 @@ fn a_halved_gaussian_blur_layer_enlarges_over_its_working_copy() {
 
 #[test]
 fn a_level_0_motion_blur_layer_holds_a_working_copy_and_its_result() {
-    // The streak reads in every direction, so it cannot write over what it still reads. Unchanged.
-    at_most("composite, Motion 20 (level 0)", || composite_peak(Some(motion(20.0))), 12.01);
+    // The kernel reads rows above and below, so it cannot write over what it still reads; its
+    // accumulator is one row of f32 RGBA.
+    assert_eq!(spatial_blur(&motion(20.0), 1.0).level, 0);
+    at_most("composite, Motion 20 (level 0)", || composite_peak(Some(motion(20.0))), 12.06);
 }
 
 #[test]
 fn a_halved_motion_blur_layer_enlarges_over_its_working_copy() {
-    at_most("composite, Motion 60 (level 2)", || composite_peak(Some(motion(60.0))), 9.63); // was 13.68
+    assert_eq!(spatial_blur(&motion(120.0), 1.0).level, 2);
+    at_most("composite, Motion 120 (level 2)", || composite_peak(Some(motion(120.0))), 9.66);
 }
 
 #[test]
```

The Mac's render:

```diff
--- a/engine/tests/mac_1_2_10.rs
+++ b/engine/tests/mac_1_2_10.rs
@@ -193,6 +193,19 @@ fn every_band_of_blend_greys_matches_the_mac_render() {
     }
 }
 
+#[test]
+fn the_motion_blur_probe_matches_the_mac_render_as_a_gaussian_along_its_angle() {
+    // 30 degrees, 24 px over a block running off the canvas and a translucent bar. The Mac passes
+    // CIMotionBlur a radius of 24 / sqrt(12) and CIMotionBlur is a Gaussian of that sigma along the
+    // angle (Filters.swift:170-208; probe results). Measured with `motion_blur` on p4a-scratch
+    // (2026-09-27): colour 6, alpha 6, mean 0.195 per byte. The even streak this port drew before
+    // measured 26, 28 and 3.108; a Gaussian cut at 4 sigmas instead of 3 was 6, 6 and 0.138, and one
+    // sampled every quarter pixel 4, 4 and 0.103, at 4 to 8 times the taps.
+    let (colour, alpha, mean) = premultiplied("motion-blur-30-24");
+    assert!(colour <= 6 && alpha <= 6, "colour {colour} alpha {alpha}");
+    assert!(mean <= 0.2, "mean {mean:.3}");
+}
+
 // The Phase 3.5c effects probes and color-balance-preserve, exported by Compositor for Mac 1.2.10 on
 // 2026-09-27 (probe results, "Phase 3.5c effects probes and color-balance-preserve"). Measured
 // premultiplied on p4a-scratch against this build (2026-09-27); the same as the controller's
```

On the GPU: the blur sizes the engine hands it, the closed-form step edge now one formula for both kernels, and the probe on the GPU (measured 6 on the release wasm, the CPU 6):

```diff
--- a/app/tests/e2e/mac-1.2.10.spec.ts
+++ b/app/tests/e2e/mac-1.2.10.spec.ts
@@ -130,7 +130,7 @@ test("the engine hands the GPU its blur sizes, the plan's reach and the canvas-a
       api.engine.spatialBlur(layer.adjustment, 2),                                  // radius absent: 10, so sigma 20, reach 60
       api.engine.spatialBlur({ ...layer.adjustment, blurRadius: 16 }, 1),           // reach 48, the limit: no halving
       api.engine.spatialBlur({ ...layer.adjustment, blurRadius: 250 }, 1),          // reach 750: four halvings
-      api.engine.spatialBlur({ ...layer.adjustment, kind: "Motion Blur", motionAngle: 30, motionDistance: 97 }, 1),   // reach 48.5, halved 3x past the Motion Blur limit of 12 (48.5 -> 24.25 -> 12.125 -> 6.0625)
+      api.engine.spatialBlur({ ...layer.adjustment, kind: "Motion Blur", motionAngle: 30, motionDistance: 97 }, 1),   // sigma 97 / sqrt(12), reach 84: one halving
     ];
     api.engine.execute(doc, { type: "SetAdjustment", id: layer.id, adjustment: { ...layer.adjustment, blurRadius: 6 } });
     return {
@@ -143,7 +143,7 @@ test("the engine hands the GPU its blur sizes, the plan's reach and the canvas-a
   expect(r.after, "3 x 6 + 2 document pixels").toBe(20);
   expect(r.blurs).toEqual([
     { level: 1, sigma: 20, distance: 0, angle: 0 }, { level: 0, sigma: 16, distance: 0, angle: 0 },
-    { level: 4, sigma: 250, distance: 0, angle: 0 }, { level: 3, sigma: 0, distance: 97, angle: 30 },
+    { level: 4, sigma: 250, distance: 0, angle: 0 }, { level: 1, sigma: 97 / Math.sqrt(12), distance: 97, angle: 30 },
   ]);
   // Radius 6 at 1 output px per document px: reach 18, no halving, pad 20 + 3 cells of 1. At 4:
   // reach 72, one halving, pad 80 + 3 x 2. At 100: six halvings, and the pad stops at 1024.
@@ -215,6 +215,15 @@ test("the blend-greys probe draws on the GPU as the Mac exported it, Soft Light
   expect(worstOf(await glPixels(page), await macPixels(page, "blend-greys"))).toBeLessThanOrEqual(1);
 });
 
+test("the motion-blur probe draws on the GPU as the Mac exported it", async ({ page }) => {
+  await openProbe(page, "motion-blur-30-24");
+  expect(await onWholePixels(page), "the document sits on whole device pixels").toBe(true);
+  // The CPU is within 6 of the Mac, premultiplied (mac_1_2_10.rs); the even streak was 28 off.
+  // Measured 6 on p4a-scratch (2026-09-27).
+  expect(worstOf(await glPixels(page), await macPixels(page, "motion-blur-30-24"))).toBeLessThanOrEqual(6);
+  await expectMatchesCpu(page, "motion-blur-30-24");
+});
+
 /** A new adjustment layer of `kind` on the active document, with `settings` merged into it; it becomes the active layer. */
 async function addAdjustment(page: Page, kind: string, settings: object | null): Promise<string> {
   return page.evaluate(({ kind, settings }) => {
@@ -448,12 +457,12 @@ const blurLevel = async (page: Page, id: string) => (await blurOf(page, id)).lev
 test("blur adjustment layers draw on the GPU as on the CPU, reduced or not, dimmed, blended and clipped", async ({ page }) => {
   await setupNoise(page, 721);
   // At one output px per document px: radius 3 reaches 9 px (exact kernel); radius 20 reaches 60,
-  // past SPATIAL_REACH_LIMIT 48 (one halving); a 21-px streak reaches 10.5, within
-  // MOTION_REACH_LIMIT 12 (exact); a 150-px streak reaches 75, which halves to 37.5, 18.75 and
-  // 9.375 (three halvings). A halved case may differ by one more level.
+  // past SPATIAL_REACH_LIMIT 48 (one halving); a 21-px Motion Blur reaches three sigmas, 18.2
+  // (exact); a 150-px one reaches 129.9, which halves to 65 and 32.5 (two halvings). A halved case
+  // may differ by one more level.
   const cases: [string, object, number][] = [
     ["Gaussian Blur", { blurRadius: 3 }, 0], ["Gaussian Blur", { blurRadius: 20 }, 1],
-    ["Motion Blur", { motionAngle: 30, motionDistance: 21 }, 0], ["Motion Blur", { motionAngle: -60, motionDistance: 150 }, 3],
+    ["Motion Blur", { motionAngle: 30, motionDistance: 21 }, 0], ["Motion Blur", { motionAngle: -60, motionDistance: 150 }, 2],
   ];
   for (const [kind, settings, level] of cases) {
     const id = await addAdjustment(page, kind, settings);
@@ -672,7 +681,7 @@ test("a step edge on the lattice blurs on the GPU to the closed form, halved or
   };
   const cases: [string, object, number][] = [
     ["Gaussian Blur", { blurRadius: 20 }, 1],
-    ["Motion Blur", { motionAngle: 0, motionDistance: 150 }, 3],
+    ["Motion Blur", { motionAngle: 0, motionDistance: 150 }, 2],
     ["Motion Blur", { motionAngle: 0, motionDistance: 21 }, 0],
   ];
   for (const [kind, settings, level] of cases) {
@@ -680,22 +689,16 @@ test("a step edge on the lattice blurs on the GPU to the closed form, halved or
     const id = await addAdjustment(page, kind, settings);
     const blur = await blurOf(page, id);
     expect(blur.level, `${label}: the engine's level`).toBe(level);
+    if (kind === "Motion Blur") expect(blur.sigma, `${label}: CIMotionBlur's sigma, distance / sqrt(12)`).toBeCloseTo(blur.distance / Math.sqrt(12), 12);
     const f = 2 ** blur.level, edge = EDGE / f;   // the step's column in the reduced copy
-    let b: (k: number) => number;
-    if (kind === "Gaussian Blur") {
-      // gaussian_blur at the reduced sigma: the share of the kernel's weight on the block. Row 96
-      // lies beyond every vertical reach from the canvas's top and bottom, so that pass keeps it.
-      const s = blur.sigma / f, radius = Math.ceil(s * 3);
-      const w = (j: number) => Math.exp(-(j * j) / (2 * s * s));
-      let total = 0; for (let j = -radius; j <= radius; j++) total += w(j);
-      b = (k) => { let on = 0; for (let j = -radius; j <= radius; j++) if (k + j >= 0 && k + j < edge) on += w(j); return 255 * on / total; };
-    } else {
-      // motion_blur at angle 0 over the reduced copy: an odd count of steps lands every sample on a
-      // texel centre, so each column is the share of the streak's samples on the block.
-      const steps = Math.max(1, Math.round(blur.distance / f)), mid = (steps - 1) / 2;
-      expect(steps % 2, `${label}: an odd streak of ${steps}`).toBe(1);
-      b = (k) => { let on = 0; for (let i = 0; i < steps; i++) { const c = k + i - mid; if (c >= 0 && c < edge) on++; } return 255 * on / steps; };
-    }
+    // Both kernels at the reduced sigma: the share of the Gaussian's weight on the block. The
+    // Gaussian's row pass and a Motion Blur at angle 0 take one tap per whole pixel on texel centres;
+    // row 96 lies beyond every vertical reach from the canvas's top and bottom, so the Gaussian's
+    // column pass keeps it (and a Motion Blur at angle 0 reads no other row).
+    const s = blur.sigma / f, radius = Math.ceil(s * 3);
+    const w = (j: number) => Math.exp(-(j * j) / (2 * s * s));
+    let total = 0; for (let j = -radius; j <= radius; j++) total += w(j);
+    const b = (k: number) => { let on = 0; for (let j = -radius; j <= radius; j++) if (k + j >= 0 && k + j < edge) on += w(j); return 255 * on / total; };
     const r = await page.evaluate(async () => {
       const api = (window as any).__compositor; const s = api.store.getState();
       await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
```

```diff
--- a/app/tests/e2e/mac-1.2.6.spec.ts
+++ b/app/tests/e2e/mac-1.2.6.spec.ts
@@ -249,7 +249,7 @@ test("a notice names what the project uses that this build does not draw, and Di
   await expect(notice).toBeVisible();
   await expect(notice, "layer effects are drawn now").not.toContainText("layer effects");
   await expect(notice).toContainText("settings from a newer version of Compositor");
-  await expect(notice).toContainText("Motion Blur adjustment layers (drawn approximately)");
+  await expect(notice, "Motion Blur is drawn as the Mac draws it now (Phase 4a)").not.toContainText("Motion Blur");
   // Scoped to the notice: the error banner's button (App.tsx:98) is also named "Dismiss".
   await notice.getByRole("button", { name: "Dismiss", exact: true }).click();
   await expect(notice).toHaveCount(0);
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test filters --test spatial_adjustments --test undrawn --test merge_undrawn --test peak_heap --test mac_1_2_10`: `filters.rs` and `spatial_adjustments.rs` do not compile (`motion_sigma`, `motion_reach`, `motion_stencil` do not exist yet). Leave those two out for a moment (`--test undrawn --test merge_undrawn --test peak_heap --test mac_1_2_10 --no-fail-fast`) and five fail as the streak draws: `the_motion_blur_probe_matches_the_mac_render_as_a_gaussian_along_its_angle` (26 > 6), `none_of_the_six_new_kinds_is_reported` and `each_undrawn_feature_is_named_once_and_sorted` (Motion Blur is still named), `merging_a_folder_holding_a_motion_blur_layer_bakes_what_the_canvas_showed` (refused) and `a_halved_motion_blur_layer_enlarges_over_its_working_copy` (the streak halves at another level). Checked on the scratch copy.

- [ ] **Step 3: The Gaussian, on the CPU and the GPU**

```diff
--- a/engine/src/adjust/filters.rs
+++ b/engine/src/adjust/filters.rs
@@ -143,29 +143,75 @@ pub fn gaussian_blur_in_place(data: &mut [u8], width: u32, height: u32, sigma: f
     }
 }
 
-/// An even streak of `distance` pixels along `angle` degrees, counter-clockwise from horizontal on
-/// screen (so the direction in top-down pixels is (cos a, -sin a)), as Photoshop smears.
-pub fn motion_blur(raster: &Raster, angle: f64, distance: f64) -> Raster {
-    let steps = distance.round().max(1.0) as i64;
-    if steps <= 1 { return raster.clone(); }
+/// The spread of a Motion Blur `distance` pixels long. The Mac hands CIMotionBlur a radius of
+/// distance / sqrt(12) (`motionRadiusPerPixel`, Filters.swift:170-173; the adjustment layer does
+/// the same), and CIMotionBlur is a Gaussian along the angle whose sigma is that radius (the
+/// motion-blur-30-24 probe, probe results "Phase 3.5b follow-up probes").
+pub fn motion_sigma(distance: f64) -> f64 { distance / 12f64.sqrt() }
+
+/// How far a Motion Blur `distance` pixels long reads along its angle: three sigmas, as far as
+/// `motion_blur` takes taps.
+pub fn motion_reach(distance: f64) -> f64 { motion_sigma(distance) * 3.0 }
+
+/// The Motion Blur kernel as whole-pixel cells: (column offset, row offset, weight), each weight a
+/// Gaussian tap's `exp(-t^2 / 2 sigma^2)` times its bilinear share, cells that several taps reach
+/// summed, rows then columns ascending; and the taps' weight sum. One tap per whole pixel `t` out
+/// to ceil(3 sigma) either side along (cos a, -sin a). A tap at (x + 0.5 + dx t, y + 0.5 + dy t)
+/// splits over the four pixels around it by the same fractions wherever (x, y) is, which is why
+/// the kernel is a fixed stencil. Empty for a blur too short to move anything.
+pub fn motion_stencil(angle: f64, distance: f64) -> (Vec<(i64, i64, f32)>, f32) {
+    let sigma = motion_sigma(distance);
+    let radius = (sigma * 3.0).ceil() as i64;
+    if !(sigma > 0.0) || radius < 1 { return (Vec::new(), 0.0); }
     let radians = angle.to_radians();
     let (dx, dy) = (radians.cos(), -radians.sin());
-    let half = (steps - 1) as f64 / 2.0;
-    let (w, h) = (raster.width, raster.height);
-    let mut out = vec![0u8; (w as usize) * (h as usize) * 4];
-    for y in 0..h { for x in 0..w {
-        let mut acc = [0f32; 4];
-        for i in 0..steps {
-            let t = i as f64 - half;
-            let s = sample_zero(raster, x as f64 + 0.5 + dx * t, y as f64 + 0.5 + dy * t);
-            for c in 0..4 { acc[c] += s[c]; }
+    let mut cells: std::collections::BTreeMap<(i64, i64), f64> = std::collections::BTreeMap::new();
+    let mut sum = 0.0f32;
+    for t in -radius..=radius {
+        let weight = (-((t * t) as f64) / (2.0 * sigma * sigma)).exp() as f32;
+        sum += weight;
+        let (fx, fy) = (dx * t as f64, dy * t as f64);
+        let (ox, oy) = (fx.floor(), fy.floor());
+        let (tx, ty) = (fx - ox, fy - oy);
+        for (cx, cy, share) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
+            if share > 0.0 { *cells.entry((oy as i64 + cy, ox as i64 + cx)).or_insert(0.0) += weight as f64 * share; }
         }
-        let i = ((y * w + x) * 4) as usize;
-        let alpha = (acc[3] / steps as f32).round().clamp(0.0, 255.0);
-        out[i + 3] = alpha as u8;
-        for c in 0..3 { out[i + c] = (acc[c] / steps as f32).round().clamp(0.0, alpha) as u8; }
-    }}
-    Raster::from_premultiplied(w, h, out)
+    }
+    (cells.into_iter().map(|((oy, ox), w)| (ox, oy, w as f32)).collect(), sum)
+}
+
+/// CIMotionBlur as the Mac draws it: a Gaussian of sigma `motion_sigma(distance)` along `angle`
+/// degrees, counter-clockwise from horizontal on screen (so the direction in top-down pixels is
+/// (cos a, -sin a)): one bilinear tap per whole pixel out to ceil(3 sigma) either side, weighted
+/// exp(-t^2 / 2 sigma^2), transparent beyond the raster (`motion_stencil`). Premultiplied; colour
+/// never exceeds alpha. Run a row at a time over the stencil's cells, so the inner loop reads one
+/// source row in order. `programs.ts` FRAG_MOTION takes the same taps on the GPU.
+pub fn motion_blur(raster: &Raster, angle: f64, distance: f64) -> Raster {
+    let (stencil, sum) = motion_stencil(angle, distance);
+    if stencil.is_empty() || raster.width == 0 || raster.height == 0 { return raster.clone(); }
+    let (w, h) = (raster.width as i64, raster.height as i64);
+    let src = raster.bytes();
+    let mut out = vec![0u8; (w * h * 4) as usize];
+    let mut acc = vec![0f32; (w * 4) as usize];
+    for y in 0..h {
+        acc.fill(0.0);
+        for &(ox, oy, weight) in &stencil {
+            let sy = y + oy;
+            if sy < 0 || sy >= h { continue; }
+            // Output columns whose source column x + ox lies inside the raster.
+            let (x0, x1) = ((-ox).max(0), (w - ox).min(w));
+            if x0 >= x1 { continue; }
+            let row = &src[((sy * w + x0 + ox) * 4) as usize..((sy * w + x1 + ox) * 4) as usize];
+            for (a, s) in acc[(x0 * 4) as usize..(x1 * 4) as usize].iter_mut().zip(row) { *a += *s as f32 * weight; }
+        }
+        let line = &mut out[(y * w * 4) as usize..((y + 1) * w * 4) as usize];
+        for (p, a) in line.chunks_exact_mut(4).zip(acc.chunks_exact(4)) {
+            let alpha = (a[3] / sum).round().clamp(0.0, 255.0);
+            p[3] = alpha as u8;
+            for c in 0..3 { p[c] = (a[c] / sum).round().clamp(0.0, alpha) as u8; }
+        }
+    }
+    Raster::from_premultiplied(raster.width, raster.height, out)
 }
 
 fn noise_hash(mut x: u32) -> u32 {
```

```diff
--- a/engine/src/adjust/spatial.rs
+++ b/engine/src/adjust/spatial.rs
@@ -2,20 +2,16 @@
 //! rules here, so they blur the same image: how much of the work runs at full resolution
 //! (`spatial_blur`), and the lattice the reduced copies are cut on and how far a render pads
 //! (`spatial_grid`, `spatial_span`). The GPU asks for all three through wasm.
-use crate::{compositor::sample, gaussian_blur, gaussian_blur_in_place, motion_blur,AdjustmentKind, LayerAdjustment, LayerDraw, PlanNode, Raster, RenderPlan};
+use crate::{compositor::sample, gaussian_blur, gaussian_blur_in_place, motion_blur, motion_reach, motion_sigma, AdjustmentKind, LayerAdjustment, LayerDraw, PlanNode, Raster, RenderPlan};
 use serde::Serialize;
 
-/// Output pixels a Gaussian may reach (3 sigma) at full resolution. A longer reach runs on a copy
+/// Output pixels a blur may reach (3 sigma) at full resolution, a Gaussian or a Motion Blur (since
+/// Phase 4a, when the Motion Blur became a Gaussian along its angle run as a row-wise stencil,
+/// `motion_blur`; the even streak before it halved past 12). A longer reach runs on a copy
 /// halved until it fits, then enlarged, which keeps a blur's cost bounded at any radius and zoom.
 /// Measured against the exact kernel (Task 4's bounds test): within 1 level in the interior, and
 /// up to 4 along the canvas edge and hard alpha edges.
 pub const SPATIAL_REACH_LIMIT: f64 = 48.0;
-/// Output pixels a Motion Blur may reach (half the streak) at full resolution. Shorter than a
-/// Gaussian's: the exact streak costs one bilinear tap per pixel of its length (a 90 px streak
-/// over 3000 x 2000 took 45 s in the release wasm), and Motion Blur layers are already drawn
-/// approximately (`Document::undrawn`: the Mac's CIMotionBlur tapers, this port's does not).
-/// Halving averages detail ACROSS the streak, which the exact one keeps (the bounds test).
-pub const MOTION_REACH_LIMIT: f64 = 12.0;
 /// The most output pixels a partial render pads each side by for its blurs (`composite_plan`, and
 /// the GPU's frame). A render zoomed far into a very large blur shows its edge within this.
 pub const SPATIAL_PAD_LIMIT: f64 = 1024.0;
@@ -33,14 +29,12 @@ fn level_within(reach: f64, limit: f64) -> u32 {
     level
 }
 
-/// How many times a Gaussian reaching `reach` output pixels halves its input first.
+/// How many times a blur reaching `reach` output pixels (three sigmas, for a Gaussian and a Motion
+/// Blur alike) halves its input first.
 pub fn spatial_level(reach: f64) -> u32 { level_within(reach, SPATIAL_REACH_LIMIT) }
 
-/// How many times a Motion Blur reaching `reach` output pixels halves its input first.
-pub fn motion_level(reach: f64) -> u32 { level_within(reach, MOTION_REACH_LIMIT) }
-
 /// A blur adjustment at `out_per_doc` output pixels per document pixel: its kernel in output
-/// pixels (`sigma` for a Gaussian; `distance` and `angle` for a Motion Blur) and how many times
+/// pixels (`sigma` for both kinds, and a Motion Blur's `distance` and `angle`) and how many times
 /// its input halves first. The absent settings resolve here (settings.rs:438-440), once.
 #[derive(Clone, Copy, Debug, PartialEq, Serialize)]
 pub struct SpatialBlur { pub level: u32, pub sigma: f64, pub distance: f64, pub angle: f64 }
@@ -48,15 +42,11 @@ pub struct SpatialBlur { pub level: u32, pub sigma: f64, pub distance: f64, pub
 pub fn spatial_blur(a: &LayerAdjustment, out_per_doc: f64) -> SpatialBlur {
     let (sigma, distance) = match a.kind {
         AdjustmentKind::GaussianBlur => (a.gaussian_radius() * out_per_doc, 0.0),
-        AdjustmentKind::MotionBlur => (0.0, a.motion_distance_pixels() * out_per_doc),
+        AdjustmentKind::MotionBlur => { let d = a.motion_distance_pixels() * out_per_doc; (motion_sigma(d), d) }
         _ => (0.0, 0.0),
     };
-    // The reach: 3 sigma for a Gaussian, half the streak for a Motion Blur.
-    let level = match a.kind {
-        AdjustmentKind::MotionBlur => motion_level(distance / 2.0),
-        _ => spatial_level(sigma * 3.0),
-    };
-    SpatialBlur { level, sigma, distance, angle: a.motion_angle_degrees() }
+    // Both kernels reach three sigmas, and both halve past SPATIAL_REACH_LIMIT.
+    SpatialBlur { level: spatial_level(sigma * 3.0), sigma, distance, angle: a.motion_angle_degrees() }
 }
 
 /// Every Gaussian or Motion Blur adjustment the plan draws, bottom to top: plain nodes, and the
@@ -149,9 +139,10 @@ pub fn blur_for_layer(raster: Raster, sigma: f64) -> Raster {
     enlarged(&small, level, raster)
 }
 
-/// A streak of `distance` output pixels along `angle` degrees (the Phase 3 filter's even streak).
+/// A Motion Blur of `distance` output pixels along `angle` degrees: the destructive filter's kernel
+/// (`motion_blur`), on a copy halved `spatial_level(motion_reach(distance))` times.
 pub fn streak_for_layer(raster: Raster, angle: f64, distance: f64) -> Raster {
-    let level = motion_level(distance / 2.0);
+    let level = spatial_level(motion_reach(distance));
     if level == 0 { return motion_blur(&raster, angle, distance); }
     let small = motion_blur(&reduced(&raster, level), angle, distance / (1u32 << level) as f64);
     enlarged(&small, level, raster)
```

```diff
--- a/engine/src/adjust/settings.rs
+++ b/engine/src/adjust/settings.rs
@@ -445,11 +445,14 @@ impl LayerAdjustment {
     pub fn noise_is_monochromatic(&self) -> bool { self.noise_monochromatic.unwrap_or(false) }
     pub fn noise_seed_or_zero(&self) -> u32 { self.noise_seed.unwrap_or(0) }
     /// Document pixels a partial render must include around what it shows so this layer's blur
-    /// sees everything within its reach (`samplingMargin`, LayerAdjustment.swift:121-127).
+    /// sees everything within its reach (`samplingMargin`, LayerAdjustment.swift:121-127): three
+    /// sigmas plus 2. The Mac pads a Motion Blur by half its distance plus 2; this port's kernel
+    /// reads three sigmas (`crate::motion_reach`, 0.87 x the distance), and a partial render must
+    /// equal the whole one, so it pads that far.
     pub fn sampling_margin(&self) -> f64 {
         match self.kind {
             AdjustmentKind::GaussianBlur => self.gaussian_radius() * 3.0 + 2.0,
-            AdjustmentKind::MotionBlur => self.motion_distance_pixels() / 2.0 + 2.0,
+            AdjustmentKind::MotionBlur => crate::motion_reach(self.motion_distance_pixels()) + 2.0,
             _ => 0.0,
         }
     }
```

```diff
--- a/engine/src/document.rs
+++ b/engine/src/document.rs
@@ -1,4 +1,4 @@
-use crate::{AdjustmentKind, BlendMode, GrayRaster, LayerAdjustment, LayerEffects, LayerRecord, LayerTransform, Manifest, Point, Raster, Size, DEFAULT_RESOLUTION};
+use crate::{BlendMode, GrayRaster, LayerAdjustment, LayerEffects, LayerRecord, LayerTransform, Manifest, Point, Raster, Size, DEFAULT_RESOLUTION};
 use uuid::Uuid;
 
 #[derive(Clone, Debug, PartialEq)]
@@ -100,19 +100,14 @@ impl Layer {
             mask_revision: 1,
         }
     }
-    /// What this layer alone contains that this build does not draw as the Mac does (a Motion Blur
-    /// adjustment, drawn approximately; unknown keys, here or inside its effects), as phrases for
-    /// the notice (`Document::undrawn` collects these across every layer, plus its own
-    /// document-level check). Not sorted or de-duplicated here -- callers that need that pool the
-    /// phrases through a set, as `Document::undrawn` does and as `merge` does when refusing to
-    /// bake one (I1).
+    /// What this layer alone contains that this build does not draw as the Mac does (unknown keys,
+    /// here or inside its effects), as phrases for the notice (`Document::undrawn` collects these
+    /// across every layer, plus its own document-level check). Not sorted or de-duplicated here --
+    /// callers that need that pool the phrases through a set, as `Document::undrawn` does and as
+    /// `merge` does when refusing to bake one (I1). Motion Blur layers left this list in Phase 4a:
+    /// they are drawn as CIMotionBlur's Gaussian (`motion_blur`), measured against the Mac.
     pub fn undrawn_features(&self) -> Vec<String> {
         let mut out = Vec::new();
-        if let Some(a) = &self.extra.adjustment {
-            // Drawn as an even streak where Core Image tapers it (Filters.swift:170-208): named,
-            // and so refused by merge, until the motion probe measures the gap (ruling G-I2).
-            if a.kind == AdjustmentKind::MotionBlur { out.push("Motion Blur adjustment layers (drawn approximately)".to_string()); }
-        }
         // Layer effects are drawn (Phase 3.5c); only keys a later Mac wrote inside them are not.
         let unknown_effects = self.extra.effects.as_ref().is_some_and(LayerEffects::has_unknown);
         if !self.extra.unknown.is_empty() || unknown_effects { out.push("settings from a newer version of Compositor".to_string()); }
@@ -155,8 +150,8 @@ impl Document {
     pub fn index_of(&self, id: Uuid) -> Option<usize> { self.layers.iter().position(|l| l.id == id) }
     pub fn layer(&self, id: Uuid) -> Option<&Layer> { self.layers.iter().find(|l| l.id == id) }
     pub fn layer_mut(&mut self, id: Uuid) -> Option<&mut Layer> { self.layers.iter_mut().find(|l| l.id == id) }
-    /// What this project contains that this build does not draw yet: Motion Blur adjustment
-    /// layers (drawn approximately) and keys from a newer version. Sorted and de-duplicated so
+    /// What this project contains that this build does not draw yet: keys from a newer version.
+    /// Sorted and de-duplicated so
     /// the notice is stable. Everything listed is preserved on save.
     pub fn undrawn(&self) -> Vec<String> {
         let mut out = std::collections::BTreeSet::new();
```

```diff
--- a/app/src/canvas/gl/programs.ts
+++ b/app/src/canvas/gl/programs.ts
@@ -390,13 +390,16 @@ void main() {
   vec4 c = acc / sum;
   color = last ? vec4(min(c.rgb, vec3(c.a)), c.a) : c;
 }`;
-// motion_blur in engine/src/adjust/filters.rs: an even streak of bilinear samples, zero beyond the
-// raster (sample_zero). dir is (cos a, sin a): these rows run bottom-up, the CPU's top-down.
+// motion_blur in engine/src/adjust/filters.rs: CIMotionBlur's Gaussian along the angle, one bilinear
+// tap per whole pixel out to `radius` (ceil(3 sigma)) either side, weighted exp(-t^2 / 2 sigma^2),
+// zero beyond the raster (sample_zero). dir is (cos a, sin a): these rows run bottom-up, the CPU's
+// top-down.
 const FRAG_MOTION = `#version 300 es
 precision highp float;
 uniform sampler2D src;
 uniform vec2 dir;
-uniform int steps;
+uniform float sigma;
+uniform int radius;
 uniform ivec2 size;
 out vec4 color;
 vec4 fetchZero(ivec2 p) { return (p.x < 0 || p.y < 0 || p.x >= size.x || p.y >= size.y) ? vec4(0.0) : texelFetch(src, p, 0); }
@@ -405,10 +408,13 @@ vec4 bilinearZero(vec2 q) {
   return mix(mix(fetchZero(i), fetchZero(i + ivec2(1, 0)), t.x), mix(fetchZero(i + ivec2(0, 1)), fetchZero(i + ivec2(1, 1)), t.x), t.y);
 }
 void main() {
-  float mid = float(steps - 1) / 2.0;
-  vec4 acc = vec4(0.0);
-  for (int i = 0; i < steps; i++) acc += bilinearZero(gl_FragCoord.xy + dir * (float(i) - mid));
-  vec4 c = acc / float(steps);
+  vec4 acc = vec4(0.0); float sum = 0.0;
+  for (int k = -radius; k <= radius; k++) {
+    float w = exp(-float(k * k) / (2.0 * sigma * sigma));
+    sum += w;
+    acc += bilinearZero(gl_FragCoord.xy + dir * float(k)) * w;
+  }
+  vec4 c = acc / sum;
   color = vec4(min(c.rgb, vec3(c.a)), c.a);
 }`;
 // spatial_target in engine/src/compositor.rs: the blurred copy enlarged (spatial.rs `enlarged`),
@@ -489,7 +495,7 @@ export function createPrograms(gl: WebGL2RenderingContext): Programs {
       "bwLow", "bwHigh", "bwTint", "cbShadows", "cbMidtones", "cbHighlights", "cbPreserve", "noiseParams", "noiseSeed"]),
     halve: compile(gl, VERT_SCREEN, FRAG_HALVE, ["src", "size"]),
     gaussian: compile(gl, VERT_SCREEN, FRAG_GAUSSIAN, ["src", "sigma", "radius", "horizontal", "last", "size"]),
-    motion: compile(gl, VERT_SCREEN, FRAG_MOTION, ["src", "dir", "steps", "size"]),
+    motion: compile(gl, VERT_SCREEN, FRAG_MOTION, ["src", "dir", "sigma", "radius", "size"]),
     spatialMix: compile(gl, VERT_SCREEN, FRAG_SPATIAL_MIX, ["original", "adjusted", "coverage", "useCoverage", "opacity", "mode", "keepsAlpha", "level", "adjustedSize", "beyond"]),
     vao, buffer,
   };
```

```diff
--- a/app/src/canvas/gl-renderer.ts
+++ b/app/src/canvas/gl-renderer.ts
@@ -388,9 +388,10 @@ export class GlRenderer implements Renderer {
       this.sizedPass(this.programs.gaussian, out, w, h, { src: tmp.tex }, set(false));
     } else {
       const radians = b.angle * Math.PI / 180;
-      const steps = Math.max(1, Math.round(b.distance / factor));
+      // motion_blur at the reduced sigma: taps out to ceil(3 sigma); none past one (a copy) below that.
+      const s = b.sigma / factor, radius = s > 0 ? Math.ceil(s * 3) : 0, sigma = s > 0 ? s : 1;
       this.sizedPass(this.programs.motion, out, w, h, { src: source }, (u) => {
-        gl.uniform2f(u.dir, Math.cos(radians), Math.sin(radians)); gl.uniform1i(u.steps, steps); gl.uniform2i(u.size, w, h);
+        gl.uniform2f(u.dir, Math.cos(radians), Math.sin(radians)); gl.uniform1f(u.sigma, sigma); gl.uniform1i(u.radius, radius); gl.uniform2i(u.size, w, h);
       });
     }
     const p = this.programs.spatialMix;
```

- [ ] **Step 4: Run the tests and watch them pass**

The six engine test files above, then the whole engine suite: 392 passed, 1 ignored (+2). `pnpm wasm:dev`, then `npx playwright test app/tests/e2e/mac-1.2.10.spec.ts app/tests/e2e/mac-1.2.6.spec.ts`.

Then the release timings: `pnpm wasm`, then `$env:PERF = "1"; pnpm e2e -- perf-spatial; Remove-Item Env:PERF`. Its "motion 90" and "motion 2000" (a Motion Blur layer over an opaque 3000 x 2000 canvas at 30 degrees, exported) measured 3,345 and 2,089 ms on the scratch copy after this task (2,676 and 2,128 with the streak). The scratch copy also timed, with the same page code and `ApplyFilter` in place of the layer: a 24 px layer 9,491 ms (12,143 before); the destructive filter at 24 px 3,200 ms (10,784) and at 90 px 10,910 ms (40,643). Record yours in the task report; a result more than twice these is a finding to report, not a failure.

- [ ] **Step 5: Prove it bites**

(1) In `motion_sigma`, return `distance / 2.0`: with `--no-fail-fast`, the probe test, the dot test (`motion_blur_streaks_along_its_angle_counterclockwise_from_horizontal`), the stencil test and `a_blur_layer_written_over_its_input_equals_the_full_frame_path_to_the_bit` fail. Restore. (2) In `motion_blur`, divide the accumulated row by `1.0` instead of `sum` (the weights' total): the dot test and the stencil test fail. Restore. (3) Give `FRAG_MOTION` (not `FRAG_GAUSSIAN`, which has the same line) taps of equal weight (`float w = 1.0;`): the e2e step-edge test fails on its 150 px Motion Blur (alpha 122 at x 120, the formula 144.61) and the motion probe on the GPU fails too (measured on the scratch copy). Restore.

- [ ] **Step 6: Commit**

```
git commit -- engine/src/adjust/filters.rs engine/src/adjust/spatial.rs engine/src/adjust/settings.rs engine/src/document.rs app/src/canvas/gl/programs.ts app/src/canvas/gl-renderer.ts engine/tests/filters.rs engine/tests/spatial_adjustments.rs engine/tests/undrawn.rs engine/tests/merge_undrawn.rs engine/tests/peak_heap.rs engine/tests/mac_1_2_10.rs app/tests/e2e/mac-1.2.10.spec.ts app/tests/e2e/mac-1.2.6.spec.ts -m "fix: Motion Blur is CIMotionBlur's Gaussian along the angle, as the Mac 1.2.10 probe shows" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: The selection: an outline, its geometry and its coverage

The Mac's `DocumentSelection` is a `CGPath`, an anti-alias flag and a feather, filled by the winding rule; its coverage is Core Graphics' fill of the path blurred by sigma feather / 2 over the region the selection can reach, cut to the canvas (`clip(canvas:)`), and every edit reads that coverage on its own pixel grid (Selection.swift:7-66; PixelAdjust.swift:23-34). This task builds that model in `engine/src/selection/` and puts it in the `Document`. Nothing uses it yet: the commands come in Task 4.

**Why fixed point (rulings OQ1-OQ5, OQ9).** Contours are i32 in 256ths of a pixel, so a whole-pixel rectangle, a union of two and a move by whole pixels are exact, two equal selections compare equal (so `same_content` records no undo step for a no-op), and i_overlay's integer booleans apply directly. The ellipse is Core Graphics' four Beziers, flattened within 0.01 px. The rasteriser is an exact-area accumulation rasteriser a row at a time (one row of f32), so an antialiased whole-pixel edge is exactly 0 or 255 and a diagonal through a pixel's corners is 128; without anti-aliasing a pixel is in when its centre is, half-open. The coverage region, blur and on-grid sampling follow the Mac's numbers (Global Constraints).

**Files:**
- Modify: `engine/Cargo.toml`, `Cargo.lock` (i_overlay 9)
- Create: `engine/src/selection/mod.rs`, `engine/src/selection/geometry.rs`, `engine/src/selection/coverage.rs`
- Modify: `engine/src/lib.rs`, `engine/src/document.rs`, `engine/src/package.rs`, `engine/src/compositor.rs`
- Create test: `engine/tests/selection_model.rs`

**Interfaces:**
- Produces (re-exported from the crate root): `Selection { contours: Vec<Contour>, antialiased: bool, feather: f64 }` with `new`, `bounds() -> Option<Rect>`, `is_empty()`, `coverage_bounds()`, `points() -> Vec<Vec<Point>>`, `point_count()`, `contains(Point)` (nonzero winding), `translated(dx, dy)` (in `SUBPIXEL` units); `Contour = Vec<[i32; 2]>`; `SelectionMode { Replace, Add, Subtract }`; `SelectionShape { Freehand, Polygonal, Rectangle, Ellipse }` with `action_name()`; `SelectionState { revision, empty, bounds, antialiased, feather, points }` (camelCase JSON); `SUBPIXEL = 256.0`, `SELECTION_COORDINATE_LIMIT = 1_000_000.0`, `MAX_FEATHER = 250.0`, `MAX_RESIZE = 500`; `selection::geometry::{quantize, polygon, rectangle, ellipse, transformed, combine, Boolean, band, CURVE_TOLERANCE}`; `rasterize(contours, left, top, width, height, antialiased) -> GrayRaster`; `SelectionClip { origin: (i64, i64), coverage: Option<GrayRaster> }` with `new(&Selection, canvas_width, canvas_height)`, `at(Point) -> f32`, `on_grid(&Affine, width, height) -> GrayRaster`; `selection_coverage(&Document, &Affine, width, height) -> Option<GrayRaster>` (None with no selection). `Document.selection: Option<Selection>`, `Document.selection_revision: u64`.
- Task 4 adds `pub mod outline; pub mod trace;` to `selection/mod.rs` and Task 5 `pub mod wand;`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/selection_model.rs` ports the coverage cases of SelectionTests, SelectionFeatherTests and SelectionEditTests with their shapes and numbers; the ellipse is compared with the true ellipse's area coverage computed in the test:

```rust
//! The selection's outline and its coverage (engine/src/selection): Core Graphics' winding fill of
//! the Mac's `DocumentSelection` (Selection.swift:7-49), its booleans and its Expand band. Expected
//! values come from the geometry (areas, Gaussian weights) or from Compositor for Mac's own tests
//! (CompositorTests/SelectionTests.swift, SelectionFeatherTests.swift).
use compositor_engine::selection::geometry::{band, combine, ellipse, polygon, rectangle, Boolean};
use compositor_engine::*;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }

/// The selection's coverage over a `width` x `height` canvas, 0-255, as the Mac's tests read it
/// (`coverage(width:height:)`): the clip placed on the canvas's own pixels.
fn coverage(selection: &Selection, width: u32, height: u32) -> GrayRaster {
    SelectionClip::new(selection, width, height).on_grid(&Affine::IDENTITY, width, height)
}
fn at(c: &GrayRaster, x: u32, y: u32) -> u8 { c.bytes()[(y * c.width + x) as usize] }

#[test]
fn a_whole_pixel_rectangle_covers_exactly_its_pixels_with_or_without_antialiasing() {
    // An oblong box off every axis of symmetry: 40 x 41 at (20, 30), on 100 x 100.
    for antialiased in [true, false] {
        let s = Selection::new(vec![rectangle(rect(20.0, 30.0, 40.0, 41.0))], antialiased, 0.0);
        let c = coverage(&s, 100, 100);
        for (x, y, want) in [(20, 30, 255), (19, 30, 0), (20, 29, 0), (59, 70, 255), (60, 70, 0), (59, 71, 0), (40, 50, 255)] {
            assert_eq!(at(&c, x, y), want, "({x}, {y}) antialiased {antialiased}");
        }
        assert_eq!(c.bytes().iter().filter(|&&v| v == 255).count(), 40 * 41);
        assert!(c.bytes().iter().all(|&v| v == 0 || v == 255));
    }
}

#[test]
fn antialiasing_controls_edge_coverage() {
    // SelectionTests.antialiasingControlsEdgeCoverage: the triangle (0,0), (100,0), (0,100). Its
    // edge x + y = 100 cuts each pixel (x, 99 - x) along its diagonal, so antialiased that pixel is
    // half covered: 255 x 0.5, rounded, 128. Aliased every pixel is 0 or 255.
    let triangle = vec![polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(0.0, 100.0)])];
    let soft = coverage(&Selection::new(triangle.clone(), true, 0.0), 100, 100);
    for x in 0..100 { assert_eq!(at(&soft, x, 99 - x), 128, "antialiased at ({x}, {})", 99 - x); }
    let hard = coverage(&Selection::new(triangle, false, 0.0), 100, 100);
    assert!(hard.bytes().iter().all(|&v| v == 0 || v == 255));
    for x in 0..99 { assert_eq!(at(&hard, x, 98 - x), 255, "inside the edge at ({x}, {})", 98 - x); }
    // A pixel whose centre lies exactly on the edge is left out (half-open, as the fill's own rule).
    for x in 0..100 { assert_eq!(at(&hard, x, 99 - x), 0); }
}

#[test]
fn a_thin_sliver_and_a_contour_off_the_canvas_cover_by_area() {
    // A 0.25-px-wide sliver down column 10 covers a quarter of each pixel: 64. A square from -30 to
    // 5 covers columns 0..5 fully: the part off the canvas folds in as nothing.
    let s = Selection::new(vec![rectangle(rect(10.5, 2.0, 0.25, 6.0)), rectangle(rect(-30.0, 3.0, 35.0, 2.0))], true, 0.0);
    let c = coverage(&s, 20, 10);
    assert_eq!(at(&c, 10, 4), 64);
    assert_eq!((at(&c, 0, 3), at(&c, 4, 4), at(&c, 5, 4)), (255, 255, 0));
}

/// The fraction of pixel (x, y) inside the ellipse `CGPath.addEllipse` builds in `box`, from a
/// 16 x 16 grid of samples against the four Beziers' implicit ellipse: the true ellipse, whose
/// distance from the Beziers is 0.027% of a half-axis.
fn true_ellipse_coverage(b: Rect, x: u32, y: u32) -> f64 {
    let (cx, cy, rx, ry) = (b.x + b.width / 2.0, b.y + b.height / 2.0, b.width / 2.0, b.height / 2.0);
    let mut inside = 0;
    for j in 0..16 { for i in 0..16 {
        let (sx, sy) = (x as f64 + (i as f64 + 0.5) / 16.0, y as f64 + (j as f64 + 0.5) / 16.0);
        if ((sx - cx) / rx).powi(2) + ((sy - cy) / ry).powi(2) <= 1.0 { inside += 1; }
    }}
    inside as f64 / 256.0
}

#[test]
fn an_ellipse_fills_its_box_as_an_oval_with_soft_edges() {
    // SelectionTests.marqueeEllipseSelectsAnOvalInItsBox: the box (10, 20)-(70, 60).
    let b = rect(10.0, 20.0, 60.0, 40.0);
    let s = Selection::new(vec![ellipse(b)], true, 0.0);
    let bounds = s.bounds().unwrap();
    assert!((bounds.x - 10.0).abs() < 0.5 && (bounds.max_x() - 70.0).abs() < 0.5 && (bounds.y - 20.0).abs() < 0.5 && (bounds.max_y() - 60.0).abs() < 0.5);
    let c = coverage(&s, 80, 80);
    assert_eq!(at(&c, 40, 40), 255, "the middle");
    assert_eq!(at(&c, 11, 21), 0, "the box's corner lies outside the oval");
    // Every pixel against the true ellipse's area coverage. Measured on p4a-scratch (2026-09-27):
    // worst 4 levels: the Beziers' own distance from the ellipse, the flattening (CURVE_TOLERANCE)
    // and the 1/256 sampling grid together. Unflattened chords of 45 degrees measure far more.
    let mut worst = 0u8;
    for y in 0..80 { for x in 0..80 {
        let want = (true_ellipse_coverage(b, x, y) * 255.0).round() as i64;
        worst = worst.max((at(&c, x, y) as i64 - want).unsigned_abs() as u8);
    }}
    assert!(worst <= 4, "worst {worst}");
}

#[test]
fn union_intersection_and_difference_keep_whole_pixel_edges_exact() {
    let a = vec![rectangle(rect(10.0, 10.0, 40.0, 40.0))];
    let b = vec![rectangle(rect(30.0, 20.0, 40.0, 50.0))];
    let union = Selection::new(combine(&a, &b, Boolean::Union), true, 0.0);
    assert_eq!(union.bounds(), Some(rect(10.0, 10.0, 60.0, 60.0)));
    let meet = Selection::new(combine(&a, &b, Boolean::Intersection), true, 0.0);
    assert_eq!(meet.bounds(), Some(rect(30.0, 20.0, 20.0, 30.0)));
    let cut = Selection::new(combine(&a, &b, Boolean::Difference), true, 0.0);
    let c = coverage(&cut, 80, 80);
    assert_eq!((at(&c, 15, 15), at(&c, 35, 35), at(&c, 35, 15), at(&c, 60, 60)), (255, 0, 255, 0));
    assert_eq!(c.bytes().iter().filter(|&&v| v == 255).count(), 40 * 40 - 20 * 30);
    // Subtracting everything leaves an outline with no area: an explicit empty selection.
    let gone = Selection::new(combine(&a, &[rectangle(rect(0.0, 0.0, 80.0, 80.0))], Boolean::Difference), true, 0.0);
    assert!(gone.is_empty() && gone.bounds().is_none());
}

#[test]
fn a_band_round_joined_about_the_outline_grows_and_shrinks_it() {
    // SelectionTests.expandAndContractGrowAndShrinkTheOutline, on the outline: the square
    // (40,40)-(60,60), grown by 5, then that shrunk by 8.
    let square = vec![rectangle(rect(40.0, 40.0, 20.0, 20.0))];
    let grown = combine(&square, &band(&square, 5.0), Boolean::Union);
    let g = Selection::new(grown.clone(), true, 0.0).bounds().unwrap();
    assert!((g.x - 35.0).abs() < 0.01 && (g.width - 30.0).abs() < 0.01, "{g:?}");
    let c = coverage(&Selection::new(grown.clone(), true, 0.0), 100, 100);
    assert_eq!((at(&c, 37, 50), at(&c, 33, 50)), (255, 0));
    // The joins are round: pixel (35, 35) is the square's grown corner, and its nearest point,
    // (36, 36), lies 5.66 px from the corner (40, 40), outside the 5-px arc. A mitred join fills it.
    assert_eq!(at(&c, 35, 35), 0);
    let shrunk = combine(&grown, &band(&grown, 8.0), Boolean::Difference);
    let s = Selection::new(shrunk, true, 0.0).bounds().unwrap();
    assert!((s.x - 43.0).abs() < 0.01 && (s.width - 14.0).abs() < 0.01, "{s:?}");
}

#[test]
fn a_feather_blurs_the_coverage_by_half_its_amount() {
    // SelectionFeatherTests.featherSoftensTheSelectionAndWhatItClips: the rectangle (20, 0)-(40, 20)
    // on 60 x 20, feather 6. Every row is the same, so the column profile is the fill's row
    // blurred by a Gaussian of sigma 3 (radius 9), the region's edge repeated: at column x,
    // 255 x (the kernel's weight on columns 20..40) / (all of it).
    let s = Selection::new(vec![rectangle(rect(20.0, 0.0, 20.0, 20.0))], true, 6.0);
    let c = coverage(&s, 60, 20);
    let w = |j: i64| (-((j * j) as f64) / 18.0).exp();
    let total: f64 = (-9..=9).map(w).sum();
    for x in 0..60i64 {
        let want = 255.0 * (-9..=9).filter(|j| (20..40).contains(&(x + j))).map(w).sum::<f64>() / total;
        assert!((at(&c, x as u32, 10) as f64 - want).abs() <= 1.0, "column {x}: {} vs {want:.2}", at(&c, x as u32, 10));
    }
    let fading = (0..60).filter(|x| { let v = at(&c, *x, 10); v > 8 && v < 247 }).count();
    assert!(fading >= 4, "the Mac's own check: {fading} fading columns");
}

#[test]
fn the_clip_covers_the_coverage_bounds_and_a_pixel_cut_to_the_canvas() {
    // `clip(canvas:)`: a 10 x 6 box at (-3, 20) with feather 2 reaches ceil(4) = 4 px further, and a
    // pixel more; the canvas cuts it at x = 0.
    let s = Selection::new(vec![rectangle(rect(-3.0, 20.0, 10.0, 6.0))], true, 2.0);
    let clip = SelectionClip::new(&s, 100, 50);
    let c = clip.coverage.as_ref().unwrap();
    assert_eq!(clip.origin, (0, 15));
    assert_eq!((c.width, c.height), (7 + 4 + 1, 6 + 2 * (4 + 1)));
    // An explicit empty selection clips everything away.
    let empty = SelectionClip::new(&Selection::new(vec![], true, 0.0), 100, 50);
    assert!(empty.coverage.is_none());
    assert!(empty.on_grid(&Affine::IDENTITY, 100, 50).bytes().iter().all(|&v| v == 0));
}

#[test]
fn coverage_on_a_scaled_layer_samples_the_canvas_coverage_at_each_pixel_centre() {
    // SelectionEditTests.clipFollowsScaledLayersAndSoftensEdges: a 50 x 20 layer stretched to
    // 100 x 40 at the origin, under the triangle (0,0), (100,0), (0,40). Layer pixel (i, j)
    // centres on document (2i + 1, 2j + 1).
    let s = Selection::new(vec![polygon(&[p(0.0, 0.0), p(100.0, 0.0), p(0.0, 40.0)])], true, 0.0);
    let layer = LayerTransform::axis_aligned(p(0.0, 0.0), Size { width: 100.0, height: 40.0 });
    let grid = SelectionClip::new(&s, 100, 40).on_grid(&layer.pixel_to_document(50, 20), 50, 20);
    let canvas = coverage(&s, 100, 40);
    assert_eq!(at(&grid, 5, 5), 255, "inside");
    assert_eq!(at(&grid, 45, 17), 0, "outside");
    // Bilinear between the canvas pixels around each centre: centre (2i + 1, 2j + 1) is the shared
    // corner of canvas pixels 2i..2i+1 and 2j..2j+1, so it is their mean.
    for j in 0..20u32 { for i in 0..50u32 {
        let mean = (at(&canvas, 2 * i, 2 * j) as f64 + at(&canvas, 2 * i + 1, 2 * j) as f64 + at(&canvas, 2 * i, 2 * j + 1) as f64 + at(&canvas, 2 * i + 1, 2 * j + 1) as f64) / 4.0;
        assert!((at(&grid, i, j) as f64 - mean).abs() <= 1.0, "({i}, {j}): {} vs {mean}", at(&grid, i, j));
    }}
    assert!(grid.bytes().iter().any(|&v| v > 0 && v < 255), "the diagonal is soft");
}

#[test]
fn coverage_on_a_layer_moved_by_whole_pixels_is_the_canvas_coverage_cropped() {
    let s = Selection::new(vec![ellipse(rect(12.0, 7.0, 31.0, 18.0))], true, 1.5);
    let canvas = coverage(&s, 60, 40);
    let layer = LayerTransform::axis_aligned(p(9.0, -4.0), Size { width: 30.0, height: 25.0 });
    let grid = SelectionClip::new(&s, 60, 40).on_grid(&layer.pixel_to_document(30, 25), 30, 25);
    for j in 0..25u32 { for i in 0..30u32 {
        let want = if j >= 4 { at(&canvas, 9 + i, j - 4) } else { 0 };
        assert_eq!(at(&grid, i, j), want, "({i}, {j})");
    }}
}

#[test]
fn an_outline_knows_what_it_contains_and_moves_by_whole_units() {
    let s = Selection::new(vec![rectangle(rect(10.0, 10.0, 20.0, 20.0))], true, 0.0);
    assert!(s.contains(p(20.0, 20.0)) && !s.contains(p(60.0, 60.0)) && !s.contains(p(9.5, 20.0)));
    let moved = s.translated(40 * SUBPIXEL as i32, 40 * SUBPIXEL as i32);
    assert_eq!(moved.bounds(), Some(rect(50.0, 50.0, 20.0, 20.0)));
    assert!(!s.is_empty());
    assert!(Selection::new(vec![polygon(&[p(3.0, 3.0), p(9.0, 3.0), p(12.0, 3.0)])], true, 0.0).is_empty(), "no area");
}
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test selection_model`: it does not compile (`Selection`, `SelectionClip`, `rasterize`, `selection::geometry` do not exist).

- [ ] **Step 3: The model**

Add i_overlay (let cargo write `Cargo.lock`; it adds i_overlay 9.0.0, i_float 5.0.0, i_shape 5.0.0, i_key_sort 0.11.0, i_tree 0.19.0 and libm 0.2.16):

```diff
--- a/engine/Cargo.toml
+++ b/engine/Cargo.toml
@@ -14,5 +14,6 @@ thiserror = "2"
 image = { version = "0.25", default-features = false, features = ["png", "jpeg", "tiff", "webp", "bmp"] }
 png = "0.17"
 jpeg-encoder = "0.6"
+i_overlay = "9"
 
 [dev-dependencies]
```

```rust
//! The selection (Phase 4a): Compositor for Mac's `DocumentSelection` (Document/Selection.swift:7-49).
//! An outline in document pixels, filled by the winding rule, with an anti-alias flag and a feather.
//! It is part of the `Document`, so undo covers it, and it is never saved (EditorSession.swift:70-71).
//!
//! The outline is a set of closed contours in fixed point, `SUBPIXEL` units per document pixel, so
//! that whole-pixel shapes stay exact through every boolean (`geometry`), a move is exact, and two
//! equal selections compare equal. `coverage` rasterises it as Core Graphics fills it; `trace` turns
//! pixels back into outlines (WandPixels.c `wand_trace`); `wand` is the Magic Wand's matcher.
pub mod coverage;
pub mod geometry;

use crate::{Point, Rect};
use serde::{Deserialize, Serialize};

/// Fixed-point units per document pixel in which an outline is stored and combined.
pub const SUBPIXEL: f64 = 256.0;
/// No outline point lies further than this from the canvas origin, in document pixels (the i32
/// fixed-point range, with room for a 500 px Expand). A move that would pass it is refused.
pub const SELECTION_COORDINATE_LIMIT: f64 = 1_000_000.0;
/// Feather amounts reach 250 px (Selection.swift:307, :330).
pub const MAX_FEATHER: f64 = 250.0;
/// Expand and Contract take 1 to 500 px (Selection.swift:307, :334).
pub const MAX_RESIZE: u32 = 500;

/// One closed contour, in `SUBPIXEL` units.
pub type Contour = Vec<[i32; 2]>;

#[derive(Clone, Debug, PartialEq)]
pub struct Selection {
    /// Closed contours in `SUBPIXEL` units of document space, top-left origin, filled by the nonzero
    /// winding rule. Empty contours or a zero-area outline make an explicit empty selection.
    pub contours: Vec<Contour>,
    /// Hard pixel edges when false (and no feather), as the Mac's Anti-alias toggle.
    pub antialiased: bool,
    /// How far the edge fades, in document pixels, 0 to `MAX_FEATHER`; sigma is half of it.
    pub feather: f64,
}

impl Selection {
    pub fn new(contours: Vec<Contour>, antialiased: bool, feather: f64) -> Selection { Selection { contours, antialiased, feather } }
    /// The outline's bounding box in document pixels; None when it has no points.
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.contours.iter().flatten();
        let first = points.next()?;
        let (mut x0, mut y0, mut x1, mut y1) = (first[0], first[1], first[0], first[1]);
        for p in points { x0 = x0.min(p[0]); y0 = y0.min(p[1]); x1 = x1.max(p[0]); y1 = y1.max(p[1]); }
        Some(Rect { x: x0 as f64 / SUBPIXEL, y: y0 as f64 / SUBPIXEL, width: (x1 - x0) as f64 / SUBPIXEL, height: (y1 - y0) as f64 / SUBPIXEL })
    }
    /// An explicit empty selection: no outline, or one whose bounds have no area (`isEmpty`).
    /// Every edit refuses it; it is not the same as no selection, which edits the whole layer.
    pub fn is_empty(&self) -> bool { self.bounds().map_or(true, |b| !(b.width > 0.0) || !(b.height > 0.0)) }
    /// The bounds grown by ceil(2 x feather): four sigmas of the fade (`coverageBounds`).
    pub fn coverage_bounds(&self) -> Option<Rect> {
        let b = self.bounds()?;
        let g = (self.feather * 2.0).ceil();
        Some(Rect { x: b.x - g, y: b.y - g, width: b.width + 2.0 * g, height: b.height + 2.0 * g })
    }
    /// The outline in document pixels.
    pub fn points(&self) -> Vec<Vec<Point>> {
        self.contours.iter().map(|c| c.iter().map(|p| Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL }).collect()).collect()
    }
    /// How many points the outline has, all contours together.
    pub fn point_count(&self) -> usize { self.contours.iter().map(Vec::len).sum() }
    /// Whether `p` lies inside, by the nonzero winding rule (`path.contains(_, using: .winding)`).
    pub fn contains(&self, p: Point) -> bool {
        let (px, py) = (p.x * SUBPIXEL, p.y * SUBPIXEL);
        let mut winding = 0i32;
        for c in &self.contours {
            for i in 0..c.len() {
                let (a, b) = (c[i], c[(i + 1) % c.len()]);
                let (ay, by) = (a[1] as f64, b[1] as f64);
                if (ay <= py) != (by <= py) {
                    let x = a[0] as f64 + (py - ay) * (b[0] - a[0]) as f64 / (by - ay);
                    if x > px { winding += if by > ay { 1 } else { -1 }; }
                }
            }
        }
        winding != 0
    }
    /// The same outline moved by whole `SUBPIXEL` units, flags kept.
    pub fn translated(&self, dx: i32, dy: i32) -> Selection {
        Selection { contours: self.contours.iter().map(|c| c.iter().map(|p| [p[0] + dx, p[1] + dy]).collect()).collect(), ..self.clone() }
    }
}

/// How a new outline meets the current selection (`SelectionMode`, Selection.swift:85-89).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionMode { Replace, Add, Subtract }

/// The drawn outline's kind (`LassoKind`, Selection.swift:75-83): the Lasso's two and the Marquee's two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionShape { Freehand, Polygonal, Rectangle, Ellipse }

impl SelectionShape {
    /// The undo name each makes (Selection.swift:228-230).
    pub fn action_name(self) -> &'static str {
        match self {
            SelectionShape::Freehand => "Lasso", SelectionShape::Polygonal => "Polygonal Lasso",
            SelectionShape::Rectangle => "Rectangular Marquee", SelectionShape::Ellipse => "Elliptical Marquee",
        }
    }
}

/// What the app needs to know about the selection without its outline (`DocumentState`): the
/// outline itself travels only when `revision` changes (`Engine::selection_outline`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionState {
    /// Issued by the engine's revision counter whenever the outline or its flags change; never reused.
    pub revision: u64,
    pub empty: bool,
    /// The outline's bounds in document pixels; None for an outline with no points.
    pub bounds: Option<Rect>,
    pub antialiased: bool,
    pub feather: f64,
    pub points: usize,
}
```

```rust
//! Outlines as Core Graphics builds and combines them for the Mac's selection (Selection.swift):
//! polygons, the Marquee's rectangle and ellipse, the canvas clip, union / intersection /
//! difference with the winding rule, and the round-joined band Expand and Contract stroke. The
//! booleans and the stroke are i_overlay's, on its i32 integer engine, in `SUBPIXEL` units, so a
//! whole-pixel shape comes back exactly.
use super::{Contour, SUBPIXEL};
use crate::{Affine, Point, Rect};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay::Overlay;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::i_float::int::angle::Angle;
use i_overlay::i_float::int::point::IntPoint;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::stroke::offset::IntStrokeOffset;
use i_overlay::mesh::int::style::{IntLineCap, IntLineJoin, IntStrokeStyle};

/// How far a flattened curve may stray from the curve, in document pixels: an ellipse's edge
/// coverage moves by at most about 1/255 per 0.004 px, so this keeps a flattened ellipse within
/// about 3 levels of the curve on any pixel. Also the chord error of an Expand's round joins.
pub const CURVE_TOLERANCE: f64 = 0.01;

/// A document point in `SUBPIXEL` units, rounded.
pub fn quantize(p: Point) -> [i32; 2] { [(p.x * SUBPIXEL).round() as i32, (p.y * SUBPIXEL).round() as i32] }

/// One closed contour through `points` (`addLines(between:)` then `closeSubpath`).
pub fn polygon(points: &[Point]) -> Contour { points.iter().map(|p| quantize(*p)).collect() }

/// The rectangle `r` as one contour.
pub fn rectangle(r: Rect) -> Contour {
    polygon(&[Point { x: r.x, y: r.y }, Point { x: r.max_x(), y: r.y }, Point { x: r.max_x(), y: r.max_y() }, Point { x: r.x, y: r.max_y() }])
}

/// `CGPath.addEllipse(in: r)`: four cubic Beziers from the right-hand point, with control points
/// at 0.5523 of each half-axis (Core Graphics' circle constant, kappa = 4 (sqrt 2 - 1) / 3), each
/// flattened into chords within `CURVE_TOLERANCE` of the curve (Wang's bound, n = ceil(sqrt(3 L /
/// 4 tol)) for the largest second difference L of its control points).
pub fn ellipse(r: Rect) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let (cx, cy, rx, ry) = (r.x + r.width / 2.0, r.y + r.height / 2.0, r.width / 2.0, r.height / 2.0);
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = |x: f64, y: f64| Point { x, y };
    // Right, bottom, left, top, and back to the right: the quadrants in y-down order.
    let arcs = [
        [p(cx + rx, cy), p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry)],
        [p(cx, cy + ry), p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy)],
        [p(cx - rx, cy), p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry)],
        [p(cx, cy - ry), p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy)],
    ];
    let mut points = Vec::new();
    for [p0, p1, p2, p3] in arcs {
        let second = |a: Point, b: Point, c: Point| ((a.x - 2.0 * b.x + c.x).powi(2) + (a.y - 2.0 * b.y + c.y).powi(2)).sqrt();
        let l = second(p0, p1, p2).max(second(p1, p2, p3));
        let n = ((0.75 * l / CURVE_TOLERANCE).sqrt().ceil() as usize).max(1);
        for i in 0..n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            points.push(Point { x: a * p0.x + b * p1.x + c * p2.x + d * p3.x, y: a * p0.y + b * p1.y + c * p2.y + d * p3.y });
        }
    }
    polygon(&points)
}

/// The contours mapped through `map` (document points in, document points out), then quantized.
pub fn transformed(contours: &[Contour], map: &Affine) -> Vec<Contour> {
    contours.iter().map(|c| c.iter().map(|p| quantize(map.apply(Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL }))).collect()).collect()
}

fn to_int(contours: &[Contour]) -> Vec<Vec<IntPoint<i32>>> {
    contours.iter().filter(|c| c.len() >= 3).map(|c| c.iter().map(|p| IntPoint::new(p[0], p[1])).collect()).collect()
}
fn from_shapes(shapes: Vec<Vec<Vec<IntPoint<i32>>>>) -> Vec<Contour> {
    shapes.into_iter().flatten().map(|c| c.into_iter().map(|p| [p.x, p.y]).collect()).collect()
}

/// A boolean of two outlines, each filled by the nonzero winding rule (`union`, `intersection`,
/// `subtracting` with `.winding`, macOS 14). The result has no overlapping contours: outer
/// boundaries and holes wound in opposite directions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boolean { Union, Intersection, Difference }

pub fn combine(a: &[Contour], b: &[Contour], op: Boolean) -> Vec<Contour> {
    let rule = match op { Boolean::Union => OverlayRule::Union, Boolean::Intersection => OverlayRule::Intersect, Boolean::Difference => OverlayRule::Difference };
    let (subject, clip) = (to_int(a), to_int(b));
    from_shapes(Overlay::from_subj_and_clip(subject.as_slice(), clip.as_slice()).overlay(rule, FillRule::NonZero))
}

/// The band `half_width` pixels either side of every contour: `copy(strokingWithWidth: 2 x
/// half_width, lineCap: .round, lineJoin: .round)` (Selection.swift:336), as a filled outline.
/// Round joins turn in steps whose chords stray at most `CURVE_TOLERANCE` from the arc.
pub fn band(contours: &[Contour], half_width: f64) -> Vec<Contour> {
    let radius = (half_width * SUBPIXEL).round();
    // The chord of an arc of radius r over angle a strays r (1 - cos(a / 2)) from it.
    let step = 2.0 * (1.0 - CURVE_TOLERANCE * SUBPIXEL / radius).clamp(-1.0, 1.0).acos();
    let arc = ArcOptions { max_step: Angle::from_radians(step).unwrap_or(ArcOptions::MIN_STEP), rotation_precision: 32 };
    let style = IntStrokeStyle::new(2 * radius as i32).line_join(IntLineJoin::Round(arc)).start_cap(IntLineCap::Round(arc)).end_cap(IntLineCap::Round(arc));
    let paths = to_int(contours);
    match paths.as_slice().stroke(&style, true) { Ok(shapes) => from_shapes(shapes), Err(_) => Vec::new() }
}
```

```rust
//! A selection as the 8-bit coverage every selection-limited edit blends through: Core Graphics'
//! winding fill of the outline, antialiased when the selection is or has a feather
//! (`DocumentSelection.coverage`, Selection.swift:15-29), then a Gaussian of sigma feather / 2
//! clamped to the region's edge; over the region the selection can reach (`clip(canvas:)`,
//! :39-48); and that coverage on a layer's own pixel grid (`PixelAdjust.coverage`,
//! PixelAdjust.swift:23-34).
use super::{Contour, Selection, SUBPIXEL};
use crate::{blur_gray, Affine, Document, GrayRaster, Point};

/// One edge of an outline in region pixels, `y0 < y1`, `dir` +1 when the contour runs down.
#[derive(Clone, Copy)]
struct Edge { x0: f64, y0: f64, x1: f64, y1: f64, dir: f32 }

/// The outline's edges in the pixel grid of a `width` x `height` region whose top-left corner is
/// at document pixel (`left`, `top`), horizontal edges dropped.
fn edges(contours: &[Contour], left: f64, top: f64) -> Vec<Edge> {
    let mut out = Vec::new();
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let (ax, ay) = (a[0] as f64 / SUBPIXEL - left, a[1] as f64 / SUBPIXEL - top);
            let (bx, by) = (b[0] as f64 / SUBPIXEL - left, b[1] as f64 / SUBPIXEL - top);
            if ay == by { continue; }
            out.push(if ay < by { Edge { x0: ax, y0: ay, x1: bx, y1: by, dir: 1.0 } } else { Edge { x0: bx, y0: by, x1: ax, y1: ay, dir: -1.0 } });
        }
    }
    out.sort_by(|a, b| a.y0.total_cmp(&b.y0));
    out
}

/// Adds one edge's share of row `y` to `acc` (`width + 2` cells): the signed area it covers to its
/// right, spread over the cells it crosses (the accumulation rasteriser). Cells left of 0 fold into
/// cell 0 and cells right of `width` are dropped: neither changes a pixel inside the row.
fn accumulate(acc: &mut [f32], width: usize, e: &Edge, y: f64) {
    let (top, bottom) = (e.y0.max(y), e.y1.min(y + 1.0));
    if !(bottom > top) { return; }
    let dxdy = (e.x1 - e.x0) / (e.y1 - e.y0);
    // The piece of the edge inside this row, split where it crosses x = 0 and x = width.
    let (xa, xb) = (e.x0 + (top - e.y0) * dxdy, e.x0 + (bottom - e.y0) * dxdy);
    let mut cuts = vec![(top, xa)];
    for bound in [0.0, width as f64] {
        if (xa - bound) * (xb - bound) < 0.0 { let t = (bound - xa) / (xb - xa); cuts.push((top + (bottom - top) * t, bound)); }
    }
    cuts.push((bottom, xb));
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in cuts.windows(2) {
        let ((ya, mut x0), (yb, mut x1)) = (pair[0], pair[1]);
        let dy = yb - ya;
        if !(dy > 0.0) { continue; }
        if (x0 + x1) * 0.5 >= width as f64 { continue; }
        if (x0 + x1) * 0.5 <= 0.0 { x0 = 0.0; x1 = 0.0; }
        let d = (dy as f32) * e.dir;
        let (lo, hi) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (lo_floor, hi_ceil) = (lo.floor(), hi.ceil());
        let (i0, i1) = (lo_floor as usize, hi_ceil as usize);
        if i1 <= i0 + 1 {
            // Within one column: the area right of the piece's middle.
            let xm = (0.5 * (x0 + x1) - lo_floor) as f32;
            acc[i0] += d - d * xm;
            acc[i0 + 1] += d * xm;
        } else {
            let s = (1.0 / (hi - lo)) as f32;
            let f0 = (lo - lo_floor) as f32;
            let a0 = 0.5 * s * (1.0 - f0) * (1.0 - f0);
            let f1 = (hi - hi_ceil + 1.0) as f32;
            let am = 0.5 * s * f1 * f1;
            acc[i0] += d * a0;
            if i1 == i0 + 2 {
                acc[i0 + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = s * (1.5 - f0);
                acc[i0 + 1] += d * (a1 - a0);
                for cell in acc.iter_mut().take(i1 - 1).skip(i0 + 2) { *cell += d * s; }
                let a2 = a1 + (i1 - i0 - 3) as f32 * s;
                acc[i1 - 1] += d * (1.0 - a2 - am);
            }
            acc[i1] += d * am;
        }
    }
}

/// The outline filled into a `width` x `height` region whose top-left corner is document pixel
/// (`left`, `top`), by the nonzero winding rule, 255 inside and 0 outside. `antialiased`: each
/// pixel is the area of it the outline covers (Core Graphics' antialiased fill); else a pixel is
/// covered when its centre is (the aliased fill). Rows one at a time, so it holds one row of f32.
pub fn rasterize(contours: &[Contour], left: f64, top: f64, width: u32, height: u32, antialiased: bool) -> GrayRaster {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; w * h];
    let all = edges(contours, left, top);
    let mut next = 0usize;
    let mut active: Vec<Edge> = Vec::new();
    let mut acc = vec![0f32; w + 2];
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for y in 0..h {
        let (row_top, row_bottom) = (y as f64, y as f64 + 1.0);
        while next < all.len() && all[next].y0 < row_bottom { active.push(all[next]); next += 1; }
        active.retain(|e| e.y1 > row_top);
        let line = &mut out[y * w..(y + 1) * w];
        if antialiased {
            acc.iter_mut().for_each(|a| *a = 0.0);
            for e in &active { accumulate(&mut acc, w, e, row_top); }
            let mut sum = 0f32;
            for (x, px) in line.iter_mut().enumerate() {
                sum += acc[x];
                *px = (sum.abs().min(1.0) * 255.0).round() as u8;
            }
        } else {
            // Nonzero winding at the row's centre line, half-open at each edge's ends.
            let yc = y as f64 + 0.5;
            crossings.clear();
            for e in &active {
                if e.y0 <= yc && yc < e.y1 { crossings.push((e.x0 + (yc - e.y0) * (e.x1 - e.x0) / (e.y1 - e.y0), e.dir as i32)); }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            for i in 0..crossings.len() {
                winding += crossings[i].1;
                if winding == 0 || i + 1 == crossings.len() { continue; }
                // Pixels whose centres x + 0.5 lie in [this crossing, the next).
                let from = (crossings[i].0 - 0.5).ceil().max(0.0) as usize;
                let to = ((crossings[i + 1].0 - 0.5).ceil().max(0.0) as usize).min(w);
                for px in line.iter_mut().take(to).skip(from) { *px = 255; }
            }
        }
    }
    GrayRaster::from_bytes(width, height, out)
}

/// The selection's coverage over the part of the canvas it can reach: `clip(canvas:)`. `origin`
/// is the region's top-left document pixel; `coverage` None is an empty selection, which clips
/// every edit away.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectionClip { pub origin: (i64, i64), pub coverage: Option<GrayRaster> }

impl SelectionClip {
    /// The coverage bounds grown by a pixel, rounded out and cut to the canvas; there, the outline
    /// filled (antialiased when the selection is, or has a feather) and blurred by sigma feather / 2
    /// with the region's edge pixels repeated beyond it (`clampedToExtent`).
    pub fn new(selection: &Selection, canvas_width: u32, canvas_height: u32) -> SelectionClip {
        let none = SelectionClip { origin: (0, 0), coverage: None };
        if selection.is_empty() { return none; }
        let Some(b) = selection.coverage_bounds() else { return none };
        let (x0, y0) = (((b.x - 1.0).floor()).max(0.0), ((b.y - 1.0).floor()).max(0.0));
        let (x1, y1) = (((b.max_x() + 1.0).ceil()).min(canvas_width as f64), ((b.max_y() + 1.0).ceil()).min(canvas_height as f64));
        if !(x1 - x0 >= 1.0) || !(y1 - y0 >= 1.0) { return none; }
        let (w, h) = ((x1 - x0) as u32, (y1 - y0) as u32);
        let filled = rasterize(&selection.contours, x0, y0, w, h, selection.antialiased || selection.feather > 0.0);
        let coverage = if selection.feather > 0.0 { blur_gray(&filled, selection.feather / 2.0) } else { filled };
        SelectionClip { origin: (x0 as i64, y0 as i64), coverage: Some(coverage) }
    }

    /// The coverage at a document point: 0 outside the region, else sampled bilinearly between the
    /// region's pixel centres, its edge pixels repeated.
    pub fn at(&self, p: Point) -> f32 {
        let Some(c) = &self.coverage else { return 0.0 };
        let (x, y) = (p.x - self.origin.0 as f64, p.y - self.origin.1 as f64);
        if x < 0.0 || y < 0.0 || x >= c.width as f64 || y >= c.height as f64 { return 0.0; }
        crate::compositor::gray_sample(c, x, y, false)
    }

    /// The coverage on a `width` x `height` pixel grid that `to_document` places on the canvas (a
    /// layer's pixels or its mask's, `pixel_to_document`), each pixel sampled at its centre
    /// (`PixelAdjust.coverage`). A grid on the canvas's own pixels, moved by whole pixels, copies
    /// them exactly.
    pub fn on_grid(&self, to_document: &Affine, width: u32, height: u32) -> GrayRaster {
        let (w, h) = (width as usize, height as usize);
        let mut out = vec![0u8; w * h];
        let Some(c) = &self.coverage else { return GrayRaster::from_bytes(width, height, out) };
        let whole = |v: f64| v.fract() == 0.0;
        if to_document.a == 1.0 && to_document.d == 1.0 && to_document.b == 0.0 && to_document.c == 0.0 && whole(to_document.tx) && whole(to_document.ty) {
            let (dx, dy) = (to_document.tx as i64 - self.origin.0, to_document.ty as i64 - self.origin.1);
            let (cw, ch) = (c.width as i64, c.height as i64);
            for y in 0..h as i64 {
                let sy = y + dy;
                if sy < 0 || sy >= ch { continue; }
                let (x0, x1) = ((-dx).max(0), (cw - dx).min(w as i64));
                if x0 >= x1 { continue; }
                let src = &c.bytes()[(sy * cw + x0 + dx) as usize..(sy * cw + x1 + dx) as usize];
                out[(y * w as i64 + x0) as usize..(y * w as i64 + x1) as usize].copy_from_slice(src);
            }
        } else {
            for y in 0..h { for x in 0..w {
                let p = to_document.apply(Point { x: x as f64 + 0.5, y: y as f64 + 0.5 });
                out[y * w + x] = (self.at(p) * 255.0).round().clamp(0.0, 255.0) as u8;
            }}
        }
        GrayRaster::from_bytes(width, height, out)
    }
}

/// The document's selection on a pixel grid (`SelectionClip::on_grid`): None when nothing is
/// selected (an edit then reaches the whole grid), all zero for an empty selection.
pub fn selection_coverage(doc: &Document, to_document: &Affine, width: u32, height: u32) -> Option<GrayRaster> {
    let selection = doc.selection.as_ref()?;
    Some(SelectionClip::new(selection, doc.width, doc.height).on_grid(to_document, width, height))
}
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -16,6 +16,7 @@ pub mod blend;
 pub mod plan;
 pub mod preview;
 pub mod effects;
+pub mod selection;
 
 pub use adjust::settings::*;
 pub use adjust::levels::*;
@@ -45,3 +46,5 @@ pub use effects::*;
 pub use effects::settings::*;
 pub use effects::render::*;
 pub use ops::masks::blur_gray;
+pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
+pub use selection::coverage::{rasterize, selection_coverage, SelectionClip};
```

The selection lives in the `Document`, compared as content and never read from a file:

```diff
--- a/engine/src/document.rs
+++ b/engine/src/document.rs
@@ -125,12 +125,19 @@ pub struct Document {
     pub active_layer_id: Option<Uuid>,
     pub guides: Vec<crate::Guide>,
     pub unknown: serde_json::Map<String, serde_json::Value>,
+    /// The selection (Phase 4a): None is no selection, an empty one is an explicit empty
+    /// selection. Part of the document so undo covers it; never saved (EditorSession.swift:70-71),
+    /// so a document opened or made fresh has none.
+    pub selection: Option<crate::Selection>,
+    /// Issued by the engine's revision counter whenever `selection` changes (`Engine::edit`), so the
+    /// app fetches an outline only when there is a new one. Not content: `same_content` skips it.
+    pub selection_revision: u64,
 }
 
 impl Document {
     pub fn new(width: u32, height: u32) -> Document {
         Document { id: Uuid::new_v4(), width, height, resolution: DEFAULT_RESOLUTION, layers: vec![], active_layer_id: None,
-            guides: vec![], unknown: Default::default() }
+            guides: vec![], unknown: Default::default(), selection: None, selection_revision: 1 }
     }
     pub fn size(&self) -> Size { Size { width: self.width as f64, height: self.height as f64 } }
     pub fn manifest(&self) -> Manifest {
@@ -232,11 +239,12 @@ impl Document {
     ///
     /// Destructured without `..` on purpose: a field added to `Document` later is then a compile
     /// error here rather than a field silently left out of the undo comparison, which would make
-    /// edits to it quietly un-undoable.
+    /// edits to it quietly un-undoable. The selection is content (the Mac's `CanvasDocument`
+    /// equality includes it, so a selection change is one undo step); its revision is bookkeeping.
     pub fn same_content(&self, other: &Document) -> bool {
-        let Document { id, width, height, resolution, layers, active_layer_id: _, guides, unknown } = self;
+        let Document { id, width, height, resolution, layers, active_layer_id: _, guides, unknown, selection, selection_revision: _ } = self;
         *id == other.id && *width == other.width && *height == other.height
             && *resolution == other.resolution && *layers == other.layers
-            && *guides == other.guides && *unknown == other.unknown
+            && *guides == other.guides && *unknown == other.unknown && *selection == other.selection
     }
 }
```

```diff
--- a/engine/src/package.rs
+++ b/engine/src/package.rs
@@ -51,6 +51,8 @@ pub fn open_package(pkg: &Package) -> Result<Document, ProjectError> {
         id: manifest.document_id, width: manifest.width as u32, height: manifest.height as u32,
         resolution: manifest.resolution.unwrap_or(DEFAULT_RESOLUTION), layers, active_layer_id: manifest.active_layer_id,
         guides: manifest.guides.clone().unwrap_or_default(), unknown: manifest.unknown.clone(),
+        // The Mac never saves a selection (ProjectStore.swift:13-56): an opened project has none.
+        selection: None, selection_revision: 1,
     })
 }
 
```

```diff
--- a/engine/src/compositor.rs
+++ b/engine/src/compositor.rs
@@ -441,7 +441,7 @@ pub fn composite(doc: &Document, region: Rect, w: u32, h: u32) -> Raster { compo
 /// Draws a single layer with Normal blend and its opacity, ignoring masks and clipping (Image Size resampling).
 pub fn render_layer(target: &mut [u8], tw: u32, th: u32, region: Rect, layer: &Layer) {
     let doc = Document { id: Uuid::nil(), width: 1, height: 1, resolution: 72.0, layers: vec![layer.clone()], active_layer_id: None,
-        guides: vec![], unknown: Default::default() };
+        guides: vec![], unknown: Default::default(), selection: None, selection_revision: 1 };
     let plan = RenderPlan { nodes: vec![], sources: vec![], spatial_margin: 0.0 };
     let (pw, ph) = layer.pixels.as_ref().map_or((0, 0), |p| (p.width, p.height));
     let draw = LayerDraw { id: layer.id, transform: layer.transform, corners: None, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
```

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test selection_model` (11 tests), then the whole engine suite: 403 passed, 1 ignored (+11).

- [ ] **Step 5: Prove it bites**

(1) In `SelectionClip::new`, blur by `selection.feather` instead of `selection.feather / 2.0`: `a_feather_blurs_the_coverage_by_half_its_amount` fails. Restore. (2) In `rasterize`'s hard-edged branch, compute `to` from `crossings[i + 1].0 + 0.5` instead of `- 0.5`: `antialiasing_controls_edge_coverage` and `a_whole_pixel_rectangle_covers_exactly_its_pixels_with_or_without_antialiasing` fail (a pixel whose centre lies on the edge comes in). Restore. (3) In `ellipse`, use `KAPPA = 0.5`: `an_ellipse_fills_its_box_as_an_oval_with_soft_edges` fails. Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/selection/mod.rs engine/src/selection/geometry.rs engine/src/selection/coverage.rs engine/tests/selection_model.rs
git commit -- engine/Cargo.toml Cargo.lock engine/src/selection engine/src/lib.rs engine/src/document.rs engine/src/package.rs engine/src/compositor.rs engine/tests/selection_model.rs -m "feat: the selection model: a fixed-point outline, its booleans and its coverage" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: The selection commands, the tracer and the outline the ants draw

Every way the Mac changes a selection is one undo step (`setSelection`, Selection.swift:249-254). This task adds them as engine commands with the Mac's undo names: the Marquee's and the Lasso's outline (`SelectShape`: `finishLasso` and `applySelection`, :209-247), Select All, Deselect, Inverse, the outline's move (whole pixels, not re-clipped, :262-290), Expand, Contract and Feather (:292-342), what Crop, Canvas Size, Image Size and Flip Canvas do to it (ruling OQ11), and `CommandError::Refused` for a state an edit refuses (ruling OQ8). `DocumentState` gains the selection's summary and the engine sends the outline itself only when asked (`selection_outline`), traced again at the screen's resolution when it is very detailed and the view is below 1:1 (ruling OQ7). That needs the tracer, WandPixels.c's `wand_trace`, which Task 5's Magic Wand uses too, so it comes here. `selection_contains` answers where a press moves the outline (`canMoveSelection(at:)`, :256-260).

**Files:**
- Create: `engine/src/selection/trace.rs`, `engine/src/selection/outline.rs`, `engine/src/ops/selection.rs`
- Modify: `engine/src/selection/mod.rs`, `engine/src/lib.rs`, `engine/src/error.rs`, `engine/src/command.rs`, `engine/src/ops/mod.rs`, `engine/src/engine.rs`
- Create test: `engine/tests/selection_commands.rs`

**Interfaces:**
- Consumes: Task 3's model.
- Produces: `Command::{SelectShape { kind, points, mode, antialiased }, SelectAll, Deselect, InvertSelection, MoveSelection { dx, dy }, ExpandSelection { amount }, ContractSelection { amount }, FeatherSelection { amount }}` (JSON `{"type": "SelectShape", "kind": "Ellipse", "points": [[x, y], ...], "mode": "Add", "antialiased": true}` and so on); `CommandError::Refused(String)`; `ops::selection::{EMPTY_SELECTION, NO_SELECTION, apply_selection, select_shape, select_all, deselect, invert_selection, move_selection, resize_selection, feather_selection, flip_selection}`; `DocumentState.selection: Option<SelectionState>`; `Engine::selection_outline(id, step) -> Vec<f64>` (`[contours, then per contour its point count and x, y pairs]`, empty with no selection); `Engine::selection_contains(id, Point) -> bool`; `trace_pixels(mask: &[u8], width, height) -> Result<Vec<Vec<[i32; 2]>>, TraceError>` (pixel-corner loops of the nonzero pixels), `to_contours(loops) -> Vec<Contour>`, `opaque_pixels(&Raster)` and `dark_pixels(&GrayRaster)` (both `-> Result<Vec<Contour>, TraceError>`), `WAND_EDGE_LIMIT = 8_000_000`; `selection_lod(&Selection, w, h, step) -> Vec<Vec<Point>>`, `OUTLINE_DETAIL_LIMIT = 20_000`, `OUTLINE_MASK_LIMIT = 40_000_000`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/selection_commands.rs` ports SelectionTests (its shapes, points and undo counts) through `Engine::execute`, plus the summary, the flat outline and its level of detail:

```rust
//! The selection commands through `Engine::execute`, ported from Compositor for Mac's
//! CompositorTests/SelectionTests.swift with its own shapes, coordinates and expected values. What
//! those tests drive through the canvas (modifier keys, drags, the M and L keys) is the app's; here
//! the engine receives what the app sends.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
fn square(x: f64, y: f64, size: f64) -> Vec<Point> { vec![p(x, y), p(x + size, y), p(x + size, y + size), p(x, y + size)] }

/// A `width` x `height` document with one blank layer, as the Mac's `makeSession`.
fn session(width: u32, height: u32) -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, true).unwrap();
    (e, id)
}
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn lasso(e: &mut Engine, id: Uuid, points: Vec<Point>, mode: SelectionMode) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Freehand, points, mode, antialiased: true });
}
fn selection(e: &Engine, id: Uuid) -> Option<Selection> { e.document(id).unwrap().selection.clone() }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
/// Coverage 0-255 at a document pixel, as the Mac's tests read it.
fn coverage(e: &Engine, id: Uuid, x: u32, y: u32) -> u8 {
    let d = e.document(id).unwrap();
    let Some(s) = &d.selection else { return 0 };
    let c = SelectionClip::new(s, d.width, d.height).on_grid(&Affine::IDENTITY, d.width, d.height);
    c.bytes()[(y * d.width + x) as usize]
}

#[test]
fn replace_add_and_subtract_combine_outlines() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 40.0), SelectionMode::Replace);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 70, 70)), (255, 0));
    lasso(&mut e, id, square(50.0, 50.0, 40.0), SelectionMode::Add);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 70, 70)), (255, 255));
    lasso(&mut e, id, square(20.0, 20.0, 20.0), SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 30, 30), coverage(&e, id, 15, 15)), (0, 255));
    lasso(&mut e, id, square(60.0, 10.0, 20.0), SelectionMode::Replace);
    assert_eq!((coverage(&e, id, 70, 20), coverage(&e, id, 70, 70), coverage(&e, id, 15, 15)), (255, 0, 0));
}

#[test]
fn a_selection_is_clipped_to_the_canvas() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(-50.0, -50.0, 100.0), SelectionMode::Replace);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 50.0, 50.0)));
}

#[test]
fn an_empty_selection_is_distinct_from_no_selection() {
    let (mut e, id) = session(100, 100);
    let before = depth(&e, id);
    lasso(&mut e, id, square(0.0, 0.0, 50.0), SelectionMode::Subtract);
    assert!(selection(&e, id).is_none() && depth(&e, id) == before, "nothing to subtract from: no change, no step");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    let s = selection(&e, id).expect("an explicit empty selection");
    assert!(s.is_empty());
    assert_eq!(coverage(&e, id, 20, 20), 0);
    assert!(e.state(id).unwrap().selection.unwrap().empty);
    run(&mut e, id, Command::Deselect);
    assert!(selection(&e, id).is_none() && e.state(id).unwrap().selection.is_none());
}

#[test]
fn a_click_deselects_and_each_selection_change_is_one_undo_step() {
    let (mut e, id) = session(100, 100);
    let count = depth(&e, id);
    lasso(&mut e, id, square(10.0, 10.0, 40.0), SelectionMode::Replace);
    assert_eq!(depth(&e, id), count + 1);
    lasso(&mut e, id, vec![p(5.0, 5.0)], SelectionMode::Replace);
    assert!(selection(&e, id).is_none() && depth(&e, id) == count + 2, "a click deselects, as one step");
    e.undo(id).unwrap();
    assert!(!selection(&e, id).unwrap().is_empty());
    e.undo(id).unwrap();
    assert!(selection(&e, id).is_none());
    e.redo(id).unwrap();
    assert_eq!(coverage(&e, id, 30, 30), 255);
    // In Add or Subtract a click changes nothing and records nothing.
    let now = depth(&e, id);
    lasso(&mut e, id, vec![p(5.0, 5.0), p(9.0, 5.0)], SelectionMode::Add);
    assert_eq!((depth(&e, id), coverage(&e, id, 30, 30)), (now, 255));
}

#[test]
fn a_polygonal_outline_closes_on_its_first_corner() {
    // SelectionTests.polygonalCornersCanBeRemovedAndClosed, after the misplaced corner came off.
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Polygonal, points: vec![p(10.0, 10.0), p(90.0, 10.0), p(90.0, 90.0), p(10.0, 90.0)], mode: SelectionMode::Replace, antialiased: true });
    assert_eq!((coverage(&e, id, 80, 80), coverage(&e, id, 5, 50)), (255, 0));
}

#[test]
fn select_all_and_inverse() {
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectAll);
    assert_eq!((coverage(&e, id, 0, 0), coverage(&e, id, 99, 99)), (255, 255));
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 100.0, 100.0)));
    lasso(&mut e, id, square(0.0, 0.0, 50.0), SelectionMode::Replace);
    run(&mut e, id, Command::InvertSelection);
    assert_eq!((coverage(&e, id, 25, 25), coverage(&e, id, 75, 75)), (0, 255));
    // Inverse keeps the flags; with no selection it does nothing.
    run(&mut e, id, Command::FeatherSelection { amount: 3 });
    run(&mut e, id, Command::InvertSelection);
    assert_eq!(selection(&e, id).unwrap().feather, 3.0);
    run(&mut e, id, Command::Deselect);
    let before = depth(&e, id);
    run(&mut e, id, Command::InvertSelection);
    assert!(selection(&e, id).is_none() && depth(&e, id) == before);
}

#[test]
fn dragging_moves_the_outline_in_whole_pixels_as_one_undo_step() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    assert!(selection(&e, id).unwrap().contains(p(20.0, 20.0)) && !selection(&e, id).unwrap().contains(p(60.0, 60.0)));
    let count = depth(&e, id);
    // The drag's last offset, as the Mac's moveSelection(by: 40.2, 40.4) rounds it.
    run(&mut e, id, Command::MoveSelection { dx: 40.2, dy: 40.4 });
    assert_eq!(depth(&e, id), count + 1);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(50.0, 50.0, 20.0, 20.0)));
    assert_eq!((coverage(&e, id, 55, 55), coverage(&e, id, 15, 15)), (255, 0));
    e.undo(id).unwrap();
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(10.0, 10.0, 20.0, 20.0)));
}

#[test]
fn moving_off_the_canvas_and_back_keeps_the_whole_shape() {
    let (mut e, id) = session(100, 100);
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    run(&mut e, id, Command::MoveSelection { dx: -25.0, dy: 0.0 });
    assert_eq!(coverage(&e, id, 0, 20), 255);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(-15.0, 10.0, 20.0, 20.0)), "not cut to the canvas");
    run(&mut e, id, Command::MoveSelection { dx: 25.0, dy: 0.0 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(10.0, 10.0, 20.0, 20.0)));
}

#[test]
fn nudges_are_one_step_each_and_need_a_real_selection() {
    let (mut e, id) = session(100, 100);
    assert!(matches!(e.execute(id, Command::MoveSelection { dx: 1.0, dy: 0.0 }), Err(CommandError::Refused(_))), "nothing selected");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    let count = depth(&e, id);
    run(&mut e, id, Command::MoveSelection { dx: 1.0, dy: 0.0 });
    run(&mut e, id, Command::MoveSelection { dx: 0.0, dy: -10.0 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(11.0, 0.0, 20.0, 20.0)));
    assert_eq!(depth(&e, id), count + 2);
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    assert!(selection(&e, id).unwrap().is_empty());
    assert_eq!(e.execute(id, Command::MoveSelection { dx: 1.0, dy: 0.0 }).unwrap_err(), CommandError::Refused("The selection is empty".into()));
}

#[test]
fn expand_and_contract_grow_and_shrink_the_outline() {
    let (mut e, id) = session(100, 100);
    assert!(e.execute(id, Command::ExpandSelection { amount: 5 }).is_err(), "nothing to modify");
    lasso(&mut e, id, square(40.0, 40.0, 20.0), SelectionMode::Replace);
    run(&mut e, id, Command::ExpandSelection { amount: 5 });
    let grown = selection(&e, id).unwrap().bounds().unwrap();
    assert!((grown.x - 35.0).abs() < 0.01 && (grown.width - 30.0).abs() < 0.01, "{grown:?}");
    assert_eq!((coverage(&e, id, 37, 50), coverage(&e, id, 33, 50)), (255, 0));
    run(&mut e, id, Command::ContractSelection { amount: 8 });
    let shrunk = selection(&e, id).unwrap().bounds().unwrap();
    assert!((shrunk.x - 43.0).abs() < 0.01 && (shrunk.width - 14.0).abs() < 0.01, "{shrunk:?}");
    e.undo(id).unwrap();
    assert!((selection(&e, id).unwrap().bounds().unwrap().width - 30.0).abs() < 0.01);
    for bad in [0, 501] { assert!(e.execute(id, Command::ExpandSelection { amount: bad }).is_err(), "{bad}"); }
}

#[test]
fn expand_stays_on_the_canvas_and_contract_can_empty_the_selection() {
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectAll);
    run(&mut e, id, Command::ExpandSelection { amount: 10 });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 0.0, 100.0, 100.0)));
    run(&mut e, id, Command::ContractSelection { amount: 10 });
    assert_eq!((coverage(&e, id, 5, 50), coverage(&e, id, 50, 50)), (0, 255), "it pulls in from the canvas edges too");
    run(&mut e, id, Command::ContractSelection { amount: 45 });
    assert!(selection(&e, id).unwrap().is_empty());
    assert!(e.execute(id, Command::ContractSelection { amount: 1 }).is_err(), "an empty selection cannot be modified");
}

#[test]
fn feathers_combine_as_blurs_do_and_stop_at_250() {
    let (mut e, id) = session(60, 20);
    run(&mut e, id, Command::SelectAll);
    run(&mut e, id, Command::FeatherSelection { amount: 6 });
    assert_eq!(selection(&e, id).unwrap().feather, 6.0);
    run(&mut e, id, Command::FeatherSelection { amount: 8 });
    assert_eq!(selection(&e, id).unwrap().feather, (36.0f64 + 64.0).sqrt());
    run(&mut e, id, Command::FeatherSelection { amount: 250 });
    assert_eq!(selection(&e, id).unwrap().feather, 250.0);
    assert!(e.execute(id, Command::FeatherSelection { amount: 251 }).is_err());
}

#[test]
fn the_marquee_selects_its_whole_pixel_box_and_a_click_deselects() {
    // SelectionTests.marqueeDrawsWholePixelRectanglesInAnyDirection, after the app's DragBox has
    // rounded the drag from (60.4, 70.6) to (20.2, 30.3) to the box (20, 30)-(60, 71).
    let (mut e, id) = session(100, 100);
    let marquee = |e: &mut Engine, x0: f64, y0: f64, x1: f64, y1: f64, mode: SelectionMode| {
        run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], mode, antialiased: true });
    };
    marquee(&mut e, 20.0, 30.0, 60.0, 71.0, SelectionMode::Replace);
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(20.0, 30.0, 40.0, 41.0)));
    assert_eq!((coverage(&e, id, 20, 30), coverage(&e, id, 19, 30)), (255, 0));
    marquee(&mut e, 80.0, 80.0, 90.0, 90.0, SelectionMode::Add);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 40, 50)), (255, 255));
    marquee(&mut e, 30.0, 40.0, 50.0, 60.0, SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 40, 50), coverage(&e, id, 25, 35)), (0, 255));
    marquee(&mut e, 5.0, 5.0, 5.0, 5.0, SelectionMode::Replace);
    assert!(selection(&e, id).is_none());
}

#[test]
fn the_elliptical_marquee_selects_an_oval_in_its_box() {
    // SelectionTests.marqueeEllipseSelectsAnOvalInItsBoxAndShiftMakesACircle.
    let (mut e, id) = session(100, 100);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(10.0, 20.0), p(70.0, 20.0), p(70.0, 60.0), p(10.0, 60.0)], mode: SelectionMode::Replace, antialiased: true });
    let b = selection(&e, id).unwrap().bounds().unwrap();
    assert!((b.x - 10.0).abs() < 0.5 && (b.max_x() - 70.0).abs() < 0.5 && (b.y - 20.0).abs() < 0.5 && (b.max_y() - 60.0).abs() < 0.5);
    assert_eq!((coverage(&e, id, 40, 40), coverage(&e, id, 11, 21)), (255, 0));
}

#[test]
fn the_selection_is_part_of_the_document_but_never_saved() {
    let (mut e, id) = session(80, 60);
    let first = e.state(id).unwrap();
    assert!(first.selection.is_none(), "a new document has none");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    let r1 = e.state(id).unwrap().selection.unwrap().revision;
    run(&mut e, id, Command::MoveSelection { dx: 3.0, dy: 0.0 });
    let r2 = e.state(id).unwrap().selection.unwrap().revision;
    assert!(r2 > r1, "a new outline, a new revision");
    let manifest = e.save_package(id).unwrap().manifest_json;
    assert!(!manifest.contains("selection"), "the Mac's manifest has no selection (ProjectStore.swift:13-56)");
    let reopened = e.open_package(&e.save_package(id).unwrap(), None).unwrap();
    assert!(e.state(reopened).unwrap().selection.is_none(), "opening starts with no selection");
    // Undo returns the earlier outline with its own revision; a different edit then gets a new one.
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().selection.unwrap().revision, r1);
    run(&mut e, id, Command::MoveSelection { dx: 0.0, dy: 4.0 });
    let r3 = e.state(id).unwrap().selection.unwrap().revision;
    assert!(r3 > r2, "never a revision an earlier outline had");
    let s = e.state(id).unwrap().selection.unwrap();
    assert_eq!((s.bounds, s.points, s.antialiased, s.feather, s.empty), (Some(rect(10.0, 14.0, 20.0, 20.0)), 4, true, 0.0, false));
}

#[test]
fn crop_canvas_size_and_image_size_drop_the_selection_and_flip_canvas_mirrors_it() {
    let (mut e, id) = session(100, 80);
    lasso(&mut e, id, vec![p(10.0, 20.0), p(40.0, 20.0), p(40.0, 60.0), p(10.0, 60.0)], SelectionMode::Replace);
    run(&mut e, id, Command::FlipCanvas { horizontal: true });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(60.0, 20.0, 30.0, 40.0)));
    run(&mut e, id, Command::FlipCanvas { horizontal: false });
    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(60.0, 20.0, 30.0, 40.0)), "top-bottom symmetric about 40");
    for command in [Command::CanvasSize { width: 120, height: 90, anchor: 4, fill: None }, Command::Crop { x: 5.0, y: 5.0, width: 50.0, height: 50.0 },
        Command::ImageSize { width: 50, height: 40, resolution: 72.0, sampling: Sampling::High }] {
        run(&mut e, id, command.clone());
        assert!(selection(&e, id).is_none(), "{command:?} drops the selection");
        e.undo(id).unwrap();
        assert!(selection(&e, id).is_some(), "{command:?}: undo brings it back");
    }
}

#[test]
fn the_commands_read_as_json_and_carry_the_macs_undo_names() {
    let c: Command = serde_json::from_str(r#"{"type":"SelectShape","kind":"Ellipse","points":[[1,2],[9,2],[9,7],[1,7]],"mode":"Subtract","antialiased":false}"#).unwrap();
    assert_eq!(c, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(1.0, 2.0), p(9.0, 2.0), p(9.0, 7.0), p(1.0, 7.0)], mode: SelectionMode::Subtract, antialiased: false });
    let names: Vec<(Command, &str)> = vec![
        (Command::SelectShape { kind: SelectionShape::Freehand, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Lasso"),
        (Command::SelectShape { kind: SelectionShape::Polygonal, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Polygonal Lasso"),
        (Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![], mode: SelectionMode::Replace, antialiased: true }, "Rectangular Marquee"),
        (c, "Elliptical Marquee"),
        (Command::SelectAll, "Select All"), (Command::Deselect, "Deselect"), (Command::InvertSelection, "Inverse"),
        (Command::MoveSelection { dx: 1.0, dy: 0.0 }, "Move Selection"), (Command::ExpandSelection { amount: 1 }, "Expand Selection"),
        (Command::ContractSelection { amount: 1 }, "Contract Selection"), (Command::FeatherSelection { amount: 1 }, "Feather Selection"),
    ];
    for (command, name) in names { assert_eq!(command.action_name(), name); }
}

#[test]
fn the_outline_travels_flat_and_a_complex_one_at_screen_resolution_below_one_to_one() {
    let (mut e, id) = session(100, 80);
    assert!(e.selection_outline(id, 1.0).unwrap().is_empty(), "nothing selected");
    lasso(&mut e, id, vec![p(10.0, 20.0), p(40.0, 20.0), p(40.0, 60.0)], SelectionMode::Replace);
    let flat = e.selection_outline(id, 0.25).unwrap();
    assert_eq!(flat[0], 1.0, "one contour");
    assert_eq!(flat[1], 3.0, "of three points");
    let mut points: Vec<(f64, f64)> = flat[2..].chunks(2).map(|c| (c[0], c[1])).collect();
    points.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(points, vec![(10.0, 20.0), (40.0, 20.0), (40.0, 60.0)], "a simple outline travels as it is, at any zoom");
    // A freehand outline with a fine sawtooth along its top: 24,000 teeth corners, half a pixel
    // deep, over (0, 20)-(90, 20), then down to (90, 60) and back to (0, 60).
    let mut teeth: Vec<Point> = (0..24_000).map(|k| p(k as f64 * 90.0 / 24_000.0, 20.0 + (k % 2) as f64 * 0.5)).collect();
    teeth.extend([p(90.0, 60.0), p(0.0, 60.0)]);
    lasso(&mut e, id, teeth, SelectionMode::Replace);
    let points = selection(&e, id).unwrap().point_count();
    assert!(points > OUTLINE_DETAIL_LIMIT, "{points} points");
    let full = e.selection_outline(id, 1.0).unwrap();
    assert_eq!((full[0], full[1] as usize), (1.0, points), "at 1:1 the outline travels as it is");
    // At a quarter the outline is filled into a 23 x 10 mask (4 document pixels a cell), any
    // coverage counting, and traced: the band (0, 20)-(90, 60) rounded out to the cells, one loop
    // of four corners, (0, 20)-(92, 60) in document pixels.
    let lod = e.selection_outline(id, 0.25).unwrap();
    assert_eq!((lod[0], lod[1]), (1.0, 4.0), "{lod:?}");
    let mut corners: Vec<(f64, f64)> = lod[2..].chunks(2).map(|c| (c[0], c[1])).collect();
    corners.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(corners, vec![(0.0, 20.0), (0.0, 60.0), (92.0, 20.0), (92.0, 60.0)]);
}


#[test]
fn a_press_moves_the_outline_only_inside_a_selection_with_something_in_it() {
    // SelectionTests.draggingMovesTheOutlineInWholePixelsAsOneUndo / arrowNudgesAndMoveIsOnlyForNewModeOnARealSelection.
    let (mut e, id) = session(100, 100);
    assert!(!e.selection_contains(id, p(20.0, 20.0)).unwrap(), "nothing selected");
    lasso(&mut e, id, square(10.0, 10.0, 20.0), SelectionMode::Replace);
    assert!(e.selection_contains(id, p(20.0, 20.0)).unwrap());
    assert!(!e.selection_contains(id, p(60.0, 60.0)).unwrap());
    lasso(&mut e, id, square(0.0, 0.0, 60.0), SelectionMode::Subtract);
    assert!(!e.selection_contains(id, p(20.0, 20.0)).unwrap(), "an empty selection does not move");
}
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test selection_commands`: it does not compile (the commands, `selection_outline`, `selection_contains` and `DocumentState.selection` do not exist).

- [ ] **Step 3: The tracer and the level of detail**

```rust
//! Pixels back into outlines: `wand_trace` (Compositor for Mac's WandPixels.c:87-174), line for
//! line, and the Mac's `MaskTracing` (Document/MaskTracing.swift:4-70), which traces a layer's
//! opaque pixels or a mask's dark ones by the same pixel-edge rule. One tracer serves both here.
use super::{Contour, SUBPIXEL};
use crate::{GrayRaster, Raster};

const EAST: u8 = 1;
const SOUTH: u8 = 2;
const WEST: u8 = 4;
const NORTH: u8 = 8;
/// Outlines with more pixel edges than this are refused: the path would be too slow to draw
/// (`wand_edge_limit`, WandPixels.c:7).
pub const WAND_EDGE_LIMIT: usize = 8_000_000;

/// Why a trace gave no outline (`wand_trace` returns -2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceError { TooDetailed }

// Headings, clockwise on screen (y grows downward): east, south, west, north.
fn turn_right(d: u8) -> u8 { if d == NORTH { EAST } else { d << 1 } }
fn turn_left(d: u8) -> u8 { if d == EAST { NORTH } else { d >> 1 } }
fn lowest(bits: u8) -> u8 { bits & bits.wrapping_neg() }

/// The outline of `mask`'s nonzero pixels along exact pixel edges, as corner points in pixel
/// units: one loop per boundary, outer boundaries clockwise on screen and holes the other way, so
/// the winding rule fills exactly the traced pixels. Where two loops touch at a corner the walk
/// turns right, which keeps them apart. `wand_trace`, WandPixels.c:91-174.
pub fn trace_pixels(mask: &[u8], width: usize, height: usize) -> Result<Vec<Vec<[i32; 2]>>, TraceError> {
    if width == 0 || height == 0 { return Ok(Vec::new()); }
    // Each vertex of the (width + 1) x (height + 1) grid records the directed boundary edges
    // leaving it: a selected pixel's unselected sides, walked clockwise around the pixel.
    let stride = width + 1;
    let vertices = stride * (height + 1);
    let mut out = vec![0u8; vertices];
    let mut edges = 0usize;
    for y in 0..height {
        let row = &mask[y * width..(y + 1) * width];
        for x in 0..width {
            if row[x] == 0 { continue; }
            if y == 0 || mask[(y - 1) * width + x] == 0 { out[y * stride + x] |= EAST; edges += 1; }
            if x + 1 == width || row[x + 1] == 0 { out[y * stride + x + 1] |= SOUTH; edges += 1; }
            if y + 1 == height || mask[(y + 1) * width + x] == 0 { out[(y + 1) * stride + x + 1] |= WEST; edges += 1; }
            if x == 0 || row[x - 1] == 0 { out[(y + 1) * stride + x] |= NORTH; edges += 1; }
        }
        if edges > WAND_EDGE_LIMIT { return Err(TraceError::TooDetailed); }
    }
    let mut loops = Vec::new();
    for start in 0..vertices {
        while out[start] != 0 {
            let mut points: Vec<[i32; 2]> = Vec::new();
            let (mut v, mut heading, mut initial) = (start, 0u8, 0u8);
            loop {
                let bits = out[v];
                let d = if heading == 0 { lowest(bits) }
                    else if bits & turn_right(heading) != 0 { turn_right(heading) }
                    else if bits & heading != 0 { heading }
                    else if bits & turn_left(heading) != 0 { turn_left(heading) }
                    else { lowest(bits) };
                if d == 0 { break; }
                out[v] &= !d;
                if d != heading { points.push([(v % stride) as i32, (v / stride) as i32]); }
                if heading == 0 { initial = d; }
                heading = d;
                v = match d { EAST => v + 1, WEST => v - 1, SOUTH => v + stride, _ => v - stride };
                if v == start { break; }
            }
            // The start is a corner unless the loop arrives on the heading it left with.
            if heading == initial && !points.is_empty() { points.remove(0); }
            loops.push(points);
        }
    }
    Ok(loops)
}

/// Pixel-unit loops as selection contours, `SUBPIXEL` units, pixel (0, 0)'s corner at the origin.
pub fn to_contours(loops: Vec<Vec<[i32; 2]>>) -> Vec<Contour> {
    let s = SUBPIXEL as i32;
    loops.into_iter().filter(|l| l.len() >= 3).map(|l| l.into_iter().map(|p| [p[0] * s, p[1] * s]).collect()).collect()
}

/// A layer's pixels at least 50% opaque, traced (`MaskTracing.opaquePixels`): alpha 128 and up.
pub fn opaque_pixels(raster: &Raster) -> Result<Vec<Contour>, TraceError> {
    let mask: Vec<u8> = raster.bytes().chunks_exact(4).map(|p| (p[3] >= 128) as u8).collect();
    Ok(to_contours(trace_pixels(&mask, raster.width as usize, raster.height as usize)?))
}

/// A mask's pixels darker than 50% grey, traced (`MaskTracing.darkPixels`): below 128.
pub fn dark_pixels(mask: &GrayRaster) -> Result<Vec<Contour>, TraceError> {
    let dark: Vec<u8> = mask.bytes().iter().map(|&v| (v < 128) as u8).collect();
    Ok(to_contours(trace_pixels(&dark, mask.width as usize, mask.height as usize)?))
}
```

```rust
//! The outline the marching ants draw (TransformOverlay.swift:92-177): the selection's own
//! contours, or, for a complex outline seen below 1:1, one traced at screen resolution so a
//! redraw never strokes more edges than the screen has pixels for.
use super::coverage::rasterize;
use super::trace::trace_pixels;
use super::{Selection, SUBPIXEL};
use crate::Point;

/// Outlines with more points than this are sent as they are only at 1:1 and closer
/// (`fullDetailLimit`, TransformOverlay.swift:100, counts path elements).
pub const OUTLINE_DETAIL_LIMIT: usize = 20_000;
/// The largest mask a screen-resolution outline is traced from (about 40 megapixels, as the Mac's).
pub const OUTLINE_MASK_LIMIT: u64 = 40_000_000;

/// The selection traced at `step` screen pixels per document pixel (a power of two below 1): its
/// outline filled into a mask that fine, over its bounds cut to the canvas, any coverage counting,
/// and traced along that mask's pixel edges, in document pixels. `traceOutline`,
/// TransformOverlay.swift:141-177. A mask over `OUTLINE_MASK_LIMIT`, or a trace over
/// `WAND_EDGE_LIMIT`, is made at half the step instead.
pub fn selection_lod(selection: &Selection, canvas_width: u32, canvas_height: u32, step: f64) -> Vec<Vec<Point>> {
    let Some(b) = selection.bounds() else { return Vec::new() };
    let (x0, y0) = (b.x.floor().max(0.0), b.y.floor().max(0.0));
    let (x1, y1) = (b.max_x().ceil().min(canvas_width as f64), b.max_y().ceil().min(canvas_height as f64));
    if !(x1 - x0 >= 1.0 && y1 - y0 >= 1.0) { return Vec::new(); }
    let mut step = step.clamp(1.0 / 4096.0, 1.0);
    loop {
        let (w, h) = (((x1 - x0) * step).ceil().max(1.0) as u32, ((y1 - y0) * step).ceil().max(1.0) as u32);
        if (w as u64) * (h as u64) > OUTLINE_MASK_LIMIT && step > 1.0 / 4096.0 { step /= 2.0; continue; }
        // The contours in the mask's pixels: moved to the region's corner, scaled by the step.
        let scaled: Vec<_> = selection.contours.iter().map(|c| c.iter().map(|p| [
            ((p[0] as f64 - x0 * SUBPIXEL) * step).round() as i32, ((p[1] as f64 - y0 * SUBPIXEL) * step).round() as i32,
        ]).collect()).collect();
        let filled = rasterize(&scaled, 0.0, 0.0, w, h, true);
        let mask: Vec<u8> = filled.bytes().iter().map(|&v| (v > 0) as u8).collect();
        match trace_pixels(&mask, w as usize, h as usize) {
            Ok(loops) => return loops.into_iter().map(|l| l.into_iter().map(|q| Point { x: x0 + q[0] as f64 / step, y: y0 + q[1] as f64 / step }).collect()).collect(),
            Err(_) if step > 1.0 / 4096.0 => step /= 2.0,
            Err(_) => return Vec::new(),
        }
    }
}
```

```diff
--- a/engine/src/selection/mod.rs
+++ b/engine/src/selection/mod.rs
@@ -8,6 +8,8 @@
 //! pixels back into outlines (WandPixels.c `wand_trace`); `wand` is the Magic Wand's matcher.
 pub mod coverage;
 pub mod geometry;
+pub mod outline;
+pub mod trace;
 
 use crate::{Point, Rect};
 use serde::{Deserialize, Serialize};
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -48,3 +48,5 @@ pub use effects::render::*;
 pub use ops::masks::blur_gray;
 pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
 pub use selection::coverage::{rasterize, selection_coverage, SelectionClip};
+pub use selection::outline::{selection_lod, OUTLINE_DETAIL_LIMIT, OUTLINE_MASK_LIMIT};
+pub use selection::trace::{dark_pixels, opaque_pixels, trace_pixels, TraceError, WAND_EDGE_LIMIT};
```

- [ ] **Step 4: The commands**

```diff
--- a/engine/src/error.rs
+++ b/engine/src/error.rs
@@ -57,4 +57,8 @@ pub enum CommandError {
     Export(#[from] ExportError),
     #[error("Invalid argument: {0}")]
     Argument(String),
+    /// An edit the document's state does not allow, said as the user should read it: an empty
+    /// selection, a Magic Wand outline too detailed to draw (Phase 4a).
+    #[error("{0}")]
+    Refused(String),
 }
```

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -1,4 +1,4 @@
-use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling};
+use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape};
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
@@ -49,6 +49,17 @@ pub enum Command {
     ApplyFilter { #[serde(with = "ids::upper")] id: Uuid, params: FilterParams },
     AddAdjustmentLayer { kind: AdjustmentKind, #[serde(default)] seed: u32, #[serde(default)] shadows: Option<[f64; 3]>, #[serde(default)] highlights: Option<[f64; 3]> },
     SetAdjustment { #[serde(with = "ids::upper")] id: Uuid, adjustment: LayerAdjustment },
+    // The selection (Phase 4a). Each is one undo step, as `setSelection` makes it.
+    /// A Marquee's box (its four corners) or a Lasso's outline, closed and combined by `mode`.
+    SelectShape { kind: SelectionShape, points: Vec<Point>, mode: SelectionMode, antialiased: bool },
+    SelectAll,
+    Deselect,
+    InvertSelection,
+    /// The outline moved by whole pixels (`dx` and `dy` rounded).
+    MoveSelection { dx: f64, dy: f64 },
+    ExpandSelection { amount: u32 },
+    ContractSelection { amount: u32 },
+    FeatherSelection { amount: u32 },
 }
 
 impl Command {
@@ -100,6 +111,15 @@ impl Command {
             Command::ApplyFilter { params, .. } => params.name(),
             Command::AddAdjustmentLayer { kind, .. } => kind.new_action_name(),
             Command::SetAdjustment { adjustment, .. } => adjustment.kind.edit_action_name(),
+            // The Mac's undo names (Selection.swift:228-358, MagicWand.swift:119, MaskTracing.swift:82-93).
+            Command::SelectShape { kind, .. } => kind.action_name(),
+            Command::SelectAll => "Select All",
+            Command::Deselect => "Deselect",
+            Command::InvertSelection => "Inverse",
+            Command::MoveSelection { .. } => "Move Selection",
+            Command::ExpandSelection { .. } => "Expand Selection",
+            Command::ContractSelection { .. } => "Contract Selection",
+            Command::FeatherSelection { .. } => "Feather Selection",
         }
     }
 }
```

```rust
//! The selection commands (Phase 4a): Compositor for Mac's `applySelection`, the Marquee and
//! Lasso's `finishLasso`, Select All / Deselect / Inverse, the outline move, Expand / Contract /
//! Feather (Document/Selection.swift:141-359), the Magic Wand (Document/MagicWand.swift:98-137) and
//! loading a layer's or a mask's pixels as a selection (Document/MaskTracing.swift:73-94).
use crate::selection::geometry::{self as g, Boolean};
use crate::*;
use uuid::Uuid;

/// Said when an edit needs a selection with something in it.
pub const EMPTY_SELECTION: &str = "The selection is empty";
/// Said when an edit needs a selection and there is none.
pub const NO_SELECTION: &str = "Nothing is selected";
fn refused(message: &str) -> CommandError { CommandError::Refused(message.to_string()) }
fn canvas(doc: &Document) -> Vec<Contour> { vec![g::rectangle(Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 })] }
fn check_point(p: Point) -> Result<(), CommandError> {
    if p.x.is_finite() && p.y.is_finite() && p.x.abs() <= SELECTION_COORDINATE_LIMIT && p.y.abs() <= SELECTION_COORDINATE_LIMIT { Ok(()) }
    else { Err(CommandError::Argument("outline points must lie within 1,000,000 pixels of the canvas".into())) }
}
/// The current selection when it has something in it; the refusal otherwise.
fn modifiable(doc: &Document) -> Result<&Selection, CommandError> {
    let s = doc.selection.as_ref().ok_or_else(|| refused(NO_SELECTION))?;
    if s.is_empty() { return Err(refused(EMPTY_SELECTION)); }
    Ok(s)
}

/// `applySelection`: `shape`, cut to the canvas, becomes the selection (Replace), joins it (Add) or
/// is taken out of it (Subtract; with no selection that changes nothing). The result takes the
/// given anti-alias flag and no feather, as the Mac's `DocumentSelection(path:antialiased:)` does.
pub fn apply_selection(doc: &mut Document, shape: &[Contour], mode: SelectionMode, antialiased: bool) {
    let clipped = g::combine(shape, &canvas(doc), Boolean::Intersection);
    let result = match (mode, &doc.selection) {
        (SelectionMode::Replace, _) | (SelectionMode::Add, None) => clipped,
        (SelectionMode::Add, Some(current)) => g::combine(&current.contours, &clipped, Boolean::Union),
        (SelectionMode::Subtract, None) => return,
        (SelectionMode::Subtract, Some(current)) => g::combine(&current.contours, &clipped, Boolean::Difference),
    };
    doc.selection = Some(Selection::new(result, antialiased, 0.0));
}

/// `finishLasso`: the drawn outline closed and combined by `mode`. A Marquee sends its box's four
/// corners; an Ellipse fills that box (`addEllipse(in:)`). A click or a line, enclosing nothing,
/// deselects in Replace and changes nothing otherwise.
pub fn select_shape(doc: &mut Document, kind: SelectionShape, points: &[Point], mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
    for p in points { check_point(*p)?; }
    let (xs, ys) = (points.iter().map(|p| p.x), points.iter().map(|p| p.y));
    let (x0, x1) = (xs.clone().fold(f64::INFINITY, f64::min), xs.fold(f64::NEG_INFINITY, f64::max));
    let (y0, y1) = (ys.clone().fold(f64::INFINITY, f64::min), ys.fold(f64::NEG_INFINITY, f64::max));
    let is_ellipse = kind == SelectionShape::Ellipse && points.len() == 4;
    if !(points.len() >= 3 || kind == SelectionShape::Ellipse) || !(x1 - x0 > 0.0) || !(y1 - y0 > 0.0) {
        if mode == SelectionMode::Replace { doc.selection = None; }
        return Ok(());
    }
    let outline = if is_ellipse { g::ellipse(Rect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }) } else { g::polygon(points) };
    apply_selection(doc, &[outline], mode, antialiased);
    Ok(())
}

/// Select All: the canvas, antialiased, no feather.
pub fn select_all(doc: &mut Document) { doc.selection = Some(Selection::new(canvas(doc), true, 0.0)); }

/// Deselect: no selection.
pub fn deselect(doc: &mut Document) { doc.selection = None; }

/// Inverse: the canvas minus the selection, its flags kept; nothing without one.
pub fn invert_selection(doc: &mut Document) {
    let Some(current) = doc.selection.clone() else { return };
    doc.selection = Some(Selection::new(g::combine(&canvas(doc), &current.contours, Boolean::Difference), current.antialiased, current.feather));
}

/// The outline moved by whole pixels (`moveSelection(by:)`: offsets rounded), not cut to the canvas,
/// so it can leave and come back whole.
pub fn move_selection(doc: &mut Document, dx: f64, dy: f64) -> Result<(), CommandError> {
    if !dx.is_finite() || !dy.is_finite() { return Err(CommandError::Argument("the offset must be finite".into())); }
    let current = modifiable(doc)?;
    let (dx, dy) = (dx.round(), dy.round());
    let b = current.bounds().unwrap_or(Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 });
    for p in [Point { x: b.x + dx, y: b.y + dy }, Point { x: b.max_x() + dx, y: b.max_y() + dy }] { check_point(p)?; }
    doc.selection = Some(current.translated((dx * SUBPIXEL) as i32, (dy * SUBPIXEL) as i32));
    Ok(())
}

/// Expand (`delta` > 0) or Contract: the round-capped, round-joined band `|delta|` wide either
/// side of the outline, added and cut to the canvas, or taken away (so Contract also moves in from
/// the canvas edges, and can leave an empty selection). `resizeSelection`, Selection.swift:333-342.
pub fn resize_selection(doc: &mut Document, delta: i64) -> Result<(), CommandError> {
    if delta == 0 || delta.unsigned_abs() > MAX_RESIZE as u64 { return Err(CommandError::Argument("Expand and Contract take 1 to 500 pixels".into())); }
    let current = modifiable(doc)?.clone();
    let band = g::band(&current.contours, delta.unsigned_abs() as f64);
    let result = if delta > 0 {
        g::combine(&g::combine(&current.contours, &band, Boolean::Union), &canvas(doc), Boolean::Intersection)
    } else {
        g::combine(&current.contours, &band, Boolean::Difference)
    };
    doc.selection = Some(Selection::new(result, current.antialiased, current.feather));
    Ok(())
}

/// Feather: two soft edges together spread as blurs do, sqrt(a^2 + b^2), at most 250.
pub fn feather_selection(doc: &mut Document, amount: u32) -> Result<(), CommandError> {
    if !(1..=MAX_FEATHER as u32).contains(&amount) { return Err(CommandError::Argument("Feather takes 1 to 250 pixels".into())); }
    let current = modifiable(doc)?;
    let softened = (current.feather * current.feather + (amount as f64) * (amount as f64)).sqrt().min(MAX_FEATHER);
    doc.selection = Some(Selection { feather: softened, ..current.clone() });
    Ok(())
}

/// The mirrored selection for Flip Canvas (LayerFlip.swift:69-76).
pub fn flip_selection(doc: &mut Document, horizontal: bool) {
    let (w, h) = ((doc.width as f64 * SUBPIXEL) as i32, (doc.height as f64 * SUBPIXEL) as i32);
    if let Some(s) = &mut doc.selection {
        for c in &mut s.contours { for p in c.iter_mut() { if horizontal { p[0] = w - p[0]; } else { p[1] = h - p[1]; } } }
    }
}
```

```diff
--- a/engine/src/ops/mod.rs
+++ b/engine/src/ops/mod.rs
@@ -9,3 +9,4 @@ pub mod transform;
 pub mod distort;
 pub mod masks;
 pub mod merge;
+pub mod selection;
```

`DocumentState` gains the summary; `renew_revisions` issues the selection's revision from the engine's counter; Crop, Canvas Size and Image Size drop the selection and Flip Canvas mirrors it:

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -59,6 +59,9 @@ pub struct DocumentState {
     pub guides: Vec<Guide>,
     /// What this project contains that this build does not draw yet (`Document::undrawn`).
     pub undrawn: Vec<String>,
+    /// The selection's summary (Phase 4a); None when nothing is selected. Its outline travels
+    /// separately, when its revision changes (`Engine::selection_outline`).
+    pub selection: Option<SelectionState>,
 }
 
 /// Preview revisions start here so they can never collide with a layer's own, which the engine
@@ -87,6 +90,8 @@ fn renew_revisions(before: &Document, next: &mut Document, counter: &mut u64) {
         if pixels_changed { layer.pixels_revision = issue(); }
         if mask_changed { layer.mask_revision = issue(); }
     }
+    // The selection's outline is fetched by the app when its revision is new (Phase 4a).
+    if next.selection != before.selection { next.selection_revision = issue(); }
 }
 
 #[derive(Default)]
@@ -174,9 +179,40 @@ impl Engine {
             }).collect(),
             guides: d.guides.clone(),
             undrawn: d.undrawn(),
+            selection: d.selection.as_ref().map(|s| SelectionState {
+                revision: d.selection_revision, empty: s.is_empty(), bounds: s.bounds(),
+                antialiased: s.antialiased, feather: s.feather, points: s.point_count(),
+            }),
         })
     }
 
+    /// The selection's outline for the marching ants, in document pixels, flat: the number of
+    /// contours, then for each its number of points and their x, y. At most `OUTLINE_DETAIL_LIMIT`
+    /// points are sent as they are; past that, and when the view shows fewer than one screen pixel
+    /// per document pixel (`step` < 1, a power of two), the outline traced at that resolution
+    /// (`selection_lod`): never more edges than the screen has pixels for. Empty with no selection.
+    pub fn selection_outline(&self, id: Uuid, step: f64) -> Result<Vec<f64>, CommandError> {
+        let doc = &self.session(id)?.document;
+        let Some(selection) = &doc.selection else { return Ok(Vec::new()) };
+        let contours = if selection.point_count() > OUTLINE_DETAIL_LIMIT && step < 1.0 {
+            selection_lod(selection, doc.width, doc.height, step)
+        } else { selection.points() };
+        let mut out = vec![contours.len() as f64];
+        for c in &contours {
+            out.push(c.len() as f64);
+            for p in c { out.push(p.x); out.push(p.y); }
+        }
+        Ok(out)
+    }
+
+    /// Whether a press at `at` lands inside a selection with something in it, by the winding rule:
+    /// where a drag in New mode moves the outline instead of drawing (`canMoveSelection(at:)`,
+    /// Selection.swift:256-260).
+    pub fn selection_contains(&self, id: Uuid, at: Point) -> Result<bool, CommandError> {
+        let doc = &self.session(id)?.document;
+        Ok(doc.selection.as_ref().map_or(false, |s| !s.is_empty() && s.contains(at)))
+    }
+
     /// The document as the canvas should show it: the stored one, or a copy with the open
     /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
     /// the ops do not, because a preview is not committed.
@@ -241,6 +277,9 @@ impl Engine {
             Command::SetActiveLayer { id } => { ops::layers::set_active_layer(doc, id)?; Ok(Dirty::structure()) }
             Command::CanvasSize { width, height, anchor, fill } => {
                 *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions { width, height, anchor, fill, content_offset: None })?;
+                // Canvas Size, Crop and Image Size make a new document on the Mac, which drops the
+                // selection in the same undo step (ImageResizer.swift:109-116).
+                doc.selection = None;
                 Ok(Dirty::everything())
             }
             Command::Crop { x, y, width, height } => {
@@ -255,11 +294,17 @@ impl Engine {
                 *doc = ops::canvas_size::canvas_size(doc, ops::canvas_size::CanvasSizeOptions {
                     width: (x1 - x0) as u32, height: (y1 - y0) as u32, anchor: 4, fill: None,
                     content_offset: Some(Point { x: -x0, y: -y0 }) })?;
+                doc.selection = None;
                 Ok(Dirty::everything())
             }
-            Command::FlipCanvas { horizontal } => { ops::flip::flip_canvas(doc, horizontal); Ok(Dirty::structure()) }
+            Command::FlipCanvas { horizontal } => {
+                ops::flip::flip_canvas(doc, horizontal);
+                ops::selection::flip_selection(doc, horizontal);
+                Ok(Dirty::structure())
+            }
             Command::ImageSize { width, height, resolution, sampling } => {
                 *doc = ops::image_size::image_size(doc, ops::image_size::ImageSizeOptions { width, height, resolution, sampling })?;
+                doc.selection = None;
                 Ok(Dirty::everything())
             }
             Command::SetLayerOpacity { id, opacity } => { ops::appearance::set_opacity(doc, id, opacity)?; Ok(Dirty::structure()) }
@@ -310,6 +355,14 @@ impl Engine {
                 ops::adjust::add_adjustment_layer(doc, kind, seed, gradient)?; Ok(Dirty::structure())
             }
             Command::SetAdjustment { id, adjustment } => { ops::adjust::set_adjustment(doc, id, &adjustment)?; Ok(Dirty::structure()) }
+            Command::SelectShape { kind, points, mode, antialiased } => { ops::selection::select_shape(doc, kind, &points, mode, antialiased)?; Ok(Dirty::structure()) }
+            Command::SelectAll => { ops::selection::select_all(doc); Ok(Dirty::structure()) }
+            Command::Deselect => { ops::selection::deselect(doc); Ok(Dirty::structure()) }
+            Command::InvertSelection => { ops::selection::invert_selection(doc); Ok(Dirty::structure()) }
+            Command::MoveSelection { dx, dy } => { ops::selection::move_selection(doc, dx, dy)?; Ok(Dirty::structure()) }
+            Command::ExpandSelection { amount } => { ops::selection::resize_selection(doc, amount as i64)?; Ok(Dirty::structure()) }
+            Command::ContractSelection { amount } => { ops::selection::resize_selection(doc, -(amount as i64))?; Ok(Dirty::structure()) }
+            Command::FeatherSelection { amount } => { ops::selection::feather_selection(doc, amount)?; Ok(Dirty::structure()) }
         })
     }
 
```

- [ ] **Step 5: Run the tests and watch them pass, then prove they bite**

`cargo test -p compositor-engine --test selection_commands` (19 tests), then the whole engine suite: 422 passed, 1 ignored (+19).

(1) In `Engine::execute`'s `Crop` arm, drop `doc.selection = None;`: `crop_canvas_size_and_image_size_drop_the_selection_and_flip_canvas_mirrors_it` fails. Restore. (2) In `renew_revisions`, drop the selection line: `the_selection_is_part_of_the_document_but_never_saved` fails (the revision does not move). Restore. (3) In `feather_selection`, add the amounts (`current.feather + amount`) instead of `sqrt(a^2 + b^2)`: `feathers_combine_as_blurs_do_and_stop_at_250` fails. Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/selection/trace.rs engine/src/selection/outline.rs engine/src/ops/selection.rs engine/tests/selection_commands.rs
git commit -- engine/src/selection engine/src/lib.rs engine/src/error.rs engine/src/command.rs engine/src/ops/mod.rs engine/src/ops/selection.rs engine/src/engine.rs engine/tests/selection_commands.rs -m "feat: selection commands with the Mac's undo names, the tracer, and the outline for the ants" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: The Magic Wand, and a layer's pixels or a mask's black areas as a selection

The Magic Wand selects the pixels whose every channel, alpha included, lies within the tolerance of the colour sampled at the click (a point, or the mean of 3 x 3 or 5 x 5), everywhere or only those 4-connected to it, from the active layer's own pixels or from everything visible (MagicWand.swift:36-137, WandPixels.c:9-85; ruling OQ6). Ctrl-clicking a layer's thumbnail loads its pixels at least half opaque; Ctrl-clicking a mask's loads its black areas, as the Mac does (MaskTracing.swift:73-94). All three trace pixels into an outline with Task 4's tracer and combine it by the mode.

**Files:**
- Create: `engine/src/selection/wand.rs`
- Modify: `engine/src/selection/mod.rs`, `engine/src/lib.rs`, `engine/src/command.rs`, `engine/src/ops/selection.rs`, `engine/src/engine.rs`
- Create test: `engine/tests/selection_wand.rs`

**Interfaces:**
- Consumes: Task 4's tracer and commands.
- Produces: `WandSettings { tolerance: u32, sample_radius: u32, contiguous: bool, all_layers: bool }` (camelCase JSON; default 32, 0, true, false); `wand_mask(rgba, width, height, seed_x, seed_y, radius, tolerance, contiguous) -> (Vec<u8>, usize)`; `magic_wand(&Raster, Point, &WandSettings) -> Result<Option<Vec<Contour>>, TraceError>`; `Command::{MagicWand { at, mode, settings, antialiased }, LoadLayerSelection { id, mode, antialiased }, LoadMaskSelection { id, mode, antialiased }}`; `ops::selection::{TOO_DETAILED, wand_sample, magic_wand_select, load_layer_selection, load_mask_selection}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/selection_wand.rs` ports MagicWandTests and the loading cases of SelectionTests and LayerMaskTests:

```rust
//! The Magic Wand (WandPixels.c, MagicWand.swift) and loading a layer's or a mask's pixels as a
//! selection (MaskTracing.swift), ported from Compositor for Mac's MagicWandTests.swift and the
//! load cases of SelectionTests.swift and LayerMaskTests.swift, with their images and values.
use compositor_engine::*;
use std::collections::BTreeSet;
use uuid::Uuid;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
/// Premultiplied RGBA, top row first (MagicWandTests `image`).
fn image(width: u32, height: u32, color: impl Fn(u32, u32) -> [u8; 4]) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width { data.extend_from_slice(&color(x, y)); } }
    Raster::from_premultiplied(width, height, data)
}
/// The pixel indices an outline covers: filled without antialiasing, at least half (MagicWandTests `pixels`).
fn pixels(contours: &[Contour], width: u32, height: u32) -> BTreeSet<u32> {
    let c = rasterize(contours, 0.0, 0.0, width, height, false);
    (0..width * height).filter(|i| c.bytes()[*i as usize] >= 128).collect()
}
fn block(columns: std::ops::Range<u32>, rows: std::ops::Range<u32>, width: u32) -> BTreeSet<u32> {
    rows.flat_map(|y| columns.clone().map(move |x| y * width + x)).collect()
}
fn settings(tolerance: u32, sample_radius: u32, contiguous: bool) -> WandSettings { WandSettings { tolerance, sample_radius, contiguous, all_layers: false } }
fn wand(image: &Raster, at: Point, s: WandSettings) -> BTreeSet<u32> {
    magic_wand(image, at, &s).unwrap().map_or_else(BTreeSet::new, |c| pixels(&c, image.width, image.height))
}

#[test]
fn contiguous_stops_at_other_colors_while_non_contiguous_finds_every_match() {
    let stripes = image(10, 4, |x, _| if x < 3 || x >= 6 { RED } else { BLUE });
    let (left, right) = (block(0..3, 0..4, 10), block(6..10, 0..4, 10));
    assert_eq!(wand(&stripes, p(1.5, 2.5), settings(32, 0, true)), left);
    assert_eq!(wand(&stripes, p(1.5, 2.5), settings(32, 0, false)), left.union(&right).copied().collect());
    // Rows stay the right way up: clicking the top row selects the top row.
    let banded = image(4, 3, |_, y| if y == 0 { RED } else { BLUE });
    assert_eq!(wand(&banded, p(1.0, 0.0), settings(32, 0, true)), [0, 1, 2, 3].into());
    assert_eq!(magic_wand(&banded, p(9.0, 0.0), &settings(32, 0, true)).unwrap(), None, "outside the image");
}

#[test]
fn tolerance_applies_to_every_channel_including_alpha() {
    let columns = [[100, 100, 100, 255], [132, 100, 100, 255], [133, 100, 100, 255], [100, 100, 100, 222]];
    let row = image(4, 1, |x, _| columns[x as usize]);
    assert_eq!(wand(&row, p(0.5, 0.5), settings(0, 0, false)), [0].into());
    assert_eq!(wand(&row, p(0.5, 0.5), settings(32, 0, false)), [0, 1].into());
    assert_eq!(wand(&row, p(0.5, 0.5), settings(33, 0, false)), [0, 1, 2, 3].into());
}

#[test]
fn sample_size_averages_the_pixels_around_the_click() {
    let dot = image(5, 5, |x, y| if x == 2 && y == 2 { [255, 255, 255, 255] } else { [0, 0, 0, 255] });
    assert_eq!(wand(&dot, p(2.5, 2.5), settings(10, 0, false)), [12].into());
    // A 3 x 3 average is grey (255 + 4) / 9 = 28: black is within 30 of it, the white centre is not.
    assert_eq!(wand(&dot, p(2.5, 2.5), settings(30, 1, false)), (0..25).filter(|i| *i != 12).collect());
}

#[test]
fn outlines_reproduce_their_pixels_with_holes_and_corner_touches() {
    let (width, height) = (8u32, 6u32);
    let mut mask = vec![0u8; 48];
    // A 3 x 3 ring around a hole, against the image's corner, and two pixels touching only diagonally.
    for y in 0..3 { for x in 0..3 { if !(x == 1 && y == 1) { mask[y * 8 + x] = 255; } } }
    mask[4 * 8 + 5] = 255;
    mask[5 * 8 + 6] = 255;
    let expected: BTreeSet<u32> = (0..48).filter(|i| mask[*i as usize] != 0).collect();
    let loops = trace_pixels(&mask, width as usize, height as usize).unwrap();
    let contours: Vec<Contour> = loops.iter().map(|l| l.iter().map(|q| [q[0] * SUBPIXEL as i32, q[1] * SUBPIXEL as i32]).collect()).collect();
    assert_eq!(pixels(&contours, width, height), expected);
    // Corners only: the ring's outer loop and its hole have four each, and the diagonal pair stays
    // two separate squares (the walk turns right where they touch).
    let mut lengths: Vec<usize> = loops.iter().map(Vec::len).collect();
    lengths.sort();
    assert_eq!(lengths, vec![4, 4, 4, 4]);
    assert!(trace_pixels(&[0u8; 4], 2, 2).unwrap().is_empty());
}

#[test]
fn an_outline_past_eight_million_edges_is_too_detailed() {
    // A 2001 x 2000 checkerboard, non-contiguous: 2,001,000 matching pixels, four edges each,
    // 8,004,000 edges, just past WAND_EDGE_LIMIT. 2000 x 2000 gives exactly 8,000,000, which passes.
    let checker = |w: u32, h: u32| image(w, h, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE });
    assert_eq!(magic_wand(&checker(2001, 2000), p(0.5, 0.5), &settings(0, 0, false)), Err(TraceError::TooDetailed));
    assert!(magic_wand(&checker(2000, 2000), p(0.5, 0.5), &settings(0, 0, false)).unwrap().is_some());
}

/// A document opened from `doc` in a fresh engine, as a project would be.
fn opened(doc: &Document) -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.open_package(&save_package(doc).unwrap(), None).unwrap();
    (e, id)
}
fn selected(e: &Engine, id: Uuid) -> BTreeSet<u32> {
    let d = e.document(id).unwrap();
    d.selection.as_ref().map_or_else(BTreeSet::new, |s| pixels(&s.contours, d.width, d.height))
}
fn coverage(e: &Engine, id: Uuid, x: u32, y: u32) -> u8 {
    let d = e.document(id).unwrap();
    let Some(s) = &d.selection else { return 0 };
    SelectionClip::new(s, d.width, d.height).on_grid(&Affine::IDENTITY, d.width, d.height).bytes()[(y * d.width + x) as usize]
}
fn wand_at(e: &mut Engine, id: Uuid, at: Point, mode: SelectionMode, all_layers: bool) {
    e.execute(id, Command::MagicWand { at, mode, settings: WandSettings { all_layers, ..WandSettings::default() }, antialiased: true }).unwrap();
}

#[test]
fn the_wand_reads_the_active_layer_or_every_visible_layer_and_combines_modes() {
    // MagicWandTests.theWandReadsTheActiveLayerOrEveryVisibleLayerAndCombinesModes: a 20 x 10 red
    // and blue image under a blank active layer.
    let mut doc = Document::new(20, 10);
    let halves = Layer::with_pixels("Halves", image(20, 10, |x, _| if x < 10 { RED } else { BLUE }), p(0.0, 0.0));
    let blank = Layer::blank("Layer 1", doc.size());
    doc.active_layer_id = Some(blank.id);
    doc.layers = vec![halves, blank];
    let (mut e, id) = opened(&doc);
    let left = block(0..10, 0..10, 20);
    // The blank active layer is transparent everywhere, so the whole canvas matches.
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Replace, false);
    assert_eq!(selected(&e, id).len(), 200);
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id), left);
    let count = e.state(id).unwrap().undo_depth;
    wand_at(&mut e, id, p(15.0, 5.0), SelectionMode::Add, true);
    assert_eq!(selected(&e, id).len(), 200);
    assert_eq!(e.state(id).unwrap().undo_depth, count + 1);
    wand_at(&mut e, id, p(2.0, 2.0), SelectionMode::Subtract, true);
    assert_eq!(selected(&e, id), (0..200).filter(|i| !left.contains(i)).collect());
    e.undo(id).unwrap();
    assert_eq!(selected(&e, id).len(), 200);
    // Clicking inside a selection makes a new wand selection rather than deselecting
    // (clickingInsideASelectionMakesANewWandSelectionRatherThanDeselecting).
    e.execute(id, Command::SelectAll).unwrap();
    wand_at(&mut e, id, p(3.5, 5.5), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id), left);
}

#[test]
fn the_wand_reads_the_active_layer_through_its_transform_but_not_its_mask_or_opacity() {
    // A 10 x 5 red layer stretched to 20 x 10 at (4, 2), at 10% opacity, under a mask hiding it all.
    // Through its opacity it would be (26, 0, 0, 26), within the default 32 of the clear canvas
    // around it; through its mask, clear; unscaled, (4, 2)-(14, 7).
    let mut doc = Document::new(40, 20);
    let mut red = Layer::with_pixels("Red", image(10, 5, |_, _| RED), p(4.0, 2.0));
    red.transform.size = Size { width: 20.0, height: 10.0 };
    red.opacity = 0.1;
    red.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![0]), enabled: true, placement: None, linked: None });
    doc.active_layer_id = Some(red.id);
    doc.layers = vec![red];
    let (mut e, id) = opened(&doc);
    wand_at(&mut e, id, p(10.0, 6.0), SelectionMode::Replace, false);
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(4.0, 2.0, 20.0, 10.0)));
    // With All Layers the mask hides it, and the transparent canvas matches everywhere.
    wand_at(&mut e, id, p(10.0, 6.0), SelectionMode::Replace, true);
    assert_eq!(selected(&e, id).len(), 800);
}

#[test]
fn a_point_off_the_canvas_is_refused_and_nothing_matching_deselects() {
    let mut doc = Document::new(8, 8);
    let mut layer = Layer::with_pixels("Dot", image(3, 3, |x, y| if x == 1 && y == 1 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }), p(0.0, 0.0));
    layer.name = "Dot".into();
    doc.active_layer_id = Some(layer.id);
    doc.layers = vec![layer];
    let (mut e, id) = opened(&doc);
    assert!(e.execute(id, Command::MagicWand { at: p(8.0, 1.0), mode: SelectionMode::Replace, settings: WandSettings::default(), antialiased: true }).is_err());
    e.execute(id, Command::SelectAll).unwrap();
    // A 3 x 3 average at the white dot is grey 28: tolerance 10 matches nothing there, the dot included.
    e.execute(id, Command::MagicWand { at: p(1.5, 1.5), mode: SelectionMode::Replace, settings: WandSettings { tolerance: 10, sample_radius: 1, contiguous: true, all_layers: false }, antialiased: true }).unwrap();
    assert!(e.document(id).unwrap().selection.is_none(), "nothing matched: Replace deselects");
}

/// A 100 x 100 mask: white, a black square (20, 30)-(60, 70), and a white hole (30, 40)-(40, 50)
/// inside it (SelectionTests `maskedLayer`).
fn masked_pixels() -> GrayRaster {
    let mut m = vec![255u8; 10_000];
    for y in 30..70 { for x in 20..60 { m[y * 100 + x] = 0; } }
    for y in 40..50 { for x in 30..40 { m[y * 100 + x] = 255; } }
    GrayRaster::from_bytes(100, 100, m)
}

#[test]
fn ctrl_clicking_a_mask_selects_its_black_areas() {
    // SelectionTests.cmdClickingAMaskSelectsItsBlackAreas.
    let mut doc = Document::new(100, 100);
    let mut layer = Layer::blank("Layer 1", doc.size());
    layer.mask = Some(Mask { pixels: masked_pixels(), enabled: true, placement: None, linked: None });
    let layer_id = layer.id;
    doc.active_layer_id = Some(layer_id);
    doc.layers = vec![layer];
    let (mut e, id) = opened(&doc);
    let load = |e: &mut Engine, mode| e.execute(id, Command::LoadMaskSelection { id: layer_id, mode, antialiased: true }).unwrap();
    load(&mut e, SelectionMode::Replace);
    assert_eq!(coverage(&e, id, 25, 35), 255, "black: selected");
    assert_eq!(coverage(&e, id, 35, 45), 0, "the white hole: not selected");
    assert_eq!(coverage(&e, id, 80, 80), 0, "white surroundings");
    assert_eq!((coverage(&e, id, 59, 69), coverage(&e, id, 60, 70)), (255, 0), "exact pixel edges");
    let square = vec![p(80.0, 80.0), p(90.0, 80.0), p(90.0, 90.0), p(80.0, 90.0)];
    e.execute(id, Command::SelectShape { kind: SelectionShape::Freehand, points: square, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    load(&mut e, SelectionMode::Add);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 25, 35)), (255, 255));
    load(&mut e, SelectionMode::Subtract);
    assert_eq!((coverage(&e, id, 85, 85), coverage(&e, id, 25, 35)), (255, 0));
    assert_eq!(Command::LoadMaskSelection { id: layer_id, mode: SelectionMode::Add, antialiased: true }.action_name(), "Load Mask Selection");
}

#[test]
fn a_mask_selection_follows_the_layer_transform_and_an_all_white_mask_selects_nothing() {
    // SelectionTests.maskSelectionFollowsTheLayerTransformAndIgnoresAllWhiteMasks: the layer and
    // its mask stretched 2x from the canvas origin.
    let mut doc = Document::new(200, 200);
    let mut layer = Layer::blank("Layer 1", Size { width: 100.0, height: 100.0 });
    layer.transform.size = Size { width: 200.0, height: 200.0 };
    layer.mask = Some(Mask { pixels: masked_pixels(), enabled: true, placement: None, linked: None });
    let mut white = Layer::blank("Layer 2", doc.size());
    white.mask = Some(Mask { pixels: GrayRaster::from_bytes(1, 1, vec![255]), enabled: true, placement: None, linked: None });
    let (masked_id, white_id) = (layer.id, white.id);
    doc.layers = vec![layer, white];
    let (mut e, id) = opened(&doc);
    e.execute(id, Command::LoadMaskSelection { id: masked_id, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(40.0, 60.0, 80.0, 80.0)));
    e.execute(id, Command::Deselect).unwrap();
    assert!(matches!(e.execute(id, Command::LoadMaskSelection { id: white_id, mode: SelectionMode::Replace, antialiased: true }), Err(CommandError::Refused(_))));
    assert!(e.document(id).unwrap().selection.is_none(), "no black anywhere: nothing to select");
}

#[test]
fn ctrl_clicking_a_layer_selects_its_opaque_pixels() {
    // SelectionTests.cmdClickingALayerSelectsItsOpaquePixels: a 50 x 50 image, an opaque ring
    // (10,10)-(40,40) around a clear (20,20)-(30,30), a 25% corner pixel, shown at 2x from (50, 50).
    let mut doc = Document::new(200, 200);
    let ring = image(50, 50, |x, y| {
        if x == 0 && y == 0 { [64, 0, 0, 64] }
        else if (10..40).contains(&x) && (10..40).contains(&y) && !((20..30).contains(&x) && (20..30).contains(&y)) { RED }
        else { [0, 0, 0, 0] }
    });
    let mut layer = Layer::with_pixels("Ring", ring, p(50.0, 50.0));
    layer.transform.size = Size { width: 100.0, height: 100.0 };
    let blank = Layer::blank("Layer 2", doc.size());
    let (ring_id, blank_id) = (layer.id, blank.id);
    doc.layers = vec![layer, blank];
    let (mut e, id) = opened(&doc);
    let load = |e: &mut Engine, layer, mode| e.execute(id, Command::LoadLayerSelection { id: layer, mode, antialiased: true });
    load(&mut e, ring_id, SelectionMode::Replace).unwrap();
    assert_eq!(e.document(id).unwrap().selection.as_ref().unwrap().bounds(), Some(rect(70.0, 70.0, 60.0, 60.0)), "2x scale");
    assert_eq!(coverage(&e, id, 75, 75), 255, "the opaque ring");
    assert_eq!(coverage(&e, id, 100, 100), 0, "the clear centre");
    assert_eq!(coverage(&e, id, 50, 50), 0, "the 25% pixel is under the threshold");
    let corner = vec![p(0.0, 0.0), p(20.0, 0.0), p(20.0, 20.0), p(0.0, 20.0)];
    e.execute(id, Command::SelectShape { kind: SelectionShape::Freehand, points: corner, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    load(&mut e, ring_id, SelectionMode::Add).unwrap();
    assert_eq!((coverage(&e, id, 10, 10), coverage(&e, id, 75, 75)), (255, 255));
    let before = e.document(id).unwrap().selection.clone();
    assert!(matches!(load(&mut e, blank_id, SelectionMode::Replace), Err(CommandError::Refused(_))));
    assert_eq!(e.document(id).unwrap().selection, before, "an empty layer selects nothing");
}

#[test]
fn a_folder_mask_loads_its_black_areas_after_an_invert() {
    // LayerMaskTests.folderMaskCanBePaintedInvertedAndLoadedAsASelection, its load: a folder over a
    // 40 x 20 red layer, its mask white but for a black dot, inverted: black everywhere but the dot.
    let mut doc = Document::new(40, 20);
    let mut folder = Layer::blank("Folder", doc.size());
    folder.is_group = true;
    let dot: Vec<u8> = (0..20).flat_map(|y: i32| (0..40).map(move |x: i32| if (x - 10).pow(2) + (y - 10).pow(2) <= 16 { 0 } else { 255 })).collect();
    folder.mask = Some(Mask { pixels: GrayRaster::from_bytes(40, 20, dot), enabled: true, placement: None, linked: None });
    let mut red = Layer::with_pixels("Red", image(40, 20, |_, _| RED), p(0.0, 0.0));
    red.parent_id = Some(folder.id);
    let folder_id = folder.id;
    doc.layers = vec![folder, red];
    let (mut e, id) = opened(&doc);
    e.execute(id, Command::InvertMask { id: folder_id }).unwrap();
    e.execute(id, Command::LoadMaskSelection { id: folder_id, mode: SelectionMode::Replace, antialiased: true }).unwrap();
    let width = e.document(id).unwrap().selection.as_ref().unwrap().bounds().unwrap().width;
    assert!(width > 30.0, "{width}");
    assert_eq!((coverage(&e, id, 10, 10), coverage(&e, id, 30, 10)), (0, 255), "the dot stays out");
}

#[test]
fn the_wand_refuses_an_outline_too_detailed_to_draw() {
    // Through the engine, the Mac's message (MagicWand.swift:28).
    let mut doc = Document::new(2001, 2000);
    let checker = Layer::with_pixels("Checker", image(2001, 2000, |x, y| if (x + y) % 2 == 0 { RED } else { BLUE }), p(0.0, 0.0));
    doc.active_layer_id = Some(checker.id);
    doc.layers = vec![checker];
    let (mut e, id) = opened(&doc);
    let err = e.execute(id, Command::MagicWand { at: p(0.5, 0.5), mode: SelectionMode::Replace, settings: WandSettings { tolerance: 0, sample_radius: 0, contiguous: false, all_layers: false }, antialiased: true }).unwrap_err();
    assert_eq!(err.to_string(), "That selection is too detailed to outline. Try a different Tolerance, or turn on Contiguous.");
    assert!(e.document(id).unwrap().selection.is_none());
}
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test selection_wand`: it does not compile (`magic_wand`, `WandSettings` and the three commands do not exist).

- [ ] **Step 3: The matcher and the commands**

```rust
//! The Magic Wand's matcher: `wand_mask` (Compositor for Mac's WandPixels.c:9-85), line for line,
//! over premultiplied RGBA, and `MagicWand.select` (Document/MagicWand.swift:36-52) around it.
use super::trace::{to_contours, trace_pixels, TraceError};
use super::Contour;
use crate::{Point, Raster};
use serde::{Deserialize, Serialize};

/// The Magic Wand's options (`WandSettings`, MagicWand.swift:11-19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WandSettings {
    /// How far (0 to 255) every channel, alpha included, may differ from the sampled colour.
    pub tolerance: u32,
    /// Pixels either side of the click averaged into the colour to match: 0 (Point), 1 (3 by 3), 2 (5 by 5).
    pub sample_radius: u32,
    /// Only matching pixels connected to the clicked one.
    pub contiguous: bool,
    /// Read the visible composite rather than the active layer's own pixels.
    pub all_layers: bool,
}

impl Default for WandSettings {
    fn default() -> WandSettings { WandSettings { tolerance: 32, sample_radius: 0, contiguous: true, all_layers: false } }
}

fn matches(p: &[u8], reference: &[i32; 4], tolerance: i32) -> bool {
    (0..4).all(|c| { let d = p[c] as i32 - reference[c]; d >= -tolerance && d <= tolerance })
}

/// 255 where a pixel matches the colour averaged over the (2 radius + 1)^2 pixels around the seed
/// (rounded, each channel), within `tolerance` on every channel; with `contiguous`, only pixels
/// 4-connected to the seed, by a scanline flood fill. Returns the mask and its count of matches.
/// `wand_mask`, WandPixels.c:17-85.
pub fn wand_mask(rgba: &[u8], width: usize, height: usize, seed_x: usize, seed_y: usize, radius: usize, tolerance: i32, contiguous: bool) -> (Vec<u8>, usize) {
    let mut mask = vec![0u8; width * height];
    if width == 0 || height == 0 || seed_x >= width || seed_y >= height { return (mask, 0); }
    let stride = width * 4;
    let (x0, x1) = (seed_x.saturating_sub(radius), (seed_x + radius).min(width - 1));
    let (y0, y1) = (seed_y.saturating_sub(radius), (seed_y + radius).min(height - 1));
    let (mut sums, mut samples) = ([0u64; 4], 0u64);
    for y in y0..=y1 { for x in x0..=x1 {
        for c in 0..4 { sums[c] += rgba[y * stride + x * 4 + c] as u64; }
        samples += 1;
    }}
    let reference = [0, 1, 2, 3].map(|c| ((sums[c] + samples / 2) / samples) as i32);
    let at = |x: usize, y: usize| &rgba[y * stride + x * 4..y * stride + x * 4 + 4];
    let mut count = 0usize;
    if !contiguous {
        for y in 0..height { for x in 0..width {
            if matches(at(x, y), &reference, tolerance) { mask[y * width + x] = 255; count += 1; }
        }}
        return (mask, count);
    }
    // Scanline flood fill: each popped seed fills its whole horizontal run, then pushes one seed
    // per matching run in the rows directly above and below it.
    let mut stack = vec![(seed_x, seed_y)];
    while let Some((x, y)) = stack.pop() {
        if mask[y * width + x] != 0 || !matches(at(x, y), &reference, tolerance) { continue; }
        let (mut left, mut right) = (x, x);
        while left > 0 && mask[y * width + left - 1] == 0 && matches(at(left - 1, y), &reference, tolerance) { left -= 1; }
        while right + 1 < width && mask[y * width + right + 1] == 0 && matches(at(right + 1, y), &reference, tolerance) { right += 1; }
        mask[y * width + left..=y * width + right].fill(255);
        count += right - left + 1;
        for side in 0..2 {
            if if side == 0 { y == 0 } else { y + 1 >= height } { continue; }
            let ny = if side == 0 { y - 1 } else { y + 1 };
            let mut in_run = false;
            for nx in left..=right {
                let candidate = mask[ny * width + nx] == 0 && matches(at(nx, ny), &reference, tolerance);
                if candidate && !in_run { stack.push((nx, ny)); }
                in_run = candidate;
            }
        }
    }
    (mask, count)
}

/// The outline, in `SUBPIXEL` units of the image's pixels, of the pixels matching the one at
/// `point`; None when the point lies outside the image or nothing matches (`MagicWand.select`).
pub fn magic_wand(image: &Raster, point: Point, settings: &WandSettings) -> Result<Option<Vec<Contour>>, TraceError> {
    if !point.x.is_finite() || !point.y.is_finite() || point.x < 0.0 || point.y < 0.0 { return Ok(None); }
    let (x, y) = (point.x.floor() as usize, point.y.floor() as usize);
    let (w, h) = (image.width as usize, image.height as usize);
    if x >= w || y >= h { return Ok(None); }
    let (mask, count) = wand_mask(image.bytes(), w, h, x, y, settings.sample_radius as usize, settings.tolerance.min(255) as i32, settings.contiguous);
    if count == 0 { return Ok(None); }
    Ok(Some(to_contours(trace_pixels(&mask, w, h)?)))
}
```

```diff
--- a/engine/src/selection/mod.rs
+++ b/engine/src/selection/mod.rs
@@ -10,6 +10,7 @@ pub mod coverage;
 pub mod geometry;
 pub mod outline;
 pub mod trace;
+pub mod wand;
 
 use crate::{Point, Rect};
 use serde::{Deserialize, Serialize};
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -50,3 +50,4 @@ pub use selection::{Contour, Selection, SelectionMode, SelectionShape, Selection
 pub use selection::coverage::{rasterize, selection_coverage, SelectionClip};
 pub use selection::outline::{selection_lod, OUTLINE_DETAIL_LIMIT, OUTLINE_MASK_LIMIT};
 pub use selection::trace::{dark_pixels, opaque_pixels, trace_pixels, TraceError, WAND_EDGE_LIMIT};
+pub use selection::wand::{magic_wand, wand_mask, WandSettings};
```

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -1,4 +1,4 @@
-use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape};
+use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape, WandSettings};
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
@@ -60,6 +60,9 @@ pub enum Command {
     ExpandSelection { amount: u32 },
     ContractSelection { amount: u32 },
     FeatherSelection { amount: u32 },
+    MagicWand { at: Point, mode: SelectionMode, settings: WandSettings, antialiased: bool },
+    LoadLayerSelection { #[serde(with = "ids::upper")] id: Uuid, mode: SelectionMode, antialiased: bool },
+    LoadMaskSelection { #[serde(with = "ids::upper")] id: Uuid, mode: SelectionMode, antialiased: bool },
 }
 
 impl Command {
@@ -120,6 +123,9 @@ impl Command {
             Command::ExpandSelection { .. } => "Expand Selection",
             Command::ContractSelection { .. } => "Contract Selection",
             Command::FeatherSelection { .. } => "Feather Selection",
+            Command::MagicWand { .. } => "Magic Wand",
+            Command::LoadLayerSelection { .. } => "Load Layer Selection",
+            Command::LoadMaskSelection { .. } => "Load Mask Selection",
         }
     }
 }
```

```diff
--- a/engine/src/ops/selection.rs
+++ b/engine/src/ops/selection.rs
@@ -10,6 +10,9 @@ use uuid::Uuid;
 pub const EMPTY_SELECTION: &str = "The selection is empty";
 /// Said when an edit needs a selection and there is none.
 pub const NO_SELECTION: &str = "Nothing is selected";
+/// The Magic Wand's refusal of an outline past `WAND_EDGE_LIMIT` (MagicWand.swift:28).
+pub const TOO_DETAILED: &str = "That selection is too detailed to outline. Try a different Tolerance, or turn on Contiguous.";
+
 fn refused(message: &str) -> CommandError { CommandError::Refused(message.to_string()) }
 fn canvas(doc: &Document) -> Vec<Contour> { vec![g::rectangle(Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 })] }
 fn check_point(p: Point) -> Result<(), CommandError> {
@@ -111,3 +114,62 @@ pub fn flip_selection(doc: &mut Document, horizontal: bool) {
         for c in &mut s.contours { for p in c.iter_mut() { if horizontal { p[0] = w - p[0]; } else { p[1] = h - p[1]; } } }
     }
 }
+
+/// What the Magic Wand reads, at canvas size (`selectionSample`): the visible composite, or the
+/// active layer's own pixels through its transform, without its mask or opacity. A folder, an
+/// adjustment layer or a blank layer reads as transparent.
+pub fn wand_sample(doc: &Document, all_layers: bool) -> Raster {
+    let canvas = Rect { x: 0.0, y: 0.0, width: doc.width as f64, height: doc.height as f64 };
+    if all_layers { return composite(doc, canvas, doc.width, doc.height); }
+    let mut target = vec![0u8; (doc.width as usize) * (doc.height as usize) * 4];
+    if let Some(layer) = doc.active_layer_id.and_then(|id| doc.layer(id)).filter(|l| !l.is_group && l.pixels.is_some()) {
+        let mut alone = layer.clone();
+        alone.opacity = 1.0;
+        alone.mask = None;
+        compositor::render_layer(&mut target, doc.width, doc.height, canvas, &alone);
+    }
+    Raster::from_premultiplied(doc.width, doc.height, target)
+}
+
+/// The Magic Wand at a document point: its outline, combined by `mode`. In Replace the traced
+/// outline is taken as it is (it already lies on the canvas); nothing matched deselects in
+/// Replace. `magicWand(at:mode:)`, MagicWand.swift:98-123.
+pub fn magic_wand_select(doc: &mut Document, at: Point, mode: SelectionMode, settings: &WandSettings, antialiased: bool) -> Result<(), CommandError> {
+    if !(at.x >= 0.0 && at.y >= 0.0 && at.x < doc.width as f64 && at.y < doc.height as f64) {
+        return Err(CommandError::Argument("the Magic Wand's point must lie on the canvas".into()));
+    }
+    if settings.tolerance > 255 || settings.sample_radius > 2 { return Err(CommandError::Argument("tolerance is 0 to 255, sample radius 0 to 2".into())); }
+    let sample = wand_sample(doc, settings.all_layers);
+    match magic_wand(&sample, at, settings) {
+        Err(TraceError::TooDetailed) => Err(refused(TOO_DETAILED)),
+        Ok(None) => { if mode == SelectionMode::Replace { doc.selection = None; } Ok(()) }
+        Ok(Some(outline)) if mode == SelectionMode::Replace => { doc.selection = Some(Selection::new(outline, antialiased, 0.0)); Ok(()) }
+        Ok(Some(outline)) => { apply_selection(doc, &outline, mode, antialiased); Ok(()) }
+    }
+}
+
+/// Ctrl-click on a layer's thumbnail (`loadLayerSelection`): its pixels at least 50% opaque,
+/// whatever its mask, through its transform, combined by `mode`.
+pub fn load_layer_selection(doc: &mut Document, id: Uuid, mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
+    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    let raster = layer.pixels.as_ref().filter(|_| !layer.is_group).ok_or_else(|| refused("The layer has no pixels to select"))?;
+    let traced = opaque_pixels(raster).map_err(|_| refused(TOO_DETAILED))?;
+    if traced.is_empty() { return Err(refused("The layer has no pixels at least half opaque")); }
+    let outline = g::transformed(&traced, &layer.transform.pixel_to_document(raster.width, raster.height));
+    apply_selection(doc, &outline, mode, antialiased);
+    Ok(())
+}
+
+/// Ctrl-click on a mask's thumbnail (`loadMaskSelection`): the mask's BLACK areas, darker than 50%
+/// grey, which is what it hides (Photoshop loads the white; the Mac, and so this port, the
+/// black), through where the mask sits, combined by `mode`.
+pub fn load_mask_selection(doc: &mut Document, id: Uuid, mode: SelectionMode, antialiased: bool) -> Result<(), CommandError> {
+    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    let mask = layer.mask.as_ref().ok_or_else(|| refused("The layer has no mask"))?;
+    let traced = dark_pixels(&mask.pixels).map_err(|_| refused(TOO_DETAILED))?;
+    if traced.is_empty() { return Err(refused("The mask has no black areas to select")); }
+    let placement = mask.placement.unwrap_or(layer.transform);
+    let outline = g::transformed(&traced, &placement.pixel_to_document(mask.pixels.width, mask.pixels.height));
+    apply_selection(doc, &outline, mode, antialiased);
+    Ok(())
+}
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -363,6 +363,9 @@ impl Engine {
             Command::ExpandSelection { amount } => { ops::selection::resize_selection(doc, amount as i64)?; Ok(Dirty::structure()) }
             Command::ContractSelection { amount } => { ops::selection::resize_selection(doc, -(amount as i64))?; Ok(Dirty::structure()) }
             Command::FeatherSelection { amount } => { ops::selection::feather_selection(doc, amount)?; Ok(Dirty::structure()) }
+            Command::MagicWand { at, mode, settings, antialiased } => { ops::selection::magic_wand_select(doc, at, mode, &settings, antialiased)?; Ok(Dirty::structure()) }
+            Command::LoadLayerSelection { id, mode, antialiased } => { ops::selection::load_layer_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
+            Command::LoadMaskSelection { id, mode, antialiased } => { ops::selection::load_mask_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
         })
     }
 
```

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test selection_wand` (13 tests; the 2001 x 2000 checkerboard takes most of their time in a debug build), then the whole engine suite: 435 passed, 1 ignored (+13).

- [ ] **Step 5: Prove it bites**

(1) In `wand_sample`, keep the layer's opacity (drop `alone.opacity = 1.0;`): `the_wand_reads_the_active_layer_through_its_transform_but_not_its_mask_or_opacity` fails (opacity 0.1 moves every pixel outside the tolerance). Restore. (2) In `load_mask_selection`, trace the light areas instead of the dark ones (`dark_pixels(&adjust::tonal::invert_gray(&mask.pixels))`): `ctrl_clicking_a_mask_selects_its_black_areas`, `a_mask_selection_follows_the_layer_transform_and_an_all_white_mask_selects_nothing` and `a_folder_mask_loads_its_black_areas_after_an_invert` fail. Restore. (3) In `matches`, compare the colour channels only (`(0..3)`): `tolerance_applies_to_every_channel_including_alpha` fails (the pixel 33 apart in alpha comes in at tolerance 32). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/selection/wand.rs engine/tests/selection_wand.rs
git commit -- engine/src/selection engine/src/lib.rs engine/src/command.rs engine/src/ops/selection.rs engine/src/engine.rs engine/tests/selection_wand.rs -m "feat: the Magic Wand, and layer pixels or mask black areas as a selection" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Adjustments, filters and Invert stay inside the selection; the histogram is weighted by it

Phase 3 built the coverage hook (`apply_adjustment(.., selection)`, `blend_by_coverage`, `histogram(.., coverage)`) and passed `None` everywhere (research). The Mac blends every destructive result back through the selection's clip on the edited image's own grid (`PixelAdjust.blend`, PixelAdjust.swift:36-46): the colour adjustments (Levels.swift:89-91), the filters on the grid a blur has grown, before trimming (Filters.swift:169-170, :399-413), and Invert, on the pixels or the mask, a 1 x 1 mask first taking the layer's pixel grid (SelectionEdits.swift:85-122). Levels and Curves weight their histogram by the same coverage (`levels_histogram`, LevelsPixels.c:17-28). An empty selection refuses them all (`canAdjustColors`, `canInvert`). This task wires that in, previews included, and fixes Phase 3's N3 on the way: a kept preview is now keyed on the stored pixels, their placement and the selection it was made from (ruling OQ10). Adjustment layers are untouched: they never read the selection.

**Files:**
- Modify: `engine/src/ops/adjust.rs` (`edit_coverage`, `apply_adjustment_to_layer`, `invert_layer`, `apply_filter`), `engine/src/adjust/apply.rs` (`blend_gray_by_coverage`), `engine/src/ops/masks.rs` (`invert_mask`), `engine/src/preview.rs` (`PreviewSource`, `compute_preview`), `engine/src/engine.rs` (`histogram`, `set_preview`)
- Create test: `engine/tests/selection_edits.rs`

**Interfaces:**
- Consumes: Task 3's `selection_coverage`, Task 4's `EMPTY_SELECTION`.
- Produces: `pub fn edit_coverage(doc, &LayerTransform, width, height) -> Result<Option<GrayRaster>, CommandError>` (None with no selection; `Refused(EMPTY_SELECTION)` for an empty one); `pub fn blend_gray_by_coverage(adjusted, original, coverage) -> GrayRaster`; `PreviewSource { pixels_revision, transform, selection_revision }` with `PreviewSource::of(&Document, layer)`; `PixelPreview.source`; `PixelPreview::answers(&request, &source)` (was `answers(&request)`). Task 7 uses `edit_coverage` and `blend_gray_by_coverage`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/selection_edits.rs` ports HueSaturationTests' `adjustmentStaysInsideTheSelectionAndIsOneUndoStep` (its 40 x 20 fixture, points and tolerance 8), LevelsTests' `histogramExcludesTransparencyAndWeightsSelection` and `selectionPreviewCancelCommitUndoAndPersistence` (the 3 x 1 and 6 x 1 fixtures), and SelectionEditTests' `invertKeepsTransparencyStaysInSelectionAndWorksOnMasks` and the uniform-mask case of `invertHandlesLargeImagesAndUniformMasksWithASelection` (at 100 x 40, selecting 30 columns rather than half, so a wrong side shows); then the grown grid, the refusals and the preview key:

```rust
//! Edits limited to the selection (Phase 4a), ported from Compositor for Mac's
//! HueSaturationTests.adjustmentStaysInsideTheSelectionAndIsOneUndoStep,
//! LevelsTests.histogramExcludesTransparencyAndWeightsSelection /
//! selectionPreviewCancelCommitUndoAndPersistence and
//! SelectionEditTests.invertKeepsTransparencyStaysInSelectionAndWorksOnMasks /
//! invertIsFastOnLargeImagesAndHandlesUniformMasksWithASelection, with their fixtures, points and
//! tolerances; plus the blur on its grown grid (Filters.swift:399-413) and preview key (N3).
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn corners(x: f64, y: f64, w: f64, h: f64) -> Vec<Point> { vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)] }
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64, antialiased: bool) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(x, y, w, h), mode: SelectionMode::Replace, antialiased });
}
/// A document of the raster's size holding it as its one layer, placed over the whole canvas.
fn session_with(raster: &Raster) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(raster.width, raster.height, false).unwrap();
    e.import_image(Some(id), &encode_png(raster, 72.0).unwrap(), "Layer", None).unwrap();
    let layer = e.state(id).unwrap().active_layer_id.unwrap();
    (e, id, layer)
}
fn canvas(e: &Engine, id: Uuid) -> Raster {
    let s = e.state(id).unwrap();
    e.composite(id, Rect { x: 0.0, y: 0.0, width: s.width as f64, height: s.height as f64 }, s.width, s.height).unwrap()
}
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { canvas(e, id).pixel(x, y) }
fn near(value: [u8; 4], target: [u8; 4], tolerance: i32) -> bool { value.iter().zip(target).all(|(a, b)| (*a as i32 - b as i32).abs() <= tolerance) }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
fn refused(e: &mut Engine, id: Uuid, c: Command) -> bool {
    matches!(e.execute(id, c), Err(CommandError::Refused(m)) if m == compositor_engine::ops::selection::EMPTY_SELECTION)
}
/// `rows` rows of `width` pixels, each `f(x, y)` premultiplied.
fn raster(width: u32, height: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width { data.extend_from_slice(&f(x, y)); } }
    Raster::from_premultiplied(width, height, data)
}
/// HueSaturationTests.makeSession: 40x20, left half red, right half grey 0.5, and rows 16-19 half
/// transparent blue across the width.
fn colors() -> Raster {
    raster(40, 20, |x, y| if y >= 16 { [0, 0, 128, 128] } else if x < 20 { [255, 0, 0, 255] } else { [128, 128, 128, 255] })
}
/// SelectionEditTests.twoColorLayer: 100x40, red left half, blue right half.
fn two_colors() -> Raster { raster(100, 40, |x, _| if x < 50 { [255, 0, 0, 255] } else { [0, 0, 255, 255] }) }
fn hue(degrees: f64) -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
    a.hsv_settings = Some(HueSaturationSettings::new(degrees, 0.0, 0.0, false, ColorRange::Master));
    a
}
fn inverting_levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { output_black: 255.0, output_white: 0.0, ..LevelRange::default() };
    a
}

#[test]
fn a_hue_rotation_stays_inside_the_selection_and_is_one_undo_step() {
    let (mut e, id, layer) = session_with(&colors());
    select(&mut e, id, 0.0, 0.0, 10.0, 20.0, true);
    let before = depth(&e, id);
    run(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: hue(120.0) });
    assert_eq!(depth(&e, id), before + 1, "one undo step");
    assert!(near(pixel(&e, id, 5, 5), [0, 255, 0, 255], 8), "inside: rotated, {:?}", pixel(&e, id, 5, 5));
    assert!(near(pixel(&e, id, 15, 5), [255, 0, 0, 255], 8), "outside: untouched, {:?}", pixel(&e, id, 15, 5));
    e.undo(id).unwrap();
    assert!(near(pixel(&e, id, 5, 5), [255, 0, 0, 255], 8));
}

#[test]
fn the_histogram_excludes_transparency_and_weights_the_selection() {
    let source = raster(3, 1, |x, _| [[255, 0, 0, 255], [0, 128, 0, 128], [0, 0, 0, 0]][x as usize]);
    let (mut e, id, layer) = session_with(&source);
    let bins = e.histogram(id, layer).unwrap();
    assert!(bins[1][255] == 1.0 && (bins[2][255] - 128.0 / 255.0).abs() < 0.00001);
    let total: f64 = bins[0].iter().sum();
    assert!((total - (1.0 + 128.0 / 255.0)).abs() < 0.00001);
    select(&mut e, id, 0.0, 0.0, 1.0, 1.0, false);
    let selected = e.histogram(id, layer).unwrap();
    assert!(selected[1][255] == 1.0 && selected[2][255] == 0.0, "only the selected red pixel counts");
    // An adjustment layer never takes the selection: its histogram reads everything beneath it.
    run(&mut e, id, Command::AddAdjustmentLayer { kind: AdjustmentKind::Levels, seed: 0, shadows: None, highlights: None });
    let adjustment = e.state(id).unwrap().active_layer_id.unwrap();
    let beneath = e.histogram(id, adjustment).unwrap();
    assert!((beneath[2][255] - 128.0 / 255.0).abs() < 0.00001, "the green pixel counts beneath an adjustment layer");
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 3.0, 1.0), mode: SelectionMode::Subtract, antialiased: false });
    assert!(e.document(id).unwrap().selection.as_ref().unwrap().is_empty());
    assert!(e.histogram(id, layer).unwrap().iter().flatten().all(|v| *v == 0.0), "an empty selection counts nothing");
}

#[test]
fn a_levels_preview_and_commit_stay_in_the_selection_through_undo_and_saving() {
    let ramp = raster(6, 1, |x, _| [[0, 0, 0, 255], [64, 64, 64, 255], [128, 128, 128, 255], [255, 255, 255, 255], [64, 32, 0, 128], [0, 0, 0, 0]][x as usize]);
    let (mut e, id, layer) = session_with(&ramp);
    select(&mut e, id, 0.0, 0.0, 2.0, 1.0, true);
    let before = depth(&e, id);
    e.set_preview(id, Some(PreviewRequest::Adjustment { layer, adjustment: inverting_levels() })).unwrap();
    let preview = e.layer_raster(id, layer, 0).unwrap().unwrap();
    assert_eq!((preview.bytes()[0], preview.bytes()[8]), (255, 128), "inside inverted, outside untouched");
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap(), &ramp, "the document is untouched");
    e.set_preview(id, None).unwrap();
    assert_eq!(depth(&e, id), before);
    run(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: inverting_levels() });
    assert_eq!(depth(&e, id), before + 1);
    let committed = e.document(id).unwrap().layer(layer).unwrap().pixels.clone().unwrap();
    assert_eq!(committed, preview, "the commit is the preview");
    e.undo(id).unwrap();
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().pixels.as_ref().unwrap(), &ramp);
    e.redo(id).unwrap();
    let package = e.save_package(id).unwrap();
    let reopened = e.open_package(&package, None).unwrap();
    assert!(e.state(reopened).unwrap().selection.is_none(), "a selection is never saved");
    assert_eq!(canvas(&e, reopened), canvas(&e, id), "the pixels are");
}

#[test]
fn invert_stays_in_the_selection_and_works_on_masks() {
    let (mut e, id, layer) = session_with(&two_colors());
    select(&mut e, id, 0.0, 0.0, 100.0, 20.0, true);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert_eq!(pixel(&e, id, 10, 5), [0, 255, 255, 255], "red to cyan");
    assert_eq!(pixel(&e, id, 90, 5), [255, 255, 0, 255], "blue to yellow");
    assert_eq!(pixel(&e, id, 10, 30), [255, 0, 0, 255], "outside the selection: unchanged");
    run(&mut e, id, Command::Deselect);
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert_eq!(pixel(&e, id, 50, 30)[3], 0, "an all-white mask inverts to hide everything");
}

#[test]
fn a_uniform_mask_inverts_only_the_selected_part() {
    let (mut e, id, layer) = session_with(&raster(100, 40, |_, _| [255, 0, 0, 255]));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 0.0, 0.0, 30.0, 40.0, true);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert!(pixel(&e, id, 10, 20)[3] == 0 && pixel(&e, id, 80, 20)[3] == 255);
    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap();
    assert_eq!((mask.pixels.width, mask.pixels.height), (100, 40), "the 1x1 mask took the layer's pixel grid");
    // Layer > Mask > Invert Mask is the same edit.
    run(&mut e, id, Command::InvertMask { id: layer });
    assert!(pixel(&e, id, 10, 20)[3] == 255 && pixel(&e, id, 80, 20)[3] == 255);
}

#[test]
fn a_blur_spreads_only_where_the_selection_reaches() {
    // A 20x20 opaque red block over x 20..40 of a 60x20 canvas; the selection covers x 0..30.
    let mut e = Engine::new();
    let id = e.new_document(60, 20, false).unwrap();
    let block = raster(20, 20, |_, _| [255, 0, 0, 255]);
    e.import_image(Some(id), &encode_png(&block, 72.0).unwrap(), "Block", Some(p(30.0, 10.0))).unwrap();
    let layer = e.state(id).unwrap().active_layer_id.unwrap();
    select(&mut e, id, 0.0, 0.0, 30.0, 20.0, true);
    let blur = FilterParams::GaussianBlur { radius: 3.0 };
    e.set_preview(id, Some(PreviewRequest::Filter { layer, params: blur.clone() })).unwrap();
    let previewed = canvas(&e, id);
    run(&mut e, id, Command::ApplyFilter { id: layer, params: blur });
    let t = e.document(id).unwrap().layer(layer).unwrap().transform;
    assert!(t.origin.x < 20.0, "the blur spread left, into the selection: {t:?}");
    assert_eq!(t.origin.x + t.size.width, 40.0, "and not right, outside it: {t:?}");
    assert_eq!(pixel(&e, id, 38, 10), [255, 0, 0, 255], "the unselected edge stays hard");
    assert!(pixel(&e, id, 20, 10)[3] < 255 && pixel(&e, id, 19, 10)[3] > 0, "the selected edge is soft");
    assert_eq!(canvas(&e, id), previewed, "the preview blended on the same grown grid");
}

#[test]
fn an_empty_selection_refuses_every_edit() {
    let (mut e, id, layer) = session_with(&two_colors());
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0, true);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 100.0, 40.0), mode: SelectionMode::Subtract, antialiased: true });
    let (before, shown) = (depth(&e, id), canvas(&e, id));
    assert!(refused(&mut e, id, Command::ApplyAdjustment { id: layer, adjustment: hue(120.0) }));
    assert!(refused(&mut e, id, Command::InvertPixels { id: layer, mask: false }));
    assert!(refused(&mut e, id, Command::InvertPixels { id: layer, mask: true }));
    assert!(refused(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 2.0 } }));
    e.set_preview(id, Some(PreviewRequest::Adjustment { layer, adjustment: hue(120.0) })).unwrap();
    assert_eq!((depth(&e, id), canvas(&e, id)), (before, shown), "nothing changed, nothing previewed");
}

#[test]
fn a_kept_preview_must_have_been_made_from_the_same_pixels_and_selection() {
    let mut doc = Document::new(8, 8);
    let layer = Layer::with_pixels("Grey", raster(8, 8, |_, _| [128, 128, 128, 255]), p(0.0, 0.0));
    let lid = layer.id;
    doc.layers.push(layer);
    let request = PreviewRequest::Adjustment { layer: lid, adjustment: inverting_levels() };
    let preview = compute_preview(&doc, &request, 1).unwrap();
    assert!(preview.answers(&request, &PreviewSource::of(&doc, lid)));
    let mut edited = doc.clone();
    edited.layer_mut(lid).unwrap().set_pixels(Some(raster(8, 8, |_, _| [0, 0, 0, 255])));
    assert!(!preview.answers(&request, &PreviewSource::of(&edited, lid)), "other pixels: recompute");
    let mut selected = doc.clone();
    selected.selection_revision += 1;
    assert!(!preview.answers(&request, &PreviewSource::of(&selected, lid)), "another selection: recompute");
}
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test selection_edits`: it does not compile (`PreviewSource`, `answers` with two arguments). With the last test left out, the other seven all fail (checked on the scratch copy): every edit still reaches the whole layer (the hue rotation turns (15, 5) green, the histogram counts the green pixel, the preview and the commit invert column 2, the invert turns row 30 cyan, the uniform mask hides everything, the blur spreads right) and an empty selection is not refused.

- [ ] **Step 3: The edits read the selection**

```diff
--- a/engine/src/adjust/apply.rs
+++ b/engine/src/adjust/apply.rs
@@ -15,6 +15,16 @@ pub fn blend_by_coverage(adjusted: &Raster, original: &Raster, coverage: &GrayRa
     Raster::from_premultiplied(adjusted.width, adjusted.height, data)
 }
 
+/// `blend_by_coverage` for a mask: `coverage * adjusted + (1 - coverage) * original`, the same size.
+pub fn blend_gray_by_coverage(adjusted: &GrayRaster, original: &GrayRaster, coverage: &GrayRaster) -> GrayRaster {
+    let base = original.bytes();
+    let data = adjusted.bytes().iter().zip(coverage.bytes()).enumerate().map(|(i, (&a, &k))| {
+        let (a, b, k) = (a as u32, base[i] as u32, k as u32);
+        ((a * k + b * (255 - k) + 127) / 255) as u8
+    }).collect();
+    GrayRaster::from_bytes(adjusted.width, adjusted.height, data)
+}
+
 /// A whole raster through one adjustment. `origin` and `units_per_pixel` place the raster in
 /// document space (Grain reads them); `selection` limits the change to its coverage.
 pub fn apply_adjustment(raster: &Raster, a: &LayerAdjustment, origin: Point, units_per_pixel: f64, selection: Option<&GrayRaster>) -> Raster {
```

```diff
--- a/engine/src/ops/adjust.rs
+++ b/engine/src/ops/adjust.rs
@@ -15,29 +15,53 @@ fn placement(layer: &Layer) -> (Point, f64) {
     (layer.transform.origin, layer.transform.size.width / w.max(1) as f64)
 }
 
+/// The selection's coverage on a `width` x `height` grid that `transform` places (a layer's pixels,
+/// a grown copy of them, or a mask): None when nothing is selected, so the edit reaches the whole
+/// grid. An empty selection refuses the edit, as the Mac's `canAdjustColors` / `canInvert` do.
+pub fn edit_coverage(doc: &Document, transform: &LayerTransform, width: u32, height: u32) -> Result<Option<GrayRaster>, CommandError> {
+    if doc.selection.as_ref().map_or(false, |s| s.is_empty()) {
+        return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into()));
+    }
+    Ok(selection_coverage(doc, &transform.pixel_to_document(width, height), width, height))
+}
+
 pub fn apply_adjustment_to_layer(doc: &mut Document, id: Uuid, a: &LayerAdjustment) -> Result<(), CommandError> {
     if !a.is_valid() { return Err(CommandError::Argument("adjustment settings out of range".into())); }
     if a.kind.is_spatial() { return Err(CommandError::Argument("a blur is applied with Filter > Gaussian Blur or Motion Blur".into())); }
     let layer = pixel_layer(doc, id)?;
     let (origin, units) = placement(layer);
     let raster = layer.pixels.as_ref().unwrap();
+    let coverage = edit_coverage(doc, &layer.transform, raster.width, raster.height)?;
     // Kernel function, not this module's own `apply_filter`: adjust::apply::apply_adjustment.
-    let adjusted = adjust::apply::apply_adjustment(raster, a, origin, units, None);
+    let adjusted = adjust::apply::apply_adjustment(raster, a, origin, units, coverage.as_ref());
     doc.layer_mut(id).unwrap().set_pixels(Some(adjusted));
     Ok(())
 }
 
+/// Image > Invert on the layer's pixels or its mask, inside the selection when there is one
+/// (`invertPixels`, SelectionEdits.swift:85-122). A uniform 1x1 mask cannot hold a partial
+/// selection, so it takes the layer's pixel grid first.
 pub fn invert_layer(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
     if mask {
         let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
-        let Some(m) = &layer.mask else { return Err(CommandError::Argument("the layer has no mask".into())); };
+        if layer.mask.is_none() { return Err(CommandError::Argument("the layer has no mask".into())); }
+        if doc.selection.as_ref().map_or(false, |s| s.is_empty()) { return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into())); }
+        if doc.selection.is_some() { ops::masks::expand_uniform(doc, id)?; }
+        let layer = doc.layer(id).unwrap();
+        let m = layer.mask.as_ref().unwrap();
+        let grid = m.placement.unwrap_or(layer.transform);
+        let coverage = edit_coverage(doc, &grid, m.pixels.width, m.pixels.height)?;
         let inverted = adjust::tonal::invert_gray(&m.pixels);
-        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = inverted;
+        let result = match coverage { Some(c) => adjust::apply::blend_gray_by_coverage(&inverted, &m.pixels, &c), None => inverted };
+        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = result;
         return Ok(());
     }
     let layer = pixel_layer(doc, id)?;
-    let inverted = adjust::tonal::invert_raster(layer.pixels.as_ref().unwrap());
-    doc.layer_mut(id).unwrap().set_pixels(Some(inverted));
+    let raster = layer.pixels.as_ref().unwrap();
+    let coverage = edit_coverage(doc, &layer.transform, raster.width, raster.height)?;
+    let inverted = adjust::tonal::invert_raster(raster);
+    let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&inverted, raster, &c), None => inverted };
+    doc.layer_mut(id).unwrap().set_pixels(Some(result));
     Ok(())
 }
 
@@ -105,7 +129,8 @@ fn carry_mask(mask: &Mask, old: &LayerTransform, new: &LayerTransform) -> GrayRa
     GrayRaster::from_bytes(w, h, data)
 }
 
-/// One filter on a layer: a blur is given room to spread, run, then cut back to what it left.
+/// One filter on a layer: a blur is given room to spread, run, blended back through the selection
+/// on that grown grid, then cut back to what it left (`commitFilter`, Filters.swift:399-413).
 pub fn apply_filter(doc: &mut Document, id: Uuid, params: &FilterParams) -> Result<(), CommandError> {
     let params = params.normalized();
     if params.is_identity() { return Ok(()); }
@@ -115,8 +140,10 @@ pub fn apply_filter(doc: &mut Document, id: Uuid, params: &FilterParams) -> Resu
         true => grown(raster, &layer.transform, params.margin()).ok_or(CommandError::Project(ProjectError::TooLarge))?,
         false => (raster.clone(), layer.transform),
     };
+    let coverage = edit_coverage(doc, &placed, source.width, source.height)?;
     // Kernel function, not this module's own `apply_filter`: adjust::filters::apply_filter.
     let filtered = adjust::filters::apply_filter(&source, &params);
+    let filtered = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &source, &c), None => filtered };
     let (result, transform) = if params.spreads() { trimmed(&filtered, &placed) } else { (filtered, placed) };
     let mask = match &layer.mask {
         Some(m) if m.placement.is_none() && !m.is_uniform() && transform != layer.transform => {
```

Layer > Mask > Invert Mask is the same edit (ruling OQ14):

```diff
--- a/engine/src/ops/masks.rs
+++ b/engine/src/ops/masks.rs
@@ -33,8 +33,10 @@ fn replace_pixels(doc: &mut Document, id: Uuid, f: impl FnOnce(&GrayRaster) -> G
     Ok(())
 }
 
+/// Layer > Mask > Invert Mask: the same edit as Image > Invert with the mask targeted, so it too
+/// stays inside the selection (the Mac has only the one, `invertPixels`).
 pub fn invert_mask(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
-    replace_pixels(doc, id, |m| GrayRaster::from_bytes(m.width, m.height, m.bytes().iter().map(|v| 255 - v).collect()))
+    crate::ops::adjust::invert_layer(doc, id, true)
 }
 
 pub fn fill_mask(doc: &mut Document, id: Uuid, white: bool) -> Result<(), CommandError> {
```

The preview takes the coverage on the grid it is computed on (reduced for a large layer, grown for a blur), and keeps what it was made from:

```diff
--- a/engine/src/preview.rs
+++ b/engine/src/preview.rs
@@ -31,13 +31,27 @@ impl PreviewRequest {
     }
 }
 
-/// The substituted pixels for one layer while a panel is open, and the request that made them.
+/// What a preview was computed from besides its request: the stored layer's pixels revision and
+/// placement, and the selection's revision (0 for a missing layer).
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub struct PreviewSource { pub pixels_revision: u64, pub transform: Option<LayerTransform>, pub selection_revision: u64 }
+
+impl PreviewSource {
+    pub fn of(doc: &Document, layer: Uuid) -> PreviewSource {
+        let l = doc.layer(layer);
+        PreviewSource { pixels_revision: l.map_or(0, |l| l.pixels_revision), transform: l.map(|l| l.transform), selection_revision: doc.selection_revision }
+    }
+}
+
+/// The substituted pixels for one layer while a panel is open, the request that made them and
+/// what they were made from.
 #[derive(Clone, Debug)]
-pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest }
+pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest, pub source: PreviewSource }
 
 impl PixelPreview {
-    /// Whether `request` would compute exactly these pixels again, so they can be kept.
-    pub fn answers(&self, request: &PreviewRequest) -> bool { self.request.same_output(request) }
+    /// Whether `request`, on a document that is now `source`, would compute exactly these pixels
+    /// again, so they can be kept.
+    pub fn answers(&self, request: &PreviewRequest, source: &PreviewSource) -> bool { self.request.same_output(request) && self.source == *source }
 }
 
 // Preview sizes. A colour adjustment's preview costs about 0.1 us per preview pixel in release
@@ -80,19 +94,23 @@ fn reduced(raster: &Raster, limit: u32) -> (Raster, f64) {
     (current, factor)
 }
 
-/// The preview raster for a request, or None when there is nothing to show.
+/// The preview raster for a request, or None when there is nothing to show (an empty selection
+/// included: every edit refuses it). The selection's coverage is taken on the grid the preview is
+/// computed on, reduced or grown, as the Mac's `previewMapping` does.
 pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
     let layer = doc.layer(request.layer())?;
     let raster = layer.pixels.as_ref()?;
     let limit = preview_limit(request);
     let (source, factor) = reduced(raster, limit);
+    let made_from = PreviewSource::of(doc, layer.id);
     match request {
         PreviewRequest::Adjustment { adjustment, .. } | PreviewRequest::DragAdjustment { adjustment, .. } => {
             if !adjustment.is_valid() { return None; }
+            let coverage = ops::adjust::edit_coverage(doc, &layer.transform, source.width, source.height).ok()?;
             // Grain and the tonal kernels read document space, which the reduced grid still covers.
             let units = layer.transform.size.width / source.width.max(1) as f64;
-            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, None);
-            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision, request: request.clone() })
+            let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, coverage.as_ref());
+            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision, request: request.clone(), source: made_from })
         }
         PreviewRequest::Filter { params, .. } => {
             let params = params.normalized();
@@ -103,8 +121,10 @@ pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64)
                 true => ops::adjust::grown(&source, &layer.transform, scaled.margin())?,
                 false => (source, layer.transform),
             };
-            let result = adjust::filters::apply_filter(&grid, &scaled);
-            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision, request: request.clone() })
+            let coverage = ops::adjust::edit_coverage(doc, &placed, grid.width, grid.height).ok()?;
+            let filtered = adjust::filters::apply_filter(&grid, &scaled);
+            let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &grid, &c), None => filtered };
+            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision, request: request.clone(), source: made_from })
         }
     }
 }
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -241,8 +241,12 @@ impl Engine {
     pub fn set_preview(&mut self, id: Uuid, request: Option<PreviewRequest>) -> Result<Dirty, CommandError> {
         // A request that would compute the pixels already showing keeps them: the settled request
         // after a Grain drag (full size either way) would otherwise recompute the whole layer.
-        if let (Some(r), Some(current)) = (&request, &self.session(id)?.preview) {
-            if current.answers(r) { return Ok(Dirty { structure: false, canvas: false, layers: vec![] }); }
+        // The key includes what the pixels were computed FROM (the stored layer's revision and
+        // placement, and the selection's revision), so an edit that forgets to clear the preview
+        // can never be answered with stale pixels (phase 3 open item N3).
+        let s = self.session(id)?;
+        if let (Some(r), Some(current)) = (&request, &s.preview) {
+            if current.answers(r, &PreviewSource::of(&s.document, r.layer())) { return Ok(Dirty { structure: false, canvas: false, layers: vec![] }); }
         }
         let revision = { self.preview_revision += 1; PREVIEW_REVISION_BASE + self.preview_revision };
         let s = self.session_mut(id)?;
@@ -484,13 +488,16 @@ impl Engine {
         Ok(compositor::composite_edit_with(&below, None, Rect { x: 0.0, y: 0.0, width: below.width as f64, height: below.height as f64 }, below.width, below.height, &self.effects))
     }
     /// A panel's histogram: an adjustment layer reads what lies beneath it, any other layer its
-    /// own stored pixels (never the preview, or the graph would chase itself).
+    /// own stored pixels (never the preview, or the graph would chase itself), weighted by the
+    /// selection's coverage on those pixels (`LevelsFilter.histogram`, Levels.swift:95-110). An
+    /// adjustment layer never takes the selection, so its histogram does not either.
     pub fn histogram(&self, id: Uuid, layer: Uuid) -> Result<Vec<Vec<f64>>, CommandError> {
         let doc = &self.session(id)?.document;
         let target = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         if target.is_adjustment() { return Ok(adjust::levels::histogram(&self.adjustment_source(id, layer)?, None)); }
         let raster = target.pixels.as_ref().ok_or(CommandError::Argument("the layer has no pixels".into()))?;
-        Ok(adjust::levels::histogram(raster, None))
+        let coverage = selection_coverage(doc, &target.transform.pixel_to_document(raster.width, raster.height), raster.width, raster.height);
+        Ok(adjust::levels::histogram(raster, coverage.as_ref()))
     }
     pub fn auto_levels(&self, id: Uuid, layer: Uuid, mode: LevelsAuto) -> Result<LevelsSettings, CommandError> {
         Ok(mode.settings(&self.histogram(id, layer)?))
```

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test selection_edits` (8 tests), then `--test preview --test adjust_ops --test levels` (unchanged, still passing), then the whole engine suite: 443 passed, 1 ignored (+8).

- [ ] **Step 5: Prove it bites**

(1) In `apply_adjustment_to_layer`, pass `None` instead of `coverage.as_ref()`: `a_hue_rotation_stays_inside_the_selection_and_is_one_undo_step` and `a_levels_preview_and_commit_stay_in_the_selection_through_undo_and_saving` fail (the commit differs from the preview). Restore. (2) In `apply_filter`, drop the blend (`let _ = coverage;`): `a_blur_spreads_only_where_the_selection_reaches` fails (the right edge moves out to 48). Both measured on the scratch copy. Restore. (3) In `PixelPreview::answers`, drop `&& self.source == *source`: `a_kept_preview_must_have_been_made_from_the_same_pixels_and_selection` fails. Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/tests/selection_edits.rs
git commit -- engine/src/ops/adjust.rs engine/src/adjust/apply.rs engine/src/ops/masks.rs engine/src/preview.rs engine/src/engine.rs engine/tests/selection_edits.rs -m "feat: adjustments, filters and Invert inside the selection; the histogram weighted by it; previews keyed on their source (N3)" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Delete clears the selected pixels; Add Mask paints through the selection

With a selection, Delete clears the selected pixels ("Clear"), or on a targeted mask fills the selection with the mask palette's background, white ("Fill Mask"), and does nothing where painting is refused (SelectionEdits.swift:49-63, `canPaint` in EditorSession+Brush.swift:5-11; ruling OQ13). Add Mask with a selection makes the mask on the layer's pixel grid, white with the selection painted black (or the reverse for a hiding mask) through the selection's clip, feather included, and uses the selection up in the same step: "Add Mask from Selection" (LayerMask.swift:228-260; ruling OQ12). This task adds both as commands; the app routes Delete and every Add Mask to them in Task 10.

**Files:**
- Modify: `engine/src/command.rs`, `engine/src/ops/selection.rs` (`clear_selected`, `add_mask_from_selection`), `engine/src/engine.rs`
- Create test: `engine/tests/selection_masks.rs`

**Interfaces:**
- Consumes: Task 6's `edit_coverage` and `blend_gray_by_coverage`; `ops::masks::expand_uniform`.
- Produces: `Command::ClearSelectedPixels { id, mask }` ("Clear" / "Fill Mask") and `Command::AddMaskFromSelection { id, revealing }` ("Add Mask from Selection"); `ops::selection::{clear_selected, add_mask_from_selection}`.

- [ ] **Step 1: Write the failing tests**

`engine/tests/selection_masks.rs` ports SelectionEditTests' `deleteClearsSelectedPixelsOrDeletesTheLayerWithoutASelection`, `clipFollowsScaledLayersAndSoftensEdges`, `maskButtonAddsWhiteMaskOrHidesTheSelection`, `layerMenuMasksUseTheSelection`, `maskFromSelectionLinesUpOnScaledLayers` and, adapted because fills are not in this phase, `maskFillHidesOnlyTheSelectedArea`; the feathered mask's expected values come from the Gaussian's formula:

```rust
//! Delete with a selection and Add Mask from Selection (Phase 4a), ported from Compositor for
//! Mac's SelectionEditTests: deleteClearsSelectedPixelsOrDeletesTheLayerWithoutASelection,
//! clipFollowsScaledLayersAndSoftensEdges, maskButtonAddsWhiteMaskOrHidesTheSelection,
//! layerMenuMasksUseTheSelection, maskFromSelectionLinesUpOnScaledLayers and (adapted: Fill is not
//! in this phase) maskFillHidesOnlyTheSelectedArea, with their sizes, points and expectations.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn corners(x: f64, y: f64, w: f64, h: f64) -> Vec<Point> { vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)] }
fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(x, y, w, h), mode: SelectionMode::Replace, antialiased: true });
}
fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Raster { Raster::from_premultiplied(width, height, rgba.repeat((width * height) as usize)) }
/// A `width` x `height` document holding `layer` stretched over the whole canvas.
fn session(width: u32, height: u32, layer: &Raster) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, false).unwrap();
    e.import_image(Some(id), &encode_png(layer, 72.0).unwrap(), "Layer", None).unwrap();
    let lid = e.state(id).unwrap().active_layer_id.unwrap();
    let mut t = e.document(id).unwrap().layer(lid).unwrap().transform;
    t.origin = p(0.0, 0.0);
    t.size = Size { width: width as f64, height: height as f64 };
    run(&mut e, id, Command::SetLayerTransform { id: lid, transform: t });
    (e, id, lid)
}
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] {
    let s = e.state(id).unwrap();
    e.composite(id, Rect { x: 0.0, y: 0.0, width: s.width as f64, height: s.height as f64 }, s.width, s.height).unwrap().pixel(x, y)
}
fn has_selection(e: &Engine, id: Uuid) -> bool { e.state(id).unwrap().selection.is_some() }
fn has_mask(e: &Engine, id: Uuid, layer: Uuid) -> bool { e.document(id).unwrap().layer(layer).unwrap().mask.is_some() }
const RED: [u8; 4] = [255, 0, 0, 255];

#[test]
fn delete_clears_the_selected_pixels_as_one_step() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    assert!(matches!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }), Err(CommandError::Refused(_))), "nothing selected: the app deletes the layer instead");
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(Command::ClearSelectedPixels { id: layer, mask: false }.action_name(), "Clear");
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1);
    assert_eq!(pixel(&e, id, 30, 20)[3], 0);
    assert_eq!(pixel(&e, id, 5, 5), RED);
    e.undo(id).unwrap();
    assert_eq!(pixel(&e, id, 30, 20), RED);
}

#[test]
fn the_clear_follows_a_scaled_layer_and_softens_a_slanted_edge() {
    // A 50x20 image stretched 2x over the 100x40 canvas, a triangle selected.
    let (mut e, id, layer) = session(100, 40, &solid(50, 20, [0, 0, 255, 255]));
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Freehand, points: vec![p(0.0, 0.0), p(100.0, 0.0), p(0.0, 40.0)], mode: SelectionMode::Replace, antialiased: true });
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(pixel(&e, id, 10, 10)[3], 0, "inside the triangle: cleared");
    assert_eq!(pixel(&e, id, 90, 35)[3], 255, "outside: untouched");
    let edge: Vec<u8> = (0..100).map(|x| pixel(&e, id, x, ((1.0 - x as f64 / 100.0) * 40.0).min(39.0) as u32)[3]).collect();
    assert!(edge.iter().any(|a| *a > 0 && *a < 255), "along the diagonal some pixels are partly cleared: {edge:?}");
}

#[test]
fn delete_on_a_mask_fills_the_selection_white() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: true });
    assert!(pixel(&e, id, 30, 20)[3] == 0 && pixel(&e, id, 5, 5)[3] == 255, "the selected area hidden");
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(Command::ClearSelectedPixels { id: layer, mask: true }.action_name(), "Fill Mask");
    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "white, the mask's background, reveals again");
    run(&mut e, id, Command::SetMaskEnabled { id: layer, enabled: false });
    assert!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: true }).is_err(), "a disabled mask takes no edit");
}

#[test]
fn add_mask_with_a_selection_hides_the_selection_and_uses_it_up() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1, "one step");
    assert!(!has_selection(&e, id), "the selection is used up");
    assert_eq!(pixel(&e, id, 30, 20)[3], 0, "selected area: black, hidden");
    assert_eq!(pixel(&e, id, 5, 5)[3], 255, "everything else: white, visible");
    e.undo(id).unwrap();
    assert!(!has_mask(&e, id, layer) && has_selection(&e, id));
}

#[test]
fn add_black_mask_with_a_selection_shows_only_the_selection() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: false });
    assert!(!has_selection(&e, id));
    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "selected area: white, visible");
    assert_eq!(pixel(&e, id, 5, 5)[3], 0, "everything else: black, hidden");
}

#[test]
fn a_mask_from_a_selection_lines_up_on_a_scaled_layer() {
    let (mut e, id, layer) = session(100, 100, &solid(50, 50, [0, 0, 255, 255]));
    select(&mut e, id, 0.0, 0.0, 50.0, 50.0);
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert_eq!(e.document(id).unwrap().layer(layer).unwrap().mask.as_ref().unwrap().pixels.width, 50, "the mask uses the layer's pixel grid");
    assert_eq!(pixel(&e, id, 25, 25)[3], 0);
    assert_eq!(pixel(&e, id, 75, 75)[3], 255);
    assert_eq!(pixel(&e, id, 75, 25)[3], 255);
}

#[test]
fn a_mask_from_a_feathered_selection_takes_the_feather() {
    // Feather 4 is a Gaussian of sigma 2 across the edge at x = 20: the pixel centred 1.5 px
    // outside (x 18) is about Phi(-0.75) = 23 % selected, 1.5 px inside (x 21) about 77 %, so a
    // revealing mask there is about 197 and 58 (LayerMask.swift:245-249 paints through the clip).
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::FeatherSelection { amount: 4 });
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
    let (outside, inside) = (mask.bytes()[20 * 100 + 18] as i32, mask.bytes()[20 * 100 + 21] as i32);
    assert!((outside - 197).abs() <= 8 && (inside - 58).abs() <= 8, "outside {outside}, inside {inside}");
}

#[test]
fn an_empty_selection_clears_nothing_but_still_makes_a_plain_mask() {
    let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 100.0, 40.0), mode: SelectionMode::Subtract, antialiased: true });
    assert!(matches!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }), Err(CommandError::Refused(m)) if m == compositor_engine::ops::selection::EMPTY_SELECTION));
    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
    assert!(!has_selection(&e, id) && pixel(&e, id, 30, 20)[3] == 255, "a plain white mask, the empty selection used up (LayerMask.swift:245)");
}
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test selection_masks`: it does not compile (the two commands do not exist).

- [ ] **Step 3: The two commands**

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -63,6 +63,10 @@ pub enum Command {
     MagicWand { at: Point, mode: SelectionMode, settings: WandSettings, antialiased: bool },
     LoadLayerSelection { #[serde(with = "ids::upper")] id: Uuid, mode: SelectionMode, antialiased: bool },
     LoadMaskSelection { #[serde(with = "ids::upper")] id: Uuid, mode: SelectionMode, antialiased: bool },
+    /// Delete with a selection: the selected pixels cleared, or the mask filled white there (`mask`).
+    ClearSelectedPixels { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool },
+    /// Add Mask with a selection: `revealing` white with the selection black, or the reverse.
+    AddMaskFromSelection { #[serde(with = "ids::upper")] id: Uuid, revealing: bool },
 }
 
 impl Command {
@@ -126,6 +130,10 @@ impl Command {
             Command::MagicWand { .. } => "Magic Wand",
             Command::LoadLayerSelection { .. } => "Load Layer Selection",
             Command::LoadMaskSelection { .. } => "Load Mask Selection",
+            // SelectionEdits.swift:46 and :55, LayerMask.swift:254.
+            Command::ClearSelectedPixels { mask: false, .. } => "Clear",
+            Command::ClearSelectedPixels { mask: true, .. } => "Fill Mask",
+            Command::AddMaskFromSelection { .. } => "Add Mask from Selection",
         }
     }
 }
```

```diff
--- a/engine/src/ops/selection.rs
+++ b/engine/src/ops/selection.rs
@@ -160,6 +160,62 @@ pub fn load_layer_selection(doc: &mut Document, id: Uuid, mode: SelectionMode, a
     Ok(())
 }
 
+/// Delete with a selection (`clearSelectedPixels`, SelectionEdits.swift:51-56): the layer's pixels
+/// fade to transparent by the selection's coverage (`clearPixels`, BrushStroke.swift:631-637); with
+/// the mask targeted, the mask fills white there instead, the mask palette's background ("Fill
+/// Mask"). A 1x1 mask takes the layer's pixel grid first. Needs a selection with something in it,
+/// and an enabled mask (`canPaint`, EditorSession+Brush.swift:5-11).
+pub fn clear_selected(doc: &mut Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
+    modifiable(doc)?;
+    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    if mask {
+        let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
+        if !m.enabled { return Err(CommandError::Argument("the mask is turned off".into())); }
+        crate::ops::masks::expand_uniform(doc, id)?;
+        let layer = doc.layer(id).unwrap();
+        let m = layer.mask.as_ref().unwrap();
+        let grid = m.placement.unwrap_or(layer.transform);
+        let coverage = crate::ops::adjust::edit_coverage(doc, &grid, m.pixels.width, m.pixels.height)?.unwrap();
+        let white = GrayRaster::from_bytes(m.pixels.width, m.pixels.height, vec![255; (m.pixels.width * m.pixels.height) as usize]);
+        let filled = adjust::apply::blend_gray_by_coverage(&white, &m.pixels, &coverage);
+        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = filled;
+        return Ok(());
+    }
+    if layer.is_group { return Err(CommandError::Argument("folders have no pixels".into())); }
+    let raster = layer.pixels.as_ref().ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
+    let coverage = crate::ops::adjust::edit_coverage(doc, &layer.transform, raster.width, raster.height)?.unwrap();
+    let mut data = raster.bytes().to_vec();
+    for (p, &k) in data.chunks_exact_mut(4).zip(coverage.bytes()) {
+        if k == 0 { continue; }
+        let keep = 255 - k as u32;
+        for v in p.iter_mut() { *v = ((*v as u32 * keep + 127) / 255) as u8; }
+    }
+    let cleared = Raster::from_premultiplied(raster.width, raster.height, data);
+    doc.layer_mut(id).unwrap().set_pixels(Some(cleared));
+    Ok(())
+}
+
+/// Add Mask with a selection (`addMask(revealing:)`, LayerMask.swift:233-260): a mask on the
+/// layer's pixel grid (its rectangle when it has none), white when `revealing` and black
+/// otherwise, painted the opposite tone through the selection's clip -- its coverage with the
+/// feather, cut to the canvas (`selection.clip(canvas:)`, :245-249). The selection is used up in
+/// the same step. An empty selection clips everything away: a plain mask, the selection used up.
+pub fn add_mask_from_selection(doc: &mut Document, id: Uuid, revealing: bool) -> Result<(), CommandError> {
+    let selection = doc.selection.clone().ok_or_else(|| refused(NO_SELECTION))?;
+    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    if layer.mask.is_some() { return Err(CommandError::Argument("the layer already has a mask".into())); }
+    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round() as i64, layer.transform.size.height.round() as i64), |p| (p.width as i64, p.height as i64));
+    if w < 1 || h < 1 || w > MAX_SIDE || h > MAX_SIDE || (w * h) as u64 > MAX_PIXELS - doc.used_mask_pixels().min(MAX_PIXELS) {
+        return Err(CommandError::Project(ProjectError::TooLarge));
+    }
+    let (w, h) = (w as u32, h as u32);
+    let coverage = SelectionClip::new(&selection, doc.width, doc.height).on_grid(&layer.transform.pixel_to_document(w, h), w, h);
+    let pixels = if revealing { GrayRaster::from_bytes(w, h, coverage.bytes().iter().map(|c| 255 - c).collect()) } else { coverage };
+    doc.layer_mut(id).unwrap().set_mask(Some(Mask { pixels, enabled: true, placement: None, linked: None }));
+    doc.selection = None;
+    Ok(())
+}
+
 /// Ctrl-click on a mask's thumbnail (`loadMaskSelection`): the mask's BLACK areas, darker than 50%
 /// grey, which is what it hides (Photoshop loads the white; the Mac, and so this port, the
 /// black), through where the mask sits, combined by `mode`.
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -370,6 +370,8 @@ impl Engine {
             Command::MagicWand { at, mode, settings, antialiased } => { ops::selection::magic_wand_select(doc, at, mode, &settings, antialiased)?; Ok(Dirty::structure()) }
             Command::LoadLayerSelection { id, mode, antialiased } => { ops::selection::load_layer_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
             Command::LoadMaskSelection { id, mode, antialiased } => { ops::selection::load_mask_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
+            Command::ClearSelectedPixels { id, mask } => { ops::selection::clear_selected(doc, id, mask)?; Ok(Dirty { structure: true, canvas: false, layers: if mask { vec![] } else { vec![id] } }) }
+            Command::AddMaskFromSelection { id, revealing } => { ops::selection::add_mask_from_selection(doc, id, revealing)?; Ok(Dirty::structure()) }
         })
     }
 
```

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test selection_masks` (8 tests), then the whole engine suite: 451 passed, 1 ignored (+8).

- [ ] **Step 5: Prove it bites**

(1) In `add_mask_from_selection`, sample the coverage on the canvas's grid (`&Affine::IDENTITY`) instead of the layer's: `a_mask_from_a_selection_lines_up_on_a_scaled_layer` fails. Restore. (2) In `clear_selected`, replace `if k == 0 { continue; }` with `let k = 255u8.max(k);`, which clears every pixel: `delete_clears_the_selected_pixels_as_one_step` and `the_clear_follows_a_scaled_layer_and_softens_a_slanted_edge` fail. Restore. Both measured on the scratch copy.

- [ ] **Step 6: Commit**

```
git add -- engine/tests/selection_masks.rs
git commit -- engine/src/command.rs engine/src/ops/selection.rs engine/src/engine.rs engine/tests/selection_masks.rs -m "feat: Delete clears the selected pixels, and Add Mask paints through the selection" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: The bridge: commands, the selection's summary and its outline in TypeScript

**Files:**
- Modify: `engine-wasm/src/lib.rs` (`selection_outline`, `selection_contains`), `app/src/engine/types.ts`, `app/src/engine/client.ts`
- Modify tests: `app/tests/unit/engine-client.test.ts`; the eight unit tests that build a `DocumentState` (`adjust-store`, `commit-transform`, `crop-seed`, `crop-tool`, `layer-rows`, `prefilter`, `selection`, `store-history`)

**Interfaces:**
- Consumes: Tasks 4, 5 and 7's commands, `Engine::selection_outline`, `Engine::selection_contains`.
- Produces: TypeScript `SelectionMode`, `SelectionShape`, `WandSettings`, `DEFAULT_WAND`, `SelectionState`, `DocumentState.selection: SelectionState | null`, the 13 `Command` variants; `EngineClient.selectionOutline(doc, step): PointTuple[][]` and `EngineClient.selectionContains(doc, at): boolean`; wasm `selection_outline(doc, step) -> Float64Array`, `selection_contains(doc, x, y) -> bool`.

- [ ] **Step 1: Write the failing tests**

The client unpacks the flat outline:

```diff
--- a/app/tests/unit/engine-client.test.ts
+++ b/app/tests/unit/engine-client.test.ts
@@ -77,6 +77,17 @@ describe("pixel views survive a wasm memory growth", () => {
     expect(calls).toEqual(["prepare", "release", "prepare", "release"]);
   });
 
+  it("selectionOutline unpacks the engine's flat outline into contours", () => {
+    // Two contours: a triangle and a two-point sliver, as `Engine::selection_outline` lays them out.
+    const flat = new Float64Array([2, 3, 0, 0, 10, 0, 0, 5.5, 2, 7, 8, 9, 10]);
+    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
+    client.wasm = { selection_outline: (doc: string, step: number) => { expect([doc, step]).toEqual(["D", 0.25]); return flat; } };
+    const c = client as unknown as EngineClient;
+    expect(c.selectionOutline("D", 0.25)).toEqual([[[0, 0], [10, 0], [0, 5.5]], [[7, 8], [9, 10]]]);
+    client.wasm = { selection_outline: () => new Float64Array([]) };
+    expect(c.selectionOutline("D", 1), "no selection").toEqual([]);
+  });
+
   it("both return null rather than a zero-length view when there is nothing to read", () => {
     const memory = new WebAssembly.Memory({ initial: 1 });
     const wasm = { layer_pixels_len: () => 0, mask_pixels_len: () => 0, layer_pixels_ptr: () => { throw new Error("must not be called"); }, mask_pixels_ptr: () => { throw new Error("must not be called"); } };
```

Every `DocumentState` literal in the unit tests gains `selection: null` (one line each; `crop-seed.test.ts` shown, the other seven are the same edit on their literal):

```diff
--- a/app/tests/unit/crop-seed.test.ts
+++ b/app/tests/unit/crop-seed.test.ts
@@ -9,3 +9,3 @@ import type { DocumentState } from "../../src/engine/types";
 function document(): DocumentState {
-  return { id: "D", documentId: "D", width: 120, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers: [] };
+  return { id: "D", documentId: "D", width: 120, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers: [] };
 }
```

```diff
--- a/app/tests/unit/adjust-store.test.ts
+++ b/app/tests/unit/adjust-store.test.ts
@@ -18,3 +18,3 @@ function document(layers: LayerState[], active: string): DocumentState {
   return { id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: active, canUndo: false,
-    canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers };
+    canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers };
 }
```

```diff
--- a/app/tests/unit/commit-transform.test.ts
+++ b/app/tests/unit/commit-transform.test.ts
@@ -16,3 +16,3 @@ function layer(): LayerState {
 function document(): DocumentState {
-  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers: [layer()] };
+  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers: [layer()] };
 }
```

```diff
--- a/app/tests/unit/crop-tool.test.ts
+++ b/app/tests/unit/crop-tool.test.ts
@@ -6,3 +6,3 @@ import type { DocumentState } from "../../src/engine/types";
 const doc: DocumentState = {
-  id: "D", documentId: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [],
+  id: "D", documentId: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null,
   layers: [{ id: "L", name: "Red", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
```

```diff
--- a/app/tests/unit/layer-rows.test.ts
+++ b/app/tests/unit/layer-rows.test.ts
@@ -8,3 +8,3 @@ function layer(id: string, o: Partial<LayerState> = {}): LayerState {
 }
-const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers });
+const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers });
 
```

```diff
--- a/app/tests/unit/prefilter.test.ts
+++ b/app/tests/unit/prefilter.test.ts
@@ -14,3 +14,3 @@ function layer(id: string, over: Partial<LayerState> = {}): LayerState {
 function document(layers: LayerState[]): DocumentState {
-  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers };
+  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers };
 }
```

```diff
--- a/app/tests/unit/selection.test.ts
+++ b/app/tests/unit/selection.test.ts
@@ -8,3 +8,3 @@ function layer(id: string, o: Partial<LayerState> = {}): LayerState {
 }
-const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], layers });
+const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers });
 
```

```diff
--- a/app/tests/unit/store-history.test.ts
+++ b/app/tests/unit/store-history.test.ts
@@ -16,3 +16,3 @@ function layer(id: string, over: Partial<LayerState> = {}): LayerState {
 function document(id: string, layers: LayerState[], activeLayerId: string | null, undoDepth = 0): DocumentState {
-  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, undoDepth, path: null, guides: [], undrawn: [], layers };
+  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, undoDepth, path: null, guides: [], undrawn: [], selection: null, layers };
 }
```

- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test -- engine-client`: `selectionOutline` is not a function. `pnpm build` fails in `tsc -p app/tests/unit` (the literals name a `selection` that `DocumentState` does not have yet).

- [ ] **Step 3: The bridge**

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -102,6 +102,14 @@ impl WasmEngine {
         let request = match request_json { Some(j) => Some(serde_json::from_str::<PreviewRequest>(&j).map_err(js_err)?), None => None };
         serde_json::to_string(&self.engine.set_preview(parse_id(doc)?, request).map_err(js_err)?).map_err(js_err)
     }
+    /// `Engine::selection_outline`: the marching ants' outline, flat (contour count, then each
+    /// contour's point count and x, y pairs), traced coarser when `step` < 1 and it is very detailed.
+    pub fn selection_outline(&self, doc: &str, step: f64) -> Result<js_sys::Float64Array, JsError> {
+        Ok(js_sys::Float64Array::from(self.engine.selection_outline(parse_id(doc)?, step).map_err(js_err)?.as_slice()))
+    }
+    pub fn selection_contains(&self, doc: &str, x: f64, y: f64) -> Result<bool, JsError> {
+        self.engine.selection_contains(parse_id(doc)?, Point { x, y }).map_err(js_err)
+    }
     pub fn histogram(&self, doc: &str, layer: &str) -> Result<String, JsError> {
         serde_json::to_string(&self.engine.histogram(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?).map_err(js_err)
     }
```

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -122,6 +122,23 @@ export interface DocumentState {
   guides: Guide[];
   /** What this project contains that this build does not draw yet (`Document::undrawn`). */
   undrawn: string[];
+  /** The selection's summary; null when nothing is selected. Its outline is fetched with
+   * `EngineClient.selectionOutline` whenever `revision` changes. */
+  selection: SelectionState | null;
+}
+
+/** How a new outline meets the selection (engine `SelectionMode`). */
+export type SelectionMode = "Replace" | "Add" | "Subtract";
+/** The drawn outline's kind (engine `SelectionShape`): the Lasso's two and the Marquee's two. */
+export type SelectionShape = "Freehand" | "Polygonal" | "Rectangle" | "Ellipse";
+/** The Magic Wand's options (engine `WandSettings`): tolerance 0-255, sample radius 0 (point),
+ * 1 (3x3) or 2 (5x5). */
+export interface WandSettings { tolerance: number; sampleRadius: number; contiguous: boolean; allLayers: boolean; }
+export const DEFAULT_WAND: WandSettings = { tolerance: 32, sampleRadius: 0, contiguous: true, allLayers: false };
+/** Engine `SelectionState`. `empty`: an explicit empty selection, which every edit refuses. */
+export interface SelectionState {
+  revision: number; empty: boolean; bounds: { x: number; y: number; width: number; height: number } | null;
+  antialiased: boolean; feather: number; points: number;
 }
 
 export type Command =
@@ -168,7 +185,20 @@ export type Command =
   | { type: "InvertPixels"; id: string; mask: boolean }
   | { type: "ApplyFilter"; id: string; params: FilterParams }
   | { type: "AddAdjustmentLayer"; kind: AdjustmentKind; seed: number; shadows: [number, number, number] | null; highlights: [number, number, number] | null }
-  | { type: "SetAdjustment"; id: string; adjustment: LayerAdjustment };
+  | { type: "SetAdjustment"; id: string; adjustment: LayerAdjustment }
+  | { type: "SelectShape"; kind: SelectionShape; points: PointTuple[]; mode: SelectionMode; antialiased: boolean }
+  | { type: "SelectAll" }
+  | { type: "Deselect" }
+  | { type: "InvertSelection" }
+  | { type: "MoveSelection"; dx: number; dy: number }
+  | { type: "ExpandSelection"; amount: number }
+  | { type: "ContractSelection"; amount: number }
+  | { type: "FeatherSelection"; amount: number }
+  | { type: "MagicWand"; at: PointTuple; mode: SelectionMode; settings: WandSettings; antialiased: boolean }
+  | { type: "LoadLayerSelection"; id: string; mode: SelectionMode; antialiased: boolean }
+  | { type: "LoadMaskSelection"; id: string; mode: SelectionMode; antialiased: boolean }
+  | { type: "ClearSelectedPixels"; id: string; mask: boolean }
+  | { type: "AddMaskFromSelection"; id: string; revealing: boolean };
 
 export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }
 
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -1,5 +1,5 @@
 import init, { WasmEngine } from "./pkg/compositor_engine.js";
-import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";
+import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PointTuple, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";
 
 export class EngineClient {
   private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory) {}
@@ -101,6 +101,23 @@ export class EngineClient {
   setPreview(doc: string, request: PreviewRequest | null): Dirty {
     return JSON.parse(this.wasm.set_preview(doc, request ? JSON.stringify(request) : undefined)) as Dirty;
   }
+  /** The selection's outline for the marching ants, in document pixels: one array of points per
+   * contour; empty with no selection. `step` is screen pixels per document pixel, a power of two:
+   * below 1 a very detailed outline comes back traced at that resolution (engine `selection_lod`). */
+  selectionOutline(doc: string, step: number): PointTuple[][] {
+    const flat = this.wasm.selection_outline(doc, step);
+    const contours: PointTuple[][] = [];
+    let i = 1;
+    for (let c = 0; c < (flat[0] ?? 0); c++) {
+      const n = flat[i++];
+      const contour: PointTuple[] = [];
+      for (let k = 0; k < n; k++, i += 2) contour.push([flat[i], flat[i + 1]]);
+      contours.push(contour);
+    }
+    return contours;
+  }
+  /** Whether `at` lies inside a selection with something in it (engine `selection_contains`). */
+  selectionContains(doc: string, at: { x: number; y: number }): boolean { return this.wasm.selection_contains(doc, at.x, at.y); }
   /** Four arrays of 256 bins: the mean of the channels, then red, green and blue. */
   histogram(doc: string, layer: string): number[][] { return JSON.parse(this.wasm.histogram(doc, layer)) as number[][]; }
   /** Auto Levels from `histogram`'s bins, which the panel already holds: nothing is recomposited. */
```

- [ ] **Step 4: Run the tests and watch them pass; measure the wasm**

`pnpm wasm:dev`, `pnpm build` (clean), `pnpm test` (115 (+1)). Then `pnpm wasm` and read the size of `app/src/engine/pkg/compositor_engine_bg.wasm`: the scratch copy measured 2,542,579 bytes here (2,210,211 at the base; i_overlay and the selection code, ruling OQ1). Record it in the task report and go back to `pnpm wasm:dev`.

- [ ] **Step 5: Prove it bites**

In `selectionOutline`, start reading at `let i = 0` instead of `1`: the new engine-client test fails. Restore.

- [ ] **Step 6: Commit**

```
git commit -- engine-wasm/src/lib.rs app/src/engine/types.ts app/src/engine/client.ts app/tests/unit/engine-client.test.ts app/tests/unit/adjust-store.test.ts app/tests/unit/commit-transform.test.ts app/tests/unit/crop-seed.test.ts app/tests/unit/crop-tool.test.ts app/tests/unit/layer-rows.test.ts app/tests/unit/prefilter.test.ts app/tests/unit/selection.test.ts app/tests/unit/store-history.test.ts -m "feat: the selection's commands, summary and outline across the wasm bridge" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: Drawing an outline: the Marquee's box, the Lasso's points, the modes

The canvas's gestures reduce to a few pure rules, ported from the Mac so they can be tested without a canvas: the mode from Shift and Alt (`selectionMode`), the Marquee's whole-pixel box (`DragBox.rect`) with Shift squaring only once pressed afresh during the drag, the Freehand Lasso's quarter-pixel step, the Polygonal Lasso's corners, Backspace, and its closing click, and the whole-pixel offset of a dragged outline with Shift keeping one axis (Selection.swift:91-231; EditorCanvas.swift:1932-2002). A finished draft is one `SelectShape`; what the engine then does with it is Task 4's.

**Files:**
- Create: `app/src/tools/selection-draft.ts`
- Create test: `app/tests/unit/selection-draft.test.ts`

**Interfaces:**
- Consumes: Task 8's `Command`, `SelectionMode`, `SelectionShape`, `PointTuple`.
- Produces: `SelectionTool`, `MarqueeKind`, `LassoKind`, `P`, `Box`; `isSelectionTool(tool)`, `selectionMode(choice, shift, alt)`, `dragBox(anchor, point, square, fromCenter)`, `CLOSE_RADIUS_PX = 8`, `MIN_STEP = 0.25`; `class SelectionDraft` (`begin(kind, mode, at, shiftAtPress)`, `isMarquee`, `dragMarquee(pixel, shift)`, `extend(pixel)`, `moveCursor(pixel)`, `removeLast(): boolean`, `click(pixel, view, firstInView, clickCount): "close" | "extend"`, `finish(antialiased): Command`); `outlineOffset(start, pixel, shift)`.

- [ ] **Step 1: Write the failing tests**

Ported from SelectionTests with its points and boxes:

```ts
import { describe, expect, it } from "vitest";
import { dragBox, isSelectionTool, outlineOffset, SelectionDraft, selectionMode } from "../../src/tools/selection-draft";

// Ported from Compositor for Mac 1.2.10's CompositorTests/SelectionTests.swift: the gestures the
// canvas turns into one SelectShape command. What the command then does is engine-tested
// (engine/tests/selection_commands.rs).
const box = (d: SelectionDraft) => {
  const xs = d.points.map((p) => p.x), ys = d.points.map((p) => p.y);
  return { x: Math.min(...xs), y: Math.min(...ys), width: Math.max(...xs) - Math.min(...xs), height: Math.max(...ys) - Math.min(...ys) };
};

describe("selection modes", () => {
  it("Shift adds, Alt subtracts with or without Shift, else the options bar's choice (modifiersPickModeAndSelectionIsClippedToCanvas)", () => {
    expect(selectionMode("Replace", false, false)).toBe("Replace");
    expect(selectionMode("Replace", true, false)).toBe("Add");
    expect(selectionMode("Replace", true, true)).toBe("Subtract");
    expect(selectionMode("Replace", false, true)).toBe("Subtract");
    expect(selectionMode("Add", false, false)).toBe("Add");
    expect(isSelectionTool("marquee") && isSelectionTool("lasso") && isSelectionTool("wand") && !isSelectionTool("move")).toBe(true);
  });
});

describe("the Marquee", () => {
  it("draws whole-pixel rectangles in any direction (marqueeDrawsWholePixelRectanglesInAnyDirection)", () => {
    const d = SelectionDraft.begin("Rectangle", "Replace", { x: 60.4, y: 70.6 });
    d.dragMarquee({ x: 20.2, y: 30.3 }, false);
    expect(box(d)).toEqual({ x: 20, y: 30, width: 40, height: 41 });
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Rectangle", mode: "Replace", antialiased: true,
      points: [[20, 30], [60, 30], [60, 71], [20, 71]] });
  });

  it("squares with Shift and grows from the anchor when centred (marqueeShiftMakesSquaresAndCenteredDragsGrowFromTheAnchor)", () => {
    expect(dragBox({ x: 10, y: 10 }, { x: 40, y: 20 }, true, false)).toEqual({ x: 10, y: 10, width: 30, height: 30 });
    expect(dragBox({ x: 50, y: 50 }, { x: 60, y: 55 }, false, true)).toEqual({ x: 40, y: 45, width: 20, height: 10 });
    expect(dragBox({ x: 50, y: 50 }, { x: 45, y: 58 }, true, true)).toEqual({ x: 42, y: 42, width: 16, height: 16 });
  });

  it("an Alt drag subtracts and never grows from the centre (optionDraggingTheMarqueeSubtractsWithoutDrawingFromTheCenter)", () => {
    const d = SelectionDraft.begin("Rectangle", selectionMode("Replace", false, true), { x: 40, y: 40 });
    d.dragMarquee({ x: 60, y: 60 }, false);
    expect(d.mode).toBe("Subtract");
    expect(box(d)).toEqual({ x: 40, y: 40, width: 20, height: 20 });
  });

  it("a Shift held at the press adds; only a fresh Shift squares (shiftStartsAnAddAndOnlyAFreshShiftSquaresTheMarquee)", () => {
    const held = SelectionDraft.begin("Rectangle", selectionMode("Replace", true, false), { x: 40, y: 40 }, true);
    held.dragMarquee({ x: 70, y: 50 }, true);
    expect(held.mode).toBe("Add");
    expect(box(held)).toEqual({ x: 40, y: 40, width: 30, height: 10 });
    const fresh = SelectionDraft.begin("Rectangle", "Add", { x: 20, y: 60 }, true);
    fresh.dragMarquee({ x: 30, y: 65 }, true);
    fresh.dragMarquee({ x: 35, y: 68 }, false);
    fresh.dragMarquee({ x: 40, y: 70 }, true);
    expect(box(fresh)).toEqual({ x: 20, y: 60, width: 20, height: 20 });
  });

  it("an Ellipse sends its box's corners for the engine to fill with the oval", () => {
    const d = SelectionDraft.begin("Ellipse", "Replace", { x: 10, y: 20 });
    d.dragMarquee({ x: 70, y: 60 }, false);
    expect(d.finish(false)).toEqual({ type: "SelectShape", kind: "Ellipse", mode: "Replace", antialiased: false,
      points: [[10, 20], [70, 20], [70, 60], [10, 60]] });
  });
});

describe("the Lasso", () => {
  it("skips freehand points closer than a quarter pixel", () => {
    const d = SelectionDraft.begin("Freehand", "Replace", { x: 10, y: 10 });
    d.extend({ x: 10.1, y: 10.1 });
    d.extend({ x: 10.3, y: 10 });
    expect(d.points).toEqual([{ x: 10, y: 10 }, { x: 10.3, y: 10 }]);
  });

  it("drops a misplaced polygonal corner and closes on the first corner (polygonalCornersCanBeRemovedAndClosed)", () => {
    const d = SelectionDraft.begin("Polygonal", "Replace", { x: 10, y: 10 });
    const view = (p: { x: number; y: number }) => ({ x: p.x * 2, y: p.y * 2 });
    const click = (x: number, y: number, count = 1) => d.click({ x, y }, view({ x, y }), view(d.points[0]), count);
    expect(click(90, 10)).toBe("extend");
    expect(click(50, 50)).toBe("extend");
    expect(d.removeLast()).toBe(true);
    click(90, 90); click(10, 90);
    expect(d.points.length).toBe(4);
    expect(click(13, 13), "4.2 document px from the first corner is 8.5 view px at 2x: another corner").toBe("extend");
    d.removeLast();
    expect(click(12, 12), "5.7 view px: close").toBe("close");
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Polygonal", mode: "Replace", antialiased: true,
      points: [[10, 10], [90, 10], [90, 90], [10, 90]] });
  });

  it("a double-click closes, and Backspace on the only corner drops the draft", () => {
    const d = SelectionDraft.begin("Polygonal", "Add", { x: 0, y: 0 });
    expect(d.click({ x: 50, y: 0 }, { x: 50, y: 0 }, { x: 0, y: 0 }, 2)).toBe("close");
    const single = SelectionDraft.begin("Polygonal", "Replace", { x: 1, y: 1 });
    expect(single.removeLast()).toBe(false);
  });

  it("a click with no drag still sends its one point: the engine deselects (clickDeselectsAndSelectionStepsUndo)", () => {
    const d = SelectionDraft.begin("Freehand", "Replace", { x: 5, y: 5 });
    expect(d.finish(true)).toEqual({ type: "SelectShape", kind: "Freehand", mode: "Replace", antialiased: true, points: [[5, 5]] });
  });
});

describe("moving the outline", () => {
  it("moves in whole pixels, and Shift keeps to one axis (draggingMovesTheOutlineInWholePixelsAsOneUndo)", () => {
    expect(outlineOffset({ x: 20, y: 20 }, { x: 30.4, y: 49.6 }, false)).toEqual({ dx: 10, dy: 30 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 60.2, y: 60.4 }, false)).toEqual({ dx: 40, dy: 40 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 35, y: 26 }, true)).toEqual({ dx: 15, dy: 0 });
    expect(outlineOffset({ x: 20, y: 20 }, { x: 26, y: 5 }, true)).toEqual({ dx: 0, dy: -15 });
  });
});
```

- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test -- selection-draft`: the module does not exist.

- [ ] **Step 3: The drawing rules**

```ts
import type { Command, PointTuple, SelectionMode, SelectionShape } from "../engine/types";

/** The three selection tools (Phase 4a): the Marquee (M), the Lasso (L) and the Magic Wand (W). */
export type SelectionTool = "marquee" | "lasso" | "wand";
export type MarqueeKind = "Rectangle" | "Ellipse";
export type LassoKind = "Freehand" | "Polygonal";
export interface P { x: number; y: number; }
export interface Box { x: number; y: number; width: number; height: number; }

export function isSelectionTool(tool: string): tool is SelectionTool { return tool === "marquee" || tool === "lasso" || tool === "wand"; }

/** Alt (with or without Shift) subtracts, Shift adds, otherwise the options bar's choice
 * (`selectionMode(shift:option:)`, Selection.swift:123-125). Ctrl is the Mac's Cmd, Alt its Option. */
export function selectionMode(choice: SelectionMode, shift: boolean, alt: boolean): SelectionMode {
  return alt ? "Subtract" : shift ? "Add" : choice;
}

/** The box a drag from `anchor` to `point` spans, in whole pixels (`DragBox.rect`, Selection.swift:93-105):
 * `square` evens the sides, `fromCenter` grows it around the anchor. The anchor is already whole. */
export function dragBox(anchor: P, point: P, square: boolean, fromCenter: boolean): Box {
  let dx = Math.round(point.x) - anchor.x, dy = Math.round(point.y) - anchor.y;
  if (square) {
    const side = Math.max(Math.abs(dx), Math.abs(dy));
    dx = dx < 0 ? -side : side;
    dy = dy < 0 ? -side : side;
  }
  return fromCenter
    ? { x: anchor.x - Math.abs(dx), y: anchor.y - Math.abs(dy), width: Math.abs(dx) * 2, height: Math.abs(dy) * 2 }
    : { x: Math.min(anchor.x, anchor.x + dx), y: Math.min(anchor.y, anchor.y + dy), width: Math.abs(dx), height: Math.abs(dy) };
}

/** A click this close (view px) to a polygon's first corner closes it (EditorCanvas.swift:1996). */
export const CLOSE_RADIUS_PX = 8;
/** Freehand points closer than this (document px) to the last one are skipped (Selection.swift:164-167). */
export const MIN_STEP = 0.25;

/** An outline being drawn with the Marquee or the Lasso (`LassoDraft`, Selection.swift:107-116), in
 * document pixels. Its mode is fixed at the press; `cursor` is the Polygonal Lasso's rubber band. */
export class SelectionDraft {
  points: P[];
  cursor: P | null = null;
  /** The Marquee's whole-pixel starting corner. */
  readonly anchor: P | null;
  /** Shift squares the Marquee only once pressed afresh: a Shift held at the press chose Add
   * (EditorCanvas.swift:1949-1956, :1963-1964). */
  private squareArmed: boolean;
  private constructor(readonly kind: SelectionShape, readonly mode: SelectionMode, at: P, shiftAtPress: boolean) {
    const marquee = kind === "Rectangle" || kind === "Ellipse";
    this.anchor = marquee ? { x: Math.round(at.x), y: Math.round(at.y) } : null;
    this.points = [this.anchor ?? at];
    this.squareArmed = !shiftAtPress;
  }
  static begin(kind: SelectionShape, mode: SelectionMode, at: P, shiftAtPress = false): SelectionDraft {
    return new SelectionDraft(kind, mode, at, shiftAtPress);
  }
  get isMarquee(): boolean { return this.anchor !== null; }
  /** The Marquee's box to `pixel`: the four corners of a whole-pixel box (`dragMarquee`). Never from
   * the centre: Alt subtracts instead (EditorCanvas.swift:1949-1956). */
  dragMarquee(pixel: P, shift: boolean): void {
    if (!this.anchor || !Number.isFinite(pixel.x) || !Number.isFinite(pixel.y)) return;
    if (!shift) this.squareArmed = true;
    const b = dragBox(this.anchor, pixel, this.squareArmed && shift, false);
    this.points = [{ x: b.x, y: b.y }, { x: b.x + b.width, y: b.y }, { x: b.x + b.width, y: b.y + b.height }, { x: b.x, y: b.y + b.height }];
  }
  /** A Freehand point, or a Polygonal corner (`extendLasso`). */
  extend(pixel: P): void {
    if (!Number.isFinite(pixel.x) || !Number.isFinite(pixel.y)) return;
    const last = this.points[this.points.length - 1];
    if (last && Math.hypot(pixel.x - last.x, pixel.y - last.y) < MIN_STEP) return;
    this.points.push(pixel);
  }
  moveCursor(pixel: P | null): void { this.cursor = pixel; }
  /** Backspace drops the last corner; false when none is left, and the draft goes (`removeLastLassoPoint`). */
  removeLast(): boolean { this.points.pop(); return this.points.length > 0; }
  /** A Polygonal click: a double-click, or a click within `CLOSE_RADIUS_PX` of the first corner with
   * three corners down, closes; anything else adds a corner. `firstInView` is the first corner on screen. */
  click(pixel: P, view: P, firstInView: P, clickCount: number): "close" | "extend" {
    if (clickCount >= 2 || (this.points.length >= 3 && Math.hypot(view.x - firstInView.x, view.y - firstInView.y) <= CLOSE_RADIUS_PX)) return "close";
    this.extend(pixel);
    return "extend";
  }
  /** The command that closes the outline (`finishLasso`): the engine deselects in Replace when it
   * encloses nothing, and changes nothing otherwise. */
  finish(antialiased: boolean): Command {
    const points = this.points.map((p) => [p.x, p.y] as PointTuple);
    return { type: "SelectShape", kind: this.kind, points, mode: this.mode, antialiased };
  }
}

/** Dragging the outline itself (`dragSelection`, EditorCanvas.swift:1932-1942): the offset from the
 * press, whole pixels; Shift keeps it on the axis the drag has gone further along. */
export function outlineOffset(start: P, pixel: P, shift: boolean): { dx: number; dy: number } {
  let dx = pixel.x - start.x, dy = pixel.y - start.y;
  if (shift) { if (Math.abs(dx) >= Math.abs(dy)) dy = 0; else dx = 0; }
  return { dx: Math.round(dx) + 0, dy: Math.round(dy) + 0 };
}
```

- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test -- selection-draft` (11 tests), `pnpm test` (126 (+11)), `pnpm build`.

- [ ] **Step 5: Prove it bites**

(1) In `dragBox`, drop the `Math.round` of the point: the whole-pixel marquee test fails (the box starts at 20.2). Restore. (2) Start `squareArmed` at `true` whatever Shift was at the press: the fresh-Shift test fails (the held Shift squares the box). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/tools/selection-draft.ts app/tests/unit/selection-draft.test.ts
git commit -- app/src/tools/selection-draft.ts app/tests/unit/selection-draft.test.ts -m "feat: the Marquee's and the Lasso's drawing rules, from the Mac's" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: The store: the tools, their options, the draft, Delete and Add Mask

**Files:**
- Modify: `app/src/state/store.ts`, `app/src/actions/layers.ts`, `app/src/tools/crop-tool.ts`
- Modify test: `app/tests/unit/crop-seed.test.ts`
- Create test: `app/tests/unit/selection-store.test.ts`

The store gains the three tools, the options the Mac keeps per session (the mode, Anti-alias, the Marquee's and the Lasso's kinds, the Magic Wand's settings, the Expand / Contract / Feather amounts, with the Mac's defaults 1, 1 and 2: EditorSession.swift:232-285), the draft, the dragged outline's offset, and `overlayTick`, which repaints the overlay without re-rendering the picture. Changing tool or kind drops a draft (`cancelLasso`). `canAdjust` refuses an empty selection. The Crop tool starts at the selection's bounds, rounded out and cut to the canvas (EditorSession.swift:349-358). Delete clears through a selection and deletes layers without one; Add Mask takes the selection; Ctrl-click loads a thumbnail (rulings OQ12, OQ13).

**Interfaces:**
- Consumes: Tasks 8 and 9.
- Produces: `Tool` adds `"marquee" | "lasso" | "wand"`; `SelectionAmountOperation`; `Sheet` adds `{ kind: "selectionAmount"; operation }`; `SelectionOptions`, `DEFAULT_SELECTION_OPTIONS`, `SELECTION_AMOUNT_MAX`; store fields `selectionOptions`, `selectionDraft`, `outlineMove`, `heldSelectionMode`, `overlayTick`; actions `repaintOverlay`, `setSelectionOptions`, `setSelectionDraft`, `finishSelectionDraft`, `setOutlineMove`, `setHeldSelectionMode`, `modifySelection(operation, amount): boolean`, `cycleToolMode`, `hasSelection`; `cropSeed(state)`; `actions/layers` `canClearSelected`, `deleteKeyPressed`, `loadSelection(id, mask, mode)`, and `addMaskToActive` and `canInvert` aware of the selection.

- [ ] **Step 1: Write the failing tests**

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_SELECTION_OPTIONS, useEditor } from "../../src/state/store";
import { addMaskToActive, canInvert, deleteKeyPressed, loadSelection } from "../../src/actions/layers";
import { SelectionDraft } from "../../src/tools/selection-draft";
import type { Command, DocumentState, LayerState, LayerTransform, SelectionState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The store's selection routing (Phase 4a), after Compositor for Mac 1.2.10: Delete clears through a
// selection (SelectionEdits.swift:60-63), Add Mask takes it (LayerMask.swift:233-260), an empty one
// refuses every edit (canAdjustColors / canInvert / canPaint), Tab switches the tool's kind.
const box: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
function layer(id: string, over: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: box,
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
    maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...over };
}
const selected = (empty = false): SelectionState => ({ revision: 7, empty, bounds: empty ? null : { x: 2, y: 2, width: 5, height: 5 }, antialiased: true, feather: 0, points: 4 });
function document(layers: LayerState[], selection: SelectionState | null): DocumentState {
  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: layers[0]?.id ?? null, canUndo: false, canRedo: false,
    isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection, layers };
}
function stub(doc: DocumentState) {
  const commands: Command[] = [];
  const engine = { state: () => doc, clipDependents: () => [],
    execute: (_id: string, c: Command) => { commands.push(c); return { structure: true, canvas: false, layers: [] }; } } as unknown as EngineClient;
  useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: doc.activeLayerId ? [doc.activeLayerId] : [] });
  return commands;
}

beforeEach(() => {
  useEditor.setState({ engine: null, documents: {}, order: [], activeId: null, viewports: {}, transformEdit: null, selectedLayerIds: [], maskSelected: false,
    error: null, tool: "move", sheet: null, adjustEdit: null, selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null });
});

describe("Delete", () => {
  it("clears the selected pixels with a selection, and deletes the layer without one", () => {
    let commands = stub(document([layer("A")], selected()));
    deleteKeyPressed();
    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: false }]);
    commands = stub(document([layer("A")], null));
    deleteKeyPressed();
    expect(commands.map((c) => c.type)).toEqual(["DeleteLayers"]);
  });

  it("fills the targeted mask, and does nothing on an empty selection, a hidden layer or a disabled mask", () => {
    let commands = stub(document([layer("A", { hasMask: true })], selected()));
    useEditor.setState({ maskSelected: true });
    deleteKeyPressed();
    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: true }]);
    for (const doc of [document([layer("A")], selected(true)), document([layer("A", { visible: false })], selected()),
      document([layer("A", { hasMask: true, maskEnabled: false })], selected())]) {
      commands = stub(doc);
      useEditor.setState({ maskSelected: doc.layers[0].hasMask });
      deleteKeyPressed();
      expect(commands).toEqual([]);
    }
  });
});

describe("Add Mask", () => {
  it("uses the selection when there is one", () => {
    let commands = stub(document([layer("A")], selected()));
    addMaskToActive(false);
    expect(commands).toEqual([{ type: "AddMaskFromSelection", id: "A", revealing: false }]);
    commands = stub(document([layer("A")], null));
    addMaskToActive(true);
    expect(commands).toEqual([{ type: "AddMask", id: "A", revealing: true }]);
  });
});

describe("an empty selection refuses every edit", () => {
  it("greys out Levels, the filters and Invert", () => {
    stub(document([layer("A")], selected(true)));
    expect(useEditor.getState().canAdjust()).toBe(false);
    expect(canInvert()).toBe(false);
    stub(document([layer("A")], selected()));
    expect(useEditor.getState().canAdjust()).toBe(true);
    expect(canInvert()).toBe(true);
  });
});

describe("selection tool state", () => {
  it("Expand / Contract / Feather check their range and remember the amount", () => {
    const commands = stub(document([layer("A")], selected()));
    expect(useEditor.getState().modifySelection("Feather", 251)).toBe(false);
    expect(useEditor.getState().modifySelection("Expand", 0)).toBe(false);
    expect(useEditor.getState().modifySelection("Contract", 500)).toBe(true);
    expect(useEditor.getState().modifySelection("Feather", 12)).toBe(true);
    expect(commands).toEqual([{ type: "ContractSelection", amount: 500 }, { type: "FeatherSelection", amount: 12 }]);
    expect(useEditor.getState().selectionOptions).toMatchObject({ contract: 500, feather: 12, expand: 1 });
  });

  it("Tab switches the Marquee's and the Lasso's kind and drops a draft; changing tool drops it too", () => {
    stub(document([layer("A")], null));
    useEditor.setState({ tool: "marquee", selectionDraft: SelectionDraft.begin("Rectangle", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().cycleToolMode();
    expect(useEditor.getState().selectionOptions.marquee).toBe("Ellipse");
    expect(useEditor.getState().selectionDraft).toBeNull();
    useEditor.setState({ tool: "lasso", selectionDraft: SelectionDraft.begin("Freehand", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().cycleToolMode();
    expect(useEditor.getState().selectionOptions.lasso).toBe("Polygonal");
    useEditor.setState({ selectionDraft: SelectionDraft.begin("Polygonal", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().setTool("move");
    expect(useEditor.getState().selectionDraft).toBeNull();
  });

  it("finishing a draft sends one SelectShape with the Anti-alias setting", () => {
    const commands = stub(document([layer("A")], null));
    useEditor.getState().setSelectionOptions({ antialiased: false });
    const draft = SelectionDraft.begin("Freehand", "Add", { x: 1, y: 1 });
    draft.extend({ x: 9, y: 1 }); draft.extend({ x: 9, y: 9 });
    useEditor.setState({ selectionDraft: draft });
    useEditor.getState().finishSelectionDraft();
    expect(commands).toEqual([{ type: "SelectShape", kind: "Freehand", mode: "Add", antialiased: false, points: [[1, 1], [9, 1], [9, 9]] }]);
    expect(useEditor.getState().selectionDraft).toBeNull();
  });

  it("Ctrl-clicking a thumbnail loads its pixels or its mask's black areas", () => {
    const commands = stub(document([layer("A", { hasMask: true })], null));
    loadSelection("A", false, "Add");
    loadSelection("A", true);
    expect(commands).toEqual([{ type: "LoadLayerSelection", id: "A", mode: "Add", antialiased: true }, { type: "LoadMaskSelection", id: "A", mode: "Replace", antialiased: true }]);
  });
});
```

```diff
--- a/app/tests/unit/crop-seed.test.ts
+++ b/app/tests/unit/crop-seed.test.ts
@@ -25,6 +25,17 @@ describe("crop tool rectangle seeding", () => {
     expect(useEditor.getState().cropRect).toEqual({ x: 10, y: 10, width: 50, height: 40 });
   });
 
+  it("seeds the selection's bounds, rounded out and cut to the canvas (EditorSession.swift:349-358)", () => {
+    const selected = { ...document(), selection: { revision: 5, empty: false, bounds: { x: -3.5, y: 10.25, width: 50, height: 20.5 }, antialiased: true, feather: 0, points: 4 } };
+    useEditor.setState({ engine: null, activeId: "D", documents: { D: selected }, tool: "marquee", cropRect: null, transformEdit: null });
+    useEditor.getState().setTool("crop");
+    expect(useEditor.getState().cropRect).toEqual({ x: 0, y: 10, width: 47, height: 21 });
+    const empty = { ...selected, selection: { ...selected.selection, empty: true } };
+    useEditor.setState({ documents: { D: empty }, tool: "marquee", cropRect: null });
+    useEditor.getState().setTool("crop");
+    expect(useEditor.getState().cropRect, "an empty selection seeds the canvas").toEqual({ x: 0, y: 0, width: 120, height: 80 });
+  });
+
   it("seeds nothing without a document", () => {
     useEditor.setState({ engine: null, activeId: null, documents: {}, tool: "move", cropRect: null, transformEdit: null });
     useEditor.getState().setTool("crop");
```

- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test -- selection-store crop-seed`: the store has no `modifySelection`, `cycleToolMode`, `finishSelectionDraft` and no selection fields; `actions/layers` has no `deleteKeyPressed` or `loadSelection`; the crop seeds the canvas.

- [ ] **Step 3: The store and the actions**

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -1,6 +1,9 @@
 import { create } from "zustand";
-import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, LevelsAuto, PreviewEdit } from "../engine/types";
+import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, LevelsAuto, PreviewEdit, SelectionMode, WandSettings } from "../engine/types";
+import { DEFAULT_WAND } from "../engine/types";
 import type { EngineClient } from "../engine/client";
+import { cropSeed } from "../tools/crop-tool";
+import type { LassoKind, MarqueeKind, SelectionDraft } from "../tools/selection-draft";
 import type { ShellBridge } from "../shell/bridge";
 import { Viewport } from "../canvas/viewport";
 import type { Rect } from "../tools/crop-geometry";
@@ -10,9 +13,25 @@ import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
 
-export type Tool = "move" | "hand" | "zoom" | "crop";
+export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
-export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" };
+/** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
+export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
+export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" }
+  | { kind: "selectionAmount"; operation: SelectionAmountOperation };
+
+/** The selection tools' settings, kept per app session as the Mac keeps them per session
+ * (EditorSession.swift:232-285): the options bar's mode, Anti-alias, the Marquee's and the Lasso's
+ * kinds, the Magic Wand's settings, and the Expand / Contract / Feather amounts. */
+export interface SelectionOptions {
+  mode: SelectionMode; antialiased: boolean; marquee: MarqueeKind; lasso: LassoKind; wand: WandSettings;
+  expand: number; contract: number; feather: number;
+}
+export const DEFAULT_SELECTION_OPTIONS: SelectionOptions = {
+  mode: "Replace", antialiased: true, marquee: "Rectangle", lasso: "Freehand", wand: DEFAULT_WAND, expand: 1, contract: 1, feather: 2,
+};
+/** The largest amount each operation takes (Selection.swift:307). */
+export const SELECTION_AMOUNT_MAX: Record<SelectionAmountOperation, number> = { Expand: 500, Contract: 500, Feather: 250 };
 
 export interface TransformEdit {
   kind: "layer" | "group" | "mask";
@@ -46,6 +65,8 @@ export interface EditorStore {
   busy: boolean;
   rendererKind: "gl" | "cpu" | null;
   renderTick: number;
+  /** Bumped when only the overlay changes (an outline being drawn or dragged): no re-render of the picture. */
+  overlayTick: number;
   recentTick: number;
   selectedLayerIds: string[];
   maskSelected: boolean;
@@ -57,6 +78,13 @@ export interface EditorStore {
   showGuides: boolean;
   blendPreview: BlendMode | null;
   adjustEdit: AdjustEdit | null;
+  selectionOptions: SelectionOptions;
+  /** A Marquee or Lasso outline being drawn; never in the document until it is finished. */
+  selectionDraft: SelectionDraft | null;
+  /** The whole-pixel offset of the outline while it is being dragged; sent as one MoveSelection on release. */
+  outlineMove: { dx: number; dy: number } | null;
+  /** The mode Shift / Alt held over the canvas imply, for the options bar (`heldSelectionMode`). */
+  heldSelectionMode: SelectionMode | null;
   setEngine(engine: EngineClient): void;
   setBridge(bridge: ShellBridge): void;
   setBusy(busy: boolean): void;
@@ -78,6 +106,7 @@ export interface EditorStore {
   setError(error: string | null): void;
   setRendererKind(kind: "gl" | "cpu"): void;
   invalidate(): void;
+  repaintOverlay(): void;
   selectLayers(ids: string[], primary: string | null): void;
   setMaskSelected(v: boolean): void;
   toggleCollapsed(id: string): void;
@@ -111,6 +140,18 @@ export interface EditorStore {
   previewSettling(): boolean;
   commitAdjust(): void;
   cancelAdjust(): void;
+  setSelectionOptions(patch: Partial<SelectionOptions>): void;
+  setSelectionDraft(draft: SelectionDraft | null): void;
+  /** Sends the draft's outline to the engine as one SelectShape and drops the draft. */
+  finishSelectionDraft(): void;
+  setOutlineMove(offset: { dx: number; dy: number } | null): void;
+  setHeldSelectionMode(mode: SelectionMode | null): void;
+  /** Expand / Contract / Feather by `amount`, remembered as that operation's amount. False when out of range or refused. */
+  modifySelection(operation: SelectionAmountOperation, amount: number): boolean;
+  /** Tab: the Marquee's Rectangle / Ellipse, the Lasso's Freehand / Polygonal (`cycleToolMode`). */
+  cycleToolMode(): void;
+  /** Whether the document has a selection with something in it (`canModifySelection`, less the draft rule). */
+  hasSelection(): boolean;
 }
 
 /** Commands that insert a layer or move one into a folder, and so make it active somewhere the
@@ -146,10 +187,11 @@ function loadShowGuides(): boolean {
 
 export const useEditor = create<EditorStore>((set, get) => ({
   engine: null, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
-  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, recentTick: 0,
+  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, overlayTick: 0, recentTick: 0,
   selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, showGuides: loadShowGuides(),
   blendPreview: null,
   adjustEdit: null,
+  selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
   setEngine: (engine) => set({ engine }),
   setBridge: (bridge) => set({ bridge }),
   setBusy: (busy) => set({ busy }),
@@ -163,7 +205,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     const viewport = new Viewport();
     set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
       viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null,
-      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null }));
+      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null }));
   },
   closeDocument: (id) => {
     get().commitTransform();
@@ -177,7 +219,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
       const activeId = s.activeId === id ? order[order.length - 1] ?? null : s.activeId;
       const active = activeId ? documents[activeId] : null;
       return { documents, viewports, order, activeId, cropRect: null,
-        selectedLayerIds: active?.activeLayerId ? [active.activeLayerId] : [], maskSelected: false, transformEdit: null };
+        selectedLayerIds: active?.activeLayerId ? [active.activeLayerId] : [], maskSelected: false, transformEdit: null,
+        ...(s.activeId === id ? { selectionDraft: null, outlineMove: null } : {}) };
     });
   },
   setActive: (id) => {
@@ -186,7 +229,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     get().commitTransform();
     dropOpenPanel();
     const state = get().documents[id];
-    set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null });
+    set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null });
   },
   refresh: (id) => {
     const target = id ?? get().activeId;
@@ -245,13 +288,16 @@ export const useEditor = create<EditorStore>((set, get) => ({
   redo: () => { if (get().panelOwnsDocument()) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
   setTool: (tool) => {
     if (get().tool === "move" && tool !== "move") get().commitTransform();
-    // Entering the crop tool seeds a full-canvas rectangle, as macOS does (EditorSession.selectTool);
-    // the frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
+    // Entering the crop tool seeds a rectangle, as macOS does (EditorSession.selectTool): the
+    // selection's bounds when there is a selection with something in it, else the canvas (cropSeed).
+    // The frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
     // or Cancel clears it nothing is drawn until the user drags a new one. Leaving the tool clears it.
+    // Changing tool drops an outline being drawn (`cancelLasso`, EditorSession.swift:284).
     const { activeId, documents, cropRect } = get();
     const doc = activeId ? documents[activeId] : null;
-    const seeded = tool === "crop" ? (cropRect ?? (doc ? { x: 0, y: 0, width: doc.width, height: doc.height } : null)) : null;
-    set({ tool, cropRect: seeded });
+    const seeded = tool === "crop" ? (cropRect ?? (doc ? cropSeed(doc) : null)) : null;
+    set({ tool, cropRect: seeded, ...(tool !== get().tool ? { selectionDraft: null, outlineMove: null } : {}) });
+    get().invalidate();
   },
   setCropRect: (cropRect) => set({ cropRect }),
   setCropRatio: (cropRatio) => set({ cropRatio }),
@@ -262,6 +308,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
   setError: (error) => set({ error }),
   setRendererKind: (rendererKind) => set({ rendererKind }),
   invalidate: () => set((s) => ({ renderTick: s.renderTick + 1 })),
+  repaintOverlay: () => set((s) => ({ overlayTick: s.overlayTick + 1 })),
   selectLayers: (ids, primary) => {
     const { engine, activeId } = get(); if (!engine || !activeId) return;
     // `SetActiveLayer` records no history, but every `execute` clears the engine's preview, so a
@@ -394,7 +441,9 @@ export const useEditor = create<EditorStore>((set, get) => ({
     if (!activeId || get().panelOwnsDocument()) return false;
     const state = documents[activeId];
     const layer = activeLayer(state);
-    return !!layer && !layer.isGroup && layer.hasPixels && !maskSelected && selectedLayerIds.length === 1 && visibleIds(state).has(layer.id);
+    // An empty selection refuses every edit (`canAdjustColors`: `selection?.isEmpty != true`).
+    return !!layer && !layer.isGroup && layer.hasPixels && !maskSelected && selectedLayerIds.length === 1 && visibleIds(state).has(layer.id)
+      && state.selection?.empty !== true;
   },
   beginAdjust: ({ kind, layerId, target }) => {
     const { engine, activeId } = get(); if (!engine || !activeId) return false;
@@ -501,4 +550,38 @@ export const useEditor = create<EditorStore>((set, get) => ({
     get().refresh(activeId);
     get().invalidate();
   },
+  setSelectionOptions: (patch) => {
+    // Changing the Marquee's or the Lasso's kind drops an outline being drawn (LassoControls.swift:10-37).
+    const kindChanged = (patch.marquee !== undefined && patch.marquee !== get().selectionOptions.marquee)
+      || (patch.lasso !== undefined && patch.lasso !== get().selectionOptions.lasso);
+    set((s) => ({ selectionOptions: { ...s.selectionOptions, ...patch }, ...(kindChanged ? { selectionDraft: null } : {}) }));
+    if (kindChanged) get().repaintOverlay();
+  },
+  setSelectionDraft: (selectionDraft) => { set({ selectionDraft }); get().repaintOverlay(); },
+  finishSelectionDraft: () => {
+    const draft = get().selectionDraft; if (!draft) return;
+    set({ selectionDraft: null });
+    get().run(draft.finish(get().selectionOptions.antialiased));
+    get().invalidate();
+  },
+  setOutlineMove: (outlineMove) => { set({ outlineMove }); get().repaintOverlay(); },
+  setHeldSelectionMode: (heldSelectionMode) => { if (heldSelectionMode !== get().heldSelectionMode) set({ heldSelectionMode }); },
+  modifySelection: (operation, amount) => {
+    if (!Number.isInteger(amount) || amount < 1 || amount > SELECTION_AMOUNT_MAX[operation] || !get().hasSelection()) return false;
+    const key = operation === "Expand" ? "expand" : operation === "Contract" ? "contract" : "feather";
+    set((s) => ({ selectionOptions: { ...s.selectionOptions, [key]: amount } }));
+    const command: Command = operation === "Expand" ? { type: "ExpandSelection", amount }
+      : operation === "Contract" ? { type: "ContractSelection", amount } : { type: "FeatherSelection", amount };
+    return get().run(command);
+  },
+  cycleToolMode: () => {
+    const { tool, selectionOptions: o } = get();
+    if (tool === "marquee") get().setSelectionOptions({ marquee: o.marquee === "Rectangle" ? "Ellipse" : "Rectangle" });
+    else if (tool === "lasso") get().setSelectionOptions({ lasso: o.lasso === "Freehand" ? "Polygonal" : "Freehand" });
+  },
+  hasSelection: () => {
+    const { activeId, documents } = get();
+    const selection = activeId ? documents[activeId]?.selection : null;
+    return !!selection && !selection.empty;
+  },
 }));
```

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -1,6 +1,6 @@
 import { useEditor } from "../state/store";
-import { activeLayer } from "../state/selection";
-import type { AdjustmentKind, BlendMode } from "../engine/types";
+import { activeLayer, visibleIds } from "../state/selection";
+import type { AdjustmentKind, BlendMode, SelectionMode } from "../engine/types";
 import { isEditableKind } from "../engine/types";
 import type { DropTarget } from "../panels/layer-rows";
 
@@ -30,7 +30,36 @@ export function addFolder(): void { const c = ctx(); if (!c) return; c.s.commitT
 // like every other action here. Without that, a pending mask move survives the mask it moves
 // and Enter later fails with "the layer has no mask"; macOS disables both menu items while a
 // transform is pending (canEditLayers requires transformEdit == nil).
-export function addMaskToActive(revealing: boolean): void { const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.commitTransform(); c.s.run({ type: "AddMask", id: c.active.id, revealing }); c.s.setMaskSelected(true); }
+// With a selection, Add Mask paints the opposite tone through it and uses it up, one undo step
+// ("Add Mask from Selection", LayerMask.swift:228-260); every Add Mask entry point on the Mac goes
+// through that one `addMask`.
+export function addMaskToActive(revealing: boolean): void {
+  const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.commitTransform();
+  const ok = c.s.run(c.doc.selection ? { type: "AddMaskFromSelection", id: c.active.id, revealing } : { type: "AddMask", id: c.active.id, revealing });
+  if (ok) c.s.setMaskSelected(true);
+}
+/** Whether Delete may clear through the selection now (`canPaint`, EditorSession+Brush.swift:5-11):
+ * one layer targeted, shown, a pixel layer or an enabled mask, and a selection with something in it. */
+export function canClearSelected(): boolean {
+  const c = ctx(); if (!c?.active || !c.doc.selection || c.doc.selection.empty || c.selected.length !== 1 || c.s.panelOwnsDocument()) return false;
+  if (!visibleIds(c.doc).has(c.active.id)) return false;
+  return c.s.maskSelected && c.active.hasMask ? c.active.maskEnabled : !c.active.isGroup && c.active.hasPixels;
+}
+/** Delete / Backspace (`deleteKeyPressed`, SelectionEdits.swift:60-63): with a selection, clears the
+ * selected pixels, or fills the targeted mask white there ("Clear" / "Fill Mask"); an edit that may
+ * not paint now does nothing. Without one, deletes the selected layers as before. */
+export function deleteKeyPressed(): void {
+  const c = ctx(); if (!c) return;
+  if (!c.doc.selection) { deleteSelected(); return; }
+  if (!canClearSelected()) return;
+  c.s.run({ type: "ClearSelectedPixels", id: c.active!.id, mask: c.s.maskSelected && c.active!.hasMask });
+}
+/** Ctrl-click on a thumbnail, or Select > Layer's Pixels / Mask's Black Areas (MaskTracing.swift:73-94). */
+export function loadSelection(id: string, mask: boolean, mode: SelectionMode = "Replace"): void {
+  const c = ctx(); if (!c) return;
+  const antialiased = c.s.selectionOptions.antialiased;
+  c.s.run(mask ? { type: "LoadMaskSelection", id, mode, antialiased } : { type: "LoadLayerSelection", id, mode, antialiased });
+}
 export function deleteMaskOfActive(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.commitTransform(); c.s.run({ type: "DeleteMask", id: c.active.id }); c.s.setMaskSelected(false); }
 export function toggleMaskEnabled(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.run({ type: "SetMaskEnabled", id: c.active.id, enabled: !c.active.maskEnabled }); }
 export function toggleMaskLink(): void { const c = ctx(); if (!c?.active?.hasMask) return; c.s.commitTransform(); c.s.run({ type: "SetMaskLinked", id: c.active.id, linked: !c.active.maskLinked }); }
@@ -74,13 +103,15 @@ export function cycleBlendMode(forward: boolean): void {
 }
 /** Image > Invert: immediate, one undo step, on the mask when the mask chip is selected. */
 export function invertActive(): void {
-  const c = ctx(); if (!c?.active) return;
+  const c = ctx(); if (!c?.active || !canInvert()) return;
   const mask = c.s.maskSelected && c.active.hasMask;
   if (!mask && !c.active.hasPixels) return;
   c.s.run({ type: "InvertPixels", id: c.active.id, mask });
 }
 export function canInvert(): boolean {
   const c = ctx(); if (!c?.active || c.active.isGroup) return false;
+  // An empty selection inverts nothing (`canInvert`, SelectionEdits.swift:77-83).
+  if (c.doc.selection?.empty) return false;
   return (c.s.maskSelected && c.active.hasMask) || c.active.hasPixels;
 }
 /** Layer > New <kind> Adjustment. Each Grain layer gets its own pattern, and a Gradient Map
```

```diff
--- a/app/src/tools/crop-tool.ts
+++ b/app/src/tools/crop-tool.ts
@@ -35,6 +35,19 @@ export function snapTargets(state: DocumentState): { xs: number[]; ys: number[]
   return { xs, ys };
 }
 
+/** Where the crop starts when the tool is chosen (`selectTool`, EditorSession.swift:349-358): the
+ * selection's bounds rounded out and cut to the canvas when there is a selection with something in
+ * it, as Photoshop's C then Enter crops to it; else, or when that is not a valid crop, the canvas. */
+export function cropSeed(state: DocumentState): Rect {
+  const canvas = { x: 0, y: 0, width: state.width, height: state.height };
+  const b = state.selection && !state.selection.empty ? state.selection.bounds : null;
+  if (!b) return canvas;
+  const x0 = Math.max(0, Math.floor(b.x)), y0 = Math.max(0, Math.floor(b.y));
+  const x1 = Math.min(state.width, Math.ceil(b.x + b.width)), y1 = Math.min(state.height, Math.ceil(b.y + b.height));
+  const rect = { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
+  return isValid(rect) ? rect : canvas;
+}
+
 export function ratioValue(choice: CropRatio, state: DocumentState): number | null {
   switch (choice) { case "Original": return state.width / state.height; case "1:1": return 1; case "4:3": return 4 / 3; case "16:9": return 16 / 9; default: return null; }
 }
```

- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test` (135 (+9)), `pnpm build`.

- [ ] **Step 5: Prove it bites**

(1) In `deleteKeyPressed`, call `deleteSelected()` whatever the selection: the Delete test fails (`DeleteLayers`, not `ClearSelectedPixels`). Restore. (2) In `cropSeed`, return the canvas always: the new crop-seed test fails. Restore. (3) In `canAdjust`, drop the empty-selection check: the refusal test fails. Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/unit/selection-store.test.ts
git commit -- app/src/state/store.ts app/src/actions/layers.ts app/src/tools/crop-tool.ts app/tests/unit/crop-seed.test.ts app/tests/unit/selection-store.test.ts -m "feat: selection tools, options and routing in the store; the crop starts at the selection" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: The canvas: drawing, moving, the Magic Wand's click and the marching ants

**Files:**
- Create: `app/src/canvas/ants.ts`
- Modify: `app/src/canvas/overlay.ts`, `app/src/canvas/CanvasView.tsx`
- Create test: `app/tests/unit/ants.test.ts`

The overlay draws the ants (a white line under a black 4/4 dash that moves one step every 120 ms, TransformOverlay.swift:283-298, EditorCanvas.swift:2004-2020) and a draft (black 0.8 alpha 2 px under white 1 px, the Polygonal Lasso's first corner as an 8 px square to click, :301-333). Painting the overlay is now its own function: the ants' timer and a draft call it without re-rendering the picture. The timer runs only while a selection with something in it exists. The outline is fetched once per document, revision and zoom step (`OutlineCache`, `outlineStep`). The pointer handling is the Mac's `lassoMouseDown` / `mouseDragged` / `mouseUp` (EditorCanvas.swift:1961-2002, :1513-1531, :1696-1716): a press in New mode inside the selection (`selectionContains`) drags the outline, one `MoveSelection` on release, and a click there without a drag deselects (the Magic Wand selects afresh at that pixel); otherwise the Magic Wand selects at the click, and the Marquee and the Lasso draw. Shift pressed or let go mid-drag reshapes the Marquee at once; Shift and Alt held show in the options bar (`heldSelectionMode`). The selection tools use a crosshair.

**Interfaces:**
- Consumes: Tasks 8 to 10.
- Produces: `ANTS_INTERVAL_MS = 120`, `nextPhase`, `outlineStep(zoom)`, `class OutlineCache`; `OverlayState.ants?: AntsState | null` and `.draft?: DraftState | null`; `ANTS_DASH = 4`.

- [ ] **Step 1: Write the failing tests**

```ts
import { describe, expect, it } from "vitest";
import { nextPhase, OutlineCache, outlineStep } from "../../src/canvas/ants";

describe("marching ants", () => {
  it("fetch the full outline at 1:1 and above, and a power-of-two step below", () => {
    expect([outlineStep(4), outlineStep(1), outlineStep(0.9), outlineStep(0.5), outlineStep(0.3), outlineStep(0.01)])
      .toEqual([1, 1, 0.5, 0.5, 0.25, 1 / 128]);
  });

  it("refetch only when the document, the selection's revision or the step changes", () => {
    const cache = new OutlineCache();
    let fetches = 0;
    const fetch = () => { fetches++; return [[[0, 0], [1, 0], [1, 1]] as [number, number][]]; };
    cache.get("D", 5, 1, fetch); cache.get("D", 5, 1, fetch);
    expect(fetches).toBe(1);
    cache.get("D", 6, 1, fetch); cache.get("D", 6, 0.5, fetch); cache.get("E", 6, 0.5, fetch);
    expect(fetches).toBe(4);
  });

  it("march one step a tick around a period of eight", () => {
    expect([nextPhase(0), nextPhase(6), nextPhase(7)]).toEqual([1, 7, 0]);
  });
});
```

- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test -- ants`: the module does not exist.

- [ ] **Step 3: The ants, the drafts and the pointer**

```ts
import type { PointTuple } from "../engine/types";

/** How often the ants march, in ms (`updateAntsTimer`, EditorCanvas.swift:2004-2020). */
export const ANTS_INTERVAL_MS = 120;
/** One march step; the phase wraps at the dash period (4 on, 4 off). */
export const nextPhase = (phase: number): number => (phase + 1) % 8;

/** The outline's detail for a zoom (device px per document px): 1 at or above 1:1, else the power of
 * two at or below the zoom, so the engine traces a very detailed outline no finer than the screen
 * shows it (`selection_lod`; TransformOverlay.swift:92-177, :284-298). */
export function outlineStep(zoom: number): number {
  if (!(zoom > 0) || zoom >= 1) return 1;
  return Math.pow(2, Math.floor(Math.log2(zoom)));
}

/** The last outline fetched, kept until the document, the selection's revision or the step
 * changes: the ants redraw every tick without asking the engine again. */
export class OutlineCache {
  private key: string | null = null;
  private contours: PointTuple[][] = [];
  get(doc: string, revision: number, step: number, fetch: () => PointTuple[][]): PointTuple[][] {
    const key = `${doc}:${revision}:${step}`;
    if (key !== this.key) { this.contours = fetch(); this.key = key; }
    return this.contours;
  }
}
```

```diff
--- a/app/src/canvas/overlay.ts
+++ b/app/src/canvas/overlay.ts
@@ -2,7 +2,12 @@ import type { Viewport } from "./viewport";
 import type { Rect } from "../tools/crop-geometry";
 import { HANDLES } from "../tools/crop-geometry";
 import type { OverlayGeometry } from "../tools/transform-geometry";
-import type { Guide } from "../engine/types";
+import type { Guide, PointTuple, SelectionShape } from "../engine/types";
+
+/** The marching ants: the selection's outline in document pixels, moved by `offset` while it is dragged. */
+export interface AntsState { contours: PointTuple[][]; offset: { dx: number; dy: number }; phase: number; }
+/** An outline being drawn, in document pixels (`LassoDraft`). */
+export interface DraftState { kind: SelectionShape; points: { x: number; y: number }[]; cursor: { x: number; y: number } | null; }
 
 export interface OverlayState {
   docWidth: number; docHeight: number; cropRect: Rect | null;
@@ -11,6 +16,58 @@ export interface OverlayState {
   transform: OverlayGeometry | null;
   /** The document's saved guides, or null while View > Hide Guides is in effect. */
   canvasGuides: Guide[] | null;
+  /** Null with no selection, or an empty one. */
+  ants?: AntsState | null;
+  draft?: DraftState | null;
+}
+
+/** The dash the ants march along, in view px (`drawSelection`, TransformOverlay.swift:283-298). */
+export const ANTS_DASH = 4;
+
+/** A white line under a black dash shifted by `phase` (TransformOverlay.swift:283-298). */
+function drawAnts(ctx: CanvasRenderingContext2D, viewport: Viewport, size: { width: number; height: number }, ants: AntsState): void {
+  ctx.save();
+  ctx.beginPath();
+  for (const contour of ants.contours) {
+    contour.forEach(([x, y], i) => {
+      const v = viewport.viewPoint({ x: x + ants.offset.dx, y: y + ants.offset.dy }, size);
+      if (i === 0) ctx.moveTo(v.x, v.y); else ctx.lineTo(v.x, v.y);
+    });
+    ctx.closePath();
+  }
+  ctx.lineWidth = 1;
+  ctx.strokeStyle = "white";
+  ctx.stroke();
+  ctx.setLineDash([ANTS_DASH, ANTS_DASH]);
+  ctx.lineDashOffset = ants.phase;
+  ctx.strokeStyle = "black";
+  ctx.stroke();
+  ctx.restore();
+}
+
+/** The outline being drawn: black 0.8 alpha 2 px under white 1 px; the Polygonal Lasso's first
+ * corner as an 8 px handle to click (`drawLassoDraft`, TransformOverlay.swift:301-333). */
+function drawDraft(ctx: CanvasRenderingContext2D, viewport: Viewport, size: { width: number; height: number }, draft: DraftState): void {
+  const points = draft.points.map((p) => viewport.viewPoint(p, size));
+  if (draft.kind === "Polygonal" && draft.cursor) points.push(viewport.viewPoint(draft.cursor, size));
+  if (points.length === 0) return;
+  ctx.save();
+  ctx.beginPath();
+  if (draft.kind === "Ellipse" && points.length === 4) {
+    const xs = points.map((p) => p.x), ys = points.map((p) => p.y);
+    const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
+    ctx.ellipse((x0 + x1) / 2, (y0 + y1) / 2, (x1 - x0) / 2, (y1 - y0) / 2, 0, 0, Math.PI * 2);
+  } else {
+    points.forEach((p, i) => { if (i === 0) ctx.moveTo(p.x, p.y); else ctx.lineTo(p.x, p.y); });
+    if (draft.kind === "Rectangle") ctx.closePath();
+  }
+  ctx.strokeStyle = "rgba(0,0,0,0.8)"; ctx.lineWidth = 2; ctx.stroke();
+  ctx.strokeStyle = "white"; ctx.lineWidth = 1; ctx.stroke();
+  if (draft.kind === "Polygonal") {
+    ctx.fillStyle = "white"; ctx.fillRect(points[0].x - 4, points[0].y - 4, 8, 8);
+    ctx.strokeStyle = "black"; ctx.strokeRect(points[0].x - 4, points[0].y - 4, 8, 8);
+  }
+  ctx.restore();
 }
 
 export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, dpr: number, state: OverlayState): void {
@@ -86,6 +143,8 @@ export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, d
       ctx.strokeRect(h.x - 3.5, h.y - 3.5, 7, 7);
     }
   }
+  if (state.ants) drawAnts(ctx, viewport, size, state.ants);
+  if (state.draft) drawDraft(ctx, viewport, size, state.draft);
   ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
   for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
   for (const y of state.guides.ys) { const v = viewport.viewPoint({ x: 0, y }, size).y; ctx.beginPath(); ctx.moveTo(0, v + 0.5); ctx.lineTo(viewport.viewSize.width, v + 0.5); ctx.stroke(); }
```

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -7,6 +7,8 @@ import { CropSession, hitTest, ratioValue, SNAP_SCREEN_PX } from "../tools/crop-
 import { TransformSession, startMode } from "../tools/transform-session";
 import { containsPoint, cornersToTuples, fromTuple, hitOverlay, overlayGeometry, snapTargets, type OverlayGeometry, type P } from "../tools/transform-geometry";
 import { activeLayer, canTransform, editedShape, transformsAsGroup } from "../state/selection";
+import { ANTS_INTERVAL_MS, nextPhase, OutlineCache, outlineStep } from "./ants";
+import { isSelectionTool, outlineOffset, SelectionDraft, selectionMode, type P as DocP } from "../tools/selection-draft";
 
 export const HIT_HANDLE_PX = 6;
 
@@ -31,6 +33,39 @@ export function CanvasView() {
   const selectedLayerIds = useEditor((s) => s.selectedLayerIds);
   const maskSelected = useEditor((s) => s.maskSelected);
   const sampleMode = useEditor((s) => s.adjustEdit?.sampleMode ?? null);
+  const overlayTick = useEditor((s) => s.overlayTick);
+  const selection = useEditor((s) => (s.activeId ? s.documents[s.activeId]?.selection ?? null : null));
+  const outlineCacheRef = useRef(new OutlineCache());
+  const antsPhaseRef = useRef(0);
+
+  /** Draws the overlay from the store as it is now: the pixel grid, guides, the crop frame, the
+   * transform handles, the marching ants and an outline being drawn. The ants' timer and an outline
+   * drag repaint only this, never the picture beneath. */
+  const paintOverlay = () => {
+    const gl = glRef.current, overlay = overlayRef.current; const s = useEditor.getState();
+    if (!gl || !overlay || !s.activeId || !s.engine) return;
+    const doc = s.documents[s.activeId], vp = s.viewports[s.activeId];
+    if (!doc || !vp) return;
+    const dpr = window.devicePixelRatio || 1;
+    overlay.width = gl.width; overlay.height = gl.height;
+    let transformGeometry: OverlayGeometry | null = null;
+    if (s.tool === "move" && (s.transformEdit || canTransform(doc, s.selectedLayerIds, s.maskSelected))) {
+      const shape = editedShape(doc, s.transformEdit, s.selectedLayerIds, s.maskSelected);
+      if (shape) transformGeometry = overlayGeometry(shape.corners ?? shape.transform, vp, { width: doc.width, height: doc.height });
+    }
+    const sel = doc.selection;
+    const engine = s.engine; const id = s.activeId;
+    const step = outlineStep(vp.zoom);
+    const ants = sel && !sel.empty
+      ? { contours: outlineCacheRef.current.get(id, sel.revision, step, () => engine.selectionOutline(id, step)), offset: s.outlineMove ?? { dx: 0, dy: 0 }, phase: antsPhaseRef.current }
+      : null;
+    const d = s.selectionDraft;
+    drawOverlay(overlay.getContext("2d")!, vp, dpr, {
+      docWidth: doc.width, docHeight: doc.height, cropRect: s.tool === "crop" ? s.cropRect : null, guides: s.snapGuides,
+      transform: transformGeometry, canvasGuides: s.showGuides ? doc.guides : null,
+      ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null,
+    });
+  };
 
   // Renderer lifetime follows the canvas element.
   useEffect(() => {
@@ -91,18 +126,21 @@ export function CanvasView() {
     if (!renderer || !gl || !overlay || !state || !viewport || !engine) return;
     const dpr = window.devicePixelRatio || 1;
     renderer.render(engine, state, viewport, dpr, { checkerboard: checkerboardRef.current }, useEditor.getState().previewEdit());
-    overlay.width = gl.width; overlay.height = gl.height;
-    let transformGeometry: OverlayGeometry | null = null;
-    if (tool === "move" && (transformEdit || canTransform(state, selectedLayerIds, maskSelected))) {
-      const shape = editedShape(state, transformEdit, selectedLayerIds, maskSelected);
-      if (shape) transformGeometry = overlayGeometry(shape.corners ?? shape.transform, viewport, { width: state.width, height: state.height });
-    }
-    drawOverlay(overlay.getContext("2d")!, viewport, dpr, {
-      docWidth: state.width, docHeight: state.height, cropRect: tool === "crop" ? cropRect : null, guides: snapGuides,
-      transform: transformGeometry, canvasGuides: showGuides ? state.guides : null,
-    });
+    paintOverlay();
   }, [state, viewport, cropRect, tool, renderTick, engine, transformEdit, snapGuides, selectedLayerIds, maskSelected, showGuides]);
 
+  // An outline being drawn or dragged repaints the overlay alone.
+  useEffect(() => { paintOverlay(); }, [overlayTick]);
+
+  // Marching ants: march every 120 ms, only while a selection with something in it exists
+  // (`updateAntsTimer`, EditorCanvas.swift:2004-2020).
+  const antsActive = !!selection && !selection.empty;
+  useEffect(() => {
+    if (!antsActive) return;
+    const timer = setInterval(() => { antsPhaseRef.current = nextPhase(antsPhaseRef.current); paintOverlay(); }, ANTS_INTERVAL_MS);
+    return () => clearInterval(timer);
+  }, [antsActive, activeId]);
+
   // Wheel: zoom with Ctrl, otherwise pan.
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
@@ -151,11 +189,12 @@ export function CanvasView() {
     return () => el.removeEventListener("pointerdown", down, { capture: true });
   }, []);
 
-  // The cursor shows an armed eyedropper regardless of which tool is otherwise selected.
+  // The cursor shows an armed eyedropper regardless of which tool is otherwise selected, and a
+  // crosshair for the selection tools.
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
-    el.style.cursor = sampleMode ? "crosshair" : "";
-  }, [sampleMode]);
+    el.style.cursor = sampleMode || isSelectionTool(tool) ? "crosshair" : "";
+  }, [sampleMode, tool]);
 
   // Drag to pan with the hand tool or the space bar.
   useEffect(() => {
@@ -292,6 +331,98 @@ export function CanvasView() {
     return () => { el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
   }, []);
 
+  // The selection tools (Phase 4a; EditorCanvas.swift:1961-2002, :1513-1531, :1696-1716). A press
+  // picks its mode from Shift / Alt; in New mode, inside a selection, it drags the outline instead
+  // (one MoveSelection on release; a click without a drag deselects, or with the wand selects afresh
+  // at that pixel). The Marquee and the Freehand Lasso draw while dragged and finish on release; the
+  // Polygonal Lasso adds a corner per click and closes on its first corner or a double-click; the
+  // Magic Wand selects at the click.
+  useEffect(() => {
+    const el = glRef.current?.parentElement; if (!el) return;
+    let moveStart: DocP | null = null;
+    let lastPixel: DocP | null = null;
+    const view = (e: { clientX: number; clientY: number }) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
+    const docPoint = (e: { clientX: number; clientY: number }): DocP => {
+      const s = useEditor.getState(); const vp = s.viewports[s.activeId!]; const d = s.documents[s.activeId!];
+      return vp.documentPoint(view(e), { width: d.width, height: d.height });
+    };
+    const down = (e: PointerEvent) => {
+      const s = useEditor.getState();
+      if (!isSelectionTool(s.tool) || e.button !== 0 || spaceRef.current || !s.activeId || !s.engine) return;
+      if (s.panelOwnsDocument(true)) return;
+      const d = s.documents[s.activeId]; const vp = s.viewports[s.activeId];
+      const pixel = docPoint(e);
+      const draft = s.selectionDraft;
+      if (draft?.kind === "Polygonal") {
+        const first = vp.viewPoint(draft.points[0], { width: d.width, height: d.height });
+        if (draft.click(pixel, view(e), first, 1) === "close") s.finishSelectionDraft(); else s.setSelectionDraft(draft);
+        return;
+      }
+      const o = s.selectionOptions;
+      const mode = selectionMode(o.mode, e.shiftKey, e.altKey);
+      if (mode === "Replace" && s.engine.selectionContains(s.activeId, pixel)) {
+        moveStart = pixel;
+        s.setOutlineMove({ dx: 0, dy: 0 });
+        el.setPointerCapture(e.pointerId);
+        return;
+      }
+      if (s.tool === "wand") {
+        // Off the canvas there is no colour to match (`magicWand(at:)` reads the canvas-sized sample).
+        if (pixel.x >= 0 && pixel.y >= 0 && pixel.x < d.width && pixel.y < d.height) {
+          s.run({ type: "MagicWand", at: [pixel.x, pixel.y], mode, settings: o.wand, antialiased: o.antialiased });
+        }
+        return;
+      }
+      lastPixel = pixel;
+      s.setSelectionDraft(SelectionDraft.begin(s.tool === "marquee" ? o.marquee : o.lasso, mode, pixel, e.shiftKey));
+      el.setPointerCapture(e.pointerId);
+    };
+    const move = (e: PointerEvent) => {
+      const s = useEditor.getState();
+      if (!isSelectionTool(s.tool) || !s.activeId) return;
+      const pixel = docPoint(e);
+      if (moveStart) { s.setOutlineMove(outlineOffset(moveStart, pixel, e.shiftKey)); return; }
+      const draft = s.selectionDraft; if (!draft) return;
+      if (draft.kind === "Polygonal") { draft.moveCursor(pixel); s.setSelectionDraft(draft); return; }
+      if (!el.hasPointerCapture(e.pointerId)) return;
+      lastPixel = pixel;
+      if (draft.isMarquee) draft.dragMarquee(pixel, e.shiftKey); else draft.extend(pixel);
+      s.setSelectionDraft(draft);
+    };
+    const up = () => {
+      const s = useEditor.getState();
+      if (moveStart) {
+        const start = moveStart, offset = s.outlineMove;
+        moveStart = null;
+        s.setOutlineMove(null);
+        if (offset && (offset.dx !== 0 || offset.dy !== 0)) s.run({ type: "MoveSelection", dx: offset.dx, dy: offset.dy });
+        else if (s.tool === "wand") s.run({ type: "MagicWand", at: [start.x, start.y], mode: "Replace", settings: s.selectionOptions.wand, antialiased: s.selectionOptions.antialiased });
+        else s.run({ type: "Deselect" });
+        return;
+      }
+      lastPixel = null;
+      const draft = s.selectionDraft;
+      if (draft && draft.kind !== "Polygonal") s.finishSelectionDraft();
+    };
+    const dblclick = () => { const s = useEditor.getState(); if (s.selectionDraft?.kind === "Polygonal") s.finishSelectionDraft(); };
+    // Shift pressed or let go mid-drag reshapes the Marquee at once, without waiting for the
+    // pointer to move (EditorCanvas.swift:611-619); Shift / Alt held show in the options bar.
+    const keys = (e: KeyboardEvent) => {
+      const s = useEditor.getState();
+      if (!isSelectionTool(s.tool)) return;
+      s.setHeldSelectionMode(e.altKey ? "Subtract" : e.shiftKey ? "Add" : null);
+      const draft = s.selectionDraft;
+      if (e.key === "Shift" && draft?.isMarquee && lastPixel) { draft.dragMarquee(lastPixel, e.type === "keydown"); s.setSelectionDraft(draft); }
+    };
+    const blur = () => useEditor.getState().setHeldSelectionMode(null);
+    el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up); el.addEventListener("dblclick", dblclick);
+    window.addEventListener("keydown", keys); window.addEventListener("keyup", keys); window.addEventListener("blur", blur);
+    return () => {
+      el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); el.removeEventListener("dblclick", dblclick);
+      window.removeEventListener("keydown", keys); window.removeEventListener("keyup", keys); window.removeEventListener("blur", blur);
+    };
+  }, []);
+
   return (
     <div data-testid="canvas-view" style={{ position: "relative", flex: 1, minWidth: 0, minHeight: 0, overflow: "hidden", background: "#292929" }}>
       <canvas ref={glRef} style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }} />
```

- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test` (138 (+3)), `pnpm build`. The canvas's behaviour is exercised end to end in Task 13.

- [ ] **Step 5: Prove it bites**

Key `OutlineCache` on the document and step only (drop the revision from `key`): the cache test fails (a new outline is not fetched). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/canvas/ants.ts app/tests/unit/ants.test.ts
git commit -- app/src/canvas/ants.ts app/src/canvas/overlay.ts app/src/canvas/CanvasView.tsx app/tests/unit/ants.test.ts -m "feat: marching ants, drafts and the selection tools' pointer handling on the canvas" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: The options bar, the Select menu, the amount sheet, the keys and the thumbnails

**Files:**
- Create: `app/src/panels/SelectionOptions.tsx`, `app/src/sheets/SelectionAmountSheet.tsx`
- Modify: `app/src/panels/MenuBar.tsx`, `app/src/panels/ToolRail.tsx`, `app/src/panels/LayersList.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`, `app/src/App.tsx`, `app/src/styles.css`
- Modify tests: `app/tests/unit/keymap.test.ts`, `app/tests/unit/selection-store.test.ts`

The options bar is the Mac's `LassoControls` (LassoControls.swift:3-155): the Marquee's shape or the Lasso's kind, New / Add / Subtract showing the held keys or a draft's own mode, the Magic Wand's tolerance, sample size, This Layer / All Layers and Contiguous, Anti-alias where edges can be soft (ruling OQ21), Expand / Contract / Feather with their amounts (enabled only with a selection that has something in it and no draft), and, with a selection, "Empty selection" when it is empty and Deselect. The Select menu is the Mac's (CompositorApp.swift:194-230) less Subject; Expand, Contract and Feather open the amount sheet (1 to 500, or 250 for Feather; OK only for a whole number in range). The keys: M, L, W; Ctrl+A, Ctrl+D, Ctrl+Shift+I; Tab switches the tool's kind; in a selection tool the arrows nudge the outline (Shift 10 px); Enter closes a draft, Escape drops it, Backspace removes its last corner; Delete goes through `deleteKeyPressed` (EditorCanvas.swift:1772-1827). Ctrl-clicking a thumbnail loads it (Ctrl+Shift adds, Ctrl+Alt subtracts; NativeLayerList.swift:1007-1019, :1235-1238); Ctrl-clicking elsewhere in the row still multi-selects.

**Interfaces:**
- Consumes: Tasks 8 to 11.
- Produces: `SelectionOptions` and `SelectionAmountSheet` components, `parseAmount(text, max)`; `ActionId` adds `tool-marquee`, `tool-lasso`, `tool-wand`, `select-all`, `deselect`, `select-inverse`, `cycle-tool-mode`; the Select menu's item ids `select-all`, `select-deselect`, `select-inverse`, `select-layer-pixels`, `select-mask-black`, `select-expand`, `select-contract`, `select-feather`; test ids `selection-options`, `marquee-rectangle` / `-ellipse`, `lasso-freehand` / `-polygonal`, `selection-mode-replace` / `-add` / `-subtract`, `wand-tolerance`, `wand-sample-size`, `wand-sample`, `wand-contiguous`, `selection-antialias`, `selection-expand` / `-contract` / `-feather` and their `-amount` fields, `selection-empty`, `selection-deselect`, `selection-amount`.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -48,6 +48,18 @@ describe("keymap", () => {
     expect(matchShortcut(ev("i", { ctrlKey: true }))).toBe("invert");
     expect(matchShortcut(ev("i", { ctrlKey: true, altKey: true }))).toBe("image-size"); // still its own binding
   });
+
+  it("maps the selection tools and the Select menu", () => {
+    expect(matchShortcut(ev("m"))).toBe("tool-marquee");
+    expect(matchShortcut(ev("L"))).toBe("tool-lasso");
+    expect(matchShortcut(ev("w"))).toBe("tool-wand");
+    expect(matchShortcut(ev("a", { ctrlKey: true }))).toBe("select-all");
+    expect(matchShortcut(ev("d", { ctrlKey: true }))).toBe("deselect");
+    expect(matchShortcut(ev("I", { ctrlKey: true, shiftKey: true }))).toBe("select-inverse");
+    expect(matchShortcut(ev("Tab"))).toBe("cycle-tool-mode");
+    expect(matchShortcut(ev("Tab", { shiftKey: true }))).toBeNull();
+    expect(matchShortcut(ev("w", { ctrlKey: true }))).toBe("close"); // still its own binding
+  });
 });
 
 describe("typeOpacityDigit", () => {
```

```diff
--- a/app/tests/unit/selection-store.test.ts
+++ b/app/tests/unit/selection-store.test.ts
@@ -2,6 +2,7 @@ import { beforeEach, describe, expect, it } from "vitest";
 import { DEFAULT_SELECTION_OPTIONS, useEditor } from "../../src/state/store";
 import { addMaskToActive, canInvert, deleteKeyPressed, loadSelection } from "../../src/actions/layers";
 import { SelectionDraft } from "../../src/tools/selection-draft";
+import { parseAmount } from "../../src/sheets/SelectionAmountSheet";
 import type { Command, DocumentState, LayerState, LayerTransform, SelectionState } from "../../src/engine/types";
 import type { EngineClient } from "../../src/engine/client";
 
@@ -68,6 +69,13 @@ describe("Add Mask", () => {
   });
 });
 
+describe("the Expand / Contract / Feather sheet", () => {
+  it("takes only a whole number from 1 to the operation's maximum (SelectionAmountSheet.amount)", () => {
+    expect([parseAmount(" 12 ", 500), parseAmount("500", 500), parseAmount("251", 250), parseAmount("0", 250), parseAmount("2.5", 250), parseAmount("", 250), parseAmount("1e2", 500)])
+      .toEqual([12, 500, null, null, null, null, null]);
+  });
+});
+
 describe("an empty selection refuses every edit", () => {
   it("greys out Levels, the filters and Invert", () => {
     stub(document([layer("A")], selected(true)));
```

- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test -- keymap selection-store`: the new keys map to nothing, and `SelectionAmountSheet` does not exist.

- [ ] **Step 3: The options bar, the sheet, the menu, the rail, the thumbnails and the keys**

```tsx
import { useEditor, SELECTION_AMOUNT_MAX, type SelectionAmountOperation } from "../state/store";
import type { SelectionMode } from "../engine/types";
import { NumberInput } from "./NumberInput";

const MODES: { mode: SelectionMode; label: string }[] = [{ mode: "Replace", label: "New" }, { mode: "Add", label: "Add" }, { mode: "Subtract", label: "Subtract" }];
const SAMPLE_SIZES = ["Point Sample", "3 by 3 Average", "5 by 5 Average"];

/** The selection tools' header (`LassoControls`, LassoControls.swift:3-155): the Marquee's shape or
 * the Lasso's kind, the mode (showing Shift / Alt while held, or an outline's own mode), the Magic
 * Wand's settings, Anti-alias where edges can be soft, and Expand / Contract / Feather with their
 * amounts; "Empty selection" and Deselect when there is a selection. */
export function SelectionOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if ((s.tool !== "marquee" && s.tool !== "lasso" && s.tool !== "wand") || !doc || s.sheet !== null) return null;
  const o = s.selectionOptions;
  const shown = s.selectionDraft?.mode ?? s.heldSelectionMode ?? o.mode;
  const editable = !s.panelOwnsDocument();
  const canModify = editable && s.hasSelection() && !s.selectionDraft;
  const modify = (operation: SelectionAmountOperation) => {
    const key = operation === "Expand" ? "expand" : operation === "Contract" ? "contract" : "feather";
    return (
      <span className="selection-modify">
        <button data-testid={`selection-${key}`} disabled={!canModify} onClick={(e) => { s.modifySelection(operation, o[key]); e.currentTarget.blur(); }}>{operation}</button>
        <NumberInput label={`${operation} amount`} testId={`selection-${key}-amount`} value={o[key]} min={1} max={SELECTION_AMOUNT_MAX[operation]} step={1}
          disabled={!canModify} onChange={(v) => s.setSelectionOptions({ [key]: Math.round(v) })} /> px
      </span>
    );
  };
  return (
    <div className="tool-options" data-testid="selection-options">
      <strong>{s.tool === "marquee" ? "Marquee" : s.tool === "wand" ? "Magic Wand" : "Lasso"}</strong>
      {s.tool === "marquee" && (
        <span className="segmented" title="Tab switches between Rectangle and Ellipse">
          {(["Rectangle", "Ellipse"] as const).map((k) => (
            <button key={k} data-testid={`marquee-${k.toLowerCase()}`} aria-pressed={o.marquee === k} onClick={(e) => { s.setSelectionOptions({ marquee: k }); e.currentTarget.blur(); }}>{k}</button>
          ))}
        </span>
      )}
      {s.tool === "lasso" && (
        <span className="segmented" title="Tab switches between Freehand and Polygonal">
          {(["Freehand", "Polygonal"] as const).map((k) => (
            <button key={k} data-testid={`lasso-${k.toLowerCase()}`} aria-pressed={o.lasso === k} onClick={(e) => { s.setSelectionOptions({ lasso: k }); e.currentTarget.blur(); }}>{k}</button>
          ))}
        </span>
      )}
      <span className="segmented" title="Hold Shift to add or Alt to subtract for one outline">
        {MODES.map(({ mode, label }) => (
          <button key={mode} data-testid={`selection-mode-${mode.toLowerCase()}`} aria-pressed={shown === mode} onClick={(e) => { s.setSelectionOptions({ mode }); e.currentTarget.blur(); }}>{label}</button>
        ))}
      </span>
      {s.tool === "wand" && (
        <>
          <label title="How far each colour channel (0-255) can differ from the clicked colour and still be selected">Tolerance{" "}
            <NumberInput label="Tolerance" testId="wand-tolerance" value={o.wand.tolerance} min={0} max={255} step={1} onChange={(v) => s.setSelectionOptions({ wand: { ...o.wand, tolerance: Math.round(v) } })} />
          </label>
          <select aria-label="Sample Size" data-testid="wand-sample-size" value={o.wand.sampleRadius} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, sampleRadius: Number(e.target.value) } })}>
            {SAMPLE_SIZES.map((label, radius) => <option key={radius} value={radius}>{label}</option>)}
          </select>
          <select aria-label="Sample" data-testid="wand-sample" value={o.wand.allLayers ? "all" : "this"} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, allLayers: e.target.value === "all" } })}>
            <option value="this">This Layer</option><option value="all">All Layers</option>
          </select>
          <label title="Select only similar pixels connected to the one you click; off selects them everywhere">
            <input type="checkbox" data-testid="wand-contiguous" checked={o.wand.contiguous} onChange={(e) => s.setSelectionOptions({ wand: { ...o.wand, contiguous: e.target.checked } })} /> Contiguous
          </label>
        </>
      )}
      {/* Rectangles snap to whole pixels, so Anti-alias does not apply to them (LassoControls.swift:48-52). */}
      {(s.tool !== "marquee" || o.marquee === "Ellipse") && (
        <label title="Smooth selection edges; turn off for hard pixel edges">
          <input type="checkbox" data-testid="selection-antialias" checked={o.antialiased} onChange={(e) => s.setSelectionOptions({ antialiased: e.target.checked })} /> Anti-alias
        </label>
      )}
      {modify("Expand")}
      {modify("Contract")}
      {modify("Feather")}
      {doc.selection && (
        <>
          {doc.selection.empty && <span data-testid="selection-empty" className="hint">Empty selection</span>}
          <button data-testid="selection-deselect" disabled={!editable} onClick={(e) => { s.run({ type: "Deselect" }); e.currentTarget.blur(); }}>Deselect</button>
        </>
      )}
    </div>
  );
}
```

```tsx
import { useState } from "react";
import { Sheet } from "./Sheet";
import { SELECTION_AMOUNT_MAX, useEditor, type SelectionAmountOperation } from "../state/store";

/** The whole number of pixels typed, when it lies in 1..max; null otherwise (`SelectionAmountSheet.amount`). */
export function parseAmount(text: string, max: number): number | null {
  const trimmed = text.trim();
  if (!/^\d+$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return value >= 1 && value <= max ? value : null;
}

/** Select > Expand / Contract / Feather: a slider and a field for the amount, OK only with a whole
 * number in range (LassoControls.swift:180-236). */
export function SelectionAmountSheet(props: { operation: SelectionAmountOperation }) {
  const s = useEditor();
  const max = SELECTION_AMOUNT_MAX[props.operation];
  const key = props.operation === "Expand" ? "expand" : props.operation === "Contract" ? "contract" : "feather";
  const [input, setInput] = useState(String(s.selectionOptions[key]));
  const amount = parseAmount(input, max);
  return (
    <Sheet title={`${props.operation} Selection`} primary="OK" canConfirm={amount !== null} onCancel={s.closeSheet}
      onConfirm={() => { if (amount !== null) { s.closeSheet(); s.modifySelection(props.operation, amount); } }}>
      <label>Amount <input aria-label="Amount slider" type="range" min={1} max={max} step={1} value={amount ?? 1} onChange={(e) => setInput(e.target.value)} />
        <input aria-label="Amount" data-testid="selection-amount" autoFocus value={input} onChange={(e) => setInput(e.target.value)} /> px</label>
      {amount === null && <p className="hint">Enter a whole number from 1 to {max} px.</p>}
    </Sheet>
  );
}
```

```diff
--- a/app/src/panels/MenuBar.tsx
+++ b/app/src/panels/MenuBar.tsx
@@ -6,7 +6,7 @@ import { activeLayer } from "../state/selection";
 import {
   addAdjustmentLayer, addMaskToActive, blurMaskOfActive, canClipActive, canEditAdjustment, canInvert, canMoveActiveBy,
   deleteMaskOfActive, deleteSelected, editAdjustmentLayer, fillMaskOfActive, flipSelected, invertMaskOfActive,
-  mergeTitle, toggleMaskEnabled, toggleMaskLink,
+  loadSelection, mergeTitle, toggleMaskEnabled, toggleMaskLink,
 } from "../actions/layers";
 import { ADJUSTMENT_KINDS, isEditableKind } from "../engine/types";
 
@@ -85,6 +85,19 @@ export function MenuBar() {
       "separator",
       { id: "layer-delete", label: "Delete Layer", run: () => deleteSelected(), enabled: editable },
     ] },
+    // Select (CompositorApp.swift:194-230): Subject is Apple Vision and not in this port.
+    { title: "Select", items: [
+      { id: "select-all", label: "All", run: () => runAction("select-all"), enabled: editable },
+      { id: "select-deselect", label: "Deselect", run: () => runAction("deselect"), enabled: editable && !!activeDoc?.selection },
+      { id: "select-inverse", label: "Inverse", run: () => runAction("select-inverse"), enabled: editable && !!activeDoc?.selection },
+      { id: "select-layer-pixels", label: "Layer's Pixels", run: () => loadSelection(active!.id, false), enabled: editable && !!active && !active.isGroup && active.hasPixels },
+      { id: "select-mask-black", label: "Mask's Black Areas", run: () => loadSelection(active!.id, true), enabled: editable && hasMask },
+      "separator",
+      ...(["Expand", "Contract", "Feather"] as const).map((operation) => ({
+        id: `select-${operation.toLowerCase()}`, label: `${operation}...`, run: () => s.openSheet({ kind: "selectionAmount", operation }),
+        enabled: editable && s.hasSelection() && !s.selectionDraft,
+      })),
+    ] },
     { title: "Filter", items: [
       { id: "filter-gaussian-blur", label: "Gaussian Blur...", run: () => s.beginAdjust({ kind: "GaussianBlur" }), enabled: s.canAdjust() },
       { id: "filter-motion-blur", label: "Motion Blur...", run: () => s.beginAdjust({ kind: "MotionBlur" }), enabled: s.canAdjust() },
```

```diff
--- a/app/src/panels/ToolRail.tsx
+++ b/app/src/panels/ToolRail.tsx
@@ -1,6 +1,8 @@
 import { useEditor, type Tool } from "../state/store";
-const TOOLS: { id: Tool; label: string; key: string }[] = [
-  { id: "move", label: "Move", key: "V" }, { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" }, { id: "crop", label: "Crop", key: "C" },
+const TOOLS: { id: Tool; label: string; key: string; glyph: string }[] = [
+  { id: "move", label: "Move", key: "V", glyph: "M" }, { id: "marquee", label: "Marquee", key: "M", glyph: "[]" },
+  { id: "lasso", label: "Lasso", key: "L", glyph: "L" }, { id: "wand", label: "Magic Wand", key: "W", glyph: "W" },
+  { id: "crop", label: "Crop", key: "C", glyph: "C" }, { id: "hand", label: "Hand", key: "H", glyph: "H" }, { id: "zoom", label: "Zoom", key: "Z", glyph: "Z" },
 ];
 export function ToolRail() {
   const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
@@ -12,7 +14,7 @@ export function ToolRail() {
         // own single-letter shortcut and, for Move, the opacity digit keys) as typed
         // into a form control and swallow it.
         <button key={t.id} data-testid={`tool-${t.id}`} title={`${t.label} (${t.key})`} className={tool === t.id ? "active" : ""}
-          onClick={(e) => { setTool(t.id); e.currentTarget.blur(); }}>{t.label[0]}</button>
+          onClick={(e) => { setTool(t.id); e.currentTarget.blur(); }}>{t.glyph}</button>
       ))}
     </div>
   );
```

```diff
--- a/app/src/panels/LayersList.tsx
+++ b/app/src/panels/LayersList.tsx
@@ -2,7 +2,16 @@ import { useState, type DragEvent } from "react";
 import { useEditor } from "../state/store";
 import { layerRows, dropTarget, type Row } from "./layer-rows";
 import { ContextMenu, type MenuItem } from "./ContextMenu";
-import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, editAdjustmentLayer, flipSelected, groupSelected, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive } from "../actions/layers";
+import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, editAdjustmentLayer, flipSelected, groupSelected, loadSelection, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive } from "../actions/layers";
+
+/** Ctrl-click on a thumbnail loads it as a selection, Ctrl-Shift adds and Ctrl-Alt subtracts
+ * (`loadMode`, NativeLayerList.swift:1007-1019, :1235-1238); Ctrl-click elsewhere in a row still
+ * multi-selects. True when it loaded. */
+function loadOnCtrlClick(e: React.MouseEvent, id: string, mask: boolean): boolean {
+  if (!(e.ctrlKey || e.metaKey)) return false;
+  loadSelection(id, mask, e.altKey ? "Subtract" : e.shiftKey ? "Add" : "Replace");
+  return true;
+}
 
 type Zone = "above" | "below" | "into";
 function zoneFor(e: DragEvent, row: Row): Zone {
@@ -90,8 +99,8 @@ export function LayersList() {
               {l.adjustment
                 ? <button data-testid={`adjustment-chip-${l.id}`} className="chip chip-adjustment" aria-label={`${l.adjustment.kind} adjustment`}
                     aria-pressed={l.id === doc.activeLayerId} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); }} />
-                : <button data-testid={`target-pixels-${l.id}`} className={"chip" + (l.isGroup ? " chip-folder" : " chip-pixels")} aria-label={`${l.name} content`} aria-pressed={l.id === doc.activeLayerId && !s.maskSelected} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); s.setMaskSelected(false); }} />}
-              {l.hasMask && <button data-testid={`target-mask-${l.id}`} className={"chip chip-mask" + (l.maskEnabled ? "" : " disabled")} aria-label={`${l.name} mask`} aria-pressed={l.id === doc.activeLayerId && s.maskSelected} onClick={(e) => { e.stopPropagation(); s.selectLayers([l.id], l.id); s.setMaskSelected(true); }} />}
+                : <button data-testid={`target-pixels-${l.id}`} className={"chip" + (l.isGroup ? " chip-folder" : " chip-pixels")} aria-label={`${l.name} content`} aria-pressed={l.id === doc.activeLayerId && !s.maskSelected} onClick={(e) => { e.stopPropagation(); if (loadOnCtrlClick(e, l.id, false)) return; s.selectLayers([l.id], l.id); s.setMaskSelected(false); }} />}
+              {l.hasMask && <button data-testid={`target-mask-${l.id}`} className={"chip chip-mask" + (l.maskEnabled ? "" : " disabled")} aria-label={`${l.name} mask`} aria-pressed={l.id === doc.activeLayerId && s.maskSelected} onClick={(e) => { e.stopPropagation(); if (loadOnCtrlClick(e, l.id, true)) return; s.selectLayers([l.id], l.id); s.setMaskSelected(true); }} />}
               {renaming?.id === l.id ? (
                 <input autoFocus value={renaming.name} onClick={(e) => e.stopPropagation()} onChange={(e) => setRenaming({ id: l.id, name: e.target.value })}
                   onBlur={() => { if (renaming.name.trim()) s.run({ type: "RenameLayer", id: l.id, name: renaming.name.trim() }); setRenaming(null); }}
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -3,7 +3,8 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "nudge-left" | "nudge-right" | "nudge-up" | "nudge-down"
   | "new-folder" | "duplicate" | "group" | "merge" | "clip" | "layer-up" | "layer-down" | "blend-next" | "blend-prev" | "delete-layer"
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
-  | "levels" | "curves" | "hue-saturation" | "invert";
+  | "levels" | "curves" | "hue-saturation" | "invert"
+  | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -30,6 +31,11 @@ export const SHORTCUTS = {
   "opacity-5": [{ key: "5" }], "opacity-6": [{ key: "6" }], "opacity-7": [{ key: "7" }], "opacity-8": [{ key: "8" }], "opacity-9": [{ key: "9" }],
   "levels": [{ key: "l", ctrl: true }], "curves": [{ key: "m", ctrl: true }],
   "hue-saturation": [{ key: "u", ctrl: true }], "invert": [{ key: "i", ctrl: true }],
+  // The selection tools and the Select menu (KeyboardShortcuts.swift; CompositorApp.swift:194-230).
+  "tool-marquee": [{ key: "m" }], "tool-lasso": [{ key: "l" }], "tool-wand": [{ key: "w" }],
+  "select-all": [{ key: "a", ctrl: true }], "deselect": [{ key: "d", ctrl: true }], "select-inverse": [{ key: "i", ctrl: true, shift: true }],
+  // Tab switches the current tool's kind (EditorCanvas.swift:1823-1827).
+  "cycle-tool-mode": [{ key: "Tab" }],
 } satisfies Record<ActionId, Shortcut[]>;
 
 export function matchShortcut(e: KeyboardEvent): ActionId | null {
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -6,7 +6,8 @@ import { isEditableTarget } from "./target";
 import { nudgeDelta } from "../tools/transform-session";
 import type { Corners, PointTuple } from "../engine/types";
 import { activeLayer } from "../state/selection";
-import { addFolder, cycleBlendMode, deleteSelected, duplicateSelected, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";
+import { addFolder, cycleBlendMode, deleteKeyPressed, duplicateSelected, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";
+import { isSelectionTool } from "../tools/selection-draft";
 
 const NUDGE_KEYS: Partial<Record<ActionId, string>> = { "nudge-left": "ArrowLeft", "nudge-right": "ArrowRight", "nudge-up": "ArrowUp", "nudge-down": "ArrowDown" };
 
@@ -17,6 +18,13 @@ export function runAction(id: ActionId, shift = false): void {
   const zoomBy = (f: number) => { if (doc && vp) { vp.setZoom(vp.zoom * f, vp.center, { width: doc.width, height: doc.height }); s.invalidate(); } };
   const nudgeKey = NUDGE_KEYS[id];
   if (nudgeKey) {
+    // In a selection tool the arrows move the outline, 1 px or 10 with Shift, one step a press
+    // (`nudgeSelection`, EditorCanvas.swift:1809-1814); only a selection with something in it moves.
+    if (doc && isSelectionTool(s.tool)) {
+      const delta = nudgeDelta(nudgeKey, shift);
+      if (delta && !s.selectionDraft && s.hasSelection()) s.run({ type: "MoveSelection", dx: delta.dx, dy: delta.dy });
+      return;
+    }
     if (!doc || s.tool !== "move") return;
     const delta = nudgeDelta(nudgeKey, shift);
     if (!delta) return;
@@ -67,16 +75,25 @@ export function runAction(id: ActionId, shift = false): void {
     case "tool-hand": s.setTool("hand"); break;
     case "tool-zoom": s.setTool("zoom"); break;
     case "tool-crop": s.setTool("crop"); break;
+    case "tool-marquee": s.setTool("marquee"); break;
+    case "tool-lasso": s.setTool("lasso"); break;
+    case "tool-wand": s.setTool("wand"); break;
+    case "select-all": if (doc) s.run({ type: "SelectAll" }); break;
+    case "deselect": if (doc?.selection) s.run({ type: "Deselect" }); break;
+    case "select-inverse": if (doc?.selection) s.run({ type: "InvertSelection" }); break;
+    case "cycle-tool-mode": s.cycleToolMode(); break;
     // An open panel answers Enter and Escape itself (AdjustPanel.tsx); neither may also reach the
-    // crop tool or a transform.
+    // crop tool or a transform. An outline being drawn takes them first (EditorCanvas.swift:1772-1777).
     case "apply":
       if (s.panelOwnsDocument()) break;
-      if (doc && s.tool === "crop") { const r = s.cropRect; if (r) { s.run({ type: "Crop", ...r }); s.setCropRect(null); } }
+      if (s.selectionDraft) s.finishSelectionDraft();
+      else if (doc && s.tool === "crop") { const r = s.cropRect; if (r) { s.run({ type: "Crop", ...r }); s.setCropRect(null); } }
       else if (s.transformEdit) s.commitTransform();
       break;
     case "cancel":
       if (s.panelOwnsDocument()) break;
-      if (s.tool === "crop") s.setCropRect(null);
+      if (s.selectionDraft) s.setSelectionDraft(null);
+      else if (s.tool === "crop") s.setCropRect(null);
       else if (s.transformEdit) s.cancelTransform();
       break;
     case "new-folder": addFolder(); break;
@@ -88,7 +105,13 @@ export function runAction(id: ActionId, shift = false): void {
     case "layer-down": moveActiveBy(-1); break;
     case "blend-next": cycleBlendMode(true); break;
     case "blend-prev": cycleBlendMode(false); break;
-    case "delete-layer": if (doc && !s.sheet) deleteSelected(); break;
+    // Backspace / Delete drop the last corner of an outline being drawn; otherwise Delete clears
+    // through a selection, or deletes the layers without one (deleteKeyPressed).
+    case "delete-layer":
+      if (!doc || s.sheet) break;
+      if (s.selectionDraft) { const d = s.selectionDraft; s.setSelectionDraft(d.removeLast() ? d : null); break; }
+      deleteKeyPressed();
+      break;
     case "levels": s.beginAdjust({ kind: "Levels" }); break;
     case "curves": s.beginAdjust({ kind: "Curves" }); break;
     case "hue-saturation": s.beginAdjust({ kind: "Hue/Saturation" }); break;
```

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -17,6 +17,8 @@ import { CanvasSizeSheet } from "./sheets/CanvasSizeSheet";
 import { ImageSizeSheet } from "./sheets/ImageSizeSheet";
 import { JpegExportSheet } from "./sheets/JpegExportSheet";
 import { CropOptions } from "./panels/CropOptions";
+import { SelectionOptions } from "./panels/SelectionOptions";
+import { SelectionAmountSheet } from "./sheets/SelectionAmountSheet";
 import { TransformInspector } from "./panels/TransformInspector";
 import { AdjustPanel } from "./panels/AdjustPanel";
 import { UndrawnNotice } from "./panels/UndrawnNotice";
@@ -85,6 +87,7 @@ export function App() {
       <MenuBar />
       <ProjectTabs />
       <CropOptions />
+      <SelectionOptions />
       <TransformInspector />
       <div className="workspace">
         <ToolRail />
@@ -102,6 +105,7 @@ export function App() {
       {sheet?.kind === "canvasSize" && <CanvasSizeSheet />}
       {sheet?.kind === "imageSize" && <ImageSizeSheet />}
       {sheet?.kind === "jpeg" && <JpegExportSheet />}
+      {sheet?.kind === "selectionAmount" && <SelectionAmountSheet operation={sheet.operation} />}
     </div>
   );
 }
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -131,3 +131,6 @@ body { margin: 0; }
 /* Error banner */
 .error-banner { position: fixed; left: 0; right: 0; bottom: 0; z-index: 200; display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 10px 16px; background: #5a1f1f; color: #fff; }
 .error-banner button { background: none; border: 1px solid #fff; color: #fff; border-radius: 4px; padding: 2px 10px; cursor: pointer; }
+/* The selection tools' header (Phase 4a). */
+.tool-options .segmented button[aria-pressed="true"] { border-color: #3a6ea5; background: #3a6ea5; color: #fff; }
+.tool-options .hint { color: #999; }
```

- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test` (140 (+2)), `pnpm build`.

- [ ] **Step 5: Prove it bites**

(1) Drop `"cycle-tool-mode"`'s binding (`[]`): the keymap test fails. Restore. (2) In `parseAmount`, accept any number (`Number(trimmed)` without the digits check): the sheet test fails on `"2.5"` and `"1e2"`. Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/panels/SelectionOptions.tsx app/src/sheets/SelectionAmountSheet.tsx
git commit -- app/src/panels/SelectionOptions.tsx app/src/sheets/SelectionAmountSheet.tsx app/src/panels/MenuBar.tsx app/src/panels/ToolRail.tsx app/src/panels/LayersList.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/src/App.tsx app/src/styles.css app/tests/unit/keymap.test.ts app/tests/unit/selection-store.test.ts -m "feat: the selection options bar, the Select menu and its amount sheet, keys and thumbnail loading" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 13: End to end: the tools, the menu and the selection-limited edits in the app

Everything the user does with a selection, through real pointer events and keys on a document made in the page: a 64 x 48 canvas, red on the left 32 columns and blue on the right (one merged layer), at 4 CSS px a document pixel, points computed from the viewport.

**Files:**
- Create test: `app/tests/e2e/selection.spec.ts`
- Modify test: `app/tests/e2e/helpers.ts` (`clickMenu` knows "Select")

- [ ] **Step 1: Write the tests**

```diff
--- a/app/tests/e2e/helpers.ts
+++ b/app/tests/e2e/helpers.ts
@@ -6,7 +6,7 @@ import type { Page } from "@playwright/test";
  * is open, so a bare `getByTestId("menu-<id>").click()` finds nothing until the title
  * has been clicked first.
  */
-export async function clickMenu(page: Page, title: "File" | "Edit" | "Layer" | "Filter" | "Image" | "View", id: string): Promise<void> {
+export async function clickMenu(page: Page, title: "File" | "Edit" | "Layer" | "Select" | "Filter" | "Image" | "View", id: string): Promise<void> {
   await page.getByRole("button", { name: title, exact: true }).click();
   await page.getByTestId(`menu-${id}`).click();
 }
```

```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4a: the selection tools and the edits a selection limits, driven through the real canvas,
// keys, menus and panels (engine behaviour is pinned in engine/tests/selection_*.rs).

type Pt = [number, number];

/** A 64 x 48 document with one opaque layer over it: red on the left 32 columns, blue on the right,
 * shown at 4 CSS px per document pixel. */
async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 32, height: 48, color: "#0000ff" });
  await page.evaluate(async ([r, b]) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(r), "Red", { x: 16, y: 24 });
    api.engine.importImage(doc, bytes(b), "Blue", { x: 48, y: 24 });
    api.engine.execute(doc, { type: "MergeLayers", ids: api.engine.state(doc).layers.map((l: any) => l.id) });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, [red, blue]);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const selection = async (page: Page) => (await state(page)).selection;
const undoDepth = async (page: Page) => (await state(page)).undoDepth;
/** Where document point `p` is on screen. */
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
async function drag(page: Page, from: Pt, to: Pt, steps = 6) {
  const a = await client(page, from), b = await client(page, to);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps }); await page.mouse.up();
}
async function click(page: Page, p: Pt) { const c = await client(page, p); await page.mouse.click(c.x, c.y); }
/** The document's own composite at one pixel, premultiplied RGBA. */
const pixel = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, p);
/** The overlay's device pixels along document row `y` from column `x0` to `x1`, as one string. */
const overlayRow = (page: Page, y: number, x0: number, x1: number) => page.evaluate(([y, x0, x1]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const dpr = window.devicePixelRatio || 1;
  const a = vp.viewPoint({ x: x0, y }, { width: d.width, height: d.height }), b = vp.viewPoint({ x: x1, y }, { width: d.width, height: d.height });
  const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
  const data = overlay.getContext("2d")!.getImageData(Math.round(a.x * dpr), Math.round(a.y * dpr) - 1, Math.round((b.x - a.x) * dpr), 3).data;
  return Array.from(data).join(",");
}, [y, x0, x1]);
const painted = (row: string) => row.split(",").some((v, i) => i % 4 === 3 && Number(v) > 0);

test("a Marquee drag selects a whole-pixel box, the ants march around it, and Ctrl+D deselects", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await expect(page.getByTestId("selection-options")).toBeVisible();
  const before = await undoDepth(page);
  await drag(page, [10.3, 12.2], [29.8, 27.6]);
  const s = await selection(page);
  expect(s.bounds).toEqual({ x: 10, y: 12, width: 20, height: 16 });
  expect(await undoDepth(page)).toBe(before + 1);
  // The ants lie on the box's top edge and march: the dash has moved a few ticks later.
  const first = await overlayRow(page, 12, 12, 28);
  expect(painted(first)).toBe(true);
  await page.waitForTimeout(400);
  expect(await overlayRow(page, 12, 12, 28)).not.toBe(first);
  await page.keyboard.press("Control+d");
  expect(await selection(page)).toBeNull();
  expect(painted(await overlayRow(page, 12, 12, 28))).toBe(false);
});

test("Shift adds, Alt subtracts, Tab makes the Marquee an ellipse", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [4, 4], [20, 20]);
  await page.keyboard.down("Shift"); await drag(page, [30, 4], [44, 20]); await page.keyboard.up("Shift");
  expect((await selection(page)).bounds).toEqual({ x: 4, y: 4, width: 40, height: 16 });
  await page.keyboard.down("Alt"); await drag(page, [4, 4], [12, 20]); await page.keyboard.up("Alt");
  expect((await selection(page)).bounds).toEqual({ x: 12, y: 4, width: 32, height: 16 });
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("marquee-ellipse")).toHaveAttribute("aria-pressed", "true");
  await drag(page, [10, 10], [50, 40]);
  const b = (await selection(page)).bounds;
  expect(Math.abs(b.x - 10) < 0.5 && Math.abs(b.y - 10) < 0.5 && Math.abs(b.width - 40) < 0.5 && Math.abs(b.height - 30) < 0.5).toBe(true);
});

test("the Polygonal Lasso adds corners, drops one with Backspace, closes on its first corner, and Escape cancels", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("l");
  await page.getByTestId("lasso-polygonal").click();
  const before = await undoDepth(page);
  await click(page, [10, 10]); await click(page, [50, 10]); await click(page, [30, 30]);
  await page.keyboard.press("Backspace");
  await click(page, [50, 40]); await click(page, [10, 40]);
  expect(await selection(page)).toBeNull();
  await click(page, [10.5, 10.5]); // within 8 view px of the first corner
  const s = await selection(page);
  expect(s.bounds).toEqual({ x: 10, y: 10, width: 40, height: 30 });
  expect(await undoDepth(page)).toBe(before + 1);
  await click(page, [5, 5]); await click(page, [20, 5]);
  await page.keyboard.press("Escape");
  expect((await selection(page)).bounds).toEqual({ x: 10, y: 10, width: 40, height: 30 });
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().selectionDraft)).toBeNull();
});

test("dragging inside the selection moves its outline, arrows nudge it, a click deselects", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  const before = await undoDepth(page);
  await drag(page, [15, 15], [25.4, 19.6]);
  expect((await selection(page)).bounds).toEqual({ x: 20, y: 15, width: 10, height: 10 });
  expect(await undoDepth(page)).toBe(before + 1);
  await page.keyboard.press("ArrowLeft"); await page.keyboard.press("Shift+ArrowDown");
  expect((await selection(page)).bounds).toEqual({ x: 19, y: 25, width: 10, height: 10 });
  await click(page, [24, 30]);
  expect(await selection(page)).toBeNull();
});

test("the Magic Wand selects the clicked colour on this layer, contiguous or everywhere", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("w");
  await click(page, [5, 5]);
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 32, height: 48 });
  await page.getByTestId("wand-contiguous").uncheck();
  await click(page, [50, 5]);
  expect((await selection(page)).bounds).toEqual({ x: 32, y: 0, width: 32, height: 48 });
});

test("Invert, Levels and Delete stay inside the selection; an adjustment layer ignores it", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [0, 0], [16, 48]);
  await page.keyboard.press("Control+i");
  expect(await pixel(page, [4, 4])).toEqual([0, 255, 255, 255]);
  expect(await pixel(page, [20, 4])).toEqual([255, 0, 0, 255]);
  // The GPU draws what the CPU composites.
  const gpu = await page.evaluate(async () => {
    const api = (window as any).__compositor; await api.setZoom(1);
    const px = api.readDocumentPixels() as Uint8Array; const s = api.store.getState(); const w = s.documents[s.activeId].width;
    const at = (x: number, y: number) => Array.from(px.slice((y * w + x) * 4, (y * w + x) * 4 + 4));
    const shown = [at(4, 4), at(20, 4)];
    await api.setZoom(4);
    return shown;
  });
  expect(gpu).toEqual([[0, 255, 255, 255], [255, 0, 0, 255]]);
  await page.keyboard.press("Delete");
  expect((await pixel(page, [4, 4]))[3]).toBe(0);
  expect(await pixel(page, [20, 4])).toEqual([255, 0, 0, 255]);
  expect((await state(page)).layers.length).toBe(1);
  await clickMenu(page, "Layer", "layer-adjustment-invert");
  expect(await pixel(page, [40, 4])).toEqual([255, 255, 0, 255]);
});

test("Add Mask with a selection hides it and uses it up; the Crop tool starts at the selection", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [8, 6], [24, 30]);
  await page.keyboard.press("c");
  expect(await page.evaluate(() => (window as any).__compositor.store.getState().cropRect)).toEqual({ x: 8, y: 6, width: 16, height: 24 });
  await page.keyboard.press("Escape");
  await page.getByTestId("layer-add-mask").click();
  const s = await state(page);
  expect(s.selection).toBeNull();
  expect(s.layers[0].hasMask).toBe(true);
  expect((await pixel(page, [12, 12]))[3]).toBe(0);
  expect((await pixel(page, [40, 40]))[3]).toBe(255);
});

test("the Select menu: All, Inverse, Expand and Feather with their amount, the layer's pixels, and Ctrl-click on a thumbnail", async ({ page }) => {
  await setup(page);
  await clickMenu(page, "Select", "select-all");
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
  await page.keyboard.press("m");
  // A New-mode drag inside the selection would move it, so deselect first.
  await page.keyboard.press("Control+d");
  await drag(page, [10, 10], [20, 20]);
  await clickMenu(page, "Select", "select-expand");
  await page.getByTestId("selection-amount").fill("3");
  await page.keyboard.press("Enter");
  expect((await selection(page)).bounds).toEqual({ x: 7, y: 7, width: 16, height: 16 });
  await clickMenu(page, "Select", "select-feather");
  await expect(page.getByRole("button", { name: "OK" })).toBeEnabled();
  await page.getByTestId("selection-amount").fill("251");
  await expect(page.getByRole("button", { name: "OK" })).toBeDisabled();
  await page.getByTestId("selection-amount").fill("4");
  await page.getByRole("button", { name: "OK" }).click();
  expect((await selection(page)).feather).toBe(4);
  await page.keyboard.press("Control+Shift+i");
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
  const id = (await state(page)).layers[0].id;
  await page.keyboard.down("Control"); await page.getByTestId(`target-pixels-${id}`).click(); await page.keyboard.up("Control");
  expect((await selection(page)).feather).toBe(0);
  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
});

test("an empty selection says so and refuses the edits", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m");
  await drag(page, [10, 10], [20, 20]);
  await page.getByTestId("selection-contract-amount").fill("6");
  await page.getByTestId("selection-contract").click();
  await expect(page.getByTestId("selection-empty")).toBeVisible();
  await page.getByRole("button", { name: "Image", exact: true }).click();
  await expect(page.getByTestId("menu-image-levels")).toBeDisabled();
  await expect(page.getByTestId("menu-image-invert")).toBeDisabled();
});
```

- [ ] **Step 2: Run them**

`pnpm wasm:dev` (if Task 12 left the dev wasm stale), then `npx playwright test app/tests/e2e/selection.spec.ts`: 9 passed (about 25 s on the scratch copy). Each test drives behaviour Tasks 4 to 12 built, so they pass as written; if one fails, the task it names has a defect: fix it there, not here.

Two of these were first written wrong on the scratch copy and show what the Mac's rules do: a New-mode drag that starts inside the selection moves it rather than drawing (hence the Ctrl+D before drawing in the Select-menu test), and the Invert through the GPU must read the canvas at 1:1 (`setZoom(1)`) where `readDocumentPixels` has one device pixel a document pixel.

- [ ] **Step 3: Prove they bite**

(1) In `CanvasView.tsx`'s ants timer, stop the phase moving (`antsPhaseRef.current = antsPhaseRef.current`): the first test fails at "the dash has moved". Measured on the scratch copy. Restore. (2) In `useShortcuts.ts`, drop the selection tools' arrow branch: the nudge test fails (the arrows then do nothing outside the Move tool). Restore. (3) In `CanvasView.tsx`'s pointer-up, send `Deselect` whatever the drag's offset: the drag-to-move test fails. Restore.

- [ ] **Step 4: Commit**

```
git add -- app/tests/e2e/selection.spec.ts
git commit -- app/tests/e2e/selection.spec.ts app/tests/e2e/helpers.ts -m "test: the selection tools, the Select menu and the selection-limited edits end to end" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 14: Docs and version 0.4.0

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-09-20-windows-port-design.md` (section 4.5), `docs/superpowers/phase3.5b-rulings-and-open-items.md` (the follow-up probes' open items), `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`, `Cargo.lock`, `app/tests/e2e/smoke.spec.ts`

- [ ] **Step 1: README, spec and the 3.5b open items**

```diff
--- a/README.md
+++ b/README.md
@@ -71,10 +71,8 @@ Photoshop.
   (a folder does not isolate its contents on its own). Double-clicking an adjustment
   layer's row reopens its panel; double-clicking an ordinary layer's row still renames it.
 - Note: an adjustment layer is never a clipping source.
-- Note: selection-limited adjustments arrive with selections in Phase 4.
 - Note: Hue/Saturation is evaluated per pixel rather than through the Mac's 33-point
   colour cube, so results are slightly more exact than the Mac app's.
-- Note: Motion Blur is an even streak rather than Core Image's tapered one.
 - Note: Image > Grain places its grain in document pixels; the Mac's uses the layer's own
   pixels, so on a scaled layer the grain size differs from the Mac's by the layer's scale.
 
@@ -101,15 +99,14 @@ Photoshop.
 - Image > Black & White and Color Balance, applied to the selected layer.
 - Grain and Add Noise use the Mac 1.2.6 patterns, for layers and for the Image and Filter menus.
 - Folder opacity and saved guides, as the Mac draws and moves them.
-- Note: Motion Blur, as a layer or a filter, is an even streak rather than Core Image's taper.
-  Until that difference is measured, a project with a Motion Blur adjustment layer names it in
-  the notice as drawn approximately, and merging it is refused.
-- Note: a Gaussian Blur reaching more than 48 screen pixels, or a Motion Blur reaching more than
-  12, is computed on a reduced copy. Measured against the exact blur: away from the edges,
-  within 1 level for a Gaussian Blur and 2 for a Motion Blur; up to 4 levels along the canvas
-  edge and along hard edges of transparency; and a long Motion Blur loses fine detail across the
-  streak (up to 20 levels on pixel-sized noise, and 17 at the canvas edge). At export (100%), a
-  Gaussian Blur up to radius 16 and a Motion Blur up to 24 px are exact. Every view of it, and
+- Motion Blur, as a layer or a filter, is Core Image's: a Gaussian along the angle whose sigma is
+  the distance / sqrt(12), within 6 levels of the Mac's own render.
+- Note: a Gaussian Blur or a Motion Blur reaching more than 48 screen pixels (three sigmas: a
+  Motion Blur reaches 0.87 x its distance) is computed on a reduced copy. Measured against the
+  exact blur: away from the edges, within 1 level; up to 4 levels for a Gaussian Blur and 9 for a
+  Motion Blur along the canvas edge and along hard edges of transparency; and a long Motion Blur
+  loses fine detail across its angle (up to 19 levels on pixel-sized noise). At export (100%), a
+  Gaussian Blur up to radius 16 and a Motion Blur up to 55 px are exact. Every view of it, and
   the export, shows the same result. Zoomed far into a very large blur, its edge can show at the
   window's edge.
 - Note: projects are limited to 100 megapixels of layer images (and of masks). Compositor for
@@ -133,6 +130,47 @@ Photoshop.
   3000 x 2000 layer with all six effects. The 8 most recently made images, up to 512 MB, are
   kept.
 
+## Phase 4a: selections
+
+- Marquee (M; Tab switches Rectangle and Ellipse), Lasso (L; Tab switches Freehand and
+  Polygonal) and Magic Wand (W), with New, Add and Subtract in the options bar, or Shift (add)
+  and Alt (subtract) held as a drag or click begins.
+- The Magic Wand's tolerance, sample size (point, 3 by 3, 5 by 5), This Layer or All Layers, and
+  Contiguous. Anti-alias for the Lasso, the Magic Wand and the elliptical Marquee.
+- Drag inside a selection to move its outline; the arrow keys nudge it (Shift = 10 px). A click
+  inside it without a drag deselects (with the Magic Wand, selects afresh from that pixel).
+- The Polygonal Lasso: a click per corner; click the first corner or double-click to close;
+  Backspace removes the last corner, Enter closes, Escape cancels.
+- Select menu: All, Deselect, Inverse, Layer's Pixels, Mask's Black Areas, and Expand, Contract
+  and Feather with an amount; the three are also in the options bar.
+- Ctrl-click a layer's thumbnail to select its pixels (at least half opaque), or a mask's to
+  select its black areas, as on the Mac; Ctrl+Shift adds and Ctrl+Alt subtracts.
+- With a selection: adjustments, filters and Invert change only what is selected (a blur still
+  grows the layer where the selection reaches), and Levels and Curves show the histogram of the
+  selected pixels. Delete clears the selected pixels, or fills a targeted mask white. Add Mask
+  hides the selection (Add Mask (Hide All) shows only it) and uses it up. The Crop tool starts at
+  the selection's bounds. Adjustment layers ignore the selection.
+- An empty selection (after Subtract or Contract) says so in the options bar, and every edit
+  refuses it until it is deselected or replaced.
+- Selections are part of undo and, as on the Mac, are never saved in the project. Crop, Canvas
+  Size and Image Size drop the selection; Flip Canvas mirrors it.
+- Note: dragging a Marquee or an outline past the window's edge does not scroll the view yet.
+- Note: object selection, Select Subject, fills, the clipboard and the brushes are not in this
+  phase.
+
+### Select menu shortcuts
+
+| Action | Shortcut |
+| --- | --- |
+| Marquee / Lasso / Magic Wand | M / L / W |
+| Switch the Marquee's shape or the Lasso's kind | Tab |
+| Select All | Ctrl+A |
+| Deselect | Ctrl+D |
+| Inverse | Ctrl+Shift+I |
+| Add to / subtract from the selection | Shift / Alt while drawing |
+| Nudge the selection (selection tools) | Arrow keys (Shift = 10 px) |
+| Clear the selected pixels | Delete |
+
 ## Prerequisites
 
 - Rust 1.95 with the `wasm32-unknown-unknown` target
@@ -166,14 +204,12 @@ projects from Compositor for Mac 1.2.10 (format version 9) and every earlier for
 writes version 9 as the Mac does, and saves them back without losing anything. What they
 contain is drawn as the Mac draws it, within the reduced-copy blur note above, except:
 
-- Motion Blur adjustment layers, which are drawn approximately and named in the same notice;
-- until follow-up measurements on the Mac are back: Color Burn and Color Dodge on an adjustment
-  layer or a clipped group, where the Mac uses Core Graphics' own formulas and this app the
-  W3C ones; and Soft Light, whose exact variant is not settled yet (within 1 level on the only
-  measurement so far); and the layer effects other than the drop shadow, which follow the Mac's
-  code and its own tests but have not been compared with a Mac render yet, and are written as
-  the Mac's code writes them, not yet checked against a project the Mac itself saved with
-  effects.
+- a layer enlarged in High quality (the default), which Compositor for Mac draws with Core
+  Graphics' high-quality filter and this app bilinearly: sharper soft edges on the Mac, up to 29
+  levels on one probe turned 25 degrees at 150 %, until the resampling probes are measured;
+- layer effects, which match the Mac's renders (11 of the 16 effects probes exactly, 4 within 3
+  levels, and the last apart from the resampling above) but are written as the Mac's code writes
+  them, not yet checked against a project the Mac itself saved with effects.
 
 ## Further reading
 
```

```diff
--- a/docs/superpowers/specs/2026-09-20-windows-port-design.md
+++ b/docs/superpowers/specs/2026-09-20-windows-port-design.md
@@ -140,14 +140,15 @@ are ported to Rust under `engine/src/adjust/`, shared by the CPU compositor and
 live preview path. The WebGL2 renderer does not re-derive the colour tables: the engine
 computes them and uploads them as textures (`adjustment_lut`, `hue_response_table`), so
 only the HSL and grain arithmetic is written twice, once in Rust and once in GLSL, and an
-end-to-end GPU/CPU parity suite holds the two within 2/255 (3/255 for Grain). Two deliberate simplifications from
+end-to-end GPU/CPU parity suite holds the two within 2/255 (3/255 for Grain). One deliberate simplification from
 the macOS behaviour:
 
 - Hue/Saturation is evaluated per pixel rather than through the Mac's 33-point colour
   cube, so results are slightly more exact than the Mac app's.
-- (Until Phase 4a) Motion Blur was an even streak rather than Core Image's tapered one. The Mac 1.2.10
-  probes (2026-09-27) measured CIMotionBlur as a Gaussian along the angle with sigma = distance / sqrt(12),
-  and Phase 4a Task 1 ports that.
+
+Motion Blur was an even streak until Phase 4a. The Mac 1.2.10 probes (2026-09-27) measured
+CIMotionBlur as a Gaussian along the angle with sigma = distance / sqrt(12), and Phase 4a
+Task 2 ports that: one kernel for the filter, the adjustment layer and the GPU.
 
 ## 5. Project format on Windows
 
```

```diff
--- a/docs/superpowers/phase3.5b-rulings-and-open-items.md
+++ b/docs/superpowers/phase3.5b-rulings-and-open-items.md
@@ -68,15 +68,15 @@ For Phase 3.5c (layer effects):
   base's effects today.
 - The `edited-rich-file` probe can only be compared once effects are drawn.
 
-From the follow-up probes (awaiting the user's Mac exports of `build-artifacts/mac-probes/`):
-- Soft Light variant (W3C now; Photoshop and Pegtop still fit the only render within 1).
-- Hard Light, Linear Light and Pin Light on non-pure sources; Hard Mix edge.
-- Color Burn and Color Dodge on adjustment layers and clipped groups: the Mac uses Core Graphics'
-  own formulas there, this port the W3C ones.
-- Motion Blur: even streak vs CIMotionBlur's taper; when settled, drop "drawn approximately" and the
-  merge refusal, or port the taper.
-- Color Balance with preserve on and off, B&W tint, Invert, both noise modes, Gaussian 6 and 40:
-  compare and pin as bit-exact oracles.
+From the follow-up probes: settled by the Mac 1.2.10 exports of 2026-09-27 (probe results;
+pinned in engine/tests/mac_1_2_10.rs by Phase 4a Tasks 1 and 2):
+- Soft Light is Pegtop's formula (the W3C one was 14 levels off at 75 % grey); now within 1.
+- Hard Light, Linear Light and Pin Light on non-pure sources and the Hard Mix edge: within 1.
+- Color Burn and Color Dodge on adjustment layers and clipped groups: bit-identical to the Mac.
+- Motion Blur: CIMotionBlur is a Gaussian along the angle, sigma = distance / sqrt(12); ported, the
+  notice entry and the merge refusal dropped; within 6 levels of the Mac's render (26 before).
+- Color Balance with preserve on and off, Invert, both noise modes: bit-identical; B&W tint within 1;
+  Gaussian 6 and 40 within 2 and 3 premultiplied.
 
 Engineering:
 - A level-0 Gaussian still holds the target plus one working copy (8 B/px); a streaming mix in
```

- [ ] **Step 2: Version 0.4.0**

In `package.json`, `src-tauri/tauri.conf.json`, the workspace `Cargo.toml` (let cargo rewrite `Cargo.lock`: the three workspace packages move to 0.4.0) and the smoke test's literal:

```diff
--- a/package.json
+++ b/package.json
@@ -1,7 +1,7 @@
 {
   "name": "compositor-windows",
   "private": true,
-  "version": "0.3.7",
+  "version": "0.4.0",
   "type": "module",
   "scripts": {
     "wasm": "powershell -ExecutionPolicy Bypass -File scripts/ensure-wasm-pack.ps1 && wasm-pack build engine-wasm --target web --release --out-dir ../app/src/engine/pkg --out-name compositor_engine",
```

```diff
--- a/src-tauri/tauri.conf.json
+++ b/src-tauri/tauri.conf.json
@@ -1,7 +1,7 @@
 {
   "$schema": "https://schema.tauri.app/config/2",
   "productName": "Compositor",
-  "version": "0.3.7",
+  "version": "0.4.0",
   "identifier": "com.compositor.windows",
   "build": {
     "frontendDist": "../app/dist",
```

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -3,7 +3,7 @@ resolver = "2"
 members = ["engine", "engine-wasm", "src-tauri"]
 
 [workspace.package]
-version = "0.3.7"
+version = "0.4.0"
 edition = "2021"
 license = "MIT"
 
```

```diff
--- a/app/tests/e2e/smoke.spec.ts
+++ b/app/tests/e2e/smoke.spec.ts
@@ -2,7 +2,7 @@ import { test, expect } from "@playwright/test";
 
 test("engine loads in the browser", async ({ page }) => {
   await page.goto("/");
-  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.3.7");
+  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.4.0");
   const ids = await page.evaluate(() => {
     const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
     api.engine.newDocument(10, 10, true);
```

Do NOT run `pnpm build:portable`.

- [ ] **Step 3: Full verification (foreground)**

`cargo test` (the workspace), `pnpm wasm`, `pnpm test`, `pnpm build`, `pnpm e2e`. Report the totals against Task 1's baseline. The scratch copy measured: `compositor-engine` 451 passed and 1 ignored (382 before; Tasks 1-7 added 69: 8, 2, 11, 19, 13, 8 and 8); the workspace 458 passed and 1 ignored; vitest 140 (114 before); e2e 128 passed and 2 skipped (117 and 2 before: Task 1 added 1, Task 2 1, Task 13 9); the release wasm 2,542,562 bytes.

- [ ] **Step 4: Commit**

```
git commit -- README.md docs/superpowers/specs/2026-09-20-windows-port-design.md docs/superpowers/phase3.5b-rulings-and-open-items.md package.json src-tauri/tauri.conf.json Cargo.toml Cargo.lock app/tests/e2e/smoke.spec.ts -m "docs: Phase 4a selections in the README, and 0.4.0" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

## Self-review notes

**Spec coverage** (section 3, "4a Selections"), point by point:
- The selection model, a vector outline with anti-alias and feather, undoable, never saved: Task 3 (model, `same_content`), Task 4 (commands, revisions, never in the manifest).
- Rectangle and Ellipse Marquee, Freehand and Polygonal Lasso, Magic Wand: Tasks 4, 5 (engine), 9, 11 (gestures), 12 (tools, keys), 13 (end to end).
- Replace, add and subtract, from the options bar and from Shift / Alt: Tasks 4, 9, 12.
- Move and nudge the outline: Tasks 4 (`MoveSelection`, whole pixels, not re-clipped), 11 (drag), 12 (arrows).
- Select All, Deselect, Inverse, Expand, Contract, Feather, from the options bar and the Select menu with an amount sheet: Tasks 4, 12.
- Anti-alias toggle: Tasks 3 (coverage), 12 (the bar).
- Load a layer's pixels or a mask's black areas (Ctrl-click with Shift / Alt, and the Select menu), following the Mac: Tasks 5, 10, 12.
- Marching ants: Task 11.
- Destructive adjustments, filters (a blur grows the layer and blends on the grown grid) and Invert limited to the selection; Levels / Curves histograms weighted by coverage; adjustment layers never take it: Task 6.
- Delete clears the selected pixels (without a selection it deletes the layer as before): Tasks 7, 10.
- Add Mask from Selection with the Mac's semantics (Reveal with a selection hides the selection): Tasks 7, 10.
- The Crop tool starts at the selection's bounds: Task 10.
- Edits refused on an empty selection: Tasks 6, 7 (engine), 10, 12 (app).
- Ctrl is the Mac's Cmd and Alt its Option throughout (Tasks 9, 12).
- Task 1 as fixed by the brief and the controller's updates: the 31 exports committed by name (the seven `sampling-*` left untracked) and pinned in `mac_1_2_10.rs` at their measured bounds; Soft Light Pegtop in `blend.rs` and GLSL. Task 2: Motion Blur is one Gaussian of sigma distance / sqrt(12) for the filter, the adjustment layer (`streak_for_layer` through `spatial_blur`) and the GPU; reach and padding re-derived (3 sigma, `SPATIAL_REACH_LIMIT`; the filter keeps the Mac's distance / 2 + 2); the canvas-edge rule kept; the notice entry and the merge refusal dropped; spec 4.5 and the 3.5b open items updated in Task 14; release timings in Task 2 Step 4; `mac_probes.rs` untouched.
- Version 0.4.0 and the README: Task 14.

**LL-067 / LL-068 pass: could each assertion fail?** For every test file, the production change that makes it fail, and the fixture choices that keep it from passing by construction:
- `mac_1_2_10.rs`: each probe's bound fails if the drawing moves past what was measured (Task 1 Step 5 (3)); W3C Soft Light fails blend-greys at 14 (Step 5 (1)); the streak fails the motion probe at 26 (Task 2).
- `filters.rs` (Task 2): the dot's profile is computed from the Gaussian in the test, and an even streak gives a different one (16 columns of 16); the stencil test compares with an independent tap-at-a-time transcription at angles on and off the axes.
- `spatial_adjustments.rs`: the level test's two lengths straddle the limit (55 px exact, 56 px halved); the halved bounds were measured on fixtures with an alpha edge off the lattice and odd sizes (95 x 63).
- `selection_model.rs`: the rectangle is 40 x 41 off every axis; the triangle's edge crosses pixel corners so a hard fill must choose (half-open), a soft one gives 128; the ellipse is compared with an independent area computation, not with our own flattening; the feather profile is computed from the formula; the clip is cut by the canvas on one side only (x = -3); the scaled layer is 2x so centres fall on canvas pixel corners and the mean is checkable.
- `selection_commands.rs`: every undo count is against a baseline depth; the Subtract-with-nothing case asserts no step; the revision test asserts a restored revision after undo and a fresh one after a different edit (LL-071); the LOD test's outline has 24,000 teeth, past the 20,000 limit, so the simplification must run (LL-068 (4)).
- `selection_wand.rs`: tolerance is pinned at 32 against pixels 32 and 33 apart, one of them in alpha only; the opacity case uses 0.1, far enough that a wand reading the opacity misses (0.3 did not: found on the scratch copy); the too-detailed case straddles the edge limit (2001 x 2000 refused, 2000 x 2000 traced); the mask loads black areas where Photoshop would load white, so a Photoshop-rule bug fails.
- `selection_edits.rs`: the selection covers 10 of 40 columns, 30 of 100, a left part of a block (x 0..30 against 20..40), never a half, so a mirrored or unclipped edit shows; the Levels preview is compared to the commit byte for byte; the blur test asserts both sides (grew left, did not grow right) and both edges' softness; the refusals are checked with the document and depth unchanged; the preview-key test changes one input at a time.
- `selection_masks.rs`: Clear is checked inside and outside and after undo; the scaled mask case asserts the mask's own width (50, the layer's grid) and three points in different quadrants; the feathered mask's two values are the Gaussian's at 1.5 px either side of the edge.
- `selection-draft.test.ts`: the boxes come from the Mac's tests with fractional points in every direction; the Shift re-arm case presses Shift throughout, then releases and presses it again; the polygon closes only within 8 view px (8.5 is a corner, 5.7 closes).
- `selection-store.test.ts`: every routing case has its opposite in the same test (with and without a selection, empty and not, pixels and mask, visible and hidden, enabled and disabled).
- `ants.test.ts`: the cache test changes the document, the revision and the step one at a time.
- `selection.spec.ts`: commits proven with `undoDepth`; the ants test reads the overlay's pixels twice 400 ms apart, then after Deselect (painted, moving, gone); the GPU is read at 1:1 on two pixels either side of the selection's edge; every drag starts from points computed from the viewport.

**Not covered by a test (disclosed):** a colour adjustment's preview on a layer larger than 4096 px, where the coverage is taken on the reduced copy's grid (the code path is `edit_coverage` with the reduced size and the layer's transform, as for the grown blur grid, which is tested); the Mac's Core Graphics coverage at soft edges (ruling OQ20); release-wasm timings of the Magic Wand and of tracing on large images (not measured).

**Placeholder scan:** every step names its files and every code block is the replayed code; no "TBD", "similar to" or unwritten helper. Each task's counts were read from the replay.

**Type consistency:** the Rust names the app relies on (`SelectionState` camelCase fields, `WandSettings` camelCase, `SelectionMode` and `SelectionShape` variant names, the command `type` tags) match `types.ts`, and `pnpm build` compiles the three TypeScript programs against the generated wasm types at every app task.

**Rulings made in advance:** OQ1 to OQ21 above.
