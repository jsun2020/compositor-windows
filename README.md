# Compositor for Windows

[![Windows CI](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml)
[![Windows Release](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml)

A free, open-source Windows image editor, ported from Compositor for macOS.
Built with Tauri 2, a Rust/WebAssembly engine, WebGL2 and React/TypeScript.

**Current release: 0.8.0. Phase 4 and Phase 5 are implemented and accepted on the
tested Windows desktop with received Compositor for Mac 1.4.5 projects.**
Phase 6 Liquify and Smudge development acceptance is complete on the tested
hardware and received Mac/Windows probes; these tools await a future release.
Phase 7 is integrated into `main`: Camera Raw, layered PSD/PSB import and camera
sensor RAW development. Mac counterparts have been received and independently
checked; remaining pixel discrepancies are still open. The complete original
hardware performance suite passed on the tested single-display Windows desktop.
Windows native file dialogs have passed the user-authorized scripted checks.
These features await a future release.

## Download

Get the portable Windows x64 ZIP from [Releases](https://github.com/jsun2020/compositor-windows/releases/latest),
extract it and run `Compositor.exe`. Microsoft Edge WebView2 Runtime is required;
Rust and Node are only needed for development. Projects are `.comp` folders:
keep their manifest and asset files together.

Automated releases include `SHA256SUMS.txt` and `release-manifest.json` with
the exact source commit, build marker and artifact hashes.

The Phase 7 development package must be extracted in full: it also includes
`CompositorRaw.exe`, three MSVC runtime DLLs and `LibRaw-notices`. The packaging
gate checks every file and rejects missing decoder components or license notices.

## Phase 7 development

- **Camera Raw filter:** Light, Color, parametric/point curves, Color Mixer,
  Point Color, Color Grading, Effects, Detail, Optics, Geometry and Calibration.
  Group resets/bypass, white-balance/Point Color sampling and diagnostic previews
  preserve the original pixels until Apply; Apply records one undo step.
- **Photoshop import:** bounded RGB 8-bit PSD/PSB raw/PackBits decoding, groups,
  visibility/opacity, blend modes, clipping, raster masks, supported Levels/Curves/
  Hue-Saturation adjustments, first-style editable text and supported live shapes.
  Text retains its cached Photoshop raster until edited. Unsupported records are
  reported. ZIP compression, 16/32-bit and non-RGB Photoshop documents are refused.
- **Camera RAW import:** a pinned LibRaw 0.22.2 helper develops the sensor data,
  with As Shot reset, exposure, estimated temperature, tint, tone curve, bounded
  preview, cancellation and full-size import. Embedded JPEGs are never substituted
  for sensor processing. Apple CIRAWFilter and LibRaw can produce different colour
  and tone results; camera coverage depends on the decoder.

The tested development portable has no test API. Native checks passed Camera Raw
preview/cancel/history, Photoshop conversion notices, project close choices and
real Canon EOS 40D sensor import. As Shot and Exposure +1 saved and reopened at
3908 × 2602 with unchanged exported pixels. The supplied real PSD also matched
its Mac export after correcting layer order. Those earlier checks use native file
APIs with supplied picker responses. Subsequent user-authorized scripts operated
16 real Windows Open, Save As and folder-selection dialogs without mocking their
responses. Five Photoshop imports, cancellation, discard/clear, edited PSD and
actual CR2 save/close/reopen passed; reopened viewport pixels were unchanged.
The current package also passed a 6.5 MP keyboard fill, one Undo/Redo and real
save/reopen; all 6,553,600 saved pixels were checked exactly. Application controls
use CDP; process-scoped Win32 edits and UI Automation button invocation operate
native dialogs. These results do not claim physical mouse input.
The earlier 0010 production portable also passed Point Color eyedropper, Visualize Range,
OK, Undo/Redo and native disk save/reopen on an isolated working copy. The range
overlay is excluded from the committed image. These controls were exercised
through CDP; that supplemental Point Color check does not establish OS input.

All 339 returned Mac files were independently checked. Fourteen returned projects
opened, saved and reopened in the Windows engine with unchanged exports. The
Light/Color/Curve, Mixer/Grading and Effects/Detail/Optics probes are pixel-exact.
The returned framework diagnostic identified RGBA8 source rounding before
opacity blending. The CPU and GPU correction passes 204 independent Mac pixel
oracles. Both grouped PSD/PSB imports now match their Mac exports exactly, and
all fourteen Mac-saved projects export identically after Windows open/save/reopen.
Recomputing Geometry from the original source still has a precision discrepancy;
its boundary alpha difference was reduced from 127 to 1. Mac Point Color crashed
with Visualize Range enabled; its failure report is retained.

After the uniform-fill transport and buffer-reuse correction, a fresh complete
original hardware suite passed **29/29** on one active Windows display. The
100 MP blank-layer fill result installation measured 244 ms against the original
strict <450 ms budget. All 381 frozen inputs verified before and after; one
worker and zero retries were used, with unchanged assertions and budgets. The
preceding failed runs remain retained; this pass does not establish a universal
display/driver fix. Production native checks passed separately on build
`20261007-1007`. Source `0a85e89` also passed hosted CI: 698 native tests, 298 unit
tests and 272 functional cases, with original ignores and hardware opt-ins kept
separate. Those package/performance results belong to `0a85e89`; the subsequent
opacity correction is `e1cfcec`. Its local checks passed 679 engine tests,
298 unit tests, three TypeScript checks, release WASM and seven renderer cases.
Its [complete hosted CI](https://github.com/jsun2020/compositor-windows/actions/runs/37568673577)
passed 699 native tests, 298 unit tests and 275 functional cases, with the ten
original native ignores and 33 hardware opt-ins kept separate. Geometry
attribution and final package/performance checks for the final rendering source
remain open. Phase 7 is not yet accepted.

For modified projects, **Save and Close** saves before closing, **Don't Save and
Close** discards the changes, and **Cancel Close** keeps the project open.
Cancelling the Save As picker keeps the project open as well.

The implementation and outstanding acceptance gates are recorded in
[the Phase 7 plan](docs/superpowers/plans/2026-10-06-phase7-camera-raw-and-import.md).
LibRaw is distributed under its CDDL-1.0 option with upstream notices and a pinned
[SDK source archive](https://www.libraw.org/data/LibRaw-0.22.2-Win64.zip).
The included Microsoft runtime files are unmodified redistributables from Visual
Studio's `VC/Redist` directory; see [Microsoft's redistribution list](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution).

## Features

- **Projects:** image import, multiple tabs, drag-and-drop, PNG/JPEG export with
  quality preview, crop, Canvas Size, Image Size, flip, zoom and pan.
- **Layers:** folders, reorder/nesting, all 24 Mac blend modes, opacity, clipping
  masks, linked/unlinked layer and folder masks, mask painting, multi-layer
  transforms, snapping, merge, duplicate and ungroup.
- **Adjustments:** Levels, Curves, Hue/Saturation, Exposure, Gradient Map, Grain,
  Invert, Black & White and Color Balance, with previews and adjustment layers.
  Gaussian/Motion Blur, Add Noise and Lens Correction are available.
- **Selections and fills:** marquee, lasso, Magic Wand, add/subtract, move outline,
  invert/expand/contract/feather, foreground/background fills and gradients.
- **Clipboard:** Windows PNG/DIB Cut, Copy, Copy Merged and Paste; Layer via Copy;
  floating-selection move/resize/rotate/distort; cancellation and one undo per
  gesture. Internal copies retain editable metadata; external PNGs carry pixels.
- **Painting and retouching:** Brush/Eraser size, hardness, opacity and smoothing,
  Shift straight lines, Blur, aligned Clone Stamp, three Spot Healing modes and
  Content-Aware Fill with Preview, Cancel and Apply. Healing/fill compile the
  original Mac 1.4.5 C kernels.
- **Text, shapes and effects:** point and fixed-box text, UTF-16 font/color runs,
  live shape redraw on resize, editable Stroke and Drop Shadow. Save/reopen,
  cancellation and undo preserve editable records. Other effects are rendered
  and preserved.

Large edits and histograms use background jobs with visible progress and
cooperative transfers. Packed RGBA reductions retain exact integer averages
and established undo/memory budgets.

## Shortcuts

| Action | Shortcut |
| --- | --- |
| Move / Type / Brush / Eraser | V / T / B / E |
| Marquee / Lasso / Magic Wand | M / L / W |
| Blur / Clone Stamp / Spot Healing | R / S / J |
| Gradient / Shape / Eyedropper | G / U / I |
| Crop / Hand / Zoom | C / H / Z |
| Copy / Copy Merged / Cut / Paste | Ctrl+C / Ctrl+Shift+C / Ctrl+X / Ctrl+V |
| Layer via Copy / duplicate | Ctrl+J |
| Select All / Deselect / Inverse | Ctrl+A / Ctrl+D / Ctrl+Shift+I |
| Fill foreground / background | Alt+Backspace / Ctrl+Backspace |
| Swap / reset palette colors | X / D |
| Levels / Curves / Hue-Saturation | Ctrl+L / Ctrl+M / Ctrl+U |
| Group / Ungroup / Merge | Ctrl+G / Ctrl+Shift+G / Ctrl+E |
| Apply / cancel transform or gradient | Enter / Escape |
| Apply / cancel text editing | Ctrl+Enter / Escape |
| Clone source | Alt-click |

## Mac interoperability and limits

Reads `.comp` versions 1–11 and writes version 11, as Mac 1.4.5 does.
Supported unknown manifest fields and text runs are preserved. The Mac
regenerates its Quick Look preview after a Windows save.

Nine complete PNG exports from the received Mac projects match every RGBA
byte; text/shapes remain editable and saved Stroke/Shadow parameters persist.
This validates those fixtures, not every possible document.

- Projects are limited to 100 megapixels of layer images and masks.
- Windows/Mac fonts and fallback can draw glyphs differently.
- Upright fractional 1:1 copies and measured affine enlargement now follow
  the Mac's sampled behavior. Rotated-edge/distortion resampling remains
  separate Phase 3.5d work.
- Very large blur/effect previews use reduced copies; research records describe
  their tolerances and padded-image limits.
- Liquify/Smudge are Phase 6. Camera Raw and PSD/RAW import are Phase 7.

## Validation

| Accepted local 0.8.0 checkpoint | Result |
| --- | --- |
| Rust workspace | 628 passed, zero failed, 10 existing ignored |
| TypeScript / Vitest | Three type checks; 287 unit tests |
| Complete functional browser suite | 199 passed; 29 performance opt-ins exercised separately |
| Complete original performance suite | 29/29; one worker, zero automatic retries |
| Native production UI | Eight original groups |
| Native Mac returns | 11 reads, 10 atomic saves, nine complete RGBA-exact PNGs |
| Native clipboard | Four protected protocols, 16 full image comparisons, original clipboard restored |

Assertions and budgets are unchanged. Historical failures, the initial menu
timeout and the independently repaired wrapper exit-status error remain
recorded. The current pass does not establish a universal intermittent fix.
See [the acceptance record](docs/superpowers/phase5-acceptance-2026-10-03.md).

Phase 6 development acceptance is complete. It has passed hosted source checks,
supplemental native WebView2 protocols and two independent 24/100 MP
responsiveness runs. Received
Mac 1.4.5 committed Liquify/Smudge PNGs match Windows WASM exactly; the styled
mask probe has a maximum one-level color difference and exact alpha. Windows
operator projects and PNGs have been verified. The rebuilt production WebView2
package also clears the picture and tool overlay after closing the last tab,
and renders an explicitly created new canvas afterward. Closing the last tab
shows an empty dark workspace. Exact-source CI passed 661 native, 295 unit
and 248 functional cases; existing ignores and opt-in skips are unchanged. See
the
[Phase 6 runtime checkpoint](docs/superpowers/research/phase6-runtime-checkpoint-2026-10-06.md).

## Build and test

Windows prerequisites: Rust **1.95.0** with `wasm32-unknown-unknown`, Node 22,
pnpm **10.34.5**, Visual Studio C++ Build Tools, LLVM (`clang` and `llvm-ar`)
and WebView2. Install `wasm-pack` with
`cargo install wasm-pack --version 0.15.0 --locked`; its pin lives in
`package.json` under `config.wasmPackVersion`.

```powershell
pnpm install --frozen-lockfile
pnpm wasm:dev             # development WASM
pnpm dev                  # browser with mock shell
pnpm tauri:dev            # native desktop development
pnpm wasm                # release WASM
pnpm build               # three type checks and frontend
pnpm test
cargo test --workspace --locked
pnpm exec playwright install chromium
pnpm exec playwright test --workers 1 --retries 0
pnpm build:portable
```

Portable outputs live in `build-artifacts/windows-x64/`. Packaging preserves
the original build-info file and refuses to overwrite an existing portable.

## Automatic releases

Windows CI runs on main/development pushes and pull requests. A new stable
version on `main`, a matching `vX.Y.Z` tag, or a manual Windows Release run
starts release validation. Keep `package.json`, the Cargo workspace version
and `src-tauri/tauri.conf.json` aligned; add `docs/releases/X.Y.Z.md`.

Original native/unit/functional gates run before a production portable build.
ZIP CRCs, staged bytes, hashes and the downloaded artifact's source receipt
are checked before publishing the ZIP, checksums and manifest. Published
versions remain immutable; an existing failed draft requires inspection.
Only the final publishing job has write permission. Actions are pinned to
reviewed commit SHAs.

Hosted CI does not replace local hardware timing, native clipboard or Mac
gesture acceptance; these remain separate recorded checks.

## Roadmap and credits

- [Design and scope](docs/superpowers/specs/2026-09-20-windows-port-design.md)
- [Phase 4/5 delivery](docs/superpowers/phase4-remaining-and-phase5-report.md)
- [Phase 6 plan](docs/superpowers/plans/2026-10-05-phase6-liquify-smudge.md)

Phase 6 development exposes Liquify/Smudge through **R**, then **Mode** in the
tool options. Blur remains the default; warp modes use Size, Hardness and
Strength. A dedicated worker owns each stroke's original snapshot and GPU
resources. Live previews retain layer effects, masks and selection, with a
bounded image on large layers; mouse-up commits the original resolution as
one history edit, subject to the existing 256 MiB undo budget. A 100 MP whole
layer replacement exceeds that budget and cannot retain its previous raster.
Escape, document/tool/mode switches and pointer cancellation
discard the preview. GPU-unavailable fallback is limited to 4 Mi pixels.

The raw kernels and complete tool have separate local 24/100 MP hardware
measurements. Two independent complete-tool runs met the unchanged response
budgets on aligned full-canvas fixtures. Packaged-runtime checks and received
Mac/Windows committed project/PNG checks are complete within the measured
boundaries above; see the
[manual checks](docs/superpowers/phase6-manual-checks.md).
This development checkpoint is not part of the published
0.8.0 release. See the runtime checkpoint for package/source identities and
retained failures; exact Metal/WebGL kernel equality is not claimed.

Original Mac sources and retouch kernels use MIT. Icons use
[Lucide](https://lucide.dev) under ISC; Apple's SF Symbols are licensed for
Apple platforms and are not used here.
