import { beforeEach, describe, expect, it } from "vitest";
import { DEFAULT_SELECTION_OPTIONS, useEditor } from "../../src/state/store";
import { addMaskToActive, canInvert, deleteKeyPressed, loadSelection } from "../../src/actions/layers";
import { SelectionDraft } from "../../src/tools/selection-draft";
import { parseAmount } from "../../src/sheets/SelectionAmountSheet";
import type { Command, DocumentState, LayerState, LayerTransform, SelectionState } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

// The store's selection routing (Phase 4a), after Compositor for Mac 1.2.10: Delete clears through a
// selection (SelectionEdits.swift:60-63), Add Mask takes it (LayerMask.swift:233-260), an empty one
// refuses every edit (canAdjustColors / canInvert / canPaint), Tab switches the tool's kind.
const box: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
function layer(id: string, over: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: box,
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
    maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...over };
}
const selected = (empty = false): SelectionState => ({ revision: 7, empty, bounds: empty ? null : { x: 2, y: 2, width: 5, height: 5 }, antialiased: true, feather: 0, points: 4 });
function document(layers: LayerState[], selection: SelectionState | null): DocumentState {
  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: layers[0]?.id ?? null, canUndo: false, canRedo: false,
    isModified: false, undoDepth: 0, path: null, guides: [], undrawn: [], selection, layers };
}
function stub(doc: DocumentState) {
  const commands: Command[] = [];
  const engine = { state: () => doc, clipDependents: () => [],
    execute: (_id: string, c: Command) => { commands.push(c); return { structure: true, canvas: false, layers: [] }; } } as unknown as EngineClient;
  useEditor.setState({ engine, activeId: "D", documents: { D: doc }, order: ["D"], selectedLayerIds: doc.activeLayerId ? [doc.activeLayerId] : [] });
  return commands;
}

beforeEach(() => {
  useEditor.setState({ engine: null, documents: {}, order: [], activeId: null, viewports: {}, transformEdit: null, selectedLayerIds: [], maskSelected: false,
    error: null, tool: "move", sheet: null, adjustEdit: null, selectionOptions: DEFAULT_SELECTION_OPTIONS, selectionDraft: null, outlineMove: null, cropRect: null });
});

describe("Delete", () => {
  it("clears the selected pixels with a selection, and deletes the layer without one", () => {
    let commands = stub(document([layer("A")], selected()));
    deleteKeyPressed();
    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: false }]);
    commands = stub(document([layer("A")], null));
    deleteKeyPressed();
    expect(commands.map((c) => c.type)).toEqual(["DeleteLayers"]);
  });

  it("fills the targeted mask, and does nothing on an empty selection, a hidden layer or a disabled mask", () => {
    let commands = stub(document([layer("A", { hasMask: true })], selected()));
    useEditor.setState({ maskSelected: true });
    deleteKeyPressed();
    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: true }]);
    for (const doc of [document([layer("A")], selected(true)), document([layer("A", { visible: false })], selected()),
      document([layer("A", { hasMask: true, maskEnabled: false })], selected())]) {
      commands = stub(doc);
      useEditor.setState({ maskSelected: doc.layers[0].hasMask });
      deleteKeyPressed();
      expect(commands).toEqual([]);
    }
  });
});

describe("Add Mask", () => {
  it("uses the selection when there is one", () => {
    let commands = stub(document([layer("A")], selected()));
    addMaskToActive(false);
    expect(commands).toEqual([{ type: "AddMaskFromSelection", id: "A", revealing: false }]);
    commands = stub(document([layer("A")], null));
    addMaskToActive(true);
    expect(commands).toEqual([{ type: "AddMask", id: "A", revealing: true }]);
  });
});

describe("the Expand / Contract / Feather sheet", () => {
  it("takes only a whole number from 1 to the operation's maximum (SelectionAmountSheet.amount)", () => {
    expect([parseAmount(" 12 ", 500), parseAmount("500", 500), parseAmount("251", 250), parseAmount("0", 250), parseAmount("2.5", 250), parseAmount("", 250), parseAmount("1e2", 500)])
      .toEqual([12, 500, null, null, null, null, null]);
  });
});

// Ruling (Task 10 review): canPaint requires no active crop (canEditLayers: cropRect == nil). Delete
// must not clear through the selection while the crop tool has a rectangle pending.
describe("Delete while a crop is pending", () => {
  it("does nothing with a crop rectangle active, and clears the selection once it is dropped", () => {
    const commands = stub(document([layer("A")], selected()));
    useEditor.setState({ cropRect: { x: 0, y: 0, width: 10, height: 10 } });
    deleteKeyPressed();
    expect(commands).toEqual([]);
    useEditor.setState({ cropRect: null });
    deleteKeyPressed();
    expect(commands).toEqual([{ type: "ClearSelectedPixels", id: "A", mask: false }]);
  });
});

describe("an empty selection refuses every edit", () => {
  it("greys out Levels, the filters and Invert", () => {
    stub(document([layer("A")], selected(true)));
    expect(useEditor.getState().canAdjust()).toBe(false);
    expect(canInvert()).toBe(false);
    stub(document([layer("A")], selected()));
    expect(useEditor.getState().canAdjust()).toBe(true);
    expect(canInvert()).toBe(true);
  });
});

describe("selection tool state", () => {
  it("Expand / Contract / Feather check their range and remember the amount", () => {
    const commands = stub(document([layer("A")], selected()));
    expect(useEditor.getState().modifySelection("Feather", 251)).toBe(false);
    expect(useEditor.getState().modifySelection("Expand", 0)).toBe(false);
    expect(useEditor.getState().modifySelection("Contract", 500)).toBe(true);
    expect(useEditor.getState().modifySelection("Feather", 12)).toBe(true);
    expect(commands).toEqual([{ type: "ContractSelection", amount: 500 }, { type: "FeatherSelection", amount: 12 }]);
    expect(useEditor.getState().selectionOptions).toMatchObject({ contract: 500, feather: 12, expand: 1 });
  });

  it("Tab switches the Marquee's and the Lasso's kind and drops a draft; changing tool drops it too", () => {
    stub(document([layer("A")], null));
    useEditor.setState({ tool: "marquee", selectionDraft: SelectionDraft.begin("Rectangle", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().cycleToolMode();
    expect(useEditor.getState().selectionOptions.marquee).toBe("Ellipse");
    expect(useEditor.getState().selectionDraft).toBeNull();
    useEditor.setState({ tool: "lasso", selectionDraft: SelectionDraft.begin("Freehand", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().cycleToolMode();
    expect(useEditor.getState().selectionOptions.lasso).toBe("Polygonal");
    useEditor.setState({ selectionDraft: SelectionDraft.begin("Polygonal", "Replace", { x: 1, y: 1 }) });
    useEditor.getState().setTool("move");
    expect(useEditor.getState().selectionDraft).toBeNull();
  });

  it("finishing a draft sends one SelectShape with the Anti-alias setting", () => {
    const commands = stub(document([layer("A")], null));
    useEditor.getState().setSelectionOptions({ antialiased: false });
    const draft = SelectionDraft.begin("Freehand", "Add", { x: 1, y: 1 });
    draft.extend({ x: 9, y: 1 }); draft.extend({ x: 9, y: 9 });
    useEditor.setState({ selectionDraft: draft });
    useEditor.getState().finishSelectionDraft();
    expect(commands).toEqual([{ type: "SelectShape", kind: "Freehand", mode: "Add", antialiased: false, points: [[1, 1], [9, 1], [9, 9]] }]);
    expect(useEditor.getState().selectionDraft).toBeNull();
  });

  it("Ctrl-clicking a thumbnail loads its pixels or its mask's black areas", () => {
    const commands = stub(document([layer("A", { hasMask: true })], null));
    loadSelection("A", false, "Add");
    loadSelection("A", true);
    expect(commands).toEqual([{ type: "LoadLayerSelection", id: "A", mode: "Add", antialiased: true }, { type: "LoadMaskSelection", id: "A", mode: "Replace", antialiased: true }]);
  });
});
