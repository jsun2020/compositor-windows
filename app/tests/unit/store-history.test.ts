import { beforeEach, describe, expect, it } from "vitest";
import { useEditor } from "../../src/state/store";
import { runAction } from "../../src/shortcuts/useShortcuts";
import type { Command, DocumentState, LayerState, LayerTransform } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

const box: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };

function layer(id: string, over: Partial<LayerState> = {}): LayerState {
  return {
    id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: box,
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
    maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...over,
  };
}
function document(id: string, layers: LayerState[], activeLayerId: string | null): DocumentState {
  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, path: null, layers };
}

/** Records every engine call. `state` serves whatever `docs` currently holds for that handle. */
function stubEngine(docs: Record<string, DocumentState>) {
  const calls: string[] = [];
  const commands: Command[] = [];
  const engine = {
    state: (id: string) => docs[id],
    execute: (_id: string, cmd: Command) => { calls.push(`execute:${cmd.type}`); commands.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    undo: () => { calls.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    redo: () => { calls.push("redo"); return { structure: true, canvas: false, layers: [] }; },
    revert: () => { calls.push("revert"); return { structure: true, canvas: false, layers: [] }; },
    closeDocument: () => { calls.push("closeDocument"); },
    canPlace: () => true,
  } as unknown as EngineClient;
  return { engine, calls, commands };
}

const pendingEdit = { kind: "layer" as const, id: "A", ids: ["A"], box, original: box, draft: { ...box, origin: [5, 5] as [number, number] }, corners: null, persistent: true, duplicated: false };

beforeEach(() => {
  useEditor.setState({ engine: null, documents: {}, order: [], activeId: null, viewports: {}, collapsed: {}, transformEdit: null, selectedLayerIds: [], maskSelected: false, error: null, tool: "move", sheet: null });
});

describe("undo while a transform is pending", () => {
  it("Ctrl+Z does nothing and leaves the edit intact", () => {
    const doc = document("D", [layer("A")], "A");
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], transformEdit: pendingEdit });
    runAction("undo");
    runAction("redo");
    expect(calls).toEqual([]);
    expect(useEditor.getState().transformEdit).toEqual(pendingEdit);
  });

  it("Ctrl+Z works normally once nothing is pending", () => {
    const doc = document("D", [layer("A")], "A");
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], transformEdit: null });
    runAction("undo");
    expect(calls).toContain("undo");
  });
});

describe("cancelling an Alt-drag duplicate", () => {
  it("reverts the duplicate instead of undoing it, so there is nothing to redo", () => {
    const doc = document("D", [layer("A"), layer("A copy")], "A copy");
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true } });
    useEditor.getState().cancelTransform();
    expect(calls).toContain("revert");
    expect(calls).not.toContain("undo");
    expect(useEditor.getState().transformEdit).toBeNull();
  });

  it("a cancelled plain transform touches history not at all", () => {
    const doc = document("D", [layer("A")], "A");
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], transformEdit: pendingEdit });
    useEditor.getState().cancelTransform();
    expect(calls).toEqual([]);
  });
});

describe("leaving a document commits its pending edit", () => {
  it("switching tabs commits rather than discarding", () => {
    const a = document("A", [layer("A")], "A");
    const b = document("B", [layer("B")], "B");
    const { engine, commands } = stubEngine({ A: a, B: b });
    useEditor.setState({ engine, activeId: "A", documents: { A: a, B: b }, order: ["A", "B"], selectedLayerIds: ["A"], transformEdit: pendingEdit });
    useEditor.getState().setActive("B");
    expect(commands.map((c) => c.type)).toEqual(["SetLayerTransform"]);
    expect(useEditor.getState().activeId).toBe("B");
  });

  it("closing a document commits rather than discarding", () => {
    const a = document("A", [layer("A")], "A");
    const { engine, commands } = stubEngine({ A: a });
    useEditor.setState({ engine, activeId: "A", documents: { A: a }, order: ["A"], viewports: {}, selectedLayerIds: ["A"], transformEdit: pendingEdit });
    useEditor.getState().closeDocument("A");
    expect(commands.map((c) => c.type)).toEqual(["SetLayerTransform"]);
  });
});

describe("refresh of a background document", () => {
  it("leaves the visible document's selection alone", () => {
    const a = document("A", [layer("A1"), layer("A2")], "A2");
    const b = document("B", [layer("B1")], "B1");
    const { engine } = stubEngine({ A: a, B: b });
    useEditor.setState({ engine, activeId: "B", documents: { A: a, B: b }, order: ["A", "B"], selectedLayerIds: ["B1"] });
    useEditor.getState().refresh("A");
    expect(useEditor.getState().selectedLayerIds).toEqual(["B1"]);
    expect(useEditor.getState().documents.A).toBe(a);
  });

  it("still reconciles the selection of the visible document", () => {
    const b = document("B", [layer("B1")], "B1");
    const { engine } = stubEngine({ B: b });
    useEditor.setState({ engine, activeId: "B", documents: { B: b }, order: ["B"], selectedLayerIds: ["gone"] });
    useEditor.getState().refresh("B");
    expect(useEditor.getState().selectedLayerIds).toEqual(["B1"]);
  });
});

describe("a layer placed inside a collapsed folder", () => {
  it("expands its ancestors so the row appears", () => {
    const layers = [layer("F", { isGroup: true, hasPixels: false, pixelsWidth: 0, pixelsHeight: 0 }), layer("Inner", { parentId: "F" }), layer("Moved", { parentId: "F" })];
    const doc = document("D", layers, "Moved");
    const { engine } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], collapsed: { D: ["F"] }, selectedLayerIds: ["Moved"] });
    useEditor.getState().run({ type: "PlaceLayer", id: "Moved", parent: "F", above: null, atBottom: false });
    expect(useEditor.getState().collapsed.D).toEqual([]);
  });

  it("leaves unrelated collapsed folders alone", () => {
    const layers = [layer("F", { isGroup: true, hasPixels: false, pixelsWidth: 0, pixelsHeight: 0 }), layer("Inner", { parentId: "F" }), layer("Root")];
    const doc = document("D", layers, "Root");
    const { engine } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], collapsed: { D: ["F"] }, selectedLayerIds: ["Root"] });
    useEditor.getState().run({ type: "AddBlankLayer" });
    expect(useEditor.getState().collapsed.D).toEqual(["F"]);
  });
});

describe("beginTransform with a duplicate", () => {
  it("routes a rejected duplicate to the error banner instead of throwing", () => {
    const doc = document("D", [layer("A")], "A");
    const engine = {
      state: () => doc,
      execute: () => { throw new Error("too many layers"); },
    } as unknown as EngineClient;
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A"], maskSelected: false });
    expect(() => useEditor.getState().beginTransform({ persistent: false, duplicate: true })).not.toThrow();
    expect(useEditor.getState().error).toBe("too many layers");
    expect(useEditor.getState().transformEdit).toBeNull();
  });
});
