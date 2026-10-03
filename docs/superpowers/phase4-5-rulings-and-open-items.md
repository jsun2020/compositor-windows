# Phase 4.5 delivery and validation

This completes Tasks 9-15 of the 2026-09-30 catch-up plan on top of Tasks 1-8
already committed on `phase4.5` (starting HEAD `3e69dac`). The release is 0.6.0.
The implementation follows the Mac `v1.4.5` tag, `086f163`, rather than the
1.2.10 files currently checked out in the adjacent Mac repository.

## Delivered behavior

- Add Mask reveals the selection, consumes it, and records Reveal Selection in
  undo. Alt-click hides the selection and records Hide Selection.
- Inverse of Select All leaves no selection; undo restores Select All.
- Ungroup Layers replaces a folder with its children, selects those children,
  and releases clipping that loses its base. Menus and Ctrl+Shift+G use the same
  action. Nested folders survive.
- Dragging a project tab by at least 3 px reorders tabs. Background work and
  project operations prevent switching/reordering; tab text is not selectable.
- Upright resize handles snap moved edges to the canvas and other layers.
  Proportional resizing chooses the nearer snap; rotated layers do not snap.
- The Move bar edits W/H from the existing origin. Its aspect lock also controls
  resize handles, with Shift inverting the lock during a drag. The bar keeps one
  row and reserves the Apply/Cancel space while those controls are hidden.
- README, format-version wording, and all release version fields say 0.6.0.

Mac behavior references are in the implementation plan and production comments:
LayerMask.swift:237-278, LayerGroups.swift:214-239, ProjectTabs.swift:160-171,
Crop.swift:139-182, and TransformInspector.swift:24-29,49-60,89-100 at v1.4.5.

## Validation performed on this checkout

Logs and mutation evidence are retained locally in
`build-artifacts/phase4-5-validation/`; generated evidence is not committed.
Numbers below are actual results, not the scratch-clone counts in the plan.

| Layer | Result | Evidence |
| --- | --- | --- |
| Native engine | 577 passed, 10 ignored, 72 test binaries | rust-full.log |
| Native shell | 9 passed | rust-full.log |
| UI units | 256 passed in 38 files | unit-restored.log |
| TypeScript and Vite | Passed | build-restored.log |
| Browser regression, first run | 171 passed, 27 skipped, 1 timeout | e2e-full.log |
| Browser regression, second full run | 172 passed, 27 skipped | e2e-second-full.log |
| Selection/ungroup after restoring mutations | 35 passed | engine-restored.log |
| Deliberately introduced bugs | All 23 failed their intended assertions; all files restored | mutation-results-*.json |
| Release wasm | 2,989,247 bytes | wasm-release.log |

The first browser run's pre-existing Mac lattice-edge blur case exceeded its
original 90 s limit while the native workspace was compiling. The second full
run passed that case in 52.5 s with the same configuration, assertions and limit.
The initial failure and its error context remain saved. Resource contention is
a plausible explanation, not a demonstrated source fix; one pass does not
establish that the intermittent timeout is fixed.

Mutation checks covered the reversed mask tones/name/Alt action, both inverse
failures, ungroup position/clipping/selection/shortcut, tab translation/index
and busy/text-selection guards, resize snapping decisions and targets, W/H
origin/ratio/handle/Shift behavior, the one-row bar, and the version smoke pin.
The mutation logs contain assertion failures, not timeouts or compile failures.

## Release performance and portable build

All nine release timing cases passed alone (two needed the plan's one manual
rerun) and then together: `perf-whole.log`, 9 passed in 4.6 min, one worker.
Every measured run reports
`ANGLE (Intel, Intel(R) HD Graphics 520 (0x00001916) Direct3D11 vs_5_0 ps_5_0, D3D11)`.
All performance assertions retain their original limits; cold warm-up costs
are logged separately. These are the complete-file results, in milliseconds:

| Path | 24 MP | 100 MP | Original limit |
| --- | --- | --- | --- |
| Delete after worker result, UI worst of 5 at fit / 40% | 71 / 70 | 258 / 263 | 150 / 400 |
| Delete's following frame, median at fit / 40% | 5 / 7 | 5 / 7 | 33 |
| Worker result's following frame, Levels / fill | 7 / 12 | 16 / 21 | 100 |
| Worker editing, longest frame gap, Levels / fill | 18 / 18 | 20 / 18 | 100 |
| Levels job input / result installation | 96 / 104 | 425 / 431 | 150 / 450 |
| 1.4.5 results, frame worst at fit / 1:1 | 39 / 34 | 34 / 43 | 80 |
| Saturation step worst at fit / 1:1 | 35 / 33 | 32 / 39 | 100 |
| Add Mask, reveal / hide step worst | 219 / 282 | 792 / 757 | 400 / 1000 |
| Add Mask, reveal / hide following frame worst | 25 / 27 | 98 / 77 | 150 / 400 |
| Inverse, step / following frame worst | 1 / 6 | 1 / 5 | 16 / 33 |
| Ungroup, step / following frame worst | 4 / 12 | 3 / 11 | 16 / 33 |
| Tab drag, start / later longest gap | 24 / 19 | 21 / 17 | 100 / 50 |
| Resize with snapping, tick / release | 7 / 5 | 6 / 5 | 50 / 150 |
| Typed W, Enter / following frame worst | 3 / 15 | 2 / 22 | 16 / 33 |

Delete used partial texture uploads only and did not recompute the display
halving; the following frames' measured layer-region reads were at most 2 ms.
The Levels input/install copies at 100 MP approach their original 450 ms
limits and remain a cost to watch on this laptop.

The first isolated F1 run measured a 138 ms frame gap during 100 MP fill,
exceeding 100 ms (`perf-alone-2.log`). Its one rerun measured 19 ms for that
path and passed (`perf-rerun-f1.log`); the complete-file run measured 18 ms.
The first isolated mask case could not find engine-ready in the original 5 s
wait (`perf-alone-4.log`), before any performance measurement. Its one rerun
passed (`perf-rerun-mask.log`), as did the complete-file run. No production
change, added retry, skipped case or relaxed limit was used to obtain those
passes; the two initial failures remain explicit intermittent observations.

Portable build: `scripts/build-windows-x64.ps1 -SkipWasm -NoSmoke`, exit 0,
using the already measured release wasm. The embedded web build has exactly
one `COMPOSITOR_BUILD_0.6.0_20261002-2157` marker and the same 2,989,247-byte
release module. The packaging script's temporary build-info source was restored
byte-for-byte. Old portable versions were preserved.

- Package: `build-artifacts/windows-x64/Compositor-portable-0.6.0-20261002-2157.zip`
- EXE: 10,723,328 bytes; FileVersion and ProductVersion both 0.6.0.
- ZIP SHA256: `D7FD3951233B05569F42ACF5BF0ECF55FA3EBB2145E5F84D6D5A900FDF719029`
- Release wasm SHA256: `00F0FE443BB3C09B722325B1F5C4C2AE44F7AA63A154E4F18739CE846AAA4413`

ZIP CRC verification and byte identity of its EXE with the staged EXE passed
(`zip-integrity.log`). After packaging and timings, `pnpm wasm:dev` restored
the development engine successfully and the unchanged engine smoke checks
passed, 2 tests (`wasm-dev-restored.log`, `dev-smoke-restored.log`).

The packaged EXE survived startup and passed real native WebView2 UI checks via
its own temporary localhost debug port, with the native Tauri bridge present
and the development test API absent: new canvas and fill, W/H aspect ratio and
unchanged origin, undo, group/ungroup and undo, and tab dragging. The runtime
reported the exact build marker above. There were no page errors. Evidence:
`native-ui.log`, `native-ui-result.json`, and `native-0.6.0.png`. Only the process
started for this check was stopped; the temporary browser profile is isolated
under the validation directory. This is packaged GUI evidence, separate from
the full browser suite and the Mac export checks.

## Remaining scope and acceptance boundaries

Phase 4.5 is the catch-up milestone inserted before the remaining Phase 4
features. It does not complete all of Phase 4. As the design spec records, the
remaining sequence is 4b-2 (floating selection and Windows clipboard), 4c
(brushes, re-researched against Mac 1.4.5), 4d (retouching), then Phase 5.
Phase 3.5d retains the sampling work and upright pixel-for-pixel copy issue.
The existing untracked sampling fixtures and `.workbuddy/` are untouched.

Mac export acceptance is also separate from source/browser validation. B1 and
the new 1.4.5 probes still need Mac exports; A1/A2 text-run fixtures remain
explicitly hand-written, and A3/A5 round-trip and A6 mask hand steps remain
listed in the plan/probe README. No test pass substitutes for those Mac saves.
Current resampling and layer-effect interoperability limitations remain in the
README; no cross-platform acceptance claim is added here.

Ego browser bootstrap and the in-app browser launch were unavailable in this
session. The browser tests drove the actual web UI using the mock file bridge;
the later packaged WebView2 checks used the real native bridge as recorded above.
The user's manual review of the portable is still pending.

Phase 5 has not started. The user requested a review/confirmation before it.
