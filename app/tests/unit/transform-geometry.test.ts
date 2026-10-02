import { describe, expect, it } from "vitest";
import { cornersOf, containsPoint, hitOverlay, homographyUnitTo, isUsableCorners, mat3Apply, mat3Invert, overlayGeometry, pixelToDocument, resizedTo, snapOffset, transformDrag } from "../../src/tools/transform-geometry";
import { HANDLES } from "../../src/tools/crop-geometry";
import { Viewport } from "../../src/canvas/viewport";
import type { LayerTransform } from "../../src/engine/types";

const t = (x: number, y: number, w: number, h: number, rotation = 0): LayerTransform => ({ origin: [x, y], size: [w, h], rotation, flipX: false, flipY: false, sampling: "High quality" });
const near = (a: number, b: number) => Math.abs(a - b) < 1e-4;

describe("transform geometry", () => {
  it("rotated resize keeps the opposite anchor at every handle", () => {
    const original = t(31, -19, 200, 100, 37);
    HANDLES.forEach((handle, index) => {
      const opposite = { x: 1 - handle.x, y: 1 - handle.y };
      const start = pointOfT(original, handle);
      const drag = transformDrag(original, start, { kind: "resize", index });
      for (const lockRatio of [true, false]) {
        const changed = drag.updated({ x: start.x + 34, y: start.y + 17 }, { lockRatio, shift: false, alt: false });
        const a = pointOfT(original, opposite), b = pointOfT(changed, opposite);
        expect(near(a.x, b.x) && near(a.y, b.y)).toBe(true);
        if (lockRatio) expect(near(changed.size[0] / changed.size[1], 2)).toBe(true);
      }
    });
  });
  it("move, rotate and shift constraints", () => {
    const original = t(0, 0, 100, 50);
    const moved = transformDrag(original, { x: 40, y: 20 }, { kind: "move" }).updated({ x: 60, y: 25 }, { lockRatio: true, shift: true, alt: false });
    expect(moved.origin).toEqual([20, 0]);
    const rotated = transformDrag(original, { x: 100, y: 25 }, { kind: "rotate" }).updated({ x: 50, y: 75 }, { lockRatio: true, shift: false, alt: false });
    expect(near(rotated.rotation, 90)).toBe(true);
    const snapped = transformDrag(original, { x: 100, y: 25 }, { kind: "rotate" }).updated({ x: 99, y: 45 }, { lockRatio: true, shift: true, alt: false });
    expect(snapped.rotation % 15).toBe(0);
    const free = transformDrag(original, { x: 100, y: 50 }, { kind: "resize", index: 4 }).updated({ x: 150, y: 50 }, { lockRatio: true, shift: true, alt: false });
    expect(free.size).toEqual([150, 50]);
    const centred = transformDrag(original, { x: 100, y: 25 }, { kind: "resize", index: 3 }).updated({ x: 120, y: 25 }, { lockRatio: false, shift: false, alt: true });
    expect(near(centred.size[0], 140) && near(centred.origin[0], -20)).toBe(true);
  });
  it("overlay hit regions match rotated edges, corners and the rotation handle", () => {
    const vp = new Viewport(); const size = { width: 1000, height: 800 };
    vp.resize({ width: 1000, height: 800 }, 1, size);
    const g = overlayGeometry(t(100, 100, 400, 300, 37), vp, size);
    for (const [s, e, expected] of [[0, 2, 1], [2, 4, 3], [4, 6, 5], [6, 0, 7]] as const) {
      const p = { x: g.handles[s].x * 0.75 + g.handles[e].x * 0.25, y: g.handles[s].y * 0.75 + g.handles[e].y * 0.25 };
      expect(hitOverlay(g, p)).toEqual({ kind: "resize", index: expected });
    }
    for (const i of [0, 2, 4, 6]) expect(hitOverlay(g, g.handles[i])).toEqual({ kind: "resize", index: i });
    expect(hitOverlay(g, g.rotationHandle)).toEqual({ kind: "rotate" });
    expect(hitOverlay(g, vp.viewPoint({ x: 300, y: 250 }, size))).toBeNull();
    expect(containsPoint(t(100, 200, 100, 50, 90), { x: 150, y: 265 })).toBe(true);
    expect(containsPoint(t(100, 200, 100, 50, 90), { x: 190, y: 225 })).toBe(false);
  });
  it("homography and pixel mapping", () => {
    const shape = [{ x: 10, y: 10 }, { x: 60, y: 10 }, { x: 30, y: 30 }, { x: 10, y: 30 }] as const;
    const h = homographyUnitTo([...shape]);
    expect(mat3Apply(h, { x: 1, y: 1 })).toEqual(expect.objectContaining({ x: expect.closeTo(30, 6), y: expect.closeTo(30, 6) }));
    expect(isUsableCorners([shape[0], shape[2], shape[1], shape[3]])).toBe(false);
    const m = pixelToDocument(t(10, 20, 100, 50), 64, 32);
    const c = mat3Apply(m, { x: 32, y: 16 });
    expect(near(c.x, 60) && near(c.y, 45)).toBe(true);
    const inv = mat3Invert(m)!;
    const back = mat3Apply(inv, c);
    expect(near(back.x, 32) && near(back.y, 16)).toBe(true);
    expect(cornersOf(t(0, 0, 10, 10))[2]).toEqual({ x: 10, y: 10 });
  });
  it("snap offset picks the smallest move per axis", () => {
    const r = snapOffset({ x: 3, y: 96, width: 10, height: 10 }, [0, 20], [100], 5);
    expect(r).toEqual({ dx: -3, dy: -1, x: 0, y: 100 });
    expect(snapOffset({ x: 50, y: 50, width: 10, height: 10 }, [0], [0], 5)).toEqual({ dx: 0, dy: 0, x: null, y: null });
  });
});

function pointOfT(tr: LayerTransform, unit: { x: number; y: number }) {
  const [ox, oy] = tr.origin; const [w, h] = tr.size; const cx = ox + w / 2, cy = oy + h / 2;
  const r = (tr.rotation % 360) * Math.PI / 180; const x = (unit.x - 0.5) * w, y = (unit.y - 0.5) * h;
  return { x: cx + x * Math.cos(r) - y * Math.sin(r), y: cy + x * Math.sin(r) + y * Math.cos(r) };
}

// The Move bar's W and H (TransformInspector.swift:89-100 at v1.4.5): the size typed, from the origin; with the
// aspect lock on the other side scales by the same factor; under 1 (or not a number) nothing changes.
describe("typed W and H (resizedTo)", () => {
  const base = t(10, 20, 200, 100, 30);
  it("keeps the origin and, locked, the ratio", () => {
    expect(resizedTo(base, 300, true, true)).toEqual({ ...base, size: [300, 100 * 300 / 200] });
    expect(resizedTo(base, 50, false, true)).toEqual({ ...base, size: [200 * 50 / 100, 50] });
  });
  it("unlocked, changes only the side typed", () => {
    expect(resizedTo(base, 300, true, false)).toEqual({ ...base, size: [300, 100] });
    expect(resizedTo(base, 7, false, false)).toEqual({ ...base, size: [200, 7] });
  });
  it("ignores a size under 1 or not a number", () => {
    for (const v of [0.5, 0, -10, Number.NaN]) expect(resizedTo(base, v, true, true)).toBe(base);
    expect(resizedTo(base, 1, false, false).size).toEqual([200, 1]);
  });
});
