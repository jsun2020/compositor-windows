import { describe, expect, it } from "vitest";
import { TransformSession, nudgeDelta, startMode } from "../../src/tools/transform-session";
import type { LayerTransform } from "../../src/engine/types";
import { roundedTransform } from "../../src/tools/transform-geometry";

const t = (x: number, y: number, w: number, h: number): LayerTransform => ({ origin: [x, y], size: [w, h], rotation: 0, flipX: false, flipY: false, sampling: "High quality" });

describe("transform session", () => {
  it("moves with snapping and reports guides", () => {
    const s = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 50, y: 50 }, original: t(40, 40, 20, 20), originalCorners: null, snap: { xs: [0, 100, 200], ys: [0, 100], tolerance: 4 } });
    const r = s.update({ x: 57, y: 53 }, { shift: false, alt: false, ctrl: false });
    // Unsnapped origin would be (47, 43): its right edge 67 and centre 57 are far from targets; left 47 is 3 from... no target at 50, so no x snap; top 43 is 3 from... no y target; nothing snaps.
    expect(r.draft.origin).toEqual([47, 43]);
    const s2 = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 50, y: 50 }, original: t(40, 40, 20, 20), originalCorners: null, snap: { xs: [83], ys: [0], tolerance: 4 } });
    const r2 = s2.update({ x: 71, y: 50 }, { shift: false, alt: false, ctrl: false });
    expect(r2.draft.origin[0]).toBe(63); // right edge 81 -> 83
    expect(r2.guides.xs).toEqual([83]);
  });
  it("shift constrains a move and distort drags corners", () => {
    const s = new TransformSession({ mode: { kind: "move" }, startDoc: { x: 0, y: 0 }, original: t(0, 0, 10, 10), originalCorners: null, snap: null });
    expect(s.update({ x: 10, y: 3 }, { shift: true, alt: false, ctrl: false }).draft.origin).toEqual([10, 0]);
    const corners = [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }, { x: 0, y: 10 }];
    const d = new TransformSession({ mode: { kind: "distort", index: 4 }, startDoc: { x: 10, y: 10 }, original: t(0, 0, 10, 10), originalCorners: corners, snap: null });
    const r = d.update({ x: 14, y: 12 }, { shift: false, alt: false, ctrl: false });
    expect(r.corners![2]).toEqual({ x: 14, y: 12 });
    expect(r.corners![0]).toEqual({ x: 0, y: 0 });
  });
  it("chooses the start mode", () => {
    expect(startMode({ kind: "resize", index: 0 }, true, true, false)).toEqual({ kind: "distort", index: 0 });
    expect(startMode({ kind: "resize", index: 1 }, true, true, false)).toEqual({ kind: "resize", index: 1 });
    expect(startMode({ kind: "resize", index: 3 }, true, false, true)).toEqual({ kind: "distort", index: 3 });
    expect(startMode(null, true, false, true)).toEqual({ kind: "distort", index: -1 });
    expect(startMode(null, false, false, false)).toEqual({ kind: "move" });
    expect(startMode({ kind: "rotate" }, false, false, false)).toEqual({ kind: "rotate" });
    expect(nudgeDelta("ArrowLeft", true)).toEqual({ dx: -10, dy: 0 });
    expect(nudgeDelta("x", false)).toBeNull();
  });
});

// ResizeSnapTests.swift (v1.4.5), ported: a 300 x 200 canvas, the layer being resized 100 x 100 at (10, 10),
// another 40 x 40 at (150, 150) whose left edge is at x 150. Targets as `snapTargets` builds them: the
// canvas's edges and centre and the other layer's edges and centre. Tolerance 5, as the Mac's tests use.
describe("resize-handle snapping (snappedResizePoint, Crop.swift:139-182)", () => {
  const targets = { xs: [0, 150, 300, 150, 170, 190], ys: [0, 100, 200, 150, 170, 190], tolerance: 5 };
  const resized = t(10, 10, 100, 100);
  const resize = (index: number, from: { x: number; y: number }, to: { x: number; y: number }, lockRatio: boolean, original = resized) => {
    const s = new TransformSession({ mode: { kind: "resize", index }, startDoc: from, original, originalCorners: null, snap: targets, lockRatio });
    const r = s.update(to, { shift: false, alt: false, ctrl: false });
    return { draft: roundedTransform(r.draft), guides: r.guides };
  };

  it("a side handle snaps its edge; out of reach it does not (aSideHandleSnapsItsEdge)", () => {
    // The right edge dragged to 147, three pixels short of the other layer's left edge.
    const { draft, guides } = resize(3, { x: 110, y: 60 }, { x: 147, y: 60 }, false);
    expect(draft.origin[0] + draft.size[0]).toBe(150);
    expect(draft.size[1]).toBe(100);
    expect(guides).toEqual({ xs: [150], ys: [] });
    const free = resize(3, { x: 110, y: 60 }, { x: 130, y: 60 }, false).draft;
    expect(free.origin[0] + free.size[0]).toBe(130);
  });

  it("a proportional corner snaps its nearer edge and keeps the ratio (aProportionalCornerSnapsItsNearerEdgeAndKeepsTheRatio)", () => {
    // Bottom right dragged toward (146, 148).
    const { draft, guides } = resize(4, { x: 110, y: 110 }, { x: 146, y: 148 }, true);
    expect(draft.origin[1] + draft.size[1]).toBe(150);
    expect(Math.abs(draft.size[0] - draft.size[1])).toBeLessThanOrEqual(1);
    // Kept proportional, one edge snaps and the other follows the ratio. Here both are 3 short of 150 and the
    // first found (the right edge, as Swift's min(by:) keeps the first) is the one that snaps.
    expect(guides).toEqual({ xs: [150], ys: [] });
  });

  it("a turned layer does not snap (aTurnedLayerDoesntSnap)", () => {
    const turned = { ...resized, rotation: 20 };
    const s = new TransformSession({ mode: { kind: "resize", index: 3 }, startDoc: { x: 110, y: 60 }, original: turned, originalCorners: null, snap: targets, lockRatio: false });
    const unsnapped = new TransformSession({ mode: { kind: "resize", index: 3 }, startDoc: { x: 110, y: 60 }, original: turned, originalCorners: null, snap: null, lockRatio: false });
    const mods = { shift: false, alt: false, ctrl: false };
    expect(s.update({ x: 147, y: 60 }, mods).draft).toEqual(unsnapped.update({ x: 147, y: 60 }, mods).draft);
    expect(s.update({ x: 147, y: 60 }, mods).guides).toEqual({ xs: [], ys: [] });
    // The Mac's one point never brings the turned box's edge near a target, so it would pass with the guard
    // gone; every pointer x from 100 to 260 does, somewhere, and none may snap.
    for (let x = 100; x <= 260; x++) {
      expect(s.update({ x, y: 60 }, mods).draft, `x ${x}`).toEqual(unsnapped.update({ x, y: 60 }, mods).draft);
    }
  });
});
