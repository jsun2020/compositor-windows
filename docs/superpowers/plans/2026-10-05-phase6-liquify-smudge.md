# Phase 6: Liquify and Smudge

The user authorized advancing from the accepted Phase 5 checkpoint on
2026-10-05, after updating README and automated Windows releases.

## Behavioral oracle

Use upstream Compositor **v1.4.5, 086f163**, not the older checked-out Mac files.
Relevant sources are `Compositor/Document/SmudgeLiquify.swift`,
`Compositor/Rendering/MetalWarp.swift`, `Compositor/Document/BlurTool.swift`
and `CompositorTests/MetalWarpTests.swift` at that tag.

- Smear has Liquify, Blur and Smudge modes; existing Blur behavior stays intact.
- Diameter is at least 2; hardness clamps to 0–0.98, strength to 0.01–1.
- Dab spacing is max(1, diameter * 0.005) for Smudge and
  max(1, diameter * 0.025) for Liquify. A first click starts the stroke without
  changing pixels; a sub-spacing movement does not advance the anchor.
- Smudge mixes carried color with the current pixel at weight * strength.
  It carries the unrounded newly mixed result, not the initial pickup forever.
- GPU Liquify advects a float offset field and samples the untouched original
  image through that field. Repeatedly resampling the edited RGBA pixels is not
  the Mac 1.4.5 GPU behavior and would progressively soften them.
- Each dab snapshots only its dependency region before writing. Coordinates
  and premultiplied RGBA are preserved; the radius uses the smoothstep falloff.
- Masks refuse Liquify/Smudge. One stroke produces one history edit. Preview
  cancellation and a changed source layer/transform must discard stale results.

## Delivery steps

- [x] README and Windows CI/release implementation prepared with immutable
  version checks, original test gates and ZIP/source-provenance verification.
- [x] Phase 6 scope re-read against the actual v1.4.5 sources.
- [x] CPU reference kernels: smudge carried colors and immutable-source
  Liquify offsets, bounded allocation and deterministic edge/rounding cases.
- [x] Bounded CPU engine job/command integration, selection coverage,
  transformed-layer writeback and single-step history/cancellation.
- [x] Independent tiled WebGL2 kernels, fractional carry / float offset
  snapshots, allocation checks and explicit disposal/context-loss handling.
- [x] GPU source/result transport through the same guarded writeback.
- [x] WebGL2 live preview without a document-sized upload on each pointer move;
  bounded CPU fallback and safe resource disposal.
- [x] Mode controls, brush options, shortcuts and refusal messages.
- [x] Native/WASM/unit/browser source checks, packaged WebView2 supplemental
  protocols and 24/100 MP responsiveness on the tested hardware.
- [x] Focused actual Mac 1.4.5 returns with committed edits, PNG comparisons,
  preserved styling/masks and Windows WASM save/reopen.
- [x] Windows operator operations and committed project/PNG returns received.
- [x] Last-tab residual-image fix: rebuilt native checks and development acceptance.

The first implementation increment is the independently tested CPU reference.
It is a foundation for the GPU and engine integration, not a shipped tool or
completion of Phase 6. Keep 0.8.0's release source separate from development;
bump the aligned package/workspace/Tauri versions when the next release is ready.

## Verification boundaries

Reference tests must distinguish retained initial color from the current
smudge pickup, and immutable-source offset advection from repeated RGBA
resampling. Include first-click and sub-spacing no-ops, clipping, alpha,
invalid inputs and thin-axis safety. Exact Metal/WASM equality is not inferred
from matching formulas: GPU float arithmetic and normalized-texture rounding
require measured probes. With a one-pixel dependency axis, the Mac shader skips
the dab at its negative-neighbor guard; Windows defines clamped bilinear reads
for thin planes and must record this boundary explicitly.

Do not alter Phase 5 assertions/budgets, claim hosted CI proves local clipboard
or Mac behavior, or include the user's unrelated sampling fixtures in commits.

## First reference checkpoint — 2026-10-05

`engine/src/warp.rs` owns the working plane, Smudge's fractional carried colors
and Liquify's untouched source/float offset field. Dabs freeze only their
offset dependency rectangle. First-click pickup and Mac mode-specific spacing
are implemented; invalid/excessive appends leave the stroke unchanged.

The reference refuses planes above 4 Mi pixels, diameters above 2000 and
appends above 4096 dabs or 16 Mi brush samples. Checked multiplication also
protects the 32-bit WASM target. These limits define a bounded oracle; they
are not the interactive 24/100 MP product limits.

Validation on the Windows development machine:

- `cargo test -p compositor-engine --lib --locked`: **12 passed**, zero failed.
  Includes analytic fractional advection, reads from one pre-dab snapshot,
  premultiplied alpha, fractional diameter falloff and the Mac bright-circle
  fixture's no-ghost-peaks/fading-tail assertions.
- `cargo check -p compositor-engine --target wasm32-unknown-unknown --locked`:
  passed. This proves compilation, not browser/Metal arithmetic equivalence.
- `rustfmt --edition 2021 --check engine/src/warp.rs` and `git diff --check`:
  passed.

At this first checkpoint the module was not connected to editing jobs or the tool rail. Existing
Blur, painting, clipboard and the accepted 0.8.0 package are unchanged. Next:
integrate one-stroke history/selection coverage and GPU preview ownership,
then expose Liquify/Smudge and measure native/Mac behavior.

## Document/job checkpoint — 2026-10-05

`WarpStroke` now runs directly or through the existing detached WASM edit-job
protocol. It processes the unmasked, full-opacity layer asset in document
coordinates, excluding other layers and effects. Only the final hard footprint
(diameter + 4) is written back, through canvas/selection coverage and the
layer's original pixel grid. Replacement includes alpha, so transparent dragged
pixels can clear the destination. Opacity, blend mode, effects and masks are
retained; editable text/shape metadata is removed only when pixel bytes change.

Actual scheduled dab centers define the final footprint. First-click pickup,
sub-spacing movement and an unchanged result preserve history/redo. A changed
stroke installs as one undo step. The existing layer stamp rejects results
after source pixels, transform, canvas, selection or mask changes. Existing
worker-preview cancellation leaves committed pixels and earlier history intact.

The shared brush grid now absorbs near-integer inverse-transform roundoff,
using the existing raster-edit tolerance. A 90-degree rotation previously
created an extra column and a white fringe in a following mask.

The CPU command checks both canvas and source against the 4 Mi pixel cap before
rendering the document plane, limits input to 4096 points and a stroke to 64 Mi
scheduled brush samples, and checks the output grid before allocating it.
`ops::warp::writeback` can accept a finished GPU plane, but its GPU transport,
interactive preview and tool controls are still pending. No 24/100 MP warp
responsiveness, native tool UI or Metal pixel equivalence is claimed here.

Validation for this checkpoint on the Windows development machine:

- `cargo test --workspace --locked`: **650 passed**, zero failed, the same
  **10 pre-existing ignored** tests. The new `warp_jobs` suite has 10 checks,
  alongside the 12 reference-kernel checks and unchanged original suites.
- `pnpm wasm` and `pnpm build`: passed, including the command's WASM build
  and all three TypeScript projects.
- `pnpm test`: **287 passed**, zero failed.
- The first targeted browser run had **5 passes and 1 navigation timeout** at
  `page.goto`, before engine readiness or any warp command. The concurrent
  workspace build was still compiling. Its log and error context were retained
  under `build-artifacts/phase6-warp-jobs-*`. The original 30-second test budget
  and zero-retry policy remain unchanged.
- Complete `pnpm exec playwright test --workers 1 --retries 0` after compilation:
  **205 passed**, zero failed. This includes all six new real-WASM worker cases:
  direct/worker byte equality for both modes, selected/whole writeback, one
  undo/redo step, pickup/sub-spacing no-op/redo and changed-layer refusal. The
  **29 existing PERF opt-in** cases remain separate; this functional run does
  not prove new warp performance. The earlier failed run remains recorded above.

## Canvas layout follow-up — 2026-10-05

On `3ac43cc`, GitHub's push run `37277409207` failed one original Shape tool
assertion: the rectangle started at Y=13 and had height 17, instead of Y=10 and
height 20. Its other 204 functional cases passed. The independent PR run
`37277429012` passed all 205, which does not make the intermittent issue fixed.
Both results and the failed run's error context were retained.

The added `canvas-layout` case defers ResizeObserver notifications while a
toolbar changes height, without changing original tests or their budgets. On
the old source it deterministically found viewport height 639 while the DOM
height was already 551. Client points computed in that interval could use
different canvas centers for the same drag.

Canvas sizing now runs in a layout effect when a document or tool changes,
before pointer input uses the committed toolbar geometry. The observer still
handles actual resizes; redundant unchanged sizes skip non-fit viewports and
fit viewports retain their previous refit behavior.

The controlled regression fails before the source fix and passes after it.
On the final follow-up source, all **10 targeted browser cases** passed (the
new layout case, three unchanged original shape cases and six warp-job cases),
and the three TypeScript checks/build plus **287 unit cases** passed again.
Rust/WASM kernel sources are unchanged by this follow-up. Fresh full GitHub
checks are required before merging; no CI rerun of the unchanged failed source
or automatic assertion retry was used.

## Next GPU integration contract

- Keep immutable source pixels and float Liquify offsets / fractional Smudge
  carry separate. Freeze each dab's dependency region before changing it; do
  not read and upload a document-sized image for every pointer movement.
- A preview must draw through the layer's existing masks, effects, blend mode,
  opacity and final selection coverage. It must not change engine pixel
  revisions or history on every dab. The current brush's translucent overlay
  is not a substitute for the warped image preview.
- Final readback must reach the worker's guarded replacement writeback using
  the original layer stamp. Display halvings stay in the worker, following F1.
  Closing/switching documents, cancellation and context loss release resources
  and discard stale results rather than applying them to the current layer.
- Check float-render-target support, texture dimensions and resource bounds
  before allocation. The existing renderer chunks large images at 2048 pixels;
  a 100 MP document must not assume it fits a single WebGL texture. Tile/source
  preparation and interactive dab costs need separate measured evidence.
- Unsupported GPU capability may use the explicitly bounded CPU path. It must
  report the existing refusal on larger canvases instead of blocking the UI
  with an unbounded fallback or silently reducing the edited image.

## Tiled GPU kernel checkpoint — 2026-10-05

`app/src/canvas/gpu-warp.ts` owns a dedicated WebGL2 context. It uploads an
immutable, premultiplied RGBA8 source once into a texture array of 1024-pixel
tiles. Work tiles are 256 pixels and allocated only along the stroke. This
allows a 10000 × 10000 source without assuming one texture can hold it.
Each Liquify dab snapshots the intersecting RG32F field into one dependency
rectangle before any tile writes. Source sampling and field interpolation are
manual, so floating-point linear-filter support is not required. Smudge uses
RGBA32F ping-pong carry, snapshots current RGBA8 tiles, retains fractional
painted values and rounds only the canvas. Row zero stays the document's top.

The session checks float framebuffer capability, texture-array depth, texture
dimensions, premultiplied inputs and an explicit 512 MiB allocation budget.
Point/work/capacity refusals occur before applying an append. Runtime GPU
errors or context loss invalidate the session. Explicit cancellation releases
textures, framebuffers, programs and the vertex array, and unbinds the current
program. `readTiles` returns only touched tiles for future guarded writeback.
It does not install pixels or claim to implement a styled canvas preview.

Eleven browser checks exercise actual GL draws against the existing WASM
reference command: exact analytic bytes, mixed alpha and fractional tips,
repeated turns through source/work boundaries, negative half centers and thin
axes, pickup/sub-spacing no-ops, resource refusals, cancellation and context
loss. The mixed-float cases permit at most one byte of channel error, separately
from exact analytic cases. Neither proves byte equality with actual Mac Metal.

Initial failures are retained under ignored `build-artifacts/phase6-gpu-warp-*`:
the first harness used the wrong Vite root; the first allocation run omitted
UNPACK_IMAGE_HEIGHT for source array tile uploads. Setting the full input
height fixes offset row uploads. The new analytic expectation also initially
assigned Smudge's fading trail to Liquify; the WASM/GPU comparison was already
exact and the immutable-source Liquify expectation was corrected. No original
test or budget was changed.

Independent opt-in `GPU_WARP_PERF=1` cases run hardware Edge kernels on 24/100
MP, keeping all original 29 `PERF` cases separate. The first cold 24 MP Liquify
append failed the new 100 ms submission/completion/frame-gap limits (136/143/
137 ms). Drivers defer native pipeline compilation until drawing, so the
session now primes every pass/format on disposable one-pixel targets during
preparation, before accepting a stroke. That failure remains recorded.
The next complete 13-case GPU run passed, including both hardware sizes on
Intel HD Graphics 520 / ANGLE D3D11. Preparation took 263–357 ms at 24 MP and
1039–1120 ms at 100 MP; maximum append submission 3.4–6.7 ms, GPU completion
9–11.5 ms and frame gap 16.9–19.2 ms. Sparse final readback was 256 KiB in
each mode/size, and allocations stayed below 512 MiB. These are raw-kernel
measurements, not worker preparation, effects/masks, complete UI or packaged
runtime acceptance. Full-source regression and fresh hosted CI remain required.

At the kernel checkpoint, still pending: raw source/result worker transport with LayerStamp, preview
through the actual transformed layer/masks/effects/selection, mode controls,
bounded fallback, packaged tool measurements and focused actual Mac returns.
The GPU module is not imported by the production editor yet. Published 0.8.0
and its release assets remain unchanged; Phase 6 is not complete.

## GPU worker transport checkpoint — 2026-10-05

`warpSource` prepares the unstyled document-space layer in the existing WASM
worker. It returns both that GPU upload plane and the exact original layer,
mask and selection buffers. The caller keeps the original JobInput/stamp;
worker heap reclamation cannot lose the snapshot. Cancellation retires the
source and all returned inputs through the existing bounded buffer pool.

`warpResult` transfers sparse RGBA tiles and bounded JSON rectangles to the
worker. It checks canvas size, premultiplication, byte counts, tile bounds,
overlaps and a 64 MiB readback limit before reconstruction. The final footprint
is recomputed from actual scheduled input dabs, using the reference's shared
spacing rule. The worker recomposes the original source once at completion,
overlays tiles, then calls the established replacement writeback and output
preparation. It never runs the CPU warp kernel on the GPU result. Selection,
transformed grids, alpha replacement, following masks, change witnesses and
display halvings retain their existing paths. Installation still checks the
original LayerStamp and produces one history entry; pickup/sub-spacing no-ops
preserve redo.

Six new native cases cover byte/record equality with the CPU command, stale
pixels/transform/canvas/selection/mask, malformed tiles, mask/pixel-free
refusals and a document above the CPU reference cap. Initial new-test failures
are retained in ignored transport logs: an unplaced mask is legitimately
followed even without growth; cache revisions differ across direct execution,
installation and undo; and Liquify does not change the initially chosen x=5
selection on the analytic stripe. The test now compares the complete persisted
record/all pixel and mask bytes, with the selection over the changed x=4 pixel.
No original test/assertion/budget was modified.

Thirteen new browser cases exercise source worker -> actual WebGL2 -> result
worker -> main-engine install/preview. The original input buffers really detach
and return; styled selected/whole results match the CPU command, final-result
preview/cancel preserves stored revisions/history, five stale-source mutations
refuse, and malformed requests leave the worker usable. A 6 MP document proves
this route works beyond the 4 Mi pixel reference cap: analytic Smudge bytes
128/64/32, sparse readback and one undo. It is a transport test, not a 24/100 MP
responsiveness measurement or a live pointer-preview test.

Local release-WASM and all three TypeScript checks/web build passed; native
workspace regression reported **656 passed, zero failed, 10 pre-existing
ignored cases**, and the unit suite **290 passed**. Thirty targeted browser
cases passed (11 kernels, six existing reference jobs, 13 transport cases).
Full functional regression and fresh hosted checks must pass before merging.

This checkpoint has no production pointer/controller imports or tool controls.
Live preview needs a separate path: this completion protocol reconstructs a
document plane and final layer once; running it on every pointer movement would
violate the live-preview allocation/latency contract. Actual transformed,
masked and styled live rendering, bounded fallback, packaged UI timing and
focused Mac returns remain required before Phase 6 can be accepted.

## Production controller and live preview checkpoint — 2026-10-05

R now exposes Liquify / Blur / Smudge mode controls. Blur stays the Windows
0.8.0 default; warp modes clamp size to at least 2, hardness to 0.98 and use
Strength instead of Opacity. Shift does not connect a warp to an unrelated
previous paint stroke. These changes are development source, not a new release.

Each gesture uses a dedicated worker, independent of the generic effects job
worker and its heap-reclamation policy. It pins the original JobInput, layer
and raw document-space source. The source uploads directly from a WASM view;
no extra JS document-sized source copy is returned. Ordered input positions
are coalesced while a preview is in flight. Pointer-up waits for preparation
and pending input. The original stamp is retained through final installation.

Interactive previews sample checked sparse GPU tiles over the immutable source.
The worker computes the final hard footprint and canvas/selection coverage on
an original or reduced layer grid, at most 1024 pixels on its longest side.
Large previews are intentionally reduced and are not claimed byte-identical
to the final full-resolution image. Small previews, including a rotated growing
layer with a nonuniform following mask, match final bytes. Existing engine
preview rendering preserves effects scaling, masks, opacity, clipping and blend
mode. The painted draft-line overlay is disabled for these warp gestures.

Completion overlays sparse tiles into the resident original source once and
uses the existing full-resolution replacement/writeback, witness/display and
stamped one-step history protocol. Readback is bounded before allocating its
JS arrays. GPU-unavailable fallback runs the bounded CPU reference only on
canvases and assets at most 4 Mi pixels; larger unsupported canvases refuse
before source-plane allocation. Mid-stroke GPU failures cancel, not restart.

Escape, Undo, tool/mode/options changes, document switches/close and pointer
capture loss terminate the worker and clear the preview. Late replies cannot
install. Source pixels, transforms, canvas, selection and masks remain protected
by the original LayerStamp, including the final staged installation callback.

Validation is recorded under ignored `build-artifacts/phase6-warp-live-*`.
The initial new native large-preview fixture incorrectly used the bounded CPU
kernel to generate a plane above 4 Mi pixels; it now supplies a synthetic tile.
The initial forced-GPU-unavailable test omitted its wrapper worker's startup
handshake and timed out; the corrected real worker passes. Typed test imports
also required Vite's browser ImportMeta types, declared in the new test files
without changing original tsconfig/CI gates. A first 6 MP preview check timed
out while native compilation ran concurrently; the independent isolated run
passed. That failure remains retained and is not declared fixed from one pass.

Five native session cases cover original snapshot isolation, selected exact
preview/completion, rotated grid/mask growth, bounded large preview, bad tiles
and bounded CPU fallback, plus shared identity-source bytes with mixed alpha.
Five client unit cases cover queued positions,
preparation/pointer-up, coalescing, cancellation, early failure and queue limits.
Production browser cases cover actual mouse-driven styled Liquify/Smudge,
preview/storage/history separation, undo/redo, five cancellation paths,
preparation cancellation and early pointer-up, stale-source refusal, real CPU
fallback and readback disposal. The 6 MP controller/worker path checks a preview
no larger than 4 MiB, full-resolution analytic commit and undo; a separate large
unsupported-GPU case checks the early refusal.

Fresh full-source regression and hosted CI are required before merge. Native
WebView2 packaged tool behavior, 24/100 MP complete-tool response, focused actual
Mac 1.4.5 returns and release readiness remain unaccepted. Existing published
0.8.0 assets must remain unchanged.

### Complete-tool hardware checkpoint

The aligned canvas-sized source now shares the immutable original raster
instead of reproducing every pixel through CPU composition. A native case
checks mixed premultiplied colors/alpha and equality with unstyled composition.
Other transforms retain their existing source-plane path. Live preview reuses
cached layer reductions on unchanged grids and samples only the brush coverage
rectangle. Full-resolution completion and original validation gates are unchanged.

The new opt-in `GPU_WARP_UI_PERF` suite exercises the production controller,
dedicated GPU worker, live previews, full commit and existing history policy.
It is independent of the original 29 performance cases and raw GPU kernel
measurements. First, 100 MP preparation exceeded 30 seconds; after sharing the
identity source, a live preview still exceeded 250 ms (390.7 ms). Both failures
remain in ignored logs. The cached preview optimization then passed **two
independent complete runs**, one worker, zero retries, with the same budgets:

| Run / size / mode | First preview ms | Input max ms | Preview max ms | Drag frame gap max ms | Commit ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 / 24 MP / Liquify | 2350 | 1.8 | 66.5 | 17.1 | 990 |
| 1 / 24 MP / Smudge | 1176 | 1.9 | 65.2 | 19.1 | 936 |
| 1 / 100 MP / Liquify | 3334 | 0.6 | 80.9 | 29.9 | 3182 |
| 1 / 100 MP / Smudge | 4574 | 0.5 | 66.6 | 17.1 | 2921 |
| 2 / 24 MP / Liquify | 820 | 0.9 | 31.7 | 20.1 | 610 |
| 2 / 24 MP / Smudge | 746 | 0.7 | 47.0 | 18.4 | 445 |
| 2 / 100 MP / Liquify | 2492 | 0.5 | 44.4 | 20.6 | 1954 |
| 2 / 100 MP / Smudge | 1953 | 1.6 | 33.4 | 17.1 | 1292 |

Hardware: Intel HD Graphics 520, Edge/ANGLE D3D11. Preview transfers were
1,500,000 / 1,562,500 bytes. Source fixture construction is excluded from these
gesture timings; first-preview timing includes snapshot/worker/GPU preparation.
These are aligned full-canvas fixtures, not transformed 100 MP or native
WebView2 acceptance. Logs: `phase6-warp-live-preview-cache-performance-{1,2}.log`.

The new benchmark initially expected undo for a 400 MB replacement. That
expectation contradicted the existing **256 MiB** retained-raster history limit,
also verified in upstream v1.4.5 `DocumentHistory.swift`. The new case now
explicitly checks the unchanged limit: 24 MP gets one undo; 100 MP drops the
unretainable entry, disables undo and leaves the committed result unchanged.
No original test or history budget was relaxed. This boundary is documented
in README and the manual instructions.

Latest release-WASM, three TypeScript checks/web build, 21 targeted native cases
and **45 targeted functional browser cases** passed. The earlier complete local
functional run was **245 passed**, with opt-ins separate, and units **295 passed**.
Fresh exact-source hosted regression is still required before merging.

`engine/examples/phase6_warp_probes.rs` generates three synthetic `.comp`
projects and seed PNGs, checks package round-trips and refuses an existing
output directory. [Manual instructions](../phase6-manual-checks.md) cover actual
Mac/Windows preview cancellation, commit, undo/redo, save/reopen, effects/masks,
selection and mask refusal. Returns must contain edited projects, exported PNGs
and gesture records. Phase 6 remains open until this real-runtime evidence is
received and checked; the published 0.8.0 release is unchanged.

### Displayed preview metadata follow-up

Native production WebView2 checks of both modes on a small painted layer
passed two independent runs on the unchanged `ed27ac1` portable. Earlier
pointer-capture and source-seeding failures remain retained. Those passes
did not cover a canvas-sized raster. A 24 MP native diagnostic produced a
GPU preview without losing pointer capture, but the displayed image stayed
unchanged and its original 30-second visual gate failed.

The controller installed the engine preview and incremented a render tick,
while leaving the store's layer dimensions and pixel revision at their
committed values. The renderer keys uploads and sizes from that store state;
a canvas-sized preview could therefore retain the existing texture. The
controller now refreshes displayed document metadata after installing a
preview and after cancellation. Stored pixels, layer stamps and history
remain unchanged during a preview.

A new 6 MP browser regression first confirms that the engine received the
reduced preview, then checks displayed metadata and actual canvas PNG changes.
On the old controller it failed with store width 3000 despite the engine's
reduced preview. After the fix it passes and checks that Escape restores the
original layer metadata, canvas PNG and history depth. All 16 focused tool
checks, 295 unit checks, three TypeScript projects and web build passed.
The unchanged 24/100 MP complete-tool hardware cases also passed two
independent zero-retry runs after this fix, retaining the original response
budgets and 256 MiB history policy. Their ignored logs are
`phase6-warp-preview-metadata-performance-{1,2}.log`.
Fresh complete hosted regression, rebuilt portable checks and actual Mac
returns are still required; earlier package passes are not evidence for this
changed controller.

### Runtime and initial Mac return checkpoint — 2026-10-06

Exact-source hosted regression and rebuilt portable checks now pass for
`a01c7ed`. Two independent supplemental production WebView2 runs passed
24/100 MP responsiveness and small-canvas cancellation/history checks.
The earlier experimental visual predicate failure remains retained; the
supplemental reply/render protocol does not alter original CI gates.

Actual Mac 1.4.5 returns were received and checked. All three projects
import and round-trip through the current Windows WASM without losing
styling or masks. However, all decoded saved layer pixels are unchanged
from their original probes; the Smudge export is JPG, with its PNG missing.
The return does not yet establish persistence of a committed warp edit.
Requested visibly edited saved copies, PNGs and explicit omitted gesture
outcomes. Windows actual OS mouse evidence also remains pending.

See [the measured runtime checkpoint](../research/phase6-runtime-checkpoint-2026-10-06.md)
for source hashes, comparisons, retained failures and remaining gates.
Phase 6 remains open; PR #8 is not merged and published 0.8.0 is unchanged.

### Corrected committed Mac return

The user identified that the initial copies had been saved after undo, then
redid, saved and recopied all three projects. Their saved warp rasters now
change at 1,897 / 3,053 / 3,361 pixels respectively. Liquify and Smudge Mac
640×480 PNGs match Windows WASM in every RGBA byte. Styled-mask retains its
mask/effects/opacity with a maximum one-level RGB rendering residual and
exact alpha. All three Windows WASM manifest/render save-reopens are exact.
The received copies and SHA256 receipts are independently preserved under
`build-artifacts/phase6-mac-return-20261006-recopied`.

The user's completion statement supplements the Mac operator record. It
does not prove Windows actual OS mouse gestures, which remain the next
human gate after another two window-capture failures. Phase 6 is still open.

### Windows returns and last-tab correction

Actual Windows operations and three committed project/PNG returns were
received. They preserve the styled mask and metadata and match current
release-WASM exports in every RGBA byte. The operator also reported a stale
image after closing the final tab. Both source rendering paths and the frozen
2328 production package reproduce this separate defect.

CanvasView now clears the picture and overlay when no document remains,
using renderer-specific clear operations without creating an `Untitled`
document. Two before-failing regressions now pass for WebGL and CPU; related
file/render checks (14), warp checks (16), unit checks (295) and all three
TypeScript projects/web build pass. New packaged verification and exact-head
CI remain required. The runtime checkpoint retains the old failures and
Windows received-file hashes.

### Final development acceptance — 2026-10-06

All delivery steps above are complete on the tested Windows hardware and
received Mac/Windows probes. Corrected source `1fbe0aa` passes both hosted
push/PR CI: 661 native, 295 unit and 248 functional cases, with 10 existing
native ignores and 33 opt-in skips unchanged. The rebuilt development
portable `0.8.0-20261006-1125` passes build/smoke, ZIP/hash/source/WASM receipts
and the final native empty-workspace protocol after read-only unlock preflight.
It clears both picture and overlay, creates no document automatically and
renders a subsequent explicit File > New. Native inputs are synthetic CDP;
the earlier received operator returns remain the human evidence.

Native result: `build-artifacts/phase6-native-last-close-20261006-115458-177`.
Earlier locked preflights, failing package/source checks and experimental
performance predicates remain retained; no assertion, budget or retry policy
changed. Earlier performance measurements retain their `a01c7ed` identity;
the final correction only changes empty-workspace clearing. Published 0.8.0
is unchanged and PR #8 remains open for review. This closes development
acceptance, with exact Metal/WebGL kernel equality outside the claim.
