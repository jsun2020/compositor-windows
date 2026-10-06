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
- [ ] Complete original hardware performance gates; no changed budgets/retries.
- [x] Rebuilt production portable with no test API, native real-file checks.
- [ ] Actual Mac 1.4.5 counterpart returns and human Windows checks.
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
`build-artifacts/phase7-acceptance-20261006-order-fixed`. Its result fields are
still blank. The remaining human checklist is
`build-artifacts/phase7-remaining-checks-20261006.md`: four Camera Raw recipes,
synthetic PSD/PSB structure and editable records, actual camera RAW on Mac 1.4.5,
and actual Windows close/save-picker gestures. The supplied real PSD counterpart
has already passed and need not be repeated. Phase 7 remains incomplete until
the original hardware budgets and the remaining human returns pass.
