import { describe, expect, it } from "vitest";
import { canTransform, groupBox, groupMembers, transformsAsGroup } from "../../src/state/selection";
import type { DocumentState, LayerState } from "../../src/engine/types";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
const doc = (layers: LayerState[], active: string | null): DocumentState => ({ id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: active, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers });

describe("selection helpers", () => {
  const f = layer("F", { isGroup: true, hasPixels: false, pixelsWidth: 0 });
  const a = layer("A", { parentId: "F" });
  const b = layer("B", { parentId: "F", visible: false });
  const c = layer("C", { transform: { origin: [20, 20], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" } });
  const blank = layer("K", { hasPixels: false, pixelsWidth: 0 });
  const d = doc([f, a, b, c, blank], "C");
  it("decides when a transform is a group transform", () => {
    expect(transformsAsGroup(d, ["C"])).toBe(false);
    expect(transformsAsGroup(d, ["F"])).toBe(true);
    expect(transformsAsGroup(d, ["A", "C"])).toBe(true);
  });
  it("collects visible pixel members and their box", () => {
    expect(groupMembers(d, ["F", "C"]).map((l) => l.id)).toEqual(["A", "C"]);
    expect(groupBox(d, ["F", "C"])).toEqual(expect.objectContaining({ origin: [0, 0], size: [30, 30] }));
    expect(groupBox(d, ["K"])).toBeNull();
  });
  it("blank layers and hidden layers cannot be transformed alone", () => {
    expect(canTransform(doc([blank], "K"), ["K"], false)).toBe(false);
    expect(canTransform(d, ["C"], false)).toBe(true);
    expect(canTransform(doc([f, b], "B"), ["B"], false)).toBe(false);
  });
});
