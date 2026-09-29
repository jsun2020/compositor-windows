import type { GradientSpec } from "../engine/types";
import type { PaletteColor } from "../tools/color";

/** The Gradient tool's settings (`GradientSettings`, Gradient.swift:14-19), kept for the app session. */
export type GradientStyle = "Foreground to Background" | "Foreground to Transparent";
export interface GradientOptions { shape: "Linear" | "Radial"; style: GradientStyle; reversed: boolean; opacity: number; }
export const DEFAULT_GRADIENT: GradientOptions = { shape: "Linear", style: "Foreground to Transparent", reversed: false, opacity: 1 };
export const GRADIENT_STYLES: GradientStyle[] = ["Foreground to Background", "Foreground to Transparent"];

/** A gradient not yet applied (`GradientEdit`): the layer and whether its mask is the target, and the
 * line's two ends in document pixels. Previewed by the engine; nothing is in the document until it is
 * applied. */
export interface GradientEdit { layerId: string; mask: boolean; start: { x: number; y: number }; end: { x: number; y: number }; }

/** The shortest line that paints (`hasLine`: at least half a pixel; engine MIN_GRADIENT_LINE). */
export const hasLine = (e: GradientEdit): boolean => Math.hypot(e.end.x - e.start.x, e.end.y - e.start.y) >= 0.5;

/** How near, in view px, a press must land to grab an end of the pending line (EditorCanvas.swift:2026). */
export const GRADIENT_HANDLE_PX = 10;

/** The two stops (`gradientColors`, Gradient.swift:72-82): the foreground to the background, or to
 * the foreground made transparent; reversed if asked. On a mask the palette is already black or white. */
export function gradientStops(options: GradientOptions, foreground: PaletteColor, background: PaletteColor): [[number, number, number, number], [number, number, number, number]] {
  const rgba = (c: PaletteColor, a: number): [number, number, number, number] => [c.red, c.green, c.blue, a];
  const stops: [[number, number, number, number], [number, number, number, number]] = options.style === "Foreground to Background"
    ? [rgba(foreground, 1), rgba(background, 1)] : [rgba(foreground, 1), rgba(foreground, 0)];
  return options.reversed ? [stops[1], stops[0]] : stops;
}

/** The engine's `GradientSpec` for the pending line. */
export function gradientSpec(edit: GradientEdit, options: GradientOptions, foreground: PaletteColor, background: PaletteColor): GradientSpec {
  const [from, to] = gradientStops(options, foreground, background);
  return { shape: options.shape, start: [edit.start.x, edit.start.y], end: [edit.end.x, edit.end.y], from, to, opacity: options.opacity };
}

/** `point` moved onto the nearest eighth of a turn about `anchor`, at the same distance: Shift's 45
 * degree steps (`snapped`, EditorCanvas.swift:2034-2039; the Shape tool's line does the same). */
export function snapped45(point: { x: number; y: number }, anchor: { x: number; y: number }): { x: number; y: number } {
  const dx = point.x - anchor.x, dy = point.y - anchor.y;
  const length = Math.hypot(dx, dy);
  // Swift's `rounded()` takes halves away from zero; Math.round takes them up.
  const steps = Math.atan2(dy, dx) / (Math.PI / 4);
  const angle = Math.sign(steps) * Math.round(Math.abs(steps)) * (Math.PI / 4);
  return { x: anchor.x + Math.cos(angle) * length, y: anchor.y + Math.sin(angle) * length };
}
