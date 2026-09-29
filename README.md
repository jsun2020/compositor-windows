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
- Note: Hue/Saturation is evaluated per pixel rather than through the Mac's 33-point
  colour cube, so results are slightly more exact than the Mac app's.
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
- Motion Blur, as a layer or a filter, is Core Image's: a Gaussian along the angle whose sigma is
  the distance / sqrt(12), within 6 levels of the Mac's own render.
- Note: a Gaussian Blur or a Motion Blur reaching more than 48 screen pixels (three sigmas: a
  Motion Blur reaches 0.87 x its distance) is computed on a reduced copy. Measured against the
  exact blur: away from the edges, within 1 level; up to 4 levels for a Gaussian Blur and 9 for a
  Motion Blur along the canvas edge and along hard edges of transparency; and a long Motion Blur
  loses fine detail across its angle (up to 19 levels on pixel-sized noise). At export (100%), a
  Gaussian Blur up to radius 16 and a Motion Blur up to 55 px are exact. Every view of it, and
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

## Phase 4a: selections

- Marquee (M; Tab switches Rectangle and Ellipse), Lasso (L; Tab switches Freehand and
  Polygonal) and Magic Wand (W), with New, Add and Subtract in the options bar, or Shift (add)
  and Alt (subtract) held as a drag or click begins.
- The Magic Wand's tolerance, sample size (point, 3 by 3, 5 by 5), This Layer or All Layers, and
  Contiguous. Anti-alias for the Lasso, the Magic Wand and the elliptical Marquee.
- Drag inside a selection to move its outline; the arrow keys nudge it (Shift = 10 px). A click
  inside it without a drag deselects (with the Magic Wand, selects afresh from that pixel).
- The Polygonal Lasso: a click per corner; click the first corner or double-click to close;
  Backspace removes the last corner, Enter closes, Escape cancels.
- Select menu: All, Deselect, Inverse, Layer's Pixels, Mask's Black Areas, and Expand, Contract
  and Feather with an amount; the three are also in the options bar.
- Ctrl-click a layer's thumbnail to select its pixels (at least half opaque), or a mask's to
  select its black areas, as on the Mac; Ctrl+Shift adds and Ctrl+Alt subtracts.
- With a selection: adjustments, filters and Invert change only what is selected (a blur still
  grows the layer where the selection reaches), and Levels and Curves show the histogram of the
  selected pixels. Delete clears the selected pixels, or fills a targeted mask with its background
  colour (growing it to the canvas, as on the Mac 1.3.7). Add Mask hides the selection (Add Mask
  (Hide All) shows only it) and uses it up. The Crop tool starts at the selection's bounds.
  Adjustment layers ignore the selection.
- An empty selection (after Subtract or Contract) says so in the options bar, and every edit
  refuses it until it is deselected or replaced.
- Selections are part of undo and, as on the Mac, are never saved in the project. Crop, Canvas
  Size and Image Size drop the selection; Flip Canvas mirrors it.
- Note: dragging a Marquee or an outline past the window's edge does not scroll the view yet.
- Note: object selection, Select Subject, the clipboard and the brushes are not in this phase.

### Select menu shortcuts

| Action | Shortcut |
| --- | --- |
| Marquee / Lasso / Magic Wand | M / L / W |
| Switch the Marquee's shape or the Lasso's kind | Tab |
| Select All | Ctrl+A |
| Deselect | Ctrl+D |
| Inverse | Ctrl+Shift+I |
| Add to / subtract from the selection | Shift / Alt while drawing |
| Nudge the selection (selection tools) | Arrow keys (Shift = 10 px) |
| Clear the selected pixels | Delete |

## Phase 4b-1: colour, fills, gradients and shapes

- The palette at the foot of the tool rail: the foreground and background colours; X swaps them
  and D restores black over white. With a layer's mask targeted they are black and white, and a
  click on a swatch asks which ("Black - Hide" or "White - Reveal").
- The colour picker, opened from a swatch: a saturation and brightness field, a hue strip, R, G, B
  and hex, OK (Enter) and Cancel (Escape). It floats, opens where it was last left, and while it
  is open a click or drag on the canvas samples the colour under the pointer, with a ring showing
  the sampled colour over the one before.
- Eyedropper (I): a click or drag on the canvas sets the foreground colour from what the canvas
  shows. Alt with the Gradient tool does the same.
- Edit > Fill with Foreground Color (Alt+Backspace) and Fill with Background Color
  (Ctrl+Backspace): the selection, or the whole layer, which grows to cover the canvas as on the
  Mac; on a targeted mask its black or white, and the mask grows past its layer to cover the
  canvas, as Compositor 1.3.7 for Mac does.
- Gradient (G): drag a line; its ends can then be dragged (Shift holds 45 degrees), Enter or Apply
  paints it, Escape or Cancel drops it, and the first Undo discards it. Linear or Radial (Tab),
  Foreground to Background or to Transparent, Reverse, and Opacity (the digit keys set it). It
  previews from a reduced copy while dragged (a patch at full size inside a small selection) and
  is applied to the full layer; switching tool or layer applies it first. A gradient on a targeted
  mask grows it the same way.
- Shape (U; Shift+U or Tab steps Rectangle, Ellipse and Line): drag to draw the shape in the
  foreground colour on a new layer above the active one (Shift squares it or holds a line to 45
  degrees, Alt draws from the centre). Rectangles take a corner Radius, lines a Width. The layer
  keeps the Mac's shape record, so Compositor for Mac redraws it crisply when it is scaled there;
  this app scales its pixels.
- A new Gradient Map, as a layer or from the Image menu, starts from the foreground to the
  background colour, and its two ends open the colour picker, which previews them live.
- Large layers (over 4 megapixels) are edited, and their Levels and Curves histograms read, off the
  interface thread: "Working..." shows meanwhile. Edits inside a selection upload only the pixels
  they change, and undo keeps the last 100 steps, fewer once they hold more than 256 MB.
- Note: a shape is drawn as the Mac draws it (Core Graphics' anti-aliasing is approximated by exact
  area coverage); a scaled shape layer is not redrawn here.
- Note: the colour picker works in sRGB, 8 bits a channel, as the Mac's does.
- Note: Compositor for Mac 1.3 saves projects in format 11, which this version cannot open yet (it
  opens formats 1-9); Mac projects saved by 1.2.x open as before. Opening and saving 1.3 projects
  is the next update.

### Colour and tool shortcuts

| Action | Shortcut |
| --- | --- |
| Eyedropper / Gradient / Shape | I / G / U |
| Swap colours / default colours | X / D |
| Fill with the foreground / background colour | Alt+Backspace / Ctrl+Backspace |
| Gradient: Linear or Radial; Shape: next kind | Tab (Shape also Shift+U) |
| Gradient opacity | 1-9 for 10-90 %, 0 for 100 % |
| Apply / cancel a pending gradient | Enter / Escape |

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

- a layer enlarged in High quality (the default), which Compositor for Mac draws with Core
  Graphics' high-quality filter and this app bilinearly: sharper soft edges on the Mac, up to 29
  levels on one probe turned 25 degrees at 150 %, until Phase 3.5d ports the Mac's resampling
  filter (probe results, "Step probes");
- layer effects, which match the Mac's renders (11 of the 16 effects probes exactly, 4 within 3
  levels, and the last apart from the resampling above) but are written as the Mac's code writes
  them, not yet checked against a project the Mac itself saved with effects.

## Further reading

- Design specs: `docs/superpowers/specs`
- Implementation plans: `docs/superpowers/plans`

## Credits

The tool rail's icons are from [Lucide](https://lucide.dev) (ISC License; the notice is in
`app/src/panels/tool-icons.tsx`). The Mac app draws its tools with Apple's SF Symbols, which are
licensed for Apple platforms only, so this port uses the closest Lucide icons instead.
