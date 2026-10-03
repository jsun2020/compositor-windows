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
- [x] Vitest: 274 passed. Current TypeScript and fixed-asset UI build pass.
- [x] Full fixed-asset browser suite: 193 passed, 29 existing opt-in perf skips.
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
621 native tests, 274 unit tests, 193 functional browser tests. Production 1654
passes eight native UI groups and four native Mac package reads/writes. Production
1556 passes six clipboard groups with four exact image checks and restoration;
1654 clipboard access is denied at read-only preflight, so its protocol suite
remains unverified. The latest single full run of all 29 performance cases is
26 passes / three failures: 100 MP Levels gap 109 / <100 ms, 24 MP growing-mask
fit drag 64 / <50 ms, 100 MP blank-gradient gap 114 / <100 ms. The asynchronous
histogram case passes; earlier intermittent failures remain unclosed by one
passing observation. All original budgets and assertions remain unchanged.
Implementation and these passing layers do not establish full acceptance.
