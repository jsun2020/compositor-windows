import { describe, expect, it } from "vitest";
import { Viewport } from "../../src/canvas/viewport";

describe("Viewport", () => {
  it("maps document and view points both ways at any zoom", () => {
    const size = { width: 1000, height: 500 };
    for (const scale of [1, 2]) {
      const v = new Viewport();
      v.resize({ width: 800, height: 600 }, scale, size);
      v.translate({ width: 37, height: -19 });
      const point = { x: -20, y: 135 };
      const view = v.viewPoint(point, size);
      const back = v.documentPoint(view, size);
      expect(Math.abs(back.x - point.x)).toBeLessThan(0.001);
      expect(Math.abs(back.y - point.y)).toBeLessThan(0.001);
    }
  });

  it("fit leaves a 96 point margin and clamps zoom", () => {
    const v = new Viewport();
    v.resize({ width: 800, height: 600 }, 2, { width: 1000, height: 500 });
    // Automatic fit stops at 100%; explicit Fit may enlarge.
    expect(v.zoom).toBe(1);
    v.fit({ width: 1000, height: 500 }, true);
    expect(v.zoom).toBeCloseTo(1.408, 6);
    expect(v.followsFit).toBe(true);
    v.setZoom(1000, { x: 0, y: 0 }, { width: 1000, height: 500 });
    expect(v.zoom).toBe(32);
    expect(v.followsFit).toBe(false);
  });

  it("zoom anchors the pointer", () => {
    const size = { width: 1000, height: 800 };
    const v = new Viewport();
    v.resize({ width: 700, height: 500 }, 2, size);
    const anchor = { x: 100, y: 120 };
    const before = v.documentPoint(anchor, size);
    v.setZoom(2.5, anchor, size);
    const after = v.documentPoint(anchor, size);
    expect(Math.abs(after.x - before.x)).toBeLessThan(1e-6);
    expect(Math.abs(after.y - before.y)).toBeLessThan(1e-6);
  });

  it("resize keeps the pan proportional when not following fit", () => {
    const v = new Viewport();
    v.resize({ width: 800, height: 600 }, 1, { width: 100, height: 100 });
    v.translate({ width: 10, height: 20 });
    v.resize({ width: 800, height: 600 }, 2, { width: 100, height: 100 });
    expect(v.pan.width).toBeCloseTo(5, 6);
    expect(v.pan.height).toBeCloseTo(10, 6);
  });
});

it("keeps explicit Fit through window resize, but recenters a resized canvas without changing zoom",()=>{
  const v=new Viewport(); const small={width:100,height:100};
  v.resize({width:800,height:600},1,small); expect(v.zoom).toBe(1);
  v.fit(small,true); expect(v.zoom).toBeCloseTo(5.04);
  v.resize({width:1000,height:800},1,small); expect(v.zoom).toBeCloseTo(7.04);
  v.translate({width:23,height:-14}); const zoom=v.zoom;
  v.recenter(); expect(v.zoom).toBe(zoom); expect(v.pan).toEqual({width:0,height:0}); expect(v.followsFit).toBe(false);
  v.resize({width:1200,height:900},1,{width:200,height:200}); expect(v.zoom).toBe(zoom);
});
