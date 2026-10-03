# Compositor for Mac 1.4.5: what the Windows port must change to catch up

Date: 2026-09-30. Research only; no code was changed.

Sources:

- Mac: the worktree `Compositor-1.2.10` (HEAD 4306206 = 1.2.10), upstream tag `v1.4.5` (086f163). Every Mac
  citation is `file:line` in `git show v1.4.5:<file>` unless it names another revision. 135 non-merge commits
  lie between 4306206 and v1.4.5 (`git log --no-merges 4306206..v1.4.5`), 144 files changed.
- Port: `compositor-windows` on `main` (0.5.0). Port citations are `path:line` in that working tree.
- Earlier research: `mac-1.2.10-delta.md`, `mac-1.2.10-probe-results.md`, `phase4b1-rulings-and-open-items.md`
  (F1 and "The 1.3.7 catch-up").

The user now runs 1.4.5, not 1.3.7 as the 4b-1 rulings record. Everything 1.3.7 changed is included in 1.4.5;
this file covers 1.2.10 -> 1.4.5 in one pass.

## 0. Summary

- **Format.** 1.4.5 writes version 11 (ProjectStore.swift:15). Version 10 adds one optional key, `text.colorRuns`;
  version 11 adds `text.fontRuns` (ProjectStore.swift:208-214; TypeTool.swift:30-34). Nothing else in the
  manifest changed: no new layer, adjustment or blend kinds, no changed encodings, no new top-level keys. Saved
  packages also carry `QuickLook/Preview.jpg`, which loading ignores (ProjectStore.swift:116-121). Dither, Camera
  Raw curves, Color Range and grid settings are not saved in a project.
- **The port cannot open any project the user's Mac saves today:** every 1.4.5 save says `"version": 11`, and
  the port refuses anything above 9 (port `engine/src/manifest.rs:8`, `:210`).
- **Wrong results the port now draws** (1.2.10 oracle, changed in 1.4): Soft Light (Pegtop -> Core Image, the
  W3C/PDF formula); adjustment layers and clipping-stack bases in the Core-Image-only modes (were Normal, now
  their real mode), plus Color Burn, Color Dodge and Soft Light there (now Core Image); positive Hue/Saturation
  saturation (x(1+a) -> /(1-a)); Add Mask with a selection (hid it, now reveals it); an upright 1:1 layer at a
  fractional origin (bilinear -> nearest, Phase 3.5d).
- **Recommended phase order:** F1, then open/save format 11, then the wrong results (re-pinned against a 1.4.5
  re-export of the existing probe set), then the small behaviour changes; new features go to the phases listed
  in section 3.

## 1. The format

### 1.1 What versions 10 and 11 add

| Version | First Mac | What it adds | Where it is declared | Where it is gated |
|---|---|---|---|---|
| 10 | 1.3.2 (commit 2bfc4ea "Type: color only the selected letters") | optional `colorRuns` in a layer's `text` | TypeTool.swift:30-32, run type :190-196 | ProjectStore.swift:211 |
| 11 | 1.3.4 (commit 0eaeb98 "Type: apply a font change to the selected letters") | optional `fontRuns` in a layer's `text` | TypeTool.swift:33-34, run type :198-202 | ProjectStore.swift:212 |

(`static let current` is 9 at v1.3.1, 10 at v1.3.2-v1.3.3, 11 from v1.3.4; read with `git show vX:Compositor/IO/ProjectStore.swift`.)

The two run types, as Swift's synthesized `Codable` writes them (TypeTool.swift:190-202):

```
"colorRuns": [ { "location": 6, "length": 5, "red": 1, "green": 0, "blue": 0 } ]
"fontRuns":  [ { "location": 0, "length": 5, "fontName": "Helvetica-Bold" } ]
```

- `location` and `length` are Swift `Int`s in UTF-16 units of `content` (TypeTool.swift:30-31); `red`, `green`,
  `blue` are `CGFloat` 0-1; `fontName` is a PostScript name. Both lists are optional (`= nil`, TypeTool.swift:32,
  :34), so a missing key or `null` decodes as none. Every key inside a run is required by the synthesized decoder.
- Validity (`LayerTextStyle.isValid`, TypeTool.swift:35-62), checked on load (ProjectStore.swift:210): runs are
  sorted, do not overlap (`run.location >= end`), have `length > 0`, do not overflow (`location <= Int.max -
  length`), end within `content.utf16.count`, and a present list is not empty (`!colorRuns.isEmpty`, :51; :61).
  Colour channels are finite and in 0...1 (:48). A font run's `fontName` is non-empty, at most 200 Swift
  `Character`s (grapheme clusters, not bytes) and contains no newline character (:58).
- Drawn by `attributedText` (TypeTool.swift:432-447): font runs first, then colour runs, each only when it fits
  the string (`containsTextRun`, :445). The layer's PNG is still the rendered text (docs/project-format.md at
  v1.4.5, "Editable text"), so a reader that ignores the runs still shows them correctly.

Other package changes, all ungated and ignored on load:

- `QuickLook/Preview.jpg`: the flattened image on white, a JPEG at most 1024 px on its long side, written on every
  save for canvases up to 50 MP (ImageExporter.swift:80-84; ProjectController.swift:230-231;
  ProjectStore.swift:116-121). Loading reads only `manifest.json` and `images/` (ProjectStore.swift:146-196).
  `docs/writing-comp-files.md:86` tells other writers to delete the folder when they change a project, and the
  Mac writes it again on its next save.
- Images are now decoded from bytes in memory (ProjectStore.swift:169-173, commit cba6efc): no format change.

What did NOT change (checked by searching the whole diff for `Codable`, `CodingKey` and decoder code; the only hits
are the two run types):

- `ProjectManifest` and `ProjectLayerRecord` keep their fields (ProjectStore.swift:13-56). Adjustment kinds are the
  twelve of 1.2.10 (LayerAdjustment.swift:5-9); blend modes the 24 (LayerAppearance.swift:28-48). Validation of
  every other field is as in 1.2.10 (ProjectStore.swift:198-262).
- Dither is a destructive filter (`FilterKind.dither`, Filters.swift:13, :198), never an adjustment layer.
- Camera Raw curves are part of the destructive Camera Raw filter (CameraRawColor.swift:48-76).
- Grid Settings are app preferences in `UserDefaults` (EditorSession.swift:281-298); commit 1c819d0 says "so the
  project format doesn't change". Color Range makes an ordinary selection, and selections are never saved.

### 1.2 What each Mac does with each version

- 1.4.5 accepts versions 1-11 (`supported = 1...current`, ProjectStore.swift:18; header check :156, full check
  :200). A **version 9 file opens unchanged**; on its next save it is written as 11, because every save builds a
  fresh manifest whose `version` defaults to `current` (ProjectStore.swift:21; EditorSession+Projects.swift:14-16).
- 1.4.5 refuses a file whose `version` is below what its text runs need: `colorRuns` with version < 10 or
  `fontRuns` with version < 11 is "not a valid Compositor project" (ProjectStore.swift:210-213). **A port that
  keeps a 1.4.5 text layer verbatim but writes version 9 produces a file 1.4.5 will not open.**
- 1.4.5 ignores keys it does not know (synthesized `Decodable`, ProjectStore.swift:13-56) and drops them on its
  next save, as 1.2.10 did.
- 1.2.10 accepts 1-9 only; 1.3.2-1.3.3 accept 1-10. Both refuse a version 11 file with the version message
  (ProjectStore.swift:69 wording).
- A 1.4.5 Mac with the project open reloads it when another program rewrites the package (ProjectWatcher.swift:3-9,
  commit ed5ddea), asking first if it has unsaved work (`docs/writing-comp-files.md:88-94`). It fingerprints the
  manifest bytes and each image's name and size (ProjectDigest.swift, whole file), so a port save that rewrites
  identical PNGs with a different encoder size counts as a change. Harmless, but worth knowing when a project sits
  in a synced folder both machines open.

### 1.3 The port today

- `CURRENT_VERSION = 9` (port `engine/src/manifest.rs:8`); the header check refuses 10 and 11 with
  `ProjectError::Version` (`manifest.rs:210`, again at `:228`). So any 1.4.5 save, with or without text runs,
  fails to open.
- `text` is kept verbatim as a JSON value (`manifest.rs:103-104`), so runs would survive a re-save once the
  version is accepted. `text_is_valid` (`manifest.rs:289-317`) checks the 1.2.10 keys only and never looks at
  `colorRuns` / `fontRuns`, so today it would accept invalid runs the Mac refuses.
- A pixel edit drops `text` (`engine/src/document.rs:62-67`), as the Mac rasterizes text on destructive edits; runs
  go with it, which is right.
- Save writes a fresh staged package with `manifest.json` and `images/` only and swaps it in
  (`src-tauri/src/commands/package.rs:73-112`), so a `QuickLook` folder is dropped. That is what
  `docs/writing-comp-files.md:86` asks other writers to do. Open reads only `manifest.json` and `images/`
  (`package.rs:41-58`), so the folder is ignored on open, as on the Mac.
- `Document::undrawn` (`engine/src/document.rs:163-168`) will report nothing for a 1.4.5 file: runs live inside
  `text`, whose PNG is drawn.

### 1.4 To OPEN a 1.4.5 project faithfully

1. `CURRENT_VERSION = 11` (the header check and `validate` both read it, `manifest.rs:210`, `:228`).
2. Extend `text_is_valid` with the run rules of TypeTool.swift:43-62, read from the verbatim value: each list
   absent, `null`, or a non-empty array of objects; required keys present with the right JSON type (`location`,
   `length` integers; colours numbers; `fontName` string); sorted, non-overlapping, positive length, within
   `content`'s UTF-16 length; colours finite in 0...1; `fontName` 1-200 grapheme clusters (use
   `unicode-segmentation` or count `char`s and accept the small difference, recorded as a deviation) with no
   newline (`\n`, `\r`, U+000B, U+000C, U+0085, U+2028, U+2029).
3. Gate them as the Mac does: `colorRuns` needs version >= 10, `fontRuns` >= 11 (ProjectStore.swift:211-212).
4. Nothing to draw: the PNG already shows the runs. No notice is needed.
5. Ignore `QuickLook/` (already so).

This is the Phase 3.5a pattern: read everything, draw what is drawn, keep the rest verbatim, refuse only what the
Mac refuses.

### 1.5 To SAVE a project 1.4.5 reads back unchanged

1. Write `"version": 11` (section 1.6). The rest of the manifest is written as now: sorted keys, pretty printed,
   Swift-compatible encodings (`manifest.rs:216-223`).
2. Keep `text` verbatim, runs included, until Phase 5 edits text; then write runs as the Mac normalizes them
   (`setUnitColors` / `setUnitFonts`, TypeTool.swift:143-186: a run only where a letter differs from the base,
   adjacent equal letters merged, `nil` rather than an empty list, and all-one-face text folded into `fontName`).
   Write `location` and `length` as JSON integers.
3. Do not write a `QuickLook` folder: the Mac never reads it, writes its own on the next save, and a stale preview
   is worse than none (`docs/writing-comp-files.md:86`). Optional later: write one so Finder previews Windows
   saves (flattened on white, JPEG, 1024 px, quality 0.8, ImageExporter.swift:80-100).
4. "Unchanged" means: the Mac opens it without an error, draws it the same, and its next save produces the same
   manifest apart from key order inside Hue/Saturation maps (already documented in the spec, section 5) and any
   unknown keys the port kept (the Mac drops those).

### 1.6 Write 11 or keep writing 9?

**Write 11.** Reasons:

- A file with text runs must say at least 10 or 11, or 1.4.5 refuses it (ProjectStore.swift:210-213). Writing 9
  would mean the port could not re-save a 1.4.5 text layer it opened, or would have to strip the runs (data loss,
  against spec section 2).
- 1.4.5 itself always writes 11 (ProjectStore.swift:21), so a Mac -> Windows -> Mac round trip keeps the version
  stable.
- The only readers that cannot open 11 are Macs older than 1.3.4. The user runs 1.4.5.

The alternative, "write the lowest version the content needs" (9 with no runs, 10 with colour runs only, 11 with
font runs), keeps files openable by 1.2.10-1.3.3 Macs. It costs a small rule in `Document::manifest`
(`engine/src/document.rs:143-150`) and a test per branch; worth it only if another machine still runs a 1.2.x or
early 1.3.x build. Ask the user; default to 11.

Also say, in the open error, which Mac version is needed, for a future format 12.

### 1.7 Fixtures a real 1.4.5 save must prove

Return the saved `.comp` folders themselves (zipped), not only PNG exports: the 4b-1 round lost the saved folders
(phase4b1-rulings OQ17).

1. `text-runs.comp`: New Canvas 480 x 200; Type "Hello World"; select "World" and colour it red (a colour run);
   select "Hello" and pick another installed face, for example Helvetica Bold or Times New Roman (a font run); add
   a second, plain text layer "Plain". Save. Also File > Export > PNG as `text-runs.png`. Proves the v11 keys, the
   run encoding, and that the PNG shows the runs.
2. `text-colour-only.comp`: one text layer with only a colour run. Saved by 1.4.5 it still says 11; the port's
   tests then edit copies to version 10 and 9 by hand to prove the gates.
3. `resaved-edited-rich-file.comp`: open the existing probe `edited-rich-file.comp` (v9, unknown keys, a text
   layer, folder opacity, guides) in 1.4.5 and Save As. Proves what 1.4.5 does to a v9 file: version 11, unknown
   keys dropped, `QuickLook/Preview.jpg` added, everything else as it was.
4. `resaved-mac-1.2.6.comp`: open the committed `engine/tests/fixtures/mac-1.2.6/` project in 1.4.5 and Save As. A
   full-feature 9 -> 11 manifest diff (adjustments of every kind, effects, masks, clipping, shape, text).
5. Round trip, after the port change: the port writes `port-v11-roundtrip.comp` (a new generator in
   `engine/tests/mac_probes.rs` with a text layer carrying both run kinds, a folder, a mask, an adjustment and
   guides); the user opens it in 1.4.5 (it must open without an error), exports `port-v11-roundtrip.png`, then
   saves it as `port-v11-roundtrip.resaved.comp` and returns both.
6. Negative, made by hand in the port's tests, no Mac needed: runs out of order, overlapping, empty list, zero
   length, past the end, colour out of range, empty font name, newline in a font name; `colorRuns` at version 9,
   `fontRuns` at version 10. Each must be refused, as ProjectStore.swift:210-213 refuses it.

## 2. Behaviour changes to features the port already has

Severity: **wrong result** (the port draws or produces different pixels or data), **missing behaviour** (a
1.4.5 behaviour the port lacks), **cosmetic** (display or wording).

### 2.1 Soft Light: now Core Image, the W3C/PDF formula (wrong result)

- Mac: commit 5a8f6ce "Compute Soft Light through Core Image everywhere" (1.4). `coreImageFilter` now includes
  `.softLight: "CISoftLightBlendMode"` (LayerAppearance.swift:51-58), so every Soft Light layer is drawn through
  `SeparableBlend.draw` (SeparableBlend.swift:22-46) in export, Copy Merged and the canvas; the GPU canvas used
  CISoftLightBlendMode already (GPUCanvas.swift:468-477). The comment says Core Graphics' formula was "up to 25
  levels off Photoshop's with a light blend color (Core Image's is within 5)" (LayerAppearance.swift:52-53).
- Which formula: CISoftLightBlendMode is documented as the PDF blend mode, which is the W3C one:
  `B = cs <= 0.5 ? cb - (1 - 2cs) cb (1 - cb) : cb + (2cs - 1)(D(cb) - cb)`, with
  `D(cb) = cb <= 0.25 ? ((16cb - 12)cb + 4)cb : sqrt(cb)`. The Mac's own test agrees: base 0.5 under 0.9 must
  export 170 +-2 (GPUCanvasTests.swift:524-544). W3C gives 0.6657 = 170; Pegtop gives 0.70 = 178 (fails);
  Photoshop's sqrt-everywhere form also gives 170 there (it differs from W3C only for cb <= 0.25 and cs > 0.5,
  by at most about 5 levels, which is the "within 5" of the comment). It is not Pegtop any more.
- Alpha: it goes through the same `SeparableBlend.draw` path as the eight Core-Image-only modes, which the 1.2.10
  probes measured as `(1 - a) cb + a B(cb, cs)` on the straight source colour (mac-1.2.10-probe-results.md,
  "Blend modes"). The port's compositing (`app/src/canvas/gl/programs.ts:74-80`) already does that.
- Port today: Pegtop on the CPU (`engine/src/blend.rs:13-16`) and the GPU (`app/src/canvas/gl/programs.ts:53`),
  pinned by `engine/tests/mac_1_2_10.rs:183` (blend-greys) and `engine/tests/blend_modes_v9.rs:42-50`.
- Change: W3C Soft Light (the formula the port had before Phase 4a Task 1) in `soft_light` and GLSL mode 15;
  re-pin against 1.4.5 exports.
- Settle with: `blend-greys.comp` and `new-blend-modes.comp` re-exported from 1.4.5, and a new
  `soft-light-dark.comp` probe whose backdrop runs 0-64 (cb <= 0.25) under sources 0.5-1.0, opaque and at 50 %,
  which separates W3C's D(cb) from Photoshop's sqrt.

### 2.2 Adjustment layers and clipping stacks blend in their real mode (wrong result)

- Mac: commits 6dc2219 "Sharpen clipping stacks and blend every mode for adjustments" and 8ab32c9 "Blend
  clipping stacks in every mode off the GPU canvas" (1.4).
  - An adjustment layer in a mode other than Normal blends its result through `SeparableBlend.blend` for every
    mode with a Core Image filter, and through Core Graphics only for the rest (LiveMaskRenderer.swift:52-57,
    SeparableBlend.swift:50-59). In 1.2.10 it always used `blendMode.cgMode`, which is Normal for Linear Burn,
    Linear Dodge, Vivid Light, Linear Light, Pin Light, Hard Mix, Subtract and Divide (LayerAppearance.swift:
    28-48) and Core Graphics' own Color Burn, Color Dodge and Soft Light.
  - A clipping stack's group is blended in its base's real mode: `stackModes` now holds `LayerBlendMode`
    (LiveMaskRenderer.swift:15, :89) and the group goes through `SeparableBlend.draw` when the mode needs a
    surface (:126-134). The Mac test: a Linear Dodge base (0.4, 0.2, 0.1) over (0.3, 0.3, 0.3) with a clipped
    (0.2, 0.05, 0) exports (153, 64, 26) where clipped and (179, 128, 102) where bare (GPUCanvasTests.swift:
    480-513).
- Port today: follows 1.2.10's quirk on purpose. `BlendMode::cg_mode` (`engine/src/manifest.rs:49-64`) maps the
  eight modes to Normal; `plan.rs:184-186` applies it to adjustment layers and `plan.rs:221-223` to stack bases;
  `app/src/canvas/gl/programs.ts:3-4` documents it. Color Burn and Color Dodge already use W3C formulas there
  (`manifest.rs:54-57`), which is now what the Mac does too (Core Image).
- Change: drop `cg_mode` from both plan sites (keep `keeps_alpha` for adjustments: the full-coverage-then-restore-
  alpha path is unchanged, LiveMaskRenderer.swift:36-61); delete `cg_mode` if nothing else reads it. Soft Light
  follows 2.1.
- Settle with: re-exports of the four existing probes that pin the old behaviour (`cgmode-blur-linear-burn`,
  `cgmode-levels-divide`, `color-dodge-adjustment`, `cgmode-stack-bases`; tests at `engine/tests/mac_1_2_10.rs:
  136-157`), plus new `stack-modes.comp` (a clipped stack over the hue sweep with its base in each of the 24
  modes) and `adjustment-modes-*.comp` (Levels-to-grey at 60 % in Soft Light, Linear Dodge, Vivid Light and Hard
  Mix; one file each, since an adjustment reaches everything below it).

### 2.3 Hue/Saturation: Photoshop's positive saturation (wrong result)

- Mac: commit fe7a83d "Match Photoshop's positive Saturation in Hue/Saturation" (1.4). `adjustedSaturation`
  (HueSaturation.swift:342-348): below 0, `s x (1 + a)`; above 0, `s / (1 - a)`, clamped to 1, and at +100 any
  non-grey colour goes to full saturation (a = amount / 100, clamped to -1...1). Used for every range
  (HueSaturation.swift:333); Colorize is unchanged (:322-325). Test: HueSaturationTests.swift:274-281 (0.2 at +50
  -> 0.4; 0.3 at +62 -> 0.3 / 0.38; 0.8 at +50 -> 1).
- The Mac still applies it through a 33-point cube (HueSaturation.swift:233-259), now trilinear on the CPU
  (`cube_apply`, LevelsPixels.c:31) instead of CIColorCube; results may move by a level. The port's per-pixel
  evaluation (spec 4.5) remains a recorded deviation.
- Port today: `saturation * (1 + s / 100)` on the CPU (`engine/src/adjust/hsv.rs:73`) and the GPU
  (`app/src/canvas/gl/programs.ts:262`). Every positive saturation is too weak (+50 gives 1.5x, the Mac 2x).
- Settle with: `hsv-sat-plus-25/50/62/100.comp` and `hsv-sat-minus-50.comp` (Master) and `hsv-reds-plus-50.comp`
  over the tonal sweep, exported from 1.4.5; unit tests copying HueSaturationTests.swift:276-281.

### 2.4 Masks (wrong result, and missing behaviour)

- **Add Mask with a selection now reveals the selection** (commit b3419ab, 1.4). `addMask(revealing:)` fills
  black and paints the clip white when revealing (LayerMask.swift:249-257), and names the step "Reveal
  Selection" or "Hide Selection" (:261). The footer button reveals, Option-click hides (LayerMaskMenu.swift:9-14);
  the layer menu's Add Mask > Reveal All / Hide All call the same function (NativeLayerList.swift:182-194,
  :300-306). Without a selection nothing changed ("Add Reveal-All Mask" / "Add Hide-All Mask", LayerMask.swift:
  270-278). Tests: SelectionEditTests.swift:160-198.
  - Port today: 1.2.10's rule, by the user's 2026-09-27 decision (spec section 3, "Add Mask (Reveal) with a
    selection hides the selection"): `add_mask_from_selection` makes `255 - coverage` when revealing
    (`engine/src/ops/selection.rs:205-225`), undo name "Add Mask from Selection" (`engine/src/command.rs:143`).
    The footer button always passes `revealing = true` and ignores Alt (`app/src/panels/LayersList.tsx:120`).
  - Severity: wrong result (the mask comes out inverted). Needs the user to lift the 2026-09-27 ruling; 1.4.5 now
    agrees with Photoshop, so the deviation has no reason left.
  - Change: swap the two tones in `add_mask_from_selection`; undo names "Reveal Selection" / "Hide Selection";
    Alt-click on the footer button adds the opposite (a black mask, or Hide Selection).
  - Settle with: engine tests from SelectionEditTests.swift:160-198, and a Mac save `mask-reveal-selection.comp`
    (elliptical marquee, antialiased, then the button on one layer and Option-click on another) to pin the mask
    bytes.
- **Option-click on a mask thumbnail shows the mask alone** (b3419ab; cursor 6955a6f). The canvas shows the
  targeted mask in greyscale over the whole canvas, its background tone past its pixels, with strokes and
  gradients as they are laid (EditorCanvas.swift:929-946; state EditorSession.swift:586-593; toggle LayerMask.swift:
  229-236); a badge names it (`MaskAloneBadge`, LayerMaskMenu.swift:22-31 and on). Targeting the pixels, another
  layer, or Option-click again ends it (EditorSession.swift:586). Port: missing (no Alt handling on the mask chip,
  `LayersList.tsx:14-16` uses Ctrl for load-selection only). Missing behaviour, M. Needed before 4c's mask
  painting is comfortable. Settle with: an e2e (Alt-click, the canvas equals the mask's grey values, Alt-click
  back) and MaskAloneTests.swift as the oracle.
- Mask thumbnails show their background white or black (a247676, CanvasThumbnail.swift:35-40). Cosmetic.
- Fill and Gradient on a mask grow it to the canvas: unchanged from 1.3.7 (`growsMask: true` at Gradient.swift:66
  and SelectionEdits.swift:217; `git diff v1.3.7 v1.4.5` of BrushStroke.swift touches only Clone/Blur samples and
  pixel moves). The port's Task 14a matches; no change.
- An unlinked, moved mask under layer effects (8d2eade, 1.2.11) was a Mac bug; the port draws the mask at its own
  placement already (`engine/src/effects/mod.rs:75`). No change.

### 2.5 The Gradient, Fill and the palette / picker

- Gradient maths is unchanged: the diff (commit 5708ed3, 133c34a) only defers filling the tiles until something
  reads them (`GradientEdit.Fill` / `applyFill`, Gradient.swift:34-50, :115) and explains a refused start
  (Gradient.swift:63). Fill is unchanged (SelectionEdits.swift:213-221).
- **Refusal messages** (commit 133c34a, 1.4): a brush, the Gradient, Smudge or Liquify that cannot start says why:
  several layers, a folder, hidden, mask off, adjustment layer, empty selection (EditorSession+Brush.swift:14-33).
  Port: check what the Gradient and Fill do on those targets and use the Mac's words. Missing behaviour, S. Test:
  e2e per case.
- Palette / picker: the picker gains targets for text runs, Dither's two colours and dialog colours
  (ColorPalette.swift:84, :178-195, :265-268); Export JPEG's background and Canvas Size's extension colour use it
  (commit c580a38). The port lists Canvas Size's colour choices as not ported (phase4b1 OQ19). RGB fields scrub by
  dragging their labels (ColorPickerSheet.swift, commit 2d36d6e). Missing behaviour, S; not needed for
  correctness.

### 2.6 Selections

- **Select All then Inverse deselects** (commit 133c34a, 1.4): Inverse of a selection covering the canvas is no
  selection, not an empty one (Selection.swift:354-361; SelectionTests.swift:126-127). Port: keeps an empty
  selection (`engine/src/ops/selection.rs:75-78`), which then blocks Fill, Delete and every later brush without
  marching ants. Wrong state; S. Test: engine unit test (Select All, Inverse -> `selection == None`), undo
  restores the full selection.
- **Cmd-A from the Layers panel** selects the canvas (commit 451281e; NativeLayerList.swift:490-492). Port:
  Ctrl+A is a window-level shortcut (`app/src/shortcuts/useShortcuts.ts:87`) blocked only in editable fields
  (`:161-164`), so it should already do this. Verify with an e2e (click a layer row, Ctrl+A, a canvas-sized
  selection, the layer selection unchanged); no change expected.
- Moving a selection outline snaps to View > Snap To (8b1369a; Crop.swift:197-207); Marquee and shapes snap where
  they start and where their corner goes (d6e3e93; Crop.swift:184-195). Port: no snapping for either (the only
  snapping is `app/src/tools/transform-session.ts:18` for layer moves and `crop-tool.ts:20-36` for crop), and the
  port has no grid or Snap To menu at all (a 1.2.10 gap). Missing behaviour, M with the grid.
- Color Range is new (section 3).

### 2.7 Move

- **Shift keeps moved pixels on a straight line** (commit 60bde4f, 1.4; EditorCanvas.swift:1873-1877): the axis
  the drag has gone further along wins. Moving pixels is Phase 4b-2, not built yet: add this to 4b-2's scope.
- **Move bar values apply without Apply, one undo step** (commit 686d8c7, 1.4.1; TransformInspector.swift:71-88,
  `fromFields` LayerTransform.swift:123). Port: typed values already apply as one step on Enter or blur
  (`app/src/panels/TransformInspector.tsx:64-71`, `:36-46`). Differences left: the Mac has W and H fields
  (TransformInspector.swift:24-25), label scrubbing (`scrubbable`, :117), Escape discards a field's change, and a
  canvas drag first applies a pending field value. Missing behaviour, S.
- **Cancel / Apply only for an edit that waits** (commit 67cc31e, 1.4; TransformInspector.swift:49-60). Port shows
  them only while corners are pending (`TransformInspector.tsx:88-91`), close enough; Ctrl+T's pending edit should
  also show them. Cosmetic.
- **A scaled layer's row gives its scale** ("100 x 100 px . 5%", NativeLayerList.swift:1372-1380). Cosmetic.
- **Resize handles snap** (commit c459f88, 1.4.1; `snappedResizePoint`, Crop.swift:139-182): each moved edge snaps
  on its own; with the ratio kept only the nearer edge snaps; Alt (from the centre) snaps too; Ctrl drags
  freely; a turned layer never snaps. Port: only a move snaps (`transform-session.ts:18-21`). Missing behaviour,
  S-M. Test: port ResizeSnapTests.swift as unit tests on `transform-geometry.ts`.
- **Command flips Auto Select both ways, Shift flips the aspect lock, shown live** (commit bca8f13, 1.4). The
  port has no Auto Select option at all (a 1.2.10 gap; no match for "auto select" in `app/src`). Missing
  behaviour, S-M; add Auto Select first.

### 2.8 Layers and tabs

- **Ungroup Layers, Shift+Ctrl+G** (commits 1318f1e, 6c3b9a5, 1.4.1): the active folder's direct children take
  its place in their inner order, the folder's own opacity, mask and effects go with it, clipping that no longer
  makes sense is released, one undo step "Ungroup Layers", the children end up selected (LayerGroups.swift:
  214-240); Layer menu item with Shift+Cmd+G (CompositorApp.swift:312-313) and the folder's context menu
  (NativeLayerList.swift:159-164). Port: missing (no "ungroup" in `app/src` or `engine/src`); Ctrl+Shift+G is free
  (`app/src/shortcuts/keymap.ts:27-28`). Missing behaviour, S. Test: GroupTests.swift's new cases (commit 1318f1e)
  as engine tests.
- **Tabs: drag to reorder, overflow into a "N more tabs" menu, the selected tab always visible** (commit 1faf7a0,
  1.4.1; ProjectWorkspace.swift:47-49; layout ProjectTabLayout.swift:1-78). Port: tabs in opening order, no drag,
  no overflow (`app/src/panels/ProjectTabs.tsx:10-19`). Missing behaviour, S-M. Test: unit tests on a pure layout
  function, as ProjectTabLayoutTests.swift does.

### 2.9 Drawing maths elsewhere

- **Resampling: an upright layer drawn pixel for pixel now copies its pixels straight across** (commit 8c0417b,
  1.4; `LayerRenderer.interpolation`, LayerRenderer.swift:39-47, also TiledLayerRenderer.swift:208). Export
  uses the same renderer (ImageExporter.swift:48-52), so an unrotated, unscaled layer at a fractional origin (say
  x = 10.5) exports nearest-neighbour in 1.4.5 where 1.2.10 used Core Graphics' Low filter. Port: bilinear
  everywhere. Wrong result, only at fractional positions of 1:1 layers; belongs to Phase 3.5d. Settle with a new
  `sampling-upright-1to1.comp` (1:1 High quality layers at (10.5, 20.25) and (10.25, 20), one flipped) and a
  re-export of `sampling-steps-high-1600.comp` as a canary that the enlarging filter did not move (1.3.5 moved the
  Mac to macOS 26, commit 38ec6f7).
- **Canvas only, no export change** (the port's canvas must still match export, as now): "Draw every canvas frame
  on the GPU" (f773f7b), "Run every adjustment on the GPU canvas" (54dd707), "Show a new mask's layer whole"
  (93de9e7), "Sharper canvas with adjustment layers" (8c0417b, AdjustmentSurface.swift and the `noiseOrigin`
  change at LayerAdjustment.swift:172-177, which is the identity at export scale). The Mac's GPU canvas blends
  every separable mode through Core Image (GPUCanvas.swift:468-487) while export uses Core Graphics for most of
  them; the Mac's own tests hold the two within a mean of 1.5 levels (GPUCanvasTests.swift:480-513), so export
  remains the oracle.
- **Unchanged**: layer effects (no diff in their renderer; MetalLayerEffects.swift and EffectsPreviewCache.swift
  changed only for threading and caching), Levels (`levels_apply` now run in bands, Levels.swift:64-78, same
  table), Curves, Exposure, Gradient Map, Grain and Add Noise in export, Gaussian and Motion Blur, Black & White,
  Color Balance, Invert, the Gradient, Fill, the Shape tool (no ShapeTool.swift diff), Transform Selection and the
  clipboard (no FloatingSelection.swift or SelectionClipboard.swift diff), Spot Healing and Content-Aware Fill (no
  diff in their kernels). Canvas Size and Image Size resampling (no CanvasResizer.swift or ImageResizer.swift
  diff).
- Grain and Add Noise on the Mac canvas now stay anchored to the document when panning and at other zooms
  (54dd707). Port: verify its GLSL keeps document coordinates at 50 % and 200 % (cosmetic if not).

### 2.10 Summary table

| # | Change | Mac commit | Severity | Port file to change |
|---|---|---|---|---|
| 1 | Format 10/11 refused | 2bfc4ea, 0eaeb98 | blocks every 1.4.5 file | engine/src/manifest.rs |
| 2 | Soft Light W3C | 5a8f6ce | wrong result | engine/src/blend.rs, programs.ts |
| 3 | Adjustments / stacks in real mode | 6dc2219, 8ab32c9 | wrong result | engine/src/plan.rs, manifest.rs |
| 4 | Positive saturation | fe7a83d | wrong result | engine/src/adjust/hsv.rs, programs.ts |
| 5 | Add Mask reveals the selection | b3419ab | wrong result (needs the user's ruling) | engine/src/ops/selection.rs, LayersList.tsx |
| 6 | Inverse of everything deselects | 133c34a | wrong state | engine/src/ops/selection.rs |
| 7 | 1:1 upright copies straight | 8c0417b | wrong result (3.5d) | resampling (3.5d) |
| 8 | Mask alone (Alt-click) | b3419ab | missing | LayersList.tsx, renderer |
| 9 | Ungroup Layers | 1318f1e | missing | engine ops, menus, keymap |
| 10 | Resize handles snap | c459f88 | missing | transform-geometry.ts |
| 11 | Move bar W/H, scrub, Escape | 686d8c7 | missing | TransformInspector.tsx |
| 12 | Refusal messages | 133c34a | missing | Gradient / Fill actions |
| 13 | Tabs drag and overflow | 1faf7a0 | missing | ProjectTabs.tsx, store |
| 14 | Shift straight pixel move | 60bde4f | missing (4b-2 scope) | 4b-2 |
| 15 | Snap marquee, shapes, outline moves | d6e3e93, 8b1369a | missing (needs grid / Snap To) | selection and shape tools |
| 16 | Scale in the layer row; thumbnails' background | 67cc31e, a247676 | cosmetic | LayersList.tsx |

## 3. New features in 1.2.11-1.4.5 the spec does not cover

| Feature | What it is (Mac source) | Size | Best phase |
|---|---|---|---|
| Select > Color Range | Click colours on the image as shown (all layers); every pixel whose R, G and B are within Fuzziness (0-200, default 40) of an included colour and of no excluded one; Shift adds, Alt removes, Invert; live selection and a black-and-white preview; OK is one undo step "Color Range" (ColorRangeSelection.swift:1-157; kernel `color_range_mask`, WandPixels.c:184; menu CompositorApp.swift:238-239) | M | a 4a follow-up, alongside 4b-2 (it reuses the wand's trace) |
| Filter > Dither | 11 styles: Atkinson, Floyd-Steinberg, Bayer 2/4/8, halftone dots/lines/diamonds, Mac Patterns, ASCII, Scanlines (CRT); Pixel Size, Pixel Shape (square/dot), Cell Size, Text Size, Line Spacing, Glow, Dots, Wobble, Angle, Tones, Diffusion, Density, Contrast, Colors (B&W / two picked / original), Light on Dark, ASCII characters (Dither.swift:6-220; C kernel DitherPixels.c, 374 lines; DitherPixels.h:48-53). Destructive only; preview at full size (Filters.swift:429-430) | L | a new filters step after Phase 4 (Phase 3's filter set, extended); needs Core Text-like glyph maps for ASCII (Dither.swift:195-219) and a Gaussian for the glow (:146-163) |
| View > Grid Settings | Grid colour (9 presets or custom), style (lines, dashed, dots), opacity 1-100 %, spacing 2-4096 px, subdivisions 1-64; app preference, not saved in projects (Guides.swift:34-120; EditorSession.swift:281-298; GridSettingsSheet.swift) | S (M with the grid itself) | Phase 1/2 follow-up; the port lacks the grid, rulers and View > Snap To entirely, which this needs first |
| Text colour runs and font runs | Colour or face per selected letters; the swatch follows the caret; font menu says (Multiple); font previews in the Type bar; Ctrl+Z while typing (TypeTool.swift:30-186, :432-447; ColorPalette.swift:84-95; commits 2bfc4ea, 0eaeb98, 916d1ab, 1bd1df4, ee68ba9) | L (part of Type) | Phase 5 (the port only keeps them verbatim until then) |
| Camera Raw curves as Photoshop's | Draggable point curve; parametric curve fitted to Photoshop's; the tone curve applies to R, G, B alike; Refine Saturation eases toward luminance-only below 0 (CameraRawColor.swift:48-76; AdjustPixels.c:741-757) | M | Phase 7 |
| Camera Raw: Color Grading under Color | Panel layout (7e9afbe) | S | Phase 7 |
| Smudge and Liquify rework | Smudge without ghost copies, dabs a pixel apart (ad9ad7c; SmudgeLiquify.swift); Liquify keeps pixels sharp by keeping offsets from the untouched layer (7164dd4; MetalWarp.swift); both on the GPU (91cb0b1), faster on big canvases (095da2f) | M | Phase 6 (use 1.4.5 as its oracle) |
| Blur brush Radius | A Radius apart from Strength, default 5 canvas px (504129d; BrushStroke.swift:21) | S | 4c |
| Clone Stamp and Blur at the layer's own resolution | Sample from the layer's own pixels on scaled layers (d3170ab; BrushStroke.swift:155, `gridRect`) | M | 4c |
| Mask alone view | Section 2.4 | M | with the catch-up, or 4c |
| Ungroup Layers | Section 2.8 | S | catch-up |
| Tabs drag and overflow | Section 2.8 | S-M | catch-up or a UI follow-up |
| Snap for marquee, shapes, outline moves, resize handles | Sections 2.6, 2.7 | M | catch-up (resize) and with the grid (the rest) |
| Reload when the package changes on disk | ProjectWatcher.swift, ProjectDigest.swift, ProjectController+ExternalChanges.swift (ed5ddea, 29f0bea); lets scripts and AI agents write projects (docs/writing-comp-files.md) | M | a files follow-up (Tauri fs watcher) |
| Save in the background | Tools wait only while the document is captured; edits during the write stay unsaved (940e188; DocumentHistory.swift:41-44) | S-M | files follow-up; check the port's save first |
| File > Open Recent | With Clear Menu, missing files left out (73dc0f4; RecentProjects.swift). The port lists recents in its File menu already (`app/src/panels/MenuBar.tsx:26-36`); add Clear and pruning | S | UI follow-up |
| New Canvas presets | 4K, 1440p, 1080p, Apple screens, social formats in a More menu (aaf3dc0; NewCanvasSheet.swift) | S | UI follow-up |
| Export JPEG preview | Zoomable full-size preview up to 8192 px; background colour through the app's picker (ac6f309, c580a38; ImageExporter.swift:145) | S | UI follow-up |
| Image Size print sizes | Keep print sizes through an invalid resolution (555417e; ImageSizeSheet.swift) | S | UI follow-up |
| Numeric label scrubbing | Drag a field's label to change it, whole numbers where they belong (2d36d6e, 680a522; NumericScrub.swift) | M (touches every panel) | UI follow-up |
| Hue/Sat, Black & White, Color Balance slider tracks | Coloured tracks, double-click resets (c8ed9ba, 6b709b4) | S | UI follow-up |
| Quick Look / Finder thumbnails | Mac-only (14aad18, 4868465) | n/a | none (optionally write Preview.jpg, section 1.5) |
| PSD import fixes | Levels gamma in hundredths; Hue/Saturation reads Master and ranges; layer masks placed where they sit (8c0417b; PSDReader.swift, PSDDocumentBuilder.swift) | S | Phase 7 |
| Always dark UI, cursors, focus fixes | d62a132, 221d6cc, 810c88a, 6955a6f | cosmetic | none |

## 4. Recommended scope and order: "Phase 4.5, catch up to 1.4.5"

Oracle for the phase and everything after it: Compositor 1.4.5 (v1.4.5, 086f163). Record in the spec (section 3)
that 1.4.5 replaces 1.2.10, and that 1.3.7's mask growth (4b-1 Task 14a) is unchanged in 1.4.5.

Order, each with a portable build at the end of the phase as usual:

1. **F1: prefilter the display level in the job worker** (phase4b1-rulings, "Open items", F1): `RasterInner`
   adopts one halving level; `run_edit_job(.., display_level)` halves in the worker; a fourth job buffer;
   `install_job` adopts it when `output.regions` is empty; the store passes the level the renderer last uploaded.
   Tests: the adopted level equals halving from scratch; no halving after install; the C1 and "jobs" 100 MP
   frame-after budgets back to 350 / 400 ms or lower. With it, the re-review residuals the 4b-1 rulings list as
   "first items of the next phase with F1" (the mask chip while working, `selectLayers` after `commitGradient`,
   `jobs.warm()` placement, surfaced effects-image failures).
2. **Open and save format 11** (section 1.4-1.6): `CURRENT_VERSION = 11`; run validation and version gates in
   `text_is_valid` / `validate`; write 11 (or the lowest needed, if the user asks); tests from fixtures 1-6 of
   section 1.7; update the spec's section 5 and README. Small (about a day), and it unblocks everything: until it
   lands the port cannot open any file the user's Mac saves.
3. **Re-export the existing probe set from 1.4.5** (section 5, item B1) before touching maths, so each fix lands
   against a measured oracle. Keep `mac-1.2.10-probes/` as is; add `mac-1.4.5-probes/` with `*.mac-1.4.5.png`,
   and a `mac_1_4_5.rs` test file mirroring `mac_1_2_10.rs`; a probe whose 1.4.5 PNG equals its 1.2.10 PNG keeps
   its pin.
4. **Fix the wrong results**, one task each, each re-pinned:
   - Soft Light W3C (2.1);
   - adjustment layers and stack bases in their real mode (2.2);
   - Hue/Saturation positive saturation (2.3);
   - Add Mask reveals the selection, Alt for the opposite, the undo names (2.4), after the user lifts the
     2026-09-27 ruling;
   - Inverse of everything deselects (2.6).
5. **Small behaviour changes**: Ungroup Layers (2.8); resize-handle snapping (2.7); Move bar W / H, Escape and
   scrubbing if cheap (2.7); refusal messages (2.5); the scale in the layer row (2.7); verify Ctrl+A from the
   Layers panel (2.6).
6. **Optional in this phase, else a UI follow-up**: the mask-alone view (2.4), tab reordering and overflow (2.8).

Not in this phase: every item in section 3 except those already named above.

How it meets the remaining work:

- **Phase 3.5d (resampling like the Mac).** Unchanged in substance: export still draws scaled layers through Core
  Graphics' filters, so the 1.2.10 step probes stand, if the canary re-export (section 5, B7) matches. Add the new
  rule from 2.9: an upright 1:1 layer is copied straight across (nearest), LayerRenderer.swift:44-47. Best done
  right after this phase: it changes every pinned image of a scaled or fractionally placed layer, and doing it
  after the blend fixes keeps each re-pin attributable.
- **Phase 4b-2 (clipboard, move pixels, Transform Selection).** FloatingSelection.swift and
  SelectionClipboard.swift did not change, so the 1.2.10 research (`phase4b-architecture-and-scope.md`) still
  holds. Add from 1.4.5: Shift keeps a pixel move on one axis (EditorCanvas.swift:1873-1877); moving pixels is
  applied only when read (SelectionEdits.swift:16-31), which is an implementation detail, not behaviour. Do 4b-2
  after this phase, on the 1.4.5 oracle.
- **Phase 4c (brushes).** Changed most: brush strokes on a mask grow it to the canvas (e753f26 / 2ffb092, as 14a
  already does for Fill and Gradient); a mask stroke's new area starts as the mask's background; Clone Stamp and
  Blur sample the layer at its own resolution (d3170ab); Blur has its own Radius (504129d); refusal messages
  (133c34a); `maxDiameter` 2100 for strokes the app lays (BrushStroke.swift:147); the stamp for a
  scaled-down layer is rendered at the layer's resolution. The mask-alone view makes mask painting usable. Refresh
  `phase4-mac-1.2.10-selection-and-retouching.md` against 1.4.5 before planning 4c.
- **Phase 4d (Spot Healing, Content-Aware Fill).** No kernel or flow change found; the 1.2.10 research holds.
- **Phase 5 (text)** must write colour and font runs as 1.4.5 does (section 1.5, item 2), and draw them.
- **Phase 6 and 7** should take 1.4.5 as their oracle (Smudge / Liquify rework; Camera Raw curves; PSD fixes).

## 5. Mac exports the user must make

All from Compositor 1.4.5. "Export" means File > Export > PNG at 100 %. Return saved projects as zipped `.comp`
folders.

A. Format (saved projects):

- A1 `text-runs.comp` + `text-runs.png` (section 1.7, item 1).
- A2 `text-colour-only.comp` (item 2).
- A3 `resaved-edited-rich-file.comp`: the port's existing probe opened and saved as (item 3).
- A4 `resaved-mac-1.2.6.comp`: the committed 1.2.6 fixture opened and saved as (item 4).
- A5 After task 2 ships: open the port's `port-v11-roundtrip.comp`, confirm it opens, export
  `port-v11-roundtrip.png`, save as `port-v11-roundtrip.resaved.comp` (item 5).
- A6 `mask-reveal-selection.comp`: on a 200 x 100 layer, an antialiased elliptical marquee, then the Add Mask button;
  on a second layer the same marquee and Option-click (section 2.4).

B. Maths (open the port's probe projects, export PNGs; the port generates the new ones in `mac_probes.rs`):

- B1 Every project in `engine/tests/fixtures/mac-1.2.10-probes/` exported again as `<name>.mac-1.4.5.png` (48
  projects: the 3.5a, 3.5b and 3.5c sets and the sampling and step probes; the four 4b-1 probes, `shapes.comp` and
  the three gradients, are not in the folder and can be regenerated and redone at the same time). Expected to differ:
  `blend-greys`, `new-blend-modes`, `cgmode-blur-linear-burn`, `cgmode-levels-divide`, `color-dodge-adjustment`,
  `cgmode-stack-bases`; everything else is expected byte-identical, and any that is not is a change this file
  missed.
- B2 `soft-light-dark.comp` (section 2.1).
- B3 `stack-modes.comp` (section 2.2).
- B4 `adjustment-modes-soft-light.comp`, `-linear-dodge.comp`, `-vivid-light.comp`, `-hard-mix.comp` (section 2.2).
- B5 `hsv-sat-plus-25.comp`, `hsv-sat-plus-50.comp`, `hsv-sat-plus-62.comp`, `hsv-sat-plus-100.comp`,
  `hsv-sat-minus-50.comp`, `hsv-reds-plus-50.comp` (section 2.3).
- B6 `sampling-upright-1to1.comp` (section 2.9).
- B7 `sampling-steps-high-1600.comp` again, as the canary for Phase 3.5d (it is also in B1).

C. Later phases (only when those phases start):

- C1 Dither: one `dither-<style>.comp` per style (11) over the tonal sweep at defaults, Filter > Dither applied and
  saved, plus exports; and Scanlines with Glow 0 and 35.
- C2 Color Range: fuzziness 40 on the hue sweep, then Add Mask, saved (`color-range.comp`), since selections are
  not saved.
- C3 Camera Raw curves (Phase 7), brush and Blur probes (4c), Smudge and Liquify probes (Phase 6): to be specified
  in those phases' research.
