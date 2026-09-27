import { describe, expect, it } from "vitest";
import { AntsPathCache, nextPhase, OUTLINE_DETAIL_LIMIT, OutlineCache, outlinePoints, outlineStep, traceOutline, type PathSink } from "../../src/canvas/ants";

/** Records what a path is built from, as `Path2D` would take it. */
class Recorder implements PathSink {
  calls: string[] = [];
  moveTo(x: number, y: number) { this.calls.push(`M${x},${y}`); }
  lineTo(x: number, y: number) { this.calls.push(`L${x},${y}`); }
  closePath() { this.calls.push("Z"); }
}
/** The engine's flat layout: the number of contours, then each one's point count and x, y pairs. */
function flat(contours: [number, number][][]): Float64Array {
  return new Float64Array([contours.length, ...contours.flatMap((c) => [c.length, ...c.flat()])]);
}

describe("marching ants", () => {
  it("fetch the full outline at 1:1 and above, and below it the power of two at or above the zoom", () => {
    // min(1, 2^ceil(log2(max(zoom, 1/4096)))) (TransformOverlay.swift:124): log2 0.9 = -0.15 rounds
    // up to 0, log2 0.3 = -1.74 to -1, log2 0.01 = -6.64 to -6; 1e-6 is held at 1/4096.
    expect([outlineStep(4), outlineStep(1), outlineStep(0.9), outlineStep(0.5), outlineStep(0.3), outlineStep(0.01), outlineStep(1e-6)])
      .toEqual([1, 1, 1, 0.5, 0.5, 1 / 64, 1 / 4096]);
  });

  it("refetch only when the document, the selection's revision or the step changes", () => {
    const cache = new OutlineCache();
    let fetches = 0;
    const fetch = () => { fetches++; return flat([[[0, 0], [1, 0], [1, 1]]]); };
    cache.get("D", 5, 1, fetch); cache.get("D", 5, 1, fetch);
    expect(fetches).toBe(1);
    cache.get("D", 6, 1, fetch); cache.get("D", 6, 0.5, fetch); cache.get("E", 6, 0.5, fetch);
    expect(fetches).toBe(4);
  });

  it("march one step a tick around a period of eight", () => {
    expect([nextPhase(0), nextPhase(6), nextPhase(7)]).toEqual([1, 7, 0]);
  });

  it("read the engine's flat outline: every contour closed, each point scaled to view px", () => {
    // A triangle and a two-point sliver (Engine::selection_outline's layout), at 2 view px a pixel.
    const outline = flat([[[0, 0], [10, 0], [0, 5.5]], [[7, 8], [9, 10]]]);
    expect(outlinePoints(outline)).toBe(5);
    const r = new Recorder();
    traceOutline(outline, 2, null, r);
    expect(r.calls).toEqual(["M0,0", "L20,0", "L0,11", "Z", "M14,16", "L18,20", "Z"]);
  });

  it("keep only the edges that meet the cull box, each run of them an open subpath", () => {
    // A 100 x 10 box, its right half beyond the cull box x <= 40: the top edge (0,0)-(100,0) and the
    // bottom edge (100,10)-(0,10) still cross it; the right edge x = 100 does not.
    const r = new Recorder();
    traceOutline(flat([[[0, 0], [100, 0], [100, 10], [0, 10]]]), 1, { x0: -5, y0: -5, x1: 40, y1: 20 }, r);
    expect(r.calls).toEqual(["M0,0", "L100,0", "M100,10", "L0,10", "L0,0"]);
  });

  it("build the path once for a march tick or a pan, and again for a new outline or zoom", () => {
    const cache = new AntsPathCache(() => new Recorder());
    const small = flat([[[0, 0], [10, 0], [10, 10]]]);
    const view = { x0: 0, y0: 0, x1: 800, y1: 600 };
    const first = cache.get(small, 1, view);
    expect(cache.get(small, 1, view)).toBe(first);
    expect(cache.get(small, 1, { x0: -300, y0: 50, x1: 500, y1: 650 }), "a small outline is whole: panning keeps it").toBe(first);
    expect(cache.builds).toBe(1);
    cache.get(small, 2, view);
    cache.get(flat([[[0, 0], [10, 0], [10, 10]]]), 2, view);
    expect(cache.builds, "another zoom, then another outline").toBe(3);
  });

  it("cull a detailed outline to the view's neighbourhood and rebuild only past it", () => {
    // A sawtooth of OUTLINE_DETAIL_LIMIT + 1 points along y = 0 from x = 0 to 20,000, back along y = 10.
    const n = OUTLINE_DETAIL_LIMIT + 1;
    const teeth: [number, number][] = Array.from({ length: n - 1 }, (_, k) => [k * 20_000 / (n - 2), (k % 2) * 0.5]);
    const outline = flat([[...teeth, [0, 10]]]);
    const cache = new AntsPathCache(() => new Recorder());
    const view = { x0: 1000, y0: -100, x1: 1800, y1: 500 };
    const path = cache.get(outline, 1, view);
    // Kept: the edges within one view's width (800 px) of the view on either side, x 200 to 2600,
    // about 2400 / 20,000 of the teeth, and none far away.
    const xs = path.calls.filter((c) => c.startsWith("L")).map((c) => Number(c.slice(1).split(",")[0]));
    expect(xs.length).toBeLessThan(n / 4);
    expect(Math.min(...xs.filter((x) => x > 0))).toBeLessThan(250);
    expect(xs.filter((x) => x > 2700 && x < 19_000)).toEqual([]);
    expect(cache.get(outline, 1, { x0: 1500, y0: 0, x1: 2300, y1: 600 }), "a pan within the kept neighbourhood").toBe(path);
    expect(cache.get(outline, 1, { x0: 5000, y0: 0, x1: 5800, y1: 600 })).not.toBe(path);
    expect(cache.builds).toBe(2);
  });
});
