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
| Frontend units/types | Source 473eab6 passes 301 units and all three type checks locally and in hosted CI. The subsequent performance correction passes 303 units and all three type checks locally; its hosted gate remains required |
| Complete native workspace | Complete hosted candidate audit: 718 passed, zero failed and ten original opt-ins ignored. The earlier clipping failure and incomplete local run remain retained |
| Complete browser functional suite | Corrected source 473eab6 passes all 328 cases in both hosted runs, using the original Vite source environment. The separate compiled preview passes the sampling cases; its complete attempt retains 300 passes and 28 source-only module 404 failures and is not accepted as a complete functional pass |
| Original 29 hardware cases | Frozen source 473eab6 completed 27 passes and two failures: 100 MP eyedropper 19.6 ms vs <16 ms, and 24 MP gradient install 163 ms vs <150 ms. One display, no compilers, all 2103 session samples unlocked, zero retries, all 730 inputs unchanged. Acceptance remains incomplete pending a product correction and fresh complete original protocol |
| Original four GPU warp hardware cases | All four pass on 473eab6 in the original Vite source environment; all 729 inputs unchanged and all 413 independent session samples unlocked. Original 24/100 MP allocation, response and history budgets are unchanged |
| Production portable | Clean source 473eab6 / marker COMPOSITOR_BUILD_0.9.0_20261010-0830: all 11 ZIP entries and provenance pass, actual bundled sensor helper passes, ten native IPC/UI cases including exact original Geometry export pass, and all 14 actual Windows file dialogs pass with 64 inputs unchanged. The performance correction requires a fresh package |
| Stable GitHub release | Candidate versions prepared as 0.9.0. Publication awaits all gates and actual downloaded asset verification; 0.8.0 remains immutable |

A disk-full compiler failure and a falsely successful truncated-WASM tool exit are retained locally. The new binary/export gate rejects the isolated truncated output, valid binaries missing editor exports and mismatched JavaScript glue; packaging runs it even with SkipWasm. Twelve release/guard tests pass on the candidate. Automatic approval rejected deletion of old PDBs; that deletion was not executed. External removal restored build space. A later focused run accidentally reused and overwrote an earlier failed aggregate JSON report; its original full log, error context and frozen failure receipt remain retained, and the report-reuse incident is recorded separately. Subsequent runs use distinct reports and refuse existing output directories.

The candidate at 9e1add9faf30b2ee4247ef0f267dd1dd9f4a3e6d passed the complete hosted native workspace, release WASM, types and 301 unit tests. Both hosted functional runs recorded 327 passed, 33 original performance opt-ins skipped and one stale smoke-test expectation: the test still required engine 0.8.0 after the product version became 0.9.0. The corrected smoke assertion checks the exact package version boundary rather than a fixed historical version. Both failures remain retained. Corrected source 473eab6afc3b9087c2fa83fd66d4881c2b68747a passes both complete hosted runs (38007147382 and 38007151662): 718 native tests, 301 frontend units, 328 functional cases, release WASM, types, pinned RAW sensor development and 12 packaging guards. The local native and browser processes also ended without their final summaries after session restoration; their partial logs are explicitly incomplete, not accepted passes.

Two historical distortion test expectations (centre-clipped trim and white outside L8) were corrected against independent Mac records. Their original source and failing output remain retained. Original performance sources are checked against f463b6264e62baf11bc55e1801acf6022ddc5356; painting/kernel goldens and timing budgets are unchanged.

Local evidence: build-artifacts/phase35d-acceptance-20261009. The original Mac probe ZIP has SHA256 4c3656e6d4bb5fff9c7e5c67a1b76a77d04dcf2a6e2863c56e68dabcd152f204. Earlier 25/32, focused 8/8, 62/63 with a startup timeout, subsequent 62/63 with the shrinking-edge mismatch, native clipping failure and disk-full records remain distinct.

## Final hardware correction

The first complete final-source hardware run is retained separately from source CI and native package evidence. A diagnostic instruments only the two failing cases and cannot replace the complete original gate. It attributes most gradient installation CPU time to lossless palette expansion; the 100 MP diagnostic also retains a 700 ms frame gap and 462 ms install, rather than treating its isolated eyedropper pass as acceptance.

The product correction avoids publishing identical foreground/ring state while still sampling the stored document on every request. The first short-band decoder attempt at c733204 passes correctness tests but retains a focused gradient failure (164 ms at 24 MP and 546 ms at 100 MP); it is not a hardware pass. That decoder change is reverted. A subsequent worker policy keeps the established decoder and chooses raw RGBA for large outputs with frequent colour changes, reusing the transferred input buffer and the existing cooperative direct-copy installation. Small palette jobs and long flat runs retain their original encoding. Palette bounds, reserved capacity, initialized length, exact bytes and LayerStamp checks remain intact. Added regression checks cover changed pixels at the same sampling location, ring movement/colours/cancel, both palette index depths and short-run/chunk boundaries. No original performance source, assertion, budget, worker count or retry policy is changed. The first correction passes all 303 frontend units, all three type checks and all 11 WASM-wrapper native unit tests locally. The revised transfer policy passes all 304 frontend units, all three type checks and all 11 WASM-wrapper native unit tests locally; The rebuilt release WASM is byte-identical to the accepted 473eab6 engine (b3f97820f8343427e5aff1e746936d28aabfbc2bc92ec4503dbc638d2b3637e6). All three palette transport functional cases pass, including every pixel of a 4096 by 1025 short-band raw gradient and its exact Undo/Redo. Fresh complete original hardware and native checks on the rebuilt 0.9.0 portable remain required.

The locally validated 473eab6 ZIP has SHA256 1115ec559715b280a0e011e577dded5de3c9ce47ee7b3f179432f7c778073b5a. Local and hosted WASM hashes are recorded independently; different compilation bytes are not claimed identical. Four compiled Camera Raw PNG exports are exact against the original independently Mac-verified references.

## Physically single-screen verification and uniform installation

The user physically disconnected the external display cable. Source c062e165 then completed the entire original 29-case protocol: 28 passed and one failed, with zero retries or skipped cases and all 730 inputs unchanged. The remaining failure is the 100 MP blank Fill installation, 457 ms against the unchanged <450 ms limit. All 2055 original session samples are unlocked; an additional independent read-only observer records 931 samples, all with one display and an unlocked session. The old dual-display restoration records remain retained separately. The outer paired controller stopped on a PowerShell native-command error after the CPU failure, before starting GPU tests.

The independent original four-case GPU protocol then passes completely, with all 729 inputs unchanged and all 488 original session samples unlocked. The additional display/session observer records 255 single-display/unlocked samples. Its outer wrapper incorrectly receives a null Process.ExitCode and reports failure; that wrapper record is retained alongside the successful original controlled result, complete four-case report and empty child stderr. This does not turn the separate 28/29 CPU result into acceptance.

A separate full-pixel diagnostic attributes 272 ms of a 278 ms 100 MP Fill installation to uniform expansion. Ordinary native chunk copying improves it only modestly. The next product correction uses native typed-array word fill directly in checked reserved WASM storage, with RGBA interpreted through byte storage to preserve JS endianness. It retains the existing 4 MiB cooperative chunks, bounds, cancellation and LayerStamp contract, and the initialized doubling fallback for unaligned/native storage. A real release-WASM browser regression covers translucent RGBA, a partial last chunk and exact Undo/Redo. The rebuilt release binary passes the binary/glue gate (5,659,773 bytes; SHA256 70035bc66228d4d358d3c85935a350d6c487207d20d10f198b427d03a782e6ae). All 304 frontend units, three type checks and seven actual browser transport cases pass locally. The complete original hardware/native/hosted gates remain required; no original timing assertion or budget is changed.

## Completion rule

Do not mark the port complete or publish stable 0.9.0 until final native/unit/functional checks, the original hardware budgets, native portable verification and actual GitHub delivery pass. Record source/build identities, hardware qualifications and retained failures.
