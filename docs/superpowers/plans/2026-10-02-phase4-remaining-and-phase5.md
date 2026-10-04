# Phase 4 remaining work and Phase 5

The user authorized publication of the existing 0.6.0 work, completion of Phase 4,
and automatic continuation into Phase 5 on 2026-10-02. The earlier confirmation
gate between those phases no longer applies.

Publication completed: branch `phase4.5`, commit
`e1be481b83b3209f200c10eb0ba7ab954397ba14`, verified against the remote ref.
Implementation continues on `codex/phase4-and-phase5`.

## Delivery sequence

- [x] Phase 4b-2 implementation: Windows PNG clipboard, Cut / Copy / Copy Merged / Paste,
  Layer via Copy, selected-pixel movement and duplication, floating selection
  resize / rotate / distort, exact cancellation, one undo per gesture.
- [x] Phase 4c implementation: Brush / Eraser, hardness / size / opacity / smoothing,
  Shift straight lines, mask painting, Blur radius, aligned Clone Stamp with
  current-layer and all-layer sampling.
- [x] Phase 4d implementation: the three Spot Healing modes and Content-Aware Fill with
  preview, cancel and commit, including extension beyond a layer's old edges.
- [x] Phase 5 implementation: editable text with UTF-16 font and color runs, live shape
  redraw during resizing, Stroke and Drop Shadow editors.

## Acceptance

Use the v1.4.5 Mac sources as the behavioral oracle. Port the original healing
and content-fill kernels, rather than substituting an unrelated algorithm.
Preserve layer effects across floating-selection merges (the specified Windows
deviation). Keep test assertions and performance budgets unchanged. Test
source behavior, browser interactions, native system clipboard and packaged
Windows runtime separately. Measure the new interactive paths in release WASM
on the hardware Edge renderer at 24 MP and 100 MP. Preserve the previous
portable releases and all unrelated user files. Record incomplete evidence
honestly; passing source tests does not establish native or Mac interoperability.

The initial implementation excluded the separate Phase 3.5d sampling backlog.
The subsequent explicit full-acceptance goal requires fixing the remaining Mac
export differences. Its current source work includes the verified Mac 1.4.5
upright 1:1 pixel-copy rule and its antialiased rectangle edge, plus the measured
Mac 1.4.5 byte interpolation for affine enlargement. Exact rotated-edge and
distortion compatibility remain separate unproven scope; the user's earlier
sampling probes remain local.

Implementation boxes describe delivered code, not completion of every acceptance
gate. The delivery report records validation results and remaining native
clipboard, large-image performance and Mac interoperability evidence separately.

- [x] Current unchanged native workspace coverage: 624 passed, zero failures,
  10 existing ignored, including the exact-half regression.
- [x] Vitest: 287 passed. Current TypeScript and fixed-asset UI build pass.
- [x] Historical separated-shader full fixed-asset browser suite: 198 passed,
  29 unchanged opt-in skips, one worker and zero retries (7.3 minutes).
- [x] 0.8.0 portable build and real Tauri window checks, including visible
  painting, eraser/undo, blur, clone, healing, fill, text and effects.
- [x] Live Windows clipboard interoperability: production 1206 passes six native
  protocol groups and four exact image comparisons; the original clipboard is restored.
- [x] Production 2139 clipboard verification: six native protocol groups,
  four exact RGBA comparisons, two editable UTF-16 records and original formats
  restored.
- [x] Historical production 1556 clipboard verification: six native protocol groups,
  four exact image comparisons, editable UTF-16 text and original formats restored.
  The historical 1455 access-denied preflight is retained.
- [x] Historical production 0244: ZIP/EXE/WASM identity, eight native UI groups,
  four Mac reads and four atomic saves; all three opened PNGs match 2328 bytes.
- [x] Production 0244 native clipboard interoperability: a later unchanged
  original protocol passes all six groups and the exact reader; initial OLE
  snapshot refusal and intermittent paste failures remain recorded.
- [x] Complete original 1527 large-image performance run: 29 passed, zero
  failures/skips, one worker and zero retries (8.8 minutes) in the received
  ordinary interactive run. Historical failures remain; their intermittent
  cause is not claimed fixed.
- [x] Live Mac return files open/save/reopen and continued text/shape/effect
  editing in the production Windows package.
- [x] Current release-WASM exports match Mac-no-edit, Mac-edited and the
  fractional/flipped upright 1:1 probe exactly, with transforms unchanged.
- [x] Mac gesture records for undo/redo and effect preview/cancel are received.
  Mac-created enlarged-text export now matches every complete RGBA byte.
- [x] Mac effect Apply 7/14 persistence: new Mac-edited-new.comp contains 7/14;
  production 1256 export is exact and native save/reopen retains editable
  text/shape and 7/14. Original 5/12 evidence is preserved.
- [x] Production 1256 byte-interpolation package: ZIP/EXE/WASM identity,
  eight native UI groups, nine native Mac reads/saves, eight exact full-PNG
  comparisons and six clipboard groups with independent RGBA/text/restoration.

- [x] Production 1527 package identity and eight native UI groups pass.
  Independent fresh-profile native Mac checks pass 11 reads / 10 atomic saves
  and all nine complete PNGs exactly; new Mac 7/14 persists on reopen.
  An initial Save As timeout is retained and not declared fixed.
- Historical failed production 1527 independent clipboard stability: first interactive
  protocol passes all six groups and strict RGBA/text/style/restoration; the
  second fails at Paste after Cut (two layers instead of three within the
  original five seconds). Both original format sets restore. Clipboard-only
  observation is prepared and passes CheckOnly; earlier host snapshot and
  elevated endpoint failures remain separately recorded.

### Current local Windows candidate, 2026-10-04

Production `COMPOSITOR_BUILD_0.8.0_20261004-2216` includes the file-operation
status fix. A held recent-file registration deterministically demonstrates
the old invisible busy guard; the fix keeps working visible until the guard
releases, after which Cut and Undo pass. Three type checks, 287 unit tests and
199 full functional cases pass (29 original opt-in skips, one worker, zero
retries). Native code and WASM are unchanged, retaining 624 native passes /
10 existing ignored. The new portable passes eight original UI groups,
11 native Mac reads / 10 atomic saves, all nine full-PNG exact comparisons
and saved/reopened editable text, shapes and effects 7/14.

The user confirms local Windows operation. Two original clipboard pairs with
GameViewer temporarily absent pass all four native protocols and strict
RGBA/text/style/restoration readers. Fresh profiles and ports are independently
owned. The process monitor observes no GameViewer PID in 545 samples at
200 ms intervals; the installed launcher restarts afterwards. Read-only
backup comparisons confirm complete current clipboard formats/data match
all four pre-test snapshots. Earlier failures are retained and not reclassified.

The complete 29-case new-asset performance run retains 24 passes / five
failures (page setup, typed-width frame, gradient drag/installation and
blank-gradient frame gap). The single separate complete GameViewer-absent
environment control finishes with 27 passes / two failures: 100 MP gradient
drag at 1:1, 58 ms / <50 ms, and motion-blur eyedropper, 2,122 ms / <2,000 ms.
Both complete runs retain unchanged assertions/budgets and zero retries,
with frozen assets verified after execution. GameViewer is reopened after
the control. Full Phase 5 acceptance remains incomplete; targeted kernel
profiling and byte-preserving optimization are the next work. See the current section of
[the acceptance record](../phase5-acceptance-2026-10-03.md) for package identity,
current results, environment limits and retained failures.

Acceptance evidence, including historical checkpoints, is [the 2026-10-03 acceptance record](../phase5-acceptance-2026-10-03.md):
622 native tests, 287 unit tests and 195 current functional browser passes
with 29 original skips. Production 1654 passes eight native
UI groups and four native Mac package reads/writes; production 1901 packages the newer
output-capacity and texture reuse changes and also passes those native UI/Mac
checks. Production 1556 passes six
clipboard groups with four exact image checks and restoration; 1654 clipboard
access was denied at read-only preflight. Production 1901 protocol validation
stops before any mutation because the independent OLE snapshot cannot open
the clipboard, despite a separate host Win32 open/close succeeding. The previous output-capacity revision full run of all 29 performance
cases was 26 passes / three failures: 100 MP Levels installation CPU 476 / <450 ms,
100 MP blank-gradient frame gap 160 / <100 ms, and a selection-Levels page-setup
timeout before its test body. All four result-frame budgets pass in that run.
A separate diagnostic proves the original F1 body can pass with texture object
reuse, but it does not close the remaining performance failures. All original
budgets and assertions remain unchanged. Four synthetic alpha projects are
ready for Mac 1.4.5 export to identify the remaining enlarged-text sampling
difference; returned Mac exports and the pending gesture confirmations are
still required. Implementation and these passing layers do not establish full
acceptance.

Completed-job scratch heap reclamation is now implemented and passes 279 unit
tests and six real-worker cases. The original 1 GiB absolute cap remains;
completed results above 768 MiB retire their unused worker before installation.
The measured 960,626,688-byte Levels heap is reclaimed in the original-body
prototype, whose 100 MP installation is 308 ms / <450 ms. The current source
full performance run finishes 20 passes / nine failures; all 194 functional
cases pass with 29 unchanged skips. 100 MP Levels installation is 284 ms /
<450 ms, but result frames and other responsiveness gates remain open.
Production 1901 evidence
applies to the preceding output-capacity/texture revision. Full acceptance
remains open until the new source, package and outstanding Mac/native gates
are verified.

The corrected F1 diagnostic identifies same-size preview/result grids with
changed source prefilter levels. Texture object reuse now covers this transition
while preserving complete uploads, bytes, filters, cache metadata and document
ownership. All 280 unit tests, three TypeScript checks and fixed-asset build
pass; the texture-level full performance run was interrupted by a tool session
reset and is retained separately. No terminal performance result is claimed.

The first draw now fits a new large canvas before uploading its textures,
avoiding the unnecessary full-resolution initial upload. All 280 unit tests,
three TypeScript checks and fixed-asset build pass; both original partial-upload
cases and the new exact pixel/initial upload regression pass. A complete
original 29-case performance run finishes 21 passes / eight failures (8.7m);
all 195 functional cases pass with 29 unchanged skips (6.0m), one worker and
zero retries. Production 2041 builds after those sequential runs; ZIP CRC,
archived executable identity, unchanged WASM, eight native UI groups, four
Mac package reads and four atomic saves pass. Its clipboard read remains
Windows access denied; user interactive verification and Mac confirmations
remain pending. No performance failure is waived.

Interactive input preparation now reserves worker priority before copying.
Running effects are displaced and replacement startup overlaps the input copy;
queued effects resume after all preparations submit or release, including
cancellation and errors. All 287 unit tests, three TypeScript checks and fixed
assets pass. The unchanged preemption case passes three internal rounds in its
targeted run (300/312/299 ms) and three rounds in the complete suite
(258/232/228 ms), all under the original <400 ms dispatch budget. Six real-worker
regressions and all 195 functional cases pass. The full original performance
suite finishes 25 passes / four failed cases (11.0 minutes): gradient previews,
eyedropper, full-canvas rectangle creation and the C1 blank-gradient worker gap.
Earlier intermittent failures remain open; no assertions or budgets change.
Production 2139 builds after functional success and passes ZIP CRC/EXE/WASM
identity checks, eight real native UI groups, four Mac package reads and four
atomic save commits. All three Mac open PNGs remain byte-identical to 2041.
Its clipboard access changes from denied to no-image; a single original guarded
protocol run then passes all six native groups and four strict image comparisons
with the original Text/UnicodeText clipboard restored. Windows interactive
verification is no longer required. Mac alpha returns and gesture confirmation
remain pending, alongside the original performance gates.

Solid rectangle rasterization now avoids the full grey coverage intermediate,
with exact independent reference coverage over fractional bounds and all
channel values. All 622 native tests and 287 unit tests pass; all nine targeted
shape/text/effect browser cases and all 195 functional cases pass with the
original 29 opt-in skips. The 100 MP rectangle takes 1603 ms in the targeted
original case and 1614 ms in the full suite, both below the original 2000 ms
limit. The complete original 29-case performance run finishes 25 passed / four
failed (27.5 minutes): three 30-second page-fixture timeouts before test bodies
and C1 blank-gradient frame gap 134 ms / <100 ms. All previous intermittent
failures remain recorded. Production 2328 builds after functional success and passes ZIP/EXE/WASM identity,
eight native UI groups, four Mac package reads and four atomic saves. All three
Mac open PNGs remain byte-identical to 2139. Clipboard validation stops at the
read-only snapshot before mutation with CLIPBRD_E_CANT_OPEN; current-package
clipboard acceptance remains open. A separate original-budget run passes the
three page-fixture-failed cases, but does not replace the full suite result.
The early-allocation diagnostic still fails an original C1 result-frame budget
and is not adopted. Mac alpha returns and gestures remain pending, so the
full-acceptance goal stays active.

A new staged-plane browser regression passes on the unchanged production-2328
WASM: all three planes keep exact bytes, one undo step, cancellation and every
size/overflow/incomplete-transfer guard. An incremental-reservation candidate
passes 287 units and nine worker cases but fails all three selected original
performance cases (including C1 gaps 107/160 ms against <100 ms). It is rejected;
the checked baseline source and WASM are restored exactly. Only the safety test
is retained, with three current TypeScript checks passing. The latest read-only
clipboard probe still returns Windows error 5. No original budget is waived.


The framebuffer 256-pixel candidate completes the full original performance
suite at 17 passed / 12 failed (10.3 minutes), followed by all 197 functional
cases passing with 29 original opt-in skips (7.3 minutes). It is not adopted.
The same-environment immutable-baseline comparison passes partial uploads
(24/100 MP fit 6.9/4.8 ms) but fails F1 result frames and C1 gradient gap,
so neither baseline failures nor candidate regressions are waived.
A smaller 64-pixel plain-view candidate retains the existing 256-pixel
spatial-margin policy and resets only changed canvas dimensions. All 287 units,
three TypeScript checks, fixed assets and 19 rendering/resource cases pass.
An independent old-renderer run confirms its new setter regression fails
exactly at two unnecessary width writes. Six original performance cases finish three passed / three failed (2.8 minutes):
F1 result frame 235 / <100 ms, partial-edit fit frame 121.8 / <33 ms and C1
gradient gap 505 / <100 ms, all at 100 MP. This smaller allocation candidate is
also rejected; the original framebuffer allocation and view mapping are restored
exactly. Only the independent canvas-dimension setter fix and its separate
regression remain under verification. The previous full functional result
belongs to the archived 256-pixel candidate, not this new source. Production 2328 remains the verified
portable; no newer native or full-acceptance result is claimed.


The retained canvas-dimension fix leaves the original framebuffer allocation,
view mapping, engine and WASM unchanged. All 287 units, three TypeScript
checks, fixed assets and 19 exact rendering cases pass. The complete current
functional suite passes all 197 cases with 29 original opt-in skips (7.0 minutes).
Six selected original performance cases finish two passed / four failed
(2.6 minutes), including a ready assertion before any new Levels body values.
These functional passes do not establish full performance or new native-package
acceptance. GPU trace thread times identify a 410.461 ms client wait with
0.828 ms thread CPU and a 401.840 ms service wait with 1.228 ms thread CPU;
the particular synchronization/driver cause is still unidentified. Mac alpha
returns/gestures and current-package clipboard acceptance remain open.


Production 0244 packages dbb4d81 after all sequential source/timing/function
checks, builds in 8m 23s and passes archive CRC, EXE and WASM identity. All
eight native UI groups, four Mac reads and four atomic saves pass; font size
38 and shape width 150 / Stroke 7 / Shadow 14 persist. All three opened PNGs
are byte-identical to production 2328, preserving the Mac-created discrepancy
(13,065 pixels, maximum channel delta 51). Clipboard stops before any write
at the original OleGetClipboard snapshot with CLIPBRD_E_CANT_OPEN.
The hash-bound interactive handoff runs all 29 original performance cases
then the unchanged guarded clipboard protocol and strict RGBA/text/restoration
comparison. Its final read-only preflight passes; its actual interactive run
is still required. The strict comparator passes the historical 2139 corpus
and rejects a one-channel error in an isolated copy. None of these reader
checks is current-package clipboard acceptance. Mac alpha PNGs/RESULT and
gesture confirmation remain missing. The full objective and original budgets
remain unchanged; completion requires these external evidence gates.


Received evidence update, 2026-10-04: the hash-bound production-0244 interactive
run passes all 29 original timing cases. Its clipboard text paste fails; a later
unchanged original protocol passes all six groups and the independent exact
RGBA/UTF-16/style/restoration reader, with intermittent failures retained. Mac
1.4.5 returns all four alpha PNGs and gesture PASS records, but the saved
rounded rectangle still contains 5/12 rather than reported 7/14; the matching
saved project is requested. A measured eight-phase enlargement prototype
reduces independent grid alpha error from 37 to 1 and passes six targeted native
tests. Full regression remains in progress. Mac-created straight-RGBA export
is still not exact (maximum difference 51), so Phase 5 is not complete. See the
received-evidence section of phase5-acceptance-2026-10-03.md for current results.


Exact-byte source checkpoint, 2026-10-04: all eight independent Mac PNGs now
match complete release-WASM exports exactly, including Mac-created, with saved
transforms unchanged. The four alpha probes also match CPU/GPU bytes exactly.
Final native workspace passes 623 cases; the newly added half-column case
passes separately with the exact grid and five existing pixel-copy cases.
All 287 units, three TypeScript checks, nineteen render cases and all 198
functional browser cases pass (29 original opt-in skips). Final original
performance is 23 passed / six failed, 10.6 minutes, one worker and zero retries;
no failure is waived. A prior-version fixed-assets control and new packaged
native acceptance are in progress. Mac gesture PASS records are received, but
the actually saved blue rectangle still reports 5/12 rather than 7/14; the
correct applied-and-reopened project is still required. Phase 5 full acceptance
remains open. The acceptance record retains every historical failure.


Production 1256 packages product commit b452534 after completed sequential
checks. All eight native UI groups, nine native Mac reads/saves, eight exact
full-PNG comparisons and six original clipboard protocol groups plus strict
RGBA/UTF-16/style/restoration pass. The 30 received original files retain their
receipt hashes. The new verifier passes read-only preflight for 25 SHA-bound
inputs. Current performance remains 23/29 passed with six failures; the
preceding-version six-case control is one pass / five failures. New interactive
performance evidence and the Mac-applied 7/14 saved project are requested.
These native/pixel successes do not complete Phase 5 or waive timing failures.


Afternoon receipt: the actual Mac-applied 7/14 project closes the saved-effect
artifact mismatch and exports identically through production 1256. Its human
interactive script is 26/29 timing passes; typed W and C1 exceed budget, and
the 24 MP brush page fixture times out before its test body. Clipboard aborts
at read-only OleGetClipboard. A shader-isolation candidate passes 287 units,
three type checks and four selected original timing cases (typed W 14/18 ms,
100 MP C1 gap 60 ms); complete regression and nine Mac comparisons are running.
No source/WASM pixel algorithm, original assertion or performance budget changes.


Separated shader checkpoint: all 287 units, three type checks and 198 complete
functional cases pass; 29 opt-in performance skips remain unchanged. All nine
independent Mac full-PNG exports are exact, including applied 7/14. Four selected
original timing cases pass, and typed W passes at 17/27 ms in the full suite.
The complete unchanged timing run is 20 passed / nine failed (10.0 minutes);
no failure is waived. Visible-window comparison and current-source native
package verification remain separate pending evidence gates.

The visible-window comparison remains one pass / one failure; no performance
failure is waived. A minimal WebGL control without application/WASM also
reproduces a 446.7 ms GPU wait. The specific driver/scheduling cause remains
unproven, and WPR GPU/CPU capture cannot enable profiling policy (0xc5585011).
Existing display services and system settings remain unchanged.


Production 1527 packages source 1942966, with archive/EXE/WASM identity and all
eight native UI groups verified. The initial Mac Save As times out; a separate
fresh-profile diagnostic completes all nine projects, 11 reads / 10 atomic
saves and exact full-PNG comparison, retaining the failed original run. The
original clipboard protocol safely aborts before any mutation at OleGetClipboard
0x800401D0; read-only Win32 OpenClipboard also returns error 5 with its sequence
unchanged. All 30 old and 11 new receipt hashes match. Current full performance
is still 20/29 passed; no budget or failure is waived. Administrator GPU/CPU
trace and current-package clipboard evidence are requested through one verified
local script. Full Phase 5 remains incomplete.

### System-trace receipt and ordinary interactive handoff, 2026-10-04

The received administrator GPU/CPU ETL is complete: 46,211,454 processed
events, zero reported lost events. All 26 frozen inputs match. Original F1
passes with a maximum 12 ms result frame; this does not close the complete
29-case performance gate. The original clipboard snapshot and restoration
pass, but the elevated package has no debugging endpoint within 45 seconds
and the six-group protocol never starts. A non-elevated same-package endpoint
control passes without clipboard access. Earlier clipboard failures remain
recorded separately.

The prepared local `verify-phase5-interactive-1527.ps1` passes CheckOnly and
requires ordinary PowerShell. It preserves all original assertions, budgets,
one worker and zero retries; default execution collects the full 29 timing
cases and two independent guarded native clipboard protocols plus strict
image/text/style/restoration comparisons. No further GPU recording is needed
for that run. Full acceptance remains open pending this evidence and resolution
of the recorded intermittent failures.

### Ordinary interactive 1527 receipt, 2026-10-04

The received ordinary run passes all 29 original timing cases in 8.8 minutes
with 26 unchanged frozen inputs. First clipboard passes six native groups,
four independently rechecked exact RGBA comparisons, two editable UTF-16
records, matching style and restoration. The independent session fails
Ctrl+V after Cut: two layers remain instead of three within the original
five-second timeout. Both original clipboard format sets restore. No retry
or assertion change was introduced. The cause is unproven; full acceptance
remains incomplete alongside the retained initial native Save As timeout.

The new local `verify-phase5-clipboard-observed-1527.ps1` runs clipboard
only and logs shortcut/IPC/state information plus a failure screenshot.
Its original body reconstructs byte for byte after removing observations;
CheckOnly and a read-only real-package logger self-check pass. It needs
ordinary interactive PowerShell, with no further timing or GPU recording
required for this diagnostic step.

### Corrected observer and isolated native instances, 2026-10-04 evening

The latest observed first clipboard session passes six native groups and
the independently rerun strict reader. Its second session fails before
clipboard activity because the observer mishandles fetch(URL) during WASM
startup. The observer defect is fixed; it does not establish a cause for the
earlier actual Paste-after-Cut failure. Both original format sets restore.
Product source and all 26 original frozen inputs remain unchanged.

The corrected observer passes early initialization across two reloads, five
fetch input forms, complete production-UI simulation and a controlled
clipboard-read refusal with original layer assertion/error capture. These
simulations access no system clipboard. Each revised session uses an
independent port, with native parent PID and fresh-profile ownership checks,
within the original 45-second readiness deadline. Two helper hashes are
verified as well as the 26 original inputs. The original action/assertion
body is exact after normalizing only observations and the connection adapter.

A further independently owned real-native Mac run passes 11 reads, 10
atomic saves and nine exact full-PNG comparisons; prior Save As failure
remains recorded without an unproven fix claim. The corrected clipboard-only
entry passes CheckOnly and is ready for the ordinary interactive desktop.
Full Phase 5 acceptance remains incomplete; the already received 29/29
original timing run does not need repeating for this diagnostic step.


### Snapshot restoration failure, 2026-10-04 18:23

The corrected observer receipt confirms all 26 original frozen inputs and
both observed-helper hashes. First native protocol and strict reader pass.
The independent session finishes the original UI operations and saves; four
exact RGBA comparisons and two editable text records independently pass.
Its final original snapshot restoration fails with 0x800401D0, as does the
existing finally fallback. The helper exits with the snapshot still unrestored
and no recoverable payload backup. This protocol remains failed.

All four native clipboard reads and three writes succeed before the helper
restoration failure; the final application has five layers, no error and is
idle. Neither this pass of the UI steps nor diagnostic pixel checks resolves
the earlier intermittent Paste-after-Cut failure. No retry, assertion or budget
is changed. A bounded ownership metadata probe is verified locally without
opening or reading/writing the clipboard; it has not captured the historical
failure. No further blind interactive rerun is requested. Full Phase 5 remains
incomplete; the 29/29 performance result does not need repeating.
