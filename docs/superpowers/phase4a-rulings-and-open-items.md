# Phase 4a: rulings and open items

Phase 4a (selections) was executed from `docs/superpowers/plans/2026-09-27-phase4a-selections.md` by
subagent-driven development on branch `phase4a-selections` (plan fixes d979498, tasks 8f563e0..847ee16,
final fix wave 63f6c44..3c97387). This file keeps the decisions and open items after the SDD workspace is
deleted.

## What shipped (0.4.0)

- Mac 1.2.10 probes: 31 exports pinned (the 14 Phase 3.5b follow-ups, color-balance-preserve, the 16 effects
  probes); Soft Light is Pegtop's formula on the CPU and the GPU; Motion Blur is CIMotionBlur's Gaussian along
  the angle (sigma = distance / sqrt(12)), one kernel for the filter, the adjustment layer and the GPU, so it
  left the undrawn notice and the merge refusal.
- The selection: a fixed-point vector outline (i_overlay 9, 256 units a pixel) with anti-alias and feather,
  part of the document so undo covers it, never saved; exact-area coverage; the Mac's feather (sigma =
  feather / 2, edge repeated). Marquee (M), Lasso (L), Magic Wand (W, WandPixels.c ported); New / Add /
  Subtract with Shift / Alt; move and nudge the outline; Select All, Deselect, Inverse, Expand, Contract,
  Feather; a layer's pixels or a mask's BLACK areas as a selection (the Mac, not Photoshop); marching ants.
- Destructive adjustments, filters and Invert stay inside the selection; the Levels / Curves histogram is
  weighted by it; adjustment layers never read it; Delete clears the selected pixels (fills a targeted mask
  white); Add Mask with a selection hides the selection (the Mac); Crop starts at the selection's bounds.
- Performance (final fix wave): the engine keeps each document's selection clip (keyed on the document, the
  selection revision, the canvas size, the outline itself and its flags); the feather is a recursive (Deriche)
  Gaussian from sigma 2; history snapshots share the outline (Arc); the ants stroke one kept Path2D, culled to
  the view and paced. A 24 MP Levels drag tick under a feather-63 selection: 17.8 s -> 14.7 ms (native
  release).

## Rulings

Pre-flight (the audit found 2 Critical, 5 Important, 11 Minor in the plan):
- C1: every commit command put `-m` after `--` (git reads it as a path) - rewritten in the plan.
- C2: reverting a Step 5 bug with `git checkout` dropped the task's own uncommitted work - backup copy,
  move back, touch LastWriteTime instead.
- I3: ten `sampling-*` fixture pairs stay untracked (Phase 3.5d), not seven.
- I4-I7, M8-M13: weak or symmetric test fixtures strengthened (vertical flip, 128 thresholds, plain mask
  asserted, Contiguous island, non-square canvas, Levels exercised, concrete bounds, computed feather values,
  computed inset, independent whole-pixel-move reference).
- M16: the coordinate limit is checked by commands that take points; traced outlines are bounded by the
  transform limits (origin 1e6, size 300,000).
- M17: Tab cycles the tool's mode app-wide, as the Mac.

Execution:
- Task 2 perf: timed the build before the Motion Blur change against HEAD, alternating, in the release wasm:
  no regression (motion 2000 px 3.2-3.5 s old vs 2.6-3.4 s new; the eyedropper through it 2.3 s old vs
  1.7-2.3 s new). The perf-spatial eyedropper budget (2 s) trips on the old build too on this machine.
- Task 3: the whole-pixel-move test first compared the copy path with itself (M13 only partly met) - one fix
  round made it exact and independent.
- Task 12: Delete (and every selection clear) does nothing while a crop rectangle is active, as the Mac's
  canPaint requires.
- Task 13: the Levels e2e asserts the formula's value, not "changed".
- Final review: Critical F1 (the clip rebuilt at full size every preview tick, and an O(sigma) feather blur -
  a plan defect, never measured at size), Important F2-F6; all fixed in one wave and re-reviewed.

## Open items

- Resampling (Phase 3.5d): the Mac draws layers with a two-tap filter whose phase is rounded to eighths
  (weights 0, 1/16, 1/8, 1/4, 1/2, 3/4, 7/8, 15/16, 1) and an anti-aliased edge; the port is bilinear and
  draws turned edges hard. Probe results "Step probes"; ten `sampling-*` exports untracked in the fixtures.
- Performance left for Phase 4b's groundwork (UI thread): Levels histogram about 2.0 s and commit about
  2.4 s on a 24 MP layer even without a selection; the ants hitch 0.6-0.8 s a step at 1:1 on a 4M-point
  outline; the outline trace at 1:2 about 0.85 s for it; kept clips have no byte budget (one per open
  document).
- Feather accuracy wording: the recursive Gaussian is within 1 level of the true Gaussian but up to 2 levels
  from the test's 3-sigma-truncated formula at very large feathers (sigma 50-125) on a hole exactly the
  kernel's width; fix the doc and test wording.
- Deferred Minors recorded by the task reviews (can wait): untested guards and messages (NO_SELECTION text,
  tolerance / radius refusal, antialias kept by Expand / Contract / Inverse, loadSelection antialias, the
  no-op undo cases), the Invert "keeps transparency" half not ported, Add Mask not tested on a turned layer,
  beginAdjust keeps a selection draft, selection edits allowed while a crop is pending, Tab outside the
  selection tools, rail aria-labels, a duplicate polygon corner on double-click, the wand release path's
  off-canvas guard, no move cursor while dragging the outline, a throw in paintOverlay stopping the ants,
  the vertically doubled motion stencil, the undrawn dedup test, comment nits.
