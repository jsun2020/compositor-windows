# Phase 4b-1: Groundwork, Colour, Fills, Gradients and Shapes - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compositor for Windows gets the groundwork Phase 4b's painting needs and the first tools that paint, as Compositor for Mac 1.2.10 has them. Groundwork (Tasks 2-9): the history is capped at the Mac's 100 entries and 256 MiB and identifies its entries; an edit that knows where it changed a layer reports that rectangle, and the GPU uploads only it (layers and masks); a job worker edits large layers, reads their histograms and makes their effects images off the interface thread; Fill and the Gradient paint as the Mac's raster edits do; gradients preview from a reduced copy while dragged and as a full-size patch inside a small selection. Colour and tools (Tasks 10-16): shapes on new layers with the Mac's shape record; the palette (X, D, black and white on a mask) with its swatches at the foot of the tool rail; the colour picker (saturation / brightness field, hue strip, R, G, B, hex, sampling the canvas while open), also for Gradient Map's two ends; the Eyedropper (I); Fill with the foreground or background colour (Alt+Backspace, Ctrl+Backspace); the Gradient tool (G: Linear or Radial, Foreground to Background or to Transparent, Reverse, Opacity, a pending line with draggable ends, Shift 45 degrees, masks); the Shape tool (U: Rectangle with a corner radius, Ellipse, Line); new Gradient Map layers from the palette. Task 1 writes the Mac probes these tools need; Task 17 documents it all and makes the version 0.5.0.

**Architecture:** The engine keeps owning every pixel. `History` gains a cap with incremental accounting of the bytes only history holds, entry ids and a saved-state token (`engine/src/history.rs`). A `Dirty` gains engine-side `regions` (`command.rs`: `Region { layer, plane, rect }`); `Engine::edit` records them in a revision lineage (`engine/src/lineage.rs`, a ring of 256 changes walked by `delta`) and seeds the new raster's halvings from the old one's outside the rectangle (`Raster::seed_halvings`), so the app asks `pixels_delta` / `mask_delta` by revision and `texSubImage2D`s only what changed (`layer-textures.ts`, `mask-textures.ts`). Jobs (`engine/src/jobs.rs`) copy one layer, its selection and the canvas size out of the document into a second engine inside a module worker (`app/src/engine/job-worker.ts`, driven by `JobClient` in `jobs.ts`); a result is put back as one undo step only if the layer's stamp is unchanged. Fill and the Gradient are `ops/raster_edit.rs` (`Paint`, `paint_layer`), previews of the Gradient are `PreviewRequest::Gradient` with three targets (`PreviewTarget::{Pixels, Patch, Mask}`, `preview.rs`), and a patch reaches the GPU through `Engine::layer_region`. Shapes are `ops/shape.rs` (`ShapeSpec`, rasterised by the selection's exact-area rasteriser). The app adds `tools/color.ts` (the palette's colours and the picker's HSB), `state/gradient-edit.ts`, `tools/shape-draft.ts`, three tools, their bars, the picker panel, the rail's palette, and `canvas/{sampling,gradient-tool,shape-tool}.ts`, each a pointer handler installed by `CanvasView`, drawing on the overlay only.

**Tech Stack:** Rust (`engine`, `engine-wasm`), serde / serde_json, TypeScript + React 18 + zustand (`app`), WebGL2, a module Web Worker, a 2D overlay canvas, vitest, Playwright (the installed Edge for GPU timings).

**Spec:** `docs/superpowers/specs/2026-09-20-windows-port-design.md`, section 3, Phase 4 ("4b Retouching and colour"), and the Phase 4b architecture and scope note of f032c39 with the user's decisions.

**Oracle:** Compositor 1.2.10, `C:\Users\sr9rfx\.claude-project\Compositor-1.2.10\Compositor` (sources) and `...\CompositorTests` (tests). Citations without a path are in that tree: `DocumentHistory.swift`, `ColorPalette.swift`, `ColorPickerSheet.swift`, `ColorPaletteControls.swift`, `Gradient.swift`, `GradientControls.swift`, `ShapeTool.swift`, `ShapeControls.swift`, `SelectionEdits.swift`, `BrushStroke.swift`, `EditorSession+Brush.swift`, `EditorCanvas.swift`, `TransformOverlay.swift`, `Filters.swift`, `LayerAdjustment.swift`, `CompositorApp.swift`. Do NOT read `C:\Users\sr9rfx\.claude-project\Compositor` (an older checkout).

**Pixel oracles:** the Mac's own tests ported with their numbers (HistoryTests' cap cases, GradientTests, ColorPickerTests, ShapeToolTests), the formulas the Mac's code states (source-over at the paint's alpha times opacity times coverage; the picker's HSB; the sampler's `(min(a, v) / a * 255).rounded() / 255`), an independent area count for every shape edge, and four new Mac probes (Task 1) that the user exports by hand; their pins land in `engine/tests/mac_1_2_10.rs` once the exports are back (Task 1, "Where the pins land").

**Measured for this plan (2026-09-28).** Every number below that is not quoted from the Mac was measured, not assumed. The code was written on one scratch copy of this repository (`C:\Users\sr9rfx\AppData\Local\Temp\claude\p4b1-scratch`, its own target directory) one task at a time and committed there at tags `t1` to `t17` over `t0` = f032c39; at each tag the whole engine suite, vitest, `pnpm build` and the whole e2e suite passed (the counts are in each task's Step 4). Every Rust and TypeScript block below is copied from those tags by a script, not retyped: a diff block is `git diff t<N-1> t<N>` for its file and a new file is `git show t<N>:<path>`. Timings are the release wasm (`pnpm wasm`) in the installed Edge (its GPU, as WebView2 uses it) at a 1440 x 900 viewport, or native release builds where a task says so; the machine is this project's Windows 10 (19045) workstation, as for every earlier phase. Not compiled: the README text (Task 17); not run: `pnpm build:portable`; the Mac probes are generated and checked for their contents, not yet compared (the user exports them).

**Base:** `phase4b` at f032c39. Leave every `sampling-*` Mac export in `engine/tests/fixtures/mac-1.2.10-probes/` untracked (ten pairs: seven sampling probes and three step probes); they belong to Phase 3.5d.

**Out of scope (Phase 4b-2, 4c, or not ported):** the clipboard (copy, cut, paste, paste in place); moving selected pixels (Ctrl-drag, Ctrl-arrows) and Transform Selection; the brushes (Brush, Eraser, Smear, Clone Stamp, Spot Healing; their tips and keys); tiled pixel storage; the Type tool and recolouring live text with a fill; redrawing a shape layer when it is scaled here (the Mac redraws it; this app keeps its pixels, and the record goes to the Mac intact); Canvas Size's Foreground / Background extension choices (the sheet keeps its colour input); a shortcut editor.

## Global Constraints

- ASCII only in every source file, test, doc and commit message.
- LL-073: every interactive path this plan adds or changes has a RELEASE timing at 24 MP (6000 x 4000) and at 100 MP (10000 x 10000), measured in the release wasm in Edge (`app/tests/e2e/perf-4b1.spec.ts`, run with `$env:PERF = "1"` after `pnpm wasm`) or natively (`engine/tests/perf_4b1.rs`, `--ignored`), and a written budget the test asserts. A path without a budget needs a recorded ruling saying why (ruling OQ20 lists them). Timings come only from `pnpm wasm`, never `pnpm wasm:dev`; rebuild `pnpm wasm:dev` afterwards.
- The Mac's numbers, exactly: history 100 entries and 256 MiB (`DocumentHistory`); the shortest gradient line 0.5 px; a gradient's handles grab within 10 view px; Shift snaps to eighths of a turn; the picker's field is 256 px with hue from 360 at the top to 0 at the bottom; the sample ring is a 116 pt box with a 24 pt grey (0.45) ring and 16 pt halves; the Shape tool's Radius slider reaches 200 and Width 100, their fields 5000; a new shape is named "<Kind> <n>" skipping names in use; a line's box is its ends grown by half its width; a shape under 1 px on a side makes nothing; the default line width is 4, the default gradient Linear, Foreground to Transparent, opacity 100 %.
- Rounding: every channel the engine paints is rounded half up in f64 (`(v + 0.5) as u8`); Swift's `rounded()` (halves away from zero) is ported as `Math.sign(v) * Math.round(Math.abs(v))` wherever a negative half can occur.
- `DocumentState` gains `undoEntryId: number | null` (Task 2). Every TypeScript `DocumentState` literal in the unit tests gains it (nine files, Task 2).
- The engine's `state()` reports a previewed layer at its preview's size, which may be a reduced copy: anything deciding by the stored layer's size asks `Engine::stored_pixels` (Task 15 found this).
- Tests: for EVERY assertion, be able to name the production change that makes it fail. Compute expected values in the test from the formula or take them from the Mac test being ported; never paste them from a passing run. Measured bounds are the exception, and each names its measurement. Prefer asymmetric fixtures. Each task's Step 5 introduces a bug, watches the test fail, and reverts (LL-067, LL-068).
- E2E: prove commits with `undoDepth` against a baseline, never `canUndo`; drive the canvas with real pointer events at points computed from the viewport, never hard-coded screen offsets; read pixels through the engine's composite or the overlay canvas.
- tsconfig is three programs (`app/src`, `app/tests/unit`, `app/tests/e2e`), each with `noUnusedLocals`.
- Build and test from the repository root, in the FOREGROUND with long timeouts, PowerShell 5.1 (no `&&`; chain with `;`): `cargo test -p compositor-engine` (timeout at least 900 s); after any change under `engine/src` or `engine-wasm`, `pnpm wasm:dev` before `pnpm build` and `pnpm e2e`; `pnpm test`; `pnpm build`; `pnpm e2e` (server 127.0.0.1:1420, one Playwright run at a time). Do not use `2>&1` on native executables; redirect with `*> file` and read the file.
- Step 5 (introduce a bug, watch the test fail, revert) runs BEFORE the task's commit, so never revert with `git checkout -- <file>`. Instead, before adding the bug, `Copy-Item <file> <file>.bak`; to revert, `Move-Item -Force <file>.bak <file>` and then `(Get-Item <file>).LastWriteTime = Get-Date` (a restored file must get a new modification time or cargo keeps the build with the bug in it); then re-run the task's tests and confirm they pass again.
- Do NOT run `pnpm build:portable`: the controller builds the zip.
- Commit with an explicit pathspec: `git add -- <new files>` first, then `git commit -m "<subject>" -m "<Co-Authored-By line>" -- <paths>`. Every `-m` comes BEFORE `--`, because git reads everything after `--` as a path. End every commit message with the Co-Authored-By line your own instructions give, as its own `-m` paragraph; the Step 6 commands below show the line for Claude Opus 5.5 (1M context), and an implementer on another model writes its own.
- Report actual counts at every Step 4: the engine total is the sum of "passed" (and of "ignored") over every test binary `cargo test -p compositor-engine` runs, not one binary's line.
- Baseline before Task 1, measured at f032c39 on the scratch copy: `cargo test -p compositor-engine` 459 passed and 4 ignored (60 test binaries); vitest 148; `pnpm build` clean; e2e 131 passed and 4 skipped. Task 1 Step 1 records them on the real repository before any change.

## Rulings made for this plan (OQ)

Each was ruled and measured here, as the brief asked; "cost if wrong" is what changing it later touches. Timings are the final code's (the tree Task 17 commits), release wasm in Edge unless marked native; the whole perf suite (`perf-4b1.spec.ts`, 8 tests) passed every budget on that run.

1. **OQ1 The history cap is the Mac's: 100 entries and 256 MiB of pixels only history holds, the oldest undo entry dropped first, then the farthest redo entry, after every push, undo and redo - even the entry just made** (DocumentHistory.swift:28 and its trim). The bytes are counted incrementally: a map of every buffer the entries reach (its size and how many entries reach it) is kept up to date as entries come and go, so a trim costs the entries it drops, not every layer of every entry; a buffer the current document shares costs nothing. An edit to a layer larger than 256 MiB applies and cannot be undone, as on the Mac (measured: 100 MP inverted 20 times keeps 0 entries and the wasm heap at 1131 MB; at 24 MP 2 entries and 368 MB). A rename at the cap with 1000 layers: 3.1 ms mean (budget 25 ms; natively 1.5-2.4 ms, the trim alone 0.16 ms). History drops the halvings of rasters only it holds, so memory is not kept for pyramids nobody draws. Cost if wrong: `history.rs` alone.
2. **OQ2 Entries have ids and the document a state token.** `DocumentState.undoEntryId` names the entry an undo would take back; the transform's Alt-drag cancel compares ids, not depths, because at the cap a push trims the oldest entry and the depth does not move (Task 2's e2e is exactly that case). The saved state is a token (the Mac's revision UUIDs), so trimming the front never loses "saved". Cost if wrong: `history.rs`, `types.ts`, the store's `TransformEdit`.
3. **OQ3 Changed rectangles travel by revision.** A command that knows where it changed a buffer reports `Region { layer, plane, rect }` in `Dirty.regions` (engine-side, never serialised); `Engine::edit` records every buffer whose revision moved in a lineage of the last `LINEAGE_LIMIT = 256` changes, with its rectangle or none; undo, redo and revert record the reverse of the change that joined the two revisions. The app asks `pixels_delta(layer, from_revision)` / `mask_delta`: the union of the rectangles from its texture's revision to now, empty when nothing changed, null (upload whole) when any step had no rectangle or fell out of the ring. Only selection-limited edits report rectangles now (Clear, Invert, adjustments and filters inside a selection, Fill and the Gradient); every other command uploads whole as before. Cost if wrong: `lineage.rs`, the commands' `within(...)`.
4. **OQ4 A new raster inherits its predecessor's halvings outside the changed rectangle** (`Raster::seed_halvings`): at a reduced zoom the next frame halves only the rectangle's blocks, and uploads only them. With the partial upload, the frame after a clear in a 1024 px selection: 9 ms at fit and 17 ms at 1:1 at 24 MP, 4.3 and 11.7 ms at 100 MP (budget 33 ms; a whole-layer Invert's frame, for comparison: 72-93 ms and 404-491 ms). The halving itself became a single pass per level (natively 22 ms to level 3 at 24 MP, 100 ms at 100 MP). Cost if wrong: `raster.rs`, `layer-textures.ts`.
5. **OQ5 Large layers go to a job worker: over `JOB_PIXELS = 4,000,000` stored pixels.** A job copies one layer (pixels, mask), its selection and the canvas size into a second engine in a module worker; the result goes back as one undo step only if the layer's stamp (revisions, transform, mask placement) is unchanged, else it is refused with "The layer changed while the edit was being made, so it was not applied." One job runs at a time; a newer job on the same channel supersedes a waiting one and discards a running one's result; a worker whose memory passed 1 GiB, or that died, is replaced. While an edit job's result is to come the store's `working` flag makes commands, undo, redo, file actions and the palette wait ("Wait for the current edit to finish."). Jobs serve destructive adjustment and filter commits, Levels and Curves histograms of pixel layers, effects images, Fill and the Gradient. Measured (Levels): opening the panel 102 / 365 ms on the UI thread (24 / 100 MP), the longest frame gap while the worker reads the histogram 22 / 17 ms, the copies out and back 65-81 ms at 24 MP and 291-380 ms at 100 MP (budgets 150 and 500; four runs measured 66-108 and 264-380), the longest gap while the worker edits 17 ms (budget 100). Not in the worker: an adjustment layer's histogram (a composite of everything under it; it stays synchronous) and Invert (one pass, already fast). A job's document holds the one layer, so the project's pixel budget is checked there without the other layers, and `install_job` checks the stamp, not the budget (disclosed; see OQ20). Cost if wrong: `jobs.rs`, `jobs.ts`, the store's `usesJob`.
6. **OQ6 Effects images are made off the UI thread past 65,536 padded pixels:** first the layer drawn plainly, then an image of the layer reduced to at most 1536 px on its longer side, then (up to 24 MP of layer pixels) the full-size image, which the worker makes and the engine keeps in its effects cache (`keep_effects_image`) so exports and the CPU compositor find it as if the engine had made it. Measured: at 24 MP the first frame 21 ms (plain), the reduced image after 2.1 s with no frame gap over 33 ms, the full image after 14.6 s with gaps under 83 ms (budget 150); at 100 MP the reduced image after 1.7 s, the longest gap 167 ms (budget 200: the first frame halves the plain 100 MP layer three times, natively 100 ms; three runs measured 133-167 ms). Cost if wrong: `effects-images.ts`, the renderer's hooks.
7. **OQ7 Fill and the Gradient paint as the Mac's raster edit does** (BrushStroke.swift:153-160, :645-675; EditorSession+Brush.swift:154-188): the layer's grid grows to cover the canvas as the layer maps it, rounded out to whole pixels; each pixel is painted at its centre, inside the canvas, source-over at the paint's alpha times the opacity times the selection's coverage (`k / 255`), each channel rounded half up; the result is trimmed to the pixels left, and a mask covering the layer follows it, white where the layer grew. A mask is painted in its own grid in grey. The Gradient is the Mac's: linear along the line or radial from the start with the end on the rim, clamped before the start and after the end, a line under 0.5 px paints nothing, and the colours are the palette's (Foreground to Background, or to the foreground at alpha 0, reversed on request). Measured natively: a gradient over the whole layer 566 ms at 24 MP and 2437 ms at 100 MP; a fill in a 2000 x 1500 ellipse 110 / 356 ms; through the worker the page's frames never gap past 17 ms (commits done after 1.5 s / 5.6 s for the Gradient, 2.2 s / 5.9 s for Fill). Cost if wrong: `raster_edit.rs`.
8. **OQ8 Gradient previews:** while the line is dragged, from the grown grid reduced to at most `GRADIENT_DRAG_LIMIT = 1024` px on its longer side (drag tick 25-31 ms, engine plus frame; budget 50); once the pointer is let go, at most `GRADIENT_SETTLED_LIMIT = 2048` (78-95 ms; budget 150). 4096, the colour adjustments' settled limit, measured 355-426 ms and was ruled out: the settled preview runs once per release and must not hold the page. Inside a selection whose rectangle holds at most `PATCH_LIMIT = 1 << 19` pixels (about 724 x 724), on a layer that already covers the canvas and draws no effects, the preview is the full-size patch alone (31-33 ms a tick; budget 50; 1 MP measured 57-60 ms and was ruled out), which the GPU takes as that rectangle through `Engine::layer_region`. A mask gradient previews the mask reduced to the same limits. Cost if wrong: `preview.rs`'s three constants.
9. **OQ9 Which layers are "large" is the stored layer's size** (`Engine::stored_pixels`). `state()` reports a previewed layer at its preview's size, which may be a reduced copy; Task 15's perf run found a 24 MP Gradient committed on the UI thread for that reason (Return 789 ms, 3.2 s at 100 MP). Cost if wrong: none known.
10. **OQ10 The Gradient's pending edit follows the Mac** (Gradient.swift, EditorSession.swift:573-590, EditorCanvas.swift): Return or Apply commits, Escape or Cancel drops, the first Undo discards it (a Redo drops it too), a change of tool, layer, target or document applies it, a new line on the same target replaces it, a press within 10 view px of an end drags that end, Shift holds the dragged end to eighths of a turn about the other, the settings and the palette re-render it, and digits set its opacity (at least 1 %). Where the Mac refuses other layer edits while a gradient is pending (`canEditLayers`), this port applies the gradient first and then runs the command, as it already does for a pending transform. Alt with the Gradient tool samples the foreground (`palettePicking`). Cost if wrong: the store's `run`, `commitGradient`.
11. **OQ11 Shapes are drawn by the selection's exact-area rasteriser** from Core Graphics' outlines: the ellipse's four kappa Beziers, a rounded rectangle's four quarter-circle Beziers (radius at most half the shorter side), a line as a capsule with round caps (width at least 1), each flattened within 0.01 px. Every pixel of a 37 x 23 ellipse, a rectangle rounded by 9 and a 5 px line at an odd angle is within 4 levels of an independent 32 x 32 area count. The layer is `Int(width)` x `Int(height)` pixels at the box's origin (fractional for lines), named "<Kind> <n>", inserted where New Layer inserts (above the active layer, inside the active folder: the port's convention; the Mac inserts right after the active row), keeping the selection; one undo step named for its kind. Its `shape` record is the Mac's `LayerShapeStyle` as Swift's JSONEncoder writes it (whole numbers without a fraction; a line's ends as fractions of its box), so the Mac redraws the shape when it is scaled there; any other pixel edit drops the record (`set_pixels`), as the Mac's `liveShape` stops matching. A shape commits on the UI thread, as an import does: over the whole canvas 706 / 353 ms at 24 MP (rectangle, the first after the wasm memory grows; ellipse) and 1318 / 937 ms at 100 MP (budgets 1000 and 2000; natively 98-119 ms and 423-431 ms). Cost if wrong: a "shape" job kind, `ops/shape.rs`.
12. **OQ12 The palette is session state** (not saved; the Mac keeps it per window), with the mask's own black and white (`maskPaintWhite`); X and D and the swatches act on whichever is showing; the palette does not change while a job's result is to come (`canEditPalette`). Cost if wrong: the store's palette.
13. **OQ13 The colour picker floats** (a fixed-position panel dragged by its title bar), opens first centred on the canvas and then where it was last left, takes Enter as OK and Escape as Cancel ahead of every other key handler (so Escape over an open Gradient Map panel closes only the picker), samples the canvas under a press or drag while open whatever the tool, and snaps its working colour to 8 bits. It edits the two palette swatches and Gradient Map's two ends (previewed live; Cancel restores). A new Gradient Map, as a layer or from the Image menu, starts from the foreground to the background (LayerAdjustment.swift:191, Filters.swift:490). Cost if wrong: `ColorPickerPanel.tsx`, the store's picker.
14. **OQ14 Sampling reads the stored document's composite** (`Engine::sample_color`, the Mac's rounding `(min(a, v) / a * 255).rounded() / 255`, nothing off the canvas or over a transparent pixel), not an open preview; the Eyedropper sets the image's foreground even while a mask is the target, as the Mac does; a sample and its ring cost 4.6 ms at worst at 24 MP and 0.9 ms at 100 MP (budget 16). Cost if wrong: `sample_color`.
15. **OQ15 Fill's keys are Alt+Backspace / Alt+Delete (foreground) and Ctrl+Backspace / Ctrl+Delete (background)**, the Mac's Option-Delete and Command-Delete, and Edit > Fill with Foreground / Background Color. Delete with a selection on a targeted mask now fills it with the mask's background colour (Phase 4a filled it white; white is still the default), as the Mac's `clearSelectedPixels` does. Fill needs what the Mac's `canPaint` needs, and no crop rectangle pending. Cost if wrong: `actions/layers.ts`.
16. **OQ16 Tool rail icons.** Eyedropper is Lucide's `pipette` (lucide-static 1.48.0, the SF `eyedropper`'s nearest); the Gradient (SF `square.bottomhalf.filled`) and the Shape (SF `square.on.circle`) have no Lucide counterpart and are drawn for this port in Lucide's style, as the polygonal lasso was. The palette sits under the tools with 12 px above it, as the Mac's does. Cost if wrong: `tool-icons.tsx`.
17. **OQ17 Mac probes.** Four projects join `engine/tests/mac_probes.rs` (Task 1): `shapes.comp` (a 101 x 61 ellipse, a 120 x 80 rectangle rounded by 20, a 150 x 40 pill asking 5000, lines 1, 4 and 15 px wide), each shape layer a 1 x 1 placeholder with the shape's record and box, which the Mac redraws when the user presses Cmd-T then Return on it; `gradient-linear.comp`, `gradient-radial.comp` and `gradient-over-colour.comp`, blank or filled canvases with guides, on which the user draws the gradients by hand as the README says (the Mac's gradients cannot be written into a project). The exports come back as `<name>.mac-1.2.10.png` with the saved `.comp` beside them. Where the pins land: `engine/tests/mac_1_2_10.rs`, in the follow-up that commits the exports - the shapes compared with this port's `shape_raster` at the saved boxes (expected within the 4 levels of OQ11 on edges, exact elsewhere), and each gradient fitted from its saved endpoints (read from the export: the line is not saved) and compared with `paint_grid` (expected within 1 level). Not compared in this plan: the exports do not exist yet.
18. **OQ18 Version 0.5.0**, in `package.json`, `src-tauri/tauri.conf.json`, the workspace `Cargo.toml`, `Cargo.lock` and the smoke test's literal; the README gains a Phase 4b-1 section. The controller builds the zip.
19. **OQ19 Not ported, disclosed:** redrawing a shape layer when it is scaled here, and the Mac's live preview of a rounded rectangle while it is scaled; recolouring a live text layer with Fill (no Type tool yet); Canvas Size's Foreground / Background extension choices; the Mac's picker for layer effects' and Vignette's colours (the effects panel and Vignette are not ported).
20. **OQ20 Paths without a budget, and why:** the Mac probes (not interactive); opening the picker, the swatches and the bars (no pixels touched); a job's project-budget check without the other layers (a correctness gap, not a timing: a Fill or Gradient that grows a layer while other layers hold most of the 100 MP can be accepted by the worker where the UI thread would refuse; cost if wrong: send the other layers' pixel count in `JobInput` and check it in `image_grid`).
21. **OQ21 A shared engine-wide revision counter is per engine.** A job's engine issues its own revisions; `insert_document` keeps the document's revisions as they are, so a job's result is never compared by revision across engines, only by the stamp read from the UI engine (Task 5's test builds its document in the engine that installs it). Cost if wrong: `jobs.rs`'s stamp.
22. **OQ22 Wasm size:** the release wasm grows from 2,571,151 bytes at f032c39 to 2,913,123 bytes after Task 17 (+341,972, +13.3 %), mostly jobs' entry points, raster edits, the gradient's previews and shapes.

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `engine/tests/mac_probes.rs` | Modify | Four Phase 4b-1 probe projects and their README section (Task 1) |
| `engine/src/history.rs` | Rewrite | The cap (100 entries, 256 MiB of pixels only history holds), entry ids, the saved-state token |
| `engine/src/raster.rs` | Modify | `buffer_id`, `forget_halvings`; `PixelRect`, a faster halving, `seed_halvings` |
| `engine/src/lineage.rs` | Create | The revision lineage: changed rectangles by revision, `delta` |
| `engine/src/command.rs` | Modify | `Plane`, `Region`, `Dirty.regions`; `Fill`, `Gradient`, `AddShape` |
| `engine/src/engine.rs` | Modify | History limits, lineage, `pixels_delta`, `mask_delta`, jobs' hooks, previews through `render_document` / `render_bytes`, `layer_region`, `stored_pixels`, `sample_color`, the new commands |
| `engine/src/selection/coverage.rs`, `selection/geometry.rs` | Modify | `SelectionClip::rect_on_grid`; `cubic` for shapes |
| `engine/src/jobs.rs` | Create | `JobInput`, `JobOutput`, `LayerStamp`; edit, histogram and effects jobs; `install_job`, `keep_effects_image` |
| `engine/src/effects/mod.rs` | Modify | `EffectsCache::contains` / `insert` |
| `engine/src/ops/raster_edit.rs` | Create | Fill and the Gradient: `GradientSpec`, `Paint`, `image_grid`, `paint_grid`, `paint_layer` |
| `engine/src/ops/shape.rs` | Create | `ShapeSpec`, `shape_raster`, `shape_record`, `add_shape` |
| `engine/src/ops/layers.rs`, `ops/mod.rs`, `ops/adjust.rs`, `compositor.rs`, `lib.rs` | Modify | `insert_above_active`; registration; `placed_like` shared; a row-wise `alpha_bounds` |
| `engine/src/preview.rs` | Modify | `PreviewRequest::Gradient`, `PreviewTarget`, the drag / settled / patch limits |
| `engine-wasm/src/lib.rs` | Modify | Deltas, job buffers and calls, effects images, `layer_region`, `stored_pixels` |
| `app/src/engine/types.ts`, `client.ts` | Modify | `undoEntryId`, `PixelRect`, `GradientSpec`, `ShapeSpec`, the commands; the client's new calls |
| `app/src/engine/jobs.ts`, `job-worker.ts` | Create | `JobClient` and the module worker |
| `app/src/canvas/layer-textures.ts`, `gl/mask-textures.ts`, `gl-renderer.ts`, `renderer.ts` | Modify | Partial uploads; effects images through hooks |
| `app/src/canvas/effects-images.ts` | Create | `EffectsImages`: plain, reduced, full |
| `app/src/canvas/sampling.ts`, `gradient-tool.ts`, `shape-tool.ts` | Create | The canvas's sampling, the Gradient tool's line, the Shape tool's drag |
| `app/src/canvas/overlay.ts`, `CanvasView.tsx` | Modify | The sample ring, the gradient line, the shape draft; installing the three handlers |
| `app/src/tools/color.ts`, `tools/shape-draft.ts`, `state/gradient-edit.ts` | Create | Colours and HSB; the shape draft; the gradient's settings, stops and 45-degree snap |
| `app/src/state/store.ts` | Modify | Entry ids; jobs and `working`; the palette, the picker, the Eyedropper; the Gradient's pending edit; the Shape draft |
| `app/src/actions/layers.ts`, `actions/files.ts` | Modify | `canPaint`, `fillActive`, Delete on a mask; Gradient Map layers from the palette; `working` refusals |
| `app/src/panels/PaletteSwatches.tsx`, `ColorPickerPanel.tsx`, `GradientOptions.tsx`, `ShapeOptions.tsx` | Create | The rail's palette, the picker, the two bars |
| `app/src/panels/ToolRail.tsx`, `tool-icons.tsx`, `MenuBar.tsx`, `FilterPanel.tsx`, `LevelsPanel.tsx`, `app/src/App.tsx`, `app/src/styles.css` | Modify | Three tools and their icons, Edit > Fill, Gradient Map's ends, mounting |
| `app/src/shortcuts/keymap.ts`, `useShortcuts.ts` | Modify | X, D, I, G, U, Shift+U, Alt / Ctrl + Backspace / Delete, the gradient's digits, Enter and Escape |
| `vite.config.ts` | Modify | Module workers |
| Engine tests created | Create | `history_cap.rs`, `changed_rects.rs`, `jobs.rs`, `raster_edits.rs`, `gradient_preview.rs`, `shapes.rs`, `sample_color.rs`, `perf_4b1.rs` (ignored timings) |
| App tests created | Create | Unit: `partial-upload`, `jobs`, `store-jobs`, `effects-images`, `color`, `palette-store`, `picker-store`, `fill-actions`, `gradient-store`, `shape-draft`. E2E: `history-cap`, `partial-upload`, `jobs`, `effects-worker`, `gradient-preview`, `color-picker`, `eyedropper`, `fill`, `gradient-tool`, `shape-tool`, `perf-4b1` (skipped unless `PERF=1`) |
| App tests modified | Modify | Nine `DocumentState` literals (Task 2), `store-history`, `engine-client`, `keymap`, `tool-rail`, `selection-store` (Delete on a mask) |
| `README.md`, `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`, `Cargo.lock`, `app/tests/e2e/smoke.spec.ts` | Modify | Docs and 0.5.0 (Task 17) |

---

### Task 1: The Phase 4b-1 Mac probes

The Mac draws a shape with Core Graphics and a gradient with `CGContext.drawLinearGradient` / `drawRadialGradient`; neither has been compared with this port yet. This task writes four probe projects for the user to export on the Mac, and the README section that says how. The shapes' layers are 1 x 1 placeholders carrying the shape's record and box, so the Mac redraws each crisply when the user presses Cmd-T then Return on it (`redrawShape`, ShapeTool.swift:159-174); the gradients cannot be written into a project, so the user draws them by hand on prepared canvases (guides at the points to drag between). Pins land later (ruling OQ17).

**Files:**
- Modify: `engine/tests/mac_probes.rs` (the generator, its README text and its test)

**Interfaces:**
- Produces: `shapes.comp`, `gradient-linear.comp`, `gradient-radial.comp`, `gradient-over-colour.comp` in `build-artifacts/mac-probes/` when the probes are generated (`cargo test -p compositor-engine --test mac_probes -- --ignored`), and their README lines.

- [ ] **Step 1: Record the baseline, then write the test**

Before touching anything, run and write down: `cargo test -p compositor-engine` (expect 459 passed, 4 ignored, over 60 test binaries), `pnpm test` (expect 148), `pnpm build` (clean), then `pnpm wasm:dev` and `pnpm e2e` (expect 131 passed, 4 skipped; Task 17 reports against these). `git status` lists the ten `sampling-*` pairs as untracked and nothing else.

The test and the generator are one file; its whole change is in Step 3. The new test, `every_4b1_probe_is_listed_and_every_shape_is_one_the_mac_will_redraw`, asserts that the four probes are named in the README; that the six shape layers are 1 x 1 placeholders on boxes larger than a pixel (the Mac redraws a live shape only when its layer's size differs from its image's, ShapeTool.swift:166), each named for the kind its record gives; that Line 1's and Line 3's ends, taken back from fractions of their boxes, land on the points they were aimed at ((20.5, 120.5) to (140.5, 120.5); (402.5, 207.5)); that whole numbers are written as Swift writes them (`"lineWidth":15`, `"cornerRadius":5000`); and that the two blank gradient canvases start with no pixels.

- [ ] **Step 2: Run the test and watch it fail**

`cargo test -p compositor-engine --test mac_probes every_4b1` before Step 3's code: it does not compile (`phase_4b1_probes` does not exist).

- [ ] **Step 3: Write the probes**

```diff
--- a/engine/tests/mac_probes.rs
+++ b/engine/tests/mac_probes.rs
@@ -470,6 +470,30 @@ New in this set (the blend curve at many sub-pixel positions):
 - sampling-steps-high-1600.comp   -> sampling-steps-high-1600.png
 - sampling-steps-smooth-1600.comp -> sampling-steps-smooth-1600.png
 - sampling-steps-high-700.comp    -> sampling-steps-high-700.png
+
+New in this set (shapes and gradients). These four need a few steps each before the export, and
+the saved project comes back too: after the steps, File > Save (Cmd-S), then export the PNG as
+above, and send the .comp folder with it.
+
+- shapes.comp -> shapes.png
+  Each shape arrives as a flat coloured block. In the Layers panel click Ellipse 1, press Cmd-T,
+  then Return; do the same for Rectangle 1, Rectangle 2, Line 1, Line 2 and Line 3. Each block
+  turns into its shape. Then save and export.
+
+- gradient-linear.comp -> gradient-linear.png
+  Press G. In the Gradient bar choose Linear and Foreground to Background, Reverse off, Opacity
+  100. Press D. Press Cmd-1 for 100%. Hold Shift and drag from the canvas's left edge to its right
+  edge, then press Return. Save and export.
+
+- gradient-radial.comp -> gradient-radial.png
+  Press G. Choose Radial and Foreground to Background, Reverse off, Opacity 100. Press D. Press
+  Cmd-1. Hold Shift and drag from where the guides cross (the centre) to the right-hand guide,
+  then press Return. Save and export.
+
+- gradient-over-colour.comp -> gradient-over-colour.png
+  Click the foreground colour swatch, type 0000FF in the # field, click OK. Press G. Choose Linear
+  and Foreground to Transparent, Reverse off, and type 37 in Opacity. Press Cmd-1. Hold Shift and
+  drag from the canvas's left edge to its right edge, then press Return. Save and export.
 ";
 
 /// 7. RULING (F5, replacing the M9 tautological final-existence loop): the Mac acceptance probe.
@@ -619,6 +643,73 @@ fn step_probes() -> Vec<(&'static str, Document)> {
     ]
 }
 
+/// A Shape tool layer for `shapes.comp` (Phase 4b-1): one pixel of `rgb` stretched over `rect`
+/// (x, y, width, height in document pixels), carrying `record` as its `shape` (the Mac's
+/// `LayerShapeStyle`, ShapeTool.swift:19-32, as Swift's Codable writes it: whole numbers without
+/// `.0`, a CGPoint as `[x, y]`). The Mac draws a live shape afresh at its layer's size whenever a
+/// transform is applied to it (`redrawShape`, EditorSession.swift:451-462), so pressing Cmd-T and
+/// Return on each layer makes the Mac rasterise every shape at exactly this size.
+fn shape_layer(name: &str, rect: [f64; 4], rgb: [u8; 3], record: serde_json::Value) -> Layer {
+    let mut layer = Layer::with_pixels(name, Raster::from_premultiplied(1, 1, vec![rgb[0], rgb[1], rgb[2], 255]), Point { x: rect[0], y: rect[1] });
+    layer.transform.size = Size { width: rect[2], height: rect[3] };
+    layer.extra.shape = Some(record);
+    layer
+}
+
+/// A Line layer: the box of `from` and `to` grown by half of `width` on every side, with the ends
+/// stored as fractions of that box (`finishShape`, ShapeTool.swift:120-140).
+fn line_layer(name: &str, from: [f64; 2], to: [f64; 2], width: f64, rgb: [u8; 3]) -> Layer {
+    let (x, y) = (from[0].min(to[0]) - width / 2.0, from[1].min(to[1]) - width / 2.0);
+    let (w, h) = ((to[0] - from[0]).abs() + width, (to[1] - from[1]).abs() + width);
+    // Swift writes a whole CGFloat without a fraction (`15`, not `15.0`).
+    let swift = |v: f64| if v.fract() == 0.0 { serde_json::json!(v as i64) } else { serde_json::json!(v) };
+    let unit = |p: [f64; 2]| serde_json::json!([swift((p[0] - x) / w), swift((p[1] - y) / h)]);
+    let channel = |v: u8| swift(v as f64 / 255.0);
+    shape_layer(name, [x, y, w, h], rgb, serde_json::json!({
+        "kind": "Line", "red": channel(rgb[0]), "green": channel(rgb[1]), "blue": channel(rgb[2]), "cornerRadius": 0,
+        "lineWidth": swift(width), "start": unit(from), "end": unit(to) }))
+}
+
+/// `shapes.comp`: the Shape tool's curves and strokes, for the Mac to rasterise (ruling OQ13): a
+/// 101 x 61 ellipse, a 120 x 80 rectangle rounded by 20, a 150 x 40 pill (radius 5000, clamped to
+/// 20), and lines 1, 4 and 15 px wide, flat, at 45 degrees and at about 30 degrees, over white.
+fn shapes_doc() -> Document {
+    let mut doc = Document::new(440, 280);
+    let white = solid_rect("Background", [255, 255, 255, 255], 440, 280, 0.0, 0.0);
+    doc.layers = vec![
+        white,
+        shape_layer("Ellipse 1", [10.0, 10.0, 101.0, 61.0], [255, 0, 0], serde_json::json!({ "kind": "Ellipse", "red": 1, "green": 0, "blue": 0, "cornerRadius": 0 })),
+        shape_layer("Rectangle 1", [130.0, 10.0, 120.0, 80.0], [0, 0, 255], serde_json::json!({ "kind": "Rectangle", "red": 0, "green": 0, "blue": 1, "cornerRadius": 20 })),
+        shape_layer("Rectangle 2", [270.0, 10.0, 150.0, 40.0], [0, 0, 0], serde_json::json!({ "kind": "Rectangle", "red": 0, "green": 0, "blue": 0, "cornerRadius": 5000 })),
+        line_layer("Line 1", [20.5, 120.5], [140.5, 120.5], 1.0, [255, 0, 0]),
+        line_layer("Line 2", [160.0, 110.0], [230.0, 180.0], 4.0, [0, 0, 255]),
+        line_layer("Line 3", [262.5, 127.5], [402.5, 207.5], 15.0, [0, 0, 0]),
+    ];
+    doc
+}
+
+/// A blank layer on a `width` x `height` canvas, for a gradient the user draws by hand on the Mac
+/// (ruling OQ13), with `guides` to aim at.
+fn gradient_doc(width: u32, height: u32, guides: &[(GuideAxis, f64)]) -> Document {
+    let mut doc = Document::new(width, height);
+    doc.layers = vec![Layer::blank("Layer 1", doc.size())];
+    doc.guides = guides.iter().map(|(axis, position)| Guide { id: uuid::Uuid::new_v4(), axis: *axis, position: *position }).collect();
+    doc
+}
+
+/// The four Phase 4b-1 probes: `shapes.comp` is drawn by the Mac from its records, the three
+/// gradients by hand (the README says how).
+fn phase_4b1_probes() -> Vec<(&'static str, Document)> {
+    let mut over_colour = Document::new(256, 32);
+    over_colour.layers = vec![solid_rect("Layer 1", [204, 77, 51, 255], 256, 32, 0.0, 0.0)];
+    vec![
+        ("shapes.comp", shapes_doc()),
+        ("gradient-linear.comp", gradient_doc(512, 32, &[])),
+        ("gradient-radial.comp", gradient_doc(256, 256, &[(GuideAxis::Vertical, 128.0), (GuideAxis::Horizontal, 128.0), (GuideAxis::Vertical, 228.0)])),
+        ("gradient-over-colour.comp", over_colour),
+    ]
+}
+
 /// Saves `doc` as `<dir>/<filename>/manifest.json` plus its `images/`, then re-opens the saved
 /// package with `open_package` -- every probe must be openable by this build's own reader before
 /// it is ever sent to a Mac.
@@ -672,6 +763,7 @@ fn write_mac_probes() {
     for (name, doc) in effects_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in sampling_probes() { write_probe(&dir, name, &doc); }
     for (name, doc) in step_probes() { write_probe(&dir, name, &doc); }
+    for (name, doc) in phase_4b1_probes() { write_probe(&dir, name, &doc); }
 
     fs::write(dir.join("README.txt"), README_TXT).unwrap_or_else(|e| panic!("failed to write README.txt: {e}"));
     assert!(README_TXT.is_ascii(), "README.txt must be ASCII only");
@@ -732,6 +824,41 @@ fn every_step_probe_is_listed_and_enlarged_as_named() {
     }
 }
 
+#[test]
+fn every_4b1_probe_is_listed_and_every_shape_is_one_the_mac_will_redraw() {
+    let probes = phase_4b1_probes();
+    assert_eq!(probes.len(), 4);
+    for (name, _) in &probes { assert!(README_TXT.contains(&format!("- {name}")), "{name} is in the README"); }
+    let shapes = &probes[0].1;
+    // The Mac redraws a live shape only when the layer's size differs from its image's
+    // (ShapeTool.swift:166): each placeholder is one pixel, drawn at the shape's size.
+    let drawn: Vec<&Layer> = shapes.layers.iter().filter(|l| l.extra.shape.is_some()).collect();
+    assert_eq!(drawn.len(), 6);
+    for layer in &drawn {
+        let pixels = layer.pixels.as_ref().unwrap();
+        assert_eq!((pixels.width, pixels.height), (1, 1), "{}", layer.name);
+        assert!(layer.transform.size.width > 1.0 && layer.transform.size.height >= 1.0, "{}", layer.name);
+        let record = layer.extra.shape.as_ref().unwrap();
+        assert!(layer.name.starts_with(record["kind"].as_str().unwrap()), "{}: kind", layer.name);
+    }
+    // A Line's ends, back from fractions of its box, land where they were aimed.
+    let t = &drawn[3].transform;
+    let record = drawn[3].extra.shape.as_ref().unwrap();
+    let at = |key: &str| (t.origin.x + record[key][0].as_f64().unwrap() * t.size.width, t.origin.y + record[key][1].as_f64().unwrap() * t.size.height);
+    assert_eq!((at("start"), at("end")), ((20.5, 120.5), (140.5, 120.5)));
+    assert_eq!((t.origin.x, t.origin.y, t.size.width, t.size.height), (20.0, 120.0, 121.0, 1.0));
+    let t = &drawn[5].transform;
+    let record = drawn[5].extra.shape.as_ref().unwrap();
+    let at = |key: &str| (t.origin.x + record[key][0].as_f64().unwrap() * t.size.width, t.origin.y + record[key][1].as_f64().unwrap() * t.size.height);
+    assert!((at("end").0 - 402.5).abs() < 1e-9 && (at("end").1 - 207.5).abs() < 1e-9, "{:?}", at("end"));
+    // Whole numbers are written as Swift writes them, without a fraction.
+    assert!(serde_json::to_string(record).unwrap().contains("\"lineWidth\":15,"), "{record}");
+    assert!(serde_json::to_string(drawn[2].extra.shape.as_ref().unwrap()).unwrap().contains("\"cornerRadius\":5000"));
+    // The hand-drawn gradients start from a blank layer (the gradient decides its pixels).
+    for (_, doc) in &probes[1..3] { assert!(doc.layers[0].pixels.is_none()); }
+    assert_eq!(probes[3].1.layers[0].pixels.as_ref().unwrap().pixel(0, 0), [204, 77, 51, 255]);
+}
+
 #[test]
 fn the_readme_names_the_mac_version_the_probes_are_for() {
     assert!(README_TXT.contains("Compositor for Mac (1.2.10 or later)") && !README_TXT.contains("1.2.6"));
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test mac_probes` (the new test passes; the generators stay ignored). The whole engine suite: 460 passed, 4 ignored (+1). Generate the probes with `cargo test -p compositor-engine --test mac_probes -- --ignored` and check that `build-artifacts/mac-probes/README.txt` has the section and the four `.comp` folders exist; `build-artifacts` is not committed.

- [ ] **Step 5: Prove it bites**

(1) Rename `gradient-radial.comp` in the README's list: the test fails at the README check. Restore. (2) Write the line width with `serde_json::json!(width)` instead of `swift(width)` (so `15.0`): the test fails at `"lineWidth":15`. Restore.

- [ ] **Step 6: Commit**

```
git commit -m "test: the Phase 4b-1 Mac probes: shapes to redraw and canvases to draw gradients on" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine/tests/mac_probes.rs
```


---

### Task 2: History is capped at 100 entries and 256 MiB, and its entries have ids

`History` kept every entry. The Mac keeps 100 and at most 256 MB of pixels that only history holds (DocumentHistory.swift:28), dropping the oldest undo entry first and then the farthest redo entry, after every push, undo and redo (ruling OQ1). The accounting is incremental: a map from each pixel buffer the entries reach to its size and the number of entries reaching it. With a cap, a depth no longer identifies an entry (a push at the cap trims one and the depth stays), so entries get ids and `DocumentState.undoEntryId` replaces the depth the store compared when an Alt-drag duplicate is cancelled (ruling OQ2). The saved state becomes a token, so trimming never loses it.

**Files:**
- Rewrite: `engine/src/history.rs`
- Modify: `engine/src/engine.rs` (limits, `undo_entry_id` in the state, trims), `engine/src/raster.rs` (`buffer_id`, `forget_halvings`)
- Modify: `app/src/engine/types.ts`, `app/src/engine/client.ts` (`wasmBytes`), `app/src/state/store.ts` (`duplicateEntry`)
- Create tests: `engine/tests/history_cap.rs`, `engine/tests/perf_4b1.rs`, `app/tests/e2e/history-cap.spec.ts`, `app/tests/e2e/perf-4b1.spec.ts`
- Modify tests: `app/tests/unit/store-history.test.ts` and the nine `DocumentState` literals

**Interfaces:**
- Produces: `HISTORY_ENTRY_LIMIT`, `HISTORY_BYTE_LIMIT`, `History::with_limits`, `History::undo_entry_id`, `History::retained_bytes`, `History::trim`, `Engine::with_history_limits`; `DocumentState.undo_entry_id` (`undoEntryId`); `Raster::buffer_id`, `Raster::forget_halvings`, `GrayRaster::buffer_id`; the perf harnesses both files grow in later tasks.

- [ ] **Step 1: Write the tests**

`history_cap.rs` ports the Mac's cap cases with small limits so a test pushes a handful of entries: the entries and the pixels only history holds stay within both limits; 101 renames keep the last 100; a raster several entries share is counted once; an edit past the byte limit applies and cannot be undone; trimming the front keeps the saved state findable; the undo entry id follows its entry across undo and redo and survives the cap; history lets go of the halvings of rasters only it holds. The e2e case is the one the depth got wrong: at the cap, Escape during an Alt-drag duplicate still takes the copy away (the history is filled to 100 first). The perf files start here: native release timings (`--ignored`) and the release-wasm suite (skipped unless `PERF=1`).

Create `app/tests/e2e/history-cap.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the history cap (engine/tests/history_cap.rs pins the engine) seen from the app.

type Pt = [number, number];
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
/** Where document point `p` is on screen. */
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);

test("at the history cap, Escape during an Alt-drag duplicate still takes the copy away", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const png = await page.evaluate(solidPngBase64, { width: 40, height: 30, color: "#20a060" });
  await page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0)), "Green", { x: 32, y: 24 });
    const layer = api.engine.state(doc).layers[0].id;
    // 100 entries: the import and 99 renames. The next push trims the oldest.
    for (let n = 0; n < 99; n++) api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `n${n}` });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, png);
  expect((await state(page)).undoDepth).toBe(100);
  await page.keyboard.press("v");
  const a = await client(page, [30, 24]), b = await client(page, [40, 30]);
  await page.keyboard.down("Alt");
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 4 });
  const during = await state(page);
  expect(during.layers.length, "the copy exists while dragging").toBe(2);
  expect(during.undoDepth, "the copy's push trimmed the oldest entry").toBe(100);
  // Alt is let go first: the keymap matches Escape only without modifiers.
  await page.keyboard.up("Alt");
  await page.keyboard.press("Escape");
  await page.mouse.up();
  const after = await state(page);
  expect(after.layers.map((l: any) => l.name)).toEqual(["n98"]);
  expect(after.undoDepth).toBe(99);
  expect(after.canRedo, "nothing to redo: the copy was reverted, not undone").toBe(false);
});
```

Create `app/tests/e2e/perf-4b1.spec.ts`:

```ts
import { test, expect } from "@playwright/test";

// Phase 4b-1's timings (LL-073), in the release engine and on the real GPU: run `pnpm wasm`, then
// `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4b1.spec.ts`, and rebuild `pnpm wasm:dev`
// afterwards. The installed Edge runs WebGL on the GPU as WebView2 does (Playwright's own Chromium
// falls back to software GL). Each test prints what it measured and checks its budget; the plan's
// tasks record the numbers.
test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");
test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });

/** Opens the page and waits for the engine. */
async function ready(page: import("@playwright/test").Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
}

test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push at the cap is cheap", async ({ page }) => {
  test.setTimeout(900_000);
  await ready(page);
  const out = await page.evaluate(() => {
    const api = (window as any).__compositor;
    const result: Record<string, number | number[]> = {};
    for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      const times: number[] = [];
      for (let i = 0; i < 20; i++) {
        const t0 = performance.now();
        api.engine.execute(doc, { type: "InvertPixels", id: layer, mask: false });
        times.push(Math.round(performance.now() - t0));
      }
      const s = api.engine.state(doc);
      result[`${label}: invert ms (20 edits)`] = times;
      result[`${label}: undo depth after 20 edits`] = s.undoDepth;
      result[`${label}: wasm MB after 20 edits`] = Math.round(api.engine.wasmBytes() / 1048576);
      api.engine.closeDocument(doc);
    }
    // 1000 layers with pixels of their own and a full history: each push trims one entry.
    const doc = api.engine.newDocument(200, 200, false);
    const png = api.engine.exportPng(api.engine.newDocument(8, 8, true));
    for (let i = 0; i < 1000; i++) api.engine.importImage(doc, png, `L${i}`, { x: 100, y: 100 });
    const layer = api.engine.state(doc).layers[0].id;
    for (let i = 0; i < 100; i++) api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `n${i}` });
    const pushes: number[] = [];
    for (let i = 0; i < 10; i++) {
      const t0 = performance.now();
      api.engine.execute(doc, { type: "RenameLayer", id: layer, name: `m${i}` });
      pushes.push(performance.now() - t0);
    }
    result["1000 layers, 100 entries: rename at the cap, ms (mean of 10)"] = Math.round(10 * pushes.reduce((a, b) => a + b, 0) / pushes.length) / 10;
    result["1000 layers: undo depth"] = api.engine.state(doc).undoDepth;
    return result;
  });
  console.log(`history (release wasm): ${JSON.stringify(out)}`);
  expect(out["100 MP: wasm MB after 20 edits"]).toBeLessThan(2560);
  expect(out["1000 layers: undo depth"]).toBe(100);
  expect(out["1000 layers, 100 entries: rename at the cap, ms (mean of 10)"]).toBeLessThan(5 + 20);
});
```

```diff
--- a/app/tests/unit/adjust-store.test.ts
+++ b/app/tests/unit/adjust-store.test.ts
@@ -16,7 +16,7 @@ function layer(id: string, o: Partial<LayerState> = {}): LayerState {
 }
 function document(layers: LayerState[], active: string): DocumentState {
   return { id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: active, canUndo: false,
-    canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers };
+    canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers };
 }
 
 function install(layers: LayerState[], active: string) {
```

```diff
--- a/app/tests/unit/commit-transform.test.ts
+++ b/app/tests/unit/commit-transform.test.ts
@@ -14,7 +14,7 @@ function layer(): LayerState {
   };
 }
 function document(): DocumentState {
-  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers: [layer()] };
+  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer()] };
 }
 
 /** A stub EngineClient recording every `execute` command, with no wasm involved. */
@@ -36,7 +36,7 @@ describe("commitTransform unchanged check", () => {
     const { engine, calls } = stubEngine(doc);
     useEditor.setState({
       engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
-      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: cornersToTuples(cornersOf(original)), persistent: true, duplicated: false, undoDepthBefore: null },
+      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: cornersToTuples(cornersOf(original)), persistent: true, duplicated: false, duplicateEntry: null },
     });
     useEditor.getState().commitTransform();
     expect(calls).toEqual([]);
@@ -50,7 +50,7 @@ describe("commitTransform unchanged check", () => {
     const moved: typeof corners = [[corners[0][0] + 5, corners[0][1]], corners[1], corners[2], corners[3]];
     useEditor.setState({
       engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
-      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: moved, persistent: true, duplicated: false, undoDepthBefore: null },
+      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: moved, persistent: true, duplicated: false, duplicateEntry: null },
     });
     useEditor.getState().commitTransform();
     expect(calls).toEqual([{ type: "DistortLayer", id: "A", transform: original, corners: moved }]);
@@ -62,7 +62,7 @@ describe("commitTransform unchanged check", () => {
     const { engine, calls } = stubEngine(doc);
     useEditor.setState({
       engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
-      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: { ...original, origin: [0.2, -0.1] }, corners: null, persistent: true, duplicated: false, undoDepthBefore: null },
+      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: { ...original, origin: [0.2, -0.1] }, corners: null, persistent: true, duplicated: false, duplicateEntry: null },
     });
     useEditor.getState().commitTransform();
     expect(calls).toEqual([]);
```

```diff
--- a/app/tests/unit/crop-seed.test.ts
+++ b/app/tests/unit/crop-seed.test.ts
@@ -7,7 +7,7 @@ import type { DocumentState } from "../../src/engine/types";
 // rectangle instead of an implicit full-canvas default that could never disappear.
 
 function document(): DocumentState {
-  return { id: "D", documentId: "D", width: 120, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers: [] };
+  return { id: "D", documentId: "D", width: 120, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [] };
 }
 
 describe("crop tool rectangle seeding", () => {
```

```diff
--- a/app/tests/unit/crop-tool.test.ts
+++ b/app/tests/unit/crop-tool.test.ts
@@ -4,7 +4,7 @@ import { Viewport } from "../../src/canvas/viewport";
 import type { DocumentState } from "../../src/engine/types";
 
 const doc: DocumentState = {
-  id: "D", documentId: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null,
+  id: "D", documentId: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null,
   layers: [{ id: "L", name: "Red", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
     transform: { origin: [150, 120], size: [100, 60], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, pixelsWidth: 100, pixelsHeight: 60, pixelsRevision: 1, hasMask: false,
     hasPixels: true, maskWidth: 0, maskHeight: 0, maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 }],
```

```diff
--- a/app/tests/unit/layer-rows.test.ts
+++ b/app/tests/unit/layer-rows.test.ts
@@ -6,7 +6,7 @@ function layer(id: string, o: Partial<LayerState> = {}): LayerState {
   return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
     pixelsWidth: 0, pixelsHeight: 0, pixelsRevision: 1, hasPixels: false, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
 }
-const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers });
+const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers });
 
 describe("layer rows", () => {
   // Bottom-to-top array: A, F(group) { B, C }, D
```

```diff
--- a/app/tests/unit/prefilter.test.ts
+++ b/app/tests/unit/prefilter.test.ts
@@ -12,7 +12,7 @@ function layer(id: string, over: Partial<LayerState> = {}): LayerState {
   };
 }
 function document(layers: LayerState[]): DocumentState {
-  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers };
+  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers };
 }
 
 // The expected values here are the same ones asserted against compositor::prefilter_level in
```

```diff
--- a/app/tests/unit/selection-store.test.ts
+++ b/app/tests/unit/selection-store.test.ts
@@ -18,7 +18,7 @@ function layer(id: string, over: Partial<LayerState> = {}): LayerState {
 const selected = (empty = false): SelectionState => ({ revision: 7, empty, bounds: empty ? null : { x: 2, y: 2, width: 5, height: 5 }, antialiased: true, feather: 0, points: 4 });
 function document(layers: LayerState[], selection: SelectionState | null): DocumentState {
   return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: layers[0]?.id ?? null, canUndo: false, canRedo: false,
-    isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection, layers };
+    isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection, layers };
 }
 function stub(doc: DocumentState) {
   const commands: Command[] = [];
```

```diff
--- a/app/tests/unit/selection.test.ts
+++ b/app/tests/unit/selection.test.ts
@@ -6,7 +6,7 @@ function layer(id: string, o: Partial<LayerState> = {}): LayerState {
   return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
     pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
 }
-const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection: null, layers });
+const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers });
 
 describe("selection helpers", () => {
   const f = layer("F", { isGroup: true, hasPixels: false, pixelsWidth: 0 });
```

```diff
--- a/app/tests/unit/store-history.test.ts
+++ b/app/tests/unit/store-history.test.ts
@@ -13,8 +13,8 @@ function layer(id: string, over: Partial<LayerState> = {}): LayerState {
     maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...over,
   };
 }
-function document(id: string, layers: LayerState[], activeLayerId: string | null, undoDepth = 0): DocumentState {
-  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, undoDepth, path: null, guides: [], undrawn: [], selection: null, layers };
+function document(id: string, layers: LayerState[], activeLayerId: string | null, undoDepth = 0, undoEntryId: number | null = null): DocumentState {
+  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, undoDepth, undoEntryId, path: null, guides: [], undrawn: [], selection: null, layers };
 }
 
 /** Records every engine call. `state` serves whatever `docs` currently holds for that handle. */
@@ -33,7 +33,7 @@ function stubEngine(docs: Record<string, DocumentState>) {
   return { engine, calls, commands };
 }
 
-const pendingEdit = { kind: "layer" as const, id: "A", ids: ["A"], box, original: box, draft: { ...box, origin: [5, 5] as [number, number] }, corners: null, persistent: true, duplicated: false, undoDepthBefore: null };
+const pendingEdit = { kind: "layer" as const, id: "A", ids: ["A"], box, original: box, draft: { ...box, origin: [5, 5] as [number, number] }, corners: null, persistent: true, duplicated: false, duplicateEntry: null };
 
 beforeEach(() => {
   useEditor.setState({ engine: null, documents: {}, order: [], activeId: null, viewports: {}, collapsed: {}, transformEdit: null, selectedLayerIds: [], maskSelected: false, error: null, tool: "move", sheet: null });
@@ -61,10 +61,11 @@ describe("undo while a transform is pending", () => {
 
 describe("cancelling an Alt-drag duplicate", () => {
   it("reverts the duplicate instead of undoing it, so there is nothing to redo", () => {
-    // The duplicate pushed one entry, so the depth is one past what it was beforehand.
-    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 1);
+    // At the history cap: the duplicate's push trimmed the oldest entry, so the depth is still 100,
+    // but the entry on top is the one the copy pushed (id 7).
+    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 100, 7);
     const { engine, calls } = stubEngine({ D: doc });
-    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, undoDepthBefore: 0 } });
+    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, duplicateEntry: 7 } });
     useEditor.getState().cancelTransform();
     expect(calls).toContain("revert");
     expect(calls).not.toContain("undo");
@@ -72,11 +73,11 @@ describe("cancelling an Alt-drag duplicate", () => {
   });
 
   it("leaves history alone when something else recorded an entry in the meantime", () => {
-    // Depth 2 where the cancel expects 1: the entry on top is no longer the duplicate's, so
+    // Entry 8 on top where the cancel expects 7: the entry on top is no longer the duplicate's, so
     // reverting would silently drop a real edit and strand the copy.
-    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 2);
+    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 2, 8);
     const { engine, calls } = stubEngine({ D: doc });
-    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, undoDepthBefore: 0 } });
+    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, duplicateEntry: 7 } });
     useEditor.getState().cancelTransform();
     expect(calls).toEqual([]);
     expect(useEditor.getState().transformEdit).toBeNull();
@@ -97,7 +98,7 @@ describe("a history-recording command closes a pending edit first", () => {
     const doc = document("D", [layer("A"), layer("A copy")], "A copy", 1);
     const { engine, commands } = stubEngine({ D: doc });
     useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A copy"], tool: "move",
-      transformEdit: { ...pendingEdit, id: "A copy", ids: ["A copy"], duplicated: true, undoDepthBefore: 0 } });
+      transformEdit: { ...pendingEdit, id: "A copy", ids: ["A copy"], duplicated: true, duplicateEntry: 7 } });
     runAction("opacity-5");
     // The transform commits before the opacity is recorded, so nothing can sit between the
     // duplicate and its own entry.
```

Create `engine/tests/history_cap.rs`:

```rust
//! The history cap (Phase 4b-1): the Mac's 100 entries and 256 MiB of pixels only history holds
//! (DocumentHistory.swift), a state token for "modified", and an entry id a gesture can check.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn layer_id(e: &Engine, id: Uuid) -> Uuid { e.state(id).unwrap().layers[0].id }
fn name(e: &Engine, id: Uuid) -> String { e.state(id).unwrap().layers[0].name.clone() }

/// A `width` x `height` document with one opaque layer (Canvas Size's fill: one undoable step).
fn filled(e: &mut Engine, width: u32, height: u32) -> Uuid {
    let id = e.new_document(10, 10, false).unwrap();
    run(e, id, Command::CanvasSize { width, height, anchor: 4, fill: Some([0.2, 0.4, 0.6]) });
    id
}

/// A document of `width` x `height` with one layer of distinct pixels, for driving `History` directly.
fn doc_with_pixels(width: u32, height: u32, value: u8) -> Document {
    let mut doc = Document::new(width, height);
    doc.layers = vec![Layer::with_pixels("Layer 1", Raster::from_premultiplied(width, height, vec![value; (width * height * 4) as usize]), Point { x: 0.0, y: 0.0 })];
    doc
}

/// One edit on `doc` recorded the way `Engine::edit` records it: the state before goes to history,
/// then history trims against the new state.
fn record(history: &mut History, doc: &mut Document, change: impl FnOnce(&mut Document)) {
    let before = doc.clone();
    change(doc);
    history.push(before);
    history.trim(doc);
}

#[test]
fn history_bounds_entries_and_unique_retained_pixels() {
    // HistoryTests.historyBoundsEntriesAndUniqueRetainedPixels (HistoryTests.swift:139-158): at most
    // two entries and no retained bytes. Three renames keep two entries and share the one image;
    // deleting the layer leaves an entry that alone holds it, so even that entry goes.
    let mut history = History::with_limits(2, 0);
    let mut doc = doc_with_pixels(64, 32, 90);
    for name in ["A", "B", "C"] { record(&mut history, &mut doc, |d| d.layers[0].name = name.into()); }
    assert_eq!(history.depth(), 2);
    assert_eq!(history.retained_bytes(&doc), 0);
    record(&mut history, &mut doc, |d| d.layers.clear());
    assert_eq!(history.depth(), 0);
    assert_eq!(history.retained_bytes(&doc), 0);
}

#[test]
fn a_hundred_and_one_renames_keep_the_last_hundred() {
    let mut e = Engine::new();
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    for n in 0..=100 { run(&mut e, id, Command::RenameLayer { id: layer, name: format!("n{n}") }); }
    assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT);
    for _ in 0..HISTORY_ENTRY_LIMIT { e.undo(id).unwrap(); }
    // The oldest entry (back to "Layer 1") went; the first rename is as far back as undo reaches.
    assert_eq!(name(&e, id), "n0");
    assert!(!e.state(id).unwrap().can_undo);
    assert_eq!(e.state(id).unwrap().undo_depth, 0);
}

#[test]
fn a_raster_several_entries_share_is_counted_once() {
    let mut history = History::with_limits(100, usize::MAX);
    let mut doc = doc_with_pixels(40, 10, 1);
    let bytes = 40 * 10 * 4;
    let replace = |d: &mut Document, v: u8| d.layers[0].set_pixels(Some(Raster::from_premultiplied(40, 10, vec![v; bytes])));
    record(&mut history, &mut doc, |d| replace(d, 2));
    record(&mut history, &mut doc, |d| d.layers[0].name = "x".into());
    record(&mut history, &mut doc, |d| d.layers[0].name = "y".into());
    // Only the first raster is history's alone: the second is the document's too.
    assert_eq!(history.retained_bytes(&doc), bytes);
    record(&mut history, &mut doc, |d| replace(d, 3));
    // The second raster now sits in three entries and counts once: two rasters, not four.
    assert_eq!(history.retained_bytes(&doc), 2 * bytes);
}

#[test]
fn an_edit_past_the_byte_limit_applies_but_cannot_be_undone() {
    // A 100 x 100 layer holds 40,000 bytes. Under a 30,000-byte cap the Invert applies and even its
    // own entry goes, as a layer over 256 MiB goes on the Mac; under 50,000 it stays undoable.
    for (limit, undoable) in [(30_000, false), (50_000, true)] {
        let mut e = Engine::with_history_limits(100, limit);
        let id = filled(&mut e, 100, 100);
        let layer = layer_id(&e, id);
        let before = e.document(id).unwrap().layers[0].pixels.as_ref().unwrap().pixel(5, 5);
        run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
        let after = e.document(id).unwrap().layers[0].pixels.as_ref().unwrap().pixel(5, 5);
        assert_eq!(after, [255 - before[0], 255 - before[1], 255 - before[2], 255], "limit {limit}: the edit applied");
        let state = e.state(id).unwrap();
        assert_eq!(state.can_undo, undoable, "limit {limit}");
        assert_eq!(state.undo_depth, if undoable { 2 } else { 0 }, "limit {limit}");
    }
}

#[test]
fn trimming_the_front_leaves_the_saved_state_findable() {
    // Saved after the first rename, then three more under a three-entry cap: one entry is trimmed
    // from the front, and three undos still land exactly on the saved state.
    let mut e = Engine::with_history_limits(3, usize::MAX);
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "saved".into() });
    e.mark_saved(id, None);
    for n in ["a", "b", "c"] { run(&mut e, id, Command::RenameLayer { id: layer, name: n.into() }); }
    assert_eq!(e.state(id).unwrap().undo_depth, 3);
    assert!(e.state(id).unwrap().is_modified);
    for _ in 0..3 { e.undo(id).unwrap(); }
    assert_eq!(name(&e, id), "saved");
    assert!(!e.state(id).unwrap().is_modified, "back at the saved state");
    e.redo(id).unwrap();
    assert!(e.state(id).unwrap().is_modified);
}

#[test]
fn the_undo_entry_id_follows_its_entry_and_survives_the_cap() {
    let mut e = Engine::with_history_limits(3, usize::MAX);
    let id = e.new_document(10, 10, true).unwrap();
    let layer = layer_id(&e, id);
    assert_eq!(e.state(id).unwrap().undo_entry_id, None);
    for n in ["a", "b", "c"] { run(&mut e, id, Command::RenameLayer { id: layer, name: n.into() }); }
    let third = e.state(id).unwrap().undo_entry_id.unwrap();
    // At the cap a duplicate pushes one entry and trims one: the depth does not move, the id does.
    run(&mut e, id, Command::DuplicateLayer { id: layer });
    let state = e.state(id).unwrap();
    assert_eq!(state.undo_depth, 3);
    let duplicate = state.undo_entry_id.unwrap();
    assert_ne!(duplicate, third);
    e.undo(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(third));
    e.redo(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(duplicate), "the same edit keeps its id through undo and redo");
    e.revert(id).unwrap();
    assert_eq!(e.state(id).unwrap().undo_entry_id, Some(third));
    assert_eq!(e.state(id).unwrap().layers.len(), 1, "the copy is gone");
}

#[test]
fn history_lets_go_of_the_halvings_of_rasters_only_it_holds() {
    let mut e = Engine::new();
    let id = filled(&mut e, 64, 48);
    let layer = layer_id(&e, id);
    let old = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let half = old.halved();
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    // The old raster now lives only in history: its halving was dropped, so halving it again makes
    // a new raster rather than handing back the kept one.
    assert!(!old.halved().same_pixels(&half));
    // The document's own raster keeps its halving.
    let current = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    let kept = current.halved();
    run(&mut e, id, Command::RenameLayer { id: layer, name: "x".into() });
    assert!(current.halved().same_pixels(&kept));
}
```

Create `engine/tests/perf_4b1.rs`:

```rust
//! Native release timings of Phase 4b-1's engine paths (LL-073). Ignored: run them with
//! `cargo test --release -p compositor-engine --test perf_4b1 -- --ignored --nocapture --test-threads=1`
//! and read the printed numbers. The budgets are the release wasm's (app/tests/e2e/perf-4b1.spec.ts);
//! these native numbers locate the cost. Each test checks only that it measured what it says.
use compositor_engine::*;
use std::time::Instant;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }

#[test]
#[ignore]
fn a_rename_at_the_history_cap_with_a_thousand_layers() {
    let mut e = Engine::new();
    let id = e.new_document(200, 200, false).unwrap();
    let png = {
        let mut small = Engine::new();
        let s = small.new_document(8, 8, true).unwrap();
        small.export_png(s).unwrap()
    };
    for i in 0..1000 { e.import_image(Some(id), &png, &format!("L{i}"), Some(Point { x: 100.0, y: 100.0 })).unwrap(); }
    let layer = e.state(id).unwrap().layers[0].id;
    for i in 0..100 { run(&mut e, id, Command::RenameLayer { id: layer, name: format!("n{i}") }); }
    let mut times = Vec::new();
    for i in 0..10 {
        let t = Instant::now();
        run(&mut e, id, Command::RenameLayer { id: layer, name: format!("m{i}") });
        times.push(ms(t));
    }
    let doc = e.document(id).unwrap().clone();
    let mut history = History::default();
    for _ in 0..100 { history.push(doc.clone()); }
    let t = Instant::now();
    for _ in 0..10 { history.trim(&doc); }
    let trim = ms(t) / 10.0;
    println!("rename at the cap, 1000 layers: {:?} ms; trim alone {trim:.2} ms", times.iter().map(|t| (t * 10.0).round() / 10.0).collect::<Vec<_>>());
    assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT);
}
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test history_cap`: does not compile (`Engine::with_history_limits`, `undo_entry_id`). `pnpm test`: the store's tests fail on `undoEntryId` (the TypeScript program does not compile). The e2e test, after Step 3's engine alone, fails: cancelling at the cap leaves the copy.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -10,6 +10,8 @@ export class EngineClient {
   }
 
   version(): string { return this.wasm.version(); }
+  /** The engine's wasm memory in bytes. It only grows; the perf harness reads its high-water mark. */
+  wasmBytes(): number { return this.memory.buffer.byteLength; }
   newDocument(width: number, height: number, emptyLayer: boolean): string { return this.wasm.new_document(width, height, emptyLayer); }
   openPackage(files: PackageFiles, path: string | null): string {
     return this.wasm.open_package(files.manifest, files.images.map((i) => i.name), files.images.map((i) => i.bytes), path ?? undefined);
```

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -116,9 +116,12 @@ export type PreviewEdit =
 export interface DocumentState {
   id: string; documentId: string; width: number; height: number; resolution: number; activeLayerId: string | null;
   canUndo: boolean; canRedo: boolean; isModified: boolean; path: string | null; layers: LayerState[];
-  /** Entries on the undo stack. A gesture that recorded one command compares this against the
-   * depth it saw beforehand to tell whether its own entry is still the one on top. */
+  /** Entries on the undo stack: at most 100, fewer when their pixels pass 256 MiB (engine `History`). */
   undoDepth: number;
+  /** The id of the entry an undo would take back, null with nothing to undo. A gesture that
+   * recorded one command reads it straight after and compares it later to tell whether its own
+   * entry is still on top; the depth cannot tell, once the cap trims the oldest entry. */
+  undoEntryId: number | null;
   guides: Guide[];
   /** What this project contains that this build does not draw yet (`Document::undrawn`). */
   undrawn: string[];
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -43,11 +43,12 @@ export interface TransformEdit {
   corners: Corners | null;
   persistent: boolean;
   duplicated: boolean;
-  /** For a duplicated edit, the engine's undo depth immediately before the DuplicateLayer that
-   * created the copy. `cancelTransform` reverts only when the depth is still exactly one past
-   * this, i.e. the entry on top of the stack is the duplicate's and nothing slipped in behind
-   * it. Null when the edit duplicated nothing. */
-  undoDepthBefore: number | null;
+  /** For a duplicated edit, the id of the history entry the DuplicateLayer that created the copy
+   * pushed (`undoEntryId` read straight after it). `cancelTransform` reverts only while that entry
+   * is still the one on top: nothing slipped in behind it. An id, not a depth: at the history cap
+   * the duplicate's push trims the oldest entry and the depth does not move. Null when the edit
+   * duplicated nothing. */
+  duplicateEntry: number | null;
 }
 
 export interface EditorStore {
@@ -355,7 +356,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     if (!canTransform(state, selectedLayerIds, maskSelected) || get().transformEdit) return false;
     if (transformsAsGroup(state, selectedLayerIds)) {
       const box = groupBox(state, selectedLayerIds)!;
-      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false, undoDepthBefore: null } });
+      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false, duplicateEntry: null } });
       return true;
     }
     let layer = activeLayer(state);
@@ -364,24 +365,23 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // (computed before any duplication, from the layer this transform is actually about).
     const maskAlone = maskSelected && layer.hasMask && !layer.maskLinked;
     const willDuplicate = !!duplicate && !maskAlone;
-    let undoDepthBefore: number | null = null;
+    let duplicateEntry: number | null = null;
     if (willDuplicate) {
-      // The depth before the copy exists, so a later cancel can prove the entry it is about to
-      // drop is the one this command pushed.
-      undoDepthBefore = state.undoDepth;
       // Not `run`: the duplicate has to be observed here to seed the edit. Its failures
       // ("too many layers", "folders are not duplicated this way") still belong in the error
       // banner rather than thrown out of a pointerdown handler.
       try { engine.execute(activeId, { type: "DuplicateLayer", id: layer.id }); }
       catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
       get().refresh(activeId);
+      // The entry the copy pushed, so a later cancel can prove the entry it is about to drop is this one.
+      duplicateEntry = get().documents[activeId].undoEntryId;
       const copy = activeLayer(get().documents[activeId]);
       if (!copy) return false;
       layer = copy;
       set({ selectedLayerIds: [layer.id] });
     }
     const t = maskAlone ? layer.maskPlacement ?? layer.transform : layer.transform;
-    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: willDuplicate, undoDepthBefore } });
+    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: willDuplicate, duplicateEntry } });
     return true;
   },
   previewTransform: (draft, corners) => { const e = get().transformEdit; if (!e || !isValidTransform(draft)) return; set({ transformEdit: { ...e, draft, corners: corners === undefined ? e.corners : corners } }); get().invalidate(); },
@@ -406,13 +406,13 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // outright. A plain `undo` here would leave it on the redo stack, and Ctrl+Shift+Z would
     // bring the cancelled copy back. macOS closes the transaction with nothing recorded.
     //
-    // Only when the entry on top is provably still the duplicate's: the depth must be exactly
-    // one past what it was before the copy was made. `run` commits any pending edit before it
-    // records, so nothing should be able to interleave, but reverting the wrong entry would
-    // silently discard a real edit and strand the copy, so this refuses rather than guesses.
+    // Only when the entry on top is provably still the duplicate's: its id must be the one the
+    // copy pushed. `run` commits any pending edit before it records, so nothing should be able to
+    // interleave, but reverting the wrong entry would silently discard a real edit and strand the
+    // copy, so this refuses rather than guesses.
     if (e.duplicated && engine && activeId) {
-      const depth = engine.state(activeId).undoDepth;
-      if (e.undoDepthBefore !== null && depth === e.undoDepthBefore + 1) { engine.revert(activeId); get().refresh(activeId); }
+      const top = engine.state(activeId).undoEntryId;
+      if (e.duplicateEntry !== null && top === e.duplicateEntry) { engine.revert(activeId); get().refresh(activeId); }
     }
     get().invalidate();
   },
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -50,9 +50,12 @@ pub struct DocumentState {
     #[serde(with = "ids::upper_opt")] pub active_layer_id: Option<Uuid>,
     pub can_undo: bool,
     pub can_redo: bool,
-    /// Entries on the undo stack. A gesture that records one command can compare this against the
-    /// depth it saw beforehand to tell whether its own entry is still the one on top.
+    /// Entries on the undo stack (at most 100, fewer when their pixels pass 256 MiB).
     pub undo_depth: usize,
+    /// The id of the entry an undo would take back (`History::undo_entry_id`), None with nothing to
+    /// undo. A gesture that recorded one command reads it straight after and compares it later to
+    /// tell whether its own entry is still on top: the depth cannot, once the cap trims the oldest.
+    pub undo_entry_id: Option<u64>,
     pub is_modified: bool,
     pub path: Option<String>,
     pub layers: Vec<LayerState>,
@@ -95,7 +98,11 @@ fn renew_revisions(before: &Document, next: &mut Document, counter: &mut u64) {
 }
 
 #[derive(Default)]
-pub struct Engine { sessions: HashMap<Uuid, Session>, order: Vec<Uuid>, preview_revision: u64, revision: u64, effects: EffectsCache, clips: SelectionClips }
+pub struct Engine {
+    sessions: HashMap<Uuid, Session>, order: Vec<Uuid>, preview_revision: u64, revision: u64, effects: EffectsCache, clips: SelectionClips,
+    /// Entries and bytes each document's history keeps; None for the Mac's 100 and 256 MiB.
+    history_limits: Option<(usize, usize)>,
+}
 
 fn check_dimensions(width: u32, height: u32) -> Result<(), CommandError> {
     if !(1..=MAX_SIDE as u32).contains(&width) || !(1..=MAX_SIDE as u32).contains(&height) {
@@ -108,6 +115,9 @@ impl Engine {
     pub fn new() -> Engine { Engine::default() }
     /// An engine keeping its effects images in `effects` (tests give it small limits).
     pub fn with_effects_cache(effects: EffectsCache) -> Engine { Engine { effects, ..Engine::default() } }
+    /// An engine whose documents keep at most `entries` history entries and `bytes` of pixels only
+    /// history holds (tests give it small limits; `History::default` has the Mac's).
+    pub fn with_history_limits(entries: usize, bytes: usize) -> Engine { Engine { history_limits: Some((entries, bytes)), ..Engine::default() } }
     /// The effects images this engine keeps (`EffectsCache`).
     pub fn effects_cache(&self) -> &EffectsCache { &self.effects }
     /// The selection clips this engine keeps (`SelectionClips`), one per document.
@@ -119,7 +129,8 @@ impl Engine {
     /// collisions with a live session never happen.
     fn insert(&mut self, document: Document, path: Option<String>) -> Uuid {
         let handle = Uuid::new_v4();
-        self.sessions.insert(handle, Session { document, history: History::default(), path, preview: None });
+        let history = self.history_limits.map_or_else(History::default, |(entries, bytes)| History::with_limits(entries, bytes));
+        self.sessions.insert(handle, Session { document, history, path, preview: None });
         self.order.push(handle);
         handle
     }
@@ -166,7 +177,8 @@ impl Engine {
         let d = &*doc;
         Ok(DocumentState {
             id, document_id: d.id, width: d.width, height: d.height, resolution: d.resolution, active_layer_id: d.active_layer_id,
-            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), undo_depth: s.history.depth(), is_modified: s.history.is_modified(),
+            can_undo: s.history.can_undo(), can_redo: s.history.can_redo(), undo_depth: s.history.depth(),
+            undo_entry_id: s.history.undo_entry_id(), is_modified: s.history.is_modified(),
             path: s.path.clone(),
             layers: d.layers.iter().map(|l| LayerState {
                 id: l.id, name: l.name.clone(), visible: l.visible, is_group: l.is_group, parent_id: l.parent_id,
@@ -282,7 +294,7 @@ impl Engine {
         let content_changed = !next.same_content(&s.document);
         renew_revisions(&s.document, &mut next, &mut self.revision);
         let before = std::mem::replace(&mut s.document, next);
-        if content_changed { s.history.push(before); }
+        if content_changed { s.history.push(before); s.history.trim(&s.document); }
         self.clips.retain_current(&s.document);
         Ok(dirty)
     }
@@ -402,14 +414,14 @@ impl Engine {
     pub fn undo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
         if self.drop_preview(id) { return Ok(Dirty::everything()); }
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
-        if let Some(before) = s.history.undo(&s.document) { s.document = before; }
+        if let Some(before) = s.history.undo(&s.document) { s.document = before; s.history.trim(&s.document); }
         self.clips.retain_current(&s.document);
         Ok(Dirty::everything())
     }
     pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
         if self.drop_preview(id) { return Ok(Dirty::everything()); }
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
-        if let Some(after) = s.history.redo(&s.document) { s.document = after; }
+        if let Some(after) = s.history.redo(&s.document) { s.document = after; s.history.trim(&s.document); }
         self.clips.retain_current(&s.document);
         Ok(Dirty::everything())
     }
```

```diff
--- a/engine/src/history.rs
+++ b/engine/src/history.rs
@@ -1,49 +1,151 @@
-use crate::Document;
+use crate::{Document, Raster};
+use std::collections::{HashMap, HashSet};
+
+/// The most entries history keeps, undo and redo together (DocumentHistory.swift:28).
+pub const HISTORY_ENTRY_LIMIT: usize = 100;
+/// The most bytes of pixels that only history holds (DocumentHistory.swift:28): a layer or mask
+/// buffer the current document does not share, counted once however many entries reach it.
+pub const HISTORY_BYTE_LIMIT: usize = 256 * 1024 * 1024;
+
+/// One snapshot: the document as it was, the id of the edit it belongs to (kept as the entry moves
+/// between undo and redo, never reused), and the state token the document had then.
+#[derive(Debug)]
+struct Entry { id: u64, doc: Document, state: u64 }
+
+/// A pixel buffer some entry reaches: its size, how many entries reach it, and (for layer pixels)
+/// a handle to drop its halving with.
+#[derive(Debug)]
+struct Held { bytes: usize, entries: usize, raster: Option<Raster> }
 
 /// Whole-document snapshots. Rasters and the selection's outline are shared, so a snapshot costs only
-/// the layer metadata.
-#[derive(Debug, Default)]
+/// the layer metadata plus whatever pixels no other snapshot or the document itself holds. Bounded as
+/// the Mac bounds it (DocumentHistory.swift): after every push, undo and redo, the oldest undo entry
+/// and then the farthest redo entry go while there are more than `entry_limit` entries or the pixels
+/// only history holds exceed `byte_limit` -- even the entry just made, so an edit to a layer larger
+/// than the limit cannot be undone, as on the Mac.
+#[derive(Debug)]
 pub struct History {
-    undo: Vec<Document>,
-    redo: Vec<Document>,
-    saved_depth: usize,
+    undo: Vec<Entry>,
+    redo: Vec<Entry>,
+    /// The token of the current document's state; `saved` is the token when it was saved (the Mac's
+    /// revision UUIDs, DocumentHistory.swift:20-21): trimming the front never changes either.
+    state: u64,
+    saved: Option<u64>,
+    /// Issues entry ids and state tokens alike; never reused.
+    next: u64,
+    entry_limit: usize,
+    byte_limit: usize,
+    /// Every buffer the entries reach, kept up to date as entries come and go, so a trim costs the
+    /// distinct buffers and the entries it drops, not every entry's every layer.
+    held: HashMap<usize, Held>,
+}
+
+impl Default for History {
+    fn default() -> History { History::with_limits(HISTORY_ENTRY_LIMIT, HISTORY_BYTE_LIMIT) }
 }
 
+/// Each pixel buffer a document holds (layer pixels and masks), once: its identity, its size and,
+/// for layer pixels, the raster.
+fn buffers(doc: &Document) -> Vec<(usize, usize, Option<Raster>)> {
+    let mut seen = HashSet::new();
+    let mut out = Vec::new();
+    for l in &doc.layers {
+        if let Some(r) = &l.pixels { if seen.insert(r.buffer_id()) { out.push((r.buffer_id(), r.bytes().len(), Some(r.clone()))); } }
+        if let Some(m) = &l.mask { if seen.insert(m.pixels.buffer_id()) { out.push((m.pixels.buffer_id(), m.pixels.bytes().len(), None)); } }
+    }
+    out
+}
+fn live(doc: &Document) -> HashSet<usize> { buffers(doc).into_iter().map(|(id, _, _)| id).collect() }
+
 impl History {
+    pub fn with_limits(entry_limit: usize, byte_limit: usize) -> History {
+        History { undo: Vec::new(), redo: Vec::new(), state: 1, saved: Some(1), next: 2, entry_limit, byte_limit, held: HashMap::new() }
+    }
+    fn issue(&mut self) -> u64 { let v = self.next; self.next += 1; v }
+    fn hold(&mut self, doc: &Document) {
+        for (id, bytes, raster) in buffers(doc) { self.held.entry(id).or_insert(Held { bytes, entries: 0, raster }).entries += 1; }
+    }
+    /// Lets go of `doc`'s buffers; returns those no entry reaches any more.
+    fn release(&mut self, doc: &Document) -> Vec<(usize, usize)> {
+        let mut gone = Vec::new();
+        for (id, _, _) in buffers(doc) {
+            if let Some(h) = self.held.get_mut(&id) {
+                h.entries -= 1;
+                if h.entries == 0 { gone.push((id, h.bytes)); self.held.remove(&id); }
+            }
+        }
+        gone
+    }
+
+    /// Records `before` as the state an edit left; the future is discarded. Call `trim` with the new
+    /// current document afterwards.
     pub fn push(&mut self, before: Document) {
-        // The saved state can no longer be reached by redo once the future is discarded.
-        // Checked against the pre-push length: the saved depth is the undo length at the
-        // moment of saving, so anything deeper than the state we are about to push from
-        // (i.e. anything only reachable via the redo stack we are about to clear) is lost.
-        if self.saved_depth > self.undo.len() { self.saved_depth = usize::MAX; }
-        self.undo.push(before);
-        self.redo.clear();
+        for e in std::mem::take(&mut self.redo) { self.release(&e.doc); }
+        self.hold(&before);
+        let id = self.issue();
+        self.undo.push(Entry { id, doc: before, state: self.state });
+        self.state = self.issue();
     }
     pub fn undo(&mut self, current: &Document) -> Option<Document> {
-        let before = self.undo.pop()?;
-        self.redo.push(current.clone());
-        Some(before)
+        let entry = self.undo.pop()?;
+        self.release(&entry.doc);
+        self.hold(current);
+        self.redo.push(Entry { id: entry.id, doc: current.clone(), state: self.state });
+        self.state = entry.state;
+        Some(entry.doc)
     }
     pub fn redo(&mut self, current: &Document) -> Option<Document> {
-        let after = self.redo.pop()?;
-        self.undo.push(current.clone());
-        Some(after)
+        let entry = self.redo.pop()?;
+        self.release(&entry.doc);
+        self.hold(current);
+        self.undo.push(Entry { id: entry.id, doc: current.clone(), state: self.state });
+        self.state = entry.state;
+        Some(entry.doc)
     }
     /// Returns to the state before the last entry and drops that entry entirely, offering no
     /// redo. This is how macOS closes a transaction the user cancelled: `cancelTransform`
     /// removes the Alt-drag copy and calls `endEdit`, whose `before.document != document` guard
     /// then records nothing, so neither undo nor redo gains an entry (EditorSession.swift).
-    /// The popped entry is the one `push` added for the cancelled gesture, so `saved_depth` is
-    /// back where it was before that push and needs no adjustment.
-    pub fn revert(&mut self) -> Option<Document> { self.undo.pop() }
-    /// How many entries deep the undo stack is. A caller that recorded this before starting a
-    /// gesture can tell whether the entry on top is still the one its own command pushed.
+    pub fn revert(&mut self) -> Option<Document> {
+        let entry = self.undo.pop()?;
+        self.release(&entry.doc);
+        self.state = entry.state;
+        Some(entry.doc)
+    }
+    /// How many entries deep the undo stack is.
     pub fn depth(&self) -> usize { self.undo.len() }
+    /// The id of the entry on top of the undo stack: the edit an undo would take back. It follows the
+    /// entry through undo and redo and is never reused, so a gesture that recorded it can tell whether
+    /// its own entry is still on top even after trimming dropped older ones.
+    pub fn undo_entry_id(&self) -> Option<u64> { self.undo.last().map(|e| e.id) }
     pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
     pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
-    pub fn mark_saved(&mut self) { self.saved_depth = self.undo.len(); }
-    pub fn is_modified(&self) -> bool { self.saved_depth != self.undo.len() }
+    pub fn mark_saved(&mut self) { self.saved = Some(self.state); }
+    pub fn is_modified(&self) -> bool { self.saved != Some(self.state) }
     /// Content exists that is not on disk (a fresh import): modified until the first save.
-    pub fn mark_never_saved(&mut self) { self.saved_depth = usize::MAX; }
-    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.saved_depth = 0; }
+    pub fn mark_never_saved(&mut self) { self.saved = None; }
+    pub fn reset(&mut self) { self.undo.clear(); self.redo.clear(); self.held.clear(); self.saved = Some(self.state); }
+
+    /// Bytes of pixels that only history holds: every layer and mask buffer an entry reaches and
+    /// `current` does not, each counted once (`retainedBytes`, DocumentHistory.swift:88-110).
+    pub fn retained_bytes(&self, current: &Document) -> usize {
+        let live = live(current);
+        self.held.iter().filter(|(id, _)| !live.contains(id)).map(|(_, h)| h.bytes).sum()
+    }
+
+    /// Drops the oldest undo entry, then the farthest redo entry, while there are more than
+    /// `entry_limit` entries or the pixels only history holds exceed `byte_limit`
+    /// (DocumentHistory.swift:112-118). The halvings of the buffers only history keeps are let go:
+    /// they are not counted, and a buffer that comes back through undo halves again.
+    pub fn trim(&mut self, current: &Document) {
+        let live = live(current);
+        let mut total: usize = self.held.iter().filter(|(id, _)| !live.contains(id)).map(|(_, h)| h.bytes).sum();
+        while self.undo.len() + self.redo.len() > self.entry_limit || total > self.byte_limit {
+            let gone = if !self.undo.is_empty() { self.undo.remove(0) } else if !self.redo.is_empty() { self.redo.remove(0) } else { break };
+            for (id, bytes) in self.release(&gone.doc) { if !live.contains(&id) { total -= bytes; } }
+        }
+        for (id, h) in &self.held {
+            if let Some(r) = &h.raster { if !live.contains(id) { r.forget_halvings(); } }
+        }
+    }
 }
```

```diff
--- a/engine/src/raster.rs
+++ b/engine/src/raster.rs
@@ -20,7 +20,11 @@ impl std::fmt::Debug for RasterInner {
 pub struct Raster { pub width: u32, pub height: u32, inner: Arc<RasterInner> }
 
 impl PartialEq for Raster {
-    fn eq(&self, other: &Self) -> bool { self.width == other.width && self.height == other.height && self.inner.data == other.inner.data }
+    /// The same buffer is equal without reading it: `Document::same_content` compares every layer on
+    /// every edit, and a 100 MP layer's bytes took a full scan to compare with themselves.
+    fn eq(&self, other: &Self) -> bool {
+        self.width == other.width && self.height == other.height && (Arc::ptr_eq(&self.inner, &other.inner) || self.inner.data == other.inner.data)
+    }
 }
 
 impl Raster {
@@ -70,6 +74,11 @@ impl Raster {
         out
     }
     pub fn same_pixels(&self, other: &Raster) -> bool { Arc::ptr_eq(&self.inner, &other.inner) }
+    /// The pixel buffer's identity: equal for every clone sharing it (history counts buffers by it).
+    pub fn buffer_id(&self) -> usize { Arc::as_ptr(&self.inner) as *const u8 as usize }
+    /// Drops the memoized halving of this buffer, for every clone that shares it (history lets go of
+    /// the halvings of buffers only it holds: they are not counted against its limit).
+    pub fn forget_halvings(&self) { *self.inner.half.lock().unwrap() = None; }
     /// Whether another clone of this raster holds the same pixel buffer.
     pub fn shared(&self) -> bool { Arc::strong_count(&self.inner) > 1 }
     /// How many handles hold these pixels, this one included.
@@ -142,6 +151,8 @@ impl GrayRaster {
     pub fn bytes(&self) -> &[u8] { &self.data }
     /// The very same pixel buffer (a clone of this raster), not merely equal pixels.
     pub fn same_pixels(&self, other: &GrayRaster) -> bool { Arc::ptr_eq(&self.data, &other.data) }
+    /// The pixel buffer's identity: equal for every clone sharing it.
+    pub fn buffer_id(&self) -> usize { Arc::as_ptr(&self.data) as *const u8 as usize }
     /// Whether another clone of this raster holds the same pixel buffer.
     pub fn shared(&self) -> bool { Arc::strong_count(&self.data) > 1 }
     /// How many handles hold these pixels, this one included.
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine`: 467 passed, 5 ignored (+7, +1 ignored). `pnpm test`: 148 (the literals changed, none added). `pnpm wasm:dev`, `pnpm build`, `pnpm e2e`: 132 passed, 5 skipped. Timings (`pnpm wasm`, then `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4b1.spec.ts -g history`): 20 whole-layer inverts at 24 MP keep 2 entries and 368 MB of wasm heap, at 100 MP 0 entries and 1131 MB (budget 2560 MB); a rename at the cap with 1000 layers 3.1 ms mean (budget 25 ms). Natively (`cargo test --release -p compositor-engine --test perf_4b1 -- --ignored --nocapture --test-threads=1`): 1.5-2.4 ms a rename, 0.16 ms the trim alone.

- [ ] **Step 5: Prove it bites**

(1) In `History::trim`, return at once: 6 of the 7 tests in `history_cap.rs` fail (all but `a_raster_several_entries_share_is_counted_once`, which checks the accounting, not the trim; measured). Restore. (2) In the store's `cancelTransform`, revert whenever the edit duplicated, without comparing the entry on top with `duplicateEntry`: `store-history.test.ts`'s "leaves history alone when something else recorded an entry in the meantime" fails. Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/history-cap.spec.ts app/tests/e2e/perf-4b1.spec.ts engine/tests/history_cap.rs engine/tests/perf_4b1.rs
git commit -m "feat: history keeps 100 entries and 256 MiB as the Mac's does, and names its entries" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/engine/client.ts app/src/engine/types.ts app/src/state/store.ts app/tests/e2e/history-cap.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/adjust-store.test.ts app/tests/unit/commit-transform.test.ts app/tests/unit/crop-seed.test.ts app/tests/unit/crop-tool.test.ts app/tests/unit/layer-rows.test.ts app/tests/unit/prefilter.test.ts app/tests/unit/selection-store.test.ts app/tests/unit/selection.test.ts app/tests/unit/store-history.test.ts engine/src/engine.rs engine/src/history.rs engine/src/raster.rs engine/tests/history_cap.rs engine/tests/perf_4b1.rs
```


---

### Task 3: Edits report where they changed a layer, and the halvings follow

A selection-limited edit changes a rectangle of a large layer, but everything downstream treated it as a new layer: the halvings were redone from scratch and the GPU uploaded it whole. This task makes the engine know the rectangle (ruling OQ3) and seed the new raster's halvings from the old raster's outside it (ruling OQ4). Commands that know their rectangle report it in `Dirty.regions`; `Engine::edit` records it in the lineage (`lineage.rs`) by revision; `pixels_delta` / `mask_delta` answer "what changed since revision R". The halving is also rewritten as one pass per level, the same bytes as before (tested at every edge and size).

**Files:**
- Create: `engine/src/lineage.rs`
- Modify: `engine/src/command.rs` (`Plane`, `Region`, `Dirty.regions`), `engine/src/engine.rs` (lineage, `seed_halvings`, deltas, the commands' regions), `engine/src/raster.rs` (`PixelRect`, halving, seeding), `engine/src/selection/coverage.rs` (`rect_on_grid`), `engine/src/lib.rs`
- Create tests: `engine/tests/changed_rects.rs`; modify `engine/tests/perf_4b1.rs`

**Interfaces:**
- Produces: `PixelRect { x, y, width, height }` (`is_empty`, `union`, `halved`), `Raster::halved`, `Raster::seed_halvings(parent, rect)`, `Plane`, `Region`, `Dirty::within`, `Engine::pixels_delta(id, layer, from) -> Option<PixelRect>` (Some(empty) = unchanged, None = take it whole), `Engine::mask_delta`, `LINEAGE_LIMIT`, `SelectionClip::rect_on_grid` (the clip's rectangle on a layer's grid, a pixel wider on each side for sampling).

- [ ] **Step 1: Write the tests**

Every rectangle below is computed from the selection in the test: a clear in (30, 20)-(50, 44) on a canvas-covering layer reports {28, 18, 24, 28} (the selection, a pixel for the clip, a pixel for sampling, each side); undo and redo report the same rectangle back; an edit without a rectangle, or one that changes the grid, is whole (None); a mask edit reports its rectangle on the mask's own grid ({3, 48, 24, 14}), and a mask that changed size is taken whole; the seeded halvings equal halving from scratch; the lineage forgets changes past its limit.

Create `engine/tests/changed_rects.rs`:

```rust
//! Changed rectangles (Phase 4b-1): the revision lineage the GPU asks for partial uploads, the halving
//! it redoes only where pixels changed, and the halving itself.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Deterministic, unequal pixels: no two neighbours alike, alpha varied.
fn pattern(width: u32, height: u32, seed: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let v = (x.wrapping_mul(73) ^ y.wrapping_mul(151) ^ seed.wrapping_mul(29)).wrapping_mul(2654435761);
        let a = (v >> 24) as u8 | 1;
        data.extend_from_slice(&[((v >> 8) as u8).min(a), ((v >> 16) as u8).min(a), (v as u8).min(a), a]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// The 2 x 2 box average written out plainly: every output pixel averages the source pixels of its
/// block that exist, rounding half up (the definition `Raster::halved` must keep).
fn reference_half(r: &Raster) -> Vec<u8> {
    let (w, h) = ((r.width / 2).max(1), (r.height / 2).max(1));
    let mut out = Vec::new();
    for y in 0..h { for x in 0..w {
        let mut sum = [0u32; 4]; let mut n = 0u32;
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (sx, sy) = (2 * x + dx, 2 * y + dy);
            if sx < r.width && sy < r.height { let px = r.pixel(sx, sy); for c in 0..4 { sum[c] += px[c] as u32; } n += 1; }
        }
        for c in 0..4 { out.push(((sum[c] + n / 2) / n) as u8); }
    }}
    out
}

#[test]
fn halving_averages_each_block_at_every_edge_and_size() {
    for (w, h) in [(7, 5), (1, 9), (9, 1), (1, 1), (2, 3), (33, 17), (64, 48)] {
        let r = pattern(w, h, w * 31 + h);
        let half = r.halved();
        assert_eq!((half.width, half.height), ((w / 2).max(1), (h / 2).max(1)));
        assert_eq!(half.bytes(), reference_half(&r).as_slice(), "{w} x {h}");
    }
}

/// `parent` with `rect` painted over by another pattern.
fn painted(parent: &Raster, rect: PixelRect) -> Raster {
    let other = pattern(parent.width, parent.height, 999);
    let mut data = parent.bytes().to_vec();
    for y in rect.y..rect.y + rect.height { for x in rect.x..rect.x + rect.width {
        let i = ((y * parent.width + x) * 4) as usize;
        data[i..i + 4].copy_from_slice(&other.bytes()[i..i + 4]);
    }}
    Raster::from_premultiplied(parent.width, parent.height, data)
}

#[test]
fn seeded_halvings_equal_halving_from_scratch() {
    // An odd-sized parent halved twice; rectangles at odd offsets, one reaching the far edges.
    for rect in [PixelRect { x: 5, y: 7, width: 9, height: 4 }, PixelRect { x: 30, y: 20, width: 7, height: 9 }, PixelRect { x: 0, y: 0, width: 1, height: 1 }] {
        let parent = pattern(37, 29, 3);
        let _ = parent.halved().halved();
        let child = painted(&parent, rect);
        child.seed_halvings(&parent, rect);
        // Seeded, not made on demand: both levels are there before anyone asks.
        let seeded = child.memoized_half().expect("level 1 seeded");
        assert!(seeded.memoized_half().is_some(), "level 2 seeded");
        let fresh = Raster::from_premultiplied(child.width, child.height, child.bytes().to_vec());
        assert_eq!(seeded.bytes(), fresh.halved().bytes(), "{rect:?}: level 1");
        assert_eq!(seeded.halved().bytes(), fresh.halved().halved().bytes(), "{rect:?}: level 2");
    }
}

/// A 120 x 80 document with one opaque layer of `pattern` pixels, active.
fn document() -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(120, 80, false).unwrap();
    let png = encode_png(&pattern(120, 80, 7), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png, "Pattern", Some(p(60.0, 40.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    (e, id, layer)
}
fn revision(e: &Engine, id: Uuid) -> u64 { e.state(id).unwrap().layers[0].pixels_revision }
fn pixels(e: &Engine, id: Uuid) -> Raster { e.document(id).unwrap().layers[0].pixels.clone().unwrap() }
fn select(e: &mut Engine, id: Uuid, x0: f64, y0: f64, x1: f64, y1: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)], mode: SelectionMode::Replace, antialiased: false });
}

#[test]
fn a_clear_inside_a_selection_reports_where_it_changed_and_undo_and_redo_follow() {
    let (mut e, id, layer) = document();
    select(&mut e, id, 30.0, 20.0, 50.0, 44.0);
    let (r0, before) = (revision(&e, id), pixels(&e, id));
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    let (r1, after) = (revision(&e, id), pixels(&e, id));
    // The clip's region is the selection's bounds grown by a pixel (SelectionClip::new); the reported
    // rectangle adds a pixel more for bilinear sampling: 30 - 2 = 28 to 50 + 2 = 52, 20 - 2 to 44 + 2.
    let expected = PixelRect { x: 28, y: 18, width: 24, height: 28 };
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected));
    // Nothing outside it changed; inside, the selected pixels did.
    for y in 0..80 { for x in 0..120 {
        let inside = x >= 28 && x < 52 && y >= 18 && y < 46;
        if !inside { assert_eq!(before.pixel(x, y), after.pixel(x, y), "({x}, {y})"); }
    }}
    assert_eq!(after.pixel(40, 30), [0, 0, 0, 0]);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), Some(PixelRect::default()), "nothing since the current revision");
    e.undo(id).unwrap();
    assert_eq!(revision(&e, id), r0);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), Some(expected), "undo changes the same rectangle back");
    e.redo(id).unwrap();
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected));
    // A second clear elsewhere: from the first revision the two rectangles together.
    select(&mut e, id, 90.0, 60.0, 100.0, 70.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(expected.union(&PixelRect { x: 88, y: 58, width: 14, height: 14 })));
}

#[test]
fn an_edit_without_a_rectangle_or_that_changes_the_grid_is_taken_whole() {
    let (mut e, id, layer) = document();
    let r0 = revision(&e, id);
    run(&mut e, id, Command::RenameLayer { id: layer, name: "x".into() });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), Some(PixelRect::default()), "a rename changes no pixels");
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), None, "no selection: the whole layer");
    let r1 = revision(&e, id);
    // A blur spreads past the layer and grows its grid: whole.
    run(&mut e, id, Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 3.0 } });
    assert!(pixels(&e, id).width > 120);
    assert_eq!(e.pixels_delta(id, layer, r1).unwrap(), None);
    assert_eq!(e.pixels_delta(id, layer, 123_456).unwrap(), None, "a revision never seen");
}

#[test]
fn a_mask_edit_inside_a_selection_reports_its_rectangle_on_the_mask() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
    select(&mut e, id, 60.0, 10.0, 70.0, 30.0);
    let m0 = e.state(id).unwrap().layers[0].mask_revision;
    // The first fill spreads the 1 x 1 mask over the layer's grid: whole.
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(e.mask_delta(id, layer, m0).unwrap(), None);
    let m1 = e.state(id).unwrap().layers[0].mask_revision;
    select(&mut e, id, 5.0, 50.0, 25.0, 60.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(e.mask_delta(id, layer, m1).unwrap(), Some(PixelRect { x: 3, y: 48, width: 24, height: 14 }));
}

#[test]
fn an_edit_inside_a_selection_hands_the_new_pixels_the_old_halvings() {
    let (mut e, id, layer) = document();
    let old = pixels(&e, id);
    let _ = old.halved().halved();
    select(&mut e, id, 33.0, 21.0, 47.0, 39.0);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
    let new = pixels(&e, id);
    let seeded = new.memoized_half().expect("seeded at the edit");
    let fresh = Raster::from_premultiplied(120, 80, new.bytes().to_vec());
    assert_eq!(seeded.bytes(), fresh.halved().bytes());
    assert_eq!(seeded.halved().bytes(), fresh.halved().halved().bytes());
}

#[test]
fn the_lineage_forgets_changes_past_its_limit() {
    let (mut e, id, layer) = document();
    select(&mut e, id, 10.0, 10.0, 12.0, 12.0);
    let r0 = revision(&e, id);
    for _ in 0..LINEAGE_LIMIT { run(&mut e, id, Command::InvertPixels { id: layer, mask: false }); }
    assert_eq!(e.pixels_delta(id, layer, r0).unwrap(), None, "the first change is forgotten");
    let recent = revision(&e, id);
    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
    assert!(e.pixels_delta(id, layer, recent).unwrap().is_some());
}
```

```diff
--- a/engine/tests/perf_4b1.rs
+++ b/engine/tests/perf_4b1.rs
@@ -9,6 +9,40 @@ use uuid::Uuid;
 fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
 fn ms(t: Instant) -> f64 { t.elapsed().as_secs_f64() * 1000.0 }
 
+/// A `width` x `height` document with one opaque layer over it (Canvas Size's fill), active.
+fn filled(width: u32, height: u32) -> (Engine, Uuid, Uuid) {
+    let mut e = Engine::new();
+    let id = e.new_document(10, 10, false).unwrap();
+    run(&mut e, id, Command::CanvasSize { width, height, anchor: 4, fill: Some([0.5, 0.4, 0.3]) });
+    let layer = e.state(id).unwrap().layers[0].id;
+    run(&mut e, id, Command::SetActiveLayer { id: Some(layer) });
+    (e, id, layer)
+}
+
+#[test]
+#[ignore]
+fn halving_and_a_clear_in_a_selection_at_24_and_100_mp() {
+    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
+        let (mut e, id, layer) = filled(w, h);
+        let raster = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
+        let t = Instant::now();
+        let _ = raster.halved().halved().halved();
+        let chain = ms(t);
+        // A 1024 x 1024 selection cleared: the edit copies the layer; the new pixels are handed the
+        // old halvings (three levels) with only the cleared part redone.
+        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![Point { x: 1000.0, y: 1000.0 }, Point { x: 2024.0, y: 1000.0 }, Point { x: 2024.0, y: 2024.0 }, Point { x: 1000.0, y: 2024.0 }], mode: SelectionMode::Replace, antialiased: false });
+        let t = Instant::now();
+        run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: false });
+        let clear = ms(t);
+        let new = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
+        let t = Instant::now();
+        let seeded = new.halved().halved().halved();
+        let after = ms(t);
+        println!("{label}: halving chain to level 3 {chain:.0} ms; clear in a 1024 px selection {clear:.0} ms; level 3 after it {after:.2} ms");
+        assert_eq!(seeded.width, w / 8);
+    }
+}
+
 #[test]
 #[ignore]
 fn a_rename_at_the_history_cap_with_a_thousand_layers() {
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test changed_rects`: does not compile (`PixelRect`, `pixels_delta`, `seed_halvings`).

- [ ] **Step 3: Implement**

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -138,6 +138,16 @@ impl Command {
     }
 }
 
+/// Which of a layer's buffers: its pixels or its mask.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
+pub enum Plane { Pixels, Mask }
+
+/// Where a command changed one layer's buffer, in that buffer's own grid: nothing outside `rect`
+/// differs. A command that knows this reports it, so the engine can record a changed rectangle
+/// (`Engine::pixels_delta`) and halve only that part again (`Raster::seed_halvings`).
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub struct Region { pub layer: Uuid, pub plane: Plane, pub rect: crate::PixelRect }
+
 /// What a command changed, so the renderer re-syncs only what it must.
 #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
 pub struct Dirty {
@@ -148,6 +158,10 @@ pub struct Dirty {
     /// Layers whose pixels were replaced.
     #[serde(serialize_with = "serialize_ids", deserialize_with = "deserialize_ids")]
     pub layers: Vec<Uuid>,
+    /// The rectangles changed buffers were changed within, where the command knows them. Engine-side
+    /// only: the app asks for changed rectangles by revision (`Engine::pixels_delta`).
+    #[serde(skip)]
+    pub regions: Vec<Region>,
 }
 
 fn serialize_ids<S: serde::Serializer>(ids: &[Uuid], s: S) -> Result<S::Ok, S::Error> {
@@ -161,6 +175,10 @@ fn deserialize_ids<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Uuid>,
 }
 
 impl Dirty {
-    pub fn everything() -> Dirty { Dirty { structure: true, canvas: true, layers: vec![] } }
+    pub fn everything() -> Dirty { Dirty { structure: true, canvas: true, ..Default::default() } }
     pub fn structure() -> Dirty { Dirty { structure: true, ..Default::default() } }
+    /// The structure, and the pixels of `layers`.
+    pub fn pixels(layers: Vec<Uuid>) -> Dirty { Dirty { structure: true, layers, ..Default::default() } }
+    /// This, with `regions` reported.
+    pub fn within(mut self, regions: Vec<Region>) -> Dirty { self.regions = regions; self }
 }
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -8,6 +8,8 @@ pub struct Session {
     pub history: History,
     pub path: Option<String>,
     pub preview: Option<PixelPreview>,
+    /// The recent changed rectangles of its layers' buffers (`Engine::pixels_delta`).
+    pub lineage: Lineage,
 }
 
 #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
@@ -104,6 +106,31 @@ pub struct Engine {
     history_limits: Option<(usize, usize)>,
 }
 
+/// Where the document's selection reaches `layer`'s pixel or mask grid: the clip's rectangle on that
+/// grid (`SelectionClip::rect_on_grid`). Nothing without a selection.
+fn selection_regions(doc: &Document, clips: &SelectionClips, layer: Uuid, plane: Plane) -> Vec<Region> {
+    let (Some(clip), Some(l)) = (clips.clip(doc), doc.layer(layer)) else { return vec![] };
+    let (grid, w, h) = match (plane, &l.pixels, &l.mask) {
+        (Plane::Pixels, Some(p), _) => (l.transform, p.width, p.height),
+        (Plane::Mask, _, Some(m)) => (m.placement.unwrap_or(l.transform), m.pixels.width, m.pixels.height),
+        _ => return vec![],
+    };
+    clip.rect_on_grid(&grid.pixel_to_document(w, h), w, h).map(|rect| vec![Region { layer, plane, rect }]).unwrap_or_default()
+}
+
+/// Gives each layer's new pixels, changed only within a reported region, the halvings its old pixels
+/// had, redone only there (`Raster::seed_halvings`): the GPU at a reduced zoom then uploads the region
+/// without halving the whole layer again.
+fn seed_halvings(before: &Document, after: &Document, regions: &[Region]) {
+    let mut per_layer: HashMap<Uuid, PixelRect> = HashMap::new();
+    for r in regions.iter().filter(|r| r.plane == Plane::Pixels) { per_layer.entry(r.layer).and_modify(|u| *u = u.union(&r.rect)).or_insert(r.rect); }
+    for (layer, rect) in per_layer {
+        let old = before.layer(layer).and_then(|l| l.pixels.as_ref());
+        let new = after.layer(layer).and_then(|l| l.pixels.as_ref());
+        if let (Some(old), Some(new)) = (old, new) { new.seed_halvings(old, rect); }
+    }
+}
+
 fn check_dimensions(width: u32, height: u32) -> Result<(), CommandError> {
     if !(1..=MAX_SIDE as u32).contains(&width) || !(1..=MAX_SIDE as u32).contains(&height) {
         return Err(CommandError::Argument("width and height must be 1 to 30000".into()));
@@ -130,7 +157,7 @@ impl Engine {
     fn insert(&mut self, document: Document, path: Option<String>) -> Uuid {
         let handle = Uuid::new_v4();
         let history = self.history_limits.map_or_else(History::default, |(entries, bytes)| History::with_limits(entries, bytes));
-        self.sessions.insert(handle, Session { document, history, path, preview: None });
+        self.sessions.insert(handle, Session { document, history, path, preview: None, lineage: Lineage::default() });
         self.order.push(handle);
         handle
     }
@@ -230,6 +257,21 @@ impl Engine {
         Ok(out)
     }
 
+    /// What changed in `layer`'s pixels since revision `from`, as the canvas shows them (a preview's
+    /// pixels have revisions of their own): an empty rectangle for nothing, a rectangle of the pixel
+    /// grid, or None when the whole raster must be uploaded again (`Lineage::delta`).
+    pub fn pixels_delta(&self, id: Uuid, layer: Uuid, from: u64) -> Result<Option<PixelRect>, CommandError> {
+        let doc = self.render_document(id)?;
+        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
+        Ok(self.session(id)?.lineage.delta(layer, Plane::Pixels, from, l.pixels_revision))
+    }
+    /// `pixels_delta` for the layer's mask, in the mask's own grid.
+    pub fn mask_delta(&self, id: Uuid, layer: Uuid, from: u64) -> Result<Option<PixelRect>, CommandError> {
+        let s = self.session(id)?;
+        let l = s.document.layer(layer).ok_or(CommandError::NoLayer)?;
+        Ok(s.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
+    }
+
     /// Whether a press at `at` lands inside a selection with something in it, by the winding rule:
     /// where a drag in New mode moves the outline instead of drawing (`canMoveSelection(at:)`,
     /// Selection.swift:256-260).
@@ -271,14 +313,14 @@ impl Engine {
         // can never be answered with stale pixels (phase 3 open item N3).
         let s = self.session(id)?;
         if let (Some(r), Some(current)) = (&request, &s.preview) {
-            if current.answers(r, &PreviewSource::of(&s.document, r.layer())) { return Ok(Dirty { structure: false, canvas: false, layers: vec![] }); }
+            if current.answers(r, &PreviewSource::of(&s.document, r.layer())) { return Ok(Dirty::default()); }
         }
         let revision = { self.preview_revision += 1; PREVIEW_REVISION_BASE + self.preview_revision };
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
         let layers: Vec<Uuid> = s.preview.iter().map(|p| p.layer).chain(request.iter().map(|r| r.layer())).collect();
         let clips = &self.clips;
         s.preview = request.as_ref().and_then(|r| preview::compute_preview_with(&s.document, clips, r, revision));
-        Ok(Dirty { structure: true, canvas: false, layers })
+        Ok(Dirty::pixels(layers))
     }
     fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }
 
@@ -293,6 +335,8 @@ impl Engine {
         // still applies, but records no history entry and so leaves redo intact, as macOS does.
         let content_changed = !next.same_content(&s.document);
         renew_revisions(&s.document, &mut next, &mut self.revision);
+        s.lineage.record_edit(&s.document, &next, &dirty.regions);
+        seed_halvings(&s.document, &next, &dirty.regions);
         let before = std::mem::replace(&mut s.document, next);
         if content_changed { s.history.push(before); s.history.trim(&s.document); }
         self.clips.retain_current(&s.document);
@@ -354,34 +398,45 @@ impl Engine {
                     ops::hierarchy::clip_dependents(doc, &ids).into_iter().filter(|d| doc.layer(*d).map_or(false, |l| l.has_pixels())).collect()
                 } else { Vec::new() };
                 ops::hierarchy::delete_layers(doc, &ids, bake)?;
-                Ok(Dirty { structure: true, canvas: false, layers: baked })
+                Ok(Dirty::pixels(baked))
             }
             Command::SetLayerTransform { id, transform } => { ops::transform::set_transform(doc, id, transform)?; Ok(Dirty::structure()) }
             Command::TransformLayers { ids, bounds, draft } => { ops::transform::transform_group(doc, &ids, &bounds, &draft)?; Ok(Dirty::structure()) }
             Command::FlipLayers { ids, horizontal } => { ops::transform::flip_layers(doc, &ids, horizontal)?; Ok(Dirty::structure()) }
             Command::NudgeLayers { ids, dx, dy } => { ops::transform::nudge(doc, &ids, dx, dy)?; Ok(Dirty::structure()) }
-            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
+            Command::DistortLayer { id, transform, corners } => { ops::distort::distort_layer(doc, id, &transform, &corners)?; Ok(Dirty::pixels(vec![id])) }
             Command::DistortLayers { ids, bounds, draft, corners } => {
                 let touched = ops::transform::members(doc, &ids);
                 ops::distort::distort_group(doc, &ids, &bounds, &draft, &corners)?;
-                Ok(Dirty { structure: true, canvas: false, layers: touched })
+                Ok(Dirty::pixels(touched))
             }
             Command::SetMaskPlacement { id, placement } => { ops::transform::set_mask_placement(doc, id, placement)?; Ok(Dirty::structure()) }
             Command::AddMask { id, revealing } => { ops::masks::add_mask(doc, id, revealing)?; Ok(Dirty::structure()) }
             Command::DeleteMask { id } => { ops::masks::delete_mask(doc, id)?; Ok(Dirty::structure()) }
             Command::SetMaskEnabled { id, enabled } => { ops::masks::set_mask_enabled(doc, id, enabled)?; Ok(Dirty::structure()) }
             Command::SetMaskLinked { id, linked } => { ops::masks::set_mask_linked(doc, id, linked)?; Ok(Dirty::structure()) }
-            Command::InvertMask { id } => { ops::adjust::invert_layer_with(doc, clips, id, true)?; Ok(Dirty::structure()) }
+            Command::InvertMask { id } => { ops::adjust::invert_layer_with(doc, clips, id, true)?; Ok(Dirty::structure().within(selection_regions(doc, clips, id, Plane::Mask))) }
             Command::FillMask { id, white } => { ops::masks::fill_mask(doc, id, white)?; Ok(Dirty::structure()) }
             Command::BlurMask { id, radius } => { ops::masks::blur_mask(doc, id, radius)?; Ok(Dirty::structure()) }
             Command::CopyMask { from, to } => { ops::masks::copy_mask(doc, from, to)?; Ok(Dirty::structure()) }
             Command::ToggleClipping { id } => { ops::hierarchy::toggle_clipping(doc, id)?; Ok(Dirty::structure()) }
             Command::ReleaseClipping { id } => { ops::hierarchy::release_clipping(doc, id)?; Ok(Dirty::structure()) }
             Command::LinkMask { source, target } => { ops::hierarchy::link_mask(doc, source, target)?; Ok(Dirty::structure()) }
-            Command::MergeLayers { ids } => { let m = ops::merge::merge(doc, &ids)?; Ok(Dirty { structure: true, canvas: false, layers: vec![m] }) }
-            Command::ApplyAdjustment { id, adjustment } => { ops::adjust::apply_adjustment_to_layer_with(doc, clips, id, &adjustment)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
-            Command::InvertPixels { id, mask } => { ops::adjust::invert_layer_with(doc, clips, id, mask)?; Ok(Dirty { structure: true, canvas: false, layers: if mask { vec![] } else { vec![id] } }) }
-            Command::ApplyFilter { id, params } => { ops::adjust::apply_filter_with(doc, clips, id, &params)?; Ok(Dirty { structure: true, canvas: false, layers: vec![id] }) }
+            Command::MergeLayers { ids } => { let m = ops::merge::merge(doc, &ids)?; Ok(Dirty::pixels(vec![m])) }
+            // Inside a selection these change only what the selection reaches on the layer's grid.
+            Command::ApplyAdjustment { id, adjustment } => {
+                ops::adjust::apply_adjustment_to_layer_with(doc, clips, id, &adjustment)?;
+                Ok(Dirty::pixels(vec![id]).within(selection_regions(doc, clips, id, Plane::Pixels)))
+            }
+            Command::InvertPixels { id, mask } => {
+                ops::adjust::invert_layer_with(doc, clips, id, mask)?;
+                let plane = if mask { Plane::Mask } else { Plane::Pixels };
+                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
+            }
+            Command::ApplyFilter { id, params } => {
+                ops::adjust::apply_filter_with(doc, clips, id, &params)?;
+                Ok(Dirty::pixels(vec![id]).within(selection_regions(doc, clips, id, Plane::Pixels)))
+            }
             Command::AddAdjustmentLayer { kind, seed, shadows, highlights } => {
                 let gradient = match (shadows, highlights) { (Some(s), Some(h)) => Some((s, h)), _ => None };
                 ops::adjust::add_adjustment_layer(doc, kind, seed, gradient)?; Ok(Dirty::structure())
@@ -398,7 +453,11 @@ impl Engine {
             Command::MagicWand { at, mode, settings, antialiased } => { ops::selection::magic_wand_select(doc, at, mode, &settings, antialiased)?; Ok(Dirty::structure()) }
             Command::LoadLayerSelection { id, mode, antialiased } => { ops::selection::load_layer_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
             Command::LoadMaskSelection { id, mode, antialiased } => { ops::selection::load_mask_selection(doc, id, mode, antialiased)?; Ok(Dirty::structure()) }
-            Command::ClearSelectedPixels { id, mask } => { ops::selection::clear_selected(doc, clips, id, mask)?; Ok(Dirty { structure: true, canvas: false, layers: if mask { vec![] } else { vec![id] } }) }
+            Command::ClearSelectedPixels { id, mask } => {
+                ops::selection::clear_selected(doc, clips, id, mask)?;
+                let plane = if mask { Plane::Mask } else { Plane::Pixels };
+                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
+            }
             Command::AddMaskFromSelection { id, revealing } => { ops::selection::add_mask_from_selection(doc, clips, id, revealing)?; Ok(Dirty::structure()) }
         })
     }
@@ -414,14 +473,22 @@ impl Engine {
     pub fn undo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
         if self.drop_preview(id) { return Ok(Dirty::everything()); }
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
-        if let Some(before) = s.history.undo(&s.document) { s.document = before; s.history.trim(&s.document); }
+        if let Some(before) = s.history.undo(&s.document) {
+            let after = std::mem::replace(&mut s.document, before);
+            s.lineage.record_return(&after, &s.document);
+            s.history.trim(&s.document);
+        }
         self.clips.retain_current(&s.document);
         Ok(Dirty::everything())
     }
     pub fn redo(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
         if self.drop_preview(id) { return Ok(Dirty::everything()); }
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
-        if let Some(after) = s.history.redo(&s.document) { s.document = after; s.history.trim(&s.document); }
+        if let Some(after) = s.history.redo(&s.document) {
+            let before = std::mem::replace(&mut s.document, after);
+            s.lineage.record_return(&before, &s.document);
+            s.history.trim(&s.document);
+        }
         self.clips.retain_current(&s.document);
         Ok(Dirty::everything())
     }
@@ -432,7 +499,10 @@ impl Engine {
     pub fn revert(&mut self, id: Uuid) -> Result<Dirty, CommandError> {
         self.clear_preview(id);
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
-        if let Some(before) = s.history.revert() { s.document = before; }
+        if let Some(before) = s.history.revert() {
+            let after = std::mem::replace(&mut s.document, before);
+            s.lineage.record_return(&after, &s.document);
+        }
         self.clips.retain_current(&s.document);
         Ok(Dirty::everything())
     }
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -17,6 +17,7 @@ pub mod plan;
 pub mod preview;
 pub mod effects;
 pub mod selection;
+pub mod lineage;
 
 pub use adjust::settings::*;
 pub use adjust::levels::*;
@@ -45,6 +46,7 @@ pub use preview::*;
 pub use effects::*;
 pub use effects::settings::*;
 pub use effects::render::*;
+pub use lineage::*;
 pub use ops::masks::blur_gray;
 pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
 pub use selection::coverage::{rasterize, selection_coverage, selection_coverage_with, SelectionClip, SelectionClips};
```

Create `engine/src/lineage.rs`:

```rust
//! Changed rectangles (Phase 4b-1): which part of a layer's pixels or mask changed from one revision
//! to another. The GPU keeps each texture keyed by the revision it uploaded, asks what changed since
//! (`Engine::pixels_delta`, `Engine::mask_delta`) and uploads only that part; a question survives
//! skipped frames, undo and redo, where a per-command notice would not.
use crate::{Document, PixelRect, Plane, Region};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// How many changes a document remembers. A renderer further behind than this uploads whole.
pub const LINEAGE_LIMIT: usize = 256;

/// One buffer moving from revision `from` to `to`: changed only within `rect` (in the new buffer's
/// grid), or wholly (None: no rectangle was known, or the buffer changed size).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Change { layer: Uuid, plane: Plane, from: u64, to: u64, rect: Option<PixelRect> }

/// A document's recent changes, oldest first.
#[derive(Debug, Default)]
pub struct Lineage { changes: VecDeque<Change> }

/// Every layer buffer of `doc`: its revision and size.
fn planes(doc: &Document) -> HashMap<(Uuid, Plane), (u64, (u32, u32))> {
    let mut out = HashMap::new();
    for l in &doc.layers {
        if let Some(p) = &l.pixels { out.insert((l.id, Plane::Pixels), (l.pixels_revision, (p.width, p.height))); }
        if let Some(m) = &l.mask { out.insert((l.id, Plane::Mask), (l.mask_revision, (m.pixels.width, m.pixels.height))); }
    }
    out
}

impl Lineage {
    fn push(&mut self, change: Change) {
        if self.changes.len() == LINEAGE_LIMIT { self.changes.pop_front(); }
        self.changes.push_back(change);
    }
    /// Records how `after` differs from `before` after an edit: every buffer whose revision moved,
    /// within the union of the `regions` reported for it when it kept its size, wholly otherwise.
    pub fn record_edit(&mut self, before: &Document, after: &Document, regions: &[Region]) {
        let old = planes(before);
        for ((layer, plane), (to, size)) in planes(after) {
            let was = old.get(&(layer, plane));
            if was.map(|w| w.0) == Some(to) { continue; }
            let reported = regions.iter().filter(|r| r.layer == layer && r.plane == plane).map(|r| r.rect).reduce(|a, b| a.union(&b));
            let rect = if was.map(|w| w.1) == Some(size) { reported } else { None };
            self.push(Change { layer, plane, from: was.map_or(0, |w| w.0), to, rect });
        }
    }
    /// Records an undo, redo or revert from `before` to `after`: each buffer whose revision moved
    /// changed where the edit joining those two revisions changed it (in either direction), or wholly
    /// when that edit is no longer remembered.
    pub fn record_return(&mut self, before: &Document, after: &Document) {
        let old = planes(before);
        for ((layer, plane), (to, size)) in planes(after) {
            let was = old.get(&(layer, plane));
            let Some(&(from, old_size)) = was else { self.push(Change { layer, plane, from: 0, to, rect: None }); continue };
            if from == to { continue; }
            let joined = self.changes.iter().rev().find(|c| c.layer == layer && c.plane == plane
                && ((c.from == from && c.to == to) || (c.from == to && c.to == from)));
            let rect = if old_size == size { joined.and_then(|c| c.rect) } else { None };
            self.push(Change { layer, plane, from, to, rect });
        }
    }
    /// What changed in `layer`'s `plane` from revision `from` to `current`: an empty rectangle when
    /// nothing did, the union of the changes in between when every one of them had a rectangle, and
    /// None when the whole buffer must be taken again.
    pub fn delta(&self, layer: Uuid, plane: Plane, from: u64, current: u64) -> Option<PixelRect> {
        let mut at = current;
        let mut union = PixelRect::default();
        for _ in 0..LINEAGE_LIMIT {
            if at == from { return Some(union); }
            let c = self.changes.iter().rev().find(|c| c.layer == layer && c.plane == plane && c.to == at)?;
            union = union.union(&c.rect?);
            at = c.from;
        }
        None
    }
}
```

```diff
--- a/engine/src/raster.rs
+++ b/engine/src/raster.rs
@@ -2,6 +2,68 @@ use std::sync::{Arc, Mutex};
 
 pub const TILE: u32 = 256;
 
+/// A rectangle of a pixel grid (a layer's pixels, or its mask's), in whole pixels.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
+pub struct PixelRect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
+
+impl PixelRect {
+    pub fn is_empty(&self) -> bool { self.width == 0 || self.height == 0 }
+    /// The smallest rectangle holding both; an empty one adds nothing.
+    pub fn union(&self, other: &PixelRect) -> PixelRect {
+        if self.is_empty() { return *other; }
+        if other.is_empty() { return *self; }
+        let (x0, y0) = (self.x.min(other.x), self.y.min(other.y));
+        let (x1, y1) = ((self.x + self.width).max(other.x + other.width), (self.y + self.height).max(other.y + other.height));
+        PixelRect { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }
+    }
+    /// The pixels of a grid halved to `width` x `height` whose 2 x 2 block meets this rectangle.
+    pub fn halved(&self, width: u32, height: u32) -> PixelRect {
+        if self.is_empty() { return PixelRect::default(); }
+        let (x0, y0) = ((self.x / 2).min(width), (self.y / 2).min(height));
+        let (x1, y1) = ((self.x + self.width).div_ceil(2).min(width), (self.y + self.height).div_ceil(2).min(height));
+        PixelRect { x: x0, y: y0, width: x1.saturating_sub(x0), height: y1.saturating_sub(y0) }
+    }
+}
+
+/// Writes the 2 x 2 box average of `src` (`sw` x `sh`, premultiplied RGBA) into the pixels of `out`
+/// (the halved grid, `sw / 2` x `sh / 2`, at least 1) that `region` covers. A block past an odd edge
+/// averages the pixels that exist (only a side of 1 has one), rounding half up.
+fn halve_into(src: &[u8], sw: u32, sh: u32, out: &mut [u8], region: PixelRect) {
+    let w = (sw / 2).max(1) as usize;
+    let (sw, sh) = (sw as usize, sh as usize);
+    let (x0, x1) = (region.x as usize, (region.x + region.width) as usize);
+    if sw >= 2 && sh >= 2 {
+        // Every block is whole (an odd last row or column is left out): four pixels, (sum + 2) / 4.
+        for y in region.y as usize..(region.y + region.height) as usize {
+            let top = &src[2 * y * sw * 4 + 8 * x0..2 * y * sw * 4 + 8 * x1];
+            let bottom = &src[(2 * y + 1) * sw * 4 + 8 * x0..(2 * y + 1) * sw * 4 + 8 * x1];
+            let line = &mut out[(y * w + x0) * 4..(y * w + x1) * 4];
+            for ((a, b), o) in top.chunks_exact(8).zip(bottom.chunks_exact(8)).zip(line.chunks_exact_mut(4)) {
+                for c in 0..4 { o[c] = ((a[c] as u16 + a[4 + c] as u16 + b[c] as u16 + b[4 + c] as u16 + 2) >> 2) as u8; }
+            }
+        }
+        return;
+    }
+    for y in region.y as usize..(region.y + region.height) as usize {
+        let (r0, r1) = (2 * y, (2 * y + 1).min(sh - 1));
+        let rows = if 2 * y + 1 < sh { 2 } else { 1 };
+        let top = &src[r0 * sw * 4..(r0 + 1) * sw * 4];
+        let bottom = &src[r1 * sw * 4..(r1 + 1) * sw * 4];
+        let line = &mut out[y * w * 4..(y + 1) * w * 4];
+        for x in region.x as usize..(region.x + region.width) as usize {
+            let (c0, c1) = (2 * x * 4, (2 * x + 1).min(sw - 1) * 4);
+            let cols = if 2 * x + 1 < sw { 2 } else { 1 };
+            let n = (rows * cols) as u32;
+            for c in 0..4 {
+                let mut sum = top[c0 + c] as u32;
+                if cols == 2 { sum += top[c1 + c] as u32; }
+                if rows == 2 { sum += bottom[c0 + c] as u32; if cols == 2 { sum += bottom[c1 + c] as u32; } }
+                line[x * 4 + c] = ((sum + n / 2) / n) as u8;
+            }
+        }
+    }
+}
+
 /// Backing storage for a `Raster`: the pixels, plus a memoized single-step box-reduction so
 /// `halved()` computed once for a given pixel buffer is shared by every clone of it (and by
 /// every clone of the halved result in turn), instead of being recomputed on every frame.
@@ -105,25 +167,33 @@ impl Raster {
         if self.width == 0 || self.height == 0 { return self.clone(); }
         let mut cache = self.inner.half.lock().unwrap();
         if let Some(r) = cache.as_ref() { return r.clone(); }
-        let w = (self.width / 2).max(1); let h = (self.height / 2).max(1);
+        let (w, h) = self.half_size();
         let mut out = vec![0u8; (w * h * 4) as usize];
-        for y in 0..h { for x in 0..w {
-            let mut sum = [0u32; 4]; let mut n = 0;
-            for dy in 0..2 { for dx in 0..2 {
-                let sx = x * 2 + dx; let sy = y * 2 + dy;
-                if sx < self.width && sy < self.height {
-                    let p = self.pixel(sx, sy);
-                    for c in 0..4 { sum[c] += p[c] as u32; }
-                    n += 1;
-                }
-            }}
-            let i = ((y * w + x) * 4) as usize;
-            for c in 0..4 { out[i + c] = ((sum[c] + n.max(1) / 2) / n.max(1)) as u8; }
-        }}
+        halve_into(&self.inner.data, self.width, self.height, &mut out, PixelRect { x: 0, y: 0, width: w, height: h });
         let result = Raster::from_premultiplied(w, h, out);
         *cache = Some(result.clone());
         result
     }
+    /// The size `halved` makes: half of each side, at least 1.
+    pub fn half_size(&self) -> (u32, u32) { ((self.width / 2).max(1), (self.height / 2).max(1)) }
+    /// The memoized halving, if it has been made (and not let go).
+    pub fn memoized_half(&self) -> Option<Raster> { self.inner.half.lock().unwrap().clone() }
+    /// Gives this raster the halvings `parent` has already made, for a raster that equals `parent`
+    /// outside `rect` (a changed rectangle: an edit that kept the grid). Each kept level is copied and
+    /// only the part `rect` reaches is halved again, so the result is bit-identical to halving from
+    /// scratch at a fraction of the cost. Levels the parent never made are left to be made on demand.
+    pub fn seed_halvings(&self, parent: &Raster, rect: PixelRect) {
+        if self.width != parent.width || self.height != parent.height || self.same_pixels(parent) { return; }
+        let Some(parent_half) = parent.memoized_half() else { return };
+        let (w, h) = self.half_size();
+        // Every output pixel whose 2 x 2 block meets `rect`.
+        let reach = rect.halved(w, h);
+        let mut data = parent_half.bytes().to_vec();
+        if !reach.is_empty() { halve_into(&self.inner.data, self.width, self.height, &mut data, reach); }
+        let half = Raster::from_premultiplied(w, h, data);
+        half.seed_halvings(&parent_half, reach);
+        *self.inner.half.lock().unwrap() = Some(half);
+    }
     /// A copy of the `w` x `h` rectangle at (x, y); clamped to the raster.
     pub fn cropped(&self, x: u32, y: u32, w: u32, h: u32) -> Raster {
         let x1 = (x + w).min(self.width); let y1 = (y + h).min(self.height);
```

```diff
--- a/engine/src/selection/coverage.rs
+++ b/engine/src/selection/coverage.rs
@@ -6,7 +6,7 @@
 //! PixelAdjust.swift:23-34).
 use super::{Contour, Selection, SUBPIXEL};
 use super::feather::feather_blur;
-use crate::{Affine, Document, GrayRaster, Point};
+use crate::{Affine, Document, GrayRaster, PixelRect, Point};
 use std::sync::{Arc, Mutex};
 use uuid::Uuid;
 
@@ -172,6 +172,25 @@ impl SelectionClip {
         crate::compositor::gray_sample(c, x, y, false)
     }
 
+    /// The part of a `width` x `height` pixel grid that `to_document` places on the canvas where
+    /// `on_grid` can give coverage above 0: the region's rectangle mapped onto the grid, a pixel
+    /// wider on every side for the bilinear sampling, cut to the grid (empty when it misses). None
+    /// for an empty selection, which clips every edit away, or a grid that cannot be inverted.
+    pub fn rect_on_grid(&self, to_document: &Affine, width: u32, height: u32) -> Option<PixelRect> {
+        let c = self.coverage.as_ref()?;
+        let inverse = to_document.invert()?;
+        let (x0, y0) = (self.origin.0 as f64, self.origin.1 as f64);
+        let (x1, y1) = (x0 + c.width as f64, y0 + c.height as f64);
+        let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| inverse.apply(Point { x, y }));
+        let (lx, hx) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.x), h.max(p.x)));
+        let (ly, hy) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.y), h.max(p.y)));
+        let gx0 = (lx.floor() - 1.0).clamp(0.0, width as f64) as u32;
+        let gy0 = (ly.floor() - 1.0).clamp(0.0, height as f64) as u32;
+        let gx1 = (hx.ceil() + 1.0).clamp(0.0, width as f64) as u32;
+        let gy1 = (hy.ceil() + 1.0).clamp(0.0, height as f64) as u32;
+        Some(PixelRect { x: gx0, y: gy0, width: gx1.saturating_sub(gx0), height: gy1.saturating_sub(gy0) })
+    }
+
     /// The coverage on a `width` x `height` pixel grid that `to_document` places on the canvas (a
     /// layer's pixels or its mask's, `pixel_to_document`), each pixel sampled at its centre
     /// (`PixelAdjust.coverage`). A grid on the canvas's own pixels, moved by whole pixels, copies
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test changed_rects` (7 tests), then the whole suite: 474 passed, 6 ignored (+7, +1 ignored). Natively: the halving chain to level 3 22 ms at 24 MP and 100 ms at 100 MP; a clear in a 1024 px selection 53 / 263 ms, after which level 3 costs 0.00 ms (seeded).

- [ ] **Step 5: Prove it bites**

(1) In `Raster::seed_halvings`, return at once: `an_edit_inside_a_selection_hands_the_new_pixels_the_old_halvings` and `seeded_halvings_equal_halving_from_scratch` fail (measured). Restore. (2) In `Lineage::record_return`, record `rect: None`: `a_clear_inside_a_selection_reports_where_it_changed_and_undo_and_redo_follow` fails at the undo (measured). Restore. (3) In `Lineage::record_edit`, keep the reported rectangle even when the buffer changed size: `a_mask_edit_inside_a_selection_reports_its_rectangle_on_the_mask` fails (its first mask edit changes the mask's size; measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/lineage.rs engine/tests/changed_rects.rs
git commit -m "feat(engine): edits report the rectangle they changed, and new rasters keep the old halvings outside it" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine/src/command.rs engine/src/engine.rs engine/src/lib.rs engine/src/lineage.rs engine/src/raster.rs engine/src/selection/coverage.rs engine/tests/changed_rects.rs engine/tests/perf_4b1.rs
```


---

### Task 4: The GPU uploads only the rectangle that changed

With Task 3's deltas, the layer and mask textures take `texSubImage2D` of the changed rectangle at the texture's level (a level-L pixel is every source block it covers: `levelRect`), chunk by chunk for layers over 2048 px, straight from the engine's raster (`UNPACK_ROW_LENGTH`, `SKIP_PIXELS`, `SKIP_ROWS`). A texture remembers the revision it holds; anything the engine cannot account for is uploaded whole as before.

**Files:**
- Modify: `app/src/canvas/layer-textures.ts` (`levelRect`, `revision`, `update`), `app/src/canvas/gl/mask-textures.ts`, `app/src/canvas/gl-renderer.ts`, `app/src/engine/client.ts` (`pixelsDelta`, `maskDelta`), `app/src/engine/types.ts` (`PixelRect`), `engine-wasm/src/lib.rs`
- Create tests: `app/tests/unit/partial-upload.test.ts`, `app/tests/e2e/partial-upload.spec.ts`; modify `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `levelRect(rect, level, width, height)`, `LayerTextures.update(...)`, `MaskTextures.sync(..., delta?)`, `EngineClient.pixelsDelta / maskDelta(doc, layer, from): PixelRect | null`.

- [ ] **Step 1: Write the tests**

The unit tests stub WebGL2 and record every upload with its pixel-store state: `levelRect` against an independent block count at levels 0-3 on a 37 x 29 layer (odd sizes, rectangles at both edges); a rectangle across the 2048 px chunk edges uploads four pieces, each from its own part of the source; a mask takes the rectangle when the engine knows it and the whole mask otherwise. The e2e clears a selection in the middle of a 2400 x 1000 noise layer at 1:1 and at fit, counts the uploads (no `texImage2D`, at most the selection's box plus two pixels a side), and compares the picture with a whole upload of the same document, byte for byte.

Create `app/tests/e2e/partial-upload.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: an edit that knows where it changed a layer (engine/tests/changed_rects.rs) reaches the
// GPU as that rectangle alone, and draws exactly what a whole upload draws.

/** Counts every texture upload the page makes, and the texels `texSubImage2D` sends. */
async function countUploads(page: Page) {
  await page.addInitScript(() => {
    const log = { image: 0, imageMax: 0, sub: 0, subArea: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) {
      log.image++;
      if (typeof args[3] === "number" && typeof args[4] === "number") log.imageMax = Math.max(log.imageMax, (args[3] as number) * (args[4] as number));
      return image.apply(this, args);
    };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; log.subArea += (args[4] as number) * (args[5] as number); return sub.apply(this, args); };
  });
}
const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
const uploads = (page: Page) => page.evaluate(() => ({ ...(window as any).__uploads }));
const resetUploads = (page: Page) => page.evaluate(() => { const u = (window as any).__uploads; u.image = 0; u.imageMax = 0; u.sub = 0; u.subArea = 0; });
/** Keeps the GPU's picture of the document in the page (reading it out costs more than the test). */
const keepPicture = (page: Page) => page.evaluate(() => { (window as any).__kept = (window as any).__compositor.readDocumentPixels(); });
/** The largest channel difference between the GPU's picture now and the kept one; -1 for another size. */
const worstAgainstKept = (page: Page) => page.evaluate(() => {
  const now = (window as any).__compositor.readDocumentPixels() as Uint8Array, kept = (window as any).__kept as Uint8Array;
  if (now.length !== kept.length) return -1;
  let worst = 0;
  for (let i = 0; i < now.length; i++) worst = Math.max(worst, Math.abs(now[i] - kept[i]));
  return worst;
});

for (const zoom of ["1:1", "fit"] as const) {
  test(`a clear inside a selection uploads only its rectangle at ${zoom}, and draws what a whole upload draws`, async ({ page }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 1280, height: 720 });
    await countUploads(page);
    await page.goto("/");
    await expect(page.getByTestId("engine-ready")).toBeVisible();
    const ids = await page.evaluate(async (zoom) => {
      const api = (window as any).__compositor;
      const doc = api.engine.newDocument(10, 10, false);
      api.engine.execute(doc, { type: "CanvasSize", width: 2400, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
      const layer = api.engine.state(doc).layers[0].id;
      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
      api.engine.execute(doc, { type: "ApplyFilter", id: layer, params: { filter: "AddNoise", amount: 30, gaussian: false, monochromatic: false, seed: 5 } });
      api.store.getState().openDocument(doc);
      if (zoom === "1:1") await api.setZoom(1);
      return { doc, layer };
    }, zoom);
    await frames(page);
    // A selection around the middle, which the view shows at either zoom.
    await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[1010, 370], [1390, 370], [1390, 630], [1010, 630]], mode: "Replace", antialiased: false }));
    await frames(page);
    await resetUploads(page);
    await page.evaluate((layer) => (window as any).__compositor.store.getState().run({ type: "ClearSelectedPixels", id: layer, mask: false }), ids.layer);
    await frames(page);
    const partial = await uploads(page);
    expect(partial.image, "no texture made afresh").toBe(0);
    expect(partial.sub).toBeGreaterThan(0);
    // The selection's 380 x 260 box plus the clip's pixel and the sampling pixel on each side, at most.
    expect(partial.subArea).toBeLessThanOrEqual(384 * 264 + 16);
    await keepPicture(page);
    // The same document again after another one: its textures were dropped, so it uploads whole.
    await resetUploads(page);
    await page.evaluate(async (doc) => {
      const api = (window as any).__compositor;
      api.store.getState().openDocument(api.engine.newDocument(10, 10, true));
      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
      api.store.getState().setActive(doc);
    }, ids.doc);
    await frames(page);
    // At fit the layer is drawn halved once (1200 x 500); at 1:1 in chunks of up to 2048 x 1000.
    expect((await uploads(page)).imageMax, "uploaded whole this time").toBeGreaterThanOrEqual(1200 * 500);
    expect(await worstAgainstKept(page), "a partial upload draws exactly what a whole one does").toBe(0);
  });
}
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -8,12 +8,90 @@ import { test, expect } from "@playwright/test";
 test.skip(!process.env.PERF, "set PERF=1 after pnpm wasm to measure");
 test.use({ channel: "msedge", viewport: { width: 1440, height: 900 } });
 
-/** Opens the page and waits for the engine. */
+/** Opens the page and waits for the engine; every texture upload is counted in `__uploads`. */
 async function ready(page: import("@playwright/test").Page) {
+  await page.addInitScript(() => {
+    const log = { image: 0, sub: 0 };
+    (window as any).__uploads = log;
+    const proto = WebGL2RenderingContext.prototype as any;
+    const image = proto.texImage2D, sub = proto.texSubImage2D;
+    proto.texImage2D = function (...args: unknown[]) { log.image++; return image.apply(this, args); };
+    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; return sub.apply(this, args); };
+  });
   await page.goto("/");
   await expect(page.getByTestId("engine-ready")).toBeVisible();
 }
 
+/** Installs `__frame()` in the page: one render of the active document through the app's renderer,
+ * ended by a 1 x 1 readPixels so the GPU work lands inside the timing; returns its milliseconds.
+ * `__lastUploads` then says how many whole and partial texture uploads that frame made. */
+async function installFrameTimer(page: import("@playwright/test").Page) {
+  await page.evaluate(() => {
+    const api = (window as any).__compositor;
+    const gl = (document.querySelector('[data-testid="canvas-view"] canvas') as HTMLCanvasElement).getContext("webgl2")!;
+    const px = new Uint8Array(4);
+    (window as any).__frame = () => {
+      const s = api.store.getState();
+      const log = (window as any).__uploads; log.image = 0; log.sub = 0;
+      const t0 = performance.now();
+      api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: true }, s.previewEdit());
+      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
+      const ms = performance.now() - t0;
+      (window as any).__lastUploads = `${log.image} whole, ${log.sub} partial`;
+      return ms;
+    };
+  });
+}
+
+test("partial uploads: the frame after an edit inside a selection, at fit and at 1:1, 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    // A fresh page per size: nothing of the other size's document is left in memory or on the GPU.
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      api.store.getState().openDocument(doc);
+      await settle(); frame();
+      const result: Record<string, number> = {};
+      // A 1024 x 1024 selection in the middle, cleared: the engine's edit, then the next frame.
+      const box = (x: number, y: number) => ({ type: "SelectShape", kind: "Rectangle", points: [[x, y], [x + 1024, y], [x + 1024, y + 1024], [x, y + 1024]], mode: "Replace", antialiased: false });
+      for (const zoom of ["fit", "1:1"]) {
+        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
+        const at = zoom === "fit" ? [w / 2 - 1500, h / 2 - 1500] : [w / 2 - 512, h / 2 - 512];
+        api.engine.execute(doc, box(at[0], at[1]));
+        api.store.getState().refresh(doc); frame(); await settle();
+        let t0 = performance.now();
+        api.engine.execute(doc, { type: "ClearSelectedPixels", id: layer, mask: false });
+        result[`clear in 1024 px at ${zoom}, engine`] = Math.round(performance.now() - t0);
+        api.store.getState().refresh(doc);
+        result[`frame after it at ${zoom}`] = Math.round(10 * frame()) / 10;
+        result[`uploads in that frame at ${zoom}: ${(window as any).__lastUploads}`] = 0;
+        await settle();
+        // For comparison: a whole-layer edit, and the frame after it (halving and a whole upload).
+        api.engine.execute(doc, { type: "Deselect" });
+        t0 = performance.now();
+        api.engine.execute(doc, { type: "InvertPixels", id: layer, mask: false });
+        result[`invert (whole) at ${zoom}, engine`] = Math.round(performance.now() - t0);
+        api.store.getState().refresh(doc);
+        result[`frame after the whole edit at ${zoom}`] = Math.round(frame());
+        await settle();
+      }
+      api.store.getState().closeDocument(doc);
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`partial uploads (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) for (const zoom of ["fit", "1:1"]) expect(out[`${label}: frame after it at ${zoom}`]).toBeLessThan(33);
+});
+
 test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push at the cap is cheap", async ({ page }) => {
   test.setTimeout(900_000);
   await ready(page);
```

Create `app/tests/unit/partial-upload.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { LayerTextures, levelRect, sizeAtLevel } from "../../src/canvas/layer-textures";
import { MaskTextures } from "../../src/canvas/gl/mask-textures";
import type { PixelRect } from "../../src/engine/types";

/** Just enough of WebGL2 for the texture caches, recording every upload with the pixel-store state. */
function stubGl() {
  const calls: { kind: "image" | "sub"; x: number; y: number; width: number; height: number; skipX: number; skipY: number; rowLength: number }[] = [];
  const store: Record<number, number> = {};
  let next = 1;
  const gl = {
    TEXTURE_2D: 1, R8: 2, RED: 3, UNSIGNED_BYTE: 4, UNPACK_ALIGNMENT: 5, UNPACK_ROW_LENGTH: 6,
    UNPACK_SKIP_PIXELS: 7, UNPACK_SKIP_ROWS: 8, TEXTURE_WRAP_S: 9, TEXTURE_WRAP_T: 10,
    CLAMP_TO_EDGE: 11, TEXTURE_MIN_FILTER: 12, TEXTURE_MAG_FILTER: 13, NEAREST: 14, LINEAR: 15, RGBA8: 16, RGBA: 17, UNPACK_PREMULTIPLY_ALPHA_WEBGL: 18,
    createTexture: () => ({ id: next++ }) as unknown as WebGLTexture,
    bindTexture: () => {}, texParameteri: () => {}, deleteTexture: () => {},
    pixelStorei: (k: number, v: number) => { store[k] = v; },
    texImage2D: (_t: number, _l: number, _i: number, width: number, height: number) => {
      calls.push({ kind: "image", x: 0, y: 0, width, height, skipX: store[7] ?? 0, skipY: store[8] ?? 0, rowLength: store[6] ?? 0 });
    },
    texSubImage2D: (_t: number, _l: number, x: number, y: number, width: number, height: number) => {
      calls.push({ kind: "sub", x, y, width, height, skipX: store[7] ?? 0, skipY: store[8] ?? 0, rowLength: store[6] ?? 0 });
    },
  } as unknown as WebGL2RenderingContext;
  return { gl, calls };
}

/** Independently of `levelRect`: the pixels after `level` halvings whose block of source pixels,
 * [x * 2^level, (x + 1) * 2^level) on each axis, meets `rect`. */
function reached(rect: PixelRect, level: number, width: number, height: number): PixelRect {
  const size = sizeAtLevel(width, height, level), f = 2 ** level;
  const xs: number[] = [], ys: number[] = [];
  for (let x = 0; x < size.width; x++) if (x * f < rect.x + rect.width && (x + 1) * f > rect.x) xs.push(x);
  for (let y = 0; y < size.height; y++) if (y * f < rect.y + rect.height && (y + 1) * f > rect.y) ys.push(y);
  if (xs.length === 0 || ys.length === 0) return { x: 0, y: 0, width: 0, height: 0 };
  return { x: xs[0], y: ys[0], width: xs.length, height: ys.length };
}

describe("levelRect", () => {
  it("takes every reduced pixel whose block the change reaches, at every level", () => {
    for (const rect of [{ x: 5, y: 7, width: 9, height: 4 }, { x: 30, y: 20, width: 7, height: 9 }, { x: 0, y: 0, width: 1, height: 1 }, { x: 36, y: 28, width: 1, height: 1 }]) {
      for (const level of [0, 1, 2, 3]) expect(levelRect(rect, level, 37, 29), `${JSON.stringify(rect)} at ${level}`).toEqual(reached(rect, level, 37, 29));
    }
  });
  it("is empty for an empty change", () => {
    expect(levelRect({ x: 4, y: 4, width: 0, height: 3 }, 2, 37, 29)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
  });
});

describe("LayerTextures.update", () => {
  it("uploads only the changed rectangle into each chunk it meets, straight from the raster", () => {
    const { gl, calls } = stubGl();
    const textures = new LayerTextures(gl);
    // 5000 x 3000: chunks start at x 0, 2048, 4096 and y 0, 2048.
    textures.sync("D", "A", "px:1", false, new Uint8Array(4), 0, { width: 5000, height: 3000 }, 1);
    expect(calls.filter((c) => c.kind === "image").length).toBe(6);
    calls.length = 0;
    textures.update("D", "A", "px:2", 2, { x: 2000, y: 1000, width: 100, height: 1100 }, new Uint8Array(4));
    // Across the x = 2048 edge and the y = 2048 edge: four pieces, each placed in its own chunk.
    expect(calls).toEqual([
      { kind: "sub", x: 2000, y: 1000, width: 48, height: 1048, skipX: 2000, skipY: 1000, rowLength: 5000 },
      { kind: "sub", x: 0, y: 1000, width: 52, height: 1048, skipX: 2048, skipY: 1000, rowLength: 5000 },
      { kind: "sub", x: 2000, y: 0, width: 48, height: 52, skipX: 2000, skipY: 2048, rowLength: 5000 },
      { kind: "sub", x: 0, y: 0, width: 52, height: 52, skipX: 2048, skipY: 2048, rowLength: 5000 },
    ]);
    const t = textures.get("D", "A")!;
    expect([t.key, t.revision]).toEqual(["px:2", 2]);
  });
});

describe("MaskTextures partial uploads", () => {
  it("takes the changed rectangle when the engine knows it, and the whole mask otherwise", () => {
    const { gl, calls } = stubGl();
    const masks = new MaskTextures(gl);
    masks.sync("D", "A", 1, 60, 40, new Uint8Array(2400));
    const asked: number[] = [];
    masks.sync("D", "A", 2, 60, 40, new Uint8Array(2400), (from) => { asked.push(from); return { x: 3, y: 5, width: 7, height: 9 }; });
    expect(asked).toEqual([1]);
    expect(calls.at(-1)).toEqual({ kind: "sub", x: 3, y: 5, width: 7, height: 9, skipX: 3, skipY: 5, rowLength: 60 });
    expect(masks.has("D", "A", 2)).toBe(true);
    masks.sync("D", "A", 3, 60, 40, new Uint8Array(2400), () => null);
    expect(calls.at(-1)?.kind).toBe("image");
    // Another size is always uploaded whole, without asking.
    const before = asked.length;
    masks.sync("D", "A", 4, 30, 40, new Uint8Array(1200), (from) => { asked.push(from); return { x: 0, y: 0, width: 1, height: 1 }; });
    expect(asked.length).toBe(before);
    expect(calls.at(-1)).toMatchObject({ kind: "image", width: 30, height: 40 });
  });
});
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: the unit test does not compile (`levelRect`, `update`). The e2e fails at "no texture made afresh".

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/gl-renderer.ts
+++ b/app/src/canvas/gl-renderer.ts
@@ -2,7 +2,7 @@ import type { Coverage, DocumentState, LayerDraw, PreviewEdit, RenderPlan } from
 import { DEFAULT_BLACK_WHITE, DEFAULT_COLOR_BALANCE, isSpatialKind, type AdjustmentKind } from "../engine/types";
 import type { EngineClient } from "../engine/client";
 import type { Viewport } from "./viewport";
-import { LayerTextures, prefilterLevel, sizeAtLevel } from "./layer-textures";
+import { LayerTextures, levelRect, prefilterLevel, sizeAtLevel } from "./layer-textures";
 import type { RenderOptions, Renderer } from "./renderer";
 import { ADJUST_KIND, BLEND_INDEX, createPrograms, disposePrograms, type Program, type Programs } from "./gl/programs";
 import { FboPool, type Target } from "./gl/framebuffers";
@@ -121,7 +121,18 @@ export class GlRenderer implements Renderer {
       if (!this.textures.needsUpload(state.id, layer.id, bytesKey, level, nearest)) continue;
       const [width, height] = fx ? [fx.width, fx.height] : [layer.pixelsWidth, layer.pixelsHeight];
       const size = sizeAtLevel(width, height, level);
-      const upload = (pixels: Uint8Array | null) => this.textures.sync(state.id, layer.id, bytesKey, nearest, pixels, level, size);
+      // Plain pixels at the same level and size: ask what changed since the uploaded revision and
+      // upload only that (Engine::pixels_delta); a change the engine cannot bound goes whole.
+      const kept = fx ? undefined : this.textures.get(state.id, layer.id);
+      if (kept && kept.revision !== null && kept.level === level && kept.nearest === nearest && kept.width === size.width && kept.height === size.height) {
+        const delta = engine.pixelsDelta(state.id, layer.id, kept.revision);
+        if (delta) {
+          const rect = levelRect(delta, level, width, height);
+          const pixels = rect.width > 0 && rect.height > 0 ? engine.layerPixels(state.id, layer.id, level) : null;
+          if (rect.width === 0 || rect.height === 0 || pixels) { this.textures.update(state.id, layer.id, bytesKey, layer.pixelsRevision, rect, pixels ?? new Uint8Array(0)); continue; }
+        }
+      }
+      const upload = (pixels: Uint8Array | null) => this.textures.sync(state.id, layer.id, bytesKey, nearest, pixels, level, size, fx ? null : layer.pixelsRevision);
       // An effects image is dropped by the engine as soon as the upload has copied it.
       if (fx) engine.drawPixels(state.id, layer.id, level, edit, upload);
       else upload(width === 0 ? null : engine.layerPixels(state.id, layer.id, level));
@@ -165,7 +176,12 @@ export class GlRenderer implements Renderer {
 
   private syncMasks(engine: EngineClient, state: DocumentState, plan: RenderPlan): void {
     const keep = new Set<string>();
-    const visit = (c: Coverage) => { keep.add(c.layerId); const px = engine.maskPixels(state.id, c.layerId); if (px) this.masks.sync(state.id, c.layerId, c.maskRevision, c.width, c.height, px); };
+    const visit = (c: Coverage) => {
+      keep.add(c.layerId);
+      if (this.masks.has(state.id, c.layerId, c.maskRevision)) return;
+      const px = engine.maskPixels(state.id, c.layerId);
+      if (px) this.masks.sync(state.id, c.layerId, c.maskRevision, c.width, c.height, px, (from) => engine.maskDelta(state.id, c.layerId, from));
+    };
     for (const n of plan.nodes) { if (n.kind === "layer") n.draw.coverages.forEach(visit); else { n.base.coverages.forEach(visit); n.children.forEach((c) => c.coverages.forEach(visit)); n.folderCoverages.forEach(visit); } }
     for (const s of plan.sources) s.coverages.forEach(visit);
     this.masks.retainOnly(state.id, keep);
```

```diff
--- a/app/src/canvas/gl/mask-textures.ts
+++ b/app/src/canvas/gl/mask-textures.ts
@@ -1,4 +1,6 @@
-interface Entry { tex: WebGLTexture; revision: number; }
+import type { PixelRect } from "../../engine/types";
+
+interface Entry { tex: WebGLTexture; revision: number; width: number; height: number; }
 export class MaskTextures {
   private masks = new Map<string, Entry>();
   constructor(private readonly gl: WebGL2RenderingContext) {}
@@ -7,17 +9,38 @@ export class MaskTextures {
    * this rather than re-entering `sync` with an empty buffer: on a cache miss `sync` would call
    * texImage2D with an undersized buffer, raise INVALID_OPERATION and cache a garbage mask. */
   get(docId: string, layerId: string): WebGLTexture | undefined { return this.masks.get(this.key(docId, layerId))?.tex; }
-  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array): WebGLTexture {
+  /** Whether the mask at `revision` is already uploaded: nothing to read from the engine. */
+  has(docId: string, layerId: string, revision: number): boolean { return this.masks.get(this.key(docId, layerId))?.revision === revision; }
+  /** Uploads the mask at `revision` unless it is there. A mask of the same size already uploaded at
+   * another revision takes only what changed since (`delta`, the engine's `mask_delta`: a rectangle,
+   * or null for the whole mask), with `texSubImage2D` straight from `pixels`. */
+  sync(docId: string, layerId: string, revision: number, width: number, height: number, pixels: Uint8Array, delta?: (from: number) => PixelRect | null): WebGLTexture {
     const k = this.key(docId, layerId); const e = this.masks.get(k);
     if (e && e.revision === revision) return e.tex;
-    if (e) this.gl.deleteTexture(e.tex);
-    const gl = this.gl; const tex = gl.createTexture()!;
+    const gl = this.gl;
+    if (e && e.width === width && e.height === height && delta) {
+      const rect = delta(e.revision);
+      if (rect) {
+        if (rect.width > 0 && rect.height > 0) {
+          gl.bindTexture(gl.TEXTURE_2D, e.tex);
+          gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, width);
+          gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, rect.x); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, rect.y);
+          gl.texSubImage2D(gl.TEXTURE_2D, 0, rect.x, rect.y, rect.width, rect.height, gl.RED, gl.UNSIGNED_BYTE, pixels);
+          gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
+          gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
+        }
+        e.revision = revision;
+        return e.tex;
+      }
+    }
+    if (e) gl.deleteTexture(e.tex);
+    const tex = gl.createTexture()!;
     gl.bindTexture(gl.TEXTURE_2D, tex);
     gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1); gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0); gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0); gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
     gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, width, height, 0, gl.RED, gl.UNSIGNED_BYTE, pixels);
     gl.pixelStorei(gl.UNPACK_ALIGNMENT, 4);
     gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
-    this.masks.set(k, { tex, revision }); return tex;
+    this.masks.set(k, { tex, revision, width, height }); return tex;
   }
   setFilter(tex: WebGLTexture, nearest: boolean): void {
     const gl = this.gl; gl.bindTexture(gl.TEXTURE_2D, tex);
```

```diff
--- a/app/src/canvas/layer-textures.ts
+++ b/app/src/canvas/layer-textures.ts
@@ -22,9 +22,31 @@ export function sizeAtLevel(width: number, height: number, level: number): { wid
   return { width: w, height: h };
 }
 
+import type { PixelRect } from "../engine/types";
+
+/** The part of the raster after `level` halvings that a changed rectangle of the full raster
+ * (`width` x `height`) reaches: each halving takes every pixel whose 2 x 2 block meets it, cut to the
+ * smaller size, exactly as the engine's `PixelRect::halved`, so a partial upload at a reduced zoom
+ * covers every texel the change moved. */
+export function levelRect(rect: PixelRect, level: number, width: number, height: number): PixelRect {
+  let r = rect, w = width, h = height;
+  for (let i = 0; i < level; i++) {
+    if (w <= 1 || h <= 1) break;
+    w = Math.max(1, Math.floor(w / 2)); h = Math.max(1, Math.floor(h / 2));
+    if (r.width === 0 || r.height === 0) return { x: 0, y: 0, width: 0, height: 0 };
+    const x0 = Math.min(Math.floor(r.x / 2), w), y0 = Math.min(Math.floor(r.y / 2), h);
+    const x1 = Math.min(Math.ceil((r.x + r.width) / 2), w), y1 = Math.min(Math.ceil((r.y + r.height) / 2), h);
+    r = { x: x0, y: y0, width: Math.max(0, x1 - x0), height: Math.max(0, y1 - y0) };
+  }
+  // A change only in a dropped odd last row or column reaches nothing.
+  return r.width === 0 || r.height === 0 ? { x: 0, y: 0, width: 0, height: 0 } : r;
+}
+
 export interface Chunk { texture: WebGLTexture; x: number; y: number; width: number; height: number; }
-/** `key` names the bytes: the layer's pixels at a revision, or its effects image (GlRenderer.syncTextures). */
-export interface LayerTexture { key: string; level: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }
+/** `key` names the bytes: the layer's pixels at a revision, or its effects image (GlRenderer.syncTextures).
+ * `revision` is the pixels revision uploaded, for asking the engine what changed since (null for an
+ * effects image, which is always uploaded whole). */
+export interface LayerTexture { key: string; revision: number | null; level: number; width: number; height: number; nearest: boolean; chunks: Chunk[]; }
 
 /** Two sessions opened from the same `.comp` file carry identical layer ids (they come from
  * the manifest), so the cache is keyed by document handle *and* layer id -- otherwise one
@@ -51,7 +73,7 @@ export class LayerTextures {
    * No mipmaps: the chain would be built per 2048-pixel chunk with CLAMP_TO_EDGE, which seams
    * a large layer at low zoom, and the CPU compositor has no equivalent. Both renderers instead
    * reduce the whole raster with the same `prefilterLevel` rule and take one LINEAR tap. */
-  sync(docId: string, id: string, bytesKey: string, nearest: boolean, pixels: Uint8Array | null, level: number, size: { width: number; height: number }): void {
+  sync(docId: string, id: string, bytesKey: string, nearest: boolean, pixels: Uint8Array | null, level: number, size: { width: number; height: number }, revision: number | null = null): void {
     const k = key(docId, id);
     const existing = this.layers.get(k);
     if (!pixels || size.width === 0 || size.height === 0) { if (existing) this.remove(docId, id); return; }
@@ -79,7 +101,32 @@ export class LayerTextures {
     gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
     gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
     gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
-    this.layers.set(k, { key: bytesKey, level, width: size.width, height: size.height, nearest, chunks });
+    this.layers.set(k, { key: bytesKey, revision, level, width: size.width, height: size.height, nearest, chunks });
+  }
+  /** Uploads only `rect` (in the texture's own pixels, at its level) of `pixels`, the whole raster at
+   * that level, into every chunk it meets, straight from the view: `texSubImage2D` with the row length
+   * and skips set, no copy. The texture then names `bytesKey` and `revision`. */
+  update(docId: string, id: string, bytesKey: string, revision: number, rect: PixelRect, pixels: Uint8Array): void {
+    const t = this.layers.get(key(docId, id));
+    if (!t) return;
+    const gl = this.gl;
+    if (rect.width > 0 && rect.height > 0) {
+      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, t.width);
+      gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
+      for (const c of t.chunks) {
+        const x0 = Math.max(rect.x, c.x), y0 = Math.max(rect.y, c.y);
+        const x1 = Math.min(rect.x + rect.width, c.x + c.width), y1 = Math.min(rect.y + rect.height, c.y + c.height);
+        if (x1 <= x0 || y1 <= y0) continue;
+        gl.bindTexture(gl.TEXTURE_2D, c.texture);
+        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x0);
+        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y0);
+        gl.texSubImage2D(gl.TEXTURE_2D, 0, x0 - c.x, y0 - c.y, x1 - x0, y1 - y0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
+      }
+      gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
+      gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
+      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
+    }
+    t.key = bytesKey; t.revision = revision;
   }
   remove(docId: string, id: string): void {
     const k = key(docId, id);
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -1,5 +1,8 @@
 import init, { WasmEngine } from "./pkg/compositor_engine.js";
-import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";
+import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, LevelsAuto, LevelsSample, LevelsSettings, PackageFiles, PixelRect, PreviewEdit, PreviewRequest, RenderPlan, SpatialBlur, SpatialGrid } from "./types";
+
+/** `[x, y, width, height]` from the engine as a rectangle; an empty array as null (take it whole). */
+function rectOf(v: ArrayLike<number>): PixelRect | null { return v.length === 4 ? { x: v[0], y: v[1], width: v[2], height: v[3] } : null; }
 
 export class EngineClient {
   private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory) {}
@@ -92,6 +95,11 @@ export class EngineClient {
     const ptr = this.wasm.mask_pixels_ptr(doc, layer);
     return new Uint8Array(this.memory.buffer, ptr, len);
   }
+  /** What changed in the layer's pixels since revision `from` (engine `pixels_delta`): a rectangle of
+   * its pixel grid, empty when nothing did, or null when the whole raster must be uploaded again. */
+  pixelsDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.pixels_delta(doc, layer, from)); }
+  /** `pixelsDelta` for the layer's mask, in the mask's own grid (engine `mask_delta`). */
+  maskDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.mask_delta(doc, layer, from)); }
   clipDependents(doc: string, ids: string[]): string[] { return JSON.parse(this.wasm.clip_dependents(doc, JSON.stringify(ids))) as string[]; }
   mergeAction(doc: string, ids: string[]): string | null { return this.wasm.merge_action(doc, JSON.stringify(ids)) ?? null; }
   groupBox(doc: string, ids: string[]): LayerTransform | null { const t = this.wasm.group_box(doc, JSON.stringify(ids)); return t ? (JSON.parse(t) as LayerTransform) : null; }
```

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -204,5 +204,7 @@ export type Command =
   | { type: "AddMaskFromSelection"; id: string; revealing: boolean };
 
 export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }
+/** A rectangle of a layer's pixel grid (or its mask's), in whole pixels (engine `PixelRect`). */
+export interface PixelRect { x: number; y: number; width: number; height: number; }
 
 export interface PackageFiles { manifest: string; images: { name: string; bytes: Uint8Array }[]; }
```

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -198,6 +198,18 @@ impl WasmEngine {
         let raster = self.engine.composite_edit(parse_id(doc)?, edit.as_ref(), Rect { x, y, width: w, height: h }, out_w, out_h).map_err(js_err)?;
         Ok(Uint8Array::from(raster.bytes()))
     }
+    /// `Engine::pixels_delta`: `[x, y, width, height]` of what changed in the layer's pixels since
+    /// revision `from` (width 0 for nothing), or an empty array when the whole raster must be uploaded.
+    /// Revisions stay below 2^53, so they travel as numbers.
+    pub fn pixels_delta(&self, doc: &str, layer: &str, from: f64) -> Result<Vec<f64>, JsError> {
+        let rect = self.engine.pixels_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
+        Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
+    }
+    /// `Engine::mask_delta`, in the mask's own grid, as `pixels_delta`.
+    pub fn mask_delta(&self, doc: &str, layer: &str, from: f64) -> Result<Vec<f64>, JsError> {
+        let rect = self.engine.mask_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
+        Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
+    }
     pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
         let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
         let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
```


- [ ] **Step 4: Run the tests and watch them pass**

`pnpm wasm:dev`; `pnpm test`: 152 (+4); `pnpm build`; `pnpm e2e`: 134 passed, 6 skipped (the e2e file runs at two zooms: +2). Timings (release wasm, Edge): the frame after a clear in a 1024 px selection 9 ms at fit and 17 ms at 1:1 at 24 MP, 4.3 and 11.7 ms at 100 MP (budget 33 ms), each with one or two partial uploads and no whole one.

- [ ] **Step 5: Prove it bites**

(1) In `levelRect`, divide by 2^level once instead of rounding out to whole blocks level by level: both `levelRect` tests fail (measured), and the fit e2e reads a stale column. Restore. (2) Make `MaskTextures.sync` ignore its delta: `takes the changed rectangle when the engine knows it...` fails (a whole upload). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/partial-upload.spec.ts app/tests/unit/partial-upload.test.ts
git commit -m "feat(app): the GPU uploads only the rectangle an edit changed" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/gl-renderer.ts app/src/canvas/gl/mask-textures.ts app/src/canvas/layer-textures.ts app/src/engine/client.ts app/src/engine/types.ts app/tests/e2e/partial-upload.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/partial-upload.test.ts engine-wasm/src/lib.rs
```


---

### Task 5: Jobs: one layer edited, read or drawn in a second engine

A job copies what one layer's edit needs out of the document - the layer's record, its pixels and mask, the selection's contours, the canvas size, and a stamp of its revisions and placement - into a second engine, runs a command there (or reads a histogram, or makes an effects image), and returns only the buffers that changed and the rectangles they changed within. `install_job` puts an edit's result back as one undo step if the stamp still matches, and refuses otherwise (ruling OQ5). This task is the engine and its wasm entry points; Task 6 runs them in a worker.

**Files:**
- Create: `engine/src/jobs.rs`
- Modify: `engine/src/engine.rs` (`insert_document`; `session`, `edit`, `clear_preview`, `render_document` crate-visible), `engine/src/lib.rs`, `engine-wasm/src/lib.rs` (the job slot and the calls)
- Create tests: `engine/tests/jobs.rs`

**Interfaces:**
- Produces: `JobInput`, `JobOutput`, `JobSelection`, `LayerStamp`, `EffectsImage`, `LAYER_CHANGED`; `Engine::job_input`, `display_job_input`, `install_job`, `insert_document`; `run_edit_job`, `run_histogram_job`, `run_effects_job`; in wasm `prepare_job`, `prepare_display_job`, `job_buffer_ptr` / `len`, `release_job`, `install_job`, `run_edit_job`, `run_histogram_job`, `run_effects_job`.

- [ ] **Step 1: Write the tests**

An edit made by a job and put back equals the edit made in place (Levels on a noise layer, byte for byte, one undo step); a job inside a selection brings its changed rectangle back; a job is not put back onto a layer that changed meanwhile (`LAYER_CHANGED`, nothing recorded); a histogram job equals the histogram in place; an effects job at full size equals the engine's own image, and a reduced one scales its effects with it.

Create `engine/tests/jobs.rs`:

```rust
//! Jobs (Phase 4b-1): a layer's edit, histogram or effects image made by a second engine from the
//! job's input alone, equal to the same work done in place; an edit put back only onto the layer it
//! was taken from.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }

/// Unequal pixels with varied alpha: no two neighbours alike.
fn pattern(width: u32, height: u32) -> Raster {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height { for x in 0..width {
        let v = (x.wrapping_mul(73) ^ y.wrapping_mul(151)).wrapping_mul(2654435761);
        let a = (v >> 24) as u8 | 0x40;
        data.extend_from_slice(&[((v >> 8) as u8).min(a), ((v >> 16) as u8).min(a), (v as u8).min(a), a]);
    }}
    Raster::from_premultiplied(width, height, data)
}

/// A 160 x 110 canvas; a 90 x 60 layer of `pattern` at (23, 17) whose mask hides it but for an
/// ellipse over part of it; that ellipse, feathered, selected.
fn document() -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(160, 110, false).unwrap();
    let png = encode_png(&pattern(90, 60), DEFAULT_RESOLUTION).unwrap();
    e.import_image(Some(id), &png, "Pattern", Some(p(68.0, 47.0))).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::AddMask { id: layer, revealing: false });
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![p(30.0, 20.0), p(95.0, 20.0), p(95.0, 70.0), p(30.0, 70.0)], mode: SelectionMode::Replace, antialiased: true });
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    run(&mut e, id, Command::FeatherSelection { amount: 4 });
    (e, id, layer)
}

/// The job's input as it crosses to the worker: JSON, and the buffers' bytes.
fn crossed(e: &Engine, id: Uuid, layer: Uuid) -> (JobInput, Option<Raster>, Option<GrayRaster>) {
    let (input, pixels, mask) = e.job_input(id, layer).unwrap();
    let input: JobInput = serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
    let pixels = pixels.map(|r| Raster::from_premultiplied(r.width, r.height, r.bytes().to_vec()));
    let mask = mask.map(|m| GrayRaster::from_bytes(m.width, m.height, m.bytes().to_vec()));
    (input, pixels, mask)
}

fn levels() -> LayerAdjustment {
    let mut a = LayerAdjustment::new(AdjustmentKind::Levels);
    a.levels.ranges[0] = LevelRange { black: 20.0, gamma: 1.4, white: 230.0, output_black: 10.0, output_white: 240.0 };
    a
}

#[test]
fn an_edit_made_by_a_job_and_put_back_equals_the_edit_made_in_place() {
    let commands = |layer: Uuid| vec![
        Command::ApplyAdjustment { id: layer, adjustment: levels() },
        // A blur spreads past the layer: its grid grows, and the covering mask with it.
        Command::ApplyFilter { id: layer, params: FilterParams::GaussianBlur { radius: 6.0 } },
        Command::InvertPixels { id: layer, mask: true },
    ];
    for i in 0..3 {
        // The same document twice: one edited in place, one through a job.
        let (mut here, id, layer) = document();
        let (mut there, tid, tlayer) = document();
        let depth = there.state(tid).unwrap().undo_depth;
        run(&mut here, id, commands(layer)[i].clone());
        let (input, pixels, mask) = crossed(&there, tid, tlayer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, commands(tlayer)[i].clone()).unwrap();
        there.install_job(tid, tlayer, input.stamp, output, new_pixels, new_mask).unwrap();
        let (a, b) = (&here.document(id).unwrap().layers[0], &there.document(tid).unwrap().layers[0]);
        assert_eq!(a.pixels.as_ref().unwrap().bytes(), b.pixels.as_ref().unwrap().bytes(), "command {i}: pixels");
        assert_eq!(a.transform, b.transform, "command {i}: transform");
        assert_eq!(a.mask.as_ref().unwrap().pixels.bytes(), b.mask.as_ref().unwrap().pixels.bytes(), "command {i}: mask");
        assert_eq!(there.state(tid).unwrap().undo_depth, depth + 1, "command {i}: one step");
    }
}

#[test]
fn a_job_inside_a_selection_brings_its_changed_rectangle_back() {
    let (mut e, id, layer) = document();
    let before = e.state(id).unwrap().layers[0].pixels_revision;
    let (input, pixels, mask) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::InvertPixels { id: layer, mask: false }).unwrap();
    assert!(new_mask.is_none(), "the mask was not touched, so it does not travel back");
    assert_eq!(output.regions.len(), 1);
    e.install_job(id, layer, input.stamp, output.clone(), new_pixels, new_mask).unwrap();
    assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(output.regions[0].1));
}

#[test]
fn a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile() {
    for change in ["pixels", "transform", "mask"] {
        let (mut e, id, layer) = document();
        let (input, pixels, mask) = crossed(&e, id, layer);
        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
        match change {
            "pixels" => run(&mut e, id, Command::InvertPixels { id: layer, mask: false }),
            "transform" => run(&mut e, id, Command::NudgeLayers { ids: vec![layer], dx: 1.0, dy: 0.0 }),
            _ => run(&mut e, id, Command::FillMask { id: layer, white: true }),
        }
        let (doc, depth) = (e.document(id).unwrap().clone(), e.state(id).unwrap().undo_depth);
        let refused = e.install_job(id, layer, input.stamp, output, new_pixels, new_mask);
        assert_eq!(refused, Err(CommandError::Refused(LAYER_CHANGED.into())), "{change}");
        assert!(e.document(id).unwrap().same_content(&doc), "{change}: untouched");
        assert_eq!(e.state(id).unwrap().undo_depth, depth, "{change}: nothing recorded");
    }
    // A change that leaves the layer's pixels, mask and place alone does not stop it.
    let (mut e, id, layer) = document();
    let (input, pixels, mask) = crossed(&e, id, layer);
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, Command::ApplyAdjustment { id: layer, adjustment: levels() }).unwrap();
    run(&mut e, id, Command::RenameLayer { id: layer, name: "Renamed".into() });
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].name, "Renamed");
}

#[test]
fn a_histogram_job_equals_the_histogram_in_place() {
    let (e, id, layer) = document();
    let (input, pixels, mask) = crossed(&e, id, layer);
    assert_eq!(run_histogram_job(&input, pixels, mask).unwrap(), e.histogram(id, layer).unwrap());
}

#[test]
fn an_effects_job_at_full_size_equals_the_engines_own_image_and_a_reduced_one_scales_its_effects() {
    let (mut e, id, layer) = document();
    run(&mut e, id, Command::Deselect);
    let effects: LayerEffects = serde_json::from_value(serde_json::json!({
        "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 6 },
        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 } })).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers[0].extra.effects = Some(effects.clone());
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    let (input, pixels, mask) = crossed(&e, id, layer);
    let (image, raster) = run_effects_job(&input, pixels.clone().unwrap(), mask.clone(), 1.0, None).unwrap().unwrap();
    let own = e.draw_raster(id, layer, 0, None).unwrap().unwrap();
    assert_eq!(raster.bytes(), own.bytes(), "the job's image is the engine's image");
    assert_eq!((image.width, image.height, image.inset), (own.width, own.height, effects.margin()));
    // Halved twice: a quarter of the pixels and effects a quarter the size.
    let quarter = pixels.unwrap().halved().halved();
    let (small, _) = run_effects_job(&input, quarter.clone(), mask, 0.25, None).unwrap().unwrap();
    let inset = effects.scaled(0.25).margin();
    assert!(inset < effects.margin());
    assert_eq!((small.width, small.height, small.inset), (quarter.width + 2 * inset, quarter.height + 2 * inset, inset));
}
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test jobs`: does not compile (`compositor_engine::jobs`).

- [ ] **Step 3: Implement**

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -5,7 +5,28 @@ use uuid::Uuid;
 use wasm_bindgen::prelude::*;
 
 #[wasm_bindgen]
-pub struct WasmEngine { engine: Engine, pending_saves: HashMap<Uuid, Package>, drawn: Option<Raster> }
+pub struct WasmEngine {
+    engine: Engine, pending_saves: HashMap<Uuid, Package>, drawn: Option<Raster>,
+    /// A job's pixel and mask buffers, kept while the app copies them out: a job's input on the main
+    /// thread, a job's result in the worker (`job_buffer_ptr`, `release_job`).
+    job: (Option<Raster>, Option<GrayRaster>),
+}
+
+/// A buffer's bytes as a raster, when its size is known.
+fn raster_of(size: Option<(u32, u32)>, bytes: Option<Vec<u8>>) -> Result<Option<Raster>, JsError> {
+    match (size, bytes) {
+        (Some((w, h)), Some(b)) if b.len() == (w as usize) * (h as usize) * 4 => Ok(Some(Raster::from_premultiplied(w, h, b))),
+        (None, None) => Ok(None),
+        _ => Err(JsError::new("a job's pixels do not match its size")),
+    }
+}
+fn gray_of(size: Option<(u32, u32)>, bytes: Option<Vec<u8>>) -> Result<Option<GrayRaster>, JsError> {
+    match (size, bytes) {
+        (Some((w, h)), Some(b)) if b.len() == (w as usize) * (h as usize) => Ok(Some(GrayRaster::from_bytes(w, h, b))),
+        (None, None) => Ok(None),
+        _ => Err(JsError::new("a job's mask does not match its size")),
+    }
+}
 
 fn js_err<E: std::fmt::Display>(e: E) -> JsError { JsError::new(&e.to_string()) }
 fn parse_id(text: &str) -> Result<Uuid, JsError> { Uuid::parse_str(text).map_err(js_err) }
@@ -15,7 +36,68 @@ impl WasmEngine {
     #[wasm_bindgen(constructor)]
     pub fn new() -> WasmEngine {
         console_error_panic_hook::set_once();
-        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new(), drawn: None }
+        WasmEngine { engine: Engine::new(), pending_saves: HashMap::new(), drawn: None, job: (None, None) }
+    }
+
+    // Jobs (Phase 4b-1, engine `jobs.rs`). On the main thread: `prepare_job` or `prepare_display_job`,
+    // copy the buffers out, `release_job`; later `install_job`. In the worker: a `run_*_job`, copy the
+    // result buffers out, `release_job`.
+
+    /// `Engine::job_input` as JSON; the layer's pixels and mask are kept for `job_buffer_ptr`.
+    pub fn prepare_job(&mut self, doc: &str, layer: &str) -> Result<String, JsError> {
+        let (input, pixels, mask) = self.engine.job_input(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?;
+        self.job = (pixels, mask);
+        serde_json::to_string(&input).map_err(js_err)
+    }
+    /// `Engine::display_job_input` (an effects job's input at `level` halvings) as JSON.
+    pub fn prepare_display_job(&mut self, doc: &str, layer: &str, level: u32) -> Result<String, JsError> {
+        let (input, pixels, mask) = self.engine.display_job_input(parse_id(doc)?, parse_id(layer)?, level).map_err(js_err)?;
+        self.job = (Some(pixels), mask);
+        serde_json::to_string(&input).map_err(js_err)
+    }
+    /// The kept job buffer: the pixels, or the mask; null when there is none. A view on it is valid
+    /// until the next engine call.
+    pub fn job_buffer_ptr(&self, mask: bool) -> *const u8 {
+        if mask { self.job.1.as_ref().map_or(std::ptr::null(), |m| m.bytes().as_ptr()) } else { self.job.0.as_ref().map_or(std::ptr::null(), |r| r.bytes().as_ptr()) }
+    }
+    pub fn job_buffer_len(&self, mask: bool) -> usize {
+        if mask { self.job.1.as_ref().map_or(0, |m| m.bytes().len()) } else { self.job.0.as_ref().map_or(0, |r| r.bytes().len()) }
+    }
+    pub fn release_job(&mut self) { self.job = (None, None); }
+    /// `Engine::install_job`: an edit job's result put back, if the layer still matches `stamp`.
+    pub fn install_job(&mut self, doc: &str, layer: &str, stamp_json: &str, output_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>) -> Result<String, JsError> {
+        let stamp: LayerStamp = serde_json::from_str(stamp_json).map_err(js_err)?;
+        let output: JobOutput = serde_json::from_str(output_json).map_err(js_err)?;
+        let (pixels, mask) = (raster_of(output.pixels, pixels)?, gray_of(output.mask, mask)?);
+        let dirty = self.engine.install_job(parse_id(doc)?, parse_id(layer)?, stamp, output, pixels, mask).map_err(js_err)?;
+        serde_json::to_string(&dirty).map_err(js_err)
+    }
+    /// `run_edit_job` (in the worker): the output as JSON; the buffers it replaced are kept.
+    pub fn run_edit_job(&mut self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, command_json: &str) -> Result<String, JsError> {
+        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
+        let command: Command = serde_json::from_str(command_json).map_err(js_err)?;
+        let (pixels, mask) = (raster_of(input.pixels, pixels)?, gray_of(input.mask, mask)?);
+        let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, command).map_err(js_err)?;
+        self.job = (new_pixels, new_mask);
+        serde_json::to_string(&output).map_err(js_err)
+    }
+    /// `run_histogram_job` (in the worker): four arrays of 256 bins, as JSON.
+    pub fn run_histogram_job(&self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>) -> Result<String, JsError> {
+        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
+        let (pixels, mask) = (raster_of(input.pixels, pixels)?, gray_of(input.mask, mask)?);
+        serde_json::to_string(&run_histogram_job(&input, pixels, mask).map_err(js_err)?).map_err(js_err)
+    }
+    /// `run_effects_job` (in the worker): the image's size and inset as JSON (the image is kept), or
+    /// None when the layer draws no effects.
+    pub fn run_effects_job(&mut self, input_json: &str, pixels: Vec<u8>, mask: Option<Vec<u8>>, factor: f64, edit_json: Option<String>) -> Result<Option<String>, JsError> {
+        let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
+        let pixels = raster_of(input.pixels, Some(pixels))?.ok_or_else(|| JsError::new("an effects job needs pixels"))?;
+        let mask = gray_of(input.mask, mask)?;
+        let edit = Self::parse_edit(edit_json)?;
+        match run_effects_job(&input, pixels, mask, factor, edit.as_ref()).map_err(js_err)? {
+            Some((image, raster)) => { self.job = (Some(raster), None); Ok(Some(serde_json::to_string(&image).map_err(js_err)?)) }
+            None => { self.job = (None, None); Ok(None) }
+        }
     }
     pub fn version(&self) -> String { Engine::version().to_string() }
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -161,7 +161,9 @@ impl Engine {
         self.order.push(handle);
         handle
     }
-    fn session(&self, id: Uuid) -> Result<&Session, CommandError> { self.sessions.get(&id).ok_or(CommandError::NoDocument) }
+    /// A document of its own, with fresh history: a job's document (`jobs.rs`).
+    pub fn insert_document(&mut self, document: Document) -> Uuid { self.insert(document, None) }
+    pub(crate) fn session(&self, id: Uuid) -> Result<&Session, CommandError> { self.sessions.get(&id).ok_or(CommandError::NoDocument) }
     fn session_mut(&mut self, id: Uuid) -> Result<&mut Session, CommandError> { self.sessions.get_mut(&id).ok_or(CommandError::NoDocument) }
 
     pub fn document_ids(&self) -> Vec<Uuid> { self.order.clone() }
@@ -283,7 +285,7 @@ impl Engine {
     /// The document as the canvas should show it: the stored one, or a copy with the open
     /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
     /// the ops do not, because a preview is not committed.
-    fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
+    pub(crate) fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
         let s = self.session(id)?;
         let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
         let mut doc = s.document.clone();
@@ -322,11 +324,11 @@ impl Engine {
         s.preview = request.as_ref().and_then(|r| preview::compute_preview_with(&s.document, clips, r, revision));
         Ok(Dirty::pixels(layers))
     }
-    fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }
+    pub(crate) fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }
 
     /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
     /// `f` reads the selection's clip through the engine's cache (`SelectionClips`).
-    fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
+    pub(crate) fn edit<F>(&mut self, id: Uuid, f: F) -> Result<Dirty, CommandError>
     where F: FnOnce(&mut Document, &SelectionClips) -> Result<Dirty, CommandError> {
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
         let mut next = s.document.clone();
```

Create `engine/src/jobs.rs`:

```rust
//! Jobs (Phase 4b-1): one layer taken out of a document and worked on by a second engine, the app's
//! job worker, off the UI thread. A job reads nothing but its input: the canvas's size, the selection,
//! and the layer itself (its record as the manifest writes it, with its pixel and mask buffers sent
//! beside the JSON). Three kinds: an edit (a destructive `Command` on that layer), whose result is put
//! back as one undo step only if the layer is still exactly what the job took (`LayerStamp`), as the
//! Mac installs a detached result (SelectionEdits.swift:108-115); a histogram; and an effects image.
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Said when a job's result finds its layer changed since the job took it.
pub const LAYER_CHANGED: &str = "The layer changed while the edit was being made, so it was not applied.";

/// What a layer was when a job took it: the edit goes back only onto exactly this.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerStamp { pub pixels_revision: u64, pub mask_revision: u64, pub transform: LayerTransform }

impl LayerStamp {
    pub fn of(layer: &Layer) -> LayerStamp { LayerStamp { pixels_revision: layer.pixels_revision, mask_revision: layer.mask_revision, transform: layer.transform } }
}

/// The selection a job clips to, as plain data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobSelection { pub contours: Vec<Contour>, pub antialiased: bool, pub feather: f64 }

/// A job's input, beside its buffers: the canvas, the selection, the layer's record and the sizes of
/// its pixel and mask buffers (None where it has none), and the stamp an edit must find again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInput {
    pub width: u32,
    pub height: u32,
    pub selection: Option<JobSelection>,
    pub layer: LayerRecord,
    pub pixels: Option<(u32, u32)>,
    pub mask: Option<(u32, u32)>,
    pub stamp: LayerStamp,
}

/// What an edit left of its layer: the transform, the mask's placement, which buffers it replaced
/// (their sizes; the buffers travel beside) and the rectangles it changed them within.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutput {
    pub transform: LayerTransform,
    pub mask_placement: Option<LayerTransform>,
    pub pixels: Option<(u32, u32)>,
    pub mask: Option<(u32, u32)>,
    pub regions: Vec<(Plane, PixelRect)>,
}

/// An effects image made by a job: its size and the transparent pixels added on every side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectsImage { pub width: u32, pub height: u32, pub inset: u32 }

impl JobInput {
    /// The document a job works on: the canvas, the selection and the one layer, active. The layer
    /// keeps its id, so a command names it as the app did.
    fn document(&self, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Result<Document, CommandError> {
        if pixels.as_ref().map(|p| (p.width, p.height)) != self.pixels || mask.as_ref().map(|m| (m.width, m.height)) != self.mask {
            return Err(CommandError::Argument("a job's buffers do not match its input".into()));
        }
        let mut doc = Document::new(self.width, self.height);
        let mut layer = Layer::from_record(&self.layer, pixels, mask);
        // A layer inside a folder keeps its id and pixels; the folder itself is not sent.
        layer.parent_id = None;
        layer.mask_source_id = None;
        doc.active_layer_id = Some(layer.id);
        doc.layers = vec![layer];
        doc.selection = self.selection.as_ref().map(|s| Selection::new(s.contours.clone(), s.antialiased, s.feather));
        Ok(doc)
    }
}

impl Engine {
    /// A job's input for `layer`, and its own pixel and mask buffers (shared: the wasm bridge copies
    /// them out once). The stored document, never an open preview: a job edits what is committed.
    pub fn job_input(&self, id: Uuid, layer: Uuid) -> Result<(JobInput, Option<Raster>, Option<GrayRaster>), CommandError> {
        let doc = &self.session(id)?.document;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let input = JobInput {
            width: doc.width, height: doc.height,
            selection: doc.selection.as_ref().map(|s| JobSelection { contours: s.contours.as_ref().clone(), antialiased: s.antialiased, feather: s.feather }),
            layer: l.record(),
            pixels: l.pixels.as_ref().map(|p| (p.width, p.height)),
            mask: l.mask.as_ref().map(|m| (m.pixels.width, m.pixels.height)),
            stamp: LayerStamp::of(l),
        };
        Ok((input, l.pixels.clone(), l.mask.as_ref().map(|m| m.pixels.clone())))
    }

    /// An effects job's input for `layer` as the canvas shows it (through any open preview), its
    /// pixels after `level` halvings (the Mac previews effects from a reduced copy); the header names
    /// that reduced size.
    pub fn display_job_input(&self, id: Uuid, layer: Uuid, level: u32) -> Result<(JobInput, Raster, Option<GrayRaster>), CommandError> {
        let doc = self.render_document(id)?;
        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
        let pixels = self.layer_raster(id, layer, level)?.ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
        let input = JobInput {
            width: doc.width, height: doc.height, selection: None, layer: l.record(),
            pixels: Some((pixels.width, pixels.height)), mask: l.mask.as_ref().map(|m| (m.pixels.width, m.pixels.height)),
            stamp: LayerStamp::of(l),
        };
        Ok((input, pixels, l.mask.as_ref().map(|m| m.pixels.clone())))
    }

    /// Puts an edit job's result back on `layer` as one undo step: the buffers it replaced, its
    /// transform and mask placement, recorded as changed within its regions. Refused, with the
    /// document untouched, unless the layer is still exactly what the job took (`stamp`).
    pub fn install_job(&mut self, id: Uuid, layer: Uuid, stamp: LayerStamp, output: JobOutput, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Result<Dirty, CommandError> {
        if pixels.as_ref().map(|p| (p.width, p.height)) != output.pixels || mask.as_ref().map(|m| (m.width, m.height)) != output.mask {
            return Err(CommandError::Argument("a job's buffers do not match its output".into()));
        }
        self.clear_preview(id);
        self.edit(id, |doc, _| {
            let l = doc.layer_mut(layer).ok_or(CommandError::NoLayer)?;
            if LayerStamp::of(l) != stamp { return Err(CommandError::Refused(LAYER_CHANGED.into())); }
            if let Some(p) = pixels { l.set_pixels(Some(p)); }
            l.transform = output.transform;
            if let Some(m) = mask {
                let target = l.mask_mut().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
                target.pixels = m;
                target.placement = output.mask_placement;
            }
            let regions = output.regions.iter().map(|(plane, rect)| Region { layer, plane: *plane, rect: *rect }).collect();
            Ok(Dirty::pixels(vec![layer]).within(regions))
        })
    }
}

/// Runs an edit job: `command` on the job's document, and what it left of the layer. Only the
/// buffers the command replaced come back.
pub fn run_edit_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>, command: Command) -> Result<(JobOutput, Option<Raster>, Option<GrayRaster>), CommandError> {
    let mut engine = Engine::new();
    let id = engine.insert_document(input.document(pixels.clone(), mask.clone())?);
    let dirty = engine.execute(id, command)?;
    let l = engine.document(id).and_then(|d| d.layer(input.layer.id)).ok_or(CommandError::NoLayer)?;
    let new_pixels = l.pixels.clone().filter(|p| !pixels.as_ref().is_some_and(|o| o.same_pixels(p)));
    let new_mask = l.mask.as_ref().map(|m| m.pixels.clone()).filter(|m| !mask.as_ref().is_some_and(|o| o.same_pixels(m)));
    let output = JobOutput {
        transform: l.transform,
        mask_placement: l.mask.as_ref().and_then(|m| m.placement),
        pixels: new_pixels.as_ref().map(|p| (p.width, p.height)),
        mask: new_mask.as_ref().map(|m| (m.width, m.height)),
        regions: dirty.regions.iter().filter(|r| r.layer == input.layer.id).map(|r| (r.plane, r.rect)).collect(),
    };
    Ok((output, new_pixels, new_mask))
}

/// Runs a histogram job: the layer's histogram weighted by the selection (`Engine::histogram`).
pub fn run_histogram_job(input: &JobInput, pixels: Option<Raster>, mask: Option<GrayRaster>) -> Result<Vec<Vec<f64>>, CommandError> {
    let mut engine = Engine::new();
    let id = engine.insert_document(input.document(pixels, mask)?);
    engine.histogram(id, input.layer.id)
}

/// Runs an effects job: the layer drawn with its effects (`effects_image`), from `pixels` that may be
/// the layer's own reduced `factor` times (its halvings), the effects scaled with them, as the Mac's
/// canvas preview does (EffectsPreviewCache.swift:114-133). None when the layer draws no effects.
pub fn run_effects_job(input: &JobInput, pixels: Raster, mask: Option<GrayRaster>, factor: f64, edit: Option<&PreviewEdit>) -> Result<Option<(EffectsImage, Raster)>, CommandError> {
    let mut reduced = input.clone();
    reduced.pixels = Some((pixels.width, pixels.height));
    let doc = reduced.document(Some(pixels), mask)?;
    let mut layer = doc.layers[0].clone();
    if factor != 1.0 { layer.extra.effects = layer.extra.effects.map(|e| e.scaled(factor)); }
    let Some(draw) = effects_draw(&layer, edit) else { return Ok(None) };
    let image = effects_image(&layer, layer.pixels.as_ref().unwrap(), &draw);
    Ok(Some((EffectsImage { width: image.width, height: image.height, inset: draw.inset }, image)))
}
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -18,6 +18,7 @@ pub mod preview;
 pub mod effects;
 pub mod selection;
 pub mod lineage;
+pub mod jobs;
 
 pub use adjust::settings::*;
 pub use adjust::levels::*;
@@ -47,6 +48,7 @@ pub use effects::*;
 pub use effects::settings::*;
 pub use effects::render::*;
 pub use lineage::*;
+pub use jobs::*;
 pub use ops::masks::blur_gray;
 pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
 pub use selection::coverage::{rasterize, selection_coverage, selection_coverage_with, SelectionClip, SelectionClips};
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test jobs` (5 tests); the whole suite 479 passed, 6 ignored (+5). `pnpm wasm:dev` builds.

- [ ] **Step 5: Prove it bites**

(1) In `install_job`, skip the stamp comparison: `a_job_is_not_put_back_onto_a_layer_that_changed_meanwhile` fails (measured). Restore. (2) In `run_edit_job`, return the mask whether it changed or not: `a_job_inside_a_selection_brings_its_changed_rectangle_back` fails (only the pixels may come back). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/jobs.rs engine/tests/jobs.rs
git commit -m "feat(engine): jobs edit, read and draw one layer in a second engine and put the result back if it is unchanged" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine-wasm/src/lib.rs engine/src/engine.rs engine/src/jobs.rs engine/src/lib.rs engine/tests/jobs.rs
```


---

### Task 6: The job worker, and large layers' edits and histograms through it

`JobClient` runs Task 5's jobs in a module worker (`job-worker.ts`: the compiled wasm module is posted to it once, and each job's buffers are transferred, never copied twice). One job runs at a time, in order; each has a channel, and a newer job on the same channel drops a waiting one and throws away a running one's result; a worker whose memory passed `WORKER_MEMORY_LIMIT` (1 GiB) after a job, or that died, is replaced. In the store, a layer over `JOB_PIXELS` (4 MP) commits destructive adjustments and filters through `runEditJob` (the panel closes, the canvas keeps its preview until the result is in, `working` holds every other command, undo, redo, the file actions and the menu), and reads its Levels / Curves histogram through the worker while the panel opens at once without it ("Reading the histogram..."; Auto is disabled until it arrives). Ruling OQ5.

**Files:**
- Create: `app/src/engine/jobs.ts`, `app/src/engine/job-worker.ts`
- Modify: `app/src/engine/client.ts` (the module kept, `jobInput`, `displayJobInput`, `installJob`), `app/src/state/store.ts` (`jobs`, `jobPixels`, `working`, `usesJob`, `runEditJob`, the commit and histogram paths), `app/src/actions/files.ts`, `app/src/panels/MenuBar.tsx`, `app/src/panels/LevelsPanel.tsx`, `app/src/App.tsx` (the client, "Working..."), `vite.config.ts` (`worker.format: "es"`)
- Create tests: `app/tests/unit/jobs.test.ts`, `app/tests/unit/store-jobs.test.ts`, `app/tests/e2e/jobs.spec.ts`; modify `app/tests/unit/engine-client.test.ts`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `JobRequest`, `JobResult`, `JobClient.run(channel, request)`, `cancel(channel)`, `dispose()`, `WORKER_MEMORY_LIMIT`; `EngineClient.module`, `jobInput(doc, layer)`, `displayJobInput(doc, layer, level)`, `installJob(...)`; the store's `JOB_PIXELS`, `BUSY_MESSAGE`, `usesJob(layer)`, `runEditJob(command, layer)`, `working`.

- [ ] **Step 1: Write the tests**

`jobs.test.ts` drives `JobClient` with a fake worker: the module first, then one job at a time in order with its buffers transferred; supersession and cancelling by channel; a failed job rejects without stopping the next; a worker past the memory limit, or one that died, is replaced. `store-jobs.test.ts` drives the store with a stub engine and a stub client: OK on a large layer goes to the worker, keeps the preview and puts the result back once; a layer at the threshold stays on the UI thread; commands and undo wait while a result is to come; a refused result takes the preview away and says why; the histogram lands only in the panel it was read for. The engine-client test checks that `jobInput` copies the kept buffers into buffers of its own and releases the engine's. The e2e sets `jobPixels` to 0 so every layer goes through the real worker: Levels opens at once, its histogram arrives, and OK applies exactly what the UI thread applies; a blur grows the layer through the worker exactly as in place.

Create `app/tests/e2e/jobs.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { noisePngBase64 } from "./helpers";

// Phase 4b-1: the job worker, a second engine in a Web Worker (engine/tests/jobs.rs pins the engine
// side). Every layer here counts as large (`jobPixels` 0), so the panels' commits and the Levels
// histogram go through the worker, and each result is compared with the same edit made in place.

/** Two documents holding the same noise layer, the first open; every layer counts as large. */
async function setup(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const b64 = await page.evaluate(noisePngBase64);
  return page.evaluate(async (data) => {
    const api = (window as any).__compositor;
    const png = Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const make = () => { const doc = api.engine.newDocument(80, 70, false); api.engine.importImage(doc, png, "noise", { x: 41, y: 33 }); return doc; };
    const [here, there] = [make(), make()];
    api.store.setState({ jobPixels: 0 });
    api.store.getState().openDocument(there);
    return { there, here, layer: api.engine.state(there).layers[0].id, other: api.engine.state(here).layers[0].id };
  }, b64);
}
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const idle = (page: Page) => page.waitForFunction(() => !(window as any).__compositor.store.getState().working);
/** The largest byte difference between the two documents' composites. */
const worst = (page: Page, a: string, b: string) => page.evaluate(([a, b]) => {
  const api = (window as any).__compositor;
  const region = { x: 0, y: 0, width: 80, height: 70 };
  const x = api.engine.composite(a, region, 80, 70) as Uint8Array, y = api.engine.composite(b, region, 80, 70) as Uint8Array;
  let d = 0; for (let i = 0; i < x.length; i++) d = Math.max(d, Math.abs(x[i] - y[i]));
  return d;
}, [a, b]);

test("Levels on a large layer: the panel opens at once, its histogram comes from the worker, and OK applies through it as one step", async ({ page }) => {
  const ids = await setup(page);
  const depth = (await state(page)).undoDepth;
  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "Levels" }));
  // The histogram the worker read is the one the engine reads in place.
  await page.waitForFunction(() => (window as any).__compositor.store.getState().adjustEdit?.histogram !== null);
  await expect(page.getByTestId("histogram-pending")).toHaveCount(0);
  const [fromWorker, inPlace] = await page.evaluate((ids) => {
    const api = (window as any).__compositor;
    return [api.store.getState().adjustEdit.histogram, api.engine.histogram(ids.there, ids.layer)];
  }, ids);
  expect(fromWorker).toEqual(inPlace);
  await page.getByLabel("Output white").fill("190");
  await page.getByLabel("Gamma").fill("1.3");
  const adjustment = await page.evaluate(() => (window as any).__compositor.store.getState().adjustEdit.adjustment);
  await page.getByRole("button", { name: "OK" }).click();
  await idle(page);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.evaluate(([ids, adjustment]) => (window as any).__compositor.engine.execute(ids.here, { type: "ApplyAdjustment", id: ids.other, adjustment }), [ids, adjustment] as const);
  expect(await worst(page, ids.there, ids.here)).toBe(0);
});

test("a blur on a large layer grows it through the worker exactly as it does in place", async ({ page }) => {
  const ids = await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().beginAdjust({ kind: "GaussianBlur" }));
  const params = { filter: "GaussianBlur", radius: 7 };
  await page.evaluate((params) => (window as any).__compositor.store.getState().updateAdjust({ params }), params);
  await page.getByRole("button", { name: "OK" }).click();
  await idle(page);
  await page.evaluate(([ids, params]) => (window as any).__compositor.engine.execute(ids.here, { type: "ApplyFilter", id: ids.other, params }), [ids, params] as const);
  const [a, b] = await page.evaluate((ids) => {
    const api = (window as any).__compositor;
    return [api.engine.state(ids.there).layers[0], api.engine.state(ids.here).layers[0]];
  }, ids);
  expect([a.pixelsWidth, a.pixelsHeight, a.transform]).toEqual([b.pixelsWidth, b.pixelsHeight, b.transform]);
  expect(a.pixelsWidth, "the blur grew the layer").toBeGreaterThan(64);
  expect(await worst(page, ids.there, ids.here)).toBe(0);
});
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -92,6 +92,59 @@ test("partial uploads: the frame after an edit inside a selection, at fit and at
   for (const label of ["24 MP", "100 MP"]) for (const zoom of ["fit", "1:1"]) expect(out[`${label}: frame after it at ${zoom}`]).toBeLessThan(33);
 });
 
+test("jobs: the Levels histogram and commit through the worker at 24 and 100 MP, and the page's frames meanwhile", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      // The main thread's two copies, timed where the store makes them.
+      const timed = (name: string) => { const f = api.engine[name].bind(api.engine); api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); } }; };
+      timed("jobInput"); timed("installJob");
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      api.store.getState().openDocument(doc);
+      await settle(); frame();
+      // The longest gap between animation frames while `until` is false.
+      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
+        let last = performance.now(), gap = 0;
+        const tick = (t: number) => { gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+        requestAnimationFrame(tick);
+      });
+      let t0 = performance.now();
+      api.store.getState().beginAdjust({ kind: "Levels" });
+      result["Levels opens (UI thread) ms"] = Math.round(performance.now() - t0);
+      result["histogram: longest frame gap while the worker reads it"] = await longestGap(() => api.store.getState().adjustEdit?.histogram !== null);
+      result["histogram arrives after ms"] = Math.round(performance.now() - t0);
+      const adjustment = JSON.parse(JSON.stringify(api.store.getState().adjustEdit.adjustment));
+      adjustment.levels.ranges[0].outputWhite = 200;
+      api.store.getState().updateAdjust({ adjustment });
+      await new Promise((r) => setTimeout(r, 400));
+      t0 = performance.now();
+      api.store.getState().commitAdjust();
+      result["OK (UI thread) ms"] = Math.round(performance.now() - t0);
+      result["commit: longest frame gap while the worker edits"] = await longestGap(() => !api.store.getState().working);
+      result["commit done after ms"] = Math.round(performance.now() - t0);
+      result["frame after the result is put back"] = Math.round(frame());
+      result["undo depth"] = api.engine.state(doc).undoDepth;
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`jobs (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) {
+    // The worker's own time never holds the page up past 100 ms; the copies are budgeted per size.
+    expect(out[`${label}: histogram: longest frame gap while the worker reads it`]).toBeLessThan(label === "24 MP" ? 150 : 450);
+    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
+  }
+});
+
 test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push at the cap is cheap", async ({ page }) => {
   test.setTimeout(900_000);
   await ready(page);
```

```diff
--- a/app/tests/unit/engine-client.test.ts
+++ b/app/tests/unit/engine-client.test.ts
@@ -87,6 +87,26 @@ describe("pixel views survive a wasm memory growth", () => {
     expect(c.selectionOutline("D", 0.25)).toBe(flat);
   });
 
+  it("jobInput copies the kept buffers out after each pointer call, into buffers of their own, then releases them", () => {
+    const memory = new WebAssembly.Memory({ initial: 1, maximum: 8 });
+    new Uint8Array(memory.buffer).set([1, 2, 3, 4, 9, 8], 0);
+    const calls: string[] = [];
+    const wasm = {
+      prepare_job: (doc: string, layer: string) => { calls.push(`prepare ${doc} ${layer}`); return '{"stamp":{}}'; },
+      job_buffer_len: (mask: boolean) => (mask ? 2 : 4),
+      // Marshalling can grow memory, detaching any buffer read before the pointer call.
+      job_buffer_ptr: (mask: boolean) => { calls.push(`ptr ${mask}`); memory.grow(1); return mask ? 4 : 0; },
+      release_job: () => { calls.push("release"); },
+    };
+    const client = Object.create(EngineClient.prototype) as Record<string, unknown>;
+    client.wasm = wasm; client.memory = memory;
+    const copy = (client as unknown as EngineClient).jobInput("D", "A");
+    expect(Array.from(new Uint8Array(copy.pixels!))).toEqual([1, 2, 3, 4]);
+    expect(Array.from(new Uint8Array(copy.mask!))).toEqual([9, 8]);
+    expect(copy.pixels!.byteLength, "a buffer of its own, transferable").toBe(4);
+    expect(calls).toEqual(["prepare D A", "ptr false", "ptr true", "release"]);
+  });
+
   it("both return null rather than a zero-length view when there is nothing to read", () => {
     const memory = new WebAssembly.Memory({ initial: 1 });
     const wasm = { layer_pixels_len: () => 0, mask_pixels_len: () => 0, layer_pixels_ptr: () => { throw new Error("must not be called"); }, mask_pixels_ptr: () => { throw new Error("must not be called"); } };
```

Create `app/tests/unit/jobs.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { JobClient, WORKER_MEMORY_LIMIT, type FromWorker, type JobRequest, type ToWorker } from "../../src/engine/jobs";

/** A worker that records what it is sent and answers when told to. */
class FakeWorker {
  sent: { message: ToWorker; transfer: unknown[] }[] = [];
  terminated = false;
  onmessage: ((e: MessageEvent<FromWorker>) => void) | null = null;
  onerror: ((e: ErrorEvent) => void) | null = null;
  postMessage(message: ToWorker, transfer: unknown[] = []) { this.sent.push({ message, transfer }); }
  terminate() { this.terminated = true; }
  reply(message: FromWorker) { this.onmessage!({ data: message } as MessageEvent<FromWorker>); }
  /** The ids of the jobs it was given, in order. */
  jobs(): number[] { return this.sent.flatMap((s) => (s.message.type === "job" ? [s.message.id] : [])); }
}
const settle = () => new Promise((r) => setTimeout(r, 0));
const module = {} as WebAssembly.Module;
const histogram = (tag: string): JobRequest => ({ kind: "histogram", input: tag, pixels: new ArrayBuffer(8), mask: null });
const done = (id: number, header: string, memory = 1): FromWorker => ({ type: "done", id, result: { header, pixels: null, mask: null }, memory });

function client() {
  const workers: FakeWorker[] = [];
  const jobs = new JobClient(module, () => { const w = new FakeWorker(); workers.push(w); return w as unknown as Worker; });
  return { jobs, workers };
}

describe("JobClient", () => {
  it("starts the worker with the compiled module, then runs one job at a time in order, transferring its buffers", async () => {
    const { jobs, workers } = client();
    const first = jobs.run("a", histogram("one"));
    const second = jobs.run("b", histogram("two"));
    await settle();
    expect(workers.length).toBe(1);
    expect(workers[0].sent[0].message).toEqual({ type: "init", module });
    workers[0].reply({ type: "ready" });
    await settle();
    expect(workers[0].jobs()).toEqual([1]);
    const sentJob = workers[0].sent[1];
    expect(sentJob.transfer).toEqual([(sentJob.message as { request: JobRequest }).request.pixels]);
    workers[0].reply(done(1, "first"));
    expect((await first)?.header).toBe("first");
    await settle();
    expect(workers[0].jobs()).toEqual([1, 2]);
    workers[0].reply(done(2, "second"));
    expect((await second)?.header).toBe("second");
  });

  it("drops a waiting job its channel superseded, and throws away a running one's result", async () => {
    const { jobs, workers } = client();
    const running = jobs.run("fx", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    const waiting = jobs.run("fx", histogram("2"));
    const newest = jobs.run("fx", histogram("3"));
    expect(await waiting).toBeNull();
    workers[0].reply(done(1, "stale"));
    expect(await running, "superseded while it ran").toBeNull();
    await settle();
    expect(workers[0].jobs(), "the waiting one never ran").toEqual([1, 3]);
    workers[0].reply(done(3, "fresh"));
    expect((await newest)?.header).toBe("fresh");
  });

  it("cancels a channel, and a failed job rejects without stopping the next", async () => {
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    const two = jobs.run("y", histogram("2"));
    jobs.cancel("y");
    expect(await two).toBeNull();
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply({ type: "failed", id: 1, error: "boom", memory: 1 });
    await expect(one).rejects.toThrow("boom");
    const three = jobs.run("x", histogram("3"));
    await settle();
    workers[0].reply(done(3, "ok"));
    expect((await three)?.header).toBe("ok");
  });

  it("replaces a worker whose memory grew past the limit, and one that died", async () => {
    const { jobs, workers } = client();
    const one = jobs.run("x", histogram("1"));
    await settle(); workers[0].reply({ type: "ready" }); await settle();
    workers[0].reply(done(1, "big", WORKER_MEMORY_LIMIT + 1));
    expect((await one)?.header, "its result still arrives").toBe("big");
    expect(workers[0].terminated).toBe(true);
    const two = jobs.run("x", histogram("2"));
    await settle();
    expect(jobs.spawned).toBe(2);
    workers[1].reply({ type: "ready" }); await settle();
    workers[1].onerror!({ message: "out of memory" } as ErrorEvent);
    await expect(two).rejects.toThrow("out of memory");
    const three = jobs.run("x", histogram("3"));
    await settle();
    expect(jobs.spawned).toBe(3);
    workers[2].reply({ type: "ready" }); await settle();
    workers[2].reply(done(3, "again"));
    expect((await three)?.header).toBe("again");
  });
});
```

Create `app/tests/unit/store-jobs.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { BUSY_MESSAGE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { defaultAdjustment } from "../../src/state/adjust-edit";
import type { Command, DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";

function layer(id: string, width: number, height: number): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [width, height], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: width, pixelsHeight: height, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
    maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function document(l: LayerState): DocumentState {
  return { id: "D", documentId: "D", width: l.pixelsWidth, height: l.pixelsHeight, resolution: 72, activeLayerId: l.id, canUndo: false,
    canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [l] };
}
const bins = () => [0, 1, 2, 3].map((c) => new Array(256).fill(c));

/** A store over one layer of `width` x `height` with a stub engine and a stub job worker whose jobs
 * finish when `finish` is called. */
function install(width: number, height: number, onInstall?: () => void) {
  const log: string[] = [];
  const previews: (PreviewRequest | null)[] = [];
  const requests: JobRequest[] = [];
  let finish: (r: JobResult | null) => void = () => {};
  const engine = {
    state: () => document(layer("A", width, height)),
    execute: (_id: string, cmd: Command) => { log.push(`execute ${cmd.type}`); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => { log.push("histogram here"); return bins(); },
    jobInput: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null }; },
    installJob: (_doc: string, layerId: string, input: string, output: string) => { log.push(`install ${layerId} ${output}`); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
  } as unknown as EngineClient;
  const jobs = { run: (_channel: string, request: JobRequest) => { requests.push(request); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: document(layer("A", width, height)) }, order: ["D"], selectedLayerIds: ["A"],
    maskSelected: false, transformEdit: null, adjustEdit: null, error: null, tool: "move", cropRect: null, sheet: null, working: false });
  return { log, previews, requests, finish: (r: JobResult | null) => finish(r) };
}
/** Opens Levels on layer A and moves a slider, so OK has something to apply. */
function levelsChanged() {
  useEditor.getState().beginAdjust({ kind: "Levels" });
  const next = defaultAdjustment("Levels");
  next.levels.ranges[0] = { ...next.levels.ranges[0], outputWhite: 200 };
  useEditor.getState().updateAdjust({ adjustment: next });
}
const flush = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => useEditor.setState({ jobs: null, working: false, error: null }));

describe("destructive commits on large layers go to the job worker", () => {
  it("OK hands a large layer's edit to the worker, keeps the preview until the result is back, and puts it back once", async () => {
    // Just over the threshold: 2001 x 2000.
    const { log, previews, requests, finish } = install(2001, 2000);
    levelsChanged();
    const shown = previews.length;
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().adjustEdit, "the panel closes at once").toBeNull();
    expect(useEditor.getState().working).toBe(true);
    expect(previews.length, "the preview stays on the canvas meanwhile").toBe(shown);
    expect(log.filter((l) => l.startsWith("execute"))).toEqual([]);
    const command = JSON.parse((requests.at(-1) as { command: string }).command) as Command;
    expect(command.type).toBe("ApplyAdjustment");
    finish({ header: "OUT", pixels: new ArrayBuffer(4), mask: null });
    await flush();
    expect(log.at(-1)).toBe("install A OUT");
    expect(useEditor.getState().working).toBe(false);
    expect(previews.length, "the engine cleared its own preview as it put the result back").toBe(shown);
  });

  it("a layer at the threshold is edited on the UI thread as before", () => {
    const { log, requests } = install(2000, 2000);
    levelsChanged();
    useEditor.getState().commitAdjust();
    expect(requests.filter((r) => r.kind === "edit")).toEqual([]);
    expect(log.filter((l) => l.startsWith("execute"))).toEqual(["execute ApplyAdjustment"]);
  });

  it("while a job's result is to come, commands and undo wait", async () => {
    const { log, finish } = install(2001, 2000);
    levelsChanged();
    useEditor.getState().commitAdjust();
    expect(useEditor.getState().run({ type: "AddBlankLayer" })).toBe(false);
    expect(useEditor.getState().error).toBe(BUSY_MESSAGE);
    useEditor.getState().undo();
    expect(log).not.toContain("undo");
    expect(log).not.toContain("execute AddBlankLayer");
    finish(null);
    await flush();
    expect(useEditor.getState().run({ type: "AddBlankLayer" })).toBe(true);
  });

  it("a result the engine will not put back takes the preview away and says why", async () => {
    const { previews, finish } = install(2001, 2000, () => { throw new Error("The layer changed while the edit was being made, so it was not applied."); });
    levelsChanged();
    useEditor.getState().commitAdjust();
    finish({ header: "OUT", pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().error).toContain("The layer changed");
    expect(previews.at(-1)).toBeNull();
    expect(useEditor.getState().working).toBe(false);
  });

  it("a large layer's Levels histogram arrives from the worker, into the panel it was read for only", async () => {
    const { log, requests, finish } = install(2001, 2000);
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit!.histogram, "the panel opens without waiting").toBeNull();
    expect(log).not.toContain("histogram here");
    expect(requests.at(-1)?.kind).toBe("histogram");
    finish({ header: JSON.stringify(bins()), pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().adjustEdit!.histogram).toEqual(bins());
    // A histogram that lands after its panel closed goes nowhere.
    useEditor.getState().cancelAdjust();
    useEditor.getState().beginAdjust({ kind: "Curves" });
    const late = finish;
    useEditor.getState().cancelAdjust();
    late({ header: JSON.stringify(bins()), pixels: null, mask: null });
    await flush();
    expect(useEditor.getState().adjustEdit).toBeNull();
  });
});
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: the two new unit files do not compile (`jobs.ts`, `usesJob`); the engine-client test fails on `jobInput`.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -1,5 +1,6 @@
 import { useEffect, useRef, useState } from "react";
 import { EngineClient } from "./engine/client";
+import { JobClient } from "./engine/jobs";
 import { BUILD_MARKER } from "./build-info";
 import { installTestApi } from "./test-api";
 import { useEditor } from "./state/store";
@@ -32,6 +33,7 @@ export function App() {
   const [version, setVersion] = useState("");
   const sheet = useEditor((s) => s.sheet);
   const banner = useEditor((s) => s.error);
+  const working = useEditor((s) => s.working);
   // React 18 StrictMode double-invokes effects in dev (which is what `pnpm dev` - and
   // so every e2e run - uses). This effect has no cleanup, so without a guard that
   // double-invoke calls EngineClient.load() twice, constructing two WasmEngine
@@ -52,6 +54,8 @@ export function App() {
     loadedRef.current = true;
     Promise.all([EngineClient.load(), getBridge()]).then(([engine, bridge]) => {
       useEditor.getState().setEngine(engine); useEditor.getState().setBridge(bridge);
+      // The job worker: a second engine for work on one layer off the UI thread (engine jobs.rs).
+      useEditor.getState().setJobs(new JobClient(engine.module, () => new Worker(new URL("./engine/job-worker.ts", import.meta.url), { type: "module" })));
       installTestApi({ engine, bridge, store: useEditor });
       bridge.onFileDrop((paths, position) => {
         const projects = paths.filter((p) => p.toLowerCase().endsWith(".comp"));
@@ -99,7 +103,7 @@ export function App() {
       </div>
       <AdjustPanel />
       <UndrawnNotice />
-      <div className="status" data-testid="engine-ready">Compositor engine {version} ({BUILD_MARKER})</div>
+      <div className="status" data-testid="engine-ready">Compositor engine {version} ({BUILD_MARKER}){working && <span data-testid="working"> - Working...</span>}</div>
       {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
       {sheet?.kind === "new" && <NewCanvasSheet />}
       {sheet?.kind === "canvasSize" && <CanvasSizeSheet />}
```

```diff
--- a/app/src/actions/files.ts
+++ b/app/src/actions/files.ts
@@ -8,7 +8,8 @@ function ctx() {
 
 async function guarded(work: () => Promise<void>): Promise<void> {
   const s = useEditor.getState();
-  if (s.busy) return;
+  // Nothing is opened or saved while an edit job's result is still to come (the Mac's isProjectBusy).
+  if (s.busy || s.working) return;
   s.setBusy(true);
   try { await work(); }
   catch (e) { useEditor.getState().setError(e instanceof Error ? e.message : String(e)); }
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -4,12 +4,20 @@ import type { Command, Dirty, DocumentState, LayerAdjustment, LayerTransform, Le
 /** `[x, y, width, height]` from the engine as a rectangle; an empty array as null (take it whole). */
 function rectOf(v: ArrayLike<number>): PixelRect | null { return v.length === 4 ? { x: v[0], y: v[1], width: v[2], height: v[3] } : null; }
 
+/** A job's input copied out of wasm memory (engine `job_input`): the JSON header and the layer's
+ * pixel and mask bytes, each its own buffer, ready to transfer to the job worker. */
+export interface JobInputCopy { input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; }
+
 export class EngineClient {
-  private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory) {}
+  private constructor(private readonly wasm: WasmEngine, private readonly memory: WebAssembly.Memory, readonly module: WebAssembly.Module) {}
 
+  /** Compiles the engine's wasm once and instantiates it; the compiled module is kept for the job
+   * worker, which instantiates a second engine from it (`JobClient`). */
   static async load(): Promise<EngineClient> {
-    const exports = await init();
-    return new EngineClient(new WasmEngine(), exports.memory);
+    const response = await fetch(new URL("./pkg/compositor_engine_bg.wasm", import.meta.url));
+    const module = await WebAssembly.compile(await response.arrayBuffer());
+    const exports = await init({ module_or_path: module });
+    return new EngineClient(new WasmEngine(), exports.memory, module);
   }
 
   version(): string { return this.wasm.version(); }
@@ -95,6 +103,29 @@ export class EngineClient {
     const ptr = this.wasm.mask_pixels_ptr(doc, layer);
     return new Uint8Array(this.memory.buffer, ptr, len);
   }
+  /** Copies the kept job buffers out of wasm memory, then lets the engine drop them. */
+  private takeJob(input: string): JobInputCopy {
+    try {
+      const copy = (mask: boolean) => {
+        const len = this.wasm.job_buffer_len(mask);
+        if (len === 0) return null;
+        const ptr = this.wasm.job_buffer_ptr(mask);
+        return new Uint8Array(this.memory.buffer, ptr, len).slice().buffer;
+      };
+      return { input, pixels: copy(false), mask: copy(true) };
+    } finally { this.wasm.release_job(); }
+  }
+  /** A job's input for `layer` (engine `job_input`): the stored layer, never a preview. */
+  jobInput(doc: string, layer: string): JobInputCopy { return this.takeJob(this.wasm.prepare_job(doc, layer)); }
+  /** An effects job's input (engine `display_job_input`): the layer as the canvas shows it, its pixels
+   * after `level` halvings. */
+  displayJobInput(doc: string, layer: string, level: number): JobInputCopy { return this.takeJob(this.wasm.prepare_display_job(doc, layer, level)); }
+  /** Puts an edit job's result back (engine `install_job`), only onto the layer exactly as the job took
+   * it (its stamp, in `input`); a changed layer refuses with the engine's message. */
+  installJob(doc: string, layer: string, input: string, output: string, pixels: ArrayBuffer | null, mask: ArrayBuffer | null): Dirty {
+    const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
+    return JSON.parse(this.wasm.install_job(doc, layer, stamp, output, pixels ? new Uint8Array(pixels) : undefined, mask ? new Uint8Array(mask) : undefined)) as Dirty;
+  }
   /** What changed in the layer's pixels since revision `from` (engine `pixels_delta`): a rectangle of
    * its pixel grid, empty when nothing did, or null when the whole raster must be uploaded again. */
   pixelsDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.pixels_delta(doc, layer, from)); }
```

Create `app/src/engine/job-worker.ts`:

```ts
// The job worker (Phase 4b-1): a second instance of the engine's wasm module, compiled once by the
// main thread and posted here, running one job at a time on transferred buffers (engine `jobs.rs`).
import { initSync, WasmEngine } from "./pkg/compositor_engine.js";
import type { FromWorker, JobRequest, JobResult, ToWorker } from "./jobs";

let engine: WasmEngine | null = null;
let memory: WebAssembly.Memory | null = null;

/** A copy of one of the engine's kept job buffers, out of wasm memory, ready to transfer. */
function kept(mask: boolean): ArrayBuffer | null {
  const len = engine!.job_buffer_len(mask);
  if (len === 0) return null;
  const ptr = engine!.job_buffer_ptr(mask);
  return new Uint8Array(memory!.buffer, ptr, len).slice().buffer;
}
const bytes = (b: ArrayBuffer | null) => (b ? new Uint8Array(b) : undefined);

function run(request: JobRequest): JobResult {
  switch (request.kind) {
    case "edit": {
      const header = engine!.run_edit_job(request.input, bytes(request.pixels), bytes(request.mask), request.command);
      const result = { header, pixels: kept(false), mask: kept(true) };
      engine!.release_job();
      return result;
    }
    case "histogram":
      return { header: engine!.run_histogram_job(request.input, bytes(request.pixels), bytes(request.mask)), pixels: null, mask: null };
    case "effects": {
      const header = engine!.run_effects_job(request.input, new Uint8Array(request.pixels), bytes(request.mask), request.factor, request.edit ?? undefined) ?? null;
      const result = { header, pixels: header ? kept(false) : null, mask: null };
      engine!.release_job();
      return result;
    }
  }
}

self.onmessage = (e: MessageEvent<ToWorker>) => {
  const message = e.data;
  const post = (reply: FromWorker, transfer: ArrayBuffer[] = []) => (self as unknown as Worker).postMessage(reply, transfer);
  if (message.type === "init") {
    memory = initSync({ module: message.module }).memory;
    engine = new WasmEngine();
    post({ type: "ready" });
    return;
  }
  try {
    const result = run(message.request);
    post({ type: "done", id: message.id, result, memory: memory!.buffer.byteLength }, [result.pixels, result.mask].filter((b): b is ArrayBuffer => b !== null));
  } catch (err) {
    post({ type: "failed", id: message.id, error: String(err instanceof Error ? err.message : err), memory: memory!.buffer.byteLength });
  }
};
```

Create `app/src/engine/jobs.ts`:

```ts
// The job worker's client (Phase 4b-1): a second engine in a Web Worker runs work on one layer off
// the UI thread (engine `jobs.rs`). The main thread copies the layer's buffers out of wasm memory once
// and transfers them; results come back transferred. One job runs at a time.

/** What a job is asked to do. `input` is the engine's JobInput JSON; the buffers travel beside it. */
export type JobRequest =
  | { kind: "edit"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; command: string }
  | { kind: "histogram"; input: string; pixels: ArrayBuffer | null; mask: ArrayBuffer | null }
  | { kind: "effects"; input: string; pixels: ArrayBuffer; mask: ArrayBuffer | null; factor: number; edit: string | null };

/** What came back: the engine's JSON answer (an edit's JobOutput, a histogram's bins, an effects
 * image's size and inset; null when an effects job found nothing to draw) and any buffers. */
export interface JobResult { header: string | null; pixels: ArrayBuffer | null; mask: ArrayBuffer | null; }

/** Messages to the worker and back. */
export type ToWorker = { type: "init"; module: WebAssembly.Module } | { type: "job"; id: number; request: JobRequest };
export type FromWorker = { type: "ready" } | { type: "done"; id: number; result: JobResult; memory: number } | { type: "failed"; id: number; error: string; memory: number };

/** A worker that has grown its wasm memory past this is replaced after its job: wasm memory never
 * shrinks, and a second heap of gigabytes would crowd the app's own. */
export const WORKER_MEMORY_LIMIT = 1024 * 1024 * 1024;

/** The buffers a request hands over, for `postMessage`'s transfer list. */
export function transferables(request: JobRequest): ArrayBuffer[] {
  return [request.pixels, request.mask].filter((b): b is ArrayBuffer => b !== null && b.byteLength > 0);
}

interface Pending { id: number; channel: string; request: JobRequest; resolve: (r: JobResult | null) => void; reject: (e: Error) => void; }

/** Runs jobs on one worker, one at a time, in the order asked. A job on a channel is superseded by a
 * newer one on the same channel: if it has not started it is dropped, and if it is running its result
 * is thrown away when it lands; either way its promise resolves null (the Mac checks for cancellation
 * at a render's start and end, EffectsPreviewCache.swift:101-110). */
export class JobClient {
  private worker: Worker | null = null;
  private ready: Promise<void> | null = null;
  private queue: Pending[] = [];
  private running: Pending | null = null;
  /** The newest job id asked for on each channel. */
  private newest = new Map<string, number>();
  private nextId = 1;
  /** How many workers have been started (a test watches the respawn). */
  spawned = 0;

  constructor(private readonly module: WebAssembly.Module, private readonly spawn: () => Worker) {}

  /** Whether a job is running or waiting. */
  get busy(): boolean { return this.running !== null || this.queue.length > 0; }

  run(channel: string, request: JobRequest): Promise<JobResult | null> {
    const id = this.nextId++;
    this.newest.set(channel, id);
    return new Promise((resolve, reject) => {
      // A job waiting on this channel is superseded before it ever starts.
      this.queue = this.queue.filter((p) => { if (p.channel !== channel) return true; p.resolve(null); return false; });
      this.queue.push({ id, channel, request, resolve, reject });
      void this.pump();
    });
  }

  /** Drops the channel's waiting job and throws away its running one's result. */
  cancel(channel: string): void {
    this.newest.set(channel, -1);
    this.queue = this.queue.filter((p) => { if (p.channel !== channel) return true; p.resolve(null); return false; });
  }

  private start(): Promise<void> {
    const worker = this.spawn();
    this.spawned++;
    this.worker = worker;
    this.ready = new Promise((resolve) => {
      worker.onmessage = (e: MessageEvent<FromWorker>) => {
        if (e.data.type === "ready") { resolve(); return; }
        this.finish(e.data);
      };
    });
    // A worker that dies (out of memory, say) fails its job and is replaced for the next one.
    worker.onerror = (e: ErrorEvent) => {
      const job = this.running;
      this.running = null;
      worker.terminate();
      if (this.worker === worker) { this.worker = null; this.ready = null; }
      job?.reject(new Error(e.message || "The job worker stopped."));
      void this.pump();
    };
    const init: ToWorker = { type: "init", module: this.module };
    worker.postMessage(init);
    return this.ready;
  }

  private async pump(): Promise<void> {
    if (this.running || this.queue.length === 0) return;
    const job = this.queue.shift()!;
    // Superseded while it waited behind another job.
    if (this.newest.get(job.channel) !== job.id) { job.resolve(null); void this.pump(); return; }
    this.running = job;
    await (this.worker && this.ready ? this.ready : this.start());
    const message: ToWorker = { type: "job", id: job.id, request: job.request };
    this.worker!.postMessage(message, transferables(job.request));
  }

  private finish(message: Exclude<FromWorker, { type: "ready" }>): void {
    const job = this.running;
    if (!job || job.id !== message.id) return;
    this.running = null;
    if (message.memory > WORKER_MEMORY_LIMIT) { this.worker?.terminate(); this.worker = null; this.ready = null; }
    if (message.type === "failed") job.reject(new Error(message.error));
    else job.resolve(this.newest.get(job.channel) === job.id ? message.result : null);
    void this.pump();
  }

  dispose(): void {
    this.worker?.terminate();
    this.worker = null;
    for (const p of this.queue) p.resolve(null);
    this.queue = [];
    this.running?.resolve(null);
    this.running = null;
  }
}
```

```diff
--- a/app/src/panels/LevelsPanel.tsx
+++ b/app/src/panels/LevelsPanel.tsx
@@ -38,7 +38,8 @@ export function LevelsPanel() {
         <label key={key}>{label} <input aria-label={label} type="number" step={key === "gamma" ? 0.01 : 1} value={range[key]}
           onChange={(e) => { const v = Number(e.target.value); if (Number.isFinite(v)) update({ [key]: v } as Partial<LevelRange>); }} /></label>
       ))}
-      <label>Auto <select data-testid="levels-auto" value="" onChange={(e) => { if (e.target.value) s.autoLevels(e.target.value as LevelsAuto); }}>
+      {!edit.histogram && <span className="hint" data-testid="histogram-pending">Reading the histogram...</span>}
+      <label>Auto <select data-testid="levels-auto" value="" disabled={!edit.histogram} onChange={(e) => { if (e.target.value) s.autoLevels(e.target.value as LevelsAuto); }}>
         <option value="">Choose...</option>
         <option value="Contrast">Contrast</option>
         <option value="Color">Color</option>
```

```diff
--- a/app/src/panels/MenuBar.tsx
+++ b/app/src/panels/MenuBar.tsx
@@ -140,7 +140,7 @@ export function MenuBar() {
           {openMenu === m.title && (
             <div className="menu-items">
               {m.items.map((it, i) => it === "separator" ? <hr key={i} /> : (
-                <button key={it.id} data-testid={`menu-${it.id}`} disabled={it.enabled === false || s.busy} onClick={() => { setOpenMenu(null); it.run(); }}>{it.label}</button>
+                <button key={it.id} data-testid={`menu-${it.id}`} disabled={it.enabled === false || s.busy || s.working} onClick={() => { setOpenMenu(null); it.run(); }}>{it.label}</button>
               ))}
             </div>
           )}
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -1,7 +1,8 @@
 import { create } from "zustand";
 import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, LevelsAuto, PreviewEdit, SelectionMode, WandSettings } from "../engine/types";
 import { DEFAULT_WAND } from "../engine/types";
-import type { EngineClient } from "../engine/client";
+import type { EngineClient, JobInputCopy } from "../engine/client";
+import type { JobClient } from "../engine/jobs";
 import { cropSeed } from "../tools/crop-tool";
 import type { LassoKind, MarqueeKind, SelectionDraft } from "../tools/selection-draft";
 import type { ShellBridge } from "../shell/bridge";
@@ -51,8 +52,21 @@ export interface TransformEdit {
   duplicateEntry: number | null;
 }
 
+/** A layer with more pixels than this is edited, and its histogram read, by the job worker rather than
+ * on the UI thread (ruling OQ5): at 4 MP a Levels commit took about 0.35 s here. Below it a job's two
+ * copies and the worker's round trip cost more than they save. */
+export const JOB_PIXELS = 4_000_000;
+/** Said when a command arrives while a job's result is still to come. */
+export const BUSY_MESSAGE = "Wait for the current edit to finish.";
+
 export interface EditorStore {
   engine: EngineClient | null;
+  /** The job worker's client (engine `jobs.rs`); null until the engine has loaded. */
+  jobs: JobClient | null;
+  /** Layers with more pixels than this use the job worker (`JOB_PIXELS`; tests lower it). */
+  jobPixels: number;
+  /** True while an edit job's result is still to come (`runEditJob`): the Mac's `isProjectBusy`. */
+  working: boolean;
   bridge: ShellBridge | null;
   documents: Record<string, DocumentState>;
   order: string[];
@@ -87,6 +101,13 @@ export interface EditorStore {
   /** The mode Shift / Alt held over the canvas imply, for the options bar (`heldSelectionMode`). */
   heldSelectionMode: SelectionMode | null;
   setEngine(engine: EngineClient): void;
+  setJobs(jobs: JobClient): void;
+  /** Whether an edit of `layerId`'s pixels goes to the job worker: it has more than `jobPixels`. */
+  usesJob(layerId: string): boolean;
+  /** Runs `command` on `layerId` in the job worker and puts the result back as one undo step, unless
+   * the layer changed meanwhile. The document is busy until then: other commands, undo and redo wait.
+   * The canvas keeps what it showed (an open panel's preview) until the result is in. */
+  runEditJob(command: Command, layerId: string): Promise<boolean>;
   setBridge(bridge: ShellBridge): void;
   setBusy(busy: boolean): void;
   bumpRecent(): void;
@@ -187,13 +208,47 @@ function loadShowGuides(): boolean {
 }
 
 export const useEditor = create<EditorStore>((set, get) => ({
-  engine: null, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
+  engine: null, jobs: null, jobPixels: JOB_PIXELS, working: false, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
   sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, overlayTick: 0, recentTick: 0,
   selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, showGuides: loadShowGuides(),
   blendPreview: null,
   adjustEdit: null,
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
   setEngine: (engine) => set({ engine }),
+  setJobs: (jobs) => set({ jobs }),
+  usesJob: (layerId) => {
+    const { jobs, activeId, documents, jobPixels } = get();
+    const layer = activeId ? documents[activeId]?.layers.find((l) => l.id === layerId) : undefined;
+    return !!jobs && !!layer && layer.pixelsWidth * layer.pixelsHeight > jobPixels;
+  },
+  runEditJob: async (command, layerId) => {
+    const { engine, jobs, activeId } = get();
+    if (!engine || !jobs || !activeId) return false;
+    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
+    const doc = activeId;
+    let copy: JobInputCopy;
+    try { copy = engine.jobInput(doc, layerId); } catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
+    set({ working: true });
+    let installed = false;
+    try {
+      const result = await jobs.run(`edit:${doc}`, { kind: "edit", input: copy.input, pixels: copy.pixels, mask: copy.mask, command: JSON.stringify(command) });
+      // Closed meanwhile: nothing to put back.
+      if (!result || !get().documents[doc]) return false;
+      engine.installJob(doc, layerId, copy.input, result.header!, result.pixels, result.mask);
+      installed = true;
+      return true;
+    } catch (e) {
+      set({ error: String(e instanceof Error ? e.message : e) });
+      return false;
+    } finally {
+      set({ working: false });
+      if (get().documents[doc]) {
+        // A result that was not put back leaves a panel's preview behind: take it away.
+        if (!installed) engine.setPreview(doc, null);
+        get().refresh(doc);
+      }
+    }
+  },
   setBridge: (bridge) => set({ bridge }),
   setBusy: (busy) => set({ busy }),
   bumpRecent: () => set((s) => ({ recentTick: s.recentTick + 1 })),
@@ -262,6 +317,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
   run: (command) => {
     // Its own commit clears `adjustEdit` before calling this, so a panel's OK is never refused.
     if (get().panelOwnsDocument(true)) return false;
+    // A job's result is still to come: the layer it will land on must not change first.
+    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
     // A pending transform is closed before any other command records history. macOS refuses
     // these outright while `transformEdit != nil` (canEditLayers); committing is the gentler
     // equivalent and is what every action in actions/layers.ts already did individually.
@@ -285,8 +342,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
   },
   // A panel owns the document while it is open, as macOS's canEditLayers does; the menu items
   // for these are already disabled, so this stays quiet rather than raising the error banner.
-  undo: () => { if (get().panelOwnsDocument()) return; const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
-  redo: () => { if (get().panelOwnsDocument()) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
+  undo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
+  redo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
   setTool: (tool) => {
     if (get().tool === "move" && tool !== "move") get().commitTransform();
     // Entering the crop tool seeds a rectangle, as macOS does (EditorSession.selectTool): the
@@ -463,13 +520,24 @@ export const useEditor = create<EditorStore>((set, get) => ({
       kind, target: editing ? "adjustmentLayer" : "layer", layerId: layer.id,
       adjustment, params: filter ? defaultFilterParams(kind as FilterKind) : null,
       original: editing ? layer.adjustment! : null, preview: true, sampleMode: null,
-      // Levels and Curves draw a histogram of what they are about to change.
-      histogram: kind === "Levels" || kind === "Curves" ? engine.histogram(activeId, layer.id) : null,
+      // Levels and Curves draw a histogram of what they are about to change: a large layer's comes from
+      // the job worker once the panel is open, which it opens without waiting for.
+      histogram: kind === "Levels" || kind === "Curves" ? (!editing && get().usesJob(layer.id) ? null : engine.histogram(activeId, layer.id)) : null,
     };
     // The crop tool's rectangle goes, as macOS's beginFilter calls cancelCrop first: a pending
     // crop would otherwise answer the same Enter and Escape as the panel.
     set({ adjustEdit: edit, cropRect: null });
     get().applyAdjustPreview();
+    if ((kind === "Levels" || kind === "Curves") && !edit.histogram) {
+      const doc = activeId, jobs = get().jobs!;
+      const copy = engine.jobInput(doc, layer.id);
+      void jobs.run(`histogram:${doc}`, { kind: "histogram", input: copy.input, pixels: copy.pixels, mask: copy.mask }).then((result) => {
+        const open = get().adjustEdit;
+        // Only into the panel it was read for.
+        if (!result?.header || get().activeId !== doc || open?.layerId !== layer.id || open.kind !== kind) return;
+        set({ adjustEdit: { ...open, histogram: JSON.parse(result.header) as number[][] } });
+      }).catch((e) => set({ error: String(e instanceof Error ? e.message : e) }));
+    }
     return true;
   },
   updateAdjust: (patch) => {
@@ -535,11 +603,20 @@ export const useEditor = create<EditorStore>((set, get) => ({
   previewSettling: () => settleTimer !== null,
   commitAdjust: () => {
     const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
-    dropOpenPanel();
-    if (isAdjustIdentity(edit, (a) => engine.adjustmentIsIdentity(a))) { get().refresh(activeId); get().invalidate(); return; }
+    const identity = isAdjustIdentity(edit, (a) => engine.adjustmentIsIdentity(a));
     const command: Command = edit.target === "adjustmentLayer" ? { type: "SetAdjustment", id: edit.layerId, adjustment: edit.adjustment! }
       : edit.params ? { type: "ApplyFilter", id: edit.layerId, params: edit.params }
       : { type: "ApplyAdjustment", id: edit.layerId, adjustment: edit.adjustment! };
+    // A large layer is edited by the job worker: the panel closes but the canvas keeps its preview
+    // until the result is put back (runEditJob clears it then); the document is busy meanwhile.
+    if (!identity && edit.target === "layer" && get().usesJob(edit.layerId)) {
+      cancelSettle();
+      set({ adjustEdit: null });
+      void get().runEditJob(command, edit.layerId);
+      return;
+    }
+    dropOpenPanel();
+    if (identity) { get().refresh(activeId); get().invalidate(); return; }
     // A refused command (settings the engine will not accept) leaves the panel open with the
     // user's settings and its preview, under the banner, rather than discarding the edit.
     if (!get().run(command)) { set({ adjustEdit: edit }); get().applyAdjustPreview(); }
```

```diff
--- a/vite.config.ts
+++ b/vite.config.ts
@@ -11,5 +11,7 @@ export default defineConfig({
   // fail to connect even though Vite reports itself ready.
   server: { host: "127.0.0.1", port: 1420, strictPort: true, fs: { allow: [".."] } },
   build: { outDir: "dist", emptyOutDir: true, target: "es2022" },
+  // The job worker (app/src/engine/job-worker.ts) is a module worker that imports the engine's JS.
+  worker: { format: "es" },
   define: { __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0") },
 });
```


- [ ] **Step 4: Run the tests and watch them pass**

`pnpm wasm:dev`; `pnpm test`: 162 (+10); `pnpm build`; `pnpm e2e`: 136 passed, 7 skipped. Timings (release wasm, Edge, `-g jobs:`): Levels opens in 102 ms at 24 MP and 365 ms at 100 MP on the UI thread (the copy out and the first preview), the histogram arrives after 2.7 / 7.4 s with the page's frames never more than 22 / 17 ms apart (budget 150: the gap takes in the frame that first draws the panel's preview, measured 17-138 ms over four runs at 24 MP); OK copies out in 66 / 292 ms, the worker's edit leaves frames 17 ms apart (budget 100), and the result goes back in 81 / 380 ms (copies' budgets 150 / 500 ms).

- [ ] **Step 5: Prove it bites**

(1) In the store's `run`, drop the `working` refusal: `store-jobs.test.ts`'s "while a job's result is to come, commands and undo wait" fails (measured). Restore. (2) In `JobClient.run`, stop dropping the channel's waiting job: "drops a waiting job its channel superseded, and throws away a running one's result" fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/engine/job-worker.ts app/src/engine/jobs.ts app/tests/e2e/jobs.spec.ts app/tests/unit/jobs.test.ts app/tests/unit/store-jobs.test.ts
git commit -m "feat(app): a job worker edits large layers and reads their histograms off the UI thread" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/App.tsx app/src/actions/files.ts app/src/engine/client.ts app/src/engine/job-worker.ts app/src/engine/jobs.ts app/src/panels/LevelsPanel.tsx app/src/panels/MenuBar.tsx app/src/state/store.ts app/tests/e2e/jobs.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/engine-client.test.ts app/tests/unit/jobs.test.ts app/tests/unit/store-jobs.test.ts vite.config.ts
```


---

### Task 7: Effects images of large layers come from the worker

A styled layer's effects image was made on the UI thread whenever its pixels changed: seconds for a large layer. Past `EFFECTS_LIMITS.sync` (65,536 padded pixels) the renderer now draws the layer plainly at once, asks the worker for an image of the layer reduced to at most 1536 px (drawn scaled up, its effects scaled with it), then - for layers of at most 24 MP - for the full-size image, which the worker returns and the engine keeps in its effects cache (`keep_effects_image`, refused if the layer changed meanwhile), so the next frame, an export and the CPU compositor all find it. The last image drawn stays until a newer one lands. Ruling OQ6.

**Files:**
- Create: `app/src/canvas/effects-images.ts`
- Modify: `app/src/canvas/gl-renderer.ts` (placements, `largeEffects`, `textureKey`), `app/src/canvas/renderer.ts` (`RenderHooks`), `app/src/canvas/CanvasView.tsx`, `app/src/engine/client.ts` (`hasEffectsImage`, `keepEffectsImage`), `engine/src/effects/mod.rs` (`contains`, `insert`), `engine/src/jobs.rs` (`has_effects_image`, `keep_effects_image`), `engine-wasm/src/lib.rs`
- Create tests: `app/tests/unit/effects-images.test.ts`, `app/tests/e2e/effects-worker.spec.ts`; modify `engine/tests/jobs.rs`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `EFFECTS_LIMITS`, `reducedLevel`, `placedLike`, `EffectsImages` (`choose`, `ask`, `retainOnly`), `RenderHooks { jobs, landed }`, `createRenderer(canvas, hooks?)`; `Engine::has_effects_image`, `Engine::keep_effects_image(id, layer, stamp, edit, image)`, `EffectsCache::contains / insert`.

- [ ] **Step 1: Write the tests**

The engine tests: a full-size image a job made is found as if the engine had made it (the export equals the engine's own); an image is not kept for a layer that changed, or at another size. The unit tests: `reducedLevel` halves until the longer side is at most 1536; `placedLike` puts the layer's own pixels in its own box and an image with an inset grown in proportion, and undoes and redoes a distorted layer's corners; `EffectsImages` asks for a reduced image, then the full one, once each, draws what has landed, and asks nothing at full size past the limit. The e2e lowers the limits on a 96 x 73 styled layer so the real worker runs: drawn plainly, then from the reduced image, then at full size, which matches the CPU compositor.

Create `app/tests/e2e/effects-worker.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { softBlobPngBase64 } from "./helpers";

// Phase 4b-1: a styled layer too large to make its effects image on the UI thread is drawn plainly
// until the job worker's reduced image lands, then from it, then from the full-size image, which is
// the CPU compositor's own (engine/tests/jobs.rs pins the images). The sizes that choose the path
// are lowered here (`effectsLimits`) so a small layer takes it.

const DOC = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF10";
const STYLED = "0B6C6B1E-4F1B-4B4E-9E0A-EFEFEFEFEF12";
const EFFECTS = {
  stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 3 },
  shadow: { angle: 120, blue: 0.3, blur: 6, distance: 7, green: 0.2, opacity: 0.75, red: 0.2 },
  outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 5 },
};

/** The 40 x 24 soft blob at (28, 24) on a 96 x 73 canvas (odd, so 1:1 lands on whole device pixels in
 * this viewport), styled, alone. */
async function open(page: Page): Promise<string> {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const blob = await page.evaluate(softBlobPngBase64);
  return page.evaluate(async ({ blob, doc, styled, effects }) => {
    const api = (window as any).__compositor;
    // Everything counts as large; the reduced image is at most 32 px (the blob halved once).
    api.effectsLimits.sync = 0; api.effectsLimits.reduced = 32;
    const manifest = { format: "com.compositor.project", version: 9, colorSpace: "sRGB", documentID: doc, width: 96, height: 73,
      layers: [{ id: styled, name: "Styled", isVisible: true, imageFile: `${styled}.png`, effects,
        transform: { origin: [28, 24], size: [40, 24], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } }] };
    const images = [{ name: `${styled}.png`, bytes: Uint8Array.from(atob(blob), (c: string) => c.charCodeAt(0)) }];
    const id = api.engine.openPackage({ manifest: JSON.stringify(manifest), images }, null);
    api.setCheckerboard(false);
    return id;
  }, { blob, doc: DOC, styled: STYLED, effects: EFFECTS });
}
/** One frame drawn now: how many of the GPU's pixels outside the blob's box show anything, and whether
 * the engine holds the full-size image. The picture is kept as `__pictures[name]`. (The CPU compositor
 * is not asked here: making its own image would put the full-size one in the engine's cache.) */
const look = (page: Page, name: string) => page.evaluate((name) => {
  const api = (window as any).__compositor;
  const s = api.store.getState();
  api.renderer.render(api.engine, s.documents[s.activeId], s.viewports[s.activeId], window.devicePixelRatio || 1, { checkerboard: false }, null);
  const gpu = api.readDocumentPixels() as Uint8Array;
  const pictures = ((window as any).__pictures ??= {});
  pictures[name] = gpu;
  let outside = 0;
  for (let y = 0; y < 73; y++) for (let x = 0; x < 96; x++) if ((x < 28 || x >= 68 || y < 24 || y >= 48) && gpu[(y * 96 + x) * 4 + 3] > 0) outside++;
  return { outside, full: api.engine.hasEffectsImage(s.activeId, s.documents[s.activeId].layers[0].id, null) as boolean };
}, name);
/** The largest channel difference between two kept pictures, or the kept `a` and the CPU's composite. */
const worst = (page: Page, a: string, b: string | "cpu") => page.evaluate(([a, b]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  const x = (window as any).__pictures[a] as Uint8Array;
  const y = b === "cpu" ? api.engine.composite(s.activeId, { x: 0, y: 0, width: 96, height: 73 }, 96, 73) as Uint8Array : (window as any).__pictures[b] as Uint8Array;
  let d = 0; for (let i = 0; i < x.length; i++) d = Math.max(d, Math.abs(x[i] - y[i]));
  return d;
}, [a, b]);

test("a styled layer too large for the UI thread is drawn plainly, then from the worker's reduced image, then at full size as the CPU draws it", async ({ page }) => {
  const doc = await open(page);
  // A worker whose every job comes to nothing: nothing but the layer itself can be drawn.
  await page.evaluate(async (doc) => {
    const api = (window as any).__compositor;
    (window as any).__realJobs = api.store.getState().jobs;
    api.store.setState({ jobs: { run: () => Promise.resolve(null), cancel: () => {} } });
    api.effectsLimits.full = 0;
    api.store.getState().openDocument(doc);
    await api.setZoom(1);
  }, doc);
  // Compared pixel for pixel, so the document must sit on whole device pixels (LL-065(6)): checked.
  const whole = await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState(); const d = s.documents[s.activeId]; const dpr = window.devicePixelRatio || 1;
    const r = s.viewports[s.activeId].documentRect({ width: d.width, height: d.height });
    return Number.isInteger(r.x * dpr) && Number.isInteger(r.y * dpr);
  });
  expect(whole, "the document sits on whole device pixels").toBe(true);
  expect((await look(page, "plain")).outside, "no effects: the layer alone").toBe(0);
  // The real worker, reduced images only: the effects appear.
  await page.evaluate(() => { const api = (window as any).__compositor; api.store.setState({ jobs: (window as any).__realJobs }); api.store.getState().invalidate(); });
  await expect.poll(async () => (await look(page, "reduced")).outside, { timeout: 15_000 }).toBeGreaterThan(50);
  expect((await look(page, "reduced")).full).toBe(false);
  // Full size allowed again: the worker's image goes into the engine's cache and the GPU draws it.
  await page.evaluate(() => { const api = (window as any).__compositor; api.effectsLimits.full = 24_000_000; api.store.getState().invalidate(); });
  await expect.poll(async () => (await look(page, "full")).full, { timeout: 15_000 }).toBe(true);
  expect((await look(page, "full")).outside).toBeGreaterThan(50);
  expect(await worst(page, "full", "cpu"), "the full-size image is the CPU's").toBeLessThanOrEqual(2);
  expect(await worst(page, "reduced", "full"), "the reduced image was a reduced one").toBeGreaterThan(2);
});
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -102,19 +102,29 @@ test("jobs: the Levels histogram and commit through the worker at 24 and 100 MP,
       const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
       const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
       const result: Record<string, number> = {};
-      // The main thread's two copies, timed where the store makes them.
-      const timed = (name: string) => { const f = api.engine[name].bind(api.engine); api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); } }; };
-      timed("jobInput"); timed("installJob");
+      // The main thread's two copies, timed where the store makes them; when the result is put back,
+      // the store's state is brought up to date and one frame drawn at once, timed too.
+      let installedAt = Infinity;
+      const timed = (name: string, after?: () => void) => {
+        const f = api.engine[name].bind(api.engine);
+        api.engine[name] = (...a: unknown[]) => {
+          const t0 = performance.now();
+          try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); after?.(); }
+        };
+      };
+      timed("jobInput");
+      timed("installJob", () => { installedAt = performance.now(); api.store.getState().refresh(api.store.getState().activeId); result["frame after the result is put back"] = Math.round((window as any).__frame()); });
       const doc = api.engine.newDocument(10, 10, false);
       api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
       const layer = api.engine.state(doc).layers[0].id;
       api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
       api.store.getState().openDocument(doc);
       await settle(); frame();
-      // The longest gap between animation frames while `until` is false.
+      // The longest gap between animation frames while `until` is false, counting only frames that
+      // came before the result was put back (`installedAt`): the worker's own time.
       const longestGap = (until: () => boolean) => new Promise<number>((done) => {
         let last = performance.now(), gap = 0;
-        const tick = (t: number) => { gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+        const tick = (t: number) => { if (t <= installedAt) gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
         requestAnimationFrame(tick);
       });
       let t0 = performance.now();
@@ -125,13 +135,14 @@ test("jobs: the Levels histogram and commit through the worker at 24 and 100 MP,
       const adjustment = JSON.parse(JSON.stringify(api.store.getState().adjustEdit.adjustment));
       adjustment.levels.ranges[0].outputWhite = 200;
       api.store.getState().updateAdjust({ adjustment });
-      await new Promise((r) => setTimeout(r, 400));
+      // The full-quality preview follows the quick one (store SETTLE_MS) and is drawn before OK.
+      await new Promise<void>((done) => { const poll = () => (api.store.getState().previewSettling() ? setTimeout(poll, 20) : done()); poll(); });
+      await new Promise((r) => setTimeout(r, 300)); await settle();
       t0 = performance.now();
       api.store.getState().commitAdjust();
       result["OK (UI thread) ms"] = Math.round(performance.now() - t0);
       result["commit: longest frame gap while the worker edits"] = await longestGap(() => !api.store.getState().working);
       result["commit done after ms"] = Math.round(performance.now() - t0);
-      result["frame after the result is put back"] = Math.round(frame());
       result["undo depth"] = api.engine.state(doc).undoDepth;
       return result;
     }, [w, h]);
@@ -139,12 +150,88 @@ test("jobs: the Levels histogram and commit through the worker at 24 and 100 MP,
   }
   console.log(`jobs (release wasm, Edge): ${JSON.stringify(out)}`);
   for (const label of ["24 MP", "100 MP"]) {
-    // The worker's own time never holds the page up past 100 ms; the copies are budgeted per size.
-    expect(out[`${label}: histogram: longest frame gap while the worker reads it`]).toBeLessThan(label === "24 MP" ? 150 : 450);
-    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 450);
+    // The worker's own time never holds the page up past 100 ms while it edits. While it reads the
+    // histogram the gap also takes in the frame that first draws the panel's preview on the UI thread
+    // (measured 17 to 138 ms at 24 MP over four runs): 150. The copies are budgeted per size
+    // (measured 66 to 108 ms at 24 MP and 264 to 380 ms at 100 MP).
+    expect(out[`${label}: histogram: longest frame gap while the worker reads it`]).toBeLessThan(150);
+    expect(out[`${label}: commit: longest frame gap while the worker edits`]).toBeLessThan(100);
+    expect(out[`${label}: jobInput ms`]).toBeLessThan(label === "24 MP" ? 150 : 500);
+    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 500);
   }
 });
 
+test("effects images: a large styled layer opens drawn plainly, then reduced, then (to 24 MP) at full size, without holding the page up", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const result: Record<string, number> = {};
+      // A styled layer this size: the package of a filled canvas, re-opened with all six effects.
+      const plain = api.engine.newDocument(10, 10, false);
+      api.engine.execute(plain, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const files = api.engine.savePackage(plain);
+      api.engine.closeDocument(plain);
+      const manifest = JSON.parse(files.manifest);
+      manifest.layers[0].effects = {
+        stroke: { blue: 0.2, green: 0.8, inside: false, opacity: 0.9, red: 0.1, size: 12 },
+        shadow: { angle: 120, blue: 0.3, blur: 30, distance: 25, green: 0.2, opacity: 0.75, red: 0.2 },
+        colorOverlay: { blue: 0.4, green: 0.1, opacity: 0.3, red: 0.9 },
+        innerShadow: { angle: -35, blue: 0.05, blur: 20, distance: 15, green: 0.05, opacity: 0.6, red: 0.05 },
+        outerGlow: { blue: 0.2, green: 0.9, opacity: 0.6, red: 1, size: 25 },
+        innerGlow: { blue: 1, green: 1, opacity: 0.5, red: 1, size: 20 } };
+      manifest.version = 9;
+      const doc = api.engine.openPackage({ manifest: JSON.stringify(manifest), images: files.images }, null);
+      const layer = api.engine.state(doc).layers[0].id;
+      // Frames and their gaps from here until the work is done.
+      let last = performance.now(), gap = 0, running = true;
+      const tick = (t: number) => { gap = Math.max(gap, t - last); last = t; if (running) requestAnimationFrame(tick); };
+      requestAnimationFrame(tick);
+      let t0 = performance.now();
+      api.store.getState().openDocument(doc);
+      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      result["first frame (the layer plainly), ms"] = Math.round(frame());
+      // What the layer's texture holds: "px" its own pixels, "rd" a reduced image, "fx" the full one.
+      const shown = () => String(api.renderer.textureKey(doc, layer) ?? "").slice(0, 2);
+      const waitFor = (done: () => boolean, limit: number) => new Promise<boolean>((resolve) => {
+        const start = performance.now();
+        const poll = () => { if (done()) resolve(true); else if (performance.now() - start > limit) resolve(false); else setTimeout(poll, 20); };
+        poll();
+      });
+      result["the first frame draws the layer plainly (1 = yes)"] = shown() === "px" ? 1 : 0;
+      gap = 0;
+      const reduced = await waitFor(() => { frame(); return shown() === "rd"; }, 60_000);
+      result["reduced image drawn after, ms"] = reduced ? Math.round(performance.now() - t0) : -1;
+      result["longest frame gap until then"] = Math.round(gap);
+      if (w * h <= 24_000_000) { // the layer's own pixels (EFFECTS_LIMITS.full)
+        gap = 0; t0 = performance.now();
+        const full = await waitFor(() => api.engine.hasEffectsImage(doc, layer, null), 120_000);
+        result["full image kept after, ms"] = full ? Math.round(performance.now() - t0) : -1;
+        result["longest frame gap while the worker made it"] = Math.round(gap);
+        result["frame that draws it (halving and upload), ms"] = Math.round(frame());
+        result["then drawn at full size (1 = yes)"] = shown() === "fx" ? 1 : 0;
+      }
+      running = false;
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`effects images (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) {
+    expect(out[`${label}: the first frame draws the layer plainly (1 = yes)`]).toBe(1);
+    expect(out[`${label}: reduced image drawn after, ms`]).toBeGreaterThan(0);
+  }
+  expect(out["24 MP: then drawn at full size (1 = yes)"]).toBe(1);
+  expect(out["24 MP: longest frame gap until then"]).toBeLessThan(100);
+  // At 100 MP the first frame halves the layer three times to draw it plainly (natively 100 ms;
+  // measured 133 to 167 ms in the release wasm over three runs).
+  expect(out["100 MP: longest frame gap until then"]).toBeLessThan(200);
+  expect(out["24 MP: longest frame gap while the worker made it"]).toBeLessThan(150);
+});
+
 test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push at the cap is cheap", async ({ page }) => {
   test.setTimeout(900_000);
   await ready(page);
```

Create `app/tests/unit/effects-images.test.ts`:

```ts
import { afterEach, describe, expect, it } from "vitest";
import { EFFECTS_LIMITS, EffectsImages, placedLike, reducedLevel } from "../../src/canvas/effects-images";
import type { Corners, LayerDraw, LayerTransform } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient, JobRequest, JobResult } from "../../src/engine/jobs";
import { homographyUnitTo, mat3Apply } from "../../src/tools/transform-geometry";

const defaults = { ...EFFECTS_LIMITS };
afterEach(() => Object.assign(EFFECTS_LIMITS, defaults));

/** The plan's draw of a styled 40 x 24 layer at (28, 24) with an inset of 10: the engine's grown
 * transform (effects/mod.rs `grown_transform`): 60 x 44 about the same centre. */
function drawOf(extra: Partial<LayerDraw> = {}): LayerDraw {
  const transform: LayerTransform = { origin: [18, 14], size: [60, 44], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
  return { id: "A", transform, corners: null, pixelsWidth: 60, pixelsHeight: 44, pixelsRevision: 3, opacity: 1, blend: "Normal", coverages: [], clip: null,
    keepsAlpha: false, effects: { inset: 10, key: "k" }, ...extra };
}

describe("reducedLevel", () => {
  it("halves until the longer side is at most 1536", () => {
    expect(reducedLevel(1536, 900)).toBe(0);
    expect(reducedLevel(1537, 10)).toBe(1);
    expect(reducedLevel(6000, 4000)).toBe(2); // 6000 -> 3000 -> 1500
    expect(reducedLevel(10000, 10000)).toBe(3); // -> 1250
  });
});

describe("placedLike", () => {
  it("puts the layer's own pixels at its own box, and an image with another inset grown in proportion", () => {
    const d = drawOf();
    expect(placedLike(d, 40, 24, 0).transform).toMatchObject({ origin: [28, 24], size: [40, 24] });
    expect(placedLike(d, 40, 24, 10).transform).toMatchObject({ origin: [18, 14], size: [60, 44] });
    // Halved pixels with an inset of 6 (rounded up from 5): 40 * (20 + 12) / 20 = 64 wide about x 48.
    expect(placedLike(d, 20, 12, 6).transform).toMatchObject({ origin: [16, 12], size: [64, 48] });
  });

  it("undoes and redoes the growth of a distorted layer's corners", () => {
    // A parallelogram for the layer's own corners, and the plan's corners for inset 10 around 40 x 24.
    const own = [{ x: 30, y: 20 }, { x: 70, y: 26 }, { x: 66, y: 50 }, { x: 26, y: 44 }];
    const h = homographyUnitTo(own);
    const [a, b] = [10 / 40, 10 / 24];
    const grown = [mat3Apply(h, { x: -a, y: -b }), mat3Apply(h, { x: 1 + a, y: -b }), mat3Apply(h, { x: 1 + a, y: 1 + b }), mat3Apply(h, { x: -a, y: 1 + b })];
    const d = drawOf({ corners: grown.map((p) => [p.x, p.y]) as Corners });
    const plain = placedLike(d, 40, 24, 0).corners!;
    plain.forEach(([x, y], i) => { expect(x).toBeCloseTo(own[i].x, 9); expect(y).toBeCloseTo(own[i].y, 9); });
  });
});

describe("EffectsImages.choose", () => {
  /** A stub engine that has the full image once `full` is set, and a stub worker whose jobs finish
   * when `finish` is called. */
  function setup() {
    const asked: JobRequest[] = [];
    const state = { full: false, kept: 0 };
    let finish: (r: JobResult | null) => void = () => {};
    const engine = {
      hasEffectsImage: () => state.full,
      displayJobInput: (_d: string, _l: string, level: number) => ({ input: JSON.stringify({ pixels: [40 >> level, 24 >> level], stamp: {} }), pixels: new ArrayBuffer(4), mask: null }),
      keepEffectsImage: () => { state.kept++; return true; },
    } as unknown as EngineClient;
    const jobs = { run: (_c: string, r: JobRequest) => { asked.push(r); return new Promise<JobResult | null>((resolve) => { finish = resolve; }); } } as unknown as JobClient;
    let landed = 0;
    const images = new EffectsImages(() => jobs, () => { landed++; });
    return { images, engine, asked, state, finish: (r: JobResult | null) => finish(r), landed: () => landed };
  }
  const flush = () => new Promise((r) => setTimeout(r, 0));

  it("asks for a reduced image, then the full one, once each, and draws what has landed", async () => {
    EFFECTS_LIMITS.reduced = 32;
    const { images, engine, asked, state, finish, landed } = setup();
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBeNull();
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBeNull();
    expect(asked.map((r) => (r as { factor: number }).factor), "one reduced job, halved once").toEqual([0.5]);
    finish({ header: JSON.stringify({ width: 32, height: 24, inset: 6 }), pixels: new ArrayBuffer(32 * 24 * 4), mask: null });
    await flush();
    expect(landed()).toBe(1);
    const reduced = images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    expect(reduced).toMatchObject({ key: "fx:1", width: 32, inset: 6 });
    expect(asked.map((r) => (r as { factor: number }).factor), "then the full one").toEqual([0.5, 1]);
    finish({ header: JSON.stringify({ width: 60, height: 44, inset: 10 }), pixels: new ArrayBuffer(60 * 44 * 4), mask: null });
    await flush();
    expect(state.kept, "the full image goes to the engine").toBe(1);
    state.full = true;
    expect(images.choose(engine, "D", "A", "fx:1", drawOf(), 40, 24, null)).toBe("full");
    // New pixels: the last reduced image is drawn meanwhile, and a reduced one for them is asked for.
    state.full = false;
    expect(images.choose(engine, "D", "A", "fx:2", drawOf(), 40, 24, null)).toMatchObject({ key: "fx:1" });
    expect(asked.length).toBe(3);
  });

  it("asks for nothing at full size past the full-size limit, and for the full image at once when no reduction is needed", async () => {
    EFFECTS_LIMITS.reduced = 32; EFFECTS_LIMITS.full = 40 * 24 - 1;
    const big = setup();
    big.images.choose(big.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    big.finish({ header: JSON.stringify({ width: 32, height: 24, inset: 6 }), pixels: new ArrayBuffer(4), mask: null });
    await flush();
    big.images.choose(big.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    expect(big.asked.length, "reduced only").toBe(1);
    EFFECTS_LIMITS.reduced = 1536; EFFECTS_LIMITS.full = defaults.full;
    const small = setup();
    small.images.choose(small.engine, "D", "A", "fx:1", drawOf(), 40, 24, null);
    expect(small.asked.map((r) => (r as { factor: number }).factor)).toEqual([1]);
  });
});
```

```diff
--- a/engine/tests/jobs.rs
+++ b/engine/tests/jobs.rs
@@ -118,6 +118,49 @@ fn a_histogram_job_equals_the_histogram_in_place() {
     assert_eq!(run_histogram_job(&input, pixels, mask).unwrap(), e.histogram(id, layer).unwrap());
 }
 
+/// The 90 x 60 `pattern` layer at (23, 17) on 160 x 110 with a stroke and a drop shadow, made in the
+/// engine it is edited in (a document moved from another engine would bring that engine's revisions).
+fn styled() -> (Engine, Uuid, Uuid) {
+    let mut doc = Document::new(160, 110);
+    let mut layer = Layer::with_pixels("Styled", pattern(90, 60), p(23.0, 17.0));
+    layer.extra.effects = Some(serde_json::from_value(serde_json::json!({
+        "stroke": { "blue": 0.1, "green": 0.55, "inside": false, "opacity": 0.8, "red": 0.95, "size": 6 },
+        "shadow": { "angle": 120, "blue": 0.5, "blur": 9, "distance": 14, "green": 0.1, "opacity": 0.7, "red": 0.2 } })).unwrap());
+    let id = layer.id;
+    doc.active_layer_id = Some(id);
+    doc.layers = vec![layer];
+    let mut e = Engine::new();
+    let handle = e.insert_document(doc);
+    (e, handle, id)
+}
+
+#[test]
+fn a_full_size_effects_image_a_job_made_is_found_as_if_the_engine_had_made_it() {
+    let (e, id, layer) = styled();
+    assert!(!e.has_effects_image(id, layer, None).unwrap());
+    let (input, pixels, mask) = crossed(&e, id, layer);
+    let (_, image) = run_effects_job(&input, pixels.unwrap(), mask, 1.0, None).unwrap().unwrap();
+    let bytes = image.bytes().to_vec();
+    assert!(e.keep_effects_image(id, layer, input.stamp, None, image).unwrap());
+    assert!(e.has_effects_image(id, layer, None).unwrap());
+    let made = e.effects_cache().made();
+    let drawn = e.draw_raster(id, layer, 0, None).unwrap().unwrap();
+    assert_eq!(e.effects_cache().made(), made, "found, not made again");
+    assert_eq!(drawn.bytes(), bytes.as_slice());
+}
+
+#[test]
+fn an_effects_image_is_not_kept_for_a_layer_that_changed_or_at_another_size() {
+    let (mut e, id, layer) = styled();
+    let (input, pixels, mask) = crossed(&e, id, layer);
+    let (_, image) = run_effects_job(&input, pixels.clone().unwrap(), mask.clone(), 1.0, None).unwrap().unwrap();
+    let (_, small) = run_effects_job(&input, pixels.unwrap().halved(), mask, 0.5, None).unwrap().unwrap();
+    assert!(!e.keep_effects_image(id, layer, input.stamp, None, small).unwrap(), "a reduced image is not the full one");
+    run(&mut e, id, Command::InvertPixels { id: layer, mask: false });
+    assert!(!e.keep_effects_image(id, layer, input.stamp, None, image).unwrap(), "the pixels changed");
+    assert!(!e.has_effects_image(id, layer, None).unwrap());
+}
+
 #[test]
 fn an_effects_job_at_full_size_equals_the_engines_own_image_and_a_reduced_one_scales_its_effects() {
     let (mut e, id, layer) = document();
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test jobs`: does not compile (`keep_effects_image`). `pnpm test`: `effects-images.test.ts` does not compile.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -3,6 +3,7 @@ import { useEditor } from "../state/store";
 import { createRenderer, type Renderer } from "./renderer";
 import { drawOverlay, type AntsState } from "./overlay";
 import { installTestApi } from "../test-api";
+import { EFFECTS_LIMITS } from "./effects-images";
 import { CropSession, hitTest, ratioValue, SNAP_SCREEN_PX } from "../tools/crop-tool";
 import { TransformSession, startMode } from "../tools/transform-session";
 import { containsPoint, cornersToTuples, fromTuple, hitOverlay, overlayGeometry, snapTargets, type OverlayGeometry, type P } from "../tools/transform-geometry";
@@ -78,7 +79,7 @@ export function CanvasView() {
   // Renderer lifetime follows the canvas element.
   useEffect(() => {
     if (!engine || !glRef.current) return;
-    const renderer = createRenderer(glRef.current);
+    const renderer = createRenderer(glRef.current, { jobs: () => useEditor.getState().jobs, landed: () => useEditor.getState().invalidate() });
     rendererRef.current = renderer;
     useEditor.getState().setRendererKind(renderer.kind);
     installTestApi({
@@ -102,6 +103,8 @@ export function CanvasView() {
         for (let y = 0; y < h; y++) out.set(all.subarray(((y0 + y) * W + x0) * 4, ((y0 + y) * W + x0 + w) * 4), y * w * 4);
         return out;
       },
+      // Where large styled layers' effects images are made (tests lower the sizes to reach the worker).
+      effectsLimits: EFFECTS_LIMITS,
       // The overlay painted now, synchronously: the perf harness times an ants tick with it.
       paintOverlay: () => paintOverlay(),
       // Exposed for e2e tests to compute where the on-screen transform handles (including the
```

Create `app/src/canvas/effects-images.ts`:

```ts
// The canvas's effects images for large styled layers (Phase 4b-1; spec section 3, the user's choice
// of 2026-09-28): a reduced image first, at most 1536 px as the Mac's canvas preview is
// (EffectsPreviewCache.swift:51, :114-133), then the full-size one while the padded image is at most
// about 24 MP; above that only the reduced one. Both are made by the job worker; until one lands the
// last image made for the layer stays on screen, and with none the layer is drawn plainly, as the Mac
// draws it until its first preview is ready. Small images are made at once on the UI thread, as before.
import type { Corners, LayerDraw, LayerTransform, PreviewEdit } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { JobClient } from "../engine/jobs";
import { sizeAtLevel } from "./layer-textures";
import { fromTuple, homographyUnitTo, mat3Apply, toTuple } from "../tools/transform-geometry";

/** The sizes that choose the path (tests lower them): a padded image of at most `sync` pixels is made
 * on the UI thread; the reduced image's longer side is at most `reduced`; the full-size image is made
 * for a layer of at most `full` pixels of its own (a 6000 x 4000 photo qualifies, effects and all). */
export const EFFECTS_LIMITS = { sync: 65_536, reduced: 1536, full: 24_000_000 };

/** An effects image the worker made at a reduction (the full-size ones go into the engine's cache). */
export interface ReducedImage { key: string; width: number; height: number; inset: number; bytes: Uint8Array; }

/** The fewest halvings that bring the layer's longer side to at most `EFFECTS_LIMITS.reduced`. */
export function reducedLevel(width: number, height: number): number {
  let level = 0;
  for (let s = sizeAtLevel(width, height, 0); Math.max(s.width, s.height) > EFFECTS_LIMITS.reduced && s.width > 1 && s.height > 1; s = sizeAtLevel(width, height, ++level)) { /* halve */ }
  return level;
}

/** Where an effects image with its own `inset` around `width` x `height` layer pixels is drawn, given
 * the plan's draw of the full padded image (`draw.effects.inset` around `draw.pixelsWidth` padded):
 * the layer's own box, grown in proportion to this image (engine `grown_transform`, `grown_corners`).
 * An inset of 0 is the layer drawn plainly. */
export function placedLike(draw: LayerDraw, width: number, height: number, inset: number): { transform: LayerTransform; corners: Corners | null } {
  const i0 = draw.effects!.inset;
  const pw = draw.pixelsWidth - 2 * i0, ph = draw.pixelsHeight - 2 * i0;
  if (draw.corners) {
    // The plan's corners map the unit square grown by (i0 / pw, i0 / ph); reach this image's.
    const grown = homographyUnitTo(draw.corners.map(fromTuple));
    const [a, b, a1, b1] = [i0 / pw, i0 / ph, inset / width, inset / height];
    const at = (u: number, v: number) => mat3Apply(grown, { x: (u + a) / (1 + 2 * a), y: (v + b) / (1 + 2 * b) });
    const corners = [at(-a1, -b1), at(1 + a1, -b1), at(1 + a1, 1 + b1), at(-a1, 1 + b1)].map(toTuple) as Corners;
    return { transform: draw.transform, corners };
  }
  const t = draw.transform;
  const own = { width: t.size[0] * pw / (pw + 2 * i0), height: t.size[1] * ph / (ph + 2 * i0) };
  const size: [number, number] = [own.width * (width + 2 * inset) / width, own.height * (height + 2 * inset) / height];
  const cx = t.origin[0] + t.size[0] / 2, cy = t.origin[1] + t.size[1] / 2;
  return { transform: { ...t, origin: [cx - size[0] / 2, cy - size[1] / 2], size }, corners: null };
}

/** The effects images of one renderer's large styled layers, and the jobs that make them. */
export class EffectsImages {
  /** The newest reduced image landed for each `doc:layer`, whatever pixels it was made from. */
  private reduced = new Map<string, ReducedImage>();
  /** What was last asked for each `doc:layer`, so a frame asks once. */
  private asked = new Map<string, string>();

  constructor(private readonly jobs: () => JobClient | null, private readonly landed: () => void) {}

  /** For a large styled layer this frame: "full" when the engine has the full-size image (draw it the
   * usual way), else the reduced image to draw (null: none yet, draw the layer plainly). Asks the
   * worker for what the image named `key` still lacks. */
  choose(engine: EngineClient, doc: string, layer: string, key: string, draw: LayerDraw, pixelsWidth: number, pixelsHeight: number, edit: PreviewEdit | null): "full" | ReducedImage | null {
    const k = `${doc}:${layer}`;
    const wantFull = pixelsWidth * pixelsHeight <= EFFECTS_LIMITS.full;
    if (wantFull && engine.hasEffectsImage(doc, layer, edit)) return "full";
    const have = this.reduced.get(k) ?? null;
    const level = reducedLevel(pixelsWidth, pixelsHeight);
    // A layer small enough needs no reduced image: its full-size one is asked for straight away.
    const next = have?.key === key || level === 0 ? (wantFull ? "full" : null) : "reduced";
    if (next && this.asked.get(k) !== `${key}:${next}`) {
      this.asked.set(k, `${key}:${next}`);
      this.ask(engine, doc, layer, key, next === "full" ? 0 : level, pixelsWidth, edit);
    }
    return have;
  }

  private ask(engine: EngineClient, doc: string, layer: string, key: string, level: number, pixelsWidth: number, edit: PreviewEdit | null): void {
    const jobs = this.jobs(); if (!jobs) return;
    const copy = engine.displayJobInput(doc, layer, level);
    const width = (JSON.parse(copy.input) as { pixels: [number, number] }).pixels[0];
    const request = { kind: "effects" as const, input: copy.input, pixels: copy.pixels!, mask: copy.mask, factor: width / pixelsWidth, edit: edit ? JSON.stringify(edit) : null };
    const k = `${doc}:${layer}`;
    const asked = `${key}:${level === 0 ? "full" : "reduced"}`;
    // However the job ends, the next frame asks for what is still missing (again, if the engine let
    // the image go or the job came to nothing), unless a newer ask has taken its place.
    const settled = () => { if (this.asked.get(k) === asked) this.asked.delete(k); };
    jobs.run(`fx:${k}`, request).then((result) => {
      settled();
      // Superseded by a newer image's job, or nothing to draw.
      if (!result?.header || !result.pixels) return;
      const image = JSON.parse(result.header) as { width: number; height: number; inset: number };
      if (level === 0) engine.keepEffectsImage(doc, layer, copy.input, edit, image.width, image.height, result.pixels);
      else this.reduced.set(k, { key, ...image, bytes: new Uint8Array(result.pixels) });
      this.landed();
    }).catch(settled);
  }

  /** Forgets the layers of `doc` not in `ids`, and every other document's. */
  retainOnly(doc: string, ids: Set<string>): void {
    for (const map of [this.reduced, this.asked] as Map<string, unknown>[]) {
      for (const k of [...map.keys()]) { const [d, l] = k.split(":"); if (d !== doc || !ids.has(l)) map.delete(k); }
    }
  }
}
```

```diff
--- a/app/src/canvas/gl-renderer.ts
+++ b/app/src/canvas/gl-renderer.ts
@@ -1,9 +1,10 @@
-import type { Coverage, DocumentState, LayerDraw, PreviewEdit, RenderPlan } from "../engine/types";
+import type { Corners, Coverage, DocumentState, LayerDraw, LayerTransform, PreviewEdit, RenderPlan } from "../engine/types";
 import { DEFAULT_BLACK_WHITE, DEFAULT_COLOR_BALANCE, isSpatialKind, type AdjustmentKind } from "../engine/types";
 import type { EngineClient } from "../engine/client";
 import type { Viewport } from "./viewport";
 import { LayerTextures, levelRect, prefilterLevel, sizeAtLevel } from "./layer-textures";
-import type { RenderOptions, Renderer } from "./renderer";
+import type { RenderHooks, RenderOptions, Renderer } from "./renderer";
+import { EFFECTS_LIMITS, EffectsImages, placedLike } from "./effects-images";
 import { ADJUST_KIND, BLEND_INDEX, createPrograms, disposePrograms, type Program, type Programs } from "./gl/programs";
 import { FboPool, type Target } from "./gl/framebuffers";
 import { MaskTextures } from "./gl/mask-textures";
@@ -40,9 +41,17 @@ export class GlRenderer implements Renderer {
   private dpr = 1;
   /** gl.MAX_TEXTURE_SIZE, read once in the constructor: the frame must fit one texture (audit F-M2). */
   private readonly maxTexture: number;
-  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext) {
+  /** Large styled layers' effects images, made by the job worker (effects-images.ts). */
+  private effectsImages: EffectsImages;
+  /** Where a large styled layer's texture is drawn when it is not the plan's full-size image: a
+   * reduced image, the last image kept, or the layer's own pixels (by layer id, this document). */
+  private placements = new Map<string, { transform: LayerTransform; corners: Corners | null }>();
+  /** The effects image a large styled layer's texture holds: its layer pixels and inset, at full size. */
+  private shown = new Map<string, { width: number; height: number; inset: number }>();
+  constructor(private readonly canvas: HTMLCanvasElement, private readonly gl: WebGL2RenderingContext, hooks: RenderHooks = { jobs: () => null, landed: () => {} }) {
     this.textures = new LayerTextures(gl); this.masks = new MaskTextures(gl); this.fbos = new FboPool(gl); this.programs = createPrograms(gl);
     this.adjustTextures = new AdjustTextures(gl);
+    this.effectsImages = new EffectsImages(hooks.jobs, hooks.landed);
     this.white = this.solid(gl.R8, gl.RED, [255]); this.transparent = this.solid(gl.RGBA8, gl.RGBA, [0, 0, 0, 0]);
     this.maxTexture = gl.getParameter(gl.MAX_TEXTURE_SIZE) as number;
   }
@@ -89,12 +98,12 @@ export class GlRenderer implements Renderer {
     const levels = new Map<string, number>();
     // A layer the plan draws with its effects: its texture is the engine's padded effects image,
     // the very bytes compositor::draw_raster samples, at the draw's padded size.
-    const padded = new Map<string, { width: number; height: number; inset: number; key: string }>();
+    const padded = new Map<string, { width: number; height: number; inset: number; key: string; draw: LayerDraw }>();
     const adjustKeys = new Set<string>();
     const note = (d: LayerDraw) => {
       if (d.adjustment) adjustKeys.add(AdjustTextures.key(d.adjustment));
       if (d.pixelsWidth === 0) return;
-      if (d.effects) padded.set(d.id, { width: d.pixelsWidth, height: d.pixelsHeight, inset: d.effects.inset, key: d.effects.key });
+      if (d.effects) padded.set(d.id, { width: d.pixelsWidth, height: d.pixelsHeight, inset: d.effects.inset, key: d.effects.key, draw: d });
       const layer = state.layers.find((l) => l.id === d.id);
       const nearest = layer?.transform.sampling === "Nearest";
       // Nearest never prefilters, and neither does a distortion: the homography resamples the
@@ -118,6 +127,8 @@ export class GlRenderer implements Renderer {
       // image by both revisions and the whole EffectsDraw: a pixel edit, a mask edit, an undo, a
       // redo or a panel preview each upload again, and a move that keeps the image does not.
       const bytesKey = fx ? `fx:${layer.pixelsRevision}:${layer.maskRevision}:${fx.inset}:${fx.key}` : `px:${layer.pixelsRevision}`;
+      if (!fx || fx.width * fx.height <= EFFECTS_LIMITS.sync || this.largeEffects(engine, state, layer.id, bytesKey, fx.draw, nearest, outPerDoc, edit)) this.placements.delete(layer.id);
+      else continue;
       if (!this.textures.needsUpload(state.id, layer.id, bytesKey, level, nearest)) continue;
       const [width, height] = fx ? [fx.width, fx.height] : [layer.pixelsWidth, layer.pixelsHeight];
       const size = sizeAtLevel(width, height, level);
@@ -136,8 +147,41 @@ export class GlRenderer implements Renderer {
       // An effects image is dropped by the engine as soon as the upload has copied it.
       if (fx) engine.drawPixels(state.id, layer.id, level, edit, upload);
       else upload(width === 0 ? null : engine.layerPixels(state.id, layer.id, level));
+      if (fx) this.shown.set(layer.id, { width: layer.pixelsWidth, height: layer.pixelsHeight, inset: fx.inset });
     }
     this.textures.retainOnly(state.id, keep);
+    this.effectsImages.retainOnly(state.id, keep);
+    for (const id of [...this.placements.keys()]) if (!keep.has(id)) this.placements.delete(id);
+    for (const id of [...this.shown.keys()]) if (!keep.has(id)) this.shown.delete(id);
+  }
+
+  /** A styled layer too large to make its effects image on the UI thread (EFFECTS_LIMITS.sync). True
+   * when the engine has the full-size image, which the usual path then draws; otherwise this draws
+   * the worker's reduced image, or keeps the image already on the texture, or draws the layer's own
+   * pixels, and says where (`placements`), while the worker makes what is missing. */
+  private largeEffects(engine: EngineClient, state: DocumentState, id: string, bytesKey: string, draw: LayerDraw, nearest: boolean, outPerDoc: number, edit: PreviewEdit | null): boolean {
+    const layer = state.layers.find((l) => l.id === id)!;
+    const choice = this.effectsImages.choose(engine, state.id, id, bytesKey, draw, layer.pixelsWidth, layer.pixelsHeight, edit);
+    if (choice === "full") return true;
+    if (choice) {
+      const key = `rd:${choice.key}:${choice.width}`;
+      this.placements.set(id, placedLike(draw, choice.width - 2 * choice.inset, choice.height - 2 * choice.inset, choice.inset));
+      this.shown.set(id, { width: choice.width - 2 * choice.inset, height: choice.height - 2 * choice.inset, inset: choice.inset });
+      if (this.textures.needsUpload(state.id, id, key, 0, nearest)) this.textures.sync(state.id, id, key, nearest, choice.bytes, 0, { width: choice.width, height: choice.height });
+      return false;
+    }
+    // Nothing for these pixels yet: the image on the texture stays, where the layer is now.
+    const shown = this.shown.get(id);
+    if (shown && this.textures.get(state.id, id)) { this.placements.set(id, placedLike(draw, shown.width, shown.height, shown.inset)); return false; }
+    // No image at all yet: the layer's own pixels, plainly.
+    const plain = placedLike(draw, layer.pixelsWidth, layer.pixelsHeight, 0);
+    this.placements.set(id, plain);
+    const plainLevel = nearest || plain.corners ? 0 : prefilterLevel(layer.pixelsWidth, layer.pixelsHeight, layer.pixelsWidth / Math.max(1e-9, plain.transform.size[0] * outPerDoc));
+    const key = `px:${layer.pixelsRevision}`;
+    if (this.textures.needsUpload(state.id, id, key, plainLevel, nearest)) {
+      this.textures.sync(state.id, id, key, nearest, engine.layerPixels(state.id, id, plainLevel), plainLevel, sizeAtLevel(layer.pixelsWidth, layer.pixelsHeight, plainLevel), layer.pixelsRevision);
+    }
+    return false;
   }
 
   render(engine: EngineClient, state: DocumentState, viewport: Viewport, dpr: number, options: RenderOptions, edit: PreviewEdit | null): void {
@@ -428,8 +472,11 @@ export class GlRenderer implements Renderer {
 
   /** Draws every chunk of a layer texture into `target` reading `backdrop`, or onto a cleared
    * target when `backdrop` is null. */
-  private drawLayer(ctx: Ctx, target: string, backdrop: WebGLTexture | null, draw: LayerDraw, mode: number, coverageLevel: number | null): void {
-    const t = this.textures.get(ctx.state.id, draw.id); if (!t) return;
+  private drawLayer(ctx: Ctx, target: string, backdrop: WebGLTexture | null, planned: LayerDraw, mode: number, coverageLevel: number | null): void {
+    const t = this.textures.get(ctx.state.id, planned.id); if (!t) return;
+    // A large styled layer's texture may not be the plan's full-size image (largeEffects).
+    const placed = this.placements.get(planned.id);
+    const draw = placed ? { ...planned, transform: placed.transform, corners: placed.corners } : planned;
     const d2v = this.docToView(ctx.viewport, ctx.state);
     const cornersDoc = draw.corners ? draw.corners.map(fromTuple) : cornersOf(draw.transform);
     const cornersView = cornersDoc.map((p) => ({ x: d2v[0] * p.x + d2v[1] * p.y + d2v[2], y: d2v[3] * p.x + d2v[4] * p.y + d2v[5] }));
@@ -487,6 +534,10 @@ export class GlRenderer implements Renderer {
     gl.disable(gl.BLEND); gl.disable(gl.SCISSOR_TEST);
   }
 
+  /** What a layer's texture holds, by its key ("px:" its pixels, "fx:" the effects image, "rd:" a
+   * reduced one): the perf harness and e2e tests watch a large styled layer's images arrive. */
+  textureKey(docId: string, id: string): string | null { return this.textures.get(docId, id)?.key ?? null; }
+
   readPixels(): Uint8Array {
     const gl = this.gl; const W = this.canvas.width, H = this.canvas.height;
     gl.bindFramebuffer(gl.FRAMEBUFFER, null);
```

```diff
--- a/app/src/canvas/renderer.ts
+++ b/app/src/canvas/renderer.ts
@@ -1,10 +1,14 @@
 import type { DocumentState, PreviewEdit } from "../engine/types";
 import type { EngineClient } from "../engine/client";
+import type { JobClient } from "../engine/jobs";
 import type { Viewport } from "./viewport";
 import { GlRenderer } from "./gl-renderer";
 import { CpuRenderer } from "./cpu-renderer";
 
 export interface RenderOptions { checkerboard: boolean; }
+/** What a renderer may use besides the engine: the job worker (for large styled layers' effects
+ * images, effects-images.ts) and a way to ask for another frame when one of its results lands. */
+export interface RenderHooks { jobs: () => JobClient | null; landed: () => void; }
 export interface Renderer {
   readonly kind: "gl" | "cpu";
   /** Uploads whatever it needs and draws. There is no separate sync step: the GL renderer's
@@ -15,9 +19,9 @@ export interface Renderer {
   dispose(): void;
 }
 
-export function createRenderer(canvas: HTMLCanvasElement): Renderer {
+export function createRenderer(canvas: HTMLCanvasElement, hooks?: RenderHooks): Renderer {
   const gl = canvas.getContext("webgl2", { premultipliedAlpha: true, preserveDrawingBuffer: true, antialias: false });
-  if (gl) return new GlRenderer(canvas, gl);
+  if (gl) return new GlRenderer(canvas, gl, hooks);
   return new CpuRenderer(canvas);
 }
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -126,6 +126,17 @@ export class EngineClient {
     const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
     return JSON.parse(this.wasm.install_job(doc, layer, stamp, output, pixels ? new Uint8Array(pixels) : undefined, mask ? new Uint8Array(mask) : undefined)) as Dirty;
   }
+  /** Whether the canvas's effects image for `layer` is made already (engine `has_effects_image`):
+   * `drawPixels` then hands it over without making it. */
+  hasEffectsImage(doc: string, layer: string, edit: PreviewEdit | null): boolean {
+    return this.wasm.has_effects_image(doc, layer, edit ? JSON.stringify(edit) : undefined);
+  }
+  /** Keeps a full-size effects image the job worker made (engine `keep_effects_image`), when the layer
+   * is still what the job took (the stamp in `input`); false when it is not kept. */
+  keepEffectsImage(doc: string, layer: string, input: string, edit: PreviewEdit | null, width: number, height: number, bytes: ArrayBuffer): boolean {
+    const stamp = JSON.stringify((JSON.parse(input) as { stamp: unknown }).stamp);
+    return this.wasm.keep_effects_image(doc, layer, stamp, edit ? JSON.stringify(edit) : undefined, width, height, new Uint8Array(bytes));
+  }
   /** What changed in the layer's pixels since revision `from` (engine `pixels_delta`): a rectangle of
    * its pixel grid, empty when nothing did, or null when the whole raster must be uploaded again. */
   pixelsDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.pixels_delta(doc, layer, from)); }
```

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -72,6 +72,19 @@ impl WasmEngine {
         let dirty = self.engine.install_job(parse_id(doc)?, parse_id(layer)?, stamp, output, pixels, mask).map_err(js_err)?;
         serde_json::to_string(&dirty).map_err(js_err)
     }
+    /// `Engine::has_effects_image`: whether the canvas's effects image for the layer is made already.
+    pub fn has_effects_image(&self, doc: &str, layer: &str, edit_json: Option<String>) -> Result<bool, JsError> {
+        let edit = Self::parse_edit(edit_json)?;
+        self.engine.has_effects_image(parse_id(doc)?, parse_id(layer)?, edit.as_ref()).map_err(js_err)
+    }
+    /// `Engine::keep_effects_image`: a full-size effects image from the worker, kept in the engine's
+    /// cache when the layer is still what the job took (`stamp_json`).
+    pub fn keep_effects_image(&self, doc: &str, layer: &str, stamp_json: &str, edit_json: Option<String>, width: u32, height: u32, bytes: Vec<u8>) -> Result<bool, JsError> {
+        let stamp: LayerStamp = serde_json::from_str(stamp_json).map_err(js_err)?;
+        let edit = Self::parse_edit(edit_json)?;
+        let image = raster_of(Some((width, height)), Some(bytes))?.unwrap();
+        self.engine.keep_effects_image(parse_id(doc)?, parse_id(layer)?, stamp, edit.as_ref(), image).map_err(js_err)
+    }
     /// `run_edit_job` (in the worker): the output as JSON; the buffers it replaced are kept.
     pub fn run_edit_job(&mut self, input_json: &str, pixels: Option<Vec<u8>>, mask: Option<Vec<u8>>, command_json: &str) -> Result<String, JsError> {
         let input: JobInput = serde_json::from_str(input_json).map_err(js_err)?;
```

```diff
--- a/engine/src/effects/mod.rs
+++ b/engine/src/effects/mod.rs
@@ -210,6 +210,26 @@ impl EffectsCache {
         }
         Some(image)
     }
+    /// Whether an image for these very buffers and this draw is kept (nothing is made).
+    pub fn contains(&self, layer: &Layer, draw: &EffectsDraw) -> bool {
+        let Some(pixels) = layer.pixels.as_ref() else { return false };
+        let mask = draw.mask.as_ref().and(layer.mask.as_ref()).map(|m| &m.pixels);
+        self.kept().entries.iter().any(|e| e.pixels.same_pixels(pixels) && e.draw == *draw
+            && match (&e.mask, mask) { (None, None) => true, (Some(a), Some(b)) => a.same_pixels(b), _ => false })
+    }
+    /// Keeps `image`, made elsewhere (the job worker, from the same layer and draw), as `image` would
+    /// have kept it: found by the same buffers and draw from now on. Not kept when it is over the
+    /// byte limit, as a made image would not be.
+    pub fn insert(&self, layer: &Layer, draw: &EffectsDraw, image: Raster) {
+        let Some(pixels) = layer.pixels.as_ref() else { return };
+        if image.bytes().len() > self.max_bytes || self.contains(layer, draw) { return; }
+        let mask = draw.mask.as_ref().and(layer.mask.as_ref()).map(|m| m.pixels.clone());
+        let mut kept = self.kept();
+        kept.entries.push(Entry { pixels: pixels.clone(), mask, draw: draw.clone(), image });
+        while kept.entries.len() > self.max_entries || kept.entries.iter().map(|e| e.image.bytes().len()).sum::<usize>() > self.max_bytes {
+            kept.entries.remove(0);
+        }
+    }
     /// Drops the images no one can find again: those whose pixel or mask buffer only the cache
     /// holds (a closed document, an ended preview, a history entry let go). They would otherwise
     /// keep their images and buffers alive and push out images still in use. A buffer counts as
```

```diff
--- a/engine/src/jobs.rs
+++ b/engine/src/jobs.rs
@@ -105,6 +105,27 @@ impl Engine {
         Ok((input, pixels, l.mask.as_ref().map(|m| m.pixels.clone())))
     }
 
+    /// Whether the canvas's effects image for `layer` (with the pending `edit`) is already made: the
+    /// renderer then draws it at once (`draw_raster` finds it); otherwise it asks the job worker.
+    pub fn has_effects_image(&self, id: Uuid, layer: Uuid, edit: Option<&PreviewEdit>) -> Result<bool, CommandError> {
+        let doc = self.render_document(id)?;
+        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
+        Ok(effects_draw(l, edit).is_some_and(|draw| self.effects_cache().contains(l, &draw)))
+    }
+    /// Keeps an effects image the job worker made at full size for `layer` as the canvas showed it
+    /// (with `edit`), in the engine's cache, where `draw_raster` finds it as if made here. Only while
+    /// the layer is still what the job took (`stamp`), draws effects, and needs an image of this size:
+    /// otherwise nothing is kept and false comes back.
+    pub fn keep_effects_image(&self, id: Uuid, layer: Uuid, stamp: LayerStamp, edit: Option<&PreviewEdit>, image: Raster) -> Result<bool, CommandError> {
+        let doc = self.render_document(id)?;
+        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
+        if LayerStamp::of(l) != stamp { return Ok(false); }
+        let (Some(draw), Some(pixels)) = (effects_draw(l, edit), l.pixels.as_ref()) else { return Ok(false) };
+        if (image.width, image.height) != (pixels.width + 2 * draw.inset, pixels.height + 2 * draw.inset) { return Ok(false); }
+        self.effects_cache().insert(l, &draw, image);
+        Ok(true)
+    }
+
     /// Puts an edit job's result back on `layer` as one undo step: the buffers it replaced, its
     /// transform and mask placement, recorded as changed within its regions. Refused, with the
     /// document untouched, unless the layer is still exactly what the job took (`stamp`).
```


- [ ] **Step 4: Run the tests and watch them pass**

Engine: 481 passed, 6 ignored (+2). `pnpm wasm:dev`; `pnpm test`: 167 (+5); `pnpm build`; `pnpm e2e`: 137 passed, 8 skipped. Timings (release wasm, Edge, `-g "effects images"`): a 24 MP layer with all six effects: the first frame draws it plainly in 21 ms, the reduced image is drawn after 2.1 s (longest frame gap 33 ms, budget 100), the full image kept after 14.6 s (gaps under 83 ms, budget 150) and drawn in a 16 ms frame; at 100 MP the reduced image after 1.7 s (longest gap 167 ms, budget 200: the first frame halves the plain 100 MP layer three times; measured 133-167 ms over three runs) and no full image (past 24 MP).

- [ ] **Step 5: Prove it bites**

(1) Set `EFFECTS_LIMITS.reduced` to 2048: `reducedLevel`'s test fails (measured). Restore. (2) In `keep_effects_image`, skip the stamp comparison: `an_effects_image_is_not_kept_for_a_layer_that_changed_or_at_another_size` fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/canvas/effects-images.ts app/tests/e2e/effects-worker.spec.ts app/tests/unit/effects-images.test.ts
git commit -m "feat: large layers' effects images are made by the worker, reduced first, then at full size" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/CanvasView.tsx app/src/canvas/effects-images.ts app/src/canvas/gl-renderer.ts app/src/canvas/renderer.ts app/src/engine/client.ts app/tests/e2e/effects-worker.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/effects-images.test.ts engine-wasm/src/lib.rs engine/src/effects/mod.rs engine/src/jobs.rs engine/tests/jobs.rs
```


---

### Task 8: Fill and the Gradient, painted as the Mac paints them

The engine gains the Mac's two raster edits (ruling OQ7): `Command::Fill { id, mask, color }` ("Fill", "Fill Mask") and `Command::Gradient { id, mask, gradient }` ("Gradient", "Gradient Mask"). Both paint through `paint_layer`: the layer's pixel grid grows to cover the canvas (`image_grid`), each pixel is painted at its centre inside the canvas, source-over at the paint's alpha times the opacity times the selection's coverage, rounded half up; the result is trimmed to the pixels left (`alpha_bounds`, now row by row), and a mask that covers the layer follows it, white where it grew. A mask is painted in its own grid, in grey. Both report the selection's rectangle when the layer kept its grid, so Task 3's partial path applies.

**Files:**
- Create: `engine/src/ops/raster_edit.rs`
- Modify: `engine/src/command.rs`, `engine/src/engine.rs`, `engine/src/lib.rs`, `engine/src/ops/mod.rs`, `engine/src/ops/adjust.rs` (`placed_like` shared), `engine/src/compositor.rs` (`alpha_bounds`)
- Create tests: `engine/tests/raster_edits.rs`; modify `engine/tests/perf_4b1.rs`

**Interfaces:**
- Produces: `GradientShape { Linear, Radial }`, `GradientSpec { shape, start, end, from, to, opacity }` (JSON: `start` and `end` as `[x, y]`, stops as straight RGBA 0..1), `MIN_GRADIENT_LINE = 0.5`, `Paint { Fill, Gradient }`, `EditGrid`, `image_grid`, `pixels_on`, `paint_grid`, `paint_layer`.

- [ ] **Step 1: Write the tests**

Ported from the Mac's GradientTests with their numbers, and the formulas: Foreground to Background fills the canvas and commits one undo step (black to white from x 0.5 to 100.5 on a blank 101 x 4 layer: column x is `round(255 x / 100)`, the middle 128 because 127.5 rounds up; undo leaves the layer blank); a radial one spreads from the start to the rim in every direction (black at the start, 128 at 20 px of a 40 px radius on four sides, white past the rim); Reverse and Opacity; Foreground to Transparent keeps the pixels under it and their alpha; a mask gradient writes coverage inside the layer; a gradient stays inside the selection; Fill takes the colour inside the selection, or the whole layer without one; a mask fill hides only the selected area; an empty selection and a line under half a pixel paint nothing; a layer smaller than the canvas grows to it, is trimmed, and its mask follows with white; a fill inside a selection on a layer over the canvas is recorded as that rectangle.

```diff
--- a/engine/tests/perf_4b1.rs
+++ b/engine/tests/perf_4b1.rs
@@ -43,6 +43,25 @@ fn halving_and_a_clear_in_a_selection_at_24_and_100_mp() {
     }
 }
 
+#[test]
+#[ignore]
+fn a_gradient_and_a_fill_at_24_and_100_mp() {
+    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
+        let (mut e, id, layer) = filled(w, h);
+        let gradient = GradientSpec { shape: GradientShape::Linear, start: Point { x: 0.0, y: 0.0 }, end: Point { x: w as f64, y: h as f64 },
+            from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 0.0], opacity: 0.8 };
+        let t = Instant::now();
+        run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient });
+        let whole = ms(t);
+        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Ellipse, points: vec![Point { x: 500.0, y: 500.0 }, Point { x: 2500.0, y: 500.0 }, Point { x: 2500.0, y: 2000.0 }, Point { x: 500.0, y: 2000.0 }], mode: SelectionMode::Replace, antialiased: true });
+        let t = Instant::now();
+        run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
+        let fill = ms(t);
+        println!("{label}: gradient over the whole layer {whole:.0} ms; fill in a 2000 x 1500 ellipse {fill:.0} ms");
+        assert_eq!(e.state(id).unwrap().layers[0].pixels_width, w);
+    }
+}
+
 #[test]
 #[ignore]
 fn a_rename_at_the_history_cap_with_a_thousand_layers() {
```

Create `engine/tests/raster_edits.rs`:

```rust
//! Fill and the Gradient (Phase 4b-1): the Mac's GradientTests and the fill cases of its
//! SelectionEditTests, with their numbers, plus the layer's growth to the canvas, its trim and the
//! mask that follows (BrushStroke.swift:153-160, EditorSession+Brush.swift:154-188).
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }

/// A `width` x `height` document with one blank layer, active (`createDocument(emptyLayer: true)`).
fn blank(width: u32, height: u32) -> (Engine, Uuid, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(width, height, true).unwrap();
    let layer = e.state(id).unwrap().layers[0].id;
    (e, id, layer)
}
/// The composite's premultiplied RGBA at one pixel, as the Mac's tests read the exported image.
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] {
    e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0)
}
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
fn linear(start: Point, end: Point, from: [f64; 4], to: [f64; 4], opacity: f64) -> GradientSpec {
    GradientSpec { shape: GradientShape::Linear, start, end, from, to, opacity }
}
/// The ramp's value at pixel column `x` for a line from x 0.5 to 100.5: the pixel's centre, x + 0.5,
/// is (x + 0.5 - 0.5) / 100 of the way along, rounded to 8 bits.
fn ramp(x: u32) -> u8 { (255.0 * x as f64 / 100.0).round() as u8 }

#[test]
fn foreground_to_background_fills_the_canvas_and_commits_one_undo() {
    // GradientTests.foregroundToBackgroundFillsCanvasAndCommitsOneUndo (:37).
    let (mut e, id, layer) = blank(101, 4);
    let count = depth(&e, id);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, WHITE, 1.0) });
    assert_eq!(depth(&e, id), count + 1);
    for x in [0, 1, 37, 50, 99, 100] {
        let v = ramp(x);
        assert_eq!(pixel(&e, id, x, 1 + x % 3), [v, v, v, 255], "column {x}");
    }
    assert_eq!(pixel(&e, id, 50, 1)[0], 128, "the middle is 128 (127.5 rounds up)");
    e.undo(id).unwrap();
    assert!(e.state(id).unwrap().layers[0].has_pixels == false, "undo leaves the blank layer blank");
}

#[test]
fn radial_spreads_from_the_start_to_the_rim_in_every_direction() {
    // GradientTests.radialSpreadsFromStartToRimInEveryDirection (:55).
    let (mut e, id, layer) = blank(101, 101);
    let g = GradientSpec { shape: GradientShape::Radial, start: p(50.5, 50.5), end: p(90.5, 50.5), from: BLACK, to: WHITE, opacity: 1.0 };
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: g });
    assert_eq!(pixel(&e, id, 50, 50), [0, 0, 0, 255]);
    // 20 px out of a 40 px radius, in four directions: 127.5, rounded up.
    for (x, y) in [(70, 50), (30, 50), (50, 70), (50, 30)] { assert_eq!(pixel(&e, id, x, y), [128, 128, 128, 255], "({x}, {y})"); }
    assert_eq!(pixel(&e, id, 100, 50), [255, 255, 255, 255], "past the rim");
    assert_eq!(pixel(&e, id, 0, 0), [255, 255, 255, 255]);
}

#[test]
fn reverse_and_opacity_follow_the_settings() {
    // GradientTests.reverseOpacityAndDirectionFollowSettings (:69): reversed (white first) at 50 %.
    let (mut e, id, layer) = blank(101, 4);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), WHITE, BLACK, 0.5) });
    // White at half alpha over nothing: premultiplied 127.5, rounded up, on both.
    assert_eq!(pixel(&e, id, 0, 2), [128, 128, 128, 128]);
    assert_eq!(pixel(&e, id, 100, 2), [0, 0, 0, 128]);
}

#[test]
fn foreground_to_transparent_keeps_the_pixels_under_it_and_their_alpha() {
    // GradientTests.foregroundToTransparentPreservesUnderlyingPixelsAndAlpha (:83).
    let (mut e, id, layer) = blank(101, 4);
    let red = [1.0, 0.0, 0.0, 1.0];
    // Everything before the start is the first colour: a line at the far right makes a red layer.
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(100.5, 2.0), p(101.0, 2.0), red, WHITE, 1.0) });
    assert_eq!(pixel(&e, id, 50, 2), [255, 0, 0, 255]);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, [0.0, 0.0, 0.0, 0.0], 1.0) });
    assert_eq!(pixel(&e, id, 0, 2), [0, 0, 0, 255]);
    assert_eq!(pixel(&e, id, 100, 2), [255, 0, 0, 255]);
    // Black at half alpha over red: 255 x 0.5 = 127.5, rounded up; alpha stays whole.
    assert_eq!(pixel(&e, id, 50, 2), [128, 0, 0, 255]);
}

#[test]
fn a_mask_gradient_writes_coverage_inside_the_layer() {
    // GradientTests.maskGradientWritesCoverageInsideLayerBounds (:126): an opaque black layer, a
    // revealing mask, and black (hide) to white (reveal) across it.
    let (mut e, id, layer) = blank(101, 4);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.0, 0.0, 0.0] });
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    let count = depth(&e, id);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 2.0), p(100.5, 2.0), BLACK, WHITE, 1.0) });
    assert_eq!(depth(&e, id), count + 1);
    for x in [0, 50, 100] { assert_eq!(pixel(&e, id, x, 2)[3], ramp(x), "column {x}"); }
    let mask = e.document(id).unwrap().layers[0].mask.clone().unwrap();
    assert_eq!((mask.pixels.width, mask.pixels.height), (101, 4), "the 1 x 1 mask spread over the layer's grid first");
}

#[test]
fn a_gradient_stays_inside_the_selection() {
    // SelectionEditTests.gradientStaysInsideTheSelection (:64).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 0.0, 0.0, 50.0, 40.0);
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: linear(p(0.5, 20.0), p(99.5, 20.0), BLACK, WHITE, 1.0) });
    assert_eq!(pixel(&e, id, 10, 20)[3], 255);
    assert_eq!(pixel(&e, id, 75, 20)[3], 0);
    // Trimmed to what the selection let through.
    let state = e.state(id).unwrap();
    assert_eq!((state.layers[0].pixels_width, state.layers[0].pixels_height), (50, 40));
}

#[test]
fn fill_uses_the_colour_inside_the_selection_or_the_whole_layer_without_one() {
    // SelectionEditTests.fillUsesPaletteInsideSelectionOrWholeLayerWithoutOne (:77).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    let count = depth(&e, id);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    assert_eq!(depth(&e, id), count + 1);
    assert_eq!(Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] }.action_name(), "Fill");
    assert_eq!(pixel(&e, id, 30, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&e, id, 5, 5)[3], 0);
    run(&mut e, id, Command::Deselect);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 1.0, 1.0] });
    assert_eq!(pixel(&e, id, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel(&e, id, 30, 20), [255, 255, 255, 255]);
}

#[test]
fn a_mask_fill_hides_only_the_selected_area() {
    // SelectionEditTests.maskFillHidesOnlyTheSelectedArea (:111): the mask palette's foreground is
    // black (hide), its background white (reveal).
    let (mut e, id, layer) = blank(100, 40);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    assert_eq!(Command::Fill { id: layer, mask: true, color: [0.0; 3] }.action_name(), "Fill Mask");
    assert_eq!(pixel(&e, id, 30, 20)[3], 0);
    assert_eq!(pixel(&e, id, 5, 5)[3], 255);
    run(&mut e, id, Command::ClearSelectedPixels { id: layer, mask: true });
    assert_eq!(pixel(&e, id, 30, 20)[3], 255);
}

#[test]
fn an_empty_selection_fills_and_paints_nothing() {
    // SelectionEditTests.emptySelectionEditsNothing (:43).
    let (mut e, id, layer) = blank(100, 40);
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 40.0), p(0.0, 40.0)], mode: SelectionMode::Subtract, antialiased: true });
    assert!(e.state(id).unwrap().selection.unwrap().empty);
    let (before, count) = (e.document(id).unwrap().clone(), depth(&e, id));
    for c in [Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] },
              Command::Gradient { id: layer, mask: false, gradient: linear(p(0.0, 20.0), p(90.0, 20.0), BLACK, WHITE, 1.0) }] {
        assert_eq!(e.execute(id, c), Err(CommandError::Refused("The selection is empty".into())));
    }
    assert!(e.document(id).unwrap().same_content(&before));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn a_gradient_line_under_half_a_pixel_paints_nothing() {
    // Gradient.swift:32, :84-87: a click without a line leaves nothing.
    let (mut e, id, layer) = blank(100, 40);
    let count = depth(&e, id);
    let short = linear(p(10.0, 10.0), p(10.3, 10.3), BLACK, WHITE, 1.0);
    assert!(matches!(e.execute(id, Command::Gradient { id: layer, mask: false, gradient: short }), Err(CommandError::Argument(_))));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn a_layer_smaller_than_the_canvas_grows_to_it_is_trimmed_and_its_mask_follows_with_white() {
    // A 20 x 10 layer at (30, 15) on 100 x 40, under a mask of its own grid: the left half black.
    let mut e = Engine::new();
    let id = e.new_document(100, 40, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(20, 10, [0, 0, 200, 255].repeat(200)), p(30.0, 15.0));
    let mask: Vec<u8> = (0..10).flat_map(|_| (0..20).map(|x| if x < 10 { 0 } else { 255 })).collect();
    layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(20, 10, mask), enabled: true, placement: None, linked: None });
    let lid = layer.id;
    doc.active_layer_id = Some(lid);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    // A selection reaching past the layer on every side but the bottom.
    select(&mut e, id, 10.0, 5.0, 80.0, 20.0);
    run(&mut e, id, Command::Fill { id: lid, mask: false, color: [1.0, 0.0, 0.0] });
    let l = e.document(id).unwrap().layers[0].clone();
    // Trimmed to the union of the old pixels (30..50 x 15..25) and the filled area (10..90 x 5..25).
    assert_eq!((l.transform.origin.x, l.transform.origin.y, l.transform.size.width, l.transform.size.height), (10.0, 5.0, 80.0, 20.0));
    let pixels = l.pixels.unwrap();
    assert_eq!((pixels.width, pixels.height), (80, 20));
    assert_eq!(pixels.pixel(0, 0), [255, 0, 0, 255], "filled where it grew");
    let m = l.mask.unwrap().pixels;
    assert_eq!((m.width, m.height), (80, 20), "the mask follows the new grid");
    let at = |x: u32, y: u32| m.bytes()[(y * 80 + x) as usize];
    // The old mask where the layer was (its left half black), white where the layer grew.
    assert_eq!((at(25, 12), at(35, 12), at(5, 12), at(70, 2)), (0, 255, 255, 255));
}

#[test]
fn a_fill_inside_a_selection_on_a_layer_over_the_canvas_is_recorded_as_that_rectangle() {
    let (mut e, id, layer) = blank(100, 40);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.2, 0.4, 0.6] });
    let before = e.state(id).unwrap().layers[0].pixels_revision;
    select(&mut e, id, 20.0, 10.0, 30.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [1.0, 0.0, 0.0] });
    // The selection's box, a pixel for its clip and one for sampling on every side.
    assert_eq!(e.pixels_delta(id, layer, before).unwrap(), Some(PixelRect { x: 18, y: 8, width: 34, height: 24 }));
}
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test raster_edits`: does not compile (`Command::Fill`, `GradientSpec`).

- [ ] **Step 3: Implement**

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -1,4 +1,4 @@
-use crate::{ids, AdjustmentKind, BlendMode, FilterParams, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape, WandSettings};
+use crate::{ids, AdjustmentKind, BlendMode, FilterParams, GradientSpec, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape, WandSettings};
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
@@ -67,6 +67,11 @@ pub enum Command {
     ClearSelectedPixels { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool },
     /// Add Mask with a selection: `revealing` white with the selection black, or the reverse.
     AddMaskFromSelection { #[serde(with = "ids::upper")] id: Uuid, revealing: bool },
+    // Colour and fills (Phase 4b-1).
+    /// Fill the selection (or the whole layer) with `color`; on the mask, its first channel is grey.
+    Fill { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool, color: [f64; 3] },
+    /// Paint a gradient over the layer's pixels or its mask, inside the selection.
+    Gradient { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool, gradient: GradientSpec },
 }
 
 impl Command {
@@ -134,6 +139,11 @@ impl Command {
             Command::ClearSelectedPixels { mask: false, .. } => "Clear",
             Command::ClearSelectedPixels { mask: true, .. } => "Fill Mask",
             Command::AddMaskFromSelection { .. } => "Add Mask from Selection",
+            // SelectionEdits.swift:34, Gradient.swift:98.
+            Command::Fill { mask: false, .. } => "Fill",
+            Command::Fill { mask: true, .. } => "Fill Mask",
+            Command::Gradient { mask: false, .. } => "Gradient",
+            Command::Gradient { mask: true, .. } => "Gradient Mask",
         }
     }
 }
```

```diff
--- a/engine/src/compositor.rs
+++ b/engine/src/compositor.rs
@@ -60,9 +60,13 @@ fn prefiltered(raster: &Raster, pixels_per_output: f64) -> (Raster, f64) {
 /// (x0, y0, x1, y1) of the pixels with alpha > 0, x1/y1 exclusive; None when fully transparent.
 pub fn alpha_bounds(r: &Raster) -> Option<(u32, u32, u32, u32)> {
     let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
-    for y in 0..r.height { for x in 0..r.width {
-        if r.pixel(x, y)[3] > 0 { x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x + 1); y1 = y1.max(y + 1); }
-    }}
+    // A row at a time over its alpha bytes: the first and last pixel with any alpha.
+    for (y, row) in r.bytes().chunks_exact(r.width as usize * 4).enumerate() {
+        let Some(first) = row.chunks_exact(4).position(|p| p[3] > 0) else { continue };
+        let last = row.chunks_exact(4).rposition(|p| p[3] > 0).unwrap();
+        x0 = x0.min(first as u32); x1 = x1.max(last as u32 + 1);
+        y0 = y0.min(y as u32); y1 = y1.max(y as u32 + 1);
+    }
     if x1 == 0 { None } else { Some((x0, y0, x1, y1)) }
 }
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -461,6 +461,16 @@ impl Engine {
                 Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
             }
             Command::AddMaskFromSelection { id, revealing } => { ops::selection::add_mask_from_selection(doc, clips, id, revealing)?; Ok(Dirty::structure()) }
+            Command::Fill { id, mask, color } => {
+                ops::raster_edit::paint_layer(doc, clips, id, mask, &ops::raster_edit::Paint::Fill(color))?;
+                let plane = if mask { Plane::Mask } else { Plane::Pixels };
+                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
+            }
+            Command::Gradient { id, mask, gradient } => {
+                ops::raster_edit::paint_layer(doc, clips, id, mask, &ops::raster_edit::Paint::Gradient(gradient))?;
+                let plane = if mask { Plane::Mask } else { Plane::Pixels };
+                Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
+            }
         })
     }
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -50,6 +50,7 @@ pub use effects::render::*;
 pub use lineage::*;
 pub use jobs::*;
 pub use ops::masks::blur_gray;
+pub use ops::raster_edit::{GradientShape, GradientSpec, Paint, MIN_GRADIENT_LINE};
 pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
 pub use selection::coverage::{rasterize, selection_coverage, selection_coverage_with, SelectionClip, SelectionClips};
 pub use selection::feather::{feather_blur, DIRECT_SIGMA_LIMIT};
```

```diff
--- a/engine/src/ops/adjust.rs
+++ b/engine/src/ops/adjust.rs
@@ -94,7 +94,7 @@ pub fn grown(raster: &Raster, transform: &LayerTransform, margin: f64) -> Option
 
 /// The transform for a `new_w` x `new_h` grid whose old grid sits at (`offset_x`, `offset_y`),
 /// keeping every old pixel over the same document point.
-fn placed_like(transform: &LayerTransform, old_w: u32, old_h: u32, new_w: u32, new_h: u32, offset_x: f64, offset_y: f64) -> LayerTransform {
+pub(crate) fn placed_like(transform: &LayerTransform, old_w: u32, old_h: u32, new_w: u32, new_h: u32, offset_x: f64, offset_y: f64) -> LayerTransform {
     let sx = transform.size.width / old_w as f64;
     let sy = transform.size.height / old_h as f64;
     let mut result = *transform;
```

```diff
--- a/engine/src/ops/mod.rs
+++ b/engine/src/ops/mod.rs
@@ -10,3 +10,4 @@ pub mod distort;
 pub mod masks;
 pub mod merge;
 pub mod selection;
+pub mod raster_edit;
```

Create `engine/src/ops/raster_edit.rs`:

```rust
//! Raster edits (Phase 4b-1): Fill and the Gradient, painted as Compositor for Mac paints them
//! (`BrushStroke.paintCanvas`, BrushStroke.swift:645-675): over the layer's original pixels, clipped
//! to the canvas and the selection, at the edit's opacity (source-over, the colour's alpha times the
//! opacity times the selection's coverage). A layer's pixel grid first grows to cover the canvas
//! (`BrushStroke.init`, :153-160); a mask keeps its own grid. The layer's result is trimmed to the
//! pixels left, and a mask covering it follows, white where the layer grew (`commitRasterEdit`,
//! EditorSession+Brush.swift:154-188; `expandMask`, BrushStroke.swift:858-866).
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Linear runs along the line; Radial spreads from the start with the end on its rim (Gradient.swift:9-13).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientShape { Linear, Radial }

/// A gradient to paint: its shape, its line in document pixels, its two stops as straight RGBA 0..1
/// (a mask reads the first channel as grey), and the edit's opacity, 0.01 to 1 (Gradient.swift:15-19).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradientSpec { pub shape: GradientShape, pub start: Point, pub end: Point, pub from: [f64; 4], pub to: [f64; 4], pub opacity: f64 }

/// The shortest gradient line that paints anything (`GradientEdit.hasLine`, Gradient.swift:32).
pub const MIN_GRADIENT_LINE: f64 = 0.5;

impl GradientSpec {
    fn check(&self) -> Result<(), CommandError> {
        let finite = [self.start.x, self.start.y, self.end.x, self.end.y, self.opacity].iter().chain(&self.from).chain(&self.to).all(|v| v.is_finite());
        if !finite || !(0.01..=1.0).contains(&self.opacity) || self.from.iter().chain(&self.to).any(|v| !(0.0..=1.0).contains(v)) {
            return Err(CommandError::Argument("gradient settings out of range".into()));
        }
        if (self.end.x - self.start.x).hypot(self.end.y - self.start.y) < MIN_GRADIENT_LINE {
            return Err(CommandError::Argument("a gradient needs a line at least half a pixel long".into()));
        }
        Ok(())
    }
    /// Where `p` falls along the gradient, 0 at the start and 1 at the end, extended past both
    /// (`drawsBeforeStartLocation`, `drawsAfterEndLocation`, BrushStroke.swift:611-621).
    pub fn position(&self, p: Point) -> f64 {
        let (dx, dy) = (self.end.x - self.start.x, self.end.y - self.start.y);
        let t = match self.shape {
            GradientShape::Linear => ((p.x - self.start.x) * dx + (p.y - self.start.y) * dy) / (dx * dx + dy * dy),
            GradientShape::Radial => (p.x - self.start.x).hypot(p.y - self.start.y) / dx.hypot(dy),
        };
        t.clamp(0.0, 1.0)
    }
    /// The straight RGBA at position `t`: the two stops mixed.
    pub fn color_at(&self, t: f64) -> [f64; 4] { std::array::from_fn(|c| self.from[c] + (self.to[c] - self.from[c]) * t) }
}

/// What an edit paints: a solid colour (Fill), or a gradient.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint { Fill([f64; 3]), Gradient(GradientSpec) }

impl Paint {
    fn check(&self) -> Result<(), CommandError> {
        match self {
            Paint::Fill(c) if c.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) => Ok(()),
            Paint::Fill(_) => Err(CommandError::Argument("fill colour out of range".into())),
            Paint::Gradient(g) => g.check(),
        }
    }
    /// The straight RGBA painted at document point `p`, and the opacity it is painted at.
    fn at(&self, p: Point) -> ([f64; 4], f64) {
        match self {
            Paint::Fill(c) => ([c[0], c[1], c[2], 1.0], 1.0),
            Paint::Gradient(g) => (g.color_at(g.position(p)), g.opacity),
        }
    }
}

/// The grid a layer's edit is painted on: `width` x `height` pixels that `transform` places, with the
/// layer's own pixels at (`x`, `y`) in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditGrid { pub width: u32, pub height: u32, pub x: u32, pub y: u32, pub transform: LayerTransform }

/// The grid for painting `layer`'s pixels: its own grid (its pixels, or its box rounded when it has
/// none) grown to cover the canvas as the layer maps it, rounded out to whole pixels
/// (`originalBounds.union(canvas.applying(inverted).integral)`, BrushStroke.swift:153-156).
pub fn image_grid(doc: &Document, layer: &Layer) -> Result<EditGrid, CommandError> {
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    let inverse = layer.transform.pixel_to_document(w, h).invert().ok_or_else(|| CommandError::Argument("the layer cannot be painted".into()))?;
    let corners = [(0.0, 0.0), (doc.width as f64, 0.0), (doc.width as f64, doc.height as f64), (0.0, doc.height as f64)].map(|(x, y)| inverse.apply(Point { x, y }));
    let x0 = corners.iter().map(|p| p.x).fold(0.0f64, f64::min).floor();
    let y0 = corners.iter().map(|p| p.y).fold(0.0f64, f64::min).floor();
    let x1 = corners.iter().map(|p| p.x).fold(w as f64, f64::max).ceil();
    let y1 = corners.iter().map(|p| p.y).fold(h as f64, f64::max).ceil();
    let (gw, gh) = (x1 - x0, y1 - y0);
    let others: u64 = doc.layers.iter().filter(|l| l.id != layer.id).filter_map(|l| l.pixels.as_ref()).map(|r| r.width as u64 * r.height as u64).sum();
    if gw > MAX_SIDE as f64 || gh > MAX_SIDE as f64 || (gw * gh) as u64 > MAX_PIXELS.saturating_sub(others) {
        return Err(CommandError::Project(ProjectError::TooLarge));
    }
    let (gw, gh, x, y) = (gw as u32, gh as u32, (-x0) as u32, (-y0) as u32);
    Ok(EditGrid { width: gw, height: gh, x, y, transform: ops::adjust::placed_like(&layer.transform, w, h, gw, gh, x as f64, y as f64) })
}

/// `layer`'s pixels placed on `grid`: transparent where it grew.
pub fn pixels_on(grid: &EditGrid, pixels: Option<&Raster>) -> Vec<u8> {
    let mut out = vec![0u8; grid.width as usize * grid.height as usize * 4];
    if let Some(p) = pixels {
        let row = p.width as usize * 4;
        for y in 0..p.height as usize {
            let at = ((y + grid.y as usize) * grid.width as usize + grid.x as usize) * 4;
            out[at..at + row].copy_from_slice(&p.bytes()[y * row..(y + 1) * row]);
        }
    }
    out
}

/// Where a document point falls along a gradient, 0 to 1, with the division done once.
enum Ramp { Solid, Linear { x: f64, y: f64, ux: f64, uy: f64 }, Radial { x: f64, y: f64, inverse: f64 } }

impl Ramp {
    fn of(paint: &Paint) -> Ramp {
        match paint {
            Paint::Fill(_) => Ramp::Solid,
            Paint::Gradient(g) => {
                let (dx, dy) = (g.end.x - g.start.x, g.end.y - g.start.y);
                let len2 = dx * dx + dy * dy;
                match g.shape {
                    GradientShape::Linear => Ramp::Linear { x: g.start.x, y: g.start.y, ux: dx / len2, uy: dy / len2 },
                    GradientShape::Radial => Ramp::Radial { x: g.start.x, y: g.start.y, inverse: 1.0 / len2.sqrt() },
                }
            }
        }
    }
    /// `GradientSpec::position` at (`x`, `y`).
    fn at(&self, x: f64, y: f64) -> f64 {
        match self {
            Ramp::Solid => 0.0,
            Ramp::Linear { x: sx, y: sy, ux, uy } => ((x - sx) * ux + (y - sy) * uy).clamp(0.0, 1.0),
            Ramp::Radial { x: sx, y: sy, inverse } => (((x - sx) * (x - sx) + (y - sy) * (y - sy)).sqrt() * inverse).min(1.0),
        }
    }
}

/// Paints `paint` over `data` (premultiplied RGBA, or grey when `grey`) on a `width` x `height` grid
/// that `transform` places: each pixel at its centre, inside the canvas, through `coverage` (the
/// selection's, None for all), source-over at the paint's alpha times its opacity times the coverage,
/// each channel rounded half up.
pub fn paint_grid(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, grey: bool) {
    let m = transform.pixel_to_document(width, height);
    let (cw, ch) = (doc.width as f64, doc.height as f64);
    let (w, channels) = (width as usize, if grey { 1 } else { 4 });
    let ramp = Ramp::of(paint);
    let (from, to, opacity) = match paint {
        Paint::Fill(c) => ([c[0], c[1], c[2], 1.0], [c[0], c[1], c[2], 1.0], 1.0),
        Paint::Gradient(g) => (g.from, g.to, g.opacity),
    };
    let delta: [f64; 4] = std::array::from_fn(|c| to[c] - from[c]);
    for y in 0..height as usize {
        // The row's first pixel centre in the document, and the step one pixel to the right.
        let first = m.apply(Point { x: 0.5, y: y as f64 + 0.5 });
        let cover = coverage.map(|c| &c.bytes()[y * w..(y + 1) * w]);
        let line = &mut data[y * w * channels..(y + 1) * w * channels];
        for x in 0..w {
            let k = cover.map_or(255, |c| c[x]);
            if k == 0 { continue; }
            let (dx, dy) = (first.x + m.a * x as f64, first.y + m.b * x as f64);
            if dx < 0.0 || dy < 0.0 || dx >= cw || dy >= ch { continue; }
            let t = ramp.at(dx, dy);
            let s = (from[3] + delta[3] * t) * opacity * k as f64 / 255.0;
            if s <= 0.0 { continue; }
            let px = &mut line[x * channels..(x + 1) * channels];
            let keep = 1.0 - s;
            if grey {
                px[0] = ((from[0] + delta[0] * t) * 255.0 * s + px[0] as f64 * keep + 0.5) as u8;
            } else {
                for c in 0..3 { px[c] = ((from[c] + delta[c] * t) * 255.0 * s + px[c] as f64 * keep + 0.5) as u8; }
                px[3] = (255.0 * s + px[3] as f64 * keep + 0.5) as u8;
            }
        }
    }
}

/// A covering mask carried onto the layer's new, trimmed grid (`crop` of `grid`): the old mask
/// stretched over where the layer's own pixels were, white where it grew (`expandMask`,
/// BrushStroke.swift:858-866). A uniform white mask stays one pixel: it shows everything either way.
fn followed(mask: &GrayRaster, old: (u32, u32), grid: &EditGrid, crop: (u32, u32, u32, u32)) -> GrayRaster {
    if mask.is_uniform() == Some(255) { return mask.clone(); }
    let (w, h) = (crop.2 - crop.0, crop.3 - crop.1);
    let mut out = vec![255u8; w as usize * h as usize];
    for y in 0..h { for x in 0..w {
        let (lx, ly) = ((x + crop.0) as i64 - grid.x as i64, (y + crop.1) as i64 - grid.y as i64);
        if lx < 0 || ly < 0 || lx >= old.0 as i64 || ly >= old.1 as i64 { continue; }
        let mx = ((lx as u64 * mask.width as u64) / old.0 as u64) as u32;
        let my = ((ly as u64 * mask.height as u64) / old.1 as u64) as u32;
        out[(y * w + x) as usize] = mask.bytes()[(my * mask.width + mx) as usize];
    }}
    GrayRaster::from_bytes(w, h, out)
}

/// A layer that can be painted: not a folder (its mask can be), not an adjustment layer, an enabled
/// mask when the mask is the target (`canPaint`, EditorSession+Brush.swift:5-11); a selection with
/// something in it when there is one.
fn check_target(doc: &Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    if doc.selection.as_ref().is_some_and(|s| s.is_empty()) { return Err(CommandError::Refused(ops::selection::EMPTY_SELECTION.into())); }
    if mask {
        let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
        if !m.enabled { return Err(CommandError::Argument("the mask is turned off".into())); }
    } else {
        if layer.is_group { return Err(CommandError::Argument("folders have no pixels".into())); }
        if layer.is_adjustment() { return Err(CommandError::Argument("an adjustment layer has no pixels".into())); }
    }
    Ok(())
}

/// A covering mask brought onto the layer's own pixel grid (a mask on its own placement keeps its
/// grid): the Mac paints a mask in the grid it covers (BrushStroke.swift:148-152), so a 1 x 1 or
/// otherwise sized covering mask is stretched onto the layer's pixels first, nearest.
fn mask_on_layer_grid(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
    let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
    if m.placement.is_some() { return Ok(()); }
    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    if (m.pixels.width, m.pixels.height) == (w, h) { return Ok(()); }
    let others = doc.used_mask_pixels() - m.pixels.width as u64 * m.pixels.height as u64;
    if (w as u64) * (h as u64) > MAX_PIXELS.saturating_sub(others) { return Err(CommandError::Project(ProjectError::TooLarge)); }
    let src = m.pixels.clone();
    let data = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| {
        let (mx, my) = ((x as u64 * src.width as u64 / w as u64) as u32, (y as u64 * src.height as u64 / h as u64) as u32);
        src.bytes()[(my * src.width + mx) as usize]
    }).collect();
    doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
    Ok(())
}

/// Paints `paint` into layer `id`'s pixels (`mask` false) or its mask, as one edit: Fill and the
/// Gradient's commit.
pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
    paint.check()?;
    check_target(doc, id, mask)?;
    if mask {
        mask_on_layer_grid(doc, id)?;
        let layer = doc.layer(id).unwrap();
        let m = layer.mask.as_ref().unwrap();
        let grid = m.placement.unwrap_or(layer.transform);
        let (w, h) = (m.pixels.width, m.pixels.height);
        let coverage = ops::adjust::edit_coverage(doc, clips, &grid, w, h)?;
        let mut data = m.pixels.bytes().to_vec();
        paint_grid(doc, &mut data, w, h, &grid, coverage.as_ref(), paint, true);
        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
        return Ok(());
    }
    let layer = doc.layer(id).unwrap().clone();
    let grid = image_grid(doc, &layer)?;
    let mut data = pixels_on(&grid, layer.pixels.as_ref());
    let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, grid.width, grid.height)?;
    paint_grid(doc, &mut data, grid.width, grid.height, &grid.transform, coverage.as_ref(), paint, false);
    let painted = Raster::from_premultiplied(grid.width, grid.height, data);
    // Trimmed to what is left; nothing left keeps the whole grid, as the Mac's `render` does.
    let crop = compositor::alpha_bounds(&painted).unwrap_or((0, 0, grid.width, grid.height));
    let (cw, ch) = (crop.2 - crop.0, crop.3 - crop.1);
    let (result, transform) = if (cw, ch) == (grid.width, grid.height) { (painted, grid.transform) } else {
        (painted.cropped(crop.0, crop.1, cw, ch), ops::adjust::placed_like(&grid.transform, grid.width, grid.height, cw, ch, -(crop.0 as f64), -(crop.1 as f64)))
    };
    // The layer's own grid before the edit (its box, rounded, when it had no pixels).
    let own = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
    let same_grid = crop == (grid.x, grid.y, grid.x + own.0, grid.y + own.1);
    let target = doc.layer_mut(id).unwrap();
    target.set_pixels(Some(result));
    target.transform = if same_grid { layer.transform } else { transform };
    if let Some(m) = target.mask.as_ref().filter(|m| m.placement.is_none() && !same_grid) {
        let mask = followed(&m.pixels, own, &grid, crop);
        target.mask_mut().unwrap().pixels = mask;
    }
    Ok(())
}
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test raster_edits` (12 tests); the suite 493 passed, 7 ignored (+12, +1 ignored). Natively: a gradient over a whole 24 MP layer 566 ms, 100 MP 2437 ms; a fill in a 2000 x 1500 ellipse 110 / 356 ms. (Through the worker, Tasks 14 and 15 measure what the page sees.)

- [ ] **Step 5: Prove it bites**

(1) In `paint_grid`, truncate the colour channels instead of rounding them half up: the black-to-white test fails at column 1 (2 for 3), and Foreground to Transparent and Reverse at their 127.5s (measured). Restore. (2) In `image_grid`, do not grow the grid to the right: `a_layer_smaller_than_the_canvas_grows_to_it_is_trimmed_and_its_mask_follows_with_white` fails (the layer stays 40 wide where it should be 80; measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/ops/raster_edit.rs engine/tests/raster_edits.rs
git commit -m "feat(engine): Fill and the Gradient, painted as the Mac paints them, the layer grown to the canvas and trimmed" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine/src/command.rs engine/src/compositor.rs engine/src/engine.rs engine/src/lib.rs engine/src/ops/adjust.rs engine/src/ops/mod.rs engine/src/ops/raster_edit.rs engine/tests/perf_4b1.rs engine/tests/raster_edits.rs
```


---

### Task 9: Gradient previews: reduced while dragged, patches inside small selections, masks

A pending gradient is an engine preview (`PreviewRequest::Gradient { layer, mask, gradient, dragging }`) with three targets (ruling OQ8): `Pixels` - the grown grid reduced to `GRADIENT_DRAG_LIMIT` (1024) while dragging or `GRADIENT_SETTLED_LIMIT` (2048) once let go, the layer's own halvings placed in it, painted; `Patch(rect)` - inside a selection whose rectangle holds at most `PATCH_LIMIT` (2^19) pixels on a layer that already covers the canvas and has no effects, that rectangle at full size, drawn over the stored pixels; `Mask(mask)` - the mask reduced to the same limits, painted. The engine's views of the document split: `render_document` (what the renderer's plan and state need; a patch changes only the revision) and `render_bytes` (what the CPU compositor draws: the patch applied). A patch's lineage records its rectangle, so the GPU takes it through `Engine::layer_region` (the stored pixels with the patch, halved on their own - the same bytes as the patched raster halved). What the commit would refuse, the preview does not show.

**Files:**
- Modify: `engine/src/preview.rs`, `engine/src/engine.rs` (`displayed`, `render_document`, `render_bytes`, `record_preview`, `layer_region`, `mask_pixels`), `engine/src/lineage.rs` (`record`), `engine/src/ops/raster_edit.rs` (`paint_layer_check`, the coverage fraction), `engine-wasm/src/lib.rs`, `app/src/engine/client.ts` (`layerRegion`), `app/src/engine/types.ts` (`GradientSpec`, the preview and the two commands), `app/src/canvas/layer-textures.ts`, `app/src/canvas/gl-renderer.ts`
- Create tests: `engine/tests/gradient_preview.rs`, `app/tests/e2e/gradient-preview.spec.ts`; modify `app/tests/unit/partial-upload.test.ts`, `engine/tests/perf_4b1.rs`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `PreviewRequest::Gradient`, `PreviewTarget { Pixels, Patch(PixelRect), Mask(GrayRaster) }`, `PixelPreview::patched`, `GRADIENT_DRAG_LIMIT`, `GRADIENT_SETTLED_LIMIT`, `PATCH_LIMIT`, `Engine::preview`, `Engine::layer_region(id, layer, level, rect)`, `EngineClient.layerRegion`; `LayerTextures.update` now takes the region's bytes.

- [ ] **Step 1: Write the tests**

The engine tests: a dragged gradient previews from a reduced copy of the layer grown to the canvas (3000 x 2000 halved to 750 x 500; the layer's own pixels land where it is: alpha 171 just left of its edge at x 500 is the gradient's own 495 / 1500 of the way, 255 just right of it), and settled at 1500 x 1000; inside a 100 x 80 selection the preview is a 104 x 84 patch whose delta is its rectangle, whose `layer_region` at level 0 and level 2 equals the patched raster's, which the commit paints exactly; a selection past the limit (730 x 720, whose patch would be 734 x 724 = 531,416 px, against 720 x 720's 724 x 724 = 524,176) and a layer that must grow preview whole; a mask gradient previews the mask reduced; what the commit would refuse shows nothing. The e2e: a patch reaches the GPU as its rectangle only (no `texImage2D`, at most the selection's box plus two pixels a side), draws what applying the gradient draws, and clearing it restores the layer by the same rectangle; a whole-layer gradient and a mask gradient small enough not to be reduced preview exactly as applying them draws.

Create `app/tests/e2e/gradient-preview.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: a gradient's preview on the GPU (engine/tests/gradient_preview.rs pins the engine). A
// patch inside a small selection reaches the texture as its rectangle alone; a preview small enough
// not to be reduced draws exactly what applying the gradient draws; clearing it goes back the same way.

/** Counts texture uploads and the texels `texSubImage2D` sends, as partial-upload.spec.ts does. */
async function countUploads(page: Page) {
  await page.addInitScript(() => {
    const log = { image: 0, sub: 0, subArea: 0 };
    (window as any).__uploads = log;
    const proto = WebGL2RenderingContext.prototype as any;
    const image = proto.texImage2D, sub = proto.texSubImage2D;
    proto.texImage2D = function (...args: unknown[]) { log.image++; return image.apply(this, args); };
    proto.texSubImage2D = function (...args: unknown[]) { log.sub++; log.subArea += (args[4] as number) * (args[5] as number); return sub.apply(this, args); };
  });
}
const frames = (page: Page) => page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
const uploads = (page: Page) => page.evaluate(() => ({ ...(window as any).__uploads }));
const resetUploads = (page: Page) => page.evaluate(() => { const u = (window as any).__uploads; u.image = 0; u.sub = 0; u.subArea = 0; });
const keep = (page: Page, name: string) => page.evaluate((name) => { (window as any)[name] = (window as any).__compositor.readDocumentPixels(); }, name);
/** The largest channel difference between the GPU's picture now and a kept one. */
const worstAgainst = (page: Page, name: string) => page.evaluate((name) => {
  const now = (window as any).__compositor.readDocumentPixels() as Uint8Array, kept = (window as any)[name] as Uint8Array;
  if (now.length !== kept.length) return -1;
  let worst = 0;
  for (let i = 0; i < now.length; i++) worst = Math.max(worst, Math.abs(now[i] - kept[i]));
  return worst;
}, name);
const setPreview = (page: Page, request: unknown) => page.evaluate((request) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  api.engine.setPreview(s.activeId, request); s.refresh(s.activeId);
}, request);

/** A 600 x 401 noise layer that covers its canvas, active, at 1:1. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await countUploads(page);
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const ids = await page.evaluate(async () => {
    const api = (window as any).__compositor;
    const doc = api.engine.newDocument(10, 10, false);
    api.engine.execute(doc, { type: "CanvasSize", width: 600, height: 401, anchor: 4, fill: [0.5, 0.4, 0.3] });
    const layer = api.engine.state(doc).layers[0].id;
    api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
    api.engine.execute(doc, { type: "ApplyFilter", id: layer, params: { filter: "AddNoise", amount: 30, gaussian: false, monochromatic: false, seed: 3 } });
    api.store.getState().openDocument(doc);
    api.setCheckerboard(false);
    await api.setZoom(1);
    return { doc, layer };
  });
  await frames(page);
  return ids;
}
const gradient = { shape: "Radial", start: [220, 190], end: [300, 240], from: [1, 0.2, 0, 1], to: [0, 0, 1, 0.3], opacity: 0.9 };

test("a gradient inside a small selection previews as a patch taken as its rectangle, draws what applying it draws, and clears the same way", async ({ page }) => {
  const { layer } = await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[193, 161], [393, 161], [393, 301], [193, 301]], mode: "Replace", antialiased: true }));
  await frames(page);
  await keep(page, "__before");
  await resetUploads(page);
  await setPreview(page, { preview: "Gradient", layer, mask: false, gradient, dragging: true });
  await frames(page);
  const patched = await uploads(page);
  expect(patched.image, "no texture made afresh").toBe(0);
  expect(patched.sub).toBeGreaterThan(0);
  // The selection's 200 x 140 box plus the clip's pixel and the sampling pixel on each side, at most.
  expect(patched.subArea).toBeLessThanOrEqual(204 * 144 + 16);
  await keep(page, "__previewed");
  await resetUploads(page);
  await setPreview(page, null);
  await frames(page);
  const cleared = await uploads(page);
  expect(cleared.image).toBe(0);
  expect(cleared.subArea).toBeLessThanOrEqual(204 * 144 + 16);
  expect(await worstAgainst(page, "__before"), "clearing the patch restores the layer").toBe(0);
  await page.evaluate(({ layer, gradient }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: false, gradient }), { layer, gradient });
  await frames(page);
  expect(await worstAgainst(page, "__previewed"), "the patch is the applied gradient").toBe(0);
});

test("a whole-layer gradient and a mask gradient small enough not to be reduced preview as applying them draws", async ({ page }) => {
  const { layer } = await setup(page);
  const linear = { ...gradient, shape: "Linear" };
  await setPreview(page, { preview: "Gradient", layer, mask: false, gradient: linear, dragging: true });
  await frames(page);
  await keep(page, "__previewed");
  await setPreview(page, null);
  await page.evaluate(({ layer, linear }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: false, gradient: linear }), { layer, linear });
  await frames(page);
  expect(await worstAgainst(page, "__previewed"), "the whole-layer preview is the applied gradient").toBe(0);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.undo(); });
  await page.evaluate((layer) => (window as any).__compositor.store.getState().run({ type: "AddMask", id: layer, revealing: true }), layer);
  const grey = { ...gradient, shape: "Linear", start: [40, 0], end: [560, 0], from: [0, 0, 0, 1], to: [1, 1, 1, 1], opacity: 1 };
  await setPreview(page, { preview: "Gradient", layer, mask: true, gradient: grey, dragging: true });
  await frames(page);
  await keep(page, "__masked");
  expect(await worstAgainst(page, "__previewed"), "the mask preview shows").toBeGreaterThan(0);
  await setPreview(page, null);
  await page.evaluate(({ layer, grey }) => (window as any).__compositor.store.getState().run({ type: "Gradient", id: layer, mask: true, gradient: grey }), { layer, grey });
  await frames(page);
  expect(await worstAgainst(page, "__masked"), "the mask preview is the applied mask gradient").toBe(0);
});
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -275,3 +275,65 @@ test("history: whole-layer edits at 24 and 100 MP stay within memory, and a push
   expect(out["1000 layers: undo depth"]).toBe(100);
   expect(out["1000 layers, 100 entries: rename at the cap, ms (mean of 10)"]).toBeLessThan(5 + 20);
 });
+
+test("gradient previews: a drag tick, the settled preview and a patch in a 700 px selection, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      api.store.getState().openDocument(doc);
+      await settle(); frame();
+      const gradient = (i: number) => ({ shape: "Linear", start: [w * 0.2 + i, h * 0.3], end: [w * 0.8, h * 0.7 - i], from: [1, 0.2, 0, 1], to: [0, 0, 1, 0.3], opacity: 0.9 });
+      // One tick: the engine's preview, then the frame that draws it (what a pointer move costs).
+      const tick = (i: number, dragging: boolean) => {
+        const t0 = performance.now();
+        api.engine.setPreview(doc, { preview: "Gradient", layer, mask: false, gradient: gradient(i), dragging });
+        const engine = performance.now() - t0;
+        api.store.getState().refresh(doc);
+        return [engine, frame()];
+      };
+      const worst = (dragging: boolean) => {
+        const ticks: number[][] = [];
+        for (let i = 0; i < 6; i++) ticks.push(tick(i * 7, dragging));
+        ticks.shift(); // the first builds the reduced copies
+        return [Math.max(...ticks.map((t) => t[0])), Math.max(...ticks.map((t) => t[1])), Math.max(...ticks.map((t) => t[0] + t[1]))].map((v) => Math.round(v));
+      };
+      for (const zoom of ["fit", "1:1"]) {
+        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
+        let [e, f, t] = worst(true);
+        result[`drag engine ${zoom}`] = e; result[`drag frame ${zoom}`] = f; result[`drag total ${zoom}`] = t;
+        [e, f, t] = worst(false);
+        result[`settled engine ${zoom}`] = e; result[`settled frame ${zoom}`] = f; result[`settled total ${zoom}`] = t;
+        api.engine.setPreview(doc, null); api.store.getState().refresh(doc); frame(); await settle();
+      }
+      // A 700 x 700 selection in the middle (its clip, a pixel wider each side, within PATCH_LIMIT):
+      // a full-size patch, at 1:1.
+      const x = w / 2 - 350, y = h / 2 - 350;
+      api.engine.execute(doc, { type: "SelectShape", kind: "Rectangle", points: [[x, y], [x + 700, y], [x + 700, y + 700], [x, y + 700]], mode: "Replace", antialiased: false });
+      api.store.getState().refresh(doc); frame(); await settle();
+      const [e, f, t] = worst(true);
+      result["patch engine"] = e; result["patch frame"] = f; result["patch total"] = t;
+      result[`patch uploads: ${(window as any).__lastUploads}`] = 0;
+      api.engine.setPreview(doc, null); api.store.getState().closeDocument(doc);
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`gradient previews (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) {
+    for (const zoom of ["fit", "1:1"]) {
+      expect(out[`${label}: drag total ${zoom}`]).toBeLessThan(50);
+      expect(out[`${label}: settled total ${zoom}`]).toBeLessThan(150);
+    }
+    expect(out[`${label}: patch total`]).toBeLessThan(50);
+  }
+});
```

```diff
--- a/app/tests/unit/partial-upload.test.ts
+++ b/app/tests/unit/partial-upload.test.ts
@@ -48,7 +48,7 @@ describe("levelRect", () => {
 });
 
 describe("LayerTextures.update", () => {
-  it("uploads only the changed rectangle into each chunk it meets, straight from the raster", () => {
+  it("uploads the changed rectangle's bytes into each chunk it meets", () => {
     const { gl, calls } = stubGl();
     const textures = new LayerTextures(gl);
     // 5000 x 3000: chunks start at x 0, 2048, 4096 and y 0, 2048.
@@ -56,12 +56,13 @@ describe("LayerTextures.update", () => {
     expect(calls.filter((c) => c.kind === "image").length).toBe(6);
     calls.length = 0;
     textures.update("D", "A", "px:2", 2, { x: 2000, y: 1000, width: 100, height: 1100 }, new Uint8Array(4));
-    // Across the x = 2048 edge and the y = 2048 edge: four pieces, each placed in its own chunk.
+    // Across the x = 2048 edge and the y = 2048 edge: four pieces, each placed in its own chunk and
+    // read from its own part of the 100 x 1100 region.
     expect(calls).toEqual([
-      { kind: "sub", x: 2000, y: 1000, width: 48, height: 1048, skipX: 2000, skipY: 1000, rowLength: 5000 },
-      { kind: "sub", x: 0, y: 1000, width: 52, height: 1048, skipX: 2048, skipY: 1000, rowLength: 5000 },
-      { kind: "sub", x: 2000, y: 0, width: 48, height: 52, skipX: 2000, skipY: 2048, rowLength: 5000 },
-      { kind: "sub", x: 0, y: 0, width: 52, height: 52, skipX: 2048, skipY: 2048, rowLength: 5000 },
+      { kind: "sub", x: 2000, y: 1000, width: 48, height: 1048, skipX: 0, skipY: 0, rowLength: 100 },
+      { kind: "sub", x: 0, y: 1000, width: 52, height: 1048, skipX: 48, skipY: 0, rowLength: 100 },
+      { kind: "sub", x: 2000, y: 0, width: 48, height: 52, skipX: 0, skipY: 1048, rowLength: 100 },
+      { kind: "sub", x: 0, y: 0, width: 52, height: 52, skipX: 48, skipY: 1048, rowLength: 100 },
     ]);
     const t = textures.get("D", "A")!;
     expect([t.key, t.revision]).toEqual(["px:2", 2]);
```

Create `engine/tests/gradient_preview.rs`:

```rust
//! The gradient's preview (Phase 4b-1): a reduced copy of the grown layer while the line is dragged,
//! a full-size patch inside a small selection on a layer over the canvas, a reduced mask on a mask;
//! the patch reaches the GPU as its rectangle (`pixels_delta`, `layer_region`).
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
fn preview(e: &mut Engine, id: Uuid, layer: Uuid, mask: bool, gradient: &GradientSpec, dragging: bool) {
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask, gradient: gradient.clone(), dragging })).unwrap();
}
/// Unequal opaque pixels.
fn pattern(width: u32, height: u32) -> Raster {
    let data = (0..width * height).flat_map(|i| { let v = i.wrapping_mul(2654435761); [(v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8, 255] }).collect();
    Raster::from_premultiplied(width, height, data)
}
/// A document of `canvas` holding one `pattern` layer of `size` at `at`, active, made in its engine.
fn document(canvas: (u32, u32), size: (u32, u32), at: Point) -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(canvas.0, canvas.1);
    let layer = Layer::with_pixels("Pattern", pattern(size.0, size.1), at);
    let id = layer.id;
    doc.active_layer_id = Some(id);
    doc.layers = vec![layer];
    let mut e = Engine::new();
    let handle = e.insert_document(doc);
    (e, handle, id)
}
fn shown(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0) }

#[test]
fn a_dragged_gradient_previews_from_a_reduced_copy_of_the_layer_grown_to_the_canvas() {
    // A 1000 x 800 layer at (500, 400) on 3000 x 2000: the grid grows to the canvas, 3000 x 2000.
    let (mut e, id, layer) = document((3000, 2000), (1000, 800), p(500.0, 400.0));
    // Black at the left fading to nothing at x 1500: the layer shows through on the right.
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 1000.0), end: p(1500.0, 1000.0), from: [0.0, 0.0, 0.0, 1.0], to: [0.0, 0.0, 0.0, 0.0], opacity: 1.0 };
    preview(&mut e, id, layer, false, &g, true);
    let state = e.state(id).unwrap().layers[0].clone();
    // Halved until the longer side is at most 1024: 3000 -> 1500 -> 750.
    assert_eq!((state.pixels_width, state.pixels_height), (750, 500));
    assert!(state.transform.origin.x <= 0.0 && state.transform.origin.y <= 0.0 && state.transform.origin.x + state.transform.size.width >= 3000.0, "{:?}", state.transform);
    // Left of the layer, where there were no pixels: the gradient alone (a quarter of the way: alpha 191).
    let left = shown(&e, id, 375, 1000);
    assert!(left[3].abs_diff(191) <= 2, "{left:?}");
    // The layer's own (halved) pixels land where the layer is: just left of its edge at x 500 only the
    // gradient (495 / 1500 of the way: alpha 171), just right of it the opaque layer under it.
    assert!(shown(&e, id, 495, 1000)[3].abs_diff(171) <= 2);
    assert_eq!(shown(&e, id, 505, 1000)[3], 255);
    // Past the layer's right edge and the gradient's end: nothing.
    assert_eq!(shown(&e, id, 1510, 1000)[3], 0);
    // Past the gradient's end, the layer's own pixels (halved twice), as the stored layer shows them.
    let right = shown(&e, id, 1400, 1000);
    e.set_preview(id, None).unwrap();
    let stored = shown(&e, id, 1400, 1000);
    assert_eq!(right[3], 255);
    // A reduced pixel averages its 4 x 4 block, which holds this one: close only where neighbours
    // agree, so compare the alpha, which is 255 in every pixel of the layer.
    assert_eq!(stored[3], 255);
    // Settled: at most 2048 across (GRADIENT_SETTLED_LIMIT): 3000 -> 1500.
    preview(&mut e, id, layer, false, &g, false);
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.pixels_width, state.pixels_height), (1500, 1000));
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
    assert_eq!(e.state(id).unwrap().undo_depth, 0, "a preview records nothing");
}

#[test]
fn a_gradient_inside_a_small_selection_previews_as_a_patch_the_commit_equals() {
    // A 600 x 400 layer over the 600 x 400 canvas; a 100 x 80 selection off its centre.
    let (mut e, id, layer) = document((600, 400), (600, 400), p(0.0, 0.0));
    select(&mut e, id, 123.0, 77.0, 100.0, 80.0);
    let stored_revision = e.state(id).unwrap().layers[0].pixels_revision;
    let g = GradientSpec { shape: GradientShape::Radial, start: p(150.0, 100.0), end: p(210.0, 140.0), from: [1.0, 0.2, 0.0, 1.0], to: [0.0, 0.0, 1.0, 0.3], opacity: 0.9 };
    preview(&mut e, id, layer, false, &g, true);
    let PreviewTarget::Patch(rect) = e.preview(id).unwrap().target.clone() else { panic!("a patch") };
    // The selection's box, a pixel for its clip and one for sampling on every side.
    assert_eq!(rect, PixelRect { x: 121, y: 75, width: 104, height: 84 });
    let preview_revision = e.state(id).unwrap().layers[0].pixels_revision;
    assert_ne!(preview_revision, stored_revision);
    assert_eq!(e.pixels_delta(id, layer, stored_revision).unwrap(), Some(rect), "the GPU uploads the patch alone");
    let patch = e.preview(id).unwrap().raster.bytes().to_vec();
    assert_eq!(e.layer_region(id, layer, 0, rect).unwrap(), patch);
    // Halved twice, the region is the patched layer's own halving there.
    let mut patched = e.preview(id).unwrap().patched(e.document(id).unwrap().layers[0].pixels.as_ref().unwrap());
    patched = patched.halved().halved();
    let quarter = PixelRect { x: rect.x / 4, y: rect.y / 4, width: (rect.x + rect.width).div_ceil(4) - rect.x / 4, height: (rect.y + rect.height).div_ceil(4) - rect.y / 4 };
    assert_eq!(e.layer_region(id, layer, 2, quarter).unwrap(), patched.cropped(quarter.x, quarter.y, quarter.width, quarter.height).into_bytes());
    // The CPU draws the patch over the stored pixels.
    let through = shown(&e, id, 150, 100);
    // Taken back: the rectangle again, to the stored pixels.
    e.set_preview(id, None).unwrap();
    assert_eq!(e.pixels_delta(id, layer, preview_revision).unwrap(), Some(rect));
    let stored = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    assert_eq!(e.layer_region(id, layer, 0, rect).unwrap(), stored.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes());
    // The commit paints exactly the patch.
    run(&mut e, id, Command::Gradient { id: layer, mask: false, gradient: g });
    let committed = e.document(id).unwrap().layers[0].pixels.clone().unwrap();
    assert_eq!(committed.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes(), patch);
    assert_eq!(shown(&e, id, 150, 100), through);
}

#[test]
fn a_large_selection_or_a_layer_that_must_grow_previews_whole() {
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(300.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    // On a layer over the canvas, 720 x 720 selected is a 724 x 724 patch (two pixels a side, as
    // above), within PATCH_LIMIT (1 << 19 = 524288 >= 524176); 730 x 720 is 734 x 724 = 531416, past it.
    let (mut e, id, layer) = document((1200, 1100), (1200, 1100), p(0.0, 0.0));
    select(&mut e, id, 10.0, 10.0, 720.0, 720.0);
    preview(&mut e, id, layer, false, &g, true);
    let target = e.preview(id).map(|p| p.target.clone());
    assert!(matches!(target, Some(PreviewTarget::Patch(PixelRect { width: 724, height: 724, .. }))), "{}", match &target { Some(PreviewTarget::Patch(r)) => format!("{r:?}"), Some(_) => "reduced".into(), None => "none".into() });
    select(&mut e, id, 10.0, 10.0, 730.0, 720.0);
    preview(&mut e, id, layer, false, &g, true);
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
    // A small selection, but the layer does not cover the canvas and grows.
    let (mut e, id, layer) = document((1200, 1100), (400, 300), p(100.0, 100.0));
    select(&mut e, id, 150.0, 150.0, 50.0, 40.0);
    preview(&mut e, id, layer, false, &g, true);
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Pixels));
}

#[test]
fn a_mask_gradient_previews_the_mask_reduced() {
    let (mut e, id, layer) = document((3000, 2000), (3000, 2000), p(0.0, 0.0));
    run(&mut e, id, Command::AddMask { id: layer, revealing: true });
    let before = e.state(id).unwrap().layers[0].clone();
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(3000.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, id, layer, true, &g, true);
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (750, 500), "the 1 x 1 mask shown on the layer's grid, reduced");
    assert_ne!(state.mask_revision, before.mask_revision);
    assert_eq!(state.pixels_revision, before.pixels_revision, "the pixels are not previewed");
    assert_eq!(e.mask_pixels(id, layer).unwrap().unwrap().width, 750);
    // Half way along: the mask half grey, the layer half shown.
    assert!(shown(&e, id, 1500, 1000)[3].abs_diff(128) <= 2);
    e.set_preview(id, None).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].mask_width, 1);
}

#[test]
fn a_gradient_the_commit_would_refuse_shows_nothing() {
    let (mut e, id, layer) = document((300, 200), (300, 200), p(0.0, 0.0));
    select(&mut e, id, 10.0, 10.0, 10.0, 10.0);
    run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(0.0, 0.0), p(300.0, 0.0), p(300.0, 200.0), p(0.0, 200.0)], mode: SelectionMode::Subtract, antialiased: true });
    let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(100.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
    preview(&mut e, id, layer, false, &g, true);
    assert!(e.preview(id).is_none(), "an empty selection: nothing");
    run(&mut e, id, Command::Deselect);
    let short = GradientSpec { end: p(0.2, 0.0), ..g };
    preview(&mut e, id, layer, false, &short, true);
    assert!(e.preview(id).is_none(), "a line under half a pixel: nothing");
}
```

```diff
--- a/engine/tests/perf_4b1.rs
+++ b/engine/tests/perf_4b1.rs
@@ -90,3 +90,30 @@ fn a_rename_at_the_history_cap_with_a_thousand_layers() {
     println!("rename at the cap, 1000 layers: {:?} ms; trim alone {trim:.2} ms", times.iter().map(|t| (t * 10.0).round() / 10.0).collect::<Vec<_>>());
     assert_eq!(e.state(id).unwrap().undo_depth, HISTORY_ENTRY_LIMIT);
 }
+
+#[test]
+#[ignore]
+fn gradient_previews_dragged_settled_and_patched_at_24_and_100_mp() {
+    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
+        let (mut e, id, layer) = filled(w, h);
+        let gradient = |i: f64| GradientSpec { shape: GradientShape::Linear, start: Point { x: w as f64 * 0.2 + i, y: h as f64 * 0.3 }, end: Point { x: w as f64 * 0.8, y: h as f64 * 0.7 - i },
+            from: [1.0, 0.2, 0.0, 1.0], to: [0.0, 0.0, 1.0, 0.3], opacity: 0.9 };
+        // The worst of five ticks after a first one (which halves the layer once for all).
+        let worst = |e: &mut Engine, dragging: bool| {
+            e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: false, gradient: gradient(0.0), dragging })).unwrap();
+            (1..6).map(|i| {
+                let t = Instant::now();
+                e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: false, gradient: gradient(i as f64 * 7.0), dragging })).unwrap();
+                ms(t)
+            }).fold(0.0, f64::max)
+        };
+        let drag = worst(&mut e, true);
+        let settled = worst(&mut e, false);
+        e.set_preview(id, None).unwrap();
+        let (x, y) = (w as f64 / 2.0 - 350.0, h as f64 / 2.0 - 350.0);
+        run(&mut e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![Point { x, y }, Point { x: x + 700.0, y }, Point { x: x + 700.0, y: y + 700.0 }, Point { x, y: y + 700.0 }], mode: SelectionMode::Replace, antialiased: false });
+        let patch = worst(&mut e, true);
+        println!("{label}: gradient preview tick dragging {drag:.0} ms, settled {settled:.0} ms, patch in a 700 px selection {patch:.0} ms");
+        assert!(e.preview(id).is_some());
+    }
+}
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test gradient_preview`: does not compile (`PreviewRequest::Gradient`, `PreviewTarget`). The unit test fails on `update`'s new signature.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/gl-renderer.ts
+++ b/app/src/canvas/gl-renderer.ts
@@ -133,14 +133,16 @@ export class GlRenderer implements Renderer {
       const [width, height] = fx ? [fx.width, fx.height] : [layer.pixelsWidth, layer.pixelsHeight];
       const size = sizeAtLevel(width, height, level);
       // Plain pixels at the same level and size: ask what changed since the uploaded revision and
-      // upload only that (Engine::pixels_delta); a change the engine cannot bound goes whole.
+      // upload only that (Engine::pixels_delta, Engine::layer_region, which reads a gradient's patch
+      // preview without making the whole patched raster); a change the engine cannot bound goes whole.
       const kept = fx ? undefined : this.textures.get(state.id, layer.id);
       if (kept && kept.revision !== null && kept.level === level && kept.nearest === nearest && kept.width === size.width && kept.height === size.height) {
         const delta = engine.pixelsDelta(state.id, layer.id, kept.revision);
         if (delta) {
           const rect = levelRect(delta, level, width, height);
-          const pixels = rect.width > 0 && rect.height > 0 ? engine.layerPixels(state.id, layer.id, level) : null;
-          if (rect.width === 0 || rect.height === 0 || pixels) { this.textures.update(state.id, layer.id, bytesKey, layer.pixelsRevision, rect, pixels ?? new Uint8Array(0)); continue; }
+          const region = rect.width > 0 && rect.height > 0 ? engine.layerRegion(state.id, layer.id, level, rect) : new Uint8Array(0);
+          this.textures.update(state.id, layer.id, bytesKey, layer.pixelsRevision, rect, region);
+          continue;
         }
       }
       const upload = (pixels: Uint8Array | null) => this.textures.sync(state.id, layer.id, bytesKey, nearest, pixels, level, size, fx ? null : layer.pixelsRevision);
```

```diff
--- a/app/src/canvas/layer-textures.ts
+++ b/app/src/canvas/layer-textures.ts
@@ -103,24 +103,24 @@ export class LayerTextures {
     gl.pixelStorei(gl.UNPACK_ROW_LENGTH, 0);
     this.layers.set(k, { key: bytesKey, revision, level, width: size.width, height: size.height, nearest, chunks });
   }
-  /** Uploads only `rect` (in the texture's own pixels, at its level) of `pixels`, the whole raster at
-   * that level, into every chunk it meets, straight from the view: `texSubImage2D` with the row length
-   * and skips set, no copy. The texture then names `bytesKey` and `revision`. */
-  update(docId: string, id: string, bytesKey: string, revision: number, rect: PixelRect, pixels: Uint8Array): void {
+  /** Uploads `region`, the bytes of `rect` (in the texture's own pixels, at its level; the engine's
+   * `layer_region`), into every chunk the rectangle meets: `texSubImage2D` with the region's row length
+   * and each chunk's skips. The texture then names `bytesKey` and `revision`. */
+  update(docId: string, id: string, bytesKey: string, revision: number, rect: PixelRect, region: Uint8Array): void {
     const t = this.layers.get(key(docId, id));
     if (!t) return;
     const gl = this.gl;
     if (rect.width > 0 && rect.height > 0) {
-      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, t.width);
+      gl.pixelStorei(gl.UNPACK_ROW_LENGTH, rect.width);
       gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
       for (const c of t.chunks) {
         const x0 = Math.max(rect.x, c.x), y0 = Math.max(rect.y, c.y);
         const x1 = Math.min(rect.x + rect.width, c.x + c.width), y1 = Math.min(rect.y + rect.height, c.y + c.height);
         if (x1 <= x0 || y1 <= y0) continue;
         gl.bindTexture(gl.TEXTURE_2D, c.texture);
-        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x0);
-        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y0);
-        gl.texSubImage2D(gl.TEXTURE_2D, 0, x0 - c.x, y0 - c.y, x1 - x0, y1 - y0, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
+        gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, x0 - rect.x);
+        gl.pixelStorei(gl.UNPACK_SKIP_ROWS, y0 - rect.y);
+        gl.texSubImage2D(gl.TEXTURE_2D, 0, x0 - c.x, y0 - c.y, x1 - x0, y1 - y0, gl.RGBA, gl.UNSIGNED_BYTE, region);
       }
       gl.pixelStorei(gl.UNPACK_SKIP_PIXELS, 0);
       gl.pixelStorei(gl.UNPACK_SKIP_ROWS, 0);
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -140,6 +140,11 @@ export class EngineClient {
   /** What changed in the layer's pixels since revision `from` (engine `pixels_delta`): a rectangle of
    * its pixel grid, empty when nothing did, or null when the whole raster must be uploaded again. */
   pixelsDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.pixels_delta(doc, layer, from)); }
+  /** The bytes of `rect` of the layer's pixels after `level` halvings, as the canvas shows them
+   * (engine `layer_region`): a copy, for a partial upload. */
+  layerRegion(doc: string, layer: string, level: number, rect: PixelRect): Uint8Array {
+    return this.wasm.layer_region(doc, layer, level, rect.x, rect.y, rect.width, rect.height);
+  }
   /** `pixelsDelta` for the layer's mask, in the mask's own grid (engine `mask_delta`). */
   maskDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.mask_delta(doc, layer, from)); }
   clipDependents(doc: string, ids: string[]): string[] { return JSON.parse(this.wasm.clip_dependents(doc, JSON.stringify(ids))) as string[]; }
```

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -73,11 +73,21 @@ export type FilterParams =
   | { filter: "LensCorrection"; distortion: number };
 export type FilterKind = FilterParams["filter"];
 
+/** A gradient (engine `GradientSpec`): its shape, its line in document pixels, its two stops as
+ * straight RGBA 0..1 (a mask reads the first channel as grey), and its opacity, 0.01 to 1. */
+export interface GradientSpec {
+  shape: "Linear" | "Radial"; start: PointTuple; end: PointTuple;
+  from: [number, number, number, number]; to: [number, number, number, number]; opacity: number;
+}
+
 export type PreviewRequest =
   | { preview: "Adjustment"; layer: string; adjustment: LayerAdjustment }
   /** The same while a slider moves: previewed from a smaller copy until input settles. */
   | { preview: "DragAdjustment"; layer: string; adjustment: LayerAdjustment }
-  | { preview: "Filter"; layer: string; params: FilterParams };
+  | { preview: "Filter"; layer: string; params: FilterParams }
+  /** A gradient not yet applied: from a smaller copy while `dragging` (engine `GRADIENT_DRAG_LIMIT`),
+   * or a full-size patch inside a small selection on a layer over the canvas. */
+  | { preview: "Gradient"; layer: string; mask: boolean; gradient: GradientSpec; dragging: boolean };
 
 export interface LayerState {
   id: string; name: string; visible: boolean; isGroup: boolean; parentId: string | null; opacity: number;
@@ -201,7 +211,9 @@ export type Command =
   | { type: "LoadLayerSelection"; id: string; mode: SelectionMode; antialiased: boolean }
   | { type: "LoadMaskSelection"; id: string; mode: SelectionMode; antialiased: boolean }
   | { type: "ClearSelectedPixels"; id: string; mask: boolean }
-  | { type: "AddMaskFromSelection"; id: string; revealing: boolean };
+  | { type: "AddMaskFromSelection"; id: string; revealing: boolean }
+  | { type: "Fill"; id: string; mask: boolean; color: [number, number, number] }
+  | { type: "Gradient"; id: string; mask: boolean; gradient: GradientSpec };
 
 export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }
 /** A rectangle of a layer's pixel grid (or its mask's), in whole pixels (engine `PixelRect`). */
```

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -305,15 +305,18 @@ impl WasmEngine {
         let rect = self.engine.mask_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
         Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
     }
+    /// The mask as the canvas shows it (`Engine::mask_pixels`): its buffer is the document's, or an
+    /// open mask preview's, which lives until the next engine call either way.
     pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
-        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
-        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
-        Ok(l.mask.as_ref().map_or(std::ptr::null(), |m| m.pixels.bytes().as_ptr()))
+        Ok(self.engine.mask_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?.map_or(std::ptr::null(), |m| m.bytes().as_ptr()))
     }
     pub fn mask_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
-        let d = self.engine.document(parse_id(doc)?).ok_or_else(|| JsError::new("no document"))?;
-        let l = d.layer(parse_id(layer)?).ok_or_else(|| JsError::new("no layer"))?;
-        Ok(l.mask.as_ref().map_or(0, |m| m.pixels.bytes().len()))
+        Ok(self.engine.mask_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?.map_or(0, |m| m.bytes().len()))
+    }
+    /// `Engine::layer_region`: the bytes of a rectangle of the layer's pixels at `level`, as shown.
+    pub fn layer_region(&self, doc: &str, layer: &str, level: u32, x: u32, y: u32, width: u32, height: u32) -> Result<Uint8Array, JsError> {
+        let bytes = self.engine.layer_region(parse_id(doc)?, parse_id(layer)?, level, PixelRect { x, y, width, height }).map_err(js_err)?;
+        Ok(Uint8Array::from(bytes.as_slice()))
     }
     pub fn clip_dependents(&self, doc: &str, ids_json: &str) -> Result<String, JsError> {
         let ids = self.engine.clip_dependents(parse_id(doc)?, &Self::parse_ids(ids_json)?).map_err(js_err)?;
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -118,6 +118,21 @@ fn selection_regions(doc: &Document, clips: &SelectionClips, layer: Uuid, plane:
     clip.rect_on_grid(&grid.pixel_to_document(w, h), w, h).map(|rect| vec![Region { layer, plane, rect }]).unwrap_or_default()
 }
 
+/// Records in the lineage how a patch preview changed what the canvas shows of a layer's pixels: from
+/// the stored pixels to a patch, from one patch to the next (both rectangles), and back. The GPU then
+/// uploads only those rectangles (`pixels_delta`, `layer_region`); any other preview goes whole.
+fn record_preview(lineage: &mut Lineage, doc: &Document, old: Option<&PixelPreview>, new: Option<&PixelPreview>) {
+    let patch = |p: Option<&PixelPreview>| p.and_then(|p| match p.target { PreviewTarget::Patch(r) => Some((p.layer, p.revision, r)), _ => None });
+    let stored = |layer: Uuid| doc.layer(layer).map(|l| l.pixels_revision);
+    match (patch(old), patch(new)) {
+        (Some((a, from, r1)), Some((b, to, r2))) if a == b => lineage.record(a, Plane::Pixels, from, to, Some(r1.union(&r2))),
+        (old_patch, new_patch) => {
+            if let Some((layer, from, r)) = old_patch { if let Some(to) = stored(layer) { lineage.record(layer, Plane::Pixels, from, to, Some(r)); } }
+            if let Some((layer, to, r)) = new_patch { if let Some(from) = stored(layer) { lineage.record(layer, Plane::Pixels, from, to, Some(r)); } }
+        }
+    }
+}
+
 /// Gives each layer's new pixels, changed only within a reported region, the halvings its old pixels
 /// had, redone only there (`Raster::seed_halvings`): the GPU at a reduced zoom then uploads the region
 /// without halving the whole layer again.
@@ -267,11 +282,48 @@ impl Engine {
         let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         Ok(self.session(id)?.lineage.delta(layer, Plane::Pixels, from, l.pixels_revision))
     }
-    /// `pixels_delta` for the layer's mask, in the mask's own grid.
+    /// `pixels_delta` for the layer's mask as the canvas shows it, in the mask's own grid.
     pub fn mask_delta(&self, id: Uuid, layer: Uuid, from: u64) -> Result<Option<PixelRect>, CommandError> {
+        let doc = self.render_document(id)?;
+        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
+        Ok(self.session(id)?.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
+    }
+    /// The preview showing on the canvas, if any.
+    pub fn preview(&self, id: Uuid) -> Option<&PixelPreview> { self.sessions.get(&id).and_then(|s| s.preview.as_ref()) }
+    /// The layer's mask as the canvas shows it (a gradient's mask preview in its place).
+    pub fn mask_pixels(&self, id: Uuid, layer: Uuid) -> Result<Option<GrayRaster>, CommandError> {
+        let doc = self.render_document(id)?;
+        Ok(doc.layer(layer).ok_or(CommandError::NoLayer)?.mask.as_ref().map(|m| m.pixels.clone()))
+    }
+    /// The bytes of `rect` (in the raster's grid after `level` halvings) of the layer's pixels as the
+    /// canvas shows them: what the GPU's partial upload sends. Through a patch preview, the rectangle
+    /// is made from the stored pixels and the patch at full size and halved on its own, which equals
+    /// the same part of the whole patched raster halved (every block lies inside the rectangle).
+    pub fn layer_region(&self, id: Uuid, layer: Uuid, level: u32, rect: PixelRect) -> Result<Vec<u8>, CommandError> {
         let s = self.session(id)?;
-        let l = s.document.layer(layer).ok_or(CommandError::NoLayer)?;
-        Ok(s.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
+        let patch = s.preview.as_ref().filter(|p| p.layer == layer).and_then(|p| match p.target { PreviewTarget::Patch(r) => Some((r, &p.raster)), _ => None });
+        let stored = s.document.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone().ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?;
+        let f = 1u32 << level;
+        // Every level of the halving keeps whole 2 x 2 blocks while both sides stay above 1.
+        let whole_blocks = (stored.width >> level.saturating_sub(1)) > 1 && (stored.height >> level.saturating_sub(1)) > 1;
+        let raster = match patch {
+            Some((prect, praster)) if whole_blocks => {
+                let (x0, y0, w, h) = (rect.x * f, rect.y * f, rect.width * f, rect.height * f);
+                let mut data = stored.cropped(x0, y0, w, h).into_bytes();
+                for y in prect.y.max(y0)..(prect.y + prect.height).min(y0 + h) {
+                    let (from, to) = (prect.x.max(x0), (prect.x + prect.width).min(x0 + w));
+                    if from >= to { continue; }
+                    let src = (((y - prect.y) * prect.width + (from - prect.x)) * 4) as usize;
+                    let dst = (((y - y0) * w + (from - x0)) * 4) as usize;
+                    data[dst..dst + ((to - from) * 4) as usize].copy_from_slice(&praster.bytes()[src..src + ((to - from) * 4) as usize]);
+                }
+                let mut region = Raster::from_premultiplied(w, h, data);
+                for _ in 0..level { region = region.halved(); }
+                return Ok(region.into_bytes());
+            }
+            _ => self.layer_raster(id, layer, level)?.ok_or_else(|| CommandError::Argument("the layer has no pixels".into()))?,
+        };
+        Ok(raster.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes())
     }
 
     /// Whether a press at `at` lands inside a selection with something in it, by the winding rule:
@@ -284,11 +336,29 @@ impl Engine {
 
     /// The document as the canvas should show it: the stored one, or a copy with the open
     /// panel's preview substituted for one layer. Every render path reads this; `export_*` and
-    /// the ops do not, because a preview is not committed.
-    pub(crate) fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
+    /// the ops do not, because a preview is not committed. A patch preview keeps the stored pixels
+    /// here, under the preview's revision: what reads the bytes asks `render_bytes`.
+    pub(crate) fn render_document(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> { self.displayed(id, false) }
+    /// `render_document` with a patch preview drawn into the layer's pixels (made once per preview).
+    pub(crate) fn render_bytes(&self, id: Uuid) -> Result<std::borrow::Cow<'_, Document>, CommandError> { self.displayed(id, true) }
+    fn displayed(&self, id: Uuid, bytes: bool) -> Result<std::borrow::Cow<'_, Document>, CommandError> {
         let s = self.session(id)?;
         let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
         let mut doc = s.document.clone();
+        match &preview.target {
+            PreviewTarget::Mask(mask) => {
+                if let Some(m) = doc.layer_mut(preview.layer).and_then(|l| { l.mask_revision = preview.revision; l.mask.as_mut() }) { m.pixels = mask.clone(); }
+                return Ok(std::borrow::Cow::Owned(doc));
+            }
+            PreviewTarget::Patch(_) => {
+                if let Some(layer) = doc.layer_mut(preview.layer) {
+                    layer.pixels_revision = preview.revision;
+                    if bytes { layer.pixels = layer.pixels.as_ref().map(|stored| preview.patched(stored)); }
+                }
+                return Ok(std::borrow::Cow::Owned(doc));
+            }
+            PreviewTarget::Pixels => {}
+        }
         if let Some(layer) = doc.layer_mut(preview.layer) {
             // A preview may come from a reduced copy (preview.rs): effects, measured in the layer's
             // pixels, shrink with it, so they show at the size the committed layer will draw them.
@@ -321,10 +391,17 @@ impl Engine {
         let s = self.sessions.get_mut(&id).ok_or(CommandError::NoDocument)?;
         let layers: Vec<Uuid> = s.preview.iter().map(|p| p.layer).chain(request.iter().map(|r| r.layer())).collect();
         let clips = &self.clips;
-        s.preview = request.as_ref().and_then(|r| preview::compute_preview_with(&s.document, clips, r, revision));
+        let next = request.as_ref().and_then(|r| preview::compute_preview_with(&s.document, clips, r, revision));
+        record_preview(&mut s.lineage, &s.document, s.preview.as_ref(), next.as_ref());
+        s.preview = next;
         Ok(Dirty::pixels(layers))
     }
-    pub(crate) fn clear_preview(&mut self, id: Uuid) { if let Ok(s) = self.session_mut(id) { s.preview = None; } }
+    pub(crate) fn clear_preview(&mut self, id: Uuid) {
+        if let Ok(s) = self.session_mut(id) {
+            record_preview(&mut s.lineage, &s.document, s.preview.as_ref(), None);
+            s.preview = None;
+        }
+    }
 
     /// Runs `f` on a copy of the document; on success the copy replaces it and the original goes to history.
     /// `f` reads the selection's clip through the engine's cache (`SelectionClips`).
@@ -547,11 +624,11 @@ impl Engine {
         Ok(compositor::export_jpeg_preview_with(&self.session(id)?.document, quality, matte, max_side, &self.effects)?)
     }
     pub fn composite(&self, id: Uuid, region: Rect, width: u32, height: u32) -> Result<Raster, CommandError> {
-        Ok(compositor::composite_edit_with(&*self.render_document(id)?, None, region, width, height, &self.effects))
+        Ok(compositor::composite_edit_with(&*self.render_bytes(id)?, None, region, width, height, &self.effects))
     }
 
     pub fn render_plan(&self, id: Uuid, edit: Option<&PreviewEdit>) -> Result<RenderPlan, CommandError> { Ok(plan::render_plan(&*self.render_document(id)?, edit)) }
-    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit_with(&*self.render_document(id)?, edit, region, w, h, &self.effects)) }
+    pub fn composite_edit(&self, id: Uuid, edit: Option<&PreviewEdit>, region: Rect, w: u32, h: u32) -> Result<Raster, CommandError> { Ok(compositor::composite_edit_with(&*self.render_bytes(id)?, edit, region, w, h, &self.effects)) }
     pub fn clip_dependents(&self, id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, CommandError> { Ok(ops::hierarchy::clip_dependents(&self.session(id)?.document, ids)) }
     pub fn merge_action(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<&'static str>, CommandError> { Ok(ops::merge::merge_plan(&self.session(id)?.document, ids).map(|p| p.action)) }
     pub fn group_box(&self, id: Uuid, ids: &[Uuid]) -> Result<Option<LayerTransform>, CommandError> { Ok(ops::transform::group_box(&self.session(id)?.document, ids)) }
@@ -560,7 +637,7 @@ impl Engine {
 
     /// The layer's raster after `level` sharp halvings, through any open preview.
     pub fn layer_raster(&self, id: Uuid, layer: Uuid, level: u32) -> Result<Option<Raster>, CommandError> {
-        let doc = self.render_document(id)?;
+        let doc = self.render_bytes(id)?;
         let Some(mut raster) = doc.layer(layer).ok_or(CommandError::NoLayer)?.pixels.clone() else { return Ok(None); };
         for _ in 0..level.min(compositor::MAX_PREFILTER_LEVEL) {
             if raster.width <= 1 || raster.height <= 1 { break; }
@@ -575,7 +652,7 @@ impl Engine {
     /// call, so a caller that hands out a pointer into it keeps the raster itself (the wasm bridge's
     /// `prepare_draw_pixels`).
     pub fn draw_raster(&self, id: Uuid, layer: Uuid, level: u32, edit: Option<&PreviewEdit>) -> Result<Option<Raster>, CommandError> {
-        let doc = self.render_document(id)?;
+        let doc = self.render_bytes(id)?;
         let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         let raster = match effects_draw(l, edit) { Some(fx) => self.effects.image(l, &fx), None => l.pixels.clone() };
         let Some(mut raster) = raster else { return Ok(None); };
```

```diff
--- a/engine/src/lineage.rs
+++ b/engine/src/lineage.rs
@@ -29,6 +29,10 @@ fn planes(doc: &Document) -> HashMap<(Uuid, Plane), (u64, (u32, u32))> {
 }
 
 impl Lineage {
+    /// Records one change of a layer's buffer from revision `from` to `to`, within `rect` (None: whole).
+    pub fn record(&mut self, layer: Uuid, plane: Plane, from: u64, to: u64, rect: Option<PixelRect>) {
+        self.push(Change { layer, plane, from, to, rect });
+    }
     fn push(&mut self, change: Change) {
         if self.changes.len() == LINEAGE_LIMIT { self.changes.pop_front(); }
         self.changes.push_back(change);
```

```diff
--- a/engine/src/ops/raster_edit.rs
+++ b/engine/src/ops/raster_edit.rs
@@ -59,13 +59,6 @@ impl Paint {
             Paint::Gradient(g) => g.check(),
         }
     }
-    /// The straight RGBA painted at document point `p`, and the opacity it is painted at.
-    fn at(&self, p: Point) -> ([f64; 4], f64) {
-        match self {
-            Paint::Fill(c) => ([c[0], c[1], c[2], 1.0], 1.0),
-            Paint::Gradient(g) => (g.color_at(g.position(p)), g.opacity),
-        }
-    }
 }
 
 /// The grid a layer's edit is painted on: `width` x `height` pixels that `transform` places, with the
@@ -135,7 +128,8 @@ impl Ramp {
 
 /// Paints `paint` over `data` (premultiplied RGBA, or grey when `grey`) on a `width` x `height` grid
 /// that `transform` places: each pixel at its centre, inside the canvas, through `coverage` (the
-/// selection's, None for all), source-over at the paint's alpha times its opacity times the coverage,
+/// selection's, None for all), source-over at the paint's alpha times its opacity times the coverage
+/// (as a fraction of 255),
 /// each channel rounded half up.
 pub fn paint_grid(doc: &Document, data: &mut [u8], width: u32, height: u32, transform: &LayerTransform, coverage: Option<&GrayRaster>, paint: &Paint, grey: bool) {
     let m = transform.pixel_to_document(width, height);
@@ -147,20 +141,21 @@ pub fn paint_grid(doc: &Document, data: &mut [u8], width: u32, height: u32, tran
         Paint::Gradient(g) => (g.from, g.to, g.opacity),
     };
     let delta: [f64; 4] = std::array::from_fn(|c| to[c] - from[c]);
+    // The selection's coverage as a fraction, once for each of its 256 values.
+    let fraction: [f64; 256] = std::array::from_fn(|k| k as f64 / 255.0);
     for y in 0..height as usize {
         // The row's first pixel centre in the document, and the step one pixel to the right.
         let first = m.apply(Point { x: 0.5, y: y as f64 + 0.5 });
         let cover = coverage.map(|c| &c.bytes()[y * w..(y + 1) * w]);
         let line = &mut data[y * w * channels..(y + 1) * w * channels];
-        for x in 0..w {
+        for (x, px) in line.chunks_exact_mut(channels).enumerate() {
             let k = cover.map_or(255, |c| c[x]);
             if k == 0 { continue; }
             let (dx, dy) = (first.x + m.a * x as f64, first.y + m.b * x as f64);
             if dx < 0.0 || dy < 0.0 || dx >= cw || dy >= ch { continue; }
             let t = ramp.at(dx, dy);
-            let s = (from[3] + delta[3] * t) * opacity * k as f64 / 255.0;
+            let s = (from[3] + delta[3] * t) * opacity * fraction[k as usize];
             if s <= 0.0 { continue; }
-            let px = &mut line[x * channels..(x + 1) * channels];
             let keep = 1.0 - s;
             if grey {
                 px[0] = ((from[0] + delta[0] * t) * 255.0 * s + px[0] as f64 * keep + 0.5) as u8;
@@ -225,11 +220,17 @@ fn mask_on_layer_grid(doc: &mut Document, id: Uuid) -> Result<(), CommandError>
     Ok(())
 }
 
+/// Whether `paint_layer` would paint: the paint's values and the target (a preview asks first, and
+/// shows nothing the commit would refuse).
+pub fn paint_layer_check(doc: &Document, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
+    paint.check()?;
+    check_target(doc, id, mask)
+}
+
 /// Paints `paint` into layer `id`'s pixels (`mask` false) or its mask, as one edit: Fill and the
 /// Gradient's commit.
 pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
-    paint.check()?;
-    check_target(doc, id, mask)?;
+    paint_layer_check(doc, id, mask, paint)?;
     if mask {
         mask_on_layer_grid(doc, id)?;
         let layer = doc.layer(id).unwrap();
```

```diff
--- a/engine/src/preview.rs
+++ b/engine/src/preview.rs
@@ -12,11 +12,15 @@ pub enum PreviewRequest {
     /// (`COLOUR_DRAG_LIMIT`) and followed by an `Adjustment` request once input settles.
     DragAdjustment { #[serde(with = "ids::upper")] layer: Uuid, adjustment: LayerAdjustment },
     Filter { #[serde(with = "ids::upper")] layer: Uuid, params: FilterParams },
+    /// A gradient not yet applied (Phase 4b-1), on the layer's pixels or its mask: while `dragging`
+    /// from a copy at most `GRADIENT_DRAG_LIMIT` across, then at most `GRADIENT_SETTLED_LIMIT`; inside a
+    /// selection on pixels that already cover the canvas, at full size as a patch (`PATCH_LIMIT`).
+    Gradient { #[serde(with = "ids::upper")] layer: Uuid, #[serde(default)] mask: bool, gradient: GradientSpec, #[serde(default)] dragging: bool },
 }
 
 impl PreviewRequest {
     pub fn layer(&self) -> Uuid {
-        match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::DragAdjustment { layer, .. } | PreviewRequest::Filter { layer, .. } => *layer }
+        match self { PreviewRequest::Adjustment { layer, .. } | PreviewRequest::DragAdjustment { layer, .. } | PreviewRequest::Filter { layer, .. } | PreviewRequest::Gradient { layer, .. } => *layer }
     }
     /// Whether two requests compute the same pixels: the same layer and settings at the same
     /// effective limit (a Grain drag and a settled Grain are both full size).
@@ -25,6 +29,7 @@ impl PreviewRequest {
         let content = match (self, other) {
             (Adjustment { adjustment: a, .. } | DragAdjustment { adjustment: a, .. }, Adjustment { adjustment: b, .. } | DragAdjustment { adjustment: b, .. }) => a == b,
             (Filter { params: a, .. }, Filter { params: b, .. }) => a == b,
+            (Gradient { mask: a, gradient: g, .. }, Gradient { mask: b, gradient: h, .. }) => a == b && g == h,
             _ => false,
         };
         content && self.layer() == other.layer() && preview_limit(self) == preview_limit(other)
@@ -43,15 +48,42 @@ impl PreviewSource {
     }
 }
 
+/// What a preview stands in for: the layer's pixels (`raster`, placed by `transform`), a patch of them
+/// (`raster` is that rectangle of the stored pixels' grid, drawn over them), or the layer's mask.
+#[derive(Clone, Debug)]
+pub enum PreviewTarget { Pixels, Patch(PixelRect), Mask(GrayRaster) }
+
 /// The substituted pixels for one layer while a panel is open, the request that made them and
 /// what they were made from.
 #[derive(Clone, Debug)]
-pub struct PixelPreview { pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest, pub source: PreviewSource }
+pub struct PixelPreview {
+    pub layer: Uuid, pub raster: Raster, pub transform: LayerTransform, pub revision: u64, pub request: PreviewRequest, pub source: PreviewSource,
+    pub target: PreviewTarget,
+    /// A patch drawn over the stored pixels, made once and only when something reads the pixels whole
+    /// (the CPU compositor, a whole texture upload): the GPU's partial path reads the patch alone.
+    patched: std::sync::OnceLock<Raster>,
+}
 
 impl PixelPreview {
     /// Whether `request`, on a document that is now `source`, would compute exactly these pixels
     /// again, so they can be kept.
     pub fn answers(&self, request: &PreviewRequest, source: &PreviewSource) -> bool { self.request.same_output(request) && self.source == *source }
+    fn new(layer: Uuid, raster: Raster, transform: LayerTransform, revision: u64, request: &PreviewRequest, source: PreviewSource, target: PreviewTarget) -> PixelPreview {
+        PixelPreview { layer, raster, transform, revision, request: request.clone(), source, target, patched: std::sync::OnceLock::new() }
+    }
+    /// The stored pixels with this preview's patch drawn over them, made on first use.
+    pub fn patched(&self, stored: &Raster) -> Raster {
+        let PreviewTarget::Patch(rect) = self.target else { return self.raster.clone() };
+        self.patched.get_or_init(|| {
+            let mut data = stored.bytes().to_vec();
+            let row = rect.width as usize * 4;
+            for y in 0..rect.height as usize {
+                let at = ((rect.y as usize + y) * stored.width as usize + rect.x as usize) * 4;
+                data[at..at + row].copy_from_slice(&self.raster.bytes()[y * row..(y + 1) * row]);
+            }
+            Raster::from_premultiplied(stored.width, stored.height, data)
+        }).clone()
+    }
 }
 
 // Preview sizes. A colour adjustment's preview costs about 0.1 us per preview pixel in release
@@ -71,6 +103,15 @@ pub const COLOUR_DRAG_LIMIT: u32 = 512;
 pub const COLOUR_PREVIEW_LIMIT: u32 = 4096;
 /// The longest side a filter previews from, as the Mac's `FilterEdit.previewLimit`.
 pub const FILTER_PREVIEW_LIMIT: u32 = 2048;
+/// The longest side a gradient previews from while its line is being dragged (ruling OQ9).
+pub const GRADIENT_DRAG_LIMIT: u32 = 1024;
+/// The longest side a gradient previews from once its line rests (ruling OQ9): the filters' 2048,
+/// so the settled preview stays near 100 ms at 24 and 100 MP; the applied gradient is full size.
+pub const GRADIENT_SETTLED_LIMIT: u32 = 2048;
+/// The most pixels a gradient's patch preview may hold (about 724 x 724; ruling OQ10): painting it
+/// takes about 25 ms in the release wasm, inside a drag tick's 50. Larger selections preview from a
+/// reduced copy as a whole-layer gradient does.
+pub const PATCH_LIMIT: u64 = 1 << 19;
 
 /// Previews render from a copy no larger than this on its longest side. Grain and Add Noise are
 /// made at full size, dragged or not: their pattern is per pixel, and a small copy enlarged
@@ -80,6 +121,7 @@ pub fn preview_limit(request: &PreviewRequest) -> u32 {
         PreviewRequest::Adjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_PREVIEW_LIMIT },
         PreviewRequest::DragAdjustment { adjustment, .. } => if matches!(adjustment.kind, AdjustmentKind::Grain | AdjustmentKind::AddNoise) { u32::MAX } else { COLOUR_DRAG_LIMIT },
         PreviewRequest::Filter { params, .. } => if matches!(params, FilterParams::AddNoise { .. }) { u32::MAX } else { FILTER_PREVIEW_LIMIT },
+        PreviewRequest::Gradient { dragging, .. } => if *dragging { GRADIENT_DRAG_LIMIT } else { GRADIENT_SETTLED_LIMIT },
     }
 }
 
@@ -104,6 +146,9 @@ pub fn compute_preview(doc: &Document, request: &PreviewRequest, revision: u64)
 /// `compute_preview` with the selection's clip from `clips`: the engine's, so every tick of a drag
 /// under one selection reuses one clip (final review F1).
 pub fn compute_preview_with(doc: &Document, clips: &SelectionClips, request: &PreviewRequest, revision: u64) -> Option<PixelPreview> {
+    if let PreviewRequest::Gradient { layer, mask, gradient, .. } = request {
+        return gradient_preview(doc, clips, request, *layer, *mask, gradient, revision);
+    }
     let layer = doc.layer(request.layer())?;
     let raster = layer.pixels.as_ref()?;
     let limit = preview_limit(request);
@@ -116,7 +161,7 @@ pub fn compute_preview_with(doc: &Document, clips: &SelectionClips, request: &Pr
             // Grain and the tonal kernels read document space, which the reduced grid still covers.
             let units = layer.transform.size.width / source.width.max(1) as f64;
             let result = adjust::apply::apply_adjustment(&source, adjustment, layer.transform.origin, units, coverage.as_ref());
-            Some(PixelPreview { layer: layer.id, raster: result, transform: layer.transform, revision, request: request.clone(), source: made_from })
+            Some(PixelPreview::new(layer.id, result, layer.transform, revision, request, made_from, PreviewTarget::Pixels))
         }
         PreviewRequest::Filter { params, .. } => {
             let params = params.normalized();
@@ -130,7 +175,85 @@ pub fn compute_preview_with(doc: &Document, clips: &SelectionClips, request: &Pr
             let coverage = ops::adjust::edit_coverage(doc, clips, &placed, grid.width, grid.height).ok()?;
             let filtered = adjust::filters::apply_filter(&grid, &scaled);
             let result = match coverage { Some(c) => adjust::apply::blend_by_coverage(&filtered, &grid, &c), None => filtered };
-            Some(PixelPreview { layer: layer.id, raster: result, transform: placed, revision, request: request.clone(), source: made_from })
+            Some(PixelPreview::new(layer.id, result, placed, revision, request, made_from, PreviewTarget::Pixels))
+        }
+        PreviewRequest::Gradient { .. } => None,
+    }
+}
+
+/// The fewest halvings that bring `width` x `height` to at most `limit` on its longer side.
+fn level_for(width: u32, height: u32, limit: u32) -> u32 {
+    let mut level = 0;
+    while (width >> level).max(height >> level) > limit && (width >> level) > 1 && (height >> level) > 1 { level += 1; }
+    level
+}
+
+/// A gradient's preview (Phase 4b-1). On a mask: the mask reduced to the request's limit, painted.
+/// On pixels inside a selection whose rectangle is small (`PATCH_LIMIT`), when the layer already
+/// covers the canvas and draws no effects: that rectangle at full size, as a patch. Otherwise the
+/// layer's grid grown to the canvas (`raster_edit::image_grid`), reduced to the limit with its
+/// origin on the reduced pixels, the layer's own halvings placed in it, painted; never trimmed.
+fn gradient_preview(doc: &Document, clips: &SelectionClips, request: &PreviewRequest, id: Uuid, mask: bool, gradient: &GradientSpec, revision: u64) -> Option<PixelPreview> {
+    use ops::raster_edit::{image_grid, paint_grid, Paint};
+    let layer = doc.layer(id)?;
+    let paint = Paint::Gradient(gradient.clone());
+    // What the commit would refuse, the preview does not show.
+    if ops::raster_edit::paint_layer_check(doc, id, mask, &paint).is_err() { return None; }
+    let made_from = PreviewSource::of(doc, id);
+    let limit = preview_limit(request);
+    if mask {
+        // The grid the commit paints: a placed mask's own, else the layer's pixels (a covering mask
+        // of another size is stretched onto it), reduced; the mask sampled onto it, nearest.
+        let m = layer.mask.as_ref()?;
+        let placement = m.placement.unwrap_or(layer.transform);
+        let (gw, gh) = if m.placement.is_some() { (m.pixels.width, m.pixels.height) } else {
+            layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height))
+        };
+        let level = level_for(gw, gh, limit);
+        let (rw, rh) = ((gw >> level).max(1), (gh >> level).max(1));
+        let (mw, mh) = (m.pixels.width as u64, m.pixels.height as u64);
+        let mut data: Vec<u8> = (0..rh as u64).flat_map(|y| (0..rw as u64).map(move |x| (x, y)))
+            .map(|(x, y)| m.pixels.bytes()[((y * mh / rh as u64) * mw + x * mw / rw as u64) as usize]).collect();
+        let coverage = ops::adjust::edit_coverage(doc, clips, &placement, rw, rh).ok()?;
+        paint_grid(doc, &mut data, rw, rh, &placement, coverage.as_ref(), &paint, true);
+        let shown = GrayRaster::from_bytes(rw, rh, data);
+        let pixels = layer.pixels.clone().unwrap_or_else(|| Raster::new_transparent(1, 1));
+        return Some(PixelPreview::new(id, pixels, layer.transform, revision, request, made_from, PreviewTarget::Mask(shown)));
+    }
+    let grid = image_grid(doc, layer).ok()?;
+    let stored = layer.pixels.as_ref();
+    // A patch: the layer covers the canvas already, the selection bounds a small rectangle of it.
+    if let (Some(pixels), Some(clip)) = (stored, clips.clip(doc)) {
+        let covers = (grid.width, grid.height) == (pixels.width, pixels.height);
+        let rect = clip.rect_on_grid(&layer.transform.pixel_to_document(pixels.width, pixels.height), pixels.width, pixels.height)?;
+        if covers && layer.extra.effects.is_none() && (rect.width as u64) * (rect.height as u64) <= PATCH_LIMIT && !rect.is_empty() {
+            let placed = ops::adjust::placed_like(&layer.transform, pixels.width, pixels.height, rect.width, rect.height, -(rect.x as f64), -(rect.y as f64));
+            let mut data = pixels.cropped(rect.x, rect.y, rect.width, rect.height).into_bytes();
+            let coverage = ops::adjust::edit_coverage(doc, clips, &placed, rect.width, rect.height).ok()?;
+            paint_grid(doc, &mut data, rect.width, rect.height, &placed, coverage.as_ref(), &paint, false);
+            let patch = Raster::from_premultiplied(rect.width, rect.height, data);
+            return Some(PixelPreview::new(id, patch, layer.transform, revision, request, made_from, PreviewTarget::Patch(rect)));
+        }
+    }
+    // Reduced: the grown grid at `level` halvings, its origin rounded out so the layer's own halved
+    // pixels land on whole reduced pixels.
+    let level = level_for(grid.width, grid.height, limit);
+    let f = 1u32 << level;
+    let (left, top) = (grid.x.div_ceil(f), grid.y.div_ceil(f));
+    let (w, h) = stored.map_or((grid.width - grid.x, grid.height - grid.y), |p| (p.width, p.height));
+    let (rw, rh) = (left + (grid.width - grid.x).div_ceil(f), top + (grid.height - grid.y).div_ceil(f));
+    let placed = ops::adjust::placed_like(&layer.transform, w, h, rw * f, rh * f, (left * f) as f64, (top * f) as f64);
+    let mut data = vec![0u8; rw as usize * rh as usize * 4];
+    if let Some(p) = stored {
+        let mut halved = p.clone();
+        for _ in 0..level { halved = halved.halved(); }
+        let row = halved.width.min(rw - left) as usize * 4;
+        for y in 0..halved.height.min(rh - top) as usize {
+            let at = ((y + top as usize) * rw as usize + left as usize) * 4;
+            data[at..at + row].copy_from_slice(&halved.bytes()[y * halved.width as usize * 4..y * halved.width as usize * 4 + row]);
         }
     }
+    let coverage = ops::adjust::edit_coverage(doc, clips, &placed, rw, rh).ok()?;
+    paint_grid(doc, &mut data, rw, rh, &placed, coverage.as_ref(), &paint, false);
+    Some(PixelPreview::new(id, Raster::from_premultiplied(rw, rh, data), placed, revision, request, made_from, PreviewTarget::Pixels))
 }
```


- [ ] **Step 4: Run the tests and watch them pass**

Engine: 498 passed, 8 ignored (+5, +1 ignored). `pnpm wasm:dev`; `pnpm test`: 167 (one test changed); `pnpm build`; `pnpm e2e`: 139 passed, 9 skipped. Timings (release wasm, Edge, `-g "gradient previews"`), a tick being the engine's preview plus the frame that draws it: while dragging 25-31 ms at 24 and 100 MP, at fit and at 1:1 (budget 50); settled 78-95 ms (budget 150); a patch in a 700 px selection 31-33 ms (budget 50), with only partial uploads. Natively: 9-10 ms, 40-43 ms and 13-14 ms. Measured and ruled out on the way (ruling OQ8): settled at 4096, 355-426 ms; a 1 MP patch, 57-60 ms.

- [ ] **Step 5: Prove it bites**

(1) In `Engine::layer_region`, copy nothing of the patch into the region: the e2e "the patch is the applied gradient" fails (measured). Restore. (2) Lower `PATCH_LIMIT` to `1 << 18`: `a_large_selection_or_a_layer_that_must_grow_previews_whole` fails at the 720 x 720 patch (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/gradient-preview.spec.ts engine/tests/gradient_preview.rs
git commit -m "feat: gradient previews, reduced while dragged, as patches in small selections, and on masks" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/gl-renderer.ts app/src/canvas/layer-textures.ts app/src/engine/client.ts app/src/engine/types.ts app/tests/e2e/gradient-preview.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/partial-upload.test.ts engine-wasm/src/lib.rs engine/src/engine.rs engine/src/lineage.rs engine/src/ops/raster_edit.rs engine/src/preview.rs engine/tests/gradient_preview.rs engine/tests/perf_4b1.rs
```


---

### Task 10: Shapes on new layers, with the Mac's shape record

`Command::AddShape { shape, color }` draws a `ShapeSpec` - `Rectangle { rect, cornerRadius }`, `Ellipse { rect }` or `Line { start, end, width }` - filled with `color` on a new layer above the active one (ruling OQ11). The box is the rectangle, or a line's ends grown by half its width; the layer is `Int(width)` x `Int(height)` at the box's origin, drawn by the selection's exact-area rasteriser from Core Graphics' outlines (a quarter-circle Bezier per rounded corner, the ellipse's four, a capsule for a line), the colour premultiplied by the coverage. The layer is named "<Kind> <n>", keeps the selection, and carries the Mac's `shape` record so the Mac can redraw it. A box under a pixel on a side is an argument error (the app sends none); past the project's pixels, the Mac's words.

**Files:**
- Create: `engine/src/ops/shape.rs`
- Modify: `engine/src/command.rs`, `engine/src/engine.rs`, `engine/src/lib.rs`, `engine/src/ops/mod.rs`, `engine/src/ops/layers.rs` (`insert_above_active`), `engine/src/selection/geometry.rs` (`cubic`), `app/src/engine/types.ts` (`ShapeSpec`, `AddShape`)
- Create tests: `engine/tests/shapes.rs`; modify `engine/tests/perf_4b1.rs`

**Interfaces:**
- Produces: `ShapeKind`, `ShapeSpec` (JSON tagged by `kind`), `SHAPE_TOO_LARGE`, `shape_raster`, `shape_record`, `next_shape_name`, `add_shape`, `ops::layers::insert_above_active`, `selection::geometry::cubic`.

- [ ] **Step 1: Write the tests**

Ported from the Mac's ShapeToolTests with their numbers: a rectangle fills a new layer with the colour as one undo step and keeps the selection ((10, 10) 30 x 20; pixels (25, 20), (10, 10), (39, 29) red and opaque; (9, 20), (40, 20), (25, 30) clear; "Rectangle 2" next; two undos leave "Layer 1"); an ellipse leaves its box's corners clear; rounded rectangles follow the radius and clamp to a pill (radius 8 cuts (10, 10) and (11, 11) but not (13, 13); 500 on a 40 x 20 box is 10). And against independent counts: every pixel of a 37 x 23 ellipse, a 40 x 26 rectangle rounded by 9 and a 5 px line from (12.5, 60.25) to (47.75, 71) is within 4 levels of a 32 x 32-sample area count; a 1 px line's end pixels hold 0.5 + pi / 8 of their area (228; a square cap would fill them, a butt cap halve them). The record: the probe's Line 3 box (255, 120) 155 x 95 and its ends as fractions, whole numbers written as Swift writes them, and no line fields on a rectangle. Names skip names in use and the layer goes above the active one; a click, a 0.5 px line and a shape past the project's pixels make nothing; painting over a shape drops its record, which a save otherwise keeps.

```diff
--- a/engine/tests/perf_4b1.rs
+++ b/engine/tests/perf_4b1.rs
@@ -117,3 +117,23 @@ fn gradient_previews_dragged_settled_and_patched_at_24_and_100_mp() {
         assert!(e.preview(id).is_some());
     }
 }
+
+#[test]
+#[ignore]
+fn a_shape_over_the_whole_canvas_at_24_and_100_mp() {
+    for (label, w, h) in [("24 MP", 6000u32, 4000u32), ("100 MP", 10000, 10000)] {
+        // An empty document: the shape's own pixels are the project's only ones.
+        let mut e = Engine::new();
+        let id = e.new_document(w, h, false).unwrap();
+        let mut times = Vec::new();
+        for (name, shape) in [("rectangle", ShapeSpec::Rectangle { rect: Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 }, corner_radius: 400.0 }),
+            ("ellipse", ShapeSpec::Ellipse { rect: Rect { x: 0.0, y: 0.0, width: w as f64, height: h as f64 } })] {
+            let t = Instant::now();
+            run(&mut e, id, Command::AddShape { shape, color: [0.2, 0.4, 0.6] });
+            times.push(format!("{name} {:.0} ms", ms(t)));
+            e.undo(id).unwrap();
+        }
+        println!("{label}: a shape over the canvas: {}", times.join(", "));
+        assert_eq!(e.state(id).unwrap().undo_depth, 0);
+    }
+}
```

Create `engine/tests/shapes.rs`:

```rust
//! The Shape tool's layers (Phase 4b-1), ported from the Mac's ShapeToolTests with their numbers,
//! and the shapes' edges against an independent area count.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect { Rect { x, y, width, height } }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
const RED: [f64; 3] = [1.0, 0.0, 0.0];

/// The Mac test's session: 100 x 80 with one empty layer, "Layer 1".
fn session() -> (Engine, Uuid) {
    let mut e = Engine::new();
    let id = e.new_document(100, 80, true).unwrap();
    (e, id)
}
fn shape(e: &mut Engine, id: Uuid, shape: ShapeSpec, color: [f64; 3]) { run(e, id, Command::AddShape { shape, color }); }
/// The flattened document's pixel: (red, alpha), premultiplied as the Mac's test reads it.
fn pixel(e: &Engine, id: Uuid, x: u32, y: u32) -> (u8, u8) {
    let px = e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0);
    (px[0], px[3])
}
fn names(e: &Engine, id: Uuid) -> Vec<String> { e.state(id).unwrap().layers.iter().map(|l| l.name.clone()).collect() }

#[test]
fn a_rectangle_fills_a_new_layer_with_the_colour_as_one_undo_step_and_keeps_the_selection() {
    // ShapeToolTests.rectangleFillsANewLayerWithTheForegroundColorAsOneUndoStep.
    let (mut e, id) = session();
    run(&mut e, id, Command::SelectAll);
    let depth = e.state(id).unwrap().undo_depth;
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 30.0, 20.0), corner_radius: 0.0 }, RED);
    let s = e.state(id).unwrap();
    assert_eq!(names(&e, id), ["Layer 1", "Rectangle 1"]);
    let layer = s.layers.iter().find(|l| l.name == "Rectangle 1").unwrap();
    assert_eq!(s.active_layer_id, Some(layer.id));
    assert_eq!(s.undo_depth, depth + 1);
    assert_eq!(Command::AddShape { shape: ShapeSpec::Ellipse { rect: rect(0.0, 0.0, 1.0, 1.0) }, color: RED }.action_name(), "Ellipse", "the undo name is the kind");
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width, layer.transform.size.height), (10.0, 10.0, 30.0, 20.0));
    assert_eq!((layer.pixels_width, layer.pixels_height), (30, 20));
    assert!(s.selection.is_some(), "unlike Paste, drawing a shape keeps the selection");
    for (x, y) in [(25, 20), (10, 10), (39, 29)] { assert_eq!(pixel(&e, id, x, y), (255, 255), "({x}, {y})"); }
    for (x, y) in [(9, 20), (40, 20), (25, 30)] { assert_eq!(pixel(&e, id, x, y).1, 0, "({x}, {y})"); }
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(60.0, 10.0, 10.0, 10.0), corner_radius: 0.0 }, RED);
    assert_eq!(names(&e, id).last().unwrap(), "Rectangle 2");
    e.undo(id).unwrap();
    e.undo(id).unwrap();
    assert_eq!(names(&e, id), ["Layer 1"]);
}

#[test]
fn an_ellipse_leaves_its_corners_clear() {
    // ShapeToolTests.ellipseLeavesItsCornersClearWithShiftCircleAndOptionFromCenter: the drag from
    // (50, 40) to (60, 45) with Shift and Option is the box (40, 30, 20, 20) (the app's DragBox).
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Ellipse { rect: rect(40.0, 30.0, 20.0, 20.0) }, RED);
    let layer = e.state(id).unwrap().layers[1].clone();
    assert_eq!(layer.name, "Ellipse 1");
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width), (40.0, 30.0, 20.0));
    assert_eq!(pixel(&e, id, 50, 40), (255, 255));
    assert!(pixel(&e, id, 41, 40).1 > 0 && pixel(&e, id, 50, 31).1 > 0);
    assert_eq!(pixel(&e, id, 40, 30).1, 0, "outside the circle, inside its box");
    assert_eq!(pixel(&e, id, 59, 49).1, 0);
}

#[test]
fn rounded_rectangles_follow_the_radius_and_clamp_to_a_pill() {
    // ShapeToolTests.roundedRectanglesFollowTheRadiusAndClampToAPill.
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 40.0, 30.0), corner_radius: 8.0 }, RED);
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(55.0, 50.0, 40.0, 20.0), corner_radius: 500.0 }, RED);
    assert_eq!(pixel(&e, id, 10, 10).1, 0, "the corner is cut away");
    assert_eq!(pixel(&e, id, 11, 11).1, 0);
    assert_eq!(pixel(&e, id, 13, 13).1, 255, "inside the rounded corner");
    assert_eq!(pixel(&e, id, 30, 10).1, 255, "straight edges stay full");
    assert_eq!(pixel(&e, id, 30, 25), (255, 255));
    assert_eq!(pixel(&e, id, 55, 50).1, 0, "the pill's corner is round");
    assert_eq!(pixel(&e, id, 75, 60), (255, 255));
    assert_eq!(e.state(id).unwrap().layers.len(), 3);
    // Clamped to 10 (half of 20): the record keeps what was asked, as the Mac's does.
    let doc = e.document(id).unwrap();
    assert_eq!(doc.layers[2].extra.shape.as_ref().unwrap()["cornerRadius"], serde_json::json!(500));
}

/// The fraction of pixel (`x`, `y`) inside `inside`, counted on a 32 x 32 grid of sample points:
/// independent of the engine's outlines and rasteriser.
fn area(x: u32, y: u32, inside: impl Fn(f64, f64) -> bool) -> f64 {
    let n = 32;
    let mut count = 0;
    for j in 0..n { for i in 0..n {
        if inside(x as f64 + (i as f64 + 0.5) / n as f64, y as f64 + (j as f64 + 0.5) / n as f64) { count += 1; }
    }}
    count as f64 / (n * n) as f64
}

#[test]
fn every_edge_pixel_is_the_area_the_shape_covers() {
    // A 37 x 23 ellipse at (5, 7), a 40 x 26 rectangle rounded by 9 at (50, 40), and a line 5 wide
    // from (12.5, 60.25) to (47.75, 71), each alone in a 100 x 80 document; every pixel of its
    // layer against the area count of the true curve (the flattening and the count: 4 levels).
    let cases: Vec<(ShapeSpec, Box<dyn Fn(f64, f64) -> bool>)> = vec![
        (ShapeSpec::Ellipse { rect: rect(5.0, 7.0, 37.0, 23.0) }, Box::new(|x, y| {
            let (u, v) = ((x - 23.5) / 18.5, (y - 18.5) / 11.5);
            u * u + v * v <= 1.0
        })),
        (ShapeSpec::Rectangle { rect: rect(50.0, 40.0, 40.0, 26.0), corner_radius: 9.0 }, Box::new(|x, y| {
            if !(50.0..=90.0).contains(&x) || !(40.0..=66.0).contains(&y) { return false; }
            let cx = x.clamp(59.0, 81.0);
            let cy = y.clamp(49.0, 57.0);
            (x - cx).hypot(y - cy) <= 9.0
        })),
        (ShapeSpec::Line { start: p(12.5, 60.25), end: p(47.75, 71.0), width: 5.0 }, Box::new(|x, y| {
            let (ax, ay, bx, by) = (12.5, 60.25, 47.75, 71.0);
            let t = (((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / ((bx - ax).powi(2) + (by - ay).powi(2))).clamp(0.0, 1.0);
            (x - (ax + t * (bx - ax))).hypot(y - (ay + t * (by - ay))) <= 2.5
        })),
    ];
    for (spec, inside) in cases {
        let (mut e, id) = session();
        shape(&mut e, id, spec.clone(), [0.0, 0.0, 1.0]);
        let layer = e.document(id).unwrap().layers[1].clone();
        let pixels = layer.pixels.unwrap();
        let (ox, oy) = (layer.transform.origin.x, layer.transform.origin.y);
        let (mut worst, mut edges) = (0u8, 0);
        for y in 0..pixels.height { for x in 0..pixels.width {
            // The layer's pixel (x, y) covers document [ox + x, ox + x + 1): count there.
            let expected = area(0, 0, |u, v| inside(ox + x as f64 + u, oy + y as f64 + v));
            let want = (expected * 255.0).round() as u8;
            let got = pixels.pixel(x, y)[3];
            if want != 0 && want != 255 { edges += 1; }
            worst = worst.max(got.abs_diff(want));
            // Blue alone, premultiplied: the colour is the coverage.
            assert_eq!(pixels.pixel(x, y)[2], got);
        }}
        assert!(worst <= 4, "{spec:?}: worst {worst}");
        assert!(edges > 20, "{spec:?}: the fixture has soft edges to compare ({edges})");
    }
}

#[test]
fn a_line_lies_in_its_ends_box_grown_by_half_its_width_with_its_ends_as_fractions() {
    // Line 3 of the shapes probe (engine/tests/mac_probes.rs): 15 wide from (262.5, 127.5) to
    // (402.5, 207.5), in a document large enough.
    let mut e = Engine::new();
    let id = e.new_document(440, 280, false).unwrap();
    shape(&mut e, id, ShapeSpec::Line { start: p(262.5, 127.5), end: p(402.5, 207.5), width: 15.0 }, [0.0, 128.0 / 255.0, 0.0]);
    let doc = e.document(id).unwrap();
    let layer = doc.layers.last().unwrap();
    assert_eq!(layer.name, "Line 1");
    // (262.5 - 7.5, 127.5 - 7.5) and 140 + 15 by 80 + 15.
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.transform.size.width, layer.transform.size.height), (255.0, 120.0, 155.0, 95.0));
    // The record as the probe writes it (Swift's whole numbers without a fraction): the ends at
    // 7.5 / 155 and 7.5 / 95 from the box's corners.
    let expected = serde_json::json!({ "kind": "Line", "red": 0, "green": 128.0 / 255.0, "blue": 0, "cornerRadius": 0, "lineWidth": 15,
        "start": [7.5 / 155.0, 7.5 / 95.0], "end": [147.5 / 155.0, 87.5 / 95.0] });
    assert_eq!(layer.extra.shape.as_ref().unwrap(), &expected);
    // A rectangle's record carries no line fields.
    let mut e2 = Engine::new();
    let id2 = e2.new_document(50, 50, false).unwrap();
    shape(&mut e2, id2, ShapeSpec::Rectangle { rect: rect(1.0, 2.0, 30.0, 20.0), corner_radius: 2.5 }, [0.25, 0.5, 1.0]);
    assert_eq!(e2.document(id2).unwrap().layers.last().unwrap().extra.shape.as_ref().unwrap(),
        &serde_json::json!({ "kind": "Rectangle", "red": 0.25, "green": 0.5, "blue": 1, "cornerRadius": 2.5 }));
}

#[test]
fn a_line_of_width_one_is_a_one_pixel_row_with_round_ends() {
    // Line 1 of the probe: 1 wide from (20.5, 120.5) to (140.5, 120.5): a 121 x 1 layer at (20, 120).
    let mut e = Engine::new();
    let id = e.new_document(200, 200, false).unwrap();
    shape(&mut e, id, ShapeSpec::Line { start: p(20.5, 120.5), end: p(140.5, 120.5), width: 1.0 }, [0.0, 0.0, 0.0]);
    let layer = e.document(id).unwrap().layers.last().unwrap().clone();
    assert_eq!((layer.transform.origin.x, layer.transform.origin.y, layer.pixels.as_ref().unwrap().width, layer.pixels.as_ref().unwrap().height), (20.0, 120.0, 121, 1));
    let px = layer.pixels.unwrap();
    // Full along the line; each end pixel holds half of a half-pixel cap plus half a pixel of line:
    // 0.5 + pi / 8 of it (0.8927 = 228), where a square cap would fill it and a butt cap halve it.
    assert_eq!(px.pixel(60, 0)[3], 255);
    for x in [0, 120] { assert!(px.pixel(x, 0)[3].abs_diff(228) <= 3, "end pixel {x}: {:?}", px.pixel(x, 0)); }
}

#[test]
fn a_shape_is_named_past_the_names_taken_and_placed_above_the_active_layer() {
    let (mut e, id) = session();
    run(&mut e, id, Command::AddBlankLayer); // "Layer 2", active, on top
    let bottom = e.state(id).unwrap().layers[0].id;
    run(&mut e, id, Command::RenameLayer { id: bottom, name: "Ellipse 1".into() });
    run(&mut e, id, Command::SetActiveLayer { id: Some(bottom) });
    shape(&mut e, id, ShapeSpec::Ellipse { rect: rect(1.0, 1.0, 5.0, 5.0) }, RED);
    assert_eq!(names(&e, id), ["Ellipse 1", "Ellipse 2", "Layer 2"], "above the active layer, not on top; the name taken skipped");
}

#[test]
fn a_click_or_a_shape_too_large_makes_nothing() {
    let (mut e, id) = session();
    let depth = e.state(id).unwrap().undo_depth;
    // Under a pixel on a side: a click, or a line along an axis of width under 1 (the Mac's
    // `rect.width >= 1, rect.height >= 1`).
    for spec in [ShapeSpec::Rectangle { rect: rect(20.0, 20.0, 0.0, 0.0), corner_radius: 0.0 },
        ShapeSpec::Ellipse { rect: rect(20.0, 20.0, 30.0, 0.5) },
        ShapeSpec::Line { start: p(10.0, 10.0), end: p(50.0, 10.0), width: 0.5 }] {
        assert!(matches!(e.execute(id, Command::AddShape { shape: spec, color: RED }), Err(CommandError::Argument(_))));
    }
    // Past the project's 100 megapixels: the Mac's words.
    let big = ShapeSpec::Rectangle { rect: rect(0.0, 0.0, 20_000.0, 6_000.0), corner_radius: 0.0 };
    match e.execute(id, Command::AddShape { shape: big, color: RED }) {
        Err(CommandError::Refused(m)) => assert_eq!(m, SHAPE_TOO_LARGE),
        other => panic!("{other:?}"),
    }
    assert_eq!(e.state(id).unwrap().undo_depth, depth);
    assert_eq!(e.state(id).unwrap().layers.len(), 1);
}

#[test]
fn painting_over_a_shape_layer_drops_its_record_and_a_save_keeps_it_otherwise() {
    let (mut e, id) = session();
    shape(&mut e, id, ShapeSpec::Rectangle { rect: rect(10.0, 10.0, 30.0, 20.0), corner_radius: 4.0 }, RED);
    let layer = e.state(id).unwrap().layers[1].id;
    let files = e.save_package(id).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&files.manifest_json).unwrap();
    let saved = manifest["layers"].as_array().unwrap().iter().find(|l| l["name"] == "Rectangle 1").unwrap();
    assert_eq!(saved["shape"]["cornerRadius"], serde_json::json!(4), "the Mac reads the record from the manifest");
    run(&mut e, id, Command::Fill { id: layer, mask: false, color: [0.0, 1.0, 0.0] });
    assert!(e.document(id).unwrap().layers[1].extra.shape.is_none(), "no longer the shape the Mac would redraw");
}
```


- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test shapes`: does not compile (`ShapeSpec`, `Command::AddShape`).

- [ ] **Step 3: Implement**

```diff
--- a/app/src/engine/types.ts
+++ b/app/src/engine/types.ts
@@ -80,6 +80,13 @@ export interface GradientSpec {
   from: [number, number, number, number]; to: [number, number, number, number]; opacity: number;
 }
 
+/** What the Shape tool made (engine `ShapeSpec`): a whole-pixel box for a rectangle or an ellipse;
+ * a line's two ends and its width. */
+export type ShapeSpec =
+  | { kind: "Rectangle"; rect: PixelRect; cornerRadius: number }
+  | { kind: "Ellipse"; rect: PixelRect }
+  | { kind: "Line"; start: PointTuple; end: PointTuple; width: number };
+
 export type PreviewRequest =
   | { preview: "Adjustment"; layer: string; adjustment: LayerAdjustment }
   /** The same while a slider moves: previewed from a smaller copy until input settles. */
@@ -213,7 +220,8 @@ export type Command =
   | { type: "ClearSelectedPixels"; id: string; mask: boolean }
   | { type: "AddMaskFromSelection"; id: string; revealing: boolean }
   | { type: "Fill"; id: string; mask: boolean; color: [number, number, number] }
-  | { type: "Gradient"; id: string; mask: boolean; gradient: GradientSpec };
+  | { type: "Gradient"; id: string; mask: boolean; gradient: GradientSpec }
+  | { type: "AddShape"; shape: ShapeSpec; color: [number, number, number] };
 
 export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }
 /** A rectangle of a layer's pixel grid (or its mask's), in whole pixels (engine `PixelRect`). */
```

```diff
--- a/engine/src/command.rs
+++ b/engine/src/command.rs
@@ -1,4 +1,4 @@
-use crate::{ids, AdjustmentKind, BlendMode, FilterParams, GradientSpec, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape, WandSettings};
+use crate::{ids, AdjustmentKind, BlendMode, FilterParams, GradientSpec, LayerAdjustment, LayerTransform, Point, Sampling, SelectionMode, SelectionShape, ShapeSpec, WandSettings};
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
 
@@ -72,6 +72,8 @@ pub enum Command {
     Fill { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool, color: [f64; 3] },
     /// Paint a gradient over the layer's pixels or its mask, inside the selection.
     Gradient { #[serde(with = "ids::upper")] id: Uuid, #[serde(default)] mask: bool, gradient: GradientSpec },
+    /// A shape filled with `color` on a new layer above the active one (the Shape tool).
+    AddShape { shape: ShapeSpec, color: [f64; 3] },
 }
 
 impl Command {
@@ -144,6 +146,8 @@ impl Command {
             Command::Fill { mask: true, .. } => "Fill Mask",
             Command::Gradient { mask: false, .. } => "Gradient",
             Command::Gradient { mask: true, .. } => "Gradient Mask",
+            // ShapeTool.swift:146: the kind's own name.
+            Command::AddShape { shape, .. } => shape.kind().name(),
         }
     }
 }
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -543,6 +543,7 @@ impl Engine {
                 let plane = if mask { Plane::Mask } else { Plane::Pixels };
                 Ok(Dirty::pixels(if mask { vec![] } else { vec![id] }).within(selection_regions(doc, clips, id, plane)))
             }
+            Command::AddShape { shape, color } => { ops::shape::add_shape(doc, &shape, color)?; Ok(Dirty::structure()) }
             Command::Gradient { id, mask, gradient } => {
                 ops::raster_edit::paint_layer(doc, clips, id, mask, &ops::raster_edit::Paint::Gradient(gradient))?;
                 let plane = if mask { Plane::Mask } else { Plane::Pixels };
```

```diff
--- a/engine/src/lib.rs
+++ b/engine/src/lib.rs
@@ -51,6 +51,7 @@ pub use lineage::*;
 pub use jobs::*;
 pub use ops::masks::blur_gray;
 pub use ops::raster_edit::{GradientShape, GradientSpec, Paint, MIN_GRADIENT_LINE};
+pub use ops::shape::{ShapeKind, ShapeSpec, SHAPE_TOO_LARGE};
 pub use selection::{Contour, Selection, SelectionMode, SelectionShape, SelectionState, MAX_FEATHER, MAX_RESIZE, SELECTION_COORDINATE_LIMIT, SUBPIXEL};
 pub use selection::coverage::{rasterize, selection_coverage, selection_coverage_with, SelectionClip, SelectionClips};
 pub use selection::feather::{feather_blur, DIRECT_SIGMA_LIMIT};
```

```diff
--- a/engine/src/ops/layers.rs
+++ b/engine/src/ops/layers.rs
@@ -21,8 +21,14 @@ fn is_inside(doc: &Document, id: Uuid, folder: Uuid) -> bool {
 
 /// Inserts a blank layer above the active layer (or at the top), inside the active folder if one is active.
 pub fn add_blank_layer(doc: &mut Document) -> Result<Uuid, CommandError> {
+    let layer = Layer::blank(&next_layer_name(doc), doc.size());
+    insert_above_active(doc, layer)
+}
+
+/// Inserts `layer` where New Layer puts one: above the active layer (or at the top), inside the
+/// active folder if one is active; it becomes the active layer.
+pub fn insert_above_active(doc: &mut Document, mut layer: Layer) -> Result<Uuid, CommandError> {
     if doc.layers.len() >= MAX_LAYERS { return Err(CommandError::Argument("too many layers".into())); }
-    let mut layer = Layer::blank(&next_layer_name(doc), doc.size());
     let active = doc.active_layer_id.and_then(|id| doc.layer(id).cloned());
     layer.parent_id = match &active { Some(a) if a.is_group => Some(a.id), Some(a) => a.parent_id, None => None };
     let mut insertion = doc.active_layer_id.and_then(|id| doc.index_of(id)).map(|i| i + 1).unwrap_or(doc.layers.len());
```

```diff
--- a/engine/src/ops/mod.rs
+++ b/engine/src/ops/mod.rs
@@ -11,3 +11,4 @@ pub mod masks;
 pub mod merge;
 pub mod selection;
 pub mod raster_edit;
+pub mod shape;
```

Create `engine/src/ops/shape.rs`:

```rust
//! The Shape tool's layers (Phase 4b-1), as Compositor for Mac makes them (ShapeTool.swift): a
//! rectangle (its corners rounded by at most half its shorter side), an ellipse, or a line stroked
//! with round ends, filled with one colour on a new layer above the active one, in one undo step.
//! The layer keeps a `shape` record the Mac reads (`LayerShapeStyle`), so the Mac redraws the shape
//! when the layer is scaled there; this port does not redraw it (ruling OQ12). Any other change to
//! the layer's pixels drops the record (`Layer::set_pixels`), as the Mac's `liveShape` does.
use crate::selection::coverage::rasterize;
use crate::selection::geometry::{cubic, ellipse, polygon, rectangle};
use crate::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The three kinds, named as the Mac names them (`ShapeKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind { Rectangle, Ellipse, Line }

impl ShapeKind {
    pub fn name(self) -> &'static str {
        match self { ShapeKind::Rectangle => "Rectangle", ShapeKind::Ellipse => "Ellipse", ShapeKind::Line => "Line" }
    }
}

/// What the Shape tool made: a box in document pixels (the Marquee's `DragBox`, whole pixels) for a
/// rectangle or an ellipse; for a line, the two points it was dragged between and its width.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ShapeSpec {
    Rectangle { rect: Rect, #[serde(rename = "cornerRadius", default)] corner_radius: f64 },
    Ellipse { rect: Rect },
    Line { start: Point, end: Point, width: f64 },
}

/// The words the Mac shows when a shape would hold too many pixels (ShapeTool.swift:131), with this
/// port's limit.
pub const SHAPE_TOO_LARGE: &str = "That shape is too large. A shape can cover up to 100 megapixels.";

impl ShapeSpec {
    pub fn kind(&self) -> ShapeKind {
        match self { ShapeSpec::Rectangle { .. } => ShapeKind::Rectangle, ShapeSpec::Ellipse { .. } => ShapeKind::Ellipse, ShapeSpec::Line { .. } => ShapeKind::Line }
    }
    /// The layer's box: the rectangle, or a line's two ends grown by half its width on every side
    /// (`finishShape`, ShapeTool.swift:123-128).
    pub fn bounds(&self) -> Rect {
        match self {
            ShapeSpec::Rectangle { rect, .. } | ShapeSpec::Ellipse { rect } => *rect,
            ShapeSpec::Line { start, end, width } => Rect {
                x: start.x.min(end.x) - width / 2.0, y: start.y.min(end.y) - width / 2.0,
                width: (end.x - start.x).abs() + width, height: (end.y - start.y).abs() + width,
            },
        }
    }
    fn check(&self) -> Result<(), CommandError> {
        let numbers: Vec<f64> = match self {
            ShapeSpec::Rectangle { rect, corner_radius } => vec![rect.x, rect.y, rect.width, rect.height, *corner_radius],
            ShapeSpec::Ellipse { rect } => vec![rect.x, rect.y, rect.width, rect.height],
            ShapeSpec::Line { start, end, width } => vec![start.x, start.y, end.x, end.y, *width],
        };
        if numbers.iter().any(|v| !v.is_finite() || v.abs() > MAX_SIDE as f64 * 4.0) {
            return Err(CommandError::Argument("shape out of range".into()));
        }
        if let ShapeSpec::Line { width, .. } = self { if !(*width > 0.0) { return Err(CommandError::Argument("a line needs a width".into())); } }
        Ok(())
    }
}

/// The outline of a rectangle `r` with corners rounded by `radius` (already clamped): Core Graphics'
/// `CGPath(roundedRect:cornerWidth:cornerHeight:)`, each corner a quarter circle as one cubic Bezier
/// with the circle constant, flattened as the ellipse is.
fn rounded_rectangle(r: Rect, radius: f64) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let k = radius * KAPPA;
    let p = |x: f64, y: f64| Point { x, y };
    let (x0, y0, x1, y1) = (r.x, r.y, r.max_x(), r.max_y());
    // Clockwise in y-down from the top edge's left end: each straight edge to a corner, then the
    // corner's curve; the last curve ends where the contour began.
    let mut points = vec![p(x0 + radius, y0), p(x1 - radius, y0)];
    cubic(&mut points, [p(x1 - radius, y0), p(x1 - radius + k, y0), p(x1, y0 + radius - k), p(x1, y0 + radius)]);
    points.push(p(x1, y1 - radius));
    cubic(&mut points, [p(x1, y1 - radius), p(x1, y1 - radius + k), p(x1 - radius + k, y1), p(x1 - radius, y1)]);
    points.push(p(x0 + radius, y1));
    cubic(&mut points, [p(x0 + radius, y1), p(x0 + radius - k, y1), p(x0, y1 - radius + k), p(x0, y1 - radius)]);
    points.push(p(x0, y0 + radius));
    cubic(&mut points, [p(x0, y0 + radius), p(x0, y0 + radius - k), p(x0 + radius - k, y0), p(x0 + radius, y0)]);
    points.pop();
    polygon(&points)
}

/// A line from `a` to `b` stroked `width` wide with round caps (`setLineCap(.round)`): two sides and
/// two half circles, each half circle two quarter-circle Beziers. A line of no length is a dot.
fn capsule(a: Point, b: Point, width: f64) -> Contour {
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let h = width / 2.0;
    let length = (b.x - a.x).hypot(b.y - a.y);
    if length == 0.0 { return ellipse(Rect { x: a.x - h, y: a.y - h, width, height: width }); }
    // Along the line (u) and across it (n), each h long; k of them for the Bezier handles.
    let (ux, uy) = ((b.x - a.x) / length * h, (b.y - a.y) / length * h);
    let (nx, ny) = (-uy, ux);
    let k = KAPPA;
    let p = |x: f64, y: f64| Point { x, y };
    // Along the n side from a to b, round b's end, back along the other side, round a's end; the
    // last curve ends where the contour began.
    let mut points = vec![p(a.x + nx, a.y + ny), p(b.x + nx, b.y + ny)];
    cubic(&mut points, [p(b.x + nx, b.y + ny), p(b.x + nx + k * ux, b.y + ny + k * uy), p(b.x + ux + k * nx, b.y + uy + k * ny), p(b.x + ux, b.y + uy)]);
    cubic(&mut points, [p(b.x + ux, b.y + uy), p(b.x + ux - k * nx, b.y + uy - k * ny), p(b.x - nx + k * ux, b.y - ny + k * uy), p(b.x - nx, b.y - ny)]);
    points.push(p(a.x - nx, a.y - ny));
    cubic(&mut points, [p(a.x - nx, a.y - ny), p(a.x - nx - k * ux, a.y - ny - k * uy), p(a.x - ux - k * nx, a.y - uy - k * ny), p(a.x - ux, a.y - uy)]);
    cubic(&mut points, [p(a.x - ux, a.y - uy), p(a.x - ux + k * nx, a.y - uy + k * ny), p(a.x + nx - k * ux, a.y + ny - k * uy), p(a.x + nx, a.y + ny)]);
    points.pop();
    polygon(&points)
}

/// The shape drawn into a layer of `bounds`: `Int(width)` x `Int(height)` pixels, the shape laid on
/// its box from the pixel grid's corner, filled with `color` at the area it covers (Core Graphics'
/// antialiased fill), premultiplied (`shapeImage`, ShapeTool.swift:197-223).
pub fn shape_raster(spec: &ShapeSpec, color: [f64; 3]) -> Raster {
    let bounds = spec.bounds();
    let (w, h) = (bounds.width as u32, bounds.height as u32);
    let local = Rect { x: 0.0, y: 0.0, width: bounds.width, height: bounds.height };
    let contour = match spec {
        ShapeSpec::Rectangle { corner_radius, .. } => {
            let radius = corner_radius.max(0.0).min(bounds.width / 2.0).min(bounds.height / 2.0);
            if radius > 0.0 { rounded_rectangle(local, radius) } else { rectangle(local) }
        }
        ShapeSpec::Ellipse { .. } => ellipse(local),
        ShapeSpec::Line { start, end, width } => {
            let at = |q: Point| Point { x: q.x - bounds.x, y: q.y - bounds.y };
            capsule(at(*start), at(*end), width.max(1.0))
        }
    };
    let coverage = rasterize(&[contour], 0.0, 0.0, w, h, true);
    let c = color.map(|v| v.clamp(0.0, 1.0) * 255.0);
    let data = coverage.bytes().iter().flat_map(|&k| {
        let a = k as f64 / 255.0;
        [(c[0] * a + 0.5) as u8, (c[1] * a + 0.5) as u8, (c[2] * a + 0.5) as u8, k]
    }).collect();
    Raster::from_premultiplied(w, h, data)
}

/// A number as Swift's JSONEncoder writes a CGFloat: a whole one without a fraction.
fn swift_number(v: f64) -> serde_json::Value {
    if v.fract() == 0.0 && v.abs() < 1e15 { serde_json::json!(v as i64) } else { serde_json::json!(v) }
}

/// The layer's `shape` record, as the Mac's `LayerShapeStyle` encodes: the kind, the colour, the
/// corner radius (0 but for rectangles), and for a line its width and its two ends as fractions of
/// the layer's box (ShapeTool.swift:136-145).
pub fn shape_record(spec: &ShapeSpec, color: [f64; 3]) -> serde_json::Value {
    let mut record = serde_json::json!({
        "kind": spec.kind().name(),
        "red": swift_number(color[0]), "green": swift_number(color[1]), "blue": swift_number(color[2]),
        "cornerRadius": swift_number(match spec { ShapeSpec::Rectangle { corner_radius, .. } => *corner_radius, _ => 0.0 }),
    });
    if let ShapeSpec::Line { start, end, width } = spec {
        let b = spec.bounds();
        let unit = |p: &Point| serde_json::json!([
            swift_number(if b.width > 0.0 { (p.x - b.x) / b.width } else { 0.5 }),
            swift_number(if b.height > 0.0 { (p.y - b.y) / b.height } else { 0.5 })]);
        record["lineWidth"] = swift_number(*width);
        record["start"] = unit(start);
        record["end"] = unit(end);
    }
    record
}

/// "Rectangle 1", "Ellipse 2", ... skipping names already in the document (`nextShapeName`).
pub fn next_shape_name(doc: &Document, kind: ShapeKind) -> String {
    let mut n = 1;
    while doc.layers.iter().any(|l| l.name == format!("{} {n}", kind.name())) { n += 1; }
    format!("{} {n}", kind.name())
}

/// Draws the shape on a new layer above the active one and makes it active; the selection stays
/// (`finishShape`, `addPixelLayer(dropsSelection: false)`). Refused when its box is under a pixel
/// on either side (a click makes nothing) or its pixels pass what the project may hold.
pub fn add_shape(doc: &mut Document, spec: &ShapeSpec, color: [f64; 3]) -> Result<Uuid, CommandError> {
    spec.check()?;
    if color.iter().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v)) { return Err(CommandError::Argument("shape colour out of range".into())); }
    let bounds = spec.bounds();
    if !(bounds.width >= 1.0 && bounds.height >= 1.0) { return Err(CommandError::Argument("a shape needs a box at least a pixel on each side".into())); }
    let (w, h) = (bounds.width as u64, bounds.height as u64);
    if w as i64 > MAX_SIDE || h as i64 > MAX_SIDE || w * h > MAX_PIXELS.saturating_sub(doc.used_pixels()) {
        return Err(CommandError::Refused(SHAPE_TOO_LARGE.into()));
    }
    let mut layer = Layer::with_pixels(&next_shape_name(doc, spec.kind()), shape_raster(spec, color), Point { x: bounds.x, y: bounds.y });
    layer.extra.shape = Some(shape_record(spec, color));
    ops::layers::insert_above_active(doc, layer)
}
```

```diff
--- a/engine/src/selection/geometry.rs
+++ b/engine/src/selection/geometry.rs
@@ -61,6 +61,20 @@ pub fn ellipse(r: Rect) -> Contour {
     polygon(&points)
 }
 
+/// One cubic Bezier `[p0, p1, p2, p3]` flattened as `ellipse` flattens its arcs, its points after
+/// `p0` up to and including `p3` pushed onto `points` (the caller has pushed `p0`).
+pub fn cubic(points: &mut Vec<Point>, [p0, p1, p2, p3]: [Point; 4]) {
+    let second = |a: Point, b: Point, c: Point| ((a.x - 2.0 * b.x + c.x).powi(2) + (a.y - 2.0 * b.y + c.y).powi(2)).sqrt();
+    let l = second(p0, p1, p2).max(second(p1, p2, p3));
+    let n = ((0.75 * l / CURVE_TOLERANCE).sqrt().ceil() as usize).max(1);
+    for i in 1..=n {
+        let t = i as f64 / n as f64;
+        let u = 1.0 - t;
+        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
+        points.push(Point { x: a * p0.x + b * p1.x + c * p2.x + d * p3.x, y: a * p0.y + b * p1.y + c * p2.y + d * p3.y });
+    }
+}
+
 /// The contours mapped through `map` (document points in, document points out), then quantized.
 pub fn transformed(contours: &[Contour], map: &Affine) -> Vec<Contour> {
     contours.iter().map(|c| c.iter().map(|p| quantize(map.apply(Point { x: p[0] as f64 / SUBPIXEL, y: p[1] as f64 / SUBPIXEL }))).collect()).collect()
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test shapes` (9 tests); the suite 507 passed, 9 ignored (+9, +1 ignored). `pnpm test` and `pnpm build` (the types) still pass. Natively: a shape over a 24 MP canvas 98-119 ms, 100 MP 423-431 ms (with Task 16's colour table; 110-130 and 489-498 ms before it). Task 16 measures what the page sees.

- [ ] **Step 5: Prove it bites**

(1) In `capsule`, set the handles to zero (`let k = 0.0 * KAPPA`: pointed caps): `a_line_of_width_one...` reads 191 for 228 at its end pixels and the area test fails the line by 183 levels (measured). Restore. (2) In `insert_above_active`, insert at the top: `a_shape_is_named_past_the_names_taken_and_placed_above_the_active_layer` fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/src/ops/shape.rs engine/tests/shapes.rs
git commit -m "feat(engine): shapes on new layers with the Mac's shape record" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/engine/types.ts engine/src/command.rs engine/src/engine.rs engine/src/lib.rs engine/src/ops/layers.rs engine/src/ops/mod.rs engine/src/ops/shape.rs engine/src/selection/geometry.rs engine/tests/perf_4b1.rs engine/tests/shapes.rs
```


---

### Task 11: The palette, and sampling the canvas as the Mac reads it

The store gains the palette (ruling OQ12): the image's foreground and background, black over white to begin with, and on a targeted mask its own black and white (`maskPaintWhite`), which `paletteColor(background)` returns while a mask is the target. X swaps and D resets whichever is showing; setting a swatch on a mask takes black or white only. `tools/color.ts` is the Mac's colour model: hex (`RRGGBB` or `RGB`, with or without `#`), 8-bit snapping, and the picker's HSB, which keeps its hue through greys and its saturation through black. The engine's `sample_color` becomes the Mac's sampler (ruling OQ14): nothing off the canvas, `(min(a, v) / a * 255).rounded() / 255` per channel.

**Files:**
- Create: `app/src/tools/color.ts`
- Modify: `app/src/state/store.ts` (`Palette`, `DEFAULT_PALETTE`, `maskTargeted`, `paletteColor`, `setPaletteColor`, `swapPalette`, `resetPalette`), `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`, `engine/src/engine.rs` (`sample_color`)
- Create tests: `app/tests/unit/color.test.ts`, `app/tests/unit/palette-store.test.ts`, `engine/tests/sample_color.rs`; modify `app/tests/unit/keymap.test.ts`

**Interfaces:**
- Produces: `PaletteColor`, `BLACK`, `WHITE`, `sameColor`, `colorTuple`, `colorOf`, `quantized`, `hexOf`, `parseHex`, `PickerHSB`, `hsbToRgb`, `withRgb`, `hsbOf`, `cssColor`; the store's palette API; the actions `swap-colors` (X) and `default-colors` (D).

- [ ] **Step 1: Write the tests**

`color.test.ts` ports ColorPickerTests' first three cases with their numbers (hex parsing, the HSB round trip of eight 8-bit colours, greys and black keeping hue and saturation) and adds hue wrapping (420 and -300 degrees are 60; 330 is (1, 0, 0.5)). `palette-store.test.ts`: black over white; set, swap, reset; a targeted mask's black and white, which swap and reset without touching the image's colours (setting the background white makes black the foreground; a foreground other than white is black); the image's colours when the mask chip is chosen on a layer without a mask; nothing while a job's result is to come. `sample_color.rs` ports the Mac test's 4 x 4 image (red on the top rows, blue below: (1.5, 0.5) is FF0000, (1.5, 3.5) 0000FF, a whole point is the pixel it starts, nothing off any edge) and a translucent pixel: premultiplied (100, 37, 250, 180) is (142, 52, 255) / 255 by the formula (100 / 180 x 255 = 141.67; 250 clamps to 180).

Create `app/tests/unit/color.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { BLACK, hexOf, hsbOf, hsbToRgb, parseHex, quantized, withRgb } from "../../src/tools/color";

// Ported from the Mac's ColorPickerTests with their numbers.
describe("the palette's colours", () => {
  it("parses full and shorthand hex and rejects the rest (hexParsesFullShorthandAndRejectsInvalid)", () => {
    expect(parseHex("#FF8000")).toEqual({ red: 1, green: 128 / 255, blue: 0 });
    expect(parseHex("0f0")).toEqual({ red: 0, green: 1, blue: 0 });
    expect(parseHex(" 00ff00 ")).toEqual({ red: 0, green: 1, blue: 0 });
    expect(parseHex("12345")).toBeNull();
    expect(parseHex("GGGGGG")).toBeNull();
    expect(hexOf({ red: 1, green: 128 / 255, blue: 0 })).toBe("FF8000");
  });
  it("round-trips 8-bit colours through hue, saturation and brightness (hsbRoundTripsEightBitColors)", () => {
    for (const hex of ["000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FF8000", "7F3FA2", "123456"]) {
      expect(hexOf(quantized(hsbToRgb(hsbOf(parseHex(hex)!))))).toBe(hex);
    }
  });
  it("keeps the hue through a grey and the saturation through black (graysAndBlackKeepPreviousHueAndSaturation)", () => {
    let hsb = hsbOf(parseHex("FF8000")!);
    const hue = hsb.hue;
    // The formula: max R, (G - B) / delta * 60 = (128 / 255) * 60.
    expect(hue).toBeCloseTo((128 / 255) * 60, 12);
    hsb = withRgb(hsb, parseHex("808080")!);
    expect([hsb.hue, hsb.saturation]).toEqual([hue, 0]);
    hsb = withRgb({ ...hsb, saturation: 0.5 }, BLACK);
    expect([hsb.hue, hsb.saturation, hsb.brightness]).toEqual([hue, 0.5, 0]);
  });
  it("wraps hues past a turn and below zero onto the same colour", () => {
    // 420 and -300 degrees are 60: yellow at full saturation and brightness.
    for (const hue of [60, 420, -300]) expect(hexOf(hsbToRgb({ hue, saturation: 1, brightness: 1 }))).toBe("FFFF00");
    // A sixth of the way between sectors: 330 degrees is (1, 0, 0.5).
    expect(hsbToRgb({ hue: 330, saturation: 1, brightness: 1 })).toEqual({ red: 1, green: 0, blue: 0.5 });
  });
});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -22,7 +22,9 @@ describe("keymap", () => {
     expect(matchShortcut(ev("Enter"))).toBe("apply");
     expect(matchShortcut(ev("Escape"))).toBe("cancel");
     expect(matchShortcut(ev("Enter", { ctrlKey: true }))).toBeNull();
-    expect(matchShortcut(ev("x"))).toBeNull();
+    expect(matchShortcut(ev("x"))).toBe("swap-colors");
+    expect(matchShortcut(ev("d"))).toBe("default-colors");
+    expect(matchShortcut(ev("x", { ctrlKey: true }))).toBeNull();
     expect(matchShortcut(ev("ArrowLeft"))).toBe("nudge-left");
     expect(matchShortcut(ev("ArrowRight", { shiftKey: true }))).toBe("nudge-right");
     expect(matchShortcut(ev("ArrowUp"))).toBe("nudge-up");
```

Create `app/tests/unit/palette-store.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, useEditor } from "../../src/state/store";
import { BLACK, WHITE, type PaletteColor } from "../../src/tools/color";
import type { DocumentState, LayerState } from "../../src/engine/types";

// The palette (ColorPalette.swift:20-58): the image's foreground and background, and black or white
// while a mask is the target.
const RED: PaletteColor = { red: 1, green: 0, blue: 0 };
const TEAL: PaletteColor = { red: 0, green: 0.5, blue: 0.5 };

function layer(hasMask: boolean): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [4, 4], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 4, pixelsHeight: 4, pixelsRevision: 1, hasPixels: true, hasMask, maskWidth: hasMask ? 4 : 0, maskHeight: hasMask ? 4 : 0,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function doc(hasMask: boolean): DocumentState {
  return { id: "D", documentId: "D", width: 4, height: 4, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer(hasMask)] };
}
function install(hasMask: boolean, maskSelected: boolean) {
  useEditor.setState({ activeId: "D", documents: { D: doc(hasMask) }, maskSelected, working: false, palette: DEFAULT_PALETTE });
}
const s = () => useEditor.getState();

describe("the palette", () => {
  beforeEach(() => install(false, false));
  it("starts black over white and sets, swaps and resets the image's colours", () => {
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
    s().setPaletteColor(RED, false);
    s().setPaletteColor(TEAL, true);
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([RED, TEAL]);
    s().swapPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([TEAL, RED]);
    s().resetPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
  });
  it("on a targeted mask shows black and white, which swap and reset without touching the image's colours", () => {
    s().setPaletteColor(RED, false);
    install(true, true);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: false } });
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([BLACK, WHITE]);
    s().swapPalette();
    expect([s().paletteColor(false), s().paletteColor(true)]).toEqual([WHITE, BLACK]);
    s().resetPalette();
    expect(s().palette.maskPaintWhite).toBe(false);
    // Setting the background to white makes black the foreground; setting the foreground to any
    // colour other than white makes it black.
    s().setPaletteColor(WHITE, true);
    expect(s().palette.maskPaintWhite).toBe(false);
    s().setPaletteColor(WHITE, false);
    expect(s().palette.maskPaintWhite).toBe(true);
    s().setPaletteColor(RED, false);
    expect(s().palette.maskPaintWhite).toBe(false);
    expect([s().palette.foreground, s().palette.background]).toEqual([RED, TEAL]);
  });
  it("is the image's while the mask chip is chosen on a layer without a mask, or a layer with one is chosen", () => {
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
    install(false, true);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
    expect(s().paletteColor(false)).toEqual(RED);
    install(true, false);
    useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
    expect(s().paletteColor(false)).toEqual(RED);
  });
  it("does not change while a job's result is to come", () => {
    useEditor.setState({ working: true });
    s().setPaletteColor(RED, false);
    s().swapPalette();
    s().resetPalette();
    expect(s().palette).toEqual(DEFAULT_PALETTE);
  });
});
```

Create `engine/tests/sample_color.rs`:

```rust
//! Sampling the canvas for the palette (Phase 4b-1): the Eyedropper and the colour picker read the
//! visible composite as the Mac's `sampleCompositeColor` does.
use compositor_engine::*;

fn p(x: f64, y: f64) -> Point { Point { x, y } }

#[test]
fn the_composite_is_read_at_the_pixel_under_the_point_and_nothing_off_the_canvas() {
    // ColorPickerTests.canvasSamplingReadsCompositeAndCommitsOnlyOnOK: a 4 x 4 image, red on its
    // top two rows and blue on the bottom two.
    let mut data = Vec::new();
    for y in 0..4 { for _ in 0..4 { data.extend_from_slice(if y < 2 { &[255, 0, 0, 255] } else { &[0, 0, 255, 255] }); } }
    let mut e = Engine::new();
    let id = e.new_document(4, 4, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers = vec![Layer::with_pixels("Split", Raster::from_premultiplied(4, 4, data), p(0.0, 0.0))];
    let id = e.insert_document(doc);
    assert_eq!(e.sample_color(id, p(1.5, 0.5)).unwrap(), Some([1.0, 0.0, 0.0]));
    assert_eq!(e.sample_color(id, p(1.5, 3.5)).unwrap(), Some([0.0, 0.0, 1.0]));
    assert_eq!(e.sample_color(id, p(2.0, 3.0)).unwrap(), Some([0.0, 0.0, 1.0]), "a whole point is the pixel it starts");
    for off in [p(-1.0, 1.0), p(4.0, 1.0), p(1.0, -0.01), p(1.0, 4.0)] { assert_eq!(e.sample_color(id, off).unwrap(), None, "{off:?}"); }
}

#[test]
fn a_translucent_pixel_is_unpremultiplied_and_snapped_to_eight_bits() {
    // Premultiplied (100, 37, 250 > alpha, 180): each channel min(a, v) / a * 255, rounded, / 255 --
    // 100 / 180 * 255 = 141.67 -> 142; 37 / 180 * 255 = 52.42 -> 52; 250 is clamped to 180 -> 255.
    let mut e = Engine::new();
    let id = e.new_document(2, 1, false).unwrap();
    let mut doc = e.document(id).unwrap().clone();
    doc.layers = vec![Layer::with_pixels("A", Raster::from_premultiplied(2, 1, vec![100, 37, 250, 180, 0, 0, 0, 0]), p(0.0, 0.0))];
    let id = e.insert_document(doc);
    assert_eq!(e.sample_color(id, p(0.5, 0.5)).unwrap(), Some([142.0 / 255.0, 52.0 / 255.0, 1.0]));
    assert_eq!(e.sample_color(id, p(1.5, 0.5)).unwrap(), None, "a transparent pixel has no colour");
}
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `color.test.ts` and `palette-store.test.ts` do not compile; `keymap.test.ts` fails at X (null). `cargo test -p compositor-engine --test sample_color`: the translucent pixel comes back unsnapped, (100 / 180, 37 / 180, 1.0); the off-canvas points already sample nothing (a region off the canvas composites transparent), so they pin the explicit check against a later change of the compositor.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -4,7 +4,8 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "new-folder" | "duplicate" | "group" | "merge" | "clip" | "layer-up" | "layer-down" | "blend-next" | "blend-prev" | "delete-layer"
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
-  | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode";
+  | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
+  | "swap-colors" | "default-colors";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -36,6 +37,8 @@ export const SHORTCUTS = {
   "select-all": [{ key: "a", ctrl: true }], "deselect": [{ key: "d", ctrl: true }], "select-inverse": [{ key: "i", ctrl: true, shift: true }],
   // Tab switches the current tool's kind (EditorCanvas.swift:1823-1827).
   "cycle-tool-mode": [{ key: "Tab" }],
+  // The palette (EditorCanvas.swift:1835-1836).
+  "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
 } satisfies Record<ActionId, Shortcut[]>;
 
 export function matchShortcut(e: KeyboardEvent): ActionId | null {
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -82,6 +82,8 @@ export function runAction(id: ActionId, shift = false): void {
     case "deselect": if (doc?.selection) s.run({ type: "Deselect" }); break;
     case "select-inverse": if (doc?.selection) s.run({ type: "InvertSelection" }); break;
     case "cycle-tool-mode": s.cycleToolMode(); break;
+    case "swap-colors": s.swapPalette(); break;
+    case "default-colors": s.resetPalette(); break;
     // An open panel answers Enter and Escape itself (AdjustPanel.tsx); neither may also reach the
     // crop tool or a transform. An outline being drawn takes them first (EditorCanvas.swift:1772-1777).
     case "apply":
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -13,6 +13,7 @@ import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } fr
 import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
+import { BLACK, WHITE, sameColor, type PaletteColor } from "../tools/color";
 
 export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
@@ -52,6 +53,12 @@ export interface TransformEdit {
   duplicateEntry: number | null;
 }
 
+/** The palette (ColorPalette.swift): the image's foreground and background colours, and, while a
+ * mask is the target, whether white is its foreground (`maskPaintWhite`; black otherwise). Kept for
+ * the app session, as the Mac keeps it per window. */
+export interface Palette { foreground: PaletteColor; background: PaletteColor; maskPaintWhite: boolean; }
+export const DEFAULT_PALETTE: Palette = { foreground: BLACK, background: WHITE, maskPaintWhite: false };
+
 /** A layer with more pixels than this is edited, and its histogram read, by the job worker rather than
  * on the UI thread (ruling OQ5): at 4 MP a Levels commit took about 0.35 s here. Below it a job's two
  * copies and the worker's round trip cost more than they save. */
@@ -100,6 +107,18 @@ export interface EditorStore {
   outlineMove: { dx: number; dy: number } | null;
   /** The mode Shift / Alt held over the canvas imply, for the options bar (`heldSelectionMode`). */
   heldSelectionMode: SelectionMode | null;
+  palette: Palette;
+  /** Whether a mask is the paint target: the mask chip of the active layer, which has one (`isMaskSelected`). */
+  maskTargeted(): boolean;
+  /** The foreground or background colour the tools use: black or white while a mask is the target (`paletteColor`). */
+  paletteColor(background: boolean): PaletteColor;
+  /** Sets one swatch; while a mask is the target only black or white, as which of them is the foreground
+   * (`setPaletteColor`). Nothing while a job's result is to come (`canEditPalette`). */
+  setPaletteColor(color: PaletteColor, background: boolean): void;
+  /** X: swap the swatches, or black and white on a mask (`swapPaletteColors`). */
+  swapPalette(): void;
+  /** D: black over white, or black as the mask's foreground (`resetPaletteColors`). */
+  resetPalette(): void;
   setEngine(engine: EngineClient): void;
   setJobs(jobs: JobClient): void;
   /** Whether an edit of `layerId`'s pixels goes to the job worker: it has more than `jobPixels`. */
@@ -214,6 +233,35 @@ export const useEditor = create<EditorStore>((set, get) => ({
   blendPreview: null,
   adjustEdit: null,
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
+  palette: DEFAULT_PALETTE,
+  maskTargeted: () => {
+    const { activeId, documents, maskSelected } = get();
+    const doc = activeId ? documents[activeId] : null;
+    return maskSelected && !!doc && !!activeLayer(doc)?.hasMask;
+  },
+  paletteColor: (background) => {
+    const p = get().palette;
+    if (get().maskTargeted()) return (background ? !p.maskPaintWhite : p.maskPaintWhite) ? WHITE : BLACK;
+    return background ? p.background : p.foreground;
+  },
+  setPaletteColor: (color, background) => {
+    if (get().working) return;
+    const p = get().palette;
+    if (get().maskTargeted()) {
+      const white = sameColor(color, WHITE);
+      set({ palette: { ...p, maskPaintWhite: background ? !white : white } });
+    } else set({ palette: background ? { ...p, background: color } : { ...p, foreground: color } });
+  },
+  swapPalette: () => {
+    if (get().working) return;
+    const p = get().palette;
+    set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: !p.maskPaintWhite } : { ...p, foreground: p.background, background: p.foreground } });
+  },
+  resetPalette: () => {
+    if (get().working) return;
+    const p = get().palette;
+    set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: false } : { ...p, foreground: BLACK, background: WHITE } });
+  },
   setEngine: (engine) => set({ engine }),
   setJobs: (jobs) => set({ jobs }),
   usesJob: (layerId) => {
```

Create `app/src/tools/color.ts`:

```ts
/** The palette's colours and the colour picker's model, as Compositor for Mac keeps them
 * (ColorPalette.swift): sRGB 0..1 per channel, a hex form, and the picker's hue / saturation /
 * brightness, which keeps its hue through greys and its saturation through black. */
export interface PaletteColor { red: number; green: number; blue: number; }

export const BLACK: PaletteColor = { red: 0, green: 0, blue: 0 };
export const WHITE: PaletteColor = { red: 1, green: 1, blue: 1 };

export const sameColor = (a: PaletteColor, b: PaletteColor): boolean => a.red === b.red && a.green === b.green && a.blue === b.blue;
export const colorTuple = (c: PaletteColor): [number, number, number] => [c.red, c.green, c.blue];
export const colorOf = (t: readonly [number, number, number]): PaletteColor => ({ red: t[0], green: t[1], blue: t[2] });

/** Snapped to the 8-bit values painting and export store (`quantized`). */
export function quantized(c: PaletteColor): PaletteColor {
  return { red: Math.round(c.red * 255) / 255, green: Math.round(c.green * 255) / 255, blue: Math.round(c.blue * 255) / 255 };
}

/** `RRGGBB`, upper case (`hex`). */
export function hexOf(c: PaletteColor): string {
  return [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).toUpperCase().padStart(2, "0")).join("");
}

/** `RRGGBB` or `RGB`, with or without a leading `#`, spaces around it ignored; null otherwise
 * (`PaletteColor(hex:)`). */
export function parseHex(text: string): PaletteColor | null {
  let t = text.trim();
  if (t.startsWith("#")) t = t.slice(1);
  if (t.length === 3) t = [...t].map((ch) => ch + ch).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(t)) return null;
  const v = parseInt(t, 16);
  return { red: ((v >> 16) & 0xff) / 255, green: ((v >> 8) & 0xff) / 255, blue: (v & 0xff) / 255 };
}

/** Hue in degrees, saturation and brightness 0..1 (`PickerHSB`). */
export interface PickerHSB { hue: number; saturation: number; brightness: number; }

/** Swift's `truncatingRemainder`: the remainder with the dividend's sign, as JavaScript's `%`. */
const rem = (a: number, b: number) => a % b;

/** The colour the picker's values name (`PickerHSB.rgb`). */
export function hsbToRgb({ hue, saturation, brightness }: PickerHSB): PaletteColor {
  const h = rem(rem(hue, 360) + 360, 360) / 60;
  const c = brightness * saturation;
  const x = c * (1 - Math.abs(rem(h, 2) - 1));
  const m = brightness - c;
  let rgb: [number, number, number];
  switch (Math.trunc(h)) {
    case 0: rgb = [c, x, 0]; break;
    case 1: rgb = [x, c, 0]; break;
    case 2: rgb = [0, c, x]; break;
    case 3: rgb = [0, x, c]; break;
    case 4: rgb = [x, 0, c]; break;
    default: rgb = [c, 0, x];
  }
  return { red: rgb[0] + m, green: rgb[1] + m, blue: rgb[2] + m };
}

/** `hsb` moved to `color`, keeping its hue for a grey and its saturation for black, as
 * Photoshop's field does (`setRGB`). */
export function withRgb(hsb: PickerHSB, color: PaletteColor): PickerHSB {
  const high = Math.max(color.red, color.green, color.blue);
  const low = Math.min(color.red, color.green, color.blue);
  const delta = high - low;
  const next = { ...hsb, brightness: high };
  if (high > 0) next.saturation = delta / high;
  if (!(delta > 0)) return next;
  let h: number;
  if (high === color.red) h = (color.green - color.blue) / delta;
  else if (high === color.green) h = (color.blue - color.red) / delta + 2;
  else h = (color.red - color.green) / delta + 4;
  h *= 60;
  next.hue = h < 0 ? h + 360 : h;
  return next;
}

/** The picker's values for `color`, from nothing (`PickerHSB(_:)`). */
export const hsbOf = (color: PaletteColor): PickerHSB => withRgb({ hue: 0, saturation: 0, brightness: 0 }, color);

/** A CSS colour for a swatch. */
export const cssColor = (c: PaletteColor): string => `rgb(${Math.round(c.red * 255)}, ${Math.round(c.green * 255)}, ${Math.round(c.blue * 255)})`;
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -734,12 +734,18 @@ impl Engine {
     /// that special case once, under this same name, and it was never used by anything but such a
     /// test -- a live production caller (the Hue/Saturation eyedropper) reused it on the strength
     /// of the name alone and sampled its own preview by mistake (a Critical, Task 15 code review).
+    /// The visible composite's colour at the document pixel under `at`, straight and snapped to
+    /// 8 bits as the Mac reads it: `(min(a, v) / a * 255).rounded() / 255` (`sampleCompositeColor`,
+    /// ColorPalette.swift:182-203). None off the canvas or over a transparent pixel. It reads the
+    /// stored document, never an open preview.
     pub fn sample_color(&self, id: Uuid, at: Point) -> Result<Option<[f64; 3]>, CommandError> {
         let doc = &self.session(id)?.document;
+        if !(at.x >= 0.0 && at.y >= 0.0 && at.x < doc.width as f64 && at.y < doc.height as f64) { return Ok(None); }
         let region = Rect { x: at.x.floor(), y: at.y.floor(), width: 1.0, height: 1.0 };
         let pixel = compositor::composite_edit_with(doc, None, region, 1, 1, &self.effects).pixel(0, 0);
         if pixel[3] == 0 { return Ok(None); }
-        Ok(Some([0, 1, 2].map(|c| (pixel[c] as f64 / pixel[3] as f64).min(1.0))))
+        let alpha = pixel[3] as f64;
+        Ok(Some([0, 1, 2].map(|c| ((pixel[c] as f64).min(alpha) / alpha * 255.0).round() / 255.0)))
     }
     pub fn levels_sampling(&self, id: Uuid, layer: Uuid, settings: &LevelsSettings, at: Point, mode: LevelsSample) -> Result<LevelsSettings, CommandError> {
         match self.sample_layer_color(id, layer, at)? { Some(rgb) => Ok(settings.sampling(rgb, mode)), None => Ok(settings.clone()) }
```


- [ ] **Step 4: Run the tests and watch them pass**

Engine: 509 passed, 9 ignored (+2). `pnpm test`: 175 (+8). `pnpm wasm:dev`, `pnpm build`, `pnpm e2e`: 139 passed, 9 skipped (the Hue/Saturation sampling e2e still passes with the snapped samples).

- [ ] **Step 5: Prove it bites**

(1) In `sample_color`, return `min(a, v) / a` unsnapped: `a_translucent_pixel_is_unpremultiplied_and_snapped_to_eight_bits` fails (measured). Restore. (2) In `paletteColor`, drop the mask branch: "on a targeted mask shows black and white..." fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/tools/color.ts app/tests/unit/color.test.ts app/tests/unit/palette-store.test.ts engine/tests/sample_color.rs
git commit -m "feat: the palette, swap and reset, and canvas sampling as the Mac reads it" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/src/state/store.ts app/src/tools/color.ts app/tests/unit/color.test.ts app/tests/unit/keymap.test.ts app/tests/unit/palette-store.test.ts engine/src/engine.rs engine/tests/sample_color.rs
```


---

### Task 12: The palette's swatches and the colour picker

The palette appears at the foot of the tool rail (ColorPaletteControls.swift): the foreground swatch over the background one, swap and default-colour buttons; on a targeted mask a click asks black or white ("Black - Hide", "White - Reveal"; the Mac's middle dot is not ASCII). Otherwise a click opens the colour picker (ColorPickerSheet.swift; ruling OQ13): a 256 px saturation / brightness field, the hue strip from 360 at the top to 0 at the bottom, the new colour, OK and Cancel, R, G, B and hex, "Click the canvas to sample". It floats over the window, first centred on the canvas and then where it was last left; Enter and Escape reach it before anything else; while it is open a press or drag on the canvas samples into it whatever the tool (`canvas/sampling.ts`, in the capture phase), with the Mac's ring about the pointer (the sampled colour above, the colour before below). Gradient Map's two ends open it too (the panel previews the working colour; Cancel restores), and a new Gradient Map - destructive or as a layer - starts from the foreground to the background.

**Files:**
- Create: `app/src/panels/PaletteSwatches.tsx`, `app/src/panels/ColorPickerPanel.tsx`, `app/src/canvas/sampling.ts`
- Modify: `app/src/state/store.ts` (`PickerTarget`, `ColorPicker`, `pickerTitle`, `SampleRing`, `colorPicker`, `pickerAt`, `sampleRing`, `openColorPicker`, `setPickerHsb`, `closeColorPicker`, `sampleIntoPicker`, the Gradient Map seed), `app/src/actions/layers.ts` (new Gradient Map layers), `app/src/canvas/overlay.ts` (the ring), `app/src/canvas/CanvasView.tsx`, `app/src/panels/ToolRail.tsx`, `app/src/panels/FilterPanel.tsx`, `app/src/App.tsx`, `app/src/styles.css`
- Create tests: `app/tests/unit/picker-store.test.ts`, `app/tests/e2e/color-picker.spec.ts`

**Interfaces:**
- Produces: `PickerTarget`, `ColorPicker`, `pickerTitle`, `SampleRing`, the store's picker API, `PICKER_FIELD`, `installSampling(el, spaceHeld)`, `samplingInto()`, `SAMPLE_RING`.

- [ ] **Step 1: Write the tests**

`picker-store.test.ts` ports ColorPickerTests.canvasSamplingReadsCompositeAndCommitsOnlyOnOK with its numbers (a stub engine samples the 4 x 4 image): sampling (2, 3) makes the picker 0000FF and leaves the foreground black until OK; OK on the background after sampling (2, 0) makes it FF0000. And: the picker keeps its colour where the canvas has none, switches swatches while open, snaps to 8 bits, does not open on a swatch while a mask is the target (and targeting one closes it uncommitted), nor while a job's result is to come; a destructive Gradient Map starts at the palette, an end previews as it changes, Cancel puts it back, OK keeps it, and the picker goes with its panel. The e2e: X, D and the swatches' colours; hex and Enter, OK only on OK; the picker first opens centred on the canvas, moves by its title and reopens where it was left; the hue strip a quarter down is 270 degrees within a pointer pixel and the field dragged past its corners saturates and clamps; a press on the canvas under the Marquee samples (FF0000, then 0000FF as it drags) with the ring drawn 43 px about the pointer, and no selection starts; on a targeted mask the swatch asks black or white; Gradient Map's end previews the canvas and Escape closes only the picker.

Create `app/tests/e2e/color-picker.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4b-1: the palette at the foot of the tool rail and the colour picker, through the real
// swatches, keys, panel and canvas (the store's rules are pinned in palette-store and picker-store).

type Pt = [number, number];

/** A 64 x 48 document, red on the left 32 columns and blue on the right, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
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
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
const palette = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().palette);
const swatch = (page: Page, which: "foreground" | "background") => page.getByTestId(`palette-${which}`).evaluate((el) => getComputedStyle(el).backgroundColor);
const picker = (page: Page) => page.getByTestId("color-picker");
const undoDepth = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });

test("the swatches show the palette; X swaps and D resets; a picker opened on one commits on OK only", async ({ page }) => {
  await setup(page);
  const depth = await undoDepth(page);
  await expect(page.getByTestId("palette")).toBeVisible();
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(0, 0, 0)", "rgb(255, 255, 255)"]);
  await page.keyboard.press("x");
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(255, 255, 255)", "rgb(0, 0, 0)"]);
  await page.keyboard.press("d");
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 0)");
  // The foreground swatch opens the picker on black; a hex typed and Enter in the field sets its
  // working colour, the swatch stays black until OK.
  await page.getByTestId("palette-foreground").click();
  await expect(picker(page)).toBeVisible();
  await expect(page.getByTestId("picker-title")).toHaveText("Color Picker (Foreground Color)");
  await page.getByTestId("picker-hex").fill("#ff8000");
  await page.getByTestId("picker-hex").press("Enter");
  await expect(page.getByTestId("picker-red")).toHaveValue("255");
  await expect(page.getByTestId("picker-green")).toHaveValue("128");
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 0)");
  // Enter outside a field is OK.
  await page.getByTestId("picker-title").click();
  await page.keyboard.press("Enter");
  await expect(picker(page)).toBeHidden();
  expect(await swatch(page, "foreground")).toBe("rgb(255, 128, 0)");
  // Escape cancels: the background stays white.
  await page.getByTestId("palette-background").click();
  await page.getByTestId("picker-blue").fill("0");
  await page.keyboard.press("Escape");
  await expect(picker(page)).toBeHidden();
  expect(await swatch(page, "background")).toBe("rgb(255, 255, 255)");
  expect(await undoDepth(page), "the palette records no history").toBe(depth);
});

test("the picker opens where it was left, and dragging its field and hue strip sets the colour", async ({ page }) => {
  await setup(page);
  await page.getByTestId("palette-foreground").click();
  const first = await picker(page).boundingBox();
  const canvas = await page.getByTestId("canvas-view").boundingBox();
  // First opened centred on the canvas.
  expect(Math.abs(first!.x + first!.width / 2 - (canvas!.x + canvas!.width / 2))).toBeLessThanOrEqual(1);
  const title = await page.getByTestId("picker-title").boundingBox();
  await page.mouse.move(title!.x + 20, title!.y + 8); await page.mouse.down();
  await page.mouse.move(title!.x - 80, title!.y - 40, { steps: 4 }); await page.mouse.up();
  const moved = await picker(page).boundingBox();
  expect([moved!.x - first!.x, moved!.y - first!.y]).toEqual([-100, -48]);
  // A quarter of the way down the hue strip is 270 degrees (within the pointer's pixel); the field
  // dragged past its top-right corner holds full saturation and brightness; past its bottom, none.
  const hue = await page.getByTestId("picker-hue").boundingBox();
  await page.mouse.click(hue!.x + hue!.width / 2, hue!.y + hue!.height / 4);
  const field = await page.getByTestId("picker-field").boundingBox();
  await page.mouse.move(field!.x + 100, field!.y + 100); await page.mouse.down();
  await page.mouse.move(field!.x + field!.width + 30, field!.y - 30, { steps: 3 }); await page.mouse.up();
  const hsb = await page.evaluate(() => (window as any).__compositor.store.getState().colorPicker.hsb);
  expect(Math.abs(hsb.hue - 270)).toBeLessThanOrEqual(360 / 256);
  expect([hsb.saturation, hsb.brightness]).toEqual([1, 1]);
  // Red is half of full at 270 degrees: 7F or 80, blue full, green none.
  await expect(page.getByTestId("picker-hex")).toHaveValue(/^(7F|80)00FF$/);
  await page.mouse.move(field!.x + 100, field!.y + 100); await page.mouse.down();
  await page.mouse.move(field!.x + 100, field!.y + field!.height + 40, { steps: 3 }); await page.mouse.up();
  await expect(page.getByTestId("picker-hex")).toHaveValue("000000");
  await page.getByTestId("picker-cancel").click();
  await page.getByTestId("palette-background").click();
  const again = await picker(page).boundingBox();
  expect([again!.x, again!.y]).toEqual([moved!.x, moved!.y]);
});

test("while the picker is open a press on the canvas samples it, showing the ring, and no tool acts", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("m"); // the Marquee: a press would otherwise start a selection
  await page.getByTestId("palette-foreground").click();
  // Moved off the document, which it opens centred over.
  await page.evaluate(() => (window as any).__compositor.store.getState().setPickerAt({ x: 700, y: 360 }));
  const left = await client(page, [10, 20]), right = await client(page, [50, 20]);
  await page.mouse.move(left.x, left.y); await page.mouse.down();
  await expect(page.getByTestId("picker-hex")).toHaveValue("FF0000");
  // The ring is drawn about the pointer: 43 px from it, the sampled colour above and black below.
  const ring = await page.evaluate(([x, y]) => {
    const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
    const r = overlay.getBoundingClientRect(); const dpr = window.devicePixelRatio || 1;
    const at = (dx: number, dy: number) => Array.from(overlay.getContext("2d")!.getImageData(Math.round((x - r.left + dx) * dpr), Math.round((y - r.top + dy) * dpr), 1, 1).data);
    return { above: at(0, -43), below: at(0, 43) };
  }, [left.x, left.y]);
  expect(ring.above).toEqual([255, 0, 0, 255]);
  expect(ring.below).toEqual([0, 0, 0, 255]);
  // Dragging keeps sampling.
  await page.mouse.move(right.x, right.y, { steps: 5 });
  await expect(page.getByTestId("picker-hex")).toHaveValue("0000FF");
  await page.mouse.up();
  const state = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { selection: s.documents[s.activeId].selection, draft: s.selectionDraft, ring: s.sampleRing }; });
  expect(state).toEqual({ selection: null, draft: null, ring: null });
  expect((await palette(page)).foreground).toEqual({ red: 0, green: 0, blue: 0 });
  await page.getByTestId("picker-ok").click();
  expect(await swatch(page, "foreground")).toBe("rgb(0, 0, 255)");
});

test("with a mask targeted a swatch offers black or white, and the picker does not open", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true });
    api.store.getState().setMaskSelected(true);
  });
  await page.getByTestId("palette-background").click();
  await expect(page.getByTestId("palette-mask-popover")).toBeVisible();
  await expect(picker(page)).toBeHidden();
  await page.getByTestId("mask-black").click();
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(255, 255, 255)", "rgb(0, 0, 0)"]);
  // The image's own colours are untouched and come back with the pixels targeted.
  await page.evaluate(() => (window as any).__compositor.store.getState().setMaskSelected(false));
  expect([await swatch(page, "foreground"), await swatch(page, "background")]).toEqual(["rgb(0, 0, 0)", "rgb(255, 255, 255)"]);
});

test("Gradient Map starts at the palette, its ends open the picker, which previews them, and Escape closes only the picker", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.setPaletteColor({ red: 1, green: 1, blue: 0 }, false); s.setPaletteColor({ red: 0, green: 0, blue: 0 }, true); });
  await clickMenu(page, "Image", "image-gradient-map");
  await expect(page.getByTestId("adjust-panel")).toBeVisible();
  // Red and blue map by their luminance between yellow (shadows) and black (highlights).
  const shown = () => page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState();
    return Array.from(api.engine.composite(s.activeId, { x: 10, y: 20, width: 1, height: 1 }, 1, 1) as Uint8Array);
  });
  const before = await shown();
  await page.getByTestId("gradient-map-shadows").click();
  await expect(page.getByTestId("picker-title")).toHaveText("Color Picker (Gradient Map Shadows)");
  await expect(page.getByTestId("picker-hex")).toHaveValue("FFFF00");
  await page.getByTestId("picker-hex").fill("00FFFF");
  await page.getByTestId("picker-hex").press("Enter");
  const previewed = await shown();
  expect(previewed, "the canvas previews the working colour").not.toEqual(before);
  await page.getByTestId("picker-title").click();
  await page.keyboard.press("Escape");
  await expect(picker(page)).toBeHidden();
  await expect(page.getByTestId("adjust-panel"), "Escape went to the picker alone").toBeVisible();
  expect(await shown(), "Cancel put the end back").toEqual(before);
  await page.getByTestId("adjust-cancel").click();
  // A new Gradient Map layer starts at the palette too.
  await clickMenu(page, "Layer", "layer-adjustment-gradient-map");
  const settings = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); const d = s.documents[s.activeId]; return d.layers.find((l: any) => l.id === d.activeLayerId).adjustment.gradientMapSettings; });
  expect([settings.shadows, settings.highlights]).toEqual([{ red: 1, green: 1, blue: 0 }, { red: 0, green: 0, blue: 0 }]);
});
```

Create `app/tests/unit/picker-store.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, pickerTitle, useEditor } from "../../src/state/store";
import { BLACK, WHITE, hexOf, hsbOf, parseHex, type PaletteColor } from "../../src/tools/color";
import { defaultAdjustment } from "../../src/state/adjust-edit";
import type { DocumentState, LayerAdjustment, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The colour picker (ColorPalette.swift:60-178; ColorPickerTests.canvasSamplingReadsCompositeAndCommitsOnlyOnOK).
const RED: PaletteColor = { red: 1, green: 0, blue: 0 };
// On the 8-bit grid, so the picker (which snaps to it) gives it back exactly.
const TEAL: PaletteColor = { red: 0, green: 128 / 255, blue: 128 / 255 };

function layer(hasMask: boolean): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [4, 4], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 4, pixelsHeight: 4, pixelsRevision: 1, hasPixels: true, hasMask, maskWidth: hasMask ? 4 : 0, maskHeight: hasMask ? 4 : 0,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 };
}
function doc(hasMask = false): DocumentState {
  return { id: "D", documentId: "D", width: 4, height: 4, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer(hasMask)] };
}
let previews: (PreviewRequest | null)[] = [];
/** The Mac test's 4 x 4 image: red on the top two rows, blue below. */
const sampled = (at: { x: number; y: number }): [number, number, number] | null =>
  at.x < 0 || at.y < 0 || at.x >= 4 || at.y >= 4 ? null : at.y < 2 ? [1, 0, 0] : [0, 0, 1];
function install(hasMask = false) {
  previews = [];
  const engine = {
    state: () => doc(hasMask),
    setPreview: (_id: string, r: PreviewRequest | null) => { previews.push(r); return { structure: true, canvas: false, layers: [] }; },
    sampleColor: (_id: string, at: { x: number; y: number }) => sampled(at),
    adjustmentIsIdentity: (a: LayerAdjustment) => JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind)),
    histogram: () => [],
  } as unknown as EngineClient;
  useEditor.setState({ engine, jobs: null, activeId: "D", documents: { D: doc(hasMask) }, order: ["D"], selectedLayerIds: ["A"], maskSelected: false,
    working: false, palette: DEFAULT_PALETTE, colorPicker: null, adjustEdit: null, transformEdit: null, error: null, tool: "move" });
}
const s = () => useEditor.getState();

describe("the colour picker", () => {
  beforeEach(() => install());
  it("samples the canvas into itself and commits only on OK, to the swatch it opened on", () => {
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(true);
    expect(pickerTitle(s().colorPicker!.target)).toBe("Color Picker (Foreground Color)");
    s().sampleIntoPicker({ x: 2, y: 3 });
    expect(hexOf(s().pickerColor()!)).toBe("0000FF");
    expect(s().palette.foreground).toEqual(BLACK);
    s().closeColorPicker(false);
    expect([s().palette.foreground, s().colorPicker]).toEqual([BLACK, null]);
    s().openColorPicker({ kind: "palette", background: true });
    s().sampleIntoPicker({ x: 2, y: 0 });
    s().closeColorPicker(true);
    expect([hexOf(s().palette.background), s().palette.foreground]).toEqual(["FF0000", BLACK]);
  });
  it("keeps its colour where the canvas has none, and switches swatches while open", () => {
    s().setPaletteColor(TEAL, false);
    s().openColorPicker({ kind: "palette", background: false });
    s().sampleIntoPicker({ x: -1, y: 1 });
    expect(s().pickerColor()).toEqual(TEAL);
    s().openColorPicker({ kind: "palette", background: true });
    expect(s().colorPicker!.original).toEqual(WHITE);
    expect(s().colorPicker!.target).toEqual({ kind: "palette", background: true });
  });
  it("snaps its working colour to 8 bits", () => {
    s().openColorPicker({ kind: "palette", background: false });
    s().setPickerHsb({ hue: 200, saturation: 0.333, brightness: 0.777 });
    const c = s().pickerColor()!;
    for (const v of [c.red, c.green, c.blue]) expect(Number.isInteger(Math.round(v * 255 * 1e9) / 1e9)).toBe(true);
    s().closeColorPicker(true);
    expect(s().palette.foreground).toEqual(c);
  });
  it("does not open on a swatch while a mask is the target, and targeting one closes it uncommitted", () => {
    install(true);
    s().openColorPicker({ kind: "palette", background: false });
    s().setPickerHsb(hsbOf(RED));
    s().setMaskSelected(true);
    expect(s().colorPicker).toBeNull();
    expect(s().palette.foreground).toEqual(BLACK);
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(false);
  });
  it("does not open while a job's result is to come", () => {
    useEditor.setState({ working: true });
    expect(s().openColorPicker({ kind: "palette", background: false })).toBe(false);
  });
});

describe("Gradient Map ends through the picker", () => {
  beforeEach(() => install());
  it("starts a destructive Gradient Map at the palette, previews an end as it changes, and Cancel puts it back", () => {
    s().setPaletteColor(RED, false);
    s().setPaletteColor(TEAL, true);
    expect(s().beginAdjust({ kind: "Gradient Map" })).toBe(true);
    const settings = () => s().adjustEdit!.adjustment!.gradientMapSettings!;
    expect([settings().shadows, settings().highlights]).toEqual([RED, TEAL]);
    expect(s().openColorPicker({ kind: "gradientMap", highlights: true })).toBe(true);
    expect(pickerTitle(s().colorPicker!.target)).toBe("Color Picker (Gradient Map Highlights)");
    expect(s().colorPicker!.original).toEqual(TEAL);
    // A second picker does not open over it.
    expect(s().openColorPicker({ kind: "gradientMap", highlights: false })).toBe(false);
    const before = previews.length;
    s().setPickerHsb(hsbOf(parseHex("FF8000")!));
    expect(hexOf(settings().highlights)).toBe("FF8000");
    expect(previews.length).toBeGreaterThan(before);
    expect((previews.at(-1) as { adjustment: LayerAdjustment }).adjustment.gradientMapSettings!.highlights).toEqual(settings().highlights);
    s().closeColorPicker(false);
    expect(settings().highlights).toEqual(TEAL);
    expect(s().palette.background).toEqual(TEAL);
  });
  it("keeps the end on OK, and goes with its panel", () => {
    s().beginAdjust({ kind: "Gradient Map" });
    s().openColorPicker({ kind: "gradientMap", highlights: false });
    s().setPickerHsb(hsbOf(RED));
    s().closeColorPicker(true);
    expect(s().adjustEdit!.adjustment!.gradientMapSettings!.shadows).toEqual(RED);
    s().openColorPicker({ kind: "gradientMap", highlights: false });
    s().cancelAdjust();
    expect(s().colorPicker).toBeNull();
    expect(s().openColorPicker({ kind: "gradientMap", highlights: false })).toBe(false);
  });
});
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `picker-store.test.ts` does not compile (`openColorPicker`). The e2e cannot find `palette`.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -23,6 +23,7 @@ import { SelectionAmountSheet } from "./sheets/SelectionAmountSheet";
 import { TransformInspector } from "./panels/TransformInspector";
 import { AdjustPanel } from "./panels/AdjustPanel";
 import { UndrawnNotice } from "./panels/UndrawnNotice";
+import { ColorPickerPanel } from "./panels/ColorPickerPanel";
 import "./styles.css";
 
 export function App() {
@@ -102,6 +103,7 @@ export function App() {
         </div>
       </div>
       <AdjustPanel />
+      <ColorPickerPanel />
       <UndrawnNotice />
       <div className="status" data-testid="engine-ready">Compositor engine {version} ({BUILD_MARKER}){working && <span data-testid="working"> - Working...</span>}</div>
       {banner && <div data-testid="error-banner" className="error-banner">{banner}<button onClick={() => useEditor.getState().setError(null)}>Dismiss</button></div>}
```

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -1,3 +1,4 @@
+import { colorTuple } from "../tools/color";
 import { useEditor } from "../state/store";
 import { activeLayer, visibleIds } from "../state/selection";
 import type { AdjustmentKind, BlendMode, SelectionMode } from "../engine/types";
@@ -116,11 +117,13 @@ export function canInvert(): boolean {
   return (c.s.maskSelected && c.active.hasMask) || c.active.hasPixels;
 }
 /** Layer > New <kind> Adjustment. Each Grain layer gets its own pattern, and a Gradient Map
- * starts from black to white (the Mac takes the palette, which Phase 4 adds). */
+ * starts from the image's foreground to its background (LayerAdjustment.swift:191). */
 export function addAdjustmentLayer(kind: AdjustmentKind): void {
   const c = ctx(); if (!c) return;
   const seed = Math.floor(Math.random() * 0xffffffff);
-  c.s.run({ type: "AddAdjustmentLayer", kind, seed, shadows: null, highlights: null });
+  const { foreground, background } = c.s.palette;
+  const ends = kind === "Gradient Map" ? { shadows: colorTuple(foreground), highlights: colorTuple(background) } : { shadows: null, highlights: null };
+  c.s.run({ type: "AddAdjustmentLayer", kind, seed, ...ends });
   // The new layer is active; open its panel straight away, as macOS does; Invert has nothing to
   // set, so it just applies (LayerAdjustment.swift:203-204).
   const created = activeLayer(useEditor.getState().documents[c.doc.id]);
```

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -10,6 +10,7 @@ import { containsPoint, cornersToTuples, fromTuple, hitOverlay, overlayGeometry,
 import { activeLayer, canTransform, editedShape, transformsAsGroup } from "../state/selection";
 import { antsDelay, AntsPathCache, ANTS_INTERVAL_MS, nextPhase, OutlineCache, outlineStep } from "./ants";
 import { isSelectionTool, outlineOffset, SelectionDraft, selectionMode, type P as DocP } from "../tools/selection-draft";
+import { installSampling } from "./sampling";
 
 export const HIT_HANDLE_PX = 6;
 
@@ -72,7 +73,7 @@ export function CanvasView() {
     drawOverlay(overlay.getContext("2d")!, vp, dpr, {
       docWidth: doc.width, docHeight: doc.height, cropRect: s.tool === "crop" ? s.cropRect : null, guides: s.snapGuides,
       transform: transformGeometry, canvasGuides: s.showGuides ? doc.guides : null,
-      ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null,
+      ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null, sampleRing: s.sampleRing,
     });
   };
 
@@ -191,6 +192,12 @@ export function CanvasView() {
     return () => { window.removeEventListener("keydown", key); window.removeEventListener("keyup", key); window.removeEventListener("blur", blur); };
   }, []);
 
+  // Sampling into the colour picker while it is open (canvas/sampling.ts): ahead of every tool gesture.
+  useEffect(() => {
+    const el = glRef.current?.parentElement; if (!el) return;
+    return installSampling(el, () => spaceRef.current);
+  }, []);
+
   // Adjustment eyedroppers: while a sample mode is armed (Levels' three, or Hue/Saturation's
   // replace/add/remove), a click reads the point under the cursor and feeds the open panel
   // instead of starting whatever gesture the active tool would otherwise begin. Registered with
@@ -214,10 +221,11 @@ export function CanvasView() {
 
   // The cursor shows an armed eyedropper regardless of which tool is otherwise selected, and a
   // crosshair for the selection tools.
+  const picking = useEditor((s) => !!s.colorPicker);
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
-    el.style.cursor = sampleMode || isSelectionTool(tool) ? "crosshair" : "";
-  }, [sampleMode, tool]);
+    el.style.cursor = sampleMode || picking || isSelectionTool(tool) ? "crosshair" : "";
+  }, [sampleMode, tool, picking]);
 
   // Drag to pan with the hand tool or the space bar.
   useEffect(() => {
```

```diff
--- a/app/src/canvas/overlay.ts
+++ b/app/src/canvas/overlay.ts
@@ -3,6 +3,7 @@ import type { Rect } from "../tools/crop-geometry";
 import { HANDLES } from "../tools/crop-geometry";
 import type { OverlayGeometry } from "../tools/transform-geometry";
 import type { Guide, SelectionShape } from "../engine/types";
+import { cssColor, type PaletteColor } from "../tools/color";
 
 /** The marching ants: the selection's outline as a path in view px about `at`, the document's
  * scaled origin (moved by the outline's offset while it is dragged), built once and kept by
@@ -21,6 +22,28 @@ export interface OverlayState {
   /** Null with no selection, or an empty one. */
   ants?: AntsState | null;
   draft?: DraftState | null;
+  /** While the canvas is sampled: the ring about the pointer (view px). */
+  sampleRing?: { at: { x: number; y: number }; sampled: PaletteColor; original: PaletteColor } | null;
+}
+
+/** The side of the sample ring's box, in view px (`SampleRingOverlay`: a 116 pt frame). */
+export const SAMPLE_RING = 116;
+
+/** A grey ring 24 px wide, its top half then the colour sampled and its bottom half the colour
+ * before sampling, each 16 px wide, on a circle inset 15 px in its box (SampleRingOverlay). */
+function drawSampleRing(ctx: CanvasRenderingContext2D, ring: NonNullable<OverlayState["sampleRing"]>): void {
+  const radius = (SAMPLE_RING - 30) / 2;
+  ctx.save();
+  ctx.beginPath(); ctx.arc(ring.at.x, ring.at.y, radius, 0, Math.PI * 2);
+  ctx.lineWidth = 24; ctx.strokeStyle = "rgb(115, 115, 115)"; ctx.stroke();
+  for (const [color, top] of [[ring.sampled, true], [ring.original, false]] as const) {
+    ctx.save();
+    ctx.beginPath(); ctx.rect(ring.at.x - SAMPLE_RING / 2, top ? ring.at.y - SAMPLE_RING / 2 : ring.at.y, SAMPLE_RING, SAMPLE_RING / 2); ctx.clip();
+    ctx.beginPath(); ctx.arc(ring.at.x, ring.at.y, radius, 0, Math.PI * 2);
+    ctx.lineWidth = 16; ctx.strokeStyle = cssColor(color); ctx.stroke();
+    ctx.restore();
+  }
+  ctx.restore();
 }
 
 /** The dash the ants march along, in view px (`drawSelection`, TransformOverlay.swift:283-298). */
@@ -141,6 +164,7 @@ export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, d
   }
   if (state.ants) drawAnts(ctx, state.ants);
   if (state.draft) drawDraft(ctx, viewport, size, state.draft);
+  if (state.sampleRing) drawSampleRing(ctx, state.sampleRing);
   ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
   for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
   for (const y of state.guides.ys) { const v = viewport.viewPoint({ x: 0, y }, size).y; ctx.beginPath(); ctx.moveTo(0, v + 0.5); ctx.lineTo(viewport.viewSize.width, v + 0.5); ctx.stroke(); }
```

Create `app/src/canvas/sampling.ts`:

```ts
import { useEditor } from "../state/store";
import type { PaletteColor } from "../tools/color";

/** Where a press on the canvas samples colour (EditorCanvas.swift:1419-1424): into the open colour
 * picker, whatever the tool; else null. */
export function samplingInto(): "picker" | null {
  return useEditor.getState().colorPicker ? "picker" : null;
}

/** The colour the sample ring shows as sampled: the picker's working colour. */
function current(): PaletteColor | null {
  return useEditor.getState().pickerColor();
}

/** Samples the canvas under the pointer while it is pressed (`sampleColor`, EditorCanvas.swift:2040-2055):
 * a press reads the pixel under it and a drag keeps reading, at most once a frame; the ring shows the
 * colour sampled over the colour the press began with, until release. Registered in the capture phase
 * so no tool gesture starts under it. Returns the cleanup. */
export function installSampling(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let original: PaletteColor | null = null;
  let pending: PointerEvent | null = null;
  let frame = 0;
  const view = (e: PointerEvent) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
  const sample = (e: PointerEvent) => {
    const s = useEditor.getState(); if (!s.activeId || !original) return;
    const vp = s.viewports[s.activeId], d = s.documents[s.activeId];
    const at = view(e);
    s.sampleIntoPicker(vp.documentPoint(at, { width: d.width, height: d.height }));
    const sampled = current();
    if (sampled) s.setSampleRing({ at, sampled, original });
  };
  const down = (e: PointerEvent) => {
    if (e.button !== 0 || spaceHeld() || !samplingInto()) return;
    e.stopImmediatePropagation();
    original = current();
    el.setPointerCapture(e.pointerId);
    sample(e);
  };
  const move = (e: PointerEvent) => {
    if (!original) return;
    e.stopImmediatePropagation();
    pending = e;
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; if (pending) sample(pending); pending = null; });
  };
  const up = (e: PointerEvent) => {
    if (!original) return;
    e.stopImmediatePropagation();
    if (frame) { cancelAnimationFrame(frame); frame = 0; if (pending) sample(pending); pending = null; }
    original = null;
    useEditor.getState().setSampleRing(null);
  };
  el.addEventListener("pointerdown", down, { capture: true });
  el.addEventListener("pointermove", move, { capture: true });
  el.addEventListener("pointerup", up, { capture: true });
  el.addEventListener("pointercancel", up, { capture: true });
  return () => {
    if (frame) cancelAnimationFrame(frame);
    el.removeEventListener("pointerdown", down, { capture: true });
    el.removeEventListener("pointermove", move, { capture: true });
    el.removeEventListener("pointerup", up, { capture: true });
    el.removeEventListener("pointercancel", up, { capture: true });
  };
}
```

Create `app/src/panels/ColorPickerPanel.tsx`:

```tsx
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { pickerTitle, useEditor } from "../state/store";
import { cssColor, hexOf, hsbToRgb, parseHex, withRgb } from "../tools/color";
import { NumberInput } from "./NumberInput";

/** The side of the saturation / brightness field and the height of the hue strip (`fieldSize`). */
export const PICKER_FIELD = 256;
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));
// Fields with their own Enter: a number or the hex field commits itself, a focused button is clicked.
const OWN_ENTER = new Set(["INPUT", "BUTTON"]);

/** The colour picker (ColorPickerSheet.swift): a saturation / brightness field, a hue strip, the new
 * colour, OK and Cancel, R / G / B and hex, and "Click the canvas to sample". It floats, so the canvas
 * stays visible and clickable; its title bar drags it, and it opens again where it was last left
 * (first centred on the canvas). Enter is OK and Escape Cancel, ahead of every other key handler. */
export function ColorPickerPanel() {
  const s = useEditor();
  const picker = s.colorPicker;
  const color = s.pickerColor();
  const [hexDraft, setHexDraft] = useState<string | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!picker) return;
    const key = (e: KeyboardEvent) => {
      const own = e.target instanceof HTMLElement && OWN_ENTER.has(e.target.tagName);
      if (e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); useEditor.getState().closeColorPicker(false); }
      else if (e.key === "Enter" && !own) { e.preventDefault(); e.stopImmediatePropagation(); useEditor.getState().closeColorPicker(true); }
    };
    window.addEventListener("keydown", key, { capture: true });
    return () => window.removeEventListener("keydown", key, { capture: true });
  }, [!!picker]);
  // The first opening centres it on the canvas; after that it stays where it was left.
  useEffect(() => {
    if (!picker || s.pickerAt || !panelRef.current) return;
    const canvas = document.querySelector('[data-testid="canvas-view"]')?.getBoundingClientRect();
    const r = panelRef.current.getBoundingClientRect();
    const x = canvas ? canvas.left + (canvas.width - r.width) / 2 : 100, y = canvas ? canvas.top + (canvas.height - r.height) / 2 : 100;
    s.setPickerAt({ x: Math.max(0, Math.round(x)), y: Math.max(0, Math.round(y)) });
  }, [!!picker]);
  if (!picker || !color) return null;
  const hsb = picker.hsb;
  const drag = (apply: (x: number, y: number) => void) => (e: ReactPointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget;
    const at = (ev: { clientX: number; clientY: number }) => { const r = el.getBoundingClientRect(); apply(ev.clientX - r.left, ev.clientY - r.top); };
    el.setPointerCapture(e.pointerId);
    at(e);
    const move = (ev: PointerEvent) => at(ev);
    const up = () => { el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  };
  const field = drag((x, y) => { const h = useEditor.getState().colorPicker!.hsb; s.setPickerHsb({ ...h, saturation: clamp01(x / PICKER_FIELD), brightness: 1 - clamp01(y / PICKER_FIELD) }); });
  const strip = drag((_x, y) => { const h = useEditor.getState().colorPicker!.hsb; s.setPickerHsb({ ...h, hue: (1 - clamp01(y / PICKER_FIELD)) * 360 }); });
  const moveTitle = (e: ReactPointerEvent<HTMLDivElement>) => {
    const el = e.currentTarget, start = { x: e.clientX, y: e.clientY }, from = useEditor.getState().pickerAt ?? { x: 0, y: 0 };
    el.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => s.setPickerAt({ x: Math.max(0, from.x + ev.clientX - start.x), y: Math.max(0, from.y + ev.clientY - start.y) });
    const up = () => { el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  };
  const channel = (label: "Red" | "Green" | "Blue", key: "red" | "green" | "blue") => (
    <label className="picker-channel">{label[0]}
      <NumberInput label={label} testId={`picker-${key}`} value={Math.round(color[key] * 255)} min={0} max={255} step={1}
        onChange={(v) => s.setPickerHsb(withRgb(useEditor.getState().colorPicker!.hsb, { ...color, [key]: Math.round(v) / 255 }))} />
    </label>
  );
  const commitHex = () => {
    if (hexDraft !== null) { const parsed = parseHex(hexDraft); if (parsed) s.setPickerHsb(withRgb(hsb, parsed)); }
    setHexDraft(null);
  };
  const at = s.pickerAt ?? { x: 100, y: 100 };
  const hueColor = cssColor(hsbToRgb({ hue: hsb.hue, saturation: 1, brightness: 1 }));
  const stops = [360, 300, 240, 180, 120, 60, 0].map((h) => cssColor(hsbToRgb({ hue: h, saturation: 1, brightness: 1 }))).join(", ");
  return (
    <div ref={panelRef} className="color-picker" data-testid="color-picker" role="dialog" aria-label={pickerTitle(picker.target)} style={{ left: at.x, top: at.y }}>
      <div className="color-picker-title" data-testid="picker-title" onPointerDown={moveTitle}>{pickerTitle(picker.target)}</div>
      <div className="color-picker-body">
        <div className="picker-field" data-testid="picker-field" aria-label="Saturation and brightness" onPointerDown={field}
          style={{ width: PICKER_FIELD, height: PICKER_FIELD, background: `linear-gradient(to bottom, transparent, #000), linear-gradient(to right, #fff, ${hueColor})` }}>
          <div className="picker-marker" style={{ left: hsb.saturation * PICKER_FIELD - 6, top: (1 - hsb.brightness) * PICKER_FIELD - 6 }} />
        </div>
        <div className="picker-hue" data-testid="picker-hue" aria-label="Hue" onPointerDown={strip} style={{ height: PICKER_FIELD }}>
          <div className="picker-hue-strip" style={{ background: `linear-gradient(to bottom, ${stops})` }} />
          <div className="picker-hue-arrows" style={{ top: (1 - hsb.hue / 360) * PICKER_FIELD - 5 }} />
        </div>
        <div className="picker-side">
          <div className="picker-top">
            <div className="picker-new" data-testid="picker-new" aria-label="New color" style={{ background: cssColor(color) }} />
            <div className="picker-buttons">
              <button className="primary" data-testid="picker-ok" onClick={() => s.closeColorPicker(true)}>OK</button>
              <button data-testid="picker-cancel" onClick={() => s.closeColorPicker(false)}>Cancel</button>
            </div>
          </div>
          <div className="picker-fields">
            {channel("Red", "red")}{channel("Green", "green")}{channel("Blue", "blue")}
            <label className="picker-channel">#
              <input aria-label="Hex color" data-testid="picker-hex" value={hexDraft ?? hexOf(color)} spellCheck={false}
                onChange={(e) => setHexDraft(e.target.value)} onBlur={commitHex}
                onKeyDown={(e) => { if (e.key === "Enter") commitHex(); }} />
            </label>
          </div>
          <div className="picker-hint">Click the canvas to sample</div>
        </div>
      </div>
    </div>
  );
}
```

```diff
--- a/app/src/panels/FilterPanel.tsx
+++ b/app/src/panels/FilterPanel.tsx
@@ -20,7 +20,6 @@ function NumberField(props: { label: string; name?: string; value: number; min:
   );
 }
 const hex = (c: AdjustmentColor) => "#" + [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).padStart(2, "0")).join("");
-const fromHex = (text: string): AdjustmentColor => ({ red: parseInt(text.slice(1, 3), 16) / 255, green: parseInt(text.slice(3, 5), 16) / 255, blue: parseInt(text.slice(5, 7), 16) / 255 });
 
 export function FilterPanel() {
   const s = useEditor();
@@ -55,9 +54,14 @@ export function FilterPanel() {
   if (a.kind === "Gradient Map") {
     const g = a.gradientMapSettings ?? { shadows: { red: 0, green: 0, blue: 0 }, highlights: { red: 1, green: 1, blue: 1 }, reversed: false };
     const set = (patch: Partial<typeof g>) => setAdjustment({ gradientMapSettings: { ...g, ...patch } });
+    // Each end opens the app's colour picker, which previews as it changes (`openGradientMapColorPicker`).
+    const end = (label: "Shadows" | "Highlights", c: AdjustmentColor) => (
+      <label>{label} <button aria-label={label} data-testid={`gradient-map-${label.toLowerCase()}`} className="color-swatch-button" style={{ background: hex(c) }}
+        disabled={!!s.colorPicker} onClick={(e) => { e.currentTarget.blur(); s.openColorPicker({ kind: "gradientMap", highlights: label === "Highlights" }); }} /></label>
+    );
     return (<>
-      <label>Shadows <input aria-label="Shadows" type="color" value={hex(g.shadows)} onChange={(e) => set({ shadows: fromHex(e.target.value) })} /></label>
-      <label>Highlights <input aria-label="Highlights" type="color" value={hex(g.highlights)} onChange={(e) => set({ highlights: fromHex(e.target.value) })} /></label>
+      {end("Shadows", g.shadows)}
+      {end("Highlights", g.highlights)}
       <label><input type="checkbox" aria-label="Reverse" checked={g.reversed} onChange={(e) => set({ reversed: e.target.checked })} /> Reverse</label>
     </>);
   }
```

Create `app/src/panels/PaletteSwatches.tsx`:

```tsx
import { useEffect, useState } from "react";
import { useEditor } from "../state/store";
import { BLACK, WHITE, cssColor, type PaletteColor } from "../tools/color";

/** The palette at the foot of the tool rail (ColorPaletteControls.swift): the foreground swatch over
 * the background one, swap (X) at the top right, default colours (D) at the bottom left. A click on a
 * swatch opens the colour picker; while a mask is the target it asks black or white instead. Every
 * button gives up the focus once used, as the rail's tools do. */
export function PaletteSwatches() {
  const s = useEditor();
  const [choosing, setChoosing] = useState<"foreground" | "background" | null>(null);
  const masked = s.maskTargeted();
  // A change of target closes the black-or-white choice (ColorPaletteControls.swift:53-54).
  useEffect(() => setChoosing(null), [masked]);
  const enabled = !s.working;
  const swatch = (background: boolean) => {
    const label = background ? "Background color" : "Foreground color";
    return (
      <button className={`palette-swatch ${background ? "background" : "foreground"}`} data-testid={`palette-${background ? "background" : "foreground"}`}
        aria-label={label} title={label} disabled={!enabled} style={{ background: cssColor(s.paletteColor(background)) }}
        onClick={(e) => {
          e.currentTarget.blur();
          if (masked) setChoosing(background ? "background" : "foreground");
          else s.openColorPicker({ kind: "palette", background });
        }} />
    );
  };
  const choose = (color: PaletteColor) => { if (choosing) s.setPaletteColor(color, choosing === "background"); setChoosing(null); };
  return (
    <div className="palette" data-testid="palette">
      {swatch(true)}
      {swatch(false)}
      <button className="palette-swap" data-testid="palette-swap" aria-label="Swap colors" title="Swap foreground and background (X)" disabled={!enabled}
        onClick={(e) => { e.currentTarget.blur(); s.swapPalette(); }}>
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M3 9.5 9.5 3" /><path d="M6.5 3h3v3" /><path d="M3 6v3.5h3.5" />
        </svg>
      </button>
      <button className="palette-reset" data-testid="palette-reset" aria-label="Default colors" title="Default colors (D)" disabled={!enabled}
        onClick={(e) => { e.currentTarget.blur(); s.resetPalette(); }}>
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <rect x="1" y="1" width="6" height="6" fill="#000" stroke="#ccc" strokeWidth="0.8" /><rect x="5" y="5" width="6" height="6" fill="#fff" stroke="#ccc" strokeWidth="0.8" />
        </svg>
      </button>
      {choosing && masked && (
        <div className="palette-mask-popover" data-testid="palette-mask-popover" role="dialog" aria-label={choosing === "background" ? "Mask background" : "Mask foreground"}>
          <strong>{choosing === "background" ? "Mask background" : "Mask foreground"}</strong>
          <div>
            <button data-testid="mask-black" onClick={() => choose(BLACK)}>Black - Hide</button>
            <button data-testid="mask-white" onClick={() => choose(WHITE)}>White - Reveal</button>
          </div>
        </div>
      )}
    </div>
  );
}
```

```diff
--- a/app/src/panels/ToolRail.tsx
+++ b/app/src/panels/ToolRail.tsx
@@ -1,5 +1,6 @@
 import { useEditor, type Tool } from "../state/store";
 import { ToolIcon, toolIconName } from "./tool-icons";
+import { PaletteSwatches } from "./PaletteSwatches";
 
 const TOOLS: { id: Tool; label: string; key: string }[] = [
   { id: "move", label: "Move", key: "V" }, { id: "marquee", label: "Marquee", key: "M" },
@@ -21,6 +22,7 @@ export function ToolRail() {
           <ToolIcon name={toolIconName(t.id, marquee, lasso)} />
         </button>
       ))}
+      <PaletteSwatches />
     </div>
   );
 }
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -13,7 +13,7 @@ import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } fr
 import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
-import { BLACK, WHITE, sameColor, type PaletteColor } from "../tools/color";
+import { BLACK, WHITE, hsbOf, hsbToRgb, quantized, sameColor, withRgb, type PaletteColor, type PickerHSB } from "../tools/color";
 
 export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
@@ -59,6 +59,22 @@ export interface TransformEdit {
 export interface Palette { foreground: PaletteColor; background: PaletteColor; maskPaintWhite: boolean; }
 export const DEFAULT_PALETTE: Palette = { foreground: BLACK, background: WHITE, maskPaintWhite: false };
 
+/** What the open colour picker edits (`ColorPickerTarget`): a palette swatch, or one end of the
+ * Gradient Map being edited in its panel. */
+export type PickerTarget = { kind: "palette"; background: boolean } | { kind: "gradientMap"; highlights: boolean };
+/** The open picker (`ColorPickerState`): its target, the colour it opened on, and its working values.
+ * Nothing reaches a swatch until OK; a Gradient Map end previews the working colour and Cancel puts
+ * the original back. */
+export interface ColorPicker { target: PickerTarget; original: PaletteColor; hsb: PickerHSB; }
+/** The picker's title bar (`ColorPickerTarget.title`). */
+export function pickerTitle(target: PickerTarget): string {
+  if (target.kind === "palette") return target.background ? "Color Picker (Background Color)" : "Color Picker (Foreground Color)";
+  return target.highlights ? "Color Picker (Gradient Map Highlights)" : "Color Picker (Gradient Map Shadows)";
+}
+/** The ring shown while the canvas is being sampled (`SampleRingOverlay`): where, in view px, the
+ * colour sampled and the colour before sampling began. */
+export interface SampleRing { at: { x: number; y: number }; sampled: PaletteColor; original: PaletteColor; }
+
 /** A layer with more pixels than this is edited, and its histogram read, by the job worker rather than
  * on the UI thread (ruling OQ5): at 4 MP a Levels commit took about 0.35 s here. Below it a job's two
  * copies and the worker's round trip cost more than they save. */
@@ -119,6 +135,24 @@ export interface EditorStore {
   swapPalette(): void;
   /** D: black over white, or black as the mask's foreground (`resetPaletteColors`). */
   resetPalette(): void;
+  colorPicker: ColorPicker | null;
+  /** Where the picker's panel was last left (its top-left, in window px); null until it is first moved. */
+  pickerAt: { x: number; y: number } | null;
+  sampleRing: SampleRing | null;
+  /** The picker's working colour, snapped to 8 bits (`ColorPickerState.color`). */
+  pickerColor(): PaletteColor | null;
+  /** Opens the picker on a swatch (not while a mask is the target: the swatch asks black or white
+   * instead) or on a Gradient Map end while that panel is open. Opening on a swatch while it is open
+   * on the other one switches it (`openColorPicker`, `openGradientMapColorPicker`). */
+  openColorPicker(target: PickerTarget): boolean;
+  /** The picker's new values; a Gradient Map end previews them at once (`previewGradientMapColor`). */
+  setPickerHsb(hsb: PickerHSB): void;
+  /** OK (`commit`) or Cancel (`closeColorPicker`). */
+  closeColorPicker(commit: boolean): void;
+  /** Loads the canvas colour under a document point into the open picker (`sampleIntoColorPicker`). */
+  sampleIntoPicker(at: { x: number; y: number }): void;
+  setPickerAt(at: { x: number; y: number }): void;
+  setSampleRing(ring: SampleRing | null): void;
   setEngine(engine: EngineClient): void;
   setJobs(jobs: JobClient): void;
   /** Whether an edit of `layerId`'s pixels goes to the job worker: it has more than `jobPixels`. */
@@ -217,10 +251,24 @@ function dropOpenPanel(): void {
   cancelSettle();
   const { adjustEdit, engine, activeId } = useEditor.getState();
   if (!adjustEdit) return;
-  useEditor.setState({ adjustEdit: null });
+  // A picker open on one of the panel's Gradient Map ends goes with it.
+  useEditor.setState({ adjustEdit: null, ...(useEditor.getState().colorPicker?.target.kind === "gradientMap" ? { colorPicker: null } : {}) });
   if (activeId) engine!.setPreview(activeId, null);
 }
 
+/** Sets one end of the open Gradient Map panel to `color`, previewing it (`setGradientMapColor`). */
+function setGradientMapEnd(highlights: boolean, color: PaletteColor): void {
+  const edit = useEditor.getState().adjustEdit;
+  const settings = edit?.adjustment?.gradientMapSettings;
+  if (!edit?.adjustment || !settings || edit.adjustment.kind !== "Gradient Map") return;
+  const end = { red: color.red, green: color.green, blue: color.blue };
+  const current = highlights ? settings.highlights : settings.shadows;
+  if (current.red === end.red && current.green === end.green && current.blue === end.blue) return;
+  useEditor.getState().updateAdjust({ adjustment: { ...edit.adjustment, gradientMapSettings: { ...settings, [highlights ? "highlights" : "shadows"]: end } } });
+}
+/** `withRgb` from an engine sample. */
+const withRgbFrom = (hsb: PickerHSB, rgb: [number, number, number]): PickerHSB => withRgb(hsb, { red: rgb[0], green: rgb[1], blue: rgb[2] });
+
 const GUIDES_KEY = "compositor.showGuides";
 function loadShowGuides(): boolean {
   try { return localStorage.getItem(GUIDES_KEY) !== "false"; } catch { return true; }
@@ -233,7 +281,42 @@ export const useEditor = create<EditorStore>((set, get) => ({
   blendPreview: null,
   adjustEdit: null,
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
-  palette: DEFAULT_PALETTE,
+  palette: DEFAULT_PALETTE, colorPicker: null, pickerAt: null, sampleRing: null,
+  pickerColor: () => { const p = get().colorPicker; return p ? quantized(hsbToRgb(p.hsb)) : null; },
+  openColorPicker: (target) => {
+    if (get().working) return false;
+    let original: PaletteColor;
+    if (target.kind === "palette") {
+      if (get().maskTargeted()) return false;
+      original = get().paletteColor(target.background);
+    } else {
+      const edit = get().adjustEdit;
+      const settings = edit?.adjustment?.kind === "Gradient Map" && !edit.params ? edit.adjustment.gradientMapSettings : undefined;
+      if (!settings || get().colorPicker) return false;
+      original = target.highlights ? settings.highlights : settings.shadows;
+    }
+    set({ colorPicker: { target, original, hsb: hsbOf(original) } });
+    return true;
+  },
+  setPickerHsb: (hsb) => {
+    const picker = get().colorPicker; if (!picker) return;
+    set({ colorPicker: { ...picker, hsb } });
+    if (picker.target.kind === "gradientMap") setGradientMapEnd(picker.target.highlights, get().pickerColor()!);
+  },
+  closeColorPicker: (commit) => {
+    const picker = get().colorPicker; if (!picker) return;
+    const color = get().pickerColor()!;
+    set({ colorPicker: null });
+    if (picker.target.kind === "palette") { if (commit && !get().maskTargeted()) get().setPaletteColor(color, picker.target.background); }
+    else setGradientMapEnd(picker.target.highlights, commit ? color : picker.original);
+  },
+  sampleIntoPicker: (at) => {
+    const { colorPicker, engine, activeId } = get(); if (!colorPicker || !engine || !activeId) return;
+    const rgb = engine.sampleColor(activeId, at);
+    if (rgb) get().setPickerHsb(withRgbFrom(colorPicker.hsb, rgb));
+  },
+  setPickerAt: (pickerAt) => set({ pickerAt }),
+  setSampleRing: (sampleRing) => { set({ sampleRing }); get().repaintOverlay(); },
   maskTargeted: () => {
     const { activeId, documents, maskSelected } = get();
     const doc = activeId ? documents[activeId] : null;
@@ -428,8 +511,13 @@ export const useEditor = create<EditorStore>((set, get) => ({
     set({ selectedLayerIds: valid, maskSelected: false });
     get().refresh(activeId);
   },
-  // Quiet: the chip handlers that call this have just had `selectLayers` raise the banner.
-  setMaskSelected: (v) => { if (!get().panelOwnsDocument()) set({ maskSelected: v }); },
+  // Quiet: the chip handlers that call this have just had `selectLayers` raise the banner. Targeting a
+  // mask closes a picker open on a swatch: a mask's palette is black and white (ColorPaletteControls.swift:53-56).
+  setMaskSelected: (v) => {
+    if (get().panelOwnsDocument()) return;
+    set({ maskSelected: v });
+    if (get().colorPicker?.target.kind === "palette" && get().maskTargeted()) get().closeColorPicker(false);
+  },
   toggleCollapsed: (id) => {
     const { activeId, collapsed } = get(); if (!activeId) return;
     const list = collapsed[activeId] ?? [];
@@ -561,6 +649,11 @@ export const useEditor = create<EditorStore>((set, get) => ({
     if (editing ? !layer.adjustment : !get().canAdjust()) return false;
     const filter = isFilterKind(kind as string);
     const adjustment = filter ? null : (editing ? layer.adjustment! : defaultAdjustment(kind as AdjustmentKind));
+    // A destructive Gradient Map starts from the image's foreground and background (Filters.swift:490).
+    if (adjustment?.gradientMapSettings && !editing) {
+      const { foreground, background } = get().palette;
+      adjustment.gradientMapSettings = { ...adjustment.gradientMapSettings, shadows: { ...foreground }, highlights: { ...background } };
+    }
     // Each destructive Grain gets a pattern of its own, as the Mac's FilterEdit draws a random
     // seed and as Add Noise already does here; an adjustment layer keeps the seed it was made with.
     if (adjustment?.grainSettings && !editing) adjustment.grainSettings.seed = Math.floor(Math.random() * 0xffffffff);
@@ -659,7 +752,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // until the result is put back (runEditJob clears it then); the document is busy meanwhile.
     if (!identity && edit.target === "layer" && get().usesJob(edit.layerId)) {
       cancelSettle();
-      set({ adjustEdit: null });
+      set({ adjustEdit: null, ...(get().colorPicker?.target.kind === "gradientMap" ? { colorPicker: null } : {}) });
       void get().runEditJob(command, edit.layerId);
       return;
     }
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -136,3 +136,36 @@ body { margin: 0; }
 /* The selection tools' header (Phase 4a). */
 .tool-options .segmented button[aria-pressed="true"] { border-color: #3a6ea5; background: #3a6ea5; color: #fff; }
 .tool-options .hint { color: #999; }
+
+/* The palette at the foot of the tool rail (ColorPaletteControls.swift): 24 px swatches, the background
+   one 12 px down and right of the foreground one, swap and default-colour buttons beside them. */
+.tool-rail .palette { position: relative; width: 36px; height: 36px; margin: auto 4px 12px; flex: 0 0 auto; }
+.tool-rail .palette button.palette-swatch { position: absolute; width: 24px; height: 24px; margin: 0; padding: 0; border-radius: 6px; border: 1px solid #000; box-shadow: inset 0 0 0 1.5px #fff; }
+.tool-rail .palette button.palette-swatch.foreground { left: 0; top: 0; z-index: 1; }
+.tool-rail .palette button.palette-swatch.background { left: 12px; top: 12px; }
+.tool-rail .palette button.palette-swap { position: absolute; left: 25px; top: -4px; width: 13px; height: 13px; margin: 0; border-radius: 3px; color: #bbb; }
+.tool-rail .palette button.palette-reset { position: absolute; left: -2px; top: 26px; width: 13px; height: 13px; margin: 0; border-radius: 3px; }
+.palette-mask-popover { position: absolute; left: 44px; bottom: 0; z-index: 60; display: flex; flex-direction: column; gap: 10px; padding: 12px; width: max-content; background: #2a2a2a; border: 1px solid #000; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5); }
+.palette-mask-popover div { display: flex; gap: 8px; }
+.tool-rail .palette-mask-popover button { width: auto; height: auto; margin: 0; padding: 4px 10px; background: #3a3a3a; border: 1px solid #000; border-radius: 4px; color: #eee; }
+/* The colour picker (ColorPickerSheet.swift): it floats over the window, so the canvas stays clickable. */
+.color-picker { position: fixed; z-index: 70; background: #2a2a2a; border: 1px solid #000; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5); }
+.color-picker-title { padding: 6px 10px; font-weight: 600; border-bottom: 1px solid #000; cursor: move; user-select: none; }
+.color-picker-body { display: flex; gap: 14px; padding: 20px; align-items: flex-start; }
+.picker-field { position: relative; outline: 1px solid rgba(0, 0, 0, 0.6); cursor: crosshair; overflow: hidden; touch-action: none; }
+.picker-marker { position: absolute; width: 12px; height: 12px; border-radius: 50%; border: 1.5px solid #fff; box-shadow: 0 0 0 0.75px #000; box-sizing: border-box; pointer-events: none; }
+.picker-hue { position: relative; width: 34px; cursor: ns-resize; touch-action: none; }
+.picker-hue-strip { position: absolute; left: 7px; width: 20px; top: 0; bottom: 0; outline: 1px solid rgba(0, 0, 0, 0.6); }
+.picker-hue-arrows { position: absolute; left: 0; width: 34px; height: 10px; pointer-events: none;
+  background: linear-gradient(to bottom right, #ddd 50%, transparent 50%) 0 0 / 7px 5px no-repeat, linear-gradient(to top right, #ddd 50%, transparent 50%) 0 5px / 7px 5px no-repeat,
+  linear-gradient(to bottom left, #ddd 50%, transparent 50%) 27px 0 / 7px 5px no-repeat, linear-gradient(to top left, #ddd 50%, transparent 50%) 27px 5px / 7px 5px no-repeat; }
+.picker-side { display: flex; flex-direction: column; width: 180px; height: 256px; }
+.picker-top { display: flex; gap: 16px; }
+.picker-new { width: 64px; height: 64px; border-radius: 5px; outline: 1px solid rgba(0, 0, 0, 0.6); }
+.picker-buttons { display: flex; flex-direction: column; gap: 8px; width: 90px; }
+.picker-fields { display: flex; flex-direction: column; gap: 6px; margin-top: auto; }
+.picker-channel { display: flex; align-items: center; gap: 8px; }
+.picker-channel input { width: 52px; }
+.picker-channel input[aria-label="Hex color"] { width: 84px; font-family: monospace; }
+.picker-hint { margin-top: 8px; font-size: 11px; color: #999; }
+.color-swatch-button { width: 36px; height: 18px; padding: 0; border: 1px solid rgba(0, 0, 0, 0.5); border-radius: 3px; cursor: pointer; }
```


- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test`: 182 (+7). `pnpm build`; `pnpm e2e`: 144 passed, 9 skipped (+5).

- [ ] **Step 5: Prove it bites**

(1) In the picker's key handler, let Escape go on to other handlers (drop `stopImmediatePropagation`): the Gradient Map e2e fails at "Escape went to the picker alone" (measured). Restore. (2) In `setMaskSelected`, stop closing a palette picker: "does not open on a swatch while a mask is the target, and targeting one closes it uncommitted" fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/canvas/sampling.ts app/src/panels/ColorPickerPanel.tsx app/src/panels/PaletteSwatches.tsx app/tests/e2e/color-picker.spec.ts app/tests/unit/picker-store.test.ts
git commit -m "feat(app): the palette's swatches and the colour picker, sampling the canvas and editing Gradient Map ends" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/App.tsx app/src/actions/layers.ts app/src/canvas/CanvasView.tsx app/src/canvas/overlay.ts app/src/canvas/sampling.ts app/src/panels/ColorPickerPanel.tsx app/src/panels/FilterPanel.tsx app/src/panels/PaletteSwatches.tsx app/src/panels/ToolRail.tsx app/src/state/store.ts app/src/styles.css app/tests/e2e/color-picker.spec.ts app/tests/unit/picker-store.test.ts
```


---

### Task 13: The Eyedropper

The Eyedropper (I; Lucide's `pipette`) samples the canvas into the image's foreground under a press and a drag, at most once a frame, with the sample ring; it sets the image's foreground even while a mask is the target, as the Mac's does (EditorCanvas.swift:2045-2048; ruling OQ14), does nothing off the canvas or over a transparent pixel, records nothing, and waits while a job's result is to come. `canvas/sampling.ts` now samples into the picker when it is open, else into the foreground with this tool (an adjustment's own armed eyedropper keeps its press).

**Files:**
- Modify: `app/src/state/store.ts` (`"eyedropper"`, `sampleForeground`), `app/src/canvas/sampling.ts`, `app/src/canvas/CanvasView.tsx`, `app/src/panels/ToolRail.tsx`, `app/src/panels/tool-icons.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`, `app/src/styles.css`
- Create tests: `app/tests/e2e/eyedropper.spec.ts`; modify `app/tests/unit/palette-store.test.ts`, `app/tests/unit/keymap.test.ts`, `app/tests/unit/tool-rail.test.tsx`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: the tool `"eyedropper"`, the action `tool-eyedropper` (I), the store's `sampleForeground(at)`.

- [ ] **Step 1: Write the tests**

The unit case: a sample becomes the image's foreground with a mask targeted (the mask's palette still shows black), nothing off the canvas, no sample at all while a job's result is to come. The e2e: I picks the tool; a press on red makes the foreground red and the ring shows red over black; dragging to blue follows, the ring keeping black below; a transparent pixel keeps blue; the swatch shows it; nothing is recorded.

Create `app/tests/e2e/eyedropper.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the Eyedropper (I) samples the canvas into the image's foreground colour.

type Pt = [number, number];

/** A 64 x 48 document: red on the left 32 columns, blue on the right, transparent below row 40,
 * at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const red = await page.evaluate(solidPngBase64, { width: 32, height: 40, color: "#ff0000" });
  const blue = await page.evaluate(solidPngBase64, { width: 32, height: 40, color: "#0000ff" });
  await page.evaluate(async ([r, b]) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(r), "Red", { x: 16, y: 20 });
    api.engine.importImage(doc, bytes(b), "Blue", { x: 48, y: 20 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
  }, [red, blue]);
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
const foreground = (page: Page) => page.evaluate(() => (window as any).__compositor.store.getState().palette.foreground);

test("I picks the Eyedropper; a press sets the foreground to the colour under it, a drag follows, a transparent pixel keeps it", async ({ page }) => {
  await setup(page);
  const depth = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  await page.keyboard.press("i");
  await expect(page.getByTestId("tool-eyedropper")).toHaveClass(/active/);
  const red = await client(page, [10, 10]), blue = await client(page, [50, 10]), clear = await client(page, [50, 44]);
  await page.mouse.move(red.x, red.y); await page.mouse.down();
  expect(await foreground(page)).toEqual({ red: 1, green: 0, blue: 0 });
  const ring = () => page.evaluate(() => (window as any).__compositor.store.getState().sampleRing);
  expect(await ring()).toMatchObject({ sampled: { red: 1, green: 0, blue: 0 }, original: { red: 0, green: 0, blue: 0 } });
  await page.mouse.move(blue.x, blue.y, { steps: 4 });
  await expect.poll(() => foreground(page)).toEqual({ red: 0, green: 0, blue: 1 });
  expect((await ring()).original, "the ring keeps the colour the press began with").toEqual({ red: 0, green: 0, blue: 0 });
  await page.mouse.move(clear.x, clear.y, { steps: 2 });
  await page.mouse.up();
  expect(await foreground(page), "a transparent pixel has no colour to take").toEqual({ red: 0, green: 0, blue: 1 });
  expect(await ring()).toBeNull();
  expect(await page.getByTestId("palette-foreground").evaluate((el) => getComputedStyle(el).backgroundColor)).toBe("rgb(0, 0, 255)");
  const after = await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId].undoDepth; });
  expect(after, "sampling records nothing").toBe(depth);
});
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -337,3 +337,34 @@ test("gradient previews: a drag tick, the settled preview and a patch in a 700 p
     expect(out[`${label}: patch total`]).toBeLessThan(50);
   }
 });
+
+test("eyedropper: a sample and the overlay that shows its ring, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const s0 = api.store.getState();
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      // One layer: the project's 100 megapixels hold no second one at 100 MP.
+      s0.openDocument(doc);
+      await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const ticks: number[] = [];
+      for (let i = 0; i < 20; i++) {
+        const t0 = performance.now();
+        const at = { x: (w * (i + 0.5)) / 20, y: h / 2 };
+        api.store.getState().sampleForeground(at);
+        api.store.getState().setSampleRing({ at: { x: 300, y: 300 }, sampled: api.store.getState().palette.foreground, original: { red: 0, green: 0, blue: 0 } });
+        api.paintOverlay();
+        ticks.push(performance.now() - t0);
+      }
+      api.store.getState().setSampleRing(null);
+      api.store.getState().closeDocument(doc);
+      return { worst: Math.round(10 * Math.max(...ticks)) / 10, mean: Math.round(10 * ticks.reduce((a, b) => a + b, 0) / ticks.length) / 10 };
+    }, [w, h]);
+    out[`${label}: sample and ring, worst ms`] = r.worst; out[`${label}: mean ms`] = r.mean;
+  }
+  console.log(`eyedropper (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) expect(out[`${label}: sample and ring, worst ms`]).toBeLessThan(16);
+});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -24,6 +24,7 @@ describe("keymap", () => {
     expect(matchShortcut(ev("Enter", { ctrlKey: true }))).toBeNull();
     expect(matchShortcut(ev("x"))).toBe("swap-colors");
     expect(matchShortcut(ev("d"))).toBe("default-colors");
+    expect(matchShortcut(ev("i"))).toBe("tool-eyedropper");
     expect(matchShortcut(ev("x", { ctrlKey: true }))).toBeNull();
     expect(matchShortcut(ev("ArrowLeft"))).toBe("nudge-left");
     expect(matchShortcut(ev("ArrowRight", { shiftKey: true }))).toBe("nudge-right");
```

```diff
--- a/app/tests/unit/palette-store.test.ts
+++ b/app/tests/unit/palette-store.test.ts
@@ -63,6 +63,21 @@ describe("the palette", () => {
     useEditor.setState({ palette: { foreground: RED, background: TEAL, maskPaintWhite: true } });
     expect(s().paletteColor(false)).toEqual(RED);
   });
+  it("takes the Eyedropper's sample as the image's foreground, even with a mask targeted, and keeps it off the canvas", () => {
+    install(true, true);
+    const at: { x: number; y: number }[] = [];
+    useEditor.setState({ engine: { sampleColor: (_d: string, p: { x: number; y: number }) => { at.push(p); return p.x < 4 ? [1, 128 / 255, 0] as [number, number, number] : null; } } as never });
+    s().sampleForeground({ x: 1.5, y: 2.5 });
+    expect(at).toEqual([{ x: 1.5, y: 2.5 }]);
+    expect(s().palette.foreground).toEqual({ red: 1, green: 128 / 255, blue: 0 });
+    expect(s().paletteColor(false), "the mask's own palette still shows").toEqual(BLACK);
+    s().sampleForeground({ x: 9, y: 2 });
+    expect(s().palette.foreground).toEqual({ red: 1, green: 128 / 255, blue: 0 });
+    useEditor.setState({ working: true });
+    s().setPaletteColor(RED, false);
+    s().sampleForeground({ x: 0, y: 0 });
+    expect(at.length, "no sample while a job's result is to come").toBe(2);
+  });
   it("does not change while a job's result is to come", () => {
     useEditor.setState({ working: true });
     s().setPaletteColor(RED, false);
```

```diff
--- a/app/tests/unit/tool-rail.test.tsx
+++ b/app/tests/unit/tool-rail.test.tsx
@@ -31,7 +31,7 @@ describe("ToolRail", () => {
   it("shows an icon, not a letter, on every tool, named for assistive technology", () => {
     mount();
     const expected: Record<string, string> = { move: "Move", marquee: "Marquee", lasso: "Lasso", wand: "Magic Wand",
-      crop: "Crop", hand: "Hand", zoom: "Zoom" };
+      crop: "Crop", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
     for (const [id, label] of Object.entries(expected)) {
       const b = button(id);
       expect(b.querySelectorAll("svg").length, id).toBe(1);
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `sampleForeground` does not exist; the rail has no Eyedropper; `keymap` fails at I.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -224,7 +224,7 @@ export function CanvasView() {
   const picking = useEditor((s) => !!s.colorPicker);
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
-    el.style.cursor = sampleMode || picking || isSelectionTool(tool) ? "crosshair" : "";
+    el.style.cursor = sampleMode || picking || tool === "eyedropper" || isSelectionTool(tool) ? "crosshair" : "";
   }, [sampleMode, tool, picking]);
 
   // Drag to pan with the hand tool or the space bar.
```

```diff
--- a/app/src/canvas/sampling.ts
+++ b/app/src/canvas/sampling.ts
@@ -2,14 +2,19 @@ import { useEditor } from "../state/store";
 import type { PaletteColor } from "../tools/color";
 
 /** Where a press on the canvas samples colour (EditorCanvas.swift:1419-1424): into the open colour
- * picker, whatever the tool; else null. */
-export function samplingInto(): "picker" | null {
-  return useEditor.getState().colorPicker ? "picker" : null;
+ * picker, whatever the tool; else into the foreground with the Eyedropper, unless an adjustment's
+ * own eyedropper is armed (it answers the press itself); else null. */
+export function samplingInto(): "picker" | "foreground" | null {
+  const s = useEditor.getState();
+  if (s.colorPicker) return "picker";
+  if (s.tool === "eyedropper" && !s.adjustEdit?.sampleMode && s.activeId) return "foreground";
+  return null;
 }
 
-/** The colour the sample ring shows as sampled: the picker's working colour. */
+/** The colour the sample ring shows as sampled: the picker's working colour, or the foreground. */
 function current(): PaletteColor | null {
-  return useEditor.getState().pickerColor();
+  const s = useEditor.getState();
+  return s.pickerColor() ?? s.palette.foreground;
 }
 
 /** Samples the canvas under the pointer while it is pressed (`sampleColor`, EditorCanvas.swift:2040-2055):
@@ -18,6 +23,7 @@ function current(): PaletteColor | null {
  * so no tool gesture starts under it. Returns the cleanup. */
 export function installSampling(el: HTMLElement, spaceHeld: () => boolean): () => void {
   let original: PaletteColor | null = null;
+  let into: "picker" | "foreground" | null = null;
   let pending: PointerEvent | null = null;
   let frame = 0;
   const view = (e: PointerEvent) => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
@@ -25,12 +31,14 @@ export function installSampling(el: HTMLElement, spaceHeld: () => boolean): () =
     const s = useEditor.getState(); if (!s.activeId || !original) return;
     const vp = s.viewports[s.activeId], d = s.documents[s.activeId];
     const at = view(e);
-    s.sampleIntoPicker(vp.documentPoint(at, { width: d.width, height: d.height }));
+    const point = vp.documentPoint(at, { width: d.width, height: d.height });
+    if (into === "picker") s.sampleIntoPicker(point); else s.sampleForeground(point);
     const sampled = current();
     if (sampled) s.setSampleRing({ at, sampled, original });
   };
   const down = (e: PointerEvent) => {
-    if (e.button !== 0 || spaceHeld() || !samplingInto()) return;
+    into = e.button === 0 && !spaceHeld() ? samplingInto() : null;
+    if (!into) return;
     e.stopImmediatePropagation();
     original = current();
     el.setPointerCapture(e.pointerId);
```

```diff
--- a/app/src/panels/ToolRail.tsx
+++ b/app/src/panels/ToolRail.tsx
@@ -5,7 +5,8 @@ import { PaletteSwatches } from "./PaletteSwatches";
 const TOOLS: { id: Tool; label: string; key: string }[] = [
   { id: "move", label: "Move", key: "V" }, { id: "marquee", label: "Marquee", key: "M" },
   { id: "lasso", label: "Lasso", key: "L" }, { id: "wand", label: "Magic Wand", key: "W" },
-  { id: "crop", label: "Crop", key: "C" }, { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" },
+  { id: "crop", label: "Crop", key: "C" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
+  { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" },
 ];
 export function ToolRail() {
   const tool = useEditor((s) => s.tool); const setTool = useEditor((s) => s.setTool);
```

```diff
--- a/app/src/panels/tool-icons.tsx
+++ b/app/src/panels/tool-icons.tsx
@@ -19,7 +19,7 @@ import type { LassoKind, MarqueeKind } from "../tools/selection-draft";
 /** Which icon a tool shows: the Marquee and the Lasso follow their mode, as on the Mac
  *  (ContentView.swift: circle.dashed in Ellipse mode, its own icon for the polygonal lasso). */
 export type ToolIconName = "move" | "marquee-rectangle" | "marquee-ellipse" | "lasso-freehand" | "lasso-polygonal"
-  | "wand" | "crop" | "hand" | "zoom";
+  | "wand" | "crop" | "eyedropper" | "hand" | "zoom";
 
 export function toolIconName(tool: Tool, marquee: MarqueeKind, lasso: LassoKind): ToolIconName {
   switch (tool) {
@@ -56,6 +56,11 @@ const SHAPES: Record<ToolIconName, ReactNode> = {
   </>,
   // lucide crop (SF crop)
   "crop": <><path d="M6 2v14a2 2 0 0 0 2 2h14" /><path d="M18 22V8a2 2 0 0 0-2-2H2" /></>,
+  // lucide pipette (SF eyedropper)
+  "eyedropper": <>
+    <path d="m12 9-8.414 8.414A2 2 0 0 0 3 18.828v1.344a2 2 0 0 1-.586 1.414A2 2 0 0 1 3.828 21h1.344a2 2 0 0 0 1.414-.586L15 12" />
+    <path d="m18 9 .4.4a1 1 0 1 1-3 3l-3.8-3.8a1 1 0 1 1 3-3l.4.4 3.4-3.4a1 1 0 1 1 3 3z" /><path d="m2 22 .414-.414" />
+  </>,
   // lucide hand (SF hand.draw)
   "hand": <>
     <path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2" /><path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2" />
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -5,7 +5,7 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
   | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
-  | "swap-colors" | "default-colors";
+  | "swap-colors" | "default-colors" | "tool-eyedropper";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -39,6 +39,7 @@ export const SHORTCUTS = {
   "cycle-tool-mode": [{ key: "Tab" }],
   // The palette (EditorCanvas.swift:1835-1836).
   "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
+  "tool-eyedropper": [{ key: "i" }],
 } satisfies Record<ActionId, Shortcut[]>;
 
 export function matchShortcut(e: KeyboardEvent): ActionId | null {
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -78,6 +78,7 @@ export function runAction(id: ActionId, shift = false): void {
     case "tool-marquee": s.setTool("marquee"); break;
     case "tool-lasso": s.setTool("lasso"); break;
     case "tool-wand": s.setTool("wand"); break;
+    case "tool-eyedropper": s.setTool("eyedropper"); break;
     case "select-all": if (doc) s.run({ type: "SelectAll" }); break;
     case "deselect": if (doc?.selection) s.run({ type: "Deselect" }); break;
     case "select-inverse": if (doc?.selection) s.run({ type: "InvertSelection" }); break;
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -15,7 +15,7 @@ import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind,
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
 import { BLACK, WHITE, hsbOf, hsbToRgb, quantized, sameColor, withRgb, type PaletteColor, type PickerHSB } from "../tools/color";
 
-export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand";
+export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
 /** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
 export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
@@ -152,6 +152,10 @@ export interface EditorStore {
   /** Loads the canvas colour under a document point into the open picker (`sampleIntoColorPicker`). */
   sampleIntoPicker(at: { x: number; y: number }): void;
   setPickerAt(at: { x: number; y: number }): void;
+  /** The Eyedropper: the canvas colour under a document point becomes the image's foreground, even
+   * while a mask is the target (`sampleColor`, EditorCanvas.swift:2045-2048). Nothing off the canvas,
+   * over a transparent pixel, or while a job's result is to come. */
+  sampleForeground(at: { x: number; y: number }): void;
   setSampleRing(ring: SampleRing | null): void;
   setEngine(engine: EngineClient): void;
   setJobs(jobs: JobClient): void;
@@ -316,6 +320,11 @@ export const useEditor = create<EditorStore>((set, get) => ({
     if (rgb) get().setPickerHsb(withRgbFrom(colorPicker.hsb, rgb));
   },
   setPickerAt: (pickerAt) => set({ pickerAt }),
+  sampleForeground: (at) => {
+    const { engine, activeId, working } = get(); if (!engine || !activeId || working) return;
+    const rgb = engine.sampleColor(activeId, at);
+    if (rgb) set({ palette: { ...get().palette, foreground: { red: rgb[0], green: rgb[1], blue: rgb[2] } } });
+  },
   setSampleRing: (sampleRing) => { set({ sampleRing }); get().repaintOverlay(); },
   maskTargeted: () => {
     const { activeId, documents, maskSelected } = get();
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -139,7 +139,7 @@ body { margin: 0; }
 
 /* The palette at the foot of the tool rail (ColorPaletteControls.swift): 24 px swatches, the background
    one 12 px down and right of the foreground one, swap and default-colour buttons beside them. */
-.tool-rail .palette { position: relative; width: 36px; height: 36px; margin: auto 4px 12px; flex: 0 0 auto; }
+.tool-rail .palette { position: relative; width: 36px; height: 36px; margin: 12px 4px 12px; flex: 0 0 auto; }
 .tool-rail .palette button.palette-swatch { position: absolute; width: 24px; height: 24px; margin: 0; padding: 0; border-radius: 6px; border: 1px solid #000; box-shadow: inset 0 0 0 1.5px #fff; }
 .tool-rail .palette button.palette-swatch.foreground { left: 0; top: 0; z-index: 1; }
 .tool-rail .palette button.palette-swatch.background { left: 12px; top: 12px; }
```


- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test`: 183 (+1). `pnpm build`; `pnpm e2e`: 145 passed, 10 skipped. Timings (release wasm, Edge, `-g eyedropper`): a sample and the overlay that shows its ring, 4.6 ms at worst at 24 MP and 0.9 ms at 100 MP (budget 16).

- [ ] **Step 5: Prove it bites**

(1) In `sampleForeground`, drop the `working` guard: the unit case fails at "no sample while a job's result is to come" (measured). Restore. (2) In `samplingInto`, sample for the Gradient's Alt only, not the Eyedropper: the e2e fails at the first press (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/eyedropper.spec.ts
git commit -m "feat(app): the Eyedropper samples the canvas into the foreground colour" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/canvas/CanvasView.tsx app/src/canvas/sampling.ts app/src/panels/ToolRail.tsx app/src/panels/tool-icons.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/src/state/store.ts app/src/styles.css app/tests/e2e/eyedropper.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/keymap.test.ts app/tests/unit/palette-store.test.ts app/tests/unit/tool-rail.test.tsx
```


---

### Task 14: Fill with the foreground or background colour

Alt+Backspace / Alt+Delete fill the selection (or the layer, grown to the canvas) with the foreground colour, Ctrl+Backspace / Ctrl+Delete with the background colour, and Edit > Fill with Foreground / Background Color does the same (CompositorApp.swift:175-186; ruling OQ15); on a targeted mask the colour is its black or white. Fill needs the Mac's `canPaint` and no crop rectangle pending, and a large layer is filled by the job worker. Delete with a selection on a targeted mask now fills it with the mask's background colour, as the Mac's `clearSelectedPixels` does.

**Files:**
- Modify: `app/src/actions/layers.ts` (`canPaint`, `fillActive`, `deleteKeyPressed`), `app/src/panels/MenuBar.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts`
- Create tests: `app/tests/unit/fill-actions.test.ts`, `app/tests/e2e/fill.spec.ts`; modify `app/tests/unit/keymap.test.ts`, `app/tests/unit/selection-store.test.ts` (Delete on a mask: a `Fill` with white)

**Interfaces:**
- Produces: `canPaint()`, `fillActive(background)`; the actions `fill-foreground`, `fill-background`; the menu items `fill-foreground`, `fill-background`.

- [ ] **Step 1: Write the tests**

The unit tests: the commands for the pixels (the palette's colours) and for a targeted mask (black, then white); a large layer goes to the worker; every case where the Mac's `canPaint` is false refuses (a folder's pixels, an adjustment layer, a hidden layer, a disabled mask as the target, an empty selection, two layers selected, an open panel, a job's result to come, a crop rectangle pending), and their opposites paint; Delete on a targeted mask fills with the mask's background colour (black after a swap), and on pixels still clears. The e2e: Alt+Backspace fills the selection red and nothing outside, one undo step; Ctrl+Delete without a selection fills the layer blue and grows it to the 60 x 40 canvas; Backspace alone still deletes; on a mask the Edit menu's fill hides with black, and a hidden layer greys both items out; with `jobPixels` 0 the worker's fill equals the fill in place, byte for byte.

Create `app/tests/e2e/fill.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { clickMenu, solidPngBase64 } from "./helpers";

// Phase 4b-1: Fill with the foreground or background colour, from the keys and the Edit menu,
// through a selection, on a mask, and through the job worker (the engine's Fill is pinned in
// engine/tests/raster_edits.rs).

/** A 60 x 40 canvas with a 30 x 20 grey layer at (10, 10), active, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 30, height: 20, color: "#808080" });
  await page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(60, 40, false);
    api.engine.importImage(doc, bytes(g), "Grey", { x: 25, y: 20 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
    const s = api.store.getState();
    s.setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
    s.setPaletteColor({ red: 0, green: 0, blue: 1 }, true);
  }, grey);
}
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, [x, y]);
const state = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const select = (page: Page, x0: number, y0: number, x1: number, y1: number) => page.evaluate(([x0, y0, x1, y1]) =>
  (window as any).__compositor.store.getState().run({ type: "SelectShape", kind: "Rectangle", points: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]], mode: "Replace", antialiased: false }), [x0, y0, x1, y1]);

test("Alt+Backspace fills the selection with the foreground, Ctrl+Delete the layer (grown to the canvas) with the background", async ({ page }) => {
  await setup(page);
  await select(page, 15, 12, 25, 18);
  const depth = (await state(page)).undoDepth;
  await page.keyboard.press("Alt+Backspace");
  expect(await pixel(page, 20, 15)).toEqual([255, 0, 0, 255]);
  expect(await pixel(page, 12, 15), "outside the selection").toEqual([128, 128, 128, 255]);
  expect((await state(page)).undoDepth).toBe(depth + 1);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "Deselect" }));
  await page.keyboard.press("Control+Delete");
  // No selection: the whole layer, grown to cover the canvas as the Mac's raster edit grows it.
  expect(await pixel(page, 2, 2)).toEqual([0, 0, 255, 255]);
  const layer = (await state(page)).layers[0];
  expect([layer.pixelsWidth, layer.pixelsHeight, layer.transform.origin]).toEqual([60, 40, [0, 0]]);
  // Backspace alone still deletes: with no selection, the layer.
  await page.keyboard.press("Backspace");
  expect((await state(page)).layers.length).toBe(0);
});

test("on a targeted mask the fill is its black or white, and the Edit menu offers both fills only when they can paint", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true }); s.setMaskSelected(true); });
  await select(page, 10, 10, 20, 30);
  await clickMenu(page, "Edit", "fill-foreground");
  expect(await pixel(page, 15, 15), "black hides").toEqual([0, 0, 0, 0]);
  expect(await pixel(page, 25, 15)).toEqual([128, 128, 128, 255]);
  // Hidden, the layer takes no fill: both items grey out.
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "SetLayerVisible", id: s.documents[s.activeId].activeLayerId, visible: false }); });
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await expect(page.getByTestId("menu-fill-foreground")).toBeDisabled();
  await expect(page.getByTestId("menu-fill-background")).toBeDisabled();
});

test("a large layer is filled by the job worker, exactly as in place", async ({ page }) => {
  await setup(page);
  await select(page, 15, 12, 45, 38);
  const inPlace = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState(); const id = s.documents[s.activeId].activeLayerId;
    api.engine.execute(s.activeId, { type: "Fill", id, mask: false, color: [1, 0, 0] });
    const bytes = Array.from(api.engine.layerPixels(s.activeId, id, 0) as Uint8Array);
    api.engine.undo(s.activeId); s.refresh(s.activeId);
    return bytes;
  });
  await page.evaluate(() => (window as any).__compositor.store.setState({ jobPixels: 0 }));
  const depth = (await state(page)).undoDepth;
  await page.keyboard.press("Alt+Backspace");
  // Put back as one undo step once the worker is done.
  await expect.poll(async () => (await state(page)).undoDepth).toBe(depth + 1);
  const viaJob = await page.evaluate(() => {
    const api = (window as any).__compositor; const s = api.store.getState(); const id = s.documents[s.activeId].activeLayerId;
    return Array.from(api.engine.layerPixels(s.activeId, id, 0) as Uint8Array);
  });
  expect(viaJob.length).toBe(inPlace.length);
  expect(viaJob).toEqual(inPlace);
});
```

Create `app/tests/unit/fill-actions.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { canPaint, deleteKeyPressed, fillActive } from "../../src/actions/layers";
import type { Command, DocumentState, LayerState, SelectionState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient } from "../../src/engine/jobs";

// Fill (SelectionEdits.swift:40-58) and the rule that gates it (`canPaint`, EditorSession+Brush.swift:5-11).
function layer(patch: Partial<LayerState> = {}): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [40, 30], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 40, pixelsHeight: 30, pixelsRevision: 1, hasPixels: true, hasMask: true, maskWidth: 40, maskHeight: 30,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...patch };
}
const SELECTION: SelectionState = { revision: 3, empty: false, bounds: { x: 1, y: 2, width: 5, height: 6 }, antialiased: true, feather: 0, points: 4 };
function doc(l: LayerState, selection: SelectionState | null = null): DocumentState {
  return { id: "D", documentId: "D", width: 40, height: 30, resolution: 72, activeLayerId: l.id, canUndo: false, canRedo: false, isModified: false,
    undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection, layers: [l] };
}
let log: string[] = [];
function install(l: LayerState = layer(), selection: SelectionState | null = null) {
  log = [];
  const state = doc(l, selection);
  const engine = {
    state: () => state,
    execute: (_id: string, c: Command) => { log.push(JSON.stringify(c)); return { structure: true, canvas: false, layers: [] }; },
    jobInput: () => { log.push("job input"); return { input: "{}", pixels: new ArrayBuffer(4), mask: null }; },
    setPreview: () => ({ structure: true, canvas: false, layers: [] }),
  } as unknown as EngineClient;
  const jobs = { run: () => new Promise(() => {}) } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [l.id], maskSelected: false,
    working: false, palette: { ...DEFAULT_PALETTE, foreground: { red: 1, green: 0.5, blue: 0 }, background: { red: 0, green: 0, blue: 1 } },
    adjustEdit: null, transformEdit: null, error: null, tool: "move", cropRect: null, sheet: null, colorPicker: null });
}
const s = () => useEditor.getState();

describe("Fill", () => {
  beforeEach(() => install());
  it("fills the pixels with the foreground or background colour, and a targeted mask with its black or white", () => {
    fillActive(false);
    fillActive(true);
    useEditor.setState({ maskSelected: true });
    fillActive(false);
    fillActive(true);
    expect(log.map((c) => JSON.parse(c))).toEqual([
      { type: "Fill", id: "A", mask: false, color: [1, 0.5, 0] },
      { type: "Fill", id: "A", mask: false, color: [0, 0, 1] },
      { type: "Fill", id: "A", mask: true, color: [0, 0, 0] },
      { type: "Fill", id: "A", mask: true, color: [1, 1, 1] },
    ]);
  });
  it("sends a large layer to the job worker", () => {
    useEditor.setState({ jobPixels: 40 * 30 - 1 });
    fillActive(false);
    expect(log).toEqual(["job input"]);
    expect(s().working).toBe(true);
  });
  it("is refused wherever the Mac's canPaint is false", () => {
    const refusals: [string, () => void][] = [
      ["a folder's pixels", () => install(layer({ isGroup: true, hasPixels: false }))],
      ["an adjustment layer", () => install(layer({ adjustment: { kind: "Invert" } as never }))],
      ["a hidden layer", () => install(layer({ visible: false }))],
      ["a disabled mask as the target", () => { install(layer({ maskEnabled: false })); useEditor.setState({ maskSelected: true }); }],
      ["an empty selection", () => install(layer(), { ...SELECTION, empty: true })],
      ["two layers selected", () => { install(); useEditor.setState({ selectedLayerIds: ["A", "B"] }); }],
      ["an open panel", () => { install(); useEditor.setState({ adjustEdit: { kind: "Levels" } as never }); }],
      ["a job's result to come", () => { install(); useEditor.setState({ working: true }); }],
      ["a crop rectangle pending", () => { install(); useEditor.setState({ tool: "crop", cropRect: { x: 0, y: 0, width: 5, height: 5 } }); }],
    ];
    for (const [why, arrange] of refusals) {
      arrange();
      expect(canPaint(), why).toBe(false);
      fillActive(false);
      expect(log, why).toEqual([]);
    }
    // Their opposites paint: a folder's enabled mask as the target, and a selection with something in it.
    install(layer({ isGroup: true, hasPixels: false }));
    useEditor.setState({ maskSelected: true });
    expect(canPaint()).toBe(true);
    install(layer(), SELECTION);
    expect(canPaint()).toBe(true);
  });
  it("makes Delete on a targeted mask fill the selection with the mask's background colour", () => {
    install(layer(), SELECTION);
    useEditor.setState({ maskSelected: true, palette: { ...DEFAULT_PALETTE, maskPaintWhite: true } });
    deleteKeyPressed();
    expect(log.map((c) => JSON.parse(c))).toEqual([{ type: "Fill", id: "A", mask: true, color: [0, 0, 0] }]);
    install(layer(), SELECTION);
    deleteKeyPressed();
    expect(log.map((c) => JSON.parse(c))).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: false }]);
  });
});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -25,6 +25,9 @@ describe("keymap", () => {
     expect(matchShortcut(ev("x"))).toBe("swap-colors");
     expect(matchShortcut(ev("d"))).toBe("default-colors");
     expect(matchShortcut(ev("i"))).toBe("tool-eyedropper");
+    expect(matchShortcut(ev("Backspace", { altKey: true }))).toBe("fill-foreground");
+    expect(matchShortcut(ev("Delete", { ctrlKey: true }))).toBe("fill-background");
+    expect(matchShortcut(ev("Backspace"))).toBe("delete-layer");
     expect(matchShortcut(ev("x", { ctrlKey: true }))).toBeNull();
     expect(matchShortcut(ev("ArrowLeft"))).toBe("nudge-left");
     expect(matchShortcut(ev("ArrowRight", { shiftKey: true }))).toBe("nudge-right");
```

```diff
--- a/app/tests/unit/selection-store.test.ts
+++ b/app/tests/unit/selection-store.test.ts
@@ -47,7 +47,8 @@ describe("Delete", () => {
     let commands = stub(document([layer("A", { hasMask: true })], selected()));
     useEditor.setState({ maskSelected: true });
     deleteKeyPressed();
-    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: true }]);
+    // With the mask palette at its default, white is the background it fills with (Phase 4b-1).
+    expect(commands).toEqual([{ type: "Fill", id: "A", mask: true, color: [1, 1, 1] }]);
     for (const doc of [document([layer("A")], selected(true)), document([layer("A", { visible: false })], selected()),
       document([layer("A", { hasMask: true, maskEnabled: false })], selected())]) {
       commands = stub(doc);
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `fill-actions.test.ts` does not compile (`fillActive`); `keymap` fails at Alt+Backspace (`delete-layer`); `selection-store.test.ts` still sees `ClearSelectedPixels`.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -54,7 +54,35 @@ export function deleteKeyPressed(): void {
   const c = ctx(); if (!c) return;
   if (!c.doc.selection) { deleteSelected(); return; }
   if (!canClearSelected()) return;
-  c.s.run({ type: "ClearSelectedPixels", id: c.active!.id, mask: c.s.maskSelected && c.active!.hasMask });
+  // On a mask the selection fills with the mask's background colour, as in Photoshop
+  // (`clearSelectedPixels`, SelectionEdits.swift:53-58); white unless the palette was swapped.
+  if (c.s.maskTargeted()) { fillActive(true); return; }
+  c.s.run({ type: "ClearSelectedPixels", id: c.active!.id, mask: false });
+}
+
+/** Whether the active layer's pixels, or its mask, can take a fill now (`canPaint`,
+ * EditorSession+Brush.swift:5-11): one layer selected and shown, not a folder unless its mask is the
+ * target, an enabled mask when it is, not an adjustment layer, no empty selection, no panel open, no
+ * job's result to come, no crop rectangle pending. */
+export function canPaint(): boolean {
+  const c = ctx(); if (!c?.active || c.selected.length !== 1 || c.s.panelOwnsDocument() || c.s.working) return false;
+  if (c.s.tool === "crop" && c.s.cropRect) return false;
+  if (c.doc.selection?.empty || !visibleIds(c.doc).has(c.active.id)) return false;
+  const mask = c.s.maskTargeted();
+  if (mask) return c.active.maskEnabled;
+  return !c.active.isGroup && !c.active.adjustment;
+}
+
+/** Alt+Backspace / Ctrl+Backspace: the selection (or the whole layer) filled with the foreground or
+ * background colour, one undo step; on a mask its black or white (`fillSelection`,
+ * SelectionEdits.swift:40-50). A large layer is filled by the job worker. */
+export function fillActive(background: boolean): void {
+  if (!canPaint()) return;
+  const c = ctx()!;
+  const mask = c.s.maskTargeted();
+  const command = { type: "Fill" as const, id: c.active!.id, mask, color: colorTuple(c.s.paletteColor(background)) };
+  if (c.s.usesJob(c.active!.id)) void c.s.runEditJob(command, c.active!.id);
+  else c.s.run(command);
 }
 /** Ctrl-click on a thumbnail, or Select > Layer's Pixels / Mask's Black Areas (MaskTracing.swift:73-94). */
 export function loadSelection(id: string, mask: boolean, mode: SelectionMode = "Replace"): void {
```

```diff
--- a/app/src/panels/MenuBar.tsx
+++ b/app/src/panels/MenuBar.tsx
@@ -4,7 +4,7 @@ import { importImages, openProject } from "../actions/files";
 import { runAction } from "../shortcuts/useShortcuts";
 import { activeLayer } from "../state/selection";
 import {
-  addAdjustmentLayer, addMaskToActive, blurMaskOfActive, canClipActive, canEditAdjustment, canInvert, canMoveActiveBy,
+  addAdjustmentLayer, addMaskToActive, blurMaskOfActive, canClipActive, canEditAdjustment, canInvert, canMoveActiveBy, canPaint,
   deleteMaskOfActive, deleteSelected, editAdjustmentLayer, fillMaskOfActive, flipSelected, invertMaskOfActive,
   loadSelection, mergeTitle, toggleMaskEnabled, toggleMaskLink,
 } from "../actions/layers";
@@ -51,6 +51,9 @@ export function MenuBar() {
       // makes both inert as well (store.undo/redo), so they grey out for it too.
       { id: "undo", label: "Undo", run: () => runAction("undo"), enabled: editable && !s.transformEdit && !!activeDoc?.canUndo },
       { id: "redo", label: "Redo", run: () => runAction("redo"), enabled: editable && !s.transformEdit && !!activeDoc?.canRedo },
+      "separator",
+      { id: "fill-foreground", label: "Fill with Foreground Color", run: () => runAction("fill-foreground"), enabled: editable && canPaint() },
+      { id: "fill-background", label: "Fill with Background Color", run: () => runAction("fill-background"), enabled: editable && canPaint() },
     ] },
     { title: "Layer", items: [
       { id: "layer-new", label: "New Layer", run: () => runAction("new-layer"), enabled: editable },
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -5,7 +5,7 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
   | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
-  | "swap-colors" | "default-colors" | "tool-eyedropper";
+  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -40,6 +40,9 @@ export const SHORTCUTS = {
   // The palette (EditorCanvas.swift:1835-1836).
   "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
   "tool-eyedropper": [{ key: "i" }],
+  // Photoshop's fill keys (CompositorApp.swift:175-186: Option-Delete and Command-Delete on the Mac).
+  "fill-foreground": [{ key: "Backspace", alt: true }, { key: "Delete", alt: true }],
+  "fill-background": [{ key: "Backspace", ctrl: true }, { key: "Delete", ctrl: true }],
 } satisfies Record<ActionId, Shortcut[]>;
 
 export function matchShortcut(e: KeyboardEvent): ActionId | null {
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -6,7 +6,7 @@ import { isEditableTarget } from "./target";
 import { nudgeDelta } from "../tools/transform-session";
 import type { Corners, PointTuple } from "../engine/types";
 import { activeLayer } from "../state/selection";
-import { addFolder, cycleBlendMode, deleteKeyPressed, duplicateSelected, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";
+import { addFolder, cycleBlendMode, deleteKeyPressed, duplicateSelected, fillActive, groupSelected, invertActive, mergeSelected, moveActiveBy, setOpacityOfSelected, toggleClippingOfActive } from "../actions/layers";
 import { isSelectionTool } from "../tools/selection-draft";
 
 const NUDGE_KEYS: Partial<Record<ActionId, string>> = { "nudge-left": "ArrowLeft", "nudge-right": "ArrowRight", "nudge-up": "ArrowUp", "nudge-down": "ArrowDown" };
@@ -79,6 +79,8 @@ export function runAction(id: ActionId, shift = false): void {
     case "tool-lasso": s.setTool("lasso"); break;
     case "tool-wand": s.setTool("wand"); break;
     case "tool-eyedropper": s.setTool("eyedropper"); break;
+    case "fill-foreground": if (doc && !s.sheet) fillActive(false); break;
+    case "fill-background": if (doc && !s.sheet) fillActive(true); break;
     case "select-all": if (doc) s.run({ type: "SelectAll" }); break;
     case "deselect": if (doc?.selection) s.run({ type: "Deselect" }); break;
     case "select-inverse": if (doc?.selection) s.run({ type: "InvertSelection" }); break;
```


- [ ] **Step 4: Run the tests and watch them pass**

`pnpm test`: 187 (+4). `pnpm build`; `pnpm e2e`: 148 passed, 10 skipped (+3). Task 16's perf test times a whole-layer Fill through the worker: the page's frames stay 17 ms apart at 24 and 100 MP (budget 100), the key press itself 97 / 299 ms (the copy out), done after 2.2 / 5.9 s.

- [ ] **Step 5: Prove it bites**

(1) In `canPaint`, drop the visibility check: "is refused wherever the Mac's canPaint is false" fails at the hidden layer (measured). Restore. (2) In `deleteKeyPressed`, go back to `ClearSelectedPixels` on a mask: "makes Delete on a targeted mask fill the selection with the mask's background colour" fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/tests/e2e/fill.spec.ts app/tests/unit/fill-actions.test.ts
git commit -m "feat(app): Fill with the foreground or background colour from the keys and the Edit menu" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/actions/layers.ts app/src/panels/MenuBar.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/tests/e2e/fill.spec.ts app/tests/unit/fill-actions.test.ts app/tests/unit/keymap.test.ts app/tests/unit/selection-store.test.ts
```


---

### Task 15: The Gradient tool

The Gradient tool (G; drawn for this port after SF `square.bottomhalf.filled`): a press starts a line on the active layer or its targeted mask, a drag moves its end, and the line stays pending with draggable ends until Return or Apply paints it (rulings OQ7, OQ8, OQ10). The bar (GradientControls.swift) has Linear / Radial (Tab), the stops over a checkerboard, Foreground to Background / to Transparent, Reverse, Opacity 1-100 % (the digits), "Mask" while a mask is the target, and Cancel / Apply while pending. The overlay draws the line (black under white), the radial rim dashed, and the ends as discs showing the stops. The pending gradient is the engine's preview (Task 9); `state/gradient-edit.ts` holds the settings, the stops and Shift's 45-degree snap. Alt with the tool samples the foreground. Committing goes through the worker on a large layer - chosen by the stored layer's size, not the reduced preview `state()` reports (ruling OQ9, found by this task's perf run; `Engine::stored_pixels`).

**Files:**
- Create: `app/src/state/gradient-edit.ts`, `app/src/panels/GradientOptions.tsx`, `app/src/canvas/gradient-tool.ts`
- Modify: `app/src/state/store.ts` (`"gradient"`, `gradientOptions`, `gradientEdit`, their actions, `canPaintNow`, `usesJob` by stored size, the pending edit's rules in `run`, `undo`, `redo`, `setTool`, `selectLayers`, `setMaskSelected`, the document switches and the palette), `app/src/actions/layers.ts` (`canPaint` through `canPaintNow`), `app/src/canvas/CanvasView.tsx`, `app/src/canvas/overlay.ts`, `app/src/canvas/sampling.ts` (Alt), `app/src/panels/MenuBar.tsx` (Undo while pending), `app/src/panels/ToolRail.tsx`, `app/src/panels/tool-icons.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts` (G, Enter, Escape, the digits), `app/src/App.tsx`, `app/src/styles.css`, `app/src/engine/client.ts` (`storedPixels`), `engine/src/engine.rs` (`stored_pixels`), `engine-wasm/src/lib.rs`
- Create tests: `app/tests/unit/gradient-store.test.ts`, `app/tests/e2e/gradient-tool.spec.ts`; modify `engine/tests/gradient_preview.rs`, `app/tests/unit/store-jobs.test.ts` and `fill-actions.test.ts` (their stub engines answer `storedPixels`), `keymap.test.ts`, `tool-rail.test.tsx`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `GradientOptions`, `DEFAULT_GRADIENT`, `GRADIENT_STYLES`, `GradientEdit`, `hasLine`, `GRADIENT_HANDLE_PX`, `gradientStops`, `gradientSpec`, `snapped45`; the store's `gradientOptions`, `gradientEdit`, `setGradientOptions`, `beginGradient`, `moveGradient`, `endGradientDrag`, `refreshGradient`, `cancelGradient`, `commitGradient`, `canPaintNow`; `installGradientTool`, `gradientLine`; `Engine::stored_pixels`, `EngineClient.storedPixels`; the action `tool-gradient` (G).

- [ ] **Step 1: Write the tests**

The unit tests: the stops for each style and Reverse; Shift's snap ((10, 3) from the origin is flat at 10.44; (-7, -9) from (1, 1) is -135 degrees); a drag previews reduced and the release settled, recording nothing; a click without a line leaves nothing; Return applies one Gradient with the previewed spec and Escape drops it; the first Undo discards it and the next undoes; a change of tool, layer or target, or another command, applies it first; its settings and the palette re-render it; a mask is painted black and white; nothing starts where nothing can be painted; a large layer (by stored size, even under a reduced preview) goes to the worker; the digits set its opacity (at least 1 %) and Tab its shape. The engine test adds `stored_pixels` under a reduced preview. The e2e: a dragged line previews without recording, each pixel the formula's (`expected`), and Return applies exactly what was previewed; an end dragged within 10 px moves alone, Shift holds 45 degrees, Escape restores grey and the first Ctrl+Z only discards; Tab, the digits and Alt-click sampling (then a radial red at 50 %: the formula at pixel (32, 24)); a mask gradient hides from the left.

Create `app/tests/e2e/gradient-tool.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Phase 4b-1: the Gradient tool through the real canvas and keys (the engine's painting is pinned in
// engine/tests/raster_edits.rs and its previews in gradient_preview.rs).

type Pt = [number, number];

/** A 64 x 48 grey layer over its whole canvas, at 4 CSS px a pixel; red foreground, blue background. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 64, height: 48, color: "#808080" });
  await page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = (data: string) => Uint8Array.from(atob(data), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(64, 48, false);
    api.engine.importImage(doc, bytes(g), "Grey", { x: 32, y: 24 });
    api.store.getState().openDocument(doc);
    await api.setZoom(4);
    const s = api.store.getState();
    s.setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
    s.setPaletteColor({ red: 0, green: 0, blue: 1 }, true);
  }, grey);
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
async function drag(page: Page, from: Pt, to: Pt, shift = false) {
  const a = await client(page, from), b = await client(page, to);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  if (shift) await page.keyboard.down("Shift");
  await page.mouse.move(b.x, b.y, { steps: 6 }); await page.mouse.up();
  if (shift) await page.keyboard.up("Shift");
}
const store = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return { edit: s.gradientEdit, options: s.gradientOptions, depth: s.documents[s.activeId].undoDepth, fg: s.palette.foreground }; });
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
}, [x, y]);
/** What a linear red-to-transparent gradient from `start` to `end` leaves over opaque grey 128 at
 * pixel (x, y)'s centre: straight red at alpha 1 - t, source-over, each channel rounded half up. */
function expected(start: { x: number; y: number }, end: { x: number; y: number }, x: number, y: number): number[] {
  const dx = end.x - start.x, dy = end.y - start.y;
  const t = Math.min(1, Math.max(0, ((x + 0.5 - start.x) * dx + (y + 0.5 - start.y) * dy) / (dx * dx + dy * dy)));
  const s = 1 - t;
  return [Math.floor(255 * s + 128 * (1 - s) + 0.5), Math.floor(128 * (1 - s) + 0.5), Math.floor(128 * (1 - s) + 0.5), 255];
}

test("a dragged line previews without recording; Return applies it as the preview showed, as one undo step", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await expect(page.getByTestId("gradient-options")).toBeVisible();
  const before = await store(page);
  await drag(page, [8, 20], [56, 28]);
  const pending = await store(page);
  expect(pending.edit).not.toBeNull();
  expect(pending.depth, "a pending gradient records nothing").toBe(before.depth);
  const { start, end } = pending.edit;
  const shown = [];
  for (const [x, y] of [[4, 4], [30, 24], [40, 13], [60, 44]]) {
    const p = await pixel(page, x, y);
    expect(p, `(${x}, ${y})`).toEqual(expected(start, end, x, y));
    shown.push(p);
  }
  await expect(page.getByTestId("gradient-apply")).toBeVisible();
  await page.keyboard.press("Enter");
  const applied = await store(page);
  expect([applied.edit, applied.depth]).toEqual([null, before.depth + 1]);
  let i = 0;
  for (const [x, y] of [[4, 4], [30, 24], [40, 13], [60, 44]]) expect(await pixel(page, x, y)).toEqual(shown[i++]);
});

test("the ends drag as handles, Shift holds 45 degrees; Escape drops it and the first Undo discards it", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await drag(page, [10, 24], [40, 24]);
  const first = (await store(page)).edit;
  // Grab the end (within 10 view px of it) and move it; the start stays.
  await drag(page, [40.5, 24.5], [50, 10]);
  const moved = (await store(page)).edit;
  expect(moved.start).toEqual(first.start);
  expect(Math.abs(moved.end.x - 50) < 0.3 && Math.abs(moved.end.y - 10) < 0.3).toBe(true);
  // Shift: the end goes onto the nearest eighth of a turn about the start, at the same distance.
  await drag(page, [50, 10], [30, 5], true);
  const snapped = (await store(page)).edit;
  const dx = snapped.end.x - snapped.start.x, dy = snapped.end.y - snapped.start.y;
  expect(Math.abs(Math.abs(dx) - Math.abs(dy)) < 1e-9 || Math.abs(dx) < 1e-9 || Math.abs(dy) < 1e-9, `${dx}, ${dy}`).toBe(true);
  const grey = [128, 128, 128, 255];
  await page.keyboard.press("Escape");
  expect((await store(page)).edit).toBeNull();
  expect(await pixel(page, 30, 24)).toEqual(grey);
  const depth = (await store(page)).depth;
  await drag(page, [10, 24], [40, 24]);
  await page.keyboard.press("Control+z");
  expect((await store(page)).edit).toBeNull();
  expect((await store(page)).depth, "the first Undo only discarded the gradient").toBe(depth);
  expect(await pixel(page, 30, 24)).toEqual(grey);
});

test("Tab switches Radial, the digits set the opacity, and Alt-click samples the foreground", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("g");
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("gradient-radial")).toHaveAttribute("aria-pressed", "true");
  await page.keyboard.press("5");
  await expect(page.getByTestId("gradient-opacity")).toHaveValue("50");
  const at = await client(page, [20, 20]);
  await page.keyboard.down("Alt");
  await page.mouse.click(at.x, at.y);
  await page.keyboard.up("Alt");
  const s = await store(page);
  expect(s.fg).toEqual({ red: 128 / 255, green: 128 / 255, blue: 128 / 255 });
  expect(s.edit, "Alt-click starts no line").toBeNull();
  // A radial red gradient at half opacity: at pixel (32, 24)'s centre, 0.707 px from the start and
  // 20 px to the rim, red at (1 - 0.707 / 20) x 0.5 over grey.
  await page.evaluate(() => (window as any).__compositor.store.getState().setPaletteColor({ red: 1, green: 0, blue: 0 }, false));
  await drag(page, [32, 24], [32, 44]);
  const edit = (await store(page)).edit;
  expect([edit.start, edit.end]).toEqual([{ x: 32, y: 24 }, { x: 32, y: 44 }]);
  await page.keyboard.press("Enter");
  const s2 = (1 - Math.SQRT1_2 / 20) * 0.5;
  expect(await pixel(page, 32, 24)).toEqual([Math.floor(255 * s2 + 128 * (1 - s2) + 0.5), Math.floor(128 * (1 - s2) + 0.5), Math.floor(128 * (1 - s2) + 0.5), 255]);
});

test("with a mask targeted the gradient paints the mask in black and white, as Gradient Mask", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.run({ type: "AddMask", id: s.documents[s.activeId].activeLayerId, revealing: true }); s.setMaskSelected(true); });
  await page.keyboard.press("g");
  const depth = (await store(page)).depth;
  await drag(page, [0, 24], [64, 24]);
  await page.keyboard.press("Enter");
  expect((await store(page)).depth).toBe(depth + 1);
  // Black (hide) at the left fading to nothing: the layer shows more to the right.
  const left = await pixel(page, 2, 24), right = await pixel(page, 60, 24);
  expect(left[3]).toBeLessThan(20);
  expect(right[3]).toBeGreaterThan(230);
});
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -368,3 +368,69 @@ test("eyedropper: a sample and the overlay that shows its ring, at 24 and 100 MP
   console.log(`eyedropper (release wasm, Edge): ${JSON.stringify(out)}`);
   for (const label of ["24 MP", "100 MP"]) expect(out[`${label}: sample and ring, worst ms`]).toBeLessThan(16);
 });
+
+test("gradient tool: drag ticks through the store and the commit through the worker, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      let installedAt = Infinity;
+      const timed = (name: string, after?: () => void) => {
+        const f = api.engine[name].bind(api.engine);
+        api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); after?.(); } };
+      };
+      timed("jobInput");
+      timed("installJob", () => { installedAt = performance.now(); });
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
+      api.store.getState().openDocument(doc);
+      api.store.getState().setTool("gradient");
+      await settle(); frame();
+      const s = () => api.store.getState();
+      s().beginGradient({ x: w * 0.2, y: h * 0.5 });
+      const ticks: number[] = [];
+      for (let i = 0; i < 8; i++) {
+        const t0 = performance.now();
+        s().moveGradient({ end: { x: w * (0.5 + i * 0.04), y: h * 0.6 } }, true);
+        frame();
+        ticks.push(performance.now() - t0);
+      }
+      ticks.shift();
+      result["drag tick (store, engine and frame), worst ms"] = Math.round(Math.max(...ticks));
+      let t0 = performance.now();
+      s().endGradientDrag(); frame();
+      result["release (settled preview and frame) ms"] = Math.round(performance.now() - t0);
+      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
+        let last = performance.now(), gap = 0;
+        const tick = (t: number) => { if (t <= installedAt) gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+        requestAnimationFrame(tick);
+      });
+      t0 = performance.now();
+      s().commitGradient();
+      result["Return (UI thread) ms"] = Math.round(performance.now() - t0);
+      result["commit: longest frame gap while the worker paints"] = await longestGap(() => !s().working);
+      result["commit done after ms"] = Math.round(performance.now() - t0);
+      result["frame after it ms"] = Math.round(frame());
+      result["undo depth"] = api.engine.state(doc).undoDepth;
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`gradient tool (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const label of ["24 MP", "100 MP"]) {
+    expect(out[`${label}: drag tick (store, engine and frame), worst ms`]).toBeLessThan(50);
+    expect(out[`${label}: release (settled preview and frame) ms`]).toBeLessThan(150);
+    expect(out[`${label}: commit: longest frame gap while the worker paints`]).toBeLessThan(100);
+    expect(out[`${label}: jobInput ms`]).toBeLessThan(label === "24 MP" ? 150 : 500);
+    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 500);
+  }
+  // Canvas Size and the Gradient at 24 MP; at 100 MP each entry holds 400 MB, past the history's
+  // 256 MiB, so none is kept (HISTORY_BYTE_LIMIT, as the Mac's DocumentHistory).
+  expect([out["24 MP: undo depth"], out["100 MP: undo depth"]]).toEqual([2, 0]);
+});
```

```diff
--- a/app/tests/unit/fill-actions.test.ts
+++ b/app/tests/unit/fill-actions.test.ts
@@ -24,6 +24,7 @@ function install(l: LayerState = layer(), selection: SelectionState | null = nul
   const engine = {
     state: () => state,
     execute: (_id: string, c: Command) => { log.push(JSON.stringify(c)); return { structure: true, canvas: false, layers: [] }; },
+    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
     jobInput: () => { log.push("job input"); return { input: "{}", pixels: new ArrayBuffer(4), mask: null }; },
     setPreview: () => ({ structure: true, canvas: false, layers: [] }),
   } as unknown as EngineClient;
```

Create `app/tests/unit/gradient-store.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_PALETTE, JOB_PIXELS, useEditor } from "../../src/state/store";
import { DEFAULT_GRADIENT, gradientStops, snapped45 } from "../../src/state/gradient-edit";
import { runAction, typeOpacityDigit } from "../../src/shortcuts/useShortcuts";
import type { Command, DocumentState, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";
import type { JobClient } from "../../src/engine/jobs";

// The Gradient tool's pending edit (Gradient.swift; EditorSession.swift:573-590; GradientTests' rules).
const RED = { red: 1, green: 0, blue: 0 }, BLUE = { red: 0, green: 0, blue: 1 };

describe("the gradient's stops and Shift's steps", () => {
  it("runs from the foreground to the background or to itself transparent, reversed on request", () => {
    expect(gradientStops(DEFAULT_GRADIENT, RED, BLUE)).toEqual([[1, 0, 0, 1], [1, 0, 0, 0]]);
    expect(gradientStops({ ...DEFAULT_GRADIENT, style: "Foreground to Background" }, RED, BLUE)).toEqual([[1, 0, 0, 1], [0, 0, 1, 1]]);
    expect(gradientStops({ ...DEFAULT_GRADIENT, style: "Foreground to Background", reversed: true }, RED, BLUE)).toEqual([[0, 0, 1, 1], [1, 0, 0, 1]]);
  });
  it("holds the moved end to the nearest eighth of a turn at the same distance", () => {
    // (10, 3) from (0, 0) is 16.7 degrees: flat, 10.44 long.
    const flat = snapped45({ x: 10, y: 3 }, { x: 0, y: 0 });
    expect(flat.x).toBeCloseTo(Math.hypot(10, 3), 12);
    expect(flat.y).toBeCloseTo(0, 12);
    // (-7, -9) from (1, 1) is -128 degrees from +x: -135, so down-left at 45 degrees.
    const diagonal = snapped45({ x: -7, y: -9 }, { x: 1, y: 1 });
    const length = Math.hypot(8, 10);
    expect(diagonal.x).toBeCloseTo(1 - length / Math.SQRT2, 12);
    expect(diagonal.y).toBeCloseTo(1 - length / Math.SQRT2, 12);
  });
});

function layer(patch: Partial<LayerState> = {}): LayerState {
  return { id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [40, 30], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 40, pixelsHeight: 30, pixelsRevision: 1, hasPixels: true, hasMask: true, maskWidth: 40, maskHeight: 30,
    maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...patch };
}
function doc(l: LayerState): DocumentState {
  return { id: "D", documentId: "D", width: 40, height: 30, resolution: 72, activeLayerId: l.id, canUndo: true, canRedo: false, isModified: false,
    undoDepth: 3, undoEntryId: 3, path: null, guides: [], undrawn: [], selection: null, layers: [l, { ...l, id: "B", name: "B" }] };
}
let log: string[] = [];
let previews: (PreviewRequest | null)[] = [];
function install(l: LayerState = layer()) {
  log = []; previews = [];
  const state = doc(l);
  const engine = {
    state: () => state,
    execute: (_id: string, c: Command) => { log.push(`execute ${JSON.stringify(c)}`); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, r: PreviewRequest | null) => { previews.push(r); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    redo: () => { log.push("redo"); return { structure: true, canvas: false, layers: [] }; },
    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
    jobInput: () => { log.push("job input"); return { input: "{}", pixels: new ArrayBuffer(4), mask: null }; },
  } as unknown as EngineClient;
  const jobs = { run: () => new Promise(() => {}) } as unknown as JobClient;
  useEditor.setState({ engine, jobs, jobPixels: JOB_PIXELS, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [l.id], maskSelected: false,
    working: false, palette: { ...DEFAULT_PALETTE, foreground: RED, background: BLUE }, gradientOptions: DEFAULT_GRADIENT, gradientEdit: null,
    adjustEdit: null, transformEdit: null, error: null, tool: "gradient", cropRect: null, sheet: null, colorPicker: null });
}
const s = () => useEditor.getState();
const draw = (from: [number, number], to: [number, number]) => {
  s().beginGradient({ x: from[0], y: from[1] });
  s().moveGradient({ end: { x: to[0], y: to[1] } }, true);
  s().endGradientDrag();
};
const last = () => previews.at(-1) as Extract<PreviewRequest, { preview: "Gradient" }>;

describe("a pending gradient", () => {
  beforeEach(() => install());
  it("previews from a reduced copy while dragged and at full quality once let go, and records nothing", () => {
    s().beginGradient({ x: 5, y: 6 });
    s().moveGradient({ end: { x: 30, y: 6 } }, true);
    expect(last()).toEqual({ preview: "Gradient", layer: "A", mask: false, dragging: true,
      gradient: { shape: "Linear", start: [5, 6], end: [30, 6], from: [1, 0, 0, 1], to: [1, 0, 0, 0], opacity: 1 } });
    s().endGradientDrag();
    expect(last().dragging).toBe(false);
    expect(log).toEqual([]);
  });
  it("leaves nothing pending after a click without a line", () => {
    draw([5, 6], [5.3, 6.3]);
    expect(s().gradientEdit).toBeNull();
    expect(previews.at(-1)).toBeNull();
  });
  it("is applied by Return as one Gradient, and dropped by Escape", () => {
    draw([5, 6], [30, 6]);
    runAction("apply");
    expect(log).toEqual([`execute ${JSON.stringify({ type: "Gradient", id: "A", mask: false, gradient: last().gradient })}`]);
    expect(s().gradientEdit).toBeNull();
    draw([1, 1], [9, 9]);
    runAction("cancel");
    expect(s().gradientEdit).toBeNull();
    expect(log.length).toBe(1);
    expect(previews.at(-1)).toBeNull();
  });
  it("is discarded by the first Undo, and the next Undo undoes", () => {
    draw([5, 6], [30, 6]);
    s().undo();
    expect([s().gradientEdit, log]).toEqual([null, []]);
    s().undo();
    expect(log).toEqual(["undo"]);
  });
  it("is applied before a change of tool, of layer, of target, or any other command", () => {
    const paint = () => log.filter((l) => l.includes('"Gradient"')).length;
    draw([5, 6], [30, 6]); s().setTool("move"); expect(paint()).toBe(1);
    s().setTool("gradient");
    draw([5, 6], [30, 6]); s().selectLayers(["B"], "B"); expect(paint()).toBe(2);
    install();
    draw([5, 6], [30, 6]); s().setMaskSelected(true); expect(paint()).toBe(1);
    install();
    draw([5, 6], [30, 6]); s().run({ type: "AddBlankLayer" });
    expect(log.map((l) => JSON.parse(l.slice("execute ".length)).type)).toEqual(["Gradient", "AddBlankLayer"]);
  });
  it("is drawn again when its settings or the palette change, in the new colours", () => {
    draw([5, 6], [30, 6]);
    s().setGradientOptions({ shape: "Radial", style: "Foreground to Background", opacity: 0.4 });
    expect(last().gradient).toMatchObject({ shape: "Radial", from: [1, 0, 0, 1], to: [0, 0, 1, 1], opacity: 0.4 });
    expect(last().dragging).toBe(false);
    s().swapPalette();
    expect(last().gradient).toMatchObject({ from: [0, 0, 1, 1], to: [1, 0, 0, 1] });
  });
  it("paints a targeted mask in black and white", () => {
    useEditor.setState({ maskSelected: true });
    draw([5, 6], [30, 6]);
    expect(last()).toMatchObject({ mask: true, gradient: { from: [0, 0, 0, 1], to: [0, 0, 0, 0] } });
  });
  it("does not start where nothing can be painted, and goes to the job worker on a large layer", () => {
    install(layer({ visible: false }));
    expect(s().beginGradient({ x: 1, y: 1 })).toBe(false);
    install();
    useEditor.setState({ jobPixels: 40 * 30 - 1 });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual(["job input"]);
  });
  it("chooses the job worker by the stored layer's size, not the reduced preview the state shows", () => {
    // The state reports a previewed layer at its preview's size: 10 x 7.5 of the stored 40 x 30.
    const shown = { ...s().documents.D, layers: s().documents.D.layers.map((l) => ({ ...l, pixelsWidth: 10, pixelsHeight: 8 })) };
    useEditor.setState({ jobPixels: 100, engine: { ...s().engine!, state: () => shown, storedPixels: () => 40 * 30 } as never });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual(["job input"]);
  });
  it("takes the digit keys as its opacity, at least 1 %, and Tab as its shape", () => {
    typeOpacityDigit(4, 1000, (v) => s().setGradientOptions({ opacity: Math.max(0.01, v) }));
    expect(s().gradientOptions.opacity).toBe(0.4);
    runAction("opacity-0");
    runAction("opacity-0");
    expect(s().gradientOptions.opacity).toBe(0.01);
    runAction("cycle-tool-mode");
    expect(s().gradientOptions.shape).toBe("Radial");
  });
});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -25,6 +25,7 @@ describe("keymap", () => {
     expect(matchShortcut(ev("x"))).toBe("swap-colors");
     expect(matchShortcut(ev("d"))).toBe("default-colors");
     expect(matchShortcut(ev("i"))).toBe("tool-eyedropper");
+    expect(matchShortcut(ev("g"))).toBe("tool-gradient");
     expect(matchShortcut(ev("Backspace", { altKey: true }))).toBe("fill-foreground");
     expect(matchShortcut(ev("Delete", { ctrlKey: true }))).toBe("fill-background");
     expect(matchShortcut(ev("Backspace"))).toBe("delete-layer");
```

```diff
--- a/app/tests/unit/store-jobs.test.ts
+++ b/app/tests/unit/store-jobs.test.ts
@@ -29,6 +29,7 @@ function install(width: number, height: number, onInstall?: () => void) {
     execute: (_id: string, cmd: Command) => { log.push(`execute ${cmd.type}`); return { structure: true, canvas: false, layers: [] }; },
     setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
     histogram: () => { log.push("histogram here"); return bins(); },
+    storedPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
     jobInput: () => { log.push("job input"); return { input: '{"stamp":{"pixelsRevision":1}}', pixels: new ArrayBuffer(4), mask: null }; },
     installJob: (_doc: string, layerId: string, input: string, output: string) => { log.push(`install ${layerId} ${output}`); onInstall?.(); expect(input).toContain("stamp"); return { structure: true, canvas: false, layers: [] }; },
     undo: () => { log.push("undo"); return { structure: true, canvas: false, layers: [] }; },
```

```diff
--- a/app/tests/unit/tool-rail.test.tsx
+++ b/app/tests/unit/tool-rail.test.tsx
@@ -31,7 +31,7 @@ describe("ToolRail", () => {
   it("shows an icon, not a letter, on every tool, named for assistive technology", () => {
     mount();
     const expected: Record<string, string> = { move: "Move", marquee: "Marquee", lasso: "Lasso", wand: "Magic Wand",
-      crop: "Crop", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
+      crop: "Crop", gradient: "Gradient", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
     for (const [id, label] of Object.entries(expected)) {
       const b = button(id);
       expect(b.querySelectorAll("svg").length, id).toBe(1);
```

```diff
--- a/engine/tests/gradient_preview.rs
+++ b/engine/tests/gradient_preview.rs
@@ -40,6 +40,8 @@ fn a_dragged_gradient_previews_from_a_reduced_copy_of_the_layer_grown_to_the_can
     let state = e.state(id).unwrap().layers[0].clone();
     // Halved until the longer side is at most 1024: 3000 -> 1500 -> 750.
     assert_eq!((state.pixels_width, state.pixels_height), (750, 500));
+    // The state shows the reduced copy; the stored layer (which decides the job worker) is unchanged.
+    assert_eq!(e.stored_pixels(id, layer).unwrap(), 1000 * 800);
     assert!(state.transform.origin.x <= 0.0 && state.transform.origin.y <= 0.0 && state.transform.origin.x + state.transform.size.width >= 3000.0, "{:?}", state.transform);
     // Left of the layer, where there were no pixels: the gradient alone (a quarter of the way: alpha 191).
     let left = shown(&e, id, 375, 1000);
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `gradient-store.test.ts` does not compile (`gradient-edit.ts`, `beginGradient`). `cargo test -p compositor-engine --test gradient_preview`: `stored_pixels` does not exist.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -19,6 +19,7 @@ import { ImageSizeSheet } from "./sheets/ImageSizeSheet";
 import { JpegExportSheet } from "./sheets/JpegExportSheet";
 import { CropOptions } from "./panels/CropOptions";
 import { SelectionOptions } from "./panels/SelectionOptions";
+import { GradientOptions } from "./panels/GradientOptions";
 import { SelectionAmountSheet } from "./sheets/SelectionAmountSheet";
 import { TransformInspector } from "./panels/TransformInspector";
 import { AdjustPanel } from "./panels/AdjustPanel";
@@ -93,6 +94,7 @@ export function App() {
       <ProjectTabs />
       <CropOptions />
       <SelectionOptions />
+      <GradientOptions />
       <TransformInspector />
       <div className="workspace">
         <ToolRail />
```

```diff
--- a/app/src/actions/layers.ts
+++ b/app/src/actions/layers.ts
@@ -1,5 +1,5 @@
 import { colorTuple } from "../tools/color";
-import { useEditor } from "../state/store";
+import { canPaintNow, useEditor } from "../state/store";
 import { activeLayer, visibleIds } from "../state/selection";
 import type { AdjustmentKind, BlendMode, SelectionMode } from "../engine/types";
 import { isEditableKind } from "../engine/types";
@@ -60,17 +60,11 @@ export function deleteKeyPressed(): void {
   c.s.run({ type: "ClearSelectedPixels", id: c.active!.id, mask: false });
 }
 
-/** Whether the active layer's pixels, or its mask, can take a fill now (`canPaint`,
- * EditorSession+Brush.swift:5-11): one layer selected and shown, not a folder unless its mask is the
- * target, an enabled mask when it is, not an adjustment layer, no empty selection, no panel open, no
- * job's result to come, no crop rectangle pending. */
+/** Whether a fill can paint now: the store's `canPaintNow`, and no crop rectangle pending (the Mac's
+ * `canEditLayers` wants `cropRect == nil`). */
 export function canPaint(): boolean {
-  const c = ctx(); if (!c?.active || c.selected.length !== 1 || c.s.panelOwnsDocument() || c.s.working) return false;
-  if (c.s.tool === "crop" && c.s.cropRect) return false;
-  if (c.doc.selection?.empty || !visibleIds(c.doc).has(c.active.id)) return false;
-  const mask = c.s.maskTargeted();
-  if (mask) return c.active.maskEnabled;
-  return !c.active.isGroup && !c.active.adjustment;
+  const s = useEditor.getState();
+  return !(s.tool === "crop" && s.cropRect) && canPaintNow();
 }
 
 /** Alt+Backspace / Ctrl+Backspace: the selection (or the whole layer) filled with the foreground or
```

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -11,6 +11,8 @@ import { activeLayer, canTransform, editedShape, transformsAsGroup } from "../st
 import { antsDelay, AntsPathCache, ANTS_INTERVAL_MS, nextPhase, OutlineCache, outlineStep } from "./ants";
 import { isSelectionTool, outlineOffset, SelectionDraft, selectionMode, type P as DocP } from "../tools/selection-draft";
 import { installSampling } from "./sampling";
+import { gradientLine, installGradientTool } from "./gradient-tool";
+import { gradientStops } from "../state/gradient-edit";
 
 export const HIT_HANDLE_PX = 6;
 
@@ -70,10 +72,13 @@ export function CanvasView() {
       ants = { path, at, phase: antsPhaseRef.current };
     }
     const d = s.selectionDraft;
+    const line = gradientLine();
+    const [from, to] = gradientStops(s.gradientOptions, s.palette.foreground, s.palette.background);
     drawOverlay(overlay.getContext("2d")!, vp, dpr, {
       docWidth: doc.width, docHeight: doc.height, cropRect: s.tool === "crop" ? s.cropRect : null, guides: s.snapGuides,
       transform: transformGeometry, canvasGuides: s.showGuides ? doc.guides : null,
       ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null, sampleRing: s.sampleRing,
+      gradientLine: line ? { ...line, radial: s.gradientOptions.shape === "Radial", from, to } : null,
     });
   };
 
@@ -198,6 +203,12 @@ export function CanvasView() {
     return installSampling(el, () => spaceRef.current);
   }, []);
 
+  // The Gradient tool's line (canvas/gradient-tool.ts).
+  useEffect(() => {
+    const el = glRef.current?.parentElement; if (!el) return;
+    return installGradientTool(el, () => spaceRef.current);
+  }, []);
+
   // Adjustment eyedroppers: while a sample mode is armed (Levels' three, or Hue/Saturation's
   // replace/add/remove), a click reads the point under the cursor and feeds the open panel
   // instead of starting whatever gesture the active tool would otherwise begin. Registered with
@@ -224,7 +235,7 @@ export function CanvasView() {
   const picking = useEditor((s) => !!s.colorPicker);
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
-    el.style.cursor = sampleMode || picking || tool === "eyedropper" || isSelectionTool(tool) ? "crosshair" : "";
+    el.style.cursor = sampleMode || picking || tool === "eyedropper" || tool === "gradient" || isSelectionTool(tool) ? "crosshair" : "";
   }, [sampleMode, tool, picking]);
 
   // Drag to pan with the hand tool or the space bar.
```

Create `app/src/canvas/gradient-tool.ts`:

```ts
import { useEditor } from "../state/store";
import { GRADIENT_HANDLE_PX, hasLine, snapped45 } from "../state/gradient-edit";

type P = { x: number; y: number };

/** The pending line's ends in view px, when it has a line (`gradientLine`, TransformOverlay.swift:83-88). */
export function gradientLine(): { start: P; end: P } | null {
  const s = useEditor.getState(); const e = s.gradientEdit;
  if (!e || !hasLine(e) || !s.activeId) return null;
  const vp = s.viewports[s.activeId], d = s.documents[s.activeId], size = { width: d.width, height: d.height };
  return { start: vp.viewPoint(e.start, size), end: vp.viewPoint(e.end, size) };
}

/** The Gradient tool's pointer (EditorCanvas.swift:2023-2031, :1539-1546, :1680-1683): a press within
 * 10 view px of an end of the pending line grabs it (the end first), otherwise starts a new line and
 * drags its end; Shift holds the dragged end to 45 degree steps about the other. Each move previews
 * from a reduced copy, at most once a frame; the release previews at full quality. Returns the cleanup. */
export function installGradientTool(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let handle: "start" | "end" | null = null;
  let pending: PointerEvent | null = null;
  let frame = 0;
  const view = (e: PointerEvent): P => { const r = el.getBoundingClientRect(); return { x: e.clientX - r.left, y: e.clientY - r.top }; };
  const docPoint = (at: P): P => {
    const s = useEditor.getState(); const d = s.documents[s.activeId!];
    return s.viewports[s.activeId!].documentPoint(at, { width: d.width, height: d.height });
  };
  const apply = (e: PointerEvent) => {
    const s = useEditor.getState(); const edit = s.gradientEdit;
    if (!handle || !edit || !s.activeId) return;
    let pixel = docPoint(view(e));
    if (e.shiftKey) pixel = snapped45(pixel, handle === "start" ? edit.end : edit.start);
    s.moveGradient(handle === "start" ? { start: pixel } : { end: pixel }, true);
  };
  const down = (e: PointerEvent) => {
    const s = useEditor.getState();
    if (s.tool !== "gradient" || e.button !== 0 || spaceHeld() || !s.activeId) return;
    const at = view(e);
    const line = gradientLine();
    if (line && Math.hypot(at.x - line.end.x, at.y - line.end.y) <= GRADIENT_HANDLE_PX) handle = "end";
    else if (line && Math.hypot(at.x - line.start.x, at.y - line.start.y) <= GRADIENT_HANDLE_PX) handle = "start";
    else handle = s.beginGradient(docPoint(at)) ? "end" : null;
    if (handle) el.setPointerCapture(e.pointerId);
    s.repaintOverlay();
  };
  const move = (e: PointerEvent) => {
    if (!handle) return;
    pending = e;
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; if (pending) apply(pending); pending = null; });
  };
  const up = (e: PointerEvent) => {
    if (!handle) return;
    if (frame) { cancelAnimationFrame(frame); frame = 0; pending = null; }
    apply(e);
    handle = null;
    useEditor.getState().endGradientDrag();
  };
  const cancel = () => { if (frame) { cancelAnimationFrame(frame); frame = 0; } pending = null; if (handle) { handle = null; useEditor.getState().endGradientDrag(); } };
  el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", cancel);
  return () => {
    cancel();
    el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", cancel);
  };
}
```

```diff
--- a/app/src/canvas/overlay.ts
+++ b/app/src/canvas/overlay.ts
@@ -22,10 +22,38 @@ export interface OverlayState {
   /** Null with no selection, or an empty one. */
   ants?: AntsState | null;
   draft?: DraftState | null;
+  /** The pending gradient's line in view px, its shape and its two stops as straight RGBA. */
+  gradientLine?: { start: { x: number; y: number }; end: { x: number; y: number }; radial: boolean; from: number[]; to: number[] } | null;
   /** While the canvas is sampled: the ring about the pointer (view px). */
   sampleRing?: { at: { x: number; y: number }; sampled: PaletteColor; original: PaletteColor } | null;
 }
 
+/** The pending gradient (`drawGradientLine`, TransformOverlay.swift): a faint dashed rim where a radial
+ * one reaches its end, the line black 3 px under white 1 px, and at each end a 12 px white disc with a
+ * grey centre the stop's colour covers. */
+function drawGradientLine(ctx: CanvasRenderingContext2D, line: NonNullable<OverlayState["gradientLine"]>): void {
+  ctx.save();
+  if (line.radial) {
+    const r = Math.hypot(line.end.x - line.start.x, line.end.y - line.start.y);
+    ctx.setLineDash([4, 4]);
+    ctx.beginPath(); ctx.arc(line.start.x, line.start.y, r, 0, Math.PI * 2);
+    ctx.strokeStyle = "rgba(0,0,0,0.5)"; ctx.lineWidth = 2; ctx.stroke();
+    ctx.strokeStyle = "rgba(255,255,255,0.8)"; ctx.lineWidth = 1; ctx.stroke();
+    ctx.setLineDash([]);
+  }
+  ctx.beginPath(); ctx.moveTo(line.start.x, line.start.y); ctx.lineTo(line.end.x, line.end.y);
+  ctx.strokeStyle = "rgba(0,0,0,0.7)"; ctx.lineWidth = 3; ctx.stroke();
+  ctx.strokeStyle = "white"; ctx.lineWidth = 1; ctx.stroke();
+  for (const [p, c] of [[line.start, line.from], [line.end, line.to]] as const) {
+    ctx.beginPath(); ctx.arc(p.x, p.y, 6, 0, Math.PI * 2);
+    ctx.fillStyle = "white"; ctx.fill(); ctx.strokeStyle = "black"; ctx.lineWidth = 1; ctx.stroke();
+    ctx.beginPath(); ctx.arc(p.x, p.y, 3.5, 0, Math.PI * 2);
+    ctx.fillStyle = "rgb(191,191,191)"; ctx.fill();
+    ctx.fillStyle = `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(c[2] * 255)}, ${c[3]})`; ctx.fill();
+  }
+  ctx.restore();
+}
+
 /** The side of the sample ring's box, in view px (`SampleRingOverlay`: a 116 pt frame). */
 export const SAMPLE_RING = 116;
 
@@ -164,6 +192,7 @@ export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, d
   }
   if (state.ants) drawAnts(ctx, state.ants);
   if (state.draft) drawDraft(ctx, viewport, size, state.draft);
+  if (state.gradientLine) drawGradientLine(ctx, state.gradientLine);
   if (state.sampleRing) drawSampleRing(ctx, state.sampleRing);
   ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
   for (const x of state.guides.xs) { const v = viewport.viewPoint({ x, y: 0 }, size).x; ctx.beginPath(); ctx.moveTo(v + 0.5, 0); ctx.lineTo(v + 0.5, viewport.viewSize.height); ctx.stroke(); }
```

```diff
--- a/app/src/canvas/sampling.ts
+++ b/app/src/canvas/sampling.ts
@@ -4,10 +4,12 @@ import type { PaletteColor } from "../tools/color";
 /** Where a press on the canvas samples colour (EditorCanvas.swift:1419-1424): into the open colour
  * picker, whatever the tool; else into the foreground with the Eyedropper, unless an adjustment's
  * own eyedropper is armed (it answers the press itself); else null. */
-export function samplingInto(): "picker" | "foreground" | null {
+export function samplingInto(alt = false): "picker" | "foreground" | null {
   const s = useEditor.getState();
   if (s.colorPicker) return "picker";
-  if (s.tool === "eyedropper" && !s.adjustEdit?.sampleMode && s.activeId) return "foreground";
+  if (s.adjustEdit?.sampleMode || !s.activeId) return null;
+  // Alt with the Gradient tool stands in for the Eyedropper (`palettePicking`, EditorCanvas.swift:71).
+  if (s.tool === "eyedropper" || (s.tool === "gradient" && alt)) return "foreground";
   return null;
 }
 
@@ -37,7 +39,7 @@ export function installSampling(el: HTMLElement, spaceHeld: () => boolean): () =
     if (sampled) s.setSampleRing({ at, sampled, original });
   };
   const down = (e: PointerEvent) => {
-    into = e.button === 0 && !spaceHeld() ? samplingInto() : null;
+    into = e.button === 0 && !spaceHeld() ? samplingInto(e.altKey) : null;
     if (!into) return;
     e.stopImmediatePropagation();
     original = current();
```

```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -97,6 +97,9 @@ export class EngineClient {
    * into a local before `this.memory.buffer` is read: argument evaluation is left to right, so
    * reading the buffer first would capture it before `mask_pixels_ptr` marshals its two string
    * arguments through `__wbindgen_malloc`, which can grow memory and detach that buffer. */
+  /** The layer's stored pixel count (`Engine::stored_pixels`): `state()` shows a previewed layer at the
+   * preview's size, which may be a reduced copy. */
+  storedPixels(doc: string, layer: string): number { return this.wasm.stored_pixels(doc, layer); }
   maskPixels(doc: string, layer: string): Uint8Array | null {
     const len = this.wasm.mask_pixels_len(doc, layer);
     if (len === 0) return null;
```

Create `app/src/panels/GradientOptions.tsx`:

```tsx
import { useEditor } from "../state/store";
import { GRADIENT_STYLES, gradientStops, type GradientStyle } from "../state/gradient-edit";
import { NumberInput } from "./NumberInput";

/** The Gradient tool's bar (GradientControls.swift): Linear or Radial, the colours over a checkerboard,
 * the style, Reverse, Opacity (1-100 %, also the digit keys), "Mask" while a mask is the target, and
 * Cancel / Apply while a gradient is pending. Every control gives up the focus once used, as the
 * selection bar's do, so Return, Escape and the digits reach the canvas. */
export function GradientOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "gradient" || !doc || s.sheet !== null) return null;
  const o = s.gradientOptions;
  const [from, to] = gradientStops(o, s.palette.foreground, s.palette.background);
  const css = (c: number[]) => `rgba(${Math.round(c[0] * 255)}, ${Math.round(c[1] * 255)}, ${Math.round(c[2] * 255)}, ${c[3]})`;
  const disabled = s.working;
  return (
    <div className="tool-options" data-testid="gradient-options">
      <strong>Gradient</strong>
      <span className="segmented" title="Linear runs along the line; Radial spreads out from the start point">
        {(["Linear", "Radial"] as const).map((k) => (
          <button key={k} data-testid={`gradient-${k.toLowerCase()}`} aria-pressed={o.shape === k} disabled={disabled}
            onClick={(e) => { s.setGradientOptions({ shape: k }); e.currentTarget.blur(); }}>{k}</button>
        ))}
      </span>
      <span className="gradient-swatch" data-testid="gradient-swatch" aria-hidden="true" style={{ backgroundImage: `linear-gradient(to right, ${css(from)}, ${css(to)}), repeating-conic-gradient(rgba(128,128,128,0.45) 0% 25%, #fff 0% 50%)`, backgroundSize: "auto, 8px 8px" }} />
      <select aria-label="Colors" data-testid="gradient-style" value={o.style} disabled={disabled}
        onChange={(e) => { s.setGradientOptions({ style: e.target.value as GradientStyle }); e.currentTarget.blur(); }}>
        {GRADIENT_STYLES.map((style) => <option key={style}>{style}</option>)}
      </select>
      <label><input type="checkbox" data-testid="gradient-reverse" checked={o.reversed} disabled={disabled}
        onChange={(e) => { s.setGradientOptions({ reversed: e.target.checked }); e.currentTarget.blur(); }} /> Reverse</label>
      <label title="Press 1-9 for 10-90%, 0 for 100%">Opacity{" "}
        <input type="range" aria-hidden tabIndex={-1} min={1} max={100} step={1} value={Math.round(o.opacity * 100)} disabled={disabled}
          onChange={(e) => s.setGradientOptions({ opacity: Number(e.target.value) / 100 })} onPointerUp={(e) => e.currentTarget.blur()} />
        <NumberInput label="Opacity" testId="gradient-opacity" value={Math.round(o.opacity * 100)} min={1} max={100} step={1} blurOnEnter disabled={disabled}
          onChange={(v) => s.setGradientOptions({ opacity: Math.round(v) / 100 })} />%
      </label>
      <span style={{ flex: 1 }} />
      {s.maskTargeted() && <span className="muted">Mask</span>}
      {s.gradientEdit && (
        <>
          <button data-testid="gradient-cancel" onClick={(e) => { e.currentTarget.blur(); s.cancelGradient(); }}>Cancel</button>
          <button data-testid="gradient-apply" className="primary" onClick={(e) => { e.currentTarget.blur(); s.commitGradient(); }}>Apply</button>
        </>
      )}
    </div>
  );
}
```

```diff
--- a/app/src/panels/MenuBar.tsx
+++ b/app/src/panels/MenuBar.tsx
@@ -49,7 +49,7 @@ export function MenuBar() {
       // A pending transform owns the gesture: macOS's `canUseHistory` requires
       // `transformEdit == nil`, so both items grey out until it commits or cancels. An open panel
       // makes both inert as well (store.undo/redo), so they grey out for it too.
-      { id: "undo", label: "Undo", run: () => runAction("undo"), enabled: editable && !s.transformEdit && !!activeDoc?.canUndo },
+      { id: "undo", label: "Undo", run: () => runAction("undo"), enabled: editable && !s.transformEdit && (!!activeDoc?.canUndo || !!s.gradientEdit) },
       { id: "redo", label: "Redo", run: () => runAction("redo"), enabled: editable && !s.transformEdit && !!activeDoc?.canRedo },
       "separator",
       { id: "fill-foreground", label: "Fill with Foreground Color", run: () => runAction("fill-foreground"), enabled: editable && canPaint() },
```

```diff
--- a/app/src/panels/ToolRail.tsx
+++ b/app/src/panels/ToolRail.tsx
@@ -5,7 +5,7 @@ import { PaletteSwatches } from "./PaletteSwatches";
 const TOOLS: { id: Tool; label: string; key: string }[] = [
   { id: "move", label: "Move", key: "V" }, { id: "marquee", label: "Marquee", key: "M" },
   { id: "lasso", label: "Lasso", key: "L" }, { id: "wand", label: "Magic Wand", key: "W" },
-  { id: "crop", label: "Crop", key: "C" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
+  { id: "crop", label: "Crop", key: "C" }, { id: "gradient", label: "Gradient", key: "G" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
   { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" },
 ];
 export function ToolRail() {
```

```diff
--- a/app/src/panels/tool-icons.tsx
+++ b/app/src/panels/tool-icons.tsx
@@ -19,7 +19,7 @@ import type { LassoKind, MarqueeKind } from "../tools/selection-draft";
 /** Which icon a tool shows: the Marquee and the Lasso follow their mode, as on the Mac
  *  (ContentView.swift: circle.dashed in Ellipse mode, its own icon for the polygonal lasso). */
 export type ToolIconName = "move" | "marquee-rectangle" | "marquee-ellipse" | "lasso-freehand" | "lasso-polygonal"
-  | "wand" | "crop" | "eyedropper" | "hand" | "zoom";
+  | "wand" | "crop" | "gradient" | "eyedropper" | "hand" | "zoom";
 
 export function toolIconName(tool: Tool, marquee: MarqueeKind, lasso: LassoKind): ToolIconName {
   switch (tool) {
@@ -56,6 +56,9 @@ const SHAPES: Record<ToolIconName, ReactNode> = {
   </>,
   // lucide crop (SF crop)
   "crop": <><path d="M6 2v14a2 2 0 0 0 2 2h14" /><path d="M18 22V8a2 2 0 0 0-2-2H2" /></>,
+  // Drawn for this port in Lucide's style after SF square.bottomhalf.filled (the Mac's symbol for the
+  // tool): a rounded square with its lower half filled.
+  "gradient": <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M3 12h18v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="currentColor" /></>,
   // lucide pipette (SF eyedropper)
   "eyedropper": <>
     <path d="m12 9-8.414 8.414A2 2 0 0 0 3 18.828v1.344a2 2 0 0 1-.586 1.414A2 2 0 0 1 3.828 21h1.344a2 2 0 0 0 1.414-.586L15 12" />
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -5,7 +5,7 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
   | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
-  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background";
+  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background" | "tool-gradient";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -39,7 +39,7 @@ export const SHORTCUTS = {
   "cycle-tool-mode": [{ key: "Tab" }],
   // The palette (EditorCanvas.swift:1835-1836).
   "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
-  "tool-eyedropper": [{ key: "i" }],
+  "tool-eyedropper": [{ key: "i" }], "tool-gradient": [{ key: "g" }],
   // Photoshop's fill keys (CompositorApp.swift:175-186: Option-Delete and Command-Delete on the Mac).
   "fill-foreground": [{ key: "Backspace", alt: true }, { key: "Delete", alt: true }],
   "fill-background": [{ key: "Backspace", ctrl: true }, { key: "Delete", ctrl: true }],
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -79,6 +79,7 @@ export function runAction(id: ActionId, shift = false): void {
     case "tool-lasso": s.setTool("lasso"); break;
     case "tool-wand": s.setTool("wand"); break;
     case "tool-eyedropper": s.setTool("eyedropper"); break;
+    case "tool-gradient": s.setTool("gradient"); break;
     case "fill-foreground": if (doc && !s.sheet) fillActive(false); break;
     case "fill-background": if (doc && !s.sheet) fillActive(true); break;
     case "select-all": if (doc) s.run({ type: "SelectAll" }); break;
@@ -92,12 +93,14 @@ export function runAction(id: ActionId, shift = false): void {
     case "apply":
       if (s.panelOwnsDocument()) break;
       if (s.selectionDraft) s.finishSelectionDraft();
+      else if (s.gradientEdit) s.commitGradient();
       else if (doc && s.tool === "crop") { const r = s.cropRect; if (r) { s.run({ type: "Crop", ...r }); s.setCropRect(null); } }
       else if (s.transformEdit) s.commitTransform();
       break;
     case "cancel":
       if (s.panelOwnsDocument()) break;
       if (s.selectionDraft) s.setSelectionDraft(null);
+      else if (s.gradientEdit) s.cancelGradient();
       else if (s.tool === "crop") s.setCropRect(null);
       else if (s.transformEdit) s.cancelTransform();
       break;
@@ -122,7 +125,11 @@ export function runAction(id: ActionId, shift = false): void {
     case "hue-saturation": s.beginAdjust({ kind: "Hue/Saturation" }); break;
     case "invert": invertActive(); break;
     default:
-      if (id.startsWith("opacity-")) { if (s.tool === "move" && doc) typeOpacityDigit(Number(id.slice(8))); }
+      if (id.startsWith("opacity-")) {
+        if (s.tool === "move" && doc) typeOpacityDigit(Number(id.slice(8)));
+        // With the Gradient tool the digits set its opacity, at least 1 % (`typeOpacityDigit`, EditorSession+Brush.swift:193-211).
+        else if (s.tool === "gradient" && !s.working) typeOpacityDigit(Number(id.slice(8)), Date.now(), (v) => s.setGradientOptions({ opacity: Math.max(0.01, v) }));
+      }
   }
 }
```

Create `app/src/state/gradient-edit.ts`:

```ts
import type { GradientSpec } from "../engine/types";
import type { PaletteColor } from "../tools/color";

/** The Gradient tool's settings (`GradientSettings`, Gradient.swift:14-19), kept for the app session. */
export type GradientStyle = "Foreground to Background" | "Foreground to Transparent";
export interface GradientOptions { shape: "Linear" | "Radial"; style: GradientStyle; reversed: boolean; opacity: number; }
export const DEFAULT_GRADIENT: GradientOptions = { shape: "Linear", style: "Foreground to Transparent", reversed: false, opacity: 1 };
export const GRADIENT_STYLES: GradientStyle[] = ["Foreground to Background", "Foreground to Transparent"];

/** A gradient not yet applied (`GradientEdit`): the layer and whether its mask is the target, and the
 * line's two ends in document pixels. Previewed by the engine; nothing is in the document until it is
 * applied. */
export interface GradientEdit { layerId: string; mask: boolean; start: { x: number; y: number }; end: { x: number; y: number }; }

/** The shortest line that paints (`hasLine`: at least half a pixel; engine MIN_GRADIENT_LINE). */
export const hasLine = (e: GradientEdit): boolean => Math.hypot(e.end.x - e.start.x, e.end.y - e.start.y) >= 0.5;

/** How near, in view px, a press must land to grab an end of the pending line (EditorCanvas.swift:2026). */
export const GRADIENT_HANDLE_PX = 10;

/** The two stops (`gradientColors`, Gradient.swift:72-82): the foreground to the background, or to
 * the foreground made transparent; reversed if asked. On a mask the palette is already black or white. */
export function gradientStops(options: GradientOptions, foreground: PaletteColor, background: PaletteColor): [[number, number, number, number], [number, number, number, number]] {
  const rgba = (c: PaletteColor, a: number): [number, number, number, number] => [c.red, c.green, c.blue, a];
  const stops: [[number, number, number, number], [number, number, number, number]] = options.style === "Foreground to Background"
    ? [rgba(foreground, 1), rgba(background, 1)] : [rgba(foreground, 1), rgba(foreground, 0)];
  return options.reversed ? [stops[1], stops[0]] : stops;
}

/** The engine's `GradientSpec` for the pending line. */
export function gradientSpec(edit: GradientEdit, options: GradientOptions, foreground: PaletteColor, background: PaletteColor): GradientSpec {
  const [from, to] = gradientStops(options, foreground, background);
  return { shape: options.shape, start: [edit.start.x, edit.start.y], end: [edit.end.x, edit.end.y], from, to, opacity: options.opacity };
}

/** `point` moved onto the nearest eighth of a turn about `anchor`, at the same distance: Shift's 45
 * degree steps (`snapped`, EditorCanvas.swift:2034-2039; the Shape tool's line does the same). */
export function snapped45(point: { x: number; y: number }, anchor: { x: number; y: number }): { x: number; y: number } {
  const dx = point.x - anchor.x, dy = point.y - anchor.y;
  const length = Math.hypot(dx, dy);
  // Swift's `rounded()` takes halves away from zero; Math.round takes them up.
  const steps = Math.atan2(dy, dx) / (Math.PI / 4);
  const angle = Math.sign(steps) * Math.round(Math.abs(steps)) * (Math.PI / 4);
  return { x: anchor.x + Math.cos(angle) * length, y: anchor.y + Math.sin(angle) * length };
}
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -13,9 +13,10 @@ import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } fr
 import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
+import { DEFAULT_GRADIENT, gradientSpec, hasLine, type GradientEdit, type GradientOptions } from "./gradient-edit";
 import { BLACK, WHITE, hsbOf, hsbToRgb, quantized, sameColor, withRgb, type PaletteColor, type PickerHSB } from "../tools/color";
 
-export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper";
+export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper" | "gradient";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
 /** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
 export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
@@ -156,6 +157,26 @@ export interface EditorStore {
    * while a mask is the target (`sampleColor`, EditorCanvas.swift:2045-2048). Nothing off the canvas,
    * over a transparent pixel, or while a job's result is to come. */
   sampleForeground(at: { x: number; y: number }): void;
+  gradientOptions: GradientOptions;
+  /** The gradient drawn but not yet applied (Gradient.swift): the engine previews it. While it is
+   * pending any other command applies it first (as a pending transform is committed), the first Undo
+   * discards it, and a change of tool, layer or target applies it (`resolveGradient`). */
+  gradientEdit: GradientEdit | null;
+  /** New settings; a pending gradient is drawn again with them (`gradientSettings` didSet). */
+  setGradientOptions(patch: Partial<GradientOptions>): void;
+  /** A press with the Gradient tool: a new line from `at` on the active layer or its mask (a pending
+   * one on the same target starts again there). False when nothing can be painted (`beginGradient`). */
+  beginGradient(at: { x: number; y: number }): boolean;
+  /** Moves an end of the pending line and previews it, from a reduced copy while `dragging`. */
+  moveGradient(ends: { start?: { x: number; y: number }; end?: { x: number; y: number } }, dragging: boolean): void;
+  /** The drag is over: a click without a line leaves nothing pending, else the full preview (`endGradientDrag`). */
+  endGradientDrag(): void;
+  /** Re-previews the pending gradient at full quality (settings or palette changed). */
+  refreshGradient(): void;
+  cancelGradient(): void;
+  /** Return or Apply: paints the pending gradient as one undo step ("Gradient" or "Gradient Mask"),
+   * through the job worker on a large layer (`commitGradient`). */
+  commitGradient(): void;
   setSampleRing(ring: SampleRing | null): void;
   setEngine(engine: EngineClient): void;
   setJobs(jobs: JobClient): void;
@@ -270,6 +291,30 @@ function setGradientMapEnd(highlights: boolean, color: PaletteColor): void {
   if (current.red === end.red && current.green === end.green && current.blue === end.blue) return;
   useEditor.getState().updateAdjust({ adjustment: { ...edit.adjustment, gradientMapSettings: { ...settings, [highlights ? "highlights" : "shadows"]: end } } });
 }
+/** Whether the active layer's pixels, or its mask, can take paint now (`canPaint`,
+ * EditorSession+Brush.swift:5-11): one layer selected and shown, not a folder unless its mask is the
+ * target, an enabled mask when it is, not an adjustment layer, no empty selection, no panel open, no
+ * job's result to come. */
+export function canPaintNow(): boolean {
+  const s = useEditor.getState();
+  const doc = s.activeId ? s.documents[s.activeId] : null;
+  const layer = doc ? activeLayer(doc) : null;
+  if (!doc || !layer || s.selectedLayerIds.length !== 1 || s.panelOwnsDocument() || s.working) return false;
+  if (doc.selection?.empty || !visibleIds(doc).has(layer.id)) return false;
+  return s.maskTargeted() ? layer.maskEnabled : !layer.isGroup && !layer.adjustment;
+}
+/** The pending gradient as the engine paints it, in the palette's colours now. */
+function currentSpec(e: GradientEdit) {
+  const s = useEditor.getState();
+  return gradientSpec(e, s.gradientOptions, s.paletteColor(false), s.paletteColor(true));
+}
+/** Shows the pending gradient: the engine's preview while it has a line, nothing otherwise. */
+function previewGradient(dragging: boolean): void {
+  const s = useEditor.getState(); const e = s.gradientEdit;
+  if (!e || !s.engine || !s.activeId) return;
+  s.engine.setPreview(s.activeId, hasLine(e) ? { preview: "Gradient", layer: e.layerId, mask: e.mask, gradient: currentSpec(e), dragging } : null);
+  s.refresh(s.activeId);
+}
 /** `withRgb` from an engine sample. */
 const withRgbFrom = (hsb: PickerHSB, rgb: [number, number, number]): PickerHSB => withRgb(hsb, { red: rgb[0], green: rgb[1], blue: rgb[2] });
 
@@ -286,6 +331,49 @@ export const useEditor = create<EditorStore>((set, get) => ({
   adjustEdit: null,
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
   palette: DEFAULT_PALETTE, colorPicker: null, pickerAt: null, sampleRing: null,
+  gradientOptions: DEFAULT_GRADIENT, gradientEdit: null,
+  setGradientOptions: (patch) => { set((s) => ({ gradientOptions: { ...s.gradientOptions, ...patch } })); get().refreshGradient(); },
+  beginGradient: (at) => {
+    const { activeId, documents, gradientEdit } = get(); if (!activeId) return false;
+    const doc = documents[activeId];
+    const layer = activeLayer(doc); if (!layer) return false;
+    const mask = get().maskTargeted();
+    // Dragging a new line replaces the pending one on the same target.
+    if (gradientEdit && gradientEdit.layerId === layer.id && gradientEdit.mask === mask) {
+      set({ gradientEdit: { ...gradientEdit, start: at, end: at } });
+      get().engine!.setPreview(activeId, null); get().refresh(activeId);
+      return true;
+    }
+    if (gradientEdit) get().commitGradient();
+    if (!canPaintNow()) return false;
+    set({ gradientEdit: { layerId: layer.id, mask, start: at, end: at } });
+    return true;
+  },
+  moveGradient: (ends, dragging) => {
+    const e = get().gradientEdit; if (!e) return;
+    set({ gradientEdit: { ...e, ...(ends.start ? { start: ends.start } : {}), ...(ends.end ? { end: ends.end } : {}) } });
+    previewGradient(dragging);
+  },
+  endGradientDrag: () => {
+    const e = get().gradientEdit; if (!e) return;
+    if (!hasLine(e)) { get().cancelGradient(); return; }
+    previewGradient(false);
+  },
+  refreshGradient: () => { if (get().gradientEdit) previewGradient(false); else get().repaintOverlay(); },
+  cancelGradient: () => {
+    const { gradientEdit, engine, activeId } = get(); if (!gradientEdit) return;
+    set({ gradientEdit: null });
+    if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); }
+  },
+  commitGradient: () => {
+    const e = get().gradientEdit; if (!e) return;
+    if (!hasLine(e) || get().working) { get().cancelGradient(); return; }
+    const command: Command = { type: "Gradient", id: e.layerId, mask: e.mask, gradient: currentSpec(e) };
+    set({ gradientEdit: null });
+    // The preview stays on screen until the result is put back (runEditJob clears it then).
+    if (get().usesJob(e.layerId)) { void get().runEditJob(command, e.layerId); return; }
+    if (!get().run(command)) { const { engine, activeId } = get(); if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); } }
+  },
   pickerColor: () => { const p = get().colorPicker; return p ? quantized(hsbToRgb(p.hsb)) : null; },
   openColorPicker: (target) => {
     if (get().working) return false;
@@ -323,7 +411,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
   sampleForeground: (at) => {
     const { engine, activeId, working } = get(); if (!engine || !activeId || working) return;
     const rgb = engine.sampleColor(activeId, at);
-    if (rgb) set({ palette: { ...get().palette, foreground: { red: rgb[0], green: rgb[1], blue: rgb[2] } } });
+    if (rgb) { set({ palette: { ...get().palette, foreground: { red: rgb[0], green: rgb[1], blue: rgb[2] } } }); get().refreshGradient(); }
   },
   setSampleRing: (sampleRing) => { set({ sampleRing }); get().repaintOverlay(); },
   maskTargeted: () => {
@@ -343,23 +431,27 @@ export const useEditor = create<EditorStore>((set, get) => ({
       const white = sameColor(color, WHITE);
       set({ palette: { ...p, maskPaintWhite: background ? !white : white } });
     } else set({ palette: background ? { ...p, background: color } : { ...p, foreground: color } });
+    get().refreshGradient();
   },
   swapPalette: () => {
     if (get().working) return;
     const p = get().palette;
     set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: !p.maskPaintWhite } : { ...p, foreground: p.background, background: p.foreground } });
+    get().refreshGradient();
   },
   resetPalette: () => {
     if (get().working) return;
     const p = get().palette;
     set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: false } : { ...p, foreground: BLACK, background: WHITE } });
+    get().refreshGradient();
   },
   setEngine: (engine) => set({ engine }),
   setJobs: (jobs) => set({ jobs }),
   usesJob: (layerId) => {
-    const { jobs, activeId, documents, jobPixels } = get();
+    const { jobs, engine, activeId, documents, jobPixels } = get();
     const layer = activeId ? documents[activeId]?.layers.find((l) => l.id === layerId) : undefined;
-    return !!jobs && !!layer && layer.pixelsWidth * layer.pixelsHeight > jobPixels;
+    // The stored layer's size: under an open preview the state shows the preview's, maybe reduced.
+    return !!jobs && !!engine && !!layer && engine.storedPixels(activeId!, layerId) > jobPixels;
   },
   runEditJob: async (command, layerId) => {
     const { engine, jobs, activeId } = get();
@@ -396,6 +488,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // Leaving the current document commits its pending edit rather than dropping it, as
     // ProjectWorkspace.select/newCanvas do on macOS.
     get().commitTransform();
+    get().commitGradient();
     dropOpenPanel();
     const state = get().engine!.state(id);
     const viewport = new Viewport();
@@ -405,6 +498,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
   },
   closeDocument: (id) => {
     get().commitTransform();
+    // A pending gradient belongs to the document on screen: closing it drops it, closing another applies it.
+    if (get().activeId === id) set({ gradientEdit: null }); else get().commitGradient();
     // A panel belongs to the active document; closing a background tab leaves it open.
     if (get().activeId === id) dropOpenPanel();
     get().engine!.closeDocument(id);
@@ -423,6 +518,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // Clicking the tab already on screen changes nothing, and so must not cancel its panel.
     if (id === get().activeId) return;
     get().commitTransform();
+    get().commitGradient();
     dropOpenPanel();
     const state = get().documents[id];
     set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null });
@@ -471,6 +567,9 @@ export const useEditor = create<EditorStore>((set, get) => ({
     // `commitTransform` clears `transformEdit` before issuing its own command, so the nested
     // `run` below sees none and this does not recurse.
     if (get().transformEdit) get().commitTransform();
+    // Likewise a pending gradient is applied before anything else records history (the Mac refuses
+    // them while it is pending: `canEditLayers`). `commitGradient` clears it before its own run.
+    if (get().gradientEdit) get().commitGradient();
     const { engine, activeId } = get();
     if (!engine || !activeId) return false;
     try {
@@ -482,10 +581,21 @@ export const useEditor = create<EditorStore>((set, get) => ({
   },
   // A panel owns the document while it is open, as macOS's canEditLayers does; the menu items
   // for these are already disabled, so this stays quiet rather than raising the error banner.
-  undo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
-  redo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
+  // Like Photoshop, the first Undo discards a pending gradient; a Redo drops it too (`undo`, `restore`).
+  undo: () => {
+    if (get().panelOwnsDocument() || get().working) return;
+    if (get().gradientEdit) { get().cancelGradient(); return; }
+    const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); }
+  },
+  redo: () => {
+    if (get().panelOwnsDocument() || get().working) return;
+    get().cancelGradient();
+    const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); }
+  },
   setTool: (tool) => {
     if (get().tool === "move" && tool !== "move") get().commitTransform();
+    // Switching tools applies a pending gradient, as in Photoshop (`resolveGradient`).
+    if (tool !== get().tool) get().commitGradient();
     // Entering the crop tool seeds a rectangle, as macOS does (EditorSession.selectTool): the
     // selection's bounds when there is a selection with something in it, else the canvas (cropSeed).
     // The frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
@@ -516,6 +626,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
     const valid = ids.filter((id) => state.layers.some((l) => l.id === id));
     const active = primary && valid.includes(primary) ? primary : valid[0] ?? null;
     get().commitTransform();
+    // Choosing a layer applies a pending gradient first (`resolveGradient`).
+    get().commitGradient();
     if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
     set({ selectedLayerIds: valid, maskSelected: false });
     get().refresh(activeId);
@@ -524,6 +636,8 @@ export const useEditor = create<EditorStore>((set, get) => ({
   // mask closes a picker open on a swatch: a mask's palette is black and white (ColorPaletteControls.swift:53-56).
   setMaskSelected: (v) => {
     if (get().panelOwnsDocument()) return;
+    // Changing the target applies a pending gradient first.
+    if (v !== get().maskSelected) get().commitGradient();
     set({ maskSelected: v });
     if (get().colorPicker?.target.kind === "palette" && get().maskTargeted()) get().closeColorPicker(false);
   },
@@ -803,6 +917,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
   },
   cycleToolMode: () => {
     const { tool, selectionOptions: o } = get();
+    if (tool === "gradient") get().setGradientOptions({ shape: get().gradientOptions.shape === "Linear" ? "Radial" : "Linear" });
     if (tool === "marquee") get().setSelectionOptions({ marquee: o.marquee === "Rectangle" ? "Ellipse" : "Rectangle" });
     else if (tool === "lasso") get().setSelectionOptions({ lasso: o.lasso === "Freehand" ? "Polygonal" : "Freehand" });
   },
```

```diff
--- a/app/src/styles.css
+++ b/app/src/styles.css
@@ -169,3 +169,5 @@ body { margin: 0; }
 .picker-channel input[aria-label="Hex color"] { width: 84px; font-family: monospace; }
 .picker-hint { margin-top: 8px; font-size: 11px; color: #999; }
 .color-swatch-button { width: 36px; height: 18px; padding: 0; border: 1px solid rgba(0, 0, 0, 0.5); border-radius: 3px; cursor: pointer; }
+.gradient-swatch { display: inline-block; width: 56px; height: 18px; border-radius: 3px; border: 1px solid rgba(0, 0, 0, 0.5); }
+.tool-options .muted { color: #999; }
```

```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -313,6 +313,10 @@ impl WasmEngine {
     pub fn mask_pixels_len(&self, doc: &str, layer: &str) -> Result<usize, JsError> {
         Ok(self.engine.mask_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)?.map_or(0, |m| m.bytes().len()))
     }
+    /// `Engine::stored_pixels`: the layer's stored pixel count, not a preview's.
+    pub fn stored_pixels(&self, doc: &str, layer: &str) -> Result<f64, JsError> {
+        Ok(self.engine.stored_pixels(parse_id(doc)?, parse_id(layer)?).map_err(js_err)? as f64)
+    }
     /// `Engine::layer_region`: the bytes of a rectangle of the layer's pixels at `level`, as shown.
     pub fn layer_region(&self, doc: &str, layer: &str, level: u32, x: u32, y: u32, width: u32, height: u32) -> Result<Uint8Array, JsError> {
         let bytes = self.engine.layer_region(parse_id(doc)?, parse_id(layer)?, level, PixelRect { x, y, width, height }).map_err(js_err)?;
```

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -291,6 +291,12 @@ impl Engine {
     /// The preview showing on the canvas, if any.
     pub fn preview(&self, id: Uuid) -> Option<&PixelPreview> { self.sessions.get(&id).and_then(|s| s.preview.as_ref()) }
     /// The layer's mask as the canvas shows it (a gradient's mask preview in its place).
+    /// The pixels the layer stores (its width times its height; 0 without pixels), whatever a preview
+    /// shows: `state()` reports a previewed layer at the preview's size, which may be a reduced copy.
+    pub fn stored_pixels(&self, id: Uuid, layer: Uuid) -> Result<u64, CommandError> {
+        let l = self.session(id)?.document.layer(layer).ok_or(CommandError::NoLayer)?;
+        Ok(l.pixels.as_ref().map_or(0, |p| p.width as u64 * p.height as u64))
+    }
     pub fn mask_pixels(&self, id: Uuid, layer: Uuid) -> Result<Option<GrayRaster>, CommandError> {
         let doc = self.render_document(id)?;
         Ok(doc.layer(layer).ok_or(CommandError::NoLayer)?.mask.as_ref().map(|m| m.pixels.clone()))
```


- [ ] **Step 4: Run the tests and watch them pass**

Engine: 509 passed, 9 ignored (the new assertion is in an existing test). `pnpm wasm:dev`; `pnpm test`: 199 (+12); `pnpm build`; `pnpm e2e`: 152 passed, 11 skipped (+4). Timings (release wasm, Edge, `-g "gradient tool"`), through the store as the pointer drives it: a drag tick 24 / 25 ms at 24 / 100 MP (budget 50), the release's settled preview 67 / 79 ms (budget 150), Return 73 / 284 ms on the UI thread (the copy out), the worker's painting leaving frames 17 ms apart (budget 100), done after 1.5 / 5.6 s, the copies 70-74 ms and 282-283 ms (budgets 150 / 500). Before `stored_pixels` the 24 MP commit ran on the UI thread: Return 789 ms, 3.2 s at 100 MP.

- [ ] **Step 5: Prove it bites**

(1) In `undo`, drop the pending gradient's discard: "is discarded by the first Undo, and the next Undo undoes" fails (measured). Restore. (2) In `usesJob`, go back to the state's `pixelsWidth * pixelsHeight`: "chooses the job worker by the stored layer's size, not the reduced preview the state shows" fails (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/canvas/gradient-tool.ts app/src/panels/GradientOptions.tsx app/src/state/gradient-edit.ts app/tests/e2e/gradient-tool.spec.ts app/tests/unit/gradient-store.test.ts
git commit -m "feat(app): the Gradient tool, a pending line with handles, previews, and Return to apply" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/App.tsx app/src/actions/layers.ts app/src/canvas/CanvasView.tsx app/src/canvas/gradient-tool.ts app/src/canvas/overlay.ts app/src/canvas/sampling.ts app/src/engine/client.ts app/src/panels/GradientOptions.tsx app/src/panels/MenuBar.tsx app/src/panels/ToolRail.tsx app/src/panels/tool-icons.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/src/state/gradient-edit.ts app/src/state/store.ts app/src/styles.css app/tests/e2e/gradient-tool.spec.ts app/tests/e2e/perf-4b1.spec.ts app/tests/unit/fill-actions.test.ts app/tests/unit/gradient-store.test.ts app/tests/unit/keymap.test.ts app/tests/unit/store-jobs.test.ts app/tests/unit/tool-rail.test.tsx engine-wasm/src/lib.rs engine/src/engine.rs engine/tests/gradient_preview.rs
```


---

### Task 16: The Shape tool

The Shape tool (U; Shift+U or Tab steps Rectangle, Ellipse, Line; drawn for this port after SF `square.on.circle`): a press starts a draft at the nearest pixel corner, a drag shapes it (Shift squares a box or holds a line to 45 degrees about its start, Alt grows a box from its centre), and the release makes the layer (ShapeTool.swift:70-149; ruling OQ11). The draft is drawn on the overlay only, in the foreground colour: a filled box (corners rounded by the radius, at most half the shorter side) or ellipse, or a line stroked with round ends at its width, at least 1 view px. A click, Escape, a tool change or a new kind makes nothing. The bar (ShapeControls.swift): the kind, a rectangle's Radius (slider to 200, field to 5000) or a line's Width (slider to 100, field to 5000), and the Fill swatch (the foreground; it opens the picker). A shape takes the image's foreground even while a mask is the target. The engine's shape raster premultiplies through a 256-entry table (this task's perf run: 1.3 s at 24 MP on the UI thread before it).

**Files:**
- Create: `app/src/tools/shape-draft.ts`, `app/src/panels/ShapeOptions.tsx`, `app/src/canvas/shape-tool.ts`
- Modify: `app/src/state/store.ts` (`"shape"`, `shapeOptions`, `shapeDraft`, `setShapeOptions`, `setShapeDraft`, `finishShape`, Tab), `app/src/canvas/CanvasView.tsx`, `app/src/canvas/overlay.ts`, `app/src/panels/ToolRail.tsx`, `app/src/panels/tool-icons.tsx`, `app/src/shortcuts/keymap.ts`, `app/src/shortcuts/useShortcuts.ts` (U, Shift+U, Escape), `app/src/App.tsx`, `engine/src/ops/shape.rs` (the table)
- Create tests: `app/tests/unit/shape-draft.test.ts`, `app/tests/e2e/shape-tool.spec.ts`; modify `keymap.test.ts`, `tool-rail.test.tsx`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Produces: `ShapeKind`, `SHAPE_KINDS`, `ShapeOptions`, `DEFAULT_SHAPE`, `SHAPE_FIELD_MAX`, `ShapeDraft`, `nextShapeKind`, `beginShape`, `dragShape`, `shapeSpec`; the store's shape API; `installShapeTool`; the actions `tool-shape` (U) and `shape-next` (Shift+U).

- [ ] **Step 1: Write the tests**

The unit tests port ShapeToolTests' drafts with their numbers: (20, 20) to (50, 50) is the box (20, 20, 30, 30); a fractional press and pointer land on the nearest corners; (50, 40) to (60, 45) with Shift and Alt is the ellipse (40, 30, 20, 20); the radius is taken for rectangles only, when the drag begins; a click or a box under a pixel makes nothing (a click with a 4 px line is a 4 x 4 dot, as on the Mac); a line keeps its end where the pointer is, and Shift holds it to 45 degrees; the kinds step round; finishing sends one AddShape in the image's foreground (with a mask targeted) and drops the draft; Escape, a tool change or a new kind drop it; Shift+U picks the tool, then steps the kind. The e2e: U and a drag make "Rectangle 1" at (10, 10) 30 x 20 in red, one undo step, keeping the selection, drawn on the overlay while dragging and gone after (the Mac test's pixels); Shift+U, a click and Escape make nothing, and Shift+Alt makes "Ellipse 1" at (40, 30) 20 x 20; a 6 px line from (20, 30) to (70, 30) is "Line 1" at (17, 27) 56 x 6.

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -434,3 +434,61 @@ test("gradient tool: drag ticks through the store and the commit through the wor
   // 256 MiB, so none is kept (HISTORY_BYTE_LIMIT, as the Mac's DocumentHistory).
   expect([out["24 MP: undo depth"], out["100 MP: undo depth"]]).toEqual([2, 0]);
 });
+
+test("shapes and fills: a shape over the canvas on the UI thread, and a fill of the whole layer through the worker, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      const s = () => api.store.getState();
+      // An empty canvas: the shape's pixels are the project's only ones.
+      const blank = api.engine.newDocument(w, h, false);
+      s().openDocument(blank);
+      await settle(); frame();
+      for (const [name, draft] of [["rectangle", { kind: "Rectangle", anchor: { x: 0, y: 0 }, rect: { x: 0, y: 0, width: w, height: h }, end: null, cornerRadius: 400 }],
+        ["ellipse", { kind: "Ellipse", anchor: { x: 0, y: 0 }, rect: { x: 0, y: 0, width: w, height: h }, end: null, cornerRadius: 0 }]] as const) {
+        s().setShapeDraft(draft);
+        const t0 = performance.now();
+        s().finishShape();
+        result[`${name} over the canvas (release, UI thread) ms`] = Math.round(performance.now() - t0);
+        result[`frame after the ${name} ms`] = Math.round(frame());
+        s().undo(); await settle();
+      }
+      s().closeDocument(blank);
+      // A fill of a whole layer, through the worker.
+      let installedAt = Infinity;
+      const f = api.engine.installJob.bind(api.engine);
+      api.engine.installJob = (...a: unknown[]) => { try { return f(...a); } finally { installedAt = performance.now(); } };
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      api.engine.execute(doc, { type: "SetActiveLayer", id: api.engine.state(doc).layers[0].id });
+      s().openDocument(doc);
+      await settle(); frame();
+      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
+        let last = performance.now(), gap = 0;
+        const tick = (t: number) => { if (t <= installedAt) gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+        requestAnimationFrame(tick);
+      });
+      const t0 = performance.now();
+      window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
+      result["Alt+Backspace (UI thread) ms"] = Math.round(performance.now() - t0);
+      result["fill: longest frame gap while the worker fills"] = await longestGap(() => !s().working);
+      result["fill done after ms"] = Math.round(performance.now() - t0);
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`shapes and fills (release wasm, Edge): ${JSON.stringify(out)}`);
+  // Shapes commit on the UI thread, as an import does (ruling OQ12); the first pays for the wasm
+  // memory growing to hold it.
+  expect(out["24 MP: rectangle over the canvas (release, UI thread) ms"]).toBeLessThan(1000);
+  expect(out["24 MP: ellipse over the canvas (release, UI thread) ms"]).toBeLessThan(1000);
+  expect(out["100 MP: rectangle over the canvas (release, UI thread) ms"]).toBeLessThan(2000);
+  expect(out["100 MP: ellipse over the canvas (release, UI thread) ms"]).toBeLessThan(2000);
+  for (const label of ["24 MP", "100 MP"]) expect(out[`${label}: fill: longest frame gap while the worker fills`]).toBeLessThan(100);
+});
```

Create `app/tests/e2e/shape-tool.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";

// Phase 4b-1: the Shape tool through the real canvas, keys and bar (the engine's shapes are pinned
// in engine/tests/shapes.rs, ported from the Mac's ShapeToolTests).

type Pt = [number, number];

/** The Mac test's session: 100 x 80 with one empty layer, red foreground, at 4 CSS px a pixel. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  await page.evaluate(async () => {
    const api = (window as any).__compositor;
    api.store.getState().openDocument(api.engine.newDocument(100, 80, true));
    await api.setZoom(4);
    api.store.getState().setPaletteColor({ red: 1, green: 0, blue: 0 }, false);
  });
}
const client = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height });
  const r = document.querySelector('[data-testid="canvas-view"]')!.getBoundingClientRect();
  return { x: r.left + v.x, y: r.top + v.y };
}, p);
const doc = (page: Page) => page.evaluate(() => { const s = (window as any).__compositor.store.getState(); return s.documents[s.activeId]; });
const pixel = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  const p = Array.from(api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array);
  return [p[0], p[3]];
}, [x, y]);
/** The overlay's colour at document point `p` (straight RGBA). */
const overlayAt = (page: Page, p: Pt) => page.evaluate(([x, y]) => {
  const s = (window as any).__compositor.store.getState(); const vp = s.viewports[s.activeId]; const d = s.documents[s.activeId];
  const v = vp.viewPoint({ x, y }, { width: d.width, height: d.height }); const dpr = window.devicePixelRatio || 1;
  const overlay = document.querySelector('[data-testid="overlay"]') as HTMLCanvasElement;
  return Array.from(overlay.getContext("2d")!.getImageData(Math.round(v.x * dpr), Math.round(v.y * dpr), 1, 1).data);
}, p);

test("U and a drag make a rectangle on a new layer in the foreground colour, one undo step, keeping the selection", async ({ page }) => {
  await setup(page);
  await page.evaluate(() => (window as any).__compositor.store.getState().run({ type: "SelectAll" }));
  const depth = (await doc(page)).undoDepth;
  await page.keyboard.press("u");
  await expect(page.getByTestId("shape-options")).toBeVisible();
  const a = await client(page, [10, 10]), b = await client(page, [40, 30]), mid = await client(page, [30, 25]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(mid.x, mid.y, { steps: 3 });
  // While dragging, the shape shows on the overlay in red, and nothing is in the document yet.
  expect(await overlayAt(page, [20, 20])).toEqual([255, 0, 0, 255]);
  expect((await doc(page)).layers.length).toBe(1);
  await page.mouse.move(b.x, b.y, { steps: 3 }); await page.mouse.up();
  const d = await doc(page);
  expect(d.layers.map((l: any) => l.name)).toEqual(["Layer 1", "Rectangle 1"]);
  const layer = d.layers[1];
  expect([d.activeLayerId, layer.transform.origin, layer.transform.size]).toEqual([layer.id, [10, 10], [30, 20]]);
  expect(d.undoDepth).toBe(depth + 1);
  expect(d.selection, "unlike Paste, drawing a shape keeps the selection").not.toBeNull();
  for (const [x, y] of [[25, 20], [10, 10], [39, 29]]) expect(await pixel(page, x, y)).toEqual([255, 255]);
  for (const [x, y] of [[9, 20], [40, 20], [25, 30]]) expect((await pixel(page, x, y))[1]).toBe(0);
  expect(await overlayAt(page, [20, 20]), "the draft is gone from the overlay").toEqual([0, 0, 0, 0]);
  await page.keyboard.press("Control+z");
  expect((await doc(page)).layers.length).toBe(1);
});

test("Shift+U steps to the Ellipse; Shift and Alt make a circle from its centre; a click and Escape make nothing", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("u");
  await page.keyboard.press("Shift+U");
  await expect(page.getByTestId("shape-ellipse")).toHaveAttribute("aria-pressed", "true");
  const depth = (await doc(page)).undoDepth;
  // A click.
  const c = await client(page, [20, 20]);
  await page.mouse.click(c.x, c.y);
  // Escape mid-drag.
  await page.mouse.move(c.x, c.y); await page.mouse.down();
  const far = await client(page, [50, 50]);
  await page.mouse.move(far.x, far.y, { steps: 3 });
  await page.keyboard.press("Escape");
  await page.mouse.up();
  expect((await doc(page)).undoDepth, "nothing made").toBe(depth);
  // (50, 40) to (60, 45) with Shift and Alt: the box (40, 30, 20, 20).
  const a = await client(page, [50, 40]), b = await client(page, [60, 45]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.keyboard.down("Shift"); await page.keyboard.down("Alt");
  await page.mouse.move(b.x, b.y, { steps: 3 }); await page.mouse.up();
  await page.keyboard.up("Alt"); await page.keyboard.up("Shift");
  const layer = (await doc(page)).layers[1];
  expect([layer.name, layer.transform.origin, layer.transform.size]).toEqual(["Ellipse 1", [40, 30], [20, 20]]);
  expect(await pixel(page, 50, 40)).toEqual([255, 255]);
  expect((await pixel(page, 41, 40))[1]).toBeGreaterThan(0);
  expect((await pixel(page, 40, 30))[1]).toBe(0);
});

test("a line takes the bar's width and lands between the points it was dragged between", async ({ page }) => {
  await setup(page);
  await page.keyboard.press("u");
  await page.getByTestId("shape-line").click();
  await page.getByTestId("shape-width").fill("6");
  await page.getByTestId("shape-width").press("Enter");
  const a = await client(page, [20, 30]), b = await client(page, [70, 30]);
  await page.mouse.move(a.x, a.y); await page.mouse.down();
  await page.mouse.move(b.x, b.y, { steps: 4 }); await page.mouse.up();
  const layer = (await doc(page)).layers[1];
  // (20, 30) to (70, 30) grown by 3 on each side: 56 x 6 at (17, 27).
  expect([layer.name, layer.transform.origin, layer.transform.size]).toEqual(["Line 1", [17, 27], [56, 6]]);
  expect(await pixel(page, 45, 29)).toEqual([255, 255]);
  expect((await pixel(page, 45, 33))[1]).toBe(0);
});
```

```diff
--- a/app/tests/unit/keymap.test.ts
+++ b/app/tests/unit/keymap.test.ts
@@ -26,6 +26,8 @@ describe("keymap", () => {
     expect(matchShortcut(ev("d"))).toBe("default-colors");
     expect(matchShortcut(ev("i"))).toBe("tool-eyedropper");
     expect(matchShortcut(ev("g"))).toBe("tool-gradient");
+    expect(matchShortcut(ev("u"))).toBe("tool-shape");
+    expect(matchShortcut(ev("U", { shiftKey: true }))).toBe("shape-next");
     expect(matchShortcut(ev("Backspace", { altKey: true }))).toBe("fill-foreground");
     expect(matchShortcut(ev("Delete", { ctrlKey: true }))).toBe("fill-background");
     expect(matchShortcut(ev("Backspace"))).toBe("delete-layer");
```

Create `app/tests/unit/shape-draft.test.ts`:

```ts
import { beforeEach, describe, expect, it } from "vitest";
import { beginShape, dragShape, DEFAULT_SHAPE, nextShapeKind, shapeSpec } from "../../src/tools/shape-draft";
import { DEFAULT_PALETTE, useEditor } from "../../src/state/store";
import { runAction } from "../../src/shortcuts/useShortcuts";
import type { Command, DocumentState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The Shape tool's drag, ported from the Mac's ShapeToolTests with their numbers.
const drag = (from: [number, number], to: [number, number], options = DEFAULT_SHAPE, square = false, fromCenter = false) =>
  dragShape(beginShape({ x: from[0], y: from[1] }, options), { x: to[0], y: to[1] }, square, fromCenter);

describe("the Shape tool's drag", () => {
  it("draws a box between whole pixels (aClickEscapeOrToolSwitchMakesNoLayer)", () => {
    expect(drag([20, 20], [50, 50]).rect).toEqual({ x: 20, y: 20, width: 30, height: 30 });
    // A fractional press and pointer land on the nearest corners.
    expect(drag([10.4, 9.6], [39.5, 29.2]).rect).toEqual({ x: 10, y: 10, width: 30, height: 19 });
  });
  it("squares with Shift and grows from the centre with Alt (ellipseLeavesItsCornersClear...)", () => {
    const d = drag([50, 40], [60, 45], { ...DEFAULT_SHAPE, kind: "Ellipse" }, true, true);
    expect(shapeSpec(d, 4)).toEqual({ kind: "Ellipse", rect: { x: 40, y: 30, width: 20, height: 20 } });
  });
  it("takes the corner radius for rectangles only, fixed when the drag begins (roundedRectangles...)", () => {
    expect(beginShape({ x: 5, y: 5 }, { ...DEFAULT_SHAPE, kind: "Ellipse", cornerRadius: 8 }).cornerRadius).toBe(0);
    const d = drag([10, 10], [50, 40], { ...DEFAULT_SHAPE, cornerRadius: 8 });
    expect(shapeSpec(d, 4)).toEqual({ kind: "Rectangle", rect: { x: 10, y: 10, width: 40, height: 30 }, cornerRadius: 8 });
  });
  it("makes nothing from a click, or from a box under a pixel on a side", () => {
    expect(shapeSpec(beginShape({ x: 20, y: 20 }, DEFAULT_SHAPE), 4)).toBeNull();
    expect(shapeSpec(drag([20, 20], [50, 20.4]), 4)).toBeNull();
    // A line's box is its ends grown by its width: a click with a 4 px line is a 4 x 4 dot, as on the Mac.
    const dot = beginShape({ x: 20, y: 20 }, { ...DEFAULT_SHAPE, kind: "Line" });
    expect(shapeSpec(dot, 4)).toEqual({ kind: "Line", start: [20, 20], end: [20, 20], width: 4 });
    expect(shapeSpec(dot, 0.5)).toBeNull();
  });
  it("keeps a line's end where the pointer is, and Shift holds it to 45 degrees about the start", () => {
    const line = { ...DEFAULT_SHAPE, kind: "Line" as const };
    expect(shapeSpec(drag([20.6, 120.4], [140.3, 121.2], line), 1)).toEqual({ kind: "Line", start: [21, 120], end: [140.3, 121.2], width: 1 });
    const d = drag([10, 10], [40, 12], line, true);
    expect(d.end!.y).toBeCloseTo(10, 12);
    expect(d.end!.x).toBeCloseTo(10 + Math.hypot(30, 2), 12);
  });
  it("steps Rectangle, Ellipse, Line and round again", () => {
    expect([nextShapeKind("Rectangle"), nextShapeKind("Ellipse"), nextShapeKind("Line")]).toEqual(["Ellipse", "Line", "Rectangle"]);
  });
});

describe("finishing a shape", () => {
  let log: Command[] = [];
  beforeEach(() => {
    log = [];
    const state = { id: "D", documentId: "D", width: 100, height: 80, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false,
      undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [] } as DocumentState;
    const engine = { state: () => state, execute: (_id: string, c: Command) => { log.push(c); return { structure: true, canvas: false, layers: [] }; } } as unknown as EngineClient;
    useEditor.setState({ engine, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [], maskSelected: true, working: false,
      palette: { ...DEFAULT_PALETTE, foreground: { red: 1, green: 0, blue: 0 } }, shapeOptions: DEFAULT_SHAPE, shapeDraft: null, tool: "shape",
      adjustEdit: null, transformEdit: null, gradientEdit: null, error: null, collapsed: {} });
  });
  it("sends one AddShape in the image's foreground colour, even with a mask targeted, and drops the draft", () => {
    const s = useEditor.getState();
    s.setShapeDraft(drag([10, 10], [40, 30]));
    s.finishShape();
    expect(log).toEqual([{ type: "AddShape", shape: { kind: "Rectangle", rect: { x: 10, y: 10, width: 30, height: 20 }, cornerRadius: 0 }, color: [1, 0, 0] }]);
    expect(useEditor.getState().shapeDraft).toBeNull();
  });
  it("is dropped by Escape, a tool change or a new kind, sending nothing; Shift+U steps the kind", () => {
    const s = useEditor.getState();
    s.setShapeDraft(drag([10, 10], [40, 30])); runAction("cancel");
    expect(useEditor.getState().shapeDraft).toBeNull();
    s.setShapeDraft(drag([10, 10], [40, 30])); s.setShapeOptions({ kind: "Line" });
    expect(useEditor.getState().shapeDraft).toBeNull();
    s.setShapeDraft(drag([10, 10], [40, 30])); s.setTool("move");
    expect(useEditor.getState().shapeDraft).toBeNull();
    expect(log).toEqual([]);
    runAction("shape-next");
    expect(useEditor.getState().tool).toBe("shape");
    runAction("shape-next");
    expect(useEditor.getState().shapeOptions.kind).toBe("Rectangle");
  });
});
```

```diff
--- a/app/tests/unit/tool-rail.test.tsx
+++ b/app/tests/unit/tool-rail.test.tsx
@@ -31,7 +31,7 @@ describe("ToolRail", () => {
   it("shows an icon, not a letter, on every tool, named for assistive technology", () => {
     mount();
     const expected: Record<string, string> = { move: "Move", marquee: "Marquee", lasso: "Lasso", wand: "Magic Wand",
-      crop: "Crop", gradient: "Gradient", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
+      crop: "Crop", gradient: "Gradient", shape: "Shape", eyedropper: "Eyedropper", hand: "Hand", zoom: "Zoom" };
     for (const [id, label] of Object.entries(expected)) {
       const b = button(id);
       expect(b.querySelectorAll("svg").length, id).toBe(1);
```


- [ ] **Step 2: Run the tests and watch them fail**

`pnpm test`: `shape-draft.test.ts` does not compile; the rail has no Shape; `keymap` fails at U.

- [ ] **Step 3: Implement**

```diff
--- a/app/src/App.tsx
+++ b/app/src/App.tsx
@@ -20,6 +20,7 @@ import { JpegExportSheet } from "./sheets/JpegExportSheet";
 import { CropOptions } from "./panels/CropOptions";
 import { SelectionOptions } from "./panels/SelectionOptions";
 import { GradientOptions } from "./panels/GradientOptions";
+import { ShapeOptions } from "./panels/ShapeOptions";
 import { SelectionAmountSheet } from "./sheets/SelectionAmountSheet";
 import { TransformInspector } from "./panels/TransformInspector";
 import { AdjustPanel } from "./panels/AdjustPanel";
@@ -95,6 +96,7 @@ export function App() {
       <CropOptions />
       <SelectionOptions />
       <GradientOptions />
+      <ShapeOptions />
       <TransformInspector />
       <div className="workspace">
         <ToolRail />
```

```diff
--- a/app/src/canvas/CanvasView.tsx
+++ b/app/src/canvas/CanvasView.tsx
@@ -12,6 +12,7 @@ import { antsDelay, AntsPathCache, ANTS_INTERVAL_MS, nextPhase, OutlineCache, ou
 import { isSelectionTool, outlineOffset, SelectionDraft, selectionMode, type P as DocP } from "../tools/selection-draft";
 import { installSampling } from "./sampling";
 import { gradientLine, installGradientTool } from "./gradient-tool";
+import { installShapeTool } from "./shape-tool";
 import { gradientStops } from "../state/gradient-edit";
 
 export const HIT_HANDLE_PX = 6;
@@ -79,6 +80,8 @@ export function CanvasView() {
       transform: transformGeometry, canvasGuides: s.showGuides ? doc.guides : null,
       ants, draft: d ? { kind: d.kind, points: d.points, cursor: d.cursor } : null, sampleRing: s.sampleRing,
       gradientLine: line ? { ...line, radial: s.gradientOptions.shape === "Radial", from, to } : null,
+      shapeDraft: s.shapeDraft ? { kind: s.shapeDraft.kind, rect: s.shapeDraft.rect, start: s.shapeDraft.anchor, end: s.shapeDraft.end,
+        cornerRadius: s.shapeDraft.cornerRadius, lineWidth: s.shapeOptions.lineWidth, color: s.palette.foreground } : null,
     });
   };
 
@@ -203,6 +206,12 @@ export function CanvasView() {
     return installSampling(el, () => spaceRef.current);
   }, []);
 
+  // The Shape tool's drag (canvas/shape-tool.ts).
+  useEffect(() => {
+    const el = glRef.current?.parentElement; if (!el) return;
+    return installShapeTool(el, () => spaceRef.current);
+  }, []);
+
   // The Gradient tool's line (canvas/gradient-tool.ts).
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
@@ -235,7 +244,7 @@ export function CanvasView() {
   const picking = useEditor((s) => !!s.colorPicker);
   useEffect(() => {
     const el = glRef.current?.parentElement; if (!el) return;
-    el.style.cursor = sampleMode || picking || tool === "eyedropper" || tool === "gradient" || isSelectionTool(tool) ? "crosshair" : "";
+    el.style.cursor = sampleMode || picking || tool === "eyedropper" || tool === "gradient" || tool === "shape" || isSelectionTool(tool) ? "crosshair" : "";
   }, [sampleMode, tool, picking]);
 
   // Drag to pan with the hand tool or the space bar.
```

```diff
--- a/app/src/canvas/overlay.ts
+++ b/app/src/canvas/overlay.ts
@@ -24,6 +24,9 @@ export interface OverlayState {
   draft?: DraftState | null;
   /** The pending gradient's line in view px, its shape and its two stops as straight RGBA. */
   gradientLine?: { start: { x: number; y: number }; end: { x: number; y: number }; radial: boolean; from: number[]; to: number[] } | null;
+  /** A shape being dragged out: its kind, its box and a line's ends in document px, its corner
+   * radius and a line's width in document px, and the colour it will be made in. */
+  shapeDraft?: { kind: "Rectangle" | "Ellipse" | "Line"; rect: { x: number; y: number; width: number; height: number }; start: { x: number; y: number }; end: { x: number; y: number } | null; cornerRadius: number; lineWidth: number; color: PaletteColor } | null;
   /** While the canvas is sampled: the ring about the pointer (view px). */
   sampleRing?: { at: { x: number; y: number }; sampled: PaletteColor; original: PaletteColor } | null;
 }
@@ -54,6 +57,30 @@ function drawGradientLine(ctx: CanvasRenderingContext2D, line: NonNullable<Overl
   ctx.restore();
 }
 
+/** The shape being dragged out, in the colour it will be made in (`drawShapeDraft`,
+ * EditorCanvas.swift:983-1005): a line stroked with round ends at its width, at least 1 view px;
+ * otherwise its box filled as a rectangle (corners rounded, at most half its shorter side) or an
+ * ellipse. Nothing until it has a size. */
+function drawShapeDraft(ctx: CanvasRenderingContext2D, viewport: Viewport, size: { width: number; height: number }, d: NonNullable<OverlayState["shapeDraft"]>): void {
+  const scale = viewport.pointsPerPixel;
+  ctx.save();
+  ctx.fillStyle = ctx.strokeStyle = cssColor(d.color);
+  if (d.kind === "Line") {
+    if (!d.end || (d.rect.width <= 0 && d.rect.height <= 0)) { ctx.restore(); return; }
+    const a = viewport.viewPoint(d.start, size), b = viewport.viewPoint(d.end, size);
+    ctx.lineWidth = Math.max(1, d.lineWidth * scale); ctx.lineCap = "round";
+    ctx.beginPath(); ctx.moveTo(a.x, a.y); ctx.lineTo(b.x, b.y); ctx.stroke();
+  } else if (d.rect.width > 0 && d.rect.height > 0) {
+    const tl = viewport.viewPoint({ x: d.rect.x, y: d.rect.y }, size);
+    const w = d.rect.width * scale, h = d.rect.height * scale;
+    ctx.beginPath();
+    if (d.kind === "Ellipse") ctx.ellipse(tl.x + w / 2, tl.y + h / 2, w / 2, h / 2, 0, 0, Math.PI * 2);
+    else ctx.roundRect(tl.x, tl.y, w, h, Math.min(Math.max(0, d.cornerRadius * scale), w / 2, h / 2));
+    ctx.fill();
+  }
+  ctx.restore();
+}
+
 /** The side of the sample ring's box, in view px (`SampleRingOverlay`: a 116 pt frame). */
 export const SAMPLE_RING = 116;
 
@@ -192,6 +219,7 @@ export function drawOverlay(ctx: CanvasRenderingContext2D, viewport: Viewport, d
   }
   if (state.ants) drawAnts(ctx, state.ants);
   if (state.draft) drawDraft(ctx, viewport, size, state.draft);
+  if (state.shapeDraft) drawShapeDraft(ctx, viewport, size, state.shapeDraft);
   if (state.gradientLine) drawGradientLine(ctx, state.gradientLine);
   if (state.sampleRing) drawSampleRing(ctx, state.sampleRing);
   ctx.strokeStyle = "#ff40ff"; ctx.lineWidth = 1;
```

Create `app/src/canvas/shape-tool.ts`:

```ts
import { useEditor } from "../state/store";
import { beginShape, dragShape } from "../tools/shape-draft";

/** The Shape tool's pointer (EditorCanvas.swift:1467-1469, :1532-1537, :1684-1687): a press starts a
 * draft at the nearest pixel corner, a drag shapes it (Shift squares a box or holds a line to 45
 * degrees, Alt grows it from its centre), the release makes the layer; a click makes nothing. The
 * draft is drawn on the overlay alone. Returns the cleanup. */
export function installShapeTool(el: HTMLElement, spaceHeld: () => boolean): () => void {
  let dragging = false;
  const docPoint = (e: PointerEvent) => {
    const s = useEditor.getState(); const d = s.documents[s.activeId!];
    const r = el.getBoundingClientRect();
    return s.viewports[s.activeId!].documentPoint({ x: e.clientX - r.left, y: e.clientY - r.top }, { width: d.width, height: d.height });
  };
  const down = (e: PointerEvent) => {
    const s = useEditor.getState();
    if (s.tool !== "shape" || e.button !== 0 || spaceHeld() || !s.activeId || s.working) return;
    if (s.panelOwnsDocument(true)) return;
    dragging = true;
    el.setPointerCapture(e.pointerId);
    s.setShapeDraft(beginShape(docPoint(e), s.shapeOptions));
  };
  const move = (e: PointerEvent) => {
    const s = useEditor.getState(); const draft = s.shapeDraft;
    if (!dragging || !draft) return;
    s.setShapeDraft(dragShape(draft, docPoint(e), e.shiftKey, e.altKey));
  };
  const up = (e: PointerEvent) => {
    if (!dragging) return;
    dragging = false;
    const s = useEditor.getState(); const draft = s.shapeDraft;
    if (!draft) return;
    s.setShapeDraft(dragShape(draft, docPoint(e), e.shiftKey, e.altKey));
    s.finishShape();
  };
  // A drag that lost its pointer makes nothing.
  const forget = () => { if (dragging) { dragging = false; useEditor.getState().setShapeDraft(null); } };
  el.addEventListener("pointerdown", down); el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", forget);
  return () => {
    el.removeEventListener("pointerdown", down); el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", forget);
  };
}
```

Create `app/src/panels/ShapeOptions.tsx`:

```tsx
import { useEditor } from "../state/store";
import { SHAPE_FIELD_MAX, SHAPE_KINDS } from "../tools/shape-draft";
import { cssColor } from "../tools/color";
import { NumberInput } from "./NumberInput";

/** The Shape tool's bar (ShapeControls.swift): Rectangle, Ellipse or Line; a rectangle's Radius or a
 * line's Width (sliders to 200 and 100, fields to 5000 px); and the Fill swatch, the foreground colour,
 * which opens the picker. Controls give up the focus once used, as the other bars' do. */
export function ShapeOptions() {
  const s = useEditor();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  if (s.tool !== "shape" || !doc || s.sheet !== null) return null;
  const o = s.shapeOptions;
  const disabled = s.working;
  const amount = (label: "Radius" | "Width", key: "cornerRadius" | "lineWidth", min: number, sliderMax: number, help: string) => (
    <label title={help}>{label}{" "}
      <input type="range" aria-hidden tabIndex={-1} min={min} max={sliderMax} step={1} value={Math.min(sliderMax, o[key])} disabled={disabled}
        onChange={(e) => s.setShapeOptions({ [key]: Math.round(Number(e.target.value)) })} onPointerUp={(e) => e.currentTarget.blur()} />
      <NumberInput label={label} testId={`shape-${key === "cornerRadius" ? "radius" : "width"}`} value={o[key]} min={min} max={SHAPE_FIELD_MAX} step={1} blurOnEnter disabled={disabled}
        onChange={(v) => s.setShapeOptions({ [key]: v })} /> px
    </label>
  );
  return (
    <div className="tool-options" data-testid="shape-options">
      <strong>Shape</strong>
      <span className="segmented" title="Shift+U (or Tab) steps through Rectangle, Ellipse and Line">
        {SHAPE_KINDS.map((k) => (
          <button key={k} data-testid={`shape-${k.toLowerCase()}`} aria-pressed={o.kind === k} disabled={disabled}
            onClick={(e) => { s.setShapeOptions({ kind: k }); e.currentTarget.blur(); }}>{k}</button>
        ))}
      </span>
      {o.kind === "Line" && amount("Width", "lineWidth", 1, 100, "The line's thickness")}
      {o.kind === "Rectangle" && amount("Radius", "cornerRadius", 0, 200, "Round the rectangle's corners by this many pixels; 0 keeps them square")}
      <label title="Shapes fill with the foreground color; click to change it">Fill{" "}
        <button className="color-swatch-button" data-testid="shape-fill" aria-label="Fill color" style={{ background: cssColor(s.palette.foreground) }} disabled={disabled}
          onClick={(e) => { e.currentTarget.blur(); s.openColorPicker({ kind: "palette", background: false }); }} />
      </label>
    </div>
  );
}
```

```diff
--- a/app/src/panels/ToolRail.tsx
+++ b/app/src/panels/ToolRail.tsx
@@ -5,7 +5,7 @@ import { PaletteSwatches } from "./PaletteSwatches";
 const TOOLS: { id: Tool; label: string; key: string }[] = [
   { id: "move", label: "Move", key: "V" }, { id: "marquee", label: "Marquee", key: "M" },
   { id: "lasso", label: "Lasso", key: "L" }, { id: "wand", label: "Magic Wand", key: "W" },
-  { id: "crop", label: "Crop", key: "C" }, { id: "gradient", label: "Gradient", key: "G" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
+  { id: "crop", label: "Crop", key: "C" }, { id: "gradient", label: "Gradient", key: "G" }, { id: "shape", label: "Shape", key: "U" }, { id: "eyedropper", label: "Eyedropper", key: "I" },
   { id: "hand", label: "Hand", key: "H" }, { id: "zoom", label: "Zoom", key: "Z" },
 ];
 export function ToolRail() {
```

```diff
--- a/app/src/panels/tool-icons.tsx
+++ b/app/src/panels/tool-icons.tsx
@@ -19,7 +19,7 @@ import type { LassoKind, MarqueeKind } from "../tools/selection-draft";
 /** Which icon a tool shows: the Marquee and the Lasso follow their mode, as on the Mac
  *  (ContentView.swift: circle.dashed in Ellipse mode, its own icon for the polygonal lasso). */
 export type ToolIconName = "move" | "marquee-rectangle" | "marquee-ellipse" | "lasso-freehand" | "lasso-polygonal"
-  | "wand" | "crop" | "gradient" | "eyedropper" | "hand" | "zoom";
+  | "wand" | "crop" | "gradient" | "shape" | "eyedropper" | "hand" | "zoom";
 
 export function toolIconName(tool: Tool, marquee: MarqueeKind, lasso: LassoKind): ToolIconName {
   switch (tool) {
@@ -59,6 +59,9 @@ const SHAPES: Record<ToolIconName, ReactNode> = {
   // Drawn for this port in Lucide's style after SF square.bottomhalf.filled (the Mac's symbol for the
   // tool): a rounded square with its lower half filled.
   "gradient": <><rect x="3" y="3" width="18" height="18" rx="2" /><path d="M3 12h18v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="currentColor" /></>,
+  // Drawn for this port in Lucide's style after SF square.on.circle (the Mac's symbol for the tool): a
+  // rounded square over the lower right of a circle.
+  "shape": <><path d="M14 8.5A6.5 6.5 0 1 0 8.5 15" /><rect x="10" y="10" width="11" height="11" rx="2" /></>,
   // lucide pipette (SF eyedropper)
   "eyedropper": <>
     <path d="m12 9-8.414 8.414A2 2 0 0 0 3 18.828v1.344a2 2 0 0 1-.586 1.414A2 2 0 0 1 3.828 21h1.344a2 2 0 0 0 1.414-.586L15 12" />
```

```diff
--- a/app/src/shortcuts/keymap.ts
+++ b/app/src/shortcuts/keymap.ts
@@ -5,7 +5,7 @@ export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "exp
   | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
   | "levels" | "curves" | "hue-saturation" | "invert"
   | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
-  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background" | "tool-gradient";
+  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background" | "tool-gradient" | "tool-shape" | "shape-next";
 
 export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }
 
@@ -40,6 +40,8 @@ export const SHORTCUTS = {
   // The palette (EditorCanvas.swift:1835-1836).
   "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
   "tool-eyedropper": [{ key: "i" }], "tool-gradient": [{ key: "g" }],
+  // U picks the Shape tool; Shift+U steps its kind (or picks the tool) (EditorCanvas.swift:1843-1845).
+  "tool-shape": [{ key: "u" }], "shape-next": [{ key: "u", shift: true }],
   // Photoshop's fill keys (CompositorApp.swift:175-186: Option-Delete and Command-Delete on the Mac).
   "fill-foreground": [{ key: "Backspace", alt: true }, { key: "Delete", alt: true }],
   "fill-background": [{ key: "Backspace", ctrl: true }, { key: "Delete", ctrl: true }],
```

```diff
--- a/app/src/shortcuts/useShortcuts.ts
+++ b/app/src/shortcuts/useShortcuts.ts
@@ -80,6 +80,8 @@ export function runAction(id: ActionId, shift = false): void {
     case "tool-wand": s.setTool("wand"); break;
     case "tool-eyedropper": s.setTool("eyedropper"); break;
     case "tool-gradient": s.setTool("gradient"); break;
+    case "tool-shape": s.setTool("shape"); break;
+    case "shape-next": if (s.tool === "shape") s.cycleToolMode(); else s.setTool("shape"); break;
     case "fill-foreground": if (doc && !s.sheet) fillActive(false); break;
     case "fill-background": if (doc && !s.sheet) fillActive(true); break;
     case "select-all": if (doc) s.run({ type: "SelectAll" }); break;
@@ -101,6 +103,7 @@ export function runAction(id: ActionId, shift = false): void {
       if (s.panelOwnsDocument()) break;
       if (s.selectionDraft) s.setSelectionDraft(null);
       else if (s.gradientEdit) s.cancelGradient();
+      else if (s.shapeDraft) s.setShapeDraft(null);
       else if (s.tool === "crop") s.setCropRect(null);
       else if (s.transformEdit) s.cancelTransform();
       break;
```

```diff
--- a/app/src/state/store.ts
+++ b/app/src/state/store.ts
@@ -13,10 +13,11 @@ import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } fr
 import type { AdjustEdit, SampleMode } from "./adjust-edit";
 import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
 import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
+import { DEFAULT_SHAPE, nextShapeKind, shapeSpec, type ShapeDraft, type ShapeOptions } from "../tools/shape-draft";
 import { DEFAULT_GRADIENT, gradientSpec, hasLine, type GradientEdit, type GradientOptions } from "./gradient-edit";
 import { BLACK, WHITE, hsbOf, hsbToRgb, quantized, sameColor, withRgb, type PaletteColor, type PickerHSB } from "../tools/color";
 
-export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper" | "gradient";
+export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper" | "gradient" | "shape";
 export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
 /** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
 export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
@@ -177,6 +178,15 @@ export interface EditorStore {
   /** Return or Apply: paints the pending gradient as one undo step ("Gradient" or "Gradient Mask"),
    * through the job worker on a large layer (`commitGradient`). */
   commitGradient(): void;
+  shapeOptions: ShapeOptions;
+  /** A shape being dragged out with the Shape tool; drawn on the overlay only, never in the document. */
+  shapeDraft: ShapeDraft | null;
+  /** New settings; a new kind drops a draft being drawn (ShapeControls.swift:9-12). */
+  setShapeOptions(patch: Partial<ShapeOptions>): void;
+  setShapeDraft(draft: ShapeDraft | null): void;
+  /** The release: the draft filled with the image's foreground colour on a new layer above the
+   * active one, one undo step named for its kind; a click makes nothing (`finishShape`). */
+  finishShape(): void;
   setSampleRing(ring: SampleRing | null): void;
   setEngine(engine: EngineClient): void;
   setJobs(jobs: JobClient): void;
@@ -257,7 +267,7 @@ export interface EditorStore {
 /** Commands that insert a layer or move one into a folder, and so make it active somewhere the
  * panel may not be showing. Each is followed by `revealActiveLayer`. */
 const REVEALING_COMMANDS: ReadonlySet<Command["type"]> = new Set<Command["type"]>([
-  "AddBlankLayer", "AddGroup", "GroupLayers", "PlaceLayer", "DuplicateLayer", "DuplicateLayerTo", "DuplicateLayerTransformed", "MergeLayers", "DeleteLayers", "DeleteLayer",
+  "AddBlankLayer", "AddShape", "AddGroup", "GroupLayers", "PlaceLayer", "DuplicateLayer", "DuplicateLayerTo", "DuplicateLayerTransformed", "MergeLayers", "DeleteLayers", "DeleteLayer",
 ]);
 
 /** How long after the last slider tick a colour adjustment's quick drag preview is replaced by
@@ -332,6 +342,22 @@ export const useEditor = create<EditorStore>((set, get) => ({
   selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
   palette: DEFAULT_PALETTE, colorPicker: null, pickerAt: null, sampleRing: null,
   gradientOptions: DEFAULT_GRADIENT, gradientEdit: null,
+  shapeOptions: DEFAULT_SHAPE, shapeDraft: null,
+  setShapeOptions: (patch) => {
+    const kindChanged = patch.kind !== undefined && patch.kind !== get().shapeOptions.kind;
+    set((s) => ({ shapeOptions: { ...s.shapeOptions, ...patch }, ...(kindChanged ? { shapeDraft: null } : {}) }));
+    if (kindChanged) get().repaintOverlay();
+  },
+  setShapeDraft: (shapeDraft) => { set({ shapeDraft }); get().repaintOverlay(); },
+  finishShape: () => {
+    const draft = get().shapeDraft; if (!draft) return;
+    set({ shapeDraft: null });
+    get().repaintOverlay();
+    const spec = shapeSpec(draft, get().shapeOptions.lineWidth);
+    if (!spec) return;
+    const { red, green, blue } = get().palette.foreground;
+    get().run({ type: "AddShape", shape: spec, color: [red, green, blue] });
+  },
   setGradientOptions: (patch) => { set((s) => ({ gradientOptions: { ...s.gradientOptions, ...patch } })); get().refreshGradient(); },
   beginGradient: (at) => {
     const { activeId, documents, gradientEdit } = get(); if (!activeId) return false;
@@ -604,7 +630,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
     const { activeId, documents, cropRect } = get();
     const doc = activeId ? documents[activeId] : null;
     const seeded = tool === "crop" ? (cropRect ?? (doc ? cropSeed(doc) : null)) : null;
-    set({ tool, cropRect: seeded, ...(tool !== get().tool ? { selectionDraft: null, outlineMove: null } : {}) });
+    set({ tool, cropRect: seeded, ...(tool !== get().tool ? { selectionDraft: null, outlineMove: null, shapeDraft: null } : {}) });
     get().invalidate();
   },
   setCropRect: (cropRect) => set({ cropRect }),
@@ -918,6 +944,7 @@ export const useEditor = create<EditorStore>((set, get) => ({
   cycleToolMode: () => {
     const { tool, selectionOptions: o } = get();
     if (tool === "gradient") get().setGradientOptions({ shape: get().gradientOptions.shape === "Linear" ? "Radial" : "Linear" });
+    if (tool === "shape") get().setShapeOptions({ kind: nextShapeKind(get().shapeOptions.kind) });
     if (tool === "marquee") get().setSelectionOptions({ marquee: o.marquee === "Rectangle" ? "Ellipse" : "Rectangle" });
     else if (tool === "lasso") get().setSelectionOptions({ lasso: o.lasso === "Freehand" ? "Polygonal" : "Freehand" });
   },
```

Create `app/src/tools/shape-draft.ts`:

```ts
import type { ShapeSpec } from "../engine/types";
import { dragBox, type P } from "./selection-draft";
import { snapped45 } from "../state/gradient-edit";

/** The Shape tool's kinds, in the order Shift+U and Tab step through them (`ShapeKind.allCases`). */
export type ShapeKind = "Rectangle" | "Ellipse" | "Line";
export const SHAPE_KINDS: ShapeKind[] = ["Rectangle", "Ellipse", "Line"];
/** The tool's settings (EditorSession.swift:236-240): the kind, a rectangle's corner radius and a
 * line's width, in document pixels; the bar's sliders reach 200 and 100, its fields 5000. */
export interface ShapeOptions { kind: ShapeKind; cornerRadius: number; lineWidth: number; }
export const DEFAULT_SHAPE: ShapeOptions = { kind: "Rectangle", cornerRadius: 0, lineWidth: 4 };
export const SHAPE_FIELD_MAX = 5000;

/** A shape being dragged out (`ShapeDraft`): its kind, where it began (whole pixels), its box, a line's
 * free end, and the corner radius fixed when the drag began (rectangles only). */
export interface ShapeDraft { kind: ShapeKind; anchor: P; rect: { x: number; y: number; width: number; height: number }; end: P | null; cornerRadius: number; }

/** Swift's `rounded()`: halves away from zero. */
const swiftRound = (v: number) => Math.sign(v) * Math.round(Math.abs(v));

export function nextShapeKind(kind: ShapeKind): ShapeKind { return SHAPE_KINDS[(SHAPE_KINDS.indexOf(kind) + 1) % SHAPE_KINDS.length]; }

/** A press: the draft starts at the pixel corner nearest `point` (`beginShape`). */
export function beginShape(point: P, options: ShapeOptions): ShapeDraft {
  const anchor = { x: swiftRound(point.x), y: swiftRound(point.y) };
  return { kind: options.kind, anchor, rect: { ...anchor, width: 0, height: 0 }, end: null, cornerRadius: options.kind === "Rectangle" ? options.cornerRadius : 0 };
}

/** A drag to `point` (`dragShape`): Shift squares a box, or holds a line to 45 degree steps; Alt grows
 * the box from its centre. A line's end is where the pointer is, not rounded. */
export function dragShape(draft: ShapeDraft, point: P, square: boolean, fromCenter: boolean): ShapeDraft {
  if (draft.kind === "Line") {
    const end = square ? snapped45(point, draft.anchor) : point;
    return { ...draft, end, rect: dragBox(draft.anchor, end, false, fromCenter) };
  }
  return { ...draft, rect: dragBox(draft.anchor, point, square, fromCenter) };
}

/** What finishing the draft makes (`finishShape`): the engine's shape, or null when its box is under a
 * pixel on a side (a click), as the Mac makes nothing then. A line's box is its ends grown by half its
 * width. */
export function shapeSpec(draft: ShapeDraft, lineWidth: number): ShapeSpec | null {
  if (draft.kind === "Line") {
    const end = draft.end ?? draft.anchor;
    if (Math.abs(end.x - draft.anchor.x) + lineWidth < 1 || Math.abs(end.y - draft.anchor.y) + lineWidth < 1) return null;
    return { kind: "Line", start: [draft.anchor.x, draft.anchor.y], end: [end.x, end.y], width: lineWidth };
  }
  if (draft.rect.width < 1 || draft.rect.height < 1) return null;
  return draft.kind === "Rectangle" ? { kind: "Rectangle", rect: draft.rect, cornerRadius: draft.cornerRadius } : { kind: "Ellipse", rect: draft.rect };
}
```

```diff
--- a/engine/src/ops/shape.rs
+++ b/engine/src/ops/shape.rs
@@ -128,11 +128,14 @@ pub fn shape_raster(spec: &ShapeSpec, color: [f64; 3]) -> Raster {
         }
     };
     let coverage = rasterize(&[contour], 0.0, 0.0, w, h, true);
+    // Each of the 256 coverages' premultiplied pixel, once; then a table lookup per pixel.
     let c = color.map(|v| v.clamp(0.0, 1.0) * 255.0);
-    let data = coverage.bytes().iter().flat_map(|&k| {
+    let table: Vec<[u8; 4]> = (0..=255u8).map(|k| {
         let a = k as f64 / 255.0;
         [(c[0] * a + 0.5) as u8, (c[1] * a + 0.5) as u8, (c[2] * a + 0.5) as u8, k]
     }).collect();
+    let mut data = vec![0u8; w as usize * h as usize * 4];
+    for (px, &k) in data.chunks_exact_mut(4).zip(coverage.bytes()) { px.copy_from_slice(&table[k as usize]); }
     Raster::from_premultiplied(w, h, data)
 }
```


- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine --test shapes` still passes (9 tests). `pnpm wasm:dev`; `pnpm test`: 207 (+8); `pnpm build`; `pnpm e2e`: 155 passed, 12 skipped (+3). Timings (release wasm, Edge, `-g "shapes and fills"`): a shape over the whole canvas, on the UI thread as an import is (ruling OQ11): at 24 MP the rectangle 706 ms (the first, as the wasm memory grows to hold it) and the ellipse 353 ms (budget 1000), at 100 MP 1318 and 937 ms (budget 2000); the frames that first draw them 78-174 ms and 328-361 ms (a new layer's upload, as for any import). Natively 98-119 ms and 423-431 ms.

- [ ] **Step 5: Prove it bites**

(1) In `shapeSpec`, drop the check for a box under a pixel: "makes nothing from a click, or from a box under a pixel on a side" fails (measured). Restore. (2) In `setTool`, keep the shape draft: "is dropped by Escape, a tool change or a new kind..." fails at the tool change (measured). Restore.

- [ ] **Step 6: Commit**

```
git add -- app/src/canvas/shape-tool.ts app/src/panels/ShapeOptions.tsx app/src/tools/shape-draft.ts app/tests/e2e/shape-tool.spec.ts app/tests/unit/shape-draft.test.ts
git commit -m "feat(app): the Shape tool, rectangles, ellipses and lines dragged onto new layers" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- app/src/App.tsx app/src/canvas/CanvasView.tsx app/src/canvas/overlay.ts app/src/canvas/shape-tool.ts app/src/panels/ShapeOptions.tsx app/src/panels/ToolRail.tsx app/src/panels/tool-icons.tsx app/src/shortcuts/keymap.ts app/src/shortcuts/useShortcuts.ts app/src/state/store.ts app/src/tools/shape-draft.ts app/tests/e2e/perf-4b1.spec.ts app/tests/e2e/shape-tool.spec.ts app/tests/unit/keymap.test.ts app/tests/unit/shape-draft.test.ts app/tests/unit/tool-rail.test.tsx engine/src/ops/shape.rs
```


---

### Task 17: Docs and version 0.5.0

**Files:**
- Modify: `README.md` (a Phase 4b-1 section and its shortcuts; Phase 4a's Delete-on-a-mask line and its out-of-scope note), `package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`, `Cargo.lock`, `app/tests/e2e/smoke.spec.ts`

- [ ] **Step 1: README, and version 0.5.0**

In `package.json`, `src-tauri/tauri.conf.json`, the workspace `Cargo.toml` (then let cargo rewrite `Cargo.lock`: `cargo metadata --format-version 1 *> $env:TEMP\meta.txt`; the three workspace packages move to 0.5.0) and the smoke test's literal:

```diff
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -345,7 +345,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-engine"
-version = "0.4.1"
+version = "0.5.0"
 dependencies = [
  "i_overlay",
  "image",
@@ -359,7 +359,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-engine-wasm"
-version = "0.4.1"
+version = "0.5.0"
 dependencies = [
  "compositor-engine",
  "console_error_panic_hook",
@@ -371,7 +371,7 @@ dependencies = [
 
 [[package]]
 name = "compositor-shell"
-version = "0.4.1"
+version = "0.5.0"
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
-version = "0.4.1"
+version = "0.5.0"
 edition = "2021"
 license = "MIT"
```

```diff
--- a/README.md
+++ b/README.md
@@ -147,7 +147,8 @@ Photoshop.
   select its black areas, as on the Mac; Ctrl+Shift adds and Ctrl+Alt subtracts.
 - With a selection: adjustments, filters and Invert change only what is selected (a blur still
   grows the layer where the selection reaches), and Levels and Curves show the histogram of the
-  selected pixels. Delete clears the selected pixels, or fills a targeted mask white. Add Mask
+  selected pixels. Delete clears the selected pixels, or fills a targeted mask with its background
+  colour (white unless the palette was swapped). Add Mask
   hides the selection (Add Mask (Hide All) shows only it) and uses it up. The Crop tool starts at
   the selection's bounds. Adjustment layers ignore the selection.
 - An empty selection (after Subtract or Contract) says so in the options bar, and every edit
@@ -155,8 +156,7 @@ Photoshop.
 - Selections are part of undo and, as on the Mac, are never saved in the project. Crop, Canvas
   Size and Image Size drop the selection; Flip Canvas mirrors it.
 - Note: dragging a Marquee or an outline past the window's edge does not scroll the view yet.
-- Note: object selection, Select Subject, fills, the clipboard and the brushes are not in this
-  phase.
+- Note: object selection, Select Subject, the clipboard and the brushes are not in this phase.
 
 ### Select menu shortcuts
 
@@ -171,6 +171,50 @@ Photoshop.
 | Nudge the selection (selection tools) | Arrow keys (Shift = 10 px) |
 | Clear the selected pixels | Delete |
 
+## Phase 4b-1: colour, fills, gradients and shapes
+
+- The palette at the foot of the tool rail: the foreground and background colours; X swaps them
+  and D restores black over white. With a layer's mask targeted they are black and white, and a
+  click on a swatch asks which ("Black - Hide" or "White - Reveal").
+- The colour picker, opened from a swatch: a saturation and brightness field, a hue strip, R, G, B
+  and hex, OK (Enter) and Cancel (Escape). It floats, opens where it was last left, and while it
+  is open a click or drag on the canvas samples the colour under the pointer, with a ring showing
+  the sampled colour over the one before.
+- Eyedropper (I): a click or drag on the canvas sets the foreground colour from what the canvas
+  shows. Alt with the Gradient tool does the same.
+- Edit > Fill with Foreground Color (Alt+Backspace) and Fill with Background Color
+  (Ctrl+Backspace): the selection, or the whole layer, which grows to cover the canvas as on the
+  Mac; on a targeted mask its black or white.
+- Gradient (G): drag a line; its ends can then be dragged (Shift holds 45 degrees), Enter or Apply
+  paints it, Escape or Cancel drops it, and the first Undo discards it. Linear or Radial (Tab),
+  Foreground to Background or to Transparent, Reverse, and Opacity (the digit keys set it). It
+  previews from a reduced copy while dragged (a patch at full size inside a small selection) and
+  is applied to the full layer; switching tool or layer applies it first.
+- Shape (U; Shift+U or Tab steps Rectangle, Ellipse and Line): drag to draw the shape in the
+  foreground colour on a new layer above the active one (Shift squares it or holds a line to 45
+  degrees, Alt draws from the centre). Rectangles take a corner Radius, lines a Width. The layer
+  keeps the Mac's shape record, so Compositor for Mac redraws it crisply when it is scaled there;
+  this app scales its pixels.
+- A new Gradient Map, as a layer or from the Image menu, starts from the foreground to the
+  background colour, and its two ends open the colour picker, which previews them live.
+- Large layers (over 4 megapixels) are edited, and their Levels and Curves histograms read, off the
+  interface thread: "Working..." shows meanwhile. Edits inside a selection upload only the pixels
+  they change, and undo keeps the last 100 steps, fewer once they hold more than 256 MB.
+- Note: a shape is drawn as the Mac draws it (Core Graphics' anti-aliasing is approximated by exact
+  area coverage); a scaled shape layer is not redrawn here.
+- Note: the colour picker works in sRGB, 8 bits a channel, as the Mac's does.
+
+### Colour and tool shortcuts
+
+| Action | Shortcut |
+| --- | --- |
+| Eyedropper / Gradient / Shape | I / G / U |
+| Swap colours / default colours | X / D |
+| Fill with the foreground / background colour | Alt+Backspace / Ctrl+Backspace |
+| Gradient: Linear or Radial; Shape: next kind | Tab (Shape also Shift+U) |
+| Gradient opacity | 1-9 for 10-90 %, 0 for 100 % |
+| Apply / cancel a pending gradient | Enter / Escape |
+
 ## Prerequisites
 
 - Rust 1.95 with the `wasm32-unknown-unknown` target
```

```diff
--- a/package.json
+++ b/package.json
@@ -1,7 +1,7 @@
 {
   "name": "compositor-windows",
   "private": true,
-  "version": "0.4.1",
+  "version": "0.5.0",
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
-  "version": "0.4.1",
+  "version": "0.5.0",
   "identifier": "com.compositor.windows",
   "build": {
     "frontendDist": "../app/dist",
```


```diff
--- a/app/tests/e2e/smoke.spec.ts
+++ b/app/tests/e2e/smoke.spec.ts
@@ -2,7 +2,7 @@ import { test, expect } from "@playwright/test";
 
 test("engine loads in the browser", async ({ page }) => {
   await page.goto("/");
-  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.4.1");
+  await expect(page.getByTestId("engine-ready")).toContainText("Compositor engine 0.5.0");
   const ids = await page.evaluate(() => {
     const api = (window as unknown as { __compositor: { engine: { newDocument(w: number, h: number, e: boolean): string; documentIds(): string[] } } }).__compositor;
     api.engine.newDocument(10, 10, true);
```


Do NOT run `pnpm build:portable`.

- [ ] **Step 2: Full verification (foreground)**

`cargo test` (the workspace), `cargo test -p compositor-engine`, `pnpm wasm`, `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4b1.spec.ts` (then `$env:PERF = ""`), `pnpm wasm:dev`, `pnpm test`, `pnpm build`, `pnpm e2e`. Report the totals against Task 1's baseline. The scratch copy measured: `compositor-engine` 509 passed and 9 ignored over 68 test binaries (459 and 4 over 60 before: Tasks 1-11 added 50 and 5 ignored timing tests: 1, 7, 7, 0, 5, 0, 2, 12, 5, 9, 2); the workspace 516 passed and 9 ignored over 73; vitest 207 in 36 files (148 before: Tasks 4, 6, 7, 11, 12, 13, 14, 15, 16 added 4, 10, 5, 8, 7, 1, 4, 12, 8); `pnpm build` clean; e2e 155 passed and 12 skipped (131 and 4 before: Tasks 2, 4, 6, 7, 9, 12, 13, 14, 15, 16 added 1, 2, 2, 1, 2, 5, 1, 3, 4, 3, and eight skipped timing tests); the perf suite's eight tests passed every budget; the release wasm 2,913,123 bytes (2,571,151 before: +341,972, +13.3 %).

- [ ] **Step 3: Commit**

```
git commit -m "docs: Phase 4b-1 colour, fills, gradients and shapes in the README, and 0.5.0" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- Cargo.lock Cargo.toml README.md app/tests/e2e/smoke.spec.ts package.json src-tauri/tauri.conf.json
```


---

## Self-review notes

**Scope coverage** (the brief's 4b-1 list, point by point):
- History cap 100 entries / 256 MiB, state token, entry id: Task 2 (rulings OQ1, OQ2).
- Per-layer changed rectangles, `texSubImage2D` for layers and masks, patch previews: Tasks 3, 4, 9 (OQ3, OQ4, OQ8).
- Job worker: effects images reduced then full up to about 24 MP (Task 7, OQ6), the Levels histogram (Task 6), destructive adjustment and filter commits (Task 6), Fill and Gradient commits (Tasks 14, 15), installed only if the layer is unchanged (Task 5's stamp, OQ5).
- Palette: X swap, D reset, a mask's black and white: Tasks 11, 12.
- Colour picker panel: the HSB field, hue strip, RGB, hex, canvas sampling while open: Task 12 (OQ13, OQ14).
- Eyedropper (I): Task 13.
- Fill (Alt+Backspace foreground, Ctrl+Backspace background, the Delete variants): Task 14 (OQ15), on Task 8's engine.
- Gradient tool: Linear / Radial, Foreground to Background / to Transparent, Reverse, opacity, the pending line with draggable ends, Shift 45 degrees, mask gradients, Tab, digits: Task 15 on Tasks 8 and 9 (OQ7, OQ8, OQ10).
- Shape tool: rectangle with corner radius, ellipse, line; a new layer; the Mac's `shape` record; no redraw on scale: Tasks 10, 16 (OQ11, OQ19).
- New Gradient Map layers (and the destructive Gradient Map) from the palette: Task 12.
- Tool rail icons from lucide-static (the pipette) and drawn in its style where Lucide has none (the gradient's half-filled square, the shape's square on a circle), and the colour swatches at the rail's foot: Tasks 12, 13, 15, 16 (OQ16).
- Out, as the brief says: the clipboard, moving pixels, Transform Selection (4b-2), brushes (4c), tiled storage.
- LL-073: every interactive path has a release timing at 24 and 100 MP with a budget the perf suite asserts (partial uploads, jobs, effects images, history, gradient previews, the Eyedropper, the Gradient tool, shapes and fills); the paths without one are listed with the reason (OQ20).
- Mac probes: added to `mac_probes.rs` with the README's hand instructions (Task 1); where the pins land is ruled (OQ17).
- Version 0.5.0 and the README: Task 17.

**LL-067 / LL-068 pass: could each assertion fail?** Every Step 5 names a production change and the test it breaks; all 32 were run on the scratch copy (on the final code, then restored with the backup-and-touch rule) and failed as each says, bar the second half of Task 4 (1) (the fit e2e), which was run when that task was written; Task 2's Step 2 e2e expectation is reasoned from the old depth check, not run. Beyond Step 5, per file:
- `history_cap.rs`: each case sets the limits it crosses (2 entries and 0 bytes, 3 entries, 100 entries with a byte limit sized from its layer) so a handful of pushes cross them; the shared-raster case holds one buffer in several entries and asserts it is counted once, which a per-entry sum breaks; the saved-state case saves, trims an older entry off the front under a three-entry cap, and still undoes exactly back to the saved state.
- `changed_rects.rs`: rectangles are computed from the selection in the test (a pixel for the clip and a pixel for sampling, each side), never pasted; undo and redo are checked in both directions; a grid that changes size, a rename, an unseen revision and a forgotten change are each asserted whole or empty as the ruling says; seeded halvings are compared with halving from scratch at levels 1 and 2 on odd sizes.
- `partial-upload.test.ts` / `.spec.ts`: `levelRect` against an independent block count on a 37 x 29 layer, rectangles touching both edges; uploads counted by wrapping WebGL, and the partial picture compared with a whole upload of the same document byte for byte, at 1:1 and at fit.
- `jobs.rs`, `jobs.test.ts`, `store-jobs.test.ts`, `jobs.spec.ts`, `fill.spec.ts`: every result through a job is compared with the same edit in place, byte for byte; the stale-layer refusal is asserted with the document and depth unchanged; the store's threshold is tested at the threshold and one past it.
- `effects-images.test.ts`, `effects-worker.spec.ts`: the order of requests (reduced, then full, once each) and the full image's equality with the CPU compositor's.
- `raster_edits.rs`: every value from the formula in the test (the ramp's `round(255 x / 100)`, 127.5 rounding up, the radial rim), asymmetric fixtures (a 20 x 10 layer at (30, 15) on 100 x 40 under a half-black mask) so a mirrored or ungrown grid, or a mask that does not follow, shows.
- `gradient_preview.rs`, `gradient-preview.spec.ts`: the reduced preview's alpha at the layer's edge from the gradient's own position (a misplaced halving shifts it); the patch limit straddled (720 x 720 against 730 x 720); the patch compared with the commit byte for byte, and on the GPU with applying the gradient.
- `shapes.rs`: the Mac test's pixels, and every pixel of three shapes against an independent 32 x 32 area count (a flattening or cap error fails it: pointed caps are 183 levels off); the line's end pixel's 228 is 0.5 + pi / 8 of 255, which a square or butt cap does not give.
- `sample_color.rs`: the translucent pixel's channels are chosen so each rounding differs from truncation (141.67, 52.42) and one channel exceeds alpha (the clamp). Its off-canvas points pass without the explicit bounds check too (a region off the canvas composites transparent; measured): they pin the Mac's rule against a later compositor change rather than today's code.
- `color.test.ts`, `palette-store.test.ts`, `picker-store.test.ts`: the Mac's own hex, HSB and sampling numbers; every mask rule has its image-side opposite in the same test.
- `gradient-store.test.ts`, `gradient-tool.spec.ts`: previews are checked by their full request, commits by their full command; the e2e's pixels come from the formula at the edit's actual ends (read from the store, not from where the pointer was aimed).
- `shape-draft.test.ts`, `shape-tool.spec.ts`: the Mac test's boxes and pixels; the e2e reads the overlay while dragging and after.
- `perf-4b1.spec.ts`: each budget is asserted; each test also asserts what it measured (uploads partial, undo depths, the image kinds drawn), so a run that measured the wrong path fails.

**Not covered by a test (disclosed):** the Mac's own renders of shapes and gradients (Task 1's probes await the user's exports; OQ17); the worker's project-budget gap (OQ20); a gradient pending on one document while another is closed from its tab (the store applies or drops it by the rule in Task 15's code, not tested end to end); the picker's panel position across a window resize.

**Placeholder scan:** every step names its files and every code block is copied from the scratch copy's tags by `gen.py`, which also checks that each file each tag changed appears exactly once and that the plan is ASCII; no "TBD", "similar to" or unwritten helper.

**Type consistency:** the engine's serde names are the TypeScript types' (`GradientSpec` with `start` / `end` as `[x, y]`, `ShapeSpec` tagged by `kind` with `cornerRadius`, the commands' `type` tags, `PreviewRequest`'s `preview: "Gradient"`, `DocumentState.undoEntryId`), and `pnpm build` compiled the three TypeScript programs against the generated wasm types at every app task.

**Rulings made in advance:** OQ1 to OQ22 above.
