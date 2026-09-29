/** The palette's colours and the colour picker's model, as Compositor for Mac keeps them
 * (ColorPalette.swift): sRGB 0..1 per channel, a hex form, and the picker's hue / saturation /
 * brightness, which keeps its hue through greys and its saturation through black. */
export interface PaletteColor { red: number; green: number; blue: number; }

export const BLACK: PaletteColor = { red: 0, green: 0, blue: 0 };
export const WHITE: PaletteColor = { red: 1, green: 1, blue: 1 };

export const sameColor = (a: PaletteColor, b: PaletteColor): boolean => a.red === b.red && a.green === b.green && a.blue === b.blue;
export const colorTuple = (c: PaletteColor): [number, number, number] => [c.red, c.green, c.blue];
export const colorOf = (t: readonly [number, number, number]): PaletteColor => ({ red: t[0], green: t[1], blue: t[2] });

/** Snapped to the 8-bit values painting and export store (`quantized`). */
export function quantized(c: PaletteColor): PaletteColor {
  return { red: Math.round(c.red * 255) / 255, green: Math.round(c.green * 255) / 255, blue: Math.round(c.blue * 255) / 255 };
}

/** `RRGGBB`, upper case (`hex`). */
export function hexOf(c: PaletteColor): string {
  return [c.red, c.green, c.blue].map((v) => Math.round(v * 255).toString(16).toUpperCase().padStart(2, "0")).join("");
}

/** `RRGGBB` or `RGB`, with or without a leading `#`, spaces around it ignored; null otherwise
 * (`PaletteColor(hex:)`). */
export function parseHex(text: string): PaletteColor | null {
  let t = text.trim();
  if (t.startsWith("#")) t = t.slice(1);
  if (t.length === 3) t = [...t].map((ch) => ch + ch).join("");
  if (!/^[0-9a-fA-F]{6}$/.test(t)) return null;
  const v = parseInt(t, 16);
  return { red: ((v >> 16) & 0xff) / 255, green: ((v >> 8) & 0xff) / 255, blue: (v & 0xff) / 255 };
}

/** Hue in degrees, saturation and brightness 0..1 (`PickerHSB`). */
export interface PickerHSB { hue: number; saturation: number; brightness: number; }

/** Swift's `truncatingRemainder`: the remainder with the dividend's sign, as JavaScript's `%`. */
const rem = (a: number, b: number) => a % b;

/** The colour the picker's values name (`PickerHSB.rgb`). */
export function hsbToRgb({ hue, saturation, brightness }: PickerHSB): PaletteColor {
  const h = rem(rem(hue, 360) + 360, 360) / 60;
  const c = brightness * saturation;
  const x = c * (1 - Math.abs(rem(h, 2) - 1));
  const m = brightness - c;
  let rgb: [number, number, number];
  switch (Math.trunc(h)) {
    case 0: rgb = [c, x, 0]; break;
    case 1: rgb = [x, c, 0]; break;
    case 2: rgb = [0, c, x]; break;
    case 3: rgb = [0, x, c]; break;
    case 4: rgb = [x, 0, c]; break;
    default: rgb = [c, 0, x];
  }
  return { red: rgb[0] + m, green: rgb[1] + m, blue: rgb[2] + m };
}

/** `hsb` moved to `color`, keeping its hue for a grey and its saturation for black, as
 * Photoshop's field does (`setRGB`). */
export function withRgb(hsb: PickerHSB, color: PaletteColor): PickerHSB {
  const high = Math.max(color.red, color.green, color.blue);
  const low = Math.min(color.red, color.green, color.blue);
  const delta = high - low;
  const next = { ...hsb, brightness: high };
  if (high > 0) next.saturation = delta / high;
  if (!(delta > 0)) return next;
  let h: number;
  if (high === color.red) h = (color.green - color.blue) / delta;
  else if (high === color.green) h = (color.blue - color.red) / delta + 2;
  else h = (color.red - color.green) / delta + 4;
  h *= 60;
  next.hue = h < 0 ? h + 360 : h;
  return next;
}

/** The picker's values for `color`, from nothing (`PickerHSB(_:)`). */
export const hsbOf = (color: PaletteColor): PickerHSB => withRgb({ hue: 0, saturation: 0, brightness: 0 }, color);

/** A CSS colour for a swatch. */
export const cssColor = (c: PaletteColor): string => `rgb(${Math.round(c.red * 255)}, ${Math.round(c.green * 255)}, ${Math.round(c.blue * 255)})`;
