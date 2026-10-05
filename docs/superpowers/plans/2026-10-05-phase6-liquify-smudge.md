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
- [ ] GPU source/result transport through the same guarded writeback.
- [ ] WebGL2 live preview without a document-sized upload on each pointer move;
  bounded CPU fallback and safe resource disposal.
- [ ] Mode controls, brush options, shortcuts and refusal messages.
- [ ] Native/WASM/unit/browser/packaged-runtime acceptance, 24/100 MP
  responsiveness and focused actual Mac 1.4.5 gesture/export returns.

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
