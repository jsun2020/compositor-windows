# Phase 7: Camera Raw, PSD/PSB and camera RAW

Authorized by the user on 2026-10-06. Development starts on `codex/phase7`,
from documentation commit `f463b6264e62baf11bc55e1801acf6022ddc5356`.
Phase 6 PR #8 and Phase 7 PR #9 were merged into `main` on 2026-10-06,
at `1de001b` and `9aafb14` respectively. Published 0.8.0 remains unchanged;
merging the implementation does not complete the acceptance gates below.

## Oracle and scope

Use Compositor v1.4.5, `086f1631573ccb2b57644e53b52bf1488fc976aa`.
An immutable local archive is retained under
`build-artifacts/phase7-mac-1.4.5-source`. Read the actual sources, not the
older 1.2.10 working tree. CameraRaw.swift, CameraRawColor.swift,
CameraRawDetailOptics.swift and CameraRawGeometryCalibration.swift define
the destructive filter. RawImporter.swift defines a separate pre-import
develop flow. IO/PSD and its adjustment/vector/text tests define import.

Camera Raw includes Light, Color, Curve, Mixer/Point Color, Color Grading,
Effects, Detail, Optics, Geometry and Calibration. Its defaults are an exact
identity, alpha is preserved, preview reads the original image, Cancel restores
pixels/history and Apply commits one edit within existing history limits.
Geometry precedes calibration, then basic Light/Color, Curve/Mixer/Grading,
Effects/Grain, Optics and Detail. Grain keeps a stable seed during preview.
Camera Raw is a destructive filter, not a saved adjustment-layer kind.

PSD and PSB import must retain supported groups, visibility, opacity, masks,
blend modes, clipping, adjustments, editable text and supported shape records.
Unsupported conversions must be reported, as on the Mac; no silent flattening.
The oracle accepts 8-bit RGB and raw/PackBits channels and can use a merged
image for background-only documents. Inspect each unsupported case explicitly.
Size, offsets, record lengths and decompressed data must be checked before
allocation; failed import must preserve the current document.

Camera RAW is decoded before conversion to 8-bit sRGB layers. Offer as-shot
reset, exposure and white balance with bounded preview, Cancel and full-size
Import. Apple CIRAWFilter is unavailable on Windows: select and pin a native
decoder, document its license/camera coverage and retain the actual sensor
processing boundary. Do not import the embedded JPEG as if it were a RAW
develop. Apple boost/tone-curve or RAW byte equality cannot be assumed across
different decoders.

## Delivery gates

- [x] Original scope and v1.4.5 source archive established.
- [x] Camera Raw settings and original pixel kernels, native/WASM source checks.
- [x] Geometry, curves and auto/sample white balance source behavior.
- [x] Worker preview, stale-result/cancel/selection/history integration.
- [x] Camera Raw controls, resets, group bypass and preview diagnostics.
- [x] Bounded PSD/PSB reader and compression/channel validation.
- [x] Groups/masks/blend/clipping and supported editable conversion records.
- [x] Import conversion report and native/browser file routing.
- [x] Pinned RAW decoder, development controls and portable dependency receipts.
- [x] Deterministic synthetic PSD/PSB/DNG probes and malformed-input checks.
- [x] Supplied real PSD and its Mac project/export counterpart verified.
- [x] Actual public camera RAW sensor development, bounded preview and full import.
- [x] Complete original source gates and exact-source hosted CI.
- [x] Complete original hardware performance gates; no changed budgets/retries.
- [x] Rebuilt production portable with no test API, native real-file checks.
- [x] Actual Mac 1.4.5 counterpart runs and returned evidence independently received.
- [x] Real Windows native pickers and close/save/reopen via user-authorized scripts.
- [ ] Remaining counterpart pixel discrepancies.
- [x] Implementation documentation, commit/push, reviewed CI and main integration.
- [ ] Final acceptance receipt after all remaining gates pass.

Each checkpoint must state what passed and what is still open. Do not mark
Phase 7 complete from source tests alone. Preserve user files, old failures,
sampling probes and `.workbuddy/`; commit only reviewed source files.

## Source checkpoint before final portable verification

The native workspace initially passed 687 cases with ten original ignores.
Subsequent PSD checks passed all eleven cases, including legacy Mac Roman names,
Curves/colorized Hue and explicit skipping of an unsupported record without a
cached raster. Thirteen preview cases then passed with the new Camera Raw
layer-local sampler. White balance/defringe read the original; Point Color reads
the prepared layer with the Mac's HSV saturation formula. Covering layers and
hide-all masks do not change the sample. Exact-source hosted checks remain open.

Release WASM build 4 and the three TypeScript checks passed. The complete local
functional run passed 258 cases with 33 unchanged performance opt-ins skipped;
that run preceded the final sampler/report fixes. All fifteen current Phase 7
UI cases subsequently passed, including grade-wheel dragging, Defringe, RAW
cancel/reset/full-failure and the visible Photoshop conversion report. Nine
release packaging regressions and all 295 current unit tests passed.
The reader now counts only records that own RGB storage toward the image budget;
a pixel-less Levels record imports with zero remaining image budget. All twelve
PSD cases pass after this correction. The limits themselves were not changed.

The supplied folder contained one PSD and no camera RAW. Its 2364x1330 RGB8
image imported with two named pixel layers and no conversions. Its original
hash remains unchanged and is recorded only in the ignored local receipts.
The original and its private test copies stay ignored. Real-camera decoding is
still an open gate; a synthetic DNG does not substitute for that evidence.

The first development portable, marker `COMPOSITOR_BUILD_0.8.0_20261006-1426`,
is retained. Its supplemental production protocol passed native RAW IPC and
dialog control checks, Camera Raw preview/Cancel/Undo/Redo/diagnostic checks and
three Photoshop inputs. It failed the conversion-notice observation; an initial
PowerShell stderr handling error truncated its first log. Both failures remain
under `build-artifacts/phase7-native-1426-*`. The wrapper now retains the real
exit and full diagnostic files. Conversion notices now use an application-owned
report instead of a host script alert. The final rebuilt portable must pass the
same warning-content requirement and the remaining real PSD route.

The private cross-platform probe bundle is
`build-artifacts/phase7-acceptance-20261006`, with `MAC-STEPS.md`, `RESULT.txt`
and supplementary `WINDOWS-STEPS.md`. Mac returns, real RAW, original complete
performance gates, final production runtime/OS input and publication checks
remain open. Stable version 0.8.0 has not been changed or republished.

## Production and original performance checkpoint

Commit `26a2b9d30de17b16c08b0902e8aa34097f7515b1` passed hosted run
`37426990777`: 692 native tests, ten original ignores, 295 unit tests and
263 functional cases, with the 33 original hardware performance opt-ins skipped.
Release WASM, TypeScript and packaging regressions also passed. Hosted CI does
not run the local hardware budgets or the actual Windows file picker.

The clean-source `COMPOSITOR_BUILD_0.8.0_20261006-1500` portable passed all eleven
ZIP entry/CRC checks, bundled helper sensor development and the complete
supplemental production protocol: RAW IPC/dialog, Camera Raw preview/cancel/
history/diagnostics and five PSD/PSB imports. Original private samples and the
frozen probe inputs were unchanged. Visual inspection then found that sheets
mounted outside the application root inherited black serif text. Explicit sheet
foreground/font styling fixes this; the native protocol now requires readable
contrast and the application font in RAW and conversion dialogs. A fresh package
must pass those additional checks before delivery.

One unchanged complete original hardware run finished with **25 passes and four
failures**, one worker and zero retries. Retained evidence is under
`build-artifacts/phase7-performance-original-29-1`; no budget was waived:

| Original assertion | Observed | Original limit |
| --- | --- | --- |
| F1 100 MP Levels: frame gap while the worker edits | 751 ms | <100 ms |
| 24 MP Levels: result installation CPU | 169 ms | <150 ms |
| 24 MP gradient at 1:1: settled preview | 169 ms | <150 ms |
| 100 MP grown-mask fill: frame after the result | 445 ms | <400 ms |

Cause is not established; neither a source regression nor environment scheduling
has been proved. This performance gate remains open. The independent original
four hardware Liquify/Smudge cases all passed; they do not replace the 29 cases.

Computer Use selected the owned 1500 package window but activation and one
refreshed recovery failed with `failed to activate captured window`. No OS input
was issued, the user's older application was untouched, and the owned process
was stopped. Do not infer an unlocked/locked desktop or claim OS input acceptance
from the successful CDP protocol. The Windows manual checklist, Mac returns and
real camera RAW remain required. Phase 7 is still incomplete.

## PSD counterpart return: file order correction

The supplied Mac and Windows project returns exposed an importer defect: layer
pixels, placement, visibility and blend settings matched, but Windows reversed
the file records. An opaque background then covered the Screen overlay. The Mac
1.4.5 reader consumes records bottom-first, including type-3 group dividers
before children and their type-1/2 folder. The Windows importer now preserves
that order. Synthetic PSD/PSB generators and browser fixtures use the same order;
old generated probes and failed receipts remain retained, not rewritten.

A new regression uses independently colored opaque background and Screen layers
and checks composited pixels plus save/reopen for PSD/PSB, raw and PackBits. It
failed against the prior reader, then passed with the fix. All 13 reader tests
passed, including group/clipping and adjustment checks. The corrected native
engine export of the supplied PSD is pixel-exact with its Mac-returned PNG.
This proves this sample's composition; other Photoshop features and Phase 7
gates remain separate. A new production portable must validate the correction
before it replaces the 1531 development package. Private sample content and
fingerprints stay in ignored local evidence.

## Unsaved project close choices

The user confirmed the corrected PSD layer composition in Windows. Closing its
unsaved project then exposed a missing action: the old two-way confirmation
offered saving or keeping the project open, with no way to discard and close.
The application now explicitly offers **Save and Close**, **Don't Save and
Close**, and **Cancel Close**. Cancelling the native Save As picker continues to
keep the project open; a failed write reports its error and retains changes.
Cancel Close and Escape preserve pending edits. Saving resolves a pending
transform/gradient before serializing, while an open adjustment must first be
applied or cancelled. Closing a background tab preserves the active panel.

All 16 related functional cases passed with one worker and zero retries,
covering all choices, save cancellation/failure, save/reopen, pending transform,
background-tab/panel preservation, duplicate close requests, keyboard focus,
and last-document canvas clearing with both renderers. The supplemental native
protocol also checks the three-way dialog, save-picker cancellation, discard
canvas clearing and actual native package save/reopen. Its picker response is
a fixture; physical OS picker operation remains distinct.

The preceding clean-source `20261006-1759` portable passed its production
protocol and four supplied-PSD/Mac-project export comparisons pixel-for-pixel.
Its complete original frozen 29 performance cases passed 25 and failed four:
F1 frame after result 165 ms (<100), Add Mask frame 472 ms (<150), Levels worker
commit frame gap 383 ms (<100), and grown-mask gradient frame 386 ms (<150).
These failures remain retained and unresolved. This close fix does not waive
them or complete the outstanding Phase 7 performance and cross-platform gates.

## Main integration and remaining acceptance

The close choices, corrected PSD order and Phase 7 implementation are in main
at merge `9aafb14f8ca8b3cf13fb30f94f2a0fb843120765`. Its tracked file tree is
identical to `cedba2a388cd880fa3a1420110334865d64b40b8`, whose hosted run
`37459575896` passed 693 native cases with ten original ignores, 295 unit cases
and 269 functional cases with the 33 original hardware opt-ins skipped. Release
WASM, the three TypeScript checks, pinned sensor decoder checks and nine release
packaging regressions also passed. The merge's own hosted run `37463621544`
also completed successfully on the exact `9aafb14` merge.

The clean-source `COMPOSITOR_BUILD_0.8.0_20261006-1955` portable passed the
complete supplemental production protocol, including three close choices,
cancelled save, actual canvas clearing and native package save/reopen. The
production test API was absent. Native RAW IPC, Camera Raw preview/cancel/
history/diagnostics and all five Photoshop imports passed as well. Only the
save-picker HTTP response was supplied; actual Windows picker/mouse operation
still requires the human checklist. Private originals and frozen probes remained
unchanged. No new version or release was published: the main release workflow
validated the existing immutable 0.8.0 identity and skipped package/publication.

The same 1955 production package also passed the actual public Canon EOS 40D
CR2 protocol: native sensor development at 3908x2602 for As Shot and Exposure +1,
a 1954x1301 bounded preview, preview/Cancel/reset/full-dialog import, and native
save/reopen of both developments. Each before/reopened export was byte-exact
with the corresponding native decoder PNG; Exposure +1 increased mean RGB
brightness. The source RAW remained unchanged. This extends the earlier 1531
sensor evidence to the current package and proves native project persistence;
the save-picker response fixture and synthetic Tauri drop still do not prove
actual OS mouse/file-picker operation or Mac CIRAWFilter behavior.

The four failing original performance scenarios were copied into an isolated
diagnostic harness. Only their fixture import changed; their bodies, assertions,
budgets, single worker and zero retries were preserved. GL calls and browser
timeline measurements add overhead, so the diagnostic is not acceptance. Three
cases passed; F1 failed with a 145 ms frame gap during 100 MP Levels (<100 ms).
That interval had no similarly long main-thread task. Large-mask diagnostic
frames also spent time uploading R8 textures and waiting for GPU readback; those
observations do not establish the cause of the four prior complete-run failures.
No speculative source fix, budget waiver or repeat-until-green run was added.
The unchanged complete 29-case result remains 25 pass / four fail.

The corrected private counterpart bundle is
`build-artifacts/phase7-acceptance-20261006-order-fixed`. Its initial result fields
were blank; the returned Mac evidence is independently checked below. The checklist is
`build-artifacts/phase7-remaining-checks-20261006.md`: four Camera Raw recipes,
synthetic PSD/PSB structure and editable records, actual camera RAW on Mac 1.4.5,
and actual Windows close/save-picker gestures, now assigned to Codex while the
user performs only the Mac checks. The supplied real PSD counterpart
has already passed and need not be repeated. Phase 7 remains incomplete until
the original hardware budgets and the remaining human returns pass.

## Current-main Windows verification, 2026-10-06

The complete original 29-case hardware run on `0a73281d49711b2134926c72bcc9f86dc3703f32`
passed 18 and failed 11, with one worker and zero retries. The assertions and
budgets remain equal to the Phase 6 originals; all 230 frozen source, asset,
package and helper inputs verified before and after the run. Runtime sources
match the clean 1955 package source. The only tracked overlay at that run was
the requested Chinese Mac instructions, recorded separately. The new result is
retained in `build-artifacts/phase7-windows-current-main-0a73281-20261006` and
does not replace the preceding 25/4 evidence. Nine cases exceeded their original
timing bounds; two were still showing Loading engine at the original five-second
readiness deadline. These failures do not establish a single CPU, GPU, memory or
desktop-lock cause, and Phase 7 performance acceptance remains incomplete.

Codex selected the actual 1955 Windows application and read its version marker
and empty workspace through Computer Use accessibility. Native screen capture
failed twice; an accessibility-only attempt then rejected input with
`coordinate input geometry is unavailable`. No input action succeeded. The
physical close/save-picker checks remain unverified; a fixture response is still
distinct from an actual OS gesture. The local evidence is retained in
`build-artifacts/phase7-windows-ui-20261006-221842`. The user need not perform the
Windows checklist; Codex will continue it when desktop input is available.

## Mac return received and independently checked

The user completed the Mac checks and returned 339 files (292,720,253 bytes) in
the corrected bundle's `Mac-return`. Thirteen new project/PNG pairs and the
preexisting `00-source` project are complete. All 64 bundle receipt inputs and
the 31 inputs recorded on Mac independently match their hashes. Light/Color/
Curve, Mixer/Grading and Effects/Detail/Optics outputs are pixel-exact with the
frozen Windows expected PNGs. Geometry/Calibration differs at 5,317 pixels;
maximum alpha difference is 127, with low-alpha boundary samples showing a
hard/clamped Windows edge against a partially covered Mac edge. No new tolerance
has been introduced to call that discrepancy passed.

The saved Mac editable text is `Editable ABC X`, its rectangle is 240x150 and
Levels output white is 200 (255 in the Cancel/original control). Native Windows
engine open, disk save and disk reopen succeeded for all 14 projects with exact
before/after exports. Twelve native exports equal the corresponding Mac PNG;
the two grouped PSD/PSB cases have a one-level blue-channel difference over
16,800 pixels, with matching names, hierarchy, masks, opacity and clipping.
Original PSD text appearance also differs across import paths; the returned
Mac text raster itself is preserved exactly by native Windows reopen. These
are separate observations, not evidence that all counterpart pixels passed.

The Mac log records successful Preview/Cancel/Undo/Redo/reopen checks and other
controls. Its returned 24 visible-canvas crop comparisons were independently
recomputed. Point Color with Visualize Range enabled crashed Mac Compositor
1.4.5 build 40 on OK: EXC_BREAKPOINT/SIGTRAP, with SwiftUI ForEachChild frames.
The actual `.ips` report is retained; no root cause is inferred and no successful
Point Color saved counterpart is claimed. This does not by itself prove a
Windows defect. The synthetic DNG rejection is explicitly recorded. The actual
Canon CR2 Mac outputs are 3888x2592, while Windows LibRaw develops 3908x2602;
the protocol does not require different decoders' dimensions/WB/RGB to match.
The original returns remain unchanged. All private files, hashes and detailed
pixel receipts stay in `build-artifacts/phase7-mac-return-audit-20261006`.

## Selection coverage optimization verification

The current-main complete run's Add Mask step exceeded its unchanged 100 MP
budget. Profiling an unmodified 64,032,004-pixel ellipse independently measured
194-224 ms in native release rasterization. Filling constant coverage spans
between touched edge ranges measured 20-26 ms with the same benchmark; all
64,032,004 output bytes and 1,206 additional aliased/antialiased cases matched
the retained pre-change bytes exactly. Nonzero winding and the float addition
order at touched cells remain unchanged. An independent rectangle-area test
covers long fractional spans, reversed holes, overlapping edge ranges, canvas
clipping and row clearing. All 677 engine tests passed, with ten original ignores.
This native diagnostic improvement does not complete release WASM, production
package or original hardware acceptance. Their results remain separate.

## Geometry boundary sampling correction

The returned Mac geometry PNG exposed hard transparent transitions at Windows
source boundaries. A 0.6-pixel translation regression first failed on the
unmodified sampler (edge alpha 0, expected 102). Geometry now interpolates clear
black beyond the source extent, samples straight colour/alpha independently,
and stores premultiplied pixels. The shared ordinary-layer sampler is unchanged.

An intermediate premultiplied interpolation candidate reduced alpha error but
increased boundary colour error; its probes and comparison are retained. The
straight-colour candidate reduced maximum alpha difference from 127 to 1 and
mean RGB difference from about 0.14 to 0.02 levels. Light/Color/Curve,
Mixer/Grading and Effects/Detail/Optics remain pixel-exact. Geometry still has
4,743 differing pixels: maximum straight RGB difference 23 at low alpha,
maximum premultiplied RGB difference 4/5/4, and maximum alpha difference 1.
No tolerance or exact-match claim was added. Current-source probes are in
`build-artifacts/phase7-geometry-straight-probes-20261006`; independent receipts,
including both intermediate candidates, are in the Mac return audit directory.

The supplemental Windows Point Color functional case explicitly checks
Visualize Range on OK, one history entry, Undo/Redo, and project save/reopen.
It uses the browser mock file bridge; production/native picker evidence remains
separate. The current-source complete engine suite passed 678 cases with ten
original ignores, release WASM and the isolated browser build completed, and
all 27 related functional cases passed. The current source is a correction
candidate; the existing 1955 portable is unchanged.

## Correction candidate verification, 2026-10-06/07

The isolated fixed browser build is retained in
`build-artifacts/phase7-selection-spans-browser-20261006`, marked
`COMPOSITOR_DIAGNOSTIC_PHASE7_GEOMETRY_SELECTION_0a73281`. Its WASM exactly matches
the newly compiled release WASM. All 384 source, asset and helper inputs verified
before and after both runs. All original performance assertions and budgets are
equal to the Phase 6 originals. Both runs used one worker and zero retries.

The complete original 29 performance cases passed 16 and failed 13. The 100 MP
Add Mask operations improved to 149/156 ms against the original <1000 ms step
budget, but the worst Hide Selection frame was 579 ms against <400 ms. Other
failures include result/frame installation, partial uploads, effects frames,
gradient/mask frames, an eyedropper sample at 16 ms against strict <16 ms, and
24/100 MP brush responsiveness at 108.7/113.2 ms against <100 ms. Full results and
failure indexes are retained. The 18/11 and earlier 25/4 runs remain unchanged;
none is replaced or waived by the optimization. This candidate does not pass
complete performance acceptance or prove a single cause for the timing failures.

All 339 Mac return files independently reverified unchanged after the current
source tests. Standard `sky.launch_app` of the 1955 application also failed with
`failed to activate captured window`. A refreshed recovery failed to retain its
state because the evidence recorder referenced an uninitialized binding; that
recorder error is explicitly retained, and no successful recovery is claimed.
No document input succeeded and actual OS gestures remain unverified. Desktop
lock is not established. Current correction portable/hosted CI receipts remain
separate from the preceding 1955 package evidence, and Phase 7 is incomplete.

## Production correction and main integration, 2026-10-07

Source `98fa5a47bef3bfc2e15108391e728100708396bc` passed hosted run `37493681473`:
695 native workspace cases, 295 unit cases in 43 files and 270 functional cases.
Ten original native ignores and 33 hardware opt-ins remain distinct from those
passing counts. The same tested source was fast-forwarded into `main`; hosted
functional CI does not replace the retained 16/13 hardware result.

The clean 0010 production portable uses that source and the corrected release
WASM. Its 11 ZIP entries passed CRC, length, hash, native binary, dependency and
license checks. The original production protocol passed RAW, Camera Raw, five
Photoshop imports, readable dialogs and close/cancel/discard/native save/reopen.
The actual Canon CR2 separately passed As Shot, Exposure +1, bounded preview,
Cancel/Reset/full import and native disk persistence. Only picker responses are
supplied by these protocols; actual OS input remains unverified.

A supplemental production Point Color check sampled the blue stripe (hue
230.526315789), shifted saturation -40, enabled Visualize Range, then verified
OK returns to the normal grade, one Undo/Redo and native disk save/reopen. It
passed with the production test API absent. Its initial helper mistakenly opened
the saved generated probe directly, so Save and Close wrote its raster back to
that probe instead of invoking the prepared Save As response. That failed run
and modified raster are retained. The single changed generated PNG was restored
from the byte-exact original source PNG; all 64 frozen inputs and all 339 Mac
return files verified against their original hashes afterward. The corrected
helper opens only a private copied project and saves on that copied path. This
passing CDP/native protocol neither substitutes for OS input nor claims a
successful Mac Point Color counterpart.

Supplemental frame attribution recorded small partial updates with only 1.7-4.3
ms of render submission but 318-390 ms inside synchronized `readPixels`. A separate
detailed Chromium trace recorded four long WebGL GPU tasks (368-504 ms) without
overlapping D3DCompile events. This narrows those stalls to GPU command completion;
it does not establish one driver, shader, desktop lock, CPU or external-process
cause, eliminate all thirteen hardware failures, or change an assertion. The
first diagnostic static server rejected its root path; that failure is retained
and a corrected server ran in a separate directory.

The existing 0.8.0 release remains immutable. Main release run `37497059404`
validated metadata and skipped packaging/publication because the version already
exists. Current local evidence, return integrity, the initial supplemental helper
failure and its restoration receipt remain under ignored build-artifacts paths.
The Mac checklist need not be repeated. Hardware performance, counterpart pixel
discrepancies and actual Windows OS gestures still prevent final acceptance.

## User-authorized Windows script fallback, 2026-10-07

The user explicitly authorized scripts when Computer Use failed, accepting that
approach for the Windows checks. A new production 0010 run used CDP for application
controls and PID/EXE-scoped Win32 control messages for actual native dialogs. It
did not mock picker responses, synthesize drag/drop events or use a production
test API. All fourteen Open, Save As and folder-selection dialogs completed.

Five PSD/PSB inputs imported through real Open dialogs, with the text/unsupported
adjustment conversion notices retained. Cancel Close, Escape and actual Save As
Cancel kept the dirty document and exact viewport pixels. Discarding the final
document cleared both layers and actual canvas. Exposure +1, one Undo/Redo,
native disk save before closing and folder-picker reopen preserved the changed
PSD viewport exactly. Actual Canon CR2 Cancel, Reset to As Shot, full import,
native save/close and folder-picker reopen also passed with exact viewport pixels.
Physical mouse input is not claimed or substituted into the receipt.

The independent post-run audit verified all 64 frozen probe inputs, all 339 Mac
return files and six imported copies/originals against their receipts. Three
initial helper failures remain retained: waiting for busy to clear before closing
the conversion report, the modern Save As edit's different control structure,
and the standalone Folder edit. The corrected complete script passed in a fresh
directory. Evidence and executable helpers are under
`build-artifacts/phase7-script-dialogs-20261007`; the final run is
`complete-folder-dialog-fixed`. No application source changed for this fallback.

After the external display was disabled, the byte-identical retained frame
diagnostic with unchanged browser assets measured a maximum 17.5 ms frame and
11.8 ms readPixels wait; the preceding hundreds-of-milliseconds waits were not
observed in that run. It is supplemental evidence and does not establish that
dual screens caused the earlier failures.

That concrete environment change and measurement justified one fresh complete
original 29-case run, using the same immutable release WASM/browser assets and
clean source `af6b0ce13b254472da4f435b32b85ba7ebbe7531` (runtime unchanged from
98fa5a4). All 380 frozen source/assets/helper inputs verified before and after;
all original assertions/budgets match f463b62. With one worker and zero retries,
28 passed and one failed. The remaining strict assertion is 100 MP blank-layer
fill result installation: 454 ms, required <450 ms. It remains a failure; no
retry or budget change was added. The 25/4, 18/11 and 16/13 runs are preserved.
The complete result is in `phase7-single-screen-performance-20261007/performance`.
Actual Windows scripted dialog acceptance is complete; the remaining performance
assertion and counterpart pixels still prevent declaring all Phase 7 complete.

## Uniform-fill correction and current Windows acceptance, 2026-10-07

Source `0a85e8976ca8d260a0a3269841d3d63c1646ed18` completes the current Windows
checkpoint. A worker Fill result is encoded as four RGBA bytes only after every
actual result pixel has been checked equal. The client expands it cooperatively
into the existing staged allocation, retaining the original dimensions, display
halving, stamp, cancellation and history checks. Partial/nonuniform fills keep
their ordinary payload. The worker also preserves the original exact-size spare
as output capacity, without treating its untouched contents as input pixels.

Hosted run [37560460816](https://github.com/jsun2020/compositor-windows/actions/runs/37560460816)
passed 698 native workspace cases, 298 unit cases in 43 files and 272 functional
cases. Ten native ignores and 33 hardware opt-ins remain separate. Release WASM,
three type checks, the actual sensor helper and packaging regressions passed.
The preceding `3bbf010` hosted run failed the original spare-buffer assertion;
that failure is retained and the assertion was unchanged for the correction.
The two new actual-worker cases verify every filled/selected/unselected pixel
and one Undo/Redo. A local integration run also retained a first-test engine
startup timeout; an earlier static-preview helper could not load the original
GPU tests' source modules and was stopped. Neither run is claimed as a complete
functional pass or substituted for the successful full hosted suite.

The clean production portable `COMPOSITOR_BUILD_0.8.0_20261007-1007` uses this
source and release WASM SHA256
`2261eb46d52786265a24bd2c9ee692d43aa5fe7810029bff10bec077f6c273e5`.
All eleven ZIP entries passed CRC, size, hash, native component, dependency and
license checks. A production run with no test API completed sixteen real Open,
Save As and folder-selection dialogs, the original Photoshop conversion notices,
close/cancel/discard/clear, Exposure +1 with one Undo/Redo, and actual Canon CR2
Cancel/As Shot/native save/reopen. A 2560 x 2560 keyboard Fill also passed one
Undo/Redo and actual native save/reopen; all 6,553,600 stored RGBA pixels were
exactly opaque black.

The new native helpers initially failed a Save As cancellation and then an Open
button action. Both control-message failures remain retained. The corrected
helper invokes the observed PID/EXE-scoped native button's UI Automation pattern
once, while the file-name Edit uses Win32 messages. All original process, path,
button, closure and application-state assertions remain. The reused protocol's
generic Win32 scope label is clarified by each dialog's `actionMessage` and the
final receipt. Application controls use CDP; physical mouse input is not claimed.
The final audit verified the four helpers, all portable entries, 64 frozen probe
inputs, all 339 original Mac return files and six imported originals/copies.
All 17,178,711 decoded RAW and edited-PSD layer pixels also matched the preceding
accepted native checkpoint.

After the owned build and native process had finished, a fresh complete original
29-case performance run passed 29, failed zero and skipped zero in 492.8 seconds.
All 381 frozen source/assets/helper inputs verified before and after. The five
original specification files match `f463b62`; workers remained one, retries zero,
and every assertion/budget was unchanged. Both environment snapshots measured
one active logical display and zero compilers. The browser worker bytes match
the production worker; the timing run uses frozen browser assets and release
WASM, while production native acceptance remains a separate receipt. The 100 MP
blank-layer Fill install took 244 ms against the original strict <450 ms budget.
Earlier 28/1, 27/2, 22/7 and other failed complete runs remain unchanged. This
checkpoint does not establish a universal cause or cure for intermittent stalls.

Evidence is retained under `build-artifacts/phase7-uniform-capacity-*`; the
combined receipt is `phase7-uniform-capacity-build-20261007/final-checkpoint.json`.
The stable 0.8.0 release remains immutable. The original hardware and current
production Windows gates are complete; Phase 7 is still incomplete because
Geometry and the grouped PSD/PSB pixel discrepancies remain open.

An automatic Mac diagnostic is prepared under
`build-artifacts/phase7-pixel-attribution-20261007/Mac-diagnostic`, with Chinese
instructions and a ZIP alongside the folder. It requires no Photoshop or manual
Compositor parameter edits. Three Geometry probes record Core Image provider and
drawn premultiplied bytes; 216 single-pixel Core Graphics cases isolate opacity
and blend rounding. Its four input files remain unchanged. Windows checked the
script contents and Bash syntax, but the Swift program has not yet compiled or
run on the Mac. No output or guessed rounding rule is treated as accepted proof.
The returned diagnostic is required before resolving those counterpart pixels
and writing the final all-gates acceptance receipt.

## Received framework diagnostic and covered-source correction

The actual Mac diagnostic was returned on 2026-10-07 under
`phase7-pixel-attribution-20261007/Mac-pixel-diagnostic-20261007-110407`.
Its fifteen received files were independently frozen. The three script/source
hashes match the original package, all four original diagnostic inputs remain
unchanged, and all 339 earlier Mac returns and 65 acceptance inputs still match
their receipts. The Swift execution succeeded on macOS 26.5.2 (25F84). It is
framework attribution evidence, not a replacement for application gestures.

The 216 one-pixel results show that drawing with opacity materializes a covered
RGBA8 source before blending. The former floating coverage differs in 72 rows;
rounding the covered source matches all 180 partial-opacity rows plus 24
full-opacity Normal/Screen controls. Five full-opacity Multiply rows give a
different result in Core Graphics. An integer fast path is an attribution
hypothesis, not an independently observed implementation detail; all twelve
full-opacity Multiply rows remain in the retained complete diagnostic and are
not claimed as passing this covered-source correction.
The public synthetic oracle documents its 204-row scope and includes a Swift
reproducer, with strict independent CPU and GPU assertions.

Source `e1cfcecb2a79f1f74faeffc83957e8605ee36df3` rounds the covered RGBA source
in both ordinary CPU layer draws and the WebGL2 layer shader. It changes no
Geometry sampler, original assertion, tolerance, retry or performance budget.
Reimporting both original grouped Photoshop inputs now gives zero different
pixels over 163,840 pixels each. All fourteen returned projects also open,
save and reopen with outputs pixel-exact to their Mac PNGs. The saved Geometry
project carries Mac-produced pixels; this does not prove Windows recomputation.

Local checks passed 679 engine tests with the ten original ignores, all 298
unit tests, the three TypeScript checks and release WASM. The optimized WASM is
3,710,196 bytes, SHA-256
`c285b8aec74f9c2b3f9ca64813f8d20bb1a8f7ddc3101aa0a171c20f4b4ba05b`.
Seven renderer cases pass on an isolated Vite port with one worker and no
retries, including all 204 exact CPU/GPU oracle rows. The new test's first run
read an empty region because the import centre was mistaken for an origin;
its three failures remain retained. Correcting the test placement produced the
passing run without weakening any pixel assertion. A local full-workspace
build was stopped during dependency compilation; it establishes no assertion
result. Exact-source hosted run
[`37568673577`](https://github.com/jsun2020/compositor-windows/actions/runs/37568673577)
subsequently passed the complete workspace: 699 native tests, zero failures and
ten original ignores. All 298 unit tests and 275 functional cases passed, with
33 original hardware opt-ins skipped. Release WASM, the three TypeScript checks,
web build, release metadata regressions and actual RAW sensor-helper checks also
passed. Its conditional production-portable step was skipped; hosted CI does
not establish a new native package or local hardware-performance result.

Geometry's returned provider bytes equal its drawn premultiplied bytes exactly.
The current independent sampler differs in 4,747 geometry-only pixels by at
most one premultiplied byte per channel. The earlier full Camera Raw recipe
still has 4,743 different pixels after calibration. Neither guessed phase
quantization nor half precision reproduces the full returned image, so no such
candidate is applied to production and no tolerance is added.

A new four-file diagnostic and ZIP are prepared under
`phase7-pixel-attribution-20261007/Mac-geometry-precision`. `MAC-STEPS.md` gives
Chinese Terminal/Finder steps, with all parameters already embedded. It records
RGBA8, half and full-float output, a controlled grey-ramp phase sweep and explicit
straight/premultiplied input controls. The software-renderer option is a request,
not proof that the Mac actually used a CPU backend. Its inputs/ZIP are hashed,
the Bash syntax and ZIP CRCs pass, but its new Swift code still requires actual
Mac execution. The package needs no Photoshop or Compositor UI edits.

The combined local checkpoint is
`phase7-pixel-attribution-20261007/received-analysis/current-checkpoint.json`.
The later publication receipt, complete hosted logs and exact-source run
snapshot are retained separately in `received-analysis`; the earlier local
checkpoint's pending-CI observation is preserved as a historical snapshot.
The previous 29/29 hardware suite and 1007 native portable receipt remain tied
to `0a85e89`. The final rendering source must complete its own delivery checks
after Geometry is resolved. Phase 7 remains incomplete; the stable 0.8.0 release
has not been changed or republished.

## Received Geometry precision controls, 2026-10-07

The next actual Mac return is `Mac-geometry-precision-20261007-123334`, under
`phase7-pixel-attribution-20261007`. All 122 files are inventoried, the three
reported executable/source hashes match the delivered inputs, and all 69
required/available precision records decode using their reported component,
alpha, byte-order and row-stride metadata. Provider allocation padding is not
interpreted as pixels. All four delivered inputs remain unchanged.

RGBAf output converted directly to bytes equals the actual RGBA8 result for
every complete recipe. Half-float output does not establish the internal
sampling format. The default, float-working-format and software-request
contexts give identical full-float pixels for this recipe; the software request
does not prove that Core Image used a CPU backend.

The byte-texture observations support eight-bit interpolation phases followed
by four fractional bits per filtered source byte. Normalizing that value to
f32, interpolating canonical straight source bytes and premultiplying the
filtered RGB matches the received ramp/edge controls. Converting that normalized
f32 to an output byte must not first round the multiply by 255 to f32: doing so
changes values just below a half-byte boundary. The Camera Raw sampler now
follows these observed rules. Its companion independent Mac oracle contains
all 3,840 pixels from fifteen grey-ramp phase rows, with a public Swift
reproducer and strict exact-RGBA Rust assertions. The received RGB channels were
independently checked equal before publishing a single `red` array per row.
All eleven original Camera Raw tests and the new oracle pass unchanged.

A fresh native recomputation preserves the original synthetic source hash and
all four original recipes. The first three recipes remain exact across all
163,840 pixels each. Geometry plus Calibration improves from 4,743 differing
pixels to 27, with a maximum delta of one byte in each RGBA channel. The separate
geometry-only sampling model also retains 27 differences. This is a measured improvement, not a
passing counterpart gate. Alternative perspective arithmetic models still
leave differences and have not been applied as a guessed production fix.

The new `Mac-coordinate-blend` four-file package and ZIP combine float coordinate
fields, byte-texture checker controls and an expanded Normal/Multiply/Screen
matrix with explicit copied-input controls. All Geometry parameters are embedded;
the Chinese `MAC-STEPS.md` requires only one Terminal command, with no Photoshop
or Compositor edits. The original source PNG hash is unchanged. Bash syntax,
ZIP CRCs and every archived input byte pass; the new Swift program still needs
actual Mac compilation/execution. Its diagnostic scope does not replace the
original application recipe, byte equality, native package or timing gates.

Evidence is retained in `precision-123334-analysis`, alongside
`mac-coordinate-blend-package.json`. The remaining 27 Geometry pixels and five
previous full-opacity Multiply controls remain open. Final production and
original performance verification must use the resolved rendering source.
Phase 7 remains incomplete, and the stable 0.8.0 release remains unchanged.

## Received coordinate and blend controls, 2026-10-07 14:07

`Mac-coordinate-blend-20261007-140755` contains 27 received files, 18 image
records and 7,776 independent blend cases. All three reported input hashes match
the four-file delivered package; its ZIP and every packaged byte are unchanged.
The float-field identity retains its original row orientation. The two original
PNG controls exactly match the previous precision return, and all 72 repeated
blend cases exactly match the first pixel diagnostic. Copy controls confirm all
39 distinct premultiplied source/backdrop tuples materialize unchanged.

The complete full-opacity Multiply matrix supports rounding the overlap and the
combined non-overlap independently, including alpha. This general formula
matches all 1,296 full-opacity pairs. At opacity 254/255 the covered source and
single final rounding match all 1,296 Multiply pairs instead. Normal and Screen
remain exact at both opacity values. No tuple exception or expected pixel
generated from Windows is used.

The CPU byte draw and WebGL ordinary layer shader now follow this distinction.
`compose_u8` applies the full-coverage byte contract; `compose_covered_u8` preserves
the single final quantization for covered sources and surface composition, and
the floating-point `compose` contract is preserved. The first full native run
retained one existing effects assertion failure: its expected byte draw still
used the old public byte helper. Unifying the helper and full-coverage layer
draw corrected that inconsistency without editing the existing effects test or
its assertions. The corrected full engine run passes 681 tests, zero failures
and ten original ignores. All 298 unit tests and three TypeScript checks pass.

The new public synthetic oracle and Swift reproducer contain all 7,776 received
cases. Six browser matrix tests check every CPU/GPU result and independently
verify that importing their PNGs preserves each premultiplied input. Together
with the three original opacity-oracle tests, the final release-WASM build after
the byte-helper API refactor passes nine cases with one worker and zero retries.
Its build, browser logs and exact-source hosted verification are recorded
separately in `coordinate-140755-analysis-v2`. Fresh PSD and PSB imports remain exact across
163,840 pixels each. Fourteen returned Mac projects remain pixel-exact across
22,121,472 pixels after native disk save/reopen; these saved pixels do not prove
recomputation of Geometry from the original source.

The returned float field and byte checker share the same quantized sample phases:
the interior coordinate field itself only measures a 1/256-pixel grid. Candidate
perspective arithmetic fits therefore remain diagnostic and are not promoted
to production. The new `Mac-generator-coordinates` package uses a public color
kernel without an input texture alongside bitmap and original-PNG controls.
Its translation control will measure whether kernel concatenation supplies
continuous coordinates; neither fusion nor its precision is assumed. All
parameters are embedded, and its Chinese instructions require one Terminal
command. ZIP CRC, every archived byte and Bash syntax pass; actual Swift
execution remains a Mac-side requirement.

The five earlier full-opacity Multiply controls are resolved by the independent
matrix. Original Geometry still has 27 differing pixels with a maximum delta of
one byte. Final production-package and original hardware-performance checks must
use the resolved rendering source. Phase 7 remains incomplete; stable 0.8.0 is
unchanged, and no original pixel assertion, timing budget or retry was relaxed.

## Actual kernel arguments and Geometry correction, 2026-10-08

The `Mac-kernel-arguments-20261008-074405` return contains 37 files. All nine
unobserved controls equal the previous independent return; all nine observed
controls equal their unobserved outputs. The identity-kernel sentinel retains
its pixels and exact host Double arguments. Four public method observers are
installed in the standalone process, forwarding the original arguments and
restoring dispatch. Eighteen calls include the sentinel and seventeen actual
perspective calls. Seven individual/combined inputs and eight additional
quadrilaterals provide fifteen unique matrix controls. Numeric signed-zero
parsing was corrected in a separately retained v2 analyzer; the first analyzer
failure is preserved and did not require a Mac rerun.

These actual rows supersede coordinate-fit matrix estimates. The general model
uses Float32 corners/differences/products, Double ratios/coefficients/inversion,
and Float32 kernel rows. Its Float32 matrix results equal all fifteen measured
controls; this does not claim bit equality of every host Double intermediate.
The sampling coordinate preserves the observed fused multiply-add order.
The complete independently measured `[1,2)` reciprocal domain is stored as a
lossless two-bit IEEE rounding delta per mantissa, adding 2,097,152 source bytes.
Its application is bounded to the independently replayed positive exponents
in `[0.5,2)`; other inputs retain IEEE division. The fixture notes document
the machine/OS scope, hashes, original observer and standalone reproduction.
No image-coordinate exception, recipe-specific coefficient or Windows-generated
pixel expectation is used.

The fresh original `phase7_camera_probes` run retains the exact original recipe
bytes and source PNG hash. All four 512 by 320 comparisons now have zero
differing pixels and zero maximum byte delta: 655,360 pixels in total. The
preceding 27-pixel Geometry failure remains recorded. The fifteen matrix tests
and full 8,388,608-mantissa reciprocal check pass; all 298 unit tests pass.
Evidence is retained in `build-artifacts/phase7-final-geometry-20261008`.
Final source, release-WASM, production package and unchanged 29-case hardware
performance verification are in progress. Phase 7 remains incomplete until
those resolved-source gates pass; stable 0.8.0 remains unchanged.
