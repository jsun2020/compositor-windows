import { describe, expect, it } from "vitest";
import { applyRatio, hitTest, snapTargets } from "../../src/tools/crop-tool";
import { Viewport } from "../../src/canvas/viewport";
import type { DocumentState } from "../../src/engine/types";

const doc: DocumentState = {
  id: "D", documentId: "D", width: 400, height: 300, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, path: null, guides: [],
  layers: [{ id: "L", name: "Red", visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal",
    transform: { origin: [150, 120], size: [100, 60], rotation: 0, flipX: false, flipY: false, sampling: "High quality" }, pixelsWidth: 100, pixelsHeight: 60, pixelsRevision: 1, hasMask: false,
    hasPixels: true, maskWidth: 0, maskHeight: 0, maskRevision: 1, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255 }],
};

describe("crop tool", () => {
  it("snap targets are the canvas and layer bounds", () => {
    const t = snapTargets(doc);
    expect(new Set(t.xs)).toEqual(new Set([0, 400, 150, 250]));
    expect(new Set(t.ys)).toEqual(new Set([0, 300, 120, 180]));
  });
  it("hit tests handles, inside and outside", () => {
    const vp = new Viewport(); vp.resize({ width: 800, height: 600 }, 1, { width: 400, height: 300 });
    const rect = { x: 100, y: 100, width: 200, height: 100 };
    const size = { width: 400, height: 300 };
    const corner = vp.viewPoint({ x: 300, y: 200 }, size);
    expect(hitTest(rect, corner, vp, size)).toEqual({ kind: "resize", index: 4 });
    expect(hitTest(rect, vp.viewPoint({ x: 200, y: 150 }, size), vp, size)).toEqual({ kind: "move" });
    expect(hitTest(rect, vp.viewPoint({ x: 10, y: 10 }, size), vp, size)).toEqual({ kind: "create" });
  });
  it("ratio keeps width and center", () => {
    expect(applyRatio({ x: 0, y: 0, width: 200, height: 50 }, 2)).toEqual({ x: 0, y: -25, width: 200, height: 100 });
  });
});
