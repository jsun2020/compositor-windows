import type { ColorRange, HueBand, HueSaturationSettings, RangeAdjustment } from "../engine/types";

/** A direct port of `HueBand`/`ColorRange`/`HueSaturationSettings` from
 * `engine/src/adjust/settings.rs` (itself a port of `HueSaturation.swift`), so the panel's band
 * math matches the engine's `weight`/`centered_on`/`include`/`exclude`/`set_handle` bit for bit. */

/** The six colours, in Photoshop's order; `ALL_RANGES` puts Master first, matching
 * `ColorRange::ALL` and `ColorRange.allCases`. */
export const COLOR_RANGES: ColorRange[] = ["Reds", "Yellows", "Greens", "Cyans", "Blues", "Magentas"];
export const ALL_RANGES: ColorRange[] = ["Master", ...COLOR_RANGES];

/** Photoshop's starting hue band for each range: falloff start, range start, range end, falloff
 * end. Master covers the whole circle (`weight` treats a zero span as "everywhere"). */
export const DEFAULT_BANDS: Record<ColorRange, HueBand> = {
  Master: { falloffStart: 0, rangeStart: 0, rangeEnd: 360, falloffEnd: 360 },
  Reds: { falloffStart: 315, rangeStart: 345, rangeEnd: 15, falloffEnd: 45 },
  Yellows: { falloffStart: 15, rangeStart: 45, rangeEnd: 75, falloffEnd: 105 },
  Greens: { falloffStart: 75, rangeStart: 105, rangeEnd: 135, falloffEnd: 165 },
  Cyans: { falloffStart: 135, rangeStart: 165, rangeEnd: 195, falloffEnd: 225 },
  Blues: { falloffStart: 195, rangeStart: 225, rangeEnd: 255, falloffEnd: 285 },
  Magentas: { falloffStart: 255, rangeStart: 285, rangeEnd: 315, falloffEnd: 345 },
};

function wrap360(v: number): number { const r = v % 360; return r < 0 ? r + 360 : r; }

/** Degrees from `from` forward to `to`, always 0..360. */
export function forward(from: number, to: number): number { return wrap360(to - from); }

/** 1 inside the range, ramping linearly through each falloff shoulder, 0 outside. Wraparound is
 * handled by measuring forward from `falloffStart`. */
export function bandWeight(band: HueBand, hue: number): number {
  const span = forward(band.falloffStart, band.falloffEnd);
  if (span <= 0) return 1; // Master covers everything.
  const position = forward(band.falloffStart, hue);
  if (position > span) return 0;
  const rampIn = forward(band.falloffStart, band.rangeStart);
  const plateauEnd = forward(band.falloffStart, band.rangeEnd);
  if (position < rampIn) return rampIn > 0 ? position / rampIn : 1;
  if (position <= plateauEnd) return 1;
  const rampOut = span - plateauEnd;
  return rampOut > 0 ? (span - position) / rampOut : 1;
}

/** Keeps all four handles in 0..360 and the band under a full circle, as
 * `HueBand.normalize`/`normalize` do on the other two platforms. */
function normalize(band: HueBand): HueBand {
  const falloffStart = wrap360(band.falloffStart);
  const rangeStart = wrap360(band.rangeStart);
  const rangeEnd = wrap360(band.rangeEnd);
  let falloffEnd = wrap360(band.falloffEnd);
  if (forward(falloffStart, falloffEnd) > 350) falloffEnd = wrap360(falloffStart + 350);
  return { falloffStart, rangeStart, rangeEnd, falloffEnd };
}

/** A band centered on one hue, keeping this band's core and shoulder widths. */
export function centeredOn(band: HueBand, hue: number): HueBand {
  const core = forward(band.rangeStart, band.rangeEnd);
  const leading = forward(band.falloffStart, band.rangeStart);
  const trailing = forward(band.rangeEnd, band.falloffEnd);
  const start = wrap360(hue - core / 2);
  return { falloffStart: wrap360(start - leading), rangeStart: start, rangeEnd: wrap360(start + core), falloffEnd: wrap360(start + core + trailing) };
}

/** Widens the band so this hue is fully inside it, moving whichever edge is nearer. A no-op when
 * the hue is already fully inside. */
export function includeHue(band: HueBand, hue: number): HueBand {
  if (bandWeight(band, hue) >= 1) return band;
  const shoulderIn = forward(band.falloffStart, band.rangeStart);
  const shoulderOut = forward(band.rangeEnd, band.falloffEnd);
  const next = forward(hue, band.rangeStart) <= forward(band.rangeEnd, hue)
    ? { ...band, rangeStart: hue, falloffStart: hue - shoulderIn }
    : { ...band, rangeEnd: hue, falloffEnd: hue + shoulderOut };
  return normalize(next);
}

/** Narrows the band so this hue falls outside it entirely, shoulder included. A no-op when the
 * hue is already fully outside. */
export function excludeHue(band: HueBand, hue: number): HueBand {
  if (bandWeight(band, hue) <= 0) return band;
  const shoulderIn = forward(band.falloffStart, band.rangeStart);
  const shoulderOut = forward(band.rangeEnd, band.falloffEnd);
  const next = forward(band.falloffStart, hue) <= forward(hue, band.falloffEnd)
    ? { ...band, falloffStart: hue + 1, rangeStart: hue + 1 + shoulderIn }
    : { ...band, falloffEnd: hue - 1, rangeEnd: hue - 1 - shoulderOut };
  return normalize(next);
}

/** Moves one handle (0 = falloffStart, 1 = rangeStart, 2 = rangeEnd, 3 = falloffEnd), keeping the
 * four in order and the band under a full circle; refused (the band is returned unchanged)
 * otherwise, matching `HueBand.setHandle`/`set_handle`. */
export function setHandle(band: HueBand, index: number, degrees: number): HueBand {
  const value = wrap360(degrees);
  const updated: HueBand = { ...band };
  if (index === 0) updated.falloffStart = value;
  else if (index === 1) updated.rangeStart = value;
  else if (index === 2) updated.rangeEnd = value;
  else updated.falloffEnd = value;
  const span = forward(updated.falloffStart, updated.falloffEnd);
  const toStart = forward(updated.falloffStart, updated.rangeStart);
  const toEnd = forward(updated.falloffStart, updated.rangeEnd);
  if (span > 1 && span <= 350 && toStart <= toEnd && toEnd <= span) return updated;
  return band;
}

/** The hue of a colour, mirroring `rgb_to_hsl`'s hue computation; null when the colour is too
 * close to neutral to have a meaningful hue, at the same 0.02-saturation floor as the Mac's
 * `sampledHue` (`hsb.saturation > 0.02 ? hsb.hue : nil`). `rgb` is 0..1 per channel. */
export function hueOf(rgb: [number, number, number]): number | null {
  const [r, g, b] = rgb;
  const high = Math.max(r, g, b);
  const low = Math.min(r, g, b);
  const delta = high - low;
  if (delta <= 0) return null; // Saturation 0: perfectly neutral.
  const lightness = (high + low) / 2;
  const saturation = Math.min(1, delta / (1 - Math.abs(2 * lightness - 1)));
  if (saturation <= 0.02) return null;
  let hue = high === r ? (g - b) / delta : high === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
  hue *= 60;
  if (hue < 0) hue += 360;
  return hue;
}

const IDENTITY_ADJUSTMENT: RangeAdjustment = { hue: 0, saturation: 0, lightness: 0 };

/** `HueSaturationSettings::default()` / plain `HueSaturationSettings()`: Master range, nothing
 * adjusted, colorize off. `bands` still covers all seven ranges (as the Rust/Swift constructor
 * always builds it), but `adjustments` holds only the one entry the constructor seeds. */
export function defaultHsv(): HueSaturationSettings {
  return { range: "Master", colorize: false, invertRange: false, adjustments: { Master: { ...IDENTITY_ADJUSTMENT } }, bands: { ...DEFAULT_BANDS } };
}

/** Photoshop's starting point when Colorize is switched on: hue 0, saturation 25, lightness 0,
 * on Master. Matches `HueSaturationSettings.colorizeStart` / `colorize_start()`. */
export function colorizeStart(): HueSaturationSettings {
  return { range: "Master", colorize: true, invertRange: false, adjustments: { Master: { hue: 0, saturation: 25, lightness: 0 } }, bands: { ...DEFAULT_BANDS } };
}
