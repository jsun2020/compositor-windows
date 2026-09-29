import { describe, expect, it } from "vitest";
import { BLACK, hexOf, hsbOf, hsbToRgb, parseHex, quantized, withRgb } from "../../src/tools/color";

// Ported from the Mac's ColorPickerTests with their numbers.
describe("the palette's colours", () => {
  it("parses full and shorthand hex and rejects the rest (hexParsesFullShorthandAndRejectsInvalid)", () => {
    expect(parseHex("#FF8000")).toEqual({ red: 1, green: 128 / 255, blue: 0 });
    expect(parseHex("0f0")).toEqual({ red: 0, green: 1, blue: 0 });
    expect(parseHex(" 00ff00 ")).toEqual({ red: 0, green: 1, blue: 0 });
    expect(parseHex("12345")).toBeNull();
    expect(parseHex("GGGGGG")).toBeNull();
    expect(hexOf({ red: 1, green: 128 / 255, blue: 0 })).toBe("FF8000");
  });
  it("round-trips 8-bit colours through hue, saturation and brightness (hsbRoundTripsEightBitColors)", () => {
    for (const hex of ["000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FF8000", "7F3FA2", "123456"]) {
      expect(hexOf(quantized(hsbToRgb(hsbOf(parseHex(hex)!))))).toBe(hex);
    }
  });
  it("keeps the hue through a grey and the saturation through black (graysAndBlackKeepPreviousHueAndSaturation)", () => {
    let hsb = hsbOf(parseHex("FF8000")!);
    const hue = hsb.hue;
    // The formula: max R, (G - B) / delta * 60 = (128 / 255) * 60.
    expect(hue).toBeCloseTo((128 / 255) * 60, 12);
    hsb = withRgb(hsb, parseHex("808080")!);
    expect([hsb.hue, hsb.saturation]).toEqual([hue, 0]);
    hsb = withRgb({ ...hsb, saturation: 0.5 }, BLACK);
    expect([hsb.hue, hsb.saturation, hsb.brightness]).toEqual([hue, 0.5, 0]);
  });
  it("wraps hues past a turn and below zero onto the same colour", () => {
    // 420 and -300 degrees are 60: yellow at full saturation and brightness.
    for (const hue of [60, 420, -300]) expect(hexOf(hsbToRgb({ hue, saturation: 1, brightness: 1 }))).toBe("FFFF00");
    // A sixth of the way between sectors: 330 degrees is (1, 0, 0.5).
    expect(hsbToRgb({ hue: 330, saturation: 1, brightness: 1 })).toEqual({ red: 1, green: 0, blue: 0.5 });
  });
});
