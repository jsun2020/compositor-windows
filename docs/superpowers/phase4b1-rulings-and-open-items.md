# Phase 4b-1: rulings and open items

Phase 4b-1 (groundwork and colour) was executed from
`docs/superpowers/plans/2026-09-28-phase4b1-groundwork-and-colour.md` by subagent-driven development on
branch `phase4b` (plan c46d184 on f032c39; Tasks 1-17 and 14a d4799d5..5496f5c; final fix wave from
5496f5c). This file keeps the decisions, the measured numbers and the open items after the SDD workspace
is deleted. Task 14a's brief, which the plan does not contain, is appended as the annex.

## What shipped (0.5.0)

- Groundwork: the Mac's history cap (100 entries, 256 MiB of history-only pixels); entry ids and a saved
  state token; changed rectangles by revision (a lineage of 256 changes) with partial GPU uploads and seeded
  halvings; a job worker (a second engine in a module worker) for large layers' destructive commits, Levels
  and Curves histograms, effects images, Fill and the Gradient.
- Colour: the palette (foreground / background, a mask's own black and white, X and D), the floating colour
  picker, the Eyedropper (and Alt with the Gradient tool), Gradient Map ends through the picker.
- Fill (Alt / Ctrl + Backspace or Delete, Edit > Fill) and the Gradient tool (linear and radial, pending
  until applied), painted as the Mac's raster edit paints; on a mask both grow the mask past its layer to
  the canvas as Compositor 1.3.7 does (Task 14a, the user's decision).
- The Shape tool: rectangles (rounded), ellipses and lines on new layers, with the Mac's `shape` record.
- Release wasm 2,971,316 bytes after the final fix wave (2,571,151 at f032c39: +400,165, +15.6 %).

## The plan's rulings, as amended during execution

Each is the plan's ruling (global-constraints.md, "Rulings made for this plan"), with the numbers that were
measured on the real repository (release wasm in Edge on the Intel HD 520, ANGLE D3D11, unless marked
native) in place of the plan's scratch-copy figures where they differ (final review minor 23).

1. **OQ1 History cap.** The Mac's: 100 entries and 256 MiB of pixels only history holds; the oldest undo
   entry, then the farthest redo entry, dropped after every push, undo and redo, even the entry just made
   (DocumentHistory.swift:28). Bytes counted incrementally by buffer identity. An edit to a layer over 256
   MiB applies and cannot be undone, as on the Mac. History drops the halvings of rasters only it holds.
   Measured (Task 2): the wasm heap after 20 inverts at 100 MP 841 MB (budget 2560); a rename at the cap
   with 1000 layers 2.8 ms, an undo there 2.1 ms (budget 25 each). The trim after a redo is pinned by the
   halvings it lets go (final fix wave): a redo restores exactly the entries the push already trimmed, so its
   byte and entry trim never drops anything new.
2. **OQ2 Entry ids and a state token.** `DocumentState.undoEntryId`; the transform's Alt-drag cancel
   compares ids, not depths; the saved state is a token, so trimming the front never loses "saved".
3. **OQ3 Changed rectangles by revision.** `Region { layer, plane, rect }` in `Dirty.regions`; the lineage
   (`LINEAGE_LIMIT = 256`); `pixels_delta` / `mask_delta` give the union since the texture's revision, empty
   for nothing, None to upload whole. A buffer that kept its size AND its grid takes the reported rectangle
   (Task 3's same-size shifted grid is whole). Amended by the final fix wave (minor 9): the lineage carries
   each buffer's identity, and a revision change that keeps the very buffer at the same size (a placed mask
   following a nudge, a mask moved on its own, the undo and redo of either) records an empty change. The
   mask revision still moves, so a job's stamp still sees the move.
4. **OQ4 Seeded halvings.** A new raster inherits its predecessor's halvings outside the changed
   rectangle. Measured (Task 4): the frame after a clear in a 1024 px selection 5.6 / 13.8 ms at 24 MP (fit
   / 1:1) and 4.2 / 6.5 ms at 100 MP, budget 33, no whole upload; Task 17's run 11.6-17.8 ms at 24 MP.
5. **OQ5 Large layers go to a job worker** past `JOB_PIXELS = 4,000,000` stored pixels (Fill and the
   Gradient by the pixels they paint, ruling C1). The result goes back as one undo step only if the layer's
   stamp (revisions, transform, canvas size, selection revision; the mask placement through the mask
   revision) is unchanged. One job at a time; edit and histogram jobs outrank effects jobs; a worker past 1
   GiB, or dead, is replaced; `working` makes commands, undo, redo, file actions, the palette, panels,
   transforms, layer clicks, the picker's OK and canvas sampling wait ("Wait for the current edit to
   finish."). Budgets: the copies out and back 150 ms at 24 MP and 450 at 100 MP (ruling I7: OQ5's "500"
   means 450); the longest gap while the worker edits 100; while it reads a histogram 150; the frame after a
   whole-layer result 350 (24 MP) and 500 (100 MP; 600 for the C1 blank-layer cases, ruling 15-perf-3).
   Measured (Task 7 final): 24 MP jobInput 72, installJob 62, histogram gap 31, commit gap 18, frame after
   258 ms; 100 MP 283 / 290 / 18 / 18 / 229 ms. The job's document holds one layer, so its own budget checks
   see none of the others: amended by the final fix wave (C-1), `install_job` refuses with TooLarge when the
   result would take the project's layer pixels or masks past 100 MP, leaving the document untouched. Not in
   the worker: an adjustment layer's histogram (a composite of everything under it) and Invert.
6. **OQ6 Effects images off the UI thread** past 65,536 padded pixels: first the layer plainly, then a
   reduced image (at most 1536 px on its longer side), then (to 24 MP of layer pixels) the full-size image,
   kept in the engine's cache. Budgets: 24 MP gap until the reduced image 100, while the full image is made
   150, the frame that draws it 150, the full-size copy out 150; 100 MP gap until the reduced image (now
   including the interval that draws it) 200; preemption: the edit job posted within 400 ms, the longest gap
   during it 300, the re-ask's copy 150. Measured (Task 7 round 5, seven runs): 24 MP reduced 25-43, full
   window 85-126, frame that draws it 60-87; 100 MP 106-152; preemption 186-297 / 92-162. Final fix wave, whole
   perf suite: 24 MP 46 / 110 / 67, full-size copy 97; 100 MP 124 (the draw's own interval 21, now charged);
   preemption (3 runs) (a) 257-270, (b) 110-132, (c) 79-115 ms. At 100 MP no full-size image is asked for or kept (asserted).
7. **OQ7 Fill and the Gradient paint as the Mac's raster edit does** (BrushStroke.swift:153-160,
   :645-675; EditorSession+Brush.swift:154-188): the layer's grid grows to cover the canvas as the layer maps
   it; each pixel painted at its centre, inside the canvas, source-over at the paint's alpha times the
   opacity times the selection's coverage (k / 255), rounded half up; trimmed to what is left; a covering
   mask follows, white where the layer grew. A mask is painted in its own grid in grey; a Fill or a Gradient
   grows it past its layer to the canvas, as Compositor 1.3.7 does (`applyPixelEdit` and `beginGradient`
   pass `growsMask: true`), its new area its background, the old grid joined with the 256-pixel tiles
   painted, placed on its own (Task 14a). An edit that reaches no pixel (a selection moved off the canvas)
   changes nothing and records nothing, as the Mac returns on an empty patch set (final fix wave, minor 8).
   Measured natively (Task 8): a gradient over the whole layer 647 / 2706 ms at 24 / 100 MP, a fill in a 2000
   x 1500 ellipse 133 / 343 ms; a canvas-sized mask Fill 435-680 / 2163 ms (Task 14a; ceiling 1000 / 3300,
   ruling 14a-perf-2); through the worker the page's frames stay under the 100 ms gap budget.
8. **OQ8 Gradient previews:** while the line is dragged from the grown grid reduced to at most
   `GRADIENT_DRAG_LIMIT = 1024` px (budget 50 a tick), once let go at most `GRADIENT_SETTLED_LIMIT = 2048`
   (budget 150); inside a selection of at most `PATCH_LIMIT = 1 << 19` pixels on a canvas-covering layer
   with no effects, the full-size patch (budget 50). A mask gradient previews the mask reduced the same way
   (the pixel gradient's budgets, ruling I2). Measured (Task 9 and fix round 1): drag 21-45, settled 62-113,
   patch 27-36, mask drag 13-32, mask settled 26-59 ms; Task 17: drag ticks 18-37 ms.
9. **OQ9 "Large" is the stored layer's size** (`Engine::stored_pixels`): `state()` reports a previewed
   layer at its preview's size. For Fill and the Gradient, "large" is `Engine::edit_pixels` (ruling C1), the
   pixels the edit paints, which counts a mask grown to the canvas (Task 14a).
10. **OQ10 The Gradient's pending edit follows the Mac** (Gradient.swift, EditorSession.swift:573-590):
    Return / Apply commits, Escape / Cancel drops, the first Undo discards it (a Redo drops it too, only when
    there is something to redo), a change of tool, layer, target or document applies it, a new line on the
    same target replaces it, 10 view px handles, Shift eighths, digits set its opacity (at least 1 %).
    Where the Mac refuses other edits while it is pending (`canEditLayers`), this port applies it first:
    `run`, a panel, a fill and an import. Amended by the final fix wave (I-1): when applying it starts a
    worker job (a large target), `run` and `importImages` then wait like any other command, rather than
    change the stamp the gradient's result must find (which lost the gradient) or land before it.
11. **OQ11 Shapes are drawn by the selection's exact-area rasteriser** from Core Graphics' outlines
    (kappa Beziers, round-capped capsules, flattened within 0.01 px); within 4 levels of an independent area
    count; the layer `Int(width)` x `Int(height)` at the box's origin, named "<Kind> <n>", inserted where New
    Layer inserts (the port's convention), one undo step; the Mac's `LayerShapeStyle` record. Committed on
    the UI thread as an import is: budgets 1000 / 2000 ms at 24 / 100 MP. Measured (Task 16): rectangle over
    the canvas 503-527 / 1156 ms, ellipse 297-339 / 882 ms; Task 17: 437 / 1105 ms. A shape whose side passes
    30,000 px under the pixel budget is refused with words about its side (final fix wave, minor 17).
12. **OQ12 The palette is session state**, with the mask's own black and white; the palette does not change
    while a job's result is to come.
13. **OQ13 The colour picker floats**, Enter OK and Escape Cancel ahead of every other key handler, samples
    the canvas while open, snaps to 8 bits, edits the two swatches and Gradient Map's ends. Its OK on a swatch
    while a job's result is to come keeps it open with the chosen colour and says why (final fix wave).
14. **OQ14 Sampling reads the stored document's composite** (`sample_color`, the Mac's rounding). Budget 16
    ms; measured (Task 13) 5.8 / 1.1 ms at 24 / 100 MP. No sampling while a job's result is to come (final
    fix wave: `samplingInto` returns null, as the Mac's canvas ignores the press, EditorCanvas.swift:1386).
15. **OQ15 Fill's keys** are Alt + Backspace / Delete (foreground) and Ctrl + Backspace / Delete
    (background); Delete with a selection on a targeted mask fills it with the mask's background colour.
    Fill needs what the Mac's `canPaint` needs, no crop rectangle pending, and no job's result to come.
16. **OQ16 Tool rail icons:** Eyedropper is Lucide's `pipette`; the Gradient and the Shape icons are drawn
    in Lucide's style.
17. **OQ17 Mac probes:** `shapes.comp`, `gradient-linear.comp`, `gradient-radial.comp`,
    `gradient-over-colour.comp` (mac_probes.rs), exported by the user by hand. As of 2026-09-29 shapes.png is
    usable, the two gradient exports are usable with their colours reversed, gradient-over-colour must be
    redone (the 37 % went to the layer opacity), and the saved `.comp` folders did not come back. Pinning is a
    follow-up; OQ17's "within 4 levels on edges" will not hold for the probes (Line 1's end pixel +28, the
    ellipse -11): per-shape tolerances or a ruling then.
18. **OQ18 Version 0.5.0**; the README's Phase 4b-1 section.
19. **OQ19 Not ported, disclosed:** redrawing a shape layer when it is scaled here, and the Mac's live
    preview of a rounded rectangle while scaled; recolouring live text with Fill; Canvas Size's Foreground /
    Background extension choices; the picker for layer effects' and Vignette's colours; the Mac's Sample Ring
    visibility toggle (ContentView.swift:54 `showsSampleRing`; Task 13 review).
20. **OQ20 Paths without a budget, and why:** the Mac probes (not interactive); opening the picker, the
    swatches and the bars (no pixels touched); the first-draw frame of a new layer (as any import; ruling
    I2). The worker-budget gap this ruling disclosed is closed: the app's `editPixels` question refuses a
    Fill or Gradient too large before sending (Task 14a), and `install_job` enforces the project budget with
    the result in place (final fix wave, C-1). Untimed paths the final review listed are under "Open items".
21. **OQ21 A revision counter per engine:** a job's result is compared only by the stamp read from the UI
    engine.
22. **OQ22 Wasm size:** 2,571,151 bytes at f032c39; 2,970,936 after Task 17 (+399,785, +15.5 %; the plan's
    2,913,123 was a scratch-copy estimate); 2,971,316 after the final fix wave.

## Controller rulings

Pre-flight (preflight.md: 1 Critical, 9 Important, 13 Minor in the plan):
- C1: Fill and the Gradient decide job versus UI thread by the pixels the edit will PAINT
  (`Engine::edit_pixels`), not the stored size; perf cases on blank layers at 24 and 100 MP. Cost if wrong:
  a 0.8-3.2 s freeze on the commonest case (a new document's blank layer).
- I1: decide by `stored_pixels` from Task 15; effects images follow the shown size by design (M13).
- I2: budgets where the plan measured without asserting: Levels open (< 200 / 500 ms), clear-in-selection
  engine time (<= 1.5x measured), the mask gradient tick (the pixel gradient's 50 / 150), the shape draft
  tick (< 16 ms), undo at the cap (<= the push's budget); the first-draw frame of a new layer added to OQ20.
- I3: the palette's "does not change while working" test rewritten to fail without the guards.
- I4: offset-and-scaled and placed-mask cases in changed_rects.rs.
- I5: the worker's use proven by counting `jobs.run`.
- I6: Step 5 bugs verified on the final code only; replacements for Tasks 4 and 13.
- I7: OQ5's "500" means 450.
- I8: the plan's rulings appended to global-constraints.md. I9: perf runs log UNMASKED_RENDERER_WEBGL;
  SwiftShader or Basic Render invalidates a run.
- M1-M13: carried to the tasks that own them (hand-applied hunks, weak assertions, own native numbers).

Execution (every `Ruling:` in the ledger):
- A selection that is Some with zero points still sends its (empty) point bytes (Task 5 / 6).
- Edit and histogram jobs outrank effects jobs; an edit or histogram request while an effects job runs
  terminates and respawns the worker and re-queues the effects job (Task 7, finding 3).
- Split, do not widen: the full-size effects ask is deferred to the frame after the reduced draw; the 150
  budget stands (Task 7 round 3). The 100 MP reduced-window gap (100-134 ms then) accepted and recorded as
  an open item (Task 7). The 170 ms budget for the 24 MP full window was withdrawn (it rested on a wrong
  attribution): the redraw after `keepEffectsImage` runs a painted frame later, 150 stands (Task 7 round 4).
- Task 7 parked: the 100 MP reduced-image draw interval outside every budget, and the (c) re-ask copy
  mis-windowed with the full-window copy unasserted: both done in the final fix wave.
- Task 9: the mask gradient tick budgets; T9-1 (the gather rewritten, the fast path dropped); T9-3b (the
  preview does not mirror the covering-mask growth refusal: a known gap, below); T9-4 (refresh inside the
  window, 50 stays; a flake is re-run once); the 66 ms spike on the HD 520 accepted as a flake risk; T9-6
  parked, then fixed in Task 15 (a pixel preview carries the covering mask onto the grown grid).
- 14a-Fill: 1.3.7 also grows the mask for Fill (`applyPixelEdit` `growsMask: true`); the user agreed.
- 14a-perf: a canvas mask Fill ceiling of 266 / 686 ms was an estimate; 14a-perf-2 made it 1000 / 3300 ms
  (it runs in the worker); the 100 MP frame after applying a grown mask 400 ms (24 MP 150); the cold first
  fill gets a warm-up before timing, the cold gap logged, and a follow-up to pre-warm the worker.
- Pre-flight of Tasks 14 / 14a / 15 (preflight-14-14a-15.md): SelectionClip::region for `editPixels`; the
  grown mask's nudge / undo / redo budgeted (< 150 ms) with keeping the revision on a placement-only move as
  a follow-up (done in the final fix wave, differently: the buffer identity, not the revision); frame gaps by
  `performance.now()` in rAF with the frame after inside the window; I5 carried into fill.spec.
- 15-perf: mask gradient frame after 150 / 400; the "jobs" 100 MP frame after a result 500. 15-perf-2: the
  Fill's cold frame warmed before timing. 15-perf-3: C1 gets a warm-up worker job before timing, the < 100
  gap kept; C1's 100 MP frame after a result 600 ms (`layerPixels` prefilters on the UI thread, 314-533 ms).
- R16-1..3: the fill frame-after 350 / 600; the C1 warm-up block; the shape draft tick < 16 ms.
- Final review triage: the reviewer's triage and fix order accepted as the single fix wave; F1 is the first
  task of the next phase, beside the 1.3.7 catch-up.

## Final review and fix wave

The final review (a9937bd..5496f5c) found 1 Critical, 2 Important and 23 Minor. Fixed:
- C-1: a job's install enforces the project budget (engine/src/jobs.rs `install_job`); a project over it
  would not reopen (package.rs, OverBudget).
- I-1: `run` and `importImages` wait after applying a pending gradient that started a job.
- I-2: `EffectsCache::insert` prunes dead entries first (they pinned old rasters, up to about 1 GB).
- Minors 1-7, 10: a layer click, the picker's OK, sampling, Save and the close prompt's OK, the histogram's
  copy out, the trap wording, the missing engine file, Delete while working.
- Minor 8: nothing painted, nothing recorded. Minor 9: placement-only mask moves upload nothing; the 14a
  nudge / Undo / Redo frame budget retightened to 33 ms (measured before: 18-27 ms at 24 MP and 61-189 ms at
  100 MP, one whole upload each; after: 4-9 and 6-12 ms, no upload).
- Minors 11, 13-17, 19-22 and the test batch (k / 255, the canvas clip, trim after redo, the settled T9-6
  preview exact, computed literals, the blur job's undoDepth, the perf windows and sentinels).
- F2, part 1: the job worker starts at app startup (`JobClient.warm`).

## Deviations from the Mac, recorded

- Mask growth follows 1.3.7, not the 1.2.10 oracle (the user's decision of 2026-09-29).
- New layers (shapes) are inserted where New Layer inserts: above the active layer, inside the active
  folder; the Mac inserts right after the active row.
- `Mask::background` reads the full-size mask's edge, where the Mac reads its thumbnail's (at most 96 px on
  its long side, LayerMask.swift:61-76): the same up to 96 px, possibly different past it (14a audit M-7).
- `image_grid` checks the whole grown grid against the pixel budget; the Mac checks the touched tiles
  (stricter here; Task 8).
- A Fill or Gradient that paints no pixel returns quietly (the Mac's `guard !edit.patches.isEmpty`); the
  port's test for "painted" is a pixel inside the canvas the selection covers, the Mac's is a tile touched.
- A shape wider or taller than 30,000 px is refused with words about its side (the Mac checks its pixel
  count only, ShapeTool.swift:130).
- Transform Selection is Phase 4b-2.
- A layer click while a job runs is refused with the busy message; the Mac allows it
  (LayerGroups.swift:96-99 has no isProjectBusy check). Skipping only the execute would let the next
  refresh snap the selection back and would drop a pending gradient.

## Open items and follow-ups

- **F1 (first task of the next phase):** prefilter the display level in the worker and return it with the
  job's result. `engine.layerPixels` halves a fresh large result on the UI thread (61-119 ms at 24 MP,
  314-533 ms at 100 MP), which is why "frame after the result" budgets are 350 / 500-600 ms. Design (final
  review): `RasterInner` adopts one halving level; `run_edit_job(.., display_level)` halves in the worker;
  a fourth job buffer; `install_job` adopts it when `output.regions` is empty; the store passes the level
  the renderer last uploaded. Tests: the adopted level equals halving from scratch; no halving after
  install; C1 and "jobs" 100 MP frame after back to 350 / 400 or lower.
- **F2 (open):** the job worker now starts at startup, but the C1 case still needs its test warm-up.
  Measured without the warm-up (final fix wave, time-boxed): the 24 MP fill's gap 263-379 ms is one app
  render of 291-299 ms about 20 ms after the key (the render the tool switch asks for), inside which no
  WebGL call takes over 2 ms and no engine call or renderer stage over 5 ms, so it is neither a shader
  compile nor an upload; runs without the warm-up also showed 219-483 ms main-thread pauses at 100 MP with
  no render or engine call in them. A CDP trace did not reproduce it. Likely a GC or browser-internal pause
  on first use; not identified. Next: a Performance-panel trace with GC events on a fresh profile, or a
  startup warm-up that replays the Fill path on a small hidden document.
- **The 1.3.7 catch-up (next, with F1 as its first task):** the user's Mac runs Compositor 1.3.7, which
  saves project format 11 (this port reads 1-9); a short phase so Windows opens and saves 1.3.7 projects,
  plus a list of 1.3.7's new features for later phases (the user's decisions of 2026-09-29).
- FOLLOW-UP items of the final review's triage:
  - The 100 MP reduced effects window's gap (102-152 ms, within 200) is unattributed: needs a profiler trace.
  - Failed effects jobs are retried on each render (remember failed keys).
  - A document switch restarts running effects jobs (cost only).
  - The effects-worker e2e's reduced placement check is weak.
  - T9-3b: a pixel gradient preview does not mirror the covering mask's growth refusal: a rare preview that
    Return then refuses with "too large"; no data loss.
  - T9's 66 ms mask-drag spike on the HD 520: a flake risk.
  - A spreading filter's preview stretches a covering mask (preview.rs, pre-existing since Phase 3; the
    fix mirrors the blur commit's mask carry, as T9-6's `followed` did for the gradient).
  - Mask levels / display-level mask uploads: masks upload at full resolution; also the path for masks
    wider than 16384 px.
  - Masks wider than 16384 px (the GPU's largest texture): pre-existing for large covering masks, one key
    away on canvases over 16384 px since 14a (display only; export is CPU). Disclosed; fix with chunked or
    reduced mask textures.
  - A uniform-fill fast path in `paint_grid` (a fill costs about what a gradient does), and a same-size
    mask texture recreated on a whole upload (mask-textures.ts; 14a M-1).
  - The native mask Fill's 549-1025 ms swing at 24 MP (allocation or zeroing under memory pressure?): watch.
  - The Sample Ring visibility toggle is not ported (added to OQ19).
  - OQ17's "within 4 levels on edges" needs per-shape tolerances in the probe pinning.
  - `image_grid` stricter than the Mac's tiles (recorded above as a deviation).
  - `alpha_bounds` on a zero-width raster (unreachable width); no serde round trip for `Command::Fill` /
    `Gradient` (cheap if time allows).
- Residuals of the fix wave's re-review (not blocking; first items of the next phase with F1):
  - A layer's pixels or mask chip clicked while a job runs is half-applied: `selectLayers` refuses but
    `setMaskSelected` (no `working` guard) still switches the active layer's target (LayersList.tsx:106-107,
    store.ts:689, :702-706). The Mac refuses a target switch while busy (LayerMask.swift:224). Fix: guard
    `setMaskSelected`, or stop the chip handlers when `selectLayers` refuses.
  - `selectLayers` applies a pending gradient (which may start a job) and still runs `SetActiveLayer`,
    clearing the preview the job keeps on screen until the result lands (store.ts:693-696): re-check
    `working` after `commitGradient`, as `run` does.
  - `jobs.warm()` runs before `installTestApi` and `onFileDrop` in App.tsx:64: a throwing `new Worker`
    would skip the rest of startup. Move it last or wrap it.
  - Non-"closed" effects-image failures are rethrown in a `setTimeout` (effects-images.ts:115) and reach the
    console only; surface them.
  - `working` is global: a job in one document blocks another (per-document busy).
  - A failed histogram job leaves "Reading the histogram..." (rare: worker death).
  - Levels could defer its `jobInput` one frame so the panel paints first.
  - A pure `canUpdate` helper with a unit test for the renderer's whole-upload fallback conditions.
  - `LayerTransform` float equality after a grow and trim (safe: a whole upload).
- Paths still without a timed budget (final review, LL-073):
  - The frame after Undo / Redo of a 24 MP worker result: history forgets halvings of history-only
    buffers, so the undo re-halves 24 MP on the UI thread (61-119 ms, the Task 15 `layerPixels` figure)
    plus a whole upload.
  - The first GL use per browser process (300-440 ms, logged unasserted after the C1 warm-up): F2 above.
  - Gradient Map end previews driven by the picker (they reuse the Phase 3 drag path; not re-timed).
  - The shape commit at 100 MP is timed, but its budget is 2000 ms on the UI thread (OQ11).
  - The 100 MP reduced-image draw interval and the full-window copy were untimed; both are budgeted now.
- The perf suite on this laptop swings between identical runs (a 24 MP first-fill frame 81 then 389 ms; 100
  MP worker-fill gaps 18 then 119-217 ms) with other programs running: a failing case is re-run once and
  both runs reported, never widened.

## The user's decisions of 2026-09-29

- The Mac runs Compositor 1.3.7 (upstream tag v1.3.7, 9e2894e), which saves project format 11; this port
  reads formats 1-9. "Finish 4b-1, then catch up": 1.2.10 stayed the oracle for this plan; right after it, a
  short phase so Windows opens and saves 1.3.7 projects, with F1 as that phase's first task, and a list of
  1.3.7's new features for later phases.
- "1.3.7: grow to canvas": a gradient, and (confirmed later that day) a fill, on a mask grows the mask past
  its layer to the canvas as 1.3.7 does (Task 14a).

## Annex: Task 14a's brief

Task 14a was added during execution (the user's decision above) and is not in the plan. Its brief, as the
planner wrote it from the v1.3.7 source and as the pre-flight audit amended it, follows verbatim.

#### Task 14a: A fill or a gradient on a mask grows the mask past its layer to the canvas (Compositor 1.3.7)

> **Controller amendments (2026-09-29, pre-flight audit preflight-14-14a-15.md)**
> - Audit I-1: the `perf-4b1.spec.ts` hunk gains two trailing context lines (the blank line and the "eyedropper" test's first line) so plain `git apply` places it before the eyedropper test Task 13 added; header recounted.
> - Audit I-5: `mask_grid` reads the area a mask edit paints from `SelectionClip::region` (new, engine/src/selection/coverage.rs), the selection clip's rectangle computed from the selection's bounds without filling the clip, so `editPixels` never rasterizes or feathers the outline on the UI thread. `mask_grid` and `painted_pixels` lose their `clips` argument and `paint_layer_check` keeps its signature. `mask_grow.rs` gains `a_selections_region_is_the_rectangle_its_clip_fills`; Step 5 gains bite (9). The fill perf case runs a second pass under a feathered Select All (the key < 150 ms).
> - Audit I-6 (ruling: budget, no redesign): the fill perf case adds (b) a second Alt+Backspace on the mask already grown, through the worker, with the first fill's budgets, and (a) a nudge (ArrowRight) of the layer whose mask grew, then Undo and Redo, each frame < 150 ms (Task 7's whole-image frame). Over budget is reported, not redesigned. Follow-up (not this task): keep `mask_revision` on a placement-only move (ops/transform.rs:17-18), so a nudge stops re-uploading the mask.
> - Audit I-7: the fill perf case measures frame gaps with `performance.now()` inside the rAF callback and times the frame after the result inside the `installJob` hook (contiguous windows), as the "jobs" case does.
> - Audit M-5: the 100 MP `installJob` budget is 450 ms (ruling I7), not 500.
> - Audit M-4: Step 2 no longer claims `PreviewTarget::Mask { .. }` fails to compile against the tuple variant (Rust accepts it); mask_grow.rs fails at runtime.
> - Audit M-6: `mask-grow.spec.ts`'s `worstGpuVsCpu` returns 256 on a size mismatch, so it can no longer pass `<= 2`.
> - Audit M-7: `Mask::background`'s doc comment says the Mac reads its thumbnail's edge (the same up to 96 px, a disclosed difference past it).
> - Audit M-8: `a_masks_background_...` gains a 3 x 3 (black corners) and a 1 x 4 (white ends) case, which a double count bites; Step 5 gains bite (10).
> - Audit I-3: "Consequences for later briefs", Task 16: its perf hunk is amended (trailing context), not "No change". Task 17 gains the OQ7 docs line (audit M-10).

The user's Mac runs Compositor 1.3.7, and for this behaviour the user chose it over the plan's 1.2.10 oracle (USER DECISIONS 2026-09-29, "follow 1.3.7: grow to canvas"): a Fill or a Gradient on a targeted mask paints the whole canvas (or the selection on it), growing the mask past its layer. Both of the Mac's callers pass `growsMask: true`: `beginGradient` (Gradient.swift:48 at v1.3.7) and `applyPixelEdit` (SelectionEdits.swift:200-210), which serves Fill with the foreground or background colour (`fillSelection`, :40-50) and, on a mask, Delete with a selection (`clearSelectedPixels` fills the mask with the background colour, :54-59). In this port by the end of Task 14 those are all one engine command, `Fill` / `Gradient` with `mask: true` (Task 14 turns Delete on a targeted mask into a `Fill` with the mask's background colour), so every mask edit through `paint_layer` grows. The mask's grid is widened to cover the canvas as that grid maps it, rounded out (BrushStroke.swift:168); the new area starts as the mask's background, white or black, whichever most of its edge is (:169-170, :585-589; LayerMask.swift:61-76); the result keeps the old grid joined with every 256-pixel tile the edit painted, counted from the widened grid's corner (:576-583, :658-670, :764); and the mask takes its own placement on the document when it grew or already had one (EditorSession+Brush.swift:180-187), still moving with a linked layer (LayerMask.swift:54-59). A solid mask on its own placement (at most 2 x 2) is painted at one pixel per document pixel over its place (BrushStroke.swift:160-165). The mask budget is checked at the grown size (EditorSession+Brush.swift:16-20; BrushStroke.swift:580-582). One function, `ops::raster_edit::mask_grid`, computes the grid, the placement and the refusals for the commit, the preview and the size the app decides the job worker by: Task 14's `Engine::edit_pixels` (ruling C1) now counts a mask at its grown size, so a Fill or a Gradient on a small layer's mask on a 100 MP canvas goes to the worker. The preview shows the grown mask, reduced as before, at its new placement (`PreviewTarget::Mask { pixels, placement }`); the GPU and the CPU read the same displayed document, so they agree, and the lineage uploads a mask whose grid changed whole. `Mask::background` reads only the mask's edge now: a grown mask can hold 100 MP, and the plan and `state()` ask for it on every frame.

**Files:**
- Modify: `engine/src/ops/raster_edit.rs` (`MaskGrid`, `mask_grid`, `painted_pixels`, `mask_on_grid`, `layer_grid`, `rect_on`, `TILE` replace `mask_grow_check` and `mask_on_layer_grid`; `paint_layer`'s mask branch), `engine/src/selection/coverage.rs` (`SelectionClip::region`, audit I-5), `engine/src/preview.rs` (`PreviewTarget::Mask { pixels, placement }`, the mask branch of `gradient_preview`), `engine/src/engine.rs` (Task 14's `Engine::edit_pixels` counts through `painted_pixels`; `displayed` shows the preview mask's placement), `engine/src/document.rs` (`Mask::background` reads the edge alone)
- Create tests: `engine/tests/mask_grow.rs`, `app/tests/e2e/mask-grow.spec.ts`; modify `engine/tests/gradient_preview.rs` (the `Mask` pattern), `engine/tests/perf_4b1.rs`, `app/tests/e2e/perf-4b1.spec.ts`

**Interfaces:**
- Consumes: Task 8's `paint_layer`, `paint_grid`, `image_grid`, `check_target`, `ops::adjust::edit_coverage`, `ops::adjust::placed_like`, `Document::used_mask_pixels`, `MAX_PIXELS`, `MAX_SIDE`; Task 9's `gradient_preview`, `level_for`, `GRADIENT_DRAG_LIMIT`, `GRADIENT_SETTLED_LIMIT`, `PixelPreview`; Task 3's `Lineage::record_edit` / `record_return` (a mask whose size or placement grid changed is recorded whole, lineage.rs:55-56, :71); Task 5's `run_edit_job` / `install_job` (the mask's placement travels in `JobOutput.mask_placement`, jobs.rs:214, :234); Task 14's `Engine::edit_pixels`, `EngineClient.editPixels` and `fillActive` exactly as "Consequences for later briefs" (Task 14) specifies them (ruling C1).
- Produces: `ops::raster_edit::{MaskGrid, mask_grid(doc, id, grows), painted_pixels(doc, id, mask)}`; `SelectionClip::region(selection, canvas_width, canvas_height)` (the clip's rectangle without filling the clip, audit I-5); `paint_layer_check(doc, id, mask, paint)` keeps its signature; `PreviewTarget::Mask { pixels, placement }` (was the tuple `Mask(GrayRaster)`); `Engine::edit_pixels` counting a mask at its grown size (the error where the edit is refused for its size included). Task 15 decides the Gradient's job worker by `editPixels` too (see "Consequences for later briefs").

The Mac rules, as read from `git show v1.3.7:<path>` in Compositor-1.2.10 (line numbers at v1.3.7):
- Which edits grow a mask: every raster edit made with `growsMask: true`: `beginGradient` (Gradient.swift:48) and `applyPixelEdit` (SelectionEdits.swift:200-210, "On a mask, a fill covers the whole canvas, past the mask's own area, as the brush can", commit e753f26 "The brush, the gradient and fills on a mask now reach past the layer's pixels"; the brush too, EditorSession+Brush.swift:54, not in this port). `applyPixelEdit` serves `fillSelection` (Fill with Foreground / Background Color, CompositorApp.swift:196-207; SelectionEdits.swift:40-50, undo name "Fill Mask") and `clearSelectedPixels` (:54-59), which on a mask is `fillSelection(with: .background)` (:56): Delete with a selection (:63-66), Edit > Clear Selection Pixels (CompositorApp.swift:208), Cut (SelectionClipboard.swift:135) and a floating selection's lift (FloatingSelection.swift:37). In this port by the end of Task 14: Fill (Alt/Ctrl+Backspace, Edit > Fill with Foreground / Background Color) and Delete with a selection on a targeted mask, all sent as `Fill { mask: true }`; there is no Clear Selection Pixels item, and the clipboard and floating selections are 4b-2. `beginGradient` is `Gradient { mask: true }` (Task 15). Not raster edits, so not grown on the Mac either: Invert (`invertPixels`, SelectionEdits.swift:90-127) and mask transforms.
- Grid (BrushStroke.swift:157-167): a mask on its own placement is painted in its own pixel grid (`placedMask`, :158), else in the layer's (`base = placedMask?.1 ?? layer.transform`, :159; a covering mask's source image, 1 x 1 or any size, is drawn stretched over the layer's grid, :590-613). A placed mask at most 2 x 2 is `solidPlaced` (:161): its grid is `max(1, round(base.size))` pixels (:162-165), one per document pixel over its place. The init refuses a grid side over `maxSide` = 30000 (:182; DocumentLimits.swift:18).
- Extent (:168): `mask && !growsMask ? originalBounds : originalBounds.union(canvas.applying(originalMapping.inverted()).integral)`: in the mask's own grid, so a rotated, scaled or flipped layer's covering mask widens along its own axes (`originalMapping = pixelToDocument(base, w, h)`, :166), exactly as `image_grid` widens a layer.
- Background (:169-170): `maskBackground = LayerMask.background(of: mask.asset.thumbnail)`: 1 (white) when the edge pixels of the mask's thumbnail (at most 96 px on its long side, LayerMask.swift:37-47) sum to at least half of 255 per pixel (`total * 2 >= count * 255`, LayerMask.swift:63-76), else 0. A new tile is filled with it before the old mask is drawn in (:585-589); the commit's canvas too (`BrushCommit.render`, :888-893, `Input.fill`, :870-871, set by `commitInput`, :857). `RasterSnapshot.replacing(..., fill: maskBackground)` (:847-848; RasterSnapshot.swift in e753f26) is the brush's `paintSnapshot` path; the Gradient and a Fill commit through `commitRasterEdit` (Gradient.swift:99; SelectionEdits.swift:207), which uses `commitInput` and `render`.
- Bounds (:576-583, :658-670, :764): `paintCanvas` paints `area = canvas` (intersected with `selectionClip.rect`, the selection's coverage box grown by a pixel, rounded out, cut to the canvas, Selection.swift:39-49), maps it into the widened grid, rounds out (`integral`), cuts it to the grid, and allocates every 256 x 256 tile (`tileSize`, :144) over it, counted from the widened grid's corner; `allocateTile` keeps `allocatedBounds = sourceRect.union(tile rects)` (a mask always has a source). `committedBounds = (allocatedBounds ?? sourceRect).integral` (:764); `render` returns a mask at its full `committedBounds` (:899-900, `pixelBounds: fullBounds`). So without a selection the mask becomes the whole widened grid; with one, the old grid joined with the tiles the selection's clip touches.
- Commit (EditorSession+Brush.swift:160-200): `transform(for: committedBounds)` (:167; BrushStroke.swift:766-773) must be valid, else TooLarge (:163, :168); the mask is `mask.replacing(asset)` and takes `placement = transform` when `mask.placement != nil || bounds != stroke.sourceRect` (:180-187), i.e. when it was placed or grew. `replacing` keeps `isEnabled`, `placement` and `isLinked` (LayerMask.swift:20-22). "A linked one still moves with its layer" is `LayerMask.placement(movingLayer:to:)` (LayerMask.swift:54-59): linked, an explicit placement is carried by `following(from:to:)`; unlinked, the mask stays where it was. The port's `Mask::follow` (document.rs:185-189) is that rule already.
- Budget: `makeRasterEdit` with the mask targeted sets `pixelLimit = documentPixelBudget - (other layers' mask pixels)` (EditorSession+Brush.swift:16-20); `allocateTile` refuses with TooLarge when the joined bounds pass `maxSideExtent` on a side or `pixelLimit` in area (BrushStroke.swift:580-582). The refusal comes at the first tile: for a Gradient at the first preview (`refreshGradient` cancels the gradient and shows the error, Gradient.swift:61-68), for a Fill before anything is committed (`applyPixelEdit` shows the error, SelectionEdits.swift:209). A Fill's `paintCanvas` (`fill`, BrushStroke.swift:639-644) allocates the same tiles as a Gradient's, so the extent, background, tiles, placement and budget are the same for both.

- [ ] **Step 1: Write the tests**

The engine tests work out every expected value from the rules above (the ramp, the background, the tile, the flipped grid's centres, the solid grid's rounding, the budget's room): a gradient grows a 20 x 10 layer's mask to its 100 x 40 canvas, places it over the canvas, follows a linked layer, and undoes back to the old mask and no placement; the grown area is the background (white edges reveal, black edges hide); inside a selection the mask keeps the old grid and one whole tile; a scaled, flipped layer's mask grows along its own grid; a solid placed mask is painted at one pixel per document pixel over its place, and grows from that grid; a Fill without a selection grows the mask to the whole grown grid (a layer overhanging the canvas: the canvas painted, the old mask kept past the canvas, the background in the corner neither covers); a Fill inside a selection grows it by exactly the tiles the selection's clip touches; the budget refuses a Fill and a Gradient at the grown size (the preview shows nothing, `edit_pixels` refuses too); the preview is the commit (level 0) and shows the placement, reduced too; the job worker leaves exactly what the commit leaves; `Mask::background` counts each edge pixel once; a selection's region (`SelectionClip::region`) is exactly the rectangle its clip fills, feathered, part off the canvas or wholly off it (audit I-5). The e2e: a mask gradient on a 40 x 30 layer on 120 x 80, previewed, applied, moved and undone, the GPU within 2 of the CPU at every pixel and the layer's alpha the formula's; then Alt+Backspace and Delete in a selection on the targeted mask grow it to the canvas through the keys.

Create `engine/tests/mask_grow.rs`:

```rust
//! Fills and gradients on a mask grow the mask past its layer to the canvas (Task 14a): Compositor
//! 1.3.7's rule, chosen by the user on 2026-09-29 over the plan's 1.2.10 oracle (Gradient.swift:48;
//! SelectionEdits.swift:200-210; BrushStroke.swift:157-173, :576-589, :639-670, :764-773;
//! EditorSession+Brush.swift:13-20, :180-187; LayerMask.swift:54-76; all at v1.3.7). Every expected value
//! is worked out here from those rules, never pasted.
use compositor_engine::*;
use uuid::Uuid;

fn run(e: &mut Engine, id: Uuid, c: Command) { e.execute(id, c).unwrap_or_else(|err| panic!("{err}")); }
fn p(x: f64, y: f64) -> Point { Point { x, y } }
fn depth(e: &Engine, id: Uuid) -> usize { e.state(id).unwrap().undo_depth }
fn select(e: &mut Engine, id: Uuid, x: f64, y: f64, w: f64, h: f64) {
    run(e, id, Command::SelectShape { kind: SelectionShape::Rectangle, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true });
}
fn linear(start: Point, end: Point, from: [f64; 4], to: [f64; 4]) -> GradientSpec {
    GradientSpec { shape: GradientShape::Linear, start, end, from, to, opacity: 1.0 }
}
const BLACK: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f64; 4] = [1.0, 1.0, 1.0, 1.0];
const CLEAR: [f64; 4] = [0.0, 0.0, 0.0, 0.0];

/// An upright transform over (`origin`) of `size` document pixels.
fn at(origin: (f64, f64), size: (f64, f64)) -> LayerTransform { LayerTransform::axis_aligned(p(origin.0, origin.1), Size { width: size.0, height: size.1 }) }
/// A mask of its layer's grid (no placement, linked), `value(x, y)` at each pixel.
fn covering(w: u32, h: u32, value: impl Fn(u32, u32) -> u8) -> Mask {
    let data = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| value(x, y)).collect();
    Mask { pixels: GrayRaster::from_bytes(w, h, data), enabled: true, placement: None, linked: None }
}
/// A `canvas` document whose last layer, active, is `size` opaque blue pixels placed by `transform` under
/// `mask`; with `hog`, a folder below it whose mask holds that many of the mask budget's pixels (a
/// folder has no pixels, so the pixel budget is untouched).
fn document(canvas: (u32, u32), size: (u32, u32), transform: LayerTransform, mask: Mask, hog: Option<(u32, u32)>) -> (Engine, Uuid, Uuid) {
    let mut doc = Document::new(canvas.0, canvas.1);
    let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(size.0, size.1, [0, 0, 200, 255].repeat((size.0 * size.1) as usize)), p(0.0, 0.0));
    layer.transform = transform;
    layer.mask = Some(mask);
    let lid = layer.id;
    let mut layers = Vec::new();
    if let Some((w, h)) = hog {
        let mut big = Layer::blank("Big Folder", doc.size());
        big.is_group = true;
        big.mask = Some(Mask { pixels: GrayRaster::from_bytes(w, h, vec![255u8; (w * h) as usize]), enabled: true, placement: None, linked: None });
        layers.push(big);
    }
    layers.push(layer);
    doc.active_layer_id = Some(lid);
    doc.layers = layers;
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    (e, id, lid)
}
fn mask_of(e: &Engine, id: Uuid) -> Mask { e.document(id).unwrap().layers.last().unwrap().mask.clone().unwrap() }
fn value(m: &Mask, x: u32, y: u32) -> u8 { m.pixels.bytes()[(y * m.pixels.width + x) as usize] }
fn shown(e: &Engine, id: Uuid, x: u32, y: u32) -> [u8; 4] { e.composite(id, Rect { x: x as f64, y: y as f64, width: 1.0, height: 1.0 }, 1, 1).unwrap().pixel(0, 0) }

#[test]
fn a_mask_gradient_grows_the_mask_to_the_canvas_places_it_there_and_undoes() {
    // A 20 x 10 layer at (30, 15) on 100 x 40, one document pixel a pixel, under a mask of its own grid
    // whose left half is black. The mask's grid is the layer's; the canvas on it runs from (-30, -15) to
    // (70, 25), so the grown grid is the canvas itself: 100 x 40, the old grid at (30, 15) in it.
    let half = covering(20, 10, |x, _| if x < 10 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), half.clone(), None);
    // What the app decides the job worker by (ruling C1): the mask grown to the canvas, not its 200 pixels.
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 100 * 40);
    let count = depth(&e, id);
    // Black to white from x 0.5 to 100.5: the pixel at column i (centre i + 0.5) is i / 100 along.
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, WHITE) });
    assert_eq!(depth(&e, id), count + 1);
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (100, 40), "grown to the canvas");
    let ramp = |i: u32| (255.0 * i as f64 / 100.0).round() as u8;
    // Past the layer on every side, and over its old black half (37) and white half (45).
    for (x, y) in [(0, 0), (1, 39), (37, 20), (45, 20), (99, 5)] { assert_eq!(value(&m, x, y), ramp(x), "({x}, {y})"); }
    let placed = m.placement.expect("placed on its own: it grew (EditorSession+Brush.swift:185)");
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 100.0, height: 40.0 }), "over the canvas, a document pixel a pixel");
    assert!(m.is_linked());
    // The layer shows through the grown mask where the layer is.
    assert_eq!(shown(&e, id, 37, 20)[3], ramp(37));
    // Linked, it moves with its layer (LayerMask.swift:54-59): 5 right and 3 down.
    run(&mut e, id, Command::SetLayerTransform { id: layer, transform: at((35.0, 18.0), (20.0, 10.0)) });
    assert_eq!(mask_of(&e, id).placement.unwrap().origin, p(5.0, 3.0));
    // Undone: the old mask, covering its layer again.
    e.undo(id).unwrap();
    e.undo(id).unwrap();
    let back = mask_of(&e, id);
    assert_eq!((back.pixels.bytes(), back.placement), (half.pixels.bytes(), None));
    assert_eq!(depth(&e, id), count);
}

#[test]
fn the_grown_area_starts_as_the_masks_background_white_or_black() {
    // Black fading to nothing from x 0.5 to 100.5: at column i the black's alpha is 1 - i / 100, over
    // what the mask held there, which where it grew is its background: background x i / 100, rounded.
    let g = linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, CLEAR);
    let over = |under: f64, i: u32| (under * i as f64 / 100.0 + 0.5).floor() as u8;
    // White edges: the background reveals (LayerMask.background, LayerMask.swift:61-76).
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(20, 10, |_, _| 255), None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g.clone() });
    let m = mask_of(&e, id);
    for i in [5, 20, 80, 95] { assert_eq!(value(&m, i, 3), over(255.0, i), "white background, column {i}"); }
    // Black edges round a white middle: the background hides; the old middle is still white under it.
    let ring = covering(20, 10, |x, y| if x == 0 || y == 0 || x == 19 || y == 9 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), ring, None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g });
    let m = mask_of(&e, id);
    for i in [5, 20, 80, 95] { assert_eq!(value(&m, i, 3), 0, "black background, column {i}"); }
    // Column 40, row 20 is the layer's pixel (10, 5), in its white middle.
    assert_eq!(value(&m, 40, 20), over(255.0, 40));
}

#[test]
fn inside_a_selection_the_mask_keeps_its_old_grid_and_the_whole_tiles_the_edit_touched() {
    // A 100 x 100 layer at (450, 250) on 1000 x 600, under a white mask of its own grid. Grown, its grid
    // would be the canvas: 1000 x 600 from (-450, -250). A selection from (50, 50) to (150, 110), its clip
    // a pixel wider, lies well inside the first 256 x 256 tile counted from that corner (BrushStroke.swift:
    // 666-670): the mask keeps the old grid joined with that tile, from (-450, -250) to (100, 100) in the
    // layer's grid: 550 x 350, placed over (0, 0) to (550, 350).
    let (mut e, id, layer) = document((1000, 600), (100, 100), at((450.0, 250.0), (100.0, 100.0)), covering(100, 100, |_, _| 255), None);
    select(&mut e, id, 50.0, 50.0, 100.0, 60.0);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 550 * 350);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.0, 0.0), p(1000.0, 0.0), BLACK, BLACK) });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (550, 350));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 550.0, height: 350.0 }));
    assert_eq!(value(&m, 100, 80), 0, "painted inside the selection");
    assert_eq!(value(&m, 200, 200), 255, "the background, in the tile past the selection");
    assert_eq!(value(&m, 300, 100), 255, "the background, past the tile and the old grid");
    assert_eq!(value(&m, 500, 300), 255, "the old mask, unpainted");
}

#[test]
fn a_scaled_and_flipped_layers_mask_grows_to_the_canvas_along_its_own_grid() {
    // A 20 x 10 layer drawn at twice its size over (30, 10)-(70, 30), flipped both ways, on 100 x 40: a
    // mask pixel is 2 x 2 document pixels, and the grid runs right to left and bottom to top. Its pixel
    // (u, v) has its centre at x = 50 - 2 (u + 0.5 - 10), y = 20 - 2 (v + 0.5 - 5), so the canvas on the
    // grid runs from u = -15 to 35 and v = -5 to 15 (BrushStroke.swift:168): grown, 50 x 20 with the old
    // grid at (15, 5), placed over the canvas (100 x 40 at (0, 0)) and flipped as the layer is. The grown
    // pixel i then has its centre at x = 50 - 2 (i - 15 + 0.5 - 10) = 99 - 2 i.
    let mut t = at((30.0, 10.0), (40.0, 20.0));
    t.flip_x = true;
    t.flip_y = true;
    let (mut e, id, layer) = document((100, 40), (20, 10), t, covering(20, 10, |_, _| 255), None);
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: linear(p(0.0, 20.0), p(100.0, 20.0), BLACK, WHITE) });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (50, 20));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size, placed.flip_x, placed.flip_y), (p(0.0, 0.0), Size { width: 100.0, height: 40.0 }, true, true));
    for i in [0u32, 10, 24, 49] {
        let x = 99.0 - 2.0 * i as f64;
        assert_eq!(value(&m, i, 7), (255.0 * x / 100.0 + 0.5).floor() as u8, "column {i}, centred at x {x}");
    }
}

#[test]
fn a_solid_mask_on_its_own_placement_is_painted_at_one_pixel_per_document_pixel() {
    // A 1 x 1 white mask placed on its own over (10, 5)-(40.4, 25), on a 100 x 40 layer over its canvas.
    // Painted, its grid is round(30.4) x 20 = 30 x 20 pixels over its place (BrushStroke.swift:160-165),
    // where 1.2.10 painted its single pixel; a fill then grows that grid to the canvas: x from
    // (0 - 10) x 30 / 30.4 to (100 - 10) x 30 / 30.4, rounded out, and y from -5 to 35 (the canvas is less
    // than a tile across, so a selection's fill grows it all the same). The old grid lands at (10, 5).
    let place = at((10.0, 5.0), (30.4, 20.0));
    let mut solid = covering(1, 1, |_, _| 255);
    solid.placement = Some(place);
    solid.linked = Some(false);
    let (mut e, id, layer) = document((100, 40), (100, 40), at((0.0, 0.0), (100.0, 40.0)), solid, None);
    // The left half of the place selected: the old grid's pixels are 30.4 / 30 wide, so its column 2
    // (centre x 12.5) is inside x < 25 and its column 27 (centre x 37.9) outside.
    select(&mut e, id, 10.0, 5.0, 15.0, 20.0);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    let (x0, x1) = ((-10.0f64 * 30.0 / 30.4).floor(), (90.0f64 * 30.0 / 30.4).ceil());
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), ((x1 - x0) as u32, 40));
    let placed = m.placement.unwrap();
    let unit = 30.4 / 30.0;
    assert!((placed.size.width - (x1 - x0) * unit).abs() < 1e-9 && placed.size.height == 40.0, "{placed:?}");
    assert!((placed.origin.x - (10.0 + x0 * unit)).abs() < 1e-9 && placed.origin.y.abs() < 1e-9, "{placed:?}");
    // Old column 2, row 10 (selected), old column 27 (not), and old column -7 (grown, the background).
    assert_eq!((value(&m, 12, 15), value(&m, 37, 15), value(&m, 3, 15)), (0, 255, 255));
}

#[test]
fn a_fill_on_a_mask_without_a_selection_grows_it_to_the_whole_grown_grid() {
    // SelectionEdits.swift:200-210: a fill on a mask covers the whole canvas past the mask's own area. A
    // 20 x 10 layer at (90, 35) overhangs the 100 x 40 canvas by 10 and 5, under a covering 1 x 1 mask of
    // 128 (its background: 128 x 2 >= 255, white). On the layer's grid the canvas runs from (-90, -35) to
    // (10, 5); joined with the old grid, (-90, -35) to (20, 10): 110 x 45, the old grid at (90, 35), placed
    // over (0, 0) to (110, 45). Black paints every pixel whose centre is on the canvas; the old grid keeps
    // its 128 past the canvas; the corner that is neither, right of x 100 and above y 35, is the background.
    let (mut e, id, layer) = document((100, 40), (20, 10), at((90.0, 35.0), (20.0, 10.0)), covering(1, 1, |_, _| 128), None);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 110 * 45);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (110, 45));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(0.0, 0.0), Size { width: 110.0, height: 45.0 }));
    assert_eq!((value(&m, 5, 5), value(&m, 95, 38)), (0, 0), "on the canvas");
    assert_eq!((value(&m, 105, 38), value(&m, 95, 42)), (128, 128), "the old mask past the canvas");
    assert_eq!(value(&m, 105, 10), 255, "neither: the background");
    assert_eq!(Command::Fill { id: layer, mask: true, color: [0.0; 3] }.action_name(), "Fill Mask");
}

#[test]
fn a_fill_on_a_mask_inside_a_selection_grows_it_by_the_tiles_the_clip_touches() {
    // A 100 x 100 layer at (450, 250) on 1000 x 600 under a covering black 1 x 1 mask (a hide-all mask:
    // its background is black). Grown, its grid would run from (-450, -250) to (550, 350); tiles count from
    // that corner. A selection from (600, 400) to (700, 450), its clip (599, 399)-(701, 451) a pixel wider,
    // is (149, 149)-(251, 201) on the layer's grid, (599, 399)-(701, 451) from the corner: tile columns 2
    // (512-768) and row 1 (256-512), so the tiles reach (62, 6)-(318, 262) on the layer's grid. Joined with
    // the old grid, (0, 0)-(318, 262): the mask grows right and down only, 318 x 262, the old grid at
    // (0, 0), placed at (450, 250).
    let (mut e, id, layer) = document((1000, 600), (100, 100), at((450.0, 250.0), (100.0, 100.0)), covering(1, 1, |_, _| 0), None);
    select(&mut e, id, 600.0, 400.0, 100.0, 50.0);
    assert_eq!(e.edit_pixels(id, layer, true).unwrap(), 318 * 262);
    run(&mut e, id, Command::Fill { id: layer, mask: true, color: [1.0, 1.0, 1.0] });
    let m = mask_of(&e, id);
    assert_eq!((m.pixels.width, m.pixels.height), (318, 262));
    let placed = m.placement.unwrap();
    assert_eq!((placed.origin, placed.size), (p(450.0, 250.0), Size { width: 318.0, height: 262.0 }));
    assert_eq!(value(&m, 200, 175), 255, "painted inside the selection");
    assert_eq!(value(&m, 300, 250), 0, "the background, in the tiles past the selection");
    assert_eq!(value(&m, 50, 50), 0, "the old mask, unpainted");
}

#[test]
fn a_mask_fill_or_gradient_is_refused_past_the_mask_budget_at_its_grown_size() {
    // The same 20 x 10 layer at (30, 15) and mask; a folder's mask holds 99,999,000 of the mask budget,
    // leaving 1,000 besides this mask's own 200 (EditorSession+Brush.swift:16-20): room for the mask as it
    // is, not for the mask grown to the canvas, 100 x 40 = 4,000 (BrushStroke.swift:582). The canvas is
    // less than a tile across, so a selection's fill grows it all the same.
    let (hog_w, hog_h) = (99_999u32, 1_000u32);
    let room = MAX_PIXELS - hog_w as u64 * hog_h as u64;
    assert!(100 * 40 > room && 20 * 10 <= room, "the fixture must starve only the grown mask");
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(20, 10, |_, _| 255), Some((hog_w, hog_h)));
    let g = linear(p(0.0, 0.0), p(100.0, 0.0), BLACK, WHITE);
    let too_large = || CommandError::Project(ProjectError::TooLarge);
    let (before, count) = (e.document(id).unwrap().clone(), depth(&e, id));
    assert_eq!(e.execute(id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] }), Err(too_large()));
    assert_eq!(e.execute(id, Command::Gradient { id: layer, mask: true, gradient: g.clone() }), Err(too_large()));
    select(&mut e, id, 32.0, 17.0, 5.0, 5.0);
    let (before_selected, count_selected) = (e.document(id).unwrap().clone(), depth(&e, id));
    assert_eq!(e.execute(id, Command::Fill { id: layer, mask: true, color: [0.0, 0.0, 0.0] }), Err(too_large()));
    assert!(e.document(id).unwrap().same_content(&before_selected));
    assert_eq!(depth(&e, id), count_selected);
    // The preview shows nothing, and the size the app decides the worker by refuses too.
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g, dragging: true })).unwrap();
    assert!(e.preview(id).is_none());
    assert_eq!(e.edit_pixels(id, layer, true), Err(too_large()));
    run(&mut e, id, Command::Deselect);
    assert!(e.document(id).unwrap().same_content(&before));
    assert_eq!(depth(&e, id), count + 2, "the selection and its removal, nothing else");
}

#[test]
fn the_preview_shows_the_grown_mask_where_the_commit_leaves_it() {
    // Small enough not to be reduced (under GRADIENT_DRAG_LIMIT): the previewed mask is the committed
    // one byte for byte, at the same placement.
    let pattern = |w: u32, h: u32| covering(w, h, |x, y| ((x * 7 + y * 13) % 256) as u8);
    let (mut e, id, layer) = document((300, 200), (60, 40), at((100.0, 80.0), (60.0, 40.0)), pattern(60, 40), None);
    let g = GradientSpec { shape: GradientShape::Radial, start: p(130.0, 100.0), end: p(230.0, 150.0), from: BLACK, to: CLEAR, opacity: 0.8 };
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g.clone(), dragging: true })).unwrap();
    assert!(matches!(e.preview(id).unwrap().target, PreviewTarget::Mask { .. }));
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (300, 200), "grown to the canvas, not reduced");
    let previewed = (e.mask_pixels(id, layer).unwrap().unwrap().bytes().to_vec(), state.mask_placement);
    let through = shown(&e, id, 120, 90);
    e.set_preview(id, None).unwrap();
    assert_eq!(e.state(id).unwrap().layers[0].mask_placement, None, "taken back: covering its layer again");
    run(&mut e, id, Command::Gradient { id: layer, mask: true, gradient: g.clone() });
    let m = mask_of(&e, id);
    assert_eq!(previewed, (m.pixels.bytes().to_vec(), m.placement));
    assert_eq!(shown(&e, id, 120, 90), through);
    // Reduced: the same layer on 3000 x 2000 grows to 3000 x 2000, shown while dragged at 750 x 500
    // (halved until at most 1024 across) over the whole canvas.
    let (mut e, id, layer) = document((3000, 2000), (60, 40), at((1000.0, 800.0), (60.0, 40.0)), pattern(60, 40), None);
    e.set_preview(id, Some(PreviewRequest::Gradient { layer, mask: true, gradient: g, dragging: true })).unwrap();
    let state = e.state(id).unwrap().layers[0].clone();
    assert_eq!((state.mask_width, state.mask_height), (750, 500));
    assert_eq!(state.mask_placement, Some(at((0.0, 0.0), (3000.0, 2000.0))));
}

#[test]
fn a_mask_gradient_through_a_job_leaves_what_it_leaves_in_place() {
    let half = covering(20, 10, |x, _| if x < 10 { 0 } else { 255 });
    let (mut e, id, layer) = document((100, 40), (20, 10), at((30.0, 15.0), (20.0, 10.0)), half, None);
    let command = Command::Gradient { id: layer, mask: true, gradient: linear(p(0.5, 20.0), p(100.5, 20.0), BLACK, CLEAR) };
    let (input, pixels, mask, points) = e.job_input(id, layer).unwrap();
    let (output, new_pixels, new_mask) = run_edit_job(&input, pixels, mask, points.as_deref(), command.clone()).unwrap();
    assert_eq!(output.mask, Some((100, 40)));
    assert_eq!(output.mask_placement.map(|t| (t.origin, t.size)), Some((p(0.0, 0.0), Size { width: 100.0, height: 40.0 })));
    e.install_job(id, layer, input.stamp, output, new_pixels, new_mask).unwrap();
    let through_job = mask_of(&e, id);
    e.undo(id).unwrap();
    run(&mut e, id, command);
    assert_eq!(mask_of(&e, id), through_job);
}

#[test]
fn a_masks_background_is_the_majority_of_its_edge_pixels_each_counted_once() {
    // `Mask::background` reads only the edge (a grown mask is canvas-sized, and the plan asks for it every
    // frame): each edge pixel counted once, white when their mean is at least half of 255.
    let by_definition = |m: &GrayRaster| {
        let (w, h) = (m.width, m.height);
        let edge: Vec<u64> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| x == 0 || y == 0 || x == w - 1 || y == h - 1)
            .map(|(x, y)| m.bytes()[(y * w + x) as usize] as u64).collect();
        if edge.iter().sum::<u64>() * 2 >= edge.len() as u64 * 255 { 255u8 } else { 0 }
    };
    // A 6 x 5 whose edge is exactly half white only when every edge pixel counts once: the top row and
    // the right column black, the bottom row and the left column white (18 edge pixels, 9 white).
    let half = covering(6, 5, |x, y| if y == 4 || (x == 0 && y > 0) { 255 } else { 0 });
    // A 3 x 3 whose four edge middles are white and four corners black (exactly half: white) turns
    // black if a corner is counted twice; a 1 x 4 with white ends and a black middle (half: white)
    // turns black if a one-pixel column's middle is counted twice (audit M-8).
    let cases = [covering(1, 1, |_, _| 127), covering(1, 1, |_, _| 128), covering(5, 1, |x, _| if x < 2 { 255 } else { 0 }),
        covering(1, 4, |_, y| if y < 3 { 255 } else { 0 }), covering(2, 2, |x, y| if x == y { 255 } else { 0 }), half.clone(),
        covering(3, 3, |x, y| if (x == 1) != (y == 1) { 255 } else { 0 }), covering(1, 4, |_, y| if y == 0 || y == 3 { 255 } else { 0 })];
    for m in &cases { assert_eq!(m.background(), by_definition(&m.pixels), "{} x {}", m.pixels.width, m.pixels.height); }
    assert_eq!(half.background(), 255);
}

#[test]
fn a_selections_region_is_the_rectangle_its_clip_fills() {
    // Pre-flight audit I-5: a mask edit reads where it paints from `SelectionClip::region` (the
    // selection's bounds grown by a pixel, rounded out and cut to the canvas), never filling the clip
    // on the UI thread; it must be exactly the rectangle `SelectionClip::new` fills, and None where
    // that has no coverage. Inside the canvas at fractional edges, feathered, part off the canvas, and
    // wholly off it.
    let (mut e, id, _) = document((300, 200), (20, 10), at((30.0, 15.0), (20.0, 10.0)), covering(1, 1, |_, _| 255), None);
    let shape = |kind: SelectionShape, x: f64, y: f64, w: f64, h: f64| Command::SelectShape { kind, points: vec![p(x, y), p(x + w, y), p(x + w, y + h), p(x, y + h)], mode: SelectionMode::Replace, antialiased: true };
    let mut seen = (0, 0);
    for (select, feather) in [(shape(SelectionShape::Rectangle, 10.5, 20.25, 100.0, 50.0), 0), (shape(SelectionShape::Ellipse, 40.0, 30.0, 120.0, 90.0), 12),
        (shape(SelectionShape::Rectangle, 250.0, -40.0, 120.0, 100.0), 0), (shape(SelectionShape::Rectangle, 400.0, 300.0, 50.0, 50.0), 0)] {
        run(&mut e, id, select.clone());
        if feather > 0 { run(&mut e, id, Command::FeatherSelection { amount: feather }); }
        let doc = e.document(id).unwrap();
        let s = doc.selection.as_ref().expect("a selection, maybe off the canvas");
        let clip = SelectionClip::new(s, doc.width, doc.height);
        let filled = clip.coverage.as_ref().map(|k| (clip.origin.0, clip.origin.1, clip.origin.0 + k.width as i64, clip.origin.1 + k.height as i64));
        assert_eq!(SelectionClip::region(s, doc.width, doc.height), filled, "{select:?}, feathered {feather}");
        if filled.is_some() { seen.0 += 1 } else { seen.1 += 1 }
    }
    assert_eq!(seen, (3, 1), "three selections reach the canvas, one does not");
}
```

Modify `engine/tests/gradient_preview.rs` (the target's `Mask` variant has fields now):

```diff
--- a/engine/tests/gradient_preview.rs
+++ b/engine/tests/gradient_preview.rs
@@ -173,7 +173,7 @@ fn a_non_uniform_covering_mask_of_a_different_size_gathers_as_the_commit_does() {
 
     let g = GradientSpec { shape: GradientShape::Linear, start: p(0.0, 0.0), end: p(200.0, 0.0), from: [0.0, 0.0, 0.0, 1.0], to: [1.0, 1.0, 1.0, 1.0], opacity: 1.0 };
     preview(&mut e, doc_id, id, true, &g, true);
-    assert!(matches!(e.preview(doc_id).unwrap().target, PreviewTarget::Mask(_)));
+    assert!(matches!(e.preview(doc_id).unwrap().target, PreviewTarget::Mask { .. }));
     let state = e.state(doc_id).unwrap().layers[0].clone();
     assert_eq!((state.mask_width, state.mask_height), (200, 150), "the canvas is small: not reduced");
     let previewed = e.mask_pixels(doc_id, id).unwrap().unwrap().bytes().to_vec();
```

Create `app/tests/e2e/mask-grow.spec.ts`:

```ts
import { test, expect, type Page } from "@playwright/test";
import { solidPngBase64 } from "./helpers";

// Task 14a: a gradient on a mask grows the mask past its layer to the canvas (Compositor 1.3.7, the
// user's decision of 2026-09-29; engine/tests/mask_grow.rs pins the engine). The GPU must draw what the
// CPU compositor draws while it is previewed, once applied, once its layer moves, and once undone.

/** A 120 x 80 canvas with a 40 x 30 grey layer at (30, 25) under a white 1 x 1 mask, at 1:1. */
async function setup(page: Page) {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  await expect(page.getByTestId("engine-ready")).toBeVisible();
  const grey = await page.evaluate(solidPngBase64, { width: 40, height: 30, color: "#808080" });
  return page.evaluate(async (g) => {
    const api = (window as any).__compositor;
    const bytes = Uint8Array.from(atob(g), (c: string) => c.charCodeAt(0));
    const doc = api.engine.newDocument(120, 80, false);
    api.engine.importImage(doc, bytes, "Grey", { x: 50, y: 40 });
    const layer = api.engine.state(doc).activeLayerId;
    api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
    api.store.getState().openDocument(doc);
    api.setCheckerboard(false);
    await api.setZoom(1);
    return { doc, layer };
  }, grey);
}
/** The largest channel difference between the GPU's picture of the document and the CPU compositor's. */
const worstGpuVsCpu = (page: Page) => page.evaluate(async () => {
  const api = (window as any).__compositor; const s = api.store.getState(); const d = s.documents[s.activeId];
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  const gl = api.readDocumentPixels() as Uint8Array;
  const cpu = api.engine.compositeEdit(d.id, null, { x: 0, y: 0, width: d.width, height: d.height }, d.width, d.height) as Uint8Array;
  // Past any channel's reach: a picture of the wrong size fails every `<= 2` below (audit M-6).
  if (gl.length !== cpu.length) return 256;
  let worst = 0;
  for (let i = 0; i < gl.length; i++) worst = Math.max(worst, Math.abs(gl[i] - cpu[i]));
  return worst;
});
const alphaAt = (page: Page, x: number, y: number) => page.evaluate(([x, y]) => {
  const api = (window as any).__compositor; const s = api.store.getState();
  return (api.engine.composite(s.activeId, { x, y, width: 1, height: 1 }, 1, 1) as Uint8Array)[3];
}, [x, y]);
const maskState = (page: Page, doc: string) => page.evaluate((doc) => {
  const s = (window as any).__compositor.engine.state(doc); const l = s.layers[0];
  return { w: l.maskWidth, h: l.maskHeight, placement: l.maskPlacement, depth: s.undoDepth };
}, doc);

test("a mask gradient grows the mask to the canvas: the GPU draws what the CPU draws, previewed, applied, moved and undone", async ({ page }) => {
  const { doc, layer } = await setup(page);
  // Black fading to nothing across the canvas over the white mask: at pixel x (centre x + 0.5) the mask
  // is 255 (x + 0.5) / 120, rounded, and the opaque grey layer shows with that alpha.
  const gradient = { shape: "Linear", start: [0, 40], end: [120, 40], from: [0, 0, 0, 1], to: [0, 0, 0, 0], opacity: 1 };
  const expected = (x: number) => Math.floor(255 * (x + 0.5) / 120 + 0.5);
  // The size the app decides the job worker by is the grown mask's (Engine::edit_pixels, ruling C1).
  expect(await page.evaluate(({ doc, layer }) => (window as any).__compositor.engine.editPixels(doc, layer, true), { doc, layer })).toBe(120 * 80);
  const before = await maskState(page, doc);
  // Previewed (120 x 80 is under GRADIENT_DRAG_LIMIT, so not reduced): the mask shown grown and placed.
  await page.evaluate(({ layer, gradient }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.setPreview(s.activeId, { preview: "Gradient", layer, mask: true, gradient, dragging: true }); s.refresh(s.activeId);
  }, { layer, gradient });
  const previewed = await maskState(page, doc);
  expect([previewed.w, previewed.h]).toEqual([120, 80]);
  expect(previewed.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "previewed").toBeLessThanOrEqual(2);
  for (const x of [32, 50, 67]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x)), `previewed at ${x}`).toBeLessThanOrEqual(1);
  // Applied: one undo step, the mask where the preview showed it.
  await page.evaluate(({ layer, gradient }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    api.engine.setPreview(s.activeId, null); s.run({ type: "Gradient", id: layer, mask: true, gradient });
  }, { layer, gradient });
  const applied = await maskState(page, doc);
  expect([applied.w, applied.h, applied.depth]).toEqual([120, 80, before.depth + 1]);
  expect(applied.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "applied").toBeLessThanOrEqual(2);
  for (const x of [32, 50, 67]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x)), `applied at ${x}`).toBeLessThanOrEqual(1);
  // Linked, the mask moves with its layer: 10 px right, the layer shows what the mask held 10 px left.
  await page.evaluate(({ layer }) => {
    const api = (window as any).__compositor; const s = api.store.getState();
    const t = api.engine.state(s.activeId).layers[0].transform;
    s.run({ type: "SetLayerTransform", id: layer, transform: { ...t, origin: [t.origin[0] + 10, t.origin[1]] } });
  }, { layer });
  expect((await maskState(page, doc)).placement).toMatchObject({ origin: [10, 0], size: [120, 80] });
  expect(await worstGpuVsCpu(page), "moved").toBeLessThanOrEqual(2);
  for (const x of [42, 60, 77]) expect(Math.abs((await alphaAt(page, x, 40)) - expected(x - 10)), `moved, at ${x}`).toBeLessThanOrEqual(1);
  // Undone twice: the 1 x 1 mask covering its layer again.
  await page.evaluate(() => { const s = (window as any).__compositor.store.getState(); s.undo(); s.undo(); });
  const undone = await maskState(page, doc);
  expect([undone.w, undone.h, undone.placement, undone.depth]).toEqual([1, 1, null, before.depth]);
  expect(await worstGpuVsCpu(page), "undone").toBeLessThanOrEqual(2);
});

test("Alt+Backspace and Delete in a selection on a targeted mask grow it to the canvas too", async ({ page }) => {
  const { doc } = await setup(page);
  // The left half of the layer selected, the mask targeted. The 120 x 80 canvas is less than a 256 px tile
  // across, so a fill inside the selection grows the mask to all of it (SelectionEdits.swift:200-210).
  await page.evaluate(() => {
    const s = (window as any).__compositor.store.getState();
    s.setMaskSelected(true);
    s.run({ type: "SelectShape", kind: "Rectangle", points: [[30, 25], [50, 25], [50, 55], [30, 55]], mode: "Replace", antialiased: false });
  });
  const before = await maskState(page, doc);
  // The mask palette's foreground is black: the selected half hides, the other half still shows.
  await page.keyboard.press("Alt+Backspace");
  const filled = await maskState(page, doc);
  expect([filled.w, filled.h, filled.depth]).toEqual([120, 80, before.depth + 1]);
  expect(filled.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect([await alphaAt(page, 40, 40), await alphaAt(page, 60, 40)]).toEqual([0, 255]);
  expect(await worstGpuVsCpu(page), "filled").toBeLessThanOrEqual(2);
  // Delete with the selection fills it with the mask's background colour, white (Task 14): a Fill, grown alike.
  await page.evaluate(() => (window as any).__compositor.store.getState().undo());
  expect((await maskState(page, doc)).w).toBe(1);
  await page.keyboard.press("Delete");
  const deleted = await maskState(page, doc);
  expect([deleted.w, deleted.h, deleted.depth]).toEqual([120, 80, before.depth + 1]);
  expect(deleted.placement).toMatchObject({ origin: [0, 0], size: [120, 80] });
  expect([await alphaAt(page, 40, 40), await alphaAt(page, 60, 40)]).toEqual([255, 255]);
  expect(await worstGpuVsCpu(page), "deleted").toBeLessThanOrEqual(2);
});
```

The timings (LL-073). The mask gradient's preview tick on a layer whose mask grows keeps the budgets of the existing mask ticks (ruling I2: drag < 50 ms, settled < 150 ms, total per tick = the engine call + the store refresh + the frame, one contiguous window, as "gradient previews" times it), at 24 and 100 MP, fit and 1:1, and the test asserts it measured the grown mask (the canvas halved to the limit). The frame after applying one (the refresh and the whole upload of a canvas-sized mask) gets 150 ms: Task 7's budget for the frame that draws a whole full-size image (measured 60-109 ms for a 24 MP RGBA image, 96 MB; a 100 MP grown mask is 100 MB of R8). The apply itself is timed on the UI thread only as a diagnostic: over `JOB_PIXELS` the Gradient tool sends it to the job worker (Task 15, by `editPixels`), and Task 15's perf case budgets that path (see "Consequences"). The native commit gets a budget: no slower than the pixel gradient's own native commit over the whole canvas that Task 8 measured on this machine (647 ms at 24 MP, 2706 ms at 100 MP): a mask gradient paints one grey byte where that paints four.

A Fill on a small layer's mask paints the whole canvas's worth of mask: 24 or 100 MP. Natively it gets twice Task 8's native Fill (133 ms at 24 MP, 343 ms at 100 MP, `a_gradient_and_a_fill_at_24_and_100_mp`): 266 / 686 ms. That Fill copied and composed its 24 / 100 MP RGBA layer but painted only a 3 MP ellipse; a canvas-sized mask Fill writes one byte a pixel but computes every pixel of the canvas, so twice is the room allowed, not a measurement. Controller ruling (2026-09-29): 266 / 686 ms is the ceiling. The implementer reports the measured fill at each size; where a measured value is below two thirds of its ceiling (under 177 ms at 24 MP, under 457 ms at 100 MP), the implementer tightens that assert to 1.5x the measured value, rounded up to a whole millisecond (ruling I2's way), and says so in the report and in the test's comment. By the worker rule (OQ5, ruling C1) a mask Fill of 24 or 100 MP never runs on the UI thread: `fillActive` asks `editPixels`, which counts the grown mask, and sends it to the worker. The e2e case presses Alt+Backspace on the targeted mask of a 1500 x 1000 layer on the 24 / 100 MP canvas and asserts that the worker ran it and the page kept its frames, with Task 14's and OQ5's budgets: the key press on the UI thread (`editPixels` and the copy out) < 150 ms, the longest frame gap while the worker fills < 100 ms, `jobInput` < 150 ms (a 7.5 MB layer and mask at either size), `installJob` < 150 / 450 ms (the grown mask, 24 / 100 MB, copied back; OQ5's "500" is 450 by ruling I7, audit M-5), the frame after it < 150 ms, timed inside the `installJob` hook as the "jobs" case does, so the windows are contiguous, and the frame gaps taken with `performance.now()` inside the rAF callback (audit I-7). The case runs twice, without a selection and under a feathered Select All: with a selection the key press must not fill the selection's clip on the UI thread (`SelectionClip::region`, audit I-5). Each run then adds (audit I-6, controller ruling: budget it, no redesign) (b) a second Alt+Backspace on the mask already grown to the canvas, through the worker, with the first fill's budgets (its `jobInput` now copies the grown mask out: the implementer reports the numbers; a budget missed is reported, not redesigned), and (a) a nudge (ArrowRight with the Move tool) of the layer whose mask grew, then Undo and Redo: each frame after < 150 ms (Task 7's budget for the frame that draws a whole full-size image). Today each of the three re-uploads the canvas-sized mask (a placement-only move bumps `mask_revision` through `mask_mut`, ops/transform.rs:17-18); keeping the revision on a placement-only move is a follow-up, not this task. The UI-thread time of the nudge, the Undo and the Redo and each frame's whole uploads are logged for the report.

```diff
--- a/engine/tests/perf_4b1.rs
+++ b/engine/tests/perf_4b1.rs
@@ -175,3 +175,56 @@ fn a_shape_over_the_whole_canvas_at_24_and_100_mp() {
         assert_eq!(e.state(id).unwrap().undo_depth, 0);
     }
 }
+
+#[test]
+#[ignore]
+fn a_mask_gradient_and_a_mask_fill_grown_to_the_canvas_at_24_and_100_mp() {
+    // Task 14a: a 1500 x 1000 layer in the middle of the canvas under a checkered mask of its own grid;
+    // a gradient, or a fill, grows the mask to the whole canvas. Budgets: the gradient no slower than the
+    // pixel gradient's own commit over the whole canvas, measured natively on this machine in Task 8 (647
+    // ms at 24 MP, 2706 ms at 100 MP, `a_gradient_and_a_fill_at_24_and_100_mp`): the mask paints one grey
+    // byte where that paints four. The fill twice Task 8's native fill there (133 / 343 ms): that one
+    // copied its 24 / 100 MP layer but painted a 3 MP ellipse; this one computes every pixel of the
+    // canvas. Above JOB_PIXELS both run in the job worker (ruling C1: `edit_pixels` counts the grown mask).
+    // The fill's are ceilings: under two thirds of one, the assert is tightened to 1.5x the measurement.
+    for (label, w, h, budget, fill_budget) in [("24 MP", 6000u32, 4000u32, 650.0, 266.0), ("100 MP", 10000, 10000, 2710.0, 686.0)] {
+        let mut doc = Document::new(w, h);
+        let mut layer = Layer::with_pixels("Small", Raster::from_premultiplied(1500, 1000, [60, 90, 120, 255].repeat(1500 * 1000)), Point { x: ((w - 1500) / 2) as f64, y: ((h - 1000) / 2) as f64 });
+        let checks: Vec<u8> = (0..1000u32).flat_map(|y| (0..1500u32).map(move |x| if (x / 50 + y / 50) % 2 == 0 { 0 } else { 255 })).collect();
+        layer.mask = Some(Mask { pixels: GrayRaster::from_bytes(1500, 1000, checks), enabled: true, placement: None, linked: None });
+        let lid = layer.id;
+        doc.active_layer_id = Some(lid);
+        doc.layers = vec![layer];
+        let mut e = Engine::new();
+        let id = e.insert_document(doc);
+        let gradient = |i: f64| GradientSpec { shape: GradientShape::Linear, start: Point { x: w as f64 * 0.2 + i, y: h as f64 * 0.3 }, end: Point { x: w as f64 * 0.8, y: h as f64 * 0.7 - i },
+            from: [0.0, 0.0, 0.0, 1.0], to: [0.0, 0.0, 0.0, 0.0], opacity: 0.9 };
+        // The preview ticks, to locate their cost (the release wasm's budgets are in perf-4b1.spec.ts):
+        // the worst of five after a first.
+        let worst = |e: &mut Engine, dragging: bool| {
+            e.set_preview(id, Some(PreviewRequest::Gradient { layer: lid, mask: true, gradient: gradient(0.0), dragging })).unwrap();
+            (1..6).map(|i| {
+                let t = Instant::now();
+                e.set_preview(id, Some(PreviewRequest::Gradient { layer: lid, mask: true, gradient: gradient(i as f64 * 7.0), dragging })).unwrap();
+                ms(t)
+            }).fold(0.0, f64::max)
+        };
+        let drag = worst(&mut e, true);
+        let settled = worst(&mut e, false);
+        e.set_preview(id, None).unwrap();
+        let t = Instant::now();
+        run(&mut e, id, Command::Gradient { id: lid, mask: true, gradient: gradient(0.0) });
+        let commit = ms(t);
+        let state = e.state(id).unwrap().layers[0].clone();
+        assert_eq!((state.mask_width, state.mask_height), (w, h), "the gradient grew the mask to the canvas");
+        e.undo(id).unwrap();
+        let t = Instant::now();
+        run(&mut e, id, Command::Fill { id: lid, mask: true, color: [0.0, 0.0, 0.0] });
+        let fill = ms(t);
+        let state = e.state(id).unwrap().layers[0].clone();
+        assert_eq!((state.mask_width, state.mask_height), (w, h), "the fill grew the mask to the canvas");
+        println!("{label}: mask grown to the canvas: gradient preview tick dragging {drag:.0} ms, settled {settled:.0} ms; gradient applied {commit:.0} ms; fill {fill:.0} ms");
+        assert!(commit <= budget, "{label}: the grown mask gradient took {commit:.0} ms, budget {budget} ms");
+        assert!(fill <= fill_budget, "{label}: the grown mask fill took {fill:.0} ms, budget {fill_budget} ms");
+    }
+}
```

```diff
--- a/app/tests/e2e/perf-4b1.spec.ts
+++ b/app/tests/e2e/perf-4b1.spec.ts
@@ -774,5 +774,201 @@ test("gradient previews: a drag tick, the settled preview and a patch in a 700 p
     expect(out[`${label}: patch whole uploads`], "the patch reaches the GPU as its rectangle, never a whole texture").toBe(0);
   }
 });
+
+test("mask gradients that grow the mask to the canvas: drag and settled ticks, and the frame after applying one, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      // A 1500 x 1000 layer in the middle of the canvas (Canvas Size with a fill, then without one)
+      // under a non-uniform mask of its own grid (an ellipse selection's): its gradient grows the mask
+      // to the whole canvas (Task 14a), 24 or 100 MP of mask from 1.5 MP.
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      const x0 = (w - 1500) / 2, y0 = (h - 1000) / 2;
+      api.engine.execute(doc, { type: "SelectShape", kind: "Ellipse", points: [[x0 + 100, y0 + 100], [x0 + 1400, y0 + 100], [x0 + 1400, y0 + 900], [x0 + 100, y0 + 900]], mode: "Replace", antialiased: true });
+      api.engine.execute(doc, { type: "AddMaskFromSelection", id: layer, revealing: true });
+      api.engine.execute(doc, { type: "Deselect" });
+      api.store.getState().openDocument(doc);
+      const fit = api.store.getState(); fit.viewports[doc].fit({ width: w, height: h }); fit.invalidate();
+      await settle(); frame();
+      const gradient = (i: number) => ({ shape: "Linear", start: [w * 0.2 + i, h * 0.3], end: [w * 0.8, h * 0.7 - i], from: [0, 0, 0, 1], to: [0, 0, 0, 0], opacity: 0.9 });
+      // One tick as "gradient previews" times it: the engine's preview, the refresh and the frame that
+      // draws it, one contiguous window.
+      const tick = (i: number, dragging: boolean) => {
+        const t0 = performance.now();
+        api.engine.setPreview(doc, { preview: "Gradient", layer, mask: true, gradient: gradient(i), dragging });
+        const engine = performance.now() - t0;
+        api.store.getState().refresh(doc);
+        const f = frame();
+        return [engine, f, performance.now() - t0];
+      };
+      const worst = (dragging: boolean) => {
+        const ticks: number[][] = [];
+        for (let i = 0; i < 6; i++) ticks.push(tick(i * 7, dragging));
+        ticks.shift(); // the first builds the selection clip and the textures
+        return [0, 1, 2].map((k) => Math.round(Math.max(...ticks.map((t) => t[k]))));
+      };
+      for (const zoom of ["fit", "1:1"]) {
+        if (zoom === "1:1") { await api.setZoom(1); await settle(); frame(); }
+        let [e, f, t] = worst(true);
+        result[`drag engine ${zoom}`] = e; result[`drag frame ${zoom}`] = f; result[`drag total ${zoom}`] = t;
+        const dragged = api.engine.state(doc).layers[0];
+        result[`dragged mask pixels ${zoom}`] = dragged.maskWidth * dragged.maskHeight;
+        [e, f, t] = worst(false);
+        result[`settled engine ${zoom}`] = e; result[`settled frame ${zoom}`] = f; result[`settled total ${zoom}`] = t;
+        const settled = api.engine.state(doc).layers[0];
+        result[`settled mask pixels ${zoom}`] = settled.maskWidth * settled.maskHeight;
+        api.engine.setPreview(doc, null); api.store.getState().refresh(doc); frame(); await settle();
+      }
+      // Applied, on the UI thread here: a diagnostic only (over JOB_PIXELS the Gradient tool sends it to
+      // the job worker, Task 15, whose perf case budgets that path). Then the refresh and the frame that
+      // draw the canvas-sized mask, uploaded whole: budgeted below.
+      let t0 = performance.now();
+      api.engine.execute(doc, { type: "Gradient", id: layer, mask: true, gradient: gradient(0) });
+      result["apply on the UI thread (diagnostic) ms"] = Math.round(performance.now() - t0);
+      t0 = performance.now();
+      api.store.getState().refresh(doc);
+      frame();
+      result["frame after applying (refresh and whole mask upload) ms"] = Math.round(performance.now() - t0);
+      result["frame after applying: whole uploads"] = (window as any).__uploads.image;
+      const applied = api.engine.state(doc).layers[0];
+      result["applied mask pixels"] = applied.maskWidth * applied.maskHeight;
+      api.store.getState().closeDocument(doc);
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`mask gradients that grow the mask (release wasm, Edge): ${JSON.stringify(out)}`);
+  // The preview's grid: the canvas halved until it fits the limit (`level_for`, preview.rs).
+  const reduced = (w: number, h: number, limit: number) => { let l = 0; while (Math.max(w >> l, h >> l) > limit && (w >> l) > 1 && (h >> l) > 1) l++; return (w >> l) * (h >> l); };
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    for (const zoom of ["fit", "1:1"]) {
+      // It timed the grown mask, not the layer's own grid.
+      expect(out[`${label}: dragged mask pixels ${zoom}`]).toBe(reduced(w, h, 1024));
+      expect(out[`${label}: settled mask pixels ${zoom}`]).toBe(reduced(w, h, 2048));
+      // Ruling I2: the mask gradient's ticks keep the pixel gradient's budgets.
+      expect(out[`${label}: drag total ${zoom}`]).toBeLessThan(50);
+      expect(out[`${label}: settled total ${zoom}`]).toBeLessThan(150);
+    }
+    expect(out[`${label}: applied mask pixels`]).toBe(w * h);
+    expect(out[`${label}: frame after applying: whole uploads`]).toBeGreaterThanOrEqual(1);
+    // Task 7's budget for the frame that draws a whole full-size image (a 100 MP grown mask is 100 MB).
+    expect(out[`${label}: frame after applying (refresh and whole mask upload) ms`]).toBeLessThan(150);
+  }
+});
+
+test("a fill on a small layer's mask: the mask grows to the canvas through the worker, then a second fill, a nudge, Undo and Redo, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    for (const selected of [false, true]) {
+      await ready(page);
+      await installFrameTimer(page);
+      const r = await page.evaluate(async ([w, h, selected]) => {
+        const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+        const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+        const s = () => api.store.getState();
+        const result: Record<string, number> = {};
+        let installedAt = Infinity, jobs = 0, pass = "first fill";
+        const timed = (name: string, after?: () => void) => {
+          const f = api.engine[name].bind(api.engine);
+          api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${pass}: ${name} ms`] = Math.round(performance.now() - t0); after?.(); } };
+        };
+        timed("jobInput");
+        // The frame that draws the result, timed where it lands, as the "jobs" case does: no interval
+        // outside a window (audit I-7).
+        timed("installJob", () => { installedAt = performance.now(); s().refresh(s().activeId); result[`${pass}: frame after it ms`] = Math.round(frame()); result[`${pass}: frame after it, whole uploads`] = (window as any).__uploads.image; });
+        const client = s().jobs; const send = client.run.bind(client);
+        client.run = (...a: unknown[]) => { jobs++; return send(...a); };
+        // A 1500 x 1000 layer in the middle of the canvas under a white mask, targeted: 1.5 MP stored, the
+        // canvas painted (Task 14a).
+        const doc = api.engine.newDocument(10, 10, false);
+        api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
+        const layer = api.engine.state(doc).layers[0].id;
+        api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
+        api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+        api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
+        s().openDocument(doc);
+        s().setMaskSelected(true);
+        // Audit I-5: under a feathered Select All the key must not fill the selection's clip on the UI
+        // thread (`SelectionClip::region`); the worker builds its own.
+        if (selected) { api.engine.execute(doc, { type: "SelectAll" }); api.engine.execute(doc, { type: "FeatherSelection", amount: 20 }); s().refresh(doc); }
+        await settle(); frame();
+        result["mask pixels the fill paints"] = api.engine.editPixels(doc, layer, true);
+        // performance.now() inside the callback, not the rAF timestamp (Task 6 fix round 1, issue 4;
+        // audit I-7). Only frames before the result is put back count.
+        const longestGap = (until: () => boolean) => new Promise<number>((done) => {
+          let last = performance.now(), gap = 0;
+          const tick = () => { const now = performance.now(); if (now <= installedAt) gap = Math.max(gap, now - last); last = now; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+          requestAnimationFrame(tick);
+        });
+        const fill = async () => {
+          installedAt = Infinity;
+          const t0 = performance.now();
+          window.dispatchEvent(new KeyboardEvent("keydown", { key: "Backspace", altKey: true }));
+          result[`${pass}: Alt+Backspace (UI thread) ms`] = Math.round(performance.now() - t0);
+          result[`${pass}: longest frame gap while the worker fills`] = await longestGap(() => !s().working);
+        };
+        await fill();
+        const l = api.engine.state(doc).layers[0];
+        result["mask pixels"] = l.maskWidth * l.maskHeight;
+        // Audit I-6 (b): a second fill on the mask already grown to the canvas: its copies out and back
+        // are the grown mask's (24 / 100 MB).
+        pass = "second fill";
+        await settle(); frame();
+        await fill();
+        result["jobs run"] = jobs;
+        // Audit I-6 (a): a nudge of the layer whose mask grew, then Undo and Redo. Each moves the placed
+        // mask with its layer, which today bumps its revision and re-uploads it whole (keeping the
+        // revision on a placement-only move is a follow-up); the frame after each is budgeted.
+        s().setTool("move");
+        await settle(); frame();
+        for (const [step, act] of [["nudge", () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" }))], ["undo", () => s().undo()], ["redo", () => s().redo()]] as [string, () => void][]) {
+          const t0 = performance.now();
+          act();
+          result[`${step} (UI thread) ms`] = Math.round(performance.now() - t0);
+          result[`frame after the ${step} ms`] = Math.round(frame());
+          result[`frame after the ${step}, whole uploads`] = (window as any).__uploads.image;
+          await settle();
+        }
+        result["layer moved by the nudge, then back, then again"] = api.engine.state(doc).layers[0].transform.origin[0] - (w - 1500) / 2;
+        s().closeDocument(doc);
+        return result;
+      }, [w, h, selected] as [number, number, boolean]);
+      for (const [k, v] of Object.entries(r)) out[`${label}${selected ? ", Select All feathered" : ""}: ${k}`] = v;
+    }
+  }
+  console.log(`a fill on a growing mask (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    for (const run of [label, `${label}, Select All feathered`]) {
+      const k = (name: string) => out[`${run}: ${name}`];
+      // The worker rule (OQ5, ruling C1): counted grown, so sent to the worker, both times.
+      expect(k("mask pixels the fill paints")).toBe(w * h);
+      expect(k("jobs run"), "both fills went to the worker").toBe(2);
+      expect(k("mask pixels")).toBe(w * h);
+      expect(k("layer moved by the nudge, then back, then again"), "the nudge, Undo and Redo ran").toBe(1);
+      for (const pass of ["first fill", "second fill"]) {
+        expect(k(`${pass}: Alt+Backspace (UI thread) ms`)).toBeLessThan(150);
+        expect(k(`${pass}: longest frame gap while the worker fills`)).toBeLessThan(100);
+        expect(k(`${pass}: jobInput ms`)).toBeLessThan(150);
+        // OQ5's "500" at 100 MP is 450 (ruling I7).
+        expect(k(`${pass}: installJob ms`)).toBeLessThan(label === "24 MP" ? 150 : 450);
+        expect(k(`${pass}: frame after it, whole uploads`), "the frame timed is the one that uploads the grown mask").toBeGreaterThanOrEqual(1);
+        expect(k(`${pass}: frame after it ms`)).toBeLessThan(150);
+      }
+      // Task 7's budget for the frame that draws a whole full-size image.
+      for (const step of ["nudge", "undo", "redo"]) expect(k(`frame after the ${step} ms`)).toBeLessThan(150);
+    }
+  }
+});

 test("eyedropper: a sample and the overlay that shows its ring, at 24 and 100 MP", async ({ page }) => {
```

- [ ] **Step 2: Run the tests and watch them fail**

`cargo test -p compositor-engine --test mask_grow`: does not compile (`SelectionClip::region` does not exist yet); without the region test it would compile and fail at the first `edit_pixels` (200, not 4,000: Task 14's count of the mask as it is). `PreviewTarget::Mask { .. }` is not a compile error before Step 3: Rust accepts the brace pattern against the tuple variant (audit M-4). `cargo test -p compositor-engine --test gradient_preview`: compiles and passes before Step 3 (its pattern change is for the struct variant Step 3 makes); `a_masks_background_...` passes before Step 3 as well (the new `background` is a speed change with the same answers). `pnpm e2e -g "grow"`: the first case fails at `editPixels` (1,200, not 9,600: the layer's 40 x 30 grid), the second at the filled mask's size (40 x 30, not 120 x 80).

- [ ] **Step 3: Implement**

```diff
--- a/engine/src/ops/raster_edit.rs
+++ b/engine/src/ops/raster_edit.rs
@@ -1,10 +1,12 @@
 //! Raster edits (Phase 4b-1): Fill and the Gradient, painted as Compositor for Mac paints them
 //! (`BrushStroke.paintCanvas`, BrushStroke.swift:645-675): over the layer's original pixels, clipped
 //! to the canvas and the selection, at the edit's opacity (source-over, the colour's alpha times the
 //! opacity times the selection's coverage). A layer's pixel grid first grows to cover the canvas
-//! (`BrushStroke.init`, :153-160); a mask keeps its own grid. The layer's result is trimmed to the
-//! pixels left, and a mask covering it follows it, white where the layer grew (`commitRasterEdit`,
-//! EditorSession+Brush.swift:154-188; `expandMask`, BrushStroke.swift:858-866).
+//! (`BrushStroke.init`, :153-160). A mask is painted in its own grid (`mask_grid`), grown past its
+//! layer to the canvas by a Fill or a Gradient as Compositor 1.3.7 grows it (Task 14a, the user's
+//! decision of 2026-09-29). The layer's result is trimmed to the pixels left, and a mask covering it
+//! follows it, white where the layer grew (`commitRasterEdit`, EditorSession+Brush.swift:154-188;
+//! `expandMask`, BrushStroke.swift:858-866).
 use crate::*;
 use serde::{Deserialize, Serialize};
 use uuid::Uuid;
@@ -200,73 +202,163 @@ fn check_target(doc: &Document, id: Uuid, mask: bool) -> Result<(), CommandError> {
     Ok(())
 }
 
-/// Whether the layer's covering mask, once brought onto its own pixel grid (`mask_on_layer_grid`),
-/// still fits the mask budget: the commit's own check before it grows the mask, shared so a preview
-/// refuses to show a mask gradient the commit would refuse for the same reason (fix round 1, item 3a;
-/// `paint_layer_check`).
-fn mask_grow_check(doc: &Document, id: Uuid) -> Result<(), CommandError> {
-    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
-    let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
-    if m.placement.is_some() { return Ok(()); }
-    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
-    if (m.pixels.width, m.pixels.height) == (w, h) { return Ok(()); }
-    let others = doc.used_mask_pixels().saturating_sub(m.pixels.width as u64 * m.pixels.height as u64);
-    if (w as u64) * (h as u64) > MAX_PIXELS.saturating_sub(others) { return Err(CommandError::Project(ProjectError::TooLarge)); }
-    Ok(())
+/// The layer's own pixel grid: its pixels, or its box rounded when it has none.
+fn layer_grid(layer: &Layer) -> (u32, u32) {
+    layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height))
 }
 
-/// A covering mask brought onto the layer's own pixel grid (a mask on its own placement keeps its
-/// grid): the Mac paints a mask in the grid it covers (BrushStroke.swift:148-152), so a 1 x 1 or
-/// otherwise sized covering mask is stretched onto the layer's pixels first, nearest.
-fn mask_on_layer_grid(doc: &mut Document, id: Uuid) -> Result<(), CommandError> {
-    mask_grow_check(doc, id)?;
-    let layer = doc.layer(id).unwrap();
-    let m = layer.mask.as_ref().unwrap();
-    if m.placement.is_some() { return Ok(()); }
-    let (w, h) = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
-    if (m.pixels.width, m.pixels.height) == (w, h) { return Ok(()); }
-    let src = m.pixels.clone();
-    let data = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| {
-        let (mx, my) = ((x as u64 * src.width as u64 / w as u64) as u32, (y as u64 * src.height as u64 / h as u64) as u32);
-        src.bytes()[(my * src.width + mx) as usize]
-    }).collect();
-    doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
-    Ok(())
+/// The document rectangle (`x0`, `y0`) to (`x1`, `y1`) on the grid `inverse` maps the document onto:
+/// the box of its corners there, rounded out to whole pixels (`CGRect.applying(_:).integral`).
+fn rect_on(inverse: &Affine, x0: f64, y0: f64, x1: f64, y1: f64) -> (i64, i64, i64, i64) {
+    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| inverse.apply(Point { x, y }));
+    let (lx, hx) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.x), h.max(p.x)));
+    let (ly, hy) = corners.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), p| (l.min(p.y), h.max(p.y)));
+    (lx.floor() as i64, ly.floor() as i64, hx.ceil() as i64, hy.ceil() as i64)
+}
+
+/// The Mac's raster-edit tile, in grid pixels (`BrushStroke.tileSize`, BrushStroke.swift:144 at v1.3.7).
+const TILE: i64 = 256;
+
+/// Where a mask edit paints, and the mask it leaves (Task 14a): the mask's grid before the edit, `w` x
+/// `h` pixels that `base` places, and the grid the edit leaves, `width` x `height` pixels that
+/// `transform` places, with the old grid at (`x`, `y`) in it. `placement` is the mask's placement
+/// afterwards: None while it still covers its layer.
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub struct MaskGrid {
+    pub w: u32, pub h: u32, pub base: LayerTransform,
+    pub width: u32, pub height: u32, pub x: u32, pub y: u32,
+    pub transform: LayerTransform, pub placement: Option<LayerTransform>,
+}
+
+impl MaskGrid {
+    /// The pixels the edit paints and leaves.
+    pub fn pixels(&self) -> u64 { self.width as u64 * self.height as u64 }
+}
+
+/// The grid a mask edit paints and the mask it leaves, as Compositor 1.3.7 makes them (BrushStroke.swift
+/// and EditorSession+Brush.swift at v1.3.7). The mask's own grid: a mask on its own placement is painted
+/// in its own pixels, a solid one (at most 2 x 2) at one pixel per document pixel over its place, a
+/// covering mask in its layer's grid, stretched onto it (`init`, :157-167). A growing edit (`grows`:
+/// the Mac's Fill and Gradient both are, `applyPixelEdit`, SelectionEdits.swift:204, and `beginGradient`,
+/// Gradient.swift:48; its other raster edits, not in this port, are not) widens that grid to the canvas
+/// as the grid maps it, rounded out (:168),
+/// and keeps every 256-pixel tile it paints, counted from the widened grid's corner: the old grid joined
+/// with the tiles over the canvas, or over the selection's clip (`paintCanvas`, :658-670; `allocateTile`,
+/// :576-583; `committedBounds`, :764). The mask then takes its own place on the document when it had one
+/// or grew (`commitRasterEdit`, EditorSession+Brush.swift:180-187; `transform(for:)`, :766-773); a place
+/// that did not change is kept exactly. Refused, as the Mac refuses, when a side passes MAX_SIDE (:182,
+/// :581), the mask passes what the other layers' masks leave of the budget (EditorSession+Brush.swift:
+/// 16-20 with the mask targeted; BrushStroke.swift:582), or its placement is not a valid one
+/// (EditorSession+Brush.swift:163, :168).
+pub fn mask_grid(doc: &Document, id: Uuid, grows: bool) -> Result<MaskGrid, CommandError> {
+    let too_large = || CommandError::Project(ProjectError::TooLarge);
+    let layer = doc.layer(id).ok_or(CommandError::NoLayer)?;
+    let m = layer.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
+    let base = m.placement.unwrap_or(layer.transform);
+    let (w, h) = match m.placement {
+        Some(p) if m.pixels.width <= 2 && m.pixels.height <= 2 => (p.size.width.round().max(1.0), p.size.height.round().max(1.0)),
+        Some(_) => (m.pixels.width as f64, m.pixels.height as f64),
+        None => { let (w, h) = layer_grid(layer); (w as f64, h as f64) }
+    };
+    if w > MAX_SIDE as f64 || h > MAX_SIDE as f64 { return Err(too_large()); }
+    let (w, h) = (w as u32, h as u32);
+    let (mut x0, mut y0, mut x1, mut y1) = (0i64, 0i64, w as i64, h as i64);
+    if grows {
+        let inverse = base.pixel_to_document(w, h).invert().ok_or_else(|| CommandError::Argument("the mask cannot be painted".into()))?;
+        let canvas = rect_on(&inverse, 0.0, 0.0, doc.width as f64, doc.height as f64);
+        let (ex0, ey0, ex1, ey1) = (canvas.0.min(0), canvas.1.min(0), canvas.2.max(w as i64), canvas.3.max(h as i64));
+        // What the edit paints: the canvas, or the selection's clip rectangle (cut to the canvas), read
+        // from the selection's bounds without filling the clip (`SelectionClip::region`): the app asks
+        // this on the UI thread before a job (audit I-5).
+        let area = match &doc.selection {
+            None => Some((0.0, 0.0, doc.width as f64, doc.height as f64)),
+            Some(s) => SelectionClip::region(s, doc.width, doc.height).map(|(x0, y0, x1, y1)| (x0 as f64, y0 as f64, x1 as f64, y1 as f64)),
+        };
+        if let Some((ax0, ay0, ax1, ay1)) = area {
+            let r = rect_on(&inverse, ax0, ay0, ax1, ay1);
+            let (rx0, ry0, rx1, ry1) = (r.0.max(ex0), r.1.max(ey0), r.2.min(ex1), r.3.min(ey1));
+            if rx0 < rx1 && ry0 < ry1 {
+                let (tx0, ty0) = (ex0 + (rx0 - ex0) / TILE * TILE, ey0 + (ry0 - ey0) / TILE * TILE);
+                let (tx1, ty1) = ((ex0 + ((rx1 - 1 - ex0) / TILE + 1) * TILE).min(ex1), (ey0 + ((ry1 - 1 - ey0) / TILE + 1) * TILE).min(ey1));
+                (x0, y0, x1, y1) = (tx0.min(0), ty0.min(0), tx1.max(w as i64), ty1.max(h as i64));
+            }
+        }
+    }
+    let (width, height) = (x1 - x0, y1 - y0);
+    let others = doc.used_mask_pixels().saturating_sub(m.pixels.width as u64 * m.pixels.height as u64);
+    if width > MAX_SIDE || height > MAX_SIDE || (width as u64) * (height as u64) > MAX_PIXELS.saturating_sub(others) { return Err(too_large()); }
+    let (width, height, x, y) = (width as u32, height as u32, (-x0) as u32, (-y0) as u32);
+    let grew = (x, y, width, height) != (0, 0, w, h);
+    let transform = if grew { ops::adjust::placed_like(&base, w, h, width, height, x as f64, y as f64) } else { base };
+    if !transform.is_valid() { return Err(too_large()); }
+    let placement = if m.placement.is_some() || grew { Some(transform) } else { None };
+    Ok(MaskGrid { w, h, base, width, height, x, y, transform, placement })
+}
+
+/// The mask as a mask edit starts from it on `grid`: its background everywhere (white reveals, black
+/// hides: `Mask::background`, the Mac's `maskBackground`, BrushStroke.swift:169-170), then its own
+/// pixels over the old grid, stretched onto it nearest where they are sized otherwise (a 1 x 1 mask, a
+/// solid placed one). Each new tile starts so (`allocateTile`, :585-613), as the commit's canvas does
+/// (`BrushCommit.render`, :888-896).
+fn mask_on_grid(m: &Mask, grid: &MaskGrid) -> Vec<u8> {
+    let mut out = vec![m.background(); grid.width as usize * grid.height as usize];
+    let (mw, mh) = (m.pixels.width as u64, m.pixels.height as u64);
+    let src = m.pixels.bytes();
+    let xs: Vec<usize> = (0..grid.w as u64).map(|x| (x * mw / grid.w as u64) as usize).collect();
+    for y in 0..grid.h as u64 {
+        let sy = (y * mh / grid.h as u64) as usize;
+        let row = &src[sy * mw as usize..(sy + 1) * mw as usize];
+        let at = ((y + grid.y as u64) * grid.width as u64 + grid.x as u64) as usize;
+        let dst = &mut out[at..at + grid.w as usize];
+        if mw == grid.w as u64 { dst.copy_from_slice(row); } else { for (d, &sx) in dst.iter_mut().zip(&xs) { *d = row[sx]; } }
+    }
+    out
+}
+
+/// The pixels a Fill or a Gradient on layer `id` paints and leaves (`paint_layer`): the layer's grid
+/// grown to the canvas (`image_grid`), or with `mask` the mask's grid grown to the canvas (`mask_grid`).
+/// What decides whether the edit goes to the job worker (ruling C1, `Engine::edit_pixels`: a small
+/// layer's mask on a large canvas counts at its grown size); an error where the edit is refused for its
+/// size, which the worker, seeing one layer, could not tell (ruling OQ20).
+pub fn painted_pixels(doc: &Document, id: Uuid, mask: bool) -> Result<u64, CommandError> {
+    if mask { return Ok(mask_grid(doc, id, true)?.pixels()); }
+    let grid = image_grid(doc, doc.layer(id).ok_or(CommandError::NoLayer)?)?;
+    Ok(grid.width as u64 * grid.height as u64)
 }
 
 /// Whether `paint_layer` would paint: the paint's values and the target (a preview asks first, and
-/// shows nothing the commit would refuse). On the mask, this also refuses what growing a covering
-/// mask onto the layer's grid would refuse (`mask_grow_check`, the same check `mask_on_layer_grid`
-/// itself makes before it grows one).
+/// shows nothing the commit would refuse). On the mask, this also refuses what the mask edit would
+/// refuse for its size (`mask_grid`, the grid the commit and the preview both paint: grown to the
+/// canvas for a Gradient, so the refusal is at the grown size).
 ///
 /// Known gap (fix round 1, item 3b, a controller ruling): on the PIXELS side, a growing layer's own
 /// covering mask may also need to grow once the paint is trimmed back (`paint_layer`'s own check
 /// below, near its `followed` call) -- that check depends on the trimmed commit result, which this
 /// preview-time check never computes (a preview never actually paints and trims the grid), so it is
 /// NOT mirrored here. A rare preview on a growing layer with a non-white covering mask near the mask
 /// budget may therefore show a gradient that Return then refuses with "too large". Parked; not fixed
 /// this round.
 pub fn paint_layer_check(doc: &Document, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
     paint.check()?;
     check_target(doc, id, mask)?;
-    if mask { mask_grow_check(doc, id)?; }
+    if mask { mask_grid(doc, id, true)?; }
     Ok(())
 }
 
 /// Paints `paint` into layer `id`'s pixels (`mask` false) or its mask, as one edit: Fill and the
 /// Gradient's commit.
 pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
     paint_layer_check(doc, id, mask, paint)?;
     if mask {
-        mask_on_layer_grid(doc, id)?;
-        let layer = doc.layer(id).unwrap();
-        let m = layer.mask.as_ref().unwrap();
-        let grid = m.placement.unwrap_or(layer.transform);
-        let (w, h) = (m.pixels.width, m.pixels.height);
-        let coverage = ops::adjust::edit_coverage(doc, clips, &grid, w, h)?;
-        let mut data = m.pixels.bytes().to_vec();
-        paint_grid(doc, &mut data, w, h, &grid, coverage.as_ref(), paint, true);
-        doc.layer_mut(id).unwrap().mask_mut().unwrap().pixels = GrayRaster::from_bytes(w, h, data);
+        // The mask on the grid the edit leaves (grown to the canvas for a Gradient), painted there, and
+        // placed where that grid sits (EditorSession+Brush.swift:180-187 at v1.3.7).
+        let grid = mask_grid(doc, id, true)?;
+        let mut data = mask_on_grid(doc.layer(id).unwrap().mask.as_ref().unwrap(), &grid);
+        let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, grid.width, grid.height)?;
+        paint_grid(doc, &mut data, grid.width, grid.height, &grid.transform, coverage.as_ref(), paint, true);
+        let target = doc.layer_mut(id).unwrap().mask_mut().unwrap();
+        target.pixels = GrayRaster::from_bytes(grid.width, grid.height, data);
+        target.placement = grid.placement;
         return Ok(());
     }
     let layer = doc.layer(id).unwrap().clone();
@@ -285,7 +375,7 @@ pub fn paint_layer(doc: &mut Document, clips: &SelectionClips, id: Uuid, mask: bool, paint: &Paint) -> Result<(), CommandError> {
     let own = layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height));
     let same_grid = crop == (grid.x, grid.y, grid.x + own.0, grid.y + own.1);
     // A covering mask that must grow onto the new grid needs the room: every other path that
-    // grows a mask checks it (masks.rs:13, :80; selection.rs:215; `mask_on_layer_grid` above), the
+    // grows a mask checks it (masks.rs:13, :80; selection.rs:215; `mask_grid` above), the
     // Mac shrinks its own paint limit by the other masks already held
     // (EditorSession+Brush.swift:22-25), and a project whose masks add up past MAX_PIXELS refuses
     // to reopen (package.rs:33-40). A uniform white mask stays 1 x 1 (`followed`'s own early
```

```diff
--- a/engine/src/preview.rs
+++ b/engine/src/preview.rs
@@ -49,9 +49,11 @@ impl PreviewSource {
 }
 
 /// What a preview stands in for: the layer's pixels (`raster`, placed by `transform`), a patch of them
-/// (`raster` is that rectangle of the stored pixels' grid, drawn over them), or the layer's mask.
+/// (`raster` is that rectangle of the stored pixels' grid, drawn over them), or the layer's mask: its
+/// `pixels` as the gradient leaves them (reduced) and where they sit (`placement`, None while it covers
+/// its layer; a mask gradient grows the mask past its layer, Task 14a).
 #[derive(Clone, Debug)]
-pub enum PreviewTarget { Pixels, Patch(PixelRect), Mask(GrayRaster) }
+pub enum PreviewTarget { Pixels, Patch(PixelRect), Mask { pixels: GrayRaster, placement: Option<LayerTransform> } }
 
 /// The substituted pixels for one layer while a panel is open, the request that made them and
 /// what they were made from.
@@ -188,56 +190,65 @@ fn level_for(width: u32, height: u32, limit: u32) -> u32 {
     level
 }
 
-/// A gradient's preview (Phase 4b-1). On a mask: the mask reduced to the request's limit, painted.
-/// On pixels inside a selection whose rectangle is small (`PATCH_LIMIT`), when the layer already
-/// covers the canvas and draws no effects: that rectangle at full size, as a patch. Otherwise the
-/// layer's grid grown to the canvas (`raster_edit::image_grid`), reduced to the limit with its
-/// origin on the reduced pixels, the layer's own halvings placed in it, painted; never trimmed.
+/// A gradient's preview (Phase 4b-1). On a mask: the grid the commit paints and leaves (`mask_grid`:
+/// the mask's own grid grown to the canvas, Task 14a), reduced to the request's limit, painted, and
+/// shown where the commit places it. On pixels inside a selection whose rectangle is small
+/// (`PATCH_LIMIT`), when the layer already covers the canvas and draws no effects: that rectangle at
+/// full size, as a patch. Otherwise the layer's grid grown to the canvas (`raster_edit::image_grid`),
+/// reduced to the limit with its origin on the reduced pixels, the layer's own halvings placed in it,
+/// painted; never trimmed.
 fn gradient_preview(doc: &Document, clips: &SelectionClips, request: &PreviewRequest, id: Uuid, mask: bool, gradient: &GradientSpec, revision: u64) -> Option<PixelPreview> {
-    use ops::raster_edit::{image_grid, paint_grid, Paint};
+    use ops::raster_edit::{image_grid, mask_grid, paint_grid, Paint};
     let layer = doc.layer(id)?;
     let paint = Paint::Gradient(gradient.clone());
     // What the commit would refuse, the preview does not show.
     if ops::raster_edit::paint_layer_check(doc, id, mask, &paint).is_err() { return None; }
     let made_from = PreviewSource::of(doc, id);
     let limit = preview_limit(request);
     if mask {
-        // The grid the commit paints: a placed mask's own, else the layer's pixels (a covering mask
-        // of another size is stretched onto it), reduced; the mask sampled onto it, nearest.
+        // The grid the commit paints and leaves: a placed mask's own, else the layer's pixels (a
+        // covering mask of another size is stretched onto it), grown to the canvas (1.3.7), reduced;
+        // the mask sampled onto it nearest, its background where it grew.
         let m = layer.mask.as_ref()?;
-        let placement = m.placement.unwrap_or(layer.transform);
-        let (gw, gh) = if m.placement.is_some() { (m.pixels.width, m.pixels.height) } else {
-            layer.pixels.as_ref().map_or((layer.transform.size.width.round().max(1.0) as u32, layer.transform.size.height.round().max(1.0) as u32), |p| (p.width, p.height))
-        };
-        let level = level_for(gw, gh, limit);
-        let (rw, rh) = ((gw >> level).max(1), (gh >> level).max(1));
+        let grid = mask_grid(doc, id, true).ok()?;
+        let level = level_for(grid.width, grid.height, limit);
+        let (rw, rh) = ((grid.width >> level).max(1), (grid.height >> level).max(1));
         let (mw, mh) = (m.pixels.width as u64, m.pixels.height as u64);
+        let grew = (grid.x, grid.y, grid.width, grid.height) != (0, 0, grid.w, grid.h);
         // A freshly added mask is 1 x 1 by its size alone (`Mask::is_uniform`, no scan of its
         // content) and needs no resample at all. Any other size is sampled nearest without a single
         // per-pixel division: each output column's source column is looked up once into `xs`, and
         // each row is then copied through it with one division for its own source row. Every tick of
         // a drag would otherwise redo a full per-pixel gather from scratch, unlike the pixel path's
         // memoized halving; scanning the mask's own content to find a uniform one (`GrayRaster::is_uniform`)
         // would cost just as much as the gather it was meant to save (fix round 1: measured 12.8 ms at
         // 24 MP and 53 ms at 100 MP for that scan alone, on a full-size uniform mask after a mask Fill).
-        let mut data = vec![0u8; rw as usize * rh as usize];
-        if m.is_uniform() {
+        let mut data = vec![m.background(); rw as usize * rh as usize];
+        if m.is_uniform() && !grew {
             data.fill(m.pixels.bytes()[0]);
         } else {
-            let xs: Vec<usize> = (0..rw as u64).map(|x| (x * mw / rw as u64) as usize).collect();
+            // A reduced column x lands on the grown grid's column x * width / rw; inside the old grid
+            // (less `grid.x`) that is the stored mask's column times mw / w, and past it `usize::MAX`,
+            // where the background stays. Rows alike.
+            let source = |r: u64, reduced: u32, full: u32, at: u32, own: u32, stored: u64| -> usize {
+                let g = (r * full as u64 / reduced as u64) as i64 - at as i64;
+                if g < 0 || g >= own as i64 { usize::MAX } else { (g as u64 * stored / own as u64) as usize }
+            };
+            let xs: Vec<usize> = (0..rw as u64).map(|x| source(x, rw, grid.width, grid.x, grid.w, mw)).collect();
             let src = m.pixels.bytes();
             for y in 0..rh as u64 {
-                let sy = (y * mh / rh as u64) as usize;
+                let sy = source(y, rh, grid.height, grid.y, grid.h, mh);
+                if sy == usize::MAX { continue; }
                 let row_src = &src[sy * mw as usize..(sy + 1) * mw as usize];
                 let row_dst = &mut data[y as usize * rw as usize..(y as usize + 1) * rw as usize];
-                for (dst, &sx) in row_dst.iter_mut().zip(xs.iter()) { *dst = row_src[sx]; }
+                for (dst, &sx) in row_dst.iter_mut().zip(xs.iter()) { if sx != usize::MAX { *dst = row_src[sx]; } }
             }
         }
-        let coverage = ops::adjust::edit_coverage(doc, clips, &placement, rw, rh).ok()?;
-        paint_grid(doc, &mut data, rw, rh, &placement, coverage.as_ref(), &paint, true);
+        let coverage = ops::adjust::edit_coverage(doc, clips, &grid.transform, rw, rh).ok()?;
+        paint_grid(doc, &mut data, rw, rh, &grid.transform, coverage.as_ref(), &paint, true);
         let shown = GrayRaster::from_bytes(rw, rh, data);
         let pixels = layer.pixels.clone().unwrap_or_else(|| Raster::new_transparent(1, 1));
-        return Some(PixelPreview::new(id, pixels, layer.transform, revision, request, made_from, PreviewTarget::Mask(shown)));
+        return Some(PixelPreview::new(id, pixels, layer.transform, revision, request, made_from, PreviewTarget::Mask { pixels: shown, placement: grid.placement }));
     }
     let grid = image_grid(doc, layer).ok()?;
     let stored = layer.pixels.as_ref();
```

The engine.rs hunks below are against the file as Task 14 leaves it: `Engine::edit_pixels` exactly as "Consequences for later briefs" (Task 14, item T14-5) has Task 14 write it, after `mask_delta`. Its mask branch counted the mask as it is painted before this task (its own grid, or its layer's); it now counts the grid `paint_layer` paints, grown to the canvas, through `painted_pixels`.

```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -292,23 +292,14 @@ impl Engine {
         let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         Ok(self.session(id)?.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
     }
-    /// The pixels a Fill or a Gradient on `layer` (its mask when `mask`) paints, from the stored
-    /// document (ruling C1): what decides whether the edit goes to the job worker, never the size the
-    /// layer stores (a blank layer on a 100 MP canvas paints 100 MP). An error where the edit is refused
-    /// for its size.
+    /// The pixels a Fill or a Gradient on `layer` (its mask when `mask`) paints, from the stored
+    /// document (ruling C1; `ops::raster_edit::painted_pixels`): what decides whether the edit goes to the
+    /// job worker, never the size the layer stores (a blank layer on a 100 MP canvas paints 100 MP, and so
+    /// does a small layer's mask, grown to that canvas: Task 14a). An error where the edit is refused for
+    /// its size, which the worker, seeing one layer, could not tell.
     pub fn edit_pixels(&self, id: Uuid, layer: Uuid, mask: bool) -> Result<u64, CommandError> {
-        let doc = &self.session(id)?.document;
-        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
-        if mask {
-            // A mask on its own placement is painted in its own grid, a covering one on its layer's
-            // (`raster_edit::paint_layer`).
-            let m = l.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
-            if m.placement.is_some() { return Ok(m.pixels.width as u64 * m.pixels.height as u64); }
-            return Ok(l.pixels.as_ref().map_or(l.transform.size.width.round().max(1.0) as u64 * l.transform.size.height.round().max(1.0) as u64, |p| p.width as u64 * p.height as u64));
-        }
-        let grid = ops::raster_edit::image_grid(doc, l)?;
-        Ok(grid.width as u64 * grid.height as u64)
+        ops::raster_edit::painted_pixels(&self.session(id)?.document, layer, mask)
     }
     /// The preview showing on the canvas, if any.
     pub fn preview(&self, id: Uuid) -> Option<&PixelPreview> { self.sessions.get(&id).and_then(|s| s.preview.as_ref()) }
     /// The layer's mask as the canvas shows it (a gradient's mask preview in its place).
@@ -367,8 +358,12 @@ impl Engine {
         let Some(preview) = &s.preview else { return Ok(std::borrow::Cow::Borrowed(&s.document)); };
         let mut doc = s.document.clone();
         match &preview.target {
-            PreviewTarget::Mask(mask) => {
-                if let Some(m) = doc.layer_mut(preview.layer).and_then(|l| { l.mask_revision = preview.revision; l.mask.as_mut() }) { m.pixels = mask.clone(); }
+            PreviewTarget::Mask { pixels, placement } => {
+                if let Some(m) = doc.layer_mut(preview.layer).and_then(|l| { l.mask_revision = preview.revision; l.mask.as_mut() }) {
+                    m.pixels = pixels.clone();
+                    // Where the commit will leave it: a mask gradient grows the mask past its layer (Task 14a).
+                    m.placement = *placement;
+                }
                 return Ok(std::borrow::Cow::Owned(doc));
             }
             PreviewTarget::Patch(_) => {
```

```diff
--- a/engine/src/document.rs
+++ b/engine/src/document.rs
@@ -171,15 +171,24 @@ impl Document {
 impl Mask {
     pub fn is_linked(&self) -> bool { self.linked != Some(false) }
     pub fn is_uniform(&self) -> bool { self.pixels.width == 1 && self.pixels.height == 1 }
-    /// What the mask shows beyond its pixels: white or black, whichever most of its edge is.
+    /// What the mask shows beyond its pixels: white or black, whichever most of its edge is, each edge
+    /// pixel counted once, at full size (`LayerMask.background`, LayerMask.swift:61-76, reads the edge
+    /// of the mask's thumbnail, at most 96 px on its long side: the same up to 96 px, a disclosed
+    /// difference past it). Reads the edge alone: a grown mask (Task 14a) can hold 100 MP, and the plan
+    /// and `state()` ask for this on every frame.
     pub fn background(&self) -> u8 {
         let (w, h) = (self.pixels.width as usize, self.pixels.height as usize);
+        if w == 0 || h == 0 { return 255; }
         let d = self.pixels.bytes();
-        let (mut total, mut count) = (0u64, 0u64);
-        for y in 0..h { for x in 0..w {
-            if y == 0 || y == h - 1 || x == 0 || x == w - 1 { total += d[y * w + x] as u64; count += 1; }
-        }}
-        if count == 0 || total * 2 >= count * 255 { 255 } else { 0 }
+        let row = |y: usize| d[y * w..(y + 1) * w].iter().map(|&v| v as u64).sum::<u64>();
+        let (mut total, mut count) = (row(0), w as u64);
+        if h > 1 { total += row(h - 1); count += w as u64; }
+        for y in 1..h.saturating_sub(1) {
+            total += d[y * w] as u64;
+            count += 1;
+            if w > 1 { total += d[y * w + w - 1] as u64; count += 1; }
+        }
+        if total * 2 >= count * 255 { 255 } else { 0 }
     }
     /// Where the mask sits once its layer moves from `old` to `new` (the macOS placement rule).
     pub fn follow(&self, old: &LayerTransform, new: &LayerTransform) -> Option<LayerTransform> {
```

The rectangle a selection's clip fills, without filling it (audit I-5; `new` is left as it is, and `a_selections_region_is_the_rectangle_its_clip_fills` pins the two equal):

```diff
--- a/engine/src/selection/coverage.rs
+++ b/engine/src/selection/coverage.rs
@@ -163,6 +163,20 @@ impl SelectionClip {
         SelectionClip { origin: (x0 as i64, y0 as i64), coverage: Some(coverage) }
     }
 
+    /// The rectangle `new` fills, from the selection's bounds alone, without filling it: (x0, y0, x1,
+    /// y1) in document pixels, None where `new`'s coverage is None. What a mask edit reads to find the
+    /// tiles it paints (`raster_edit::mask_grid`, Task 14a), so the app's `editPixels` question never
+    /// rasterizes or feathers the outline on the UI thread (pre-flight audit I-5). The rounding is
+    /// `new`'s; mask_grow.rs pins the two equal.
+    pub fn region(selection: &Selection, canvas_width: u32, canvas_height: u32) -> Option<(i64, i64, i64, i64)> {
+        if selection.is_empty() { return None; }
+        let b = selection.coverage_bounds()?;
+        let (x0, y0) = (((b.x - 1.0).floor()).max(0.0), ((b.y - 1.0).floor()).max(0.0));
+        let (x1, y1) = (((b.max_x() + 1.0).ceil()).min(canvas_width as f64), ((b.max_y() + 1.0).ceil()).min(canvas_height as f64));
+        if !(x1 - x0 >= 1.0) || !(y1 - y0 >= 1.0) { return None; }
+        Some((x0 as i64, y0 as i64, x1 as i64, y1 as i64))
+    }
+
     /// The coverage at a document point: 0 outside the region, else sampled bilinearly between the
     /// region's pixel centres, its edge pixels repeated.
     pub fn at(&self, p: Point) -> f32 {
```

Nothing else changes, and why:
- The app. Fill and Delete on a targeted mask already send `Fill { mask: true }` and decide the worker by `editPixels` (Task 14, as specified in "Consequences"); the wasm `edit_pixels` and `EngineClient.editPixels` Task 14 adds pass the engine's count through, grown now. The unit tests stub `editPixels`, so they do not change.
- Two older mask commands stay inside the mask, and why. `ClearSelectedPixels { mask: true }` (Phase 4a, fills white inside the selection) is no longer sent by the app once Task 14 routes Delete on a mask through `Fill`; it is left as it is (its tests pin it) and is not the Mac's path. `FillMask` (Layer > Mask > Fill Mask White / Black, a port command since Phase 2 with no counterpart in 1.3.7's menus) replaces the mask's own pixels; it is not a raster edit, so it does not grow. Invert Mask is `invertPixels` on the Mac, not a raster edit, so it does not grow there either.
- The renderers. The GPU and the CPU compositor draw from the same displayed document (`render_document` / `render_bytes`), whose preview mask now carries its placement; the plan's `Coverage` takes `placement`, `width`, `height` and `background` from it (plan.rs:123-136), and both renderers place the mask from the coverage (gl-renderer.ts:300; compositor.rs:156-161). A mask texture is keyed by its revision and size (mask-textures.ts:22-25, :34): a preview has a revision of its own that no lineage entry reaches, so `mask_delta` returns None and the mask uploads whole; a commit that grew the mask changes its size and grid, which the lineage records whole (lineage.rs:55-56); undo and redo the same (lineage.rs:71). Whole uploads are what this needs.
- The job worker. `run_edit_job` returns the new mask with its placement and `install_job` sets both (jobs.rs:211-215, :230-238); the grown mask always comes back as a buffer (its size changed), so the parked Task 5 minor (placement-only mask changes dropped) is not reached: a mask edit that does not grow keeps its placement exactly. In the worker the budget sees one layer (ruling OQ20's gap), which is why the app asks `editPixels` on the UI thread first and sends nothing it refuses (Task 14's `fillActive`, Task 15's `commitGradient`, "Consequences").
- Parked T9-6 (a reduced PIXELS preview on a growing layer with a non-uniform covering mask stretches the mask over the grown box) is not made reachable here: a mask edit never changes the layer's pixels or transform, and a mask gradient's preview now shows the mask at its own placement. After a mask fill or gradient grows a mask, that mask is placed, so T9-6 no longer applies to that layer. It stays parked for Task 15 and the final review; this task's `PreviewTarget::Mask { placement }` is the shape of its fix (see "Consequences").

- [ ] **Step 4: Run the tests and watch them pass**

`cargo test -p compositor-engine`: every binary passes; `mask_grow.rs` adds 12 tests (11 and the region test of audit I-5), `perf_4b1.rs` one ignored. `pnpm wasm:dev`; `pnpm test` (unchanged); `pnpm build`; `pnpm e2e`: +2 (`mask-grow.spec.ts`), the two new perf cases skipped without PERF. Timings, reported in the task report with the renderer string (UNMASKED_RENDERER_WEBGL; SwiftShader or Basic Render is not a valid run): `cargo test --release -p compositor-engine --test perf_4b1 -- --ignored --nocapture --test-threads=1 a_mask_gradient_and_a_mask_fill_grown_to_the_canvas_at_24_and_100_mp` (the ticks, the gradient against 650 / 2710 ms, the fill against 266 / 686 ms, the ceilings: a measured fill under two thirds of its ceiling tightens its assert to 1.5x the measurement, rounded up, per the controller's ruling above, reported and commented); then `pnpm wasm`, `$env:PERF = "1"; npx playwright test app/tests/e2e/perf-4b1.spec.ts -g "grow"` (the gradient ticks: drag < 50, settled < 150 at fit and 1:1, the frame after applying < 150, the diagnostic UI-thread apply; the fill through the worker, without a selection and under a feathered Select All, first and second fill: the key press < 150, the longest gap < 100, `jobInput` < 150, `installJob` < 150 / 450, the frame after it < 150; the nudge, Undo and Redo: each frame after < 150, the UI-thread times and whole uploads logged), and `pnpm wasm:dev` afterwards. A budget missed is reported with its numbers, never widened.

- [ ] **Step 5: Prove it bites**

Each change below is made on the final code, the named test run and seen to fail, and the file then restored: `Copy-Item <file> <file>.bak` before the change, `Move-Item -Force <file>.bak <file>` after, then `(Get-Item <file>).LastWriteTime = Get-Date` (never `git checkout`).

(1) In `mask_grid` (raster_edit.rs), change `if grows {` to `if !grows {` (no mask edit grows): `a_mask_gradient_grows_the_mask_to_the_canvas_places_it_there_and_undoes` fails at `edit_pixels` (200, not 4,000), and `a_fill_on_a_mask_without_a_selection_grows_it_to_the_whole_grown_grid` at `edit_pixels` (200, not 4,950). Restore. (2) In `paint_layer`'s mask branch only, pass `matches!(paint, Paint::Gradient(_))` for `grows` (the old rule, a Fill keeping to the mask): `a_fill_on_a_mask_without_a_selection_grows_it_to_the_whole_grown_grid` fails at the size ((20, 10), not (110, 45)). Restore. (3) In `mask_on_grid`, start from `vec![255u8; ...]` instead of `m.background()`: `the_grown_area_starts_as_the_masks_background_white_or_black` fails at "black background, column 5" (13, not 0). Restore. (4) In `mask_grid`, drop the tiles: replace the two `let (tx..` lines with `let (tx0, ty0, tx1, ty1) = (rx0, ry0, rx1, ry1);`: `inside_a_selection_the_mask_keeps_its_old_grid_and_the_whole_tiles_the_edit_touched` fails at `edit_pixels` (501 x 301 = 150,801, not 550 x 350 = 192,500), and `a_fill_on_a_mask_inside_a_selection_grows_it_by_the_tiles_the_clip_touches` at `edit_pixels` (251 x 201 = 50,451, not 318 x 262 = 83,316). Restore. (5) In `mask_grid`, delete the `Some(p) if m.pixels.width <= 2 ...` arm: `a_solid_mask_on_its_own_placement_is_painted_at_one_pixel_per_document_pixel` fails ((4, 3), not (99, 40): its one pixel is 30.4 x 20 document pixels). Restore. (6) In `mask_grid`'s budget check, use `(w as u64) * (h as u64)` for the area: `a_mask_fill_or_gradient_is_refused_past_the_mask_budget_at_its_grown_size` fails (the Fill is accepted: `Ok(..)`, not the TooLarge error). Restore. (7) In `displayed` (engine.rs), delete `m.placement = *placement;`: `the_preview_shows_the_grown_mask_where_the_commit_leaves_it` fails (the previewed placement is None, the committed one Some), and after `pnpm wasm:dev` the e2e fails at `previewed.placement` (null) and at the previewed alphas (the 120 x 80 mask stretched over the 40 x 30 layer: at x 32 about 16 for 69). Restore and rebuild. (8) In `Mask::background`, delete the line `if h > 1 { total += row(h - 1); count += w as u64; }`: `a_masks_background_is_the_majority_of_its_edge_pixels_each_counted_once` fails at 6 x 5 (0, not 255). Restore. (9) In `SelectionClip::region` (coverage.rs), change `(b.x - 1.0).floor()` to `b.x.floor()`: `a_selections_region_is_the_rectangle_its_clip_fills` fails at the first selection (x0 10, not 9; audit I-5). Restore. (10) In `Mask::background`, drop the `if w > 1` guard (`total += d[y * w + w - 1] as u64; count += 1;` always): `a_masks_background_is_the_majority_of_its_edge_pixels_each_counted_once` fails at 1 x 4 (0, not 255; audit M-8). Restore.

- [ ] **Step 6: Commit**

```
git add -- engine/tests/mask_grow.rs app/tests/e2e/mask-grow.spec.ts
git commit -m "feat(engine): a fill or a gradient on a mask grows the mask past its layer to the canvas, as Compositor 1.3.7 does" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>" -- engine/src/ops/raster_edit.rs engine/src/preview.rs engine/src/engine.rs engine/src/document.rs engine/src/selection/coverage.rs engine/tests/mask_grow.rs engine/tests/gradient_preview.rs engine/tests/perf_4b1.rs app/tests/e2e/mask-grow.spec.ts app/tests/e2e/perf-4b1.spec.ts
```

### Consequences for later briefs

(Record of what the planner applied. Where the pre-flight audit of 2026-09-29 later amended task-14, task-15 or task-16-brief.md, those briefs' own "Controller amendments" blocks and text win over the copies below.)

Task 14 runs before this task but its brief is not dispatched yet, so its changes are listed here too: ruling C1's `Engine::edit_pixels` is written by Task 14 exactly as below, and this task's engine.rs hunk replaces its body. Task 14's line numbers are the plan's (docs/superpowers/plans/2026-09-28-phase4b1-groundwork-and-colour.md, "### Task 14" at 9194); in the workspace's task-14-brief.md each is 9193 less. Task 15's and Task 16's are their briefs' as they stand. Where the controller has already carried ruling C1 into those briefs in other words, these replace that text.

**Task 14 (Fill)**

Why a change there rather than a replacement here alone: ruling C1 has Task 14 add `Engine::edit_pixels` but its brief does not say how, and this task's hunk must apply to known text. Task 14 counts a mask as it is painted then (its own grid, or its layer's); Task 14a replaces the body with `painted_pixels`, which counts the mask grown to the canvas. Task 14's store code and tests stub `editPixels`, so they need no change at 14a.

(T14-1) Plan line 9196, the statement: replace "and a large layer is filled by the job worker." with "and a fill that paints more than `JOB_PIXELS` is made by the job worker, decided by the pixels it paints (`Engine::edit_pixels`, ruling C1: a blank layer on a large canvas paints the canvas), never the size the layer stores; a fill the engine refuses for its size shows why and sends nothing."

(T14-2) Line 9199, Files: after `app/src/shortcuts/useShortcuts.ts` add ", `app/src/engine/client.ts` (`editPixels`), `engine/src/engine.rs` (`edit_pixels`), `engine-wasm/src/lib.rs` (`edit_pixels`)"; line 9200: after "`app/tests/unit/selection-store.test.ts` (Delete on a mask: a `Fill` with white)" add ", `engine/tests/raster_edits.rs` (`edit_pixels`)".

(T14-3) Line 9203, Interfaces: add "`Engine::edit_pixels(id, layer, mask) -> Result<u64, CommandError>`, `EngineClient.editPixels(doc, layer, mask): number` (throws where the edit is refused for its size; ruling C1)" to Produces.

(T14-4) Line 9207, Step 1's summary: after "a large layer goes to the worker;" add " the worker is chosen by the pixels the fill paints (a blank layer on a 6000 x 4000 canvas goes, by `editPixels`), and a fill the engine refuses for its size sends nothing; `edit_pixels` counts a blank layer's canvas and a small layer grown to its canvas;".

(T14-5) Step 3, three new hunks, against the files as they are now:
```diff
--- a/engine/src/engine.rs
+++ b/engine/src/engine.rs
@@ -292,6 +292,23 @@ impl Engine {
         let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
         Ok(self.session(id)?.lineage.delta(layer, Plane::Mask, from, l.mask_revision))
     }
+    /// The pixels a Fill or a Gradient on `layer` (its mask when `mask`) paints, from the stored
+    /// document (ruling C1): what decides whether the edit goes to the job worker, never the size the
+    /// layer stores (a blank layer on a 100 MP canvas paints 100 MP). An error where the edit is refused
+    /// for its size.
+    pub fn edit_pixels(&self, id: Uuid, layer: Uuid, mask: bool) -> Result<u64, CommandError> {
+        let doc = &self.session(id)?.document;
+        let l = doc.layer(layer).ok_or(CommandError::NoLayer)?;
+        if mask {
+            // A mask on its own placement is painted in its own grid, a covering one on its layer's
+            // (`raster_edit::paint_layer`).
+            let m = l.mask.as_ref().ok_or_else(|| CommandError::Argument("the layer has no mask".into()))?;
+            if m.placement.is_some() { return Ok(m.pixels.width as u64 * m.pixels.height as u64); }
+            return Ok(l.pixels.as_ref().map_or(l.transform.size.width.round().max(1.0) as u64 * l.transform.size.height.round().max(1.0) as u64, |p| p.width as u64 * p.height as u64));
+        }
+        let grid = ops::raster_edit::image_grid(doc, l)?;
+        Ok(grid.width as u64 * grid.height as u64)
+    }
     /// The preview showing on the canvas, if any.
     pub fn preview(&self, id: Uuid) -> Option<&PixelPreview> { self.sessions.get(&id).and_then(|s| s.preview.as_ref()) }
     /// The layer's mask as the canvas shows it (a gradient's mask preview in its place).
```
```diff
--- a/engine-wasm/src/lib.rs
+++ b/engine-wasm/src/lib.rs
@@ -326,6 +326,12 @@ impl WasmEngine {
         let rect = self.engine.mask_delta(parse_id(doc)?, parse_id(layer)?, from as u64).map_err(js_err)?;
         Ok(rect.map_or_else(Vec::new, |r| vec![r.x as f64, r.y as f64, r.width as f64, r.height as f64]))
     }
+    /// `Engine::edit_pixels`: the pixels a Fill or a Gradient on the layer (its mask when `mask`) paints
+    /// (ruling C1). Throws where the edit is refused for its size. Counts stay below 2^53, so they travel
+    /// as numbers.
+    pub fn edit_pixels(&self, doc: &str, layer: &str, mask: bool) -> Result<f64, JsError> {
+        Ok(self.engine.edit_pixels(parse_id(doc)?, parse_id(layer)?, mask).map_err(js_err)? as f64)
+    }
     /// The mask as the canvas shows it (`Engine::mask_pixels`): its buffer is the document's, or an
     /// open mask preview's, which lives until the next engine call either way.
     pub fn mask_pixels_ptr(&self, doc: &str, layer: &str) -> Result<*const u8, JsError> {
```
```diff
--- a/app/src/engine/client.ts
+++ b/app/src/engine/client.ts
@@ -165,6 +165,9 @@ export class EngineClient {
   }
   /** `pixelsDelta` for the layer's mask, in the mask's own grid (engine `mask_delta`). */
   maskDelta(doc: string, layer: string, from: number): PixelRect | null { return rectOf(this.wasm.mask_delta(doc, layer, from)); }
+  /** The pixels a Fill or a Gradient on the layer, or its mask, paints (engine `edit_pixels`, ruling C1):
+   * what decides the job worker. Throws where the edit is refused for its size. */
+  editPixels(doc: string, layer: string, mask: boolean): number { return this.wasm.edit_pixels(doc, layer, mask); }
   clipDependents(doc: string, ids: string[]): string[] { return JSON.parse(this.wasm.clip_dependents(doc, JSON.stringify(ids))) as string[]; }
   mergeAction(doc: string, ids: string[]): string | null { return this.wasm.merge_action(doc, JSON.stringify(ids)) ?? null; }
   groupBox(doc: string, ids: string[]): LayerTransform | null { const t = this.wasm.group_box(doc, JSON.stringify(ids)); return t ? (JSON.parse(t) as LayerTransform) : null; }
```

(T14-6) Lines 9463-9472, `fillActive` in the `app/src/actions/layers.ts` hunk: replace those ten `+` lines with the seventeen below, and change the hunk header on line 9439 from `@@ -54,7 +54,35 @@` to `@@ -54,7 +54,42 @@`. Line 9463's text is kept, as Task 15's layers.ts hunk (task-15-brief.md:522) takes it as context. `editPixels` is asked only when there is a job worker (as `usesJob` asks `!!jobs` first): without one, `run` sends the fill and the engine refuses it itself, and store tests whose stub engine has no worker (selection-store.test.ts's Delete on a mask) need no `editPixels`.
```ts
+/** Alt+Backspace / Ctrl+Backspace: the selection (or the whole layer) filled with the foreground or
+ * background colour, one undo step; on a mask its black or white (`fillSelection`,
+ * SelectionEdits.swift:40-50). With the job worker, a fill that paints more than `jobPixels` goes to
+ * it, decided by the pixels it paints (`editPixels`, ruling C1), not the size the layer stores; one the
+ * engine refuses for its size shows why and sends nothing (the worker, seeing one layer, could not). */
+export function fillActive(background: boolean): void {
+  if (!canPaint()) return;
+  const c = ctx()!;
+  const mask = c.s.maskTargeted();
+  const command = { type: "Fill" as const, id: c.active!.id, mask, color: colorTuple(c.s.paletteColor(background)) };
+  if (c.s.jobs) {
+    let painted: number;
+    try { painted = c.s.engine!.editPixels(c.s.activeId!, c.active!.id, mask); }
+    catch (e) { useEditor.setState({ error: String(e instanceof Error ? e.message : e) }); return; }
+    if (painted > c.s.jobPixels) { void c.s.runEditJob(command, c.active!.id); return; }
+  }
+  c.s.run(command);
```

(T14-7) `fill-actions.test.ts`: after line 9332 (the stub's `setPreview`) add
```ts
    editPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
```
and after line 9395 (the end of the Delete case) add
```ts
  it("decides the job worker by the pixels the fill paints, not the size the layer stores (ruling C1)", () => {
    // A blank layer stores nothing but paints its 6000 x 4000 canvas.
    install(layer({ hasPixels: false, pixelsWidth: 0, pixelsHeight: 0 }));
    useEditor.setState({ engine: { ...s().engine!, editPixels: () => 6000 * 4000 } as never });
    fillActive(false);
    expect(log).toEqual(["job input"]);
  });
  it("refuses a fill too large to paint before anything is sent", () => {
    useEditor.setState({ engine: { ...s().engine!, editPixels: () => { throw new Error("too large"); } } as never });
    fillActive(false);
    expect([log, s().error]).toEqual([[], "too large"]);
  });
```
(The case "sends a large layer to the job worker", `jobPixels` 40 x 30 - 1, still sends one: the stub counts 1,200.)

(T14-8) `engine/tests/raster_edits.rs`: append
```rust

#[test]
fn edit_pixels_counts_what_a_fill_paints_not_what_the_layer_stores() {
    // Ruling C1: the job worker is chosen by the grid `paint_layer` paints. A blank layer stores nothing
    // but paints its 100 x 40 canvas; a 20 x 10 layer at (30, 15) grows to that canvas too.
    let (e, id, layer) = blank(100, 40);
    assert_eq!(e.edit_pixels(id, layer, false).unwrap(), 100 * 40);
    let mut doc = Document::new(100, 40);
    let small = Layer::with_pixels("Small", Raster::from_premultiplied(20, 10, [0, 0, 200, 255].repeat(200)), p(30.0, 15.0));
    let lid = small.id;
    doc.active_layer_id = Some(lid);
    doc.layers = vec![small];
    let mut e = Engine::new();
    let id = e.insert_document(doc);
    assert_eq!(e.edit_pixels(id, lid, false).unwrap(), 100 * 40);
}
```
(The mask side is pinned by Task 14a's `mask_grow.rs`, at its grown size.)

(T14-9) Line 9432, Step 2: add "`cargo test -p compositor-engine --test raster_edits`: `edit_pixels` does not exist." Line 9552, Step 4: `pnpm test: 187 (+4)` becomes `189 (+6)`; add "Engine: +1 test (raster_edits)." Line 9556, Step 5: add "(3) In `fillActive`, decide by `c.s.usesJob(c.active!.id)` again: 'decides the job worker by the pixels the fill paints' fails (the blank layer is filled on the UI thread: the command, not `job input`). Restore. (4) In `Engine::edit_pixels`, return the stored pixels (`l.pixels.as_ref().map_or(0, |p| p.width as u64 * p.height as u64)`) on the pixels side: `edit_pixels_counts_what_a_fill_paints_not_what_the_layer_stores` fails (0, not 4,000). Restore." Line 9562, Step 6: add `engine/src/engine.rs engine-wasm/src/lib.rs app/src/engine/client.ts engine/tests/raster_edits.rs` to the paths after `--`.

(T14-10) Ruling C1's perf cases (Alt+Backspace and Return on `newDocument(w, h, true)` at 24 and 100 MP, preflight.md:269) stay with the controller's carry to Tasks 14 and 15; Task 14a adds the mask's (a fill on a small layer's mask through the worker).

**Task 15 (the Gradient tool)**

(1) Line 3, the statement's last sentence. Replace
"Committing goes through the worker on a large layer - chosen by the stored layer's size, not the reduced preview `state()` reports (ruling OQ9, found by this task's perf run; `Engine::stored_pixels`)."
with
"Committing goes through the worker when the gradient paints more than `JOB_PIXELS`, counted by `Engine::edit_pixels` (ruling C1, Task 14; a mask counted grown, Task 14a) from the stored document: the layer's grid grown to the canvas or, on a mask, the mask grown to the canvas (a gradient on a mask grows the mask past its layer, as Compositor 1.3.7 does), never the reduced preview `state()` reports (ruling OQ9); a gradient too large to paint is refused before anything is sent. `usesJob` decides the other large-layer paths by the stored layer's size (`Engine::stored_pixels`)."

(2) Line 11, Interfaces: add a line before "- Produces:":
"- Consumes: `Engine::edit_pixels`, `EngineClient.editPixels` (Task 14, ruling C1; a mask counted at its grown size, Task 14a): throws where the edit is refused for its size."

(3) Line 15, Step 1's summary. Replace "a large layer (by stored size, even under a reduced preview) goes to the worker;" with "a large layer (by stored size, even under a reduced preview) is large to `usesJob`; a gradient goes to the worker by the pixels it paints, a small layer's mask grown to a large canvas at its grown size, and one too large to paint is refused before anything is sent;".

(4) Line 306, in `gradient-store.test.ts`'s `install`: after the `storedPixels` line add
```ts
    editPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
```
(The existing "goes to the job worker on a large layer" case at 380-388 then still sends a job: 40 x 30 = 1,200 > `jobPixels` 1,199.)

(4b) Lines 239-249, the `fill-actions.test.ts` hunk: Task 14 (T14-7) adds an `editPixels` line to that stub after `setPreview`, inside this hunk's trailing context. After line 248 (`setPreview: ...`) add the context line
```ts
     editPixels: () => useEditor.getState().documents.D.layers[0].pixelsWidth * useEditor.getState().documents.D.layers[0].pixelsHeight,
```
(one space, then the four of indentation) and change the header on line 242 from `@@ -24,6 +24,7 @@` to `@@ -24,7 +24,8 @@`.

(5) Lines 389-396: the case "chooses the job worker by the stored layer's size, not the reduced preview the state shows" no longer passes through `commitGradient` (its stub would count the shown 10 x 8 and run on the UI thread). Replace lines 389-396 with:
```ts
  it("chooses the job worker by the stored layer's size, not the reduced preview the state shows", () => {
    // The state reports a previewed layer at its preview's size: 10 x 7.5 of the stored 40 x 30.
    const shown = { ...s().documents.D, layers: s().documents.D.layers.map((l) => ({ ...l, pixelsWidth: 10, pixelsHeight: 8 })) };
    useEditor.setState({ jobPixels: 100, engine: { ...s().engine!, state: () => shown, storedPixels: () => 40 * 30 } as never });
    draw([5, 6], [30, 6]);
    expect(s().usesJob("A")).toBe(true);
  });
  it("sends a gradient to the job worker by the pixels it paints: a small layer's mask grown to a large canvas counts grown", () => {
    // 40 x 30 pixels, but on its mask the gradient paints the mask grown to a 6000 x 4000 canvas (Task 14a).
    useEditor.setState({ jobPixels: 40 * 30, engine: { ...s().engine!, editPixels: (_d: string, _l: string, mask: boolean) => (mask ? 6000 * 4000 : 40 * 30) } as never });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual([`execute ${JSON.stringify({ type: "Gradient", id: "A", mask: false, gradient: last().gradient })}`]);
    log.length = 0;
    useEditor.setState({ maskSelected: true });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect(log).toEqual(["job input"]);
  });
  it("refuses a gradient too large to paint before anything is sent", () => {
    useEditor.setState({ maskSelected: true, engine: { ...s().engine!, editPixels: () => { throw new Error("too large"); } } as never });
    draw([5, 6], [30, 6]);
    s().commitGradient();
    expect([log, s().gradientEdit, s().error, previews.at(-1)]).toEqual([[], null, "too large", null]);
  });
```

(6) Line 1077, in `commitGradient`. Replace
```ts
+    if (get().usesJob(e.layerId)) { void get().runEditJob(command, e.layerId); return; }
```
with
```ts
+    // By the pixels the gradient paints (ruling C1, as `fillActive` does; Task 14a): on a mask, the mask
+    // grown to the canvas, so a small layer's mask on a large canvas is a large edit. One too large to
+    // paint is refused here, as the commit would refuse it: the job worker sees one layer, not the budget
+    // the others leave.
+    // Asked only with a job worker, as `usesJob` and `fillActive` do.
+    if (get().jobs) {
+      let painted: number;
+      try { painted = get().engine!.editPixels(get().activeId!, e.layerId, e.mask); }
+      catch (err) {
+        set({ error: String(err instanceof Error ? err.message : err) });
+        const { engine, activeId } = get(); if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); }
+        return;
+      }
+      if (painted > get().jobPixels) { void get().runEditJob(command, e.layerId); return; }
+    }
```

(7) Lines 172-236, the perf case: the "gradient tool" case keeps its canvas-covering layer (`editPixels` is the canvas there, so it still goes to the worker). Add a second case after it: in the diff at line 164-237, append these lines after line 236 (`+});`) and change the hunk header on line 167 from `@@ -368,3 +368,69 @@` to `@@ -368,3 +368,132 @@` (63 lines added). Leave line 236 and the two lines before it as they are: Task 16's perf hunk (task-16-brief.md:20-23) takes them as its context.
```ts
+
+test("gradient tool on a small layer's mask: the mask grows to the canvas and the commit goes through the worker, at 24 and 100 MP", async ({ page }) => {
+  test.setTimeout(900_000);
+  const out: Record<string, number> = {};
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    await ready(page);
+    await installFrameTimer(page);
+    const r = await page.evaluate(async ([w, h]) => {
+      const api = (window as any).__compositor; const frame = (window as any).__frame as () => number;
+      const settle = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
+      const result: Record<string, number> = {};
+      let installedAt = Infinity, jobs = 0;
+      const timed = (name: string, after?: () => void) => {
+        const f = api.engine[name].bind(api.engine);
+        api.engine[name] = (...a: unknown[]) => { const t0 = performance.now(); try { return f(...a); } finally { result[`${name} ms`] = Math.round(performance.now() - t0); after?.(); } };
+      };
+      timed("jobInput");
+      timed("installJob", () => { installedAt = performance.now(); });
+      const client = api.store.getState().jobs; const send = client.run.bind(client);
+      client.run = (...a: unknown[]) => { jobs++; return send(...a); };
+      // A 1500 x 1000 layer in the middle of the canvas under a white mask, targeted (Task 14a).
+      const doc = api.engine.newDocument(10, 10, false);
+      api.engine.execute(doc, { type: "CanvasSize", width: 1500, height: 1000, anchor: 4, fill: [0.5, 0.4, 0.3] });
+      const layer = api.engine.state(doc).layers[0].id;
+      api.engine.execute(doc, { type: "CanvasSize", width: w, height: h, anchor: 4, fill: null });
+      api.engine.execute(doc, { type: "SetActiveLayer", id: layer });
+      api.engine.execute(doc, { type: "AddMask", id: layer, revealing: true });
+      const s = () => api.store.getState();
+      s().openDocument(doc);
+      s().setMaskSelected(true);
+      s().setTool("gradient");
+      await settle(); frame();
+      s().beginGradient({ x: w * 0.2, y: h * 0.5 });
+      s().moveGradient({ end: { x: w * 0.8, y: h * 0.6 } }, true);
+      s().endGradientDrag(); frame();
+      const longestGap = (until: () => boolean) => new Promise<number>((done) => {
+        let last = performance.now(), gap = 0;
+        const tick = (t: number) => { if (t <= installedAt) gap = Math.max(gap, t - last); last = t; if (until()) done(Math.round(gap)); else requestAnimationFrame(tick); };
+        requestAnimationFrame(tick);
+      });
+      const t0 = performance.now();
+      s().commitGradient();
+      result["Return (UI thread) ms"] = Math.round(performance.now() - t0);
+      result["commit: longest frame gap while the worker paints"] = await longestGap(() => !s().working);
+      result["frame after it ms"] = Math.round(frame());
+      result["jobs run"] = jobs;
+      const l = api.engine.state(doc).layers[0];
+      result["mask pixels"] = l.maskWidth * l.maskHeight;
+      return result;
+    }, [w, h]);
+    for (const [k, v] of Object.entries(r)) out[`${label}: ${k}`] = v;
+  }
+  console.log(`gradient tool on a growing mask (release wasm, Edge): ${JSON.stringify(out)}`);
+  for (const [label, w, h] of [["24 MP", 6000, 4000], ["100 MP", 10000, 10000]] as [string, number, number][]) {
+    expect(out[`${label}: jobs run`], "the grown mask went to the worker").toBe(1);
+    expect(out[`${label}: mask pixels`]).toBe(w * h);
+    expect(out[`${label}: Return (UI thread) ms`]).toBeLessThan(150);
+    expect(out[`${label}: commit: longest frame gap while the worker paints`]).toBeLessThan(100);
+    expect(out[`${label}: jobInput ms`]).toBeLessThan(150);
+    expect(out[`${label}: installJob ms`]).toBeLessThan(label === "24 MP" ? 150 : 500);
+    expect(out[`${label}: frame after it ms`]).toBeLessThan(150);
+  }
+});
```
Budgets: `jobInput` copies the small layer and its mask (7.5 MB) at either size, so 150 at both; `installJob` copies the grown mask back (24 or 100 MB), so the OQ5 copy budgets 150 / 500; Return (the `editPixels` question and the copy out) 150; the gap 100 and the frame 150 as in the "gradient tool" case and Task 14a.

(8) Line 1257, Step 4: `pnpm test: 199 (+12)` becomes `pnpm test: 201 (+14)` (two more store cases); the e2e count is unchanged (the new perf case is skipped without PERF).

(9) Line 1261, Step 5: keep (1) and (2) (in (2), the failing case now fails at `expect(s().usesJob("A")).toBe(true)`), and add: "(3) In `commitGradient`, decide by `get().usesJob(e.layerId)` instead of `painted > get().jobPixels`: 'sends a gradient to the job worker by the pixels it paints' fails (the mask's gradient runs on the UI thread: `execute`, not `job input`). Restore. (4) In `commitGradient`, drop the `try`/`catch` around `editPixels` (let it throw): 'refuses a gradient too large to paint before anything is sent' fails (the error escapes the store). Restore."

(10) No change to the e2e case "with a mask targeted the gradient paints the mask in black and white, as Gradient Mask" (lines 149-161): its layer covers the canvas, so the mask does not grow (no placement, the same 64 x 48); growth is pinned by Task 14a's `mask-grow.spec.ts`. The undo name stays "Gradient Mask" (command.rs:148; Gradient.swift:99), grown or not.

(11) Parked T9-6 stays with this task and the final review, and is still reachable here (a pixel gradient on a layer that grows, with a non-uniform covering mask): `displayed`'s Pixels arm swaps in the grown preview transform and the covering mask stretches over it. If it is fixed, the fix is the shape Task 14a used: the pixels preview carries, for a covering non-uniform mask on a growing layer, the mask as `followed` leaves it on the preview's grid (reduced, white where the layer grew, raster_edit.rs `followed`), and `displayed` substitutes it. Pinning the mask with `placement = Some(old layer transform)` instead is wrong for black-edged masks: past its place the coverage is the mask's background (black), where the commit's `followed` reveals (white, EditorSession+Brush.swift:171; BrushStroke.swift:877-887).

**Task 16 (the Shape tool)**

Its "shapes and fills" perf case (lines 25-81) fills a canvas-covering layer's pixels with Alt+Backspace: `editPixels` counts the canvas there, so it still goes to the worker. Its perf hunk (lines 17-23) takes the last three lines of Task 15's "gradient tool" case as context; (7) leaves them untouched and appends after them, so the Shape tool's case lands between the two. Corrected by the pre-flight audit (I-3): that hunk had no trailing context, so plain `git apply` anchored it at the end of the file, which after (7) is the mask case; task-16-brief.md's hunk now also takes the blank line and the mask case's first line as trailing context (header `@@ -1026,5 +1026,...`), and is amended there (see its "Controller amendments"). Its store.ts hunks (lines 621-690) take `commitGradient`'s doc comment, `gradientOptions`, `setGradientOptions`, `beginGradient` and `cycleToolMode` as context, none of which (6) changes. The Shape tool paints new layers, never a mask.

**For the controller (not a brief)**

- global-constraints.md OQ7 ("A mask is painted in its own grid in grey"): add "; a Fill or a Gradient grows it past its layer to the canvas, as Compositor 1.3.7 does (`applyPixelEdit` and `beginGradient` pass `growsMask: true`), its new area its background, the old grid joined with the 256-pixel tiles painted, placed on its own (Task 14a)". OQ9: "large" for Fill and the Gradient is `Engine::edit_pixels` (ruling C1), which counts a mask grown. OQ20's worker-budget gap now covers masks grown in the worker too; the app's `editPixels` question on the UI thread, which refuses before sending, closes it for Fill and the Gradient.
- task-17-brief.md line 95 (README, Fill): after "on a targeted mask its black or white" add ", the mask growing past its layer to the canvas as in Compositor 1.3.7". Line 100 (Gradient): after "switching tool or layer applies it first." add " On a targeted mask it reaches past the layer too." Line 199 (coverage): "Task 14 (OQ15), on Task 8's engine" becomes "Task 14 (OQ15), on Task 8's engine and Task 14a's mask growth"; line 200: "Task 15 on Tasks 8 and 9" becomes "Task 15 on Tasks 8, 9 and 14a".
- Task 17's docs (pre-flight audit M-10, controller ruling 2026-09-29): the rulings text Task 17 writes carries the OQ7 / OQ9 / OQ20 additions of the first bullet above (global-constraints.md is not edited before then), and discloses that `Mask::background` reads the full-size edge where the Mac reads its thumbnail's (the same up to 96 px, audit M-7).
- Follow-up (pre-flight audit I-6, controller ruling 2026-09-29: budgeted now, no redesign): keep `mask_revision` on a placement-only move (ops/transform.rs:17-18 sets the placement through `mask_mut`), so a nudge, Undo or Redo of a layer whose mask grew stops re-uploading the canvas-sized mask; this task's perf case measures the cost meanwhile.
- The Mac probes (mac_probes.rs, the 1.2.10 oracle): none fills or paints a mask, so none changes; a 1.3.7 probe of a mask fill past the layer would pin the tile rule on the Mac itself (not requested).
- Ledger: Task 5's minor "placement-only mask changes from a job are dropped (none today; Tasks 14/15 could)" is still unreached (a grown mask returns its buffer; a mask edit that does not grow keeps its placement exactly).
