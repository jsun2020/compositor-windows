import { beforeEach, describe, expect, it } from "vitest";
import { useEditor, type EditorStore } from "../../src/state/store";
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
function document(id: string, layers: LayerState[], activeLayerId: string | null, undoDepth = 0): DocumentState {
  return { id, documentId: id, width: 100, height: 100, resolution: 72, activeLayerId, canUndo: true, canRedo: true, isModified: false, undoDepth, path: null, guides: [], undrawn: [], layers };
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

const pendingEdit = { kind: "layer" as const, id: "A", ids: ["A"], box, original: box, draft: { ...box, origin: [5, 5] as [number, number] }, corners: null, persistent: true, duplicated: false, undoDepthBefore: null };

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
    // The duplicate pushed one entry, so the depth is one past what it was beforehand.
    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 1);
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, undoDepthBefore: 0 } });
    useEditor.getState().cancelTransform();
    expect(calls).toContain("revert");
    expect(calls).not.toContain("undo");
    expect(useEditor.getState().transformEdit).toBeNull();
  });

  it("leaves history alone when something else recorded an entry in the meantime", () => {
    // Depth 2 where the cancel expects 1: the entry on top is no longer the duplicate's, so
    // reverting would silently drop a real edit and strand the copy.
    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 2);
    const { engine, calls } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A copy"], transformEdit: { ...pendingEdit, duplicated: true, undoDepthBefore: 0 } });
    useEditor.getState().cancelTransform();
    expect(calls).toEqual([]);
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

describe("a history-recording command closes a pending edit first", () => {
  it("a bare opacity digit commits the drag rather than interleaving an entry", async () => {
    const { runAction } = await import("../../src/shortcuts/useShortcuts");
    const doc = document("D", [layer("A"), layer("A copy")], "A copy", 1);
    const { engine, commands } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A copy"], tool: "move",
      transformEdit: { ...pendingEdit, id: "A copy", ids: ["A copy"], duplicated: true, undoDepthBefore: 0 } });
    runAction("opacity-5");
    // The transform commits before the opacity is recorded, so nothing can sit between the
    // duplicate and its own entry.
    expect(commands.map((c) => c.type)).toEqual(["SetLayerTransform", "SetLayersOpacity"]);
    expect(useEditor.getState().transformEdit).toBeNull();
  });

  it("the same holds for a panel command issued straight through run", () => {
    const doc = document("D", [layer("A")], "A", 0);
    const { engine, commands } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A"], transformEdit: pendingEdit });
    useEditor.getState().run({ type: "SetLayerVisible", id: "A", visible: false });
    expect(commands.map((c) => c.type)).toEqual(["SetLayerTransform", "SetLayerVisible"]);
    expect(useEditor.getState().transformEdit).toBeNull();
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

describe("import closes a pending edit", () => {
  it("commits a pending distortion before importing", async () => {
    const { importImages } = await import("../../src/actions/files");
    const doc = document("D", [layer("A")], "A", 1);
    const { engine, commands, calls } = stubEngine({ D: doc });
    const imported: string[] = [];
    (engine as unknown as { importImage: unknown }).importImage = (_d: string, _b: Uint8Array, name: string) => { imported.push(name); return "D"; };
    const bridge = {
      pickImportImages: async () => ["C:/p/one.png"],
      readFile: async () => new Uint8Array([1, 2, 3]),
      baseName: (p: string) => p.split("/").pop()!,
    } as unknown as EditorStore["bridge"];
    // A persistent distortion: the kind that survives pointerup and can still be open when a
    // menu action runs.
    const corners: [number, number][] = [[0, 0], [10, 1], [9, 9], [0, 10]];
    useEditor.setState({ engine, bridge, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A"],
      transformEdit: { ...pendingEdit, corners: corners as never, persistent: true } });
    await importImages();
    // The distortion is recorded first, then the import runs.
    expect(commands.map((c) => c.type)).toEqual(["DistortLayer"]);
    expect(imported).toEqual(["one.png"]);
    expect(useEditor.getState().transformEdit).toBeNull();
    expect(calls).not.toContain("revert");
  });
});

describe("benign no-ops raise no error banner", () => {
  it("Bring Forward at the top of the stack does nothing", async () => {
    const { canMoveActiveBy, moveActiveBy } = await import("../../src/actions/layers");
    const doc = document("D", [layer("low"), layer("high")], "high");
    const { engine, commands } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["high"] });
    expect(canMoveActiveBy(1)).toBe(false);
    moveActiveBy(1);
    expect(commands).toEqual([]);
    expect(useEditor.getState().error).toBeNull();
    // Downwards there is a sibling, so it still runs.
    expect(canMoveActiveBy(-1)).toBe(true);
    moveActiveBy(-1);
    expect(commands.map((c) => c.type)).toEqual(["MoveLayerBy"]);
  });

  it("siblings are counted within the parent folder, not the whole document", async () => {
    const { canMoveActiveBy } = await import("../../src/actions/layers");
    const layers = [layer("F", { isGroup: true }), layer("root"), layer("inner", { parentId: "F" })];
    const doc = document("D", layers, "inner");
    const { engine } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["inner"] });
    // "inner" is the only child of F, so it cannot move in either direction even though the
    // document has layers above and below it in the array.
    expect(canMoveActiveBy(1)).toBe(false);
    expect(canMoveActiveBy(-1)).toBe(false);
  });

  it("the clipping shortcut with nothing below to clip to does nothing", async () => {
    const { toggleClippingOfActive } = await import("../../src/actions/layers");
    const doc = document("D", [layer("only")], "only");
    const engine = {
      state: () => doc,
      execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
      canToggleClipping: () => false,
    } as unknown as EngineClient;
    const calls: Command[] = [];
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["only"] });
    toggleClippingOfActive();
    expect(calls).toEqual([]);
    expect(useEditor.getState().error).toBeNull();
  });
});

describe("mask actions commit a pending edit first", () => {
  it("Delete Mask commits the pending mask move before removing the mask", async () => {
    const { addMaskToActive, deleteMaskOfActive } = await import("../../src/actions/layers");
    const doc = document("D", [layer("A", { hasMask: true, maskWidth: 10, maskHeight: 10, maskLinked: false })], "A");
    const { engine, commands } = stubEngine({ D: doc });
    useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: ["A"], maskSelected: true, transformEdit: { ...pendingEdit, kind: "mask" } });
    deleteMaskOfActive();
    expect(commands.map((c) => c.type)).toEqual(["SetMaskPlacement", "DeleteMask"]);
    expect(useEditor.getState().transformEdit).toBeNull();

    const bare = document("D", [layer("A")], "A");
    const second = stubEngine({ D: bare });
    useEditor.setState({ engine: second.engine, activeId: "D", documents: { D: bare }, order: ["D"], selectedLayerIds: ["A"], maskSelected: false, transformEdit: pendingEdit });
    addMaskToActive(true);
    expect(second.commands.map((c) => c.type)).toEqual(["SetLayerTransform", "AddMask"]);
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
