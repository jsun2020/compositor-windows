# Phase 5 acceptance, 2026-10-03

This tracks acceptance against the Phase 4 remaining / Phase 5 plan, using the
existing assertions and budgets. The user's explicit goal is full acceptance;
implementation completion and individual passing checks are not a substitute.
The unrelated Phase 3.5d sampling probes and `.workbuddy` remain untouched.

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

## Evidence in progress

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

The final external clipboard run aborts at its read-only snapshot:
`CLIPBRD_E_CANT_OPEN` (0x800401D0). No clipboard mutation occurs. The production
read-only probe also reports Windows error 5, and an independent Win32 probe
cannot open the clipboard (error 5, no locking window). The process is on
`WinSta0`, and the readable input desktop is `Default`; a locked desktop is not
established as the cause. Earlier intermediate builds completed native clipboard
protocol checks but failed bitmap pixel comparison, exposing the DIB bug fixed
here. Until the final package can complete native pixel comparison, this gate
remains open. A manual Windows copy/paste result has been requested, not assumed.
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

Full acceptance remains incomplete. The seven performance failures above need
fixes and repeatable evidence under their original budgets. Final production
clipboard pixel comparison needs successful native access; prior protocol-only
checks and bitmap unit tests do not close it. Mac gesture confirmation remains
pending, and the edited/created Mac-export edge differences remain quantified
and open. No assertion, budget, warm-up policy, skip, retry, or image tolerance
has been weakened to close those gates.

The checkpoint stages only reviewed source, regression tests and these reports.
Mac return data, clipboard data, test traces, packages, `.workbuddy` and the user's
untracked sampling probes remain local and untouched. The goal remains active.