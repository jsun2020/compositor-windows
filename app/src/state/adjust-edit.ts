import type { AdjustmentKind, FilterKind, FilterParams, LayerAdjustment, PreviewRequest } from "../engine/types";
import { DEFAULT_BLACK_WHITE, DEFAULT_COLOR_BALANCE } from "../engine/types";
import { defaultHsv } from "../tools/hue-band";
import { freshCameraRaw, cameraRawIsIdentity,effectiveCameraRaw,type CameraRawGroupName } from "../engine/camera-raw";

/** Which eyedropper is armed: the Levels three, or the Hue/Saturation band tools. */
export type SampleMode = "Black" | "Gray" | "White" | "replace" | "add" | "remove" | "CameraWhiteBalance" | "CameraPointColor" | "CameraDefringe";
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
  cameraRawBypass?:CameraRawGroupName[];
  cameraRawView?:{clipping:number;visualize:number;sharpen_mask:boolean;shadow_overlay:boolean;highlight_overlay:boolean};
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
    case "CameraRaw": return { filter: "CameraRaw", settings: freshCameraRaw() };
    case "GaussianBlur": return { filter: "GaussianBlur", radius: 1 };
    case "MotionBlur": return { filter: "MotionBlur", angle: 0, distance: 10 };
    case "AddNoise": return { filter: "AddNoise", amount: 10, gaussian: false, monochromatic: false, seed: Math.floor(Math.random() * 0xffffffff) };
    default: return { filter: "LensCorrection", distortion: 0 };
  }
}

export const FILTER_TITLES: Record<FilterKind, string> = {
  CameraRaw: "Camera Raw",
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

/** The engine's `LayerAdjustment::is_identity` (`EngineClient.adjustmentIsIdentity`). */
export type EngineIdentity = (adjustment: LayerAdjustment) => boolean;

/** Whether the panel's current settings would change nothing (so OK records no undo step and a
 * destructive panel previews nothing).
 *
 * For a destructive edit that is the engine's own per-kind rule, asked rather than mirrored:
 * Gradient Map always recolours, Grain applies at any amount above 0, and the Levels/Curves
 * channel and Hue/Saturation range selectors are where the panel is looking, not settings. Equal
 * to the kind's default is wrong for Grain and Gradient Map, whose defaults do something.
 *
 * For an adjustment layer it is "equal to `edit.original`", the settings already on the layer,
 * which may themselves be far from any default: a no-op reopen of a customised layer is not an
 * edit. */
export function effectiveFilterParams(edit:Pick<AdjustEdit,"params"|"cameraRawBypass">):FilterParams|null {const p=edit.params;return p?.filter==="CameraRaw"?{...p,settings:effectiveCameraRaw(p.settings,edit.cameraRawBypass)}:p;}
export function isAdjustIdentity(edit: Pick<AdjustEdit, "adjustment" | "params" | "original"|"cameraRawBypass">, engineIdentity: EngineIdentity): boolean {
  if (edit.params) {
    const p = effectiveFilterParams(edit)!;
    return p.filter === "CameraRaw" ? cameraRawIsIdentity(p.settings) : p.filter === "LensCorrection" ? p.distortion === 0 : false;
  }
  const a = edit.adjustment;
  if (!a) return true;
  return edit.original ? sameShape(a, edit.original) : engineIdentity(a);
}

/** What Reset puts back: the kind's neutral settings. What a panel cannot choose survives it:
 * Grain's seed (drawn when the panel or the layer was made) and, on an adjustment layer, its
 * Gradient Map colours. An adjustment layer resets only its own kind's settings and gains no
 * optional settings object it did not already carry, so Reset then OK on an untouched layer
 * still records nothing. */
export function resetAdjustment(current: LayerAdjustment, original: LayerAdjustment | null): LayerAdjustment {
  const fresh = defaultAdjustment(current.kind);
  const seed = (original ?? current).grainSettings?.seed;
  if (fresh.grainSettings && seed !== undefined) fresh.grainSettings.seed = seed;
  if (!original) return fresh;
  const out: LayerAdjustment = { ...original };
  switch (current.kind) {
    case "Levels": out.levels = fresh.levels; break;
    case "Curves": out.curves = fresh.curves; break;
    case "Hue/Saturation":
      if (original.hsvSettings) out.hsvSettings = defaultHsv();
      else Object.assign(out, { hue: 0, saturation: 0, lightness: 0, colorize: false });
      break;
    case "Exposure": if (original.exposureSettings) out.exposureSettings = fresh.exposureSettings; break;
    case "Gradient Map": if (original.gradientMapSettings) out.gradientMapSettings = { ...original.gradientMapSettings, reversed: false }; break;
    case "Grain": if (original.grainSettings) out.grainSettings = fresh.grainSettings; break;
    case "Black & White": if (original.blackWhiteSettings) out.blackWhiteSettings = { ...DEFAULT_BLACK_WHITE }; break;
    case "Color Balance": if (original.colorBalanceSettings) out.colorBalanceSettings = { ...DEFAULT_COLOR_BALANCE }; break;
    case "Gaussian Blur": if (original.blurRadius !== undefined) out.blurRadius = 10; break;
    case "Motion Blur":
      if (original.motionAngle !== undefined) out.motionAngle = 0;
      if (original.motionDistance !== undefined) out.motionDistance = 10;
      break;
    case "Add Noise":
      if (original.noiseAmount !== undefined) out.noiseAmount = 10;
      if (original.noiseGaussian !== undefined) out.noiseGaussian = false;
      if (original.noiseMonochromatic !== undefined) out.noiseMonochromatic = false;
      break;
  }
  return out;
}

/** The pixel preview a destructive panel asks the engine for; null when there is nothing to show.
 * `dragging` asks for a colour adjustment's quick, reduced preview (see `COLOUR_DRAG_LIMIT` in
 * engine/src/preview.rs); filters have one quality only. */
export function previewRequestFor(edit: AdjustEdit, engineIdentity: EngineIdentity, dragging = false): PreviewRequest | null {
  if (edit.target !== "layer" || !edit.preview) return null;
  const params=effectiveFilterParams(edit),view=edit.cameraRawView;
  if(params?.filter==="CameraRaw"&&view&&(view.clipping!==0||view.visualize>=0||view.sharpen_mask||view.shadow_overlay||view.highlight_overlay))return{preview:"CameraRawView",layer:edit.layerId,settings:params.settings,...view};
  if(isAdjustIdentity(edit, engineIdentity))return null;
  if (edit.params) return { preview: "Filter", layer: edit.layerId, params: effectiveFilterParams(edit)! };
  return edit.adjustment ? { preview: dragging ? "DragAdjustment" : "Adjustment", layer: edit.layerId, adjustment: edit.adjustment } : null;
}
