import { describe, expect, it } from "vitest";
import { TransformSession, nudgeDelta, startMode } from "../../src/tools/transform-session";
import type { LayerTransform } from "../../src/engine/types";

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
