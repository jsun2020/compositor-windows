# Compositor for Windows

[![Windows CI](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml)
[![Windows Release](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml)

A free, open-source Windows image editor, ported from Compositor for macOS.
Built with Tauri 2, a Rust/WebAssembly engine, WebGL2 and React/TypeScript.

**Current release: 0.8.0. Phase 4 and Phase 5 are implemented and accepted on the
tested Windows desktop with received Compositor for Mac 1.4.5 projects.**
Liquify and Smudge are Phase 6 development work and are not part of 0.8.0.

## Download

Get the portable Windows x64 ZIP from [Releases](https://github.com/jsun2020/compositor-windows/releases/latest),
extract it and run `Compositor.exe`. Microsoft Edge WebView2 Runtime is required;
Rust and Node are only needed for development. Projects are `.comp` folders:
keep their manifest and asset files together.

Automated releases include `SHA256SUMS.txt` and `release-manifest.json` with
the exact source commit, build marker and artifact hashes.

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

Phase 6 development has CPU reference kernels and guarded single-stroke
editing jobs for Liquify/Smudge. GPU preview, tool controls, large-canvas timing
and Mac gesture comparisons are still in progress; these tools are not yet
exposed in the application.

Original Mac sources and retouch kernels use MIT. Icons use
[Lucide](https://lucide.dev) under ISC; Apple's SF Symbols are licensed for
Apple platforms and are not used here.
