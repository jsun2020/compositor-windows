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
upright 1:1 pixel-copy rule and its antialiased rectangle edge. Enlarged/rotated
filter changes are not yet included; the user's sampling probes remain local.

Implementation boxes describe delivered code, not completion of every acceptance
gate. The delivery report records validation results and remaining native
clipboard, large-image performance and Mac interoperability evidence separately.

- [x] Native workspace tests: 622 passed, zero failures, 10 existing ignored.
- [x] Vitest: 287 passed. Current TypeScript and fixed-asset UI build pass.
- [x] Latest solid-rectangle source full fixed-asset browser suite: 195 passed,
  29 unchanged opt-in skips, one worker and zero retries (8.7 minutes).
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
- [ ] All large-image performance budgets: repeated failures retained in report.
- [x] Live Mac return files open/save/reopen and continued text/shape/effect
  editing in the production Windows package.
- [x] Current release-WASM exports match Mac-no-edit, Mac-edited and the
  fractional/flipped upright 1:1 probe exactly, with transforms unchanged.
- [ ] Mac undo/redo and effect preview/cancel/apply confirmation, and exact
  Mac-created enlarged-text export fidelity. Covered overlap edges are now exact.


Acceptance evidence, including historical checkpoints, is [the 2026-10-03 acceptance record](../phase5-acceptance-2026-10-03.md):
621 native tests, 287 unit tests and 195 current functional browser passes
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
