# Phase 6 runtime checkpoint — 2026-10-06

Phase 6 acceptance remains open. The received Mac projects preserve their
format and styling, but do not contain a committed warp edit. Actual Windows
OS mouse gestures also remain unconfirmed. Keep PR #8 open and published
0.8.0 assets unchanged.

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

## Remaining human evidence

Follow [the manual checks](../phase6-manual-checks.md), retaining a visible
committed edit before saving each copy. Release the drag, save, close/reopen
and export three 640×480 PNGs. Return the three edited `.comp` folders and
explicit gesture outcomes in `RESULT.txt`.

Smudge Escape is absent from its record line; styled undo/redo/save-reopen
also need explicit confirmation. Actual Windows OS mouse behavior remains
separate: Computer Use window capture failed on both attempts, so the
passing synthetic CDP protocols cannot fill that gate.
