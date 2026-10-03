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

- [x] Native workspace tests: 621 passed, zero failures, 10 existing ignored.
- [x] Vitest: 278 passed. Current TypeScript and fixed-asset UI build pass.
- [ ] Current full fixed-asset browser suite: 193 passed, one page-setup timeout,
  29 existing opt-in perf skips. The previous source revision passed all 194
  functional cases; the current revision requires completion of this gate.
- [x] 0.8.0 portable build and real Tauri window checks, including visible
  painting, eraser/undo, blur, clone, healing, fill, text and effects.
- [x] Live Windows clipboard interoperability: production 1206 passes six native
  protocol groups and four exact image comparisons; the original clipboard is restored.
- [x] Current production 1556 clipboard verification: six native protocol groups,
  four exact image comparisons, editable UTF-16 text and original formats restored.
  The historical 1455 access-denied preflight is retained.
- [ ] All large-image performance budgets: repeated failures retained in report.
- [x] Live Mac return files open/save/reopen and continued text/shape/effect
  editing in the production Windows package.
- [x] Current release-WASM exports match Mac-no-edit, Mac-edited and the
  fractional/flipped upright 1:1 probe exactly, with transforms unchanged.
- [ ] Mac undo/redo and effect preview/cancel/apply confirmation, and exact
  Mac-created enlarged-text export fidelity. Covered overlap edges are now exact.


Latest acceptance evidence is [the 2026-10-03 acceptance record](../phase5-acceptance-2026-10-03.md):
621 native tests, 278 unit tests, 193 current functional browser passes, one
page-setup timeout and 29 original skips. Production 1654 passes eight native
UI groups and four native Mac package reads/writes; packaging the newer output
capacity and texture reuse changes is in progress. Production 1556 passes six
clipboard groups with four exact image checks and restoration; 1654 clipboard
access was denied at read-only preflight. The latest full run of all 29 performance
cases is 26 passes / three failures: 100 MP Levels installation CPU 476 / <450 ms,
100 MP blank-gradient frame gap 160 / <100 ms, and a selection-Levels page-setup
timeout before its test body. All four result-frame budgets pass in that run.
A separate diagnostic proves the original F1 body can pass with texture object
reuse, but it does not close the remaining performance failures. All original
budgets and assertions remain unchanged. Four synthetic alpha projects are
ready for Mac 1.4.5 export to identify the remaining enlarged-text sampling
difference; returned Mac exports and the pending gesture confirmations are
still required. Implementation and these passing layers do not establish full
acceptance.
