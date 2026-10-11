# Compositor for Windows

[![Windows CI](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/ci.yml)
[![Windows Release](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml/badge.svg)](https://github.com/jsun2020/compositor-windows/actions/workflows/release.yml)

A free, open-source Windows image editor ported from Compositor for macOS, built with Tauri 2, Rust/WebAssembly, WebGL2 and React/TypeScript.

**Version 0.9.0 includes Phase 6 Liquify/Smudge, Phase 7 Camera Raw and Photoshop/camera RAW import, and the final Phase 3.5d sampling corrections.** The original v1 scope and its acceptance evidence are linked below. Published 0.8.0 artifacts remain unchanged.

## Download

Get the portable Windows x64 ZIP from [Releases](https://github.com/jsun2020/compositor-windows/releases/latest), extract the entire archive and run `Compositor.exe`. Microsoft Edge WebView2 Runtime is required. Node and Rust are needed only for development.

Keep `CompositorRaw.exe`, the three MSVC runtime DLLs and `LibRaw-notices` beside the application. Releases include `SHA256SUMS.txt` and `release-manifest.json` with artifact hashes, the build marker and exact source commit. Projects are `.comp` folders; keep their manifest and assets together.

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
- **Liquify and Smudge:** press **R** and choose **Mode** in the tool options. Size, Hardness and Strength control each stroke; Escape cancels it and mouse-up commits one undo step. Large live previews use bounded copies.
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


## Camera Raw and Photoshop import

- **Camera Raw filter:** Light, Color, parametric/point curves, Color Mixer, Point Color, Color Grading, Effects, Detail, Optics, Geometry and Calibration. Resets, group bypass, sampling and diagnostic previews preserve the original pixels until Apply. Apply records one undo step.
- **PSD/PSB:** bounded RGB 8-bit raw/PackBits decoding, folders, visibility/opacity, supported blend modes and clipping, raster masks, supported adjustments, first-style editable text and supported live shapes. Text keeps its cached Photoshop raster until edited. Unsupported records are reported; ZIP compression, 16/32-bit and non-RGB documents are refused.
- **Camera RAW:** pinned LibRaw 0.22.2 sensor development, As Shot reset, exposure, estimated temperature, tint, tone curve, bounded preview and cancellation. An embedded JPEG is never substituted for sensor processing. LibRaw and Apple CIRAWFilter can produce different colour and tone; supported cameras depend on the decoder.

For modified projects, **Save and Close** saves first, **Don't Save and Close** discards changes, and **Cancel Close** keeps the project open. Cancelling the Save As picker also keeps it open. Closing the last tab clears the canvas and shows an empty dark workspace; use File > New to create another canvas.

## Interoperability and limits

Reads `.comp` versions 1–11 and writes version 11, as Compositor for Mac 1.4.5 does. Supported unknown manifest fields and text runs survive a round trip. Mac regenerates its Quick Look preview after a Windows save.

- Layer images and masks are limited to 100 megapixels. History retains at most 256 MiB of distinct rasters; a whole 100 MP replacement cannot retain its previous raster for Undo.
- Windows/Mac fonts and fallback can draw different glyphs.
- CG affine rotation, reduction and mask sampling are checked against independently returned synthetic Mac images. All ordinary affine and ten convex-distortion RGBA records are byte exact; six mask-related records differ by at most one byte.
- The four fixed Camera Raw recipes, including Geometry/Calibration, match the Mac exports byte for byte. The [perspective arithmetic reference](engine/tests/fixtures/core-image-perspective.md) records the measured hardware and exponent scope; those results do not establish equality for every possible image or GPU.
- Large blur/effect and warp previews use reduced images. Liquify/Smudge without a GPU uses a fallback limited to 4 Mi pixels. Exact Metal/WebGL retouch-kernel equality is not claimed.
- The v1 scope excludes Vision-based selection, HEIC, auto-update, installers/signing and single-file project containers.

## Validation

Source tests, hardware timing, production WebView2, native clipboard and actual Mac execution are separate gates. Hardware checks use one worker, zero retries and the original timing assertions/budgets. Historical failures and hardware qualifications remain in the acceptance records.

- [Phase 4/5 delivery](docs/superpowers/phase4-remaining-and-phase5-report.md) and [Phase 5 acceptance](docs/superpowers/phase5-acceptance-2026-10-03.md).
- [Phase 6 runtime acceptance](docs/superpowers/research/phase6-runtime-checkpoint-2026-10-06.md).
- [Phase 7 plan and evidence](docs/superpowers/plans/2026-10-06-phase7-camera-raw-and-import.md).
- [Final sampling and stable-release gates](docs/superpowers/plans/2026-10-09-phase35d-and-final-release.md).

## Build and test

Windows prerequisites: Rust **1.95.0** with `wasm32-unknown-unknown`, Node 22, pnpm **10.34.5**, Visual Studio C++ Build Tools, LLVM (`clang` and `llvm-ar`) and WebView2. Install the pinned WASM tool with `cargo install wasm-pack --version 0.15.0 --locked`.

```powershell
pnpm install --frozen-lockfile
pnpm wasm:dev             # development WASM
pnpm dev                  # browser with mock shell
pnpm tauri:dev            # native desktop development
pnpm wasm                # production WASM plus binary/export verification
pnpm build               # three type checks and frontend build
pnpm test
cargo test --workspace --locked
pnpm exec playwright install chromium
pnpm exec playwright test --workers 1 --retries 0
pnpm build:portable
```

Portable outputs are in `build-artifacts/windows-x64/`. Packaging restores the original build-info file and refuses to overwrite an existing portable. The WASM gate rejects invalid/truncated binaries and JavaScript/export mismatches, including when packaging with `-SkipWasm`.

## Automatic releases

CI runs on main/development pushes and pull requests. A new stable version on `main`, its matching `vX.Y.Z` tag, or a manual Windows Release run starts validation. Keep package, Cargo and Tauri versions aligned and add `docs/releases/X.Y.Z.md`.

Native/unit/functional tests run before the production portable build. ZIP CRCs, required decoder/runtime/license files, hashes and the downloaded artifact's source receipt are verified before publication. Published releases are immutable. Only the publishing job has write permission; actions are pinned to reviewed commits. Hosted CI does not replace local hardware, native clipboard or Mac evidence.

## Scope and credits

[Original design and scope](docs/superpowers/specs/2026-09-20-windows-port-design.md). Original Mac sources and retouch kernels use MIT. Icons use [Lucide](https://lucide.dev) under ISC; Apple's SF Symbols are not used in this Windows port.
