# Mac Compositor 1.2.10: delta against 1.2.6

Read-only research. Nothing in either Mac tree or in the port was modified.

- OLD = `C:\Users\sr9rfx\.claude-project\Compositor-1.2.6` (commit 5d6be39, "Publish update feed for Compositor 1.2.6").
- NEW = `C:\Users\sr9rfx\.claude-project\Compositor-1.2.10` (git worktree at 4306206, "Publish update feed for
  Compositor 1.2.10", upstream robbietilton/Compositor main).
- PORT = `C:\Users\sr9rfx\.claude-project\compositor-windows`.
- Companion: `mac-1.2.6-format-delta.md` (called "the 1.2.6 research" below; its section numbers are quoted as
  "1.2.6 research 3.4" and so on).

All `file:line` citations without a prefix are in NEW under `Compositor/`. OLD and PORT citations say so. Every diff
below was taken between the two commits with `git diff 5d6be39 4306206`, so the CRLF difference noted in the 1.2.6
research does not arise here.

## 0. Scope

37 commits and 55 files (2,280 insertions, 189 deletions). By release:

| Release | Version commit | What landed |
|---|---|---|
| 1.2.7 | d302214 | Live text colour preview while the colour picker is open (3317d02, 4e5f4ec); title-bar drag fixed (e8ed83b); tab-strip fade steady (1fb6a3f); `docs/project-format.md` brought up to v9 (156c38c, 4455a72); CI runs unit tests (f0ccb6e) |
| 1.2.8 | 092146a | `DocumentLimits`: a separate whole-document pixel budget, scaled to the Mac's memory, and a 200 MP single-surface cap (7dadc88, bc865fb) |
| 1.2.9 | a2160fa | PSB import (b8e3cd1); Photoshop text imported as editable text (b8cce6e, fa2b342); background-only PSD/PSB imported from its merged image (e6bcfa1); double-click live text with the Move tool to edit it (a5721b2); Camera Raw grading wheels drawn in the right hue direction (96edd5f) |
| 1.2.10 | 17d2454 | Clicked text starts on its first baseline at the pointer; pointer stays visible while typing (292cb6b, 072fa08; committed after the 1.2.9 version bump, before its feed); oversized PSD layers cropped to the canvas when the file would not fit (1fcf854); marching-ants level of detail (8eba077); SVG import (0534e00) |

The rest is README, appcast, Xcode project and tests.

**Short answer:** 1.2.10 changes no Codable shape, no version number, no rendering maths and no draw order. The
only format-visible change is that the size ceilings were raised and one of them now depends on the Mac's RAM
(section 1.3). Everything else is import features, text-tool placement and UI.

## 1. Project format

### 1.1 Version: unchanged at 9

- `static let current = 9` (IO/ProjectStore.swift:15), `supported = 1...ProjectManifest.current` (:18),
  `var version = ProjectManifest.current` (:21). The whole ProjectStore.swift diff is three things: the
  `photoshopLargeImage` and `.svg` import types (:9-10), the `.tooLarge` message (:71) and the two limit
  literals replaced by `DocumentLimits` (:194, :250).
- The header gate, decode and `validate` order is unchanged (ProjectStore.swift:146-152), and `save` still
  validates first (:85).
- No test that pins the version changed (GroupTests.swift, GuideTests.swift, LayerAppearanceTests.swift and
  ProjectTests.swift are not in the diff).
- `docs/project-format.md` now says "version `9` for new saves (versions `1`-`8` remain readable)" (NEW
  docs/project-format.md:5).

### 1.2 Codable shapes: unchanged

Checked by diff. None of these declarations changed:

| Type | File | Diff |
|---|---|---|
| `ProjectManifest`, `ProjectLayerRecord` | IO/ProjectStore.swift | none to either struct (only the lines listed in 1.1) |
| `LayerAdjustment`, `AdjustmentKind` | Document/LayerAdjustment.swift | file not in the diff |
| `BlackWhiteSettings`, `ColorBalanceSettings` and the older settings | Document/ImageAdjustments.swift, Levels.swift, Curves.swift, HueSaturation.swift | not in the diff |
| `LayerEffects` and its six effect structs | Document/LayerEffects.swift | one line, the render size guard at :441 (section 2.4); no Codable change |
| `LayerTextStyle` | Document/TypeTool.swift | fields unchanged; only the `boxIsValid` limits (:25-29), see 1.4 |
| `LayerShapeStyle`, `ShapeKind` | Document/ShapeTool.swift | fields unchanged; only `maxShapePixels` (:68) and its message (:131) |
| `LayerBlendMode` | Document/LayerAppearance.swift | not in the diff |
| `CanvasGuide` | Document/Guides.swift | not in the diff |
| `LayerTransform` | Document/LayerTransform.swift | not in the diff |

There is still no custom `init(from:)` / `encode(to:)` anywhere; the rules in the 1.2.6 research section 0 still
apply.

### 1.3 `DocumentLimits.swift` (new, 1.2.8)

Document/DocumentLimits.swift:16-42 collects the literals that were repeated everywhere and splits one budget in
two:

| Constant | Value | Replaces | Line |
|---|---|---|---|
| `maxSide` / `maxSideExtent` | 30,000 | 30,000 (same value) | :18, :21 |
| `maxSurfacePixels` / `maxSurfaceExtent` | 200,000,000 | 100,000,000 for one surface | :25, :28 |
| `documentPixelBudget` | `min(800_000_000, max(200_000_000, physicalMemory / 16))` | 100,000,000 for the whole document | :36-37 |

`documentPixelBudget` depends on the machine: `physicalMemory / 16` is a quarter of RAM at 4 bytes a pixel
(comment at :32-35). That gives about 537 MP on an 8 GB Mac and the 800 MP cap on 16 GB or more. The floor of
200 MP only matters below about 3.2 GB of RAM.

Where each one now applies (NEW):

- **Whole-document budget (machine-dependent):**
  - project load and save: `checkSize` (IO/ProjectStore.swift:249-254), called for layer images at :95 and
    :170 and, separately, for masks at :94 and :169. Images and masks each have their own running total, as
    before;
  - imports (EditorSession.swift:761-762, 772, 778, 783, 797; IO/ImageImporter.swift:34, 54, 87;
    IO/PSD/PSDReader.swift:24, 29);
  - Canvas Size fill (IO/CanvasResizer.swift:38);
  - Image Size per-layer and per-mask resamples (IO/ImageResizer.swift:45-46, 79-80);
  - brush allocation (Document/BrushStroke.swift:115; EditorSession+Brush.swift:20, 24);
  - copying layers between projects (Document/ProjectWorkspace.swift:194).
- **Single-surface cap (200 MP):**
  - export (IO/ImageExporter.swift:22-23);
  - the Image Size result canvas (IO/ImageResizer.swift:25);
  - effects render (Document/LayerEffects.swift:441);
  - clipping-stack and live-mask surfaces (Rendering/LiveMaskRenderer.swift:85, 132);
  - adjustment surface (Rendering/AdjustmentSurface.swift:9);
  - Distort (Document/Distort.swift:95-96);
  - filter canvas growth (Document/Filters.swift:396);
  - floating-selection merge (Document/FloatingSelection.swift:131);
  - new layer mask (Document/LayerMask.swift:241);
  - selection mask expansion (Document/SelectionEdits.swift:130);
  - shapes (Document/ShapeTool.swift:68);
  - the text box and text render (Document/TypeTool.swift:28, 247);
  - PSD canvas size (IO/PSD/PSDReader.swift:41-42);
  - downsample cache (Rendering/DownsampleCache.swift:16).
- **Unchanged:**
  - canvas side 1...30,000 and at most 10,000 layers (IO/ProjectStore.swift:194-195);
  - resolution 1...9600 (:191-192);
  - guides: at most 1,000, each within 1,000,000 (:234-247);
  - the 4 MB manifest and 512 MB per-asset byte limits (:111, :142, :160).

### 1.4 Validation changes (what files are valid)

Only two `validate`-level facts moved, and both are relaxations:

1. **Layer and mask raster totals:** a file is accepted when the sum of its image pixels, and separately the sum
   of its mask pixels, is at most `documentPixelBudget` (ProjectStore.swift:250). Before, the limit was 100 MP
   (OLD ProjectStore.swift, same function). **The limit now depends on the reading Mac.** A project saved on a
   16 GB Mac holding 700 MP of layers opens there and fails with `.tooLarge` on an 8 GB Mac (537 MP).
2. **Text `boxSize` area:** `boxIsValid` now allows `width * height <= 200,000,000` (Document/TypeTool.swift:28),
   up from 100,000,000. The side range stays 16...30,000. `validate` calls `text.isValid`
   (ProjectStore.swift:197-198), so a file whose paragraph box area is between 100 M and 200 M is now valid.

Nothing became stricter. The adjustment, effects, guide, folder-opacity and v9 gates are line-for-line the same
(ProjectStore.swift:196-231).

### 1.5 Compatibility

**Port (v9) to 1.2.10: opens.** The port writes version 9 (PORT engine/src/manifest.rs:8), which 1.2.10 accepts
(ProjectStore.swift:18). It holds itself to 100 MP (PORT manifest.rs:11; package.rs:11-18, used on open at :39, :49
and on save at :70, :76) and to a 100 MP text box area (PORT manifest.rs:292). Both are at most 1.2.10's floors (200
MP), so the only rules that moved are satisfied. The 1.2.6 research's findings about what 1.2.6 does with a port file
apply unchanged.

**1.2.10 to port: opens, except for large documents.** It is the same v9 format, so everything in the 1.2.6
research section 6 and Phase 3.5a applies. A 1.2.10 file fails in the port in these cases only:

- total layer-image pixels or total mask pixels above 100 MP: `ProjectError::TooLarge` at PORT package.rs:15;
- a live text `boxSize` with area above 100 M (up to 200 M): `Invalid` at PORT manifest.rs:292;
- a canvas above 100 MP opens, provided its layers fit, but cannot be exported: the port refuses at PORT
  compositor.rs:320, while 1.2.10 exports up to 200 MP.

Files likely to reach these limits come from the new large-document work, such as big PSD/PSB imports. Matching
1.2.10's upper bound (800 MP, 3.2 GB of RGBA) is not possible in a wasm32 engine with a 4 GiB address space, so the
port needs a ruling here, not a mechanical copy (section 5).

### 1.6 `docs/project-format.md` (1.2.7)

Documentation only. The doc now covers v7-v9 and the additive fields (NEW docs/project-format.md:29-40) and adds
`innerGlow` to the effects list (:48). Where it disagrees with the code, the code wins:

- It says adjustment layers "affect everything composited below them within their folder" (:29). The renderer
  applies them to the whole composite beneath, including layers outside the folder (1.2.6 research 3.4 step 1;
  LiveMaskRenderer.swift is unchanged apart from the size guards).
- It says every adjustment setting is "optional and defaulting to an identity adjustment" (:29). In fact `hue`,
  `saturation`, `lightness`, `colorize`, `levels` and `curves` are non-optional with defaults, so they are
  required on decode (Document/LayerAdjustment.swift:47-57). An absent `blackWhiteSettings` resolves to the
  Photoshop weights, which are not an identity (:84-87).
- It confirms that `text.fontName` is a **PostScript** font name (:44). PSD import stores Photoshop's `FontSet`
  `Name` (IO/PSD/PSDText.swift:142-146), which is also a PostScript name. This settles 1.2.6 research uncertainty
  5.

## 2. Rendering that Phase 3.5b will port: unchanged

### 2.1 Files not touched between 5d6be39 and 4306206

`git diff --stat 5d6be39 4306206` over the following paths prints nothing:

- all `*.metal`, `*.c` and `*.h` files, which includes Rendering/AdjustPixels.c, NoisePixels.c,
  LevelsPixels.c, BrushPixels.c, HealPixels.c, LensPixels.c, WandPixels.c and ContentFill.c;
- Rendering/SeparableBlend.swift, MetalLayerEffects.swift, LayerEffectsSurface.swift, EffectsPreviewCache.swift
  and LayerRenderer.swift;
- Document/LayerAppearance.swift, LayerAdjustment.swift, ImageAdjustments.swift, PixelInvert.swift,
  AdjustmentEditing.swift, LayerGroups.swift, LiveLayerMask.swift, Guides.swift, LayerFlip.swift, LayerMerge.swift
  and ImageTrim.swift.

The only Rendering/ diffs are:

- the size-guard literals in LiveMaskRenderer.swift (:85, :132), AdjustmentSurface.swift (:9) and
  DownsampleCache.swift (:16);
- canvas UI in EditorCanvas.swift (double-click text at :1472-1473 and :2105-2118, and the ants timer at :2009),
  InlineTextEditor.swift (pointer and caret) and TransformOverlay.swift (the marching-ants level of detail at
  :92-177, used at :286).

None of them computes pixels in export.

### 2.2 Per item

| Item | Maths changed? | Draw order changed? | Evidence |
|---|---|---|---|
| 11 new blend modes | No | No | LayerAppearance.swift and SeparableBlend.swift not in the diff; the `cgMode` call sites in LiveMaskRenderer.swift (:40, :74, :105) are untouched |
| Add Noise adjustment and its position hash | No | - | NoisePixels.c not in the diff; the Filters.swift diff is only :396 (the growth guard) |
| Gaussian Blur, Motion Blur | No | - | Filters.swift dispatch not in the diff (only :396 changed) |
| Invert | No | - | PixelInvert.swift and LayerAdjustment.swift not in the diff |
| Black & White, Color Balance | No | - | AdjustPixels.c and ImageAdjustments.swift not in the diff |
| Adjustment-layer compositing (global, blend, opacity, mask clip) | No | No | LiveMaskRenderer.swift `adjust` untouched; ImageExporter.swift diff is only :10 and :22-23 |
| Six layer effects (Metal and CPU paths) | No | No | MetalLayerEffects.swift not in the diff; LayerEffects.swift diff is the one guard at :441 |
| Grain kernel (1.2.6 roughness form) | No | - | AdjustPixels.c not in the diff |
| Folder opacity, clipping stacks | No | No | LayerGroups.swift not in the diff; LiveMaskRenderer.swift changed only at :85 and :132 |

So the 1.2.6 research sections 3.4, 4.1, 4.2 and 4.4 and 5.1 and 5.4 remain the oracle for 1.2.10, word for word.

### 2.3 The only pixel-visible rendering changes: raised ceilings

These change output only for documents or layers beyond the old 100 MP limits:

- **Effects on a very large layer:** `LayerEffectsRenderer.render` refuses a padded image over
  `maxSurfacePixels` (LayerEffects.swift:441). `cached` turns that throw into nil (`try?`, :405), and the layer
  then draws WITHOUT its effects. The threshold rose from 100 MP to 200 MP. In 1.2.6 a layer whose
  effects-padded image (layer pixels plus `2 * inset` each way, 1.2.6 research 4.2 step 3) fell between 100 MP
  and 200 MP silently lost its effects. In 1.2.10 it renders them. **Phase 3.5b: use 200 MP as the
  effects-surface limit, and draw the layer plainly (not an error) above it.**
- **Clipping stacks and live masks:** above the cap, `drawComposite` falls back to drawing the base and the
  children unstacked (LiveMaskRenderer.swift:84-89), and `coverage` returns nil, so the linked layer is not drawn
  (:131-132). The cap is now 200 MP. `bounds` is the export canvas, and export itself is capped at 200 MP
  (ImageExporter.swift:22-23), so in export these fallbacks can no longer fire. In 1.2.6 they could not fire
  either, because export was capped at 100 MP. The consequence is no change for any exportable canvas.
- **Export:** canvases up to 200 MP export (ImageExporter.swift:22-23). The 1.2.6 limit was 100 MP.

## 3. Edit operations the port mirrors

"File" means whether the change alters what gets written to `.comp`, and "Pixels" whether it alters pixel results
for inputs that were valid in 1.2.6.

| File | Diff (NEW lines) | File | Pixels |
|---|---|---|---|
| IO/CanvasResizer.swift | :8 `maxSide` (same 30,000); :38 the Canvas Size **fill layer** budget is now `documentPixelBudget - used`, was 100 MP | Only larger fills are now allowed | No |
| IO/ImageResizer.swift | :15 `maxSide` (same); :25 result canvas at most 200 MP (was 100); :45-46 and :79-80 per-layer and per-mask resample budget is now `documentPixelBudget` | Only larger results are now allowed | No |
| Document/Crop.swift | :12 `CropGeometry.valid` uses `maxSideExtent` (same 30,000) | No | No |
| Document/CanvasSize.swift | :27-28 `CanvasSizeDraft.valid` uses `maxSideExtent` (same) | No | No |
| Document/Distort.swift | :95-96 warped bounds area at most 200 MP (was 100) | Only larger results are now allowed | No |
| Document/TypeTool.swift | :25-29 `boxIsValid` area at most 200 M (see 1.4); :247 text render surface at most 200 MP; **:87-91 a click now puts the first baseline at the pointer**: `origin = (point.x - padding, point.y - (padding + lineHeight - |descender|))`, where `padding` = 12 (:22), instead of `origin = point`; :168 a dragged paragraph box is placed exactly at the drag rectangle's origin | The new text layer's `transform.origin` differs from 1.2.6 for the same click; the style fields are the same | Rendering of a given style is unchanged (`textImage` is not in the diff); placement only |
| Document/ShapeTool.swift | :68 `maxShapePixels` 200 MP (was 100); :131 message | Only larger shapes are now allowed | No |
| IO/ImageExporter.swift | :10 message; :22-23 export at most 200 MP (was 100) | - | Only larger exports are now allowed |
| LayerFlip.swift, LayerMerge.swift, Guides.swift, ImageTrim.swift | not in the diff | No | No |

The TypeTool placement oracle is TypeToolTests.swift (NEW) :18-23 and :31. `textColorPickerPreviewsAndRestoresDraft`
(:48-70) pins colour preview and restore during editing. This is UI state and nothing reaches the file until the
text is applied.

**The 1.2.6 Mac quirks are still present in 1.2.10:**

- `CanvasResizer` builds each record without `effects` (IO/CanvasResizer.swift:24-31).
- `applyDocumentSize` rebuilds every `ImageLayer` without `shape`, `effects` or `text` (IO/ImageResizer.swift:109-118).
  Canvas Size (ProjectController.swift:69), Image Size (ImageResizer.swift:106), Crop (Document/Crop.swift:177) and
  Trim (ProjectController.swift:115; Document/ImageTrim.swift:207) all go through it.

So Canvas Size, Image Size, Crop and Trim still appear to drop live shapes, effects and live text on the Mac. The
1.2.6 research section 5 caveat (read from code, not run) still applies.

Guides are still offset by Canvas Size (CanvasResizer.swift:18) and scaled by Image Size (ImageResizer.swift:28).

## 4. New features, one line each, with roadmap phase

Roadmap phases: 4 selections and retouching, 5 text and effects, 6 Liquify/Smudge, 7 Camera Raw, PSD and RAW.

| Feature | Where (NEW) | Phase |
|---|---|---|
| PSB (Photoshop Large Document) import: v2 header, 64-bit lengths, 4-byte RLE row counts | IO/PSD/PSDReader.swift:32-33, 69-74, 124-126, 188-195; PSDChannelCoder.swift:105-111 | 7 |
| Simple horizontal Photoshop text imported as live `text` (point or paragraph box, uniform scale, rotation, vertical flip; warp, faux styles and extra runs are reported); vertical, sheared or unevenly scaled text stays pixels | IO/PSD/PSDText.swift (682 lines); PSDDocumentBuilder.swift:27-41, 88-94; PSDReader.swift:359-361 | 7 (depends on 5's text model) |
| Background-only PSD/PSB imported as one layer from its merged image | Document/EditorSession.swift:781-787; IO/ImageImporter.swift:53-69 | 7 |
| Oversized PSD layers and masks cropped to the canvas when the file would exceed the budget, with a conversion note | IO/PSD/PSDReader.swift:82-87, 226-281; PSDChannelCoder.swift:4-56 and the cropped RLE decoder after :140; PSDDocumentBuilder.swift:28-31 | 7 |
| SVG import, rasterized once by AppKit and fitted inside the canvas (aspect kept), as an ordinary image layer | IO/ImageImporter.swift:32-51; EditorSession.swift:769-773; ProjectStore.swift:10 | none of 4-7 (image import, Phase 1 scope); needs a ruling |
| Document pixel budget scaled to RAM; 200 MP surface cap | Document/DocumentLimits.swift | cross-cutting (format and ops); needs a ruling (section 5) |
| Double-click live text with the Move tool to edit it | Rendering/EditorCanvas.swift:1472-1473, 2105-2118 | 5 |
| Clicked text starts on its first baseline at the pointer; dragged box exactly at the drag | Document/TypeTool.swift:87-91, 168 | 5 |
| Caret goes to the end when reopening text; pointer stays visible while typing | Rendering/InlineTextEditor.swift:31-32, 170-173 | 5 |
| Foreground colour picker previews on text being edited and restores on Cancel | Document/ColorPalette.swift:60-68, 90-106, 137-150; UI/TypeControls.swift:81 | 5 |
| Marching ants drawn from a screen-resolution outline when zoomed out on complex selections | Rendering/TransformOverlay.swift:92-177, 286 | 4 |
| Camera Raw grading wheels drawn with hue running counterclockwise | UI/CameraRawColorControls.swift:467-471 | 7 |
| Title-bar free space drags the window; tab-strip fade only when tabs overflow | UI/ProjectTabs.swift | none (Mac window chrome) |
| CI runs unit tests on push | .github/workflows/verify.yml | none |

New test suites that can serve as oracles: CropToCanvasImportTests.swift (5 tests), PSBImportTests.swift (3), 12
new PSD text tests in PSDRoundTripTests.swift (:581-715) and TitleBarDragTests.swift (UI).

## 5. What this means for the port (for rulings; nothing is decided here)

1. **Phase 3.5b scope is unaffected.** Port the rendering exactly as the 1.2.6 research describes it (section 2).
   The single addition is the 200 MP effects-surface threshold, with a plain draw above it (2.3).
2. **Pixel limits.** 1.2.10 accepts files up to 200-800 MP of layers, depending on the Mac. The port uses 100 MP
   everywhere (PORT manifest.rs:11; package.rs:15; compositor.rs:320; ops/image_size.rs:30, 56, 65;
   ops/canvas_size.rs:57; ops/distort.rs:10; ops/layers.rs:75; ops/masks.rs:13, 78; ops/adjust.rs:49). The
   options, which need a ruling:
   - (a) keep 100 MP and name the reason in the `TooLarge` message;
   - (b) raise the single-surface and export caps to 200 MP, and set a fixed document budget that the wasm heap
     can hold.
   The port cannot follow the Mac's RAM-dependent formula faithfully. The Mac itself does not give one answer: an
   8 GB Mac rejects what a 16 GB Mac wrote.
3. **Text box area.** Raising PORT manifest.rs:292 from 100 M to 200 M matches `boxIsValid` exactly and costs
   nothing, because the port does not render live text.
4. **Text click placement** (TypeTool.swift:87-91) belongs to Phase 5 when the port gains a Type tool. It does
   not affect any existing file.

## Points of uncertainty

1. Whether 292cb6b (text baseline placement) shipped in the 1.2.9 binary or first in 1.2.10. It was committed after
   the 1.2.9 version bump (a2160fa) and before that release's feed (a0eb31c). It makes no difference to file
   content.
2. The effects fallback in 2.3 (a layer drawn without effects above the cap) is read from `try?` at
   LayerEffects.swift:405. I have not observed it on a Mac.
3. The disagreements between the Mac's own `docs/project-format.md` and its code (1.6) are resolved in favour of
   the code, as the 1.2.6 research read it. They have not been confirmed by a Mac render.
4. Everything the 1.2.6 research lists as uncertain remains uncertain, except item 5 (fontName), which is settled
   in 1.6.
