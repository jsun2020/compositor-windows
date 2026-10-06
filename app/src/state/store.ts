import { create } from "zustand";
import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, LevelsAuto, PreviewEdit, SelectionMode, WandSettings } from "../engine/types";
import { DEFAULT_WAND } from "../engine/types";
import type { EngineClient, JobInputCopy } from "../engine/client";
import {releaseJobResult,type JobClient,type JobResult} from "../engine/jobs";
import { cropSeed } from "../tools/crop-tool";
import type { LassoKind, MarqueeKind, SelectionDraft } from "../tools/selection-draft";
import type { ShellBridge } from "../shell/bridge";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";
import { cornersOf, cornersToTuples, isValidTransform, roundedTransform } from "../tools/transform-geometry";
import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } from "./selection";
import { reorderedTabs } from "./tab-reorder";
import type { AdjustEdit, SampleMode } from "./adjust-edit";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor,effectiveFilterParams } from "./adjust-edit";
import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";
import { DEFAULT_SHAPE, nextShapeKind, shapeSpec, type ShapeDraft, type ShapeOptions } from "../tools/shape-draft";
import { DEFAULT_GRADIENT, gradientSpec, hasLine, type GradientEdit, type GradientOptions } from "./gradient-edit";
import { BLACK, WHITE, hsbOf, hsbToRgb, quantized, sameColor, withRgb, type PaletteColor, type PickerHSB } from "../tools/color";
import { DEFAULT_BRUSH, cancelBrush, type BrushOptions, type BrushDraft } from "../tools/brush";
import type { ContentFillEdit } from "../actions/content-fill";
import {cancelText,type TextEdit} from "../actions/text";
import {cancelEffects,type EffectsEdit} from "../actions/effects";

export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand" | "eyedropper" | "gradient" | "shape" | "brush" | "eraser" | "blur" | "clone" | "healing" | "text";
export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
/** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" } | {kind:"rawDevelop"} | {kind:"importReport"}
  | { kind: "selectionAmount"; operation: SelectionAmountOperation };

/** The selection tools' settings, kept per app session as the Mac keeps them per session
 * (EditorSession.swift:232-285): the options bar's mode, Anti-alias, the Marquee's and the Lasso's
 * kinds, the Magic Wand's settings, and the Expand / Contract / Feather amounts. */
export interface SelectionOptions {
  mode: SelectionMode; antialiased: boolean; marquee: MarqueeKind; lasso: LassoKind; wand: WandSettings;
  expand: number; contract: number; feather: number;
}
export const DEFAULT_SELECTION_OPTIONS: SelectionOptions = {
  mode: "Replace", antialiased: true, marquee: "Rectangle", lasso: "Freehand", wand: DEFAULT_WAND, expand: 1, contract: 1, feather: 2,
};
/** The largest amount each operation takes (Selection.swift:307). */
export const SELECTION_AMOUNT_MAX: Record<SelectionAmountOperation, number> = { Expand: 500, Contract: 500, Feather: 250 };

export interface TransformEdit {
  floating?: boolean;
  kind: "layer" | "group" | "mask";
  id: string;
  ids: string[];
  box: LayerTransform;
  original: LayerTransform;
  draft: LayerTransform;
  corners: Corners | null;
  persistent: boolean;
  duplicated: boolean;
  /** For a duplicated edit, the id of the history entry the DuplicateLayer that created the copy
   * pushed (`undoEntryId` read straight after it). `cancelTransform` reverts only while that entry
   * is still the one on top: nothing slipped in behind it. An id, not a depth: at the history cap
   * the duplicate's push trims the oldest entry and the depth does not move. Null when the edit
   * duplicated nothing. */
  duplicateEntry: number | null;
}

/** The palette (ColorPalette.swift): the image's foreground and background colours, and, while a
 * mask is the target, whether white is its foreground (`maskPaintWhite`; black otherwise). Kept for
 * the app session, as the Mac keeps it per window. */
export interface Palette { foreground: PaletteColor; background: PaletteColor; maskPaintWhite: boolean; }
export const DEFAULT_PALETTE: Palette = { foreground: BLACK, background: WHITE, maskPaintWhite: false };

/** What the open colour picker edits (`ColorPickerTarget`): a palette swatch, or one end of the
 * Gradient Map being edited in its panel. */
export type PickerTarget = { kind: "palette"; background: boolean } | { kind: "gradientMap"; highlights: boolean };
/** The open picker (`ColorPickerState`): its target, the colour it opened on, and its working values.
 * Nothing reaches a swatch until OK; a Gradient Map end previews the working colour and Cancel puts
 * the original back. */
export interface ColorPicker { target: PickerTarget; original: PaletteColor; hsb: PickerHSB; }
/** The picker's title bar (`ColorPickerTarget.title`). */
export function pickerTitle(target: PickerTarget): string {
  if (target.kind === "palette") return target.background ? "Color Picker (Background Color)" : "Color Picker (Foreground Color)";
  return target.highlights ? "Color Picker (Gradient Map Highlights)" : "Color Picker (Gradient Map Shadows)";
}
/** The ring shown while the canvas is being sampled (`SampleRingOverlay`): where, in view px, the
 * colour sampled and the colour before sampling began. */
export interface SampleRing { at: { x: number; y: number }; sampled: PaletteColor; original: PaletteColor; }

/** A layer with more pixels than this is edited, and its histogram read, by the job worker rather than
 * on the UI thread (ruling OQ5): at 4 MP a Levels commit took about 0.35 s here. Below it a job's two
 * copies and the worker's round trip cost more than they save. */
export const JOB_PIXELS = 4_000_000;
/** Said when a command arrives while a job's result is still to come. */
export const BUSY_MESSAGE = "Wait for the current edit to finish.";

export interface EditorStore {
  textEdit:TextEdit|null;
  effectsEdit:EffectsEdit|null;
  cloneSource:{document:string;point:[number,number]}|null;
  cloneOffset:[number,number]|null;
  contentFill: ContentFillEdit | null;
  brushOptions: BrushOptions;
  brushDraft: BrushDraft | null;
  setBrushOptions(patch: Partial<BrushOptions>): void;
  invalidateOverlay(): void;
  engine: EngineClient | null;
  /** The job worker's client (engine `jobs.rs`); null until the engine has loaded. */
  jobs: JobClient | null;
  /** Layers with more pixels than this use the job worker (`JOB_PIXELS`; tests lower it). */
  jobPixels: number;
  /** True while an edit job's result is still to come (`runEditJob`): the Mac's `isProjectBusy`. */
  working: boolean;
  bridge: ShellBridge | null;
  documents: Record<string, DocumentState>;
  order: string[];
  activeId: string | null;
  viewports: Record<string, Viewport>;
  tool: Tool;
  cropRect: Rect | null;
  cropRatio: CropRatio;
  sheet: Sheet;
  error: string | null;
  busy: boolean;
  rendererKind: "gl" | "cpu" | null;
  renderTick: number;
  /** Bumped when only the overlay changes (an outline being drawn or dragged): no re-render of the picture. */
  overlayTick: number;
  recentTick: number;
  selectedLayerIds: string[];
  maskSelected: boolean;
  collapsed: Record<string, string[]>;
  transformEdit: TransformEdit | null;
  snapGuides: { xs: number[]; ys: number[] };
  /** The Move bar's aspect lock (`locksTransformRatio`, EditorSession.swift:197 at v1.4.5): whether a handle and a
   * typed W or H keep the ratio. On at first, and not saved. */
  locksTransformRatio: boolean;
  /** Whether the document's saved guides are drawn (View > Hide/Show Guides). Persisted so the
   * choice survives a relaunch, as it does on the Mac. */
  showGuides: boolean;
  blendPreview: BlendMode | null;
  adjustEdit: AdjustEdit | null;
  selectionOptions: SelectionOptions;
  /** A Marquee or Lasso outline being drawn; never in the document until it is finished. */
  selectionDraft: SelectionDraft | null;
  /** The whole-pixel offset of the outline while it is being dragged; sent as one MoveSelection on release. */
  outlineMove: { dx: number; dy: number } | null;
  /** The mode Shift / Alt held over the canvas imply, for the options bar (`heldSelectionMode`). */
  heldSelectionMode: SelectionMode | null;
  palette: Palette;
  /** Whether a mask is the paint target: the mask chip of the active layer, which has one (`isMaskSelected`). */
  maskTargeted(): boolean;
  /** The foreground or background colour the tools use: black or white while a mask is the target (`paletteColor`). */
  paletteColor(background: boolean): PaletteColor;
  /** Sets one swatch; while a mask is the target only black or white, as which of them is the foreground
   * (`setPaletteColor`). Nothing while a job's result is to come (`canEditPalette`). */
  setPaletteColor(color: PaletteColor, background: boolean): void;
  /** X: swap the swatches, or black and white on a mask (`swapPaletteColors`). */
  swapPalette(): void;
  /** D: black over white, or black as the mask's foreground (`resetPaletteColors`). */
  resetPalette(): void;
  colorPicker: ColorPicker | null;
  /** Where the picker's panel was last left (its top-left, in window px); null until it is first moved. */
  pickerAt: { x: number; y: number } | null;
  sampleRing: SampleRing | null;
  /** The picker's working colour, snapped to 8 bits (`ColorPickerState.color`). */
  pickerColor(): PaletteColor | null;
  /** Opens the picker on a swatch (not while a mask is the target: the swatch asks black or white
   * instead) or on a Gradient Map end while that panel is open. Opening on a swatch while it is open
   * on the other one switches it (`openColorPicker`, `openGradientMapColorPicker`). */
  openColorPicker(target: PickerTarget): boolean;
  /** The picker's new values; a Gradient Map end previews them at once (`previewGradientMapColor`). */
  setPickerHsb(hsb: PickerHSB): void;
  /** OK (`commit`) or Cancel (`closeColorPicker`). */
  closeColorPicker(commit: boolean): void;
  /** Loads the canvas colour under a document point into the open picker (`sampleIntoColorPicker`). */
  sampleIntoPicker(at: { x: number; y: number }): void;
  setPickerAt(at: { x: number; y: number }): void;
  /** The Eyedropper: the canvas colour under a document point becomes the image's foreground, even
   * while a mask is the target (`sampleColor`, EditorCanvas.swift:2045-2048). Nothing off the canvas,
   * over a transparent pixel, or while a job's result is to come. */
  sampleForeground(at: { x: number; y: number }): void;
  gradientOptions: GradientOptions;
  /** The gradient drawn but not yet applied (Gradient.swift): the engine previews it. While it is
   * pending any other command applies it first (as a pending transform is committed), the first Undo
   * discards it, and a change of tool, layer or target applies it (`resolveGradient`). */
  gradientEdit: GradientEdit | null;
  /** New settings; a pending gradient is drawn again with them (`gradientSettings` didSet). `dragging`
   * previews from a reduced copy, for the opacity slider's continuous input events (fix round 1, M-1);
   * every other setting settles at once. */
  setGradientOptions(patch: Partial<GradientOptions>, dragging?: boolean): void;
  /** A press with the Gradient tool: a new line from `at` on the active layer or its mask (a pending
   * one on the same target starts again there). False when nothing can be painted (`beginGradient`). */
  beginGradient(at: { x: number; y: number }): boolean;
  /** Moves an end of the pending line and previews it, from a reduced copy while `dragging`. */
  moveGradient(ends: { start?: { x: number; y: number }; end?: { x: number; y: number } }, dragging: boolean): void;
  /** The drag is over: a click without a line leaves nothing pending, else the full preview (`endGradientDrag`). */
  endGradientDrag(): void;
  /** Re-previews the pending gradient, from a reduced copy while `dragging` (settings or palette changed). */
  refreshGradient(dragging?: boolean): void;
  cancelGradient(): void;
  /** Return or Apply: paints the pending gradient as one undo step ("Gradient" or "Gradient Mask"),
   * through the job worker on a large layer (`commitGradient`). */
  commitGradient(): void;
  shapeOptions: ShapeOptions;
  /** A shape being dragged out with the Shape tool; drawn on the overlay only, never in the document. */
  shapeDraft: ShapeDraft | null;
  /** New settings; a new kind drops a draft being drawn (ShapeControls.swift:9-12). */
  setShapeOptions(patch: Partial<ShapeOptions>): void;
  setShapeDraft(draft: ShapeDraft | null): void;
  /** The release: the draft filled with the image's foreground colour on a new layer above the
   * active one, one undo step named for its kind; a click makes nothing (`finishShape`). */
  finishShape(): void;
  setSampleRing(ring: SampleRing | null): void;
  setEngine(engine: EngineClient): void;
  setJobs(jobs: JobClient): void;
  /** Whether an edit of `layerId`'s pixels goes to the job worker: it has more than `jobPixels`. */
  usesJob(layerId: string): boolean;
  /** Runs `command` on `layerId` in the job worker and puts the result back as one undo step, unless
   * the layer changed meanwhile. The document is busy until then: other commands, undo and redo wait.
   * The canvas keeps what it showed (an open panel's preview) until the result is in. */
  runEditJob(command: Command, layerId: string): Promise<boolean>;
  setBridge(bridge: ShellBridge): void;
  setBusy(busy: boolean): void;
  bumpRecent(): void;
  openDocument(id: string): void;
  closeDocument(id: string): void;
  setActive(id: string): void;
  /** A tab dragged to `index` in the strip's order (`ProjectWorkspace.moveTab`): chrome, no undo step. */
  moveTab(id: string, index: number): void;
  refresh(id?: string): void;
  revealActiveLayer(): void;
  /** True when the engine accepted the command; a refusal raises the banner. */
  run(command: Command): boolean;
  undo(): void;
  redo(): void;
  setTool(tool: Tool): void;
  setCropRect(rect: Rect | null): void;
  setCropRatio(ratio: CropRatio): void;
  openSheet(sheet: Sheet): void;
  closeSheet(): void;
  setError(error: string | null): void;
  setRendererKind(kind: "gl" | "cpu"): void;
  invalidate(): void;
  repaintOverlay(): void;
  selectLayers(ids: string[], primary: string | null): void;
  setMaskSelected(v: boolean): void;
  toggleCollapsed(id: string): void;
  beginTransform(opts: { persistent: boolean; duplicate?: boolean }): boolean;
  previewTransform(draft: LayerTransform, corners?: Corners | null): void;
  beginDistort(): void;
  commitTransform(): void;
  cancelTransform(): void;
  setSnapGuides(g: { xs: number[]; ys: number[] }): void;
  setLocksTransformRatio(v: boolean): void;
  toggleGuides(): void;
  setBlendPreview(m: BlendMode | null): void;
  previewEdit(): PreviewEdit | null;
  /** True while an adjustment or filter panel is open. The panel owns the document then, as
   * macOS's canEditLayers/canUseHistory make it: nothing may record history, change the
   * selection, open a sheet or disturb the panel's preview. With `refuse`, also raises the
   * refusal banner. Every such entry point and menu flag goes through this one guard. */
  panelOwnsDocument(refuse?: boolean): boolean;
  canAdjust(): boolean;
  beginAdjust(opts: { kind: AdjustmentKind | FilterKind; layerId?: string; target?: "layer" | "adjustmentLayer" }): boolean;
  updateAdjust(patch: { adjustment?: AdjustEdit["adjustment"]; params?: AdjustEdit["params"]; cameraRawBypass?:AdjustEdit["cameraRawBypass"];cameraRawView?:AdjustEdit["cameraRawView"] }): void;
  setAdjustPreview(on: boolean): void;
  setAdjustSample(mode: SampleMode | null): void;
  sampleAt(at: { x: number; y: number }): void;
  autoLevels(mode: LevelsAuto): void;
  /** Pushes the open panel's current settings to the engine as a preview. Not part of the
   * Task 12 brief's public action list, but needed by beginAdjust/updateAdjust/setAdjustPreview,
   * which all share it rather than duplicating the branch between a pixel-layer preview (through
   * `engine.setPreview`) and an adjustment-layer preview (through the render plan). */
  applyAdjustPreview(dragging?: boolean): void;
  /** True while a quick drag preview is showing and the full-quality one is still to come. */
  previewSettling(): boolean;
  commitAdjust(): void;
  cancelAdjust(): void;
  setSelectionOptions(patch: Partial<SelectionOptions>): void;
  setSelectionDraft(draft: SelectionDraft | null): void;
  /** Sends the draft's outline to the engine as one SelectShape and drops the draft. */
  finishSelectionDraft(): void;
  setOutlineMove(offset: { dx: number; dy: number } | null): void;
  setHeldSelectionMode(mode: SelectionMode | null): void;
  /** Expand / Contract / Feather by `amount`, remembered as that operation's amount. False when out of range or refused. */
  modifySelection(operation: SelectionAmountOperation, amount: number): boolean;
  /** Tab: the Marquee's Rectangle / Ellipse, the Lasso's Freehand / Polygonal (`cycleToolMode`). */
  cycleToolMode(): void;
  /** Whether the document has a selection with something in it (`canModifySelection`, less the draft rule). */
  hasSelection(): boolean;
}

/** Commands that insert a layer or move one into a folder, and so make it active somewhere the
 * panel may not be showing. Each is followed by `revealActiveLayer`. */
const REVEALING_COMMANDS: ReadonlySet<Command["type"]> = new Set<Command["type"]>([
  "AddBlankLayer", "AddShape", "AddGroup", "GroupLayers", "PlaceLayer", "DuplicateLayer", "DuplicateLayerTo", "DuplicateLayerTransformed", "MergeLayers", "DeleteLayers", "DeleteLayer",
]);

/** How long after the last slider tick a colour adjustment's quick drag preview is replaced by
 * the full-quality one (engine/src/preview.rs explains the two sizes). */
export const SETTLE_MS = 150;
let settleTimer: ReturnType<typeof setTimeout> | null = null;
let histogramRequestId = 0;
function cancelSettle(): void {
  if (settleTimer !== null) { clearTimeout(settleTimer); settleTimer = null; }
}

/** Closes an open panel without applying it and clears its engine-side preview. `adjustEdit`,
 * like `transformEdit`, names no document of its own: it always belongs to whatever is
 * `activeId` at the time, so that is the document whose preview is cleared. Callers leaving or
 * closing the active document use this; closing some other, background tab must not. */
function dropOpenPanel(): void {
  cancelText();cancelEffects();
  cancelSettle();
  const { adjustEdit, engine, activeId } = useEditor.getState();
  if (!adjustEdit) return;
  // A picker open on one of the panel's Gradient Map ends goes with it.
  useEditor.setState({ adjustEdit: null, ...(useEditor.getState().colorPicker?.target.kind === "gradientMap" ? { colorPicker: null } : {}) });
  if (activeId) engine!.setPreview(activeId, null);
}

/** Sets one end of the open Gradient Map panel to `color`, previewing it (`setGradientMapColor`). */
function setGradientMapEnd(highlights: boolean, color: PaletteColor): void {
  const edit = useEditor.getState().adjustEdit;
  const settings = edit?.adjustment?.gradientMapSettings;
  if (!edit?.adjustment || !settings || edit.adjustment.kind !== "Gradient Map") return;
  const end = { red: color.red, green: color.green, blue: color.blue };
  const current = highlights ? settings.highlights : settings.shadows;
  if (current.red === end.red && current.green === end.green && current.blue === end.blue) return;
  useEditor.getState().updateAdjust({ adjustment: { ...edit.adjustment, gradientMapSettings: { ...settings, [highlights ? "highlights" : "shadows"]: end } } });
}
/** Whether the active layer's pixels, or its mask, can take paint now (`canPaint`,
 * EditorSession+Brush.swift:5-11): one layer selected and shown, not a folder unless its mask is the
 * target, an enabled mask when it is, not an adjustment layer, no empty selection, no panel open, no
 * job's result to come. */
export function canPaintNow(): boolean {
  const s = useEditor.getState();
  const doc = s.activeId ? s.documents[s.activeId] : null;
  const layer = doc ? activeLayer(doc) : null;
  if (!doc || !layer || s.selectedLayerIds.length !== 1 || s.panelOwnsDocument() || s.working) return false;
  if (doc.selection?.empty || !visibleIds(doc).has(layer.id)) return false;
  return s.maskTargeted() ? layer.maskEnabled : !layer.isGroup && !layer.adjustment;
}
/** The pending gradient as the engine paints it, in the palette's colours now. */
function currentSpec(e: GradientEdit) {
  const s = useEditor.getState();
  return gradientSpec(e, s.gradientOptions, s.paletteColor(false), s.paletteColor(true));
}
/** Shows the pending gradient: the engine's preview while it has a line, nothing otherwise. */
function previewGradient(dragging: boolean): void {
  const s = useEditor.getState(); const e = s.gradientEdit;
  if (!e || !s.engine || !s.activeId) return;
  s.engine.setPreview(s.activeId, hasLine(e) ? { preview: "Gradient", layer: e.layerId, mask: e.mask, gradient: currentSpec(e), dragging } : null);
  s.refresh(s.activeId);
}
/** `withRgb` from an engine sample. */
const withRgbFrom = (hsb: PickerHSB, rgb: [number, number, number]): PickerHSB => withRgb(hsb, { red: rgb[0], green: rgb[1], blue: rgb[2] });

const GUIDES_KEY = "compositor.showGuides";
function loadShowGuides(): boolean {
  try { return localStorage.getItem(GUIDES_KEY) !== "false"; } catch { return true; }
}

export const useEditor = create<EditorStore>((set, get) => ({
  textEdit:null,effectsEdit:null,
  cloneSource:null,cloneOffset:null,
  contentFill:null,
  brushOptions: DEFAULT_BRUSH, brushDraft: null,
  setBrushOptions: (patch) => {
    if(get().brushDraft)cancelBrush();
    const next = { ...get().brushOptions };
    for (const key of ["diameter","hardness","opacity","strength","smoothing","blurRadius"] as const) {
      const v=patch[key]; if (v===undefined || !Number.isFinite(v)) continue;
      const [lo,hi]=key==="diameter"?[1,2000]:key==="opacity"||key==="strength"?[0.01,1]:key==="hardness"?[0,1]:key==="blurRadius"?[0.5,50]:[0,100];
      next[key]=Math.min(hi,Math.max(lo,v));
    }
    if(typeof patch.aligned==="boolean")next.aligned=patch.aligned;
    if(typeof patch.allLayers==="boolean")next.allLayers=patch.allLayers;
    if(patch.smearMode&&["Blur","Liquify","Smudge"].includes(patch.smearMode))next.smearMode=patch.smearMode;
    if(patch.healingMode&&["Content-Aware","Create Texture","Proximity Match"].includes(patch.healingMode))next.healingMode=patch.healingMode;
    set({brushOptions:next});
  },
  invalidateOverlay: () => set(s=>({overlayTick:s.overlayTick+1})),
  engine: null, jobs: null, jobPixels: JOB_PIXELS, working: false, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, overlayTick: 0, recentTick: 0,
  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, locksTransformRatio: true, showGuides: loadShowGuides(),
  blendPreview: null,
  adjustEdit: null,
  selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
  palette: DEFAULT_PALETTE, colorPicker: null, pickerAt: null, sampleRing: null,
  gradientOptions: DEFAULT_GRADIENT, gradientEdit: null,
  shapeOptions: DEFAULT_SHAPE, shapeDraft: null,
  setShapeOptions: (patch) => {
    const kindChanged = patch.kind !== undefined && patch.kind !== get().shapeOptions.kind;
    set((s) => ({ shapeOptions: { ...s.shapeOptions, ...patch }, ...(kindChanged ? { shapeDraft: null } : {}) }));
    if (kindChanged) get().repaintOverlay();
  },
  setShapeDraft: (shapeDraft) => { set({ shapeDraft }); get().repaintOverlay(); },
  finishShape: () => {
    const draft = get().shapeDraft; if (!draft) return;
    set({ shapeDraft: null });
    get().repaintOverlay();
    const spec = shapeSpec(draft, get().shapeOptions.lineWidth);
    if (!spec) return;
    const { red, green, blue } = get().palette.foreground;
    get().run({ type: "AddShape", shape: spec, color: [red, green, blue] });
  },
  setGradientOptions: (patch, dragging = false) => { set((s) => ({ gradientOptions: { ...s.gradientOptions, ...patch } })); get().refreshGradient(dragging); },
  beginGradient: (at) => {
    const { activeId, documents, gradientEdit } = get(); if (!activeId) return false;
    const doc = documents[activeId];
    const layer = activeLayer(doc); if (!layer) return false;
    const mask = get().maskTargeted();
    // Dragging a new line replaces the pending one on the same target.
    if (gradientEdit && gradientEdit.layerId === layer.id && gradientEdit.mask === mask) {
      set({ gradientEdit: { ...gradientEdit, start: at, end: at } });
      get().engine!.setPreview(activeId, null); get().refresh(activeId);
      return true;
    }
    if (gradientEdit) get().commitGradient();
    if (!canPaintNow()) return false;
    set({ gradientEdit: { layerId: layer.id, mask, start: at, end: at } });
    return true;
  },
  moveGradient: (ends, dragging) => {
    const e = get().gradientEdit; if (!e) return;
    set({ gradientEdit: { ...e, ...(ends.start ? { start: ends.start } : {}), ...(ends.end ? { end: ends.end } : {}) } });
    previewGradient(dragging);
  },
  endGradientDrag: () => {
    const e = get().gradientEdit; if (!e) return;
    if (!hasLine(e)) { get().cancelGradient(); return; }
    previewGradient(false);
  },
  refreshGradient: (dragging = false) => { if (get().gradientEdit) previewGradient(dragging); else get().repaintOverlay(); },
  cancelGradient: () => {
    const { gradientEdit, engine, activeId } = get(); if (!gradientEdit) return;
    set({ gradientEdit: null });
    if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); }
  },
  commitGradient: () => {
    const e = get().gradientEdit; if (!e) return;
    if (!hasLine(e) || get().working) { get().cancelGradient(); return; }
    const command: Command = { type: "Gradient", id: e.layerId, mask: e.mask, gradient: currentSpec(e) };
    set({ gradientEdit: null });
    // The preview stays on screen until the result is put back (runEditJob clears it then).
    // By the pixels the gradient paints (ruling C1, as `fillActive` does; Task 14a): on a mask, the mask
    // grown to the canvas, so a small layer's mask on a large canvas is a large edit. One too large to
    // paint is refused here, as the commit would refuse it: the job worker sees one layer, not the budget
    // the others leave.
    // Asked only with a job worker, as `usesJob` and `fillActive` do.
    if (get().jobs) {
      let painted: number;
      try { painted = get().engine!.editPixels(get().activeId!, e.layerId, e.mask); }
      catch (err) {
        set({ error: String(err instanceof Error ? err.message : err) });
        const { engine, activeId } = get(); if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); }
        return;
      }
      if (painted > get().jobPixels) { void get().runEditJob(command, e.layerId); return; }
    }
    if (!get().run(command)) { const { engine, activeId } = get(); if (engine && activeId) { engine.setPreview(activeId, null); get().refresh(activeId); } }
  },
  pickerColor: () => { const p = get().colorPicker; return p ? quantized(hsbToRgb(p.hsb)) : null; },
  openColorPicker: (target) => {
    if (get().working) return false;
    let original: PaletteColor;
    if (target.kind === "palette") {
      if (get().maskTargeted()) return false;
      original = get().paletteColor(target.background);
    } else {
      const edit = get().adjustEdit;
      const settings = edit?.adjustment?.kind === "Gradient Map" && !edit.params ? edit.adjustment.gradientMapSettings : undefined;
      if (!settings || get().colorPicker) return false;
      original = target.highlights ? settings.highlights : settings.shadows;
    }
    set({ colorPicker: { target, original, hsb: hsbOf(original) } });
    return true;
  },
  setPickerHsb: (hsb) => {
    const picker = get().colorPicker; if (!picker) return;
    set({ colorPicker: { ...picker, hsb } });
    if (picker.target.kind === "gradientMap") setGradientMapEnd(picker.target.highlights, get().pickerColor()!);
  },
  closeColorPicker: (commit) => {
    const picker = get().colorPicker; if (!picker) return;
    // OK on a swatch while a job's result is to come: the palette does not change then
    // (`canEditPalette`), so the picker stays open with the colour chosen, and says why, rather than
    // close and lose it (final review minor 2).
    if (commit && picker.target.kind === "palette" && get().working) { set({ error: BUSY_MESSAGE }); return; }
    const color = get().pickerColor()!;
    set({ colorPicker: null });
    if (picker.target.kind === "palette") { if (commit && !get().maskTargeted()) get().setPaletteColor(color, picker.target.background); }
    else setGradientMapEnd(picker.target.highlights, commit ? color : picker.original);
  },
  sampleIntoPicker: (at) => {
    const { colorPicker, engine, activeId } = get(); if (!colorPicker || !engine || !activeId) return;
    const rgb = engine.sampleColor(activeId, at);
    if (rgb) get().setPickerHsb(withRgbFrom(colorPicker.hsb, rgb));
  },
  setPickerAt: (pickerAt) => set({ pickerAt }),
  sampleForeground: (at) => {
    const { engine, activeId, working } = get(); if (!engine || !activeId || working) return;
    const rgb = engine.sampleColor(activeId, at);
    if (rgb) { set({ palette: { ...get().palette, foreground: { red: rgb[0], green: rgb[1], blue: rgb[2] } } }); get().refreshGradient(); }
  },
  setSampleRing: (sampleRing) => { set({ sampleRing }); get().repaintOverlay(); },
  maskTargeted: () => {
    const { activeId, documents, maskSelected } = get();
    const doc = activeId ? documents[activeId] : null;
    return maskSelected && !!doc && !!activeLayer(doc)?.hasMask;
  },
  paletteColor: (background) => {
    const p = get().palette;
    if (get().maskTargeted()) return (background ? !p.maskPaintWhite : p.maskPaintWhite) ? WHITE : BLACK;
    return background ? p.background : p.foreground;
  },
  setPaletteColor: (color, background) => {
    if (get().working) return;
    const p = get().palette;
    if (get().maskTargeted()) {
      const white = sameColor(color, WHITE);
      set({ palette: { ...p, maskPaintWhite: background ? !white : white } });
    } else set({ palette: background ? { ...p, background: color } : { ...p, foreground: color } });
    get().refreshGradient();
  },
  swapPalette: () => {
    if (get().working) return;
    const p = get().palette;
    set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: !p.maskPaintWhite } : { ...p, foreground: p.background, background: p.foreground } });
    get().refreshGradient();
  },
  resetPalette: () => {
    if (get().working) return;
    const p = get().palette;
    set({ palette: get().maskTargeted() ? { ...p, maskPaintWhite: false } : { ...p, foreground: BLACK, background: WHITE } });
    get().refreshGradient();
  },
  setEngine: (engine) => set({ engine }),
  setJobs: (jobs) => set({ jobs }),
  usesJob: (layerId) => {
    const { jobs, engine, activeId, documents, jobPixels } = get();
    const layer = activeId ? documents[activeId]?.layers.find((l) => l.id === layerId) : undefined;
    // The stored layer's size: under an open preview the state shows the preview's, maybe reduced.
    return !!jobs && !!engine && !!layer && engine.storedPixels(activeId!, layerId) > jobPixels;
  },
  runEditJob: async (command, layerId) => {
    const { engine, jobs, activeId } = get();
    if (!engine || !jobs || !activeId) return false;
    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
    const doc = activeId;
    const allLayers=command.type==="BrushStroke"&&command.brush.operation?.kind==="Clone"&&command.brush.operation.allLayers;
    let copy: JobInputCopy;
    let preparation: (() => void) | null = null;
    const releasePreparation = () => { const release = preparation; preparation = null; release?.(); };
    set({ working: true });
    try { preparation=jobs.prepareInteractive();const prepared=allLayers ? {input:engine.jobHeader(doc,layerId),pixels:null,mask:null,points:null} : engine.jobInputAsync(doc, layerId);copy=prepared instanceof Promise?await prepared:prepared; }
    catch (e) {
      releasePreparation();
      set({ error: String(e instanceof Error ? e.message : e) });
      // Input preparation refused before the job's finally block: clear its preview and busy flag.
      engine.setPreview(doc, null); get().refresh(doc);
      set({working:false});
      return false;
    }
    set({ working: true });
    let installed = false,result:JobResult|null=null;
    // The scale the canvas draws this document at, in device pixels per document pixel: the worker
    // halves the result to the level the renderer will upload it at (F1).
    const outPerDoc = (get().viewports[doc]?.pointsPerPixel ?? 0) * (globalThis.devicePixelRatio || 1);
    try {
      const request=allLayers ? { kind:"documentEdit" as const,...await engine.clipboardInputAsync(doc,layerId,false,true),layer:layerId,pixels:null,mask:null,command:JSON.stringify(command),outPerDoc }
        : {kind:"edit" as const,input:copy.input,pixels:copy.pixels,mask:copy.mask,points:copy.points,command:JSON.stringify(command),outPerDoc};
      const pending = jobs.run(`edit:${doc}`,request);
      releasePreparation();
      result = await pending;
      // Closed meanwhile: nothing to put back.
      if (!result || !get().documents[doc]) return false;
      await engine.installJobAsync(doc, layerId, copy.input, result.header!, result.pixels, result.mask, result.display ?? null);
      installed = true;
      return true;
    } catch (e) {
      set({ error: String(e instanceof Error ? e.message : e) });
      return false;
    } finally {
      releasePreparation();
      releaseJobResult(result);
      set({ working: false });
      if (get().documents[doc]) {
        // A result that was not put back leaves a panel's preview behind: take it away.
        if (!installed) engine.setPreview(doc, null);
        get().refresh(doc);
      }
    }
  },
  setBridge: (bridge) => set({ bridge }),
  setBusy: (busy) => set({ busy }),
  bumpRecent: () => set((s) => ({ recentTick: s.recentTick + 1 })),
  openDocument: (id) => {
    if(get().brushDraft)cancelBrush();
    // Leaving the current document commits its pending edit rather than dropping it, as
    // ProjectWorkspace.select/newCanvas do on macOS.
    get().commitTransform();
    get().commitGradient();
    dropOpenPanel();
    const state = get().engine!.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null,
      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null }));
  },
  closeDocument: (id) => {
    if(get().brushDraft?.document===id)cancelBrush();
    get().commitTransform();
    // A pending gradient belongs to the document on screen: closing it drops it, closing another applies it.
    if (get().activeId === id) set({ gradientEdit: null }); else get().commitGradient();
    // A panel belongs to the active document; closing a background tab leaves it open.
    if (get().activeId === id) dropOpenPanel();
    get().engine!.closeDocument(id);
    set((s) => {
      const { [id]: _d, ...documents } = s.documents;
      const { [id]: _v, ...viewports } = s.viewports;
      const order = s.order.filter((o) => o !== id);
      const activeId = s.activeId === id ? order[order.length - 1] ?? null : s.activeId;
      const active = activeId ? documents[activeId] : null;
      return { documents, viewports, order, activeId, cropRect: null,
        selectedLayerIds: active?.activeLayerId ? [active.activeLayerId] : [], maskSelected: false, transformEdit: null,
        ...(s.activeId === id ? { selectionDraft: null, outlineMove: null } : {}) };
    });
  },
  moveTab: (id, index) => set((s) => ({ order: reorderedTabs(s.order, id, index) })),
  setActive: (id) => {
    // Clicking the tab already on screen changes nothing, and so must not cancel its panel.
    if (id === get().activeId) return;
    if(get().brushDraft)cancelBrush();
    get().commitTransform();
    get().commitGradient();
    dropOpenPanel();
    const state = get().documents[id];
    set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null });
  },
  refresh: (id) => {
    const target = id ?? get().activeId;
    if (!target) return;
    const state = get().engine!.state(target);
    // `selectedLayerIds` and `maskSelected` belong to the document on screen. A background
    // document finishing a slow import must not rewrite them, or the visible panel loses its
    // selection and Delete/opacity become silent no-ops on ids the active document never had.
    if (target !== get().activeId) { set((s) => ({ documents: { ...s.documents, [target]: state }, renderTick: s.renderTick + 1 })); return; }
    const keep = get().selectedLayerIds.filter((sid) => state.layers.some((l) => l.id === sid));
    const selected = state.activeLayerId && !keep.includes(state.activeLayerId) ? [state.activeLayerId] : keep;
    set((s) => ({ documents: { ...s.documents, [target]: state }, renderTick: s.renderTick + 1, selectedLayerIds: selected }));
  },
  /** Expands every collapsed folder between the root and the active layer, so a layer that was
   * just inserted or reparented is actually on screen. macOS expands the destination on each of
   * these paths (LayerGroups.placeLayer/addGroup/groupSelectedLayers, EditorSession.addBlankLayer). */
  revealActiveLayer: () => {
    const { activeId, collapsed, documents } = get(); if (!activeId) return;
    const state = documents[activeId]; const list = collapsed[activeId] ?? [];
    if (!state || list.length === 0) return;
    const byId = new Map(state.layers.map((l) => [l.id, l]));
    const ancestors = new Set<string>();
    let node = state.activeLayerId ? byId.get(state.activeLayerId) : undefined;
    let steps = 0;
    while (node?.parentId && steps++ < 65) { ancestors.add(node.parentId); node = byId.get(node.parentId); }
    const next = list.filter((x) => !ancestors.has(x));
    if (next.length !== list.length) set({ collapsed: { ...collapsed, [activeId]: next } });
  },
  run: (command) => {
    // Its own commit clears `adjustEdit` before calling this, so a panel's OK is never refused.
    if (get().panelOwnsDocument(true)) return false;
    // A job's result is still to come: the layer it will land on must not change first.
    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
    // A pending transform is closed before any other command records history. macOS refuses
    // these outright while `transformEdit != nil` (canEditLayers); committing is the gentler
    // equivalent and is what every action in actions/layers.ts already did individually.
    //
    // This is also what keeps `cancelTransform` honest: with no command able to interleave
    // between the Alt-drag duplicate and the cancel, the entry `revert` pops is always the
    // duplicate's. Shortcuts stay live during a drag (a captured pointer does not stop
    // keydown), so before this a bare digit could slip a SetLayersOpacity entry in between.
    //
    // `commitTransform` clears `transformEdit` before issuing its own command, so the nested
    // `run` below sees none and this does not recurse.
    if (get().transformEdit) get().commitTransform();
    // Likewise a pending gradient is applied before anything else records history (the Mac refuses
    // them while it is pending: `canEditLayers`). `commitGradient` clears it before its own run.
    if (get().gradientEdit) get().commitGradient();
    // On a large target that sends the gradient to the job worker: this command waits like any other,
    // or it would change the stamp the gradient's result must find (a Deselect, a nudge, a Canvas
    // Size) and lose the gradient, or land before it in the history (final review I-1).
    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
    const { engine, activeId } = get();
    if (!engine || !activeId) return false;
    try {
      engine.execute(activeId, command);
      get().refresh(activeId);
      if (REVEALING_COMMANDS.has(command.type)) get().revealActiveLayer();
      return true;
    } catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
  },
  // A panel owns the document while it is open, as macOS's canEditLayers does; the menu items
  // for these are already disabled, so this stays quiet rather than raising the error banner.
  // Like Photoshop, the first Undo discards a pending gradient; a Redo drops it too (`undo`, `restore`).
  undo: () => {
    if(get().brushDraft){cancelBrush();return;}
    if (get().panelOwnsDocument() || get().working) return;
    if (get().gradientEdit) { get().cancelGradient(); return; }
    const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); }
  },
  redo: () => {
    if (get().panelOwnsDocument() || get().working) return;
    const { activeId, documents } = get();
    const doc = activeId ? documents[activeId] : null;
    // Redo with nothing to redo must not silently drop a pending gradient (fix round 1, M-2).
    if (!doc?.canRedo) return;
    get().cancelGradient();
    const { engine } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); }
  },
  setTool: (tool) => {
    if(get().textEdit||get().effectsEdit){get().panelOwnsDocument(true);return;}
    if (get().brushDraft && tool!==get().tool) cancelBrush();
    if (get().tool === "move" && tool !== "move") get().commitTransform();
    // Switching tools applies a pending gradient, as in Photoshop (`resolveGradient`).
    if (tool !== get().tool) get().commitGradient();
    // Entering the crop tool seeds a rectangle, as macOS does (EditorSession.selectTool): the
    // selection's bounds when there is a selection with something in it, else the canvas (cropSeed).
    // The frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
    // or Cancel clears it nothing is drawn until the user drags a new one. Leaving the tool clears it.
    // Changing tool drops an outline being drawn (`cancelLasso`, EditorSession.swift:284).
    const { activeId, documents, cropRect } = get();
    const doc = activeId ? documents[activeId] : null;
    const seeded = tool === "crop" ? (cropRect ?? (doc ? cropSeed(doc) : null)) : null;
    set({ tool, cropRect: seeded, ...(tool !== get().tool ? { selectionDraft: null, outlineMove: null, shapeDraft: null } : {}) });
    get().invalidate();
  },
  setCropRect: (cropRect) => set({ cropRect }),
  setCropRatio: (cropRatio) => set({ cropRatio }),
  // A sheet and a panel would both answer Enter and Escape (each listens on `window`), so a
  // sheet never opens over a panel.
  openSheet: (sheet) => { if (!get().panelOwnsDocument(true)) set({ sheet }); },
  closeSheet: () => set({ sheet: null }),
  setError: (error) => set({ error }),
  setRendererKind: (rendererKind) => set({ rendererKind }),
  invalidate: () => set((s) => ({ renderTick: s.renderTick + 1 })),
  repaintOverlay: () => set((s) => ({ overlayTick: s.overlayTick + 1 })),
  selectLayers: (ids, primary) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return;
    // `SetActiveLayer` records no history, but every `execute` clears the engine's preview, so a
    // row click under an open destructive panel would wipe what the panel is showing.
    if (get().panelOwnsDocument(true)) return;
    // Nor while a job's result is to come (final review minor 1): `runEditJob` keeps the canvas as it
    // was until the result lands, and the execute would clear the preview it keeps showing; the pending
    // gradient `commitGradient` would apply is dropped while working. The click waits, as a command does.
    if (get().working) { set({ error: BUSY_MESSAGE }); return; }
    const state = get().documents[activeId];
    const valid = ids.filter((id) => state.layers.some((l) => l.id === id));
    const active = primary && valid.includes(primary) ? primary : valid[0] ?? null;
    get().commitTransform();
    // Choosing a layer applies a pending gradient first (`resolveGradient`).
    get().commitGradient();
    // On a large target that sends the gradient to the job worker: the click waits, as `run` does, or
    // its SetActiveLayer would clear the preview the job keeps on screen until the result lands.
    if (get().working) { set({ error: BUSY_MESSAGE }); return; }
    if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
    set({ selectedLayerIds: valid, maskSelected: false });
    get().refresh(activeId);
  },
  // Quiet: the chip handlers that call this have just had `selectLayers` raise the banner. Targeting a
  // mask closes a picker open on a swatch: a mask's palette is black and white (ColorPaletteControls.swift:53-56).
  setMaskSelected: (v) => {
    // Nor while a job's result is to come: the Mac changes no target then (`selectLayerTarget`,
    // LayerMask.swift:222-228 at v1.4.5, `guard !isProjectBusy`). Quiet too: the chip's own
    // `selectLayers` has just raised the banner.
    if (get().panelOwnsDocument() || get().working) return;
    // Changing the target applies a pending gradient first.
    if (v !== get().maskSelected) get().commitGradient();
    set({ maskSelected: v });
    if (get().colorPicker?.target.kind === "palette" && get().maskTargeted()) get().closeColorPicker(false);
  },
  toggleCollapsed: (id) => {
    const { activeId, collapsed } = get(); if (!activeId) return;
    const list = collapsed[activeId] ?? [];
    const collapsing = !list.includes(id);
    const next = collapsing ? [...list, id] : list.filter((x) => x !== id);
    // Collapsing a folder whose descendant is active selects the folder itself, as macOS does.
    let insideFolder = false;
    if (collapsing) {
      const state = get().documents[activeId];
      const byId = new Map(state.layers.map((l) => [l.id, l]));
      let node = state.activeLayerId ? byId.get(state.activeLayerId) : undefined;
      let steps = 0;
      while (node?.parentId && steps++ < 65) { if (node.parentId === id) { insideFolder = true; break; } node = byId.get(node.parentId); }
    }
    // That selection is refused while a panel is open, and hiding the active layer's row without
    // it would leave nothing selected on screen, so the collapse is refused with it.
    if (insideFolder && get().panelOwnsDocument(true)) return;
    set({ collapsed: { ...collapsed, [activeId]: next } });
    if (insideFolder) get().selectLayers([id], id);
  },
  beginTransform: ({ persistent, duplicate }) => {
    // A panel owns the document while it is open, as macOS's canTransform (which gates on
    // canEditLayers) does. `selection.ts`'s `canTransform` has no view of `adjustEdit` -- it
    // takes only `DocumentState` plus the selection, shared with UI hit-testing that has no
    // reason to know about panels -- so this stays here rather than widening that signature.
    if (get().panelOwnsDocument()) return false;
    // A job's result is still to come: an Alt-drag duplicate would push a DuplicateLayer straight
    // through the engine (below), bypassing `run`'s own `working` gate, and a plain drag's commit
    // would be refused by `runEditJob`/`run` anyway once released.
    if (get().working) return false;
    const { engine, activeId, selectedLayerIds, maskSelected } = get(); if (!engine || !activeId) return false;
    const state = get().documents[activeId];
    if (!canTransform(state, selectedLayerIds, maskSelected) || get().transformEdit) return false;
    const source = activeLayer(state);
    if (state.selection && !state.selection.empty && selectedLayerIds.length === 1 && !maskSelected && source?.hasPixels && !source.isGroup && !source.adjustment) {
      try {
        engine.beginFloating(activeId, source.id, !!duplicate);
        get().refresh(activeId);
        const layer = activeLayer(get().documents[activeId])!;
        const t = layer.transform;
        set({ transformEdit: { kind: "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: false, duplicateEntry: null, floating: true } });
        return true;
      } catch (e) { set({ error: e instanceof Error ? e.message : String(e) }); return false; }
    }
    if (transformsAsGroup(state, selectedLayerIds)) {
      const box = groupBox(state, selectedLayerIds)!;
      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false, duplicateEntry: null } });
      return true;
    }
    let layer = activeLayer(state);
    if (!layer) return false;
    // An unlinked mask alone has no pixel layer of its own to duplicate: ignore `duplicate`
    // (computed before any duplication, from the layer this transform is actually about).
    const maskAlone = maskSelected && layer.hasMask && !layer.maskLinked;
    const willDuplicate = !!duplicate && !maskAlone;
    let duplicateEntry: number | null = null;
    if (willDuplicate) {
      // Not `run`: the duplicate has to be observed here to seed the edit. Its failures
      // ("too many layers", "folders are not duplicated this way") still belong in the error
      // banner rather than thrown out of a pointerdown handler.
      try { engine.execute(activeId, { type: "DuplicateLayer", id: layer.id }); }
      catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
      get().refresh(activeId);
      // The entry the copy pushed, so a later cancel can prove the entry it is about to drop is this one.
      duplicateEntry = get().documents[activeId].undoEntryId;
      const copy = activeLayer(get().documents[activeId]);
      if (!copy) return false;
      layer = copy;
      set({ selectedLayerIds: [layer.id] });
    }
    const t = maskAlone ? layer.maskPlacement ?? layer.transform : layer.transform;
    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: willDuplicate, duplicateEntry } });
    return true;
  },
  previewTransform: (draft, corners) => { const e = get().transformEdit; if (!e || !isValidTransform(draft)) return; const c=corners===undefined?e.corners:corners;set({ transformEdit: { ...e, draft, corners:c } });
    const {engine,activeId,documents}=get();if(engine&&activeId&&e.kind==="layer"&&!e.floating&&documents[activeId].layers.find(l=>l.id===e.id)?.shape){engine.setPreview(activeId,c?null:{preview:"Shape",layer:e.id,draft});}
    get().invalidate(); },
  beginDistort: () => { const e = get().transformEdit; if (!e || e.corners || e.kind === "mask") return; set({ transformEdit: { ...e, corners: cornersToTuples(cornersOf(e.draft)), persistent: true } }); },
  commitTransform: () => {
    const e = get().transformEdit; const { engine, activeId } = get(); if (!e || !engine || !activeId) return;
    if(get().documents[activeId]?.layers.find(l=>l.id===e.id)?.shape)engine.setPreview(activeId,null);
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    const draft = roundedTransform(e.draft);
    if (e.floating) {
      try { engine.commitFloating(activeId, e.corners ? e.draft : draft, e.corners); }
      catch (error) { engine.cancelFloating(activeId); set({ error: error instanceof Error ? error.message : String(error) }); }
      get().refresh(activeId);
      return;
    }
    const unchanged = e.corners
      ? JSON.stringify(e.corners) === JSON.stringify(cornersToTuples(cornersOf(e.original)))
      : JSON.stringify(draft) === JSON.stringify(roundedTransform(e.original));
    // A no-op edit (including a duplicate that was never moved) commits nothing; the duplicate stays in place.
    if (unchanged) { get().invalidate(); return; }
    if (e.kind === "mask") get().run({ type: "SetMaskPlacement", id: e.id, placement: draft });
    else if (e.kind === "group") get().run(e.corners ? { type: "DistortLayers", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners } : { type: "TransformLayers", ids: e.ids, box: e.box, draft });
    else get().run(e.corners ? { type: "DistortLayer", id: e.id, transform: e.draft, corners: e.corners } : { type: "SetLayerTransform", id: e.id, transform: draft });
  },
  cancelTransform: () => {
    const e = get().transformEdit; const { engine, activeId } = get(); if (!e) return;
    if(engine&&activeId&&get().documents[activeId]?.layers.find(l=>l.id===e.id)?.shape)engine.setPreview(activeId,null);
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    if (e.floating && engine && activeId) { engine.cancelFloating(activeId); get().refresh(activeId); return; }
    // The Alt-drag copy is dropped with `revert`, which removes the DuplicateLayer entry
    // outright. A plain `undo` here would leave it on the redo stack, and Ctrl+Shift+Z would
    // bring the cancelled copy back. macOS closes the transaction with nothing recorded.
    //
    // Only when the entry on top is provably still the duplicate's: its id must be the one the
    // copy pushed. `run` commits any pending edit before it records, so nothing should be able to
    // interleave, but reverting the wrong entry would silently discard a real edit and strand the
    // copy, so this refuses rather than guesses.
    if (e.duplicated && engine && activeId) {
      const top = engine.state(activeId).undoEntryId;
      if (e.duplicateEntry !== null && top === e.duplicateEntry) { engine.revert(activeId); get().refresh(activeId); }
    }
    get().invalidate();
  },
  setSnapGuides: (g) => set({ snapGuides: g }),
  setLocksTransformRatio: (v) => set({ locksTransformRatio: v }),
  toggleGuides: () => set((s) => {
    const showGuides = !s.showGuides;
    try { localStorage.setItem(GUIDES_KEY, String(showGuides)); } catch { /* ignore */ }
    return { showGuides };
  }),
  setBlendPreview: (m) => set({ blendPreview: m }),
  previewEdit: () => {
    const fx=get().effectsEdit;if(fx?.preview)return{kind:"effects",id:fx.layer,effects:fx.effects};
    const a = get().adjustEdit;
    if (a && a.target === "adjustmentLayer" && a.preview && a.adjustment) return { kind: "adjustment", id: a.layerId, adjustment: a.adjustment };
    const e = get().transformEdit; if (!e) return null;
    if (e.kind === "group") return { kind: "group", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners };
    if (e.kind === "mask") return { kind: "mask", id: e.id, draft: e.draft };
    return { kind: "layer", id: e.id, draft: e.draft, corners: e.corners };
  },
  panelOwnsDocument: (refuse = false) => {
    if(get().contentFill||get().brushDraft||get().textEdit||get().effectsEdit){if(refuse)set({error:"Apply or cancel the pending edit first"});return true;}
    if (!get().adjustEdit) return false;
    if (refuse) set({ error: "Apply or cancel the open adjustment first" });
    return true;
  },
  canAdjust: () => {
    const { activeId, documents, selectedLayerIds, maskSelected } = get();
    // A job's result is still to come: no panel may open onto a layer that may change underneath it.
    if (get().working) return false;
    if (!activeId || get().panelOwnsDocument()) return false;
    const state = documents[activeId];
    const layer = activeLayer(state);
    // An empty selection refuses every edit (`canAdjustColors`: `selection?.isEmpty != true`).
    return !!layer && !layer.isGroup && layer.hasPixels && !maskSelected && selectedLayerIds.length === 1 && visibleIds(state).has(layer.id)
      && state.selection?.empty !== true;
  },
  beginAdjust: ({ kind, layerId, target }) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return false;
    // Shortcuts (Ctrl+L / Ctrl+M / Ctrl+U) call this directly, without going through `canAdjust`
    // first: a job's result still to come must refuse here too, or its histogram queues behind the
    // edit and its `clear_preview` (install_job) wipes the new panel's own preview.
    if (get().working) return false;
    get().commitTransform();
    // A pending gradient is applied first, as the Mac's canPaint requires `gradientEdit == nil`
    // (EditorSession.swift:608); on a large layer that sends it to the job worker, so `working` is
    // re-checked here too (fix round 1, I-1).
    get().commitGradient();
    if (get().working) return false;
    const state = get().documents[activeId];
    const id = layerId ?? state.activeLayerId;
    const layer = id ? state.layers.find((l) => l.id === id) : null;
    if (!layer || get().panelOwnsDocument()) return false;
    const editing = target === "adjustmentLayer";
    if (editing ? !layer.adjustment : !get().canAdjust()) return false;
    const requestId = ++histogramRequestId;
    const filter = isFilterKind(kind as string);
    const adjustment = filter ? null : (editing ? layer.adjustment! : defaultAdjustment(kind as AdjustmentKind));
    // A destructive Gradient Map starts from the image's foreground and background (Filters.swift:490).
    if (adjustment?.gradientMapSettings && !editing) {
      const { foreground, background } = get().palette;
      adjustment.gradientMapSettings = { ...adjustment.gradientMapSettings, shadows: { ...foreground }, highlights: { ...background } };
    }
    // Each destructive Grain gets a pattern of its own, as the Mac's FilterEdit draws a random
    // seed and as Add Noise already does here; an adjustment layer keeps the seed it was made with.
    if (adjustment?.grainSettings && !editing) adjustment.grainSettings.seed = Math.floor(Math.random() * 0xffffffff);
    const edit: AdjustEdit = {
      kind, target: editing ? "adjustmentLayer" : "layer", layerId: layer.id,
      adjustment, params: filter ? defaultFilterParams(kind as FilterKind) : null,
      original: editing ? layer.adjustment! : null, preview: true, sampleMode: null,
      // Levels and Curves draw a histogram of what they are about to change: a large layer's comes from
      // the job worker once the panel is open, which it opens without waiting for.
      histogram: kind === "Levels" || kind === "Curves" ? (!editing && get().usesJob(layer.id) ? null : engine.histogram(activeId, layer.id)) : null,
    };
    // The crop tool's rectangle goes, as macOS's beginFilter calls cancelCrop first: a pending
    // crop would otherwise answer the same Enter and Escape as the panel.
    set({ adjustEdit: edit, cropRect: null });
    get().applyAdjustPreview();
    if ((kind === "Levels" || kind === "Curves") && !edit.histogram) {
      const doc = activeId, jobs = get().jobs!;
      let prepared: JobInputCopy | Promise<JobInputCopy>;
      let preparation: (() => void) | null = null;
      const releasePreparation = () => { const release = preparation; preparation = null; release?.(); };
      // The copy out can fail (a RangeError allocating a 400 MB slice): the panel then closes and says
      // why, rather than wait on "Reading the histogram..." forever and throw out of the key handler
      // (final review minor 5).
      try { preparation = jobs.prepareInteractive(); prepared = engine.jobInputAsync(doc, layer.id); }
      catch (e) {
        releasePreparation();
        dropOpenPanel();
        set({ error: String(e instanceof Error ? e.message : e) });
        get().refresh(doc);
        return false;
      }
      const current = () => {
        const open = get().adjustEdit;
        return requestId === histogramRequestId && get().activeId === doc && open?.layerId === layer.id && open.kind === kind;
      };
      const failedPreparation = (e: unknown) => {
        releasePreparation();
        if (!current()) return;
        dropOpenPanel(); set({ error: String(e instanceof Error ? e.message : e) }); get().refresh(doc);
      };
      const submit = (copy: JobInputCopy) => {
        try {
          if (!current()) { releaseJobResult({ header: null, pixels: copy.pixels, mask: copy.mask, display: copy.points }); return; }
          void jobs.run(`histogram:${doc}`, { kind: "histogram", input: copy.input, pixels: copy.pixels, mask: copy.mask, points: copy.points }).then((result) => {
            const open = get().adjustEdit;
            // A close/reopen of the same kind on the same layer is a new request.
            if (!result?.header || !current() || !open) return;
            set({ adjustEdit: { ...open, histogram: JSON.parse(result.header) as number[][] } });
          }).catch((e) => { if (current()) set({ error: String(e instanceof Error ? e.message : e) }); });
        } finally { releasePreparation(); }
      };
      if (prepared instanceof Promise) void prepared.then(submit).catch(failedPreparation);
      else submit(prepared);
    }
    return true;
  },
  updateAdjust: (patch) => {
    const edit = get().adjustEdit; if (!edit) return;
    set({ adjustEdit: { ...edit, ...patch } });
    get().applyAdjustPreview(true);
  },
  setAdjustPreview: (preview) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, preview } }); get().applyAdjustPreview(); },
  setAdjustSample: (sampleMode) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, sampleMode } }); },
  /** A click on the canvas while an eyedropper is armed. Levels calibrates from the layer's own
   * pixels (as macOS's sampleLevels does); the Hue/Saturation tools read the visible composite. */
  sampleAt: (at) => {
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId || !adjustEdit?.sampleMode) return;
    const mode = adjustEdit.sampleMode;
    if(mode==="CameraPointColor"&&adjustEdit.params?.filter==="CameraRaw"){
      const rgb=engine.cameraRawSampleColor(activeId,adjustEdit.layerId,at);if(!rgb)return;const settings=adjustEdit.params.settings;if(settings.mixer.points.length>=8)return;
      const [r,g,b]=rgb,max=Math.max(r,g,b),min=Math.min(r,g,b),l=(max+min)/2,d=max-min;
      const hue=hueOf(rgb)??0,saturation=max===0?0:d/max;
      get().updateAdjust({params:{filter:"CameraRaw",settings:{...settings,mixer:{...settings.mixer,points:[...settings.mixer.points,{hue,saturation,luminance:l,hueShift:0,saturationShift:0,luminanceShift:0,hueRange:30,saturationRange:.4,luminanceRange:.4}]}}}});get().setAdjustSample(null);return;
    }
    if(mode==="CameraDefringe"&&adjustEdit.params?.filter==="CameraRaw"){
      const rgb=engine.cameraRawSampleColor(activeId,adjustEdit.layerId,at,false);if(!rgb)return;const hue=hueOf(rgb)??0;
      const optics={...adjustEdit.params.settings.optics};
      if(Math.abs(hue-290)<Math.abs(hue-90)){optics.purpleHueLow=Math.max(0,hue-25);optics.purpleHueHigh=Math.min(360,hue+25);if(optics.purpleAmount===0)optics.purpleAmount=50;}
      else{optics.greenHueLow=Math.max(0,hue-25);optics.greenHueHigh=Math.min(360,hue+25);if(optics.greenAmount===0)optics.greenAmount=50;}
      get().updateAdjust({params:{filter:"CameraRaw",settings:{...adjustEdit.params.settings,optics}}});return;
    }
    if (mode === "CameraWhiteBalance" && adjustEdit.params?.filter === "CameraRaw") {
      const balanced=engine.cameraRawWhiteBalance(activeId,adjustEdit.layerId,at);
      if(balanced){const [temperature,tint]=balanced;get().updateAdjust({params:{filter:"CameraRaw",settings:{...adjustEdit.params.settings,whiteBalance:"Custom",temperature:Math.max(-100,Math.min(100,temperature)),tint:Math.max(-100,Math.min(100,tint))}}});}
      return;
    }
    if (mode === "Black" || mode === "Gray" || mode === "White") {
      const levels = engine.levelsSampling(activeId, adjustEdit.layerId, adjustEdit.adjustment!.levels, at, mode);
      get().updateAdjust({ adjustment: { ...adjustEdit.adjustment!, levels } });
      return;
    }
    // `sampleColor` reads the stored document, never the open preview -- sampling the panel's own
    // live edit would chase whatever the sliders just did (the same reason `histogram` never
    // reads the preview).
    const rgb = engine.sampleColor(activeId, at);
    const hue = rgb ? hueOf(rgb) : null;
    if (hue === null) return;
    const settings = adjustEdit.adjustment!.hsvSettings ?? defaultHsv();
    if (settings.range === "Master" || settings.colorize) return;
    const band = settings.bands[settings.range] ?? DEFAULT_BANDS[settings.range];
    const next = mode === "replace" ? centeredOn(band, hue) : mode === "add" ? includeHue(band, hue) : excludeHue(band, hue);
    get().updateAdjust({ adjustment: { ...adjustEdit.adjustment!, hsvSettings: { ...settings, bands: { ...settings.bands, [settings.range]: next } } } });
  },
  /** Replaces the panel's Levels settings with the engine's auto-stretch for `mode`, read from
   * the same histogram the panel already opened with (never the live preview -- see the
   * histogram note on `beginAdjust`). Those bins are passed in rather than recomputed: for an
   * adjustment layer they come from a full composite of everything beneath it. */
  autoLevels: (mode) => {
    const { engine, adjustEdit } = get(); if (!engine || !adjustEdit?.adjustment || !adjustEdit.histogram) return;
    const levels = engine.autoLevels(adjustEdit.histogram, mode);
    get().updateAdjust({ adjustment: { ...adjustEdit.adjustment, levels } });
  },
  /** Pushes the panel's settings to the engine: a pixel preview for a destructive edit, or a
   * plan-level preview (through `previewEdit`) when an adjustment layer is being edited. */
  applyAdjustPreview: (dragging = false) => {
    cancelSettle();
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId) return;
    if (!adjustEdit || adjustEdit.target === "adjustmentLayer") { if (!adjustEdit) engine.setPreview(activeId, null); get().invalidate(); return; }
    const request = previewRequestFor(adjustEdit, (a) => engine.adjustmentIsIdentity(a), dragging);
    engine.setPreview(activeId, request);
    get().refresh(activeId);
    // A slider tick previews a colour adjustment from a small copy; once ticks stop, the
    // full-quality preview replaces it. It is built from `adjustEdit` as it is when the timer
    // fires, and never lands on another document or a closed panel: every path that closes the
    // panel or leaves the document cancels the timer (dropOpenPanel), and this checks again.
    if (request?.preview === "DragAdjustment") {
      const doc = activeId;
      settleTimer = setTimeout(() => {
        settleTimer = null;
        const s = get();
        if (s.activeId === doc && s.adjustEdit) s.applyAdjustPreview();
      }, SETTLE_MS);
    }
  },
  previewSettling: () => settleTimer !== null,
  commitAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    // A previous job's result is still to come (only possible for an adjustment-layer edit or a
    // small layer's OK, since a large layer's own OK is what sets `working`): checked before the
    // panel closes, so the user's settings and preview stay up rather than being lost to a banner.
    if (get().working) { set({ error: BUSY_MESSAGE }); return; }
    const identity = isAdjustIdentity(edit, (a) => engine.adjustmentIsIdentity(a));
    const command: Command = edit.target === "adjustmentLayer" ? { type: "SetAdjustment", id: edit.layerId, adjustment: edit.adjustment! }
      : edit.params ? { type: "ApplyFilter", id: edit.layerId, params: effectiveFilterParams(edit)! }
      : { type: "ApplyAdjustment", id: edit.layerId, adjustment: edit.adjustment! };
    // A large layer is edited by the job worker: the panel closes but the canvas keeps its preview
    // until the result is put back (runEditJob clears it then); the document is busy meanwhile.
    if (!identity && edit.target === "layer" && get().usesJob(edit.layerId)) {
      cancelSettle();
      set({ adjustEdit: null, ...(get().colorPicker?.target.kind === "gradientMap" ? { colorPicker: null } : {}) });
      void get().runEditJob(command, edit.layerId);
      return;
    }
    dropOpenPanel();
    if (identity) { get().refresh(activeId); get().invalidate(); return; }
    // A refused command (settings the engine will not accept) leaves the panel open with the
    // user's settings and its preview, under the banner, rather than discarding the edit.
    if (!get().run(command)) { set({ adjustEdit: edit }); get().applyAdjustPreview(); }
  },
  cancelAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    dropOpenPanel();
    get().refresh(activeId);
    get().invalidate();
  },
  setSelectionOptions: (patch) => {
    // Changing the Marquee's or the Lasso's kind drops an outline being drawn (LassoControls.swift:10-37).
    const kindChanged = (patch.marquee !== undefined && patch.marquee !== get().selectionOptions.marquee)
      || (patch.lasso !== undefined && patch.lasso !== get().selectionOptions.lasso);
    set((s) => ({ selectionOptions: { ...s.selectionOptions, ...patch }, ...(kindChanged ? { selectionDraft: null } : {}) }));
    if (kindChanged) get().repaintOverlay();
  },
  setSelectionDraft: (selectionDraft) => { set({ selectionDraft }); get().repaintOverlay(); },
  finishSelectionDraft: () => {
    const draft = get().selectionDraft; if (!draft) return;
    set({ selectionDraft: null });
    get().run(draft.finish(get().selectionOptions.antialiased));
    get().invalidate();
  },
  setOutlineMove: (outlineMove) => { set({ outlineMove }); get().repaintOverlay(); },
  setHeldSelectionMode: (heldSelectionMode) => { if (heldSelectionMode !== get().heldSelectionMode) set({ heldSelectionMode }); },
  modifySelection: (operation, amount) => {
    if (!Number.isInteger(amount) || amount < 1 || amount > SELECTION_AMOUNT_MAX[operation] || !get().hasSelection()) return false;
    const key = operation === "Expand" ? "expand" : operation === "Contract" ? "contract" : "feather";
    set((s) => ({ selectionOptions: { ...s.selectionOptions, [key]: amount } }));
    const command: Command = operation === "Expand" ? { type: "ExpandSelection", amount }
      : operation === "Contract" ? { type: "ContractSelection", amount } : { type: "FeatherSelection", amount };
    return get().run(command);
  },
  cycleToolMode: () => {
    const { tool, selectionOptions: o } = get();
    if (tool === "gradient") get().setGradientOptions({ shape: get().gradientOptions.shape === "Linear" ? "Radial" : "Linear" });
    if (tool === "shape") get().setShapeOptions({ kind: nextShapeKind(get().shapeOptions.kind) });
    if (tool === "marquee") get().setSelectionOptions({ marquee: o.marquee === "Rectangle" ? "Ellipse" : "Rectangle" });
    else if (tool === "lasso") get().setSelectionOptions({ lasso: o.lasso === "Freehand" ? "Polygonal" : "Freehand" });
  },
  hasSelection: () => {
    const { activeId, documents } = get();
    const selection = activeId ? documents[activeId]?.selection : null;
    return !!selection && !selection.empty;
  },
}));
