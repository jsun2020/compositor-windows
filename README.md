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
