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

No sampling changes from the separate Phase 3.5d backlog are included here.

Implementation boxes describe delivered code, not completion of every acceptance
gate. The delivery report records validation results and remaining native
clipboard, large-image performance and Mac interoperability evidence separately.

- [x] Native workspace tests: 613 passed, zero failures, 10 existing ignored.
- [x] Vitest: 259 passed. Production TypeScript/UI build passes.
- [x] Full fixed-asset browser suite: 188 passed, 29 existing opt-in perf skips.
- [x] 0.8.0 portable build and real Tauri window checks, including visible
  painting, eraser/undo, blur, clone, healing, fill, text and effects.
- [ ] Live Windows clipboard interoperability: session access denied.
- [ ] All large-image performance budgets: repeated failures retained in report.
- [ ] Live Mac round-trip of newly authored text, shapes and effects.
