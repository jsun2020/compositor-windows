import type { AdjustmentKind, FilterKind, FilterParams, LayerAdjustment, PreviewRequest } from "../engine/types";

/** Which eyedropper is armed: the Levels three, or the Hue/Saturation band tools. */
export type SampleMode = "Black" | "Gray" | "White" | "replace" | "add" | "remove";
/** Everything a panel edits. `kind` is an adjustment kind or a filter kind; the two never mix. */
export interface AdjustEdit {
  kind: AdjustmentKind | FilterKind;
  /** "layer" rewrites a pixel layer on OK; "adjustmentLayer" edits an adjustment layer's settings. */
  target: "layer" | "adjustmentLayer";
  layerId: string;
  adjustment: LayerAdjustment | null;
  params: FilterParams | null;
  /** The settings to put back when an adjustment-layer edit is cancelled. */
  original: LayerAdjustment | null;
  preview: boolean;
  sampleMode: SampleMode | null;
  histogram: number[][] | null;
}

const IDENTITY_LEVELS = { black: 0, gamma: 1, white: 255, outputBlack: 0, outputWhite: 255 };
const IDENTITY_CURVE = [{ x: 0, y: 0 }, { x: 255, y: 255 }];

export function defaultAdjustment(kind: AdjustmentKind): LayerAdjustment {
  const base: LayerAdjustment = {
    kind, hue: 0, saturation: 0, lightness: 0, colorize: false,
    levels: { channel: "RGB", ranges: [0, 1, 2, 3].map(() => ({ ...IDENTITY_LEVELS })) },
    curves: { channel: "RGB", channels: [0, 1, 2, 3].map(() => IDENTITY_CURVE.map((p) => ({ ...p }))) },
  };
  if (kind === "Exposure") base.exposureSettings = { exposure: 0, offset: 0, gamma: 1 };
  if (kind === "Gradient Map") base.gradientMapSettings = { shadows: { red: 0, green: 0, blue: 0 }, highlights: { red: 1, green: 1, blue: 1 }, reversed: false };
  if (kind === "Grain") base.grainSettings = { amount: 25, size: 1.5, roughness: 50, seed: 0 };
  return base;
}

export function defaultFilterParams(kind: FilterKind): FilterParams {
  switch (kind) {
    case "GaussianBlur": return { filter: "GaussianBlur", radius: 1 };
    case "MotionBlur": return { filter: "MotionBlur", angle: 0, distance: 10 };
    case "AddNoise": return { filter: "AddNoise", amount: 10, gaussian: false, monochromatic: false, seed: Math.floor(Math.random() * 0xffffffff) };
    default: return { filter: "LensCorrection", distortion: 0 };
  }
}

export const FILTER_TITLES: Record<FilterKind, string> = {
  GaussianBlur: "Gaussian Blur", MotionBlur: "Motion Blur", AddNoise: "Add Noise", LensCorrection: "Lens Correction",
};
export function adjustTitle(edit: Pick<AdjustEdit, "kind" | "params">): string {
  return edit.params ? FILTER_TITLES[edit.params.filter] : (edit.kind as string);
}
export const isFilterKind = (kind: string): kind is FilterKind => kind in FILTER_TITLES;

/** Deep-equal, ignoring an object's key order (two adjustments equal in content but built via a
 * different literal order, or round-tripped through JSON, must still compare equal). */
function sameShape(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((v, i) => sameShape(v, b[i]));
  }
  if (a && b && typeof a === "object" && typeof b === "object") {
    const ak = Object.keys(a as Record<string, unknown>);
    const bk = Object.keys(b as Record<string, unknown>);
    if (ak.length !== bk.length) return false;
    return ak.every((k) => sameShape((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));
  }
  return false;
}

/** Whether the panel's current settings would change nothing (so OK records no undo step).
 * Identity means "equal to what the panel opened with": for a destructive edit on a pixel
 * layer that is always the neutral default (there is nothing else it could have started from),
 * but for an adjustment layer it is `edit.original` -- the settings already on the layer, which
 * may themselves be far from the default. Comparing against the default unconditionally would
 * judge a no-op reopen of an already-customised adjustment layer as "changed". */
export function isAdjustIdentity(edit: Pick<AdjustEdit, "kind" | "adjustment" | "params" | "original">): boolean {
  if (edit.params) {
    const p = edit.params;
    return p.filter === "LensCorrection" ? p.distortion === 0 : false;
  }
  const a = edit.adjustment;
  if (!a) return true;
  const baseline = edit.original ?? defaultAdjustment(a.kind);
  return sameShape(a, baseline);
}

/** The pixel preview a destructive panel asks the engine for; null when there is nothing to show. */
export function previewRequestFor(edit: AdjustEdit): PreviewRequest | null {
  if (edit.target !== "layer" || !edit.preview || isAdjustIdentity(edit)) return null;
  if (edit.params) return { preview: "Filter", layer: edit.layerId, params: edit.params };
  return edit.adjustment ? { preview: "Adjustment", layer: edit.layerId, adjustment: edit.adjustment } : null;
}
