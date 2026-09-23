import { describe, expect, it } from "vitest";
import { DEFAULT_BANDS, bandWeight, centeredOn, excludeHue, forward, hueOf, includeHue, setHandle } from "../../src/tools/hue-band";

describe("hue bands", () => {
  it("ramps through the falloff shoulders and wraps", () => {
    const reds = DEFAULT_BANDS.Reds;
    expect(bandWeight(reds, 0)).toBe(1);
    expect(bandWeight(reds, 345)).toBe(1);
    expect(bandWeight(reds, 330)).toBeCloseTo(0.5, 3);
    expect(bandWeight(reds, 30)).toBeCloseTo(0.5, 3);
    expect(bandWeight(reds, 180)).toBe(0);
    expect(bandWeight(DEFAULT_BANDS.Master, 123)).toBe(1);
    expect(forward(350, 10)).toBe(20);
  });
  it("re-centres, widens and narrows", () => {
    const centered = centeredOn(DEFAULT_BANDS.Greens, 0);
    expect(bandWeight(centered, 0)).toBe(1);
    expect(bandWeight(centered, 120)).toBe(0);
    const wider = includeHue(centered, 240);
    expect(bandWeight(wider, 240)).toBe(1);
    expect(bandWeight(excludeHue(wider, 240), 240)).toBe(0);
  });
  it("refuses a handle move that crosses its neighbours", () => {
    const greens = DEFAULT_BANDS.Greens;
    expect(setHandle(greens, 1, 200)).toEqual(greens);
    expect(setHandle(greens, 1, 110).rangeStart).toBe(110);
  });
  it("reads a hue from a colour, and nothing from a neutral one", () => {
    expect(hueOf([1, 0, 0])).toBeCloseTo(0, 3);
    expect(hueOf([0, 0, 1])).toBeCloseTo(240, 3);
    expect(hueOf([0.5, 0.5, 0.5])).toBeNull();
  });
  it("floors neutrality by HSB saturation (delta/high), not HSL's", () => {
    // rgb(1, 0.99, 0.99): 0.01 in HSB terms (delta/high = 0.01/1), still under the Mac's 0.02
    // floor -- but delta/(1-|2L-1|), the HSL formula used elsewhere in this file, blows up as
    // lightness approaches 1 and reads this as fully saturated (saturation 1), which is the
    // Critical this test guards: a near-white or near-black sample must stay neutral.
    expect(hueOf([1, 0.99, 0.99])).toBeNull();
    // The same colour, pushed just past the floor (delta/high = 0.03), does read a hue -- proving
    // the null above is the floor doing its job, not some other guard.
    expect(hueOf([1, 0.97, 0.97])).not.toBeNull();
  });
});
