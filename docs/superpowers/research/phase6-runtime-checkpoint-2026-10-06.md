# Phase 6 runtime checkpoint — 2026-10-06

Phase 6 development acceptance is complete on the tested Windows hardware
and received Mac/Windows probes, including the final last-tab correction.
The corrected Mac return now contains committed edits in all three projects,
three lossless PNGs and preserved styling/masks. The initial unchanged return
and intermediate recheck remain retained below. Keep PR #8 open and published
0.8.0 assets unchanged.

Windows operator returns have now been received and verified. The operator
also reported a stale picture after closing the final tab. Source regressions
reproduce and fix this issue in both WebGL and CPU rendering; a rebuilt
development portable and exact-source hosted checks now pass. Earlier pending
checkpoints below are retained chronologically; the final results are recorded
at the end of this document.

## Frozen Windows development package

Source: `a01c7ed4a4eab45c706c2c14c26367e8f62b604d`.
Marker: `COMPOSITOR_BUILD_0.8.0_20261005-2328`.
EXE SHA256:
`d154bdb3d312c924f07e5d643090d8b1a0cea711294bba56275dec56bc24177a`.
WASM SHA256:
`289c4a8a96808283732123c12d1be53b81c1ad3db994116b00edef6b21801079`.

Exact-source hosted push and PR CI passed: 661 native cases, 295 unit cases
and 246 functional cases. Ten pre-existing native ignores and 33 opt-in
skips remain unchanged. CI does not establish local native clipboard,
actual OS mouse or Mac gestures.

Two independent supplemental production WebView2 protocols passed eight
24/100 MP Liquify/Smudge cases and two small-canvas protocols covering both
modes' preview, Escape, commit and undo/redo. Fresh ports/profiles were used
sequentially, with no automatic retries. All 866/881 session observations
were unlocked; sampling does not establish state between observations.

The maximum subsequent preview response, including two rendering frames
and PNG capture, was 197.1156 ms; frame gap 54.3 ms; input 0.2 ms; completion
3430.7596 ms. Full dimensions and 96/400 MB result buffers were checked.
The existing 256 MiB history policy retains 24 MP undo and refuses to
retain the 100 MP preceding raster. These are aligned fixture measurements,
not universal performance guarantees.

The original experimental native 100 MP visual predicate remains failed.
Independent byte observations found identical final consecutive previews
after a move over a uniform-color area. The supplemental protocol requires
the corresponding GPU reply and rendering completion without requiring
every move to change pixels. It retains the first visible change, final
committed change, full-resolution checks and original timing budgets. It
does not erase the earlier failure or modify original CI assertions.

Ignored evidence: `build-artifacts/phase6-native-2328-20261006-verified.json`
and `build-artifacts/phase6-native-checkpoint-20261006.md`.

## Initial Mac return

Received directory: `build-artifacts/phase6-warp-probes-20261005-live`.
The operator's `RESULT.txt` reports platform 26.5.2 / Compositor 1.4.5,
successful gestures and no failed steps. These are operator statements;
saved files alone cannot demonstrate cancellation or undo gestures.

All frozen source assets except the intentionally edited `RESULT.txt`
match `SOURCE.json`. All three returned edited projects import in the
current release WASM and preserve exact manifest and rendered output
through a Windows save/reopen. Transforms, effects, layer opacity, blend
mode and decoded mask pixels match their originals.

However, **every decoded layer raster in all three edited projects is
identical to the original**. Their effective saved manifests are also
unchanged. PNG recompression produces different file hashes, but no pixel
edit. This establishes file compatibility, not persistence of a committed
Liquify/Smudge result.

| Return | Changed saved warp pixels | Mac export vs Windows WASM |
| --- | ---: | --- |
| 01 Liquify | 0 | 640×480 PNG; every RGBA byte equal |
| 02 Smudge | 0 | 640×480 JPG received; lossless PNG missing |
| 03 Styled-mask | 0 | 640×480 PNG; maximum RGB difference 1, alpha exact |

Styled-mask differs at 64,324 pixels / 163,608 channels; the mean absolute
difference over all RGBA bytes is 0.13314453125. No new rendering tolerance
is inferred. These comparisons render the Mac's saved rasters; they do not
compare identical input gestures or establish Metal/WebGL kernel equality.

Initial ignored evidence:
`build-artifacts/phase6-mac-return-20261006-checked/checks.json`.
The received edited packages, exports and operator record are preserved
under that directory's `received-snapshot`, with SHA256 receipts. A first
helper attempt omitted WASM `prepare_save`; its error is retained under
`phase6-mac-return-20261006-initial`. The corrected helper uses the existing
save protocol. Production code and original input files were unchanged.

## Follow-up requested after the initial return

Follow [the manual checks](../phase6-manual-checks.md), retaining a visible
committed edit before saving each copy. Release the drag, save, close/reopen
and export three 640×480 PNGs. Return the three edited `.comp` folders and
explicit gesture outcomes in `RESULT.txt`.

Smudge Escape is absent from its record line; styled undo/redo/save-reopen
also need explicit confirmation. Actual Windows OS mouse behavior remains
separate: Computer Use window capture failed on both attempts, so the
passing synthetic CDP protocols cannot fill that gate.

## Verified committed Mac return

The user explained that the initial files had been saved after undo. After
instructions to redo before saving, close/reopen and export PNGs, the user
confirmed completion and recopied the returns. The new observations are
retained independently under
`build-artifacts/phase6-mac-return-20261006-recopied`. The intermediate check
before the new files arrived is retained under
`phase6-mac-return-20261006-resaved`; only its operator record had changed.

All three corrected projects contain visible committed edits. Their
background rasters remain unchanged, and transforms/effects/opacity/blend
mode and the decoded styled mask are preserved. All frozen original source
assets still match the probe receipt. Every project imports and passes an
exact Windows release-WASM save/reopen of manifest and rendered output.

| Corrected return | Changed saved warp pixels | Mac PNG vs Windows WASM |
| --- | ---: | --- |
| 01 Liquify | 1,897 | 640×480 PNG; every RGBA byte equal |
| 02 Smudge | 3,053 | 640×480 PNG; every RGBA byte equal |
| 03 Styled-mask | 3,361 | 640×480 PNG; maximum RGB difference 1, alpha exact |

The styled composite changes at 3,361 pixels. The corrected styled Mac PNG
differs from Windows at 64,329 pixels / 164,120 channels, with mean absolute
RGBA difference 0.13356119791666668 and maximum 1. Report this measured
rounding residual rather than claiming byte equality or introducing a new
tolerance. These checks establish saved-project interoperability and edit
persistence, not exact Metal/WebGL warp equality.

The received files, `RESULT.txt` and SHA256 receipts are preserved in
`received-snapshot` within the new evidence directory. The old JPG is not
used for lossless comparison. No prior evidence or original test assertion
was overwritten. The frozen development EXE and WASM hashes remain unchanged.

`RESULT.txt` reports successful Mac live preview, undo/redo, save/reopen and
styled mask/effects/selection/refusal. The user's later completion statement
followed the redo-before-save and Smudge Escape instructions; this is
supplementary human confirmation, distinct from file-based evidence. The
received record itself still omits Escape from the Smudge line.

Actual Windows OS mouse evidence remains pending. A fresh Computer Use
attempt again failed with `FrameArrived timed out: timed out waiting on
channel`; refreshed selection and one retry failed with `window capture
timed out: timed out waiting on channel`. No actual OS drag was issued or
declared passed. Requested the human Windows result for both modes' live
held preview, Escape, release/commit and Ctrl+Z / Ctrl+Shift+Z.

## Windows operator returns and final-tab defect

The user reports Windows operations completed and supplied
`phase6-warp-probes-20261005-live/Windows-edited`, plus screenshots of a
residual image after closing the last tab. No Windows `RESULT.txt` was
received; the conversation provides human operation evidence. Saved files
alone do not establish each individual held-preview/Escape/undo gesture.

All three edited projects preserve metadata and styling, contain committed
warp pixels and pass exact release-WASM manifest/render save-reopens:

| Windows return | Changed saved warp pixels | PNG vs release WASM |
| --- | ---: | --- |
| 01 Liquify | 1,593 | All RGBA bytes equal |
| 02 Smudge | 2,086 | All RGBA bytes equal |
| 03 Styled-mask | 3,050 | All RGBA bytes equal; mask/effects preserved |

All exports are 640×480. Background pixels remain unchanged. The screenshot
shows strength 100%, so these gestures are not asserted to repeat the Mac's
parameters or establish exact kernel equality. Files and SHA256 receipts
are preserved under `phase6-windows-return-20261006-verified`.

Closing the final tab correctly removed its document state, but the persistent
canvas draw effect returned early instead of clearing its picture and overlay.
Both WebGL and CPU regressions failed with the active document null while
the two backing stores still contained pixels. The frozen 2328 production
WebView2 package also fails the last-tab empty-picture gate; that failure is
retained under `phase6-native-last-close-20261006-112307-677`.

The renderer now exposes an empty-workspace clear operation in both paths.
When no document remains, CanvasView clears the visible framebuffer and
overlay and forgets brush hover. It keeps the renderer available for the next
document. Closing a background tab retains the active picture; closing the
last tab creates no new document. An `Untitled` checkerboard requires the
user to choose File > New.

The same two source regressions pass after the fix, including explicit
new-canvas rendering afterward. Related file/render checks pass (14 cases),
warp preview/cancel/commit regressions pass (16 cases), all 295 unit cases
pass and three TypeScript checks/web build pass. Original assertions, budgets
and history policy are unchanged. Rebuilt native checks and exact-head CI
remain required before final acceptance.

## Final corrected-package acceptance

Corrected code source: `1fbe0aa830dcd6001d851ef5685300e8dc1c46fd`.
Both exact-source hosted runs passed: push `37408867139` and PR `37408873909`.
Each passed 661 native cases across 88 suites, 295 unit cases and 248
functional cases. Ten pre-existing native ignores and 33 opt-in skips are
unchanged. Source receipt: `build-artifacts/phase6-close-last-ci-verified.json`.

Development marker: `COMPOSITOR_BUILD_0.8.0_20261006-1125`.
EXE SHA256:
`d37b0c5dccc85343d863c5195dc8f5aee4d5db8f7b541088837c03b7afc10f63`.
ZIP SHA256:
`0dcf2538fb3ae4e8c7f6a3323bda9d39ee2f384865d8c2c65cd412ed6292b36c`.
WASM SHA256 remains
`289c4a8a96808283732123c12d1be53b81c1ad3db994116b00edef6b21801079`.
Build, smoke launch, ZIP CRCs, staged bytes and complete source/WASM receipts
pass; the receipt is in `phase6-close-last-portable-verified`.

The first read-only preflight reported the desktop locked, so the new native
protocol was not launched then. After the user unlocked it, the retained
preflight at `2026-10-06T03:54:53.6788741Z` reports session 1 unlocked. A fresh
production WebView2 process/profile on port 19372 ran the original helper
once and exited 0. Evidence is retained under
`build-artifacts/phase6-native-last-close-20261006-115458-177`.
The native bridge is present, the test API is undefined, and the exact marker
and EXE hash above are verified. Closing the only document removes all tabs
and layer rows, shows the empty-workspace hint, clears the entire picture to
RGB 41 (within the original one-level predicate), clears every overlay byte,
and creates no new document. Explicit File > New renders afterward; no page
errors occurred. The retained screenshot was visually checked and shows the
dark empty workspace without the closed image or brush outline.

This protocol uses synthetic CDP menu/pointer input. Human warp operation
evidence comes from the separately received Mac/Windows statements and
projects. All Phase 6 development delivery gates are satisfied within those
recorded scopes. No assertion/budget was relaxed, no automatic retry added,
and no earlier failure removed. Previous performance timings remain tied to
`a01c7ed`; the final fix affects only empty-workspace clearing. PR #8 remains
open, and published 0.8.0 assets are unchanged. A subsequent documentation-only
commit does not change the verified executable or code source identity.
