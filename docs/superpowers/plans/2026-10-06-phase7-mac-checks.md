# Phase 7 Mac 1.4.5 checks

Use the generated Phase 7 probe folder. Work on copies; keep all source files and
the Windows expected exports unchanged. Record failed steps too. Never undo the
final accepted edit before saving the edited project.

## Camera Raw (four independent projects)

For each `NN-…-source.comp`, open it in Compositor **1.4.5**. Select the
`Camera Raw probe` layer. Choose **Filter → Camera Raw**. Leave parameters not
listed below at their initial defaults. Expand the named section to find its
controls. Type a number and press Return to commit that field.

| Project | Values to change |
| --- | --- |
| 01-light-color-curve | Light: Exposure **0.5**, Contrast **12**, Highlights **−20**, Shadows **15**. Color: Temperature **10**, Tint **−6**, Vibrance **18**. Curve: Darks **−10**. |
| 02-mixer-grading | Color Mixer: Reds Hue **12**, Blues Saturation **−20**, Greens Luminance **10**. Color Grading: Shadows Hue **210**, Saturation **15**; Highlights Hue **40**, Saturation **12**. |
| 03-effects-detail-optics | Effects: Texture **12**, Clarity **10**, Dehaze **8**, Vignette Amount **−15**. Detail: Sharpening Amount **35**, Noise Reduction Luminance **15**. Optics: Distortion **8**. |
| 04-geometry-calibration | Geometry: Rotate **3**, Vertical **12**, Horizontal **−6**, Aspect **8**. Calibration: Shadow Tint **10**, Red Primary Hue **15**, Blue Primary Saturation **−10**. |

For each project:

1. Change the listed values. Toggle **Preview** off/on and confirm that only the
   on state shows the changes. Click **Cancel**; confirm the source is restored.
2. Open Camera Raw again, enter the same values and click **OK/Apply**. Press
   **Cmd+Z** once to restore the source; **Cmd+Shift+Z** once to restore the edit.
3. Keep the edited result. Save a new copy as `NN-…-mac-edited.comp`; export a
   transparent PNG at **512 × 320**, named `NN-…-mac.png`.
4. Close/reopen the edited copy and confirm it retains the applied pixels.

On one copy also check white-balance Auto/eyedropper, Point Color eyedropper,
group enable/reset, a dragged point-curve handle, clipping overlays, sharpening
mask and Histogram/Vectorscope switching. These diagnostic views must disappear
after Apply. Record the exact gesture and result; a saved project cannot prove
Cancel or preview behaviour.

## Photoshop files

Open the four synthetic PSD/PSB files through **File → Import** (or the Mac's
image-open route if it presents the import dialog). Save the imported project as
`<original-stem>-mac.comp`; export `<original-stem>-mac.png` at **512 × 320**.

- `01-groups-masks-clipping.psd` and `.psb`: folder nesting, hidden green layer,
  Unicode layer name, Multiply blend, partial opacity, raster mask and clipping.
  Use different output names for PSD and PSB, for example `01-…-psd-mac.comp`
  and `01-…-psb-mac.comp`.
- `02-editable-text-shape.psd`: double-click text, append ` X`, keep/save it;
  resize the rectangle, keep/save it. Both should remain editable after reopening.
  Record font fallback or any conversion warning. Also retain an untouched import
  export before these edits for comparison.
- `03-adjustment-conversions.psd`: Levels remains editable; unsupported Posterize
  has an explicit conversion notice. Record the complete warning text.
- Also import a copy of the supplied real PSD and return its `.comp` and PNG.
  Retain its original layer names, visibility and blend modes.

## Camera sensor RAW

Try the synthetic `bayer-no-preview.dng`. It has no embedded JPEG/thumbnail.
Record whether Apple CIRAWFilter accepts it; rejection does not prove a LibRaw
defect. The final real-camera acceptance still needs a camera RAW sample.

For an accepted synthetic or real RAW:

1. Open/import it. Record initial dimensions and As Shot white balance.
2. Change Exposure to **+1**, Temperature to **4000**, Tint to **+20** and the
   tone/boost control if available. Check that preview responds; Cancel imports
   nothing.
3. Open again. Change settings, then **Reset to As Shot**; Import the reset result.
   Save/export as `raw-as-shot-mac.comp` / `raw-as-shot-mac.png`.
4. Open again, set only Exposure **+1**, Import, save/export as
   `raw-exposure-plus1-mac.comp` / `.png`. Close/reopen both projects.

Apple and LibRaw use different colour/tone pipelines. These RAW checks compare
actual sensor import, controls, dimensions, cancellation and persistence; they
do not require identical RGB pixels between the decoders.

## Return evidence

Return the Mac `.comp` **folders with their images**, exports, and `RESULT.txt`.
In `RESULT.txt`, record Mac/application versions, each completed gesture, any
missing controls, warnings or failures. Copy the entire return folder back to
the Windows Phase 7 probe directory. Leave originals unchanged.
