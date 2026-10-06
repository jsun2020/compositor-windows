# Phase 7: Camera Raw, PSD/PSB and camera RAW

Authorized by the user on 2026-10-06. Development starts on `codex/phase7`,
from documentation commit `f463b6264e62baf11bc55e1801acf6022ddc5356`.
Phase 6 PR #8 and published 0.8.0 remain separate from this work.

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
- [ ] Import conversion report and native/browser file routing.
- [x] Pinned RAW decoder, development controls and portable dependency receipts.
- [x] Deterministic synthetic PSD/PSB/DNG probes and malformed-input checks.
- [ ] User-provided RAW/PSD sample verification (paths requested).
- [ ] Complete original source and performance gates; no changed budgets/retries.
- [ ] Rebuilt production portable with no test API, native real-file checks.
- [ ] Actual Mac 1.4.5 counterpart returns and human Windows checks.
- [ ] Documentation, commit/push, exact-source CI and final acceptance receipt.

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
