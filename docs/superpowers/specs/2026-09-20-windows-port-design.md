# Compositor for Windows: design

Date: 2026-09-20
Status: approved design, awaiting implementation plan

## 1. Purpose

Compositor is a free, open-source image editor for macOS built around a Photoshop-style compositing workflow. This project delivers a Windows version so that everyday image editing (crop, resize, layers, adjustments, retouching) no longer requires launching Photoshop, which stays reserved for complex work.

The macOS app is 23k lines of Swift on AppKit, SwiftUI, Core Image, Metal and Vision, plus eight small C pixel kernels. None of the Swift or Apple frameworks run on Windows, so this is a rewrite of the feature set, not a port of the code. The macOS source, its 48 unit-test files and `docs/project-format.md` serve as the behavioural oracle.

## 2. Goals and non-goals

Goals:

- Feature parity with the macOS app for the four areas listed in section 3, delivered in usable phases.
- Open and save the same `.comp` folder packages as the macOS app so projects move between machines. This tracks the macOS app's CURRENT format, not a fixed version: at the time of writing that is Compositor 1.2.6, which writes format version 9 (the port was begun against a v7 snapshot; see Phase 3.5). Opening a file must never silently lose data on re-save, and anything the port cannot yet render must be preserved verbatim.
- Ship as a portable zip (no installer) built by a PowerShell script, matching the user's other Windows desktop apps.
- Smooth interactive painting and live adjustment previews on canvases up to the macOS limits (30,000 px per side, 100 million source pixels).

Non-goals for v1:

- Remove Background. The macOS app uses Apple Vision; Windows would need a bundled ONNX segmentation model. Deferred.
- HEIC import. Windows has no guaranteed HEIF codec. Supported imports: JPEG, PNG, TIFF, WebP, BMP.
- Auto-update (Sparkle on macOS). No updater in v1.
- Dragging layers between open projects.
- A single-file project container. Projects are folder packages only, for macOS interoperability.

## 3. Scope and phases

Each phase is independently usable and ends with a portable build.

Phase 1, canvas and files:
new canvas, import images (as new project or as a layer), open and save `.comp`, export JPEG (with quality preview) and PNG, crop with snapping, Canvas Size, Image Size with resolution, Flip Canvas, zoom and pan, pixel grid when zoomed in, sharp downsampling when zoomed out, multiple projects in tabs.

Phase 2, layers:
layer stack with folders, visibility, opacity, blend modes (Normal, Multiply, Screen, Overlay, Darken, Lighten, Difference, Color Dodge, Color Burn), raster layer masks and folder masks (paint, fill, invert, blur, feather, link/unlink), clipping masks, non-destructive move, scale, rotate, flip and free distort with snapping and guides, exact transform values, transform several layers or a folder together, merge down, merge layers, merge group, duplicate, rename, reorder and nest by drag, undo and redo.

Phase 3, adjustments and filters:
Levels with Auto, Curves, Hue/Saturation, Exposure, Gradient Map, Grain, Invert, Gaussian Blur and Motion Blur that spread past layer edges, Add Noise, Lens Correction. Live previews, limited to the selection when there is one. Adjustment layers for Hue/Saturation, Levels, Curves, Exposure, Gradient Map and Grain.

Phase 3.5, compatibility with macOS Compositor 1.2.6 (added 2026-09-24):
open and save project format versions 8 and 9 exactly as the macOS app does. Folder opacity and saved guides (v8); Gaussian Blur, Motion Blur and Add Noise as adjustment layers (v9); the ungated additions Invert, Black & White and Color Balance adjustment layers, eleven new blend modes, and layer effects (stroke and drop shadow) rendered as the Mac renders them; `text`, `shape` and any unrecognised fields preserved verbatim on re-save; and the 1.2.6 changes to existing kernels (Grain's roughness detail, Add Noise's hash) so v1-v7 files also render as the current Mac renders them. The research behind this phase is `docs/superpowers/research/mac-1.2.6-format-delta.md`, and its ground truth is a project saved by the Mac app (`engine/tests/fixtures/mac-1.2.6/`). Editing text and effects is Phase 5.

The behavioural oracle from Phase 3.5 onward is the user's fork at Compositor 1.2.6 (https://github.com/jsun2020/Compositor, cloned beside this repository as `Compositor-1.2.6`), not the earlier v7 snapshot.

Phase 4, selections and retouching:
Rectangle and Ellipse Marquee, Freehand and Polygonal Lasso, Magic Wand, add and subtract, move outline, move and duplicate pixels inside, load layer or mask as selection, Brush with size, hardness and opacity and Shift for straight lines, Spot Healing Brush, Clone Stamp (aligned or not, current layer or all), Blur tool, Content-Aware Fill including extending past edges, Gradient tool, Shape tool (rectangle, rounded rectangle, ellipse), Eyedropper, full colour picker, Copy Merged, Photoshop-style keyboard shortcuts. Selection-limited adjustments and filters (the coverage path Phase 3 built but always passed `None`). Re-scoped against Compositor 1.2.6 before planning: its object selection and floating selection are candidates.

Phase 5, text and layer effects (added 2026-09-24 at the user's request): the type tool and editable text layers, live shape layers, and editing layer effects (stroke, drop shadow). Phase 3.5 already renders and preserves all three.

Phase 6, Liquify and Smudge (added 2026-09-24): the Smear tool's Liquify, Blur and Smudge modes.

Phase 7, Camera Raw, PSD and RAW (added 2026-09-24): the raw develop panel, and PSD and RAW import.

## 4. Architecture

Three units with one job each. Repository layout:

```
compositor-windows/
  engine/          Rust crate `compositor-engine`: documents, pixels, ops, codecs
  engine-wasm/     Rust crate: wasm-bindgen wrapper exposing the engine to the UI
  app/             Vite + React + TypeScript UI, WebGL2 renderer, wasm loader
  src-tauri/       Tauri 2 shell: window, dialogs, file bytes, recent files
  scripts/         build-windows-x64.ps1 (portable zip), helpers
  docs/            specs, format notes, performance notes
```

### 4.1 Engine (`engine/`)

A pure Rust library with no Tauri, browser or GPU dependency. It compiles natively for `cargo test` and to `wasm32-unknown-unknown` for the app. Responsibilities:

- Document model: canvas size, resolution, layer tree (layers and groups), per-layer transform (origin, size, rotation, flips, sampling), opacity, blend mode, raster mask, mask enabled flag, clipping-mask source, active layer, selection.
- Tiled raster storage: every layer's pixels and mask live in 256 by 256 tiles of premultiplied RGBA8 (masks are 8-bit grey). Tiles are immutable and reference-counted; writes copy the touched tile. Blank tiles are not allocated.
- History: a command log. Each entry stores the command and the tiles or metadata it replaced, so undo of a brush stroke costs only the touched tiles. Undo and redo are session-only, as on macOS.
- Operations: every pixel and structural operation from section 3, implemented as a `Command` enum (serde-serialisable) executed by `Engine::execute`. The UI and the tests speak the same command vocabulary.
- Kernels: the eight macOS C kernels (adjust, brush, content fill, heal, lens, levels, noise, wand) ported line-for-line to Rust. Porting rather than compiling C keeps one toolchain and identical native and wasm behaviour. Phase 3 ported adjust, lens, levels and noise into `engine/src/adjust/` (see 4.5); brush, content fill, heal and wand arrive with Phase 4's selection and retouching tools. (An earlier draft named a single `engine/src/kernels/` folder; the kernels live with the feature that uses them instead.)
- CPU compositor: composites any document region to RGBA8. It is the reference implementation used by export, by tests, and by the WebGL fallback.
- Codecs: `.comp` manifest parsing and validation for versions 1 to 7 with the limits in `docs/project-format.md`; PNG and JPEG decode and encode with resolution metadata; TIFF, WebP and BMP decode via the `image` crate.

Public surface (sketch):

```
Engine::new() -> Engine
Engine::new_document(width, height, resolution) -> DocId
Engine::open_package(manifest: &str, images: &[(name, bytes)]) -> Result<DocId, OpenError>
Engine::save_package(doc) -> PackageEntries { manifest, images }
Engine::import_image(doc, bytes) -> Result<LayerId, ImportError>
Engine::execute(doc, Command) -> Result<Dirty, CommandError>   // Dirty = list of tile coords per layer + metadata changed flag
Engine::undo(doc) / redo(doc) -> Dirty
Engine::tile(doc, layer, tx, ty) -> Option<&[u8]>              // zero-copy view for the renderer
Engine::composite(doc, rect, scale) -> Vec<u8>                  // CPU reference
Engine::export_png(doc) / export_jpeg(doc, quality) -> Vec<u8>
Engine::state(doc) -> DocumentState                             // layer tree, selection, active layer, history counts
```

### 4.2 Wasm wrapper (`engine-wasm/`)

A thin `wasm-bindgen` layer. It keeps one `Engine`, marshals `Command` and `DocumentState` as JSON, and exposes tile pointers plus lengths so the UI builds `Uint8Array` views directly on wasm memory without copying. Single-threaded in v1. Heavy operations (Spot Healing, Content-Aware Fill, large blurs) are chunked so the UI can show progress and stay responsive. Running the engine in a Web Worker over shared memory is a documented upgrade path, not v1 work.

### 4.3 UI and renderer (`app/`)

Vite, React 18, TypeScript, Tailwind, matching the user's other Tauri apps. Structure:

- `src/engine/`: loads the wasm module and wraps it in a typed client. Owns the dirty-tile queue.
- `src/canvas/`: the WebGL2 renderer. Each layer's tiles are uploaded to a texture array; composition runs bottom to top through ping-pong framebuffers with one shader per blend mode; masks, folder masks and clipping-mask coverage multiply alpha in the shader; groups render to an intermediate framebuffer and are then blended as one. A view transform draws the composite with mipmapped downsampling when zoomed out and nearest sampling plus a pixel grid when zoomed in. Overlays (selection marching ants, transform handles, guides, brush cursor, crop frame) draw in a separate pass.
- `src/canvas/cpu-fallback.ts`: when WebGL2 is unavailable, asks the engine for `composite(viewRect, scale)` and blits it to a 2D canvas. Everything works, only slower.
- `src/panels/`: layers panel, tool options header, adjustment and filter sheets, colour picker, transform inspector, project tabs. Layout mirrors the macOS app.
- `src/shell/`: the file bridge. In Tauri it calls shell commands; in a plain browser (for Playwright) it uses an in-memory mock.
- `src/shortcuts/`: Photoshop-style key map.

Data flow is one direction: gesture, command, engine mutation, dirty tiles, texture upload, redraw. React state holds only `DocumentState` snapshots and tool UI state, never pixels.

### 4.4 Tauri shell (`src-tauri/`)

Tauri 2 with the dialog plugin. Commands:

- `pick_open_package`, `pick_save_package`: folder pickers for `.comp` packages. Open also accepts a file picker on `manifest.json` inside a package, and dropping a `.comp` folder on the window.
- `read_package(path) -> { manifest, images: [(name, bytes)] }` with the format size limits enforced before bytes reach the engine.
- `write_package(path, entries)`: writes to `<name>.comp.tmp-<uuid>`, then swaps: rename existing to `.bak`, rename tmp to final, delete `.bak`. A failure leaves either the old or the new package intact.
- `read_image(path) -> bytes`, `write_file(path, bytes)` for import and export, using the file picker.
- `recent_packages` stored in the app data directory.

The shell never decodes or touches pixels. The window uses the fixed `tauri://localhost` origin so local storage is stable across launches (LL-036).

### 4.5 Adjustments (`engine/src/adjust/`)

The macOS adjustment and filter kernels (Levels, Curves, Hue/Saturation, Exposure,
Gradient Map, Grain, Invert, Gaussian Blur, Motion Blur, Add Noise, Lens Correction)
are ported to Rust under `engine/src/adjust/`, shared by the CPU compositor and by the
live preview path. The WebGL2 renderer does not re-derive the colour tables: the engine
computes them and uploads them as textures (`adjustment_lut`, `hue_response_table`), so
only the HSL and grain arithmetic is written twice, once in Rust and once in GLSL, and an
end-to-end GPU/CPU parity suite holds the two within 2/255 (3/255 for Grain). Two deliberate simplifications from
the macOS behaviour:

- Hue/Saturation is evaluated per pixel rather than through the Mac's 33-point colour
  cube, so results are slightly more exact than the Mac app's.
- Motion Blur is an even streak rather than Core Image's tapered one.

## 5. Project format on Windows

The Windows app reads and writes the macOS folder package unchanged: a `<name>.comp` directory containing `manifest.json` and `images/<layer UUID>.png` plus `<layer UUID>.mask.png`. The macOS source (`ProjectStore.swift`) is ahead of `docs/project-format.md`: it reads versions 1 to 7 and writes version 7, where version 7 adds per-layer `adjustment`, `maskPlacement`, `maskLinked` and `shape` records. The Windows app reads 1 to 7 and writes 7. `shape`, whose feature arrives in a later phase, is preserved verbatim through open and save. `adjustment` is parsed since Phase 3: every field the Mac writes for its six kinds is read and written, but a key inside an adjustment that this version does not know is dropped on re-save, and an unknown `kind` makes the project refuse to open. Swift encodes the Hue/Saturation `adjustments` and `bands` maps (keyed by a `String` enum that is not `CodingKeyRepresentable`) as arrays of alternating key and value; the manifest reads that form (and the object form early 0.3.0 builds wrote) and writes it in `ColorRange` declaration order, so the order can differ from a Mac save of the same settings, whose order is hash-seeded. Validation rejects unsupported versions, invalid metadata, missing assets, unsafe paths and oversized data before the live document is replaced, exactly as `ProjectStore.swift` does. Round-trip tests open every fixture version and re-save it.

JSON encoding follows the structure of Swift's Codable output so either app parses the other's manifest: UUIDs are uppercase hyphenated strings, `origin` is a two-element array `[x, y]`, `size` is `[width, height]`, enum values are their display strings (`"High quality"`, `"Color Dodge"`), absent optionals are omitted, keys are sorted and the JSON is pretty-printed. Whitespace need not match byte for byte.

Undo history and viewport are session-only. Opening fits the canvas, restores the selection and starts with clean history.

## 6. Performance and limits

- Limits match macOS: 30,000 px per side, 100 million source pixels plus 100 million mask pixels, 10,000 layers, 64 nesting levels, 4 MiB manifest, 512 MiB per encoded asset.
- Tiles are 256 px, copy-on-write. A brush step touches only the tiles under the dab and re-uploads only those textures.
- Live adjustment previews run the adjustment on the visible tiles first, then the rest, so sliders feel immediate on large canvases.
- Downsampled display uses GPU mipmaps; the export path uses the engine's high-quality resampler so exports are not affected by the viewer.
- Target: brush at 60 fps on a 4000 by 3000 canvas with 20 layers on an integrated GPU; open and save of a 100 MP project under 10 s.

## 7. Error handling

- Engine commands return typed errors (`CommandError`, `OpenError`, `ImportError`). Nothing panics across the wasm boundary; the wrapper converts panics into a fatal error that offers to save a recovery copy.
- File errors surface with the path and the OS message. The app has no network access.
- Package validation errors name the failing rule (version, missing asset, limit exceeded) so a user can tell a corrupt file from an unsupported one.
- Out-of-memory in wasm (4 GiB address space) is prevented by enforcing the pixel limits before allocation, not caught after.

## 8. Testing

- Engine: `cargo test`. The 48 macOS test files are the oracle; each phase ports the tests for its features (layers, masks, transforms, levels, curves, hue/saturation, selection, brush, clone stamp, spot healing, crop, canvas size, image size, export, project round-trip). Pixel tests compare against expected values with the same tolerances the Swift tests use.
- Renderer: a golden test composites fixture documents with the WebGL2 renderer and with the CPU compositor and asserts a per-channel tolerance, so the GPU path cannot drift from the reference.
- UI: vitest for tool state, shortcut map and panel logic.
- End to end: Playwright drives the UI in a plain browser with the mocked file bridge (open fixture, paint, adjust, export, compare hash). A final smoke test launches the portable exe.
- Build verification: the app embeds a version marker; the build script asserts the marker is present in the bundled assets before zipping (per the project rule that source-committed is not deployed).

## 9. Build and packaging

- Toolchain: Rust 1.95 stable with `wasm32-unknown-unknown`, Node 22, pnpm, `@tauri-apps/cli` 2.x as a dev dependency, `wasm-pack` pinned as a dev dependency and driven from `package.json` scripts.
- `pnpm dev` runs Vite with the engine in a browser; `pnpm tauri dev` runs the shell.
- `scripts/build-windows-x64.ps1` builds the wasm engine, the Vite bundle and the Tauri release binary, verifies the version marker, and zips `Compositor-portable-<version>-<date>.zip` under `build-artifacts/windows-x64/`, matching the layout of superai-agent so the rebuild-portable skill applies.
- Requires the WebView2 Evergreen runtime, present on Windows 10 19045 and later.
- No installer, no code signing, no updater in v1.

## 10. Decisions taken

- Stack: Tauri 2 + Rust engine as WebAssembly + WebGL2 renderer. Chosen over a native Rust engine with IPC (too slow for painting) and over native wgpu into a child window (fragile on Windows, untestable in a browser).
- C kernels are ported to Rust, not compiled to wasm.
- Folder `.comp` packages only, for macOS interoperability. No zip container.
- Separate repository from the macOS app, as a sibling folder `compositor-windows`.
- Engine single-threaded on the main thread in v1; worker and shared memory later if profiling demands it.

## 11. Risks

- WebGL2 disabled by corporate policy: mitigated by the CPU fallback renderer.
- wasm memory ceiling: mitigated by enforcing pixel limits before allocation.
- Content-Aware Fill and Spot Healing are CPU-heavy: mitigated by chunked execution with progress; a worker is the upgrade path.
- Feature breadth: mitigated by phased delivery with a portable build at the end of each phase.
