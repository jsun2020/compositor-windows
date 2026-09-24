# Mac Compositor 1.2.6: project format and rendering delta against the v7 snapshot

Read-only research. Nothing in either Mac tree or in the port was modified.

- OLD = `C:\Users\sr9rfx\.claude-project\Compositor` (format v7, what the port was built against).
- NEW = `C:\Users\sr9rfx\.claude-project\Compositor-1.2.6` (git HEAD 5d6be39, "Publish update feed for Compositor 1.2.6").
- PORT = `C:\Users\sr9rfx\.claude-project\compositor-windows`.

All `file:line` citations without a prefix are in NEW under `Compositor/`. OLD and PORT citations say so.
The two Mac trees differ in line endings (OLD is CRLF), so every diff below was taken with
`diff --strip-trailing-cr`; files reported as "differ" by a plain diff but empty under that flag are unchanged.

## 0. How Swift encodes everything below (the rules this document relies on)

Checked, not assumed:

- NEW contains no custom `init(from:)`, `encode(to:)`, `CodingKeys`, `singleValueContainer` or
  `unkeyedContainer` anywhere (grep over all of `Compositor/` finds none; the only "encoder" hits are Metal
  compute encoders). Every type below uses compiler-synthesized `Codable`.
- Synthesized encoding writes an `Optional` property with `encodeIfPresent`: nil means the key is omitted,
  never `null`. Synthesized decoding reads it with `decodeIfPresent`: a missing key is fine.
- A non-optional `var` with an initial value (for example `var inside = false`) is still REQUIRED on decode:
  synthesis uses `decode`, not the default. A file missing such a key fails to decode, which makes
  `ProjectStore.load` throw `.invalid` (ProjectStore.swift:149-150).
- `UUID` encodes as its uppercase hyphenated string. `String`-backed enums encode as the raw value.
  `CGFloat` encodes as a JSON number (as `Double`). `CGSize` encodes as `[width, height]` and `CGPoint` as
  `[x, y]` (unkeyed), the same shape the fixture shows for `transform.size` and `transform.origin`.
- A `Dictionary` whose key is neither `String`/`Int` nor `CodingKeyRepresentable` encodes as a flat array of
  alternating key and value.
- The encoder is `JSONEncoder` with `[.prettyPrinted, .sortedKeys]` (ProjectStore.swift:103-104), unchanged.

## 1. Format versions 8 and 9

### 1.1 Version constants and what the Mac writes

- `ProjectManifest.current = 9` and `supported = 1...current` (ProjectStore.swift:14, 17).
- `var version = ProjectManifest.current` (ProjectStore.swift:20). No code path assigns `version` on save:
  `projectSnapshot()` builds the manifest without a version argument (EditorSession+Projects.swift:14-16), and
  so do `CanvasResizer.resize` (IO/CanvasResizer.swift:16-18) and `ImageResizer.resize`
  (IO/ImageResizer.swift:18-20). A repo-wide grep for `version =` finds only ProjectStore.swift:20.
- **So the Mac always writes `"version" : 9`, whatever the content.** It does not write the lowest version that
  fits. Tests pin this: CompositorTests/GroupTests.swift:80, GuideTests.swift:59 and
  LayerAppearanceTests.swift:127 all expect 9. That is why the fixture (which uses no v8 or v9 feature) says 9.
- READ: header check `ProjectManifest.supported.contains(header.version)` else `ProjectError.version(n)`
  (ProjectStore.swift:148), full decode (:149-150), then `validate` (:151). `save` also runs `validate` first
  (ProjectStore.swift:84), so the Mac refuses to
  write anything its reader would reject.
- History (git, NEW): v8 arrived with rulers/guides (e3a474a, merged in 66c3a37). v9 arrived with the blur
  adjustment layers (98ad817), Add Noise joined the v9 gate in 24052b5, and a361901 fixed the header gate that
  still stopped at 8. The a361901 message says 1.2.2 shipped before v9, so shipped 1.2.2 writes v8 and 1.2.3 and
  later write v9. Invert, Black & White and Color Balance shipped in 1.2.2 (0d31ccb / e3fbdd1), i.e. in v8 files,
  which is why they are not v9-gated (see 1.3).

### 1.2 Version 8

Two things are gated on `version >= 8`.

**(a) Guides** - new manifest field `guides: [CanvasGuide]? = nil` (ProjectStore.swift:29).

`CanvasGuide` (Document/Guides.swift:5-10), synthesized Codable:

```
struct CanvasGuide: Codable { enum Axis: String, Codable { case horizontal, vertical }
  var id: UUID; var axis: Axis; var position: Double }
```

JSON (keys sorted):

```
"guides" : [
  { "axis" : "vertical",   "id" : "8F0C...-UPPERCASE", "position" : 16 },
  { "axis" : "horizontal", "id" : "1A2B...-UPPERCASE", "position" : 12.5 }
]
```

- `axis` raw values are the case names `"horizontal"` / `"vertical"` (no explicit raw values).
- `position` is document pixels: Y for a horizontal guide, X for a vertical one (Guides.swift:9). It may be
  fractional and may lie outside the canvas.
- Validation, `validateGuides` (ProjectStore.swift:233-246): if `version < 8` the array must be absent or empty,
  else `.invalid`. For v8+: at most 1000 guides (`.tooLarge`), ids unique, `position` finite and
  `abs(position) <= 1_000_000`, else `.invalid`. No check that position is on the canvas.
- READ: `manifest.guides ?? []` into `CanvasDocument.guides` (EditorSession+Projects.swift:34).
- WRITE: `guides: document.guides.isEmpty ? nil : document.guides` (EditorSession+Projects.swift:16), so the key
  is omitted when there are none, else written in document order (append order; not sorted).
- Oracle: GuideTests.swift:51-76 (round trip, and a v7 manifest carrying guides is rejected with `.invalid`).

**(b) Folder opacity** - no new field; a rule change on the existing `opacity`.

- NEW rule (ProjectStore.swift:214-216): `opacity` finite in 0...1 for every layer; `version >= 3` or
  (opacity 1 and Normal); and for a group: `blend == .normal && (version >= 8 || opacity == 1)`.
- OLD rule (OLD ProjectStore.swift, same block): a group needed `opacity == 1 && blend == .normal` at every
  version.
- So a folder's `blendMode` must still be absent or `"Normal"` at every version (folders are pass-through), and a
  folder `opacity` other than 1 needs v8+.
- READ: `opacity ?? 1` (EditorSession+Projects.swift:30). WRITE: `ImageLayer.opacity` is non-optional
  (Document/EditorSession.swift:16), so `opacity` (and `blendMode`, `isGroup`) are written on every record,
  groups included; the fixture shows this.
- Rendering: section 4.1. Oracle: ProjectTests.swift `aDimmedFolderSavesAndReopens` (new, lines 74-95).

### 1.3 Version 9

One rule (ProjectStore.swift:199-204):

```
if let adjustment = layer.adjustment {
    guard manifest.version >= 7, layer.isGroup != true, layer.imageFile == nil, adjustment.isValid else { .invalid }
    if adjustment.kind == .gaussianBlur || .motionBlur || .addNoise { guard manifest.version >= 9 else { .invalid } }
}
```

- Gaussian Blur, Motion Blur and Add Noise adjustment layers need v9. Confirmed.
- **Invert, Black & White and Color Balance are NOT v9-gated**: they only need the existing v7 adjustment rule.
  A v7 or v8 file may legally contain them (1.2.2 wrote them into v8 files).
- JSON shape, validation and rendering of all six new kinds: section 3.

### 1.4 Everything else new is ungated

`effects`, `text`, the Line shape fields, the eleven new blend-mode strings, and the three ungated adjustment
kinds carry no version check at all. `text` and `effects` would be accepted even in a version-1 manifest
(section 2). The only version rules in `validate` are the ones quoted above plus the unchanged v1/v3/v4/v5/v6/v7
rules (ProjectStore.swift:200, 206, 215, 220-221).

## 2. Ungated layer-record additions

`ProjectLayerRecord` gains exactly two fields (ProjectStore.swift:52-54); `ProjectManifest` gains only `guides`
(section 1.2). OLD vs NEW diff of both structs shows nothing else.

```
var effects: LayerEffects? = nil      // ProjectStore.swift:53
var text: LayerTextStyle? = nil       // ProjectStore.swift:54
```

`shape` existed in v7 but `LayerShapeStyle` grew. Separately, `blendMode` gained eleven values (2.4) and
`adjustment.kind` six (section 3), which change what an existing field may contain.

### 2.1 `effects` (LayerEffects) - NOT baked into pixels

Type (Document/LayerEffects.swift:127-134), every member optional, so absent members are omitted:

```
struct LayerEffects: Codable {
  var stroke: StrokeEffect? ; var shadow: ShadowEffect? ; var colorOverlay: ColorOverlayEffect?
  var innerShadow: InnerShadowEffect? ; var outerGlow: OuterGlowEffect? ; var innerGlow: InnerGlowEffect? }
```

Nested types (all synthesized; `enabled` is the only optional member in each, the rest are REQUIRED on decode):

| Type (LayerEffects.swift) | Keys: type = default | isValid |
|---|---|---|
| `StrokeEffect` :6-22 | `enabled`: Bool? = nil (nil = visible); `size` = 4; `red`,`green`,`blue` = 0; `opacity`: Double = 1; `inside`: Bool = false | size 0...500, opacity 0...1, rgb 0...1, all finite |
| `ShadowEffect` (Drop Shadow) :25-50 | `enabled`?; `angle` = 90; `distance` = 20; `blur` = 20; rgb = 0; `opacity` = 0.5 | angle -360...360, distance 0...5000, blur 0...500, opacity and rgb 0...1 |
| `ColorOverlayEffect` :53-64 | `enabled`?; rgb = 0; `opacity` = 1 | opacity, rgb 0...1 |
| `InnerShadowEffect` :67-89 | `enabled`?; `angle` = 90; `distance` = 10; `blur` = 10; rgb = 0; `opacity` = 0.5 | as ShadowEffect |
| `OuterGlowEffect` :92-106 | `enabled`?; `size` = 20; rgb = 1; `opacity` = 0.75 | size 0...500, opacity, rgb 0...1 |
| `InnerGlowEffect` :109-123 | `enabled`?; `size` = 10; rgb = 1; `opacity` = 0.75 | as OuterGlow |

(The UI seeds a new Stroke and Color Overlay with the background colour: LayerEffects.swift:239-250.)

Example as the Mac writes it:

```
"effects" : {
  "shadow" : { "angle" : 90, "blue" : 0, "blur" : 20, "distance" : 20, "green" : 0, "opacity" : 0.5, "red" : 0 },
  "stroke" : { "blue" : 1, "enabled" : false, "green" : 1, "inside" : false, "opacity" : 1, "red" : 1, "size" : 4 }
}
```

- `enabled` appears only once the user has toggled the effect (`setEnabled`, LayerEffects.swift:192-201);
  `false` hides it but keeps it in the file.
- Colours are straight sRGB 0-1. `size`, `distance`, `blur` are in the layer's own pixel units (section 4.2),
  despite the "document-pixel" wording at LayerEffects.swift:7.
- The Mac never writes `"effects" : {}`: `setEffects` stores nil when empty (LayerEffects.swift:308).
- **Validation: none.** `ProjectStore.validate` never looks at `effects` (not at `isValid`, the version,
  `isGroup`, or `imageFile`). A file with out-of-range or misplaced effects loads. At render time
  `LayerEffectsRenderer.cached` drops them silently when `effects.visible.isValid` is false
  (LayerEffects.swift:403-408), and `setEffects` refuses edits (:302). Effects on a group, an adjustment layer or
  an empty layer load but never render (4.2).
- READ: `effects: $0.effects` straight through (EditorSession+Projects.swift:32). WRITE: `effects: layer.effects`
  (EditorSession+Projects.swift:12).
- **Pixels:** effects are drawn at render time around `imageFile`'s pixels; they are never in the PNG. An app
  that ignores `effects` renders the layer WRONG (no stroke, shadow, glow, overlay). The PORT today also drops
  the key on re-save (section 6).
- Oracles: ProjectTests.swift `projectRoundTripPreservesAllLayerEffects` and `layerEffectsAreRenderedInExport`
  (lines 209-311); InnerGlowTests.swift and OuterGlowTests.swift (backward-compatible JSON samples at
  OuterGlowTests.swift:67-89 and InnerGlowTests.swift:53-73).

### 2.2 `text` (LayerTextStyle) - pixels ARE in imageFile

Type (Document/TypeTool.swift:3-37):

```
enum TextAlignment: String, Codable { case left = "Left", center = "Center", right = "Right" }
struct LayerTextStyle: Codable {
  var content = "Text"; var fontName = "Helvetica"; var fontSize: CGFloat = 72
  var red: CGFloat = 0; var green: CGFloat = 0; var blue: CGFloat = 0
  var alignment: TextAlignment = .left; var tracking: CGFloat = 0
  var leading: CGFloat = 0          // 0 = Auto = 1.2 x fontSize, in layer pixels
  var boxSize: CGSize? = nil        // nil = point text; else a fixed paragraph box, [w, h]
}
```

```
"text" : { "alignment" : "Left", "blue" : 0, "boxSize" : [ 400, 200 ], "content" : "Hello\nworld",
           "fontName" : "Helvetica", "fontSize" : 72, "green" : 0, "leading" : 0, "red" : 0, "tracking" : 0 }
```

- All keys except `boxSize` are required on decode.
- `fontName` is passed to `NSFont(name:size:)` with a system-font fallback (TypeTool.swift:208, 215). I did not
  confirm whether the UI stores PostScript names or display names.
- Validation (ProjectStore.swift:196-198): `text.isValid && imageFile != nil && isGroup != true &&
  adjustment == nil`, else `.invalid`. `isValid` (TypeTool.swift:30-36): content <= 100,000 UTF-16 units;
  `boxSize` (if present) finite, each side 16...30,000, area <= 100 M; fontSize 1...2000; rgb 0...1;
  tracking -100...1000; leading 0...5000. No version gate.
- READ: `LayerText.loaded(style, image:)` keeps the style whenever it is valid and the layer has pixels
  (TypeTool.swift:45-48), with no check that the pixels still match. WRITE: `layer.liveText?.style`
  (EditorSession+Projects.swift:12). `liveText` is nil once anything replaces the layer's image
  (TypeTool.swift:52-55), so the Mac stops writing `text` as soon as the pixels are edited. Oracle:
  TypeToolTests.swift:68-90.
- **Pixels:** the rendered text is the layer's PNG. An app that ignores `text` renders correctly.
  **Obligation for the port:** if it keeps `text` on re-save it must drop it whenever it changes that layer's
  pixels (the port already does this for `shape`, PORT engine/src/document.rs:60). Otherwise the Mac reopens
  the layer as live text, and its next text edit re-renders from the style and discards the port's pixel
  changes.

### 2.3 `shape` (LayerShapeStyle) - extended; pixels ARE in imageFile

Diff OLD to NEW (Document/ShapeTool.swift:3-32):

- `ShapeKind` gains `case line = "Line"` (ShapeTool.swift:6).
- `LayerShapeStyle` gains three optionals (ShapeTool.swift:25-30): `lineWidth: CGFloat?` (stroke thickness in
  layer pixels), `start: CGPoint?` and `end: CGPoint?` (line ends as fractions 0-1 of the layer box, JSON
  `[x, y]`). They are nil (omitted) on rectangles and ellipses.
- The existing required keys `kind`, `red`, `green`, `blue`, `cornerRadius` are unchanged.

```
"shape" : { "blue" : 0, "cornerRadius" : 0, "end" : [ 1, 0.25 ], "green" : 0, "kind" : "Line",
            "lineWidth" : 6, "red" : 0, "start" : [ 0, 0.75 ] }
```

- Validation: none in ProjectStore, in OLD or NEW. An unknown `kind` string fails decoding and so the whole
  manifest.
- Redraw (only needed when the port rescales a live shape): a line is stroked with round caps, width
  `max(1, lineWidth)`, from `start` to `end` scaled by the box size, or corner to corner inset by half the
  thickness when the ends are absent (ShapeTool.swift:200-216).
- Pixels are in `imageFile`, so ignoring `shape` renders correctly. The PORT keeps `shape` as opaque JSON
  (PORT manifest.rs:51), so the Line fields already round-trip.

### 2.4 New blend-mode strings (existing `blendMode` field)

`LayerBlendMode` (Document/LayerAppearance.swift:4-13), raw values exactly:

```
OLD 13: "Normal" "Multiply" "Screen" "Overlay" "Darken" "Lighten" "Difference" "Color Dodge" "Color Burn"
        "Hue" "Saturation" "Color" "Luminosity"
NEW adds 11: "Linear Burn" "Linear Dodge (Add)" "Soft Light" "Hard Light" "Vivid Light" "Linear Light"
             "Pin Light" "Hard Mix" "Exclusion" "Subtract" "Divide"
```

- Note the parentheses in `"Linear Dodge (Add)"`. Darker Color and Lighter Color are deliberately absent
  (LayerAppearance.swift:69-70).
- There is no version gate, apart from the old rule that anything other than Normal needs v3+. PSD import
  produces these modes as well (IO/PSD/PSDTypes.swift:62, `fromPSD`).
- The PORT enum (PORT manifest.rs:16-32) lacks them, so any file using one fails with `Invalid`. Verified: see
  section 6.
- Rendering: section 4.4.

## 3. Adjustment changes

### 3.1 `AdjustmentKind` (Document/LayerAdjustment.swift:4-9)

| Case | Raw value | New? | Version gate |
|---|---|---|---|
| hsv | "Hue/Saturation" | old | 7 |
| levels | "Levels" | old | 7 |
| curves | "Curves" | old | 7 |
| exposure | "Exposure" | old | 7 |
| gradientMap | "Gradient Map" | old | 7 |
| grain | "Grain" | old | 7 |
| addNoise | "Add Noise" | NEW | 9 |
| gaussianBlur | "Gaussian Blur" | NEW | 9 |
| motionBlur | "Motion Blur" | NEW | 9 |
| invert | "Invert" | NEW | 7 |
| blackWhite | "Black & White" | NEW | 7 |
| colorBalance | "Color Balance" | NEW | 7 |

That is six new kinds. The PORT has the first six only (PORT engine/src/adjust/settings.rs:28-35).

### 3.2 New `LayerAdjustment` fields (LayerAdjustment.swift:62-71), all optional and omitted when nil

```
var blackWhiteSettings: BlackWhiteSettings?    // object
var colorBalanceSettings: ColorBalanceSettings? // object
var blurRadius: Double?          // Gaussian Blur; nil means 10
var motionAngle: Double?         // Motion Blur; nil means 0
var motionDistance: Double?      // Motion Blur; nil means 10
var noiseAmount: Double?         // Add Noise; nil means 10
var noiseGaussian: Bool?         // Add Noise; nil means false
var noiseMonochromatic: Bool?    // Add Noise; nil means false
var noiseSeed: UInt32?           // Add Noise; nil means 0; JSON integer 0...4294967295
```

Defaults are applied by the resolved accessors (LayerAdjustment.swift:84-119). The blur and noise values are
flat keys on the adjustment object, not a nested settings object.

What the Mac writes:
- `addAdjustment` (LayerAdjustment.swift:184-205) creates the layer with only `kind`, plus a random `noiseSeed`
  for Add Noise (:194). Grain gets a random `grainSettings.seed` as before.
- Committing the editor writes only the edited kind's own fields (Document/AdjustmentEditing.swift, NEW
  lines 84-93): `blurRadius` for Gaussian; `motionAngle` and `motionDistance` for Motion; `noiseAmount`,
  `noiseGaussian` and `noiseMonochromatic` for Noise; `blackWhiteSettings` or `colorBalanceSettings` for those.
- Invert has nothing to set and opens no editor (LayerAdjustment.swift:28, 204).
- A reader must therefore treat every one of these as optional with the defaults above.

Example (Gaussian Blur layer after editing):

```
"adjustment" : { "blurRadius" : 24, "colorize" : false, "curves" : { ...default... }, "hue" : 0,
                 "kind" : "Gaussian Blur", "levels" : { ...default... }, "lightness" : 0, "saturation" : 0 }
```

(`hue`, `saturation`, `lightness`, `colorize`, `levels` and `curves` are non-optional with defaults, so they are
still always written and are required on decode, exactly as in v7.)

**`BlackWhiteSettings`** (Document/ImageAdjustments.swift:114-141), all keys required inside the object:

```
{ "blues" : 20, "cyans" : 60, "greens" : 40, "magentas" : 80, "reds" : 40,
  "tint" : false, "tintHue" : 40, "tintSaturation" : 20, "yellows" : 60 }
```

- Valid: the six weights -200...300, tintHue 0...360, tintSaturation 0...100.
- **Absent `blackWhiteSettings` resolves to these Photoshop defaults, not to identity**
  (LayerAdjustment.swift:84-87). A freshly added B&W layer that was never edited still converts to grey.

**`ColorBalanceSettings`** (ImageAdjustments.swift:146-176), all keys required inside the object:

```
{ "highlightCyanRed" : 0, "highlightMagentaGreen" : 0, "highlightYellowBlue" : 0,
  "midCyanRed" : 0, "midMagentaGreen" : 0, "midYellowBlue" : 0, "preserveLuminosity" : true,
  "shadowCyanRed" : 0, "shadowMagentaGreen" : 0, "shadowYellowBlue" : 0 }
```

- Valid: each value -100...100. Absent resolves to all zero, which is identity (`apply` returns the image
  unchanged, :167).

**`isValid` additions** (LayerAdjustment.swift:136-140), applied to the RESOLVED values of every adjustment,
whatever its kind:

- `blackWhite.isValid && colorBalance.isValid`
- gaussianRadius 0.1...250; motionAngle -90...90; motionDistance 1...2000; noiseAmount 0.1...400; all finite.
- There is no check on `noiseSeed` beyond decoding as UInt32 (a negative number or a value above 2^32-1 fails
  decoding, making the manifest invalid).
- A failing `isValid` makes the whole file `.invalid` (ProjectStore.swift:200).

### 3.3 Existing fields and encodings: unchanged

- `LevelsSettings`, `CurvesSettings`, `HueSaturationSettings`, `ExposureSettings`, `GradientMapSettings`,
  `GrainSettings`, `AdjustmentColor`, `LayerTransform`, `LayerMask`-related record fields: no declaration
  change (Levels.swift diff touches only `LevelsFilter.run`; Curves.swift identical; HueSaturation.swift diff is
  only `canVignette` at :414-425; ImageAdjustments.swift keeps the old structs, with a comment edit at :188;
  LayerTransform.swift diff is only handle-drag logic at :180-205).
- **HSV maps confirmed still arrays:** `var adjustments: [ColorRange: RangeAdjustment]` and
  `var bands: [ColorRange: HueBand]` (Document/HueSaturation.swift:176-177), and
  `enum ColorRange: String, CaseIterable, Sendable, Hashable, Codable` (HueSaturation.swift:5), which is not
  `CodingKeyRepresentable`. There is no custom Codable anywhere in NEW. So both still encode as
  `["Master", {...}, "Reds", {...}, ...]`. The PORT's `adjustment_file` shim (PORT manifest.rs:62-110) remains
  correct.

### 3.4 How each new adjustment-layer kind renders

Common path, unchanged from v7 apart from the `scale` argument: `LiveMaskRenderer.adjust`
(Rendering/LiveMaskRenderer.swift:21-60).

1. `original = context.makeImage()`: the whole composite drawn so far on the current surface, which in export is
   the full canvas (IO/ImageExporter.swift:40). An adjustment layer is global: it changes everything beneath it,
   including layers outside its folder (folders are pass-through). Inside a clipping stack it acts on the stack's
   group surface only (:98-100).
2. `settings.apply(original, region: bounds, scale: adjustmentScale)`. Export leaves `adjustmentScale` at 1
   (ImageExporter.swift never sets it); the canvas sets it to the zoom (Rendering/EditorCanvas.swift:953). **So
   every radius and distance below is in document pixels.**
3. A non-Normal blend mode blends at full coverage, then restores the original alpha (:24-46), using
   `blendMode.cgMode`. See the note in 4.4: the eight Core-Image-only modes map to `.normal` there.
4. Opacity below 1: `CIBlendWithMask` between adjusted and original by the effective opacity, which includes
   folder opacity (:47-55; ImageExporter.swift:61).
5. Drawn back with `.copy` (BrushRaster.draw), clipped by the adjustment layer's own enabled mask
   (ImageExporter.swift:62-66) and by every enclosing folder mask (FolderMaskClip.draw, ImageExporter.swift:68-72).
   Outside that coverage the composite is untouched. Inside it, spatial filters still sample the whole
   composite, including pixels outside the mask.

Per kind, all dispatched from `LayerAdjustment.apply` (LayerAdjustment.swift:144-180):

- **Gaussian Blur** (:159-175 into Document/Filters.swift:199-201):
  `CIImage(cgImage:).applyingGaussianBlur(sigma: blurRadius * scale)`, where `blurRadius` is the standard
  deviation (Filters.swift:41) clamped to 0.1...250 by `normalized` (:92). The source is deliberately not
  clamped to its extent (Filters.swift:181-183), so outside the canvas counts as transparent. **The blurred
  composite loses alpha near the canvas edges** (about 3 sigma deep). Derived from the code plus Core Image's
  documented unclamped behaviour; not pixel-tested on a Mac. CI runs through `PixelAdjust.ciContext`, which has
  `workingColorSpace: NSNull()` (Document/PixelAdjust.swift:8), so the blur works on premultiplied
  sRGB-ENCODED values, not linear light.
- **Motion Blur** (Filters.swift:202-208): `CIMotionBlur` with
  `inputRadius = motionDistance * scale / sqrt(12)` (`motionRadiusPerPixel`, :173) and
  `inputAngle = motionAngle * pi / 180`, counterclockwise from horizontal as in Photoshop. Same unclamped
  source, same unmanaged colour. UNCERTAIN: CIMotionBlur's exact kernel is undocumented; the comment at
  Filters.swift:170-172 says it tapers like a Gaussian with spread about its radius. The port cannot be bit-exact
  here without a Mac oracle.
- **Add Noise** (Filters.swift:209-218, kernel in Rendering/NoisePixels.c:20-49): `noise_add_at` with
  `seed = noiseSeed` and origin `floor(region.origin)`, which is (0,0) in export. Per document pixel:
  `base = hash(seed ^ hash(px*0x9e3779b9 ^ hash(py*0x85ebca6b)))`. Pixels with alpha 0 are skipped. Channel key
  `c` is `base + c*0x9e3779b9`, or `base` for all three when monochromatic. Spread is
  `amount/100 * 127.5`. Uniform: `(u*2-1)*spread`. Gaussian: Box-Muller `* spread * 2/3`. Added to the
  unpremultiplied value, clamped 0-255, re-premultiplied with `lroundf`. Exactly portable.
- **Invert** (LayerAdjustment.swift:176-178; Document/PixelInvert.swift:28-36): premultiplied
  `c' = alpha - c` per colour channel, alpha kept (vImage fixed-point x256 / 256, which is exact).
- **Black & White** (ImageAdjustments.swift:132-140; Rendering/AdjustPixels.c:110-152): unpremultiply to 0-1;
  `gray = min + (mid-min)*w[secondary] + (max-mid)*w[primary]`, with weights / 100 in the order red, yellow,
  green, cyan, blue, magenta, and primary/secondary chosen as at :124-126; clamp. Optional tint: HSL-style
  colour at `tintHue` with chroma `(1-|2g-1|) * tintSaturation/100` and lightness g. Write
  `min(alpha, round(v*alpha))`.
- **Color Balance** (ImageAdjustments.swift:165-175; AdjustPixels.c:156-196): per channel, `tonal_weights`
  (a=0.25, b=0.333, scale 0.7) splits the value into shadow/mid/highlight weights; add
  `shadows[i]*s + mid[i]*m + hi[i]*h` (each /100); clamp. With `preserveLuminosity`, rescale by
  `before/after` luma (0.299, 0.587, 0.114) when `after > 0.0001`. Write `min(alpha, round(v*alpha))`.

The canvas pads its redraw by `samplingMargin` (gaussian `r*3+2`, motion `d/2+2`: LayerAdjustment.swift:121-127;
AdjustmentSurface.swift:4-8; EditorCanvas.swift:803-808). That is canvas-only and does not affect export.

## 4. Rendering semantics the port must match

The ground truth is `ImageExporter.render` (IO/ImageExporter.swift:20-76). The canvas path (Document/
LiveLayerMask.swift:159-200 and Rendering/EditorCanvas.swift:799-960) mirrors it.

### 4.1 Folder opacity

- `LayerOpacity.effective` (Document/LayerGroups.swift:49-64): a layer's drawn opacity is its own opacity times
  the opacity of every ancestor folder, up to 64 levels. There are `ImageLayer` and `ProjectLayerRecord`
  wrappers at :66-81.
- Used for: every pixel layer's draw (ImageExporter.swift:42; LiveLayerMask.swift:167; EditorCanvas.swift:817),
  every adjustment layer's strength (ImageExporter.swift:61), and mask-source layers when baking
  (LiveLayerMask.swift:89).
- **It is per-descendant multiplication, not group compositing.** The folder is never drawn as a unit
  (pass-through; the comment at LayerGroups.swift:49-52 says so). Consequence: two overlapping opaque children
  in a 50% folder each draw at 50%, so the lower child shows through the upper. Photoshop would composite the
  group first and then fade it. Match the Mac, not Photoshop.
- **With folder masks:** independent. FolderMaskClip.draw (Document/LayerMask.swift:193-211) clips each
  descendant by every enclosing folder's enabled mask; opacity multiplies on top. Both simply multiply.
- **With clipping stacks** (LiveMaskRenderer.swift:78-111): the base is drawn with its effective opacity, its
  alpha is extracted, the colours are made opaque, each clipped child is drawn with ITS effective opacity (which
  already includes the folder factor), then the base alpha is restored and the group is composited with the
  base's `cgMode`. For base colour B, child colour C, child opacity c, base alpha A, base opacity b and folder
  factor f, the Mac yields colour `mix(B, C, c*f)` and alpha `A*b*f`. Photoshop yields `mix(B, C, c)` at the same
  alpha. **Clipped children are dimmed by the folder twice.** Clipped adjustment layers in a stack behave the
  same way (:99, with `adjustmentOpacity` including f). This comes from reading the code; I have not confirmed
  it with a Mac render.
- **Non-stack live-mask links:** the coverage is the source drawn by `drawOwn`, including the source's effective
  opacity (and effects) (LiveMaskRenderer.swift:115-145).
- **Adjustment layers inside a dimmed folder:** strength = own opacity x folder factors; they are clipped by
  folder masks through FolderMaskClip.draw.
- **Blend:** a folder's blend mode must be Normal (validation); there is no folder blend.

### 4.2 Layer effects

Pipeline (export; the canvas uses the same renderer through EffectsPreviewCache):

1. `mask = snapshot.mask(for:).clipImage(placement:over:width:height:)`: the layer's enabled mask resampled into
   the layer's own pixel grid, or nil if absent or disabled (ImageExporter.swift:43; LayerMask.swift:107-127).
2. `LayerEffectsRenderer.cached(image, mask:, effects:)` (LayerEffects.swift:403-408): only `visible` (enabled)
   effects, only if `isValid`. Otherwise nil and the layer draws normally.
3. `render` (LayerEffects.swift:436-496). `shown` = pixels through the mask (:510-521), so **effects follow
   the MASKED shape**. The image is padded by
   `inset = ceil(max(outside stroke size, shadow.distance + 3*shadow.blur, 3*outerGlow.size)) + 2`
   (`margin`, :421-432). An inside stroke, colour overlay, inner shadow and inner glow add no margin.
4. **On any Mac with Metal (every supported Mac), the GPU path is what renders**
   (LayerEffects.swift:446-453 into Rendering/MetalLayerEffects.swift:58-207). The CPU path (:454-495) is only
   a fallback and differs in details (for example its shadow offset is nearest-sampled). Port the Metal path.
5. The result is placed by growing the layer transform proportionally (`placed`, :411-419) and drawn ONCE with
   the layer's effective opacity and blend mode and `mask: nil` (ImageExporter.swift:46-50). SeparableBlend
   modes draw it as a unit through the surface (:57).

Metal path specifics (MetalLayerEffects.swift), all in the padded LAYER-PIXEL grid:

- Coverage `shape` = alpha / 255 (:219-226).
- **Stroke** (:89-97, kernels :229-270): `reach = max(1, round(size))` whole pixels. Separable max (outside) or
  min (inside) over `[-reach, +reach]` on rows then columns, which is a SQUARE (Chebyshev) reach, not round
  (see also LayerEffects.swift:571-572). Samples past the padded image read 0. `ring = clamp(dilated - shape)`
  outside, or `clamp(shape - eroded)` inside. The ring follows soft alpha continuously. **Positions are only
  outside or inside; there is no centre.** Colour = straight sRGB rgb, coverage `ring * opacity`. Skipped when
  size <= 0 or opacity <= 0.
- **Drop shadow** (:98-113): offset `(dx, dy) = (-cos(a)*d, sin(a)*d)` with `a` = `angle` in degrees
  counterclockwise from the right (the light direction), y downward in layer pixels (LayerEffects.swift:38-43).
  So 90 drops the shadow straight down and 0 puts it to the left. The shift is bilinear subpixel (:272-290).
  Blur: separable Gaussian, `sigma = blur/2`, kernel radius `max(1, round(3*sigma))`, normalized weights,
  clamp-to-edge sampling (:292-322); skipped when sigma <= 0.01. There is no "spread" or "size" parameter
  beyond these. Coverage `shadow * opacity`. The shadow is not knocked out under the layer; it is simply
  covered by the pixels source-over, so it shows through translucent pixels.
- **Inner shadow** (:114-135): the same shift and blur applied to `shape`, then `shape * (1 - moved)`.
- **Outer glow** (:137-153): `blur(shape, sigma = size/2)`, composited as `glow * (1 - shape) * opacity`.
- **Inner glow** (:155-174): `shape * (1 - blur(shape, sigma = size/2))`.
- **Colour overlay:** `shape * opacity`, flat colour, no blend mode.
- **Compose order** (:335-392), premultiplied source-over each step: drop shadow, then outer glow, then outside
  stroke, then the layer's pixels, then colour overlay, then inner glow, then inner shadow, then inside stroke.
  8-bit output `clamp(v)*255 + 0.5`. The CPU order is identical (LayerEffects.swift:455-493).

Interactions:

- **Opacity and blend mode:** applied to the whole effects-plus-pixels image as one unit (there is no separate
  fill opacity). Folder opacity and folder masks apply the same way.
- **Layer mask:** baked into the source BEFORE the effects are computed, so a stroke outlines the visible
  (masked) shape and is not itself cut by the mask. A disabled mask means the effects follow the unmasked pixels.
- **Transforms:** the effects are computed in the layer's pixel grid and then drawn through the layer's
  transform, so sizes and distances SCALE with the layer and the shadow direction ROTATES and FLIPS with it
  (LayerRenderer.swift:19-24 applies rotation and flips to the grown image). A vertically flipped layer casts
  its 90-degree shadow upward. This comes from the code; I did not see a Mac test pinning it.
- **Clipping:** a base's drawOwn includes its effects, so layers clipped to it see the stroke and shadow as
  coverage. A clipped layer's own effects are drawn inside the stack and end up limited to the base's alpha.
- **Groups, adjustment layers, empty layers: no effects.** The UI forbids them (`canEditEffects`,
  LayerEffects.swift:222), and render paths only reach layers with pixels (`guard let image = ...`,
  ImageExporter.swift:41). The file validator does not forbid them (2.1); they are ignored.
- **Hidden layers:** not drawn, effects included.

### 4.3 Guides

Display and snapping only. They are never drawn in export (`ImageExporter` has no reference to guides).

- The canvas draws them as 1-device-pixel cyan lines, `CGColor(srgbRed: 0, green: 1, blue: 1, alpha: 0.9)`,
  across the whole view (Document/Guides.swift:68; Rendering/TransformOverlay.swift:145-167), when View > Show
  Guides is on.
- Show, snap and lock are per-user preferences in UserDefaults and are not saved in the project
  (Document/EditorSession.swift:270-280).
- Snap targets: Guides.swift:182-249. Moves and crops snap to them only when both Snap and Snap To Guides are on
  and guides are visible (Document/Crop.swift, NEW, now calls `alignmentSnapTargets`).
- Document operations that move guides: Canvas Size, Crop and Trim offset them (IO/CanvasResizer.swift:18; Crop
  and Trim both go through CanvasResizer, Crop.swift:173 and ImageTrim.swift:193); Image Size scales them
  (IO/ImageResizer.swift:28); Flip Canvas mirrors the perpendicular ones (Document/LayerFlip.swift:77). Oracle:
  GuideTests.swift:78-104.

### 4.4 New blend modes (needed as soon as a file uses them)

- Drawn by Core Graphics directly: Soft Light, Hard Light and Exclusion (`cgMode`, LayerAppearance.swift:28-49).
  I believe CG follows the PDF-spec formulas (PDF Soft Light differs slightly from Photoshop's) but have not
  verified it.
- Drawn through Core Image on a copy of the canvas (Rendering/SeparableBlend.swift:22-46, `coreImageFilter`,
  LayerAppearance.swift:53-67): Color Burn, Color Dodge, Linear Burn, Linear Dodge, Vivid Light, Linear Light,
  Pin Light, Hard Mix, Subtract, Divide. The CIContext uses an sRGB working space (SeparableBlend.swift:13-17),
  so the blend is computed on sRGB-encoded values.
- UNCERTAIN: how CI's blend filters treat translucent premultiplied sources is undocumented. The eight new
  CI-only modes need a Mac pixel oracle before the port can claim a match.
- Quirk: the three places that use `cgMode` rather than SeparableBlend turn the eight CI-only modes into Normal.
  They are an adjustment layer's blend mode (LiveMaskRenderer.swift:40), a clipping-stack base's group
  composite (LiveMaskRenderer.swift:74, 105), and a non-bitmap target (SeparableBlend.draw returns false; not
  the case in export).

## 5. Changes that alter how a v1-v7 file renders

Only output differences are listed. Everything else in the mirrored files is unchanged or a refactor. Unchanged:
LayerRenderer.swift, TiledLayerRenderer.swift, DownsampleCache.swift, RasterSnapshot.swift, Curves.swift, the
FolderMaskClip part of LayerMask.swift, and LevelsPixels.c, BrushPixels.c, HealPixels.c, LensPixels.c,
WandPixels.c and ContentFill.c. HueSaturation.swift has no rendering change. The ImageAdjustments.swift Gradient
Map table change (:91-104) is the same arithmetic split into steps for the type checker.

1. **Grain roughness changed (affects every v7 Grain adjustment layer with roughness > 0; the default is 50).**
   - OLD (OLD Rendering/AdjustPixels.c:46-87): the fine component was
     `lattice(floor(u), floor(v), fineSeed)`, un-interpolated noise in document pixels.
   - NEW (AdjustPixels.c:47-96): the fine component is `grain_field(u, v, detailSize, fineSeed)`, a
     smoothstep-interpolated lattice at `detailSize = max(0.5, size * 0.35)`, the same interpolation as the main
     field. `noise = smooth + (fine - smooth) * rough` is unchanged, as are strength and the midtone curve.
   - The PORT implements OLD in both the CPU (PORT engine/src/adjust/grain.rs:17-34, line 33) and GL (PORT
     app/src/canvas/gl/programs.ts:229-242, line 241) paths. **Must change.**
   - Oracle: ImageAdjustmentTests.swift `grainSizeControlsParticleScaleEvenWithRoughness` (new, :89-102). The
     destructive Grain filter shares the kernel.
2. **Levels on soft edges: double unpremultiply removed.** OLD Levels.swift:75-87 unpremultiplied before
   `levels_apply`, which unpremultiplies again, and re-premultiplied after, so 50% grey at half alpha came out at
   a quarter. NEW (Levels.swift:75-78) calls `levels_apply` alone. This changes Levels adjustment layers and the
   Levels filter on any pixel with 0 < alpha < 255. **The PORT already matches NEW** (PORT
   engine/src/adjust/levels.rs:12-27 unpremultiplies once). Curves never had the extra step (Curves.swift:33-41).
3. **Color Dodge and Color Burn are now computed in sRGB.** OLD used a CIContext with the default (linear)
   working space. NEW sets an sRGB working space (SeparableBlend.swift:13-17); the comment says 80% grey over
   40% dodged to 62% instead of 100% and burned to 0% instead of 25%. NEW's
   LayerAppearanceTests.swift:73-94 expects 1.0 and 0.25, and the PORT's formulas on encoded values (PORT
   engine/src/blend.rs:16-17, with Phase 2 tests expecting 1.0 and 0.25) **already match NEW**. The alpha
   handling of CI's filters for translucent layers is still unverified (4.4).
4. **Add Noise pattern changed (destructive filter only; no effect on file rendering).** OLD hashed the pixel
   index, `noise_hash(seed ^ noise_hash(y*width + x))`. NEW hashes the position
   (NoisePixels.c:30-32; `noise_add` now calls `noise_add_at` with origin (0,0), :15-18). The PORT's Add Noise
   filter uses the OLD hash (PORT engine/src/adjust/filters.rs:156), so it will not reproduce the Mac's
   pattern, and it must use the NEW hash for Add Noise adjustment layers.
5. **Validation relaxed; no rendering change for v1-v7:** new blend-mode strings and the Invert, B&W and Color
   Balance kinds are accepted at v3+ and v7+ respectively, and `text` and `effects` at any version. A v7 file
   written by some other writer could therefore contain them.
6. **Canvas-only changes (not export):** adjustment `scale` and padding (LiveMaskRenderer.swift:20, 23;
   AdjustmentSurface.swift:4-8).

Mac behaviours observed while reading, which the port should NOT copy without a ruling:
- `applyDocumentSize` (IO/ImageResizer.swift:109-118) rebuilds every `ImageLayer` without
  `shape`/`effects`/`text`, and `CanvasResizer` omits `effects` from its records (CanvasResizer.swift:25-32).
  So Canvas Size, Image Size, Crop and Trim appear to drop live shapes, effects and live text on the Mac. This is
  from reading the code; it looks like a bug.
- LayerMerge.swift is unchanged from OLD, so merging does not rasterize effects (unverified what a merged
  layer's effects become).

## 6. The real file: `engine/tests/fixtures/mac-1.2.6/Mac-test-for-windows.comp`

Contents of `manifest.json` (1278 bytes):
- Manifest keys: `activeLayerID`, `colorSpace` "sRGB", `documentID`, `format`, `height` 1080, `layers`,
  `resolution` 72, `version` 9, `width` 962. There is no `guides` key.
- Layer "Layer 1": no `imageFile`, not a group, no adjustment, so an empty layer (existed in v7).
  Normal, opacity 1, transform origin [-479, 0], size [1920, 1080], "High quality".
- Layer "7e64ddbe-...": `imageFile` "C739274B-....png" (962 x 1708, 2.6 MB PNG), origin [0, -314], Normal,
  opacity 1.

What it exercises: **only the version-9 header.** No guides, folder opacity, effects, text, shape, new blend
mode or new adjustment. The screenshot shows the Smear tool (Liquify/Blur/Smudge) active and smeared marks and
Chinese lettering on the image; those are baked into the PNG (the lettering is not a `text` layer). The v9 number
is there only because 1.2.6 always writes 9 (1.1).

What stops the PORT today, verified by building a throwaway crate outside the repo against `compositor-engine`
and calling `package::open_package` / `Manifest::parse` on the fixture:

| Input | Result |
|---|---|
| fixture as is | `Err(Version(9))`, from `if !(1..=CURRENT_VERSION).contains(&header.version)` with `CURRENT_VERSION = 7` (PORT manifest.rs:7, 150; again at :168) |
| same manifest with `version` edited to 7 | `Ok`, 2 layers (PNG decodes, empty layer accepted) |
| v7 plus `"blendMode" : "Soft Light"` | `Err(Invalid)` (unknown enum string, PORT manifest.rs:16-32) |
| v7 plus an `effects` object on a layer | `Ok`: **silently ignored, and dropped on re-save** (serde default: no `deny_unknown_fields` on `LayerRecord`, PORT manifest.rs:34-52) |
| v7 plus a `guides` array | `Ok`: **silently ignored, and dropped on re-save** (`Manifest`, PORT manifest.rs:123-134) |

So nothing besides `version: 9` blocks this particular file. Other 1.2.6 files would additionally hit:
- new blend-mode strings: `Invalid`;
- new `AdjustmentKind` strings: `Invalid` (PORT settings.rs:28-35);
- `text` and `effects`: loaded but lost on save; effects also render wrongly;
- `guides`: lost on save;
- folder `opacity < 1`: `Invalid` even after the version gate is lifted, because PORT manifest.rs:192 still
  requires opacity 1 for groups at every version;
- the new adjustment `isValid` ranges are not checked.

## 7. New feature areas (pointers only)

Line counts are NEW file lengths. "Format" is the effect on `.comp`.

| Area | Main files | Size | Format impact |
|---|---|---|---|
| Text tool | Document/TypeTool.swift; Rendering/InlineTextEditor.swift; UI/TypeControls.swift | 256 + 442 + 155 | `text` (2.2) plus pixels. Faithful re-rendering needs AppKit text layout (NSLayoutManager, TypeTool.swift:233-255); not reproducible exactly on Windows. |
| Liquify / Smudge / Blur ("Smear") | Document/SmudgeLiquify.swift (`WarpStroke`); Document/BlurTool.swift; EditorSession+Brush.swift; EditorCanvas.swift warp preview | 210 + 41 | None (pixels). **Both files are identical to OLD**, so this tool is not new since v7. |
| Object selection | Document/ObjectSelection.swift (Vision instance masks) | 298 | None (selection only). |
| Camera Raw develop (a destructive filter) | Document/CameraRaw.swift, CameraRawColor.swift, CameraRawDetailOptics.swift, CameraRawGeometryCalibration.swift; UI/CameraRaw*.swift (5 files); about 950 new lines of C in Rendering/AdjustPixels.c (:198-1151) | about 1300 + 1390 + 950 | None (a `FilterKind.cameraRaw` bake, Filters.swift:15). |
| PSD import (read only) | IO/PSD/PSDReader.swift, PSDDocumentBuilder.swift, PSDTypes.swift, PSDVector.swift, PSDChannelCoder.swift; UI/PSDConversionSheet.swift | 1053 + 69 | Produces folder opacity, the new blend modes, live shapes, Levels/Curves/HSV adjustments and masks; discards effects (PSDDocumentBuilder.swift:35-36). |
| RAW import | IO/RawImporter.swift; UI/RawDevelopSheet.swift | 124 + 90 | None (develops to 8-bit pixels at import). |
| Floating selection | Document/FloatingSelection.swift | 159 | None. Exists in OLD; only feather pass-through changed (:87, 93). |
| Layer effects | Document/LayerEffects.swift; Rendering/MetalLayerEffects.swift, LayerEffectsSurface.swift, EffectsPreviewCache.swift; UI/EffectsSheet.swift | 638 + 394 + 158 + 141 + 192 | `effects` (2.1, 4.2) |
| Rulers, guides, grid, snap | Document/Guides.swift; UI/CanvasRulers.swift | 250 + 185 | `guides` (1.2) |
| New adjustment layers | LayerAdjustment.swift, ImageAdjustments.swift, AdjustmentEditing.swift, AdjustPixels.c :106-196, NoisePixels.c | - | section 3 |
| New blend modes | LayerAppearance.swift; SeparableBlend.swift | - | 2.4, 4.4 |
| Finishing filters: Vignette, Bloom/Glow, Tonal Contrast (1.2.4) | Filters.swift :219-252; AdjustPixels.c `adjust_colored_vignette`, `adjust_tonal_contrast` | - | None (destructive). Vignette can fill an empty layer. |
| Image Trim | Document/ImageTrim.swift; UI/TrimSheet.swift | 210 + 68 | None (through CanvasResizer). |
| Also new | UI/KeyboardShortcuts.swift (319, UserDefaults); Document/ToolDefaults.swift (22, UserDefaults); selection feather; brush `smoothing` (BrushStroke.swift:17-19); Line shape; Crop ratios 3:4 and 9:16; handle drag past the opposite side flips the layer (LayerTransform.swift:180-205); whole-layer copy/paste between projects; layer context menu; stepped keyboard zoom | - | None |

Test suites. New files in `CompositorTests` (line counts), usable as behavioural oracles:
CameraRawSliderTests.swift 63, CameraRawTests.swift 651, FinishingFilterTests.swift 122, GuideTests.swift 176,
ImageTrimTests.swift 180, InnerGlowTests.swift 249, OuterGlowTests.swift 345, PSDFixture.swift 355,
PSDRoundTripTests.swift 580, PSDVectorFixtures.swift 22, SelectionFeatherTests.swift 47, TypeToolTests.swift 173.
No test file was removed.

New tests inside existing files that matter for the format and rendering:
- ProjectTests.swift: `theCurrentFormatVersionIsOneTheReaderAccepts`, `aDimmedFolderSavesAndReopens`,
  `projectRoundTripPreservesAllLayerEffects`, `layerEffectsAreRenderedInExport` (a 6 px outside stroke is green
  at 5 px from the edge).
- AdjustmentLayerTests.swift: `invertAppliesWithoutAnEditor`, plus round trips for all new kinds (:157-246).
- ImageAdjustmentTests.swift: `grainSizeControlsParticleScaleEvenWithRoughness`.
- FilterTests.swift: `gaussianBlurSoftensAHardEdgeAndSpreadsPastTheLayerEdgeAsOneUndoStep`.
- GroupTests.swift:80, LayerAppearanceTests.swift:127: the version is 9.

## Points of uncertainty

1. The CIMotionBlur and CIGaussianBlur discrete kernels (3.4): the port can be close, not bit-exact, without Mac
   pixel oracles.
2. The alpha semantics of the eight Core-Image-only blend modes, and CG's Soft Light, Hard Light and Exclusion
   formulas (4.4).
3. Clipped children double-dimmed by folder opacity (4.1), and effects rotating and flipping with the layer
   (4.2): from reading the code, not observed on a Mac.
4. Canvas-edge alpha loss under a Gaussian or Motion Blur adjustment layer (3.4): from the code plus documented
   Core Image behaviour.
5. Whether `fontName` holds PostScript names (2.2).
6. Mac quirks in section 5 (Canvas Size, Image Size, Crop and Trim dropping effects, shapes and text; merge and
   effects): read from code, not run.
