# Phase 3 rulings and open items

This file preserves the decisions and open issues from Phase 3 (adjustments and filters) of the
Windows port, branch `phase3-adjustments`, recorded 2026-09-24. Phase 3's working notes lived in a
git-ignored SDD workspace (`.superpowers/sdd/2026-09-22-phase3-adjustments-and-filters/`) that is
about to be deleted; this file is their permanent record, covering the execution ledger
(`progress.md`), the consolidated deferred-items list, the final whole-branch review, the fix-wave
report, and the fix wave's scoped re-review.

---

## 1. Open items after Phase 3

Everything below is still open after the final fix wave (511ea1e..1913e88). Items the fix wave
closed - C1, I1-I6, M1-M9, M12, M13, and deferred 1.1, 1.2, 1.4, 1.5, 1.6, 2.1, 2.3, 2.6 and 2.7 -
are not listed; the scoped re-review (`final-rereview.md`) confirmed every one of them FIXED, with
mutation proofs. Deferred items 3.3 (a test-count reporting mismatch) and 3.4 (a leftover no-op
double-click) were resolved during the phase and are also left out.

### Should fix next

**The Mac-compatibility fixture for Hue/Saturation is still hand-written, not Mac-produced.**
- Where: `engine/tests/interop.rs` (`MAC_HSV_ARRAY_FORM`), backing the C1 fix in
  `engine/src/manifest.rs` (`adjustment_file` adapter).
- What is wrong: the fixture that proves the Mac's `[key, value, key, value, ...]` array encoding
  for `hsvSettings.adjustments` and `.bands` is hand-written from Swift's documented `Dictionary`
  encoding rules, because no Mac was available during Phase 3. It has never been checked against a
  project actually saved by the macOS app.
- Why it matters: the C1 fix was verified by direct inspection of the Swift source and by an
  end-to-end trace in the running app, and the fix wave's own re-review calls this "a confirmation
  step, not a blocker." But a hand-written "Mac JSON" fixture is exactly the kind of artifact that
  let the original C1 defect (every Mac HSV adjustment layer failing to open) ship undetected for
  the entire phase. This must be confirmed against, or replaced by, a manifest saved by the real
  macOS app before the byte-compatibility claim is fully trusted.

**N1 - `importImages` checks the panel guard once, before its async loop.**
- Where: `app/src/actions/files.ts:80` (guard), `:92-93` (`await bridge.readFile`, then
  `engine.importImage`).
- What is wrong: keyboard shortcuts that open a panel (Ctrl+L/M/U) are not covered by the `busy`
  flag that disables the Import menu during a multi-file import, so a panel can open mid-import;
  the loop's remaining iterations then import through `Engine::edit` under an open panel.
- Severity: Minor (final-rereview N1). Reopens the I4 invariant ("an undo step recorded while undo
  is inert") through a timing window; nothing is corrupted, since import never touches the
  previewed layer.

**N2 - the crop tool can be re-armed under an open panel, and Apply then discards the rectangle.**
- Where: `app/src/state/store.ts:236` (`setTool` unguarded, re-seeds a full-canvas rectangle),
  `app/src/panels/CropOptions.tsx:17`.
- What is wrong: after `beginAdjust` clears `cropRect`, the crop tool can be re-selected and a new
  rectangle dragged while a panel is open. Clicking CropOptions' Apply button runs `Crop` (refused
  with a banner) and then discards the rectangle.
- Severity: Minor, cosmetic (final-rereview N2). Enter/Escape are already correctly routed to the
  panel alone; only the Apply button is affected, with no history or document change.

**N3 - `set_preview`'s dedupe key omits the layer's stored revision (latent).**
- Where: `engine/src/engine.rs:163-165`, `engine/src/preview.rs:23` (`same_output`).
- What is wrong: the dedupe key is layer + settings + limit. Correctness depends on an unstated
  invariant - every path that mutates a stored layer clears the preview first - which holds today
  but is not enforced anywhere.
- Severity: Minor, latent only (final-rereview N3). A future mutation path that forgets to clear
  the preview (a brush, say) would serve stale pixels from the dedupe cache. Cheap fix: fold
  `pixels_revision`/`transform` into the key, or state the invariant explicitly on `set_preview`.

**M10 remainder - unknown keys inside an adjustment are not preserved.**
- Where: `engine/src/adjust/settings.rs` (`LayerAdjustment`); spec section 5.
- What is wrong: the fix wave updated the spec sentence so it correctly says `adjustment` is now
  parsed rather than preserved verbatim, and an unknown key inside it is silently dropped on
  re-save; an unknown `kind` rejects the whole project. Nothing the current Mac writes is affected,
  but this is narrower forward compatibility than Phase 2's opaque-`Value` handling.
- Severity: Minor (final-review M10). The `#[serde(flatten)] extra: Map` fix was deliberately NOT
  done in the fix wave because it interacts with `mac_number` formatting and the C1 array/object
  conversion; explicitly recorded for Phase 4.

**1.3 - grain GPU/CPU coefficient-drift blind spot.**
- Where: `gl-renderer.ts` grain shader vs. `engine/src` `grain.rs` (weight constant 2.4).
- What is wrong: the parity mutation table shows a weight change from 2.4 to 4.0 is caught
  (22/255, fails), but 2.4 to 2.6 (an 8 percent drift) passes, sitting under the 3/255 tolerance.
- Severity: Minor; not a present bug (the committed 2.4 is verified correct today). Proposed fix,
  not yet applied: raise the grain fixture's amount so an 8 percent coefficient drift exceeds the
  tolerance - the same technique used to fix the hue-residual issue, changing the fixture rather
  than the tolerance (the tolerance itself cannot be tightened; it exists for the f32 hash path).

**1.7 - the apply_tables single-colour/whole-raster agreement test tolerance may hide a rounding
bug.**
- Where: `engine/tests`, the agreement test between `color()` and `apply_tables` for table-based
  adjustments.
- What is wrong: the test tolerates a difference of one level, so a genuine off-by-one rounding
  disagreement between the two paths would pass silently. The tolerance came from the plan's own
  brief.
- Severity: Minor, test-quality gap only; a tighter bound (exact equality for the table kinds,
  which share their interpolation) was suggested but never ordered.

### Decide

**A.1 - the Filter menu order differs from the Mac.**
Windows reads Layer, Filter, Image; the Mac declares Image, Filter, Layer. Purely positional,
inherited from the plan's Task 16 step. User-visible but changes only menu position, not any
`clickMenu`-based spec's title. The final review explicitly kept this at "decide," unchanged.

**A.3 - the Exposure dialog's aria-label equals its own field's label (latent).**
`getByLabel("Exposure")` is ambiguous even with exact matching, because the dialog and the field
share an accessible name. The one place this mattered (Task 17's cancel test) was fixed by
locating the field by role instead; the underlying collision in the code is untouched and latent
for any future test.

**A.4 - an open adjustment panel covers part of the layers list.**
At 1280x720 the panel covers most of the layer rows, leaving a narrow sliver visible. The final
review reclassifies this as purely cosmetic now that I3 refuses layer selection while a panel is
open (so nothing is blocked functionally) - but whether to reposition the panel, make it
draggable, or leave it as is remains undecided. Any change must keep `position: fixed`.

**A.5 - an adjustment-layer chip's aria-label can substring-collide with panel names.**
Same class as A.3: `aria-label="<kind> adjustment"` on the layer chip can match a non-exact
`getByLabel` alongside the panel's own dialog and field. No current test exercises it. Decide
alongside A.3 whether to rename accessible names so nothing shares a substring, or keep the
role-locator convention as the standing rule.

**A.6 - e2e flakiness under parallel workers or machine load.**
Seen once in Task 18 (3 tests failed together under load, each passed alone, and a full re-run was
clean); did not reproduce during the final review's own run (65/65 clean, 3.6 min). Decide whether
to lower Playwright's worker count, raise a timeout, or simply keep recording it as a known flake.

**2.2 - a store test conflates two rejection reasons in one assertion.**
`app/tests/unit/adjust-store.test.ts` "test 5" covers two different rejection reasons in a single
check, weakening what a failure of that test actually tells you. Deferred to the final review's
triage, which left it unaddressed; no fix ordered.

**2.4 - Task 13 GPU/CPU review observations (Task 13 review's own M4, M5, M6 labels - distinct from
final-review.md's M-numbering).**
- M4: three reassociated formulas (the mix/clamp order between `programs.ts` and `compositor.rs`,
  grain's clamp happening at a different numeric scale on GPU vs. CPU, and different f32 rounding
  points for the grain-strength uniform) all measure under tolerance today but round differently
  in principle.
- M5: the LUT is quantized to 8 bits, which spends roughly half the 2/255 parity budget by
  construction - not a defect, but the reason that budget cannot simply be tightened to catch the
  grain-drift item above (1.3).
- M6 (Task 13's label): an identity Hue/Saturation adjustment round-trips through HSL on the GPU
  while the CPU short-circuits, so the two take different paths for a no-op; measures under
  tolerance today.
All three are observational; none is a live divergence, and no fix is recorded for any of them.

**2.5 - Task 9's `plan.rs:155` (an adjustment layer can never be a stack base) has no dedicated unit
test.**
Only covered indirectly today, through the hierarchy guard in `adjust_ops.rs`. Noted as a Minor
gap but never ordered for a fix; the indirect coverage was treated as adequate for now.

### Informational or latent

**M11 - the same user action produces different bytes on OK (informational).**
On an unchanged OK, the Mac always writes the kind's optional settings object
(`hsvSettings`/`exposureSettings`/`gradientMapSettings`/`grainSettings`) and records an edit; Windows
writes the object only when a field was touched and records nothing for an unchanged OK. Both
decode to the same adjustment. Rated informational only in the final review.

**3.1 - Viewport half-pixel document centring (Phase 2 item, carried forward).**
`Viewport.documentRect` can centre the document on a half device pixel; at certain viewport sizes
the GL pixel-centre rule and the `Math.round` crop in `readDocumentPixels` disagree, producing
spurious diffs in pixel-comparison specs. A known workaround already exists (pick a pixel-aligned
viewport, as `blend.spec.ts` and `zoom-render.spec.ts` do); ownership belongs to whichever phase
owns `Viewport.documentRect`.

**3.2 - Task 9's folder-mask test only covers a canvas-sized folder.**
Not a defect: `group_layers` currently sizes a new folder to the full canvas, which is exactly what
makes the existing folder-mask test sound. A future test using a smaller, content-sized folder
would need different transform arithmetic. Forward-looking note only, not scoped to any phase.

**A.2 - no UI-level (e2e) test of Cancel after a grown blur preview.**
Coverage gap only. Cancel restoring the original layer size after a Gaussian or Motion Blur
preview has grown it is covered at the engine level (`engine/tests/preview.rs`) but not end to end.

---

## 2. Rulings

Every ruling recorded in `progress.md`, in the order they occur, numbered and grouped by the task
they belong to. Wording is trimmed of pure ledger bookkeeping (commit hashes, dispatch notes) but
each ruling's reasoning and cost-if-wrong is kept.

### Pre-flight scan (before Task 1)

1. **FINDING 1** - `adjust::filters::apply_filter` and `ops::adjust::apply_filter` share a name; a
   local definition shadows the glob re-export, so it compiles, but T8's brief calls the kernel by
   its full path `adjust::filters::apply_filter` and adds a comment naming both. Cost if wrong: a
   confusing recursive-looking call site, caught by the type checker either way.
2. **FINDING 2** - T14's AdjustPanel routes only Levels and Curves and renders a placeholder div
   for other kinds; T15 adds the Hue/Saturation route with its own file and T16 adds the filter
   route with its own file, each task building on its own. Cost if wrong: Task 14 would not
   compile, blocking the loop for a round.
3. **FINDING 3** - T14 also modifies `app/src/state/store.ts` (adds `autoLevels`), and T12 must
   export `visibleIds` into store.ts's imports for `canAdjust`. Cost if wrong: one missing import,
   caught by tsc.
4. **FINDING 4** - filter kinds are spelled as the serde variant with no space ("GaussianBlur",
   "MotionBlur", "AddNoise", "LensCorrection"); the human-readable title comes from
   `FILTER_TITLES`. T12's test literal "Gaussian Blur" is corrected to "GaussianBlur". Cost if
   wrong: the panel opens on the wrong kind, caught by the first e2e.
5. **FINDING 5** - T5's grain tests: the upper bound `spread <= 2*(0.35*255)+2` ignores the 1.6
   lattice scaling (the real bound is about 287), and the roughness comparison compares two
   different mixes. Replaced with `spread(&mid) > 20 && spread(&mid) > spread(&black)` and a
   smoothness check at equal roughness 0. Cost if wrong: a flaky or vacuous grain test.
6. **FINDING 6** - assertion messages go in `expect(value, "message")`, never as a second argument
   to `toBe`/`toEqual`/`toBeGreaterThan` (Playwright and vitest both take the message on `expect`).
   Cost if wrong: a type error at build time.
7. **FINDING 7** - T13's e2e copies the working setup from `app/tests/e2e/zoom-render.spec.ts`:
   `page.setViewportSize({ width: 1280, height: 720 })` before `goto`, and the CPU side composites
   at the rounded device size derived from `documentRect`, not the document's own size. Cost if
   wrong: a half-pixel rasterization tie makes every adjustment comparison fail for reasons
   unrelated to adjustments.
8. Adjustment panels are floating (`position: fixed`) rather than a tool-options row, precisely so
   they do not change the canvas height mid-test; the pixel-comparing specs depend on that. Cost
   if wrong: every parity spec would need re-tuning whenever a panel opens.

### Task 1

9. Task 1 needed three serialization details the brief's code omitted, each forced by the brief's
   own byte-exact test literal and by Swift's `JSONEncoder(.sortedKeys)`: a `mac_number`
   `serialize_with` that writes whole f64 values as `0` not `0.0`; a hand-written `Ord` on
   `ColorRange` sorting by JSON name rather than declaration order; and alphabetical field
   declaration in `RangeAdjustment` and `HueBand`. Accepted: without them a Mac-written project
   re-saved on Windows would differ, and Task 17's interop test compares `serde_json::Value`s where
   `Number(u64 120) != Number(f64 120.0)`. Cost if wrong: numbers written as `0` instead of `0.0`,
   which every JSON reader parses identically.
10. Task 1's brief wrongly claimed no fixture carried an adjustment; a fixture manifest held an
    invented placeholder blob that never matched the macOS schema. Replaced with a valid
    non-identity `LayerAdjustment`, and `manifest.rs`'s unknown-later-phase-fields test switched to
    the still-untyped `shape` field to keep proving what it was written to prove. Accepted as a
    correction, not a weakening. Cost if wrong: an interop fixture that tests the schema we
    actually ship.

### Task 4

11. Task 4 changed the brief's `let mut lightness_amount = 0.0;` to `let lightness_amount;`
    because both branches always assign it and the initializer raised `unused_assignments`.
    Accepted: same behaviour, and the zero-warning constraint is binding. Cost if wrong: none.
12. Task 4's colorize branch is corrected to match `HueSaturation.swift` exactly: read
    `settings.adjustment(settings.range)` rather than hard-coding Master, and drop the
    `if hue < 0 { hue += 360 }` wrap, since Swift applies only `truncatingRemainder(dividingBy:
    360)`. Reachable on the Mac by picking a colour range and ticking Colorize, and the same code
    becomes GLSL in Task 13, so the divergence would be baked into both renderers. Verified by hand
    that Rust's `hsl_to_rgb` with a negative hue already matches Swift's `toRGB`, so dropping the
    wrap is sufficient for parity. Cost if wrong: a colorized layer with a negative hue or a
    non-Master range renders differently from the Mac.

### Task 6

13. Task 6's brief had a real defect: `add_noise`'s per-channel key wrote
    `c as u32 * 0x9e37_79b9`, which overflows u32 for the blue channel and panics in a debug
    build, where the C oracle relies on silent unsigned wraparound. Changed to `wrapping_mul`,
    preserving the C semantics exactly. Accepted. Cost if wrong: none; the alternative was a
    guaranteed panic on every colour-noise call.

### Task 8

14. Task 8's blur test asserted the layer grows equally on left and right, but its fixture is
    horizontally asymmetric (opaque block flush against the left edge, ten transparent columns
    before the right edge), so a 9px blur reach clears the left buffer and falls short of the
    right, and the trim correctly pulls the right edge inward. Replaced with assertions that are
    true and sharper: vertical symmetry (where content spans flush to both edges), a positive left
    growth, and an explicitly negative right growth proving the trim cuts back where nothing
    reached. Cost if wrong: the test would pin trim behaviour less tightly; the growth maths itself
    is unchanged and still covered.
15. Task 8's brief wrote `layer.is_group()`; `Layer::is_group` is a bool field, not a method.
    Corrected. Cost if wrong: it would not compile.
16. Task 8's brief left an unused `before` binding in the mask-carry test, which warns; the
    zero-warning constraint is binding, so it is removed. Cost if wrong: none.
17. Task 8's positioned-mask finding is rejected and the code stands. A blur does not move the
    layer's content: `placed_like` grows the grid around pixels that stay at the same document
    points, so a mask anchored by its own placement transform must keep that placement unchanged.
    Calling `Mask::follow` there, as the finding proposed, would scale the placement by the grid
    ratio and visibly move a mask that should not move. macOS agrees: `Filters.swift`'s
    `commitFilter` carries only the covering case and leaves a positioned mask untouched. Cost if
    wrong: a positioned mask would desync after a blur; guarded now by a new regression test.

### Task 9

18. The plan's folder-scoping claim is wrong and the test that demanded it is corrected, not the
    compositor. macOS applies an adjustment to the whole context, limited only by the adjustment's
    own mask, its clipping source, and the enclosing folders' masks - nothing isolates a folder's
    pixels from an adjustment inside it, and the compositor already reproduces that exactly. So:
    revert `draw_scoped_adjustment` and `is_descendant_of`, and correct the test to pin the real
    mechanism (an adjustment inside an unmasked folder reaches a layer outside it; a folder mask is
    what limits it). Cost if wrong: an adjustment inside a folder would reach further than on the
    Mac; caught by the Task 13 GPU/CPU parity suite and by opening the same file in both apps.
19. The reverted machinery also carried an approximation the implementer could not verify
    (patching RGB but not alpha for semi-transparent folder content over an outside backdrop).
    Unverified approximations do not belong in the compositor, a second reason to remove it rather
    than test around it. Cost if wrong: none; the code it replaces is the oracle's own behaviour.
20. Commit the Task 9 fix personally rather than resume the implementer to redo it. The content was
    already correct and test-verified; re-dispatching only to re-author a commit message spends a
    seat for nothing. Cost if wrong: none, the diff is under review either way.

### Task 10

21. The `set_preview` assertion's intent is "set_preview records nothing," so it becomes a
    before/after comparison of `is_modified`, `can_undo` and `can_redo` captured immediately before
    `set_preview`, not an absolute - because `seeded()` imports into an existing document, so both
    flags are already true before the preview exists. Cost if wrong: a preview that quietly pushed
    history would still be caught by the existing pixel-read assertion and by the undo/redo/preview
    test.
22. The implementer's undo/redo short-circuit is correct and stays, forced by the brief's own test:
    an undo with a preview active must leave the canvas gray, but the only remaining history step
    is the import, so an undo that both cleared the preview and popped history would delete the
    layer. Undo and redo are therefore inert while a preview is up, matching the app-level
    constraint that they are inert while a panel is open; `execute` still clears the preview and
    executes. Cost if wrong: a stray undo during a preview would cancel the preview and silently
    undo an unrelated edit, two effects from one keypress.
23. Revert does NOT short-circuit - it clears the preview and then performs the revert. A no-op
    revert is a silent failure of an explicit user request to discard changes, and unlike
    undo/redo nothing pins it, so it gets its own test. Cost if wrong: revert during a preview
    reloads the file when the user expected only the preview dropped; recoverable and loudly
    visible.
24. The reviewer's Minor is misclassified and goes into the same fix round rather than the ledger:
    a blur-preview test previews a 300x300 raster against a 2048 limit, so `reduced()` never enters
    its loop and the halving path (and the `scaled(factor)` call from an earlier ruling) has zero
    coverage - a test that asserts nothing meaningful is a defect by the review rubric, so this is
    Important, not Minor. Cost if wrong: one extra fix round on a task that already passed.
25. Fix the blur-preview coverage gap by making the fixture cross the limit cheaply - a 2100x100
    raster is under a megabyte but its longest side exceeds 2048, so exactly one halving runs and
    factor is 0.5 - rather than allocating a multi-thousand-square raster that would slow the
    suite. Cost if wrong: none, the assertion either exercises the loop or it does not.
26. The histogram does NOT read the preview. The plan contradicted itself (the Global Constraint
    listed `histogram` among the preview's readers, while the histogram's own doc comment said
    "never the preview, or the graph would chase itself"); the Task 10 implementer and reviewer
    both resolved it correctly without flagging the conflict. Corrected the constraint so later
    panel tasks cannot regenerate it. Cost if wrong: a panel histogram would redraw from its own
    preview as the user drags a slider and chase itself; highly visible.
27. Drop the blur test's aspect-ratio assertion. It is algebraically vacuous: `grown()` pads a
    fixed pixel margin on every side, so the per-axis growth ratios are equal only when width
    equals height, which the old square fixture always was - a sixth assertion in this plan that
    could not fail. Cost if wrong: none, it tested nothing.
28. Replace the dropped assertion with "centre kept": the blur growth constraint says the
    transform is enlarged in place with the size scaled by the grid ratio and the centre kept, a
    claim that is shape-independent and meaningful on the oblong fixture, and nothing else would
    catch a corner-anchored grow. Cost if wrong: a blur preview would visibly drift the layer,
    which a manual panel check would catch.

### Task 12

29. Run a second pre-flight over Tasks 13-18 in parallel with Task 12, targeted at the blind spot
    the first scan had: every plan defect so far was a task contradicting the Global Constraints or
    the Mac source behind them, which file-sharing and per-task self-consistency checks do not
    catch. Three passes: constraint consistency, falsifiability of every assertion, and
    satisfiability, with Task 13 weighted heaviest because a CPU/GPU maths divergence would show as
    wrong colours on screen while every Rust test stayed green. Cost if wrong: one reviewer seat
    spent on an audit that finds nothing, against five defects that each cost a blocked task and a
    fix round.
30. Both findings from the Tasks 13-18 pre-flight audit are fixed in the plan before dispatch.
31. Approve the implementer's deviation on `closeDocument`/`openDocument`/`setActive`: it targets
    `activeId` rather than the literal `id` parameter, and guards `closeDocument` with
    `id === activeId`. Verified in `store.ts` that `AdjustEdit` carries no document id of its own,
    exactly like `TransformEdit`, so a live panel always belongs to whatever is `activeId` when its
    actions run; using the literal `id` would clear the preview on the wrong document. Cost if
    wrong: a stale engine-side preview on a background tab, visible immediately and fixable in one
    line.
32. Accept the structure-dirty cost for now, with a measurement gate at Task 14.
    `applyAdjustPreview` discards `setPreview`'s Dirty flag and unconditionally calls
    `refresh(activeId)`, a full `engine.state()` round trip on every `updateAdjust`. This is the
    store-wide pattern and the brief specified it, so it is not a Task 12 defect, but it differs in
    kind from discrete actions because it runs per-tick during a drag. Task 14, the first task with
    real sliders, must report drag responsiveness on a realistic document, with a ruling then. Cost
    if wrong: laggy sliders in the headline feature of the phase, caught by measurement before any
    user sees it.
33. Fix Finding 2 (the `beginTransform` guard gap) now, even though it is pre-existing code outside
    the brief's named scope: `beginTransform` calls `engine.execute(DuplicateLayer)` directly,
    bypassing the guard, and the oracle gates this the same way (`EditorSession.canTransform`
    starts with `guard canEditLayers`). Unreachable today only because no panel UI exists yet -
    Tasks 14-17 wire exactly that. Cost if wrong: a transform refused while a panel is open when it
    should have been allowed; loud and trivially reversible.
34. Fold Minor 5 (`isAdjustIdentity`'s `JSON.stringify` is key-order sensitive) into the Critical
    fix, since that comparison is being rewritten anyway and a key-order-sensitive compare would
    reintroduce the same bug by another route. Cost if wrong: none, it is strictly more robust.
35. Defer Minors 4 (three-way duplicated clear-panel logic) and 6 (a store test conflates two
    rejection reasons) to the final review's triage. Neither affects behaviour and neither is in
    code the fix round touches.

### Task 13

36. The 3 e2e failures found in Task 13's first pass are NOT this task's: a document with no
    adjustment layer shows a nonzero worst diff at one viewport size and zero at an adjacent one,
    the known Phase 2 deferred half-pixel document-centring item. Use a pixel-aligned viewport for
    this spec, the established precedent in `blend.spec.ts` and `zoom-render.spec.ts`. Do NOT fix
    `documentRect` inside Task 13, since it would change the viewport three other pinned specs
    depend on, late in the phase, for a defect already on the deferred list. Cost if wrong: the
    half-pixel centring ships another phase; a known item with a known workaround and no new risk.
37. The half-degree hue residual is fixed in the fixture, not the tolerance. The Global Constraint
    already says parity fixtures use hues that stay clear of half-degree boundaries, so the
    constraint anticipated this and the fixture violated it; the implementer confirmed it is not a
    translation bug by reproducing GL's output through Rust's exact formula. Cost if wrong: none,
    this is what the constraint was written for.
38. Both brief defects the Task 13 implementer found are real and its fixes stand. Defect 1 is the
    team lead's own: a `rem()` correction was undone on the very next line by re-adding the
    `if (hsl.x < 0.0) hsl.x += 360.0;` wrap, which would have defeated the negative-hue parity case
    added in the same edit. Defect 2, a zustand snapshot read before `beginAdjust` ran, is a
    straightforward stale-read bug.
39. C1 and I1 (Task 13 review findings, not the final-review labels of the same name) are both the
    team lead's own, from the fix made one round earlier for the same bug class. C1: a
    `?? hsv?.adjustments?.Master` fallback has no counterpart in either oracle (both are zero), and
    `is_valid` never requires `range` to be a key, so the bad shape passes validation - measured
    26/255 on shipped code. I1: the negative-hue test used -300, and -300/60 is exactly -5.0, so
    trunc and floor agree and the test could not distinguish them. Fixed both in the plan and
    ordered them in code, plus a -30 case and an absent-range case, each requiring proof of bite.
40. Fold Minors M1 (`ADJUST_KIND` dead while `adjustPass` re-derives kind codes inline) and M3
    (mirror `GrainSettings::normalized` on the GPU) into this fix round, since both sit in files
    already being edited and M3 removes the exact CPU-normalises/GPU-does-not asymmetry that
    produced C1. Defer M2, M4, M5, M6 to the final review. Cost if wrong: two tiny edits reviewed
    in a round that was happening anyway.
41. Defer the grain-tolerance blind spot (M2/1.3) to the final review rather than adding a seventh
    fix item now. The committed 2.4 is verified correct against `grain.rs`, so this guards future
    drift rather than fixing a present bug, and the 3/255 tolerance cannot be tightened because it
    exists for the f32 hash path's rounding. Proposed fix for later: change the fixture, not the
    tolerance, as with the hue residual. Cost if wrong: a future GPU grain-coefficient drift ships
    unnoticed; bounded, since the coefficient only changes if someone edits that line.
42. Finish the grain-normalization clamp fix (Item 6) to completion despite being genuinely
    unreachable today (validation already rejects non-finite input on every real path), because a
    half-applied defensive fix is worse than none - it reads as done and preserves the exact hazard
    the fix was meant to retire. Cost if wrong: three lines of dead defensive code; the alternative
    is a latent divergence that looks handled.

### Task 14

43. All four Task 14 deviations are approved: two are the team lead's own recurring defects
    (message-as-second-argument, and a `canUndo === false` assertion on a fixture that imports into
    an existing document); one is the eleventh vacuous assertion (a uniform-noise fixture that
    clears the auto-levels clip threshold regardless of implementation, fixed with a narrow-range
    fixture); one matches the pre-flight ruling that Task 14 would not compile without files Tasks
    15-16 create.
44. The Task 14 slider-latency performance problem is real in release, not a debug artefact: one
    tick costs about 65ms against a 16ms budget (4x over) and about 150ms including paint (9x
    over), visibly stuttering. The diagnosis holds in both builds: `engine.state()` is a small
    fraction of the cost either way, and `setPreview` is 95-97 percent of it either way.
45. The root cause is the Task 10 preview limit, not the store. Colour adjustments only halve
    above 4096px on the longest side, so an 800x600 layer gets no reduction and recomputes all
    480,000 pixels every tick. The 4096 constant was taken from the Mac without accounting for the
    difference in cost model (GPU-backed and lazy there, CPU-bound in wasm here). Cost if wrong:
    sliders stutter in the headline feature of the phase.
46. Fix the preview-cap problem in the final wave, not now, with the fix already decided so the
    wave has a concrete instruction: preferably lower the colour-adjustment preview cap (1024 gives
    one halving and about 4x, 512 gives two halvings and about 16x); the architecturally better but
    larger answer (route destructive previews through the GPU adjust pass) is too big to start at
    task 14 of 18. Cost if wrong: a stutter ships that a later phase fixes; recorded with numbers
    either way.
47. Delegate the plan-wide sweep for `canUndo` defects to a dedicated agent (p3-sweep-undo) rather
    than doing it inline, since an incomplete sweep is precisely the team lead's recurring failure
    and a fresh agent given one question to apply uniformly is less likely to tire of it mid-context.
    Cost if wrong: a mechanical edit to a plan file, reviewed before any of it is implemented.
48. Fold all three Task 14 review Minors into the fix round: the Enter handler firing
    `commitAdjust` from a focused `<select>` is a real misbehaviour in the headline UI; the
    untested preview-off claim is a test asserting nothing where its own comment says otherwise;
    the `nearestPoint` tie-break needs one line of documentation. Together under ten lines, all in
    files already being edited. Cost if wrong: trivial rework in a round that was happening anyway.
49. Defer the one non-blocking Curves observation to the final review: on a refused insert at the
    32-point cap, `onPointerDown` still sets dragging and captures the pointer, so a drag right
    after a refused click moves the previously selected point instead of doing nothing. The
    re-review confirmed this is pre-existing and not a regression, replacing a crash with a mild
    surprise at an edge case users rarely reach, and Task 14 had already had a fix round. Cost if
    wrong: a surprising drag at the point cap; recorded for triage.

### Task 15

50. The team lead's own verification stands in for a scoped re-review of the Task 15 fix round,
    since the implementer's agent went unreachable mid-round and the lead finished it from the
    working tree, being independent of the implementer, reading the full diff line by line, and
    running mutation tests on both fixes - stronger evidence than a read-only re-review would
    produce. Cost if wrong: the final whole-branch review still covers this diff.

### Task 16

51. Both Task 16 deviations approved: the `AdjustPanel.tsx` routing was directed by the team lead
    directly; the `NumberField` fix is a genuine brief defect and a second instance of the
    locator-ambiguity class (a slider and a number input inside one `<label>` shared an accessible
    name, and `aria-hidden` does not drop a focusable element from Chromium's accessibility tree).
    The reviewer is asked to confirm the number input keeps its accessible name and is a full
    keyboard equivalent for the now-unfocusable slider.
52. The implementer's "latent" Exposure aria-label flag was actually live for the next task: the
    Exposure panel's dialog aria-label equals its own field's label, and Task 17's cancel test
    fills `getByLabel("Exposure")` with that panel open. Fixed in the plan with a role-based
    locator (a dialog can never have the spinbutton role). Third instance of the class lesson: for
    form fields, locate by role plus name, not by label text alone.
53. All three Task 16 review Minors deferred, appended to the deferred-items list as A.1-A.3: the
    menu order differing from the Mac (positional only, decide at final review); no end-to-end test
    of Cancel after a grown blur preview (engine-covered); the Exposure dialog/field accessible-name
    collision (latent in code, fixed in Task 17's own test by a role locator).

### Task 17

54. Delete the dead double-click line in Task 17's cancel test rather than working around it: it
    timed out because the auto-opened Exposure panel floats over the layers list and intercepts the
    click at the row's centre. No CSS change, no `force: true` - `force` would pass by hiding
    exactly the interception a user would hit, and moving the panel changes shipped layout to
    accommodate a test line that should not exist. The earlier deferral of this line as harmless
    was the error: dead test code should be deleted, not deferred, because it can start failing
    once the surrounding UI exists.
55. Fix the second Task 17 test defect (an assertion reading `exposureSettings.exposure` on a
    never-committed Exposure layer) in the assertion, not the engine: reading the absent key as
    identity is correct, since eagerly storing a default would make Windows serialize a key the Mac
    never writes for a fresh layer - the exact drift the byte-compatibility constraint exists to
    prevent. Cost if wrong: none for behaviour; the accessor already treats absent as identity.
56. Split Task 18: as written it would build the 0.3.0 portable zip before the final whole-branch
    review and its fix wave, which will change source (at minimum the colour-adjustment preview
    cap, the phase's headline performance fix), leaving the user a stale zip - and the project's
    own guidelines are explicit that source-committed is not deployed. Task 18 now does the docs,
    spec notes, and version bump now, with `pnpm build:portable` moved to after the final fix wave.
    Cost if wrong: none; building once at the end is strictly cheaper than building twice.
57. Verify the Task 17 rename fix-round diff personally instead of a scoped re-review: a single
    test, both mutation directions evidenced, read line by line. Cost if wrong: the final
    whole-branch review covers it.

### Final review and fix wave

58. When the final review's first attempt (on the most capable model) failed immediately for lack
    of credits, re-dispatch the identical prompt on the next most capable model rather than stop to
    ask the user to buy credits. Cost if wrong: a marginally less capable final reviewer; the fix
    wave and scoped re-review still follow.
59. Rulings for the one fix wave (a single dispatch, grouped commits), covering every open item at
    once: convert the Hue/Saturation maps at the manifest boundary only for C1 (array form on disk,
    object form on the bridge, with a hand-written-and-labelled fixture since no Mac was available,
    plus deferred 1.4 and the M10 spec-sentence update); do a single "panel owns the document" sweep
    for I3, I4, I5 and M8 (refuse selection, import, sheets, crop and the relevant menu items while
    a panel is open; fold deferred 2.1's three copies into one helper); align identity and inputs
    for I1, I2 and M13 (destructive identity ported from the engine's own rule; a random destructive
    Grain seed like Add Noise; clamped panel inputs; a kind-aware Reset); fix performance via 1.1
    (lower the colour-adjustment preview cap) and I6 (1x1 sampling for the adjustment-layer
    eyedropper, a histogram computed once per panel open); harden robustness via M9 (validate on
    the preview/plan/LUT paths, falling back to the stored adjustment); fix test honesty for M1-M5,
    M7, and deferred 1.2, 1.5, 1.6 and 2.6, plus the drag-after-refused-insert behaviour in 2.7;
    record M12's grain coordinate deviation in the README. Explicitly NOT in this wave: A.1 (menu
    order), A.4 (panel placement, cosmetic once I3 lands), A.6 (flakiness, did not reproduce), M10's
    `serde(flatten)` (interacts with mac_number and ordering, Phase 4), M11 (informational), 1.3
    (grain drift fixture), 1.7 (tolerance), 2.2, 2.4, 2.5, A.2, A.3, A.5.
60. For deferred 1.1 (the preview-cap performance fix), adopt option C: preview at a small cap
    (512) during an active drag, then request the full 4096-quality preview once input settles
    (about 150ms debounce), for colour adjustments only - filters and grain/noise are unchanged.
    This matches the spec's "sliders feel immediate, full result follows." Specified the race rules
    to test with fake timers (a new tick cancels the pending settle; the settle reads the edit's
    live state at fire time; commit/cancel/preview-off/document-switch all clear the timer; opening
    a panel shows settled quality) and required e2e to wait for the settled preview explicitly
    rather than by timing. Cost if wrong: about 40 lines plus tests in a separately reviewable
    commit; the alternatives were a stutter or a permanently blurred preview.
61. Approve the fix wave's deviation of a new `PreviewRequest::DragAdjustment` variant instead of
    an optional `limit` field: a `#[serde(default)]` field still has to be written into every
    existing Rust struct literal (six test literals would change), while a variant leaves every
    existing caller, test and JSON request byte-identical for the same behaviour. Cost if wrong:
    none.
62. Defer the fix wave's own new findings N1, N2 and N3 to Phase 4 (all Minor, none corrupts data;
    N3 is a documentation-of-invariant gap rather than a live bug). Recorded in this open-items
    document.

---

## 3. Process lessons

- **Fixture and assertion falsifiability was the phase's single biggest recurring failure mode.**
  At least twelve assertions across the plan turned out to be unable to fail (a symmetric fixture
  that made "correct" indistinguishable from "no-op," a viewport pinned by a square that hid a
  broken aspect-ratio check, a double-click test that never actually double-clicked). The standing
  rule that emerged - every new assertion must name a production change that would break it, and
  any assertion added to close a review finding must be proven to bite by temporarily introducing
  the bug - is what eventually caught most of them; it should be applied from the first task of a
  phase, not discovered partway through one.
- **A correction made in one task does not automatically reach a task that mirrors the same
  behaviour.** Task 4's colorize ruling was recorded and applied, but Task 13's GLSL port of the
  same logic still carried the old, wrong mental model because it was written from the same
  original understanding. The lesson recorded in the ledger: when a ruling corrects a behaviour,
  grep the whole plan for every other place that behaviour appears, rather than patching the one
  task that surfaced it.
- **A stale task brief looks identical to a current one, so regenerating briefs must be
  unconditional.** Task 15 was dispatched from a brief generated before a plan sweep that fixed
  defects in it, so its implementer received and had to re-discover a defect the plan no longer
  contained; the same silent staleness then hit Task 17. The rule that followed - regenerate a
  task's brief immediately before every dispatch, with no judgement call about whether the plan
  changed in a way that affects it - is cheap and the failure it prevents is invisible until it
  happens.
- **A byte-compatibility claim needs a fixture produced by the other application, not a fixture
  written by hand from documentation.** Every "Mac JSON" fixture in the phase was hand-written from
  a belief about how Swift's `Codable` would encode a given type, and that belief was wrong for
  dictionaries keyed by a non-string enum - a defect (C1) that broke Mac interoperability in both
  directions and went undetected through eighteen tasks because the entire compatibility suite was
  internally consistent with itself. The one fixture backing the C1 fix is still hand-written for
  the same reason (no Mac was available) and is explicitly flagged as owed.
- **Mechanical, uniform sweeps over a large plan or codebase are better delegated than performed
  inline.** Two of the team lead's own passes to remove message-argument assertions and vacuous
  `canUndo` checks were each incomplete in a different way (the wrong question applied once, an
  incomplete regex applied the second time); a dedicated agent given a single question to apply
  uniformly found defects both passes had missed. Holding the same repetitive question in mind
  across a long context is a weaker guarantee than delegating it fresh.
- **Keeping raw measurements and per-binary evidence in the ledger, rather than only conclusions,
  paid for itself.** A three-task-old discrepancy in the whole-suite test count was resolved in one
  step by diffing recorded per-binary breakdowns, instead of re-deriving the history from scratch;
  the same habit made the release-vs-debug performance investigation in Task 14 auditable rather
  than anecdotal.
