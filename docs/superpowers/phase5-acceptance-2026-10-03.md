# Phase 5 acceptance, 2026-10-03

This tracks acceptance against the Phase 4 remaining / Phase 5 plan, using the
existing assertions and budgets. The user's explicit goal is full acceptance;
implementation completion and individual passing checks are not a substitute.
The user's Phase 3.5d sampling probes and `.workbuddy` remain untouched. The
full-acceptance goal now includes the pixel-copy compatibility work below.

## Changes found during acceptance

Native Copy and Cut sent document coordinates in HTTP `Origin`. WebView2 owns
that header and replaced it with `http://tauri.localhost`, so the native command
rejected the write as `Invalid clipboard origin`. The production bridge and Rust
command now use `compositor-pixel-origin`; finite-coordinate validation remains.
A native unit check includes the actual browser Origin alongside the new header.

Pixel comparison of a real Windows Forms bitmap producer exposed a second bug:
the synthesized DIBV5 has a repeated RGB mask table after its V5 header. The
converter treated that table as pixels, dropping the final row and making the
two-row sample appear vertically reversed. Conversion now recognizes matching
mask tables only when the complete pixel span establishes their position.
Ordinary V4/V5 bitmaps with embedded masks keep their original offset. New native
tests cover both row directions, alpha and the synthesized extra table. The
Windows behavior is consistent with Microsoft's description of the
[BITMAPV5HEADER bmiColors masks](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapv5header).

Large worker edits now reuse the transferred input buffer for a same-sized
output. The main thread retains at most one spare of 4 MiB to 400 MB, moves
ownership out of retired callers and releases it after 30 seconds idle. This
avoids repeated 400 MB allocations without retaining every history buffer.
Different-sized outputs still allocate their required size.
Histogram and edit workers return unused input buffers as well: the
next edit can reuse them, including after cancellation or supersession. Retired
result buffers also return to the bounded pool when no caller can install them.
The spare is released on pagehide as well as idle timeout. Histogram
bins and result ownership are unchanged.

Staged installation copies each JavaScript chunk directly into the reserved
WASM destination. The old `Vec<u8>` binding copied into an intermediate vector
and then into the destination. Overflow, invalid planes and incomplete results
remain rejected before installation. A real-worker browser check verifies exact
pixels, repeated edits and undo/redo for a chunked layer.

## Production 1206 baseline

Evidence is retained in `build-artifacts/phase5-acceptance/`. Interrupted runs and
failed diagnostic attempts remain retained separately. Builds, performance,
native clipboard, native UI and Mac checks are distinct evidence layers.

- Native workspace: 616 passed, zero failed, 10 existing ignored;
  `native-workspace-bitmap.log`.
- Unit suite: 268 passed in 42 files; `unit-retired-results.log`.
- Production TypeScript/UI build passes; `ui-build-retired-results.log`.
- Full fixed-asset browser suite: 190 passed, zero failed, 29 unchanged opt-in
  performance skips (7.3 minutes); `e2e-retired-results.log`. These skips are
  not performance acceptance.
- Final release WASM SHA-256:
  `928BDB0293AB9D06E1FE0CC2983E633DAD9309860BAC8871C9A1D6EA8FD801EE`.
- Final bundled test WASM has the same SHA-256. The first attempt used an
  optimization-intermediate binary and was interrupted rather than accepted.

The final direct-install binary first passed all five targeted performance
cases. Its independent repetition failed 100 MP Levels input at 461 ms against
450 ms. Returning histogram input buffers subsequently reduced that input to
51 ms, but F1's 100 MP blank-layer fill still recorded a 142 ms gap against
100 ms. The other four cases passed, including 100 MP brush/blur/healing/content
fill (maximum gaps 47/22/20/41 ms), Add Mask and tab drag. F1 is not yet accepted
as stable. Logs: `perf-final-stable-first.log`,
`perf-final-stable-repeat.log`, `perf-histogram-recycle-first.log`.

A broader 11-case baseline passed 10 and failed F1's 24 MP Levels result frame
(409 ms / 100 ms). After unused-input and canceled-result retirement, the
Levels budgets pass, but the 100 MP blank-fill result frame still reaches
491 ms / 100 ms. This broader run finishes 7 passed / 4 failed: Add Mask also
exceeds its frame budget (24 MP Hide Selection 415 ms; 100 MP frames 466-522 ms),
and Content-Aware Fill reaches 104 ms at 24 MP and 314 ms at 100 MP against
100 ms. The 100 MP Delete case passes its original median-frame
budget while recording a 514 ms worst frame. This is retained as a limit,
not waived as jitter. Logs: `perf-page-lifecycle-baseline.log` and
`perf-retired-results.log`. A diagnostic separates render, texture upload and
GPU readback without changing the acceptance test. In the diagnostic, the original
Delete and F1 assertions both pass, but other/cold frames record 21 slow
`readPixels` calls (maximum 403 ms), 13 slow `texImage2D` calls (maximum 50 ms),
and seven slow render calls (maximum 837 ms). These setup/cold measurements are
not F1 acceptance failures and do not reproduce or explain every measured stall.
`trace-f1-gpu-summary.json` and the full retained trace distinguish them.

## Legacy performance instrumentation

The remaining 18 performance cases finish 10 passed / 8 failed
(`perf-remaining-coverage.log`). Six failures reveal stale instrumentation:
those tests wrapped synchronous `jobInput`/`installJob`, while the store now
uses the cooperative async APIs. Missing metrics are not accepted as zero and
not counted as product timings. The other two failures are real recorded
budget overruns: partial-upload frame 498 ms / 33 ms, and the full-size effects
window 513 ms / 150 ms.

The six affected tests now wrap the actual async APIs, capture their measured
CPU copy/install costs, and draw the result frame after awaited installation.
The separate animation-frame gap monitors still charge every yield and stall.
Synchronous `layerPixels` instrumentation remains synchronous. An AST comparison
against the published file verifies all 100 assertions, 14 test titles and 16
skip/timeout/browser-option calls are identical (`perf-instrumentation-integrity.json`).
TypeScript checks pass. The six corrected cases finish five passed / one failed
(`perf-cooperative-instrumentation-direct.log`): C1's 100 MP blank Fill installation
records 508 ms / 450 ms, with result frame 18 ms and maximum gap 23 ms. The
previous missing metrics are superseded by these real timings, not omitted.
Across the 29 distinct performance cases, the latest applicable results are
22 passed / seven failed. This is coverage across separate runs, not a passing
single full performance suite or stable acceptance.

## Production package follow-up

Portable `Compositor-portable-0.8.0-20261003-1206.zip` contains the clipboard
origin/DIB fixes and the final buffer retirement code. Its marker is
`COMPOSITOR_BUILD_0.8.0_20261003-1206`; `package-bitmap-final.log` records the
successful optimized native build. Earlier portables and failed logs remain.
ZIP: 4,908,294 bytes, SHA-256
`DE8B480807C8FDD9E67311D70BCF3886A30D178B193FE5539A86C046911D1496`.
EXE: 11,975,168 bytes, SHA-256
`09F544DB8B62077974844AD0E61D27DF7ADAA58DB3254E91D83D012735034FB1`.
ZIP CRCs pass, the executable matches the archive, and both production and
fixed-test assets carry the release WASM hash above. All three native Mac PNGs
match the previously validated Windows exports exactly; all 34 re-saved and
continued package files match the prior checked native outputs. This retains,
not resolves, the previously quantified Mac-export edge differences. Evidence:
`final-delivery-checks.json`.

The real production window passes seven UI groups with zero page errors and
no development test API, including effect preview toggles, exact parameter
retention after cancel, and effect undo/redo. Evidence:
`native-ui-bitmap-final.log`, `native-final/native-overlay-0.8.0-result.json`.

The same package opens the three 1.4.5 return projects, saves copies and reopens
continued edits: four native reads, four atomic package commits, zero page
errors. This run additionally asserts text font-size undo/redo, rounded-shape
width undo/redo, and effect preview/cancel before Apply. The reopened values
remain font size 38, shape width 150, Stroke 7 and Drop Shadow Distance 14.
Only file-picker choices are substituted; reads, edits, rendering and writes
use the production bridge. Evidence: `native-mac-bitmap-final.log` and
`native-final/native-mac-return/`. This proves the Windows half of those gestures.

The first final external clipboard run aborts at its read-only snapshot:
`CLIPBRD_E_CANT_OPEN` (0x800401D0). No clipboard mutation occurs. The production
read-only probe also reports Windows error 5, and an independent Win32 probe
cannot open the clipboard (error 5, no locking window). The process is on
`WinSta0`, and the readable input desktop is `Default`; a locked desktop is not
established as the cause. Earlier intermediate builds completed native clipboard
protocol checks but failed bitmap pixel comparison, exposing the DIB bug fixed
here. At 12:46 the independent Win32 probe confirms access has returned. The exact
1206 portable then passes six native protocol groups with zero page errors and
no development API: external PNG paste, independent PNG consumption of native
Copy, synthesized Windows bitmap paste, independent bitmap consumption of Cut,
Cut/Paste with Undo/Redo, and private-token editable text paste. All four saved
and consumed PNG/bitmap comparisons have maximum channel difference **zero**,
including alpha and row ordering; two editable UTF-16 text records persist.
The original clipboard is restored with its original format set. This closes
the final native clipboard gate; the denied preflight above remains historical.
Evidence: `native-clipboard-restored-access.log`,
`clipboard-20261003-124643/clipboard-image-checks.json` and the native result.

## Rejected performance experiments

Three controlled browser experiments used the original case bodies and assertions,
with diagnostic wrappers outside the official runner. They are diagnostic
evidence only and do not replace the original acceptance runs above.

- Repeating a known uniform fill's first 4 MiB source chunk did not improve
  100 MP installation CPU time: 260 ms baseline versus 272 ms with the experiment.
  The experimental run also failed the unchanged gradient frame-gap assertion.
  No corresponding application change was adopted.
- Requesting a CPU-backed overlay context did not give stable performance:
  F1 failed at 182 ms baseline and 392 ms experimentally against 100 ms;
  24 MP Content-Aware Fill failed at 232.3 and 195.8 ms; the 100 MP case passed
  baseline but failed experimentally at 111.1 ms against 100 ms.
  No overlay context change was adopted.
- Reusing same-size GL texture objects did not close the failing frame budgets.
  The experimental F1 result failed at 283 ms against 100 ms; its effects-image
  case failed at 158 ms against 100 ms (baseline effects: 162 ms). Partial uploads
  passed experimentally but failed at 284.5 ms against 33 ms in the baseline.
  This mixed result is insufficient evidence of stable improvement, so no
  texture-cache change was adopted. The staged WASM destination already reserves
  its complete plane capacity before copying; chunk-by-chunk vector growth is
  not an outstanding allocation issue in this implementation.

Evidence: `uniform-copy-diagnostic-2.json`,
`overlay-context-diagnostic.json` and `texture-reuse-diagnostic.json`, with their
retained logs.

## Pixel-copy source work in progress

An in-memory-only placement diagnostic retains the returned files byte for byte.
For `Mac-edited`, replacing only fractional layers' draw sampling with Nearest
reduces the original 2,118 changed pixels / maximum 121 to **zero differences**.
For `Mac-created`, it reduces 14,037 changed pixels to 13,117, but differences
remain in scaled text and 52 rectangle-edge pixels. Changing the saved origin or
sampling mode is not adopted as a fix. Evidence:
`pixel-placement-diagnostic/comparison.json`, plus its variant PNGs and runner.

The authoritative source is `git show v1.4.5:Compositor/Rendering/LayerRenderer.swift`
in the adjacent `Compositor-1.2.10` repository, not that repository's older
checkout or the unversioned adjacent `Compositor` directory. Lines 44–47 explicitly
choose `.none` for upright final sampling within 0.001 of 1:1. Line 19 preserves
antialiasing according to the layer's saved setting. The initial copy-only
diagnostic's residual rectangle-edge differences therefore need edge coverage,
not a saved-coordinate change.

Current CPU and GL source work chooses that filter at the final prefilter/device
scale, including clipping sources and reduced/full effects representations.
Only outer rectangle edges receive coverage; internal texture-chunk edges retain
their disjoint geometry. Explicit Nearest keeps its hard edge. Stored transforms,
sampling settings and bitmap records remain unchanged. New regression coverage
checks transparent texel preservation, clipping-source alpha, rectangle edges,
rotation/enlargement exclusions and GPU/CPU agreement. Release WASM, original
Mac return exports and the new production package must still verify this source
before the pixel-copy gate can be closed. Scaled-text fidelity remains open.

The first complete release-WASM checkpoint passes 620 native tests (0 failures,
10 existing ignores), 269 UI unit tests and 191 functional browser cases (29
unchanged opt-in performance skips). Its source/package WASM SHA-256 is
`C801FCED79B12E62F39B4E83F531DB962603C19D3C0A34EB9ACC4EACA61EFCF4`.
Against untouched Mac returns, `Mac-no-edit` and `Mac-edited` now export with
**zero changed pixels**. `Mac-created` improves to 13,089 changed pixels / max 51;
24 of these are shape-edge rounding, the remainder is enlarged text.
Evidence: `pixel-copy-mac-checks/comparison.json` and the PNGs.

The upright 1:1 probe initially retains 54 changed boundary pixels / max 8 in
straight RGBA, but only 1 in premultiplied channels. Quantizing rectangle coverage
to `floor(coverage * 255) / 255` before multiplying colour/alpha gives an exact
scratch-model match to that Mac 1.4.5 PNG. The quantized CPU/GL source and a new
four-flip regression are undergoing their own final WASM/build checks; earlier
checkpoint results do not establish those newer checks.

The first probe GPU comparison was captured before the viewport had settled;
its apparent 150-level difference was not a proven application regression.
After fixing the diagnostic to align the viewport on device pixels and draw
synchronously, both the upright and flipped layer agree with CPU within 1/255.
Evidence: `pixel-copy-upright-final-frame.log`; the earlier diagnostic logs remain
retained. Export comparisons above do not depend on viewport alignment.

The final quantized release-WASM checkpoint also passes 620 native tests with
0 failures / 10 existing ignores, both TypeScript checks and all five rendering
cases, including the four-flip edge test. Source and fixed test assets share
SHA-256 `22665BCEDD97916CB4A88433039FA51A0123DD6435CBBEEECEFFAA3464E43607`.
Actual exports now match `Mac-no-edit`, `Mac-edited` **and** the upright 1:1 probe
exactly. The probe's GPU and CPU agree within 1/255. `Mac-created` retains
13,089 changed pixels / max 51, including its 24 overlapping shape-edge pixels.
Evidence: `pixel-copy-coverage-mac-checks/comparison.json`, `checks.json`,
`pixel-copy-coverage-workspace.log`, `pixel-copy-coverage-render-e2e.log` and
`coverage-quantization-model.json`. The quantized full browser suite passes
**192 cases**, with the same 29 opt-in performance skips. The new production
package results are recorded below; performance remains open.

## Production 1455 pixel-copy checkpoint

Commit `051d23bf488bfdf2822e7f414a8d214b0abf4bc1` is pushed and matches
`origin/codex/phase4-and-phase5`. The portable package is
`build-artifacts/windows-x64/Compositor-portable-0.8.0-20261003-1455.zip`,
4,743,423 bytes, SHA-256
`48E83A21F5A0ECDBF5E381D082E80763F952C498459D3A9D9C5CA8E9BCD7FCD6`.
Its EXE is 11,976,704 bytes, SHA-256
`F97BC821E15CB0A09064D14C067EC2E8AD8FFDEC32FD07D3F2E2FB35191F53C4`.
Bundled production WASM matches the quantized test WASM above. The build wrapper
preserves and restores the original build-info bytes; the historical portable
packages and validation outputs remain retained.

Real production Tauri checks pass all seven UI groups with no page errors and no
development API. Real package I/O checks also pass: four reads, four atomic write
commits, preserved editable text and continued Mac-edited text/shape/effects
edits reopened with width 150, stroke 7 and shadow 14. All three native opening
exports are byte-identical to their current WASM exports. Mac-no-edit and
Mac-edited remain exact against the returned Mac PNGs; Mac-created retains the
same 13,089 changed pixels / max 51. Evidence: `native-ui-pixel-copy.log`,
`native-pixel-copy-0.8.0-result.json`, `native-mac-pixel-copy.log`, and
`native-pixel-copy-mac/native-pixel-comparison.json`.

The 1455 clipboard rerun fails **before mutation**: the helper cannot snapshot
the original clipboard. A separate read-only Windows probe verifies no open
clipboard window, WinSta0/Default, and `OpenClipboard` error 5. This is current
OS access rejection, not proof of an application bitmap regression or a
successful current-package protocol check. Earlier 1206 six-group/four-image
proof remains retained. A Windows manual confirmation is requested; current
clipboard contents were not changed. Evidence: `native-clipboard-pixel-copy.log`,
`clipboard-20261003-150934/helper-error.txt` and
`clipboard-pixel-copy-readonly-preflight.json`.

The uniform-mask prototype does not apply to the failing Add Mask fixture,
which is an antialiased ellipse and hence nonuniform. It records **zero**
uniform-upload substitutions. The baseline fails at 519/150 ms, one no-op variant
passes, and the other fails at 371/150 ms. No optimization is adopted and that
isolated pass does not establish stable performance. Evidence:
`mask-uniform-diagnostic.json` and its retained log. The 24 remaining overlapping
shape-edge pixels have an exact independent model after rounding covered source
bytes before blending; this model is not yet a production fix. Evidence:
`copied-overlap-rounding-diagnostic.json`.

## Further GPU diagnostics

The partial-upload/Add Mask trace reproduces the 24 MP Add Mask result-frame
failure (415 ms / 150 ms). Its marked `readPixels` call consumes 412.7 ms;
overlapping trace events show `WaitForGetOffset` at 412.68 ms and the GPU WebGL
command-buffer execution at 396.95 ms. The partial-upload case passes in this
diagnostic. These measurements locate the reproduced wait, not every previous
failure or a proven root cause. Evidence: `trace-partial-mask-summary.json`,
the full trace and log.

Serializing upload/draw commands with diagnostic-only `gl.finish()` still leaves
100 MP Add Mask frames at 474/496 ms against 400 ms; no such synchronization is
added to the application. Reusing one same-size retired mask with a full
`texSubImage2D` also leaves failures (407/506 ms against 400 ms), so no spare-mask
cache is adopted. Evidence: `mask-gpu-phases.json` and
`mask-spare-diagnostic.json` and their logs.

Backend inventory verifies Intel hardware Vulkan and D3D11on12 contexts. The GL
request falls back to SwiftShader, which is rejected as hardware acceptance.
Original case-body diagnostics still fail Add Mask under Vulkan (458 ms / 150 ms)
and D3D11on12 (529 ms / 400 ms); Vulkan also fails partial uploads (401 ms / 33 ms).
Production remains on the original hardware D3D11 path. Alternative-backend
passes do not supersede the official 22/7 result or establish acceptance.
Evidence: `angle-backend-inventory.json` and `angle-budget-diagnostic.json`.

## Covered source-byte rounding checkpoint

The CPU compositor and GL layer shader now round covered premultiplied source
channels to bytes before blending them over the backdrop. A regression derived
from the Mac-created return checks `[28,29,47,255]` for its partially transparent
edge over `[3,4,24,255]`. The source fix removes all 24 remaining rectangle-overlap
differences without changing the saved transforms or sampling settings.

Release WASM SHA-256 is
`A2083E169200D98D85728AC2F4C1977EA8AA870F14D29C4E77C882682727B919`;
the fixed test assets contain the same bytes. Workspace tests pass 621 with zero
failures and the same 10 ignored. All three TypeScript checks pass. The complete
functional browser run passes 193 with the same 29 opt-in performance skips;
the separate rendering run passes all 14 render/adjust/zoom cases. An earlier
render runner encountered an overlapping preview server and connection-refused
errors; its log is preserved and does not represent a pixel assertion failure.

Actual exports of all three Mac returns and 11 sampling probes preserve every
transform. Mac-no-edit, Mac-edited, explicit Nearest enlargement and the upright
fractional/flipped 1:1 probe are exact. Mac-created now has 13,065 changed pixels,
maximum difference 51, confined to the enlarged text at `[137,267,1280,339]`.
Its overlap edges are exact; enlarged-text, scaled and rotated probe differences
remain open. Evidence: `copied-overlap-mac-checks/checks.json`, `comparison.json`,
`copied-overlap-workspace-final.log`, `copied-overlap-render-cmd.log` and
`copied-overlap-functional.log`.

A diagnostic-only viewport-mask prototype reduces a 6000 x 4000 nonuniform mask
upload to 1136 x 798 samples. After correcting its cache transition, the actual
frame differs by at most one channel level, with zero samples above the existing
GPU tolerance of two. The original Add Mask case passes for both the baseline
and prototype in the subsequent isolated diagnostic. This does not establish a
fix for its intermittent performance failures, so the prototype is not adopted.
The original failing cache-transition log remains. Evidence:
`mask-viewport-pixels-cache-fix.log` and `mask-viewport-diagnostic.json`.

Fitting the Mac-created text's alpha by sampling phase still leaves residuals;
77 of 81 phase groups cannot be matched by any four-tap linear coefficients
followed by a single nearest-byte rounding within that grouping. This is a
diagnostic constraint on the model, not a general claim about Core Graphics.
Neither fitted coefficients nor the approximate eighth-phase filter is adopted
as an exact Mac renderer. Evidence: `text-phase-fit-diagnostic.json`.

## Production 1556 covered-source checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-1556` contains the covered-source fix and the
release WASM above. Its portable ZIP is 4,743,691 bytes, SHA-256
`CAB371029BE5FBE2FE41CA84F8912C7876974437B0FC691198CE217ED312AA3A`.
The 11,976,704-byte executable has SHA-256
`8E6B2456BF31519B498DD95A07A1E503D715ABB52C181CB137A207AB9E739954`.
Archive CRCs pass and the archived executable matches the one actually tested.
The build-info source bytes are restored after packaging.

The real production window passes all seven UI groups with zero page errors and
no development test API. Four native Mac package reads and four atomic save
commits pass, including continued editable text, shape width 150, Stroke 7 and
Drop Shadow Distance 14 after save/close/reopen. All three native open exports
are byte-identical to the fixed release-WASM exports; the remaining Mac-created
text differences above therefore also apply to this production package.
Evidence: `native-ui-copied-overlap.log`, `native-mac-copied-overlap.log`,
`native-copied-overlap-mac/` and `copied-overlap-package-integrity.json`.

Clipboard access has returned: production 1556 passes all six native clipboard
groups through the real bridge and an independent Windows Forms consumer.
All four saved/consumed PNG and bitmap checks have maximum channel difference
zero, including alpha and row ordering; two editable UTF-16 text records persist.
The original format set is restored. The previous access-denied preflights are
retained as historical failures, not a current 1556 clipboard failure. Evidence:
`native-clipboard-copied-overlap.log` and `clipboard-20261003-161117/`.

The subsequent single complete performance run uses the same release WASM and
fixed assets, after compilation and native GUI checks have finished. All 29
original cases run with one worker and no retries: **27 pass / two fail** in
8.3 minutes. The failures are the 24 MP Levels histogram frame gap (151 ms,
budget strictly below 150 ms) and the 100 MP blank-layer gradient frame gap
(102 ms, budget strictly below 100 ms). Neither limit is changed. The earlier
seven failure groups do not all reproduce in this run, but a single passing
observation does not establish a fix or stable acceptance for intermittent
failures. This current full-run result supersedes the earlier 22/7 aggregation
as the latest observation, not as full acceptance. Evidence:
`copied-overlap-performance.log`.

## Production 1654 asynchronous histogram checkpoint

Levels and Curves now prepare large histogram inputs with the existing cooperative
copy path. The panel remains editable while the copy is pending; a request epoch
rejects input/results from an older instance after close/reopen. Allocation failures
close the waiting panel and report the error. Both stored-buffer copies and staged
result installation yield before copying if their initial allocation already consumes
the 8 ms quantum. CPU accounting includes allocation and excludes awaited frames.

Validation: 274 unit tests across 42 files pass, all three TypeScript checks pass,
and the fixed-asset build passes. The complete functional suite passes 193 cases
with the same 29 opt-in performance skips. The engine is unchanged: the 621 native
passes and 10 existing ignores above still apply. New tests cover waiting-panel
editing, stale same-kind reopen results, asynchronous failure, exact staged bytes,
and cancellation during the initial yielded frame. Evidence: `histogram-async-unit.log`,
`histogram-async-test-build.log`, `histogram-async-functional.log` and
`allocation-frame-client-final.log`.

`COMPOSITOR_BUILD_0.8.0_20261003-1654` portable ZIP is 4,743,566 bytes, SHA-256
`A8633356C737FCD34D98C1ABDCB2E7C2D9C547F2835369B07EACFC94F7530E9F`.
Its 11,976,704-byte executable has SHA-256
`5BF540A516F151FB8FBC2BC3CD4F26D1F64AC3F89026A82CBDFE9531937B00E4`.
The archived executable matches the actual tested package and CRC checks pass;
build-info source bytes are restored. Release WASM remains the same as 1556.

The real production window passes eight UI groups with no development test API
and zero page errors. The additional group fills a 24 MP canvas, opens Levels
while the histogram is pending, cancels, reopens and waits for the new histogram.
Four native Mac package reads and four atomic save commits pass; edited font 38,
shape width 150, Stroke 7 and Shadow Distance 14 persist after reopening.
All three native open exports are byte-identical to the fixed release-WASM
exports. Evidence: `native-ui-histogram-async.log`,
`native-async-jobs-0.8.0-result.json`, `native-mac-histogram-async.log`,
`native-async-jobs-mac/` and `histogram-async-package-integrity.json`.

The 1654 read-only clipboard preflight reports Windows access denied (OS error 5).
The mutating protocol suite is not attempted against that preflight. The successful
1556 six-group/four-exact-image evidence remains valid for 1556; it does not establish
the same protocol result for 1654. No user clipboard data is changed by this denied
preflight, and no desktop permissions are changed to bypass it.

The two targeted original job performance cases pass, but the subsequent single
complete 29-case run is **26 passes / three failures** in 8.5 minutes. This is the
latest full-run observation, superseding the historical 27/2 above. The histogram
case passes (24 MP gap 87 ms, 100 MP gap 23 ms). The failures are:

- F1 100 MP Levels: worker-edit frame gap 109 ms, strictly below 100 ms required.
  The actual result-display frame is 12 ms and installation CPU time is 363 ms.
- Growing-mask 24 MP gradient at fit: drag total 64 ms, strictly below 50 ms
  required (engine 12 ms, frame 53 ms).
- C1 100 MP blank gradient: worker-edit frame gap 114 ms, strictly below 100 ms
  required (result frame 17 ms, installation CPU time 355 ms).

Evidence: `histogram-async-performance-targeted.log` and
`histogram-async-performance.log`. Budgets, assertions, warm-ups, skips and retries
are unchanged. Diagnostic traces execute the original case bodies and assertions,
but do not reproduce those three failures; C1 instead has a 24 MP result-frame
failure of 517 ms against its 350 ms budget. The 100 MP blank-gradient reservation
contains a 64 ms V8 sweeping completion. This identifies a remaining allocation
cost, not a complete root cause or stable fix. Evidence:
`async-histogram-job-trace-results.json` and `async-histogram-*-trace.json`.
A diagnostic mask-storage reuse prototype fails the original 24 MP post-apply
frame budget (423 ms / <150 ms), while the isolated baseline passed. It is not
adopted; evidence: `mask-storage-job-trace-results.json`.

## Output capacity and texture-object work in progress

Blank pixel Fill/Gradient jobs can now borrow an existing exact-size canvas RGBA
spare immediately before worker posting. It is a separate output-capacity buffer,
never a WASM input raster. The worker copies its complete output into that buffer;
unused capacity is returned with the normal recycled inputs. Mask edits and
histograms do not borrow it. This avoids simultaneously retaining the old spare
on the main thread while allocating the same-sized blank-layer result.

The real-worker regression verifies complete-byte equality against the direct
engine for both Fill and Gradient, exactly one undo step, blank undo, exact redo
and real transfer detachment. The initial fixture had no layer and failed before
any edit; adding its explicit blank layer corrects the fixture. Original assertions
and timeouts are preserved. Evidence: `blank-output-pool-staging-final.log`.
The output-only source checkpoint passes 277 unit tests and 194 functional cases
with 29 unchanged opt-in skips (`blank-output-pool-unit-full.log`,
`blank-output-pool-functional.log`). Its targeted performance run passes C1 and
growing-mask gradients, but F1 fails with a 24 MP Levels result frame of 511 ms;
100 MP fill also logs a 104 ms worker-edit frame gap. Those failures are retained
in `blank-output-pool-performance-targeted.log`.

A GPU trace of the original F1 body fails at a 101 ms timed 24 MP Fill result frame:
95 ms is `readPixels`, overlapping a 94.5 ms GPU WebGL command flush. A separate
325 ms cold warm-up readback overlaps about 304 ms of Chromium raster-worker
flushing; it is not a timed acceptance failure. This establishes GPU command waits,
not a complete explanation for the unprofiled 511 ms observation. Evidence:
`blank-output-gpu-trace-summary.json` and `blank-output-gpu-job-trace-results.json`.
The host has about 18 GiB free physical RAM at the subsequent snapshot; low free
system RAM is not inferred as the cause.

A diagnostic same-sized texture-object reuse variant passes the original F1
assertions, retaining all required `texImage2D` whole uploads. The source now keeps
existing chunk GL objects for matching dimensions and prefilter levels, replaces
every texel and updates filters; different dimensions or levels still rebuild.
No upload assertion is changed. A chunk-boundary unit regression covers the new
bytes/revision, filter mode, both full chunk uploads, resized replacement and
separate document ownership. Current 278 unit tests and all three TypeScript
checks pass; the rebuilt fixed assets pass. Complete performance and functional
results are pending in this work-in-progress section.

An independent host PInvoke probe confirms current `OpenClipboard` access denied
(error 5) on WinSta0 / Default in active console session 1, with no clipboard
owner process returned. It reads no clipboard contents and performs no mutation
or permission bypass. Evidence: `windows-clipboard-host-access-probe.json`.

## Production 1901 output capacity and texture reuse checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-1901` contains the reviewed output-capacity
and texture reuse changes (source commit `ca361b7d0de45f2870cee142c1b413aa2193f3d8`,
verified at the authorized GitHub branch). The portable ZIP is 4,743,463 bytes,
SHA-256 `26D10B5F0038C64560278F1D11E1A01EC659DF130EC2B5261285CB8EF8B4D34F`.
Its 11,976,704-byte executable has SHA-256
`C61FE8B5BA54D3C514807CB8D96608E2E3FDCEA5CCC36C964A49F0A7554D63B1`.
ZIP CRC checks pass and its executable exactly matches the tested portable file.
The build-info source bytes are restored; the release and bundled WASM SHA
remains unchanged. Evidence: `output-capacity-package-build-canonical-env.log`
and `output-capacity-package-integrity.json`.

Eight real native UI groups pass, including 24 MP fill and asynchronous histogram
cancel/reopen, with the production native bridge, no development test API and
zero page errors. Four Mac package reads and four atomic save commits also pass.
Continued edits preserve the font size 38, shape width 150, Stroke 7 and Shadow
Distance 14 after reopening. All three native Mac open PNGs are byte-identical
to the prior 1654 exports, so the exact no-edit/edited matches and the remaining
Mac-created enlarged-text discrepancy still apply. Evidence:
`native-ui-output-capacity.log`, `native-output-capacity-0.8.0-result.json`,
`native-mac-output-capacity.log` and `native-output-capacity-mac/`.

A separate normal-user host PInvoke preflight now opens/closes the clipboard
without reading contents. Its captured GetLastError is stale on success and is
not treated as a failed call. This alone does not prove OLE snapshot or native
HWND clipboard access: the 1901 native UI read still reports access denied, and
the independent protocol helper fails at OleGetClipboard with
`CLIPBRD_E_CANT_OPEN (0x800401D0)` before snapshot completion or any mutation.
The protocol suite therefore never starts, and user clipboard contents are
unchanged. No permission/desktop change, protocol retry or unsafe snapshot
workaround is used. Evidence: `output-capacity-clipboard-preflight.json`,
`native-clipboard-output-capacity.log` and
`clipboard-20261003-191221/helper-error.txt`. The prior successful production
1556 clipboard evidence is retained; it is not a 1901 acceptance result.

The subsequent C1 diagnostic runs its unchanged original body and assertions.
It does not reproduce the full-run 100 MP gradient gap failure (96 ms / <100 ms
in the diagnostic), but fails the 24 MP gradient result-frame budget at
366 ms / <350 ms and also records a 100 MP fill worker-edit gap of 463 ms
against <100 ms. The failing 24 MP frame includes a 358.5 ms readPixels call
that overlaps 357.0 ms of GPU command-buffer flushing. This identifies another
synchronous GPU wait without proving the cause of the separate full-run
160 ms gradient gap. These failures remain open; no untested scheduling or
sampling change is adopted. The first diagnostic matched an incorrect case
prefix and failed before executing a case; the corrected entry selects the
original `ruling C1:` test. Evidence: `texture-c1-profile-job-trace-final.log`,
`texture-c1-profile-job-trace-results.json`, `texture-c1-profile-summary.json`
and `texture-c1-profile-blank-gradient-trace.json`.

## Completed-job scratch heap reclamation in progress

A more detailed C1 trace keeps the original body and assertions. Its failures
are 24 MP and 100 MP gradient worker-edit gaps of 333 ms and 385 ms / <100 ms.
The corresponding rAF intervals overlap 327.2 ms and 369.3 ms of GPU
RendererCompositor command flushing, with no overlapping main-thread long task.
The 334 ms GetGLError wait is in the first navigation's cold warm-up, not either
failed timed gradient window. No per-frame application getError call exists,
so removing one would not fix those failures. Evidence:
`texture-c1-detailed-job-trace.log`, `texture-c1-detailed-summary.json` and
`texture-c1-detailed-blank-gradient-trace.json`.

The separate diagnostic-only `--enable-gpu-service-tracing` launch records
per-driver GL calls using [Chromium's documented tracing method](https://chromium.googlesource.com/chromium/src/+/master/docs/gpu/debugging_gpu_related_code.md).
It passes the original C1 assertions once, while its timed 100 MP result frame
still contains 351.6 ms in glReadPixelsRobustANGLE (367 ms total / <600 ms).
It does not establish a source fix or close earlier intermittent failures.
The official regression configuration receives no extra tracing flags. Evidence:
`texture-c1-service-job-trace.log` and `texture-c1-service-blank-gradient-trace.json`.

Texture inspection disproves the proposed level-only reuse opportunity: the
24 MP gradient changes from a 1500 x 1000 preview to a 1264 x 1000 result; the
100 MP gradient changes from 1250 x 1250 to 1123 x 1250. No equal-sized texture
can be reused for these transitions. The first inspection attached too early
before a renderer existed; the corrected attachment records all four navigations.
Evidence: `texture-c1-texture-inspect-job-trace-final.log` and
`texture-c1-texture-transitions.json`. A same-process, same-STA clipboard
comparison fails Win32 open with both a null handle and a valid own hidden
window (error 5), and fails OLE access with CLIPBRD_E_CANT_OPEN. It reads no
contents and changes no data or permissions. Evidence:
`clipboard-owner-compare.json`. The host's power plan is read-only checked
as Balanced; no power setting is changed.

A separate 400 MB synthetic copy experiment compares fresh browser pages.
Original staged WASM copies take 254.2 and 302.0 ms CPU. Zero-prefilling each
chunk before its copy costs 326.9 ms total (276.6 ms just prefilling); plain
ArrayBuffer copying costs 339.6 ms and verifies every byte. Prefill adds CPU
cost and is not adopted. Evidence: `staged-copy-prefill-diagnostic.json`.

The original F1 diagnostic then records a 100 MP Levels worker heap of
960,626,688 bytes after its completed job. That is below the original absolute
1 GiB cap, so it otherwise stays resident while the main engine installs its
400 MB result. An early-reclamation prototype retires that idle heap and
passes the unchanged F1 body, with 308 ms installation CPU, 12 ms result frame
and 20 ms worker-edit gap for 100 MP Levels. It does not prove stable acceptance
by itself. Evidence: `worker-768-prototype-job-trace.log` and
`worker-768-prototype-job-trace-results.json`.

The source now retires a worker after a completed result above 768 MiB, before
resolving that result to its caller. The original 1 GiB absolute cap and fatal
trap handling remain. Transferred pixels belong to the main thread already;
no input retry or result reexecution occurs, and queued jobs use a fresh worker.
A regression covers the measured heap size below the absolute cap, unchanged
result bytes and the queued job on the replacement. All 279 unit tests, all
three TypeScript checks and the fixed-asset build pass. Six real-worker cases
pass, including exact selected Levels/blur pixels and undo, complete Fill/Gradient
bytes, transferred output reuse and rejection of incomplete staged planes.
The complete original performance run finishes with 20 passes and nine failures
in 9.1 minutes. Functional regression passes all 194 cases with the 29 unchanged
opt-in performance skips in 6.1 minutes. Both run sequentially with one worker
and no retries, retaining separate outputs and all prior failed evidence.
100 MP Levels installation is 284 ms / <450 ms (24 MP 64 ms); 100 MP blank Fill
installation is 269 ms. GPU result frames remain over budget: 24 MP Levels
312 ms and 100 MP Levels 230 ms / <100 ms. Other failed gates are Add Mask,
partial uploads, histogram/commit, gradient previews, growing-mask gradients,
small-layer mask Fill, gradient tool on a small mask, and 100 MP brush/retouch
(ContentFill frame gap 101.5 ms / <100 ms). C1 passes this run, but previous
intermittent failures remain open. Evidence: `worker-reclaim-source-check.log`,
`worker-reclaim-jobs-e2e.log` and `worker-reclaim-regression-summary.json`.

## Preview-to-result texture grid reuse in progress

The first F1 texture diagnostic selected the separate Delete-after-job case
because its title shares the prefix; its evidence is preserved and is not used
to justify the result-frame change. The corrected selector runs the unchanged
F1 result body and assertions, recording all four navigations. For Levels, the
settled 24 MP preview and result both upload 1500 x 1000, with prefilter levels
1 and 2; the 100 MP pair both upload 1250 x 1250, with levels 1 and 3.
The diagnostic passes once (100 MP Levels installation 270 ms, result 75 ms,
worker gap 25 ms); this does not close the preceding full-run failures.
Evidence: `texture-f1-result-level-inspect-job-trace.log`,
`texture-f1-result-level-inspect-job-trace-results.json` and
`texture-f1-result-level-transitions.json`.

The source now reuses existing GL chunk objects for the same physical raster
dimensions even when its source prefilter level changes. Byte keys, revisions,
filter mode and level metadata still change; every texel is replaced through
the existing complete texImage2D uploads. Changed dimensions and separate
documents retain their existing ownership behavior. The additional regression
checks a two-chunk boundary, every byte, LINEAR filter updates and the new
level-sensitive upload cache. The initial source replacement did not match
CRLF line endings; the new regression failed against the unchanged guard, and
its failed log is retained. After applying and verifying the actual source diff,
all 280 unit tests, all three TypeScript checks and fixed-asset build pass.
Complete regressions are in progress with separate output paths and all original
acceptance budgets unchanged. Evidence: `texture-level-source-check.log`,
`texture-level-source-check-final.log` and `texture-level-regression-summary.json`.

## Additional Mac alpha sampling handoff

Four generated projects isolate white-pixel alpha filtering from text rendering:
horizontal/vertical High-quality strips at 1600 percent and a 32 x 32 alpha grid
scaled to 34 x 34 with High quality or Smooth. All four open/export in the Windows
release WASM, preserve their exact transforms, and draw within the unchanged GPU
pixel tolerance (strip maximum zero, grid maximum one channel level).
The first validation compared JSON property order and falsely reported a transform
change; the corrected structural comparison preserves every numeric field.
Evidence: `mac-alpha-probe-validation-final.log` and
`alpha-probe-validation-mac-checks/checks.json`. No Mac oracle is claimed yet.

The local handoff is `mac-alpha-probes-20261003.zip`, 7,241 bytes, SHA-256
`616edb17d97adf8980f5a086c1489e18e6a173afb59fb771d4c409fc97eb0455`;
archive CRC verification passes. README requests unedited transparent PNG exports
in Mac Compositor 1.4.5. RESULT also lists the still-unconfirmed text/shape undo-redo
and effects preview/cancel/apply/reopen gestures. The Mac return report still leaves
those fields blank. The original return files and older user sampling probes are
unchanged.

## Mac confirmation still required

Compositor 1.4.5 return files already prove live opening, saving and text/shape
editing. The return sheet leaves undo/redo and effect preview/cancel/apply blank,
and the manifests retain the original effect parameters. These gestures cannot
be established from the saved files alone.

On a copy of `Mac-edited.comp`, confirm the following:

1. Change the point text, then Cmd+Z and Cmd+Shift+Z. Do the same after resizing
   the rounded rectangle. Confirm the content/size returns and reapplies.
2. Open the rounded rectangle's effects. Change Stroke Size from 5 to 7 and
   Drop Shadow Distance from 12 to 14; toggle preview, then cancel. Reopen and
   confirm the original 5 and 12 remain.
3. Apply those changes, save to a new project, close and reopen. Confirm 7 and
   14 remain and the rectangle and text remain editable.

The existing exact no-edit composite match and quantified edited/created edge
differences are recorded in [the Mac return report](mac-roundtrip-results-2026-10-03.md).
Pixel-identical edited/created export acceptance remains open; no sampling fix
is inferred from metadata preservation or the font-face correction.

## Outstanding gates and publication scope

Full acceptance remains incomplete. The nine current performance failures and
earlier intermittent failures need fixes and repeatable evidence under their
original budgets. Production 1206
clipboard protocol and exact synthetic-image pixel checks pass; production 1556
also passes six native groups and four exact image comparisons, with the original
clipboard restored. Current production 1901 OLE snapshot preflight fails before mutation. Earlier
protocol-only checks and bitmap unit tests alone were insufficient. Mac gesture
confirmation remains pending. Current Mac-edited export is exact; Mac-created
enlarged text remains quantified and open; covered overlap edges now match. No assertion,
budget, warm-up policy, skip, retry, or image tolerance
has been weakened to close those gates.

The checkpoint stages only reviewed source, regression tests and these reports.
Mac return data, clipboard data, test traces, packages, `.workbuddy` and the user's
untracked sampling probes remain local and untouched. The goal remains active.
