import { describe, expect, it } from "vitest";
import { CanvasSizeDraft } from "../../src/tools/canvas-size-draft";

describe("CanvasSizeDraft", () => {
  it("relative, ratio and units use final dimensions", () => {
    const d = new CanvasSizeDraft(1000, 500, 100);
    d.relative = true; d.locked = true;
    d.set(200, true);
    expect([d.width, d.height]).toEqual([1200, 600]);
    expect(d.displayed(false)).toBe(100);
    d.set(-250, false);
    expect([d.width, d.height]).toEqual([500, 250]);
    d.relative = false; d.unit = "Inches";
    d.set(10, true);
    expect([d.width, d.height]).toEqual([1000, 500]);
    d.unit = "Percent";
    d.set(50, true);
    expect([d.width, d.height]).toEqual([500, 250]);
    d.unit = "Centimeters";
    expect(Math.abs(d.displayed(true) - 12.7)).toBeLessThan(0.001);
    d.unit = "Pixels";
    d.set(0, true);
    expect(d.valid).toBe(false);
  });
});
