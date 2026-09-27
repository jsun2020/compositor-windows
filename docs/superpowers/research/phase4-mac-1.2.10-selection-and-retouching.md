# Phase 4 research: selections and retouching against Compositor for Mac 1.2.10

Date: 2026-09-27. Read-only research; no code changed.

Oracle: `Compositor-1.2.10` (sources `Compositor/`, tests `CompositorTests/`), cited as `MAC <file>:<lines>`. Port: `compositor-windows` at 12a0e34 (0.3.7), cited as `PORT <path>:<lines>`. Spec: `docs/superpowers/specs/2026-09-20-windows-port-design.md` section 3 (Phase 4) and 4.

## 0. Summary

- The Mac keeps a selection as a **vector outline** (`CGPath`, winding rule) plus `antialiased` and `feather`, inside the document so undo covers it, and **never saves it** (MAC Document/EditorSession.swift:70-71; MAC IO/ProjectStore.swift:13-56 has no selection key). Every edit rasterises it on demand into a gray coverage clip (MAC Document/Selection.swift:15-49).
- The spec's list is close to the Mac's, but the Mac has much more around it: Select All / Deselect / Inverse, Expand / Contract / Feather, an Anti-alias toggle, Fill and Clear, Cut / Copy / Paste (also with other apps), Layer via Copy, Transform Selection (the floating selection), Add Mask from Selection, the Brush's Erase mode, Smoothing and mask painting, three Spot Healing modes, the Line shape, and shortcuts the user can remap. These are listed in section 1.14.
- Apple-only pieces: Object Selection and Select Subject use **Vision** (`VNGenerateForegroundInstanceMaskRequest`, macOS 14). Recommend **out** of Phase 4 (section 2). Everything else is CoreGraphics / Core Image / Metal convenience that ports to Rust. The biggest item without a direct Windows equivalent is **CGPath boolean operations** (`union`, `intersection`, `subtracting`, `copy(strokingWithWidth:)`, macOS 14). The port needs a polygon-boolean crate or a mask-based design (section 3.7).
- The port has **no** selection, brush, palette, clipboard or tool beyond Move/Hand/Zoom/Crop. It does have the coverage hook (`blend_by_coverage`, `apply_adjustment(.., selection)`, `histogram(.., coverage)`), which every caller sets to `None`. Its rasters are contiguous whole-layer buffers, **not the tiled copy-on-write storage the spec describes**, and its history holds whole-document snapshots with **no entry or byte cap** (section 3.5). Both matter before a brush exists.
- The proposed split is 4a Selections, 4b Colour, fills and clipboard (including the OQ6 worker), 4c Brushes, 4d Healing and Content-Aware Fill (section 5).

## 1. Feature inventory (Mac 1.2.10)

Undo is the Mac's `beginEdit(name)` / `endEdit()` transaction around a whole-document snapshot (MAC Document/DocumentHistory.swift:50-70). History keeps at most 100 entries and 256 MB of pixels that only history holds (:28, :112-118). "One step" below means one such entry, with the name given in quotes.

### 1.1 Selection model (shared by every selection tool)

- **For the user:** marching ants. An explicit empty selection (every edit refused, "Empty selection" shown) is not the same as no selection (every edit reaches the whole layer). The Anti-alias toggle applies to the Lasso, the Wand and the Elliptical Marquee only (MAC UI/LassoControls.swift:48-52).
- **Mac code:** `DocumentSelection` (MAC Document/Selection.swift:7-30). Coverage bounds and the canvas clip are at :32-49 and `SelectionClip.apply` at :53-66. `applySelection` does replace / add / subtract with CGPath booleans, clipped to the canvas (:233-247). `setSelection` makes one step (:249-254). Select All, Deselect and Inverse are at :344-359.
- **Data model:**
  - `path: CGPath` in document pixels, top-left origin.
  - `antialiased: Bool`, default true.
  - `feather: CGFloat` in document pixels, 0 to 250.
  - Stored in `CanvasDocument.selection`, so undo covers it and saving drops it (MAC Document/EditorSession.swift:70-71).
  - Raster form: a CG winding fill of the path, antialiased when `antialiased || feather > 0`. A feather then adds a Core Image Gaussian with sigma = feather / 2, clamped to the extent (Selection.swift:15-29).
  - The coverage bounds grow by ceil(2 x feather) (:33-36).
  - Adjustments rasterise the coverage on the layer's own pixel grid (MAC Document/PixelAdjust.swift:23-34) and blend `coverage x adjusted + (1 - coverage) x original` (:38-46).
- **Pixel maths:** CoreGraphics path fill and CIGaussianBlur. No C kernel.
- **Modifiers:** Shift adds and Option subtracts, otherwise the options-bar mode (Selection.swift:122-125). The cursor badge keeps an outline's starting mode (:129-139).
- **Undo:** one step per change, named for the tool ("Lasso", "Rectangular Marquee", "Magic Wand", "Select All", "Deselect", "Inverse", and so on).
- **Tests:** SelectionTests `replaceAddAndSubtractCombineOutlines`, `modifiersPickModeAndSelectionIsClippedToCanvas`, `emptySelectionIsDistinctFromNoSelection`, `clickDeselectsAndSelectionStepsUndo`, `antialiasingControlsEdgeCoverage`, `selectAllInverseAndToolSwitchCancelsDraft`, `cursorBadgeFollowsModifiersButKeepsAnOutlinesStartingMode`; HistoryTests `everyLayerEditRoundTripsWithSelection`.
- **Apple-only:** the CGPath booleans (macOS 14 API) and CoreGraphics antialiasing. There is no Windows framework equivalent (section 3.7).

### 1.2 Rectangle and Ellipse Marquee (M)

- **For the user:** the box snaps to whole pixels in any drag direction.
  - Shift squares the box, but only once Shift is pressed afresh during the drag. A Shift already held at the press means Add (MAC Rendering/EditorCanvas.swift:1949-1956, :1963-1964).
  - Option subtracts and never draws from the centre (Selection.swift:152-162).
  - The view autoscrolls near the canvas edge (EditorCanvas.swift:1899-1931).
  - The shape is switched in the tool bar or with Tab. M keeps the shape last used (Selection.swift:182-191).
- **Mac code:**
  - `DragBox.rect` (Selection.swift:93-105), shared with the Shape tool.
  - `beginLasso` / `dragMarquee` / `finishLasso` (:141-231). The ellipse is `addEllipse` in the box (:215-218).
  - Draft overlay: MAC Rendering/TransformOverlay.swift:301-330.
- **Data and maths:** a 4-point draft, then a closed polygon or an ellipse path. A click enclosing nothing deselects in New mode (:224-227).
- **Undo:** one step, "Rectangular Marquee" or "Elliptical Marquee".
- **Tests (SelectionTests):** `marqueeDrawsWholePixelRectanglesInAnyDirection`, `marqueeShiftMakesSquaresAndCenteredDragsGrowFromTheAnchor`, `marqueeEllipseSelectsAnOvalInItsBoxAndShiftMakesACircle`, `optionDraggingTheMarqueeSubtractsWithoutDrawingFromTheCenter`, `mKeyChoosesTheMarqueeAndKeepsTheShapeLastSet`, `shiftStartsAnAddAndOnlyAFreshShiftSquaresTheMarquee`.
- **Apple-only:** none beyond 1.1.

### 1.3 Freehand and Polygonal Lasso (L)

- **For the user:**
  - Freehand follows the drag and skips points closer than 0.25 px (Selection.swift:164-170).
  - Polygonal adds a corner per click and shows a rubber band to the pointer. It closes on a double-click or on a click within 8 view points of the first corner (EditorCanvas.swift:1995-2000).
  - Delete removes the last corner, Return closes and Esc cancels (:1772-1777).
- **Mac code:** Selection.swift:165-231 and EditorCanvas.swift:1961-2002.
- **Data:** `LassoDraft` holds the points, a cursor and the mode chosen at the press (Selection.swift:109-116).
- **Undo:** one step, "Lasso" or "Polygonal Lasso".
- **Tests (SelectionTests):** `polygonalCornersCanBeRemovedAndClosed`, `lKeyChoosesTheLassoAndKeepsTheModeLastSet`.
- **Apple-only:** none.

### 1.4 Magic Wand (W, the Magic tool in Wand mode)

- **For the user:**
  - Tolerance is 0 to 255 per channel, alpha included.
  - Sample size is Point, 3x3 or 5x5, averaged and rounded.
  - Contiguous or not.
  - It reads This Layer (the active layer's own pixels, mask ignored) or All Layers (the visible composite) (MAC Document/MagicWand.swift:3-19, :125-137).
  - A click inside a selection makes a fresh wand selection instead of deselecting (EditorCanvas.swift:1700-1704).
  - A result with too many edges shows "too detailed" (MagicWand.swift:25-31).
- **Mac code:** `MagicWand.select` / `outline` (MagicWand.swift:36-80) and `magicWand(at:mode:)`, which runs detached and sets `isProjectBusy` (:98-123).
- **Pixel maths:** `MAC Rendering/WandPixels.c`.
  - `wand_matches` (:9-15) compares **premultiplied** RGBA from an sRGB premultipliedLast context.
  - `wand_mask` (:17-85) averages the reference colour, then runs a scanline 4-connected flood fill, or a full-image scan when not contiguous.
  - `wand_trace` (:91-174) builds exact pixel-edge loops. It turns right at shared corners so touching loops stay apart, holes wind the other way, and more than 8,000,000 edges is refused (-2).
- **Data:** the traced path. In replace mode the canvas clip is skipped (MagicWand.swift:116-120).
- **Undo:** one step, "Magic Wand".
- **Tests (MagicWandTests, all 6):** `contiguousStopsAtOtherColorsWhileNonContiguousFindsEveryMatch`, `toleranceAppliesToEveryChannelIncludingAlpha`, `sampleSizeAveragesThePixelsAroundTheClick`, `outlinesReproduceTheirPixelsWithHolesAndCornerTouches`, `theWandReadsTheActiveLayerOrEveryVisibleLayerAndCombinesModes`, `clickingInsideASelectionMakesANewWandSelectionRatherThanDeselecting`.
- **Apple-only:** none. The C ports line for line.

### 1.5 Add and subtract, move outline, modify

- **For the user:**
  - Add and subtract: see 1.1. Subtracting from no selection does nothing (Selection.swift:241-244).
  - Move outline: in New mode, dragging inside the selection with a selection tool moves the outline only (EditorCanvas.swift:1977-1982). Offsets are whole pixels and the outline is not re-clipped, so it can leave the canvas and come back whole (Selection.swift:263-283).
  - Shift during that drag locks one axis (EditorCanvas.swift:1932-1942).
  - Arrows nudge the outline 1 px, or 10 px with Shift (:1809-1814; Selection.swift:285-290).
  - Expand and Contract take 1 to 500 px. They add or remove a round-capped band 2|d| wide around the outline, and Contract also moves away from the canvas edges (:316-342).
  - Feather takes 1 to 250 px and combines with an existing feather as sqrt(a^2 + b^2) (:323-331).
  - These are in the tool header (UI/LassoControls.swift:53-72) and in Select > Expand / Contract / Feather, which open `SelectionAmountSheet` (:181-236; MAC CompositorApp.swift:224-229).
- **Undo:** "Move Selection" (one per drag or nudge), "Expand Selection", "Contract Selection", "Feather Selection".
- **Tests:** SelectionTests `draggingMovesTheOutlineInWholePixelsAsOneUndo`, `movingOffCanvasAndBackKeepsTheWholeShape`, `arrowNudgesAndMoveIsOnlyForNewModeOnARealSelection`, `expandAndContractGrowAndShrinkTheOutline`, `expandStaysOnCanvasAndContractCanEmptyTheSelection`; SelectionFeatherTests `featherSoftensTheSelectionAndWhatItClips`.
- **Apple-only:** `CGPath.copy(strokingWithWidth:...)` and the booleans.

### 1.6 Move and duplicate pixels inside

- **For the user:**
  - Cmd-drag inside the selection (any selection tool) cuts the pixels and moves them. Cmd-Option-drag duplicates them (EditorCanvas.swift:1968-1974).
  - Cmd-arrow moves them 1 px, or 10 px with Shift, in **any** tool (:1802-1808).
  - Image pixels only: masks refuse, with a beep.
  - The outline follows the pixels and never flashes at its old place (MAC Document/SelectionEdits.swift:163-172).
- **Mac code:** `PixelMove` (SelectionEdits.swift:14-29) and begin / move / finish / cancel / nudge (:141-198). The raster side is `BrushStroke.liftSelection` / `moveLifted` (MAC Document/BrushStroke.swift:680-747).
- **Maths:** the lifted pixels are drawn through the selection clip. The hole is cleared with `destinationOut` unless duplicating. Whole-pixel offsets are drawn without interpolation.
- **Undo:** one step, "Move Pixels" or "Duplicate Pixels". The pixels and the moved outline change together (`commitRasterEdit(..., alsoApply:)`, MAC Document/EditorSession+Brush.swift:154-188).
- **Tests (SelectionEditTests):** `cmdDragMovesSelectedPixelsAndOutlineAsOneUndo`, `duplicatePixelDragPreservesSourceAndUndoesTogether`, `cmdArrowNudgesPixelsAndMasksRefuse`, `pixelMoveNeverShowsTheOutlineAtItsOldSpot`.
- **Apple-only:** none.

### 1.7 Load layer or mask as selection

- **For the user:**
  - Cmd-click a layer thumbnail selects its pixels that are at least 50% opaque, ignoring the mask.
  - Cmd-click a mask thumbnail selects the mask's **black (hidden) areas**, under 50% gray. This is the opposite of Photoshop, which loads white.
  - Shift adds and Option subtracts (MAC UI/NativeLayerList.swift:1007-1019, :1237).
  - Menu equivalents: Select > Layer's Pixels and Select > Mask's Black Areas (MAC CompositorApp.swift:212-222).
- **Mac code:** `MaskTracing.trace` (MAC Document/MaskTracing.swift:17-70) is a Swift edge tracer that keeps corners only. `loadMaskSelection` and `loadLayerSelection` (:76-94) map the result through the layer or mask transform.
- **Undo:** "Load Mask Selection" or "Load Layer Selection".
- **Tests:** SelectionTests `cmdClickingAMaskSelectsItsBlackAreas`, `maskSelectionFollowsTheLayerTransformAndIgnoresAllWhiteMasks`, `cmdClickingALayerSelectsItsOpaquePixels`; LayerMaskTests `folderMaskCanBePaintedInvertedAndLoadedAsASelection`.
- **Apple-only:** none. The same loop-building job as `wand_trace`, so one Rust tracer can serve both.

### 1.8 Brush (B) and Eraser (E), mask painting

- **For the user:**
  - Size 1 to 2000 px, hardness 0 to 100%, opacity 1 to 100%, and Smoothing 0 to 100. Smoothing is a lazy string, length = smoothing / zoom, Brush only (MAC Document/EditorSession+Brush.swift:77-87; UI/BrushControls.swift:40-81).
  - Erase clears alpha (`destinationOut`).
  - On a mask the brush paints black (hide) or white (reveal) (BrushControls.swift:82-86).
  - Shift-click draws a straight line from the last stroke's end on the same target (EditorSession+Brush.swift:88-92; EditorCanvas.swift:1449-1455).
  - Shift-drag locks the stroke horizontal or vertical, the axis settled after 3 px. Not 45 degrees (EditorCanvas.swift:1552-1568).
  - `[` and `]` step the size by a fifth, at least 1 px. Shift-`[` and Shift-`]` step hardness in quarters (EditorSession+Brush.swift:212-226).
  - Digits set opacity, and two quick digits give an exact value (:191-210).
  - Right-drag resizes the tip; Shift+right-drag changes hardness (EditorCanvas.swift:1347-1381).
  - Option turns the tool into a temporary eyedropper (:71).
  - Esc cancels a stroke in progress (:1768-1770).
  - Painting is refused on: several selected layers, a hidden layer, a disabled mask, an adjustment layer (unless its mask is the target), a folder (unless its mask is the target), and an empty selection (EditorSession+Brush.swift:5-11).
- **Mac code:**
  - `BrushSettings` (BrushStroke.swift:9-25).
  - `BrushStroke`, a tiled raster edit (:91-842). The live stroke replaces the layer's tiles on the canvas (EditorCanvas.swift:820-830).
  - Commit: `commitPaintSnapshot` (EditorSession+Brush.swift:128-150) and `BrushCommit` (BrushStroke.swift:844-901).
  - Brush cursor overlay: EditorCanvas.swift:1157-1178; MAC Rendering/BrushCursorOverlay.swift.
- **Pixel maths:**
  - **GPU path, the default whenever Metal exists** (`useGPU: true`, BrushStroke.swift:149-150). Metal `continuousBrush` (MAC Rendering/MetalBrushCoverage.swift:81-160):
    - Samples are joined by centripetal Catmull-Rom, subdivided until within 0.2 px (BrushStroke.swift:281-309).
    - Hard tips take the distance to the nearest segment with a 1-px antialiased rim.
    - Soft tips integrate optical density along each segment with 8-point Gauss-Legendre quadrature, where density = -log(1 - falloff) and coverage = 1 - exp(-sum). So paint depends on distance travelled, not on the number of events.
    - A provisional tail to the pointer is kept apart from the settled paint.
  - **CPU fallback:** dabs at `diameter x 1.5%` (hard) or `2.5%` (soft) spacing (:409-430). Each dab stamps a pre-rendered tip into per-tile gray coverage with `lighten` (hard) or `screen` (soft) (:501-566).
  - Falloff: a normalised Gaussian with k = 2.5 from the hardness radius to the rim (:74-79).
  - Composite: original + colour x coverage x opacity per tile, so opacity caps the whole stroke (:432-496). The selection clip is applied per tile (:446-452).
  - Painting an image layer grows its grid to the canvas (:162). The commit trims to the alpha bounds with `brush_alpha_bounds` (MAC Rendering/BrushPixels.c:3-21) and grows a covering mask to match (EditorSession+Brush.swift:134-140).
  - Committed tiles are shared immutable patches (MAC Rendering/RasterSnapshot.swift:3-60), so a stroke's history costs only the touched tiles.
- **Undo:** one step per stroke: "Brush Stroke", "Erase" or "Paint Mask" (EditorSession+Brush.swift:141).
- **Tests:**
  - BrushTests (all 19): `bracketKeysReachTheBrushWhereverFocusIsExceptTextFields`, `shiftBracketsStepHardnessFromTheCanvas`, `continuousStrokeCrossesTilesAndCommitsOneUndo`, `softBrushProducesPartialAlphaAndCancelPreservesDocument`, `softMaskPaintingPreviewMatchesCommitAndPersists`, `brushStaysCircularOnNonuniformRotatedFlippedLayer`, `maskedImagePaintingUsesCoverageAndOpacityOnlyOnce`, `importedImageLayerExpandsAcrossCanvasWithoutMovingImageOrMask`, `paintedBoundsTrimTilePaddingAndKeepSoftEdges`, `opacityCapsTheWholeStrokeEvenWhereItOverlapsItself`, `softStrokeBuildsCoverageWhileKeepingItsFeatheredRim`, `spacedDabsLeaveNoVisibleRippleAlongTheStroke`, `sparseMouseSamplesFollowACurveInsteadOfStraightChords`, `liveStrokeReachesNewestSampleAndTailIsReplacedExactly`, `opacityAppliesToMaskPainting`, `shiftBracketsStepHardnessByQuarters`, `numberKeysSetBrushAndGradientOpacity`, `largeBlankCanvasOnlyAllocatesTouchedTilesUntilCommit`, `foldersHiddenLayersAndDisabledMasksRejectPainting`.
  - BrushIntersectionTests: `selfCrossingsBlendInsteadOfTakingTheStrongestEdge`, `accumulationDependsOnDistanceNotEventCount`, `softCrossingsRespectStrokeOpacityAndFlushIsIdempotent`. BrushPerformanceTests: `fourKInteractiveStroke`. RasterSnapshotTests: all 5. TiledLayerTests: `paintingAScaledDownLayerDoesNotShiftItsPixels`, `paintingAScaledDownLayersMaskDoesNotShiftItsPixels`. SelectionEditTests: `brushPaintsOnlyInsideTheSelection`, `emptySelectionEditsNothing`.
- **Apple-only:** Metal for the default coverage. It is portable maths: a per-pixel loop over segments, which can run in Rust on the CPU per tile, as 3.5c did with the effects kernels. The port must choose to match the GPU path, since that is what users see; the dab path differs in shape.

### 1.9 Spot Healing Brush (J)

- **For the user:**
  - Paint over a blemish. It shows as a dark wash (gray 0.12 at 45%) while painting, and on release it is rebuilt from its surroundings (BrushStroke.swift:475-478, :759-815).
  - Three modes, cycled with Tab: Content-Aware, Create Texture and Proximity Match.
  - Image pixels only; masks refuse (EditorSession+Brush.swift:29-31).
  - It shares its tip with the Brush.
- **Mac code:** `BrushStroke.heal()` (BrushStroke.swift:762-815). The work region is the painted bounds padded by (max(w, h) + 32) x 3.2. The seed is `UInt32.random` per stroke.
- **Pixel maths:** `spot_heal` (MAC Rendering/HealPixels.c:115-259; the mode notes are in HealPixels.h:7-18).
  - The ring is size/8, clamped to 2..16.
  - The patch search tries 5 distance factors x 24 angles (2 factors, with a stronger nearness penalty, for Proximity Match), then refines by +-3 px (:159-188).
  - The membrane is the ring difference, spread by an SOR Laplace solve (omega 1.8, 300 iterations, with a coarse-grid start of 40 iterations) (:52-113).
  - Create Texture has no source: it makes a smooth fill plus Gaussian grain from `heal_hash`, scaled to 0.9 x the ring's RMS detail (:192-243).
  - The result blends by coverage x opacity, colour clamped at or below alpha (:225-251).
- **Undo:** one step, "Spot Healing".
- **Tests:** SpotHealingTests `healsTheBlemishUnderTheBrushAndNothingElse`, run for each of the 3 modes.
- **Apple-only:** none.

### 1.10 Clone Stamp (S)

- **For the user:**
  - Option-click sets the source and restarts alignment (MAC Document/CloneStamp.swift:14-18; EditorCanvas.swift:1441-1447).
  - Aligned on: the offset carries over between strokes. Off: each stroke restarts at the source (:20-27).
  - This Layer or All Layers (the visible composite), toggled with Tab (MAC Document/EditorSession.swift:378).
  - It keeps its own tip, soft 40 px at the start (EditorSession.swift:220-225, :338-346).
  - Hovering shows a crosshair at the source and a preview of the stamp inside the brush circle (EditorCanvas.swift:1157-1239).
  - Image pixels only.
- **Mac code:** CloneStamp.swift:4-48 and EditorSession+Brush.swift:33-41. The paint path is at BrushStroke.swift:453-474: the sample is shifted by the offset, drawn through the coverage at the stroke's opacity with medium interpolation, and optionally with `.copy` (`replacesWithClone`).
- **Data:** the sample is a document-size image taken when the stroke starts (CloneStamp.swift:39-48), made from the active layer's displayed transform without its mask, or from `drawLiveComposite`.
- **Undo:** one step, "Clone Stamp".
- **Tests (CloneStampTests):** `copiesTheSourceUnderTheBrushKeepingAlignmentUntilItIsTurnedOff`, `cloneStampKeepsItsOwnSoftBrushTip`.
- **Apple-only:** none.

### 1.11 Blur tool (R, the Smear tool in Blur mode)

- **For the user:** the Smear tool has three modes: Liquify (the default), Blur and Smudge (MAC Document/SmudgeLiquify.swift:10-14; EditorSession.swift:207). **Only Blur is Phase 4.** Liquify and Smudge are `WarpStroke` (SmudgeLiquify.swift:19-199), which is Phase 6.
  - Blur paints a softened copy of the layer, or of its mask, through the tip. The strength is the opacity.
  - The copy is taken per stroke, so going over an area again compounds (MAC Document/BlurTool.swift:5-7).
  - It keeps its own tip (EditorSession.swift:222-225).
- **Mac code:** `blurSample` (BlurTool.swift:8-40). The stroke uses the clone path with a zero offset (EditorSession+Brush.swift:42-46; BrushStroke.swift:453).
- **Maths:** Core Image Gaussian with sigma = clamp(diameter / 10, 1.5, 30).
  - On an image layer it is **not** clamped to the extent, so edges fade toward transparency.
  - On a mask it is clamped, and the area past the mask's pixels takes the mask's edge tone.
- **Undo:** one step, "Blur".
- **Tests:** none found.
- **Apple-only:** Core Image blur only. The port's `gaussian_blur` (PORT engine/src/adjust/filters.rs:79-99) is a candidate, to be checked against CI at small sigmas.

### 1.12 Content-Aware Fill, including extending past edges

- **For the user:** Edit > Content-Aware Fill, or Shift-Delete (EditorCanvas.swift:1756-1759). It needs a non-empty selection on an image layer, not a mask (MAC Document/Filters.swift:467-469). It previews first; OK applies it.
- **Past the edge:** when the selection reaches past the layer, the layer's grid first grows to cover the selection's bounding box within the canvas (Filters.swift:492-499, :353-368, :392-413). The canvas previews on the grown layer (EditorSession.swift:490-491). A covering mask is carried onto the grown grid, with its edge tone beyond the old edge (Filters.swift:625-634).
- **Mac code:** `ContentFill.run` (MAC Document/ContentFill.swift:8-25). It is always previewed at full size (Filters.swift:421) and the cached preview is reused on commit (:606).
- **Pixel maths:** `content_fill` (MAC Rendering/ContentFill.c:23-93).
  - Target = any nonzero selection coverage.
  - Known = unselected pixels with alpha 255.
  - Donors are pixels whose whole 5x5 neighbourhood is known.
  - Filling is BFS order from the boundary. Each pixel tries 4 propagated offsets plus 24 random donors (an LCG seeded with the fixed `0x6d2b79f5`, so the result is deterministic), then a random search at radius 64 down to 1.
  - It copies the best donor pixel. A selected region touched only by transparency is seeded by a scan.
  - No donors gives `Failure.noSource` ("Not enough unselected, opaque image pixels").
  - Soft edges are then blended through the selection by `PixelFilter.run` (Filters.swift:264-265).
- **Undo:** one step, "Content-Aware Fill".
- **Tests (SmartEditTests):** `fillContinuesRepeatingTexture`, `fillReconstructsBackgroundInsideSelectionAndUndoes`, `cancelAndNoSourceLeaveOriginalUntouched`. There is **no Mac test for filling past the layer edge**.
- **Apple-only:** none. The cost is roughly (4 + 24 + 7) x 25 comparisons per filled pixel, which is the spec's "chunked with progress" risk.

### 1.13 Gradient (G), Shape (U), Eyedropper (I), colour picker, Copy Merged

**Gradient**
- **For the user:**
  - Linear or Radial (Tab). Foreground to Background, or Foreground to Transparent (the default). Reverse and Opacity (MAC Document/Gradient.swift:3-19; UI/GradientControls.swift:3-61).
  - Drag the line; Shift snaps it to 45 degrees (EditorCanvas.swift:2034-2040). Endpoints stay draggable (:2022-2032).
  - It stays **pending** until Return or Apply. Switching tool or layer applies it; Esc cancels. The first Undo discards it (Gradient.swift:104-108; EditorSession.swift:573-578).
  - On a mask it draws a gray gradient.
- **Maths:** a two-stop `CGGradient` over the whole canvas (the layer grows to the canvas), clipped to the selection. It is redrawn from each tile's original content, so previews never pile up (BrushStroke.swift:605-623, :645-675).
- **Undo:** "Gradient" or "Gradient Mask".
- **Tests (GradientTests, all 8):** `foregroundToBackgroundFillsCanvasAndCommitsOneUndo`, `radialSpreadsFromStartToRimInEveryDirection`, `reverseOpacityAndDirectionFollowSettings`, `foregroundToTransparentPreservesUnderlyingPixelsAndAlpha`, `cancelUndoAndClicksLeaveDocumentUntouched`, `redraggingReplacesPendingLineWithoutAccumulating`, `maskGradientWritesCoverageInsideLayerBounds`, `paletteChangesUpdatePendingPreview`.
- **Apple-only:** the CGGradient interpolation rule (straight or premultiplied, in sRGB) must be matched and probed.

**Shape**
- **For the user:** Rectangle (corner radius 0 to 5000 px, which clamps to a pill), Ellipse, and **Line** (width 1 to 5000, round caps; not in the spec list) (MAC Document/ShapeTool.swift:3-16; UI/ShapeControls.swift:3-64).
  - Shift gives a square or circle, or snaps a line to 45 degrees. Option draws from the centre (ShapeTool.swift:83-100).
  - Filled with the foreground colour on a **new layer** above the active one, named "Rectangle 1" and so on. It keeps the selection (:113-157).
  - Shift-U or Tab cycles the kind.
  - The max is 200 MP.
- **Data:** the layer's `shape` record, `LayerShapeStyle` (:19-32). This **is** written to the manifest (MAC IO/ProjectStore.swift:52).
- **Undo:** one step, named for the kind.
- **Live redraw:** scaling redraws the shape (`redrawShape`, :161-174); that is Phase 5's "live shape layers".
- **Tests (ShapeToolTests, all 4):** `rectangleFillsANewLayerWithTheForegroundColorAsOneUndoStep`, `ellipseLeavesItsCornersClearWithShiftCircleAndOptionFromCenter`, `aClickEscapeOrToolSwitchMakesNoLayer`, `roundedRectanglesFollowTheRadiusAndClampToAPill`.

**Eyedropper**
- **For the user:** click or drag samples the composited, un-premultiplied colour into the foreground (MAC Document/ColorPalette.swift:180-203; EditorCanvas.swift:2042-2055). A sample ring shows the old and new colours (MAC Rendering/SampleRingOverlay.swift).
  - Option does the same in Brush, Spot Healing and Gradient (EditorCanvas.swift:71).
  - Clicking the canvas while the picker is open samples into the picker (ColorPalette.swift:174-178).
- **Tests:** ColorPickerTests `canvasSamplingReadsCompositeAndCommitsOnlyOnOK`; CanvasEntryTests `eyedropperShortcutSelectsTool`.

**Colour picker and palette**
- **Palette:** foreground and background swatches (MAC UI/ColorPaletteControls.swift). X swaps and D resets. With a mask targeted the palette is only black and white (ColorPalette.swift:20-58).
- **Picker panel** (MAC UI/ColorPickerSheet.swift:7-185):
  - A 256-px saturation/brightness field and a hue strip. HSB is the source of truth, so hue survives through grays (ColorPalette.swift:247-293).
  - R, G and B fields 0 to 255, and hex as RRGGBB or RGB (:295-311).
  - The result is quantised to 8 bits.
  - It is a floating panel that remembers where it was.
  - Targets: the palette, effect colours, Gradient Map ends, Vignette and text (:206-223).
- **Tests (ColorPickerTests):** `hexParsesFullShorthandAndRejectsInvalid`, `hsbRoundTripsEightBitColors`, `graysAndBlackKeepPreviousHueAndSaturation`, `pickerReopensWhereItWasLastLeft`.

**Copy Merged and the clipboard family** (MAC Document/SelectionClipboard.swift)
- **Copy (Cmd-C)** takes the active layer's pixels, or the mask as opaque gray, as they sit on the canvas, through the selection with soft edges kept, at the selection's whole-pixel bounds (:24-62, :103-118). With no selection it copies **the layer itself**, for pasting here or in another project.
- **Copy Merged (Shift-Cmd-C)** composites the visible layers first, then applies the selection (:64-95).
- Both put a PNG on the system pasteboard (:121-129).
- **Paste** makes a new layer above the active one, at the original origin, and drops the selection. An image copied in another app is centred instead (:138-157).
- **Cut** = Copy + Clear (:132-136).
- **Layer via Copy (Cmd-J)** copies the selection to a new layer in place, or duplicates the layer when there is no selection (:161-170).
- **Undo:** "Paste" and "Layer via Copy". Copy records nothing.
- **Tests (SelectionClipboardTests):** `copyAndPastePutsPixelsOnANewLayerInPlace`, `cutLeavesAHoleAndPasteRestoresThePixels`, `layerViaCopyCopiesTheSelectionOrDuplicatesTheLayer`, `lassoShapedSelectionCopiesAndPastes`, `copyMergedTakesEveryVisibleLayerNotJustTheActiveOne`.
- **Apple-only:** NSPasteboard. On Windows this needs a clipboard bridge (section 3.6).

### 1.14 Selection-related features the spec list misses

**Fill and Clear**
- Option-Delete fills with the foreground colour and Cmd-Delete with the background. Delete clears the selected pixels, or on a mask fills the selection with the background (SelectionEdits.swift:40-75; CompositorApp.swift:177-192).
- Delete **without** a selection deletes the mask or the layer (SelectionEdits.swift:61-75).
- Undo: "Fill", "Fill Mask" or "Clear".
- Tests (SelectionEditTests): `fillUsesPaletteInsideSelectionOrWholeLayerWithoutOne`, `deleteClearsSelectedPixelsOrDeletesTheLayerWithoutASelection`, `maskFillHidesOnlyTheSelectedArea`, `deletingWithTheMaskTargetedRemovesOnlyTheMask`.

**Invert inside the selection (Cmd-I)**
- SelectionEdits.swift:77-127. A 1x1 uniform mask is expanded first.
- Tests (SelectionEditTests): `invertKeepsTransparencyStaysInSelectionAndWorksOnMasks`, `invertHandlesLargeImagesAndUniformMasksWithASelection`, `invertWorksInEveryTool`.
- Apple-only: vImage (Accelerate), which is trivial to replace.

**Add Mask from Selection** (MAC Document/LayerMask.swift:233-260)
- The mask is the chosen colour with the selected area painted the opposite. "Reveal" therefore gives a white mask that **hides** the selection, which is the opposite of Photoshop's reveal-selection.
- The selection is used up. Undo: "Add Mask from Selection".
- Tests (SelectionEditTests): `maskButtonAddsWhiteMaskOrHidesTheSelection`, `layerMenuMasksUseTheSelection`, `maskFromSelectionLinesUpOnScaledLayers`.

**Selection-limited adjustments and filters (the spec's coverage item)**
- Every destructive filter and adjustment takes `selection?.clip(...)`: FilterEdit (Filters.swift:499), Levels (MAC Document/Levels.swift:155) and Hue/Saturation (MAC Document/HueSaturation.swift:437). Each blends its result through the selection on the layer's pixel grid (Filters.swift:264-265; Levels.swift:80-82; HueSaturation.swift:244-248).
- The Levels histogram is **weighted by coverage** (Levels.swift:86-101).
- A blur grows the layer and blends on the grown grid.
- **Adjustment layers never take the selection** (MAC Document/AdjustmentEditing.swift:28-56; LayerAdjustment.swift:149-205).
- Tests: HueSaturationTests `adjustmentStaysInsideTheSelectionAndIsOneUndoStep`; LevelsTests `histogramExcludesTransparencyAndWeightsSelection`, `selectionPreviewCancelCommitUndoAndPersistence`; SelectionEditTests `clipFollowsScaledLayersAndSoftensEdges`, `gradientStaysInsideTheSelection`.

**Transform Selection, the floating selection**
- Cmd-T with a selection lifts the pixels onto a temporary "Floating Selection" layer that takes the normal handles, distortion included. On commit it merges back and grows the source layer and its mask (MAC Document/FloatingSelection.swift:6-158).
- Esc restores the document exactly. An unchanged commit restores too, so soft edges get no seam (EditorSession.swift:423-436).
- Undo: one step, "Transform Selection".
- Tests (SelectionClipboardTests): `transformSelectionMovesPixelsAndOutlineAsOneUndo`, `escapeRestoresExactlyWithoutAnUndoStep`, `applyingAnUnchangedTransformLeavesSoftEdgesUntouched`, `scalingAndMovingPastTheLayerEdgeGrowsTheLayer`.

**Crop starts at the selection bounds** (EditorSession.swift:349-358). The port's crop always seeds the full canvas (PORT app/src/state/store.ts:246-255).

**Marching ants detail.** The ants redraw on a 0.12 s timer (EditorCanvas.swift:2004-2020). Past 20,000 path elements and below 1:1 zoom they are drawn from an outline re-traced at screen resolution on a background task (MAC Rendering/TransformOverlay.swift:92-177, :284-298).

**Tool modes and shortcuts**
- Tab cycles the tool's own mode (EditorSession.swift:363-382). A selects no tool. Space held gives the temporary Hand.
- The shortcut table is at MAC UI/KeyboardShortcuts.swift:70-127. The user can remap it, with conflict checks, stored in UserDefaults (:129-213). CursorTests and BlendShortcutTests cover the cursors and keys.

**Remove Background** (Filters.swift:195-196, :644-679; SubjectRemoval.swift) is Vision-based and stays out of v1 per the spec.

**Liquify and Smudge** (SmudgeLiquify.swift) are Phase 6.

## 2. Object selection, guided matte, subject removal

| Feature | Apple framework | Where | Recommendation |
|---|---|---|---|
| Object Selection (Magic tool, Object mode) | **Vision** `VNGenerateForegroundInstanceMaskRequest` (macOS 14), plus Core Image `CIEdgePreserveUpsampleFilter`. The rest is portable: threshold, erode/dilate edge offset +-10, `wand_trace`, and Douglas-Peucker (1.6) + 3x Chaikin smoothing | MAC Document/ObjectSelection.swift:1-298; UI/LassoControls.swift:113-137 | **Out of Phase 4.** It needs an instance-segmentation model, the same blocker the spec gives for Remove Background (spec section 2). Hide the Wand/Object switch until a model is bundled. |
| Select > Subject (Cmd-Opt-A) | **Vision**, the same request, through `SubjectRemoval.subjectMask`, then `MaskTracing.whitePixels` | MAC Document/SubjectRemoval.swift:29-38, :114-142 | **Out**, with Remove Background. |
| Guided Matte | **None.** Pure Swift arithmetic (He/Sun/Tang guided filter, running box sums). Only CGContext is used, for resizing | MAC Document/GuidedMatte.swift:7-117 | **Out for now.** Its only caller is Remove Background "Advanced" (SubjectRemoval.swift:45-83). It ports cheaply once a mask source exists. |
| Floating selection (Transform Selection) | None (CoreGraphics drawing, and `DistortWarp`, which the port already has in `ops/distort.rs`) | MAC Document/FloatingSelection.swift | **In** (part 4b). |

The spec's "candidates" sentence (spec section 3) should therefore resolve to: floating selection in, object selection out.

## 3. What the Windows port already has

### 3.1 Document and manifest: no selection yet

- **Document:** `Document` has id, size, resolution, layers, `active_layer_id`, guides and unknown keys, and no selection (PORT engine/src/document.rs:123-133).
- **Undo comparison:** `same_content` destructures `Document` without `..` (:241-246). Adding a `selection` field is therefore a compile error there until it is compared. That is right, since the Mac's undo covers the selection (HistoryTests `everyLayerEditRoundTripsWithSelection`).
- **Manifest:** the Mac never writes a selection, so the port's manifest needs no field. The top-level unknown-key map already round-trips (document.rs:132, :146).
- **Spec correction:** spec section 5 says opening "restores the selection". There is nothing to restore.

### 3.2 The Phase 3 coverage path

- `blend_by_coverage(adjusted, original, &GrayRaster)` (PORT engine/src/adjust/apply.rs:3-16) matches `PixelAdjust.blend`. `apply_adjustment(.., selection: Option<&GrayRaster>)` (:20-38) is called with `None` in two places:
  - `ops::adjust::apply_adjustment_to_layer` (PORT engine/src/ops/adjust.rs:25);
  - `preview::compute_preview` (PORT engine/src/preview.rs:94).
- `adjust::levels::histogram(raster, coverage: Option<&GrayRaster>)` weights by coverage (PORT engine/src/adjust/levels.rs:30-35). `Engine::histogram` passes `None` (PORT engine/src/engine.rs:432-438).
- Not yet selection-aware:
  - `apply_filter` (ops/adjust.rs:108-132) and the filter preview (preview.rs:97-108): no coverage argument. The grown grid needs its own coverage raster.
  - `invert_layer` (ops/adjust.rs:30-42).
  - `Command::ApplyAdjustment`, `ApplyFilter` and `InvertPixels` (PORT engine/src/command.rs:47-49).
- **Missing piece:** a "selection to coverage on this layer's pixel grid" rasteriser, the equivalent of `PixelAdjust.coverage` (MAC Document/PixelAdjust.swift:23-34), which goes through `pixel_to_document`. It would be shared with brushes, fills and clipboard ops.
- `plan::Coverage` (PORT engine/src/plan.rs:8-17) is **mask** coverage in the render plan, not selection. Do not conflate the two.

### 3.3 Mask editing today

- `ops/masks.rs` (PORT engine/src/ops/masks.rs:10-94) has add (as a 1x1 uniform mask), delete, enable, link, invert, fill, blur, `expand_uniform` and copy.
- There is **no mask painting and no selection-aware mask op**. `FillMask` fills the whole mask.
- The Layer menu exposes these (PORT app/src/panels/MenuBar.tsx:69-77).
- Painting a mask arrives with the 4c brush.

### 3.4 App tool framework

- **Tools:** `type Tool = "move" | "hand" | "zoom" | "crop"` (PORT app/src/state/store.ts:13). `setTool` commits a pending transform and seeds the crop rectangle (:246-255). The tool rail is a static list of four (PORT app/src/panels/ToolRail.tsx:2-4).
- **Pointer handling:** `CanvasView` registers one `useEffect` of pointerdown / move / up listeners per tool, and each returns early unless `s.tool` matches (PORT app/src/canvas/CanvasView.tsx):
  - hand :160-169; zoom :171-191; crop :193-224; move :226-293;
  - a capture-phase listener for panel eyedroppers :133-158.
  - Gesture state lives in pure, unit-tested session classes (`tools/crop-tool.ts`, `tools/transform-session.ts`).
- **Overlay:** a separate 2D canvas redrawn from `OverlayState` (pixel grid, guides, crop frame, transform handles, snap lines) whenever the store changes (PORT app/src/canvas/overlay.ts:7-92; CanvasView.tsx:89-104).
  - No animation timer (the ants need one).
  - No cursor overlay beyond CSS `cursor` (CanvasView.tsx:154-158).
- **Store:** `run(command)` commits a pending transform, executes and refreshes (store.ts:218-241). `panelOwnsDocument` blocks edits while a panel is open (:387-391). `previewEdit` sends transform or adjustment-layer drafts to the render plan (:379-386).
- **Shortcuts:**
  - A static `SHORTCUTS` table (PORT app/src/shortcuts/keymap.ts:10-33) and a `runAction` switch (PORT app/src/shortcuts/useShortcuts.ts:13-99).
  - Tool letters are V, H, Z and C only. Opacity digits work only in Move (:97).
  - Delete always deletes layers (keymap.ts:28).
  - No user remapping.
- **Colour sampling:**
  - `Engine::sample_color` composites one pixel from the stored document (engine.rs:481-487).
  - `sample_layer_color` (:445-466) and `store.sampleAt` (store.ts:435-454) serve the Levels and Hue/Saturation eyedroppers only.
  - No palette, eyedropper tool or colour picker. New Gradient Map layers start black to white "until Phase 4 adds the palette" (PORT app/src/actions/layers.ts:86-91).
- **Clipboard:** none. `ShellBridge` has no clipboard call (PORT app/src/shell/bridge.ts:3-19), and the Tauri shell has only the dialog plugin (PORT src-tauri/Cargo.toml:14-15).

### 3.5 History, raster storage and uploads (the brush's real prerequisites)

- **Rasters:** `Raster` is one contiguous `Arc<Vec<u8>>` per layer (PORT engine/src/raster.rs:17-20; GrayRaster :133-153). `tile_rgba` exists only as a copy-out helper (:79-91).
  - This is **not** the 256-px copy-on-write tiling of spec section 4.1.
- **History:** `History` stores whole `Document` clones with Arc-shared rasters (PORT engine/src/history.rs:3-48) and has **no entry or byte limit**. `Engine::edit` clones, mutates and pushes (engine.rs:219-232).
  - Each stroke replacing a layer's buffer keeps a full copy of the old one in history: about 48 MB per stroke on a 4000x3000 layer.
  - The Mac caps history at 100 entries / 256 MB (DocumentHistory.swift:28, :112-118) and shares untouched tiles (RasterSnapshot.swift:3-60).
- **Previews:** `PixelPreview` substitutes a whole-layer raster (preview.rs:34-36; engine.rs:183-216).
- **GPU uploads:** the renderer re-uploads a layer whole, in 2048-px `texImage2D` chunks, whenever its revision changes (PORT app/src/canvas/layer-textures.ts:54-83; gl-renderer.ts:87-130). Masks work the same way (PORT app/src/canvas/gl/mask-textures.ts:10-21).
  - There is no `texSubImage2D` dirty-rect path. A live brush tick would re-upload the layer, and on a styled layer re-render its effects image.
- **Revisions:** revisions come from one engine-wide counter (engine.rs:64-90), which a dirty-tile scheme can keep using.
- **Phase 3 N3:** `set_preview`'s dedupe key omits the stored revision (PORT docs/superpowers/phase3-rulings-and-open-items.md:55-62). A live-stroke preview path must fix N3 or state the invariant.

### 3.6 Available building blocks

- `compositor::composite_edit_with(doc, None, region, w, h, cache)` (PORT engine/src/compositor.rs:435) gives All Layers samples and Copy Merged.
- `alpha_bounds` (:61) is the equivalent of `brush_alpha_bounds`.
- `ops::adjust::grown` / `trimmed` / `carry_mask` (ops/adjust.rs:44-106) are exactly what Content-Aware Fill's growing, brush canvas-growing and floating merge need.
- `ops::distort` is available for the floating selection.
- `Layer::set_pixels` already drops `shape` and `text` when pixels change (PORT engine/src/document.rs:56-67), matching the Mac's `liveShape`.
- `LayerExtra.shape` is kept as opaque JSON (:16). The Shape tool must write the Mac's `LayerShapeStyle` keys.

### 3.7 Selection representation: what to build

The Mac's parity tests depend on vector behaviour:
- moving off the canvas and back keeps the shape (`movingOffCanvasAndBackKeepsTheWholeShape`);
- the ellipse is antialiased while rectangles are pixel-exact;
- Expand and Contract are stroked bands;
- the ants follow a path.

Recommendation: keep a vector outline in the engine, as polygons with the ellipse flattened finely, plus `antialiased` and `feather`.
- Booleans and offsetting come from a pure-Rust polygon crate that builds for wasm32 (for example `i_overlay`, which also offsets with round joins).
- Rasterise coverage with an antialiasing scanline filler, then the existing separable Gaussian for feather.
- Trace wand and mask results with a Rust `wand_trace`.

The Mac probes then settle CoreGraphics-versus-port edge coverage within a tolerance.

A canvas-sized 8-bit mask would be simpler, but it breaks off-canvas moves and makes Expand a morphology that differs from the Mac's.

## 4. The Mac project format for selections

- **Not saved.** `ProjectManifest` has format, version, colorSpace, resolution, documentID, width, height, activeLayerID, layers and guides (MAC IO/ProjectStore.swift:13-31). `ProjectLayerRecord` has no selection field (:33-56).
- `CanvasDocument.selection` is "Not saved to disk" (MAC Document/EditorSession.swift:70-71).
- Nothing in Phase 4 changes the format, except that the Shape tool **writes** `shape` (`LayerShapeStyle`: kind, red, green, blue, cornerRadius, and the optional lineWidth, start and end, MAC Document/ShapeTool.swift:19-32). That shape is documented in PORT docs/superpowers/research/mac-1.2.6-format-delta.md section 2.3.

## 5. Proposed split

Each part ends with a portable build, in dependency order, as 3.5a/b/c did. Each part adds its own shortcuts to the keymap. Photoshop-for-Windows equivalents: Cmd is Ctrl and Option is Alt. Alt+Backspace fills with the foreground, Ctrl+Backspace with the background, and Shift+Backspace opens Content-Aware Fill.

### 4a Selections

**Scope.** The engine selection model (outline, antialias, feather; part of `Document`, so undo covers it, and never saved).
- Tools: Rectangle and Ellipse Marquee (M), Freehand and Polygonal Lasso (L), Magic Wand (W, Wand mode only) with its full options.
- Replace, add and subtract by options bar or Shift/Alt.
- Move outline by drag and arrows. Select All, Deselect, Inverse. Expand, Contract and Feather in the header and the Select menu. The Anti-alias toggle.
- Load layer or mask as selection: Ctrl-click thumbnails with Shift/Alt, and the menu.
- Marching ants on a timer, with the screen-resolution LOD for complex outlines.
- Selection-limited destructive adjustments, filters (grown grid) and Invert, and the Levels/Curves histogram weighted by coverage.
- Delete clears the selected pixels (routing Delete to the Mac's `deleteKeyPressed`). Add Mask from Selection. The Crop tool seeds from the selection bounds.
- Refuse edits on an empty selection.

The result is usable: select an area, then adjust, filter, clear or mask just that area.

**Engine pieces:**
- `Selection` type, polygon booleans and offset, AA rasteriser, coverage on a layer grid.
- `wand.rs` (`wand_mask`, `wand_trace`) and a mask/alpha tracer.
- Commands: `SetSelection`, `CombineSelection`, `MoveSelection`, `ExpandSelection`, `ContractSelection`, `FeatherSelection`, `InvertSelection`, `SelectAll`, `Deselect`, `LoadLayerSelection`, `LoadMaskSelection`, `MagicWand` (sample: layer or composite), `ClearSelectedPixels`, `AddMaskFromSelection`.
- A selection argument on `ApplyAdjustment`, `ApplyFilter`, `InvertPixels`, the previews and `histogram`.
- `DocumentState` carries the outline for the overlay, as polygons or a simplified outline plus bounds.
- Fix N3.

**App pieces:**
- Tool registry: extend `Tool`, the rail and the per-tool pointer handlers, and add a `selection-session.ts` alongside `crop-tool.ts`.
- Overlay: ants, draft, polygon handles, cursor badges.
- Select menu, header controls, amount sheet, Tab mode cycling, autoscroll.
- Keys: M, L, W, Ctrl+A/D, Ctrl+Shift+I, arrows.
- Tests: port the SelectionTests, MagicWandTests and SelectionFeatherTests cases, plus the selection cases of LevelsTests, HueSaturationTests and SelectionEditTests.

### 4b Colour, fills, gradient, shapes and the pixel clipboard

**Scope:**
- Foreground/background palette with X and D (mask: black and white). The colour picker panel (HSB field, hue strip, RGB, hex, canvas sampling while open). The Eyedropper tool (I), with its sample ring.
- Fill with foreground or background inside the selection.
- Gradient tool: linear or radial, both styles, reverse, opacity. Pending edit with draggable ends and 45-degree Shift. Mask gradients.
- Shape tool: rectangle with corner radius, ellipse, and line if ruled in. It writes the Mac `shape` record.
- Cut, Copy, Paste, Copy Merged, Layer via Copy. Move and duplicate pixels (Ctrl-drag, Ctrl+Alt-drag, Ctrl+arrows). Transform Selection (Ctrl+T with a selection).
- Gradient Map layers start from the palette.

This part also builds the **raster-edit infrastructure** that 4c reuses:
- An engine "raster edit" session: grow to the canvas, clip to the selection, trim on commit, carry the mask.
- A history entry and byte cap matching the Mac (100 / 256 MB).
- Dirty-rect `texSubImage2D` uploads for layers and masks.
- **The recorded prerequisite (3.5c OQ6):** reduced display effects images (at most 1536 px, as MAC Rendering/EffectsPreviewCache.swift:51, :73) built **off the UI thread** in a Web Worker holding its own engine instance. Gradient drags and pixel moves on a styled layer are the first continuous pixel previews, so OQ6 lands here rather than waiting for the brush.

**Engine pieces:**
- Raster-edit ops: `FillSelection`, `Gradient`, `LiftAndMovePixels`, `FloatingMerge`, `AddShapeLayer`, `PasteLayer`.
- `copy_region` and `copy_merged` returning PNG bytes.
- Colour quantising helpers.

**App pieces:**
- Palette widget, picker panel, eyedropper, gradient and shape headers, the Edit menu clipboard group.
- A clipboard bridge. Internal copies can stay in memory; system PNG interop needs WebView2's `navigator.clipboard` or a Tauri clipboard plugin.
- The worker.
- Tests: ColorPickerTests, GradientTests, ShapeToolTests, SelectionClipboardTests and SelectionEditTests (fill, move, invert).

### 4c Brushes

**Scope:**
- Brush and Eraser (B, E) with size, hardness, opacity and smoothing. Shift-click lines and Shift axis lock. `[` and `]`, Shift-`[` and Shift-`]`, digit opacity, right-drag size and hardness, Alt eyedropper.
- Mask painting in black or white, folder masks included. Painting clipped to the selection. The brush cursor circle.
- Blur tool: the Smear tool's Blur mode only, with Liquify and Smudge hidden until Phase 6.
- Clone Stamp (S): Alt-click source, Aligned, This Layer / All Layers, its own tip, the hover preview and source crosshair.
- Live strokes update only the touched tiles, and the effects preview updates through the 4b worker.

**Engine pieces:**
- `brush.rs`: a Rust port of the Metal continuous-coverage kernel (MetalBrushCoverage.swift:81-160) with the Catmull-Rom subdivision, per 256-px tile, with a provisional tail. Decide whether the dab fallback is needed at all.
- A stroke session API: begin / append / flush / cancel / commit, one history entry.
- Sampling for clone and blur (layer or composite at document size, a Gaussian at the Mac's sigma).
- A `brush_alpha_bounds` trim.

**App pieces:** brush header, cursor overlay, pointer smoothing, and tip families (Brush/Spot, Clone, Smear).

**Tests:** BrushTests, BrushIntersectionTests, RasterSnapshotTests, CloneStampTests, and a 60 fps performance probe at 4000x3000 (spec section 6).

### 4d Healing and Content-Aware Fill

**Scope:**
- Spot Healing Brush (J) with Content-Aware, Create Texture and Proximity Match. It shows the wash while painting and heals on release.
- Content-Aware Fill (Edit menu, Shift+Backspace) with preview and OK/Cancel. It **extends past the layer edge** by growing the grid to the selection bounds within the canvas and carrying the mask.
- Both run chunked in the worker with progress (spec section 11 risk).

**Engine pieces:**
- `heal.rs`: a line-by-line port of `HealPixels.c`, taking a seed parameter so tests are deterministic.
- `content_fill.rs`: a port of `ContentFill.c`, with the same LCG seed.
- A grown-grid preview in `preview.rs`.

**App pieces:** Spot Healing mode control, filter panel reuse, and progress UI.

**Tests:** SpotHealingTests (3 modes) and SmartEditTests. Add a new past-edge test and a Mac probe, since the Mac has none.

## 6. Open questions for the user

1. **Object Selection and Select Subject:** confirm both are out of Phase 4 and deferred with Remove Background (they need a bundled segmentation model).
2. **Mac quirks versus Photoshop:** the oracle loads a mask's *black* areas on Ctrl-click, and "Add Mask (Reveal)" with a selection *hides* the selection (sections 1.7 and 1.14). Follow the Mac, as the port has done so far, or Photoshop?
3. **Shape tool extent:** include the Mac's Line shape (not in the spec list)? And should Phase 4 also redraw shapes on scale (`redrawShape`), or leave live shapes to Phase 5 so a Windows-drawn shape stretches until then?
4. **System clipboard:** must Phase 4 copy to and paste from other Windows apps (PNG), as the Mac does, or is an in-app clipboard enough for now? Related: the Mac's whole-layer Copy/Paste between projects is close to the spec's v1 non-goal "dragging layers between open projects". In or out?
5. **Remappable shortcuts:** the Mac has a Keyboard Shortcuts editor. Is a fixed Photoshop-style map enough for Phase 4?
