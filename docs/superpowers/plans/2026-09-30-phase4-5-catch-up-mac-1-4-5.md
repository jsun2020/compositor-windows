# Phase 4.5: Catch Up With Compositor for Mac 1.4.5 - Implementation Plan

## Execution status on the real checkout, 2026-10-02

Tasks 1-8 were already committed on `phase4.5` at starting HEAD `3e69dac`.
The continuation delivered Tasks 9-15. Actual checkout results, the two
isolated performance failures and their one rerun, the full browser timeout
and second full run, native portable UI checks, and remaining Mac acceptance
are recorded in [the delivery report](../phase4-5-rulings-and-open-items.md).
Scratch-clone measurements below remain historical planning evidence.

- [x] Task 9: selection mask reveal/hide and Alt action.
- [x] Task 10: whole-canvas inverse leaves no selection.
- [x] Task 11: Ungroup Layers in the engine, menus and shortcuts.
- [x] Task 12: tab drag reorder with busy guards.
- [x] Task 13: resize-handle snapping.
- [x] Task 14: W/H and aspect lock in the Move bar.
- [x] Task 15: README/spec and 0.6.0; full suites and release timings.
- [x] Controller portable build and real packaged WebView2 interaction checks.

The original per-step checkboxes are retained as the plan's proposed sequence;
validation was consolidated across the tasks, with 23 restored mutation checks
and one local completion commit, rather than seven separate task commits.
This finishes the Phase 4.5 milestone; the rest of Phase 4 and Phase 5 are not
marked complete by this status update.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compositor for Windows catches up with the Mac app the user now runs, Compositor 1.4.5 (project format 11). Tasks 1-2 pin the Phase 4b-1 Mac exports and write the Phase 4.5 Mac probes with their hand steps; Tasks 3-4 are F1 (the job worker returns its result already halved to the level the canvas draws it at, so the UI thread never halves a large result) and the fix wave's residuals; Task 5 opens and saves project format 11 (text colour and font runs kept verbatim, refused as the Mac refuses them, every save written at 11); Tasks 6-10 are the results 1.4.5 changed (Soft Light by Core Image's W3C formula, adjustment layers and clipping-stack bases blended in their real modes, Photoshop's positive saturation, Add Mask revealing the selection with Alt for the opposite, Inverse of a whole-canvas selection leaving none); Tasks 11-14 are the small 1.4.5 behaviours (Ungroup Layers, dragging tabs to reorder them, resize handles that snap as moving does, the Move bar's W and H fields); Task 15 documents it all and makes the version 0.6.0.

**Architecture:** The engine keeps owning every pixel. F1 gives a raster one adopted halving (`RasterInner.adopted`, `Raster::adopt` / `reduced`), which the job worker makes (`run_edit_job(.., out_per_doc)`, `JobOutput.display`, a fourth job buffer) and `install_job` hands to the installed pixels; `Raster::reduced` is the one road to a halving for the GPU upload (`Engine::layer_raster`), the CPU compositor's prefilter and the seeding of an edit inside a selection. Format 11 is `manifest.rs` alone: `CURRENT_VERSION = 11`, `text_is_valid(text, version)` with the run rules of TypeTool.swift and the version gates of ProjectStore.swift. The 1.4.5 results change `blend.rs` and GLSL mode 15 (Soft Light), `plan.rs` (no more `cg_mode`), `adjust/hsv.rs` and its GLSL twin (`adjusted_saturation`), and `ops/selection.rs` (Add Mask's tones, Inverse). The small behaviours add `Command::UngroupLayers` (`ops/hierarchy.rs`), a pure tab-reorder module and `moveTab`, `snappedResizePoint` in `transform-geometry.ts`, and the W / H fields with the aspect lock in `TransformInspector.tsx`.

**Tech Stack:** Rust (`engine`, `engine-wasm`, `src-tauri`), serde / serde_json, TypeScript + React 18 + zustand (`app`), WebGL2, a module Web Worker, vitest, Playwright (the installed Edge for GPU timings).

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md`, section 3 (the decisions of 2026-09-29 and 2026-09-30 after "4d"), with `docs/superpowers/research/mac-1.4.5-delta.md` (the research this plan follows) and `docs/superpowers/phase4b1-rulings-and-open-items.md` (F1's design, the fix wave's residuals, the conventions).

**Oracle:** Compositor 1.4.5 (upstream tag `v1.4.5`, 086f163). Read it with `git -C C:\Users\sr9rfx\.claude-project\Compositor-1.2.10 show v1.4.5:<path>`; that worktree's HEAD is 1.2.10, so never read its files from disk as if they were 1.4.5. Every Mac citation below is `file:line` at v1.4.5 unless it names another version.

**How this plan was made:** every task was built for real on a private clone of this branch (e961db6), committed there as one commit per task and tagged t1-t15; each task's code blocks are generated from those commits (a new file whole, a changed one as its diff), so every line below compiled and passed the whole set on that clone, and every number the prose gives (test counts, timings, the bugs Step 5 introduces and what they fail) is what was measured there. Diffs are against the previous task's end, so the tasks apply in order.
## Global Constraints

- ASCII only in every source file, test, doc and commit message.
- The Mac is the oracle: every Mac behaviour cites v1.4.5 `file:line`. Where 1.4.5 and the spec disagree, the spec's recorded decisions win, and the task says so.
- LL-073: every interactive path this plan adds or changes has a RELEASE timing at 24 MP (6000 x 4000) and at 100 MP (10000 x 10000) with a written budget the test asserts, in `app/tests/e2e/perf-4-5.spec.ts` (new in Task 3, grown by Tasks 8-14). Run it in the installed Edge: `pnpm wasm`, then `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts`, then `$env:PERF = ""` and `pnpm wasm:dev`. Every test logs UNMASKED_RENDERER_WEBGL; SwiftShader or Microsoft Basic Render invalidates a run. A path without a budget needs a ruling saying why (ruling OQ17 lists them).
- Frame gaps are taken with `performance.now()` inside the rAF callback, in contiguous windows (a frame timed inside the install that lands it). Each perf case warms its path up before timing it and logs the cold costs without asserting them (LL-074), opens a fresh page per size, and must pass both alone (`-g "<title>"`) and in the whole file. On this laptop a failing case is re-run once and both runs reported, never widened.
- Tests: every assertion must be able to fail (LL-068), and expected values are computed in the test from the formula, the Mac's own test numbers or the Mac's export, never pasted from a passing run; a measured bound names its measurement. Each task's Step 5 introduces named production bugs and names the tests they fail; the plan says "(measured)" for each one run on the scratch clone.
- Interop fixtures (LL-069): a claim of format compatibility needs a file the Mac saved. The real format-11 save here is `engine/tests/fixtures/mac-4b1-probes/shapes.mac-1.3.7.comp`; the text-run fixtures under `engine/tests/fixtures/hand-written-1.4.5/` are hand-written from the Mac's Codable encoding and are marked so, to be replaced by the user's A1 and A2 saves.
- The ten untracked `sampling-*` pairs under `engine/tests/fixtures/mac-1.2.10-probes/` belong to Phase 3.5d: never add them.
- Build and test from the repository root, in the FOREGROUND with long timeouts, in Windows PowerShell 5.1 (no `&&`, chain with `;`; no `2>&1` on native tools, redirect with `*> file` and read the file). After any change under `engine/src` or `engine-wasm`: `cargo test -p compositor-engine` (timeout at least 900 s), `pnpm wasm:dev` (before build and e2e), `pnpm test`, `pnpm build`, `pnpm e2e` (server 127.0.0.1:1420, one Playwright run at a time), and report the counts seen. `cargo test -p compositor-shell` where a task touches `src-tauri`. The machine is behind a proxy in China: never probe google.com or similar hosts.
- Step 5 runs BEFORE the task's commit, so never revert with `git checkout`. Before adding a bug, `Copy-Item <file> <file>.bak`; to revert, `Move-Item -Force <file>.bak <file>`, then `(Get-Item <file>).LastWriteTime = Get-Date` (cargo keeps a stale build otherwise), then re-run the task's tests.
- Commit with an explicit pathspec: `git add -- <new files>` first, then `git commit -m "<subject>" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- <paths>`, every `-m` before `--`. An implementer on another model writes its own Co-Authored-By line.
- Engine totals at Step 4 are the sum over every test binary `cargo test -p compositor-engine` runs (doc-tests not counted as a binary).
- Do NOT run `pnpm build:portable`: the controller builds the zip.
- Baseline, measured at e961db6 on the scratch clone: `cargo test -p compositor-engine` 546 passed and 10 ignored over 68 test binaries; vitest 240 in 37 files; `pnpm build` clean; e2e 159 passed and 18 skipped; release wasm 2,971,316 bytes (0.5.0). Task 1 Step 1 records them on the real repository before any change.

## Rulings

Every open question the brief, the spec and the research leave is decided here. The implementer follows these; the controller checks the ones marked **(check)**.

**OQ1. Pinning the Phase 4b-1 exports (Task 1).** The five files the user exported for 4b-1 are pinned under a new `engine/tests/fixtures/mac-4b1-probes/` with names that say which Mac made them: `shapes.mac-1.3.7.png` and the saved `shapes.mac-1.3.7.comp` (both exported 2026-09-29 from 1.3.7), `gradient-linear.mac-1.3.7.png`, `gradient-radial.mac-1.3.7.png` (2026-09-29, 1.3.7), and `gradient-over-colour-2.mac-1.3.7-or-1.4.5.png` (2026-09-30; the user had most likely updated to 1.4.5 by then, and the name records the doubt rather than guessing). The source copies stay untracked in `build-artifacts/mac-probes/`. The ten untracked `sampling-*` pairs under `mac-1.2.10-probes/` are Phase 3.5d's and are never added. The shapes are compared with `shape_raster` at the boxes the Mac saved in `shapes.comp` (its manifest is read as JSON, so the test uses the Mac's own numbers), with per-shape bounds on every channel inside each shape's box, measured on the scratch clone and named as measurements: the ellipse within 10, the two rounded rectangles within 4, Line 2 within 6 and Line 3 within 7 (all on anti-aliased edge pixels: 4b-1's OQ11 held the port within 4 of an exact area count, and Core Graphics' coverage is not an exact area); Line 1 exact except its two round ends at x 20 and 140, where the port's end pixel is within 4 of the exact area (0.5 + pi / 8 of a pixel) and the Mac's is darker by at most 29 (Core Graphics fills the cap's sub-pixel sliver; recorded, not copied). The brief quoted ellipse -11, Line 3 -9, Line 2 -7 and Line 1's end +28; the bounds committed are the worst per-channel differences measured on the scratch clone against the pinned files (10, 7, 6 and 29), which differ from the brief's by 1 or 2 **(check)**. Outside every shape both are 0. The gradients are fitted from the exports (least squares, Gauss-Jordan) and compared with `paint_grid` along the fitted line: linear black to white within 1, radial black centre to white within 2 (centre about (125.8, 127.2), radius about 98.7), and over-colour-2 (0000FF to transparent at opacity 0.37 over (204, 77, 51)) within 1; each fit also checks it describes the drag the README asked for (linear: start within 8 px of the left edge and end within 12 of the right; radial: centre within 5 of the guides' crossing and radius within 5 of 100; over-colour-2: both ends within 16 of the edges).

**OQ2. The Phase 4.5 probe set (Task 2).** `mac_probes.rs` writes 13 new probes and a README section, and the README names the Mac as "Compositor for Mac 1.4.5". The probes are B2 `soft-light-dark` (260 x 120, 65 dark backdrop steps under 7 light sources, the region where W3C and Pegtop Soft Light disagree most), B3 `stack-modes` (240 x 160, a clipping stack based in each of the 24 modes, 6 x 4 cells, in the Mac's menu order), B4 `adjustment-modes-soft-light`, `-linear-dodge`, `-vivid-light`, `-hard-mix` (a Levels layer pulling to mid grey, at 60% opacity, over the tonal sweep in each of those modes: Soft Light for Task 6 and three of the eight modes Task 7 changes, one from each family), B5 `hsv-sat-plus-25`, `-plus-50`, `-plus-62`, `-plus-100`, `-minus-50` and `hsv-reds-plus-50` (the four positive amounts the Mac's own tests use, one negative and one range), and B6 `sampling-upright-1to1` (an upright layer at (10.5, 20.25) and a flipped one at (10.25, 44), the half-pixel placements the 1:1 sampling rule is about). A6 `mask-reveal-selection` is written as a canvas with guides (240 x 250, layers "Reveal" at (20, 20) and "Hide" at (20, 130), 200 x 100 each, guides at x 50 / 190 and y 35 / 105 / 145 / 215) and README hand steps, because the marquee and the button click are the Mac's own. A1 `text-runs`, A2 `text-colour-only` and A3 `resaved-edited-rich-file` are hand steps only (the port has no text tool). A5 `port-v11-roundtrip` (Task 5) is the port's own save for the Mac to open.

**OQ3. F1: which level the worker halves to (Task 3) (check).** The design in the 4b-1 rulings says the worker returns "the display level of the texture the canvas already uploaded"; this plan passes the canvas's output scale instead (`outPerDoc = pointsPerPixel * devicePixelRatio`, the level `prefilter_level` picks from it), because a blank layer (ruling C1's fill and gradient) has no texture yet, and a layer drawn at a different zoom since its last upload would get the wrong level. The worker returns no halving for Nearest sampling, for a result inside a pixel-region selection (its halvings are seeded instead), or when the scale is unknown (0). A zoom change after the install recomputes the level from the adopted halving (`Raster::reduced` starts from it whenever it is at or below the level asked for). Budgets, measured on the HD 520 in release wasm: the frame after the result is put back < 100 ms at both sizes (acceptance was 350 / 400; measured 8-39 ms, and once 127 ms at 24 MP Levels in the first case of a whole-file run, re-run once: 11 ms, reported and not widened), the halving inside that frame < 20 ms (0 measured), frame gaps while the worker edits < 100 ms (18-25), `jobInput` and `installJob` < 150 / 450 ms at 24 / 100 MP (measured 65-97 at 24 MP; 273-335 and 244-401 at 100 MP, the 401 in Task 15's whole-file run, the nearest any F1 number came to its budget).

**OQ4. The adopted halving and a later edit (Task 3).** An edit inside a selection seeds its result's halvings from the source's (`seed_halvings`), and now its adopted halving too (`seed_adopted`, recomputing only the selection's block-aligned reach at that level). A level whose width or height falls below 2 is not seeded. Any other edit makes a new raster with no adoption, so a stale halving can never be drawn.

**OQ5. Format 11 (Task 5) (check).** The port reads versions 1 to 11 and always writes 11 (the Mac writes 11 for every save, ProjectStore.swift:141). Text layers' `colorRuns` (format 10) and `fontRuns` (format 11) are kept verbatim through open, edit and save; the port never makes them. A file is refused as the Mac refuses it (TypeTool.swift:43-62, ProjectStore.swift:211-212): runs in a lower version than their gate; an empty list; runs out of order, overlapping, shorter than one unit or ending past the text (UTF-16 units, as NSString counts); a colour run with a channel outside 0-1; a font run whose name is empty, longer than 200 characters or holds a line break (Character.isNewline's set: LF, CR, VT, FF, NEL, U+2028, U+2029). Deviation: the Mac counts a font name's length in Characters (grapheme clusters) and the port counts Unicode scalars, which differ only for names with combining marks; recorded, not ported. JSON numbers that are whole floats (`3.0`) are accepted where the Mac's `Int(exactly:)` accepts them. `QuickLook/Preview.jpg` is ignored on read and not written (saving over a Mac file drops it; the Mac rewrites it on its next save). The version error says "This project uses format version N. This app supports versions 1-11, which Compositor for Mac saves up to version 1.4.5." The interop fixtures under `hand-written-1.4.5/` are hand-written from the Mac's Codable encoding and say so (LL-069); the user's A1 and A2 saves replace them.

**OQ6. Soft Light (Task 6).** Soft Light is the W3C / PDF formula Core Image uses (1.4.5 draws every mode through Core Image), on the CPU and in GLSL. Of the pinned 1.2.10 probes only `blend-greys`' 75% columns change (W3C and Pegtop agree where the source is at most 0.5, and where it is 1 over 0); those columns are held to the formula within 1 and must differ from the 1.2.10 export by at least 4, and are marked awaiting the B1 re-export. `new-blend-modes` is unchanged.

**OQ7. Real modes for adjustment layers and stack bases (Task 7).** `cg_mode` is deleted: an adjustment layer blends in its own mode and a clipping stack composites in its base's own mode (LiveMaskRenderer.swift:52-57, :126-134). The pins that followed 1.2.10's Normal drawing (`cgmode-levels-divide`, `cgmode-stack-bases`, the Linear Burn blur) now assert the formula and a measurable difference from the 1.2.10 export, and are marked awaiting the B1 re-exports; `color-dodge-adjustment` is unchanged (Color Dodge was already drawn in its mode). The GPU is held to the CPU within 2 in all eight Core-Image-only modes, except a stack based in Vivid Light, within 3 (measured: Vivid Light divides by what is left of the group, so the 8-bit group surface's one level becomes three).

**OQ8. Positive saturation (Task 8).** `adjusted_saturation` is HueSaturation.swift:342-348 exactly, on the CPU and in GLSL; the port keeps its per-pixel evaluation (spec 4.5's recorded deviation from the Mac's 33-point cube). Budgets for a frame of a document with every changed result in it: < 80 ms per frame and < 100 ms per Saturation step at 24 and 100 MP (measured 24-42 and 26-43; one first step at 100 MP took 834 ms before a warm-up step was added, which is now logged as cold).

**OQ9. Add Mask with a selection (Task 9) (check).** The user's ruling of 2026-09-30 replaces the 2026-09-27 rule: the button and Add Mask (Reveal All) with a selection reveal it ("Reveal Selection"), Alt-click and Add Mask (Hide All) hide it ("Hide Selection"); without a selection the undo names become "Add Reveal-All Mask" / "Add Hide-All Mask" (LayerMask.swift:261, :274). An empty selection makes a plain black mask when revealing and a white one when hiding. The footer button's tooltip follows LayerMaskMenu.swift:13-14 with "Alt-click" for "Option-click". The step keeps its Phase 4a cost (the clip rasterized at the layer's size on the UI thread): budgets < 400 / 1000 ms per step and < 150 / 400 ms for the frame after (measured 162-212 / 506-545 and 19-20 / 59-77). Moving it to the worker is not in scope.

**OQ10. Inverse of everything (Task 10).** Inverse whose result is empty leaves no selection (Selection.swift:354-362), as one step; Inverse with no selection still does nothing and records nothing.

**OQ11. Ungroup Layers (Task 11).** Ported exactly, with two recorded differences forced by the port: a folder's blend mode is always Normal here (pass-through), so "the folder's blend mode goes" has nothing to do; and the Mac's tests clip to blank layers, which the port cannot (its blank layers have no pixels), so the ported tests use painted layers. The action needs a folder as the active layer and no open panel (`canUngroupLayers`); a running job refuses it as every command is refused (`BUSY_MESSAGE`). The folder's collapsed state is dropped with it. Budgets: step < 16 ms, frame after < 33 ms (measured 2-5 and 10-13).
**OQ12. Dragging tabs (Task 12) (check).** Ported from ProjectTabs.swift:21-205 and ProjectWorkspace.swift:47-55 as a pure module (`app/src/panels/tab-reorder.ts`: the drag state, `nearestSlot`, `renderX`, `commitTarget`, `reorderedTabs`) and the strip's pointer handlers. A press that moves 3 px becomes a drag and selects its tab; let go, the tab lands in the nearest gap; the others slide aside while it moves. A drag starts only when no job's result is to come and no project operation runs, and one that ends in that state puts the tabs back, as the Mac's `canSwitch` does; the port's tab click itself stays as it is (it drops an open panel rather than being refused, a Phase 2 behaviour this plan does not change). Reordering is chrome: no undo step, and the documents are untouched. Not ported: the Mac's overflow into an "N more tabs" menu (ProjectTabLayout.swift), since the port's strip scrolls; recorded as an open item.

**OQ13. Resize-handle snapping (Task 13) (check).** `snappedResizePoint` (Crop.swift:139-182) is ported to `transform-geometry.ts` and runs for every resize-handle drag with the same targets and tolerance as a move (10 screen px): each moved edge snaps on its own; kept proportional only the nearer edge snaps (the first on a tie, as Swift's `min(by:)` keeps it); Alt (from the centre) snaps too; a turned layer never snaps. The Mac's Control-drag (drag freely) has no key here: Ctrl is this port's Command, and a Ctrl-press on a corner already starts a distortion (`startMode`), so snapping cannot be turned off during a resize, as a move's cannot today. Not ported (a 1.2.10 behaviour the port never had, out of this phase): dragging a handle past the opposite side turns the layer over (LayerTransform.swift:183-205); the port stops at 1 px.

**OQ14. The Move bar's W and H, and the aspect lock (Task 14).** W and H fields (TransformInspector.swift:24-25, :89-100): typing a size keeps the origin (the top-left of the upright box) and, with the lock on, scales the other side by the same factor; a value under 1 changes nothing. A lock toggle beside them (on by default, `locksTransformRatio = true`, EditorSession.swift:197) sets whether a corner or side handle keeps the ratio, Shift turning it the other way while dragging, and the button shows it turned while Shift is held. One lock for the whole app (the Mac keeps one per window); it is not saved.

**OQ15. "Shift keeps moved pixels on a straight line" is not in this phase.** It is about moving selected pixels (EditorCanvas.swift:1873-1877), which is Phase 4b-2's; a layer move with Shift already holds one axis here (`transformDrag`'s `move`). It goes into 4b-2's scope.

**OQ16. Timings.** Every interactive path this plan adds or changes has a case in `perf-4-5.spec.ts` with its budget: F1 (Task 3), the frame of every 1.4.5 result and a Saturation step (Task 8), Add Mask with a selection (Task 9), Inverse (Task 10), Ungroup (Task 11), a tab drag (Task 12), a resize-handle drag with snapping (Task 13), a typed W (Task 14). Budgets follow 4b-1's: 16 ms for a step inside a frame, 33 ms for a frame, 50 ms for a drag tick, 150 / 400 ms for a frame that uploads a whole mask; anything else is written from the measurement with room for this laptop, and says so.

**OQ17. Paths with no budget.** Opening and saving a project (format 11, Task 5) is not interactive and keeps its Phase 3.5a treatment; a refused file costs a parse. Soft Light's and the real modes' formulas (Tasks 6-7) have no path of their own: their frame cost is in Task 8's case. A tab click without a drag is unchanged.

**OQ18. Out of scope, recorded.** Showing a mask alone (Option-click on its thumbnail), mask thumbnails' background tone, the refusal messages of a brush or gradient that cannot start, the picker's new targets and label scrubbing, Auto Select, the tab overflow menu, snapping for Marquee and shape starts, and flipping by dragging past the opposite side (mac-1.4.5-delta.md sections 2.4-2.8). 4c is re-researched against 1.4.5.

**OQ19. Open item found while building Task 3 (pre-existing).** A layer overhanging an 80 x 70 canvas at zoom 0.25 draws differently on the GPU and the CPU (255 at its edge), with F1 turned off as with it on; F1's e2e uses a layer covering the canvas instead. Not fixed here; it belongs with Phase 3.5d's resampling work.
## Mac exports this phase asks for

Task 2 (and Task 5 for A5) writes the probes to `build-artifacts/mac-probes/` with a README; the user opens them in Compositor for Mac 1.4.5 and sends back `mac-exports/`. Nothing in this plan waits for them: the tasks pin what they can now (the formulas, the Mac's own test numbers, the 4b-1 exports) and mark each pin a re-export will settle. File names are exact; a `.comp` that comes back is a saved project folder, zipped.

**B1, re-exported from 1.4.5 (six PNGs; the `.comp` files are the ones already sent for 1.2.10):**
`blend-greys.png`, `new-blend-modes.png`, `cgmode-blur-linear-burn.png`, `cgmode-levels-divide.png`, `color-dodge-adjustment.png`, `cgmode-stack-bases.png`. They settle Tasks 6 and 7's pins marked "awaiting B1".

**New probes, exported as PNG (13):**
- B2 `soft-light-dark.png` (Task 6)
- B3 `stack-modes.png` (Task 7)
- B4 `adjustment-modes-soft-light.png`, `adjustment-modes-linear-dodge.png`, `adjustment-modes-vivid-light.png`, `adjustment-modes-hard-mix.png` (Tasks 6-7)
- B5 `hsv-sat-plus-25.png`, `hsv-sat-plus-50.png`, `hsv-sat-plus-62.png`, `hsv-sat-plus-100.png`, `hsv-sat-minus-50.png`, `hsv-reds-plus-50.png` (Task 8)
- B6 `sampling-upright-1to1.png` (Phase 3.5d; written now because the user is on 1.4.5 now)

**Finished by hand on the Mac, saved and exported (the README's steps):**
- A6 `mask-reveal-selection.comp` and `.png` (Task 9: the elliptical marquee between the guides, the Add Mask button on "Reveal", Option-click on "Hide")
- A1 `text-runs.comp` and `.png` (Task 5: "Hello World" with World red and Hello in another face, and a plain "Plain")
- A2 `text-colour-only.comp` and `.png` (Task 5: colour runs without font runs)
- A3 `resaved-edited-rich-file.comp` and `.png` (Task 5: `edited-rich-file.comp` opened and saved again by 1.4.5, unchanged)

**The port's own save, for the Mac to open (Task 5):**
- A5 `port-v11-roundtrip.comp`: must open without an error; export `port-v11-roundtrip.png`, then Save As `port-v11-roundtrip.resaved.comp` and send both.

In all: 19 PNG exports of port-written probes, four hand-made projects with their PNGs, and A5's export and re-save.

## File structure

| File | Task | What changes |
| --- | --- | --- |
| `engine/tests/fixtures/mac-4b1-probes/` | 1 | Create: the Mac's 4b-1 exports and saved `shapes.comp`, copied |
| `engine/tests/mac_4b1_probes.rs` | 1 | Create: shapes at their saved boxes, gradients fitted from the exports |
| `.gitattributes` | 1 | The new fixtures are binary as saved |
| `engine/tests/mac_probes.rs` | 2, 5 | The Phase 4.5 probes and README; A5 at format 11 |
| `engine/src/raster.rs` | 3 | The adopted halving: `adopt`, `adopted`, `reduced`, `seed_adopted` |
| `engine/src/jobs.rs`, `engine-wasm/src/lib.rs` | 3 | `run_edit_job(.., out_per_doc)`, `JobOutput.display`, `install_job(.., display)`, the fourth job buffer |
| `engine/src/engine.rs`, `engine/src/compositor.rs` | 3, 7, 11 | Halvings through `reduced`; real modes; `UngroupLayers` |
| `app/src/engine/jobs.ts`, `job-worker.ts`, `client.ts` | 3 | `outPerDoc` out, the halving back |
| `app/src/state/store.ts` | 3, 4, 12, 14 | `runEditJob`'s scale; the busy guards; `moveTab`; `locksTransformRatio` |
| `app/src/App.tsx`, `app/src/canvas/effects-images.ts`, `renderer.ts`, `gl-renderer.ts`, `CanvasView.tsx` | 4, 13, 14 | The worker starts last; failed effects images said; resize snapping; the lock |
| `engine/src/manifest.rs`, `engine/src/error.rs` | 5, 7 | Format 11 and its run rules; `cg_mode` deleted |
| `src-tauri/src/commands/package.rs`, `src-tauri/src/atomic.rs` | 5 | Tests: a Mac save opens, QuickLook left alone, dropped on save over |
| `engine/src/blend.rs`, `app/src/canvas/gl/programs.ts` | 6, 7, 8 | W3C Soft Light; comments; `adjustedSaturation` |
| `engine/src/plan.rs` | 7 | Adjustment layers and stack bases in their own modes |
| `engine/src/adjust/hsv.rs` | 8 | `adjusted_saturation` |
| `engine/src/ops/selection.rs`, `engine/src/command.rs` | 9, 10, 11 | Add Mask's tones and names; Inverse of everything; `UngroupLayers` |
| `engine/src/ops/hierarchy.rs` | 11 | `ungroup_layers` |
| `app/src/actions/layers.ts`, `app/src/shortcuts/keymap.ts`, `useShortcuts.ts`, `app/src/panels/MenuBar.tsx`, `LayersList.tsx`, `app/src/engine/types.ts` | 9, 11 | The mask button's Alt; Ungroup's action, key, menu item and context menu |
| `app/src/state/tab-reorder.ts`, `app/src/panels/ProjectTabs.tsx`, `app/src/styles.css` | 12, 14 | Create the tab drag's state; the strip's handlers; unselectable titles; the one-row Move bar |
| `app/src/tools/transform-geometry.ts`, `transform-session.ts` | 13, 14 | `snappedResizePoint`, `SessionInit.lockRatio`; `resizedTo` |
| `app/src/panels/TransformInspector.tsx` | 14 | W, H and the lock; Cancel and Apply kept in place |
| `app/tests/e2e/perf-4-5.spec.ts` | 3, 8-14 | Create: every timing this plan budgets |
| `README.md`, `package.json`, `Cargo.toml`, `Cargo.lock`, `src-tauri/tauri.conf.json`, `app/tests/e2e/smoke.spec.ts`, `docs/superpowers/specs/2026-09-20-windows-port-design.md` | 15 | Phase 4.5 documented; 0.6.0; the spec's stale format wording |

Tests created: `engine/tests/mac_4b1_probes.rs`, `display_level.rs`, `format_v11.rs`, `ungroup.rs`, the two `hand-written-1.4.5` manifests; `app/tests/unit/tab-reorder.test.ts`; `app/tests/e2e/perf-4-5.spec.ts`, `format-v11.spec.ts`, `modes-1-4-5.spec.ts`, `ungroup.spec.ts`, `tabs.spec.ts`. Every other test file a task touches is listed in that task.

---

### Task 1: Pin the Phase 4b-1 Mac exports

The user exported five probes for Phase 4b-1 (shapes and three gradients) and saved `shapes.comp`; they sit untracked in `build-artifacts/mac-probes/`. This task commits them under names that say which Mac made them and holds the port's Shape tool and Gradient to them (ruling OQ1). It changes no production code: it pins what 4b-1 built.

**Files:**
- Create: `engine/tests/fixtures/mac-4b1-probes/` (the Mac's files, copied), `engine/tests/mac_4b1_probes.rs`
- Modify: `.gitattributes` (the fixtures are binary as saved)

**Mac:** ShapeTool.swift:136-145 (a line's ends are fractions of its box); the exports themselves.

- [ ] **Step 1: Record the baseline on the real repository**

Run the whole set once on `phase4.5` at e961db6 before any change and note the counts (Global Constraints give the scratch clone's: 546 passed and 10 ignored over 68 binaries; vitest 240 in 37 files; build clean; e2e 159 passed and 18 skipped).

- [ ] **Step 2: Copy the Mac's files in**

```powershell
New-Item -ItemType Directory -Force engine\tests\fixtures\mac-4b1-probes
$src = "build-artifacts\mac-probes"; $dst = "engine\tests\fixtures\mac-4b1-probes"
Copy-Item "$src\shapes.png" "$dst\shapes.mac-1.3.7.png"
Copy-Item -Recurse "$src\shapes.comp" "$dst\shapes.mac-1.3.7.comp"
Copy-Item "$src\gradient-linear.png" "$dst\gradient-linear.mac-1.3.7.png"
Copy-Item "$src\gradient-radial.png" "$dst\gradient-radial.mac-1.3.7.png"
Copy-Item "$src\gradient-over-colour-2.png" "$dst\gradient-over-colour-2.mac-1.3.7-or-1.4.5.png"
Get-ChildItem -Recurse $dst | Select-Object FullName, Length
```

Expected: `shapes.mac-1.3.7.comp` holds `manifest.json` (version 11), seven images and `QuickLook/Preview.jpg`; the PNGs are 8,642 (shapes), 2,421 (linear), 22,258 (radial) and 3,264 (over-colour-2) bytes. Do not copy `gradient-over-colour.png` (the first attempt, superseded by `-2`) or any `sampling-*` file.

- [ ] **Step 3: Write the tests**

Create `engine/tests/mac_4b1_probes.rs`:

```rust
//! The Phase 4b-1 Mac probes (Phase 4.5, Task 1): `shapes.png`, `gradient-linear.png` and
//! `gradient-radial.png` exported by Compositor for Mac 1.3.7 on 2026-09-29, and
//! `gradient-over-colour-2.png` exported on 2026-09-30 by 1.3.7 or 1.4.5 (the user moved to 1.4.5
//! that day; neither the Shape tool nor the Gradient changed between the two, mac-1.4.5-delta.md 2.5
//! and 2.9), committed under tests/fixtures/mac-4b1-probes with the saved `shapes.comp`. That project
//! is format 11, which this build opens only from Task 5 on, so its manifest is read here as JSON.
//! The shapes are drawn by this port's `shape_raster` at the boxes the Mac saved; the gradients'
//! lines were dragged by hand and are not saved, so each is fitted from the export itself.
use compositor_engine::ops::shape::shape_raster;
use compositor_engine::*;
use serde_json::Value;

fn fixture(name: &str) -> String { format!("{}/tests/fixtures/mac-4b1-probes/{name}", env!("CARGO_MANIFEST_DIR")) }

/// The Mac's export: width, height and straight RGBA8 as the PNG stores it.
fn mac(name: &str) -> (u32, u32, Vec<u8>) {
    let rgba = image::load_from_memory(&std::fs::read(fixture(name)).unwrap()).unwrap().to_rgba8();
    (rgba.width(), rgba.height(), rgba.into_raw())
}

/// Each shape layer of the saved `shapes.comp`, bottom to top: its name, the shape its box and its
/// `shape` record describe (a line's ends are fractions of its box, ShapeTool.swift:136-145), and its
/// colour.
fn saved_shapes() -> Vec<(String, ShapeSpec, [f64; 3])> {
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(fixture("shapes.mac-1.3.7.comp/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["version"], 11, "saved by a Mac that writes format 11");
    manifest["layers"].as_array().unwrap().iter().filter(|l| l.get("shape").is_some()).map(|l| {
        let (t, s) = (&l["transform"], &l["shape"]);
        let n = |v: &Value| v.as_f64().unwrap();
        let rect = Rect { x: n(&t["origin"][0]), y: n(&t["origin"][1]), width: n(&t["size"][0]), height: n(&t["size"][1]) };
        let spec = match s["kind"].as_str().unwrap() {
            "Ellipse" => ShapeSpec::Ellipse { rect },
            "Rectangle" => ShapeSpec::Rectangle { rect, corner_radius: n(&s["cornerRadius"]) },
            "Line" => {
                let at = |key: &str| Point { x: rect.x + n(&s[key][0]) * rect.width, y: rect.y + n(&s[key][1]) * rect.height };
                ShapeSpec::Line { start: at("start"), end: at("end"), width: n(&s["lineWidth"]) }
            }
            other => panic!("no shape {other}"),
        };
        (l["name"].as_str().unwrap().to_string(), spec, [n(&s["red"]), n(&s["green"]), n(&s["blue"])])
    }).collect()
}

#[test]
fn the_mac_saved_the_shapes_the_probe_asked_for() {
    // mac_probes.rs `shapes_doc`: the boxes it wrote, which the Mac redrew and saved as they were.
    let shapes = saved_shapes();
    let boxes: Vec<(&str, [f64; 4])> = shapes.iter().map(|(name, spec, _)| { let b = spec.bounds(); (name.as_str(), [b.x, b.y, b.width, b.height]) }).collect();
    assert_eq!(boxes, [("Ellipse 1", [10.0, 10.0, 101.0, 61.0]), ("Rectangle 1", [130.0, 10.0, 120.0, 80.0]), ("Rectangle 2", [270.0, 10.0, 150.0, 40.0]),
        ("Line 1", [20.0, 120.0, 121.0, 1.0]), ("Line 2", [158.0, 108.0, 74.0, 74.0]), ("Line 3", [255.0, 120.0, 155.0, 95.0])]);
    let ShapeSpec::Line { start, end, width } = &shapes[5].1 else { panic!("Line 3 is a line") };
    assert!((start.x - 262.5).abs() < 1e-9 && (start.y - 127.5).abs() < 1e-9 && (end.x - 402.5).abs() < 1e-9 && (end.y - 207.5).abs() < 1e-9 && *width == 15.0);
}

#[test]
fn every_shape_matches_the_mac_render_within_its_own_measured_edge_bound() {
    // This port's shapes over white, composited as the Mac exported them. Per-shape bounds, measured
    // on p45-scratch (2026-09-30): the ellipse 10, the two rounded rectangles 4, Line 2 6 and Line 3
    // 7 (all on anti-aliased edge pixels; 4b-1's OQ11 held the port within 4 of an exact area count, and Core
    // Graphics' own coverage is not an exact area); Line 1 is exact but at its two end pixels, where
    // Core Graphics fills the round cap's sub-pixel sliver fully (29 levels darker). Nothing differs
    // outside the shapes' boxes.
    let (w, h, theirs) = mac("shapes.mac-1.3.7.png");
    let shapes = saved_shapes();
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::with_pixels("Background", Raster::from_premultiplied(w, h, vec![255; (w * h * 4) as usize]), Point { x: 0.0, y: 0.0 })];
    for (name, spec, color) in &shapes {
        let b = spec.bounds();
        doc.layers.push(Layer::with_pixels(name, shape_raster(spec, *color), Point { x: b.x, y: b.y }));
    }
    let ours = composite(&doc, Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, w, h).to_straight();
    let at = |x: u32, y: u32, c: usize| ((y * w + x) * 4) as usize + c;
    let inside = |spec: &ShapeSpec, x: u32, y: u32| { let b = spec.bounds(); x as f64 + 1.0 > b.x && (x as f64) < b.max_x() && y as f64 + 1.0 > b.y && (y as f64) < b.max_y() };
    let bound = |name: &str| match name { "Ellipse 1" => 10, "Rectangle 1" | "Rectangle 2" => 4, "Line 2" => 6, "Line 3" => 7, "Line 1" => 0, other => panic!("{other}") };
    for (name, spec, _) in &shapes {
        let mut worst = (0u8, (0, 0));
        for y in 0..h { for x in 0..w {
            if !inside(spec, x, y) { continue; }
            // Line 1's two end pixels are checked below.
            if name == "Line 1" && (x == 20 || x == 140) { continue; }
            for c in 0..4 { let d = ours[at(x, y, c)].abs_diff(theirs[at(x, y, c)]); if d > worst.0 { worst = (d, (x, y)); } }
        }}
        assert!(worst.0 <= bound(name), "{name}: {} at {:?}, bound {}", worst.0, worst.1, bound(name));
    }
    // Line 1 (red, 1 px, y 120.5 from x 20.5 to 140.5): each end pixel holds half a pixel of line and a
    // half-disc cap of radius 0.5, 0.5 + pi / 8 of the pixel, which the port covers within 4b-1's OQ11's 4
    // levels; the Mac covers more.
    for x in [20, 140] {
        let (mac_green, port_green) = (theirs[at(x, 120, 1)], ours[at(x, 120, 1)]);
        let exact = 255.0 * (1.0 - (0.5 + std::f64::consts::PI / 8.0));
        assert!((port_green as f64 - exact).abs() <= 4.0, "the port's end pixel at x {x}: {port_green}, the area {exact:.1} (4b-1's OQ11: within 4)");
        assert!(mac_green < port_green && port_green - mac_green <= 29, "Line 1 at x {x}: the Mac {mac_green}, the port {port_green}");
    }
    let outside = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| !shapes.iter().any(|(_, s, _)| inside(s, x, y)))
        .map(|(x, y)| (0..4).map(|c| ours[at(x, y, c)].abs_diff(theirs[at(x, y, c)])).max().unwrap()).max().unwrap();
    assert_eq!(outside, 0, "white where no shape is");
}

/// Least squares of `ys` on `xs`, each row of `xs` one sample's regressors: the coefficients.
fn least_squares(xs: &[Vec<f64>], ys: &[f64]) -> Vec<f64> {
    let n = xs[0].len();
    // The normal equations, solved by Gauss-Jordan elimination with partial pivoting.
    let mut m = vec![vec![0.0; n + 1]; n];
    for (row, y) in xs.iter().zip(ys) {
        for i in 0..n { for j in 0..n { m[i][j] += row[i] * row[j]; } m[i][n] += row[i] * y; }
    }
    for col in 0..n {
        let pivot = (col..n).max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs())).unwrap();
        m.swap(col, pivot);
        for r in 0..n { if r != col { let f = m[r][col] / m[col][col]; for c in col..=n { m[r][c] -= f * m[col][c]; } } }
    }
    (0..n).map(|i| m[i][n] / m[i][i]).collect()
}

/// The line a linear gradient runs along, from `t`, the fraction of the way along it, at each pixel
/// column's centre: `t = (x + 0.5 - s) / (e - s)` fitted to the columns where `t` lies inside (0, 1).
fn fit_ends(ts: &[(u32, f64)]) -> (f64, f64) {
    let inner: Vec<&(u32, f64)> = ts.iter().filter(|(_, t)| *t > 0.03 && *t < 0.97).collect();
    let c = least_squares(&inner.iter().map(|(x, _)| vec![*x as f64 + 0.5, 1.0]).collect::<Vec<_>>(), &inner.iter().map(|(_, t)| *t).collect::<Vec<_>>());
    let s = -c[1] / c[0];
    (s, s + 1.0 / c[0])
}

/// This port's composite of `doc` after the gradient, straight RGBA8, and the worst difference from
/// the Mac's export.
fn paint_and_compare(mut doc: Document, gradient: GradientSpec, theirs: &[u8]) -> u8 {
    let (w, h) = (doc.width, doc.height);
    let layer = doc.layers[0].id;
    doc.active_layer_id = Some(layer);
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    e.execute(id, Command::Gradient { id: layer, mask: false, gradient }).unwrap();
    let ours = e.composite(id, Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, w, h).unwrap().to_straight();
    assert_eq!(ours.len(), theirs.len());
    ours.iter().zip(theirs).map(|(a, b)| a.abs_diff(*b)).max().unwrap()
}

const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];

#[test]
fn the_linear_gradient_matches_the_mac_render_along_the_line_fitted_from_it() {
    // Black to white on a blank 512 x 32 layer, dragged by hand from the left edge to the right with
    // Shift: every row alike, the line found from the ramp. Core Graphics steps its ramp (runs of four
    // equal levels); this port's exact ramp is within 1 level of it everywhere (measured on
    // p45-scratch, 2026-09-30).
    let (w, h, theirs) = mac("gradient-linear.mac-1.3.7.png");
    assert_eq!((w, h), (512, 32));
    let ts: Vec<(u32, f64)> = (0..w).map(|x| (x, theirs[((16 * w + x) * 4) as usize] as f64 / 255.0)).collect();
    let (s, e) = fit_ends(&ts);
    assert!(s.abs() < 8.0 && (e - 512.0).abs() < 12.0, "dragged from edge to edge: {s:.2} to {e:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::blank("Layer 1", doc.size())];
    let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: s, y: 16.0 }, end: Point { x: e, y: 16.0 }, from: BLACK, to: WHITE, opacity: 1.0 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 1, "worst {worst} along {s:.3} to {e:.3}");
}

#[test]
fn the_radial_gradient_matches_the_mac_render_about_the_centre_and_rim_fitted_from_it() {
    // Black at the centre to white at the rim on a blank 256 x 256 layer, dragged by hand from where
    // the guides cross (128, 128) to the guide at x 228. The value at a pixel is 255 d / r, d its
    // centre's distance from the gradient's: (x - cx)^2 + (y - cy)^2 = (v r / 255)^2, which is linear in
    // cx, cy, cx^2 + cy^2 and r^2, fitted where the ramp is inside (0, 1). Measured 2 levels at worst
    // (p45-scratch, 2026-09-30): Core Graphics steps its radial ramp as it does the linear one.
    let (w, h, theirs) = mac("gradient-radial.mac-1.3.7.png");
    assert_eq!((w, h), (256, 256));
    let (mut rows, mut ys) = (Vec::new(), Vec::new());
    for y in 0..h { for x in 0..w {
        let v = theirs[((y * w + x) * 4) as usize] as f64 / 255.0;
        if v <= 0.06 || v >= 0.94 { continue; }
        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
        rows.push(vec![2.0 * px, 2.0 * py, -1.0, v * v]);
        ys.push(px * px + py * py);
    }}
    let c = least_squares(&rows, &ys);
    let (cx, cy, r) = (c[0], c[1], c[3].sqrt());
    assert!((cx - 128.0).abs() < 5.0 && (cy - 128.0).abs() < 5.0 && (r - 100.0).abs() < 5.0, "dragged from the guides' crossing to the guide 100 px right: ({cx:.2}, {cy:.2}) r {r:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::blank("Layer 1", doc.size())];
    let gradient = GradientSpec { shape: GradientShape::Radial, start: Point { x: cx, y: cy }, end: Point { x: cx + r, y: cy }, from: BLACK, to: WHITE, opacity: 1.0 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 2, "worst {worst} about ({cx:.3}, {cy:.3}) r {r:.3}");
}

#[test]
fn the_translucent_gradient_over_a_colour_matches_the_mac_render_along_the_line_fitted_from_it() {
    // 0000FF to transparent at 37 % opacity, dragged by hand across a solid (204, 77, 51) layer,
    // painted into that layer (source-over, as the Mac's raster edit paints). The blue's alpha at a
    // column is 0.37 (1 - t), which the red channel shows: red = 204 (1 - alpha). Measured 1 level at
    // worst (p45-scratch, 2026-09-30).
    let (w, h, theirs) = mac("gradient-over-colour-2.mac-1.3.7-or-1.4.5.png");
    assert_eq!((w, h), (256, 32));
    let ts: Vec<(u32, f64)> = (0..w).map(|x| (x, 1.0 - (1.0 - theirs[((16 * w + x) * 4) as usize] as f64 / 204.0) / 0.37)).collect();
    let (s, e) = fit_ends(&ts);
    assert!(s.abs() < 16.0 && (e - 256.0).abs() < 16.0, "dragged from edge to edge: {s:.2} to {e:.2}");
    let mut doc = Document::new(w, h);
    doc.layers = vec![Layer::with_pixels("Layer 1", Raster::from_premultiplied(w, h, [204u8, 77, 51, 255].repeat((w * h) as usize)), Point { x: 0.0, y: 0.0 })];
    let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: s, y: 16.0 }, end: Point { x: e, y: 16.0 },
        from: [0.0, 0.0, 1.0, 1.0], to: [0.0, 0.0, 1.0, 0.0], opacity: 0.37 };
    let worst = paint_and_compare(doc, gradient, &theirs);
    assert!(worst <= 1, "worst {worst} along {s:.3} to {e:.3}");
}
```


- [ ] **Step 4: Mark the fixtures binary, and run**

```diff
--- a/.gitattributes
+++ b/.gitattributes
@@ -1,3 +1,4 @@
 # Files produced by the macOS app are compared byte for byte; never convert their line endings.
 engine/tests/fixtures/mac-1.2.6/** -text
 engine/tests/fixtures/mac-1.2.10-probes/** -text
+engine/tests/fixtures/mac-4b1-probes/** -text
```


Run: `cargo test -p compositor-engine --test mac_4b1_probes`
Expected: 5 passed. These tests pin code that already exists, so there is no red step; Step 5's bugs prove each assertion can fail. Then the whole engine set: 551 passed, 10 ignored (69 binaries).

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**

Each on its own, `Copy-Item <file> <file>.bak` first, reverted with `Move-Item -Force <file>.bak <file>` and `(Get-Item <file>).LastWriteTime = Get-Date`:
1. `engine/src/ops/raster_edit.rs` `paint_grid`: truncate the colour instead of rounding it. The three gradient tests fail (measured).
2. `engine/src/ops/shape.rs`: a rounded rectangle's corner constant `k = radius * 0.5`. `every_shape_matches_the_mac_render_within_its_own_measured_edge_bound` fails on Rectangle 1 (measured).
3. `engine/src/ops/shape.rs`: the line capsule's half-width `width - 0.5`. The same test fails on the lines (measured).

- [ ] **Step 6: Commit**

```
git add -- engine/tests/mac_4b1_probes.rs engine/tests/fixtures/mac-4b1-probes
git commit -m "test: pin the Phase 4b-1 Mac probes: shapes at their saved boxes, gradients along lines fitted from the exports" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- .gitattributes engine/tests/fixtures/mac-4b1-probes engine/tests/mac_4b1_probes.rs
```


---

### Task 2: The Phase 4.5 Mac probes and their hand steps

The probes that settle what 1.4.5 changed, written by the port for the user to open and export on the Mac (ruling OQ2), and the README steps for the ones only the Mac can make. Task 1 comes first so that regenerating the probes can never overwrite the Mac's own `shapes.comp` before it is pinned: `write_mac_probes` rewrites `build-artifacts/mac-probes/shapes.comp`.

**Files:**
- Modify: `engine/tests/mac_probes.rs`

**Mac:** LayerBlendMode.allCases (LayerAppearance.swift:4-13) for the 24 modes in menu order; HueSaturationTests.swift:274-282 for the saturation amounts; mac-1.4.5-delta.md section 5 for the probe list.

- [ ] **Step 1: Write the probes and the test that holds them**

```diff
--- a/engine/tests/mac_probes.rs
+++ b/engine/tests/mac_probes.rs
@@ -1,13 +1,14 @@
-//! Probe `.comp` projects for the user to open in Compositor for Mac (1.2.10 or later) (Task 7).
-//! Each one is built and saved through this build's own `save_package`, so it is a real, valid v9
-//! project; the ignored test below re-opens every one with `open_package` as a sanity floor,
-//! then writes it (and a README telling the user what to do with it) to
-//! `build-artifacts/mac-probes/`, which is git-ignored.
+//! Probe `.comp` projects for the user to open in Compositor for Mac 1.4.5 (Task 7; the Mac the
+//! user runs since 2026-09-30). Each one is built and saved through this build's own
+//! `save_package`, so it is a real, valid project; the ignored test below re-opens every one with
+//! `open_package` as a sanity floor, then writes it (and a README telling the user what to do with
+//! it) to `build-artifacts/mac-probes/`, which is git-ignored.
 //!
-//! The Mac's renders of the first seven are committed under tests/fixtures/mac-1.2.10-probes
-//! and compared in mac_1_2_10.rs. The rest settle what those could not (the light blend modes,
-//! Color Balance, the blurs, Add Noise, the cgMode path of adjustment layers and clipped groups)
-//! and join mac_1_2_10.rs when their renders come back.
+//! The Mac 1.2.10 renders of the earlier sets are committed under tests/fixtures/mac-1.2.10-probes
+//! and compared in mac_1_2_10.rs; the Phase 4b-1 renders under tests/fixtures/mac-4b1-probes
+//! (mac_4b1_probes.rs). The Phase 4.5 set settles what Compositor 1.4.5 changed: Soft Light, the
+//! modes of adjustment layers and clipping stacks, positive saturation, the upright 1:1 copy and
+//! Add Mask with a selection (docs/superpowers/research/mac-1.4.5-delta.md, section 5).
 
 use compositor_engine::*;
 use std::fs;
@@ -398,7 +399,7 @@ This folder holds test projects for Compositor on the Mac.
 
 For each project listed below:
 
-1. Open it in Compositor for Mac (1.2.10 or later).
+1. Open it in Compositor for Mac 1.4.5.
 2. Confirm it opens without an error.
 3. File > Export > PNG, at 100%, into a folder named mac-exports, using the file name given below.
 4. Send the mac-exports folder back.
@@ -494,6 +495,62 @@ above, and send the .comp folder with it.
   Click the foreground colour swatch, type 0000FF in the # field, click OK. Press G. Choose Linear
   and Foreground to Transparent, Reverse off, and type 37 in Opacity. Press Cmd-1. Hold Shift and
   drag from the canvas's left edge to its right edge, then press Return. Save and export.
+
+Phase 4.5: everything below is for Compositor for Mac 1.4.5. Where a step says to save, send the
+saved .comp folder back too (zipped), beside the PNG.
+
+Export these six again from 1.4.5, with the same file names as before (1.4.5 draws them
+differently from 1.2.10):
+
+- blend-greys.comp             -> blend-greys.png
+- new-blend-modes.comp         -> new-blend-modes.png
+- cgmode-blur-linear-burn.comp -> cgmode-blur-linear-burn.png
+- cgmode-levels-divide.comp    -> cgmode-levels-divide.png
+- color-dodge-adjustment.comp  -> color-dodge-adjustment.png
+- cgmode-stack-bases.comp      -> cgmode-stack-bases.png
+
+New in this set (blend modes, saturation, layers drawn one pixel for one pixel):
+
+- soft-light-dark.comp               -> soft-light-dark.png
+- stack-modes.comp                   -> stack-modes.png
+- adjustment-modes-soft-light.comp   -> adjustment-modes-soft-light.png
+- adjustment-modes-linear-dodge.comp -> adjustment-modes-linear-dodge.png
+- adjustment-modes-vivid-light.comp  -> adjustment-modes-vivid-light.png
+- adjustment-modes-hard-mix.comp     -> adjustment-modes-hard-mix.png
+- hsv-sat-plus-25.comp               -> hsv-sat-plus-25.png
+- hsv-sat-plus-50.comp               -> hsv-sat-plus-50.png
+- hsv-sat-plus-62.comp               -> hsv-sat-plus-62.png
+- hsv-sat-plus-100.comp              -> hsv-sat-plus-100.png
+- hsv-sat-minus-50.comp              -> hsv-sat-minus-50.png
+- hsv-reds-plus-50.comp              -> hsv-reds-plus-50.png
+- sampling-upright-1to1.comp         -> sampling-upright-1to1.png
+
+New in this set, finished by hand on the Mac, then saved (File > Save) and exported:
+
+- mask-reveal-selection.comp -> mask-reveal-selection.png
+  View > Snap To: turn Guides on. Choose the Elliptical Marquee (press M, and Shift-M until the
+  ellipse shows), Anti-alias on, Feather 0. In the Layers panel click Reveal. Drag from where the
+  upper guides cross at the top left (x 50, y 35) to where they cross at the bottom right (x 190,
+  y 105). Click the Add Mask button at the foot of the Layers panel. Then click Hide, drag the same
+  ellipse from (50, 145) to (190, 215), and Option-click the Add Mask button. Save, then export.
+
+Made on the Mac from nothing in this folder (the first two) or from edited-rich-file.comp:
+
+- text-runs.comp -> text-runs.png
+  File > New Canvas, 480 x 200. With the Type tool (T) click near the top left and type
+  Hello World. Select the word World and colour it red (FF0000). Select the word Hello and choose
+  another installed face for it, for example Helvetica Bold or Times New Roman. Click outside the
+  text to finish it. With the Type tool click lower down and type Plain, and finish it. Save as
+  text-runs.comp, then export.
+
+- text-colour-only.comp -> text-colour-only.png
+  File > New Canvas, 300 x 120. With the Type tool type Colour Only. Select the word Only and
+  colour it blue (0000FF); leave the face as it is. Finish the text. Save as
+  text-colour-only.comp, then export.
+
+- resaved-edited-rich-file.comp -> resaved-edited-rich-file.png
+  Open edited-rich-file.comp from this folder and change nothing. File > Save As,
+  resaved-edited-rich-file.comp. Then export.
 ";
 
 /// 7. RULING (F5, replacing the M9 tautological final-existence loop): the Mac acceptance probe.
@@ -710,6 +767,121 @@ fn phase_4b1_probes() -> Vec<(&'static str, Document)> {
     ]
 }
 
+/// Every blend mode in the Mac's own order (`LayerBlendMode.allCases`, LayerAppearance.swift:4-13 at
+/// v1.4.5): the order of `stack-modes.comp`'s cells.
+const MAC_MODES: [BlendMode; 24] = [
+    BlendMode::Normal, BlendMode::Darken, BlendMode::Multiply, BlendMode::ColorBurn, BlendMode::LinearBurn,
+    BlendMode::Lighten, BlendMode::Screen, BlendMode::ColorDodge, BlendMode::LinearDodge,
+    BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight, BlendMode::VividLight, BlendMode::LinearLight,
+    BlendMode::PinLight, BlendMode::HardMix, BlendMode::Difference, BlendMode::Exclusion, BlendMode::Subtract,
+    BlendMode::Divide, BlendMode::Hue, BlendMode::Saturation, BlendMode::Color, BlendMode::Luminosity,
+];
+
+/// A mode's name as the manifest spells it.
+fn mode_name(mode: BlendMode) -> String { serde_json::to_value(mode).unwrap().as_str().unwrap().to_string() }
+
+/// The dark backdrop greys of `soft-light-dark.comp`: 0 to 64, one per 4-px column.
+const DARK_STEPS: u32 = 65;
+/// The source greys and alphas of `soft-light-dark.comp`'s ten bands: 0.5 to 1.0, opaque, then the
+/// same at half alpha.
+const LIGHT_SOURCES: [(u32, u32); 10] = [(128, 255), (160, 255), (192, 255), (224, 255), (255, 255), (128, 128), (160, 128), (192, 128), (224, 128), (255, 128)];
+
+/// `soft-light-dark.comp` (B2): a grey backdrop running 0 to 64 (cb <= 0.25, where the W3C formula's
+/// D(cb) is not Photoshop's square root) under ten 12-row Soft Light bands of light greys, opaque and
+/// at half alpha (mac-1.4.5-delta.md, 2.1).
+fn soft_light_dark_doc() -> Document {
+    let (width, height) = (DARK_STEPS * 4, 12 * LIGHT_SOURCES.len() as u32);
+    let mut doc = Document::new(width, height);
+    let ramp: Vec<u8> = (0..height).flat_map(|_| (0..width).flat_map(|x| { let v = (x / 4) as u8; [v, v, v, 255] })).collect();
+    let mut layers = vec![Layer::with_pixels("Dark ramp", Raster::from_premultiplied(width, height, ramp), Point { x: 0.0, y: 0.0 })];
+    for (band, (grey, alpha)) in LIGHT_SOURCES.iter().enumerate() {
+        let v = ((grey * alpha + 127) / 255) as u8;
+        let mut l = solid_rect(&format!("Soft Light {grey} at {alpha}"), [v, v, v, *alpha as u8], width, 12, 0.0, band as f64 * 12.0);
+        l.blend_mode = BlendMode::SoftLight;
+        layers.push(l);
+    }
+    doc.layers = layers;
+    doc
+}
+
+/// `stack-modes.comp` (B3): 24 clipping stacks over the hue sweep, six across and four down in 40-px
+/// cells, in `MAC_MODES` order: an opaque 30 x 30 base in that mode and a half-alpha 20 x 30 child
+/// clipped to it (mac-1.4.5-delta.md, 2.2).
+fn stack_modes_doc() -> Document {
+    let mut doc = Document::new(240, 160);
+    let mut layers = vec![Layer::with_pixels("Hue sweep", colourful_gradient(240, 160), Point { x: 0.0, y: 0.0 })];
+    for (i, mode) in MAC_MODES.iter().enumerate() {
+        let (x, y) = ((i % 6) as f64 * 40.0 + 5.0, (i / 6) as f64 * 40.0 + 5.0);
+        let mut base = solid_rect(&format!("{} base", mode_name(*mode)), [60, 150, 110, 255], 30, 30, x, y);
+        base.blend_mode = *mode;
+        let mut child = solid_rect(&format!("{} child", mode_name(*mode)), [40, 20, 90, 128], 20, 30, x + 10.0, y);
+        child.mask_source_id = Some(base.id);
+        layers.push(base);
+        layers.push(child);
+    }
+    doc.layers = layers;
+    doc
+}
+
+/// A Hue/Saturation adjustment layer's settings: `saturation` on `range` (the Master scalars too, as
+/// the Mac keeps them for Master).
+fn hue_saturation(range: ColorRange, saturation: f64) -> LayerAdjustment {
+    let mut a = LayerAdjustment::new(AdjustmentKind::Hsv);
+    a.hsv_settings = Some(HueSaturationSettings::new(0.0, saturation, 0.0, false, range));
+    if range == ColorRange::Master { a.saturation = saturation; }
+    a
+}
+
+/// The Master saturations of the `hsv-sat-*` probes (B5), with their names.
+const MASTER_SATURATIONS: [(&str, f64); 5] = [("hsv-sat-plus-25.comp", 25.0), ("hsv-sat-plus-50.comp", 50.0), ("hsv-sat-plus-62.comp", 62.0),
+    ("hsv-sat-plus-100.comp", 100.0), ("hsv-sat-minus-50.comp", -50.0)];
+
+/// `sampling-upright-1to1.comp` (B6): the sampling pattern drawn one pixel for one pixel, upright, at a
+/// fractional origin: at (10.5, 20.25), and flipped at (10.25, 44). Compositor 1.4.5 copies such a
+/// layer straight across (LayerRenderer.swift:39-47); 1.2.10 filtered it (mac-1.4.5-delta.md, 2.9).
+fn sampling_upright_doc() -> Document {
+    let mut doc = Document::new(96, 72);
+    let first = Layer::with_pixels("Upright", sampling_pattern(), Point { x: 10.5, y: 20.25 });
+    let mut second = Layer::with_pixels("Upright flipped", sampling_pattern(), Point { x: 10.25, y: 44.0 });
+    second.transform.flip_x = true;
+    doc.layers = vec![first, second];
+    doc
+}
+
+/// The guides `mask-reveal-selection.comp` (A6) aims the ellipses at: x 50 and 190, and y 35, 105,
+/// 145 and 215 (an ellipse from (50, 35) to (190, 105) over Reveal, and from (50, 145) to (190, 215)
+/// over Hide).
+const MASK_GUIDES: [(GuideAxis, f64); 6] = [(GuideAxis::Vertical, 50.0), (GuideAxis::Vertical, 190.0), (GuideAxis::Horizontal, 35.0),
+    (GuideAxis::Horizontal, 105.0), (GuideAxis::Horizontal, 145.0), (GuideAxis::Horizontal, 215.0)];
+
+/// `mask-reveal-selection.comp` (A6): two 200 x 100 layers on a 240 x 250 canvas, Reveal at (20, 20)
+/// and Hide at (20, 130), and the guides the user drags an antialiased elliptical marquee between on
+/// the Mac before Add Mask (Reveal) and Option-click Add Mask (Hide) (mac-1.4.5-delta.md, 2.4).
+fn mask_reveal_doc() -> Document {
+    let mut doc = Document::new(240, 250);
+    doc.layers = vec![
+        solid_rect("Reveal", [200, 60, 40, 255], 200, 100, 20.0, 20.0),
+        solid_rect("Hide", [40, 90, 200, 255], 200, 100, 20.0, 130.0),
+    ];
+    doc.guides = MASK_GUIDES.iter().map(|(axis, position)| Guide { id: uuid::Uuid::new_v4(), axis: *axis, position: *position }).collect();
+    doc
+}
+
+/// The Phase 4.5 probes the port can write (B2-B6, and A6's canvas); the hand-made ones (A1-A3) are
+/// only in the README.
+fn phase_4_5_probes() -> Vec<(&'static str, Document)> {
+    let mut probes = vec![("soft-light-dark.comp", soft_light_dark_doc()), ("stack-modes.comp", stack_modes_doc())];
+    for (name, mode) in [("adjustment-modes-soft-light.comp", BlendMode::SoftLight), ("adjustment-modes-linear-dodge.comp", BlendMode::LinearDodge),
+        ("adjustment-modes-vivid-light.comp", BlendMode::VividLight), ("adjustment-modes-hard-mix.comp", BlendMode::HardMix)] {
+        probes.push((name, over_sweep(levels_to_mid_grey(), mode, 0.6)));
+    }
+    for (name, saturation) in MASTER_SATURATIONS { probes.push((name, over_sweep(hue_saturation(ColorRange::Master, saturation), BlendMode::Normal, 1.0))); }
+    probes.push(("hsv-reds-plus-50.comp", over_sweep(hue_saturation(ColorRange::Reds, 50.0), BlendMode::Normal, 1.0)));
+    probes.push(("sampling-upright-1to1.comp", sampling_upright_doc()));
+    probes.push(("mask-reveal-selection.comp", mask_reveal_doc()));
+    probes
+}
+
 /// Saves `doc` as `<dir>/<filename>/manifest.json` plus its `images/`, then re-opens the saved
 /// package with `open_package` -- every probe must be openable by this build's own reader before
 /// it is ever sent to a Mac.
@@ -764,6 +936,7 @@ fn write_mac_probes() {
     for (name, doc) in sampling_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in step_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in phase_4b1_probes() { write_probe(&dir, name, &doc); }
+    for (name, doc) in phase_4_5_probes() { write_probe(&dir, name, &doc); }
 
     fs::write(dir.join("README.txt"), README_TXT).unwrap_or_else(|e| panic!("failed to write README.txt: {e}"));
     assert!(README_TXT.is_ascii(), "README.txt must be ASCII only");
@@ -859,7 +1032,65 @@ fn every_4b1_probe_is_listed_and_every_shape_is_one_the_mac_will_redraw() {
     assert_eq!(probes[3].1.layers[0].pixels.as_ref().unwrap().pixel(0, 0), [204, 77, 51, 255]);
 }
 
+#[test]
+fn every_4_5_probe_is_listed_and_holds_what_it_is_named_for() {
+    let probes = phase_4_5_probes();
+    assert_eq!(probes.len(), 14);
+    for (name, _) in &probes { assert!(README_TXT.contains(&format!("- {name}")), "{name} is in the README"); }
+    let get = |n: &str| &probes.iter().find(|(name, _)| *name == n).unwrap_or_else(|| panic!("{n}")).1;
+    // B2: a backdrop from 0 to 64 (cb <= 0.25) under ten Soft Light bands of greys from 0.5 up.
+    let dark = get("soft-light-dark.comp");
+    let ramp = dark.layers[0].pixels.as_ref().unwrap();
+    assert_eq!((ramp.pixel(0, 0), ramp.pixel(ramp.width - 1, 0)), ([0, 0, 0, 255], [64, 64, 64, 255]));
+    assert_eq!(dark.layers.len(), 1 + LIGHT_SOURCES.len());
+    for (layer, (grey, alpha)) in dark.layers[1..].iter().zip(LIGHT_SOURCES) {
+        let p = layer.pixels.as_ref().unwrap().pixel(0, 0);
+        assert_eq!(layer.blend_mode, BlendMode::SoftLight, "{}", layer.name);
+        assert!(grey >= 128 && p[3] as u32 == alpha && (p[0] as f64 / p[3] as f64 - grey as f64 / 255.0).abs() < 0.005, "{}: {p:?}", layer.name);
+    }
+    // B3: 24 stacks with their bases in the Mac's order, each child clipped to the base just below it.
+    let stacks = get("stack-modes.comp");
+    let bases: Vec<BlendMode> = stacks.layers.iter().filter(|l| l.name.ends_with(" base")).map(|l| l.blend_mode).collect();
+    assert_eq!(bases, MAC_MODES.to_vec());
+    for pair in stacks.layers[1..].chunks(2) { assert_eq!(pair[1].mask_source_id, Some(pair[0].id), "{}", pair[1].name); }
+    // B4: Levels to mid grey at 60 % in the mode each is named for.
+    for (name, mode) in [("adjustment-modes-soft-light.comp", BlendMode::SoftLight), ("adjustment-modes-linear-dodge.comp", BlendMode::LinearDodge),
+        ("adjustment-modes-vivid-light.comp", BlendMode::VividLight), ("adjustment-modes-hard-mix.comp", BlendMode::HardMix)] {
+        let layer = &get(name).layers[1];
+        assert_eq!((layer.blend_mode, layer.opacity, layer.extra.adjustment.as_ref().unwrap().kind), (mode, 0.6, AdjustmentKind::Levels), "{name}");
+    }
+    // B5: the saturation each is named for, on Master or on Reds alone.
+    for (name, saturation) in MASTER_SATURATIONS {
+        let hsv = get(name).layers[1].extra.adjustment.as_ref().unwrap().resolved_hsv();
+        assert_eq!(hsv.adjustment(ColorRange::Master).saturation, saturation, "{name}");
+    }
+    let reds = get("hsv-reds-plus-50.comp").layers[1].extra.adjustment.as_ref().unwrap().resolved_hsv();
+    assert_eq!((reds.adjustment(ColorRange::Reds).saturation, reds.adjustment(ColorRange::Master)), (50.0, RangeAdjustment::default()));
+    // B6: upright layers drawn one pixel for one pixel, at fractional origins, one of them flipped.
+    let upright = get("sampling-upright-1to1.comp");
+    for layer in &upright.layers {
+        let (p, t) = (layer.pixels.as_ref().unwrap(), layer.transform);
+        assert_eq!((t.size.width, t.size.height, t.rotation), (p.width as f64, p.height as f64, 0.0), "{}", layer.name);
+        assert!(t.origin.x.fract() != 0.0, "{}: a fractional origin", layer.name);
+    }
+    assert_eq!(upright.layers.iter().filter(|l| l.transform.flip_x).count(), 1);
+    // A6: each guided ellipse lies inside the layer it is drawn over.
+    let masks = get("mask-reveal-selection.comp");
+    assert_eq!(masks.layers.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["Reveal", "Hide"]);
+    for (layer, (top, bottom)) in masks.layers.iter().zip([(35.0, 105.0), (145.0, 215.0)]) {
+        let t = layer.transform;
+        assert!(t.origin.x < 50.0 && t.origin.x + t.size.width > 190.0 && t.origin.y < top && t.origin.y + t.size.height > bottom, "{}", layer.name);
+        assert!(masks.guides.iter().any(|g| g.axis == GuideAxis::Horizontal && g.position == top) && masks.guides.iter().any(|g| g.axis == GuideAxis::Horizontal && g.position == bottom));
+    }
+    // The hand-made projects and the six re-exports are in the README, after the Phase 4.5 heading.
+    let section = &README_TXT[README_TXT.find("Phase 4.5").expect("a Phase 4.5 section")..];
+    for name in ["text-runs.comp", "text-colour-only.comp", "resaved-edited-rich-file.comp", "blend-greys.comp", "new-blend-modes.comp",
+        "cgmode-blur-linear-burn.comp", "cgmode-levels-divide.comp", "color-dodge-adjustment.comp", "cgmode-stack-bases.comp"] {
+        assert!(section.contains(&format!("- {name}")), "{name} is in the Phase 4.5 section");
+    }
+}
+
 #[test]
 fn the_readme_names_the_mac_version_the_probes_are_for() {
-    assert!(README_TXT.contains("Compositor for Mac (1.2.10 or later)") && !README_TXT.contains("1.2.6"));
+    assert!(README_TXT.contains("Compositor for Mac 1.4.5") && !README_TXT.contains("1.2.10 or later") && !README_TXT.contains("1.2.6"));
 }
```


- [ ] **Step 2: Run it**

Run: `cargo test -p compositor-engine --test mac_probes`
Expected: every test passes (552 in the whole engine set). Like Task 1 this adds test code only, so Step 5 proves the new test can fail.

- [ ] **Step 3: Generate the probes**

Run: `cargo test -p compositor-engine --test mac_probes -- --ignored write_mac_probes`
Expected: `build-artifacts/mac-probes/` holds 66 probes (the 52 earlier ones and 14 new: the 13 of ruling OQ2 and A6's canvas) and a README whose Phase 4.5 section lists, in this order: the six B1 re-exports, the 13 new exports, A6's hand steps and the hand steps for A1, A2 and A3. Hand the folder to the user; the plan does not wait for the exports (Tasks 6-9 mark what they will settle).

- [ ] **Step 4: Full engine set**

`cargo test -p compositor-engine`: 552 passed, 10 ignored.

- [ ] **Step 5: Introduce bugs and watch the test fail (measured)**
1. Rename one new probe in the README text only (`soft-light-dark` to `soft-light-dim`). `every_4_5_probe_is_listed_and_holds_what_it_is_named_for` fails on the README check (measured).
2. In `stack_modes_doc`, leave the child unclipped. The same test fails on "Normal child" (measured).
3. Swap `hue_saturation`'s range and saturation arguments in `phase_4_5_probes`. The same test fails at `hsv-sat-plus-25` (0 read where 25 was written) (measured).

- [ ] **Step 6: Commit**

```
git commit -m "test: the Phase 4.5 Mac probes: Soft Light over dark greys, stacks and adjustments in every mode, saturation, 1:1 layers, the mask reveal, and the hand-made text projects" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine/tests/mac_probes.rs
```


---

### Task 3: F1, the job worker returns its result halved to the canvas's level

Phase 4b-1 left the frame after a large job's result doing the halving on the UI thread: 314-533 ms at 100 MP (phase4b1 rulings, F1). The worker now halves the result to the level the canvas draws it at and sends that halving back with it; `install_job` hands it to the installed pixels, and every road to a halving starts from it (rulings OQ3, OQ4).

**Files:**
- Modify: `engine/src/raster.rs` (`RasterInner.adopted`, `Raster::adopt`, `adopted`, `reduced`, `size_at_level`, `seed_adopted`), `engine/src/engine.rs` and `engine/src/compositor.rs` (read through `reduced`), `engine/src/jobs.rs` (`JobOutput.display`, `DisplayHalving`, `run_edit_job(.., out_per_doc)`, `install_job(.., display)`), `engine-wasm/src/lib.rs` (the fourth job buffer), `app/src/engine/jobs.ts`, `app/src/engine/job-worker.ts`, `app/src/engine/client.ts`, `app/src/state/store.ts` (`runEditJob` sends `outPerDoc` and passes the halving back)
- Create: `engine/tests/display_level.rs`, `app/tests/e2e/perf-4-5.spec.ts`
- Modify (tests): `engine/tests/jobs.rs`, `engine/tests/mask_grow.rs`, `engine/tests/raster_edits.rs` (the new argument), `app/tests/unit/store-jobs.test.ts`, `app/tests/e2e/jobs.spec.ts`

**Interfaces this task produces:**
- `Raster::adopt(&self, level: u32, reduced: Raster) -> bool` (refuses level 0 and a raster of the wrong size), `Raster::adopted(&self) -> Option<(u32, Raster)>`, `Raster::reduced(&self, level: u32) -> Raster` (starts from the adopted halving when its level is at or below `level`).
- `run_edit_job(input, pixels, mask, points, command, out_per_doc: f64) -> (JobOutput, Option<Raster>, Option<GrayRaster>, Option<Raster>)`; `JobOutput.display: Option<DisplayHalving { level, width, height }>`.
- `install_job(engine, doc, output, pixels, mask, display: Option<Raster>)`; in wasm, `job_display_ptr` / `job_display_len` and `installJob(.., display)`; in TypeScript, `JobRequest` edit `outPerDoc: number`, `JobResult.display?: ArrayBuffer | null`.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/jobs.spec.ts
+++ b/app/tests/e2e/jobs.spec.ts
@@ -124,3 +124,48 @@ test("a blur on a large layer grows it through the worker exactly as it does in
   // Ruling I5: the blur's commit ran in the worker, not silently on the UI thread.
   expect(await jobKinds(page), "the commit ran in the job worker").toEqual(["edit"]);
 });
+
+test("F1: on a zoomed-out canvas a worker's result comes back halved to the canvas's level, and draws as the CPU draws it", async ({ page }) => {
+  // The layer covers its canvas exactly, as zoom-render.spec.ts's layers do, so the comparison is the
+  // layer's own pixels at every output pixel.
+  await page.setViewportSize({ width: 1280, height: 720 });
+  await page.goto("/");
+  await expect(page.getByTestId("engine-ready")).toBeVisible();
+  const b64 = await page.evaluate(noisePngBase64);
+  const doc = await page.evaluate(async (data) => {
+    const api = (window as any).__compositor;
+    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
+    const doc = api.engine.newDocument(64, 64, false);
+    api.engine.importImage(doc, png, "noise", { x: 32, y: 32 });
+    api.store.setState({ jobPixels: 0 });
+    api.store.getState().openDocument(doc);
+    // What each install was handed as the result halved to the canvas's level (engine JobOutput.display).
+    const install = api.engine.installJob.bind(api.engine);
+    (window as any).__displays = [];
+    api.engine.installJob = (...a: unknown[]) => { (window as any).__displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0); return install(...a); };
+    await api.setZoom(0.25);
+    api.setCheckerboard(false);
+    return doc;
+  }, b64);
+  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "Levels" }));
+  await page.waitForFunction(() => (window as any).__compositor.store.getState().adjustEdit?.histogram !== null);
+  await page.getByLabel("Output white").fill("190");
+  await page.getByRole("button", { name: "OK" }).click();
+  await idle(page);
+  // A quarter of a device pixel per document pixel: the 64 x 64 layer is drawn after one halving, 32 x 32,
+  // which the worker made and sent back with the result.
+  expect(await page.evaluate(() => (window as any).__displays)).toEqual([32 * 32 * 4]);
+  const r = await page.evaluate(async (doc) => {
+    const api = (window as any).__compositor;
+    const s = api.store.getState(); const d = s.documents[doc]; const vp = s.viewports[doc]; const dpr = window.devicePixelRatio || 1;
+    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+    const gl = Array.from(api.readDocumentPixels()) as number[];
+    const rect = vp.documentRect({ width: d.width, height: d.height });
+    const w = Math.round((rect.x + rect.width) * dpr) - Math.round(rect.x * dpr);
+    const h = Math.round((rect.y + rect.height) * dpr) - Math.round(rect.y * dpr);
+    const cpu = Array.from(api.engine.composite(doc, { x: 0, y: 0, width: d.width, height: d.height }, w, h)) as number[];
+    return { gl, cpu };
+  }, doc);
+  expect(r.gl.length).toBe(r.cpu.length);
+  expect(r.gl.reduce((m, v, i) => Math.max(m, Math.abs(v - r.cpu[i])), 0), "the GPU draws the worker's halving as the CPU draws its own").toBeLessThanOrEqual(2);
+});
\ No newline at end of file
```

Create `app/tests/e2e/perf-4-5.spec.ts`:

```ts
import { test, expect } from "@playwright/test";

// Phase 4.5's timings (LL-073), in the release engine and on the real GPU: run `pnpm wasm`, then
// `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts`, and rebuild `pnpm wasm:dev`
// afterwards. The installed Edge runs WebGL on the GPU as WebView2 does; each test logs
// UNMASKED_RENDERER_WEBGL, and a software renderer (SwiftShader, Basic Render) invalidates a run. Each
// test prints what it measured and checks its budgets. Every test opens a fresh page per size and warms
// the path up before timing it (LL-074): the cold costs are logged, never asserted.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");
test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });

const SIZES: [string, number, number][] = [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]];

/** Opens the page and waits for the engine; every texture upload is counted in `__uploads`. Returns the
 * GPU's name (UNMASKED_RENDERER_WEBGL). */
async function ready(page: import("@playwright/test").Page): Promise<string> {
  await page.addInitScript(() => {
    const log = { image: 0, sub: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) { log.image++; return image.apply(this, args); };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; return sub.apply(this, args); };
  });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  // `__frame()`: one render of the active document through the app's renderer, ended by a 1 x 1
  // readPixels so the GPU work lands inside the timing; its milliseconds. `__uploads` then says how many
  // whole and partial texture uploads that frame made.
  return page.evaluate(() => {
    const api = (window as any).__compositor;
    const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
    const px = new Uint8Array(4);
    (window as any).__frame = () => {
      const s = api.store.getState();
      const log = (window as any).__uploads; log.image = 0; log.sub = 0;
      const t0 = performance.now();
      api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: true }, s.previewEdit());
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
      return performance.now() - t0;
    };
    const info = gl.getExtension("WEBGL_debug_renderer_info");
    return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "unknown";
  });
}

test("F1: the frame after a job's result at 24 and 100 MP, Levels on a layer and a fill of a blank layer, with no halving on the UI thread", async ({ page }) => {
  test.setTimeout(900_000);
  const out: Record<string, number> = {};
  let renderer = "";
  for (const [label, w, h] of SIZES) {
    for (const edit of ["levels", "fill"]) {
      renderer = await ready(page);
      const r = await page.evaluate(async ([w, h, edit]) => {
        const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
        const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        const s = () => api.store.getState();
        const result: Record<string, number> = {};
        const idle = () => new Promise<void>((done) => { const poll = () => (s().working ? setTimeout(poll, 20) : done()); poll(); });
        // Warm-up (LL-074): one fill through the worker on a small document, so neither the worker's
        // first job nor the renderer's first draw of a job's result is timed; its costs are logged only.
        const warm = api.engine.newDocument(2560, 2560, true);
        s().openDocument(warm);
        await settle(); frame();
        let t0 = performance.now();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        await idle();
        result["cold warm-up fill through the worker ms"] = Math.round(performance.now() - t0);
        result["cold warm-up frame after ms"] = Math.round(frame());
        s().closeDocument(warm);
        await settle();
        // The timed document, drawn once at fit before the edit.
        let doc: string;
        if (edit === "levels") {
          doc = api.engine.newDocument(10, 10, false);
          api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
          api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
        } else doc = api.engine.newDocument(w, h, true);
        s().openDocument(doc);
        await settle(); frame();
        // The UI thread's copies, timed where the store makes them; the frame that draws the result is
        // timed inside the install, so the gap windows below are contiguous with it, and `layerPixels`
        // (the halving that used to run here) is timed inside that frame.
        let installedAt = Infinity, inFrame = false, halving = 0, displays: number[] = [];
        const timed = (name: string, before?: (a: unknown[]) => void, after?: () => void) => {
          const f = api.engine[name].bind(api.engine);
          api.engine[name] = (...a: unknown[]) => { before?.(a); const t = performance.now(); try { return f(...a); } finally { const ms = performance.now() - t; if (name === "layerPixels") { if (inFrame) halving += ms; } else result[`${name} ms`] = Math.round(ms); after?.(); } };
        };
        timed("jobInput");
        timed("layerPixels");
        timed("installJob", (a) => displays.push((a[6] as ArrayBuffer | null)?.byteLength ?? 0), () => {
          installedAt = performance.now();
          s().refresh(s().activeId);
          inFrame = true;
          result["frame after the result is put back ms"] = Math.round(frame());
          inFrame = false;
          result["that frame's whole uploads"] = (window as any).__uploads.image;
        });
        const longestGap = (until: () => boolean) => new Promise<number>((done) => {
          let last = performance.now(), gap = 0;
          const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
          requestAnimationFrame(tick);
        });
        if (edit === "levels") {
          s().beginAdjust({ kind: "Levels" });
          await new Promise<void>((done) => { const poll = () => (s().adjustEdit?.histogram === null ? setTimeout(poll, 20) : done()); poll(); });
          const adjustment = JSON.parse(JSON.stringify(s().adjustEdit.adjustment));
          adjustment.levels.ranges[0].outputWhite = 200;
          s().updateAdjust({ adjustment });
          await new Promise<void>((done) => { const poll = () => (s().previewSettling() ? setTimeout(poll, 20) : done()); poll(); });
          await new Promise((r) => setTimeout(r, 300)); await settle();
          t0 = performance.now();
          s().commitAdjust();
        } else {
          t0 = performance.now();
          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
        }
        result["the key or OK (UI thread) ms"] = Math.round(performance.now() - t0);
        result["longest frame gap while the worker edits"] = await longestGap(() => !s().working);
        result["done after ms"] = Math.round(performance.now() - t0);
        result["layerPixels in the frame after ms"] = Math.round(halving);
        result["display buffer bytes"] = displays[0] ?? -1;
        const vp = s().viewports[doc];
        result["canvas scale x 1000"] = Math.round(vp.pointsPerPixel * (window.devicePixelRatio || 1) * 1000);
        s().closeDocument(doc);
        return result;
      }, [w, h, edit] as [number, number, string]);
      for (const [k, v] of Object.entries(r)) out[`${label} ${edit}: ${k}`] = v;
    }
  }
  console.log(`F1 (release wasm, Edge): ${JSON.stringify(out)}`);
  console.log(`F1 renderer: ${renderer}`);
  for (const [label] of SIZES) {
    for (const edit of ["levels", "fill"]) {
      const k = (name: string) => out[`${label} ${edit}: ${name}`];
      expect(k("display buffer bytes"), "the worker sent the result halved to the canvas's level").toBeGreaterThan(0);
      expect(k("that frame's whole uploads"), "the frame timed is the one that uploads the result").toBeGreaterThanOrEqual(1);
      // The halving that cost 314-533 ms at 100 MP on the UI thread (phase4b1 rulings, F1) is gone: the
      // frame reads the adopted halving.
      expect(k("layerPixels in the frame after ms"), "no halving on the UI thread").toBeLessThan(20);
      // F1's acceptance was 350 / 400 ms or lower (the 4b-1 budgets were 350 and 500-600); measured on the
      // HD 520 (p45-scratch, 2026-09-30): 8-21 ms at 24 MP and 12-23 ms at 100 MP, one upload of the
      // adopted halving. 100 ms at both sizes leaves room for this laptop's swings (ruling OQ3).
      expect(k("frame after the result is put back ms")).toBeLessThan(100);
      expect(k("longest frame gap while the worker edits")).toBeLessThan(100);
      expect(k("jobInput ms")).toBeLessThan(label === "24 MP" ? 150 : 450);
      expect(k("installJob ms")).toBeLessThan(label === "24 MP" ? 150 : 450);
    }
  }
});
```

```diff
--- a/app/tests/unit/store-jobs.test.ts
+++ b/app/tests/unit/store-jobs.test.ts
@@ -6,6 +6,7 @@ import type { Command, DocumentState, LayerAdjustment, LayerState, PreviewReques
 import type { EngineClient } from "../../src/engine/client";
 import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";
 import type { ShellBridge } from "../../src/shell/bridge";
+import type { Viewport } from "../../src/canvas/viewport";
 
 function layer(id: string, width: number, height: number): LayerState {
   return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
@@ -25,6 +26,7 @@ function install(width: number, height: number, onInstall?: () => void) {
   const log: string[] = [];
   const previews: (PreviewRequest | null)[] = [];
   const requests: JobRequest[] = [];
+  const displays: (ArrayBuffer | null)[] = [];
   let finish: (r: JobResult | null) => void = () => {};
   const engine = {
     state: () => document(layer("A", width, height)),
@@ -35,14 +37,14 @@ function install(width: number, height: number, onInstall?: () => void) {
     // Task 5's jobs API: a job's input carries its selection's points as a separate buffer (null
     // here -- these fixtures have no selection), beside the JSON and the pixel/mask buffers.
     jobInput: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null, points: null }; },
-    installJob: (_doc: string, layerId: string, input: string, output: string) => { log.push(`install ${layerId} ${output}`); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
+    installJob: (_doc: string, layerId: string, input: string, output: string, _pixels: ArrayBuffer | null, _mask: ArrayBuffer | null, display: ArrayBuffer | null) => { log.push(`install ${layerId} ${output}`); displays.push(display); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
     undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
     adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
   } as unknown as EngineClient;
   const jobs = { run: (_channel: string, request: JobRequest) => { requests.push(request); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
   useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: document(layer("A", width, height)) }, order: ["D"], selectedLayerIds: ["A"],
-    maskSelected: false, transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, working: false });
-  return { log, previews, requests, finish: (r: JobResult | null) => finish(r) };
+    maskSelected: false, transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, working: false, viewports: {} });
+  return { log, previews, requests, displays, finish: (r: JobResult | null) => finish(r) };
 }
 /** Opens Levels on layer A and moves a slider, so OK has something to apply. */
 function levelsChanged() {
@@ -75,6 +77,21 @@ describe("destructive commits on large layers go to the job worker", () => {
     expect(previews.length, "the engine cleared its own preview as it put the result back").toBe(shown);
   });
 
+  it("an edit job carries the scale the canvas draws at, and the result's halving goes back with it (F1)", async () => {
+    const { requests, displays, finish } = install(2001, 2000);
+    // An eighth of a CSS pixel per document pixel; the node test has no devicePixelRatio, so 1.
+    useEditor.setState({ viewports: { D: { pointsPerPixel: 0.125 } as unknown as Viewport } });
+    levelsChanged();
+    useEditor.getState().commitAdjust();
+    const request = requests.at(-1)!;
+    expect(request.kind === "edit" && request.outPerDoc).toBe(0.125);
+    const display = new ArrayBuffer(8);
+    finish({ header: "OUT", pixels: new ArrayBuffer(4), mask: null, display });
+    await flush();
+    expect(displays).toEqual([display]);
+    expect(displays[0], "the very buffer the worker sent").toBe(display);
+  });
+
   it("a layer at the threshold is edited on the UI thread as before", () => {
     const { log, requests } = install(2000, 2000);
     levelsChanged();
```

Create `engine/tests/display_level.rs`:

```rust
//! F1 (Phase 4.5): the job worker halves an edit's whole result to the level the canvas draws it at
//! and returns it with the result; the installed pixels adopt it, so the UI thread never halves a
//! large result itself (phase4b1-rulings-and-open-items.md, "Open items", F1). Every expected
//! halving is made here from a fresh copy of the bytes, which has no halvings of its own.
use compositor_engine::*;
use uuid::Uuid;

fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Colour and alpha changing every pixel in both directions, so a misplaced or reused halving shows.
fn pattern(width: u32, height: u32) -> Raster {
    let data = (0..height).flat_map(|y| (0..width).flat_map(move |x| {
        let a = 60 + ((x * 7 + y * 3) % 196);
        [((x * 13 + y) % 256).min(a), ((x + y * 11) % 256).min(a), ((x * 5 + y * 9) % 256).min(a), a].map(|v| v as u8)
    })).collect();
    Raster::from_premultiplied(width, height, data)
}

/// `raster` halved `level` times from a fresh copy of its bytes: halvings from scratch, sharing
/// nothing with `raster`'s own.
fn from_scratch(raster: &Raster, level: u32) -> Raster {
    Raster::from_premultiplied(raster.width, raster.height, raster.bytes().to_vec()).reduced(level)
}

/// An odd-sized 1001 x 603 layer on its own canvas, active.
fn document() -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(1001, 603);
    let layer = Layer::with_pixels("Pattern", pattern(1001, 603), p(0.0, 0.0));
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let handle = e.insert_document(doc);
    (e, handle, id)
}

/// Levels that changes every channel.
fn levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0].output_white = 200.0;
    a
}

/// The bytes of `raster` in a buffer of their own, as they cross from the worker (the wasm bridge's
/// `raster_of`): no halving made in the worker comes with them.
fn crossed(raster: Option<Raster>) -> Option<Raster> { raster.map(|r| Raster::from_premultiplied(r.width, r.height, r.bytes().to_vec())) }

/// The job for `command` on `layer`, run at `out_per_doc`, its buffers crossed back as bytes.
fn job(e: &Engine, id: Uuid, layer: Uuid, command: Command, out_per_doc: f64) -> (JobInput, JobOutput, Option<Raster>, Option<GrayRaster>, Option<Raster>) {
    let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
    let (output, new_pixels, new_mask, display) = run_edit_job(&input, pixels, mask, points.as_deref(), command, out_per_doc).unwrap();
    (input, output, crossed(new_pixels), new_mask, crossed(display))
}

#[test]
fn a_job_halves_its_whole_result_to_the_canvas_level_and_the_install_adopts_it() {
    let (mut e, id, layer) = document();
    // A tenth of a device pixel per document pixel: halve while one output pixel covers more than 2
    // source pixels, 10 -> 5 -> 2.5 -> 1.25, three times (`prefilterLevel`, the renderers' rule).
    let (input, output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert_eq!(prefilter_level(1001, 603, 10.0), 3);
    let new_pixels = new_pixels.expect("Levels replaces the pixels");
    let display = display.expect("a whole result is halved to the canvas level");
    // 1001 -> 500 -> 250 -> 125 and 603 -> 301 -> 150 -> 75.
    assert_eq!(output.display, Some(DisplayHalving { level: 3, width: 125, height: 75 }));
    assert_eq!(display.bytes(), from_scratch(&new_pixels, 3).bytes(), "the worker's halving is halving from scratch");
    e.install_job(id, layer, input.stamp, output, Some(new_pixels.clone()), new_mask, Some(display.clone())).unwrap();
    // No halving after install: the canvas's level is the very buffer the job made.
    let shown = e.layer_raster(id, layer, 3).unwrap().unwrap();
    assert!(shown.same_pixels(&display), "level 3 is the adopted halving itself");
    // A level past it halves the adopted halving; a level before it halves the pixels: both as from scratch.
    assert_eq!(e.layer_raster(id, layer, 4).unwrap().unwrap().bytes(), from_scratch(&new_pixels, 4).bytes());
    assert_eq!(e.layer_raster(id, layer, 2).unwrap().unwrap().bytes(), from_scratch(&new_pixels, 2).bytes());
    // The CPU compositor prefilters through it too.
    let region = Rect { x: 0.0, y: 0.0, width: 1001.0, height: 603.0 };
    let mut fresh = e.document(id).unwrap().clone();
    fresh.layers[0].pixels = Some(Raster::from_premultiplied(1001, 603, new_pixels.bytes().to_vec()));
    assert_eq!(e.composite(id, region, 100, 60).unwrap().bytes(), composite(&fresh, region, 100, 60).bytes());
}

#[test]
fn a_blank_layer_a_fill_grows_to_the_canvas_is_halved_on_the_grid_it_grew_to() {
    // Ruling C1's case: a new document's blank layer stores no pixels and so has no texture yet; the
    // fill paints the 800 x 600 canvas, and at a quarter of a device pixel per document pixel the
    // canvas draws it after one halving (4 -> 2).
    let mut e = Engine::new();
    let id = e.new_document(800, 600, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    let (_, output, new_pixels, _, display) = job(&e, id, layer, Command::Fill { id: layer, mask: false, color: [0.2, 0.4, 0.6] }, 0.25);
    assert_eq!(output.display, Some(DisplayHalving { level: 1, width: 400, height: 300 }));
    assert_eq!(display.unwrap().bytes(), from_scratch(&new_pixels.unwrap(), 1).bytes());
}

#[test]
fn nothing_is_halved_inside_a_selection_for_nearest_sampling_or_at_full_size() {
    let (mut e, id, layer) = document();
    // At 1:1 and closer the canvas does not prefilter; with no scale nothing is known.
    for scale in [1.0, 4.0, 0.0] {
        let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, scale);
        assert!(output.display.is_none() && display.is_none(), "at {scale}");
    }
    // Nearest is never prefiltered (`compositor::prefilters`).
    let mut nearest = e.document(id).unwrap().layers[0].transform;
    nearest.sampling = Sampling::Nearest;
    e.execute(id, Command::SetLayerTransform { id: layer, transform: nearest }).unwrap();
    let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert!(output.display.is_none() && display.is_none(), "Nearest");
    // Inside a selection the edit reports its rectangle, and the UI thread seeds the halvings.
    nearest.sampling = Sampling::High;
    e.execute(id, Command::SetLayerTransform { id: layer, transform: nearest }).unwrap();
    e.execute(id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(100.0, 100.0), p(300.0, 100.0), p(300.0, 200.0), p(100.0, 200.0)], mode: SelectionMode::Replace, antialiased: false }).unwrap();
    let (_, output, _, _, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    assert!(!output.regions.is_empty() && output.display.is_none() && display.is_none(), "inside a selection");
}

#[test]
fn an_edit_inside_a_selection_after_a_job_redoes_only_its_rectangle_of_the_adopted_halving() {
    let (mut e, id, layer) = document();
    let (input, output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, display).unwrap();
    // A clear in a selection on the UI thread: an odd rectangle, off every block boundary.
    e.execute(id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(101.0, 51.0), p(333.0, 51.0), p(333.0, 197.0), p(101.0, 197.0)], mode: SelectionMode::Replace, antialiased: true }).unwrap();
    e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }).unwrap();
    let cleared = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let (level, seeded) = cleared.adopted().expect("the new pixels inherit the adopted halving");
    assert_eq!(level, 3);
    assert_eq!(seeded.bytes(), from_scratch(&cleared, 3).bytes(), "equal to halving the cleared pixels from scratch");
    assert!(e.layer_raster(id, layer, 3).unwrap().unwrap().same_pixels(&seeded), "and the canvas's level is it, halved nowhere");
}

#[test]
fn install_refuses_a_display_halving_that_does_not_fit_its_pixels() {
    let (mut e, id, layer) = document();
    let (input, mut output, new_pixels, new_mask, display) = job(&e, id, layer, Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.1);
    let depth = e.state(id).unwrap().undo_depth;
    // The right size, said to be two halvings: 125 x 75 is what three make.
    output.display = output.display.map(|d| DisplayHalving { level: 2, ..d });
    assert!(matches!(e.install_job(id, layer, input.stamp, output.clone(), new_pixels.clone(), new_mask.clone(), display.clone()), Err(CommandError::Argument(_))));
    // A buffer that is not the size the output names.
    output.display = output.display.map(|d| DisplayHalving { level: 3, ..d });
    let small = Some(Raster::from_premultiplied(1, 1, vec![0; 4]));
    assert!(matches!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, small), Err(CommandError::Argument(_))));
    assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing was put back");
}

#[test]
fn letting_go_of_halvings_lets_go_of_an_adopted_one_too() {
    // History drops the halvings of rasters only it holds (ruling OQ1).
    let r = pattern(40, 30);
    assert!(r.adopt(2, from_scratch(&r, 2)));
    assert!(!r.adopt(2, from_scratch(&r, 1)), "the wrong size is refused");
    assert!(!r.adopt(0, r.clone()), "level 0 is the pixels themselves");
    assert_eq!(r.adopted().map(|(level, _)| level), Some(2));
    r.forget_halvings();
    assert!(r.adopted().is_none());
}
```

```diff
--- a/engine/tests/jobs.rs
+++ b/engine/tests/jobs.rs
@@ -65,8 +65,8 @@ fn an_edit_made_by_a_job_and_put_back_equals_the_edit_made_in_place() {
         let depth = there.state(tid).unwrap().undo_depth;
         run(&mut here, id, commands(layer)[i].clone());
         let (input, pixels, mask, points) = crossed(&there, tid, tlayer);
-        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), commands(tlayer)[i].clone()).unwrap();
-        there.install_job(tid, tlayer, input.stamp, output, new_pixels, new_mask).unwrap();
+        let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), commands(tlayer)[i].clone(), 0.0).unwrap();
+        there.install_job(tid, tlayer, input.stamp, output, new_pixels, new_mask, None).unwrap();
         let (a, b) = (&here.document(id).unwrap().layers[0], &there.document(tid).unwrap().layers[0]);
         assert_eq!(a.pixels.as_ref().unwrap().bytes(), b.pixels.as_ref().unwrap().bytes(), "command {i}: pixels");
         assert_eq!(a.transform, b.transform, "command {i}: transform");
@@ -80,10 +80,10 @@ fn a_job_inside_a_selection_brings_its_changed_rectangle_back() {
     let (mut e, id, layer) = document();
     let before = e.state(id).unwrap().layers[0].pixels_revision;
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }, 0.0).unwrap();
     assert!(new_mask.is_none(), "the mask was not touched, so it does not travel back");
     assert_eq!(output.regions.len(), 1);
-    e.install_job(id, layer, input.stamp, output.clone(), new_pixels, new_mask).unwrap();
+    e.install_job(id, layer, input.stamp, output.clone(), new_pixels, new_mask, None).unwrap();
     assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(output.regions[0].1));
 }
 
@@ -92,14 +92,14 @@ fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
     for change in ["pixels", "transform", "mask"] {
         let (mut e, id, layer) = document();
         let (input, pixels, mask, points) = crossed(&e, id, layer);
-        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
+        let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.0).unwrap();
         match change {
             "pixels" => run(&mut e, id, Command::InvertPixels { id: layer, mask: false }),
             "transform" => run(&mut e, id, Command::NudgeLayers { ids: vec![layer], dx: 1.0, dy: 0.0 }),
             _ => run(&mut e, id, Command::FillMask { id: layer, white: true }),
         }
         let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
-        let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
+        let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None);
         assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())), "{change}");
         assert!(e.document(id).unwrap().same_content(&doc), "{change}: untouched");
         assert_eq!(e.state(id).unwrap().undo_depth, depth, "{change}: nothing recorded");
@@ -107,9 +107,9 @@ fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
     // A change that leaves the layer's pixels, mask and place alone does not stop it.
     let (mut e, id, layer) = document();
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.0).unwrap();
     run(&mut e, id, Command::RenameLayer { id: layer, name: "Renamed".into() });
-    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
+    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None).unwrap();
     assert_eq!(e.state(id).unwrap().layers[0].name, "Renamed");
 }
 
@@ -120,11 +120,11 @@ fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
 fn a_selection_change_between_job_input_and_install_refuses_the_result() {
     let (mut e, id, layer) = document();
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.0).unwrap();
     // Nothing about the layer itself changes: only the selection a clipped job was computed against.
     run(&mut e, id, Command::Deselect);
     let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
-    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
+    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None);
     assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())));
     assert!(e.document(id).unwrap().same_content(&doc), "untouched");
     assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
@@ -138,14 +138,14 @@ fn a_top_left_canvas_size_between_job_input_and_install_refuses_the_result() {
     let (mut e, id, layer) = document();
     run(&mut e, id, Command::Deselect); // isolate the canvas size: no selection change either
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::ApplyAdjustment { id: layer, adjustment: levels() }, 0.0).unwrap();
     let before = e.document(id).unwrap().layers[0].clone();
     run(&mut e, id, Command::CanvasSize { width: 200, height: 150, anchor: 0, fill: None });
     let after = e.document(id).unwrap().layers[0].clone();
     assert_eq!(before.transform, after.transform, "a top-left anchor leaves this layer's transform alone");
     assert_eq!(before.pixels_revision, after.pixels_revision, "and its pixels");
     let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
-    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
+    let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None);
     assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())));
     assert!(e.document(id).unwrap().same_content(&doc), "untouched");
     assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
@@ -177,13 +177,13 @@ fn a_fractional_transform_survives_the_json_the_wasm_bridge_uses_for_install() {
     assert_eq!(input.stamp.transform.origin, origin, "the stamp survives prepare_job's JSON round trip");
 
     // The worker's edit (one that does not move the layer) and its own JSON (JobOutput).
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::InvertPixels { id: layer, mask: false }, 0.0).unwrap();
     let output: JobOutput = serde_json::from_str(&serde_json::to_string(&output).unwrap()).unwrap();
     assert_eq!(output.transform.origin, origin, "an edit that does not move the layer reports the same origin, exactly");
 
     // `install_job`'s own JSON of the stamp it is handed back (`stamp_json`).
     let stamp: LayerStamp = serde_json::from_str(&serde_json::to_string(&input.stamp).unwrap()).unwrap();
-    e.install_job(id, layer, stamp, output, new_pixels, new_mask).unwrap();
+    e.install_job(id, layer, stamp, output, new_pixels, new_mask, None).unwrap();
     assert_eq!(e.document(id).unwrap().layers[0].transform.origin, origin, "install keeps the exact origin");
 }
 
@@ -237,10 +237,10 @@ fn a_job_whose_result_passes_the_mask_budget_is_not_put_back() {
     assert_eq!(e.execute(id, fill.clone()), Err(CommandError::Project(ProjectError::TooLarge)), "in place");
     // Through a job: the worker sees one layer and makes the result...
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), fill).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), fill, 0.0).unwrap();
     assert_eq!(output.mask.map(|(w, h)| w as u64 * h as u64), Some(grown), "the worker grew the mask with its layer");
     // ...and the install refuses it.
-    assert_eq!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask), Err(CommandError::Project(ProjectError::TooLarge)));
+    assert_eq!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None), Err(CommandError::Project(ProjectError::TooLarge)));
     assert!(e.document(id).unwrap().same_content(&before), "untouched");
     assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
     assert_eq!(e.document(id).unwrap().used_mask_pixels(), held + own, "the budget as it was");
@@ -260,17 +260,17 @@ fn a_job_whose_result_passes_the_pixel_budget_is_not_put_back() {
     let (before, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
     assert_eq!(e.execute(id, fill.clone()), Err(CommandError::Project(ProjectError::TooLarge)), "in place");
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), fill).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), fill, 0.0).unwrap();
     assert_eq!(output.pixels.map(|(w, h)| w as u64 * h as u64), Some(grown), "the worker grew the layer");
-    assert_eq!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask), Err(CommandError::Project(ProjectError::TooLarge)));
+    assert_eq!(e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None), Err(CommandError::Project(ProjectError::TooLarge)));
     assert!(e.document(id).unwrap().same_content(&before), "untouched");
     assert_eq!(e.state(id).unwrap().undo_depth, depth, "nothing recorded");
     assert_eq!(e.document(id).unwrap().used_pixels(), held + own, "the budget as it was");
     // With room for it (one hog fewer), the same job's result goes back as one step.
     let (mut e, id, layer) = crowded(None, false, hogs - 1, hog);
     let (input, pixels, mask, points) = crossed(&e, id, layer);
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }).unwrap();
-    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }, 0.0).unwrap();
+    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None).unwrap();
     assert_eq!(e.document(id).unwrap().used_pixels(), held - hog.0 as u64 * hog.1 as u64 + grown);
 }
```

```diff
--- a/engine/tests/mask_grow.rs
+++ b/engine/tests/mask_grow.rs
@@ -299,10 +299,10 @@ fn a_mask_gradient_through_a_job_leaves_what_it_leaves_in_place() {
     let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), half, None);
     let command = Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, CLEAR) };
     let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), command.clone()).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), command.clone(), 0.0).unwrap();
     assert_eq!(output.mask, Some((100, 40)));
     assert_eq!(output.mask_placement.map(|t| (t.origin, t.size)), Some((p(0.0, 0.0), Size { width: 100.0, height: 40.0 })));
-    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
+    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None).unwrap();
     let through_job = mask_of(&e, id);
     e.undo(id).unwrap();
     run(&mut e, id, command);
```

```diff
--- a/engine/tests/raster_edits.rs
+++ b/engine/tests/raster_edits.rs
@@ -290,9 +290,9 @@ fn a_fill_or_gradient_whose_selection_is_off_the_canvas_changes_nothing_and_reco
     }
     // Through a job, likewise: nothing comes back, and the install records nothing.
     let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
-    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }).unwrap();
+    let (output, new_pixels, new_mask, _) = run_edit_job(&input, pixels, mask, points.as_deref(), Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }, 0.0).unwrap();
     assert_eq!((output.pixels, output.mask), (None, None));
-    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
+    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask, None).unwrap();
     assert_eq!(depth(&e, id), count, "job: nothing recorded");
 }
```


`crossed` copies a raster's bytes, as the wasm boundary does: without it the test's result would share the source's memo chain in-process and pass with the adoption broken (found by Step 5's first bug).

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test display_level`
Expected: does not compile (`Raster::adopt`, `run_edit_job`'s sixth argument and `install_job`'s `display` do not exist). `pnpm test -- store-jobs` fails on `outPerDoc` (undefined) and on the halving not passed to `installJob`.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -146,10 +146,12 @@ export class EngineClient {
    * after `level` halvings. */
   displayJobInput(doc: string, layer: string, level: number): JobInputCopy { return this.takeJob(this.wasm.prepare_display_job(doc, layer, level)); }
   /** Puts an edit job's result back (engine `install_job`), only onto the layer exactly as the job took
-   * it (its stamp, read from `input`); a changed layer refuses with the engine's message. */
-  installJob(doc: string, layer: string, input: string, output: string, pixels: ArrayBuffer | null, mask: ArrayBuffer | null): Dirty {
+   * it (its stamp, read from `input`); a changed layer refuses with the engine's message. `display` is
+   * the result halved to the canvas's level, which the new pixels adopt (F1). */
+  installJob(doc: string, layer: string, input: string, output: string, pixels: ArrayBuffer | null, mask: ArrayBuffer | null, display: ArrayBuffer | null = null): Dirty {
     const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
-    return JSON.parse(this.wasm.install_job(doc, layer, stamp, output, pixels ? new Uint8Array(pixels) : undefined, mask ? new Uint8Array(mask) : undefined)) as Dirty;
+    const view = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);
+    return JSON.parse(this.wasm.install_job(doc, layer, stamp, output, view(pixels), view(mask), view(display))) as Dirty;
   }
   /** Whether the canvas's effects image for `layer` is made already (engine `has_effects_image`):
    * `drawPixels` then hands it over without making it. */
```

```diff
--- a/app/src/engine/job-worker.ts
+++ b/app/src/engine/job-worker.ts
@@ -14,12 +14,19 @@ function kept(mask: boolean): ArrayBuffer | null {
   return new Uint8Array(memory!.buffer, ptr, len).slice().buffer;
 }
 const bytes = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);
+/** A copy of an edit's result halved to the canvas's level (the fourth job buffer, F1), or null. */
+function keptDisplay(): ArrayBuffer | null {
+  const len = engine!.job_display_len();
+  if (len === 0) return null;
+  const ptr = engine!.job_display_ptr();
+  return new Uint8Array(memory!.buffer, ptr, len).slice().buffer;
+}
 
 function run(request: JobRequest): JobResult {
   switch (request.kind) {
     case "edit": {
-      const header = engine!.run_edit_job(request.input, bytes(request.pixels), bytes(request.mask), bytes(request.points), request.command);
-      const result = { header, pixels: kept(false), mask: kept(true) };
+      const header = engine!.run_edit_job(request.input, bytes(request.pixels), bytes(request.mask), bytes(request.points), request.command, request.outPerDoc);
+      const result = { header, pixels: kept(false), mask: kept(true), display: keptDisplay() };
       engine!.release_job();
       return result;
     }
@@ -45,7 +52,7 @@ self.onmessage = (e: MessageEvent<ToWorker>) => {
   }
   try {
     const result = run(message.request);
-    post({ type: "done", id: message.id, result, memory: memory!.buffer.byteLength }, [result.pixels, result.mask].filter((b): b is ArrayBuffer => b !== null));
+    post({ type: "done", id: message.id, result, memory: memory!.buffer.byteLength }, [result.pixels, result.mask, result.display ?? null].filter((b): b is ArrayBuffer => b !== null));
   } catch (err) {
     // A wasm trap (an `unreachable` panic, an allocation abort) throws a `WebAssembly.RuntimeError`,
     // not the ordinary `JsError` a refused command throws: it leaves this instance unusable (jobs.ts's
```

```diff
--- a/app/src/engine/jobs.ts
+++ b/app/src/engine/jobs.ts
@@ -10,13 +10,15 @@
 
 /** What a job is asked to do. `input` is the engine's JobInput JSON; the buffers travel beside it. */
 export type JobRequest =
-  | { kind: "edit"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null; command: string }
+  | { kind: "edit"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null; command: string; outPerDoc: number }
   | { kind: "histogram"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; points: ArrayBuffer | null }
   | { kind: "effects"; input: string; pixels: ArrayBuffer; mask: ArrayBuffer | null; factor: number; edit: string | null };
 
 /** What came back: the engine's JSON answer (an edit's JobOutput, a histogram's bins, an effects
- * image's size and inset; null when an effects image found nothing to draw) and any buffers. */
-export interface JobResult { header: string | null; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; }
+ * image's size and inset; null when an effects image found nothing to draw) and any buffers. An edit
+ * run at a scale (`outPerDoc`, device pixels per document pixel) also brings its new pixels halved to
+ * the level the canvas draws them at (`display`, engine `JobOutput.display`; F1). */
+export interface JobResult { header: string | null; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; display?: ArrayBuffer | null; }
 
 /** Messages to the worker and back. `fatal` on a failure marks a wasm trap (an `unreachable` panic,
  * an allocation abort): `RuntimeError.prototype instanceof WebAssembly.RuntimeError`, as the worker's
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -515,11 +515,14 @@ export const useEditor = create<EditorStore>((set, get) => ({
     }
     set({ working: true });
     let installed = false;
+    // The scale the canvas draws this document at, in device pixels per document pixel: the worker
+    // halves the result to the level the renderer will upload it at (F1).
+    const outPerDoc = (get().viewports[doc]?.pointsPerPixel ?? 0) * (globalThis.devicePixelRatio || 1);
     try {
-      const result = await jobs.run(`edit:${doc}`, { kind: "edit", input: copy.input, pixels: copy.pixels, mask: copy.mask, points: copy.points, command: JSON.stringify(command) });
+      const result = await jobs.run(`edit:${doc}`, { kind: "edit", input: copy.input, pixels: copy.pixels, mask: copy.mask, points: copy.points, command: JSON.stringify(command), outPerDoc });
       // Closed meanwhile: nothing to put back.
       if (!result || !get().documents[doc]) return false;
-      engine.installJob(doc, layerId, copy.input, result.header!, result.pixels, result.mask);
+      engine.installJob(doc, layerId, copy.input, result.header!, result.pixels, result.mask, result.display ?? null);
       installed = true;
       return true;
     } catch (e) {
```

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -9,8 +9,9 @@ pub struct WasmEngine {
     engine: Engine, pending_saves: HashMap<Uuid, Package>, drawn: Option<Raster>,
     /// A job's pixel, mask and selection-point buffers, kept while the app copies them out: a job's
     /// input on the main thread, a job's result in the worker (`job_buffer_ptr`, `job_points_ptr`,
-    /// `release_job`).
-    job: (Option<Raster>, Option<GrayRaster>, Option<Vec<i32>>),
+    /// `release_job`). The fourth is an edit's result halved to the canvas's level (`job_display_ptr`,
+    /// F1), only in the worker.
+    job: (Option<Raster>, Option<GrayRaster>, Option<Vec<i32>>, Option<Raster>),
 }
 
 /// A buffer's bytes as a raster, when its size is known.
@@ -46,7 +47,7 @@ impl WasmEngine {
     #[wasm_bindgen(constructor)]
     pub fn new() -> WasmEngine {
         console_error_panic_hook::set_once();
-        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new(), drawn: None, job: (None, None, None) }
+        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new(), drawn: None, job: (None, None, None, None) }
     }
 
     // Jobs (Phase 4b-1, engine `jobs.rs`). On the main thread: `prepare_job` or `prepare_display_job`,
@@ -57,14 +58,14 @@ impl WasmEngine {
     /// kept for `job_buffer_ptr` / `job_points_ptr`.
     pub fn prepare_job(&mut self, doc: &str, layer: &str) -> Result<String, JsError> {
         let (input, pixels, mask, points) = self.engine.job_input(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?;
-        self.job = (pixels, mask, points);
+        self.job = (pixels, mask, points, None);
         serde_json::to_string(&input).map_err(js_err)
     }
     /// `Engine::display_job_input` (an effects job's input at `level` halvings) as JSON. An effects
     /// job never clips to a selection, so it keeps no points.
     pub fn prepare_display_job(&mut self, doc: &str, layer: &str, level: u32) -> Result<String, JsError> {
         let (input, pixels, mask) = self.engine.display_job_input(parse_id(doc)?, parse_id(layer)?, level).map_err(js_err)?;
-        self.job = (Some(pixels), mask, None);
+        self.job = (Some(pixels), mask, None, None);
         serde_json::to_string(&input).map_err(js_err)
     }
     /// The kept job buffer: the pixels, or the mask; null when there is none. A view on it is valid
@@ -79,13 +80,19 @@ impl WasmEngine {
     /// shape); null when the job has no selection. A view on it is valid until the next engine call.
     pub fn job_points_ptr(&self) -> *const u8 { self.job.2.as_ref().map_or(std::ptr::null(), |p| p.as_ptr() as *const u8) }
     pub fn job_points_len(&self) -> usize { self.job.2.as_ref().map_or(0, |p| p.len() * std::mem::size_of::<i32>()) }
-    pub fn release_job(&mut self) { self.job = (None, None, None); }
-    /// `Engine::install_job`: an edit job's result put back, if the layer still matches `stamp`.
-    pub fn install_job(&mut self, doc: &str, layer: &str, stamp_json: &str, output_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>) -> Result<String, JsError> {
+    /// The kept edit result halved to the canvas's level (`JobOutput.display`, F1); null when there is
+    /// none. A view on it is valid until the next engine call.
+    pub fn job_display_ptr(&self) -> *const u8 { self.job.3.as_ref().map_or(std::ptr::null(), |r| r.bytes().as_ptr()) }
+    pub fn job_display_len(&self) -> usize { self.job.3.as_ref().map_or(0, |r| r.bytes().len()) }
+    pub fn release_job(&mut self) { self.job = (None, None, None, None); }
+    /// `Engine::install_job`: an edit job's result put back, if the layer still matches `stamp`; its
+    /// pixels adopt `display`, their halving to the canvas's level, when the job made one.
+    pub fn install_job(&mut self, doc: &str, layer: &str, stamp_json: &str, output_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, display: Option<Vec<u8>>) -> Result<String, JsError> {
         let stamp: LayerStamp = serde_json::from_str(stamp_json).map_err(js_err)?;
         let output: JobOutput = serde_json::from_str(output_json).map_err(js_err)?;
         let (pixels, mask) = (raster_of(output.pixels, pixels)?, gray_of(output.mask, mask)?);
-        let dirty = self.engine.install_job(parse_id(doc)?, parse_id(layer)?, stamp, output, pixels, mask).map_err(js_err)?;
+        let display = raster_of(output.display.map(|d| (d.width, d.height)), display)?;
+        let dirty = self.engine.install_job(parse_id(doc)?, parse_id(layer)?, stamp, output, pixels, mask, display).map_err(js_err)?;
         serde_json::to_string(&dirty).map_err(js_err)
     }
     /// `Engine::has_effects_image`: whether the canvas's effects image for the layer is made already.
@@ -102,15 +109,16 @@ impl WasmEngine {
         let image = raster_of(Some((width, height)), Some(bytes))?.unwrap();
         self.engine.keep_effects_image(parse_id(doc)?, parse_id(layer)?, stamp, key, edit.as_ref(), image).map_err(js_err)
     }
-    /// `run_edit_job` (in the worker): the output as JSON; the buffers it replaced are kept. `points`
-    /// as `job_points_ptr` hands them out, when `input.selection` is not None.
-    pub fn run_edit_job(&mut self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, points: Option<Vec<u8>>, command_json: &str) -> Result<String, JsError> {
+    /// `run_edit_job` (in the worker): the output as JSON; the buffers it replaced, and their halving
+    /// to the canvas's level at `out_per_doc` (F1), are kept. `points` as `job_points_ptr` hands them
+    /// out, when `input.selection` is not None.
+    pub fn run_edit_job(&mut self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, points: Option<Vec<u8>>, command_json: &str, out_per_doc: f64) -> Result<String, JsError> {
         let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
         let command: Command = serde_json::from_str(command_json).map_err(js_err)?;
         let (pixels, mask) = (raster_of(input.pixels, pixels)?, gray_of(input.mask, mask)?);
         let points = points_of(input.selection.as_ref().map(JobSelection::point_count), points)?;
-        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), command).map_err(js_err)?;
-        self.job = (new_pixels, new_mask, None);
+        let (output, new_pixels, new_mask, display) = run_edit_job(&input, pixels, mask, points.as_deref(), command, out_per_doc).map_err(js_err)?;
+        self.job = (new_pixels, new_mask, None, display);
         serde_json::to_string(&output).map_err(js_err)
     }
     /// `run_histogram_job` (in the worker): four arrays of 256 bins, as JSON. `points` as
@@ -129,8 +137,8 @@ impl WasmEngine {
         let mask = gray_of(input.mask, mask)?;
         let edit = Self::parse_edit(edit_json)?;
         match run_effects_job(&input, pixels, mask, factor, edit.as_ref()).map_err(js_err)? {
-            Some((image, raster)) => { self.job = (Some(raster), None, None); Ok(Some(serde_json::to_string(&image).map_err(js_err)?)) }
-            None => { self.job = (None, None, None); Ok(None) }
+            Some((image, raster)) => { self.job = (Some(raster), None, None, None); Ok(Some(serde_json::to_string(&image).map_err(js_err)?)) }
+            None => { self.job = (None, None, None, None); Ok(None) }
         }
     }
     pub fn version(&self) -> String { Engine::version().to_string() }
```

```diff
--- a/engine/src/compositor.rs
+++ b/engine/src/compositor.rs
@@ -48,13 +48,8 @@ pub fn prefilters(sampling: Sampling, distorted: bool) -> bool { sampling != Sam
 
 /// Sharp halvings for large reductions: reduce until one output pixel covers at most 2 source pixels.
 fn prefiltered(raster: &Raster, pixels_per_output: f64) -> (Raster, f64) {
-    let mut current = raster.clone();
-    let mut scale = 1.0;
-    for _ in 0..prefilter_level(raster.width, raster.height, pixels_per_output) {
-        current = current.halved();
-        scale *= 0.5;
-    }
-    (current, scale)
+    let level = prefilter_level(raster.width, raster.height, pixels_per_output);
+    (raster.reduced(level), 0.5f64.powi(level as i32))
 }
 
 /// (x0, y0, x1, y1) of the pixels with alpha > 0, x1/y1 exclusive; None when fully transparent.
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -665,15 +665,11 @@ impl Engine {
     pub fn can_toggle_clipping(&self, id: Uuid, layer: Uuid) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_toggle_clipping(&self.session(id)?.document, layer)) }
     pub fn can_place(&self, id: Uuid, layer: Uuid, parent: Option<Uuid>) -> Result<bool, CommandError> { Ok(ops::hierarchy::can_place(&self.session(id)?.document, layer, parent)) }
 
-    /// The layer's raster after `level` sharp halvings, through any open preview.
+    /// The layer's raster after `level` sharp halvings, through any open preview: from a halving the
+    /// pixels adopted from the job worker when there is one (`Raster::reduced`, F1).
     pub fn layer_raster(&self, id: Uuid, layer: Uuid, level: u32) -> Result<Option<Raster>, CommandError> {
         let doc = self.render_bytes(id)?;
-        let Some(mut raster) = doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone() else { return Ok(None); };
-        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
-            if raster.width <= 1 || raster.height <= 1 { break; }
-            raster = raster.halved();
-        }
-        Ok(Some(raster))
+        Ok(doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.as_ref().map(|raster| raster.reduced(level)))
     }
     /// The raster the plan's draw of `layer` samples, through any open preview and pending edit,
     /// after `level` sharp halvings: the layer with its effects around it when the plan draws them
@@ -685,12 +681,7 @@ impl Engine {
         let doc = self.render_bytes(id)?;
         let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         let raster = match effects_draw(l, edit) { Some(fx) => self.effects.image(l, &fx), None => l.pixels.clone() };
-        let Some(mut raster) = raster else { return Ok(None); };
-        for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
-            if raster.width <= 1 || raster.height <= 1 { break; }
-            raster = raster.halved();
-        }
-        Ok(Some(raster))
+        Ok(raster.map(|raster| raster.reduced(level)))
     }
     /// The stored document with the adjustment layer and everything above it hidden: what renders
     /// beneath it, as macOS renders the layers underneath for its histogram and eyedroppers.
```

```diff
--- a/engine/src/jobs.rs
+++ b/engine/src/jobs.rs
@@ -79,7 +79,9 @@ pub struct JobInput {
 }
 
 /// What an edit left of its layer: the transform, the mask's placement, which buffers it replaced
-/// (their sizes; the buffers travel beside) and the rectangles it changed them within.
+/// (their sizes; the buffers travel beside), the rectangles it changed them within, and the new
+/// pixels already halved to the level the canvas draws them at (`display`: the level and the size;
+/// the halved buffer travels beside, the fourth job buffer, F1).
 #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
 #[serde(rename_all = "camelCase")]
 pub struct JobOutput {
@@ -88,8 +90,15 @@ pub struct JobOutput {
     pub pixels: Option<(u32, u32)>,
     pub mask: Option<(u32, u32)>,
     pub regions: Vec<(Plane, PixelRect)>,
+    #[serde(default)]
+    pub display: Option<DisplayHalving>,
 }
 
+/// The new pixels after `level` halvings (`width` x `height`): what the canvas uploads at the zoom the
+/// edit was asked at, made in the worker so the UI thread never halves a large result (F1).
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
+pub struct DisplayHalving { pub level: u32, pub width: u32, pub height: u32 }
+
 /// An effects image made by a job: its size and the transparent pixels added on every side.
 #[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
 pub struct EffectsImage { pub width: u32, pub height: u32, pub inset: u32 }
@@ -196,10 +205,18 @@ impl Engine {
     /// goes through `Layer::mask_mut`, which moves the revision, so a mask moved while the job ran
     /// (a nudge, a Transform of the mask alone, an undo of either) refuses the result as a pixel edit
     /// would.
-    pub fn install_job(&mut self, id: Uuid, layer: Uuid, stamp: LayerStamp, output: JobOutput, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Result<Dirty, CommandError> {
-        if pixels.as_ref().map(|p| (p.width, p.height)) != output.pixels || mask.as_ref().map(|m| (m.width, m.height)) != output.mask {
+    ///
+    /// `display` is the new pixels already halved to the canvas's level (`output.display`), which the
+    /// new pixels adopt (`Raster::adopt`): the next frame uploads it without halving the layer on the
+    /// UI thread (F1).
+    pub fn install_job(&mut self, id: Uuid, layer: Uuid, stamp: LayerStamp, output: JobOutput, pixels: Option<Raster>, mask: Option<GrayRaster>, display: Option<Raster>) -> Result<Dirty, CommandError> {
+        if pixels.as_ref().map(|p| (p.width, p.height)) != output.pixels || mask.as_ref().map(|m| (m.width, m.height)) != output.mask
+            || display.as_ref().map(|d| (d.width, d.height)) != output.display.map(|d| (d.width, d.height)) {
             return Err(CommandError::Argument("a job's buffers do not match its output".into()));
         }
+        if let (Some(p), Some(d), Some(spec)) = (&pixels, &display, output.display) {
+            if !p.adopt(spec.level, d.clone()) { return Err(CommandError::Argument("a job's display halving does not match its pixels".into())); }
+        }
         self.clear_preview(id);
         self.edit(id, |doc, _| {
             let now = LayerStamp::of(doc, doc.layer(layer).ok_or(CommandError::NoLayer)?);
@@ -234,21 +251,34 @@ impl Engine {
 /// Runs an edit job: `command` on the job's document, and what it left of the layer. Only the
 /// buffers the command replaced come back. `points` is the selection's flat buffer, as `job_input`
 /// returns it; required exactly when `input.selection` is.
-pub fn run_edit_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>, command: Command) -> Result<(JobOutput, Option<Raster>, Option<GrayRaster>), CommandError> {
+///
+/// `out_per_doc` is the scale the canvas draws the document at (device pixels per document pixel; 0
+/// for none): new pixels that changed as a whole are also returned halved to the level the canvas
+/// uploads them at (`compositor::prefilter_level`, the renderers' rule, on the result's own grid, so
+/// a blank layer a fill grew to the canvas counts too), which `install_job` hands them to adopt (F1).
+/// None when that level is 0, for Nearest sampling (never prefiltered), and when the edit reports
+/// changed rectangles (the UI thread then seeds the halvings from the old pixels).
+pub fn run_edit_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, points: Option<&[i32]>, command: Command, out_per_doc: f64) -> Result<(JobOutput, Option<Raster>, Option<GrayRaster>, Option<Raster>), CommandError> {
     let mut engine = Engine::new();
     let id = engine.insert_document(input.document(pixels.clone(), mask.clone(), points)?);
     let dirty = engine.execute(id, command)?;
     let l = engine.document(id).and_then(|d| d.layer(input.layer.id)).ok_or(CommandError::NoLayer)?;
     let new_pixels = l.pixels.clone().filter(|p| !pixels.as_ref().is_some_and(|o| o.same_pixels(p)));
     let new_mask = l.mask.as_ref().map(|m| m.pixels.clone()).filter(|m| !mask.as_ref().is_some_and(|o| o.same_pixels(m)));
+    let regions: Vec<(Plane, PixelRect)> = dirty.regions.iter().filter(|r| r.layer == input.layer.id).map(|r| (r.plane, r.rect)).collect();
+    let display = new_pixels.as_ref().filter(|_| out_per_doc > 0.0 && l.transform.sampling != Sampling::Nearest && !regions.iter().any(|(plane, _)| *plane == Plane::Pixels))
+        .map(|p| (compositor::prefilter_level(p.width, p.height, p.width as f64 / (l.transform.size.width * out_per_doc).max(1e-9)), p))
+        .filter(|(level, _)| *level > 0)
+        .map(|(level, p)| (level, p.reduced(level)));
     let output = JobOutput {
         transform: l.transform,
         mask_placement: l.mask.as_ref().and_then(|m| m.placement),
         pixels: new_pixels.as_ref().map(|p| (p.width, p.height)),
         mask: new_mask.as_ref().map(|m| (m.width, m.height)),
-        regions: dirty.regions.iter().filter(|r| r.layer == input.layer.id).map(|r| (r.plane, r.rect)).collect(),
+        regions,
+        display: display.as_ref().map(|(level, d)| DisplayHalving { level: *level, width: d.width, height: d.height }),
     };
-    Ok((output, new_pixels, new_mask))
+    Ok((output, new_pixels, new_mask, display.map(|(_, d)| d)))
 }
 
 /// Runs a histogram job: the layer's histogram weighted by the selection (`Engine::histogram`).
```

```diff
--- a/engine/src/raster.rs
+++ b/engine/src/raster.rs
@@ -1,3 +1,4 @@
+use crate::compositor::MAX_PREFILTER_LEVEL;
 use std::sync::{Arc, Mutex};
 
 pub const TILE: u32 = 256;
@@ -67,8 +68,10 @@ fn halve_into(src: &[u8], sw: u32, sh: u32, out: &mut [u8], region: PixelRect) {
 /// Backing storage for a `Raster`: the pixels, plus a memoized single-step box-reduction so
 /// `halved()` computed once for a given pixel buffer is shared by every clone of it (and by
 /// every clone of the halved result in turn), instead of being recomputed on every frame.
+/// `adopted` is one deeper halving made elsewhere (the job worker, Phase 4.5 F1): the raster after
+/// that many halvings, handed over with the pixels so the UI thread never halves them itself.
 /// `std::sync::Mutex` is fine here: the engine runs single-threaded on wasm32.
-struct RasterInner { data: Vec<u8>, half: Mutex<Option<Raster>> }
+struct RasterInner { data: Vec<u8>, half: Mutex<Option<Raster>>, adopted: Mutex<Option<(u32, Raster)>> }
 
 impl std::fmt::Debug for RasterInner {
     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
@@ -91,7 +94,7 @@ impl PartialEq for Raster {
 
 impl Raster {
     fn wrap(width: u32, height: u32, data: Vec<u8>) -> Raster {
-        Raster { width, height, inner: Arc::new(RasterInner { data, half: Mutex::new(None) }) }
+        Raster { width, height, inner: Arc::new(RasterInner { data, half: Mutex::new(None), adopted: Mutex::new(None) }) }
     }
     pub fn new_transparent(width: u32, height: u32) -> Raster {
         Raster::wrap(width, height, vec![0; (width as usize) * (height as usize) * 4])
@@ -138,9 +141,13 @@ impl Raster {
     pub fn same_pixels(&self, other: &Raster) -> bool { Arc::ptr_eq(&self.inner, &other.inner) }
     /// The pixel buffer's identity: equal for every clone sharing it (history counts buffers by it).
     pub fn buffer_id(&self) -> usize { Arc::as_ptr(&self.inner) as *const u8 as usize }
-    /// Drops the memoized halving of this buffer, for every clone that shares it (history lets go of
-    /// the halvings of buffers only it holds: they are not counted against its limit).
-    pub fn forget_halvings(&self) { *self.inner.half.lock().unwrap() = None; }
+    /// Drops the memoized halving of this buffer, and a halving it adopted, for every clone that shares
+    /// it (history lets go of the halvings of buffers only it holds: they are not counted against its
+    /// limit).
+    pub fn forget_halvings(&self) {
+        *self.inner.half.lock().unwrap() = None;
+        *self.inner.adopted.lock().unwrap() = None;
+    }
     /// Whether another clone of this raster holds the same pixel buffer.
     pub fn shared(&self) -> bool { Arc::strong_count(&self.inner) > 1 }
     /// How many handles hold these pixels, this one included.
@@ -178,6 +185,38 @@ impl Raster {
     pub fn half_size(&self) -> (u32, u32) { ((self.width / 2).max(1), (self.height / 2).max(1)) }
     /// The memoized halving, if it has been made (and not let go).
     pub fn memoized_half(&self) -> Option<Raster> { self.inner.half.lock().unwrap().clone() }
+    /// The size after `level` halvings: half of each side each time, at least 1, stopping once a side
+    /// is 1 (the rule `reduced` and both renderers' prefilter follow).
+    pub fn size_at_level(&self, level: u32) -> (u32, u32) {
+        let (mut w, mut h) = (self.width, self.height);
+        for _ in 0..level.min(MAX_PREFILTER_LEVEL) {
+            if w <= 1 || h <= 1 { break; }
+            w = (w / 2).max(1);
+            h = (h / 2).max(1);
+        }
+        (w, h)
+    }
+    /// The raster after `level` halvings, stopping once a side is 1: from the halving this buffer
+    /// adopted when that is `level` or fewer halvings, else from the pixels; each step is memoized.
+    pub fn reduced(&self, level: u32) -> Raster {
+        let level = level.min(MAX_PREFILTER_LEVEL);
+        let (mut current, mut at) = match self.adopted() { Some((l, r)) if l <= level => (r, l), _ => (self.clone(), 0) };
+        while at < level && current.width > 1 && current.height > 1 {
+            current = current.halved();
+            at += 1;
+        }
+        current
+    }
+    /// Keeps `reduced`, this raster after `level` halvings made elsewhere (the job worker), for every
+    /// clone of it: `reduced(level)` then returns it without halving anything. Refused (false) at level
+    /// 0 or when it is not the size `level` halvings make.
+    pub fn adopt(&self, level: u32, reduced: Raster) -> bool {
+        if level == 0 || level > MAX_PREFILTER_LEVEL || (reduced.width, reduced.height) != self.size_at_level(level) { return false; }
+        *self.inner.adopted.lock().unwrap() = Some((level, reduced));
+        true
+    }
+    /// The halving this buffer adopted, and after how many halvings it is.
+    pub fn adopted(&self) -> Option<(u32, Raster)> { self.inner.adopted.lock().unwrap().clone() }
     /// Gives this raster the halvings `parent` has already made, for a raster that equals `parent`
     /// outside `rect` (a changed rectangle: an edit that kept the grid). Each kept level is copied and
     /// only the part `rect` reaches is halved again, so the result is bit-identical to halving from
@@ -191,6 +230,7 @@ impl Raster {
     /// `seed_halvings`, Task 3 fix round 1, bug 1).
     pub fn seed_halvings(&self, parent: &Raster, rect: PixelRect) {
         if self.width != parent.width || self.height != parent.height || self.same_pixels(parent) { return; }
+        self.seed_adopted(parent, rect);
         let Some(parent_half) = parent.memoized_half() else { return };
         let (w, h) = self.half_size();
         // Every output pixel whose 2 x 2 block meets `rect`.
@@ -201,6 +241,33 @@ impl Raster {
         half.seed_halvings(&parent_half, reach);
         *self.inner.half.lock().unwrap() = Some(half);
     }
+    /// `seed_halvings` for a halving `parent` adopted (F1): the same halving for this raster, with the
+    /// part `rect` reaches made again from this raster's pixels, so an edit inside a selection after a
+    /// job's result still halves only its rectangle. Only while every step halves whole 2 x 2 blocks
+    /// (both sides at least 2 before the last step), where a block-aligned part halves exactly as the
+    /// whole raster does.
+    fn seed_adopted(&self, parent: &Raster, rect: PixelRect) {
+        let Some((level, parent_reduced)) = parent.adopted() else { return };
+        if (self.width >> (level - 1)) < 2 || (self.height >> (level - 1)) < 2 { return; }
+        let (mut reach, mut w, mut h) = (rect, self.width, self.height);
+        for _ in 0..level {
+            w /= 2;
+            h /= 2;
+            reach = reach.halved(w, h);
+        }
+        let mut data = parent_reduced.bytes().to_vec();
+        if !reach.is_empty() {
+            let f = 1u32 << level;
+            let mut part = self.cropped(reach.x * f, reach.y * f, reach.width * f, reach.height * f);
+            for _ in 0..level { part = part.halved(); }
+            let row = reach.width as usize * 4;
+            for y in 0..reach.height as usize {
+                let at = ((reach.y as usize + y) * w as usize + reach.x as usize) * 4;
+                data[at..at + row].copy_from_slice(&part.bytes()[y * row..(y + 1) * row]);
+            }
+        }
+        *self.inner.adopted.lock().unwrap() = Some((level, Raster::from_premultiplied(w, h, data)));
+    }
     /// A copy of the `w` x `h` rectangle at (x, y); clamped to the raster.
     pub fn cropped(&self, x: u32, y: u32, w: u32, h: u32) -> Raster {
         let x1 = (x + w).min(self.width); let y1 = (y + h).min(self.height);
```


- [ ] **Step 4: Run the whole set**

`cargo test -p compositor-engine`: 558 passed, 10 ignored (70 binaries). `pnpm wasm:dev`; `pnpm test`: 241 in 37 files; `pnpm build` clean; `pnpm e2e`: 160 passed, 19 skipped (the perf file skips without PERF).

Then the release timings: `pnpm wasm`, `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts -g "F1"`, `$env:PERF = ""`, `pnpm wasm:dev`. Measured on the scratch clone (HD 520, "ANGLE (Intel, Intel(R) HD Graphics 520 (0x00001916) Direct3D11 vs_5_0 ps_5_0, D3D11)"): 24 MP Levels `jobInput` 65 ms, OK 66, `installJob` 86, the frame after 8, the longest gap while the worker edits 18, `layerPixels` in that frame 0, display buffer 6,000,000 bytes at scale 0.173; 24 MP fill `installJob` 45, frame 21, gap 18; 100 MP Levels `jobInput` 273, `installJob` 299, frame 12, gap 23, display 6,250,000 bytes (level 3, 1250 x 1250); 100 MP fill `installJob` 244, frame 23, gap 20. Cold warm-ups (logged only): 1149, 339, 656 and 319 ms. The release wasm grows by 7,229 bytes (2,978,545).

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `raster.rs` `reduced` ignores the adopted halving (halves from level 0). Two `display_level` tests fail (measured, after `crossed` was added; before it they passed, which is why it exists).
2. `seed_adopted` returns at once. `an_edit_inside_a_selection_after_a_job_redoes_only_its_rectangle_of_the_adopted_halving` fails (measured).
3. `jobs.rs` drops the pixel-region check. `nothing_is_halved_inside_a_selection_for_nearest_sampling_or_at_full_size` fails (measured).
4. `store.ts` sends `outPerDoc: 0`. The store-jobs unit test fails (measured).
5. `job-worker.ts` sends `display: null`. The F1 e2e in `jobs.spec.ts` fails (measured).

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/perf-4-5.spec.ts engine/tests/display_level.rs
git commit -m "feat: F1: the job worker halves an edit's result to the level the canvas draws it at, and the install adopts it" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/engine/client.ts app/src/engine/job-worker.ts app/src/engine/jobs.ts app/src/state/store.ts app/tests/e2e/jobs.spec.ts app/tests/e2e/perf-4-5.spec.ts app/tests/unit/store-jobs.test.ts engine-wasm/src/lib.rs engine/src/compositor.rs engine/src/engine.rs engine/src/jobs.rs engine/src/raster.rs engine/tests/display_level.rs engine/tests/jobs.rs engine/tests/mask_grow.rs engine/tests/raster_edits.rs
```


---

### Task 4: The fix wave's residuals

Four small things the 4b-1 re-review left: a mask chip clicked while a job's result is to come half-applied (the layer click was refused, the mask target was not); a layer click that applies a pending gradient through the worker went on to `SetActiveLayer` and cleared the preview the job keeps on screen; `jobs.warm()` ran before the file-drop listener and, if the worker could not be made, stopped the app starting; a failed effects image reached only the console.

**Files:**
- Modify: `app/src/state/store.ts` (`selectLayers` re-checks `working` after `commitGradient`; `setMaskSelected` returns while working, LayerMask.swift:222-228), `app/src/App.tsx` (`jobs.warm()` last, its failure said in the banner), `app/src/canvas/effects-images.ts` (a `failed` callback), `app/src/canvas/renderer.ts`, `app/src/canvas/gl-renderer.ts`, `app/src/canvas/CanvasView.tsx` (the callback reaches `setError`)
- Modify (tests): `app/tests/unit/gradient-store.test.ts`, `app/tests/unit/effects-images.test.ts`, `app/tests/e2e/smoke.spec.ts`

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/smoke.spec.ts
+++ b/app/tests/e2e/smoke.spec.ts
@@ -45,3 +45,15 @@ test("phase 2 client calls reach the engine", async ({ page }) => {
   expect(result.autoIsIdentity).toBe(true);   // a blank document has nothing to stretch
   expect(result.drawHasAdjustment).toBe(true);
 });
+
+test("a job worker that cannot be made does not stop the app starting, and says why", async ({ page }) => {
+  // A policy that forbids workers: the constructor throws. Startup goes on (the engine, the test API,
+  // file drops) and the banner names the worker (re-review residual: jobs.warm() used to run first).
+  await page.addInitScript(() => {
+    (window as unknown as { Worker: unknown }).Worker = class { constructor() { throw new Error("workers are blocked here"); } };
+  });
+  await page.goto("/");
+  await expect(page.getByTestId("engine-ready")).toBeVisible();
+  await expect(page.getByTestId("error-banner")).toContainText("The job worker could not start: workers are blocked here");
+  expect(await page.evaluate(() => typeof (window as unknown as { __compositor?: { store?: unknown } }).__compositor?.store)).toBe("function");
+});
\ No newline at end of file
```

```diff
--- a/app/tests/unit/effects-images.test.ts
+++ b/app/tests/unit/effects-images.test.ts
@@ -2,7 +2,7 @@ import { afterEach, describe, expect, it } from "vitest";
 import { EFFECTS_LIMITS, EffectsImages, closedMeanwhile, placedLike, reducedLevel } from "../../src/canvas/effects-images";
 import type { Corners, LayerDraw, LayerTransform } from "../../src/engine/types";
 import type { EngineClient } from "../../src/engine/client";
-import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";
+import { EFFECTS_JOB_DISPLACED, type JobClient, type JobRequest, type JobResult } from "../../src/engine/jobs";
 import { homographyUnitTo, mat3Apply } from "../../src/tools/transform-geometry";
 
 const defaults = { ...EFFECTS_LIMITS };
@@ -58,23 +58,26 @@ describe("EffectsImages.choose", () => {
    * when `finish` is called. */
   function setup() {
     const asked: JobRequest[] = [];
-    const state = { full: false, kept: 0, closed: false };
+    const state = { full: false, kept: 0, closed: false, broken: null as string | null };
     let finish: (r: JobResult | null) => void = () => {};
+    let fail: (e: Error) => void = () => {};
+    const failures: string[] = [];
     const engine = {
       hasEffectsImage: () => state.full,
       displayJobInput: (_d: string, _l: string, level: number) => {
         if (state.closed) throw new Error("No document with that id."); // the engine's NoDocument, word for word
+        if (state.broken) throw new Error(state.broken);
         return { input: JSON.stringify({ pixels: [40 >> level, 24 >> level], stamp: {} }), pixels: new ArrayBuffer(4), mask: null };
       },
       keepEffectsImage: () => { state.kept++; return true; },
     } as unknown as EngineClient;
-    const jobs = { run: (_c: string, r: JobRequest) => { asked.push(r); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
+    const jobs = { run: (_c: string, r: JobRequest) => { asked.push(r); return new Promise<JobResult | null>((resolve, reject) => { finish = resolve; fail = reject; }); } } as unknown as JobClient;
     let landed = 0;
     // Frames are painted only when the test says so (`paint`).
     const frames: (() => void)[] = [];
-    const images = new EffectsImages(() => jobs, () => { landed++; }, (f) => { frames.push(f); });
+    const images = new EffectsImages(() => jobs, () => { landed++; }, (f) => { frames.push(f); }, (m) => { failures.push(m); });
     const paint = () => { for (const f of frames.splice(0)) f(); };
-    return { images, engine, asked, state, finish: (r: JobResult | null) => finish(r), landed: () => landed, paint };
+    return { images, engine, asked, state, failures, finish: (r: JobResult | null) => finish(r), fail: (e: Error) => fail(e), landed: () => landed, paint };
   }
   const flush = () => new Promise((r) => setTimeout(r, 0));
 
@@ -173,4 +176,26 @@ describe("EffectsImages.choose", () => {
     await flush();
     expect(closed.asked.length, "asked again").toBe(1);
   });
+
+  it("tells the user of a failure that is not a closed document or a displaced job (re-review residual: it reached the console only)", async () => {
+    const t = setup();
+    // An ask the engine refuses for another reason: said, not thrown into a task nobody catches.
+    t.state.broken = "RangeError: Array buffer allocation failed";
+    t.images.choose(t.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
+    await flush();
+    expect(t.failures).toEqual(["RangeError: Array buffer allocation failed"]);
+    // A job that fails is said too.
+    t.state.broken = null;
+    t.images.choose(t.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
+    await flush();
+    t.fail(new Error("The edit failed and was stopped."));
+    await flush();
+    expect(t.failures).toEqual(["RangeError: Array buffer allocation failed", "The edit failed and was stopped."]);
+    // One an edit displaced is asked for again on the next frame, and not said.
+    t.images.choose(t.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
+    await flush();
+    t.fail(new Error(EFFECTS_JOB_DISPLACED));
+    await flush();
+    expect([t.failures.length, t.landed()]).toEqual([2, 1]);
+  });
 });
```

```diff
--- a/app/tests/unit/gradient-store.test.ts
+++ b/app/tests/unit/gradient-store.test.ts
@@ -79,6 +79,25 @@ describe("a layer click while a job's result is to come (final review minor 1)",
     s().selectLayers(["B"], "B");
     expect(log).toEqual([`execute ${JSON.stringify({ type: "SetActiveLayer", id: "B" })}`]);
   });
+  it("a mask chip clicked meanwhile targets nothing either: the layer's click is refused and so is the target (re-review residual)", () => {
+    useEditor.setState({ working: true });
+    // What LayersList's mask chip does: choose the layer, then target its mask.
+    s().selectLayers(["A"], "A");
+    s().setMaskSelected(true);
+    expect([s().maskSelected, s().maskTargeted(), s().error]).toEqual([false, false, BUSY_MESSAGE]);
+  });
+  it("a click that applies a pending gradient through the worker waits too: no SetActiveLayer to clear the preview the job keeps (re-review residual)", () => {
+    draw([5, 6], [30, 6]);
+    const sent: string[] = [];
+    const jobs = { run: (_channel: string, msg: { command?: string }) => { if (msg.command) sent.push(JSON.parse(msg.command).type as string); return new Promise(() => {}); } } as unknown as JobClient;
+    useEditor.setState({ jobPixels: 1, jobs });
+    const shown = previews.length;
+    s().selectLayers(["B"], "B");
+    expect(sent, "the gradient went to the worker").toEqual(["Gradient"]);
+    expect(log.filter((l) => l.includes("SetActiveLayer"))).toEqual([]);
+    expect([s().selectedLayerIds, s().error, s().working]).toEqual([["A"], BUSY_MESSAGE, true]);
+    expect(previews.length, "nothing cleared the gradient's preview").toBe(shown);
+  });
 });
 
 describe("a pending gradient", () => {
```


- [ ] **Step 2: Run them and see them fail**

Run: `pnpm test -- gradient-store effects-images`
Expected: the two new gradient-store tests fail (the mask chip becomes the target; `SetActiveLayer` is sent), the effects-images test fails (`failures` stays empty). `npx playwright test app/tests/e2e/smoke.spec.ts -g "cannot be made"` fails: `engine-ready` never shows.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -60,8 +60,6 @@ export function App() {
       // The job worker: a second engine for work on one layer off the UI thread (engine jobs.rs).
       const jobs = new JobClient(engine.module, () => new Worker(new URL("./engine/job-worker.ts", import.meta.url), { type: "module" }));
       useEditor.getState().setJobs(jobs);
-      // Started now, not by the first large edit (final review F2).
-      jobs.warm();
       installTestApi({ engine, bridge, store: useEditor });
       bridge.onFileDrop((paths, position) => {
         const projects = paths.filter((p) => p.toLowerCase().endsWith(".comp"));
@@ -85,6 +83,11 @@ export function App() {
           void importImages(images, at);
         }
       });
+      // Started now, not by the first large edit (final review F2), and last: a worker that cannot be
+      // made (`new Worker` throws) must not stop the rest of startup. It says why in the banner; each
+      // large edit then tries again, and fails with its own message.
+      try { jobs.warm(); }
+      catch (e) { useEditor.getState().setError(`The job worker could not start: ${e instanceof Error ? e.message : String(e)}`); }
       setVersion(engine.version());
       setReady(true);
     }).catch((e) => setError(String(e)));
```

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -91,7 +91,8 @@ export function CanvasView() {
   // Renderer lifetime follows the canvas element.
   useEffect(() => {
     if (!engine || !glRef.current) return;
-    const renderer = createRenderer(glRef.current, { jobs: () => useEditor.getState().jobs, landed: () => useEditor.getState().invalidate() });
+    const renderer = createRenderer(glRef.current, { jobs: () => useEditor.getState().jobs, landed: () => useEditor.getState().invalidate(),
+      failed: (message) => useEditor.getState().setError(message) });
     rendererRef.current = renderer;
     useEditor.getState().setRendererKind(renderer.kind);
     installTestApi({
```

```diff
--- a/app/src/canvas/effects-images.ts
+++ b/app/src/canvas/effects-images.ts
@@ -66,9 +66,12 @@ export class EffectsImages {
    * at 24 MP) and the render that halves and uploads it (60-109 ms) then never share one frame. */
   private held = new Map<string, string>();
 
-  /** `nextFrame` runs its callback once a frame has been painted after this one (tests pass their own). */
+  /** `nextFrame` runs its callback once a frame has been painted after this one (tests pass their own).
+   * `failed` is told a failure the user should read: an ask the engine refused for another reason than
+   * a closed document or layer, or a job that failed (not one an edit displaced). */
   constructor(private readonly jobs: () => JobClient | null, private readonly landed: () => void,
-    private readonly nextFrame: (f: () => void) => void = (f) => requestAnimationFrame(() => requestAnimationFrame(f))) {}
+    private readonly nextFrame: (f: () => void) => void = (f) => requestAnimationFrame(() => requestAnimationFrame(f)),
+    private readonly failed: (message: string) => void = (message) => console.error(message)) {}
 
   /** For a large styled layer this frame: "full" when the engine has the full-size image (draw it the
    * usual way), else the reduced image to draw (null: none yet, draw the layer plainly). Asks the
@@ -111,8 +114,8 @@ export class EffectsImages {
           catch (e) {
             this.asked.delete(k);
             // The document or layer closed meanwhile: nothing to ask for. Anything else is a real
-            // failure and is not swallowed (final review minor 13).
-            if (!closedMeanwhile(e)) throw e;
+            // failure, and the user is told (final review minor 13; re-review: it reached the console only).
+            if (!closedMeanwhile(e)) this.failed(e instanceof Error ? e.message : String(e));
           }
         }, 0);
       } else {
@@ -151,8 +154,9 @@ export class EffectsImages {
       settled();
       // An edit or histogram job displaced this one (fix round 1, issue 3): it still needs making,
       // so the next frame must notice and ask again -- unlike an ordinary refusal or a dead worker,
-      // which leave the picture exactly as it was, nothing new to redraw for.
+      // which leave the picture exactly as it was, nothing new to redraw for, and are said.
       if (e instanceof Error && e.message === EFFECTS_JOB_DISPLACED) this.landed();
+      else this.failed(e instanceof Error ? e.message : String(e));
     });
   }
```

```diff
--- a/app/src/canvas/gl-renderer.ts
+++ b/app/src/canvas/gl-renderer.ts
@@ -51,7 +51,7 @@ export class GlRenderer implements Renderer {
   constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext, hooks: RenderHooks = { jobs: () => null, landed: () => {} }) {
     this.textures = new LayerTextures(gl); this.masks = new MaskTextures(gl); this.fbos = new FboPool(gl); this.programs = createPrograms(gl);
     this.adjustTextures = new AdjustTextures(gl);
-    this.effectsImages = new EffectsImages(hooks.jobs, hooks.landed);
+    this.effectsImages = new EffectsImages(hooks.jobs, hooks.landed, undefined, hooks.failed);
     this.white = this.solid(gl.R8, gl.RED, [255]); this.transparent = this.solid(gl.RGBA8, gl.RGBA, [0, 0, 0, 0]);
     this.maxTexture = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
   }
```

```diff
--- a/app/src/canvas/renderer.ts
+++ b/app/src/canvas/renderer.ts
@@ -7,8 +7,9 @@ import { CpuRenderer } from "./cpu-renderer";
 
 export interface RenderOptions { checkerboard: boolean; }
 /** What a renderer may use besides the engine: the job worker (for large styled layers' effects
- * images, effects-images.ts) and a way to ask for another frame when one of its results lands. */
-export interface RenderHooks { jobs: () => JobClient | null; landed: () => void; }
+ * images, effects-images.ts), a way to ask for another frame when one of its results lands, and a way
+ * to say that making one failed (the error banner). */
+export interface RenderHooks { jobs: () => JobClient | null; landed: () => void; failed?: (message: string) => void; }
 export interface Renderer {
   readonly kind: "gl" | "cpu";
   /** Uploads whatever it needs and draws. There is no separate sync step: the GL renderer's
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -696,6 +696,9 @@ export const useEditor = create<EditorStore>((set, get) => ({
     get().commitTransform();
     // Choosing a layer applies a pending gradient first (`resolveGradient`).
     get().commitGradient();
+    // On a large target that sends the gradient to the job worker: the click waits, as `run` does, or
+    // its SetActiveLayer would clear the preview the job keeps on screen until the result lands.
+    if (get().working) { set({ error: BUSY_MESSAGE }); return; }
     if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
     set({ selectedLayerIds: valid, maskSelected: false });
     get().refresh(activeId);
@@ -703,7 +706,10 @@ export const useEditor = create<EditorStore>((set, get) => ({
   // Quiet: the chip handlers that call this have just had `selectLayers` raise the banner. Targeting a
   // mask closes a picker open on a swatch: a mask's palette is black and white (ColorPaletteControls.swift:53-56).
   setMaskSelected: (v) => {
-    if (get().panelOwnsDocument()) return;
+    // Nor while a job's result is to come: the Mac changes no target then (`selectLayerTarget`,
+    // LayerMask.swift:222-228 at v1.4.5, `guard !isProjectBusy`). Quiet too: the chip's own
+    // `selectLayers` has just raised the banner.
+    if (get().panelOwnsDocument() || get().working) return;
     // Changing the target applies a pending gradient first.
     if (v !== get().maskSelected) get().commitGradient();
     set({ maskSelected: v });
```


- [ ] **Step 4: Run the whole set**

No engine change: `pnpm test` 244 in 37 files; `pnpm build` clean; `pnpm e2e` 161 passed, 19 skipped.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `setMaskSelected` without its `working` guard: the mask chip test fails (measured).
2. `selectLayers` without the re-check after `commitGradient`: the gradient click test fails (measured).
3. `effects-images.ts`'s catch logs instead of calling `failed`: the effects-images test fails (measured).
4. `jobs.warm()` back first and unwrapped: the smoke e2e fails, `engine-ready` not found (measured).

- [ ] **Step 6: Commit**

```
git commit -m "fix(app): a chip or layer click waits while a job's result is to come, the job worker starts last, and failed effects images say why" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/App.tsx app/src/canvas/CanvasView.tsx app/src/canvas/effects-images.ts app/src/canvas/gl-renderer.ts app/src/canvas/renderer.ts app/src/state/store.ts app/tests/e2e/smoke.spec.ts app/tests/unit/effects-images.test.ts app/tests/unit/gradient-store.test.ts
```


---

### Task 5: Open and save project format 11

Compositor 1.4.5 writes format 11: text layers may carry `colorRuns` (from format 10) and `fontRuns` (from 11). The port reads 1 to 11, keeps both kinds of run exactly as they came, refuses what the Mac refuses, and writes 11 (ruling OQ5). The port has no text tool, so it never makes a run.

**Files:**
- Modify: `engine/src/manifest.rs` (`CURRENT_VERSION = 11`, `text_is_valid(text, version)` with `runs_are_valid`, `colour_run_is_valid`, `font_run_is_valid`, `swift_int`), `engine/src/error.rs` (the version message)
- Modify (tests in source files): `src-tauri/src/commands/package.rs` (a Mac save opens, its QuickLook preview ignored), `src-tauri/src/atomic.rs` (saving over it drops the preview)
- Create: `engine/tests/format_v11.rs`, `engine/tests/fixtures/hand-written-1.4.5/text-runs.manifest.json`, `engine/tests/fixtures/hand-written-1.4.5/text-colour-only.manifest.json`, `app/tests/e2e/format-v11.spec.ts`
- Modify (tests): `engine/tests/format_v9.rs`, `engine/tests/interop.rs`, `engine/tests/manifest.rs`, `engine/tests/adjust_settings.rs`, `engine/tests/package.rs` (they pinned "writes 9"; they now follow `CURRENT_VERSION`), `engine/tests/mac_probes.rs` (A5 `port-v11-roundtrip`), `app/tests/e2e/adjust-layers.spec.ts` and `app/tests/e2e/mac-1.2.6.spec.ts` (their saved-version pins)

**Mac:** TypeTool.swift (the run records and their rules), ProjectStore.swift:141 (every save writes the current version) and its version gates, Character.isNewline.

The two fixtures are hand-written from the Mac's Codable encoding (sorted keys, `" : "` separators, runs as `location` / `length` in UTF-16 units) and say so in their names (LL-069): they are not a Mac save. The real format-11 Mac save this task proves against is Task 1's `shapes.mac-1.3.7.comp`. The user's A1 and A2 saves replace the hand-written pair when they come.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/adjust-layers.spec.ts
+++ b/app/tests/e2e/adjust-layers.spec.ts
@@ -109,7 +109,8 @@ test("an adjustment layer saves, reopens and still renders the same", async ({ p
     return { same: before.every((v, i) => v === after[i]), version: manifest.version, adjustment: manifest.layers[1].adjustment };
   });
   expect(saved.same).toBe(true);
-  expect(saved.version).toBe(9);
+  // Written at 11, as Compositor for Mac 1.4.5 writes every save (ProjectStore.swift:15).
+  expect(saved.version).toBe(11);
   expect(saved.adjustment.kind).toBe("Gradient Map");
   expect(saved.adjustment.gradientMapSettings).toBeTruthy();
   expect(saved.adjustment.exposureSettings).toBeUndefined();
```

Create `app/tests/e2e/format-v11.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { clickMenu } from "./helpers";

// Project format 11 through the app (Phase 4.5): a real Compositor for Mac save at format 11 opens from
// File > Open and saves again at 11; a newer format says which versions this build reads. The engine's
// own tests (engine/tests/format_v11.rs) pin the text runs.
const SHAPES = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "engine", "tests", "fixtures", "mac-4b1-probes", "shapes.mac-1.3.7.comp");

/** Puts the Mac's saved package into the mock bridge at `at`, its manifest changed by `edit`. Its
 * QuickLook folder is left out, as the real bridge reads only manifest.json and images/. */
async function seed(page: Page, at: string, edit: (manifest: string) => string = (m) => m) {
  const manifest = edit(fs.readFileSync(path.join(SHAPES, "manifest.json"), "utf8"));
  const images = fs.readdirSync(path.join(SHAPES, "images")).map((name) => ({ name, bytes: Array.from(fs.readFileSync(path.join(SHAPES, "images", name))) }));
  await page.evaluate(async ({ at, manifest, images }) => {
    const api = (window as any).__compositor;
    await api.bridge.writePackage(at, { manifest, images: images.map((i: { name: string; bytes: number[] }) => ({ name: i.name, bytes: new Uint8Array(i.bytes) })) });
  }, { at, manifest, images });
}

test("a Mac save at format 11 opens from File > Open and saves again at 11, its shapes as the Mac wrote them", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await seed(page, "C:/mac/shapes.comp");
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/mac/shapes.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("layer-row")).toHaveCount(7);
  await expect(page.getByTestId("error-banner")).toHaveCount(0);
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/win/shapes.comp"));
  await clickMenu(page, "File", "save-as");
  const [before, after] = await page.evaluate(async () => {
    const bridge = (window as any).__compositor.bridge;
    return [JSON.parse((await bridge.readPackage("C:/mac/shapes.comp")).manifest), JSON.parse((await bridge.readPackage("C:/win/shapes.comp")).manifest)];
  });
  expect(before.version).toBe(11);
  expect(after.version).toBe(11);
  expect(after.layers.map((l: { shape?: unknown }) => l.shape ?? null)).toEqual(before.layers.map((l: { shape?: unknown }) => l.shape ?? null));
});

test("a project from a newer format says which versions this build reads", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await seed(page, "C:/mac/future.comp", (m) => m.replace('"version" : 11', '"version" : 12'));
  await page.evaluate(() => (window as any).__compositor.bridge.setNextPick("C:/mac/future.comp"));
  await clickMenu(page, "File", "open");
  await expect(page.getByTestId("error-banner")).toContainText("This project uses format version 12. This app supports versions 1-11, which Compositor for Mac saves up to version 1.4.5.");
  await expect(page.getByTestId("layer-row")).toHaveCount(0);
});
```

```diff
--- a/app/tests/e2e/mac-1.2.6.spec.ts
+++ b/app/tests/e2e/mac-1.2.6.spec.ts
@@ -291,8 +291,9 @@ test("the Mac-saved fixture opens, matches and re-saves to a re-openable v9 mani
   expect(result.before.layers[1].transform.size).toEqual([962, 1708]);
   await expect(page.getByTestId("undrawn-notice")).toHaveCount(0);
 
-  // The save round-trips to version 9 with every layer's id and transform unchanged.
-  expect(result.savedManifest.version).toBe(9);
+  // The save round-trips at version 11 (as Compositor for Mac 1.4.5 re-saves a 1.2.6 file) with every
+  // layer's id and transform unchanged.
+  expect(result.savedManifest.version).toBe(11);
   expect(result.savedManifest.layers.map((l: any) => l.id)).toEqual(inputManifest.layers.map((l) => l.id));
   inputManifest.layers.forEach((l, i) => {
     expect(result.savedManifest.layers[i].transform).toEqual(l.transform);
```

```diff
--- a/engine/tests/adjust_settings.rs
+++ b/engine/tests/adjust_settings.rs
@@ -180,7 +180,8 @@ fn manifests_accept_adjustment_layers_only_when_well_formed() {
     assert_eq!(parsed.layers[0].adjustment.as_ref().unwrap().kind, AdjustmentKind::Levels);
     let group = json.replacen("\"isVisible\": true", "\"isVisible\": true, \"isGroup\": true", 1);
     assert!(matches!(Manifest::parse(&group), Err(ProjectError::Invalid)), "a folder cannot carry an adjustment");
-    let old = json.replacen("\"version\": 9", "\"version\": 6", 1);
+    let old = json.replacen(&format!("\"version\": {CURRENT_VERSION}"), "\"version\": 6", 1);
+    assert_ne!(old, json, "the version was replaced");
     assert!(matches!(Manifest::parse(&old), Err(ProjectError::Invalid)), "adjustments need version 7");
     let with_image = json.replacen("\"adjustment\": {", "\"imageFile\": \"X.png\", \"adjustment\": {", 1);
     assert!(matches!(Manifest::parse(&with_image), Err(ProjectError::Invalid)));
```

Create `engine/tests/fixtures/hand-written-1.4.5/text-colour-only.manifest.json`:

```json
{
  "activeLayerID" : "0E5B4A39-2D1C-4B0A-8F7E-6D5C4B3A2917",
  "colorSpace" : "sRGB",
  "documentID" : "0E5B4A39-2D1C-4B0A-8F7E-6D5C4B3A2910",
  "format" : "com.compositor.project",
  "height" : 120,
  "layers" : [
    {
      "blendMode" : "Normal",
      "id" : "0E5B4A39-2D1C-4B0A-8F7E-6D5C4B3A2917",
      "imageFile" : "0E5B4A39-2D1C-4B0A-8F7E-6D5C4B3A2917.png",
      "isGroup" : false,
      "isVisible" : true,
      "name" : "Colour Only",
      "opacity" : 1,
      "text" : {
        "alignment" : "Left",
        "blue" : 0,
        "colorRuns" : [
          {
            "blue" : 1,
            "green" : 0,
            "length" : 4,
            "location" : 7,
            "red" : 0
          }
        ],
        "content" : "Colour Only",
        "fontName" : "Helvetica",
        "fontSize" : 48,
        "green" : 0,
        "leading" : 0,
        "red" : 0,
        "tracking" : 0
      },
      "transform" : {
        "flipX" : false,
        "flipY" : false,
        "origin" : [
          10,
          20
        ],
        "rotation" : 0,
        "sampling" : "High quality",
        "size" : [
          280,
          70
        ]
      }
    }
  ],
  "resolution" : 72,
  "version" : 11,
  "width" : 300
}
```

Create `engine/tests/fixtures/hand-written-1.4.5/text-runs.manifest.json`:

```json
{
  "activeLayerID" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B61",
  "colorSpace" : "sRGB",
  "documentID" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B60",
  "format" : "com.compositor.project",
  "height" : 200,
  "layers" : [
    {
      "blendMode" : "Normal",
      "id" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B62",
      "imageFile" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B62.png",
      "isGroup" : false,
      "isVisible" : true,
      "name" : "Background",
      "opacity" : 1,
      "transform" : {
        "flipX" : false,
        "flipY" : false,
        "origin" : [
          0,
          0
        ],
        "rotation" : 0,
        "sampling" : "High quality",
        "size" : [
          480,
          200
        ]
      }
    },
    {
      "blendMode" : "Normal",
      "id" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B61",
      "imageFile" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B61.png",
      "isGroup" : false,
      "isVisible" : true,
      "name" : "Hello World",
      "opacity" : 1,
      "text" : {
        "alignment" : "Left",
        "blue" : 0,
        "colorRuns" : [
          {
            "blue" : 0,
            "green" : 0,
            "length" : 5,
            "location" : 6,
            "red" : 1
          }
        ],
        "content" : "Hello World",
        "fontName" : "Helvetica",
        "fontRuns" : [
          {
            "fontName" : "Helvetica-Bold",
            "length" : 5,
            "location" : 0
          }
        ],
        "fontSize" : 72,
        "green" : 0,
        "leading" : 0,
        "red" : 0,
        "tracking" : 0
      },
      "transform" : {
        "flipX" : false,
        "flipY" : false,
        "origin" : [
          20,
          20
        ],
        "rotation" : 0,
        "sampling" : "High quality",
        "size" : [
          420,
          110
        ]
      }
    },
    {
      "blendMode" : "Normal",
      "id" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B63",
      "imageFile" : "6D0C1B7A-5E2F-4C3D-9A8B-1F2E3D4C5B63.png",
      "isGroup" : false,
      "isVisible" : true,
      "name" : "Plain",
      "opacity" : 1,
      "text" : {
        "alignment" : "Left",
        "blue" : 0,
        "content" : "Plain",
        "fontName" : "Helvetica",
        "fontSize" : 36,
        "green" : 0,
        "leading" : 0,
        "red" : 0,
        "tracking" : 0
      },
      "transform" : {
        "flipX" : false,
        "flipY" : false,
        "origin" : [
          20,
          140
        ],
        "rotation" : 0,
        "sampling" : "High quality",
        "size" : [
          120,
          50
        ]
      }
    }
  ],
  "resolution" : 72,
  "version" : 11,
  "width" : 480
}
```

Create `engine/tests/format_v11.rs`:

```rust
//! Project format 11 (Phase 4.5): Compositor for Mac 1.4.5 writes version 11 (ProjectStore.swift:15);
//! version 10 added a text's colour runs and 11 its font runs (TypeTool.swift:30-62, :190-202;
//! ProjectStore.swift:208-214). This build reads 1-11, keeps a text's runs verbatim (the layer's PNG
//! already shows them), refuses what the Mac refuses, and writes 11.
//!
//! The real Mac save here is `mac-4b1-probes/shapes.mac-1.3.7.comp` (format 11, no text). The two text
//! fixtures under `hand-written-1.4.5` are HAND-WRITTEN from the Mac's Codable encoding of
//! `LayerTextStyle` and its runs (TypeTool.swift:7-34, :190-202) and `ProjectLayerRecord`
//! (ProjectStore.swift:33-56), sorted and pretty printed as `JSONEncoder` writes them; no Mac saved
//! them (LL-069). Replace them with the user's A1 `text-runs.comp` and A2 `text-colour-only.comp` when
//! those come back.
use compositor_engine::*;
use serde_json::{json, Value};

fn fixtures() -> String { format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR")) }

/// A package folder's manifest and its `images/` (a Mac save's `QuickLook/` is never read, as the Mac
/// reads only these two, ProjectStore.swift:146-196).
fn package_dir(dir: &str) -> Package {
    let manifest_json = std::fs::read_to_string(format!("{dir}/manifest.json")).unwrap();
    let images = std::fs::read_dir(format!("{dir}/images")).unwrap().map(|entry| {
        let entry = entry.unwrap();
        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
    }).collect();
    Package { manifest_json, images }
}

/// A package for a hand-written manifest: each layer's image a plain PNG of its transform's size (the
/// rendered text's pixels do not matter here).
fn package_of(manifest: &Value) -> Package {
    let images = manifest["layers"].as_array().unwrap().iter().filter_map(|l| {
        let file = l["imageFile"].as_str()?;
        let (w, h) = (l["transform"]["size"][0].as_u64().unwrap() as u32, l["transform"]["size"][1].as_u64().unwrap() as u32);
        Some((file.to_string(), encode_png(&Raster::from_premultiplied(w, h, [20u8, 30, 40, 255].repeat((w * h) as usize)), 72.0).unwrap()))
    }).collect();
    Package { manifest_json: manifest.to_string(), images }
}
fn hand_written(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{}/hand-written-1.4.5/{name}.manifest.json", fixtures())).unwrap()).unwrap()
}
/// The manifest with its first text layer's `text` changed by `edit`, at `version`.
fn with_text(mut manifest: Value, version: u32, edit: impl Fn(&mut Value)) -> Package {
    manifest["version"] = json!(version);
    let layer = manifest["layers"].as_array_mut().unwrap().iter_mut().find(|l| l.get("text").is_some()).unwrap();
    edit(&mut layer["text"]);
    package_of(&manifest)
}
fn saved(doc: &Document) -> Value { serde_json::from_str(&save_package(doc).unwrap().manifest_json).unwrap() }
/// Each layer's `text`, by name.
fn texts(manifest: &Value) -> Vec<(String, Value)> {
    manifest["layers"].as_array().unwrap().iter().filter(|l| l.get("text").is_some()).map(|l| (l["name"].as_str().unwrap().to_string(), l["text"].clone())).collect()
}

#[test]
fn a_real_format_11_save_opens_and_saves_as_11_with_its_shape_records_kept() {
    // Compositor for Mac 1.3.7's save of shapes.comp (Phase 4b-1's probe), QuickLook folder and all.
    let dir = format!("{}/mac-4b1-probes/shapes.mac-1.3.7.comp", fixtures());
    assert!(std::path::Path::new(&format!("{dir}/QuickLook/Preview.jpg")).exists(), "the Mac's save carries its Quick Look preview");
    let package = package_dir(&dir);
    let original: Value = serde_json::from_str(&package.manifest_json).unwrap();
    assert_eq!(original["version"], 11);
    let doc = open_package(&package).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(doc.layers.len(), 7);
    let again = saved(&doc);
    assert_eq!(again["version"], json!(CURRENT_VERSION));
    assert_eq!(CURRENT_VERSION, 11);
    for (a, b) in original["layers"].as_array().unwrap().iter().zip(again["layers"].as_array().unwrap()) {
        assert_eq!(a.get("shape"), b.get("shape"), "{}: the shape record comes back as the Mac wrote it", a["name"]);
    }
}

#[test]
fn text_runs_open_and_come_back_exactly_as_they_were() {
    for name in ["text-runs", "text-colour-only"] {
        let manifest = hand_written(name);
        let doc = open_package(&package_of(&manifest)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let again = saved(&doc);
        assert_eq!(again["version"], json!(11), "{name}");
        assert_eq!(texts(&again), texts(&manifest), "{name}: every run, integer for integer");
        // Nothing to notice: the layer's PNG already shows the runs (docs/project-format.md at v1.4.5).
        assert!(doc.undrawn().is_empty(), "{name}: {:?}", doc.undrawn());
    }
}

#[test]
fn colour_runs_need_format_10_and_font_runs_format_11() {
    // ProjectStore.swift:211-212: a run older than its format makes the project invalid.
    let colour_only = hand_written("text-colour-only");
    assert!(open_package(&with_text(colour_only.clone(), 10, |_| {})).is_ok(), "colour runs at 10");
    assert_eq!(open_package(&with_text(colour_only.clone(), 9, |_| {})).unwrap_err(), ProjectError::Invalid, "colour runs at 9");
    let both = hand_written("text-runs");
    assert_eq!(open_package(&with_text(both.clone(), 10, |_| {})).unwrap_err(), ProjectError::Invalid, "font runs at 10");
    assert!(open_package(&with_text(both.clone(), 11, |_| {})).is_ok());
    // The same text without its runs opens at 9 (so the refusals above are the runs' gates).
    assert!(open_package(&with_text(both, 9, |t| { t.as_object_mut().unwrap().remove("colorRuns"); t.as_object_mut().unwrap().remove("fontRuns"); })).is_ok());
    assert!(open_package(&with_text(colour_only, 9, |t| { t["colorRuns"] = Value::Null; })).is_ok(), "null is none");
}

#[test]
fn runs_the_mac_refuses_are_refused() {
    // TypeTool.swift:43-62 (`colorRunsAreValid`, `fontRunsAreValid`) and the synthesized decoder.
    let colour = |location: Value, length: Value| json!({ "location": location, "length": length, "red": 1, "green": 0, "blue": 0 });
    let font = |name: &str| json!({ "location": 0, "length": 5, "fontName": name });
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("out of order", Box::new(move |t| t["colorRuns"] = json!([colour(json!(6), json!(2)), colour(json!(0), json!(2))]))),
        ("overlapping", Box::new(move |t| t["colorRuns"] = json!([colour(json!(0), json!(4)), colour(json!(3), json!(2))]))),
        ("an empty list", Box::new(|t| t["colorRuns"] = json!([]))),
        ("a zero length", Box::new(move |t| t["colorRuns"] = json!([colour(json!(2), json!(0))]))),
        ("a negative location", Box::new(move |t| t["colorRuns"] = json!([colour(json!(-1), json!(2))]))),
        ("past the end", Box::new(move |t| t["colorRuns"] = json!([colour(json!(6), json!(6))]))),
        ("a fractional location", Box::new(move |t| t["colorRuns"] = json!([colour(json!(1.5), json!(2))]))),
        ("a channel past 1", Box::new(|t| t["colorRuns"] = json!([{ "location": 0, "length": 1, "red": 1.5, "green": 0, "blue": 0 }]))),
        ("a missing channel", Box::new(|t| t["colorRuns"] = json!([{ "location": 0, "length": 1, "red": 1, "green": 0 }]))),
        ("a run that is not an object", Box::new(|t| t["colorRuns"] = json!([6]))),
        ("runs that are not a list", Box::new(|t| t["colorRuns"] = json!({ "location": 0 }))),
        ("an empty face", Box::new(move |t| t["fontRuns"] = json!([font("")]))),
        ("a line feed in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\nBold")]))),
        ("a carriage return in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\rBold")]))),
        ("a line separator in a face", Box::new(move |t| t["fontRuns"] = json!([font("Helvetica\u{2028}Bold")]))),
        ("a face of 201 characters", Box::new(move |t| t["fontRuns"] = json!([font(&"H".repeat(201))]))),
        ("a face that is not a string", Box::new(|t| t["fontRuns"] = json!([{ "location": 0, "length": 5, "fontName": 7 }]))),
    ];
    for (what, edit) in &cases {
        assert_eq!(open_package(&with_text(hand_written("text-runs"), 11, |t| edit(t))).unwrap_err(), ProjectError::Invalid, "{what}");
    }
    // And what the Mac takes: runs meeting end to end, a whole number written with a fraction (Swift's
    // `Int(exactly:)`), a key it does not know inside a run, a face of 200 characters.
    for (what, edit) in [
        ("runs meeting", Box::new(move |t: &mut Value| t["colorRuns"] = json!([colour(json!(0), json!(6)), colour(json!(6), json!(5))])) as Box<dyn Fn(&mut Value)>),
        ("6.0 for 6", Box::new(move |t: &mut Value| t["colorRuns"] = json!([colour(json!(6.0), json!(5))]))),
        ("an unknown key", Box::new(|t: &mut Value| t["fontRuns"] = json!([{ "location": 0, "length": 5, "fontName": "Times-Roman", "tracking": 3 }]))),
        ("200 characters", Box::new(move |t: &mut Value| t["fontRuns"] = json!([font(&"H".repeat(200))]))),
    ] {
        assert!(open_package(&with_text(hand_written("text-runs"), 11, |t| edit(t))).is_ok(), "{what}");
    }
}

#[test]
fn runs_are_measured_in_utf16_units_of_the_content() {
    // TypeTool.swift:30: offsets into `content.utf16`. A grinning face is two units, an accented e
    // written as e and a combining acute is two, so "e\u{301}\u{1F600}!" is 5 units long.
    let content = |t: &mut Value| t["content"] = json!("e\u{301}\u{1F600}!");
    let run = |location: i64, length: i64| json!([{ "location": location, "length": length, "red": 0, "green": 0, "blue": 1 }]);
    assert!(open_package(&with_text(hand_written("text-colour-only"), 11, |t| { content(t); t["colorRuns"] = run(2, 3); })).is_ok(), "the face and the ! end at 5");
    assert_eq!(open_package(&with_text(hand_written("text-colour-only"), 11, |t| { content(t); t["colorRuns"] = run(2, 4); })).unwrap_err(), ProjectError::Invalid, "6 is past the end");
}

#[test]
fn a_newer_format_is_refused_with_the_versions_this_build_reads() {
    let mut manifest = hand_written("text-runs");
    manifest["version"] = json!(12);
    let refused = open_package(&package_of(&manifest)).unwrap_err();
    assert_eq!(refused, ProjectError::Version(12));
    assert_eq!(refused.to_string(), "This project uses format version 12. This app supports versions 1-11, which Compositor for Mac saves up to version 1.4.5.");
}
```

```diff
--- a/engine/tests/format_v9.rs
+++ b/engine/tests/format_v9.rs
@@ -26,10 +26,11 @@ fn a_project_saved_by_the_mac_app_opens() {
 }
 
 #[test]
-fn saving_writes_version_9_like_the_mac() {
+fn saving_writes_version_11_like_the_mac_1_4_5() {
     let doc = open_package(&mac_fixture()).unwrap();
     let saved: Value = serde_json::from_str(&save_package(&doc).unwrap().manifest_json).unwrap();
-    assert_eq!(saved["version"], json!(9));
+    // Compositor for Mac 1.4.5 writes every save at 11, a version 9 project too (ProjectStore.swift:15, :21).
+    assert_eq!(saved["version"], json!(11));
     // Round-trip the saved file: it must re-open.
     let again = Package { manifest_json: saved.to_string(), images: save_package(&doc).unwrap().images };
     assert_eq!(open_package(&again).unwrap().layers.len(), 2);
```

```diff
--- a/engine/tests/interop.rs
+++ b/engine/tests/interop.rs
@@ -73,7 +73,7 @@ fn assert_keys_are_sorted(text: &str) {
 }
 
 #[test]
-fn every_fixture_version_opens_resaves_at_v9_and_preserves_later_phase_fields() {
+fn every_fixture_version_opens_resaves_at_v11_and_preserves_later_phase_fields() {
     for (name, json) in FIXTURES {
         let pkg = Package { manifest_json: json.to_string(), images: images_for(json) };
         let doc = open_package(&pkg).unwrap_or_else(|e| panic!("{name}: open_package failed: {e:?}"));
@@ -84,7 +84,7 @@ fn every_fixture_version_opens_resaves_at_v9_and_preserves_later_phase_fields()
 
         let before: serde_json::Value = serde_json::from_str(json).unwrap();
         let after: serde_json::Value = serde_json::from_str(&resaved.manifest_json).unwrap();
-        assert_eq!(after["version"], serde_json::json!(9), "{name}: re-saved manifest must be version 9");
+        assert_eq!(after["version"], serde_json::json!(11), "{name}: re-saved manifest must be version 11, as Compositor for Mac 1.4.5 writes");
 
         let before_layers = before["layers"].as_array().unwrap();
         let after_layers = after["layers"].as_array().unwrap();
```

```diff
--- a/engine/tests/mac_probes.rs
+++ b/engine/tests/mac_probes.rs
@@ -551,6 +551,12 @@ Made on the Mac from nothing in this folder (the first two) or from edited-rich-
 - resaved-edited-rich-file.comp -> resaved-edited-rich-file.png
   Open edited-rich-file.comp from this folder and change nothing. File > Save As,
   resaved-edited-rich-file.comp. Then export.
+
+Written by Compositor for Windows at project format 11 (0.6.0 and later):
+
+- port-v11-roundtrip.comp -> port-v11-roundtrip.png
+  It must open without an error. Export it, then File > Save As port-v11-roundtrip.resaved.comp
+  and send that back too.
 ";
 
 /// 7. RULING (F5, replacing the M9 tautological final-existence loop): the Mac acceptance probe.
@@ -882,6 +888,38 @@ fn phase_4_5_probes() -> Vec<(&'static str, Document)> {
     probes
 }
 
+/// A5 `port-v11-roundtrip.comp` (Phase 4.5 Task 5): what this build writes at format 11, for the Mac
+/// to open (without an error), export and save again: a text layer in a folder carrying both kinds of
+/// run (TypeTool.swift:190-202 at v1.4.5), its mask, a Levels adjustment layer and a guide. Opened
+/// here through `open_package`, as a Mac file would be, so the runs are the verbatim values the port
+/// keeps.
+fn port_v11_roundtrip_doc() -> Document {
+    let (folder, text, adj) = ("5A1B2C3D-4E5F-4A6B-8C7D-111111111111", "5A1B2C3D-4E5F-4A6B-8C7D-222222222222", "5A1B2C3D-4E5F-4A6B-8C7D-333333333333");
+    let transform = |x: i64, y: i64, w: i64, h: i64| serde_json::json!({ "origin": [x, y], "size": [w, h], "rotation": 0, "flipX": false, "flipY": false, "sampling": "High quality" });
+    let manifest = serde_json::json!({
+        "format": "com.compositor.project", "version": 11, "colorSpace": "sRGB", "resolution": 72,
+        "documentID": "5A1B2C3D-4E5F-4A6B-8C7D-000000000000", "width": 240, "height": 120, "activeLayerID": text,
+        "guides": [ { "axis": "horizontal", "id": "5A1B2C3D-4E5F-4A6B-8C7D-444444444444", "position": 60 } ],
+        "layers": [
+            { "id": folder, "name": "Folder", "isVisible": true, "isGroup": true, "opacity": 0.8, "transform": transform(0, 0, 240, 120) },
+            { "id": text, "name": "Hello World", "isVisible": true, "parentID": folder, "imageFile": format!("{text}.png"),
+              "maskFile": format!("{text}.mask.png"), "maskEnabled": true, "transform": transform(20, 30, 200, 60),
+              "text": { "alignment": "Left", "blue": 0, "content": "Hello World", "fontName": "Helvetica", "fontSize": 40, "green": 0,
+                        "leading": 0, "red": 0, "tracking": 0,
+                        "colorRuns": [ { "location": 6, "length": 5, "red": 1, "green": 0, "blue": 0 } ],
+                        "fontRuns": [ { "location": 0, "length": 5, "fontName": "Helvetica-Bold" } ] } },
+            { "id": adj, "name": "Levels", "isVisible": true, "transform": transform(0, 0, 240, 120),
+              "adjustment": serde_json::to_value(LayerAdjustment::new(AdjustmentKind::Levels)).unwrap() }
+        ]
+    });
+    let ramp: Vec<u8> = (0..60u32).flat_map(|y| (0..200u32).map(move |x| ((x + y) * 255 / 258) as u8)).collect();
+    let package = Package { manifest_json: manifest.to_string(), images: vec![
+        (format!("{text}.png"), encode_png(&colourful_gradient(200, 60), 72.0).unwrap()),
+        (format!("{text}.mask.png"), encode_gray_png(&GrayRaster::from_bytes(200, 60, ramp)).unwrap()),
+    ] };
+    open_package(&package).unwrap_or_else(|e| panic!("port-v11-roundtrip.comp: does not open: {e:?}"))
+}
+
 /// Saves `doc` as `<dir>/<filename>/manifest.json` plus its `images/`, then re-opens the saved
 /// package with `open_package` -- every probe must be openable by this build's own reader before
 /// it is ever sent to a Mac.
@@ -937,6 +975,7 @@ fn write_mac_probes() {
     for (name, doc) in step_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in phase_4b1_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in phase_4_5_probes() { write_probe(&dir, name, &doc); }
+    write_probe(&dir, "port-v11-roundtrip.comp", &port_v11_roundtrip_doc());
 
     fs::write(dir.join("README.txt"), README_TXT).unwrap_or_else(|e| panic!("failed to write README.txt: {e}"));
     assert!(README_TXT.is_ascii(), "README.txt must be ASCII only");
@@ -1090,6 +1129,16 @@ fn every_4_5_probe_is_listed_and_holds_what_it_is_named_for() {
     }
 }
 
+#[test]
+fn the_round_trip_probe_is_listed_and_written_at_format_11_with_its_runs() {
+    assert!(README_TXT.contains("- port-v11-roundtrip.comp"));
+    let saved: serde_json::Value = serde_json::from_str(&save_package(&port_v11_roundtrip_doc()).unwrap().manifest_json).unwrap();
+    assert_eq!(saved["version"], 11);
+    let text = saved["layers"].as_array().unwrap().iter().find(|l| l["name"] == "Hello World").unwrap();
+    assert_eq!((text["text"]["colorRuns"][0]["location"].as_i64(), text["text"]["fontRuns"][0]["fontName"].as_str()), (Some(6), Some("Helvetica-Bold")));
+    assert!(text["maskFile"].is_string() && text["parentID"].is_string());
+}
+
 #[test]
 fn the_readme_names_the_mac_version_the_probes_are_for() {
     assert!(README_TXT.contains("Compositor for Mac 1.4.5") && !README_TXT.contains("1.2.10 or later") && !README_TXT.contains("1.2.6"));
```

```diff
--- a/engine/tests/manifest.rs
+++ b/engine/tests/manifest.rs
@@ -15,7 +15,7 @@ fn round_trips_json_with_swift_shapes() {
     let m = manifest();
     let json = m.to_json_pretty().unwrap();
     assert!(json.contains("\"format\": \"com.compositor.project\""));
-    assert!(json.contains("\"version\": 9"));
+    assert!(json.contains("\"version\": 11"), "a new manifest is written at 1.4.5's format");
     assert!(json.contains("\"colorSpace\": \"sRGB\""));
     assert!(!json.contains("\"parentID\""), "absent optionals are omitted");
     let upper = ids::upper_string(&m.layers[0].id);
```

```diff
--- a/engine/tests/package.rs
+++ b/engine/tests/package.rs
@@ -48,7 +48,7 @@ fn corrupt_future_and_unsafe_manifests_are_rejected() {
     doc.layers.push(Layer::blank("Layer 1", doc.size()));
     let pkg = save_package(&doc).unwrap();
     let mut future = pkg.clone();
-    future.manifest_json = pkg.manifest_json.replace("\"version\": 9", "\"version\": 42");
+    future.manifest_json = pkg.manifest_json.replace(&format!("\"version\": {CURRENT_VERSION}"), "\"version\": 42");
     assert_eq!(open_package(&future).unwrap_err(), ProjectError::Version(42));
     let mut unsafe_pkg = pkg.clone();
     unsafe_pkg.manifest_json = pkg.manifest_json.replace("\"name\": \"Layer 1\"", "\"imageFile\": \"../../outside.png\", \"name\": \"Layer 1\"");
```


- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test format_v11`
Expected: all six fail: format 11 is refused ("This project uses format version 11. This app supports versions 1-9"). `format_v9`, `interop`, `manifest`, `adjust_settings` and `package` fail where they now expect 11; `mac_probes`' round-trip test fails (the probe saves 9); the two e2e files fail on the version.

- [ ] **Step 3: Implement**

```diff
--- a/engine/src/error.rs
+++ b/engine/src/error.rs
@@ -4,7 +4,10 @@ use thiserror::Error;
 pub enum ProjectError {
     #[error("This is not a valid Compositor project, or its metadata is damaged.")]
     Invalid,
-    #[error("This project uses format version {0}. This app supports versions 1-9.")]
+    /// The Mac's words (ProjectStore.swift:69 at v1.4.5), the upper bound read from the version this
+    /// build writes so the two cannot drift apart, and which Mac saves them: a project from a newer
+    /// Mac needs a newer build of this app.
+    #[error("This project uses format version {0}. This app supports versions 1-{max}, which Compositor for Mac saves up to version 1.4.5.", max = crate::manifest::CURRENT_VERSION)]
     Version(u32),
     #[error("An image inside the project is missing or damaged. The current document has not been replaced.")]
     MissingImage,
```

```diff
--- a/engine/src/manifest.rs
+++ b/engine/src/manifest.rs
@@ -5,7 +5,10 @@ use std::collections::{HashMap, HashSet};
 use uuid::Uuid;
 
 pub const MANIFEST_FORMAT: &str = "com.compositor.project";
-pub const CURRENT_VERSION: u32 = 9;
+/// The format version this build writes and the newest it reads: Compositor for Mac 1.4.5's
+/// (`ProjectManifest.current`, ProjectStore.swift:15). Version 10 added text colour runs and 11 text
+/// font runs (ProjectStore.swift:209-213); every save writes 11, as the Mac's does.
+pub const CURRENT_VERSION: u32 = 11;
 pub const MAX_GUIDES: usize = 1_000;
 pub const MAX_SIDE: i64 = 30_000;
 pub const MAX_PIXELS: u64 = 100_000_000;
@@ -254,9 +257,10 @@ impl Manifest {
             // own arrived in v8 (ProjectStore.swift:212-216).
             if layer.is_group() && (blend != BlendMode::Normal || (self.version < 8 && opacity != 1.0)) { return Err(Invalid); }
             // Live text is a valid LayerTextStyle on a pixel layer that is not a folder or an
-            // adjustment (ProjectStore.swift:196-198).
+            // adjustment, its colour runs from version 10 and its font runs from 11
+            // (ProjectStore.swift:208-214 at v1.4.5).
             if let Some(text) = &layer.text {
-                if !text_is_valid(text) || layer.image_file.is_none() || layer.is_group() || layer.adjustment.is_some() { return Err(Invalid); }
+                if !text_is_valid(text, self.version) || layer.image_file.is_none() || layer.is_group() || layer.adjustment.is_some() { return Err(Invalid); }
             }
         }
         validate_hierarchy(&self.layers)?;
@@ -286,13 +290,14 @@ impl Manifest {
     }
 }
 
-/// `LayerTextStyle.isValid` (TypeTool.swift:25-36), read from the verbatim value. Swift's
-/// synthesized decode requires every key but `boxSize`, so a missing required key or a wrong type
-/// makes the text invalid, as it makes the Mac refuse the project. `boxSize` is a CGSize, which
-/// Swift encodes as `[width, height]`. Its area follows 1.2.10's `boxIsValid` (200,000,000,
-/// TypeTool.swift:28), not this build's 100 MP image budget: it validates a value the port never
-/// renders, and refusing it would stop a 1.2.10 file opening.
-fn text_is_valid(text: &Value) -> bool {
+/// `LayerTextStyle.isValid` (TypeTool.swift:25-62 at v1.4.5), read from the verbatim value, with the
+/// version gates on its runs (ProjectStore.swift:211-212). Swift's synthesized decode requires every
+/// key but `boxSize`, `colorRuns` and `fontRuns`, so a missing required key or a wrong type makes the
+/// text invalid, as it makes the Mac refuse the project. `boxSize` is a CGSize, which Swift encodes as
+/// `[width, height]`. Its area follows 1.2.10's `boxIsValid` (200,000,000, TypeTool.swift:28), not this
+/// build's 100 MP image budget: it validates a value the port never renders, and refusing it would stop
+/// a Mac file opening.
+fn text_is_valid(text: &Value, version: u32) -> bool {
     let number = |key: &str, range: std::ops::RangeInclusive<f64>| {
         text.get(key).and_then(Value::as_f64).is_some_and(|v| v.is_finite() && range.contains(&v))
     };
@@ -306,7 +311,8 @@ fn text_is_valid(text: &Value) -> bool {
             _ => false,
         },
     };
-    content.chars().map(char::len_utf16).sum::<usize>() <= 100_000
+    let units = content.chars().map(char::len_utf16).sum::<usize>();
+    units <= 100_000
         && text.get("fontName").is_some_and(Value::is_string)
         && matches!(text.get("alignment").and_then(Value::as_str), Some("Left" | "Center" | "Right"))
         && number("fontSize", 1.0..=2000.0)
@@ -314,6 +320,52 @@ fn text_is_valid(text: &Value) -> bool {
         && number("tracking", -100.0..=1000.0)
         && number("leading", 0.0..=5000.0)
         && box_ok
+        && runs_are_valid(text.get("colorRuns"), units, 10, version, colour_run_is_valid)
+        && runs_are_valid(text.get("fontRuns"), units, 11, version, font_run_is_valid)
+}
+
+/// A Swift `Int` as its synthesized decoder reads one: a JSON integer, or a number with no fraction
+/// that fits (`Int(exactly:)`).
+fn swift_int(value: Option<&Value>) -> Option<i64> {
+    let v = value?;
+    v.as_i64().or_else(|| v.as_f64().filter(|f| f.fract() == 0.0 && f.abs() < 9.2e18).map(|f| f as i64))
+}
+
+/// A text's colour or font runs (`colorRunsAreValid` / `fontRunsAreValid`, TypeTool.swift:43-62):
+/// absent or null, or a non-empty list of objects, each with its `location` and `length` as integers,
+/// sorted, not overlapping (a run starts at or after the last one's end), each at least one UTF-16 unit
+/// long, not overflowing, and ending within the text's `units`; `own` checks a run's other keys. A
+/// present list needs format version `since` (ProjectStore.swift:211-212).
+fn runs_are_valid(runs: Option<&Value>, units: usize, since: u32, version: u32, own: fn(&Map<String, Value>) -> bool) -> bool {
+    let list = match runs {
+        None | Some(Value::Null) => return true,
+        Some(Value::Array(list)) => list,
+        Some(_) => return false,
+    };
+    if version < since || list.is_empty() { return false; }
+    let mut end = 0i64;
+    for run in list {
+        let Some(run) = run.as_object() else { return false };
+        let (Some(location), Some(length)) = (swift_int(run.get("location")), swift_int(run.get("length"))) else { return false };
+        if location < end || length <= 0 || location > i64::MAX - length || !own(run) { return false; }
+        end = location + length;
+    }
+    end <= units as i64
+}
+
+/// A colour run's channels: numbers, finite, 0 to 1 (TypeTool.swift:48).
+fn colour_run_is_valid(run: &Map<String, Value>) -> bool {
+    ["red", "green", "blue"].into_iter().all(|k| run.get(k).and_then(Value::as_f64).is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v)))
+}
+
+/// A font run's face: a string, not empty, at most 200 characters and no newline (TypeTool.swift:58).
+/// Swift counts `Character`s (grapheme clusters); this counts Unicode scalars, which is the same for a
+/// PostScript name and stricter only for a name of over 200 scalars that forms at most 200 clusters (a
+/// recorded deviation, ruling OQ5). `Character.isNewline` is true for these scalars.
+fn font_run_is_valid(run: &Map<String, Value>) -> bool {
+    let Some(name) = run.get("fontName").and_then(Value::as_str) else { return false };
+    !name.is_empty() && name.chars().count() <= 200
+        && !name.chars().any(|c| matches!(c, '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}'))
 }
 
 pub fn validate_hierarchy(layers: &[LayerRecord]) -> Result<(), ProjectError> {
```

```diff
--- a/src-tauri/src/atomic.rs
+++ b/src-tauri/src/atomic.rs
@@ -72,6 +72,24 @@ mod tests {
         assert_eq!(leftovers, vec!["A.comp".to_string()]);
     }
 
+    #[test]
+    fn saving_over_a_mac_save_drops_its_quick_look_preview() {
+        // Compositor for Mac asks other writers to delete QuickLook/ when they change a project, and
+        // writes it again on its next save (docs/writing-comp-files.md:86 at v1.4.5): a save here
+        // stages manifest.json and images/ only, so the stale preview goes with the old package.
+        let root = temp();
+        let final_path = root.join("Mac.comp");
+        fs::create_dir_all(final_path.join("QuickLook")).unwrap();
+        fs::write(final_path.join("QuickLook").join("Preview.jpg"), b"jpeg").unwrap();
+        fs::write(final_path.join("manifest.json"), b"old").unwrap();
+        let stage = stage_dir(&final_path).unwrap();
+        fs::create_dir_all(stage.join("images")).unwrap();
+        fs::write(stage.join("manifest.json"), b"new").unwrap();
+        commit(&stage, &final_path).unwrap();
+        assert_eq!(fs::read(final_path.join("manifest.json")).unwrap(), b"new");
+        assert!(!final_path.join("QuickLook").exists(), "no out-of-date preview is left behind");
+    }
+
     #[test]
     fn commit_into_a_file_path_fails_and_keeps_the_original() {
         let root = temp();
```

```diff
--- a/src-tauri/src/commands/package.rs
+++ b/src-tauri/src/commands/package.rs
@@ -157,6 +157,19 @@ mod tests {
     // check with no Windows-specific behavior, and is covered by code review; running it here
     // would require CI to grant that privilege, which is out of scope for this pass.
 
+    #[test]
+    fn a_mac_save_opens_with_its_images_and_its_quick_look_preview_left_alone() {
+        // Compositor for Mac 1.3.7's save (format 11) carries QuickLook/Preview.jpg, which the Mac's
+        // own loading ignores (ProjectStore.swift:116-121, :146-196 at v1.4.5): only images/ is read.
+        let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../engine/tests/fixtures/mac-4b1-probes/shapes.mac-1.3.7.comp"));
+        assert!(root.join("QuickLook").join("Preview.jpg").is_file());
+        let header = read_package_manifest(root.to_string_lossy().to_string()).unwrap();
+        assert!(header.manifest.contains("\"version\" : 11"));
+        assert_eq!(header.image_names.len(), 7, "the seven layers' images, and nothing else: {:?}", header.image_names);
+        assert!(header.image_names.iter().all(|n| valid_image_name(n)));
+        assert!(read_package_image(root.to_string_lossy().to_string(), "Preview.jpg".into()).is_err(), "the preview is not an image of the project");
+    }
+
     #[test]
     fn image_names_are_uuid_pngs_only() {
         assert!(valid_image_name("E621E1F8-C36C-495A-93FC-0C247A3E6E5F.png"));
```


- [ ] **Step 4: Run the whole set**

`cargo test -p compositor-engine`: 565 passed, 10 ignored (71 binaries). `cargo test -p compositor-shell`: 9 passed (2 new). `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 163 passed, 19 skipped. Regenerate the probes (Task 2 Step 3) so A5 `port-v11-roundtrip.comp` is in the folder the user gets. Opening a project is not an interactive path with a budget (ruling OQ17).

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Drop the version gate (accept `fontRuns` at 10): `colour_runs_need_format_10_and_font_runs_format_11` fails (measured).
2. Count the text in `chars()` instead of UTF-16 units: `runs_are_measured_in_utf16_units_of_the_content` fails (measured).
3. Leave U+2028 out of the newline set: `runs_the_mac_refuses_are_refused` fails (measured).
4. Put back the old wording ("supports versions 1-9"): `a_newer_format_is_refused_with_the_versions_this_build_reads` fails (measured).

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/format-v11.spec.ts engine/tests/fixtures/hand-written-1.4.5/text-colour-only.manifest.json engine/tests/fixtures/hand-written-1.4.5/text-runs.manifest.json engine/tests/format_v11.rs
git commit -m "feat(engine): open and save project format 11: text colour and font runs kept verbatim, refused as the Mac refuses them" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/tests/e2e/adjust-layers.spec.ts app/tests/e2e/format-v11.spec.ts app/tests/e2e/mac-1.2.6.spec.ts engine/src/error.rs engine/src/manifest.rs engine/tests/adjust_settings.rs engine/tests/fixtures/hand-written-1.4.5/text-colour-only.manifest.json engine/tests/fixtures/hand-written-1.4.5/text-runs.manifest.json engine/tests/format_v11.rs engine/tests/format_v9.rs engine/tests/interop.rs engine/tests/mac_probes.rs engine/tests/manifest.rs engine/tests/package.rs src-tauri/src/atomic.rs src-tauri/src/commands/package.rs
```


---

### Task 6: Soft Light by Core Image's W3C formula

Compositor 1.2.10 drew Soft Light with Core Graphics (Pegtop's formula), and Phase 3.5 matched it. 1.4.5 draws every mode through Core Image, whose Soft Light is the W3C / PDF formula: `cb - (1 - 2cs) cb (1 - cb)` for `cs <= 0.5`, else `cb + (2cs - 1)(D(cb) - cb)` with `D(cb) = ((16cb - 12)cb + 4)cb` for `cb <= 0.25` and `sqrt(cb)` above (ruling OQ6). The two agree when the source is at most half grey, so of the pinned 1.2.10 probes only `blend-greys`' 75% columns move.

**Files:**
- Modify: `engine/src/blend.rs` (`soft_light`), `app/src/canvas/gl/programs.ts` (mode 15)
- Modify (tests): `engine/tests/blend_modes_v9.rs`, `engine/tests/mac_1_2_10.rs` (`every_band_of_blend_greys_matches_the_mac_render`, `open`), `app/tests/e2e/mac-1.2.10.spec.ts` (the blend-greys test)

**Mac:** 1.4.5 blends through Core Image (LiveMaskRenderer.swift:52-57); the formula is the W3C compositing spec's, which CISoftLightBlendMode implements.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/mac-1.2.10.spec.ts
+++ b/app/tests/e2e/mac-1.2.10.spec.ts
@@ -207,12 +207,26 @@ async function onWholePixels(page: Page): Promise<boolean> {
   });
 }
 
-test("the blend-greys probe draws on the GPU as the Mac exported it, Soft Light included", async ({ page }) => {
+test("the blend-greys probe draws on the GPU as the Mac exported it, and Soft Light at 75% grey as the CPU draws it", async ({ page }) => {
+  // Two whole-document comparisons in the page: 13 s alone on the HD 520, and over the default 30 s once in a
+  // loaded whole-suite run (p45-scratch, 2026-09-30).
+  test.setTimeout(90_000);
   await openProbe(page, "blend-greys");
   expect(await onWholePixels(page), "the document sits on whole device pixels").toBe(true);
-  // The CPU is within 1 of the Mac (mac_1_2_10.rs); W3C's Soft Light was 14 off at the 75% grey.
-  // Measured 1 on p4a-scratch (2026-09-27).
-  expect(worstOf(await glPixels(page), await macPixels(page, "blend-greys"))).toBeLessThanOrEqual(1);
+  // The CPU is within 1 of the Mac 1.2.10 export (mac_1_2_10.rs) everywhere but Soft Light's 75% grey
+  // columns (band 0, x 80-120 and 200-240): Compositor 1.4.5 draws Soft Light by the W3C formula,
+  // 1.2.10 by Pegtop's, and the two differ only above half grey. There the GPU is held to the CPU,
+  // which mac_1_2_10.rs holds to the formula, until the 1.4.5 re-export (B1). Measured 1 on p4a-scratch
+  // (2026-09-27) and p45-scratch (2026-09-30).
+  const gl = await glPixels(page), mac = await macPixels(page, "blend-greys");
+  const cpu = await page.evaluate(() => {
+    const api = (window as any).__compositor; const s = api.store.getState(); const d = s.documents[s.activeId];
+    return Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
+  });
+  const brightSoftLight = (i: number) => { const p = i >> 2, x = p % 240, y = Math.floor(p / 240); return y < 60 && ((x >= 80 && x < 120) || (x >= 200 && x < 240)); };
+  expect(gl.length).toBe(mac.length);
+  expect(gl.reduce((m, v, i) => (brightSoftLight(i) ? m : Math.max(m, Math.abs(v - mac[i]))), 0), "as the Mac 1.2.10 exported it").toBeLessThanOrEqual(1);
+  expect(gl.reduce((m, v, i) => (brightSoftLight(i) ? Math.max(m, Math.abs(v - cpu[i])) : m), 0), "Soft Light at 75% grey, as the CPU").toBeLessThanOrEqual(1);
 });
 
 test("the motion-blur probe draws on the GPU as the Mac exported it", async ({ page }) => {
```

```diff
--- a/engine/tests/blend_modes_v9.rs
+++ b/engine/tests/blend_modes_v9.rs
@@ -47,8 +47,9 @@ fn expected_blend(mode: &str, cb: f32, cs: f32) -> f32 {
     match mode {
         "Linear Burn" => (cb + cs - 1.0).max(0.0),
         "Linear Dodge (Add)" => (cb + cs).min(1.0),
-        // Pegtop's, which the blend-greys probe fits within 1 level (probe results).
-        "Soft Light" => (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb,
+        // Core Image's, the W3C / PDF formula, as Compositor 1.4.5 draws it (LayerAppearance.swift:51-58);
+        // 1.2.10 drew Pegtop's through Core Graphics.
+        "Soft Light" => soft_light_w3c(cb, cs),
         "Hard Light" => if cs <= 0.5 { cb * 2.0 * cs } else { let s = 2.0 * cs - 1.0; cb + s - cb * s },
         "Vivid Light" => if cs <= 0.5 { burn(cb, 2.0 * cs) } else { dodge(cb, 2.0 * cs - 1.0) },
         "Linear Light" => (cb + 2.0 * cs - 1.0).clamp(0.0, 1.0),
@@ -62,6 +63,49 @@ fn expected_blend(mode: &str, cb: f32, cs: f32) -> f32 {
     }
 }
 
+/// The W3C / PDF Soft Light, written out from the spec: `D(cb)` is the square root but for a dark
+/// backdrop.
+fn soft_light_w3c(cb: f32, cs: f32) -> f32 {
+    if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) }
+    else { cb + (2.0 * cs - 1.0) * ((if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() }) - cb) }
+}
+
+/// One opaque `source` grey in `mode` over an opaque `backdrop` grey: the composite's red.
+fn grey_over_grey(mode: BlendMode, backdrop: u8, source: u8) -> u8 {
+    let mut doc = Document::new(1, 1);
+    let under = Layer::with_pixels("B", Raster::from_premultiplied(1, 1, vec![backdrop, backdrop, backdrop, 255]), Point { x: 0.0, y: 0.0 });
+    let mut over = Layer::with_pixels("S", Raster::from_premultiplied(1, 1, vec![source, source, source, 255]), Point { x: 0.0, y: 0.0 });
+    over.blend_mode = mode;
+    doc.layers = vec![under, over];
+    composite(&doc, Rect { x: 0.0, y: 0.0, width: 1.0, height: 1.0 }, 1, 1).pixel(0, 0)[0]
+}
+
+#[test]
+fn soft_light_is_the_w3c_formula_compositor_1_4_5_draws() {
+    // GPUCanvasTests.softLightMatchesPhotoshop (GPUCanvasTests.swift:522-545 at v1.4.5): a 0.5 backdrop
+    // under 0.9 exports 170, within 2. Pegtop's formula, 1.2.10's, gives 178 there.
+    let (cb, cs) = (128u8, 230u8);
+    let want = (soft_light_w3c(cb as f32 / 255.0, cs as f32 / 255.0) * 255.0).round() as u8;
+    assert_eq!(grey_over_grey(BlendMode::SoftLight, cb, cs), want);
+    assert!(want.abs_diff(170) <= 2, "the Mac's own bound: {want}");
+    let pegtop = |b: f32, s: f32| (1.0 - 2.0 * s) * b * b + 2.0 * s * b;
+    assert!(((pegtop(cb as f32 / 255.0, cs as f32 / 255.0) * 255.0).round() as u8).abs_diff(want) >= 6, "the fixture tells the two formulas apart");
+    // Over a dark backdrop (cb <= 0.25) under light sources, W3C's D(cb) is not the square root
+    // Photoshop uses: the soft-light-dark probe's case. Every pair is the formula, and some pair tells
+    // it from the square root.
+    let mut discriminates = false;
+    for backdrop in [0u8, 16, 40, 64] {
+        for source in [128u8, 192, 255] {
+            let (b, s) = (backdrop as f32 / 255.0, source as f32 / 255.0);
+            let want = (soft_light_w3c(b, s) * 255.0).round() as u8;
+            assert!(grey_over_grey(BlendMode::SoftLight, backdrop, source).abs_diff(want) <= 1, "{backdrop} under {source}");
+            let root = ((b + (2.0 * s - 1.0) * (b.sqrt() - b)) * 255.0).round() as u8;
+            discriminates |= root.abs_diff(want) >= 4;
+        }
+    }
+    assert!(discriminates, "the dark backdrops tell W3C's D(cb) from a square root");
+}
+
 #[test]
 fn each_new_mode_composites_by_its_formula_over_an_opaque_backdrop() {
     // An asymmetric backdrop and three translucent greys at alpha 153 (0.6), premultiplied so that
```

```diff
--- a/engine/tests/mac_1_2_10.rs
+++ b/engine/tests/mac_1_2_10.rs
@@ -177,20 +177,57 @@ fn the_gaussian_blur_probes_match_the_mac_render_premultiplied() {
 #[test]
 fn every_band_of_blend_greys_matches_the_mac_render() {
     // Six 60-row bands over a hue sweep, one per mode, each six 40-px grey columns: 25%, 50% and
-    // 75% grey, opaque then at half alpha. Soft Light is Pegtop's formula: measured 1 there and in
-    // Hard Light, 0 in the other four. The W3C Soft Light this port drew before was 14 levels off at
-    // the 75% grey (probe results).
+    // 75% grey, opaque then at half alpha. Measured 1 in Hard Light, 0 in the other four; Soft Light
+    // below.
     let (width, theirs) = mac("blend-greys");
     let port = ours("blend-greys");
-    for (band, mode, measured) in [(0u32, "Soft Light", 1u8), (1, "Hard Light", 1), (2, "Linear Light", 0), (3, "Pin Light", 0), (4, "Vivid Light", 0), (5, "Hard Mix", 0)] {
-        let rows = band * 60..band * 60 + 60;
+    let worst_in = |rows: std::ops::Range<u32>, columns: &dyn Fn(u32) -> bool| {
         let mut d = 0u8;
-        for y in rows { for x in 0..width { for c in 0..4 {
+        for y in rows { for x in (0..width).filter(|x| columns(*x)) { for c in 0..4 {
             let i = ((y * width + x) * 4 + c) as usize;
             d = d.max(port[i].abs_diff(theirs[i]));
         }}}
+        d
+    };
+    for (band, mode, measured) in [(1u32, "Hard Light", 1u8), (2, "Linear Light", 0), (3, "Pin Light", 0), (4, "Vivid Light", 0), (5, "Hard Mix", 0)] {
+        let d = worst_in(band * 60..band * 60 + 60, &|_| true);
         assert!(d <= measured, "{mode}: {d}, measured {measured}");
     }
+    // Soft Light (band 0) is Core Image's W3C formula in Compositor 1.4.5 (blend.rs `soft_light`), where
+    // 1.2.10 drew Pegtop's. The two agree for a source at or under half grey, so the 25% and 50%
+    // columns still match this 1.2.10 export (measured 1); the 75% ones (x 80-120 and 200-240) are
+    // checked against the formula below, and differ from this export, until its 1.4.5 re-export (B1).
+    let bright = |x: u32| (80..120).contains(&x) || (200..240).contains(&x);
+    assert!(worst_in(0..60, &|x| !bright(x)) <= 1, "Soft Light at 25% and 50% grey");
+    assert!(worst_in(0..60, &bright) >= 4, "1.2.10's Pegtop at 75% grey is not what 1.4.5 draws");
+    let doc = open(&format!("{}/blend-greys.comp", fixtures()));
+    let sweep = doc.layers[0].pixels.as_ref().unwrap();
+    let w3c = |cb: f64, cs: f64| if cs <= 0.5 { cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb) } else { cb + (2.0 * cs - 1.0) * ((if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() }) - cb) };
+    let mut worst = 0u8;
+    for column in [2u32, 5] {
+        // The column's grey as the compositor reads it: premultiplied over its alpha.
+        let source = doc.layers[1 + column as usize].pixels.as_ref().unwrap().pixel(0, 0);
+        let (cs, a) = (source[0] as f64 / source[3] as f64, source[3] as f64 / 255.0);
+        for y in 0..60 { for x in column * 40..column * 40 + 40 {
+            let under = sweep.pixel(x, y);
+            for c in 0..3 {
+                let cb = under[c] as f64 / 255.0;
+                let want = (((1.0 - a) * cb + a * w3c(cb, cs)) * 255.0).round() as u8;
+                worst = worst.max(port[((y * width + x) * 4 + c as u32) as usize].abs_diff(want));
+            }
+        }}
+    }
+    assert!(worst <= 1, "Soft Light at 75% grey is the W3C formula: {worst}");
+}
+
+/// A probe project opened as the Mac would.
+fn open(comp: &str) -> Document {
+    let manifest_json = std::fs::read_to_string(format!("{comp}/manifest.json")).unwrap();
+    let images = std::fs::read_dir(format!("{comp}/images")).unwrap().map(|entry| {
+        let entry = entry.unwrap();
+        (entry.file_name().into_string().unwrap(), std::fs::read(entry.path()).unwrap())
+    }).collect();
+    open_package(&Package { manifest_json, images }).unwrap()
 }
 
 #[test]
```


The expected values are the formula computed in the test (`soft_light_w3c`), and for `blend-greys` the formula applied to the probe's own sweep and greys, opened from its `.comp`. The 75% columns must also differ from the 1.2.10 export by at least 4, so the pin cannot pass by matching the old Mac; they are marked awaiting the B1 re-export.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test blend_modes_v9 --test mac_1_2_10`
Expected: `soft_light_is_the_w3c_formula_compositor_1_4_5_draws` fails (128 under 230 gives 179, Pegtop, where the formula gives 170); `every_band_of_blend_greys_matches_the_mac_render` fails on "Soft Light at 75% grey is the W3C formula".

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/gl/programs.ts
+++ b/app/src/canvas/gl/programs.ts
@@ -50,7 +50,11 @@ float sep(int mode, float cb, float cs) {
   if (mode == 8) return cb >= 1.0 ? 1.0 : (cs <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / cs));
   if (mode == 13) return max(0.0, cb + cs - 1.0);
   if (mode == 14) return min(1.0, cb + cs);
-  if (mode == 15) return (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb;   // Pegtop, as blend.rs soft_light
+  if (mode == 15) {   // W3C / Core Image, as blend.rs soft_light
+    if (cs <= 0.5) return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb);
+    float d = cb <= 0.25 ? ((16.0 * cb - 12.0) * cb + 4.0) * cb : sqrt(cb);
+    return cb + (2.0 * cs - 1.0) * (d - cb);
+  }
   if (mode == 16) { if (cs <= 0.5) return cb * 2.0 * cs; float s = 2.0 * cs - 1.0; return cb + s - cb * s; }
   if (mode == 17) {
     if (cs <= 0.5) { float s = 2.0 * cs; return cb >= 1.0 ? 1.0 : (s <= 0.0 ? 0.0 : 1.0 - min(1.0, (1.0 - cb) / s)); }
```

```diff
--- a/engine/src/blend.rs
+++ b/engine/src/blend.rs
@@ -10,10 +10,17 @@ pub const HARD_MIX_MARGIN: f32 = 0.5 / 255.0;
 fn color_dodge(cb: f32, cs: f32) -> f32 { if cb <= 0.0 { 0.0 } else if cs >= 1.0 { 1.0 } else { (cb / (1.0 - cs)).min(1.0) } }
 fn color_burn(cb: f32, cs: f32) -> f32 { if cb >= 1.0 { 1.0 } else if cs <= 0.0 { 0.0 } else { 1.0 - ((1.0 - cb) / cs).min(1.0) } }
 
-/// Pegtop's Soft Light, `(1 - 2 cs) cb^2 + 2 cs cb`: what Compositor for Mac 1.2.10 draws. The
-/// blend-greys probe fits it within 1 level at every grey and alpha, where the W3C / PDF formula is
-/// 14 levels off at a 75% grey source (probe results, "Phase 3.5b follow-up probes").
-fn soft_light(cb: f32, cs: f32) -> f32 { (1.0 - 2.0 * cs) * cb * cb + 2.0 * cs * cb }
+/// Soft Light as Compositor for Mac 1.4.5 draws it everywhere: Core Image's `CISoftLightBlendMode`
+/// (LayerAppearance.swift:51-58, commit 5a8f6ce), the W3C / PDF formula. A source under half grey
+/// darkens the backdrop by `(1 - 2 cs) cb (1 - cb)`; one over it lightens the backdrop toward `D(cb)`,
+/// which is `sqrt(cb)` except over a dark backdrop (`cb <= 0.25`), where it is
+/// `((16 cb - 12) cb + 4) cb`. The Mac's own test: a 0.5 backdrop under 0.9 exports 170
+/// (GPUCanvasTests.swift:522-545). Compositor 1.2.10 drew Pegtop's formula through Core Graphics.
+fn soft_light(cb: f32, cs: f32) -> f32 {
+    if cs <= 0.5 { return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb); }
+    let d = if cb <= 0.25 { ((16.0 * cb - 12.0) * cb + 4.0) * cb } else { cb.sqrt() };
+    cb + (2.0 * cs - 1.0) * (d - cb)
+}
 
 /// PDF separable blend function B(cb, cs) on straight (unpremultiplied) channel values.
 pub fn separable(mode: BlendMode, cb: f32, cs: f32) -> f32 {
```


- [ ] **Step 4: Run the whole set**

`cargo test -p compositor-engine`: 566 passed, 10 ignored. `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 163 passed, 19 skipped (the blend-greys e2e now holds the bright Soft Light columns to the CPU within 1 and the rest to the Mac within 1). A blend formula is not a path of its own: its frame cost is timed with the other result changes in Task 8's perf case.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `D(cb) = sqrt(cb)` for every backdrop: `soft_light_is_the_w3c_formula...` fails on the dark backdrops (measured).
2. Put Pegtop back: that test (179, not 170), `each_new_mode...` and the blend-greys 75% check fail (measured).
3. GLSL mode 15 with the square root only: the e2e "eleven modes" test fails on Soft Light, 7 levels (measured).

- [ ] **Step 6: Commit**

```
git commit -m "fix: Soft Light is Core Image's W3C formula, as Compositor 1.4.5 draws it" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/gl/programs.ts app/tests/e2e/mac-1.2.10.spec.ts engine/src/blend.rs engine/tests/blend_modes_v9.rs engine/tests/mac_1_2_10.rs
```


---

### Task 7: Adjustment layers and clipping-stack bases blend in their own modes

Compositor 1.2.10 drew an adjustment layer, and a clipping stack's base, through Core Graphics, which does not have the eight modes only Core Image computes (Linear Burn, Linear Dodge, Vivid Light, Linear Light, Pin Light, Hard Mix, Subtract, Divide), so it drew them as Normal; the port copied that as `cg_mode`. 1.4.5 blends both through Core Image in their real modes (LiveMaskRenderer.swift:52-57, :126-134). `cg_mode` goes (ruling OQ7).

**Files:**
- Modify: `engine/src/plan.rs` (an adjustment node's `blend` is its layer's mode; a stack's base is drawn with `draw_for` in its own mode), `engine/src/manifest.rs` (`cg_mode` deleted), `engine/src/compositor.rs` and `app/src/canvas/gl/programs.ts` (comments that described the mapping)
- Create: `app/tests/e2e/modes-1-4-5.spec.ts`
- Modify (tests): `engine/tests/blend_modes_v9.rs`, `engine/tests/mac_1_2_10.rs`, `engine/tests/spatial_adjustments.rs`, `app/tests/e2e/mac-1.2.10.spec.ts` (a comment)

**Mac:** LiveMaskRenderer.swift:52-57 (an adjustment's result composited with its layer's CIBlendKernel), :126-134 (a stack's group composited in its base's mode).

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/mac-1.2.10.spec.ts
+++ b/app/tests/e2e/mac-1.2.10.spec.ts
@@ -489,7 +489,7 @@ test("blur adjustment layers draw on the GPU as on the CPU, reduced or not, dimm
   await expectMatchesCpu(page, "opacity");
   await run(page, { type: "SetLayerBlendMode", id, mode: "Multiply" });
   await expectMatchesCpu(page, "blend mode");
-  // A Core-Image-only mode: Normal at full coverage with the original alpha kept (keepsAlpha).
+  // A Core-Image-only mode: that mode at full coverage with the original alpha kept (keepsAlpha).
   await run(page, { type: "SetLayerBlendMode", id, mode: "Linear Burn" });
   await expectMatchesCpu(page, "Linear Burn");
   await run(page, { type: "SetLayerBlendMode", id, mode: "Normal" });
```

Create `app/tests/e2e/modes-1-4-5.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { hueSafeNoisePngBase64, noisePngBase64 } from "./helpers";

// Compositor 1.4.5's blend results (Phase 4.5): an adjustment layer and a clipping stack's base blend in
// their real modes (LiveMaskRenderer.swift:52-57, :126-134 at v1.4.5), where 1.2.10 drew the eight
// modes only Core Image computes as Normal. The engine pins the formulas (engine/tests/blend_modes_v9.rs);
// here the GPU is held to the CPU in every such mode.
const CORE_IMAGE_ONLY = ["Linear Burn", "Linear Dodge (Add)", "Vivid Light", "Linear Light", "Pin Light", "Hard Mix", "Subtract", "Divide"];

/** A 64 x 64 noise layer at zoom 1 on a pinned viewport (adjust-render.spec.ts's setup), with a second,
 * different noise above it and a third clipped to that one; returns the ids of the second and third. */
async function setup(page: Page): Promise<{ doc: string; base: string; child: string }> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const [under, over] = [await page.evaluate(hueSafeNoisePngBase64), await page.evaluate(noisePngBase64)];
  return page.evaluate(async ([under, over]) => {
    const api = (window as any).__compositor;
    const png = (b64: string) => Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 64, false);
    api.engine.importImage(doc, png(under), "under", { x: 32, y: 32 });
    api.engine.importImage(doc, png(over), "base", { x: 32, y: 32 });
    const base = api.engine.state(doc).activeLayerId;
    api.engine.importImage(doc, png(under), "child", { x: 32, y: 32 });
    const child = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "SetLayerOpacity", id: child, opacity: 0.5 });
    api.engine.execute(doc, { type: "ToggleClipping", id: child });
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
    api.setCheckerboard(false);
    return { doc, base, child };
  }, [under, over]);
}

const run = (page: Page, cmd: unknown) => page.evaluate((cmd) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  api.engine.execute(s.activeId, cmd); s.refresh(); s.invalidate();
}, cmd);

/** The largest byte difference between what the GPU drew and the CPU compositor, over the document. */
async function worstAgainstCpu(page: Page): Promise<number> {
  return page.evaluate(async () => {
    const api = (window as any).__compositor;
    const s = api.store.getState(); const d = s.documents[s.activeId];
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    const gl = Array.from(api.readDocumentPixels()) as number[];
    const cpu = Array.from(api.engine.composite(d.id, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height)) as number[];
    return gl.length === cpu.length ? gl.reduce((m, v, i) => Math.max(m, Math.abs(v - cpu[i])), 0) : 256;
  });
}

test("a clipping stack based in each Core-Image-only mode draws on the GPU as on the CPU, and not as Normal", async ({ page }) => {
  const ids = await setup(page);
  const normal = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[]; });
  for (const mode of CORE_IMAGE_ONLY) {
    await run(page, { type: "SetLayerBlendMode", id: ids.base, mode });
    const plan = await page.evaluate((doc) => (window as any).__compositor.engine.renderPlan(doc, null), ids.doc);
    expect(plan.nodes.find((n: { kind: string }) => n.kind === "stack").base.blend, `${mode}: the stack's own mode`).toBe(mode);
    // Vivid Light divides by what is left of the group, which turns the one level the GPU's 8-bit
    // group surface can differ by into up to 3 (measured on p45-scratch, 2026-09-30).
    expect(await worstAgainstCpu(page), mode).toBeLessThanOrEqual(mode === "Vivid Light" ? 3 : 2);
    const cpu = await page.evaluate(() => { const api = (window as any).__compositor; const s = api.store.getState(); return Array.from(api.engine.composite(s.activeId, { x: 0, y: 0, width: 64, height: 64 }, 64, 64)) as number[]; });
    expect(cpu.some((v, i) => Math.abs(v - normal[i]) > 8), `${mode} is not drawn as Normal`).toBe(true);
  }
});

test("a Levels layer in each Core-Image-only mode draws on the GPU as on the CPU", async ({ page }) => {
  await setup(page);
  // Pinned at 721 while an adjustment layer is selected (the chrome grows by a row).
  await page.setViewportSize({ width: 1280, height: 721 });
  const levels = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.execute(s.activeId, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
    const layer = api.engine.state(s.activeId).layers.find((l: any) => l.adjustment);
    const adjustment = JSON.parse(JSON.stringify(layer.adjustment));
    adjustment.levels.ranges[0].outputBlack = 128; adjustment.levels.ranges[0].outputWhite = 128;
    api.engine.execute(s.activeId, { type: "SetAdjustment", id: layer.id, adjustment });
    s.refresh(); s.invalidate();
    return layer.id as string;
  });
  for (const mode of CORE_IMAGE_ONLY) {
    await run(page, { type: "SetLayerBlendMode", id: levels, mode });
    expect(await worstAgainstCpu(page), mode).toBeLessThanOrEqual(2);
  }
});
```

```diff
--- a/engine/tests/blend_modes_v9.rs
+++ b/engine/tests/blend_modes_v9.rs
@@ -153,18 +153,22 @@ fn levels_to_mid_grey() -> LayerAdjustment {
 const CORE_IMAGE_ONLY: [BlendMode; 8] = [BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::VividLight,
     BlendMode::LinearLight, BlendMode::PinLight, BlendMode::HardMix, BlendMode::Subtract, BlendMode::Divide];
 
+/// A mode's name as the formulas above spell it.
+fn name(mode: BlendMode) -> String { serde_json::to_value(mode).unwrap().as_str().unwrap().to_string() }
+
 #[test]
-fn an_adjustment_layer_blends_in_the_core_graphics_mode() {
-    // LiveMaskRenderer.adjust branches on the layer's OWN mode (LiveMaskRenderer.swift:24): any
-    // mode but Normal takes the full-coverage path that keeps the original alpha, and only the
-    // draw inside it goes through `cgMode` (:40), which is Normal for the eight modes only Core
-    // Image computes (LayerAppearance.swift:46-47). Soft Light is a Core Graphics mode, so it
-    // stays. Over an opaque backdrop a per-pixel kind keeps alpha 255 either way, so the eight
-    // composite exactly as Normal does.
+fn an_adjustment_layer_blends_in_its_own_mode() {
+    // Compositor 1.4.5: an adjustment layer in any mode but Normal takes the full-coverage path that
+    // keeps the original alpha (LiveMaskRenderer.swift:36-61), and blends its result in its real mode,
+    // through Core Image for the modes Core Graphics lacks or gets wrong (SeparableBlend.blend,
+    // SeparableBlend.swift:50-59; LiveMaskRenderer.swift:52-57). 1.2.10 drew the eight Core-Image-only
+    // modes as Normal there (`cgMode`). Levels to mid grey over an opaque backdrop: each channel is the
+    // mode's formula of the backdrop and 128.
     let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
+    let backdrop = [200u8, 90, 30, 255];
     let with_mode = |mode: BlendMode| {
         let mut doc = Document::new(2, 1);
-        let under = Layer::with_pixels("B", Raster::from_premultiplied(2, 1, [200u8, 90, 30, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
+        let under = Layer::with_pixels("B", Raster::from_premultiplied(2, 1, backdrop.repeat(2)), Point { x: 0.0, y: 0.0 });
         let mut adj = Layer::blank("Levels", doc.size());
         adj.extra.adjustment = Some(levels_to_mid_grey());
         adj.blend_mode = mode;
@@ -174,41 +178,65 @@ fn an_adjustment_layer_blends_in_the_core_graphics_mode() {
     let (plan, normal) = with_mode(BlendMode::Normal);
     let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
     assert!(!draw.keeps_alpha, "Normal composites through its coverage, alpha and all");
-    for mode in CORE_IMAGE_ONLY {
+    for mode in CORE_IMAGE_ONLY.into_iter().chain([BlendMode::SoftLight]) {
         let (plan, px) = with_mode(mode);
         let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
-        assert_eq!(draw.blend, BlendMode::Normal, "{mode:?} reaches both renderers as Normal");
+        assert_eq!(draw.blend, mode, "{mode:?} reaches both renderers as itself");
         assert!(draw.keeps_alpha, "{mode:?} is not Normal, so the original alpha is kept");
-        assert_eq!(px, normal, "{mode:?}");
+        let want: Vec<u8> = (0..3).map(|c| (expected_blend(&name(mode), backdrop[c] as f32 / 255.0, 128.0 / 255.0).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
+        for c in 0..3 { assert!(px[c].abs_diff(want[c]) <= 1, "{mode:?}, channel {c}: {px:?} against {want:?}"); }
+        assert_eq!(px[3], 255);
+        assert_ne!(px, normal, "{mode:?} is not drawn as Normal");
     }
-    let (plan, soft) = with_mode(BlendMode::SoftLight);
-    let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the adjustment is a plain node") };
-    assert_eq!(draw.blend, BlendMode::SoftLight);
-    assert!(draw.keeps_alpha);
-    assert_ne!(soft, normal, "a Core Graphics mode still blends");
 }
 
 #[test]
-fn a_clipping_stack_composites_in_its_base_core_graphics_mode() {
-    // prepareStacks stores `blend(base).cgMode` (LiveMaskRenderer.swift:74) for the group composite.
+fn a_clipping_stack_composites_in_its_bases_own_mode() {
+    // LiveMaskRenderer.swift:89 (`stackModes` holds the real mode) and :126-134 (the group through
+    // SeparableBlend.draw when the mode needs a surface), at v1.4.5; 1.2.10 stored `cgMode`. The
+    // group is the child over the base, opaque where the base is; it blends over the backdrop by the
+    // base's own formula.
     let region = Rect { x: 0.0, y: 0.0, width: 2.0, height: 1.0 };
+    let (backdrop, base_px, child_px) = ([200u8, 90, 30, 255], [60u8, 150, 110, 255], [40u8, 20, 90, 128]);
     let stack = |mode: BlendMode| {
         let mut doc = Document::new(2, 1);
-        let under = Layer::with_pixels("Backdrop", Raster::from_premultiplied(2, 1, [200u8, 90, 30, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
-        let mut base = Layer::with_pixels("Base", Raster::from_premultiplied(2, 1, [60u8, 150, 110, 255].repeat(2)), Point { x: 0.0, y: 0.0 });
+        let under = Layer::with_pixels("Backdrop", Raster::from_premultiplied(2, 1, backdrop.repeat(2)), Point { x: 0.0, y: 0.0 });
+        let mut base = Layer::with_pixels("Base", Raster::from_premultiplied(2, 1, base_px.repeat(2)), Point { x: 0.0, y: 0.0 });
         base.blend_mode = mode;
-        let mut child = Layer::with_pixels("Child", Raster::from_premultiplied(2, 1, [40u8, 20, 90, 128].repeat(2)), Point { x: 0.0, y: 0.0 });
+        let mut child = Layer::with_pixels("Child", Raster::from_premultiplied(2, 1, child_px.repeat(2)), Point { x: 0.0, y: 0.0 });
         child.mask_source_id = Some(base.id);
         doc.layers = vec![under, base, child];
         (render_plan(&doc, None), composite(&doc, region, 2, 1).pixel(0, 0))
     };
+    let group: Vec<f32> = (0..3).map(|c| (child_px[c] as f32 + base_px[c] as f32 * (1.0 - child_px[3] as f32 / 255.0)) / 255.0).collect();
     let (_, normal) = stack(BlendMode::Normal);
     for mode in CORE_IMAGE_ONLY {
         let (plan, px) = stack(mode);
         let PlanNode::Stack { base, .. } = &plan.nodes[1] else { panic!("base and child form a stack") };
-        assert_eq!(base.blend, BlendMode::Normal, "{mode:?}");
-        assert_eq!(px, normal, "{mode:?}: Core Image only, so the stack composites as Normal");
+        assert_eq!(base.blend, mode, "{mode:?}");
+        let want: Vec<u8> = (0..3).map(|c| (expected_blend(&name(mode), backdrop[c] as f32 / 255.0, group[c]).clamp(0.0, 1.0) * 255.0).round() as u8).collect();
+        for c in 0..3 { assert!(px[c].abs_diff(want[c]) <= 1, "{mode:?}, channel {c}: {px:?} against {want:?}"); }
+        assert_ne!(px, normal, "{mode:?} is not drawn as Normal");
     }
-    let (_, multiply) = stack(BlendMode::Multiply);
-    assert_ne!(multiply, normal, "a Core Graphics mode composites the stack in that mode");
 }
+
+#[test]
+fn a_linear_dodge_stack_exports_the_macs_own_numbers() {
+    // GPUCanvasTests.clippingStacksBlendInTheirBasesMode (GPUCanvasTests.swift:477-514 at v1.4.5):
+    // Base (0.4, 0.2, 0.1); Blended (0.3, 0.3, 0.3) in Linear Dodge, the stack's base; Clipped (0.2,
+    // 0.05, 0), 50 x 50 at the corner, clipped to Blended. Exported: (153, 64, 26) where Clipped
+    // covers, (179, 128, 102) where only Blended does, each within 1.
+    let byte = |v: f64| (v * 255.0).round() as u8;
+    let solid = |name: &str, rgb: [f64; 3], size: u32| Layer::with_pixels(name, Raster::from_premultiplied(size, size, [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255].repeat((size * size) as usize)), Point { x: 0.0, y: 0.0 });
+    let mut doc = Document::new(100, 100);
+    let base = solid("Base", [0.4, 0.2, 0.1], 100);
+    let mut blended = solid("Blended", [0.3, 0.3, 0.3], 100);
+    blended.blend_mode = BlendMode::LinearDodge;
+    let mut clipped = solid("Clipped", [0.2, 0.05, 0.0], 50);
+    clipped.mask_source_id = Some(blended.id);
+    doc.layers = vec![base, blended, clipped];
+    let at = |x: f64, y: f64| composite(&doc, Rect { x, y, width: 1.0, height: 1.0 }, 1, 1).pixel(0, 0);
+    let (covered, bare) = (at(20.0, 20.0), at(80.0, 80.0));
+    for (c, want) in [153u8, 64, 26].into_iter().enumerate() { assert!(covered[c].abs_diff(want) <= 1, "covered {covered:?}"); }
+    for (c, want) in [179u8, 128, 102].into_iter().enumerate() { assert!(bare[c].abs_diff(want) <= 1, "bare {bare:?}"); }
+}
\ No newline at end of file
```

```diff
--- a/engine/tests/mac_1_2_10.rs
+++ b/engine/tests/mac_1_2_10.rs
@@ -130,11 +130,11 @@ fn premultiplied(name: &str) -> (u8, u8, f64) {
 
 #[test]
 fn the_follow_up_probes_this_port_draws_exactly_match_the_mac_render_bit_for_bit() {
-    // Color Balance without Preserve Luminosity, both Add Noise modes, a Levels layer in Divide and
-    // one in Color Dodge (Core Graphics' own formulas agree with this port's here), both clipping
-    // stack bases, and an Invert layer.
-    for name in ["color-balance-no-preserve", "add-noise-uniform", "add-noise-gaussian-mono", "cgmode-levels-divide",
-        "color-dodge-adjustment", "cgmode-stack-bases", "invert"] {
+    // Color Balance without Preserve Luminosity, both Add Noise modes, a Levels layer in Color Dodge
+    // (Core Graphics' own formula agrees with this port's W3C one here, and 1.4.5 draws it through Core
+    // Image), and an Invert layer. `cgmode-levels-divide` and `cgmode-stack-bases` left this list in
+    // Phase 4.5: Compositor 1.4.5 blends them in their real modes (below).
+    for name in ["color-balance-no-preserve", "add-noise-uniform", "add-noise-gaussian-mono", "color-dodge-adjustment", "invert"] {
         let (width, theirs) = mac(name);
         let (d, at) = worst(&ours(name), &theirs, width, 0..width);
         assert_eq!(d, 0, "{name}: worst at {at:?}");
@@ -149,21 +149,101 @@ fn a_tinted_black_and_white_layer_matches_the_mac_render_within_one_level() {
     assert!(d <= 1, "worst {d} at {at:?}");
 }
 
+/// The largest difference over the colour channels of the pixels `inside` picks, straight RGBA8.
+fn worst_where(a: &[u8], b: &[u8], width: u32, inside: impl Fn(u32, u32) -> bool) -> u8 {
+    let mut d = 0u8;
+    for (p, (x, y)) in (0..a.len() / 4).map(|p| (p, ((p as u32) % width, (p as u32) / width))) {
+        if !inside(x, y) { continue; }
+        for c in 0..3 { d = d.max(a[p * 4 + c].abs_diff(b[p * 4 + c])); }
+    }
+    d
+}
+
+// Compositor 1.4.5 blends adjustment layers and clipping stacks in their real modes (LiveMaskRenderer.swift
+// :52-57, :89, :126-134 at v1.4.5), where 1.2.10, whose exports these are, drew the eight modes only Core
+// Image computes as Normal. Until their 1.4.5 re-exports (B1) arrive, the parts 1.4.5 draws differently
+// are held to the formulas and shown to differ from these exports; the rest still matches them.
+
+#[test]
+fn a_levels_layer_in_divide_blends_in_divide() {
+    // Levels to mid grey (128) at 60% over the tonal sweep: each channel moves 60% of the way to the
+    // backdrop divided by 128/255 (at most 1).
+    let (width, theirs) = mac("cgmode-levels-divide");
+    let port = ours("cgmode-levels-divide");
+    let doc = open(&format!("{}/cgmode-levels-divide.comp", fixtures()));
+    let sweep = doc.layers[0].pixels.as_ref().unwrap();
+    let grey = 128.0 / 255.0;
+    let mut want = port.clone();
+    for y in 0..sweep.height { for x in 0..sweep.width {
+        let p = sweep.pixel(x, y);
+        for c in 0..3 {
+            let cb = p[c] as f64 / 255.0;
+            let divided = (cb / grey).min(1.0);
+            want[((y * width + x) * 4) as usize + c] = ((cb + (divided - cb) * 0.6) * 255.0).round() as u8;
+        }
+    }}
+    assert!(worst_where(&port, &want, width, |_, _| true) <= 1, "Divide at 60%");
+    assert!(worst_where(&port, &theirs, width, |_, _| true) >= 4, "1.2.10 drew it as Normal");
+}
+
+#[test]
+fn a_clipping_stack_based_in_subtract_blends_in_subtract() {
+    // Two stacks over the hue sweep: an opaque (60, 150, 110) base, 100 x 100 at (10, 10), in Subtract,
+    // then one in Color Burn at (130, 10), each under a half-alpha (40, 20, 90) child 60 px wide, 20 px
+    // in from the base's left edge. The Color Burn stack is what 1.2.10 drew too (Core Graphics' own
+    // formula agreed with this port's there): bit for bit. The Subtract one is the backdrop minus the
+    // group (the child over the base), at least 0.
+    let (width, theirs) = mac("cgmode-stack-bases");
+    let port = ours("cgmode-stack-bases");
+    assert_eq!(worst_where(&port, &theirs, width, |x, _| x >= 120), 0, "the Color Burn stack");
+    let doc = open(&format!("{}/cgmode-stack-bases.comp", fixtures()));
+    let sweep = doc.layers[0].pixels.as_ref().unwrap();
+    let (base, child) = ([60.0f64, 150.0, 110.0], [40.0f64, 20.0, 90.0, 128.0]);
+    let mut want = port.clone();
+    for y in 10..110u32 { for x in 10..110u32 {
+        let p = sweep.pixel(x, y);
+        for c in 0..3 {
+            let group = if (30..90).contains(&x) { child[c] + base[c] * (1.0 - child[3] / 255.0) } else { base[c] };
+            want[((y * width + x) * 4) as usize + c] = (p[c] as f64 - group).max(0.0).round() as u8;
+        }
+    }}
+    assert!(worst_where(&port, &want, width, |x, _| x < 120) <= 1, "Subtract");
+    assert!(worst_where(&port, &theirs, width, |x, _| x < 120) >= 4, "1.2.10 drew the Subtract stack as Normal");
+}
+
 #[test]
 fn a_blur_layer_in_linear_burn_keeps_the_original_alpha_as_the_mac_does() {
-    // Ruling E-I1 confirmed by the Mac: the canvas edge does not fade (alpha exact). Measured colour
-    // 3, on 28 pixels over 2, all at the canvas edge.
+    // Ruling E-I1 confirmed by the Mac: the canvas edge does not fade (alpha exact against the 1.2.10
+    // export; 1.4.5 keeps the original alpha the same way, LiveMaskRenderer.swift:58). The colour is now
+    // Linear Burn of the original and its blur, both made opaque, the original alpha put back
+    // (`blended_keeping_alpha`), worked out here from this port's own composite without the blur layer
+    // and with it in Normal (the blur itself).
     let (width, theirs) = mac("cgmode-blur-linear-burn");
-    let port = ours("cgmode-blur-linear-burn");
-    let (mut colour, mut alpha) = (0u8, 0u8);
-    for (i, (a, b)) in port.iter().zip(&theirs).enumerate() {
-        if i % 4 == 3 { alpha = alpha.max(a.abs_diff(*b)); } else { colour = colour.max(a.abs_diff(*b)); }
-    }
+    let port = composite_of("cgmode-blur-linear-burn");
+    let straight = port.to_straight();
+    let alpha = (0..straight.len() / 4).map(|p| straight[p * 4 + 3].abs_diff(theirs[p * 4 + 3])).max().unwrap();
     assert_eq!(alpha, 0, "alpha");
-    assert!(colour <= 3, "colour {colour}");
     assert_eq!(width, 160);
+    let comp = format!("{}/cgmode-blur-linear-burn.comp", fixtures());
+    let region = Rect { x: 0.0, y: 0.0, width: 160.0, height: 100.0 };
+    let mut doc = open(&comp);
+    doc.layers[2].visible = false;
+    let original = composite(&doc, region, 160, 100);
+    doc.layers[2].visible = true;
+    doc.layers[2].blend_mode = BlendMode::Normal;
+    let blurred = composite(&doc, region, 160, 100);
+    let opaque = |p: &[u8], c: usize| { let a = p[3] as u32; if a == 0 { 0.0 } else { ((p[c] as u32 * 255 + a / 2) / a).min(255) as f64 / 255.0 } };
+    let mut colour = 0u8;
+    for ((o, b), got) in original.bytes().chunks_exact(4).zip(blurred.bytes().chunks_exact(4)).zip(port.bytes().chunks_exact(4)) {
+        for c in 0..3 {
+            let burnt = (opaque(o, c) + opaque(b, c) - 1.0).max(0.0);
+            let want = (((burnt * 255.0).round() as u32 * o[3] as u32 + 127) / 255) as u8;
+            colour = colour.max(got[c].abs_diff(want));
+        }
+    }
+    assert!(colour <= 1, "Linear Burn of the original and its blur: {colour}");
+    assert!(worst_where(&straight, &theirs, width, |_, _| true) >= 4, "1.2.10 drew the blur in Normal");
 }
-
 #[test]
 fn the_gaussian_blur_probes_match_the_mac_render_premultiplied() {
     // Measured colour and alpha: radius 6 (the exact kernel) 2 and 2; radius 40 (the halved path) 2
```

```diff
--- a/engine/tests/spatial_adjustments.rs
+++ b/engine/tests/spatial_adjustments.rs
@@ -253,12 +253,13 @@ fn a_blend_mode_blends_at_full_coverage_and_keeps_the_original_alpha() {
 
 #[test]
 fn a_blur_in_a_core_image_only_mode_keeps_the_original_alpha_as_the_mac_does() {
-    // Ruling E-I1. LiveMaskRenderer.swift:24 branches on the layer's OWN mode: a blur layer in
-    // Linear Burn takes the full-coverage path, drawn in Normal (its cgMode, :40) over the opaque
-    // original, then gets the original alpha back (:43). So the canvas edge does not fade, the
-    // colour is the unpremultiplied blur (of a flat colour: that colour), and nothing spreads
-    // where the original is clear. Taking the Normal path instead gives alpha 139 at x = 0 and
-    // [3, 1, 0, 4] at x = 40 (measured).
+    // Ruling E-I1. LiveMaskRenderer.swift:36-61 (v1.4.5) branches on the layer's OWN mode: a blur
+    // layer in Linear Burn takes the full-coverage path, blended in Linear Burn itself (through Core
+    // Image, :52-57; 1.2.10 drew it as Normal) over the opaque original, then gets the original alpha
+    // back. So the canvas edge does not fade, the colour is Linear Burn of the colour and its blur (of
+    // a flat colour: that colour, so 2 c - 255, at least 0), and nothing spreads where the original is
+    // clear. Taking the Normal path instead gives alpha 139 at x = 0 and [3, 1, 0, 4] at x = 40
+    // (measured).
     let mut doc = Document::new(64, 20);
     let block = Layer::with_pixels("Block", Raster::from_premultiplied(48, 20, [200u8, 90, 30, 255].repeat(960)), Point { x: -16.0, y: 0.0 });
     let mut b = blur(&doc, 4.0);
@@ -266,12 +267,12 @@ fn a_blur_in_a_core_image_only_mode_keeps_the_original_alpha_as_the_mac_does() {
     doc.layers = vec![block, b];
     let plan = render_plan(&doc, None);
     let PlanNode::Layer { draw } = &plan.nodes[1] else { panic!("the blur is a plain node") };
-    assert!(draw.keeps_alpha && draw.blend == BlendMode::Normal);
+    assert!(draw.keeps_alpha && draw.blend == BlendMode::LinearBurn);
     let out = full(&doc);
     for x in [0u32, 5, 31] {
         let p = out.pixel(x, 10);
         assert_eq!(p[3], 255, "alpha at x {x} is the original's: {p:?}");
-        for (c, want) in [200i32, 90, 30].into_iter().enumerate() {
+        for (c, want) in [200i32, 90, 30].map(|v| (2 * v - 255).max(0)).into_iter().enumerate() {
             assert!((p[c] as i32 - want).abs() <= 1, "colour at x {x}, channel {c}: {p:?}");
         }
     }
```


`a_linear_dodge_stack_exports_the_macs_own_numbers` holds a Linear Dodge stack to the numbers the formula gives for its colours, (153, 64, 26) over (179, 128, 102), within 1. In `mac_1_2_10.rs`, `cgmode-levels-divide` and `cgmode-stack-bases` leave the bit-for-bit list: each has its own test that holds the Divide / Subtract parts to the formula and requires a measurable difference from the 1.2.10 export (awaiting the B1 re-exports), and keeps the parts 1.2.10 already drew in their mode (Color Burn) bit for bit. The Linear Burn blur keeps its alpha exactly (the Mac's own rule) and takes its colour from the formula over the port's own hidden and Normal composites. `color-dodge-adjustment` is unchanged: Color Dodge was always drawn in its mode.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test blend_modes_v9 --test mac_1_2_10 --test spatial_adjustments`
Expected: `an_adjustment_layer_blends_in_its_own_mode`, `a_clipping_stack_composites_in_its_bases_own_mode`, `a_linear_dodge_stack_exports_the_macs_own_numbers`, `a_levels_layer_in_divide_blends_in_divide`, `a_clipping_stack_based_in_subtract_blends_in_subtract`, the Linear Burn blur test and the spatial Linear Burn test fail (each sees Normal). The e2e file fails in every mode ("not drawn as Normal" and the plan's `base.blend`).

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/gl/programs.ts
+++ b/app/src/canvas/gl/programs.ts
@@ -1,7 +1,7 @@
 import type { BlendMode } from "../../engine/types";
 
-// Mirrors BlendMode in engine/src/blend.rs. An adjustment layer's and a stack base's draw arrive
-// already mapped to their Core Graphics mode by the plan (BlendMode::cg_mode).
+// Mirrors BlendMode in engine/src/blend.rs. Every draw arrives in its own mode, an adjustment layer's
+// and a stack base's too (Compositor 1.4.5 blends them through Core Image where Core Graphics cannot).
 export const BLEND_INDEX: Record<BlendMode, number> = { Normal: 0, Multiply: 1, Screen: 2, Overlay: 3, Darken: 4, Lighten: 5, Difference: 6, "Color Dodge": 7, "Color Burn": 8, Hue: 9, Saturation: 10, Color: 11, Luminosity: 12,
   "Linear Burn": 13, "Linear Dodge (Add)": 14, "Soft Light": 15, "Hard Light": 16, "Vivid Light": 17, "Linear Light": 18, "Pin Light": 19, "Hard Mix": 20, Exclusion: 21, Subtract: 22, Divide: 23 };
 export const ADJUST_KIND: Record<string, number> = { identity: 0, tables: 1, gradientMap: 2, hsv: 3, grain: 4, invert: 5, blackWhite: 6, colorBalance: 7, addNoise: 8 };
```

```diff
--- a/engine/src/compositor.rs
+++ b/engine/src/compositor.rs
@@ -249,7 +249,7 @@ fn adjust_target(doc: &Document, plan: &RenderPlan, target: &mut Target, draw: &
         if k <= 0.0 { continue; }
         // Full strength in Normal: the Mac's own 8-bit kernel, so an export matches it to the level.
         // Not for a layer whose own mode is not Normal (`keeps_alpha`): the Mac takes its
-        // full-coverage path there even when the Core Graphics mode is Normal.
+        // full-coverage path there.
         if k >= 1.0 && blend == BlendMode::Normal && !draw.keeps_alpha {
             let px = [target.data[i], target.data[i + 1], target.data[i + 2], target.data[i + 3]];
             if let Some(out) = prepared.pixel(px, p) { target.data[i..i + 4].copy_from_slice(&out); continue; }
@@ -325,8 +325,8 @@ fn spatial_target(doc: &Document, plan: &RenderPlan, target: &mut Target, draw:
         if k <= 0.0 { continue; }
         let original = [target.data[i], target.data[i + 1], target.data[i + 2], target.data[i + 3]];
         let mut result = [blurred[i], blurred[i + 1], blurred[i + 2], blurred[i + 3]];
-        // The layer's own mode is not Normal: full coverage in its Core Graphics mode, original
-        // alpha kept, even when that mode is Normal (Task 1, `keeps_alpha`).
+        // The layer's own mode is not Normal: full coverage in that mode, original alpha kept
+        // (`keeps_alpha`).
         if draw.keeps_alpha { result = blended_keeping_alpha(blend, original, result); }
         target.data[i..i + 4].copy_from_slice(&toward(original, result, k));
     }}
```

```diff
--- a/engine/src/manifest.rs
+++ b/engine/src/manifest.rs
@@ -48,25 +48,6 @@ pub enum BlendMode {
     #[serde(rename = "Divide")] Divide,
 }
 
-impl BlendMode {
-    /// The mode Core Graphics draws for this one (`LayerBlendMode.cgMode`, LayerAppearance.swift:28-49):
-    /// the same mode, except the eight modes only Core Image computes, which are Normal there. The
-    /// Mac blends through `cgMode` where it composites a whole surface in one draw: an adjustment
-    /// layer's blend (LiveMaskRenderer.swift:40, inside the branch its REAL mode chose at :24, which
-    /// the plan carries as `LayerDraw.keeps_alpha`) and a clipping stack's group (:74, :105). A
-    /// layer drawn on its own goes through SeparableBlend and gets the real mode. Color Burn and
-    /// Color Dodge map to Core Graphics' own modes, whose formulas the Mac calls wrong
-    /// (LayerAppearance.swift:51-52); this port applies its W3C formulas there too, and the Task 12
-    /// probes measure the difference. The render plan applies this, so both renderers inherit it.
-    pub fn cg_mode(self) -> BlendMode {
-        match self {
-            BlendMode::LinearBurn | BlendMode::LinearDodge | BlendMode::VividLight | BlendMode::LinearLight
-            | BlendMode::PinLight | BlendMode::HardMix | BlendMode::Subtract | BlendMode::Divide => BlendMode::Normal,
-            other => other,
-        }
-    }
-}
-
 /// A saved alignment guide (v8, `CanvasGuide`, Document/Guides.swift:5-10). `position` is in
 /// document pixels: X for a vertical guide, Y for a horizontal one; it may be fractional and may
 /// lie outside the canvas.
```

```diff
--- a/engine/src/plan.rs
+++ b/engine/src/plan.rs
@@ -28,10 +28,8 @@ pub struct LayerDraw {
     pub opacity: f64,
     pub blend: BlendMode,
     /// An adjustment layer whose own mode is not Normal: the Mac blends its result over the
-    /// original at full coverage, both made opaque, in `blend` (its Core Graphics mode), then puts
-    /// the ORIGINAL alpha back (LiveMaskRenderer.swift:24-46; BrushPixels.c:23-45). `blend` alone
-    /// cannot say this: the eight Core-Image-only modes arrive there as Normal. False for every
-    /// other draw.
+    /// original at full coverage, both made opaque, in `blend`, then puts the ORIGINAL alpha back
+    /// (LiveMaskRenderer.swift:36-61 at v1.4.5; BrushPixels.c:23-45). False for every other draw.
     pub keeps_alpha: bool,
     pub coverages: Vec<Coverage>,
     #[serde(with = "ids::upper_opt")] pub clip: Option<Uuid>,
@@ -181,8 +179,10 @@ pub(crate) fn draw_for(_doc: &Document, by_id: &HashMap<Uuid, &Layer>, layer: &L
     LayerDraw {
         id: layer.id, transform, corners, pixels_width: pw, pixels_height: ph, pixels_revision: layer.pixels_revision,
         opacity: effective_opacity(by_id, layer),
-        // An adjustment layer blends through Core Graphics (BlendMode::cg_mode).
-        blend: if layer.is_adjustment() { layer.blend_mode.cg_mode() } else { layer.blend_mode },
+        // Every draw in its own mode: an adjustment layer too, since Compositor 1.4.5 blends it through
+        // Core Image for the modes Core Graphics lacks or gets wrong (LiveMaskRenderer.swift:52-57), where
+        // 1.2.10 drew those as Normal.
+        blend: layer.blend_mode,
         keeps_alpha: layer.is_adjustment() && layer.blend_mode != BlendMode::Normal,
         coverages, clip: layer.mask_source_id,
         adjustment: displayed_adjustment(layer, edit),
@@ -217,10 +217,10 @@ pub fn render_plan(doc: &Document, edit: Option<&PreviewEdit>) -> RenderPlan {
         // out of reach there is nothing beneath it to adjust, as macOS's drawComposite does.
         if layer.is_adjustment() && layer.mask_source_id.is_some() { continue; }
         if let Some(children) = stacks.get(id) {
-            let mut base = draw_for(doc, &by_id, layer, edit, false);
-            // The group composites in the base's Core Graphics mode (BlendMode::cg_mode). The base
-            // itself is drawn into the group as Normal by both renderers, so nothing else reads it.
-            base.blend = base.blend.cg_mode();
+            // The group composites in the base's own mode, through Core Image where Core Graphics has
+            // none (LiveMaskRenderer.swift:89, :126-134 at v1.4.5; 1.2.10 drew those as Normal). The base
+            // itself is drawn into the group as Normal by both renderers.
+            let base = draw_for(doc, &by_id, layer, edit, false);
             let folder = folder_coverages(&by_id, layer, edit);
             let kids: Vec<LayerDraw> = children.iter().map(|c| { let mut d = draw_for(doc, &by_id, by_id[c], edit, false); d.clip = None; d }).collect();
             note_source(&base, &mut needed_sources);
```


- [ ] **Step 4: Run the whole set**

`cargo test -p compositor-engine`: 569 passed, 10 ignored. `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 165 passed, 19 skipped. The GPU is within 2 of the CPU in all eight modes for the Levels layer and the stack, except the stack in Vivid Light, within 3 (measured, ruling OQ7). The frame cost is timed in Task 8's perf case.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Map an adjustment layer's mode through a Normal-for-the-eight table again: `an_adjustment_layer_blends_in_its_own_mode`, `a_levels_layer_in_divide_blends_in_divide` and the Linear Burn blur test fail (measured).
2. Composite the stack's base as Normal: `a_clipping_stack_composites_in_its_bases_own_mode`, `a_linear_dodge_stack_exports_the_macs_own_numbers` and `a_clipping_stack_based_in_subtract_blends_in_subtract` fail (measured).
3. In the GPU renderer (`app/src/canvas/gl-renderer.ts`), composite the stack's group as Normal instead of in `base.blend`: the modes e2e fails, Linear Burn 128 levels (measured).

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/modes-1-4-5.spec.ts
git commit -m "fix: adjustment layers and clipping-stack bases blend in their own modes, as Compositor 1.4.5 blends them" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/gl/programs.ts app/tests/e2e/mac-1.2.10.spec.ts app/tests/e2e/modes-1-4-5.spec.ts engine/src/compositor.rs engine/src/manifest.rs engine/src/plan.rs engine/tests/blend_modes_v9.rs engine/tests/mac_1_2_10.rs engine/tests/spatial_adjustments.rs
```


---

### Task 8: Positive saturation divides by what is left

Compositor 1.2.10 multiplied saturation by `1 + amount / 100` both ways, so +50 gave 1.5 times. 1.4.5 follows Photoshop: below 0 it scales toward grey, above 0 it divides by what is left, `min(1, s / (1 - a))`, so +50 doubles it and +100 takes any colour all the way (`adjustedSaturation`, HueSaturation.swift:342-348; ruling OQ8).

**Files:**
- Modify: `engine/src/adjust/hsv.rs` (`adjusted_saturation`, used by `adjust_rgb`), `app/src/canvas/gl/programs.ts` (`adjustedSaturation` in `ADJUST_GLSL`)
- Modify (tests): `engine/tests/hsv.rs`, `app/tests/e2e/perf-4-5.spec.ts` (the frame of every 1.4.5 result)

**Mac:** HueSaturation.swift:342-348; HueSaturationTests.swift:274-282 (the numbers ported).

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -148,3 +148,72 @@ test("F1: the frame after a job's result at 24 and 100 MP, Levels on a layer and
     }
   }
 });
+
+test("1.4.5 results: frames of a document with a Soft Light layer, Levels in Linear Dodge, a stack based in Vivid Light and Hue/Saturation +50, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const run = (doc: string, cmd: unknown) => api.engine.execute(doc, cmd);
+      const doc = api.engine.newDocument(10, 10, false);
+      run(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const base = api.engine.state(doc).layers[0].id;
+      // A Soft Light copy, a stack based in Vivid Light with a half-opaque copy clipped to it, a Levels
+      // layer in Linear Dodge and a Hue/Saturation layer at +50: every result Tasks 6-8 changed.
+      const top = () => api.engine.state(doc).activeLayerId as string;
+      run(doc, { type: "DuplicateLayer", id: base }); run(doc, { type: "SetLayerBlendMode", id: top(), mode: "Soft Light" });
+      run(doc, { type: "DuplicateLayer", id: base }); const stack = top(); run(doc, { type: "SetLayerBlendMode", id: stack, mode: "Vivid Light" });
+      run(doc, { type: "DuplicateLayer", id: base }); const child = top();
+      run(doc, { type: "SetLayerOpacity", id: child, opacity: 0.5 }); run(doc, { type: "ToggleClipping", id: child });
+      run(doc, { type: "AddAdjustmentLayer", kind: "Levels", seed: 0, shadows: null, highlights: null });
+      run(doc, { type: "SetLayerBlendMode", id: top(), mode: "Linear Dodge (Add)" });
+      run(doc, { type: "AddAdjustmentLayer", kind: "Hue/Saturation", seed: 0, shadows: null, highlights: null });
+      const hsv = api.engine.state(doc).layers.find((l: any) => l.id === top());
+      const saturate = (amount: number) => run(doc, { type: "SetAdjustment", id: hsv.id, adjustment: { ...hsv.adjustment,
+        hsvSettings: { range: "Master", colorize: false, invertRange: false,
+          adjustments: { Master: { hue: 0, saturation: amount, lightness: 0 } }, bands: hsv.adjustment.hsvSettings?.bands ?? {} } } });
+      saturate(50);
+      s().openDocument(doc);
+      await settle();
+      // Cold (LL-074): the first frame compiles the programs and uploads every layer; logged only.
+      result["cold first frame ms"] = Math.round(frame());
+      for (const zoom of ["fit", "1:1"]) {
+        if (zoom === "1:1") { await api.setZoom(1); await settle(); result["cold first 1:1 frame ms"] = Math.round(frame()); }
+        let worst = 0;
+        for (let i = 0; i < 5; i++) { s().invalidate(); worst = Math.max(worst, frame()); await settle(); }
+        result[`${zoom}: frame, worst of 5 ms`] = Math.round(worst);
+        // Dragging the Saturation field: the engine's edit, the store and the frame, per step. One step first,
+        // logged only (LL-074): a first run measured 834 ms for the first step at 100 MP fit, 16-31 ms after.
+        let t = performance.now(); saturate(45); s().refresh(doc); frame();
+        result[`cold first saturation step at ${zoom} ms`] = Math.round(performance.now() - t);
+        await settle();
+        let step = 0;
+        for (const amount of [40, 45, 55, 60, 50]) {
+          const t0 = performance.now();
+          saturate(amount); s().refresh(doc);
+          frame();
+          step = Math.max(step, performance.now() - t0);
+          await settle();
+        }
+        result[`${zoom}: saturation step (engine, store and frame), worst ms`] = Math.round(step);
+      }
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`1.4.5 results (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`1.4.5 results renderer: ${renderer}`);
+  // Measured on the HD 520 (p45-scratch, 2026-09-30, three runs): frames 24-42 ms and saturation steps 26-43 ms at both
+  // sizes and zooms; the seven-layer document with a group costs about what 4b-1's drag tick did (50 ms budget).
+  for (const [label] of SIZES) for (const zoom of ["fit", "1:1"]) {
+    expect(out[`${label} ${zoom}: frame, worst of 5 ms`]).toBeLessThan(80);
+    expect(out[`${label} ${zoom}: saturation step (engine, store and frame), worst ms`]).toBeLessThan(100);
+  }
+});
\ No newline at end of file
```

```diff
--- a/engine/tests/hsv.rs
+++ b/engine/tests/hsv.rs
@@ -103,3 +103,32 @@ fn colorize_matches_the_macs_raw_negative_hue_wrap() {
     let px = straight(&out, 0);
     assert!(near(px, expected_bytes([-30.0_f64 % 360.0, 1.0, 0.5])), "{px:?}");
 }
+
+#[test]
+fn positive_saturation_divides_by_what_is_left_as_compositor_1_4_5_does() {
+    // HueSaturationTests.swift:275-282 (v1.4.5), number for number.
+    assert!((adjusted_saturation(0.2, 50.0) - 0.4).abs() < 1e-9);
+    assert!((adjusted_saturation(0.3, 62.0) - 0.3 / 0.38).abs() < 1e-9);
+    assert_eq!(adjusted_saturation(0.1, 100.0), 1.0);
+    assert_eq!(adjusted_saturation(0.8, 50.0), 1.0);
+    assert_eq!(adjusted_saturation(0.0, 100.0), 0.0);
+    assert!((adjusted_saturation(0.6, -50.0) - 0.3).abs() < 1e-9);
+}
+
+#[test]
+fn a_pale_red_at_plus_50_doubles_its_saturation() {
+    // (153, 102, 102) is hue 0, saturation 0.2, lightness 0.5. +50 makes it 0.4, which is
+    // (178.5, 76.5, 76.5); Compositor 1.2.10's 1 + amount / 100 made it 0.3, 12 levels short.
+    let pale = Raster::from_premultiplied(2, 1, vec![153, 102, 102, 255, 128, 128, 128, 255]);
+    let hsl = rgb_to_hsl([0.6, 0.4, 0.4]);
+    assert!((hsl[1] - 0.2).abs() < 1e-9 && (hsl[2] - 0.5).abs() < 1e-9, "{hsl:?}");
+    let out = apply_hsv(&pale, &HueSaturationSettings::new(0.0, 50.0, 0.0, false, ColorRange::Master));
+    let px = straight(&out, 0);
+    let expected = expected_bytes([0.0, 0.4, 0.5]);
+    for c in 0..3 { assert!((px[c] - expected[c]).abs() <= 1, "{px:?} vs {expected:?}"); }
+    let old = expected_bytes([0.0, 0.3, 0.5]);
+    assert!((px[0] - old[0]).abs() >= 10, "1.2.10's multiply would give {old:?}; got {px:?}");
+    assert_eq!(straight(&out, 1), [128, 128, 128, 255], "grey stays grey");
+    let full = apply_hsv(&pale, &HueSaturationSettings::new(0.0, 100.0, 0.0, false, ColorRange::Master));
+    assert!(near(straight(&full, 0), expected_bytes([0.0, 1.0, 0.5])), "+100 takes any colour all the way: {:?}", straight(&full, 0));
+}
\ No newline at end of file
```


The perf case builds, at 24 and 100 MP, a document holding every result Tasks 6-8 changed (a Soft Light copy, a stack based in Vivid Light with a half-opaque copy clipped to it, Levels in Linear Dodge, Hue/Saturation at +50) and times the frame, at fit and at 1:1, and a Saturation step (the engine's edit, the store and the frame) after one step logged as cold.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test hsv`
Expected: does not compile (`adjusted_saturation` does not exist). With the function stubbed as the old multiply, both new tests fail (0.3 where 0.4 is expected; the pale red comes out 166, not 178 or 179).

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/gl/programs.ts
+++ b/app/src/canvas/gl/programs.ts
@@ -173,6 +173,14 @@ vec3 rgbToHsl(vec3 c) {
 // wherever hsv.rs uses '%' on a value that can be negative; the non-colorize path wraps negatives
 // itself, which makes it equal to a floored mod, so mod() stays correct there.
 float rem(float x, float y) { return x - y * trunc(x / y); }
+// hsv.rs adjusted_saturation (HueSaturation.swift:342-348): below 0 scales toward grey, above 0
+// divides by what is left, +100 takes any colour all the way.
+float adjustedSaturation(float s, float amount) {
+  float a = clamp(amount / 100.0, -1.0, 1.0);
+  if (a <= 0.0) return max(0.0, s * (1.0 + a));
+  if (a >= 1.0) return s > 0.0 ? 1.0 : 0.0;
+  return min(1.0, s / (1.0 - a));
+}
 vec3 hslToRgb(vec3 hsl) {
   if (hsl.y <= 0.0) return vec3(hsl.z);
   float chroma = (1.0 - abs(2.0 * hsl.z - 1.0)) * hsl.y;
@@ -263,7 +271,7 @@ vec3 throughHsl(vec3 c) {
     lightnessAmount = sampled.z / 100.0;
     hsl.x = mod(hsl.x + sampled.x, 360.0);
     if (hsl.x < 0.0) hsl.x += 360.0;
-    hsl.y = clamp(hsl.y * (1.0 + sampled.y / 100.0), 0.0, 1.0);
+    hsl.y = adjustedSaturation(hsl.y, sampled.y);
   }
   float amount = clamp(lightnessAmount, -1.0, 1.0);
   hsl.z = amount >= 0.0 ? hsl.z + (1.0 - hsl.z) * amount : hsl.z * (1.0 + amount);
```

```diff
--- a/engine/src/adjust/hsv.rs
+++ b/engine/src/adjust/hsv.rs
@@ -54,6 +54,18 @@ pub fn shifted_hue(hue: f64, settings: &HueSaturationSettings) -> f64 {
     if shifted < 0.0 { shifted + 360.0 } else { shifted }
 }
 
+/// Photoshop's Saturation, as Compositor 1.4.5 computes it (`adjustedSaturation`,
+/// HueSaturation.swift:342-348, commit fe7a83d): `amount` (-100 to 100) below 0 scales toward grey
+/// (-100 is grey); above 0 it divides by what is left, so +50 doubles it and +100 takes any colour all
+/// the way. Multiplicative both ways, so neutral greys stay neutral. Compositor 1.2.10 multiplied by
+/// `1 + amount / 100` both ways.
+pub fn adjusted_saturation(saturation: f64, amount: f64) -> f64 {
+    let a = (amount / 100.0).clamp(-1.0, 1.0);
+    if a <= 0.0 { return (saturation * (1.0 + a)).max(0.0); }
+    if a >= 1.0 { return if saturation > 0.0 { 1.0 } else { 0.0 }; }
+    (saturation / (1.0 - a)).min(1.0)
+}
+
 /// One straight colour through the settings. `response` is `hue_response`, passed in so a whole
 /// raster shares it.
 pub fn adjust_rgb(rgb: [f64; 3], settings: &HueSaturationSettings, response: &[[f64; 3]]) -> [f64; 3] {
@@ -69,8 +81,7 @@ pub fn adjust_rgb(rgb: [f64; 3], settings: &HueSaturationSettings, response: &[[
         lightness_amount = sampled[2] / 100.0;
         hue = (hue + sampled[0]) % 360.0;
         if hue < 0.0 { hue += 360.0; }
-        // Multiplicative, so neutral grays stay neutral.
-        saturation = (saturation * (1.0 + sampled[1] / 100.0)).clamp(0.0, 1.0);
+        saturation = adjusted_saturation(saturation, sampled[1]);
     }
     // Lightness pulls toward white above 0 and toward black below, reaching either at +/-100.
     let amount = lightness_amount.clamp(-1.0, 1.0);
```


- [ ] **Step 4: Run the whole set, then the timings**

`cargo test -p compositor-engine`: 571 passed, 10 ignored. `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 165 passed, 20 skipped (`adjust-render.spec.ts` holds the GPU's saturation to the CPU's within 2, with Master +15, Blues +60, Reds +70 and +80).

Timings (`pnpm wasm`, then `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts -g "1.4.5 results"`), three runs on the scratch clone: frames 24-42 ms and Saturation steps 26-43 ms at both sizes and zooms, budgets 80 and 100. Before the cold step was added, the first step at 100 MP fit once took 834 ms; it is now logged, not timed (LL-074). Then the whole file: the first run failed F1's 24 MP Levels frame-after at 127 ms (budget 100) while every other case passed; re-run once, everything passed (F1 frames after 11, 18, 39, 18). Both runs reported; nothing widened (ruling OQ3). Release wasm 2,981,475 bytes.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `adjusted_saturation` multiplies by `1 + a` above 0: both new `hsv` tests fail (measured).
2. Drop the `a >= 1` branch: `positive_saturation_divides_by_what_is_left_as_compositor_1_4_5_does` fails, 1 where 0 is expected (a grey at +100 divides 0 by 0, and `min` turns the NaN into 1) (measured).
3. GLSL keeps the old multiply: `adjust-render.spec.ts`'s Hue/Saturation test fails, 69 levels (measured).

- [ ] **Step 6: Commit**

```
git commit -m "fix: Hue/Saturation raises saturation by dividing by what is left, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/gl/programs.ts app/tests/e2e/perf-4-5.spec.ts engine/src/adjust/hsv.rs engine/tests/hsv.rs
```


---

### Task 9: Add Mask with a selection reveals it; Alt-click hides it

The user's ruling of 2026-09-30 lifts the 2026-09-27 rule: 1.4.5 (and Photoshop) reveal the selection when Add Mask is clicked with one, and Option-click (Alt here) hides it (ruling OQ9). Without a selection nothing changes but the undo names.

**Files:**
- Modify: `engine/src/ops/selection.rs` (`add_mask_from_selection`'s tones swapped), `engine/src/command.rs` (the action names: "Reveal Selection", "Hide Selection", "Add Reveal-All Mask", "Add Hide-All Mask"), `app/src/panels/LayersList.tsx` (the footer button passes `!e.altKey`, and its tooltip), `app/src/actions/layers.ts` (a comment)
- Modify (tests): `engine/tests/selection_masks.rs`, `app/tests/e2e/selection.spec.ts`, `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** LayerMask.swift:237-267 (`addMask(revealing:)`: black filled, the clip painted white when revealing, "Reveal Selection" / "Hide Selection"), :270-278 ("Add Reveal-All Mask" / "Add Hide-All Mask"), LayerMaskMenu.swift:9-14 (the button, Option-click, the tooltips), NativeLayerList.swift:182-194 and :300-306 (the menu's Reveal All / Hide All go through the same `addMask`); SelectionEditTests.swift:160-219 (ported).

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -216,4 +216,58 @@ test("1.4.5 results: frames of a document with a Soft Light layer, Levels in Lin
     expect(out[`${label} ${zoom}: frame, worst of 5 ms`]).toBeLessThan(80);
     expect(out[`${label} ${zoom}: saturation step (engine, store and frame), worst ms`]).toBeLessThan(100);
   }
+});
+test("Add Mask with a selection at 24 and 100 MP: Reveal Selection and Hide Selection, the step and the frame after", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      s().openDocument(doc);
+      await settle(); frame();
+      // An antialiased ellipse over the middle 80%, then the button (Reveal) or Alt-click (Hide): the
+      // store's step (the engine rasterizes the clip at the layer's size) and the frame that uploads the mask.
+      const once = async (revealing: boolean) => {
+        api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[w * 0.1, h * 0.1], [w * 0.9, h * 0.1], [w * 0.9, h * 0.9], [w * 0.1, h * 0.9]], mode: "Replace", antialiased: true });
+        s().refresh(doc); frame(); await settle();
+        const t0 = performance.now();
+        s().run({ type: "AddMaskFromSelection", id: layer, revealing });
+        const step = performance.now() - t0;
+        const after = frame();
+        await settle();
+        api.engine.execute(doc, { type: "DeleteMask", id: layer }); s().refresh(doc); frame(); await settle();
+        return [Math.round(step), Math.round(after)];
+      };
+      // Cold (LL-074): the first one at this size, logged only.
+      [result["cold step ms"], result["cold frame after ms"]] = await once(true);
+      for (const [name, revealing] of [["Reveal Selection", true], ["Hide Selection", false]] as [string, boolean][]) {
+        let step = 0, after = 0;
+        for (let i = 0; i < 3; i++) { const [a, b] = await once(revealing); step = Math.max(step, a); after = Math.max(after, b); }
+        result[`${name}: step, worst of 3 ms`] = step;
+        result[`${name}: frame after, worst of 3 ms`] = after;
+      }
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`Add Mask with a selection (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`Add Mask renderer: ${renderer}`);
+  // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 162-212 ms at 24 MP and 506-545 ms at 100 MP
+  // (the clip rasterized at the layer's size on the UI thread, as since Phase 4a; the swap of tones costs
+  // nothing, ruling OQ9); frames after 19-20 ms and 59-77 ms. The frame budgets are 4b-1's for a whole mask
+  // upload (150 / 400); the step ones leave this laptop's swings room (ruling OQ17).
+  for (const [label] of SIZES) for (const name of ["Reveal Selection", "Hide Selection"]) {
+    expect(out[`${label} ${name}: step, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 400 : 1000);
+    expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 150 : 400);
+  }
 });
\ No newline at end of file
```

```diff
--- a/app/tests/e2e/selection.spec.ts
+++ b/app/tests/e2e/selection.spec.ts
@@ -200,17 +200,25 @@ test("Invert, Levels and Delete stay inside the selection; an adjustment layer i
   expect(await pixel(page, [40, 4])).toEqual([255, 255, 0, 255]);
 });
 
-test("Add Mask with a selection hides it and uses it up; the Crop tool starts at the selection", async ({ page }) => {
+test("Add Mask with a selection reveals it (Alt-click hides it) and uses it up; the Crop tool starts at the selection", async ({ page }) => {
   await setup(page);
   await page.keyboard.press("m");
   await drag(page, [8, 6], [24, 30]);
   await page.keyboard.press("c");
   expect(await page.evaluate(() => (window as any).__compositor.store.getState().cropRect)).toEqual({ x: 8, y: 6, width: 16, height: 24 });
   await page.keyboard.press("Escape");
+  await expect(page.getByTestId("layer-add-mask")).toHaveAttribute("title", /revealing the selection/);
   await page.getByTestId("layer-add-mask").click();
   const s = await state(page);
   expect(s.selection).toBeNull();
   expect(s.layers[0].hasMask).toBe(true);
+  // Reveal Selection (LayerMask.swift:237-267 at v1.4.5): the selection shows, the rest is hidden.
+  expect((await pixel(page, [12, 12]))[3]).toBe(255);
+  expect((await pixel(page, [40, 40]))[3]).toBe(0);
+  await page.keyboard.press("Control+z");
+  expect((await state(page)).selection).not.toBeNull();
+  await page.getByTestId("layer-add-mask").click({ modifiers: ["Alt"] });
+  expect((await state(page)).layers[0].hasMask).toBe(true);
   expect((await pixel(page, [12, 12]))[3]).toBe(0);
   expect((await pixel(page, [40, 40]))[3]).toBe(255);
 });
```

```diff
--- a/engine/tests/selection_masks.rs
+++ b/engine/tests/selection_masks.rs
@@ -1,8 +1,10 @@
 //! Delete with a selection and Add Mask from Selection (Phase 4a), ported from Compositor for
 //! Mac's SelectionEditTests: deleteClearsSelectedPixelsOrDeletesTheLayerWithoutASelection,
-//! clipFollowsScaledLayersAndSoftensEdges, maskButtonAddsWhiteMaskOrHidesTheSelection,
-//! layerMenuMasksUseTheSelection, maskFromSelectionLinesUpOnScaledLayers and (adapted: Fill is not
-//! in this phase) maskFillHidesOnlyTheSelectedArea, with their sizes, points and expectations.
+//! clipFollowsScaledLayersAndSoftensEdges, and (adapted: Fill is not in this phase)
+//! maskFillHidesOnlyTheSelectedArea, with their sizes, points and expectations. Add Mask with a
+//! selection follows v1.4.5 (Phase 4.5, the user's ruling of 2026-09-30):
+//! maskButtonAddsWhiteMaskOrRevealsTheSelection, hidingMasksUseTheSelection and
+//! maskFromSelectionLinesUpOnScaledLayers (SelectionEditTests.swift:160-219 at v1.4.5).
 use compositor_engine::*;
 use uuid::Uuid;
 
@@ -88,34 +90,51 @@ fn delete_on_a_mask_fills_the_selection_white() {
 }
 
 #[test]
-fn add_mask_with_a_selection_hides_the_selection_and_uses_it_up() {
+fn the_mask_button_adds_a_white_mask_or_reveals_the_selection() {
+    // maskButtonAddsWhiteMaskOrRevealsTheSelection (SelectionEditTests.swift:160-179 at v1.4.5).
     let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
+    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
+    assert_eq!(Command::AddMask { id: layer, revealing: true }.action_name(), "Add Reveal-All Mask");
+    assert_eq!(pixel(&e, id, 30, 20)[3], 255);
+    e.undo(id).unwrap();
+    assert!(!has_mask(&e, id, layer));
     select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
     let before = e.state(id).unwrap().undo_depth;
     run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
+    assert_eq!(Command::AddMaskFromSelection { id: layer, revealing: true }.action_name(), "Reveal Selection");
     assert_eq!(e.state(id).unwrap().undo_depth, before + 1, "one step");
     assert!(!has_selection(&e, id), "the selection is used up");
-    assert_eq!(pixel(&e, id, 30, 20)[3], 0, "selected area: black, hidden");
-    assert_eq!(pixel(&e, id, 5, 5)[3], 255, "everything else: white, visible");
+    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "selected area: white, visible");
+    assert_eq!(pixel(&e, id, 5, 5)[3], 0, "everything else: black, hidden");
     e.undo(id).unwrap();
     assert!(!has_mask(&e, id, layer) && has_selection(&e, id));
 }
 
 #[test]
-fn add_black_mask_with_a_selection_shows_only_the_selection() {
+fn hiding_masks_use_the_selection() {
+    // hidingMasksUseTheSelection (SelectionEditTests.swift:181-200 at v1.4.5): Alt-click on the
+    // button, or Add Mask (Hide All), with a selection hides it; without one it is a plain black mask.
     let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
     select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
     run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: false });
+    assert_eq!(Command::AddMaskFromSelection { id: layer, revealing: false }.action_name(), "Hide Selection");
     assert!(!has_selection(&e, id));
-    assert_eq!(pixel(&e, id, 30, 20)[3], 255, "selected area: white, visible");
-    assert_eq!(pixel(&e, id, 5, 5)[3], 0, "everything else: black, hidden");
+    assert_eq!(pixel(&e, id, 30, 20)[3], 0, "selected area: black, hidden");
+    assert_eq!(pixel(&e, id, 5, 5)[3], 255, "everything else: white, visible");
+    e.undo(id).unwrap();
+    assert!(!has_mask(&e, id, layer) && has_selection(&e, id));
+    run(&mut e, id, Command::Deselect);
+    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
+    assert_eq!(Command::AddMask { id: layer, revealing: false }.action_name(), "Add Hide-All Mask");
+    assert_eq!(pixel(&e, id, 30, 20)[3], 0);
 }
 
 #[test]
 fn a_mask_from_a_selection_lines_up_on_a_scaled_layer() {
+    // maskFromSelectionLinesUpOnScaledLayers (SelectionEditTests.swift:202-219 at v1.4.5): Hide Selection.
     let (mut e, id, layer) = session(100, 100, &solid(50, 50, [0, 0, 255, 255]));
     select(&mut e, id, 0.0, 0.0, 50.0, 50.0);
-    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
+    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: false });
     assert_eq!(e.document(id).unwrap().layer(layer).unwrap().mask.as_ref().unwrap().pixels.width, 50, "the mask uses the layer's pixel grid");
     assert_eq!(pixel(&e, id, 25, 25)[3], 0);
     assert_eq!(pixel(&e, id, 75, 75)[3], 255);
@@ -126,8 +145,8 @@ fn a_mask_from_a_selection_lines_up_on_a_scaled_layer() {
 fn a_mask_from_a_feathered_selection_takes_the_feather() {
     // Feather 4 is a Gaussian of sigma 2 across the edge at x = 20 (global constraint: feather
     // sigma = feather / 2). The pixel centred at x = 18 sits 1.5 px outside the edge, x = 21 sits
-    // 1.5 px inside; coverage there is Phi(distance / sigma) and a revealing mask paints
-    // 255 * (1 - coverage) through the clip (LayerMask.swift:245-249).
+    // 1.5 px inside; coverage there is Phi(distance / sigma) and a revealing mask is black painted
+    // white through the clip, 255 * coverage (LayerMask.swift:249-257 at v1.4.5).
     let (mut e, id, layer) = session(100, 40, &solid(100, 40, RED));
     select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
     run(&mut e, id, Command::FeatherSelection { amount: 4 });
@@ -135,8 +154,8 @@ fn a_mask_from_a_feathered_selection_takes_the_feather() {
     let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
     let (outside, inside) = (mask.bytes()[20 * 100 + 18] as f64, mask.bytes()[20 * 100 + 21] as f64);
     let sigma = 2.0;
-    let expected_outside = 255.0 * (1.0 - phi(-1.5 / sigma));
-    let expected_inside = 255.0 * (1.0 - phi(1.5 / sigma));
+    let expected_outside = 255.0 * phi(-1.5 / sigma);
+    let expected_inside = 255.0 * phi(1.5 / sigma);
     // Tolerance, not measured: the feather's formula truncates the Gaussian kernel at 3 sigma
     // (radius = ceil(sigma * 3)) and renormalizes, which redistributes under 0.3% of the mass (the
     // two-tailed mass beyond 3 sigma), and it convolves discrete pixel samples rather than
@@ -156,14 +175,18 @@ fn an_empty_selection_clears_nothing_but_still_makes_a_plain_mask() {
     run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: corners(0.0, 0.0, 100.0, 40.0), mode: SelectionMode::Subtract, antialiased: true });
     assert!(matches!(e.execute(id, Command::ClearSelectedPixels { id: layer, mask: false }), Err(CommandError::Refused(m)) if m == compositor_engine::ops::selection::EMPTY_SELECTION));
     run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: true });
-    assert!(!has_selection(&e, id) && pixel(&e, id, 30, 20)[3] == 255, "a plain white mask, the empty selection used up (LayerMask.swift:245)");
-    // Ruling I6: an opaque layer composites white regardless of whether a mask exists at all, so
-    // the pixel check above alone would pass with the mask creation dropped. Assert the mask was
-    // actually made, and that it is the plain white mask the brief describes (LayerMask.swift:245:
-    // an empty selection clips everything away, so a revealing mask is entirely white).
+    // An empty selection clips everything away (LayerMask.swift:253-257 at v1.4.5), so Reveal
+    // Selection leaves the black it starts from: the layer hidden, the empty selection used up.
+    assert!(!has_selection(&e, id) && pixel(&e, id, 30, 20)[3] == 0, "a plain black mask, the empty selection used up");
+    let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
+    assert!(mask.bytes().iter().all(|&b| b == 0), "the mask is plain black everywhere, not just at the one sampled pixel");
+    // Hide Selection starts from white, and an empty clip paints nothing on it. Ruling I6: an opaque
+    // layer composites the same with no mask at all, so the mask itself is checked.
+    e.undo(id).unwrap();
+    run(&mut e, id, Command::AddMaskFromSelection { id: layer, revealing: false });
     assert!(has_mask(&e, id, layer), "Add Mask from Selection must still create a mask");
     let mask = e.document(id).unwrap().layer(layer).unwrap().mask.clone().unwrap().pixels;
-    assert!(mask.bytes().iter().all(|&b| b == 255), "the mask is plain white everywhere, not just at the one sampled pixel");
+    assert!(mask.bytes().iter().all(|&b| b == 255), "the mask is plain white everywhere");
 }
 
 #[test]
```


The two 1.2.10-rule tests (`add_mask_with_a_selection_hides_the_selection_and_uses_it_up`, `add_black_mask_with_a_selection_shows_only_the_selection`) are replaced by ports of the Mac's `maskButtonAddsWhiteMaskOrRevealsTheSelection` and `hidingMasksUseTheSelection`; the scaled-layer test follows the Mac's (Hide Selection); the feather test's expected values flip from `255 (1 - Phi)` to `255 Phi`; the empty-selection test now expects a black mask when revealing and a white one when hiding.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test selection_masks`
Expected: five fail (the button test, the hiding test, the scaled layer, the feather and the empty selection: every tone is the other way round, and the names are the old ones). The e2e fails: the selected area is hidden (0 where 255 is expected).

- [ ] **Step 3: Implement**

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -31,9 +31,9 @@ export function addFolder(): void { const c = ctx(); if (!c) return; c.s.commitT
 // like every other action here. Without that, a pending mask move survives the mask it moves
 // and Enter later fails with "the layer has no mask"; macOS disables both menu items while a
 // transform is pending (canEditLayers requires transformEdit == nil).
-// With a selection, Add Mask paints the opposite tone through it and uses it up, one undo step
-// ("Add Mask from Selection", LayerMask.swift:228-260); every Add Mask entry point on the Mac goes
-// through that one `addMask`.
+// With a selection, Add Mask reveals it ("Reveal Selection") or, when `revealing` is false, hides it
+// ("Hide Selection"), and uses it up, one undo step (LayerMask.swift:237-267 at v1.4.5); every Add
+// Mask entry point on the Mac goes through that one `addMask`.
 export function addMaskToActive(revealing: boolean): void {
   const c = ctx(); if (!c?.active || c.active.hasMask) return; c.s.commitTransform();
   const ok = c.s.run(c.doc.selection ? { type: "AddMaskFromSelection", id: c.active.id, revealing } : { type: "AddMask", id: c.active.id, revealing });
```

```diff
--- a/app/src/panels/LayersList.tsx
+++ b/app/src/panels/LayersList.tsx
@@ -117,7 +117,9 @@ export function LayersList() {
       <div className="layers-footer">
         <button data-testid="layer-add" title="New layer" onClick={() => s.run({ type: "AddBlankLayer" })}>+</button>
         <button data-testid="layer-add-folder" title="New folder" onClick={addFolder}>[ ]</button>
-        <button data-testid="layer-add-mask" title="Add mask" onClick={() => addMaskToActive(true)}>M</button>
+        {/* LayerMaskMenu.swift:9-14 at v1.4.5: reveals, Alt-click (the Mac's Option-click) the opposite. */}
+        <button data-testid="layer-add-mask" title={doc.selection ? "Add layer mask revealing the selection (Alt-click to hide it)" : "Add layer mask (Alt-click for a black mask)"}
+          onClick={(e) => addMaskToActive(!e.altKey)}>M</button>
         <button data-testid="layer-delete" title="Delete" onClick={deleteSelected}>x</button>
       </div>
       {menu && <ContextMenu at={menu} items={menuItems()} onClose={() => setMenu(null)} />}
```

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -65,7 +65,8 @@ pub enum Command {
     LoadMaskSelection { #[serde(with = "ids::upper")] id: Uuid, mode: SelectionMode, antialiased: bool },
     /// Delete with a selection: the selected pixels cleared, or the mask filled white there (`mask`).
     ClearSelectedPixels { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool },
-    /// Add Mask with a selection: `revealing` white with the selection black, or the reverse.
+    /// Add Mask with a selection: `revealing` shows only the selection (Reveal Selection), otherwise
+    /// it hides it (Hide Selection), as Compositor 1.4.5 does.
     AddMaskFromSelection { #[serde(with = "ids::upper")] id: Uuid, revealing: bool },
     // Colour and fills (Phase 4b-1).
     /// Fill the selection (or the whole layer) with `color`; on the mask, its first channel is grey.
@@ -107,7 +108,9 @@ impl Command {
             Command::DistortLayer { .. } => "Distort",
             Command::DistortLayers { .. } => "Distort Layers",
             Command::SetMaskPlacement { .. } => "Transform Layer Mask",
-            Command::AddMask { .. } => "Add Mask",
+            // LayerMask.swift:261 and :274 at v1.4.5.
+            Command::AddMask { revealing: true, .. } => "Add Reveal-All Mask",
+            Command::AddMask { revealing: false, .. } => "Add Hide-All Mask",
             Command::DeleteMask { .. } => "Delete Layer Mask",
             Command::SetMaskEnabled { .. } => "Enable Layer Mask",
             Command::SetMaskLinked { .. } => "Link Layer Mask",
@@ -140,7 +143,8 @@ impl Command {
             // SelectionEdits.swift:46 and :55, LayerMask.swift:254.
             Command::ClearSelectedPixels { mask: false, .. } => "Clear",
             Command::ClearSelectedPixels { mask: true, .. } => "Fill Mask",
-            Command::AddMaskFromSelection { .. } => "Add Mask from Selection",
+            Command::AddMaskFromSelection { revealing: true, .. } => "Reveal Selection",
+            Command::AddMaskFromSelection { revealing: false, .. } => "Hide Selection",
             // SelectionEdits.swift:34, Gradient.swift:98.
             Command::Fill { mask: false, .. } => "Fill",
             Command::Fill { mask: true, .. } => "Fill Mask",
```

```diff
--- a/engine/src/ops/selection.rs
+++ b/engine/src/ops/selection.rs
@@ -202,11 +202,13 @@ pub fn clear_selected(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask
     Ok(())
 }
 
-/// Add Mask with a selection (`addMask(revealing:)`, LayerMask.swift:233-260): a mask on the
-/// layer's pixel grid (its rectangle when it has none), white when `revealing` and black
-/// otherwise, painted the opposite tone through the selection's clip -- its coverage with the
-/// feather, cut to the canvas (`selection.clip(canvas:)`, :245-249). The selection is used up in
-/// the same step. An empty selection clips everything away: a plain mask, the selection used up.
+/// Add Mask with a selection (`addMask(revealing:)`, LayerMask.swift:237-267 at v1.4.5): a mask on
+/// the layer's pixel grid (its rectangle when it has none). Reveal Selection fills it black and
+/// paints white through the selection's clip -- its coverage with the feather, cut to the canvas
+/// (`selection.clip(canvas:)`, :253-257) -- so only the selection shows; Hide Selection (Alt-click)
+/// is the reverse. Compositor 1.2.10 had the tones the other way round. The selection is used up in
+/// the same step. An empty selection clips everything away: a plain black mask when revealing, a
+/// plain white one when hiding.
 pub fn add_mask_from_selection(doc: &mut Document, clips: &SelectionClips, id: Uuid, revealing: bool) -> Result<(), CommandError> {
     if doc.selection.is_none() { return Err(refused(NO_SELECTION)); }
     let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
@@ -218,7 +220,7 @@ pub fn add_mask_from_selection(doc: &mut Document, clips: &SelectionClips, id: U
     let (w, h) = (w as u32, h as u32);
     // The clip itself, empty or not (`clip(canvas:)`): an empty selection clips everything away.
     let Some(coverage) = selection_coverage_with(doc, clips, &layer.transform.pixel_to_document(w, h), w, h) else { return Err(refused(NO_SELECTION)) };
-    let pixels = if revealing { GrayRaster::from_bytes(w, h, coverage.bytes().iter().map(|c| 255 - c).collect()) } else { coverage };
+    let pixels = if revealing { coverage } else { GrayRaster::from_bytes(w, h, coverage.bytes().iter().map(|c| 255 - c).collect()) };
     doc.layer_mut(id).unwrap().set_mask(Some(Mask { pixels, enabled: true, placement: None, linked: None }));
     doc.selection = None;
     Ok(())
```


- [ ] **Step 4: Run the whole set, then the timings**

`cargo test -p compositor-engine`: 571 passed, 10 ignored. `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 165 passed, 21 skipped.

Timings (`-g "Add Mask with a selection"`, release), two runs: the step 162-212 ms at 24 MP and 506-545 ms at 100 MP, the frame after 19-20 and 59-77 ms; cold 253-322 / 510-557 and 58-60 / 255-256 (logged). Budgets 400 / 1000 and 150 / 400. The step's cost is Phase 4a's (the clip rasterized at the layer's size, on the UI thread); swapping the tones adds nothing measurable (ruling OQ9).

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Put 1.2.10's tones back (`if !revealing { coverage } else { 255 - coverage }`): five `selection_masks` tests fail (measured).
2. Name Hide Selection "Reveal Selection": `hiding_masks_use_the_selection` fails (measured).
3. The footer button always reveals (`addMaskToActive(true)`): the selection e2e fails at the Alt-click, 255 where 0 is expected (measured).

- [ ] **Step 6: Commit**

```
git commit -m "fix: Add Mask with a selection reveals it and Alt-click hides it, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/actions/layers.ts app/src/panels/LayersList.tsx app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/selection.spec.ts engine/src/command.rs engine/src/ops/selection.rs engine/tests/selection_masks.rs
```


---

### Task 10: Inverse of everything leaves no selection

Select All then Inverse left an empty selection, which quietly refuses every edit (Fill, Delete, a brush) until the user deselects. 1.4.5 leaves no selection instead, as Photoshop does (Selection.swift:354-362; ruling OQ10).

**Files:**
- Modify: `engine/src/ops/selection.rs` (`invert_selection`)
- Modify (tests): `engine/tests/selection_commands.rs`, `app/tests/e2e/selection.spec.ts`, `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** Selection.swift:354-362; SelectionTests.swift:126-134 (`inverseOfEverythingDeselects`, ported).

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -270,4 +270,57 @@ test("Add Mask with a selection at 24 and 100 MP: Reveal Selection and Hide Sele
     expect(out[`${label} ${name}: step, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 400 : 1000);
     expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(label === "24 MP" ? 150 : 400);
   }
+});
+test("Inverse at 24 and 100 MP: of Select All (no selection left) and of a marquee, the step and the frame after", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      s().openDocument(doc);
+      await settle(); frame();
+      const marquee = { type: "SelectShape", kind: "Rectangle", points: [[w * 0.2, h * 0.2], [w * 0.8, h * 0.2], [w * 0.8, h * 0.8], [w * 0.2, h * 0.8]], mode: "Replace", antialiased: true };
+      // Ctrl+Shift+I as the key sends it (useShortcuts: `select-inverse`), the store's run and the frame after.
+      const once = async (select: unknown) => {
+        s().run(select); frame(); await settle();
+        const t0 = performance.now();
+        window.dispatchEvent(new KeyboardEvent("keydown", { key: "I", ctrlKey: true, shiftKey: true }));
+        const step = performance.now() - t0;
+        const after = frame();
+        const none = api.engine.state(doc).selection === null ? 1 : 0;
+        await settle();
+        return [Math.round(step), Math.round(after), none];
+      };
+      [result["cold step ms"], result["cold frame after ms"]] = await once({ type: "SelectAll" });
+      for (const [name, select] of [["of Select All", { type: "SelectAll" }], ["of a marquee", marquee]] as [string, unknown][]) {
+        let step = 0, after = 0, none = 0;
+        for (let i = 0; i < 3; i++) { const [a, b, n] = await once(select); step = Math.max(step, a); after = Math.max(after, b); none += n; }
+        result[`${name}: step, worst of 3 ms`] = step;
+        result[`${name}: frame after, worst of 3 ms`] = after;
+        result[`${name}: times no selection was left`] = none;
+      }
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`Inverse (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`Inverse renderer: ${renderer}`);
+  for (const [label] of SIZES) {
+    expect(out[`${label} of Select All: times no selection was left`], "the inverse of everything is no selection").toBe(3);
+    expect(out[`${label} of a marquee: times no selection was left`]).toBe(0);
+    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 1 ms and frames 4 ms at both sizes (the
+    // outline is geometry, not pixels). A step inside a frame and 4b-1's frame budget.
+    for (const name of ["of Select All", "of a marquee"]) {
+      expect(out[`${label} ${name}: step, worst of 3 ms`]).toBeLessThan(16);
+      expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(33);
+    }
+  }
 });
\ No newline at end of file
```

```diff
--- a/app/tests/e2e/selection.spec.ts
+++ b/app/tests/e2e/selection.spec.ts
@@ -223,6 +223,22 @@ test("Add Mask with a selection reveals it (Alt-click hides it) and uses it up;
   expect((await pixel(page, [40, 40]))[3]).toBe(255);
 });
 
+test("Inverse of Select All leaves nothing selected, so Fill reaches the whole layer again; undo brings the selection back", async ({ page }) => {
+  // Selection.swift:354-362 at v1.4.5 (Phase 4.5).
+  await setup(page);
+  await page.keyboard.press("Control+a");
+  const depth = await undoDepth(page);
+  await page.keyboard.press("Control+Shift+i");
+  expect(await selection(page)).toBeNull();
+  expect(await undoDepth(page)).toBe(depth + 1);
+  // With an empty selection left instead, Fill would be refused; with none it fills everything.
+  await page.keyboard.press("Alt+Backspace");
+  await expect.poll(() => pixel(page, [60, 40])).toEqual([0, 0, 0, 255]);
+  await page.keyboard.press("Control+z");
+  await page.keyboard.press("Control+z");
+  expect((await selection(page)).bounds).toEqual({ x: 0, y: 0, width: 64, height: 48 });
+});
+
 test("the Select menu: All, Inverse, Expand and Feather with their amount, the layer's pixels, and Ctrl-click on a thumbnail", async ({ page }) => {
   await setup(page);
   await clickMenu(page, "Select", "select-all");
```

```diff
--- a/engine/tests/selection_commands.rs
+++ b/engine/tests/selection_commands.rs
@@ -112,6 +112,30 @@ fn select_all_and_inverse() {
     assert!(selection(&e, id).is_none() && depth(&e, id) == before);
 }
 
+#[test]
+fn the_inverse_of_everything_deselects() {
+    // SelectionTests.inverseOfEverythingDeselects (SelectionTests.swift:126-134 at v1.4.5): Select All
+    // then Inverse leaves nothing selected, as one step, and undo brings the whole canvas back.
+    let (mut e, id) = session(100, 80);
+    run(&mut e, id, Command::SelectAll);
+    let before = depth(&e, id);
+    run(&mut e, id, Command::InvertSelection);
+    assert!(selection(&e, id).is_none() && e.state(id).unwrap().selection.is_none(), "no selection, not an empty one");
+    assert_eq!(depth(&e, id), before + 1, "one step");
+    e.undo(id).unwrap();
+    let back = selection(&e, id).expect("the selection is back");
+    assert!(!back.is_empty());
+    assert_eq!(back.bounds(), Some(rect(0.0, 0.0, 100.0, 80.0)));
+    // A selection past the canvas covers all of it once cut to it, so its inverse is nothing too.
+    lasso(&mut e, id, square(-10.0, -10.0, 200.0), SelectionMode::Replace);
+    run(&mut e, id, Command::InvertSelection);
+    assert!(selection(&e, id).is_none());
+    // Anything short of the whole canvas still leaves the rest selected.
+    lasso(&mut e, id, vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 79.0), p(0.0, 79.0)], SelectionMode::Replace);
+    run(&mut e, id, Command::InvertSelection);
+    assert_eq!(selection(&e, id).unwrap().bounds(), Some(rect(0.0, 79.0, 100.0, 1.0)));
+}
+
 #[test]
 fn dragging_moves_the_outline_in_whole_pixels_as_one_undo_step() {
     let (mut e, id) = session(100, 100);
```


- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test selection_commands`
Expected: `the_inverse_of_everything_deselects` fails ("no selection, not an empty one"). The e2e fails at `toBeNull` (an empty selection is left) and Fill is refused.

- [ ] **Step 3: Implement**

```diff
--- a/engine/src/ops/selection.rs
+++ b/engine/src/ops/selection.rs
@@ -71,10 +71,13 @@ pub fn select_all(doc: &mut Document) { doc.selection = Some(Selection::new(canv
 /// Deselect: no selection.
 pub fn deselect(doc: &mut Document) { doc.selection = None; }
 
-/// Inverse: the canvas minus the selection, its flags kept; nothing without one.
+/// Inverse: the canvas minus the selection, its flags kept; nothing without one. The inverse of
+/// everything is no selection at all, as in Photoshop, not an empty one that stops every edit
+/// (`invertSelection`, Selection.swift:354-362 at v1.4.5).
 pub fn invert_selection(doc: &mut Document) {
     let Some(current) = doc.selection.clone() else { return };
-    doc.selection = Some(Selection::new(g::combine(&canvas(doc), &current.contours, Boolean::Difference), current.antialiased, current.feather));
+    let inverse = Selection::new(g::combine(&canvas(doc), &current.contours, Boolean::Difference), current.antialiased, current.feather);
+    doc.selection = if inverse.is_empty() { None } else { Some(inverse) };
 }
 
 /// The outline moved by whole pixels (`moveSelection(by:)`: offsets rounded), not cut to the canvas,
```


- [ ] **Step 4: Run the whole set, then the timings**

`cargo test -p compositor-engine`: 572 passed, 10 ignored. `pnpm wasm:dev`; `pnpm test` 244; `pnpm build` clean; `pnpm e2e` 166 passed, 22 skipped.

Timings (`-g "Inverse at"`, release), two runs: the step (Ctrl+Shift+I through `useShortcuts`, the store's run) 1 ms and the frame after 4 ms at both sizes, whether the inverse of Select All (no selection left, three times of three) or of a marquee (a selection left). Budgets 16 and 33.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Keep the empty selection (`doc.selection = Some(inverse)`): `the_inverse_of_everything_deselects` and the Inverse e2e fail (measured).
2. Leave the selection as it was when the inverse is empty (`if inverse.is_empty() { return; }`): `the_inverse_of_everything_deselects` fails, and the undo depth does not grow (measured).

- [ ] **Step 6: Commit**

```
git commit -m "fix: Inverse of a selection covering the whole canvas leaves no selection, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/selection.spec.ts engine/src/ops/selection.rs engine/tests/selection_commands.rs
```


---

### Task 11: Ungroup Layers

1.4.5 adds Ungroup Layers (Shift+Cmd+G, the Layer menu, and a folder's context menu): the folder's direct children take its place among its siblings, in the order they had inside it, and the folder goes with its own opacity, mask and effects; the children are selected, the first one active; a clipped layer no longer next to its base stops clipping (ruling OQ11).

**Files:**
- Modify: `engine/src/command.rs` (`Command::UngroupLayers { id }`, "Ungroup Layers"), `engine/src/engine.rs` (dispatch), `engine/src/ops/hierarchy.rs` (`ungroup_layers`), `app/src/engine/types.ts`, `app/src/actions/layers.ts` (`canUngroupActive`, `ungroupActive`), `app/src/shortcuts/keymap.ts` and `app/src/shortcuts/useShortcuts.ts` (`ungroup`, Shift+Ctrl+G), `app/src/panels/MenuBar.tsx` (Layer > Ungroup Layers), `app/src/panels/LayersList.tsx` (the folder's context menu)
- Create: `engine/tests/ungroup.rs`, `app/tests/e2e/ungroup.spec.ts`
- Modify (tests): `app/tests/unit/keymap.test.ts`, `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** LayerGroups.swift:214-239 (`canUngroupLayers`, `ungroupLayers`), LiveLayerMask.swift:224-237 (`releaseDetachedClipping`, the port's `release_detached_clipping` already), CompositorApp.swift:312-313 (the menu item and Shift+Cmd+G), KeyboardShortcuts.swift:93, NativeLayerList.swift:159-165 (the folder's context menu); GroupTests.swift:110-189 (ported).

**Interfaces this task produces:** `Command::UngroupLayers { id }` (JSON `{ "type": "UngroupLayers", "id": ... }`); `ops::hierarchy::ungroup_layers(doc, id) -> Result<Vec<Uuid>, CommandError>` (the children, bottom first; the first becomes active); `ungroupActive()` / `canUngroupActive()`; the `ungroup` action id.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -323,4 +323,64 @@ test("Inverse at 24 and 100 MP: of Select All (no selection left) and of a marqu
       expect(out[`${label} ${name}: frame after, worst of 3 ms`]).toBeLessThan(33);
     }
   }
-});
\ No newline at end of file
+});
+test("Ungroup Layers at 24 and 100 MP: the step and the frame after", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const base = api.engine.state(doc).layers[0].id;
+      // A folder at half opacity holding a full-size copy in Screen and a copy clipped to it:
+      // ungrouping drops the folder's look, so the frame after composites the children anew.
+      api.engine.execute(doc, { type: "DuplicateLayer", id: base });
+      const a = api.engine.state(doc).activeLayerId;
+      api.engine.execute(doc, { type: "SetLayerBlendMode", id: a, mode: "Screen" });
+      api.engine.execute(doc, { type: "DuplicateLayer", id: a });
+      const b = api.engine.state(doc).activeLayerId;
+      api.engine.execute(doc, { type: "SetLayerBlendMode", id: b, mode: "Normal" });
+      api.engine.execute(doc, { type: "SetLayerOpacity", id: b, opacity: 0.5 });
+      api.engine.execute(doc, { type: "ToggleClipping", id: b });
+      s().openDocument(doc);
+      await settle(); frame();
+      const once = async () => {
+        api.engine.execute(doc, { type: "GroupLayers", ids: [a, b] });
+        const folder = api.engine.state(doc).activeLayerId;
+        api.engine.execute(doc, { type: "SetLayerOpacity", id: folder, opacity: 0.5 });
+        s().refresh(doc); s().selectLayers([folder], folder); frame(); await settle();
+        const t0 = performance.now();
+        window.dispatchEvent(new KeyboardEvent("keydown", { key: "G", ctrlKey: true, shiftKey: true }));
+        const left = api.engine.state(doc).layers.some((l: any) => l.id === folder) ? 1 : 0;
+        const step = performance.now() - t0;
+        const after = frame();
+        await settle();
+        return [Math.round(step), Math.round(after), left];
+      };
+      [result["cold step ms"], result["cold frame after ms"]] = await once();
+      let step = 0, after = 0, left = 0;
+      for (let i = 0; i < 3; i++) { const [x, y, f] = await once(); step = Math.max(step, x); after = Math.max(after, y); left += f; }
+      result["folders left after Shift+Ctrl+G"] = left;
+      result["step, worst of 3 ms"] = step;
+      result["frame after, worst of 3 ms"] = after;
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`Ungroup (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`Ungroup renderer: ${renderer}`);
+  for (const [label] of SIZES) {
+    expect(out[`${label} folders left after Shift+Ctrl+G`], "every timed step ungrouped").toBe(0);
+    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): steps 2-5 ms and frames after 10-13 ms at both sizes
+    // (a structure change: no pixels move). A step inside a frame and 4b-1's frame budget.
+    expect(out[`${label} step, worst of 3 ms`]).toBeLessThan(16);
+    expect(out[`${label} frame after, worst of 3 ms`]).toBeLessThan(33);
+  }
+});
```

Create `app/tests/e2e/ungroup.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Ungroup Layers (Compositor 1.4.5, LayerGroups.swift:214-239): the Layer menu and Shift+Ctrl+G
// (CompositorApp.swift:312-313), and a folder's context menu (NativeLayerList.swift:159-165).

/** A 64 x 48 document: "Below", a folder holding "A" and "B" at half opacity, and "Above". */
async function setup(page: Page): Promise<{ below: string; folder: string; a: string; b: string; above: string }> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 8, height: 8, color: "#ff0000" });
  return page.evaluate(async (red) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(red), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    const add = (name: string) => { api.engine.importImage(doc, png, name, { x: 32, y: 24 }); return api.engine.state(doc).activeLayerId as string; };
    const below = add("Below"), a = add("A"), b = add("B"), above = add("Above");
    api.engine.execute(doc, { type: "GroupLayers", ids: [a, b] });
    const folder = api.engine.state(doc).activeLayerId as string;
    api.engine.execute(doc, { type: "SetLayerOpacity", id: folder, opacity: 0.5 });
    api.store.getState().openDocument(doc);
    return { below, folder, a, b, above };
  }, red);
}
const doc = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { d: s.documents[s.activeId], selected: s.selectedLayerIds as string[] }; });
const row = (page: Page, id: string) => page.locator(`[data-testid="layer-row"][data-layer-id="${id}"]`);

test("a folder's context menu ungroups it: its layers take its place, selected, and the folder goes as one step", async ({ page }) => {
  const ids = await setup(page);
  // A plain layer's menu has no Ungroup.
  await row(page, ids.above).click({ button: "right" });
  await expect(page.getByTestId("ctx-ungroup")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await row(page, ids.folder).click({ button: "right" });
  const depth = (await doc(page)).d.undoDepth;
  await page.getByTestId("ctx-ungroup").click();
  const { d, selected } = await doc(page);
  expect(d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
  expect(d.layers.every((l: any) => l.parentId === null)).toBe(true);
  expect(d.layers.map((l: any) => l.opacity)).toEqual([1, 1, 1, 1]);
  expect(selected).toEqual([ids.a, ids.b]);
  expect(d.activeLayerId).toBe(ids.a);
  expect(d.undoDepth).toBe(depth + 1);
});

test("Shift+Ctrl+G and Layer > Ungroup Layers ungroup the active folder, and only a folder", async ({ page }) => {
  const ids = await setup(page);
  await row(page, ids.above).click();
  await page.getByRole("button", { name: "Layer", exact: true }).click();
  await expect(page.getByTestId("menu-layer-ungroup")).toBeDisabled();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Shift+Control+g");
  expect((await doc(page)).d.layers.length).toBe(5);
  await row(page, ids.folder).click();
  await page.keyboard.press("Shift+Control+g");
  expect((await doc(page)).d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
  await page.keyboard.press("Control+z");
  const back = (await doc(page)).d;
  expect(back.layers.find((l: any) => l.id === ids.folder)?.isGroup).toBe(true);
  expect(back.layers.filter((l: any) => l.parentId === ids.folder).map((l: any) => l.id)).toEqual([ids.a, ids.b]);
  await row(page, ids.folder).click();
  await clickMenu(page, "Layer", "layer-ungroup");
  expect((await doc(page)).d.layers.map((l: any) => l.id)).toEqual([ids.below, ids.a, ids.b, ids.above]);
});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -38,6 +38,12 @@ describe("keymap", () => {
     expect(matchShortcut(ev("ArrowDown", { shiftKey: true }))).toBe("nudge-down");
   });
 
+  it("groups with Ctrl+G and ungroups with Shift+Ctrl+G (KeyboardShortcuts.swift:92-93 at v1.4.5)", () => {
+    expect(matchShortcut(ev("g", { ctrlKey: true }))).toBe("group");
+    expect(matchShortcut(ev("G", { ctrlKey: true, shiftKey: true }))).toBe("ungroup");
+    expect(matchShortcut(ev("g", { ctrlKey: true, altKey: true }))).toBe("clip");
+  });
+
   it("maps layer shortcuts", () => {
     expect(matchShortcut(ev("j", { ctrlKey: true }))).toBe("duplicate");
     expect(matchShortcut(ev("g", { ctrlKey: true }))).toBe("group");
```

Create `engine/tests/ungroup.rs`:

```rust
//! Ungroup Layers (Phase 4.5), ported from Compositor 1.4.5's GroupTests
//! (CompositorTests/GroupTests.swift:110-189 at v1.4.5): ungroupLayersRestoresChildrenAtTheFoldersSpotAndUndoes,
//! ungroupPreservesClippingBetweenTwoOfAFoldersOwnChildren and ungroupingReleasesClippingThatNoLongerMakesSense,
//! with their steps and expectations.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { let what = format!("{c:?}"); e.execute(id, c).unwrap_or_else(|err| panic!("{what}: {err}")); }
fn active(e: &Engine, id: Uuid) -> Uuid { e.state(id).unwrap().active_layer_id.unwrap() }
fn blank(e: &mut Engine, id: Uuid) -> Uuid { run(e, id, Command::AddBlankLayer); active(e, id) }
/// A layer with pixels (a clip needs a base with pixels; the Mac's blank layers have them, this port's
/// do not), placed above the active layer as `AddBlankLayer` places one.
fn painted(e: &mut Engine, id: Uuid) -> Uuid {
    let png = encode_png(&Raster::from_premultiplied(4, 4, [255, 0, 0, 255].repeat(16)), 72.0).unwrap();
    e.import_image(Some(id), &png, "Painted", Some(Point { x: 50.0, y: 50.0 })).unwrap();
    active(e, id)
}
fn ids(e: &Engine, id: Uuid) -> Vec<Uuid> { e.state(id).unwrap().layers.iter().map(|l| l.id).collect() }
fn parent(e: &Engine, id: Uuid, layer: Uuid) -> Option<Uuid> { e.state(id).unwrap().layers.iter().find(|l| l.id == layer).unwrap().parent_id }
fn clip_source(e: &Engine, id: Uuid, layer: Uuid) -> Option<Uuid> { e.document(id).unwrap().layer(layer).unwrap().mask_source_id }

#[test]
fn ungroup_restores_the_children_at_the_folders_spot_and_undoes() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let below = blank(&mut e, id);
    assert!(e.execute(id, Command::UngroupLayers { id: below }).is_err(), "a plain layer has nothing to unwrap");
    run(&mut e, id, Command::AddGroup);
    let group = active(&e, id);
    let child_a = blank(&mut e, id);
    let child_b = blank(&mut e, id);
    run(&mut e, id, Command::SetActiveLayer { id: None });
    let above = blank(&mut e, id);
    let parents: Vec<Option<Uuid>> = e.state(id).unwrap().layers.iter().map(|l| l.parent_id).collect();
    assert_eq!(parents, vec![None, None, Some(group), Some(group), None]);

    run(&mut e, id, Command::SetActiveLayer { id: Some(group) });
    let before = e.state(id).unwrap().undo_depth;
    run(&mut e, id, Command::UngroupLayers { id: group });
    assert_eq!(Command::UngroupLayers { id: group }.action_name(), "Ungroup Layers");
    let order = ids(&e, id);
    assert!(!order.contains(&group), "the folder itself goes");
    assert_eq!((parent(&e, id, child_a), parent(&e, id, child_b)), (None, None));
    // Spliced in where the folder sat: below stays below both children, above stays above both.
    assert_eq!(order, vec![below, child_a, child_b, above]);
    assert_eq!(active(&e, id), child_a, "the first child is the active layer (selectLayers(childIDs, primary: children.first))");
    assert_eq!(e.state(id).unwrap().undo_depth, before + 1);

    e.undo(id).unwrap();
    assert_eq!(parent(&e, id, child_a), Some(group));
    assert!(e.state(id).unwrap().layers.iter().find(|l| l.id == group).unwrap().is_group);
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().layers.len(), 4);
}

#[test]
fn ungroup_keeps_a_clip_between_two_of_the_folders_own_children() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let base = painted(&mut e, id);
    let clipped = painted(&mut e, id);
    run(&mut e, id, Command::LinkMask { source: base, target: clipped });
    run(&mut e, id, Command::GroupLayers { ids: vec![base, clipped] });
    let group = active(&e, id);
    run(&mut e, id, Command::UngroupLayers { id: group });
    // Spliced in together at the folder's old spot, so the pair stays adjacent.
    assert_eq!(clip_source(&e, id, clipped), Some(base));
}

#[test]
fn ungroup_releases_a_clip_that_no_longer_makes_sense() {
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    let outside_base = painted(&mut e, id);
    let between = painted(&mut e, id);
    let child_source = painted(&mut e, id);
    // A clip can be set up across a folder boundary -- `LinkMask` does not forbid it -- though the two
    // layers are not really adjacent once the folder is in between.
    run(&mut e, id, Command::LinkMask { source: outside_base, target: child_source });
    run(&mut e, id, Command::GroupLayers { ids: vec![child_source] });
    let group = active(&e, id);
    assert_eq!(ids(&e, id), vec![outside_base, between, group, child_source]);
    run(&mut e, id, Command::UngroupLayers { id: group });
    // Ungrouped, child_source lands right after `between`, no longer next to its base, so the clip goes.
    assert_eq!(ids(&e, id), vec![outside_base, between, child_source]);
    assert_eq!(clip_source(&e, id, child_source), None);
}

#[test]
fn ungroup_drops_the_folders_own_look_and_keeps_nested_folders_whole() {
    // LayerGroups.swift:217-219: the folder's opacity, mask and effects go with it (a folder's blend mode
    // is always Normal, pass-through); a folder inside it is one of its children and keeps its contents.
    let mut e = Engine::new();
    let id = e.new_document(100, 100, false).unwrap();
    run(&mut e, id, Command::AddGroup);
    let outer = active(&e, id);
    run(&mut e, id, Command::AddGroup);
    let inner = active(&e, id);
    let deep = blank(&mut e, id);
    run(&mut e, id, Command::SetLayerOpacity { id: outer, opacity: 0.25 });
    run(&mut e, id, Command::UngroupLayers { id: outer });
    assert!(e.document(id).unwrap().layer(outer).is_none());
    assert_eq!((parent(&e, id, inner), parent(&e, id, deep)), (None, Some(inner)));
    let inner_layer = e.document(id).unwrap().layer(inner).unwrap().clone();
    assert_eq!(inner_layer.opacity, 1.0, "the children keep their own look, not the folder's 0.25");
}
```


The Mac's tests use blank layers as clip bases; the port's blank layers have no pixels and cannot be clipped to, so `painted` imports a 4 x 4 red layer where the Mac's test adds a blank one. The folder's own look is checked with its opacity: a folder's blend mode is always Normal here (pass-through), so the Mac's "blend mode discarded" has nothing to test.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p compositor-engine --test ungroup`
Expected: does not compile (no `Command::UngroupLayers`). `pnpm test -- keymap` fails (`matchShortcut` gives null for Shift+Ctrl+G); the e2e fails (no `ctx-ungroup`).

- [ ] **Step 3: Implement**

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -24,6 +24,20 @@ export function deleteSelected(): void {
 }
 export function duplicateSelected(): void { const c = ctx(); if (!c?.active || c.active.isGroup) return; c.s.commitTransform(); c.s.run({ type: "DuplicateLayer", id: c.active.id }); }
 export function groupSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "GroupLayers", ids: c.selected }); }
+/** Ungroup Layers needs a folder as the active layer (`canUngroupLayers`, LayerGroups.swift:215 at v1.4.5). */
+export function canUngroupActive(): boolean { const c = ctx(); return !!c?.active?.isGroup && !c.s.panelOwnsDocument(); }
+/** The active folder's children take its place and the folder goes; the children are selected, the first
+ * one active (`ungroupLayers`, LayerGroups.swift:220-239 at v1.4.5), and the folder's collapsed state goes. */
+export function ungroupActive(): void {
+  const c = ctx(); if (!c || !canUngroupActive()) return;
+  const folder = c.active!.id;
+  const children = c.doc.layers.filter((l) => l.parentId === folder).map((l) => l.id);
+  c.s.commitTransform();
+  if (!c.s.run({ type: "UngroupLayers", id: folder })) return;
+  const st = useEditor.getState();
+  st.selectLayers(children, children[0] ?? null);
+  if (st.activeId && (st.collapsed[st.activeId] ?? []).includes(folder)) st.toggleCollapsed(folder);
+}
 export function mergeSelected(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); if (c.engine.mergeAction(c.doc.id, c.selected)) c.s.run({ type: "MergeLayers", ids: c.selected }); }
 export function mergeTitle(): string { const c = ctx(); return (c && c.engine.mergeAction(c.doc.id, c.selected)) || "Merge Down"; }
 export function addFolder(): void { const c = ctx(); if (!c) return; c.s.commitTransform(); c.s.run({ type: "AddGroup" }); }
```

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -176,6 +176,7 @@ export type Command =
   | { type: "SetLayerBlendMode"; id: string; mode: BlendMode }
   | { type: "AddGroup" }
   | { type: "GroupLayers"; ids: string[] }
+  | { type: "UngroupLayers"; id: string }
   | { type: "PlaceLayer"; id: string; parent: string | null; above: string | null; atBottom: boolean }
   | { type: "MoveLayerBy"; id: string; offset: number }
   | { type: "DuplicateLayer"; id: string }
```

```diff
--- a/app/src/panels/LayersList.tsx
+++ b/app/src/panels/LayersList.tsx
@@ -2,8 +2,9 @@ import { useState, type DragEvent } from "react";
 import { useEditor } from "../state/store";
 import type { LayerState } from "../engine/types";
 import { layerRows, dropTarget, type Row } from "./layer-rows";
+import { activeLayer } from "../state/selection";
 import { ContextMenu, type MenuItem } from "./ContextMenu";
-import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, editAdjustmentLayer, flipSelected, groupSelected, loadSelection, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive } from "../actions/layers";
+import { addFolder, addMaskToActive, canClipActive, deleteSelected, duplicateSelected, editAdjustmentLayer, flipSelected, groupSelected, loadSelection, mergeSelected, mergeTitle, placeDropped, toggleClippingOfActive, ungroupActive } from "../actions/layers";
 
 /** Ctrl-click on a thumbnail loads it as a selection, Ctrl-Shift adds and Ctrl-Alt subtracts
  * (`loadMode`, NativeLayerList.swift:1007-1019, :1235-1238); Ctrl-click elsewhere in a row still
@@ -60,6 +61,8 @@ export function LayersList() {
   const menuItems = (): MenuItem[] => [
     { id: "duplicate", label: "Duplicate Layer", run: duplicateSelected },
     { id: "group", label: "Group Layers", run: groupSelected },
+    // A folder right-clicked can be ungrouped (NativeLayerList.swift:159-165 at v1.4.5).
+    ...(activeLayer(doc)?.isGroup ? [{ id: "ungroup", label: "Ungroup Layers", run: ungroupActive }] : []),
     { id: "merge", label: mergeTitle(), run: mergeSelected },
     "separator",
     { id: "mask-reveal", label: "Add Mask (Reveal All)", run: () => addMaskToActive(true) },
```

```diff
--- a/app/src/panels/MenuBar.tsx
+++ b/app/src/panels/MenuBar.tsx
@@ -6,7 +6,7 @@ import { activeLayer } from "../state/selection";
 import {
   addAdjustmentLayer, addMaskToActive, blurMaskOfActive, canClipActive, canEditAdjustment, canInvert, canMoveActiveBy, canPaint,
   deleteMaskOfActive, deleteSelected, editAdjustmentLayer, fillMaskOfActive, flipSelected, invertMaskOfActive,
-  loadSelection, mergeTitle, toggleMaskEnabled, toggleMaskLink,
+  loadSelection, mergeTitle, toggleMaskEnabled, toggleMaskLink, canUngroupActive,
 } from "../actions/layers";
 import { ADJUSTMENT_KINDS, isEditableKind } from "../engine/types";
 
@@ -60,6 +60,7 @@ export function MenuBar() {
       { id: "layer-new-folder", label: "New Folder", run: () => runAction("new-folder"), enabled: editable },
       { id: "layer-duplicate", label: "Duplicate Layer", run: () => runAction("duplicate"), enabled: editable },
       { id: "layer-group", label: "Group Layers", run: () => runAction("group"), enabled: editable },
+      { id: "layer-ungroup", label: "Ungroup Layers", run: () => runAction("ungroup"), enabled: editable && canUngroupActive() },
       { id: "layer-merge", label: mergeTitle(), run: () => runAction("merge"), enabled: editable },
       "separator",
       ...ADJUSTMENT_KINDS.map((kind) => ({
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -1,7 +1,7 @@
 export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "export-jpeg" | "close" | "undo" | "redo" | "new-layer"
   | "canvas-size" | "image-size" | "zoom-in" | "zoom-out" | "fit" | "actual" | "tool-move" | "tool-hand" | "tool-zoom" | "tool-crop" | "apply" | "cancel"
   | "nudge-left" | "nudge-right" | "nudge-up" | "nudge-down"
-  | "new-folder" | "duplicate" | "group" | "merge" | "clip" | "layer-up" | "layer-down" | "blend-next" | "blend-prev" | "delete-layer"
+  | "new-folder" | "duplicate" | "group" | "ungroup" | "merge" | "clip" | "layer-up" | "layer-down" | "blend-next" | "blend-prev" | "delete-layer"
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
   | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
@@ -24,7 +24,7 @@ export const SHORTCUTS = {
   "nudge-right": [{ key: "ArrowRight" }, { key: "ArrowRight", shift: true }],
   "nudge-up": [{ key: "ArrowUp" }, { key: "ArrowUp", shift: true }],
   "nudge-down": [{ key: "ArrowDown" }, { key: "ArrowDown", shift: true }],
-  "new-folder": [], "duplicate": [{ key: "j", ctrl: true }], "group": [{ key: "g", ctrl: true }], "merge": [{ key: "e", ctrl: true }],
+  "new-folder": [], "duplicate": [{ key: "j", ctrl: true }], "group": [{ key: "g", ctrl: true }], "ungroup": [{ key: "g", ctrl: true, shift: true }], "merge": [{ key: "e", ctrl: true }],
   "clip": [{ key: "g", ctrl: true, alt: true }], "layer-up": [{ key: "]", ctrl: true }], "layer-down": [{ key: "[", ctrl: true }],
   "blend-next": [{ key: "=", shift: true }, { key: "+", shift: true }], "blend-prev": [{ key: "-", shift: true }, { key: "_", shift: true }],
   "delete-layer": [{ key: "Delete" }, { key: "Backspace" }],
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -6,7 +6,7 @@ import { isEditableTarget } from "./target";
 import { nudgeDelta } from "../tools/transform-session";
 import type { Corners, PointTuple } from "../engine/types";
 import { activeLayer } from "../state/selection";
-import { addFolder, cycleBlendMode, deleteKeyPressed, duplicateSelected, fillActive, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";
+import { addFolder, cycleBlendMode, deleteKeyPressed, duplicateSelected, fillActive, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive, ungroupActive } from "../actions/layers";
 import { isSelectionTool } from "../tools/selection-draft";
 
 const NUDGE_KEYS: Partial<Record<ActionId, string>> = { "nudge-left": "ArrowLeft", "nudge-right": "ArrowRight", "nudge-up": "ArrowUp", "nudge-down": "ArrowDown" };
@@ -110,6 +110,7 @@ export function runAction(id: ActionId, shift = false): void {
     case "new-folder": addFolder(); break;
     case "duplicate": duplicateSelected(); break;
     case "group": groupSelected(); break;
+    case "ungroup": ungroupActive(); break;
     case "merge": mergeSelected(); break;
     case "clip": toggleClippingOfActive(); break;
     case "layer-up": moveActiveBy(1); break;
```

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -19,6 +19,8 @@ pub enum Command {
     SetLayerBlendMode { #[serde(with = "ids::upper")] id: Uuid, mode: BlendMode },
     AddGroup,
     GroupLayers { #[serde(deserialize_with = "deserialize_ids", serialize_with = "serialize_ids")] ids: Vec<Uuid> },
+    /// Ungroup Layers: the folder's children take its place and the folder goes (LayerGroups.swift:214-239).
+    UngroupLayers { #[serde(with = "ids::upper")] id: Uuid },
     PlaceLayer { #[serde(with = "ids::upper")] id: Uuid, #[serde(default, with = "ids::upper_opt")] parent: Option<Uuid>, #[serde(default, with = "ids::upper_opt")] above: Option<Uuid>, #[serde(default, rename = "atBottom")] at_bottom: bool },
     MoveLayerBy { #[serde(with = "ids::upper")] id: Uuid, offset: i32 },
     DuplicateLayer { #[serde(with = "ids::upper")] id: Uuid },
@@ -95,6 +97,7 @@ impl Command {
             Command::SetLayerBlendMode { .. } => "Layer Blend Mode",
             Command::AddGroup => "New Folder",
             Command::GroupLayers { .. } => "Group Layers",
+            Command::UngroupLayers { .. } => "Ungroup Layers",
             Command::PlaceLayer { .. } => "Move Layer",
             Command::MoveLayerBy { .. } => "Reorder Layers",
             Command::DuplicateLayer { .. } => "Duplicate Layer",
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -496,6 +496,7 @@ impl Engine {
             Command::SetLayerBlendMode { id, mode } => { ops::appearance::set_blend_mode(doc, id, mode)?; Ok(Dirty::structure()) }
             Command::AddGroup => { ops::hierarchy::add_group(doc)?; Ok(Dirty::structure()) }
             Command::GroupLayers { ids } => { ops::hierarchy::group_layers(doc, &ids)?; Ok(Dirty::structure()) }
+            Command::UngroupLayers { id } => { ops::hierarchy::ungroup_layers(doc, id)?; Ok(Dirty::structure()) }
             Command::PlaceLayer { id, parent, above, at_bottom } => { ops::hierarchy::place_layer(doc, id, parent, above, at_bottom)?; Ok(Dirty::structure()) }
             Command::MoveLayerBy { id, offset } => { ops::hierarchy::move_layer_by(doc, id, offset)?; Ok(Dirty::structure()) }
             Command::DuplicateLayer { id } => { ops::hierarchy::duplicate_layer(doc, id)?; Ok(Dirty::structure()) }
```

```diff
--- a/engine/src/ops/hierarchy.rs
+++ b/engine/src/ops/hierarchy.rs
@@ -83,6 +83,34 @@ pub fn group_layers(doc: &mut Document, ids: &[Uuid]) -> Result<Uuid, CommandErr
     Ok(gid)
 }
 
+/// Ungroup Layers (`ungroupLayers`, LayerGroups.swift:214-239 at v1.4.5): the folder's direct children
+/// take its place among its own siblings, in the order they had inside it, and the folder goes, its own
+/// opacity, blend mode, mask and effects with it, as Photoshop's Ungroup does. A clipped layer no longer
+/// next to its base stops clipping (`releaseDetachedClipping`). The first child becomes the active
+/// layer (`selectLayers(childIDs, primary: children.first?.id)`); the children, bottom first, are
+/// returned for the app to select.
+pub fn ungroup_layers(doc: &mut Document, id: Uuid) -> Result<Vec<Uuid>, CommandError> {
+    let group = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    if !group.is_group { return Err(CommandError::Argument("only a folder can be ungrouped".into())); }
+    let parent = group.parent_id;
+    let child_ids: HashSet<Uuid> = doc.layers.iter().filter(|l| l.parent_id == Some(id)).map(|l| l.id).collect();
+    let children: Vec<Layer> = doc.layers.iter().filter(|l| child_ids.contains(&l.id)).cloned()
+        .map(|mut l| { l.parent_id = parent; l }).collect();
+    // Spliced in at the folder's own spot, so they land exactly where it sat among its siblings.
+    let mut layers: Vec<Layer> = Vec::with_capacity(doc.layers.len());
+    for layer in &doc.layers {
+        if layer.id == id { layers.extend(children.iter().cloned()); }
+        else if !child_ids.contains(&layer.id) { layers.push(layer.clone()); }
+    }
+    release_detached_clipping(&mut layers);
+    let ordered: Vec<Uuid> = children.iter().map(|l| l.id).collect();
+    // The folder may be the active layer; it is gone, so the active layer moves before the check.
+    let previous = (std::mem::replace(&mut doc.layers, layers), doc.active_layer_id);
+    doc.active_layer_id = ordered.first().copied();
+    if let Err(e) = validate(doc) { (doc.layers, doc.active_layer_id) = previous; return Err(e); }
+    Ok(ordered)
+}
+
 pub fn can_place(doc: &Document, id: Uuid, parent: Option<Uuid>) -> bool {
     if doc.layer(id).is_none() { return false; }
     let Some(parent) = parent else { return true; };
```


The folder may itself be the active layer, and it is gone after the splice, so `ungroup_layers` moves the active layer to the first child before `validate` (which refuses an active id that names no layer) and puts both back if the check fails.

- [ ] **Step 4: Run the whole set, then the timings**

`cargo test -p compositor-engine`: 576 passed, 10 ignored (72 binaries). `pnpm wasm:dev`; `pnpm test` 245 in 37 files; `pnpm build` clean; `pnpm e2e` 168 passed, 23 skipped.

Timings (`-g "Ungroup Layers at"`, release; the case ungroups a folder at half opacity holding a full-size Screen copy and a copy clipped to it, through Shift+Ctrl+G), two runs: the step 2-5 ms and the frame after 10-13 ms at both sizes (cold frames 63-84 ms at 24 MP, logged). Budgets 16 and 33.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Append the children at the top instead of splicing them in at the folder's spot: `ungroup_restores_the_children_at_the_folders_spot_and_undoes` fails on the order (measured).
2. Drop `release_detached_clipping`: `ungroup_releases_a_clip_that_no_longer_makes_sense` fails (measured).
3. `ungroupActive` does not select the children: the e2e's `selected` check fails (measured).
4. Leave Shift+Ctrl+G unmapped (the entry asks for Alt too): the keymap test fails, null for `ungroup` (measured).

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/ungroup.spec.ts engine/tests/ungroup.rs
git commit -m "feat: Ungroup Layers from the Layer menu, Shift+Ctrl+G and a folder's context menu, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/actions/layers.ts app/src/engine/types.ts app/src/panels/LayersList.tsx app/src/panels/MenuBar.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/ungroup.spec.ts app/tests/unit/keymap.test.ts engine/src/command.rs engine/src/engine.rs engine/src/ops/hierarchy.rs engine/tests/ungroup.rs
```


---

### Task 12: Dragging a tab reorders the tabs

1.4.1 lets the user drag a project tab along the strip (ruling OQ12). The Mac's drag state and its arithmetic are ported as a pure module; the strip gets the pointer handlers.

**Files:**
- Create: `app/src/state/tab-reorder.ts` (`TabSlot`, `TabReorderState`, `TAB_DRAG_THRESHOLD`, `reorderedTabs`, `makeReorderState`, `nearestSlot`, `dragged`, `renderX`, `commitTarget`), `app/tests/unit/tab-reorder.test.ts`, `app/tests/e2e/tabs.spec.ts`
- Modify: `app/src/panels/ProjectTabs.tsx` (pointer handlers, the tabs sliding aside, `data-doc-id`), `app/src/state/store.ts` (`moveTab`), `app/src/styles.css` (a tab's title is not selectable)
- Modify (tests): `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** ProjectTabs.swift:21-43 (`TabReorderState`, `nearestSlot`), :147-155 (`renderX`), :157-173 (`handleReorder`: 3 px, the drag selects its tab, `canSwitch`), :175-188 (`makeReorderState`), :192-205 (`commitReorder`), :353-355 (`DragGesture(minimumDistance: 3)`); ProjectWorkspace.swift:27-31 (`canSwitch`), :47-55 (`moveTab`); ProjectWorkspaceTests.swift:126-145 (ported).

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -384,3 +384,76 @@ test("Ungroup Layers at 24 and 100 MP: the step and the frame after", async ({ p
     expect(out[`${label} frame after, worst of 3 ms`]).toBeLessThan(33);
   }
 });
+
+test("dragging a tab at 24 and 100 MP: the tick that starts the drag (it selects the tab) and every later tick, as frame gaps", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    // Two documents of this size and a small one; the drag takes the second big one's tab to the end.
+    const ids = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const make = (width: number, height: number) => {
+        const doc = api.engine.newDocument(10, 10, false);
+        api.engine.execute(doc, { type: "CanvasSize", width, height, anchor: 4, fill: [0.5, 0.4, 0.3] });
+        api.store.getState().openDocument(doc);
+        return doc as string;
+      };
+      const ids = [make(w, h), make(w, h), make(256, 256)];
+      await settle();
+      return ids;
+    }, [w, h] as [number, number]);
+    const tab = (id: string) => page.locator(`[data-testid="project-tab"][data-doc-id="${id}"]`);
+    // Warm-up (LL-074): switch to each tab once so every document's layers are on the GPU.
+    for (const id of ids) { await tab(id).click(); await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)))); }
+    await tab(ids[0]).click();
+    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
+    // Frame gaps in one contiguous window from the press to the release.
+    await page.evaluate(() => {
+      const w = window as any; w.__gaps = [] as number[]; w.__gapping = true;
+      let last = performance.now();
+      const tick = () => { const now = performance.now(); w.__gaps.push(now - last); last = now; if (w.__gapping) requestAnimationFrame(tick); };
+      requestAnimationFrame(tick);
+    });
+    const box = (await tab(ids[1]).boundingBox())!;
+    const end = (await tab(ids[2]).boundingBox())!;
+    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
+    await page.mouse.down();
+    const startAt = await page.evaluate(() => (window as any).__gaps.length);
+    await page.mouse.move(box.x + box.width / 2 + 6, box.y + box.height / 2);
+    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
+    const afterStart = await page.evaluate(() => (window as any).__gaps.length);
+    const steps = 20;
+    for (let i = 1; i <= steps; i++) await page.mouse.move(box.x + box.width / 2 + 6 + (end.x + end.width - box.x) * i / steps, box.y + box.height / 2);
+    await page.mouse.up();
+    const r = await page.evaluate(([startAt, afterStart, ids]) => {
+      const w = window as any; w.__gapping = false;
+      const gaps: number[] = w.__gaps;
+      const s = w.__compositor.store.getState();
+      return {
+        start: Math.round(Math.max(0, ...gaps.slice(startAt, afterStart))),
+        later: Math.round(Math.max(0, ...gaps.slice(afterStart))),
+        moved: s.order.join() === [ids[0], ids[2], ids[1]].join() ? 1 : 0,
+        active: s.activeId === ids[1] ? 1 : 0,
+      };
+    }, [startAt, afterStart, ids] as [number, number, string[]]);
+    out[`${label} longest gap around the tick that starts the drag, ms`] = r.start;
+    out[`${label} longest gap over the later ticks, ms`] = r.later;
+    out[`${label} dropped at the end`] = r.moved;
+    out[`${label} the dragged tab is active`] = r.active;
+    await page.evaluate((ids) => { const s = (window as any).__compositor.store.getState(); for (const id of ids) s.closeDocument(id); }, ids);
+  }
+  console.log(`tab drag (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`tab drag renderer: ${renderer}`);
+  for (const [label] of SIZES) {
+    expect(out[`${label} dropped at the end`], "the drag reordered the tabs").toBe(1);
+    expect(out[`${label} the dragged tab is active`], "dragging a tab selects it").toBe(1);
+    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): 22-24 ms around the tick that starts the drag (it
+    // selects the tab, so a frame of the other 24 or 100 MP document follows) and 18-33 ms over the later ticks.
+    // 4b-1's gap budget for the first and its drag-tick budget for the rest.
+    expect(out[`${label} longest gap around the tick that starts the drag, ms`]).toBeLessThan(100);
+    expect(out[`${label} longest gap over the later ticks, ms`]).toBeLessThan(50);
+  }
+});
```

Create `app/tests/e2e/tabs.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";

// Dragging a project tab along the strip reorders the tabs (Compositor 1.4.5, ProjectTabs.swift:147-205):
// a press that moves 3 px or more becomes a drag and selects its tab; let go, the tab lands in the gap
// nearest it; a job's result to come (or a project operation) leaves the tabs where they were.

async function threeTabs(page: Page): Promise<string[]> {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  return page.evaluate(() => {
    const api = (window as any).__compositor;
    return [40, 50, 60].map((size) => { const doc = api.engine.newDocument(size, size, false); api.store.getState().openDocument(doc); return doc as string; });
  });
}
const tab = (page: Page, id: string) => page.locator(`[data-testid="project-tab"][data-doc-id="${id}"]`);
const order = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().order as string[]);
const active = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().activeId as string);
/** Presses the tab's title, moves by `dx` in `steps` moves, and lets go. */
async function drag(page: Page, id: string, dx: number, steps = 8) {
  const box = (await tab(page, id).locator("span").boundingBox())!;
  const x = box.x + box.width / 2, y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  for (let i = 1; i <= steps; i++) await page.mouse.move(x + dx * i / steps, y);
  await page.mouse.up();
}

test("dragging a tab past its neighbours moves it there and selects it; a 2 px wobble is a click", async ({ page }) => {
  const [a, b, c] = await threeTabs(page);
  expect(await order(page)).toEqual([a, b, c]);
  expect(await active(page)).toBe(c);
  // Under the 3 px threshold nothing moves; the press is a click on `a`.
  await drag(page, a, 2, 2);
  expect(await order(page)).toEqual([a, b, c]);
  // `c` dragged to the far left lands first and is selected.
  await tab(page, a).click();
  const toStart = (await tab(page, a).boundingBox())!.x - (await tab(page, c).boundingBox())!.x - 20;
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([c, a, b]);
  expect(await active(page)).toBe(c);
  // `c` dragged past the last tab lands last.
  const toEnd = (await tab(page, b).boundingBox())!.x + (await tab(page, b).boundingBox())!.width - (await tab(page, c).boundingBox())!.x + 20;
  await drag(page, c, toEnd);
  expect(await order(page)).toEqual([a, b, c]);
});

test("while a job's result is to come a tab drag moves nothing", async ({ page }) => {
  const [a, b, c] = await threeTabs(page);
  await page.evaluate(() => (window as any).__compositor.store.setState({ working: true }));
  const toStart = (await tab(page, a).boundingBox())!.x - (await tab(page, c).boundingBox())!.x - 20;
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([a, b, c]);
  await page.evaluate(() => (window as any).__compositor.store.setState({ working: false }));
  await drag(page, c, toStart);
  expect(await order(page)).toEqual([c, a, b]);
});
```

Create `app/tests/unit/tab-reorder.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { commitTarget, dragged, makeReorderState, nearestSlot, renderX, reorderedTabs, type TabSlot } from "../../src/state/tab-reorder";

// Compositor 1.4.5's tab reordering (ProjectTabs.swift and ProjectWorkspace.swift at v1.4.5).
describe("reorderedTabs (ProjectWorkspace.moveTab)", () => {
  it("reorders, clamps and ignores a tab already in place or unknown", () => {
    // moveTabReordersWithoutTouchingSelectionOrDocuments (ProjectWorkspaceTests.swift:126-145), step for step.
    let order = ["a", "b", "c"];
    order = reorderedTabs(order, "c", 0);
    expect(order).toEqual(["c", "a", "b"]);
    order = reorderedTabs(order, "a", 2);
    expect(order).toEqual(["c", "b", "a"]);
    order = reorderedTabs(order, "c", 99);
    expect(order).toEqual(["b", "a", "c"]);
    expect(reorderedTabs(order, "c", 2)).toBe(order);
    expect(reorderedTabs(order, "zz", 0)).toBe(order);
    expect(reorderedTabs(order, "b", -5)).toBe(order);
  });
});

// Three tabs 100, 60 and 80 wide with 4 between them, from x 10.
const W = { a: 100, b: 60, c: 80 }, GAP = 4, X0 = 10;
const layout: TabSlot[] = [
  { id: "a", x: X0, width: W.a },
  { id: "b", x: X0 + W.a + GAP, width: W.b },
  { id: "c", x: X0 + W.a + GAP + W.b + GAP, width: W.c },
];

describe("a tab drag (TabReorderState)", () => {
  it("packs the other tabs as if the dragged one were gone", () => {
    const s = makeReorderState("a", layout, GAP);
    expect(s.others).toEqual(["b", "c"]);
    expect(s.compactedX).toEqual({ b: X0, c: X0 + W.b + GAP });
    expect([s.startX, s.originX]).toEqual([X0, X0]);
  });

  it("drops at the slot nearest the dragged tab's left edge, the end included", () => {
    const s = makeReorderState("a", layout, GAP);
    const slots = [X0, X0 + W.b + GAP, X0 + W.b + GAP + W.c + GAP];
    for (const [translation, expected] of [[0, 0], [slots[1] - X0 - 1, 1], [slots[2] - X0 + 500, 2]] as const) {
      expect(nearestSlot({ ...s, translation }), `translation ${translation}`).toBe(expected);
    }
    // Halfway between two slots the first wins, as Swift's min(by:) keeps the first.
    expect(nearestSlot({ ...s, translation: (slots[0] + slots[1]) / 2 - X0 })).toBe(0);
    // Dragging c to the left end.
    expect(dragged(makeReorderState("c", layout, GAP), -1000).targetIndex).toBe(0);
  });

  it("commits to the neighbour's place, past the last one, or where it was", () => {
    const order = ["a", "b", "c"];
    // a let go at the end: after c, less the one it leaves (commitReorder, :196-204).
    const aToEnd = dragged(makeReorderState("a", layout, GAP), 1000);
    expect(reorderedTabs(order, "a", commitTarget(order, aToEnd)!)).toEqual(["b", "c", "a"]);
    // c let go at slot 0: before a.
    const cToStart = dragged(makeReorderState("c", layout, GAP), -1000);
    expect(reorderedTabs(order, "c", commitTarget(order, cToStart)!)).toEqual(["c", "a", "b"]);
    // b let go where it started: no change.
    const bStays = dragged(makeReorderState("b", layout, GAP), 2);
    expect(reorderedTabs(order, "b", commitTarget(order, bStays)!)).toBe(order);
    // The tab closed under the drag: nothing to commit.
    expect(commitTarget(["a", "c"], bStays)).toBeNull();
  });

  it("draws the dragged tab under the pointer inside the strip and opens a gap at the target", () => {
    const content = X0 + W.a + GAP + W.b + GAP + W.c;
    const s = dragged(makeReorderState("a", layout, GAP), 70);
    expect(renderX(s, "a", X0, content)).toBe(X0 + 70);
    expect(renderX(dragged(s, -50), "a", X0, content)).toBe(X0);
    expect(renderX(dragged(s, 5000), "a", X0, content)).toBe(content - W.a);
    // Target 1: b stays at the front, c moves over to make room for a.
    expect(s.targetIndex).toBe(1);
    expect(renderX(s, "b", layout[1].x, content)).toBe(X0);
    expect(renderX(s, "c", layout[2].x, content)).toBe(X0 + W.b + GAP + W.a + GAP);
    expect(renderX(null, "c", layout[2].x, content)).toBe(layout[2].x);
  });
});
```


The unit tests compute every position from the widths and gap they lay out (three tabs 100, 60 and 80 wide, 4 apart, from x 10); `reorderedTabs` follows the Mac's `moveTab` test step for step.

- [ ] **Step 2: Run them and see them fail**

Run: `pnpm test -- tab-reorder`
Expected: the suite fails to load (`../../src/state/tab-reorder` does not exist). `npx playwright test app/tests/e2e/tabs.spec.ts` fails: the order never changes.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/panels/ProjectTabs.tsx
+++ b/app/src/panels/ProjectTabs.tsx
@@ -1,17 +1,82 @@
+import { useRef, useState } from "react";
 import { useEditor } from "../state/store";
 import { closeProject } from "../actions/files";
+import { TAB_DRAG_THRESHOLD, commitTarget, dragged, makeReorderState, renderX, type TabReorderState, type TabSlot } from "../state/tab-reorder";
 
 const MODIFIED_DOT = " " + String.fromCharCode(0x2022);
 
+/** A press on a tab, until it is let go: where it started and the strip as it was laid out then. */
+interface Press { id: string; pointerId: number; startX: number; layout: TabSlot[]; spacing: number; contentWidth: number; }
+
+/** The Mac's `workspace.canSwitch` as far as a reorder needs it: no job's result to come and no project
+ * operation running (ProjectWorkspace.swift:27-31 at v1.4.5). */
+function canReorder(): boolean { const s = useEditor.getState(); return !s.working && !s.busy; }
+
 export function ProjectTabs() {
   const s = useEditor();
+  const strip = useRef<HTMLDivElement>(null);
+  const press = useRef<Press | null>(null);
+  // Mirrored in a ref so the pointer handlers, which run between renders, read the latest state.
+  const reorderRef = useRef<TabReorderState | null>(null);
+  const [reorder, setReorderState] = useState<TabReorderState | null>(null);
+  const setReorder = (r: TabReorderState | null) => { reorderRef.current = r; setReorderState(r); };
+
+  const layout = (): { slots: TabSlot[]; spacing: number; contentWidth: number } => {
+    const tabs = Array.from(strip.current?.querySelectorAll<HTMLElement>('[data-testid="project-tab"]') ?? []);
+    const slots = tabs.map((el) => ({ id: el.dataset.docId!, x: el.offsetLeft, width: el.offsetWidth }));
+    const spacing = slots.length > 1 ? Math.max(0, slots[1].x - (slots[0].x + slots[0].width)) : 0;
+    const last = slots[slots.length - 1];
+    return { slots, spacing, contentWidth: last ? last.x + last.width : 0 };
+  };
+  const down = (e: React.PointerEvent, id: string) => {
+    if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
+    const { slots, spacing, contentWidth } = layout();
+    press.current = { id, pointerId: e.pointerId, startX: e.clientX, layout: slots, spacing, contentWidth };
+    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
+  };
+  const move = (e: React.PointerEvent) => {
+    const p = press.current;
+    if (!p || e.pointerId !== p.pointerId) return;
+    const translation = e.clientX - p.startX;
+    let r = reorderRef.current;
+    if (!r) {
+      // ProjectTabs.swift:160-164: a press that moves 3 px becomes a drag, and the drag selects its tab.
+      if (!canReorder() || Math.abs(translation) < TAB_DRAG_THRESHOLD) return;
+      useEditor.getState().setActive(p.id);
+      r = makeReorderState(p.id, p.layout, p.spacing);
+    }
+    if (r.id !== p.id) return;
+    setReorder(dragged(r, translation));
+  };
+  const up = (e: React.PointerEvent) => {
+    const p = press.current;
+    if (!p || e.pointerId !== p.pointerId) return;
+    const r = reorderRef.current;
+    // Busy by the time the drag ends: the tabs go back where they were (:169-171).
+    if (r && r.id === p.id && canReorder()) {
+      const st = useEditor.getState();
+      const target = commitTarget(st.order, r);
+      if (target !== null) st.moveTab(r.id, target);
+    }
+    press.current = null;
+    setReorder(null);
+  };
+  const cancel = () => { press.current = null; setReorder(null); };
+
+  const contentWidth = press.current?.contentWidth ?? 0;
   return (
-    <div className="tabs">
+    <div className="tabs" ref={strip}>
       {s.order.map((id) => {
         const d = s.documents[id];
         const title = d.path ? s.bridge!.baseName(d.path) : "Untitled";
+        const slot = press.current?.layout.find((l) => l.id === id);
+        const offset = reorder && slot ? renderX(reorder, id, slot.x, contentWidth) - slot.x : 0;
+        const isDragged = reorder?.id === id;
         return (
-          <div key={id} data-testid="project-tab" className={"tab" + (id === s.activeId ? " active" : "")} onClick={() => s.setActive(id)}>
+          <div key={id} data-testid="project-tab" data-doc-id={id} className={"tab" + (id === s.activeId ? " active" : "") + (isDragged ? " dragging" : "")}
+            style={reorder ? { transform: `translateX(${offset}px)`, transition: isDragged ? "none" : "transform 0.15s ease-out", zIndex: isDragged ? 1 : 0, position: "relative" } : undefined}
+            onClick={() => s.setActive(id)}
+            onPointerDown={(e) => down(e, id)} onPointerMove={move} onPointerUp={up} onPointerCancel={cancel} onLostPointerCapture={cancel}>
             <span>{title}{d.isModified ? MODIFIED_DOT : ""}</span>
             <button aria-label={`Close ${title}`} onClick={(e) => { e.stopPropagation(); void closeProject(id); }}>x</button>
           </div>
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -10,6 +10,7 @@ import { Viewport } from "../canvas/viewport";
 import type { Rect } from "../tools/crop-geometry";
 import { cornersOf, cornersToTuples, isValidTransform, roundedTransform } from "../tools/transform-geometry";
 import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } from "./selection";
+import { reorderedTabs } from "./tab-reorder";
 import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
@@ -204,6 +205,8 @@ export interface EditorStore {
   openDocument(id: string): void;
   closeDocument(id: string): void;
   setActive(id: string): void;
+  /** A tab dragged to `index` in the strip's order (`ProjectWorkspace.moveTab`): chrome, no undo step. */
+  moveTab(id: string, index: number): void;
   refresh(id?: string): void;
   revealActiveLayer(): void;
   /** True when the engine accepted the command; a refusal raises the banner. */
@@ -570,6 +573,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
         ...(s.activeId === id ? { selectionDraft: null, outlineMove: null } : {}) };
     });
   },
+  moveTab: (id, index) => set((s) => ({ order: reorderedTabs(s.order, id, index) })),
   setActive: (id) => {
     // Clicking the tab already on screen changes nothing, and so must not cancel its panel.
     if (id === get().activeId) return;
```

Create `app/src/state/tab-reorder.ts`:

```ts
// Dragging a project tab along the strip to reorder it, as Compositor 1.4.5 does (ProjectTabs.swift at
// v1.4.5: TabReorderState :21-43, renderX :147-155, handleReorder :157-173, makeReorderState :175-188,
// commitReorder :192-205; ProjectWorkspace.moveTab, ProjectWorkspace.swift:47-55). Kept free of React
// so the strip and its tests share one definition.

/** A tab where the strip lays it out, in pixels from the strip's left edge. */
export interface TabSlot { id: string; x: number; width: number; }

/** A tab mid-drag: the rest of the tabs laid out as if it were not there (`compactedX`), captured once
 * when the drag starts, and how far the pointer has moved since the press. */
export interface TabReorderState {
  id: string;
  others: string[];
  widths: Record<string, number>;
  compactedX: Record<string, number>;
  startX: number;
  originX: number;
  spacing: number;
  translation: number;
  targetIndex: number;
}

/** A press becomes a drag once it has moved this many pixels (`DragGesture(minimumDistance: 3)`, :353, :161). */
export const TAB_DRAG_THRESHOLD = 3;

/** `order` with `id` moved to `index` in the final order, clamped to the ends; unchanged when `id` is not
 * a tab or already sits there (`moveTab`). Chrome, not a document edit: it never touches undo. */
export function reorderedTabs(order: string[], id: string, index: number): string[] {
  const from = order.indexOf(id);
  if (from < 0) return order;
  const target = Math.min(Math.max(0, index), order.length - 1);
  if (target === from) return order;
  const next = order.filter((o) => o !== id);
  next.splice(target, 0, id);
  return next;
}

/** The drag's starting state: the other tabs packed from the first tab's x with `spacing` between them. */
export function makeReorderState(id: string, visible: TabSlot[], spacing: number): TabReorderState {
  const widths: Record<string, number> = {};
  for (const s of visible) widths[s.id] = s.width;
  const others = visible.map((s) => s.id).filter((o) => o !== id);
  const startX = visible[0]?.x ?? 0;
  const compactedX: Record<string, number> = {};
  let x = startX;
  for (const o of others) { compactedX[o] = x; x += (widths[o] ?? 0) + spacing; }
  const originX = visible.find((s) => s.id === id)?.x ?? startX;
  return { id, others, widths, compactedX, startX, originX, spacing, translation: 0, targetIndex: 0 };
}

/** The gap the dragged tab is nearest: slot k opens where the k-th other tab sits, or after the last one.
 * The first of two equally near slots wins, as Swift's `min(by:)` keeps the first. */
export function nearestSlot(s: TabReorderState): number {
  const x = s.originX + s.translation;
  const last = s.others[s.others.length - 1];
  const end = last !== undefined ? (s.compactedX[last] ?? s.startX) + (s.widths[last] ?? 0) + s.spacing : s.startX;
  const slots = [...s.others.map((o) => s.compactedX[o] ?? s.startX), end];
  let best = 0;
  for (let i = 1; i < slots.length; i++) if (Math.abs(slots[i] - x) < Math.abs(slots[best] - x)) best = i;
  return best;
}

/** The state with the pointer `translation` pixels from the press, and the drop target that gives. */
export function dragged(s: TabReorderState, translation: number): TabReorderState {
  const moved = { ...s, translation };
  return { ...moved, targetIndex: nearestSlot(moved) };
}

/** Where a tab draws mid-drag: the dragged one follows the pointer, held inside the strip; the others open
 * a gap at the drop target. `slotX` is where it sits when nothing is dragged. */
export function renderX(s: TabReorderState | null, id: string, slotX: number, contentWidth: number): number {
  if (!s) return slotX;
  if (id === s.id) {
    const width = s.widths[id] ?? 0;
    return Math.min(Math.max(s.originX + s.translation, s.startX), Math.max(s.startX, contentWidth - width));
  }
  const x = s.compactedX[id];
  const index = s.others.indexOf(id);
  if (x === undefined || index < 0) return slotX;
  return index >= s.targetIndex ? x + (s.widths[s.id] ?? 0) + s.spacing : x;
}

/** The index in `order` the dragged tab moves to when let go (`commitReorder`), or null when it is gone. */
export function commitTarget(order: string[], s: TabReorderState): number | null {
  const from = order.indexOf(s.id);
  if (from < 0) return null;
  let target: number;
  const neighbour = s.targetIndex < s.others.length ? order.indexOf(s.others[s.targetIndex]) : -1;
  const last = s.others.length > 0 ? order.indexOf(s.others[s.others.length - 1]) : -1;
  if (neighbour >= 0) target = neighbour;
  else if (last >= 0) target = last + 1;
  else target = from;
  if (from < target) target -= 1;
  return target;
}
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -26,7 +26,7 @@ body { margin: 0; }
 
 /* Project tabs */
 .tabs { display: flex; flex: 0 0 auto; background: #232323; border-bottom: 1px solid #000; overflow-x: auto; }
-.tab { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-right: 1px solid #000; cursor: pointer; color: #999; }
+.tab { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-right: 1px solid #000; cursor: pointer; color: #999; user-select: none; }
 .tab.active { background: #1e1e1e; color: #fff; }
 .tab button { background: none; border: none; color: inherit; cursor: pointer; padding: 0 2px; }
```


`user-select: none` on `.tab` is load-bearing: without it the first drag leaves the title selected, and the next press on it starts the browser's own drag of the selected text, which cancels the pointer (`pointercancel`) and puts the tab back (found on the scratch clone: the e2e's second drag of the same tab failed).

- [ ] **Step 4: Run the whole set, then the timings**

No engine change. `pnpm wasm:dev`; `pnpm test` 250 in 38 files; `pnpm build` clean; `pnpm e2e` 170 passed, 24 skipped.

Timings (`-g "dragging a tab"`, release; two documents of the size and a small one, the second big one's tab dragged past the last in 20 moves, frame gaps in one window from the press to the release), two runs: 22-24 ms around the tick that starts the drag (it selects the tab, so the other big document is drawn) and 18-33 ms over the later ticks. Budgets 100 and 50.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `nearestSlot` measures from `originX` without the translation: three unit tests fail (the slot cases, a let go at the end, the gap opened at target 1) (measured).
2. `commitTarget` without `if (from < target) target -= 1`: "b let go where it started" moves b to the end, `[a, c, b]` (measured).
3. The strip starts a drag while `working`: the e2e's second test fails (measured).
4. `.tab` without `user-select: none`: both e2e tests fail at their second drag of a tab (measured).

- [ ] **Step 6: Commit**

```
git add -- app/src/state/tab-reorder.ts app/tests/e2e/tabs.spec.ts app/tests/unit/tab-reorder.test.ts
git commit -m "feat(app): drag a project tab along the strip to reorder the tabs, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/panels/ProjectTabs.tsx app/src/state/store.ts app/src/state/tab-reorder.ts app/src/styles.css app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/tabs.spec.ts app/tests/unit/tab-reorder.test.ts
```


---

### Task 13: Resize handles snap as moving does

In 1.4.1 a resize handle snaps the edges it moves to the canvas and the other layers, within the same reach as a move (ruling OQ13). The port snapped only moves.

**Files:**
- Modify: `app/src/tools/transform-geometry.ts` (`snappedResizePoint`), `app/src/tools/transform-session.ts` (a resize snaps; `SessionInit.lockRatio`), `app/src/canvas/CanvasView.tsx` (a resize session gets the snap targets)
- Modify (tests): `app/tests/unit/transform-session.test.ts`, `app/tests/e2e/transform.spec.ts`, `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** Crop.swift:139-182 (`snappedResizePoint`), EditorCanvas.swift:1971-1985 (a resize snaps before the drag computes its draft; `proportional` is `locksTransformRatio != shift`), LayerTransform.swift:26 (`radians`), :221 (`TransformSnap.distance` 10); ResizeSnapTests.swift (ported).

**Interfaces this task produces:** `snappedResizePoint(point, original, start, index, proportional, xs, ys, tolerance, update) -> { point, guides }`; `SessionInit.lockRatio?: boolean` (true when left out). Task 14 passes the store's lock through it.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -457,3 +457,75 @@ test("dragging a tab at 24 and 100 MP: the tick that starts the drag (it selects
     expect(out[`${label} longest gap over the later ticks, ms`]).toBeLessThan(50);
   }
 });
+
+test("a resize-handle drag with snapping at 24 and 100 MP: the tick (snapping, store and frame) and the release", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      // The layer at half size in the top-left quarter; its right-middle handle is dragged toward the
+      // canvas's centre line, which it snaps to on the last tick.
+      const t = api.engine.state(doc).layers[0].transform;
+      api.engine.execute(doc, { type: "SetLayerTransform", id: layer, transform: { ...t, origin: [w * 0.1, h * 0.1], size: [w * 0.25, h * 0.5] } });
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      s().openDocument(doc); s().setTool("move");
+      await settle(); frame();
+      const el = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLElement).parentElement!;
+      const at = (x: number, y: number) => {
+        const vp = s().viewports[doc]; const p = vp.viewPoint({ x, y }, { width: w, height: h }); const rect = el.getBoundingClientRect();
+        return { clientX: rect.left + p.x, clientY: rect.top + p.y };
+      };
+      const fire = (type: string, x: number, y: number) => el.dispatchEvent(new PointerEvent(type, { ...at(x, y), pointerId: 1, button: 0, buttons: type === "pointerup" ? 0 : 1, bubbles: true }));
+      // The handle sits at (0.35 w, 0.35 h). A synthetic pointer cannot be captured, so the press's
+      // setPointerCapture throws after the session starts; the page logs it and the drag goes on.
+      const drag = async (ticks: number) => {
+        const x0 = w * 0.35, y = h * 0.35, x1 = w * 0.5 - 2 / s().viewports[doc].pointsPerPixel;
+        fire("pointerdown", x0, y);
+        const times: number[] = [];
+        for (let i = 1; i <= ticks; i++) {
+          const t0 = performance.now();
+          fire("pointermove", x0 + (x1 - x0) * i / ticks, y);
+          frame();
+          times.push(performance.now() - t0);
+        }
+        const draft = s().transformEdit?.draft;
+        const snapped = draft ? Math.abs(draft.origin[0] + draft.size[0] - w * 0.5) < 1e-6 : false;
+        const t0 = performance.now();
+        fire("pointerup", x1, y); frame();
+        const release = performance.now() - t0;
+        await settle();
+        return { times, snapped, release };
+      };
+      // Cold (LL-074): one drag first, logged; then the timed one.
+      const cold = await drag(4);
+      result["cold first tick ms"] = Math.round(cold.times[0]);
+      api.engine.undo(doc); s().refresh(doc); frame(); await settle();
+      const timed = await drag(10);
+      result["tick (snapping, store and frame), worst ms"] = Math.round(Math.max(...timed.times));
+      result["release (commit and frame) ms"] = Math.round(timed.release);
+      result["the right edge snapped to the centre line"] = timed.snapped ? 1 : 0;
+      result["undo depth"] = api.engine.state(doc).undoDepth;
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`resize drag (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`resize drag renderer: ${renderer}`);
+  for (const [label] of SIZES) {
+    expect(out[`${label} the right edge snapped to the centre line`], "the drag snapped").toBe(1);
+    // Measured on the HD 520 (p45-scratch, 2026-09-30, two release runs): ticks 5-7 ms and releases 4-5 ms at both sizes
+    // (a transform moves no pixels until it is applied). 4b-1's drag-tick and release budgets.
+    expect(out[`${label} tick (snapping, store and frame), worst ms`]).toBeLessThan(50);
+    expect(out[`${label} release (commit and frame) ms`]).toBeLessThan(150);
+  }
+});
```

```diff
--- a/app/tests/e2e/transform.spec.ts
+++ b/app/tests/e2e/transform.spec.ts
@@ -135,3 +135,33 @@ test("an unlinked mask alone: arrow nudges and Alt-drag move the mask, not the l
   expect(d.layers[0].maskPlacement.origin).not.toEqual(afterNudge.maskPlacement.origin);
   expect(d.layers[0].transform.origin).toEqual(before.transform.origin);
 });
+
+test("a resize handle snaps the edge it drags to another layer's edge, as a move snaps (Crop.swift:139-182 at v1.4.5)", async ({ page }) => {
+  await setup(page);
+  const b64 = await page.evaluate(redSquarePngBase64);
+  // A second layer whose left edge, x 300, is the target; the red layer's right edge starts at 250.
+  await page.evaluate(async (b64) => {
+    const api = (window as any).__compositor; const s = api.store.getState(); const doc = s.activeId;
+    const red = api.engine.state(doc).activeLayerId;
+    api.engine.importImage(doc, Uint8Array.from(atob(b64), (c: string) => c.charCodeAt(0)), "target", { x: 325, y: 45 });
+    const id = api.engine.state(doc).activeLayerId; const t = api.engine.state(doc).layers.find((l: any) => l.id === id).transform;
+    api.engine.execute(doc, { type: "SetLayerTransform", id, transform: { ...t, origin: [300, 20], size: [50, 50] } });
+    api.engine.execute(doc, { type: "SetActiveLayer", id: red });
+    s.refresh(); s.selectLayers([red], red);
+  }, b64);
+  // The right-middle handle dragged to x 294, 6 short of 300 and inside the 10 px reach: the edge lands on 300.
+  const a = await viewPoint(page, 250, 150), b = await viewPoint(page, 294, 150);
+  await page.mouse.move(a.x, a.y); await page.mouse.down(); await page.mouse.move((a.x + b.x) / 2, a.y); await page.mouse.move(b.x, b.y);
+  expect((await page.evaluate(() => (window as any).__compositor.store.getState().snapGuides)).xs).toEqual([300]);
+  await page.mouse.up();
+  const l = (await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers; })).find((x: any) => x.name === "red");
+  expect(l.transform.origin[0] + l.transform.size[0]).toBe(300);
+  // The aspect lock is on (the Mac's default): the height follows the width, about the left-middle handle.
+  expect(l.transform.size).toEqual([150, 150]);
+  expect(l.transform.origin).toEqual([150, 75]);
+  // Out of reach (x 280, 20 short) it does not snap.
+  const c = await viewPoint(page, 300, 150), d = await viewPoint(page, 280, 150);
+  await page.mouse.move(c.x, c.y); await page.mouse.down(); await page.mouse.move((c.x + d.x) / 2, c.y); await page.mouse.move(d.x, d.y); await page.mouse.up();
+  const m = (await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers; })).find((x: any) => x.name === "red");
+  expect(m.transform.origin[0] + m.transform.size[0]).toBe(280);
+});
\ No newline at end of file
```

```diff
--- a/app/tests/unit/transform-session.test.ts
+++ b/app/tests/unit/transform-session.test.ts
@@ -1,6 +1,7 @@
 import { describe, expect, it } from "vitest";
 import { TransformSession, nudgeDelta, startMode } from "../../src/tools/transform-session";
 import type { LayerTransform } from "../../src/engine/types";
+import { roundedTransform } from "../../src/tools/transform-geometry";
 
 const t = (x: number, y: number, w: number, h: number): LayerTransform => ({ origin: [x, y], size: [w, h], rotation: 0, flipX: false, flipY: false, sampling: "High quality" });
 
@@ -35,3 +36,50 @@ describe("transform session", () => {
     expect(nudgeDelta("x", false)).toBeNull();
   });
 });
+
+// ResizeSnapTests.swift (v1.4.5), ported: a 300 x 200 canvas, the layer being resized 100 x 100 at (10, 10),
+// another 40 x 40 at (150, 150) whose left edge is at x 150. Targets as `snapTargets` builds them: the
+// canvas's edges and centre and the other layer's edges and centre. Tolerance 5, as the Mac's tests use.
+describe("resize-handle snapping (snappedResizePoint, Crop.swift:139-182)", () => {
+  const targets = { xs: [0, 150, 300, 150, 170, 190], ys: [0, 100, 200, 150, 170, 190], tolerance: 5 };
+  const resized = t(10, 10, 100, 100);
+  const resize = (index: number, from: { x: number; y: number }, to: { x: number; y: number }, lockRatio: boolean, original = resized) => {
+    const s = new TransformSession({ mode: { kind: "resize", index }, startDoc: from, original, originalCorners: null, snap: targets, lockRatio });
+    const r = s.update(to, { shift: false, alt: false, ctrl: false });
+    return { draft: roundedTransform(r.draft), guides: r.guides };
+  };
+
+  it("a side handle snaps its edge; out of reach it does not (aSideHandleSnapsItsEdge)", () => {
+    // The right edge dragged to 147, three pixels short of the other layer's left edge.
+    const { draft, guides } = resize(3, { x: 110, y: 60 }, { x: 147, y: 60 }, false);
+    expect(draft.origin[0] + draft.size[0]).toBe(150);
+    expect(draft.size[1]).toBe(100);
+    expect(guides).toEqual({ xs: [150], ys: [] });
+    const free = resize(3, { x: 110, y: 60 }, { x: 130, y: 60 }, false).draft;
+    expect(free.origin[0] + free.size[0]).toBe(130);
+  });
+
+  it("a proportional corner snaps its nearer edge and keeps the ratio (aProportionalCornerSnapsItsNearerEdgeAndKeepsTheRatio)", () => {
+    // Bottom right dragged toward (146, 148).
+    const { draft, guides } = resize(4, { x: 110, y: 110 }, { x: 146, y: 148 }, true);
+    expect(draft.origin[1] + draft.size[1]).toBe(150);
+    expect(Math.abs(draft.size[0] - draft.size[1])).toBeLessThanOrEqual(1);
+    // Kept proportional, one edge snaps and the other follows the ratio. Here both are 3 short of 150 and the
+    // first found (the right edge, as Swift's min(by:) keeps the first) is the one that snaps.
+    expect(guides).toEqual({ xs: [150], ys: [] });
+  });
+
+  it("a turned layer does not snap (aTurnedLayerDoesntSnap)", () => {
+    const turned = { ...resized, rotation: 20 };
+    const s = new TransformSession({ mode: { kind: "resize", index: 3 }, startDoc: { x: 110, y: 60 }, original: turned, originalCorners: null, snap: targets, lockRatio: false });
+    const unsnapped = new TransformSession({ mode: { kind: "resize", index: 3 }, startDoc: { x: 110, y: 60 }, original: turned, originalCorners: null, snap: null, lockRatio: false });
+    const mods = { shift: false, alt: false, ctrl: false };
+    expect(s.update({ x: 147, y: 60 }, mods).draft).toEqual(unsnapped.update({ x: 147, y: 60 }, mods).draft);
+    expect(s.update({ x: 147, y: 60 }, mods).guides).toEqual({ xs: [], ys: [] });
+    // The Mac's one point never brings the turned box's edge near a target, so it would pass with the guard
+    // gone; every pointer x from 100 to 260 does, somewhere, and none may snap.
+    for (let x = 100; x <= 260; x++) {
+      expect(s.update({ x, y: 60 }, mods).draft, `x ${x}`).toEqual(unsnapped.update({ x, y: 60 }, mods).draft);
+    }
+  });
+});
```


The unit tests are the Mac's three with its canvas, layers, points and tolerance; the targets are what `snapTargets` builds for that document (the canvas's edges and centre, the other layer's edges and centre). In the proportional case both edges end 3 px short of 150; the first found (the right edge) snaps, as Swift's `min(by:)` keeps the first, and the bottom follows the ratio to 150 too. The Mac's turned-layer test checks one pointer position, which never brings the turned box's edge near a target, so it would pass with the guard removed (measured); the port's test sweeps every pointer x from 100 to 260 as well (LL-068). The e2e drags a real handle 6 px short of another layer's edge (inside the 10 px reach at zoom 1) and 20 px short (outside it).

- [ ] **Step 2: Run them and see them fail**

Run: `pnpm test -- transform-session`
Expected: the side-handle and proportional tests fail (147 where 150 is expected); the turned-layer test passes already (nothing snaps a resize yet), and Step 5's third bug shows it can fail. The e2e fails: no guide at 300, and the edge at 294.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -358,7 +358,7 @@ export function CanvasView() {
       // Re-read the document: beginTransform may have just duplicated the layer, adding it
       // to the layers array snapTargets scans (and excludes by movingIds).
       const targets = snapTargets(useEditor.getState().documents[s0.activeId!], after.ids);
-      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" ? { ...targets, tolerance } : null });
+      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" || mode.kind === "resize" ? { ...targets, tolerance } : null });
       el.setPointerCapture(e.pointerId);
     };
     const move = (e: PointerEvent) => {
```

```diff
--- a/app/src/tools/transform-geometry.ts
+++ b/app/src/tools/transform-geometry.ts
@@ -216,3 +216,46 @@ export function snapOffset(box: RectLike, xs: number[], ys: number[], tolerance:
   const v = shift([box.y, box.y + box.height / 2, box.y + box.height], ys, tolerance);
   return { dx: h.move, dy: v.move, x: h.target, y: v.target };
 }
+
+/** A resize handle dragged to `point`, nudged so the edges the handle moves land on a nearby target within
+ * `tolerance` document pixels, as a moved layer's do (`snappedResizePoint`, Crop.swift:139-182 at v1.4.5).
+ * `update` is the drag's own result for a pointer. Each edge snaps on its own; kept `proportional`, only the
+ * nearer one does and the other follows the ratio. An upright layer only: a turned one's edges do not run
+ * along the targets. */
+export function snappedResizePoint(point: P, original: LayerTransform, start: P, index: number, proportional: boolean,
+  xs: number[], ys: number[], tolerance: number, update: (p: P) => LayerTransform): { point: P; guides: { xs: number[]; ys: number[] } } {
+  if (radians(original) !== 0) return { point, guides: { xs: [], ys: [] } };
+  const handle = HANDLES[index];
+  const grab = pointOf(original, handle);
+  // Where the dragged handle is, to tell its edge from the one across from it.
+  const at = { x: grab.x + point.x - start.x, y: grab.y + point.y - start.y };
+  const edge = (t: LayerTransform, horizontal: boolean): number => {
+    const [lo, hi] = horizontal ? [t.origin[0], t.origin[0] + t.size[0]] : [t.origin[1], t.origin[1] + t.size[1]];
+    const v = horizontal ? at.x : at.y;
+    return Math.abs(lo - v) <= Math.abs(hi - v) ? lo : hi;
+  };
+  const nearest = (value: number, lines: number[]): number | null => {
+    let best: number | null = null;
+    for (const l of lines) if (Math.abs(l - value) <= tolerance && (best === null || Math.abs(l - value) < Math.abs(best - value))) best = l;
+    return best;
+  };
+  const draft = update(point);
+  let snaps: { horizontal: boolean; target: number }[] = [];
+  if (handle.x !== 0.5) { const x = nearest(edge(draft, true), xs); if (x !== null) snaps.push({ horizontal: true, target: x }); }
+  if (handle.y !== 0.5) { const y = nearest(edge(draft, false), ys); if (y !== null) snaps.push({ horizontal: false, target: y }); }
+  if (proportional && snaps.length === 2) {
+    const off = (s: { horizontal: boolean; target: number }) => Math.abs(s.target - edge(draft, s.horizontal));
+    snaps = [off(snaps[1]) < off(snaps[0]) ? snaps[1] : snaps[0]];
+  }
+  // An edge follows the pointer in a straight line along each axis, so one step measured across a pixel lands it.
+  const result = { ...point };
+  for (const snap of snaps) {
+    const before = edge(update(result), snap.horizontal);
+    const nudged = snap.horizontal ? { x: result.x + 1, y: result.y } : { x: result.x, y: result.y + 1 };
+    const perPixel = edge(update(nudged), snap.horizontal) - before;
+    if (Math.abs(perPixel) <= 0.01) continue;
+    const shift = (snap.target - before) / perPixel;
+    if (snap.horizontal) result.x += shift; else result.y += shift;
+  }
+  return { point: result, guides: { xs: snaps.filter((s) => s.horizontal).map((s) => s.target), ys: snaps.filter((s) => !s.horizontal).map((s) => s.target) } };
+}
```

```diff
--- a/app/src/tools/transform-session.ts
+++ b/app/src/tools/transform-session.ts
@@ -1,8 +1,10 @@
 import type { LayerTransform } from "../engine/types";
-import { boundsOf, cornersDrag, snapOffset, transformDrag, type P, type TransformDragMode } from "./transform-geometry";
+import { boundsOf, cornersDrag, snapOffset, snappedResizePoint, transformDrag, type P, type TransformDragMode } from "./transform-geometry";
 
 export interface SnapTargetsWithTolerance { xs: number[]; ys: number[]; tolerance: number; }
-export interface SessionInit { mode: TransformDragMode; startDoc: P; original: LayerTransform; originalCorners: P[] | null; snap: SnapTargetsWithTolerance | null; }
+/** `lockRatio`: whether a handle keeps the aspect ratio (Shift turns it the other way); true when left out,
+ * as the Mac's `locksTransformRatio` starts. */
+export interface SessionInit { mode: TransformDragMode; startDoc: P; original: LayerTransform; originalCorners: P[] | null; snap: SnapTargetsWithTolerance | null; lockRatio?: boolean; }
 export interface SessionResult { draft: LayerTransform; corners: P[] | null; guides: { xs: number[]; ys: number[] }; }
 
 export class TransformSession {
@@ -14,7 +16,17 @@ export class TransformSession {
       const corners = cornersDrag(originalCorners, startDoc, mode.index < 0 ? "move" : mode.index).updated(point, mods.shift);
       return { draft: original, corners, guides };
     }
-    let draft = transformDrag(original, startDoc, mode).updated(point, { lockRatio: mode.kind === "resize", shift: mods.shift, alt: mods.alt });
+    const lockRatio = this.init.lockRatio ?? true;
+    const drag = transformDrag(original, startDoc, mode);
+    const opts = { lockRatio, shift: mods.shift, alt: mods.alt };
+    // A resize handle snaps the edges it moves (Crop.swift:139-182 at v1.4.5), with the ratio as the drag keeps it.
+    let target = point;
+    if (mode.kind === "resize" && snap) {
+      const snapped = snappedResizePoint(point, original, startDoc, mode.index, lockRatio !== mods.shift, snap.xs, snap.ys, snap.tolerance, (p) => drag.updated(p, opts));
+      target = snapped.point;
+      guides.xs.push(...snapped.guides.xs); guides.ys.push(...snapped.guides.ys);
+    }
+    let draft = drag.updated(target, opts);
     if (mode.kind === "move" && snap) {
       const s = snapOffset(boundsOf(draft), snap.xs, snap.ys, snap.tolerance);
       if (s.dx !== 0 || s.dy !== 0) draft = { ...draft, origin: [draft.origin[0] + s.dx, draft.origin[1] + s.dy] };
```


- [ ] **Step 4: Run the whole set, then the timings**

No engine change. `pnpm wasm:dev`; `pnpm test` 253 in 38 files; `pnpm build` clean; `pnpm e2e` 171 passed, 25 skipped (`transform.spec.ts`'s earlier handle test still gets [150, 150]: no target is in reach of its drag).

Timings (`-g "resize-handle drag"`, release; the right-middle handle of a layer a quarter of the canvas wide and half as high dragged in 10 pointer events toward the canvas's centre line, which the last one snaps to; each tick is the event, the snapping, the store and a frame), two runs: ticks 5-7 ms and releases 4-5 ms at both sizes. Budgets 50 and 150 (4b-1's drag tick and release).

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. Snap both edges of a proportional corner (drop the `proportional && snaps.length === 2` step): the proportional test fails, guides `{ xs: [150], ys: [150] }` (measured).
2. Take the edge across from the handle instead of the one under it: the side-handle and proportional tests fail, 147 where 150 is expected (measured).
3. Leave out the `radians(original) !== 0` guard: the turned-layer test fails at x 149 of its sweep (measured; the Mac's single point alone did not fail).
4. `CanvasView` gives a resize session no targets (`snap: null` unless moving): the e2e fails (measured).

- [ ] **Step 6: Commit**

```
git commit -m "feat(app): resize handles snap the edges they move to the canvas and other layers, as Compositor 1.4.5 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/CanvasView.tsx app/src/tools/transform-geometry.ts app/src/tools/transform-session.ts app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/transform.spec.ts app/tests/unit/transform-session.test.ts
```


---

### Task 14: The Move bar's W and H, and the aspect lock

1.4 puts W and H fields in the Move bar beside X and Y, and a lock that keeps the ratio both for them and for the handles, Shift turning it the other way while it is held (ruling OQ14).

**Files:**
- Modify: `app/src/tools/transform-geometry.ts` (`resizedTo`), `app/src/state/store.ts` (`locksTransformRatio`, `setLocksTransformRatio`), `app/src/canvas/CanvasView.tsx` (a handle drag reads the lock), `app/src/panels/TransformInspector.tsx` (W, H, the lock with Shift shown, Cancel and Apply kept in place), `app/src/styles.css` (the bar keeps to one row)
- Modify (tests): `app/tests/unit/transform-geometry.test.ts`, `app/tests/e2e/transform.spec.ts`, `app/tests/e2e/perf-4-5.spec.ts`

**Mac:** TransformInspector.swift:20 and :48 (one scrolling row, indicators hidden), :24-29 (the fields and the lock, Shift shown turned), :49-60 (Cancel and Apply left in place unseen), :89-100 (`resize`: from the origin, the other side scaled when locked, under 1 ignored); EditorSession.swift:197 (`locksTransformRatio = true`); EditorCanvas.swift:1979-1985 (a handle drag uses the lock, Shift turning it).

**Interfaces this task produces:** `resizedTo(t, value, width, lockRatio)`; the store's `locksTransformRatio` / `setLocksTransformRatio`, passed to `TransformSession` as Task 13's `lockRatio`.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/app/tests/e2e/perf-4-5.spec.ts
+++ b/app/tests/e2e/perf-4-5.spec.ts
@@ -529,3 +529,59 @@ test("a resize-handle drag with snapping at 24 and 100 MP: the tick (snapping, s
     expect(out[`${label} release (commit and frame) ms`]).toBeLessThan(150);
   }
 });
+
+test("a typed W at 24 and 100 MP: Enter in the Move bar (the transform applied, one step) and the frame after", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  let renderer = "";
+  for (const [label, w, h] of SIZES) {
+    renderer = await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const s = () => api.store.getState();
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      s().openDocument(doc); s().setTool("move");
+      await settle(); frame();
+      // The W field as the user types into it: the value set the way React sees typing, then Enter.
+      const type = async (value: number) => {
+        const input = document.querySelector('input[aria-label="W"]') as HTMLInputElement;
+        Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, String(value));
+        input.dispatchEvent(new Event("input", { bubbles: true }));
+        await settle();
+        const t0 = performance.now();
+        input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
+        const step = performance.now() - t0;
+        const after = frame();
+        await settle();
+        return [Math.round(step), Math.round(after)];
+      };
+      [result["cold step ms"], result["cold frame after ms"]] = await type(Math.round(w * 0.9));
+      let step = 0, after = 0;
+      for (const f of [0.8, 0.7, 0.6]) { const [a, b] = await type(Math.round(w * f)); step = Math.max(step, a); after = Math.max(after, b); }
+      result["Enter (store, engine), worst of 3 ms"] = step;
+      result["frame after, worst of 3 ms"] = after;
+      const t = api.engine.state(doc).layers[0].transform;
+      result["width x 1000 / canvas"] = Math.round(t.size[0] / w * 1000);
+      result["height x 1000 / canvas"] = Math.round(t.size[1] / h * 1000);
+      s().closeDocument(doc);
+      return result;
+    }, [w, h] as [number, number]);
+    for (const [k, v] of Object.entries(r)) out[`${label} ${k}`] = v;
+  }
+  console.log(`typed W (release wasm, Edge): ${JSON.stringify(out)}`);
+  console.log(`typed W renderer: ${renderer}`);
+  for (const [label] of SIZES) {
+    // The last value typed was 0.6 of the width; the lock (on) scaled the height with it.
+    expect(out[`${label} width x 1000 / canvas`]).toBe(600);
+    expect(out[`${label} height x 1000 / canvas`]).toBe(600);
+    // Measured on the HD 520 (p45-scratch, 2026-09-30, two runs): Enter 1-2 ms and the frame after 4-15 ms at both sizes
+    // (a transform moves no pixels). A step inside a frame and 4b-1's frame budget.
+    expect(out[`${label} Enter (store, engine), worst of 3 ms`]).toBeLessThan(16);
+    expect(out[`${label} frame after, worst of 3 ms`]).toBeLessThan(33);
+  }
+});
\ No newline at end of file
```

```diff
--- a/app/tests/e2e/transform.spec.ts
+++ b/app/tests/e2e/transform.spec.ts
@@ -164,4 +164,30 @@ test("a resize handle snaps the edge it drags to another layer's edge, as a move
   await page.mouse.move(c.x, c.y); await page.mouse.down(); await page.mouse.move((c.x + d.x) / 2, c.y); await page.mouse.move(d.x, d.y); await page.mouse.up();
   const m = (await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].layers; })).find((x: any) => x.name === "red");
   expect(m.transform.origin[0] + m.transform.size[0]).toBe(280);
+});
+
+test("the Move bar's W and H resize from the origin, and the lock keeps the ratio there and on a handle (TransformInspector.swift:24-29 at v1.4.5)", async ({ page }) => {
+  await setup(page);
+  const lock = page.getByTestId("transform-lock");
+  await expect(lock).toHaveAttribute("aria-pressed", "true");
+  await page.getByLabel("W", { exact: true }).fill("200");
+  await page.keyboard.press("Enter");
+  let l = await layer(page);
+  expect(l.transform.size).toEqual([200, 200]);
+  expect(l.transform.origin).toEqual([150, 100]);
+  await lock.click();
+  await expect(lock).toHaveAttribute("aria-pressed", "false");
+  await page.getByLabel("H", { exact: true }).fill("50");
+  await page.keyboard.press("Enter");
+  l = await layer(page);
+  expect(l.transform.size).toEqual([200, 50]);
+  // Unlocked, a corner handle changes each side on its own: the bottom-right corner, at (350, 150), dragged
+  // to (370, 190) makes the layer 220 x 90.
+  await drag(page, { x: 350, y: 150 }, { x: 370, y: 190 });
+  expect((await layer(page)).transform.size).toEqual([220, 90]);
+  // Shift turns the lock the other way while held, and the button shows it turned.
+  await page.keyboard.down("Shift");
+  await expect(lock).toHaveAttribute("aria-pressed", "true");
+  await page.keyboard.up("Shift");
+  await expect(lock).toHaveAttribute("aria-pressed", "false");
 });
\ No newline at end of file
```

```diff
--- a/app/tests/unit/transform-geometry.test.ts
+++ b/app/tests/unit/transform-geometry.test.ts
@@ -1,5 +1,5 @@
 import { describe, expect, it } from "vitest";
-import { cornersOf, containsPoint, hitOverlay, homographyUnitTo, isUsableCorners, mat3Apply, mat3Invert, overlayGeometry, pixelToDocument, snapOffset, transformDrag } from "../../src/tools/transform-geometry";
+import { cornersOf, containsPoint, hitOverlay, homographyUnitTo, isUsableCorners, mat3Apply, mat3Invert, overlayGeometry, pixelToDocument, resizedTo, snapOffset, transformDrag } from "../../src/tools/transform-geometry";
 import { HANDLES } from "../../src/tools/crop-geometry";
 import { Viewport } from "../../src/canvas/viewport";
 import type { LayerTransform } from "../../src/engine/types";
@@ -74,3 +74,21 @@ function pointOfT(tr: LayerTransform, unit: { x: number; y: number }) {
   const r = (tr.rotation % 360) * Math.PI / 180; const x = (unit.x - 0.5) * w, y = (unit.y - 0.5) * h;
   return { x: cx + x * Math.cos(r) - y * Math.sin(r), y: cy + x * Math.sin(r) + y * Math.cos(r) };
 }
+
+// The Move bar's W and H (TransformInspector.swift:89-100 at v1.4.5): the size typed, from the origin; with the
+// aspect lock on the other side scales by the same factor; under 1 (or not a number) nothing changes.
+describe("typed W and H (resizedTo)", () => {
+  const base = t(10, 20, 200, 100, 30);
+  it("keeps the origin and, locked, the ratio", () => {
+    expect(resizedTo(base, 300, true, true)).toEqual({ ...base, size: [300, 100 * 300 / 200] });
+    expect(resizedTo(base, 50, false, true)).toEqual({ ...base, size: [200 * 50 / 100, 50] });
+  });
+  it("unlocked, changes only the side typed", () => {
+    expect(resizedTo(base, 300, true, false)).toEqual({ ...base, size: [300, 100] });
+    expect(resizedTo(base, 7, false, false)).toEqual({ ...base, size: [200, 7] });
+  });
+  it("ignores a size under 1 or not a number", () => {
+    for (const v of [0.5, 0, -10, Number.NaN]) expect(resizedTo(base, v, true, true)).toBe(base);
+    expect(resizedTo(base, 1, false, false).size).toEqual([200, 1]);
+  });
+});
\ No newline at end of file
```


- [ ] **Step 2: Run them and see them fail**

Run: `pnpm test -- transform-geometry`
Expected: the three `resizedTo` tests fail (`resizedTo is not a function`). The e2e fails: no `transform-lock`, no W field.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -358,7 +358,7 @@ export function CanvasView() {
       // Re-read the document: beginTransform may have just duplicated the layer, adding it
       // to the layers array snapTargets scans (and excludes by movingIds).
       const targets = snapTargets(useEditor.getState().documents[s0.activeId!], after.ids);
-      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" || mode.kind === "resize" ? { ...targets, tolerance } : null });
+      session = new TransformSession({ mode, startDoc: docPoint(e), original: after.draft, originalCorners: after.corners ? after.corners.map(fromTuple) : null, snap: mode.kind === "move" || mode.kind === "resize" ? { ...targets, tolerance } : null, lockRatio: useEditor.getState().locksTransformRatio });
       el.setPointerCapture(e.pointerId);
     };
     const move = (e: PointerEvent) => {
```

```diff
--- a/app/src/panels/TransformInspector.tsx
+++ b/app/src/panels/TransformInspector.tsx
@@ -3,7 +3,7 @@ import { useEditor } from "../state/store";
 import type { TransformEdit } from "../state/store";
 import type { DocumentState, LayerTransform, Sampling } from "../engine/types";
 import { activeLayer, canTransform, editedShape, groupBox, transformsAsGroup } from "../state/selection";
-import { scalePercent, scaledToPercent } from "../tools/transform-geometry";
+import { resizedTo, scalePercent, scaledToPercent } from "../tools/transform-geometry";
 
 interface PixelSize { width: number; height: number; }
 
@@ -48,8 +48,22 @@ function NumberField({ label, value, onCommit }: { label: string; value: number;
   );
 }
 
+/** Whether Shift is held: the aspect lock shows turned the other way while it is, as the Mac's button does
+ * (TransformInspector.swift:26-29 at v1.4.5, `HeldModifiers`). */
+function useHeldShift(): boolean {
+  const [held, setHeld] = useState(false);
+  useEffect(() => {
+    const key = (e: KeyboardEvent) => setHeld(e.shiftKey);
+    const blur = () => setHeld(false);
+    window.addEventListener("keydown", key); window.addEventListener("keyup", key); window.addEventListener("blur", blur);
+    return () => { window.removeEventListener("keydown", key); window.removeEventListener("keyup", key); window.removeEventListener("blur", blur); };
+  }, []);
+  return held;
+}
+
 export function TransformInspector() {
   const s = useEditor();
+  const held = useHeldShift();
   const doc = s.activeId ? s.documents[s.activeId] : null;
   if (!doc || s.tool !== "move" || s.sheet !== null) return null;
   if (!s.transformEdit && !canTransform(doc, s.selectedLayerIds, s.maskSelected)) return null;
@@ -73,11 +87,17 @@ export function TransformInspector() {
     <NumberField key={label} label={label} value={value} onCommit={(v) => apply((t) => set(v, t))} />
   );
   const t = shape.transform;
+  const locked = s.locksTransformRatio !== held;
   return (
-    <div className="tool-options" data-testid="transform-inspector">
+    <div className="tool-options one-row" data-testid="transform-inspector">
       <span>{isMaskEdit ? "Transform Mask" : "Transform"}</span>
       {field("X", t.origin[0], (v, t) => ({ ...t, origin: [v, t.origin[1]] }))}
       {field("Y", t.origin[1], (v, t) => ({ ...t, origin: [t.origin[0], v] }))}
+      {field("W", t.size[0], (v, t) => resizedTo(t, v, true, useEditor.getState().locksTransformRatio))}
+      {field("H", t.size[1], (v, t) => resizedTo(t, v, false, useEditor.getState().locksTransformRatio))}
+      {/* Shift turns the lock the other way while dragging a handle, and the button shows it turned. */}
+      <button data-testid="transform-lock" aria-pressed={locked} title="Lock aspect ratio. Hold Shift while dragging a handle to turn it the other way."
+        onClick={() => s.setLocksTransformRatio(!locked !== held)}>Lock</button>
       {pixel && field("Scale", scalePercent(t, pixel), (v, t) => scaledToPercent(t, v, pixel))}
       {field("Angle", t.rotation, (v, t) => ({ ...t, rotation: v % 360 }))}
       <label>Sampling
@@ -85,10 +105,12 @@ export function TransformInspector() {
           <option>Nearest</option><option>Smooth</option><option>High quality</option>
         </select>
       </label>
-      {s.transformEdit?.corners && <>
+      {/* Left in place unseen when nothing waits for them (TransformInspector.swift:49-60 at v1.4.5), so nothing in
+          the bar shifts when a distortion starts in the middle of a drag. */}
+      <span style={{ visibility: s.transformEdit?.corners ? "visible" : "hidden" }}>
         <button data-testid="transform-cancel" onClick={() => s.cancelTransform()}>Cancel</button>
         <button data-testid="transform-apply" className="primary" onClick={() => s.commitTransform()}>Apply</button>
-      </>}
+      </span>
     </div>
   );
 }
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -114,6 +114,9 @@ export interface EditorStore {
   collapsed: Record<string, string[]>;
   transformEdit: TransformEdit | null;
   snapGuides: { xs: number[]; ys: number[] };
+  /** The Move bar's aspect lock (`locksTransformRatio`, EditorSession.swift:197 at v1.4.5): whether a handle and a
+   * typed W or H keep the ratio. On at first, and not saved. */
+  locksTransformRatio: boolean;
   /** Whether the document's saved guides are drawn (View > Hide/Show Guides). Persisted so the
    * choice survives a relaunch, as it does on the Mac. */
   showGuides: boolean;
@@ -231,6 +234,7 @@ export interface EditorStore {
   commitTransform(): void;
   cancelTransform(): void;
   setSnapGuides(g: { xs: number[]; ys: number[] }): void;
+  setLocksTransformRatio(v: boolean): void;
   toggleGuides(): void;
   setBlendPreview(m: BlendMode | null): void;
   previewEdit(): PreviewEdit | null;
@@ -341,7 +345,7 @@ function loadShowGuides(): boolean {
 export const useEditor = create<EditorStore>((set, get) => ({
   engine: null, jobs: null, jobPixels: JOB_PIXELS, working: false, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
   sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, overlayTick: 0, recentTick: 0,
-  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, showGuides: loadShowGuides(),
+  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, locksTransformRatio: true, showGuides: loadShowGuides(),
   blendPreview: null,
   adjustEdit: null,
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
@@ -815,6 +819,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     get().invalidate();
   },
   setSnapGuides: (g) => set({ snapGuides: g }),
+  setLocksTransformRatio: (v) => set({ locksTransformRatio: v }),
   toggleGuides: () => set((s) => {
     const showGuides = !s.showGuides;
     try { localStorage.setItem(GUIDES_KEY, String(showGuides)); } catch { /* ignore */ }
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -136,6 +136,10 @@ body { margin: 0; }
 /* The selection tools' header (Phase 4a). */
 .tool-options .segmented button[aria-pressed="true"] { border-color: #3a6ea5; background: #3a6ea5; color: #fff; }
 .tool-options .hint { color: #999; }
+/* The Move bar keeps to one row and scrolls sideways with no scroll bar, as the Mac's does (TransformInspector.swift:20,
+   :48 at v1.4.5: a horizontal ScrollView with its indicators hidden): a second row would shrink the canvas under it. */
+/* flex-shrink 0: a flex item that scrolls may otherwise shrink below its row (it did, to 10 px, over a tall document). */
+.tool-options.one-row { white-space: nowrap; overflow-x: auto; scrollbar-width: none; flex-shrink: 0; }
 
 /* The palette at the foot of the tool rail (ColorPaletteControls.swift): 24 px swatches, the background
    one 12 px down and right of the foreground one, swap and default-colour buttons beside them. */
```

```diff
--- a/app/src/tools/transform-geometry.ts
+++ b/app/src/tools/transform-geometry.ts
@@ -29,6 +29,14 @@ export function isValidTransform(t: LayerTransform): boolean {
 export function roundedTransform(t: LayerTransform): LayerTransform {
   return { ...t, origin: [Math.round(t.origin[0]), Math.round(t.origin[1])], size: [Math.max(1, Math.round(t.size[0])), Math.max(1, Math.round(t.size[1]))], rotation: Math.round(t.rotation) };
 }
+/** The Move bar's W or H typed (TransformInspector.swift:89-100 at v1.4.5): the size set from the origin, the
+ * other side scaled by the same factor when `lockRatio`; under 1, or not a number, changes nothing. */
+export function resizedTo(t: LayerTransform, value: number, width: boolean, lockRatio: boolean): LayerTransform {
+  if (!(value >= 1)) return t;
+  let [w, h] = t.size;
+  if (width) { if (lockRatio) h *= value / w; w = value; } else { if (lockRatio) w *= value / h; h = value; }
+  return { ...t, size: [w, h] };
+}
 export function scalePercent(t: LayerTransform, pixel: SizeLike): number { return t.size[0] / Math.max(1, pixel.width) * 100; }
 export function scaledToPercent(t: LayerTransform, percent: number, pixel: SizeLike): LayerTransform {
   const c = center(t); const w = pixel.width * percent / 100, h = pixel.height * percent / 100;
```


The Move bar keeps to one row and scrolls sideways with no scroll bar, as the Mac's does (a horizontal ScrollView with hidden indicators, TransformInspector.swift:20, :48). This is load-bearing: with W, H and the lock the bar no longer fits 1280 px, and wrapping to a second row shrank the canvas, took the document off whole device pixels and failed 26 GPU-against-CPU e2e tests on the scratch clone (and moved the document a pixel under the pointer when Cancel and Apply appeared mid-drag). `flex-shrink: 0` is load-bearing too: a flex item that scrolls may shrink below its row, and over the tall `blend-greys` probe it did, to 10 px, which took that document off whole pixels (measured). Cancel and Apply also stay in the bar, hidden, until a distortion waits for them, as the Mac leaves them in place, so nothing in the bar shifts when one starts.

- [ ] **Step 4: Run the whole set, then the timings**

No engine change. `pnpm wasm:dev`; `pnpm test` 256 in 38 files; `pnpm build` clean; `pnpm e2e` 172 passed, 26 skipped.

Timings (`-g "a typed W"`, release; the W field set as typing sets it, then Enter, which applies the transform as one step; the last value is 0.6 of the canvas and the lock scales the height with it), two runs: Enter 1-2 ms and the frame after 4-15 ms at both sizes (cold frame 23-25 ms at 100 MP, logged). Budgets 16 and 33.

- [ ] **Step 5: Introduce bugs and watch the tests fail (measured)**
1. `resizedTo` ignores the lock: "keeps the origin and, locked, the ratio" fails (measured).
2. `resizedTo` resizes about the centre (the origin moves by half the change): both size tests fail on the origin, (-40, -5) for (10, 20), and so does the e2e (measured).
3. `CanvasView` passes `lockRatio: true` whatever the lock says: the e2e's unlocked corner drag keeps the ratio, its bottom edge snaps back to the canvas's centre line at y 150, and the layer stays 200 x 50 instead of 220 x 90 (measured).
4. The lock button ignores Shift: the e2e fails at the held-Shift check (measured).
5. The Move bar without `flex-shrink: 0`: the blend-greys e2e fails, the document off whole device pixels (measured).

- [ ] **Step 6: Commit**

```
git commit -m "feat(app): W and H in the Move bar and an aspect lock the handles follow, as Compositor 1.4.5 has them" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/CanvasView.tsx app/src/panels/TransformInspector.tsx app/src/state/store.ts app/src/styles.css app/src/tools/transform-geometry.ts app/tests/e2e/perf-4-5.spec.ts app/tests/e2e/transform.spec.ts app/tests/unit/transform-geometry.test.ts
```


---

### Task 15: The README, the spec's stale format wording, and 0.6.0

**Files:**
- Modify: `README.md` (a Phase 4.5 section with its shortcuts; the 4b-1 note about format 11 goes; Phase 3.5's and 4a's sentences that 1.4.5 changed; the interoperability section), `docs/superpowers/specs/2026-09-20-windows-port-design.md` (section 5 said "reads 1 to 7 and writes 7"; a sentence says what superseded it), `Cargo.toml`, `Cargo.lock`, `package.json`, `src-tauri/tauri.conf.json` (0.6.0), `app/tests/e2e/smoke.spec.ts` (the version the start screen shows)

- [ ] **Step 1: The version test first**

```diff
--- a/app/tests/e2e/smoke.spec.ts
+++ b/app/tests/e2e/smoke.spec.ts
@@ -2,7 +2,7 @@ import { test, expect } from "@playwright/test";
 
 test("engine loads in the browser", async ({ page }) => {
   await page.goto("/");
-  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.5.0");
+  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.6.0");
   const ids = await page.evaluate(() => {
     const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
     api.engine.newDocument(10, 10, true);
```


Run: `npx playwright test app/tests/e2e/smoke.spec.ts -g "engine"` (after `pnpm wasm:dev`). Expected: fails, the page says "Compositor engine 0.5.0".

- [ ] **Step 2: The version, the README and the spec**

```diff
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -345,7 +345,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-engine"
-version = "0.5.0"
+version = "0.6.0"
 dependencies = [
  "i_overlay",
  "image",
@@ -359,7 +359,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-engine-wasm"
-version = "0.5.0"
+version = "0.6.0"
 dependencies = [
  "compositor-engine",
  "console_error_panic_hook",
@@ -371,7 +371,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-shell"
-version = "0.5.0"
+version = "0.6.0"
 dependencies = [
  "regex",
  "serde",
```

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -3,7 +3,7 @@ resolver = "2"
 members = ["engine", "engine-wasm", "src-tauri"]
 
 [workspace.package]
-version = "0.5.0"
+version = "0.6.0"
 edition = "2021"
 license = "MIT"
```

```diff
--- a/README.md
+++ b/README.md
@@ -90,9 +90,9 @@ Photoshop.
 - All 24 of the Mac's blend modes, in its menu order. New: Linear Burn, Linear Dodge (Add),
   Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, Exclusion, Subtract
   and Divide. As on the Mac, an adjustment layer in any mode but Normal blends its result at
-  full strength and keeps the alpha of what lies beneath it; in the eight modes the Mac computes
-  with Core Image, that blend is Normal. A clipped group in one of those eight composites as
-  Normal, as on the Mac.
+  full strength and keeps the alpha of what lies beneath it. (Until Phase 4.5 the eight modes
+  Compositor 1.2 computed with Core Image were drawn as Normal for an adjustment layer and a
+  clipped group, as 1.2 drew them; see Phase 4.5.)
 - Adjustment layers for Add Noise, Gaussian Blur, Motion Blur, Invert, Black & White and
   Color Balance, from Layer > New Adjustment, with panels (Invert has nothing to set). Blur
   layers blur everything beneath them, fading at the canvas edge as the Mac's do.
@@ -149,8 +149,9 @@ Photoshop.
   grows the layer where the selection reaches), and Levels and Curves show the histogram of the
   selected pixels. Delete clears the selected pixels, or fills a targeted mask with its background
   colour (white unless its edges are mostly black), growing it to cover the canvas as Compositor
-  1.3.7 for Mac does. Add Mask hides the selection (Add Mask (Hide All) shows only it) and uses it
-  up. The Crop tool starts at the selection's bounds. Adjustment layers ignore the selection.
+  1.3.7 for Mac does. Add Mask reveals the selection (Alt-click, or Add Mask (Hide All), hides it)
+  and uses it up (Phase 4.5; 1.2 hid it). The Crop tool starts at the selection's bounds.
+  Adjustment layers ignore the selection.
 - An empty selection (after Subtract or Contract) says so in the options bar, and every edit
   refuses it until it is deselected or replaced.
 - Selections are part of undo and, as on the Mac, are never saved in the project. Crop, Canvas
@@ -208,9 +209,6 @@ Photoshop.
 - Note: a shape is drawn as the Mac draws it (Core Graphics' anti-aliasing is approximated by exact
   area coverage); a scaled shape layer is not redrawn here.
 - Note: the colour picker works in sRGB, 8 bits a channel, as the Mac's does.
-- Note: Compositor for Mac 1.3 saves projects in format 11, which this version cannot open yet (it
-  opens formats 1-9); Mac projects saved by 1.2.x open as before. Opening and saving 1.3 projects
-  is the next update.
 
 ### Colour and tool shortcuts
 
@@ -223,6 +221,40 @@ Photoshop.
 | Gradient opacity | 1-9 for 10-90 %, 0 for 100 % |
 | Apply / cancel a pending gradient | Enter / Escape |
 
+## Phase 4.5: Compositor for Mac 1.4.5
+
+- Projects saved by Compositor for Mac 1.3 and 1.4 (format 11) open, and every save is written at
+  format 11. A text layer's coloured words and its words in another face are kept exactly as the
+  Mac saved them (this app has no text tool yet, and draws text as the Mac last drew it). A project
+  from a newer Mac says which formats this version reads.
+- Soft Light is drawn as Compositor 1.4.5 draws it (the W3C formula; 1.2 used another one, which
+  differs where the top layer is lighter than mid grey).
+- An adjustment layer, and a clipping mask's base layer, blend in their own mode in all 24 modes
+  (1.2 drew Linear Burn, Linear Dodge, Vivid Light, Linear Light, Pin Light, Hard Mix, Subtract and
+  Divide as Normal there).
+- Hue/Saturation raises saturation as Photoshop does: +50 doubles it, +100 takes any colour all the
+  way.
+- Add Mask with a selection reveals the selection; Alt-click on the mask button hides it.
+- Inverse of a selection that covers the whole canvas leaves nothing selected.
+- Layer > Ungroup Layers (Shift+Ctrl+G, or a folder's context menu): the folder's layers take its
+  place and the folder goes.
+- Drag a tab along the tab strip to reorder the open projects.
+- Resize handles snap the edges they move to the canvas and other layers, as a move does.
+- The Move bar has W and H fields and an aspect lock (on at first), which the handles follow too;
+  Shift turns it the other way while held.
+- A large edit through the background worker comes back ready to draw at the canvas's zoom, so the
+  frame after it no longer stalls (up to half a second at 100 megapixels before).
+- Note: the Mac's overflow menu for many tabs, showing a mask alone (Alt-click on its thumbnail) and
+  flipping a layer by dragging a handle past the opposite side are not in this version.
+
+### Phase 4.5 shortcuts
+
+| Action | Shortcut |
+| --- | --- |
+| Ungroup Layers | Shift+Ctrl+G |
+| Add a mask hiding the selection (or all black) | Alt-click the mask button |
+| Keep or free the aspect ratio while dragging a handle | Shift (turns the Move bar's lock the other way) |
+
 ## Prerequisites
 
 - Rust 1.95 with the `wasm32-unknown-unknown` target
@@ -252,9 +284,11 @@ pnpm build:portable   # build and package the portable Windows zip
 ## Project file interoperability
 
 Projects are `.comp` folder packages, compatible with Compositor for macOS. This app opens
-projects from Compositor for Mac 1.2.10 (format version 9) and every earlier format (1 to 9),
-writes version 9 as the Mac does, and saves them back without losing anything. What they
-contain is drawn as the Mac draws it, within the reduced-copy blur note above, except:
+projects from Compositor for Mac 1.4.5 (format version 11) and every earlier format (1 to 11),
+writes version 11 as the Mac does, and saves them back without losing anything: a text layer's
+colour and font runs come back exactly as the Mac wrote them, and the Mac's Quick Look preview
+inside a project is left out of a save (the Mac makes it again on its next save). What they
+contain is drawn as Compositor 1.4.5 draws it, within the reduced-copy blur note above, except:
 
 - a layer enlarged in High quality (the default), which Compositor for Mac draws with Core
   Graphics' high-quality filter and this app bilinearly: sharper soft edges on the Mac, up to 29
@@ -262,7 +296,11 @@ contain is drawn as the Mac draws it, within the reduced-copy blur note above, e
   filter (probe results, "Step probes");
 - layer effects, which match the Mac's renders (11 of the 16 effects probes exactly, 4 within 3
   levels, and the last apart from the resampling above) but are written as the Mac's code writes
-  them, not yet checked against a project the Mac itself saved with effects.
+  them, not yet checked against a project the Mac itself saved with effects;
+- text colour and font runs, checked so far against projects written by hand from the Mac's code
+  (and a real 1.4.5 save without text), until the user's Mac saves with text runs come back;
+- an upright layer placed at a fraction of a pixel and drawn at 100 %, which Compositor 1.4.5
+  copies pixel for pixel and this app still blends (Phase 3.5d).
 
 ## Further reading
```

```diff
--- a/docs/superpowers/specs/2026-09-20-windows-port-design.md
+++ b/docs/superpowers/specs/2026-09-20-windows-port-design.md
@@ -160,7 +160,7 @@ Task 2 ports that: one kernel for the filter, the adjustment layer and the GPU.
 
 ## 5. Project format on Windows
 
-The Windows app reads and writes the macOS folder package unchanged: a `<name>.comp` directory containing `manifest.json` and `images/<layer UUID>.png` plus `<layer UUID>.mask.png`. The macOS source (`ProjectStore.swift`) is ahead of `docs/project-format.md`: it reads versions 1 to 7 and writes version 7, where version 7 adds per-layer `adjustment`, `maskPlacement`, `maskLinked` and `shape` records. The Windows app reads 1 to 7 and writes 7. `shape`, whose feature arrives in a later phase, is preserved verbatim through open and save. `adjustment` is parsed since Phase 3: every field the Mac writes for its six kinds is read and written, but a key inside an adjustment that this version does not know is dropped on re-save, and an unknown `kind` makes the project refuse to open. Swift encodes the Hue/Saturation `adjustments` and `bands` maps (keyed by a `String` enum that is not `CodingKeyRepresentable`) as arrays of alternating key and value; the manifest reads that form (and the object form early 0.3.0 builds wrote) and writes it in `ColorRange` declaration order, so the order can differ from a Mac save of the same settings, whose order is hash-seeded. Validation rejects unsupported versions, invalid metadata, missing assets, unsafe paths and oversized data before the live document is replaced, exactly as `ProjectStore.swift` does. Round-trip tests open every fixture version and re-save it.
+The Windows app reads and writes the macOS folder package unchanged: a `<name>.comp` directory containing `manifest.json` and `images/<layer UUID>.png` plus `<layer UUID>.mask.png`. The macOS source (`ProjectStore.swift`) is ahead of `docs/project-format.md`: it reads versions 1 to 7 and writes version 7, where version 7 adds per-layer `adjustment`, `maskPlacement`, `maskLinked` and `shape` records. The Windows app reads 1 to 7 and writes 7. (Since superseded: Phase 3.5a read and wrote Mac 1.2.10's version 9, and Phase 4.5 reads 1 to 11 and writes 11, Compositor 1.4.5's format, keeping text layers' `colorRuns` and `fontRuns` verbatim.) `shape`, whose feature arrives in a later phase, is preserved verbatim through open and save. `adjustment` is parsed since Phase 3: every field the Mac writes for its six kinds is read and written, but a key inside an adjustment that this version does not know is dropped on re-save, and an unknown `kind` makes the project refuse to open. Swift encodes the Hue/Saturation `adjustments` and `bands` maps (keyed by a `String` enum that is not `CodingKeyRepresentable`) as arrays of alternating key and value; the manifest reads that form (and the object form early 0.3.0 builds wrote) and writes it in `ColorRange` declaration order, so the order can differ from a Mac save of the same settings, whose order is hash-seeded. Validation rejects unsupported versions, invalid metadata, missing assets, unsafe paths and oversized data before the live document is replaced, exactly as `ProjectStore.swift` does. Round-trip tests open every fixture version and re-save it.
 
 JSON encoding follows the structure of Swift's Codable output so either app parses the other's manifest: UUIDs are uppercase hyphenated strings, `origin` is a two-element array `[x, y]`, `size` is `[width, height]`, enum values are their display strings (`"High quality"`, `"Color Dodge"`), absent optionals are omitted, keys are sorted and the JSON is pretty-printed. Whitespace need not match byte for byte.
```

```diff
--- a/package.json
+++ b/package.json
@@ -1,7 +1,7 @@
 {
   "name": "compositor-windows",
   "private": true,
-  "version": "0.5.0",
+  "version": "0.6.0",
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
-  "version": "0.5.0",
+  "version": "0.6.0",
   "identifier": "com.compositor.windows",
   "build": {
     "frontendDist": "../app/dist",
```


The spec's section 5 is not rewritten: its sentence stays as the record of what Phase 1 decided, and one added sentence says that 3.5a moved to version 9 and this phase to 11 (spec decisions win; this is a fact the spec had not caught up with, flagged to the controller).

- [ ] **Step 3: Run the whole set**

`cargo test -p compositor-engine`: 576 passed, 10 ignored (72 binaries). `cargo test -p compositor-shell`: 9 passed. `pnpm wasm:dev`; `pnpm test`: 256 in 38 files; `pnpm build` clean; `pnpm e2e`: 172 passed, 26 skipped (the eight perf cases skip without PERF). Release wasm (`pnpm wasm`): 2,989,215 bytes (0.5.0 was 2,971,316).

- [ ] **Step 4: The whole timing file, in one run**

`pnpm wasm`, then `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4-5.spec.ts`, then `$env:PERF = ""` and `pnpm wasm:dev`. Every case has passed alone in its task; this run proves they pass together. Measured on the scratch clone, all eight cases passed in 2.5 min, every one on the HD 520 (UNMASKED_RENDERER_WEBGL "ANGLE (Intel, Intel(R) HD Graphics 520 (0x00001916) Direct3D11 vs_5_0 ps_5_0, D3D11)"): F1's frame after 9-24 ms, `jobInput` 82 / 335, `installJob` 58-97 / 374-401, gaps 18-29; the 1.4.5 results' frames 29-45 and Saturation steps 30-37; Add Mask's step 202-240 / 656-683 and frame after 21-22 / 70-77; Inverse 1-2 and 4-8; Ungroup 2 and 12-15; the tab drag 22 and 18-21; the resize tick 6-7 and release 4; the typed W's Enter 2 and frame 15 / 27 (24 MP / 100 MP where they differ). Every budget held; the closest were the typed W's 100 MP frame (27 of 33) and Add Mask's 100 MP step (683 of 1000).

- [ ] **Step 5: Introduce a bug and watch the test fail (measured)**

Leave `smoke.spec.ts` at 0.5.0 with the workspace at 0.6.0: the smoke test fails, the page says "Compositor engine 0.6.0" (measured).

- [ ] **Step 6: Commit**

```
git commit -m "docs: Phase 4.5 (catch-up with Compositor for Mac 1.4.5) in the README, the spec's format versions, and 0.6.0" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- Cargo.lock Cargo.toml README.md app/tests/e2e/smoke.spec.ts docs/superpowers/specs/2026-09-20-windows-port-design.md package.json src-tauri/tauri.conf.json
```


The controller then builds the portable zip (`pnpm build:portable`) and checks the new code is in it (Global Constraints: never run it here).

---

## Self-review

**Spec coverage** (spec section 3, the decisions of 2026-09-29 and 2026-09-30, and the brief):
- The 4b-1 Mac probes pinned: Task 1 (ruling OQ1). The Phase 4.5 probes and hand steps (about 20 exports: text runs, a re-saved rich project, the mask reveal, the six earlier probes, blend-mode and saturation probes): Task 2 and the exports list above (ruling OQ2).
- F1 (the worker returns the display level; no UI-thread halving of a large result; frame-after back under 350 / 400 ms at 100 MP): Task 3 (rulings OQ3, OQ4). The fix wave's residuals (chip click while working, `selectLayers` and the job preview, `jobs.warm()` order, effects-image failures): Task 4.
- Format 11 (read 1-11, runs kept verbatim, write 11, the version wording, hand-written fixtures marked, QuickLook ignored and not written): Task 5 (ruling OQ5).
- The 1.4.5 results with CPU and GLSL tests: Soft Light, Task 6; real modes for adjustment layers and stack bases, `cg_mode` removed, Task 7; positive saturation, Task 8; Add Mask reveals with Alt for the opposite (the 2026-09-30 ruling), Task 9; Inverse of everything, Task 10. The 1.2.10 pins these contradict are rewritten and marked awaiting B1: Tasks 6-7 (no saturation or mask pin existed).
- Small behaviours: Ungroup Layers, Task 11; tab dragging, Task 12; resize snapping, Task 13; W / H, Task 14; "Shift keeps moved pixels on a straight line" ruled out of this phase (OQ15).
- Docs, README, 0.6.0: Task 15.
- LL-073: every path this plan adds or changes has a release timing at 24 and 100 MP with an asserted budget in `perf-4-5.spec.ts` (ruling OQ16), or a ruling saying why not (OQ17).

**Placeholder scan.** No "TBD", "TODO", "later" or "similar to Task N" stands for code: every code block is generated from the scratch clone's tags t1-t15 (`git show` for a new file, `git diff` for a changed one), so every line in them compiled and passed there. Numbers in the prose are the scratch clone's measurements and say so.

**Type and name consistency.** `Raster::adopt` / `adopted` / `reduced` (Task 3) are what `seed_adopted`, `layer_raster` and the compositor call. `run_edit_job(input, pixels, mask, points, command, out_per_doc)` returns four things everywhere (engine, wasm, `jobs.rs` tests, `display_level.rs`, `mask_grow.rs`, `raster_edits.rs`), and `install_job` takes `display` last in Rust, wasm and `client.ts`. `CURRENT_VERSION` is 11 in every test that pins a written version (Task 5 replaced the literal 9s). `adjusted_saturation` (Rust) and `adjustedSaturation` (GLSL) have the same four branches. `Command::UngroupLayers { id }` matches `{ type: "UngroupLayers"; id }` in `types.ts`. `reorderedTabs` is the only order change (`moveTab` calls it). `SessionInit.lockRatio` (Task 13, default true) is what Task 14 sets from `locksTransformRatio`; `snappedResizePoint` gets `lockRatio !== shift` as `proportional`, as the Mac passes `locksTransformRatio != shift`.

**LL-068, per test file** (the bug each is shown to catch, all measured on the scratch clone):
- `mac_4b1_probes.rs`: colour truncation in `paint_grid`; a rounded rectangle's corner constant; a line's half-width (Task 1).
- `mac_probes.rs`: a README name, an unclipped stack child, swapped Hue/Saturation arguments (Task 2); A5's version is held by its own test (Task 5).
- `display_level.rs`, `store-jobs.test.ts`, `jobs.spec.ts`: `reduced` ignoring the adoption, `seed_adopted` doing nothing, the region check dropped, `outPerDoc` 0, no halving sent (Task 3).
- `gradient-store.test.ts`, `effects-images.test.ts`, `smoke.spec.ts`: the four residuals put back one at a time (Task 4); the version literal (Task 15).
- `format_v11.rs` and the version pins: the gate, UTF-16 counting, U+2028, the wording (Task 5).
- `blend_modes_v9.rs`, `mac_1_2_10.rs`, `mac-1.2.10.spec.ts`: Pegtop back, one-branch W3C, GLSL mode 15 (Task 6); the Normal mapping for adjustments and stacks on the CPU and the GPU (Task 7).
- `spatial_adjustments.rs`, `modes-1-4-5.spec.ts`: the same Task 7 bugs.
- `hsv.rs`, `adjust-render.spec.ts`: the old multiply on the CPU and in GLSL, the +100 branch (Task 8).
- `selection_masks.rs`, `selection.spec.ts`: 1.2.10's tones, the Hide name, the footer ignoring Alt (Task 9); Inverse keeping an empty selection, twice (Task 10).
- `selection_commands.rs`: Task 10's two bugs.
- `ungroup.rs`, `ungroup.spec.ts`, `keymap.test.ts`: the children appended at the top, no clip release, no selection, the key unmapped (Task 11).
- `tab-reorder.test.ts`, `tabs.spec.ts`: the translation ignored, the commit's off-by-one, a drag while working, a selectable title (Task 12).
- `transform-session.test.ts`, `transform.spec.ts`: both edges snapping, the far edge, the turned-layer guard (which the Mac's own single point did not catch; the sweep does), no targets for a resize (Task 13); the lock ignored, a centre resize, the handle ignoring the lock, Shift not shown (Task 14).
- `transform-geometry.test.ts`: Task 14's `resizedTo` bugs.
- `perf-4-5.spec.ts`: each case asserts that the path did what it times (the halving sent, the snap made, the tab moved, the selection gone, the folder gone, the width typed) besides its budgets, so a case that times nothing fails.
