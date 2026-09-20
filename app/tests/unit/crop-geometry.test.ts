import { describe, expect, it } from "vitest";
import { create, cropDrag, CropSnap, HANDLES, isValid } from "../../src/tools/crop-geometry";

describe("crop geometry", () => {
  it("supports reverse drags, ratios, moves and every handle", () => {
    const rect = create({ x: 100, y: 100 }, { x: 20, y: 60 }, 2, false);
    expect(rect).toEqual({ x: 20, y: 60, width: 80, height: 40 });
    const move = cropDrag({ kind: "move" }, { x: 40, y: 70 }, rect);
    expect(move.updated({ x: 30, y: 40 }, null, false)).toEqual({ x: 10, y: 30, width: 80, height: 40 });
    for (let index = 0; index < 8; index++) {
      const unit = HANDLES[index];
      const start = { x: rect.x + unit.x * rect.width, y: rect.y + unit.y * rect.height };
      const drag = cropDrag({ kind: "resize", index }, start, rect);
      const next = drag.updated({ x: start.x + (unit.x * 2 - 1) * 20, y: start.y + (unit.y * 2 - 1) * 10 }, 2, false);
      expect(isValid(next)).toBe(true);
      expect(Math.abs(next.width / next.height - 2)).toBeLessThan(0.05);
      expect(next).not.toEqual(rect);
    }
  });

  it("snaps edges to nearby targets", () => {
    const snap = new CropSnap([0, 200, 50, 150], [0, 100, 20, 80], 6);
    const rect = { x: 10, y: 10, width: 60, height: 40 };
    const move = cropDrag({ kind: "move" }, { x: 30, y: 30 }, rect);
    const movedTo = { x: 26, y: 34 };
    expect(snap.apply(move.updated(movedTo, null, false), move, movedTo, null, false)).toEqual({ x: 0, y: 20, width: 60, height: 40 });
    const cornerIndex = HANDLES.findIndex((h) => h.x === 1 && h.y === 1);
    const resize = cropDrag({ kind: "resize", index: cornerIndex }, { x: 70, y: 50 }, rect);
    const near = { x: 146, y: 83 };
    expect(snap.apply(resize.updated(near, null, false), resize, near, null, false)).toEqual({ x: 10, y: 10, width: 140, height: 70 });
    const far = { x: 120, y: 60 };
    expect(snap.apply(resize.updated(far, null, false), resize, far, null, false)).toEqual({ x: 10, y: 10, width: 110, height: 50 });
    const ratioRect = { x: 10, y: 10, width: 138, height: 69 };
    expect(snap.apply(ratioRect, resize, { x: 148, y: 79 }, 2, false)).toEqual(ratioRect);
    const createDrag = cropDrag({ kind: "create" }, { x: 52, y: 18 }, { x: 0, y: 0, width: 0, height: 0 });
    const dragged = { x: 147, y: 77 };
    expect(snap.apply(createDrag.updated(dragged, null, false), createDrag, dragged, null, false)).toEqual({ x: 52, y: 18, width: 98, height: 62 });
  });

  it("symmetric creation grows from the start point", () => {
    expect(create({ x: 50, y: 50 }, { x: 60, y: 70 }, null, true)).toEqual({ x: 40, y: 30, width: 20, height: 40 });
  });
});
