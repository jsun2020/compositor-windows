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
  const state = document(layers, active);
  const engine = {
    state: () => state,
    execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    setPreview: (_id: string, request: PreviewRequest | null) => { previews.push(request); return { structure: true, canvas: false, layers: [] }; },
    histogram: () => [new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1), new Array(256).fill(1)],
    undo: () => ({ structure: true, canvas: false, layers: [] }),
    redo: () => ({ structure: true, canvas: false, layers: [] }),
  } as unknown as EngineClient;
  useEditor.setState({ engine, activeId: "D", documents: { D: state }, selectedLayerIds: [active], maskSelected: false,
    transformEdit: null, adjustEdit: null, error: null, tool: "move" });
  return { calls, previews };
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
    const { calls } = install([layer("A")], "A");
    useEditor.getState().beginAdjust({ kind: "Levels" });
    useEditor.getState().run({ type: "AddBlankLayer" });
    useEditor.getState().undo();
    useEditor.getState().redo();
    expect(calls).toEqual([]);
    expect(useEditor.getState().error).toMatch(/Apply or cancel/);
    expect(useEditor.getState().adjustEdit).not.toBeNull();
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

  it("knows the default settings and which ones do nothing", () => {
    expect(defaultAdjustment("Grain").grainSettings?.amount).toBe(25);
    expect(defaultFilterParams("MotionBlur")).toEqual({ filter: "MotionBlur", angle: 0, distance: 10 });
    expect(isAdjustIdentity({ kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null } as never)).toBe(true);
    expect(previewRequestFor({ target: "layer", layerId: "A", kind: "Levels", adjustment: defaultAdjustment("Levels"), params: null, preview: true } as never)).toBeNull();
  });
});
