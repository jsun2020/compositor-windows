# Phase 3.5d and final stable release

Status: in progress. Phase 6/7 acceptance remains recorded at runtime source f6b244ea2f53464e6002338cfd4cba336647c863 and main 134cc0c64d2415f1a7f35b26bb835f68a0b6015b. This plan closes sampling and delivery; historical results are not replaced by later passes.

## Scope

- CG affine rotation boundaries, reduction byte phases and grayscale clipping.
- CI convex free-distortion premultiplied pixels/masks, flips and integer bounds.
- Stable 0.9.0 containing accepted Phase 6/7 and the sampling corrections.
- Preserve upright pixel copies, Image Size arithmetic, editable metadata, undo, and original hardware assertions/budgets. CI distortion uses its own interpolation even for saved Nearest metadata, as the Mac does.
- Keep the original v1 non-goals: Vision selection, HEIC, updater, installers/signing and single-file project containers.

## Independent Mac evidence

Mac-phase35d-20261009-204401 was received from Apple M5 Pro / macOS 26.5.2 (25F84). All 82 fixed input hashes match. Both actual application export controls are byte exact against the diagnostic outputs. The harness mechanically extracts original Compositor 1.4.5 source at 086f1631573ccb2b57644e53b52bf1488fc976aa; it records its source hashes and every raw output.

All 42 cases / 44 raw records are checked completely. Ordinary affine images, six rotated Nearest cases and all ten CI perspective RGBA records are exact. Four affine-mask and two CI-L8 records have maximum one-byte differences; affine-mask means remain below 0.01. All placed transforms match. Public fixtures contain only the fixed synthetic inputs and raw references; operator media and private return receipts remain outside Git.

CI Apply is checked against independently cropped Mac pixels, including linked masks. Ten actual release-WASM command cases pass Apply, exact Undo/Redo and disk-package serialization/reopen, plus GPU/CPU comparison of the committed image. This gate does not claim that every intermediate GPU drag preview is bit-identical to a Mac Core Image bake.

## Gates

| Gate | Current evidence |
| --- | --- |
| Independent legacy CG images | Four focused tests pass, including the original strict rotation, mask and reduction thresholds and full-turn periodicity |
| Complete Mac 1.4.5 matrix | All 44 records pass the independent full-image oracle |
| Original clipping reduction | Endpoint snapping initially discarded the final 2:1 pair. Corrected product keeps both half-weight taps; all ten original order tests pass without assertion edits |
| GPU/WASM combined sampling | 73/73 pass, one worker, zero retries; all 855 recorded inputs unchanged. The final 2:1 shader and 0.9.0 build require the fresh complete gate below |
| Frontend units/types | Earlier 301/301 and type checks pass; final candidate checks running |
| Complete native workspace | Earlier complete engine audit has one clipping failure, retained; focused product correction passes. Final 0.9.0 workspace check running with hosted-CI test-profile settings |
| Complete browser functional suite | Required on final built assets; all 32 current affine cases now run in ordinary CI, with ten CI command cases and a GPU clipping regression |
| Original 29 hardware cases | Required on final source, one active display, one worker, zero retries and unchanged original assertions/budgets |
| Production portable | Required independently; source/browser tests do not replace native package checks |
| Stable GitHub release | Candidate versions prepared as 0.9.0. Publication awaits all gates and actual downloaded asset verification; 0.8.0 remains immutable |

A disk-full compiler failure and a falsely successful truncated-WASM tool exit are retained locally. The new binary/export gate rejects the isolated truncated output, valid binaries missing editor exports and mismatched JavaScript glue; packaging runs it even with SkipWasm. Twelve release/guard tests pass at the earlier checkpoint, with a fresh candidate run required. Automatic approval rejected deletion of old PDBs; that deletion was not executed. External removal restored build space. No failed evidence was erased or relabelled.

Two historical distortion test expectations (centre-clipped trim and white outside L8) were corrected against independent Mac records. Their original source and failing output remain retained. Original performance sources are checked against f463b6264e62baf11bc55e1801acf6022ddc5356; painting/kernel goldens and timing budgets are unchanged.

Local evidence: build-artifacts/phase35d-acceptance-20261009. The original Mac probe ZIP has SHA256 4c3656e6d4bb5fff9c7e5c67a1b76a77d04dcf2a6e2863c56e68dabcd152f204. Earlier 25/32, focused 8/8, 62/63 with a startup timeout, subsequent 62/63 with the shrinking-edge mismatch, native clipping failure and disk-full records remain distinct.

## Completion rule

Do not mark the port complete or publish stable 0.9.0 until final native/unit/functional checks, the original hardware budgets, native portable verification and actual GitHub delivery pass. Record source/build identities, hardware qualifications and retained failures.
