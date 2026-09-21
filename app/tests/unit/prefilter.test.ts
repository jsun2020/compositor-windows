import { describe, expect, it } from "vitest";
import { prefilterLevel, sizeAtLevel } from "../../src/canvas/layer-textures";
import { renderOrder } from "../../src/canvas/renderer";
import type { DocumentState, LayerState, LayerTransform } from "../../src/engine/types";

const box: LayerTransform = { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" };
function layer(id: string, over: Partial<LayerState> = {}): LayerState {
  return {
    id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: box,
    pixelsWidth: 10, pixelsHeight: 10, pixelsRevision: 1, hasPixels: true, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0,
    maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...over,
  };
}
function document(layers: LayerState[]): DocumentState {
  return { id: "D", documentId: "D", width: 100, height: 100, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, path: null, layers };
}

// The expected values here are the same ones asserted against compositor::prefilter_level in
// engine/tests/order.rs; the two implementations have to stay in step for the GL renderer and
// the CPU compositor to prefilter identically.
describe("prefilterLevel mirrors the engine's halving rule", () => {
  it("halves once per doubling past two source pixels per output pixel", () => {
    expect(prefilterLevel(1024, 1024, 1)).toBe(0);
    expect(prefilterLevel(1024, 1024, 2)).toBe(0);
    expect(prefilterLevel(1024, 1024, 4)).toBe(1);
    expect(prefilterLevel(1024, 1024, 16)).toBe(3);
  });
  it("stops when the raster runs out of pixels", () => {
    expect(prefilterLevel(2, 2, 1024)).toBe(1);
    expect(prefilterLevel(1, 1, 1024)).toBe(0);
  });
  it("reports the reduced size the same way", () => {
    expect(sizeAtLevel(1024, 512, 3)).toEqual({ width: 128, height: 64 });
    expect(sizeAtLevel(9, 9, 1)).toEqual({ width: 4, height: 4 });
    expect(sizeAtLevel(2, 2, 5)).toEqual({ width: 1, height: 1 });
    expect(sizeAtLevel(10, 10, 0)).toEqual({ width: 10, height: 10 });
  });
});

describe("renderOrder walks the hierarchy, not the array", () => {
  it("draws a folder's children at the folder's place even when they sit later in the array", () => {
    // The array a group gesture leaves behind: wrapper, then the untouched top layer, then the
    // wrapped children appended at the end.
    const doc = document([layer("G", { isGroup: true }), layer("C"), layer("A", { parentId: "G" }), layer("B", { parentId: "G" })]);
    expect(renderOrder(doc).map((l) => l.id)).toEqual(["A", "B", "C"]);
  });
  it("drops folders themselves and anything under a hidden ancestor", () => {
    const doc = document([layer("G", { isGroup: true, visible: false }), layer("C"), layer("A", { parentId: "G" })]);
    expect(renderOrder(doc).map((l) => l.id)).toEqual(["C"]);
  });
});
