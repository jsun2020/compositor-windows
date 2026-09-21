# Compositor for Windows

Compositor is a free, open-source image editor for macOS built around a Photoshop-style
compositing workflow (crop, resize, layers, adjustments, retouching). This project is a
Windows rewrite of that app, built with Tauri 2, a Rust/wasm image-compositing engine, and
a React/TypeScript UI, so everyday image editing on Windows no longer requires launching
Photoshop.

## Phase 1 features

- New canvas, and image import as a new project or as a layer
- Open and save `.comp` project packages
- Export PNG, and JPEG with a quality preview
- Crop with snapping, Canvas Size, Image Size with resolution, Flip Canvas
- Zoom and pan, pixel grid when zoomed in, sharp downsampling when zoomed out
- Multiple projects in tabs
- Keyboard shortcuts and drag-and-drop import

## Phase 2: layers

- Layer stack with folders, drag reorder and nest
- Opacity and 13 blend modes (Normal, Multiply, Screen, Overlay, Darken, Lighten,
  Difference, Color Dodge, Color Burn, Hue, Saturation, Color, Luminosity)
- Layer and folder masks: invert, fill (white or black), blur/feather, link/unlink
- Clipping masks
- Non-destructive move, scale, rotate, flip and free distort, with snapping
- Transform several layers or a whole folder together
- Merge down, merge layers, merge group
- Duplicate
- Note: mask painting arrives with the brush tool in Phase 4; today masks are edited
  through fill, invert and blur only.
- Note: Alt-dragging a layer on the canvas to duplicate it is two undo steps
  (duplicate, then move), not one.

### Layer menu shortcuts

| Action | Shortcut |
| --- | --- |
| New Layer | Ctrl+Shift+N |
| Duplicate | Ctrl+J |
| Group | Ctrl+G |
| Merge | Ctrl+E |
| Clipping Mask | Ctrl+Alt+G |
| Bring Forward | Ctrl+] |
| Send Backward | Ctrl+[ |
| Cycle blend mode forward | Shift+= |
| Cycle blend mode backward | Shift+- |
| Set layer opacity (move tool) | 0-9 (10% steps; two digits combine, e.g. 2 then 5 for 25%) |
| Nudge selection | Arrow keys (Shift = 10 px) |
| Apply transform | Enter |
| Cancel transform | Escape |

## Prerequisites

- Rust 1.95 with the `wasm32-unknown-unknown` target
- wasm-pack 0.15.0, installed with `cargo install wasm-pack --version 0.15.0 --locked`.
  It is not an npm dependency: the npm package's postinstall step downloads a binary from
  GitHub, which the corporate proxy this project runs behind may block. The pinned version
  lives in `package.json`'s `config.wasmPackVersion`; `pnpm wasm` and `pnpm wasm:dev` run
  `scripts/ensure-wasm-pack.ps1` first and fail with the install command above if the
  `wasm-pack` on PATH does not match.
- Node 22 and pnpm
- Visual Studio Build Tools with the C++ (VC.Tools.x86.x64) workload
- Microsoft Edge WebView2 Runtime (preinstalled on Windows 10 19045 and later)

## Commands

```
pnpm install         # install dependencies
pnpm wasm:dev         # build the wasm engine (dev profile)
pnpm dev              # run the app in a browser against a mock shell
pnpm tauri:dev        # run the full desktop app in dev mode
pnpm test             # run vitest unit tests
pnpm e2e              # run Playwright end-to-end tests
cargo test            # run the Rust engine and shell test suites
pnpm build:portable   # build and package the portable Windows zip
```

## Project file interoperability

Projects are `.comp` folder packages, compatible with Compositor for macOS. The macOS app
currently writes format version 7; this app reads format versions 1 through 7 and writes
version 7, so projects created or edited by either app can be opened by the other.

## Further reading

- Design specs: `docs/superpowers/specs`
- Implementation plans: `docs/superpowers/plans`
