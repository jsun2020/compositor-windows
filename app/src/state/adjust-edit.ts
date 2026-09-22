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

/** Whether the panel's current settings would change nothing (so OK records no undo step). */
export function isAdjustIdentity(edit: Pick<AdjustEdit, "kind" | "adjustment" | "params">): boolean {
  if (edit.params) {
    const p = edit.params;
    return p.filter === "LensCorrection" ? p.distortion === 0 : false;
  }
  const a = edit.adjustment;
  if (!a) return true;
  return JSON.stringify(a) === JSON.stringify(defaultAdjustment(a.kind));
}

/** The pixel preview a destructive panel asks the engine for; null when there is nothing to show. */
export function previewRequestFor(edit: AdjustEdit): PreviewRequest | null {
  if (edit.target !== "layer" || !edit.preview || isAdjustIdentity(edit)) return null;
  if (edit.params) return { preview: "Filter", layer: edit.layerId, params: edit.params };
  return edit.adjustment ? { preview: "Adjustment", layer: edit.layerId, adjustment: edit.adjustment } : null;
}
