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
- [ ] CPU reference kernels: smudge carried colors and immutable-source
  Liquify offsets, bounded allocation and deterministic edge/rounding cases.
- [ ] Engine job/command integration, selection coverage, transformed-layer
  writeback and single-step history/cancellation.
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
require measured probes. The Mac shader's one-pixel axes have undefined neighbor
reads; Windows must handle them safely and record that boundary explicitly.

Do not alter Phase 5 assertions/budgets, claim hosted CI proves local clipboard
or Mac behavior, or include the user's unrelated sampling fixtures in commits.
