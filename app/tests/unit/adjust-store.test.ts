import { beforeEach, describe, expect, it } from "vitest";
import { useEditor } from "../../src/state/store";
import { defaultAdjustment, defaultFilterParams, isAdjustIdentity, previewRequestFor } from "../../src/state/adjust-edit";
import type { Command, DocumentState, LayerState, PreviewRequest } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0,
    maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
function document(layers: LayerState[], active: string): DocumentState {
  return { id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: active, canUndo: false,
    canRedo: false, isModified: false, undoDepth: 0, path: null, layers };
}

function install(layers: LayerState[], active: string) {
  const calls: Command[] = [];
  const previews: (PreviewRequest | null)[] = [];
  const previewDocs: string[] = [];
  const history: string[] = [];
  const closed: string[] = [];
  const state = document(layers, active);
  const engine = {
    state: () => state,
    execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (id: string, request: PreviewRequest | null) => { previewDocs.push(id); previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => [new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1)],
    undo: () => { history.push("undo"); return { structure: true, canvas: false, layers: [] }; },
    redo: () => { history.push("redo"); return { structure: true, canvas: false, layers: [] }; },
    closeDocument: (id: string) => { closed.push(id); },
  } as unknown as EngineClient;
  useEditor.setState({ engine, activeId: "D", documents: { D: state }, order: ["D"], selectedLayerIds: [active], maskSelected: false,
    transformEdit: null, adjustEdit: null, error: null, tool: "move" });
  return { calls, previews, previewDocs, history, closed };
}

describe("adjustment panels", () => {
  beforeEach(() => useEditor.setState({ adjustEdit: null, error: null }));

  it("opening a panel on a pixel layer previews without recording anything", () => {
    const { calls, previews } = install([layer("A")], "A");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(true);
    const edit = useEditor.getState().adjustEdit!;
    expect(edit.target).toBe("layer");
    expect(edit.histogram?.length).toBe(4);
    expect(previews.at(-1)).toBeNull();   // an identity adjustment previews nothing
    const next = defaultAdjustment("Levels");
    next.levels.ranges[0] = { ...next.levels.ranges[0], outputWhite: 0 };
    useEditor.getState().updateAdjust({ adjustment: next });
    expect((previews.at(-1) as any).preview).toBe("Adjustment");
    expect(calls).toEqual([]);
    useEditor.getState().commitAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([{ type: "ApplyAdjustment", id: "A", adjustment: next }]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("an unchanged panel commits nothing and cancel clears the preview", () => {
    const { calls, previews } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
    useEditor.getState().beginAdjust({ kind: "Gaussian Blur" as never });
    useEditor.getState().updateAdjust({ params: { filter: "GaussianBlur", radius: 4 } });
    useEditor.getState().cancelAdjust();
    expect(previews.at(-1)).toBeNull();
    expect(calls).toEqual([]);
    expect(useEditor.getState().adjustEdit).toBeNull();
  });

  it("editing an adjustment layer previews through the plan and commits SetAdjustment", () => {
    const adjustment = defaultAdjustment("Curves");
    const { calls, previews } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment })], "J");
    expect(useEditor.getState().beginAdjust({ kind: "Curves", layerId: "J", target: "adjustmentLayer" })).toBe(true);
    const next = defaultAdjustment("Curves");
    next.curves.channels[0] = [{ x: 0, y: 255 }, { x: 255, y: 255 }];
    useEditor.getState().updateAdjust({ adjustment: next });
    expect(previews).toEqual([]);   // no pixel preview: the plan carries it
    expect(useEditor.getState().previewEdit()).toEqual({ kind: "adjustment", id: "J", adjustment: next });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([{ type: "SetAdjustment", id: "J", adjustment: next }]);
    expect(useEditor.getState().previewEdit()).toBeNull();
  });

  it("a panel owns the document: other commands, undo and redo are refused while it is open", () => {
    const { calls, history } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().run({ type: "AddBlankLayer" });
    useEditor.getState().undo();
    useEditor.getState().redo();
    expect(calls).toEqual([]);
    expect(history).toEqual([]);   // the engine's undo/redo must never be reached, not just no-op
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    expect(useEditor.getState().adjustEdit).not.toBeNull();
  });

  it("a panel owns the document: beginTransform is refused too (an Alt-drag duplicate must not record while a panel is open)", () => {
    const { calls } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().beginTransform({ persistent: true, duplicate: true })).toBe(false);
    expect(useEditor.getState().transformEdit).toBeNull();
    expect(calls).toEqual([]);
  });

  it("switching documents clears the panel and clears the preview on the document it belonged to, not the one being switched to", () => {
    const { previewDocs } = install([layer("A")], "A");
    useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    useEditor.getState().setActive("E");
    expect(useEditor.getState().adjustEdit).toBeNull();
    expect(useEditor.getState().activeId).toBe("E");
    // The clearing setPreview must target D (where the panel was), not E (where we are going).
    expect(previewDocs.at(-1)).toBe("D");
  });

  it("closing a background document leaves a panel open on the active document untouched", () => {
    const { previewDocs, closed } = install([layer("A")], "A");
    useEditor.setState((s) => ({ documents: { ...s.documents, E: document([layer("B")], "B") }, order: [...s.order, "E"] }));
    useEditor.getState().beginAdjust({ kind: "Levels" });
    expect(useEditor.getState().adjustEdit).not.toBeNull();
    const previewCallsBefore = previewDocs.length;
    useEditor.getState().closeDocument("E");
    expect(closed).toEqual(["E"]);
    expect(useEditor.getState().adjustEdit).not.toBeNull();      // D's panel is untouched
    expect(previewDocs.length).toBe(previewCallsBefore);         // no setPreview call for the close at all
    expect(useEditor.getState().documents.E).toBeUndefined();
    expect(useEditor.getState().activeId).toBe("D");
  });

  it("refuses to open on a folder, a hidden layer or a layer with no pixels", () => {
    install([layer("F", { isGroup: true, hasPixels: false })], "F");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("H", { visible: false })], "H");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    install([layer("B", { hasPixels: false, pixelsWidth: 0 })], "B");
    expect(useEditor.getState().beginAdjust({ kind: "Levels" })).toBe(false);
    expect(useEditor.getState().canAdjust()).toBe(false);
  });

  it("reopening a customised adjustment layer unchanged commits nothing", () => {
    // Regression for isAdjustIdentity comparing against defaultAdjustment(kind) instead of
    // edit.original: a customised adjustment layer is never equal to the identity default, so
    // that comparison judged every no-op reopen "changed" and issued a spurious SetAdjustment.
    const custom = defaultAdjustment("Curves");
    custom.curves.channels[0] = [{ x: 0, y: 40 }, { x: 255, y: 220 }];
    const { calls } = install([layer("A"), layer("J", { hasPixels: false, pixelsWidth: 0, adjustment: custom })], "J");
    useEditor.getState().beginAdjust({ kind: "Curves", layerId: "J", target: "adjustmentLayer" });
    useEditor.getState().commitAdjust();
    expect(calls).toEqual([]);
  });

  it("knows the default settings and which ones do nothing", () => {
    expect(defaultAdjustment("Grain").grainSettings?.amount).toBe(25);
    expect(defaultFilterParams("MotionBlur")).toEqual({ filter: "MotionBlur", angle: 0, distance: 10 });
    expect(isAdjustIdentity({ kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null } as never)).toBe(true);
    expect(previewRequestFor({ target: "layer", layerId: "A", kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null, preview: true } as never)).toBeNull();
  });
});
