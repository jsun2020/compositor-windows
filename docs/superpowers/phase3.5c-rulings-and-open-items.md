# Phase 3.5c: rulings and open items

Phase 3.5c (draw the six layer effects as Compositor for Mac 1.2.10 does) was executed from
`docs/superpowers/plans/2026-09-25-phase3.5c-draw-layer-effects.md` by subagent-driven development
on branch `phase3.5c-effects` (e2ba37a..780d26c, then this file). This file keeps the decisions
and open items after the SDD workspace is deleted.

## What shipped (0.3.7)

- Typed layer effects: stroke (inside, outside), drop shadow, inner shadow, outer glow, inner glow
  and colour overlay, read and written as the Mac's Codable does (required members, whole numbers
  without `.0`), unknown keys kept and named in the notice.
- The Mac's Metal kernels on the CPU, streaming rows: bit-exact against a line-by-line
  transcription at every blur within 48 layer px; past that a blur runs on a halved copy whose
  halo is built from the edge-clamped full-size pixels, within 1 level at any inset (a 5164-case
  sweep, and a fixed-seed 180-case sweep in the suite).
- Each styled layer is drawn from one effects image in its own pixel grid, through a grown
  transform; the CPU (`draw_raster`) and the GPU (the same image as the layer's texture, no GLSL
  copy) both sample it, so screen and export match by construction. A partial render equals the
  whole render to the bit.
- An engine-level `EffectsCache`: matched by buffer identity and an equal draw, LRU at 8 images
  and 512 MB, an image over 512 MB never kept, dead entries pruned.
- Effects left the undrawn notice and the merge guard; merge, delete-with-bake, export, the
  histogram and the eyedroppers see them as drawn. `edited-rich-file` matches the Mac 1.2.10
  render within colour 1 and alpha 3.
- Image Size scales effects; Canvas Size and Crop keep them; reduced previews scale them.
- Revisions are issued by one engine-wide counter and never reused, so the GPU never keeps a
  texture an undo replaced; Image Size bumps the mask revision.
- 16 Mac probes for the user's 1.2.10 exports.

## Rulings

Planning (OQ = the planner's open questions, then the pre-flight audit):
- OQ1: the GPU draws the engine's effects image; no GLSL mirror.
- OQ2: only the Metal path is ported, at every size up to 200 M padded px; the Mac's CPU fallback
  above 80 MP is a recorded deviation.
- OQ5 + F-B1/F-B2: one engine-level cache (8 entries, 512 MB, LRU; over-cap images not kept); the
  wasm bridge owns the raster it hands out, for the upload only.
- OQ6 (re-ruled with measurements): the multi-second redraw of a large styled layer after a pixel
  edit or during an unlinked mask drag is accepted for 3.5c; a reduced display image built off the
  UI thread (the Mac previews at <= 1536 px on a worker) is a Phase 4 prerequisite.
- OQ7: `EFFECTS_SURFACE_LIMIT` = 200 M padded px, as the Mac. OQ8: a number is refused where the
  Mac's decoder expects a Bool.
- E-M4: Image Size scales effects (the Mac drops them); Canvas Size and Crop keep them (the Mac
  drops them); delete-with-bake bakes a clipping base's effects, matching the Mac's live view (its
  baker leaves them out). All recorded deviations.
- G-I3: halved effect blurs run one at a time, each reduced plane freed before the next.

Execution:
- Task 3: the halved blur first repeated the edge CELL past the image (Metal repeats the edge
  PIXEL): inner glow and inner shadow at tight insets were 21-116 levels off. Fixed in two rounds
  (edge-clamped halo, full-block edge averaging, a blurred halo ring in the enlargement).
- Task 4: an over-cap image must not evict the images already kept (test added before review).
- Task 7: every peak budget is this repo's own measurement plus slack (four Gaussians at level 1:
  8.36 B a padded pixel).
- Disk: the C: drive filled with this session's scratch build copies; every scratch crate now
  deletes its own target.
- Final review: I1 revision reuse (engine-wide counter), I2 Image Size mask revision, the flaky GPU
  lattice test (90 s timeout), skip layers outside the region before making their image, prune
  dead cache entries (counting the cache's own holders out), invalid effects stay undrawn through
  Image Size and previews, the bridge releases the raster after the upload. Worst-case wasm32
  memory about 2.66 GB.

## Open items

- The 16 effects probes and the 22 Phase 3.5b follow-up probes in `build-artifacts/mac-probes/`
  await the user's Mac 1.2.10 exports; until then the effects other than the drop shadow are
  compared with the Mac's code, not a Mac render. A Mac-authored project with effects would also
  confirm the written JSON.
- Phase 4 prerequisite (OQ6): reduced display effects images built off the UI thread before brush
  painting on styled layers.
- Rulings 14/OQ3 (Core Graphics mask rounding) and Metal fast-math ulp differences are settled by
  the probes.
- Deferred Minors are recorded in the final review.
