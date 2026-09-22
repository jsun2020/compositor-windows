import { describe, expect, it } from "vitest";
import { curveSamples, curveValue, insertPoint, movePoint, nearestPoint, removePoint } from "../../src/tools/curves-editor";

const identity = [{ x: 0, y: 0 }, { x: 255, y: 255 }];

describe("curves editor", () => {
  it("interpolates the way the engine does", () => {
    expect(curveValue(identity, 100)).toBeCloseTo(100, 6);
    const s = [{ x: 0, y: 0 }, { x: 128, y: 190 }, { x: 255, y: 255 }];
    expect(curveValue(s, 128)).toBeCloseTo(190, 6);
    let last = -1;
    for (let x = 0; x <= 255; x++) { const v = curveValue(s, x); expect(v).toBeGreaterThanOrEqual(last - 1e-9); expect(v).toBeLessThanOrEqual(255); last = v; }
    expect(curveSamples(s, 256).length).toBe(256);
  });
  it("adds, moves and removes points under the editor's rules", () => {
    const three = insertPoint(identity, { x: 128, y: 190 });
    expect(three.map((p) => p.x)).toEqual([0, 128, 255]);
    expect(insertPoint(three, { x: 128, y: 10 })).toEqual(three);
    expect(nearestPoint(three, { x: 130, y: 188 }, 8)).toBe(1);
    expect(nearestPoint(three, { x: 60, y: 60 }, 8)).toBeNull();
    expect(movePoint(three, 1, { x: 200, y: 20 })[1]).toEqual({ x: 200, y: 20 });
    expect(movePoint(three, 1, { x: 300, y: 300 })[1]).toEqual({ x: 254, y: 255 });
    expect(movePoint(three, 0, { x: 40, y: 30 })[0]).toEqual({ x: 0, y: 30 });
    expect(removePoint(three, 1).map((p) => p.x)).toEqual([0, 255]);
    expect(removePoint(identity, 0)).toEqual(identity);
    const full = Array.from({ length: 32 }, (_, i) => ({ x: Math.round((i * 255) / 31), y: 0 }));
    expect(insertPoint(full, { x: 3, y: 3 })).toEqual(full);
  });
});
