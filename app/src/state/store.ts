import { create } from "zustand";
import type { AdjustmentKind, BlendMode, Command, Corners, DocumentState, FilterKind, LayerTransform, PreviewEdit } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { ShellBridge } from "../shell/bridge";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";
import { cornersOf, cornersToTuples, isValidTransform, roundedTransform } from "../tools/transform-geometry";
import { activeLayer, canTransform, groupBox, transformsAsGroup, visibleIds } from "./selection";
import type { AdjustEdit, SampleMode } from "./adjust-edit";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, isFilterKind, previewRequestFor } from "./adjust-edit";

export type Tool = "move" | "hand" | "zoom" | "crop";
export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" };

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
  /** For a duplicated edit, the engine's undo depth immediately before the DuplicateLayer that
   * created the copy. `cancelTransform` reverts only when the depth is still exactly one past
   * this, i.e. the entry on top of the stack is the duplicate's and nothing slipped in behind
   * it. Null when the edit duplicated nothing. */
  undoDepthBefore: number | null;
}

export interface EditorStore {
  engine: EngineClient | null;
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
  recentTick: number;
  selectedLayerIds: string[];
  maskSelected: boolean;
  collapsed: Record<string, string[]>;
  transformEdit: TransformEdit | null;
  snapGuides: { xs: number[]; ys: number[] };
  blendPreview: BlendMode | null;
  adjustEdit: AdjustEdit | null;
  setEngine(engine: EngineClient): void;
  setBridge(bridge: ShellBridge): void;
  setBusy(busy: boolean): void;
  bumpRecent(): void;
  openDocument(id: string): void;
  closeDocument(id: string): void;
  setActive(id: string): void;
  refresh(id?: string): void;
  revealActiveLayer(): void;
  run(command: Command): void;
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
  selectLayers(ids: string[], primary: string | null): void;
  setMaskSelected(v: boolean): void;
  toggleCollapsed(id: string): void;
  beginTransform(opts: { persistent: boolean; duplicate?: boolean }): boolean;
  previewTransform(draft: LayerTransform, corners?: Corners | null): void;
  beginDistort(): void;
  commitTransform(): void;
  cancelTransform(): void;
  setSnapGuides(g: { xs: number[]; ys: number[] }): void;
  setBlendPreview(m: BlendMode | null): void;
  previewEdit(): PreviewEdit | null;
  canAdjust(): boolean;
  beginAdjust(opts: { kind: AdjustmentKind | FilterKind; layerId?: string; target?: "layer" | "adjustmentLayer" }): boolean;
  updateAdjust(patch: { adjustment?: AdjustEdit["adjustment"]; params?: AdjustEdit["params"] }): void;
  setAdjustPreview(on: boolean): void;
  setAdjustSample(mode: SampleMode | null): void;
  /** Pushes the open panel's current settings to the engine as a preview. Not part of the
   * Task 12 brief's public action list, but needed by beginAdjust/updateAdjust/setAdjustPreview,
   * which all share it rather than duplicating the branch between a pixel-layer preview (through
   * `engine.setPreview`) and an adjustment-layer preview (through the render plan). */
  applyAdjustPreview(): void;
  commitAdjust(): void;
  cancelAdjust(): void;
}

/** Commands that insert a layer or move one into a folder, and so make it active somewhere the
 * panel may not be showing. Each is followed by `revealActiveLayer`. */
const REVEALING_COMMANDS: ReadonlySet<Command["type"]> = new Set<Command["type"]>([
  "AddBlankLayer", "AddGroup", "GroupLayers", "PlaceLayer", "DuplicateLayer", "DuplicateLayerTo", "DuplicateLayerTransformed", "MergeLayers", "DeleteLayers", "DeleteLayer",
]);

export const useEditor = create<EditorStore>((set, get) => ({
  engine: null, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, recentTick: 0,
  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, blendPreview: null,
  adjustEdit: null,
  setEngine: (engine) => set({ engine }),
  setBridge: (bridge) => set({ bridge }),
  setBusy: (busy) => set({ busy }),
  bumpRecent: () => set((s) => ({ recentTick: s.recentTick + 1 })),
  openDocument: (id) => {
    // Leaving the current document commits its pending edit rather than dropping it, as
    // ProjectWorkspace.select/newCanvas do on macOS.
    get().commitTransform();
    const { engine, activeId } = get();
    if (get().adjustEdit) { set({ adjustEdit: null }); if (activeId) engine!.setPreview(activeId, null); }
    const state = engine!.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null,
      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null }));
  },
  closeDocument: (id) => {
    get().commitTransform();
    // `adjustEdit`, like `transformEdit`, names no document of its own: it always belongs to
    // whatever is `activeId` at the time. Closing some other, background tab must not touch it,
    // so this clears the engine-side preview on `activeId` (not the possibly-different `id`
    // being closed) and only when a panel could actually be open on it.
    const closingActive = get().activeId === id;
    if (closingActive && get().adjustEdit) { set({ adjustEdit: null }); get().engine!.setPreview(id, null); }
    get().engine!.closeDocument(id);
    set((s) => {
      const { [id]: _d, ...documents } = s.documents;
      const { [id]: _v, ...viewports } = s.viewports;
      const order = s.order.filter((o) => o !== id);
      const activeId = s.activeId === id ? order[order.length - 1] ?? null : s.activeId;
      const active = activeId ? documents[activeId] : null;
      return { documents, viewports, order, activeId, cropRect: null,
        selectedLayerIds: active?.activeLayerId ? [active.activeLayerId] : [], maskSelected: false, transformEdit: null };
    });
  },
  setActive: (id) => {
    get().commitTransform();
    const { engine, activeId: leavingId } = get();
    if (get().adjustEdit) { set({ adjustEdit: null }); if (leavingId) engine!.setPreview(leavingId, null); }
    const state = get().documents[id];
    set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null });
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
    // A panel owns the document while it is open, as macOS's canEditLayers does. Its own commit
    // clears `adjustEdit` before calling this, so OK is never refused.
    if (get().adjustEdit) { set({ error: "Apply or cancel the open adjustment first" }); return; }
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
    if (!engine || !activeId) return;
    try {
      engine.execute(activeId, command);
      get().refresh(activeId);
      if (REVEALING_COMMANDS.has(command.type)) get().revealActiveLayer();
    } catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); }
  },
  // A panel owns the document while it is open, as macOS's canEditLayers does; the menu items
  // for these are already disabled, so this stays quiet rather than raising the error banner.
  undo: () => { if (get().adjustEdit) return; const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
  redo: () => { if (get().adjustEdit) return; const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
  setTool: (tool) => {
    if (get().tool === "move" && tool !== "move") get().commitTransform();
    // Entering the crop tool seeds a full-canvas rectangle, as macOS does (EditorSession.selectTool);
    // the frame, the size readout and the Apply/Cancel buttons follow that rectangle, so once Apply
    // or Cancel clears it nothing is drawn until the user drags a new one. Leaving the tool clears it.
    const { activeId, documents, cropRect } = get();
    const doc = activeId ? documents[activeId] : null;
    const seeded = tool === "crop" ? (cropRect ?? (doc ? { x: 0, y: 0, width: doc.width, height: doc.height } : null)) : null;
    set({ tool, cropRect: seeded });
  },
  setCropRect: (cropRect) => set({ cropRect }),
  setCropRatio: (cropRatio) => set({ cropRatio }),
  openSheet: (sheet) => set({ sheet }),
  closeSheet: () => set({ sheet: null }),
  setError: (error) => set({ error }),
  setRendererKind: (rendererKind) => set({ rendererKind }),
  invalidate: () => set((s) => ({ renderTick: s.renderTick + 1 })),
  selectLayers: (ids, primary) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return;
    const state = get().documents[activeId];
    const valid = ids.filter((id) => state.layers.some((l) => l.id === id));
    const active = primary && valid.includes(primary) ? primary : valid[0] ?? null;
    get().commitTransform();
    if (active !== state.activeLayerId) { engine.execute(activeId, { type: "SetActiveLayer", id: active }); }
    set({ selectedLayerIds: valid, maskSelected: false });
    get().refresh(activeId);
  },
  setMaskSelected: (v) => set({ maskSelected: v }),
  toggleCollapsed: (id) => {
    const { activeId, collapsed } = get(); if (!activeId) return;
    const list = collapsed[activeId] ?? [];
    const collapsing = !list.includes(id);
    const next = collapsing ? [...list, id] : list.filter((x) => x !== id);
    set({ collapsed: { ...collapsed, [activeId]: next } });
    // Collapsing a folder whose descendant is active selects the folder itself, as macOS does.
    if (collapsing) {
      const state = get().documents[activeId];
      const byId = new Map(state.layers.map((l) => [l.id, l]));
      let node = state.activeLayerId ? byId.get(state.activeLayerId) : undefined;
      let steps = 0; let insideFolder = false;
      while (node?.parentId && steps++ < 65) { if (node.parentId === id) { insideFolder = true; break; } node = byId.get(node.parentId); }
      if (insideFolder) get().selectLayers([id], id);
    }
  },
  beginTransform: ({ persistent, duplicate }) => {
    // A panel owns the document while it is open, as macOS's canTransform (which gates on
    // canEditLayers) does. `selection.ts`'s `canTransform` has no view of `adjustEdit` -- it
    // takes only `DocumentState` plus the selection, shared with UI hit-testing that has no
    // reason to know about panels -- so this stays here rather than widening that signature.
    if (get().adjustEdit) return false;
    const { engine, activeId, selectedLayerIds, maskSelected } = get(); if (!engine || !activeId) return false;
    const state = get().documents[activeId];
    if (!canTransform(state, selectedLayerIds, maskSelected) || get().transformEdit) return false;
    if (transformsAsGroup(state, selectedLayerIds)) {
      const box = groupBox(state, selectedLayerIds)!;
      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false, undoDepthBefore: null } });
      return true;
    }
    let layer = activeLayer(state);
    if (!layer) return false;
    // An unlinked mask alone has no pixel layer of its own to duplicate: ignore `duplicate`
    // (computed before any duplication, from the layer this transform is actually about).
    const maskAlone = maskSelected && layer.hasMask && !layer.maskLinked;
    const willDuplicate = !!duplicate && !maskAlone;
    let undoDepthBefore: number | null = null;
    if (willDuplicate) {
      // The depth before the copy exists, so a later cancel can prove the entry it is about to
      // drop is the one this command pushed.
      undoDepthBefore = state.undoDepth;
      // Not `run`: the duplicate has to be observed here to seed the edit. Its failures
      // ("too many layers", "folders are not duplicated this way") still belong in the error
      // banner rather than thrown out of a pointerdown handler.
      try { engine.execute(activeId, { type: "DuplicateLayer", id: layer.id }); }
      catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); return false; }
      get().refresh(activeId);
      const copy = activeLayer(get().documents[activeId]);
      if (!copy) return false;
      layer = copy;
      set({ selectedLayerIds: [layer.id] });
    }
    const t = maskAlone ? layer.maskPlacement ?? layer.transform : layer.transform;
    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: willDuplicate, undoDepthBefore } });
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
    // Only when the entry on top is provably still the duplicate's: the depth must be exactly
    // one past what it was before the copy was made. `run` commits any pending edit before it
    // records, so nothing should be able to interleave, but reverting the wrong entry would
    // silently discard a real edit and strand the copy, so this refuses rather than guesses.
    if (e.duplicated && engine && activeId) {
      const depth = engine.state(activeId).undoDepth;
      if (e.undoDepthBefore !== null && depth === e.undoDepthBefore + 1) { engine.revert(activeId); get().refresh(activeId); }
    }
    get().invalidate();
  },
  setSnapGuides: (g) => set({ snapGuides: g }),
  setBlendPreview: (m) => set({ blendPreview: m }),
  previewEdit: () => {
    const a = get().adjustEdit;
    if (a && a.target === "adjustmentLayer" && a.preview && a.adjustment) return { kind: "adjustment", id: a.layerId, adjustment: a.adjustment };
    const e = get().transformEdit; if (!e) return null;
    if (e.kind === "group") return { kind: "group", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners };
    if (e.kind === "mask") return { kind: "mask", id: e.id, draft: e.draft };
    return { kind: "layer", id: e.id, draft: e.draft, corners: e.corners };
  },
  canAdjust: () => {
    const { activeId, documents, selectedLayerIds, maskSelected, adjustEdit } = get();
    if (!activeId || adjustEdit) return false;
    const state = documents[activeId];
    const layer = activeLayer(state);
    return !!layer && !layer.isGroup && layer.hasPixels && !maskSelected && selectedLayerIds.length === 1 && visibleIds(state).has(layer.id);
  },
  beginAdjust: ({ kind, layerId, target }) => {
    const { engine, activeId } = get(); if (!engine || !activeId) return false;
    get().commitTransform();
    const state = get().documents[activeId];
    const id = layerId ?? state.activeLayerId;
    const layer = id ? state.layers.find((l) => l.id === id) : null;
    if (!layer || get().adjustEdit) return false;
    const editing = target === "adjustmentLayer";
    if (editing ? !layer.adjustment : !get().canAdjust()) return false;
    const filter = isFilterKind(kind as string);
    const adjustment = filter ? null : (editing ? layer.adjustment! : defaultAdjustment(kind as AdjustmentKind));
    const edit: AdjustEdit = {
      kind, target: editing ? "adjustmentLayer" : "layer", layerId: layer.id,
      adjustment, params: filter ? defaultFilterParams(kind as FilterKind) : null,
      original: editing ? layer.adjustment! : null, preview: true, sampleMode: null,
      // Levels and Curves draw a histogram of what they are about to change.
      histogram: kind === "Levels" || kind === "Curves" ? engine.histogram(activeId, layer.id) : null,
    };
    set({ adjustEdit: edit });
    get().applyAdjustPreview();
    return true;
  },
  updateAdjust: (patch) => {
    const edit = get().adjustEdit; if (!edit) return;
    set({ adjustEdit: { ...edit, ...patch } });
    get().applyAdjustPreview();
  },
  setAdjustPreview: (preview) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, preview } }); get().applyAdjustPreview(); },
  setAdjustSample: (sampleMode) => { const e = get().adjustEdit; if (!e) return; set({ adjustEdit: { ...e, sampleMode } }); },
  /** Pushes the panel's settings to the engine: a pixel preview for a destructive edit, or a
   * plan-level preview (through `previewEdit`) when an adjustment layer is being edited. */
  applyAdjustPreview: () => {
    const { engine, activeId, adjustEdit } = get(); if (!engine || !activeId) return;
    if (!adjustEdit || adjustEdit.target === "adjustmentLayer") { if (!adjustEdit) engine.setPreview(activeId, null); get().invalidate(); return; }
    engine.setPreview(activeId, previewRequestFor(adjustEdit));
    get().refresh(activeId);
  },
  commitAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    set({ adjustEdit: null });
    engine.setPreview(activeId, null);
    if (isAdjustIdentity(edit)) { get().refresh(activeId); get().invalidate(); return; }
    if (edit.target === "adjustmentLayer") { get().run({ type: "SetAdjustment", id: edit.layerId, adjustment: edit.adjustment! }); return; }
    if (edit.params) get().run({ type: "ApplyFilter", id: edit.layerId, params: edit.params });
    else get().run({ type: "ApplyAdjustment", id: edit.layerId, adjustment: edit.adjustment! });
  },
  cancelAdjust: () => {
    const edit = get().adjustEdit; const { engine, activeId } = get(); if (!edit || !engine || !activeId) return;
    set({ adjustEdit: null });
    engine.setPreview(activeId, null);
    get().refresh(activeId);
    get().invalidate();
  },
}));
