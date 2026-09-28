import { describe, expect, it } from "vitest";
import { useEditor } from "../../src/state/store";
import { cornersOf, cornersToTuples } from "../../src/tools/transform-geometry";
import type { Command, DocumentState, LayerState, LayerTransform } from "../../src/engine/types";
import type { EngineClient } from "../../src/engine/client";

const original: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };

function layer(): LayerState {
  return {
    id: "A", name: "A", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: original,
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
    maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255,
  };
}
function document(): DocumentState {
  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: "A", canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers: [layer()] };
}

/** A stub EngineClient recording every `execute` command, with no wasm involved. */
function stubEngine(doc: DocumentState): { engine: EngineClient; calls: Command[] } {
  const calls: Command[] = [];
  const engine = {
    state: () => doc,
    execute: (_id: string, cmd: Command) => { calls.push(cmd); return { structure: true, canvas: false, layers: [] }; },
    undo: () => ({ structure: true, canvas: false, layers: [] }),
    redo: () => ({ structure: true, canvas: false, layers: [] }),
    closeDocument: () => {},
  } as unknown as EngineClient;
  return { engine, calls };
}

describe("commitTransform unchanged check", () => {
  it("a distortion released without moving commits nothing", () => {
    const doc = document();
    const { engine, calls } = stubEngine(doc);
    useEditor.setState({
      engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: cornersToTuples(cornersOf(original)), persistent: true, duplicated: false, duplicateEntry: null },
    });
    useEditor.getState().commitTransform();
    expect(calls).toEqual([]);
    expect(useEditor.getState().transformEdit).toBeNull();
  });

  it("a distortion with a moved corner commits a DistortLayer", () => {
    const doc = document();
    const { engine, calls } = stubEngine(doc);
    const corners = cornersToTuples(cornersOf(original));
    const moved: typeof corners = [[corners[0][0] + 5, corners[0][1]], corners[1], corners[2], corners[3]];
    useEditor.setState({
      engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: original, corners: moved, persistent: true, duplicated: false, duplicateEntry: null },
    });
    useEditor.getState().commitTransform();
    expect(calls).toEqual([{ type: "DistortLayer", id: "A", transform: original, corners: moved }]);
    expect(useEditor.getState().transformEdit).toBeNull();
  });

  it("a plain move with a rounded-equal draft and no corners commits nothing", () => {
    const doc = document();
    const { engine, calls } = stubEngine(doc);
    useEditor.setState({
      engine, activeId: "D", documents: { D: doc }, selectedLayerIds: ["A"], maskSelected: false,
      transformEdit: { kind: "layer", id: "A", ids: ["A"], box: original, original, draft: { ...original, origin: [0.2, -0.1] }, corners: null, persistent: true, duplicated: false, duplicateEntry: null },
    });
    useEditor.getState().commitTransform();
    expect(calls).toEqual([]);
  });
});
