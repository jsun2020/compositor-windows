# Phase 4 remaining work and Phase 5 delivery

## Scope and publication

The user authorized committing and pushing the existing work first, then
implementing the remaining Phase 4 features and automatically entering Phase 5.
The prior confirmation gate before Phase 5 was superseded.

The existing 0.6.0 work was published as commit
`e1be481b83b3209f200c10eb0ba7ab954397ba14` on `origin/phase4.5`. The remote ref
was verified after publication. New implementation is on
`codex/phase4-and-phase5`; it is not included in that earlier push. At the user's
next publication request, implementation commit
`04902eb80b5f4087baed50ec8ba994b6c8847e03` was pushed to that branch and its exact
remote SHA was verified. Only the reviewed source allowlist was published.

The implementation below is delivered in 0.8.0. Final validation is recorded
separately from feature implementation. Phase 3.5d sampling remains outside this
work, and the user's sampling probe files and `.workbuddy` directory are preserved.

## Implemented behavior

### Clipboard and floating selection

- Ctrl+C, Ctrl+Shift+C, Ctrl+X and Ctrl+V use the Windows PNG clipboard. DIBV5
  supports bitmap consumers; DIB/DIBV5 reads support external bitmap producers.
  PNG retains alpha and an internal origin retains document coordinates.
- A whole layer or folder copied in this running app retains masks, effects,
  hierarchy and editable metadata through a private clipboard token. After the
  app closes, external PNG paste retains the rendered image rather than those
  internal records. External images are centered when no internal origin exists.
- Cut edits only after a successful clipboard write. A failed write preserves
  the source and history. Ctrl+J makes a layer from selected pixels; without a
  selection it duplicates the selected layer.
- Move and Free Transform operate on floating selected pixels. Move, duplicate,
  resize, rotate, distort and Shift-constrained movement commit one undo. Escape
  restores the exact document and history. Apply/Cancel controls remain available.
- Floating merges retain layer effects, as explicitly specified for Windows.

### Paint and retouch

- Brush B and Eraser E expose diameter (1-2000), hardness and opacity. Brush adds
  string smoothing, including catch-up at mouse-up; Shift connects straight lines.
  A settled centripetal Catmull-Rom path feeds the dab spacing. Mask painting uses
  grayscale and preserves white when growing a revealing mask.
- Blur R exposes its radius (default 5). Clone Stamp S supports Alt-click source,
  aligned sampling, current-layer sampling and visible all-layer sampling onto
  a blank target. Source pixels are frozen for each stroke.
- Spot Healing J provides Content-Aware, Create Texture and Proximity Match.
  Healing and Content-Aware Fill compile the original v1.4.5 C kernels for native
  Windows and WASM. Kernel bodies are unchanged; runtime includes and allocation/
  math bridges are adapted. The portable includes the original MIT license.
- Edit > Content-Aware Fill supports preview off/on, Cancel and Apply, including
  extending past a layer's old edge. Preview and cancellation add no history;
  Apply adds one undo. Canceled or superseded asynchronous results cannot install.

### Editable text, shapes and effects

- Type T creates a point text layer or a wrapping fixed box by dragging. Layer >
  Edit Text reopens content, font, size, color, alignment, tracking, leading and
  box dimensions. Selection font/color ranges use UTF-16 positions, including
  surrogate pairs. Text and runs survive `.comp` save/reopen and undo.
- Preview does not change saved content or history. Ctrl+Enter applies; Escape
  cancels. IME composition Escape does not cancel the text editor. Font rendering
  uses Windows browser fonts and fallback; Arial 72 is the Windows default.
- Shape resizing redraws its source geometry, keeping corner radius and line
  width instead of scaling an existing bitmap. Preview, cancellation, masks,
  editable records and one-step undo are preserved. Pixel painting rasterizes
  editable text/shape records.
- Layer > Layer Effects edits Stroke and Drop Shadow with preview, Cancel and
  Apply. Raw pixels and editable text remain intact. Unknown fields and the other
  existing effects are preserved.

## Responsiveness changes and performance evidence

Large input copies and result installation use cooperative 4 MiB chunks. Small
edits retain the established direct path. A checked region witness avoids a
whole-image equality scan at installation. Worker-produced display halvings are
adopted without recopying their whole cache on the UI thread. Committed large
transferred buffers are detached after installation when the runtime supports
`ArrayBuffer.transfer`; prepared previews keep their buffers until Apply/Cancel.

Release WASM measurements use hardware Edge, Intel HD Graphics 520 / D3D11, at
24 MP and 100 MP. The original assertions and time budgets are retained. Perf
instrumentation was updated to observe asynchronous input/install calls; CPU
timing excludes voluntary frame waits, and frame-gap assertions remain intact.

Before these changes, measured 100 MP frame gaps were approximately 487 ms for
paint, 365 ms for blur and 337 ms for healing. The final repeated targeted run
recorded paint 47 ms, blur 23 ms and healing 65 ms, with main engine memory about
900 MiB. These improvements do not close every acceptance gate:

- Content-Aware Fill passed once at both sizes, then failed repetitions at
  100 MP (106-144 ms gaps, against the unchanged 100 ms budget). A 24 MP
  repetition also measured 125 ms. It is not accepted as a stable performance fix.
- The earlier F1 Delete-after-job test passes after updating its observer to the
  actual async installation path. All fills return and adopt their display cache.
- F1's first frame after filling a blank layer still fails (131 ms at 100 MP,
  and a 384 ms 24 MP outlier). The measured large-job CPU copying and installation
  remain within the existing budgets, but the frame assertion does not.
- The broader Phase 4.5 run also failed Add Mask at 100 MP (571 ms versus 400 ms)
  and tab-drag start at 24 MP (392 ms versus 100 ms). No assertion was loosened,
  and these failures remain open rather than being inferred fixed from other runs.
- Floating-selection commits, very large text rasters and full clipboard
  round-trips are not covered by the new brush performance measurement.

Evidence is retained under `build-artifacts/phase4-remaining-validation/`, notably
`perf-brush-before.log`, `perf-brush-final.log`,
`perf-final-repeat-and-baseline.log`, `perf-buffer-release.log` and
`profile-final.log`. Native GUI checks and source tests do not substitute for
these performance gates.

## Validation and packages

Final TypeScript production builds pass. Vitest: 259 passed in 39 files.
The full fixed-asset browser suite: 188 passed, 29 unchanged opt-in performance
skips, zero failures (12.9 minutes). This uses release WASM and a separately
bundled mock bridge/test API, so native packaging cannot trigger hot reloads
inside a running regression. The 29 skips are not evidence of performance
acceptance; the separate performance runs above retain their failures.

Final native workspace tests: 613 passed, zero failures, 10 existing ignored
tests. The successful log is `native-final-compact.log`. The first final native-test/build
attempt failed because C: ran out of disk space, not from a test assertion.
After verifying no compiler remained active, only this project's ignored,
regenerable `target/debug/incremental` subtree was removed. This recovered
7.26 GiB of actual free space; source, logs, EXEs/DLLs and previous portable
packages were retained. The native-test retry disables incremental compilation
and test debug symbols; assertions and execution logic remain unchanged.

The final portable is
`build-artifacts/windows-x64/Compositor-portable-0.8.0-20261003-0225.zip`,
4,741,956 bytes. ZIP SHA-256:
`20AF1363CB2FF9F53F04FC933456DE5392C0CC0986C0EADCDB76EB2BBD0DE05C`.
The executable is 11,972,608 bytes, SHA-256:
`804E984A9C206DD8AC6FA3F60A2F851D28A24827CBBF1EB1FEAEC93A70577B4D`.
Bundled release WASM SHA-256:
`0E96D2C6185565BFA8B96BD51D6FC812C1DDCF3EB0BE62C039F13DBA2F66DA88`.

The actual portable launched and survived the initial 5-second check. A first
CDP connection arrived before its fresh WebView initialized; that log is retained
as `native-runtime-final.log`. The harness now waits for endpoint readiness before
connecting, without retrying or changing any functional assertion. The subsequent
run passed in the real Tauri window at `http://tauri.localhost/`, marker
`COMPOSITOR_BUILD_0.8.0_20261003-0225`, with the development test API absent and
zero page errors. It verified visible brush pixel changes, eraser/undo, blur,
aligned clone through Alt-click, all three healing modes, content-fill preview/
cancel/apply, editable text and reopening its editor, and Stroke/Drop Shadow
Apply and reopening their settings. The native read-only clipboard command
returned the same Windows access-denied message as the preflight.

Runtime evidence: `native-runtime-ready.log`, `native-0.8.0-result.json` and
`native-0.8.0.png` under the validation directory. The harness uses a separate
WebView data directory and stops only the portable process it launches.

The intermediate Phase 4 package is
`build-artifacts/windows-x64/Compositor-portable-0.7.0-20261003-0017.zip`.
Its executable SHA-256 is
`D6141A688850B6F0B0E68F4CECF89E5964DDBE28DD2325F8BA26A2EF6044E82A`.
This package predates the large-buffer fixes and Phase 5.

## Follow-up after publication: overlay and Mac handoff

The user offered to perform the Mac half of the round trip. The generated
`build-artifacts/Mac-roundtrip-0.8.0.zip` contains a Windows-authored project,
reference PNG, layer state, Chinese instructions and a result template. The
project was saved and reopened in the release-WASM app: seven layers, two
editable text records, four live shape records, a grayscale mask, UTF-16
color/font runs, stroke and shadow. All referenced layer and mask assets are
present in the verified 20-entry archive. SHA-256:
`C95E1718036DC90371951383360DE6E0FF86BF36F38B05541790056C6780D179`.
The no-edit and editing return protocol is in
`mac-roundtrip-acceptance.md`. Live Mac acceptance remains pending returned files.

A traced 100 MP Content-Aware Fill run recorded a 166 ms frame gap and a
165 ms browser `HitTest` task on the canvas while the kernel was running on the
dedicated worker. Overlay painting reset the canvas's width and height on every
ants/cursor tick, even when unchanged. It now resizes only when the viewport size
actually changes; the existing overlay drawing clears old pixels as before.

Production TypeScript/Vite builds pass. The follow-up passed 259 unit tests and
40 focused browser tests covering selections, brush/retouch, text/effects,
crop, transform, gradients and shapes. No performance assertion or budget changed.

Two isolated follow-up runs both passed F1 result-frame and UI-copy budgets at
24 and 100 MP. The measured first frames were 8-13 ms; the largest worker-edit
gap in those runs was 62 ms. Both 24 MP brush/retouch runs passed, including
Content-Aware Fill at 22 and 20 ms maximum gaps. The second run also passed
the original Add Mask and tab-drag tests: 100 MP mask upload frames were 72-73 ms,
and tab-drag start gaps were 21 ms at both sizes. These last two tests have one
post-change pass, so they do not yet establish repeated acceptance.

100 MP Content-Aware Fill still failed both runs at 125 and 140 ms against the
unchanged 100 ms limit. A diagnostic experiment forcing the 2D overlay onto a
CPU-oriented context also exceeded the limit (106 ms), with a 95 ms incremental
browser GC task in its trace; that experiment overlapped the end of unit-test
execution and is diagnostic only. The context change was not adopted. The
trace does not establish a single cause for every intermittent stall.

Logs: `build-overlay.log`, `unit-overlay.log`, `e2e-overlay.log`,
`perf-overlay-first.log`, `perf-overlay-repeat.log`, `profile-f1.log`,
`trace-content-fill.log` and `trace-overlay-cpu-experiment.log` in the validation
directory. The failed runs remain retained. The prior performance failures above
are historical measurements, superseded only to the extent shown here.

The updated portable is
`build-artifacts/windows-x64/Compositor-portable-0.8.0-20261003-0811.zip`
(4,741,921 bytes), SHA-256:
`5DFDE8512360E8DEC9372E5E31BA7AA492598C75CD2032712E0C6589DB4106D3`.
Its executable SHA-256 is
`B646911E65FA5D18289138456EC1D4F31C89FD7FE551EC1A70A2A7ACD4DBC43F`.
It passed the same seven native UI checks with marker
`COMPOSITOR_BUILD_0.8.0_20261003-0811`, a production Tauri bridge, no development
test API and zero page errors. Evidence is `package-overlay.log`,
`native-runtime-overlay.log`, `native-overlay-0.8.0-result.json` and
`native-overlay-0.8.0.png`. The earlier portable and evidence remain retained.

This native run's read-only clipboard probe reached the clipboard and correctly
reported that it contained no image, rather than the earlier error 5. A format-only
inventory identified a shell/file-drop clipboard. Its data was not read, replaced
or published. Full external-image copy/paste remains unverified; the earlier
access-denied result does not describe this latest read-only preflight.

## Acceptance still requiring evidence

- Native Windows image clipboard interoperability: earlier probes returned
  error 5; the latest probe can open the clipboard and reports no image for its
  current shell/file-drop formats. The user's clipboard was not overwritten.
  Mock clipboard and bitmap conversion checks do not establish live
  external-application copy/paste.
- Stable large-image performance for the failed gates above.
- Pixel fidelity for the Mac-edited and Mac-created exports: live Compositor
  1.4.5 return files now pass Windows native open/save/reopen and continued
  editing, with retained text/shape/effect/mask records and an exact no-edit
  composite match. The other two exports have quantified edge differences;
  see [the Mac return results](mac-roundtrip-results-2026-10-03.md).
- During a brush gesture the overlay shows the path; the full soft brush,
  erasing, masking and retouch result is installed at mouse-up. Floating preview
  does not reproduce the final union of layer effects exactly. These preview
  limitations are distinct from final committed pixels and retained records.
- Windows font fallback, shaping and kerning need not be pixel-identical to Mac
  font rendering. A missing font falls back through the browser font stack.

## Mac 1.4.5 return follow-up, 2026-10-03

The user's three Mac return projects were checked in the release engine and the
production portable. A continued text edit exposed CSS fallback for PostScript
face names such as `Verdana-Bold` and `TimesNewRomanPSMT`. Rasterization now maps
known faces to their installed Windows family and weight/style, while saving the
original font names and UTF-16 run ranges. The new real-worker regression requires
the exact installed bold Verdana bitmap. All 261 unit tests and 13 focused browser
file/format/text tests pass.

Portable `Compositor-portable-0.8.0-20261003-0927.zip` is 4,742,141 bytes, SHA-256
`05F4777A81408E788494B0EDDB0361CF6E411C56367346053A7EC99E690DCDDE`.
The executable SHA-256 is
`1D177B74314D5A799BE80E62DBE5036D1C1D5D33F0BB26283772F7B4DD49F42B`.
Native checks use marker `COMPOSITOR_BUILD_0.8.0_20261003-0927`, the production
Tauri bridge, and no development test API. They perform four actual native package
reads and four atomic package commits with zero page errors. File-picker choices
alone are substituted through a dialog-only IPC response; filesystem operations,
editing, rendering and saving remain native. See the linked Mac report for the
artifact checks and exact limits of this evidence. No performance budgets changed
and no new performance acceptance is inferred from this font-only change.

## Full-acceptance checkpoint, 2026-10-03

The explicit full-acceptance goal remains active. [The acceptance record](phase5-acceptance-2026-10-03.md)
is the authoritative latest snapshot; the historical logs and packages above
remain retained. Production marker `COMPOSITOR_BUILD_0.8.0_20261003-1206` includes
native clipboard origin/DIB fixes, worker-buffer reuse and retirement, and direct
staged WASM chunk copies. It passes 616 native tests, 268 unit tests and 190
functional browser checks. The 29 unchanged opt-in skips are measured separately:
latest applicable performance coverage is 22 passed / seven failed. Six legacy
cases now monitor the async APIs actually used, with every original assertion,
timeout and budget preserved.

The real portable passes the native UI groups (including effect preview/cancel
and undo/redo) and four native Mac package reads/four atomic writes, continued
text/shape undo/redo and saved effect parameters. Its three native PNGs and 34
saved package files match the prior validated Windows outputs. The exact no-edit
Mac match and edited/created export edge differences retain their earlier limits.

The final clipboard preflight cannot open the system clipboard and aborts before
any mutation; an independent Win32 probe and production read also return error 5.
A manual Windows result and the missing Mac gesture confirmations are requested.
These gates and the recorded performance overruns remain open. No full acceptance
is claimed, and only reviewed source/tests/docs are published.