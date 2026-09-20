import { create } from "zustand";
import type { Command, DocumentState } from "../engine/types";
import type { EngineClient } from "../engine/client";
import { Viewport } from "../canvas/viewport";
import type { Rect } from "../tools/crop-geometry";

export type Tool = "move" | "hand" | "zoom" | "crop";
export type CropRatio = "None" | "Original" | "1:1" | "4:3" | "16:9";
export type Sheet = null | { kind: "new" } | { kind: "canvasSize" } | { kind: "imageSize" } | { kind: "jpeg" };

export interface EditorStore {
  engine: EngineClient | null;
  documents: Record<string, DocumentState>;
  order: string[];
  activeId: string | null;
  viewports: Record<string, Viewport>;
  tool: Tool;
  cropRect: Rect | null;
  cropRatio: CropRatio;
  sheet: Sheet;
  error: string | null;
  rendererKind: "gl" | "cpu" | null;
  renderTick: number;
  setEngine(engine: EngineClient): void;
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
}

export const useEditor = create<EditorStore>((set, get) => ({
  engine: null, documents: {}, order: [], activeId: null, viewports: {}, tool: "move", cropRect: null, cropRatio: "None",
  sheet: null, error: null, rendererKind: null, renderTick: 0,
  setEngine: (engine) => set({ engine }),
  openDocument: (id) => {
    const engine = get().engine!;
    const state = engine.state(id);
    const viewport = new Viewport();
    set((s) => ({ documents: { ...s.documents, [id]: state }, order: s.order.includes(id) ? s.order : [...s.order, id],
      viewports: { ...s.viewports, [id]: viewport }, activeId: id, cropRect: null }));
  },
  closeDocument: (id) => {
    get().engine!.closeDocument(id);
    set((s) => {
      const { [id]: _d, ...documents } = s.documents;
      const { [id]: _v, ...viewports } = s.viewports;
      const order = s.order.filter((o) => o !== id);
      const activeId = s.activeId === id ? order[order.length - 1] ?? null : s.activeId;
      return { documents, viewports, order, activeId, cropRect: null };
    });
  },
  setActive: (id) => set({ activeId: id, cropRect: null }),
  refresh: (id) => {
    const target = id ?? get().activeId;
    if (!target) return;
    const state = get().engine!.state(target);
    set((s) => ({ documents: { ...s.documents, [target]: state }, renderTick: s.renderTick + 1 }));
  },
  run: (command) => {
    const { engine, activeId } = get();
    if (!engine || !activeId) return;
    try { engine.execute(activeId, command); get().refresh(activeId); }
    catch (e) { set({ error: String(e instanceof Error ? e.message : e) }); }
  },
  undo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.undo(activeId); get().refresh(activeId); } },
  redo: () => { const { engine, activeId } = get(); if (engine && activeId) { engine.redo(activeId); get().refresh(activeId); } },
  setTool: (tool) => set({ tool, cropRect: tool === "crop" ? get().cropRect : null }),
  setCropRect: (cropRect) => set({ cropRect }),
  setCropRatio: (cropRatio) => set({ cropRatio }),
  openSheet: (sheet) => set({ sheet }),
  closeSheet: () => set({ sheet: null }),
  setError: (error) => set({ error }),
  setRendererKind: (rendererKind) => set({ rendererKind }),
  invalidate: () => set((s) => ({ renderTick: s.renderTick + 1 })),
}));
