import { create } from "zustand";
import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, LevelsAuto, PreviewEdit, SelectionMode, WandSettings } from "../engine/types";
import { DEFAULT_WAND } from "../engine/types";
import type { EngineClient, JobInputCopy } from "../engine/client";
import type { JobClient } from "../engine/jobs";
import { cropSeed } from "../tools/crop-tool";
import type { LassoKind, MarqueeKind, SelectionDraft } from "../tools/selection-draft";
import type { ShellBridge } from "../shell/bridge";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";
import { cornersOf, cornersToTuples, isValidTransform, roundedTransform } from "../tools/transform-geometry";
import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } from "./selection";
import type { AdjustEdit, SampleMode } from "./adjust-edit";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";
import { DEFAULT_BANDS, centeredOn, defaultHsv, excludeHue, hueOf, includeHue } from "../tools/hue-band";

export type Tool = "move" | "hand" | "zoom" | "crop" | "marquee" | "lasso" | "wand";
export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
/** Select > Expand / Contract / Feather ask for an amount (`SelectionAmountSheet`, LassoControls.swift:180-236). */
export type SelectionAmountOperation = "Expand" | "Contract" | "Feather";
export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" }
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

/** A layer with more pixels than this is edited, and its histogram read, by the job worker rather than
 * on the UI thread (ruling OQ5): at 4 MP a Levels commit took about 0.35 s here. Below it a job's two
 * copies and the worker's round trip cost more than they save. */
export const JOB_PIXELS = 4_000_000;
/** Said when a command arrives while a job's result is still to come. */
export const BUSY_MESSAGE = "Wait for the current edit to finish.";

export interface EditorStore {
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
  updateAdjust(patch: { adjustment?: AdjustEdit["adjustment"]; params?: AdjustEdit["params"] }): void;
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
  "AddBlankLayer", "AddGroup", "GroupLayers", "PlaceLayer", "DuplicateLayer", "DuplicateLayerTo", "DuplicateLayerTransformed", "MergeLayers", "DeleteLayers", "DeleteLayer",
]);

/** How long after the last slider tick a colour adjustment's quick drag preview is replaced by
 * the full-quality one (engine/src/preview.rs explains the two sizes). */
export const SETTLE_MS = 150;
let settleTimer: ReturnType<typeof setTimeout> | null = null;
function cancelSettle(): void {
  if (settleTimer !== null) { clearTimeout(settleTimer); settleTimer = null; }
}

/** Closes an open panel without applying it and clears its engine-side preview. `adjustEdit`,
 * like `transformEdit`, names no document of its own: it always belongs to whatever is
 * `activeId` at the time, so that is the document whose preview is cleared. Callers leaving or
 * closing the active document use this; closing some other, background tab must not. */
function dropOpenPanel(): void {
  cancelSettle();
  const { adjustEdit, engine, activeId } = useEditor.getState();
  if (!adjustEdit) return;
  useEditor.setState({ adjustEdit: null });
  if (activeId) engine!.setPreview(activeId, null);
}

const GUIDES_KEY = "compositor.showGuides";
function loadShowGuides(): boolean {
  try { return localStorage.getItem(GUIDES_KEY) !== "false"; } catch { return true; }
}

export const useEditor = create<EditorStore>((set, get) => ({
  engine: null, jobs: null, jobPixels: JOB_PIXELS, working: false, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, overlayTick: 0, recentTick: 0,
  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, showGuides: loadShowGuides(),
  blendPreview: null,
  adjustEdit: null,
  selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, heldSelectionMode: null,
  setEngine: (engine) => set({ engine }),
  setJobs: (jobs) => set({ jobs }),
  usesJob: (layerId) => {
    const { jobs, activeId, documents, jobPixels } = get();
    const layer = activeId ? documents[activeId]?.layers.find((l) => l.id === layerId) : undefined;
    return !!jobs && !!layer && layer.pixelsWidth * layer.pixelsHeight > jobPixels;
  },
  runEditJob: async (command, layerId) => {
    const { engine, jobs, activeId } = get();
    if (!engine || !jobs || !activeId) return false;
    if (get().working) { set({ error: BUSY_MESSAGE }); return false; }
    const doc = activeId;
    let copy: JobInputCopy;
    try { copy = engine.jobInput(doc, layerId); } catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
    set({ working: true });
    let installed = false;
    try {
      const result = await jobs.run(`edit:${doc}`, { kind: "edit", input: copy.input, pixels: copy.pixels, mask: copy.mask, points: copy.points, command: JSON.stringify(command) });
      // Closed meanwhile: nothing to put back.
      if (!result || !get().documents[doc]) return false;
      engine.installJob(doc, layerId, copy.input, result.header!, result.pixels, result.mask);
      installed = true;
      return true;
    } catch (e) {
      set({ error: String(e instanceof Error ? e.message : e) });
      return false;
    } finally {
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
    // Leaving the current document commits its pending edit rather than dropping it, as
    // ProjectWorkspace.select/newCanvas do on macOS.
    get().commitTransform();
    dropOpenPanel();
    const state = get().engine!.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null,
      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null, selectionDraft: null, outlineMove: null }));
  },
  closeDocument: (id) => {
    get().commitTransform();
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
  setActive: (id) => {
    // Clicking the tab already on screen changes nothing, and so must not cancel its panel.
    if (id === get().activeId) return;
    get().commitTransform();
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
  undo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
  redo: () => { if (get().panelOwnsDocument() || get().working) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
  setTool: (tool) => {
    if (get().tool === "move" && tool !== "move") get().commitTransform();
    // Entering the crop tool seeds a rectangle, as macOS does (EditorSession.selectTool): the
    // selection's bounds when there is a selection with something in it, else the canvas (cropSeed).
    // The frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
    // or Cancel clears it nothing is drawn until the user drags a new one. Leaving the tool clears it.
    // Changing tool drops an outline being drawn (`cancelLasso`, EditorSession.swift:284).
    const { activeId, documents, cropRect } = get();
    const doc = activeId ? documents[activeId] : null;
    const seeded = tool === "crop" ? (cropRect ?? (doc ? cropSeed(doc) : null)) : null;
    set({ tool, cropRect: seeded, ...(tool !== get().tool ? { selectionDraft: null, outlineMove: null } : {}) });
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
    const state = get().documents[activeId];
    const valid = ids.filter((id) => state.layers.some((l) => l.id === id));
    const active = primary && valid.includes(primary) ? primary : valid[0] ?? null;
    get().commitTransform();
    if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
    set({ selectedLayerIds: valid, maskSelected: false });
    get().refresh(activeId);
  },
  // Quiet: the chip handlers that call this have just had `selectLayers` raise the banner.
  setMaskSelected: (v) => { if (!get().panelOwnsDocument()) set({ maskSelected: v }); },
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
  previewTransform: (draft, corners) => { const e = get().transformEdit; if (!e || !isValidTransform(draft)) return; set({ transformEdit: { ...e, draft, corners: corners === undefined ? e.corners : corners } }); get().invalidate(); },
  beginDistort: () => { const e = get().transformEdit; if (!e || e.corners || e.kind === "mask") return; set({ transformEdit: { ...e, corners: cornersToTuples(cornersOf(e.draft)), persistent: true } }); },
  commitTransform: () => {
    const e = get().transformEdit; const { engine, activeId } = get(); if (!e || !engine || !activeId) return;
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    const draft = roundedTransform(e.draft);
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
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
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
  toggleGuides: () => set((s) => {
    const showGuides = !s.showGuides;
    try { localStorage.setItem(GUIDES_KEY, String(showGuides)); } catch { /* ignore */ }
    return { showGuides };
  }),
  setBlendPreview: (m) => set({ blendPreview: m }),
  previewEdit: () => {
    const a = get().adjustEdit;
    if (a && a.target === "adjustmentLayer" && a.preview && a.adjustment) return { kind: "adjustment", id: a.layerId, adjustment: a.adjustment };
    const e = get().transformEdit; if (!e) return null;
    if (e.kind === "group") return { kind: "group", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners };
    if (e.kind === "mask") return { kind: "mask", id: e.id, draft: e.draft };
    return { kind: "layer", id: e.id, draft: e.draft, corners: e.corners };
  },
  panelOwnsDocument: (refuse = false) => {
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
    const state = get().documents[activeId];
    const id = layerId ?? state.activeLayerId;
    const layer = id ? state.layers.find((l) => l.id === id) : null;
    if (!layer || get().panelOwnsDocument()) return false;
    const editing = target === "adjustmentLayer";
    if (editing ? !layer.adjustment : !get().canAdjust()) return false;
    const filter = isFilterKind(kind as string);
    const adjustment = filter ? null : (editing ? layer.adjustment! : defaultAdjustment(kind as AdjustmentKind));
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
      const copy = engine.jobInput(doc, layer.id);
      void jobs.run(`histogram:${doc}`, { kind: "histogram", input: copy.input, pixels: copy.pixels, mask: copy.mask, points: copy.points }).then((result) => {
        const open = get().adjustEdit;
        // Only into the panel it was read for.
        if (!result?.header || get().activeId !== doc || open?.layerId !== layer.id || open.kind !== kind) return;
        set({ adjustEdit: { ...open, histogram: JSON.parse(result.header) as number[][] } });
      }).catch((e) => set({ error: String(e instanceof Error ? e.message : e) }));
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
      : edit.params ? { type: "ApplyFilter", id: edit.layerId, params: edit.params }
      : { type: "ApplyAdjustment", id: edit.layerId, adjustment: edit.adjustment! };
    // A large layer is edited by the job worker: the panel closes but the canvas keeps its preview
    // until the result is put back (runEditJob clears it then); the document is busy meanwhile.
    if (!identity && edit.target === "layer" && get().usesJob(edit.layerId)) {
      cancelSettle();
      set({ adjustEdit: null });
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
    if (tool === "marquee") get().setSelectionOptions({ marquee: o.marquee === "Rectangle" ? "Ellipse" : "Rectangle" });
    else if (tool === "lasso") get().setSelectionOptions({ lasso: o.lasso === "Freehand" ? "Polygonal" : "Freehand" });
  },
  hasSelection: () => {
    const { activeId, documents } = get();
    const selection = activeId ? documents[activeId]?.selection : null;
    return !!selection && !selection.empty;
  },
}));
