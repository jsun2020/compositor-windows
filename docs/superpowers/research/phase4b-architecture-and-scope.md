# Phase 4b research: architecture and scope (colour, fills, clipboard, and the groundwork)

Date: 2026-09-28. Research only; no product code changed. Branch `phase4b` at a9937bd (0.4.1).

Oracle: `Compositor-1.2.10`, cited as `MAC <file>:<lines>` (sources under `Compositor/`, tests under
`CompositorTests/`). Port: cited as `PORT <path>:<lines>`. Inputs: spec sections 3, 4, 6 and 10;
`research/phase4-mac-1.2.10-selection-and-retouching.md` (1.13, 1.14, 3.4-3.6, 5); the 4a and 3.5c rulings
(OQ6); Claude-LL LL-071 and LL-073.

## 0. Summary

- **UI-thread blocks (section 1, release wasm, 100 MP):**
  - Levels histogram 6.9 s; Levels commit 8.4 s; Hue/Saturation commit 12.1 s;
  - full-size effects image 26.7 s; save encode 5.5 s; open decode 3.2 s;
  - a 4b drag frame done as edits work today: 1.3-2.5 s (0.3-0.6 s at 24 MP), plus 7-28 s on a styled layer;
  - with no history cap, six whole-layer edits reached the 4 GiB wasm ceiling (4261 MB).
- **Off the UI thread (section 2).** Recommended: a **job worker**, a second instance of the same wasm module
  in a Web Worker.
  - It runs read-only or compute-then-install jobs on transferred inputs; edits and history stay on the main
    thread.
  - Main cost: one copy out of wasm memory per full-size input (63 ms at 24 MP, 290 ms at 100 MP).
  - SharedArrayBuffer stays the upgrade path.
- **Groundwork (section 3).**
  - The Mac's history cap (100 entries, 256 MiB held only by history), with a state token and an entry id so
    the app's Alt-drag cancel survives the cap.
  - A revision lineage for partial uploads, patch previews, and incremental halving. At fit zoom the CPU
    halving chain, not the upload, is the cost.
- **Split (section 4).**
  - 4b-1: groundwork, palette, picker, Eyedropper, Fill, Gradient, Shape.
  - 4b-2: clipboard, moving pixels, Transform Selection.
- **The rest.** Section 5 gives the Mac behaviours and tests to port; the gradient's straight-versus-
  premultiplied question is moot (5.5). Section 6 lists probes, section 7 three product questions.

## 1. Measurements: where the UI thread blocks, and for how long

**Method.** A temporary Playwright spec (like `app/tests/e2e/perf-spatial.spec.ts`, since deleted) timed calls
with `performance.now()` inside `page.evaluate`, on the release wasm (`pnpm wasm`), PERF=1, one size per fresh
page. Layers came from Canvas Size's fill plus 25% Gaussian Add Noise, which is PNG's worst case, so save and
open are pessimistic. GPU frames end with a 1x1 `readPixels`, so uploads finish inside the timing. Runs are
single, except the 100 MP render rows, which ran twice.

**Machine.** i5-6200U (2 cores, 4 threads), 32 GB RAM, Intel HD Graphics 520, viewport 1440x900 (fit zoom
0.173 at 24 MP, 0.072 at 100 MP). The browser was installed **Edge 154, headless** (`channel: "msedge"`),
which runs WebGL on ANGLE/D3D11 on the real GPU, as WebView2 does. Playwright's bundled Chromium falls back to
SwiftShader (an unchanged frame took 160-280 ms), so **GPU timings must use the Edge channel**. Engine rows
agreed within about 10% between the two browsers.

### 1.1 Engine calls on the UI thread today (ms)

| Operation | 24 MP (6000x4000) | 100 MP (10000x10000) |
|---|---|---|
| Levels / Curves histogram when the panel opens (PORT store.ts:467; engine.rs:515) | 1711 | 6930 |
| Levels commit (`ApplyAdjustment`) | 1984 | 8371 |
| Hue/Saturation commit | 2908 | 12101 |
| Add Noise commit (a filter) | 5483 | 21685 |
| Cheapest whole-layer replacement (`InvertPixels`: clone, one pass, history push) | 229 | 976 |
| Levels drag tick, the first after an edit (512 cap, includes the halving chain) | 326 | 1300 |
| Levels settled preview (4096 cap) | 531 | 531 |
| Hue/Saturation settled preview (4096 cap) | 862 | 912 |
| Gaussian 10 filter preview tick (2048 cap), first then second | 541 / 336 | 355 / 244 |
| Halving chain to level 3 after an edit (`Raster::halved`, PORT raster.rs:95) | 220 | 984 |
| Save: engine PNG encode (85.7 MB / 357.1 MB of PNG) | 1054 | 5488 |
| Open: engine PNG decode | 689 | 3195 |
| Undo, redo (snapshot swap) | 0 | 0 |
| Wasm memory at the end of the run | 1024 MB | **4261 MB (the 4 GiB ceiling)** |

- **The 100 MP run hit the ceiling.** It made six whole-layer edits (noise, Levels, Hue/Saturation, invert,
  undo, redo) plus previews. History has no cap (PORT history.rs:5-10), so each edit kept 400 MB, and the
  next would have failed to allocate. The cap is a correctness fix at this size.
- **Open and save** time the engine only. Tauri IPC and disk time were not measured, because the Playwright
  bridge is an in-memory mock.

### 1.2 Frames and uploads (ms, Edge, real GPU)

| Frame | 24 MP | 100 MP (run 1 / run 2) |
|---|---|---|
| Nothing changed, fit | 7 | 4 / 4 |
| After a whole-layer edit, fit (halving + upload + draw) | 370, 232 | 1340, 1030 / 1490, 1526 |
| ... the halving chain alone | 224 (level 2) | 1044 / 1037-1103 (level 3) |
| ... upload and draw once the halving is done | 9 | 369 (outlier) / 8, 11, 14 |
| Nothing changed, 1:1 | 5 | 6 / 5 |
| After a whole-layer edit, 1:1 (full-size `texImage2D`) | 59, 60 | 379, 468 / 298, 489 |
| memcpy of the whole layer in JS (reference) | 58 | 252 / 264 |
| `texSubImage2D` 256 / 512 / 1024 / 2048 px square, from a view on a layer-wide buffer | 2.0 / 2.3 / 3.7 / 13.4 | 1.4 / 2.0 / 3.4 / 12.5 (run 2: 1.8 / 2.0 / 3.8 / 17.2) |

- **Fit zoom:** an edit's cost is the CPU halving chain, rebuilt before the reduced level uploads (PORT
  gl-renderer.ts:102, :127).
- **1:1:** the whole raster re-uploads in 2048-px chunks (PORT layer-textures.ts:54-83), even for a tiny edit.
- **Sub-rectangles:** uploading straight from a view on wasm memory (`UNPACK_ROW_LENGTH` and `UNPACK_SKIP_*`,
  no copy) takes 2-4 ms up to 1024x1024.

### 1.3 Effects images (OQ6) (ms)

| Operation | 24 MP | 100 MP |
|---|---|---|
| Reduced effects image, 1536 px long side, all six effects, radii unscaled (upper bound) | 932 (1536x1024) | 1323 (1536x1536) |
| Full-size effects image (first texture) | 7104 | 26683 |
| Effects texture at the fit level, image kept (halving the padded image) | 269 | 1275 |
| After a pixel edit: a new image plus halving | 7204 | 27950 |

- **Port today.** Every pixel edit to a styled layer, and every frame of a mask drag, remakes the full-size
  image on the UI thread (PORT gl-renderer.ts:120, :126; engine.rs:483-493).
- **Mac.** It never shows a full-size effects image on the canvas. It renders at most 1536 px on one
  background queue and keeps showing the last result until the new one lands (MAC
  Rendering/EffectsPreviewCache.swift:3-4, :46, :51, :95-112).

### 1.4 Moving a snapshot to a worker (ms)

| Step | 24 MP (96 MB) | 100 MP (400 MB) |
|---|---|---|
| Copy the layer out of wasm memory (`slice`), main thread | 63 | 290 |
| `postMessage` with transfer, main thread | 0 | 0 |
| Copy into a fresh buffer in the worker (stands in for its wasm heap) | 67 | 283 |
| `postMessage` by structured clone: main thread (worker side) | 66 (130) | 277 (560) |

Wasm memory's ArrayBuffer cannot be transferred or detached, and it cannot be shared without
SharedArrayBuffer. So every job input costs one main-thread copy, about 0.65-0.75 ms/MB, and that copy is
then transferred for free. Always transfer: a structured clone adds a second main-thread copy.

### 1.5 What a 4b drag frame would cost today

A gradient drag covers the whole canvas, and a pixel drag covers the source plus the target. A Transform
Selection drag only transforms a small floating layer, which is already cheap.

If each frame of a gradient or pixel drag replaced the whole layer as edits do now, the cost per frame would
be the engine's at least 229 ms (24 MP) or 976 ms (100 MP), plus the next frame:

| Cost per frame | 24 MP | 100 MP |
|---|---|---|
| At fit zoom (engine + halving + upload) | 0.46-0.6 s | 2.0-2.5 s |
| At 1:1 (engine + full upload) | 0.29 s | 1.3-1.5 s |
| On a styled layer, add | 7.2 s | 28 s |

The budget is 16-33 ms a frame. `InvertPixels` is a lower bound for this, and each frame would also push a
whole-raster history entry.

### 1.6 Kernels that are slow in themselves

- **Histogram:** 71 ns a pixel (PORT engine/src/adjust/levels.rs:29-41), dividing in f64 per channel.
- **Levels apply:** 83 ns a pixel (:13-27), dividing in f32 per channel.
- **Fix:** an alpha-255 fast path (integer bins, a byte table) should cut both several-fold on opaque photos.
  This is an estimate that 4b-1 should measure.

## 2. Off-UI-thread options for this codebase

### 2.1 What the app depends on synchronously today

- **Direct engine calls: 48 in 10 files** (`engine.<method>(` under `app/src`, excluding `engine/client.ts`):
  store.ts 19, gl-renderer.ts 8, files.ts 6, layers.ts 5, JpegExportSheet.tsx 3, adjust-textures.ts 2,
  CanvasView.tsx 2, and one each in App.tsx, cpu-renderer.ts and LayersList.tsx.
- **Store actions: about 50 `run(` sites, 134 action calls in all.** They assume the command and the
  refreshed state land in the same tick:
  - `run` returns a boolean that callers branch on (PORT store.ts:261-284, :545);
  - `beginTransform` reads the duplicate it just made (:368-381);
  - `cancelTransform` reads `undoDepth` (:413-416);
  - `beginAdjust` computes the histogram inline (:467);
  - pointerdown asks `selectionContains` (CanvasView.tsx:393).
- **Every frame, the renderer reads:**
  - the plan: `renderPlan`, `spatialGrid`/`spatialSpan` and `spatialBlur` (gl-renderer.ts:137, :63-68,
    :358), plus the LUT tables (adjust-textures.ts:20, :34);
  - the pixels: `layerPixels`, `drawPixels` and `maskPixels` (gl-renderer.ts:126-127, :168). These are views on
    wasm memory, valid until the next engine call (client.ts:48-92; engine-wasm lib.rs:173-183).
- **Tests:** 464 synchronous `api.engine.*` / `__compositor` uses in the e2e specs, and 7 unit-test files that
  wire an engine into the store.

### 2.2 Option (a): the whole engine in a worker, with an async client

- **Cost.**
  - All the calls and actions above become async, and gestures need queueing, because a pointerdown cannot
    await before its pointermoves arrive.
  - Each texture upload becomes a copy out of the worker (63 ms at 24 MP, 290 ms at 100 MP per full-size layer
    at 1:1), where today it is a zero-copy view.
  - The renderer must draw from a cached plan.
  - The 464 e2e uses and the unit tests need rewriting.
- **Verdict:** not for 4b. During a long job the engine still answers nothing, so the canvas freezes unless
  plans and pixels are cached. This is the largest rewrite, for a benefit (b) mostly delivers.

### 2.3 Option (b): a job worker with its own engine (recommended)

**Design.**
- `app/src/engine/job-worker.ts` instantiates the same wasm module. The main thread posts the compiled
  `WebAssembly.Module` (it is cloneable).
- A `JobClient` sends `{ id, kind, inputs, settings }`. Inputs are transferred copies: a layer at some
  halving level, a mask, a selection coverage raster, effects JSON.
- One job runs at a time and the newest wins; superseded queued jobs are dropped. A cancel is checked at a
  job's start and end, as on the Mac (MAC EffectsPreviewCache.swift:101-110).
- Results return transferred. They either fill a display cache (effects images, histograms), or install
  through a guarded `ReplaceLayerPixels { id, expect_revision, raster }`, refused if the revision moved. That
  is the Mac's own check before it installs a detached result (MAC Document/SelectionEdits.swift:108-115).

**First clients.**
1. **Reduced display effects images (OQ6).** The input is the smallest memoized level of at least 1536 px, or
   the full raster (290 ms at 100 MP). The job takes about 0.9-1.3 s off-thread, with the last image kept
   meanwhile.
2. **The Levels/Curves histogram.** The panel opens at once, and the histogram follows 1.7-6.9 s later.
3. **Destructive adjustment and filter commits.** The UI thread pays two copies (63 + 63 ms at 24 MP,
   290 + 290 ms at 100 MP) instead of 2-12 s. The document is busy meanwhile, as with the Mac's
   `isProjectBusy` (EditorSession+Brush.swift:155-156).
4. **Later:** settled previews, save encode, and open decode.

**Cost.**
- One main-thread copy per full-size input (0.65-0.75 ms/MB).
- A second wasm heap. Wasm memory never shrinks, so respawn the worker after a job that grew it past about
  1 GiB.
- A task of protocol code.
- A spike: module worker plus wasm under the packaged CSP (`script-src 'self' 'wasm-unsafe-eval'`, PORT
  src-tauri/tauri.conf.json).

**Defers.**
- Edits, which stay on the main thread; they are cheap once drags use previews and patches (section 3).
- Zero-copy transfer and parallel kernels.
- Mid-job cancellation (terminate the worker instead).

### 2.4 Option (c): SharedArrayBuffer and wasm threads

- **Isolation is possible.**
  - Tauri 2 adds configured headers to every custom-protocol response (`app.security.headers`) and documents
    COOP `same-origin` plus COEP `require-corp` for exactly this (tauri-utils 2.9.3 src/config.rs:2716-2770,
    :2955-2958; the repo uses Tauri 2.11.6).
  - Dev and Playwright need the same pair in `vite.config.ts` (`server.headers`, `preview.headers`).
  - The app loads nothing cross-origin, so COEP blocks nothing.
  - A spike must still confirm `self.crossOriginIsolated`, IPC and dialogs in the portable build.
- **Costs.**
  - Wasm atomics need nightly `-Z build-std`, against the spec's pinned stable 1.95.
  - `Engine` keeps its sessions in one `HashMap` behind `&mut self`, so threads need locking.
  - `Atomics.wait` is barred on the main thread.
- **Gain:** zero-copy transfer, and at most about 2x on this 2-core machine.
- **Verdict:** the upgrade path, keeping the job protocol and changing only the transport.

### 2.5 Option (d): chunked work with yielding on the main thread

- **Cost.** Every heavy kernel becomes a resumable state machine: the histogram, the colour passes, the effects
  blurs and the PNG codec. The document is frozen, or the job restarted, on an edit.
- **Why not.** All the CPU time stays on the UI thread; at a 50% duty cycle a 27 s effects image takes about a
  minute.
- **Verdict:** not the mechanism. Worker jobs can post progress at row boundaries instead.

### 2.6 Recommendation

Adopt (b) in 4b-1, with section 3's patch previews and partial uploads, the fast paths of 1.6, and the
history cap. The UI thread's worst blocks on 4b's interactive paths then drop to the snapshot copies (at most
290 ms at 100 MP) plus one halving derivation per commit. That fits spec section 10: profiling now demands the
worker, but not yet shared memory.

## 3. Groundwork designs

### 3.1 History cap (Mac: 100 entries, 256 MiB)

**The Mac** (MAC Document/DocumentHistory.swift):
- **When it trims.** After every `end`, `undo` and `redo` (:69, :76, :84).
- **What it drops.** While there are more than 100 entries, or the retained bytes exceed 256 MiB, it drops the
  oldest undo entry, then the farthest redo entry (:28, :112-118).
- **What counts.** `bytesPerRow * height` of each distinct image (layer, mask, thumbnail) that any entry
  reaches but the current document does not, counted once each (:88-110).
- **Even the entry just made can go** (HistoryTests `historyBoundsEntriesAndUniqueRetainedPixels`,
  :139-158).
- **Brush commits** build whole images (MAC BrushStroke.swift:869-901), so they count at full size.

**Consequences.** A 24 MP layer keeps about two undoable pixel edits; a layer over 256 MiB (100 MP) keeps none.
The port needs a cap regardless (1.1).

**Port design** (PORT engine/src/history.rs; engine.rs:275-288):
- **Entry ids.** Entries become `Entry { id: u64, name, doc: Document, state: u64 }`, with `id` never reused.
- **Saved state.** `saved_depth` becomes a state token, as the Mac's revision UUID (DocumentHistory.swift:
  20-21, :38), so trimming the front cannot corrupt `is_modified`.
- **`trim(&current)`.**
  1. Build a `HashMap<*const u8, (bytes, count)>` over every Raster and GrayRaster the entries reach, keyed by
     `Arc::as_ptr` (PORT raster.rs:72 already compares pointers).
  2. Subtract the buffers the current document holds.
  3. Drop entries front-first, decrementing the counts: O(layers) per removal.
  4. Clear the memoized halving caches of buffers only history holds, since they are not counted.
- **Undo entry id.** `DocumentState` gains `undo_entry_id` (PORT engine.rs:55, :169). At the cap a
  `DuplicateLayer` push trims the oldest entry, so the depth stays at 100. The Alt-drag cancel's
  `depth === undoDepthBefore + 1` check (PORT store.ts:414-415) would then refuse and strand the copy.
- **Other budgets.** `revert` does not trim. The effects cache (512 MB, PORT effects/mod.rs:156), the
  selection clips and the worker keep their own budgets.
- **Tests.** Port the HistoryTests case. Add:
  - 101 renames keep 100 entries;
  - a shared raster is counted once;
  - an over-limit edit applies, with `can_undo` false;
  - `is_modified` stays correct after trimming;
  - the cancel check at the cap.
- **Gate (LL-073).** 20 edits at 100 MP stay under 2.5 GB of wasm memory; a trim with 100 entries and 1,000
  layers takes under 5 ms.

### 3.2 Partial texture uploads, patch previews, incremental halving

`texSubImage2D` alone fixes only 1:1 (1.2). At fit zoom the cost is the halving chain. During a drag it is the
whole-raster copy every edit makes, since a raster is one `Arc<Vec<u8>>` (PORT raster.rs:9-20), and the
whole-raster preview (PORT preview.rs:49; engine.rs:232-253).

**Engine.**
- **Revision lineage.**
  - Each session keeps a bounded ring of `(layer, Pixels | Mask, from_rev, to_rev, rect)`. `rect` is in the new
    raster's grid; `None` means the whole raster, or a resize.
  - `edit()` records one entry per changed buffer (PORT engine.rs:77-95 already finds them). Commands that
    know their rect report it; undo and redo record the undone entry's rect.
  - `Engine::pixels_delta(doc, layer, from_rev) -> Option<IRect>` and `mask_delta` return the union since
    `from_rev`.
  - It is a query because the renderer keys textures by revision (PORT gl-renderer.ts:120) and the store
    ignores `Dirty` (PORT command.rs:141-150; store.ts:279). A query survives skipped frames and undo.
- **Incremental halving.** A raster made from a parent plus a rect remembers both.
  - `halved()` copies the parent's memoized level and recomputes only the rect's footprint, aligned out to 2^k.
  - The 2x2 box is local even at odd edges (PORT raster.rs:95-120), so the result is bit-identical.
  - Estimate: about 80 ms instead of 1 s at 100 MP. Tiling stays 4c's decision.
- **Patch previews.** `PixelPreview` gains `{ layer, rect, raster (rect-sized), revision }`.
  - The GPU `texSubImage2D`s the patch, halved locally, and restores the touched rects when the preview ends.
  - The CPU fallback composites a patched copy.
  - Pixel moves use patches. Gradient drags use the existing reduced preview (a 1024-2048 cap, 1-4 MP), with
    the full size built at pointerup as a job.
  - `set_preview`'s dedupe key must include the patch (N3, PORT engine.rs:255-263).

**Renderer.**
- `LayerTextures.sync` (PORT layer-textures.ts:54-83): if the level, size and sampling match and
  `pixels_delta` returns a rect, upload that rect into each 2048-px chunk it meets. Upload it straight from
  wasm memory, using `UNPACK_ROW_LENGTH` and `UNPACK_SKIP_*`. Otherwise upload the whole layer.
- `MaskTextures.sync` (PORT mask-textures.ts:10-21) does the same with R8.
- Styled layers upload the worker's reduced image whole (at most 1536 px, about 8 ms). Draw it with its own
  size and inset, as the Mac does (EffectsPreviewCache.swift:35-41).

**Budgets for the plan** (LL-073; release wasm, Edge channel, at 24 MP and 100 MP):

| Measurement | Budget |
|---|---|
| Pixel-move frame with a 1024x1024 patch, at fit and at 1:1 | 33 ms or less |
| Gradient drag frame | 50 ms or less |
| Frame after a gradient commit | 150 ms or less |
| rAF gap while a worker job runs | 100 ms or less |

## 4. Scope and split: two plans, each ending with a portable build

**4b-1: groundwork, colour, Fill, Gradient and Shape.**
- **Groundwork first:**
  - the history cap (3.1);
  - lineage, incremental halving, patch previews and partial uploads (3.2);
  - the job worker with OQ6 effects images, the histogram and commits (2.3);
  - the opaque fast paths (1.6).
- **Features:**
  - palette (X, D; black and white on a mask) and colour picker panel;
  - Eyedropper (I, and Alt in Gradient) with its sample ring;
  - Fill (Alt/Ctrl+Backspace);
  - Gradient (G) with its pending edit;
  - Shape (U: rectangle, ellipse, line), writing the Mac's `shape` record;
  - Gradient Map seeded from the palette.
- **Shared engine piece:** a raster-edit session that grows the layer to the canvas, clips to the selection,
  trims on commit and carries a following mask. It builds on `grown` (:81), `trimmed` (:110) and
  `carry_mask` (:124) in PORT ops/adjust.rs, and 4b-2 and 4c reuse it.
- **Usable at the end:** colours, fills, gradients and shapes, with no freezes on large styled layers.

**4b-2: clipboard, moving pixels, Transform Selection.**
- **Features:**
  - Cut, Copy, Paste, Copy Merged and Layer via Copy on the Windows clipboard (PNG);
  - New Canvas takes its size from the clipboard;
  - the whole-layer copy, if ruled in (question 1);
  - move and duplicate pixels with patch previews (Ctrl-drag, Ctrl+Alt-drag, Ctrl+arrows);
  - Transform Selection (Ctrl+T), a floating layer merged back with growth and a following mask, restored
    exactly on Escape or an unchanged commit.
- **Usable at the end:** pixels exchanged with other Windows apps; selections moved, duplicated and
  transformed.

**Clipboard bridge (technical choice).** Two Tauri commands over raw Win32 formats, e.g. via `clipboard-win`:
- `clipboard_write_image(png, dib)` puts the registered "PNG" format and `CF_DIBV5` on the clipboard.
- `clipboard_read_image()` returns `{ png }` or `{ bmp }`, where the BMP is the DIB behind a 14-byte header.

Supporting points:
- The shell never decodes (spec 4.4); the engine handles both formats.
- `GetClipboardSequenceNumber` plays the role of the Mac's `changeCount` (MAC SelectionClipboard.swift:7-8,
  :127, :149).
- WebView2's `navigator.clipboard.read()` needs a permission, and a menu Paste cannot use the `paste` event.
- **Spike first:** paste from Snipping Tool, Paint, Edge and Photoshop; copy into Paint, Word and Photoshop.

**Not in 4b.**
- Tiled storage (spec 4.1): decide at 4c's start against a measured stroke commit at 4000x3000 and at 100 MP.
- Brushes (4c), healing and Content-Aware Fill (4d).
- Live shapes and the picker's effect and text targets (Phase 5).

## 5. Mac behaviours each 4b feature must reproduce

### 5.1 Palette and colour picker (4b-1)

- **Palette** (MAC Document/ColorPalette.swift).
  - Foreground black, background white (BrushStroke.swift:9-14; EditorSession.swift:228).
  - X swaps and D resets (:45-58; KeyboardShortcuts.swift:104).
  - With a mask targeted, the palette is black and white only (`maskPaintWhite`, :26-36) and the picker does
    not open (:61).
  - The palette is frozen while busy or stroking (:25).
- **Picker.**
  - HSB is the source of truth: greys keep their hue and black keeps its saturation (:247-293). Colours are
    quantised to 8 bits (:296-299).
  - Hex accepts RRGGBB or RGB, with or without `#` (:303-310). OK commits and Cancel restores (:85-118).
  - The panel has a 256-pt saturation/brightness field, a hue strip, new/current swatches, RGB and hex fields,
    and "Click the canvas to sample" (UI/ColorPickerSheet.swift:4-37, :71-90; ColorPalette.swift:174-178). It
    floats and reopens where it was left.
  - In 4b it targets the palette and the Gradient Map ends (:120-125, :167-173).
- **Tests (ColorPickerTests):** `hexParsesFullShorthandAndRejectsInvalid` (:7), `hsbRoundTripsEightBitColors`
  (:16), `graysAndBlackKeepPreviousHueAndSaturation` (:23), `canvasSamplingReadsCompositeAndCommitsOnlyOnOK`
  (:33), `pickerReopensWhereItWasLastLeft` (:63).

### 5.2 Eyedropper (4b-1)

- **Keys.** I selects it (MAC EditorCanvas.swift:1846); Alt turns Brush, Spot Healing and Gradient into it
  (:71).
- **Sampling.**
  - A click or drag samples the visible composite at the floored point.
  - The value is un-premultiplied as `(min(a, v) / a * 255).rounded() / 255`. Off the canvas, or over
    transparency, nothing is sampled (ColorPalette.swift:182-203).
  - The sample always sets the image foreground, even with a mask targeted (EditorCanvas.swift:2047-2048).
- **Sample ring.** 116 pt, new colour above old, on a grey 0.45 ring (SampleRingOverlay.swift:15-27;
  EditorCanvas.swift:2050-2054). It can be switched off (EditorSession.swift:107).
- **Port notes.**
  - Add the Mac's rounding to `sample_color` (PORT engine.rs:565-571).
  - A 1-px composite through a blur layer took about 2 s (4a Task 2), so keep one drag sample in flight at a
    time.
- **Tests:** CanvasEntryTests `eyedropperShortcutSelectsTool` (:7), and the ColorPickerTests sampling test.

### 5.3 Fill (4b-1)

- **Keys.** Alt+Backspace fills with the foreground, Ctrl+Backspace with the background (Mac Option- and
  Cmd-Delete, MAC CompositorApp.swift:177-188). Add Delete variants.
- **What.** The selection, or the whole layer with none; an empty selection fills nothing. On a mask the colour
  is black or white (SelectionEdits.swift:37-50).
- **How.** The layer grows to the canvas, the selection clip keeps soft edges, and the commit trims the result
  (BrushStroke.swift:626-631, :645-675; EditorSession+Brush.swift:154-188).
- **Refusals** follow `canPaint` (:5-11). Undo: "Fill" or "Fill Mask".
- **Tests (SelectionEditTests):** `fillUsesPaletteInsideSelectionOrWholeLayerWithoutOne` (:77),
  `maskFillHidesOnlyTheSelectedArea` (:111), `emptySelectionEditsNothing` (:43).

### 5.4 Gradient (4b-1)

- **Settings.** Linear or Radial (Tab); Foreground to Background, or Foreground to Transparent (the default);
  Reverse; Opacity 1-100%, which digits also set (MAC Document/Gradient.swift:3-19;
  UI/GradientControls.swift:9-29; EditorSession+Brush.swift:189-191).
- **Gesture.**
  - A drag needs `canPaint`; redragging replaces the pending line (Gradient.swift:36-51).
  - Either end can be grabbed within 10 pt (EditorCanvas.swift:2023-2032); Shift snaps to 45 degrees
    (:2034-2040).
  - A line under 0.5 px cancels (Gradient.swift:32, :84-87).
- **Pending edit.**
  - Return or Apply commits, Escape or Cancel discards, and a tool or layer switch applies it
    (EditorSession.swift:331, :337; Gradient.swift:104-108).
  - The first Undo discards it (EditorSession.swift:573-578), and history is blocked while it is pending
    (:608).
  - Palette and setting changes re-render it (:205, :227-229).
- **Drawing** (BrushStroke.swift:605-623, :645-675).
  - It is a two-stop `CGGradient` in sRGB (DeviceGray on a mask), drawn over each tile's original pixels at
    the opacity and clipped to the canvas and the selection.
  - Linear extends past both ends. Radial is centred on the start, with radius |end - start|.
  - The colours come from the palette (Gradient.swift:72-82).
- **Commit.** "Gradient" or "Gradient Mask", through `commitRasterEdit`, which trims and grows a following mask
  (:95-102).
- **Tests.**
  - All eight GradientTests: `foregroundToBackgroundFillsCanvasAndCommitsOneUndo` (:37),
    `radialSpreadsFromStartToRimInEveryDirection` (:55), `reverseOpacityAndDirectionFollowSettings` (:69),
    `foregroundToTransparentPreservesUnderlyingPixelsAndAlpha` (:83),
    `cancelUndoAndClicksLeaveDocumentUntouched` (:102), `redraggingReplacesPendingLineWithoutAccumulating`
    (:116), `maskGradientWritesCoverageInsideLayerBounds` (:126), `paletteChangesUpdatePendingPreview` (:144).
  - SelectionEditTests `gradientStaysInsideTheSelection` (:64).
  - The tolerance is 3 levels, because "Core Graphics quantizes gradient ramps" (GradientTests.swift:33-35).
  - Pixels are read at their centres: a line from x 0.5 to 100.5 gives pixel 50 the value 128.

### 5.5 The CGGradient interpolation question, settled on paper

For both Mac styles, straight and premultiplied interpolation give **identical pixels**:
- Foreground to Background has two opaque stops, so alpha stays 1.
- Foreground to Transparent has the same RGB at both stops (Gradient.swift:78-80):
  - straight interpolation gives c at alpha (1 - t), which premultiplies to c(1 - t);
  - premultiplied interpolation lerps c to 0, which is also c(1 - t), at alpha (1 - t).
- Reverse only swaps the stops.

No probe is needed for that question. The probes must settle instead:
- whether CG's 8-bit ramp rounds or truncates, and whether it dithers (the tests' "few levels short");
- the pixel-centre and radial-centre conventions;
- the rounding when the opacity composites over existing pixels.

### 5.6 Shape (4b-1)

- **Kinds.**
  - Rectangle: corner radius 0-5000, clamped to half the shorter side, so it can become a pill.
  - Ellipse.
  - Line: width 1-5000, round caps (MAC Document/ShapeTool.swift:3-16, :197-223).
  - Controls: sliders 0-200 and 1-100, defaults radius 0 and width 4 (UI/ShapeControls.swift:17-46;
    EditorSession.swift:236-240). Shift+U or Tab cycles the kind (ShapeTool.swift:106-111).
- **Gesture.**
  - The anchor rounds to whole pixels, and the box is `DragBox.rect`: Shift for a square or circle, Alt from
    the centre (:72, :84-100; MAC Document/Selection.swift:93-105).
  - Shift snaps a Line to 45 degrees (:86-96).
  - A click, Escape or a tool switch makes nothing (EditorSession.swift:337).
- **Result.**
  - A new layer above the active one, named "Rectangle 1" and so on, in one undo step named for the kind.
    The selection is kept (:113-157; SelectionClipboard.swift:251-265 with `dropsSelection: false`).
  - A Line's layer is the box of its ends grown by half the width, with the ends stored as fractions of that
    box (:120-140).
  - Shapes under 1 px or over 200 MP are refused (:129-133).
  - The shape is an antialiased CG path fill, or a round-capped stroke at least 1 px wide (:197-223).
- **Record.** It writes `LayerShapeStyle` as `shape` (:19-32): kind, red, green, blue, cornerRadius, and for a
  Line only lineWidth, start and end.
  - The port keeps `shape` as opaque JSON and drops it when the pixels change (PORT document.rs:16, :62-65).
  - The tool must write what Swift's Codable writes, with CGPoint as `[x, y]`.
  - Redraw on scale (:161-194) is Phase 5.
- **Tests (ShapeToolTests):** `rectangleFillsANewLayerWithTheForegroundColorAsOneUndoStep` (:32),
  `ellipseLeavesItsCornersClearWithShiftCircleAndOptionFromCenter` (:54),
  `aClickEscapeOrToolSwitchMakesNoLayer` (:67), `roundedRectanglesFollowTheRadiusAndClampToAPill` (:84).

### 5.7 Clipboard family (4b-2)

Source: MAC Document/SelectionClipboard.swift. Menus: CompositorApp.swift:146-171, :280-281.

- **Copy (Ctrl+C) with a selection.**
  - It copies the active layer as the canvas shows it, or the mask as opaque grey (a placed mask's background
    fills outside it).
  - The copy keeps the selection's soft edges and is cropped to its whole-pixel bounds with a 0.001
    tolerance, within the canvas (:24-62).
  - An empty selection refuses (:36-39).
  - Copy puts the PNG on the pasteboard and keeps an internal copy with its origin and the change count
    (:120-129).
- **Copy with no selection** copies the layer itself, with no PNG (:97-118). It pastes here or in another
  project through the workspace (CompositorApp.swift:167). See question 1.
- **Copy Merged (Ctrl+Shift+C)** composites the region, then clips it (:64-95).
- **Cut (Ctrl+X)** copies, then clears; it needs a selection, and its undo step is "Clear" (:131-136).
- **Paste (Ctrl+V)** makes a new layer, "Layer N", above the active one, as one "Paste" step.
  - A current internal copy returns to its origin.
  - Anything else is converted to sRGB and centred with floor (:138-157).
  - Paste drops the selection (:249-272).
- **Layer via Copy (Ctrl+J)** copies the selected pixels to a new layer in place and drops the selection.
  With no selection it is Duplicate Layer (:159-170).
- **New Canvas** suggests the clipboard image's size (MAC UI/NewCanvasSheet.swift:51-59).
- **Tests.**
  - SelectionClipboardTests: `copyAndPastePutsPixelsOnANewLayerInPlace` (:48),
    `cutLeavesAHoleAndPasteRestoresThePixels` (:68), `layerViaCopyCopiesTheSelectionOrDuplicatesTheLayer`
    (:78), `lassoShapedSelectionCopiesAndPastes` (:162),
    `copyMergedTakesEveryVisibleLayerNotJustTheActiveOne` (:178).
  - CanvasEntryTests `clipboardSuggestsImagePixelsAndIgnoresText` (:43).

### 5.8 Moving and duplicating selected pixels (4b-2)

- **Keys.**
  - With any selection tool, Ctrl-drag inside the selection moves the pixels and Ctrl+Alt-drag duplicates
    them (MAC EditorCanvas.swift:1968-1974).
  - In any tool, Ctrl+arrows move them 1 px, or 10 px with Shift (KeyboardShortcuts.swift:114-117;
    EditorCanvas.swift:1802-1808).
- **Scope.** Image pixels only: a mask refuses with a beep (SelectionEdits.swift:141-151, :193-198). Offsets are
  whole pixels (:155-161).
- **Each frame.** The pixels are lifted through the clip (BrushStroke.swift:684-699). Then the tiles over the
  source and target are rebuilt from the original: the hole is cleared unless duplicating, and the lift is
  drawn at the offset without interpolation (:703-747).
- **The outline** shows at its moved place throughout (SelectionEdits.swift:163-172).
- **Commit.** "Move Pixels" or "Duplicate Pixels", one step together with the moved selection; a zero offset
  commits nothing (:176-185).
- **Tests (SelectionEditTests):** `cmdDragMovesSelectedPixelsAndOutlineAsOneUndo` (:249),
  `duplicatePixelDragPreservesSourceAndUndoesTogether` (:270), `cmdArrowNudgesPixelsAndMasksRefuse` (:289),
  `pixelMoveNeverShowsTheOutlineAtItsOldSpot` (:331).

### 5.9 Transform Selection (4b-2)

- **When.** Ctrl+T with a non-empty selection on an image layer, not a mask (MAC
  Document/FloatingSelection.swift:15-25; CompositorApp.swift:278-279).
- **Lift** (:27-56). Inside one "Transform Selection" edit, the pixels are lifted as displayed and cleared
  from the source. A "Floating Selection" layer, with the source's opacity and blend mode, goes above it, and
  the app switches to Move with a persistent transform. The outline follows the handles (:59-64;
  SelectionEdits.swift:166-170).
- **Commit** (:68-108, :117-158).
  - A distortion is warped first, with `DistortWarp` (the port has ops/distort.rs).
  - The pixels merge into the source's grid, growing the layer and a following mask (white in the new
    area), and the selection moves with them.
- **Restores.** An unchanged commit, and Escape, restore the document exactly, and Escape leaves no undo step
  (EditorSession.swift:432-434, :477; FloatingSelection.swift:110-114).
- **Quirk.** The merge rebuilds the layer without its effects (FloatingSelection.swift:98-100), because
  `ImageLayer`'s `effects` defaults to nil (EditorSession.swift:43). See question 3.
- **Tests (SelectionClipboardTests):** `transformSelectionMovesPixelsAndOutlineAsOneUndo` (:92),
  `escapeRestoresExactlyWithoutAnUndoStep` (:117), `applyingAnUnchangedTransformLeavesSoftEdgesUntouched`
  (:131), `scalingAndMovingPastTheLayerEdgeGrowsTheLayer` (:143).

### 5.10 Shortcuts to add to the fixed map (PORT app/src/shortcuts/keymap.ts)

| Keys | Action |
|---|---|
| I, G, U | Eyedropper, Gradient, Shape tool |
| Shift+U, or Tab in the Shape tool | Cycle the shape kind |
| Tab in the Gradient tool | Cycle the gradient shape |
| X, D | Swap and reset the colours |
| Alt+Backspace/Delete, Ctrl+Backspace/Delete | Fill with the foreground, with the background |
| Ctrl+X/C/V, Ctrl+Shift+C | Cut, Copy, Paste, Copy Merged |
| Ctrl+J | Layer via Copy when there is a selection |
| Ctrl+T | Transform Selection or Transform Layer |
| Ctrl+arrows, Ctrl+Shift+arrows | Move the selected pixels 1 px, 10 px |
| Digits | Gradient opacity |

Mac sources: KeyboardShortcuts.swift:74-117; CompositorApp.swift:146-192, :278-281; EditorSession.swift:363-382
(Tab).

## 6. Mac probes for the user to export (PNG from Mac 1.2.10)

Each probe is a small `.comp` under `build-artifacts/mac-probes/` with a one-line instruction. The drags are
made by hand, so the analysis fits the endpoints and boxes from each export.

| Probe | Setup and gesture | Settles |
|---|---|---|
| gradient-linear-ramp | 512x32 white; black to white, Foreground to Background; Shift-drag across at 200% | Ramp rounding, dither, pixel centres |
| gradient-radial | 256x256; radial black to white, from the centre out to about 100 px | Radial distance, the centre pixel |
| gradient-opacity-over-colour | 256x32 filled (0.8, 0.3, 0.2); blue to transparent at 37% | 8-bit compositing over existing pixels |
| gradient-mask | The same layer with a white mask, mask targeted, black to white | The DeviceGray ramp in a mask |
| shape-curves | Red ellipse 101x61; rounded rectangle 120x80 at radius 20; pill 150x40 (radius 5000) | CG fill antialiasing |
| shape-lines | Lines 1, 4 and 15 px wide, at 0, 45 and about 30 degrees | Stroke antialiasing and round caps |
| fill-feathered | Foreground fill in an antialiased ellipse, feather 8, over a coloured layer | Fill through a soft clip |
| clipboard-png | Copy a feathered selection of a 50%-opacity layer; Copy Merged across two layers; each via Preview's New from Clipboard, then export | The pasteboard PNG: soft alpha, rounding, colour tag |
| transform-selection-rotate | Cmd-T on a selection, rotate 30 degrees, scale 150%, apply | Floating-merge resampling (ties to Phase 3.5d sampling) |

The Eyedropper's rounding and the palette maths need no probe: the code is explicit (5.1, 5.2).

## 7. Product questions

1. **Whole-layer Copy and Paste.** With no selection, the Mac's Ctrl+C copies the layer itself (folder
   contents, mask, effects), and Ctrl+V pastes it into this or another open project. Nothing goes to other
   apps (MAC SelectionClipboard.swift:11-18, :97-118).
   - It is close to the spec's v1 non-goal "dragging layers between open projects". Include it in 4b-2, or
     leave Ctrl+C with no selection doing nothing?
   - Recommendation: include it. One engine holds every open document, so a copy only shares rasters.
2. **Canvas effects on large styled layers.** The Mac always draws canvas effects from an image of at most
   1536 px, sharing a 64 MiB budget (MAC EffectsPreviewCache.swift:51, :67-74), which looks soft at 100%. The
   port draws the full-size image, which is sharp and matches export but takes 7 s at 24 MP and 27 s at
   100 MP.
   - Options: (a) always reduced, as the Mac; (b) reduced at once, then full size when the worker finishes.
   - Recommendation: (b) up to about 24 MP padded, (a) above that.
3. **Transform Selection and layer effects.** The Mac's Transform Selection commit drops the layer's effects
   (FloatingSelection.swift:98-100), as its Invert does (SelectionEdits.swift:120-122). The port's Invert keeps
   them (PORT document.rs:62-65).
   - Keep them, as a recorded deviation, or drop them as the Mac does?
   - Recommendation: keep them.

A consequence of the binding rule, not a question: the history cap makes destructive edits on any layer over
256 MiB (such as 100 MP) impossible to undo, exactly as on the Mac (3.1).

## 8. Housekeeping

- **Release wasm.** A release wasm (2.57 MB, built 07:56 from a9937bd, presumably by the 0.4.1 portable build)
  was already in place when this began. `pnpm wasm` rebuilt it unchanged for the measurements.
- **Dev wasm now in place.** As instructed, `pnpm wasm:dev` rebuilt the dev package at the end (10.4 MB,
  08:29). Run `pnpm wasm` before any PERF timing.
- **Cleanup.** The temporary spec (`app/tests/e2e/perf-4b-probe.spec.ts`) and a scratch GL-backend probe script
  are deleted.
- **Nothing else changed:** no product code, no repository copy.
