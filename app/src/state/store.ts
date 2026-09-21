import { create } from "zustand";
import type { BlendMode, Command, Corners, DocumentState, LayerTransform, PreviewEdit } from "../engine/types";
import type { EngineClient } from "../engine/client";
import type { ShellBridge } from "../shell/bridge";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";
import { cornersOf, cornersToTuples, isValidTransform, roundedTransform } from "../tools/transform-geometry";
import { activeLayer, canTransform, groupBox, transformsAsGroup } from "./selection";

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
  setEngine(engine: EngineClient): void;
  setBridge(bridge: ShellBridge): void;
  setBusy(busy: boolean): void;
  bumpRecent(): void;
  openDocument(id: string): void;
  closeDocument(id: string): void;
  setActive(id: string): void;
  refresh(id?: string): void;
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
}

export const useEditor = create<EditorStore>((set, get) => ({
  engine: null, bridge: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, busy: false, rendererKind: null, renderTick: 0, recentTick: 0,
  selectedLayerIds: [], maskSelected: false, collapsed: {}, transformEdit: null, snapGuides: { xs: [], ys: [] }, blendPreview: null,
  setEngine: (engine) => set({ engine }),
  setBridge: (bridge) => set({ bridge }),
  setBusy: (busy) => set({ busy }),
  bumpRecent: () => set((s) => ({ recentTick: s.recentTick + 1 })),
  openDocument: (id) => {
    const engine = get().engine!;
    const state = engine.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null,
      selectedLayerIds: state.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null }));
  },
  closeDocument: (id) => {
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
    const state = get().documents[id];
    set({ activeId: id, cropRect: null, selectedLayerIds: state?.activeLayerId ? [state.activeLayerId] : [], maskSelected: false, transformEdit: null });
  },
  refresh: (id) => {
    const target = id ?? get().activeId;
    if (!target) return;
    const state = get().engine!.state(target);
    const keep = get().selectedLayerIds.filter((sid) => state.layers.some((l) => l.id === sid));
    const selected = state.activeLayerId && !keep.includes(state.activeLayerId) ? [state.activeLayerId] : keep;
    set((s) => ({ documents: { ...s.documents, [target]: state }, renderTick: s.renderTick + 1, selectedLayerIds: selected }));
  },
  run: (command) => {
    const { engine, activeId } = get();
    if (!engine || !activeId) return;
    try { engine.execute(activeId, command); get().refresh(activeId); }
    catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); }
  },
  undo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
  redo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
  setTool: (tool) => {
    if (get().tool === "move" && tool !== "move") get().commitTransform();
    set({ tool, cropRect: tool === "crop" ? get().cropRect : null });
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
    const { engine, activeId, selectedLayerIds, maskSelected } = get(); if (!engine || !activeId) return false;
    const state = get().documents[activeId];
    if (!canTransform(state, selectedLayerIds, maskSelected) || get().transformEdit) return false;
    if (transformsAsGroup(state, selectedLayerIds)) {
      const box = groupBox(state, selectedLayerIds)!;
      set({ transformEdit: { kind: "group", id: state.activeLayerId!, ids: selectedLayerIds, box, original: box, draft: box, corners: null, persistent, duplicated: false } });
      return true;
    }
    let layer = activeLayer(state)!;
    // An unlinked mask alone has no pixel layer of its own to duplicate: ignore `duplicate`
    // (computed before any duplication, from the layer this transform is actually about).
    const maskAlone = maskSelected && layer.hasMask && !layer.maskLinked;
    const willDuplicate = !!duplicate && !maskAlone;
    if (willDuplicate) { engine.execute(activeId, { type: "DuplicateLayer", id: layer.id }); get().refresh(activeId); layer = activeLayer(get().documents[activeId])!; set({ selectedLayerIds: [layer.id] }); }
    const t = maskAlone ? layer.maskPlacement ?? layer.transform : layer.transform;
    set({ transformEdit: { kind: maskAlone ? "mask" : "layer", id: layer.id, ids: [layer.id], box: t, original: t, draft: t, corners: null, persistent, duplicated: willDuplicate } });
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
    const e = get().transformEdit; if (!e) return;
    set({ transformEdit: null, snapGuides: { xs: [], ys: [] } });
    if (e.duplicated) { get().undo(); }
    get().invalidate();
  },
  setSnapGuides: (g) => set({ snapGuides: g }),
  setBlendPreview: (m) => set({ blendPreview: m }),
  previewEdit: () => {
    const e = get().transformEdit; if (!e) return null;
    if (e.kind === "group") return { kind: "group", ids: e.ids, box: e.box, draft: e.draft, corners: e.corners };
    if (e.kind === "mask") return { kind: "mask", id: e.id, draft: e.draft };
    return { kind: "layer", id: e.id, draft: e.draft, corners: e.corners };
  },
}));
