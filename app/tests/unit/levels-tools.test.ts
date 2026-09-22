import { describe, expect, it } from "vitest";
import { clampRange, histogramScale } from "../../src/tools/levels-tools";

describe("levels tools", () => {
  it("keeps the distribution visible beside a clipping spike", () => {
    const bins = new Array(256).fill(100);
    bins[255] = 100_000;
    expect(histogramScale(bins)).toBe(400);
    bins[0] = 200_000;
    expect(histogramScale(bins)).toBe(400);
    bins[128] = 500_000;
    expect(histogramScale(bins)).toBe(400);
    expect(bins[128]).toBe(500_000);
    expect(histogramScale(new Array(256).fill(100))).toBe(100);
    const sparse = new Array(256).fill(0);
    expect(histogramScale(sparse)).toBe(0);
    sparse[255] = 50; expect(histogramScale(sparse)).toBe(50);
    sparse[0] = 100; expect(histogramScale(sparse)).toBe(100);
    sparse[128] = 200; expect(histogramScale(sparse)).toBe(200);
  });
  it("clamps a range the way the engine does", () => {
    const r = clampRange({ black: 300, gamma: Number.NaN, white: -1, outputBlack: -100, outputWhite: 400 });
    expect(r.black).toBeLessThan(r.white);
    expect(r).toEqual({ black: 254, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 });
  });
});
