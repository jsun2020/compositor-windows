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

## Phase 3: adjustments and filters

- Levels, with Auto and the three eyedroppers (black point, gray point, white point)
- Curves
- Hue/Saturation, with seven colour ranges (Reds, Yellows, Greens, Cyans, Blues,
  Magentas, and the master range), Colorize, and the band eyedroppers
- Exposure
- Gradient Map
- Grain
- Invert
- Gaussian Blur and Motion Blur, both spreading past the layer's edges
- Add Noise
- Lens Correction
- Live previews for every adjustment: the document is untouched until OK; Cancel
  restores it exactly
- Adjustment layers for all six kinds above that support them (Levels, Curves,
  Hue/Saturation, Exposure, Gradient Map, Grain): maskable, clippable, and reaching
  everything beneath them in the stack unless limited by an enclosing folder's mask
  (a folder does not isolate its contents on its own). Double-clicking an adjustment
  layer's row reopens its panel; double-clicking an ordinary layer's row still renames it.
- Note: an adjustment layer is never a clipping source.
- Note: selection-limited adjustments arrive with selections in Phase 4.
- Note: Hue/Saturation is evaluated per pixel rather than through the Mac's 33-point
  colour cube, so results are slightly more exact than the Mac app's.
- Note: Motion Blur is an even streak rather than Core Image's tapered one.
- Note: Image > Grain places its grain in document pixels; the Mac's uses the layer's own
  pixels, so on a scaled layer the grain size differs from the Mac's by the layer's scale.

### Image and Filter menu shortcuts

| Action | Shortcut |
| --- | --- |
| Levels | Ctrl+L |
| Curves | Ctrl+M |
| Hue/Saturation | Ctrl+U |
| Invert | Ctrl+I |

## Phase 3.5: Compositor for Mac 1.2.6 and later

- All 24 of the Mac's blend modes, in its menu order. New: Linear Burn, Linear Dodge (Add),
  Soft Light, Hard Light, Vivid Light, Linear Light, Pin Light, Hard Mix, Exclusion, Subtract
  and Divide. As on the Mac, an adjustment layer in any mode but Normal blends its result at
  full strength and keeps the alpha of what lies beneath it; in the eight modes the Mac computes
  with Core Image, that blend is Normal. A clipped group in one of those eight composites as
  Normal, as on the Mac.
- Adjustment layers for Add Noise, Gaussian Blur, Motion Blur, Invert, Black & White and
  Color Balance, from Layer > New Adjustment, with panels (Invert has nothing to set). Blur
  layers blur everything beneath them, fading at the canvas edge as the Mac's do.
- Image > Black & White and Color Balance, applied to the selected layer.
- Grain and Add Noise use the Mac 1.2.6 patterns, for layers and for the Image and Filter menus.
- Folder opacity and saved guides, as the Mac draws and moves them.
- Note: Motion Blur, as a layer or a filter, is an even streak rather than Core Image's taper.
  Until that difference is measured, a project with a Motion Blur adjustment layer names it in
  the notice as drawn approximately, and merging it is refused.
- Note: a Gaussian Blur reaching more than 48 screen pixels, or a Motion Blur reaching more than
  12, is computed on a reduced copy. Measured against the exact blur: away from the edges,
  within 1 level for a Gaussian Blur and 2 for a Motion Blur; up to 4 levels along the canvas
  edge and along hard edges of transparency; and a long Motion Blur loses fine detail across the
  streak (up to 20 levels on pixel-sized noise, and 17 at the canvas edge). At export (100%), a
  Gaussian Blur up to radius 16 and a Motion Blur up to 24 px are exact. Every view of it, and
  the export, shows the same result. Zoomed far into a very large blur, its edge can show at the
  window's edge.
- Note: projects are limited to 100 megapixels of layer images (and of masks). Compositor for
  Mac allows more on a Mac with more memory; such a project is refused with a message saying so.
- Layer effects, drawn as the Mac draws them: stroke (outside or inside), drop shadow, inner
  shadow, outer glow, inner glow and colour overlay, on the canvas and in every export. Merging
  bakes them, and every eyedropper and histogram that reads the picture sees them. They cannot be
  edited yet.
- Note: an effect whose blur reaches more than 48 pixels (a shadow blur or a glow size above 32)
  is computed on a reduced copy, within 1 level of the exact result. A layer whose effects would
  need an image of more than 200 megapixels is drawn without them, as on the Mac.
- Note: Image Size scales a layer's effects with it, and Canvas Size and Crop keep them.
  Compositor for Mac removes them in Image Size, Canvas Size, Crop and Trim.
- Note: deleting a layer that others clip to, with Bake, keeps its stroke and shadow in the
  baked pixels, as the canvas showed them; Compositor for Mac bakes without them.
- Note: an unknown key inside an effect is kept and named in the notice; Compositor for Mac
  drops it.
- Note: a layer with effects is drawn from an image made when its pixels, mask or effects change,
  and on every step of dragging its unlinked mask. On a large layer that takes a few seconds:
  measured at about 3 s for a pixel edit and 5 s for each frame of an unlinked mask drag, on a
  3000 x 2000 layer with all six effects. The 8 most recently made images, up to 512 MB, are
  kept.

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

Projects are `.comp` folder packages, compatible with Compositor for macOS. This app opens
projects from Compositor for Mac 1.2.10 (format version 9) and every earlier format (1 to 9),
writes version 9 as the Mac does, and saves them back without losing anything. What they
contain is drawn as the Mac draws it, within the reduced-copy blur note above, except:

- Motion Blur adjustment layers, which are drawn approximately and named in the same notice;
- until follow-up measurements on the Mac are back: Color Burn and Color Dodge on an adjustment
  layer or a clipped group, where the Mac uses Core Graphics' own formulas and this app the
  W3C ones; and Soft Light, whose exact variant is not settled yet (within 1 level on the only
  measurement so far); and the layer effects other than the drop shadow, which follow the Mac's
  code and its own tests but have not been compared with a Mac render yet, and are written as
  the Mac's code writes them, not yet checked against a project the Mac itself saved with
  effects.

## Further reading

- Design specs: `docs/superpowers/specs`
- Implementation plans: `docs/superpowers/plans`
