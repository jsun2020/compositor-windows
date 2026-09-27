import { describe, expect, it } from "vitest";
import { nextPhase, OutlineCache, outlineStep } from "../../src/canvas/ants";

describe("marching ants", () => {
  it("fetch the full outline at 1:1 and above, and a power-of-two step below", () => {
    expect([outlineStep(4), outlineStep(1), outlineStep(0.9), outlineStep(0.5), outlineStep(0.3), outlineStep(0.01)])
      .toEqual([1, 1, 0.5, 0.5, 0.25, 1 / 128]);
  });

  it("refetch only when the document, the selection's revision or the step changes", () => {
    const cache = new OutlineCache();
    let fetches = 0;
    const fetch = () => { fetches++; return [[[0, 0], [1, 0], [1, 1]] as [number, number][]]; };
    cache.get("D", 5, 1, fetch); cache.get("D", 5, 1, fetch);
    expect(fetches).toBe(1);
    cache.get("D", 6, 1, fetch); cache.get("D", 6, 0.5, fetch); cache.get("E", 6, 0.5, fetch);
    expect(fetches).toBe(4);
  });

  it("march one step a tick around a period of eight", () => {
    expect([nextPhase(0), nextPhase(6), nextPhase(7)]).toEqual([1, 7, 0]);
  });
});
